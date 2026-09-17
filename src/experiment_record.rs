//! Training-only, schema-specific immutable experiment records. Native control never parses JSON.
use super::*;
use replica_v3::{
    codec::{Reader, publish_new, put_bytes, put_varint},
    event::{GenerationLimits, RelationKind},
    retrieval::{Evidence, EvidenceBundle, RelationStep},
};
use sha2::{Digest as _, Sha256};
use std::io::{Read, Seek, SeekFrom};

type Hash = [u8; 32];
const HEADER: usize = 48;
const MAX_FILE: usize = 128 * 1024 * 1024;
const MAX_CASES: usize = 16384;
const MAX_ROWS: usize = 512;
const MAX_TOKENS: usize = 2048;
const MAX_TEXT: usize = 262144;
const CONTRACT: &str = "R3-BINARY-EVAL-RESUME-1.0";

fn bad(s: &str) -> Error {
    Error::Corrupt(format!("R3ER: {s}"))
}
fn hash(b: &[u8]) -> Hash {
    Sha256::digest(b).into()
}
fn hex(h: &Hash) -> String {
    h.iter().map(|b| format!("{b:02x}")).collect()
}
fn unhex(s: &str) -> Result<Hash> {
    if s.len() != 64 || !s.bytes().all(|b| b.is_ascii_hexdigit()) {
        return Err(bad("digest syntax"));
    }
    let mut out = [0; 32];
    for (i, b) in out.iter_mut().enumerate() {
        *b = u8::from_str_radix(&s[i * 2..i * 2 + 2], 16).map_err(|_| bad("digest syntax"))?;
    }
    Ok(out)
}
fn count(r: &mut Reader<'_>, maximum: usize) -> Result<usize> {
    let n = usize::try_from(r.var()?).map_err(|_| bad("count overflow"))?;
    if n > maximum {
        return Err(bad("count bound"));
    }
    Ok(n)
}
fn text(r: &mut Reader<'_>) -> Result<String> {
    String::from_utf8(r.bytes(MAX_TEXT)?).map_err(|_| bad("text UTF-8"))
}
fn string(b: &mut Vec<u8>, s: &str) {
    put_bytes(b, s.as_bytes());
}
fn digest_read(r: &mut Reader<'_>) -> Result<Hash> {
    Ok(r.take(32)?.try_into().expect("checked digest"))
}
fn u32_read(r: &mut Reader<'_>) -> Result<u32> {
    u32::try_from(r.var()?).map_err(|_| bad("u32 overflow"))
}
fn signed(b: &mut Vec<u8>, n: i64) {
    put_varint(b, ((n as u64) << 1) ^ ((n >> 63) as u64));
}
fn signed_read(r: &mut Reader<'_>) -> Result<i64> {
    let n = r.var()?;
    Ok(((n >> 1) as i64) ^ -((n & 1) as i64))
}
fn optional<T>(b: &mut Vec<u8>, value: Option<&T>, write: impl FnOnce(&mut Vec<u8>, &T)) {
    b.push(u8::from(value.is_some()));
    if let Some(value) = value {
        write(b, value);
    }
}
fn integers(b: &mut Vec<u8>, ns: &[u32]) {
    put_varint(b, ns.len() as u64);
    for n in ns {
        put_varint(b, u64::from(*n));
    }
}
fn integers_read(r: &mut Reader<'_>, max: usize) -> Result<Vec<u32>> {
    let n = count(r, max)?;
    (0..n).map(|_| u32_read(r)).collect()
}
fn ids(b: &mut Vec<u8>, ns: &[i64]) {
    put_varint(b, ns.len() as u64);
    for n in ns {
        signed(b, *n);
    }
}
fn ids_read(r: &mut Reader<'_>) -> Result<Vec<i64>> {
    let n = count(r, 256)?;
    (0..n).map(|_| signed_read(r)).collect()
}

/// Nonfinite observations retain their bits in a failure variant, never a successful scalar.
#[derive(Clone, Debug, Serialize)]
enum Scalar {
    F32(f32),
    F64(f64),
    Nonfinite32(u32),
    Nonfinite64(u64),
}
impl Scalar {
    fn encode(&self, b: &mut Vec<u8>) {
        match self {
            Self::F32(n) => {
                b.push(0);
                b.extend(n.to_bits().to_le_bytes());
            }
            Self::F64(n) => {
                b.push(1);
                b.extend(n.to_bits().to_le_bytes());
            }
            Self::Nonfinite32(n) => {
                b.push(2);
                b.extend(n.to_le_bytes());
            }
            Self::Nonfinite64(n) => {
                b.push(3);
                b.extend(n.to_le_bytes());
            }
        }
    }
    fn decode(r: &mut Reader<'_>) -> Result<Self> {
        let scalar = match r.byte()? {
            0 => Self::F32(f32::from_bits(u32::from_le_bytes(
                r.take(4)?.try_into().unwrap(),
            ))),
            1 => Self::F64(f64::from_bits(u64::from_le_bytes(
                r.take(8)?.try_into().unwrap(),
            ))),
            2 => Self::Nonfinite32(u32::from_le_bytes(r.take(4)?.try_into().unwrap())),
            3 => Self::Nonfinite64(u64::from_le_bytes(r.take(8)?.try_into().unwrap())),
            _ => return Err(bad("scalar tag")),
        };
        let valid = match scalar {
            Self::F32(x) => x.is_finite(),
            Self::F64(x) => x.is_finite(),
            Self::Nonfinite32(x) => !f32::from_bits(x).is_finite(),
            Self::Nonfinite64(x) => !f64::from_bits(x).is_finite(),
        };
        if !valid {
            return Err(bad("scalar finite/error classification"));
        }
        Ok(scalar)
    }
    fn finite(&self) -> Result<f64> {
        match self {
            Self::F32(n) if n.is_finite() => Ok(f64::from(*n)),
            Self::F64(n) if n.is_finite() => Ok(*n),
            _ => Err(bad("nonfinite success scalar")),
        }
    }
}

#[derive(Clone, Debug, Serialize)]
struct FileRef {
    locator: String,
    digest: Hash,
}
impl FileRef {
    fn encode(&self, b: &mut Vec<u8>) {
        string(b, &self.locator);
        b.extend(self.digest);
    }
    fn decode(r: &mut Reader<'_>) -> Result<Self> {
        Ok(Self {
            locator: text(r)?,
            digest: digest_read(r)?,
        })
    }
}
#[derive(Clone, Debug, Serialize)]
struct CheckpointRef {
    file: FileRef,
    run: Hash,
    segment: u32,
    model: Hash,
    tokenizer: Hash,
    architecture: Hash,
    step: u64,
    adam: Option<Hash>,
    counters: [u64; 3],
}
impl CheckpointRef {
    fn encode(&self, b: &mut Vec<u8>) {
        self.file.encode(b);
        b.extend(self.run);
        put_varint(b, u64::from(self.segment));
        for h in [self.model, self.tokenizer, self.architecture] {
            b.extend(h);
        }
        put_varint(b, self.step);
        optional(b, self.adam.as_ref(), |b, h| b.extend(h));
        for n in self.counters {
            put_varint(b, n);
        }
    }
    fn decode(r: &mut Reader<'_>) -> Result<Self> {
        Ok(Self {
            file: FileRef::decode(r)?,
            run: digest_read(r)?,
            segment: u32_read(r)?,
            model: digest_read(r)?,
            tokenizer: digest_read(r)?,
            architecture: digest_read(r)?,
            step: r.var()?,
            adam: r.opt(digest_read)?,
            counters: [r.var()?, r.var()?, r.var()?],
        })
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize)]
enum PanelKind {
    Dev,
    Watch,
    Cross,
    Ordinary,
    DevParity,
    OrdinaryParity,
}
impl PanelKind {
    fn tag(self) -> u8 {
        match self {
            Self::Dev => 0,
            Self::Watch => 1,
            Self::Cross => 2,
            Self::Ordinary => 3,
            Self::DevParity => 4,
            Self::OrdinaryParity => 5,
        }
    }
    fn read(r: &mut Reader<'_>) -> Result<Self> {
        match r.byte()? {
            0 => Ok(Self::Dev),
            1 => Ok(Self::Watch),
            2 => Ok(Self::Cross),
            3 => Ok(Self::Ordinary),
            4 => Ok(Self::DevParity),
            5 => Ok(Self::OrdinaryParity),
            _ => Err(bad("panel kind")),
        }
    }
    fn name(self) -> &'static str {
        match self {
            Self::Dev => "dev",
            Self::Watch => "watch",
            Self::Cross => "cross",
            Self::Ordinary => "ordinary",
            Self::DevParity => "dev-parity",
            Self::OrdinaryParity => "ordinary-parity",
        }
    }
}
#[derive(Clone, Debug, Serialize)]
struct PanelSpec {
    kind: PanelKind,
    dataset: Hash,
    cases: Vec<u32>,
}
impl PanelSpec {
    fn encode(&self, b: &mut Vec<u8>) {
        b.push(self.kind.tag());
        b.extend(self.dataset);
        integers(b, &self.cases);
    }
    fn decode(r: &mut Reader<'_>) -> Result<Self> {
        Ok(Self {
            kind: PanelKind::read(r)?,
            dataset: digest_read(r)?,
            cases: integers_read(r, MAX_ROWS)?,
        })
    }
}
#[derive(Clone, Debug, Serialize)]
struct Origin {
    role: String,
    original: FileRef,
}
#[derive(Clone, Debug, Serialize)]
struct Draw {
    indices: Vec<u32>,
    sampler: u64,
    input: u64,
    target: u64,
}
#[derive(Clone, Debug, Serialize)]
struct RunSnapshot {
    contract: String,
    run: Hash,
    policy: Hash,
    frozen: Hash,
    source: Hash,
    parent: CheckpointRef,
    historical: bool,
    tiny_spec: bool,
    origins: Vec<Origin>,
    cases: Vec<Episode>,
    train: Vec<u32>,
    panels: Vec<PanelSpec>,
    tape: Vec<Draw>,
    eval_steps: Vec<u32>,
    lr_policy: u8,
    lr_offset: u64,
    baseline: [u64; 3],
    anchor_floor: u64,
}

