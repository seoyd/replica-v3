//! Train-only immutable packed samples. This derivative never replaces source episodes.
use super::*;
const TOK_HEADER: usize = 320;
const MAGIC: &[u8; 8] = b"R3TOK\0\0\0";

#[derive(Clone)]
struct Key {
    hashes: [Hash; 6],
    ordinals: Vec<u32>,
    seq: usize,
    vocab: usize,
}
struct Packed {
    bytes: Vec<u8>,
    offsets: Vec<usize>,
    starts: Vec<usize>,
    curricula: Vec<bool>,
    supported: Vec<bool>,
    spans: Vec<Vec<target_loss::Span>>,
    token_offset: usize,
    width: usize,
}
fn u32le(r: &mut Reader<'_>) -> Result<u32> {
    Ok(u32::from_le_bytes(r.take(4)?.try_into().unwrap()))
}
fn size(r: &mut Reader<'_>, max: usize) -> Result<usize> {
    let v = u64::from_le_bytes(r.take(8)?.try_into().unwrap());
    usize::try_from(v)
        .ok()
        .filter(|v| *v <= max)
        .ok_or_else(|| bad("token cache length bound"))
}
fn bools(r: &mut Reader<'_>, n: usize) -> Result<Vec<bool>> {
    r.take(n)?
        .iter()
        .map(|v| match v {
            0 => Ok(false),
            1 => Ok(true),
            _ => Err(bad("token cache flag")),
        })
        .collect()
}
fn key(s: &RunSnapshot, raw: &[u8], l: &Loaded) -> Result<Key> {
    let mut split = Vec::new();
    for i in &s.train {
        episode_encode(&s.cases[*i as usize], &mut split);
    }
    let mut ordinals = Vec::new();
    integers(&mut ordinals, &s.train);
    let seq = l
        .manifest
        .training
        .as_ref()
        .ok_or_else(|| bad("cache training config absent"))?
        .config
        .seq_len;
    let mut framing = neural::PROMPT_FORMAT.as_bytes().to_vec();
    framing.extend(include_bytes!("neural.rs"));
    framing.extend((seq as u64).to_le_bytes());
    let mut objective = target_loss::REVISION.as_bytes().to_vec();
    objective.extend(s.binding());
    Ok(Key {
        hashes: [
            hash(raw),
            hash(&split),
            hash(&ordinals),
            unhex(&l.tokenizer.semantic_id())?,
            hash(&framing),
            hash(&objective),
        ],
        ordinals: s.train.clone(),
        seq,
        vocab: l.tokenizer.vocab_size(),
    })
}
fn encode(
    key: &Key,
    samples: &[Sample],
    annotations: &[target_loss::Annotation],
) -> Result<Vec<u8>> {
    let n = samples.len();
    if n == 0
        || n > MAX_CASES
        || n != key.ordinals.len()
        || n != annotations.len()
        || key.seq > 8192
    {
        return Err(bad("token cache sample count/sequence"));
    }
    let max_id = samples
        .iter()
        .flat_map(|s| &s.tokens)
        .copied()
        .max()
        .unwrap_or(0);
    let width = if key.vocab <= 65536 && max_id <= u16::MAX.into() {
        2
    } else {
        4
    };
    let mut offsets = vec![0u64];
    let mut span_offsets = vec![0u64];
    for (s, a) in samples.iter().zip(annotations) {
        offsets.push(
            offsets
                .last()
                .unwrap()
                .checked_add(s.tokens.len() as u64)
                .ok_or_else(|| bad("token count overflow"))?,
        );
        span_offsets.push(
            span_offsets
                .last()
                .unwrap()
                .checked_add(a.spans.len() as u64)
                .ok_or_else(|| bad("span count overflow"))?,
        );
    }
    if *offsets.last().unwrap() > (MAX_FILE / width as usize) as u64
        || *span_offsets.last().unwrap() > (n * 32) as u64
    {
        return Err(bad("token cache allocation bound"));
    }
    let mut b = vec![0; TOK_HEADER];
    for v in &offsets {
        b.extend(v.to_le_bytes());
    }
    for s in samples {
        b.extend(
            u32::try_from(s.response_start)
                .map_err(|_| bad("response offset overflow"))?
                .to_le_bytes(),
        );
    }
    for v in &key.ordinals {
        b.extend(v.to_le_bytes());
    }
    b.extend(samples.iter().map(|s| u8::from(s.curriculum)));
    b.extend(annotations.iter().map(|a| u8::from(a.supported)));
    for v in &span_offsets {
        b.extend(v.to_le_bytes());
    }
    let token_offset = b.len();
    for s in samples {
        for &id in &s.tokens {
            if width == 2 {
                b.extend(
                    u16::try_from(id)
                        .map_err(|_| bad("u16 token overflow"))?
                        .to_le_bytes(),
                );
            } else {
                b.extend(id.to_le_bytes());
            }
        }
    }
    let span_offset = b.len();
    for a in annotations {
        for span in &a.spans {
            b.extend(span.start.to_le_bytes());
            b.extend(span.end.to_le_bytes());
            b.push(span.role);
        }
    }
    if b.len() > MAX_FILE {
        return Err(bad("token cache file bound"));
    }
    let mut header = MAGIC.to_vec();
    header.extend(1u16.to_le_bytes());
    header.extend([1, width]);
    header.extend((TOK_HEADER as u32).to_le_bytes());
    header.extend((b.len() as u64).to_le_bytes());
    header.extend((n as u32).to_le_bytes());
    header.extend((key.seq as u32).to_le_bytes());
    header.extend(offsets.last().unwrap().to_le_bytes());
    for v in [TOK_HEADER, token_offset, span_offset] {
        header.extend((v as u64).to_le_bytes());
    }
    for h in key.hashes {
        header.extend(h);
    }
    header.extend(hash(&b[TOK_HEADER..]));
    header.extend(hash(&header));
    if header.len() != TOK_HEADER {
        return Err(bad("token cache header size"));
    }
    b[..TOK_HEADER].copy_from_slice(&header);
    decode(b.clone(), key)?;
    Ok(b)
}
fn decode(bytes: Vec<u8>, key: &Key) -> Result<Packed> {
    if bytes.len() < TOK_HEADER
        || bytes.len() > MAX_FILE
        || hash(&bytes[..288]) != bytes[288..320]
        || hash(&bytes[TOK_HEADER..]) != bytes[256..288]
    {
        return Err(bad("token cache header/body checksum"));
    }
    let mut r = Reader::new(&bytes);
    if r.take(8)? != MAGIC || r.take(2)? != 1u16.to_le_bytes() || r.byte()? != 1 {
        return Err(bad("token cache magic/version/endian"));
    }
    let width = r.byte()? as usize;
    if width != if key.vocab <= 65536 { 2 } else { 4 } {
        return Err(bad("token cache width/vocab"));
    }
    if u32le(&mut r)? as usize != TOK_HEADER || size(&mut r, MAX_FILE)? != bytes.len() {
        return Err(bad("token cache header/total"));
    }
    let n = u32le(&mut r)? as usize;
    let seq = u32le(&mut r)? as usize;
    if n == 0 || n > MAX_CASES || n != key.ordinals.len() || seq != key.seq || seq > 8192 {
        return Err(bad("token cache count/sequence"));
    }
    let total = size(&mut r, MAX_FILE / 2)?;
    let index_start = size(&mut r, MAX_FILE)?;
    let token_offset = size(&mut r, MAX_FILE)?;
    let span_offset = size(&mut r, MAX_FILE)?;
    if index_start != TOK_HEADER {
        return Err(bad("token cache index start"));
    }
    for expected in key.hashes {
        if digest_read(&mut r)? != expected {
            return Err(bad(
                "token cache source/split/ordinal/tokenizer/framing/policy mismatch",
            ));
        }
    }
    r.take(64)?;
    let offsets = (0..=n)
        .map(|_| size(&mut r, total))
        .collect::<Result<Vec<_>>>()?;
    if offsets[0] != 0
        || offsets[n] != total
        || offsets
            .windows(2)
            .any(|v| v[0] >= v[1] || v[1] - v[0] > seq + 1)
    {
        return Err(bad("token cache reverse/gap/overlap offsets"));
    }
    let starts = (0..n)
        .map(|_| u32le(&mut r).map(|v| v as usize))
        .collect::<Result<Vec<_>>>()?;
    for i in &key.ordinals {
        if u32le(&mut r)? != *i {
            return Err(bad("token cache ordinal order"));
        }
    }
    let curricula = bools(&mut r, n)?;
    let supported = bools(&mut r, n)?;
    let span_offsets = (0..=n)
        .map(|_| size(&mut r, n * 32))
        .collect::<Result<Vec<_>>>()?;
    if span_offsets[0] != 0
        || span_offsets
            .windows(2)
            .any(|v| v[0] > v[1] || v[1] - v[0] > 32)
        || r.position() != token_offset
        || token_offset.checked_add(
            total
                .checked_mul(width)
                .ok_or_else(|| bad("token size overflow"))?,
        ) != Some(span_offset)
    {
        return Err(bad("token cache body sections"));
    }
    r.take(total * width)?;
    let mut spans = Vec::with_capacity(n);
    for i in 0..n {
        let mut row = Vec::new();
        for _ in span_offsets[i]..span_offsets[i + 1] {
            let span = target_loss::Span {
                start: u32le(&mut r)?,
                end: u32le(&mut r)?,
                role: r.byte()?,
            };
            if span.start >= span.end
                || span.end as usize > MAX_TEXT
                || !(1..=5).contains(&span.role)
            {
                return Err(bad("token cache role span"));
            }
            row.push(span);
        }
        if !supported[i] && !row.is_empty() {
            return Err(bad("token cache unsupported span"));
        }
        spans.push(row);
    }
    if !r.finished() {
        return Err(bad("token cache trailing bytes"));
    }
    let packed = Packed {
        bytes,
        offsets,
        starts,
        curricula,
        supported,
        spans,
        token_offset,
        width,
    };
    for i in 0..n {
        let s = packed.sample(i)?;
        if s.tokens.len() < 3
            || s.response_start < 2
            || s.response_start >= s.tokens.len()
            || s.tokens[0] != BOS
            || s.tokens[s.response_start - 1] != neural::ASSISTANT_ROLE
            || s.tokens.last() != Some(&EOS)
            || s.tokens
                .iter()
                .any(|v| *v as usize >= key.vocab || *v == PAD)
            || s.tokens[1..s.tokens.len() - 1]
                .iter()
                .any(|v| matches!(*v, BOS | EOS))
            || s.tokens[s.response_start..s.tokens.len() - 1]
                .iter()
                .any(|v| *v < neural::SPECIALS as u32)
        {
            return Err(bad("token cache invalid ID/BOS/EOS/response boundary"));
        }
    }
    Ok(packed)
}
impl Packed {
    fn sample(&self, i: usize) -> Result<Sample> {
        if i >= self.starts.len() {
            return Err(bad("token cache sample index"));
        }
        let a = self.token_offset + self.offsets[i] * self.width;
        let b = self.token_offset + self.offsets[i + 1] * self.width;
        let tokens = self.bytes[a..b]
            .chunks_exact(self.width)
            .map(|x| {
                if self.width == 2 {
                    u16::from_le_bytes(x.try_into().unwrap()) as u32
                } else {
                    u32::from_le_bytes(x.try_into().unwrap())
                }
            })
            .collect();
        Ok(Sample {
            tokens,
            response_start: self.starts[i],
            curriculum: self.curricula[i],
        })
    }
    fn batch(&self, indices: &[usize]) -> Result<Batch> {
        let selected = indices
            .iter()
            .map(|i| self.sample(*i))
            .collect::<Result<Vec<_>>>()?;
        batch(
            &selected,
            &(0..selected.len()).collect::<Vec<_>>(),
            &Device::Cpu,
        )
    }
}
fn parity(
    packed: &Packed,
    framed: &[Sample],
    annotations: &[target_loss::Annotation],
) -> Result<()> {
    if packed.starts.len() != framed.len() || annotations.len() != framed.len() {
        return Err(bad("cache parity cardinality"));
    }
    for (i, s) in framed.iter().enumerate() {
        let actual = packed.sample(i)?;
        if actual.tokens != s.tokens
            || actual.response_start != s.response_start
            || actual.curriculum != s.curriculum
            || packed.spans[i] != annotations[i].spans
            || packed.supported[i] != annotations[i].supported
        {
            return Err(bad("cache sample/target/order/role parity"));
        }
    }
    Ok(())
}
fn same_batch(a: &Batch, b: &Batch) -> Result<()> {
    if a.tokens != b.tokens
        || a.valid != b.valid
        || a.input.dims() != b.input.dims()
        || a.input.flatten_all()?.to_vec1::<u32>()? != b.input.flatten_all()?.to_vec1::<u32>()?
        || a.target.flatten_all()?.to_vec1::<u32>()? != b.target.flatten_all()?.to_vec1::<u32>()?
        || a.mask.flatten_all()?.to_vec1::<f32>()? != b.mask.flatten_all()?.to_vec1::<f32>()?
        || a.first_target_mask.flatten_all()?.to_vec1::<f32>()?
            != b.first_target_mask.flatten_all()?.to_vec1::<f32>()?
    {
        return Err(bad("cache actual batch/shift/mask parity"));
    }
    Ok(())
}
fn inputs(root: &Path) -> Result<(RunSnapshot, Vec<u8>, Loaded)> {
    let raw = neural::read_bounded(&root.join("inputs.r3er"), MAX_FILE)?;
    let Record::Inputs(s) = Record::decode(&raw)? else {
        return Err(bad("cache requires owned snapshot"));
    };
    if s.objective.is_none() && !s.path_parity() {
        return Err(bad("cache requires explicit train role policy"));
    }
    if s.path_parity() {
        native_source(root, &s)?;
    }
    let l = resolve_native(root, &s, &s.parent, true)?;
    Ok((*s, raw, l))
}
fn framed(s: &RunSnapshot, l: &Loaded) -> Result<(Vec<Sample>, Vec<target_loss::Annotation>)> {
    let episodes = s
        .train
        .iter()
        .map(|i| s.cases[*i as usize].clone())
        .collect::<Vec<_>>();
    let f = samples(
        &episodes,
        &l.tokenizer,
        l.manifest.training.as_ref().unwrap().config.seq_len,
    )?;
    let annotations = target_loss::annotations(
        &episodes,
        &f,
        &l.tokenizer,
        s.authorization
            .as_ref()
            .map_or(&[][..], |a| a.pools[1].as_slice()),
    )?;
    if s.objective
        .as_ref()
        .is_some_and(|o| hash(&target_loss::annotation_bytes(&annotations)) != o.annotation)
    {
        return Err(bad("cache annotation identity"));
    }
    Ok((f, annotations))
}
pub(super) fn load_samples(
    root: &Path,
    s: &RunSnapshot,
    l: &Loaded,
    cache: &Path,
) -> Result<Vec<Sample>> {
    let raw = neural::read_bounded(&root.join("inputs.r3er"), MAX_FILE)?;
    let key = key(s, &raw, l)?;
    let (_lock, hashes) = receipt(cache)?;
    let bytes = neural::read_bounded(&cache.join("train.r3tok"), MAX_FILE)?;
    if hash(&bytes) != hashes[0] {
        return Err(bad("cache physical receipt"));
    }
    let packed = decode(bytes, &key)?;
    let annotations = packed.annotations(s, &l.tokenizer)?;
    if s.objective
        .as_ref()
        .is_some_and(|o| hash(&target_loss::annotation_bytes(&annotations)) != o.annotation)
    {
        return Err(bad("cache objective annotation"));
    }
    (0..packed.starts.len()).map(|i| packed.sample(i)).collect()
}
fn write_bytes(path: &Path, bytes: &[u8]) -> Result<()> {
    publish_new(path, |f, _| {
        f.write_all(bytes)?;
        Ok(())
    })
}
fn decompress(bytes: &[u8]) -> Result<Vec<u8>> {
    let decoder = zstd::stream::read::Decoder::new(bytes)?;
    let mut raw = Vec::new();
    decoder.take(MAX_FILE as u64 + 1).read_to_end(&mut raw)?;
    if raw.len() > MAX_FILE {
        return Err(bad("token cache compressed expansion bound"));
    }
    Ok(raw)
}
pub(super) fn compile(root: &Path, output: &Path, control: &mut RunControl) -> Result<()> {
    control.check("token_cache_compile_start")?;
    let started = Instant::now();
    let (s, raw, l) = inputs(root)?;
    let setup = started.elapsed().as_secs_f64();
    let started = Instant::now();
    let key = key(&s, &raw, &l)?;
    let (f, annotations) = framed(&s, &l)?;
    let tokenize = started.elapsed().as_secs_f64();
    let started = Instant::now();
    let bytes = encode(&key, &f, &annotations)?;
    let packed = decode(bytes.clone(), &key)?;
    parity(&packed, &f, &annotations)?;
    for draw in s.tape.iter().take(5) {
        let ids = draw.indices.iter().map(|i| *i as usize).collect::<Vec<_>>();
        same_batch(&batch(&f, &ids, &Device::Cpu)?, &packed.batch(&ids)?)?;
    }
    let encode_seconds = started.elapsed().as_secs_f64();
    let started = Instant::now();
    let compressed = zstd::stream::encode_all(bytes.as_slice(), 3)?;
    if decompress(&compressed)? != bytes {
        return Err(bad("cache compression bit parity"));
    }
    let compress_seconds = started.elapsed().as_secs_f64();
    control.check("token_cache_before_publish")?;
    std::fs::create_dir(output)?;
    let lock = std::fs::File::open(output)?;
    lock.try_lock()
        .map_err(|e| bad(&format!("cache writer lock: {e}")))?;
    write_bytes(&output.join("compile.pending"), &key.hashes.concat())?;
    let started = Instant::now();
    write_bytes(&output.join("train.r3tok"), &bytes)?;
    let cold = compressed.len() < bytes.len();
    if cold {
        write_bytes(&output.join("train.r3tok.zst"), &compressed)?;
    }
    let mut receipt = b"R3TKDONE".to_vec();
    receipt.extend(hash(&bytes));
    receipt.extend(if cold { hash(&compressed) } else { [0; 32] });
    write_bytes(&output.join("complete.bin"), &receipt)?;
    // Derivative remains unconfirmed if any preceding publication failed. The
    // directory lock serializes a consumer with this last commit boundary.
    std::fs::remove_file(output.join("compile.pending"))?;
    if let Err(e) = lock.sync_all() {
        eprintln!("CACHE_COMMITTED_CLEANUP_WARNING={e}");
    }
    println!(
        "TOKEN_CACHE_COMPILE=PASS samples={} tokens={} width={} raw_bytes={} zstd3_bytes={} cold_selected={cold} metadata_bytes={} setup_native_s={setup} tokenize_annotation_s={tokenize} encode_verify_s={encode_seconds} compress_verify_s={compress_seconds} durable_publish_s={} PARITY=ALL_SAMPLES_AND_5_BATCHES SMALL_UPDATES=0 GENERATIONS=0 TEACHERS=0",
        f.len(),
        packed.offsets.last().unwrap(),
        packed.width,
        bytes.len(),
        compressed.len(),
        receipt.len(),
        started.elapsed().as_secs_f64()
    );
    println!(
        "CACHE_SHA={} SNAPSHOT_SHA={} SPLIT_SHA={} ORDINAL_SHA={} TOKENIZER_SHA={} FRAMING_SHA={} POLICY_SHA={}",
        hex(&hash(&bytes)),
        hex(&key.hashes[0]),
        hex(&key.hashes[1]),
        hex(&key.hashes[2]),
        hex(&key.hashes[3]),
        hex(&key.hashes[4]),
        hex(&key.hashes[5])
    );
    Ok(())
}
fn receipt(root: &Path) -> Result<(std::fs::File, [Hash; 2])> {
    let lock = std::fs::File::open(root)?;
    lock.try_lock_shared()
        .map_err(|e| bad(&format!("cache reader lock: {e}")))?;
    if root.join("compile.pending").exists() {
        return Err(bad(
            "cache compilation unconfirmed; preserve and compile a new path",
        ));
    }
    let raw = neural::read_bounded(&root.join("complete.bin"), 72)?;
    if raw.len() != 72 || &raw[..8] != b"R3TKDONE" {
        return Err(bad("cache completion record"));
    }
    Ok((
        lock,
        [
            raw[8..40].try_into().unwrap(),
            raw[40..72].try_into().unwrap(),
        ],
    ))
}
impl Packed {
    fn annotations(&self, s: &RunSnapshot, tok: &ByteBpe) -> Result<Vec<target_loss::Annotation>> {
        s.train
            .iter()
            .enumerate()
            .map(|(i, ordinal)| {
                let sample = self.sample(i)?;
                let (fractions, roles) = target_loss::token_roles(
                    &s.cases[*ordinal as usize].answer,
                    &sample,
                    tok,
                    &self.spans[i],
                )?;
                Ok(target_loss::Annotation {
                    spans: self.spans[i].clone(),
                    fractions,
                    roles,
                    supported: self.supported[i],
                })
            })
            .collect()
    }
}
pub(super) fn measure(
    root: &Path,
    cache: &Path,
    corpus: &Path,
    repetitions: u8,
    control: &mut RunControl,
) -> Result<()> {
    if !matches!(repetitions, 1 | 3) {
        return Err(bad(
            "cache measurements require 1 fresh or 3 warm repetitions",
        ));
    }
    let setup = Instant::now();
    let (s, source, l) = inputs(root)?;
    let key = key(&s, &source, &l)?;
    let (expected, expected_roles) = framed(&s, &l)?;
    let (_lock, hashes) = receipt(cache)?;
    let draws = s
        .tape
        .iter()
        .take(5)
        .map(|d| d.indices.iter().map(|i| *i as usize).collect::<Vec<_>>())
        .collect::<Vec<_>>();
    if draws.len() != 5 {
        return Err(bad("cache measurement requires five existing draws"));
    }
    println!(
        "CACHE_MEASURE_SETUP_S={} REPETITIONS={repetitions} PROCESS={} OS_CACHE_COLD=false SETUP=verified_native_and_expected_samples_outside_timing SOURCE={} BINARY={}",
        setup.elapsed().as_secs_f64(),
        std::process::id(),
        hex(&evaluator_source()),
        file_hash(&std::env::current_exe()?)?
    );
    let mut sizes = BTreeMap::new();
    for format in ["source-json", "snapshot", "cache-raw", "cache-zstd3"] {
        if format == "cache-zstd3" && hashes[1] == [0; 32] {
            println!("CACHE_FORMAT=cache-zstd3 NOT_SELECTED_NOT_SMALLER");
            continue;
        }
        for repetition in 0..repetitions {
            control.check("token_cache_measurement")?;
            let start = Instant::now();
            let mut phase = [0.; 4];
            let mut point = Instant::now();
            let (plain, packed, role_rows, bytes, scope): (
                Option<Vec<Sample>>,
                Option<Packed>,
                Vec<target_loss::Annotation>,
                usize,
                String,
            ) = match format {
                "source-json" => {
                    let manifest_bytes =
                        neural::read_bounded(&corpus.join("manifest.json"), MAX_FILE)?;
                    phase[0] += point.elapsed().as_secs_f64();
                    point = Instant::now();
                    let manifest: data::CorpusManifest = serde_json::from_slice(&manifest_bytes)?;
                    phase[2] += point.elapsed().as_secs_f64();
                    point = Instant::now();
                    for split in [&manifest.train, &manifest.validation] {
                        if Path::new(&split.file).components().count() != 1
                            || split.file.starts_with('.')
                            || split.file.contains('\\')
                        {
                            return Err(bad("benchmark source split path"));
                        }
                    }
                    let train = neural::read_bounded(&corpus.join(&manifest.train.file), MAX_FILE)?;
                    let validation =
                        neural::read_bounded(&corpus.join(&manifest.validation.file), MAX_FILE)?;
                    phase[0] += point.elapsed().as_secs_f64();
                    point = Instant::now();
                    if hex(&hash(&train)) != manifest.train.sha256
                        || train.len() != manifest.train.bytes
                        || hex(&hash(&validation)) != manifest.validation.sha256
                        || validation.len() != manifest.validation.bytes
                    {
                        return Err(bad("benchmark source JSON manifest binding"));
                    }
                    phase[1] += point.elapsed().as_secs_f64();
                    point = Instant::now();
                    let episodes: Vec<Episode> = serde_json::from_slice(&train)?;
                    let dev: Vec<Episode> = serde_json::from_slice(&validation)?;
                    if episodes.len() != manifest.train.documents
                        || dev.len() != manifest.validation.documents
                        || episodes.len() != s.train.len()
                        || episodes
                            .iter()
                            .zip(&s.train)
                            .any(|(e, i)| case_hash(e) != case_hash(&s.cases[*i as usize]))
                    {
                        return Err(bad(
                            "benchmark source JSON ordered train differs from snapshot",
                        ));
                    }
                    let frozen = s.cases.iter().map(case_hash).collect::<BTreeSet<_>>();
                    if dev.iter().any(|e| !frozen.contains(&case_hash(e))) {
                        return Err(bad("benchmark source JSON validation content differs"));
                    }
                    phase[2] += point.elapsed().as_secs_f64();
                    point = Instant::now();
                    let f = samples(&episodes, &l.tokenizer, key.seq)?;
                    let a = target_loss::annotations(
                        &episodes,
                        &f,
                        &l.tokenizer,
                        &s.authorization.as_ref().unwrap().pools[1],
                    )?;
                    phase[3] = point.elapsed().as_secs_f64();
                    (
                        Some(f),
                        None,
                        a,
                        manifest_bytes.len() + train.len() + validation.len(),
                        format!(
                            "train{}+validation{}+manifest;train_bytes={}",
                            episodes.len(),
                            dev.len(),
                            train.len()
                        ),
                    )
                }
                "snapshot" => {
                    let raw = neural::read_bounded(&root.join("inputs.r3er"), MAX_FILE)?;
                    phase[0] = point.elapsed().as_secs_f64();
                    point = Instant::now();
                    if hash(&raw) != key.hashes[0] {
                        return Err(bad("benchmark snapshot changed"));
                    }
                    phase[1] = point.elapsed().as_secs_f64();
                    point = Instant::now();
                    let Record::Inputs(owned) = Record::decode(&raw)? else {
                        return Err(bad("benchmark snapshot type"));
                    };
                    phase[2] = point.elapsed().as_secs_f64();
                    point = Instant::now();
                    let (f, a) = framed(&owned, &l)?;
                    phase[3] = point.elapsed().as_secs_f64();
                    (
                        Some(f),
                        None,
                        a,
                        raw.len(),
                        format!(
                            "all{}cases+panels+tape+policy;train{}",
                            owned.cases.len(),
                            owned.train.len()
                        ),
                    )
                }
                _ => {
                    let cold = format == "cache-zstd3";
                    let bytes = neural::read_bounded(
                        &cache.join(if cold {
                            "train.r3tok.zst"
                        } else {
                            "train.r3tok"
                        }),
                        MAX_FILE,
                    )?;
                    phase[0] = point.elapsed().as_secs_f64();
                    point = Instant::now();
                    if hash(&bytes) != hashes[usize::from(cold)] {
                        return Err(bad("benchmark cache physical hash"));
                    }
                    phase[1] = point.elapsed().as_secs_f64();
                    point = Instant::now();
                    let n = bytes.len();
                    let packed = decode(if cold { decompress(&bytes)? } else { bytes }, &key)?;
                    let a = packed.annotations(&s, &l.tokenizer)?;
                    phase[2] = point.elapsed().as_secs_f64();
                    (
                        None,
                        Some(packed),
                        a,
                        n,
                        "train_only;tokens+indices+sparse_roles;no_original_text".into(),
                    )
                }
            };
            let batch_start = Instant::now();
            let first = match &packed {
                Some(p) => p.batch(&draws[0])?,
                None => batch(plain.as_ref().unwrap(), &draws[0], &Device::Cpu)?,
            };
            let first_batch = batch_start.elapsed().as_secs_f64();
            let ready = start.elapsed().as_secs_f64();
            // Parity is outside the measured first-batch path, never skipped.
            same_batch(&first, &batch(&expected, &draws[0], &Device::Cpu)?)?;
            if let Some(p) = &packed {
                parity(p, &expected, &expected_roles)?;
            } else if plain.as_ref().unwrap().iter().zip(&expected).any(|(a, b)| {
                a.tokens != b.tokens
                    || a.response_start != b.response_start
                    || a.curriculum != b.curriculum
            }) {
                return Err(bad("benchmark full sample parity"));
            }
            if role_rows.len() != expected_roles.len()
                || role_rows.iter().zip(&expected_roles).any(|(a, b)| {
                    a.spans != b.spans
                        || a.fractions != b.fractions
                        || a.roles != b.roles
                        || a.supported != b.supported
                })
            {
                return Err(bad("benchmark dense annotation parity"));
            }
            let mut batch_times = Vec::new();
            for draw in &draws {
                let started = Instant::now();
                let actual = match &packed {
                    Some(p) => p.batch(draw)?,
                    None => batch(plain.as_ref().unwrap(), draw, &Device::Cpu)?,
                };
                batch_times.push(started.elapsed().as_secs_f64());
                same_batch(&actual, &batch(&expected, draw, &Device::Cpu)?)?;
            }
            println!(
                "CACHE_MEASURE format={format} repetition={repetition} bytes={bytes} read_s={} hash_s={} parse_integrity_role_decode_s={} tokenize_annotate_s={} first_batch_s={first_batch} ready_first_batch_s={ready} batch5_s={batch_times:?} PARITY=EXACT scope={scope}",
                phase[0], phase[1], phase[2], phase[3]
            );
            sizes.insert(format, bytes);
        }
    }
    let json = sizes["source-json"];
    let snapshot = sizes["snapshot"];
    let raw = sizes["cache-raw"];
    let cold = sizes.get("cache-zstd3").copied().unwrap_or(0);
    println!(
        "STORAGE_RETAINED source_json={json} snapshot={snapshot} raw_cache={raw} cold_cache={cold} completion_metadata=72 original_plus_snapshot={} with_raw_cache={} retained_all={} MODEL_NATIVE_BYTES={} MODEL_FORMAT_UNCHANGED=true NEW_SMALL_UPDATES=0 GENERATIONS=0 TEACHERS=0",
        json + snapshot,
        json + snapshot + raw + 72,
        json + snapshot + raw + cold + 72,
        std::fs::metadata(owned_path(root, &s.parent.file.locator, true)?)?.len()
    );
    Ok(())
}

