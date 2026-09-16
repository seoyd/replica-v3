//! Explicit offline training tool; never imported by the product inference library.
#[path = "contrast.rs"]
pub mod contrast;
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
    pub curriculum: bool,
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
                    curriculum: false,
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
                curriculum: e.family.starts_with("copy/"),
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
                curriculum: false,
            })
        })
        .collect()
}
struct Batch {
    input: Tensor,
    target: Tensor,
    mask: Tensor,
    first_target_mask: Tensor,
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
    let mut first_target_mask = mask.clone();
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
            first_target_mask[i] = f32::from((p + 1 == sample.response_start.max(1)) as u8);
            tokens += 1;
        }
    }
    Ok(Batch {
        input: Tensor::from_vec(input, (indices.len(), len), device)?,
        target: Tensor::from_vec(target, (indices.len(), len), device)?,
        mask: Tensor::from_vec(mask, (indices.len(), len), device)?,
        first_target_mask: Tensor::from_vec(first_target_mask, (indices.len(), len), device)?,
        valid,
        tokens,
    })
}
// Both losses use the real number of target tokens. The optional objective adds
// (weight-1)*NLL only at each sample's first supervised position; it never weights
// padding/prompt tokens or conditions on a token value, question, seed or answer.
fn response_loss(logits: &Tensor, batch: &Batch, weight: f64) -> Result<(Tensor, Tensor, usize)> {
    if !weight.is_finite() || !(1.0..=16.0).contains(&weight) {
        return Err(Error::Invalid("first target weight 1..16".into()));
    }
    let (ce, n) = masked_loss(logits, &batch.target, &batch.mask)?;
    let objective = if weight == 1. {
        ce.clone()
    } else {
        let (first, count) = masked_loss(logits, &batch.target, &batch.first_target_mask)?;
        (&ce + (first * ((weight - 1.) * count as f64 / n as f64))?)?
    };
    Ok((ce, objective, n))
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
// An oracle paraphrase ablation of already explicit query targets. No expected
// answer, evidence value or citation ID is accepted by this diagnostic helper.
// It is private to the training executable, never part of a product request.
fn mentions_target(question: &str, target: &str) -> bool {
    !target.is_empty()
        && question.match_indices(target).any(|(start, matched)| {
            (!target.as_bytes()[0].is_ascii_digit()
                || start == 0
                || !question.as_bytes()[start - 1].is_ascii_digit())
                && question[start + matched.len()..]
                    .chars()
                    .next()
                    .is_none_or(|c| !c.is_ascii_digit())
        })
}
fn known_question_form(
    category: usize,
    original: &str,
    entity: &str,
    context: &str,
) -> Result<String> {
    if !mentions_target(original, entity) || !mentions_target(original, context) {
        return Err(Error::Invalid(
            "ablation target must be explicit in the original question".into(),
        ));
    }
    match category {
        0 => Ok(format!("{entity} {context} 현재 이동 지시는 무엇인가?")),
        2 => Ok(format!("{entity}는 {context}에서 어느 방향으로 가야 하나?")),
        _ => Err(Error::Invalid(
            "question-form ablation is limited to QA0/QA2".into(),
        )),
    }
}
// The field is an explicit oracle task label for this ablation. Gold answers and
// record contents are not arguments; binding identifiers must occur in the question.
fn known_field_question_form(
    original: &str,
    field: &str,
    entity: &str,
    context: &str,
) -> Result<String> {
    if field == "entity-cue" {
        return Ok("제공된 원문에서 숫자 앞에 적힌 대상의 분류명만 그대로 복사해줘.".into());
    }
    let number = entity.trim_start_matches(|c: char| !c.is_ascii_digit());
    let target = if field == "value" { number } else { entity };
    if !mentions_target(original, target) || !mentions_target(original, context) {
        return Err(Error::Invalid(
            "field ablation target must be explicit in the original question".into(),
        ));
    }
    match field {
        "number" => Ok(format!(
            "{context} 자료의 대상 {entity}에서 분류명 뒤 숫자만 원문대로 적어줘."
        )),
        "context" => Ok(format!(
            "제공 기록에서 {entity}의 {context} 위치 이름 전체만 복사하라."
        )),
        "value" => Ok(format!(
            "식별 숫자가 {number}인 대상의 {context} current 원문에서 '이동 지시는' 다음 방향 단어만 복사해줘."
        )),
        _ => Err(Error::Invalid("unsupported oracle field task".into())),
    }
}
pub enum FieldAblation {
    None,
    Question,
    Record,
    QuestionAndRecord,
}
// Oracle evidence-selection diagnostic, never retrieval or product inference.
// Selection reads only the explicit identifiers and current status, not gold values.
fn isolate_current_record(
    request: &mut replica_v3::model::ModelRequest,
    entity: &str,
    context: &str,
) -> Result<()> {
    let number = entity.trim_start_matches(|c: char| !c.is_ascii_digit());
    if !mentions_target(&request.input, number) || !mentions_target(&request.input, context) {
        return Err(Error::Invalid(
            "record ablation target must be explicit in the question".into(),
        ));
    }
    let matches: Vec<_> = request
        .evidence
        .items
        .iter()
        .filter(|e| {
            e.version_status == "current"
                && e.original_excerpt
                    .split_once("의 ")
                    .is_some_and(|(name, text)| {
                        name.trim_start_matches(|c: char| !c.is_ascii_digit()) == number
                            && text.starts_with(&format!("{context} 이동 지시는 "))
                    })
        })
        .cloned()
        .collect();
    if matches.len() != 1 {
        return Err(Error::Invalid(
            "record ablation requires exactly one current match".into(),
        ));
    }
    request.evidence.items = matches;
    Ok(())
}
/// Diagnostic on a predeclared corpus split, not the independent final test.
pub fn evaluate_corpus(
    checkpoint: &Path,
    corpus: &Path,
    output: &Path,
    limit: usize,
    split: &str,
    rephrase: bool,
    field_ablation: FieldAblation,
) -> Result<()> {
    use std::io::Write;
    let rephrase_field = matches!(
        field_ablation,
        FieldAblation::Question | FieldAblation::QuestionAndRecord
    );
    let single_record = matches!(
        field_ablation,
        FieldAblation::Record | FieldAblation::QuestionAndRecord
    );
    let (manifest, train, validation) = data::load(corpus)?;
    let (episodes, split_hash) = match split {
        "train" => (train, &manifest.train.sha256),
        "validation" => (validation, &manifest.validation.sha256),
        _ => return Err(Error::Invalid("diagnostic split".into())),
    };
    if ((rephrase || rephrase_field || single_record) && split != "validation")
        || (rephrase && (rephrase_field || single_record))
    {
        return Err(Error::Invalid(
            "question-form ablation requires validation".into(),
        ));
    }
    let episodes: Vec<_> = episodes
        .iter()
        .filter(|episode| {
            if single_record {
                episode.family.starts_with("copy/") && episode.family.ends_with("/value")
            } else if rephrase_field {
                episode.family.starts_with("copy/")
            } else {
                !rephrase
                    || (matches!(episode.category, 0 | 2) && !episode.family.starts_with("copy/"))
            }
        })
        .collect();
    if limit == 0 || limit > episodes.len() {
        return Err(Error::Invalid("diagnostic generation count".into()));
    }
    let loaded = checkpoint::load(checkpoint, Device::Cpu, false)?;
    if let Some(state) = &loaded.manifest.training
        && (state.corpus_hash != manifest.train.sha256
            || state.validation_hash != manifest.validation.sha256)
    {
        return Err(Error::Invalid("diagnostic corpus lineage mismatch".into()));
    }
    let mut log = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(output)?;
    writeln!(
        log,
        "{}",
        serde_json::json!({"header":true,"split":split,"checkpoint_sha256":loaded.manifest.weights_sha256,"split_sha256":split_hash,"limit":limit,"final_heldout":false,"oracle_question_ablation":rephrase || rephrase_field,"oracle_field_task_label":rephrase_field || single_record,"oracle_record_selection":single_record,"eligible_episodes":episodes.len()})
    )?;
    let mut exact = 0;
    let mut failed = 0;
    let mut groups: BTreeMap<String, [usize; 2]> = BTreeMap::new();
    for episode in episodes.iter().take(limit) {
        let mut request = episode.request.clone();
        if single_record {
            let mut fields = episode.binding.split('/');
            isolate_current_record(
                &mut request,
                fields.next().unwrap_or_default(),
                fields.next().unwrap_or_default(),
            )?;
        }
        if rephrase {
            let mut fields = episode.binding.split('/');
            request.input = known_question_form(
                episode.category,
                &request.input,
                fields.next().unwrap_or_default(),
                fields.next().unwrap_or_default(),
            )?;
        }
        if rephrase_field {
            let mut fields = episode.binding.split('/');
            request.input = known_field_question_form(
                &request.input,
                episode.family.rsplit('/').next().unwrap_or_default(),
                fields.next().unwrap_or_default(),
                fields.next().unwrap_or_default(),
            )?;
        }
        let prompt = loaded.tokenizer.prepare(
            &request,
            loaded.model.config.context as u32,
            &loaded.model.config.id()?,
        )?;
        let result = loaded
            .model
            .generate(
                &prompt.token_ids,
                episode.request.limits.max_tokens as usize,
                episode.request.limits.timeout_ms,
                &AtomicBool::new(false),
                &episode.id,
            )
            .and_then(|actual| Ok((loaded.tokenizer.decode(&actual.tokens)?, actual)));
        let (text, generated, error) = match result {
            Ok((text, generated)) => (Some(text), Some(generated), None),
            Err(e) => {
                failed += 1;
                (None, None, Some(e.to_string()))
            }
        };
        let matched = text.as_deref() == Some(episode.answer.as_str())
            && generated.as_ref().is_some_and(|g| g.finish == "stop");
        // Gold tokens enter only this teacher-forced diagnostic AFTER the actual
        // autoregressive result above. They never reach the generation request.
        let teacher_forced = (|| -> Result<serde_json::Value> {
            let mut sequence = prompt.token_ids.clone();
            sequence.extend(loaded.tokenizer.encode(episode.answer.as_bytes())?);
            sequence.push(EOS);
            if sequence.len() > loaded.model.config.context {
                return Err(Error::ContextTooSmall);
            }
            let start = prompt.token_ids.len();
            let targets = sequence.len() - start;
            let logits = loaded
                .model
                .forward(
                    &Tensor::new(&sequence[..sequence.len() - 1], &Device::Cpu)?.unsqueeze(0)?,
                    None,
                )?
                .narrow(1, start - 1, targets)?
                .squeeze(0)?;
            let ids = Tensor::new(&sequence[start..], &Device::Cpu)?.unsqueeze(1)?;
            let nll = candle_nn::ops::log_softmax(&logits, 1)?
                .gather(&ids, 1)?
                .neg()?
                .flatten_all()?
                .to_vec1::<f32>()?;
            if nll.iter().any(|n| !n.is_finite()) {
                return Err(Error::Model("nonfinite diagnostic NLL".into()));
            }
            let predicted = logits.argmax(1)?.to_vec1::<u32>()?;
            let correct = predicted
                .iter()
                .zip(&sequence[start..])
                .filter(|(a, b)| a == b)
                .count();
            let training_prompt_matches_generation = loaded
                .manifest
                .training
                .as_ref()
                .map(|state| {
                    let framed = samples(
                        std::slice::from_ref(*episode),
                        &loaded.tokenizer,
                        state.config.seq_len,
                    )?;
                    Ok::<_, Error>(framed[0].tokens[..framed[0].response_start] == prompt.token_ids)
                })
                .transpose()?;
            Ok(
                serde_json::json!({"first_target_nll":nll[0],"remaining_mean_nll":nll.iter().skip(1).map(|&v|f64::from(v)).sum::<f64>()/(targets-1).max(1) as f64,"target_tokens_including_eos":targets,"mean_nll":nll.iter().map(|&v|f64::from(v)).sum::<f64>()/targets as f64,"teacher_forced_correct_tokens":correct,"first_target_correct":predicted[0] == sequence[start],"training_prompt_matches_generation":training_prompt_matches_generation,"answer_tokenizer_roundtrip":loaded.tokenizer.decode(&sequence[start..sequence.len()-1])? == episode.answer}),
            )
        })();
        let teacher_forced = match teacher_forced {
            Ok(value) => value,
            Err(error) => serde_json::json!({"error":error.to_string()}),
        };
        exact += usize::from(matched);
        let group = if episode.family.starts_with("copy/") {
            "copy".to_string()
        } else {
            format!("qa-{}", episode.category)
        };
        let count = groups.entry(group).or_default();
        count[0] += usize::from(matched);
        count[1] += 1;
        writeln!(
            log,
            "{}",
            serde_json::json!({"id":episode.id,"category":episode.category,"family":episode.family,"question":episode.request.input,"generated_question":request.input,"evidence":episode.request.evidence,"generated_evidence":request.evidence,"expected":episode.answer,"actual":text,"generation":generated,"error":error,"provided":prompt.provided,"excluded":prompt.excluded,"exact_match":matched,"teacher_forced_diagnostic_after_generation":teacher_forced})
        )?;
        log.flush()?;
        println!(
            "{split} id={} exact={matched} actual={text:?} expected={:?} error={error:?}",
            episode.id, episode.answer
        );
    }
    let summary = serde_json::json!({"summary":true,"split":split,"exact_matches":exact,"denominator":limit,"generation_failures":failed,"groups_correct_total":groups,"final_heldout":false,"oracle_question_ablation":rephrase || rephrase_field,"oracle_field_task_label":rephrase_field || single_record,"oracle_record_selection":single_record});
    writeln!(log, "{summary}")?;
    log.sync_all()?;
    println!("{summary}");
    Ok(())
}
// Group size 1 preserves the original sampler exactly. Larger groups draw complete
// consecutive blocks from the eligible pool; they do not mix samples inside attention.
fn draw_indices(pool: &[usize], config: &TrainConfig, rng: &mut Rng) -> Result<Vec<usize>> {
    let group = config.sample_group_size;
    if group == 0
        || config.microbatch == 0
        || !config.microbatch.is_multiple_of(group)
        || pool.is_empty()
        || !pool.len().is_multiple_of(group)
    {
        return Err(Error::Invalid(
            "sample pool/microbatch must contain complete groups".into(),
        ));
    }
    let mut selected = Vec::with_capacity(config.microbatch);
    for _ in 0..config.microbatch / group {
        let start = (rng.next_u64() % (pool.len() / group) as u64) as usize * group;
        selected.extend_from_slice(&pool[start..start + group]);
    }
    Ok(selected)
}
pub fn sampling_exposure(start: &Path, end: &Path, corpus: &Path, limit: usize) -> Result<()> {
    let state = |path| -> Result<TrainingState> {
        checkpoint::load(path, Device::Cpu, false)?
            .manifest
            .training
            .ok_or_else(|| Error::Invalid("sampling audit requires training state".into()))
    };
    let start = state(start)?;
    let end = state(end)?;
    let (manifest, episodes, _) = data::load(corpus)?;
    if start.config != end.config
        || start.corpus_hash != end.corpus_hash
        || end.corpus_hash != manifest.train.sha256
        || start.validation_hash != end.validation_hash
        || end.validation_hash != manifest.validation.sha256
        || start.step > end.step
        || limit == 0
        || limit > episodes.len()
        || episodes.iter().any(|e| e.answer.is_empty())
    {
        return Err(Error::Invalid(
            "sampling audit requires one unchanged QA run and corpus".into(),
        ));
    }
    let copy: Vec<_> = episodes
        .iter()
        .enumerate()
        .filter_map(|(i, e)| e.family.starts_with("copy/").then_some(i))
        .collect();
    let mut counts = vec![0usize; episodes.len()];
    let all: Vec<_> = (0..episodes.len()).collect();
    let mut rng = Rng::new(start.sampler_state);
    for step in start.step..end.step {
        let pool = if step < end.config.curriculum_steps {
            &copy
        } else {
            &all
        };
        for _ in 0..end.config.accumulation {
            for index in draw_indices(pool, &end.config, &mut rng)? {
                counts[index] += 1;
            }
        }
    }
    if rng.state != end.sampler_state {
        return Err(Error::Corrupt(
            "replayed sampler does not match saved state".into(),
        ));
    }
    let histogram = |values: &[usize]| {
        let mut result = BTreeMap::new();
        for &n in values {
            *result.entry(n).or_insert(0usize) += 1;
        }
        result
    };
    println!(
        "{}",
        serde_json::to_string_pretty(&serde_json::json!({
            "method":"DETERMINISTIC_SAMPLER_REPLAY_NOT_PER_DRAW_LOG", "sampler_state_matches":true,
            "start_step":start.step,"end_step":end.step,"corpus_sha256":manifest.train.sha256,
            "episodes":episodes.len(),"draws":counts.iter().sum::<usize>(),
            "all_exposure_histogram":histogram(&counts),"prefix_exposure_histogram":histogram(&counts[..limit]),
            "prefix":episodes.iter().zip(&counts).take(limit).map(|(e,n)|serde_json::json!({"id":e.id,"draws":n})).collect::<Vec<_>>()
        }))?
    );
    Ok(())
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
    pub measure_rss: bool,
    pub extend_steps: Option<usize>,
    pub extend_microbatch: Option<usize>,
    pub extend_sample_group_size: Option<usize>,
    pub extend_curriculum_steps: Option<usize>,
    pub extend_first_target_weight: Option<f64>,
    pub source_id: Option<String>,
    pub replace_corpus: bool,
}
pub fn train(run: Run<'_>, cancel: &AtomicBool) -> Result<()> {
    if run.extend_steps.is_none()
        && (run.extend_microbatch.is_some()
            || run.extend_sample_group_size.is_some()
            || run.extend_curriculum_steps.is_some()
            || run.extend_first_target_weight.is_some())
    {
        return Err(Error::Invalid(
            "batch/curriculum/objective changes require explicit extension".into(),
        ));
    }
    if run.replace_corpus && (!run.resume || run.extend_steps.is_none() || run.numeric_probe) {
        return Err(Error::Invalid(
            "corpus change requires an explicit extension and real corpus".into(),
        ));
    }
    let started = Instant::now();
    let mut loaded = checkpoint::load(run.checkpoint, Device::Cpu, run.resume)?;
    let mut config = if run.resume {
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
    if let Some(additional) = run.extend_steps {
        let previous = loaded
            .manifest
            .training
            .as_ref()
            .filter(|_| run.resume)
            .ok_or_else(|| Error::Invalid("extension requires a resume checkpoint".into()))?;
        let source_id = run
            .source_id
            .as_ref()
            .filter(|s| s.len() == 64 && s.bytes().all(|b| b.is_ascii_hexdigit()))
            .ok_or_else(|| {
                Error::Invalid("extension requires current source manifest SHA-256".into())
            })?;
        if additional == 0 || additional > 5000 {
            return Err(Error::Invalid(
                "extension budget is 1..5000 additional steps".into(),
            ));
        }
        config.budget_start_step = previous.step;
        config.budget_start_tokens = previous.consumed_tokens;
        config.max_steps = previous
            .step
            .checked_add(additional)
            .ok_or_else(|| Error::Invalid("step budget overflow".into()))?;
        if let Some(microbatch) = run.extend_microbatch {
            config.microbatch = microbatch;
        }
        if let Some(group) = run.extend_sample_group_size {
            config.sample_group_size = group;
        }
        if let Some(weight) = run.extend_first_target_weight {
            config.first_target_weight = weight;
        }
        if let Some(steps) = run.extend_curriculum_steps {
            if steps > additional {
                return Err(Error::Invalid(
                    "copy curriculum exceeds extension budget".into(),
                ));
            }
            config.curriculum_steps = previous.step + steps;
        }
        config.max_tokens = previous
            .consumed_tokens
            .checked_add(20_000_000)
            .ok_or_else(|| Error::Invalid("token budget overflow".into()))?;
        loaded.manifest.source_id = source_id.clone();
        println!(
            "explicit_extension start_step={} start_tokens={} additional_steps={additional} additional_token_cap=20000000 microbatch={} sample_group_size={} copy_until_step={} first_target_weight={} schedule_restart=true optimizer_reset=false source_id={source_id}",
            previous.step,
            previous.consumed_tokens,
            config.microbatch,
            config.sample_group_size,
            config.curriculum_steps,
            config.first_target_weight
        );
    }
    config.validate(loaded.model.config.context)?;
    let estimated_bytes = config.microbatch
        * loaded.model.config.layers
        * loaded.model.config.heads
        * config.seq_len
        * config.seq_len
        * 4
        * 12
        + loaded.model.config.parameters() * 4 * 16;
    if estimated_bytes > 16usize * 1024 * 1024 * 1024 {
        return Err(Error::Invalid(
            "training tensor planning guard exceeds 16 GiB".into(),
        ));
    }
    println!("tensor_planning_guard_bytes={estimated_bytes} planning_guard_is_measurement=false");
    let (train, validation, corpus_hash, validation_hash, corpus_manifest) = if run.numeric_probe {
        let data = numeric_samples(&loaded.tokenizer)?;
        (
            data.clone(),
            data,
            loaded.tokenizer.train_hash.clone(),
            neural::hash(b"NUMERIC_OVERFIT_NOT_HELDOUT"),
            None,
        )
    } else {
        let (manifest, train, validation) = data::load(
            run.corpus
                .ok_or_else(|| Error::Invalid("explicit --corpus required".into()))?,
        )?;
        let resumed_corpus = run.resume
            && loaded
                .manifest
                .training
                .as_ref()
                .is_some_and(|s| s.corpus_hash == manifest.train.sha256);
        if manifest.train.sha256 != loaded.tokenizer.train_hash
            && !resumed_corpus
            && !run.replace_corpus
        {
            return Err(Error::Corrupt("tokenizer/corpus mismatch".into()));
        }
        (
            samples(&train, &loaded.tokenizer, config.seq_len)?,
            samples(&validation, &loaded.tokenizer, loaded.model.config.context)?,
            manifest.train.sha256.clone(),
            manifest.validation.sha256.clone(),
            Some(manifest),
        )
    };
    if train.iter().any(|s| s.tokens.len() > config.seq_len + 1) {
        return Err(Error::Invalid("training sample context".into()));
    }
    let copy_indices: Vec<_> = train
        .iter()
        .enumerate()
        .filter_map(|(i, s)| s.curriculum.then_some(i))
        .collect();
    let all_indices: Vec<_> = (0..train.len()).collect();
    let starting_step = if run.resume {
        loaded
            .manifest
            .training
            .as_ref()
            .expect("checked resume")
            .step
    } else {
        0
    };
    if starting_step < config.curriculum_steps && copy_indices.is_empty() {
        return Err(Error::Invalid(
            "curriculum requires explicit copy training episodes".into(),
        ));
    }
    if !train.len().is_multiple_of(config.sample_group_size)
        || (starting_step < config.curriculum_steps
            && !copy_indices.len().is_multiple_of(config.sample_group_size))
    {
        return Err(Error::Invalid(
            "training sample pool has an incomplete group".into(),
        ));
    }
    let mut state = if run.resume {
        let state = loaded.manifest.training.clone().expect("checked");
        if state.contrast16 {
            return Err(Error::Invalid(
                "contrast16 requires its fixed full-pass diagnostic trainer".into(),
            ));
        }
        state
    } else {
        TrainingState {
            contrast16: false,
            parent_checkpoint_hash: Some(loaded.manifest.weights_sha256.clone()),
            config: config.clone(),
            step: 0,
            consumed_tokens: 0,
            target_tokens: 0,
            sampler_state: config.seed,
            corpus_hash: corpus_hash.clone(),
            validation_hash: validation_hash.clone(),
            previous_corpora: Vec::new(),
            initial_weight_hash: loaded.manifest.initial_weight_hash.clone(),
            train_loss: None,
            validation_loss: None,
        }
    };
    state.config = config.clone();
    if run.replace_corpus {
        if state.corpus_hash != corpus_hash && !state.previous_corpora.contains(&state.corpus_hash)
        {
            state.previous_corpora.push(state.corpus_hash.clone());
        }
        if state.previous_corpora.len() > 32 {
            return Err(Error::Invalid("training corpus lineage limit".into()));
        }
        println!(
            "explicit_corpus_change previous={} current={} validation={} frozen_tokenizer_training_hash={}",
            state.corpus_hash, corpus_hash, validation_hash, loaded.tokenizer.train_hash
        );
        state.corpus_hash = corpus_hash.clone();
        state.validation_hash = validation_hash.clone();
        state.validation_loss = None;
        state.train_loss = None;
    }
    if state.corpus_hash != corpus_hash || state.validation_hash != validation_hash {
        return Err(Error::Corrupt("resume corpus/split mismatch".into()));
    }
    println!(
        "available_framed_train_tokens={} available_framed_validation_tokens={} frozen_tokenizer={} previous_corpora={:?}",
        train.iter().map(|s| s.tokens.len()).sum::<usize>(),
        validation.iter().map(|s| s.tokens.len()).sum::<usize>(),
        loaded.tokenizer.id(),
        state.previous_corpora
    );
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
    if let Some(mut manifest) = corpus_manifest {
        manifest.train.tokens = Some(train.iter().map(|s| s.tokens.len()).sum());
        manifest.validation.tokens = Some(validation.iter().map(|s| s.tokens.len()).sum());
        neural::write_new(
            &run.output.join("corpus-manifest.json"),
            &serde_json::to_vec_pretty(&serde_json::json!({
                "corpus":manifest,"source_directory":run.corpus,"tokenizer_sha256":loaded.tokenizer.id(),
                "tokenizer_training_hash":loaded.tokenizer.train_hash,"previous_corpora":state.previous_corpora,
                "token_count_kind":"available framed samples before sampling/repetition; LM overlap context included"
            }))?,
        )?;
    }
    let observe = || -> Result<Option<u64>> {
        if run.measure_rss {
            rss_kib().map(Some)
        } else {
            Ok(None)
        }
    };
    let mut peak = observe()?;
    let mut rng = Rng::new(state.sampler_state);
    let initial_loss = validation_loss(&loaded.model, &validation)?;
    println!(
        "backend={} dtype=F32 profile={} parameters={} train_samples={} validation_samples={} initial_validation_loss={initial_loss:.8} numeric_overfit_only={} loaded_weight_hash={} random_initial_weight_hash={} peak_sampled_rss_KiB={peak:?}",
        neural::cpu_backend(),
        loaded.model.config.profile,
        loaded.model.config.parameters(),
        train.len(),
        validation.len(),
        run.numeric_probe,
        loaded.model.weight_hash()?,
        loaded.manifest.initial_weight_hash
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
        let mut objective_sum = 0f64;
        let mut targets = 0usize;
        let mut step_tokens = 0;
        let mut aborted = false;
        for _ in 0..config.accumulation {
            let pool = if state.step < config.curriculum_steps {
                &copy_indices
            } else {
                &all_indices
            };
            let indices = draw_indices(pool, &config, &mut rng)?;
            let b = batch(&train, &indices, &loaded.model.device)?;
            if state
                .consumed_tokens
                .checked_add(b.tokens as u64)
                .is_none_or(|n| n > config.max_tokens)
                || cancel.load(Ordering::Relaxed)
            {
                aborted = true;
                break;
            }
            let (loss, objective, n) = response_loss(
                &loaded.model.forward(&b.input, Some(&b.valid))?,
                &b,
                config.first_target_weight,
            )?;
            let value = f64::from(loss.to_scalar::<f32>()?);
            let objective_value = f64::from(objective.to_scalar::<f32>()?);
            if !value.is_finite() || !objective_value.is_finite() {
                return Err(Error::Model(
                    "nonfinite training loss; previous checkpoint retained".into(),
                ));
            }
            let grads = objective.backward()?;
            state.consumed_tokens += b.tokens as u64;
            step_tokens += b.tokens;
            loss_sum += value * n as f64;
            objective_sum += objective_value * n as f64;
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
        let rss = observe()?;
        peak = peak.max(rss);
        println!(
            "step={} loss={:.8} objective={:.8} first_target_weight={} grad_norm={grad_norm:.8} weight_delta_l2={delta:.8} lr={:.8} consumed_tokens={} target_tokens={} step_tokens={step_tokens} elapsed_s={:.3} rss_KiB={rss:?} peak_sampled_rss_KiB={peak:?}",
            state.step,
            state.train_loss.expect("observed"),
            objective_sum / targets as f64,
            config.first_target_weight,
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
        if rss.is_some_and(|n| n > 16 * 1024 * 1024) {
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
        "TRAIN_END reason={reason} steps={} consumed_tokens={} target_tokens={} train_loss={:?} validation_loss={:?} elapsed_s={:.3} peak_sampled_rss_KiB={peak:?} checkpoint={} sha256={} exact_resume=optimizer_boundary task_quality=NOT_EVALUATED",
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
    fn masked_first_response_learns_context_and_generates_without_future_labels() {
        let device = Device::Cpu;
        let model =
            Transformer::init(neural::transformer::Config::tiny(264), 137, device.clone()).unwrap();
        // Every prompt ends identically. Its only varying token precedes the local
        // window; the first response must therefore learn to use earlier context.
        let samples: Vec<_> = (8..12)
            .map(|cue| {
                let mut tokens = vec![BOS, cue, 31, 32, 33, 34, 35, 36, 37, neural::ASSISTANT_ROLE];
                let response_start = tokens.len();
                tokens.push(cue);
                tokens.extend(40..51);
                tokens.push(EOS);
                Sample {
                    tokens,
                    response_start,
                    curriculum: false,
                }
            })
            .collect();
        let b = batch(&samples, &[0, 1, 2, 3], &device).unwrap();
        let first_loss = || {
            masked_loss(
                &model.forward(&b.input, Some(&b.valid)).unwrap(),
                &b.target,
                &b.first_target_mask,
            )
            .unwrap()
            .0
            .to_scalar::<f32>()
            .unwrap()
        };
        let initial = first_loss();
        let config = TrainConfig {
            lr: 0.005,
            warmup: 10,
            max_steps: 200,
            seq_len: 64,
            microbatch: 4,
            accumulation: 1,
            weight_decay: 0.,
            ..Default::default()
        };
        config.validate(64).unwrap();
        let mut adam = Adam::new(&model.vars).unwrap();
        for step in 1..=config.max_steps {
            let (_, objective, _) =
                response_loss(&model.forward(&b.input, Some(&b.valid)).unwrap(), &b, 1.).unwrap();
            let gradients = objective.backward().unwrap();
            let gradients = model
                .vars
                .iter()
                .map(|(name, var)| (name.clone(), gradients.get(var).unwrap().detach()))
                .collect();
            let (norm, delta) = adam.step(&model.vars, &gradients, &config, step).unwrap();
            assert!(norm.is_finite() && delta.is_finite());
            if step == 1 {
                assert!(norm > 0. && delta > 0.);
            }
        }
        let learned = first_loss();
        println!("TINY_NUMERIC_TEST_ONLY first_target_nll {initial}->{learned}; 200 real updates");
        assert!(learned < initial);
        for sample in &samples {
            // Autoregressive generation gets only the prompt, never its labels.
            let generated = model
                .generate(
                    &sample.tokens[..sample.response_start],
                    24,
                    30000,
                    &AtomicBool::new(false),
                    "first-response-regression",
                )
                .unwrap();
            assert_eq!(generated.tokens.first(), Some(&sample.tokens[1]));
        }
    }
    #[test]
    fn record_ablation_selects_explicit_current_target_without_gold() {
        use replica_v3::{
            model::ModelRequest,
            retrieval::{Evidence, EvidenceBundle},
        };
        let record = |id, name: &str, context: &str, value: &str, status: &str| Evidence {
            event_id: id,
            original_excerpt: format!("{name}의 {context} 이동 지시는 {value}이다."),
            excerpt_truncated: false,
            source: "fixture".into(),
            recorded_at: 100 - id,
            observed_at: None,
            version_status: status.into(),
            retrieval_reason: "lexical".into(),
            relation_path: vec![],
        };
        let request = ModelRequest {
            request_id: "record-ablation".into(),
            system: String::new(),
            input: "번호 531904인 대상의 구역8에 현재 적용되는 방향 값 한 단어만 답하라.".into(),
            limits: Default::default(),
            evidence: EvidenceBundle {
                items: vec![
                    record(71, "센서531904", "구역80", "왼쪽", "current"),
                    record(5, "센서531904", "구역8", "오른쪽", "superseded"),
                    record(99, "센서1531904", "구역8", "직진", "current"),
                    record(14, "센서531904", "구역8", "서쪽", "current"),
                ],
                ..Default::default()
            },
        };
        let mut isolated = request.clone();
        isolate_current_record(&mut isolated, "센서531904", "구역8").unwrap();
        assert_eq!(isolated.evidence.items.len(), 1);
        assert_eq!(
            serde_json::to_value(&isolated.evidence.items[0]).unwrap(),
            serde_json::to_value(&request.evidence.items[3]).unwrap()
        );
        assert_eq!(isolated.input, request.input);
        let mut changed = request.clone();
        changed.evidence.items[3].original_excerpt =
            "센서531904의 구역8 이동 지시는 대기이다.".into();
        isolate_current_record(&mut changed, "센서531904", "구역8").unwrap();
        assert_eq!(changed.evidence.items[0].event_id, 14);
        assert!(
            changed.evidence.items[0]
                .original_excerpt
                .ends_with("대기이다.")
        );
        for (entity, context) in [("센서53190", "구역8"), ("센서531904", "구역80")] {
            let mut invalid = request.clone();
            assert!(isolate_current_record(&mut invalid, entity, context).is_err());
            assert_eq!(invalid.evidence.items.len(), 4);
        }
        for duplicate in [false, true] {
            let mut invalid = request.clone();
            if duplicate {
                invalid
                    .evidence
                    .items
                    .push(invalid.evidence.items[3].clone());
            } else {
                invalid.evidence.items.pop();
            }
            let before = serde_json::to_value(&invalid).unwrap();
            assert!(isolate_current_record(&mut invalid, "센서531904", "구역8").is_err());
            assert_eq!(serde_json::to_value(&invalid).unwrap(), before);
        }
    }
    #[test]
    fn grouped_sampling_preserves_default_draws_and_complete_blocks() {
        let pool = [2, 5, 9, 12, 17, 21, 24, 28];
        let mut config = TrainConfig {
            microbatch: 8,
            ..Default::default()
        };
        let mut actual = Rng::new(97);
        let mut reference = Rng::new(97);
        for _ in 0..5 {
            let expected: Vec<_> = (0..8)
                .map(|_| pool[(reference.next_u64() % 8) as usize])
                .collect();
            assert_eq!(draw_indices(&pool, &config, &mut actual).unwrap(), expected);
            assert_eq!(actual.state, reference.state);
        }
        config.sample_group_size = 4;
        for _ in 0..5 {
            let selected = draw_indices(&pool, &config, &mut actual).unwrap();
            for block in selected.as_chunks::<4>().0 {
                let offset = (reference.next_u64() % 2) as usize * 4;
                assert_eq!(block, &pool[offset..offset + 4]);
            }
            assert_eq!(actual.state, reference.state);
        }
        let before = actual.state;
        for invalid_pool in [&pool[..7], &pool[..0]] {
            assert!(draw_indices(invalid_pool, &config, &mut actual).is_err());
            assert_eq!(actual.state, before);
        }
        for invalid_group in [0, 3, 16] {
            config.sample_group_size = invalid_group;
            assert!(config.validate(2048).is_err());
            assert!(draw_indices(&pool, &config, &mut actual).is_err());
            assert_eq!(actual.state, before);
        }
        let mut legacy = serde_json::to_value(TrainConfig::default()).unwrap();
        legacy.as_object_mut().unwrap().remove("sample_group_size");
        let decoded: TrainConfig = serde_json::from_value(legacy).unwrap();
        assert_eq!(decoded.sample_group_size, 1);
    }
    #[test]
    fn field_question_ablation_rejects_unmentioned_targets_and_never_reads_gold() {
        let q = "장비531904(구역8)의 식별 숫자 부분만 보여줘.";
        let actual = known_field_question_form(q, "number", "장비531904", "구역8").unwrap();
        assert!(actual.contains("장비531904") && actual.contains("구역8"));
        assert!(!actual.contains("event:") && !actual.contains("왼쪽"));
        assert!(known_field_question_form(q, "number", "장비53190", "구역8").is_err());
        assert!(known_field_question_form(q, "context", "장비531904", "구역80").is_err());
        let q = "번호 531904인 대상의 구역8에 현재 적용되는 방향 값 한 단어만 답하라.";
        assert!(known_field_question_form(q, "value", "장비531904", "구역8").is_ok());
        assert!(known_field_question_form(q, "value", "장비53190", "구역8").is_err());
        assert!(
            known_field_question_form(
                "번호 1531904인 대상의 구역8",
                "value",
                "장비531904",
                "구역8"
            )
            .is_err()
        );
        assert!(known_field_question_form(q, "value", "장비", "구역8").is_err());
        assert!(known_field_question_form(q, "unsupported", "531904", "구역8").is_err());
        assert!(
            known_field_question_form("대상의 종류 이름만 적어줘.", "entity-cue", "", "").is_ok()
        );
    }
    #[test]
    fn question_ablation_preserves_explicit_targets_without_reading_answer_fields() {
        let original = "자료에 현장9도 나오지만 구역8에 해당하는 장비531904의 방향을 읽어줘.";
        let a = known_question_form(2, original, "장비531904", "구역8").unwrap();
        assert!(a.contains("장비531904") && a.contains("구역8"));
        assert!(!a.contains("event:") && !a.contains("오른쪽"));
        assert!(known_question_form(2, original, "장비123", "구역8").is_err());
        assert!(known_question_form(2, original, "장비531904", "통로7").is_err());
        assert!(known_question_form(2, original, "장비53190", "구역8").is_err());
        assert!(known_question_form(2, original, "", "구역8").is_err());
        assert!(known_question_form(1, original, "장비531904", "구역8").is_err());
        let current = known_question_form(0, original, "장비531904", "구역8").unwrap();
        assert!(current.contains("장비531904") && current.contains("구역8"));
    }
    #[test]
    fn first_target_objective_matches_scalar_ce_and_gradients_without_mask_leakage() {
        let device = Device::Cpu;
        let samples = [
            Sample {
                tokens: vec![BOS, 8, 9, 10, EOS],
                response_start: 3,
                curriculum: false,
            },
            Sample {
                tokens: vec![BOS, 11, EOS],
                response_start: 1,
                curriculum: false,
            },
        ];
        let b = batch(&samples, &[0, 1], &device).unwrap();
        assert_eq!(
            b.first_target_mask.to_vec2::<f32>().unwrap(),
            [vec![0., 0., 1., 0.], vec![1., 0., 0., 0.]]
        );
        let values: Vec<f32> = (0..96).map(|i| (i % 19) as f32 * 0.1 - 0.9).collect();
        let logits = Var::from_vec(values.clone(), (2, 4, 12), &device).unwrap();
        let labels = b.target.flatten_all().unwrap().to_vec1::<u32>().unwrap();
        let masks = b.mask.flatten_all().unwrap().to_vec1::<f32>().unwrap();
        for weight in [1., 8.] {
            let (ce, objective, count) = response_loss(&logits, &b, weight).unwrap();
            assert_eq!(count, 4); // Denominator is actual supervised tokens, not weighted count or PAD.
            let gradients = objective.backward().unwrap();
            let actual = gradients
                .get(&logits)
                .unwrap()
                .flatten_all()
                .unwrap()
                .to_vec1::<f32>()
                .unwrap();
            let mut raw = 0.;
            let mut weighted = 0.;
            for (row, tokens) in values.as_chunks::<12>().0.iter().enumerate() {
                let sum: f64 = tokens.iter().map(|v| f64::from(*v).exp()).sum();
                let label = labels[row] as usize;
                let nll = sum.ln() - f64::from(tokens[label]);
                let w = if row == 2 || row == 4 { weight } else { 1. };
                raw += nll * f64::from(masks[row]) / 4.;
                weighted += nll * f64::from(masks[row]) * w / 4.;
                for (column, token) in tokens.iter().enumerate() {
                    let p = f64::from(*token).exp() / sum;
                    let expected =
                        (p - f64::from(column == label)) * f64::from(masks[row]) * w / 4.;
                    assert!((f64::from(actual[row * 12 + column]) - expected).abs() < 1e-6);
                }
            }
            assert!((f64::from(ce.to_scalar::<f32>().unwrap()) - raw).abs() < 1e-6);
            assert!((f64::from(objective.to_scalar::<f32>().unwrap()) - weighted).abs() < 2e-6);
        }
        for invalid in [0., 17., f64::NAN, f64::INFINITY] {
            assert!(response_loss(&logits, &b, invalid).is_err());
        }
        let mut legacy = serde_json::to_value(TrainConfig::default()).unwrap();
        legacy
            .as_object_mut()
            .unwrap()
            .remove("first_target_weight");
        let decoded: TrainConfig = serde_json::from_value(legacy).unwrap();
        assert_eq!(decoded.first_target_weight, 1.);
    }
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
                curriculum: false,
            },
            Sample {
                tokens: vec![BOS, 11, EOS],
                response_start: 1,
                curriculum: false,
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
