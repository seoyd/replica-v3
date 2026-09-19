//! Training-only fixed contrast diagnostic. Gold parsing never enters the model library.
use super::{Adam, Sample, batch, response_loss, samples};
use crate::data::{self, Episode};
use candle_core::{Device, Tensor};
use replica_v3::{
    Error, Result,
    model::ModelRequest,
    neural::{
        self, ByteBpe, EOS,
        checkpoint::{self, TrainConfig, TrainingState},
        hash, read_bounded,
        transformer::{Config, Rng, Transformer},
        write_new,
    },
};
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet},
    path::Path,
    sync::atomic::{AtomicBool, Ordering},
    time::Instant,
};

// Declared before observing new runs. Same F32 backend; no tolerance selected by outcome.
const LOGIT_TOLERANCE: f32 = 5e-4;
const MAX_UPDATES: usize = 1000;
const MAX_INPUT: u64 = 3_200_000;
const MAX_SECONDS: u64 = 45 * 60;
const HELDOUT_SEED: u64 = 917_031;

fn group_order(rng: &mut Rng) -> [usize; 4] {
    let mut order = [0usize, 1, 2, 3];
    for i in (1..order.len()).rev() {
        let j = (rng.next_u64() % (i as u64 + 1)) as usize;
        order.swap(i, j);
    }
    order
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Frozen {
    version: u32,
    corpus_hash: String,
    original_log_hash: String,
    tokenizer_hash: String,
    train_hash: String,
    heldout_hash: String,
    heldout_seed: u64,
    train: Vec<Episode>,
    heldout: Vec<Episode>,
}
fn ambiguity() -> Error {
    Error::Invalid(
        "DATA_AMBIGUITY: serialized question/evidence does not identify one supported answer"
            .into(),
    )
}
fn fields(text: &str) -> Result<(&str, &str, &str)> {
    let (entity, rest) = text.split_once("의 ").ok_or_else(ambiguity)?;
    let (context, value) = rest.split_once(" 이동 지시는 ").ok_or_else(ambiguity)?;
    let value = value
        .strip_suffix("이다.")
        .filter(|s| !s.is_empty())
        .ok_or_else(ambiguity)?;
    Ok((entity, context, value))
}
fn mentions(text: &str, term: &str) -> bool {
    !term.is_empty()
        && text.match_indices(term).any(|(at, _)| {
            !text[..at]
                .chars()
                .next_back()
                .is_some_and(|c| c.is_ascii_digit())
                && !text[at + term.len()..]
                    .chars()
                    .next()
                    .is_some_and(|c| c.is_ascii_digit())
        })
}
// Independent of Episode.binding/answer, family labels and generator state.
fn supported(request: &ModelRequest) -> Result<(i64, String)> {
    let q = &request.input;
    let past = ["과거", "예전에", "superseded"]
        .iter()
        .any(|s| q.contains(s));
    let current = ["현재", "지금", "current"].iter().any(|s| q.contains(s));
    if past == current {
        return Err(ambiguity());
    }
    let mut matches = Vec::new();
    for record in &request.evidence.items {
        let (entity, context, value) = fields(&record.original_excerpt)?;
        let number = entity.trim_start_matches(|c: char| !c.is_ascii_digit());
        if !number.is_empty()
            && number.bytes().all(|b| b.is_ascii_digit())
            && mentions(q, number)
            && mentions(q, context)
            && record.version_status == if past { "superseded" } else { "current" }
            && !record.excerpt_truncated
        {
            matches.push((record.event_id, value.to_string()));
        }
    }
    if matches.len() != 1 {
        return Err(ambiguity());
    }
    Ok(matches.remove(0))
}
fn validate_cases(cases: &[Episode], expected: usize) -> Result<()> {
    if cases.len() != expected || !expected.is_multiple_of(4) {
        return Err(ambiguity());
    }
    let mut ids = BTreeSet::new();
    for group in cases.as_chunks::<4>().0 {
        if group[0].request.input != group[2].request.input
            || group[1].request.input != group[3].request.input
            || group[0].request.input == group[1].request.input
            || group[0].answer != group[3].answer
            || group[1].answer != group[2].answer
            || group[0].answer == group[1].answer
            || replica_v3::binary::to_vec(&group[0].request.evidence)?
                != replica_v3::binary::to_vec(&group[1].request.evidence)?
            || replica_v3::binary::to_vec(&group[2].request.evidence)?
                != replica_v3::binary::to_vec(&group[3].request.evidence)?
        {
            return Err(ambiguity());
        }
        let base = group[0].id.rsplit_once('/').ok_or_else(ambiguity)?.0;
        for e in group {
            if !ids.insert(&e.id)
                || e.id.rsplit_once('/').ok_or_else(ambiguity)?.0 != base
                || supported(&e.request)?.1 != e.answer
                || !replica_v3::app::citations(&e.answer)?.is_empty()
            {
                return Err(ambiguity());
            }
        }
    }
    Ok(())
}
fn prepare_heldout(train: &[Episode], max_id: i64) -> Result<Vec<Episode>> {
    let mut rng = Rng::new(HELDOUT_SEED);
    let mut out = Vec::new();
    if max_id.checked_add(32).is_none() {
        return Err(ambiguity());
    }
    for group in 0..16 {
        let template = &train[(group % 4) * 4..(group % 4 + 1) * 4];
        let mut replacements = BTreeMap::new();
        let mut id_map = BTreeMap::new();
        for record in &template[0].request.evidence.items {
            let (entity, context, value) = fields(&record.original_excerpt)?;
            let number = entity.trim_start_matches(|c: char| !c.is_ascii_digit());
            replacements
                .entry(number.to_string())
                .or_insert_with(|| (10_000_000 + rng.next_u64() % 80_000_000).to_string());
            replacements
                .entry(context.to_string())
                .or_insert_with(|| format!("진단장소{}", rng.next_u64()));
            replacements
                .entry(value.to_string())
                .or_insert_with(|| format!("경로{}", rng.next_u64() % 1_000_000));
            id_map.insert(
                record.event_id,
                max_id + 1 + group as i64 * 2 + id_map.len() as i64,
            );
        }
        let mut replacements: Vec<_> = replacements.into_iter().collect();
        replacements.sort_by_key(|(a, _)| std::cmp::Reverse(a.len()));
        for (variant, original) in template.iter().enumerate() {
            let mut e = original.clone();
            e.id = format!("contrast-heldout/{HELDOUT_SEED}/{group}/{variant}");
            e.family = format!("contrast-heldout/{group}");
            e.request.input = data::replace_training_literals(&e.request.input, &replacements)?;
            for r in &mut e.request.evidence.items {
                r.event_id = id_map[&r.event_id];
                r.original_excerpt =
                    data::replace_training_literals(&r.original_excerpt, &replacements)?;
            }
            // Same reversal for every member of a group: order is a heldout change,
            // never an answer-dependent permutation.
            if group % 2 == 0 {
                e.request.evidence.items.reverse();
            }
            e.answer = data::replace_training_literals(&e.answer, &replacements)?;
            e.binding = hash(&replica_v3::binary::to_vec(&(group, &replacements))?);
            e.sequence = hash(&replica_v3::binary::to_vec(&e.request)?);
            out.push(e);
        }
    }
    validate_cases(&out, 64)?;
    for e in &out {
        if train
            .iter()
            .any(|old| old.answer == e.answer || old.binding == e.binding || old.id == e.id)
            || e.request
                .evidence
                .items
                .iter()
                .any(|r| r.event_id <= max_id)
        {
            return Err(Error::Invalid("heldout scene/value/ID overlap".into()));
        }
    }
    Ok(out)
}
pub fn freeze(corpus: &Path, log: &Path, tokenizer: &Path, output: &Path) -> Result<()> {
    let (m, train, validation) = data::load(corpus)?;
    let max_id = train
        .iter()
        .chain(&validation)
        .flat_map(|e| &e.request.evidence.items)
        .map(|r| r.event_id)
        .max()
        .ok_or_else(ambiguity)?;
    let cases: Vec<_> = train
        .into_iter()
        .take(400)
        .filter(|e| e.family.starts_with("copy/") && e.family.ends_with("/value"))
        .collect();
    validate_cases(&cases, 16)?;
    let log_bytes = read_bounded(log, 16 * 1024 * 1024)?;
    let rows: Vec<replica_v3::binary::Value> = replica_v3::binary::records_from_slice(&log_bytes)?;
    let header = rows.first().ok_or_else(ambiguity)?;
    if header["split"] != "train"
        || header["split_sha256"] != m.train.sha256
        || header["oracle_question_ablation"] != false
        || header["oracle_record_selection"] != false
    {
        return Err(Error::Invalid(
            "freeze requires original train generation log".into(),
        ));
    }
    for e in &cases {
        let matching: Vec<_> = rows.iter().filter(|r| r["id"] == e.id).collect();
        if matching.len() != 1
            || matching[0]["expected"] != e.answer
            || matching[0]["question"] != e.request.input
            || matching[0]["evidence"] != replica_v3::binary::to_value(&e.request.evidence)?
        {
            return Err(Error::Corrupt(
                "original failure log/corpus mismatch".into(),
            ));
        }
    }
    let tok = ByteBpe::load(tokenizer)?;
    let heldout = prepare_heldout(&cases, max_id)?;
    let frozen = Frozen {
        version: 1,
        corpus_hash: m.train.sha256,
        original_log_hash: hash(&log_bytes),
        tokenizer_hash: tok.id(),
        train_hash: hash(&replica_v3::binary::to_vec(&cases)?),
        heldout_hash: hash(&replica_v3::binary::to_vec(&heldout)?),
        heldout_seed: HELDOUT_SEED,
        train: cases,
        heldout,
    };
    // Publish only after the independent serialized-input validator and token path pass.
    let framed = samples(&frozen.train, &tok, 512)?;
    for (e, s) in frozen.train.iter().zip(framed) {
        verify_framing(e, &s, &tok)?;
    }
    samples(&frozen.heldout, &tok, 512)?;
    write_new(output, &replica_v3::binary::to_vec(&frozen)?)?;
    eprintln!(
        "freeze_train={} freeze_heldout={} original_log={} tokenizer={} train=16 groups=4 heldout=64 groups=16 heldout_seed={} original_bytes_preserved=true citation_target=NOT_REQUESTED",
        frozen.train_hash,
        frozen.heldout_hash,
        frozen.original_log_hash,
        frozen.tokenizer_hash,
        frozen.heldout_seed
    );
    Ok(())
}
fn load_fixture(path: &Path, tok: &ByteBpe) -> Result<Frozen> {
    let f: Frozen = replica_v3::binary::from_slice(&read_bounded(path, 2 * 1024 * 1024)?)?;
    if f.version != 1
        || f.tokenizer_hash != tok.id()
        || f.train_hash != hash(&replica_v3::binary::to_vec(&f.train)?)
        || f.heldout_hash != hash(&replica_v3::binary::to_vec(&f.heldout)?)
        || f.heldout_seed != HELDOUT_SEED
    {
        return Err(Error::Corrupt("contrast frozen identity".into()));
    }
    validate_cases(&f.train, 16)?;
    validate_cases(&f.heldout, 64)?;
    Ok(f)
}
fn verify_framing(e: &Episode, s: &Sample, tok: &ByteBpe) -> Result<()> {
    let p = tok.prepare(&e.request, 512, "contrast")?;
    if p.token_ids != s.tokens[..s.response_start]
        || !p.excluded.is_empty()
        || s.tokens.last() != Some(&EOS)
        || tok.decode(&s.tokens[s.response_start..s.tokens.len() - 1])? != e.answer
    {
        return Err(Error::Corrupt("contrast prompt/target/EOS mismatch".into()));
    }
    // Assemble role spans independently from prepare(), preserving every raw byte.
    let mut ids = vec![neural::BOS, neural::SYSTEM_ROLE];
    let mut spans = Vec::new();
    let start = ids.len();
    ids.extend(tok.encode(e.request.system.as_bytes())?);
    spans.push(("system".to_string(), start, ids.len()));
    ids.extend([neural::END_ROLE, neural::USER_ROLE]);
    let start = ids.len();
    ids.extend(tok.encode(e.request.input.as_bytes())?);
    spans.push(("question".to_string(), start, ids.len()));
    ids.push(neural::END_ROLE);
    for record in &e.request.evidence.items {
        ids.push(neural::EVIDENCE_ROLE);
        let start = ids.len();
        ids.extend(tok.encode(neural::evidence_text(record).as_bytes())?);
        spans.push((format!("evidence:{}", record.event_id), start, ids.len()));
        ids.push(neural::END_ROLE);
    }
    ids.push(neural::ASSISTANT_ROLE);
    if ids != p.token_ids {
        return Err(Error::Corrupt("independent role framing".into()));
    }
    replica_v3::binary::print_record(&replica_v3::binary::record!({"framing":true,"id":e.id,"prompt_ids":ids,"role_spans":spans,
        "prompt_tokens":s.response_start,"target_tokens_with_eos":s.tokens.len()-s.response_start,
        "target_ids":&s.tokens[s.response_start..],"supported_record":supported(&e.request)?.0,
        "citation_target":"NOT_REQUESTED","input_tokens":s.tokens.len()-1}))?;
    Ok(())
}
fn max_error(a: &Tensor, b: &Tensor) -> Result<f32> {
    let value = (a - b)?.abs()?.max_all()?.to_scalar::<f32>()?;
    if !value.is_finite() || value > LOGIT_TOLERANCE {
        return Err(Error::Model(format!(
            "contrast logit parity error={value} tolerance={LOGIT_TOLERANCE}"
        )));
    }
    Ok(value)
}
fn last_logits(model: &Transformer, ids: &[u32]) -> Result<Vec<f32>> {
    Ok(model
        .forward(&Tensor::new(ids, &model.device)?.unsqueeze(0)?, None)?
        .narrow(1, ids.len() - 1, 1)?
        .flatten_all()?
        .to_vec1::<f32>()?)
}
fn uncached(model: &Transformer, prompt: &[u32]) -> Result<(Vec<u32>, String)> {
    let mut all = prompt.to_vec();
    let mut tokens = Vec::new();
    for _ in 0..32 {
        let row = last_logits(model, &all)?;
        // Candle argmax selects the first maximum, including exact ties.
        let mut best = 0;
        for i in 1..row.len() {
            if row[i] > row[best] {
                best = i;
            }
        }
        if best == EOS as usize {
            return Ok((tokens, "stop".into()));
        }
        if best < neural::SPECIALS {
            return Ok((tokens, "control".into()));
        }
        tokens.push(best as u32);
        all.push(best as u32);
    }
    Ok((tokens, "length".into()))
}
pub fn check(fixture: &Path, path: &Path) -> Result<()> {
    let loaded = checkpoint::load(path, Device::Cpu, false)?;
    let f = load_fixture(fixture, &loaded.tokenizer)?;
    let model = &loaded.model;
    let framed = samples(&f.train, &loaded.tokenizer, 512)?;
    eprintln!(
        "check_weight={} tolerance={LOGIT_TOLERANCE} backend={}",
        loaded.manifest.weights_sha256,
        neural::cpu_backend()
    );
    for (e, sample) in f.train.iter().zip(&framed) {
        verify_framing(e, sample, &loaded.tokenizer)?;
    }
    for group in 0..4 {
        let indices: Vec<_> = (group * 4..group * 4 + 4).collect();
        let b = batch(&framed, &indices, &Device::Cpu)?;
        let logits = model.forward(&b.input, Some(&b.valid))?;
        let targets = b.target.to_vec2::<u32>()?;
        let masks = b.mask.to_vec2::<f32>()?;
        let inputs = b.input.to_vec2::<u32>()?;
        let mut expected_count = 0;
        for (row, &i) in indices.iter().enumerate() {
            let s = &framed[i];
            let single = batch(&framed, &[i], &Device::Cpu)?;
            for p in 0..inputs[row].len() {
                let valid = p < s.tokens.len() - 1;
                let expected_mask = f32::from(valid && p + 1 >= s.response_start);
                if masks[row][p] != expected_mask
                    || (valid
                        && (inputs[row][p] != s.tokens[p] || targets[row][p] != s.tokens[p + 1]))
                {
                    return Err(Error::Corrupt("independent shift/target-mask check".into()));
                }
                expected_count += usize::from(expected_mask == 1.);
            }
            let error = max_error(
                &model.forward(&single.input, Some(&single.valid))?,
                &logits.narrow(0, row, 1)?.narrow(1, 0, s.tokens.len() - 1)?,
            )?;
            let prompt = &s.tokens[..s.response_start];
            let direct = model.forward(&Tensor::new(prompt, &Device::Cpu)?.unsqueeze(0)?, None)?;
            let mut chunk_errors = Vec::new();
            for size in [127, 128, 257] {
                let mut cache = model.cache("contrast-check");
                let mut parts = Vec::new();
                for chunk in prompt.chunks(size) {
                    parts.push(model.forward_cached(
                        &Tensor::new(chunk, &Device::Cpu)?.unsqueeze(0)?,
                        &mut cache,
                        "contrast-check",
                    )?);
                }
                chunk_errors.push((size, max_error(&direct, &Tensor::cat(&parts, 1)?)?));
            }
            let u = uncached(model, prompt)?;
            let cached = model.generate(
                prompt,
                32,
                120_000,
                &AtomicBool::new(false),
                "contrast-check",
            );
            let equal = match &cached {
                Ok(c) => c.tokens == u.0 && c.finish == u.1,
                Err(e) => u.1 == "control" && e.to_string().contains("control token"),
            };
            if !equal {
                return Err(Error::Model(
                    "contrast cached/uncached greedy mismatch".into(),
                ));
            }
            replica_v3::binary::print_record(&replica_v3::binary::record!({"parity_case":i,"batch_error":error,"chunk_errors":chunk_errors,"greedy_parity":equal,"uncached_ids":u.0,"finish":u.1}))?;
        }
        let (loss, _, count) = response_loss(&logits, &b, 1.)?;
        if count != expected_count {
            return Err(Error::Corrupt("independent loss denominator".into()));
        }
        let grads = loss.backward()?;
        for (name, var) in &model.vars {
            let g = grads
                .get(var)
                .ok_or_else(|| Error::Model(format!("detached gradient {name}")))?;
            let norm = g.sqr()?.sum_all()?.to_scalar::<f32>()?;
            if !norm.is_finite() || norm == 0. {
                return Err(Error::Model(format!(
                    "contrast gradient needs diagnosis: {name} norm2={norm}"
                )));
            }
            eprintln!(
                "gradient_group={group} tensor={name} norm2={norm} finite_nonzero=true output=tied_embedding"
            );
        }
    }
    for len in [255, 256, 257] {
        for window in [None, Some(256)] {
            let allowed =
                neural::transformer::attention_mask(0..len, 0..len, window, None, 1, &Device::Cpu)?;
            let mask = allowed.flatten_all()?.to_vec1::<f32>()?;
            for q in 0..len {
                for k in 0..len {
                    let expected = k <= q && window.is_none_or(|w| q - k < w);
                    if mask[q * len + k] != f32::from(expected) {
                        return Err(Error::Corrupt(
                            "independent SMALL causal/local boundary".into(),
                        ));
                    }
                }
            }
        }
        let ids: Vec<_> = framed[0].tokens.iter().copied().cycle().take(len).collect();
        let reference = model.forward(
            &Tensor::new(ids.as_slice(), &Device::Cpu)?.unsqueeze(0)?,
            None,
        )?;
        let mut cache = model.cache("boundary");
        let mut chunks = Vec::new();
        for c in ids.chunks(128) {
            chunks.push(model.forward_cached(
                &Tensor::new(c, &Device::Cpu)?.unsqueeze(0)?,
                &mut cache,
                "boundary",
            )?);
        }
        eprintln!(
            "boundary={len} max_error={}",
            max_error(&reference, &Tensor::cat(&chunks, 1)?)?
        );
    }
    eprintln!("contrast_preflight=PASS quality=NOT_EVALUATED");
    Ok(())
}
fn branch(row: &[f32], expected: u32, alternate: u32) -> replica_v3::binary::Value {
    let max = row.iter().copied().fold(f32::NEG_INFINITY, f32::max);
    let denom: f64 = row.iter().map(|&v| f64::from(v - max).exp()).sum();
    replica_v3::binary::record!({"expected_id":expected,"alternate_id":alternate,
        "probability":f64::from(row[expected as usize]-max).exp()/denom,
        "expected_minus_alternate_logit":row[expected as usize]-row[alternate as usize]})
}
fn score(
    model: &Transformer,
    tok: &ByteBpe,
    cases: &[Episode],
    step: usize,
) -> Result<(usize, usize)> {
    let framed = samples(cases, tok, 512)?;
    let mut exact = Vec::new();
    let mut values = 0;
    let mut record_inferred = 0;
    let mut eos = 0;
    for (i, (e, s)) in cases.iter().zip(&framed).enumerate() {
        let prompt = &s.tokens[..s.response_start];
        let generated = model.generate(
            prompt,
            32,
            120_000,
            &AtomicBool::new(false),
            "contrast-eval",
        );
        let (tokens, finish, error) = match generated {
            Ok(g) => (g.tokens, g.finish, None),
            Err(e) => (Vec::new(), "error".into(), Some(e.to_string())),
        };
        let text = tok.decode(&tokens).ok();
        let value = text.as_deref() == Some(e.answer.as_str());
        let stopped = finish == "stop";
        exact.push(value && stopped);
        values += usize::from(value);
        eos += usize::from(stopped);
        let support = supported(&e.request)?.0;
        let inferred: Vec<_> = e
            .request
            .evidence
            .items
            .iter()
            .filter_map(|r| {
                fields(&r.original_excerpt)
                    .ok()
                    .filter(|(_, _, v)| Some(*v) == text.as_deref())
                    .map(|_| r.event_id)
            })
            .collect();
        let record = inferred == [support];
        record_inferred += usize::from(record);
        let answer = &s.tokens[s.response_start..];
        let other = &framed[i ^ 1];
        let alternate = &other.tokens[other.response_start..];
        let divergence = answer
            .iter()
            .zip(alternate)
            .position(|(a, b)| a != b)
            .ok_or_else(ambiguity)?;
        let mut teacher = prompt.to_vec();
        teacher.extend(&answer[..divergence]);
        let teacher_branch = branch(
            &last_logits(model, &teacher)?,
            answer[divergence],
            alternate[divergence],
        );
        let greedy_branch = if tokens.len() >= divergence {
            let mut p = prompt.to_vec();
            p.extend(&tokens[..divergence]);
            Some(branch(
                &last_logits(model, &p)?,
                answer[divergence],
                alternate[divergence],
            ))
        } else {
            None
        };
        replica_v3::binary::print_record(&replica_v3::binary::record!({"evaluation":true,"step":step,"id":e.id,"group":i/4,"expected":e.answer,"actual":text,"generated_ids":tokens,"finish":finish,"error":error,
            "full_exact":value&&stopped,"value_match":value,"eos":stopped,"record_id_target":support,"record_id_output":"NOT_REQUESTED",
            "value_inferred_record_ids":inferred,"value_inferred_record_match":record,"first_answer_divergence":divergence,
            "teacher_forced_branch":teacher_branch,"greedy_prefix_branch":greedy_branch,"prompt_tokens":s.response_start,"target_tokens":answer.len(),"output_tokens":tokens.len()+usize::from(stopped)}))?;
    }
    let groups = exact
        .as_chunks::<4>()
        .0
        .iter()
        .filter(|g| g.iter().all(|&v| v))
        .count();
    let total = exact.iter().filter(|&&x| x).count();
    replica_v3::binary::print_record(&replica_v3::binary::record!({"contrast_summary":true,"step":step,"exact":total,"cases":cases.len(),"groups_all_correct":groups,"groups":cases.len()/4,"value":values,"value_inferred_record":record_inferred,"eos":eos,"citation_grade":"NOT_REQUESTED","goal1_ready":false}))?;
    Ok((total, groups))
}
pub fn evaluate(fixture: &Path, path: &Path, heldout: bool) -> Result<()> {
    let loaded = checkpoint::load(path, Device::Cpu, false)?;
    let f = load_fixture(fixture, &loaded.tokenizer)?;
    let step = loaded.manifest.training.as_ref().map_or(0, |s| s.step);
    eprintln!(
        "checkpoint={} model_hash={} frozen_train={} frozen_heldout={} heldout={heldout}",
        loaded.manifest.weights_sha256,
        loaded.model.weight_hash()?,
        f.train_hash,
        f.heldout_hash
    );
    let (n, g) = score(
        &loaded.model,
        &loaded.tokenizer,
        if heldout { &f.heldout } else { &f.train },
        step,
    )?;
    eprintln!(
        "diagnostic_match={n} groups_correct={g} source_quality=UNQUALIFIED goal1_ready=false"
    );
    Ok(())
}

// Training-derived development ablations. These never read or score heldout cases,
// change model parameters, or select evidence in the product inference path.
fn transfer_cases(train: &[Episode], factor: &str) -> Result<Vec<Episode>> {
    let mut cases = train.to_vec();
    for (group, quartet) in cases.as_chunks_mut::<4>().0.iter_mut().enumerate() {
        let mut replacements = BTreeMap::new();
        for record in &quartet[0].request.evidence.items {
            let (entity, context, value) = fields(&record.original_excerpt)?;
            let number = entity.trim_start_matches(|c: char| !c.is_ascii_digit());
            let index = replacements.len();
            let pair = match factor {
                "entity" => Some((number, (800_001 + group * 14 + index * 2).to_string())),
                "entity_long" => Some((number, (80_000_001 + group * 14 + index * 2).to_string())),
                "context" => Some((context, format!("통로{}", 40 + group * 2 + index))),
                "known_values" => {
                    let values = [
                        "오른쪽",
                        "왼쪽",
                        "직진",
                        "대기",
                        "북쪽",
                        "남쪽",
                        "동쪽",
                        "서쪽",
                    ];
                    let i = values
                        .iter()
                        .position(|v| *v == value)
                        .ok_or_else(ambiguity)?;
                    Some((value, values[(i + 1) % values.len()].to_string()))
                }
                "new_values" => {
                    Some((value, format!("경로{}", 100_003 + (group * 2 + index) * 79)))
                }
                "identity" | "record_ids" | "order" => None,
                _ => return Err(Error::Invalid("unknown transfer factor".into())),
            };
            if let Some((old, new)) = pair {
                replacements.entry(old.to_string()).or_insert(new);
            }
        }
        let mut replacements: Vec<_> = replacements.into_iter().collect();
        replacements.sort_by_key(|(old, _)| std::cmp::Reverse(old.len()));
        for e in quartet {
            e.request.input = data::replace_training_literals(&e.request.input, &replacements)?;
            e.answer = data::replace_training_literals(&e.answer, &replacements)?;
            for record in &mut e.request.evidence.items {
                record.original_excerpt =
                    data::replace_training_literals(&record.original_excerpt, &replacements)?;
                if factor == "record_ids" {
                    record.event_id = record
                        .event_id
                        .checked_add(1_000_000)
                        .ok_or_else(ambiguity)?;
                }
            }
            if factor == "order" {
                e.request.evidence.items.reverse();
            }
        }
    }
    validate_cases(&cases, train.len())?;
    Ok(cases)
}

pub fn transfer(fixture: &Path, path: &Path) -> Result<()> {
    let loaded = checkpoint::load(path, Device::Cpu, false)?;
    let frozen = load_fixture(fixture, &loaded.tokenizer)?;
    let mut trained_targets = BTreeSet::new();
    for e in &frozen.train {
        trained_targets.extend(loaded.tokenizer.encode(e.answer.as_bytes())?);
    }
    let step = loaded.manifest.trained_steps;
    eprintln!(
        "transfer_development=true source_train={} weights_content={} backend={} heldout=NOT_RUN training=NOT_RUN",
        frozen.train_hash,
        loaded.model.weights_content_id()?,
        neural::cpu_backend()
    );
    for factor in [
        "identity",
        "record_ids",
        "order",
        "entity",
        "entity_long",
        "context",
        "known_values",
        "new_values",
    ] {
        let cases = transfer_cases(&frozen.train, factor)?;
        let mut new_tokens = BTreeSet::new();
        for e in &cases {
            new_tokens.extend(
                loaded
                    .tokenizer
                    .encode(e.answer.as_bytes())?
                    .into_iter()
                    .filter(|id| !trained_targets.contains(id)),
            );
        }
        eprintln!(
            "factor={factor} inputs_sha256={} target_ids_absent_from_frozen16_answers={new_tokens:?}",
            hash(&replica_v3::binary::to_vec(&cases)?)
        );
        let (exact, groups) = score(&loaded.model, &loaded.tokenizer, &cases, step)?;
        eprintln!(
            "transfer_summary factor={factor} exact={exact}/16 groups={groups}/4 final_heldout=false"
        );
    }
    Ok(())
}
pub fn train(
    fixture: &Path,
    path: &Path,
    output: &Path,
    source: &str,
    start_kind: &str,
    both_orders: bool,
    cancel: &AtomicBool,
) -> Result<()> {
    if source.len() != 64 || !source.bytes().all(|b| b.is_ascii_hexdigit()) {
        return Err(Error::Invalid("contrast source hash".into()));
    }
    let mut loaded = checkpoint::load(path, Device::Cpu, false)?;
    if let Some(state) = &loaded.manifest.training {
        checkpoint::ResumeBinding::require_default(state, &loaded.tokenizer)?;
    }
    if loaded.model.config != Config::small(loaded.tokenizer.vocab_size()) {
        return Err(Error::Invalid("contrast requires unchanged SMALL".into()));
    }
    let parent = loaded.manifest.weights_sha256.clone();
    let parent_model = loaded.model.weight_hash()?;
    match start_kind {
        "random"
            if loaded.manifest.training.is_none()
                && loaded.manifest.init_seed == 17
                && loaded.manifest.initial_weight_hash == parent_model => {}
        "qa" if loaded
            .manifest
            .training
            .as_ref()
            .is_some_and(|s| s.step > 0 && s.config.curriculum_steps < s.step && !s.contrast16) => {
        }
        "diagnostic"
            if both_orders
                && loaded
                    .manifest
                    .training
                    .as_ref()
                    .is_some_and(|s| s.step > 0 && s.contrast16) => {}
        _ => {
            return Err(Error::Invalid(
                "contrast initialization/provenance; QA excludes auxiliary-only checkpoint".into(),
            ));
        }
    }
    if start_kind == "random" {
        let mut fresh = Transformer::init(loaded.model.config.clone(), 17, Device::Cpu)?;
        if fresh.weight_hash()? != parent_model {
            return Err(Error::Corrupt(
                "seed17 initialization/reference mismatch".into(),
            ));
        }
        fresh.bind_tokenizer(&loaded.tokenizer.id())?;
        loaded.model = fresh;
    }
    let f = load_fixture(fixture, &loaded.tokenizer)?;
    let mut train = f.train.clone();
    if both_orders {
        let mut reversed = transfer_cases(&f.train, "order")?;
        for e in &mut reversed {
            e.id = format!("reversed/{}", e.id);
        }
        train.extend(reversed);
    }
    validate_cases(&train, if both_orders { 32 } else { 16 })?;
    let train_hash = hash(&replica_v3::binary::to_vec(&train)?);
    let framed = samples(&train, &loaded.tokenizer, 512)?;
    let config = TrainConfig {
        warmup: 20,
        max_steps: MAX_UPDATES,
        max_tokens: MAX_INPUT,
        microbatch: 4,
        sample_group_size: 4,
        accumulation: train.len() / 4,
        ..TrainConfig::default()
    };
    config.validate(loaded.model.config.context)?;
    let step_input: usize = framed.iter().map(|s| s.tokens.len() - 1).sum();
    let mut previous = loaded
        .manifest
        .training
        .as_ref()
        .map(|s| s.previous_corpora.clone())
        .unwrap_or_default();
    for h in [loaded.tokenizer.train_hash.clone(), f.corpus_hash.clone()] {
        if !previous.contains(&h) {
            previous.push(h);
        }
    }
    let mut state = TrainingState {
        resume_binding: None,
        contrast16: true,
        parent_checkpoint_hash: Some(parent.clone()),
        config: config.clone(),
        step: 0,
        consumed_tokens: 0,
        target_tokens: 0,
        sampler_state: 17,
        corpus_hash: train_hash.clone(),
        validation_hash: f.heldout_hash.clone(),
        previous_corpora: previous,
        initial_weight_hash: loaded.manifest.initial_weight_hash.clone(),
        train_loss: None,
        validation_loss: None,
    };
    state.resume_binding = Some(checkpoint::ResumeBinding::default_for(
        &state,
        &loaded.tokenizer,
    ));
    let mut adam = Adam::new(&loaded.model.vars)?;
    let mut rng = Rng::new(17);
    std::fs::create_dir(output)?;
    loaded.manifest.source_id = source.into();
    loaded.manifest.status = "TRAINING".into();
    let started = Instant::now();
    let mut reason = "BUDGET_EXHAUSTED";
    let mut streak = 0;
    replica_v3::binary::print_record(&replica_v3::binary::record!({"run_start":true,"kind":start_kind,"parent_checkpoint_hash":parent,"parent_model_hash":parent_model,
        "config":config,"fresh_adam":true,"sampler_state":rng.state,"per_update_input":step_input,"train_hash":train_hash,"original_train_hash":f.train_hash,"heldout_hash":f.heldout_hash,
        "both_evidence_orders":both_orders,"cases":train.len(),"max_seconds":MAX_SECONDS,"backend":neural::cpu_backend(),"source_id":source,"sampling":"quartets shuffled within each view, each declared case exactly once per update","quality":"DIAGNOSTIC_ONLY"}))?;
    loop {
        if cancel.load(Ordering::Relaxed) {
            reason = "CANCELLED";
            break;
        }
        if started.elapsed().as_secs() >= MAX_SECONDS {
            break;
        }
        if state.step.is_multiple_of(25) {
            let rss = super::rss_kib()?;
            eprintln!("resource_step={} current_rss_KiB={rss}", state.step);
            if rss > 16 * 1024 * 1024 {
                reason = "RESOURCE_LIMIT";
                break;
            }
        }
        if state.step.is_multiple_of(100) {
            loaded.model.refresh_identity()?;
            loaded.manifest.training = Some(state.clone());
            let frozen = output.join(format!("step-{:06}", state.step));
            let saved = checkpoint::save(
                &frozen,
                &loaded.model,
                &loaded.tokenizer,
                loaded.manifest.clone(),
                &adam.moments,
            )?;
            eprintln!(
                "snapshot={} sha256={}",
                frozen.display(),
                saved.weights_sha256
            );
            let (n, g) = score(&loaded.model, &loaded.tokenizer, &train, state.step)?;
            streak = if n == train.len() && g == train.len() / 4 {
                streak + 1
            } else {
                0
            };
            if streak >= 2 {
                reason = "TWO_EVALUATIONS_PASS_PENDING_FRESH_RELOAD";
                break;
            }
        }
        if cancel.load(Ordering::Relaxed) {
            reason = "CANCELLED";
            break;
        }
        if state.step >= MAX_UPDATES
            || state.consumed_tokens + step_input as u64 > MAX_INPUT
            || started.elapsed().as_secs() >= MAX_SECONDS
        {
            break;
        }
        let mut order = Vec::new();
        for view in 0..train.len() / 16 {
            order.extend(group_order(&mut rng).map(|group| view * 4 + group));
        }
        let mut gradients = BTreeMap::new();
        let mut targets = 0;
        let mut total_loss = 0.;
        for group in order {
            let ids: Vec<_> = (group * 4..group * 4 + 4).collect();
            let b = batch(&framed, &ids, &Device::Cpu)?;
            let (loss, objective, n) =
                response_loss(&loaded.model.forward(&b.input, Some(&b.valid))?, &b, 1.)?;
            let value = f64::from(loss.to_scalar::<f32>()?);
            if !value.is_finite() {
                return Err(Error::Model(
                    "contrast nonfinite loss; prior snapshot preserved".into(),
                ));
            }
            let grads = objective.backward()?;
            targets += n;
            total_loss += value * n as f64;
            for (name, var) in &loaded.model.vars {
                let g = grads
                    .get(var)
                    .ok_or_else(|| Error::Model(format!("contrast missing gradient {name}")))?;
                let weighted = (g * n as f64)?.detach();
                let next = match gradients.remove(name) {
                    Some(old) => (old + weighted)?,
                    None => weighted,
                };
                gradients.insert(name.clone(), next.detach());
            }
        }
        for g in gradients.values_mut() {
            *g = (&*g / targets as f64)?;
        }
        let (norm, delta) = adam.step(&loaded.model.vars, &gradients, &config, state.step + 1)?;
        state.step += 1;
        state.consumed_tokens += step_input as u64;
        state.target_tokens += targets as u64;
        state.sampler_state = rng.state;
        state.train_loss = Some(total_loss / targets as f64);
        eprintln!(
            "step={} loss={:.9} input_tokens={} target_tokens={} each_case_exposure={} grad_norm={norm} weight_delta={delta} elapsed_s={:.3}",
            state.step,
            state.train_loss.expect("observed"),
            state.consumed_tokens,
            state.target_tokens,
            state.step,
            started.elapsed().as_secs_f64()
        );
    }
    loaded.model.refresh_identity()?;
    loaded.manifest.training = Some(state.clone());
    loaded.manifest.status = match reason {
        "CANCELLED" => "CANCELLED",
        "RESOURCE_LIMIT" => "RESOURCE_LIMIT",
        "BUDGET_EXHAUSTED" => "BUDGET_EXHAUSTED",
        _ => "DIAGNOSTIC_COMPLETE",
    }
    .into();
    let saved = checkpoint::save(
        &output.join("final"),
        &loaded.model,
        &loaded.tokenizer,
        loaded.manifest,
        &adam.moments,
    )?;
    eprintln!(
        "CONTRAST_END reason={reason} updates={} input_tokens={} target_tokens={} sampler_state={} each_case_exposure={} checkpoint={} sha256={} elapsed_s={:.3} heldout=NOT_RUN goal1_ready=false",
        state.step,
        state.consumed_tokens,
        state.target_tokens,
        state.sampler_state,
        state.step,
        output.join("final").display(),
        saved.weights_sha256,
        started.elapsed().as_secs_f64()
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn transfer_changes_only_declared_factor_and_retains_unique_input_answer() {
        let mut train = Vec::new();
        for group in 0..4 {
            for variant in 0..4 {
                let values = if variant < 2 {
                    ["북쪽", "동쪽"]
                } else {
                    ["동쪽", "북쪽"]
                };
                let current = variant % 2 == 0;
                train.push(Episode {
                    id: format!("literal/{group}/{variant}"),
                    category: 1,
                    family: "fixture".into(),
                    binding: "fixture".into(),
                    sequence: "fixture".into(),
                    request: ModelRequest {
                        request_id: "fixture".into(),
                        system: String::new(),
                        input: format!(
                            "번호 123456의 통로2 {} 방향",
                            if current { "현재" } else { "과거" }
                        ),
                        limits: Default::default(),
                        evidence: replica_v3::retrieval::EvidenceBundle {
                            items: values
                                .iter()
                                .enumerate()
                                .map(|(i, value)| replica_v3::retrieval::Evidence {
                                    event_id: 7 + i as i64,
                                    source: "literal".into(),
                                    recorded_at: 10 + i as i64,
                                    observed_at: Some(9),
                                    version_status: if i == 0 { "current" } else { "superseded" }
                                        .into(),
                                    original_excerpt: format!(
                                        "대상123456의 통로2 이동 지시는 {value}이다."
                                    ),
                                    excerpt_truncated: false,
                                    relation_path: Vec::new(),
                                    retrieval_reason: "fixture".into(),
                                })
                                .collect(),
                            ..Default::default()
                        },
                    },
                    answer: values[usize::from(!current)].into(),
                });
            }
        }
        validate_cases(&train, 16).unwrap();
        let before = replica_v3::binary::to_vec(&train).unwrap();
        assert_eq!(
            replica_v3::binary::to_vec(&transfer_cases(&train, "identity").unwrap()).unwrap(),
            before
        );
        for factor in [
            "record_ids",
            "order",
            "entity",
            "entity_long",
            "context",
            "known_values",
            "new_values",
        ] {
            let changed = transfer_cases(&train, factor).unwrap();
            for (old, new) in train.iter().zip(&changed) {
                assert_eq!(new.request.evidence.items.len(), 2);
                assert_eq!(supported(&new.request).unwrap().1, new.answer);
                if matches!(factor, "entity" | "entity_long" | "context") {
                    assert_ne!(old.request.input, new.request.input);
                    assert_eq!(old.answer, new.answer);
                } else {
                    assert_eq!(old.request.input, new.request.input);
                }
                if factor == "record_ids" {
                    for (a, b) in old
                        .request
                        .evidence
                        .items
                        .iter()
                        .zip(&new.request.evidence.items)
                    {
                        let mut expected = a.clone();
                        expected.event_id += 1_000_000;
                        assert_eq!(
                            replica_v3::binary::to_vec(b).unwrap(),
                            replica_v3::binary::to_vec(&expected).unwrap()
                        );
                    }
                } else if factor == "order" {
                    assert_eq!(
                        new.request.evidence.items[0].event_id,
                        old.request.evidence.items[1].event_id
                    );
                } else {
                    assert_eq!(
                        new.request.evidence.items[0].event_id,
                        old.request.evidence.items[0].event_id
                    );
                }
            }
        }
        assert_eq!(replica_v3::binary::to_vec(&train).unwrap(), before);
        assert!(transfer_cases(&train, "unknown").is_err());
    }
    #[test]
    fn full_pass_exposure_and_sampler_restart_are_exact() {
        let mut rng = Rng::new(17);
        let mut exposure = [0usize; 16];
        for _ in 0..100 {
            let before = rng.state;
            let groups = group_order(&mut rng);
            assert_eq!(group_order(&mut Rng::new(before)), groups);
            let mut sorted = groups;
            sorted.sort();
            assert_eq!(sorted, [0, 1, 2, 3]);
            for group in groups {
                for count in &mut exposure[group * 4..group * 4 + 4] {
                    *count += 1;
                }
            }
        }
        assert_eq!(exposure, [100; 16]);
    }
    #[test]
    fn serialized_answer_validator_rejects_ambiguous_status_and_numeric_prefixes() {
        let mut request = ModelRequest {
            request_id: "fixture".into(),
            system: String::new(),
            input: "번호 123456의 통로2 현재 방향".into(),
            evidence: Default::default(),
            limits: Default::default(),
        };
        request
            .evidence
            .items
            .push(replica_v3::retrieval::Evidence {
                event_id: 7,
                source: "fixture".into(),
                recorded_at: 10,
                observed_at: Some(9),
                version_status: "current".into(),
                original_excerpt: "대상123456의 통로2 이동 지시는 북쪽이다.".into(),
                excerpt_truncated: false,
                relation_path: Vec::new(),
                retrieval_reason: "literal independent fixture".into(),
            });
        assert_eq!(supported(&request).unwrap(), (7, "북쪽".into()));
        request.input = "번호 1234567의 통로2 현재 방향".into();
        assert!(supported(&request).is_err());
        request.input = "번호 123456의 통로20 현재 방향".into();
        assert!(supported(&request).is_err());
        request.input = "번호 123456의 통로2 현재 과거 방향".into();
        assert!(supported(&request).is_err());
        request.input = "번호 123456의 통로2 현재 방향".into();
        let mut duplicate = request.evidence.items[0].clone();
        duplicate.event_id = 8;
        request.evidence.items.push(duplicate);
        assert!(supported(&request).is_err());
    }
}
