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
const ANCHOR_CONTRACT: &str = "R3-NATIVE-STORAGE-QUALITY-1.0";
const RESTART_CONTRACT: &str = "R3-DURABILITY-PAIR-RESTART-1.0";
const COOLDOWN_CONTRACT: &str = "R3-PREFLIGHT-ONCE-AND-COOLDOWN-1.0";

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

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
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
struct AnchorAuthorization {
    pair: Hash,
    baseline: Hash,
    expected_parent: Hash,
    anchors: u8,
    pools: [Vec<u32>; 2],
}
impl AnchorAuthorization {
    fn encode(&self, b: &mut Vec<u8>) {
        for h in [self.pair, self.baseline, self.expected_parent] {
            b.extend(h);
        }
        b.push(self.anchors);
        for pool in &self.pools {
            integers(b, pool);
        }
    }
    fn decode(r: &mut Reader<'_>) -> Result<Self> {
        Ok(Self {
            pair: digest_read(r)?,
            baseline: digest_read(r)?,
            expected_parent: digest_read(r)?,
            anchors: r.byte()?,
            pools: [integers_read(r, MAX_CASES)?, integers_read(r, MAX_CASES)?],
        })
    }
    fn name(&self) -> &'static str {
        if self.anchors == 4 { "C50" } else { "A75" }
    }
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
    authorization: Option<AnchorAuthorization>,
    purpose: RunPurpose,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