fn episode_encode(e: &Episode, b: &mut Vec<u8>) {
    string(b, &e.id);
    put_varint(b, e.category as u64);
    for s in [
        &e.family,
        &e.binding,
        &e.sequence,
        &e.request.request_id,
        &e.request.system,
        &e.request.input,
        &e.answer,
    ] {
        string(b, s);
    }
    let limits = &e.request.limits;
    for n in [
        u64::from(limits.max_tokens),
        u64::from(limits.context_tokens),
        limits.timeout_ms,
    ] {
        put_varint(b, n);
    }
    let bundle = &e.request.evidence;
    b.push(u8::from(bundle.truncated));
    for n in [
        bundle.visited,
        bundle.candidates_fetched,
        bundle.edges_fetched,
        bundle.eligible,
    ] {
        put_varint(b, n as u64);
    }
    put_varint(b, bundle.items.len() as u64);
    for item in &bundle.items {
        signed(b, item.event_id);
        string(b, &item.original_excerpt);
        b.push(u8::from(item.excerpt_truncated));
        string(b, &item.source);
        signed(b, item.recorded_at);
        optional(b, item.observed_at.as_ref(), |b, n| signed(b, *n));
        string(b, &item.version_status);
        string(b, &item.retrieval_reason);
        put_varint(b, item.relation_path.len() as u64);
        for relation in &item.relation_path {
            signed(b, relation.from);
            signed(b, relation.to);
            b.push(relation.relation as u8);
            signed(b, relation.origin);
        }
    }
}
fn episode_decode(r: &mut Reader<'_>) -> Result<Episode> {
    let id = text(r)?;
    let category = count(r, 1024)?;
    let family = text(r)?;
    let binding = text(r)?;
    let sequence = text(r)?;
    let request_id = text(r)?;
    let system = text(r)?;
    let input = text(r)?;
    let answer = text(r)?;
    let limits = GenerationLimits {
        max_tokens: u32_read(r)?,
        context_tokens: u32_read(r)?,
        timeout_ms: r.var()?,
    };
    let truncated = r.bool()?;
    let visited = count(r, 1_000_000)?;
    let candidates_fetched = count(r, 1_000_000)?;
    let edges_fetched = count(r, 1_000_000)?;
    let eligible = count(r, 1_000_000)?;
    let n = count(r, 256)?;
    let mut items = Vec::with_capacity(n);
    for _ in 0..n {
        let event_id = signed_read(r)?;
        let original_excerpt = text(r)?;
        let excerpt_truncated = r.bool()?;
        let source = text(r)?;
        let recorded_at = signed_read(r)?;
        let observed_at = r.opt(signed_read)?;
        let version_status = text(r)?;
        let retrieval_reason = text(r)?;
        let count = count(r, 256)?;
        let mut relation_path = Vec::with_capacity(count);
        for _ in 0..count {
            relation_path.push(RelationStep {
                from: signed_read(r)?,
                to: signed_read(r)?,
                relation: RelationKind::from_tag(r.byte()?)?,
                origin: signed_read(r)?,
            });
        }
        items.push(Evidence {
            event_id,
            original_excerpt,
            excerpt_truncated,
            source,
            recorded_at,
            observed_at,
            version_status,
            retrieval_reason,
            relation_path,
        });
    }
    Ok(Episode {
        id,
        category,
        family,
        binding,
        sequence,
        answer,
        request: ModelRequest {
            request_id,
            system,
            input,
            limits,
            evidence: EvidenceBundle {
                items,
                truncated,
                visited,
                candidates_fetched,
                edges_fetched,
                eligible,
            },
        },
    })
}
fn case_hash(e: &Episode) -> Hash {
    let mut b = Vec::new();
    episode_encode(e, &mut b);
    hash(&b)
}
impl RunSnapshot {
    fn encode(&self, b: &mut Vec<u8>) {
        string(b, &self.contract);
        for h in [self.run, self.policy, self.frozen, self.source] {
            b.extend(h);
        }
        self.parent.encode(b);
        b.push(u8::from(self.historical));
        b.push(u8::from(self.tiny_spec));
        put_varint(b, self.origins.len() as u64);
        for origin in &self.origins {
            string(b, &origin.role);
            origin.original.encode(b);
        }
        self.content(b);
    }
    fn content(&self, b: &mut Vec<u8>) {
        put_varint(b, self.cases.len() as u64);
        for e in &self.cases {
            episode_encode(e, b);
        }
        integers(b, &self.train);
        put_varint(b, self.panels.len() as u64);
        for p in &self.panels {
            p.encode(b);
        }
        put_varint(b, self.tape.len() as u64);
        for d in &self.tape {
            integers(b, &d.indices);
            for n in [d.sampler, d.input, d.target] {
                put_varint(b, n);
            }
        }
        integers(b, &self.eval_steps);
        b.push(self.lr_policy);
        put_varint(b, self.lr_offset);
        for n in self.baseline {
            put_varint(b, n);
        }
        put_varint(b, self.anchor_floor);
    }
    fn binding(&self) -> Hash {
        let mut b = b"R3ER-input-binding-v1\0".to_vec();
        string(&mut b, &self.contract);
        for h in [
            self.run,
            self.policy,
            self.frozen,
            self.parent.model,
            self.parent.tokenizer,
            self.parent.architecture,
        ] {
            b.extend(h);
        }
        put_varint(&mut b, self.parent.step);
        b.extend(self.parent.file.digest);
        optional(&mut b, self.parent.adam.as_ref(), |b, h| b.extend(h));
        for n in self.parent.counters {
            put_varint(&mut b, n);
        }
        b.push(u8::from(self.historical));
        b.push(u8::from(self.tiny_spec));
        self.content(&mut b);
        hash(&b)
    }
    fn decode(r: &mut Reader<'_>) -> Result<Self> {
        let contract = text(r)?;
        let run = digest_read(r)?;
        let policy = digest_read(r)?;
        let frozen = digest_read(r)?;
        let source = digest_read(r)?;
        let parent = CheckpointRef::decode(r)?;
        let historical = r.bool()?;
        let tiny_spec = r.bool()?;
        let n = count(r, 256)?;
        let mut origins = Vec::with_capacity(n);
        for _ in 0..n {
            origins.push(Origin {
                role: text(r)?,
                original: FileRef::decode(r)?,
            });
        }
        let n = count(r, MAX_CASES)?;
        let cases = (0..n)
            .map(|_| episode_decode(r))
            .collect::<Result<Vec<_>>>()?;
        let train = integers_read(r, MAX_CASES)?;
        let n = count(r, 4)?;
        let panels = (0..n)
            .map(|_| PanelSpec::decode(r))
            .collect::<Result<Vec<_>>>()?;
        let n = count(r, 512)?;
        let mut tape = Vec::with_capacity(n);
        for _ in 0..n {
            tape.push(Draw {
                indices: integers_read(r, 256)?,
                sampler: r.var()?,
                input: r.var()?,
                target: r.var()?,
            });
        }
        let eval_steps = integers_read(r, 512)?;
        let lr_policy = r.byte()?;
        let lr_offset = r.var()?;
        let baseline = [r.var()?, r.var()?, r.var()?];
        let anchor_floor = r.var()?;
        let s = Self {
            contract,
            run,
            policy,
            frozen,
            source,
            parent,
            historical,
            tiny_spec,
            origins,
            cases,
            train,
            panels,
            tape,
            eval_steps,
            lr_policy,
            lr_offset,
            baseline,
            anchor_floor,
        };
        s.validate()?;
        Ok(s)
    }
    fn validate(&self) -> Result<()> {
        if self.contract != CONTRACT
            || self.parent.run != self.run
            || self.cases.is_empty()
            || self.lr_policy > 1
            || self.panels.len() != 4
            || self
                .panels
                .iter()
                .map(|p| p.kind)
                .collect::<BTreeSet<_>>()
                .len()
                != 4
            || self
                .cases
                .iter()
                .map(|e| &e.id)
                .collect::<BTreeSet<_>>()
                .len()
                != self.cases.len()
            || self.eval_steps.windows(2).any(|s| s[0] >= s[1])
            || self.train.iter().any(|n| *n as usize >= self.cases.len())
            || (self.historical && !self.tape.is_empty())
        {
            return Err(bad("run snapshot identity/membership/budget"));
        }
        for p in &self.panels {
            let expected = match p.kind {
                PanelKind::Dev => 256,
                PanelKind::Watch => 32,
                PanelKind::Cross => 512,
                PanelKind::Ordinary => 400,
                PanelKind::DevParity | PanelKind::OrdinaryParity => 16,
            };
            if p.cases.is_empty()
                || (!self.tiny_spec && p.cases.len() != expected)
                || p.cases.iter().any(|n| *n as usize >= self.cases.len())
                || p.cases.iter().collect::<BTreeSet<_>>().len() != p.cases.len()
            {
                return Err(bad("frozen panel membership"));
            }
        }
        for d in &self.tape {
            if d.indices.is_empty() || d.indices.iter().any(|n| *n as usize >= self.train.len()) {
                return Err(bad("draw membership"));
            }
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Serialize)]
struct Difference {
    index: u32,
    gold: u32,
    actual: Option<u32>,
    argmax: u32,
    log_probability: Scalar,
    margin: Scalar,
}
#[derive(Clone, Debug, Serialize)]
struct FieldAccuracy {
    field: String,
    correct: u64,
    total: u64,
}
#[derive(Clone, Debug, Serialize)]
struct TeacherStats {
    target: u64,
    correct: u64,
    mean: Scalar,
    first: Scalar,
    remaining: Scalar,
    objective: Scalar,
    first_weight: Scalar,
    first_correct: bool,
    last_correct: bool,
    eos_correct: bool,
    first_argmax: u32,
    first_gold: u32,
    eos_probability: Scalar,
    gold_probability: Scalar,
    argmax_probability: Scalar,
    difference: Option<Difference>,
    byte_difference: Option<u64>,
    difference_field: Option<String>,
    field_accuracy: Vec<FieldAccuracy>,
    prompt_matches: bool,
    answer_roundtrip: bool,
}
#[derive(Clone, Debug, Serialize)]
enum TeacherRecord {
    NotRequested,
    Measured(Box<TeacherStats>),
    Failed(String),
}
impl TeacherRecord {
    fn encode(&self, b: &mut Vec<u8>) {
        let Self::Measured(t) = self else {
            match self {
                Self::NotRequested => b.push(0),
                Self::Failed(s) => {
                    b.push(2);
                    string(b, s);
                }
                _ => unreachable!(),
            };
            return;
        };
        b.push(1);
        put_varint(b, t.target);
        put_varint(b, t.correct);
        for s in [
            &t.mean,
            &t.first,
            &t.remaining,
            &t.objective,
            &t.first_weight,
        ] {
            s.encode(b);
        }
        for flag in [t.first_correct, t.last_correct, t.eos_correct] {
            b.push(u8::from(flag));
        }
        put_varint(b, u64::from(t.first_argmax));
        put_varint(b, u64::from(t.first_gold));
        for s in [
            &t.eos_probability,
            &t.gold_probability,
            &t.argmax_probability,
        ] {
            s.encode(b);
        }
        optional(b, t.difference.as_ref(), |b, d| {
            put_varint(b, u64::from(d.index));
            put_varint(b, u64::from(d.gold));
            optional(b, d.actual.as_ref(), |b, n| put_varint(b, u64::from(*n)));
            put_varint(b, u64::from(d.argmax));
            d.log_probability.encode(b);
            d.margin.encode(b);
        });
        optional(b, t.byte_difference.as_ref(), |b, n| put_varint(b, *n));
        optional(b, t.difference_field.as_ref(), |b, s| string(b, s));
        put_varint(b, t.field_accuracy.len() as u64);
        for f in &t.field_accuracy {
            string(b, &f.field);
            put_varint(b, f.correct);
            put_varint(b, f.total);
        }
        b.push(u8::from(t.prompt_matches));
        b.push(u8::from(t.answer_roundtrip));
    }
    fn decode(r: &mut Reader<'_>) -> Result<Self> {
        match r.byte()? {
            0 => return Ok(Self::NotRequested),
            2 => return Ok(Self::Failed(text(r)?)),
            1 => {}
            _ => return Err(bad("teacher kind")),
        }
        let target = r.var()?;
        let correct = r.var()?;
        let mean = Scalar::decode(r)?;
        let first = Scalar::decode(r)?;
        let remaining = Scalar::decode(r)?;
        let objective = Scalar::decode(r)?;
        let first_weight = Scalar::decode(r)?;
        let first_correct = r.bool()?;
        let last_correct = r.bool()?;
        let eos_correct = r.bool()?;
        let first_argmax = u32_read(r)?;
        let first_gold = u32_read(r)?;
        let eos_probability = Scalar::decode(r)?;
        let gold_probability = Scalar::decode(r)?;
        let argmax_probability = Scalar::decode(r)?;
        let difference = r.opt(|r| {
            Ok(Difference {
                index: u32_read(r)?,
                gold: u32_read(r)?,
                actual: r.opt(u32_read)?,
                argmax: u32_read(r)?,
                log_probability: Scalar::decode(r)?,
                margin: Scalar::decode(r)?,
            })
        })?;
        let byte_difference = r.opt(|r| r.var())?;
        let difference_field = r.opt(text)?;
        let n = count(r, 32)?;
        let mut field_accuracy = Vec::with_capacity(n);
        for _ in 0..n {
            field_accuracy.push(FieldAccuracy {
                field: text(r)?,
                correct: r.var()?,
                total: r.var()?,
            });
        }
        if field_accuracy.windows(2).any(|f| f[0].field >= f[1].field) {
            return Err(bad("teacher field ordering/duplicate"));
        }
        Ok(Self::Measured(Box::new(TeacherStats {
            target,
            correct,
            mean,
            first,
            remaining,
            objective,
            first_weight,
            first_correct,
            last_correct,
            eos_correct,
            first_argmax,
            first_gold,
            eos_probability,
            gold_probability,
            argmax_probability,
            difference,
            byte_difference,
            difference_field,
            field_accuracy,
            prompt_matches: r.bool()?,
            answer_roundtrip: r.bool()?,
        })))
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
enum Finish {
    NotStarted,
    Eos,
    Length,
    Error,
}
impl Finish {
    fn tag(self) -> u8 {
        match self {
            Self::NotStarted => 0,
            Self::Eos => 1,
            Self::Length => 2,
            Self::Error => 3,
        }
    }
    fn read(r: &mut Reader<'_>) -> Result<Self> {
        match r.byte()? {
            0 => Ok(Self::NotStarted),
            1 => Ok(Self::Eos),
            2 => Ok(Self::Length),
            3 => Ok(Self::Error),
            _ => Err(bad("finish")),
        }
    }
}
#[derive(Clone, Debug, Serialize)]
struct EvalRow {
    ordinal: u32,
    case: Hash,
    prompt: Hash,
    prompt_len: u32,
    native_prompt: Hash,
    provided: Vec<i64>,
    excluded: Vec<i64>,
    tokens: Vec<u32>,
    eos: Option<u32>,
    started: bool,
    completed: bool,
    finish: Finish,
    error: Option<String>,
    error_class: Option<String>,
    interruption: Option<StopReason>,
    effective_timeout: Option<u64>,
    // generated count, first-token ms, generation ms, KV bytes, attention workspace bytes.
    timing: Option<[u64; 5]>,
    retained: Vec<u32>,
    teacher: TeacherRecord,
    diagnostic: Option<Scalar>,
}
fn stop_tag(s: StopReason) -> u8 {
    match s {
        StopReason::Cancelled => 0,
        StopReason::TimeBudget => 1,
        StopReason::ResourceLimit => 2,
        StopReason::ResourceObservationFailed => 3,
        StopReason::IntegrityFail => 4,
        StopReason::QualityGuard => 5,
        StopReason::TokenBudget => 6,
        StopReason::AuditIncomplete => 7,
    }
}
fn stop_read(r: &mut Reader<'_>) -> Result<StopReason> {
    match r.byte()? {
        0 => Ok(StopReason::Cancelled),
        1 => Ok(StopReason::TimeBudget),
        2 => Ok(StopReason::ResourceLimit),
        3 => Ok(StopReason::ResourceObservationFailed),
        4 => Ok(StopReason::IntegrityFail),
        5 => Ok(StopReason::QualityGuard),
        6 => Ok(StopReason::TokenBudget),
        7 => Ok(StopReason::AuditIncomplete),
        _ => Err(bad("stop reason")),
    }
}
impl EvalRow {
    fn encode(&self, b: &mut Vec<u8>) {
        put_varint(b, u64::from(self.ordinal));
        b.extend(self.case);
        b.extend(self.prompt);
        put_varint(b, u64::from(self.prompt_len));
        b.extend(self.native_prompt);
        ids(b, &self.provided);
        ids(b, &self.excluded);
        integers(b, &self.tokens);
        optional(b, self.eos.as_ref(), |b, n| put_varint(b, u64::from(*n)));
        b.push(u8::from(self.started));
        b.push(u8::from(self.completed));
        b.push(self.finish.tag());
        optional(b, self.error.as_ref(), |b, s| string(b, s));
        optional(b, self.error_class.as_ref(), |b, s| string(b, s));
        optional(b, self.interruption.as_ref(), |b, s| b.push(stop_tag(*s)));
        optional(b, self.effective_timeout.as_ref(), |b, n| put_varint(b, *n));
        optional(b, self.timing.as_ref(), |b, ns| {
            for n in ns {
                put_varint(b, *n);
            }
        });
        integers(b, &self.retained);
        self.teacher.encode(b);
        optional(b, self.diagnostic.as_ref(), |b, s| s.encode(b));
    }
    fn decode(r: &mut Reader<'_>) -> Result<Self> {
        Ok(Self {
            ordinal: u32_read(r)?,
            case: digest_read(r)?,
            prompt: digest_read(r)?,
            prompt_len: u32_read(r)?,
            native_prompt: digest_read(r)?,
            provided: ids_read(r)?,
            excluded: ids_read(r)?,
            tokens: integers_read(r, MAX_TOKENS)?,
            eos: r.opt(u32_read)?,
            started: r.bool()?,
            completed: r.bool()?,
            finish: Finish::read(r)?,
            error: r.opt(text)?,
            error_class: r.opt(text)?,
            interruption: r.opt(stop_read)?,
            effective_timeout: r.opt(|r| r.var())?,
            timing: r.opt(|r| Ok([r.var()?, r.var()?, r.var()?, r.var()?, r.var()?]))?,
            retained: integers_read(r, 128)?,
            teacher: TeacherRecord::decode(r)?,
            diagnostic: r.opt(Scalar::decode)?,
        })
    }
}
#[derive(Clone, Debug, Serialize)]
struct EvalPayload {
    run: Hash,
    binding: Hash,
    source: Hash,
    model: Hash,
    tokenizer: Hash,
    architecture: Hash,
    step: u64,
    new_updates: u64,
    kind: PanelKind,
    expected: u32,
    rows: Vec<EvalRow>,
}
impl EvalPayload {
    fn encode(&self, b: &mut Vec<u8>) {
        for h in [
            self.run,
            self.binding,
            self.source,
            self.model,
            self.tokenizer,
            self.architecture,
        ] {
            b.extend(h);
        }
        put_varint(b, self.step);
        put_varint(b, self.new_updates);
        b.push(self.kind.tag());
        put_varint(b, u64::from(self.expected));
        put_varint(b, self.rows.len() as u64);
        for row in &self.rows {
            row.encode(b);
        }
    }
    fn decode(r: &mut Reader<'_>) -> Result<Self> {
        let run = digest_read(r)?;
        let binding = digest_read(r)?;
        let source = digest_read(r)?;
        let model = digest_read(r)?;
        let tokenizer = digest_read(r)?;
        let architecture = digest_read(r)?;
        let step = r.var()?;
        let new_updates = r.var()?;
        let kind = PanelKind::read(r)?;
        let expected = u32_read(r)?;
        let n = count(r, MAX_ROWS)?;
        let rows = (0..n)
            .map(|_| EvalRow::decode(r))
            .collect::<Result<Vec<_>>>()?;
        if expected as usize > MAX_ROWS
            || rows
                .iter()
                .map(|r| r.ordinal)
                .collect::<BTreeSet<_>>()
                .len()
                != rows.len()
        {
            return Err(bad("evaluation membership"));
        }
        Ok(Self {
            run,
            binding,
            source,
            model,
            tokenizer,
            architecture,
            step,
            new_updates,
            kind,
            expected,
            rows,
        })
    }
}

#[derive(Clone, Debug, Serialize)]
struct EvalDecision {
    run: Hash,
    binding: Hash,
    dev: Hash,
    watch: Hash,
    step: u64,
    before: [u64; 3],
    after: [u64; 3],
    applied: bool,
    quality_stop: bool,
}
impl EvalDecision {
    fn encode(&self, b: &mut Vec<u8>) {
        for h in [self.run, self.binding, self.dev, self.watch] {
            b.extend(h);
        }
        put_varint(b, self.step);
        for n in self.before.into_iter().chain(self.after) {
            put_varint(b, n);
        }
        b.push(u8::from(self.applied));
        b.push(u8::from(self.quality_stop));
    }
    fn decode(r: &mut Reader<'_>) -> Result<Self> {
        Ok(Self {
            run: digest_read(r)?,
            binding: digest_read(r)?,
            dev: digest_read(r)?,
            watch: digest_read(r)?,
            step: r.var()?,
            before: [r.var()?, r.var()?, r.var()?],
            after: [r.var()?, r.var()?, r.var()?],
            applied: r.bool()?,
            quality_stop: r.bool()?,
        })
    }
}
#[derive(Clone, Debug, Serialize)]
struct EvaluationRef {
    payload: FileRef,
    native: Option<CheckpointRef>,
}
#[derive(Clone, Debug, Default, Serialize)]
struct Score {
    planned: u64,
    completed: u64,
    exact: u64,
    entity: u64,
    event: u64,
    errors: u64,
    qa: [u64; 2],
    aux: [u64; 2],
    base: [u64; 2],
}
impl Score {
    fn encode(&self, b: &mut Vec<u8>) {
        for n in [
            self.planned,
            self.completed,
            self.exact,
            self.entity,
            self.event,
            self.errors,
        ]
        .into_iter()
        .chain(self.qa)
        .chain(self.aux)
        .chain(self.base)
        {
            put_varint(b, n);
        }
    }
    fn decode(r: &mut Reader<'_>) -> Result<Self> {
        Ok(Self {
            planned: r.var()?,
            completed: r.var()?,
            exact: r.var()?,
            entity: r.var()?,
            event: r.var()?,
            errors: r.var()?,
            qa: [r.var()?, r.var()?],
            aux: [r.var()?, r.var()?],
            base: [r.var()?, r.var()?],
        })
    }
}
#[derive(Clone, Debug, Serialize)]
struct SegmentReceipt {
    run: Hash,
    binding: Hash,
    segment: u32,
    parent: Option<FileRef>,
    native: Option<CheckpointRef>,
    evaluations: Vec<EvaluationRef>,
    decisions: Vec<FileRef>,
    guard: [u64; 3],
    stop: Vec<StopReason>,
    complete: bool,
    resume: bool,
    candidate: bool,
    save_error: Option<String>,
    updates: u64,
    generations: u64,
    teachers: u64,
    elapsed: Scalar,
    cleanup: Scalar,
    draws: Vec<Draw>,
}
impl SegmentReceipt {
    fn encode(&self, b: &mut Vec<u8>) {
        b.extend(self.run);
        b.extend(self.binding);
        put_varint(b, u64::from(self.segment));
        optional(b, self.parent.as_ref(), |b, r| r.encode(b));
        optional(b, self.native.as_ref(), |b, r| r.encode(b));
        put_varint(b, self.evaluations.len() as u64);
        for e in &self.evaluations {
            e.payload.encode(b);
            optional(b, e.native.as_ref(), |b, r| r.encode(b));
        }
        put_varint(b, self.decisions.len() as u64);
        for d in &self.decisions {
            d.encode(b);
        }
        for n in self.guard {
            put_varint(b, n);
        }
        put_varint(b, self.stop.len() as u64);
        for s in &self.stop {
            b.push(stop_tag(*s));
        }
        for f in [self.complete, self.resume, self.candidate] {
            b.push(u8::from(f));
        }
        optional(b, self.save_error.as_ref(), |b, s| string(b, s));
        for n in [self.updates, self.generations, self.teachers] {
            put_varint(b, n);
        }
        self.elapsed.encode(b);
        self.cleanup.encode(b);
        put_varint(b, self.draws.len() as u64);
        for d in &self.draws {
            integers(b, &d.indices);
            for n in [d.sampler, d.input, d.target] {
                put_varint(b, n);
            }
        }
    }
    fn decode(r: &mut Reader<'_>) -> Result<Self> {
        let run = digest_read(r)?;
        let binding = digest_read(r)?;
        let segment = u32_read(r)?;
        let parent = r.opt(FileRef::decode)?;
        let native = r.opt(CheckpointRef::decode)?;
        let n = count(r, 2048)?;
        let mut evaluations = Vec::with_capacity(n);
        for _ in 0..n {
            evaluations.push(EvaluationRef {
                payload: FileRef::decode(r)?,
                native: r.opt(CheckpointRef::decode)?,
            });
        }
        let n = count(r, 512)?;
        let decisions = (0..n)
            .map(|_| FileRef::decode(r))
            .collect::<Result<Vec<_>>>()?;
        let guard = [r.var()?, r.var()?, r.var()?];
        let n = count(r, 8)?;
        let stop = (0..n).map(|_| stop_read(r)).collect::<Result<Vec<_>>>()?;
        if stop
            .iter()
            .map(|s| stop_tag(*s))
            .collect::<BTreeSet<_>>()
            .len()
            != stop.len()
        {
            return Err(bad("duplicate stop"));
        }
        let complete = r.bool()?;
        let resume = r.bool()?;
        let candidate = r.bool()?;
        let save_error = r.opt(text)?;
        let updates = r.var()?;
        let generations = r.var()?;
        let teachers = r.var()?;
        let elapsed = Scalar::decode(r)?;
        let cleanup = Scalar::decode(r)?;
        let n = count(r, 512)?;
        let mut draws = Vec::with_capacity(n);
        for _ in 0..n {
            draws.push(Draw {
                indices: integers_read(r, 256)?,
                sampler: r.var()?,
                input: r.var()?,
                target: r.var()?,
            });
        }
        Ok(Self {
            run,
            binding,
            segment,
            parent,
            native,
            evaluations,
            decisions,
            guard,
            stop,
            complete,
            resume,
            candidate,
            save_error,
            updates,
            generations,
            teachers,
            elapsed,
            cleanup,
            draws,
        })
    }
}
#[derive(Clone, Debug, Serialize)]
struct ComparisonReceipt {
    run: Hash,
    binding: Hash,
    terminal: FileRef,
    panels: Vec<(PanelKind, Score)>,
    guard: [u64; 3],
    candidate: bool,
    historical: bool,
}
impl ComparisonReceipt {
    fn encode(&self, b: &mut Vec<u8>) {
        b.extend(self.run);
        b.extend(self.binding);
        self.terminal.encode(b);
        put_varint(b, self.panels.len() as u64);
        for (kind, score) in &self.panels {
            b.push(kind.tag());
            score.encode(b);
        }
        for n in self.guard {
            put_varint(b, n);
        }
        b.push(u8::from(self.candidate));
        b.push(u8::from(self.historical));
    }
    fn decode(r: &mut Reader<'_>) -> Result<Self> {
        let run = digest_read(r)?;
        let binding = digest_read(r)?;
        let terminal = FileRef::decode(r)?;
        let n = count(r, 4)?;
        let panels = (0..n)
            .map(|_| Ok((PanelKind::read(r)?, Score::decode(r)?)))
            .collect::<Result<Vec<_>>>()?;
        Ok(Self {
            run,
            binding,
            terminal,
            panels,
            guard: [r.var()?, r.var()?, r.var()?],
            candidate: r.bool()?,
            historical: r.bool()?,
        })
    }
}

#[derive(Clone, Debug, Serialize)]
#[allow(clippy::large_enum_variant)] // Bodies own their vectors; avoid an extra allocation per terminal read.
enum Record {
    Inputs(Box<RunSnapshot>),
    Evaluation(EvalPayload),
    Decision(EvalDecision),
    Segment(SegmentReceipt),
    Comparison(ComparisonReceipt),
}
impl Record {
    fn encode(&self) -> Result<Vec<u8>> {
        let mut body = Vec::new();
        let kind = match self {
            Self::Inputs(v) => {
                v.encode(&mut body);
                1
            }
            Self::Evaluation(v) => {
                v.encode(&mut body);
                2
            }
            Self::Decision(v) => {
                v.encode(&mut body);
                3
            }
            Self::Segment(v) => {
                v.encode(&mut body);
                4
            }
            Self::Comparison(v) => {
                v.encode(&mut body);
                5
            }
        };
        if body.len() > MAX_FILE - HEADER {
            return Err(bad("file bound"));
        }
        let mut out = b"R3ER".to_vec();
        out.extend(1u16.to_le_bytes());
        out.push(kind);
        out.push(1);
        out.extend((body.len() as u64).to_le_bytes());
        let mut domain = b"R3ER-record-v1\0".to_vec();
        domain.push(kind);
        domain.extend(&body);
        out.extend(hash(&domain));
        out.extend(body);
        // Writer applies the same schema/finite/bounds validation as every reader.
        let _ = Self::decode(&out)?;
        Ok(out)
    }
    fn decode(bytes: &[u8]) -> Result<Self> {
        if bytes.len() < HEADER
            || bytes.len() > MAX_FILE
            || &bytes[..4] != b"R3ER"
            || bytes[4..6] != 1u16.to_le_bytes()
            || bytes[7] != 1
        {
            return Err(bad("magic/version/byte-order/size"));
        }
        let length = u64::from_le_bytes(bytes[8..16].try_into().unwrap());
        if length != (bytes.len() - HEADER) as u64 {
            return Err(bad("payload length"));
        }
        let kind = bytes[6];
        let mut domain = b"R3ER-record-v1\0".to_vec();
        domain.push(kind);
        domain.extend(&bytes[HEADER..]);
        if hash(&domain) != bytes[16..48] {
            return Err(bad("integrity digest"));
        }
        let mut r = Reader::new(&bytes[HEADER..]);
        let record = match kind {
            1 => Self::Inputs(Box::new(RunSnapshot::decode(&mut r)?)),
            2 => Self::Evaluation(EvalPayload::decode(&mut r)?),
            3 => Self::Decision(EvalDecision::decode(&mut r)?),
            4 => Self::Segment(SegmentReceipt::decode(&mut r)?),
            5 => Self::Comparison(ComparisonReceipt::decode(&mut r)?),
            _ => return Err(bad("record kind")),
        };
        if !r.finished() {
            return Err(bad("trailing fields"));
        }
        Ok(record)
    }
    fn identity(bytes: &[u8]) -> Result<Hash> {
        Self::decode(bytes)?;
        Ok(bytes[16..48].try_into().unwrap())
    }
}
fn publish(root: &Path, locator: &str, record: &Record) -> Result<FileRef> {
    let path = owned_path(root, locator, false)?;
    let bytes = record.encode()?;
    publish_new(&path, |file, _| {
        file.write_all(&bytes)?;
        file.seek(SeekFrom::Start(0))?;
        let mut readback = Vec::new();
        file.take(MAX_FILE as u64 + 1).read_to_end(&mut readback)?;
        let _ = Record::decode(&readback)?;
        if readback != bytes {
            return Err(bad("write readback"));
        }
        Ok(())
    })?;
    Ok(FileRef {
        locator: locator.into(),
        digest: hash(&bytes),
    })
}
fn owned_path(root: &Path, locator: &str, existing: bool) -> Result<PathBuf> {
    let path = Path::new(locator);
    if path.as_os_str().is_empty()
        || path
            .components()
            .any(|c| !matches!(c, std::path::Component::Normal(_)))
    {
        return Err(bad("reference outside registered root"));
    }
    let root = root.canonicalize()?;
    let out = root.join(path);
    let checked = if existing {
        out.canonicalize()?
    } else {
        out.parent()
            .ok_or_else(|| bad("reference parent"))?
            .canonicalize()?
    };
    if !checked.starts_with(&root) {
        return Err(bad("reference symlink escape"));
    }
    Ok(out)
}
fn read_record(root: &Path, reference: &FileRef) -> Result<Record> {
    let bytes = neural::read_bounded(&owned_path(root, &reference.locator, true)?, MAX_FILE)?;
    if hash(&bytes) != reference.digest {
        return Err(bad("file reference digest"));
    }
    Record::decode(&bytes)
}

// The old evaluator's numerical teacher routine is retained as a noncanonical adapter.
// Its transient values are extracted into explicit fields; no JSON bytes or Value maps enter records.
fn scalar_value(v: &Value, key: &str) -> Result<Scalar> {
    let n = v[key]
        .as_f64()
        .filter(|n| n.is_finite())
        .ok_or_else(|| bad("missing/nonfinite teacher scalar"))?;
    Ok(Scalar::F64(n))
}
fn bool_value(v: &Value, key: &str) -> Result<bool> {
    v[key].as_bool().ok_or_else(|| bad("missing teacher bool"))
}
fn teacher_record(v: &Value) -> Result<TeacherRecord> {
    if v.is_null() {
        return Ok(TeacherRecord::NotRequested);
    }
    if let Some(error) = v["error"].as_str() {
        return Ok(TeacherRecord::Failed(error.into()));
    }
    let difference = if v["first_difference"].is_null() {
        None
    } else {
        let d = &v["first_difference"];
        Some(Difference {
            index: u32::try_from(progress_u64(d, "index")?).map_err(|_| bad("teacher index"))?,
            gold: u32::try_from(progress_u64(d, "gold_id")?).map_err(|_| bad("teacher token"))?,
            actual: d["actual_id"]
                .as_u64()
                .map(|n| u32::try_from(n).map_err(|_| bad("teacher actual token")))
                .transpose()?,
            argmax: u32::try_from(progress_u64(d, "teacher_argmax")?)
                .map_err(|_| bad("teacher token"))?,
            log_probability: scalar_value(d, "gold_log_probability")?,
            margin: scalar_value(d, "gold_minus_rival_logit")?,
        })
    };
    let fields = v["field_token_accuracy"]
        .as_object()
        .ok_or_else(|| bad("teacher field counters"))?;
    let field_accuracy = fields
        .iter()
        .map(|(field, n)| {
            Ok(FieldAccuracy {
                field: field.clone(),
                correct: n[0].as_u64().ok_or_else(|| bad("teacher correct count"))?,
                total: n[1].as_u64().ok_or_else(|| bad("teacher total count"))?,
            })
        })
        .collect::<Result<Vec<_>>>()?;
    Ok(TeacherRecord::Measured(Box::new(TeacherStats {
        target: progress_u64(v, "target_tokens_including_eos")?,
        correct: progress_u64(v, "teacher_forced_correct_tokens")?,
        mean: scalar_value(v, "mean_nll")?,
        first: scalar_value(v, "first_target_nll")?,
        remaining: scalar_value(v, "remaining_mean_nll")?,
        objective: scalar_value(v, "objective")?,
        first_weight: scalar_value(v, "first_target_weight")?,
        first_correct: bool_value(v, "first_target_correct")?,
        last_correct: bool_value(v, "last_content_correct")?,
        eos_correct: bool_value(v, "eos_correct")?,
        first_argmax: u32::try_from(progress_u64(v, "first_argmax")?)
            .map_err(|_| bad("teacher token"))?,
        first_gold: u32::try_from(progress_u64(v, "first_gold_id")?)
            .map_err(|_| bad("teacher token"))?,
        eos_probability: scalar_value(v, "first_eos_probability")?,
        gold_probability: scalar_value(v, "first_gold_probability")?,
        argmax_probability: scalar_value(v, "first_argmax_probability")?,
        difference,
        byte_difference: v["first_byte_difference"].as_u64(),
        difference_field: v["first_difference_field"].as_str().map(String::from),
        field_accuracy,
        prompt_matches: bool_value(v, "training_prompt_matches_generation")?,
        answer_roundtrip: bool_value(v, "answer_tokenizer_roundtrip")?,
    })))
}
fn token_hash(tokens: &[u32]) -> Hash {
    let mut b = b"R3ER-prompt-v1\0".to_vec();
    integers(&mut b, tokens);
    hash(&b)
}
fn evaluator_source() -> Hash {
    let mut h = Sha256::new();
    h.update(b"R3ER-evaluator-source-v1\0");
    h.update(include_bytes!("experiment_record.rs"));
    h.update(include_bytes!("quality_recovery.rs"));
    h.update(include_bytes!("training.rs"));
    h.finalize().into()
}
fn normal_error_class(error: &str) -> &'static str {
    if error.contains("UTF-8") {
        "strict_utf8"
    } else if error.contains("control token") {
        "control_token"
    } else if error.contains("timeout") {
        "timeout"
    } else if error.contains("cancel") {
        "cancelled"
    } else {
        "generation_or_mapping"
    }
}
fn evaluate_row(
    l: &Loaded,
    e: &Episode,
    ordinal: u32,
    control: &mut RunControl,
    with_teacher: bool,
) -> Result<EvalRow> {
    control.check("case_started")?;
    control.attempted_case_count += 1;
    let prompt = l.tokenizer.prepare(
        &e.request,
        l.model.config.context as u32,
        &l.model.config.id()?,
    )?;
    control.check("prompt_prepared")?;
    let timeout = control.effective_timeout(e.request.limits.timeout_ms)?;
    control.generation_calls += 1;
    let mut tokens = Vec::new();
    let result = l.model.generate_observed(
        &prompt.token_ids,
        e.request.limits.max_tokens as usize,
        timeout,
        &control.cancel,
        &e.id,
        |id| tokens.push(id),
    );
    if matches!(&result, Err(Error::Cancelled)) {
        control.observe(StopReason::Cancelled);
    }
    if result
        .as_ref()
        .err()
        .is_some_and(|e| e.to_string().contains("timeout"))
        && timeout < e.request.limits.timeout_ms
    {
        control.observe(StopReason::TimeBudget);
    }
    if result
        .as_ref()
        .err()
        .is_some_and(|e| e.to_string().contains("nonfinite"))
    {
        control.observe(StopReason::IntegrityFail);
    }
    let _ = control.check("generation_returned");
    let (_, generated, error) = decode_generated(&l.tokenizer, result);
    let finish = match generated.as_ref().map(|g| g.finish.as_str()) {
        Some("stop") => Finish::Eos,
        Some("length") => Finish::Length,
        None => Finish::Error,
        _ => return Err(bad("native finish")),
    };
    let mut row = EvalRow {
        ordinal,
        case: case_hash(e),
        prompt: token_hash(&prompt.token_ids),
        prompt_len: prompt.token_ids.len() as u32,
        native_prompt: unhex(&prompt.token_digest)?,
        provided: prompt.provided,
        excluded: prompt.excluded,
        eos: tokens.iter().position(|t| *t == EOS).map(|n| n as u32),
        tokens,
        started: true,
        completed: control.stop.is_none(),
        finish,
        error_class: error.as_deref().map(normal_error_class).map(String::from),
        error,
        interruption: control.stop,
        effective_timeout: Some(timeout),
        timing: generated.as_ref().map(|g| {
            [
                g.generated as u64,
                g.first_token_ms,
                g.generation_ms,
                g.cache_bytes as u64,
                g.attention_workspace_bytes as u64,
            ]
        }),
        retained: generated
            .as_ref()
            .map(|g| g.retained.iter().map(|n| *n as u32).collect())
            .unwrap_or_default(),
        teacher: TeacherRecord::NotRequested,
        diagnostic: None,
    };
    control.completed_generation_count += usize::from(row.completed);
    if with_teacher && control.check("before_teacher").is_ok() {
        row.teacher = match teacher(l, e, &prompt.token_ids, &row.tokens, control) {
            Ok(v) => teacher_record(&v)?,
            Err(error) => {
                if error.to_string().contains("nonfinite") {
                    control.classify_error(&error);
                }
                TeacherRecord::Failed(error.to_string())
            }
        };
    }
    row.interruption = control.stop;
    Ok(row)
}
fn panel(s: &RunSnapshot, kind: PanelKind) -> Result<&PanelSpec> {
    s.panels
        .iter()
        .find(|p| p.kind == kind)
        .ok_or_else(|| bad("missing panel spec"))
}
fn evaluate(
    s: &RunSnapshot,
    l: &Loaded,
    kind: PanelKind,
    step: u64,
    control: &mut RunControl,
    with_teacher: bool,
) -> Result<EvalPayload> {
    let spec = panel(s, kind)?;
    let mut rows = Vec::new();
    for ordinal in &spec.cases {
        if control.check("panel_next_case").is_err() {
            break;
        }
        let row = evaluate_row(
            l,
            &s.cases[*ordinal as usize],
            *ordinal,
            control,
            with_teacher,
        )?;
        #[cfg(feature = "test-support")]
        let row = {
            let mut row = row;
            if s.tiny_spec {
                row.diagnostic = Some(Scalar::F64(f64::from(0.009906131_f32)));
            }
            row
        };
        rows.push(row);
        if control.check("panel_case_returned").is_err() {
            break;
        }
    }
    let _ = control.check("panel_completed");
    Ok(EvalPayload {
        run: s.run,
        binding: s.binding(),
        source: evaluator_source(),
        model: unhex(&l.model.weight_hash()?)?,
        tokenizer: unhex(&l.tokenizer.semantic_id())?,
        architecture: unhex(&l.model.config.semantic_id()?)?,
        step,
        new_updates: step
            .checked_sub(s.parent.step)
            .ok_or_else(|| bad("evaluation step underflow"))?,
        kind,
        expected: spec.cases.len() as u32,
        rows,
    })
}
fn row_output(row: &EvalRow, l: &Loaded) -> Result<(Option<String>, bool)> {
    let eos = row.tokens.iter().position(|t| *t == EOS);
    if eos.map(|i| i as u32) != row.eos
        || eos.is_some_and(|i| i + 1 != row.tokens.len())
        || (row.finish == Finish::Eos) != eos.is_some()
    {
        return Err(bad("raw EOS/finish mismatch"));
    }
    if row.finish == Finish::Error || row.finish == Finish::NotStarted {
        return Ok((None, true));
    }
    let end = eos.unwrap_or(row.tokens.len());
    let (_, _, decoded_error) = decode_generated(
        &l.tokenizer,
        Ok(neural::transformer::Generated {
            tokens: row.tokens[..end].to_vec(),
            generated: row.tokens.len(),
            finish: if row.finish == Finish::Eos {
                "stop"
            } else {
                "length"
            }
            .into(),
            first_token_ms: 0,
            generation_ms: 0,
            cache_bytes: 0,
            retained: vec![],
            attention_workspace_bytes: 0,
        }),
    );
    if decoded_error != row.error
        || row.error.as_deref().map(normal_error_class) != row.error_class.as_deref()
    {
        return Err(bad("raw tokenizer/error mismatch"));
    }
    let actual = if decoded_error.is_none() {
        Some(l.tokenizer.decode(&row.tokens[..end])?)
    } else {
        None
    };
    let abnormal = !row.started
        || !row.completed
        || row.interruption.is_some()
        || row.finish != Finish::Eos
        || row.error.is_some()
        || actual.as_ref().is_none_or(|s| s.trim().is_empty());
    Ok((actual, abnormal))
}
fn rescore(s: &RunSnapshot, e: &EvalPayload, l: &Loaded, require_complete: bool) -> Result<Score> {
    let spec = panel(s, e.kind)?;
    if e.run != s.run
        || e.binding != s.binding()
        || e.model != unhex(&l.model.weight_hash()?)?
        || e.tokenizer != unhex(&l.tokenizer.semantic_id())?
        || e.architecture != unhex(&l.model.config.semantic_id()?)?
        || e.step
            != s.parent
                .step
                .checked_add(e.new_updates)
                .ok_or_else(|| bad("step overflow"))?
        || e.expected as usize != spec.cases.len()
        || e.rows.len() > spec.cases.len()
    {
        return Err(bad("evaluation binding/model/step"));
    }
    let mut score = Score {
        planned: u64::from(e.expected),
        ..Default::default()
    };
    let mut bases: BTreeMap<String, (u64, u64)> = BTreeMap::new();
    for (row, ordinal) in e.rows.iter().zip(&spec.cases) {
        if row.ordinal != *ordinal {
            return Err(bad("ordered row membership"));
        }
        let case = &s.cases[*ordinal as usize];
        let prompt = l.tokenizer.prepare(
            &case.request,
            l.model.config.context as u32,
            &l.model.config.id()?,
        )?;
        if row.case != case_hash(case)
            || row.prompt != token_hash(&prompt.token_ids)
            || row.native_prompt != unhex(&prompt.token_digest)?
            || row.prompt_len as usize != prompt.token_ids.len()
            || row.provided != prompt.provided
            || row.excluded != prompt.excluded
            || row.tokens.len() > case.request.limits.max_tokens as usize
        {
            return Err(bad("case/prompt/raw binding"));
        }
        let (actual, abnormal) = row_output(row, l)?;
        let exact = strict_answer_match(
            actual.as_deref(),
            &case.answer,
            row.finish == Finish::Eos,
            row.error.is_some(),
        ) && row.completed
            && row.interruption.is_none();
        score.completed += u64::from(row.completed && row.interruption.is_none());
        score.exact += u64::from(exact);
        score.errors += u64::from(abnormal);
        if let (Some(a), Some(g)) = (actual.as_deref().and_then(fields), fields(&case.answer)) {
            score.entity += u64::from(a.0 == g.0);
        }
        score.event += u64::from(
            actual
                .as_deref()
                .and_then(|a| citations(a).ok())
                .is_some_and(|a| Some(a) == citations(&case.answer).ok()),
        );
        let group = if case.family.starts_with("copy/") {
            &mut score.aux
        } else {
            &mut score.qa
        };
        group[0] += u64::from(exact);
        group[1] += 1;
        let b = bases.entry(scene(case).to_owned()).or_default();
        b.0 += u64::from(exact);
        b.1 += 1;
        if let TeacherRecord::Measured(t) = &row.teacher {
            if t.correct > t.target || t.target == 0 {
                return Err(bad("teacher denominator"));
            }
            for x in [
                &t.mean,
                &t.first,
                &t.remaining,
                &t.objective,
                &t.first_weight,
                &t.eos_probability,
                &t.gold_probability,
                &t.argmax_probability,
            ] {
                x.finite()?;
            }
            if let Some(d) = &t.difference {
                d.log_probability.finite()?;
                d.margin.finite()?;
            }
        }
        if let Some(d) = &row.diagnostic
            && row.error.is_none()
        {
            d.finite()?;
        }
    }
    score.base = [
        bases.values().filter(|(a, b)| *a == 4 && *b == 4).count() as u64,
        bases.len() as u64,
    ];
    if require_complete && (score.completed != score.planned || e.rows.len() != spec.cases.len()) {
        return Err(bad("INCOMPLETE evaluation"));
    }
    Ok(score)
}
fn read_inputs(root: &Path) -> Result<RunSnapshot> {
    let bytes = neural::read_bounded(&root.join("inputs.r3er"), MAX_FILE)?;
    match Record::decode(&bytes)? {
        Record::Inputs(s) => Ok(*s),
        _ => Err(bad("inputs kind")),
    }
}
fn native_reference(
    root: &Path,
    locator: &str,
    s: &RunSnapshot,
    l: &Loaded,
    segment: u32,
) -> Result<CheckpointRef> {
    let state = l.manifest.training.as_ref();
    Ok(CheckpointRef {
        file: FileRef {
            locator: locator.into(),
            digest: unhex(&file_hash(&owned_path(root, locator, true)?)?)?,
        },
        run: s.run,
        segment,
        model: unhex(&l.model.weight_hash()?)?,
        tokenizer: unhex(&l.tokenizer.semantic_id())?,
        architecture: unhex(&l.model.config.semantic_id()?)?,
        step: state.map_or(l.manifest.trained_steps as u64, |s| s.step as u64),
        adam: if state.is_some() {
            Some(unhex(&optimizer_hash(&l.optimizer)?)?)
        } else {
            None
        },
        counters: state.map_or([0; 3], |s| {
            [s.consumed_tokens, s.target_tokens, s.sampler_state]
        }),
    })
}
fn resolve_native(root: &Path, s: &RunSnapshot, n: &CheckpointRef, resume: bool) -> Result<Loaded> {
    if n.run != s.run
        || n.tokenizer != s.parent.tokenizer
        || n.architecture != s.parent.architecture
        || (resume && n.adam.is_none())
    {
        return Err(bad("native run/tokenizer/architecture/kind"));
    }
    let path = owned_path(root, &n.file.locator, true)?;
    if unhex(&file_hash(&path)?)? != n.file.digest {
        return Err(bad("native physical digest"));
    }
    let l = checkpoint::load(&path, Device::Cpu, n.adam.is_some())?;
    let observed = native_reference(root, &n.file.locator, s, &l, n.segment)?;
    if observed.model != n.model
        || observed.tokenizer != n.tokenizer
        || observed.architecture != n.architecture
        || observed.step != n.step
        || observed.adam != n.adam
        || observed.counters != n.counters
        || observed.file.digest != n.file.digest
    {
        return Err(bad("native content/step/Adam/cursor"));
    }
    if s.tiny_spec && l.model.config.profile != "TINY_NUMERIC_TEST_ONLY" {
        return Err(bad("small test spec on product model"));
    }
    Ok(l)
}

#[derive(Subcommand)]
pub enum Action {
    /// Continue an owned binary experiment; historical imports cannot train or resume.
    Run {
        #[arg(long)]
        root: PathBuf,
        #[arg(long)]
        resume: Option<String>,
    },
    /// Verify all native references, decisions and fixed panels before publishing comparison.
    Close {
        #[arg(long)]
        root: PathBuf,
        #[arg(long)]
        terminal: String,
    },
    /// Explicit read-only conversion and recount of an existing endpoint.
    Import {
        #[arg(long)]
        policy: PathBuf,
        #[arg(long)]
        endpoint: PathBuf,
        #[arg(long)]
        evaluation: PathBuf,
        #[arg(long)]
        cross: PathBuf,
        #[arg(long)]
        ordinary: PathBuf,
        #[arg(long)]
        output: PathBuf,
    },
    /// Compare exactly 16 dev and 16 ordinary generations, selected without output scores.
    Parity {
        #[arg(long)]
        root: PathBuf,
        #[arg(long)]
        output: PathBuf,
    },
    /// Equal-content, uncompressed storage measurements; never decision input.
    Bench {
        #[arg(long)]
        root: PathBuf,
        #[arg(long)]
        terminal: String,
        #[arg(long)]
        output: PathBuf,
    },
    #[cfg(feature = "test-support")]
    Fixture {
        #[arg(long)]
        output: PathBuf,
    },
    #[cfg(feature = "test-support")]
    FixtureFork {
        #[arg(long)]
        from: PathBuf,
        #[arg(long)]
        output: PathBuf,
        #[arg(long)]
        untrained: bool,
    },
    #[cfg(feature = "test-support")]
    FixtureCheck {
        #[arg(long, required = true)]
        roots: Vec<PathBuf>,
    },
}
pub(super) fn command(action: Action) -> Result<()> {
    let mut control = RunControl::command(false)?;
    control.deadline = control.start + Duration::from_secs(1800);
    match action {
        Action::Run { root, resume } => run_native(&root, resume.as_deref(), &mut control),
        Action::Close { root, terminal } => {
            close_native(&root, &terminal, &mut control).map(|_| ())
        }
        Action::Import {
            policy,
            endpoint,
            evaluation,
            cross,
            ordinary,
            output,
        } => import_legacy(
            &policy,
            &endpoint,
            &evaluation,
            &cross,
            &ordinary,
            &output,
            &mut control,
        ),
        Action::Parity { root, output } => parity(&root, &output, &mut control),
        Action::Bench {
            root,
            terminal,
            output,
        } => bench(&root, &terminal, &output, &mut control),
        #[cfg(feature = "test-support")]
        Action::Fixture { output } => fixture(&output),
        #[cfg(feature = "test-support")]
        Action::FixtureFork {
            from,
            output,
            untrained,
        } => fixture_fork(&from, &output, untrained),
        #[cfg(feature = "test-support")]
        Action::FixtureCheck { roots } => fixture_check(&roots),
    }
}
fn reference(root: &Path, locator: &str) -> Result<FileRef> {
    Ok(FileRef {
        locator: locator.into(),
        digest: unhex(&file_hash(&owned_path(root, locator, true)?)?)?,
    })
}
fn segment(root: &Path, r: &FileRef, s: &RunSnapshot) -> Result<SegmentReceipt> {
    let Record::Segment(t) = read_record(root, r)? else {
        return Err(bad("terminal kind"));
    };
    if t.run != s.run
        || t.binding != s.binding()
        || t.candidate && (t.resume || !t.complete || !t.stop.is_empty())
    {
        return Err(bad("terminal run/binding/flags"));
    }
    t.elapsed.finite()?;
    t.cleanup.finite()?;
    Ok(t)
}
fn lineage(root: &Path, s: &RunSnapshot, last: &FileRef) -> Result<Vec<(FileRef, SegmentReceipt)>> {
    let mut chain = Vec::new();
    let mut cursor = Some(last.clone());
    let mut seen = BTreeSet::new();
    while let Some(r) = cursor {
        if chain.len() >= 64 || !seen.insert(r.digest) {
            return Err(bad("terminal lineage cycle/bound"));
        }
        let t = segment(root, &r, s)?;
        cursor = t.parent.clone();
        chain.push((r, t));
    }
    chain.reverse();
    for (i, (_, t)) in chain.iter().enumerate() {
        if t.segment as usize != i || i > 0 && !chain[i - 1].1.resume {
            return Err(bad("non-resumable or discontinuous lineage"));
        }
        let n = t
            .native
            .as_ref()
            .ok_or_else(|| bad("terminal PendingNativeBinding"))?;
        if n.segment != t.segment
            || t.save_error.is_some()
            || t.updates
                != n.step
                    .checked_sub(s.parent.step)
                    .ok_or_else(|| bad("native clock"))?
        {
            return Err(bad("terminal native/step/save mismatch"));
        }
        resolve_native(root, s, n, true)?;
        let start = if i == 0 { 0 } else { chain[i - 1].1.updates };
        if t.updates < start
            || t.updates > s.tape.len() as u64
            || t.draws.len() as u64 != t.updates - start
        {
            return Err(bad("terminal optimizer/tape clock"));
        }
        for (a, b) in t
            .draws
            .iter()
            .zip(&s.tape[start as usize..t.updates as usize])
        {
            if a.indices != b.indices
                || a.sampler != b.sampler
                || a.input != b.input
                || a.target != b.target
            {
                return Err(bad("terminal tape exposure changed"));
            }
        }
        let input: u64 = s.tape[..t.updates as usize].iter().map(|d| d.input).sum();
        let target: u64 = s.tape[..t.updates as usize].iter().map(|d| d.target).sum();
        let sampler = s
            .tape
            .get(t.updates.saturating_sub(1) as usize)
            .filter(|_| t.updates > 0)
            .map_or(s.parent.counters[2], |d| d.sampler);
        if n.counters
            != [
                s.parent.counters[0] + input,
                s.parent.counters[1] + target,
                sampler,
            ]
        {
            return Err(bad("native exposure clock"));
        }
        let clean_time = t.stop == [StopReason::TimeBudget]
            && t.save_error.is_none()
            && !t.complete
            && !s.historical;
        if t.resume != clean_time {
            return Err(bad("terminal resume eligibility"));
        }
    }
    Ok(chain)
}
fn payload(root: &Path, s: &RunSnapshot, r: &EvaluationRef) -> Result<(EvalPayload, Loaded)> {
    let Record::Evaluation(e) = read_record(root, &r.payload)? else {
        return Err(bad("evaluation kind"));
    };
    let n = r
        .native
        .as_ref()
        .ok_or_else(|| bad("PendingNativeBinding"))?;
    let l = resolve_native(root, s, n, true)?;
    if e.step != n.step
        || e.model != n.model
        || e.tokenizer != n.tokenizer
        || e.architecture != n.architecture
    {
        return Err(bad("evaluation/native identity"));
    }
    rescore(s, &e, &l, false)?;
    Ok((e, l))
}
fn decision_for(
    s: &RunSnapshot,
    dev: &EvalPayload,
    watch: &EvalPayload,
    before: [u64; 3],
    d: Hash,
    w: Hash,
    l: &Loaded,
) -> Result<EvalDecision> {
    let ds = rescore(s, dev, l, true)?;
    let ws = rescore(s, watch, l, true)?;
    if dev.kind != PanelKind::Dev
        || watch.kind != PanelKind::Watch
        || dev.step != watch.step
        || dev.model != watch.model
    {
        return Err(bad("guard panel pair"));
    }
    let mut after = before;
    // Zero-step observation does not increment the existing guard streak policy.
    let quality_stop = dev.new_updates > 0
        && guard_counts(
            s.baseline,
            [ds.exact, ws.exact, ds.errors + ws.errors],
            &mut after,
            ds.planned + ws.planned,
        );
    Ok(EvalDecision {
        run: s.run,
        binding: s.binding(),
        dev: d,
        watch: w,
        step: dev.step,
        before,
        after,
        applied: true,
        quality_stop,
    })
}
struct History {
    evaluations: BTreeMap<(u64, PanelKind), EvaluationRef>,
    decisions: BTreeMap<u64, (FileRef, EvalDecision)>,
    guard: [u64; 3],
    quality: bool,
}
fn verified_history(
    root: &Path,
    s: &RunSnapshot,
    chain: &[(FileRef, SegmentReceipt)],
) -> Result<History> {
    let mut h = History {
        evaluations: BTreeMap::new(),
        decisions: BTreeMap::new(),
        guard: [0; 3],
        quality: false,
    };
    for (_, t) in chain {
        for r in &t.evaluations {
            let (e, _) = payload(root, s, r)?;
            if r.native.as_ref().unwrap().segment > t.segment
                || e.step > t.native.as_ref().unwrap().step
            {
                return Err(bad("future native evaluation binding"));
            }
            let key = (e.step, e.kind);
            if let Some(old) = h.evaluations.get(&key)
                && old.payload.digest != r.payload.digest
            {
                return Err(bad("ambiguous same-step raw observation"));
            }
            h.evaluations.insert(key, r.clone());
        }
        for r in &t.decisions {
            let Record::Decision(d) = read_record(root, r)? else {
                return Err(bad("decision kind"));
            };
            if let Some((old, _)) = h.decisions.get(&d.step) {
                if old.digest != r.digest {
                    return Err(bad("duplicate different guard decision"));
                }
                continue;
            }
            let dr = h
                .evaluations
                .get(&(d.step, PanelKind::Dev))
                .ok_or_else(|| bad("guard dev missing"))?;
            let wr = h
                .evaluations
                .get(&(d.step, PanelKind::Watch))
                .ok_or_else(|| bad("guard watch missing"))?;
            let (dev, l) = payload(root, s, dr)?;
            let (watch, _) = payload(root, s, wr)?;
            let expected = decision_for(
                s,
                &dev,
                &watch,
                h.guard,
                Record::identity(&neural::read_bounded(
                    &owned_path(root, &dr.payload.locator, true)?,
                    MAX_FILE,
                )?)?,
                Record::identity(&neural::read_bounded(
                    &owned_path(root, &wr.payload.locator, true)?,
                    MAX_FILE,
                )?)?,
                &l,
            )?;
            let mut a = Vec::new();
            let mut b = Vec::new();
            d.encode(&mut a);
            expected.encode(&mut b);
            if a != b {
                return Err(bad("guard before/after/identity"));
            }
            h.guard = d.after;
            h.quality |= d.quality_stop;
            h.decisions.insert(d.step, (r.clone(), d));
        }
        if h.guard != t.guard
            || h.quality && (!t.stop.contains(&StopReason::QualityGuard) || t.resume)
        {
            return Err(bad("guard terminal disagreement"));
        }
        if !s.historical
            && h.evaluations.keys().any(|(step, kind)| {
                *kind == PanelKind::Dev
                    && (*step < t.native.as_ref().unwrap().step || t.complete)
                    && !h.decisions.contains_key(step)
            })
        {
            return Err(bad("optimizer advanced past pending evaluation decision"));
        }
    }
    Ok(h)
}
fn eligible(
    s: &RunSnapshot,
    scores: &[(PanelKind, Score)],
    quality: bool,
    stops: &[StopReason],
) -> bool {
    if s.historical
        || s.tiny_spec
        || quality
        || stops.iter().any(|r| *r != StopReason::TimeBudget)
        || scores.len() != 4
    {
        return false;
    }
    let find = |kind| scores.iter().find(|(k, _)| *k == kind).map(|(_, v)| v);
    let (Some(d), Some(c), Some(o), Some(w)) = (
        find(PanelKind::Dev),
        find(PanelKind::Cross),
        find(PanelKind::Ordinary),
        find(PanelKind::Watch),
    ) else {
        return false;
    };
    d.exact >= 244
        && d.entity >= 254
        && d.event >= 254
        && c.exact >= 487
        && c.entity >= 507
        && c.event >= 507
        && o.qa[0] >= s.anchor_floor
        && [d, c, o, w]
            .iter()
            .all(|p| p.errors == 0 && p.completed == p.planned)
}
fn close_native(
    root: &Path,
    terminal: &str,
    control: &mut RunControl,
) -> Result<ComparisonReceipt> {
    if root.join("close-stop.r3er").exists() {
        return Err(bad("sticky previous close failure"));
    }
    let result = close_native_inner(root, terminal, control);
    if let Err(error) = &result {
        control.classify_error(error);
        if let Ok(s) = read_inputs(root)
            && let Ok(r) = reference(root, terminal)
            && let Ok(mut t) = segment(root, &r, &s)
        {
            t.stop.extend(
                control
                    .observed
                    .iter()
                    .copied()
                    .filter(|r| !t.stop.contains(r))
                    .collect::<Vec<_>>(),
            );
            t.complete = false;
            t.resume = false;
            t.candidate = false;
            let _ = publish(root, "close-stop.r3er", &Record::Segment(t));
        }
    }
    result
}
fn close_native_inner(
    root: &Path,
    terminal: &str,
    control: &mut RunControl,
) -> Result<ComparisonReceipt> {
    control.check("binary_close_start")?;
    let s = read_inputs(root)?;
    let last = reference(root, terminal)?;
    let chain = lineage(root, &s, &last)?;
    let h = verified_history(root, &s, &chain)?;
    let t = &chain.last().ok_or_else(|| bad("empty lineage"))?.1;
    if !t.complete || t.resume || t.save_error.is_some() {
        return Err(bad("incomplete final terminal"));
    }
    let step = t.native.as_ref().unwrap().step;
    if !s.historical && !h.decisions.contains_key(&step) {
        return Err(bad("final guard decision pending"));
    }
    let mut scores = Vec::new();
    for kind in [
        PanelKind::Dev,
        PanelKind::Watch,
        PanelKind::Cross,
        PanelKind::Ordinary,
    ] {
        control.check("binary_close_panel")?;
        let r = h
            .evaluations
            .get(&(step, kind))
            .ok_or_else(|| bad("missing final panel"))?;
        let (e, l) = payload(root, &s, r)?;
        scores.push((kind, rescore(&s, &e, &l, true)?));
    }
    let stops: Vec<_> = chain
        .iter()
        .flat_map(|(_, t)| t.stop.iter().copied())
        .collect();
    let candidate = eligible(&s, &scores, h.quality, &stops);
    if candidate != t.candidate {
        return Err(bad("terminal candidate disagrees with raw recount"));
    }
    let result = ComparisonReceipt {
        run: s.run,
        binding: s.binding(),
        terminal: last,
        panels: scores,
        guard: h.guard,
        candidate,
        historical: s.historical,
    };
    control.check("binary_close_publish")?;
    publish(root, "comparison.r3er", &Record::Comparison(result.clone()))?;
    for (kind, score) in &result.panels {
        println!(
            "PANEL={} full={}/{} entity={} event={} QA={}/{} aux={}/{} errors={}",
            kind.name(),
            score.exact,
            score.planned,
            score.entity,
            score.event,
            score.qa[0],
            score.qa[1],
            score.aux[0],
            score.aux[1],
            score.errors
        );
    }
    println!(
        "CLOSE=VERIFIED candidate={} historical={} CANONICAL_JSON_WRITES=0 LEGACY_JSON_READS=0",
        candidate, s.historical
    );
    Ok(result)
}
#[cfg(feature = "test-support")]
fn fault(s: &RunSnapshot, phase: &str, n: u64, control: &mut RunControl) {
    if !s.tiny_spec {
        return;
    }
    let wanted = std::env::var("R3ER_TEST_STOP").unwrap_or_default();
    if wanted == format!("{phase}:{n}") {
        control.deadline = Instant::now();
    }
    if wanted == format!("cancel-{phase}:{n}") {
        control.cancel.store(true, Ordering::Relaxed);
        control.deadline = Instant::now();
    }
}
#[cfg(not(feature = "test-support"))]
fn fault(_: &RunSnapshot, _: &str, _: u64, _: &mut RunControl) {}
#[allow(clippy::too_many_arguments)] // Existing native save boundary plus the explicit durable reference.
fn save_native(
    root: &Path,
    name: &str,
    s: &RunSnapshot,
    l: &mut Loaded,
    state: &TrainingState,
    adam: &Adam,
    index: u32,
    reason: &str,
) -> Result<CheckpointRef> {
    let path = owned_path(root, name, false)?;
    save_arm(l, state, adam, &path, reason)?;
    let durable = checkpoint::load(&path, Device::Cpu, true)?;
    native_reference(root, name, s, &durable, index)
}
fn run_native(root: &Path, resume: Option<&str>, control: &mut RunControl) -> Result<()> {
    let s = read_inputs(root)?;
    if s.historical || !s.tiny_spec {
        return Err(bad(
            "this repair authorizes no SMALL optimizer updates; historical import cannot resume",
        ));
    }
    let mut h = History {
        evaluations: BTreeMap::new(),
        decisions: BTreeMap::new(),
        guard: [0; 3],
        quality: false,
    };
    let mut parent = None;
    let mut index = 0;
    let initial = if let Some(name) = resume {
        let r = reference(root, name)?;
        let chain = lineage(root, &s, &r)?;
        h = verified_history(root, &s, &chain)?;
        let t = &chain.last().unwrap().1;
        if !t.resume {
            return Err(bad("only a clean time stop can resume"));
        }
        index = t
            .segment
            .checked_add(1)
            .ok_or_else(|| bad("segment overflow"))?;
        parent = Some(r);
        t.native.clone().unwrap()
    } else {
        s.parent.clone()
    };
    let mut l = resolve_native(root, &s, &initial, true)?;
    let mut state = l
        .manifest
        .training
        .clone()
        .ok_or_else(|| bad("training state"))?;
    let mut adam = Adam {
        moments: std::mem::take(&mut l.optimizer),
    };
    let start = state.step as u64 - s.parent.step;
    let name = format!("segment-{index:02}");
    std::fs::create_dir(root.join(&name))?;
    let mut evaluations = Vec::new();
    let mut decisions = Vec::new();
    let mut draws = Vec::new();
    let mut complete = false;
    let outcome = (|| -> Result<()> {
        let episodes: Vec<_> = s
            .train
            .iter()
            .map(|i| s.cases[*i as usize].clone())
            .collect();
        let framed = samples(&episodes, &l.tokenizer, state.config.seq_len)?;
        loop {
            let n = state.step as u64 - s.parent.step;
            l.model.refresh_identity()?;
            if s.eval_steps.contains(&(n as u32)) {
                for kind in [PanelKind::Dev, PanelKind::Watch] {
                    if let std::collections::btree_map::Entry::Vacant(entry) =
                        h.evaluations.entry((state.step as u64, kind))
                    {
                        control.check("binary_before_evaluation")?;
                        let e = evaluate(&s, &l, kind, state.step as u64, control, true)?;
                        let r = publish(
                            root,
                            &format!("{name}/{}-{n:04}.r3er", kind.name()),
                            &Record::Evaluation(e),
                        )?;
                        let er = EvaluationRef {
                            payload: r,
                            native: None,
                        };
                        entry.insert(er.clone());
                        evaluations.push(er);
                    }
                }
                fault(&s, "raw", n, control);
                // Freeze both panels before the decision; cleanup links the actual final native.
                control.check("binary_eval_recorded")?;
                if !h.decisions.contains_key(&(state.step as u64)) {
                    let get = |kind| -> Result<(FileRef, EvalPayload, Hash)> {
                        let r = &h.evaluations[&(state.step as u64, kind)].payload;
                        let bytes =
                            neural::read_bounded(&owned_path(root, &r.locator, true)?, MAX_FILE)?;
                        if hash(&bytes) != r.digest {
                            return Err(bad("pending payload physical hash"));
                        }
                        let Record::Evaluation(e) = Record::decode(&bytes)? else {
                            return Err(bad("pending payload kind"));
                        };
                        Ok((r.clone(), e, Record::identity(&bytes)?))
                    };
                    let (_, dev, dh) = get(PanelKind::Dev)?;
                    let (_, watch, wh) = get(PanelKind::Watch)?;
                    let d = decision_for(&s, &dev, &watch, h.guard, dh, wh, &l)?;
                    let r = publish(
                        root,
                        &format!("{name}/decision-{n:04}.r3er"),
                        &Record::Decision(d.clone()),
                    )?;
                    h.guard = d.after;
                    h.quality |= d.quality_stop;
                    h.decisions.insert(state.step as u64, (r.clone(), d));
                    decisions.push(r);
                }
                // A completed short decision is sticky before another deadline check.
                if h.quality {
                    control.observe(StopReason::QualityGuard);
                }
                let native = save_native(
                    root,
                    &format!("{name}/step-{n:04}.r3m"),
                    &s,
                    &mut l,
                    &state,
                    &adam,
                    index,
                    "RECOVERY_SCREENING",
                )?;
                for kind in [PanelKind::Dev, PanelKind::Watch] {
                    let mut r = h.evaluations[&(state.step as u64, kind)].clone();
                    r.native = Some(native.clone());
                    if let Some(existing) = evaluations
                        .iter_mut()
                        .find(|e| e.payload.digest == r.payload.digest)
                    {
                        *existing = r.clone();
                    } else {
                        evaluations.push(r.clone());
                    }
                    h.evaluations.insert((state.step as u64, kind), r);
                }
                fault(&s, "checkpoint", n, control);
                control.check("binary_checkpoint_recorded")?;
            }
            if n == s.tape.len() as u64 {
                for kind in [PanelKind::Cross, PanelKind::Ordinary] {
                    if h.evaluations.contains_key(&(state.step as u64, kind)) {
                        continue;
                    }
                    control.check("binary_final_panel")?;
                    let e = evaluate(&s, &l, kind, state.step as u64, control, true)?;
                    let r = publish(
                        root,
                        &format!("{name}/{}.r3er", kind.name()),
                        &Record::Evaluation(e),
                    )?;
                    let er = EvaluationRef {
                        payload: r,
                        native: None,
                    };
                    h.evaluations.insert((state.step as u64, kind), er.clone());
                    evaluations.push(er);
                    control.check("binary_final_panel_saved")?;
                }
                complete = true;
                break;
            }
            control.check("binary_before_forward")?;
            let d = &s.tape[n as usize];
            let indices: Vec<_> = d.indices.iter().map(|i| *i as usize).collect();
            let b = batch(&framed, &indices, &Device::Cpu)?;
            let targets: usize = indices
                .iter()
                .map(|i| framed[*i].tokens.len() - framed[*i].response_start)
                .sum();
            if b.tokens as u64 != d.input || targets as u64 != d.target {
                return Err(bad("frozen actual batch denominator"));
            }
            let (ce, obj, actual) = response_loss(
                &l.model.forward(&b.input, Some(&b.valid))?,
                &b,
                state.config.first_target_weight,
            )?;
            control.check("binary_after_forward")?;
            let ce = ce.to_scalar::<f32>()?;
            let objective = obj.to_scalar::<f32>()?;
            if !ce.is_finite() || !objective.is_finite() || actual != targets {
                return Err(bad("nonfinite loss/denominator"));
            }
            let grads = obj.backward()?;
            control.check("binary_after_backward")?;
            let grads = l
                .model
                .vars
                .iter()
                .map(|(k, v)| {
                    Ok((
                        k.clone(),
                        grads
                            .get(v)
                            .ok_or_else(|| bad("missing gradient"))?
                            .detach(),
                    ))
                })
                .collect::<Result<BTreeMap<_, _>>>()?;
            let rate = if s.tiny_spec {
                state.config.lr
            } else {
                progress_lr(
                    if s.lr_policy == 0 { "C" } else { "L" },
                    (n + 1 + s.lr_offset) as usize,
                )?
            };
            control.check("binary_before_optimizer")?;
            adam.step_constant(&l.model.vars, &grads, &state.config, state.step + 1, rate)?;
            state.step += 1;
            state.consumed_tokens += d.input;
            state.target_tokens += d.target;
            state.sampler_state = d.sampler;
            state.train_loss = Some(f64::from(ce));
            state.validation_loss = None;
            draws.push(d.clone());
            println!(
                "ACTUAL_TINY_UPDATE={} MODEL_STEP={} INPUT_TOKENS={} TARGET_TOKENS={} LR_BITS={}",
                draws.len(),
                state.step,
                d.input,
                d.target,
                rate.to_bits()
            );
        }
        Ok(())
    })();
    if let Err(e) = &outcome {
        control.classify_error(e);
    }
    let _ = control.check("binary_before_preservation");
    let cleanup = Instant::now();
    let reason = control
        .stop
        .map_or("SCREENING_BUDGET_REACHED", StopReason::name);
    let saved = save_native(
        root,
        &format!("{name}/final.r3m"),
        &s,
        &mut l,
        &state,
        &adam,
        index,
        reason,
    );
    let save_error = saved.as_ref().err().map(ToString::to_string);
    if let Err(e) = &saved {
        control.classify_error(e);
    }
    let native = saved.ok();
    for r in &mut evaluations {
        if r.native.is_none() {
            r.native = native.clone();
        }
    }
    if evaluations.iter().any(|r| read_record(root,&r.payload).is_ok_and(|v| matches!(v,
        Record::Evaluation(e) if e.rows.len()!=e.expected as usize || e.rows.iter().any(|r|!r.completed || r.interruption.is_some())))) {
        control.observe(StopReason::AuditIncomplete);
    }
    if cleanup.elapsed() > Duration::from_secs(120) {
        control.observe(StopReason::ResourceLimit);
    }
    let _ = control.check("binary_terminal");
    let t = SegmentReceipt {
        run: s.run,
        binding: s.binding(),
        segment: index,
        parent,
        native,
        evaluations,
        decisions,
        guard: h.guard,
        stop: control.observed.clone(),
        complete,
        resume: control.observed == [StopReason::TimeBudget] && !complete && save_error.is_none(),
        candidate: false,
        save_error,
        updates: state.step as u64 - s.parent.step,
        generations: control.generation_calls as u64,
        teachers: control.teacher_calls as u64,
        elapsed: Scalar::F64(control.start.elapsed().as_secs_f64()),
        cleanup: Scalar::F64(cleanup.elapsed().as_secs_f64()),
        draws,
    };
    publish(
        root,
        &format!("{name}/terminal.r3er"),
        &Record::Segment(t.clone()),
    )?;
    println!(
        "SEGMENT={index} NEW_TINY_UPDATES={} NEW_SMALL_UPDATES=0 GENERATIONS={} TEACHERS={} STOP={:?} resume={} complete={} CANONICAL_JSON_WRITES=0 LEGACY_JSON_READS=0",
        t.updates - start,
        t.generations,
        t.teachers,
        t.stop,
        t.resume,
        t.complete
    );
    if t.complete {
        close_native(root, &format!("{name}/terminal.r3er"), control)?;
    }
    match outcome {
        Err(_) if t.resume => Ok(()),
        other => other,
    }
}

fn import_row(v: &Value, e: &Episode, ordinal: u32, l: &Loaded) -> Result<EvalRow> {
    let prompt = l.tokenizer.prepare(
        &e.request,
        l.model.config.context as u32,
        &l.model.config.id()?,
    )?;
    let required = |key: &str| v.get(key).ok_or_else(|| bad("missing legacy row field"));
    let finish = match required("finish_reason")?.as_str() {
        Some("stop") => Finish::Eos,
        Some("length") => Finish::Length,
        None if !v["error"].is_null() => Finish::Error,
        _ => return Err(bad("legacy finish")),
    };
    let g = required("generation")?;
    let timing = if g.is_null() {
        None
    } else {
        Some([
            progress_u64(g, "generated")?,
            progress_u64(g, "first_token_ms")?,
            progress_u64(g, "generation_ms")?,
            progress_u64(g, "cache_bytes")?,
            progress_u64(g, "attention_workspace_bytes")?,
        ])
    };
    let tokens: Vec<u32> = serde_json::from_value(required("raw_tokens")?.clone())?;
    if v["raw_generated_count"] != tokens.len() || v["expected"] != e.answer {
        return Err(bad("legacy tokens/expected"));
    }
    let row = EvalRow {
        ordinal,
        case: case_hash(e),
        prompt: token_hash(&prompt.token_ids),
        prompt_len: u32::try_from(progress_u64(v, "prompt_length")?)
            .map_err(|_| bad("prompt bound"))?,
        native_prompt: unhex(
            v["native_prompt_digest"]
                .as_str()
                .ok_or_else(|| bad("legacy native prompt"))?,
        )?,
        provided: serde_json::from_value(required("provided")?.clone())?,
        excluded: serde_json::from_value(required("excluded")?.clone())?,
        tokens,
        eos: v["eos_index"]
            .as_u64()
            .map(|n| u32::try_from(n).map_err(|_| bad("EOS overflow")))
            .transpose()?,
        started: bool_value(v, "generation_started")?,
        completed: bool_value(v, "generation_completed")?,
        finish,
        error: serde_json::from_value(required("error")?.clone())?,
        error_class: serde_json::from_value(required("error_class")?.clone())?,
        interruption: if required("interruption")?.is_null() {
            None
        } else {
            return Err(bad("legacy interrupted panel"));
        },
        effective_timeout: Some(progress_u64(v, "effective_timeout_ms")?),
        timing,
        retained: if g.is_null() {
            vec![]
        } else {
            serde_json::from_value(g["retained"].clone())?
        },
        teacher: teacher_record(required("teacher_forced_diagnostic_after_generation")?)?,
        diagnostic: None,
    };
    let (actual, abnormal) = row_output(&row, l)?;
    if actual.as_deref() != v["actual"].as_str()
        || bool_value(v, "exact_match")?
            != strict_answer_match(
                actual.as_deref(),
                &e.answer,
                row.finish == Finish::Eos,
                abnormal,
            )
    {
        return Err(bad("legacy strict score disagreement"));
    }
    Ok(row)
}
fn add_cases(cases: &mut Vec<Episode>, items: &[Episode]) -> Result<Vec<u32>> {
    items
        .iter()
        .map(|e| {
            if let Some(i) = cases.iter().position(|c| c.id == e.id) {
                if case_hash(&cases[i]) != case_hash(e) {
                    return Err(bad("same ID different owned content"));
                }
                Ok(i as u32)
            } else {
                let i = cases.len();
                cases.push(e.clone());
                Ok(i as u32)
            }
        })
        .collect()
}
// Two existing legacy schemas: arm panels carry their summary; post-hoc panels
// carry raw rows/binding and keep the summary in their explicit terminal.
fn legacy_panel_summary<'a>(
    raw: &'a Value,
    terminal: &'a Value,
    kind: PanelKind,
    key: &str,
    posthoc: bool,
) -> Result<&'a Value> {
    let summary = match raw.get(key) {
        Some(v) => v,
        None if posthoc
            && matches!(kind, PanelKind::Cross | PanelKind::Ordinary)
            && raw.get("binding").is_some() =>
        {
            terminal
                .get(kind.name())
                .ok_or_else(|| bad("post-hoc terminal summary missing"))?
        }
        None => return Err(bad("legacy panel summary missing")),
    };
    if !summary.is_object() {
        return Err(bad("legacy panel summary null/type"));
    }
    Ok(summary)
}
fn read_legacy_owned(path: &Path) -> Result<(Value, FileRef)> {
    let bytes = neural::read_bounded(path, MAX_FILE)?;
    let reference = FileRef {
        locator: path.canonicalize()?.display().to_string(),
        digest: hash(&bytes),
    };
    Ok((serde_json::from_slice(&bytes)?, reference))
}
#[allow(clippy::too_many_arguments)]
fn import_legacy(
    policy: &Path,
    endpoint: &Path,
    evaluation: &Path,
    cross: &Path,
    ordinary: &Path,
    output: &Path,
    control: &mut RunControl,
) -> Result<()> {
    control.check("legacy_import_start")?;
    // This is the sole explicit compatibility boundary. No normal reader calls it.
    let (p, policy_ref) = read_legacy_owned(policy)?;
    let inputs = load_verified_inputs(&progress_path(&p, "a0")?, Some(&p))?;
    let l = checkpoint::load(endpoint, Device::Cpu, true)?;
    let state = l
        .manifest
        .training
        .as_ref()
        .ok_or_else(|| bad("legacy native training clock"))?;
    let (dev_raw, dev_ref) = read_legacy_owned(evaluation)?;
    let (cross_raw, cross_ref) = read_legacy_owned(cross)?;
    let (ordinary_raw, ordinary_ref) = read_legacy_owned(ordinary)?;
    let raw = [dev_raw, cross_raw, ordinary_raw];
    let raw_refs = [dev_ref, cross_ref, ordinary_ref];
    let terminal_path = cross
        .parent()
        .ok_or_else(|| bad("legacy terminal directory"))?
        .join("result.json");
    let (old_terminal, terminal_ref) = read_legacy_owned(&terminal_path)?;
    if old_terminal["candidate_eligible"] != false || old_terminal["resume_allowed"] != false {
        return Err(bad(
            "historical endpoint eligibility must remain explicitly false",
        ));
    }
    let posthoc = raw[1].get("binding").is_some();
    let registration_path = cross.parent().unwrap().join("registration.json");
    let (registration, registration_ref) = if posthoc {
        let (value, reference) = read_legacy_owned(&registration_path)?;
        (Some(value), Some(reference))
    } else {
        (None, None)
    };
    if let Some(reg) = &registration {
        if reg["native_sha256"] != file_hash(endpoint)?
            || reg["model_hash"] != l.model.weight_hash()?
            || reg["step"] != state.step
            || reg["tokenizer"] != l.tokenizer.semantic_id()
            || reg["policy_sha256"] != hex(&policy_ref.digest)
        {
            return Err(bad("historical registration/native/policy"));
        }
    } else if old_terminal["checkpoint_file_sha256"] != file_hash(endpoint)?
        || old_terminal["model_content_hash"] != l.model.weight_hash()?
        || old_terminal["cumulative_model_step"] != state.step
    {
        return Err(bad("historical terminal/native/clock"));
    }
    if raw[0]["model_content_hash"] != l.model.weight_hash()?
        || raw[0]["final_evaluation_complete"] != true
    {
        return Err(bad("legacy evaluation/model"));
    }
    let initial = checkpoint::load(&progress_path(&p, "parent")?, Device::Cpu, true)?;
    let initial_step = initial
        .manifest
        .training
        .as_ref()
        .ok_or_else(|| bad("legacy parent clock"))?
        .step;
    if raw[0]["new_updates"]
        .as_u64()
        .and_then(|n| (initial_step as u64).checked_add(n))
        != Some(state.step as u64)
    {
        return Err(bad("legacy absolute clock"));
    }
    let source = p["source_digest"]
        .as_str()
        .ok_or_else(|| bad("legacy source provenance"))?;
    let policy_hash = hex(&policy_ref.digest);
    let mut cases = Vec::new();
    let train = add_cases(&mut cases, &inputs.train)?;
    let mut panels = Vec::new();
    let mut converted = Vec::new();
    for (kind, episodes, split, v, rows_key, score_key) in [
        (
            PanelKind::Dev,
            &inputs.dev,
            &inputs.hashes["dev"],
            &raw[0],
            "dev_rows",
            "dev",
        ),
        (
            PanelKind::Watch,
            &inputs.frozen.watch,
            &inputs.hashes["ordinary"],
            &raw[0],
            "watch_rows",
            "watch",
        ),
        (
            PanelKind::Cross,
            &inputs.cross,
            &inputs.hashes["cross"],
            &raw[1],
            "rows",
            "score",
        ),
        (
            PanelKind::Ordinary,
            &inputs.ordinary,
            &inputs.hashes["ordinary"],
            &raw[2],
            "rows",
            "score",
        ),
    ] {
        control.check("legacy_import_panel")?;
        let rows = v[rows_key].as_array().ok_or_else(|| bad("legacy rows"))?;
        let spec = super::PanelSpec {
            id: kind.name(),
            cases: episodes,
            split_hash: split.as_str().ok_or_else(|| bad("split digest"))?,
            policy_hash: &policy_hash,
            source,
            model: &l.model.weight_hash()?,
            step: state.step,
        };
        // Old receipts are explicitly audit-only: don't claim JSON float identity was bit stable.
        // Existing content/token checks remain mandatory; current binding is separately recorded.
        let score = verify_panel_and_rescore(&spec, rows, None, &l, true)?;
        verify_score(
            legacy_panel_summary(v, &old_terminal, kind, score_key, posthoc)?,
            &score,
        )?;
        if matches!(kind, PanelKind::Cross | PanelKind::Ordinary) {
            verify_score(&old_terminal[kind.name()], &score)?;
            if let Some(binding) = v
                .get("binding")
                .or_else(|| v.get("bindings").and_then(|b| b.get(kind.name())))
                && (binding["model_hash"] != l.model.weight_hash()?
                    || binding["model_step"] != state.step
                    || binding["tokenizer_hash"] != l.tokenizer.semantic_id()
                    || binding["dataset_digest"] != *split
                    || binding["complete"] != true
                    || binding["completed_count"] != episodes.len()
                    || binding["decoding"] != "normal_greedy_v1")
            {
                return Err(bad("legacy bound panel/native/dataset mismatch"));
            }
        }
        let ordinals = add_cases(&mut cases, episodes)?;
        let typed = rows
            .iter()
            .zip(episodes)
            .zip(&ordinals)
            .map(|((v, e), i)| import_row(v, e, *i, &l))
            .collect::<Result<Vec<_>>>()?;
        panels.push(PanelSpec {
            kind,
            dataset: unhex(spec.split_hash)?,
            cases: ordinals,
        });
        converted.push((kind, typed, score));
    }
    // Input verification completes before any normal output is published.
    std::fs::create_dir(output)?;
    std::fs::copy(endpoint, output.join("parent.r3m"))?;
    std::fs::File::open(output.join("parent.r3m"))?.sync_all()?;
    if file_hash(endpoint)? != file_hash(&output.join("parent.r3m"))? {
        return Err(bad("native copy changed"));
    }
    let mut run_material = b"R3ER-read-only-import-v1\0".to_vec();
    run_material.extend(unhex(&policy_hash)?);
    run_material.extend(unhex(&file_hash(endpoint)?)?);
    let run = hash(&run_material);
    let mut origins = Vec::new();
    for (role, original) in [
        ("legacy-policy", policy_ref),
        (
            "legacy-endpoint",
            FileRef {
                locator: endpoint.canonicalize()?.display().to_string(),
                digest: unhex(&file_hash(endpoint)?)?,
            },
        ),
        ("legacy-evaluation", raw_refs[0].clone()),
        ("legacy-cross", raw_refs[1].clone()),
        ("legacy-ordinary", raw_refs[2].clone()),
        ("legacy-terminal", terminal_ref),
    ] {
        origins.push(Origin {
            role: role.into(),
            original,
        });
    }
    if let Some(original) = registration_ref {
        origins.push(Origin {
            role: "legacy-post-hoc-registration".into(),
            original,
        });
    }
    let parent = CheckpointRef {
        file: reference(output, "parent.r3m")?,
        run,
        segment: 0,
        model: unhex(&l.model.weight_hash()?)?,
        tokenizer: unhex(&l.tokenizer.semantic_id())?,
        architecture: unhex(&l.model.config.semantic_id()?)?,
        step: state.step as u64,
        adam: Some(unhex(&optimizer_hash(&l.optimizer)?)?),
        counters: [
            state.consumed_tokens,
            state.target_tokens,
            state.sampler_state,
        ],
    };
    let s = RunSnapshot {
        contract: CONTRACT.into(),
        run,
        policy: unhex(&policy_hash)?,
        frozen: unhex(
            inputs.hashes["frozen"]
                .as_str()
                .ok_or_else(|| bad("frozen digest"))?,
        )?,
        source: unhex(source)?,
        parent,
        historical: true,
        tiny_spec: false,
        origins,
        cases,
        train,
        panels,
        tape: vec![],
        eval_steps: vec![],
        lr_policy: match p["lr_policy"].as_str() {
            Some("C") => 0,
            Some("L") => 1,
            _ => return Err(bad("legacy LR policy")),
        },
        lr_offset: progress_u64(&p, "lr_offset")?,
        baseline: [
            progress_u64(&p, "baseline_dev")?,
            progress_u64(&p, "baseline_watch")?,
            progress_u64(&p, "baseline_errors")?,
        ],
        anchor_floor: progress_u64(&p, "anchor_floor")?,
    };
    publish(output, "inputs.r3er", &Record::Inputs(Box::new(s.clone())))?;
    let mut refs = Vec::new();
    for (kind, rows, old_score) in converted {
        let e = EvalPayload {
            run,
            binding: s.binding(),
            source: match kind {
                PanelKind::Dev | PanelKind::Watch => raw_refs[0].digest,
                PanelKind::Cross => raw_refs[1].digest,
                PanelKind::Ordinary => raw_refs[2].digest,
                _ => return Err(bad("legacy quality panel kind")),
            },
            model: s.parent.model,
            tokenizer: s.parent.tokenizer,
            architecture: s.parent.architecture,
            step: s.parent.step,
            new_updates: 0,
            kind,
            expected: panel(&s, kind)?.cases.len() as u32,
            rows,
        };
        let score = rescore(&s, &e, &l, true)?;
        if old_score["exact_matches"] != score.exact
            || old_score["qa"][0] != score.qa[0]
            || old_score["auxiliary"][0] != score.aux[0]
        {
            return Err(bad("legacy/native scorer disagreement"));
        }
        let r = publish(
            output,
            &format!("{}.r3er", kind.name()),
            &Record::Evaluation(e),
        )?;
        refs.push(EvaluationRef {
            payload: r,
            native: Some(s.parent.clone()),
        });
    }
    let t = SegmentReceipt {
        run,
        binding: s.binding(),
        segment: 0,
        parent: None,
        native: Some(s.parent.clone()),
        evaluations: refs,
        decisions: vec![],
        guard: [0; 3],
        stop: vec![],
        complete: true,
        resume: false,
        candidate: false,
        save_error: None,
        updates: 0,
        generations: 0,
        teachers: 0,
        elapsed: Scalar::F64(control.start.elapsed().as_secs_f64()),
        cleanup: Scalar::F64(0.),
        draws: vec![],
    };
    publish(output, "terminal.r3er", &Record::Segment(t))?;
    close_native(output, "terminal.r3er", control)?;
    let note = format!(
        "MODE=READ_ONLY_IMPORT_REAUDIT\nPARSER=serde_json-1.0.151; features=default,std,alloc; float_roundtrip=false\nCONVERTER=R3ER-v1\nCONVERTER_SOURCE={}\nHISTORICAL_IN_MEMORY_FLOAT_BITS=UNKNOWN\nLEGACY_FLOAT_IDENTITY=LEGACY_ROUNDTRIP_UNPROVEN\nHISTORICAL_CANDIDATE_PROMOTION=NOT_AUTHORIZED\nOLD_RESUME_FLAGS=UNCHANGED\nCANONICAL_JSON_WRITES=0\nLEGACY_JSON_READS=EXPLICIT_IMPORT_ONLY (shared recursive frozen validator included)\n",
        hex(&evaluator_source())
    );
    publish_bytes(&output.join("import-report.txt"), note.as_bytes())?;
    Ok(())
}
fn parity(root: &Path, output: &Path, control: &mut RunControl) -> Result<()> {
    let s = read_inputs(root)?;
    if !s.historical || s.tiny_spec {
        return Err(bad("parity requires read-only production import"));
    }
    let l = resolve_native(root, &s, &s.parent, true)?;
    let mut selected = Vec::new();
    for kind in [PanelKind::Dev, PanelKind::Ordinary] {
        let mut cases = panel(&s, kind)?.cases.clone();
        // Entire rule depends only on frozen metadata, never actual output or exact_match.
        cases.sort_by_key(|i| {
            let e = &s.cases[*i as usize];
            (
                e.category,
                e.family.clone(),
                scene(e).to_owned(),
                e.id.clone(),
            )
        });
        let chosen: Vec<_> = (0..16).map(|j| cases[j * cases.len() / 16]).collect();
        selected.push((kind, chosen));
    }
    std::fs::create_dir(output)?;
    let mut registration = s.clone();
    registration.train.clear();
    std::fs::copy(
        owned_path(root, &s.parent.file.locator, true)?,
        output.join("parent.r3m"),
    )?;
    registration.parent.file = reference(output, "parent.r3m")?;
    if registration.parent.file.digest != s.parent.file.digest {
        return Err(bad("parity native copy"));
    }
    for p in &mut registration.panels {
        if let Some((_, ids)) = selected.iter().find(|(k, _)| *k == p.kind) {
            p.cases = ids.clone();
            p.kind = if p.kind == PanelKind::Dev {
                PanelKind::DevParity
            } else {
                PanelKind::OrdinaryParity
            };
        }
    }
    // Registration is a bounded parity test spec, not a replacement H3/S4 quality panel.
    publish(
        output,
        "inputs.r3er",
        &Record::Inputs(Box::new(registration.clone())),
    )?;
    let mut total = 0;
    let mut refs = Vec::new();
    let mut scores = Vec::new();
    let mut failure = None;
    for (kind, chosen) in selected {
        let Record::Evaluation(old) =
            read_record(root, &reference(root, &format!("{}.r3er", kind.name()))?)?
        else {
            return Err(bad("parity original"));
        };
        let mut rows = Vec::new();
        for ordinal in chosen {
            if let Err(e) = control.check("parity_next_case") {
                failure = Some(e);
                break;
            }
            let row = match evaluate_row(&l, &s.cases[ordinal as usize], ordinal, control, false) {
                Ok(r) => r,
                Err(e) => {
                    failure = Some(e);
                    break;
                }
            };
            let expected = old
                .rows
                .iter()
                .find(|r| r.ordinal == ordinal)
                .ok_or_else(|| bad("parity ID"))?;
            let same = row.tokens == expected.tokens
                && row.eos == expected.eos
                && row.finish == expected.finish
                && row.error == expected.error
                && row_output(&row, &l)? == row_output(expected, &l)?
                && row.prompt == expected.prompt
                && row.native_prompt == expected.native_prompt;
            rows.push(row);
            total += 1;
            if !same {
                failure = Some(bad("MODEL_OUTPUT_PARITY_MISMATCH; no retries permitted"));
                break;
            }
        }
        let e = EvalPayload {
            run: s.run,
            binding: registration.binding(),
            source: evaluator_source(),
            model: s.parent.model,
            tokenizer: s.parent.tokenizer,
            architecture: s.parent.architecture,
            step: s.parent.step,
            new_updates: 0,
            kind: if kind == PanelKind::Dev {
                PanelKind::DevParity
            } else {
                PanelKind::OrdinaryParity
            },
            expected: 16,
            rows,
        };
        scores.push((e.kind, rescore(&registration, &e, &l, failure.is_none())?));
        let r = publish(
            output,
            &format!("{}.r3er", kind.name()),
            &Record::Evaluation(e),
        )?;
        refs.push(EvaluationRef {
            payload: r,
            native: Some(registration.parent.clone()),
        });
        if failure.is_some() {
            break;
        }
    }
    if let Some(e) = &failure {
        control.classify_error(e);
    }
    let t = SegmentReceipt {
        run: s.run,
        binding: registration.binding(),
        segment: 0,
        parent: None,
        native: Some(registration.parent.clone()),
        evaluations: refs,
        decisions: vec![],
        guard: [0; 3],
        stop: control.observed.clone(),
        complete: false,
        resume: false,
        candidate: false,
        save_error: None,
        updates: 0,
        generations: control.generation_calls as u64,
        teachers: control.teacher_calls as u64,
        elapsed: Scalar::F64(control.start.elapsed().as_secs_f64()),
        cleanup: Scalar::F64(0.),
        draws: vec![],
    };
    let terminal = publish(output, "terminal.r3er", &Record::Segment(t))?;
    if let Some(e) = failure {
        return Err(e);
    }
    control.check("parity_result_publish")?;
    publish(
        output,
        "parity.r3er",
        &Record::Comparison(ComparisonReceipt {
            run: s.run,
            binding: registration.binding(),
            terminal,
            panels: scores,
            guard: [0; 3],
            candidate: false,
            historical: true,
        }),
    )?;
    println!(
        "MODEL_OUTPUT_PARITY={total}/32 NEW_SMALL_UPDATES=0 GENERATIONS={} TEACHERS={} POLICY=normal_greedy_v1",
        control.generation_calls, control.teacher_calls
    );
    Ok(())
}

