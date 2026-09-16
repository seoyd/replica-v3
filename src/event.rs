use crate::{Error, Result};
use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};

pub type EventId = i64;
pub const MAX_PAYLOAD: usize = 262_144;
pub const MAX_TEXT: usize = 1024;
pub const MAX_EVIDENCE: usize = 8;

pub fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
        .min(i64::MAX as u128) as i64
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GenerationLimits {
    pub max_tokens: u32,
    pub context_tokens: u32,
    pub timeout_ms: u64,
}
impl Default for GenerationLimits {
    fn default() -> Self {
        Self {
            max_tokens: 512,
            context_tokens: 8192,
            timeout_ms: 120_000,
        }
    }
}
impl GenerationLimits {
    pub fn validate(&self) -> Result<()> {
        if self.max_tokens == 0
            || self.max_tokens > 512
            || self.context_tokens > 8192
            || self.context_tokens <= self.max_tokens
            || self.timeout_ms == 0
            || self.timeout_ms > 120_000
        {
            return Err(Error::Invalid("generation limits".into()));
        }
        Ok(())
    }
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Slot {
    pub entity: String,
    pub predicate: String,
    pub context: String,
}
impl Slot {
    pub fn key(&self, scope: &str) -> Vec<u8> {
        let mut out = Vec::new();
        for value in [scope, &self.entity, &self.predicate, &self.context] {
            crate::codec::put_bytes(&mut out, value.as_bytes());
        }
        out
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u8)]
pub enum RelationKind {
    UsedEvidence,
    Precedes,
    Supports,
    Contradicts,
    CausalHypothesis,
    Supersedes,
    Restores,
}
impl RelationKind {
    pub fn from_tag(tag: u8) -> Result<Self> {
        match tag {
            0 => Ok(Self::UsedEvidence),
            1 => Ok(Self::Precedes),
            2 => Ok(Self::Supports),
            3 => Ok(Self::Contradicts),
            4 => Ok(Self::CausalHypothesis),
            5 => Ok(Self::Supersedes),
            6 => Ok(Self::Restores),
            _ => Err(Error::Corrupt("unknown relation tag".into())),
        }
    }
}
impl std::str::FromStr for RelationKind {
    type Err = Error;
    fn from_str(s: &str) -> Result<Self> {
        match s {
            "used_evidence" => Ok(Self::UsedEvidence),
            "precedes" => Ok(Self::Precedes),
            "supports" => Ok(Self::Supports),
            "contradicts" => Ok(Self::Contradicts),
            "causal_hypothesis" => Ok(Self::CausalHypothesis),
            "supersedes" => Ok(Self::Supersedes),
            "restores" => Ok(Self::Restores),
            _ => Err(Error::Invalid("unknown relation type".into())),
        }
    }
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GenerationInfo {
    pub model_id: String,
    pub model_revision: String,
    pub runtime_revision: String,
    pub quantization: String,
    pub license: String,
    pub finish_reason: String,
    pub input_tokens: Option<u64>,
    pub output_tokens: Option<u64>,
    pub limits: GenerationLimits,
    pub load_ms: u64,
    pub first_token_ms: Option<u64>,
    pub generation_ms: u64,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Kind {
    Observation {
        question: Option<GenerationLimits>,
    },
    Fact {
        slot: Slot,
        previous: Option<EventId>,
        restored_from: Option<EventId>,
        valid_from: Option<i64>,
        valid_until: Option<i64>,
    },
    Relation {
        relation: RelationKind,
        from: EventId,
        to: EventId,
        evidence: Vec<EventId>,
    },
    AssistantAnswer {
        input: EventId,
        evidence: Vec<EventId>,
        provided: Vec<EventId>,
        excluded: Vec<EventId>,
        generation: GenerationInfo,
        retrieval_truncated: bool,
    },
    Failure {
        input: EventId,
        code: String,
    },
}
impl Kind {
    pub fn tag(&self) -> u8 {
        match self {
            Self::Observation { .. } => 0,
            Self::Fact { .. } => 1,
            Self::Relation { .. } => 2,
            Self::AssistantAnswer { .. } => 3,
            Self::Failure { .. } => 4,
        }
    }
    pub fn input(&self) -> Option<EventId> {
        match self {
            Self::AssistantAnswer { input, .. } | Self::Failure { input, .. } => Some(*input),
            _ => None,
        }
    }
    pub fn slot(&self) -> Option<&Slot> {
        if let Self::Fact { slot, .. } = self {
            Some(slot)
        } else {
            None
        }
    }
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Event {
    pub id: EventId,
    pub recorded_at: i64,
    pub observed_at: Option<i64>,
    pub source: String,
    pub scope: String,
    pub session: String,
    pub request_key: Option<[u8; 16]>,
    pub payload: Vec<u8>,
    pub kind: Kind,
}
impl Event {
    pub fn observation(scope: &str, session: &str, source: &str, payload: Vec<u8>) -> Self {
        Self {
            id: 0,
            recorded_at: 0,
            observed_at: None,
            source: source.into(),
            scope: scope.into(),
            session: session.into(),
            request_key: None,
            payload,
            kind: Kind::Observation { question: None },
        }
    }
    pub fn validate(&self, assigned: bool) -> Result<()> {
        if assigned && self.id <= 0 {
            return Err(Error::Invalid("event ID must be positive".into()));
        }
        for s in [&self.source, &self.scope, &self.session] {
            check_text(s)?;
        }
        if self.payload.len() > MAX_PAYLOAD {
            return Err(Error::Invalid("payload too large".into()));
        }
        std::str::from_utf8(&self.payload)
            .map_err(|_| Error::Invalid("payload must be UTF-8".into()))?;
        match &self.kind {
            Kind::Observation {
                question: Some(limits),
            } => {
                limits.validate()?;
                if self.request_key.is_none() {
                    return Err(Error::Invalid("question requires request key".into()));
                }
            }
            Kind::Fact {
                slot,
                previous,
                restored_from,
                valid_from,
                valid_until,
            } => {
                for s in [&slot.entity, &slot.predicate, &slot.context] {
                    check_text(s)?;
                }
                if restored_from.is_some() && previous.is_none() {
                    return Err(Error::Invalid("restore without previous".into()));
                }
                if let Some(end) = valid_until
                    && valid_from.is_none_or(|start| start >= *end)
                {
                    return Err(Error::Invalid("invalid validity interval".into()));
                }
            }
            Kind::Relation { evidence, .. } => check_refs(evidence)?,
            Kind::AssistantAnswer {
                evidence,
                provided,
                excluded,
                generation,
                ..
            } => {
                for refs in [evidence, provided, excluded] {
                    check_refs(refs)?;
                }
                if evidence.iter().any(|id| !provided.contains(id))
                    || excluded.iter().any(|id| provided.contains(id))
                {
                    return Err(Error::Invalid("evidence not in provided bundle".into()));
                }
                for s in [
                    &generation.model_id,
                    &generation.model_revision,
                    &generation.runtime_revision,
                    &generation.quantization,
                    &generation.license,
                    &generation.finish_reason,
                ] {
                    check_text(s)?;
                }
                generation.limits.validate()?;
            }
            Kind::Failure { code, .. } => check_text(code)?,
            _ => {}
        }
        Ok(())
    }
    pub fn same_request(&self, other: &Self) -> bool {
        let mut a = self.clone();
        let mut b = other.clone();
        a.id = 0;
        b.id = 0;
        a.recorded_at = 0;
        b.recorded_at = 0;
        a == b
    }
}
pub fn check_text(s: &str) -> Result<()> {
    if s.is_empty() || s.len() > MAX_TEXT || s.contains('\0') {
        Err(Error::Invalid("empty/oversized/NUL identity".into()))
    } else {
        Ok(())
    }
}
pub fn check_refs(ids: &[EventId]) -> Result<()> {
    let unique: std::collections::BTreeSet<_> = ids.iter().collect();
    if ids.len() > MAX_EVIDENCE || unique.len() != ids.len() || ids.iter().any(|id| *id <= 0) {
        Err(Error::Invalid("invalid evidence IDs".into()))
    } else {
        Ok(())
    }
}
pub fn parse_key(s: &str) -> Result<[u8; 16]> {
    if s.len() != 32 || !s.is_ascii() {
        return Err(Error::Invalid("request key needs 32 hex digits".into()));
    }
    let mut out = [0; 16];
    for (i, b) in out.iter_mut().enumerate() {
        *b = u8::from_str_radix(&s[i * 2..i * 2 + 2], 16)
            .map_err(|_| Error::Invalid("request key hex".into()))?;
    }
    Ok(out)
}
