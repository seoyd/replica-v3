//! Explicit offline training tool; never imported by the product inference library.
use crate::data::{self, Episode};
use candle_core::{DType, Device, Tensor, Var};
use replica_v3::{
    Error, Result,
    app::citations,
    neural::{
        self, BOS, ByteBpe, EOS, PAD,
        checkpoint::{self, TrainConfig, TrainingState},
        transformer::{Rng, Transformer, masked_loss},
    },
};
use std::{
    collections::BTreeMap,
    path::Path,
    sync::atomic::{AtomicBool, Ordering},
    time::Instant,
};

pub struct Adam {
    pub moments: BTreeMap<String, Tensor>,
}
impl Adam {
    pub fn new(vars: &BTreeMap<String, Var>) -> Result<Self> {
        let mut moments = BTreeMap::new();
        for (name, var) in vars {
            moments.insert(
                format!("adam.m.{name}"),
                Tensor::zeros(var.dims(), DType::F32, var.device())?,
            );
            moments.insert(
                format!("adam.v.{name}"),
                Tensor::zeros(var.dims(), DType::F32, var.device())?,
            );
        }
        Ok(Self { moments })
    }
    pub fn step(
        &mut self,
        vars: &BTreeMap<String, Var>,
        grads: &BTreeMap<String, Tensor>,
        config: &TrainConfig,
        step: usize,
    ) -> Result<(f64, f64)> {
        let mut norm2 = 0f64;
        for name in vars.keys() {
            let grad = grads
                .get(name)
                .ok_or_else(|| Error::Model(format!("disconnected trainable variable {name}")))?;
            let square = grad.sqr()?.sum_all()?.to_scalar::<f32>()? as f64;
            if !square.is_finite() {
                return Err(Error::Model("nonfinite gradient".into()));
            }
            norm2 += square;
        }
        let norm = norm2.sqrt();
        let clip = (config.clip / (norm + 1e-12)).min(1.);
        let lr = config.learning_rate(step);
        let mut updates = Vec::new();
        let mut delta2 = 0f64;
        for (name, var) in vars {
            let g = (&grads[name] * clip)?;
            let m = ((&self.moments[&format!("adam.m.{name}")] * config.beta1)?
                + (&g * (1. - config.beta1))?)?;
            let v = ((&self.moments[&format!("adam.v.{name}")] * config.beta2)?
                + (g.sqr()? * (1. - config.beta2))?)?;
            let corrected_m = (&m / (1. - config.beta1.powi(step as i32)))?;
            let corrected_v = (&v / (1. - config.beta2.powi(step as i32)))?;
            let update = (corrected_m / (corrected_v.sqrt()? + config.eps)?)?;
            let old = var.as_detached_tensor();
            let next = ((&old * (1. - lr * config.weight_decay))? - (update * lr)?)?;
            let delta = (&next - &old)?.sqr()?.sum_all()?.to_scalar::<f32>()? as f64;
            if !delta.is_finite() {
                return Err(Error::Model("nonfinite optimizer update".into()));
            }
            delta2 += delta;
            updates.push((name.clone(), next.detach(), m.detach(), v.detach()));
        }
        // Validate/allocate the complete step before touching any master weight.
        for (name, next, m, v) in updates {
            vars[&name].set(&next)?;
            self.moments.insert(format!("adam.m.{name}"), m);
            self.moments.insert(format!("adam.v.{name}"), v);
        }
        Ok((norm, delta2.sqrt()))
    }
}
#[derive(Clone)]
pub struct Sample {
    pub tokens: Vec<u32>,
    pub response_start: usize,
}
pub fn samples(episodes: &[Episode], tok: &ByteBpe, seq_len: usize) -> Result<Vec<Sample>> {
    let mut out = Vec::new();
    for e in episodes {
        if e.answer.is_empty() {
            let mut tokens = vec![BOS];
            tokens.extend(tok.encode(e.request.input.as_bytes())?);
            tokens.push(EOS);
            // Episode split precedes chunking. One boundary token is context only.
            for start in (0..tokens.len() - 1).step_by(seq_len) {
                let end = (start + seq_len + 1).min(tokens.len());
                out.push(Sample {
                    tokens: tokens[start..end].to_vec(),
                    response_start: 1,
                });
            }
        } else {
            let answer = tok.encode(e.answer.as_bytes())?;
            let mut request = e.request.clone();
            request.limits.context_tokens = seq_len as u32;
            request.limits.max_tokens = (answer.len() + 1) as u32;
            let prompt = tok.prepare(&request, seq_len as u32, "training")?;
            if citations(&e.answer)?
                .iter()
                .any(|id| !prompt.provided.contains(id))
            {
                return Err(Error::Invalid(format!(
                    "training evidence excluded for {} at length {seq_len}",
                    e.id
                )));
            }
            let response_start = prompt.token_ids.len();
            let mut tokens = prompt.token_ids;
            tokens.extend(answer);
            tokens.push(EOS);
            out.push(Sample {
                tokens,
                response_start,
            });
        }
    }
    if out.is_empty() {
        return Err(Error::Invalid("empty training samples".into()));
    }
    Ok(out)
}
fn numeric_samples(tok: &ByteBpe) -> Result<Vec<Sample>> {
    ["가 나 다 가 나 다", "오른쪽 왼쪽 오른쪽 왼쪽"]
        .iter()
        .map(|s| {
            let mut tokens = vec![BOS];
            tokens.extend(tok.encode(s.as_bytes())?);
            tokens.push(EOS);
            Ok(Sample {
                tokens,
                response_start: 1,
            })
        })
        .collect()
}
struct Batch {
    input: Tensor,
    target: Tensor,
    mask: Tensor,
    valid: Vec<bool>,
    tokens: usize,
}
fn batch(samples: &[Sample], indices: &[usize], device: &Device) -> Result<Batch> {
    let len = indices
        .iter()
        .map(|&i| samples[i].tokens.len() - 1)
        .max()
        .ok_or_else(|| Error::Invalid("empty batch".into()))?;
    let mut input = vec![PAD; indices.len() * len];
    let mut target = input.clone();
    let mut mask = vec![0f32; input.len()];
    let mut valid = vec![false; input.len()];
    let mut tokens = 0;
    for (row, &index) in indices.iter().enumerate() {
        let sample = &samples[index];
        if sample.tokens.len() < 2 || sample.response_start >= sample.tokens.len() {
            return Err(Error::Invalid("sample/teacher forcing boundary".into()));
        }
        for p in 0..sample.tokens.len() - 1 {
            let i = row * len + p;
            input[i] = sample.tokens[p];
            target[i] = sample.tokens[p + 1];
            valid[i] = true;
            mask[i] = f32::from((p + 1 >= sample.response_start) as u8);
            tokens += 1;
        }
    }
    Ok(Batch {
        input: Tensor::from_vec(input, (indices.len(), len), device)?,
        target: Tensor::from_vec(target, (indices.len(), len), device)?,
        mask: Tensor::from_vec(mask, (indices.len(), len), device)?,
        valid,
        tokens,
    })
}
pub fn validation_loss(model: &Transformer, samples: &[Sample]) -> Result<f64> {
    let mut total = 0f64;
    let mut targets = 0;
    for index in 0..samples.len() {
        let b = batch(samples, &[index], &model.device)?;
        let (loss, n) = masked_loss(
            &model.forward(&b.input, Some(&b.valid))?,
            &b.target,
            &b.mask,
        )?;
        total += f64::from(loss.to_scalar::<f32>()?) * n as f64;
        targets += n;
    }
    let loss = total / targets as f64;
    if !loss.is_finite() {
        return Err(Error::Model("nonfinite validation loss".into()));
    }
    Ok(loss)
}
fn rss_kib() -> Result<u64> {
    let out = std::process::Command::new("ps")
        .args(["-o", "rss=", "-p", &std::process::id().to_string()])
        .output()?;
    if !out.status.success() {
        return Err(Error::Invalid("RSS observation failed".into()));
    }
    String::from_utf8_lossy(&out.stdout)
        .trim()
        .parse()
        .map_err(|_| Error::Invalid("RSS parse".into()))
}
pub struct Run<'a> {
    pub checkpoint: &'a Path,
    pub corpus: Option<&'a Path>,
    pub output: &'a Path,
    pub resume: bool,
    pub numeric_probe: bool,
    pub config: TrainConfig,
    pub stop_after: Option<usize>,
}
pub fn train(run: Run<'_>, cancel: &AtomicBool) -> Result<()> {
    let started = Instant::now();
    let mut loaded = checkpoint::load(run.checkpoint, Device::Cpu, run.resume)?;
    let config = if run.resume {
        loaded
            .manifest
            .training
            .as_ref()
            .ok_or_else(|| Error::Invalid("resume requires optimizer state".into()))?
            .config
            .clone()
    } else {
        run.config
    };
    config.validate(loaded.model.config.context)?;
    let (train, validation, corpus_hash, validation_hash) = if run.numeric_probe {
        let data = numeric_samples(&loaded.tokenizer)?;
        (
            data.clone(),
            data,
            loaded.tokenizer.train_hash.clone(),
            neural::hash(b"NUMERIC_OVERFIT_NOT_HELDOUT"),
        )
    } else {
        let (manifest, train, validation) = data::load(
            run.corpus
                .ok_or_else(|| Error::Invalid("explicit --corpus required".into()))?,
        )?;
        if manifest.train.sha256 != loaded.tokenizer.train_hash {
            return Err(Error::Corrupt("tokenizer/corpus mismatch".into()));
        }
        (
            samples(&train, &loaded.tokenizer, config.seq_len)?,
            samples(&validation, &loaded.tokenizer, loaded.model.config.context)?,
            manifest.train.sha256,
            manifest.validation.sha256,
        )
    };
    if train.iter().any(|s| s.tokens.len() > config.seq_len + 1) {
        return Err(Error::Invalid("training sample context".into()));
    }
    let mut state = if run.resume {
        loaded.manifest.training.clone().expect("checked")
    } else {
        TrainingState {
            config: config.clone(),
            step: 0,
            consumed_tokens: 0,
            target_tokens: 0,
            sampler_state: config.seed,
            corpus_hash: corpus_hash.clone(),
            validation_hash: validation_hash.clone(),
            initial_weight_hash: loaded.manifest.initial_weight_hash.clone(),
            train_loss: None,
            validation_loss: None,
        }
    };
    if state.corpus_hash != corpus_hash || state.validation_hash != validation_hash {
        return Err(Error::Corrupt("resume corpus/split mismatch".into()));
    }
    let mut adam = if run.resume {
        Adam {
            moments: loaded.optimizer,
        }
    } else {
        Adam::new(&loaded.model.vars)?
    };
    if !run.resume && loaded.manifest.training.is_some() {
        return Err(Error::Invalid(
            "trained checkpoint requires --resume".into(),
        ));
    }
    std::fs::create_dir(run.output)?;
    let mut peak = rss_kib()?;
    let mut rng = Rng::new(state.sampler_state);
    let initial_loss = validation_loss(&loaded.model, &validation)?;
    println!(
        "backend=CPU dtype=F32 profile={} parameters={} train_samples={} validation_samples={} initial_validation_loss={initial_loss:.8} numeric_overfit_only={} initial_weight_hash={} peak_sampled_rss_KiB={peak}",
        loaded.model.config.profile,
        loaded.model.config.parameters(),
        train.len(),
        validation.len(),
        run.numeric_probe,
        loaded.model.weight_hash()?
    );
    let mut best = state.validation_loss.unwrap_or(initial_loss);
    state.validation_loss = Some(initial_loss);
    let mut initial = loaded.manifest.clone();
    initial.training = Some(state.clone());
    initial.status = "TRAINING".into();
    checkpoint::save(
        &run.output.join("start"),
        &loaded.model,
        &loaded.tokenizer,
        initial,
        &adam.moments,
    )?;
    let mut last_saved = usize::MAX;
    let mut reason = "BUDGET_REACHED";
    while state.step < config.max_steps {
        if cancel.load(Ordering::Relaxed) {
            reason = "CANCELLED";
            break;
        }
        if run.stop_after.is_some_and(|n| state.step >= n) {
            reason = "TRAINING";
            break;
        }
        let sampler_before = rng.state;
        let mut gradients: BTreeMap<String, Tensor> = BTreeMap::new();
        let mut loss_sum = 0f64;
        let mut targets = 0usize;
        let mut step_tokens = 0;
        let mut aborted = false;
        for _ in 0..config.accumulation {
            let indices: Vec<_> = (0..config.microbatch)
                .map(|_| (rng.next_u64() % train.len() as u64) as usize)
                .collect();
            let b = batch(&train, &indices, &loaded.model.device)?;
            if state.consumed_tokens + b.tokens as u64 > config.max_tokens
                || cancel.load(Ordering::Relaxed)
            {
                aborted = true;
                break;
            }
            let (loss, n) = masked_loss(
                &loaded.model.forward(&b.input, Some(&b.valid))?,
                &b.target,
                &b.mask,
            )?;
            let value = f64::from(loss.to_scalar::<f32>()?);
            if !value.is_finite() {
                return Err(Error::Model(
                    "nonfinite training loss; previous checkpoint retained".into(),
                ));
            }
            let grads = loss.backward()?;
            state.consumed_tokens += b.tokens as u64;
            step_tokens += b.tokens;
            loss_sum += value * n as f64;
            targets += n;
            for (name, var) in &loaded.model.vars {
                let g = grads
                    .get(var)
                    .ok_or_else(|| Error::Model(format!("missing gradient {name}")))?;
                let weighted = (g * n as f64)?.detach();
                let accumulated = match gradients.remove(name) {
                    Some(old) => (old + weighted)?,
                    None => weighted,
                };
                gradients.insert(name.clone(), accumulated.detach());
            }
        }
        if aborted {
            rng.state = sampler_before;
            if cancel.load(Ordering::Relaxed) {
                reason = "CANCELLED";
            }
            break;
        }
        for gradient in gradients.values_mut() {
            *gradient = (&*gradient / targets as f64)?;
        }
        let (grad_norm, delta) =
            adam.step(&loaded.model.vars, &gradients, &config, state.step + 1)?;
        state.step += 1;
        state.target_tokens += targets as u64;
        state.sampler_state = rng.state;
        state.train_loss = Some(loss_sum / targets as f64);
        let rss = rss_kib()?;
        peak = peak.max(rss);
        println!(
            "step={} loss={:.8} grad_norm={grad_norm:.8} weight_delta_l2={delta:.8} lr={:.8} consumed_tokens={} target_tokens={} step_tokens={step_tokens} elapsed_s={:.3} rss_KiB={rss} peak_sampled_rss_KiB={peak}",
            state.step,
            state.train_loss.expect("observed"),
            config.learning_rate(state.step),
            state.consumed_tokens,
            state.target_tokens,
            started.elapsed().as_secs_f64()
        );
        if state.step == 1 {
            println!(
                "after_first_update_weight_hash={}",
                loaded.model.weight_hash()?
            );
        }
        if rss > 16 * 1024 * 1024 {
            reason = "RESOURCE_LIMIT";
            eprintln!("RESOURCE_LIMIT: sampled RSS exceeded 16 GiB");
            break;
        }
        if state.step.is_multiple_of(config.validate_every) || state.step == config.max_steps {
            let value = validation_loss(&loaded.model, &validation)?;
            state.validation_loss = Some(value);
            println!(
                "validation step={} loss={value:.8} samples={} selection=VALIDATION_ONLY",
                state.step,
                validation.len()
            );
            let improved = value < best;
            best = best.min(value);
            loaded.model.refresh_identity()?;
            let mut manifest = loaded.manifest.clone();
            manifest.training = Some(state.clone());
            manifest.status = "TRAINING".into();
            let path = run.output.join(format!("step-{:06}", state.step));
            let m = checkpoint::save(
                &path,
                &loaded.model,
                &loaded.tokenizer,
                manifest,
                &adam.moments,
            )?;
            println!(
                "checkpoint={} sha256={} best_validation={improved}",
                path.display(),
                m.weights_sha256
            );
            last_saved = state.step;
            if improved {
                let record = serde_json::json!({"checkpoint":path.file_name(),"validation_loss":value,"step":state.step});
                neural::write_new(
                    &run.output.join(format!("best-{:06}.json", state.step)),
                    &serde_json::to_vec(&record)?,
                )?;
            }
        }
    }
    state.sampler_state = rng.state;
    if state.validation_loss.is_none() || last_saved != state.step {
        state.validation_loss = Some(validation_loss(&loaded.model, &validation)?);
    }
    loaded.model.refresh_identity()?;
    let mut manifest = loaded.manifest;
    manifest.training = Some(state.clone());
    manifest.status = reason.into();
    let path = run.output.join("final");
    let saved = checkpoint::save(
        &path,
        &loaded.model,
        &loaded.tokenizer,
        manifest,
        &adam.moments,
    )?;
    println!(
        "TRAIN_END reason={reason} steps={} consumed_tokens={} target_tokens={} train_loss={:?} validation_loss={:?} elapsed_s={:.3} peak_sampled_rss_KiB={peak} checkpoint={} sha256={} exact_resume=optimizer_boundary task_quality=NOT_EVALUATED",
        state.step,
        state.consumed_tokens,
        state.target_tokens,
        state.train_loss,
        state.validation_loss,
        started.elapsed().as_secs_f64(),
        path.display(),
        saved.weights_sha256
    );
    match reason {
        "CANCELLED" => Err(Error::Cancelled),
        "RESOURCE_LIMIT" => Err(Error::Model(
            "training RSS limit; boundary checkpoint saved".into(),
        )),
        _ => Ok(()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn adam_matches_independent_reference_and_teacher_forcing_masks() {
        let device = Device::Cpu;
        let mut vars = BTreeMap::new();
        vars.insert("x".into(), Var::new(&[1f32, -2.], &device).unwrap());
        let mut adam = Adam::new(&vars).unwrap();
        let config = TrainConfig {
            warmup: 0,
            max_steps: 10,
            clip: 100.,
            ..Default::default()
        };
        let mut grads = BTreeMap::new();
        grads.insert("x".into(), Tensor::new(&[0.5f32, -0.25], &device).unwrap());
        let lr = config.learning_rate(1);
        adam.step(&vars, &grads, &config, 1).unwrap();
        let actual = vars["x"].to_vec1::<f32>().unwrap();
        for (index, (w, g)) in [(1f64, 0.5f64), (-2., -0.25)].into_iter().enumerate() {
            let expected = w * (1. - lr * config.weight_decay) - lr * g / (g.abs() + config.eps);
            assert!((actual[index] as f64 - expected).abs() < 1e-6);
        }
        let samples = vec![
            Sample {
                tokens: vec![BOS, 8, 9, 10, EOS],
                response_start: 3,
            },
            Sample {
                tokens: vec![BOS, 11, EOS],
                response_start: 1,
            },
        ];
        let b = batch(&samples, &[0, 1], &device).unwrap();
        assert_eq!(
            b.input.to_vec2::<u32>().unwrap(),
            vec![vec![BOS, 8, 9, 10], vec![BOS, 11, PAD, PAD]]
        );
        assert_eq!(
            b.target.to_vec2::<u32>().unwrap(),
            vec![vec![8, 9, 10, EOS], vec![11, EOS, PAD, PAD]]
        );
        assert_eq!(
            b.mask.to_vec2::<f32>().unwrap(),
            vec![vec![0., 0., 1., 1.], vec![1., 1., 0., 0.]]
        );
        let logits = Tensor::zeros((2, 4, 264), DType::F32, &device).unwrap();
        let (loss, n) = masked_loss(&logits, &b.target, &b.mask).unwrap();
        assert_eq!(n, 4);
        assert!((loss.to_scalar::<f32>().unwrap() as f64 - 264f64.ln()).abs() < 1e-6);
        let mut clipped = Adam::new(&vars).unwrap();
        let config = TrainConfig { clip: 1., ..config };
        grads.insert("x".into(), Tensor::new(&[3f32, 4.], &device).unwrap());
        let (norm, _) = clipped.step(&vars, &grads, &config, 1).unwrap();
        assert!((norm - 5.).abs() < 1e-6);
        let moment = clipped.moments["adam.m.x"].to_vec1::<f32>().unwrap();
        assert!((moment[0] - 0.06).abs() < 1e-6 && (moment[1] - 0.08).abs() < 1e-6);
        let before = vars["x"].to_vec1::<f32>().unwrap();
        grads.insert("x".into(), Tensor::new(&[f32::NAN, 1.], &device).unwrap());
        assert!(clipped.step(&vars, &grads, &config, 2).is_err());
        assert_eq!(vars["x"].to_vec1::<f32>().unwrap(), before);
    }
}