fn timed(
    control: &mut RunControl,
    repetitions: usize,
    mut f: impl FnMut(usize) -> Result<()>,
) -> Result<(u128, u128)> {
    let mut values = Vec::new();
    for i in 0..5 + repetitions {
        control.check("storage_measurement")?;
        let start = Instant::now();
        f(i)?;
        if i >= 5 {
            values.push(start.elapsed().as_nanos());
        }
    }
    values.sort();
    Ok((
        values[values.len() / 2],
        values[(values.len() * 95).div_ceil(100) - 1],
    ))
}
fn bench(root: &Path, terminal: &str, output: &Path, control: &mut RunControl) -> Result<()> {
    let s = read_inputs(root)?;
    let chain = lineage(root, &s, &reference(root, terminal)?)?;
    let h = verified_history(root, &s, &chain)?;
    let r = h
        .evaluations
        .get(&(s.parent.step, PanelKind::Dev))
        .ok_or_else(|| bad("benchmark dev endpoint"))?;
    let (e, l) = payload(root, &s, r)?;
    let record = Record::Evaluation(e.clone());
    let binary = record.encode()?;
    // Comparison serialization includes every typed field, including teacher/timing and hashes.
    // JSON byte-array hashes are explicitly reported; original legacy schema is a separate figure.
    let json = serde_json::to_vec(&record)?;
    let original = s
        .origins
        .iter()
        .find(|o| o.role == "legacy-evaluation")
        .ok_or_else(|| bad("benchmark legacy origin missing"))?;
    let old_bytes = neural::read_bounded(Path::new(&original.original.locator), MAX_FILE)?;
    if hash(&old_bytes) != original.original.digest {
        return Err(bad("benchmark original changed"));
    }
    let old: Value = serde_json::from_slice(&old_bytes)?;
    let old_rows = old["dev_rows"]
        .as_array()
        .ok_or_else(|| bad("benchmark legacy rows"))?;
    if old_rows.len() != e.rows.len()
        || old_rows.iter().zip(&e.rows).any(|(a, b)| {
            serde_json::from_value::<Vec<u32>>(a["raw_tokens"].clone())
                .ok()
                .as_ref()
                != Some(&b.tokens)
        })
    {
        return Err(bad("benchmark content differs"));
    }
    let legacy = serde_json::to_vec(old_rows)?;
    let cases: Vec<_> = panel(&s, PanelKind::Dev)?
        .cases
        .iter()
        .map(|i| s.cases[*i as usize].clone())
        .collect();
    let mut case_bytes = Vec::new();
    for case in &cases {
        episode_encode(case, &mut case_bytes);
    }
    std::fs::create_dir(output)?;
    let count = e.rows.len();
    let tokens: usize = e.rows.iter().map(|r| r.tokens.len()).sum();
    let mut report = format!(
        "CONTENT=identical typed record fields; JSON byte-array digests; no compression\nROWS={count} TOKENS={tokens}\nBINARY_BYTES={} TYPED_JSON_BYTES={}\nWARMUP=5 REPETITIONS=10 BUILD={} BACKEND={} RAYON_THREADS={:?} VECLIB_THREADS={:?}\nTIMES=nanoseconds median/p95; durable includes write+sync+readback+directory sync\n",
        binary.len(),
        json.len(),
        if cfg!(debug_assertions) {
            "debug"
        } else {
            "release"
        },
        neural::cpu_backend(),
        std::env::var("RAYON_NUM_THREADS"),
        std::env::var("VECLIB_MAXIMUM_THREADS")
    );
    report.push_str(&format!("LEGACY_ROWS_JSON_BYTES={} ACTIVE_BINARY_CASE_BYTES={} ACTIVE_CASES_PLUS_BINARY_PANEL={}\nLEGACY_COMPARISON=schema dedup + codec; legacy rows repeat input/expected/derived text; binary reconstructs it from the included active owned cases and tokenizer\nCANONICAL_INPUTS_FILE_BYTES={} (also includes unused training/other panels, excluded from this panel comparison)\n",legacy.len(),case_bytes.len(),binary.len()+case_bytes.len(),std::fs::metadata(root.join("inputs.r3er"))?.len()));
    for format in ["binary", "typed-json", "legacy-row-json"] {
        for operation in ["encode", "decode", "hash", "verify", "durable"] {
            let (median, p95) = timed(control, 10, |i| {
                match (format, operation) {
                    ("binary", "encode") => {
                        std::hint::black_box(record.encode()?);
                    }
                    ("typed-json", "encode") => {
                        std::hint::black_box(serde_json::to_vec(&record)?);
                    }
                    ("legacy-row-json", "encode") => {
                        std::hint::black_box(serde_json::to_vec(old_rows)?);
                    }
                    ("binary", "decode") => {
                        std::hint::black_box(Record::decode(&binary)?);
                    }
                    ("typed-json", "decode") => {
                        std::hint::black_box(serde_json::from_slice::<Value>(&json)?);
                    }
                    ("legacy-row-json", "decode") => {
                        std::hint::black_box(serde_json::from_slice::<Vec<Value>>(&legacy)?);
                    }
                    ("binary", "hash") => {
                        std::hint::black_box(hash(&binary));
                    }
                    ("typed-json", "hash") => {
                        std::hint::black_box(hash(&json));
                    }
                    ("legacy-row-json", "hash") => {
                        std::hint::black_box(hash(&legacy));
                    }
                    ("binary", "verify") => {
                        let Record::Evaluation(v) = Record::decode(&binary)? else {
                            return Err(bad("bench record"));
                        };
                        std::hint::black_box(rescore(&s, &v, &l, true)?);
                    }
                    ("typed-json", "verify") => {
                        std::hint::black_box(serde_json::from_slice::<Value>(&json)?);
                        std::hint::black_box(hash(&json));
                    }
                    ("legacy-row-json", "verify") => {
                        let rows: Vec<Value> = serde_json::from_slice(&legacy)?;
                        let spec = super::PanelSpec {
                            id: "dev",
                            cases: &cases,
                            split_hash: &hex(&panel(&s, PanelKind::Dev)?.dataset),
                            policy_hash: &hex(&s.policy),
                            source: &hex(&s.source),
                            model: &hex(&s.parent.model),
                            step: s.parent.step as usize,
                        };
                        std::hint::black_box(verify_panel_and_rescore(
                            &spec, &rows, None, &l, true,
                        )?);
                    }
                    (_, "durable") => {
                        let bytes = match format {
                            "binary" => &binary,
                            "typed-json" => &json,
                            _ => &legacy,
                        };
                        let p = output.join(format!("{format}-{i}.dat"));
                        publish_bytes(&p, bytes)?;
                        let loaded = neural::read_bounded(&p, MAX_FILE)?;
                        if &loaded != bytes {
                            return Err(bad("benchmark reload"));
                        }
                    }
                    _ => unreachable!(),
                }
                Ok(())
            })?;
            report.push_str(&format!("{format} {operation}: {median}/{p95}\n"));
        }
    }
    report.push_str(&format!("RSS_KIB_AFTER={:?}\nRSS_LIMITATION=process snapshot, not allocation peak; encode includes validation allocations; JSON decode is dynamic Value, not native control; semantic verify times have different work and are not a speed ratio\n",rss_kib().ok()));
    publish_bytes(&output.join("measurement.txt"), report.as_bytes())?;
    print!("{report}");
    Ok(())
}
#[cfg(feature = "test-support")]
fn fixture(output: &Path) -> Result<()> {
    std::fs::create_dir(output)?;
    let tok = ByteBpe::train(
        &[b"abc b".to_vec()],
        &neural::hash(b"binary-regression-fixture"),
        264,
    )?;
    let model = Transformer::init(
        neural::transformer::Config::tiny(tok.vocab_size()),
        17,
        Device::Cpu,
    )?;
    let manifest =
        checkpoint::initialized(&model, &tok, 17, neural::hash(b"binary-regression-fixture"))?;
    let mut l = Loaded {
        model,
        tokenizer: tok,
        manifest,
        optimizer: BTreeMap::new(),
    };
    let mut cases = Vec::new();
    for i in 0..4 {
        let id = format!("binary-fixture/{i}");
        cases.push(Episode {
            id: id.clone(),
            category: 0,
            family: id.clone(),
            binding: id.clone(),
            sequence: id.clone(),
            request: ModelRequest {
                request_id: id,
                system: String::new(),
                input: "abc".into(),
                evidence: Default::default(),
                limits: GenerationLimits {
                    context_tokens: 64,
                    max_tokens: 8,
                    timeout_ms: 30000,
                },
            },
            answer: "b".into(),
        });
    }
    let state = TrainingState {
        contrast16: false,
        parent_checkpoint_hash: None,
        config: TrainConfig {
            lr: 0.01,
            warmup: 0,
            max_steps: 64,
            seq_len: 64,
            microbatch: 1,
            accumulation: 1,
            ..Default::default()
        },
        step: 0,
        consumed_tokens: 0,
        target_tokens: 0,
        sampler_state: 17,
        corpus_hash: neural::hash(b"binary-fixture"),
        validation_hash: neural::hash(b"binary-fixture"),
        previous_corpora: vec![l.tokenizer.train_hash.clone()],
        initial_weight_hash: l.manifest.initial_weight_hash.clone(),
        train_loss: None,
        validation_loss: None,
    };
    let adam = Adam::new(&l.model.vars)?;
    save_arm(
        &mut l,
        &state,
        &adam,
        &output.join("parent.r3m"),
        "RECOVERY_SCREENING",
    )?;
    l.manifest.training = Some(state.clone());
    l.optimizer = adam.moments;
    let run = hash(b"explicit-tiny-numeric-regression-run");
    let mut s = RunSnapshot {
        contract: CONTRACT.into(),
        run,
        policy: hash(b"tiny-two-update-regression"),
        frozen: hash(b"tiny-frozen-four-cases"),
        source: evaluator_source(),
        parent: CheckpointRef {
            file: reference(output, "parent.r3m")?,
            run,
            segment: 0,
            model: unhex(&l.model.weight_hash()?)?,
            tokenizer: unhex(&l.tokenizer.semantic_id())?,
            architecture: unhex(&l.model.config.semantic_id()?)?,
            step: 0,
            adam: Some(unhex(&optimizer_hash(&l.optimizer)?)?),
            counters: [0, 0, 17],
        },
        historical: false,
        tiny_spec: true,
        origins: vec![],
        cases,
        train: vec![0],
        panels: vec![],
        tape: vec![],
        eval_steps: vec![24],
        lr_policy: 0,
        lr_offset: 0,
        baseline: [0, 0, 0],
        anchor_floor: 178,
    };
    for (i, kind) in [
        PanelKind::Dev,
        PanelKind::Watch,
        PanelKind::Cross,
        PanelKind::Ordinary,
    ]
    .into_iter()
    .enumerate()
    {
        s.panels.push(PanelSpec {
            kind,
            dataset: case_hash(&s.cases[i]),
            cases: vec![i as u32],
        });
    }
    let framed = samples(&[s.cases[0].clone()], &l.tokenizer, 64)?;
    let b = batch(&framed, &[0], &Device::Cpu)?;
    for i in 0..24 {
        s.tape.push(Draw {
            indices: vec![0],
            sampler: 18 + i,
            input: b.tokens as u64,
            target: (framed[0].tokens.len() - framed[0].response_start) as u64,
        });
    }
    publish(output, "inputs.r3er", &Record::Inputs(Box::new(s)))?;
    println!("FIXTURE=REAL_RANDOM_INIT_TINY PREPARATION_UPDATES=0 PLANNED_BOOTSTRAP=24");
    Ok(())
}
fn publish_bytes(path: &Path, bytes: &[u8]) -> Result<()> {
    publish_new(path, |file, _| {
        file.write_all(bytes)?;
        file.seek(SeekFrom::Start(0))?;
        let mut readback = Vec::new();
        file.take(MAX_FILE as u64 + 1).read_to_end(&mut readback)?;
        if readback != bytes {
            return Err(bad("write readback"));
        }
        Ok(())
    })
}
#[cfg(feature = "test-support")]
fn fixture_fork(from: &Path, output: &Path, untrained: bool) -> Result<()> {
    let mut s = read_inputs(from)?;
    if !s.tiny_spec {
        return Err(bad("fixture fork requires explicit TINY test spec"));
    }
    let native = if untrained {
        s.parent.clone()
    } else {
        let t = segment(from, &reference(from, "segment-00/terminal.r3er")?, &s)?;
        if !t.complete {
            return Err(bad("fixture bootstrap incomplete"));
        }
        t.native.ok_or_else(|| bad("fixture bootstrap native"))?
    };
    let l = resolve_native(from, &s, &native, true)?;
    std::fs::create_dir(output)?;
    std::fs::copy(
        owned_path(from, &native.file.locator, true)?,
        output.join("parent.r3m"),
    )?;
    s.parent = native_reference(output, "parent.r3m", &s, &l, 0)?;
    s.eval_steps = vec![1, 2];
    s.tape.truncate(2);
    for (i, d) in s.tape.iter_mut().enumerate() {
        d.sampler = s.parent.counters[2] + i as u64 + 1;
    }
    s.baseline = [0; 3];
    publish(output, "inputs.r3er", &Record::Inputs(Box::new(s)))?;
    Ok(())
}
#[cfg(feature = "test-support")]
#[allow(clippy::type_complexity)] // Test-only comparison of existing typed state, not a new runtime abstraction.
fn fixture_check(roots: &[PathBuf]) -> Result<()> {
    let mut expected: Option<(Hash, Hash, [u64; 3], Vec<(PanelKind, Vec<u32>)>, [u64; 3])> = None;
    for root in roots {
        let s = read_inputs(root)?;
        let Record::Comparison(c) = read_record(root, &reference(root, "comparison.r3er")?)? else {
            return Err(bad("fixture comparison"));
        };
        let chain = lineage(root, &s, &c.terminal)?;
        let h = verified_history(root, &s, &chain)?;
        if h.decisions.len() != 2 || c.candidate || c.historical {
            return Err(bad("fixture decisions/eligibility"));
        }
        let n = chain.last().unwrap().1.native.as_ref().unwrap();
        let mut raw = Vec::new();
        for kind in [
            PanelKind::Dev,
            PanelKind::Watch,
            PanelKind::Cross,
            PanelKind::Ordinary,
        ] {
            let (e, l) = payload(root, &s, &h.evaluations[&(n.step, kind)])?;
            for r in &e.rows {
                if let Some(Scalar::F64(f)) = r.diagnostic {
                    if f.to_bits() != f64::from(0.009906131_f32).to_bits() {
                        return Err(bad("diagnostic bits changed"));
                    }
                } else {
                    return Err(bad("diagnostic missing"));
                }
            }
            rescore(&s, &e, &l, true)?;
            raw.push((
                kind,
                e.rows
                    .iter()
                    .flat_map(|r| r.tokens.iter().copied())
                    .collect(),
            ));
        }
        let current = (
            n.model,
            n.adam.ok_or_else(|| bad("Adam"))?,
            n.counters,
            raw,
            h.guard,
        );
        if expected.as_ref().is_some_and(|e| *e != current) {
            return Err(bad("continuous/split model/Adam/raw/tape/guard mismatch"));
        }
        expected = Some(current);
        println!(
            "FIXTURE_VERIFIED root={} segments={} model={} step={} decision_count={} guard={:?} diagnostic_bits={} candidate=false",
            root.display(),
            chain.len(),
            hex(&n.model),
            n.step,
            h.decisions.len(),
            h.guard,
            f64::from(0.009906131_f32).to_bits()
        );
    }
    Ok(())
}

