//! Strict tensor-only artifacts; manifest publication is the commit point.
use super::{
    ByteBpe, hash, read_bounded,
    transformer::{Config, Transformer},
    write_new,
};
use crate::{Error, Result};
use candle_core::{Device, Tensor};
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, HashMap},
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
    pub accumulation: usize,
    pub seq_len: usize,
    pub validate_every: usize,
    pub seed: u64,
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
            accumulation: 4,
            seq_len: 512,
            validate_every: 100,
            seed: 17,
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
            || self.max_steps == 0
            || self.max_steps > 5000
            || self.max_tokens == 0
            || self.max_tokens > 20_000_000
            || self.warmup > self.max_steps
            || self.microbatch == 0
            || self.microbatch > 8
            || self.accumulation == 0
            || self.accumulation > 32
            || self.seq_len < 2
            || self.seq_len > context
            || self.validate_every == 0
        {
            return Err(Error::Invalid("training configuration budget".into()));
        }
        Ok(())
    }
    pub fn learning_rate(&self, step: usize) -> f64 {
        if self.warmup > 0 && step <= self.warmup {
            self.lr * step as f64 / self.warmup as f64
        } else {
            let progress = (step.saturating_sub(self.warmup) as f64
                / (self.max_steps - self.warmup).max(1) as f64)
                .min(1.);
            self.lr * (0.1 + 0.9 * 0.5 * (1. + (std::f64::consts::PI * progress).cos()))
        }
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TrainingState {
    pub config: TrainConfig,
    pub step: usize,
    pub consumed_tokens: u64,
    pub target_tokens: u64,
    pub sampler_state: u64,
    pub corpus_hash: String,
    pub validation_hash: String,
    pub initial_weight_hash: String,
    pub train_loss: Option<f64>,
    pub validation_loss: Option<f64>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Manifest {
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
}
pub struct Loaded {
    pub model: Transformer,
    pub tokenizer: ByteBpe,
    pub manifest: Manifest,
    pub optimizer: BTreeMap<String, Tensor>,
}
fn expected(m: &Manifest) -> BTreeMap<String, Vec<usize>> {
    let shapes = m.architecture.shapes();
    let mut result = BTreeMap::new();
    for (name, shape) in shapes {
        result.insert(format!("model.{name}"), shape.clone());
        if m.training.is_some() {
            result.insert(format!("adam.m.{name}"), shape.clone());
            result.insert(format!("adam.v.{name}"), shape);
        }
    }
    result
}
fn validate<'a>(
    m: &Manifest,
    tok: &ByteBpe,
    bytes: &'a [u8],
) -> Result<safetensors::SafeTensors<'a>> {
    m.architecture.validate()?;
    if m.version != 1
        || m.dtype != "F32"
        || m.tokenizer_sha256 != tok.id()
        || m.architecture.vocab != tok.vocab_size()
        || m.weights_bytes != bytes.len()
        || m.weights_sha256 != hash(bytes)
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
        ]
        .contains(&m.status.as_str())
    {
        return Err(Error::Corrupt(
            "checkpoint version/shape/dtype/tokenizer/checksum/lineage".into(),
        ));
    }
    if let Some(s) = &m.training {
        s.config.validate(m.architecture.context)?;
        if s.step > s.config.max_steps
            || s.consumed_tokens > s.config.max_tokens
            || s.target_tokens > s.consumed_tokens
            || s.corpus_hash != tok.train_hash
            || s.initial_weight_hash != m.initial_weight_hash
            || s.train_loss.is_some_and(|v| !v.is_finite())
            || s.validation_loss.is_some_and(|v| !v.is_finite())
        {
            return Err(Error::Corrupt("checkpoint training state".into()));
        }
    }
    let tensors =
        safetensors::SafeTensors::deserialize(bytes).map_err(|e| Error::Corrupt(e.to_string()))?;
    if tensors.len() != m.tensors.len() {
        return Err(Error::Corrupt("checkpoint tensor count".into()));
    }
    for (name, shape) in &m.tensors {
        let t = tensors
            .tensor(name)
            .map_err(|e| Error::Corrupt(e.to_string()))?;
        if t.shape() != shape
            || t.dtype() != safetensors::Dtype::F32
            || t.data()
                .as_chunks::<4>()
                .0
                .iter()
                .any(|b| !f32::from_le_bytes(*b).is_finite())
        {
            return Err(Error::Corrupt(format!("invalid checkpoint tensor {name}")));
        }
    }
    Ok(tensors)
}
pub fn load(path: &Path, device: Device, resume: bool) -> Result<Loaded> {
    let manifest: Manifest =
        serde_json::from_slice(&read_bounded(&path.join("manifest.json"), 1024 * 1024)?)?;
    manifest.architecture.validate()?;
    let tok = ByteBpe::load(&path.join("tokenizer.json"))?;
    let cap =
        manifest.architecture.parameters() * 4 * if manifest.training.is_some() { 3 } else { 1 }
            + 1024 * 1024;
    let bytes = read_bounded(&path.join("weights.safetensors"), cap)?;
    let tensors = validate(&manifest, &tok, &bytes)?;
    let mut model = BTreeMap::new();
    let mut optimizer = BTreeMap::new();
    for name in manifest.tensors.keys() {
        if let Some(key) = name.strip_prefix("model.") {
            let view = tensors
                .tensor(name)
                .map_err(|e| Error::Corrupt(e.to_string()))?;
            let values: Vec<f32> = view
                .data()
                .as_chunks::<4>()
                .0
                .iter()
                .map(|b| f32::from_le_bytes(*b))
                .collect();
            model.insert(
                key.to_string(),
                Tensor::from_vec(values, view.shape(), &device)?,
            );
        } else if resume {
            let view = tensors
                .tensor(name)
                .map_err(|e| Error::Corrupt(e.to_string()))?;
            let values: Vec<f32> = view
                .data()
                .as_chunks::<4>()
                .0
                .iter()
                .map(|b| f32::from_le_bytes(*b))
                .collect();
            optimizer.insert(
                name.clone(),
                Tensor::from_vec(values, view.shape(), &device)?,
            );
        }
    }
    let mut model = Transformer::from_tensors(manifest.architecture.clone(), model, device)?;
    model.bind_tokenizer(&tok.id())?;
    Ok(Loaded {
        model,
        tokenizer: tok,
        manifest,
        optimizer,
    })
}
pub fn save(
    path: &Path,
    model: &Transformer,
    tokenizer: &ByteBpe,
    mut manifest: Manifest,
    optimizer: &BTreeMap<String, Tensor>,
) -> Result<Manifest> {
    std::fs::create_dir(path)?;
    let mut tensors: HashMap<String, Tensor> = model
        .vars
        .iter()
        .map(|(name, var)| (format!("model.{name}"), var.as_detached_tensor()))
        .collect();
    for (name, t) in optimizer {
        if tensors.insert(name.clone(), t.detach()).is_some() {
            return Err(Error::Invalid("duplicate checkpoint tensor".into()));
        }
    }
    let weights = path.join("weights.safetensors");
    candle_core::safetensors::save(&tensors, &weights)?;
    std::fs::File::open(&weights)?.sync_all()?;
    tokenizer.save(&path.join("tokenizer.json"))?;
    let cap = model.config.parameters() * 12 + 1024 * 1024;
    let bytes = read_bounded(&weights, cap)?;
    manifest.weights_sha256 = hash(&bytes);
    manifest.weights_bytes = bytes.len();
    manifest.tokenizer_sha256 = tokenizer.id();
    manifest.architecture = model.config.clone();
    manifest.tensors = tensors
        .iter()
        .map(|(k, t)| (k.clone(), t.dims().to_vec()))
        .collect();
    validate(&manifest, tokenizer, &bytes)?;
    let temporary = path.join("manifest.pending");
    write_new(&temporary, &serde_json::to_vec_pretty(&manifest)?)?;
    // Atomic no-clobber publication; an interrupted directory without manifest is
    // explicitly incomplete. Earlier checkpoint directories are never modified.
    std::fs::hard_link(&temporary, path.join("manifest.json"))?;
    std::fs::File::open(path)?.sync_all()?;
    std::fs::remove_file(temporary)?;
    Ok(manifest)
}
pub fn initialized(
    model: &Transformer,
    tok: &ByteBpe,
    seed: u64,
    source_id: String,
) -> Result<Manifest> {
    Ok(Manifest {
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
    })
}
