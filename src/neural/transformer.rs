//! Native decoder built from generic tensors/Vars; training and generation share it.
use super::{EOS, SPECIALS, hash};
use crate::{Error, Result};
use candle_core::{D, DType, Device, Tensor, Var};
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    sync::atomic::{AtomicBool, Ordering},
    time::Instant,
};
#[cfg(feature = "test-support")]
thread_local! {
    static FIXTURE_TIMEOUT: std::cell::Cell<Option<usize>> = const { std::cell::Cell::new(None) };
}
/// One native TINY call's clock only; neither logits nor token selection changes.
#[cfg(feature = "test-support")]
pub fn fixture_timeout_after(tokens: usize) {
    FIXTURE_TIMEOUT.with(|clock| clock.set(Some(tokens)));
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Config {
    pub profile: String,
    pub vocab: usize,
    pub layers: usize,
    pub hidden: usize,
    pub heads: usize,
    pub kv_heads: usize,
    pub head_dim: usize,
    pub ffn: usize,
    pub local_layers: usize,
    pub window: usize,
    pub context: usize,
    pub eps: f64,
    pub rope_theta: f64,
}
impl Config {
    pub fn small(vocab: usize) -> Self {
        Self {
            profile: "NATIVE_TRPP_G1_SMALL".into(),
            vocab,
            layers: 6,
            hidden: 384,
            heads: 8,
            kv_heads: 2,
            head_dim: 48,
            ffn: 1024,
            local_layers: 5,
            window: 256,
            context: 2048,
            eps: 1e-6,
            rope_theta: 10000.,
        }
    }
    pub fn tiny(vocab: usize) -> Self {
        Self {
            profile: "TINY_NUMERIC_TEST_ONLY".into(),
            vocab,
            layers: 2,
            hidden: 32,
            heads: 4,
            kv_heads: 2,
            head_dim: 8,
            ffn: 64,
            local_layers: 1,
            window: 8,
            context: 64,
            eps: 1e-6,
            rope_theta: 10000.,
        }
    }
    pub fn validate(&self) -> Result<()> {
        if self.layers == 0
            || self.layers > 6
            || self.hidden == 0
            || self.hidden > 384
            || self.heads == 0
            || self.heads > 8
            || self.kv_heads == 0
            || !self.heads.is_multiple_of(self.kv_heads)
            || self.head_dim == 0
            || self.head_dim > 48
            || !self.head_dim.is_multiple_of(2)
            || self.hidden != self.heads * self.head_dim
            || self.ffn == 0
            || self.ffn > 1024
            || self.local_layers >= self.layers
            || self.window == 0
            || self.window > 256
            || self.context < self.window
            || self.context > 2048
            || !(264..=4096).contains(&self.vocab)
            || !self.eps.is_finite()
            || self.eps <= 0.
            || !self.rope_theta.is_finite()
            || self.rope_theta <= 1.
        {
            return Err(Error::Invalid(
                "native architecture bounds/unsupported feature".into(),
            ));
        }
        if *self != Self::small(self.vocab)
            && *self != Self::tiny(self.vocab)
            && self.profile != "NATIVE_TRPP_EXPERIMENTAL_V1"
        {
            return Err(Error::Invalid(
                "unsupported native profile/configuration".into(),
            ));
        }
        Ok(())
    }
    pub fn id(&self) -> Result<String> {
        Ok(hash(&crate::binary::to_vec(self)?))
    }
    /// Formula identity, independent of the legacy JSON wire representation and profile label.
    pub fn semantic_id(&self) -> Result<String> {
        self.validate()?;
        let mut bytes = Vec::new();
        for text in [
            MIXER.family_id,
            MIXER.equation_version,
            MIXER.parameter_schema_id,
            MIXER.state_schema_id,
            MIXER.numeric_policy,
        ] {
            crate::codec::put_varint(&mut bytes, text.len() as u64);
            bytes.extend_from_slice(text.as_bytes());
        }
        for n in [
            self.vocab,
            self.layers,
            self.hidden,
            self.heads,
            self.kv_heads,
            self.head_dim,
            self.ffn,
            self.local_layers,
            self.window,
            self.context,
        ] {
            crate::codec::put_varint(&mut bytes, n as u64);
        }
        bytes.extend_from_slice(&self.eps.to_le_bytes());
        bytes.extend_from_slice(&self.rope_theta.to_le_bytes());
        Ok(hash(&bytes))
    }
    pub fn shapes(&self) -> BTreeMap<String, Vec<usize>> {
        let mut out = BTreeMap::new();
        out.insert("embedding".into(), vec![self.vocab, self.hidden]);
        out.insert("final_norm".into(), vec![self.hidden]);
        for i in 0..self.layers {
            for (name, shape) in [
                ("attn_norm", vec![self.hidden]),
                ("ffn_norm", vec![self.hidden]),
                ("q_norm", vec![self.heads, 1, self.head_dim]),
                ("k_norm", vec![self.kv_heads, 1, self.head_dim]),
                ("q", vec![self.hidden, self.hidden]),
                ("k", vec![self.kv_heads * self.head_dim, self.hidden]),
                ("v", vec![self.kv_heads * self.head_dim, self.hidden]),
                ("o", vec![self.hidden, self.hidden]),
                ("gate", vec![self.ffn, self.hidden]),
                ("up", vec![self.ffn, self.hidden]),
                ("down", vec![self.hidden, self.ffn]),
            ] {
                out.insert(format!("layer.{i}.{name}"), shape);
            }
        }
        out
    }
    pub fn parameters(&self) -> usize {
        self.shapes()
            .values()
            .map(|s| s.iter().product::<usize>())
            .sum()
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct OperatorSpec {
    pub family_id: &'static str,
    pub equation_version: &'static str,
    pub parameter_schema_id: &'static str,
    pub state_schema_id: &'static str,
    pub numeric_policy: &'static str,
}
pub const MIXER: OperatorSpec = OperatorSpec {
    family_id: "native-trpp-gqa",
    equation_version: "prerms-qknorm-rope-causal-local-global-swiglu-tied-v1",
    parameter_schema_id: "native-trpp-parameters-v1",
    state_schema_id: "absolute-kv-history-v1",
    numeric_policy: "cpu-f32-reference",
};
#[derive(Clone, Debug)]
pub struct Capabilities {
    pub training: bool,
    pub backward: bool,
    pub prefill: bool,
    pub decode: bool,
    pub cache: bool,
    pub dtype: DType,
    pub device: &'static str,
    pub context: usize,
}
/// Only the decode linear operation is experimental. Training and prefill explicitly
/// dispatch to the differentiable reference; candidate tensors must not enter autograd.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Kernel {
    #[default]
    Reference,
    RustDecodeGemv,
}
impl Kernel {
    pub fn id(self) -> &'static str {
        match self {
            Self::Reference => "candle-linear-v1",
            Self::RustDecodeGemv => "rust-f32-decode-gemv-v1",
        }
    }
    pub fn capabilities(self, context: usize) -> Capabilities {
        Capabilities {
            training: self == Self::Reference,
            backward: self == Self::Reference,
            prefill: self == Self::Reference,
            decode: true,
            cache: true,
            dtype: DType::F32,
            device: "CPU",
            context,
        }
    }
}
/// Inference only. Includes input/weight copies in its measured cost; no stale weight cache.
pub fn decode_linear(x: &Tensor, weight: &Tensor, kernel: Kernel) -> candle_core::Result<Tensor> {
    if kernel == Kernel::Reference {
        return linear(x, weight);
    }
    let (n, k) = weight.dims2()?;
    if x.dtype() != DType::F32
        || weight.dtype() != DType::F32
        || !x.device().is_cpu()
        || !weight.device().is_cpu()
        || k == 0
        || n == 0
        || x.elem_count() != k
        || x.dims().last() != Some(&k)
        || !x.is_contiguous()
        || !weight.is_contiguous()
    {
        candle_core::bail!("RustDecodeGemv requires contiguous CPU F32 M=1, nonzero N/K");
    }
    let input = x.flatten_all()?.to_vec1::<f32>()?;
    let weights = weight.flatten_all()?.to_vec1::<f32>()?;
    let output: Vec<f32> = weights
        .chunks_exact(k)
        .map(|row| row.iter().zip(&input).map(|(w, x)| w * x).sum())
        .collect();
    let mut shape = x.dims().to_vec();
    *shape.last_mut().expect("nonempty input") = n;
    Tensor::from_vec(output, shape, x.device())
}
#[derive(Default, Debug)]
pub struct ForwardTimings {
    pub linear_ns: u128,
    pub linear_calls: usize,
    pub linear_shapes: BTreeMap<String, usize>,
}
fn measured_linear(
    x: &Tensor,
    weight: &Tensor,
    kernel: Kernel,
    profile: &mut Option<&mut ForwardTimings>,
) -> candle_core::Result<Tensor> {
    let Some(profile) = profile else {
        return decode_linear(x, weight, kernel);
    };
    let start = Instant::now();
    let result = decode_linear(x, weight, kernel)?;
    profile.linear_ns += start.elapsed().as_nanos();
    profile.linear_calls += 1;
    let input = weight.dim(1)?;
    let description = format!(
        "B/T={:?} M={} N={} K={} x_stride={:?} weight_stride={:?} F32 CPU",
        &x.dims()[..x.rank() - 1],
        x.elem_count() / input,
        weight.dim(0)?,
        input,
        x.stride(),
        weight.stride()
    );
    *profile.linear_shapes.entry(description).or_default() += 1;
    Ok(result)
}
// Explicit deterministic initialization/sampler state, independent of backend RNG.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Rng {
    pub state: u64,
}
impl Rng {
    pub fn new(seed: u64) -> Self {
        Self { state: seed }
    }
    pub fn next_u64(&mut self) -> u64 {
        self.state = self.state.wrapping_add(0x9e3779b97f4a7c15);
        let mut z = self.state;
        z = (z ^ (z >> 30)).wrapping_mul(0xbf58476d1ce4e5b9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94d049bb133111eb);
        z ^ (z >> 31)
    }
    fn normal(&mut self) -> f32 {
        let a = ((self.next_u64() >> 11) as f64 + 1.) / ((1u64 << 53) as f64 + 1.);
        let b = (self.next_u64() >> 11) as f64 / (1u64 << 53) as f64;
        ((-2. * a.ln()).sqrt() * (std::f64::consts::TAU * b).cos()) as f32
    }
}
pub struct Transformer {
    pub config: Config,
    pub vars: BTreeMap<String, Var>,
    pub device: Device,
    identity: String,
    tokenizer_identity: String,
    kernel: Kernel,
    frozen: Option<BTreeMap<String, Tensor>>,
    adapter: Option<QvAdapter>,
    pub(crate) adapter_base: Option<super::artifact::AdapterBase>,
}
/// Explicit execution profile; no task-dependent selection or dropout.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct QvAdapter {
    pub profile: String,
    pub rank: usize,
    pub alpha_bits: u64,
    pub seed: u64,
}
impl QvAdapter {
    pub fn registered(seed: u64) -> Self {
        Self { profile: "R3-QV-LORA-R8-A8-F32-v1".into(), rank: 8, alpha_bits: 8f64.to_bits(), seed }
    }
    pub fn shapes(&self, config: &Config) -> Result<BTreeMap<String, Vec<usize>>> {
        config.validate()?;
        if self.profile != "R3-QV-LORA-R8-A8-F32-v1" || self.rank != 8 || self.alpha_bits != 8f64.to_bits() {
            return Err(Error::Invalid("unsupported q/v adapter profile/rank/scaling".into()));
        }
        let mut shapes = BTreeMap::new();
        for i in 0..config.layers {
            for (name, out) in [("q",config.heads*config.head_dim),("v",config.kv_heads*config.head_dim)] {
                shapes.insert(format!("adapter.layer.{i}.{name}.a"),vec![self.rank,config.hidden]);
                shapes.insert(format!("adapter.layer.{i}.{name}.b"),vec![out,self.rank]);
            }
        }
        Ok(shapes)
    }
}
pub fn attention_mask(
    query: std::ops::Range<usize>,
    keys: std::ops::Range<usize>,
    window: Option<usize>,
    padding: Option<&[bool]>,
    batch: usize,
    device: &Device,
) -> Result<Tensor> {
    if query.is_empty()
        || keys.is_empty()
        || query.end > 2048
        || keys.end > query.end
        || batch == 0
        || batch > 8
        || window == Some(0)
        || padding.is_some_and(|p| p.len() != batch * query.end)
    {
        return Err(Error::Invalid("attention mask bounds".into()));
    }
    let mut mask = Vec::with_capacity(batch * query.len() * keys.len());
    for b in 0..batch {
        for q in query.clone() {
            for k in keys.clone() {
                let valid = k <= q
                    && window.is_none_or(|w| k + w > q)
                    && padding.is_none_or(|p| p[b * query.end + q] && p[b * query.end + k]);
                mask.push(if valid { 1f32 } else { 0. });
            }
        }
    }
    Ok(Tensor::from_vec(
        mask,
        (batch, 1, query.len(), keys.len()),
        device,
    )?)
}
/// Read-only diagnostic hook; receives the actual post-RoPE Q/K and explicit mask.
pub type AttentionObserver<'a> = dyn FnMut(usize, &Tensor, &Tensor, &Tensor) -> Result<()> + 'a;

