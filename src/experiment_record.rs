//! Training-only, schema-specific immutable experiment records. Native control never parses JSON.
use super::*;
use replica_v3::codec::{Reader, publish_new, put_bytes, put_varint};
#[cfg(feature = "test-support")]
use replica_v3::event::GenerationLimits;
use sha2::{Digest as _, Sha256};
use std::io::{Read, Seek, SeekFrom};
#[path = "token_cache.rs"]
mod token_cache;

type Hash = [u8; 32];
const HEADER: usize = 48;
const MAX_FILE: usize = 128 * 1024 * 1024;
const MAX_CASES: usize = 16384;
const MAX_ROWS: usize = 512;
const MAX_TOKENS: usize = 2048;
const MAX_SEGMENT_UPDATES: usize = 512;
const MAX_TEXT: usize = 262144;
const CONTRACT: &str = "R3-BINARY-EVAL-RESUME-1.0";
const ANCHOR_CONTRACT: &str = "R3-NATIVE-STORAGE-QUALITY-1.0";
const RESTART_CONTRACT: &str = "R3-DURABILITY-PAIR-RESTART-1.0";
const COOLDOWN_CONTRACT: &str = "R3-PREFLIGHT-ONCE-AND-COOLDOWN-1.0";
const OBJECTIVE_CONTRACT: &str = "R3-DATA-BINARY-AND-TARGET-LOSS-1.0";
const NATIVE_CORPUS_CONTRACT: &str = "R3-NATIVE-CORPUS-OBJECTIVE-BINDING-1.0";
const BRIDGE_CONTRACT: &str = "R3-QUALITY-FIRST-BRIDGE-1.0";
const BRIDGE_RESTART_CONTRACT: &str = "R3-BRIDGE-EVIDENCE-RESTART-1.0";
const BOUNDED_BRIDGE_CONTRACT: &str = "R3-QUALITY-RECOVERY-BOUNDED-BRIDGE-1.0";

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
#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
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
    LegacyDev,
    Conditional,
    Sanity,
    Screen,
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
            Self::LegacyDev => 6,
            Self::Conditional => 7,
            Self::Sanity => 8,
            Self::Screen => 9,
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
            6 => Ok(Self::LegacyDev),
            7 => Ok(Self::Conditional),
            8 => Ok(Self::Sanity),
            9 => Ok(Self::Screen),
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
            Self::LegacyDev => "legacy-dev",
            Self::Conditional => "conditional",
            Self::Sanity => "sanity",
            Self::Screen => "screen",
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
    objective: Option<ObjectivePolicy>,
}
#[derive(Clone, Debug, Serialize)]
struct ObjectivePolicy {
    span: bool,
    annotation: Hash,
    probes: Vec<u32>,
}
impl ObjectivePolicy {
    fn encode(&self, b: &mut Vec<u8>) {
        b.push(1); // Explicit objective revision; equation/tokenizer identity remains unchanged.
        b.push(u8::from(self.span));
        b.extend(self.annotation);
        integers(b, &self.probes);
    }
    fn decode(r: &mut Reader<'_>) -> Result<Self> {
        if r.byte()? != 1 {
            return Err(bad("objective revision"));
        }
        Ok(Self {
            span: r.bool()?,
            annotation: digest_read(r)?,
            probes: integers_read(r, 64)?,
        })
    }
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

use crate::data::native::{episode_decode, episode_encode};
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
        if matches!(
            self.contract.as_str(),
            RESTART_CONTRACT
                | COOLDOWN_CONTRACT
                | OBJECTIVE_CONTRACT
                | NATIVE_CORPUS_CONTRACT
                | BRIDGE_CONTRACT
                | BOUNDED_BRIDGE_CONTRACT
        ) {
            b.push(self.purpose.tag());
        }
        if let Some(objective) = &self.objective {
            objective.encode(b);
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
        if matches!(
            self.contract.as_str(),
            RESTART_CONTRACT
                | COOLDOWN_CONTRACT
                | OBJECTIVE_CONTRACT
                | NATIVE_CORPUS_CONTRACT
                | BRIDGE_CONTRACT
                | BOUNDED_BRIDGE_CONTRACT
        ) {
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
        let n = count(r, 8)?;
        let panels = (0..n)
            .map(|_| PanelSpec::decode(r))
            .collect::<Result<Vec<_>>>()?;
        let n = count(r, MAX_SEGMENT_UPDATES)?;
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
        let purpose = if matches!(
            contract.as_str(),
            RESTART_CONTRACT
                | COOLDOWN_CONTRACT
                | OBJECTIVE_CONTRACT
                | NATIVE_CORPUS_CONTRACT
                | BRIDGE_CONTRACT
                | BOUNDED_BRIDGE_CONTRACT
        ) {
            RunPurpose::read(r)?
        } else {
            RunPurpose::Anchor
        };
        let s = Self {
            objective: if contract == OBJECTIVE_CONTRACT {
                Some(ObjectivePolicy::decode(r)?)
            } else {
                None
            },
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
        ) && (!matches!(
            self.contract.as_str(),
            RESTART_CONTRACT | NATIVE_CORPUS_CONTRACT
        ) || self.tape.len() != 2
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
            != if self.screen() {
                BOUNDED_BRIDGE_CONTRACT
            } else if self.bridge() {
                BRIDGE_CONTRACT
            } else if self.path_parity() {
                NATIVE_CORPUS_CONTRACT
            } else if self.objective.is_some() {
                OBJECTIVE_CONTRACT
            } else if self.cooldown() {
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
            || self.lr_policy
                > if self.continuation() || self.path_parity() {
                    3
                } else {
                    1
                }
            || self.panels.len()
                != if self.screen() {
                    8
                } else if self.bridge() {
                    7
                } else {
                    4
                }
            || self
                .panels
                .iter()
                .map(|p| p.kind)
                .collect::<BTreeSet<_>>()
                .len()
                != if self.screen() {
                    8
                } else if self.bridge() {
                    7
                } else {
                    4
                }
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
        if self.bridge()
            && (self.lr_policy != 2
                || self.lr_offset != 0
                || self.objective.is_some()
                || !self.historical
                    && (self.tape.len() != if self.tiny_spec { 2 } else { 512 }
                        || self.eval_steps
                            != if self.tiny_spec {
                                vec![1, 2]
                            } else if self.screen() {
                                vec![32, 64, 128, 256, 512]
                            } else {
                                vec![256, 512]
                            }))
        {
            return Err(bad("bridge fixed default objective/LR/horizon"));
        }
        if self.cooldown()
            && (self.historical
                || !matches!(self.lr_policy, 2 | 3)
                || self.lr_offset != 0
                || !matches!(self.purpose, RunPurpose::LrContinuous | RunPurpose::LrSplit)
                || self.tape.len() != if self.tiny_spec { 2 } else { 256 }
                || if self.tiny_spec {
                    !self.eval_steps.is_empty() && self.eval_steps != [1, 2]
                } else {
                    self.eval_steps != [128, 256]
                })
        {
            return Err(bad("cooldown fixed horizon/policy/purpose"));
        }
        if let Some(o) = &self.objective
            && (self.historical
                || self.lr_policy != 2
                || self.lr_offset != 0
                || self.purpose != RunPurpose::LrContinuous
                || self.tape.len() != if self.tiny_spec { 2 } else { 512 }
                || self.eval_steps
                    != if self.tiny_spec {
                        vec![1, 2]
                    } else {
                        vec![256, 512]
                    }
                || o.probes.len() != if self.tiny_spec { 1 } else { 64 }
                || o.probes.iter().any(|i| !self.train.contains(i))
                || o.probes.iter().collect::<BTreeSet<_>>().len() != o.probes.len())
        {
            return Err(bad("objective fixed policy/probe/horizon"));
        }
        for p in &self.panels {
            let expected = match p.kind {
                PanelKind::Dev => 256,
                PanelKind::Watch => 32,
                PanelKind::Cross => 512,
                PanelKind::Ordinary => 400,
                PanelKind::DevParity | PanelKind::OrdinaryParity => 16,
                PanelKind::LegacyDev => 256,
                PanelKind::Conditional => 144,
                PanelKind::Sanity => 64,
                PanelKind::Screen => 224,
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
                || self.tiny_spec && !self.bridge()
                || !matches!(a.anchors, 4 | 6)
                || !self.tiny_spec
                    && self.parent.step != if self.continuation() { 24310 } else { 23798 }
                || self.parent.adam.is_none()
                || self.parent.file.digest != a.expected_parent
                || !self.continuation() && (self.lr_policy != 1 || self.lr_offset != 1024)
                || self.continuation() && a.anchors != 6
                || self.anchor_floor != 178
                || self.train.len() != if self.tiny_spec { 2 } else { 2560 }
                || a.pools[0].len() != if self.tiny_spec { 1 } else { 2048 }
                || a.pools[1].len() != if self.tiny_spec { 1 } else { 512 }
                || all != (0..if self.tiny_spec { 2 } else { 2560 }).collect()
                || self.tape.len()
                    != if self.preflight() || self.tiny_spec {
                        2
                    } else if self.cooldown() {
                        256
                    } else {
                        512
                    }
                || self.eval_steps
                    != if self.tiny_spec {
                        vec![1, 2]
                    } else if self.preflight() {
                        vec![]
                    } else if self.cooldown() {
                        vec![128, 256]
                    } else if self.screen() {
                        vec![32, 64, 128, 256, 512]
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
        ) || self.cooldown() && self.tiny_spec && self.eval_steps.is_empty()
    }
    fn path_parity(&self) -> bool {
        self.contract == NATIVE_CORPUS_CONTRACT
    }
    fn cooldown(&self) -> bool {
        self.contract == COOLDOWN_CONTRACT
    }
    fn continuation(&self) -> bool {
        self.cooldown() || self.objective.is_some() || self.bridge()
    }
    fn bridge(&self) -> bool {
        self.contract == BRIDGE_CONTRACT || self.screen()
    }
    fn screen(&self) -> bool {
        self.contract == BOUNDED_BRIDGE_CONTRACT
    }
    fn study_arms(&self) -> &'static [&'static str] {
        if self.screen() {
            &["T-SCREEN"]
        } else if self.bridge() {
            &["C-COPYMATCH", "T-TEMPORAL"]
        } else if self.objective.is_some() {
            &["B-BASE", "S-SPAN"]
        } else {
            &["K-KEEP", "D-DECAY"]
        }
    }
    fn supports_partial_resume(&self) -> bool {
        matches!(
            self.contract.as_str(),
            RESTART_CONTRACT
                | COOLDOWN_CONTRACT
                | OBJECTIVE_CONTRACT
                | NATIVE_CORPUS_CONTRACT
                | BRIDGE_CONTRACT
                | BOUNDED_BRIDGE_CONTRACT
        ) && !self.historical
            && (self.tiny_spec || self.authorization.is_some())
    }
    fn arm_name(&self) -> &'static str {
        if self.screen() {
            return "T-SCREEN";
        }
        if self.bridge() {
            return if self.purpose == RunPurpose::LrContinuous {
                "C-COPYMATCH"
            } else {
                "T-TEMPORAL"
            };
        }
        if let Some(o) = &self.objective {
            return if o.span { "S-SPAN" } else { "B-BASE" };
        }
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
    // prepare/forward-loss/backward/optimizer seconds, gradient/update norm, objective, clipped.
    objective_metrics: Option<Vec<[f64; 8]>>,
}
impl SegmentReceipt {
    // Disposable schema-capacity value, never an execution receipt or future weights.
    fn capacity_value(draws: Vec<Draw>, rates: bool, metrics: bool) -> Self {
        let count = draws.len();
        Self {
            run: [0; 32],
            binding: [0; 32],
            segment: 0,
            parent: None,
            native: None,
            evaluations: vec![],
            decisions: vec![],
            guard: [0; 3],
            stop: vec![],
            complete: false,
            resume: false,
            candidate: false,
            save_error: Some("SCHEMA_CAPACITY_ONLY_NOT_EXECUTION_EVIDENCE".into()),
            updates: count as u64,
            generations: 0,
            teachers: 0,
            elapsed: Scalar::F64(0.),
            cleanup: Scalar::F64(0.),
            draws,
            lr_bits: rates.then(|| vec![1e-4f64.to_bits(); count]),
            objective_metrics: metrics.then(|| vec![[0.; 8]; count]),
        }
    }
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
        if let Some(metrics) = &self.objective_metrics {
            put_varint(b, metrics.len() as u64);
            for row in metrics {
                for v in row {
                    Scalar::F64(*v).encode(b);
                }
            }
        }
    }
    fn decode(r: &mut Reader<'_>, with_rates: bool, with_metrics: bool) -> Result<Self> {
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
        let n = count(r, MAX_SEGMENT_UPDATES)?;
        if r.remaining() < n * 4 {
            return Err(bad("truncated segment draws"));
        }
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
            let n = count(r, MAX_SEGMENT_UPDATES)?;
            if n != draws.len() {
                return Err(bad("segment LR/draw count mismatch"));
            }
            // Validate the complete byte span before allocating the bounded LR vector.
            let bytes = r.take(n.checked_mul(8).ok_or_else(|| bad("LR byte overflow"))?)?;
            let rates = bytes
                .as_chunks::<8>()
                .0
                .iter()
                .map(|b| u64::from_le_bytes(*b))
                .collect::<Vec<_>>();
            if rates
                .iter()
                .any(|b| !f64::from_bits(*b).is_finite() || f64::from_bits(*b) <= 0.)
            {
                return Err(bad("segment LR must be finite and positive"));
            }
            Some(rates)
        } else {
            None
        };
        let draw_count = draws.len();
        if updates < draw_count as u64 {
            return Err(bad("segment cumulative update clock"));
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
            lr_bits,
            objective_metrics: if with_metrics {
                let n = count(r, MAX_SEGMENT_UPDATES)?;
                if n != draw_count {
                    return Err(bad("segment metrics/draw count mismatch"));
                }
                if r.remaining() < n * 8 * 5 {
                    return Err(bad("truncated segment metrics"));
                }
                let mut rows = Vec::with_capacity(n);
                for _ in 0..n {
                    let mut row = [0.; 8];
                    for v in &mut row {
                        *v = Scalar::decode(r)?.finite()?;
                    }
                    if row.iter().any(|v| *v < 0.) || !matches!(row[7], 0. | 1.) {
                        return Err(bad("objective metric values"));
                    }
                    rows.push(row);
                }
                Some(rows)
            } else {
                None
            },
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
        let n = count(r, 6)?;
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
    ScreenRegistration {
        root: String,
        input: Hash,
        audit: FileRef,
        observation: FileRef,
        parent: FileRef,
        preparation: Scalar,
    },
    ArtifactAudit(Box<ArtifactAudit>),
    PreparationMeasure {
        bindings: [Hash; 5],
        rows: Vec<PreparationTiming>,
    },
    Conditional(Box<ConditionalRecord>),
    BridgeReplacement {
        source_sha: String,
        source: Hash,
        binary: Hash,
        root: String,
        old_root: String,
        old_source: Hash,
        old_binary: FileRef,
        preserved: Vec<FileRef>,
    },
    ConditionalRegistration {
        plan: FileRef,
        source: Hash,
        binary: Hash,
        model: u8,
        native: CheckpointRef,
        attempt: Hash,
        child: String,
        cases: Vec<Hash>,
    },
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
    VerificationPending {
        start: FileRef,
        final_digest: Hash,
    },
    TrainProbe(TrainProbe),
}
// Diagnostic evidence only: it cannot be consumed as a terminal or resume certificate.
#[derive(Clone, Debug, Serialize)]
struct PrefixCheck {
    model: u8,
    ordinal: u32,
    position: u32,
    gold: u32,
    full: u32,
    cached: u32,
    // max_abs, gold-minus-full-argmax, gold-minus-cached-argmax, full top-two gap.
    values: [f64; 4],
}
#[derive(Clone, Debug, Serialize)]
struct ArtifactAudit {
    source: Hash,
    binary: Hash,
    input: FileRef,
    native: CheckpointRef,
    originals: Vec<FileRef>,
    selection: Vec<u32>,
    fresh: Vec<(u8, EvalRow)>,
    numeric: Vec<PrefixCheck>,
    panels: Vec<(PanelKind, Score)>,
    // Generation entered/returned, diagnostic forward entered/returned.
    calls: [u64; 4],
    elapsed: Scalar,
    complete: bool,
    stop: Vec<StopReason>,
    error: Option<String>,
}
impl ArtifactAudit {
    fn encode(&self, b: &mut Vec<u8>) {
        string(b, BOUNDED_BRIDGE_CONTRACT);
        b.extend(self.source);
        b.extend(self.binary);
        self.input.encode(b);
        self.native.encode(b);
        put_varint(b, self.originals.len() as u64);
        for r in &self.originals {
            r.encode(b);
        }
        integers(b, &self.selection);
        put_varint(b, self.fresh.len() as u64);
        for (model, row) in &self.fresh {
            b.push(*model);
            row.encode(b);
        }
        put_varint(b, self.numeric.len() as u64);
        for row in &self.numeric {
            b.push(row.model);
            for n in [row.ordinal, row.position, row.gold, row.full, row.cached] {
                put_varint(b, n.into());
            }
            for n in row.values {
                Scalar::F64(n).encode(b);
            }
        }
        put_varint(b, self.panels.len() as u64);
        for (k, s) in &self.panels {
            b.push(k.tag());
            s.encode(b);
        }
        for n in self.calls {
            put_varint(b, n);
        }
        self.elapsed.encode(b);
        b.push(u8::from(self.complete));
        put_varint(b, self.stop.len() as u64);
        for s in &self.stop {
            b.push(stop_tag(*s));
        }
        optional(b, self.error.as_ref(), |b, s| string(b, s));
    }
    fn decode(r: &mut Reader<'_>) -> Result<Self> {
        if text(r)? != BOUNDED_BRIDGE_CONTRACT {
            return Err(bad("diagnostic contract"));
        }
        let source = digest_read(r)?;
        let binary = digest_read(r)?;
        let input = FileRef::decode(r)?;
        let native = CheckpointRef::decode(r)?;
        let n = count(r, 4096)?;
        let originals = (0..n).map(|_| FileRef::decode(r)).collect::<Result<_>>()?;
        let selection = integers_read(r, 32)?;
        let n = count(r, 64)?;
        let fresh = (0..n)
            .map(|_| Ok((r.byte()?, EvalRow::decode(r)?)))
            .collect::<Result<Vec<_>>>()?;
        let n = count(r, 20)?;
        let mut numeric = Vec::with_capacity(n);
        for _ in 0..n {
            let model = r.byte()?;
            let ordinal = u32_read(r)?;
            let position = u32_read(r)?;
            let gold = u32_read(r)?;
            let full = u32_read(r)?;
            let cached = u32_read(r)?;
            let mut values = [0.; 4];
            for v in &mut values {
                *v = Scalar::decode(r)?.finite()?;
            }
            numeric.push(PrefixCheck {
                model,
                ordinal,
                position,
                gold,
                full,
                cached,
                values,
            });
        }
        let n = count(r, 6)?;
        let panels = (0..n)
            .map(|_| Ok((PanelKind::read(r)?, Score::decode(r)?)))
            .collect::<Result<_>>()?;
        let calls = [r.var()?, r.var()?, r.var()?, r.var()?];
        let elapsed = Scalar::decode(r)?;
        let complete = r.bool()?;
        let n = count(r, 8)?;
        let stop = (0..n).map(|_| stop_read(r)).collect::<Result<Vec<_>>>()?;
        let error = r.opt(text)?;
        if calls.iter().any(|n| *n > 64)
            || calls[1] > calls[0]
            || calls[3] > calls[2]
            || elapsed.finite()? < 0.
            || fresh.iter().any(|(m, _)| *m > 1)
            || numeric.iter().any(|v| v.model > 1)
            || complete
                && (error.is_some()
                    || !stop.is_empty()
                    || calls[0] != calls[1]
                    || calls[2] != calls[3])
        {
            return Err(bad("diagnostic status/counters"));
        }
        Ok(Self {
            source,
            binary,
            input,
            native,
            originals,
            selection,
            fresh,
            numeric,
            panels,
            calls,
            elapsed,
            complete,
            stop,
            error,
        })
    }
}
#[derive(Clone, Debug, Serialize)]
struct PreparationTiming {
    format: String,
    repetition: u8,
    bytes: u64,
    seconds: [f64; 9],
    rss: Option<u64>,
}
#[derive(Clone, Debug, Serialize)]
struct ConditionalRecord {
    registration: Option<FileRef>,
    seed: u64,
    source: Hash,
    plan: Option<FileRef>,
    models: Vec<CheckpointRef>,
    cases: Vec<Episode>,
    foils: Vec<String>,
    rows: Vec<EvalRow>,
    raw_bytes: Vec<Vec<u8>>,
    model_index: u8,
    generations: u64,
    teachers: u64,
    elapsed: Scalar,
    complete: bool,
    stop: Vec<StopReason>,
    error: Option<String>,
}
impl ConditionalRecord {
    fn encode(&self, b: &mut Vec<u8>) {
        put_varint(b, self.seed);
        b.extend(self.source);
        optional(b, self.plan.as_ref(), |b, v| v.encode(b));
        put_varint(b, self.models.len() as u64);
        for v in &self.models {
            v.encode(b);
        }
        put_varint(b, self.cases.len() as u64);
        for e in &self.cases {
            episode_encode(e, b);
        }
        put_varint(b, self.foils.len() as u64);
        for f in &self.foils {
            string(b, f);
        }
        put_varint(b, self.rows.len() as u64);
        for row in &self.rows {
            row.encode(b);
        }
        put_varint(b, self.raw_bytes.len() as u64);
        for raw in &self.raw_bytes {
            put_bytes(b, raw);
        }
        b.push(self.model_index);
        put_varint(b, self.generations);
        put_varint(b, self.teachers);
        self.elapsed.encode(b);
        b.push(u8::from(self.complete));
        put_varint(b, self.stop.len() as u64);
        for s in &self.stop {
            b.push(stop_tag(*s));
        }
        optional(b, self.error.as_ref(), |b, s| string(b, s));
    }
    fn decode(r: &mut Reader<'_>) -> Result<Self> {
        let seed = r.var()?;
        let source = digest_read(r)?;
        let plan = r.opt(FileRef::decode)?;
        let n = count(r, 2)?;
        let models = (0..n)
            .map(|_| CheckpointRef::decode(r))
            .collect::<Result<Vec<_>>>()?;
        let n = count(r, 144)?;
        let cases = (0..n)
            .map(|_| episode_decode(r))
            .collect::<Result<Vec<_>>>()?;
        let n = count(r, 144)?;
        let foils = (0..n).map(|_| text(r)).collect::<Result<Vec<_>>>()?;
        let n = count(r, 144)?;
        let rows = (0..n)
            .map(|_| EvalRow::decode(r))
            .collect::<Result<Vec<_>>>()?;
        let n = count(r, 144)?;
        let raw_bytes = (0..n)
            .map(|_| r.bytes(MAX_TEXT))
            .collect::<Result<Vec<_>>>()?;
        let model_index = r.byte()?;
        let generations = r.var()?;
        let teachers = r.var()?;
        let elapsed = Scalar::decode(r)?;
        let complete = r.bool()?;
        let n = count(r, 8)?;
        let stop = (0..n).map(|_| stop_read(r)).collect::<Result<Vec<_>>>()?;
        let error = r.opt(text)?;
        if models.len() != 2
            || model_index > 1
            || generations > 144
            || teachers > 144
            || rows.len() != raw_bytes.len()
            || elapsed.finite()? < 0.
            || (plan.is_none()
                && (cases.is_empty() || foils.len() != cases.len() || !rows.is_empty()))
            || (plan.is_some() && (!cases.is_empty() || !foils.is_empty()))
            || (complete
                && (rows.is_empty()
                    || generations != rows.len() as u64
                    || teachers != rows.len() as u64
                    || !stop.is_empty()
                    || error.is_some()))
        {
            return Err(bad("conditional record shape/budget/completion"));
        }
        Ok(Self {
            registration: None,
            seed,
            source,
            plan,
            models,
            cases,
            foils,
            rows,
            raw_bytes,
            model_index,
            generations,
            teachers,
            elapsed,
            complete,
            stop,
            error,
        })
    }
}
#[derive(Clone, Debug, Serialize)]
struct TrainProbe {
    binding: Hash,
    source: Hash,
    native: CheckpointRef,
    complete: bool,
    rows: Vec<(u32, target_loss::Probe)>,
}
impl TrainProbe {
    fn encode(&self, b: &mut Vec<u8>) {
        b.extend(self.binding);
        b.extend(self.source);
        self.native.encode(b);
        b.push(u8::from(self.complete));
        put_varint(b, self.rows.len() as u64);
        for (ordinal, p) in &self.rows {
            put_varint(b, u64::from(*ordinal));
            put_varint(b, u64::from(p.targets));
            put_varint(b, u64::from(p.correct));
            optional(b, p.first_error_role.as_ref(), |b, v| b.push(*v));
            for v in [p.base, p.span]
                .into_iter()
                .chain(p.mass)
                .chain(p.nll)
                .chain(p.weighted_nll)
            {
                Scalar::F64(v).encode(b);
            }
        }
    }
    fn decode(r: &mut Reader<'_>) -> Result<Self> {
        let binding = digest_read(r)?;
        let source = digest_read(r)?;
        let native = CheckpointRef::decode(r)?;
        let complete = r.bool()?;
        let n = count(r, 64)?;
        let mut rows = Vec::with_capacity(n);
        for _ in 0..n {
            let ordinal = u32_read(r)?;
            let targets = u32_read(r)?;
            let correct = u32_read(r)?;
            let first_error_role = r.opt(|r| r.byte())?;
            let mut values = [0.; 20];
            for v in &mut values {
                *v = Scalar::decode(r)?.finite()?;
            }
            if targets == 0
                || correct > targets
                || first_error_role.is_some_and(|r| r > 5)
                || values.iter().any(|v| *v < 0.)
            {
                return Err(bad("train probe values"));
            }
            rows.push((
                ordinal,
                target_loss::Probe {
                    targets,
                    correct,
                    first_error_role,
                    base: values[0],
                    span: values[1],
                    mass: values[2..8].try_into().unwrap(),
                    nll: values[8..14].try_into().unwrap(),
                    weighted_nll: values[14..20].try_into().unwrap(),
                },
            ));
        }
        Ok(Self {
            binding,
            source,
            native,
            complete,
            rows,
        })
    }
}
#[derive(Clone, Debug, Serialize)]
struct VerificationStart {
    publication_v2: bool,
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
    fn decode(r: &mut Reader<'_>, scope: u8, publication_v2: bool) -> Result<Self> {
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
                3 => 320,
                4 => 48,
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
            publication_v2,
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
            || teachers > 64
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
    teacher_evidence: Option<Vec<FileRef>>,
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
            teacher_evidence: None,
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
            Self::ScreenRegistration {
                root,
                input,
                audit,
                observation,
                parent,
                preparation,
            } => {
                string(&mut body, BOUNDED_BRIDGE_CONTRACT);
                string(&mut body, root);
                body.extend(input);
                for r in [audit, observation, parent] {
                    r.encode(&mut body);
                }
                preparation.encode(&mut body);
                31
            }
            Self::ArtifactAudit(v) => {
                v.encode(&mut body);
                30
            }
            Self::BridgeReplacement {
                source_sha,
                source,
                binary,
                root,
                old_root,
                old_source,
                old_binary,
                preserved,
            } => {
                string(&mut body, BRIDGE_RESTART_CONTRACT);
                string(&mut body, "replacement-01");
                string(&mut body, source_sha);
                body.extend(source);
                body.extend(binary);
                string(&mut body, root);
                string(&mut body, old_root);
                body.extend(old_source);
                old_binary.encode(&mut body);
                for limit in [1024, 4608, 192, 7200, 1800, 120] {
                    put_varint(&mut body, limit);
                }
                put_varint(&mut body, preserved.len() as u64);
                for r in preserved {
                    r.encode(&mut body);
                }
                29
            }
            Self::PreparationMeasure { bindings, rows } => {
                for h in bindings {
                    body.extend(h);
                }
                put_varint(&mut body, rows.len() as u64);
                for row in rows {
                    string(&mut body, &row.format);
                    body.push(row.repetition);
                    put_varint(&mut body, row.bytes);
                    for f in row.seconds {
                        Scalar::F64(f).encode(&mut body);
                    }
                    optional(&mut body, row.rss.as_ref(), |b, n| put_varint(b, *n));
                }
                24
            }
            Self::Conditional(v) => {
                v.encode(&mut body);
                if let Some(registration) = &v.registration {
                    registration.encode(&mut body);
                    28
                } else {
                    23
                }
            }
            Self::ConditionalRegistration {
                plan,
                source,
                binary,
                model,
                native,
                attempt,
                child,
                cases,
            } => {
                plan.encode(&mut body);
                body.extend(source);
                body.extend(binary);
                body.push(*model);
                native.encode(&mut body);
                body.extend(attempt);
                string(&mut body, child);
                put_varint(&mut body, cases.len() as u64);
                for case in cases {
                    body.extend(case);
                }
                26
            }
            Self::Inputs(v) => {
                if matches!(
                    v.contract.as_str(),
                    RESTART_CONTRACT
                        | COOLDOWN_CONTRACT
                        | OBJECTIVE_CONTRACT
                        | NATIVE_CORPUS_CONTRACT
                        | BRIDGE_CONTRACT
                        | BOUNDED_BRIDGE_CONTRACT
                ) {
                    body.push(u8::from(v.authorization.is_some()));
                }
                v.encode(&mut body);
                if v.bridge() {
                    25
                } else if v.objective.is_some() {
                    19
                } else if v.cooldown() {
                    13
                } else if v.contract == RESTART_CONTRACT {
                    8
                } else if v.path_parity() {
                    22
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
                if v.objective_metrics.is_some() {
                    21
                } else if v.lr_bits.is_some() {
                    14
                } else {
                    4
                }
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
                let extended =
                    v.teacher_evidence.is_some() || v.generations > 8 || v.panels.len() != 2;
                v.encode(&mut body, extended);
                if let Some(refs) = &v.teacher_evidence {
                    put_varint(&mut body, refs.len() as u64);
                    for r in refs {
                        r.encode(&mut body);
                    }
                    27
                } else if extended {
                    16
                } else {
                    9
                }
            }
            Self::VerificationStart(v) => {
                if v.scope != 0 || v.publication_v2 {
                    body.push(v.scope);
                }
                v.encode(&mut body);
                if v.publication_v2 {
                    17
                } else if v.scope == 0 {
                    10
                } else {
                    15
                }
            }
            Self::VerificationRow(v) => {
                v.encode(&mut body);
                11
            }
            Self::VerificationFinal(v) => {
                v.encode(&mut body);
                12
            }
            Self::TrainProbe(v) => {
                v.encode(&mut body);
                20
            }
            Self::VerificationPending {
                start,
                final_digest,
            } => {
                start.encode(&mut body);
                body.extend(final_digest);
                18
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
            31 => {
                if text(&mut r)? != BOUNDED_BRIDGE_CONTRACT {
                    return Err(bad("screen registration contract"));
                }
                Self::ScreenRegistration {
                    root: text(&mut r)?,
                    input: digest_read(&mut r)?,
                    audit: FileRef::decode(&mut r)?,
                    observation: FileRef::decode(&mut r)?,
                    parent: FileRef::decode(&mut r)?,
                    preparation: Scalar::decode(&mut r)?,
                }
            }
            30 => Self::ArtifactAudit(Box::new(ArtifactAudit::decode(&mut r)?)),
            24 => {
                let bindings = [
                    digest_read(&mut r)?,
                    digest_read(&mut r)?,
                    digest_read(&mut r)?,
                    digest_read(&mut r)?,
                    digest_read(&mut r)?,
                ];
                let n = count(&mut r, 30)?;
                let mut rows = Vec::new();
                for _ in 0..n {
                    let format = text(&mut r)?;
                    let repetition = r.byte()?;
                    let bytes = r.var()?;
                    let mut seconds = [0.; 9];
                    for f in &mut seconds {
                        *f = Scalar::decode(&mut r)?.finite()?;
                        if *f < 0. {
                            return Err(bad("measurement negative time"));
                        }
                    }
                    let rss = r.opt(|r| r.var())?;
                    rows.push(PreparationTiming {
                        format,
                        repetition,
                        bytes,
                        seconds,
                        rss,
                    });
                }
                Self::PreparationMeasure { bindings, rows }
            }
            29 => {
                if text(&mut r)? != BRIDGE_RESTART_CONTRACT || text(&mut r)? != "replacement-01" {
                    return Err(bad("replacement contract/attempt"));
                }
                let source_sha = text(&mut r)?;
                if source_sha.len() != 40 || !source_sha.bytes().all(|b| b.is_ascii_hexdigit()) {
                    return Err(bad("replacement source SHA"));
                }
                let source = digest_read(&mut r)?;
                let binary = digest_read(&mut r)?;
                let root = text(&mut r)?;
                let old_root = text(&mut r)?;
                let old_source = digest_read(&mut r)?;
                let old_binary = FileRef::decode(&mut r)?;
                for limit in [1024, 4608, 192, 7200, 1800, 120] {
                    if r.var()? != limit {
                        return Err(bad("replacement budget"));
                    }
                }
                let count = count(&mut r, 4096)?;
                let preserved = (0..count)
                    .map(|_| FileRef::decode(&mut r))
                    .collect::<Result<Vec<_>>>()?;
                if count == 0 || preserved.windows(2).any(|w| w[0].locator >= w[1].locator) {
                    return Err(bad("replacement preservation manifest"));
                }
                Self::BridgeReplacement {
                    source_sha,
                    source,
                    binary,
                    root,
                    old_root,
                    old_source,
                    old_binary,
                    preserved,
                }
            }
            23 | 28 => {
                let mut v = ConditionalRecord::decode(&mut r)?;
                if kind == 28 {
                    v.registration = Some(FileRef::decode(&mut r)?);
                }
                Self::Conditional(Box::new(v))
            }
            26 => {
                let plan = FileRef::decode(&mut r)?;
                let source = digest_read(&mut r)?;
                let binary = digest_read(&mut r)?;
                let model = r.byte()?;
                let native = CheckpointRef::decode(&mut r)?;
                let attempt = digest_read(&mut r)?;
                let child = text(&mut r)?;
                let n = count(&mut r, 144)?;
                let cases = (0..n)
                    .map(|_| digest_read(&mut r))
                    .collect::<Result<Vec<_>>>()?;
                if model > 1 || child != format!("model-{model}") || n == 0 {
                    return Err(bad("conditional registration bounds/locator"));
                }
                Self::ConditionalRegistration {
                    plan,
                    source,
                    binary,
                    model,
                    native,
                    attempt,
                    child,
                    cases,
                }
            }
            1 | 6 | 8 | 13 | 19 | 22 | 25 => {
                let authorized = if matches!(kind, 8 | 13 | 19 | 22 | 25) {
                    r.bool()?
                } else {
                    kind == 6
                };
                let s = RunSnapshot::decode(&mut r, authorized)?;
                if (kind == 8) != (s.contract == RESTART_CONTRACT)
                    || (kind == 13) != s.cooldown()
                    || (kind == 19) != s.objective.is_some()
                    || (kind == 22) != s.path_parity()
                    || (kind == 25) != s.bridge()
                {
                    return Err(bad("purpose record kind"));
                }
                Self::Inputs(Box::new(s))
            }
            2 => Self::Evaluation(EvalPayload::decode(&mut r)?),
            3 => Self::Decision(EvalDecision::decode(&mut r)?),
            4 | 14 | 21 => Self::Segment(SegmentReceipt::decode(&mut r, kind != 4, kind == 21)?),
            5 => Self::Comparison(ComparisonReceipt::decode(&mut r)?),
            7 => Self::Command(CommandOutcome::decode(&mut r)?),
            9 | 16 | 27 => {
                let mut p = PreflightReceipt::decode(&mut r, kind != 9)?;
                if kind == 27 {
                    let n = count(&mut r, 130)?;
                    if n < 6 || n % 2 != 0 {
                        return Err(bad("teacher evidence reference count"));
                    }
                    p.teacher_evidence = Some(
                        (0..n)
                            .map(|_| FileRef::decode(&mut r))
                            .collect::<Result<_>>()?,
                    );
                }
                Self::Preflight(p)
            }
            10 | 15 | 17 => {
                let scope = if kind != 10 { r.byte()? } else { 0 };
                Self::VerificationStart(VerificationStart::decode(&mut r, scope, kind == 17)?)
            }
            11 => Self::VerificationRow(VerificationRow::decode(&mut r)?),
            12 => Self::VerificationFinal(VerificationFinal::decode(&mut r)?),
            20 => Self::TrainProbe(TrainProbe::decode(&mut r)?),
            18 => Self::VerificationPending {
                start: FileRef::decode(&mut r)?,
                final_digest: digest_read(&mut r)?,
            },
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
    #[cfg(feature = "test-support")]
    if std::env::var("R3ER_TEST_STOP").as_deref() == Ok("audit-final-sync-and-record-fail")
        && root.join("audit-final.r3er").exists()
    {
        return Err(std::io::Error::other("injected all subsequent audit writes fail").into());
    }
    #[cfg(feature = "test-support")]
    if std::env::var("R3ER_TEST_STOP").as_deref() == Ok("pv-final-sync-and-record-fail")
        && root.join("preflight-final.r3er").exists()
    {
        return Err(std::io::Error::other("injected all subsequent record writes fail").into());
    }
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
    h.update(include_bytes!("target_loss.rs"));
    h.update(include_bytes!("token_cache.rs"));
    h.update(include_bytes!("native_corpus.rs"));
    h.update(include_bytes!("neural/checkpoint.rs"));
    h.update(include_bytes!("neural/artifact.rs"));
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
        #[cfg(feature = "test-support")]
        if s.tiny_spec && kind == PanelKind::Dev {
            let boundary = if step - s.parent.step == 1 { 128 } else { 256 };
            let mode = std::env::var("R3ER_TEST_STOP").unwrap_or_default();
            if mode == format!("partial-dev:{boundary}:{}", rows.len()) {
                println!(
                    "TINY_LOGICAL_BOUNDARY={boundary} COMPLETED_PREFIX={}",
                    rows.len()
                );
                control.deadline = Instant::now();
            }
        }
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
                if s.screen()
                    && kind == PanelKind::Dev
                    && step - s.parent.step == 2
                    && std::env::var("R3ER_TEST_STOP").as_deref() == Ok("screen-synthetic-quality")
                {
                    row.tokens.clear();
                    row.eos = None;
                    row.finish = Finish::Error;
                    row.error = Some("explicit synthetic generation error fixture".into());
                    row.error_class = Some("model".into());
                    row.timing = None;
                    println!("SYNTHETIC_QUALITY_ERROR_ROW=true NOT_MODEL_QUALITY_EVIDENCE=true");
                }
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
fn objective_binding(
    s: &RunSnapshot,
    state: &TrainingState,
    tok: &ByteBpe,
) -> Result<checkpoint::ResumeBinding> {
    let mut binding = checkpoint::ResumeBinding::default_for(state, tok);
    binding.execution = 1;
    binding.policy = s.binding();
    binding.provenance = s.policy;
    let mut ordered = Vec::new();
    for &i in &s.train {
        episode_encode(&s.cases[i as usize], &mut ordered);
    }
    binding.train_order = hash(&ordered);
    if let Some(o) = &s.objective
        && o.span
    {
        binding.family = 2;
        binding.normalizer = 2;
        binding.span_alpha_bits = Some(1f64.to_bits());
        binding.annotation = Some(o.annotation);
    }
    binding.validate(state, tok)?;
    Ok(binding)
}
fn native_source(root: &Path, s: &RunSnapshot) -> Result<data::native::Corpus> {
    let origin = s
        .origins
        .iter()
        .find(|o| o.role == "native-source")
        .ok_or_else(|| bad("native source binding absent"))?;
    let c = data::native::read(&owned_path(root, &origin.original.locator, true)?)?;
    let expected = s
        .train
        .iter()
        .map(|i| s.cases[*i as usize].clone())
        .collect::<Vec<_>>();
    let dev = panel(s, PanelKind::Dev)?
        .cases
        .iter()
        .map(|i| s.cases[*i as usize].clone())
        .collect::<Vec<_>>();
    if c.physical != origin.original.digest
        || data::native::ordered_bytes(&c.train) != data::native::ordered_bytes(&expected)
        || data::native::ordered_bytes(&c.validation) != data::native::ordered_bytes(&dev)
    {
        return Err(bad("native source physical/order/content mismatch"));
    }
    Ok(c)
}
fn conditional_prepare(
    parent: &Path,
    base: &Path,
    output: &Path,
    seed: u64,
    control: &mut RunControl,
) -> Result<()> {
    let mut models = Vec::new();
    let mut prior = Vec::new();
    for root in [parent, base] {
        let s = read_inputs(root)?;
        if s.objective.as_ref().is_some_and(|o| o.span) || s.tiny_spec {
            return Err(bad("conditional requires actual default SMALL lineage"));
        }
        let commands = arm_commands(root, &s)?;
        let (_, command) = commands
            .last()
            .ok_or_else(|| bad("conditional endpoint missing"))?;
        close_native_inner(root, &command.terminal.locator, control, false)?;
        let chain = lineage(root, &s, &command.terminal)?;
        let mut n = chain
            .last()
            .unwrap()
            .1
            .native
            .clone()
            .ok_or_else(|| bad("conditional durable endpoint"))?;
        let _ = resolve_native(root, &s, &n, true)?;
        n.file.locator = owned_path(root, &n.file.locator, true)?
            .canonicalize()?
            .display()
            .to_string();
        models.push(n);
        prior.extend(s.cases);
    }
    if models[0].step != 24310 || models[1].step != 24822 {
        return Err(bad("conditional registered model steps"));
    }
    let (cases, foils) = data::conditional_panel(&prior, seed)?;
    let p = ConditionalRecord {
        registration: None,
        seed,
        source: evaluator_source(),
        plan: None,
        models,
        cases,
        foils,
        rows: vec![],
        raw_bytes: vec![],
        model_index: 0,
        generations: 0,
        teachers: 0,
        elapsed: Scalar::F64(0.),
        complete: false,
        stop: vec![],
        error: None,
    };
    std::fs::create_dir(output)?;
    publish(output, "plan.r3er", &Record::Conditional(Box::new(p)))?;
    println!(
        "CONDITIONAL_PLAN=FROZEN base=24 views=6 generation_budget=288 teacher_budget=288 quality_updates=0 SEAL=NOT_OPENED"
    );
    Ok(())
}
fn conditional_plan(root: &Path, require_execution_source: bool) -> Result<ConditionalRecord> {
    let Record::Conditional(p) = read_record(root, &reference(root, "plan.r3er")?)? else {
        return Err(bad("conditional plan kind"));
    };
    if p.plan.is_some() || require_execution_source && p.source != evaluator_source() {
        return Err(bad("conditional source/plan changed"));
    }
    if p.cases.len() != 144 {
        #[cfg(feature = "test-support")]
        if p.cases.len() == 6
            && p.models.iter().all(|n| {
                checkpoint::metadata(Path::new(&n.file.locator))
                    .is_ok_and(|(m, _)| m.architecture.profile == "TINY_NUMERIC_TEST_ONLY")
            })
        {
            return Ok(*p);
        }
        return Err(bad("conditional production panel count"));
    }
    Ok(*p)
}
fn version_parity(
    parent: &Path,
    path: &Path,
    output: &Path,
    compare: Option<&Path>,
    control: &mut RunControl,
) -> Result<()> {
    let s = read_inputs(parent)?;
    let commands = arm_commands(parent, &s)?;
    let (_, c) = commands
        .last()
        .ok_or_else(|| bad("parity parent terminal"))?;
    let chain = lineage(parent, &s, &c.terminal)?;
    let n = chain
        .last()
        .unwrap()
        .1
        .native
        .as_ref()
        .ok_or_else(|| bad("parity saved endpoint"))?;
    let l = checkpoint::load(path, Device::Cpu, false)?;
    if unhex(&l.model.weight_hash()?)? != n.model
        || unhex(&l.tokenizer.semantic_id())? != n.tokenizer
        || l.manifest.trained_steps as u64 != n.step
    {
        return Err(bad("version parity endpoint identity"));
    }
    let mut ids = panel(&s, PanelKind::Dev)?.cases.clone();
    ids.sort_by_key(|i| (&s.cases[*i as usize].family, &s.cases[*i as usize].id));
    if ids.len() != 256 {
        return Err(bad("version parity fixed dev denominator"));
    }
    let ids = (0..16).map(|i| ids[i * 16]).collect::<Vec<_>>();
    let mut e = EvalPayload {
        run: unhex(&file_hash(path)?)?,
        binding: s.binding(),
        source: evaluator_source(),
        model: n.model,
        tokenizer: n.tokenizer,
        architecture: n.architecture,
        step: n.step,
        new_updates: 0,
        kind: PanelKind::DevParity,
        expected: 16,
        rows: vec![],
    };
    std::fs::create_dir(output)?;
    publish(output, "start.r3er", &Record::Evaluation(e.clone()))?;
    control.generation_limit = 16;
    for (i, ordinal) in ids.iter().enumerate() {
        let row = evaluate_row(&l, &s.cases[*ordinal as usize], *ordinal, control, false)?;
        e.rows.push(row);
        publish(
            output,
            &format!("row-{i:02}.r3er"),
            &Record::Evaluation(e.clone()),
        )?;
        control.check("version_parity_row_saved")?;
    }
    publish(output, "final.r3er", &Record::Evaluation(e.clone()))?;
    if let Some(other) = compare {
        let Record::Evaluation(old) =
            Record::decode(&neural::read_bounded(&other.join("final.r3er"), MAX_FILE)?)?
        else {
            return Err(bad("version parity reference kind"));
        };
        if old.model != e.model
            || old.tokenizer != e.tokenizer
            || old.binding != e.binding
            || old.rows.len() != 16
            || old.step != e.step
        {
            return Err(bad("version parity comparison identity"));
        }
        for (a, b) in old.rows.iter().zip(&e.rows) {
            if a.case != b.case
                || a.prompt != b.prompt
                || a.tokens != b.tokens
                || a.eos != b.eos
                || a.error != b.error
                || a.finish != b.finish
            {
                return Err(bad("version normal greedy mismatch"));
            }
        }
        println!("V1_V2_NORMAL_GREEDY_PARITY=16/16");
    }
    println!(
        "VERSION_PARITY_GENERATIONS={} TEACHERS={} QUALITY_UPDATES=0 SELECTION=sorted_family_id_every16_frozen_dev",
        control.generation_calls, control.teacher_calls
    );
    Ok(())
}
fn conditional_model(n: &CheckpointRef) -> Result<Loaded> {
    if unhex(&file_hash(Path::new(&n.file.locator))?)? != n.file.digest {
        return Err(bad("conditional checkpoint physical hash"));
    }
    let l = checkpoint::load(Path::new(&n.file.locator), Device::Cpu, false)?;
    if unhex(&l.model.weight_hash()?)? != n.model
        || unhex(&l.tokenizer.semantic_id())? != n.tokenizer
        || unhex(&l.model.config.semantic_id()?)? != n.architecture
        || l.manifest.trained_steps as u64 != n.step
    {
        return Err(bad("conditional model/tokenizer/step"));
    }
    Ok(l)
}
fn conditional_record(
    root: &Path,
    name: &str,
    p: &ConditionalRecord,
    plan: &FileRef,
    model: u8,
) -> Result<ConditionalRecord> {
    let Record::Conditional(r) = read_record(root, &reference(root, name)?)? else {
        return Err(bad("conditional record kind"));
    };
    if r.plan.as_ref() != Some(plan)
        || r.source != p.source
        || r.models != p.models
        || r.seed != p.seed
        || r.model_index != model
        || r.rows.len() > p.cases.len()
    {
        return Err(bad("conditional attempt identity"));
    }
    let parent = root.parent().ok_or_else(|| bad("conditional root"))?;
    if parent.join(format!("attempt-{model}.r3er")).exists()
        && r.registration != Some(reference(parent, &format!("attempt-{model}.r3er"))?)
    {
        return Err(bad("conditional child registration reference"));
    }
    for (i, row) in r.rows.iter().enumerate() {
        if row.ordinal as usize != i || row.case != case_hash(&p.cases[i]) || !row.started {
            return Err(bad("conditional row identity/order"));
        }
    }
    Ok(*r)
}
fn same_conditional_rows(a: &[EvalRow], b: &[EvalRow]) -> bool {
    let encode = |rows: &[EvalRow]| {
        let mut bytes = Vec::new();
        for row in rows {
            row.encode(&mut bytes);
        }
        bytes
    };
    encode(a) == encode(b)
}
fn conditional_attempt(
    root: &Path,
    p: &ConditionalRecord,
    model: u8,
    historical: bool,
) -> Result<Option<ConditionalRecord>> {
    let dir = root.join(format!("model-{model}"));
    let plan = reference(root, "plan.r3er")?;
    let registration_name = format!("attempt-{model}.r3er");
    let registered = root.join(&registration_name).exists();
    if registered {
        let Record::ConditionalRegistration {
            plan: registered_plan,
            source,
            binary,
            model: index,
            native,
            attempt,
            child,
            cases,
        } = read_record(root, &reference(root, &registration_name)?)?
        else {
            return Err(bad("conditional registration kind"));
        };
        let mut identity = plan.digest.to_vec();
        identity.extend(p.source);
        identity.push(model);
        string(&mut identity, &root.canonicalize()?.display().to_string());
        if registered_plan != plan
            || source != p.source
            || index != model
            || !historical && binary != unhex(&file_hash(&std::env::current_exe()?)?)?
            || native != p.models[model as usize]
            || attempt != hash(&identity)
            || child != format!("model-{model}")
            || cases != p.cases.iter().map(case_hash).collect::<Vec<_>>()
        {
            return Err(bad("conditional root registration identity"));
        }
        if !dir.is_dir() {
            return Err(bad("MISSING_ATTEMPT_EVIDENCE UNKNOWN usage; plan blocked"));
        }
    } else {
        if !dir.exists() {
            if std::fs::read_dir(root)?.any(|e| {
                e.is_ok_and(|e| e.file_name().to_string_lossy().contains(&registration_name))
            }) {
                return Err(bad("conditional registration pending; UNKNOWN usage"));
            }
            return Ok(None);
        }
        if !historical {
            return Err(bad(
                "conditional historical child without root registration; plan blocked",
            ));
        }
    }
    let start = conditional_record(&dir, "start.r3er", p, &plan, model)?;
    if !start.rows.is_empty()
        || start.generations != 0
        || start.teachers != 0
        || start.complete
        || !start.stop.is_empty()
        || start.error.is_some()
        || registered && start.registration != Some(reference(root, &registration_name)?)
    {
        return Err(bad("conditional start state"));
    }
    let mut observed = start;
    let mut entries = 0;
    let mut teacher_entries = 0;
    for i in 0..p.cases.len() {
        for (prefix, expected_rows) in [
            ("entry", i),
            ("generation", i + 1),
            ("teacher-entry", i + 1),
            ("row", i + 1),
        ] {
            let name = format!("{prefix}-{i:03}.r3er");
            if !dir.join(&name).exists() {
                continue;
            }
            let r = conditional_record(&dir, &name, p, &plan, model)?;
            if r.rows.len() != expected_rows
                || r.generations < observed.generations
                || r.teachers < observed.teachers
                || r.elapsed.finite()? < observed.elapsed.finite()?
            {
                return Err(bad("conditional progress counters"));
            }
            let prior = i.min(observed.rows.len());
            if !same_conditional_rows(&r.rows[..prior], &observed.rows[..prior])
                || r.raw_bytes[..prior] != observed.raw_bytes[..prior]
            {
                return Err(bad("conditional immutable prefix"));
            }
            if prefix == "entry" {
                entries += 1;
            }
            if prefix == "teacher-entry" {
                teacher_entries += 1;
            }
            observed = r;
        }
    }
    if !dir.join("final.r3er").exists() || dir.join("publication-pending.r3er").exists() {
        println!(
            "CONDITIONAL_ATTEMPT={model} STATUS=INCOMPLETE_OR_UNCERTAIN KNOWN_GENERATIONS={} GENERATION_ENTRIES={entries} KNOWN_TEACHERS={} TEACHER_ENTRIES={teacher_entries} ELAPSED_LOWER_BOUND={} UNKNOWN_TAIL=true RESERVED_UPPER_BOUND={} NEW_GENERATIONS=0 NEW_TEACHERS=0",
            observed.generations,
            observed.teachers,
            observed.elapsed.finite()?,
            p.cases.len()
        );
        return Err(bad(
            "conditional incomplete/pending attempt; UNKNOWN usage; plan blocked",
        ));
    }
    let final_state = conditional_record(&dir, "final.r3er", p, &plan, model)?;
    if final_state.generations < observed.generations
        || final_state.teachers < observed.teachers
        || final_state.elapsed.finite()? < observed.elapsed.finite()?
        || final_state.rows.len() < observed.rows.len()
    {
        return Err(bad("conditional final lost observed usage"));
    }
    if !final_state.complete || !final_state.stop.is_empty() || final_state.error.is_some() {
        println!(
            "CONDITIONAL_ATTEMPT={model} STATUS=FAILED KNOWN_GENERATIONS={} KNOWN_TEACHERS={} OBSERVED_TOKENS={} STOP={:?} ERROR={:?} NEW_GENERATIONS=0 NEW_TEACHERS=0",
            final_state.generations,
            final_state.teachers,
            final_state
                .rows
                .iter()
                .map(|r| r.tokens.len())
                .sum::<usize>(),
            final_state.stop,
            final_state.error
        );
        return Err(bad("conditional persisted execution failure; plan blocked"));
    }
    let historical_teacher_entries = teacher_entries == 0 && p.source != evaluator_source();
    if final_state.rows.len() != p.cases.len()
        || entries != p.cases.len()
        || (teacher_entries != p.cases.len() && !historical_teacher_entries)
        || !same_conditional_rows(&final_state.rows, &observed.rows)
        || final_state.raw_bytes != observed.raw_bytes
    {
        return Err(bad("conditional final coverage/prefix"));
    }
    for (i, row) in final_state.rows.iter().enumerate() {
        let generation =
            conditional_record(&dir, &format!("generation-{i:03}.r3er"), p, &plan, model)?;
        let returned = conditional_record(&dir, &format!("row-{i:03}.r3er"), p, &plan, model)?;
        let mut raw = row.clone();
        raw.teacher = TeacherRecord::NotRequested;
        raw.diagnostic = None;
        if !same_conditional_rows(&[raw], &generation.rows[i..])
            || !same_conditional_rows(&final_state.rows[..=i], &returned.rows)
            || !row.completed
            || row.interruption.is_some()
            || !matches!(row.teacher, TeacherRecord::Measured(_))
        {
            return Err(bad("conditional completed raw/teacher agreement"));
        }
    }
    Ok(Some(final_state))
}
fn conditional_run(root: &Path, model: u8, control: &mut RunControl) -> Result<()> {
    if model > 1 {
        return Err(bad("conditional model index"));
    }
    let lock = std::fs::File::open(root.join("plan.r3er"))?;
    lock.try_lock()
        .map_err(|e| Error::Conflict(format!("conditional plan writer: {e}")))?;
    let p = conditional_plan(root, true)?;
    let mut completed = 0;
    for i in 0..2 {
        if conditional_attempt(root, &p, i, false)?.is_some() {
            completed += 1;
        }
    }
    if completed != model || root.join(format!("model-{model}")).exists() {
        return Err(bad("conditional model order/retry; plan blocked"));
    }
    let dir = root.join(format!("model-{model}"));
    let plan = reference(root, "plan.r3er")?;
    let mut identity = plan.digest.to_vec();
    identity.extend(p.source);
    identity.push(model);
    string(&mut identity, &root.canonicalize()?.display().to_string());
    let registration = publish(
        root,
        &format!("attempt-{model}.r3er"),
        &Record::ConditionalRegistration {
            plan: plan.clone(),
            source: p.source,
            binary: unhex(&file_hash(&std::env::current_exe()?)?)?,
            model,
            native: p.models[model as usize].clone(),
            attempt: hash(&identity),
            child: format!("model-{model}"),
            cases: p.cases.iter().map(case_hash).collect(),
        },
    )?;
    std::fs::create_dir(&dir)?;
    let mut result = ConditionalRecord {
        registration: Some(registration),
        cases: vec![],
        foils: vec![],
        plan: Some(reference(root, "plan.r3er")?),
        model_index: model,
        ..p.clone()
    };
    publish(
        &dir,
        "start.r3er",
        &Record::Conditional(Box::new(result.clone())),
    )?;
    control.generation_limit = p.cases.len();
    control.teacher_limit = p.cases.len();
    let outcome = (|| -> Result<()> {
        println!("CONDITIONAL_MODEL_LOAD=1 MODEL_INDEX={model}");
        let l = conditional_model(&p.models[model as usize])?;
        let tiny = l.model.config.profile == "TINY_NUMERIC_TEST_ONLY";
        verification_fault(tiny, "started", control)?;
        for (i, e) in p.cases.iter().enumerate() {
            control.check("conditional_before_generation")?;
            let mut row = evaluate_row_with_entry(&l, e, i as u32, control, false, true, || {
                // An entry without its returned row is an unknown tail; create-new blocks retry.
                publish(
                    &dir,
                    &format!("entry-{i:03}.r3er"),
                    &Record::Conditional(Box::new(result.clone())),
                )?;
                Ok(())
            })?;
            result.rows.push(row.clone());
            result.raw_bytes.push(Vec::new());
            result.generations = control.generation_calls as u64;
            result.elapsed = Scalar::F64(control.start.elapsed().as_secs_f64());
            let bytes = l.tokenizer.decode_bytes(
                &row.tokens
                    .iter()
                    .copied()
                    .take_while(|id| *id >= neural::SPECIALS as u32)
                    .collect::<Vec<_>>(),
            )?;
            result.raw_bytes[i] = bytes;
            publish(
                &dir,
                &format!("generation-{i:03}.r3er"),
                &Record::Conditional(Box::new(result.clone())),
            )?;
            verification_fault(tiny, "row", control)?;
            control.check("conditional_before_teacher")?;
            let prompt = l.tokenizer.prepare(
                &e.request,
                l.model.config.context as u32,
                &l.model.config.id()?,
            )?;
            publish(
                &dir,
                &format!("teacher-entry-{i:03}.r3er"),
                &Record::Conditional(Box::new(result.clone())),
            )?;
            verification_fault(tiny, "teacher", control)?;
            let t = teacher_with_foil(
                &l,
                e,
                &prompt.token_ids,
                &row.tokens,
                control,
                Some(&p.foils[i]),
            )?;
            row.teacher = teacher_record(&t)?;
            row.diagnostic = Some(scalar_value(&t["conditional_foil"], "margin")?);
            result.rows[i] = row;
            result.teachers = control.teacher_calls as u64;
            result.elapsed = Scalar::F64(control.start.elapsed().as_secs_f64());
            publish(
                &dir,
                &format!("row-{i:03}.r3er"),
                &Record::Conditional(Box::new(result.clone())),
            )?;
            if (i + 1) % 12 == 0 {
                println!(
                    "CONDITIONAL_MODEL={model} completed={}/{} generation_calls={} teacher_forwards={} elapsed_s={:.3} step={} QUALITY_UPDATES=0",
                    i + 1,
                    p.cases.len(),
                    control.generation_calls,
                    control.teacher_calls,
                    control.start.elapsed().as_secs_f64(),
                    l.manifest.trained_steps
                );
            }
        }
        Ok(())
    })();
    if let Err(e) = &outcome {
        control.classify_error(e);
    }
    result.generations = control.generation_calls as u64;
    result.teachers = control.teacher_calls as u64;
    result.elapsed = Scalar::F64(control.start.elapsed().as_secs_f64());
    result.stop = control.observed.clone();
    result.error = outcome.as_ref().err().map(ToString::to_string);
    result.complete =
        outcome.is_ok() && result.rows.len() == p.cases.len() && control.stop.is_none();
    let exact = result
        .rows
        .iter()
        .zip(&result.raw_bytes)
        .zip(&p.cases)
        .filter(|((r, raw), e)| {
            r.completed
                && r.finish == Finish::Eos
                && r.error.is_none()
                && raw.as_slice() == e.answer.as_bytes()
        })
        .count();
    println!("CONDITIONAL_EXACT={exact}/{}", p.cases.len());
    let final_record = Record::Conditional(Box::new(result));
    publish(
        &dir,
        "publication-pending.r3er",
        &Record::VerificationPending {
            start: reference(&dir, "start.r3er")?,
            final_digest: hash(&final_record.encode()?),
        },
    )?;
    let tiny = p.cases.len() == 6;
    let saved = verification_fault(tiny, "final", control)
        .and_then(|_| publish(&dir, "final.r3er", &final_record));
    println!(
        "CONDITIONAL_FINAL NEW_GENERATIONS={} NEW_TEACHERS={} COMPLETE={} ORIGINAL_ERROR={:?} SAVE_ERROR={:?}",
        control.generation_calls,
        control.teacher_calls,
        outcome.is_ok(),
        outcome.as_ref().err(),
        saved.as_ref().err()
    );
    saved?;
    std::fs::remove_file(dir.join("publication-pending.r3er"))?;
    if let Err(e) = std::fs::File::open(&dir)?.sync_all() {
        eprintln!("PUBLICATION_COMMITTED=true CLEANUP_WARNING={e}");
    }
    outcome
}
fn conditional_report(root: &Path, control: &mut RunControl) -> Result<()> {
    // Read-only recount retains the registered execution source; a newer scorer
    // cannot authorize generation under that old plan.
    let p = conditional_plan(root, false)?;
    // The same durable admission checks protect reports; incomplete attempts never become comparisons.
    let lock = std::fs::File::open(root.join("plan.r3er"))?;
    lock.try_lock_shared()
        .map_err(|e| Error::Conflict(format!("conditional plan reader: {e}")))?;
    for model in 0..2 {
        conditional_attempt(root, &p, model, true)?;
    }
    println!(
        "CONDITIONAL_RAW_SOURCE={} RECOUNT_SOURCE={} NEW_GENERATIONS=0 NEW_TEACHERS=0",
        hex(&p.source),
        hex(&evaluator_source())
    );
    for model in 0..2 {
        control.check("conditional_recount")?;
        let dir = root.join(format!("model-{model}"));
        let Record::Conditional(r) = read_record(&dir, &reference(&dir, "final.r3er")?)? else {
            return Err(bad("conditional final kind"));
        };
        if r.plan != Some(reference(root, "plan.r3er")?)
            || r.models != p.models
            || r.model_index != model
            || r.source != p.source
        {
            return Err(bad("conditional result binding"));
        }
        let l = conditional_model(&p.models[model as usize])?;
        let mut good = [false; 144];
        let mut views = [[0usize; 5]; 6];
        let mut fields_count = BTreeMap::<String, [usize; 2]>::new();
        let mut strata = BTreeMap::<String, [usize; 2]>::new();
        let mut first = BTreeMap::<String, usize>::new();
        let mut raw_errors = BTreeMap::<String, usize>::new();
        let mut margins = [0usize; 6];
        let mut prompt_lengths = [0u64; 6];
        for (i, row) in r.rows.iter().enumerate() {
            if let Some(kind) = &row.error_class {
                *raw_errors.entry(kind.clone()).or_default() += 1;
            }
            let e = &p.cases[i];
            let raw = l.tokenizer.decode_bytes(
                &row.tokens
                    .iter()
                    .copied()
                    .take_while(|id| *id >= neural::SPECIALS as u32)
                    .collect::<Vec<_>>(),
            )?;
            if row.ordinal as usize != i || row.case != case_hash(e) || raw != r.raw_bytes[i] {
                return Err(bad("conditional row order/content/raw"));
            }
            let (actual, error) = row_output(row, &l)?;
            let v = i % 6;
            let exact = row.completed
                && !error
                && row.finish == Finish::Eos
                && actual.as_deref() == Some(&e.answer);
            good[i] = exact;
            views[v][0] += usize::from(exact);
            views[v][1] += usize::from(error);
            views[v][2] += usize::from(row.finish == Finish::Eos);
            views[v][3] += usize::from(raw.is_empty());
            views[v][4] += usize::from(row.finish == Finish::Length);
            prompt_lengths[v] += u64::from(row.prompt_len);
            if let Some(m) = &row.diagnostic {
                margins[v] += usize::from(m.finite()? > 0.);
            }
            let group = e
                .family
                .rsplit_once("/view-")
                .map_or(e.family.as_str(), |(s, _)| s)
                .to_string();
            let g = strata.entry(group).or_default();
            g[0] += usize::from(exact);
            g[1] += 1;
            for (name, value) in components(actual.as_deref(), &e.answer, &row.provided)
                .as_object()
                .into_iter()
                .flatten()
            {
                let n = fields_count.entry(name.clone()).or_default();
                n[0] += usize::from(
                    value.as_bool().unwrap_or(false) && !error && row.finish == Finish::Eos,
                );
                n[1] += 1; // Missing/invalid citation is a failure on the full panel denominator.
            }
            if !exact {
                let at = e
                    .answer
                    .as_bytes()
                    .iter()
                    .zip(&raw)
                    .position(|(a, b)| a != b)
                    .unwrap_or(e.answer.len().min(raw.len()));
                *first.entry(field_at(&e.answer, at).into()).or_default() += 1;
            }
        }
        let joint = (0..24)
            .filter(|b| good[b * 6..b * 6 + 6].iter().all(|x| *x))
            .count();
        let paired = (1..6)
            .map(|v| (0..24).filter(|b| good[b * 6] && good[b * 6 + v]).count())
            .collect::<Vec<_>>();
        println!(
            "CONDITIONAL_RECOUNT model={model} step={} complete={} generations={} teachers={} total_exact={}/144 views_exact_error_eos_empty_length={views:?} base_joint={joint}/24 paired_v0_with_v1_to_v5={paired:?} gold_foil_positive={margins:?} prompt_length_sums={prompt_lengths:?} fields={fields_count:?} strata={strata:?} first_errors={first:?} FINAL_QUALITY_GATE=NOT_APPLIED",
            p.models[model as usize].step,
            r.complete,
            r.generations,
            r.teachers,
            good.iter().filter(|x| **x).count()
        );
        println!(
            "CONDITIONAL_TOKEN_COUNTS model={model} prompt={} generated_including_eos={} teacher_targets_including_eos={} completed_rows={} planned=144 raw_error_classes={raw_errors:?}",
            prompt_lengths.iter().sum::<u64>(),
            r.rows.iter().map(|row| row.tokens.len()).sum::<usize>(),
            r.rows
                .iter()
                .map(|row| match &row.teacher {
                    TeacherRecord::Measured(t) => t.target,
                    _ => 0,
                })
                .sum::<u64>(),
            r.rows.len()
        );
    }
    Ok(())
}
fn exposure_report(root: &Path, control: &mut RunControl) -> Result<()> {
    let s = read_inputs(root)?;
    let commands = arm_commands(root, &s)?;
    let (_, last) = commands
        .last()
        .ok_or_else(|| bad("exposure endpoint absent"))?;
    let chain = lineage(root, &s, &last.terminal)?;
    let n = chain
        .last()
        .unwrap()
        .1
        .native
        .as_ref()
        .ok_or_else(|| bad("exposure native absent"))?;
    let l = resolve_native(root, &s, n, true)?;
    let episodes = s
        .train
        .iter()
        .map(|i| s.cases[*i as usize].clone())
        .collect::<Vec<_>>();
    let samples = samples(
        &episodes,
        &l.tokenizer,
        l.manifest.training.as_ref().unwrap().config.seq_len,
    )?;
    let mut groups = BTreeMap::<String, (BTreeSet<String>, usize, u64, u64)>::new();
    let key = |e: &Episode| {
        format!(
            "category={}/family={}/task={}",
            e.category,
            e.family.split('/').next().unwrap_or(""),
            e.family.rsplit('/').next().unwrap_or("")
        )
    };
    for e in &episodes {
        let g = groups.entry(key(e)).or_default();
        g.0.insert(scene(e).to_string());
        g.1 += 1;
    }
    for (_, segment) in &chain {
        for d in &segment.draws {
            control.check("exposure_recount")?;
            for i in &d.indices {
                let i = *i as usize;
                let g = groups.get_mut(&key(&episodes[i])).unwrap();
                g.2 += 1;
                g.3 += (samples[i].tokens.len() - samples[i].response_start) as u64;
            }
        }
    }
    for (key, (bases, views, draws, targets)) in groups {
        println!(
            "EXPOSURE {key} unique_bases={} pool_views={views} actual_draws={draws} target_tokens={targets}",
            bases.len()
        );
    }
    let ordinary = panel(&s, PanelKind::Ordinary)?;
    for i in &ordinary.cases {
        let e = &s.cases[*i as usize];
        if e.family.starts_with("copy/") {
            let task = e.family.rsplit('/').next().unwrap();
            let n = episodes
                .iter()
                .filter(|x| x.family.starts_with("copy/") && x.family.ends_with(task))
                .count();
            println!(
                "AUX_COVERAGE task={task} pool_views={n} observation=coverage_not_causal_proof"
            );
        }
    }
    println!(
        "EXPOSURE_RECOUNT=VERIFIED model_step={} updates={} NEW_OPTIMIZER=0",
        n.step,
        chain.iter().map(|(_, t)| t.draws.len()).sum::<usize>()
    );
    Ok(())
}
fn bridge_origins(s: &RunSnapshot, observation: Option<&Path>) -> Result<()> {
    for o in s.origins.iter().filter(|o| o.role.starts_with("bridge-")) {
        if unhex(&file_hash(Path::new(&o.original.locator))?)? != o.original.digest {
            return Err(bad("bridge frozen origin changed"));
        }
    }
    if let Some(origin) = s
        .origins
        .iter()
        .find(|o| o.role == "bridge-replacement-registration")
    {
        let Record::BridgeReplacement {
            source,
            binary,
            root,
            old_root,
            old_binary,
            preserved,
            ..
        } = Record::decode(&neural::read_bounded(
            Path::new(&origin.original.locator),
            MAX_FILE,
        )?)?
        else {
            return Err(bad("replacement registration kind"));
        };
        let inferred = s
            .origins
            .iter()
            .find(|o| o.role == "bridge-observation-final")
            .and_then(|o| Path::new(&o.original.locator).parent());
        let actual = observation
            .or(inferred)
            .ok_or_else(|| bad("replacement observation locator absent"))?;
        if actual.canonicalize()? != Path::new(&root) {
            return Err(bad("REGISTERED_OBSERVATION_ROOT_MISMATCH"));
        }
        if source != s.source
            || s.origins
                .iter()
                .find(|o| o.role == "execution-binary")
                .is_none_or(|o| o.original.digest != binary)
            || unhex(&file_hash(Path::new(&old_binary.locator))?)? != old_binary.digest
        {
            return Err(bad("replacement source/binary binding"));
        }
        for r in preserved {
            if reference(Path::new(&old_root), &r.locator)? != r {
                return Err(bad("original failed evidence changed"));
            }
        }
    }
    Ok(())
}

fn register_bridge_replacement(
    output: &Path,
    old_root: &Path,
    old_source: Hash,
    old_binary: FileRef,
) -> Result<FileRef> {
    let mut preserved = std::fs::read_dir(old_root)?
        .map(|e| {
            let e = e?;
            if !e.file_type()?.is_file() {
                return Err(bad("replacement preservation expects flat observation"));
            }
            reference(old_root, &e.file_name().to_string_lossy())
        })
        .collect::<Result<Vec<_>>>()?;
    preserved.sort_by(|a, b| a.locator.cmp(&b.locator));
    let parent = output
        .parent()
        .ok_or_else(|| bad("replacement registration root"))?
        .canonicalize()?;
    let name = output.file_name().ok_or_else(|| bad("replacement name"))?;
    let mut registered = publish(
        &parent,
        &format!("{}.r3er", name.to_string_lossy()),
        &Record::BridgeReplacement {
            source_sha: source_commit()?,
            source: evaluator_source(),
            binary: unhex(&file_hash(&std::env::current_exe()?)?)?,
            root: parent.join(name).display().to_string(),
            old_root: old_root.canonicalize()?.display().to_string(),
            old_source,
            old_binary,
            preserved,
        },
    )?;
    registered.locator = parent.join(&registered.locator).display().to_string();
    Ok(registered)
}

#[allow(clippy::too_many_arguments)] // One explicit replacement, retaining its failed predecessor.
fn bridge_observe_prepare(
    parent: &Path,
    checkpoint_path: &Path,
    data: &Path,
    conditional: &Path,
    output: &Path,
    replacement_of: &Path,
    previous_binary: &Path,
    control: &mut RunControl,
) -> Result<()> {
    if output.file_name().and_then(|n| n.to_str()) != Some("replacement-01") {
        return Err(bad(
            "only the explicitly authorized replacement-01 is permitted",
        ));
    }
    let old_attempt = read_inputs(replacement_of)?;
    let old_outcome = read_preflight_outcome(replacement_of)?
        .ok_or_else(|| bad("replacement missing failed predecessor"))?;
    if !old_attempt.bridge() || !old_attempt.historical || old_outcome.succeeded {
        return Err(bad("replacement predecessor must remain failed"));
    }
    let Record::VerificationStart(previous_start) = read_record(
        replacement_of,
        &reference(replacement_of, "preflight-start.r3er")?,
    )?
    else {
        return Err(bad("replacement old intent"));
    };
    if previous_start.binary != unhex(&file_hash(previous_binary)?)? {
        return Err(bad("replacement original observation binary"));
    }
    let registration = register_bridge_replacement(
        output,
        replacement_of,
        old_attempt.source,
        FileRef {
            locator: previous_binary.canonicalize()?.display().to_string(),
            digest: previous_start.binary,
        },
    )?;
    let old = read_inputs(parent)?;
    let commands = arm_commands(parent, &old)?;
    let command = &commands
        .last()
        .ok_or_else(|| bad("bridge parent incomplete"))?
        .1;
    let prior = close_native_inner(parent, &command.terminal.locator, control, false)?;
    let chain = lineage(parent, &old, &command.terminal)?;
    let n = chain.last().unwrap().1.native.as_ref().unwrap();
    let original = resolve_native(parent, &old, n, true)?;
    let l = checkpoint::load(checkpoint_path, Device::Cpu, true)?;
    if old_attempt.parent.model != unhex(&l.model.weight_hash()?)?
        || old_attempt.parent.file.digest != unhex(&file_hash(checkpoint_path)?)?
    {
        return Err(bad("replacement must retain exact previous parent"));
    }
    for (role, path) in [
        ("bridge-control-corpus", data.join("C-COPYMATCH.r3c")),
        ("bridge-temporal-corpus", data.join("T-TEMPORAL.r3c")),
        ("bridge-sanity-corpus", data.join("sanity.r3c")),
    ] {
        if old_attempt
            .origins
            .iter()
            .find(|o| o.role == role)
            .is_none_or(|o| {
                unhex(&file_hash(&path).unwrap_or_default()).ok() != Some(o.original.digest)
            })
        {
            return Err(bad("replacement data must retain original bytes"));
        }
    }
    let state = l
        .manifest
        .training
        .as_ref()
        .ok_or_else(|| bad("bridge parent training"))?;
    let mut a = original.manifest.training.clone().unwrap();
    let mut b = state.clone();
    a.resume_binding = None;
    b.resume_binding = None;
    if old.arm_name() != "A75-R"
        || n.step != 24310
        || n.model != unhex("dfc3efb664351578340e39d3cfc95b90f270541041d4641d72d6f41a53871b11")?
        || a != b
        || original.model.weight_hash()? != l.model.weight_hash()?
        || optimizer_hash(&original.optimizer)? != optimizer_hash(&l.optimizer)?
        || state.resume_binding.as_ref() != Some(&objective_binding(&old, state, &l.tokenizer)?)
    {
        return Err(bad("bridge exact migrated parent/objective/Adam"));
    }
    let c = data::native::read(&data.join("C-COPYMATCH.r3c"))?;
    let t = data::native::read(&data.join("T-TEMPORAL.r3c"))?;
    let sanity = data::native::read(&data.join("sanity.r3c"))?;
    let auth = old
        .authorization
        .as_ref()
        .ok_or_else(|| bad("bridge parent pools"))?;
    let anchors = auth.pools[0]
        .iter()
        .map(|i| old.cases[old.train[*i as usize] as usize].clone())
        .collect::<Vec<_>>();
    if c.train.len() != 2560
        || t.train.len() != 2560
        || t.validation.len() != 256
        || sanity.validation.len() != 64
        || data::native::ordered_bytes(&c.train[..2048]) != data::native::ordered_bytes(&anchors)
        || data::native::ordered_bytes(&t.train[..2048]) != data::native::ordered_bytes(&anchors)
        || data::native::ordered_bytes(&c.validation) != data::native::ordered_bytes(&t.validation)
    {
        return Err(bad("bridge frozen native pools"));
    }
    for (a, b) in c.train[2048..].iter().zip(&t.train[2048..]) {
        data::bridge_pair(a, b, &l.tokenizer)?;
    }
    let cf = samples(&c.train[2048..], &l.tokenizer, state.config.seq_len)?;
    let tf = samples(&t.train[2048..], &l.tokenizer, state.config.seq_len)?;
    if cf.iter().zip(&tf).any(|(a, b)| {
        a.response_start != b.response_start
            || a.tokens[a.response_start..] != b.tokens[b.response_start..]
    }) {
        return Err(bad("replacement prompt length/target token mismatch"));
    }
    for e in &t.validation {
        data::bridge_support(e)?;
    }
    for (name, cases) in [
        ("retained-control", &c.train[2048..]),
        ("retained-temporal", &t.train[2048..]),
        ("retained-dev", t.validation.as_slice()),
        ("retained-sanity", sanity.validation.as_slice()),
    ] {
        bridge_audit(name, cases, &l, true)?;
    }
    let conditional_plan = conditional_plan(conditional, false)?;
    for (kind, current) in [
        (PanelKind::Dev, &t.validation),
        (PanelKind::Sanity, &sanity.validation),
        (PanelKind::Conditional, &conditional_plan.cases),
    ] {
        let retained = panel(&old_attempt, kind)?
            .cases
            .iter()
            .map(|i| old_attempt.cases[*i as usize].clone())
            .collect::<Vec<_>>();
        if data::native::ordered_bytes(current) != data::native::ordered_bytes(&retained) {
            return Err(bad("replacement frozen membership/order changed"));
        }
    }
    let retained_train = old_attempt
        .train
        .iter()
        .map(|i| old_attempt.cases[*i as usize].clone())
        .collect::<Vec<_>>();
    if data::native::ordered_bytes(&t.train) != data::native::ordered_bytes(&retained_train) {
        return Err(bad("replacement original focus/anchor membership"));
    }
    let mut s = old.clone();
    s.contract = BRIDGE_CONTRACT.into();
    s.source = evaluator_source();
    s.historical = true;
    s.authorization = None;
    s.objective = None;
    s.tape.clear();
    s.eval_steps.clear();
    s.lr_policy = 2;
    s.lr_offset = 0;
    s.purpose = RunPurpose::LrSplit;
    s.cases = t.train;
    s.train = (0..2560).collect();
    s.panels.clear();
    for kind in [
        PanelKind::LegacyDev,
        PanelKind::Cross,
        PanelKind::Ordinary,
        PanelKind::Watch,
    ] {
        let old_kind = if kind == PanelKind::LegacyDev {
            PanelKind::Dev
        } else {
            kind
        };
        let mut indices = Vec::new();
        for i in &panel(&old, old_kind)?.cases {
            let e = &old.cases[*i as usize];
            let index = if let Some(i) = s
                .cases
                .iter()
                .position(|other| case_hash(other) == case_hash(e))
            {
                i
            } else {
                s.cases.push(e.clone());
                s.cases.len() - 1
            };
            indices.push(index as u32);
        }
        s.panels.push(PanelSpec {
            kind,
            dataset: panel(&old, old_kind)?.dataset,
            cases: indices,
        });
    }
    for (kind, episodes) in [
        (PanelKind::Dev, t.validation),
        (PanelKind::Sanity, sanity.validation),
        (PanelKind::Conditional, conditional_plan.cases),
    ] {
        let indices = (s.cases.len() as u32..(s.cases.len() + episodes.len()) as u32).collect();
        let dataset = hash(&data::native::ordered_bytes(&episodes));
        s.cases.extend(episodes);
        s.panels.push(PanelSpec {
            kind,
            dataset,
            cases: indices,
        });
    }
    s.origins.clear();
    s.origins.push(Origin {
        role: "bridge-replacement-registration".into(),
        original: FileRef {
            locator: registration.locator,
            digest: registration.digest,
        },
    });
    for (role, path) in [
        ("execution-binary", std::env::current_exe()?),
        ("bridge-parent-inputs", parent.join("inputs.r3er")),
        (
            "bridge-parent-terminal",
            parent.join(&command.terminal.locator),
        ),
        (
            "bridge-parent-command",
            parent.join(command_locator(&command.terminal)?),
        ),
        ("bridge-parent-native", checkpoint_path.to_path_buf()),
        ("bridge-control-corpus", data.join("C-COPYMATCH.r3c")),
        ("bridge-temporal-corpus", data.join("T-TEMPORAL.r3c")),
        ("bridge-sanity-corpus", data.join("sanity.r3c")),
        ("bridge-conditional-plan", conditional.join("plan.r3er")),
    ] {
        s.origins.push(Origin {
            role: role.into(),
            original: FileRef {
                locator: path.canonicalize()?.display().to_string(),
                digest: unhex(&file_hash(&path)?)?,
            },
        });
    }
    let mut identity = BRIDGE_CONTRACT.as_bytes().to_vec();
    identity.extend(s.source);
    for o in &s.origins {
        o.original.encode(&mut identity);
    }
    s.run = hash(&identity);
    s.policy = s.run;
    s.frozen = hash(&data::native::ordered_bytes(&s.cases));
    s.parent = n.clone();
    s.parent.run = s.run;
    s.parent.segment = 0;
    s.parent.file = FileRef {
        locator: "parent.r3m".into(),
        digest: unhex(&file_hash(checkpoint_path)?)?,
    };
    s.validate()?;
    std::fs::create_dir(output)?;
    std::fs::copy(checkpoint_path, output.join("parent.r3m"))?;
    std::fs::copy(
        parent.join(command_locator(&command.terminal)?),
        output.join("parent-command.r3er"),
    )?;
    publish(output, "inputs.r3er", &Record::Inputs(Box::new(s.clone())))?;
    resolve_native(output, &s, &s.parent, true)?;
    println!(
        "BRIDGE_REGISTERED_OBSERVATION binding={} model={} parent_step={} prior_recount={:?} NEW_SMALL_UPDATES=0",
        hex(&s.binding()),
        hex(&s.parent.model),
        s.parent.step,
        prior.panels
    );
    Ok(())
}

fn bridge_observe(root: &Path, control: &mut RunControl) -> Result<()> {
    let s = read_inputs(root)?;
    if !s.bridge() || !s.historical || s.source != evaluator_source() {
        return Err(bad("bridge observation source/mode"));
    }
    if !s
        .origins
        .iter()
        .any(|o| o.role == "bridge-replacement-registration")
    {
        return Err(bad("bridge requires explicit replacement registration"));
    }
    bridge_origins(&s, Some(root))?;
    if read_preflight_outcome(root)?.is_some() {
        return Err(bad("bridge observation already attempted"));
    }
    let binary = unhex(&file_hash(&std::env::current_exe()?)?)?;
    if s.origins
        .iter()
        .find(|o| o.role == "execution-binary")
        .is_none_or(|o| o.original.digest != binary)
    {
        return Err(bad("bridge binary changed"));
    }
    let kinds = [PanelKind::Sanity, PanelKind::Dev];
    control.teacher_limit = 64;
    let order = kinds
        .iter()
        .enumerate()
        .flat_map(|(arm, k)| {
            panel(&s, *k)
                .unwrap()
                .cases
                .iter()
                .map(move |i| (arm as u8, *i))
        })
        .collect::<Vec<_>>();
    let start = VerificationStart {
        publication_v2: true,
        scope: 3,
        root: root.canonicalize()?.display().to_string(),
        pair: s.run,
        source: s.source,
        binary,
        inputs: [
            reference(root, "inputs.r3er")?,
            reference(root, "inputs.r3er")?,
        ],
        commands: [
            reference(root, "parent-command.r3er")?,
            reference(root, "parent-command.r3er")?,
        ],
        natives: [
            reference(root, "parent.r3m")?,
            reference(root, "parent.r3m")?,
        ],
        generation_limit: order.len() as u64,
        order,
        seconds_limit: 1800,
    };
    finish_verification_attempt(
        root,
        start.clone(),
        s.tiny_spec,
        control,
        |control, returned| {
            let l = resolve_native(root, &s, &s.parent, true)?;
            let sr = reference(root, "preflight-start.r3er")?;
            let mut panels = Vec::new();
            for kind in kinds {
                let mut rows = Vec::new();
                for &ordinal in &panel(&s, kind)?.cases {
                    let index = returned.len();
                    let elapsed = control.start.elapsed().as_secs_f64();
                    let row = evaluate_row_with_entry(
                        &l,
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
                                    start: sr.clone(),
                                    index: index as u32,
                                    row: None,
                                    elapsed: Scalar::F64(elapsed),
                                }),
                            )
                            .map(|_| ())
                        },
                    )?;
                    returned.push(row.clone());
                    publish(
                        root,
                        &format!("preflight-row-{index:02}.r3er"),
                        &Record::VerificationRow(VerificationRow {
                            start: sr.clone(),
                            index: index as u32,
                            row: Some(row.clone()),
                            elapsed: Scalar::F64(control.start.elapsed().as_secs_f64()),
                        }),
                    )?;
                    rows.push(row);
                    control.check("bridge_observation_returned")?;
                }
                let e = bridge_payload(&s, &s.parent, kind, rows);
                print_anchor_panel(&s, &e, &l)?;
                panels.push(publish(
                    root,
                    &format!("{}.r3er", kind.name()),
                    &Record::Evaluation(e),
                )?);
            }
            collect_bridge_teacher(root, "parent-probe", &s, &s.parent, &l, control)?;
            let teacher = read_verified_bridge_teacher(root, "parent-probe", &s, &s.parent)?;
            publish(
                root,
                "preflight-proof.r3er",
                &Record::Preflight(PreflightReceipt {
                    teacher_evidence: Some(teacher.files),
                    pair: start.pair,
                    source: start.source,
                    binary: start.binary,
                    commands: start.commands.clone(),
                    panels,
                    elapsed: Scalar::F64(control.start.elapsed().as_secs_f64()),
                    generations: control.generation_calls as u64,
                }),
            )?;
            Ok(())
        },
    )
}
fn bridge_payload(
    s: &RunSnapshot,
    n: &CheckpointRef,
    kind: PanelKind,
    rows: Vec<EvalRow>,
) -> EvalPayload {
    EvalPayload {
        run: s.run,
        binding: s.binding(),
        source: s.source,
        model: n.model,
        tokenizer: n.tokenizer,
        architecture: n.architecture,
        step: n.step,
        new_updates: n.step - s.parent.step,
        kind,
        expected: panel(s, kind).unwrap().cases.len() as u32,
        rows,
    }
}

fn bridge_prepare(observation: &Path, output: &Path, control: &mut RunControl) -> Result<()> {
    let old = read_inputs(observation)?;
    bridge_origins(&old, Some(observation))?;
    read_preflight_outcome(observation)?
        .ok_or_else(|| bad("bridge parent observation absent"))?
        .require_current_success()?;
    if !old.bridge() || !old.historical || old.source != evaluator_source() {
        return Err(bad("bridge parent observation identity"));
    }
    let l = resolve_native(observation, &old, &old.parent, true)?;
    let state = l.manifest.training.as_ref().unwrap();
    let Record::Evaluation(dev) = read_record(observation, &reference(observation, "dev.r3er")?)?
    else {
        return Err(bad("bridge baseline raw"));
    };
    let baseline = rescore(&old, &dev, &l, true)?;
    let watch = if old.tiny_spec {
        // Explicit small test panel; the authorization and admission below are shared.
        baseline.clone()
    } else {
        let parent = PathBuf::from(
            &old.origins
                .iter()
                .find(|o| o.role == "bridge-parent-inputs")
                .unwrap()
                .original
                .locator,
        );
        let parent = parent.parent().unwrap();
        let prior = read_inputs(parent)?;
        let closed = arm_commands(parent, &prior)?;
        let previous = close_native_inner(
            parent,
            &closed.last().unwrap().1.terminal.locator,
            control,
            false,
        )?;
        previous
            .panels
            .iter()
            .find(|(k, _)| *k == PanelKind::Watch)
            .unwrap()
            .1
            .clone()
    };
    let registered = output
        .parent()
        .ok_or_else(|| bad("bridge root"))?
        .canonicalize()?
        .join(output.file_name().ok_or_else(|| bad("bridge name"))?);
    let proof = reference(observation, "preflight-final.r3er")?;
    let mut key = BRIDGE_CONTRACT.as_bytes().to_vec();
    key.extend(old.binding());
    key.extend(proof.digest);
    string(&mut key, &registered.display().to_string());
    let pair = hash(&key);
    let mut registrations = Vec::new();
    for purpose in [RunPurpose::LrContinuous, RunPurpose::LrSplit] {
        let mut s = old.clone();
        s.historical = false;
        s.purpose = purpose;
        let name = s.arm_name();
        let train = if old.tiny_spec {
            old.train
                .iter()
                .map(|i| old.cases[*i as usize].clone())
                .collect::<Vec<_>>()
        } else {
            let source = origin_path(
                &old,
                if purpose == RunPurpose::LrContinuous {
                    "bridge-control-corpus"
                } else {
                    "bridge-temporal-corpus"
                },
            )?;
            data::native::read(&source)?.train
        };
        for (&ordinal, e) in s.train.iter().zip(&train) {
            s.cases[ordinal as usize] = e.clone();
        }
        s.origins.push(Origin {
            role: "bridge-observation-final".into(),
            original: FileRef {
                locator: observation
                    .join("preflight-final.r3er")
                    .canonicalize()?
                    .display()
                    .to_string(),
                digest: proof.digest,
            },
        });
        s.origins.push(Origin {
            role: "cooldown-registered-root".into(),
            original: FileRef {
                locator: registered.display().to_string(),
                digest: pair,
            },
        });
        let mut id = key.clone();
        id.push(purpose.tag());
        s.run = hash(&id);
        s.policy = s.run;
        s.parent.run = s.run;
        s.baseline = [baseline.exact, watch.exact, baseline.errors + watch.errors];
        s.anchor_floor = 178;
        let anchor_count = if s.tiny_spec { 1 } else { 2048 };
        let horizon: usize = if s.tiny_spec { 2 } else { 512 };
        let pools = [
            (0..anchor_count).collect::<Vec<_>>(),
            (anchor_count..train.len() as u32).collect::<Vec<_>>(),
        ];
        let framed = samples(&train, &l.tokenizer, state.config.seq_len)?;
        s.tape = anchor_tape_through(&pools, 6, s.parent.counters[2], &framed, horizon as u64)?.0;
        s.eval_steps = vec![(horizon / 2) as u32, horizon as u32];
        s.authorization = Some(AnchorAuthorization {
            pair,
            baseline: proof.digest,
            expected_parent: s.parent.file.digest,
            anchors: 6,
            pools,
        });
        let mut exposures = vec![0; train.len()];
        for d in &s.tape {
            for i in &d.indices {
                exposures[*i as usize] += 1;
            }
        }
        if exposures[anchor_count as usize..]
            .iter()
            .any(|n| *n != if s.tiny_spec { 4 } else { 2 })
        {
            return Err(bad("bridge focus twice exact exposure"));
        }
        s.validate()?;
        for n in [1, horizon / 2, horizon] {
            let mut future = state.clone();
            fork_budget(
                &mut future,
                s.parent.step,
                s.parent.counters[0],
                horizon,
                2_000_000,
            )?;
            future.step += n;
            future.consumed_tokens += s.tape[..n].iter().map(|d| d.input).sum::<u64>();
            future.target_tokens += s.tape[..n].iter().map(|d| d.target).sum::<u64>();
            future.sampler_state = s.tape[n - 1].sampler;
            future.resume_binding = Some(objective_binding(&s, &future, &l.tokenizer)?);
            let mut m = l.manifest.clone();
            m.training = Some(future);
            checkpoint::validate_metadata(&m, &l.tokenizer)?;
        }
        println!(
            "BRIDGE_AUTHORIZATION arm={name} planned_updates={horizon} focus_exposure={} actual_lr_bits={} inherited_config_lr={} first_target_weight={} input={} target={} parent_step={} model={} Adam={} source={} binding={} ACTUAL_UPDATES=0",
            horizon * 2,
            1e-4f64.to_bits(),
            state.config.lr,
            state.config.first_target_weight,
            s.tape.iter().map(|d| d.input).sum::<u64>(),
            s.tape.iter().map(|d| d.target).sum::<u64>(),
            s.parent.step,
            hex(&s.parent.model),
            hex(&s.parent.adam.unwrap()),
            hex(&s.source),
            hex(&s.binding())
        );
        registrations.push(s);
    }
    control.check("bridge_register")?;
    std::fs::create_dir(output)?;
    for s in registrations {
        let root = output.join(s.arm_name());
        std::fs::create_dir(&root)?;
        std::fs::copy(observation.join("parent.r3m"), root.join("parent.r3m"))?;
        publish(&root, "inputs.r3er", &Record::Inputs(Box::new(s.clone())))?;
        resolve_native(&root, &s, &s.parent, true)?;
    }
    Ok(())
}

fn bridge_observe_report(root: &Path, control: &mut RunControl) -> Result<()> {
    let s = read_inputs(root)?;
    bridge_origins(&s, Some(root))?;
    let outcome =
        read_preflight_outcome(root)?.ok_or_else(|| bad("bridge observation intent missing"))?;
    if !s.bridge() || !s.historical {
        return Err(bad("bridge observation report mode"));
    }
    let l = resolve_native(root, &s, &s.parent, true)?;
    let previous = s
        .origins
        .iter()
        .find(|o| o.role == "bridge-replacement-registration")
        .map(|o| -> Result<_> {
            let Record::BridgeReplacement {
                old_root,
                old_source,
                ..
            } = Record::decode(&neural::read_bounded(
                Path::new(&o.original.locator),
                MAX_FILE,
            )?)?
            else {
                return Err(bad("replacement report kind"));
            };
            let old_root = PathBuf::from(old_root);
            let old = read_inputs(&old_root)?;
            if old.source != old_source
                || old.parent.file.digest != s.parent.file.digest
                || old.parent.model != s.parent.model
                || old.parent.step != s.parent.step
            {
                return Err(bad("replacement prior observation identity"));
            }
            Ok((old_root, old))
        })
        .transpose()?;
    for kind in [PanelKind::Sanity, PanelKind::Dev] {
        let Record::Evaluation(e) =
            read_record(root, &reference(root, &format!("{}.r3er", kind.name()))?)?
        else {
            return Err(bad("bridge observation panel"));
        };
        print_anchor_panel(&s, &e, &l)?;
        if let Some((old_root, old)) = &previous {
            if !old_root.join(format!("{}.r3er", kind.name())).exists() {
                println!(
                    "PREDECESSOR_RAW_PARITY panel={} NOT_RUN_MISSING_ORIGINAL_RAW OLD_COMMAND_UNCHANGED=true",
                    kind.name()
                );
                continue;
            }
            let Record::Evaluation(prior) = read_record(
                old_root,
                &reference(old_root, &format!("{}.r3er", kind.name()))?,
            )?
            else {
                return Err(bad("previous raw panel kind"));
            };
            rescore(old, &prior, &l, true)?;
            if prior.rows.len() != e.rows.len()
                || prior.rows.iter().zip(&e.rows).any(|(a, b)| {
                    a.ordinal != b.ordinal
                        || a.case != b.case
                        || a.prompt != b.prompt
                        || a.tokens != b.tokens
                        || a.eos != b.eos
                        || a.finish != b.finish
                        || a.error != b.error
                        || a.completed != b.completed
                        || a.interruption != b.interruption
                })
            {
                return Err(bad("replacement fixed-weight raw token/EOS/error mismatch"));
            }
            println!(
                "PREDECESSOR_RAW_PARITY panel={} equal={}/{} OLD_COMMAND_REMAINS_FAILED=true NEW_GENERATIONS=0 NEW_TEACHERS=0",
                kind.name(),
                e.rows.len(),
                e.expected
            );
        }
        if kind == PanelKind::Sanity {
            for view in 0..4 {
                let mut count = [0u64; 2];
                for row in e.rows.iter().skip(view).step_by(4) {
                    let (actual, abnormal) = row_output(row, &l)?;
                    count[0] += u64::from(
                        !abnormal
                            && actual.as_deref()
                                == Some(s.cases[row.ordinal as usize].answer.as_str()),
                    );
                    count[1] += 1;
                }
                println!(
                    "BRIDGE_SANITY condition={} full={count:?}",
                    [
                        "A-single",
                        "B-distinct-entity",
                        "C-same-entity",
                        "D-past-question"
                    ][view]
                );
            }
        }
    }
    print_bridge_teacher(
        &read_verified_bridge_teacher(root, "parent-probe", &s, &s.parent)?,
        s.parent.step,
    )?;
    println!(
        "BRIDGE_OBSERVATION_READ_ONLY generation_entries_known={} elapsed_lower_bound={} historical_attempt_succeeded={} UNKNOWN_TAIL={} NEW_GENERATIONS={} NEW_TEACHERS={} NEW_SMALL_UPDATES=0 NOT_AUTHORIZED_TO_RESUME=true",
        outcome.entries,
        outcome.elapsed,
        outcome.succeeded,
        !outcome.succeeded,
        control.generation_calls,
        control.teacher_calls
    );
    Ok(())
}

fn bridge_probe_indices(s: &RunSnapshot) -> Result<Vec<u32>> {
    let mut out = Vec::new();
    for mut indices in [
        s.train
            .iter()
            .skip(if s.tiny_spec { 0 } else { 2048 })
            .copied()
            .collect::<Vec<_>>(),
        panel(s, PanelKind::Dev)?.cases.clone(),
    ] {
        indices.sort_by_key(|i| hash(s.cases[*i as usize].id.as_bytes()));
        indices.truncate(if s.tiny_spec { 1 } else { 32 });
        out.extend(indices);
    }
    Ok(out)
}

fn read_absolute(r: &FileRef) -> Result<Record> {
    let bytes = neural::read_bounded(Path::new(&r.locator), MAX_FILE)?;
    if hash(&bytes) != r.digest {
        return Err(bad("screen immutable reference changed"));
    }
    Record::decode(&bytes)
}
fn verified_artifact_audit(file: &FileRef) -> Result<ArtifactAudit> {
    verify_audit_publication(file)?;
    reaudit_artifact(file)
}

// Publication admission is independent of a computed positive result. Legacy audit
// data may be recounted explicitly, but cannot acquire a retrospective commit proof.
fn verify_audit_publication(file: &FileRef) -> Result<()> {
    let root = Path::new(&file.locator)
        .parent()
        .ok_or_else(|| bad("audit root"))?;
    if root.join("audit-publication-pending.r3er").exists() {
        return Err(bad(
            "AUDIT_PUBLICATION_PENDING: admission blocked; model_calls=0",
        ));
    }
    let Record::VerificationPending {
        start,
        final_digest,
    } = read_record(root, &reference(root, "audit-publication.r3er")?)?
    else {
        return Err(bad("audit publication identity"));
    };
    if final_digest != file.digest {
        return Err(bad("audit publication final digest"));
    }
    let Record::ArtifactAudit(initial) = read_record(root, &start)? else {
        return Err(bad("audit publication start"));
    };
    let Record::ArtifactAudit(final_record) = read_absolute(file)? else {
        return Err(bad("audit publication final"));
    };
    if initial.input != final_record.input
        || initial.native != final_record.native
        || initial.source != final_record.source
        || initial.binary != final_record.binary
        || initial.originals != final_record.originals
        || initial.selection != final_record.selection
        || initial.complete
    {
        return Err(bad("audit attempt/model/input identity"));
    }
    Ok(())
}

fn publish_audit_final(root: &Path, audit: &ArtifactAudit) -> Result<()> {
    let final_record = Record::ArtifactAudit(Box::new(audit.clone()));
    let identity = Record::VerificationPending {
        start: reference(root, "audit-start.r3er")?,
        final_digest: hash(&final_record.encode()?),
    };
    publish(root, "audit-publication-pending.r3er", &identity)?;
    publish(root, "audit-final.r3er", &final_record)?;
    publish(root, "audit-publication.r3er", &identity)?;
    // Same linearization as preflight: both publications have synced before unlink.
    std::fs::remove_file(root.join("audit-publication-pending.r3er"))?;
    if let Err(e) = std::fs::File::open(root).and_then(|f| f.sync_all()) {
        eprintln!("PUBLICATION_COMMITTED=true CLEANUP_WARNING={e}");
    }
    verify_audit_publication(&absolute_reference(&root.join("audit-final.r3er"))?)
}

fn reaudit_artifact(file: &FileRef) -> Result<ArtifactAudit> {
    let Record::ArtifactAudit(a) = read_absolute(file)? else {
        return Err(bad("screen requires artifact audit"));
    };
    let s = match read_absolute(&a.input)? {
        Record::Inputs(s) => s,
        _ => return Err(bad("audit input")),
    };
    if !s.bridge()
        || s.arm_name() != "C-COPYMATCH"
        || s.tiny_spec
        || !a.complete
        || !a.stop.is_empty()
        || a.error.is_some()
        || a.calls != [64, 64, 60, 60]
        || a.fresh.len() != 64
        || a.numeric.len() != 20
        || a.panels.len() != 5
        || a.selection != bridge_diagnostic_selection(&s)?
    {
        return Err(bad("screen diagnostic incomplete or failed"));
    }
    for r in &a.originals {
        if absolute_reference(Path::new(&r.locator))? != *r {
            return Err(bad("audit original changed"));
        }
    }
    for model in 0..2u8 {
        if a.fresh
            .iter()
            .filter(|(m, _)| *m == model)
            .map(|(_, r)| r.ordinal)
            .collect::<Vec<_>>()
            != a.selection
            || a.numeric
                .iter()
                .filter(|r| r.model == model)
                .map(|r| r.ordinal)
                .collect::<Vec<_>>()
                != a.selection[..10]
        {
            return Err(bad("audit ordered numeric/generation cases"));
        }
    }
    let original_root = Path::new(&a.input.locator)
        .parent()
        .ok_or_else(|| bad("audit root"))?;
    if original_root.join("segment-01/terminal.r3er").exists()
        || original_root.join("segment-01/command.r3er").exists()
    {
        return Err(bad("historical failure changed"));
    }
    let l = resolve_native(original_root, &s, &a.native, true)?;
    for (kind, reported) in &a.panels {
        let name = format!("segment-01/{}-0512.r3er", kind.name());
        let Record::Evaluation(raw) =
            read_record(original_root, &reference(original_root, &name)?)?
        else {
            return Err(bad("audit original raw"));
        };
        let score = rescore(&s, &raw, &l, true)?;
        let mut expected = Vec::new();
        let mut actual = Vec::new();
        score.encode(&mut expected);
        reported.encode(&mut actual);
        if expected != actual || raw.source != s.source || raw.step != a.native.step {
            return Err(bad("audit raw score/source/step changed"));
        }
    }
    Ok(*a)
}
fn screen_selection(s: &RunSnapshot) -> Result<Vec<u32>> {
    let mut all = Vec::new();
    for kind in [
        PanelKind::LegacyDev,
        PanelKind::Cross,
        PanelKind::Ordinary,
        PanelKind::Dev,
    ] {
        let mut groups: BTreeMap<String, Vec<u32>> = BTreeMap::new();
        for &i in &panel(s, kind)?.cases {
            let e = &s.cases[i as usize];
            if kind == PanelKind::Ordinary {
                if !e.family.starts_with("copy/") {
                    groups.entry(e.category.to_string()).or_default().push(i);
                }
            } else {
                groups.entry(scene(e).into()).or_default().push(i);
            }
        }
        if kind == PanelKind::Ordinary {
            for v in groups.values_mut() {
                v.sort_by_key(|i| hash(s.cases[*i as usize].id.as_bytes()));
            }
            let mut selected = Vec::new();
            for index in 0..32 {
                for v in groups.values() {
                    if let Some(i) = v.get(index) {
                        selected.push(*i);
                        if selected.len() == 32 {
                            break;
                        }
                    }
                }
                if selected.len() == 32 {
                    break;
                }
            }
            if selected.len() != 32 {
                return Err(bad("screen QA32 membership"));
            }
            all.extend(selected);
        } else {
            if groups.values().any(|v| v.len() != 4) {
                return Err(bad("screen base must retain four views"));
            }
            let mut groups = groups.into_iter().collect::<Vec<_>>();
            groups.sort_by_key(|(k, _)| {
                hash(format!("bounded-screen-v1/{}/{k}", kind.name()).as_bytes())
            });
            if groups.len() < 16 {
                return Err(bad("screen 16 bases unavailable"));
            }
            all.extend(groups.into_iter().take(16).flat_map(|(_, v)| v));
        }
    }
    if all.len() != 224 || all.iter().collect::<BTreeSet<_>>().len() != 224 {
        return Err(bad("screen unique224"));
    }
    Ok(all)
}
fn screen_scores(s: &RunSnapshot, e: &EvalPayload, l: &Loaded) -> Result<[Score; 4]> {
    rescore(s, e, l, true)?;
    let mut result: [Score; 4] = Default::default();
    let mut start = 0;
    for (i, count) in (if s.tiny_spec {
        [1, 1, 1, 1]
    } else {
        [64, 64, 32, 64]
    })
    .into_iter()
    .enumerate()
    {
        let mut subset = s.clone();
        let spec = subset
            .panels
            .iter_mut()
            .find(|p| p.kind == PanelKind::Screen)
            .ok_or_else(|| bad("screen panel"))?;
        spec.cases = spec.cases[start..start + count].to_vec();
        let mut raw = e.clone();
        raw.rows = raw.rows[start..start + count].to_vec();
        raw.expected = count as u32;
        raw.binding = subset.binding();
        result[i] = rescore(&subset, &raw, l, true)?;
        start += count;
    }
    Ok(result)
}
fn screen_policy(
    parent: &[Score; 4],
    now: &[Score; 4],
    mut streak: [u64; 3],
    updates: u64,
) -> ([u64; 3], Option<&'static str>) {
    let loss: [u64; 3] = std::array::from_fn(|i| parent[i].exact.saturating_sub(now[i].exact));
    for i in 0..3 {
        streak[i] = if loss[i] >= if i == 2 { 4 } else { 8 } {
            streak[i] + 1
        } else {
            0
        };
    }
    if loss[0] >= 12 || loss[1] >= 12 || streak.iter().any(|n| *n >= 2) {
        return (streak, Some("QUALITY_REGRESSION_STOP"));
    }
    let errors = now.iter().map(|s| s.errors).sum::<u64>();
    if errors >= 5 && errors >= parent.iter().map(|s| s.errors).sum::<u64>() + 4 {
        return (streak, Some("GENERATION_ERROR_REGRESSION_STOP"));
    }
    if matches!(updates, 128 | 256)
        && !(now[3].exact >= parent[3].exact + 8
            && now[3].base[0] >= 2
            && loss[0] <= 4
            && loss[1] <= 4
            && loss[2] <= 2)
    {
        return (streak, Some("NO_SUFFICIENT_TRANSFER_SIGNAL_WITHIN_BUDGET"));
    }
    (streak, None)
}
fn screen_parent(s: &RunSnapshot) -> Result<(PathBuf, EvalPayload, f64)> {
    let a = s
        .origins
        .iter()
        .find(|o| o.role == "bounded-audit")
        .ok_or_else(|| bad("screen audit binding"))?;
    let audit_root = Path::new(&a.original.locator)
        .parent()
        .ok_or_else(|| bad("screen audit root"))?;
    let Record::ScreenRegistration {
        root,
        input,
        audit,
        observation,
        parent,
        preparation,
    } = read_record(
        audit_root,
        &reference(audit_root, "t-screen-registration.r3er")?,
    )?
    else {
        return Err(bad("screen registration"));
    };
    let root = PathBuf::from(root);
    if root.canonicalize()? != root
        || reference(&root, "inputs.r3er")?.digest != input
        || audit != a.original
        || read_inputs(&root)?.binding() != s.binding()
    {
        return Err(bad("screen registered root/input"));
    }
    let proof = s
        .origins
        .iter()
        .find(|o| o.role == "bridge-observation-final")
        .ok_or_else(|| bad("screen parent proof"))?;
    if proof.original != observation {
        return Err(bad("screen observation identity"));
    }
    let Record::Evaluation(raw) = read_absolute(&parent)? else {
        return Err(bad("screen parent raw"));
    };
    let preparation = preparation.finite()?;
    if preparation < 0. {
        return Err(bad("screen preparation clock"));
    }
    Ok((root, raw, preparation))
}
fn screen_budget(root: &Path, s: &RunSnapshot) -> Result<(u64, usize, f64)> {
    let (registered, baseline, preparation) = screen_parent(s)?;
    if root.canonicalize()? != registered {
        return Err(bad("screen moved outside registration"));
    }
    let a = verified_artifact_audit(
        &s.origins
            .iter()
            .find(|o| o.role == "bounded-audit")
            .unwrap()
            .original,
    )?;
    for o in s
        .origins
        .iter()
        .filter(|o| o.role.starts_with("bounded-parent-raw-"))
    {
        if absolute_reference(Path::new(&o.original.locator))? != o.original {
            return Err(bad("original parent raw changed"));
        }
    }
    bridge_origins(s, None)?;
    let observation = origin_path(s, "bridge-observation-final")?;
    let observation = observation.parent().unwrap();
    let prior = read_inputs(observation)?;
    bridge_origins(&prior, Some(observation))?;
    read_preflight_outcome(observation)?
        .ok_or_else(|| bad("screen parent proof absent"))?
        .require_current_success()?;
    let l = resolve_native(root, s, &s.parent, true)?;
    if prior.parent.model != s.parent.model
        || prior.parent.adam != s.parent.adam
        || baseline.step != s.parent.step
        || baseline.new_updates != 0
    {
        return Err(bad("screen unchanged parent"));
    }
    screen_scores(s, &baseline, &l)?;
    if panel(s, PanelKind::Screen)?.cases != screen_selection(s)? {
        return Err(bad("screen metadata selection changed"));
    }
    let original = origin_path(s, "bounded-prior-inputs")?;
    let old = read_inputs(original.parent().unwrap())?;
    if old.parent.model != s.parent.model
        || old.parent.adam != s.parent.adam
        || old.parent.counters != s.parent.counters
        || old.cases.iter().map(case_hash).collect::<Vec<_>>()
            != s.cases.iter().map(case_hash).collect::<Vec<_>>()
        || old.tape.iter().zip(&s.tape).any(|(a, b)| {
            a.indices != b.indices
                || a.input != b.input
                || a.target != b.target
                || a.sampler != b.sampler
        })
        || old.tape.len() != s.tape.len()
    {
        return Err(bad("screen original T data/tape changed"));
    }
    let mut updates = 0;
    let mut generations = a.calls[0] as usize;
    let mut seconds = a.elapsed.finite()? + preparation + 240.;
    let mut teachers = a.calls[2];
    for (t, c) in arm_commands(root, s)? {
        updates += t.draws.len() as u64;
        generations += t.generations as usize;
        teachers += t.teachers;
        seconds += c.elapsed.finite()? + 120.;
    }
    if updates > 512 || generations > 4096 || teachers > 192 || seconds >= 7200. {
        return Err(bad("bounded screen global budget"));
    }
    println!(
        "SCREEN_BUDGET prior_updates={updates} generations={generations}/4096 forwards={teachers}/192 charged_seconds={seconds}/7200 cleanup_reservation=120"
    );
    Ok((updates, generations, seconds))
}
fn screen_report(root: &Path, terminal: &str, control: &mut RunControl) -> Result<()> {
    let s = read_inputs(root)?;
    if !s.screen() {
        return Err(bad("report requires bounded screen"));
    }
    let (registered, _, _) = screen_parent(&s)?;
    if root.canonicalize()? != registered {
        return Err(bad("screen report registered root"));
    }
    let reference = reference(root, terminal)?;
    let chain = lineage(root, &s, &reference)?;
    verify_ancestor_commands(root, &s, &chain)?;
    let history = verified_history(root, &s, &chain)?;
    let t = &chain.last().unwrap().1;
    let Record::Command(c) =
        read_record(root, &self::reference(root, &command_locator(&reference)?)?)?
    else {
        return Err(bad("screen command absent"));
    };
    if c.run != s.run
        || c.binding != s.binding()
        || c.terminal != reference
        || c.error.is_some()
        || t.save_error.is_some()
        || c.stop != t.stop
        || root.join("close-stop.r3er").exists()
    {
        return Err(bad("screen execution/storage/cancel error"));
    }
    if history.quality {
        if !t.stop.contains(&StopReason::QualityGuard)
            || t.stop
                .iter()
                .any(|v| !matches!(v, StopReason::QualityGuard | StopReason::TimeBudget))
            || t.resume
            || t.complete
            || t.candidate
            || c.status != CommandStatus::Failed
            || c.comparison.is_some()
        {
            return Err(bad("screen stopped research is not a positive command"));
        }
        println!(
            "T_SCREEN_COMPLETED=true STOPPED_AT={} COMMAND_STATUS=Failed QUALITY_STOP_PRESERVED=true RESUME=false candidate=false H3_JOINT=false",
            t.updates
        );
    } else {
        if t.updates != 512 || !t.complete {
            return Err(bad("screen research incomplete"));
        }
        effective_outcome(root, &s, &reference)?;
        let close = close_native_inner(root, terminal, control, false)?;
        let native = t.native.as_ref().unwrap();
        print_bridge_teacher(
            &read_verified_bridge_teacher(root, "final-probe", &s, native)?,
            native.step,
        )?;
        println!(
            "T_SCREEN_COMPLETED=true COMPLETE_512=true H3_JOINT={} FULL_PANELS={:?}",
            close.candidate, close.panels
        );
    }
    let native = t.native.as_ref().unwrap();
    println!(
        "BOUNDED_RESEARCH NEW_SMALL_UPDATES={} INPUT_TOKENS={} TARGET_TOKENS={} NEW_GENERATIONS={} NEW_FORWARDS={} LAST_DURABLE_NATIVE={} file={} model={} step={} COMPARISON=POSTHOC_DIFFERENT_SOURCE_AND_STOPPING HISTORICAL_C_FINALIZATION=FAILED_UNCHANGED GOAL1_ACCEPTED=false",
        t.updates,
        native.counters[0] - s.parent.counters[0],
        native.counters[1] - s.parent.counters[1],
        chain.iter().map(|(_, t)| t.generations).sum::<u64>(),
        chain.iter().map(|(_, t)| t.teachers).sum::<u64>(),
        native.file.locator,
        hex(&native.file.digest),
        hex(&native.model),
        native.step
    );
    Ok(())
}
fn screen_prepare(
    prior_arm: &Path,
    observation: &Path,
    audit_path: &Path,
    output: &Path,
    control: &mut RunControl,
) -> Result<()> {
    let audit_ref = absolute_reference(&audit_path.join("audit-final.r3er"))?;
    let audit = verified_artifact_audit(&audit_ref)?;
    println!(
        "AUDIT_REUSED source={} binary={} calls={:?} generated_tokens={} elapsed={} NEW_CALLS=0 HISTORICAL_COMMAND_STILL_FAILED=true",
        hex(&audit.source),
        hex(&audit.binary),
        audit.calls,
        audit
            .fresh
            .iter()
            .map(|(_, r)| r.tokens.len())
            .sum::<usize>(),
        audit.elapsed.finite()?
    );
    let old = read_inputs(prior_arm)?;
    if old.tiny_spec
        || !old.bridge()
        || old.arm_name() != "T-TEMPORAL"
        || !arm_commands(prior_arm, &old)?.is_empty()
    {
        return Err(bad("screen requires unchanged unstarted T"));
    }
    bridge_origins(&old, None)?;
    let Record::Inputs(audited_input) = read_absolute(&audit.input)? else {
        return Err(bad("audited parent input"));
    };
    if audited_input.parent.file.digest != old.parent.file.digest
        || audited_input.parent.model != old.parent.model
        || audited_input.parent.adam != old.parent.adam
        || audited_input.parent.tokenizer != old.parent.tokenizer
        || audited_input.parent.architecture != old.parent.architecture
        || audited_input.parent.step != old.parent.step
        || audited_input.parent.counters != old.parent.counters
    {
        return Err(bad(
            "screen and C diagnostic must share the same original parent",
        ));
    }
    let obs = read_inputs(observation)?;
    bridge_origins(&obs, Some(observation))?;
    read_preflight_outcome(observation)?
        .ok_or_else(|| bad("screen parent observation"))?
        .require_current_success()?;
    let l = resolve_native(prior_arm, &old, &old.parent, true)?;
    if obs.parent.model != old.parent.model || obs.parent.adam != old.parent.adam {
        return Err(bad("screen observation parent"));
    }
    let train = old
        .train
        .iter()
        .map(|i| old.cases[*i as usize].clone())
        .collect::<Vec<_>>();
    exact_training_inputs(&train, &l)?;
    let temporal = data::native::read(&origin_path(&old, "bridge-temporal-corpus")?)?;
    if data::native::ordered_bytes(&train) != data::native::ordered_bytes(&temporal.train) {
        return Err(bad("screen original T native bytes"));
    }
    // Read original parent panels at the same native endpoint; never regenerate320.
    let prior_input = origin_path(&old, "bridge-parent-inputs")?;
    let parent_root = prior_input.parent().unwrap();
    let parent_s = read_inputs(parent_root)?;
    let closed = arm_commands(parent_root, &parent_s)?;
    let terminal = &closed
        .last()
        .ok_or_else(|| bad("parent terminal missing"))?
        .1
        .terminal;
    close_native_inner(parent_root, &terminal.locator, control, false)?;
    let chain = lineage(parent_root, &parent_s, terminal)?;
    let h = verified_history(parent_root, &parent_s, &chain)?;
    let mut rows = BTreeMap::new();
    let mut inherited_raw = Vec::new();
    for kind in [PanelKind::Dev, PanelKind::Cross, PanelKind::Ordinary] {
        let er = h
            .evaluations
            .get(&(old.parent.step, kind))
            .ok_or_else(|| bad("same parent raw missing"))?;
        let (raw, loaded) = payload(parent_root, &parent_s, er)?;
        rescore(&parent_s, &raw, &loaded, true)?;
        if raw.model != old.parent.model {
            return Err(bad("parent screen model mismatch"));
        }
        for r in raw.rows {
            rows.insert(r.case, r);
        }
        inherited_raw.push(absolute_reference(&parent_root.join(&er.payload.locator))?);
    }
    let Record::Evaluation(dev) = read_record(observation, &reference(observation, "dev.r3er")?)?
    else {
        return Err(bad("parent newdev raw"));
    };
    rescore(&obs, &dev, &l, true)?;
    for r in dev.rows {
        rows.insert(r.case, r);
    }
    inherited_raw.push(absolute_reference(&observation.join("dev.r3er"))?);
    let registered = output
        .parent()
        .ok_or_else(|| bad("screen root"))?
        .canonicalize()?
        .join("T-SCREEN");
    if output.file_name().and_then(|n| n.to_str()) != Some("T-SCREEN") || output.exists() {
        return Err(bad("screen exactly one new T-SCREEN root"));
    }
    let mut s = old.clone();
    s.contract = BOUNDED_BRIDGE_CONTRACT.into();
    s.source = evaluator_source();
    s.eval_steps = vec![32, 64, 128, 256, 512];
    let selected = screen_selection(&s)?;
    let bytes = data::native::ordered_bytes(
        &selected
            .iter()
            .map(|i| s.cases[*i as usize].clone())
            .collect::<Vec<_>>(),
    );
    s.panels.push(PanelSpec {
        kind: PanelKind::Screen,
        dataset: hash(&bytes),
        cases: selected,
    });
    s.origins.retain(|o| {
        !matches!(
            o.role.as_str(),
            "bridge-replacement-registration" | "execution-binary" | "cooldown-registered-root"
        )
    });
    for (role, file) in [
        ("bounded-audit", audit_ref.clone()),
        (
            "bounded-prior-inputs",
            absolute_reference(&prior_arm.join("inputs.r3er"))?,
        ),
        (
            "execution-binary",
            absolute_reference(&std::env::current_exe()?)?,
        ),
    ] {
        s.origins.push(Origin {
            role: role.into(),
            original: file,
        });
    }
    for (i, r) in inherited_raw.into_iter().enumerate() {
        s.origins.push(Origin {
            role: format!("bounded-parent-raw-{i}"),
            original: r,
        });
    }
    let mut key = BOUNDED_BRIDGE_CONTRACT.as_bytes().to_vec();
    key.extend(audit_ref.digest);
    key.extend(old.binding());
    key.extend(s.source);
    string(&mut key, &registered.display().to_string());
    s.run = hash(&key);
    s.policy = s.run;
    s.parent.run = s.run;
    s.authorization.as_mut().unwrap().pair = s.run;
    s.origins.push(Origin {
        role: "cooldown-registered-root".into(),
        original: FileRef {
            locator: registered.parent().unwrap().display().to_string(),
            digest: s.run,
        },
    });
    s.validate()?;
    let mut future = l.manifest.training.clone().unwrap();
    fork_budget(
        &mut future,
        s.parent.step,
        s.parent.counters[0],
        512,
        2_000_000,
    )?;
    future.step += 512;
    future.consumed_tokens += s.tape.iter().map(|d| d.input).sum::<u64>();
    future.target_tokens += s.tape.iter().map(|d| d.target).sum::<u64>();
    future.sampler_state = s.tape.last().unwrap().sampler;
    future.resume_binding = Some(objective_binding(&s, &future, &l.tokenizer)?);
    let mut m = l.manifest.clone();
    m.training = Some(future);
    checkpoint::validate_metadata(&m, &l.tokenizer)?;
    Record::Segment(SegmentReceipt::capacity_value(s.tape.clone(), true, false)).encode()?;
    let raw = panel(&s, PanelKind::Screen)?
        .cases
        .iter()
        .map(|i| {
            let mut r = rows
                .get(&case_hash(&s.cases[*i as usize]))
                .cloned()
                .ok_or_else(|| bad("parent screen exact case absent"))?;
            r.ordinal = *i;
            Ok(r)
        })
        .collect::<Result<Vec<_>>>()?;
    let mut parent = bridge_payload(&s, &s.parent, PanelKind::Screen, raw);
    parent.expected = 224;
    let scores = screen_scores(&s, &parent, &l)?;
    let parent_bytes = Record::Evaluation(parent.clone()).encode()?;
    let input = Record::Inputs(Box::new(s.clone()));
    let proof = absolute_reference(&observation.join("preflight-final.r3er"))?;
    // Register before creating the child. Missing child/final files block another attempt.
    publish(
        audit_path,
        "t-screen-registration.r3er",
        &Record::ScreenRegistration {
            root: registered.display().to_string(),
            input: hash(&input.encode()?),
            audit: audit_ref,
            observation: proof,
            parent: FileRef {
                locator: registered.join("parent-screen.r3er").display().to_string(),
                digest: hash(&parent_bytes),
            },
            preparation: Scalar::F64(control.start.elapsed().as_secs_f64()),
        },
    )?;
    std::fs::create_dir(&registered)?;
    std::fs::copy(prior_arm.join("parent.r3m"), registered.join("parent.r3m"))?;
    publish(&registered, "inputs.r3er", &input)?;
    publish(
        &registered,
        "parent-screen.r3er",
        &Record::Evaluation(parent),
    )?;
    screen_budget(&registered, &s)?;
    println!(
        "T_SCREEN_REGISTERED parent_step={} parent_model={} Adam={} baseline={scores:?} HORIZON=512 INITIAL_LIMIT=128 EVAL=32,64,128,256,512 LR_BITS={} NEW_GENERATIONS=0 NEW_UPDATES=0 audit_calls={:?} PARENT_ROWS=DERIVED_FROM_EXISTING_RAW",
        s.parent.step,
        hex(&s.parent.model),
        hex(&s.parent.adam.unwrap()),
        1e-4f64.to_bits(),
        audit.calls
    );
    Ok(())
}

fn absolute_reference(path: &Path) -> Result<FileRef> {
    Ok(FileRef {
        locator: path.canonicalize()?.display().to_string(),
        digest: unhex(&file_hash(path)?)?,
    })
}
fn exact_training_inputs(episodes: &[Episode], l: &Loaded) -> Result<()> {
    let framed = samples(
        episodes,
        &l.tokenizer,
        l.manifest
            .training
            .as_ref()
            .ok_or_else(|| bad("input audit state"))?
            .config
            .seq_len,
    )?;
    if framed.len() != episodes.len() {
        return Err(bad("input audit dropped episodes"));
    }
    let mut seen: BTreeMap<Hash, Vec<usize>> = BTreeMap::new();
    for (i, sample) in framed.iter().enumerate() {
        let prompt = &sample.tokens[..sample.response_start];
        let prepared = l.tokenizer.prepare(
            &episodes[i].request,
            l.model.config.context as u32,
            &l.model.config.id()?,
        )?;
        if prompt != prepared.token_ids {
            println!(
                "INPUT_FRAMING_MISMATCH id={} training_prompt_tokens={} product_prompt_tokens={} training_hash={} product_hash={} seq_len={} context={}",
                episodes[i].id,
                prompt.len(),
                prepared.token_ids.len(),
                hex(&token_hash(prompt)),
                hex(&token_hash(&prepared.token_ids)),
                l.manifest.training.as_ref().unwrap().config.seq_len,
                l.model.config.context
            );
            return Err(bad("training/generation prompt token mismatch"));
        }
        let bucket = seen.entry(token_hash(prompt)).or_default();
        for j in bucket.iter().copied() {
            let old = &framed[j];
            if prompt == &old.tokens[..old.response_start]
                && sample.tokens[sample.response_start..] != old.tokens[old.response_start..]
            {
                println!(
                    "DATA_CONFLICT first={} second={} prompt={}",
                    episodes[j].id,
                    episodes[i].id,
                    hex(&token_hash(prompt))
                );
                return Err(bad("DATA_CONFLICT exact prompt with differing target"));
            }
        }
        bucket.push(i);
    }
    println!(
        "EXACT_INPUT_AUDIT cases={} conflicts=0 TOKEN_HASH_COLLISIONS_CHECKED_BY_IDS=true",
        episodes.len()
    );
    Ok(())
}
fn bridge_diagnostic_selection(s: &RunSnapshot) -> Result<Vec<u32>> {
    let mut out = Vec::new();
    for k in [
        PanelKind::LegacyDev,
        PanelKind::Cross,
        PanelKind::Ordinary,
        PanelKind::Dev,
    ] {
        let mut strata: BTreeMap<(usize, String, usize), Vec<u32>> = BTreeMap::new();
        for i in &panel(s, k)?.cases {
            let e = &s.cases[*i as usize];
            // Category/family/length only; no generated output or correctness is consulted.
            let family = e
                .family
                .split("/replica-")
                .next()
                .unwrap_or(&e.family)
                .to_owned();
            strata
                .entry((e.category, family, e.request.input.len() / 32))
                .or_default()
                .push(*i);
        }
        for v in strata.values_mut() {
            v.sort_by_key(|i| hash(s.cases[*i as usize].id.as_bytes()));
        }
        let mut selected = Vec::new();
        for row in 0..panel(s, k)?.cases.len() {
            for v in strata.values() {
                if let Some(i) = v.get(row) {
                    selected.push(*i);
                    if selected.len() == 8 {
                        break;
                    }
                }
            }
            if selected.len() == 8 {
                break;
            }
        }
        out.extend(selected);
    }
    Ok(out)
}
fn bridge_regression_pattern(e: &Episode, row: &EvalRow, l: &Loaded) -> Result<(String, String)> {
    let (actual, abnormal) = row_output(row, l)?;
    let Some(a) = actual else {
        return Ok((
            row.error_class.clone().unwrap_or("UNKNOWN_OUTPUT".into()),
            "unknown".into(),
        ));
    };
    let first = a
        .bytes()
        .zip(e.answer.bytes())
        .position(|(a, b)| a != b)
        .unwrap_or(a.len().min(e.answer.len()));
    let field = if a == e.answer {
        "none"
    } else {
        field_at(&e.answer, first)
    };
    let class = if abnormal {
        "GENERATION_ERROR"
    } else if a == e.answer && row.finish == Finish::Eos {
        "STRICT_EXACT"
    } else if a.is_empty() {
        "EMPTY"
    } else if row.finish == Finish::Length {
        "LENGTH_STOP"
    } else if fields(a.as_str()).is_none()
        && fields(&e.answer).is_some()
        && a.split_once("입니다. [event:")
            .is_some_and(|(value, tail)| {
                !value.is_empty() && tail.ends_with(']') && citations(&a).is_ok()
            })
    {
        let value = a.split_once("입니다. [event:").unwrap().0;
        let expected = fields(&e.answer).unwrap().2;
        // Auxiliary grammar-based classification only; strict EM is never repaired.
        if value == expected {
            if citations(&a).ok() == citations(&e.answer).ok() {
                "SHORT_QA_FORMAT_VALUE_AND_CITATION_MATCH"
            } else {
                "SHORT_QA_FORMAT_VALUE_MATCH_WRONG_CITATION"
            }
        } else if e
            .request
            .evidence
            .items
            .iter()
            .any(|v| fields(&v.original_excerpt).is_some_and(|f| f.2 == value))
        {
            "SHORT_QA_FORMAT_OTHER_SUPPLIED_VALUE"
        } else {
            "SHORT_QA_FORMAT_WRONG_VALUE_OR_DIGITS"
        }
    } else if fields(&a).is_none() {
        "UNKNOWN_FORMAT"
    } else if citations(&a).ok() != citations(&e.answer).ok() {
        "WRONG_CITATION_OR_RECORD"
    } else if let (Some(x), Some(y)) = (fields(&a), fields(&e.answer)) {
        if x.0 != y.0 {
            "WRONG_ENTITY"
        } else if x.1 != y.1 {
            "WRONG_CONTEXT"
        } else if x.2 != y.2
            && e.request
                .evidence
                .items
                .iter()
                .any(|v| fields(&v.original_excerpt).is_some_and(|f| f.2 == x.2))
        {
            "OTHER_SUPPLIED_VALUE"
        } else if x.2 != y.2 {
            "WRONG_VALUE_OR_DIGITS"
        } else {
            "FORMAT_OR_EOS"
        }
    } else {
        "UNKNOWN_FORMAT"
    };
    Ok((class.into(), field.into()))
}
fn audit_forward(
    a: &mut ArtifactAudit,
    output: &Path,
    control: &mut RunControl,
    forward: impl FnOnce() -> Result<Tensor>,
) -> Result<Tensor> {
    control.check("artifact_diagnostic_before_forward")?;
    if a.calls[2] >= 64 || control.teacher_calls >= control.teacher_limit {
        return Err(bad("diagnostic forward cap"));
    }
    a.calls[2] += 1;
    control.teacher_calls += 1;
    a.elapsed = Scalar::F64(control.start.elapsed().as_secs_f64());
    publish(
        output,
        &format!("forward-entry-{:02}.r3er", a.calls[2]),
        &Record::ArtifactAudit(Box::new(a.clone())),
    )?;
    let result = forward();
    a.calls[3] += 1;
    publish(
        output,
        &format!("forward-return-{:02}.r3er", a.calls[3]),
        &Record::ArtifactAudit(Box::new(a.clone())),
    )?;
    result
}
fn diagnostic_prefix(
    a: &mut ArtifactAudit,
    output: &Path,
    control: &mut RunControl,
    l: &Loaded,
    e: &Episode,
    row: &EvalRow,
    model: u8,
) -> Result<()> {
    let framed = samples(
        std::slice::from_ref(e),
        &l.tokenizer,
        l.model.config.context,
    )?;
    let sample = framed
        .first()
        .ok_or_else(|| bad("diagnostic sample omitted"))?;
    let prompt = l.tokenizer.prepare(
        &e.request,
        l.model.config.context as u32,
        &l.model.config.id()?,
    )?;
    if sample.tokens[..sample.response_start] != prompt.token_ids {
        return Err(bad("same-prefix framing mismatch"));
    }
    let gold = &sample.tokens[sample.response_start..];
    let position = (0..gold.len())
        .find(|i| row.tokens.get(*i) != gold.get(*i))
        .unwrap_or(gold.len() - 1);
    let mut prefix = prompt.token_ids;
    prefix.extend_from_slice(&row.tokens[..position]); // only the identical pre-error free prefix
    let scope = "bounded-first-error";
    let mut cache = l.model.cache(scope);
    let pre = Tensor::new(&prefix[..prefix.len() - 1], &Device::Cpu)?.unsqueeze(0)?;
    let last = Tensor::new(&prefix[prefix.len() - 1..], &Device::Cpu)?.unsqueeze(0)?;
    let all = Tensor::new(prefix.as_slice(), &Device::Cpu)?.unsqueeze(0)?;
    let _ = audit_forward(a, output, control, || {
        l.model.forward_cached(&pre, &mut cache, scope)
    })?;
    let cached = audit_forward(a, output, control, || {
        l.model.forward_cached(&last, &mut cache, scope)
    })?;
    let full = audit_forward(a, output, control, || l.model.forward(&all, None))?.narrow(
        1,
        prefix.len() - 1,
        1,
    )?;
    let parity = compare(&full, &cached)?; // existing abs1e-4 + rel1e-3 per-logit rule
    let fv = full.flatten_all()?.to_vec1::<f32>()?;
    let cv = cached.flatten_all()?.to_vec1::<f32>()?;
    let full_id = full.argmax(2)?.flatten_all()?.to_vec1::<u32>()?[0];
    let cached_id = cached.argmax(2)?.flatten_all()?.to_vec1::<u32>()?[0];
    let top = fv[full_id as usize];
    let second = fv
        .iter()
        .enumerate()
        .filter(|(i, _)| *i != full_id as usize)
        .map(|(_, v)| *v)
        .max_by(f32::total_cmp)
        .unwrap();
    let gap = f64::from(top - second);
    let tie = 2. * (1e-4 + 1e-3 * f64::from(top.abs().max(second.abs())));
    if full_id != cached_id && gap > tie {
        return Err(bad("cached/uncached argmax mismatch outside near tie"));
    }
    let values = [
        parity["max_abs"]
            .as_f64()
            .ok_or_else(|| bad("numeric result"))?,
        f64::from(fv[gold[position] as usize] - top),
        f64::from(cv[gold[position] as usize] - cv[cached_id as usize]),
        gap,
    ];
    a.numeric.push(PrefixCheck {
        model,
        ordinal: row.ordinal,
        position: position as u32,
        gold: gold[position],
        full: full_id,
        cached: cached_id,
        values,
    });
    println!(
        "PREFIX_NUMERIC model={model} ordinal={} first_error_token={position} full={full_id} cached={cached_id} gold={} max_abs={} gold_margin_full={} gold_margin_cached={} top_gap={gap} near_tie_threshold={tie} prompt_equal=true",
        row.ordinal, gold[position], values[0], values[1], values[2]
    );
    control.check("artifact_diagnostic_after_forward")
}
fn bridge_artifact_audit(
    root: &Path,
    checkpoint_name: &str,
    expected: &str,
    output: &Path,
    control: &mut RunControl,
) -> Result<()> {
    let s = read_inputs(root)?;
    if !s.bridge() || s.arm_name() != "C-COPYMATCH" || s.historical {
        return Err(bad("diagnostic requires preserved C run"));
    }
    bridge_origins(&s, None)?;
    let path = owned_path(root, checkpoint_name, true)?;
    if file_hash(&path)? != expected {
        return Err(bad("diagnostic expected physical checkpoint"));
    }
    let l = checkpoint::load(&path, Device::Cpu, true)?;
    let n = native_reference(root, checkpoint_name, &s, &l, 1)?;
    let parent = resolve_native(root, &s, &s.parent, true)?;
    let state = l
        .manifest
        .training
        .as_ref()
        .ok_or_else(|| bad("diagnostic native state"))?;
    let updates = s.tape.len();
    if n.step != s.parent.step + updates as u64
        || n.tokenizer != s.parent.tokenizer
        || n.architecture != s.parent.architecture
        || n.counters
            != [
                s.parent.counters[0] + s.tape.iter().map(|d| d.input).sum::<u64>(),
                s.parent.counters[1] + s.tape.iter().map(|d| d.target).sum::<u64>(),
                s.tape.last().ok_or_else(|| bad("empty tape"))?.sampler,
            ]
        || state.resume_binding.as_ref() != Some(&objective_binding(&s, state, &l.tokenizer)?)
    {
        return Err(bad("diagnostic native/objective/parent/tape/clock"));
    }
    let mut expected_state = parent.manifest.training.clone().unwrap();
    fork_budget(
        &mut expected_state,
        s.parent.step,
        s.parent.counters[0],
        updates,
        2_000_000,
    )?;
    if state.config != expected_state.config
        || state.corpus_hash != expected_state.corpus_hash
        || state.validation_hash != expected_state.validation_hash
    {
        return Err(bad("diagnostic native training config changed"));
    }
    checkpoint::validate_metadata(&l.manifest, &l.tokenizer)?;
    let original_terminal = root.join("segment-01/terminal.r3er");
    if original_terminal.exists() || root.join("segment-01/command.r3er").exists() {
        return Err(bad("diagnostic expects unchanged failed C finalization"));
    }
    let train = s
        .train
        .iter()
        .map(|i| s.cases[*i as usize].clone())
        .collect::<Vec<_>>();
    let temporal = if s.tiny_spec {
        train.clone()
    } else {
        data::native::read(&origin_path(&s, "bridge-temporal-corpus")?)?.train
    };
    let framed = samples(&train, &l.tokenizer, state.config.seq_len)?;
    for d in &s.tape {
        if d.input
            != d.indices
                .iter()
                .map(|i| framed[*i as usize].tokens.len() as u64 - 1)
                .sum::<u64>()
            || d.target
                != d.indices
                    .iter()
                    .map(|i| {
                        (framed[*i as usize].tokens.len() - framed[*i as usize].response_start)
                            as u64
                    })
                    .sum::<u64>()
        {
            return Err(bad("diagnostic actual tape token counts"));
        }
    }
    let mut originals = vec![
        absolute_reference(&path)?,
        absolute_reference(&root.join("parent.r3m"))?,
    ];
    for o in &s.origins {
        if Path::new(&o.original.locator).is_file() {
            let mut path = PathBuf::from(&o.original.locator);
            if o.role == "execution-binary" && unhex(&file_hash(&path)?)? != o.original.digest {
                path = root
                    .parent()
                    .and_then(Path::parent)
                    .ok_or_else(|| bad("preserved execution root"))?
                    .join("executed-replica-train");
            }
            let r = absolute_reference(&path)?;
            if r.digest != o.original.digest {
                return Err(bad("diagnostic origin digest changed"));
            }
            originals.push(r);
        }
    }
    let mut panels = Vec::new();
    let mut raw = BTreeMap::new();
    for kind in [
        PanelKind::LegacyDev,
        PanelKind::Cross,
        PanelKind::Ordinary,
        PanelKind::Dev,
        PanelKind::Conditional,
    ] {
        let name = format!("segment-01/{}-{updates:04}.r3er", kind.name());
        let p = root.join(&name);
        let r = reference(root, &name)?;
        let Record::Evaluation(e) = read_record(root, &r)? else {
            return Err(bad("diagnostic panel kind"));
        };
        if e.source != s.source || e.step != n.step || e.model != n.model {
            return Err(bad("diagnostic raw source/endpoint"));
        }
        panels.push((kind, rescore(&s, &e, &l, true)?));
        print_anchor_panel(&s, &e, &l)?;
        let mut classes: BTreeMap<String, u64> = BTreeMap::new();
        let mut first: BTreeMap<String, u64> = BTreeMap::new();
        for row in &e.rows {
            let (class, field) =
                bridge_regression_pattern(&s.cases[row.ordinal as usize], row, &l)?;
            *classes.entry(class).or_default() += 1;
            *first.entry(field).or_default() += 1;
            raw.insert(row.ordinal, row.clone());
        }
        println!(
            "C512_RAW_RECOUNT panel={} classes={classes:?} first_byte_error_fields={first:?} STRICT_METRIC_UNCHANGED=true",
            kind.name()
        );
        originals.push(absolute_reference(&p)?);
    }
    let teacher = read_verified_bridge_teacher(root, "final-probe", &s, &n)?;
    for r in &teacher.files {
        originals.push(absolute_reference(&root.join(&r.locator))?);
    }
    print_bridge_teacher(&teacher, n.step)?;
    println!(
        "C512_ARTIFACT=VERIFIED file={} model={} Adam={} step={} HISTORICAL_COMMAND=FAILED_PUBLICATION_UNCHANGED HISTORICAL_TOTAL_USAGE=UNKNOWN C512_PROMOTION=false C512_TRAIN_RESUME=false",
        hex(&n.file.digest),
        hex(&n.model),
        hex(&n.adam.unwrap()),
        n.step
    );
    let selection = bridge_diagnostic_selection(&s)?;
    std::fs::create_dir(output)?;
    std::fs::copy(std::env::current_exe()?, output.join("executed-binary"))?;
    originals.push(absolute_reference(&output.join("executed-binary"))?);
    let mut a = ArtifactAudit {
        source: evaluator_source(),
        binary: unhex(&file_hash(&std::env::current_exe()?)?)?,
        input: absolute_reference(&root.join("inputs.r3er"))?,
        native: n,
        originals,
        selection,
        fresh: vec![],
        numeric: vec![],
        panels,
        calls: [0; 4],
        elapsed: Scalar::F64(control.start.elapsed().as_secs_f64()),
        complete: false,
        stop: vec![],
        error: None,
    };
    publish(
        output,
        "audit-start.r3er",
        &Record::ArtifactAudit(Box::new(a.clone())),
    )?;
    println!(
        "DIAGNOSTIC_SELECTION ordinals={:?} policy=category/family/length/hash NORMAL_GREEDY_LIMIT=64 PREFIX_CASES_PER_MODEL=10 FORWARD_LIMIT=64 ABS=0.0001 REL=0.001 NEAR_TIE=2*(ABS+REL*top_magnitude)",
        a.selection
    );
    control.generation_limit = 64;
    control.teacher_limit = 64;
    let outcome = (|| -> Result<()> {
        // A failed input audit still preserves the independent native/raw recount.
        exact_training_inputs(&train, &l)?;
        exact_training_inputs(&temporal, &l)?;
        for (model, loaded) in [(0u8, &parent), (1u8, &l)] {
            for (i, ordinal) in a.selection.clone().into_iter().enumerate() {
                let case = &s.cases[ordinal as usize];
                let entry_name = format!("generation-entry-{model}-{i:02}.r3er");
                let row =
                    evaluate_row_with_entry(loaded, case, ordinal, control, false, true, || {
                        a.calls[0] += 1;
                        publish(
                            output,
                            &entry_name,
                            &Record::ArtifactAudit(Box::new(a.clone())),
                        )?;
                        Ok(())
                    })?;
                a.calls[1] += 1;
                a.fresh.push((model, row.clone()));
                publish(
                    output,
                    &format!("generation-row-{model}-{i:02}.r3er"),
                    &Record::ArtifactAudit(Box::new(a.clone())),
                )?;
                control.check("artifact_diagnostic_generation_returned")?;
                if model == 1 {
                    let prior = raw
                        .get(&ordinal)
                        .ok_or_else(|| bad("diagnostic original row absent"))?;
                    if row.tokens != prior.tokens
                        || row.finish != prior.finish
                        || row.error != prior.error
                        || row.eos != prior.eos
                    {
                        return Err(bad("C512 fresh/raw token/EOS/error mismatch"));
                    }
                }
                let pattern = bridge_regression_pattern(case, &row, loaded)?;
                println!(
                    "FRESH_DIAGNOSTIC model={model} ordinal={ordinal} exact={} pattern={pattern:?}",
                    row_output(&row, loaded)?.0.as_deref() == Some(case.answer.as_str())
                );
                if i < 10 {
                    diagnostic_prefix(&mut a, output, control, loaded, case, &row, model)?;
                }
            }
        }
        Ok(())
    })();
    if let Err(e) = &outcome {
        control.classify_error(e);
        a.error = Some(e.to_string());
    }
    a.stop = control.observed.clone();
    a.elapsed = Scalar::F64(control.start.elapsed().as_secs_f64());
    a.complete = outcome.is_ok() && a.stop.is_empty();
    publish_audit_final(output, &a)?;
    println!(
        "C512_DIAGNOSTIC complete={} calls={:?} seconds={} STOP={:?} ERROR={:?} HISTORICAL_COMMAND_STILL_FAILED=true RESUME_AUTHORIZATION=false NEW_SMALL_UPDATES=0",
        a.complete,
        a.calls,
        a.elapsed.finite()?,
        a.stop,
        a.error
    );
    outcome
}
// Teacher-only observations use existing typed rows; started=false/tokens=[] means
// no free generation was performed. retained=[foil index,gold ID,foil ID].
fn collect_bridge_teacher(
    root: &Path,
    prefix: &str,
    s: &RunSnapshot,
    n: &CheckpointRef,
    l: &Loaded,
    control: &mut RunControl,
) -> Result<()> {
    let indices = bridge_probe_indices(s)?;
    let mut raw = bridge_payload(s, n, PanelKind::Dev, Vec::new());
    raw.expected = indices.len() as u32;
    let start = format!("{prefix}-start.r3er");
    if root.join(&start).exists() {
        return print_bridge_teacher(&read_verified_bridge_teacher(root, prefix, s, n)?, n.step);
    }
    publish(root, &start, &Record::Evaluation(raw.clone()))?;
    for (index, &ordinal) in indices.iter().enumerate() {
        let result = format!("{prefix}-row-{index:02}.r3er");
        let entry = format!("{prefix}-entry-{index:02}.r3er");
        let e = &s.cases[ordinal as usize];
        let prompt = l.tokenizer.prepare(
            &e.request,
            l.model.config.context as u32,
            &l.model.config.id()?,
        )?;
        if root.join(&entry).exists() {
            return Err(bad("bridge teacher interrupted UNKNOWN; no retry"));
        }
        if control.teacher_calls >= control.teacher_limit
            || root.join(format!("{prefix}-final.r3er")).exists()
        {
            return Err(bad(
                "bridge teacher budget or incomplete previously final raw",
            ));
        }
        control.check("bridge_before_teacher")?;
        let foil = bridge_foil(s, e)?;
        let mut row = EvalRow {
            ordinal,
            case: case_hash(e),
            prompt: token_hash(&prompt.token_ids),
            prompt_len: prompt.token_ids.len() as u32,
            native_prompt: unhex(&prompt.token_digest)?,
            provided: prompt.provided.clone(),
            excluded: prompt.excluded.clone(),
            tokens: Vec::new(),
            eos: None,
            started: false,
            completed: false,
            finish: Finish::NotStarted,
            error: None,
            error_class: None,
            interruption: None,
            effective_timeout: None,
            timing: None,
            retained: Vec::new(),
            teacher: TeacherRecord::NotRequested,
            diagnostic: None,
        };
        let mut one = raw.clone();
        one.rows = vec![row.clone()];
        publish(root, &entry, &Record::Evaluation(one.clone()))?;
        verification_fault(s.tiny_spec, "teacher", control)?;
        #[cfg(feature = "test-support")]
        if s.tiny_spec && std::env::var("R3ER_TEST_STOP").as_deref() == Ok("bridge-teacher-cancel")
        {
            control.cancel.store(true, Ordering::Relaxed);
        }
        let v = teacher_with_foil(l, e, &prompt.token_ids, &[], control, Some(&foil))?;
        row.teacher = teacher_record(&v)?;
        row.diagnostic = Some(scalar_value(&v["conditional_foil"], "margin")?);
        row.retained = ["index", "gold", "foil"]
            .iter()
            .map(|key| {
                u32::try_from(progress_u64(&v["conditional_foil"], key)?)
                    .map_err(|_| bad("foil token"))
            })
            .collect::<Result<_>>()?;
        row.completed = true;
        one.rows = vec![row.clone()];
        publish(root, &result, &Record::Evaluation(one))?;
        raw.rows.push(row);
    }
    let final_path = format!("{prefix}-final.r3er");
    publish(root, &final_path, &Record::Evaluation(raw))?;
    print_bridge_teacher(&read_verified_bridge_teacher(root, prefix, s, n)?, n.step)
}

fn bridge_foil(s: &RunSnapshot, e: &Episode) -> Result<String> {
    #[cfg(feature = "test-support")]
    if s.tiny_spec {
        return Ok(format!("{}a", e.answer));
    }
    let _ = s;
    let selected = data::bridge_support(e)?;
    let other = &e.request.evidence.items[1 - selected];
    Ok(format!(
        "{} [event:{}]",
        other.original_excerpt, other.event_id
    ))
}

struct VerifiedTeacherEvidence {
    files: Vec<FileRef>,
    rows: Vec<EvalRow>,
}

// Pure reader: no RunControl, forward, collector, publisher or writable lock.
fn read_verified_bridge_teacher(
    root: &Path,
    prefix: &str,
    s: &RunSnapshot,
    n: &CheckpointRef,
) -> Result<VerifiedTeacherEvidence> {
    let indices = bridge_probe_indices(s)?;
    if indices.len() != if s.tiny_spec { 2 } else { 64 } {
        return Err(bad("teacher fixed metadata denominator"));
    }
    verify_bridge_teacher_rows(root, prefix, s, n, &indices)
}

fn verify_bridge_teacher_rows(
    root: &Path,
    prefix: &str,
    s: &RunSnapshot,
    n: &CheckpointRef,
    indices: &[u32],
) -> Result<VerifiedTeacherEvidence> {
    let native = owned_path(root, &n.file.locator, true)?;
    if unhex(&file_hash(&native)?)? != n.file.digest {
        return Err(bad("teacher native file changed"));
    }
    let loaded = checkpoint::load(&native, Device::Cpu, false)?;
    let model = unhex(&loaded.model.weight_hash()?)?;
    let manifest = loaded.manifest;
    let tok = loaded.tokenizer;
    if model != n.model
        || unhex(&tok.semantic_id())? != n.tokenizer
        || unhex(&manifest.architecture.semantic_id()?)? != n.architecture
        || manifest.trained_steps as u64 != n.step
    {
        return Err(bad("teacher actual native identity"));
    }
    if indices.is_empty()
        || indices.len() > 64
        || !indices.len().is_multiple_of(2)
        || indices.iter().collect::<BTreeSet<_>>().len() != indices.len()
    {
        return Err(bad("teacher fixed metadata sample"));
    }
    let mut expected = bridge_payload(s, n, PanelKind::Dev, vec![]);
    expected.expected = indices.len() as u32;
    let mut files = Vec::new();
    let mut read = |name: String| -> Result<EvalPayload> {
        let reference = reference(root, &name)?;
        let Record::Evaluation(v) = read_record(root, &reference)? else {
            return Err(bad("teacher record kind"));
        };
        let mut header = v.clone();
        header.rows.clear();
        if Record::Evaluation(header).encode()? != Record::Evaluation(expected.clone()).encode()? {
            return Err(bad("teacher snapshot/source/model/step/sample identity"));
        }
        files.push(reference);
        Ok(v)
    };
    if !read(format!("{prefix}-start.r3er"))?.rows.is_empty() {
        return Err(bad("teacher start rows"));
    }
    let mut rows = Vec::new();
    for (index, &ordinal) in indices.iter().enumerate() {
        let entry = read(format!("{prefix}-entry-{index:02}.r3er"))?;
        let returned = read(format!("{prefix}-row-{index:02}.r3er"))?;
        if entry.rows.len() != 1 || returned.rows.len() != 1 {
            return Err(bad("teacher row denominator"));
        }
        let e = &s.cases[ordinal as usize];
        let prompt = tok.prepare(
            &e.request,
            manifest.architecture.context as u32,
            &manifest.architecture.id()?,
        )?;
        let r = &returned.rows[0];
        let mut before = r.clone();
        before.teacher = TeacherRecord::NotRequested;
        before.completed = false;
        before.retained.clear();
        before.diagnostic = None;
        if !same_conditional_rows(&[before], &entry.rows)
            || r.ordinal != ordinal
            || r.case != case_hash(e)
            || r.prompt != token_hash(&prompt.token_ids)
            || r.prompt_len as usize != prompt.token_ids.len()
            || r.native_prompt != unhex(&prompt.token_digest)?
            || r.provided != prompt.provided
            || r.excluded != prompt.excluded
            || r.started
            || !r.completed
            || r.interruption.is_some()
            || !r.tokens.is_empty()
            || r.eos.is_some()
            || r.finish != Finish::NotStarted
            || r.error.is_some()
            || r.error_class.is_some()
            || r.timing.is_some()
            || r.effective_timeout.is_some()
        {
            return Err(bad("teacher entry/returned case/prompt/state"));
        }
        let TeacherRecord::Measured(t) = &r.teacher else {
            return Err(bad("teacher not measured"));
        };
        let mut gold = tok.encode(e.answer.as_bytes())?;
        gold.push(EOS);
        let mut foil = tok.encode(bridge_foil(s, e)?.as_bytes())?;
        foil.push(EOS);
        let difference = gold
            .iter()
            .zip(&foil)
            .position(|(a, b)| a != b)
            .ok_or_else(|| bad("teacher foil identical"))?;
        if r.retained != [difference as u32, gold[difference], foil[difference]]
            || t.target != gold.len() as u64
            || t.target == 0
            || t.correct > t.target
            || t.first_gold != gold[0]
            || t.first_argmax as usize >= tok.vocab_size()
            || t.first_correct != (t.first_argmax == t.first_gold)
            || !t.prompt_matches
            || !t.answer_roundtrip
            || t.first_weight.finite()?.to_bits()
                != manifest
                    .training
                    .as_ref()
                    .ok_or_else(|| bad("teacher training metadata"))?
                    .config
                    .first_target_weight
                    .to_bits()
            || t.field_accuracy.iter().any(|f| f.correct > f.total)
            || t.field_accuracy.iter().map(|f| f.total).sum::<u64>() != t.target
            || t.field_accuracy.iter().map(|f| f.correct).sum::<u64>() != t.correct
        {
            return Err(bad("teacher measured token/count/foil contract"));
        }
        for scalar in [
            &t.mean,
            &t.first,
            &t.remaining,
            &t.objective,
            &t.first_weight,
        ] {
            if scalar.finite()? < 0. {
                return Err(bad("teacher negative loss/weight"));
            }
        }
        for scalar in [
            &t.eos_probability,
            &t.gold_probability,
            &t.argmax_probability,
        ] {
            if !(0. ..=1.).contains(&scalar.finite()?) {
                return Err(bad("teacher probability"));
            }
        }
        r.diagnostic
            .as_ref()
            .ok_or_else(|| bad("teacher missing foil margin"))?
            .finite()?;
        if let Some(d) = &t.difference {
            if d.index as usize >= gold.len()
                || d.gold != gold[d.index as usize]
                || d.argmax as usize >= tok.vocab_size()
                || d.actual.is_some_and(|id| id as usize >= tok.vocab_size())
            {
                return Err(bad("teacher difference token range"));
            }
            d.log_probability.finite()?;
            d.margin.finite()?;
        }
        rows.push(r.clone());
    }
    let final_record = read(format!("{prefix}-final.r3er"))?;
    if !same_conditional_rows(&rows, &final_record.rows) {
        return Err(bad("teacher final ordered rows"));
    }
    Ok(VerifiedTeacherEvidence { files, rows })
}

fn print_bridge_teacher(evidence: &VerifiedTeacherEvidence, step: u64) -> Result<()> {
    for (group, rows) in evidence.rows.chunks(evidence.rows.len() / 2).enumerate() {
        let mut nll = 0.;
        let mut margin = 0.;
        let mut correct = 0;
        let mut tokens = 0;
        for r in rows {
            let TeacherRecord::Measured(t) = &r.teacher else {
                return Err(bad("bridge teacher missing"));
            };
            nll += t.mean.finite()?;
            margin += r.diagnostic.as_ref().unwrap().finite()?;
            correct += t.correct;
            tokens += t.target;
        }
        println!(
            "BRIDGE_TEACHER group={} step={} cases={} mean_case_nll={} mean_foil_margin={} teacher_correct_tokens={correct} target_tokens={tokens} FREE_GENERATIONS=0",
            if group == 0 { "train" } else { "dev" },
            step,
            rows.len(),
            nll / rows.len() as f64,
            margin / rows.len() as f64
        );
    }
    Ok(())
}

fn bridge_audit(label: &str, episodes: &[Episode], l: &Loaded, require_all: bool) -> Result<()> {
    let seq = l.manifest.training.as_ref().unwrap().config.seq_len;
    let framed = samples(episodes, &l.tokenizer, seq)?;
    if framed.len() != episodes.len() {
        return Err(bad("bridge audit sample alignment"));
    }
    let mut inputs = BTreeMap::new();
    let mut counts = BTreeMap::<String, u64>::new();
    let mut lengths = Vec::new();
    let mut distances = Vec::new();
    for (e, s) in episodes.iter().zip(&framed) {
        let mut request = e.request.clone();
        request.limits.context_tokens = seq as u32;
        request.limits.max_tokens = (s.tokens.len() - s.response_start) as u32;
        let prepared = l.tokenizer.prepare(&request, seq as u32, "training")?;
        if prepared.token_ids != s.tokens[..s.response_start]
            || citations(&e.answer)?
                .iter()
                .any(|id| !prepared.provided.contains(id))
            || require_all && !prepared.excluded.is_empty()
        {
            return Err(bad("BRIDGE_FRAMING_OR_EXCLUDED_SUPPORT"));
        }
        let digest = token_hash(&prepared.token_ids);
        let target = s.tokens[s.response_start..].to_vec();
        if inputs
            .insert(digest, target.clone())
            .is_some_and(|old| old != target)
        {
            return Err(bad("BRIDGE_SAME_INPUT_CONFLICTING_TARGET"));
        }
        for key in [
            format!("evidence{}", e.request.evidence.items.len().min(3)),
            format!("category{}", e.category),
            format!(
                "task/{}",
                if e.family.starts_with("copy/") {
                    "aux"
                } else if e
                    .request
                    .evidence
                    .items
                    .iter()
                    .any(|r| e.answer == format!("{} [event:{}]", r.original_excerpt, r.event_id))
                {
                    "full-copy"
                } else {
                    "other-QA"
                }
            ),
            format!(
                "question/{}",
                if e.request.input.contains("과거") {
                    "past"
                } else if e.request.input.contains("현재") {
                    "current"
                } else {
                    "other"
                }
            ),
        ] {
            *counts.entry(key).or_default() += 1;
        }
        *counts.entry("excluded_records".into()).or_default() += prepared.excluded.len() as u64;
        let mut statements = BTreeMap::<(&str, &str), Vec<&replica_v3::retrieval::Evidence>>::new();
        for r in &e.request.evidence.items {
            if let Some((entity, rest)) = r.original_excerpt.split_once("의 ")
                && let Some((slot, _)) = rest.split_once(" 이동 지시는 ")
            {
                statements.entry((entity, slot)).or_default().push(r);
            }
        }
        for records in statements.values().filter(|rs| rs.len() > 1) {
            *counts.entry("same_slot_competition".into()).or_default() += 1;
            if let (Some(current), Some(past)) = (
                records.iter().find(|r| r.version_status == "current"),
                records.iter().find(|r| r.version_status == "superseded"),
            ) {
                *counts
                    .entry(format!(
                        "current_id_larger/{}",
                        current.event_id > past.event_id
                    ))
                    .or_default() += 1;
                *counts
                    .entry(format!(
                        "current_time_larger/{}",
                        current.recorded_at > past.recorded_at
                    ))
                    .or_default() += 1;
            }
        }
        lengths.push((s.response_start, target.len()));
        for (i, _) in prepared
            .token_ids
            .iter()
            .enumerate()
            .filter(|(_, t)| **t == neural::EVIDENCE_ROLE)
        {
            distances.push(s.response_start - i);
        }
    }
    lengths.sort();
    distances.sort();
    println!(
        "BRIDGE_COVERAGE={label} cases={} counts={counts:?} prompt_target_min_median_max={:?}/{:?}/{:?} source_distance_min_median_max={:?}/{:?}/{:?} first_target_weight_bits={} input_conflicts=0 JSON_READS=0 SQLITE_OPENS_SOURCE_PATH=0",
        episodes.len(),
        lengths.first(),
        lengths.get(lengths.len() / 2),
        lengths.last(),
        distances.first(),
        distances.get(distances.len() / 2),
        distances.last(),
        l.manifest
            .training
            .as_ref()
            .unwrap()
            .config
            .first_target_weight
            .to_bits()
    );
    Ok(())
}
#[allow(clippy::too_many_arguments)] // Explicit source identities for one bounded study registration.
fn bridge_data(
    parent: &Path,
    checkpoint_path: &Path,
    corpus: &Path,
    base: &Path,
    conditional: &Path,
    output: &Path,
    seed: u64,
    control: &mut RunControl,
) -> Result<()> {
    let s = read_inputs(parent)?;
    let l = checkpoint::load(checkpoint_path, Device::Cpu, true)?;
    let state = l
        .manifest
        .training
        .as_ref()
        .ok_or_else(|| bad("bridge parent state"))?;
    if state.step != 24310
        || state.resume_binding.as_ref() != Some(&objective_binding(&s, state, &l.tokenizer)?)
        || state.resume_binding.as_ref().unwrap().family != 1
    {
        return Err(bad("bridge proven migrated A75 parent"));
    }
    let commands = arm_commands(parent, &s)?;
    let (_, command) = commands
        .last()
        .ok_or_else(|| bad("bridge original parent terminal"))?;
    let chain = lineage(parent, &s, &command.terminal)?;
    let n = chain.last().unwrap().1.native.as_ref().unwrap();
    let original = resolve_native(parent, &s, n, true)?;
    let mut old = original.manifest.training.clone().unwrap();
    old.resume_binding = None;
    let mut new = state.clone();
    new.resume_binding = None;
    if n.model != unhex("dfc3efb664351578340e39d3cfc95b90f270541041d4641d72d6f41a53871b11")?
        || original.model.weight_hash()? != l.model.weight_hash()?
        || optimizer_hash(&original.optimizer)? != optimizer_hash(&l.optimizer)?
        || old != new
    {
        return Err(bad("bridge migration weights/Adam/state lineage"));
    }
    let c = data::native::read(corpus)?;
    let train = s
        .train
        .iter()
        .map(|i| s.cases[*i as usize].clone())
        .collect::<Vec<_>>();
    if data::native::ordered_bytes(&c.train) != data::native::ordered_bytes(&train) {
        return Err(bad("bridge native source parent content"));
    }
    let auth = s
        .authorization
        .as_ref()
        .ok_or_else(|| bad("bridge original pools"))?;
    let anchors = auth.pools[0]
        .iter()
        .map(|i| train[*i as usize].clone())
        .collect::<Vec<_>>();
    for (label, indices) in [
        ("parent-anchor", &auth.pools[0]),
        ("parent-focus", &auth.pools[1]),
    ] {
        bridge_audit(
            label,
            &indices
                .iter()
                .map(|i| train[*i as usize].clone())
                .collect::<Vec<_>>(),
            &l,
            false,
        )?;
    }
    let b = read_inputs(base)?;
    for (label, indices) in [
        ("base-anchor", &b.authorization.as_ref().unwrap().pools[0]),
        ("base-focus", &b.authorization.as_ref().unwrap().pools[1]),
    ] {
        bridge_audit(
            label,
            &indices
                .iter()
                .map(|i| b.cases[b.train[*i as usize] as usize].clone())
                .collect::<Vec<_>>(),
            &l,
            false,
        )?;
    }
    bridge_audit("parent-validation", &c.validation, &l, false)?;
    let previous = conditional_plan(conditional, false)?;
    let mut prior = s.cases.clone();
    prior.extend(b.cases);
    prior.extend(previous.cases);
    let [control_focus, temporal_focus, dev, sanity] =
        data::bridge_panel(&prior, &l.tokenizer, seed)?;
    bridge_audit("copy-match-focus", &control_focus, &l, true)?;
    bridge_audit("temporal-focus", &temporal_focus, &l, true)?;
    bridge_audit("new-development", &dev, &l, true)?;
    bridge_audit("sanity", &sanity, &l, true)?;
    let cf = samples(&control_focus, &l.tokenizer, state.config.seq_len)?;
    let tf = samples(&temporal_focus, &l.tokenizer, state.config.seq_len)?;
    let mismatched = cf
        .iter()
        .zip(&tf)
        .filter(|(a, b)| a.response_start != b.response_start)
        .count();
    if cf
        .iter()
        .zip(&tf)
        .any(|(a, b)| a.tokens[a.response_start..] != b.tokens[b.response_start..])
    {
        return Err(bad("bridge target token equality"));
    }
    println!(
        "BRIDGE_MATCHED cases=512 target_token_equal=true prompt_length_mismatches={mismatched} PLANNED_SMALL_UPDATES=1024 ACTUAL_SMALL_UPDATES=0"
    );
    control.check("bridge_data_before_publish")?;
    std::fs::create_dir(output)?;
    for (name, focus, validation) in [
        ("C-COPYMATCH.r3c", control_focus, dev.clone()),
        ("T-TEMPORAL.r3c", temporal_focus, dev),
        ("sanity.r3c", Vec::new(), sanity),
    ] {
        // R3CORP has two nonempty splits. The sanity file retains the same historical
        // anchors as train; only its validation split is observed, never trained.
        let mut episodes = anchors.clone();
        episodes.extend(focus);
        let mut meta = c.manifest.clone();
        meta.generator = "temporal-record-bridge-v1".into();
        meta.seed = seed;
        meta.split_rule = format!(
            "native-parent={}; unseen entity namespaces exclude prior and reserved seal bucket; train128x4/dev64x4/sanity16x4; {name}",
            hex(&c.semantic)
        );
        let mut built = data::native::from_episodes(meta, episodes, validation)?;
        built.origins.push(data::native::Origin {
            role: "source-parent".into(),
            path: corpus.display().to_string(),
            physical: c.physical,
            bytes: std::fs::metadata(corpus)?.len(),
        });
        data::native::write(&output.join(name), &built, true)?;
        let read = data::native::read(&output.join(name))?;
        for e in read.train.iter().skip(2048).chain(if name == "sanity.r3c" {
            [].iter()
        } else {
            read.validation.iter()
        }) {
            data::bridge_support(e)?;
        }
        println!(
            "BRIDGE_NATIVE={name} train={} dev={} physical={} semantic={}",
            read.train.len(),
            read.validation.len(),
            hex(&read.physical),
            hex(&read.semantic)
        );
    }
    Ok(())
}
fn path_prepare(
    parent_arm: &Path,
    parent: &Path,
    corpus: &Path,
    legacy: Option<&Path>,
    output: &Path,
    control: &mut RunControl,
) -> Result<()> {
    control.check("native_path_prepare")?;
    let original = read_inputs(parent_arm)?;
    let l = checkpoint::load(parent, Device::Cpu, true)?;
    let state = l
        .manifest
        .training
        .as_ref()
        .ok_or_else(|| bad("parity parent Adam absent"))?;
    let expected = objective_binding(&original, state, &l.tokenizer)?;
    if state.resume_binding.as_ref() != Some(&expected) || expected.family != 1 {
        return Err(bad("parity requires provenance-migrated default parent"));
    }
    let closed = arm_commands(parent_arm, &original)?;
    let (_, last) = closed
        .last()
        .ok_or_else(|| bad("parity parent terminal absent"))?;
    let chain = lineage(parent_arm, &original, &last.terminal)?;
    let parent_reference = chain
        .last()
        .unwrap()
        .1
        .native
        .as_ref()
        .ok_or_else(|| bad("parity parent native absent"))?;
    let verified = resolve_native(parent_arm, &original, parent_reference, true)?;
    let mut old_state = verified.manifest.training.clone().unwrap();
    old_state.resume_binding = None;
    let mut new_state = state.clone();
    new_state.resume_binding = None;
    if verified.model.weight_hash()? != l.model.weight_hash()?
        || optimizer_hash(&verified.optimizer)? != optimizer_hash(&l.optimizer)?
        || old_state != new_state
    {
        return Err(bad("migrated parent differs from registered original"));
    }
    let source = data::native::read(corpus)?;
    let frozen = original
        .train
        .iter()
        .map(|i| original.cases[*i as usize].clone())
        .collect::<Vec<_>>();
    let dev = panel(&original, PanelKind::Dev)?
        .cases
        .iter()
        .map(|i| original.cases[*i as usize].clone())
        .collect::<Vec<_>>();
    if data::native::ordered_bytes(&source.train) != data::native::ordered_bytes(&frozen) {
        return Err(bad(
            "native source differs from registered training content/order",
        ));
    }
    if data::native::ordered_bytes(&source.validation) != data::native::ordered_bytes(&dev) {
        return Err(bad(
            "native development split differs from registered ordered cases",
        ));
    }
    if let Some(legacy) = legacy {
        let (old_manifest, old_train, old_dev) = data::load_legacy(legacy)?;
        if data::native::ordered_bytes(&source.train) != data::native::ordered_bytes(&old_train)
            || data::native::ordered_bytes(&source.validation)
                != data::native::ordered_bytes(&old_dev)
            || source.legacy.as_ref().is_none_or(|m| {
                m.train.sha256 != old_manifest.train.sha256
                    || m.validation.sha256 != old_manifest.validation.sha256
            })
        {
            return Err(bad("native/legacy full logical equality absent"));
        }
    }
    std::fs::create_dir(output)?;
    for (name, purpose) in [
        ("legacy-reference", RunPurpose::SaveContinuous),
        ("native", RunPurpose::SaveSplit),
    ] {
        let dir = output.join(name);
        std::fs::create_dir(&dir)?;
        std::fs::copy(parent, dir.join("parent.r3m"))?;
        std::fs::File::open(dir.join("parent.r3m"))?.sync_all()?;
        std::fs::copy(corpus, dir.join("source.r3c"))?;
        std::fs::File::open(dir.join("source.r3c"))?.sync_all()?;
        let mut s = original.clone();
        s.contract = NATIVE_CORPUS_CONTRACT.into();
        s.historical = false;
        s.authorization = None;
        s.objective = None;
        s.source = evaluator_source();
        s.purpose = purpose;
        s.tape.truncate(2);
        s.eval_steps.clear();
        s.lr_policy = 2;
        s.lr_offset = 0;
        // Same existing ordered tape, actual constant rate and Adam; only source preparation differs.
        for (i, d) in s.tape.iter_mut().enumerate() {
            d.sampler = state
                .sampler_state
                .checked_add(i as u64 + 1)
                .ok_or_else(|| bad("parity cursor"))?;
        }
        s.run = hash(
            &[
                b"PATH_PARITY_ONLY-v1\0".as_slice(),
                &source.semantic,
                &unhex(&file_hash(parent)?)?,
            ]
            .concat(),
        );
        s.policy = hash(
            &[
                b"same-parent-tape-lr1e-4-two-updates\0".as_slice(),
                &original.binding(),
                &source.semantic,
            ]
            .concat(),
        );
        s.parent = native_reference(&dir, "parent.r3m", &s, &l, 0)?;
        s.origins = vec![
            Origin {
                role: "native-source".into(),
                original: reference(&dir, "source.r3c")?,
            },
            Origin {
                role: "execution-binary".into(),
                original: FileRef {
                    locator: std::env::current_exe()?.display().to_string(),
                    digest: unhex(&file_hash(&std::env::current_exe()?)?)?,
                },
            },
        ];
        if name == "legacy-reference"
            && let Some(legacy) = legacy
        {
            s.origins.push(Origin {
                role: "explicit-legacy-reference".into(),
                original: FileRef {
                    locator: legacy.canonicalize()?.display().to_string(),
                    digest: source.semantic,
                },
            });
        }
        s.validate()?;
        publish(&dir, "inputs.r3er", &Record::Inputs(Box::new(s)))?;
    }
    println!(
        "PATH_PREPARE=VERIFIED QUALITY_UPDATES=0 PARITY_BUDGET=4 TAPE=2+1+1 JSON_NATIVE_FULL_EQUAL={}",
        legacy.is_some()
    );
    Ok(())
}
fn path_verify(root: &Path, control: &mut RunControl) -> Result<()> {
    let mut previous = None;
    for (name, purpose, segments) in [
        ("legacy-reference", RunPurpose::SaveContinuous, 1),
        ("native", RunPurpose::SaveSplit, 2),
    ] {
        control.check("path_parity_recount")?;
        let dir = root.join(name);
        let s = read_inputs(&dir)?;
        if !s.path_parity() || s.purpose != purpose {
            return Err(bad("path parity registration"));
        }
        native_source(&dir, &s)?;
        let commands = arm_commands(&dir, &s)?;
        if commands.len() != segments {
            return Err(bad("path parity process count"));
        }
        let last = &commands.last().unwrap().1;
        let chain = lineage(&dir, &s, &last.terminal)?;
        let t = &chain.last().unwrap().1;
        if last.status != CommandStatus::Complete
            || !t.complete
            || !t.stop.is_empty()
            || t.updates != 2
            || chain.iter().map(|(_, t)| t.draws.len()).sum::<usize>() != 2
        {
            return Err(bad("path parity endpoint incomplete"));
        }
        close_native_inner(&dir, &last.terminal.locator, control, false)?;
        let n = t
            .native
            .as_ref()
            .ok_or_else(|| bad("path parity native absent"))?;
        let l = resolve_native(&dir, &s, n, true)?;
        let mut state = l.manifest.training.clone().unwrap();
        let descriptor = state
            .resume_binding
            .as_ref()
            .ok_or_else(|| bad("path objective absent"))?;
        if descriptor != &objective_binding(&s, &state, &l.tokenizer)? || descriptor.family != 1 {
            return Err(bad("path objective mismatch"));
        }
        // Per-run policy/provenance differs by explicit source path and split purpose.
        // All executed model/optimizer/clock/config/token states still compare exactly.
        state.resume_binding = None;
        let rates = chain
            .iter()
            .flat_map(|(_, t)| t.lr_bits.clone().unwrap_or_default())
            .collect::<Vec<_>>();
        let actual = (
            l.model.weight_hash()?,
            optimizer_hash(&l.optimizer)?,
            l.tokenizer.semantic_id(),
            state,
            rates,
        );
        if previous.as_ref().is_some_and(|old| old != &actual) {
            return Err(bad("path weights/Adam/tokenizer/state/LR mismatch"));
        }
        println!(
            "PATH_ENDPOINT arm={name} step={} weights={} adam={} lr_bits={:?} inputs={}",
            n.step,
            actual.0,
            actual.1,
            actual.4,
            hex(&s.binding())
        );
        previous = Some(actual);
    }
    println!("PATH_PARITY=PASS ACTUAL_UPDATES=4 QUALITY_UPDATES=0 PROMOTION=NOT_AUTHORIZED");
    Ok(())
}
fn upgrade_resume(
    root: &Path,
    terminal: &str,
    output: &Path,
    control: &mut RunControl,
) -> Result<()> {
    control.check("upgrade_resume")?;
    let s = read_inputs(root)?;
    if s.historical
        || !matches!(
            s.contract.as_str(),
            CONTRACT
                | RESTART_CONTRACT
                | COOLDOWN_CONTRACT
                | OBJECTIVE_CONTRACT
                | NATIVE_CORPUS_CONTRACT
        )
    {
        return Err(bad(
            "LEGACY_OBJECTIVE_UNKNOWN: unsupported historical provenance",
        ));
    }
    let chain = lineage(root, &s, &reference(root, terminal)?)?;
    let t = &chain
        .last()
        .ok_or_else(|| bad("migration terminal absent"))?
        .1;
    let n = t
        .native
        .as_ref()
        .ok_or_else(|| bad("migration durable native absent"))?;
    let mut l = resolve_native(root, &s, n, true)?;
    // Validate actual source policy/endpoint/panel/command bindings, not a filename whitelist.
    close_native_inner(root, terminal, control, false)?;
    let state = l
        .manifest
        .training
        .as_mut()
        .ok_or_else(|| bad("migration Adam absent"))?;
    let expected = objective_binding(&s, state, &l.tokenizer)?;
    if state
        .resume_binding
        .as_ref()
        .is_some_and(|b| b != &expected)
    {
        return Err(bad("migration existing objective mismatch"));
    }
    state.resume_binding = Some(expected);
    let before_model = l.model.weight_hash()?;
    let before_adam = optimizer_hash(&l.optimizer)?;
    checkpoint::save(output, &l.model, &l.tokenizer, l.manifest, &l.optimizer)?;
    let reread = checkpoint::load(output, Device::Cpu, true)?;
    if reread.model.weight_hash()? != before_model
        || optimizer_hash(&reread.optimizer)? != before_adam
    {
        return Err(bad("migration tensor parity"));
    }
    println!(
        "MIGRATION=VERIFIED OPTIMIZER_CALLS=0 OLD_PHYSICAL={} NEW_PHYSICAL={} MODEL={} ADAM={} STEP={}",
        hex(&n.file.digest),
        file_hash(output)?,
        before_model,
        before_adam,
        n.step
    );
    Ok(())
}
pub(super) fn reject_unbound_objective_resume(path: &Path) -> Result<()> {
    let (manifest, tok) = checkpoint::metadata(path)?;
    let state = manifest
        .training
        .as_ref()
        .ok_or_else(|| bad("resume state absent"))?;
    checkpoint::ResumeBinding::require_default(state, &tok)
}
pub(super) fn evaluate_source(
    checkpoint_path: &Path,
    corpus: &Path,
    output: &Path,
    limit: usize,
    split: &str,
    control: &mut RunControl,
) -> Result<()> {
    if limit == 0 || limit > 512 {
        return Err(bad("native source evaluation limit 1..512"));
    }
    let c = data::native::read(corpus)?;
    let cases = match split {
        "train" => &c.train,
        "validation" => &c.validation,
        _ => return Err(bad("native eval split")),
    };
    let l = checkpoint::load(checkpoint_path, Device::Cpu, false)?;
    let selected = cases.iter().take(limit).cloned().collect::<Vec<_>>();
    let mut e = EvalPayload {
        run: unhex(&file_hash(checkpoint_path)?)?,
        binding: c.physical,
        source: evaluator_source(),
        model: unhex(&l.model.weight_hash()?)?,
        tokenizer: unhex(&l.tokenizer.semantic_id())?,
        architecture: unhex(&l.model.config.semantic_id()?)?,
        step: l.manifest.trained_steps as u64,
        new_updates: 0,
        kind: PanelKind::Dev,
        expected: selected.len() as u32,
        rows: vec![],
    };
    let root = output.parent().unwrap_or(Path::new("."));
    let name = output
        .file_name()
        .and_then(|v| v.to_str())
        .ok_or_else(|| bad("eval output name"))?;
    publish(
        root,
        &format!("{name}.start.r3er"),
        &Record::Evaluation(e.clone()),
    )?;
    control.generation_limit = selected.len();
    let result = (|| -> Result<()> {
        for (i, case) in selected.iter().enumerate() {
            let row = evaluate_row(&l, case, i as u32, control, false)?;
            e.rows.push(row);
            publish(
                root,
                &format!("{name}.row-{i:03}.r3er"),
                &Record::Evaluation(e.clone()),
            )?;
            control.check("native_source_eval_row_saved")?;
        }
        Ok(())
    })();
    if let Err(error) = &result {
        control.classify_error(error);
    }
    publish(root, name, &Record::Evaluation(e.clone()))?;
    println!(
        "NATIVE_SOURCE_EVAL planned={} completed={} generations={} JSON_READS=0 CANONICAL=R3ER STOP={:?}",
        e.expected,
        e.rows.iter().filter(|r| r.completed).count(),
        control.generation_calls,
        control.observed
    );
    result
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
    #[cfg(feature = "test-support")]
    FixtureAuditPublication {
        #[arg(long)]
        from: PathBuf,
        #[arg(long)]
        output: PathBuf,
        #[arg(long)]
        verify: bool,
    },
    #[cfg(feature = "test-support")]
    FixtureAncestorFailed {
        #[arg(long)]
        root: PathBuf,
    },
    #[cfg(feature = "test-support")]
    FixtureScreen {
        #[arg(long)]
        observation: PathBuf,
        #[arg(long)]
        output: PathBuf,
    },
    /// Register the single bounded temporal screen, from verified read-only evidence.
    ScreenPrepare {
        #[arg(long)]
        prior_arm: PathBuf,
        #[arg(long)]
        observation: PathBuf,
        #[arg(long)]
        audit: PathBuf,
        #[arg(long)]
        output: PathBuf,
    },
    /// Read the stopped research without granting command success or resume.
    ScreenReport {
        #[arg(long)]
        root: PathBuf,
        #[arg(long)]
        terminal: String,
    },
    /// One bounded audit of the preserved failed C endpoint. Never repairs its command.
    BridgeArtifactAudit {
        #[arg(long)]
        root: PathBuf,
        #[arg(long)]
        checkpoint: String,
        #[arg(long)]
        expected: String,
        #[arg(long)]
        output: PathBuf,
    },
    #[cfg(feature = "test-support")]
    FixtureBridge {
        #[arg(long)]
        from: PathBuf,
        #[arg(long)]
        output: PathBuf,
        #[arg(long)]
        continuous: bool,
        #[arg(long)]
        observation: bool,
    },
    BridgeObservePrepare {
        #[arg(long)]
        parent_arm: PathBuf,
        #[arg(long)]
        checkpoint: PathBuf,
        #[arg(long)]
        data: PathBuf,
        #[arg(long)]
        conditional: PathBuf,
        #[arg(long)]
        output: PathBuf,
        #[arg(long)]
        replacement_of: PathBuf,
        #[arg(long)]
        previous_binary: PathBuf,
    },
    BridgeObserve {
        #[arg(long)]
        root: PathBuf,
    },
    BridgeObserveReport {
        #[arg(long)]
        root: PathBuf,
    },
    BridgePrepare {
        #[arg(long)]
        observation: PathBuf,
        #[arg(long)]
        output: PathBuf,
    },
    BridgeData {
        #[arg(long)]
        parent_arm: PathBuf,
        #[arg(long)]
        checkpoint: PathBuf,
        #[arg(long)]
        corpus: PathBuf,
        #[arg(long)]
        base_arm: PathBuf,
        #[arg(long)]
        conditional: PathBuf,
        #[arg(long)]
        output: PathBuf,
        #[arg(long, default_value_t = 19419)]
        seed: u64,
    },
    #[cfg(feature = "test-support")]
    FixtureConditional {
        #[arg(long)]
        from: PathBuf,
        #[arg(long)]
        output: PathBuf,
    },
    VersionParity {
        #[arg(long)]
        parent_arm: PathBuf,
        #[arg(long)]
        checkpoint: PathBuf,
        #[arg(long)]
        output: PathBuf,
        #[arg(long)]
        compare: Option<PathBuf>,
    },
    SourceMeasure {
        #[arg(long,value_parser=["source-json","native-raw","native-zstd3","cache-raw","cache-zstd3"])]
        format: Option<String>,
        #[arg(long)]
        root: PathBuf,
        #[arg(long)]
        legacy: PathBuf,
        #[arg(long)]
        raw: PathBuf,
        #[arg(long)]
        cold: PathBuf,
        #[arg(long)]
        cache: PathBuf,
        #[arg(long)]
        output: PathBuf,
        #[arg(long, default_value_t = 3)]
        repetitions: u8,
    },
    ConditionalPrepare {
        #[arg(long)]
        parent_arm: PathBuf,
        #[arg(long)]
        base_arm: PathBuf,
        #[arg(long)]
        output: PathBuf,
        #[arg(long, default_value_t = 19317)]
        seed: u64,
    },
    ConditionalRun {
        #[arg(long)]
        root: PathBuf,
        #[arg(long,value_parser=clap::value_parser!(u8).range(0..=1))]
        model: u8,
    },
    ConditionalReport {
        #[arg(long)]
        root: PathBuf,
    },
    ExposureReport {
        #[arg(long)]
        root: PathBuf,
    },
    #[cfg(feature = "test-support")]
    FixtureNativeCorpus {
        #[arg(long)]
        from: PathBuf,
        #[arg(long)]
        output: PathBuf,
        #[arg(long)]
        split: bool,
    },
    /// Native-only preparation of the bounded input-path parity pair, not a quality study.
    PathPrepare {
        #[arg(long)]
        parent_arm: PathBuf,
        #[arg(long)]
        parent: PathBuf,
        #[arg(long)]
        corpus: PathBuf,
        #[arg(long)]
        legacy_reference: Option<PathBuf>,
        #[arg(long)]
        output: PathBuf,
    },
    /// Read-only comparison of the two authorized input-path endpoints.
    PathVerify {
        #[arg(long)]
        root: PathBuf,
    },
    /// Explicit provenance-backed wire-v1 upgrade; originals and eligibility remain unchanged.
    UpgradeResume {
        #[arg(long)]
        root: PathBuf,
        #[arg(long)]
        terminal: String,
        #[arg(long)]
        output: PathBuf,
    },
    /// Recount already completed train-only teacher probes without model calls.
    ObjectiveProbes {
        #[arg(long)]
        root: PathBuf,
    },
    /// Compile an immutable train-only derivative after the bounded model study.
    TokenCacheCompile {
        #[arg(long)]
        root: PathBuf,
        #[arg(long)]
        output: PathBuf,
    },
    /// Measure equivalent source/snapshot/cache batches; no optimizer or generation.
    TokenCacheMeasure {
        #[arg(long)]
        root: PathBuf,
        #[arg(long)]
        cache: PathBuf,
        #[arg(long)]
        corpus: PathBuf,
        #[arg(long, default_value_t = 3)]
        repetitions: u8,
    },
    /// Register the single BASE/SPAN target-loss study from the preserved A75-R endpoint.
    ObjectivePrepare {
        #[arg(long)]
        parent_arm: PathBuf,
        #[arg(long)]
        expected_parent: String,
        #[arg(long)]
        output: PathBuf,
    },
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
        #[arg(long)]
        cooldown_panel: bool,
        #[arg(long, default_value_t = 1)]
        panel_rows: usize,
        #[arg(long, num_args = 1)]
        objective_span: Option<bool>,
        #[arg(long, requires = "objective_span")]
        nonzero_role: bool,
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
        #[cfg(feature = "test-support")]
        Action::FixtureAuditPublication {
            from,
            output,
            verify,
        } => {
            let s = read_inputs(&from)?;
            if !s.tiny_spec {
                return Err(bad("publication fixture requires TINY"));
            }
            if !verify {
                std::fs::create_dir(&output)?;
                let mut a = ArtifactAudit {
                    source: evaluator_source(),
                    binary: unhex(&file_hash(&std::env::current_exe()?)?)?,
                    input: absolute_reference(&from.join("inputs.r3er"))?,
                    native: s.parent.clone(),
                    originals: vec![],
                    selection: vec![],
                    fresh: vec![],
                    numeric: vec![],
                    panels: vec![],
                    calls: [0; 4],
                    elapsed: Scalar::F64(0.009906131),
                    complete: false,
                    stop: vec![],
                    error: None,
                };
                publish(
                    &output,
                    "audit-start.r3er",
                    &Record::ArtifactAudit(Box::new(a.clone())),
                )?;
                a.complete = true;
                publish_audit_final(&output, &a)?;
            }
            verify_audit_publication(&absolute_reference(&output.join("audit-final.r3er"))?)?;
            println!("AUDIT_PUBLICATION_VERIFIED=true SYNTHETIC_PAYLOAD_ONLY=true MODEL_CALLS=0");
            Ok(())
        }
        #[cfg(feature = "test-support")]
        Action::FixtureAncestorFailed { root } => {
            if !read_inputs(&root)?.tiny_spec {
                return Err(bad("TINY mutation only"));
            }
            let path = "segment-00/command.r3er";
            let Record::Command(mut c) = read_record(&root, &reference(&root, path)?)? else {
                return Err(bad("command"));
            };
            c.status = CommandStatus::Failed;
            c.error = Some("explicit synthetic ancestor failure".into());
            std::fs::write(root.join(path), Record::Command(c).encode()?)?;
            Ok(())
        }
        #[cfg(feature = "test-support")]
        Action::FixtureScreen {
            observation,
            output,
        } => {
            let mut s = read_inputs(&observation)?;
            if !s.tiny_spec || !s.historical {
                return Err(bad("screen fixture requires actual TINY observation"));
            }
            bridge_origins(&s, Some(&observation))?;
            read_preflight_outcome(&observation)?
                .ok_or_else(|| bad("fixture real parent proof"))?
                .require_current_success()?;
            let l = resolve_native(&observation, &s, &s.parent, true)?;
            let framed = samples(
                &s.train
                    .iter()
                    .map(|i| s.cases[*i as usize].clone())
                    .collect::<Vec<_>>(),
                &l.tokenizer,
                l.manifest.training.as_ref().unwrap().config.seq_len,
            )?;
            s.tape =
                anchor_tape_through(&[vec![0], vec![1]], 6, s.parent.counters[2], &framed, 2)?.0;
            s.contract = BOUNDED_BRIDGE_CONTRACT.into();
            s.historical = false;
            s.eval_steps = vec![1, 2];
            s.source = evaluator_source();
            s.run = hash(output.to_string_lossy().as_bytes());
            s.policy = s.run;
            s.parent.run = s.run;
            s.origins
                .retain(|o| o.role != "bridge-replacement-registration");
            let proof = absolute_reference(&observation.join("preflight-final.r3er"))?;
            let fixture_root = output.parent().unwrap().join("screen-fixture-registration");
            std::fs::create_dir(&fixture_root)?;
            // Explicit test scope: the audit reference is a real scope3 TINY parent proof.
            let audit = FileRef {
                locator: fixture_root
                    .canonicalize()?
                    .join("parent-proof.r3er")
                    .display()
                    .to_string(),
                digest: proof.digest,
            };
            std::fs::copy(&proof.locator, &audit.locator)?;
            s.origins.extend([
                Origin {
                    role: "bounded-audit".into(),
                    original: audit.clone(),
                },
                Origin {
                    role: "bounded-prior-inputs".into(),
                    original: absolute_reference(&observation.join("inputs.r3er"))?,
                },
                Origin {
                    role: "bridge-observation-final".into(),
                    original: proof.clone(),
                },
            ]);
            let template = s.cases[panel(&s, PanelKind::Dev)?.cases[0] as usize].clone();
            let mut indices = Vec::new();
            for kind in [
                PanelKind::LegacyDev,
                PanelKind::Cross,
                PanelKind::Ordinary,
                PanelKind::Dev,
            ] {
                let mut e = template.clone();
                e.id = format!("screen-test/{}/0", kind.name());
                let index = s.cases.len() as u32;
                s.cases.push(e);
                indices.push(index);
                s.panels.iter_mut().find(|p| p.kind == kind).unwrap().cases = vec![index];
            }
            s.panels
                .iter_mut()
                .find(|p| p.kind == PanelKind::Watch)
                .unwrap()
                .cases = vec![indices[2]];
            s.panels.push(PanelSpec {
                kind: PanelKind::Screen,
                dataset: hash(b"EXPLICIT_TINY_FOUR_CASE_TEST"),
                cases: indices,
            });
            s.validate()?;
            let raw = evaluate(
                &s,
                &l,
                PanelKind::Screen,
                s.parent.step,
                &mut control,
                false,
                vec![],
            )?;
            screen_scores(&s, &raw, &l)?;
            let registered = output
                .parent()
                .unwrap()
                .canonicalize()?
                .join(output.file_name().unwrap());
            let input = Record::Inputs(Box::new(s));
            let parent = Record::Evaluation(raw);
            publish(
                &fixture_root,
                "t-screen-registration.r3er",
                &Record::ScreenRegistration {
                    root: registered.display().to_string(),
                    input: hash(&input.encode()?),
                    audit,
                    observation: proof,
                    parent: FileRef {
                        locator: registered.join("parent-screen.r3er").display().to_string(),
                        digest: hash(&parent.encode()?),
                    },
                    preparation: Scalar::F64(control.start.elapsed().as_secs_f64()),
                },
            )?;
            std::fs::create_dir(&output)?;
            std::fs::copy(observation.join("parent.r3m"), output.join("parent.r3m"))?;
            publish(&output, "inputs.r3er", &input)?;
            publish(&output, "parent-screen.r3er", &parent)?;
            println!(
                "EXPLICIT_SCREEN_TEST_PARENT GENERATIONS=4 UPDATES=0 PRODUCTION_REPLACEMENT_PROOF=true"
            );
            Ok(())
        }
        Action::ScreenPrepare {
            prior_arm,
            observation,
            audit,
            output,
        } => screen_prepare(&prior_arm, &observation, &audit, &output, &mut control),
        Action::ScreenReport { root, terminal } => screen_report(&root, &terminal, &mut control),
        Action::BridgeArtifactAudit {
            root,
            checkpoint,
            expected,
            output,
        } => bridge_artifact_audit(&root, &checkpoint, &expected, &output, &mut control),
        #[cfg(feature = "test-support")]
        Action::FixtureBridge {
            from,
            output,
            continuous,
            observation,
        } => {
            let original = read_inputs(&from)?;
            if !original.tiny_spec {
                return Err(bad("bridge fixture requires existing TINY"));
            }
            let t = segment(
                &from,
                &reference(&from, "segment-00/terminal.r3er")?,
                &original,
            )?;
            if !t.complete {
                return Err(bad("TINY bootstrap incomplete"));
            }
            let n = t
                .native
                .as_ref()
                .ok_or_else(|| bad("TINY bootstrap native"))?;
            resolve_native(&from, &original, n, true)?;
            let mut s = original.clone();
            s.contract = BRIDGE_CONTRACT.into();
            s.source = evaluator_source();
            s.authorization = None;
            s.historical = false;
            s.objective = None;
            s.purpose = RunPurpose::LrContinuous;
            s.lr_policy = 2;
            s.lr_offset = 0;
            s.eval_steps = vec![1, 2];
            s.tape.truncate(2);
            for (i, d) in s.tape.iter_mut().enumerate() {
                d.sampler = n.counters[2] + i as u64 + 1;
            }
            s.run = hash(output.to_string_lossy().as_bytes());
            s.policy = s.run;
            s.parent = n.clone();
            s.parent.run = s.run;
            s.parent.segment = 0;
            s.parent.file.locator = "parent.r3m".into();
            s.origins.clear();
            // Separate train/dev ordinals for the same production teacher layout.
            s.train = vec![1, 2];
            for e in &mut s.cases {
                e.answer = "c".into();
            }
            s.panels
                .iter_mut()
                .find(|p| p.kind == PanelKind::Watch)
                .unwrap()
                .cases = vec![3];
            if continuous {
                s.origins.push(Origin {
                    role: "bridge-continuous-test".into(),
                    original: FileRef {
                        locator: "parent.r3m".into(),
                        digest: n.file.digest,
                    },
                });
            }
            for kind in [
                PanelKind::LegacyDev,
                PanelKind::Conditional,
                PanelKind::Sanity,
            ] {
                let mut p = panel(&s, PanelKind::Dev)?.clone();
                p.kind = kind;
                s.panels.push(p);
            }
            if observation {
                s.historical = true;
                s.tape.clear();
                s.eval_steps.clear();
                let binary = std::env::current_exe()?;
                s.origins.push(Origin {
                    role: "execution-binary".into(),
                    original: FileRef {
                        locator: binary.canonicalize()?.display().to_string(),
                        digest: unhex(&file_hash(&binary)?)?,
                    },
                });
                // Test-sized predecessor is explicitly incomplete (no generation/teacher).
                // Registration uses the production publisher and verifier, never a tiny bypass.
                let previous = output.with_extension("previous");
                std::fs::create_dir(&previous)?;
                std::fs::copy(from.join(&n.file.locator), previous.join("parent.r3m"))?;
                publish(
                    &previous,
                    "inputs.r3er",
                    &Record::Inputs(Box::new(s.clone())),
                )?;
                let registered = register_bridge_replacement(
                    &output,
                    &previous,
                    s.source,
                    FileRef {
                        locator: binary.canonicalize()?.display().to_string(),
                        digest: unhex(&file_hash(&binary)?)?,
                    },
                )?;
                s.origins.push(Origin {
                    role: "bridge-replacement-registration".into(),
                    original: registered,
                });
            }
            s.validate()?;
            std::fs::create_dir(&output)?;
            std::fs::copy(from.join(&n.file.locator), output.join("parent.r3m"))?;
            if observation {
                std::fs::copy(
                    from.join("segment-00/terminal.r3er"),
                    output.join("parent-command.r3er"),
                )?;
            }
            publish(&output, "inputs.r3er", &Record::Inputs(Box::new(s)))?;
            Ok(())
        }
        Action::BridgeObservePrepare {
            parent_arm,
            checkpoint,
            data,
            conditional,
            output,
            replacement_of,
            previous_binary,
        } => bridge_observe_prepare(
            &parent_arm,
            &checkpoint,
            &data,
            &conditional,
            &output,
            &replacement_of,
            &previous_binary,
            &mut control,
        ),
        Action::BridgeObserve { root } => bridge_observe(&root, &mut control),
        Action::BridgeObserveReport { root } => bridge_observe_report(&root, &mut control),
        Action::BridgePrepare {
            observation,
            output,
        } => bridge_prepare(&observation, &output, &mut control),
        Action::VersionParity {
            parent_arm,
            checkpoint,
            output,
            compare,
        } => version_parity(
            &parent_arm,
            &checkpoint,
            &output,
            compare.as_deref(),
            &mut control,
        ),
        Action::SourceMeasure {
            format,
            root,
            legacy,
            raw,
            cold,
            cache,
            output,
            repetitions,
        } => token_cache::measure_source(
            &root,
            &legacy,
            &raw,
            &cold,
            &cache,
            &output,
            repetitions,
            format.as_deref(),
            &mut control,
        ),
        Action::ConditionalPrepare {
            parent_arm,
            base_arm,
            output,
            seed,
        } => conditional_prepare(&parent_arm, &base_arm, &output, seed, &mut control),
        #[cfg(feature = "test-support")]
        Action::FixtureConditional { from, output } => {
            let s = read_inputs(&from)?;
            if !s.tiny_spec {
                return Err(bad("conditional fixture requires TINY"));
            }
            let _ = resolve_native(&from, &s, &s.parent, true)?;
            let mut n = s.parent.clone();
            n.file.locator = owned_path(&from, &n.file.locator, true)?
                .canonicalize()?
                .display()
                .to_string();
            let mut cases = Vec::new();
            for i in 0..6 {
                let mut e = s.cases[0].clone();
                e.id = format!("conditional-fixture/{i}");
                e.answer = "cccc".into();
                cases.push(e);
            }
            let p = ConditionalRecord {
                registration: None,
                seed: 17,
                source: evaluator_source(),
                plan: None,
                models: vec![n.clone(), n],
                cases,
                foils: vec!["dddd".into(); 6],
                rows: vec![],
                raw_bytes: vec![],
                model_index: 0,
                generations: 0,
                teachers: 0,
                elapsed: Scalar::F64(0.),
                complete: false,
                stop: vec![],
                error: None,
            };
            std::fs::create_dir(&output)?;
            publish(&output, "plan.r3er", &Record::Conditional(Box::new(p)))?;
            Ok(())
        }
        Action::BridgeData {
            parent_arm,
            checkpoint,
            corpus,
            base_arm,
            conditional,
            output,
            seed,
        } => bridge_data(
            &parent_arm,
            &checkpoint,
            &corpus,
            &base_arm,
            &conditional,
            &output,
            seed,
            &mut control,
        ),
        Action::ConditionalRun { root, model } => conditional_run(&root, model, &mut control),
        Action::ConditionalReport { root } => conditional_report(&root, &mut control),
        Action::ExposureReport { root } => exposure_report(&root, &mut control),
        #[cfg(feature = "test-support")]
        Action::FixtureNativeCorpus {
            from,
            output,
            split,
        } => {
            let mut s = read_inputs(&from)?;
            if !s.tiny_spec {
                return Err(bad("native fixture requires TINY"));
            }
            let mut l = resolve_native(&from, &s, &s.parent, true)?;
            let train = vec![s.cases[0].clone()];
            let dev = vec![s.cases[1].clone()];
            let m = data::CorpusManifest {
                version: 1,
                scope: "TINY_PATH_REGRESSION".into(),
                permission: "synthetic test".into(),
                generator: "existing-native-fixture".into(),
                seed: 17,
                split_rule: "disjoint original fixture cases".into(),
                train: data::native::split("train", &train),
                validation: data::native::split("validation", &dev),
            };
            let c = data::native::from_episodes(m, train, dev)?;
            std::fs::create_dir(&output)?;
            data::native::write(&output.join("source.r3c"), &c, true)?;
            let state = l.manifest.training.as_mut().unwrap();
            state.corpus_hash = c.manifest.train.sha256;
            state.validation_hash = c.manifest.validation.sha256;
            state.previous_corpora.push(l.tokenizer.train_hash.clone());
            state.previous_corpora.sort();
            state.previous_corpora.dedup();
            state.config.max_steps = state.step + 2;
            state.config.warmup = 0;
            state.config.validate_every = 100;
            state.resume_binding =
                Some(checkpoint::ResumeBinding::default_for(state, &l.tokenizer));
            checkpoint::save(
                &output.join("parent.r3m"),
                &l.model,
                &l.tokenizer,
                l.manifest.clone(),
                &l.optimizer,
            )?;
            s.contract = NATIVE_CORPUS_CONTRACT.into();
            s.source = evaluator_source();
            s.historical = false;
            s.authorization = None;
            s.objective = None;
            s.purpose = if split {
                RunPurpose::SaveSplit
            } else {
                RunPurpose::SaveContinuous
            };
            s.lr_policy = 2;
            s.lr_offset = 0;
            s.eval_steps.clear();
            s.tape.truncate(2);
            for (i, d) in s.tape.iter_mut().enumerate() {
                d.sampler = l.manifest.training.as_ref().unwrap().sampler_state + i as u64 + 1;
            }
            s.panels
                .iter_mut()
                .find(|p| p.kind == PanelKind::Dev)
                .unwrap()
                .cases = vec![1];
            s.parent = native_reference(&output, "parent.r3m", &s, &l, 0)?;
            s.origins = vec![
                Origin {
                    role: "native-source".into(),
                    original: reference(&output, "source.r3c")?,
                },
                Origin {
                    role: "execution-binary".into(),
                    original: FileRef {
                        locator: std::env::current_exe()?.display().to_string(),
                        digest: unhex(&file_hash(&std::env::current_exe()?)?)?,
                    },
                },
            ];
            publish(&output, "inputs.r3er", &Record::Inputs(Box::new(s)))?;
            println!("NATIVE_FIXTURE=READY OPTIMIZER_CALLS=0 JSON_WRITES=0");
            Ok(())
        }
        Action::PathPrepare {
            parent_arm,
            parent,
            corpus,
            legacy_reference,
            output,
        } => path_prepare(
            &parent_arm,
            &parent,
            &corpus,
            legacy_reference.as_deref(),
            &output,
            &mut control,
        ),
        Action::PathVerify { root } => path_verify(&root, &mut control),
        Action::UpgradeResume {
            root,
            terminal,
            output,
        } => upgrade_resume(&root, &terminal, &output, &mut control),
        Action::ObjectiveProbes { root } => objective_probes(&root, &mut control),
        Action::TokenCacheCompile { root, output } => {
            token_cache::compile(&root, &output, &mut control)
        }
        Action::TokenCacheMeasure {
            root,
            cache,
            corpus,
            repetitions,
        } => token_cache::measure(&root, &cache, &corpus, repetitions, &mut control),
        Action::ObjectivePrepare {
            parent_arm,
            expected_parent,
            output,
        } => cooldown_prepare(
            &parent_arm,
            unhex(&expected_parent)?,
            &output,
            &mut control,
            true,
        ),
        Action::CooldownPrepare {
            parent_arm,
            expected_parent,
            output,
        } => cooldown_prepare(
            &parent_arm,
            unhex(&expected_parent)?,
            &output,
            &mut control,
            false,
        ),
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
            cooldown_panel,
            panel_rows,
            objective_span,
            nonzero_role,
        } => {
            fixture_fork_inner(
                &from,
                &output,
                untrained,
                false,
                RunPurpose::Anchor,
                restart_spec || cooldown_panel || objective_span.is_some(),
            )?;
            if cooldown_panel || panel_rows != 1 || objective_span.is_some() {
                let mut s = read_inputs(&output)?;
                if cooldown_panel {
                    s.contract = COOLDOWN_CONTRACT.into();
                    s.purpose = RunPurpose::LrContinuous;
                    s.lr_policy = 2;
                    s.lr_offset = 0;
                }
                if let Some(span) = objective_span {
                    if nonzero_role {
                        let e = &mut s.cases[s.train[0] as usize];
                        e.answer = "원인은 확정되지 않았습니다.".into();
                        e.category = 4;
                    }
                    let l = resolve_native(&output, &s, &s.parent, true)?;
                    let episodes = s
                        .train
                        .iter()
                        .map(|i| s.cases[*i as usize].clone())
                        .collect::<Vec<_>>();
                    let samples = samples(
                        &episodes,
                        &l.tokenizer,
                        l.manifest.training.as_ref().unwrap().config.seq_len,
                    )?;
                    let a = target_loss::annotations(&episodes, &samples, &l.tokenizer, &[])?;
                    if nonzero_role {
                        if !a.iter().any(|a| a.fractions.iter().any(|f| *f > 0.)) {
                            return Err(bad("nonzero role fixture absent"));
                        }
                        for draw in &mut s.tape {
                            let ids = draw.indices.iter().map(|i| *i as usize).collect::<Vec<_>>();
                            draw.input = batch(&samples, &ids, &Device::Cpu)?.tokens as u64;
                            draw.target = ids
                                .iter()
                                .map(|i| {
                                    (samples[*i].tokens.len() - samples[*i].response_start) as u64
                                })
                                .sum();
                        }
                    }
                    s.contract = OBJECTIVE_CONTRACT.into();
                    s.purpose = RunPurpose::LrContinuous;
                    s.lr_policy = 2;
                    s.lr_offset = 0;
                    s.objective = Some(ObjectivePolicy {
                        span,
                        annotation: hash(&target_loss::annotation_bytes(&a)),
                        probes: vec![s.train[0]],
                    });
                }
                if !(1..=3).contains(&panel_rows) {
                    return Err(bad("TINY panel fixture bound"));
                }
                let mut ordinals = panel(&s, PanelKind::Dev)?.cases.clone();
                for i in 1..panel_rows {
                    let mut e = s.cases[ordinals[0] as usize].clone();
                    e.id = format!("tiny-prefix-{i}");
                    ordinals.push(s.cases.len() as u32);
                    s.cases.push(e);
                }
                s.panels
                    .iter_mut()
                    .find(|p| p.kind == PanelKind::Dev)
                    .unwrap()
                    .cases = ordinals;
                std::fs::write(
                    output.join("inputs.r3er"),
                    Record::Inputs(Box::new(s)).encode()?,
                )?;
            }
            Ok(())
        }
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
    if s.objective.is_some() != t.objective_metrics.is_some()
        || t.objective_metrics
            .as_ref()
            .is_some_and(|m| m.len() != t.draws.len())
    {
        return Err(bad("objective trace policy/count mismatch"));
    }
    if s.continuation() || s.path_parity() {
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
                (if s.objective.is_some() || s.path_parity() || s.bridge() {
                    Ok(1e-4)
                } else {
                    cooldown_lr(s.lr_policy, first + i as u64 + 1)
                })
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
    if s.screen() && dev.kind == PanelKind::Screen {
        if dev.step != watch.step || d != w {
            return Err(bad("screen decision identity"));
        }
        let (_, parent, _) = screen_parent(s)?;
        let parent_root = PathBuf::from(
            &s.origins
                .iter()
                .find(|o| o.role == "bounded-prior-inputs")
                .ok_or_else(|| bad("screen original parent"))?
                .original
                .locator,
        );
        let parent_model = resolve_native(parent_root.parent().unwrap(), s, &s.parent, true)?;
        let base = screen_scores(s, &parent, &parent_model)?;
        let scores = screen_scores(s, dev, l)?;
        let (after, stop) = screen_policy(&base, &scores, before, dev.new_updates);
        println!(
            "T_SCREEN_RESULT updates={} OLD={}/{} CROSS={}/{} QA={}/{} NEW={}/{} NEW_BASE4={}/{} ERRORS={} parent={:?} STOP={stop:?}",
            dev.new_updates,
            scores[0].exact,
            scores[0].planned,
            scores[1].exact,
            scores[1].planned,
            scores[2].exact,
            scores[2].planned,
            scores[3].exact,
            scores[3].planned,
            scores[3].base[0],
            scores[3].base[1],
            scores.iter().map(|s| s.errors).sum::<u64>(),
            base.iter().map(|s| s.exact).collect::<Vec<_>>()
        );
        return Ok(EvalDecision {
            run: s.run,
            binding: s.binding(),
            dev: d,
            watch: w,
            step: dev.step,
            before,
            after,
            applied: true,
            quality_stop: stop.is_some(),
        });
    }
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
fn preserves_partial_prefix(previous: &EvalPayload, next: &EvalPayload) -> bool {
    let prefix = previous
        .rows
        .iter()
        .take_while(|r| r.completed && r.interruption.is_none())
        .collect::<Vec<_>>();
    prefix.len() < previous.expected as usize
        && previous.model == next.model
        && previous.run == next.run
        && previous.binding == next.binding
        && previous.tokenizer == next.tokenizer
        && previous.architecture == next.architecture
        && previous.source == next.source
        && previous.step == next.step
        && previous.new_updates == next.new_updates
        && previous.kind == next.kind
        && previous.expected == next.expected
        && next.rows.len() >= prefix.len()
        && next.rows.len() <= next.expected as usize
        && prefix.iter().zip(&next.rows).all(|(a, b)| {
            let mut x = Vec::new();
            let mut y = Vec::new();
            a.encode(&mut x);
            b.encode(&mut y);
            x == y
        })
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
    for (segment_index, (_, t)) in chain.iter().enumerate() {
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
                let origin = chain[..segment_index]
                    .iter()
                    .rev()
                    .find(|(_, t)| t.evaluations.iter().any(|r| r.payload == old.payload))
                    .ok_or_else(|| bad("same-segment duplicate raw observation"))?;
                if !s.supports_partial_resume()
                    || effective_outcome(root, s, &origin.0)?.status != CommandStatus::TimePause
                    || origin.1.save_error.is_some()
                    || !preserves_partial_prefix(&previous, &e)
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
            let screening = s.screen() && d.step < s.parent.step + s.tape.len() as u64;
            let dr = h
                .evaluations
                .get(&(
                    d.step,
                    if screening {
                        PanelKind::Screen
                    } else {
                        PanelKind::Dev
                    },
                ))
                .ok_or_else(|| bad("guard dev missing"))?;
            let wr = h
                .evaluations
                .get(&(
                    d.step,
                    if screening {
                        PanelKind::Screen
                    } else {
                        PanelKind::Watch
                    },
                ))
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
                matches!(kind, PanelKind::Dev | PanelKind::Screen)
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
        || scores.len() != if s.bridge() { 6 } else { 4 }
    {
        return false;
    }
    let find = |kind| scores.iter().find(|(k, _)| *k == kind).map(|(_, v)| v);
    let (Some(d), Some(c), Some(o), Some(w)) = (
        find(if s.bridge() {
            PanelKind::LegacyDev
        } else {
            PanelKind::Dev
        }),
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
        && [d, c, o, w].iter().all(|p| p.errors == 0)
        && scores
            .iter()
            .map(|(_, p)| p)
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

fn verify_ancestor_commands(
    root: &Path,
    s: &RunSnapshot,
    chain: &[(FileRef, SegmentReceipt)],
) -> Result<()> {
    if !s.historical {
        for (r, _) in chain.iter().take(chain.len().saturating_sub(1)) {
            if effective_outcome(root, s, r)?.status != CommandStatus::TimePause {
                return Err(bad("ancestor command is not a verified time pause"));
            }
        }
    }
    Ok(())
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
    verify_ancestor_commands(root, &s, &chain)?;
    for (i, (_, stored)) in chain.iter().enumerate() {
        let continued_time_stop = i + 1 < chain.len()
            && stored.resume
            && !stored.complete
            && stored.stop == [StopReason::TimeBudget];
        if !stored.stop.is_empty() && !continued_time_stop {
            return Err(bad("stored terminal stop forbids normal close"));
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
    } else if s.bridge() {
        &[
            PanelKind::Dev,
            PanelKind::Watch,
            PanelKind::Cross,
            PanelKind::Ordinary,
            PanelKind::LegacyDev,
            PanelKind::Conditional,
        ]
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
    let started = Instant::now();
    let path = owned_path(root, name, false)?;
    save_arm(l, state, adam, &path, reason)?;
    let write_seconds = started.elapsed().as_secs_f64();
    let durable = checkpoint::load(&path, Device::Cpu, true)?;
    let reference = native_reference(root, name, s, &durable, index)?;
    if s.objective.is_some() {
        println!(
            "OBJECTIVE_IO operation=save_and_validate step={} file={name} bytes={} write_sync_s={write_seconds} reload_hash_validate_s={}",
            state.step,
            std::fs::metadata(&path)?.len(),
            started.elapsed().as_secs_f64() - write_seconds
        );
    }
    Ok(reference)
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
    objective: bool,
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
    let horizon = if objective { 512 } else { 256 };
    let objective_annotations = if objective {
        let episodes = old
            .train
            .iter()
            .map(|i| old.cases[*i as usize].clone())
            .collect::<Vec<_>>();
        let mut semantics = BTreeMap::new();
        for e in &episodes {
            if e.family.starts_with("copy/") {
                continue;
            }
            let (status, reason) = target_semantics(&e.request, &e.answer);
            *semantics.entry(format!("{status:?}")).or_insert(0usize) += 1;
            if matches!(status, SemanticState::Contradicted) {
                println!("DATA_CONTRACT_FAIL reason={reason} case={e:?}");
                return Err(bad(
                    "DATA_CONTRACT_FAIL: existing target semantics contradicted; no label repair or learning",
                ));
            }
        }
        println!("TRAIN_LABEL_SEMANTICS={semantics:?}");
        let annotations =
            target_loss::annotations(&episodes, &framed, &l.tokenizer, &auth.pools[1])?;
        for (pool, indices) in auth.pools.iter().enumerate() {
            let supported = indices
                .iter()
                .filter(|i| annotations[**i as usize].supported)
                .count();
            println!(
                "ANNOTATION_POOL={pool} CASES={} SUPPORTED={supported} FALLBACK={} FIRST_TARGET_WEIGHT={}",
                indices.len(),
                indices.len() - supported,
                state.config.first_target_weight
            );
            type TaskStats = (usize, usize, usize, usize, f64, BTreeSet<String>);
            let mut tasks: BTreeMap<usize, TaskStats> = BTreeMap::new();
            for &i in indices {
                let e = &episodes[i as usize];
                let n = framed[i as usize].tokens.len() - framed[i as usize].response_start;
                let v =
                    tasks
                        .entry(e.category)
                        .or_insert((0, 0, usize::MAX, 0, 0., BTreeSet::new()));
                v.0 += 1;
                v.1 += n;
                v.2 = v.2.min(n);
                v.3 = v.3.max(n);
                v.4 += n as f64 - 1. + state.config.first_target_weight as f64;
                v.5.insert(scene(e).to_string());
            }
            for (task, (cases, targets, min, max, mass, bases)) in tasks {
                println!(
                    "TRAIN_POOL={pool} category={task} cases={cases} unique_bases={} targets={targets} target_len_min={min} target_len_max={max} target_len_mean={} baseline_mass={mass}",
                    bases.len(),
                    targets as f64 / cases as f64
                );
            }
        }
        Some(hash(&target_loss::annotation_bytes(&annotations)))
    } else {
        None
    };
    let mut probes = old.train.clone();
    probes.sort_by_key(|i| {
        let e = &old.cases[*i as usize];
        let mut key = 17u64.to_le_bytes().to_vec();
        put_varint(&mut key, e.category as u64);
        for text in [&e.family, scene(e), &e.id] {
            string(&mut key, text);
        }
        hash(&key)
    });
    probes.truncate(64);
    let (continued, _) = anchor_tape_through(
        &auth.pools,
        6,
        old.parent.counters[2],
        &framed,
        512 + horizon,
    )?;
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
    string(
        &mut material,
        if objective {
            OBJECTIVE_CONTRACT
        } else {
            COOLDOWN_CONTRACT
        },
    );
    if let Some(annotation) = objective_annotations {
        material.extend(annotation);
        integers(&mut material, &probes);
    }
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
        s.contract = if objective {
            OBJECTIVE_CONTRACT
        } else {
            COOLDOWN_CONTRACT
        }
        .into();
        s.objective = objective_annotations.map(|annotation| ObjectivePolicy {
            span: lr == 3,
            annotation,
            probes: probes.clone(),
        });
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
        s.lr_policy = if objective { 2 } else { lr };
        s.lr_offset = 0;
        s.eval_steps = vec![(horizon / 2) as u32, horizon as u32];
        s.purpose = if objective {
            RunPurpose::LrContinuous
        } else {
            RunPurpose::LrSplit
        };
        s.baseline = [dev.exact, watch.exact, dev.errors + watch.errors];
        s.authorization = Some(AnchorAuthorization {
            pair,
            baseline: command.terminal.digest,
            expected_parent: expected,
            anchors: 6,
            pools: auth.pools.clone(),
        });
        s.validate()?;
        for n in [1, horizon as usize / 2, horizon as usize] {
            let mut future = state.clone();
            fork_budget(
                &mut future,
                s.parent.step,
                s.parent.counters[0],
                horizon as usize,
                if objective { 2_000_000 } else { 1_000_000 },
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
                if objective {
                    1e-4f64.to_bits()
                } else {
                    cooldown_lr(lr, n as u64)?.to_bits()
                }
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
            "REGISTERED ARM={} source={} binary={} binding={} parent={} model={} Adam={} cursor={} tape=EXACT_A75_CONTINUATION HORIZON={horizon} predicted_input={} predicted_target={} SMALL_UPDATES=0",
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
    let study = read_inputs(&root.join(if root.join("C-COPYMATCH").exists() {
        "C-COPYMATCH"
    } else if root.join("B-BASE").exists() {
        "B-BASE"
    } else {
        "K-KEEP"
    }))?;
    let arms = study.study_arms();
    if study.bridge() && !confirmation {
        return Err(bad(
            "bridge uses full parent observation, not additional parity",
        ));
    }
    if !study.continuation() {
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
        for arm in arms {
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
            let signal = if study.bridge() {
                let other = read_inputs(&root.join(arms[0]))?;
                let closed = arm_commands(&root.join(arms[0]), &other)?;
                let Record::Comparison(c) = read_record(
                    &root.join(arms[0]),
                    closed
                        .last()
                        .unwrap()
                        .1
                        .comparison
                        .as_ref()
                        .ok_or_else(|| bad("bridge control comparison"))?,
                )?
                else {
                    return Err(bad("bridge control comparison kind"));
                };
                let get = |rows: &[(PanelKind, Score)], k| {
                    rows.iter()
                        .find(|(kind, _)| *kind == k)
                        .map(|(_, v)| v.clone())
                        .ok_or_else(|| bad("bridge required score"))
                };
                let dev = get(&score.panels, PanelKind::Dev)?;
                *arm == arms[1]
                    && dev.exact >= 192
                    && dev.exact as i64 - get(&c.panels, PanelKind::Dev)?.exact as i64 >= 26
                    && dev.base[0] >= 40
                    && dev.errors == 0
                    && get(&score.panels, PanelKind::Ordinary)?.qa[0] >= 178
            } else {
                true
            };
            if score.candidate && signal {
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
                        if study.objective.is_some() {
                            get(PanelKind::Ordinary)?.qa[0]
                        } else {
                            get(PanelKind::Dev)?.exact
                        },
                        if study.objective.is_some() {
                            get(PanelKind::Dev)?.exact
                        } else {
                            get(PanelKind::Ordinary)?.qa[0]
                        },
                        u8::from(*arm == arms[0]),
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
        (dir, s, n, c, None, if study.bridge() { 4 } else { 2 })
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
            reference(root, &format!("{}/inputs.r3er", arms[0]))?,
            reference(root, &format!("{}/inputs.r3er", arms[1]))?,
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
            reference(root, &format!("{}/parent.r3m", arms[0]))?,
            reference(root, &format!("{}/parent.r3m", arms[1]))?,
        ]
    };
    let specs = if confirmation {
        [PanelKind::Dev, PanelKind::Cross, PanelKind::Ordinary]
            .iter()
            .map(|k| {
                Ok((
                    *k,
                    if study.bridge() {
                        metadata_sample(&snapshot, *k, 16)?
                    } else {
                        panel(&snapshot, *k)?.cases.clone()
                    },
                ))
            })
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
        publication_v2: true,
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
                    anchor_budget(&root.join(arms[1]), &read_inputs(&root.join(arms[1]))?)?;
                if g + start.order.len() > if study.bridge() { 4608 } else { 4096 } {
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
                        read_inputs(&root.join(arms[1]))?
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
                    new_updates: if confirmation {
                        snapshot.tape.len() as u64
                    } else {
                        0
                    },
                    kind: *kind,
                    expected: cases.len() as u32,
                    rows,
                };
                if confirmation && !study.bridge() {
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
            if confirmation && !study.bridge() {
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
                    teacher_evidence: None,
                    elapsed: Scalar::F64(control.start.elapsed().as_secs_f64()),
                    generations: control.generation_calls as u64,
                }),
            )?;
            Ok(())
        },
    )
}

fn anchor_budget(root: &Path, s: &RunSnapshot) -> Result<(u64, usize, f64)> {
    if s.screen() {
        return screen_budget(root, s);
    }
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
    let study_arms = s.study_arms();
    let arms: &[&str] = if s.continuation() {
        study_arms
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
        if s.continuation()
            && (other.contract != s.contract
                || other.source != s.source
                || other.parent.file.digest != s.parent.file.digest
                || other.parent.counters != s.parent.counters
                || other.train != s.train
                || !s.bridge()
                    && other.cases.iter().map(case_hash).collect::<Vec<_>>()
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
        if s.bridge() {
            bridge_origins(&other, None)?;
            let l = resolve_native(root, s, &s.parent, true)?;
            for (i, (x, y)) in other.cases.iter().zip(&s.cases).enumerate() {
                if !(2048..2560).contains(&i) || other.arm_name() == s.arm_name() {
                    if case_hash(x) != case_hash(y) {
                        return Err(bad("bridge shared cases changed"));
                    }
                } else if other.arm_name() == "C-COPYMATCH" {
                    data::bridge_pair(x, y, &l.tokenizer)?;
                } else {
                    data::bridge_pair(y, x, &l.tokenizer)?;
                }
            }
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
    if s.bridge() {
        let path = origin_path(s, "bridge-observation-final")?;
        let observation = path
            .parent()
            .ok_or_else(|| bad("bridge observation root"))?;
        let p = read_preflight_outcome(observation)?
            .ok_or_else(|| bad("bridge observation missing"))?;
        p.require_current_success()?;
        generations += p.entries as usize;
        seconds += p.elapsed + p.publication_reserve;
    }
    if !s.preflight()
        && !s.bridge()
        && matches!(
            s.contract.as_str(),
            RESTART_CONTRACT
                | COOLDOWN_CONTRACT
                | OBJECTIVE_CONTRACT
                | NATIVE_CORPUS_CONTRACT
                | BRIDGE_CONTRACT
                | BOUNDED_BRIDGE_CONTRACT
        )
    {
        let p = read_preflight_outcome(parent)?.ok_or_else(|| bad("missing preflight outcome"))?;
        generations += p.entries as usize;
        seconds += p.elapsed + p.publication_reserve;
        println!(
            "PAIR_USAGE_KNOWN updates={updates} generations={generations} charged_seconds={seconds} verification_elapsed_lower_bound={} publication_reserve_seconds={}",
            p.elapsed, p.publication_reserve
        );
        p.require_current_success()?;
        if s.continuation() {
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
                        reference(parent, &format!("{}/inputs.r3er", study_arms[0]))?,
                        reference(parent, &format!("{}/inputs.r3er", study_arms[1]))?,
                    ]
            {
                return Err(bad("parent parity study binding"));
            }
        }
    }
    if updates
        > if s.objective.is_some() {
            1024
        } else if s.cooldown() {
            512
        } else if s.contract == RESTART_CONTRACT {
            1028
        } else {
            1024
        }
        || generations
            > if s.bridge() {
                4608
            } else if s.continuation() {
                4096
            } else {
                7500
            }
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
        publication_v2: true,
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
    // Readers serialize with the final publisher on the immutable intent inode.
    // A live publisher is not a completed authorization, even if final bytes are visible.
    let _lock = if start_path.exists() {
        let file = std::fs::File::open(&start_path)?;
        file.try_lock_shared()
            .map_err(|e| Error::Conflict(format!("verification publication in progress: {e}")))?;
        Some(file)
    } else {
        None
    };
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
                    || n.starts_with("preflight-publication")
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
    if start.scope == 3 {
        bridge_origins(&read_inputs(root)?, Some(root))?;
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
    let pending = root.join("preflight-publication-pending.r3er");
    if pending.exists() {
        let Record::VerificationPending {
            start: pending_start,
            final_digest,
        } = read_record(
            root,
            &reference(root, "preflight-publication-pending.r3er")?,
        )?
        else {
            return Err(bad("publication pending kind"));
        };
        if !start.publication_v2
            || pending_start != sr
            || root.join("preflight-final.r3er").exists()
                && reference(root, "preflight-final.r3er")?.digest != final_digest
        {
            return Err(bad("publication pending/final attempt conflict"));
        }
    }
    if pending.exists() || !root.join("preflight-final.r3er").exists() {
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
        || start.scope != 3 && f.teachers != 0
    {
        return Err(bad("verification final usage/raw binding"));
    }
    if f.succeeded {
        if start.scope == 3 {
            let Record::Inputs(s) = read_record(root, &start.inputs[0])? else {
                return Err(bad("bridge teacher intent snapshot"));
            };
            let expected = bridge_probe_indices(&s)?.len() as u64;
            if !s.bridge() || !s.historical || f.teachers != expected {
                return Err(bad("bridge teacher completed usage"));
            }
        }
        let proof_ref = f
            .proof
            .as_ref()
            .ok_or_else(|| bad("verification success without proof"))?;
        let Record::Preflight(p) = read_record(root, proof_ref)? else {
            return Err(bad("verification final proof kind"));
        };
        if start.scope == 3 {
            let Record::Inputs(s) = read_record(root, &start.inputs[0])? else {
                return Err(bad("bridge teacher inputs"));
            };
            let teacher = read_verified_bridge_teacher(root, "parent-probe", &s, &s.parent)?;
            if p.teacher_evidence.as_ref() != Some(&teacher.files)
                || f.teachers != teacher.rows.len() as u64
                || start.source != s.source
                || start.pair != s.run
                || start.natives[0] != s.parent.file
            {
                return Err(bad("bridge teacher proof references/observation identity"));
            }
            println!(
                "TEACHER_EVIDENCE_VERIFIED={} FILES={} EXECUTION_SOURCE={} RECOUNT_SOURCE={}",
                teacher.rows.len(),
                teacher.files.len(),
                hex(&s.source),
                hex(&evaluator_source())
            );
        } else if p.teacher_evidence.is_some() {
            return Err(bad("unexpected teacher binding outside scope3"));
        }
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
        if p.panels.len() != if matches!(start.scope, 2 | 4) { 3 } else { 2 } {
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
                    if matches!(start.scope, 2 | 4) {
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
                let Record::Inputs(s) = read_record(
                    root,
                    &start.inputs[if matches!(start.scope, 2 | 4) { 0 } else { arm }],
                )?
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
                if start.scope == 2 || start.scope == 3 {
                    rescore(&s, &e, l, true)?;
                } else if e.rows.iter().map(|r| r.ordinal).collect::<Vec<_>>()
                    != metadata_sample(&s, e.kind, if start.scope == 4 { 16 } else { 8 })?
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
                let key = (
                    if matches!(start.scope, 2 | 4) { 0 } else { arm },
                    row.ordinal,
                );
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
        legacy: !start.publication_v2,
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
    let publication_lock = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open(root.join("preflight-start.r3er"))?;
    publication_lock
        .try_lock()
        .map_err(|e| Error::Conflict(format!("verification writer: {e}")))?;
    println!(
        "VERIFICATION_BEGIN_DURABLE=true CONTRACT={} ATTEMPT_ID={} RESERVED_UPPER_BOUND={} UNKNOWN_TAIL=true",
        if matches!(start.scope, 3 | 4) {
            BRIDGE_CONTRACT
        } else {
            COOLDOWN_CONTRACT
        },
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
        start: sr.clone(),
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
    let final_record = Record::VerificationFinal(f);
    // Pending is durable BEFORE publication. No error path removes this blocker.
    publish(
        root,
        "preflight-publication-pending.r3er",
        &Record::VerificationPending {
            start: sr,
            final_digest: hash(&final_record.encode()?),
        },
    )?;
    #[cfg(feature = "test-support")]
    if tiny && std::env::var("R3ER_TEST_STOP").as_deref() == Ok("pv-pending-kill") {
        std::process::exit(94);
    }
    let saved = verification_fault(tiny, "final", control)
        .and_then(|_| publish(root, "preflight-final.r3er", &final_record));
    println!(
        "VERIFICATION_COMMAND_ELAPSED={} FINAL_PUBLICATION_OK={} ORIGINAL_ERROR={:?} PUBLICATION_ERROR={:?}",
        control.start.elapsed().as_secs_f64(),
        saved.is_ok(),
        result.as_ref().err(),
        saved.as_ref().err()
    );
    saved?;
    verification_fault(tiny, "pending-release", control)?;
    // Authorization linearizes at unlink under the writer lock, only after final
    // file AND directory sync succeeded. Unlink failure leaves Pending blocked.
    std::fs::remove_file(root.join("preflight-publication-pending.r3er"))?;
    // Final is already durable. Cleanup sync failure may resurrect Pending after
    // a crash (fail closed); it cannot lose the committed final. This is a warning,
    // never a command failure that a later reader could launder into success.
    let cleanup = verification_fault(tiny, "cleanup", control)
        .and_then(|_| std::fs::File::open(root)?.sync_all().map_err(Error::from));
    if let Err(e) = cleanup {
        eprintln!("PUBLICATION_COMMITTED=true CLEANUP_WARNING={e}");
    }
    drop(publication_lock);
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
    let mut at = al
        .manifest
        .training
        .as_ref()
        .ok_or_else(|| bad("preflight training state"))?
        .clone();
    let mut bt = bl
        .manifest
        .training
        .as_ref()
        .ok_or_else(|| bad("preflight training state"))?
        .clone();
    if !legacy
        && (at.resume_binding.as_ref() != Some(&objective_binding(a, &at, &al.tokenizer)?)
            || bt.resume_binding.as_ref() != Some(&objective_binding(b, &bt, &bl.tokenizer)?))
    {
        return Err(bad("preflight objective/policy mismatch"));
    }
    // Each policy is validated above. Continuous/split purpose is provenance,
    // and must not obscure equality of the executed numeric state.
    at.resume_binding = None;
    bt.resume_binding = None;
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
                teacher_evidence: None,
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
        if case.family.starts_with("bridge/") {
            for key in case.family.split('/').filter(|v| {
                v.starts_with("digits-")
                    || v.starts_with("repeated-")
                    || v.starts_with("order-")
                    || v.starts_with("newer-id-larger-")
                    || v.starts_with("kind-")
            }) {
                let counts = strata.entry(key.to_string()).or_default();
                counts[0] += u64::from(exact);
                counts[1] += 1;
            }
            let key = if case.request.input.contains("현재 유효한") {
                "question/current"
            } else {
                "question/past"
            };
            let counts = strata.entry(key.into()).or_default();
            counts[0] += u64::from(exact);
            counts[1] += 1;
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
) -> Result<[[u64; 4]; 4]> {
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
    let am = marks(a)?;
    let bm = marks(b)?;
    let view = paired_counts(&am, &bm)?;
    let bases = |s: &RunSnapshot, e: &EvalPayload, marks: &[(Hash, bool, bool)]| {
        let mut bases: BTreeMap<String, (Vec<Hash>, bool)> = BTreeMap::new();
        for (row, mark) in e.rows.iter().zip(marks) {
            let key = scene(&s.cases[row.ordinal as usize]).to_string();
            let value = bases.entry(key).or_insert((Vec::new(), true));
            value.0.push(mark.0);
            value.1 &= mark.1;
        }
        bases
    };
    let ab = bases(a.0, a.1, &am);
    let bb = bases(b.0, b.1, &bm);
    if ab.len() != bb.len() {
        return Err(bad("paired base denominator"));
    }
    let mut base = [0; 4];
    for (key, a) in &ab {
        let b = bb.get(key).ok_or_else(|| bad("paired base identity"))?;
        if a.0 != b.0 {
            return Err(bad("paired base membership"));
        }
        base[usize::from(a.1) * 2 + usize::from(b.1)] += 1;
    }
    Ok([view[0], view[1], view[2], base])
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
    let bridge = root.join("C-COPYMATCH").exists();
    let cooldown = root.join("K-KEEP").exists();
    let objective = root.join("B-BASE").exists();
    if cooldown || objective {
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
    let arms = if bridge {
        ["C-COPYMATCH", "T-TEMPORAL"]
    } else if objective {
        ["B-BASE", "S-SPAN"]
    } else if cooldown {
        ["K-KEEP", "D-DECAY"]
    } else if root.join("C50-R").exists() {
        ["C50-R", "A75-R"]
    } else {
        ["C50", "A75"]
    };
    for arm in arms {
        let dir = root.join(arm);
        let s = read_inputs(&dir)?;
        if bridge {
            bridge_origins(&s, None)?;
        }
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
        if s.objective.is_some() {
            let metrics = chain
                .iter()
                .flat_map(|(_, t)| t.objective_metrics.iter().flatten())
                .collect::<Vec<_>>();
            let mut sums = [0.; 8];
            for row in &metrics {
                for (sum, value) in sums.iter_mut().zip(row.iter()) {
                    *sum += value;
                }
            }
            println!(
                "OBJECTIVE_METRICS arm={arm} actual_updates={} preparation_s={} forward_loss_s={} backward_s={} optimizer_s={} mean_gradient_norm={} mean_update_norm={} mean_training_objective={} clipped={} clip_fraction={}",
                metrics.len(),
                sums[0],
                sums[1],
                sums[2],
                sums[3],
                sums[4] / metrics.len() as f64,
                sums[5] / metrics.len() as f64,
                sums[6] / metrics.len() as f64,
                sums[7],
                sums[7] / metrics.len() as f64
            );
        }
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
            for kind in if bridge {
                vec![
                    PanelKind::Dev,
                    PanelKind::LegacyDev,
                    PanelKind::Cross,
                    PanelKind::Ordinary,
                    PanelKind::Conditional,
                ]
            } else {
                vec![PanelKind::Dev, PanelKind::Cross, PanelKind::Ordinary]
            } {
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
        if bridge {
            reference(&dir, "final-probe-final.r3er")?;
            print_bridge_teacher(
                &read_verified_bridge_teacher(&dir, "final-probe", &s, native)?,
                native.step,
            )?;
            let observation = origin_path(&s, "bridge-observation-final")?;
            let observation = observation.parent().unwrap();
            read_preflight_outcome(observation)?
                .ok_or_else(|| bad("bridge observation missing"))?
                .require_current_success()?;
            let parent = read_inputs(observation)?;
            let pl = resolve_native(observation, &parent, &parent.parent, true)?;
            reference(observation, "parent-probe-final.r3er")?;
            print_bridge_teacher(
                &read_verified_bridge_teacher(
                    observation,
                    "parent-probe",
                    &parent,
                    &parent.parent,
                )?,
                parent.parent.step,
            )?;
            for kind in [PanelKind::Sanity, PanelKind::Dev] {
                let Record::Evaluation(e) = read_record(
                    observation,
                    &reference(observation, &format!("{}.r3er", kind.name()))?,
                )?
                else {
                    return Err(bad("bridge parent raw"));
                };
                print_anchor_panel(&parent, &e, &pl)?;
            }
        }
        if cooldown || objective {
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
                let endpoint = s.tape.len() as u64;
                let (b, bl) = payload(&dir, &s, &h.evaluations[&(s.parent.step + endpoint, kind)])?;
                let counts = paired_panel((&old, &a, &al), (&s, &b, &bl))?;
                println!(
                    "PAIRED parent_A75_R512_to={arm} updates={endpoint} panel={} both_wrong/gain/loss/both_correct={:?} base_all_views={:?}",
                    kind.name(),
                    counts[0],
                    counts[3]
                );
                if kind == PanelKind::Ordinary {
                    println!(
                        "PAIRED parent_A75_R512_to={arm} updates={endpoint} QA336={:?} AUX64={:?}",
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
        for kind in if bridge {
            vec![
                PanelKind::Dev,
                PanelKind::LegacyDev,
                PanelKind::Cross,
                PanelKind::Ordinary,
                PanelKind::Conditional,
            ]
        } else {
            vec![PanelKind::Dev, PanelKind::Cross, PanelKind::Ordinary]
        } {
            let mut rows = Vec::new();
            for (i, (s, h, _)) in endpoints.iter().enumerate() {
                if let Some(r) = h.evaluations.get(&(s.parent.step + n, kind)) {
                    let (e, l) = payload(&root.join(arms[i]), s, r)?;
                    rows.push((s, e, l));
                }
            }
            if rows.len() == 2 {
                let counts = paired_panel(
                    (rows[0].0, &rows[0].1, &rows[0].2),
                    (rows[1].0, &rows[1].1, &rows[1].2),
                )?;
                if bridge && n == 512 && kind == PanelKind::Dev {
                    let t = rescore(rows[1].0, &rows[1].1, &rows[1].2, true)?;
                    let ts = &endpoints[1].0;
                    let th = &endpoints[1].1;
                    let (oe, ol) = payload(
                        &root.join(arms[1]),
                        ts,
                        &th.evaluations[&(ts.parent.step + 512, PanelKind::Ordinary)],
                    )?;
                    let ordinary = rescore(ts, &oe, &ol, true)?;
                    let signal = t.exact >= 192
                        && counts[0][1] as i64 - counts[0][2] as i64 >= 26
                        && t.base[0] >= 40
                        && t.errors == 0
                        && ordinary.qa[0] >= 178;
                    println!(
                        "BINDING_LEARNING_SIGNAL={signal} primary_T_minus_C={} full={} base4={:?} QA={:?} H3_JOINT={:?} MODEL_QUALITY_RECOVERED={} GOAL1_READY=false H3_SEAL=NOT_OPENED",
                        counts[0][1] as i64 - counts[0][2] as i64,
                        t.exact,
                        t.base,
                        ordinary.qa,
                        endpoints.iter().map(|x| x.2).collect::<Vec<_>>(),
                        signal && endpoints[1].2
                    );
                    for (s, e, l) in &rows {
                        let mut query = [0u64; 2];
                        let mut swap = [0u64; 2];
                        let mut changes = [0u64; 2];
                        for quartet in e.rows.as_chunks::<4>().0 {
                            let outputs = quartet
                                .iter()
                                .map(|r| row_output(r, l))
                                .collect::<Result<Vec<_>>>()?;
                            let marks = quartet
                                .iter()
                                .zip(&outputs)
                                .map(|(r, (a, abnormal))| {
                                    !abnormal
                                        && a.as_deref()
                                            == Some(s.cases[r.ordinal as usize].answer.as_str())
                                })
                                .collect::<Vec<_>>();
                            for (a, b) in [(0, 1), (2, 3)] {
                                query[0] += u64::from(marks[a] && marks[b]);
                                query[1] += 1;
                                changes[0] += u64::from(outputs[a].0 != outputs[b].0);
                                changes[1] += 1;
                            }
                            for (a, b) in [(0, 2), (1, 3)] {
                                swap[0] += u64::from(marks[a] && marks[b]);
                                swap[1] += 1;
                            }
                        }
                        println!(
                            "BRIDGE_JOINT arm={} query_change_both_correct={query:?} value_swap_both_correct={swap:?} query_output_change={changes:?}",
                            s.arm_name()
                        );
                    }
                }
                println!(
                    "PAIRED updates={n} panel={} both_wrong/gain/loss/both_correct={:?} base_all_views={:?}",
                    kind.name(),
                    counts[0],
                    counts[3]
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
fn objective_probes(root: &Path, control: &mut RunControl) -> Result<()> {
    for (arm, label) in [
        ("B-BASE", "parent"),
        ("B-BASE", "final"),
        ("S-SPAN", "final"),
    ] {
        control.check("objective_probe_recount")?;
        let dir = root.join(arm);
        let s = read_inputs(&dir)?;
        let policy = s
            .objective
            .as_ref()
            .ok_or_else(|| bad("probe report policy"))?;
        let Record::TrainProbe(p) = read_record(
            &dir,
            &reference(&dir, &format!("train-{label}-final.r3er"))?,
        )?
        else {
            return Err(bad("probe report kind"));
        };
        let native = if label == "parent" {
            s.parent.clone()
        } else {
            let commands = arm_commands(&dir, &s)?;
            let (t, c) = commands
                .last()
                .ok_or_else(|| bad("probe report missing command"))?;
            if c.status != CommandStatus::Complete || !t.complete {
                return Err(bad("probe report incomplete endpoint"));
            }
            t.native
                .clone()
                .ok_or_else(|| bad("probe report native absent"))?
        };
        if !p.complete
            || p.source != s.source
            || p.binding != s.binding()
            || p.native != native
            || p.rows.iter().map(|(i, _)| *i).collect::<Vec<_>>() != policy.probes
            || unhex(&file_hash(&owned_path(&dir, &native.file.locator, true)?)?)?
                != native.file.digest
        {
            return Err(bad("probe report native/source/panel binding"));
        }
        let mut targets = 0u64;
        let mut correct = 0u64;
        let mut exact = 0u64;
        let mut base = 0.;
        let mut span = 0.;
        let mut mass = [0.; 6];
        let mut nll = [0.; 6];
        let mut weighted = [0.; 6];
        let l = resolve_native(&dir, &s, &native, false)?;
        let (_, annotations) = token_cache::framed(&s, &l)?;
        let mut token_mass = [0.; 6];
        let mut first = [0u64; 7];
        let mut tasks: BTreeMap<usize, [u64; 3]> = BTreeMap::new();
        for (ordinal, r) in &p.rows {
            let i = s
                .train
                .iter()
                .position(|i| i == ordinal)
                .ok_or_else(|| bad("probe train ordinal"))?;
            if annotations[i].roles.len() != r.targets as usize {
                return Err(bad("probe target denominator"));
            }
            let mut expected_mass = [0.; 6];
            for (j, roles) in annotations[i].roles.iter().enumerate() {
                for k in 0..6 {
                    token_mass[k] += roles[k];
                    expected_mass[k] += roles[k]
                        * if j == 0 {
                            l.manifest
                                .training
                                .as_ref()
                                .unwrap()
                                .config
                                .first_target_weight
                        } else {
                            1.
                        };
                }
            }
            if expected_mass
                .iter()
                .zip(r.mass)
                .any(|(a, b)| (a - b).abs() > 1e-9)
            {
                return Err(bad("probe role mass reconstruction"));
            }
            targets += u64::from(r.targets);
            correct += u64::from(r.correct);
            exact += u64::from(r.correct == r.targets);
            base += r.base * f64::from(r.targets);
            span += r.span * f64::from(r.targets);
            first[r.first_error_role.map_or(6, usize::from)] += 1;
            for i in 0..6 {
                mass[i] += r.mass[i];
                nll[i] += r.nll[i];
                weighted[i] += r.weighted_nll[i];
            }
            let v = tasks
                .entry(s.cases[*ordinal as usize].category)
                .or_default();
            v[0] += 1;
            v[1] += u64::from(r.correct);
            v[2] += u64::from(r.targets);
        }
        println!(
            "TRAIN_PROBE_RECOUNT arm={arm} label={label} step={} rows={} targets={targets} correct={correct} full_teacher={exact} BASE_OBJECTIVE={} SPAN_OBJECTIVE={} ROLE_ORDER={:?} mass={mass:?} unweighted_nll={nll:?} weighted_nll={weighted:?} first_error_or_none={first:?} category_cases_correct_targets={tasks:?} SOURCE=DERIVED_FROM_EXISTING_TRAIN_PROBES NEW_TEACHERS=0 INDEPENDENT_GENERALIZATION=false",
            native.step,
            p.rows.len(),
            base / targets as f64,
            span / targets as f64,
            target_loss::ROLE_NAMES
        );
        let mean = std::array::from_fn::<_, 6, _>(|i| {
            if token_mass[i] > 0. {
                Some(nll[i] / token_mass[i])
            } else {
                None
            }
        });
        let weighted_mean = std::array::from_fn::<_, 6, _>(|i| {
            if mass[i] > 0. {
                Some(weighted[i] / mass[i])
            } else {
                None
            }
        });
        println!(
            "TRAIN_ROLE_MEANS arm={arm} label={label} fractional_token_counts={token_mass:?} mean_nll_per_token={mean:?} weighted_mean_per_weight_mass={weighted_mean:?} NEW_FORWARDS=0"
        );
    }
    Ok(())
}
#[allow(clippy::too_many_arguments)] // Existing owned run/native/samples and one bounded train-only observation.
fn train_probe(
    root: &Path,
    label: &str,
    s: &RunSnapshot,
    l: &Loaded,
    native: &CheckpointRef,
    framed: &[Sample],
    annotations: &[target_loss::Annotation],
    control: &mut RunControl,
) -> Result<()> {
    let policy = s
        .objective
        .as_ref()
        .ok_or_else(|| bad("train probe objective policy"))?;
    let final_name = format!("train-{label}-final.r3er");
    let start_name = format!("train-{label}-start.r3er");
    if root.join(&start_name).exists() {
        let Record::TrainProbe(p) = read_record(root, &reference(root, &final_name)?)? else {
            return Err(bad("train probe missing complete final; retry prohibited"));
        };
        if !p.complete
            || p.binding != s.binding()
            || p.native.file != native.file
            || p.source != s.source
            || p.rows.iter().map(|(i, _)| *i).collect::<Vec<_>>() != policy.probes
        {
            return Err(bad("train probe incomplete/changed; retry prohibited"));
        }
        return Ok(());
    }
    let mut p = TrainProbe {
        binding: s.binding(),
        source: s.source,
        native: native.clone(),
        complete: false,
        rows: vec![],
    };
    publish(root, &start_name, &Record::TrainProbe(p.clone()))?;
    for (index, ordinal) in policy.probes.iter().enumerate() {
        control.check("train_probe_before_teacher")?;
        if control.teacher_calls >= control.teacher_limit {
            return Err(bad("teacher call cap"));
        }
        let position = s
            .train
            .iter()
            .position(|i| i == ordinal)
            .ok_or_else(|| bad("probe outside train"))?;
        control.teacher_calls += 1;
        println!(
            "TRAIN_PROBE_API_ENTRY={} label={label} ordinal={ordinal} step={} TRAIN_ONLY=true",
            control.teacher_calls, native.step
        );
        let stats = target_loss::probe(
            &l.model,
            &framed[position],
            &annotations[position],
            l.manifest
                .training
                .as_ref()
                .unwrap()
                .config
                .first_target_weight,
        )?;
        println!(
            "TRAIN_PROBE label={label} ordinal={ordinal} targets={} correct={} first_error_role={:?} base={} span={} ROLE_ORDER={:?} mass={:?} nll={:?} weighted_nll={:?}",
            stats.targets,
            stats.correct,
            stats.first_error_role,
            stats.base,
            stats.span,
            target_loss::ROLE_NAMES,
            stats.mass,
            stats.nll,
            stats.weighted_nll
        );
        p.rows.push((*ordinal, stats));
        // Returned prefixes remain available even when the next control check cancels.
        publish(
            root,
            &format!("train-{label}-row-{index:02}.r3er"),
            &Record::TrainProbe(p.clone()),
        )?;
        control.check("train_probe_after_teacher")?;
    }
    p.complete = true;
    publish(root, &final_name, &Record::TrainProbe(p))?;
    Ok(())
}
fn run_native(root: &Path, resume: Option<&str>, control: &mut RunControl) -> Result<()> {
    let s = read_inputs(root)?;
    if s.screen() && s.tiny_spec {
        let (registered, _, _) = screen_parent(&s)?;
        if root.canonicalize()? != registered {
            return Err(bad("TINY screen registered root"));
        }
        let proof = origin_path(&s, "bridge-observation-final")?;
        let observation = proof.parent().unwrap();
        let prior = read_inputs(observation)?;
        bridge_origins(&prior, Some(observation))?;
        read_preflight_outcome(observation)?
            .ok_or_else(|| bad("TINY screen original proof"))?
            .require_current_success()?;
    }
    // Pure, discarded capacity check using the actual terminal encoder; no future receipt.
    Record::Segment(SegmentReceipt::capacity_value(
        s.tape.clone(),
        s.continuation() || s.path_parity(),
        s.objective.is_some(),
    ))
    .encode()?;
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
    if s.historical || (!s.tiny_spec && s.authorization.is_none() && !s.path_parity()) {
        return Err(bad(
            "this repair authorizes no SMALL optimizer updates; historical import cannot resume",
        ));
    }
    if s.path_parity() {
        if s.source != evaluator_source()
            || s.tape.len() != 2
            || !s.preflight()
            || s.lr_policy != 2
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
            return Err(bad("path parity fixed source/policy/budget"));
        }
        native_source(root, &s)?;
    }
    if s.authorization.is_some() {
        if !matches!(
            s.contract.as_str(),
            RESTART_CONTRACT
                | COOLDOWN_CONTRACT
                | OBJECTIVE_CONTRACT
                | NATIVE_CORPUS_CONTRACT
                | BRIDGE_CONTRACT
                | BOUNDED_BRIDGE_CONTRACT
        ) {
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
        if s.objective.is_some() || s.bridge() {
            let parent = root.parent().ok_or_else(|| bad("objective pair root"))?;
            let mut teachers = if s.screen() {
                60
            } else if s.bridge() {
                bridge_probe_indices(&s)?.len()
            } else {
                0
            };
            for arm in s.study_arms() {
                let dir = parent.join(arm);
                let other = read_inputs(&dir)?;
                teachers += arm_commands(&dir, &other)?
                    .iter()
                    .map(|(t, _)| t.teachers as usize)
                    .sum::<usize>();
            }
            control.teacher_limit = (if s.bridge() { 192usize } else { 256usize })
                .checked_sub(teachers)
                .ok_or_else(|| bad("pair teacher budget"))?;
        }
        if s.continuation() {
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
            for role in if s.bridge() {
                &[][..]
            } else {
                &[
                    "cooldown-parent-inputs",
                    "cooldown-parent-terminal",
                    "cooldown-parent-command",
                    "cooldown-legacy-proof",
                ][..]
            } {
                origin_path(&s, role)?;
            }
        }
        control.deadline = control.start + Duration::from_secs_f64((7200. - seconds).min(1800.));
        control.generation_limit = if s.screen() {
            4096
        } else if s.bridge() {
            4608
        } else if s.continuation() {
            4096
        } else {
            7500
        } - generations;
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
        if !s.preflight() && !s.continuation() {
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
    let load_started = Instant::now();
    let mut l = resolve_native(root, &s, &initial, true)?;
    if s.objective.is_some() {
        println!(
            "OBJECTIVE_IO operation=load_hash_validate step={} seconds={}",
            initial.step,
            load_started.elapsed().as_secs_f64()
        );
    }
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
        let expected = objective_binding(&s, &state, &l.tokenizer)?;
        if resume.is_some() && state.resume_binding.as_ref() != Some(&expected) {
            return Err(bad(
                "OBJECTIVE_POLICY_BINDING_MISMATCH: native resume; optimizer_calls=0",
            ));
        }
        if resume.is_none() && state.resume_binding.is_none() && !s.tiny_spec {
            return Err(bad(
                "LEGACY_OBJECTIVE_UNKNOWN: migrate parent explicitly; optimizer_calls=0",
            ));
        }
        state.resume_binding = Some(expected);
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
    let mut lr_bits = (s.continuation() || s.path_parity()).then(Vec::new);
    let mut objective_metrics = s.objective.as_ref().map(|_| Vec::new());
    let mut complete = false;
    let mut heartbeat = Instant::now();
    let mut last_saved = if resume.is_some() {
        Some(initial.clone())
    } else {
        None
    };
    let outcome = (|| -> Result<()> {
        let mut episodes: Vec<_> = s
            .train
            .iter()
            .map(|i| s.cases[*i as usize].clone())
            .collect();
        if s.path_parity() {
            let source = native_source(root, &s)?;
            episodes = source.train;
            if let Some(legacy) = s
                .origins
                .iter()
                .find(|o| o.role == "explicit-legacy-reference")
            {
                let (_, train, dev) = data::load_legacy(Path::new(&legacy.original.locator))?;
                if data::native::ordered_bytes(&train) != data::native::ordered_bytes(&episodes)
                    || data::native::ordered_bytes(&dev)
                        != data::native::ordered_bytes(&source.validation)
                {
                    return Err(bad("legacy reference changed"));
                }
                episodes = train;
                println!("PATH=EXPLICIT_LEGACY_REFERENCE JSON_READS=3");
            } else {
                println!("PATH=NATIVE_CORPUS JSON_READS=0");
            }
        }
        let preparation_started = Instant::now();
        let framed = if s.path_parity()
            && !s
                .origins
                .iter()
                .any(|o| o.role == "explicit-legacy-reference")
        {
            let framed = token_cache::load_samples(root, &s, &l, &root.join("cache"))?;
            println!("INPUT_CACHE=R3TOK VERIFIED=true TOKENIZATION_CALLS=0");
            framed
        } else {
            samples(&episodes, &l.tokenizer, state.config.seq_len)?
        };
        let annotations = s
            .objective
            .as_ref()
            .map(|policy| {
                let focus = s
                    .authorization
                    .as_ref()
                    .map_or(&[][..], |a| a.pools[1].as_slice());
                let values = target_loss::annotations(&episodes, &framed, &l.tokenizer, focus)?;
                if hash(&target_loss::annotation_bytes(&values)) != policy.annotation {
                    return Err(bad("bound objective annotation changed"));
                }
                Ok(values)
            })
            .transpose()?;
        if s.objective.is_some() {
            println!(
                "OBJECTIVE_IO operation=tokenize_annotate examples={} seconds={}",
                framed.len(),
                preparation_started.elapsed().as_secs_f64()
            );
        }
        if s.objective.as_ref().is_some_and(|o| !o.span) && state.step as u64 == s.parent.step {
            train_probe(
                root,
                "parent",
                &s,
                &l,
                &s.parent,
                &framed,
                annotations.as_ref().unwrap(),
                control,
            )?;
        }
        loop {
            let n = state.step as u64 - s.parent.step;
            if (s.bridge() || matches!(s.purpose, RunPurpose::SaveSplit | RunPurpose::LrSplit))
                && !(s.tiny_spec && s.origins.iter().any(|o| o.role == "bridge-continuous-test"))
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
                let mid = s.cooldown() && n == 128
                    || (s.objective.is_some() || s.bridge())
                        && n == if s.tiny_spec { 1 } else { 256 };
                let screening = s.screen() && n < s.tape.len() as u64;
                let kinds: &[PanelKind] = if screening {
                    &[PanelKind::Screen]
                } else if mid {
                    &[PanelKind::Dev, PanelKind::Watch]
                } else if s.bridge() {
                    &[
                        PanelKind::Dev,
                        PanelKind::Cross,
                        PanelKind::Ordinary,
                        PanelKind::Watch,
                        PanelKind::LegacyDev,
                        PanelKind::Conditional,
                    ]
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
                        let evaluation_started = Instant::now();
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
                        let e = if kind == PanelKind::Watch && s.authorization.is_some() && !mid {
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
                                s.tiny_spec && !s.screen(),
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
                        if s.objective.is_some() {
                            println!(
                                "OBJECTIVE_IO operation=evaluate_publish panel={} step={} seconds={}",
                                kind.name(),
                                state.step,
                                evaluation_started.elapsed().as_secs_f64()
                            );
                        }
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
                    let (_, dev, dh) = get(if screening {
                        PanelKind::Screen
                    } else {
                        PanelKind::Dev
                    })?;
                    let (_, watch, wh) = get(if screening {
                        PanelKind::Screen
                    } else {
                        PanelKind::Watch
                    })?;
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
                if s.screen() && h.quality {
                    // The policy ended this research. Preserve a non-resumable quality
                    // terminal; do not turn it into a normal completion/candidate.
                    break;
                }
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
                if s.objective.is_some() {
                    let native = last_saved
                        .as_ref()
                        .ok_or_else(|| bad("train probe final saved endpoint absent"))?;
                    train_probe(
                        root,
                        "final",
                        &s,
                        &l,
                        native,
                        &framed,
                        annotations.as_ref().unwrap(),
                        control,
                    )?;
                }
                if s.bridge() {
                    collect_bridge_teacher(
                        root,
                        "final-probe",
                        &s,
                        last_saved
                            .as_ref()
                            .ok_or_else(|| bad("bridge final saved endpoint"))?,
                        &l,
                        control,
                    )?;
                }
                complete = true;
                break;
            }
            control.check("binary_before_forward")?;
            let d = &s.tape[n as usize];
            let prepare_time = Instant::now();
            let indices: Vec<_> = d.indices.iter().map(|i| *i as usize).collect();
            let b = batch(&framed, &indices, &Device::Cpu)?;
            let targets: usize = indices
                .iter()
                .map(|i| framed[*i].tokens.len() - framed[*i].response_start)
                .sum();
            if b.tokens as u64 != d.input || targets as u64 != d.target {
                return Err(bad("frozen actual batch denominator"));
            }
            let prepare_s = prepare_time.elapsed().as_secs_f64();
            let forward_time = Instant::now();
            let logits = l.model.forward(&b.input, Some(&b.valid))?;
            let (ce, obj, actual) = if s.objective.as_ref().is_some_and(|o| o.span) {
                target_loss::loss(
                    &logits,
                    &b,
                    &framed,
                    &indices,
                    annotations.as_ref().unwrap(),
                    state.config.first_target_weight,
                )?
            } else {
                response_loss(&logits, &b, state.config.first_target_weight)?
            };
            let forward_s = forward_time.elapsed().as_secs_f64();
            control.check("binary_after_forward")?;
            let ce = ce.to_scalar::<f32>()?;
            let objective = obj.to_scalar::<f32>()?;
            if !ce.is_finite() || !objective.is_finite() || actual != targets {
                return Err(bad("nonfinite loss/denominator"));
            }
            let backward_time = Instant::now();
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
            let rate = if s.objective.is_some() || s.path_parity() || s.bridge() {
                1e-4
            } else if s.cooldown() {
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
            let backward_s = backward_time.elapsed().as_secs_f64();
            let optimizer_time = Instant::now();
            let (norm, delta) =
                adam.step_constant(&l.model.vars, &grads, &state.config, state.step + 1, rate)?;
            let optimizer_s = optimizer_time.elapsed().as_secs_f64();
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
            if s.objective.is_some() {
                objective_metrics.as_mut().unwrap().push([
                    prepare_s,
                    forward_s,
                    backward_s,
                    optimizer_s,
                    norm,
                    delta,
                    f64::from(objective),
                    f64::from(norm > state.config.clip),
                ]);
                println!(
                    "OBJECTIVE_STEP={} CE={ce} OBJECTIVE={objective} GRAD_NORM={norm} UPDATE_NORM={delta} CLIPPED={} PREPARE_S={prepare_s} FORWARD_LOSS_S={forward_s} BACKWARD_S={backward_s} OPTIMIZER_S={optimizer_s}",
                    n + 1,
                    norm > state.config.clip
                );
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
            if heartbeat.elapsed() >= Duration::from_secs(15) || (n + 1).is_multiple_of(32) {
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
    if !(s.supports_partial_resume() && control.observed == [StopReason::TimeBudget]) && evaluations.iter().any(|r| read_record(root,&r.payload).is_ok_and(|v| matches!(v,
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
        objective_metrics,
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
        "SEGMENT={index} NEW_TINY_UPDATES={} NEW_SMALL_UPDATES={} GENERATIONS={} TEACHERS={} STOP={:?} resume={} complete={} CANONICAL_JSON_WRITES=0 EXPLICIT_LEGACY_REFERENCE={}",
        if s.tiny_spec { t.updates - start } else { 0 },
        if s.tiny_spec { 0 } else { t.updates - start },
        t.generations,
        t.teachers,
        t.stop,
        t.resume,
        t.complete,
        s.origins
            .iter()
            .any(|o| o.role == "explicit-legacy-reference")
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
        objective: None,
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
        objective_metrics: None,
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
        objective_metrics: None,
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
    let mut state = TrainingState {
        resume_binding: None,
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
    state.resume_binding = Some(checkpoint::ResumeBinding::default_for(&state, &l.tokenizer));
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
        objective: None,
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
        if s.cooldown() || s.bridge() {
            let l = resolve_native(root, &s, n, true)?;
            let rates = chain
                .iter()
                .flat_map(|(_, t)| t.lr_bits.clone().unwrap_or_default())
                .collect::<Vec<_>>();
            if rates.len() != 2 {
                return Err(bad("TINY numeric actual LR count"));
            }
            let mut state = l.manifest.training.clone().unwrap();
            if state.resume_binding.as_ref() != Some(&objective_binding(&s, &state, &l.tokenizer)?)
            {
                return Err(bad("fixture objective policy mismatch"));
            }
            state.resume_binding = None;
            let current = (Some(state), rates);
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
    #[test]
    fn bounded_screen_policy_stops_and_extensions_are_fixed() {
        let base: [Score; 4] = std::array::from_fn(|i| Score {
            exact: if i == 3 { 0 } else { 32 },
            planned: if i == 2 { 32 } else { 64 },
            ..Default::default()
        });
        let mut now = base.clone();
        now[0].exact -= 12;
        assert_eq!(
            screen_policy(&base, &now, [0; 3], 32).1,
            Some("QUALITY_REGRESSION_STOP")
        );
        now = base.clone();
        now[1].exact -= 8;
        let (streak, stop) = screen_policy(&base, &now, [0; 3], 32);
        assert!(stop.is_none());
        assert_eq!(
            screen_policy(&base, &now, streak, 64).1,
            Some("QUALITY_REGRESSION_STOP")
        );
        now = base.clone();
        now[2].exact -= 4;
        let (streak, stop) = screen_policy(&base, &now, [0; 3], 32);
        assert!(stop.is_none());
        assert!(screen_policy(&base, &now, streak, 64).1.is_some());
        now = base.clone();
        now[3].exact = 8;
        now[3].base = [2, 16];
        now[0].exact -= 4;
        now[1].exact -= 4;
        now[2].exact -= 2;
        for step in [128, 256] {
            assert!(screen_policy(&base, &now, [0; 3], step).1.is_none());
        }
        for field in 0..4 {
            let mut failed = now.clone();
            if field == 3 {
                failed[3].exact -= 1;
            } else {
                failed[field].exact -= 1;
            }
            assert_eq!(
                screen_policy(&base, &failed, [0; 3], 128).1,
                Some("NO_SUFFICIENT_TRANSFER_SIGNAL_WITHIN_BUDGET")
            );
        }
        now[3].base[0] = 1;
        assert!(screen_policy(&base, &now, [0; 3], 128).1.is_some());
        now = base.clone();
        now[3].errors = 4;
        assert!(screen_policy(&base, &now, [0; 3], 32).1.is_none());
        now[3].errors = 5;
        assert_eq!(
            screen_policy(&base, &now, [0; 3], 32).1,
            Some("GENERATION_ERROR_REGRESSION_STOP")
        );
        assert_eq!(
            screen_policy(&base, &base, [0; 3], 128).1,
            Some("NO_SUFFICIENT_TRANSFER_SIGNAL_WITHIN_BUDGET")
        );
    }
    #[test]
    fn segment_capacity_record_publisher_roundtrip() {
        let dir = tempfile::tempdir().unwrap();
        for metrics in [false, true] {
            for count in [0, 1, 2, 128, 255, 256, 257, 511, 512] {
                let draws = vec![
                    Draw {
                        indices: vec![0],
                        sampler: 1,
                        input: 2,
                        target: 1
                    };
                    count
                ];
                let record = Record::Segment(SegmentReceipt::capacity_value(draws, true, metrics));
                let file = publish(
                    dir.path(),
                    &format!("synthetic-{metrics}-{count}.r3er"),
                    &record,
                )
                .unwrap_or_else(|e| panic!("metrics={metrics} count={count}: {e}"));
                let bytes = std::fs::read(dir.path().join(&file.locator)).unwrap();
                let read = read_record(dir.path(), &file).unwrap();
                assert_eq!(bytes, read.encode().unwrap());
                for index in [6, 7] {
                    let mut invalid = bytes.clone();
                    invalid[index] = 255;
                    assert!(Record::decode(&invalid).is_err());
                }
                let mut trailing = bytes.clone();
                trailing.push(0);
                assert!(Record::decode(&trailing).is_err());
                assert!(
                    matches!(read, Record::Segment(t) if t.draws.len()==count && t.lr_bits.as_ref().unwrap().len()==count)
                );
            }
        }
        println!(
            "SYNTHETIC_TERMINAL_CAPACITY writer/publisher/reader counts=0,1,2,128,255,256,257,511,512 OPTIMIZER_CALLS=0"
        );
    }
    #[cfg(feature = "test-support")]
    #[test]
    fn artifact_input_conflict_and_registered_identity_fail_closed() {
        let d = tempfile::tempdir().unwrap();
        let root = d.path().join("observation");
        command(Action::FixtureBridge {
            from: PathBuf::from(std::env::var_os("R3ER_TEST_BOOTSTRAP").unwrap()),
            output: root.clone(),
            continuous: false,
            observation: true,
        })
        .unwrap();
        let s = read_inputs(&root).unwrap();
        bridge_origins(&s, Some(&root)).unwrap();
        let l = resolve_native(&root, &s, &s.parent, true).unwrap();
        let e = s.cases[s.train[0] as usize].clone();
        exact_training_inputs(&[e.clone(), e.clone()], &l).unwrap();
        let mut changed = e.clone();
        changed.answer.push(' ');
        assert!(
            exact_training_inputs(&[e, changed], &l)
                .unwrap_err()
                .to_string()
                .contains("DATA_CONFLICT")
        );
        for field in 0..3 {
            let mut changed = s.clone();
            match field {
                0 => changed.source[0] ^= 1,
                1 => {
                    changed
                        .origins
                        .iter_mut()
                        .find(|o| o.role == "bridge-replacement-registration")
                        .unwrap()
                        .original
                        .digest[0] ^= 1
                }
                _ => {
                    changed
                        .origins
                        .iter_mut()
                        .find(|o| o.role == "execution-binary")
                        .unwrap()
                        .original
                        .digest[0] ^= 1
                }
            }
            assert!(bridge_origins(&changed, Some(&root)).is_err());
        }
        println!(
            "STATIC_CONFLICT_AND_REGISTRY_IDENTITY OPTIMIZER_CALLS=0 GENERATIONS=0 FORWARDS=0"
        );
    }
    #[test]
    fn segment_capacity_rejects_mismatch_nonfinite_and_forged_bounds() {
        let draw = Draw {
            indices: vec![0],
            sampler: 1,
            input: 2,
            target: 1,
        };
        for metrics in [false, true] {
            for (draws, rates) in [(512, 511), (511, 512), (513, 513)] {
                let mut t =
                    SegmentReceipt::capacity_value(vec![draw.clone(); draws], true, metrics);
                t.lr_bits = Some(vec![1e-4f64.to_bits(); rates]);
                assert!(
                    Record::Segment(t).encode().is_err(),
                    "draws={draws} rates={rates}"
                );
            }
            for rate in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY, -1., 0.] {
                let mut t = SegmentReceipt::capacity_value(vec![draw.clone()], true, metrics);
                t.lr_bits = Some(vec![rate.to_bits()]);
                assert!(Record::Segment(t).encode().is_err());
            }
            let mut t = SegmentReceipt::capacity_value(vec![draw.clone(); 2], true, metrics);
            if metrics {
                t.objective_metrics.as_mut().unwrap().pop();
                assert!(Record::Segment(t.clone()).encode().is_err());
                t.objective_metrics = Some(vec![[0.; 8]; 2]);
            }
            let encoded = Record::Segment(t).encode().unwrap();
            for removed in [1, 8, 16] {
                assert!(Record::decode(&encoded[..encoded.len() - removed]).is_err());
                assert!(
                    SegmentReceipt::decode(
                        &mut Reader::new(&encoded[HEADER..encoded.len() - removed]),
                        true,
                        metrics
                    )
                    .is_err()
                );
            }
        }
        let mut empty = Vec::new();
        SegmentReceipt::capacity_value(vec![], true, false).encode(&mut empty);
        for forged in [513, u64::MAX] {
            let mut bytes = empty[..empty.len() - 2].to_vec();
            put_varint(&mut bytes, forged); // draws count, no attacker-sized allocation
            bytes.push(0);
            assert!(SegmentReceipt::decode(&mut Reader::new(&bytes), true, false).is_err());
        }
        // Equal-sized positive rates are schema-valid but not an authorized LR policy.
        let from = std::env::var_os("R3ER_TEST_BOOTSTRAP")
            .map(PathBuf::from)
            .unwrap();
        let mut s = read_inputs(&from).unwrap();
        s.contract = BRIDGE_CONTRACT.into();
        let dir = tempfile::tempdir().unwrap();
        let mut t = SegmentReceipt::capacity_value(vec![draw; 2], true, false);
        t.run = s.run;
        t.binding = s.binding();
        t.save_error = None;
        t.lr_bits = Some(vec![2e-4f64.to_bits(); 2]);
        let r = publish(
            dir.path(),
            "synthetic-wrong-policy.r3er",
            &Record::Segment(t),
        )
        .unwrap();
        assert!(
            segment(dir.path(), &r, &s)
                .unwrap_err()
                .to_string()
                .contains("actual LR bits/horizon/cursor")
        );
        println!("NEGATIVE_CAPACITY_AND_POLICY OPTIMIZER_CALLS=0");
    }
    #[cfg(feature = "test-support")]
    #[test]
    fn bridge_teacher_raw_mutations_and_64_row_schema() {
        let d = tempfile::tempdir().unwrap();
        let root = d.path().join("observation");
        let from = PathBuf::from(std::env::var_os("R3ER_TEST_BOOTSTRAP").unwrap());
        command(Action::FixtureBridge {
            from,
            output: root.clone(),
            continuous: false,
            observation: true,
        })
        .unwrap();
        let mut control = RunControl::new(
            Arc::new(AtomicBool::new(false)),
            Duration::from_secs(120),
            u64::MAX,
        )
        .unwrap();
        bridge_observe(&root, &mut control).unwrap();
        assert_eq!((control.generation_calls, control.teacher_calls), (2, 2));
        let s = read_inputs(&root).unwrap();
        let original = read_verified_bridge_teacher(&root, "parent-probe", &s, &s.parent).unwrap();
        let row_path = root.join("parent-probe-row-00.r3er");
        let bytes = std::fs::read(&row_path).unwrap();
        for mutation in 0..19 {
            let Record::Evaluation(mut e) = Record::decode(&bytes).unwrap() else {
                unreachable!()
            };
            let row = &mut e.rows[0];
            let TeacherRecord::Measured(t) = &mut row.teacher else {
                unreachable!()
            };
            match mutation {
                0 => e.source[0] ^= 1,
                1 => e.model[0] ^= 1,
                2 => e.step += 1,
                3 => row.ordinal += 1,
                4 => row.case[0] ^= 1,
                5 => row.prompt[0] ^= 1,
                6 => row.native_prompt[0] ^= 1,
                7 => row.provided.push(999),
                8 => row.completed = false,
                9 => row.interruption = Some(StopReason::Cancelled),
                10 => t.correct = t.target + 1,
                11 => t.mean = Scalar::Nonfinite64(f64::NAN.to_bits()),
                12 => row.diagnostic = Some(Scalar::Nonfinite64(f64::INFINITY.to_bits())),
                13 => row.retained[1] = u32::MAX,
                14 => t.first_argmax = u32::MAX,
                15 => t.gold_probability = Scalar::F64(1.1),
                16 => t.field_accuracy[0].correct = t.target + 1,
                17 => e.expected -= 1,
                18 => row.excluded.push(999),
                _ => unreachable!(),
            }
            std::fs::write(&row_path, Record::Evaluation(e).encode().unwrap()).unwrap();
            assert!(
                read_verified_bridge_teacher(&root, "parent-probe", &s, &s.parent).is_err(),
                "mutation {mutation}"
            );
            assert!(
                read_preflight_outcome(&root).is_err(),
                "approval mutation {mutation}"
            );
            std::fs::write(&row_path, &bytes).unwrap();
        }
        // 64 synthetic copies of two actual TINY measurements: schema coverage,
        // never reported as 64 forwards or as model quality.
        let mut expanded = s.clone();
        let template = expanded.cases[original.rows[0].ordinal as usize].clone();
        expanded.cases = (0..64)
            .map(|i| {
                let mut e = template.clone();
                e.id = format!("schema-only/{i}");
                e
            })
            .collect();
        let indices = (0..64).collect::<Vec<u32>>();
        let mut all = bridge_payload(&expanded, &expanded.parent, PanelKind::Dev, vec![]);
        all.expected = 64;
        publish(&root, "schema-start.r3er", &Record::Evaluation(all.clone())).unwrap();
        for &i in &indices {
            let mut row = original.rows[0].clone();
            row.ordinal = i;
            row.case = case_hash(&expanded.cases[i as usize]);
            let mut one = all.clone();
            one.rows = vec![row.clone()];
            publish(
                &root,
                &format!("schema-row-{i:02}.r3er"),
                &Record::Evaluation(one.clone()),
            )
            .unwrap();
            one.rows[0].teacher = TeacherRecord::NotRequested;
            one.rows[0].completed = false;
            one.rows[0].diagnostic = None;
            one.rows[0].retained.clear();
            publish(
                &root,
                &format!("schema-entry-{i:02}.r3er"),
                &Record::Evaluation(one),
            )
            .unwrap();
            all.rows.push(row);
        }
        publish(&root, "schema-final.r3er", &Record::Evaluation(all.clone())).unwrap();
        assert_eq!(
            verify_bridge_teacher_rows(&root, "schema", &expanded, &expanded.parent, &indices)
                .unwrap()
                .rows
                .len(),
            64
        );
        all.rows.pop();
        std::fs::write(
            root.join("schema-final.r3er"),
            Record::Evaluation(all).encode().unwrap(),
        )
        .unwrap();
        assert!(
            verify_bridge_teacher_rows(&root, "schema", &expanded, &expanded.parent, &indices)
                .is_err()
        );
        let swapped = std::fs::read(root.join("schema-row-01.r3er")).unwrap();
        std::fs::write(root.join("schema-row-00.r3er"), swapped).unwrap();
        assert!(
            verify_bridge_teacher_rows(&root, "schema", &expanded, &expanded.parent, &indices)
                .is_err()
        );
        println!(
            "TEACHER_SCHEMA SYNTHETIC_ROWS=64 ACTUAL_TINY_GENERATIONS=2 ACTUAL_TINY_TEACHERS=2 TINY_UPDATES=0 SMALL_UPDATES=0"
        );
    }
    #[test]
    fn partial_prefix_rejects_content_identity_denominator_and_complete_duplicates() {
        let row = EvalRow {
            ordinal: 0,
            case: [1; 32],
            prompt: [2; 32],
            prompt_len: 1,
            native_prompt: [3; 32],
            provided: vec![],
            excluded: vec![],
            tokens: vec![EOS],
            eos: Some(0),
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
            diagnostic: None,
        };
        let old = EvalPayload {
            run: [1; 32],
            binding: [2; 32],
            source: [3; 32],
            model: [4; 32],
            tokenizer: [5; 32],
            architecture: [6; 32],
            step: 128,
            new_updates: 1,
            kind: PanelKind::Dev,
            expected: 3,
            rows: vec![row.clone()],
        };
        let mut next = old.clone();
        next.rows.push(row);
        assert!(preserves_partial_prefix(&old, &next));
        for field in 0..14 {
            let mut altered = next.clone();
            match field {
                0 => altered.rows[0].tokens.push(9),
                1 => altered.rows[0].ordinal += 1,
                2 => altered.model[0] ^= 1,
                3 => altered.tokenizer[0] ^= 1,
                4 => altered.binding[0] ^= 1,
                5 => altered.expected += 1,
                6 => altered.rows.clear(),
                7 => altered.run[0] ^= 1,
                8 => altered.source[0] ^= 1,
                9 => altered.architecture[0] ^= 1,
                10 => altered.step += 1,
                11 => altered.new_updates += 1,
                12 => altered.rows[0].diagnostic = Some(Scalar::F64(0.009906130842864513)),
                13 => altered.kind = PanelKind::Watch,
                _ => unreachable!(),
            }
            assert!(!preserves_partial_prefix(&old, &altered), "field {field}");
        }
        let mut complete = old.clone();
        complete.expected = 1;
        assert!(!preserves_partial_prefix(&complete, &complete));
        let mut empty = old.clone();
        empty.rows.clear();
        assert!(preserves_partial_prefix(&empty, &next));
        let mut interrupted = old.clone();
        interrupted.rows[0].completed = false;
        interrupted.rows[0].interruption = Some(StopReason::TimeBudget);
        assert!(preserves_partial_prefix(&interrupted, &next));
    }
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
                objective_metrics: None,
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
