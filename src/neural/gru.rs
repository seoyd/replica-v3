//! Separate dense reset-after recurrent core; no attention, routing or external memory.
use super::{
    hash,
    transformer::{Config as TrConfig, Rng, linear, rms_norm},
};
use crate::{Error, Result};
use candle_core::{DType, Device, Tensor, Var};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

pub const EQUATION: &str = "FULL_GRU_RESET_AFTER_V1";
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    pub equation: String,
    pub vocab: usize,
    pub embedding: usize,
    pub hidden: usize,
    pub layers: usize,
    pub context: usize,
    pub eps: f64,
}
impl Config {
    pub fn matched(vocab: usize) -> Self {
        let target = TrConfig::small(vocab).parameters();
        let mut c = Self {
            equation: EQUATION.into(),
            vocab,
            embedding: 384,
            hidden: 128,
            layers: 4,
            context: 2048,
            eps: 1e-6,
        };
        let h = (128..=1024)
            .step_by(32)
            .min_by_key(|&h| {
                c.hidden = h;
                c.parameters().abs_diff(target)
            })
            .unwrap();
        c.hidden = h;
        c
    }
    pub fn validate(&self) -> Result<()> {
        if self.equation != EQUATION
            || !(264..=4096).contains(&self.vocab)
            || self.embedding == 0
            || self.embedding > 384
            || self.hidden == 0
            || self.hidden > 1024
            || self.layers == 0
            || self.layers > 4
            || self.context == 0
            || self.context > 2048
            || self.eps != 1e-6
        {
            return Err(Error::Invalid("GRU core descriptor".into()));
        }
        Ok(())
    }
    pub fn shapes(&self) -> BTreeMap<String, Vec<usize>> {
        let mut out = BTreeMap::from([
            ("embedding".into(), vec![self.vocab, self.embedding]),
            ("output".into(), vec![self.embedding, self.hidden]),
            ("final_norm".into(), vec![self.embedding]),
        ]);
        for i in 0..self.layers {
            let input = if i == 0 { self.embedding } else { self.hidden };
            // Three independent dense gate matrices packed by rows: r,z,n.
            out.insert(format!("layers.{i}.input"), vec![3 * self.hidden, input]);
            out.insert(
                format!("layers.{i}.recurrent"),
                vec![3 * self.hidden, self.hidden],
            );
            out.insert(format!("layers.{i}.bias"), vec![3 * self.hidden]);
            out.insert(
                format!("layers.{i}.candidate_recurrent_bias"),
                vec![self.hidden],
            );
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

pub struct Gru {
    pub config: Config,
    pub vars: BTreeMap<String, Var>,
    pub device: Device,
    identity: String,
    tokenizer: String,
}
pub struct State {
    hidden: Vec<Tensor>,
    positions: Vec<usize>,
    identity: String,
    scope: String,
}
#[derive(Serialize,Deserialize)]
#[serde(deny_unknown_fields)]
struct Snapshot {
    schema:u32, core:Config, identity:String, tokenizer:String, framing:[u8;32],
    scope:String, positions:Vec<usize>, hidden:Vec<Vec<f32>>,
}
impl State {
    pub fn bytes(&self) -> usize {
        self.hidden.iter().map(|v| v.elem_count() * 4).sum()
    }
    pub fn positions(&self) -> &[usize] {
        &self.positions
    }
}
impl Gru {
    pub fn snapshot(&self,state:&State,scope:&str)->Result<Vec<u8>> {
        if state.identity!=self.identity||state.scope!=scope||state.hidden.len()!=self.config.layers {
            return Err(Error::Invalid("GRU snapshot identity".into()));
        }
        self.device.synchronize()?;
        Ok(crate::binary::to_vec(&Snapshot {schema:1,core:self.config.clone(),identity:self.identity.clone(),
            tokenizer:self.tokenizer.clone(),framing:super::Framing::QuestionEvidence.digest(),scope:scope.into(),
            positions:state.positions.clone(),hidden:state.hidden.iter().map(|t|t.flatten_all()?.to_vec1::<f32>()).collect::<candle_core::Result<_>>()?})?)
    }
    pub fn restore_state(&self,bytes:&[u8],scope:&str)->Result<State> {
        let s:Snapshot=crate::binary::from_canonical_slice(bytes)?;
        let batch=s.positions.len();
        if s.schema!=1||s.core!=self.config||s.identity!=self.identity||s.tokenizer!=self.tokenizer
            ||s.framing!=super::Framing::QuestionEvidence.digest()||s.scope!=scope||!(1..=8).contains(&batch)
            ||s.positions.iter().any(|&p|p>self.config.context)||s.hidden.len()!=self.config.layers
            ||s.hidden.iter().any(|v|v.len()!=batch*self.config.hidden||v.iter().any(|x|!x.is_finite())) {
            return Err(Error::Invalid("GRU snapshot core/weight/tokenizer/scope/position".into()));
        }
        Ok(State {hidden:s.hidden.into_iter().map(|v|Tensor::from_vec(v,(batch,self.config.hidden),&self.device)).collect::<candle_core::Result<_>>()?,
            positions:s.positions,identity:s.identity,scope:s.scope})
    }
    pub fn generate_observed(&self,prompt:&[u32],max_new:usize,timeout_ms:u64,cancel:&std::sync::atomic::AtomicBool,scope:&str,observe:impl FnMut(u32))->Result<super::transformer::Generated>{
        let mut state=self.state(1,scope)?;
        let mut result=super::transformer::greedy_generate(prompt,max_new,timeout_ms,cancel,self.config.context,&self.device,observe,true,false,
            |ids|self.forward_state(&Tensor::new(ids,&self.device)?.unsqueeze(0)?,None,&mut state,scope))?;
        result.cache_bytes=state.bytes();result.retained=state.positions.clone();Ok(result)
    }
    pub fn init(config: Config, seed: u64, embedding: &Tensor, device: Device) -> Result<Self> {
        config.validate()?;
        if embedding.dims() != [config.vocab, config.embedding] || embedding.dtype() != DType::F32 {
            return Err(Error::Invalid("GRU common embedding shape/dtype".into()));
        }
        let mut tensors = BTreeMap::new();
        for (name, shape) in config.shapes() {
            let n: usize = shape.iter().product();
            let t = if name == "embedding" {
                // Copy the common initialization, never share a mutable Var with TR.
                embedding.to_device(&device)?.copy()?
            } else {
                let data = if name == "final_norm" {
                    vec![1.; n]
                } else if shape.len() == 1 {
                    vec![0.; n]
                } else {
                    let h = hash(format!("{EQUATION}:{seed}:{name}").as_bytes());
                    let mut rng = Rng::new(u64::from_str_radix(&h[..16], 16).unwrap());
                    // Packed gate matrices use each gate's fan-out, not packed 3H.
                    let fan_out = if name.starts_with("layers.") {
                        config.hidden
                    } else {
                        shape[0]
                    };
                    let bound = (6.0 / (shape[1] + fan_out) as f64).sqrt();
                    (0..n)
                        .map(|_| {
                            (((rng.next_u64() >> 11) as f64 / (1u64 << 53) as f64) * 2. - 1.) as f32
                                * bound as f32
                        })
                        .collect()
                };
                Tensor::from_vec(data, shape, &device)?
            };
            tensors.insert(name, t);
        }
        Self::from_tensors(config, tensors, device)
    }
    pub fn from_tensors(
        config: Config,
        tensors: BTreeMap<String, Tensor>,
        device: Device,
    ) -> Result<Self> {
        config.validate()?;
        if tensors.len() != config.shapes().len() {
            return Err(Error::Corrupt("GRU tensor count".into()));
        }
        let mut vars = BTreeMap::new();
        for (name, shape) in config.shapes() {
            let t = tensors
                .get(&name)
                .ok_or_else(|| Error::Corrupt(format!("GRU missing {name}")))?;
            if t.dims() != shape
                || t.dtype() != DType::F32
                || !t.device().same_device(&device)
                || t.flatten_all()?
                    .to_vec1::<f32>()?
                    .iter()
                    .any(|x| !x.is_finite())
            {
                return Err(Error::Corrupt(format!("GRU tensor {name}")));
            }
            vars.insert(name, Var::from_tensor(t)?);
        }
        let mut s = Self {
            config,
            vars,
            device,
            identity: String::new(),
            tokenizer: String::new(),
        };
        s.refresh_identity()?;
        Ok(s)
    }
    pub fn weights_content_id(&self) -> Result<String> {
        use sha2::{Digest, Sha256};
        let mut h = Sha256::new();
        h.update(crate::binary::to_vec(&self.config)?);
        for (n, v) in &self.vars {
            h.update(n.as_bytes());
            for x in v.flatten_all()?.to_vec1::<f32>()? {
                h.update(x.to_le_bytes());
            }
        }
        Ok(format!("{:x}", h.finalize()))
    }
    pub fn refresh_identity(&mut self) -> Result<()> {
        self.identity = hash(
            format!(
                "{}:{}:QE:reset-per-request:v1",
                self.weights_content_id()?,
                self.tokenizer
            )
            .as_bytes(),
        );
        Ok(())
    }
    pub fn bind_tokenizer(&mut self, id: &str) -> Result<()> {
        self.tokenizer = id.into();
        self.refresh_identity()
    }
    pub fn state(&self, batch: usize, scope: &str) -> Result<State> {
        if batch == 0 || batch > 8 {
            return Err(Error::Invalid("GRU batch bound".into()));
        }
        Ok(State {
            hidden: (0..self.config.layers)
                .map(|_| Tensor::zeros((batch, self.config.hidden), DType::F32, &self.device))
                .collect::<candle_core::Result<_>>()?,
            positions: vec![0; batch],
            identity: self.identity.clone(),
            scope: scope.into(),
        })
    }
    pub fn forward(&self, ids: &Tensor, padding: Option<&[bool]>) -> Result<Tensor> {
        let mut state = self.state(ids.dim(0)?, "train")?;
        self.forward_state(ids, padding, &mut state, "train")
    }
    pub fn forward_state(
        &self,
        ids: &Tensor,
        padding: Option<&[bool]>,
        state: &mut State,
        scope: &str,
    ) -> Result<Tensor> {
        let (b, t) = ids.dims2()?;
        if ids.dtype() != DType::U32
            || !ids.device().same_device(&self.device)
            || t == 0
            || b == 0
            || b > 8
            || state.identity != self.identity
            || state.scope != scope
            || state.hidden.len() != self.config.layers
            || state.positions.len() != b
            || state
                .positions
                .iter()
                .any(|p| p.checked_add(t).is_none_or(|n| n > self.config.context))
            || padding.is_some_and(|p| p.len() != b * t)
        {
            return Err(Error::Invalid(
                "GRU request/state/core/position binding".into(),
            ));
        }
        let valid: Vec<bool> = padding.map_or_else(|| vec![true; b * t], |v| v.to_vec());
        let masks: Vec<_> = (0..t)
            .map(|at| {
                Tensor::from_vec(
                    (0..b)
                        .map(|row| u8::from(valid[row * t + at]))
                        .collect::<Vec<_>>(),
                    (b, 1),
                    &self.device,
                )
            })
            .collect::<candle_core::Result<_>>()?;
        let mut x = self.vars["embedding"]
            .index_select(&ids.flatten_all()?, 0)?
            .reshape((b, t, self.config.embedding))?;
        let mut next = Vec::new();
        for i in 0..self.config.layers {
            let h = self.config.hidden;
            let projected = linear(&x, &self.vars[&format!("layers.{i}.input")])?
                .broadcast_add(&self.vars[&format!("layers.{i}.bias")])?;
            let recurrent = &self.vars[&format!("layers.{i}.recurrent")];
            let bias = &self.vars[&format!("layers.{i}.candidate_recurrent_bias")];
            let mut hidden = state.hidden[i].clone();
            let mut rows = Vec::with_capacity(t);
            for (at, mask) in masks.iter().enumerate() {
                let input = projected.narrow(1, at, 1)?.squeeze(1)?.contiguous()?;
                let rec = linear(&hidden, recurrent)?;
                let r =
                    candle_nn::ops::sigmoid(&input.narrow(1, 0, h)?.add(&rec.narrow(1, 0, h)?)?)?;
                let z =
                    candle_nn::ops::sigmoid(&input.narrow(1, h, h)?.add(&rec.narrow(1, h, h)?)?)?;
                let q = rec.narrow(1, 2 * h, h)?.broadcast_add(bias)?;
                let n = input.narrow(1, 2 * h, h)?.add(&(r * q)?)?.tanh()?;
                let proposed = ((&z * &hidden)? + ((1.0 - &z)? * n)?)?;
                hidden = mask.broadcast_as((b, h))?.where_cond(&proposed, &hidden)?;
                rows.push(hidden.clone());
            }
            x = Tensor::stack(&rows, 1)?;
            next.push(hidden);
        }
        let projected = linear(&x, &self.vars["output"])?;
        let normalized = rms_norm(&projected, &self.vars["final_norm"], self.config.eps)?;
        let logits = linear(&normalized, &self.vars["embedding"])?;
        state.hidden = next;
        for row in 0..b {
            state.positions[row] += valid[row * t..(row + 1) * t].iter().filter(|&&v| v).count();
        }
        Ok(logits)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn fixture(device: Device) -> Result<Gru> {
        let config = Config {
            equation: EQUATION.into(),
            vocab: 264,
            embedding: 3,
            hidden: 2,
            layers: 1,
            context: 64,
            eps: 1e-6,
        };
        let mut tensors = BTreeMap::new();
        for (name, shape) in config.shapes() {
            let n: usize = shape.iter().product();
            let data = (0..n)
                .map(|i| (((i * 7 + name.len() * 3) % 23) as f32 - 11.) / 29.)
                .collect::<Vec<_>>();
            tensors.insert(name, Tensor::from_vec(data, shape, &device)?);
        }
        Gru::from_tensors(config, tensors, device)
    }
    fn values(g: &Gru) -> Result<BTreeMap<String, Vec<f64>>> {
        g.vars
            .iter()
            .map(|(n, t)| {
                Ok((
                    n.clone(),
                    t.flatten_all()?
                        .to_vec1::<f32>()?
                        .into_iter()
                        .map(f64::from)
                        .collect(),
                ))
            })
            .collect()
    }
    fn oracle(w: &BTreeMap<String, Vec<f64>>, ids: &[u32], before: bool) -> Vec<f64> {
        let mut h = vec![0.; 2];
        let sigmoid = |x: f64| 1. / (1. + (-x).exp());
        for &id in ids {
            let x = &w["embedding"][id as usize * 3..id as usize * 3 + 3];
            let mut input = vec![0.; 6];
            let mut rec = vec![0.; 6];
            for row in 0..6 {
                input[row] = w["layers.0.bias"][row]
                    + (0..3)
                        .map(|j| w["layers.0.input"][row * 3 + j] * x[j])
                        .sum::<f64>();
                rec[row] = (0..2)
                    .map(|j| w["layers.0.recurrent"][row * 2 + j] * h[j])
                    .sum();
            }
            let r: Vec<_> = (0..2).map(|j| sigmoid(input[j] + rec[j])).collect();
            let z: Vec<_> = (0..2).map(|j| sigmoid(input[2 + j] + rec[2 + j])).collect();
            h = (0..2)
                .map(|j| {
                    let q = if before {
                        (0..2)
                            .map(|k| w["layers.0.recurrent"][(4 + j) * 2 + k] * r[k] * h[k])
                            .sum::<f64>()
                            + w["layers.0.candidate_recurrent_bias"][j]
                    } else {
                        r[j] * (rec[4 + j] + w["layers.0.candidate_recurrent_bias"][j])
                    };
                    z[j] * h[j] + (1. - z[j]) * (input[4 + j] + q).tanh()
                })
                .collect();
        }
        h
    }
    fn close(a: &[f64], b: &[f64], abs: f64, rel: f64) {
        assert_eq!(a.len(), b.len());
        for (x, y) in a.iter().zip(b) {
            assert!(
                x.is_finite() && y.is_finite() && (x - y).abs() <= abs + rel * x.abs(),
                "{x} != {y}"
            );
        }
    }
    #[test]
    fn reset_after_f64_oracle_and_gate_vjp() -> Result<()> {
        let g = fixture(Device::Cpu)?;
        let ids = [8, 9, 11];
        let w = values(&g)?;
        let input = Tensor::new(&ids, &g.device)?.unsqueeze(0)?;
        let mut state = g.state(1, "oracle")?;
        g.forward_state(&input, None, &mut state, "oracle")?;
        let expected = oracle(&w, &ids, false);
        let wrong = oracle(&w, &ids, true);
        assert!(
            expected
                .iter()
                .zip(wrong)
                .any(|(x, y)| (x - y).abs() > 1e-3),
            "fixture must distinguish reset-before"
        );
        close(
            &expected,
            &state.hidden[0]
                .flatten_all()?
                .to_vec1::<f32>()?
                .into_iter()
                .map(f64::from)
                .collect::<Vec<_>>(),
            2e-6,
            2e-5,
        );
        let vjp = Tensor::new(&[0.7f32, -0.3], &g.device)?.unsqueeze(0)?;
        let grad = state.hidden[0].mul(&vjp)?.sum_all()?.backward()?;
        let objective = |v: &BTreeMap<String, Vec<f64>>| {
            let h = oracle(v, &ids, false);
            h[0] * 0.7 - h[1] * 0.3
        };
        let mut checked = 0;
        for name in [
            "layers.0.input",
            "layers.0.recurrent",
            "layers.0.bias",
            "layers.0.candidate_recurrent_bias",
            "embedding",
        ] {
            let actual = grad
                .get(&g.vars[name])
                .expect("connected prompt/full BPTT")
                .flatten_all()?
                .to_vec1::<f32>()?;
            let indices: Vec<_> = if name == "embedding" {
                (24..30).collect()
            } else {
                (0..w[name].len()).collect()
            };
            for i in indices {
                let mut plus = w.clone();
                let mut minus = w.clone();
                plus.get_mut(name).unwrap()[i] += 1e-5;
                minus.get_mut(name).unwrap()[i] -= 1e-5;
                let numeric = (objective(&plus) - objective(&minus)) / 2e-5;
                close(&[numeric], &[actual[i] as f64], 2e-6, 3e-4);
                checked += 1;
            }
        }
        println!(
            "reset-after f64 forward + gate/input VJP entries={checked}; reset-before negative control distinguished"
        );
        Ok(())
    }
    #[test]
    fn sequence_step_padding_reset_and_identity() -> Result<()> {
        let g = fixture(Device::Cpu)?;
        let ids = Tensor::new(&[[8u32, 9, 11], [10, 12, 0]], &g.device)?;
        let valid = [true, true, true, true, true, false];
        let mut full = g.state(2, "request")?;
        let output = g.forward_state(&ids, Some(&valid), &mut full, "request")?;
        let mut step = g.state(2, "request")?;
        let mut rows = vec![];
        for t in 0..3 {
            rows.push(g.forward_state(
                &ids.narrow(1, t, 1)?.contiguous()?,
                Some(&[valid[t], valid[3 + t]]),
                &mut step,
                "request",
            )?);
        }
        let same = Tensor::cat(&rows, 1)?;
        close(
            &output
                .flatten_all()?
                .to_vec1::<f32>()?
                .into_iter()
                .map(f64::from)
                .collect::<Vec<_>>(),
            &same
                .flatten_all()?
                .to_vec1::<f32>()?
                .into_iter()
                .map(f64::from)
                .collect::<Vec<_>>(),
            1e-6,
            1e-5,
        );
        assert_eq!(full.positions(), [3, 2]);
        for (row, n) in [(0, 3), (1, 2)] {
            let mut single = g.state(1, "single")?;
            g.forward_state(
                &ids.narrow(0, row, 1)?.narrow(1, 0, n)?.contiguous()?,
                None,
                &mut single,
                "single",
            )?;
            close(
                &full.hidden[0]
                    .get(row)?
                    .to_vec1::<f32>()?
                    .into_iter()
                    .map(f64::from)
                    .collect::<Vec<_>>(),
                &single.hidden[0]
                    .flatten_all()?
                    .to_vec1::<f32>()?
                    .into_iter()
                    .map(f64::from)
                    .collect::<Vec<_>>(),
                1e-6,
                1e-5,
            );
        }
        assert!(g.forward_state(&ids, None, &mut full, "different").is_err());
        let mut other = fixture(Device::Cpu)?;
        other.bind_tokenizer("different-tokenizer")?;
        assert!(
            other
                .forward_state(&ids, None, &mut full, "request")
                .is_err()
        );
        assert_eq!(
            g.forward(&ids, Some(&valid))?.to_vec3::<f32>()?,
            output.to_vec3::<f32>()?
        );
        let config = Config::matched(562);
        assert_eq!(config.hidden, 640);
        assert_eq!(config.parameters(), 9_811_072);
        assert!(
            (config.parameters() as f64 / TrConfig::small(562).parameters() as f64 - 1.).abs()
                < 0.05
        );
        println!(
            "sequence/token step, padding freeze, batch isolation, reset, scope/tokenizer binding PASS; SMALL H640 params={}",
            config.parameters()
        );
        Ok(())
    }
    #[cfg(feature = "metal")]
    #[test]
    #[ignore = "actual Metal F32 forward/backward numeric validation; no optimizer"]
    fn metal_forward_backward() -> Result<()> {
        let cpu = fixture(Device::Cpu)?;
        let gpu = fixture(super::super::Backend::Metal0.open()?)?;
        let mut results = vec![];
        for g in [&cpu, &gpu] {
            let ids = Tensor::new(&[[8u32, 9, 11], [10, 12, 0]], &g.device)?;
            let out = g.forward(&ids, Some(&[true, true, true, true, true, false]))?;
            let grads = out.sqr()?.mean_all()?.backward()?;
            g.device.synchronize()?;
            let tensors = g
                .vars
                .iter()
                .map(|(n, v)| {
                    Ok((
                        n.clone(),
                        grads
                            .get(v)
                            .expect("full gradient")
                            .flatten_all()?
                            .to_vec1::<f32>()?,
                    ))
                })
                .collect::<Result<BTreeMap<_, _>>>()?;
            results.push((out.flatten_all()?.to_vec1::<f32>()?, tensors));
        }
        close(
            &results[0]
                .0
                .iter()
                .copied()
                .map(f64::from)
                .collect::<Vec<_>>(),
            &results[1]
                .0
                .iter()
                .copied()
                .map(f64::from)
                .collect::<Vec<_>>(),
            2e-4,
            2e-3,
        );
        for (name, a) in &results[0].1 {
            let b = &results[1].1[name];
            let norm = a.iter().map(|&v| (v as f64).powi(2)).sum::<f64>().sqrt();
            let normb = b.iter().map(|&v| (v as f64).powi(2)).sum::<f64>().sqrt();
            let err = a
                .iter()
                .zip(b)
                .map(|(&x, &y)| (x as f64 - y as f64).powi(2))
                .sum::<f64>()
                .sqrt();
            assert!(a.iter().chain(b).all(|v| v.is_finite()));
            let floor = 1e-6 * (a.len() as f64).sqrt();
            if norm > floor {
                assert!(err / norm <= 0.003, "{name} nrmse={}", err / norm);
                let cos = a
                    .iter()
                    .zip(b)
                    .map(|(&x, &y)| x as f64 * y as f64)
                    .sum::<f64>()
                    / (norm * normb);
                assert!(cos >= 0.999, "{name} cosine={cos}");
            } else {
                assert!(a.iter().zip(b).all(|(&x, &y)| (x - y).abs() <= 2e-5));
            }
            println!(
                "Metal gradient {name} nrmse={} norm={norm}",
                err / norm.max(floor)
            );
        }
        Ok(())
    }
    #[test]
    fn common_embedding_is_equal_but_independently_mutable()->Result<()> {
        let tr=super::super::transformer::Transformer::init(TrConfig::tiny(264),17,Device::Cpu)?;
        let config=Config{equation:EQUATION.into(),vocab:264,embedding:32,hidden:32,layers:2,context:64,eps:1e-6};
        let mut gr=Gru::init(config,17,&tr.vars["embedding"],Device::Cpu)?;
        let original=tr.vars["embedding"].flatten_all()?.to_vec1::<f32>()?;
        assert_eq!(original,gr.vars["embedding"].flatten_all()?.to_vec1::<f32>()?);
        let mut state=gr.state(1,"request")?;
        gr.forward_state(&Tensor::new(&[[8u32,9]],&Device::Cpu)?,None,&mut state,"request")?;
        let bytes=gr.snapshot(&state,"request")?;
        assert_eq!(gr.restore_state(&bytes,"request")?.positions(),[2]);
        assert!(gr.restore_state(&bytes[..bytes.len()-1],"request").is_err());
        gr.vars["embedding"].set(&Tensor::zeros((264,32),DType::F32,&Device::Cpu)?)?;
        gr.refresh_identity()?;
        assert_eq!(original,tr.vars["embedding"].flatten_all()?.to_vec1::<f32>()?);
        assert!(gr.restore_state(&bytes,"request").is_err());
        assert!(gr.forward_state(&Tensor::new(&[[8u32]],&Device::Cpu)?,None,&mut state,"request").is_err());
        Ok(())
    }
}