pub fn rms_norm(x: &Tensor, weight: &Tensor, eps: f64) -> candle_core::Result<Tensor> {
    x.broadcast_div(&(x.sqr()?.mean_keepdim(D::Minus1)? + eps)?.sqrt()?)?
        .broadcast_mul(weight)
}
pub fn linear(x: &Tensor, weight: &Tensor) -> candle_core::Result<Tensor> {
    let mut shape = x.dims().to_vec();
    let last = shape.len() - 1;
    let input = shape[last];
    shape[last] = weight.dim(0)?;
    x.reshape((x.elem_count() / input, input))?
        .matmul(&weight.t()?)?
        .reshape(shape)
}
pub fn rotary(x: &Tensor, position: usize, theta: f64) -> candle_core::Result<Tensor> {
    let (b, h, t, d) = x.dims4()?;
    if d == 0 || !d.is_multiple_of(2) {
        candle_core::bail!("even nonzero rotary dimension required");
    }
    let mut cos = Vec::with_capacity(t * d / 2);
    let mut sin = Vec::with_capacity(t * d / 2);
    for p in position..position + t {
        for i in 0..d / 2 {
            let angle = p as f64 / theta.powf((2 * i) as f64 / d as f64);
            cos.push(angle.cos() as f32);
            sin.push(angle.sin() as f32);
        }
    }
    let cos = Tensor::from_vec(cos, (1, 1, t, d / 2), x.device())?;
    let sin = Tensor::from_vec(sin, (1, 1, t, d / 2), x.device())?;
    let pairs = x.reshape((b, h, t, d / 2, 2))?;
    let even = pairs.narrow(4, 0, 1)?.squeeze(4)?;
    let odd = pairs.narrow(4, 1, 1)?.squeeze(4)?;
    let re = (even.broadcast_mul(&cos)? - odd.broadcast_mul(&sin)?)?;
    let im = (even.broadcast_mul(&sin)? + odd.broadcast_mul(&cos)?)?;
    Tensor::stack(&[re, im], 4)?.reshape((b, h, t, d))
}
pub fn repeat_kv(kv: &Tensor, heads: usize) -> candle_core::Result<Tensor> {
    let (b, k, t, d) = kv.dims4()?;
    if k == 0 || !heads.is_multiple_of(k) {
        candle_core::bail!("GQA group mismatch");
    }
    kv.unsqueeze(2)?
        .broadcast_as((b, k, heads / k, t, d))?
        .contiguous()?
        .reshape((b, heads, t, d))
}
/// Current mixer equation, kept separate from projections and artifact/state storage.
/// Candle tensors/autograd remain the runtime boundary; this is not a tensor-runtime API.
pub fn gqa_attention(
    q: &Tensor,
    k: &Tensor,
    v: &Tensor,
    allowed: &Tensor,
) -> candle_core::Result<Tensor> {
    let (_, heads, _, dim) = q.dims4()?;
    let bias = ((allowed - 1.)? * 1e9)?;
    let expanded_k = repeat_kv(k, heads)?;
    let expanded_v = repeat_kv(v, heads)?;
    let logits = (q
        .contiguous()?
        .matmul(&expanded_k.transpose(2, 3)?.contiguous()?)?
        / (dim as f64).sqrt())?
    .broadcast_add(&bias)?;
    // All-masked rows produce zero attention, without NaNs or future leak.
    candle_nn::ops::softmax(&logits, D::Minus1)?
        .broadcast_mul(allowed)?
        .matmul(&expanded_v)
}
#[derive(Clone)]
struct LayerCache {
    k: Tensor,
    v: Tensor,
    start: usize,
}
#[derive(Clone)]
pub struct Cache {
    layers: Vec<Option<LayerCache>>,
    pub position: usize,
    identity: String,
    device: Device,
    scope: String,
    history: sha2::Sha256,
    pub max_attention_bytes: usize,
}
impl Cache {
    pub fn history_id(&self) -> String {
        use sha2::Digest;
        format!("{:x}", self.history.clone().finalize())
    }
    pub fn bytes(&self) -> usize {
        self.layers
            .iter()
            .flatten()
            .map(|l| (l.k.elem_count() + l.v.elem_count()) * l.k.dtype().size_in_bytes())
            .sum()
    }
    pub fn retained_tokens(&self) -> Vec<usize> {
        self.layers
            .iter()
            .map(|l| l.as_ref().map_or(0, |l| l.k.dims()[2]))
            .collect()
    }
    pub fn reset(&mut self, scope: &str) {
        self.layers.fill(None);
        self.position = 0;
        self.scope = scope.into();
        self.history = Default::default();
        self.max_attention_bytes = 0;
    }
}
impl Transformer {
    pub fn init(config: Config, seed: u64, device: Device) -> Result<Self> {
        config.validate()?;
        let mut rng = Rng::new(seed);
        let mut vars = BTreeMap::new();
        for (name, shape) in config.shapes() {
            let n = shape.iter().product();
            let data = if name.ends_with("norm") {
                vec![1f32; n]
            } else {
                (0..n).map(|_| rng.normal() * 0.02).collect()
            };
            vars.insert(name, Var::from_vec(data, shape, &device)?);
        }
        let mut model = Self {
            config,
            vars,
            device,
            identity: String::new(),
            tokenizer_identity: String::new(),
            kernel: Kernel::Reference,
            frozen: None,
            adapter: None,
            adapter_base: None,
        };
        model.refresh_identity()?;
        Ok(model)
    }
    pub fn from_tensors(
        config: Config,
        tensors: BTreeMap<String, Tensor>,
        device: Device,
    ) -> Result<Self> {
        config.validate()?;
        let shapes = config.shapes();
        if tensors.len() != shapes.len() {
            return Err(Error::Corrupt("native tensor count".into()));
        }
        let mut vars = BTreeMap::new();
        for (name, shape) in shapes {
            let t = tensors
                .get(&name)
                .ok_or_else(|| Error::Corrupt(format!("missing tensor {name}")))?;
            if t.dims() != shape
                || t.dtype() != DType::F32
                || !t.device().same_device(&device)
                || t.flatten_all()?
                    .to_vec1::<f32>()?
                    .iter()
                    .any(|x| !x.is_finite())
            {
                return Err(Error::Corrupt(format!(
                    "tensor shape/dtype/device/nonfinite: {name}"
                )));
            }
            vars.insert(name, Var::from_tensor(t)?);
        }
        let mut model = Self {
            config,
            vars,
            device,
            identity: String::new(),
            tokenizer_identity: String::new(),
            kernel: Kernel::Reference,
            frozen: None,
            adapter: None,
            adapter_base: None,
        };
        model.refresh_identity()?;
        Ok(model)
    }
    pub fn weight_hash(&self) -> Result<String> {
        use sha2::{Digest, Sha256};
        let mut digest = Sha256::new();
        digest.update(self.config.id()?);
        for (name, var) in self.content_tensors() {
            digest.update(name.as_bytes());
            for x in var.flatten_all()?.to_vec1::<f32>()? {
                digest.update(x.to_le_bytes());
            }
        }
        Ok(format!("{:x}", digest.finalize()))
    }
    pub fn set_kernel(&mut self, kernel: Kernel) {
        self.kernel = kernel;
    }
    pub fn kernel(&self) -> Kernel {
        self.kernel
    }
    pub fn refresh_identity(&mut self) -> Result<()> {
        self.identity = hash(
            format!(
                "{}:{}:{}:{}",
                self.config.semantic_id()?,
                self.weights_content_id()?,
                self.tokenizer_identity,
                MIXER.state_schema_id
            )
            .as_bytes(),
        );
        Ok(())
    }
    /// Content only; legacy weight_hash intentionally remains unchanged for imported lineage.
    pub fn weights_content_id(&self) -> Result<String> {
        use sha2::{Digest, Sha256};
        let mut digest = Sha256::new();
        for (name, var) in self.content_tensors() {
            digest.update((name.len() as u64).to_le_bytes());
            digest.update(name.as_bytes());
            digest.update((var.rank() as u64).to_le_bytes());
            for &n in var.dims() {
                digest.update((n as u64).to_le_bytes());
            }
            for x in var.flatten_all()?.to_vec1::<f32>()? {
                digest.update(x.to_le_bytes());
            }
        }
        Ok(format!("{:x}", digest.finalize()))
    }
    fn content_tensors(&self) -> BTreeMap<&str, &Tensor> {
        let mut out: BTreeMap<_,_> = self.vars.iter().map(|(n,v)|(n.as_str(),v.as_tensor())).collect();
        if let Some(base)=&self.frozen { out.extend(base.iter().map(|(n,t)|(n.as_str(),t))); }
        out
    }
    pub fn adapter(&self) -> Option<&QvAdapter> { self.adapter.as_ref() }
    pub fn base_tensors(&self) -> BTreeMap<String,Tensor> {
        self.frozen.clone().unwrap_or_else(||self.vars.iter().map(|(n,v)|(n.clone(),v.as_detached_tensor())).collect())
    }
    pub fn base_content_id(&self)->Result<String> {
        if let Some(base)=&self.frozen {
            use sha2::{Digest,Sha256};let mut h=Sha256::new();
            for(name,t)in base {h.update((name.len() as u64).to_le_bytes());h.update(name.as_bytes());h.update((t.rank() as u64).to_le_bytes());
                for &n in t.dims(){h.update((n as u64).to_le_bytes());}for x in t.flatten_all()?.to_vec1::<f32>()?{h.update(x.to_le_bytes());}}
            Ok(format!("{:x}",h.finalize()))
        }else{self.weights_content_id()}
    }
    fn weight(&self,name:&str)->&Tensor {
        match &self.frozen {Some(base)=>&base[name],None=>self.vars[name].as_tensor()}
    }
    pub fn initialize_adapter(&mut self, descriptor: QvAdapter) -> Result<()> {
        let mut rng=Rng::new(descriptor.seed);let mut tensors=BTreeMap::new();
        for(name,shape)in descriptor.shapes(&self.config)? {
            let n=shape.iter().product();let scale=1f32/(self.config.hidden as f32).sqrt();
            let values=if name.ends_with(".a"){(0..n).map(|_|rng.normal()*scale).collect()}else{vec![0f32;n]};
            tensors.insert(name,Tensor::from_vec(values,shape,&self.device)?);
        }
        self.install_adapter(descriptor,tensors)
    }
    pub fn install_adapter(&mut self,descriptor:QvAdapter,tensors:BTreeMap<String,Tensor>)->Result<()> {
        if self.adapter.is_some()||self.frozen.is_some(){return Err(Error::Invalid("adapter already installed".into()));}
        let shapes=descriptor.shapes(&self.config)?;
        if tensors.len()!=shapes.len(){return Err(Error::Corrupt("missing/extra adapter tensor".into()));}
        let mut vars=BTreeMap::new();
        for(name,shape)in shapes {
            let t=tensors.get(&name).ok_or_else(||Error::Corrupt(format!("missing adapter {name}")))?;
            if t.dims()!=shape||t.dtype()!=DType::F32||!t.device().same_device(&self.device)
                ||t.flatten_all()?.to_vec1::<f32>()?.iter().any(|v|!v.is_finite()) {
                return Err(Error::Corrupt(format!("adapter shape/dtype/nonfinite {name}")));
            }
            vars.insert(name,Var::from_tensor(t)?);
        }
        // Drop all base Vars: autograd retains activation flow through constant
        // weights, but no base leaf gradients or optimizer slots can be allocated.
        self.frozen=Some(self.base_tensors());self.vars=vars;self.adapter=Some(descriptor);
        self.refresh_identity()
    }
    fn projection(&self,x:&Tensor,name:&str,kernel:Kernel,profile:&mut Option<&mut ForwardTimings>)->Result<Tensor> {
        let base=measured_linear(x,self.weight(name),kernel,profile)?;
        if let Some(a)=&self.adapter {
            let down=measured_linear(x,self.vars[&format!("adapter.{name}.a")].as_tensor(),kernel,profile)?;
            let up=measured_linear(&down,self.vars[&format!("adapter.{name}.b")].as_tensor(),kernel,profile)?;
            Ok((base+(up*(f64::from_bits(a.alpha_bits)/a.rank as f64))?)?)
        }else{Ok(base)}
    }
    pub fn bind_tokenizer(&mut self, id: &str) -> Result<()> {
        self.tokenizer_identity = id.into();
        self.refresh_identity()
    }
    pub fn cache(&self, scope: &str) -> Cache {
        Cache {
            layers: vec![None; self.config.layers],
            position: 0,
            identity: self.identity.clone(),
            device: self.device.clone(),
            scope: scope.into(),
            history: Default::default(),
            max_attention_bytes: 0,
        }
    }
    /// Device-aware execution claims; the historical kernel profile is CPU-only.
    /// Metal inference is opt-in, and the measured gradient failure blocks training.
    pub fn capabilities(&self) -> Capabilities {
        let mut c=self.kernel.capabilities(self.config.context);
        if self.device.is_metal() {
            c.device="Metal";c.training=false;c.backward=false;
            if self.kernel!=Kernel::Reference {c.prefill=false;c.decode=false;c.cache=false;}
        }
        c
    }
    pub fn forward(&self, ids: &Tensor, padding: Option<&[bool]>) -> Result<Tensor> {
        self.forward_inner(ids, padding, None, "", None, None)
    }
    pub fn forward_observed(
        &self,
        ids: &Tensor,
        padding: Option<&[bool]>,
        observe: &mut AttentionObserver<'_>,
    ) -> Result<Tensor> {
        self.forward_inner(ids, padding, None, "", None, Some(observe))
    }
    pub fn forward_cached(&self, ids: &Tensor, cache: &mut Cache, scope: &str) -> Result<Tensor> {
        self.forward_inner(ids, None, Some(cache), scope, None, None)
    }
    pub fn forward_profiled(
        &self,
        ids: &Tensor,
        cache: &mut Cache,
        scope: &str,
        timings: &mut ForwardTimings,
    ) -> Result<Tensor> {
        self.forward_inner(ids, None, Some(cache), scope, Some(timings), None)
    }
    fn forward_inner(
        &self,
        ids: &Tensor,
        padding: Option<&[bool]>,
        mut cache: Option<&mut Cache>,
        scope: &str,
        mut profile: Option<&mut ForwardTimings>,
        mut observe: Option<&mut AttentionObserver<'_>>,
    ) -> Result<Tensor> {
        let (batch, len) = ids.dims2()?;
        if self.kernel == Kernel::RustDecodeGemv && !self.device.is_cpu() {
            return Err(Error::Unsupported("RustDecodeGemv requires CPU; select Reference explicitly".into()));
        }
        // A forward used for training always retains the reference autograd graph.
        let kernel = if cache.is_some() && len == 1 {
            self.kernel
        } else {
            Kernel::Reference
        };
        let c = &self.config;
        let position = cache.as_ref().map_or(0, |c| c.position);
        if ids.dtype() != DType::U32
            || !ids.device().same_device(&self.device)
            || batch == 0
            || batch > 8
            || len == 0
            || position.checked_add(len).is_none_or(|n| n > c.context)
            || padding.is_some_and(|p| p.len() != batch * len)
            || cache
                .as_ref()
                .is_some_and(|k| k.identity != self.identity || !k.device.same_device(&self.device) || k.scope != scope || batch != 1)
        {
            return Err(Error::Invalid(
                "native input shape/dtype/device/context/cache identity".into(),
            ));
        }
        if ids
            .flatten_all()?
            .to_vec1::<u32>()?
            .iter()
            .any(|&v| v as usize >= c.vocab)
        {
            return Err(Error::Invalid("native token ID".into()));
        }
        let embed = self.weight("embedding");
        let mut x = embed
            .index_select(&ids.flatten_all()?, 0)?
            .reshape((batch, len, c.hidden))?;
        for i in 0..c.layers {
            let w = |name: &str| self.weight(&format!("layer.{i}.{name}"));
            let norm = rms_norm(&x, w("attn_norm"), c.eps)?;
            let q = self.projection(&norm, &format!("layer.{i}.q"), kernel, &mut profile)?
                .reshape((batch, len, c.heads, c.head_dim))?
                .transpose(1, 2)?;
            let k = measured_linear(&norm, w("k"), kernel, &mut profile)?
                .reshape((batch, len, c.kv_heads, c.head_dim))?
                .transpose(1, 2)?;
            let v = self.projection(&norm, &format!("layer.{i}.v"), kernel, &mut profile)?
                .reshape((batch, len, c.kv_heads, c.head_dim))?
                .transpose(1, 2)?;
            let q = rotary(&rms_norm(&q, w("q_norm"), c.eps)?, position, c.rope_theta)?;
            let mut k = rotary(&rms_norm(&k, w("k_norm"), c.eps)?, position, c.rope_theta)?;
            let mut v = v;
            let mut start = position;
            if let Some(previous) = cache.as_ref().and_then(|k| k.layers[i].as_ref()) {
                start = previous.start;
                k = Tensor::cat(&[&previous.k, &k], 2)?;
                v = Tensor::cat(&[&previous.v, &v], 2)?;
            }
            let keys = k.dim(2)?;
            let local = i < c.local_layers;
            let allowed = attention_mask(
                position..position + len,
                start..start + keys,
                local.then_some(c.window),
                padding,
                batch,
                &self.device,
            )?;
            if let Some(observer) = &mut observe {
                observer(i, &q, &k, &allowed)?;
            }
            let attention = gqa_attention(&q, &k, &v, &allowed)?
                .transpose(1, 2)?
                .contiguous()?
                .reshape((batch, len, c.hidden))?;
            x = (x + measured_linear(&attention, w("o"), kernel, &mut profile)?)?;
            let norm = rms_norm(&x, w("ffn_norm"), c.eps)?;
            let hidden = (measured_linear(&norm, w("gate"), kernel, &mut profile)?.silu()?
                * measured_linear(&norm, w("up"), kernel, &mut profile)?)?;
            x = (x + measured_linear(&hidden, w("down"), kernel, &mut profile)?)?;
            if let Some(cache) = cache.as_mut() {
                // Evict only after the entire chunk has attended to its history.
                let keep = if local { keys.min(c.window) } else { keys };
                let offset = keys - keep;
                cache.layers[i] = Some(LayerCache {
                    k: k.narrow(2, offset, keep)?.contiguous()?.detach(),
                    v: v.narrow(2, offset, keep)?.contiguous()?.detach(),
                    start: start + offset,
                });
                cache.max_attention_bytes = cache
                    .max_attention_bytes
                    .max(batch * c.heads * len * keys * 4);
            }
        }
        if let Some(cache) = cache {
            use sha2::Digest;
            for id in ids.flatten_all()?.to_vec1::<u32>()? {
                cache.history.update(id.to_le_bytes());
            }
            cache.position += len;
        }
        Ok(measured_linear(
            &rms_norm(&x, self.weight("final_norm"), c.eps)?,
            embed,
            kernel,
            &mut profile,
        )?)
    }
    pub fn generate(
        &self,
        prompt: &[u32],
        max_new: usize,
        timeout_ms: u64,
        cancel: &AtomicBool,
        scope: &str,
    ) -> Result<Generated> {
        self.generate_observed(prompt, max_new, timeout_ms, cancel, scope, |_| {})
    }
    /// Observe actual argmax IDs, including EOS/control, before any error is returned.
    /// Uses the identical product generation loop; the observer cannot change logits.
    #[allow(clippy::too_many_arguments)]
    pub fn generate_observed(
        &self,
        prompt: &[u32],
        max_new: usize,
        timeout_ms: u64,
        cancel: &AtomicBool,
        scope: &str,
        mut observe: impl FnMut(u32),
    ) -> Result<Generated> {
        if prompt.is_empty()
            || max_new == 0
            || max_new > 512
            || prompt
                .len()
                .checked_add(max_new)
                .is_none_or(|n| n > self.config.context)
        {
            return Err(Error::ContextTooSmall);
        }
        let start = Instant::now();
        let mut cache = self.cache(scope);
        let mut logits = None;
        #[cfg(feature = "test-support")]
        let timeout_after = FIXTURE_TIMEOUT.with(|clock| clock.take());
        #[cfg(feature = "test-support")]
        if timeout_after.is_some() {
            assert_eq!((self.config.hidden, self.config.layers), (32, 2));
        }
        #[cfg(feature = "test-support")]
        let observed_tokens = std::cell::Cell::new(0usize);
        let check = || -> Result<()> {
            let elapsed = start.elapsed().as_millis();
            #[cfg(feature = "test-support")]
            let elapsed = if timeout_after.is_some_and(|n| observed_tokens.get() >= n) {
                u128::from(timeout_ms) + 1
            } else {
                elapsed
            };
            if cancel.load(Ordering::Relaxed) {
                Err(Error::Cancelled)
            } else if elapsed > u128::from(timeout_ms) {
                Err(Error::Model("native generation timeout".into()))
            } else {
                Ok(())
            }
        };
        for chunk in prompt.chunks(128) {
            check()?;
            let input = Tensor::new(chunk, &self.device)?.unsqueeze(0)?;
            logits = Some(self.forward_cached(&input, &mut cache, scope)?);
        }
        let mut logits = logits.expect("nonempty prompt");
        let mut tokens = Vec::new();
        let mut first_token_ms = None;
        let mut finish = "length";
        let mut generated = 0;
        for n in 0..max_new {
            check()?;
            let len = logits.dim(1)?;
            let row = logits.narrow(1, len - 1, 1)?.flatten_all()?;
            if row.to_vec1::<f32>()?.iter().any(|x| !x.is_finite()) {
                return Err(Error::Model("nonfinite logits".into()));
            }
            let id = row.argmax(0)?.to_scalar::<u32>()?;
            observe(id);
            #[cfg(feature = "test-support")]
            observed_tokens.set(observed_tokens.get() + 1);
            generated += 1;
            first_token_ms.get_or_insert(start.elapsed().as_millis() as u64);
            if id == EOS {
                finish = "stop";
                break;
            }
            if id < SPECIALS as u32 {
                return Err(Error::Model("model generated a control token".into()));
            }
            tokens.push(id);
            if n + 1 < max_new {
                logits = self.forward_cached(
                    &Tensor::new(&[id], &self.device)?.unsqueeze(0)?,
                    &mut cache,
                    scope,
                )?;
            }
        }
        Ok(Generated {
            tokens,
            generated,
            finish: finish.into(),
            first_token_ms: first_token_ms.unwrap_or(0),
            generation_ms: start.elapsed().as_millis() as u64,
            cache_bytes: cache.bytes(),
            retained: cache.retained_tokens(),
            attention_workspace_bytes: cache.max_attention_bytes,
        })
    }
}
#[derive(Debug, Serialize)]
pub struct Generated {
    pub tokens: Vec<u32>,
    pub generated: usize,
    pub finish: String,
    pub first_token_ms: u64,
    pub generation_ms: u64,
    pub cache_bytes: usize,
    pub retained: Vec<usize>,
    pub attention_workspace_bytes: usize,
}
pub fn masked_loss(logits: &Tensor, targets: &Tensor, mask: &Tensor) -> Result<(Tensor, usize)> {
    let (b, t, v) = logits.dims3()?;
    if targets.dims() != [b, t]
        || mask.dims() != [b, t]
        || targets.dtype() != DType::U32
        || mask.dtype() != DType::F32
    {
        return Err(Error::Invalid("loss shape/dtype".into()));
    }
    let values = mask.flatten_all()?.to_vec1::<f32>()?;
    if values.iter().any(|&m| m != 0. && m != 1.) {
        return Err(Error::Invalid("loss mask must be binary".into()));
    }
    let count = values.iter().filter(|&&m| m == 1.).count();
    if count == 0 {
        return Err(Error::Invalid("no target tokens".into()));
    }
    let log_probs = candle_nn::ops::log_softmax(&logits.reshape((b * t, v))?, 1)?;
    let selected = log_probs
        .gather(&targets.reshape((b * t, 1))?, 1)?
        .reshape((b, t))?;
    Ok((((selected * mask)?.sum_all()? / -(count as f64))?, count))
}