#[allow(clippy::too_many_arguments)] // One bounded comparison of the same source plus its train-only cache.
pub(super) fn measure_source(
    root: &Path,
    legacy: &Path,
    raw: &Path,
    cold: &Path,
    cache: &Path,
    output: &Path,
    repetitions: u8,
    selected_format: Option<&str>,
    control: &mut RunControl,
) -> Result<()> {
    if !(1..=3).contains(&repetitions) {
        return Err(bad("source measurement repetitions"));
    }
    let setup = Instant::now();
    let (s, snapshot, l) = inputs(root)?;
    let k = key(&s, &snapshot, &l)?;
    let source = native_source(root, &s)?;
    let (expected, annotations) = framed(&s, &l)?;
    let (legacy_meta, legacy_train, legacy_dev) = data::load_legacy(legacy)?;
    if data::native::ordered_bytes(&source.train) != data::native::ordered_bytes(&legacy_train)
        || data::native::ordered_bytes(&source.validation)
            != data::native::ordered_bytes(&legacy_dev)
    {
        return Err(bad("measurement logical scope"));
    }
    let (_lock, cache_hashes) = receipt(cache)?;
    let bindings = [
        source.semantic,
        unhex(&file_hash(raw)?)?,
        unhex(&file_hash(cold)?)?,
        cache_hashes[0],
        cache_hashes[1],
    ];
    let draws = s
        .tape
        .iter()
        .take(5)
        .map(|d| d.indices.iter().map(|i| *i as usize).collect::<Vec<_>>())
        .collect::<Vec<_>>();
    if draws.is_empty() {
        return Err(bad("measurement tape absent"));
    }
    std::fs::create_dir(output)?;
    println!(
        "SOURCE_MEASURE setup_s={} PROCESS={} repetitions={repetitions} OS_CACHE_COLD=false PHASES=read,physical_hash,decode_with_integrity,tokenize,first_batch,steady_batch,encode,verification,durable_publish SCOPE_SOURCE=full_train_dev_metadata SCOPE_CACHE=train_only",
        setup.elapsed().as_secs_f64(),
        std::process::id()
    );
    let formats = [
        "source-json",
        "native-raw",
        "native-zstd3",
        "cache-raw",
        "cache-zstd3",
    ];
    if selected_format.is_some_and(|f| !formats.contains(&f)) {
        return Err(bad("unknown source measurement format"));
    }
    let mut rows = Vec::new();
    for rep in 0..repetitions {
        for offset in 0..formats.len() {
            control.check("source_measurement")?;
            let format = formats[(offset + rep as usize) % formats.len()];
            if selected_format.is_some_and(|selected| selected != format) {
                continue;
            }
            let mut phase = [0.; 9];
            let t = Instant::now();
            let mut files = Vec::new();
            if format == "source-json" {
                for name in [
                    "manifest.json",
                    legacy_meta.train.file.as_str(),
                    legacy_meta.validation.file.as_str(),
                ] {
                    files.push(neural::read_bounded(&legacy.join(name), MAX_FILE)?);
                }
            } else {
                let path = match format {
                    "native-raw" => raw.to_owned(),
                    "native-zstd3" => cold.to_owned(),
                    "cache-raw" => cache.join("train.r3tok"),
                    _ => cache.join("train.r3tok.zst"),
                };
                files.push(neural::read_bounded(&path, MAX_FILE)?);
            }
            phase[0] = t.elapsed().as_secs_f64();
            let size = files.iter().map(Vec::len).sum::<usize>();
            let t = Instant::now();
            for f in &files {
                std::hint::black_box(hash(f));
            }
            phase[1] = t.elapsed().as_secs_f64();
            let t = Instant::now();
            let mut packed = None;
            let mut episodes = None;
            if format == "source-json" {
                let m: data::CorpusManifest = serde_json::from_slice(&files[0])?;
                if neural::hash(&files[1]) != m.train.sha256
                    || neural::hash(&files[2]) != m.validation.sha256
                {
                    return Err(bad("measurement legacy hash"));
                }
                let train: Vec<Episode> = serde_json::from_slice(&files[1])?;
                let dev: Vec<Episode> = serde_json::from_slice(&files[2])?;
                data::validate_episodes(&train)?;
                data::validate_episodes(&dev)?;
                data::check_split(&train, &dev)?;
                episodes = Some(train);
            } else if format.starts_with("native-") {
                let c = data::native::decode(&files[0])?;
                if c.semantic != source.semantic {
                    return Err(bad("measurement source mismatch"));
                }
                episodes = Some(c.train);
            } else {
                let bytes = if format == "cache-zstd3" {
                    if hash(&files[0]) != cache_hashes[1] {
                        return Err(bad("cold cache hash"));
                    }
                    decompress(&files[0])?
                } else {
                    files[0].clone()
                };
                if hash(&bytes) != cache_hashes[0] {
                    return Err(bad("raw cache hash"));
                }
                packed = Some(decode(bytes, &k)?);
            }
            phase[2] = t.elapsed().as_secs_f64();
            let t = Instant::now();
            let plain = episodes
                .as_ref()
                .map(|e| samples(e, &l.tokenizer, k.seq))
                .transpose()?;
            phase[3] = if plain.is_some() {
                t.elapsed().as_secs_f64()
            } else {
                0.
            };
            let t = Instant::now();
            let first = if let Some(p) = &packed {
                p.batch(&draws[0])?
            } else {
                batch(plain.as_ref().unwrap(), &draws[0], &Device::Cpu)?
            };
            phase[4] = t.elapsed().as_secs_f64();
            same_batch(&first, &batch(&expected, &draws[0], &Device::Cpu)?)?;
            if let Some(p) = &packed {
                parity(p, &expected, &annotations)?;
            } else {
                for (a, b) in plain.as_ref().unwrap().iter().zip(&expected) {
                    if a.tokens != b.tokens
                        || a.response_start != b.response_start
                        || a.curriculum != b.curriculum
                    {
                        return Err(bad("all train sample parity"));
                    }
                }
            }
            let t = Instant::now();
            for ids in &draws {
                if let Some(p) = &packed {
                    std::hint::black_box(p.batch(ids)?);
                } else {
                    std::hint::black_box(batch(plain.as_ref().unwrap(), ids, &Device::Cpu)?);
                }
            }
            phase[5] = t.elapsed().as_secs_f64() / draws.len() as f64;
            let t = Instant::now();
            let encoded = if format == "source-json" {
                let train = serde_json::to_vec(&legacy_train)?;
                let dev = serde_json::to_vec(&legacy_dev)?;
                let mut exported = legacy_meta.clone();
                exported.train.sha256 = neural::hash(&train);
                exported.train.bytes = train.len();
                exported.validation.sha256 = neural::hash(&dev);
                exported.validation.bytes = dev.len();
                vec![serde_json::to_vec_pretty(&exported)?, train, dev]
            } else if format.starts_with("native-") {
                vec![data::native::encode(&source, format == "native-zstd3")?]
            } else {
                let b = encode(&k, &expected, &annotations)?;
                vec![if format == "cache-zstd3" {
                    zstd::stream::encode_all(b.as_slice(), 3)?
                } else {
                    b
                }]
            };
            phase[6] = t.elapsed().as_secs_f64();
            let t = Instant::now();
            if format == "source-json" {
                let m: data::CorpusManifest = serde_json::from_slice(&encoded[0])?;
                let train: Vec<Episode> = serde_json::from_slice(&encoded[1])?;
                let dev: Vec<Episode> = serde_json::from_slice(&encoded[2])?;
                if m.train.sha256 != neural::hash(&encoded[1])
                    || m.validation.sha256 != neural::hash(&encoded[2])
                    || data::native::ordered_bytes(&train)
                        != data::native::ordered_bytes(&source.train)
                    || data::native::ordered_bytes(&dev)
                        != data::native::ordered_bytes(&source.validation)
                {
                    return Err(bad("measurement JSON export equality"));
                }
            } else if format.starts_with("native-") {
                if data::native::decode(&encoded[0])?.semantic != source.semantic {
                    return Err(bad("measurement native export equality"));
                }
            } else {
                let b = if format == "cache-zstd3" {
                    decompress(&encoded[0])?
                } else {
                    encoded[0].clone()
                };
                parity(&decode(b, &k)?, &expected, &annotations)?;
            }
            phase[7] = t.elapsed().as_secs_f64();
            let t = Instant::now();
            for (i, bytes) in encoded.iter().enumerate() {
                write_bytes(&output.join(format!("{format}-{rep}-{i}.bytes")), bytes)?;
            }
            phase[8] = t.elapsed().as_secs_f64();
            let rss = super::super::rss_kib().ok();
            println!(
                "SOURCE_MEASURE format={format} repetition={rep} bytes={size} first_batch_ready_s={} phases_s={phase:?} rss_KiB={rss:?} PARITY=PASS",
                phase[..5].iter().sum::<f64>()
            );
            rows.push(PreparationTiming {
                format: format.into(),
                repetition: rep,
                bytes: size as u64,
                seconds: phase,
                rss,
            });
        }
    }
    publish(
        output,
        "measurement.r3er",
        &Record::PreparationMeasure { bindings, rows },
    )?;
    println!(
        "STORAGE_MEASURED=PASS SMALL_UPDATES=0 GENERATIONS=0 TEACHERS=0 TRAINING_THROUGHPUT=NOT_RUN"
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn fixture() -> (Key, Vec<Sample>, Vec<target_loss::Annotation>) {
        let key = Key {
            hashes: std::array::from_fn(|i| hash(&[i as u8])),
            ordinals: vec![3, 8],
            seq: 16,
            vocab: 264,
        };
        let samples = vec![
            Sample {
                tokens: vec![
                    BOS,
                    neural::USER_ROLE,
                    neural::END_ROLE,
                    neural::ASSISTANT_ROLE,
                    8,
                    EOS,
                ],
                response_start: 4,
                curriculum: false,
            },
            Sample {
                tokens: vec![
                    BOS,
                    neural::USER_ROLE,
                    neural::END_ROLE,
                    neural::ASSISTANT_ROLE,
                    9,
                    10,
                    EOS,
                ],
                response_start: 4,
                curriculum: true,
            },
        ];
        let annotations = vec![
            target_loss::Annotation {
                spans: vec![target_loss::Span {
                    start: 0,
                    end: 1,
                    role: 1,
                }],
                fractions: vec![1., 0.],
                roles: vec![],
                supported: true,
            },
            target_loss::Annotation {
                spans: vec![],
                fractions: vec![0.; 3],
                roles: vec![],
                supported: false,
            },
        ];
        (key, samples, annotations)
    }
    fn rehash(bytes: &mut [u8]) {
        let body = hash(&bytes[TOK_HEADER..]);
        bytes[256..288].copy_from_slice(&body);
        let header = hash(&bytes[..288]);
        bytes[288..320].copy_from_slice(&header);
    }
    #[test]
    fn token_cache_packed_u16_u32_actual_batch_and_compression_parity() {
        let (mut key, mut f, a) = fixture();
        for vocab in [264, 65537] {
            key.vocab = vocab;
            f[1].tokens[4] = (vocab - 1) as u32;
            let bytes = encode(&key, &f, &a).unwrap();
            let p = decode(bytes.clone(), &key).unwrap();
            assert_eq!(p.width, if vocab <= 65536 { 2 } else { 4 });
            parity(&p, &f, &a).unwrap();
            for indices in [vec![0], vec![1], vec![0, 1], vec![1, 0], vec![1, 1, 0]] {
                same_batch(
                    &batch(&f, &indices, &Device::Cpu).unwrap(),
                    &p.batch(&indices).unwrap(),
                )
                .unwrap();
            }
            let compressed = zstd::stream::encode_all(bytes.as_slice(), 3).unwrap();
            assert_eq!(decompress(&compressed).unwrap(), bytes);
            assert!(decompress(&compressed[..compressed.len() - 1]).is_err());
        }
    }
    #[test]
    fn token_cache_rejects_incompatible_corrupt_offsets_ids_and_trailing() {
        let (key, f, a) = fixture();
        let original = encode(&key, &f, &a).unwrap();
        let packed = decode(original.clone(), &key).unwrap();
        for field in 0..6 {
            let mut changed = key.clone();
            changed.hashes[field][0] ^= 1;
            assert!(decode(original.clone(), &changed).is_err());
        }
        for mutation in 0..24 {
            let mut b = original.clone();
            match mutation {
                0 => b[0] ^= 1,
                1 => b[8] = 2,
                2 => b[10] = 2,
                3 => b[11] = 4,
                4 => b[16..24].copy_from_slice(&u64::MAX.to_le_bytes()),
                5 => b[24..28].copy_from_slice(&u32::MAX.to_le_bytes()),
                6 => b[28..32].copy_from_slice(&17u32.to_le_bytes()),
                7 => b[32..40].copy_from_slice(&u64::MAX.to_le_bytes()),
                8 => b[40..48].copy_from_slice(&321u64.to_le_bytes()),
                9 => b[48] ^= 1,
                10 => b[56] ^= 1,
                11 => b[64] ^= 1,
                12 => b[320..328].copy_from_slice(&1u64.to_le_bytes()),
                13 => b[328..336].copy_from_slice(&0u64.to_le_bytes()),
                14 => b[344..348].copy_from_slice(&0u32.to_le_bytes()),
                15 => b[352..356].copy_from_slice(&8u32.to_le_bytes()),
                16 => b[360] = 2,
                17 => b[362] = 0,
                18 => b[packed.token_offset..packed.token_offset + 2]
                    .copy_from_slice(&(PAD as u16).to_le_bytes()),
                19 => {
                    let at = packed.token_offset + (packed.offsets[1] - 1) * 2;
                    b[at..at + 2].copy_from_slice(&(BOS as u16).to_le_bytes());
                }
                20 => {
                    let at = packed.token_offset + 4 * 2;
                    b[at..at + 2].copy_from_slice(&65535u16.to_le_bytes());
                }
                21 => {
                    let end = b.len();
                    b[end - 5..end - 1].copy_from_slice(&0u32.to_le_bytes());
                }
                22 => *b.last_mut().unwrap() = 7,
                23 => {
                    b.push(0);
                    let len = b.len() as u64;
                    b[16..24].copy_from_slice(&len.to_le_bytes());
                }
                _ => unreachable!(),
            }
            rehash(&mut b);
            assert!(decode(b, &key).is_err(), "mutation {mutation}");
        }
        for n in [0, 319, original.len() - 1] {
            assert!(decode(original[..n].to_vec(), &key).is_err());
        }
        let mut corrupt = original.clone();
        *corrupt.last_mut().unwrap() ^= 1;
        assert!(decode(corrupt, &key).is_err());
        // A self-recomputed checksum is not authentication. Full baseline parity
        // and the externally bound physical completion digest reject altered IDs.
        let mut changed = original;
        let at = packed.token_offset + 4 * 2;
        changed[at..at + 2].copy_from_slice(&9u16.to_le_bytes());
        rehash(&mut changed);
        assert!(parity(&decode(changed, &key).unwrap(), &f, &a).is_err());
    }
    #[test]
    fn token_cache_pending_and_active_writer_block_readers() {
        let d = tempfile::tempdir().unwrap();
        let mut final_bytes = b"R3TKDONE".to_vec();
        final_bytes.extend([7; 64]);
        write_bytes(&d.path().join("complete.bin"), &final_bytes).unwrap();
        write_bytes(&d.path().join("compile.pending"), b"unfinished").unwrap();
        assert!(receipt(d.path()).is_err());
        std::fs::remove_file(d.path().join("compile.pending")).unwrap();
        let lock = std::fs::File::open(d.path()).unwrap();
        lock.try_lock().unwrap();
        assert!(receipt(d.path()).is_err());
        drop(lock);
        assert_eq!(receipt(d.path()).unwrap().1, [[7; 32]; 2]);
    }
}