enum RunPurpose {
    Anchor,
    SaveContinuous,
    SaveSplit,
    LrContinuous,
    LrSplit,
}
impl RunPurpose {
    fn tag(self) -> u8 {
        match self {
            Self::Anchor => 0,
            Self::SaveContinuous => 1,
            Self::SaveSplit => 2,
            Self::LrContinuous => 3,
            Self::LrSplit => 4,
        }
    }
    fn read(r: &mut Reader<'_>) -> Result<Self> {
        match r.byte()? {
            0 => Ok(Self::Anchor),
            1 => Ok(Self::SaveContinuous),
            2 => Ok(Self::SaveSplit),
            3 => Ok(Self::LrContinuous),
            4 => Ok(Self::LrSplit),
            _ => Err(bad("unknown run purpose")),
        }
    }
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
        if let Some(a) = &self.authorization {
            a.encode(b);
        }
        if matches!(self.contract.as_str(), RESTART_CONTRACT | COOLDOWN_CONTRACT) {
            b.push(self.purpose.tag());
        }
    }
    fn binding(&self) -> Hash {
        let mut b = if self.authorization.is_some() {
            b"R3ER-anchor-input-v1\0".to_vec()
        } else {
            b"R3ER-input-binding-v1\0".to_vec()
        };
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
        if matches!(self.contract.as_str(), RESTART_CONTRACT | COOLDOWN_CONTRACT) {
            b.extend(self.source);
            for o in &self.origins {
                string(&mut b, &o.role);
                o.original.encode(&mut b);
            }
        }
        self.content(&mut b);
        hash(&b)
    }
    fn decode(r: &mut Reader<'_>, authorized: bool) -> Result<Self> {
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
        let authorization = if authorized {
            Some(AnchorAuthorization::decode(r)?)
        } else {
            None
        };
        let purpose = if matches!(contract.as_str(), RESTART_CONTRACT | COOLDOWN_CONTRACT) {
            RunPurpose::read(r)?
        } else {
            RunPurpose::Anchor
        };
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
            authorization,
            purpose,
        };
        s.validate()?;
        Ok(s)
    }
    fn validate(&self) -> Result<()> {
        if matches!(
            self.purpose,
            RunPurpose::SaveContinuous | RunPurpose::SaveSplit
        ) && (self.contract != RESTART_CONTRACT
            || self.tape.len() != 2
            || !self.eval_steps.is_empty())
        {
            return Err(bad("preflight purpose/update/evaluation bounds"));
        }
        if self.contract == RESTART_CONTRACT && !self.tiny_spec {
            for role in [
                "replacement-inputs",
                "replacement-terminal",
                "replacement-command",
                "execution-binary",
            ] {
                if self.origins.iter().filter(|o| o.role == role).count() != 1 {
                    return Err(bad("restart authorization provenance"));
                }
            }
        }
        if self.contract
            != if self.cooldown() {
                COOLDOWN_CONTRACT
            } else if self.contract == RESTART_CONTRACT
                && (self.authorization.is_some() || self.tiny_spec)
            {
                RESTART_CONTRACT
            } else if self.authorization.is_some() {
                ANCHOR_CONTRACT
            } else {
                CONTRACT
            }
            || self.parent.run != self.run
            || self.cases.is_empty()
            || self.lr_policy > if self.cooldown() { 3 } else { 1 }
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
        if self.cooldown()
            && (self.historical
                || !matches!(self.lr_policy, 2 | 3)
                || self.lr_offset != 0
                || !matches!(self.purpose, RunPurpose::LrContinuous | RunPurpose::LrSplit)
                || self.tape.len() != if self.tiny_spec { 2 } else { 256 }
                || self.eval_steps
                    != if self.tiny_spec {
                        vec![]
                    } else {
                        vec![128, 256]
                    })
        {
            return Err(bad("cooldown fixed horizon/policy/purpose"));
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
        if let Some(a) = &self.authorization {
            let all: BTreeSet<_> = a.pools.iter().flatten().copied().collect();
            if self.historical
                || self.tiny_spec
                || !matches!(a.anchors, 4 | 6)
                || self.parent.step != if self.cooldown() { 24310 } else { 23798 }
                || self.parent.adam.is_none()
                || self.parent.file.digest != a.expected_parent
                || !self.cooldown() && (self.lr_policy != 1 || self.lr_offset != 1024)
                || self.cooldown() && a.anchors != 6
                || self.anchor_floor != 178
                || self.train.len() != 2560
                || a.pools[0].len() != 2048
                || a.pools[1].len() != 512
                || all != (0..2560).collect()
                || self.tape.len()
                    != if self.preflight() {
                        2
                    } else if self.cooldown() {
                        256
                    } else {
                        512
                    }
                || self.eval_steps
                    != if self.preflight() {
                        vec![]
                    } else if self.cooldown() {
                        vec![128, 256]
                    } else {
                        vec![256, 512]
                    }
                || self.preflight() && (self.contract != RESTART_CONTRACT || a.anchors != 4)
                || self.tape.iter().map(|d| d.input).sum::<u64>()
                    > if self.cooldown() {
                        1_000_000
                    } else {
                        2_000_000
                    }
                || self.tape.iter().map(|d| d.target).sum::<u64>()
                    > if self.cooldown() { 250_000 } else { 500_000 }
            {
                return Err(bad("anchor experiment authorization/pools/budget"));
            }
            let anchor: BTreeSet<_> = a.pools[0].iter().copied().collect();
            if self.tape.iter().any(|d| {
                d.indices.len() != 8
                    || d.indices.iter().filter(|i| anchor.contains(i)).count()
                        != usize::from(a.anchors)
            }) {
                return Err(bad("anchor ratio differs from authorized batch"));
            }
            let ordinary: BTreeSet<_> = panel(self, PanelKind::Ordinary)?
                .cases
                .iter()
                .copied()
                .collect();
            if panel(self, PanelKind::Watch)?
                .cases
                .iter()
                .any(|i| !ordinary.contains(i))
            {
                return Err(bad("watch is not ordinary subset"));
            }
        }
        Ok(())
    }
    fn preflight(&self) -> bool {
        matches!(
            self.purpose,
            RunPurpose::SaveContinuous | RunPurpose::SaveSplit
        ) || self.cooldown() && self.tiny_spec
    }
    fn cooldown(&self) -> bool {
        self.contract == COOLDOWN_CONTRACT
    }
    fn arm_name(&self) -> &'static str {
        match self.purpose {
            RunPurpose::LrContinuous | RunPurpose::LrSplit => {
                if self.lr_policy == 2 {
                    "K-KEEP"
                } else {
                    "D-DECAY"
                }
            }
            RunPurpose::SaveContinuous => "preflight-A",
            RunPurpose::SaveSplit => "preflight-B",
            RunPurpose::Anchor => match &self.authorization {
                Some(a) if self.contract == RESTART_CONTRACT => {
                    if a.anchors == 4 {
                        "C50-R"
                    } else {
                        "A75-R"
                    }
                }
                Some(a) => a.name(),
                None => "TINY",
            },
        }
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
    // Actual Adam arguments, distinct from the inherited config LR. None is legacy.
    lr_bits: Option<Vec<u64>>,
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
        if let Some(rates) = &self.lr_bits {
            put_varint(b, rates.len() as u64);
            for bits in rates {
                b.extend(bits.to_le_bytes());
            }
        }
    }
    fn decode(r: &mut Reader<'_>, with_rates: bool) -> Result<Self> {
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
        let lr_bits = if with_rates {
            let n = count(r, 256)?;
            Some(
                (0..n)
                    .map(|_| Ok(u64::from_le_bytes(r.take(8)?.try_into().unwrap())))
                    .collect::<Result<Vec<_>>>()?,
            )
        } else {
            None
        };
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
            lr_bits,
        })
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
enum CommandStatus {
    Complete,
    TimePause,
    Failed,
}
#[derive(Clone, Debug, Serialize)]
struct CommandOutcome {
    run: Hash,
    binding: Hash,
    terminal: FileRef,
    comparison: Option<FileRef>,
    status: CommandStatus,
    stop: Vec<StopReason>,
    error: Option<String>,
    elapsed: Scalar,
}
impl CommandOutcome {
    fn encode(&self, b: &mut Vec<u8>) {
        b.extend(self.run);
        b.extend(self.binding);
        self.terminal.encode(b);
        optional(b, self.comparison.as_ref(), |b, r| r.encode(b));
        b.push(match self.status {
            CommandStatus::Complete => 0,
            CommandStatus::TimePause => 1,
            CommandStatus::Failed => 2,
        });
        put_varint(b, self.stop.len() as u64);
        for s in &self.stop {
            b.push(stop_tag(*s));
        }
        optional(b, self.error.as_ref(), |b, e| string(b, e));
        self.elapsed.encode(b);
    }
    fn decode(r: &mut Reader<'_>) -> Result<Self> {
        let run = digest_read(r)?;
        let binding = digest_read(r)?;
        let terminal = FileRef::decode(r)?;
        let comparison = r.opt(FileRef::decode)?;
        let status = match r.byte()? {
            0 => CommandStatus::Complete,
            1 => CommandStatus::TimePause,
            2 => CommandStatus::Failed,
            _ => return Err(bad("unknown command status")),
        };
        let n = count(r, 8)?;
        let stop = (0..n).map(|_| stop_read(r)).collect::<Result<Vec<_>>>()?;
        let error = r.opt(text)?;
        let elapsed = Scalar::decode(r)?;
        if stop
            .iter()
            .map(|s| stop_tag(*s))
            .collect::<BTreeSet<_>>()
            .len()
            != stop.len()
            || elapsed.finite()? < 0.
        {
            return Err(bad("command stops/elapsed"));
        }
        Ok(Self {
            run,
            binding,
            terminal,
            comparison,
            status,
            stop,
            error,
            elapsed,
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
    Command(CommandOutcome),
    Preflight(PreflightReceipt),
    VerificationStart(VerificationStart),
    VerificationRow(VerificationRow),
    VerificationFinal(VerificationFinal),
}
#[derive(Clone, Debug, Serialize)]
struct VerificationStart {
    // 0: two-step save parity; 1: registered parent parity; 2: one joint candidate confirmation.
    scope: u8,
    root: String,
    pair: Hash,
    source: Hash,
    binary: Hash,
    inputs: [FileRef; 2],
    commands: [FileRef; 2],
    natives: [FileRef; 2],
    order: Vec<(u8, u32)>,
    generation_limit: u64,
    seconds_limit: u64,
}
impl VerificationStart {
    fn encode(&self, b: &mut Vec<u8>) {
        string(b, &self.root);
        for h in [self.pair, self.source, self.binary] {
            b.extend(h);
        }
        for r in self
            .inputs
            .iter()
            .chain(&self.commands)
            .chain(&self.natives)
        {
            r.encode(b);
        }
        put_varint(b, self.order.len() as u64);
        for (arm, ordinal) in &self.order {
            b.push(*arm);
            put_varint(b, u64::from(*ordinal));
        }
        put_varint(b, self.generation_limit);
        put_varint(b, self.seconds_limit);
    }
    fn decode(r: &mut Reader<'_>, scope: u8) -> Result<Self> {
        let root = text(r)?;
        let pair = digest_read(r)?;
        let source = digest_read(r)?;
        let binary = digest_read(r)?;
        let inputs = [FileRef::decode(r)?, FileRef::decode(r)?];
        let commands = [FileRef::decode(r)?, FileRef::decode(r)?];
        let natives = [FileRef::decode(r)?, FileRef::decode(r)?];
        let n = count(
            r,
            match scope {
                0 => 4,
                1 => 16,
                2 => 1168,
                _ => return Err(bad("verification scope")),
            },
        )?;
        let order = (0..n)
            .map(|_| Ok((r.byte()?, u32_read(r)?)))
            .collect::<Result<Vec<_>>>()?;
        let generation_limit = r.var()?;
        let seconds_limit = r.var()?;
        if n == 0
            || generation_limit != n as u64
            || seconds_limit != 1800
            || order.iter().any(|(a, _)| *a > 1)
        {
            return Err(bad("verification intent bounds"));
        }
        Ok(Self {
            scope,
            root,
            pair,
            source,
            binary,
            inputs,
            commands,
            natives,
            order,
            generation_limit,
            seconds_limit,
        })
    }
}
#[derive(Clone, Debug, Serialize)]
struct VerificationRow {
    start: FileRef,
    index: u32,
    // None reserves one API entry. It is not evidence that the call actually started.
    row: Option<EvalRow>,
    elapsed: Scalar,
}
impl VerificationRow {
    fn encode(&self, b: &mut Vec<u8>) {
        self.start.encode(b);
        put_varint(b, u64::from(self.index));
        optional(b, self.row.as_ref(), |b, row| row.encode(b));
        self.elapsed.encode(b);
    }
    fn decode(r: &mut Reader<'_>) -> Result<Self> {
        let v = Self {
            start: FileRef::decode(r)?,
            index: u32_read(r)?,
            row: r.opt(EvalRow::decode)?,
            elapsed: Scalar::decode(r)?,
        };
        if v.index >= 1168 || v.elapsed.finite()? < 0. {
            return Err(bad("verification row bounds"));
        }
        Ok(v)
    }
}
#[derive(Clone, Debug, Serialize)]
struct VerificationFinal {
    start: FileRef,
    rows: Vec<FileRef>,
    proof: Option<FileRef>,
    succeeded: bool,
    entries: u64,
    completed: u64,
    interrupted: u64,
    tokens: u64,
    teachers: u64,
    elapsed: Scalar,
    unknown_tail: bool,
    stop: Vec<StopReason>,
    error: Option<String>,
    save_error: Option<String>,
}
impl VerificationFinal {
    fn encode(&self, b: &mut Vec<u8>) {
        self.start.encode(b);
        put_varint(b, self.rows.len() as u64);
        for r in &self.rows {
            r.encode(b);
        }
        optional(b, self.proof.as_ref(), |b, r| r.encode(b));
        b.push(u8::from(self.succeeded));
        for n in [
            self.entries,
            self.completed,
            self.interrupted,
            self.tokens,
            self.teachers,
        ] {
            put_varint(b, n);
        }
        self.elapsed.encode(b);
        b.push(u8::from(self.unknown_tail));
        put_varint(b, self.stop.len() as u64);
        for s in &self.stop {
            b.push(stop_tag(*s));
        }
        optional(b, self.error.as_ref(), |b, e| string(b, e));
        optional(b, self.save_error.as_ref(), |b, e| string(b, e));
    }
    fn decode(r: &mut Reader<'_>) -> Result<Self> {
        let start = FileRef::decode(r)?;
        let n = count(r, 1168)?;
        let rows = (0..n)
            .map(|_| FileRef::decode(r))
            .collect::<Result<Vec<_>>>()?;
        let proof = r.opt(FileRef::decode)?;
        let succeeded = r.bool()?;
        let entries = r.var()?;
        let completed = r.var()?;
        let interrupted = r.var()?;
        let tokens = r.var()?;
        let teachers = r.var()?;
        let elapsed = Scalar::decode(r)?;
        let unknown_tail = r.bool()?;
        let n = count(r, 8)?;
        let stop = (0..n).map(|_| stop_read(r)).collect::<Result<Vec<_>>>()?;
        let error = r.opt(text)?;
        let save_error = r.opt(text)?;
        if entries > 1168
            || completed + interrupted > entries
            || teachers != 0
            || elapsed.finite()? < 0.
            || tokens > 1168 * MAX_TOKENS as u64
            || succeeded
                && (proof.is_none()
                    || unknown_tail
                    || !stop.is_empty()
                    || error.is_some()
                    || save_error.is_some()
                    || completed != entries)
        {
            return Err(bad("verification outcome counters/status"));
        }
        Ok(Self {
            start,
            rows,
            proof,
            succeeded,
            entries,
            completed,
            interrupted,
            tokens,
            teachers,
            elapsed,
            unknown_tail,
            stop,
            error,
            save_error,
        })
    }
}
#[derive(Clone, Debug, Serialize)]
struct PreflightReceipt {
    pair: Hash,
    source: Hash,
    binary: Hash,
    commands: [FileRef; 2],
    panels: Vec<FileRef>,
    elapsed: Scalar,
    generations: u64,
}
impl PreflightReceipt {
    fn encode(&self, b: &mut Vec<u8>, extended: bool) {
        b.extend(self.pair);
        b.extend(self.source);
        b.extend(self.binary);
        for r in &self.commands {
            r.encode(b);
        }
        if extended {
            put_varint(b, self.panels.len() as u64);
        }
        for r in &self.panels {
            r.encode(b);
        }
        self.elapsed.encode(b);
        put_varint(b, self.generations);
    }
    fn decode(r: &mut Reader<'_>, extended: bool) -> Result<Self> {
        let pair = digest_read(r)?;
        let source = digest_read(r)?;
        let binary = digest_read(r)?;
        let commands = [FileRef::decode(r)?, FileRef::decode(r)?];
        let n = if extended { count(r, 3)? } else { 2 };
        let p = Self {
            pair,
            source,
            binary,
            commands,
            panels: (0..n).map(|_| FileRef::decode(r)).collect::<Result<_>>()?,
            elapsed: Scalar::decode(r)?,
            generations: r.var()?,
        };
        if p.elapsed.finite()? < 0. || p.generations > if extended { 1168 } else { 8 } || n == 0 {
            return Err(bad("preflight observation bounds"));
        }
        Ok(p)
    }
}
impl Record {
    fn encode(&self) -> Result<Vec<u8>> {
        let mut body = Vec::new();
        let kind = match self {
            Self::Inputs(v) => {
                if matches!(v.contract.as_str(), RESTART_CONTRACT | COOLDOWN_CONTRACT) {
                    body.push(u8::from(v.authorization.is_some()));
                }
                v.encode(&mut body);
                if v.cooldown() {
                    13
                } else if v.contract == RESTART_CONTRACT {
                    8
                } else if v.authorization.is_some() {
                    6
                } else {
                    1
                }
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
                if v.lr_bits.is_some() { 14 } else { 4 }
            }
            Self::Comparison(v) => {
                v.encode(&mut body);
                5
            }
            Self::Command(v) => {
                v.encode(&mut body);
                7
            }
            Self::Preflight(v) => {
                let extended = v.generations > 8 || v.panels.len() != 2;
                v.encode(&mut body, extended);
                if extended { 16 } else { 9 }
            }
            Self::VerificationStart(v) => {
                if v.scope != 0 {
                    body.push(v.scope);
                }
                v.encode(&mut body);
                if v.scope == 0 { 10 } else { 15 }
            }
            Self::VerificationRow(v) => {
                v.encode(&mut body);
                11
            }
            Self::VerificationFinal(v) => {
                v.encode(&mut body);
                12
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
            1 | 6 | 8 | 13 => {
                let authorized = if matches!(kind, 8 | 13) {
                    r.bool()?
                } else {
                    kind == 6
                };
                let s = RunSnapshot::decode(&mut r, authorized)?;
                if (kind == 8) != (s.contract == RESTART_CONTRACT) || (kind == 13) != s.cooldown() {
                    return Err(bad("purpose record kind"));
                }
                Self::Inputs(Box::new(s))
            }
            2 => Self::Evaluation(EvalPayload::decode(&mut r)?),
            3 => Self::Decision(EvalDecision::decode(&mut r)?),
            4 | 14 => Self::Segment(SegmentReceipt::decode(&mut r, kind == 14)?),
            5 => Self::Comparison(ComparisonReceipt::decode(&mut r)?),
            7 => Self::Command(CommandOutcome::decode(&mut r)?),
            9 | 16 => Self::Preflight(PreflightReceipt::decode(&mut r, kind == 16)?),
            10 | 15 => {
                let scope = if kind == 15 { r.byte()? } else { 0 };
                Self::VerificationStart(VerificationStart::decode(&mut r, scope)?)
            }
            11 => Self::VerificationRow(VerificationRow::decode(&mut r)?),
            12 => Self::VerificationFinal(VerificationFinal::decode(&mut r)?),
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
    evaluate_row_with_entry(l, e, ordinal, control, with_teacher, false, || Ok(()))
}
#[allow(clippy::too_many_arguments)] // One optional durable API entry at the existing generation boundary.
fn evaluate_row_with_entry(
    l: &Loaded,
    e: &Episode,
    ordinal: u32,
    control: &mut RunControl,
    with_teacher: bool,
    durable_entry: bool,
    before_entry: impl FnOnce() -> Result<()>,
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
    before_entry()?;
    control.check("durable_generation_entry")?;
    control.generation_calls += 1;
    if durable_entry {
        println!(
            "VERIFICATION_GENERATION_API_ENTRY={} CASE_ORDINAL={ordinal}",
            control.generation_calls
        );
    }
    let mut tokens = Vec::new();
    let cancel = control.cancel.clone();
    let result = l.model.generate_observed(
        &prompt.token_ids,
        e.request.limits.max_tokens as usize,
        timeout,
        &cancel,
        &e.id,
        |id| {
            tokens.push(id);
            #[cfg(feature = "test-support")]
            if l.model.config.profile == "TINY_NUMERIC_TEST_ONLY"
                && matches!(
                    std::env::var("R3ER_TEST_STOP").as_deref(),
                    Ok("pv-first-token" | "pv-nonfinite")
                )
            {
                cancel.store(true, Ordering::Relaxed);
            }
        },
    );
    #[cfg(feature = "test-support")]
    let result = if l.model.config.profile == "TINY_NUMERIC_TEST_ONLY"
        && std::env::var("R3ER_TEST_STOP").as_deref() == Ok("pv-nonfinite")
    {
        cancel.store(false, Ordering::Relaxed);
        Err(Error::Model(
            "nonfinite injected at generation return".into(),
        ))
    } else {
        result
    };
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
#[allow(clippy::too_many_arguments)] // Optional verified prefix for same-model time-pause continuation.
fn evaluate(
    s: &RunSnapshot,
    l: &Loaded,
    kind: PanelKind,
    step: u64,
    control: &mut RunControl,
    with_teacher: bool,
    mut rows: Vec<EvalRow>,
) -> Result<EvalPayload> {
    let spec = panel(s, kind)?;
    let mut heartbeat = Instant::now();
    for ordinal in spec.cases.iter().skip(rows.len()) {
        if control.generation_calls >= control.generation_limit {
            control.observe(StopReason::ResourceLimit);
            break;
        }
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
        if heartbeat.elapsed() >= Duration::from_secs(15) {
            println!(
                "NODE=G2 STATE=EVALUATING panel={} model_step={step} completed={}/{} generations={} elapsed_s={:.3}",
                kind.name(),
                rows.len(),
                spec.cases.len(),
                control.generation_calls,
                control.start.elapsed().as_secs_f64()
            );
            heartbeat = Instant::now();
        }
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
    /// Register the one authorized A75-R512 LR continuation pair, without learning.
    CooldownPrepare {
        #[arg(long)]
        parent_arm: PathBuf,
        #[arg(long)]
        expected_parent: String,
        #[arg(long)]
        output: PathBuf,
    },
    /// One durable parent parity attempt, or the gated single fresh confirmation.
    CooldownVerify {
        #[arg(long)]
        root: PathBuf,
        #[arg(long)]
        confirmation: bool,
    },
    /// Register the explicitly authorized F512 anchor-ratio pair; no optimizer calls.
    AnchorPrepare {
        #[arg(long)]
        baseline: PathBuf,
        #[arg(long)]
        policy: PathBuf,
        #[arg(long)]
        expected_parent: String,
        #[arg(long)]
        output: PathBuf,
        /// Explicit one-time replacement of the preserved C50 native-save failure.
        #[arg(long)]
        replacement_of: Option<PathBuf>,
    },
    /// Verify continuous/split preflight endpoints; never use them as quality parents.
    SavePreflightVerify {
        #[arg(long)]
        root: PathBuf,
        /// Explicit audit of immutable old proofs; never authorizes a new verification.
        #[arg(long)]
        legacy_read_only: bool,
    },
    /// Independently recount the registered pair endpoints; never extend its budget.
    AnchorReport {
        #[arg(long)]
        root: PathBuf,
    },
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
        #[arg(long)]
        restart_spec: bool,
    },
    #[cfg(feature = "test-support")]
    FixturePreflight {
        #[arg(long)]
        from: PathBuf,
        #[arg(long)]
        output: PathBuf,
    },
    #[cfg(feature = "test-support")]
    FixtureCooldown {
        #[arg(long)]
        from: PathBuf,
        #[arg(long)]
        output: PathBuf,
    },
    #[cfg(feature = "test-support")]
    FixturePair {
        #[arg(long)]
        from: PathBuf,
        #[arg(long)]
        output: PathBuf,
    },
    #[cfg(feature = "test-support")]
    FixtureCheck {
        #[arg(long, required = true)]
        roots: Vec<PathBuf>,
    },
}
pub(super) fn command(action: Action) -> Result<()> {
    let mut control = RunControl::command(matches!(action, Action::Run { .. }))?;
    control.deadline = control.start + Duration::from_secs(1800);
    match action {
        Action::CooldownPrepare {
            parent_arm,
            expected_parent,
            output,
        } => cooldown_prepare(&parent_arm, unhex(&expected_parent)?, &output, &mut control),
        Action::CooldownVerify { root, confirmation } => {
            cooldown_verify(&root, confirmation, &mut control)
        }
        Action::AnchorPrepare {
            baseline,
            policy,
            expected_parent,
            output,
            replacement_of,
        } => anchor_prepare(
            &baseline,
            &policy,
            &expected_parent,
            &output,
            replacement_of.as_deref(),
            &mut control,
        ),
        Action::SavePreflightVerify {
            root,
            legacy_read_only,
        } => {
            if legacy_read_only {
                let status = read_preflight_outcome(&root)?
                    .ok_or_else(|| bad("missing legacy preflight"))?;
                if !status.legacy {
                    return Err(bad("explicit legacy mode requires legacy evidence"));
                }
                preflight_body(&root, &mut control, false, true, &mut Vec::new())
            } else {
                verify_save_preflight(&root, &mut control, true)
            }
        }
        Action::AnchorReport { root } => anchor_report(&root, &mut control),
        Action::Run { root, resume } => {
            let outcome = run_native(&root, resume.as_deref(), &mut control);
            // Account for verification/close as well as training in the cumulative command budget.
            if let Ok(s) = read_inputs(&root)
                && !s.historical
            {
                let mut segments = std::fs::read_dir(&root)?
                    .filter_map(|v| v.ok())
                    .filter(|e| e.file_name().to_string_lossy().starts_with("segment-"))
                    .map(|e| e.path())
                    .collect::<Vec<_>>();
                segments.sort();
                if let Some(dir) = segments.last()
                    && dir.join("terminal.r3er").is_file()
                    && !dir.join("command.r3er").exists()
                {
                    let prefix = dir.file_name().unwrap().to_string_lossy();
                    finalize_command(
                        &root,
                        &s,
                        &format!("{prefix}/terminal.r3er"),
                        &outcome,
                        &mut control,
                    )?;
                }
            }
            outcome
        }
        Action::Close { root, terminal } => {
            let s = read_inputs(&root)?;
            if !s.historical {
                effective_outcome(&root, &s, &reference(&root, &terminal)?)?;
            }
            close_native_inner(&root, &terminal, &mut control, false).map(|_| ())
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
            restart_spec,
        } => fixture_fork_inner(
            &from,
            &output,
            untrained,
            false,
            RunPurpose::Anchor,
            restart_spec,
        ),
        #[cfg(feature = "test-support")]
        Action::FixtureCooldown { from, output } => {
            std::fs::create_dir(&output)?;
            for (name, purpose) in [
                ("continuous", RunPurpose::LrContinuous),
                ("split", RunPurpose::LrSplit),
            ] {
                let dir = output.join(name);
                fixture_fork_inner(&from, &dir, false, false, RunPurpose::SaveContinuous, true)?;
                let mut s = read_inputs(&dir)?;
                s.contract = COOLDOWN_CONTRACT.into();
                s.purpose = purpose;
                s.lr_policy = 3;
                s.lr_offset = 0;
                std::fs::write(
                    dir.join("inputs.r3er"),
                    Record::Inputs(Box::new(s)).encode()?,
                )?;
            }
            Ok(())
        }
        #[cfg(feature = "test-support")]
        Action::FixturePreflight { from, output } => {
            std::fs::create_dir(&output)?;
            for (arm, purpose) in [
                ("preflight-A", RunPurpose::SaveContinuous),
                ("preflight-B", RunPurpose::SaveSplit),
            ] {
                fixture_fork_inner(&from, &output.join(arm), false, false, purpose, true)?;
            }
            for arm in ["C50", "A75"] {
                let dir = output.join(arm);
                fixture_fork_inner(&from, &dir, false, true, RunPurpose::Anchor, false)?;
                // The fixture consumers use their enclosing verification attempt, including after a copy.
                let mut s = read_inputs(&dir)?;
                s.origins.push(Origin {
                    role: "test-verification".into(),
                    original: FileRef {
                        locator: "preflight-start.r3er".into(),
                        digest: [0; 32],
                    },
                });
                // This is a newly owned fixture; the production publisher never replaces records.
                std::fs::write(
                    dir.join("inputs.r3er"),
                    Record::Inputs(Box::new(s)).encode()?,
                )?;
            }
            Ok(())
        }
        #[cfg(feature = "test-support")]
        Action::FixturePair { from, output } => {
            std::fs::create_dir(&output)?;
            for arm in ["C50", "A75"] {
                fixture_fork_inner(
                    &from,
                    &output.join(arm),
                    false,
                    true,
                    RunPurpose::Anchor,
                    false,
                )?;
            }
            Ok(())
        }
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
    if s.cooldown() {
        let rates = t
            .lr_bits
            .as_ref()
            .ok_or_else(|| bad("missing actual LR trace"))?;
        let first = t
            .updates
            .checked_sub(t.draws.len() as u64)
            .ok_or_else(|| bad("LR cursor"))?;
        if rates.len() != t.draws.len()
            || rates.iter().enumerate().any(|(i, bits)| {
                cooldown_lr(s.lr_policy, first + i as u64 + 1)
                    .map(f64::to_bits)
                    .ok()
                    != Some(*bits)
            })
        {
            return Err(bad("actual LR bits/horizon/cursor mismatch"));
        }
    } else if t.lr_bits.is_some() {
        return Err(bad("LR trace outside registered policy"));
    }
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
    let l = resolve_native(root, s, n, false)?;
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
                let (previous, _) = payload(root, s, old)?;
                let prefix = previous
                    .rows
                    .iter()
                    .take_while(|r| r.completed && r.interruption.is_none())
                    .collect::<Vec<_>>();
                if s.contract != RESTART_CONTRACT
                    || prefix.len() == previous.expected as usize
                    || previous.model != e.model
                    || e.rows.len() < prefix.len()
                    || prefix.iter().zip(&e.rows).any(|(a, b)| {
                        let mut x = Vec::new();
                        let mut y = Vec::new();
                        a.encode(&mut x);
                        b.encode(&mut y);
                        x != y
                    })
                {
                    return Err(bad("ambiguous same-step raw observation"));
                }
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
    if s.preflight() {
        return false;
    }
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
    let result = close_native_inner(root, terminal, control, true);
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
            #[cfg(feature = "test-support")]
            if s.tiny_spec
                && std::env::var("R3ER_TEST_STOP")
                    .unwrap_or_default()
                    .contains("stop-write-fail")
            {
                return result;
            }
            let _ = publish(root, "close-stop.r3er", &Record::Segment(t));
        }
    }
    result
}

fn command_locator(terminal: &FileRef) -> Result<String> {
    let parent = Path::new(&terminal.locator)
        .parent()
        .ok_or_else(|| bad("terminal path"))?;
    Ok(parent.join("command.r3er").to_string_lossy().into_owned())
}

// Closed-command readers share this policy. A terminal or provisional comparison alone
// cannot authorize another command; the current writer calls finalize_command instead.
fn effective_outcome(root: &Path, s: &RunSnapshot, terminal: &FileRef) -> Result<CommandOutcome> {
    let t = segment(root, terminal, s)?;
    let locator = command_locator(terminal)?;
    let Record::Command(c) = read_record(root, &reference(root, &locator)?)? else {
        return Err(bad(
            "missing typed command finalization (legacy is not upgraded)",
        ));
    };
    if c.run != s.run || c.binding != s.binding() || c.terminal != *terminal {
        return Err(bad("command terminal/run/policy binding"));
    }
    if root.join("close-stop.r3er").exists() {
        return Err(bad("persisted close failure blocks command/pair"));
    }
    if t.save_error.is_some() || c.error.is_some() || c.status == CommandStatus::Failed {
        return Err(bad(&format!(
            "failed command: stops={:?} error={:?} save={:?}",
            c.stop, c.error, t.save_error
        )));
    }
    if t.stop.iter().any(|r| !c.stop.contains(r)) {
        return Err(bad("command dropped terminal stop"));
    }
    match c.status {
        CommandStatus::Complete => {
            if !t.complete || t.resume || !t.stop.is_empty() || !c.stop.is_empty() {
                return Err(bad("normal command contains stopped/incomplete terminal"));
            }
            let reference = c
                .comparison
                .as_ref()
                .ok_or_else(|| bad("missing positive close certificate"))?;
            let Record::Comparison(p) = read_record(root, reference)? else {
                return Err(bad("command comparison kind"));
            };
            if reference.locator != "comparison.r3er"
                || p.run != s.run
                || p.binding != s.binding()
                || p.terminal != *terminal
                || p.candidate != t.candidate
                || p.historical != s.historical
            {
                return Err(bad("command comparison binding"));
            }
        }
        CommandStatus::TimePause => {
            if t.complete
                || !t.resume
                || t.stop != [StopReason::TimeBudget]
                || c.stop != [StopReason::TimeBudget]
                || c.comparison.is_some()
            {
                return Err(bad("command is not a clean resumable time pause"));
            }
        }
        CommandStatus::Failed => unreachable!(),
    }
    Ok(c)
}

fn finalize_command(
    root: &Path,
    s: &RunSnapshot,
    terminal: &str,
    result: &Result<()>,
    control: &mut RunControl,
) -> Result<()> {
    let terminal = reference(root, terminal)?;
    let t = segment(root, &terminal, s)?;
    if let Err(e) = result {
        control.classify_error(e);
    }
    // Finite cooperative decision point; signals after this seal are not retroactive.
    let _ = control.seal_terminal();
    let mut stop = t.stop.clone();
    for r in &control.observed {
        if !stop.contains(r) {
            stop.push(*r);
        }
    }
    let comparison = if root.join("comparison.r3er").is_file() {
        Some(reference(root, "comparison.r3er")?)
    } else {
        None
    };
    let error = result
        .as_ref()
        .err()
        .map(ToString::to_string)
        .or(t.save_error.clone());
    let status = if error.is_none() && t.complete && stop.is_empty() && comparison.is_some() {
        CommandStatus::Complete
    } else if error.is_none()
        && t.resume
        && !t.complete
        && stop == [StopReason::TimeBudget]
        && comparison.is_none()
    {
        CommandStatus::TimePause
    } else {
        CommandStatus::Failed
    };
    let c = CommandOutcome {
        run: s.run,
        binding: s.binding(),
        terminal: terminal.clone(),
        comparison,
        status,
        stop,
        error,
        elapsed: Scalar::F64(control.start.elapsed().as_secs_f64()),
    };
    #[cfg(feature = "test-support")]
    if s.tiny_spec
        && std::env::var("R3ER_TEST_STOP")
            .unwrap_or_default()
            .contains("command-write-fail")
    {
        return Err(std::io::Error::other("injected command outcome publication failure").into());
    }
    publish(
        root,
        &command_locator(&terminal)?,
        &Record::Command(c.clone()),
    )?;
    println!(
        "COMMAND_FINALIZATION={:?} terminal={} STOP={:?} error={:?}",
        c.status, terminal.locator, c.stop, c.error
    );
    if status == CommandStatus::Failed {
        return Err(bad("command finalization failed"));
    }
    effective_outcome(root, s, &terminal)?;
    Ok(())
}

fn close_native_inner(
    root: &Path,
    terminal: &str,
    control: &mut RunControl,
    publish_result: bool,
) -> Result<ComparisonReceipt> {
    let s = read_inputs(root)?;
    #[cfg(feature = "test-support")]
    if s.tiny_spec
        && std::env::var("R3ER_TEST_STOP")
            .unwrap_or_default()
            .starts_with("cancel-close")
    {
        control.cancel.store(true, Ordering::Relaxed);
    }
    control.check("binary_close_start")?;
    let last = reference(root, terminal)?;
    if root.join("close-stop.r3er").exists() {
        return Err(bad("persisted close failure blocks validation"));
    }
    if !publish_result && !s.historical {
        let c = effective_outcome(root, &s, &last)?;
        if c.status != CommandStatus::Complete {
            return Err(bad("close requires completed command"));
        }
    }
    let chain = lineage(root, &s, &last)?;
    for (i, (_, stored)) in chain.iter().enumerate() {
        let continued_time_stop = i + 1 < chain.len()
            && stored.resume
            && !stored.complete
            && stored.stop == [StopReason::TimeBudget];
        if !stored.stop.is_empty() && !continued_time_stop {
            return Err(bad("stored terminal stop forbids normal close"));
        }
        if !s.historical
            && i + 1 < chain.len()
            && effective_outcome(root, &s, &chain[i].0)?.status != CommandStatus::TimePause
        {
            return Err(bad("ancestor command is not a verified time pause"));
        }
    }
    let h = verified_history(root, &s, &chain)?;
    let t = &chain.last().ok_or_else(|| bad("empty lineage"))?.1;
    if !t.complete || t.resume || t.save_error.is_some() {
        return Err(bad("incomplete final terminal"));
    }
    let final_native = t.native.as_ref().unwrap();
    let step = final_native.step;
    if !s.historical && !s.preflight() && !h.decisions.contains_key(&step) {
        return Err(bad("final guard decision pending"));
    }
    let mut scores = Vec::new();
    let kinds: &[PanelKind] = if s.preflight() {
        &[]
    } else {
        &[
            PanelKind::Dev,
            PanelKind::Watch,
            PanelKind::Cross,
            PanelKind::Ordinary,
        ]
    };
    if s.preflight()
        && (t.updates != 2 || !h.evaluations.is_empty() || !h.decisions.is_empty() || t.candidate)
    {
        return Err(bad("preflight is not an exact two-update save observation"));
    }
    for &kind in kinds {
        control.check("binary_close_panel")?;
        let r = h
            .evaluations
            .get(&(step, kind))
            .ok_or_else(|| bad("missing final panel"))?;
        let (e, l) = payload(root, &s, r)?;
        #[cfg(feature = "test-support")]
        if s.tiny_spec
            && std::env::var("R3ER_TEST_STOP")
                .unwrap_or_default()
                .starts_with("error-close")
        {
            return Err(bad("injected close validation failure"));
        }
        if e.model != final_native.model
            || e.tokenizer != final_native.tokenizer
            || e.architecture != final_native.architecture
            || e.step != final_native.step
        {
            return Err(bad("final panel model/tokenizer/equation/step mismatch"));
        }
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
    if publish_result {
        publish(root, "comparison.r3er", &Record::Comparison(result.clone()))?;
    } else if !s.historical {
        let c = effective_outcome(root, &s, &result.terminal)?;
        let stored = read_record(
            root,
            c.comparison
                .as_ref()
                .ok_or_else(|| bad("close certificate missing"))?,
        )?;
        if stored.encode()? != Record::Comparison(result.clone()).encode()? {
            return Err(bad("comparison disagrees with independent raw recount"));
        }
    }
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
fn reuse_native(
    root: &Path,
    s: &RunSnapshot,
    native: &CheckpointRef,
    l: &Loaded,
    state: &TrainingState,
    adam: &Adam,
    index: u32,
) -> Result<CheckpointRef> {
    let durable = resolve_native(root, s, native, true)?;
    if durable.manifest.training.as_ref() != Some(state)
        || durable.model.weight_hash()? != l.model.weight_hash()?
        || optimizer_hash(&durable.optimizer)? != optimizer_hash(&adam.moments)?
    {
        return Err(bad("same-step checkpoint differs from current state"));
    }
    let mut reference = native.clone();
    reference.segment = index;
    Ok(reference)
}

// Fork budget metadata is local to the new run; parent bytes and Adam clock stay intact.
fn fork_budget(
    state: &mut TrainingState,
    parent_step: u64,
    parent_tokens: u64,
    updates: usize,
    tokens: u64,
) -> Result<()> {
    state.config.budget_start_step = usize::try_from(parent_step).map_err(|_| bad("fork step"))?;
    state.config.budget_start_tokens = parent_tokens;
    state.config.max_steps = state
        .config
        .budget_start_step
        .checked_add(updates)
        .ok_or_else(|| bad("fork step overflow"))?;
    state.config.max_tokens = parent_tokens
        .checked_add(tokens)
        .ok_or_else(|| bad("fork tokens overflow"))?;
    Ok(())
}

fn anchor_prepare(
    baseline: &Path,
    policy: &Path,
    expected_parent: &str,
    output: &Path,
    replacement_of: Option<&Path>,
    control: &mut RunControl,
) -> Result<()> {
    control.check("anchor_registration")?;
    let base = read_inputs(baseline)?;
    let observed = close_native_inner(baseline, "terminal.r3er", control, false)?;
    let expected_parent = unhex(expected_parent)?;
    let l = resolve_native(baseline, &base, &base.parent, true)?;
    let state = l
        .manifest
        .training
        .as_ref()
        .ok_or_else(|| bad("anchor parent Adam"))?;
    if !base.historical
        || base.tiny_spec
        || base.parent.file.digest != expected_parent
        || state.step != 23798
        || l.tokenizer.vocab_size() != 801
        || l.model.config.profile != "NATIVE_TRPP_G1_SMALL"
    {
        return Err(bad("authorized F512 parent/profile/clock"));
    }
    let (p, policy_ref) = read_legacy_owned(policy)?;
    if p["node"] != "A2"
        || p["arm"] != "F"
        || p["lr_policy"] != "L"
        || p["lr_offset"] != 512
        || base.policy != policy_ref.digest
        || progress_lr("L", 1024)? != 1e-4
    {
        return Err(bad("F512 policy/constant LR lineage"));
    }
    let verified = load_verified_inputs(&progress_path(&p, "a0")?, Some(&p))?;
    if base.frozen
        != unhex(
            verified.hashes["frozen"]
                .as_str()
                .ok_or_else(|| bad("frozen hash"))?,
        )?
        || base.train.len() != verified.train.len()
        || base
            .train
            .iter()
            .zip(&verified.train)
            .any(|(i, e)| case_hash(&base.cases[*i as usize]) != case_hash(e))
    {
        return Err(bad("anchor owned corpus differs from frozen F corpus"));
    }
    let (original_manifest, original, _) = data::load(&verified.frozen.parent_corpus)?;
    if original_manifest.train.sha256 != verified.frozen.parent_train_hash {
        return Err(bad("anchor original training provenance"));
    }
    let original: BTreeMap<_, _> = original
        .iter()
        .map(|e| (e.id.as_str(), case_hash(e)))
        .collect();
    let mut pools = [Vec::new(), Vec::new()];
    for (i, e) in verified.train.iter().enumerate() {
        match original.get(e.id.as_str()) {
            Some(h) if *h == case_hash(e) && !e.family.starts_with("copy/") => {
                pools[0].push(i as u32)
            }
            None if e.family.starts_with("renewal/H3/train/") => pools[1].push(i as u32),
            _ => return Err(bad("unproven anchor/focus source or changed original case")),
        }
    }
    if pools[0].len() != 2048 || pools[1].len() != 512 {
        return Err(bad("anchor pool counts"));
    }
    data::check_split(&verified.train, &verified.dev)?;
    data::check_split(&verified.train, &verified.ordinary)?;
    data::check_split(&verified.train, &verified.cross)?;
    let heldout: BTreeSet<_> = verified
        .dev
        .iter()
        .chain(&verified.ordinary)
        .chain(&verified.cross)
        .map(case_hash)
        .collect();
    if verified
        .train
        .iter()
        .any(|e| heldout.contains(&case_hash(e)))
    {
        return Err(bad("training/evaluation overlap"));
    }
    let framed = samples(&verified.train, &l.tokenizer, state.config.seq_len)?;
    let baseline_ref = reference(baseline, "comparison.r3er")?;
    let mut material = b"R3-anchor-ratio-pair-v1\0".to_vec();
    material.extend(base.binding());
    material.extend(expected_parent);
    material.extend(baseline_ref.digest);
    let mut replacements = Vec::new();
    if let Some(previous) = replacement_of {
        let old_root = previous.join("C50");
        let old = read_inputs(&old_root)?;
        let t = segment(
            &old_root,
            &reference(&old_root, "segment-00/terminal.r3er")?,
            &old,
        )?;
        if old.contract != ANCHOR_CONTRACT
            || old.parent.file.digest != expected_parent
            || old.parent.model != base.parent.model
            || old.parent.adam != base.parent.adam
            || t.updates != 256
            || t.native.is_some()
            || t.stop != [StopReason::IntegrityFail]
            || t.save_error.as_deref() != Some("corruption: checkpoint training state")
            || t.resume
            || previous.join("A75/segment-00").exists()
        {
            return Err(bad(
                "replacement requires the specific preserved C50 save failure",
            ));
        }
        for (role, name) in [
            ("replacement-inputs", "inputs.r3er"),
            ("replacement-terminal", "segment-00/terminal.r3er"),
            ("replacement-command", "segment-00/command.r3er"),
        ] {
            let r = reference(&old_root, name)?;
            replacements.push(Origin {
                role: role.into(),
                original: FileRef {
                    locator: old_root.join(name).canonicalize()?.display().to_string(),
                    digest: r.digest,
                },
            });
        }
        string(&mut material, RESTART_CONTRACT);
        material.extend(evaluator_source());
        material.extend(unhex(&file_hash(&std::env::current_exe()?)?)?);
        for o in &replacements {
            material.extend(o.original.digest);
        }
    }
    for pool in &pools {
        integers(&mut material, pool);
    }
    let pair = hash(&material);
    let stats = |kind| {
        observed
            .panels
            .iter()
            .find(|(k, _)| *k == kind)
            .map(|(_, v)| v)
            .ok_or_else(|| bad("baseline panel"))
    };
    let dev = stats(PanelKind::Dev)?;
    let watch = stats(PanelKind::Watch)?;
    let ordinary_set: BTreeSet<_> = panel(&base, PanelKind::Ordinary)?
        .cases
        .iter()
        .copied()
        .collect();
    if panel(&base, PanelKind::Watch)?
        .cases
        .iter()
        .any(|i| !ordinary_set.contains(i))
    {
        return Err(bad("watch not identical ordinary cases"));
    }
    let mut registrations = Vec::new();
    for anchors in [4u8, 6] {
        let mut s = base.clone();
        let authorization = AnchorAuthorization {
            pair,
            baseline: baseline_ref.digest,
            expected_parent,
            anchors,
            pools: pools.clone(),
        };
        s.contract = if replacement_of.is_some() {
            RESTART_CONTRACT
        } else {
            ANCHOR_CONTRACT
        }
        .into();
        s.origins.extend(replacements.clone());
        s.historical = false;
        s.source = evaluator_source();
        let mut identity = material.clone();
        identity.push(anchors);
        s.run = hash(&identity);
        s.parent.run = s.run;
        s.parent.segment = 0;
        s.policy = hash(&identity);
        s.lr_policy = 1;
        s.lr_offset = 1024;
        s.baseline = [dev.exact, watch.exact, dev.errors + watch.errors];
        s.anchor_floor = 178;
        s.eval_steps = vec![256, 512];
        s.origins.push(Origin {
            role: "verified-parent-baseline-read-only".into(),
            original: FileRef {
                locator: baseline
                    .canonicalize()?
                    .join("comparison.r3er")
                    .display()
                    .to_string(),
                digest: baseline_ref.digest,
            },
        });
        s.origins.push(Origin {
            role: "anchor-original-train".into(),
            original: FileRef {
                locator: verified
                    .frozen
                    .parent_corpus
                    .join(&original_manifest.train.file)
                    .canonicalize()?
                    .display()
                    .to_string(),
                digest: unhex(&original_manifest.train.sha256)?,
            },
        });
        s.origins.push(Origin {
            role: "execution-binary".into(),
            original: FileRef {
                locator: std::env::current_exe()?.display().to_string(),
                digest: unhex(&file_hash(&std::env::current_exe()?)?)?,
            },
        });
        let (tape, _) = anchor_tape(&pools, anchors, state.sampler_state, &framed)?;
        s.tape = tape;
        s.authorization = Some(authorization);
        if let Some(previous) = replacement_of {
            let old = read_inputs(&previous.join(if anchors == 4 { "C50" } else { "A75" }))?;
            if old.train != s.train
                || old.cases.iter().map(case_hash).collect::<Vec<_>>()
                    != s.cases.iter().map(case_hash).collect::<Vec<_>>()
                || old.tape.len() != s.tape.len()
                || old.tape.iter().zip(&s.tape).any(|(x, y)| {
                    x.indices != y.indices
                        || x.sampler != y.sampler
                        || x.input != y.input
                        || x.target != y.target
                })
                || old
                    .panels
                    .iter()
                    .zip(&s.panels)
                    .any(|(x, y)| x.kind != y.kind || x.dataset != y.dataset || x.cases != y.cases)
            {
                return Err(bad("replacement changed original frozen cases/panels/tape"));
            }
        }
        s.validate()?;
        // Validate the furthest registered save before publishing authorization or doing work.
        let mut end = state.clone();
        fork_budget(
            &mut end,
            s.parent.step,
            s.parent.counters[0],
            s.tape.len(),
            2_000_000,
        )?;
        end.step += s.tape.len();
        end.consumed_tokens += s.tape.iter().map(|d| d.input).sum::<u64>();
        end.target_tokens += s.tape.iter().map(|d| d.target).sum::<u64>();
        let mut manifest = l.manifest.clone();
        manifest.training = Some(end);
        checkpoint::validate_metadata(&manifest, &l.tokenizer)?;
        registrations.push(s);
    }
    if replacement_of.is_some() {
        for purpose in [RunPurpose::SaveContinuous, RunPurpose::SaveSplit] {
            let mut s = registrations[0].clone();
            s.purpose = purpose;
            s.tape.truncate(2);
            s.eval_steps.clear();
            let mut identity = material.clone();
            identity.extend(b"SAVE_RESUME_PREFLIGHT");
            identity.push(purpose.tag());
            s.run = hash(&identity);
            s.policy = s.run;
            s.parent.run = s.run;
            s.validate()?;
            registrations.push(s);
        }
    }
    control.check("anchor_before_publication")?;
    std::fs::create_dir(output)?;
    for s in registrations {
        let root = output.join(s.arm_name());
        std::fs::create_dir(&root)?;
        publish_new(&root.join("parent.r3m"), |file, _| {
            std::io::copy(
                &mut std::fs::File::open(owned_path(baseline, &base.parent.file.locator, true)?)?,
                file,
            )?;
            Ok(())
        })?;
        resolve_native(&root, &s, &s.parent, true)?;
        publish(&root, "inputs.r3er", &Record::Inputs(Box::new(s.clone())))?;
        println!(
            "ATTEMPT={} CONTRACT={} PURPOSE={:?} PLANNED_UPDATES={} PLANNED_END_STEP={} guard_parent={:?} guard=consecutive(dev_drop26/watch_drop4/error_increase6)_or_single_error20pct",
            hex(&pair),
            s.contract,
            s.purpose,
            s.tape.len(),
            s.parent.step + s.tape.len() as u64,
            s.baseline
        );
        println!(
            "NODE=D2 ARM={} STATE=REGISTERED parent={} model={} Adam={} step={} LR=0.0001 input={} target={} anchor_draws={} focus_draws={} baseline={}/{}/{} source={} binary={} optimizer_calls=0",
            s.arm_name(),
            hex(&expected_parent),
            hex(&s.parent.model),
            hex(&s.parent.adam.unwrap()),
            s.parent.step,
            s.tape.iter().map(|d| d.input).sum::<u64>(),
            s.tape.iter().map(|d| d.target).sum::<u64>(),
            usize::from(anchors_for(&s)) * s.tape.len(),
            (8 - usize::from(anchors_for(&s))) * s.tape.len(),
            dev.exact,
            stats(PanelKind::Cross)?.exact,
            stats(PanelKind::Ordinary)?.qa[0],
            hex(&s.source),
            file_hash(&std::env::current_exe()?)?
        );
    }
    Ok(())
}
fn anchors_for(s: &RunSnapshot) -> u8 {
    s.authorization.as_ref().map_or(0, |a| a.anchors)
}
fn anchor_tape(
    pools: &[Vec<u32>; 2],
    anchors: u8,
    seed: u64,
    framed: &[Sample],
) -> Result<(Vec<Draw>, [u64; 2])> {
    anchor_tape_through(pools, anchors, seed, framed, 512)
}
fn anchor_tape_through(
    pools: &[Vec<u32>; 2],
    anchors: u8,
    seed: u64,
    framed: &[Sample],
    count: u64,
) -> Result<(Vec<Draw>, [u64; 2])> {
    let mut streams = pools.clone();
    let mut rngs = [
        Rng {
            state: seed ^ 0xa11ce,
        },
        Rng {
            state: seed ^ 0xf0c05,
        },
    ];
    let mut cursors = [streams[0].len(), streams[1].len()];
    let mut tape = Vec::new();
    let mut pool_targets = [0; 2];
    for step in 0..count {
        let mut indices = Vec::new();
        for (pool, count) in [usize::from(anchors), 8 - usize::from(anchors)]
            .into_iter()
            .enumerate()
        {
            for _ in 0..count {
                if cursors[pool] == streams[pool].len() {
                    // Restart from the same pool before each deterministic cycle shuffle.
                    streams[pool].clone_from(&pools[pool]);
                    for i in (1..streams[pool].len()).rev() {
                        let j = (rngs[pool].next_u64() % (i + 1) as u64) as usize;
                        streams[pool].swap(i, j);
                    }
                    cursors[pool] = 0;
                }
                let i = streams[pool][cursors[pool]];
                cursors[pool] += 1;
                pool_targets[pool] +=
                    (framed[i as usize].tokens.len() - framed[i as usize].response_start) as u64;
                indices.push(i);
            }
        }
        tape.push(Draw {
            input: indices
                .iter()
                .map(|i| (framed[*i as usize].tokens.len() - 1) as u64)
                .sum(),
            target: indices
                .iter()
                .map(|i| {
                    (framed[*i as usize].tokens.len() - framed[*i as usize].response_start) as u64
                })
                .sum(),
            indices,
            sampler: seed
                .checked_add(step + 1)
                .ok_or_else(|| bad("sampler clock overflow"))?,
        });
    }
    Ok((tape, pool_targets))
}

fn cooldown_lr(policy: u8, n: u64) -> Result<f64> {
    if !(1..=256).contains(&n) {
        return Err(bad("cooldown schedule cursor outside fixed horizon256"));
    }
    match policy {
        2 => Ok(1e-4),
        3 => Ok(1e-5
            + 0.5 * (1e-4 - 1e-5) * (1. + (std::f64::consts::PI * (n - 1) as f64 / 255.).cos())),
        _ => Err(bad("unknown cooldown LR policy")),
    }
}

fn cooldown_prepare(
    parent: &Path,
    expected: Hash,
    output: &Path,
    control: &mut RunControl,
) -> Result<()> {
    let old = read_inputs(parent)?;
    let auth = old
        .authorization
        .as_ref()
        .ok_or_else(|| bad("cooldown requires verified A75-R pools"))?;
    if old.contract != RESTART_CONTRACT || old.arm_name() != "A75-R" || auth.anchors != 6 {
        return Err(bad("cooldown parent policy"));
    }
    let closed = arm_commands(parent, &old)?;
    let (_, command) = closed.last().ok_or_else(|| bad("parent not executed"))?;
    let observed = close_native_inner(parent, &command.terminal.locator, control, false)?;
    let chain = lineage(parent, &old, &command.terminal)?;
    let history = verified_history(parent, &old, &chain)?;
    let terminal = &chain.last().unwrap().1;
    let native = terminal
        .native
        .as_ref()
        .ok_or_else(|| bad("parent missing native"))?;
    if native.step != 24310 || native.file.digest != expected || terminal.updates != 512 {
        return Err(bad("cooldown actual parent file/step"));
    }
    // Explicit legacy observation: do not rewrite or promote the original proof.
    preflight_body(
        parent.parent().ok_or_else(|| bad("legacy pair root"))?,
        control,
        false,
        true,
        &mut Vec::new(),
    )?;
    let l = resolve_native(parent, &old, native, true)?;
    let state = l
        .manifest
        .training
        .as_ref()
        .ok_or_else(|| bad("parent full Adam state"))?;
    let framed = samples(
        &old.train
            .iter()
            .map(|i| old.cases[*i as usize].clone())
            .collect::<Vec<_>>(),
        &l.tokenizer,
        state.config.seq_len,
    )?;
    let (continued, _) = anchor_tape_through(&auth.pools, 6, old.parent.counters[2], &framed, 768)?;
    if continued[..512].iter().zip(&old.tape).any(|(a, b)| {
        a.indices != b.indices
            || a.sampler != b.sampler
            || a.input != b.input
            || a.target != b.target
    }) || native.counters[2] != continued[511].sampler
    {
        return Err(bad("A75 stream cursor cannot be reconstructed"));
    }
    let tape = continued[512..].to_vec();
    let score = |kind| {
        observed
            .panels
            .iter()
            .find(|(k, _)| *k == kind)
            .map(|(_, s)| s)
            .ok_or_else(|| bad("parent score absent"))
    };
    let dev = score(PanelKind::Dev)?;
    let watch = score(PanelKind::Watch)?;
    let mut material = Vec::new();
    string(&mut material, COOLDOWN_CONTRACT);
    material.extend(expected);
    material.extend(old.binding());
    material.extend(evaluator_source());
    let binary = unhex(&file_hash(&std::env::current_exe()?)?)?;
    material.extend(binary);
    // One registered path prevents copying a study to restart a closed authorization.
    let registered = output
        .parent()
        .ok_or_else(|| bad("study parent"))?
        .canonicalize()?
        .join(output.file_name().ok_or_else(|| bad("study name"))?);
    string(&mut material, &registered.to_string_lossy());
    for d in &tape {
        integers(&mut material, &d.indices);
        for n in [d.sampler, d.input, d.target] {
            put_varint(&mut material, n);
        }
    }
    let pair = hash(&material);
    let mut origins = old.origins.clone();
    origins.retain(|o| o.role != "execution-binary");
    for (role, path) in [
        ("execution-binary", std::env::current_exe()?),
        ("cooldown-parent-inputs", parent.join("inputs.r3er")),
        (
            "cooldown-parent-terminal",
            parent.join(&command.terminal.locator),
        ),
        (
            "cooldown-parent-command",
            parent.join(command_locator(&command.terminal)?),
        ),
        (
            "cooldown-legacy-proof",
            parent.parent().unwrap().join("preflight-proof.r3er"),
        ),
    ] {
        origins.push(Origin {
            role: role.into(),
            original: FileRef {
                digest: unhex(&file_hash(&path)?)?,
                locator: path.canonicalize()?.display().to_string(),
            },
        });
    }
    origins.push(Origin {
        role: "cooldown-registered-root".into(),
        original: FileRef {
            locator: registered.display().to_string(),
            digest: pair,
        },
    });
    let mut registrations = Vec::new();
    for lr in [2, 3] {
        let mut s = old.clone();
        s.contract = COOLDOWN_CONTRACT.into();
        s.source = evaluator_source();
        s.origins = origins.clone();
        let mut id = material.clone();
        id.push(lr);
        s.run = hash(&id);
        s.policy = s.run;
        s.parent = native.clone();
        s.parent.run = s.run;
        s.parent.segment = 0;
        s.parent.file.locator = "parent.r3m".into();
        s.tape = tape.clone();
        s.lr_policy = lr;
        s.lr_offset = 0;
        s.eval_steps = vec![128, 256];
        s.purpose = RunPurpose::LrSplit;
        s.baseline = [dev.exact, watch.exact, dev.errors + watch.errors];
        s.authorization = Some(AnchorAuthorization {
            pair,
            baseline: command.terminal.digest,
            expected_parent: expected,
            anchors: 6,
            pools: auth.pools.clone(),
        });
        s.validate()?;
        for n in [1, 128, 256] {
            let mut future = state.clone();
            fork_budget(
                &mut future,
                s.parent.step,
                s.parent.counters[0],
                256,
                1_000_000,
            )?;
            future.step += n;
            future.consumed_tokens += tape[..n].iter().map(|d| d.input).sum::<u64>();
            future.target_tokens += tape[..n].iter().map(|d| d.target).sum::<u64>();
            future.sampler_state = tape[n - 1].sampler;
            let mut manifest = l.manifest.clone();
            manifest.training = Some(future);
            checkpoint::validate_metadata(&manifest, &l.tokenizer)?;
            println!(
                "SAVE_METADATA_PRECHECK arm={} local_step={n} absolute_step={} LR_BITS={} ACTUAL_UPDATES=0",
                s.arm_name(),
                s.parent.step + n as u64,
                cooldown_lr(lr, n as u64)?.to_bits()
            );
        }
        registrations.push(s);
    }
    control.check("cooldown_register")?;
    std::fs::create_dir(output)?;
    std::fs::copy(
        parent.join(command_locator(&command.terminal)?),
        output.join("parent-command.r3er"),
    )?;
    for s in registrations {
        let dir = output.join(s.arm_name());
        std::fs::create_dir(&dir)?;
        std::fs::copy(parent.join(&native.file.locator), dir.join("parent.r3m"))?;
        publish(&dir, "inputs.r3er", &Record::Inputs(Box::new(s.clone())))?;
        resolve_native(&dir, &s, &s.parent, true)?;
        println!(
            "REGISTERED ARM={} source={} binary={} binding={} parent={} model={} Adam={} cursor={} tape=EXACT_A75_CONTINUATION HORIZON=256 predicted_input={} predicted_target={} SMALL_UPDATES=0",
            s.arm_name(),
            hex(&s.source),
            hex(&binary),
            hex(&s.binding()),
            hex(&expected),
            hex(&s.parent.model),
            hex(&s.parent.adam.unwrap()),
            s.parent.counters[2],
            s.tape.iter().map(|d| d.input).sum::<u64>(),
            s.tape.iter().map(|d| d.target).sum::<u64>()
        );
    }
    for kind in [PanelKind::Dev, PanelKind::Cross, PanelKind::Ordinary] {
        let (e, l) = payload(parent, &old, &history.evaluations[&(native.step, kind)])?;
        print_anchor_panel(&old, &e, &l)?;
    }
    Ok(())
}

fn metadata_sample(s: &RunSnapshot, kind: PanelKind, count: usize) -> Result<Vec<u32>> {
    let mut cases = panel(s, kind)?.cases.clone();
    cases.sort_by_key(|i| {
        let e = &s.cases[*i as usize];
        (
            e.category,
            e.family.clone(),
            scene(e).to_owned(),
            e.id.clone(),
        )
    });
    let n = count.min(cases.len());
    Ok((0..n).map(|i| cases[i * cases.len() / n]).collect())
}
fn origin_path(s: &RunSnapshot, role: &str) -> Result<PathBuf> {
    let refs = s
        .origins
        .iter()
        .filter(|o| o.role == role)
        .collect::<Vec<_>>();
    if refs.len() != 1 {
        return Err(bad("unique provenance role"));
    }
    let r = &refs[0].original;
    let p = PathBuf::from(&r.locator);
    if unhex(&file_hash(&p)?)? != r.digest {
        return Err(bad("provenance bytes changed"));
    }
    Ok(p)
}
fn cooldown_verify(root: &Path, confirmation: bool, control: &mut RunControl) -> Result<()> {
    let study = read_inputs(&root.join("K-KEEP"))?;
    if !study.cooldown() {
        return Err(bad("verification requires cooldown study"));
    }
    let target = if confirmation {
        root.join("confirmation")
    } else {
        root.to_path_buf()
    };
    if target.exists()
        && let Some(outcome) = read_preflight_outcome(&target)?
    {
        outcome.require_current_success()?;
        println!("COOLDOWN_VERIFICATION=READ_ONLY NEW_GENERATIONS=0");
        return Ok(());
    }
    let (dir, snapshot, native, command, baseline, scope) = if confirmation {
        let mut candidates = Vec::new();
        for arm in ["K-KEEP", "D-DECAY"] {
            let dir = root.join(arm);
            let s = read_inputs(&dir)?;
            let closed = arm_commands(&dir, &s)?;
            let (t, c) = closed
                .last()
                .ok_or_else(|| bad("confirmation before pair complete"))?;
            if c.status != CommandStatus::Complete || !t.complete {
                return Err(bad("confirmation incomplete arm"));
            }
            let Record::Comparison(score) = read_record(
                &dir,
                c.comparison
                    .as_ref()
                    .ok_or_else(|| bad("comparison missing"))?,
            )?
            else {
                return Err(bad("comparison kind"));
            };
            if score.candidate {
                let get = |k| {
                    score
                        .panels
                        .iter()
                        .find(|(kind, _)| *kind == k)
                        .map(|(_, s)| s)
                        .ok_or_else(|| bad("candidate panel missing"))
                };
                candidates.push((
                    (
                        get(PanelKind::Cross)?.exact,
                        get(PanelKind::Dev)?.exact,
                        get(PanelKind::Ordinary)?.qa[0],
                        u8::from(arm == "K-KEEP"),
                    ),
                    dir,
                    s,
                    t.native.clone().ok_or_else(|| bad("candidate native"))?,
                    c.clone(),
                ));
            }
        }
        candidates.sort_by_key(|v| v.0);
        let Some((_, dir, s, n, c)) = candidates.pop() else {
            println!("FRESH_CONFIRMATION=NOT_RUN_NO_JOINT_CANDIDATE H3_SEAL=NOT_OPENED");
            return Ok(());
        };
        // Selection uses stored metadata only; the complete independent close follows durable intent.
        (dir, s, n, c, None, 2)
    } else {
        let path = origin_path(&study, "cooldown-parent-inputs")?;
        let dir = path
            .parent()
            .ok_or_else(|| bad("parent provenance root"))?
            .to_path_buf();
        let old = read_inputs(&dir)?;
        let cp = origin_path(&study, "cooldown-parent-command")?;
        let Record::Command(c) = read_record(
            &dir,
            &reference(
                &dir,
                &cp.strip_prefix(&dir)
                    .map_err(|_| bad("parent command path"))?
                    .to_string_lossy(),
            )?,
        )?
        else {
            return Err(bad("parent command kind"));
        };
        let t = segment(&dir, &c.terminal, &old)?;
        (
            dir,
            old,
            t.native.ok_or_else(|| bad("parent native"))?,
            c,
            Some(study.clone()),
            1,
        )
    };
    if confirmation {
        std::fs::create_dir(&target)?;
        std::fs::copy(dir.join("inputs.r3er"), target.join("inputs.r3er"))?;
        std::fs::copy(dir.join(&native.file.locator), target.join("native.r3m"))?;
        std::fs::copy(
            dir.join(command_locator(&command.terminal)?),
            target.join("command.r3er"),
        )?;
    }
    let inputs = if confirmation {
        [
            reference(&target, "inputs.r3er")?,
            reference(&target, "inputs.r3er")?,
        ]
    } else {
        [
            reference(root, "K-KEEP/inputs.r3er")?,
            reference(root, "D-DECAY/inputs.r3er")?,
        ]
    };
    let commands = if confirmation {
        [
            reference(&target, "command.r3er")?,
            reference(&target, "command.r3er")?,
        ]
    } else {
        [
            reference(root, "parent-command.r3er")?,
            reference(root, "parent-command.r3er")?,
        ]
    };
    let natives = if confirmation {
        [
            reference(&target, "native.r3m")?,
            reference(&target, "native.r3m")?,
        ]
    } else {
        [
            reference(root, "K-KEEP/parent.r3m")?,
            reference(root, "D-DECAY/parent.r3m")?,
        ]
    };
    let specs = if confirmation {
        [PanelKind::Dev, PanelKind::Cross, PanelKind::Ordinary]
            .iter()
            .map(|k| Ok((*k, panel(&snapshot, *k)?.cases.clone())))
            .collect::<Result<Vec<_>>>()?
    } else {
        [PanelKind::Dev, PanelKind::Ordinary]
            .iter()
            .map(|k| Ok((*k, metadata_sample(&snapshot, *k, 8)?)))
            .collect::<Result<Vec<_>>>()?
    };
    let order = specs
        .iter()
        .enumerate()
        .flat_map(|(i, (_, cases))| {
            cases
                .iter()
                .map(move |n| (if confirmation { 0 } else { i as u8 }, *n))
        })
        .collect::<Vec<_>>();
    let start = VerificationStart {
        scope,
        root: target.canonicalize()?.display().to_string(),
        pair: study
            .authorization
            .as_ref()
            .ok_or_else(|| bad("study authorization"))?
            .pair,
        source: evaluator_source(),
        binary: unhex(&file_hash(&std::env::current_exe()?)?)?,
        inputs,
        commands,
        natives,
        generation_limit: order.len() as u64,
        seconds_limit: 1800,
        order,
    };
    if start.source != study.source
        || origin_path(&study, "execution-binary")? != std::env::current_exe()?.canonicalize()?
    {
        return Err(bad("verification registered source/binary"));
    }
    finish_verification_attempt(
        &target,
        start.clone(),
        snapshot.tiny_spec,
        control,
        |control, returned| {
            if confirmation {
                let (_, g, seconds) =
                    anchor_budget(&root.join("D-DECAY"), &read_inputs(&root.join("D-DECAY"))?)?;
                if g + start.order.len() > 4096 {
                    return Err(bad("confirmation generation budget"));
                }
                control.deadline =
                    control.start + Duration::from_secs_f64((7200. - seconds).min(1800.));
            }
            close_native_inner(&dir, &command.terminal.locator, control, false)?;
            let chain = lineage(&dir, &snapshot, &command.terminal)?;
            let h = verified_history(&dir, &snapshot, &chain)?;
            let l = resolve_native(&dir, &snapshot, &native, true)?;
            let sr = reference(&target, "preflight-start.r3er")?;
            let mut proof_panels = Vec::new();
            let mut scores = Vec::new();
            for (panel_index, (kind, cases)) in specs.iter().enumerate() {
                let (old, _) = payload(&dir, &snapshot, &h.evaluations[&(native.step, *kind)])?;
                let mut rows = Vec::new();
                for ordinal in cases {
                    let i = returned.len();
                    let elapsed = control.start.elapsed().as_secs_f64();
                    let row = evaluate_row_with_entry(
                        &l,
                        &snapshot.cases[*ordinal as usize],
                        *ordinal,
                        control,
                        false,
                        true,
                        || {
                            publish(
                                &target,
                                &format!("preflight-row-{i:02}-start.r3er"),
                                &Record::VerificationRow(VerificationRow {
                                    start: sr.clone(),
                                    index: i as u32,
                                    row: None,
                                    elapsed: Scalar::F64(elapsed),
                                }),
                            )
                            .map(|_| ())
                        },
                    )?;
                    returned.push(row.clone());
                    verification_fault(snapshot.tiny_spec, "raw", control)?;
                    publish(
                        &target,
                        &format!("preflight-row-{i:02}.r3er"),
                        &Record::VerificationRow(VerificationRow {
                            start: sr.clone(),
                            index: i as u32,
                            row: Some(row.clone()),
                            elapsed: Scalar::F64(control.start.elapsed().as_secs_f64()),
                        }),
                    )?;
                    verification_fault(snapshot.tiny_spec, "returned", control)?;
                    control.check("cooldown_verification_returned")?;
                    let original = old
                        .rows
                        .iter()
                        .find(|r| r.ordinal == *ordinal)
                        .ok_or_else(|| bad("parent raw ordinal"))?;
                    row_output(&row, &l)?;
                    if row.tokens != original.tokens
                        || row.eos != original.eos
                        || row.finish != original.finish
                        || row.error != original.error
                        || !row.completed
                        || row.interruption.is_some()
                    {
                        return Err(bad("fresh normal token/EOS/error parity mismatch"));
                    }
                    rows.push(row);
                }
                let s = if let Some(b) = &baseline {
                    if panel_index == 0 {
                        b.clone()
                    } else {
                        read_inputs(&root.join("D-DECAY"))?
                    }
                } else {
                    snapshot.clone()
                };
                let e = EvalPayload {
                    run: s.run,
                    binding: s.binding(),
                    source: evaluator_source(),
                    model: native.model,
                    tokenizer: native.tokenizer,
                    architecture: native.architecture,
                    step: native.step,
                    new_updates: if confirmation { 256 } else { 0 },
                    kind: *kind,
                    expected: cases.len() as u32,
                    rows,
                };
                if confirmation {
                    let score = rescore(&s, &e, &l, true)?;
                    print_anchor_panel(&s, &e, &l)?;
                    scores.push((*kind, score));
                }
                proof_panels.push(publish(
                    &target,
                    &format!("verification-{}.r3er", kind.name()),
                    &Record::Evaluation(e),
                )?);
                println!(
                    "VERIFICATION_PANEL={} completed={} total_generations={}",
                    kind.name(),
                    cases.len(),
                    control.generation_calls
                );
            }
            if confirmation {
                // Watch is the same ordinary subset. It must not consume another model call.
                let Record::Evaluation(mut watch) =
                    read_record(&target, proof_panels.last().unwrap())?
                else {
                    return Err(bad("confirmation ordinary kind"));
                };
                let spec = panel(&snapshot, PanelKind::Watch)?;
                watch.rows = spec
                    .cases
                    .iter()
                    .map(|n| {
                        watch
                            .rows
                            .iter()
                            .find(|r| r.ordinal == *n)
                            .cloned()
                            .ok_or_else(|| bad("confirmation watch member"))
                    })
                    .collect::<Result<_>>()?;
                watch.kind = PanelKind::Watch;
                watch.expected = spec.cases.len() as u32;
                scores.push((PanelKind::Watch, rescore(&snapshot, &watch, &l, true)?));
                if !eligible(&snapshot, &scores, false, &[]) {
                    return Err(bad("fresh confirmation joint gate failed"));
                }
            }
            control.check("cooldown_verification_proof")?;
            publish(
                &target,
                "preflight-proof.r3er",
                &Record::Preflight(PreflightReceipt {
                    pair: start.pair,
                    source: start.source,
                    binary: start.binary,
                    commands: start.commands.clone(),
                    panels: proof_panels,
                    elapsed: Scalar::F64(control.start.elapsed().as_secs_f64()),
                    generations: control.generation_calls as u64,
                }),
            )?;
            Ok(())
        },
    )
}

fn anchor_budget(root: &Path, s: &RunSnapshot) -> Result<(u64, usize, f64)> {
    let a = s
        .authorization
        .as_ref()
        .ok_or_else(|| bad("missing anchor authorization"))?;
    if root.file_name().and_then(|v| v.to_str()) != Some(s.arm_name()) {
        return Err(bad("pair arm locator"));
    }
    let parent = root.parent().ok_or_else(|| bad("pair root"))?;
    let mut updates = 0;
    let mut generations = 0;
    let mut seconds = 0.;
    let arms: &[&str] = if s.cooldown() {
        &["K-KEEP", "D-DECAY"]
    } else if s.contract == RESTART_CONTRACT {
        &["preflight-A", "preflight-B", "C50-R", "A75-R"]
    } else {
        &["C50", "A75"]
    };
    let current = arms
        .iter()
        .position(|arm| *arm == s.arm_name())
        .ok_or_else(|| bad("unknown arm"))?;
    for (position, &arm) in arms.iter().enumerate() {
        let arm_root = parent.join(arm);
        let other = read_inputs(&arm_root)?;
        let other_a = other
            .authorization
            .as_ref()
            .ok_or_else(|| bad("pair authorization missing"))?;
        if other_a.pair != a.pair
            || other.arm_name() != arm
            || other.parent.model != s.parent.model
            || other.parent.adam != s.parent.adam
        {
            return Err(bad("pair parent/policy mismatch"));
        }
        if s.cooldown()
            && (!other.cooldown()
                || other.source != s.source
                || other.parent.file.digest != s.parent.file.digest
                || other.parent.counters != s.parent.counters
                || other.train != s.train
                || other.cases.iter().map(case_hash).collect::<Vec<_>>()
                    != s.cases.iter().map(case_hash).collect::<Vec<_>>()
                || other_a.pools != a.pools
                || other.tape.len() != s.tape.len()
                || other.tape.iter().zip(&s.tape).any(|(a, b)| {
                    a.indices != b.indices
                        || a.sampler != b.sampler
                        || a.input != b.input
                        || a.target != b.target
                }))
        {
            return Err(bad("cooldown identical parent/data/tape/source"));
        }
        let closed = arm_commands(&arm_root, &other)?;
        if arm != s.arm_name()
            && closed
                .last()
                .is_some_and(|(_, c)| c.status != CommandStatus::Complete)
        {
            return Err(bad("other arm is not complete"));
        }
        if position < current && closed.is_empty() {
            return Err(bad("previous preflight/control arm must complete first"));
        }
        if position > current && !closed.is_empty() {
            return Err(bad("later arm already started; earlier arm is closed"));
        }
        for (t, c) in closed {
            updates += t.draws.len() as u64;
            generations += t.generations as usize;
            seconds += c.elapsed.finite()?;
        }
    }
    if !s.preflight() && matches!(s.contract.as_str(), RESTART_CONTRACT | COOLDOWN_CONTRACT) {
        let p = read_preflight_outcome(parent)?.ok_or_else(|| bad("missing preflight outcome"))?;
        generations += p.entries as usize;
        seconds += p.elapsed + p.publication_reserve;
        println!(
            "PAIR_USAGE_KNOWN updates={updates} generations={generations} charged_seconds={seconds} verification_elapsed_lower_bound={} publication_reserve_seconds={}",
            p.elapsed, p.publication_reserve
        );
        p.require_current_success()?;
        if s.cooldown() {
            let Record::VerificationStart(start) =
                read_record(parent, &reference(parent, "preflight-start.r3er")?)?
            else {
                return Err(bad("parent parity intent"));
            };
            if start.scope != 1
                || start.pair != a.pair
                || start.source != s.source
                || start.inputs
                    != [
                        reference(parent, "K-KEEP/inputs.r3er")?,
                        reference(parent, "D-DECAY/inputs.r3er")?,
                    ]
            {
                return Err(bad("parent parity study binding"));
            }
        }
    }
    if updates
        > if s.cooldown() {
            512
        } else if s.contract == RESTART_CONTRACT {
            1028
        } else {
            1024
        }
        || generations > if s.cooldown() { 4096 } else { 7500 }
        || seconds >= 7200.
    {
        return Err(bad("anchor pair budget exhausted"));
    }
    Ok((updates, generations, seconds))
}
fn arm_commands(root: &Path, s: &RunSnapshot) -> Result<Vec<(SegmentReceipt, CommandOutcome)>> {
    let mut paths = std::fs::read_dir(root)?
        .map(|e| e.map(|e| e.path()))
        .collect::<std::io::Result<Vec<_>>>()?;
    paths.retain(|p| {
        p.file_name()
            .is_some_and(|v| v.to_string_lossy().starts_with("segment-"))
    });
    paths.sort();
    let mut out = Vec::new();
    for p in paths {
        let locator = p
            .join("terminal.r3er")
            .strip_prefix(root)
            .map_err(|_| bad("segment path"))?
            .to_string_lossy()
            .into_owned();
        let reference = reference(root, &locator)?;
        let c = effective_outcome(root, s, &reference)?;
        out.push((segment(root, &reference, s)?, c));
    }
    Ok(out)
}
struct PreflightOutcome {
    succeeded: bool,
    legacy: bool,
    entries: u64,
    elapsed: f64,
    publication_reserve: f64,
}
impl PreflightOutcome {
    fn require_current_success(&self) -> Result<()> {
        if !self.succeeded || self.legacy {
            return Err(bad(
                "verification failed/incomplete/legacy; retry and subsequent arm prohibited",
            ));
        }
        Ok(())
    }
}
fn verification_intent(root: &Path) -> Result<VerificationStart> {
    // Metadata and exact file references only. No native load, close, forward or generation here.
    let mut inputs = Vec::new();
    let mut commands = Vec::new();
    let mut natives = Vec::new();
    let mut order = Vec::new();
    let mut pair = None;
    for (i, (arm, purpose, end)) in [
        ("preflight-A", RunPurpose::SaveContinuous, 0),
        ("preflight-B", RunPurpose::SaveSplit, 1),
    ]
    .iter()
    .enumerate()
    {
        let dir = root.join(arm);
        let s = read_inputs(&dir)?;
        if !s.tiny_spec {
            return Err(bad(
                "PRE_START_REJECTED closed SMALL preflight authorization; explicit legacy read-only only",
            ));
        }
        if s.purpose != *purpose || s.tape.len() != 2 || s.source != evaluator_source() {
            return Err(bad("PRE_START_REJECTED preflight purpose/source"));
        }
        let current = s.authorization.as_ref().map_or(s.parent.model, |a| a.pair);
        if pair.is_some_and(|p| p != current) {
            return Err(bad("PRE_START_REJECTED mismatched pair"));
        }
        pair = Some(current);
        inputs.push(reference(root, &format!("{arm}/inputs.r3er"))?);
        let c = reference(root, &format!("{arm}/segment-{end:02}/command.r3er"))?;
        let Record::Command(command) = read_record(root, &c)? else {
            return Err(bad("PRE_START_REJECTED command kind"));
        };
        let Record::Segment(t) = read_record(&dir, &command.terminal)? else {
            return Err(bad("PRE_START_REJECTED terminal kind"));
        };
        let n = t
            .native
            .ok_or_else(|| bad("PRE_START_REJECTED endpoint absent"))?;
        let native = reference(root, &format!("{arm}/{}", n.file.locator))?;
        if native.digest != n.file.digest {
            return Err(bad("PRE_START_REJECTED endpoint digest"));
        }
        commands.push(c);
        natives.push(native);
        order.extend(
            panel(&s, PanelKind::Ordinary)?
                .cases
                .iter()
                .take(2)
                .map(|o| (i as u8, *o)),
        );
    }
    Ok(VerificationStart {
        scope: 0,
        root: root.canonicalize()?.to_string_lossy().into_owned(),
        pair: pair.ok_or_else(|| bad("empty verification"))?,
        source: evaluator_source(),
        binary: unhex(&file_hash(&std::env::current_exe()?)?)?,
        inputs: inputs.try_into().map_err(|_| bad("verification inputs"))?,
        commands: commands
            .try_into()
            .map_err(|_| bad("verification commands"))?,
        natives: natives
            .try_into()
            .map_err(|_| bad("verification natives"))?,
        generation_limit: order.len() as u64,
        seconds_limit: 1800,
        order,
    })
}
fn verification_rows(
    root: &Path,
    start_ref: &FileRef,
    start: &VerificationStart,
) -> Result<Vec<(FileRef, EvalRow, f64)>> {
    let mut rows = Vec::new();
    let inputs = start
        .inputs
        .iter()
        .map(|r| match read_record(root, r)? {
            Record::Inputs(s) => Ok(s),
            _ => Err(bad("verification inputs kind")),
        })
        .collect::<Result<Vec<_>>>()?;
    let mut missing = false;
    for (i, (arm, ordinal)) in start.order.iter().enumerate() {
        let reservation = format!("preflight-row-{i:02}-start.r3er");
        let name = format!("preflight-row-{i:02}.r3er");
        if !root.join(&name).exists() {
            missing = true;
            continue;
        }
        if missing {
            return Err(bad("verification row sequence gap"));
        }
        let Record::VerificationRow(reserved) = read_record(root, &reference(root, &reservation)?)?
        else {
            return Err(bad("verification entry kind"));
        };
        let r = reference(root, &name)?;
        let Record::VerificationRow(v) = read_record(root, &r)? else {
            return Err(bad("verification row kind"));
        };
        if reserved.start != *start_ref
            || reserved.index != i as u32
            || reserved.row.is_some()
            || v.start != *start_ref
            || v.index != i as u32
        {
            return Err(bad("verification row/intent binding"));
        }
        let row = v
            .row
            .ok_or_else(|| bad("missing returned verification row"))?;
        let s = &inputs[*arm as usize];
        if row.ordinal != *ordinal
            || s.cases
                .get(*ordinal as usize)
                .is_none_or(|c| row.case != case_hash(c))
            || !row.started
        {
            return Err(bad("verification case/entry binding"));
        }
        rows.push((r, row, v.elapsed.finite()?));
    }
    Ok(rows)
}
fn read_preflight_outcome(root: &Path) -> Result<Option<PreflightOutcome>> {
    let start_path = root.join("preflight-start.r3er");
    if !start_path.exists() {
        if root.join("preflight-proof.r3er").exists() {
            let Record::Preflight(p) =
                read_record(root, &reference(root, "preflight-proof.r3er")?)?
            else {
                return Err(bad("legacy preflight kind"));
            };
            println!(
                "PREFLIGHT_STATUS=LEGACY_SUCCESS_WITHOUT_ATTEMPT_INTENT GENERATION_API_ENTRIES={} ELAPSED_LOWER_BOUND={} NEW_GENERATIONS=0",
                p.generations,
                p.elapsed.finite()?
            );
            return Ok(Some(PreflightOutcome {
                succeeded: true,
                legacy: true,
                entries: p.generations,
                elapsed: p.elapsed.finite()?,
                publication_reserve: 0.,
            }));
        }
        let fragments = std::fs::read_dir(root)?
            .collect::<std::io::Result<Vec<_>>>()?
            .iter()
            .any(|e| {
                let n = e.file_name();
                let n = n.to_string_lossy();
                n.starts_with("preflight-row-")
                    || n.starts_with("preflight-final")
                    || n.starts_with(".preflight-")
            });
        if fragments
            || root.join("preflight-A/parity.r3er").exists()
            || root.join("preflight-B/parity.r3er").exists()
        {
            return Err(bad(
                "INTERRUPTED_UNKNOWN verification fragments without intent; retry prohibited",
            ));
        }
        return Ok(None);
    }
    let sr = reference(root, "preflight-start.r3er")?;
    let Record::VerificationStart(start) = read_record(root, &sr)? else {
        return Err(bad("verification intent kind"));
    };
    if start.root != root.canonicalize()?.to_string_lossy() {
        return Err(bad("verification attempt moved outside registered root"));
    }
    for r in start
        .inputs
        .iter()
        .chain(&start.commands)
        .chain(&start.natives)
    {
        if reference(root, &r.locator)? != *r {
            return Err(bad("verification input/endpoint changed"));
        }
    }
    let rows = verification_rows(root, &sr, &start)?;
    let entries = rows.len() as u64;
    let tokens = rows
        .iter()
        .map(|(_, r, _)| r.tokens.len() as u64)
        .sum::<u64>();
    let elapsed = rows.last().map_or(0., |(_, _, t)| *t);
    if !root.join("preflight-final.r3er").exists() {
        println!(
            "PREFLIGHT_STATUS=INTERRUPTED_UNKNOWN BEGIN_DURABLE=true GENERATION_API_ENTRIES_KNOWN={entries} OBSERVED_TOKENS_KNOWN={tokens} ELAPSED_LOWER_BOUND={elapsed} RESERVED_UPPER_BOUND={} RESERVED_SECONDS={} UNKNOWN_TAIL=true ELAPSED_TOTAL=UNKNOWN TOKEN_TOTAL=UNKNOWN NEW_GENERATIONS=0",
            start.generation_limit, start.seconds_limit
        );
        return Ok(Some(PreflightOutcome {
            succeeded: false,
            legacy: false,
            entries,
            elapsed,
            publication_reserve: start.seconds_limit as f64,
        }));
    }
    let Record::VerificationFinal(f) =
        read_record(root, &reference(root, "preflight-final.r3er")?)?
    else {
        return Err(bad("verification final kind"));
    };
    if f.start != sr
        || f.rows != rows.iter().map(|(r, _, _)| r.clone()).collect::<Vec<_>>()
        || f.entries < entries
        || f.entries > start.generation_limit
        || f.tokens < tokens
        || f.elapsed.finite()? < elapsed
    {
        return Err(bad("verification final usage/raw binding"));
    }
    if f.succeeded {
        let proof_ref = f
            .proof
            .as_ref()
            .ok_or_else(|| bad("verification success without proof"))?;
        let Record::Preflight(p) = read_record(root, proof_ref)? else {
            return Err(bad("verification final proof kind"));
        };
        if proof_ref.locator != "preflight-proof.r3er"
            || p.pair != start.pair
            || p.source != start.source
            || p.binary != start.binary
            || p.commands != start.commands
            || p.generations != f.entries
            || entries != start.order.len() as u64
            || f.tokens != tokens
            || rows
                .iter()
                .any(|(_, r, _)| !r.completed || r.interruption.is_some())
        {
            return Err(bad("verification positive final/proof mismatch"));
        }
        let mut accounted = BTreeSet::new();
        if p.panels.len() != if start.scope == 2 { 3 } else { 2 } {
            return Err(bad("verification panel count"));
        }
        let loaded = if start.scope > 0 {
            if start.natives[0].digest != start.natives[1].digest {
                return Err(bad(
                    "scoped verification must use one exact native endpoint",
                ));
            }
            Some(checkpoint::load(
                &owned_path(root, &start.natives[0].locator, true)?,
                Device::Cpu,
                true,
            )?)
        } else {
            None
        };
        for (arm, r) in p.panels.iter().enumerate() {
            let Record::Evaluation(e) = read_record(root, r)? else {
                return Err(bad("verification proof raw kind"));
            };
            let expected = rows
                .iter()
                .enumerate()
                .filter(|(i, (_, row, _))| {
                    if start.scope == 2 {
                        e.rows.iter().any(|r| r.ordinal == row.ordinal)
                    } else {
                        start.order[*i].0 as usize == arm
                    }
                })
                .map(|(_, (_, row, _))| row)
                .collect::<Vec<_>>();
            let encode = |row: &EvalRow| {
                let mut b = Vec::new();
                row.encode(&mut b);
                b
            };
            if let Some(l) = &loaded {
                let Record::Inputs(s) =
                    read_record(root, &start.inputs[if start.scope == 2 { 0 } else { arm }])?
                else {
                    return Err(bad("verification snapshot kind"));
                };
                if e.run != s.run
                    || e.binding != s.binding()
                    || e.source != start.source
                    || e.model != unhex(&l.model.weight_hash()?)?
                    || e.tokenizer != unhex(&l.tokenizer.semantic_id())?
                    || e.architecture != unhex(&l.model.config.semantic_id()?)?
                    || e.step
                        != l.manifest
                            .training
                            .as_ref()
                            .ok_or_else(|| bad("verification endpoint step"))?
                            .step as u64
                    || e.expected as usize != expected.len()
                {
                    return Err(bad("verification proof actual endpoint/policy"));
                }
                if start.scope == 2 {
                    rescore(&s, &e, l, true)?;
                } else if e.rows.iter().map(|r| r.ordinal).collect::<Vec<_>>()
                    != metadata_sample(&s, e.kind, 8)?
                {
                    return Err(bad("parent metadata-only sample changed"));
                }
                for row in &e.rows {
                    row_output(row, l)?;
                }
            }
            if e.rows.len() != expected.len()
                || e.rows
                    .iter()
                    .zip(expected)
                    .any(|(a, b)| encode(a) != encode(b))
            {
                return Err(bad("proof dropped/changed returned rows"));
            }
            for row in &e.rows {
                let key = (if start.scope == 2 { 0 } else { arm }, row.ordinal);
                if !accounted.insert(key) {
                    return Err(bad("duplicate proof row"));
                }
            }
        }
        if accounted.len() != rows.len() {
            return Err(bad("proof panel coverage"));
        }
    }
    println!(
        "PREFLIGHT_STATUS={} BEGIN_DURABLE=true GENERATION_API_ENTRIES={} COMPLETED={} INTERRUPTED={} OBSERVED_TOKENS={} TEACHERS={} OPTIMIZER_CALLS=0 ELAPSED_LOWER_BOUND={} RESERVED_UPPER_BOUND={} UNKNOWN_TAIL={} PROOF_BOUND={} STOP={:?} ERROR={:?} SAVE_ERROR={:?} NEW_GENERATIONS=0",
        if f.succeeded { "Succeeded" } else { "Failed" },
        f.entries,
        f.completed,
        f.interrupted,
        f.tokens,
        f.teachers,
        f.elapsed.finite()?,
        start.generation_limit,
        f.unknown_tail,
        f.succeeded,
        f.stop,
        f.error,
        f.save_error
    );
    Ok(Some(PreflightOutcome {
        succeeded: f.succeeded,
        legacy: false,
        entries: f.entries,
        elapsed: f.elapsed.finite()?,
        // Last immutable publication cannot contain its own completed fsync duration.
        // Retain its observed lower bound and charge the finite cleanup reservation, not zero.
        publication_reserve: 120.,
    }))
}
fn verification_fault(tiny: bool, boundary: &str, control: &mut RunControl) -> Result<()> {
    #[cfg(feature = "test-support")]
    if tiny {
        let mode = std::env::var("R3ER_TEST_STOP").unwrap_or_default();
        if boundary == "started" && mode == "pv-start-kill" {
            std::process::exit(93);
        }
        if boundary == "started" && mode == "pv-hold-start" {
            std::thread::sleep(Duration::from_millis(300));
        }
        if boundary == "row" && mode.starts_with("pv-after-row") {
            control.cancel.store(true, Ordering::Relaxed);
        }
        if boundary == "row" && mode == "pv-row-deadline" {
            control.deadline = Instant::now();
        }
        if mode == format!("pv-{boundary}-write-fail")
            || boundary == "final" && mode == "pv-after-row-final-write-fail"
        {
            return Err(
                std::io::Error::other(format!("injected {boundary} publication failure")).into(),
            );
        }
    }
    let _ = (tiny, boundary, control);
    Ok(())
}
fn verify_save_preflight(root: &Path, control: &mut RunControl, generate: bool) -> Result<()> {
    if let Some(status) = read_preflight_outcome(root)? {
        status.require_current_success()?;
        return preflight_body(root, control, false, false, &mut Vec::new());
    }
    if !generate {
        return Err(bad(
            "missing verification intent/final; subsequent arm prohibited",
        ));
    }
    let start = verification_intent(root)?;
    let tiny = read_inputs(&root.join("preflight-A"))?.tiny_spec;
    finish_verification_attempt(root, start, tiny, control, |control, returned| {
        preflight_body(root, control, true, false, returned)
    })
}
fn finish_verification_attempt(
    root: &Path,
    start: VerificationStart,
    tiny: bool,
    control: &mut RunControl,
    work: impl FnOnce(&mut RunControl, &mut Vec<EvalRow>) -> Result<()>,
) -> Result<()> {
    control.generation_limit = start.generation_limit as usize;
    verification_fault(tiny, "start", control)?;
    // hard_link(create-new) in the existing publisher elects a single observer. No work before success.
    let sr = publish(
        root,
        "preflight-start.r3er",
        &Record::VerificationStart(start.clone()),
    )?;
    println!(
        "VERIFICATION_BEGIN_DURABLE=true CONTRACT={COOLDOWN_CONTRACT} ATTEMPT_ID={} RESERVED_UPPER_BOUND={} UNKNOWN_TAIL=true",
        hex(&sr.digest),
        start.generation_limit
    );
    let mut returned = Vec::new();
    let result = (|| {
        verification_fault(tiny, "started", control)?;
        work(control, &mut returned)
    })();
    if let Err(e) = &result {
        control.classify_error(e);
    }
    let _ = control.seal_terminal();
    let rows = (0..returned.len())
        .filter_map(|i| reference(root, &format!("preflight-row-{i:02}.r3er")).ok())
        .collect::<Vec<_>>();
    let proof = reference(root, "preflight-proof.r3er").ok();
    let f = VerificationFinal {
        start: sr,
        unknown_tail: rows.len() != returned.len(),
        rows,
        proof,
        succeeded: result.is_ok() && control.observed.is_empty(),
        entries: control.generation_calls as u64,
        completed: returned
            .iter()
            .filter(|r| r.completed && r.interruption.is_none())
            .count() as u64,
        interrupted: returned.iter().filter(|r| r.interruption.is_some()).count() as u64,
        tokens: returned.iter().map(|r| r.tokens.len() as u64).sum(),
        teachers: control.teacher_calls as u64,
        elapsed: Scalar::F64(control.start.elapsed().as_secs_f64()),
        stop: control.observed.clone(),
        error: result.as_ref().err().map(ToString::to_string),
        save_error: result
            .as_ref()
            .err()
            .filter(|e| matches!(e, Error::Io(_)))
            .map(ToString::to_string),
    };
    let saved = verification_fault(tiny, "final", control)
        .and_then(|_| publish(root, "preflight-final.r3er", &Record::VerificationFinal(f)));
    println!(
        "VERIFICATION_COMMAND_ELAPSED={} FINAL_PUBLICATION_OK={} ORIGINAL_ERROR={:?} PUBLICATION_ERROR={:?}",
        control.start.elapsed().as_secs_f64(),
        saved.is_ok(),
        result.as_ref().err(),
        saved.as_ref().err()
    );
    saved?;
    let status = read_preflight_outcome(root)?.ok_or_else(|| bad("verification outcome absent"))?;
    result?;
    status.require_current_success()
}
fn preflight_body(
    root: &Path,
    control: &mut RunControl,
    generate: bool,
    legacy: bool,
    returned: &mut Vec<EvalRow>,
) -> Result<()> {
    if generate {
        let Record::VerificationStart(start) =
            read_record(root, &reference(root, "preflight-start.r3er")?)?
        else {
            return Err(bad("preflight durable intent missing"));
        };
        control.generation_limit = start.generation_limit as usize;
    }
    let mut endpoints = Vec::new();
    let mut commands = Vec::new();
    for (arm, purpose, segments) in [
        ("preflight-A", RunPurpose::SaveContinuous, 1),
        ("preflight-B", RunPurpose::SaveSplit, 2),
    ] {
        control.check("preflight_verify")?;
        let dir = root.join(arm);
        let s = read_inputs(&dir)?;
        if s.purpose != purpose || s.tape.len() != 2 || !legacy && s.source != evaluator_source() {
            return Err(bad("preflight purpose/source/budget"));
        }
        let closed = arm_commands(&dir, &s)?;
        if closed.len() != segments
            || closed
                .last()
                .is_none_or(|(_, c)| c.status != CommandStatus::Complete)
        {
            return Err(bad("preflight continuous/split finalization"));
        }
        let c = &closed.last().unwrap().1;
        commands.push(reference(
            root,
            &format!("{arm}/{}", command_locator(&c.terminal)?),
        )?);
        close_native_inner(&dir, &c.terminal.locator, control, false)?;
        let chain = lineage(&dir, &s, &c.terminal)?;
        let n = chain
            .last()
            .unwrap()
            .1
            .native
            .as_ref()
            .ok_or_else(|| bad("preflight endpoint"))?;
        let l = resolve_native(&dir, &s, n, true)?;
        if n.step != s.parent.step + 2
            || chain.iter().map(|(_, t)| t.draws.len()).sum::<usize>() != 2
        {
            return Err(bad("preflight actual optimizer count"));
        }
        println!(
            "SAVE_CAPABLE=WRITER_READER_VERIFIED PURPOSE=SAVE_RESUME_PREFLIGHT arm={arm} native={} file={} step={} model={} Adam={} counters={:?} quality_eligible=false",
            dir.join(&n.file.locator).display(),
            hex(&n.file.digest),
            n.step,
            hex(&n.model),
            hex(&n.adam.unwrap()),
            n.counters
        );
        endpoints.push((s, l));
    }
    let (a, al) = &endpoints[0];
    let (b, bl) = &endpoints[1];
    let at = al
        .manifest
        .training
        .as_ref()
        .ok_or_else(|| bad("preflight training state"))?;
    let bt = bl
        .manifest
        .training
        .as_ref()
        .ok_or_else(|| bad("preflight training state"))?;
    if a.parent.file.digest != b.parent.file.digest
        || a.parent.adam != b.parent.adam
        || al.model.weight_hash()? != bl.model.weight_hash()?
        || optimizer_hash(&al.optimizer)? != optimizer_hash(&bl.optimizer)?
        || al.tokenizer.semantic_id() != bl.tokenizer.semantic_id()
        || at != bt
        || at.train_loss.map(f64::to_bits) != bt.train_loss.map(f64::to_bits)
        || a.lr_policy != b.lr_policy
        || a.lr_offset != b.lr_offset
        || a.tape.iter().zip(&b.tape).any(|(x, y)| {
            x.indices != y.indices
                || x.sampler != y.sampler
                || x.input != y.input
                || x.target != y.target
        })
    {
        return Err(bad("SMALL save/resume preflight numeric/tape/state parity"));
    }
    if generate && root.join("preflight-proof.r3er").exists() {
        return Err(bad("preflight already certified"));
    }
    if generate
        && endpoints.iter().enumerate().any(|(i, _)| {
            root.join(if i == 0 { "preflight-A" } else { "preflight-B" })
                .join("parity.r3er")
                .exists()
        })
    {
        return Err(bad("preflight generation already attempted"));
    }
    let mut observations = Vec::new();
    for (i, (s, l)) in endpoints.iter().enumerate() {
        let dir = root.join(if i == 0 { "preflight-A" } else { "preflight-B" });
        let spec = panel(s, PanelKind::Ordinary)?;
        if generate {
            let mut rows = Vec::new();
            for &ordinal in spec.cases.iter().take(2) {
                let index = returned.len();
                let start_ref = reference(root, "preflight-start.r3er")?;
                let elapsed = control.start.elapsed().as_secs_f64();
                let row = evaluate_row_with_entry(
                    l,
                    &s.cases[ordinal as usize],
                    ordinal,
                    control,
                    false,
                    true,
                    || {
                        publish(
                            root,
                            &format!("preflight-row-{index:02}-start.r3er"),
                            &Record::VerificationRow(VerificationRow {
                                start: start_ref.clone(),
                                index: index as u32,
                                row: None,
                                elapsed: Scalar::F64(elapsed),
                            }),
                        )
                        .map(|_| ())
                    },
                )?;
                returned.push(row.clone());
                verification_fault(s.tiny_spec, "raw", control)?;
                publish(
                    root,
                    &format!("preflight-row-{index:02}.r3er"),
                    &Record::VerificationRow(VerificationRow {
                        start: start_ref,
                        index: index as u32,
                        row: Some(row.clone()),
                        elapsed: Scalar::F64(control.start.elapsed().as_secs_f64()),
                    }),
                )?;
                rows.push(row);
                verification_fault(s.tiny_spec, "row", control)?;
                control.check("preflight_returned_row_saved")?;
            }
            let e = EvalPayload {
                run: s.run,
                binding: s.binding(),
                source: s.source,
                model: unhex(&l.model.weight_hash()?)?,
                tokenizer: unhex(&l.tokenizer.semantic_id())?,
                architecture: unhex(&l.model.config.semantic_id()?)?,
                step: s.parent.step + 2,
                new_updates: 2,
                kind: PanelKind::Ordinary,
                expected: spec.cases.len() as u32,
                rows,
            };
            publish(&dir, "parity.r3er", &Record::Evaluation(e))?;
        }
        let Record::Evaluation(e) = read_record(&dir, &reference(&dir, "parity.r3er")?)? else {
            return Err(bad("preflight parity record"));
        };
        rescore(s, &e, l, false)?;
        if e.rows.len() != spec.cases.len().min(2)
            || e.rows
                .iter()
                .any(|r| !r.completed || r.interruption.is_some())
        {
            return Err(bad("preflight parity incomplete"));
        }
        observations.push(
            e.rows
                .iter()
                .map(|r| (r.tokens.clone(), r.eos, r.finish, r.error.clone()))
                .collect::<Vec<_>>(),
        );
    }
    if observations[0] != observations[1] {
        return Err(bad("preflight native generation parity"));
    }
    let pair = a.authorization.as_ref().map_or(a.parent.model, |v| v.pair);
    let source = a.source;
    let binary = if legacy {
        let origin = a
            .origins
            .iter()
            .find(|o| o.role == "execution-binary")
            .ok_or_else(|| bad("legacy binary provenance absent"))?;
        if unhex(&file_hash(Path::new(&origin.original.locator))?)? != origin.original.digest {
            return Err(bad("legacy binary changed"));
        }
        origin.original.digest
    } else {
        unhex(&file_hash(&std::env::current_exe()?)?)?
    };
    let commands: [FileRef; 2] = commands
        .try_into()
        .map_err(|_| bad("preflight command count"))?;
    let panels = vec![
        reference(root, "preflight-A/parity.r3er")?,
        reference(root, "preflight-B/parity.r3er")?,
    ];
    if generate {
        control.check("preflight_proof_publish")?;
        verification_fault(a.tiny_spec, "proof", control)?;
        publish(
            root,
            "preflight-proof.r3er",
            &Record::Preflight(PreflightReceipt {
                pair,
                source,
                binary,
                commands,
                panels,
                elapsed: Scalar::F64(control.start.elapsed().as_secs_f64()),
                generations: control.generation_calls as u64,
            }),
        )?;
        #[cfg(feature = "test-support")]
        if a.tiny_spec && std::env::var("R3ER_TEST_STOP").as_deref() == Ok("pv-proof-kill") {
            std::process::exit(93);
        }
    } else {
        let Record::Preflight(p) = read_record(root, &reference(root, "preflight-proof.r3er")?)?
        else {
            return Err(bad("preflight certificate kind"));
        };
        if p.pair != pair
            || p.source != source
            || p.binary != binary
            || p.commands != commands
            || p.panels != panels
        {
            return Err(bad("preflight certificate binding"));
        }
    }
    println!(
        "SAVE_RESUME_PREFLIGHT=PASS PROFILE={} continuous_updates=2 split_updates=2 ACTUAL_NEW_GENERATIONS={} TEACHERS={} actual_LR_bits={} stored_config_LR={} quality_eligible=false",
        al.model.config.profile,
        control.generation_calls,
        control.teacher_calls,
        if a.tiny_spec {
            at.config.lr
        } else {
            progress_lr("L", 1025)?
        }
        .to_bits(),
        at.config.lr
    );
    Ok(())
}

fn print_anchor_panel(s: &RunSnapshot, e: &EvalPayload, l: &Loaded) -> Result<()> {
    let score = rescore(s, e, l, true)?;
    println!(
        "RAW_USAGE panel={} step={} observed_tokens={} completed={}/{}",
        e.kind.name(),
        e.step,
        e.rows.iter().map(|r| r.tokens.len()).sum::<usize>(),
        score.completed,
        score.planned
    );
    let mut context = 0;
    let mut value = 0;
    let mut empty = 0;
    let mut eos = 0;
    let mut wrong_citation = 0;
    let mut strata: BTreeMap<String, [u64; 2]> = BTreeMap::new();
    for row in &e.rows {
        let case = &s.cases[row.ordinal as usize];
        let (actual, _) = row_output(row, l)?;
        if row.error.is_some() || actual.as_deref() == Some("") {
            println!(
                "RAW_ERROR_CLASS panel={} step={} ordinal={} tokens={} zero_tokens={} actual_absent={} decoded_empty={} EOS={:?} finish={:?} class={:?}",
                e.kind.name(),
                e.step,
                row.ordinal,
                row.tokens.len(),
                row.tokens.is_empty(),
                actual.is_none(),
                actual.as_deref() == Some(""),
                row.eos,
                row.finish,
                row.error_class
            );
        }
        let a = actual.as_deref().unwrap_or("");
        let exact = strict_answer_match(
            actual.as_deref(),
            &case.answer,
            row.finish == Finish::Eos,
            row.error.is_some(),
        ) && row.completed
            && row.interruption.is_none();
        empty += usize::from(a.is_empty());
        eos += usize::from(row.finish == Finish::Eos);
        wrong_citation += usize::from(citations(a).ok() != citations(&case.answer).ok());
        if let (Some(a), Some(g)) = (fields(a), fields(&case.answer)) {
            context += usize::from(a.1 == g.1);
            value += usize::from(a.2 == g.2);
        }
        for key in [
            format!("category/{}", case.category),
            format!("input-bytes/{}", case.request.input.len() / 32),
            format!(
                "family/{}",
                case.family
                    .split("/replica-")
                    .next()
                    .unwrap_or(&case.family)
                    .split("/view-")
                    .next()
                    .unwrap_or(&case.family)
            ),
        ] {
            let v = strata.entry(key).or_default();
            v[0] += u64::from(exact);
            v[1] += 1;
        }
    }
    println!(
        "NODE=G2 ARM={} PANEL={} step={} updates={} model={} full={}/{} entity={} context={context} value={value} event={} errors={} empty={empty} EOS={eos} wrong_citation={wrong_citation} base4={}/{} QA={}/{} AUX={}/{}",
        s.arm_name(),
        e.kind.name(),
        e.step,
        e.new_updates,
        hex(&e.model),
        score.exact,
        score.planned,
        score.entity,
        score.event,
        score.errors,
        score.base[0],
        score.base[1],
        score.qa[0],
        score.qa[1],
        score.aux[0],
        score.aux[1]
    );
    for (key, v) in strata {
        println!(
            "STRATUM panel={} {key} exact={}/{}",
            e.kind.name(),
            v[0],
            v[1]
        );
    }
    Ok(())
}
fn paired_panel(
    a: (&RunSnapshot, &EvalPayload, &Loaded),
    b: (&RunSnapshot, &EvalPayload, &Loaded),
) -> Result<[[u64; 4]; 3]> {
    let marks =
        |(s, e, l): (&RunSnapshot, &EvalPayload, &Loaded)| -> Result<Vec<(Hash, bool, bool)>> {
            rescore(s, e, l, true)?;
            e.rows
                .iter()
                .map(|r| {
                    let case = &s.cases[r.ordinal as usize];
                    let (actual, _) = row_output(r, l)?;
                    Ok((
                        case_hash(case),
                        r.completed
                            && r.interruption.is_none()
                            && strict_answer_match(
                                actual.as_deref(),
                                &case.answer,
                                r.finish == Finish::Eos,
                                r.error.is_some(),
                            ),
                        case.family.starts_with("copy/"),
                    ))
                })
                .collect()
        };
    let a = marks(a)?;
    let b = marks(b)?;
    paired_counts(&a, &b)
}
// Same frozen family classification as rescore; correctness never changes a denominator.
fn paired_counts(a: &[(Hash, bool, bool)], b: &[(Hash, bool, bool)]) -> Result<[[u64; 4]; 3]> {
    if a.len() != b.len() {
        return Err(bad("paired denominator"));
    }
    let mut counts = [[0; 4]; 3];
    for (a, b) in a.iter().zip(b) {
        if a.0 != b.0 || a.2 != b.2 {
            return Err(bad("paired source contents"));
        }
        let index = usize::from(a.1) * 2 + usize::from(b.1);
        counts[0][index] += 1;
        counts[if a.2 { 2 } else { 1 }][index] += 1;
    }
    Ok(counts)
}
fn anchor_report(root: &Path, control: &mut RunControl) -> Result<()> {
    let cooldown = root.join("K-KEEP").exists();
    if cooldown {
        read_preflight_outcome(root)?
            .ok_or_else(|| bad("missing parent verification"))?
            .require_current_success()?;
        if root.join("confirmation").exists() {
            read_preflight_outcome(&root.join("confirmation"))?
                .ok_or_else(|| bad("confirmation missing intent"))?
                .require_current_success()?;
        }
    }
    if root.join("preflight-A").is_dir() {
        let p = read_preflight_outcome(root)?.ok_or_else(|| bad("missing preflight attempt"))?;
        if p.legacy {
            // Report-only historical observation, never new run authorization.
            preflight_body(root, control, false, true, &mut Vec::new())?;
        } else {
            p.require_current_success()?;
        }
    }
    let arms = if cooldown {
        ["K-KEEP", "D-DECAY"]
    } else if root.join("C50-R").exists() {
        ["C50-R", "A75-R"]
    } else {
        ["C50", "A75"]
    };
    for arm in arms {
        let dir = root.join(arm);
        let s = read_inputs(&dir)?;
        if let Err(e) = arm_commands(&dir, &s) {
            println!(
                "MODEL_PAIR=FAILED_OR_INCOMPLETE ARM={arm} REASON={e} candidate=false GOAL1_ACCEPTED=false"
            );
            return Err(e);
        }
    }
    let mut incomplete = false;
    for arm in arms {
        let dir = root.join(arm);
        let s = read_inputs(&dir)?;
        let mut paths = std::fs::read_dir(&dir)?
            .filter_map(|e| e.ok())
            .filter(|e| e.file_name().to_string_lossy().starts_with("segment-"))
            .map(|e| e.path().join("terminal.r3er"))
            .collect::<Vec<_>>();
        paths.sort();
        let Some(path) = paths.last() else {
            println!("NODE=G2 ARM={arm} STATE=NOT_RUN updates=0 generations=0");
            incomplete = true;
            continue;
        };
        let name = path
            .strip_prefix(&dir)
            .map_err(|_| bad("report path"))?
            .to_string_lossy();
        let t = segment(&dir, &reference(&dir, &name)?, &s)?;
        if !t.complete || !t.stop.is_empty() || t.save_error.is_some() {
            incomplete = true;
            println!(
                "NODE=G2 ARM={arm} STATE=INCOMPLETE updates={} generations={} teachers={} elapsed_s={} native_present={} save_error={:?} STOP={:?}",
                t.updates,
                t.generations,
                t.teachers,
                t.elapsed.finite()?,
                t.native.is_some(),
                t.save_error,
                t.stop
            );
            let (input, target) = t
                .draws
                .iter()
                .fold((0, 0), |(i, t), d| (i + d.input, t + d.target));
            println!("ACTUAL_INPUT_TOKENS={input} ACTUAL_TARGET_TOKENS={target}");
            for r in &t.evaluations {
                let Record::Evaluation(e) = read_record(&dir, &r.payload)? else {
                    return Err(bad("report panel kind"));
                };
                println!(
                    "RAW_PANEL={} step={} rows={}/{} file_hash={} native_binding={} SCORE=OBSERVED_ONLY_NOT_ENDPOINT_VERIFIED",
                    e.kind.name(),
                    e.step,
                    e.rows.len(),
                    e.expected,
                    hex(&r.payload.digest),
                    r.native.is_some()
                );
            }
        }
    }
    if incomplete {
        println!("MODEL_PAIR=QUALITY_INCONCLUSIVE H3_SEAL=NOT_OPENED GOAL1_ACCEPTED=false");
        return Ok(());
    }
    let mut endpoints = Vec::new();
    for arm in arms {
        let dir = root.join(arm);
        let s = read_inputs(&dir)?;
        let a = s
            .authorization
            .as_ref()
            .ok_or_else(|| bad("pair report authorization"))?;
        let mut terminals = std::fs::read_dir(&dir)?
            .filter_map(|v| v.ok())
            .filter(|e| e.file_name().to_string_lossy().starts_with("segment-"))
            .map(|e| e.path().join("terminal.r3er"))
            .collect::<Vec<_>>();
        terminals.sort();
        let last = terminals.last().ok_or_else(|| bad("arm not executed"))?;
        let locator = last
            .strip_prefix(&dir)
            .map_err(|_| bad("terminal path"))?
            .to_string_lossy();
        let chain = lineage(&dir, &s, &reference(&dir, &locator)?)?;
        let h = verified_history(&dir, &s, &chain)?;
        let t = &chain.last().unwrap().1;
        let l = resolve_native(&dir, &s, t.native.as_ref().unwrap(), true)?;
        let native = t.native.as_ref().unwrap();
        let command_seconds = arm_commands(&dir, &s)?
            .iter()
            .map(|(_, c)| c.elapsed.finite())
            .collect::<Result<Vec<_>>>()?
            .iter()
            .sum::<f64>();
        let mut raw_tokens = BTreeMap::new();
        for er in h.evaluations.values() {
            let Record::Evaluation(e) = read_record(&dir, &er.payload)? else {
                return Err(bad("usage panel kind"));
            };
            for r in e.rows {
                if let Some(old) = raw_tokens.insert((e.step, r.ordinal), r.tokens.clone())
                    && old != r.tokens
                {
                    return Err(bad("derived watch raw differs from ordinary"));
                }
            }
        }
        println!(
            "VERIFIED_NATIVE arm={arm} file={} physical={} model={} Adam={} counters={:?} actual_final_LR_bits={:?} inherited_config_LR={} command_seconds={command_seconds} unique_generation_rows={} observed_tokens={} SOURCE=DERIVED_FROM_EXISTING_RAW",
            native.file.locator,
            hex(&native.file.digest),
            hex(&native.model),
            hex(&native.adam.ok_or_else(|| bad("report Adam absent"))?),
            native.counters,
            t.lr_bits.as_ref().and_then(|r| r.last()),
            l.manifest.training.as_ref().unwrap().config.lr,
            raw_tokens.len(),
            raw_tokens.values().map(Vec::len).sum::<usize>()
        );
        let framed = samples(
            &s.train
                .iter()
                .map(|i| s.cases[*i as usize].clone())
                .collect::<Vec<_>>(),
            &l.tokenizer,
            l.manifest.training.as_ref().unwrap().config.seq_len,
        )?;
        let mut draws = [0u64; 2];
        let mut input = [0u64; 2];
        let mut target = [0u64; 2];
        let mut unique = [BTreeSet::new(), BTreeSet::new()];
        let mut bases = [BTreeSet::new(), BTreeSet::new()];
        let anchors: BTreeSet<_> = a.pools[0].iter().copied().collect();
        for d in &s.tape[..t.updates as usize] {
            for i in &d.indices {
                let pool = usize::from(!anchors.contains(i));
                draws[pool] += 1;
                input[pool] += (framed[*i as usize].tokens.len() - 1) as u64;
                target[pool] +=
                    (framed[*i as usize].tokens.len() - framed[*i as usize].response_start) as u64;
                unique[pool].insert(*i);
                bases[pool].insert(scene(&s.cases[s.train[*i as usize] as usize]).to_string());
            }
        }
        println!(
            "NODE=G2 ARM={arm} STATE=REPORT updates={} absolute_step={} input={input:?} supervised={target:?} draws={draws:?} unique_views={:?} unique_bases={:?} elapsed_s={} generations={} teachers={} STOP={:?}",
            t.updates,
            t.native.as_ref().unwrap().step,
            unique.each_ref().map(|x| x.len()),
            bases.each_ref().map(|x| x.len()),
            chain
                .iter()
                .map(|(_, t)| t.elapsed.finite())
                .collect::<Result<Vec<_>>>()?
                .iter()
                .sum::<f64>(),
            chain.iter().map(|(_, t)| t.generations).sum::<u64>(),
            chain.iter().map(|(_, t)| t.teachers).sum::<u64>(),
            t.stop
        );
        for n in if cooldown { [128, 256] } else { [256, 512] } {
            for kind in [PanelKind::Dev, PanelKind::Cross, PanelKind::Ordinary] {
                if let Some(r) = h.evaluations.get(&(s.parent.step + n, kind)) {
                    let (e, l) = payload(&dir, &s, r)?;
                    print_anchor_panel(&s, &e, &l)?;
                }
            }
        }
        let candidate = if t.complete && t.stop.is_empty() {
            close_native_inner(&dir, &locator, control, false)?.candidate
        } else {
            false
        };
        if cooldown {
            let original = origin_path(&s, "cooldown-parent-inputs")?;
            let parent = original.parent().ok_or_else(|| bad("paired parent root"))?;
            let old = read_inputs(parent)?;
            let closed = arm_commands(parent, &old)?;
            let command = &closed
                .last()
                .ok_or_else(|| bad("paired parent terminal"))?
                .1;
            let old_h = verified_history(parent, &old, &lineage(parent, &old, &command.terminal)?)?;
            for kind in [PanelKind::Dev, PanelKind::Cross, PanelKind::Ordinary] {
                let (a, al) = payload(parent, &old, &old_h.evaluations[&(s.parent.step, kind)])?;
                let (b, bl) = payload(&dir, &s, &h.evaluations[&(s.parent.step + 256, kind)])?;
                let counts = paired_panel((&old, &a, &al), (&s, &b, &bl))?;
                println!(
                    "PAIRED parent_A75_R512_to={arm} updates=256 panel={} both_wrong/gain/loss/both_correct={:?}",
                    kind.name(),
                    counts[0]
                );
                if kind == PanelKind::Ordinary {
                    println!(
                        "PAIRED parent_A75_R512_to={arm} updates=256 QA336={:?} AUX64={:?}",
                        counts[1], counts[2]
                    );
                }
            }
        }
        endpoints.push((s, h, candidate));
    }
    if endpoints[0].0.authorization.as_ref().unwrap().pair
        != endpoints[1].0.authorization.as_ref().unwrap().pair
    {
        return Err(bad("report mismatched pair"));
    }
    for n in if cooldown { [128, 256] } else { [256, 512] } {
        for kind in [PanelKind::Dev, PanelKind::Cross, PanelKind::Ordinary] {
            let mut rows = Vec::new();
            for (i, (s, h, _)) in endpoints.iter().enumerate() {
                if let Some(r) = h.evaluations.get(&(s.parent.step + n, kind)) {
                    let (e, l) = payload(&root.join(arms[i]), s, r)?;
                    rescore(s, &e, &l, true)?;
                    rows.push(
                        e.rows
                            .iter()
                            .map(|r| {
                                let c = &s.cases[r.ordinal as usize];
                                let (a, _) = row_output(r, &l)?;
                                Ok((
                                    case_hash(c),
                                    strict_answer_match(
                                        a.as_deref(),
                                        &c.answer,
                                        r.finish == Finish::Eos,
                                        r.error.is_some(),
                                    ) && r.completed
                                        && r.interruption.is_none(),
                                    c.family.starts_with("copy/"),
                                ))
                            })
                            .collect::<Result<Vec<_>>>()?,
                    );
                }
            }
            if rows.len() == 2 {
                let counts = paired_counts(&rows[0], &rows[1])?;
                println!(
                    "PAIRED updates={n} panel={} both_wrong/gain/loss/both_correct={:?}",
                    kind.name(),
                    counts[0]
                );
                if kind == PanelKind::Ordinary {
                    println!(
                        "PAIRED updates={n} QA336={:?} AUX64={:?}",
                        counts[1], counts[2]
                    );
                }
            }
        }
    }
    println!(
        "MODEL_PAIR={} first_arm_candidate={} second_arm_candidate={} GOAL1_ACCEPTED=false H3_SEAL=NOT_OPENED",
        if endpoints.iter().any(|e| e.2) {
            "JOINT_GATE_PASS_PENDING_FRESH_PROCESS"
        } else {
            "STUDY_COMPLETE_QUALITY_FAIL"
        },
        endpoints[0].2,
        endpoints[1].2
    );
    Ok(())
}
fn run_native(root: &Path, resume: Option<&str>, control: &mut RunControl) -> Result<()> {
    let s = read_inputs(root)?;
    #[cfg(feature = "test-support")]
    if s.tiny_spec && s.origins.iter().any(|o| o.role == "test-verification") {
        let p = read_preflight_outcome(
            root.parent()
                .ok_or_else(|| bad("test verification parent"))?,
        )?
        .ok_or_else(|| bad("test verification intent absent"))?;
        p.require_current_success()?;
    }
    #[cfg(feature = "test-support")]
    if s.tiny_spec && s.origins.iter().any(|o| o.role == "test-pair") {
        let parent = root.parent().ok_or_else(|| bad("test pair parent"))?;
        for arm in ["C50", "A75"] {
            let dir = parent.join(arm);
            let other = read_inputs(&dir)?;
            if !other.tiny_spec {
                return Err(bad("test pair requires TINY"));
            }
            let closed = arm_commands(&dir, &other)?;
            if dir != root
                && closed
                    .last()
                    .is_some_and(|(_, c)| c.status != CommandStatus::Complete)
            {
                return Err(bad("test pair other arm incomplete"));
            }
        }
    }
    if s.historical || (!s.tiny_spec && s.authorization.is_none()) {
        return Err(bad(
            "this repair authorizes no SMALL optimizer updates; historical import cannot resume",
        ));
    }
    if s.authorization.is_some() {
        if !matches!(s.contract.as_str(), RESTART_CONTRACT | COOLDOWN_CONTRACT) {
            return Err(bad("closed historical study is not restart authorization"));
        }
        if s.source != evaluator_source()
            || s.origins
                .iter()
                .find(|o| o.role == "execution-binary")
                .is_none_or(|o| {
                    file_hash(&std::env::current_exe().unwrap_or_default())
                        .ok()
                        .as_deref()
                        != Some(hex(&o.original.digest).as_str())
                })
        {
            return Err(bad("registered source/binary changed before training"));
        }
        let (_, generations, seconds) = anchor_budget(root, &s)?;
        if s.cooldown() {
            let registration = s
                .origins
                .iter()
                .find(|o| o.role == "cooldown-registered-root")
                .ok_or_else(|| bad("study root binding"))?;
            if root
                .parent()
                .ok_or_else(|| bad("study root"))?
                .canonicalize()?
                .to_string_lossy()
                != registration.original.locator
            {
                return Err(bad("study moved outside registered authorization"));
            }
            for role in [
                "cooldown-parent-inputs",
                "cooldown-parent-terminal",
                "cooldown-parent-command",
                "cooldown-legacy-proof",
            ] {
                origin_path(&s, role)?;
            }
        }
        control.deadline = control.start + Duration::from_secs_f64((7200. - seconds).min(1800.));
        control.generation_limit = if s.cooldown() { 4096 } else { 7500 } - generations;
        println!(
            "NODE=G2 ARM={} STATE=START prior_generations={generations} prior_command_s={seconds:.3} source={} binary={} input_binding={}",
            s.arm_name(),
            hex(&evaluator_source()),
            file_hash(&std::env::current_exe()?)?,
            hex(&s.binding())
        );
        for origin in s
            .origins
            .iter()
            .filter(|o| o.role.starts_with("replacement-"))
        {
            if unhex(&file_hash(Path::new(&origin.original.locator))?)? != origin.original.digest {
                return Err(bad("replacement source changed"));
            }
        }
        if !s.preflight() && !s.cooldown() {
            verify_save_preflight(
                root.parent().ok_or_else(|| bad("preflight root"))?,
                control,
                false,
            )?;
        }
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
        if effective_outcome(root, &s, &r)?.status != CommandStatus::TimePause {
            return Err(bad("resume requires positive time-pause finalization"));
        }
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
    if !s.historical {
        fork_budget(
            &mut state,
            s.parent.step,
            s.parent.counters[0],
            s.tape.len(),
            if s.cooldown() { 1_000_000 } else { 2_000_000 },
        )?;
        let mut manifest = l.manifest.clone();
        manifest.training = Some(state.clone());
        checkpoint::validate_metadata(&manifest, &l.tokenizer)?;
    }
    let mut adam = Adam {
        moments: std::mem::take(&mut l.optimizer),
    };
    let start = state.step as u64 - s.parent.step;
    let name = format!("segment-{index:02}");
    std::fs::create_dir(root.join(&name))?;
    let mut evaluations = Vec::new();
    let mut decisions = Vec::new();
    let mut draws = Vec::new();
    let mut lr_bits = s.cooldown().then(Vec::new);
    let mut complete = false;
    let mut heartbeat = Instant::now();
    let mut last_saved = if resume.is_some() {
        Some(initial.clone())
    } else {
        None
    };
    let outcome = (|| -> Result<()> {
        let episodes: Vec<_> = s
            .train
            .iter()
            .map(|i| s.cases[*i as usize].clone())
            .collect();
        let framed = samples(&episodes, &l.tokenizer, state.config.seq_len)?;
        loop {
            let n = state.step as u64 - s.parent.step;
            if matches!(s.purpose, RunPurpose::SaveSplit | RunPurpose::LrSplit)
                && index == 0
                && n == 1
            {
                println!(
                    "PURPOSE={:?} explicit_one_update_segment_deadline=true budgeted_update=true",
                    s.purpose
                );
                control.deadline = Instant::now();
                control.check("preflight_split_save_boundary")?;
            }
            l.model.refresh_identity()?;
            if s.eval_steps.contains(&(n as u32)) {
                if s.authorization.is_some() {
                    let native = if let Some(saved) =
                        last_saved.as_ref().filter(|r| r.step == state.step as u64)
                    {
                        reuse_native(root, &s, saved, &l, &state, &adam, index)?
                    } else {
                        save_native(
                            root,
                            &format!("{name}/step-{n:04}.r3m"),
                            &s,
                            &mut l,
                            &state,
                            &adam,
                            index,
                            "RECOVERY_SCREENING",
                        )?
                    };
                    println!(
                        "LAST_DURABLE_NATIVE={} step={} file_hash={} STATE=SAVED_BEFORE_EVALUATION",
                        native.file.locator,
                        native.step,
                        hex(&native.file.digest)
                    );
                    last_saved = Some(native);
                }
                let kinds: &[PanelKind] = if s.cooldown() && n == 128 {
                    &[PanelKind::Dev, PanelKind::Watch]
                } else if s.authorization.is_some() {
                    &[
                        PanelKind::Dev,
                        PanelKind::Cross,
                        PanelKind::Ordinary,
                        PanelKind::Watch,
                    ]
                } else {
                    &[PanelKind::Dev, PanelKind::Watch]
                };
                for &kind in kinds {
                    let previous = h
                        .evaluations
                        .get(&(state.step as u64, kind))
                        .map(|r| payload(root, &s, r).map(|v| v.0))
                        .transpose()?;
                    let done = previous.as_ref().is_some_and(|e| {
                        e.rows.len() == e.expected as usize
                            && e.rows
                                .iter()
                                .all(|r| r.completed && r.interruption.is_none())
                    });
                    if !done {
                        control.check("binary_before_evaluation")?;
                        let prefix = previous
                            .map(|e| {
                                e.rows
                                    .into_iter()
                                    .take_while(|r| r.completed && r.interruption.is_none())
                                    .collect()
                            })
                            .unwrap_or_default();
                        #[cfg(feature = "test-support")]
                        if s.tiny_spec
                            && kind == PanelKind::Dev
                            && n == 1
                            && std::env::var("R3ER_TEST_STOP").as_deref() == Ok("partial-dev")
                        {
                            control.deadline = Instant::now();
                        }
                        let e = if kind == PanelKind::Watch
                            && s.authorization.is_some()
                            && !(s.cooldown() && n == 128)
                        {
                            let r =
                                &h.evaluations[&(state.step as u64, PanelKind::Ordinary)].payload;
                            let Record::Evaluation(mut e) = read_record(root, r)? else {
                                return Err(bad("ordinary watch source"));
                            };
                            let spec = panel(&s, kind)?;
                            e.rows = spec
                                .cases
                                .iter()
                                .map(|i| {
                                    e.rows.iter().find(|r| r.ordinal == *i).cloned().ok_or_else(
                                        || bad("incomplete ordinary cannot derive watch"),
                                    )
                                })
                                .collect::<Result<_>>()?;
                            e.kind = kind;
                            e.expected = spec.cases.len() as u32;
                            e
                        } else {
                            evaluate(
                                &s,
                                &l,
                                kind,
                                state.step as u64,
                                control,
                                s.tiny_spec,
                                prefix,
                            )?
                        };
                        if s.authorization.is_some()
                            && e.rows.len() == e.expected as usize
                            && e.rows
                                .iter()
                                .all(|r| r.completed && r.interruption.is_none())
                        {
                            print_anchor_panel(&s, &e, &l)?;
                        }
                        let r = publish(
                            root,
                            &format!("{name}/{}-{n:04}.r3er", kind.name()),
                            &Record::Evaluation(e),
                        )?;
                        let er = EvaluationRef {
                            payload: r,
                            native: if s.authorization.is_some() {
                                last_saved.clone()
                            } else {
                                None
                            },
                        };
                        h.evaluations.insert((state.step as u64, kind), er.clone());
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
                let native = if let Some(native) = h
                    .evaluations
                    .get(&(state.step as u64, PanelKind::Dev))
                    .and_then(|e| e.native.as_ref())
                    .or_else(|| last_saved.as_ref().filter(|r| r.step == state.step as u64))
                {
                    reuse_native(root, &s, native, &l, &state, &adam, index)?
                } else {
                    save_native(
                        root,
                        &format!("{name}/step-{n:04}.r3m"),
                        &s,
                        &mut l,
                        &state,
                        &adam,
                        index,
                        "RECOVERY_SCREENING",
                    )?
                };
                for &kind in kinds {
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
                last_saved = Some(native);
                fault(&s, "checkpoint", n, control);
                control.check("binary_checkpoint_recorded")?;
            }
            if n == s.tape.len() as u64 && s.preflight() {
                complete = true;
                break;
            }
            if n == s.tape.len() as u64 {
                for kind in [PanelKind::Cross, PanelKind::Ordinary] {
                    if h.evaluations.contains_key(&(state.step as u64, kind)) {
                        continue;
                    }
                    control.check("binary_final_panel")?;
                    let e = evaluate(&s, &l, kind, state.step as u64, control, true, Vec::new())?;
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
            let rate = if s.cooldown() {
                cooldown_lr(s.lr_policy, n + 1)?
            } else if s.tiny_spec {
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
            if let Some(rates) = &mut lr_bits {
                rates.push(rate.to_bits());
            }
            println!(
                "ACTUAL_{}_UPDATE={} MODEL_STEP={} INPUT_TOKENS={} TARGET_TOKENS={} LOSS={ce} LR_BITS={}",
                if s.tiny_spec { "TINY" } else { "SMALL" },
                draws.len(),
                state.step,
                d.input,
                d.target,
                rate.to_bits()
            );
            if heartbeat.elapsed() >= Duration::from_secs(15) {
                let n = state.step as u64 - s.parent.step;
                println!(
                    "NODE=D3 ARM={} STATE=TRAINING completed_updates={n}/{} optimizer_absolute_step={} input_tokens={} target_tokens={} anchor_draws={} focus_draws={} last_complete_eval={:?} quality_gate=NOT_EVALUATED elapsed_s={:.3} rss_kib={:?} last_stop={:?} last_saved_step={:?}",
                    s.arm_name(),
                    s.tape.len(),
                    state.step,
                    state.consumed_tokens - s.parent.counters[0],
                    state.target_tokens - s.parent.counters[1],
                    n * u64::from(anchors_for(&s)),
                    n * (8 - u64::from(anchors_for(&s))),
                    h.decisions.keys().next_back(),
                    control.start.elapsed().as_secs_f64(),
                    control.last_rss_kib,
                    control.stop,
                    last_saved.as_ref().map(|r| r.step)
                );
                heartbeat = Instant::now();
            }
        }
        Ok(())
    })();
    if let Err(e) = &outcome {
        control.classify_error(e);
    }
    #[cfg(feature = "test-support")]
    let crash_after_cancelled_terminal = s.tiny_spec
        && complete
        && std::env::var("R3ER_TEST_STOP").as_deref() == Ok("stored-cleanup-cancel");
    #[cfg(feature = "test-support")]
    if crash_after_cancelled_terminal {
        control.cancel.store(true, Ordering::Relaxed);
    }
    let _ = control.check("binary_before_preservation");
    let cleanup = Instant::now();
    let reason = control
        .stop
        .map_or("SCREENING_BUDGET_REACHED", StopReason::name);
    let saved = (|| {
        if let Some(native) = last_saved
            .as_ref()
            .filter(|r| r.step == state.step as u64)
            .or_else(|| {
                h.evaluations
                    .get(&(state.step as u64, PanelKind::Dev))
                    .and_then(|e| e.native.as_ref())
            })
        {
            let reference = reuse_native(root, &s, native, &l, &state, &adam, index)?;
            println!(
                "NATIVE_FINAL_REUSED={} BYTES_WRITTEN=0 terminal_stop={reason}",
                reference.file.locator
            );
            Ok(reference)
        } else {
            save_native(
                root,
                &format!("{name}/final.r3m"),
                &s,
                &mut l,
                &state,
                &adam,
                index,
                reason,
            )
        }
    })();
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
    if !(matches!(s.contract.as_str(), RESTART_CONTRACT | COOLDOWN_CONTRACT) && control.observed == [StopReason::TimeBudget]) && evaluations.iter().any(|r| read_record(root,&r.payload).is_ok_and(|v| matches!(v,
        Record::Evaluation(e) if e.rows.len()!=e.expected as usize || e.rows.iter().any(|r|!r.completed || r.interruption.is_some())))) {
        control.observe(StopReason::AuditIncomplete);
    }
    if cleanup.elapsed() > Duration::from_secs(120) {
        control.observe(StopReason::ResourceLimit);
    }
    let _ = control.check("binary_terminal");
    let mut t = SegmentReceipt {
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
        lr_bits,
    };
    if t.complete
        && t.stop.is_empty()
        && t.save_error.is_none()
        && s.authorization.is_some()
        && !s.preflight()
    {
        let scores = t
            .evaluations
            .iter()
            .filter_map(|r| match read_record(root, &r.payload) {
                Ok(Record::Evaluation(e)) if e.step == state.step as u64 => {
                    Some(rescore(&s, &e, &l, true).map(|v| (e.kind, v)))
                }
                _ => None,
            })
            .collect::<Result<Vec<_>>>()?;
        t.candidate = eligible(&s, &scores, h.quality, &t.stop);
    }
    publish(
        root,
        &format!("{name}/terminal.r3er"),
        &Record::Segment(t.clone()),
    )?;
    println!(
        "SEGMENT={index} NEW_TINY_UPDATES={} NEW_SMALL_UPDATES={} GENERATIONS={} TEACHERS={} STOP={:?} resume={} complete={} CANONICAL_JSON_WRITES=0 LEGACY_JSON_READS=0",
        if s.tiny_spec { t.updates - start } else { 0 },
        if s.tiny_spec { 0 } else { t.updates - start },
        t.generations,
        t.teachers,
        t.stop,
        t.resume,
        t.complete
    );
    #[cfg(feature = "test-support")]
    if crash_after_cancelled_terminal {
        std::process::exit(91);
    }
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
        authorization: None,
        purpose: RunPurpose::Anchor,
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
        lr_bits: None,
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
        lr_bits: None,
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
        authorization: None,
        purpose: RunPurpose::Anchor,
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
fn fixture_fork_inner(
    from: &Path,
    output: &Path,
    untrained: bool,
    pair: bool,
    purpose: RunPurpose,
    restart: bool,
) -> Result<()> {
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
    s.purpose = purpose;
    if restart {
        s.contract = RESTART_CONTRACT.into();
        s.source = evaluator_source();
    }
    if s.preflight() {
        s.eval_steps.clear();
    }
    if pair {
        s.origins.push(Origin {
            role: "test-pair".into(),
            original: reference(from, "inputs.r3er")?,
        });
    }
    publish(output, "inputs.r3er", &Record::Inputs(Box::new(s)))?;
    Ok(())
}
#[cfg(feature = "test-support")]
#[allow(clippy::type_complexity)] // Test-only comparison of existing typed state, not a new runtime abstraction.
fn fixture_check(roots: &[PathBuf]) -> Result<()> {
    let mut expected_lr_state = None;
    let mut expected: Option<(Hash, Hash, [u64; 3], Vec<(PanelKind, Vec<u32>)>, [u64; 3])> = None;
    for root in roots {
        let s = read_inputs(root)?;
        let Record::Comparison(c) = read_record(root, &reference(root, "comparison.r3er")?)? else {
            return Err(bad("fixture comparison"));
        };
        let chain = lineage(root, &s, &c.terminal)?;
        let h = verified_history(root, &s, &chain)?;
        if h.decisions.len() != if s.preflight() { 0 } else { 2 } || c.candidate || c.historical {
            return Err(bad("fixture decisions/eligibility"));
        }
        let n = chain.last().unwrap().1.native.as_ref().unwrap();
        if s.cooldown() {
            let l = resolve_native(root, &s, n, true)?;
            let rates = chain
                .iter()
                .flat_map(|(_, t)| t.lr_bits.clone().unwrap_or_default())
                .collect::<Vec<_>>();
            if rates.len() != 2 {
                return Err(bad("TINY numeric actual LR count"));
            }
            let current = (l.manifest.training.clone(), rates);
            if expected_lr_state
                .as_ref()
                .is_some_and(|old| *old != current)
            {
                return Err(bad("continuous/split full state/LR mismatch"));
            }
            expected_lr_state = Some(current);
        }
        let mut raw = Vec::new();
        for kind in if s.preflight() {
            vec![]
        } else {
            vec![
                PanelKind::Dev,
                PanelKind::Watch,
                PanelKind::Cross,
                PanelKind::Ordinary,
            ]
        } {
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
    #[cfg(feature = "test-support")]
    #[test]
    fn anchor_fork_exhausted_parent_budget_native_save() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("fork");
        fixture(&root).unwrap();
        let s = read_inputs(&root).unwrap();
        let mut l = resolve_native(&root, &s, &s.parent, true).unwrap();
        let mut state = l.manifest.training.clone().unwrap();
        state.step = state.config.max_steps;
        let parent_step = state.step as u64;
        let parent = state.clone();
        state.step += 2; // Metadata fixture only: no forward/backward/optimizer calls.
        let adam = Adam {
            moments: std::mem::take(&mut l.optimizer),
        };
        assert!(
            save_arm(
                &mut l,
                &state,
                &adam,
                &root.join("rejected.r3m"),
                "RECOVERY_SCREENING"
            )
            .is_err()
        );
        assert!(!root.join("rejected.r3m").exists());
        fork_budget(&mut state, parent_step, parent.consumed_tokens, 2, 1000).unwrap();
        let mut unchanged = state.config.clone();
        unchanged.max_steps = parent.config.max_steps;
        unchanged.max_tokens = parent.config.max_tokens;
        unchanged.budget_start_step = parent.config.budget_start_step;
        unchanged.budget_start_tokens = parent.config.budget_start_tokens;
        assert_eq!(unchanged, parent.config);
        save_arm(
            &mut l,
            &state,
            &adam,
            &root.join("accepted.r3m"),
            "RECOVERY_SCREENING",
        )
        .unwrap();
        let saved = checkpoint::load(&root.join("accepted.r3m"), Device::Cpu, true).unwrap();
        assert_eq!(saved.manifest.training.as_ref().unwrap(), &state);
        assert_eq!(
            optimizer_hash(&saved.optimizer).unwrap(),
            optimizer_hash(&adam.moments).unwrap()
        );
        assert_eq!(
            saved.model.weight_hash().unwrap(),
            l.model.weight_hash().unwrap()
        );
        println!(
            "EXHAUSTED_PARENT_SAVE RED_REPRODUCED GREEN_NATIVE_RELOAD SMALL_UPDATES=0 TINY_UPDATES=0 GENERATIONS=0"
        );
    }
    #[test]
    fn anchor_ratio_master_streams_and_supervised_denominators() {
        let pools = [(0..16).collect::<Vec<_>>(), (16..24).collect()];
        let framed = (0..24)
            .map(|i| Sample {
                tokens: vec![3; 8 + i],
                response_start: 4 + i / 2,
                curriculum: false,
            })
            .collect::<Vec<_>>();
        let (c, ct) = anchor_tape(&pools, 4, 31, &framed).unwrap();
        let (a, at) = anchor_tape(&pools, 6, 31, &framed).unwrap();
        let (again, _) = anchor_tape(&pools, 6, 31, &framed).unwrap();
        for (t, n) in [(&c, 4), (&a, 6)] {
            assert_eq!(t.len(), 512);
            for (step, d) in t.iter().enumerate() {
                assert_eq!(d.indices.len(), 8);
                assert_eq!(d.indices.iter().filter(|i| **i < 16).count(), n);
                assert_eq!(d.sampler, 32 + step as u64);
                assert_eq!(
                    d.input,
                    d.indices.iter().map(|i| 7 + u64::from(*i)).sum::<u64>()
                );
                assert_eq!(
                    d.target,
                    d.indices
                        .iter()
                        .map(|i| 4 + u64::from(*i) - u64::from(*i) / 2)
                        .sum::<u64>()
                );
            }
        }
        for pool in 0..2 {
            let take = |t: &[Draw]| {
                t.iter()
                    .flat_map(|d| d.indices.iter())
                    .filter(|i| usize::from(**i >= 16) == pool)
                    .copied()
                    .collect::<Vec<_>>()
            };
            let x = take(&c);
            let y = take(&a);
            let n = x.len().min(y.len());
            assert_eq!(x[..n], y[..n]);
        }
        assert_eq!(
            c.iter().map(|d| d.target).sum::<u64>(),
            ct.iter().sum::<u64>()
        );
        assert_eq!(
            a.iter().map(|d| d.target).sum::<u64>(),
            at.iter().sum::<u64>()
        );
        assert_ne!(ct, at);
        assert!(
            a.iter()
                .zip(&again)
                .all(|(a, b)| a.indices == b.indices && a.sampler == b.sampler)
        );
        let auth = AnchorAuthorization {
            pair: hash(b"pair"),
            baseline: hash(b"baseline"),
            expected_parent: hash(b"parent"),
            anchors: 6,
            pools,
        };
        let mut bytes = Vec::new();
        auth.encode(&mut bytes);
        let mut reader = Reader::new(&bytes);
        let restored = AnchorAuthorization::decode(&mut reader).unwrap();
        assert!(reader.finished());
        assert_eq!(restored.pools, auth.pools);
        assert_eq!(restored.anchors, 6);
        println!("ANCHOR_RATIO_REGRESSION SMALL_UPDATES=0 TINY_UPDATES=0 GENERATIONS=0");
    }
    #[test]
    fn cooldown_scalar_fixed_horizon_and_bits() {
        let midpoint = 0.000055 + 0.000045 * (std::f64::consts::PI / 510.).sin();
        for (n, expected) in [(1, 0.0001), (128, midpoint), (256, 0.00001)] {
            assert!((cooldown_lr(3, n).unwrap() - expected).abs() < 2e-20);
            assert_eq!(cooldown_lr(2, n).unwrap().to_bits(), 1e-4f64.to_bits());
        }
        assert!(cooldown_lr(3, 0).is_err());
        assert!(cooldown_lr(3, 257).is_err());
        assert!(cooldown_lr(1, 1).is_err());
        assert!(cooldown_lr(3, 2).unwrap() > cooldown_lr(3, 128).unwrap());
        println!("LR_FORMULA_OBSERVATIONS=6 SCALAR_OPTIMIZER_CALLS=0 TINY_UPDATES=0");
    }
    #[test]
    fn paired_qa_aux_denominators_are_frozen_and_separate() {
        let a = vec![
            ([0; 32], false, false),
            ([1; 32], true, false),
            ([2; 32], false, true),
            ([3; 32], true, true),
        ];
        let b = vec![
            ([0; 32], true, false),
            ([1; 32], false, false),
            ([2; 32], false, true),
            ([3; 32], true, true),
        ];
        assert_eq!(
            paired_counts(&a, &b).unwrap(),
            [[1, 1, 1, 1], [0, 1, 1, 0], [1, 0, 0, 1]]
        );
        assert!(paired_counts(&a, &b[..3]).is_err());
        let mut changed = b.clone();
        changed[0].2 = true;
        assert!(paired_counts(&a, &changed).is_err());
        changed = b;
        changed[0].0 = [9; 32];
        assert!(paired_counts(&a, &changed).is_err());
    }
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
        check_close_modes(0..10);
    }
    #[test]
    fn final_native_model_must_match_all_panels() {
        check_close_modes([10, 11, 12, 13, 18]);
    }
    #[test]
    fn stored_terminal_stop_blocks_fresh_close_without_sidecar() {
        check_close_modes(14..18);
    }
    fn check_close_modes(modes: impl IntoIterator<Item = usize>) {
        let directory = tempfile::tempdir().unwrap();
        let seed = directory.path().join("seed");
        fixture(&seed).unwrap();
        let mut count = 0;
        for mode in modes {
            count += 1;
            let root = directory.path().join(format!("case-{mode}"));
            std::fs::create_dir(&root).unwrap();
            std::fs::copy(seed.join("parent.r3m"), root.join("parent.r3m")).unwrap();
            let mut s = read_inputs(&seed).unwrap();
            s.tape.clear();
            s.eval_steps = vec![0];
            publish(&root, "inputs.r3er", &Record::Inputs(Box::new(s.clone()))).unwrap();
            let l = resolve_native(&root, &s, &s.parent, true).unwrap();
            let alternate = if (10..14).contains(&mode) || mode == 18 {
                let mut other =
                    checkpoint::load(&root.join("parent.r3m"), Device::Cpu, true).unwrap();
                if mode != 13 && mode != 18 {
                    let var = other.model.vars.values().next().unwrap();
                    let mut values = var.flatten_all().unwrap().to_vec1::<f32>().unwrap();
                    values[0] += 0.25;
                    var.set(&Tensor::from_vec(values, var.dims(), &Device::Cpu).unwrap())
                        .unwrap();
                }
                let state = other.manifest.training.clone().unwrap();
                let adam = Adam {
                    moments: std::mem::take(&mut other.optimizer),
                };
                if mode == 18 {
                    neural::artifact::export_inference(&root.join("alternate.r3m"), &other)
                        .unwrap();
                } else {
                    save_arm(
                        &mut other,
                        &state,
                        &adam,
                        &root.join("alternate.r3m"),
                        "SCREENING_BUDGET_REACHED",
                    )
                    .unwrap();
                }
                let loaded =
                    checkpoint::load(&root.join("alternate.r3m"), Device::Cpu, mode != 18).unwrap();
                let n = native_reference(&root, "alternate.r3m", &s, &loaded, 0).unwrap();
                assert_ne!(n.file.digest, s.parent.file.digest);
                assert_eq!(n.step, s.parent.step);
                assert_eq!(n.tokenizer, s.parent.tokenizer);
                assert_eq!(n.architecture, s.parent.architecture);
                assert_eq!(n.model == s.parent.model, mode == 13 || mode == 18);
                Some((n, loaded))
            } else {
                None
            };
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
                let use_alternate = mode == 13
                    || mode == 18
                    || mode == 10 && kind == PanelKind::Cross
                    || mode == 11 && kind == PanelKind::Ordinary
                    || mode == 12 && matches!(kind, PanelKind::Dev | PanelKind::Watch);
                let native = if use_alternate {
                    let (n, _) = alternate.as_ref().unwrap();
                    payload.model = n.model;
                    n.clone()
                } else {
                    s.parent.clone()
                };
                let reference = publish(
                    &root,
                    &format!("{}.r3er", kind.name()),
                    &Record::Evaluation(payload.clone()),
                )
                .unwrap();
                refs.push(EvaluationRef {
                    payload: reference,
                    native: Some(native),
                });
                // The alternate file and its panel are valid on their own.
                if (10..14).contains(&mode) || mode == 18 {
                    super::payload(&root, &s, refs.last().unwrap()).unwrap();
                }
                payloads.push(payload);
            }
            let dr = Record::identity(&std::fs::read(root.join("dev.r3er")).unwrap()).unwrap();
            let wr = Record::identity(&std::fs::read(root.join("watch.r3er")).unwrap()).unwrap();
            let decision_model = if mode == 12 {
                &alternate.as_ref().unwrap().1
            } else {
                &l
            };
            let mut d = decision_for(
                &s,
                &payloads[0],
                &payloads[1],
                [0; 3],
                dr,
                wr,
                decision_model,
            )
            .unwrap();
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
                stop: match mode {
                    14 => vec![StopReason::Cancelled],
                    15 => vec![StopReason::QualityGuard],
                    16 => vec![StopReason::IntegrityFail],
                    17 => vec![StopReason::AuditIncomplete],
                    _ => vec![],
                },
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
                lr_bits: None,
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
            if mode <= 1 || mode == 13 || mode == 18 {
                let comparison = result.unwrap();
                assert!(!comparison.candidate);
                assert!(
                    comparison
                        .panels
                        .iter()
                        .all(|(_, p)| p.exact == u64::from(mode != 1))
                );
            } else {
                assert!(result.is_err(), "mode{mode}");
                if (10..13).contains(&mode) {
                    assert!(
                        result
                            .unwrap_err()
                            .to_string()
                            .contains("final panel model")
                    );
                }
                assert!(!root.join("comparison.r3er").exists());
            }
        }
        println!("CONSTRUCTED_CLOSE_FIXTURES={count} ACTUAL_OPTIMIZER_UPDATES=0 GENERATIONS=0");
    }
}