#[cfg(test)]
mod adapter_tests {
    use super::*;
    #[test]
    fn adapter_scalar_gradient_and_frozen_registry()->Result<()> {
        let mut m=Transformer::init(Config::tiny(264),71,Device::Cpu)?;
        let before=m.weights_content_id()?;m.initialize_adapter(QvAdapter::registered(20260924))?;
        assert_eq!(m.base_content_id()?,before);
        assert!(m.frozen.as_ref().unwrap().values().all(|t|!t.is_variable()));
        assert_eq!(m.vars.len(),8);
        assert_eq!(QvAdapter::registered(1).shapes(&Config::small(562))?.values().map(|s|s.iter().product::<usize>()).sum::<usize>(),59904);
        for rows in [1usize,5] {for name in ["layer.0.q","layer.0.v"] {
            let input:Vec<f32>=(0..rows*32).map(|n|(n as f32-25.)/100.).collect();
            let x=Tensor::from_vec(input.clone(),(1,rows,32),&Device::Cpu)?;
            let a=format!("adapter.{name}.a");let b=format!("adapter.{name}.b");let out=m.vars[&b].dim(0)?;
            m.vars[&b].set(&Tensor::zeros((out,8),DType::F32,&Device::Cpu)?)?;
            let y=m.projection(&x,name,Kernel::Reference,&mut None)?;let base=linear(&x,m.weight(name))?;
            assert_eq!(y.flatten_all()?.to_vec1::<f32>()?,base.flatten_all()?.to_vec1::<f32>()?);
            let gradients=y.sum_all()?.backward()?;let ga=gradients.get(&m.vars[&a]).unwrap().flatten_all()?.to_vec1::<f32>()?;
            assert!(ga.iter().all(|&x|x==0.));let gb=gradients.get(&m.vars[&b]).unwrap().to_vec2::<f32>()?;
            let av=m.vars[&a].to_vec2::<f32>()?;
            for row in &gb{for k in 0..8{let expected=(0..rows).map(|n|(0..32).map(|i|input[n*32+i]as f64*av[k][i]as f64).sum::<f64>()).sum::<f64>();assert!((row[k]as f64-expected).abs()<2e-5);}}
            assert!(gb.iter().flatten().any(|&v|v.abs()>1e-5));
            let bv:Vec<f32>=(0..out*8).map(|i|((i%7)as f32-3.)/100.).collect();m.vars[&b].set(&Tensor::from_vec(bv.clone(),(out,8),&Device::Cpu)?)?;
            let y=m.projection(&x,name,Kernel::Reference,&mut None)?;let result=y.flatten_all()?.to_vec1::<f32>()?;let base=base.flatten_all()?.to_vec1::<f32>()?;
            for n in 0..rows {for o in 0..out {let delta=(0..8).map(|k|bv[o*8+k]as f64*(0..32).map(|i|input[n*32+i]as f64*av[k][i]as f64).sum::<f64>()).sum::<f64>();assert!((result[n*out+o]as f64-base[n*out+o]as f64-delta).abs()<2e-5);}}
            let g=y.sum_all()?.backward()?;let ga=g.get(&m.vars[&a]).unwrap().to_vec2::<f32>()?;
            for k in 0..8{for i in 0..32{let expected=(0..out).map(|o|bv[o*8+k]as f64).sum::<f64>()*(0..rows).map(|n|input[n*32+i]as f64).sum::<f64>();assert!((ga[k][i]as f64-expected).abs()<2e-5);}}
            assert!(ga.iter().flatten().any(|&v|v.abs()>1e-5));
        }}
        assert_eq!(m.base_content_id()?,before);println!("ADAPTER scalar q/v batch1,5 forward/gradient PASS; synthetic_backward8 native_generation0 optimizer0");Ok(())
    }
    #[test]
    fn adapter_prefill_cached_identity_and_zero_parity()->Result<()> {
        let mut m=Transformer::init(Config::tiny(264),72,Device::Cpu)?;let ids=Tensor::from_vec(vec![10u32,11,12,13,14,15],(1,6),&Device::Cpu)?;
        let plain=m.forward(&ids,None)?.flatten_all()?.to_vec1::<f32>()?;let mut stale=m.cache("scope");
        m.initialize_adapter(QvAdapter::registered(20260924))?;
        assert!(m.forward_cached(&ids,&mut stale,"scope").is_err());
        assert_eq!(plain,m.forward(&ids,None)?.flatten_all()?.to_vec1::<f32>()?);
        for(n,v)in &m.vars {if n.ends_with(".b"){v.set(&Tensor::full(0.005f32,v.shape(),&m.device)?)?;}}
        m.refresh_identity()?;let full=m.forward(&ids,None)?;let mut cache=m.cache("scope");
        for n in 0..6 {let cached=m.forward_cached(&ids.narrow(1,n,1)?,&mut cache,"scope")?;
            for(a,b)in cached.flatten_all()?.to_vec1::<f32>()?.iter().zip(full.narrow(1,n,1)?.flatten_all()?.to_vec1::<f32>()?){assert!((*a-b).abs()<2e-5);}}
        assert!(m.install_adapter(QvAdapter::registered(2),BTreeMap::new()).is_err());
        let mut bad=QvAdapter::registered(1);bad.rank=4;assert!(bad.shapes(&m.config).is_err());
        println!("ADAPTER native full forwards3 cached forwards6; normal generation0 optimizer0; stale cache rejected");Ok(())
    }
}