#[cfg(test)]
mod binary_tests {
    use super::*;
    use std::fs;
    #[test]
    fn legacy_posthoc_summary_location_is_explicit_and_null_is_rejected() {
        let score = json!({"auxiliary":[2,4],"qa":[1,3]});
        let terminal = json!({"cross":score});
        let rows_only = json!({"binding":{"complete":true},"rows":[]});
        assert_eq!(
            legacy_panel_summary(&rows_only, &terminal, PanelKind::Cross, "score", true).unwrap(),
            &terminal["cross"]
        );
        assert!(
            legacy_panel_summary(&rows_only, &terminal, PanelKind::Cross, "score", false).is_err()
        );
        assert!(
            legacy_panel_summary(&rows_only, &Value::Null, PanelKind::Cross, "score", true)
                .is_err()
        );
        let null = json!({"binding":{},"score":null});
        assert!(legacy_panel_summary(&null, &terminal, PanelKind::Cross, "score", true).is_err());
    }
    #[test]
    fn legacy_owned_digest_is_from_the_consumed_bytes() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("legacy.json");
        let before = br#"{"number":0.009906130842864513}"#;
        fs::write(&file, before).unwrap();
        let (owned, reference) = read_legacy_owned(&file).unwrap();
        fs::write(&file, br#"{"number":0}"#).unwrap();
        assert_eq!(reference.digest, hash(before));
        assert_ne!(reference.digest, hash(&fs::read(&file).unwrap()));
        assert_ne!(owned["number"], 0);
    }
    #[test]
    fn scalar_literal_bits_and_bounded_records() {
        let golden = [1, 0x9a, 0x99, 0x99, 0x99, 0x99, 0x99, 0xb9, 0x3f];
        let Scalar::F64(n) = Scalar::decode(&mut Reader::new(&golden)).unwrap() else {
            panic!()
        };
        assert_eq!(n.to_bits(), 0.1_f64.to_bits());
        let values = [
            0.,
            -0.,
            f64::MIN_POSITIVE,
            f64::from_bits(1),
            f64::MAX,
            f64::from(0.009906131_f32),
        ];
        for n in values {
            let mut b = vec![];
            Scalar::F64(n).encode(&mut b);
            assert_eq!(&b[1..], &n.to_bits().to_le_bytes());
            let Scalar::F64(r) = Scalar::decode(&mut Reader::new(&b)).unwrap() else {
                panic!()
            };
            assert_eq!(n.to_bits(), r.to_bits());
        }
        for n in [0., -0., f32::MIN_POSITIVE, f32::from_bits(1), f32::MAX] {
            let mut b = vec![];
            Scalar::F32(n).encode(&mut b);
            let Scalar::F32(r) = Scalar::decode(&mut Reader::new(&b)).unwrap() else {
                panic!()
            };
            assert_eq!(n.to_bits(), r.to_bits());
        }
        let mut nan = vec![1];
        nan.extend(f64::NAN.to_bits().to_le_bytes());
        assert!(Scalar::decode(&mut Reader::new(&nan)).is_err());
        nan[0] = 3;
        let s = Scalar::decode(&mut Reader::new(&nan)).unwrap();
        assert!(s.finite().is_err());
        let mut large = vec![];
        put_varint(&mut large, u64::MAX);
        put_varint(&mut large, u32::MAX as u64);
        let mut r = Reader::new(&large);
        assert_eq!(r.var().unwrap(), u64::MAX);
        assert_eq!(u32_read(&mut r).unwrap(), u32::MAX);
        let d = EvalDecision {
            run: [1; 32],
            binding: [2; 32],
            dev: [3; 32],
            watch: [4; 32],
            step: 17,
            before: [0, 1, 2],
            after: [1, 2, 0],
            applied: true,
            quality_stop: false,
        };
        // Independent, literal ordered body (no call to the writer to build expected bytes).
        let mut body = vec![];
        for byte in 1..=4 {
            body.extend([byte; 32]);
        }
        body.extend([17, 0, 1, 2, 1, 2, 0, 1, 0]);
        let mut domain = b"R3ER-record-v1\0".to_vec();
        domain.push(3);
        domain.extend(&body);
        let mut literal = b"R3ER\x01\x00\x03\x01".to_vec();
        literal.extend((body.len() as u64).to_le_bytes());
        literal.extend(hash(&domain));
        literal.extend(body);
        let encoded = Record::Decision(d).encode().unwrap();
        assert_eq!(encoded, literal);
        for index in [0, 4, 6, 7, 8, 16, encoded.len() - 1] {
            let mut bad = encoded.clone();
            bad[index] ^= 1;
            assert!(Record::decode(&bad).is_err(), "byte {index}");
        }
        let mut trailing = encoded.clone();
        trailing.push(0);
        assert!(Record::decode(&trailing).is_err());
        assert!(Record::decode(&encoded[..encoded.len() - 1]).is_err());
    }
    #[cfg(feature = "test-support")]
    #[test]
    fn native_references_and_owned_input_fail_closed() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("native");
        fixture(&root).unwrap();
        let s = read_inputs(&root).unwrap();
        resolve_native(&root, &s, &s.parent, true).unwrap();
        for field in 0..7 {
            let mut r = s.parent.clone();
            match field {
                0 => r.model[0] ^= 1,
                1 => r.step += 1,
                2 => r.tokenizer[0] ^= 1,
                3 => r.run[0] ^= 1,
                4 => r.file.digest[0] ^= 1,
                5 => r.adam = None,
                6 => r.counters[0] += 1,
                _ => unreachable!(),
            };
            assert!(
                resolve_native(&root, &s, &r, true).is_err(),
                "field {field}"
            );
        }
        let mut r = s.parent.clone();
        r.file.locator = "missing.r3m".into();
        assert!(resolve_native(&root, &s, &r, true).is_err());
        r.file.locator = "../parent.r3m".into();
        assert!(resolve_native(&root, &s, &r, true).is_err());
        let mut bad = s.clone();
        let first = bad.panels[0].cases[0];
        bad.panels[0].cases.push(first);
        assert!(Record::Inputs(Box::new(bad)).encode().is_err());
        let mut bytes = fs::read(root.join("inputs.r3er")).unwrap();
        let last = bytes.len() - 1;
        bytes[last] ^= 1;
        fs::write(root.join("corrupt.r3er"), bytes).unwrap();
        assert!(read_record(&root, &reference(&root, "corrupt.r3er").unwrap()).is_err());
        println!("DIRECT_REFERENCE_FIXTURE_UPDATES=0 GENERATIONS=0");
    }
}
#[cfg(all(test, feature = "test-support"))]
mod binary_tests_close {
    use super::*;
    #[test]
    fn writer_reader_actual_close_rejects_missing_ambiguous_and_tampered_panels() {
        let directory = tempfile::tempdir().unwrap();
        let seed = directory.path().join("seed");
        fixture(&seed).unwrap();
        for mode in 0..10 {
            let root = directory.path().join(format!("case-{mode}"));
            std::fs::create_dir(&root).unwrap();
            std::fs::copy(seed.join("parent.r3m"), root.join("parent.r3m")).unwrap();
            let mut s = read_inputs(&seed).unwrap();
            s.tape.clear();
            s.eval_steps = vec![0];
            publish(&root, "inputs.r3er", &Record::Inputs(Box::new(s.clone()))).unwrap();
            let l = resolve_native(&root, &s, &s.parent, true).unwrap();
            let mut refs = vec![];
            let mut payloads = vec![];
            for kind in [
                PanelKind::Dev,
                PanelKind::Watch,
                PanelKind::Cross,
                PanelKind::Ordinary,
            ] {
                let ordinal = panel(&s, kind).unwrap().cases[0];
                let e = &s.cases[ordinal as usize];
                let prompt = l
                    .tokenizer
                    .prepare(
                        &e.request,
                        l.model.config.context as u32,
                        &l.model.config.id().unwrap(),
                    )
                    .unwrap();
                // Independent oracle fixture; no generator/optimizer calls and no quality claim.
                let mut tokens = l
                    .tokenizer
                    .encode(if mode == 1 { b"c" } else { e.answer.as_bytes() })
                    .unwrap();
                tokens.push(EOS);
                let row = EvalRow {
                    ordinal,
                    case: case_hash(e),
                    prompt: token_hash(&prompt.token_ids),
                    prompt_len: prompt.token_ids.len() as u32,
                    native_prompt: unhex(&prompt.token_digest).unwrap(),
                    provided: prompt.provided,
                    excluded: prompt.excluded,
                    eos: Some((tokens.len() - 1) as u32),
                    tokens,
                    started: true,
                    completed: true,
                    finish: Finish::Eos,
                    error: None,
                    error_class: None,
                    interruption: None,
                    effective_timeout: Some(30000),
                    timing: None,
                    retained: vec![],
                    teacher: TeacherRecord::NotRequested,
                    diagnostic: Some(Scalar::F64(f64::from(0.009906131_f32))),
                };
                let mut payload = EvalPayload {
                    run: s.run,
                    binding: s.binding(),
                    source: s.source,
                    model: s.parent.model,
                    tokenizer: s.parent.tokenizer,
                    architecture: s.parent.architecture,
                    step: 0,
                    new_updates: 0,
                    kind,
                    expected: 1,
                    rows: vec![row],
                };
                if mode == 2 && kind == PanelKind::Ordinary {
                    payload.rows.clear();
                }
                if mode == 3 && kind == PanelKind::Ordinary {
                    payload.rows[0].case[0] ^= 1;
                }
                let reference = publish(
                    &root,
                    &format!("{}.r3er", kind.name()),
                    &Record::Evaluation(payload.clone()),
                )
                .unwrap();
                refs.push(EvaluationRef {
                    payload: reference,
                    native: Some(s.parent.clone()),
                });
                payloads.push(payload);
            }
            let dr = Record::identity(&std::fs::read(root.join("dev.r3er")).unwrap()).unwrap();
            let wr = Record::identity(&std::fs::read(root.join("watch.r3er")).unwrap()).unwrap();
            let mut d = decision_for(&s, &payloads[0], &payloads[1], [0; 3], dr, wr, &l).unwrap();
            if mode == 5 {
                d.after[0] = 1;
            }
            let decision = publish(&root, "decision.r3er", &Record::Decision(d)).unwrap();
            if mode == 8 {
                let mut alternate = payloads[0].clone();
                alternate.rows[0].diagnostic = Some(Scalar::F64(0.1));
                let other =
                    publish(&root, "other-dev.r3er", &Record::Evaluation(alternate)).unwrap();
                refs.push(EvaluationRef {
                    payload: other,
                    native: Some(s.parent.clone()),
                });
            }
            let t = SegmentReceipt {
                run: s.run,
                binding: s.binding(),
                segment: 0,
                parent: None,
                native: Some(s.parent.clone()),
                evaluations: refs,
                decisions: if mode == 4 { vec![] } else { vec![decision] },
                guard: [0; 3],
                stop: vec![],
                complete: true,
                resume: false,
                candidate: false,
                save_error: None,
                updates: 0,
                generations: 0,
                teachers: 0,
                elapsed: Scalar::F64(-0.),
                cleanup: Scalar::F64(0.1),
                draws: vec![],
            };
            publish(&root, "terminal.r3er", &Record::Segment(t)).unwrap();
            if mode == 6 {
                std::fs::remove_file(root.join("parent.r3m")).unwrap();
            }
            if mode == 7 {
                let path = root.join("ordinary.r3er");
                let mut bytes = std::fs::read(&path).unwrap();
                let n = bytes.len() - 1;
                bytes[n] ^= 1;
                std::fs::write(path, bytes).unwrap();
            }
            let mut control = RunControl::new(
                Arc::new(AtomicBool::new(false)),
                Duration::from_secs(60),
                u64::MAX,
            )
            .unwrap();
            control.measure_rss = false;
            if mode == 9 {
                control.cancel.store(true, Ordering::Relaxed);
                control.deadline = Instant::now();
            }
            let result = close_native(&root, "terminal.r3er", &mut control);
            if mode <= 1 {
                let comparison = result.unwrap();
                assert!(!comparison.candidate);
                assert!(
                    comparison
                        .panels
                        .iter()
                        .all(|(_, p)| p.exact == u64::from(mode == 0))
                );
            } else {
                assert!(result.is_err(), "mode{mode}");
                assert!(!root.join("comparison.r3er").exists());
            }
        }
        println!("CONSTRUCTED_CLOSE_FIXTURES=10 ACTUAL_OPTIMIZER_UPDATES=0 GENERATIONS=0");
    }
}
