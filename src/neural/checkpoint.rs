//! Strict tensor-only artifacts; manifest publication is the commit point.
use super::{
    ByteBpe,
    transformer::{Config, Transformer},
};
use crate::{Error, Result};
use candle_core::{Device, Tensor};
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    path::Path,
};
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct TrainConfig {
    pub lr: f64,
    pub beta1: f64,
    pub beta2: f64,
    pub eps: f64,
    pub weight_decay: f64,
    pub clip: f64,
    pub warmup: usize,
    pub max_steps: usize,
    pub max_tokens: u64,
    pub microbatch: usize,
    #[serde(default = "unit_group")]
    pub sample_group_size: usize,
    pub accumulation: usize,
    pub seq_len: usize,
    pub validate_every: usize,
    pub seed: u64,
    #[serde(default = "unit_weight")]
    pub first_target_weight: f64,
    #[serde(default)]
    pub curriculum_steps: usize,
    #[serde(default)]
    pub budget_start_step: usize,
    #[serde(default)]
    pub budget_start_tokens: u64,
}
fn unit_weight() -> f64 {
    1.
}
fn unit_group() -> usize {
    1
}
impl Default for TrainConfig {
    fn default() -> Self {
        Self {
            lr: 0.001,
            beta1: 0.9,
            beta2: 0.999,
            eps: 1e-8,
            weight_decay: 0.01,
            clip: 1.,
            warmup: 100,
            max_steps: 5000,
            max_tokens: 20_000_000,
            microbatch: 1,
            sample_group_size: 1,
            accumulation: 4,
            seq_len: 512,
            validate_every: 100,
            seed: 17,
            first_target_weight: 1.,
            curriculum_steps: 0,
            budget_start_step: 0,
            budget_start_tokens: 0,
        }
    }
}
impl TrainConfig {
    pub fn validate(&self, context: usize) -> Result<()> {
        if !self.lr.is_finite()
            || self.lr <= 0.
            || self.lr > 0.1
            || !(0.0..1.0).contains(&self.beta1)
            || !(0.0..1.0).contains(&self.beta2)
            || !self.eps.is_finite()
            || self.eps <= 0.
            || !self.weight_decay.is_finite()
            || !(0.0..=1.0).contains(&self.weight_decay)
            || !self.clip.is_finite()
            || self.clip <= 0.
            || self.max_steps > i32::MAX as usize
            || self
                .max_steps
                .checked_sub(self.budget_start_step)
                .is_none_or(|n| n == 0 || n > 5000 || self.warmup > n)
            || self
                .max_tokens
                .checked_sub(self.budget_start_tokens)
                .is_none_or(|n| n == 0 || n > 20_000_000)
            || self.curriculum_steps > self.max_steps
            || self.microbatch == 0
            || self.microbatch > 8
            || self.sample_group_size == 0
            || self.sample_group_size > self.microbatch
            || !self.microbatch.is_multiple_of(self.sample_group_size)
            || self.accumulation == 0
            || self.accumulation > 32
            || self.seq_len < 2
            || self.seq_len > context
            || self.validate_every == 0
            || !self.first_target_weight.is_finite()
            || !(1.0..=16.0).contains(&self.first_target_weight)
        {
            return Err(Error::Invalid("training configuration budget".into()));
        }
        Ok(())
    }
    pub fn learning_rate(&self, step: usize) -> f64 {
        let step = step.saturating_sub(self.budget_start_step);
        if self.warmup > 0 && step <= self.warmup {
            self.lr * step as f64 / self.warmup as f64
        } else {
            let progress = (step.saturating_sub(self.warmup) as f64
                / (self.max_steps - self.budget_start_step - self.warmup).max(1) as f64)
                .min(1.);
            self.lr * (0.1 + 0.9 * 0.5 * (1. + (std::f64::consts::PI * progress).cos()))
        }
    }
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct TrainingState {
    /// None means legacy/unknown, never the default objective.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub resume_binding: Option<ResumeBinding>,
    /// Full passes over base sixteen cases, optionally both evidence orders (32).
    /// Historical field name is retained; never the ordinary with-replacement sampler.
    #[serde(default)]
    pub contrast16: bool,
    #[serde(default)]
    pub parent_checkpoint_hash: Option<String>,
    pub config: TrainConfig,
    pub step: usize,
    pub consumed_tokens: u64,
    pub target_tokens: u64,
    pub sampler_state: u64,
    pub corpus_hash: String,
    pub validation_hash: String,
    #[serde(default)]
    pub previous_corpora: Vec<String>,
    pub initial_weight_hash: String,
    pub train_loss: Option<f64>,
    pub validation_loss: Option<f64>,
}

/// Explicit per-answer CE meaning; no new tensor or descriptor layout.
pub const ANSWER_MEAN_FAMILY: u8 = 6;
pub const ANSWER_MEAN_OBJECTIVE: &str = "response_ce_answer_mean_v1";
/// Execution identity, independent of tensor/model identity and filesystem location.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ResumeBinding {
    pub version: u8,
    /// 1: response CE, 2: normalized answer-byte span CE,
    /// 3: response CE + 0.1 * mean paired first-divergence softplus (margin1).
    /// 4: response CE + 0.1 * mean of per-side first-divergence softplus (margin1).
    /// 5: family4 + 0.1 * mean first-query last-layer selected-record attention NLL.
    /// 6: response_ce_answer_mean_v1, per-answer token mean then example mean.
    pub family: u8,
    pub first_target_weight_bits: u64,
    /// Historical slot: span alpha for2, fixed auxiliary coefficient0.1 for3/4/5.
    pub span_alpha_bits: Option<u64>,
    /// 1: target denominator; 2: example mass; 3: pair mean; 4: side mean; 5: side + record means.
    pub normalizer: u8,
    pub annotation: Option<[u8; 32]>,
    pub train_order: [u8; 32],
    pub tokenizer: [u8; 32],
    pub framing: [u8; 32],
    pub config: [u8; 32],
    pub corpus: [u8; 32],
    pub validation: [u8; 32],
    /// 0: generic config schedule; 1: bound native execution policy.
    pub execution: u8,
    pub policy: [u8; 32],
    /// Original policy digest is retained across explicit format migration.
    pub provenance: [u8; 32],
}
impl ResumeBinding {
    pub fn digest_bytes(bytes: &[u8]) -> [u8; 32] {
        use sha2::{Digest, Sha256};
        Sha256::digest(bytes).into()
    }
    pub fn encode(&self, b: &mut Vec<u8>) {
        b.extend([self.version, self.family, self.normalizer, self.execution]);
        b.extend(self.first_target_weight_bits.to_le_bytes());
        b.push(u8::from(self.span_alpha_bits.is_some()));
        if let Some(alpha) = self.span_alpha_bits {
            b.extend(alpha.to_le_bytes());
        }
        b.push(u8::from(self.annotation.is_some()));
        if let Some(annotation) = self.annotation {
            b.extend(annotation);
        }
        for h in [
            self.train_order,
            self.tokenizer,
            self.framing,
            self.config,
            self.corpus,
            self.validation,
            self.policy,
            self.provenance,
        ] {
            b.extend(h);
        }
    }
    pub fn digest(&self) -> [u8; 32] {
        let mut b = b"R3-OBJECTIVE-RESUME-v1\0".to_vec();
        self.encode(&mut b);
        Self::digest_bytes(&b)
    }
    pub fn validate(&self, state: &TrainingState, tok: &ByteBpe) -> Result<()> {
        if self.version != 1
            || ![1, 2, 3, 4, 5, ANSWER_MEAN_FAMILY].contains(&self.family)
            || self.execution > 1
            || self.first_target_weight_bits != state.config.first_target_weight.to_bits()
            || self.tokenizer != Self::digest_bytes(tok.semantic_id().as_bytes())
            || super::Framing::from_digest(self.framing).is_err()
            || self.config != Self::config_digest(&state.config)
            || self.corpus != Self::digest_bytes(state.corpus_hash.as_bytes())
            || self.validation != Self::digest_bytes(state.validation_hash.as_bytes())
            || (self.family == 1
                && (self.normalizer != 1
                    || self.span_alpha_bits.is_some()
                    || self.annotation.is_some()))
            || (self.family == 2
                && (self.normalizer != 2
                    || self.span_alpha_bits != Some(1f64.to_bits())
                    || self.annotation.is_none()
                    || self.execution != 1))
            || (self.family == ANSWER_MEAN_FAMILY
                && (self.normalizer != 2
                    || self.first_target_weight_bits != 1f64.to_bits()
                    || self.span_alpha_bits.is_some()
                    || self.annotation.is_some()
                    || self.execution != 1))
            || ([3,4,5].contains(&self.family)
                && (self.normalizer != self.family
                    || self.first_target_weight_bits != 1f64.to_bits()
                    || self.span_alpha_bits != Some(0.1f64.to_bits())
                    || self.annotation.is_none()
                    || self.execution != 1))
        {
            return Err(Error::Corrupt("OBJECTIVE_POLICY_BINDING_MISMATCH".into()));
        }
        Ok(())
    }
    pub fn config_digest(c: &TrainConfig) -> [u8; 32] {
        let mut b = b"R3-TRAIN-CONFIG-v1\0".to_vec();
        for f in [
            c.lr,
            c.beta1,
            c.beta2,
            c.eps,
            c.weight_decay,
            c.clip,
            c.first_target_weight,
        ] {
            b.extend(f.to_bits().to_le_bytes());
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
            b.extend((n as u64).to_le_bytes());
        }
        for n in [c.max_tokens, c.seed, c.budget_start_tokens] {
            b.extend(n.to_le_bytes());
        }
        Self::digest_bytes(&b)
    }
    pub fn default_for(s: &TrainingState, tok: &ByteBpe) -> Self {
        let config = Self::config_digest(&s.config);
        Self {
            version: 1,
            family: 1,
            first_target_weight_bits: s.config.first_target_weight.to_bits(),
            span_alpha_bits: None,
            normalizer: 1,
            annotation: None,
            train_order: Self::digest_bytes(s.corpus_hash.as_bytes()),
            tokenizer: Self::digest_bytes(tok.semantic_id().as_bytes()),
            framing: Self::digest_bytes(super::PROMPT_FORMAT.as_bytes()),
            config,
            corpus: Self::digest_bytes(s.corpus_hash.as_bytes()),
            validation: Self::digest_bytes(s.validation_hash.as_bytes()),
            execution: 0,
            policy: config,
            provenance: config,
        }
    }
    pub fn require_default(s: &TrainingState, tok: &ByteBpe) -> Result<()> {
        let actual = s.resume_binding.as_ref().ok_or_else(|| Error::Invalid("LEGACY_OBJECTIVE_UNKNOWN: explicit provenance-backed migration required; optimizer_calls=0".into()))?;
        actual.validate(s, tok)?;
        if actual != &Self::default_for(s, tok) {
            return Err(Error::Invalid("OBJECTIVE_POLICY_UNSUPPORTED: use the exact bound native policy; optimizer_calls=0".into()));
        }
        Ok(())
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LegacyIdentity {
    pub config_json_sha256: String,
    pub tokenizer_json_sha256: String,
    pub tensor_file_sha256: String,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Manifest {
    /// Explicit research optimizer; absent retains the historical Adam schema.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub optimizer_protocol: Option<OptimizerProtocol>,
    pub version: u32,
    pub architecture: Config,
    pub tokenizer_sha256: String,
    pub weights_sha256: String,
    pub weights_bytes: usize,
    pub tensors: BTreeMap<String, Vec<usize>>,
    pub dtype: String,
    pub source_id: String,
    pub init_seed: u64,
    pub initial_weight_hash: String,
    pub status: String,
    pub training: Option<TrainingState>,
    #[serde(default)]
    pub trained_steps: usize,
    #[serde(default)]
    pub diagnostic_only: bool,
    #[serde(default)]
    pub legacy_identity: Option<LegacyIdentity>,
    #[serde(default)]
    pub model_content_digest: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct OptimizerProtocol {
    pub version: u8,
    pub family: String,
    pub roles: BTreeMap<String, bool>, // true: hidden Muon matrix, false: AdamW
    pub role_digest: String,
    pub momentum_bits: u64,
    pub nesterov: bool,
    pub ns_bits: [u64; 3],
    pub ns_steps: usize,
    pub norm_epsilon_bits: u64,
    pub scale_bits: u64,
    pub dtype: String,
    pub runtime_digest: String,
    pub parent_step: usize,
    pub local_step: usize,
}
impl OptimizerProtocol {
    pub fn roles(c: &Config, muon: bool) -> BTreeMap<String, bool> {
        let hidden = (0..c.layers).flat_map(|i| ["q","k","v","o","gate","up","down"].map(move |n| format!("layer.{i}.{n}"))).collect::<std::collections::BTreeSet<_>>();
        c.shapes().keys().map(|n| (n.clone(), muon && hidden.contains(n))).collect()
    }
    pub fn new(c:&Config, muon:bool, runtime_digest:String, parent_step:usize) -> Result<Self> {
        let roles=Self::roles(c,muon);
        Ok(Self {version:1,family:if muon{"MUON-F32-SUM-NESTEROV-NS5-MATCHRMS-V1"}else{"ADAMW-FRESH-V1"}.into(),
            role_digest:super::hash(&crate::binary::to_vec(&roles)?),roles,momentum_bits:0.95f64.to_bits(),nesterov:true,
            ns_bits:[3.4445f64.to_bits(),(-4.7750f64).to_bits(),2.0315f64.to_bits()],ns_steps:5,norm_epsilon_bits:1e-7f64.to_bits(),
            scale_bits:0.2f64.to_bits(),dtype:"F32".into(),runtime_digest,parent_step,local_step:0})
    }
    pub fn validate(&self,c:&Config,s:&TrainingState)->Result<()> {
        let muon=match self.family.as_str(){"ADAMW-FRESH-V1"=>false,"MUON-F32-SUM-NESTEROV-NS5-MATCHRMS-V1"=>true,_=>return Err(Error::Corrupt("unsupported optimizer family".into()))};
        let mut expected=Self::new(c,muon,self.runtime_digest.clone(),self.parent_step)?;expected.local_step=self.local_step;
        if *self!=expected || self.runtime_digest.len()!=64 || !self.runtime_digest.bytes().all(|v|v.is_ascii_hexdigit())
            || self.parent_step.checked_add(self.local_step)!=Some(s.step) || s.config.budget_start_step!=self.parent_step
            || s.resume_binding.as_ref().is_none_or(|b|b.execution!=1||b.family!=ANSWER_MEAN_FAMILY) {
            return Err(Error::Corrupt("optimizer role/clock/runtime/objective mismatch".into()));
        } Ok(())
    }
}
pub struct Loaded {
    pub model: Transformer,
    pub tokenizer: ByteBpe,
    pub manifest: Manifest,
    pub optimizer: BTreeMap<String, Tensor>,
}
impl Manifest {
    pub fn framing(&self) -> Result<super::Framing> {
        self.training
            .as_ref()
            .and_then(|s| s.resume_binding.as_ref())
            .map_or(Ok(super::Framing::QuestionEvidence), |b| {
                super::Framing::from_digest(b.framing)
            })
    }
    pub fn require_default_framing(&self) -> Result<()> {
        if self.framing()? != super::Framing::QuestionEvidence {
            return Err(Error::Invalid(
                "EQ_FRAMING_REQUIRES_BOUND_RESEARCH_PATH".into(),
            ));
        }
        Ok(())
    }
}
pub(super) fn expected(m: &Manifest) -> BTreeMap<String, Vec<usize>> {
    let shapes = m.architecture.shapes();
    let mut result = BTreeMap::new();
    for (name, shape) in shapes {
        result.insert(format!("model.{name}"), shape.clone());
        if m.training.is_some() {
            if m.optimizer_protocol.as_ref().is_some_and(|p|p.roles.get(&name)==Some(&true)) {
                result.insert(format!("muon.m.{name}"),shape);
            } else {
                result.insert(format!("adam.m.{name}"), shape.clone());
                result.insert(format!("adam.v.{name}"), shape);
            }
        }
    }
    result
}
pub fn validate_metadata(m: &Manifest, tok: &ByteBpe) -> Result<()> {
    m.architecture.validate()?;
    if m.version != 1
        || m.dtype != "F32"
        || m.tokenizer_sha256 != tok.id()
        || m.architecture.vocab != tok.vocab_size()
        || m.tensors != expected(m)
        || m.source_id.len() != 64
        || !m.source_id.bytes().all(|b| b.is_ascii_hexdigit())
        || m.initial_weight_hash.len() != 64
        || ![
            "RANDOM_INITIALIZED",
            "TRAINING",
            "BUDGET_REACHED",
            "CANCELLED",
            "RESOURCE_LIMIT",
            "VERIFIED_CANDIDATE",
            "DIAGNOSTIC_COMPLETE",
            "BUDGET_EXHAUSTED",
        ]
        .contains(&m.status.as_str())
    {
        return Err(Error::Corrupt(
            "checkpoint version/shape/dtype/tokenizer/checksum/lineage".into(),
        ));
    }
    if let Some(s) = &m.training {
        if let Some(p)=&m.optimizer_protocol {p.validate(&m.architecture,s)?;}
        s.config.validate(m.architecture.context)?;
        if let Some(binding) = &s.resume_binding {
            binding.validate(s, tok)?;
        }
        if s.step > s.config.max_steps
            || s.parent_checkpoint_hash
                .as_ref()
                .is_some_and(|h| h.len() != 64 || !h.bytes().all(|b| b.is_ascii_hexdigit()))
            || (s.contrast16
                && (s.config.max_steps > 1000
                    || s.config.max_tokens > 3_200_000
                    || ![16, 32].contains(&(s.config.microbatch * s.config.accumulation))
                    || s.config.sample_group_size != 4
                    || s.config.budget_start_step != 0
                    || s.config.budget_start_tokens != 0
                    || s.config.first_target_weight != 1.))
            || s.step < s.config.budget_start_step
            || s.consumed_tokens < s.config.budget_start_tokens
            || s.consumed_tokens > s.config.max_tokens
            || s.target_tokens > s.consumed_tokens
            || (s.corpus_hash != tok.train_hash && !s.previous_corpora.contains(&tok.train_hash))
            || s.previous_corpora.len() > 32
            || s.previous_corpora
                .iter()
                .any(|h| h.len() != 64 || !h.bytes().all(|b| b.is_ascii_hexdigit()))
            || s.initial_weight_hash != m.initial_weight_hash
            || s.train_loss.is_some_and(|v| !v.is_finite())
            || s.validation_loss.is_some_and(|v| !v.is_finite())
        {
            return Err(Error::Corrupt("checkpoint training state".into()));
        }
    }
    if m.training.is_none() && m.optimizer_protocol.is_some() {return Err(Error::Corrupt("inference contains optimizer descriptor".into()));}
    Ok(())
}
// Retired input API: no text or safetensors fallback, including explicit imports.
pub fn legacy_metadata(_path: &Path) -> Result<(Manifest, ByteBpe)> {
    Err(Error::Unsupported("legacy model format retired; use native R3MODEL".into()))
}
pub fn legacy_load(_path: &Path, _device: Device, _resume: bool) -> Result<Loaded> {
    Err(Error::Unsupported("legacy model format retired; use native R3MODEL".into()))
}
pub fn initialized(
    model: &Transformer,
    tok: &ByteBpe,
    seed: u64,
    source_id: String,
) -> Result<Manifest> {
    Ok(Manifest {
        optimizer_protocol: None,
        version: 1,
        architecture: model.config.clone(),
        tokenizer_sha256: tok.id(),
        weights_sha256: String::new(),
        weights_bytes: 0,
        tensors: BTreeMap::new(),
        dtype: "F32".into(),
        source_id,
        init_seed: seed,
        initial_weight_hash: model.weight_hash()?,
        status: "RANDOM_INITIALIZED".into(),
        training: None,
        trained_steps: 0,
        diagnostic_only: false,
        legacy_identity: None,
        model_content_digest: model.weights_content_id()?,
    })
}
// Native is the only loader; retired text checkpoint inputs fail explicitly.
pub use super::artifact::{load, metadata, save};
