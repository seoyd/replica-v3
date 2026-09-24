//! Immutable native F32 inference/resume container. All integers use explicit LE or canonical ULEB128.
//! No JSON header, tokenizer parser, SQLite, or external model dependency in this loader.
use super::{
    ByteBpe,
    checkpoint::{
        self, LegacyIdentity, Loaded, Manifest, ResumeBinding, TrainConfig, TrainingState,
    },
    transformer::{Config, MIXER, Transformer},
};
use crate::{
    Error, Result,
    codec::{PublicationTiming, Reader, publish_new_measured, put_bytes, put_varint},
};
use candle_core::{Device, Tensor};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    fs::File,
    io::{Read, Seek, SeekFrom, Write},
    path::{Path,PathBuf},
    time::Instant,
};

pub const WIRE_VERSION: u16 = 2;
const PREFIX: usize = 64;
const MAX_HEADER: usize = 2 * 1024 * 1024;
const MAX_FILE: u64 = 192 * 1024 * 1024;
const MAGIC: &[u8; 8] = b"R3MODEL\0";
type TensorMap = BTreeMap<String, Tensor>;
const DELTA_FORMAT: &str = "R3-QV-ADAPTER-F32-RESUME-v1";
const MAX_DELTA:usize=4*1024*1024;
#[derive(Clone,Debug,Serialize,Deserialize,PartialEq,Eq)]
#[serde(deny_unknown_fields)]
pub struct AdapterBase {
    pub path:PathBuf,
    pub physical:String,
    pub content:String,
    pub architecture:String,
    pub tokenizer:String,
    pub framing:[u8;32],
    pub step:usize,
}
#[derive(Clone,Debug,Serialize,Deserialize)]
#[serde(deny_unknown_fields)]
struct DeltaTensor { name:String, shape:Vec<usize>, digest:String, bytes:crate::binary::Value }
#[derive(Clone,Debug,Serialize,Deserialize)]
#[serde(deny_unknown_fields)]
struct DeltaRecord {
    format:String, base:AdapterBase, descriptor:super::transformer::QvAdapter,
    adapter_updates:usize, effective_step:usize, manifest:Manifest, tensors:Vec<DeltaTensor>,
}
fn delta_file(path:&Path)->Result<bool>{let mut magic=[0;8];File::open(path)?.read_exact(&mut magic)?;Ok(&magic==b"R3BIN\0\0\0")}
fn delta_record(path:&Path)->Result<DeltaRecord>{
    if path.with_extension("pending.r3b").exists(){return Err(bad("adapter publication pending"));}
    Ok(crate::binary::from_slice(&super::read_bounded(path,MAX_DELTA)?)?)
}
pub fn bind_adapter_base(model:&mut Transformer,base:AdapterBase)->Result<()> {
    if model.adapter().is_none()||model.base_content_id()?!=base.content||model.config.semantic_id()?!=base.architecture {
        return Err(bad("adapter base tensor identity"));
    }
    model.adapter_base=Some(base);Ok(())
}
pub fn adapter_base(model:&Transformer)->Option<&AdapterBase>{model.adapter_base.as_ref()}
fn decode_delta(record:&DeltaRecord,device:Device,resume:bool)->Result<Loaded>{
    let b=&record.base;let m=&record.manifest;let s=m.training.as_ref().ok_or_else(||bad("adapter resume state required"))?;
    if record.format!=DELTA_FORMAT||!b.path.is_absolute()||delta_file(&b.path)?
        ||super::hash(&super::read_bounded(&b.path,MAX_FILE as usize)?)!=b.physical
        ||record.effective_step!=s.step||record.adapter_updates!=s.step.checked_sub(b.step).ok_or_else(||bad("adapter clock"))?
        ||s.sampler_state!=s.step as u64||s.config.weight_decay!=0.||s.config.lr.to_bits()!=3e-4f64.to_bits()||s.config.warmup!=0
        ||s.config.first_target_weight!=1.||s.config.microbatch!=8||s.config.accumulation!=1
        ||s.step>s.config.max_steps||s.consumed_tokens>s.config.max_tokens||s.target_tokens>s.consumed_tokens
        ||s.parent_checkpoint_hash.as_deref()!=Some(b.physical.as_str())||s.config.budget_start_step!=b.step
        ||s.resume_binding.as_ref().is_none_or(|v|v.family!=checkpoint::ANSWER_MEAN_FAMILY||v.normalizer!=2||v.execution!=1||v.framing!=b.framing) {
        return Err(bad("adapter profile/base/clock/objective/optimizer contract"));
    }
    let mut l=load(&b.path,device.clone(),false)?;
    if l.model.weights_content_id()?!=b.content||l.model.config.semantic_id()?!=b.architecture||l.tokenizer.id()!=b.tokenizer
        ||l.manifest.framing()?.digest()!=b.framing||l.manifest.training.as_ref().map(|v|v.step)!=Some(b.step)
        ||m.architecture!=l.model.config||m.tokenizer_sha256!=b.tokenizer||m.dtype!="F32"
        ||m.trained_steps!=record.effective_step||m.initial_weight_hash!=l.manifest.initial_weight_hash {
        return Err(bad("adapter original model/tokenizer/framing mismatch"));
    }
    // The common validator owns lineage, status, finite losses and optimizer
    // metadata. Only the trainable tensor registry differs for a delta.
    let mut common=m.clone();common.tensors=checkpoint::expected(m);
    checkpoint::validate_metadata(&common,&l.tokenizer)?;
    let shapes=record.descriptor.shapes(&m.architecture)?;let mut expected=BTreeMap::new();
    for(n,d)in &shapes{for prefix in ["model.","adam.m.","adam.v."]{expected.insert(format!("{prefix}{n}"),d.clone());}}
    if m.tensors!=expected||record.tensors.len()!=expected.len(){return Err(bad("adapter tensor registry/Adam count"));}
    let(mut weights,mut optimizer)=(BTreeMap::new(),BTreeMap::new());let mut seen=std::collections::BTreeSet::new();
    for t in &record.tensors {
        let bytes=match &t.bytes{crate::binary::Value::Bytes(v)=>v,_=>return Err(bad("adapter payload is not raw bytes"))};
        if !seen.insert(&t.name)||expected.get(&t.name)!=Some(&t.shape)||bytes.len()!=t.shape.iter().product::<usize>()*4||super::hash(bytes)!=t.digest {
            return Err(bad("adapter tensor shape/length/hash"));
        }
        let values=bytes.chunks_exact(4).map(|b|f32::from_le_bytes(b.try_into().unwrap())).collect::<Vec<_>>();
        if values.iter().any(|v|!v.is_finite())||record.adapter_updates==0&&t.name.starts_with("adam.")&&values.iter().any(|&v|v!=0.) {return Err(bad("adapter nonfinite/nonfresh moments"));}
        let tensor=Tensor::from_vec(values,t.shape.clone(),&device)?;
        if let Some(n)=t.name.strip_prefix("model."){weights.insert(n.to_owned(),tensor);}else if resume {optimizer.insert(t.name.clone(),tensor);}
    }
    l.model.install_adapter(record.descriptor.clone(),weights)?;bind_adapter_base(&mut l.model,b.clone())?;
    if l.model.weights_content_id()?!=m.model_content_digest||l.model.weight_hash()?!=m.weights_sha256
        ||m.weights_bytes!=shapes.values().map(|d|d.iter().product::<usize>()*4).sum::<usize>() {return Err(bad("combined adapter model identity"));}
    l.manifest=m.clone();l.optimizer=optimizer;Ok(l)
}
fn save_delta(path:&Path,model:&Transformer,tok:&ByteBpe,mut m:Manifest,adam:&TensorMap)->Result<(Manifest,SaveStats)> {
    let started=Instant::now();let base=model.adapter_base.clone().ok_or_else(||bad("adapter missing explicit base reference"))?;
    let descriptor=model.adapter().cloned().ok_or_else(||bad("adapter missing descriptor"))?;
    if model.base_content_id()?!=base.content||tok.id()!=base.tokenizer{return Err(bad("frozen base changed"));}
    let s=m.training.as_ref().ok_or_else(||bad("adapter state absent"))?;let step=s.step;
    let mut tensors=vec![];let mut registry=BTreeMap::new();
    for(n,t)in model.vars.iter().map(|(n,v)|(format!("model.{n}"),v.as_detached_tensor())).chain(adam.iter().map(|(n,t)|(n.clone(),t.clone()))) {
        let bytes=tensor_bytes(&t)?;registry.insert(n.clone(),t.dims().to_vec());
        tensors.push(DeltaTensor{name:n,shape:t.dims().to_vec(),digest:super::hash(&bytes),bytes:crate::binary::Value::Bytes(bytes)});
    }
    m.architecture=model.config.clone();m.dtype="F32".into();m.tokenizer_sha256=tok.id();m.weights_sha256=model.weight_hash()?;
    m.model_content_digest=model.weights_content_id()?;m.weights_bytes=model.vars.values().map(|v|v.elem_count()*4).sum();m.tensors=registry;m.trained_steps=step;
    let record=DeltaRecord{format:DELTA_FORMAT.into(),adapter_updates:step.checked_sub(base.step).ok_or_else(||bad("adapter step before base"))?,effective_step:step,
        base,descriptor,manifest:m.clone(),tensors};
    let bytes=crate::binary::to_storage_vec(&record)?;
    if bytes.len()>MAX_DELTA{return Err(bad("adapter byte cap"));}
    let _:Loaded=decode_delta(&record,Device::Cpu,true)?;
    // Reuse the confirmed-publication interlock and the native no-clobber/fsync
    // publisher. A failed final sync leaves pending and is never loadable.
    let pending=path.with_extension("pending.r3b");
    crate::codec::publish_new(&pending,|f,_|{f.write_all(&crate::binary::to_vec(&crate::binary::record!({"destination":path,"content":super::hash(&bytes)}))?)?;Ok(())})?;
    let(_,publication)=publish_new_measured(path,|f,_|{f.write_all(&bytes)?;f.seek(SeekFrom::Start(0))?;let mut read=vec![];f.read_to_end(&mut read)?;
        let r:DeltaRecord=crate::binary::from_slice(&read)?;decode_delta(&r,Device::Cpu,true)?;Ok(())})?;
    std::fs::remove_file(pending)?;
    Ok((m,SaveStats{publication,total_ms:started.elapsed().as_secs_f64()*1000.,..Default::default()}))
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ArtifactKind {
    Inference,
    Resume,
}
impl ArtifactKind {
    fn tag(self) -> u8 {
        match self {
            Self::Inference => 1,
            Self::Resume => 2,
        }
    }
    fn from_tag(tag: u8) -> Result<Self> {
        match tag {
            1 => Ok(Self::Inference),
            2 => Ok(Self::Resume),
            _ => Err(bad("unknown artifact kind")),
        }
    }
}
#[derive(Debug, Default)]
pub struct LoadStats {
    pub header_bytes: u64,
    pub model_bytes: u64,
    pub optimizer_bytes: u64,
    pub padding_bytes: u64,
    pub directory_bytes: u64,
    pub tensor_read_ms: f64,
    pub tensor_hash_ms: f64,
    pub materialize_ms: f64,
    pub ready_ms: f64,
}
#[derive(Debug, Default)]
pub struct SaveStats {
    pub encode_hash_ms: f64,
    pub tensor_write_ms: f64,
    pub readback_ms: f64,
    pub publication: PublicationTiming,
    pub total_ms: f64,
}
impl LoadStats {
    pub fn bytes_read(&self) -> u64 {
        self.header_bytes + self.model_bytes + self.optimizer_bytes + self.padding_bytes
    }
}
#[derive(Clone)]
struct Entry {
    name: String,
    shape: Vec<usize>,
    offset: u64,
    length: u64,
    digest: [u8; 32],
}
struct Header {
    kind: ArtifactKind,
    manifest: Manifest,
    tokenizer: ByteBpe,
    entries: Vec<Entry>,
    model_id: String,
    header_end: u64,
    total: u64,
    directory_bytes: u64,
}
fn bad(s: &str) -> Error {
    Error::Corrupt(format!("native artifact: {s}"))
}
fn text(b: &mut Vec<u8>, s: &str) {
    put_bytes(b, s.as_bytes());
}
fn number(r: &mut Reader<'_>, max: usize) -> Result<usize> {
    let n = usize::try_from(r.var()?).map_err(|_| bad("integer overflow"))?;
    if n > max {
        return Err(bad("integer bound"));
    }
    Ok(n)
}
fn f64_read(r: &mut Reader<'_>) -> Result<f64> {
    let v = f64::from_le_bytes(r.take(8)?.try_into().expect("checked"));
    if !v.is_finite() {
        return Err(bad("nonfinite float"));
    }
    Ok(v)
}
fn u64_read(r: &mut Reader<'_>) -> Result<u64> {
    Ok(u64::from_le_bytes(r.take(8)?.try_into().expect("checked")))
}
fn digest_text(r: &mut Reader<'_>) -> Result<String> {
    let s = r.text()?;
    if s.len() != 64 || !s.bytes().all(|b| b.is_ascii_hexdigit()) {
        return Err(bad("digest text"));
    }
    Ok(s)
}
fn align(n: u64) -> Result<u64> {
    n.checked_add(63)
        .map(|n| n & !63)
        .ok_or_else(|| bad("alignment overflow"))
}
fn config_encode(b: &mut Vec<u8>, c: &Config) {
    text(b, &c.profile);
    for n in [
        c.vocab,
        c.layers,
        c.hidden,
        c.heads,
        c.kv_heads,
        c.head_dim,
        c.ffn,
        c.local_layers,
        c.window,
        c.context,
    ] {
        put_varint(b, n as u64);
    }
    b.extend_from_slice(&c.eps.to_le_bytes());
    b.extend_from_slice(&c.rope_theta.to_le_bytes());
}
fn config_decode(r: &mut Reader<'_>) -> Result<Config> {
    let c = Config {
        profile: r.text()?,
        vocab: number(r, 4096)?,
        layers: number(r, 6)?,
        hidden: number(r, 384)?,
        heads: number(r, 8)?,
        kv_heads: number(r, 8)?,
        head_dim: number(r, 48)?,
        ffn: number(r, 1024)?,
        local_layers: number(r, 6)?,
        window: number(r, 256)?,
        context: number(r, 2048)?,
        eps: f64_read(r)?,
        rope_theta: f64_read(r)?,
    };
    c.validate()?;
    Ok(c)
}
fn state_encode(b: &mut Vec<u8>, s: &TrainingState) {
    b.push(u8::from(s.contrast16));
    b.push(u8::from(s.parent_checkpoint_hash.is_some()));
    if let Some(h) = &s.parent_checkpoint_hash {
        text(b, h);
    }
    let c = &s.config;
    for v in [
        c.lr,
        c.beta1,
        c.beta2,
        c.eps,
        c.weight_decay,
        c.clip,
        c.first_target_weight,
    ] {
        b.extend_from_slice(&v.to_le_bytes());
    }
    for n in [
        c.warmup,
        c.max_steps,
        c.microbatch,
        c.sample_group_size,
        c.accumulation,
        c.seq_len,
        c.validate_every,
        c.curriculum_steps,
        c.budget_start_step,
    ] {
        put_varint(b, n as u64);
    }
    for n in [
        c.max_tokens,
        c.seed,
        c.budget_start_tokens,
        s.step as u64,
        s.consumed_tokens,
        s.target_tokens,
        s.sampler_state,
    ] {
        put_varint(b, n);
    }
    for h in [&s.corpus_hash, &s.validation_hash, &s.initial_weight_hash] {
        text(b, h);
    }
    put_varint(b, s.previous_corpora.len() as u64);
    for h in &s.previous_corpora {
        text(b, h);
    }
    for v in [s.train_loss, s.validation_loss] {
        b.push(u8::from(v.is_some()));
        if let Some(v) = v {
            b.extend_from_slice(&v.to_le_bytes());
        }
    }
    b.push(u8::from(s.resume_binding.is_some()));
    if let Some(binding) = &s.resume_binding {
        binding.encode(b);
        b.extend(binding.digest());
    }
}
fn state_decode(r: &mut Reader<'_>, wire: u16) -> Result<TrainingState> {
    let contrast16 = r.bool()?;
    let parent_checkpoint_hash = r.opt(digest_text)?;
    let mut c = TrainConfig {
        lr: f64_read(r)?,
        beta1: f64_read(r)?,
        beta2: f64_read(r)?,
        eps: f64_read(r)?,
        weight_decay: f64_read(r)?,
        clip: f64_read(r)?,
        first_target_weight: f64_read(r)?,
        ..Default::default()
    };
    c.warmup = number(r, i32::MAX as usize)?;
    c.max_steps = number(r, i32::MAX as usize)?;
    c.microbatch = number(r, 8)?;
    c.sample_group_size = number(r, 8)?;
    c.accumulation = number(r, 32)?;
    c.seq_len = number(r, 2048)?;
    c.validate_every = number(r, i32::MAX as usize)?;
    c.curriculum_steps = number(r, i32::MAX as usize)?;
    c.budget_start_step = number(r, i32::MAX as usize)?;
    c.max_tokens = r.var()?;
    c.seed = r.var()?;
    c.budget_start_tokens = r.var()?;
    let step = number(r, i32::MAX as usize)?;
    let consumed_tokens = r.var()?;
    let target_tokens = r.var()?;
    let sampler_state = r.var()?;
    let corpus_hash = digest_text(r)?;
    let validation_hash = digest_text(r)?;
    let initial_weight_hash = digest_text(r)?;
    let count = number(r, 32)?;
    let previous_corpora = (0..count).map(|_| digest_text(r)).collect::<Result<_>>()?;
    Ok(TrainingState {
        contrast16,
        parent_checkpoint_hash,
        config: c,
        step,
        consumed_tokens,
        target_tokens,
        sampler_state,
        corpus_hash,
        validation_hash,
        previous_corpora,
        initial_weight_hash,
        train_loss: r.opt(f64_read)?,
        validation_loss: r.opt(f64_read)?,
        resume_binding: if wire == 1 {
            None
        } else {
            r.opt(binding_decode)?
        },
    })
}
fn binding_decode(r: &mut Reader<'_>) -> Result<ResumeBinding> {
    fn digest(r: &mut Reader<'_>) -> Result<[u8; 32]> {
        Ok(r.take(32)?.try_into().expect("checked"))
    }
    let s = ResumeBinding {
        version: r.byte()?,
        family: r.byte()?,
        normalizer: r.byte()?,
        execution: r.byte()?,
        first_target_weight_bits: u64_read(r)?,
        span_alpha_bits: r.opt(u64_read)?,
        annotation: r.opt(digest)?,
        train_order: digest(r)?,
        tokenizer: digest(r)?,
        framing: digest(r)?,
        config: digest(r)?,
        corpus: digest(r)?,
        validation: digest(r)?,
        policy: digest(r)?,
        provenance: digest(r)?,
    };
    if digest(r)? != s.digest() {
        return Err(bad("objective descriptor digest"));
    }
    Ok(s)
}
fn tokenizer_encode(b: &mut Vec<u8>, tok: &ByteBpe) {
    put_varint(b, 1); // mapping/segmentation schema: reserved 0..7, each ASCII digit/separator is a segment.
    text(b, &tok.train_hash);
    text(b, &tok.id());
    text(b, &tok.semantic_id());
    let (tokens, merges) = tok.mapping();
    put_varint(b, tokens.len() as u64);
    for token in tokens {
        put_bytes(b, token);
    }
    put_varint(b, merges.len() as u64);
    for &(a, c) in merges {
        put_varint(b, u64::from(a));
        put_varint(b, u64::from(c));
    }
}
pub fn tokenizer_metadata_bytes(tok: &ByteBpe) -> usize {
    let mut bytes = Vec::new();
    tokenizer_encode(&mut bytes, tok);
    bytes.len()
}
fn tokenizer_decode(r: &mut Reader<'_>) -> Result<ByteBpe> {
    if r.var()? != 1 {
        return Err(bad("unsupported tokenizer semantics"));
    }
    let train = digest_text(r)?;
    let wire = digest_text(r)?;
    let semantic = digest_text(r)?;
    let n = number(r, 4096)?;
    if n < 264 {
        return Err(bad("tokenizer vocabulary"));
    }
    let tokens = (0..n).map(|_| r.bytes(32)).collect::<Result<_>>()?;
    let count = number(r, 4096 - 264)?;
    let merges = (0..count)
        .map(|_| Ok((number(r, n - 1)? as u32, number(r, n - 1)? as u32)))
        .collect::<Result<_>>()?;
    let tok = ByteBpe::from_mapping(tokens, merges, train, wire)?;
    if tok.semantic_id() != semantic {
        return Err(bad("tokenizer semantic digest"));
    }
    Ok(tok)
}
fn tensor_digest(entries: &[Entry]) -> String {
    let mut h = Sha256::new();
    for e in entries {
        h.update((e.name.len() as u64).to_le_bytes());
        h.update(e.name.as_bytes());
        h.update(e.digest);
    }
    format!("{:x}", h.finalize())
}
fn header_encode(
    m: &Manifest,
    tok: &ByteBpe,
    model_id: &str,
    entries: &[Entry],
) -> Result<Vec<u8>> {
    let mut b = Vec::new();
    for s in [
        MIXER.family_id,
        MIXER.equation_version,
        MIXER.parameter_schema_id,
        MIXER.state_schema_id,
        MIXER.numeric_policy,
    ] {
        text(&mut b, s);
    }
    config_encode(&mut b, &m.architecture);
    text(&mut b, &m.architecture.semantic_id()?);
    text(&mut b, model_id);
    for s in [&m.source_id, &m.initial_weight_hash, &m.status] {
        text(&mut b, s);
    }
    put_varint(&mut b, m.init_seed);
    put_varint(&mut b, m.trained_steps as u64);
    b.push(u8::from(m.diagnostic_only));
    b.push(u8::from(m.legacy_identity.is_some()));
    if let Some(old) = &m.legacy_identity {
        for s in [
            &old.config_json_sha256,
            &old.tokenizer_json_sha256,
            &old.tensor_file_sha256,
        ] {
            text(&mut b, s);
        }
    }
    tokenizer_encode(&mut b, tok);
    b.push(u8::from(m.training.is_some()));
    if let Some(s) = &m.training {
        state_encode(&mut b, s);
    }
    put_varint(&mut b, entries.len() as u64);
    for e in entries {
        text(&mut b, &e.name);
        b.push(1); // F32
        put_varint(&mut b, e.shape.len() as u64);
        for &d in &e.shape {
            put_varint(&mut b, d as u64);
        }
        b.extend_from_slice(&e.offset.to_le_bytes());
        b.extend_from_slice(&e.length.to_le_bytes());
        b.push(0); // raw tensor codec
        b.extend_from_slice(&e.digest);
    }
    if b.len() > MAX_HEADER {
        return Err(bad("header size"));
    }
    Ok(b)
}
fn read_header(file: &mut File) -> Result<Header> {
    let total = file.metadata()?.len();
    if !(PREFIX as u64..=MAX_FILE).contains(&total) {
        return Err(bad("total size"));
    }
    file.seek(SeekFrom::Start(0))?;
    let mut prefix = [0u8; PREFIX];
    file.read_exact(&mut prefix)?;
    let wire = u16::from_le_bytes(prefix[8..10].try_into().expect("fixed"));
    if &prefix[..8] != MAGIC
        || ![1, WIRE_VERSION].contains(&wire)
        || prefix[11] != 0
        || prefix[56..].iter().any(|&b| b != 0)
    {
        return Err(bad("magic/version/flags/endianness"));
    }
    let kind = ArtifactKind::from_tag(prefix[10])?;
    let n = u32::from_le_bytes(prefix[12..16].try_into().expect("fixed")) as usize;
    if n > MAX_HEADER
        || n as u64 + PREFIX as u64 > total
        || u64::from_le_bytes(prefix[16..24].try_into().expect("fixed")) != total
    {
        return Err(bad("header/total length/trailing bytes"));
    }
    let mut bytes = vec![0; n];
    file.read_exact(&mut bytes)?;
    if Sha256::digest(&bytes)[..] != prefix[24..56] {
        return Err(bad("header checksum"));
    }
    let mut r = Reader::new(&bytes);
    for expected in [
        MIXER.family_id,
        MIXER.equation_version,
        MIXER.parameter_schema_id,
        MIXER.state_schema_id,
        MIXER.numeric_policy,
    ] {
        if r.text()? != expected {
            return Err(Error::Invalid(
                "unsupported model equation/parameter/state/numeric schema".into(),
            ));
        }
    }
    let config = config_decode(&mut r)?;
    if digest_text(&mut r)? != config.semantic_id()? {
        return Err(bad("architecture identity"));
    }
    let model_id = digest_text(&mut r)?;
    let source_id = digest_text(&mut r)?;
    let initial_weight_hash = digest_text(&mut r)?;
    let status = r.text()?;
    let init_seed = r.var()?;
    let trained_steps = number(&mut r, i32::MAX as usize)?;
    let diagnostic_only = r.bool()?;
    let legacy_identity = r.opt(|r| {
        Ok(LegacyIdentity {
            config_json_sha256: digest_text(r)?,
            tokenizer_json_sha256: digest_text(r)?,
            tensor_file_sha256: digest_text(r)?,
        })
    })?;
    let tokenizer = tokenizer_decode(&mut r)?;
    if legacy_identity
        .as_ref()
        .is_some_and(|old| old.tokenizer_json_sha256 != tokenizer.id())
    {
        return Err(bad("legacy tokenizer migration binding"));
    }
    let training = r.opt(|r| state_decode(r, wire))?;
    if (kind == ArtifactKind::Resume) != training.is_some()
        || training
            .as_ref()
            .is_some_and(|s| s.step != trained_steps || (s.contrast16 && !diagnostic_only))
    {
        return Err(bad("artifact kind/step/purpose"));
    }
    let mut manifest = Manifest {
        version: 1,
        architecture: config,
        tokenizer_sha256: tokenizer.id(),
        weights_sha256: String::new(),
        weights_bytes: 0,
        tensors: BTreeMap::new(),
        dtype: "F32".into(),
        source_id,
        init_seed,
        initial_weight_hash,
        status,
        training,
        trained_steps,
        diagnostic_only,
        legacy_identity,
        model_content_digest: model_id.clone(),
    };
    let expected = checkpoint::expected(&manifest);
    let directory_start = r.position();
    let count = number(&mut r, 204)?;
    if count != expected.len() {
        return Err(bad("tensor count"));
    }
    let header_end = (PREFIX + n) as u64;
    let mut next = align(header_end)?;
    let mut entries = Vec::with_capacity(count);
    for _ in 0..count {
        let name = r.text()?;
        if r.byte()? != 1 {
            return Err(bad("unknown tensor dtype"));
        }
        let rank = number(&mut r, 3)?;
        if rank == 0 {
            return Err(bad("tensor rank"));
        }
        let shape = (0..rank)
            .map(|_| number(&mut r, 4096))
            .collect::<Result<Vec<_>>>()?;
        let offset = u64_read(&mut r)?;
        let length = u64_read(&mut r)?;
        if r.byte()? != 0 {
            return Err(bad("tensor codec"));
        }
        let digest = r.take(32)?.try_into().expect("checked");
        let product = shape
            .iter()
            .try_fold(4u64, |a, &b| a.checked_mul(b as u64))
            .ok_or_else(|| bad("shape overflow"))?;
        if expected.get(&name) != Some(&shape)
            || length != product
            || offset != next
            || offset.checked_add(length).is_none_or(|end| end > total)
            || manifest
                .tensors
                .insert(name.clone(), shape.clone())
                .is_some()
            || entries.last().is_some_and(|e: &Entry| e.name >= name)
        {
            return Err(bad("tensor name/shape/offset/overlap/order"));
        }
        next = align(offset + length)?;
        manifest.weights_bytes = manifest
            .weights_bytes
            .checked_add(length as usize)
            .ok_or_else(|| bad("tensor size overflow"))?;
        entries.push(Entry {
            name,
            shape,
            offset,
            length,
            digest,
        });
    }
    if !r.finished() || entries.last().is_none_or(|e| e.offset + e.length != total) {
        return Err(bad("trailing header/body"));
    }
    manifest.weights_sha256 = tensor_digest(&entries);
    checkpoint::validate_metadata(&manifest, &tokenizer)?;
    Ok(Header {
        kind,
        manifest,
        tokenizer,
        entries,
        model_id,
        header_end,
        total,
        directory_bytes: (n - directory_start) as u64,
    })
}
fn read_tensors(
    file: &mut File,
    h: &Header,
    resume: bool,
    device: &Device,
) -> Result<(TensorMap, TensorMap, LoadStats)> {
    if resume && h.kind != ArtifactKind::Resume {
        return Err(Error::Invalid(
            "INFERENCE artifact cannot resume: Adam/state absent".into(),
        ));
    }
    let mut model = BTreeMap::new();
    let mut optimizer = BTreeMap::new();
    let mut stats = LoadStats {
        header_bytes: h.header_end,
        directory_bytes: h.directory_bytes,
        ..Default::default()
    };
    let mut previous = h.header_end;
    for e in &h.entries {
        let pad = (e.offset - previous) as usize;
        if pad > 63 {
            return Err(bad("alignment padding"));
        }
        file.seek(SeekFrom::Start(previous))?;
        let mut padding = [0u8; 63];
        file.read_exact(&mut padding[..pad])?;
        stats.padding_bytes += pad as u64;
        if padding[..pad].iter().any(|&b| b != 0) {
            return Err(bad("nonzero alignment padding"));
        }
        previous = e.offset + e.length;
        let model_name = e.name.strip_prefix("model.");
        if model_name.is_none() && !resume {
            continue;
        } // never read optimizer payload in inference view.
        file.seek(SeekFrom::Start(e.offset))?;
        let mut bytes = vec![0; e.length as usize];
        let phase = Instant::now();
        file.read_exact(&mut bytes)?;
        stats.tensor_read_ms += phase.elapsed().as_secs_f64() * 1000.;
        let phase = Instant::now();
        if Sha256::digest(&bytes)[..] != e.digest {
            return Err(bad("tensor checksum"));
        }
        stats.tensor_hash_ms += phase.elapsed().as_secs_f64() * 1000.;
        let phase = Instant::now();
        let values = bytes
            .as_chunks::<4>()
            .0
            .iter()
            .map(|b| f32::from_le_bytes(*b))
            .collect::<Vec<_>>();
        if values.iter().any(|x| !x.is_finite()) {
            return Err(bad("nonfinite tensor"));
        }
        let tensor = Tensor::from_vec(values, e.shape.as_slice(), device)?;
        stats.materialize_ms += phase.elapsed().as_secs_f64() * 1000.;
        if let Some(name) = model_name {
            stats.model_bytes += e.length;
            model.insert(name.into(), tensor);
        } else {
            stats.optimizer_bytes += e.length;
            optimizer.insert(e.name.clone(), tensor);
        }
    }
    Ok((model, optimizer, stats))
}
pub fn metadata(path: &Path) -> Result<(Manifest, ByteBpe)> {
    if delta_file(path)?{let l=decode_delta(&delta_record(path)?,Device::Cpu,false)?;return Ok((l.manifest,l.tokenizer));}
    let h = read_header(&mut File::open(path)?)?;
    Ok((h.manifest, h.tokenizer))
}
pub fn load_with_stats(path: &Path, device: Device, resume: bool) -> Result<(Loaded, LoadStats)> {
    let start = Instant::now();
    if delta_file(path)?{let r=delta_record(path)?;let l=decode_delta(&r,device,resume)?;return Ok((l,LoadStats{ready_ms:start.elapsed().as_secs_f64()*1000.,header_bytes:std::fs::metadata(path)?.len(),..Default::default()}));}
    let mut file = File::open(path)?;
    let h = read_header(&mut file)?;
    let (weights, optimizer, mut stats) = read_tensors(&mut file, &h, resume, &device)?;
    let mut model = Transformer::from_tensors(h.manifest.architecture.clone(), weights, device)?;
    if model.weights_content_id()? != h.model_id {
        return Err(bad("model content identity"));
    }
    model.bind_tokenizer(&h.tokenizer.semantic_id())?;
    stats.ready_ms = start.elapsed().as_secs_f64() * 1000.;
    Ok((
        Loaded {
            model,
            tokenizer: h.tokenizer,
            manifest: h.manifest,
            optimizer,
        },
        stats,
    ))
}
pub fn load(path: &Path, device: Device, resume: bool) -> Result<Loaded> {
    load_with_stats(path, device, resume).map(|(loaded, _)| loaded)
}
fn tensor_bytes(t: &Tensor) -> Result<Vec<u8>> {
    if t.dtype() != candle_core::DType::F32 {
        return Err(bad("export requires F32"));
    }
    let values = t.flatten_all()?.to_vec1::<f32>()?;
    if values.iter().any(|x| !x.is_finite()) {
        return Err(bad("export nonfinite tensor"));
    }
    Ok(values.iter().flat_map(|x| x.to_le_bytes()).collect())
}
pub fn save(
    path: &Path,
    model: &Transformer,
    tokenizer: &ByteBpe,
    manifest: Manifest,
    optimizer: &BTreeMap<String, Tensor>,
) -> Result<Manifest> {
    save_with_stats(path, model, tokenizer, manifest, optimizer).map(|(m, _)| m)
}
pub fn save_with_stats(
    path: &Path,
    model: &Transformer,
    tokenizer: &ByteBpe,
    mut manifest: Manifest,
    optimizer: &BTreeMap<String, Tensor>,
) -> Result<(Manifest, SaveStats)> {
    // Publication must not certify merely enqueued device work.
    model.device.synchronize()?;
    if model.adapter().is_some(){return save_delta(path,model,tokenizer,manifest,optimizer);}
    let start = Instant::now();
    let mut stats = SaveStats::default();
    if let Some(s) = &manifest.training {
        if s.resume_binding.is_none() {
            return Err(bad(
                "LEGACY_OBJECTIVE_UNKNOWN: native RESUME v2 requires verified objective binding",
            ));
        }
        manifest.trained_steps = s.step;
        manifest.diagnostic_only |= s.contrast16;
    }
    manifest.architecture = model.config.clone();
    manifest.tokenizer_sha256 = tokenizer.id();
    let kind = if manifest.training.is_some() {
        ArtifactKind::Resume
    } else {
        ArtifactKind::Inference
    };
    let mut tensors: BTreeMap<String, Tensor> = model
        .vars
        .iter()
        .map(|(n, v)| (format!("model.{n}"), v.as_detached_tensor()))
        .collect();
    for (n, t) in optimizer {
        if tensors.insert(n.clone(), t.detach()).is_some() {
            return Err(bad("duplicate export tensor"));
        }
    }
    manifest.tensors = tensors
        .iter()
        .map(|(n, t)| (n.clone(), t.dims().to_vec()))
        .collect();
    checkpoint::validate_metadata(&manifest, tokenizer)?;
    let mut entries = Vec::with_capacity(tensors.len());
    for (name, t) in &tensors {
        let bytes = tensor_bytes(t)?;
        entries.push(Entry {
            name: name.clone(),
            shape: t.dims().to_vec(),
            offset: 0,
            length: bytes.len() as u64,
            digest: Sha256::digest(&bytes).into(),
        });
    }
    let model_id = model.weights_content_id()?;
    manifest.model_content_digest = model_id.clone();
    let provisional = header_encode(&manifest, tokenizer, &model_id, &entries)?;
    let mut next = align((PREFIX + provisional.len()) as u64)?;
    for e in &mut entries {
        e.offset = next;
        next = align(next + e.length)?;
    }
    let total = entries.last().ok_or_else(|| bad("empty model"))?.offset
        + entries.last().expect("nonempty").length;
    if total > MAX_FILE {
        return Err(bad("export file budget"));
    }
    let header = header_encode(&manifest, tokenizer, &model_id, &entries)?;
    if header.len() != provisional.len() {
        return Err(bad("directory offset width"));
    }
    let mut prefix = [0u8; PREFIX];
    prefix[..8].copy_from_slice(MAGIC);
    prefix[8..10].copy_from_slice(&WIRE_VERSION.to_le_bytes());
    prefix[10] = kind.tag();
    prefix[12..16].copy_from_slice(&(header.len() as u32).to_le_bytes());
    prefix[16..24].copy_from_slice(&total.to_le_bytes());
    prefix[24..56].copy_from_slice(&Sha256::digest(&header));
    stats.encode_hash_ms = start.elapsed().as_secs_f64() * 1000.;
    let (manifest, timing) = publish_new_measured(path, |file, _temporary| {
        let phase = Instant::now();
        file.write_all(&prefix)?;
        file.write_all(&header)?;
        let mut position = (PREFIX + header.len()) as u64;
        for e in &entries {
            file.write_all(&[0u8; 63][..(e.offset - position) as usize])?;
            let bytes = tensor_bytes(&tensors[&e.name])?;
            if Sha256::digest(&bytes)[..] != e.digest {
                return Err(bad("source changed during export"));
            }
            file.write_all(&bytes)?;
            position = e.offset + e.length;
        }
        #[cfg(feature = "test-support")]
        crate::store::test_pause("artifact_before_publish");
        stats.tensor_write_ms = phase.elapsed().as_secs_f64() * 1000.;
        let phase = Instant::now();
        let verified = read_header(file)?;
        // Full readback validates every payload before publication, including Adam.
        let _ = read_tensors(file, &verified, kind == ArtifactKind::Resume, &Device::Cpu)?;
        if verified.total != total {
            return Err(bad("export total"));
        }
        stats.readback_ms = phase.elapsed().as_secs_f64() * 1000.;
        Ok(verified.manifest)
    })?;
    stats.publication = timing;
    stats.total_ms = start.elapsed().as_secs_f64() * 1000.;
    Ok((manifest, stats))
}
pub fn export_inference(path: &Path, loaded: &Loaded) -> Result<Manifest> {
    // This inference schema drops training/binding; it cannot preserve opt-in EQ.
    loaded.manifest.require_default_framing()?;
    let mut manifest = loaded.manifest.clone();
    manifest.trained_steps = manifest
        .training
        .as_ref()
        .map_or(manifest.trained_steps, |s| s.step);
    manifest.diagnostic_only |= manifest.training.as_ref().is_some_and(|s| s.contrast16);
    manifest.training = None;
    save(
        path,
        &loaded.model,
        &loaded.tokenizer,
        manifest,
        &BTreeMap::new(),
    )
}
pub fn import_legacy(source: &Path, output: &Path, kind: ArtifactKind) -> Result<Manifest> {
    let mut loaded = checkpoint::legacy_load(source, Device::Cpu, kind == ArtifactKind::Resume)?;
    // Legacy format carries no independently verified quality gate. Conversion cannot promote it.
    loaded.manifest.diagnostic_only = true;
    match kind {
        ArtifactKind::Inference => export_inference(output, &loaded),
        ArtifactKind::Resume => {
            if loaded.manifest.training.is_none() {
                return Err(Error::Invalid("legacy source has no resume state".into()));
            }
            save(
                output,
                &loaded.model,
                &loaded.tokenizer,
                loaded.manifest,
                &loaded.optimizer,
            )
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    // Independent wire writer used only for this literal fixture. Never calls the production encoder.
    fn uint(b: &mut Vec<u8>, mut n: u64) {
        while n >= 128 {
            b.push(n as u8 | 128);
            n >>= 7;
        }
        b.push(n as u8);
    }
    fn blob(b: &mut Vec<u8>, bytes: &[u8]) {
        uint(b, bytes.len() as u64);
        b.extend_from_slice(bytes);
    }
    fn str_field(b: &mut Vec<u8>, s: &str) {
        blob(b, s.as_bytes());
    }
    fn fixture() -> (Vec<u8>, Vec<usize>, usize) {
        let mut h = Vec::new();
        let specs = [
            "native-trpp-gqa",
            "prerms-qknorm-rope-causal-local-global-swiglu-tied-v1",
            "native-trpp-parameters-v1",
            "absolute-kv-history-v1",
            "cpu-f32-reference",
        ];
        for s in specs {
            str_field(&mut h, s);
        }
        str_field(&mut h, "NATIVE_TRPP_EXPERIMENTAL_V1");
        let config_literal = [0x88, 0x02, 1, 2, 1, 1, 2, 3, 0, 1, 2]; // vocab264,layers1,H2,Q1,KV1,D2,FF3,local0,W1,C2
        h.extend_from_slice(&config_literal);
        h.extend_from_slice(&1e-6f64.to_le_bytes());
        h.extend_from_slice(&10000f64.to_le_bytes());
        let mut semantic = Vec::new();
        for s in specs {
            str_field(&mut semantic, s);
        }
        semantic.extend_from_slice(&config_literal);
        semantic.extend_from_slice(&1e-6f64.to_le_bytes());
        semantic.extend_from_slice(&10000f64.to_le_bytes());
        str_field(&mut h, &super::super::hash(&semantic));
        let tensors: [(&str, &[usize]); 13] = [
            ("embedding", &[264, 2]),
            ("final_norm", &[2]),
            ("layer.0.attn_norm", &[2]),
            ("layer.0.down", &[2, 3]),
            ("layer.0.ffn_norm", &[2]),
            ("layer.0.gate", &[3, 2]),
            ("layer.0.k", &[2, 2]),
            ("layer.0.k_norm", &[1, 1, 2]),
            ("layer.0.o", &[2, 2]),
            ("layer.0.q", &[2, 2]),
            ("layer.0.q_norm", &[1, 1, 2]),
            ("layer.0.up", &[3, 2]),
            ("layer.0.v", &[2, 2]),
        ];
        let mut model_digest = Sha256::new();
        let mut payloads = Vec::new();
        for (name, shape) in tensors {
            let value = if name.ends_with("norm") { 1f32 } else { 0f32 };
            let values: Vec<u8> = (0..shape.iter().product::<usize>())
                .flat_map(|_| value.to_le_bytes())
                .collect();
            model_digest.update((name.len() as u64).to_le_bytes());
            model_digest.update(name.as_bytes());
            model_digest.update((shape.len() as u64).to_le_bytes());
            for &n in shape {
                model_digest.update((n as u64).to_le_bytes());
            }
            model_digest.update(&values);
            payloads.push(values);
        }
        str_field(&mut h, &format!("{:x}", model_digest.finalize()));
        for s in [&"1".repeat(64), &"2".repeat(64), "RANDOM_INITIALIZED"] {
            str_field(&mut h, s);
        }
        h.extend_from_slice(&[7, 0, 0, 0]); // init_seed7,trained_steps0,diagnostic false,migration absent
        uint(&mut h, 1);
        str_field(&mut h, &"3".repeat(64));
        str_field(&mut h, &"4".repeat(64));
        let mut toksemantic = b"native-byte-bpe-special8-ascii-segments-v1".to_vec();
        uint(&mut toksemantic, 264);
        toksemantic.extend_from_slice(&[0; 8]);
        for b in 0..=255u8 {
            toksemantic.extend_from_slice(&[1, b]);
        }
        toksemantic.push(0);
        str_field(&mut h, &super::super::hash(&toksemantic));
        uint(&mut h, 264);
        h.extend_from_slice(&[0; 8]);
        for b in 0..=255u8 {
            h.extend_from_slice(&[1, b]);
        }
        let merges_at = h.len();
        h.push(0);
        h.push(0);
        h.push(13); // zero merges, no training,13 tensors
        let mut entries_at = Vec::new();
        let mut offsets_at = Vec::new();
        for ((name, shape), payload) in tensors.iter().zip(&payloads) {
            entries_at.push(h.len());
            str_field(&mut h, &format!("model.{name}"));
            h.push(1);
            uint(&mut h, shape.len() as u64);
            for &n in *shape {
                uint(&mut h, n as u64);
            }
            offsets_at.push(h.len());
            h.extend_from_slice(&0u64.to_le_bytes());
            h.extend_from_slice(&(payload.len() as u64).to_le_bytes());
            h.push(0);
            h.extend_from_slice(&Sha256::digest(payload));
        }
        let mut file = vec![0u8; 64 + h.len()];
        let mut cursor = file.len();
        for (&at, payload) in offsets_at.iter().zip(&payloads) {
            while !cursor.is_multiple_of(64) {
                file.push(0);
                cursor += 1;
            }
            h[at..at + 8].copy_from_slice(&(cursor as u64).to_le_bytes());
            file.extend_from_slice(payload);
            cursor += payload.len();
        }
        // Literal prefix bytes: native magic, version1, inference1, flags0, reserved all zero.
        file[..12].copy_from_slice(&[82, 51, 77, 79, 68, 69, 76, 0, 1, 0, 1, 0]);
        file[12..16].copy_from_slice(&(h.len() as u32).to_le_bytes());
        file[16..24].copy_from_slice(&(cursor as u64).to_le_bytes());
        file[24..56].copy_from_slice(&Sha256::digest(&h));
        file[64..64 + h.len()].copy_from_slice(&h);
        (
            file,
            entries_at.into_iter().map(|n| n + 64).collect(),
            merges_at + 64,
        )
    }
    fn checksum(bytes: &mut [u8]) {
        let n = u32::from_le_bytes(bytes[12..16].try_into().unwrap()) as usize;
        let digest = Sha256::digest(&bytes[64..64 + n]);
        bytes[24..56].copy_from_slice(&digest);
    }
    #[test]
    fn literal_native_fixture_and_hand_calculated_config_encoding() {
        let (bytes, _, _) = fixture();
        let d = tempfile::tempdir().unwrap();
        let path = d.path().join("literal");
        std::fs::write(&path, &bytes).unwrap();
        let (loaded, stats) = load_with_stats(&path, Device::Cpu, false).unwrap();
        assert_eq!(loaded.model.config.parameters(), 572);
        assert_eq!(stats.model_bytes, 572 * 4);
        assert_eq!(stats.optimizer_bytes, 0);
        assert!(load(&path, Device::Cpu, true).is_err());
        let raw: Vec<u8> = (0..=255).collect();
        let ids = loaded.tokenizer.encode(&raw).unwrap();
        assert_eq!(ids, (8..264).collect::<Vec<_>>()); // all 256 bytes + 8, no normalization.
        assert_eq!(loaded.tokenizer.decode_bytes(&ids).unwrap(), raw);
        let mut actual = Vec::new();
        config_encode(&mut actual, &loaded.model.config);
        let mut expected =
            b"\x1bNATIVE_TRPP_EXPERIMENTAL_V1\x88\x02\x01\x02\x01\x01\x02\x03\x00\x01\x02".to_vec();
        expected.extend_from_slice(&1e-6f64.to_le_bytes());
        expected.extend_from_slice(&10000f64.to_le_bytes());
        assert_eq!(actual, expected);
        let output = d.path().join("roundtrip");
        save(
            &output,
            &loaded.model,
            &loaded.tokenizer,
            loaded.manifest,
            &BTreeMap::new(),
        )
        .unwrap();
        let mut expected_v2 = bytes;
        expected_v2[8..10].copy_from_slice(&2u16.to_le_bytes());
        checksum(&mut expected_v2);
        assert_eq!(std::fs::read(output).unwrap(), expected_v2); // v1 inference body unchanged; only explicit version and checksum differ.
    }
    #[test]
    fn native_container_rejects_independently_corrupted_fields() {
        let (good, entries, merges) = fixture();
        let d = tempfile::tempdir().unwrap();
        let path = d.path().join("bad");
        for case in [
            "kind",
            "equation",
            "state_schema",
            "flags",
            "version",
            "header_checksum",
            "oversized_header",
            "truncated",
            "trailing",
            "dtype",
            "duplicate",
            "shape",
            "overlap",
            "merge",
            "payload",
            "nonfinite",
        ] {
            let mut b = good.clone();
            match case {
                "kind" => b[10] = 9,
                "equation" | "state_schema" => {
                    let text: &[u8] = if case == "equation" {
                        b"prerms-qknorm-rope-causal-local-global-swiglu-tied-v1"
                    } else {
                        b"absolute-kv-history-v1"
                    };
                    let at = b.windows(text.len()).position(|v| v == text).unwrap();
                    b[at] = b'X';
                    checksum(&mut b);
                }
                "flags" => b[11] = 1,
                "version" => b[8] = 3,
                "header_checksum" => b[24] ^= 1,
                "oversized_header" => b[12..16].copy_from_slice(&u32::MAX.to_le_bytes()),
                "truncated" => {
                    b.pop();
                }
                "trailing" => b.push(0),
                "dtype" => {
                    let at = entries[0];
                    let len = b[at] as usize;
                    b[at + 1 + len] = 2;
                    checksum(&mut b);
                }
                "duplicate" => {
                    let at = entries[6];
                    let len = b[at] as usize;
                    b[at + len] = b'q';
                    checksum(&mut b);
                }
                "shape" => {
                    let at = entries[0];
                    let len = b[at] as usize;
                    b[at + len + 3] = 0xff;
                    b[at + len + 4] = 0x7f;
                    checksum(&mut b);
                }
                "overlap" => {
                    let at = entries[1];
                    let len = b[at] as usize;
                    let offset = at + 1 + len + 1 + 1 + 1;
                    b[offset..offset + 8].copy_from_slice(&64u64.to_le_bytes());
                    checksum(&mut b);
                }
                "merge" => {
                    b[merges] = 1;
                    checksum(&mut b);
                }
                "payload" => {
                    let last = b.len() - 1;
                    b[last] ^= 1;
                }
                "nonfinite" => {
                    let at = entries[0];
                    let len = b[at] as usize;
                    let offset_at = at + 1 + len + 1 + 1 + 3;
                    let offset = u64::from_le_bytes(b[offset_at..offset_at + 8].try_into().unwrap())
                        as usize;
                    let length =
                        u64::from_le_bytes(b[offset_at + 8..offset_at + 16].try_into().unwrap())
                            as usize;
                    b[offset..offset + 4].copy_from_slice(&f32::NAN.to_le_bytes());
                    let digest = Sha256::digest(&b[offset..offset + length]);
                    b[offset_at + 17..offset_at + 49].copy_from_slice(&digest);
                    checksum(&mut b);
                }
                _ => unreachable!(),
            }
            std::fs::write(&path, b).unwrap();
            assert!(load(&path, Device::Cpu, false).is_err(), "{case}");
        }
    }
    #[test]
    fn inference_view_never_reads_adam_and_publication_never_clobbers() {
        let (bytes, _, _) = fixture();
        let d = tempfile::tempdir().unwrap();
        let source = d.path().join("source");
        std::fs::write(&source, &bytes).unwrap();
        let mut loaded = load(&source, Device::Cpu, false).unwrap();
        loaded.manifest.training = Some(TrainingState {
            resume_binding: None,
            contrast16: false,
            parent_checkpoint_hash: None,
            config: TrainConfig {
                max_steps: 6,
                warmup: 0,
                seq_len: 2,
                microbatch: 1,
                accumulation: 1,
                ..Default::default()
            },
            step: 3,
            consumed_tokens: 6,
            target_tokens: 3,
            sampler_state: u64::MAX - 17,
            corpus_hash: "3".repeat(64),
            validation_hash: "5".repeat(64),
            previous_corpora: Vec::new(),
            initial_weight_hash: "2".repeat(64),
            train_loss: Some(0.25),
            validation_loss: Some(0.5),
        });
        let state = loaded.manifest.training.as_mut().unwrap();
        state.resume_binding = Some(ResumeBinding::default_for(state, &loaded.tokenizer));
        ResumeBinding::require_default(state, &loaded.tokenizer).unwrap();
        let valid = state.clone();
        for change in 0..8 {
            let mut invalid = valid.clone();
            let binding = invalid.resume_binding.as_mut().unwrap();
            match change {
                0 => binding.family = 9,
                1 => binding.version = 9,
                2 => binding.policy[0] ^= 1,
                3 => binding.span_alpha_bits = Some(2f64.to_bits()),
                4 => binding.annotation = Some([9; 32]),
                5 => binding.train_order[0] ^= 1,
                6 => binding.tokenizer[0] ^= 1,
                7 => invalid.resume_binding = None,
                _ => unreachable!(),
            }
            assert!(ResumeBinding::require_default(&invalid, &loaded.tokenizer).is_err());
        }
        for (name, var) in &loaded.model.vars {
            for role in ["m", "v"] {
                loaded.optimizer.insert(
                    format!("adam.{role}.{name}"),
                    Tensor::zeros(var.shape(), candle_core::DType::F32, &Device::Cpu).unwrap(),
                );
            }
        }
        let resume = d.path().join("resume");
        save(
            &resume,
            &loaded.model,
            &loaded.tokenizer,
            loaded.manifest.clone(),
            &loaded.optimizer,
        )
        .unwrap();
        let (view, stats) = load_with_stats(&resume, Device::Cpu, false).unwrap();
        assert_eq!(stats.optimizer_bytes, 0);
        assert!(view.optimizer.is_empty());
        assert!(stats.bytes_read() < std::fs::metadata(&resume).unwrap().len());
        let (full, stats) = load_with_stats(&resume, Device::Cpu, true).unwrap();
        assert_eq!(stats.optimizer_bytes, 2 * 572 * 4);
        assert_eq!(
            full.manifest.training.as_ref().unwrap().sampler_state,
            u64::MAX - 17
        );
        // The fixed paired objective reuses the descriptor bytes, but can never
        // silently resume as default CE or lose its coefficient/annotation.
        for family in [3,4,5,super::checkpoint::ANSWER_MEAN_FAMILY] {
        let mut contrast = full.manifest.clone();
        let state = contrast.training.as_mut().unwrap();
        let binding = state.resume_binding.as_mut().unwrap();
        binding.family = family; binding.normalizer = if family==6 {2}else{family}; binding.execution = 1;
        binding.span_alpha_bits = if family==6 {None}else{Some(0.1f64.to_bits())};
        binding.annotation = if family==6 {None}else{Some([7;32])};
        let path = d.path().join(format!("paired-objective-{family}"));
        save(&path,&full.model,&full.tokenizer,contrast.clone(),&full.optimizer).unwrap();
        let restored = load(&path,Device::Cpu,true).unwrap();
        if family==6 {
            let view=load(&path,Device::Cpu,false).unwrap();
            assert!(view.optimizer.is_empty());
            assert_eq!(view.model.weights_content_id().unwrap(),restored.model.weights_content_id().unwrap());
        }
        assert_eq!(restored.manifest.training,contrast.training);
        assert!(ResumeBinding::require_default(restored.manifest.training.as_ref().unwrap(),&restored.tokenizer).is_err());
        for change in 0..5 {
            let mut invalid = contrast.training.clone().unwrap();
            let b = invalid.resume_binding.as_mut().unwrap();
            match change {
                0 => b.span_alpha_bits = Some(0.2f64.to_bits()),
                1 => b.annotation = if family==6 {Some([7;32])}else{None},
                2 => b.normalizer = if family==3 {4}else{3},
                3 => b.execution = 0,
                4 => b.first_target_weight_bits = 4f64.to_bits(),
                _ => unreachable!(),
            }
            assert!(b.clone().validate(&invalid,&full.tokenizer).is_err());
        }
        }
        let inference = d.path().join("inference");
        export_inference(&inference, &full).unwrap();
        let exported = load(&inference, Device::Cpu, false).unwrap();
        assert!(exported.manifest.training.is_none());
        assert_eq!(exported.manifest.trained_steps, 3);
        assert!(exported.optimizer.is_empty());
        assert_eq!(
            exported.model.weights_content_id().unwrap(),
            full.model.weights_content_id().unwrap()
        );
        assert!(load(&inference, Device::Cpu, true).is_err());
        // Fixed-pass diagnostics admit only the original16 or explicitly doubled32 cases.
        // The existing state schema already stores microbatch/accumulation and the corpus digest.
        for accumulation in [3, 4, 6, 8, 9] {
            let mut manifest = full.manifest.clone();
            let state = manifest.training.as_mut().unwrap();
            state.contrast16 = true;
            state.config.max_tokens = 3200;
            state.config.microbatch = 4;
            state.config.sample_group_size = 4;
            state.config.accumulation = accumulation;
            state.resume_binding = Some(ResumeBinding::default_for(state, &full.tokenizer));
            let path = d.path().join(format!("contrast-{accumulation}"));
            let result = save(
                &path,
                &full.model,
                &full.tokenizer,
                manifest.clone(),
                &full.optimizer,
            );
            if matches!(accumulation, 4 | 8) {
                result.unwrap();
                let restored = load(&path, Device::Cpu, true).unwrap();
                assert_eq!(restored.manifest.training, manifest.training);
                assert!(restored.manifest.diagnostic_only);
            } else {
                assert!(matches!(result, Err(Error::Corrupt(_))));
                assert!(!path.exists());
            }
        }
        let original = std::fs::read(&inference).unwrap();
        assert!(export_inference(&inference, &full).is_err());
        assert_eq!(std::fs::read(&inference).unwrap(), original);
        let h = read_header(&mut File::open(&resume).unwrap()).unwrap();
        let at = h
            .entries
            .iter()
            .find(|e| e.name.starts_with("adam."))
            .unwrap()
            .offset as usize;
        let mut broken = std::fs::read(&resume).unwrap();
        broken[at] ^= 1;
        std::fs::write(&resume, broken).unwrap();
        assert!(load(&resume, Device::Cpu, false).is_ok());
        assert!(load(&resume, Device::Cpu, true).is_err());
        assert_eq!(std::fs::read(&source).unwrap(), bytes);
        let absent = d.path().join("failed");
        let result: Result<()> = crate::codec::publish_new(&absent, |file, _| {
            file.write_all(b"partial")?;
            Err(bad("injected interruption before publication"))
        });
        assert!(result.is_err());
        assert!(!absent.exists());
        assert!(std::fs::read_dir(d.path()).unwrap().all(|e| {
            !e.unwrap()
                .file_name()
                .to_string_lossy()
                .ends_with("pending")
        }));
    }
}
