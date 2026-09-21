//! One bounded value/citation continuation. All execution and storage stay in
//! the existing binding/fresh harness; these transformations are training-only.
use super::*;

const DATA: &str = "value-citation-bridge-v1";
const CONTRACT: &str = "R3-VALUE-CITATION-BRIDGE-1.0";
pub(super) const ARM: &str = "VALUE-CITATION";
const ORIGIN: usize = 4352;
const UPDATES: usize = 3072;
const POOL: usize = 1536;
fn terminal_path(root: &Path, end: &Segment) -> Result<PathBuf> {
    let segment = Path::new(&end.checkpoint)
        .parent()
        .and_then(Path::file_name)
        .and_then(|n| n.to_str())
        .ok_or_else(|| bad("citation parent checkpoint segment"))?;
    if !segment.starts_with("segment-")
        || segment.len() != 12
        || !segment[8..].bytes().all(|b| b.is_ascii_digit())
    {
        return Err(bad("citation native segment path"));
    }
    Ok(root.join(format!("{segment}-finished.r3b")))
}

pub(in super::super::super) fn is(p: &Plan) -> bool {
    p.identifiable.as_ref().is_some_and(|v| v.dataset == DATA)
}
pub(super) fn evaluation(p: &Plan) -> EvaluationPolicy {
    let mut e = super::evaluation(p.tiny);
    e.screen_steps.clear();
    e.train_steps = if p.tiny {
        vec![p.origin_step() + 2]
    } else {
        [32, 128, 384, 768, 1536, 3072].map(|n| ORIGIN + n).to_vec()
    };
    e.generation_limit = 16384;
    e.teacher_limit = 16384;
    e.active_seconds = 10800;
    e
}
pub(in super::super::super) fn endpoint(p: &Plan, step: usize, pending: bool) -> Result<usize> {
    if !is(p) || step < p.origin_step() || step > p.config.max_steps {
        return Err(bad("citation cursor"));
    }
    if pending && p.evaluation_due(step) {
        return Ok(step);
    }
    if p.tiny {
        return (step < p.config.max_steps)
            .then_some(p.config.max_steps)
            .ok_or_else(|| bad("citation fixture closed"));
    }
    [4384, 4480, 4736, 5120, 5632, 5888, 6400, 6912, 7424]
        .into_iter()
        .find(|&n| n > step)
        .ok_or_else(|| bad("citation budget closed"))
}
fn suffix() -> Vec<[usize; 8]> {
    (0..UPDATES)
        .map(|step| {
            let mut row = [0; 8];
            for slot in 0..2 {
                let v = (step * 2 + slot) % 768;
                let a = (v % 384) * 4 + (v / 384) * 2;
                let b = POOL * (1 + (step / 192 + slot) % 2)
                    + ((step * 2 + slot + 192) % 384) * 4
                    + ((step / 384) % 2) * 2;
                row[slot * 2] = a;
                row[slot * 2 + 1] = a + 1;
                row[4 + slot * 2] = b;
                row[5 + slot * 2] = b + 1;
            }
            row
        })
        .collect()
}
fn renamed_ids(count: usize, split: &str, used: &mut BTreeSet<i64>) -> Result<Vec<[i64; 2]>> {
    let mut rng = stream(17, &format!("{DATA}/{split}/event"));
    (0..count)
        .map(|_| {
            let ids = [
                10_000_000 + (rng.next_u64() % 90_000_000) as i64,
                10_000_000 + (rng.next_u64() % 90_000_000) as i64,
            ];
            if ids.iter().any(|&id| !used.insert(id)) {
                return Err(bad("finite citation ID collision; no silent reseed"));
            }
            Ok(ids)
        })
        .collect()
}
pub(super) fn variant(
    es: &[Episode],
    ms: &[Meta],
    name: &str,
    ids: Option<&[[i64; 2]]>,
) -> Result<(Vec<Episode>, Vec<Meta>)> {
    if es.len() != ms.len() || es.len() % 4 != 0 || ids.is_some_and(|ids| ids.len() != es.len() / 4)
    {
        return Err(bad("citation variant count"));
    }
    let mut out = vec![];
    let mut metadata = vec![];
    for (i, (source, m)) in es.iter().zip(ms).enumerate() {
        if source.request.input.matches(VALUE_QUERY).count() != 1
            || resolve_request(&source.request)? != source.answer
        {
            return Err(bad("citation source request/label"));
        }
        let mut e = source.clone();
        let mut meta = m.clone();
        e.request.input = e.request.input.replace(VALUE_QUERY, CITATION_QUERY);
        if let Some(ids) = ids {
            let skeleton = semantic_skeleton(source)?;
            for r in &mut e.request.evidence.items {
                let key = parsed_record(r)?.0;
                let side = if key == format!("장치{}", skeleton[0]) {
                    0
                } else if key == format!("장치{}", skeleton[1]) {
                    1
                } else {
                    return Err(bad("citation record key"));
                };
                r.event_id = ids[i / 4][side];
            }
        }
        e.answer = resolve_request(&e.request)?;
        e.id = format!("{DATA}/{name}/{}", source.id);
        e.family = format!("{DATA}/{name}/{}", m.base);
        e.sequence = e.family.clone();
        e.binding = digest(&e.request.evidence)?;
        meta.id = e.id.clone();
        meta.base = e.family.clone();
        meta.template = name.into();
        meta.source_id = Some(source.id.clone());
        out.push(e);
        metadata.push(meta);
    }
    Ok((out, metadata))
}
fn check_variant(
    source: &[Episode],
    es: &[Episode],
    ms: &[Meta],
    tok: &ByteBpe,
    citation: bool,
) -> Result<binary::Value> {
    if source.len() != es.len() || es.len() != ms.len() || es.len() % 4 != 0 {
        return Err(bad("citation complete variant"));
    }
    let frames = samples(es, tok, 256)?;
    let mut restored = vec![];
    let mut annotations = vec![];
    for (i, ((a, b), s)) in source.iter().zip(es).zip(&frames).enumerate() {
        let gold = resolve_request(&b.request)?;
        if b.answer != gold
            || b.request.limits.max_tokens != 32
            || b.request.limits.context_tokens != 2048
            || s.tokens.len() > 256
            || s.tokens.len() - s.response_start > 32
            || s.tokens.last() != Some(&EOS)
        {
            return Err(bad("citation label/complete sequence/generation limit"));
        }
        let prompt = tok.prepare_with_framing(
            &b.request,
            neural::Framing::QuestionEvidence,
            2048,
            "citation-prepare",
        )?;
        if prompt.provided.len() != 2
            || !prompt.excluded.is_empty()
            || prompt.token_ids != s.tokens[..s.response_start]
            || tok.decode(&s.tokens[s.response_start..s.tokens.len() - 1])? != b.answer
        {
            return Err(bad("citation tokenizer/production prompt parity"));
        }
        let first_value = tok.encode(&b.answer.as_bytes()[..1])?;
        if first_value.len() != 1 || s.tokens[s.response_start] != first_value[0] {
            return Err(bad("citation first-value token boundary"));
        }
        let mut inverse = b.clone();
        if citation {
            inverse.request.input = inverse.request.input.replace(CITATION_QUERY, VALUE_QUERY);
        }
        inverse.answer = resolve_request(&inverse.request)?;
        let mut request = inverse.request.clone();
        for (r, original) in request
            .evidence
            .items
            .iter_mut()
            .zip(&a.request.evidence.items)
        {
            r.event_id = original.event_id;
        }
        if digest(&request)? != digest(&a.request)? || inverse.answer != a.answer {
            return Err(bad("citation transform outside query/IDs/answer"));
        }
        let original_tokens = tok
            .prepare_with_framing(
                &a.request,
                neural::Framing::QuestionEvidence,
                2048,
                "citation-token-diff",
            )?
            .token_ids;
        let restored_tokens = tok
            .prepare_with_framing(
                &request,
                neural::Framing::QuestionEvidence,
                2048,
                "citation-token-diff",
            )?
            .token_ids;
        if restored_tokens != original_tokens {
            return Err(bad("citation token changes outside allowed query/ID spans"));
        }
        if !citation
            && (digest(a)? != digest(b)?
                || samples(std::slice::from_ref(a), tok, 256)?[0].tokens != s.tokens)
        {
            return Err(bad("original value sample changed"));
        }
        annotations.push(binary::record!({"id":b.id,"base":ms[i].base,"orbit":semantic_skeleton(b)?,
            "assignment":ms[i].view/2,"query":ms[i].view%2,"output_mode":if citation {"VALUE_CITATION"} else {"VALUE"},
            "id_variant":ms[i].template,"source_row":a.id,"input":s.tokens.len()-1,"target":s.tokens.len()-s.response_start,
            "original_prompt_tokens":original_tokens.len(),"prompt_tokens":prompt.token_ids.len(),
            "allowed_spans_restored_token_parity":true,"source_target_tokens":tok.encode(a.answer.as_bytes())?.len()+1}));
        restored.push(inverse);
    }
    // Original IDs are shared between some old/new key assignments. Preserve
    // that history; validate each original512-row block without claiming global
    // event uniqueness. New IDs are separately globally checked before writing.
    let mut audits = vec![];
    for (es, ms) in restored.chunks(512).zip(ms.chunks(512)) {
        audits.push(orbit_validate(es, ms, tok, es.len())?);
    }
    Ok(binary::record!({"rows":annotations,"selector_audit":audits}))
}
fn make_pool(
    c: &data::native::Corpus,
    tm: &[Meta],
    dm: &[Meta],
    tok: &ByteBpe,
    reserved: &[i64],
    used_confirmation: &[i64],
) -> Result<(
    data::native::Corpus,
    (Vec<Meta>, Vec<Meta>, Vec<Meta>),
    binary::Value,
)> {
    if c.train.len() != POOL || c.validation.len() != 512 || tm.len() != POOL || dm.len() != 512 {
        return Err(bad("citation parent pool"));
    }
    let original_ids = c
        .train
        .iter()
        .chain(&c.validation)
        .flat_map(|e| e.request.evidence.items.iter().map(|r| r.event_id))
        .collect::<BTreeSet<_>>();
    let mut used = original_ids.clone();
    used.extend(reserved);
    used.extend(used_confirmation);
    let train_ids = renamed_ids(384, "train", &mut used)?;
    let dev_ids = renamed_ids(128, "dev", &mut used)?;
    let (vc0, m0) = variant(&c.train, tm, "VC0", None)?;
    let (vc1, m1) = variant(&c.train, tm, "VC1", Some(&train_ids))?;
    let (vcdev, md) = variant(&c.validation, dm, "VC-DEV", None)?;
    let (vcid, mi) = variant(&c.validation, dm, "VC-ID", Some(&dev_ids))?;
    let audits = binary::record!({"V":check_variant(&c.train,&c.train,tm,tok,false)?,
        "VC0":check_variant(&c.train,&vc0,&m0,tok,true)?,"VC1":check_variant(&c.train,&vc1,&m1,tok,true)?,
        "V-DEV":check_variant(&c.validation,&c.validation,dm,tok,false)?,
        "VC-DEV":check_variant(&c.validation,&vcdev,&md,tok,true)?,"VC-ID":check_variant(&c.validation,&vcid,&mi,tok,true)?,
        "original_distinct_ids":original_ids.len(),"original_record_slots":(384+128)*2,
        "new_train_ids":train_ids,"new_dev_ids":dev_ids,"reserved_ids":reserved,"used_confirmation_ids":used_confirmation});
    let train = [c.train.clone(), vc0, vc1].concat();
    let dev = [c.validation.clone(), vcdev, vcid].concat();
    let mut prompts = BTreeMap::new();
    for e in train.iter().chain(&dev) {
        let prompt = digest(&e.request)?;
        if prompts.insert(prompt, e.answer.clone()).is_some() {
            return Err(bad("duplicate citation prompt or conflicting gold"));
        }
    }
    let metadata = (
        [tm.to_vec(), m0, m1].concat(),
        [dm.to_vec(), md, mi].concat(),
    );
    let corpus = orbit_native(train, dev)?;
    Ok((corpus, (metadata.0, metadata.1.clone(), metadata.1), audits))
}
fn tape_cost(rows: &[[usize; 8]], es: &[Episode], tok: &ByteBpe) -> Result<binary::Value> {
    let samples = samples(es, tok, 256)?;
    let mut counts = vec![0usize; POOL * 3];
    let mut task = [[0usize; 3]; 3];
    let mut padding = 0;
    for row in rows {
        if row.iter().any(|&i| i >= samples.len() || i >= POOL * 3) {
            return Err(bad("citation tape index"));
        }
        if row
            .iter()
            .filter(|&&i| (POOL..POOL * 2).contains(&i))
            .count()
            != 2
            || row.iter().filter(|&&i| i >= POOL * 2).count() != 2
        {
            return Err(bad("citation batch VC0/VC1 balance"));
        }
        let mut bases = BTreeSet::new();
        let width = row
            .iter()
            .map(|&i| samples[i].tokens.len() - 1)
            .max()
            .unwrap();
        for (slot, pair) in row.chunks_exact(2).enumerate() {
            if pair[0] >= POOL * 3
                || pair[1] != pair[0] + 1
                || pair[0] % 2 != 0
                || (pair[0] < POOL) != (slot < 2)
                || !bases.insert(pair[0] % POOL / 4)
            {
                return Err(bad(
                    "citation batch must contain V4/VC4, four bases, both queries",
                ));
            }
            for &i in pair {
                counts[i] += 1;
                let s = &samples[i];
                let k = i / POOL;
                task[k][0] += 1;
                task[k][1] += s.tokens.len() - 1;
                task[k][2] += s.tokens.len() - s.response_start;
                padding += width - (s.tokens.len() - 1);
            }
        }
    }
    if rows.len() == UPDATES
        && counts
            .iter()
            .enumerate()
            .any(|(i, &n)| n != if i < POOL { 8 } else { 4 })
    {
        return Err(bad("citation full exposure"));
    }
    let input = task.iter().map(|s| s[1]).sum::<usize>();
    let target = task.iter().map(|s| s[2]).sum::<usize>();
    if input > 6_300_000 || target > 800_000 {
        return Err(bad("BLOCKED_PLAN: actual citation token budget"));
    }
    Ok(
        binary::record!({"updates":rows.len(),"counts":counts,"tasks_V_VC0_VC1_samples_input_target":task,"input":input,"target":target,"padding":padding}),
    )
}
fn plan(old: &Plan, state: &TrainingState, study: &Path, s: &binary::Value) -> Result<Plan> {
    let text = |key: &str| -> Result<String> {
        Ok(s[key]
            .as_str()
            .ok_or_else(|| bad("citation selection field"))?
            .into())
    };
    let mut p = old.clone();
    p.source = text("source")?;
    p.binary = text("binary")?;
    p.initial = s["parent_endpoint"]["checkpoint_hash"]
        .as_str()
        .ok_or_else(|| bad("citation parent physical"))?
        .into();
    p.initial_weights = text("parent_weights")?;
    p.corpus = text("corpus")?;
    p.transfer = p.corpus.clone();
    p.metadata = text("metadata")?;
    p.config.budget_start_step = state.step;
    p.config.budget_start_tokens = state.consumed_tokens;
    p.config.max_steps = state.step + if old.tiny { 2 } else { UPDATES };
    p.config.max_tokens = state.consumed_tokens + 6_300_000;
    p.config.warmup = 0;
    p.config.lr = 3e-4;
    let mut rows = own(old).rows[..state.step].to_vec();
    rows.extend(
        suffix()
            .into_iter()
            .take(if old.tiny { 2 } else { UPDATES }),
    );
    p.train_order = digest(&rows)?;
    p.order = vec![(0..POOL * 3).collect()];
    p.identifiable = Some(Policy {
        study: study.into(),
        arm: ARM.into(),
        dataset: DATA.into(),
        rows,
    });
    p.fork = Some(Fork {
        study: study.into(),
        study_hash: file_hash(&study.join("selection.r3b"))?,
        arm: ARM.into(),
        parent_policy: digest(old)?,
        parent_state: digest(state)?,
        parent_adam: text("parent_adam")?,
        origin_step: state.step,
        origin_input: state.consumed_tokens,
        origin_target: state.target_tokens,
        original_corpus: old.corpus.clone(),
        tokenizer_training_hash: text("tokenizer_training_hash")?,
        variants: None,
        variant_metadata: None,
        alternate_first: vec![],
        selector: None,
        selector_metadata: None,
        flip_first: vec![],
        constant_lr: 3e-4,
        target_limit: 800_000,
    });
    p.evaluation = evaluation(&p);
    Ok(p)
}
fn read_ids(path: &Path, count: usize) -> Result<Vec<i64>> {
    let r: binary::Value = read(path)?;
    let ids: Vec<i64> = binary::from_value(r["event_ids"].clone())?;
    if ids.len() != count
        || r["event_id_count"] != count
        || ids.windows(2).any(|a| a[0] >= a[1])
        || ids.iter().any(|id| !(10_000_000..=99_999_999).contains(id))
    {
        return Err(bad("citation reserved ID inventory"));
    }
    Ok(ids)
}
pub(in super::super::super) fn prepare(
    parent: &Path,
    output: &Path,
    reservation: &Path,
    used_ids: &Path,
) -> Result<()> {
    if cfg!(feature = "test-support") {
        return Err(bad("SMALL citation requires production binary"));
    }
    prepare_inner(parent, output, reservation, used_ids, false)
}
fn source_pool(
    parent: &Path,
    old: &Plan,
    tok: &ByteBpe,
) -> Result<(data::native::Corpus, Vec<Meta>, Vec<Meta>)> {
    let c = verified_corpus(&parent.join("corpus.r3cor"), &old.corpus)?;
    let (tm, dm, _) = verified_metadata(parent, old)?;
    if old.tiny {
        if !cfg!(all(test, feature = "test-support")) {
            return Err(bad("explicit TINY citation test spec required"));
        }
        let (train, tm, _) = expansion_pool_from(&c, tm, &dm, tok, &own(old).study)?;
        return Ok((orbit_native(train, c.validation)?, tm, dm));
    }
    Ok((c, tm, dm))
}
fn prepare_inner(
    parent: &Path,
    output: &Path,
    reservation: &Path,
    used_ids: &Path,
    tiny: bool,
) -> Result<()> {
    let parent = parent.canonicalize()?;
    let output = std::path::absolute(output)?;
    let old = historical_plan(&parent)?;
    if old.tiny != tiny || (tiny && !cfg!(all(test, feature = "test-support"))) {
        return Err(bad("citation parent profile"));
    }
    let end = if tiny {
        let h = history(&parent, &old)?;
        let e = h
            .last()
            .ok_or_else(|| bad("fixture parent incomplete"))?
            .clone();
        if e.step != 2
            || e.resume
            || e.phase.as_deref() != Some("Finished")
            || e.stop != "BUDGET_REACHED"
        {
            return Err(bad("fixture parent complete native endpoint required"));
        }
        e
    } else {
        let (_, end, decision) = consolidation_close_bound(&parent, old.clone())?;
        if end.step != ORIGIN
            || end.stop != "CANDIDATE_FIXED_AT_4352"
            || end.resume
            || decision["eligible"] != true
        {
            return Err(bad("accepted4352 parent required"));
        }
        let comparison = consolidation_comparison_bound(&own(&old).study, &old, &end, &decision)?;
        if confirmation_current_checked(&own(&old).study, &old, &end, &comparison)?["status"]
            != "COMPLETED_PASS"
        {
            return Err(bad("parent confirmation not verified"));
        }
        end
    };
    let loaded = checkpoint::load(&parent.join(&end.checkpoint), Device::Cpu, true)?;
    let state = loaded
        .manifest
        .training
        .as_ref()
        .ok_or_else(|| bad("citation parent Adam absent"))?;
    if state.step != end.step
        || state.sampler_state != end.step as u64
        || state.resume_binding.as_ref() != Some(&old.binding(state, &loaded.tokenizer)?)
        || old.framing() != neural::Framing::QuestionEvidence
        || old.config.first_target_weight != 1.
        || old.config.microbatch != 8
        || old.config.accumulation != 1
        || old.config.lr != 3e-4
    {
        return Err(bad("citation parent clock/objective/framing"));
    }
    let (c, tm, dm) = source_pool(&parent, &old, &loaded.tokenizer)?;
    let reserved = read_ids(reservation, 128)?;
    let used = read_ids(used_ids, 128)?;
    let (corpus, metadata, audit) = make_pool(&c, &tm, &dm, &loaded.tokenizer, &reserved, &used)?;
    let costs = tape_cost(&suffix(), &corpus.train, &loaded.tokenizer)?;
    let baseline = consolidation_baselines(&parent, &old, end.step)?;
    // Full worst-case accounting includes both full train checks, even if the
    // first is a miss, plus parent/parity/B/confirmation. No model is called here.
    let generations = 16 + 64 + 2 * 128 + 2 * 192 + 2 * 1664 + 2 * 4608 + 32 + 32 + 512;
    let teachers = 2 * 128 + 2 * 192 + 2 * 1664 + 2 * 4608;
    if generations > 16384 || teachers > 16384 {
        return Err(bad("citation planned panel budget"));
    }
    std::fs::create_dir(&output)?;
    let root = output.join(ARM);
    std::fs::create_dir(&root)?;
    data::native::write(&root.join("corpus.r3cor"), &corpus, true)?;
    copy_native(&root.join("corpus.r3cor"), &root.join("transfer.r3cor"))?;
    write(&root.join("metadata.r3b"), &metadata)?;
    copy_native(&parent.join("tokenizer.r3b"), &root.join("tokenizer.r3b"))?;
    copy_native(&parent.join(&end.checkpoint), &root.join("initial.r3m"))?;
    copy_native(reservation, &output.join("confirmation-reservation.r3b"))?;
    copy_native(used_ids, &output.join("used-confirmation-event-ids.r3b"))?;
    write(&output.join("data-audit.r3b"), &audit)?;
    let s = binary::record!({"contract":CONTRACT,"source":source_digest()?,"binary":file_hash(&std::env::current_exe()?)?,
        "parent":parent,"parent_policy":file_hash(&parent.join("plan.r3b"))?,"parent_endpoint":end,
        "parent_state":digest(state)?,"parent_content":loaded.model.weights_content_id()?,"parent_weights":loaded.model.weight_hash()?,
        "parent_adam":optimizer_hash(&loaded.optimizer)?,"tokenizer_training_hash":loaded.tokenizer.train_hash,
        "parent_acceptance":if tiny {None}else{Some(file_hash(&own(&old).study.join("confirmation-result.r3b"))?)},
        "parent_candidate":if tiny {None}else{Some(file_hash(&own(&old).study.join("confirmation-candidate.r3b"))?)},
        "parent_terminal":file_hash(&terminal_path(&parent,&end)?)?,
        "parent_baseline":baseline,"corpus":file_hash(&root.join("corpus.r3cor"))?,"metadata":file_hash(&root.join("metadata.r3b"))?,
        "reservation":file_hash(&output.join("confirmation-reservation.r3b"))?,"used_confirmation_ids":file_hash(&output.join("used-confirmation-event-ids.r3b"))?,
        "data_audit":file_hash(&output.join("data-audit.r3b"))?,"costs":costs,"planned_generation_upper":generations,
        "planned_teacher_upper":teachers,"new_updates":UPDATES,"input_limit":6300000,"target_limit":800000,
        "observer_backward":0,"origin":ORIGIN,"maximum":ORIGIN+UPDATES});
    write(&output.join("selection.r3b"), &s)?;
    let p = plan(&old, state, &output, &s)?;
    write(&root.join("plan.r3b"), &p)?;
    verify_plan(&root, &p)?;
    if !p.parent_entry(&root.join("initial.r3m"), &loaded)? {
        return Err(bad("citation native parent entry"));
    }
    publish_confirmed(
        &output.join("preparation.r3b"),
        &binary::record!({"contract":CONTRACT,"source":p.source,"binary":p.binary,
        "selection":file_hash(&output.join("selection.r3b"))?,"arms":BTreeMap::from([(ARM,binary::record!({"policy":file_hash(&root.join("plan.r3b"))?,
        "corpus":p.corpus,"metadata":p.metadata,"initial":p.initial,"tokenizer":p.tokenizer,"tape":p.train_order,"costs":costs}))]),
        "optimizer":0,"generation":0,"teacher":0,"independent_confirmation":"RESERVED_PENDING_SEAL"}),
    )?;
    println!(
        "CITATION_PREPARED parent{} tiny={tiny} V1536 VC01536 VC11536 updates0 generation0 teacher0 input={} target={} A_PENDING",
        state.step, costs["input"], costs["target"]
    );
    Ok(())
}
pub(super) fn verify_plan(root: &Path, p: &Plan) -> Result<()> {
    let o = own(p);
    let s: binary::Value = read(&o.study.join("selection.r3b"))?;
    let parent = Path::new(
        s["parent"]
            .as_str()
            .ok_or_else(|| bad("citation parent path"))?,
    );
    let old: Plan = read(&parent.join("plan.r3b"))?;
    let end: Segment = binary::from_value(s["parent_endpoint"].clone())?;
    let (native, _) = checkpoint::metadata(&root.join("initial.r3m"))?;
    let state = native
        .training
        .as_ref()
        .ok_or_else(|| bad("citation native state"))?;
    if !is(p)
        || o.arm != ARM
        || root != o.study.join(ARM)
        || *p != plan(&old, state, &o.study, &s)?
        || s["contract"] != CONTRACT
        || s["parent_policy"] != file_hash(&parent.join("plan.r3b"))?
        || state.step != p.origin_step()
        || state.sampler_state != state.step as u64
        || state.resume_binding.as_ref()
            != Some(&old.binding(state, &ByteBpe::load(&root.join("tokenizer.r3b"))?)?)
        || end.checkpoint_hash != file_hash(&parent.join(&end.checkpoint))?
        || end.checkpoint_hash != p.initial
        || (!p.tiny && (state.step != ORIGIN || end.stop != "CANDIDATE_FIXED_AT_4352"))
        || end.resume
        || end.phase.as_deref() != Some("Finished")
        || file_hash(&root.join("initial.r3m"))? != p.initial
        || file_hash(&root.join("tokenizer.r3b"))? != file_hash(&parent.join("tokenizer.r3b"))?
        || s["parent_terminal"] != file_hash(&terminal_path(parent, &end)?)?
        || digest(&read_confirmed::<Segment>(&terminal_path(parent, &end)?)?)? != digest(&end)?
        || (!p.tiny
            && (s["parent_acceptance"]
                != file_hash(&own(&old).study.join("confirmation-result.r3b"))?
                || s["parent_candidate"]
                    != file_hash(&own(&old).study.join("confirmation-candidate.r3b"))?))
        || s["reservation"] != file_hash(&o.study.join("confirmation-reservation.r3b"))?
        || s["used_confirmation_ids"]
            != file_hash(&o.study.join("used-confirmation-event-ids.r3b"))?
        || s["data_audit"] != file_hash(&o.study.join("data-audit.r3b"))?
    {
        return Err(bad("citation immutable parent/policy/data identity"));
    }
    let tok = ByteBpe::load(&root.join("tokenizer.r3b"))?;
    let (original, tm, dm) = source_pool(parent, &old, &tok)?;
    let (expected, meta, audit) = make_pool(
        &original,
        &tm,
        &dm,
        &tok,
        &read_ids(&o.study.join("confirmation-reservation.r3b"), 128)?,
        &read_ids(&o.study.join("used-confirmation-event-ids.r3b"), 128)?,
    )?;
    let actual = verified_corpus(&root.join("corpus.r3cor"), &p.corpus)?;
    if digest(&(&actual.train, &actual.validation))?
        != digest(&(&expected.train, &expected.validation))?
        || digest(&verified_metadata(root, p)?)? != digest(&meta)?
        || p.transfer != p.corpus
        || audit != read::<binary::Value>(&o.study.join("data-audit.r3b"))?
        || s["costs"] != tape_cost(&suffix(), &actual.train, &tok)?
    {
        return Err(bad("citation owned data/tape costs changed"));
    }
    Ok(())
}
#[derive(Debug, Serialize, Deserialize, PartialEq)]
struct CitationScore {
    joint: OrbitScore,
    value_correct: usize,
    citation_syntax_valid: usize,
    citation_provided: usize,
    citation_support_correct: usize,
    value_and_citation_correct: usize,
    outside_id: usize,
    first_mismatch: BTreeMap<String, usize>,
    field_errors: BTreeMap<String, usize>,
    ids: Vec<Option<i64>>,
}
fn value(text: &str) -> Option<u8> {
    text.as_bytes().first().copied().filter(u8::is_ascii_digit)
}
fn score_citation(
    es: &[Episode],
    ms: &[Meta],
    rows: &[binary::Value],
    tok: &ByteBpe,
) -> Result<CitationScore> {
    if es.len() % 4 != 0 || es.len() != rows.len() || ms.len() != es.len() {
        return Err(bad("citation complete panel"));
    }
    let full = score(rows, es, ms)?;
    let mut s = CitationScore {
        joint: OrbitScore {
            total: full.total,
            full: full.exact,
            query_both: 0,
            swap_both: 0,
            all4: 0,
            eos: full.eos,
            errors: full.errors,
            gold_foil_other_malformed: [0; 4],
            same_across_queries: 0,
            same_across_assignments: 0,
            exact: vec![],
        },
        value_correct: 0,
        citation_syntax_valid: 0,
        citation_provided: 0,
        citation_support_correct: 0,
        value_and_citation_correct: 0,
        outside_id: 0,
        first_mismatch: BTreeMap::new(),
        field_errors: BTreeMap::new(),
        ids: vec![],
    };
    for ((e, m), r) in es.iter().zip(ms).zip(rows) {
        verify_generated(r, tok)?;
        if resolve_request(&e.request)? != e.answer
            || !e.request.input.ends_with(CITATION_QUERY)
            || m.id != e.id
            || m.base != e.family
        {
            return Err(bad("citation semantic gold/group"));
        }
        let actual = r["actual"].as_str().unwrap_or("");
        let expected = citations(&e.answer)?;
        if expected.len() != 1 {
            return Err(bad("citation unique selected event"));
        }
        let parsed = citations(actual);
        let id = parsed
            .as_ref()
            .ok()
            .and_then(|ids| (ids.len() == 1).then(|| ids[0]));
        let syntax = id.is_some_and(|id| {
            (10_000_000..=99_999_999).contains(&id)
                && value(actual)
                    .is_some_and(|v| actual == format!("{}입니다. [event:{id}]", v as char))
        });
        let provided =
            id.is_some_and(|id| e.request.evidence.items.iter().any(|r| r.event_id == id));
        let support = id == Some(expected[0]);
        let v = value(actual) == value(&e.answer) && value(actual).is_some();
        let outside = parsed.as_ref().map_or(actual.contains("[event:"), |ids| {
            ids.iter()
                .any(|id| !e.request.evidence.items.iter().any(|r| r.event_id == *id))
        });
        s.value_correct += usize::from(v);
        s.citation_syntax_valid += usize::from(syntax);
        s.citation_provided += usize::from(provided);
        s.citation_support_correct += usize::from(support);
        s.value_and_citation_correct += usize::from(v && support);
        s.outside_id += usize::from(outside);
        s.ids.push(id);
        let eos = r["generation_completed"] == true
            && r["finish_reason"] == "stop"
            && r["error"].is_null();
        let exact = eos && actual == e.answer;
        s.joint.exact.push(exact);
        let other = e
            .request
            .evidence
            .items
            .iter()
            .find(|r| r.event_id != expected[0])
            .ok_or_else(|| bad("citation second record"))?;
        let foil = format!(
            "{}입니다. [event:{}]",
            parsed_record(other)?.2,
            other.event_id
        );
        let class = if exact {
            0
        } else if eos && actual == foil {
            1
        } else if eos && syntax {
            2
        } else {
            3
        };
        s.joint.gold_foil_other_malformed[class] += 1;
        let timeout = r["error_class"] == "timeout"
            || r["error"]
                .as_str()
                .is_some_and(|v| v.to_ascii_lowercase().contains("timeout"));
        let utf8 = r["error_class"] == "strict_utf8";
        let first = if exact {
            "none"
        } else if timeout {
            "timeout"
        } else if utf8 {
            "UTF8"
        } else if !eos {
            "EOS"
        } else if !v {
            "value"
        } else if !syntax {
            "format"
        } else {
            "citation_digit"
        };
        *s.first_mismatch.entry(first.into()).or_default() += 1;
        for (field, error) in [
            ("value", !v),
            ("citation_digit", !support),
            ("format", !syntax),
            ("EOS", !eos),
            ("timeout", timeout),
            ("UTF8", utf8),
        ] {
            *s.field_errors.entry(field.into()).or_default() += usize::from(error);
        }
    }
    for ((q, r), m) in s
        .joint
        .exact
        .chunks_exact(4)
        .zip(rows.chunks_exact(4))
        .zip(ms.chunks_exact(4))
    {
        if m.iter()
            .enumerate()
            .any(|(i, v)| v.view != i || v.base != m[0].base)
        {
            return Err(bad("citation ALL4 order"));
        }
        s.joint.query_both += usize::from(q[0] && q[1]) + usize::from(q[2] && q[3]);
        s.joint.swap_both += usize::from(q[0] && q[2]) + usize::from(q[1] && q[3]);
        s.joint.all4 += usize::from(q.iter().all(|&v| v));
        for (a, b) in [(0, 1), (2, 3)] {
            s.joint.same_across_queries +=
                usize::from(r[a]["actual"].is_string() && r[a]["actual"] == r[b]["actual"]);
        }
        for (a, b) in [(0, 2), (1, 3)] {
            s.joint.same_across_assignments +=
                usize::from(r[a]["actual"].is_string() && r[a]["actual"] == r[b]["actual"]);
        }
    }
    Ok(s)
}
fn pass(s: &OrbitScore, total: usize, full: usize, qb: usize, all4: usize) -> bool {
    s.total == total
        && s.full >= full
        && s.query_both >= qb
        && s.all4 >= all4
        && s.errors == 0
        && s.eos == total
}
type Panel = (String, Vec<Episode>, Vec<Meta>);
fn base_panels(root: &Path, p: &Plan, step: usize) -> Result<Vec<Panel>> {
    if !p.evaluation_due(step) {
        return Err(bad("citation unregistered evaluation"));
    }
    let c = verified_corpus(&root.join("corpus.r3cor"), &p.corpus)?;
    let (tm, dm, _) = verified_metadata(root, p)?;
    if c.train.len() != POOL * 3
        || c.validation.len() != 1536
        || dm.len() != 1536
        || tm.len() != POOL * 3
    {
        return Err(bad("citation frozen full panel pool"));
    }
    let offset = step - p.origin_step();
    let full = !p.tiny && [1536, 3072].contains(&offset);
    let n = if p.tiny {
        4
    } else if full {
        512
    } else {
        64
    };
    let mut out = vec![
        (
            format!("value{n}"),
            c.validation[..n].to_vec(),
            dm[..n].to_vec(),
        ),
        (
            format!("citation{n}"),
            c.validation[512..512 + n].to_vec(),
            dm[512..512 + n].to_vec(),
        ),
    ];
    if offset >= 384 || p.tiny {
        out.push((
            format!("renamed{n}"),
            c.validation[1024..1024 + n].to_vec(),
            dm[1024..1024 + n].to_vec(),
        ));
    }
    if full {
        out.push((
            "citation-train128".into(),
            [
                c.train[POOL..POOL + 64].to_vec(),
                c.train[POOL * 2..POOL * 2 + 64].to_vec(),
            ]
            .concat(),
            [
                tm[POOL..POOL + 64].to_vec(),
                tm[POOL * 2..POOL * 2 + 64].to_vec(),
            ]
            .concat(),
        ));
    }
    Ok(out)
}
fn fit_panels(root: &Path, p: &Plan) -> Result<Vec<Panel>> {
    let c = verified_corpus(&root.join("corpus.r3cor"), &p.corpus)?;
    let (tm, _, _) = verified_metadata(root, p)?;
    Ok([
        ("old512", 0, 512),
        ("new1024", 512, POOL),
        ("VC0train1536", POOL, POOL * 2),
        ("VC1train1536", POOL * 2, POOL * 3),
    ]
    .into_iter()
    .map(|(name, a, b)| (name.into(), c.train[a..b].to_vec(), tm[a..b].to_vec()))
    .collect())
}
fn read_score(root: &Path, p: &Plan, step: usize, panel: &Panel) -> Result<binary::Value> {
    let (name, es, ms) = panel;
    let tok = ByteBpe::load(&root.join("tokenizer.r3b"))?;
    let summary = audit_panel(root, p, step, name, es, ms, &tok)?;
    let raw = binary::read_value_records(&root.join(format!("eval-{step:04}-{name}.r3rows")))?;
    let is_v = es[0].request.input.ends_with(VALUE_QUERY);
    let mut out = if is_v {
        binary::record!({"joint":orbit_score(es,ms,&raw[1..],&tok)?})
    } else {
        binary::to_value(score_citation(es, ms, &raw[1..], &tok)?)?
    };
    let teachers =
        binary::read_value_records(&root.join(format!("eval-{step:04}-{name}-teachers.r3rows")))?;
    let mut value_nll = 0.;
    let mut cite_nll = 0.;
    let mut eos_nll = 0.;
    let mut citation_tokens = 0;
    let mut id_nll = 0.;
    let mut id_tokens = 0;
    for (r, e) in teachers[1..].iter().zip(es) {
        let t = &r["teacher"]["target_token_observation"];
        let n: Vec<f64> = binary::from_value(t["nll"].clone())?;
        let mut gold = tok.encode(e.answer.as_bytes())?;
        gold.push(EOS);
        if t["gold"] != binary::record!(gold)
            || n.len() != gold.len()
            || n.len() < 2
            || n.iter().any(|v| !v.is_finite())
        {
            return Err(bad("citation teacher actual target tokens"));
        }
        value_nll += n[0];
        eos_nll += n[n.len() - 1];
        cite_nll += n[1..n.len() - 1].iter().sum::<f64>();
        citation_tokens += n.len() - 2;
        if let Some(start) = e.answer.find("[event:").map(|p| p + 7) {
            let end = e.answer.len() - 1;
            let mut offset = 0;
            for (&token, &loss) in gold[..gold.len() - 1].iter().zip(&n) {
                let size = tok.decode_bytes(&[token])?.len();
                if offset < end && offset + size > start {
                    id_nll += loss;
                    id_tokens += 1;
                }
                offset += size;
            }
            if offset != e.answer.len() {
                return Err(bad("citation teacher token byte spans"));
            }
        }
    }
    out["model"] = binary::record!(summary.model);
    out["raw"] = binary::record!(summary.raw_hash);
    out["cases"] = binary::record!(digest(es)?);
    out["first_value_nll"] = binary::record!(value_nll / es.len() as f64);
    out["eos_nll"] = binary::record!(eos_nll / es.len() as f64);
    out["citation_target_nll"] = if citation_tokens == 0 {
        binary::Value::Null
    } else {
        binary::record!(cite_nll / citation_tokens as f64)
    };
    out["citation_target_tokens"] = binary::record!(citation_tokens);
    out["citation_target_scope"] =
        binary::record!("all answer suffix tokens after first value, before EOS");
    out["citation_id_nll"] = if id_tokens == 0 {
        binary::Value::Null
    } else {
        binary::record!(id_nll / id_tokens as f64)
    };
    out["citation_id_tokens"] = binary::record!(id_tokens);
    Ok(out)
}
fn dev_pass(scores: &BTreeMap<String, binary::Value>) -> Result<bool> {
    let joint: OrbitScore = binary::from_value(scores["value512"]["joint"].clone())?;
    // The complete diagnostic record has extra provenance/NLL fields; deserialize
    // only the fixed metric keys rather than trusting a stored gate boolean.
    let citation = |key: &str| -> Result<bool> {
        let v = &scores[key];
        let joint: OrbitScore = binary::from_value(v["joint"].clone())?;
        Ok(pass(&joint, 512, 488, 232, 116)
            && v["value_correct"].as_u64().is_some_and(|v| v >= 508)
            && v["citation_support_correct"]
                .as_u64()
                .is_some_and(|v| v >= 508)
            && v["outside_id"] == 0)
    };
    Ok(pass(&joint, 512, 488, 232, 116) && citation("citation512")? && citation("renamed512")?)
}
pub(super) fn panels(root: &Path, p: &Plan, step: usize) -> Result<Vec<Panel>> {
    let mut out = base_panels(root, p, step)?;
    if !p.tiny && [1536, 3072].contains(&(step - p.origin_step())) {
        let present = out[..3]
            .iter()
            .all(|(name, _, _)| root.join(format!("eval-{step:04}-{name}.r3b")).exists());
        if present {
            let scores = out[..3]
                .iter()
                .map(|panel| Ok((panel.0.clone(), read_score(root, p, step, panel)?)))
                .collect::<Result<BTreeMap<_, _>>>()?;
            if dev_pass(&scores)? {
                out.extend(fit_panels(root, p)?);
            }
        }
    }
    Ok(out)
}
fn evaluation_result(root: &Path, p: &Plan, step: usize) -> Result<binary::Value> {
    let cases = panels(root, p, step)?;
    let scores = cases
        .iter()
        .map(|panel| Ok((panel.0.clone(), read_score(root, p, step, panel)?)))
        .collect::<Result<BTreeMap<_, _>>>()?;
    let models = scores
        .values()
        .map(|s| {
            s["model"]
                .as_str()
                .ok_or_else(|| bad("citation panel model"))
        })
        .collect::<Result<BTreeSet<_>>>()?;
    if models.len() != 1 {
        return Err(bad("mixed citation endpoint models"));
    }
    let value_panel = &cases[0];
    let tok = ByteBpe::load(&root.join("tokenizer.r3b"))?;
    let raw =
        binary::read_value_records(&root.join(format!("eval-{step:04}-{}.r3rows", value_panel.0)))?;
    let n = if p.tiny { 4 } else { 64 };
    let screen = orbit_score(
        &value_panel.1[..n],
        &value_panel.2[..n],
        &raw[1..n + 1],
        &tok,
    )?;
    let selection: binary::Value = read(&own(p).study.join("selection.r3b"))?;
    let baseline: OrbitScore =
        binary::from_value(selection["parent_baseline"]["dev"]["screen"].clone())?;
    if baseline.total != n {
        return Err(bad("citation retention denominator"));
    }
    let regression = baseline.full.saturating_sub(screen.full) >= 16
        || baseline.all4.saturating_sub(screen.all4) >= 4
        || screen.errors >= 4;
    let full = !p.tiny && [1536, 3072].contains(&(step - p.origin_step()));
    let development = full && dev_pass(&scores)?;
    let joint = |key: &str| -> Result<OrbitScore> {
        binary::from_value(
            scores.get(key).ok_or_else(|| bad("missing fit panel"))?["joint"].clone(),
        )
        .map_err(Into::into)
    };
    let fits = if development {
        pass(&joint("old512")?, 512, 508, 252, 124)
            && pass(&joint("new1024")?, 1024, 1016, 504, 248)
            && pass(&joint("VC0train1536")?, 1536, 1524, 756, 372)
            && pass(&joint("VC1train1536")?, 1536, 1524, 756, 372)
    } else {
        false
    };
    let eligible = development && fits && !regression;
    let action = if regression {
        "QUALITY_REGRESSION".to_string()
    } else if eligible {
        format!("CANDIDATE_FIXED_AT_{step}")
    } else if step == p.config.max_steps {
        format!("FINAL_QUALITY_FAIL_AT_{step}")
    } else {
        "CONTINUE_WITHIN_REGISTERED_CAP".into()
    };
    let extend = !regression && !eligible && step < p.config.max_steps;
    let renamed = cases
        .iter()
        .find(|(name, _, _)| name.starts_with("renamed"));
    let id_pairs = if let Some((name, es, ms)) = renamed {
        let n = es.len();
        let citation_name = format!("citation{n}");
        let a = &scores[&citation_name];
        let b = &scores[name];
        let ae: Vec<bool> = binary::from_value(a["joint"]["exact"].clone())?;
        let be: Vec<bool> = binary::from_value(b["joint"]["exact"].clone())?;
        let ai: Vec<Option<i64>> = binary::from_value(a["ids"].clone())?;
        let bi: Vec<Option<i64>> = binary::from_value(b["ids"].clone())?;
        if ae.len() != n || be.len() != n || ai.len() != n || bi.len() != n || ms.len() != n {
            return Err(bad("ID paired denominator"));
        }
        binary::record!({"count":n,"ID_BOTH":ae.iter().zip(&be).filter(|(a,b)|**a&&**b).count(),
            "same_cited_id_when_id_changed":ai.iter().zip(&bi).filter(|(a,b)|a.is_some()&&a==b).count(),"scope":"same semantic cases, changed event IDs; not SWAP_BOTH"})
    } else {
        binary::Value::Null
    };
    Ok(
        binary::record!({"policy":digest(p)?,"step":step,"model":models.into_iter().next(),"panels":scores,
        "V64":screen,"parent_V64":baseline,"ID_pairs":id_pairs,"development":development,"fit":fits,"eligible":eligible,
        "regression":regression,"action":action,"extend":extend,"stop":(!extend).then_some(action)}),
    )
}
pub(super) fn evaluate(
    p: &Plan,
    root: &Path,
    path: &Path,
    step: usize,
    control: &mut recovery::RunControl,
) -> Result<Option<String>> {
    for (name, es, ms) in base_panels(root, p, step)? {
        evaluate_panel(p, root, path, step, &name, &es, &ms, control)?;
    }
    // Recompute development from its complete raw before scheduling fit panels.
    for (name, es, ms) in panels(root, p, step)?.into_iter().filter(|(n, _, _)| {
        ["old512", "new1024", "VC0train1536", "VC1train1536"].contains(&n.as_str())
    }) {
        control.check("citation_conditional_fit")?;
        evaluate_panel(p, root, path, step, &name, &es, &ms, control)?;
    }
    let d = evaluation_result(root, p, step)?;
    let path = root.join(format!("citation-decision-{step:04}.r3b"));
    if path.exists() {
        if read_confirmed::<binary::Value>(&path)? != d {
            return Err(bad("citation decision/raw disagreement"));
        }
    } else {
        publish_confirmed(&path, &d)?;
    }
    println!("CITATION_DECISION {d}");
    Ok(d["stop"].as_str().map(str::to_owned))
}
fn trace(root: &Path, p: &Plan, end: &Segment) -> Result<binary::Value> {
    let c = verified_corpus(&root.join("corpus.r3cor"), &p.corpus)?;
    let tok = ByteBpe::load(&root.join("tokenizer.r3b"))?;
    let samples = samples(&c.train, &tok, p.config.seq_len)?;
    let mut cursor = p.origin_step();
    let mut input = 0u64;
    let mut target = 0u64;
    let mut counts = vec![0usize; c.train.len()];
    let mut task = [[0usize; 3]; 3];
    let mut files = BTreeMap::new();
    for index in 0..128 {
        let terminal = root.join(format!("segment-{index:04}-finished.r3b"));
        if !terminal.exists() {
            break;
        }
        let segment: Segment = read_confirmed(&terminal)?;
        let path = root.join(format!("segment-{index:04}/updates.r3rows"));
        if path.exists() {
            for r in binary::read_value_records(&path)? {
                if cursor >= p.config.max_steps {
                    return Err(bad("citation trace exceeds cap"));
                }
                let draw = p.training_draw(cursor);
                let (mut ni, mut nt) = (0, 0);
                for &i in &draw {
                    let s = samples
                        .get(i)
                        .ok_or_else(|| bad("citation trace sample index"))?;
                    let (a, b) = (s.tokens.len() - 1, s.tokens.len() - s.response_start);
                    ni += a;
                    nt += b;
                    counts[i] += 1;
                    task[i / POOL][0] += 1;
                    task[i / POOL][1] += a;
                    task[i / POOL][2] += b;
                }
                cursor += 1;
                if r["step"] != cursor
                    || r["sampler"] != cursor
                    || r["sample_indices"] != binary::record!(draw)
                    || r["lr_bits"] != p.learning_rate(cursor).to_bits()
                    || r["input"] != ni
                    || r["target"] != nt
                    || ["ce", "objective", "grad_norm", "delta_norm"]
                        .iter()
                        .any(|key| r[*key].as_f64().is_none_or(|v| !v.is_finite()))
                {
                    return Err(bad("citation actual trace/LR/token/gradient mismatch"));
                }
                input += ni as u64;
                target += nt as u64;
            }
            files.insert(index, file_hash(&path)?);
        }
        if cursor != segment.step {
            return Err(bad("citation trace missing committed step"));
        }
    }
    let f = p.fork.as_ref().ok_or_else(|| bad("citation fork"))?;
    if cursor != end.step
        || f.origin_input + input != end.input
        || f.origin_target + target != end.target
    {
        return Err(bad("citation native cumulative trace mismatch"));
    }
    Ok(
        binary::record!({"files":files,"updates":cursor-p.origin_step(),"step":cursor,"input":input,"target":target,
        "counts":counts,"tasks_V_VC0_VC1_samples_input_target":task,"unique_V_VC0_VC1":counts.chunks(POOL).map(|c|c.iter().filter(|&&n|n>0).count()).collect::<Vec<_>>()}),
    )
}
fn seal_receipt(study: &Path) -> Result<binary::Value> {
    let s: binary::Value = read_confirmed(&study.join("confirmation-seal.r3b"))?;
    if s["preparation"] != file_hash(&study.join("preparation.r3b"))?
        || s["selection"] != file_hash(&study.join("selection.r3b"))?
        || s["reservation"] != file_hash(&study.join("confirmation-reservation.r3b"))?
        || s["count"] != 512
        || s["skeletons"] != 64
        || s["corpus"] != file_hash(&study.join("sealed/confirmation.r3cor"))?
        || s["metadata"] != file_hash(&study.join("sealed/metadata.r3b"))?
    {
        return Err(bad("citation independent seal binding"));
    }
    Ok(s)
}
fn observed(
    study: &Path,
    name: &str,
    p: &Plan,
    checkpoint: &str,
    es: &[Episode],
    matched: bool,
) -> Result<Vec<binary::Value>> {
    let r: binary::Value = read_confirmed(&study.join(format!("{name}-finished.r3b")))?;
    if r["policy"] != digest(p)?
        || r["checkpoint"] != checkpoint
        || r["binding"]["identity"]["policy"] != digest(p)?
        || r["binding"]["checkpoint"] != checkpoint
        || r["binding"]["source"] != p.source
        || r["binding"]["binary"] != p.binary
        || r["binding"]["cases"] != digest(&es)?
        || r["completed"] != es.len()
        || (matched && r["matched"] != es.len())
        || r["control"]["generation_calls"] != es.len()
        || r["control"]["teacher_calls"] != 0
        || r["control"]["terminal_reason"] != "COMPLETED"
        || !r["error"].is_null()
        || r["raw"] != file_hash(&study.join(format!("{name}.r3rows")))?
    {
        return Err(bad("citation observation receipt"));
    }
    let rows = binary::read_value_records(&study.join(format!("{name}.r3rows")))?;
    if rows.len() != es.len() + 1 || rows[0] != r["binding"] {
        return Err(bad("citation observation complete raw"));
    }
    for (i, (e, row)) in es.iter().zip(&rows[1..]).enumerate() {
        call_attempt(study, name, "generation", &rows[0], e, i, Some(row))?;
    }
    Ok(rows[1..].to_vec())
}
pub(super) fn authorize(root: &Path, p: &Plan) -> Result<()> {
    let study = &own(p).study;
    if root.canonicalize()? != study.join(ARM).canonicalize()? {
        return Err(bad("citation registered root"));
    }
    let prep: binary::Value = read_confirmed(&study.join("preparation.r3b"))?;
    if prep["selection"] != file_hash(&study.join("selection.r3b"))? {
        return Err(bad("citation reviewed selection"));
    }
    seal_receipt(study)?;
    let c = verified_corpus(&root.join("corpus.r3cor"), &p.corpus)?;
    let n = if p.tiny { 4 } else { 16 };
    let nc = if p.tiny { 4 } else { 64 };
    observed(study, "legacy", p, &p.initial, &c.validation[..n], true)?;
    observed(
        study,
        "citation-parent",
        p,
        &p.initial,
        &c.validation[512..512 + nc],
        false,
    )?;
    Ok(())
}
pub(in super::super::super) fn parent_observe(study: &Path, citation: bool) -> Result<()> {
    let study = study.canonicalize()?;
    let root = study.join(ARM);
    let p = plan_read(&root)?;
    expansion_review(&p)?;
    seal_receipt(&study)?;
    if !history(&root, &p)?.is_empty() {
        return Err(bad("citation parent observation must precede updates"));
    }
    let c = verified_corpus(&root.join("corpus.r3cor"), &p.corpus)?;
    let (_, dm, _) = verified_metadata(&root, &p)?;
    let n = if p.tiny {
        4
    } else if citation {
        64
    } else {
        16
    };
    let start = if citation { 512 } else { 0 };
    let s: binary::Value = read(&study.join("selection.r3b"))?;
    let parent = Path::new(
        s["parent"]
            .as_str()
            .ok_or_else(|| bad("citation parent path"))?,
    );
    let old: Plan = read(&parent.join("plan.r3b"))?;
    let raw = if citation {
        None
    } else {
        Some(binary::read_value_records(&parent.join(format!(
            "eval-{:04}-{}.r3rows",
            p.origin_step(),
            if old.tiny { "dev4" } else { "dev512" }
        )))?)
    };
    let name = if citation {
        "citation-parent"
    } else {
        "legacy"
    };
    let es = &c.validation[start..start + n];
    let ms = &dm[start..start + n];
    let rows = orbit_observe(
        &study,
        name,
        &root.join("initial.r3m"),
        es,
        &binary::record!({"policy":digest(&p)?,"checkpoint":p.initial}),
        raw.as_ref().map(|r| &r[1..n + 1]),
        observation_control(&p, n, 0)?,
    )?;
    let tok = ByteBpe::load(&root.join("tokenizer.r3b"))?;
    let score = if citation {
        binary::to_value(score_citation(es, ms, &rows, &tok)?)?
    } else {
        binary::to_value(orbit_score(es, ms, &rows, &tok)?)?
    };
    publish_confirmed(&study.join(format!("{name}-result.r3b")), &score)?;
    println!("CITATION_PARENT citation={citation} generation={n} optimizer0 teacher0 {score}");
    Ok(())
}
fn close(root: &Path, p: &Plan) -> Result<(Segment, binary::Value)> {
    super::authorize(root, p)?;
    let h = history(root, p)?;
    let end = h
        .last()
        .ok_or_else(|| bad("citation endpoint absent"))?
        .clone();
    if end.resume || end.phase.as_deref() != Some("Finished") || !p.evaluation_due(end.step) {
        return Err(bad("citation endpoint incomplete"));
    }
    let d = evaluation_result(root, p, end.step)?;
    if d != read_confirmed::<binary::Value>(
        &root.join(format!("citation-decision-{:04}.r3b", end.step)),
    )? || d["stop"] != end.stop
    {
        return Err(bad("citation close terminal/raw mismatch"));
    }
    trace(root, p, &end)?;
    Ok((end, d))
}
pub(super) fn gate(root: &Path, p: &Plan) -> Result<bool> {
    let (_, d) = close(root, p)?;
    Ok(d["eligible"] == true)
}
pub(in super::super::super) fn report(study: &Path) -> Result<()> {
    let study = study.canonicalize()?;
    let root = study.join(ARM);
    let p = historical_plan(&root)?;
    let h = history(&root, &p)?;
    let Some(end) = h.last() else {
        println!("CITATION NOT_RUN");
        return Ok(());
    };
    for &step in p.evaluation.train_steps.iter().filter(|&&s| s <= end.step) {
        if step == end.step && end.phase.as_deref() == Some("EvaluationPending") {
            continue;
        }
        let d = evaluation_result(&root, &p, step)?;
        if d != read_confirmed::<binary::Value>(
            &root.join(format!("citation-decision-{step:04}.r3b")),
        )? {
            return Err(bad("citation report/raw disagreement"));
        }
        println!("CITATION_PANEL {d}");
    }
    println!("CITATION_TRACE {}", trace(&root, &p, end)?);
    println!(
        "CITATION_STATE step={} durable={} stop={} resume={} usage={:?} Goal1=false",
        end.step,
        end.checkpoint_hash,
        end.stop,
        end.resume,
        work(&p)?
    );
    if !end.resume && end.phase.as_deref() == Some("Finished") {
        let (_, d) = close(&root, &p)?;
        println!("CITATION_FINAL {d}");
        println!("CITATION_COMPARISON {}", comparison(&study, &p, end, &d)?);
        if study.join("confirmation-result.r3b").exists() {
            let actual = verified_confirmation(&study, &p, end, &d)?;
            println!("CITATION_CONFIRMATION_CURRENT {actual}");
        } else {
            println!(
                "CITATION_CONFIRMATION_CURRENT {}",
                if study.join("confirmation-candidate.r3b").exists() {
                    "PENDING_OR_FAILED_EXECUTION"
                } else if d["eligible"] == true {
                    "NOT_OPENED"
                } else {
                    "NOT_ELIGIBLE"
                }
            );
        }
    }
    Ok(())
}
pub(in super::super::super) fn parity(study: &Path, citation: bool, reviewer: bool) -> Result<()> {
    let study = study.canonicalize()?;
    let root = study.join(ARM);
    let p = historical_plan(&root)?;
    let (end, d) = close(&root, &p)?;
    if d["regression"] != false
        || !(d["eligible"] == true
            || (reviewer
                && end.step == p.config.max_steps
                && end.stop == format!("FINAL_QUALITY_FAIL_AT_{}", end.step)))
    {
        return Err(bad(
            "citation normal complete reproduction; no resume/candidate permission",
        ));
    }
    let name = match (citation, reviewer) {
        (false, false) => "citation-parity-value",
        (true, false) => "citation-parity-citation",
        (false, true) => "citation-review-value",
        (true, true) => "citation-review-citation",
    };
    let panel = panels(&root, &p, end.step)?
        .into_iter()
        .find(|(name, _, _)| {
            name == if p.tiny {
                if citation { "citation4" } else { "value4" }
            } else if citation {
                "citation512"
            } else {
                "value512"
            }
        })
        .ok_or_else(|| bad("citation parity panel"))?;
    let n = panel.1.len().min(16);
    let raw =
        binary::read_value_records(&root.join(format!("eval-{:04}-{}.r3rows", end.step, panel.0)))?;
    orbit_observe(
        &study,
        name,
        &root.join(&end.checkpoint),
        &panel.1[..n],
        &binary::record!({"policy":digest(&p)?,"checkpoint":end.checkpoint_hash,"reviewer":reviewer}),
        Some(&raw[1..n + 1]),
        observation_control(&p, n, 0)?,
    )?;
    println!(
        "CITATION_PARITY reviewer={reviewer} citation={citation} matched{n}/{n} generation{n} teacher0 optimizer0 candidate={}",
        d["eligible"]
    );
    Ok(())
}
// Only the independent preparer calls this before learning. The public plan
// binds the reservation; the implementation never opens the private selection.
pub(in super::super::super) fn seal(study: &Path, private: &Path) -> Result<()> {
    let study = study.canonicalize()?;
    let root = study.join(ARM);
    let p = plan_read(&root)?;
    if root.join("segment-0000-started.r3b").exists() {
        return Err(bad("citation seal must precede learning"));
    }
    let public: binary::Value = read(&study.join("confirmation-reservation.r3b"))?;
    let r: binary::Value = read(private)?;
    let sc: Vec<Skeleton> = binary::from_value(r["skeletons"].clone())?;
    let ids: Vec<[i64; 2]> = binary::from_value(r["event_pairs"].clone())?;
    let mut flat = ids.iter().flatten().copied().collect::<Vec<_>>();
    flat.sort_unstable();
    if sc.len() != 64
        || ids.len() != 64
        || digest(&sc)? != public["skeleton_digest"]
        || flat != read_ids(&study.join("confirmation-reservation.r3b"), 128)?
        || public["reviewer_private_file_hash"] != file_hash(private)?
    {
        return Err(bad("citation private/public reservation mismatch"));
    }
    let c = verified_corpus(&root.join("corpus.r3cor"), &p.corpus)?;
    let tok = ByteBpe::load(&root.join("tokenizer.r3b"))?;
    let forbidden = c
        .train
        .iter()
        .chain(&c.validation)
        .map(semantic_skeleton)
        .collect::<Result<BTreeSet<_>>>()?;
    if sc.iter().any(|s| forbidden.contains(s)) || sc.iter().collect::<BTreeSet<_>>().len() != 64 {
        return Err(bad("citation confirmation semantic leak"));
    }
    let used = c
        .train
        .iter()
        .chain(&c.validation)
        .flat_map(|e| e.request.evidence.items.iter().map(|r| r.event_id))
        .collect::<BTreeSet<_>>();
    if flat.iter().any(|id| used.contains(id)) {
        return Err(bad("citation confirmation event leak"));
    }
    let (mut v, vm) = heldout_cases(&sc, "value-citation-confirmation", &c.train)?;
    for (i, e) in v.iter_mut().enumerate() {
        for event in &mut e.request.evidence.items {
            let side = usize::from(parsed_record(event)?.0 != format!("장치{}", sc[i / 4][0]));
            event.event_id = ids[i / 4][side];
        }
        e.binding = digest(&e.request.evidence)?;
    }
    orbit_validate(&v, &vm, &tok, 256)?;
    let (vc, cm) = variant(&v, &vm, "confirmation", None)?;
    check_variant(&v, &vc, &cm, &tok, true)?;
    let es = [v, vc].concat();
    let ms = [vm, cm].concat();
    std::fs::create_dir(study.join("sealed"))?;
    data::native::write(
        &study.join("sealed/confirmation.r3cor"),
        &orbit_native(c.train, es)?,
        true,
    )?;
    write(&study.join("sealed/metadata.r3b"), &ms)?;
    publish_confirmed(
        &study.join("confirmation-seal.r3b"),
        &binary::record!({"preparation":file_hash(&study.join("preparation.r3b"))?,
        "selection":file_hash(&study.join("selection.r3b"))?,"reservation":file_hash(&study.join("confirmation-reservation.r3b"))?,
        "corpus":file_hash(&study.join("sealed/confirmation.r3cor"))?,"metadata":file_hash(&study.join("sealed/metadata.r3b"))?,
        "count":512,"skeletons":64,"skeleton_digest":digest(&sc)?,"new_optimizer":0,"generation":0,"teacher":0}),
    )?;
    println!("CITATION_SEALED V256 VC256 skeletons64 calls0");
    Ok(())
}
fn comparison(study: &Path, p: &Plan, end: &Segment, d: &binary::Value) -> Result<binary::Value> {
    let endpoint = binary::record!({"policy":digest(p)?,"checkpoint":end.checkpoint_hash,"step":end.step,"decision":d});
    Ok(
        binary::record!({"contract":CONTRACT,"source":p.source,"preparation":file_hash(&study.join("preparation.r3b"))?,
        "endpoints":BTreeMap::from([(ARM,endpoint)]),"selected":if d["eligible"]==true {Some(ARM)}else{None},"goal1_ready":false}),
    )
}
pub(in super::super::super) fn confirm(study: &Path) -> Result<()> {
    let study = study.canonicalize()?;
    let root = study.join(ARM);
    let p = plan_read(&root)?;
    let (end, d) = close(&root, &p)?;
    if d["eligible"] != true || p.tiny {
        return Err(bad("NOT_RUN_PREREQUISITE: no value/citation candidate"));
    }
    let comparison = comparison(&study, &p, &end, &d)?;
    let b: binary::Value = read_confirmed(&study.join("review-b.r3b"))?;
    if b["verdict"] != "PASS"
        || b["preparation"] != file_hash(&study.join("preparation.r3b"))?
        || b["endpoints"] != comparison["endpoints"]
        || b["report_hash"]
            != file_hash(Path::new(
                b["report_path"]
                    .as_str()
                    .ok_or_else(|| bad("citation B report"))?,
            ))?
    {
        return Err(bad("independent citation B required"));
    }
    let c = verified_corpus(&root.join("corpus.r3cor"), &p.corpus)?;
    for (name, es) in [
        ("citation-review-value", &c.validation[..16]),
        ("citation-review-citation", &c.validation[512..528]),
    ] {
        observed(&study, name, &p, &end.checkpoint_hash, es, true)?;
    }
    let seal = seal_receipt(&study)?;
    write(
        &study.join("confirmation-candidate.r3b"),
        &binary::record!({"checkpoint":end.checkpoint_hash,"comparison":comparison,
        "seal":file_hash(&study.join("confirmation-seal.r3b"))?,"review":file_hash(&study.join("review-b.r3b"))?,"framing":p.framing()}),
    )?;
    let c = verified_corpus(
        &study.join("sealed/confirmation.r3cor"),
        seal["corpus"]
            .as_str()
            .ok_or_else(|| bad("citation seal corpus"))?,
    )?;
    let ms: Vec<Meta> = read(&study.join("sealed/metadata.r3b"))?;
    let es = c.validation;
    if es.len() != 512 || ms.len() != 512 {
        return Err(bad("complete V256/VC256 confirmation required"));
    }
    let tok = ByteBpe::load(&root.join("tokenizer.r3b"))?;
    orbit_validate(&es[..256], &ms[..256], &tok, 256)?;
    check_variant(&es[..256], &es[256..], &ms[256..], &tok, true)?;
    let rows = orbit_observe(
        &study,
        "confirmation",
        &root.join(&end.checkpoint),
        &es,
        &binary::record!({"policy":digest(&p)?,"checkpoint":end.checkpoint_hash,"candidate":file_hash(&study.join("confirmation-candidate.r3b"))?}),
        None,
        observation_control(&p, 512, 0)?,
    )?;
    let v = orbit_score(&es[..256], &ms[..256], &rows[..256], &tok)?;
    let vc = score_citation(&es[256..], &ms[256..], &rows[256..], &tok)?;
    let passed = pass(&v, 256, 244, 116, 58)
        && pass(&vc.joint, 256, 244, 116, 58)
        && vc.value_correct >= 254
        && vc.citation_support_correct >= 254
        && vc.outside_id == 0;
    publish_confirmed(
        &study.join("confirmation-result.r3b"),
        &binary::record!({"checkpoint":end.checkpoint_hash,"V":v,"VC":vc,
        "value_citation_baseline_verified":passed,"scope":"two current records, one-digit key/value, fixed grammar, eight-digit event ID",
        "goal1_ready":false,"goal1_accepted":false,"s4":false,"s5":false,"s6":false}),
    )?;
    println!(
        "CITATION_CONFIRMATION value={} citation={} support={} baseline={passed} Goal1=false",
        v.full, vc.joint.full, vc.citation_support_correct
    );
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    fn gold(e: &Episode, tok: &ByteBpe) -> Result<binary::Value> {
        let tokens = tok.encode(e.answer.as_bytes())?;
        let mut raw = tokens.clone();
        raw.push(EOS);
        Ok(
            binary::record!({"row_version":2,"id":e.id,"question":e.request.input,"generated_evidence":e.request.evidence,
            "expected":e.answer,"actual":e.answer,"raw_tokens":raw,"finish_reason":"stop","generation_completed":true,
            "generation":{"tokens":tokens,"generated":raw.len(),"finish":"stop"},"error":null,"exact_match":true}),
        )
    }
    fn fixture_reservation(parent: &Path, dir: &Path) -> Result<(PathBuf, PathBuf, PathBuf)> {
        let old = plan_read(parent)?;
        let tok = ByteBpe::load(&parent.join("tokenizer.r3b"))?;
        let (c, _, _) = source_pool(parent, &old, &tok)?;
        let excluded = c
            .train
            .iter()
            .chain(&c.validation)
            .map(semantic_skeleton)
            .collect::<Result<Vec<_>>>()?;
        let sc = heldout_skeletons(&excluded, 64);
        let mut used = c
            .train
            .iter()
            .chain(&c.validation)
            .flat_map(|e| e.request.evidence.items.iter().map(|r| r.event_id))
            .collect::<BTreeSet<_>>();
        let pairs = renamed_ids(64, "fixture-confirmation", &mut used)?;
        let oldids = renamed_ids(64, "fixture-used", &mut used)?;
        let mut ids = pairs.iter().flatten().copied().collect::<Vec<_>>();
        ids.sort_unstable();
        let mut past = oldids.iter().flatten().copied().collect::<Vec<_>>();
        past.sort_unstable();
        let private = dir.join("private.r3b");
        write(
            &private,
            &binary::record!({"schema":1,"skeletons":sc,"event_pairs":pairs}),
        )?;
        let public = dir.join("reservation.r3b");
        write(
            &public,
            &binary::record!({"event_id_count":128,"event_ids":ids,
            "skeleton_digest":digest(&sc)?,"reviewer_private_file_hash":file_hash(&private)?}),
        )?;
        let used = dir.join("used.r3b");
        write(
            &used,
            &binary::record!({"event_id_count":128,"event_ids":past}),
        )?;
        Ok((public, private, used))
    }
    #[test]
    fn value_citation_data_scorer_and_native_boundaries() -> Result<()> {
        let tmp = tempfile::tempdir()?;
        let study = super::super::tests::orbit_fixture(tmp.path())?;
        let parent = study.join("BOTH");
        let p = plan_read(&parent)?;
        let tok = ByteBpe::load(&parent.join("tokenizer.r3b"))?;
        let (c, tm, dm) = source_pool(&parent, &p, &tok)?;
        let (public, _, used) = fixture_reservation(&parent, tmp.path())?;
        let (c, m, audit) = make_pool(
            &c,
            &tm,
            &dm,
            &tok,
            &read_ids(&public, 128)?,
            &read_ids(&used, 128)?,
        )?;
        let cost = tape_cost(&suffix(), &c.train, &tok)?;
        assert_eq!(cost["updates"], 3072);
        assert_eq!(cost["tasks_V_VC0_VC1_samples_input_target"][0][0], 12288);
        assert_eq!(cost["tasks_V_VC0_VC1_samples_input_target"][1][0], 6144);
        assert_eq!(cost["tasks_V_VC0_VC1_samples_input_target"][2][0], 6144);
        let mut bad_rows = suffix();
        bad_rows[0][7] = usize::MAX;
        assert!(tape_cost(&bad_rows[..1], &c.train, &tok).is_err());
        bad_rows[0] = [0, 1, 0, 1, 1536, 1537, 1538, 1539];
        assert!(tape_cost(&bad_rows[..1], &c.train, &tok).is_err());
        let samples = samples(&c.train, &tok, 256)?;
        let b = batch(&samples, &suffix()[0], &Device::Cpu)?;
        for (j, &i) in suffix()[0].iter().enumerate() {
            let s = &samples[i];
            let mask = b.mask.to_vec2::<f32>()?;
            let labels = b.target.to_vec2::<u32>()?;
            assert_eq!(&labels[j][..s.tokens.len() - 1], &s.tokens[1..]);
            assert_eq!(
                mask[j].iter().sum::<f32>() as usize,
                s.tokens.len() - s.response_start
            );
            assert!(mask[j][..s.response_start - 1].iter().all(|&v| v == 0.));
        }
        let es = &c.validation[512..516];
        let ms = &m.1[512..516];
        let raw = es
            .iter()
            .map(|e| gold(e, &tok))
            .collect::<Result<Vec<_>>>()?;
        let s = score_citation(es, ms, &raw, &tok)?;
        assert_eq!(
            (
                s.joint.full,
                s.joint.query_both,
                s.joint.swap_both,
                s.joint.all4,
                s.citation_support_correct
            ),
            (4, 2, 2, 1, 4)
        );
        for kind in 0..6 {
            let mut altered = raw.clone();
            let mut e = es[0].clone();
            let id = e
                .request
                .evidence
                .items
                .iter()
                .find(|r| r.event_id != citations(&e.answer).unwrap()[0])
                .unwrap()
                .event_id;
            let v = value(&e.answer).unwrap() as char;
            e.answer = match kind {
                0 => format!("{v}입니다. [event:{id}]"),
                1 => format!("{v}입니다. [event:10000000]"),
                2 => format!("{v}입니다. [event:{id}] "),
                3 => v.to_string(),
                4 => format!("{v}입니다. [event:{id}][event:{id}]"),
                _ => String::new(),
            };
            altered[0] = gold(&e, &tok)?;
            altered[0]["expected"] = binary::record!(es[0].answer);
            altered[0]["exact_match"] = binary::record!(false);
            let s = score_citation(es, ms, &altered, &tok)?;
            assert_eq!(s.joint.full, 3);
            assert_eq!(s.joint.all4, 0);
            if kind == 0 {
                assert_eq!(s.citation_provided, 4);
                assert_eq!(s.citation_support_correct, 3);
            }
        }
        assert!(score_citation(es, ms, &raw[..3], &tok).is_err());
        let mut wrong = es.to_vec();
        wrong[0].answer = wrong[1].answer.clone();
        assert!(score_citation(&wrong, ms, &raw, &tok).is_err());
        // Actual typed writer -> confirmed publication -> reader at all registered
        // output/trace array bounds, without hundreds of optimizer calls.
        for count in [512usize, 1024, 1536, 3072] {
            let object = binary::record!({"step":7424,"raw_fixture":vec![raw[0].clone();count],"lr_trace":vec![3e-4f64;512],"sample_indices":suffix(),"value":0.33333333333333337f64});
            let path = tmp.path().join(format!("boundary-{count}.r3b"));
            publish_confirmed(&path, &object)?;
            assert_eq!(read_confirmed::<binary::Value>(&path)?, object);
        }
        println!(
            "CITATION_DATA_BOUNDARIES optimizer0 generation0 teacher0 input={} target={} distinct_original_ids={} full_pool4608 all_targets_native_roundtrip scorer_gold4; synthetic score is not model quality",
            cost["input"], cost["target"], audit["original_distinct_ids"]
        );
        Ok(())
    }
    #[test]
    fn value_citation_native_process_resume() -> Result<()> {
        const CHILD: &str = "R3_CITATION_CHILD";
        if let Ok(root) = std::env::var(CHILD) {
            let root = Path::new(&root);
            return match std::env::var("R3_CITATION_ACTION").as_deref() {
                Ok("parent-v") => parent_observe(root, false),
                Ok("parent-vc") => parent_observe(root, true),
                Ok("review-v") => parity(root, false, true),
                Ok("review-vc") => parity(root, true, true),
                _ => run(
                    root,
                    std::env::var("R3_CITATION_CONTINUOUS").as_deref() == Ok("1"),
                ),
            };
        }
        if !cfg!(feature = "test-support") {
            return Err(bad("TINY process fixture requires test-support"));
        }
        let tmp = tempfile::tempdir()?;
        let base = std::env::var_os("R3_CITATION_TEST_ROOT")
            .map(PathBuf::from)
            .unwrap_or_else(|| tmp.path().join("evidence"));
        std::fs::create_dir(&base)?;
        let base = base.canonicalize()?;
        let child = |root: &Path,
                     action: &str,
                     continuous: bool,
                     fault: bool,
                     label: &str|
         -> Result<()> {
            let mut cmd = std::process::Command::new(std::env::current_exe()?);
            cmd.args(["--exact","training::fresh::identifiable::binding::citation::tests::value_citation_native_process_resume","--nocapture"])
                .env(CHILD,root).env("R3_CITATION_ACTION",action).env("R3_CITATION_CONTINUOUS",if continuous {"1"}else{"0"})
                .env("VECLIB_MAXIMUM_THREADS","1").env("OMP_NUM_THREADS","1");
            if fault {
                cmd.env(
                    "R3_FRESH_CALL_STOP",
                    "eval-0004-citation4/generation/1/fresh_panel_row_durable",
                );
            }
            let result = cmd.output()?;
            std::fs::write(base.join(format!("{label}.stdout")), &result.stdout)?;
            std::fs::write(base.join(format!("{label}.stderr")), &result.stderr)?;
            assert!(
                result.status.success(),
                "{label}: {} {}",
                String::from_utf8_lossy(&result.stdout),
                String::from_utf8_lossy(&result.stderr)
            );
            assert!(String::from_utf8_lossy(&result.stdout).contains("1 passed"));
            Ok(())
        };
        let parents = super::super::tests::orbit_fixture(&base)?;
        child(&parents.join("FIXED"), "run", true, false, "parent-fixed")?;
        child(&parents.join("BOTH"), "run", true, false, "parent-both")?;
        let parent = parents.join("BOTH");
        let (public, private, used) = fixture_reservation(&parent, &base)?;
        let mut ends = vec![];
        let mut outputs = vec![];
        let mut actual = [4usize, 32, 32];
        for mode in ["continuous", "split", "evaluation-only"] {
            let study = base.join(mode);
            prepare_inner(&parent, &study, &public, &used, true)?;
            seal(&study, &private)?;
            super::super::tests::fixture_review(&study)?;
            child(
                &study,
                "parent-v",
                false,
                false,
                &format!("{mode}-parent-v"),
            )?;
            child(
                &study,
                "parent-vc",
                false,
                false,
                &format!("{mode}-parent-vc"),
            )?;
            actual[1] += 8;
            let root = study.join(ARM);
            let p = plan_read(&root)?;
            let mut wrong = p.clone();
            wrong.config.lr = 1e-4;
            assert!(verify_plan(&root, &wrong).is_err());
            wrong = p.clone();
            wrong.identifiable.as_mut().unwrap().rows[2].swap(0, 1);
            assert!(verify_plan(&root, &wrong).is_err());
            child(
                &root,
                "run",
                mode != "split",
                mode == "evaluation-only",
                &format!("{mode}-0"),
            )?;
            let prefix = if mode == "evaluation-only" {
                let h = history(&root, &p)?;
                assert_eq!(
                    h.last().unwrap().phase.as_deref(),
                    Some("EvaluationPending")
                );
                assert_eq!(h.last().unwrap().step, 4);
                Some(binary::read_value_records(
                    &root.join("eval-0004-citation4.r3rows"),
                )?)
            } else {
                None
            };
            if mode != "continuous" {
                child(&root, "run", true, false, &format!("{mode}-1"))?;
            }
            if let Some(prefix) = prefix {
                let rows = binary::read_value_records(&root.join("eval-0004-citation4.r3rows"))?;
                assert_eq!(rows[..prefix.len()], prefix);
                let ctl: binary::Value = read(&root.join("segment-0001/train-control.r3b"))?;
                assert_eq!(ctl["optimizer_calls"], 0);
            }
            let (end, d) = close(&root, &p)?;
            assert_eq!(end.step, 4);
            assert_eq!(d["eligible"], false);
            assert_eq!(end.stop, "FINAL_QUALITY_FAIL_AT_4");
            let l = checkpoint::load(&root.join(&end.checkpoint), Device::Cpu, true)?;
            let st = l.manifest.training.as_ref().unwrap();
            ends.push((
                l.model.weights_content_id()?,
                optimizer_hash(&l.optimizer)?,
                st.step,
                st.sampler_state,
                st.consumed_tokens,
                st.target_tokens,
            ));
            let mut output = vec![];
            for (name, es, ms) in panels(&root, &p, 4)? {
                let raw =
                    binary::read_value_records(&root.join(format!("eval-0004-{name}.r3rows")))?;
                score(&raw[1..], &es, &ms)?;
                for row in &raw[1..] {
                    output.push(binary::record!({"actual":row["actual"],"tokens":row["raw_tokens"],"finish":row["finish_reason"],"error":row["error"]}));
                }
            }
            outputs.push(output);
            for s in history(&root, &p)? {
                let ctl: binary::Value = read(
                    &root
                        .join(s.checkpoint)
                        .parent()
                        .unwrap()
                        .join("train-control.r3b"),
                )?;
                actual[0] += ctl["optimizer_calls"].as_u64().unwrap() as usize;
                actual[1] += s.generations;
                actual[2] += s.teachers;
            }
            assert!(!gate(&root, &p)?);
            assert!(parity(&study, false, false).is_err());
            assert!(confirm(&study).is_err());
            assert!(run(&root, false).is_err());
        }
        assert!(ends.windows(2).all(|e| e[0] == e[1]));
        assert!(outputs.windows(2).all(|e| e[0] == e[1]));
        child(
            &base.join("continuous"),
            "review-v",
            false,
            false,
            "normal-fail-review-v",
        )?;
        child(
            &base.join("continuous"),
            "review-vc",
            false,
            false,
            "normal-fail-review-vc",
        )?;
        actual[1] += 8;
        println!(
            "CITATION_TINY_PROCESS actual optimizer{} generation{} teacher{}; EOS tensor fixture, same native V/VC CE2 vs1+1 vs2+0 weight/Adam/raw equality; final quality fail reviewer admitted, candidate/confirmation/resume rejected; evidence={}",
            actual[0],
            actual[1],
            actual[2],
            base.display()
        );
        Ok(())
    }
}
fn verified_confirmation(
    study: &Path,
    p: &Plan,
    end: &Segment,
    d: &binary::Value,
) -> Result<binary::Value> {
    if d["eligible"] != true {
        return Err(bad("confirmation without eligible endpoint"));
    }
    let comparison = comparison(study, p, end, d)?;
    let b: binary::Value = read_confirmed(&study.join("review-b.r3b"))?;
    if b["verdict"] != "PASS"
        || b["preparation"] != comparison["preparation"]
        || b["endpoints"] != comparison["endpoints"]
        || b["report_hash"]
            != file_hash(Path::new(
                b["report_path"]
                    .as_str()
                    .ok_or_else(|| bad("citation review path"))?,
            ))?
    {
        return Err(bad("citation historical B binding"));
    }
    let candidate: binary::Value = read(&study.join("confirmation-candidate.r3b"))?;
    if candidate
        != binary::record!({"checkpoint":end.checkpoint_hash,"comparison":comparison,"seal":file_hash(&study.join("confirmation-seal.r3b"))?,
        "review":file_hash(&study.join("review-b.r3b"))?,"framing":p.framing()})
    {
        return Err(bad("citation candidate changed"));
    }
    let seal = seal_receipt(study)?;
    let corpus = verified_corpus(
        &study.join("sealed/confirmation.r3cor"),
        seal["corpus"]
            .as_str()
            .ok_or_else(|| bad("citation seal corpus"))?,
    )?;
    let es = corpus.validation;
    let ms: Vec<Meta> = read(&study.join("sealed/metadata.r3b"))?;
    if es.len() != 512 || ms.len() != 512 {
        return Err(bad("citation confirmation full denominator"));
    }
    let rows = observed(study, "confirmation", p, &end.checkpoint_hash, &es, false)?;
    let header: binary::Value = read(&study.join("confirmation-started.r3b"))?;
    if header["identity"]["candidate"] != file_hash(&study.join("confirmation-candidate.r3b"))? {
        return Err(bad("citation opening identity"));
    }
    let tok = ByteBpe::load(&study.join(ARM).join("tokenizer.r3b"))?;
    orbit_validate(&es[..256], &ms[..256], &tok, 256)?;
    check_variant(&es[..256], &es[256..], &ms[256..], &tok, true)?;
    let v = orbit_score(&es[..256], &ms[..256], &rows[..256], &tok)?;
    let vc = score_citation(&es[256..], &ms[256..], &rows[256..], &tok)?;
    let passed = pass(&v, 256, 244, 116, 58)
        && pass(&vc.joint, 256, 244, 116, 58)
        && vc.value_correct >= 254
        && vc.citation_support_correct >= 254
        && vc.outside_id == 0;
    let expected = binary::record!({"checkpoint":end.checkpoint_hash,"V":v,"VC":vc,"value_citation_baseline_verified":passed,
        "scope":"two current records, one-digit key/value, fixed grammar, eight-digit event ID","goal1_ready":false,"goal1_accepted":false,"s4":false,"s5":false,"s6":false});
    if read_confirmed::<binary::Value>(&study.join("confirmation-result.r3b"))? != expected {
        return Err(bad("citation confirmation raw/result disagreement"));
    }
    Ok(expected)
}
