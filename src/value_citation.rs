//! One bounded value/citation continuation. All execution and storage stay in
//! the existing binding/fresh harness; these transformations are training-only.
use super::*;

const DATA: &str = "value-citation-bridge-v1";
const CONTRACT: &str = "R3-VALUE-CITATION-BRIDGE-1.0";
pub(super) const ARM: &str = "VALUE-CITATION";
const ORIGIN: usize = 4352;
const UPDATES: usize = 3072;
const POOL: usize = 1536;
const MEAN_DATA: &str = "answer-mean-citation-v1";
const MEAN_CONTRACT: &str = "R3-ANSWER-MEAN-CITATION-1.0";
const MEAN_ARMS: [&str;2] = ["TOKEN-CONTROL", "ANSWER-MEAN"];
const CONT_DATA: &str = "citation-continuation-v1";
const CONT_CONTRACT: &str = "R3-CITATION-CONTINUATION-1.0";
const FID_DATA: &str = "citation-fidelity-v1";
const FID_CONTRACT: &str = "R3-CITATION-FIDELITY-CONSOLIDATION-1.0";
const PREC_DATA: &str = "citation-precision-v1";
const PREC_CONTRACT: &str = "R3-CITATION-PRECISION-1.0";
const QA_DATA: &str = "retained-qa-transfer-v1";
const QA_CONTRACT: &str = "R3-RETAINED-QA-TRANSFER-1.0";
const QA_POOL: usize = 8192;
const QA_SCORER: &str = "strict-qa-independent-citation-ids-v2";
const BRIDGE_CONTRACT: &str = "R3-QA-INTEGRITY-AND-BRIDGE-1.0";
const BRIDGE_DATA: &str = "retained-instruction-bridge-v1";
const BRIDGE_COMPLETION_DATA: &str = "instruction-bridge-completion-v1";
const BRIDGE_COMPLETION_CONTRACT: &str = "R3-INSTRUCTION-BRIDGE-COMPLETION-1.0";
fn bridge_completion(p:&Plan)->bool {p.identifiable.as_ref().is_some_and(|o|o.dataset==BRIDGE_COMPLETION_DATA)}
pub(in super::super::super) fn instruction_bridge(p:&Plan)->bool {bridge_completion(p)||p.identifiable.as_ref().is_some_and(|o|o.dataset==BRIDGE_DATA)}
fn qa_task(p:&Plan,index:usize)->usize {if retained_qa(p)&&index>=3*POOL {3+(index-3*POOL)/QA_POOL}else{index/POOL}}
pub(in super::super::super) fn retained_qa(p:&Plan)->bool {p.identifiable.as_ref().is_some_and(|o|o.dataset==QA_DATA)}
pub(in super::super::super) fn precision(p:&Plan)->bool {p.identifiable.as_ref().is_some_and(|o|o.dataset==PREC_DATA)}
pub(in super::super::super) fn fidelity(p:&Plan)->bool {p.identifiable.as_ref().is_some_and(|o|o.dataset==FID_DATA)}
fn continuation(p:&Plan)->bool {precision(p) || fidelity(p) || p.identifiable.as_ref().is_some_and(|o|o.dataset==CONT_DATA)}
fn citation_offset(p:&Plan,step:usize)->usize {step-p.origin_step()+if continuation(p)&&!fidelity(p)&&!precision(p)&&!p.tiny {32}else{0}}
fn full_evaluation(p:&Plan,step:usize)->bool {
    if instruction_bridge(p) {return !p.tiny&&[768,1536].contains(&(step-p.origin_step()));}
    if retained_qa(p) { return !p.tiny && [2048,4096].contains(&(step-p.origin_step())); }
    !p.tiny && if precision(p) {[768,1536].contains(&citation_offset(p,step))}
        else {[1536,3072].contains(&citation_offset(p,step))}
}
pub(in super::super::super) fn is_mean(p: &Plan) -> bool {
    instruction_bridge(p) || retained_qa(p) || continuation(p) || p.identifiable.as_ref().is_some_and(|o|o.dataset == MEAN_DATA)
}
pub(in super::super::super) fn answer_mean(p: &Plan) -> bool {
    is_mean(p) && own(p).arm == MEAN_ARMS[1]
}
pub(super) fn arms(p: &Plan) -> &'static [&'static str] {
    if instruction_bridge(p) || retained_qa(p) || continuation(p) { &[MEAN_ARMS[1]] } else if is_mean(p) { &MEAN_ARMS } else { &[ARM] }
}
fn selected_root(study: &Path) -> PathBuf {
    study.join(if study.join(MEAN_ARMS[1]).is_dir() { MEAN_ARMS[1] } else { ARM })
}
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
    is_mean(p) || p.identifiable.as_ref().is_some_and(|v| v.dataset == DATA)
}
pub(super) fn evaluation(p: &Plan) -> EvaluationPolicy {
    if instruction_bridge(p) {
        let mut e=super::evaluation(p.tiny);e.screen_steps.clear();
        e.train_steps=if p.tiny{vec![p.origin_step()+2]}else{[256,768,1536].map(|n|p.origin_step()+n).to_vec()};
        e.generation_limit=9216;e.teacher_limit=8192;e.active_seconds=7200;return e;
    }
    if retained_qa(p) {
        let mut e=super::evaluation(p.tiny);
        e.screen_steps.clear();
        e.train_steps=if p.tiny {vec![p.origin_step()+2]}else{[128,512,1024,2048,3072,4096].map(|n|p.origin_step()+n).to_vec()};
        e.generation_limit=24576;e.teacher_limit=18432;e.active_seconds=14400;
        return e;
    }
    let mut e = super::evaluation(p.tiny);
    e.screen_steps.clear();
    e.train_steps = if p.tiny {
        vec![p.origin_step() + 2]
    } else if precision(p) {
        [256,768,1536].map(|n|p.origin_step()+n).to_vec()
    } else if fidelity(p) {
        [256,768,1536,2304,3072].map(|n|p.origin_step()+n).to_vec()
    } else if continuation(p) {
        [64,128,256,384,768,1536,3072].map(|n|ORIGIN+n).to_vec()
    } else {
        [32, 128, 384, 768, 1536, 3072].map(|n| ORIGIN + n).to_vec()
    };
    e.generation_limit = if fidelity(p)||precision(p) {14336}else{16384};
    // TINY's explicit three-by-four panel exercises exact-cap finalization.
    e.teacher_limit = if fidelity(p)||precision(p) {if p.tiny {12}else{13120}}else{16384};
    e.active_seconds = if precision(p) {7200}else{10800};
    e.train_steps.retain(|&s|s<=p.config.max_steps);
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
    if retained_qa(p) {
        return [1,128,512,1024,1536,2048,2560,3072,3584,4096].into_iter()
            .map(|n|p.origin_step()+n).find(|&n|n>step).ok_or_else(||bad("retained QA budget closed"));
    }
    if instruction_bridge(p) {return [1,256,768,1280,1536].into_iter().map(|n|p.origin_step()+n).find(|&n|n>step).ok_or_else(||bad("bridge budget closed"));}
    let points:&[usize]=if precision(p) {&[10497,10752,11264,11776,12032]}
        else if fidelity(p) {&[7425,7680,8192,8704,8960,9472,9728,10240,10496]}
        else if continuation(p) {&[4416,4480,4608,4736,5120,5632,5888,6400,6912,7424]}
        else{&[4384,4480,4736,5120,5632,5888,6400,6912,7424]};
    points.iter().copied()
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
    if instruction_bridge(p) {return bridge_verify_plan(root,p);}
    if retained_qa(p) {return qa_verify_plan(root,p);}
    if continuation(p) {return verify_continuation(root,p);}
    if is_mean(p) { return verify_mean_plan(root,p); }
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
// Diagnostic extraction only. The product's whole-response parser stays strict.
// Inspect every start, including one nested after an unclosed/malformed prefix.
fn individually_valid_ids(text: &str) -> BTreeSet<i64> {
    text.match_indices("[event:").filter_map(|(start,_)| {
        let rest=&text[start+7..];
        rest.split_once(']').and_then(|(id,_)|id.parse::<i64>().ok()).filter(|id|*id>0)
    }).collect()
}
fn has_outside_id(e: &Episode, text: Option<&str>) -> bool {
    text.is_some_and(|text|individually_valid_ids(text).iter().any(|id|
        !e.request.evidence.items.iter().any(|event|event.event_id==*id)))
}
fn valid_outside_ids(es: &[Episode], rows: &[binary::Value]) -> usize {
    es.iter().zip(rows).filter(|(e,r)|has_outside_id(e,r["actual"].as_str())).count()
}
fn score_citation(
    es: &[Episode],
    ms: &[Meta],
    rows: &[binary::Value],
    tok: &ByteBpe,
) -> Result<CitationScore> {
    score_citation_profile(es,ms,rows,tok,false)
}
fn score_citation_profile(es:&[Episode],ms:&[Meta],rows:&[binary::Value],tok:&ByteBpe,bridge:bool)->Result<CitationScore> {
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
        if (if bridge {bridge_resolve(&e.request)}else{resolve_request(&e.request)})? != e.answer
            || (!bridge && !e.request.input.ends_with(CITATION_QUERY))
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
    if instruction_bridge(p) {return bridge_panels(root,p,step);}
    if retained_qa(p) {return qa_base_panels(root,p,step);}
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
    let offset = citation_offset(p,step);
    let full = full_evaluation(p,step);
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
    if fidelity(p) || precision(p) || offset >= 384 || p.tiny {
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
    if retained_qa(p) && panel.0.starts_with("qa-") {return qa_score(root,p,step,panel);}
    let (name, es, ms) = panel;
    let tok = ByteBpe::load(&root.join("tokenizer.r3b"))?;
    let summary = audit_panel(root, p, step, name, es, ms, &tok)?;
    let raw = binary::read_value_records(&root.join(format!("eval-{step:04}-{name}.r3rows")))?;
    let is_v = es[0].request.input.ends_with(VALUE_QUERY);
    let mut out = if is_v {
        binary::record!({"joint":orbit_score(es,ms,&raw[1..],&tok)?})
    } else {
        binary::to_value(score_citation_profile(es, ms, &raw[1..], &tok,instruction_bridge(p))?)?
    };
    if is_mean(p) {
        let mut first=0;let mut extra=0;let mut length=0;let mut parsed=0;let mut runtime=0;
        for (e,r) in es.iter().zip(&raw[1..]) {
            let actual=r["actual"].as_str().unwrap_or("");
            let good=value(actual).is_some() && value(actual)==value(&e.answer);
            first+=usize::from(good);extra+=usize::from(good && is_v && actual.len()>1);
            length+=usize::from(r["finish_reason"]=="length");runtime+=usize::from(!r["error"].is_null());
            if let Ok(ids)=citations(actual) { if ids.len()==1 {
                parsed+=1;
            }}
        }
        out["first_value_correct"]=binary::record!(first);out["wrong_first_value"]=binary::record!(es.len()-first);
        out["correct_value_with_extra_output"]=binary::record!(extra);out["length"]=binary::record!(length);
        out["runtime_errors"]=binary::record!(runtime);
        if !is_v {
            out["parsed_single_id"]=binary::record!(parsed);out["citation_parse_failures"]=binary::record!(es.len()-parsed);
            out["valid_outside_id"]=binary::record!(valid_outside_ids(es,&raw[1..]));
            if instruction_bridge(p){out["parse_failure_rows"]=binary::record!(raw[1..].iter().filter(|r|r["actual"].as_str().is_none_or(|s|citations(s).is_err())).count());}
        }
        out["teacher_objective"]=binary::record!("gold-prefix token CE diagnostic; not answer-mean training objective");
    }
    let teachers =
        binary::read_value_records(&root.join(format!("eval-{step:04}-{name}-teachers.r3rows")))?;
    let mut value_nll = 0.;
    let mut cite_nll = 0.;
    let mut eos_nll = 0.;
    let mut citation_tokens = 0;
    let mut id_nll = 0.;
    let mut id_tokens = 0;
    let mut id_positions=[0.;8];let mut id_position_counts=[0usize;8];
    let mut grammar_nll=0.;let mut grammar_tokens=0;let mut after_value_margin=0.;
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
        if continuation(p) {
            after_value_margin+=r["teacher"]["after_value"]["eos_minus_citation_logit"].as_f64().filter(|n|n.is_finite()).ok_or_else(||bad("continuation after-value teacher signal missing"))?;
        }
        if let Some(start) = e.answer.find("[event:").map(|p| p + 7) {
            let end = e.answer.len() - 1;
            let mut offset = 0;
            for (&token, &loss) in gold[..gold.len() - 1].iter().zip(&n) {
                let size = tok.decode_bytes(&[token])?.len();
                if offset < end && offset + size > start {
                    id_nll += loss;
                    id_tokens += 1;
                    for position in 0..8 {if offset<=start+position && offset+size>start+position {id_positions[position]+=loss;id_position_counts[position]+=1;}}
                }
                if offset>0 && offset+size<=start {grammar_nll+=loss;grammar_tokens+=1;}
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
    if continuation(p) {
        out["after_value_eos_minus_citation_logit"]=binary::record!(after_value_margin/es.len() as f64);
        out["grammar_prefix_nll"]=binary::record!((grammar_tokens>0).then(||grammar_nll/grammar_tokens as f64));
        out["grammar_prefix_tokens"]=binary::record!(grammar_tokens);
        out["id_position_token_nll"]=binary::record!(id_positions.iter().zip(id_position_counts).map(|(&n,c)|(c>0).then(||n/c as f64)).collect::<Vec<_>>());
        out["id_position_counts"]=binary::record!(id_position_counts);
    }
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
            && if v["valid_outside_id"].is_null() {v["outside_id"]==0}else{v["valid_outside_id"]==0})
    };
    Ok(pass(&joint, 512, 488, 232, 116) && citation("citation512")? && citation("renamed512")?)
}
pub(super) fn panels(root: &Path, p: &Plan, step: usize) -> Result<Vec<Panel>> {
    let mut out = base_panels(root, p, step)?;
    if full_evaluation(p,step)||(bridge_completion(p)&&p.tiny&&step==p.config.max_steps) {
        let present = out.iter().take(if retained_qa(p)||instruction_bridge(p) {out.len()}else{3})
            .all(|(name, _, _)| root.join(format!("eval-{step:04}-{name}.r3b")).exists());
        if present {
            let scores = out.iter().take(if retained_qa(p)||instruction_bridge(p) {out.len()}else{3})
                .map(|panel| Ok((panel.0.clone(), read_score(root, p, step, panel)?)))
                .collect::<Result<BTreeMap<_, _>>>()?;
            let fit_due=if instruction_bridge(p) {
                !qa_guard(root,p,step,&out)?["stop"].is_string()
                    && ((bridge_completion(p)&&step==p.config.max_steps)||(!p.tiny&&bridge_dev_pass(&scores)?))
            }else{dev_pass(&scores)?&&(!retained_qa(p)||qa_dev_pass(&scores)?)};
            if fit_due {
                if instruction_bridge(p){let c=verified_corpus(&root.join("corpus.r3cor"),&p.corpus)?;let(tm,_,_)=verified_metadata(root,p)?;
                    let count=if p.tiny{4}else{1536};
                    out.push((format!("bridge-fit{count}"),c.train[3*POOL..3*POOL+count].to_vec(),tm[3*POOL..3*POOL+count].to_vec()));
                }else{out.extend(fit_panels(root, p)?);}
            }
        }
    }
    Ok(out)
}
fn evaluation_result(root: &Path, p: &Plan, step: usize) -> Result<binary::Value> {
    if instruction_bridge(p) {return bridge_decision(root,p,step);}
    if retained_qa(p) {return qa_decision(root,p,step);}
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
    let guard = if continuation(p) {Some(continuation_guard(root,p,step,&screen,file_hash(&root.join(format!("eval-{step:04}-{}.r3rows",value_panel.0)))?)?)}else{None};
    let regression = if let Some(g)=&guard {g["stop"].is_string()}else{baseline.full.saturating_sub(screen.full) >= 16
        || baseline.all4.saturating_sub(screen.all4) >= 4
        || screen.errors >= 4};
    let full = full_evaluation(p,step);
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
    let (action,extend)=evaluation_action(p,step,eligible,regression,guard.as_ref());
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
    let mut result = binary::record!({"policy":digest(p)?,"step":step,"model":models.into_iter().next(),"panels":scores,
        "V64":screen,"parent_V64":baseline,"ID_pairs":id_pairs,"development":development,"fit":fits,"eligible":eligible,
        "regression":regression,"action":action,"extend":extend,"stop":(!extend).then_some(action)});
    if let Some(g)=guard {result["guard"]=g;}
    Ok(result)
}
fn evaluation_action(p:&Plan,step:usize,eligible:bool,regression:bool,guard:Option<&binary::Value>)->(String,bool) {
    let action = if regression {
        guard.as_ref().and_then(|g|g["stop"].as_str()).unwrap_or("QUALITY_REGRESSION").to_string()
    } else if eligible {
        format!("CANDIDATE_FIXED_AT_{step}")
    } else if step == p.config.max_steps {
        format!("FINAL_QUALITY_FAIL_AT_{step}")
    } else {
        "CONTINUE_WITHIN_REGISTERED_CAP".into()
    };
    let extend = !regression && !eligible && step < p.config.max_steps;
    (action,extend)
}
fn retention_decision(full:usize,all4:usize,errors:usize,before:usize)->(usize,Option<&'static str>) {
    let streak=if full<60 || all4<12 {before+1}else{0};
    let stop=if full<=48 || errors>=4 {Some("SEVERE_RETENTION_REGRESSION")}
        else if streak>=2 {Some("PERSISTENT_RETENTION_REGRESSION")}else{None};
    (streak,stop)
}
fn continuation_guard(root:&Path,p:&Plan,step:usize,s:&OrbitScore,raw:String)->Result<binary::Value> {
    let mut before=0;let mut previous=None;
    for &prior in p.evaluation.train_steps.iter().filter(|&&n|n<step) {
        let path=root.join(format!("citation-decision-{prior:04}.r3b"));
        let d:binary::Value=read_confirmed(&path)?;
        let panel=base_panels(root,p,prior)?.remove(0);
        if d["policy"]!=digest(p)? || d["step"]!=prior || d["guard"]["before"]!=before
            || d["guard"]["raw"]!=file_hash(&root.join(format!("eval-{prior:04}-{}.r3rows",panel.0)))?
            || d["guard"]["previous"]!=binary::record!(previous) || !d["guard"]["stop"].is_null() {
            return Err(bad("continuation guard chain ambiguous or already stopped"));
        }
        let score:OrbitScore=binary::from_value(d["V64"].clone())?;
        let rows=binary::read_value_records(&root.join(format!("eval-{prior:04}-{}.r3rows",panel.0)))?;
        let tok=ByteBpe::load(&root.join("tokenizer.r3b"))?;let n=if p.tiny {4}else{64};
        if rows.len()<n+1 || score!=orbit_score(&panel.1[..n],&panel.2[..n],&rows[1..n+1],&tok)? {return Err(bad("continuation prior raw/guard score disagreement"));}
        let (after,stop)=retention_decision(score.full,score.all4,score.errors,before);
        if d["guard"]["after"]!=after || d["guard"]["stop"]!=binary::record!(stop) {return Err(bad("continuation guard decision mismatch"));}
        before=after;previous=Some(file_hash(&path)?);
    }
    let (after,stop)=if p.tiny {(0,None)}else{retention_decision(s.full,s.all4,s.errors,before)};
    Ok(binary::record!({"policy":digest(p)?,"step":step,"raw":raw,"previous":previous,"before":before,"after":after,"stop":stop,
        "origin_counts_as_warning":false,"application":"immutable evaluation/decision receipt; prior distinct scheduled evaluations only"}))
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
        ["old512", "new1024", "VC0train1536", "VC1train1536", "bridge-fit1536"].contains(&n.as_str())
            || (p.tiny&&bridge_completion(p)&&n=="bridge-fit4")
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
    let mut task = vec![[0usize; 3];if retained_qa(p) {5}else if instruction_bridge(p){4}else{3}];
    let mut files = BTreeMap::new();
    let mut mean_steps = Vec::new();
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
                    let group=qa_task(p,i);
                    task[group][0] += 1;
                    task[group][1] += a;
                    task[group][2] += b;
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
                if is_mean(p) {
                    let stats=r["tasks"].as_array().ok_or_else(||bad("mean trace examples"))?;
                    let mut token=0.;let mut answer=0.;let mut actual_targets=0;let mut contributions=vec![0.;task.len()];
                    for (s,&i) in stats.iter().zip(&draw) {
                        let ce=s["ce"].as_f64().ok_or_else(||bad("mean sample CE"))?;
                        let n=s["target"].as_u64().ok_or_else(||bad("mean sample targets"))? as usize;
                        if s["index"]!=i || n!=samples[i].tokens.len()-samples[i].response_start || n==0 || !ce.is_finite() {return Err(bad("mean sample trace binding"));}
                        token+=ce*n as f64;answer+=ce;actual_targets+=n;
                        contributions[qa_task(p,i)]+=ce*if answer_mean(p){1.}else{n as f64};
                    }
                    token/=actual_targets as f64;answer/=stats.len() as f64;
                    let objective=if answer_mean(p){answer}else{token};
                    if stats.len()!=draw.len() || actual_targets!=nt || r["examples"]!=stats.len()
                        || r["target_tokens"]!=nt || r["objective_denominator"]!=if answer_mean(p){stats.len()}else{nt}
                        || [("token_ce",token),("answer_mean_ce",answer),("effective_training_objective",objective),("objective",objective)]
                            .iter().any(|(k,v)|r[*k].as_f64().is_none_or(|a|!a.is_finite()||(a-v).abs()>2e-5*(1.+v.abs()))) {
                        return Err(bad("mean trace reduction/usage denominator"));
                    }
                    let denominator=if answer_mean(p){stats.len()}else{nt};
                    for c in &mut contributions {*c/=denominator as f64;}
                    let mut reduction=binary::record!({"step":cursor,"token_ce":token,"answer_mean_ce":answer,"effective_training_objective":objective,
                        "objective_denominator":denominator,"target_tokens":nt,"examples":stats.len(),"V_VC0_VC1_scalar_contribution":contributions,
                        "grad_norm":r["grad_norm"],"clip":r["clip"],"delta_norm":r["delta_norm"],"scope":"scalar contributions, not task gradient norms"});
                    if retained_qa(p) {if let binary::Value::Object(m)=&mut reduction {m.remove("V_VC0_VC1_scalar_contribution");} reduction["V_VC0_VC1_Q0_Q1_scalar_contribution"]=binary::record!(contributions);}
                    if instruction_bridge(p) {if let binary::Value::Object(m)=&mut reduction {m.remove("V_VC0_VC1_scalar_contribution");} reduction["V_VC0_VC1_bridge_scalar_contribution"]=binary::record!(contributions);}
                    mean_steps.push(reduction);
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
    let mut result=binary::record!({"files":files,"updates":cursor-p.origin_step(),"step":cursor,"input":input,"target":target,
        "counts":counts,"tasks_V_VC0_VC1_samples_input_target":task,"unique_V_VC0_VC1":counts.chunks(POOL).map(|c|c.iter().filter(|&&n|n>0).count()).collect::<Vec<_>>()});
    if is_mean(p) {result["reduction_steps"]=binary::record!(mean_steps);}
    if retained_qa(p) {
        let mut unique=vec![0usize;5];for (i,&n) in counts.iter().enumerate(){unique[qa_task(p,i)]+=usize::from(n>0);}
        if let binary::Value::Object(m)=&mut result {m.remove("tasks_V_VC0_VC1_samples_input_target");m.remove("unique_V_VC0_VC1");}
        result["tasks_V_VC0_VC1_Q0_Q1_samples_input_target"]=binary::record!(task);
        result["unique_V_VC0_VC1_Q0_Q1"]=binary::record!(unique);
    }
    if instruction_bridge(p) {
        if let binary::Value::Object(m)=&mut result{m.remove("tasks_V_VC0_VC1_samples_input_target");m.remove("unique_V_VC0_VC1");}
        result["tasks_V_VC0_VC1_bridge_samples_input_target"]=binary::record!(task);
        result["unique_V_VC0_VC1_bridge"]=binary::record!(counts.chunks(POOL).map(|c|c.iter().filter(|&&n|n>0).count()).collect::<Vec<_>>());
    }
    Ok(result)
}
fn seal_receipt(study: &Path) -> Result<binary::Value> {
    let selection:binary::Value=read(&study.join("selection.r3b"))?;
    if selection["contract"]==CONT_CONTRACT || selection["contract"]==FID_CONTRACT || selection["contract"]==PREC_CONTRACT {
        let previous=Path::new(selection["previous_study"].as_str().ok_or_else(||bad("continuation seal source"))?);
        let link:binary::Value=read_confirmed(&study.join("confirmation-link.r3b"))?;
        if link["preparation"]!=file_hash(&study.join("preparation.r3b"))? || link["source_study"]!=binary::record!(previous)
            || link["source_seal"]!=file_hash(&seal_owner(previous)?.join("confirmation-seal.r3b"))? || link["status"]!="VERIFIED_UNUSED" {
            return Err(bad("continuation seal lineage"));
        }
        return seal_receipt(previous);
    }
    if selection["contract"]==MEAN_CONTRACT {
        let link:binary::Value=read_confirmed(&study.join("confirmation-link.r3b"))?;
        let previous=Path::new(selection["previous_study"].as_str().ok_or_else(||bad("mean seal source"))?);
        if link["preparation"]!=file_hash(&study.join("preparation.r3b"))?
            || link["source_study"]!=binary::record!(previous)
            || link["source_seal"]!=selection["source_seal"]
            || link["source_seal"]!=file_hash(&previous.join("confirmation-seal.r3b"))?
            || link["status"]!="VERIFIED_UNUSED" {return Err(bad("mean inherited seal binding"));}
        return seal_receipt(previous);
    }
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
    if instruction_bridge(p) {return bridge_authorize(root,p);}
    if retained_qa(p) {return qa_authorize(root,p);}
    if continuation(p) {return continuation_authorize(root,p);}
    if is_mean(p) { return mean_authorize(root,p); }
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
        binary::record!({"contract":if bridge_completion(p){BRIDGE_COMPLETION_CONTRACT}else if instruction_bridge(p){BRIDGE_CONTRACT}else if retained_qa(p){QA_CONTRACT}else if precision(p){PREC_CONTRACT}else if fidelity(p){FID_CONTRACT}else if continuation(p){CONT_CONTRACT}else if is_mean(p){MEAN_CONTRACT}else{CONTRACT},"source":p.source,"preparation":file_hash(&study.join("preparation.r3b"))?,
        "endpoints":BTreeMap::from([(own(p).arm.as_str(),endpoint)]),"selected":if d["eligible"]==true {Some(own(p).arm.as_str())}else{None},"goal1_ready":false}),
    )
}
fn verify_review_b(study:&Path,comparison:&binary::Value)->Result<()> {
    let b:binary::Value=read_confirmed(&study.join("review-b.r3b"))?;
    if b["verdict"]!="PASS" || b["preparation"]!=file_hash(&study.join("preparation.r3b"))?
        || b["endpoints"]!=comparison["endpoints"]
        || b["report_hash"]!=file_hash(Path::new(b["report_path"].as_str().ok_or_else(||bad("citation B report"))?))? {
        return Err(bad("independent citation B required"));
    }
    Ok(())
}
pub(in super::super::super) fn confirm(study: &Path) -> Result<()> {
    let study = study.canonicalize()?;
    let lock = std::fs::File::open(&study)?;
    lock.try_lock().map_err(|_|bad("confirmation study already in use"))?;
    let root = selected_root(&study);
    let p = plan_read(&root)?;
    let (end, d) = close(&root, &p)?;
    if !confirmation_admitted(&study,&p,&end,&d)? {
        return Err(bad("NOT_RUN_PREREQUISITE: no value/citation candidate"));
    }
    let comparison = comparison(&study, &p, &end, &d)?;
    verify_review_b(&study,&comparison)?;
    verify_review_observations(&study,&root,&p,end.step,&end.checkpoint_hash)?;
    let seal = seal_receipt(&study)?;
    let candidate = confirmation_candidate(&study,&p,&end,&comparison,&seal)?;
    let candidate_path=study.join("confirmation-candidate.r3b");
    let owner=seal_owner(&study)?;
    let owner_lock=if owner!=study {Some(std::fs::File::open(&owner)?)}else{None};
    if let Some(lock)=&owner_lock {lock.try_lock().map_err(|_|bad("confirmation seal in use"))?;}
    let claim_path=owner.join("confirmation-claim.r3b");
    if !claim_path.exists() {
        let mut prior=study.clone();
        loop {
            if prior!=study {unused_seal(&prior)?;}
            else if !candidate_path.exists() {unused_seal(&prior)?;}
            if prior==owner {break;}
            let s:binary::Value=read(&prior.join("selection.r3b"))?;
            prior=PathBuf::from(s["previous_study"].as_str().ok_or_else(||bad("confirmation ancestor chain"))?);
        }
    }
    if candidate_path.exists() {
        if read_confirmed::<binary::Value>(&candidate_path)?!=candidate {return Err(bad("confirmation candidate mismatch"));}
    } else { publish_confirmed(&candidate_path,&candidate)?; }
    // A claim in the actual seal owner prevents a different study from opening
    // the same heldout body. Directory locks do not change original artifacts.
    let claim=binary::record!({"root":study,"candidate":file_hash(&candidate_path)?,"seal":candidate["seal"]});
    if claim_path.exists() {
        if read_confirmed::<binary::Value>(&claim_path)?!=claim {return Err(bad("confirmation seal already claimed"));}
    } else { publish_confirmed(&claim_path,&claim)?; }
    let c = verified_corpus(
        &seal_owner(&study)?.join("sealed/confirmation.r3cor"),
        seal["corpus"]
            .as_str()
            .ok_or_else(|| bad("citation seal corpus"))?,
    )?;
    let mut ms: Vec<Meta> = read(&seal_owner(&study)?.join("sealed/metadata.r3b"))?;
    let mut es = c.validation;
    if es.len() != 512 || ms.len() != 512 {
        return Err(bad("complete V256/VC256 confirmation required"));
    }
    let tok = ByteBpe::load(&root.join("tokenizer.r3b"))?;
    orbit_validate(&es[..256], &ms[..256], &tok, 256)?;
    check_variant(&es[..256], &es[256..], &ms[256..], &tok, true)?;
    if p.tiny { es=[es[..4].to_vec(),es[256..260].to_vec()].concat();ms=[ms[..4].to_vec(),ms[256..260].to_vec()].concat(); }
    let n=es.len()/2;
    if study.join("confirmation-result.r3b").exists() {
        println!("CITATION_CONFIRMATION {}",verified_confirmation(&study,&p,&end,&d)?);
        return Ok(());
    }
    let returned=if study.join("confirmation.r3rows").exists() {
        binary::read_value_records(&study.join("confirmation.r3rows"))?.len().checked_sub(1).ok_or_else(||bad("confirmation missing header"))?
    }else{0};
    let remaining=es.len().checked_sub(returned).ok_or_else(||bad("confirmation extra rows"))?;
    let mut control=observation_control(&p,remaining,0)?;
    #[cfg(feature="test-support")]
    if p.tiny && std::env::var("R3_CONFIRM_STOP").as_deref()==Ok("candidate") {control.fixture_boundary=Some("confirmation_next_generation".into());}
    let rows = confirmation_collect(
        &study,
        &root.join(&end.checkpoint),
        &es,
        &tok,
        &binary::record!({"policy":digest(&p)?,"checkpoint":end.checkpoint_hash,"candidate":file_hash(&study.join("confirmation-candidate.r3b"))?}),
        control,
    )?;
    let v = orbit_score(&es[..n], &ms[..n], &rows[..n], &tok)?;
    let vc = score_citation(&es[n..], &ms[n..], &rows[n..], &tok)?;
    let passed = pass(&v, 256, 244, 116, 58)
        && pass(&vc.joint, 256, 244, 116, 58)
        && vc.value_correct >= 254
        && vc.citation_support_correct >= 254
        && if is_mean(&p) {valid_outside_ids(&es[n..],&rows[n..])==0}else{vc.outside_id == 0};
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
fn confirmation_admitted(study:&Path,p:&Plan,end:&Segment,d:&binary::Value)->Result<bool> {
    if retained_qa(p)||instruction_bridge(p) {return Ok(false);} // The inherited citation confirmation is already consumed.
    #[cfg(all(test,feature="test-support"))]
    if p.tiny {
        let expected=binary::record!({"scope":"TINY confirmation protocol only; no quality acceptance","policy":digest(p)?,"checkpoint":end.checkpoint_hash,"count":8});
        return Ok(read_confirmed::<binary::Value>(&study.join("fixture-confirmation-admission.r3b"))?==expected);
    }
    let _=(study,end);
    Ok(!p.tiny && d["eligible"]==true)
}
fn confirmation_candidate(study:&Path,p:&Plan,end:&Segment,comparison:&binary::Value,seal:&binary::Value)->Result<binary::Value> {
    Ok(binary::record!({"protocol":1,"root":study,"checkpoint":end.checkpoint_hash,"step":end.step,"comparison":comparison,
        "model":comparison["endpoints"][own(p).arm.as_str()]["decision"]["model"],
        "seal":file_hash(&seal_owner(study)?.join("confirmation-seal.r3b"))?,"seal_owner":seal_owner(study)?,
        "corpus":seal["corpus"],"metadata":seal["metadata"],"count":seal["count"],
        "row_order_binding":"exact ordered typed metadata bytes authenticated by metadata digest",
        "review":file_hash(&study.join("review-b.r3b"))?,"framing":p.framing(),"tokenizer":p.tokenizer,
        "source":p.source,"binary":p.binary,"policy":digest(p)?,"evaluation":p.evaluation,"evaluated_count":if p.tiny {8}else{512},
        "decoding":"normal-greedy-strict-utf8-eos","max_new_tokens":32,"architecture":p.architecture,"dtype":"F32","backend":"CPU/Accelerate/thread1"}))
}
// Same existing citation harness with a separately bound reduction comparison.
// Historical files remain inputs; no collector or missing receipt is synthesized.
fn mean_plan(previous: &Plan, study: &Path, s: &binary::Value, arm: &str) -> Result<Plan> {
    if !MEAN_ARMS.contains(&arm) { return Err(bad("unknown reduction arm")); }
    let mut p=previous.clone();
    p.source=s["source"].as_str().ok_or_else(||bad("mean source"))?.into();
    p.binary=s["binary"].as_str().ok_or_else(||bad("mean binary"))?.into();
    let origin=p.origin_step();
    p.config.max_steps=origin+if p.tiny {2}else if arm==MEAN_ARMS[0] {32}else{UPDATES};
    p.config.max_tokens=p.config.budget_start_tokens+4_000_000;
    let o=p.identifiable.as_mut().ok_or_else(||bad("mean tape missing"))?;
    o.study=study.into();o.arm=arm.into();o.dataset=MEAN_DATA.into();
    o.rows.truncate(p.config.max_steps);
    p.train_order=digest(&o.rows)?;
    let f=p.fork.as_mut().ok_or_else(||bad("mean parent missing"))?;
    f.study=study.into();f.study_hash=file_hash(&study.join("selection.r3b"))?;
    f.arm=arm.into();f.target_limit=260_000;
    p.evaluation=evaluation(&p);
    Ok(p)
}
fn unused_seal(study:&Path)->Result<binary::Value> {
    let seal=seal_receipt(study)?;
    if seal["new_optimizer"]!=0 || seal["generation"]!=0 || seal["teacher"]!=0 {
        return Err(bad("citation seal creation usage unknown"));
    }
    // Examine every registered observation trace, not only a result path.
    for entry in std::fs::read_dir(study)? {
        let name=entry?.file_name(); let name=name.to_string_lossy();
        if name.starts_with("confirmation-candidate") || name.starts_with("confirmation-started")
            || name.starts_with("confirmation-finished") || name.starts_with("confirmation-result")
            || name.starts_with("confirmation-generation") || name.starts_with("confirmation.r3rows")
            || name=="confirmation" || name.starts_with("confirmation-call") || name.starts_with("confirmation-claim") {
            return Err(bad("citation confirmation already attempted or ambiguous"));
        }
    }
    Ok(seal)
}
fn seal_owner(study:&Path)->Result<PathBuf> {
    let s:binary::Value=read(&study.join("selection.r3b"))?;
    if s["contract"]==CONT_CONTRACT || s["contract"]==FID_CONTRACT || s["contract"]==PREC_CONTRACT {return seal_owner(Path::new(s["previous_study"].as_str().ok_or_else(||bad("continuation seal owner"))?));}
    if s["contract"]==MEAN_CONTRACT {
        Ok(PathBuf::from(s["previous_study"].as_str().ok_or_else(||bad("mean seal owner"))?))
    }else{Ok(study.into())}
}
fn continuation_plan(old:&Plan,study:&Path,s:&binary::Value)->Result<Plan> {
    let fid=s["contract"]==FID_CONTRACT;let prec=s["contract"]==PREC_CONTRACT;
    if !fid && !prec && s["contract"]!=CONT_CONTRACT {return Err(bad("unknown citation continuation profile"));}
    let mut p=old.clone();
    let end:Segment=binary::from_value(s["parent_endpoint"].clone())?;
    let state:TrainingState=binary::from_value(s["parent_training_state"].clone())?;
    p.source=s["source"].as_str().ok_or_else(||bad("continuation source"))?.into();
    p.binary=s["binary"].as_str().ok_or_else(||bad("continuation binary"))?.into();
    p.initial=end.checkpoint_hash;
    p.initial_weights=s["parent_weights"].as_str().ok_or_else(||bad("continuation weights"))?.into();
    p.config.budget_start_step=state.step;p.config.budget_start_tokens=state.consumed_tokens;
    p.config.max_steps=state.step+if p.tiny {2}else if prec {1536}else if fid {UPDATES}else{3040};
    p.config.max_tokens=state.consumed_tokens+if prec {2_000_000}else{4_000_000};
    if prec {p.config.lr=3e-5;}
    let o=p.identifiable.as_mut().unwrap();o.study=study.into();o.dataset=if prec {PREC_DATA}else if fid {FID_DATA}else{CONT_DATA}.into();
    if (fid||prec) && o.rows.len()!=state.step {return Err(bad("citation fork requires exhausted parent tape"));}
    if p.tiny || fid || prec {o.rows.extend(suffix().into_iter().take(if p.tiny {2}else if prec {1536}else{UPDATES}));}
    p.train_order=digest(&o.rows)?;
    let f=p.fork.as_mut().unwrap();f.study=study.into();f.study_hash=file_hash(&study.join("selection.r3b"))?;
    f.parent_policy=digest(old)?;f.parent_state=digest(&state)?;
    f.parent_adam=s["parent_adam"].as_str().ok_or_else(||bad("continuation Adam"))?.into();
    f.origin_step=state.step;f.origin_input=state.consumed_tokens;f.origin_target=state.target_tokens;
    f.target_limit=if prec {130_000}else{260_000};
    if prec {f.constant_lr=3e-5;}
    p.evaluation=evaluation(&p);
    Ok(p)
}
pub(in super::super::super) fn continuation_prepare(previous:&Path,output:&Path,tiny:bool)->Result<()> {
    continuation_prepare_profile(previous,output,tiny,CONT_CONTRACT,None)
}
pub(in super::super::super) fn fidelity_prepare(previous:&Path,output:&Path,report:&Path,tiny:bool)->Result<()> {
    continuation_prepare_profile(previous,output,tiny,FID_CONTRACT,Some(report))
}
pub(in super::super::super) fn precision_prepare(previous:&Path,output:&Path,report:&Path,tiny:bool)->Result<()> {
    continuation_prepare_profile(previous,output,tiny,PREC_CONTRACT,Some(report))
}
fn continuation_parent_profile(old:&Plan,end:&Segment,contract:&str,tiny:bool)->Result<()> {
    let prec=contract==PREC_CONTRACT;let fid=contract==FID_CONTRACT;
    if ![PREC_CONTRACT,FID_CONTRACT,CONT_CONTRACT].contains(&contract) {return Err(bad("citation parent profile"));}
    if old.tiny!=tiny || own(old).dataset!=if prec {FID_DATA}else if fid {CONT_DATA}else{MEAN_DATA} || own(old).arm!=MEAN_ARMS[1]
        || end.resume || end.phase.as_deref()!=Some("Finished")
        || end.step!=if fid||prec {old.config.max_steps}else{old.origin_step()+if tiny {2}else{32}}
        || (!tiny && (end.step!=if prec {10496}else if fid {7424}else{4384}
            || end.stop!=if prec {"FINAL_QUALITY_FAIL_AT_10496"}else if fid {"FINAL_QUALITY_FAIL_AT_7424"}else{"QUALITY_REGRESSION"})) {
        return Err(bad("specific complete ANSWER parent required"));
    }
    Ok(())
}
fn continuation_prepare_profile(previous:&Path,output:&Path,tiny:bool,contract:&str,parent_report:Option<&Path>)->Result<()> {
    let fid=contract==FID_CONTRACT;let prec=contract==PREC_CONTRACT;
    if tiny!=cfg!(all(test,feature="test-support")) {return Err(bad("continuation production/TINY profile"));}
    let previous=previous.canonicalize()?;let output=std::path::absolute(output)?;
    let oldroot=previous.join(MEAN_ARMS[1]);let old=historical_plan(&oldroot)?;
    let (end,d)=close(&oldroot,&old)?;
    continuation_parent_profile(&old,&end,contract,tiny)?;
    let l=checkpoint::load(&oldroot.join(&end.checkpoint),Device::Cpu,true)?;
    let state=l.manifest.training.as_ref().ok_or_else(||bad("continuation parent Adam absent"))?;
    if state.step!=end.step || state.sampler_state!=end.step as u64 || state.resume_binding.as_ref()!=Some(&old.binding(state,&l.tokenizer)?)
        || state.resume_binding.as_ref().is_none_or(|b|b.family!=checkpoint::ANSWER_MEAN_FAMILY || b.normalizer!=2)
        || old.config.lr!=3e-4 || old.config.first_target_weight!=1. || old.config.microbatch!=8 || old.config.accumulation!=1
        || old.framing()!=neural::Framing::QuestionEvidence {return Err(bad("continuation parent native/objective/clock"));}
    unused_seal(&previous)?;unused_seal(&seal_owner(&previous)?)?;
    if fid||prec {
        let mut ancestor=previous.clone();
        loop {
            unused_seal(&ancestor)?;
            let prior:binary::Value=read(&ancestor.join("selection.r3b"))?;
            if ![FID_CONTRACT,CONT_CONTRACT,MEAN_CONTRACT].iter().any(|&c|prior["contract"]==c) {break;}
            ancestor=PathBuf::from(prior["previous_study"].as_str().ok_or_else(||bad("fidelity prior seal chain"))?);
        }
    }
    let c=verified_corpus(&oldroot.join("corpus.r3cor"),&old.corpus)?;
    let rows=suffix();let range=if prec {0..1536}else if fid {0..UPDATES}else{32..UPDATES};
    let costs=tape_cost(&rows[range],&c.train,&l.tokenizer)?;
    let old_cycles=if prec {[rows.clone(),rows.clone()].concat()}else{rows.clone()};
    if !tiny && (own(&old).rows[ORIGIN..]!=old_cycles || costs["input"]!=if prec {1830912}else if fid {3661824}else{3623680}
        || costs["target"]!=if prec {116736}else if fid {233472}else{231040} || costs["padding"]!=if prec {49152}else if fid {98304}else{97280}) {return Err(bad("continuation original suffix"));}
    let evaluation_generations=if prec {192+2*1664+2*4608}else if fid {3*192+2*1664+2*4608}else{3*128+2*192+2*1664+2*4608};
    let generations=evaluation_generations+32+32+512+640;
    if generations>if fid||prec {14336}else{16384} || evaluation_generations>if fid||prec {13120}else{16384} {return Err(bad("continuation evaluation cap"));}
    let mut s=binary::record!({"objective":checkpoint::ANSWER_MEAN_OBJECTIVE,"input_limit":if prec {2000000}else{4000000},"target_limit":if prec {130000}else{260000}});
    if prec {s["parent_lr_bits"]=binary::record!(old.config.lr.to_bits());s["child_lr_bits"]=binary::record!(3e-5f64.to_bits());}
    s["contract"]=binary::record!(contract);s["source"]=binary::record!(source_digest()?);
    s["binary"]=binary::record!(file_hash(&std::env::current_exe()?)?);
    s["previous_study"]=binary::record!(previous);s["previous_preparation"]=binary::record!(file_hash(&previous.join("preparation.r3b"))?);
    s["parent"]=binary::record!(oldroot);s["parent_endpoint"]=binary::record!(end);
    s["parent_policy"]=binary::record!(file_hash(&oldroot.join("plan.r3b"))?);
    s["parent_terminal"]=binary::record!(file_hash(&terminal_path(&oldroot,&end)?)?);
    s["parent_decision"]=binary::record!(file_hash(&oldroot.join(format!("citation-decision-{:04}.r3b",end.step)))?);
    s["parent_training_state"]=binary::record!(state);s["parent_adam"]=binary::record!(optimizer_hash(&l.optimizer)?);
    s["parent_weights"]=binary::record!(l.model.weight_hash()?);s["parent_content"]=binary::record!(l.model.weights_content_id()?);
    s["parent_baseline"]=binary::record!({"dev":{"screen":d["V64"]},"scope":"same complete ANSWER parent V64 raw; no inherited scalar4352 score"});
    s["parent_quality"]=binary::record!(end.stop);s["historical_resume"]=binary::record!(false);
    s["new_authorization"]=binary::record!(contract);s["costs_remaining"]=costs;
    s["planned_generation_upper"]=binary::record!(generations);s["planned_teacher_upper"]=binary::record!(evaluation_generations);
    s["new_updates"]=binary::record!(if tiny {2}else if prec {1536}else if fid {UPDATES}else{3040});
    if let Some(report)=parent_report {
        let report=report.canonicalize()?;
        s["parent_report"]=binary::record!(report);s["parent_report_hash"]=binary::record!(file_hash(&report)?);
        s["half_costs"]=tape_cost(&rows[..if prec {768}else{1536}],&c.train,&l.tokenizer)?;
        if !tiny && (s["half_costs"]["input"]!=if prec {915456}else{1830912} || s["half_costs"]["target"]!=if prec {58368}else{116736} || s["half_costs"]["padding"]!=if prec {24576}else{49152}) {return Err(bad("citation half-interval token costs"));}
    }
    std::fs::create_dir(&output)?;let root=output.join(MEAN_ARMS[1]);std::fs::create_dir(&root)?;
    write(&output.join("selection.r3b"),&s)?;
    for name in ["corpus.r3cor","transfer.r3cor","metadata.r3b","tokenizer.r3b"] {copy_native(&oldroot.join(name),&root.join(name))?;}
    copy_native(&oldroot.join(&end.checkpoint),&root.join("initial.r3m"))?;
    let p=continuation_plan(&old,&output,&s)?;write(&root.join("plan.r3b"),&p)?;
    verify_continuation(&root,&p)?;
    if !p.parent_entry(&root.join("initial.r3m"),&l)? {return Err(bad("continuation parent entry"));}
    publish_confirmed(&output.join("preparation.r3b"),&binary::record!({"contract":contract,"source":p.source,"binary":p.binary,
        "selection":file_hash(&output.join("selection.r3b"))?,"arms":{(MEAN_ARMS[1]):{"policy":file_hash(&root.join("plan.r3b"))?,"initial":p.initial,"corpus":p.corpus,"tape":p.train_order}},
        "optimizer":0,"generation":0,"teacher":0}))?;
    publish_confirmed(&output.join("confirmation-link.r3b"),&binary::record!({"preparation":file_hash(&output.join("preparation.r3b"))?,
        "source_study":previous,"source_seal":file_hash(&seal_owner(&previous)?.join("confirmation-seal.r3b"))?,"status":"VERIFIED_UNUSED"}))?;
    println!("CITATION_CONTINUATION_PREPARED origin={} maximum={} new_updates0 input={} target={} padding={} generations_upper={generations}",p.origin_step(),p.config.max_steps,s["costs_remaining"]["input"],s["costs_remaining"]["target"],s["costs_remaining"]["padding"]);Ok(())
}
fn verify_continuation(root:&Path,p:&Plan)->Result<()> {
    let fid=fidelity(p);let prec=precision(p);let contract=if prec {PREC_CONTRACT}else if fid {FID_CONTRACT}else{CONT_CONTRACT};
    let study=&own(p).study;let s:binary::Value=read(&study.join("selection.r3b"))?;
    let prior=Path::new(s["parent"].as_str().ok_or_else(||bad("continuation parent path"))?);
    let old=historical_plan(prior)?;let end:Segment=binary::from_value(s["parent_endpoint"].clone())?;
    continuation_parent_profile(&old,&end,contract,p.tiny)?;
    if root!=study.join(MEAN_ARMS[1])
        || s["contract"]!=contract || s["new_authorization"]!=contract || s["historical_resume"]!=false
        || s["parent_quality"]!=end.stop || s["previous_study"]!=binary::record!(own(&old).study)
        || s["previous_preparation"]!=file_hash(&own(&old).study.join("preparation.r3b"))?
        || digest(&read_confirmed::<Segment>(&terminal_path(prior,&end)?)?)?!=digest(&end)?
        || s["parent_policy"]!=file_hash(&prior.join("plan.r3b"))? || s["parent_terminal"]!=file_hash(&terminal_path(prior,&end)?)?
        || s["parent_decision"]!=file_hash(&prior.join(format!("citation-decision-{:04}.r3b",end.step)))?
        || *p!=continuation_plan(&old,study,&s)? || p.initial!=file_hash(&prior.join(&end.checkpoint))?
        || p.initial!=file_hash(&root.join("initial.r3m"))? {return Err(bad("continuation immutable parent/policy"));}
    for name in ["corpus.r3cor","transfer.r3cor","metadata.r3b","tokenizer.r3b"] {
        if file_hash(&root.join(name))?!=file_hash(&prior.join(name))? {return Err(bad("continuation owned input"));}
    }
    let c=verified_corpus(&root.join("corpus.r3cor"),&p.corpus)?;let tok=ByteBpe::load(&root.join("tokenizer.r3b"))?;
    let rows=suffix();let range=if prec {0..1536}else if fid {0..UPDATES}else{32..UPDATES};
    if s["costs_remaining"]!=tape_cost(&rows[range],&c.train,&tok)? {return Err(bad("continuation costs changed"));}
    if prec && (s["parent_lr_bits"]!=3e-4f64.to_bits() || s["child_lr_bits"]!=3e-5f64.to_bits()) {return Err(bad("precision actual LR policy"));}
    if fid||prec {
        let report=Path::new(s["parent_report"].as_str().ok_or_else(||bad("fidelity parent report absent"))?);
        let old_cycles=if prec {[rows.clone(),rows.clone()].concat()}else{rows.clone()};
        if s["parent_report_hash"]!=file_hash(report)? || s["half_costs"]!=tape_cost(&rows[..if prec {768}else{1536}],&c.train,&tok)?
            || (!p.tiny && own(&old).rows[ORIGIN..]!=old_cycles) {return Err(bad("citation cycle/report binding"));}
    }
    Ok(())
}
fn continuation_authorize(root:&Path,p:&Plan)->Result<()> {
    verify_continuation(root,p)?;let study=&own(p).study;
    let prep:binary::Value=read_confirmed(&study.join("preparation.r3b"))?;
    if prep["selection"]!=file_hash(&study.join("selection.r3b"))? {return Err(bad("continuation reviewed preparation"));}
    seal_receipt(study)?;
    for (name,citation) in [("continuation-parent-value",false),("continuation-parent-citation",true)] {
        let (es,_,_)=parent_reproduction(root,p,citation)?;
        observed(study,name,p,&p.initial,&es,true)?;
    }
    Ok(())
}
// Evaluation-only selection: complete four-view orbits, wrong orbits first in
// frozen metadata order. Expected answers never enter the generation request.
fn reproduction_indices(p:&Plan,es:&[Episode],ms:&[Meta],rows:&[binary::Value],tok:&ByteBpe)->Result<Vec<usize>> {
    let n=if p.tiny {4}else{16};
    if !precision(p) {return Ok((0..n).collect());}
    if es.len()<n || !es.len().is_multiple_of(4) || ms.len()!=es.len() || rows.len()!=es.len() {return Err(bad("precision reproduction complete panel"));}
    let mut failed=vec![];let mut correct=vec![];
    for (orbit,quad) in rows.as_chunks::<4>().0.iter().enumerate() {
        let mut wrong=false;
        for (view,row) in quad.iter().enumerate() {
            let i=orbit*4+view;let e=&es[i];let m=&ms[i];
            if m.id!=e.id || m.view!=view || m.base!=ms[orbit*4].base || row["id"]!=e.id || row["expected"]!=e.answer {return Err(bad("precision reproduction frozen content"));}
            verify_generated(row,tok)?;
            wrong|=row["actual"]!=e.answer || !row["error"].is_null() || row["generation_completed"]!=true || row["finish_reason"]!="stop";
        }
        if wrong {failed.push(orbit);}else{correct.push(orbit);}
    }
    Ok(failed.into_iter().chain(correct).take(n/4).flat_map(|base|base*4..base*4+4).collect())
}
fn parent_reproduction(root:&Path,p:&Plan,citation:bool)->Result<(Vec<Episode>,Vec<binary::Value>,Vec<usize>)> {
    let s:binary::Value=read(&own(p).study.join("selection.r3b"))?;
    let prior=Path::new(s["parent"].as_str().ok_or_else(||bad("citation parent reproduction path"))?);
    let count=if p.tiny {4}else if fidelity(p)||precision(p) {512}else{64};let start=if citation {512}else{0};
    let raw=binary::read_value_records(&prior.join(format!("eval-{:04}-{}{count}.r3rows",p.origin_step(),if citation {"citation"}else{"value"})))?;
    let c=verified_corpus(&root.join("corpus.r3cor"),&p.corpus)?;let (_,dm,_)=verified_metadata(root,p)?;
    let tok=ByteBpe::load(&root.join("tokenizer.r3b"))?;
    let indices=reproduction_indices(p,&c.validation[start..start+count],&dm[start..start+count],&raw[1..],&tok)?;
    Ok((indices.iter().map(|&i|c.validation[start+i].clone()).collect(),indices.iter().map(|&i|raw[i+1].clone()).collect(),indices))
}
fn endpoint_reproduction(root:&Path,p:&Plan,step:usize,citation:bool)->Result<(Vec<Episode>,Vec<binary::Value>,Vec<usize>)> {
    let panel=panels(root,p,step)?.into_iter().find(|(n,_,_)|n.starts_with(if citation {"citation"}else{"value"})).ok_or_else(||bad("mean reproduction panel"))?;
    let raw=binary::read_value_records(&root.join(format!("eval-{step:04}-{}.r3rows",panel.0)))?;
    let tok=ByteBpe::load(&root.join("tokenizer.r3b"))?;
    let indices=reproduction_indices(p,&panel.1,&panel.2,&raw[1..],&tok)?;
    Ok((indices.iter().map(|&i|panel.1[i].clone()).collect(),indices.iter().map(|&i|raw[i+1].clone()).collect(),indices))
}
fn verify_review_observations(study:&Path,root:&Path,p:&Plan,step:usize,checkpoint:&str)->Result<()> {
    for citation in [false,true] {
        let kind=if citation {"citation"}else{"value"};
        let (name,es)=if is_mean(p) {
            (format!("mean-review-{}-{kind}",own(p).arm),endpoint_reproduction(root,p,step,citation)?.0)
        }else {
            let c=verified_corpus(&root.join("corpus.r3cor"),&p.corpus)?;
            let start=if citation {512}else{0};let n=if p.tiny {4}else{16};
            (format!("citation-review-{kind}"),c.validation[start..start+n].to_vec())
        };
        observed(study,&name,p,checkpoint,&es,true)?;
    }
    Ok(())
}
pub(in super::super::super) fn continuation_parent(study:&Path,citation:bool)->Result<()> {
    let study=study.canonicalize()?;let root=selected_root(&study);let p=plan_read(&root)?;
    if !continuation(&p) || !history(&root,&p)?.is_empty() {return Err(bad("continuation parent observation before learning"));}
    expansion_review(&p)?;
    let (es,raw,indices)=parent_reproduction(&root,&p,citation)?;let n=es.len();
    let name=if citation {"continuation-parent-citation"}else{"continuation-parent-value"};
    orbit_observe(&study,name,&root.join("initial.r3m"),&es,
        &binary::record!({"policy":digest(&p)?,"checkpoint":p.initial,"sample_indices":indices}),Some(&raw),observation_control(&p,n,0)?)?;
    println!("CONTINUATION_PARENT {name} matched{n} generation{n} teacher0 optimizer0");Ok(())
}
pub(in super::super::super) fn mean_prepare(previous:&Path,output:&Path,tiny:bool)->Result<()> {
    if tiny != cfg!(all(test,feature="test-support")) {
        return Err(bad("explicit mean production/TINY profile"));
    }
    let previous=previous.canonicalize()?;let output=std::path::absolute(output)?;
    let oldroot=previous.join(ARM);let old=historical_plan(&oldroot)?;
    if own(&old).dataset!=DATA || old.tiny!=tiny {return Err(bad("original citation data required"));}
    let (end,decision)=close(&oldroot,&old)?;
    if end.step!=old.origin_step()+if tiny {2}else{32}
        || (!tiny && end.stop!="QUALITY_REGRESSION") {
        return Err(bad("expected closed citation32 reference"));
    }
    let prior:binary::Value=read(&previous.join("selection.r3b"))?;
    unused_seal(&previous)?;
    let c=verified_corpus(&oldroot.join("corpus.r3cor"),&old.corpus)?;
    let tok=ByteBpe::load(&oldroot.join("tokenizer.r3b"))?;
    let full=tape_cost(&suffix(),&c.train,&tok)?;
    let first=tape_cost(&suffix()[..32],&c.train,&tok)?;
    if (!tiny && (first["input"]!=38144 || first["target"]!=2432 || first["padding"]!=1024
        || full["input"]!=3661824 || full["target"]!=233472 || full["padding"]!=98304))
        || own(&old).rows[old.origin_step()..] != suffix()[..if tiny {2}else{UPDATES}] {
        return Err(bad("frozen citation tape/cost mismatch"));
    }
    let generated=16+128+2*128+2*192+2*1664+2*4608+64+512;
    let teachers=128+2*128+2*192+2*1664+2*4608;
    if generated>16384 || teachers>16384 {return Err(bad("mean panel budget"));}
    std::fs::create_dir(&output)?;
    let mut s=prior.clone();
    s["contract"]=binary::record!(MEAN_CONTRACT);s["source"]=binary::record!(source_digest()?);
    s["binary"]=binary::record!(file_hash(&std::env::current_exe()?)?);
    s["previous_study"]=binary::record!(previous);
    s["previous_preparation"]=binary::record!(file_hash(&previous.join("preparation.r3b"))?);
    s["previous_selection"]=binary::record!(file_hash(&previous.join("selection.r3b"))?);
    s["previous_plan"]=binary::record!(file_hash(&oldroot.join("plan.r3b"))?);
    s["reference_terminal"]=binary::record!(file_hash(&terminal_path(&oldroot,&end)?)?);
    s["reference_endpoint"]=binary::to_value(&end)?;s["reference_decision"]=decision;
    s["source_seal"]=binary::record!(file_hash(&previous.join("confirmation-seal.r3b"))?);
    s["costs32"]=first;s["costs3072"]=full;
    s["objective"]=binary::record!(checkpoint::ANSWER_MEAN_OBJECTIVE);
    s["new_updates"]=binary::record!(3104);s["input_limit"]=binary::record!(4_000_000);
    s["target_limit"]=binary::record!(260_000);
    s["planned_generation_upper"]=binary::record!(generated);s["planned_teacher_upper"]=binary::record!(teachers);
    s["scalar_tolerance"]=binary::record!(2e-6);s["native_gradient_tolerance"]=binary::record!(2e-5);
    s["control_raw_tolerance"]=binary::record!("exact tokens/text/finish/EOS/errors; tensor/Adam bitwise");
    write(&output.join("selection.r3b"),&s)?;
    let mut plans=BTreeMap::new();
    for arm in MEAN_ARMS {
        let root=output.join(arm);std::fs::create_dir(&root)?;
        for name in ["corpus.r3cor","transfer.r3cor","metadata.r3b","tokenizer.r3b","initial.r3m"] {
            copy_native(&oldroot.join(name),&root.join(name))?;
        }
        let p=mean_plan(&old,&output,&s,arm)?;write(&root.join("plan.r3b"),&p)?;
        verify_mean_plan(&root,&p)?;
        plans.insert(arm,binary::record!({"policy":file_hash(&root.join("plan.r3b"))?,"initial":p.initial,"corpus":p.corpus,
            "metadata":p.metadata,"tokenizer":p.tokenizer,"tape":p.train_order,"objective":if answer_mean(&p){checkpoint::ANSWER_MEAN_OBJECTIVE}else{"response_ce_token_mean"}}));
    }
    publish_confirmed(&output.join("preparation.r3b"),&binary::record!({"contract":MEAN_CONTRACT,"source":s["source"],"binary":s["binary"],
        "selection":file_hash(&output.join("selection.r3b"))?,"arms":plans,"source_seal":s["source_seal"],"optimizer":0,"generation":0,"teacher":0}))?;
    publish_confirmed(&output.join("confirmation-link.r3b"),&binary::record!({"preparation":file_hash(&output.join("preparation.r3b"))?,
        "source_study":previous,"source_seal":s["source_seal"],"status":"VERIFIED_UNUSED"}))?;
    println!("ANSWER_MEAN_PREPARED TOKEN32 ANSWER3072 actual_plan_input3699968 target235904 padding99328 model_calls0 A_PENDING");
    Ok(())
}
fn verify_mean_plan(root:&Path,p:&Plan)->Result<()> {
    let o=own(p);let s:binary::Value=read(&o.study.join("selection.r3b"))?;
    let previous=Path::new(s["previous_study"].as_str().ok_or_else(||bad("previous citation root"))?);
    let oldroot=previous.join(ARM);let old=historical_plan(&oldroot)?;
    let end:Segment=binary::from_value(s["reference_endpoint"].clone())?;
    if !is_mean(p) || root!=o.study.join(&o.arm) || *p!=mean_plan(&old,&o.study,&s,&o.arm)?
        || s["contract"]!=MEAN_CONTRACT || s["objective"]!=checkpoint::ANSWER_MEAN_OBJECTIVE
        || s["previous_preparation"]!=file_hash(&previous.join("preparation.r3b"))?
        || s["previous_selection"]!=file_hash(&previous.join("selection.r3b"))?
        || s["previous_plan"]!=file_hash(&oldroot.join("plan.r3b"))?
        || s["reference_terminal"]!=file_hash(&terminal_path(&oldroot,&end)?)?
        || end.checkpoint_hash!=file_hash(&oldroot.join(&end.checkpoint))?
        || s["source_seal"]!=file_hash(&previous.join("confirmation-seal.r3b"))? {
        return Err(bad("answer-mean immutable source/parent/policy"));
    }
    for name in ["corpus.r3cor","transfer.r3cor","metadata.r3b","tokenizer.r3b","initial.r3m"] {
        if file_hash(&root.join(name))?!=file_hash(&oldroot.join(name))? {return Err(bad("mean frozen owned bytes"));}
    }
    let c=verified_corpus(&root.join("corpus.r3cor"),&p.corpus)?;
    let tok=ByteBpe::load(&root.join("tokenizer.r3b"))?;
    if s["costs32"]!=tape_cost(&suffix()[..32],&c.train,&tok)? || s["costs3072"]!=tape_cost(&suffix(),&c.train,&tok)? {
        return Err(bad("mean immutable costs"));
    }
    seal_receipt(previous)?;
    Ok(())
}
fn reference_match(study:&Path)->Result<binary::Value> {
    let root=study.join(MEAN_ARMS[0]);let p=historical_plan(&root)?;
    let h=history(&root,&p)?;let e=h.last().ok_or_else(||bad("TOKEN reference not run"))?;
    if !mean_reference_complete(e,p.config.max_steps) {
        return Err(bad("TOKEN reference incomplete/cancelled/unknown"));
    }
    let s:binary::Value=read(&study.join("selection.r3b"))?;
    let prior=Path::new(s["previous_study"].as_str().ok_or_else(||bad("reference path"))?).join(ARM);
    let oldend:Segment=binary::from_value(s["reference_endpoint"].clone())?;
    let d=evaluation_result(&root,&p,e.step)?;
    if read_confirmed::<binary::Value>(&root.join(format!("citation-decision-{:04}.r3b",e.step)))?!=d || d["stop"]!=e.stop {
        return Err(bad("TOKEN reference decision mismatch"));
    }
    trace(&root,&p,e)?;
    let a=checkpoint::load(&root.join(&e.checkpoint),Device::Cpu,true)?;
    let b=checkpoint::load(&prior.join(&oldend.checkpoint),Device::Cpu,true)?;
    if a.model.weights_content_id()?!=b.model.weights_content_id()? || optimizer_hash(&a.optimizer)?!=optimizer_hash(&b.optimizer)? {
        return Err(bad("TOKEN reference tensor/Adam mismatch"));
    }
    let mut count=0;
    for (name,es,_) in panels(&root,&p,e.step)? {
        let path=format!("eval-{:04}-{name}.r3rows",e.step);
        let a=binary::read_value_records(&root.join(&path))?;let b=binary::read_value_records(&prior.join(&path))?;
        if a.len()!=es.len()+1 || a.len()!=b.len() {return Err(bad("TOKEN reference raw count"));}
        for (a,b) in a[1..].iter().zip(&b[1..]) {
            for k in ["actual","raw_tokens","finish_reason","generation_completed","error","error_class","expected","question","generated_evidence"] {
                if a[k]!=b[k] {return Err(bad("TOKEN reference raw parity mismatch"));}
            }
            count+=1;
        }
    }
    Ok(binary::record!({"matched":count,"tensor":a.model.weights_content_id()?,"adam":optimizer_hash(&a.optimizer)?,"step":e.step,
        "candidate":null,"resume":false,"reference_only":true,"quality":false}))
}
fn mean_reference_complete(e:&Segment,maximum:usize)->bool {
    !e.resume && e.phase.as_deref()==Some("Finished") && e.step==maximum
        && (e.stop=="QUALITY_REGRESSION" || e.stop==format!("FINAL_QUALITY_FAIL_AT_{}",e.step))
}
fn mean_authorize(root:&Path,p:&Plan)->Result<()> {
    let study=&own(p).study;
    if root.canonicalize()?!=study.join(&own(p).arm).canonicalize()? {return Err(bad("mean root"));}
    let prep:binary::Value=read_confirmed(&study.join("preparation.r3b"))?;
    if prep["selection"]!=file_hash(&study.join("selection.r3b"))? {return Err(bad("mean preparation binding"));}
    seal_receipt(study)?;
    let token=historical_plan(&study.join(MEAN_ARMS[0]))?;
    let c=verified_corpus(&root.join("corpus.r3cor"),&p.corpus)?;
    observed(study,"mean-parent",&token,&token.initial,&c.validation[..if p.tiny {4}else{16}],true)?;
    if answer_mean(p) {reference_match(study)?;}
    // An incomplete/cancelled peer never inherits the reference-only exception.
    for arm in MEAN_ARMS {
        let peer=study.join(arm);let pp=historical_plan(&peer)?;let hh=history(&peer,&pp)?;
        if hh.last().is_some_and(|e|!e.resume && (e.phase.as_deref()!=Some("Finished")
            || !["QUALITY_REGRESSION".to_string(),format!("FINAL_QUALITY_FAIL_AT_{}",e.step),format!("CANDIDATE_FIXED_AT_{}",e.step)].contains(&e.stop))) {
            return Err(bad("mean failed peer execution"));
        }
    }
    Ok(())
}
pub(in super::super::super) fn mean_parent(study:&Path)->Result<()> {
    let study=study.canonicalize()?;let root=study.join(MEAN_ARMS[0]);let p=plan_read(&root)?;
    expansion_review(&p)?;
    for arm in MEAN_ARMS {if !history(&study.join(arm),&plan_read(&study.join(arm))?)?.is_empty() {return Err(bad("mean parent must precede updates"));}}
    let s:binary::Value=read(&study.join("selection.r3b"))?;
    let prior=Path::new(s["parent"].as_str().ok_or_else(||bad("mean parent path"))?);
    let old:Plan=read(&prior.join("plan.r3b"))?;
    let raw=binary::read_value_records(&prior.join(format!("eval-{:04}-{}.r3rows",p.origin_step(),if old.tiny {"dev4"}else{"dev512"})))?;
    let c=verified_corpus(&root.join("corpus.r3cor"),&p.corpus)?;
    let n=if p.tiny {4}else{16};
    orbit_observe(&study,"mean-parent",&root.join("initial.r3m"),&c.validation[..n],
        &binary::record!({"policy":digest(&p)?,"checkpoint":p.initial}),Some(&raw[1..n+1]),observation_control(&p,n,0)?)?;
    println!("ANSWER_MEAN_PARENT matched{n} generation{n} optimizer0 teacher0");Ok(())
}
pub(in super::super::super) fn mean_report(study:&Path)->Result<()> {
    let study=study.canonicalize()?;
    let selected=historical_plan(&selected_root(&study))?;
    if retained_qa(&selected){return qa_report(&study);}
    for &arm in arms(&selected) {
        let root=study.join(arm);let p=historical_plan(&root)?;
        let h=history(&root,&p)?;
        if let Some(end)=h.last() {
            println!("ANSWER_MEAN_TRACE arm={arm} {}",trace(&root,&p,end)?);
            for &step in p.evaluation.train_steps.iter().filter(|&&s|s<=end.step) {
                if step==end.step && end.phase.as_deref()==Some("EvaluationPending") {continue;}
                let d=evaluation_result(&root,&p,step)?;
                if d!=read_confirmed::<binary::Value>(&root.join(format!("citation-decision-{step:04}.r3b")))? {return Err(bad("mean report decision"));}
                println!("ANSWER_MEAN_PANEL arm={arm} {d}");
            }
            println!("ANSWER_MEAN_STATE arm={arm} step={} stop={} resume={} checkpoint={} usage={:?}",end.step,end.stop,end.resume,end.checkpoint_hash,work(&p)?);
            if answer_mean(&p) && !end.resume && end.phase.as_deref()==Some("Finished") {
                let (_,d)=close(&root,&p)?;
                println!("ANSWER_MEAN_COMPARISON {}",comparison(&study,&p,end,&d)?);
                if study.join("confirmation-result.r3b").exists() {
                    println!("ANSWER_MEAN_CONFIRMATION {}",verified_confirmation(&study,&p,end,&d)?);
                }
            }
        }else {println!("ANSWER_MEAN_STATE arm={arm} NOT_RUN");}
    }
    if !continuation(&selected) && !history(&study.join(MEAN_ARMS[0]),&historical_plan(&study.join(MEAN_ARMS[0]))?)?.is_empty() {
        println!("TOKEN_REFERENCE {}",reference_match(&study)?);
    }
    println!("GOAL1_READY=false GOAL1_ACCEPTED=false");Ok(())
}
pub(in super::super::super) fn mean_review(root:&Path,citation:bool)->Result<()> {
    let root=root.canonicalize()?;let p=historical_plan(&root)?;
    if !is_mean(&p) {return Err(bad("mean reviewer scope"));}
    let (end,_)=close(&root,&p)?;
    if !["QUALITY_REGRESSION".to_string(),format!("FINAL_QUALITY_FAIL_AT_{}",end.step),format!("CANDIDATE_FIXED_AT_{}",end.step)].contains(&end.stop)
        && !((continuation(&p)||retained_qa(&p))&&["SEVERE_RETENTION_REGRESSION","PERSISTENT_RETENTION_REGRESSION"].contains(&end.stop.as_str()))
        && !(retained_qa(&p)&&end.stop=="STUDY_COMPLETE_QUALITY_FAIL") {
        return Err(bad("mean reviewer requires complete quality endpoint"));
    }
    let (es,old,indices)=endpoint_reproduction(&root,&p,end.step,citation)?;let n=es.len();
    let name=format!("mean-review-{}-{}",own(&p).arm,if citation {"citation"}else{"value"});
    orbit_observe(&own(&p).study,&name,&root.join(&end.checkpoint),&es,
        &binary::record!({"policy":digest(&p)?,"checkpoint":end.checkpoint_hash,"sample_indices":indices}),Some(&old),observation_control(&p,n,0)?)?;
    println!("ANSWER_MEAN_REVIEW arm={} matched{n} generation{n} optimizer0 teacher0 candidate_permission=false",own(&p).arm);Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn bridge_returned_finalization_process() -> Result<()> {
        const NAME:&str="bridge-qa-primary";
        const TEST:&str="training::fresh::identifiable::binding::citation::tests::bridge_returned_finalization_process";
        let original=PathBuf::from(std::env::var_os("R3_RETURNED_FIXTURE").ok_or_else(||bad("preserved four-row RETURNED fixture required; no model calls permitted"))?);
        let root=original.join(MEAN_ARMS[1]);let p=historical_plan(&root)?;
        if !p.tiny{return Err(bad("four-row TINY fixture only"));}
        let(end,d)=bridge_review_endpoint(&root,&p)?;bridge_diagnostic_admission(&p,&end,&d)?;
        verify_review_b(&original,&comparison(&original,&p,&end,&d)?)?;
        let panel=bridge_diagnostic_cases(&original,&p,false)?;assert_eq!(panel.1.len(),4);
        let tok=ByteBpe::load(&root.join("tokenizer.r3b"))?;
        let binding:binary::Value=read_confirmed(&original.join(format!("{NAME}-started.r3b")))?;
        let prior=bridge_diagnostic_prior_usage(&p,NAME)?;
        if let Some(dir)=std::env::var_os("R3_RETURNED_CHILD") {
            let result=bridge_diagnostic_output(Path::new(&dir),&root,&p,&end,&panel,&tok,NAME,&binding["identity"],prior);
            let expected=std::env::var("R3_RETURNED_EXPECT").unwrap();
            if expected=="PASS" {result?;}else{let error=result.expect_err("invalid journals must fail").to_string();
                assert!(!error.contains("FORBIDDEN_MODEL_ENTRY"),"model entry attempted");assert!(error.contains(&expected),"expected {expected}: {error}");}
            println!("RETURNED_PROCESS optimizer=0 generation=0 teacher=0 backward=0 result={expected}");return Ok(());
        }
        if !cfg!(feature="test-support"){return Err(bad("test-support required"));}
        let base=PathBuf::from(std::env::var_os("R3_RETURNED_OUTPUT").ok_or_else(||bad("new evidence output required"))?);
        std::fs::create_dir(&base)?;let base=base.canonicalize()?;
        let original_hash=file_hash(&original.join(format!("{NAME}.r3rows")))?;
        let copy=|label:&str|->Result<PathBuf>{let dir=base.join(label);std::fs::create_dir(&dir)?;
            for entry in std::fs::read_dir(&original)?{let entry=entry?;let n=entry.file_name();let s=n.to_string_lossy();
                if s.starts_with(NAME)&&s!=format!("{NAME}-finished.r3b")&&s!=format!("{NAME}-score.r3b")&& !s.contains("finalization"){std::fs::copy(entry.path(),dir.join(n))?;}}
            let first:binary::Value=read_confirmed(&dir.join(format!("{NAME}-segment-000-finished.r3b")))?;
            let path=dir.join(format!("{NAME}-segment-001-finished.r3b"));let mut last:binary::Value=read_confirmed(&path)?;
            last["control"]["elapsed_seconds"]=binary::record!(p.evaluation.active_seconds as f64-prior.0-first["control"]["elapsed_seconds"].as_f64().unwrap());
            last["control"]["terminal_reason"]=binary::record!("TIME_BUDGET");last["control"]["observed_conditions"]=binary::record!(["TIME_BUDGET"]);last["error"]=binary::record!("model: TIME_BUDGET");
            std::fs::write(path,binary::to_storage_vec(&last)?)?;Ok(dir)};
        let child=|dir:&Path,expected:&str,fault:bool,label:&str|->Result<()>{let mut cmd=std::process::Command::new(std::env::current_exe()?);
            cmd.args(["--exact",TEST,"--nocapture"]).env("R3_RETURNED_CHILD",dir).env("R3_RETURNED_EXPECT",expected).env("R3_FINALIZATION_NO_MODEL","1");
            if fault{cmd.env("R3_FRESH_PUBLISH_FAULT","finished-sync");}
            let out=cmd.output()?;std::fs::write(base.join(format!("{label}.stdout")),&out.stdout)?;std::fs::write(base.join(format!("{label}.stderr")),&out.stderr)?;
            assert!(out.status.success(),"{label}: {} {}",String::from_utf8_lossy(&out.stdout),String::from_utf8_lossy(&out.stderr));assert!(String::from_utf8_lossy(&out.stdout).contains("1 passed"));Ok(())};
        let success=copy("complete")?;assert_eq!(segmented_usage(&success,NAME)?.0+prior.0,p.evaluation.active_seconds as f64);
        // Boundary reader and the actual shared caller run on the same bytes.
        assert_eq!(segmented_returned(&success,NAME,&binding,&panel.1,&tok)?.0.len(),4);
        child(&success,"PASS",false,"complete")?;
        let hashes=["r3rows","finished.r3b","score.r3b"].map(|suffix|file_hash(&success.join(if suffix=="r3rows"{format!("{NAME}.{suffix}")}else{format!("{NAME}-{suffix}")})).unwrap());
        child(&success,"PASS",false,"idempotent")?;
        assert_eq!(hashes,["r3rows","finished.r3b","score.r3b"].map(|suffix|file_hash(&success.join(if suffix=="r3rows"{format!("{NAME}.{suffix}")}else{format!("{NAME}-{suffix}")})).unwrap()));
        for label in ["missing","unknown","model","policy","order","truncated","extra","duplicate","cancel","mixed","remaining"] {
            let dir=copy(label)?;let raw=dir.join(format!("{NAME}.r3rows"));let mut rows=binary::read_value_records(&raw)?;
            let lastpath=dir.join(format!("{NAME}-segment-001-finished.r3b"));let mut last:binary::Value=read_confirmed(&lastpath)?;
            match label {
                "missing"=>{std::fs::remove_file(dir.join(format!("{NAME}-generation-0003-000-resolved.r3b")))?;},
                "unknown"=>{std::fs::remove_file(&lastpath)?;},
                "model"|"policy"=>{let mut b=binding.clone();if label=="model"{b["checkpoint"]=binary::record!("wrong");}else{b["identity"]["policy"]=binary::record!("wrong");}std::fs::write(dir.join(format!("{NAME}-started.r3b")),binary::to_storage_vec(&b)?)?;},
                "cancel"|"mixed"=>{last["control"]["observed_conditions"]=binary::record!(["TIME_BUDGET",if label=="cancel"{"CANCELLED"}else{"IO_ERROR"}]);std::fs::write(&lastpath,binary::to_storage_vec(&last)?)?;},
                "truncated"=>{let b=std::fs::read(&raw)?;std::fs::write(&raw,&b[..b.len()-1])?;},
                _=>{match label{"order"=>rows.swap(1,2),"extra"=>rows.push(rows[1].clone()),"duplicate"=>rows[2]=rows[1].clone(),"remaining"=>{rows.pop();
                        for suffix in ["prepared","resolved"]{std::fs::remove_file(dir.join(format!("{NAME}-generation-0003-000-{suffix}.r3b")))?;}
                        last["returned_after"]=binary::record!(3);last["control"]["generation_calls"]=binary::record!(1);
                        last["prefix"]=binary::record!(digest(&rows)?);last["call_records"]=binary::to_value(segmented_call_files(&dir,NAME)?)?;
                        last["generated_tokens"]=binary::record!(rows[3]["raw_tokens"].as_array().unwrap().len());std::fs::write(&lastpath,binary::to_storage_vec(&last)?)?;},_=>unreachable!()}
                    let mut f=std::fs::File::create(&raw)?;for r in &rows{binary::write_value_record(&mut f,r)?;}}
            }
            child(&dir,if label=="remaining"{"budget exhausted"}else{""},false,label)?;
            assert!(!dir.join(format!("{NAME}-finished.r3b")).exists());
        }
        let sync=copy("sync")?;child(&sync,"sync error AFTER",true,"sync")?;
        assert!(sync.join(format!("{NAME}-finished.r3b")).exists());assert!(pending_path(&sync.join(format!("{NAME}-finished.r3b"))).exists());
        child(&sync,"publication UNKNOWN",false,"sync-sticky")?;
        assert_eq!(file_hash(&original.join(format!("{NAME}.r3rows")))?,original_hash);
        println!("FINALIZATION_PROCESS 15 child processes; returned4 cap7200; no-call strict positives/negatives; optimizer0 generation0 teacher0 backward0 evidence={}",base.display());Ok(())
    }
    fn generic_resume_rejected(path:&Path)->Result<()> {
        let l=checkpoint::load(path,Device::Cpu,true)?;
        assert_eq!(l.manifest.training.as_ref().unwrap().resume_binding.as_ref().unwrap().family,checkpoint::ANSWER_MEAN_FAMILY);
        for extend_lr in [None,Some(3e-4)] {
            let output=path.with_extension("blocked-generic-output");
            let result=crate::training::train(crate::training::Run{checkpoint:path,corpus:None,output:&output,resume:true,
                numeric_probe:false,config:l.manifest.training.as_ref().unwrap().config.clone(),stop_after:None,measure_rss:false,
                extend_steps:None,extend_microbatch:None,extend_sample_group_size:None,extend_curriculum_steps:None,
                extend_first_target_weight:None,extend_lr,extend_warmup:None,source_id:None,replace_corpus:false},
                std::sync::Arc::new(AtomicBool::new(false)));
            assert!(result.unwrap_err().to_string().contains("OBJECTIVE_POLICY_UNSUPPORTED"));assert!(!output.exists());
        }
        println!("PRECISION_GENERIC_RESUME same/different-LR rejected optimizer0 generation0 teacher0");Ok(())
    }
    fn qa_fixture_sources(base:&Path)->Result<(PathBuf,PathBuf)> {
        let balanced=base.join("balanced");let old=base.join("old-qa");
        std::fs::create_dir(&balanced)?;std::fs::create_dir(&old)?;
        let (tr,tm)=super::super::super::generate_balanced(512,0,20260921)?;
        let (dv,dm)=super::super::super::generate_balanced(32,1,20260921)?;
        let (tx,xm)=super::super::super::generate_balanced(8,2,20260921)?;
        data::native::write(&balanced.join("corpus.r3cor"),&super::super::super::new_corpus(tr.clone(),dv,20260921)?,true)?;
        data::native::write(&balanced.join("transfer.r3cor"),&super::super::super::new_corpus(tr,tx,20260921)?,true)?;
        write(&balanced.join("metadata.r3b"),&(tm,dm,xm))?;
        let (tr,tm)=super::super::super::super::generate(2,0,20260919)?;
        let (dv,dm)=super::super::super::super::generate(16,1,20260920)?;
        let (tx,xm)=super::super::super::super::generate(4,2,20260921)?;
        data::native::write(&old.join("corpus.r3cor"),&super::super::super::super::corpus(tr.clone(),dv,20260919)?,true)?;
        data::native::write(&old.join("transfer.r3cor"),&super::super::super::super::corpus(tr,tx,20260919)?,true)?;
        write(&old.join("metadata.r3b"),&(tm,dm,xm))?;Ok((balanced,old))
    }
    #[test]
    fn retained_qa_tape_input_objective()->Result<()> {
        let t=tempfile::tempdir()?;let study=super::super::tests::orbit_fixture(t.path())?;
        let root=study.join("BOTH");let mut p=plan_read(&root)?;let tok=ByteBpe::load(&root.join("tokenizer.r3b"))?;
        let(c,tm,dm)=source_pool(&root,&p,&tok)?;let(public,_,used)=fixture_reservation(&root,t.path())?;
        let(r,(mut rm,_,_),_)=make_pool(&c,&tm,&dm,&tok,&read_ids(&public,128)?,&read_ids(&used,128)?)?;
        let(balanced,old)=qa_fixture_sources(t.path())?;let(q,_,qm,_,_,_,_,_,_)=qa_sources(&balanced,&old)?;
        let(v,vm)=qa_variant(&q.train,&qm)?;
        for ((a,b),(ma,mb)) in q.train.iter().zip(&v).zip(qm.iter().zip(&vm)) {
            let mut b=b.clone();b.id=a.id.clone();b.request.request_id=a.request.request_id.clone();b.request.input=a.request.input.clone();
            assert_eq!(digest(a)?,digest(&b)?);assert_eq!(ma.base,mb.base);assert_eq!(ma.view,mb.view);assert_eq!(mb.source_id.as_deref(),Some(a.id.as_str()));
        }
        let mut es=r.train;es.extend(q.train);es.extend(v);rm.extend(qm);rm.extend(vm);
        let rows=qa_tape(&rm)?;let cost=qa_token_cost(&es,&rows,&tok)?;
        let count:Vec<usize>=binary::from_value(cost["counts"].clone())?;
        assert_eq!(rows.len(),4096);assert!(count[4608..].iter().all(|&n|n==1));
        assert_eq!(count[..1536].iter().sum::<usize>(),8192);assert_eq!(count[1536..3072].iter().sum::<usize>(),4096);assert_eq!(count[3072..4608].iter().sum::<usize>(),4096);
        for(step,row)in rows.iter().enumerate(){for j in 4..8 {assert_eq!(row[j]-step/2048*8192,rows[step%2048][j]);}for pair in row.chunks_exact(2){assert_eq!(pair[1],pair[0]+1);assert_eq!(rm[pair[0]].base,rm[pair[1]].base);}}
        let mut duplicated=es.clone();duplicated.push(es[0].clone());assert!(data::validate_episodes(&duplicated).is_err());
        let ss=samples(&es,&tok,512)?;let ids=rows[0];let b=batch(&ss,&ids,&Device::Cpu)?;
        assert!(ids.iter().map(|&i|ss[i].tokens.len()-ss[i].response_start).collect::<BTreeSet<_>>().len()>1);
        let vocab=tok.vocab_size();let width=b.input.dim(1)?;
        let z=candle_core::Var::from_vec((0..8*width*vocab).map(|i|((i%19)as f32-9.)/13.).collect::<Vec<_>>(),(8,width,vocab),&Device::Cpu)?;
        let (_,loss,n,den)=crate::training::response_objective(z.as_tensor(),&b,1.,true)?;
        assert_eq!(den,8);assert_eq!(n,ids.iter().map(|&i|ss[i].tokens.len()-ss[i].response_start).sum::<usize>());
        let grad=loss.backward()?.get(&z).unwrap().flatten_all()?.to_vec1::<f32>()?;let mut accumulated=vec![0f32;grad.len()];let mut value=0.;
        for offset in [0,4]{let mb=batch(&ss,&ids[offset..offset+4],&Device::Cpu)?;let w=mb.input.dim(1)?;let local=z.narrow(0,offset,4)?.narrow(1,0,w)?;
            let(_,l,_,d)=crate::training::response_objective(&local,&mb,1.,true)?;assert_eq!(d,4);value+=f64::from(l.to_scalar::<f32>()?)/2.;
            let g=(l*0.5)?.backward()?.get(&z).unwrap().flatten_all()?.to_vec1::<f32>()?;for(a,b)in accumulated.iter_mut().zip(g){*a+=b;}}
        assert!((value-f64::from(loss.to_scalar::<f32>()?)).abs()<2e-5);assert!(grad.iter().zip(&accumulated).all(|(a,b)|(a-b).abs()<2e-6));
        let masks=b.mask.to_vec2::<f32>()?;for row in 0..8 {let s=&ss[ids[row]];assert_eq!(s.tokens.last(),Some(&EOS));assert_eq!(masks[row][s.tokens.len()-2],1.);for pos in 0..width{if masks[row][pos]==0.{assert!(grad[(row*width+pos)*vocab..(row*width+pos+1)*vocab].iter().all(|x|*x==0.));}}}
        assert!(es[..4608].iter().all(|e|e.request.limits.max_tokens==32));assert!(es[4608..].iter().all(|e|e.request.limits.max_tokens==128));
        p.tiny=false;p.identifiable.as_mut().unwrap().dataset=QA_DATA.into();p.identifiable.as_mut().unwrap().arm=MEAN_ARMS[1].into();
        p.fork=Some(Fork{study,study_hash:String::new(),arm:MEAN_ARMS[1].into(),parent_policy:String::new(),parent_state:String::new(),parent_adam:String::new(),origin_step:11264,origin_input:0,origin_target:0,original_corpus:p.corpus.clone(),tokenizer_training_hash:tok.train_hash.clone(),variants:None,variant_metadata:None,alternate_first:vec![],selector:None,selector_metadata:None,flip_first:vec![],constant_lr:3e-5,target_limit:2400000});
        p.config.max_steps=15360;p.evaluation=evaluation(&p);assert_eq!(p.evaluation.train_steps,vec![11392,11776,12288,13312,14336,15360]);
        assert_eq!(endpoint(&p,11264,false)?,11265);assert_eq!(endpoint(&p,15360,true)?,15360);assert!(endpoint(&p,15360,false).is_err());
        println!("RETAINED_QA_TAPE rows20992 samples32768 cost_input={} target={} mixed_gradient_8_vs_4plus4=PASS prompt_padding_zero/EOS_included optimizer0 generation0 teacher0 FD0",cost["input"],cost["target"]);Ok(())
    }
    #[test]
    fn retained_qa_native_process()->Result<()> {
        const TEST:&str="training::fresh::identifiable::binding::citation::tests::retained_qa_native_process";
        if let Ok(path)=std::env::var("R3_QA_CHILD") {let root=Path::new(&path);return match std::env::var("R3_QA_ACTION").as_deref(){
            Ok("old-v")=>parent_observe(root,false),Ok("old-vc")=>parent_observe(root,true),Ok("mean-parent")=>mean_parent(root),
            Ok("prior-v")=>continuation_parent(root,false),Ok("prior-vc")=>continuation_parent(root,true),
            Ok("qa-value")=>qa_parent(root,"value"),Ok("qa-citation")=>qa_parent(root,"citation"),Ok("qa-balanced")=>qa_parent(root,"balanced"),Ok("qa-transfer")=>qa_parent(root,"transfer"),
            Ok("review-old")=>qa_review(root,"old_qa"),Ok("review-balanced")=>qa_review(root,"balanced"),
            Ok("review-v")=>mean_review(root,false),Ok("review-vc")=>mean_review(root,true),Ok("one")=>run(root,false),_=>run(root,true)};}
        if !cfg!(feature="test-support"){return Err(bad("explicit retained QA TINY fixture"));}
        let tmp=tempfile::tempdir()?;let base=std::env::var_os("R3_QA_TEST_ROOT").map(PathBuf::from).unwrap_or_else(||tmp.path().join("evidence"));std::fs::create_dir(&base)?;let base=base.canonicalize()?;
        let child=|root:&Path,action:&str,fault:Option<&str>,label:&str|->Result<()> {let mut cmd=std::process::Command::new(std::env::current_exe()?);
            cmd.args(["--exact",TEST,"--nocapture"]).env("R3_QA_CHILD",root).env("R3_QA_ACTION",action).env("VECLIB_MAXIMUM_THREADS","1").env("OMP_NUM_THREADS","1");if let Some(f)=fault{cmd.env("R3_FRESH_CALL_STOP",f);}
            let out=cmd.output()?;std::fs::write(base.join(format!("{label}.stdout")),&out.stdout)?;std::fs::write(base.join(format!("{label}.stderr")),&out.stderr)?;
            assert!(out.status.success(),"{label}: {} {}",String::from_utf8_lossy(&out.stdout),String::from_utf8_lossy(&out.stderr));assert!(String::from_utf8_lossy(&out.stdout).contains("1 passed"));Ok(())};
        // Real bounded TINY ancestry once. No historical SMALL experiments or confirmations run.
        let reused=std::env::var_os("R3_QA_PREVIOUS_FIXTURE").map(PathBuf::from);
        let mut roots=vec![];let mut observations=0;
        let report=base.join("fixture-parent-report.txt");std::fs::write(&report,b"Explicit TINY ancestry, not independent SMALL quality acceptance")?;
        let prior=if let Some(previous)=&reused {
            let prior=previous.join("precision");let p=historical_plan(&selected_root(&prior))?;
            if !p.tiny || !precision(&p){return Err(bad("reuse requires explicit TINY precision fixture"));}prior
        }else{
        let parents=super::super::tests::orbit_fixture(&base)?;
        for arm in ["FIXED","BOTH"]{let r=parents.join(arm);child(&r,"run",None,arm)?;roots.push(r);}
        let parent=parents.join("BOTH");let(public,private,used)=fixture_reservation(&parent,&base)?;
        let old=base.join("reference");prepare_inner(&parent,&old,&public,&used,true)?;seal(&old,&private)?;super::super::tests::fixture_review(&old)?;
        for a in ["old-v","old-vc"]{child(&old,a,None,a)?;}observations+=8;child(&old.join(ARM),"run",None,"reference-run")?;roots.push(old.join(ARM));
        let mean=base.join("mean");mean_prepare(&old,&mean,true)?;super::super::tests::fixture_review(&mean)?;child(&mean,"mean-parent",None,"mean-parent")?;observations+=4;
        for arm in MEAN_ARMS {let r=mean.join(arm);child(&r,"run",None,arm)?;roots.push(r);}
        let mut prior=mean;
        for stage in ["continuation","fidelity","precision"] {let next=base.join(stage);match stage{"continuation"=>continuation_prepare(&prior,&next,true)?,"fidelity"=>fidelity_prepare(&prior,&next,&report,true)?,_=>precision_prepare(&prior,&next,&report,true)?};
            super::super::tests::fixture_review(&next)?;for a in ["prior-v","prior-vc"]{child(&next,a,None,&format!("{stage}-{a}"))?;}observations+=8;
            child(&selected_root(&next),"run",None,&format!("{stage}-run"))?;roots.push(selected_root(&next));prior=next;}
        prior};
        let(balanced,old_qa)=if let Some(previous)=&reused{(previous.join("balanced"),previous.join("old-qa"))}else{qa_fixture_sources(&base)?};let mut ends=vec![];let mut outputs=vec![];
        let modes=if reused.is_some(){vec!["eval-only"]}else{vec!["continuous","split","eval-only"]};
        for mode in &modes {let mode=*mode;let study=base.join(mode);qa_prepare(&prior,&balanced,&old_qa,&study,&report,true)?;super::super::tests::fixture_review(&study)?;
            for a in ["qa-value","qa-citation","qa-balanced","qa-transfer"]{child(&study,a,None,&format!("{mode}-{a}"))?;}observations+=40;
            let root=selected_root(&study);let p=plan_read(&root)?;let step=p.config.max_steps;
            let mut wrong=p.clone();wrong.config.seq_len=256;assert!(qa_verify_plan(&root,&wrong).is_err());wrong=p.clone();wrong.config.lr=3e-4;assert!(qa_verify_plan(&root,&wrong).is_err());
            let fault=format!("eval-{step:04}-qa-balanced-primary/generation/1/fresh_panel_row_durable");
            child(&root,if mode=="split"{"one"}else{"run"},(mode=="eval-only").then_some(fault.as_str()),&format!("{mode}-0"))?;
            let prefix=if mode=="eval-only" {let h=history(&root,&p)?;assert_eq!(h.last().unwrap().phase.as_deref(),Some("EvaluationPending"));assert_eq!(h.last().unwrap().step,step);Some(binary::read_value_records(&root.join(format!("eval-{step:04}-qa-balanced-primary.r3rows")))?)}else{None};
            if mode!="continuous"{child(&root,"run",None,&format!("{mode}-1"))?;}
            if let Some(prefix)=prefix{let after=binary::read_value_records(&root.join(format!("eval-{step:04}-qa-balanced-primary.r3rows")))?;assert_eq!(&after[..prefix.len()],prefix);assert_eq!(read::<binary::Value>(&root.join("segment-0001/train-control.r3b"))?["optimizer_calls"],0);}
            let(end,d)=close(&root,&p)?;assert_eq!(d["eligible"],false);assert_eq!(d["extend"],false);assert_eq!(end.stop,"STUDY_COMPLETE_QUALITY_FAIL");assert!(!end.resume);
            let l=checkpoint::load(&root.join(&end.checkpoint),Device::Cpu,true)?;let st=l.manifest.training.as_ref().unwrap();ends.push((l.model.weights_content_id()?,optimizer_hash(&l.optimizer)?,st.step,st.sampler_state,st.consumed_tokens,st.target_tokens));
            let mut output=vec![];for(name,_,_)in panels(&root,&p,step)?{for r in binary::read_value_records(&root.join(format!("eval-{step:04}-{name}.r3rows")))?.into_iter().skip(1){output.push(binary::record!({"tokens":r["raw_tokens"],"actual":r["actual"],"finish":r["finish_reason"],"error":r["error"]}));}}outputs.push(output);
            assert!(!gate(&root,&p)?);assert!(run(&root,false).is_err());assert!(confirm(&study).is_err());roots.push(root);
        }
        if let Some(previous)=&reused {
            let root=selected_root(&previous.join("continuous"));let p=historical_plan(&root)?;let end=history(&root,&p)?.last().cloned().ok_or_else(||bad("fixture reference endpoint"))?;
            let l=checkpoint::load(&root.join(&end.checkpoint),Device::Cpu,true)?;let s=l.manifest.training.as_ref().unwrap();
            ends.push((l.model.weights_content_id()?,optimizer_hash(&l.optimizer)?,s.step,s.sampler_state,s.consumed_tokens,s.target_tokens));
            let mut output=vec![];for(name,_,_)in panels(&root,&p,end.step)?{for r in binary::read_value_records(&root.join(format!("eval-{:04}-{name}.r3rows",end.step)))?.into_iter().skip(1){output.push(binary::record!({"tokens":r["raw_tokens"],"actual":r["actual"],"finish":r["finish_reason"],"error":r["error"]}));}}outputs.push(output);
        }
        assert!(ends.len()>=2&&ends.windows(2).all(|v|v[0]==v[1]));assert!(outputs.len()>=2&&outputs.windows(2).all(|v|v[0]==v[1]));
        for a in ["review-v","review-vc","review-old","review-balanced"]{child(&selected_root(&base.join(modes[0])),a,None,a)?;}observations+=40;
        let mut actual=[0usize,observations,0];for root in roots{let p=historical_plan(&root)?;for end in history(&root,&p)?{actual[0]+=read::<binary::Value>(&root.join(&end.checkpoint).parent().unwrap().join("train-control.r3b"))?["optimizer_calls"].as_u64().unwrap()as usize;actual[1]+=end.generations;actual[2]+=end.teachers;}}
        assert!(actual[0]<=96&&actual[1]<=768&&actual[2]<=768);
        println!("RETAINED_QA_PROCESS optimizer={} generation={} teacher={} continuous2/newprocess1+1/evaluation_only2+0 SAME native/Adam/clock/raw; EOS TINY tensor fixture, no quality claim; evidence={}",actual[0],actual[1],actual[2],base.display());Ok(())
    }
    #[test]
    fn retained_qa_full_raw_gate()->Result<()> {
        let tmp=tempfile::tempdir()?;let study=super::super::tests::orbit_fixture(tmp.path())?;let source=study.join("BOTH");
        let p=plan_read(&source)?;let tok=ByteBpe::load(&source.join("tokenizer.r3b"))?;
        let(c,tm,dm)=source_pool(&source,&p,&tok)?;let(public,_,used)=fixture_reservation(&source,tmp.path())?;
        let(r,(mut tm,dm,_),_)=make_pool(&c,&tm,&dm,&tok,&read_ids(&public,128)?,&read_ids(&used,128)?)?;
        let(balanced,old)=qa_fixture_sources(tmp.path())?;let(q,_,qm,_,_,_,_,_,_)=qa_sources(&balanced,&old)?;let(v,vm)=qa_variant(&q.train,&qm)?;
        let mut train=r.train;train.extend(q.train);train.extend(v);tm.extend(qm);tm.extend(vm);
        let mut manifest=r.manifest;manifest.train=data::native::split("train",&train);
        let corpus=data::native::from_episodes(manifest,train,r.validation)?;let rows=qa_tape(&tm)?;
        let mut sources=BTreeMap::new();for(label,path)in[("balanced",&balanced),("old_qa",&old)]{for name in ["corpus.r3cor","transfer.r3cor","metadata.r3b"]{sources.insert(format!("{label}/{name}"),file_hash(&path.join(name))?);}}
        for(label,step,bad_qa,bad_fit,malformed)in[("positive",13312,false,false,None),("failed-fit",15360,false,true,None),("failed-QA-bucket",15360,true,false,None),
            ("malformed-outside",15360,false,false,Some(" [event:9223372036854775807] [event:]")),("malformed-only",15360,false,false,Some(" [event:]"))]{
            let root=tmp.path().join(label);std::fs::create_dir(&root)?;
            data::native::write(&root.join("corpus.r3cor"),&corpus,true)?;write(&root.join("metadata.r3b"),&(tm.clone(),dm.clone(),dm.clone()))?;
            copy_native(&source.join("tokenizer.r3b"),&root.join("tokenizer.r3b"))?;
            write(&root.join("selection.r3b"),&binary::record!({"fixture":"explicit complete writer/reader QA gate; model calls0","balanced":balanced,"old_qa":old,"sources":sources}))?;
            let mut p=p.clone();p.tiny=false;p.corpus=file_hash(&root.join("corpus.r3cor"))?;p.metadata=file_hash(&root.join("metadata.r3b"))?;
            let o=p.identifiable.as_mut().unwrap();o.study=root.clone();o.dataset=QA_DATA.into();o.arm=MEAN_ARMS[1].into();o.rows=[vec![[0;8];11264],rows.clone()].concat();p.train_order=digest(&o.rows)?;
            p.fork=Some(Fork{study:root.clone(),study_hash:String::new(),arm:MEAN_ARMS[1].into(),parent_policy:String::new(),parent_state:String::new(),parent_adam:String::new(),origin_step:11264,origin_input:0,origin_target:0,original_corpus:p.corpus.clone(),tokenizer_training_hash:tok.train_hash.clone(),variants:None,variant_metadata:None,alternate_first:vec![],selector:None,selector_metadata:None,flip_first:vec![],constant_lr:3e-5,target_limit:2400000});
            p.config.max_steps=15360;p.config.budget_start_step=11264;p.config.max_tokens=18_000_000;p.config.warmup=0;p.config.seq_len=512;p.config.lr=3e-5;p.evaluation=evaluation(&p);p.evaluation.train_steps=vec![step];
            let mut l=checkpoint::load(&source.join("initial.r3m"),Device::Cpu,false)?;
            let mut st=TrainingState{resume_binding:None,contrast16:false,parent_checkpoint_hash:None,config:p.config.clone(),step,sampler_state:step as u64,consumed_tokens:0,target_tokens:0,corpus_hash:corpus.manifest.train.sha256.clone(),validation_hash:corpus.manifest.validation.sha256.clone(),previous_corpora:vec![tok.train_hash.clone()],initial_weight_hash:l.manifest.initial_weight_hash.clone(),train_loss:None,validation_loss:None};
            st.resume_binding=Some(p.binding(&st,&tok)?);l.manifest.training=Some(st);let adam=Adam::new(&l.model.vars)?;let native=root.join("explicit-record-fixture.r3m");checkpoint::save(&native,&l.model,&tok,l.manifest,&adam.moments)?;
            let base=qa_base_panels(&root,&p,step)?;assert_eq!(base.iter().map(|v|v.1.len()).collect::<Vec<_>>(),vec![512,512,512,512,128,512,128,128]);
            for panel in &base[..7]{precision_panel_fixture_with_suffix(&root,&p,step,&native,panel,if bad_qa&&panel.0=="qa-balanced-primary"{8}else{0},malformed.filter(|_|panel.0=="qa-balanced-primary"))?;}
            assert!(qa_decision(&root,&p,step).is_err()); // Mandatory QA train panel cannot silently disappear.
            precision_panel_fixture(&root,&p,step,&native,&base[7],0)?;
            let all=panels(&root,&p,step)?;assert_eq!(all.len(),if bad_qa||malformed.is_some(){8}else{12});
            if !bad_qa&&malformed.is_none() {assert!(qa_decision(&root,&p,step).is_err());for panel in &all[8..]{precision_panel_fixture(&root,&p,step,&native,panel,if bad_fit&&panel.0=="old512"{8}else{0})?;}}
            let d=qa_decision(&root,&p,step)?;assert_eq!(d["eligible"],!bad_qa&&!bad_fit&&malformed.is_none());assert_eq!(d["extend"],false);
            assert_eq!(d["action"],if bad_qa||bad_fit||malformed.is_some(){"STUDY_COMPLETE_QUALITY_FAIL".into()}else{format!("CANDIDATE_FIXED_AT_{step}")});
            let q=&d["panels"]["qa-balanced-primary"];assert_eq!(q["ALL4"],"NOT_DEFINED");assert_eq!(q["H_fixed_planned_emitted_strict"],binary::record!([64,64,64]));
            if malformed.is_some(){assert_eq!(q["full"],511);assert_eq!(q["EOS"],512);assert_eq!(q["parse_failure_rows"],1);assert_eq!(q["buckets"][0][6],1);
                assert_eq!(q["outside_id"],usize::from(label=="malformed-outside"));assert_eq!(q["buckets"][0][4],q["outside_id"]);assert_eq!(q["buckets"][0][3],63);
                println!("QA_MALFORMED_GATE {label} full={} EOS={} outside={} parse={} eligible={}",q["full"],q["EOS"],q["outside_id"],q["parse_failure_rows"],d["eligible"]);}
            let g=q["G_resolution_planned_unresolved_resolved_both"].as_object().unwrap();assert_eq!(g.len(),3);assert_eq!(g.values().map(|v|v[0].as_u64().unwrap()).sum::<u64>(),32);
            let old=&d["panels"]["qa-old_qa-primary"];for b in 0..8{assert_eq!(old["ALL4"][b],binary::record!([16,16]));assert_eq!(old["old_view_pairs"][b][1],16);assert_eq!(old["old_view_pairs"][b][6],16);}
            if !bad_qa&&malformed.is_none() {let scores:BTreeMap<String,binary::Value>=binary::from_value(d["panels"].clone())?;assert!(qa_dev_pass(&scores)?);
                for(field,value)in[("errors",1),("EOS",511),("outside_id",1),("parse_failure_rows",1),("full",486)]{let mut wrong=scores.clone();wrong.get_mut("qa-balanced-primary").unwrap()[field]=binary::record!(value);assert!(!qa_dev_pass(&wrong)?);}
                let mut wrong=scores;wrong.get_mut("qa-balanced-primary").unwrap()["QUERY_BOTH_planned_correct"][2][1]=binary::record!(28);assert!(!qa_dev_pass(&wrong)?);
            }
        }
        println!("RETAINED_QA_FULL typed writer->reader->decision all2944+conditional4608; missing/failed fit, bucket/pair/EOS/outside gates, G/H and old4view checked; synthetic records optimizer0 generation0 teacher0");Ok(())
    }
    fn random_updates(input:&Path,output:&Path,root:&Path,steps:usize)->Result<()> {
        let p=historical_plan(root)?;let mut l=checkpoint::load(input,Device::Cpu,true)?;
        let mut s=l.manifest.training.clone().ok_or_else(||bad("random native state"))?;
        if s.resume_binding!=Some(p.binding(&s,&l.tokenizer)?) {return Err(bad("random native exact objective policy"));}
        let c=verified_corpus(&root.join("corpus.r3cor"),&p.corpus)?;
        let ss=samples(&c.train,&l.tokenizer,p.config.seq_len)?;
        let mut adam=Adam{moments:l.optimizer};
        for _ in 0..steps {
            let ids=p.training_draw(s.step);let b=batch(&ss,&ids,&Device::Cpu)?;
            let logits=l.model.forward(&b.input,Some(&b.valid))?;
            let (ce,loss,n,den)=crate::training::response_objective(&logits,&b,1.,true)?;
            assert_eq!(den,ids.len());let grads=loss.backward()?;
            let gs=l.model.vars.iter().map(|(name,v)|Ok((name.clone(),grads.get(v).ok_or_else(||bad("random native gradient"))?.detach()))).collect::<Result<BTreeMap<_,_>>>()?;
            adam.step_constant(&l.model.vars,&gs,&p.config,s.step+1,p.learning_rate(s.step+1))?;
            s.step+=1;s.sampler_state=s.step as u64;s.consumed_tokens+=b.tokens as u64;s.target_tokens+=n as u64;
            s.train_loss=Some(f64::from(ce.to_scalar::<f32>()?));s.validation_loss=None;
        }
        l.model.refresh_identity()?;l.manifest.training=Some(s);
        checkpoint::save(output,&l.model,&l.tokenizer,l.manifest,&adam.moments)?;
        println!("RANDOM_ANSWER_NATIVE optimizer={steps} forward={steps} backward={steps} generation0 teacher0");Ok(())
    }
    #[test]
    fn citation_continuation_guard_and_record_boundaries()->Result<()> {
        assert_eq!(retention_decision(60,12,0,1),(0,None));
        assert_eq!(retention_decision(59,11,0,0),(1,None));
        assert_eq!(retention_decision(59,11,0,1),(2,Some("PERSISTENT_RETENTION_REGRESSION")));
        assert_eq!(retention_decision(48,12,0,0).1,Some("SEVERE_RETENTION_REGRESSION"));
        assert_eq!(retention_decision(64,16,4,0).1,Some("SEVERE_RETENTION_REGRESSION"));
        let tmp=tempfile::tempdir()?;let fixture=super::super::tests::orbit_fixture(tmp.path())?;
        let p=plan_read(&fixture.join("BOTH"))?;let tok=ByteBpe::load(&fixture.join("BOTH/tokenizer.r3b"))?;
        let c=verified_corpus(&fixture.join("BOTH/corpus.r3cor"),&p.corpus)?;
        for count in [0,1,511,512] {
            let root=tmp.path().join(format!("records-{count}"));std::fs::create_dir(&root)?;
            let es=&c.validation[..count];let checkpoint="synthetic record boundary; no native model calls";
            let binding=binary::record!({"fixture":"explicit writer/reader panel; no model calls","cases":digest(&&c.validation[..512])?,
                "identity":{"policy":digest(&p)?},"checkpoint":checkpoint,"source":p.source,"binary":p.binary});
            let mut f=std::fs::OpenOptions::new().create_new(true).write(true).open(root.join("confirmation.r3rows"))?;append_row(&mut f,&binding)?;
            let control=recovery::RunControl::new(std::sync::Arc::new(AtomicBool::new(false)),std::time::Duration::from_secs(30),0)?;
            for (i,e) in es.iter().enumerate() {
                let attempt=prepare_call(&root,"confirmation","generation",&binding,e,i)?;
                let mut row=gold(e,&tok)?;row["attempt"]=binary::record!(attempt.file_name().unwrap().to_string_lossy());
                append_row(&mut f,&row)?;resolve_call(&attempt,Some(&row),&control)?;
            }
            drop(f);assert_eq!(confirmation_prefix(&root,&binding,es,&tok)?.len(),count);
            publish_confirmed(&root.join("confirmation-finished.r3b"),&binary::record!({"fixture":"synthetic logical rows; actual model calls0",
                "policy":digest(&p)?,"checkpoint":checkpoint,"binding":binding,"completed":count,"matched":0,"error":null,
                "raw":file_hash(&root.join("confirmation.r3rows"))?,"control":{"generation_calls":count,"teacher_calls":0,"terminal_reason":"COMPLETED"}}))?;
            let complete=observed(&root,"confirmation",&p,checkpoint,&c.validation[..512],false);
            assert_eq!(complete.is_ok(),count==512);
            let altered=binary::record!({"fixture":"different model/candidate","cases":digest(&es)?});
            assert!(confirmation_prefix(&root,&altered,es,&tok).is_err());
            let bytes=std::fs::read(root.join("confirmation.r3rows"))?;
            let truncated=root.join("truncated.r3rows");std::fs::write(&truncated,&bytes[..bytes.len()-1])?;
            assert!(binary::read_value_records(&truncated).is_err());
            if count>0 {
                let missing=&es[..count-1];assert!(confirmation_prefix(&root,&binding,missing,&tok).is_err());
                let mut changed=es.to_vec();changed[0].answer.push('0');assert!(confirmation_prefix(&root,&binding,&changed,&tok).is_err());
                let resolved=root.join("confirmation-generation-0000-000-resolved.r3b");let original=std::fs::read(&resolved)?;
                let mut mixed:binary::Value=binary::from_slice(&original)?;
                mixed["control"]["observed_conditions"]=binary::record!(["TIME_BUDGET","CANCELLED"]);
                std::fs::write(&resolved,binary::to_vec(&mixed)?)?;
                assert!(confirmation_prefix(&root,&binding,es,&tok).is_err());std::fs::write(&resolved,original)?;
                let extra=root.join("confirmation-generation-0000-002-prepared.r3b");write(&extra,&binary::record!({"fixture":"attempt gap"}))?;
                assert!(confirmation_prefix(&root,&binding,es,&tok).is_err());
            }
        }
        println!("CITATION_CONTINUATION_RECORDS counts0/1/511/512 optimizer0 generation0 teacher0 FD0");Ok(())
    }
    #[test]
    fn citation_continuation_process()->Result<()> {
        continuation_process(CONT_CONTRACT)
    }
    #[test]
    fn citation_fidelity_process()->Result<()> {
        continuation_process(FID_CONTRACT)
    }
    #[test]
    fn citation_precision_process()->Result<()> {
        continuation_process(PREC_CONTRACT)
    }
    fn continuation_process(profile:&str)->Result<()> {
        let prec=profile==PREC_CONTRACT;let fid=profile!=CONT_CONTRACT;
        let test=if prec {"training::fresh::identifiable::binding::citation::tests::citation_precision_process"}
            else if fid {"training::fresh::identifiable::binding::citation::tests::citation_fidelity_process"}
            else{"training::fresh::identifiable::binding::citation::tests::citation_continuation_process"};
        if let Ok(path)=std::env::var("R3_CONT_CHILD") {
            let root=Path::new(&path);let action=std::env::var("R3_CONT_ACTION").unwrap();
            let result=match action.as_str() {
                "generic-reject"=>generic_resume_rejected(root),
                "confirmation-io"|"confirmation-io-retry"=>{
                    let p=historical_plan(root)?;let end=history(root,&p)?.last().unwrap().clone();
                    let native=root.join(&end.checkpoint);let l=checkpoint::load(&native,Device::Cpu,false)?;
                    let (_,es,_)=base_panels(root,&p,end.step)?.remove(0);
                    let out=root.join("confirmation-io");if !out.exists(){std::fs::create_dir(&out)?;}
                    let before=confirmation_call_files(&out)?;
                    let mut control=recovery::RunControl::new(std::sync::Arc::new(AtomicBool::new(false)),std::time::Duration::from_secs(30),16*1024*1024)?;
                    control.set_call_limits(1,0);
                    let result=confirmation_collect(&out,&native,&es[..1],&l.tokenizer,&binary::record!({"fixture":"resolution failure accounting","policy":digest(&p)?}),control);
                    let error=result.unwrap_err().to_string();
                    if action=="confirmation-io" {assert!(error.contains("injected resolution sync failure"));}
                    else {assert!(error.contains("non-time failure"));assert_eq!(confirmation_call_files(&out)?,before);}
                    let rows=binary::read_value_records(&out.join("confirmation.r3rows"))?;
                    let final_record:binary::Value=read_confirmed(&out.join("confirmation-segment-000-finished.r3b"))?;
                    assert_eq!(rows.len(),2);assert_eq!(final_record["returned_after"],1);
                    let tokens=rows[1]["raw_tokens"].as_array().unwrap().len();assert!(tokens>0);
                    assert_eq!(final_record["generated_tokens"],tokens);assert_eq!(final_record["control"]["generation_calls"],1);
                    assert!(!final_record["error"].is_null());assert!(confirmation_usage(&out).is_err());
                    println!("CONFIRMATION_IO known_tokens={tokens} pending_sticky=true new_generation={} optimizer0 teacher0",usize::from(action=="confirmation-io"));Ok(())
                },
                "random-step"=>random_updates(&root.join("../random-one.r3m"),&root.join("../random-split.r3m"),root,1),
                "random-eval"|"random-eval-split"|"random-reentry"=>{
                    let p=historical_plan(root)?;let (name,es,ms)=base_panels(root,&p,p.config.max_steps)?.remove(0);
                    let out=root.join(if action=="random-eval" {"random-eval"}else{"random-eval-split"});
                    if !out.exists(){std::fs::create_dir(&out)?;}
                    let mut control=recovery::RunControl::new(std::sync::Arc::new(AtomicBool::new(false)),std::time::Duration::from_secs(30),16*1024*1024)?;
                    control.set_call_limits(4,4);
                    let native=root.join("../random-split.r3m");let before=file_hash(&native)?;
                    let prefix=format!("eval-{:04}-{name}",p.config.max_steps);
                    if action=="random-reentry" {
                        let raw=out.join(format!("{prefix}.r3rows"));let hash=file_hash(&raw)?;
                        let rows=binary::read_value_records(&raw)?;
                        assert!(prepare_call(&out,&prefix,"generation",&rows[0]["binding"],&es[0],0).is_err());
                        assert_eq!(file_hash(&raw)?,hash);assert_eq!(control.receipt()["generation_calls"],0);
                        println!("RANDOM_RETURNED_REENTRY blocked=true optimizer0 generation0 teacher0");Ok(())
                    }else{
                        let result=evaluate_panel(&p,&out,&native,p.config.max_steps,&name,&es,&ms,&mut control);
                        assert_eq!(file_hash(&native)?,before);
                        let time=control.receipt()["observed_conditions"]==binary::record!(["TIME_BUDGET"]);
                        publish_confirmed(&out.join(if time {"no-call-control.r3b"}else{"returned-control.r3b"}),&control.receipt())?;
                        if time {assert_eq!(control.receipt()["generation_calls"],0);result.map(|_|())}else{
                            assert!(result.unwrap_err().to_string().contains("INTEGRITY_FAIL"));
                            let rows=binary::read_value_records(&out.join(format!("{prefix}.r3rows")))?;
                            assert_eq!(rows.len(),2);assert_eq!(rows[1]["error_class"],"control_token");
                            assert_eq!(control.receipt()["generation_calls"],1);assert_eq!(control.receipt()["teacher_calls"],0);
                            println!("RANDOM_EVALUATION expected_control_token_failure=true completed=false optimizer0 generation1 teacher0 native_unchanged=true");Ok(())
                        }
                    }
                },
                "old-v"=>parent_observe(root,false),"old-vc"=>parent_observe(root,true),"mean-parent"=>mean_parent(root),
                "parent-v"=>continuation_parent(root,false),"parent-vc"=>continuation_parent(root,true),
                "review-v"=>mean_review(root,false),"review-vc"=>mean_review(root,true),"confirm"=>confirm(root),
                "report"=>{let p=historical_plan(&selected_root(root))?;let (e,d)=close(&selected_root(root),&p)?;println!("PURE_CONFIRM {}",verified_confirmation(root,&p,&e,&d)?);Ok(())},
                _=>run(root,action=="continuous"),
            };
            if std::env::var("R3_CONT_EXPECT_TIME").as_deref()==Ok("1") {assert!(result.unwrap_err().to_string().contains("TIME_BUDGET"));Ok(())}else{result}
        }else{
            if !cfg!(feature="test-support") {return Err(bad("explicit continuation TINY test profile"));}
            let base=std::env::var_os("R3_CONT_TEST_ROOT").map(PathBuf::from).unwrap_or_else(||std::env::temp_dir().join(format!("replica-citation-continuation-{}-{}",std::process::id(),std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos())));
            std::fs::create_dir(&base)?;let base=base.canonicalize()?;
            let child=|root:&Path,action:&str,fault:Option<&str>,time:bool,label:&str|->Result<()> {
                let mut cmd=std::process::Command::new(std::env::current_exe()?);
                cmd.args(["--exact",test,"--nocapture"]).env("R3_CONT_CHILD",root).env("R3_CONT_ACTION",action)
                    .env("VECLIB_MAXIMUM_THREADS","1").env("OMP_NUM_THREADS","1");
                if let Some(f)=fault {if f=="candidate" {cmd.env("R3_CONFIRM_STOP",f);}else if f=="resolution" {cmd.env("R3_FRESH_RESOLUTION_FAIL","1");}else{cmd.env("R3_FRESH_CALL_STOP",f);}}
                if time {cmd.env("R3_CONT_EXPECT_TIME","1");}
                let out=cmd.output()?;std::fs::write(base.join(format!("{label}.stdout")),&out.stdout)?;std::fs::write(base.join(format!("{label}.stderr")),&out.stderr)?;
                assert!(out.status.success(),"{label}: {} {}",String::from_utf8_lossy(&out.stdout),String::from_utf8_lossy(&out.stderr));
                assert!(String::from_utf8_lossy(&out.stdout).contains("1 passed"));Ok(())
            };
            let parents=super::super::tests::orbit_fixture(&base)?;
            for arm in ["FIXED","BOTH"] {child(&parents.join(arm),"continuous",None,false,arm)?;}
            let parent=parents.join("BOTH");let (public,private,used)=fixture_reservation(&parent,&base)?;
            let old=base.join("reference");prepare_inner(&parent,&old,&public,&used,true)?;seal(&old,&private)?;
            super::super::tests::fixture_review(&old)?;
            child(&old,"old-v",None,false,"old-v")?;child(&old,"old-vc",None,false,"old-vc")?;
            child(&old.join(ARM),"continuous",None,false,"old-run")?;
            let mean=base.join("mean");mean_prepare(&old,&mean,true)?;super::super::tests::fixture_review(&mean)?;
            child(&mean,"mean-parent",None,false,"mean-parent")?;
            for arm in MEAN_ARMS {child(&mean.join(arm),"continuous",None,false,arm)?;}
            let mut parent=base.join("continuation-parent");let parent_report=base.join("fixture-parent-report.txt");
            if fid {
                std::fs::write(&parent_report,b"TINY parent provenance fixture, not independent model acceptance")?;
                assert!(fidelity_prepare(&mean,&base.join("wrong-parent"),&parent_report,true).is_err());
                assert!(!base.join("wrong-parent").exists());
                continuation_prepare(&mean,&parent,true)?;super::super::tests::fixture_review(&parent)?;
                child(&parent,"parent-v",None,false,"prior-parent-v")?;
                child(&parent,"parent-vc",None,false,"prior-parent-vc")?;
                child(&selected_root(&parent),"continuous",None,false,"prior-run")?;
            }
            if prec {
                assert!(precision_prepare(&parent,&base.join("wrong-precision-parent"),&parent_report,true).is_err());
                assert!(!base.join("wrong-precision-parent").exists());
                let next=base.join("fidelity-parent");fidelity_prepare(&parent,&next,&parent_report,true)?;
                super::super::tests::fixture_review(&next)?;
                child(&next,"parent-v",None,false,"fidelity-parent-v")?;
                child(&next,"parent-vc",None,false,"fidelity-parent-vc")?;
                child(&selected_root(&next),"continuous",None,false,"fidelity-parent-run")?;
                parent=next;
            }
            let mut endpoints=vec![];let mut model_outputs=vec![];let mut totals=[0usize;3];
            for mode in if fid {vec!["split","eval-only","teacher-final","continuous"]}else{vec!["split","eval-only","continuous"]} {
                let study=base.join(mode);
                if prec {precision_prepare(&parent,&study,&parent_report,true)?;}
                else if fid {fidelity_prepare(&parent,&study,&parent_report,true)?;}else{continuation_prepare(&mean,&study,true)?;}
                super::super::tests::fixture_review(&study)?;
                child(&study,"parent-v",None,false,&format!("{mode}-parent-v"))?;child(&study,"parent-vc",None,false,&format!("{mode}-parent-vc"))?;
                let root=selected_root(&study);
                let p=historical_plan(&root)?;
                let fault=if mode=="teacher-final" {format!("eval-{:04}-renamed4/teacher/3/fresh_teacher_durable",p.config.max_steps)}
                    else{format!("eval-{:04}-citation4/generation/1/fresh_panel_row_durable",p.config.max_steps)};
                let pending=mode=="eval-only" || mode=="teacher-final";
                child(&root,if mode=="split" {"one"}else{"continuous"},pending.then_some(fault.as_str()),false,&format!("{mode}-run0"))?;
                if mode!="continuous" {
                    let prior=history(&root,&p)?.last().unwrap().clone();
                    if pending {assert_eq!(prior.step,p.config.max_steps);assert_eq!(prior.phase.as_deref(),Some("EvaluationPending"));}
                    if mode=="teacher-final" {assert_eq!(work(&p)?.2,p.evaluation.teacher_limit);}
                    child(&root,"continuous",None,false,&format!("{mode}-run1"))?;
                    if pending {assert_eq!(read::<binary::Value>(&root.join("segment-0001/train-control.r3b"))?["optimizer_calls"],0);}
                    if mode=="teacher-final" {let done=history(&root,&p)?.last().unwrap().clone();assert_eq!((done.generations,done.teachers),(0,0));assert_eq!(done.phase.as_deref(),Some("Finished"));}
                }
                let (e,d)=close(&root,&p)?;let l=checkpoint::load(&root.join(&e.checkpoint),Device::Cpu,true)?;
                if prec {
                    assert_eq!(p.learning_rate(e.step).to_bits(),3e-5f64.to_bits());
                    assert_eq!(l.manifest.training.as_ref().unwrap().config.lr.to_bits(),3e-5f64.to_bits());
                    assert_eq!(l.manifest.training.as_ref().unwrap().resume_binding,Some(p.binding(l.manifest.training.as_ref().unwrap(),&l.tokenizer)?));
                    let mut other=p.clone();other.config.lr=3e-4;
                    assert_ne!(l.manifest.training.as_ref().unwrap().resume_binding,Some(other.binding(l.manifest.training.as_ref().unwrap(),&l.tokenizer)?));
                    if mode=="continuous" {child(&root.join(&e.checkpoint),"generic-reject",None,false,"precision-generic-reject")?;}
                }
                endpoints.push((l.model.weights_content_id()?,optimizer_hash(&l.optimizer)?));
                for end in history(&root,&p)? {totals[0]+=read::<binary::Value>(&root.join(&end.checkpoint).parent().unwrap().join("train-control.r3b"))?["optimizer_calls"].as_u64().unwrap() as usize;totals[1]+=end.generations;totals[2]+=end.teachers;}
                if pending {continue;}
                if fid {
                    let previous=historical_plan(&selected_root(&parent))?;
                    assert_eq!(&own(&p).rows[..p.origin_step()],own(&previous).rows.as_slice());
                    assert_eq!(&own(&p).rows[p.origin_step()..],&suffix()[..2]);
                    assert_eq!(seal_owner(&study)?,seal_owner(&parent)?);
                    assert_eq!(seal_receipt(&study)?,seal_receipt(&parent)?);
                    let link=study.join("confirmation-link.r3b");let original:binary::Value=read_confirmed(&link)?;
                    // Malformed lineage is confined to this disposable fixture.
                    let wrong=study.join("wrong-link.r3b");let mut changed=original.clone();changed["source_study"]=binary::record!(mean);
                    std::fs::rename(&link,&wrong)?;write(&link,&changed)?;
                    assert!(seal_receipt(&study).is_err());std::fs::remove_file(&link)?;std::fs::rename(&wrong,&link)?;
                    assert_eq!(seal_receipt(&study)?,seal_receipt(&parent)?);
                    assert!(confirmation_admitted(&study,&p,&e,&d).is_err()); // No fixture admission is a hard refusal.
                    assert!(run(&root,true).is_err());
                    if mode=="split" {continue;}
                    child(&root,"review-v",None,false,"fidelity-review-v")?;
                    child(&root,"review-vc",None,false,"fidelity-review-vc")?;totals[1]+=8;
                }
                if !fid {
                child(&root,"review-v",None,false,&format!("{mode}-review-v"))?;child(&root,"review-vc",None,false,&format!("{mode}-review-vc"))?;
                let comp=comparison(&study,&p,&e,&d)?;let report=study.join("fixture-b.txt");std::fs::write(&report,b"TINY fixture B only; no SMALL acceptance")?;
                publish_confirmed(&study.join("review-b.r3b"),&binary::record!({"verdict":"PASS","preparation":file_hash(&study.join("preparation.r3b"))?,"endpoints":comp["endpoints"],"report_path":report,"report_hash":file_hash(&report)?}))?;
                publish_confirmed(&study.join("fixture-confirmation-admission.r3b"),&binary::record!({"scope":"TINY confirmation protocol only; no quality acceptance","policy":digest(&p)?,"checkpoint":e.checkpoint_hash,"count":8}))?;
                // Each fixture uses a distinct sealed reservation owner. A second
                // study opening the actual same seal must remain blocked.
                if mode=="split" {continue;}
                for (label,fault) in [("candidate","candidate"),("no-call","confirmation/generation/0/case_started"),("returned","confirmation/generation/0/confirmation_row_durable"),("last","confirmation/generation/7/confirmation_before_final")] {
                    child(&study,"confirm",Some(fault),true,label)?;
                }
                child(&study,"confirm",None,false,"confirmation-finish")?;
                let before=file_hash(&study.join("confirmation.r3rows"))?;
                child(&study,"confirm",None,false,"confirmation-repeat")?;child(&study,"report",None,false,"confirmation-pure")?;
                assert_eq!(file_hash(&study.join("confirmation.r3rows"))?,before);
                assert_eq!(confirmation_usage(&study)?.1,8);
                let rows=binary::read_value_records(&study.join("confirmation.r3rows"))?;
                model_outputs.push(rows.iter().skip(1).map(|r|r["raw_tokens"].clone()).collect::<Vec<_>>());
                let seal=seal_receipt(&study)?;let sealed=verified_corpus(&seal_owner(&study)?.join("sealed/confirmation.r3cor"),seal["corpus"].as_str().unwrap())?;
                let es=[sealed.validation[..4].to_vec(),sealed.validation[256..260].to_vec()].concat();
                let observation=base.join("continuous-observation");std::fs::create_dir(&observation)?;
                let mut control=recovery::RunControl::new(std::sync::Arc::new(AtomicBool::new(false)),std::time::Duration::from_secs(30),16*1024*1024)?;
                control.set_call_limits(8,0);
                let reference=confirmation_collect(&observation,&root.join(&e.checkpoint),&es,&l.tokenizer,&rows[0]["identity"],control)?;
                assert_eq!(reference.iter().map(|r|r["raw_tokens"].clone()).collect::<Vec<_>>(),model_outputs[0]);
                child(&root,"confirmation-io",Some("resolution"),false,"confirmation-io")?;
                child(&root,"confirmation-io-retry",None,false,"confirmation-io-retry")?;totals[1]+=1;
                }
                let model=Transformer::init(l.model.config.clone(),93,Device::Cpu)?;
                let mut manifest=l.manifest.clone();let mut state=manifest.training.clone().unwrap();
                state.step=p.origin_step();state.sampler_state=state.step as u64;state.consumed_tokens=p.fork.as_ref().unwrap().origin_input;
                state.target_tokens=p.fork.as_ref().unwrap().origin_target;state.train_loss=None;state.validation_loss=None;
                state.initial_weight_hash=model.weight_hash()?;state.resume_binding=Some(p.binding(&state,&l.tokenizer)?);
                manifest.initial_weight_hash=model.weight_hash()?;manifest.init_seed=93;manifest.training=Some(state);
                let initial=study.join("random-initial.r3m");let adam=Adam::new(&model.vars)?;
                checkpoint::save(&initial,&model,&l.tokenizer,manifest,&adam.moments)?;
                random_updates(&initial,&study.join("random-continuous.r3m"),&root,2)?;
                random_updates(&initial,&study.join("random-one.r3m"),&root,1)?;
                child(&root,"random-step",None,false,"random-step")?;totals[0]+=4;
                let a=checkpoint::load(&study.join("random-continuous.r3m"),Device::Cpu,true)?;
                let b=checkpoint::load(&study.join("random-split.r3m"),Device::Cpu,true)?;
                assert_eq!(a.model.weights_content_id()?,b.model.weights_content_id()?);assert_eq!(optimizer_hash(&a.optimizer)?,optimizer_hash(&b.optimizer)?);assert_eq!(a.manifest.training,b.manifest.training);
                child(&root,"random-eval",None,false,"random-eval")?;
                let fault=format!("eval-{:04}-value4/generation/0/case_started",p.config.max_steps);
                child(&root,"random-eval-split",Some(&fault),true,"random-eval-stop")?;
                child(&root,"random-eval-split",None,false,"random-eval-resume")?;totals[1]+=2;
                child(&root,"random-reentry",None,false,"random-returned-reentry")?;
                let x=binary::read_value_records(&root.join(format!("random-eval/eval-{:04}-value4.r3rows",p.config.max_steps)))?;
                let y=binary::read_value_records(&root.join(format!("random-eval-split/eval-{:04}-value4.r3rows",p.config.max_steps)))?;
                assert_eq!(x[0],y[0]);
                for field in ["raw_tokens","raw_bytes","actual","error","error_class","finish_reason","generation_completed","expected","id"] {assert_eq!(x[1][field],y[1][field]);}
                let after=checkpoint::load(&study.join("random-split.r3m"),Device::Cpu,true)?;
                assert_eq!(b.model.weights_content_id()?,after.model.weights_content_id()?);assert_eq!(optimizer_hash(&b.optimizer)?,optimizer_hash(&after.optimizer)?);assert_eq!(b.manifest.training,after.manifest.training);
            }
            assert!(endpoints.windows(2).all(|w|w[0]==w[1]));
            for root in [parents.join("FIXED"),parents.join("BOTH"),old.join(ARM),mean.join(MEAN_ARMS[0]),mean.join(MEAN_ARMS[1])] {
                let p=historical_plan(&root)?;for end in history(&root,&p)? {totals[0]+=read::<binary::Value>(&root.join(&end.checkpoint).parent().unwrap().join("train-control.r3b"))?["optimizer_calls"].as_u64().unwrap() as usize;totals[1]+=end.generations;totals[2]+=end.teachers;}
            }
            totals[1]+=12+3*8+2*8+16;
            if fid {
                totals[1]-=2*8+16; // No repeated confirmation or reviewer observations.
                totals[1]+=16; // Parent parity plus the additional exact-teacher-cap mode's parity.
                let root=selected_root(&parent);let p=historical_plan(&root)?;
                for end in history(&root,&p)? {totals[0]+=read::<binary::Value>(&root.join(&end.checkpoint).parent().unwrap().join("train-control.r3b"))?["optimizer_calls"].as_u64().unwrap() as usize;totals[1]+=end.generations;totals[2]+=end.teachers;}
            }
            if prec {
                // The additional ancestor uses actual2 updates and its own8 parity calls.
                let root=selected_root(&base.join("continuation-parent"));let p=historical_plan(&root)?;
                for end in history(&root,&p)? {totals[0]+=read::<binary::Value>(&root.join(&end.checkpoint).parent().unwrap().join("train-control.r3b"))?["optimizer_calls"].as_u64().unwrap() as usize;totals[1]+=end.generations;totals[2]+=end.teachers;}
                totals[1]+=8;
            }
            println!("CITATION_TINY profile={profile} optimizer={} generation={} teacher={} FD0 evidence={}",totals[0],totals[1],totals[2],base.display());
            write(&base.join("usage.r3b"),&binary::record!({"optimizer":totals[0],"generation":totals[1],"teacher":totals[2],"FD":0}))?;
            Ok(())
        }
    }
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
    // Synthetic writer/reader fixture only: no generation, teacher or optimizer.
    fn precision_panel_fixture(root:&Path,p:&Plan,step:usize,native:&Path,panel:&Panel,wrong:usize)->Result<()> {
        precision_panel_fixture_with_suffix(root,p,step,native,panel,wrong,None)
    }
    fn precision_panel_fixture_with_suffix(root:&Path,p:&Plan,step:usize,native:&Path,panel:&Panel,wrong:usize,suffix:Option<&str>)->Result<()> {
        let (name,es,ms)=panel;let l=checkpoint::load(native,Device::Cpu,false)?;let tok=&l.tokenizer;
        let key=format!("eval-{step:04}-{name}");let model=l.model.weight_hash()?;
        let binding=binary::record!({"fixture":"synthetic panel; model calls0","policy":digest(p)?,"dataset":digest(&es)?,"step":step,"planned":es.len(),"model":model});
        let mut f=std::fs::OpenOptions::new().create_new(true).write(true).open(root.join(format!("{key}.r3rows")))?;
        append_row(&mut f,&binary::record!({"checkpoint":native,"physical":file_hash(native)?,"binding":binding}))?;
        let mut t=std::fs::OpenOptions::new().create_new(true).write(true).open(root.join(format!("{key}-teachers.r3rows")))?;append_row(&mut t,&binding)?;
        let mut rows=vec![];let mut teachers=vec![];
        for (i,e) in es.iter().enumerate() {
            let mut output=e.clone();if i<wrong {let end=output.answer.chars().next().ok_or_else(||bad("fixture empty answer"))?.len_utf8();output.answer.replace_range(..end,if e.answer.starts_with('0') {"1"}else{"0"});}
            if i==0 {if let Some(suffix)=suffix {output.answer.push_str(suffix);assert!(tok.encode(output.answer.as_bytes())?.len()+1<=e.request.limits.max_tokens as usize);}}
            let mut row=gold(&output,tok)?;row["expected"]=binary::record!(e.answer);row["exact_match"]=binary::record!(output.answer==e.answer);
            row["native_prompt_digest"]=binary::record!(tok.prepare_with_framing(&e.request,p.framing(),l.model.config.context as u32,&l.model.config.id()?)?.token_digest);
            row["framing"]=binary::record!(p.framing().id());append_row(&mut f,&row)?;rows.push(row);
            let mut gold=tok.encode(e.answer.as_bytes())?;gold.push(EOS);
            let teacher=binary::record!({"id":e.id,"ordinal":i,"case":digest(e)?,"teacher":{"training_prompt_matches_generation":true,
                "target_tokens_including_eos":gold.len(),"mean_nll":1.,"target_token_observation":{"gold":gold,"nll":vec![1.;gold.len()]},"after_value":{"eos_minus_citation_logit":0.}}});
            append_row(&mut t,&teacher)?;teachers.push(teacher);
        }
        drop(f);drop(t);let mut s=score(&rows,es,ms)?;s.step=step;s.panel=name.clone();s.model=model;s.dataset=digest(&es)?;
        s.raw_hash=file_hash(&root.join(format!("{key}.r3rows")))?;s.teachers_completed=Some(es.len());s.ce=Some(teacher_ce(&teachers)?);
        publish_confirmed(&root.join(format!("{key}.r3b")),&s)
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
    fn citation_precision_executed_token_caps()->Result<()> {
        let tmp=tempfile::tempdir()?;let study=super::super::tests::orbit_fixture(tmp.path())?;
        let original=plan_read(&study.join("BOTH"))?;
        for (name,input,target) in [("zero",Some(0),0),("uncommitted",Some(1192),76),("cap",Some(2_000_000),130_000),
            ("input-over",Some(2_000_001),0),("target-over",Some(0),130_001),("unknown",None,0)] {
            let dir=tmp.path().join(name);let root=dir.join(MEAN_ARMS[1]);std::fs::create_dir_all(root.join("segment-0000"))?;
            let mut p=original.clone();let o=p.identifiable.as_mut().unwrap();o.study=dir;o.dataset=PREC_DATA.into();o.arm=MEAN_ARMS[1].into();
            let segment=Segment{schema:None,start_hash:None,control_hash:None,phase:None,policy:digest(&p)?,
                checkpoint:"synthetic usage fixture; native/model calls0".into(),checkpoint_hash:String::new(),step:0,input:0,target:0,
                elapsed:0.,generations:0,teachers:0,resume:false,stop:"explicit unit record fixture".into()};
            publish_confirmed(&root.join("segment-0000-finished.r3b"),&segment)?;
            write(&root.join("segment-0000/train-control.r3b"),&binary::record!({"fixture":"known executed work independent of committed0; model calls0",
                "executed_input_tokens_including_uncommitted":input,"executed_target_tokens_including_uncommitted":target}))?;
            let a=p.remaining_input();let b=p.remaining_targets(&root);
            if let Some(input)=input {
                match 2_000_000u64.checked_sub(input) {Some(n)=>assert_eq!(a?,Some(n)),None=>assert!(a.is_err())}
                match 130_000u64.checked_sub(target) {Some(n)=>assert_eq!(b?,Some(n)),None=>assert!(b.is_err())}
            }else{assert!(a.is_err());assert!(b.is_err());}
        }
        println!("PRECISION_EXECUTED_CAPS zero/uncommitted/equal/over/UNKNOWN actual remaining route optimizer0 generation0 teacher0");Ok(())
    }
    #[test]
    fn citation_precision_tape_schedule_and_samples()->Result<()> {
        let tmp=tempfile::tempdir()?;let study=super::super::tests::orbit_fixture(tmp.path())?;
        let source=study.join("BOTH");let mut p=plan_read(&source)?;
        let tok=ByteBpe::load(&source.join("tokenizer.r3b"))?;
        let (c,tm,dm)=source_pool(&source,&p,&tok)?;
        let (public,_,used)=fixture_reservation(&source,tmp.path())?;
        let (c,metadata,_)=make_pool(&c,&tm,&dm,&tok,&read_ids(&public,128)?,&read_ids(&used,128)?)?;
        let root=tmp.path().join("panels");std::fs::create_dir(&root)?;
        copy_native(&source.join("tokenizer.r3b"),&root.join("tokenizer.r3b"))?;
        data::native::write(&root.join("corpus.r3cor"),&c,true)?;
        write(&root.join("metadata.r3b"),&metadata)?;
        p.corpus=file_hash(&root.join("corpus.r3cor"))?;p.metadata=file_hash(&root.join("metadata.r3b"))?;
        p.tiny=false;p.identifiable.as_mut().unwrap().dataset=PREC_DATA.into();
        p.identifiable.as_mut().unwrap().arm=MEAN_ARMS[1].into();
        p.fork=Some(Fork {study:study.clone(),study_hash:String::new(),parent_policy:String::new(),parent_state:String::new(),parent_adam:String::new(),
            origin_step:10496,origin_input:12371968,origin_target:536576,arm:MEAN_ARMS[1].into(),constant_lr:3e-5,target_limit:130000,
            original_corpus:p.corpus.clone(),tokenizer_training_hash:tok.train_hash.clone(),variants:None,variant_metadata:None,alternate_first:vec![],selector:None,selector_metadata:None,flip_first:vec![]});
        p.config.lr=3e-5;p.config.budget_start_step=10496;p.config.max_steps=12032;p.evaluation=evaluation(&p);
        assert_eq!(p.evaluation.train_steps,vec![10752,11264,12032]);
        assert_eq!((p.evaluation.active_seconds,p.evaluation.generation_limit,p.evaluation.teacher_limit),(7200,14336,13120));
        for step in 10497..=12032 {assert_eq!(p.learning_rate(step).to_bits(),3e-5f64.to_bits());}
        for (a,b) in [(10496,10497),(10497,10752),(10752,11264),(11264,11776),(11776,12032)] {assert_eq!(endpoint(&p,a,false)?,b);}
        assert_eq!(endpoint(&p,12032,true)?,12032);assert!(endpoint(&p,12032,false).is_err());assert!(endpoint(&p,12033,true).is_err());
        assert!(!p.evaluation_due(11776));assert!(base_panels(&root,&p,11776).is_err());
        assert_eq!(base_panels(&root,&p,10752)?.iter().map(|v|v.1.len()).collect::<Vec<_>>(),vec![64,64,64]);
        for step in [11264,12032] {
            assert_eq!(base_panels(&root,&p,step)?.iter().map(|v|v.1.len()).collect::<Vec<_>>(),vec![512,512,512,128]);
            assert_eq!(evaluation_action(&p,step,true,false,None),(format!("CANDIDATE_FIXED_AT_{step}"),false));
            assert_eq!(evaluation_action(&p,step,false,false,None),if step==12032 {(format!("FINAL_QUALITY_FAIL_AT_{step}"),false)}else{("CONTINUE_WITHIN_REGISTERED_CAP".into(),true)});
        }
        let rows=suffix();let prefix=vec![[0;8];10496];p.identifiable.as_mut().unwrap().rows=[prefix.clone(),rows[..1536].to_vec()].concat();
        assert_eq!(p.training_draw(10496),rows[0]);assert_eq!(p.training_draw(12031),rows[1535]);assert_eq!(&own(&p).rows[..10496],prefix.as_slice());
        let costs=tape_cost(&rows[..1536],&c.train,&tok)?;let half=tape_cost(&rows[..768],&c.train,&tok)?;
        let counts:Vec<usize>=binary::from_value(costs["counts"].clone())?;
        for (i,pool) in counts.chunks(POOL).enumerate() {assert!(pool.iter().all(|&n|n==if i==0 {4}else{2}));}
        for k in ["input","target","padding"] {assert_eq!(costs[k].as_u64(),half[k].as_u64().map(|v|v*2));}
        for offset in [0,512] {
            let es=&c.validation[offset..offset+512];let ms=&metadata.1[offset..offset+512];
            let mut raw=es.iter().map(|e|gold(e,&tok)).collect::<Result<Vec<_>>>()?;
            for i in [23usize,100,477,479] {let mut e=es[i].clone();e.answer="0".into();if e.answer==es[i].answer {e.answer="1".into();}raw[i]=gold(&e,&tok)?;raw[i]["expected"]=binary::record!(es[i].answer);raw[i]["exact_match"]=binary::record!(false);}
            assert_eq!(reproduction_indices(&p,es,ms,&raw,&tok)?,[20..24,100..104,476..480,0..4].into_iter().flatten().collect::<Vec<_>>());
            assert!(reproduction_indices(&p,es,ms,&raw[..511],&tok).is_err());
            let citation=offset==512;let kind=if citation {"citation"}else{"value"};
            let mut f=std::fs::OpenOptions::new().create_new(true).write(true).open(root.join(format!("eval-12032-{kind}512.r3rows")))?;
            append_row(&mut f,&binary::record!({"fixture":"explicit raw panel; model calls0"}))?;
            for row in &raw {append_row(&mut f,row)?;}drop(f);
            let (selected,rows,indices)=endpoint_reproduction(&root,&p,12032,citation)?;
            assert_eq!(indices,[20..24,100..104,476..480,0..4].into_iter().flatten().collect::<Vec<_>>());
            let name=format!("mean-review-ANSWER-MEAN-{kind}");let checkpoint="synthetic observation binding; model calls0";
            let binding=binary::record!({"fixture":"explicit B receipt consumer boundary; model calls0","cases":digest(&selected)?,
                "identity":{"policy":digest(&p)?,"sample_indices":indices},"checkpoint":checkpoint,"source":p.source,"binary":p.binary});
            let mut f=std::fs::OpenOptions::new().create_new(true).write(true).open(root.join(format!("{name}.r3rows")))?;append_row(&mut f,&binding)?;
            let control=recovery::RunControl::new(std::sync::Arc::new(AtomicBool::new(false)),std::time::Duration::from_secs(30),0)?;
            for (i,(e,mut row)) in selected.iter().zip(rows).enumerate() {
                let attempt=prepare_call(&root,&name,"generation",&binding,e,i)?;
                row["attempt"]=binary::record!(attempt.file_name().unwrap().to_string_lossy());append_row(&mut f,&row)?;resolve_call(&attempt,Some(&row),&control)?;
            }drop(f);
            publish_confirmed(&root.join(format!("{name}-finished.r3b")),&binary::record!({"fixture":"synthetic receipt; actual model calls0",
                "policy":digest(&p)?,"checkpoint":checkpoint,"binding":binding,"completed":16,"matched":16,"error":null,
                "raw":file_hash(&root.join(format!("{name}.r3rows")))?,"control":{"generation_calls":16,"teacher_calls":0,"terminal_reason":"COMPLETED"}}))?;
            assert!(observed(&root,&name,&p,checkpoint,&es[..16],true).is_err());
            raw[0]["expected"]=binary::record!("wrong");assert!(reproduction_indices(&p,es,ms,&raw,&tok).is_err());
        }
        verify_review_observations(&root,&root,&p,12032,"synthetic observation binding; model calls0")?;
        write(&root.join("preparation.r3b"),&binary::record!({"fixture":"B admission; model calls0"}))?;
        let report=root.join("review.txt");std::fs::write(&report,b"synthetic B fixture, not independent acceptance")?;
        let comparison=binary::record!({"endpoints":{"ANSWER-MEAN":{"policy":digest(&p)?,"checkpoint":"synthetic-native","step":12032}}});
        publish_confirmed(&root.join("review-b.r3b"),&binary::record!({"verdict":"PASS","preparation":file_hash(&root.join("preparation.r3b"))?,
            "endpoints":comparison["endpoints"],"report_path":report,"report_hash":file_hash(&report)?}))?;
        verify_review_b(&root,&comparison)?;
        let mut different=comparison.clone();different["endpoints"]["ANSWER-MEAN"]["checkpoint"]=binary::record!("different model");assert!(verify_review_b(&root,&different).is_err());
        different=comparison.clone();different["endpoints"]["ANSWER-MEAN"]["policy"]=binary::record!("different policy");assert!(verify_review_b(&root,&different).is_err());
        std::fs::write(&report,b"changed B report")?;assert!(verify_review_b(&root,&comparison).is_err());
        for (label,step,failed_fit,failed_dev) in [("early-pass",11264,false,false),("fit-fail",11264,true,false),("final-fail",12032,false,true)] {
            let dir=tmp.path().join(label);std::fs::create_dir(&dir)?;
            for name in ["tokenizer.r3b","corpus.r3cor","metadata.r3b"] {copy_native(&root.join(name),&dir.join(name))?;}
            let mut q=p.clone();q.identifiable.as_mut().unwrap().study=dir.clone();q.fork.as_mut().unwrap().study=dir.clone();
            // Explicit single-endpoint record spec; the real multi-step schedule
            // and guard/process chain are checked separately above and in process.
            q.evaluation.train_steps=vec![step];
            let mut l=checkpoint::load(&source.join("initial.r3m"),Device::Cpu,false)?;
            let mut state=TrainingState{resume_binding:None,contrast16:false,parent_checkpoint_hash:None,config:q.config.clone(),
                step,sampler_state:step as u64,consumed_tokens:0,target_tokens:0,corpus_hash:c.manifest.train.sha256.clone(),
                validation_hash:c.manifest.validation.sha256.clone(),previous_corpora:vec![l.tokenizer.train_hash.clone()],initial_weight_hash:l.manifest.initial_weight_hash.clone(),train_loss:None,validation_loss:None};
            state.resume_binding=Some(q.binding(&state,&l.tokenizer)?);l.manifest.training=Some(state);
            let native=dir.join("fixture.r3m");let adam=Adam::new(&l.model.vars)?;checkpoint::save(&native,&l.model,&l.tokenizer,l.manifest,&adam.moments)?;
            let base=base_panels(&dir,&q,step)?;
            let baseline=orbit_score(&base[0].1[..64],&base[0].2[..64],&base[0].1[..64].iter().map(|e|gold(e,&tok)).collect::<Result<Vec<_>>>()?,&tok)?;
            write(&dir.join("selection.r3b"),&binary::record!({"parent_baseline":{"dev":{"screen":baseline}}}))?;
            for panel in &base {precision_panel_fixture(&dir,&q,step,&native,panel,if failed_dev&&panel.0=="citation512" {32}else{0})?;}
            let all=panels(&dir,&q,step)?;assert_eq!(all.len(),if failed_dev {4}else{8});
            if !failed_dev {
                assert!(evaluation_result(&dir,&q,step).is_err()); // Missing fit is never accepted.
                for panel in &all[4..] {precision_panel_fixture(&dir,&q,step,&native,panel,if failed_fit&&panel.0=="old512" {8}else{0})?;}
            }
            let result=evaluation_result(&dir,&q,step)?;
            assert_eq!(result["development"],!failed_dev);assert_eq!(result["fit"],!failed_dev&&!failed_fit);
            assert_eq!(result["eligible"],!failed_dev&&!failed_fit);
            assert_eq!(result["action"],if failed_dev {"FINAL_QUALITY_FAIL_AT_12032"}else if failed_fit {"CONTINUE_WITHIN_REGISTERED_CAP"}else{"CANDIDATE_FIXED_AT_11264"});
            assert_eq!(result["extend"],failed_fit);
        }
        p.identifiable.as_mut().unwrap().dataset=FID_DATA.into();p.config.lr=3e-4;
        assert_eq!(p.learning_rate(10497).to_bits(),3e-4f64.to_bits());
        println!("CITATION_PRECISION_TAPE schedule/half-cycle/orbit-selection LR bits verified optimizer0 generation0 teacher0");Ok(())
    }
    #[test]
    fn citation_fidelity_tape_and_schedule()->Result<()> {
        let tmp=tempfile::tempdir()?;let study=super::super::tests::orbit_fixture(tmp.path())?;
        let root=study.join("BOTH");let mut p=plan_read(&root)?;
        let tok=ByteBpe::load(&root.join("tokenizer.r3b"))?;
        let (c,tm,dm)=source_pool(&root,&p,&tok)?;
        let (public,_,used)=fixture_reservation(&root,tmp.path())?;
        let (c,_,_)=make_pool(&c,&tm,&dm,&tok,&read_ids(&public,128)?,&read_ids(&used,128)?)?;
        let rows=suffix();let full=tape_cost(&rows,&c.train,&tok)?;let half=tape_cost(&rows[..1536],&c.train,&tok)?;
        for (cost,exposures) in [(&full,[8,4,4]),(&half,[4,2,2])] {
            let counts:Vec<usize>=binary::from_value(cost["counts"].clone())?;
            for (pool,chunk) in counts.chunks(POOL).enumerate() {assert!(chunk.iter().all(|&n|n==exposures[pool]));}
        }
        for key in ["input","target","padding"] {assert_eq!(full[key].as_u64(),half[key].as_u64().map(|v|v*2));}
        p.tiny=false;p.identifiable.as_mut().unwrap().dataset=FID_DATA.into();
        // Synthetic absolute policy boundaries: no thousands of optimizer calls.
        p.fork=Some(Fork {study:study.clone(),study_hash:String::new(),parent_policy:String::new(),parent_state:String::new(),parent_adam:String::new(),
            origin_step:7424,origin_input:0,origin_target:0,arm:MEAN_ARMS[1].into(),constant_lr:3e-4,target_limit:260000,
            original_corpus:p.corpus.clone(),tokenizer_training_hash:tok.train_hash.clone(),variants:None,variant_metadata:None,alternate_first:vec![],selector:None,selector_metadata:None,flip_first:vec![]});
        p.config.max_steps=10496;p.evaluation=evaluation(&p);
        assert_eq!(p.evaluation.train_steps,vec![7680,8192,8960,9728,10496]);
        assert_eq!((p.evaluation.generation_limit,p.evaluation.teacher_limit),(14336,13120));
        for (a,b) in [(7424,7425),(7425,7680),(7680,8192),(8192,8704),(8704,8960),(8960,9472),(9472,9728),(9728,10240),(10240,10496)] {assert_eq!(endpoint(&p,a,false)?,b);}
        assert_eq!(citation_offset(&p,8960),1536);assert_eq!(citation_offset(&p,10496),3072);
        assert_eq!(endpoint(&p,10496,true)?,10496);assert!(endpoint(&p,10496,false).is_err());
        assert!(endpoint(&p,7423,false).is_err());assert!(endpoint(&p,10497,true).is_err());
        assert!(!p.evaluation_due(8704));assert!(!p.evaluation_due(9472));assert!(!p.evaluation_due(10240));
        let prefix=vec![[0;8];7424];p.identifiable.as_mut().unwrap().rows=[prefix.clone(),rows.clone()].concat();
        assert_eq!(p.training_draw(7424),rows[0]);assert_eq!(p.training_draw(8960),rows[1536]);assert_eq!(p.training_draw(10495),rows[3071]);
        assert_eq!(&own(&p).rows[..7424],prefix.as_slice());
        println!("CITATION_FIDELITY_TAPE half/full exposure, absolute cursor, evaluation-only final; optimizer0 generation0 teacher0");Ok(())
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
    fn answer_mean_parsed_outside_id_is_not_malformed_prefix() -> Result<()> {
        let (es,_)=generate("C",0,&[])?;let e=&es[0];
        let row=|text:&str|binary::record!({"actual":text});
        for text in ["4입니다. [event:","4입니다. [event:xx]","4",e.answer.as_str()] {
            assert_eq!(valid_outside_ids(std::slice::from_ref(e),&[row(text)]),0);
        }
        let outside=(10_000_000..100_000_000).find(|id|!e.request.evidence.items.iter().any(|r|r.event_id==*id)).unwrap();
        assert_eq!(valid_outside_ids(std::slice::from_ref(e),&[row(&format!("4입니다. [event:{outside}]"))]),1);
        assert_eq!(valid_outside_ids(std::slice::from_ref(e),&[row(&format!("{} [event:{outside}]",e.answer))]),1);
        assert_eq!(valid_outside_ids(std::slice::from_ref(e),&[row(&format!("[event:{outside}] [event:"))]),1);
        println!("ANSWER_CITATION_PARSE malformed/provided/single-and-multiple-valid-outside model_calls0");Ok(())
    }
    #[test]
    fn retained_qa_citation_grammar_boundaries() -> Result<()> {
        let (es,_)=generate("C",0,&[])?;let e=&es[0];
        let provided=e.request.evidence.items[0].event_id;
        let outside=i64::MAX;
        assert!(!e.request.evidence.items.iter().any(|e|e.event_id==outside));
        for text in [format!("[event:{outside}] [event:]"),format!("[event:] [event:{outside}]"),
            format!("한글 [event:미완료 [event:{outside}]"),format!("[event:[event:{outside}]"),
            format!("[event:{outside}] [event:{outside}] [event:{}] [event:]",outside-1)] {
            assert!(citations(&text).is_err());assert!(has_outside_id(e,Some(&text)));
            assert_eq!(valid_outside_ids(std::slice::from_ref(e),&[binary::record!({"actual":text})]),1);
        }
        for text in [format!("[event:{provided}] [event:]"),"[event:]".into(),"[event:0] [event:-1] [event:9223372036854775808]".into()] {
            assert!(citations(&text).is_err());assert!(!has_outside_id(e,Some(&text)));
        }
        for text in ["근거가 없습니다.".into(),format!("한글 [event:{provided}] [event:+{provided}] [event:{outside}]"),format!("[event:000{provided}]")] {
            assert_eq!(individually_valid_ids(&text),citations(&text)?.into_iter().collect());
        }
        assert!(!has_outside_id(e,None));
        assert_eq!(citations("근거가 없습니다.")?,Vec::<i64>::new());
        assert!(None::<&str>.and_then(|s|citations(s).ok()).is_none());
        println!("QA_CITATION_GRAMMAR strict/individual outside-row boundary, nested/unclosed/overflow/UTF8/empty; model_calls0");Ok(())
    }
    #[test]
    fn qa_bridge_two_factor_request_invariants()->Result<()> {
        let(es,_)=generate("C",0,&[])?;
        for name in ["S0Q0","S1Q0","S0Q1","S1Q1"] {
            let changed=bridge_variant(&es[..16],SYSTEM,name)?;
            for(e,old)in changed.iter().zip(&es) {
                assert_eq!(e.id,old.id);assert_eq!(e.family,old.family);assert_eq!(e.answer,old.answer);
                assert_eq!(e.request.request_id,old.request.request_id);assert_eq!(e.request.evidence,old.request.evidence);assert_eq!(e.request.limits,old.request.limits);
                assert_eq!(bridge_resolve(&e.request)?,old.answer);
                let mut q=e.request.clone();q.system=old.request.system.clone();q.input=old.request.input.clone();assert_eq!(digest(&q)?,digest(&old.request)?);
                if name.ends_with("Q1"){assert!(resolve_request(&e.request).is_err());} // Other profiles remain strict.
            }
        }
        let mut wrong=es[0].request.clone();wrong.input=wrong.input.replace(CITATION_QUERY,phrases(Intent::Previous)[0]);assert!(bridge_resolve(&wrong).is_err());
        wrong.input=wrong.input.replace(phrases(Intent::Previous)[0],phrases(Intent::Current)[1]);assert!(bridge_resolve(&wrong).is_err());
        assert!(bridge_variant(&es[..16],SYSTEM,"unregistered").is_err());
        println!("QA_FACTOR_REQUESTS 4x16 same record/ID/value/time/order/gold; only registered system/task changes; model_calls0");Ok(())
    }
    #[test]
    fn qa_bridge_data_tape_and_answer_objective()->Result<()> {
        let t=tempfile::tempdir()?;let study=super::super::tests::orbit_fixture(t.path())?;let root=study.join("BOTH");let p=plan_read(&root)?;let tok=ByteBpe::load(&root.join("tokenizer.r3b"))?;
        let(c,tm,dm)=source_pool(&root,&p,&tok)?;let(public,_,used)=fixture_reservation(&root,t.path())?;
        let(r,(tm,dm,_),_)=make_pool(&c,&tm,&dm,&tok,&read_ids(&public,128)?,&read_ids(&used,128)?)?;
        let originals=digest(&(&r.train,&r.validation))?;let (c,tm,dm,a)=bridge_pool_owned(r.clone(),tm,dm,&tok)?;
        assert_eq!(digest(&(&c.train[..4608],&c.validation[..1536]))?,originals);assert_eq!((c.train.len(),tm.len(),c.validation.len(),dm.len()),(6144,6144,3072,3072));
        let rows=bridge_tape();assert_eq!(a["sample_counts"],binary::record!([3072,1536,1536,6144]));
        for epoch in 0..4 {let drawn=rows[epoch*384..(epoch+1)*384].iter().flat_map(|r|r[4..].iter().copied()).collect::<BTreeSet<_>>();assert_eq!(drawn.len(),1536);}
        let frames=samples(&c.train,&tok,512)?;let ids=rows[0];let b=batch(&frames,&ids,&Device::Cpu)?;let width=b.input.dim(1)?;let vocab=tok.vocab_size();
        let z=candle_core::Var::from_vec((0..8*width*vocab).map(|i|((i%19)as f32-9.)/13.).collect::<Vec<_>>(),(8,width,vocab),&Device::Cpu)?;
        let(_,loss,n,den)=crate::training::response_objective(z.as_tensor(),&b,1.,true)?;assert_eq!(den,8);assert_eq!(n,ids.iter().map(|&i|frames[i].tokens.len()-frames[i].response_start).sum::<usize>());
        let grad=loss.backward()?.get(&z).unwrap().flatten_all()?.to_vec1::<f32>()?;let mut sum=vec![0f32;grad.len()];let mut objective=0.;
        for offset in [0,4]{let mb=batch(&frames,&ids[offset..offset+4],&Device::Cpu)?;let logits=z.narrow(0,offset,4)?.narrow(1,0,mb.input.dim(1)?)?;
            let(_,l,_,d)=crate::training::response_objective(&logits,&mb,1.,true)?;assert_eq!(d,4);objective+=f64::from(l.to_scalar::<f32>()?)/2.;
            for(a,b)in sum.iter_mut().zip((l*0.5)?.backward()?.get(&z).unwrap().flatten_all()?.to_vec1::<f32>()?){*a+=b;}}
        assert!((objective-f64::from(loss.to_scalar::<f32>()?)).abs()<2e-5);assert!(grad.iter().zip(sum).all(|(a,b)|(a-b).abs()<2e-6));
        let mask=b.mask.to_vec2::<f32>()?;for row in 0..8 {let s=&frames[ids[row]];assert_eq!(s.tokens.last(),Some(&EOS));assert_eq!(mask[row][s.tokens.len()-2],1.);
            for pos in 0..width{if mask[row][pos]==0.{assert!(grad[(row*width+pos)*vocab..(row*width+pos+1)*vocab].iter().all(|x|*x==0.));}}}
        let mut scores=BTreeMap::new();for(name,start)in ["value512","citation512","renamed512","S1Q0512","S0Q1512","S1Q1512"].into_iter().zip((0..6).map(|i|i*512)) {
            let es=&c.validation[start..start+512];let ms=&dm[start..start+512];let raw=es.iter().map(|e|gold(e,&tok)).collect::<Result<Vec<_>>>()?;
            let mut s=if name=="value512"{binary::record!({"joint":orbit_score(es,ms,&raw,&tok)?})}else{binary::to_value(score_citation_profile(es,ms,&raw,&tok,true)?)?};
            s["valid_outside_id"]=binary::record!(0);s["parse_failure_rows"]=binary::record!(0);scores.insert(name.into(),s);
        }
        assert!(bridge_dev_pass(&scores)?);for name in scores.keys(){for(field,n)in[("swap_both",231),("all4",115),("full",487)]{let mut bad=scores.clone();bad.get_mut(name).unwrap()["joint"][field]=binary::record!(n);assert!(!bridge_dev_pass(&bad)?);}}
        for field in ["valid_outside_id","parse_failure_rows"] {let mut bad=scores.clone();bad.get_mut("S1Q1512").unwrap()[field]=binary::record!(1);assert!(!bridge_dev_pass(&bad)?);}
        println!("QA_BRIDGE_DATA rows6144 bridge1536 samples12288 input={} target={} complete_pairs/exact_exposure/no_leak/mixed_answer_gradient_8_vs_4plus4/EOS_mask PASS optimizer0 generation0 teacher0 synthetic_backward3 FD0",a["costs"]["input"],a["costs"]["target"]);Ok(())
    }
    #[test]
    fn qa_bridge_raw_conditional_fit()->Result<()> {
        bridge_raw_fit_fixture(false)
    }
    #[test]
    fn bridge_completion_raw_gate()->Result<()> {bridge_raw_fit_fixture(true)}
    fn bridge_raw_fit_fixture(completion:bool)->Result<()> {
        let tmp=tempfile::tempdir()?;let study=super::super::tests::orbit_fixture(tmp.path())?;let source=study.join("BOTH");let original=plan_read(&source)?;
        let tok=ByteBpe::load(&source.join("tokenizer.r3b"))?;let(c,tm,dm)=source_pool(&source,&original,&tok)?;let(public,_,used)=fixture_reservation(&source,tmp.path())?;
        let(c,(tm,dm,_),_)=make_pool(&c,&tm,&dm,&tok,&read_ids(&public,128)?,&read_ids(&used,128)?)?;let(c,tm,dm,_)=bridge_pool_owned(c,tm,dm,&tok)?;
        for label in if completion{vec!["middle-fail","final-fail","fit-fail","mixed-model"]}else{vec!["pass","fit-fail","guard-stop"]} {
            let root=tmp.path().join(label);std::fs::create_dir(&root)?;copy_native(&source.join("tokenizer.r3b"),&root.join("tokenizer.r3b"))?;
            data::native::write(&root.join("corpus.r3cor"),&c,true)?;write(&root.join("metadata.r3b"),&(tm.clone(),dm.clone(),dm.clone()))?;
            let mut p=original.clone();p.tiny=false;p.corpus=file_hash(&root.join("corpus.r3cor"))?;p.metadata=file_hash(&root.join("metadata.r3b"))?;
            let o=p.identifiable.as_mut().unwrap();o.study=root.clone();o.dataset=if completion{BRIDGE_COMPLETION_DATA}else{BRIDGE_DATA}.into();o.arm=MEAN_ARMS[1].into();
            p.fork=Some(Fork{study:root.clone(),study_hash:String::new(),parent_policy:String::new(),parent_state:String::new(),parent_adam:String::new(),origin_step:11264,origin_input:0,origin_target:0,
                arm:MEAN_ARMS[1].into(),constant_lr:3e-5,target_limit:300000,original_corpus:p.corpus.clone(),tokenizer_training_hash:tok.train_hash.clone(),variants:None,variant_metadata:None,alternate_first:vec![],selector:None,selector_metadata:None,flip_first:vec![]});
            if completion{p.fork.as_mut().unwrap().origin_step=12800;}
            let origin=p.origin_step();p.config.max_steps=origin+1536;p.config.budget_start_step=origin;p.config.max_tokens=4_000_000;p.config.warmup=0;p.config.lr=3e-5;p.config.seq_len=512;p.evaluation=evaluation(&p);
            if completion{assert_eq!(p.evaluation.train_steps,vec![13056,13568,14336]);}
            let step=origin+if completion&&label!="middle-fail"{1536}else{768};
            // Explicit single-endpoint record fixture; process tests exercise the actual schedule.
            p.evaluation.train_steps=vec![step];let mut l=checkpoint::load(&source.join("initial.r3m"),Device::Cpu,false)?;
            let mut state=TrainingState{resume_binding:None,contrast16:false,parent_checkpoint_hash:None,config:p.config.clone(),step,sampler_state:step as u64,consumed_tokens:0,target_tokens:0,
                corpus_hash:c.manifest.train.sha256.clone(),validation_hash:c.manifest.validation.sha256.clone(),previous_corpora:vec![l.tokenizer.train_hash.clone()],initial_weight_hash:l.manifest.initial_weight_hash.clone(),train_loss:None,validation_loss:None};
            state.resume_binding=Some(p.binding(&state,&l.tokenizer)?);l.manifest.training=Some(state);let native=root.join("fixture.r3m");let adam=Adam::new(&l.model.vars)?;
            checkpoint::save(&native,&l.model,&l.tokenizer,l.manifest,&adam.moments)?;
            let alternate=root.join("alternate.r3m");
            if label=="mixed-model"{let mut other=checkpoint::load(&native,Device::Cpu,false)?;let v=other.model.vars.values().next().unwrap();v.set(&(v.as_tensor()+0.01)?)?;
                other.model.refresh_identity()?;checkpoint::save(&alternate,&other.model,&other.tokenizer,other.manifest,&adam.moments)?;}
            for panel in bridge_panels(&root,&p,step)? {
                let wrong=if label=="guard-stop"&&panel.0=="value512"{24}else{0};
                let suffix=(completion&&["middle-fail","final-fail"].contains(&label)&&panel.0=="S1Q1512").then_some(" [event:9] [event:]");
                let panel_native=if label=="mixed-model"&&panel.0=="S1Q1512"{alternate.clone()}else{native.clone()};
                precision_panel_fixture_with_suffix(&root,&p,step,&panel_native,&panel,wrong,suffix)?;
            }
            let all=panels(&root,&p,step)?;
            if label=="guard-stop"||label=="middle-fail"{assert_eq!(all.len(),6);}else{assert_eq!(all.len(),7);assert!(bridge_decision(&root,&p,step).is_err());
                precision_panel_fixture(&root,&p,step,&native,&all[6],if label=="fit-fail"{13}else{0})?;}
            if label=="mixed-model"{assert!(bridge_decision(&root,&p,step).is_err());continue;}
            if completion {
                let d=bridge_decision(&root,&p,step)?;assert_eq!(d["eligible"],false);assert_eq!(d["extend"],label=="middle-fail");
                assert_eq!(d["action"],match label{"middle-fail"=>"CONTINUE_WITHIN_REGISTERED_CAP","fit-fail"=>"BRIDGE_FIT_NOT_MET",_=>"FINAL_BRIDGE_QUALITY_FAIL_AT_14336"});
                if label!="fit-fail"{let s=&d["panels"]["S1Q1512"];assert_eq!(s["joint"]["full"],511);assert_eq!(s["valid_outside_id"],1);assert_eq!(s["parse_failure_rows"],1);}
                publish_confirmed(&root.join("fixture-decision.r3b"),&d)?;assert_eq!(read_confirmed::<binary::Value>(&root.join("fixture-decision.r3b"))?,d);continue;
            }
            let d=bridge_decision(&root,&p,step)?;assert_eq!(d["development"],true);assert_eq!(d["eligible"],label=="pass");assert_eq!(d["fit"],label=="pass");assert_eq!(d["extend"],false);
            assert_eq!(d["action"],match label{"pass"=>"CANDIDATE_FIXED_AT_12032","fit-fail"=>"BRIDGE_FIT_NOT_MET",_=>"SEVERE_RETENTION_REGRESSION"});
            publish_confirmed(&root.join("fixture-decision.r3b"),&d)?;assert_eq!(read_confirmed::<binary::Value>(&root.join("fixture-decision.r3b"))?,d);
        }
        println!("QA_BRIDGE_RAW_GATE completion={completion} actual writer/reader/scorer/decision; missing-fit rejected; endpoint gates verified; model_calls0");Ok(())
    }
    #[test]
    #[ignore = "explicit preserved TINY precision parent required; no SMALL fixture substitution"]
    fn qa_bridge_native_process()->Result<()> {
        qa_bridge_process(false)
    }
    #[test]
    #[ignore = "explicit preserved TINY bridge parent and original QA required"]
    fn bridge_completion_native_process()->Result<()> {qa_bridge_process(true)}
    fn qa_bridge_process(completion:bool)->Result<()> {
        let test=if completion{"training::fresh::identifiable::binding::citation::tests::bridge_completion_native_process"}else{"training::fresh::identifiable::binding::citation::tests::qa_bridge_native_process"};
        if let Ok(path)=std::env::var("R3_BRIDGE_CHILD") {
            return match std::env::var("R3_BRIDGE_ACTION").as_deref(){
                Ok("parent")=>bridge_parent_parity(Path::new(&path)),
                Ok("review")=>bridge_review(Path::new(&path),false),
                Ok("errors")=>bridge_review(Path::new(&path),true),
                Ok("qa")=>bridge_diagnostic_qa(Path::new(&path),false),
                Ok("qa-stop")=>{assert!(bridge_diagnostic_qa(Path::new(&path),false).is_err());Ok(())},
                action=>run(Path::new(&path),action!=Ok("one")),
            };
        }
        if !cfg!(feature="test-support"){return Err(bad("bridge explicit TINY test support"));}
        let parent=PathBuf::from(std::env::var("R3_BRIDGE_TEST_PARENT").map_err(|_|bad("preserved TINY parent required"))?);
        let base=PathBuf::from(std::env::var("R3_BRIDGE_TEST_ROOT").map_err(|_|bad("new process evidence path required"))?);std::fs::create_dir(&base)?;let base=base.canonicalize()?;
        let old=historical_plan(&parent)?;if !old.tiny||if completion{!instruction_bridge(&old)}else{!precision(&old)}{return Err(bad("TINY preserved parent profile"));}
        let child=|root:&Path,mode:&str,fault:Option<&str>,label:&str|->Result<()> {
            let mut cmd=std::process::Command::new(std::env::current_exe()?);cmd.args(["--ignored","--exact",test,"--nocapture","--test-threads=1"])
                .env("R3_BRIDGE_CHILD",root).env("R3_BRIDGE_ACTION",mode).env("VECLIB_MAXIMUM_THREADS","1").env("RAYON_NUM_THREADS","1");if let Some(f)=fault{cmd.env("R3_FRESH_CALL_STOP",f);}
            let out=cmd.output()?;std::fs::write(base.join(format!("{label}.stdout")),&out.stdout)?;std::fs::write(base.join(format!("{label}.stderr")),&out.stderr)?;
            assert!(out.status.success(),"{label}: {} {}",String::from_utf8_lossy(&out.stdout),String::from_utf8_lossy(&out.stderr));assert!(String::from_utf8_lossy(&out.stdout).contains("1 passed"));Ok(())};
        let mut natives=vec![];let mut outputs=vec![];let mut totals=[0usize;3];
        for mode in ["continuous","split","eval-only"] {
            let study=base.join(mode);if completion{bridge_prepare_completion(parent.parent().unwrap(),&study,true)?;}else{bridge_prepare_inner(&parent,None,None,&study,true)?;}super::super::tests::fixture_review(&study)?;
            let root=study.join(MEAN_ARMS[1]);let p=plan_read(&root)?;let step=p.config.max_steps;
            if completion{assert_eq!(p.learning_rate(p.origin_step()+1).to_bits(),3e-5f64.to_bits());
                if mode=="continuous"{child(&study,"parent",None,"parent-parity")?;totals[1]+=24;}}
            for which in 0..3{let mut wrong=p.clone();match which{0=>wrong.initial.push('0'),1=>wrong.config.lr=3e-4,_=>wrong.config.seq_len=256};assert!(bridge_verify_plan(&root,&wrong).is_err());}
            let fault=format!("eval-{step:04}-{}/generation/1/fresh_panel_row_durable",if completion{"bridge-fit4"}else{"S0Q14"});
            child(&root,if mode=="split"{"one"}else{"run"},(mode=="eval-only").then_some(fault.as_str()),&format!("{mode}-0"))?;
            let prefix_path=root.join(format!("eval-{step:04}-{}.r3rows",if completion{"bridge-fit4"}else{"S0Q14"}));
            let prefix=if mode=="eval-only"{let h=history(&root,&p)?;assert_eq!(h.last().unwrap().step,step);assert_eq!(h.last().unwrap().phase.as_deref(),Some("EvaluationPending"));Some(binary::read_value_records(&prefix_path)?)}else{None};
            if mode!="continuous"{child(&root,"run",None,&format!("{mode}-1"))?;}
            if let Some(prefix)=prefix{assert_eq!(&binary::read_value_records(&prefix_path)?[..prefix.len()],prefix.as_slice());let h=history(&root,&p)?;
                assert_eq!(read::<binary::Value>(&root.join(format!("segment-{:04}/train-control.r3b",h.len()-1)))?["optimizer_calls"],0);}
            let h=history(&root,&p)?;let end=h.last().unwrap();assert!(!end.resume);assert_eq!(end.stop,if completion{format!("FINAL_BRIDGE_QUALITY_FAIL_AT_{step}")}else{"BRIDGE_DEVELOPMENT_FAIL".into()});assert!(run(&root,true).is_err());
            bridge_review_endpoint(&root,&p)?;let(es,raw,_)=bridge_review_cases(&root,&p,step,false)?;assert_eq!(es.len(),24);assert_eq!(raw.len(),24);
            assert!(bridge_qa(&study,false).is_err());assert!(confirm(&study).is_err());
            if completion {
                let(_,d)=close(&root,&p)?;bridge_diagnostic_admission(&p,end,&d)?;
                for stop in ["CANCELLED","UNKNOWN","INTEGRITY_FAIL","SEVERE_RETENTION_REGRESSION"] {let mut bad=end.clone();bad.stop=stop.into();assert!(bridge_diagnostic_admission(&p,&bad,&d).is_err());}
                let mut missing=d.clone();missing["panels"]["bridge-fit4"]["joint"]["total"]=binary::record!(3);assert!(bridge_diagnostic_admission(&p,end,&missing).is_err());
                if mode=="eval-only" {
                    child(&study,"review",None,"review-normal")?;child(&study,"errors",None,"review-errors")?;
                    for errors in [false,true]{totals[1]+=bridge_review_cases(&root,&p,step,errors)?.0.len();}
                    let report=study.join("fixture-review-b.txt");std::fs::write(&report,b"TINY test authority only; no independent SMALL approval")?;
                    let cmp=comparison(&study,&p,end,&d)?;publish_confirmed(&study.join("review-b.r3b"),&binary::record!({"verdict":"PASS","preparation":file_hash(&study.join("preparation.r3b"))?,"endpoints":cmp["endpoints"],"report_path":report,"report_hash":file_hash(&report)?}))?;
                    child(&study,"qa-stop",Some("bridge-qa-primary/generation/1/confirmation_row_durable"),"qa-0")?;
                    let raw=study.join("bridge-qa-primary.r3rows");let prefix=binary::read_value_records(&raw)?;assert_eq!(prefix.len(),3);
                    let final_path=study.join("bridge-qa-primary-segment-000-finished.r3b");let saved=std::fs::read(&final_path)?;
                    std::fs::rename(&final_path,study.join("fixture-hidden-final"))?;child(&study,"qa-stop",None,"qa-unknown")?;
                    std::fs::rename(study.join("fixture-hidden-final"),&final_path)?;
                    std::fs::write(&final_path,&saved[..saved.len()-1])?;child(&study,"qa-stop",None,"qa-corrupt")?;std::fs::write(&final_path,&saved)?;
                    let mut cancelled_end:binary::Value=binary::from_slice(&saved)?;cancelled_end["control"]["terminal_reason"]=binary::record!("CANCELLED");cancelled_end["control"]["observed_conditions"]=binary::record!(["TIME_BUDGET","CANCELLED"]);
                    std::fs::write(&final_path,binary::to_vec(&cancelled_end)?)?;child(&study,"qa-stop",None,"qa-cancelled-command")?;std::fs::write(&final_path,&saved)?;
                    let resolved=study.join("bridge-qa-primary-generation-0000-000-resolved.r3b");let original=std::fs::read(&resolved)?;
                    let mut cancelled:binary::Value=binary::from_slice(&original)?;cancelled["control"]["observed_conditions"]=binary::record!(["TIME_BUDGET","CANCELLED"]);
                    std::fs::write(&resolved,binary::to_vec(&cancelled)?)?;child(&study,"qa-stop",None,"qa-cancelled")?;std::fs::write(&resolved,&original)?;
                    assert_eq!(binary::read_value_records(&raw)?,prefix);
                    child(&study,"qa",None,"qa-1")?;let full=binary::read_value_records(&raw)?;assert_eq!(full.len(),5);assert_eq!(&full[..prefix.len()],prefix.as_slice());
                    let hash=file_hash(&raw)?;child(&study,"qa",None,"qa-idempotent")?;assert_eq!(file_hash(&raw)?,hash);
                    let usage=segmented_usage(&study,"bridge-qa-primary")?;assert_eq!((usage.1,usage.2),(4,2));totals[1]+=usage.1;
                    let score:binary::Value=read_confirmed(&study.join("bridge-qa-primary-score.r3b"))?;assert_eq!(score["mode"],"DIAGNOSTIC_ONLY");assert_eq!(score["candidate_authority"],false);
                    assert!(bridge_qa(&study,false).is_err());assert!(confirm(&study).is_err());assert!(run(&root,true).is_err());
                }
            }
            let l=checkpoint::load(&root.join(&end.checkpoint),Device::Cpu,true)?;let s=l.manifest.training.as_ref().unwrap();
            natives.push((l.model.weights_content_id()?,optimizer_hash(&l.optimizer)?,s.step,s.sampler_state,s.consumed_tokens,s.target_tokens));
            let mut raw=vec![];for(name,_,_)in bridge_panels(&root,&p,step)?{for r in binary::read_value_records(&root.join(format!("eval-{step:04}-{name}.r3rows")))?.into_iter().skip(1){raw.push(binary::record!({"actual":r["actual"],"tokens":r["raw_tokens"],"finish":r["finish_reason"],"error":r["error"]}));}}outputs.push(raw);
            for(i,e)in h.iter().enumerate(){totals[0]+=read::<binary::Value>(&root.join(format!("segment-{i:04}/train-control.r3b")))?["optimizer_calls"].as_u64().unwrap()as usize;totals[1]+=e.generations;totals[2]+=e.teachers;}
        }
        assert!(natives.windows(2).all(|x|x[0]==x[1]));assert!(outputs.windows(2).all(|x|x[0]==x[1]));if completion{assert_eq!(totals[0],6);assert_eq!(totals[2],84);}else{assert_eq!(totals,[6,72,72]);}
        write(&base.join("usage.r3b"),&binary::record!({"optimizer":totals[0],"generation":totals[1],"teacher":totals[2],"completion":completion}))?;
        println!("QA_BRIDGE_PROCESS optimizer={} generation={} teacher={} continuous2/split1+1/evaluation_only2+0 SAME weights/Adam/clock/raw; preserved prefix; no quality claim; evidence={}",totals[0],totals[1],totals[2],base.display());Ok(())
    }
    #[test]
    fn answer_mean_native_process_resume() -> Result<()> {
        const CHILD:&str="R3_MEAN_CHILD";
        // Random native numeric continuation supplements the full fresh/evaluation
        // process fixture below. It does not generate, force EOS or claim quality.
        if let Ok(root)=std::env::var(CHILD) {
            let root=Path::new(&root);
            return match std::env::var("R3_MEAN_ACTION").as_deref() {
                Ok("old-v")=>parent_observe(root,false),Ok("old-vc")=>parent_observe(root,true),
                Ok("parent")=>mean_parent(root),Ok("review-v")=>mean_review(root,false),Ok("review-vc")=>mean_review(root,true),
                Ok("generic")=>{
                    let loaded=checkpoint::load(root,Device::Cpu,true)?;
                    assert_eq!(loaded.manifest.training.as_ref().unwrap().resume_binding.as_ref().unwrap().family,checkpoint::ANSWER_MEAN_FAMILY);
                    let output=root.with_extension("blocked-output");
                    let result=crate::training::train(crate::training::Run{checkpoint:root,corpus:None,output:&output,resume:true,
                        numeric_probe:false,config:loaded.manifest.training.as_ref().unwrap().config.clone(),stop_after:None,measure_rss:false,
                        extend_steps:None,extend_microbatch:None,extend_sample_group_size:None,extend_curriculum_steps:None,
                        extend_first_target_weight:None,extend_lr:None,extend_warmup:None,source_id:None,replace_corpus:false},
                        std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false)));
                    assert!(result.unwrap_err().to_string().contains("OBJECTIVE_POLICY_UNSUPPORTED"));assert!(!output.exists());
                    println!("MOVED_ANSWER_GENERIC rejected optimizer0 generation0");Ok(())
                },
                Ok("random")=>random_updates(&root.join("../random-one.r3m"),&root.join("../random-split.r3m"),root,1),
                _=>run(root,std::env::var("R3_MEAN_CONTINUOUS").as_deref()==Ok("1")),
            };
        }
        if !cfg!(feature="test-support"){return Err(bad("explicit TINY process profile"));}
        let tmp=tempfile::tempdir()?;
        let base=std::env::var_os("R3_MEAN_TEST_ROOT").map(PathBuf::from).unwrap_or_else(||tmp.path().join("evidence"));
        std::fs::create_dir(&base)?;let base=base.canonicalize()?;
        let child=|root:&Path,action:&str,continuous:bool,fault:bool,label:&str|->Result<()> {
            let mut cmd=std::process::Command::new(std::env::current_exe()?);
            cmd.args(["--exact","training::fresh::identifiable::binding::citation::tests::answer_mean_native_process_resume","--nocapture"])
                .env(CHILD,root).env("R3_MEAN_ACTION",action).env("R3_MEAN_CONTINUOUS",if continuous {"1"}else{"0"})
                .env("VECLIB_MAXIMUM_THREADS","1").env("OMP_NUM_THREADS","1");
            if fault {cmd.env("R3_FRESH_CALL_STOP","eval-0004-citation4/generation/1/fresh_panel_row_durable");}
            let out=cmd.output()?;std::fs::write(base.join(format!("{label}.stdout")),&out.stdout)?;
            std::fs::write(base.join(format!("{label}.stderr")),&out.stderr)?;
            assert!(out.status.success(),"{label}: {} {}",String::from_utf8_lossy(&out.stdout),String::from_utf8_lossy(&out.stderr));
            assert!(String::from_utf8_lossy(&out.stdout).contains("1 passed"));Ok(())
        };
        let parents=super::super::tests::orbit_fixture(&base)?;
        child(&parents.join("FIXED"),"run",true,false,"base-fixed")?;
        child(&parents.join("BOTH"),"run",true,false,"base-both")?;
        let parent=parents.join("BOTH");let (public,private,used)=fixture_reservation(&parent,&base)?;
        let old=base.join("reference");prepare_inner(&parent,&old,&public,&used,true)?;seal(&old,&private)?;
        super::super::tests::fixture_review(&old)?;
        child(&old,"old-v",false,false,"old-parent-v")?;child(&old,"old-vc",false,false,"old-parent-vc")?;
        child(&old.join(ARM),"run",false,false,"old-1")?;child(&old.join(ARM),"run",true,false,"old-2")?;
        let mut totals=[0usize,8,0]; // Completed old-parent observations; updates and evaluations come from receipts.
        for root in [parents.join("FIXED"),parents.join("BOTH"),old.join(ARM)] {
            let p=historical_plan(&root)?;for e in history(&root,&p)? {
                let ctl:binary::Value=read(&root.join(&e.checkpoint).parent().unwrap().join("train-control.r3b"))?;
                totals[0]+=ctl["optimizer_calls"].as_u64().unwrap() as usize;totals[1]+=e.generations;totals[2]+=e.teachers;
            }
        }
        let mut ends=vec![];let mut outputs=vec![];
        for mode in ["continuous","split","evaluation-only"] {
            let study=base.join(mode);mean_prepare(&old,&study,true)?;super::super::tests::fixture_review(&study)?;
            let token=study.join(MEAN_ARMS[0]);let answer=study.join(MEAN_ARMS[1]);
            assert!(authorize(&answer,&plan_read(&answer)?).is_err());
            child(&study,"parent",false,false,&format!("{mode}-parent"))?;totals[1]+=4;
            assert!(authorize(&answer,&plan_read(&answer)?).is_err());
            child(&token,"run",false,false,&format!("{mode}-token1"))?;
            assert!(authorize(&answer,&plan_read(&answer)?).is_err());
            child(&token,"run",true,false,&format!("{mode}-token2"))?;
            assert_eq!(reference_match(&study)?["matched"],12);
            let tp=historical_plan(&token)?;let mut te=history(&token,&tp)?.last().unwrap().clone();
            te.stop="QUALITY_REGRESSION".into();assert!(mean_reference_complete(&te,tp.config.max_steps));
            for failure in ["CANCELLED","UNKNOWN","SAVE_ERROR","INTEGRITY_FAIL"] {
                te.stop=failure.into();assert!(!mean_reference_complete(&te,tp.config.max_steps));
            }
            te.stop="QUALITY_REGRESSION".into();te.phase=Some("EvaluationPending".into());
            assert!(!mean_reference_complete(&te,tp.config.max_steps));
            let ap=plan_read(&answer)?;authorize(&answer,&ap)?;
            let mut wrong=ap.clone();wrong.identifiable.as_mut().unwrap().arm=MEAN_ARMS[0].into();
            assert!(verify_mean_plan(&answer,&wrong).is_err());
            child(&answer,"run",mode!="split",mode=="evaluation-only",&format!("{mode}-answer0"))?;
            let prefix=if mode=="evaluation-only" {
                let h=history(&answer,&ap)?;assert_eq!(h.last().unwrap().phase.as_deref(),Some("EvaluationPending"));
                Some(binary::read_value_records(&answer.join("eval-0004-citation4.r3rows"))?)
            }else{None};
            if mode!="continuous" {child(&answer,"run",true,false,&format!("{mode}-answer1"))?;}
            if let Some(prefix)=prefix {
                let rows=binary::read_value_records(&answer.join("eval-0004-citation4.r3rows"))?;assert_eq!(rows[..prefix.len()],prefix);
                let ctl:binary::Value=read(&answer.join("segment-0001/train-control.r3b"))?;assert_eq!(ctl["optimizer_calls"],0);
            }
            let (end,decision)=close(&answer,&ap)?;assert_eq!(decision["eligible"],false);assert!(!end.resume);
            let loaded=checkpoint::load(&answer.join(&end.checkpoint),Device::Cpu,true)?;
            let state=loaded.manifest.training.as_ref().unwrap();assert_eq!(state.resume_binding,Some(ap.binding(state,&loaded.tokenizer)?));
            ends.push((loaded.model.weights_content_id()?,optimizer_hash(&loaded.optimizer)?,state.step,state.sampler_state,state.consumed_tokens,state.target_tokens));
            let mut rows=vec![];
            for (name,_,_) in panels(&answer,&ap,end.step)? {
                let raw=binary::read_value_records(&answer.join(format!("eval-{:04}-{name}.r3rows",end.step)))?;
                for row in &raw[1..] {rows.push(binary::record!({"actual":row["actual"],"tokens":row["raw_tokens"],"finish":row["finish_reason"],"error":row["error"]}));}
            }outputs.push(rows);
            for root in [&token,&answer] {
                let p=historical_plan(root)?;for e in history(root,&p)? {
                    let ctl:binary::Value=read(&root.join(&e.checkpoint).parent().unwrap().join("train-control.r3b"))?;
                    totals[0]+=ctl["optimizer_calls"].as_u64().unwrap() as usize;totals[1]+=e.generations;totals[2]+=e.teachers;
                }
            }
            if mode=="continuous" {
                let model=Transformer::init(loaded.model.config.clone(),93,Device::Cpu)?;
                let mut manifest=loaded.manifest.clone();let mut state=manifest.training.clone().unwrap();
                state.step=ap.origin_step();state.sampler_state=state.step as u64;
                state.consumed_tokens=ap.fork.as_ref().unwrap().origin_input;state.target_tokens=ap.fork.as_ref().unwrap().origin_target;
                state.train_loss=None;state.validation_loss=None;state.initial_weight_hash=model.weight_hash()?;
                state.resume_binding=Some(ap.binding(&state,&loaded.tokenizer)?);
                manifest.initial_weight_hash=model.weight_hash()?;manifest.init_seed=93;manifest.training=Some(state);
                let initial=study.join("random-initial.r3m");let adam=Adam::new(&model.vars)?;
                checkpoint::save(&initial,&model,&loaded.tokenizer,manifest,&adam.moments)?;
                random_updates(&initial,&study.join("random-continuous.r3m"),&answer,2)?;
                random_updates(&initial,&study.join("random-one.r3m"),&answer,1)?;
                child(&answer,"random",false,false,"random-split")?;totals[0]+=4;
                let a=checkpoint::load(&study.join("random-continuous.r3m"),Device::Cpu,true)?;
                let b=checkpoint::load(&study.join("random-split.r3m"),Device::Cpu,true)?;
                assert_eq!(a.model.weights_content_id()?,b.model.weights_content_id()?);
                assert_eq!(optimizer_hash(&a.optimizer)?,optimizer_hash(&b.optimizer)?);
                assert_eq!(a.manifest.training,b.manifest.training);
                let moved=base.join("moved-answer-native");copy_native(&answer.join(&end.checkpoint),&moved)?;
                child(&moved,"generic",false,false,"moved-generic")?;
                child(&answer,"review-v",false,false,"review-value")?;child(&answer,"review-vc",false,false,"review-citation")?;totals[1]+=8;
            }
            assert!(confirm(&study).is_err());assert!(run(&answer,false).is_err());
            if mode=="continuous" {mean_report(&study)?;}
        }
        assert!(ends.windows(2).all(|v|v[0]==v[1]));assert!(outputs.windows(2).all(|v|v[0]==v[1]));
        // An unresolved peer segment remains blocking even after a valid negative reference.
        let study=base.join("split");let answer=study.join(MEAN_ARMS[1]);let token=study.join(MEAN_ARMS[0]);
        write(&token.join("segment-0002-started.r3b"),&binary::record!({"started":true}))?;
        assert!(authorize(&answer,&plan_read(&answer)?).is_err());
        println!("ANSWER_TINY_PROCESS optimizer{} generation{} teacher{} EOS-tensor-fixture actual fresh2 vs1+1 vs2+0 native/Adam/raw equal; separate random native2 vs newprocess1+1 bitwise, last-step resume optimizer0, moved objective blocked; evidence={}",totals[0],totals[1],totals[2],base.display());Ok(())
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
    if !confirmation_admitted(study,p,end,d)? {
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
    let candidate: binary::Value = read_confirmed(&study.join("confirmation-candidate.r3b"))?;
    let seal = seal_receipt(study)?;
    if candidate
        != confirmation_candidate(study,p,end,&comparison,&seal)?
    {
        return Err(bad("citation candidate changed"));
    }
    let claim:binary::Value=read_confirmed(&seal_owner(study)?.join("confirmation-claim.r3b"))?;
    if claim!=binary::record!({"root":study,"candidate":file_hash(&study.join("confirmation-candidate.r3b"))?,"seal":candidate["seal"]}) {
        return Err(bad("confirmation seal claim mismatch"));
    }
    let corpus = verified_corpus(
        &seal_owner(study)?.join("sealed/confirmation.r3cor"),
        seal["corpus"]
            .as_str()
            .ok_or_else(|| bad("citation seal corpus"))?,
    )?;
    let mut es = corpus.validation;
    let mut ms: Vec<Meta> = read(&seal_owner(study)?.join("sealed/metadata.r3b"))?;
    if es.len() != 512 || ms.len() != 512 {
        return Err(bad("citation confirmation full denominator"));
    }
    if p.tiny {es=[es[..4].to_vec(),es[256..260].to_vec()].concat();ms=[ms[..4].to_vec(),ms[256..260].to_vec()].concat();}
    let n=es.len()/2;
    let rows = observed(study, "confirmation", p, &end.checkpoint_hash, &es, false)?;
    let (elapsed,calls,_)=confirmation_usage(study)?;
    let finished:binary::Value=read_confirmed(&study.join("confirmation-finished.r3b"))?;
    if calls!=es.len() || finished["control"]["elapsed_seconds"]!=elapsed {return Err(bad("confirmation aggregate usage"));}
    let header: binary::Value = read_confirmed(&study.join("confirmation-started.r3b"))?;
    if header["identity"]["candidate"] != file_hash(&study.join("confirmation-candidate.r3b"))? {
        return Err(bad("citation opening identity"));
    }
    let tok = ByteBpe::load(&selected_root(study).join("tokenizer.r3b"))?;
    if confirmation_prefix(study,&header,&es,&tok)?!=rows
        || finished["control"]["observed_conditions"]!=binary::record!([]) {
        return Err(bad("confirmation report call/prefix/terminal disagreement"));
    }
    orbit_validate(&es[..n], &ms[..n], &tok, n)?;
    check_variant(&es[..n], &es[n..], &ms[n..], &tok, true)?;
    let v = orbit_score(&es[..n], &ms[..n], &rows[..n], &tok)?;
    let vc = score_citation(&es[n..], &ms[n..], &rows[n..], &tok)?;
    let passed = pass(&v, 256, 244, 116, 58)
        && pass(&vc.joint, 256, 244, 116, 58)
        && vc.value_correct >= 254
        && vc.citation_support_correct >= 254
        && if is_mean(p) {valid_outside_ids(&es[n..],&rows[n..])==0}else{vc.outside_id == 0};
    let expected = binary::record!({"checkpoint":end.checkpoint_hash,"V":v,"VC":vc,"value_citation_baseline_verified":passed,
        "scope":"two current records, one-digit key/value, fixed grammar, eight-digit event ID","goal1_ready":false,"goal1_accepted":false,"s4":false,"s5":false,"s6":false});
    if read_confirmed::<binary::Value>(&study.join("confirmation-result.r3b"))? != expected {
        return Err(bad("citation confirmation raw/result disagreement"));
    }
    Ok(expected)
}

// One retained-capability curriculum. These helpers only prepare/verify native
// training inputs; product inference never imports a resolver or a bucket.
fn qa_variant(es:&[Episode],ms:&[Meta])->Result<(Vec<Episode>,Vec<Meta>)> {
    super::super::validate_balanced(es,ms)?;
    let mut out=es.to_vec();let mut meta=ms.to_vec();
    for ((e,m),original) in out.iter_mut().zip(&mut meta).zip(es) {
        let (entity,context,intent)=question_intent(&original.request.input)?;
        let choice=u64::from_str_radix(&digest(&(&m.base,intent,"retained-qa-phrase-v1"))?[..16],16).map_err(|_|bad("phrase digest"))? as usize%2;
        e.request.input=format!("{entity} {context} {}",phrases(intent)[choice]);
        e.id=format!("{}-train-phrase",original.id);e.request.request_id=e.id.clone();
        m.id=e.id.clone();m.source_id=Some(original.id.clone());m.template=format!("Q1/{intent:?}/{choice}");
        if question_intent(&e.request.input)?.2!=intent || resolve(&e.request)?!=original.answer
            || e.request.system!=original.request.system || e.request.evidence!=original.request.evidence
            || e.request.limits!=original.request.limits || e.answer!=original.answer {return Err(bad("QA phrase changed intent/evidence/target"));}
    }
    super::super::validate_balanced(&out,&meta)?;Ok((out,meta))
}
// Only the bounded bridge accepts this existing train alias. This resolver is
// a training/evaluation label check, never part of product generation.
fn bridge_resolve(request:&ModelRequest)->Result<String> {
    let(entity,context,intent)=question_intent(&request.input)?;
    let clause=request.input.splitn(3,' ').nth(2).ok_or_else(||bad("bridge task clause"))?;
    if intent!=Intent::Current || ![CITATION_QUERY,phrases(Intent::Current)[0]].contains(&clause) {
        return Err(bad("bridge only permits the two registered current/citation clauses"));
    }
    let mut q=request.clone();q.input=format!("{entity} {context} {CITATION_QUERY}");resolve_request(&q)
}
fn bridge_variant(es:&[Episode],long_system:&str,variant:&str)->Result<Vec<Episode>> {
    if !["S0Q0","S1Q0","S0Q1","S1Q1"].contains(&variant){return Err(bad("unknown bridge factor"));}
    es.iter().map(|original| {
        if original.request.system!=SHORT_SYSTEM || !original.request.input.ends_with(CITATION_QUERY)
            || resolve_request(&original.request)?!=original.answer || original.request.limits.max_tokens!=32 {return Err(bad("bridge original request"));}
        let mut e=original.clone();
        if variant.starts_with("S1"){e.request.system=long_system.into();}
        if variant.ends_with("Q1") {
            let(entity,context,_)=question_intent(&e.request.input)?;
            e.request.input=format!("{entity} {context} {}",phrases(Intent::Current)[0]);
        }
        if bridge_resolve(&e.request)?!=e.answer{return Err(bad("bridge changed selection/answer"));}Ok(e)
    }).collect()
}
fn bridge_probe_cases(study:&Path,name:&str)->Result<(Panel,Option<Vec<binary::Value>>)> {
    let prep:binary::Value=read_confirmed(&study.join("probe-preparation.r3b"))?;
    let parent=Path::new(prep["parent"].as_str().ok_or_else(||bad("probe parent path"))?);
    let old=historical_plan(parent)?;
    if prep["parent_policy"]!=file_hash(&parent.join("plan.r3b"))? {return Err(bad("probe parent policy changed"));}
    let c=verified_corpus(&parent.join("corpus.r3cor"),&old.corpus)?;let(_,dm,_)=verified_metadata(parent,&old)?;
    let start=if name=="value"{0}else{512};let base=&c.validation[start..start+16];let ms=dm[start..start+16].to_vec();
    if ms.chunks_exact(4).any(|g|g.iter().enumerate().any(|(i,m)|m.base!=g[0].base||m.view!=i)) {return Err(bad("probe first four metadata groups"));}
    let es=if ["value","citation"].contains(&name){base.to_vec()}else{bridge_variant(base,prep["long_system"].as_str().ok_or_else(||bad("probe long system"))?,name)?};
    let previous=if ["value","citation"].contains(&name) {
        let raw=parent.join(format!("eval-11264-{name}512.r3rows"));
        if prep["parent_raw"][name]!=file_hash(&raw)? {return Err(bad("probe parent raw changed"));}
        Some(binary::read_value_records(&raw)?[1..17].to_vec())
    }else{None};
    Ok(((name.into(),es,ms),previous))
}
fn bridge_probe_plan(study:&Path)->Result<Plan> {
    let prep:binary::Value=read_confirmed(&study.join("probe-preparation.r3b"))?;
    let p:Plan=read(&study.join("probe-policy.r3b"))?;
    let parent=Path::new(prep["parent"].as_str().ok_or_else(||bad("probe parent"))?);
    let native=Path::new(prep["checkpoint"].as_str().ok_or_else(||bad("probe checkpoint"))?);
    let review=Path::new(prep["review_a1"].as_str().ok_or_else(||bad("probe A1"))?);
    let r:binary::Value=read_confirmed(review)?;
    if prep["contract"]!=BRIDGE_CONTRACT || prep["policy"]!=digest(&p)? || own(&p).study!=study
        || !precision(&p) || p.framing()!=neural::Framing::QuestionEvidence || p.evaluation.generation_limit!=80
        || p.evaluation.teacher_limit!=0 || p.evaluation.active_seconds!=7200 || p.evaluation.segment_seconds!=900
        || prep["parent_policy"]!=file_hash(&parent.join("plan.r3b"))? || p.initial!=file_hash(native)?
        || prep["acceptance"]!=file_hash(&parent.parent().ok_or_else(||bad("probe parent study"))?.join("confirmation-result.r3b"))?
        || prep["balanced_corpus"]!=file_hash(&Path::new(prep["balanced_source"].as_str().ok_or_else(||bad("probe balanced source"))?).join("corpus.r3cor"))?
        || prep["long_system"]!=SYSTEM || prep["short_system"]!=SHORT_SYSTEM
        || prep["review_a1_hash"]!=file_hash(review)? || r["stage"]!="A1" || r["verdict"]!="PASS"
        || r["report_hash"]!=file_hash(Path::new(r["report_path"].as_str().ok_or_else(||bad("A1 report"))?))? {
        return Err(bad("probe preparation/parent/A1 binding"));
    }
    for name in ["value","citation","S1Q0","S0Q1","S1Q1"] {
        let(panel,_)=bridge_probe_cases(study,name)?;
        if prep["cases"][name]!=digest(&panel)? {return Err(bad("probe fixed cases changed"));}
    }
    Ok(p)
}
pub(in super::super::super) fn bridge_probe_prepare(previous:&Path,balanced:&Path,output:&Path,review_a1:&Path)->Result<()> {
    let previous=previous.canonicalize()?;let output=std::path::absolute(output)?;
    let parent=previous.join(MEAN_ARMS[1]);let old=historical_plan(&parent)?;
    let end=history(&parent,&old)?.last().cloned().ok_or_else(||bad("probe parent endpoint"))?;
    let native=parent.join(&end.checkpoint);let l=checkpoint::load(&native,Device::Cpu,true)?;
    let state=l.manifest.training.as_ref().ok_or_else(||bad("probe Adam absent"))?;
    let accepted:binary::Value=read_confirmed(&previous.join("confirmation-result.r3b"))?;
    let r:binary::Value=read_confirmed(review_a1)?;
    if !precision(&old)||old.tiny||end.step!=11264||end.resume||end.stop!="CANDIDATE_FIXED_AT_11264"||end.phase.as_deref()!=Some("Finished")
        || state.step!=end.step || state.sampler_state!=end.step as u64 || state.resume_binding!=Some(old.binding(state,&l.tokenizer)?)
        || !answer_mean(&old)||old.config.lr!=3e-5||old.framing()!=neural::Framing::QuestionEvidence
        || accepted["checkpoint"]!=end.checkpoint_hash||accepted["value_citation_baseline_verified"]!=true
        || r["stage"]!="A1"||r["verdict"]!="PASS" {return Err(bad("probe requires protected accepted11264 and A1"));}
    let q=data::native::read(&balanced.join("corpus.r3cor"))?;
    let long=q.train.first().ok_or_else(||bad("QA system source missing"))?.request.system.clone();
    if long!=SYSTEM||q.train.iter().any(|e|e.request.system!=long) {return Err(bad("actual balanced QA system"));}
    let mut p=old.clone();p.source=source_digest()?;p.binary=file_hash(&std::env::current_exe()?)?;p.initial=end.checkpoint_hash.clone();
    p.initial_weights=l.model.weight_hash()?;p.identifiable.as_mut().unwrap().study=output.clone();
    p.evaluation.generation_limit=80;p.evaluation.teacher_limit=0;p.evaluation.active_seconds=7200;p.evaluation.segment_seconds=900;
    let mut prep=binary::record!({"contract":BRIDGE_CONTRACT,"read_only":true,"parent":parent,"checkpoint":native,"parent_policy":file_hash(&parent.join("plan.r3b"))?,
        "parent_terminal":file_hash(&terminal_path(&parent,&end)?)?,"physical":file_hash(&native)?,"weights":l.model.weights_content_id()?,"adam":optimizer_hash(&l.optimizer)?,
        "state":state,"tokenizer":l.tokenizer.id(),"acceptance":file_hash(&previous.join("confirmation-result.r3b"))?,"long_system":long,"short_system":SHORT_SYSTEM,
        "balanced_source":balanced.canonicalize()?,"balanced_corpus":file_hash(&balanced.join("corpus.r3cor"))?,"review_a1":review_a1.canonicalize()?,"review_a1_hash":file_hash(review_a1)?,
        "policy":digest(&p)?,"source":p.source,"binary":p.binary,"parent_raw":{},"cases":{},"token_lengths":{},"selection":"first4 citation dev metadata groups x4 views; no confirmation; no performance filtering","new_updates":0});
    let c=verified_corpus(&parent.join("corpus.r3cor"),&old.corpus)?;let(_,dm,_)=verified_metadata(&parent,&old)?;
    for name in ["value","citation","S1Q0","S0Q1","S1Q1"] {
        let start=if name=="value"{0}else{512};let es=if ["value","citation"].contains(&name){c.validation[start..start+16].to_vec()}else{bridge_variant(&c.validation[start..start+16],&long,name)?};
        qa_token_cost(&es,&[],&l.tokenizer)?;
        let lengths=es.iter().map(|e|Ok(l.tokenizer.prepare_with_framing(&e.request,p.framing(),2048,&l.model.config.id()?)?.token_ids.len())).collect::<Result<Vec<_>>>()?;
        prep["cases"][name]=binary::record!(digest(&(name,&es,&dm[start..start+16]))?);prep["token_lengths"][name]=binary::record!(lengths);
        if ["value","citation"].contains(&name){prep["parent_raw"][name]=binary::record!(file_hash(&parent.join(format!("eval-11264-{name}512.r3rows")))?);}
    }
    std::fs::create_dir(&output)?;write(&output.join("probe-policy.r3b"),&p)?;publish_confirmed(&output.join("probe-preparation.r3b"),&prep)?;
    bridge_probe_plan(&output)?;
    println!("QA_FACTOR_PREPARED parent11264 bases4 generation_cap80 optimizer0 teacher0 no_confirmation_rows");Ok(())
}
pub(in super::super::super) fn bridge_probe(study:&Path,name:&str)->Result<()> {
    let study=study.canonicalize()?;let p=bridge_probe_plan(&study)?;
    if p.source!=source_digest()? || p.binary!=file_hash(&std::env::current_exe()?)?{return Err(bad("probe source/binary freeze"));}
    let prep:binary::Value=read_confirmed(&study.join("probe-preparation.r3b"))?;
    let (panel,previous)=bridge_probe_cases(&study,name)?;let key=format!("qa-factor-{name}");
    orbit_observe(&study,&key,Path::new(prep["checkpoint"].as_str().unwrap()),&panel.1,&binary::record!({"policy":digest(&p)?,"checkpoint":p.initial}),previous.as_deref(),observation_control(&p,16,0)?)?;
    observed(&study,&key,&p,&p.initial,&panel.1,previous.is_some())?;
    println!("QA_FACTOR_OBSERVED {name} generations16 teacher0 optimizer0 parity={}",previous.is_some());Ok(())
}
pub(in super::super::super) fn bridge_probe_report(study:&Path)->Result<()> {
    let study=study.canonicalize()?;let p=bridge_probe_plan(&study)?;
    let prep:binary::Value=read_confirmed(&study.join("probe-preparation.r3b"))?;
    let tok=ByteBpe::load(&Path::new(prep["parent"].as_str().unwrap()).join("tokenizer.r3b"))?;
    let mut out=BTreeMap::new();let mut needs_training=false;let mut output_tokens=0usize;
    for name in ["value","citation","S1Q0","S0Q1","S1Q1"] {
        let (panel,previous)=bridge_probe_cases(&study,name)?;let rows=observed(&study,&format!("qa-factor-{name}"),&p,&p.initial,&panel.1,previous.is_some())?;
        output_tokens+=rows.iter().map(|r|r["raw_tokens"].as_array().map_or(0,Vec::len)).sum::<usize>();
        if name=="value" {orbit_score(&panel.1,&panel.2,&rows,&tok)?;continue;}
        let s=score_citation_profile(&panel.1,&panel.2,&rows,&tok,true)?;
        let outside=valid_outside_ids(&panel.1,&rows);let parse=rows.iter().filter(|r|r["actual"].as_str().is_none_or(|s|citations(s).is_err())).count();
        let pass=pass(&s.joint,16,15,7,3)&&s.joint.swap_both>=7&&outside==0&&parse==0;
        if name!="citation"{needs_training|=!pass;}
        let entry=binary::record!({"score":s,"valid_outside_rows":outside,"parse_failure_rows":parse,"gate":pass,"raw":file_hash(&study.join(format!("qa-factor-{name}.r3rows")))?});
        println!("QA_FACTOR {name} FULL={}/16 QB={}/8 SB={}/8 ALL4={}/4 value={} support={} outside={outside} parse={parse} errors={}",s.joint.full,s.joint.query_both,s.joint.swap_both,s.joint.all4,s.value_correct,s.citation_support_correct,s.joint.errors);
        out.insert(if name=="citation"{"S0Q0"}else{name},entry);
    }
    let usage=work(&p)?;if usage.1!=80||usage.2!=0{return Err(bad("factor aggregate usage"));}
    let result=binary::record!({"contract":BRIDGE_CONTRACT,"preparation":file_hash(&study.join("probe-preparation.r3b"))?,"factors":out,"needs_training":needs_training,
        "status":if needs_training{"BRIDGE_PREPARATION_REQUIRED"}else{"NO_TRAINING_JUSTIFIED_BY_THIS_PROBE"},"scope":"four dependent bases; prompt positions/length change; not a general QA gate",
        "usage":usage,"output_tokens":output_tokens,"optimizer":0,"S4":"NOT_OPENED","GOAL1_ACCEPTED":false});
    let path=study.join("probe-result.r3b");if path.exists(){if read_confirmed::<binary::Value>(&path)?!=result{return Err(bad("factor result disagreement"));}}else{publish_confirmed(&path,&result)?;}
    println!("QA_FACTOR_COMPLETE generation80 teacher0 optimizer0 output_tokens={output_tokens} needs_training={needs_training}");Ok(())
}
fn bridge_tape()->Vec<[usize;8]> {
    // Each64-pair round visits distinct bases before changing form/ID/assignment.
    let pairs=(0..12).flat_map(|round|(0..64).map(move|base|
        3*POOL+(round%3)*512+((round/3)%2)*256+base*4+(round/6)*2)).collect::<Vec<_>>();
    (0..1536).map(|step|{let v=(step%768)*2;let c=(1+step%2)*POOL+((step/2)%768)*2;
        let a=pairs[(2*step)%768];let b=pairs[(2*step+1)%768];[v,v+1,c,c+1,a,a+1,b,b+1]}).collect()
}
fn bridge_episodes(es:&[Episode],ms:&[Meta],name:&str)->Result<(Vec<Episode>,Vec<Meta>)> {
    let mut out=bridge_variant(es,SYSTEM,name)?;let mut meta=ms.to_vec();
    if out.len()!=meta.len(){return Err(bad("bridge metadata length"));}
    for (i,((e,m),source))in out.iter_mut().zip(&mut meta).zip(es).enumerate() {
        e.id=format!("{BRIDGE_DATA}/{name}/{}",source.id);e.family=format!("{}/{BRIDGE_DATA}/{name}",source.family);e.sequence=e.family.clone();
        e.request.request_id=e.id.clone();m.source_id=Some(source.id.clone());m.id=e.id.clone();m.base=e.family.clone();m.template=format!("{name}/{}",m.template);
        let mut restored=e.request.clone();restored.system=source.request.system.clone();restored.input=source.request.input.clone();restored.request_id=source.request.request_id.clone();
        if digest(&restored)?!=digest(&source.request)?||e.answer!=source.answer||m.view!=ms[i].view{return Err(bad("bridge changes outside declared fields"));}
    }
    Ok((out,meta))
}
fn bridge_pool(parent:&Path,tok:&ByteBpe)->Result<(data::native::Corpus,Vec<Meta>,Vec<Meta>,binary::Value)> {
    let old=historical_plan(parent)?;let c=verified_corpus(&parent.join("corpus.r3cor"),&old.corpus)?;let(tm,dm,_)=verified_metadata(parent,&old)?;
    bridge_pool_owned(c,tm,dm,tok)
}
fn bridge_pool_owned(c:data::native::Corpus,tm:Vec<Meta>,dm:Vec<Meta>,tok:&ByteBpe)->Result<(data::native::Corpus,Vec<Meta>,Vec<Meta>,binary::Value)> {
    if c.train.len()!=4608||c.validation.len()!=1536||tm.len()!=4608||dm.len()!=1536{return Err(bad("bridge retained corpus shape"));}
    let mut selected=(0..384).map(|i|Ok((digest(&(&tm[POOL+4*i].base,BRIDGE_DATA))?,i))).collect::<Result<Vec<_>>>()?;selected.sort();selected.truncate(64);
    let mut source=vec![];let mut meta=vec![];
    for id_version in 1..=2 {for(_,base)in &selected {
        let at=id_version*POOL+4*base;
        if tm[at..at+4].iter().enumerate().any(|(view,m)|m.base!=tm[at].base||m.view!=view){return Err(bad("bridge complete source orbit"));}
        source.extend_from_slice(&c.train[at..at+4]);meta.extend_from_slice(&tm[at..at+4]);
    }}
    let mut train=c.train.clone();let mut train_meta=tm.clone();let mut dev=c.validation.clone();let mut dev_meta=dm.clone();
    for name in ["S1Q0","S0Q1","S1Q1"] {
        let(es,ms)=bridge_episodes(&source,&meta,name)?;train.extend(es);train_meta.extend(ms);
        let(es,ms)=bridge_episodes(&c.validation[512..1024],&dm[512..1024],name)?;dev.extend(es);dev_meta.extend(ms);
    }
    let heldout=c.validation.iter().map(semantic_skeleton).collect::<Result<BTreeSet<_>>>()?;
    let mut prompts=BTreeSet::new();
    for e in &train {if heldout.contains(&semantic_skeleton(e)?){return Err(bad("bridge train/dev semantic overlap"));}
        prompts.insert(digest(&tok.prepare_with_framing(&e.request,neural::Framing::QuestionEvidence,2048,"bridge-split")?.token_ids)?);}
    for e in &dev {if prompts.contains(&digest(&tok.prepare_with_framing(&e.request,neural::Framing::QuestionEvidence,2048,"bridge-split")?.token_ids)?){return Err(bad("bridge prompt leakage"));}}
    let rows=bridge_tape();let costs=qa_token_cost(&train,&rows,tok)?;qa_token_cost(&dev,&[],tok)?;
    if costs["input"].as_u64().is_none_or(|n|n>4_000_000)||costs["target"].as_u64().is_none_or(|n|n>300_000){return Err(bad("BLOCKED_BUDGET bridge token costs"));}
    for(i,count)in costs["counts"].as_array().unwrap().iter().enumerate(){let expected=if i<POOL{2}else if i<3*POOL{1}else{4};if *count!=expected{return Err(bad("bridge exact exposure"));}}
    for row in &rows {for pair in row.chunks_exact(2) {if pair[1]!=pair[0]+1||train_meta[pair[0]].base!=train_meta[pair[1]].base||train_meta[pair[1]].view!=train_meta[pair[0]].view+1{return Err(bad("bridge complete query pair"));}}
        if semantic_skeleton(&train[row[4]])?==semantic_skeleton(&train[row[6]])?{return Err(bad("bridge batch repeats semantic base"));}}
    let mut manifest=c.manifest;manifest.generator=BRIDGE_DATA.into();manifest.split_rule="unchanged retention; train-only hash64 sources, three request variants; inherited dev semantic groups".into();manifest.train=data::native::split("train",&train);manifest.validation=data::native::split("validation",&dev);
    let audit=binary::record!({"selected":selected,"source_metadata":meta,"costs":costs,"physical_rows":6144,"bridge_rows":1536,"bridge_dev_rows":1536,"independent_dev_bases":128,"case_counts_V_VC0_VC1_bridge":[2,1,1,4],"sample_counts":[3072,1536,1536,6144],"ANSWER_coefficients":[0.25,0.125,0.125,0.5],"normalizer":8,"selection_uses_model_scores":false,"confirmation_read":false});
    Ok((data::native::from_episodes(manifest,train,dev)?,train_meta,dev_meta,audit))
}
fn bridge_plan(old:&Plan,study:&Path,s:&binary::Value)->Result<Plan> {
    let mut p=old.clone();let state:TrainingState=binary::from_value(s["parent_state"].clone())?;
    let get=|key:&str|s[key].as_str().map(str::to_owned).ok_or_else(||bad("bridge policy field"));
    p.source=get("source")?;p.binary=get("binary")?;p.initial=get("physical")?;p.initial_weights=get("weights")?;
    p.corpus=get("corpus")?;p.transfer=get("transfer")?;p.metadata=get("metadata")?;
    p.config.seq_len=512;p.config.lr=3e-5;p.config.warmup=0;p.config.max_steps=state.step+if p.tiny{2}else{1536};
    p.config.budget_start_step=state.step;p.config.budget_start_tokens=state.consumed_tokens;p.config.max_tokens=state.consumed_tokens+4_000_000;
    let o=p.identifiable.as_mut().ok_or_else(||bad("bridge parent profile"))?;o.study=study.into();o.dataset=if s["contract"]==BRIDGE_COMPLETION_CONTRACT{BRIDGE_COMPLETION_DATA}else{BRIDGE_DATA}.into();o.rows.truncate(state.step);
    if o.rows.len()!=state.step{return Err(bad("bridge parent tape cursor"));}o.rows.extend(bridge_tape().into_iter().take(if p.tiny{2}else{1536}));p.train_order=digest(&o.rows)?;p.order=vec![(0..6144).collect()];
    let f=p.fork.as_mut().ok_or_else(||bad("bridge parent fork"))?;f.study=study.into();f.study_hash=file_hash(&study.join("selection.r3b"))?;
    f.parent_policy=digest(old)?;f.parent_state=digest(&state)?;f.parent_adam=get("adam")?;f.origin_step=state.step;f.origin_input=state.consumed_tokens;f.origin_target=state.target_tokens;
    f.original_corpus=p.corpus.clone();f.constant_lr=3e-5;f.target_limit=300_000;p.evaluation=evaluation(&p);Ok(p)
}
pub(in super::super::super) fn bridge_prepare(probe:&Path,old_qa:&Path,output:&Path)->Result<()> {
    let probe=probe.canonicalize()?;bridge_probe_plan(&probe)?;
    let observed:binary::Value=read_confirmed(&probe.join("probe-result.r3b"))?;
    if observed["status"]!="BRIDGE_PREPARATION_REQUIRED"||observed["needs_training"]!=true||observed["usage"][1]!=80||observed["usage"][2]!=0{return Err(bad("probe does not justify bridge training"));}
    let prep:binary::Value=read_confirmed(&probe.join("probe-preparation.r3b"))?;
    bridge_prepare_inner(Path::new(prep["parent"].as_str().ok_or_else(||bad("bridge parent"))?),Some(&probe),Some(old_qa),output,false)
}
fn bridge_prepare_inner(parent:&Path,probe:Option<&Path>,old_qa:Option<&Path>,output:&Path,tiny:bool)->Result<()> {
    if tiny!=cfg!(all(test,feature="test-support")) || tiny!=probe.is_none(){return Err(bad("explicit bridge TINY/production boundary"));}
    let parent=parent.canonicalize()?;let old=historical_plan(&parent)?;let end=history(&parent,&old)?.last().cloned().ok_or_else(||bad("bridge parent endpoint"))?;
    let native=parent.join(&end.checkpoint);let l=checkpoint::load(&native,Device::Cpu,true)?;let state=l.manifest.training.as_ref().ok_or_else(||bad("bridge parent optimizer"))?;
    if !precision(&old)||old.tiny!=tiny||end.resume||end.phase.as_deref()!=Some("Finished")||(!tiny&&(end.step!=11264||end.stop!="CANDIDATE_FIXED_AT_11264"))
        || state.step!=end.step||state.sampler_state!=end.step as u64||state.resume_binding!=Some(old.binding(state,&l.tokenizer)?)
        || !answer_mean(&old)||old.config.lr!=3e-5||old.config.first_target_weight!=1.||old.config.microbatch!=8||old.config.accumulation!=1||old.framing()!=neural::Framing::QuestionEvidence{return Err(bad("bridge exact protected parent/objective/clock"));}
    let(c,tm,dm,audit)=bridge_pool(&parent,&l.tokenizer)?;let output=std::path::absolute(output)?;
    let mut s=binary::record!({"contract":BRIDGE_CONTRACT,"source":source_digest()?,"binary":file_hash(&std::env::current_exe()?)?,"parent":parent,"parent_endpoint":end,"parent_policy":file_hash(&parent.join("plan.r3b"))?,"parent_terminal":file_hash(&terminal_path(&parent,&end)?)?,
        "parent_state":state,"physical":file_hash(&native)?,"weights":l.model.weight_hash()?,"adam":optimizer_hash(&l.optimizer)?,"probe":probe,"audit":audit,"new_updates":if tiny{2}else{1536},"historical_resume_unchanged":true,"objective":checkpoint::ANSWER_MEAN_OBJECTIVE,"lr_bits":3e-5f64.to_bits()});
    if let Some(probe)=probe {s["probe_result"]=binary::record!(file_hash(&probe.join("probe-result.r3b"))?);s["probe_preparation"]=binary::record!(file_hash(&probe.join("probe-preparation.r3b"))?);}
    if let Some(path)=old_qa {let path=path.canonicalize()?;let mut hashes=BTreeMap::new();for name in ["corpus.r3cor","transfer.r3cor","metadata.r3b"]{hashes.insert(name,file_hash(&path.join(name))?);}s["old_qa"]=binary::record!(path);s["old_qa_hashes"]=binary::record!(hashes);
        for transfer in [false,true]{bridge_old_qa(&s,transfer)?;}
    }else if !tiny{return Err(bad("bridge conditional original QA provenance required"));}
    std::fs::create_dir(&output)?;let root=output.join(MEAN_ARMS[1]);std::fs::create_dir(&root)?;
    data::native::write(&root.join("corpus.r3cor"),&c,true)?;copy_native(&root.join("corpus.r3cor"),&root.join("transfer.r3cor"))?;write(&root.join("metadata.r3b"),&(tm,dm.clone(),dm))?;
    copy_native(&native,&root.join("initial.r3m"))?;copy_native(&parent.join("tokenizer.r3b"),&root.join("tokenizer.r3b"))?;
    for name in ["corpus","transfer","metadata"]{s[name]=binary::record!(file_hash(&root.join(format!("{name}.{}",if name=="metadata"{"r3b"}else{"r3cor"})))?);}
    write(&output.join("selection.r3b"),&s)?;let p=bridge_plan(&old,&output,&s)?;write(&root.join("plan.r3b"),&p)?;bridge_verify_plan(&root,&p)?;
    if !p.parent_entry(&root.join("initial.r3m"),&l)?{return Err(bad("bridge native parent entry"));}
    publish_confirmed(&output.join("preparation.r3b"),&binary::record!({"contract":BRIDGE_CONTRACT,"source":p.source,"binary":p.binary,"selection":file_hash(&output.join("selection.r3b"))?,"arms":{(MEAN_ARMS[1]):{"policy":file_hash(&root.join("plan.r3b"))?,"initial":p.initial,"corpus":p.corpus,"tape":p.train_order}},"optimizer":0,"generation":0,"teacher":0}))?;
    println!("QA_BRIDGE_PREPARED rows6144 bridge1536 updates0 input={} target={} samples12288 A2_PENDING",s["audit"]["costs"]["input"],s["audit"]["costs"]["target"]);Ok(())
}
fn bridge_completion_parent(old:&Plan,end:&Segment,d:&binary::Value)->Result<()> {
    if own(old).dataset!=BRIDGE_DATA||!answer_mean(old)||end.resume||end.phase.as_deref()!=Some("Finished")
        || end.step!=old.config.max_steps||(!old.tiny&&end.step!=12800)||end.stop!="BRIDGE_DEVELOPMENT_FAIL"
        ||d["step"]!=end.step||d["stop"]!=end.stop||d["extend"]!=false||d["eligible"]!=false||d["regression"]!=false
        ||old.config.lr.to_bits()!=3e-5f64.to_bits()||old.config.first_target_weight!=1.||old.config.seq_len!=512
        ||old.config.microbatch!=8||old.config.accumulation!=1||old.config.warmup!=0||old.framing()!=neural::Framing::QuestionEvidence {
        return Err(bad("completion requires the closed original bridge12800 parent"));
    }Ok(())
}
fn bridge_completion_tape(old:&Plan,end:&Segment)->Result<Vec<[usize;8]>> {
    let count=if old.tiny{2}else{1536};let tape=bridge_tape()[..count].to_vec();
    if end.step.checked_sub(old.origin_step())!=Some(count)||own(old).rows.get(old.origin_step()..end.step)!=Some(tape.as_slice()) {
        return Err(bad("original consumed bridge suffix differs from registered tape"));
    }Ok(tape)
}
pub(in super::super::super) fn bridge_prepare_completion(previous:&Path,output:&Path,tiny:bool)->Result<()> {
    if tiny!=cfg!(all(test,feature="test-support")){return Err(bad("explicit completion production/TINY boundary"));}
    let previous=previous.canonicalize()?;let parent=previous.join(MEAN_ARMS[1]);let old=historical_plan(&parent)?;
    let(end,d)=close(&parent,&old)?;bridge_completion_parent(&old,&end,&d)?;
    if old.tiny!=tiny{return Err(bad("completion model size"));}
    let cmp=comparison(&previous,&old,&end,&d)?;
    if !tiny {verify_review_b(&previous,&cmp)?;}
    let tape=bridge_completion_tape(&old,&end)?;let native=parent.join(&end.checkpoint);
    let l=checkpoint::load(&native,Device::Cpu,true)?;let state=l.manifest.training.as_ref().ok_or_else(||bad("completion missing Adam/state"))?;
    if state.step!=end.step||state.sampler_state!=end.step as u64||state.resume_binding!=Some(old.binding(state,&l.tokenizer)?)
        ||state.resume_binding.as_ref().is_none_or(|b|b.family!=checkpoint::ANSWER_MEAN_FAMILY||b.normalizer!=2)
        ||state.config!=old.config||(!tiny&&l.optimizer.len()!=136){return Err(bad("completion parent native/objective/Adam/clock"));}
    let c=verified_corpus(&parent.join("corpus.r3cor"),&old.corpus)?;let(tm,dm,xm)=verified_metadata(&parent,&old)?;
    if c.train.len()!=6144||c.validation.len()!=3072||tm.len()!=6144||dm.len()!=3072||digest(&xm)?!=digest(&dm)?{return Err(bad("completion unchanged corpus shape"));}
    let costs=qa_token_cost(&c.train,&tape,&l.tokenizer)?;
    if !tiny&&(costs["input"]!=2_064_384||costs["target"]!=162_816||costs["padding"]!=233_472){return Err(bad("completion original token costs mismatch"));}
    let prior:binary::Value=read(&previous.join("selection.r3b"))?;
    let mut protected=BTreeMap::<PathBuf,String>::new();
    for path in [parent.join("plan.r3b"),terminal_path(&parent,&end)?,parent.join(format!("citation-decision-{:04}.r3b",end.step)),native.clone(),
        parent.join("corpus.r3cor"),parent.join("transfer.r3cor"),parent.join("metadata.r3b"),parent.join("tokenizer.r3b"),previous.join("selection.r3b"),previous.join("preparation.r3b")] {
        protected.insert(path.clone(),file_hash(&path)?);
    }
    if !tiny {let b:binary::Value=read_confirmed(&previous.join("review-b.r3b"))?;for path in [previous.join("review-b.r3b"),PathBuf::from(b["report_path"].as_str().ok_or_else(||bad("parent B report"))?)]{protected.insert(path.clone(),file_hash(&path)?);}}
    let output=std::path::absolute(output)?;
    let mut s=binary::record!({"contract":BRIDGE_COMPLETION_CONTRACT,"source":source_digest()?,"binary":file_hash(&std::env::current_exe()?)?,"parent":parent,
        "parent_endpoint":end,"parent_policy":digest(&old)?,"parent_terminal":file_hash(&terminal_path(&parent,&end)?)?,"parent_state":state,"parent_protected":protected,
        "physical":file_hash(&native)?,"weights":l.model.weight_hash()?,"manifest_weights":l.manifest.weights_sha256,"tensor_content":l.model.weights_content_id()?,"adam":optimizer_hash(&l.optimizer)?,
        "corpus":old.corpus,"transfer":old.transfer,"metadata":old.metadata,"tokenizer":old.tokenizer,"tape_suffix":digest(&tape)?,"costs":costs,
        "old_qa":prior["old_qa"],"old_qa_hashes":prior["old_qa_hashes"],"parent_retention":d["panels"],"new_updates":tape.len(),
        "historical_resume_unchanged":true,"objective":checkpoint::ANSWER_MEAN_OBJECTIVE,"lr_bits":3e-5f64.to_bits(),"diagnostic_scope":"normal-complete endpoint plus independent B; old QA640 only; no candidate/confirmation/S4 authority"});
    #[cfg(all(test,feature="test-support"))]
    if tiny {let path=PathBuf::from(std::env::var("R3_BRIDGE_QA_FIXTURE").map_err(|_|bad("explicit original QA TINY fixture required"))?).canonicalize()?;
        let mut hashes=BTreeMap::new();for name in ["corpus.r3cor","transfer.r3cor","metadata.r3b"]{hashes.insert(name,file_hash(&path.join(name))?);}s["old_qa"]=binary::record!(path);s["old_qa_hashes"]=binary::record!(hashes);}
    if !tiny{for transfer in [false,true]{bridge_old_qa(&s,transfer)?;}}
    std::fs::create_dir(&output)?;let root=output.join(MEAN_ARMS[1]);std::fs::create_dir(&root)?;
    for name in ["corpus.r3cor","transfer.r3cor","metadata.r3b","tokenizer.r3b"]{copy_native(&parent.join(name),&root.join(name))?;}
    copy_native(&native,&root.join("initial.r3m"))?;write(&output.join("selection.r3b"),&s)?;
    let p=bridge_plan(&old,&output,&s)?;write(&root.join("plan.r3b"),&p)?;bridge_completion_verify(&root,&p)?;
    if !p.parent_entry(&root.join("initial.r3m"),&l)?{return Err(bad("completion parent entry"));}
    publish_confirmed(&output.join("preparation.r3b"),&binary::record!({"contract":BRIDGE_COMPLETION_CONTRACT,"source":p.source,"binary":p.binary,"selection":file_hash(&output.join("selection.r3b"))?,
        "arms":{(MEAN_ARMS[1]):{"policy":file_hash(&root.join("plan.r3b"))?,"initial":p.initial,"corpus":p.corpus,"tape":p.train_order}},"optimizer":0,"generation":0,"teacher":0}))?;
    println!("QA_BRIDGE_COMPLETION_PREPARED parent={} rows6144 unchanged suffix={} input={} target={} A_PENDING optimizer0 generation0 teacher0",end.step,tape.len(),s["costs"]["input"],s["costs"]["target"]);Ok(())
}
fn bridge_completion_verify(root:&Path,p:&Plan)->Result<()> {
    let s:binary::Value=read(&own(p).study.join("selection.r3b"))?;
    let parent=Path::new(s["parent"].as_str().ok_or_else(||bad("completion parent path"))?);let old:Plan=read(&parent.join("plan.r3b"))?;
    let end:Segment=binary::from_value(s["parent_endpoint"].clone())?;
    let d:binary::Value=read_confirmed(&parent.join(format!("citation-decision-{:04}.r3b",end.step)))?;
    bridge_completion_parent(&old,&end,&d)?;let tape=bridge_completion_tape(&old,&end)?;
    let protected:BTreeMap<PathBuf,String>=binary::from_value(s["parent_protected"].clone())?;
    for(path,hash)in &protected{if file_hash(path)?!=*hash{return Err(bad("completion consumed parent changed"));}}
    if s["contract"]!=BRIDGE_COMPLETION_CONTRACT||s["parent_policy"]!=digest(&old)?||s["tape_suffix"]!=digest(&tape)?
        ||*p!=bridge_plan(&old,&own(p).study,&s)?||root!=own(p).study.join(MEAN_ARMS[1])||p.initial!=file_hash(&parent.join(&end.checkpoint))?
        ||p.initial!=file_hash(&root.join("initial.r3m"))?||s["parent_terminal"]!=file_hash(&terminal_path(parent,&end)?)?
        ||s["parent_retention"]!=d["panels"]||p.corpus!=old.corpus||p.transfer!=old.transfer||p.metadata!=old.metadata||p.tokenizer!=old.tokenizer {
        return Err(bad("completion frozen source/parent/data/tape policy"));
    }
    for(name,hash)in[("corpus.r3cor",&p.corpus),("transfer.r3cor",&p.transfer),("metadata.r3b",&p.metadata)]{
        if file_hash(&root.join(name))?!=*hash||file_hash(&parent.join(name))?!=*hash{return Err(bad("completion byte-identical inputs"));}}
    if ByteBpe::load(&root.join("tokenizer.r3b"))?.id()!=p.tokenizer||file_hash(&root.join("tokenizer.r3b"))?!=file_hash(&parent.join("tokenizer.r3b"))?{return Err(bad("completion tokenizer mapping"));}
    if !p.tiny {verify_review_b(&own(&old).study,&comparison(&own(&old).study,&old,&end,&d)?)?;for transfer in [false,true]{bridge_old_qa(&s,transfer)?;}}
    Ok(())
}
fn bridge_parent_parity_cases(p:&Plan)->Result<(Vec<Episode>,Vec<binary::Value>,binary::Value)> {
    let s:binary::Value=read(&own(p).study.join("selection.r3b"))?;let parent=Path::new(s["parent"].as_str().ok_or_else(||bad("completion parity parent"))?);
    let old:Plan=read(&parent.join("plan.r3b"))?;let end:Segment=binary::from_value(s["parent_endpoint"].clone())?;
    bridge_review_cases(parent,&old,end.step,false)
}
fn bridge_parent_parity_verified(root:&Path,p:&Plan)->Result<()> {
    let(es,_,_)=bridge_parent_parity_cases(p)?;let rows=observed(&own(p).study,"bridge-parent-parity",p,&p.initial,&es,true)?;
    if rows.len()!=if p.tiny{24}else{32}||file_hash(&root.join("initial.r3m"))?!=p.initial{return Err(bad("completion parent parity32"));}Ok(())
}
pub(in super::super::super) fn bridge_parent_parity(study:&Path)->Result<()> {
    let study=study.canonicalize()?;let root=study.join(MEAN_ARMS[1]);let p=plan_read(&root)?;
    if !bridge_completion(&p){return Err(bad("completion parent parity scope"));}bridge_completion_verify(&root,&p)?;expansion_review(&p)?;
    let(es,raw,indices)=bridge_parent_parity_cases(&p)?;
    orbit_observe(&study,"bridge-parent-parity",&root.join("initial.r3m"),&es,&binary::record!({"policy":digest(&p)?,"checkpoint":p.initial,"indices":indices}),Some(&raw),observation_control(&p,es.len(),0)?)?;
    bridge_parent_parity_verified(&root,&p)?;println!("QA_BRIDGE_PARENT matched={} optimizer0 teacher0",es.len());Ok(())
}
fn bridge_verify_plan(root:&Path,p:&Plan)->Result<()> {
    if bridge_completion(p){return bridge_completion_verify(root,p);}
    let s:binary::Value=read(&own(p).study.join("selection.r3b"))?;let parent=Path::new(s["parent"].as_str().ok_or_else(||bad("bridge parent path"))?);let old=historical_plan(parent)?;
    let end:Segment=binary::from_value(s["parent_endpoint"].clone())?;
    if s["contract"]!=BRIDGE_CONTRACT||*p!=bridge_plan(&old,&own(p).study,&s)?||root!=own(p).study.join(MEAN_ARMS[1])
        || s["parent_policy"]!=file_hash(&parent.join("plan.r3b"))?||s["parent_terminal"]!=file_hash(&terminal_path(parent,&end)?)?
        || p.initial!=file_hash(&parent.join(&end.checkpoint))?||p.initial!=file_hash(&root.join("initial.r3m"))?{return Err(bad("bridge frozen policy/parent"));}
    let tok=ByteBpe::load(&root.join("tokenizer.r3b"))?;let(c,tm,dm,audit)=bridge_pool(parent,&tok)?;
    let actual=verified_corpus(&root.join("corpus.r3cor"),&p.corpus)?;let(am,ad,ax)=verified_metadata(root,p)?;
    if digest(&(&c.manifest,&c.train,&c.validation))?!=digest(&(&actual.manifest,&actual.train,&actual.validation))?
        || digest(&(&tm,&dm,&dm))?!=digest(&(&am,&ad,&ax))?||audit!=s["audit"]||tok.id()!=p.tokenizer {return Err(bad("bridge exact owned corpus/tape/cost"));}
    if !p.tiny {for transfer in [false,true]{bridge_old_qa(&s,transfer)?;}}
    bridge_probe_usage(p)?;Ok(())
}
pub(super) fn bridge_probe_usage(p:&Plan)->Result<(f64,usize,usize)> {
    if p.tiny||bridge_completion(p){return Ok((0.,0,0));}
    let s:binary::Value=read(&own(p).study.join("selection.r3b"))?;let probe=Path::new(s["probe"].as_str().ok_or_else(||bad("bridge probe path"))?);
    if s["probe_result"]!=file_hash(&probe.join("probe-result.r3b"))?||s["probe_preparation"]!=file_hash(&probe.join("probe-preparation.r3b"))?{return Err(bad("bridge probe identity"));}
    let q=bridge_probe_plan(probe)?;let result:binary::Value=read_confirmed(&probe.join("probe-result.r3b"))?;
    let usage=work(&q)?;if result["needs_training"]!=true||result["usage"]!=binary::record!(usage)||usage.1!=80||usage.2!=0{return Err(bad("bridge probe missing/failed usage"));}
    Ok((usage.0,usage.1,usage.2))
}
fn bridge_authorize(root:&Path,p:&Plan)->Result<()> {
    bridge_verify_plan(root,p)?;expansion_review(p)?;
    let prep:binary::Value=read_confirmed(&own(p).study.join("preparation.r3b"))?;
    if prep["selection"]!=file_hash(&own(p).study.join("selection.r3b"))?{return Err(bad("bridge A2 selection binding"));}
    if bridge_completion(p)&&!p.tiny{bridge_parent_parity_verified(root,p)?;}Ok(())
}
fn bridge_panels(root:&Path,p:&Plan,step:usize)->Result<Vec<Panel>> {
    if !p.evaluation_due(step){return Err(bad("unregistered bridge evaluation"));}
    let c=verified_corpus(&root.join("corpus.r3cor"),&p.corpus)?;let(_,ms,_)=verified_metadata(root,p)?;
    let n=if p.tiny{4}else if full_evaluation(p,step){512}else{64};
    ["value","citation","renamed","S1Q0","S0Q1","S1Q1"].into_iter().enumerate().map(|(i,name)|{
        let es=c.validation.get(i*512..i*512+n).ok_or_else(||bad("missing bridge panel"))?;
        Ok((format!("{name}{n}"),es.to_vec(),ms[i*512..i*512+n].to_vec()))
    }).collect()
}
fn bridge_dev_pass(scores:&BTreeMap<String,binary::Value>)->Result<bool> {
    for name in ["value512","citation512","renamed512","S1Q0512","S0Q1512","S1Q1512"] {
        let s=scores.get(name).ok_or_else(||bad("missing bridge development panel"))?;let j:OrbitScore=binary::from_value(s["joint"].clone())?;
        if !pass(&j,512,488,232,116)||j.swap_both<232{return Ok(false);}
        if name!="value512"&&(s["value_correct"].as_u64().is_none_or(|n|n<508)||s["citation_support_correct"].as_u64().is_none_or(|n|n<508)||s["valid_outside_id"]!=0||s["parse_failure_rows"]!=0){return Ok(false);}
    }Ok(true)
}
fn bridge_decision(root:&Path,p:&Plan,step:usize)->Result<binary::Value> {
    let panels=panels(root,p,step)?;let scores=panels.iter().map(|panel|Ok((panel.0.clone(),read_score(root,p,step,panel)?))).collect::<Result<BTreeMap<_,_>>>()?;
    let models=scores.values().map(|s|s["model"].as_str().ok_or_else(||bad("bridge model identity"))).collect::<Result<BTreeSet<_>>>()?;if models.len()!=1{return Err(bad("mixed bridge endpoint"));}
    let guard=qa_guard(root,p,step,&panels)?;let regression=guard["stop"].is_string();let dev=full_evaluation(p,step)&&bridge_dev_pass(&scores)?;
    let fit_due=!regression&&(dev||(bridge_completion(p)&&step==p.config.max_steps));
    let fit=if fit_due{let n=if p.tiny{4}else{1536};let f=scores.get(&format!("bridge-fit{n}")).ok_or_else(||bad("bridge fit missing"))?;let j:OrbitScore=binary::from_value(f["joint"].clone())?;
        pass(&j,n,if p.tiny{4}else{1524},if p.tiny{2}else{756},if p.tiny{1}else{372})&&f["valid_outside_id"]==0&&f["parse_failure_rows"]==0}else{false};
    let eligible=dev&&fit&&!regression;
    let(action,extend)=if regression{(guard["stop"].as_str().unwrap().to_string(),false)}else if eligible{(format!("CANDIDATE_FIXED_AT_{step}"),false)}else if dev{("BRIDGE_FIT_NOT_MET".into(),false)}else if step==p.config.max_steps{(if bridge_completion(p){format!("FINAL_BRIDGE_QUALITY_FAIL_AT_{step}")}else{"BRIDGE_DEVELOPMENT_FAIL".into()},false)}else{("CONTINUE_WITHIN_REGISTERED_CAP".into(),true)};
    Ok(binary::record!({"policy":digest(p)?,"step":step,"model":models.into_iter().next(),"panels":scores,"guard":guard,"development":dev,"fit":fit,"eligible":eligible,"regression":regression,"action":action,"extend":extend,"stop":(!extend).then_some(action),"S4":"NOT_OPENED","GOAL1_ACCEPTED":false}))
}
pub(in super::super::super) fn bridge_report(study:&Path)->Result<()> {
    let study=study.canonicalize()?;let root=study.join(MEAN_ARMS[1]);let p=historical_plan(&root)?;
    if !instruction_bridge(&p){return Err(bad("bridge report profile"));}let h=history(&root,&p)?;let end=h.last().ok_or_else(||bad("bridge not run"))?;
    for step in p.evaluation.train_steps.iter().copied().filter(|&n|n<=end.step) {
        if step==end.step&&end.phase.as_deref()==Some("EvaluationPending"){continue;}
        let d=bridge_decision(&root,&p,step)?;if d!=read_confirmed::<binary::Value>(&root.join(format!("citation-decision-{step:04}.r3b")))?{return Err(bad("bridge raw/decision mismatch"));}
        for(name,s)in d["panels"].as_object().unwrap(){println!("QA_BRIDGE_PANEL step={step} {name} full={} QB={} SB={} ALL4={} value={} support={} outside={} parse={} EOS={} errors={}",s["joint"]["full"],s["joint"]["query_both"],s["joint"]["swap_both"],s["joint"]["all4"],s["value_correct"],s["citation_support_correct"],s["valid_outside_id"],s["parse_failure_rows"],s["joint"]["eos"],s["joint"]["errors"]);}
    }
    let t=trace(&root,&p,end)?;println!("QA_BRIDGE_TRACE updates={} input={} target={} exposures={} usage={:?} durable={} step={} resume={} stop={}",t["updates"],t["input"],t["target"],t["tasks_V_VC0_VC1_bridge_samples_input_target"],work(&p)?,end.checkpoint_hash,end.step,end.resume,end.stop);
    if !end.resume&&end.phase.as_deref()==Some("Finished"){let(e,d)=close(&root,&p)?;println!("QA_BRIDGE_COMPARISON {}",comparison(&study,&p,&e,&d)?);}Ok(())
}
fn bridge_review_cases(root:&Path,p:&Plan,step:usize,errors:bool)->Result<(Vec<Episode>,Vec<binary::Value>,binary::Value)> {
    let tok=ByteBpe::load(&root.join("tokenizer.r3b"))?;let mut es=vec![];let mut expected=vec![];let mut selected=vec![];
    // Fixed metadata-only coverage is 8/4/4/8/4/4. Error selection is a separate
    // diagnostic of complete failed query pairs and is not an accuracy estimate.
    for (panel_index,(name,cases,ms)) in bridge_panels(root,p,step)?.into_iter().enumerate() {
        let raw=binary::read_value_records(&root.join(format!("eval-{step:04}-{name}.r3rows")))?;
        let normal=if p.tiny{4}else{[8,4,4,8,4,4][panel_index]};
        if raw.len()!=cases.len()+1{return Err(bad("bridge reproduction complete raw"));}
        let mut indices=vec![];
        for start in (0..cases.len()).step_by(2) {
            if ms[start].base!=ms[start+1].base||ms[start].view%2!=0||ms[start+1].view!=ms[start].view+1{return Err(bad("bridge reproduction query pair"));}
            let mut wrong=false;
            for i in start..start+2 {let r=&raw[i+1];verify_generated(r,&tok)?;if r["id"]!=cases[i].id||r["expected"]!=cases[i].answer{return Err(bad("bridge reproduction raw content"));}
                wrong|=!recovery::strict_answer_match(r["actual"].as_str(),&cases[i].answer,r["finish_reason"]=="stop",!r["error"].is_null())||r["generation_completed"]!=true;}
            if if errors{start>=normal&&wrong&&es.len()+indices.len()<32}else{start<normal}{indices.extend([start,start+1]);}
        }
        for &i in &indices {es.push(cases[i].clone());expected.push(raw[i+1].clone());}
        selected.push(binary::record!({"panel":name,"indices":indices,"raw":file_hash(&root.join(format!("eval-{step:04}-{name}.r3rows")))?}));
    }
    if es.len()>32{return Err(bad("bridge reproduction call cap"));}Ok((es,expected,binary::record!(selected)))
}
fn bridge_review_endpoint(root:&Path,p:&Plan)->Result<(Segment,binary::Value)> {
    if !instruction_bridge(p){return Err(bad("bridge reviewer profile"));}let(end,d)=close(root,p)?;
    if !(bridge_completion(p)&&end.stop==format!("FINAL_BRIDGE_QUALITY_FAIL_AT_{}",p.config.max_steps))&&!["BRIDGE_DEVELOPMENT_FAIL","BRIDGE_FIT_NOT_MET","SEVERE_RETENTION_REGRESSION","PERSISTENT_RETENTION_REGRESSION"].contains(&end.stop.as_str())
        && !(d["eligible"]==true&&end.stop==format!("CANDIDATE_FIXED_AT_{}",end.step)){return Err(bad("bridge reviewer complete quality endpoint required"));}Ok((end,d))
}
pub(in super::super::super) fn bridge_review(study:&Path,errors:bool)->Result<()> {
    let study=study.canonicalize()?;let root=study.join(MEAN_ARMS[1]);let p=historical_plan(&root)?;let(end,_)=bridge_review_endpoint(&root,&p)?;
    let(es,raw,indices)=bridge_review_cases(&root,&p,end.step,errors)?;
    if es.is_empty(){println!("QA_BRIDGE_REVIEW errors={errors} no additional failed pairs; generation0");return Ok(());}
    let name=if errors{"bridge-review-errors"}else{"bridge-review-normal"};
    orbit_observe(&study,name,&root.join(&end.checkpoint),&es,&binary::record!({"policy":digest(&p)?,"checkpoint":end.checkpoint_hash,"indices":indices,"posthoc_errors":errors}),Some(&raw),observation_control(&p,es.len(),0)?)?;
    observed(&study,name,&p,&end.checkpoint_hash,&es,true)?;
    println!("QA_BRIDGE_REVIEW errors={errors} matched={} generation={} teacher0 optimizer0",es.len(),es.len());Ok(())
}
fn bridge_old_qa(s:&binary::Value,transfer:bool)->Result<Panel> {
    let path=Path::new(s["old_qa"].as_str().ok_or_else(||bad("bridge original QA path"))?);let name=if transfer{"transfer.r3cor"}else{"corpus.r3cor"};
    let c=verified_corpus(&path.join(name),s["old_qa_hashes"][name].as_str().ok_or_else(||bad("bridge original QA hash"))?)?;
    let bytes=std::fs::read(path.join("metadata.r3b"))?;if s["old_qa_hashes"]["metadata.r3b"]!=neural::hash(&bytes){return Err(bad("bridge original QA metadata"));}
    let(_,dm,xm):(Vec<Meta>,Vec<Meta>,Vec<Meta>)=binary::from_slice(&bytes)?;let ms=if transfer{xm}else{dm};let count=if transfer{128}else{512};
    if c.validation.len()!=count||ms.len()!=count||c.validation.iter().zip(&ms).any(|(e,m)|e.id!=m.id||e.answer.is_empty()||e.request.limits.max_tokens!=128||e.request.limits.timeout_ms!=120000){return Err(bad("bridge unchanged QA128 inputs"));}
    Ok((format!("qa-old_qa-{}",if transfer{"transfer"}else{"primary"}),c.validation,ms))
}
pub(in super::super::super) fn bridge_qa(study:&Path,transfer:bool)->Result<()> {
    let study=study.canonicalize()?;let root=study.join(MEAN_ARMS[1]);let p=plan_read(&root)?;let(end,d)=bridge_review_endpoint(&root,&p)?;
    if d["eligible"]!=true{return Err(bad("bridge development and fit required before QA"));}
    verify_review_b(&study,&comparison(&study,&p,&end,&d)?)?;
    for errors in [false,true]{let(es,_,_)=bridge_review_cases(&root,&p,end.step,errors)?;if !es.is_empty(){observed(&study,if errors{"bridge-review-errors"}else{"bridge-review-normal"},&p,&end.checkpoint_hash,&es,true)?;}}
    let s:binary::Value=read(&study.join("selection.r3b"))?;let panel=bridge_old_qa(&s,transfer)?;let name=if transfer{"bridge-qa-transfer"}else{"bridge-qa-primary"};
    orbit_observe(&study,name,&root.join(&end.checkpoint),&panel.1,&binary::record!({"policy":digest(&p)?,"checkpoint":end.checkpoint_hash,"old_qa":s["old_qa_hashes"],"generation_limit":128}),None,observation_control(&p,panel.1.len(),0)?)?;
    let rows=observed(&study,name,&p,&end.checkpoint_hash,&panel.1,false)?;let tok=ByteBpe::load(&root.join("tokenizer.r3b"))?;
    let score=qa_rows_score(&panel,&rows,&tok,p.tiny,&end.checkpoint_hash,&file_hash(&study.join(format!("{name}.r3rows")))?)?;
    publish_confirmed(&study.join(format!("{name}-score.r3b")),&score)?;println!("QA_BRIDGE_SCOPE {score} S4=false S5=false S6=false GOAL1_ACCEPTED=false");Ok(())
}
fn bridge_diagnostic_admission(p:&Plan,end:&Segment,d:&binary::Value)->Result<()> {
    let at=end.step;let scheduled=if p.tiny{at==p.config.max_steps}else{[p.origin_step()+768,p.origin_step()+1536].contains(&at)};
    let normal=end.stop==format!("CANDIDATE_FIXED_AT_{at}")||end.stop=="BRIDGE_FIT_NOT_MET"||end.stop==format!("FINAL_BRIDGE_QUALITY_FAIL_AT_{}",p.config.max_steps);
    if !bridge_completion(p)||!scheduled||!normal||end.resume||end.phase.as_deref()!=Some("Finished")
        ||d["step"]!=at||d["stop"]!=end.stop||d["action"]!=end.stop||d["extend"]!=false||d["regression"]!=false {
        return Err(bad("DIAGNOSTIC_ONLY requires normal complete registered bridge endpoint"));
    }
    let n=if p.tiny{4}else{512};
    for name in ["value","citation","renamed","S1Q0","S0Q1","S1Q1"] {if d["panels"][&format!("{name}{n}")]["joint"]["total"]!=n{return Err(bad("diagnostic incomplete development panels"));}}
    let n=if p.tiny{4}else{1536};if d["panels"][&format!("bridge-fit{n}")]["joint"]["total"]!=n{return Err(bad("diagnostic incomplete fit"));}
    Ok(())
}
fn bridge_diagnostic_cases(study:&Path,p:&Plan,transfer:bool)->Result<Panel> {
    let s:binary::Value=read(&study.join("selection.r3b"))?;let mut panel=bridge_old_qa(&s,transfer)?;
    if p.tiny {panel.1.truncate(4);panel.2.truncate(4);}Ok(panel)
}
pub(in super::super::super) fn bridge_diagnostic_qa(study:&Path,transfer:bool)->Result<()> {
    let study=study.canonicalize()?;let lock=std::fs::File::open(&study)?;lock.try_lock().map_err(|_|bad("bridge diagnostic already running"))?;
    let root=study.join(MEAN_ARMS[1]);let p=historical_plan(&root)?;let(end,d)=bridge_review_endpoint(&root,&p)?;bridge_diagnostic_admission(&p,&end,&d)?;
    verify_review_b(&study,&comparison(&study,&p,&end,&d)?)?;
    // Reviewer reproduction is required independently of candidate eligibility.
    for errors in [false,true]{let(es,_,_)=bridge_review_cases(&root,&p,end.step,errors)?;if !es.is_empty(){observed(&study,if errors{"bridge-review-errors"}else{"bridge-review-normal"},&p,&end.checkpoint_hash,&es,true)?;}}
    let panel=bridge_diagnostic_cases(&study,&p,transfer)?;let name=if transfer{"bridge-qa-transfer"}else{"bridge-qa-primary"};
    let s:binary::Value=read(&study.join("selection.r3b"))?;let tok=ByteBpe::load(&root.join("tokenizer.r3b"))?;
    let identity=binary::record!({"policy":digest(&p)?,"checkpoint":end.checkpoint_hash,"mode":"DIAGNOSTIC_ONLY","old_qa":s["old_qa_hashes"],
        "generation_limit":128,"generation_count":panel.1.len(),"teacher_limit":0,"context":2048,"candidate_authority":false,"S4_authority":false});
    bridge_diagnostic_output(&study,&root,&p,&end,&panel,&tok,name,&identity,bridge_diagnostic_prior_usage(&p,name)?)
}
fn bridge_diagnostic_prior_usage(p:&Plan,name:&str)->Result<(f64,usize,usize)> {
    let mut total=usage(p)?;
    if own(p).study.join(format!("{name}-started.r3b")).exists() {
        let local=segmented_usage(&own(p).study,name)?;
        total.0-=local.0;total.1=total.1.checked_sub(local.1).ok_or_else(||bad("diagnostic usage underflow"))?;
    }
    if !total.0.is_finite()||total.0<0. {return Err(bad("diagnostic prior usage invalid"));}Ok(total)
}
// This is the shared production output caller, including strict admission,
// scoring and confirmed publication. Process tests reuse historical RETURNED
// journals in an isolated output directory; they never simulate model output.
fn bridge_diagnostic_output(study:&Path,root:&Path,p:&Plan,end:&Segment,panel:&Panel,tok:&ByteBpe,name:&str,identity:&binary::Value,prior:(f64,usize,usize))->Result<()> {
    const IO_BYTES:u64=512*1024*1024;
    let raw=study.join(format!("{name}.r3rows"));
    let mut input_bytes=0u64;let mut entries=0usize;
    for entry in std::fs::read_dir(study)? {
        let entry=entry?;entries+=1;if entries>16_384{return Err(bad("finalization directory bound"));}
        if entry.file_name().to_string_lossy().starts_with(name) {
            let m=std::fs::symlink_metadata(entry.path())?;
            if !m.is_file()||m.file_type().is_symlink(){return Err(bad("finalization input is not a regular file"));}
            input_bytes=input_bytes.checked_add(m.len()).ok_or_else(||bad("finalization input size overflow"))?;
            if input_bytes>IO_BYTES {return Err(bad("finalization input byte bound"));}
        }
    }
    let count=if raw.exists(){binary::read_value_records(&raw)?.len().checked_sub(1).ok_or_else(||bad("QA missing header"))?}else{0};
    let remaining=panel.1.len().checked_sub(count).ok_or_else(||bad("QA extra returned rows"))?;
    let local=if study.join(format!("{name}-started.r3b")).exists(){segmented_usage(study,name)?}else{(0.,0,0)};
    let active=(prior.0+local.0,prior.1+local.1,prior.2);
    if !active.0.is_finite()||active.0<0.||active.1>p.evaluation.generation_limit||active.2>p.evaluation.teacher_limit {
        return Err(bad("finalization usage invalid"));
    }
    let management_start=study.join(format!("{name}-finalization-started.r3b"));
    let management_end=study.join(format!("{name}-finalization.r3b"));
    if (management_start.exists()||pending_path(&management_start).exists()) && !management_end.exists() {
        return Err(bad("finalization interrupted; publication UNKNOWN"));
    }
    if pending_path(&management_end).exists(){return Err(bad("finalization pending publication"));}
    let mut management=None;
    let rows=if remaining==0 {
        let mut control=recovery::RunControl::command(false)?;control.set_call_limits(0,0);
        control.check("qa_finalization_admission")?;
        let binding=binary::record!({"segments":1,"identity":identity,"checkpoint":file_hash(&root.join(&end.checkpoint))?,
            "cases":digest(&panel.1)?,"source":p.source,"binary":p.binary,"decoding":"normal-greedy-strict-utf8-eos"});
        if binding["checkpoint"]!=end.checkpoint_hash {return Err(bad("finalization native identity"));}
        let(rows,elapsed)=segmented_returned(study,name,&binding,&panel.1,tok)?;
        // A bounded cooperative deadline may return just beyond its limit, but
        // unexplained over-cap accounting is not a fresh finalization permission.
        if active.0>p.evaluation.active_seconds as f64 {
            let(_,_,segments)=segmented_usage(study,name)?;
            let last:binary::Value=read_confirmed(&study.join(format!("{name}-segment-{:03}-finished.r3b",segments.checked_sub(1).ok_or_else(||bad("finalization missing segment"))?)))?;
            if last["control"]["terminal_reason"]!="TIME_BUDGET"||last["control"]["observed_conditions"]!=binary::record!(["TIME_BUDGET"]) {
                return Err(bad("finalization unexplained over-cap usage"));
            }
        }
        let final_path=study.join(format!("{name}-finished.r3b"));
        let needs_publication=!final_path.exists()||!study.join(format!("{name}-score.r3b")).exists();
        let final_record=binary::record!({"policy":identity["policy"],"checkpoint":binding["checkpoint"],"binding":binding,
            "completed":rows.len(),"matched":0,"control":{"terminal_reason":"COMPLETED","observed_conditions":[],
            "generation_calls":rows.len(),"teacher_calls":0,"elapsed_seconds":elapsed},
            "generated_tokens":rows.iter().map(|r|r["raw_tokens"].as_array().unwrap().len()).sum::<usize>(),"error":null,"raw":file_hash(&raw)?});
        control.check("qa_finalization_publish")?;
        if needs_publication {
            publish_confirmed(&management_start,&binary::record!({"schema":1,"source":source_digest()?,"binary":file_hash(&std::env::current_exe()?)?,
                "binding":digest(&binding)?,"raw":file_hash(&raw)?,"active_seconds":active.0,"active_cap":p.evaluation.active_seconds,
                "management_seconds_limit":900,"input_bytes":input_bytes,"input_bytes_limit":IO_BYTES,"generation":0,"teacher":0,"optimizer":0}))?;
        }
        if final_path.exists() {if read_confirmed::<binary::Value>(&final_path)?!=final_record{return Err(bad("diagnostic final changed"));}}
        else {publish_confirmed(&final_path,&final_record)?;}
        management=Some((control,active,needs_publication));rows
    }else{
        // Only the original frozen executable can admit more model work. The
        // historical reader above grants no relaxed time or source admission.
        if active.0>=p.evaluation.active_seconds as f64 {return Err(bad("binding observation budget exhausted"));}
        if plan_read(root)?!=*p{return Err(bad("diagnostic current plan changed"));}
        let mut control=observation_control(p,remaining,0)?;
        #[cfg(feature="test-support")]
        if p.tiny&&std::env::var("R3_BRIDGE_QA_STOP").as_deref()==Ok("before"){control.fixture_boundary=Some("confirmation_next_generation".into());}
        segmented_collect(study,name,&root.join(&end.checkpoint),&panel.1,tok,identity,control)?
    };
    observed(study,name,p,&end.checkpoint_hash,&panel.1,false)?;
    let mut score=qa_rows_score(panel,&rows,tok,p.tiny,&end.checkpoint_hash,&file_hash(&raw)?)?;
    score["mode"]=binary::record!("DIAGNOSTIC_ONLY");score["candidate_authority"]=binary::record!(false);
    let path=study.join(format!("{name}-score.r3b"));if path.exists(){if read_confirmed::<binary::Value>(&path)?!=score{return Err(bad("diagnostic score changed"));}}else{publish_confirmed(&path,&score)?;}
    if let Some((mut control,active,needs_publication))=management {
        control.check("qa_finalization_score_durable")?;
        let receipt=study.join(format!("{name}-finalization.r3b"));
        if receipt.exists(){let r:binary::Value=read_confirmed(&receipt)?;
            if r["raw"]!=file_hash(&raw)?||r["final"]!=file_hash(&study.join(format!("{name}-finished.r3b")))?||r["score"]!=file_hash(&path)? {return Err(bad("finalization receipt binding"));}}
        else if needs_publication {publish_confirmed(&receipt,&binary::record!({"schema":1,"mode":"NO_CALL_FINALIZATION","start":file_hash(&management_start)?,"source":source_digest()?,"binary":file_hash(&std::env::current_exe()?)?,
            "producer_source":p.source,"producer_binary":p.binary,"active_seconds":active.0,"active_cap":p.evaluation.active_seconds,
            "management_seconds":control.receipt()["elapsed_seconds"],"management_seconds_limit":900,"input_bytes":input_bytes,"input_bytes_limit":IO_BYTES,
            "input_bytes_kind":"unique observation files; not physical IO","generation":0,"teacher":0,"optimizer":0,
            "raw":file_hash(&raw)?,"final":file_hash(&study.join(format!("{name}-finished.r3b")))?,"score":file_hash(&path)?}))?;}
    }
    println!("QA_BRIDGE_DIAGNOSTIC panel={} full={}/{} EOS={} outside={} parse={} generation_count={} S4=false GOAL1_ACCEPTED=false",panel.0,score["full"],score["total"],score["EOS"],score["outside_id"],score["parse_failure_rows"],rows.len());Ok(())
}
fn qa_tape(ms:&[Meta])->Result<Vec<[usize;8]>> {
    if ms.len()!=3*POOL+2*QA_POOL {return Err(bad("retained QA metadata pool"));}
    let mut r=vec![];
    for group in 0..3 {let mut ids=(0..POOL/2).map(|i|group*POOL+i*2).collect::<Vec<_>>();shuffle(&mut ids,&mut stream(29,&format!("retained/r/{group}")));r.push(ids);}
    let mut q=vec![];
    for b in 0..8 {
        let mut ids=(0..QA_POOL).step_by(2).filter(|&i|ms[3*POOL+i].bucket==b).collect::<Vec<_>>();
        if ids.len()!=512 || ids.iter().any(|&i|ms[3*POOL+i].view!=0 || ms[3*POOL+i+1].view!=1 || ms[3*POOL+i].base!=ms[3*POOL+i+1].base) {return Err(bad("QA complete base pairs"));}
        shuffle(&mut ids,&mut stream(29,&format!("retained/q/{b}")));q.push(ids);
    }
    Ok((0..4096).map(|step| {
        let v=r[0][step%(POOL/2)];let group=1+step%2;let c=r[group][(step/2)%(POOL/2)];
        let b=2*(step%4);let i=(step%2048)/4;let offset=3*POOL+(step/2048)*QA_POOL;
        let a=offset+q[b][i];let z=offset+q[b+1][i];[v,v+1,c,c+1,a,a+1,z,z+1]
    }).collect())
}
fn qa_token_cost(es:&[Episode],rows:&[[usize;8]],tok:&ByteBpe)->Result<binary::Value> {
    data::validate_episodes(es)?;
    let ss=samples(es,tok,512)?;let mut input=0u64;let mut target=0u64;let mut padding=0u64;let mut counts=vec![0usize;es.len()];
    for (e,s) in es.iter().zip(&ss) {
        if ![32,128].contains(&e.request.limits.max_tokens) || e.request.limits.timeout_ms!=120000 {return Err(bad("retained QA original generation limits"));}
        let prompt=tok.prepare_with_framing(&e.request,neural::Framing::QuestionEvidence,2048,"retained-qa-input-check")?;
        if !prompt.excluded.is_empty() || s.tokens.len()>512 || s.tokens[..s.response_start]!=prompt.token_ids {return Err(bad("QA full evidence/train-generation prompt or length"));}
    }
    for row in rows {
        let mut lengths=vec![];
        for &i in row {let s=ss.get(i).ok_or_else(||bad("QA tape index"))?;input+=(s.tokens.len()-1)as u64;target+=(s.tokens.len()-s.response_start)as u64;lengths.push(s.tokens.len()-1);counts[i]+=1;}
        padding+=(lengths.iter().max().unwrap()*8-lengths.iter().sum::<usize>())as u64;
    }
    if input>18_000_000 || target>2_400_000 {return Err(bad("QA planned token budget"));}
    Ok(binary::record!({"input":input,"target":target,"padding":padding,"samples":rows.len()*8,"counts":counts,"maximum_sequence":ss.iter().map(|s|s.tokens.len()).max(),"excluded":0,"prompt_parity":true}))
}
fn qa_sources(balanced:&Path,old_qa:&Path)->Result<(data::native::Corpus,data::native::Corpus,Vec<Meta>,Vec<Meta>,Vec<Meta>,data::native::Corpus,data::native::Corpus,Vec<Meta>,Vec<Meta>)> {
    let q=data::native::read(&balanced.join("corpus.r3cor"))?;let x=data::native::read(&balanced.join("transfer.r3cor"))?;
    let (tm,dm,xm):(Vec<Meta>,Vec<Meta>,Vec<Meta>)=read(&balanced.join("metadata.r3b"))?;
    if q.manifest.generator!="joint-binding-balanced-v1" || q.manifest.seed!=20260921 || q.train.len()!=QA_POOL || q.validation.len()!=512 || x.validation.len()!=128 || digest(&q.train)?!=digest(&x.train)? {return Err(bad("accepted balanced source manifest"));}
    super::super::validate_splits(&[(&q.train,&tm),(&q.validation,&dm),(&x.validation,&xm)])?;
    let old=data::native::read(&old_qa.join("corpus.r3cor"))?;let ox=data::native::read(&old_qa.join("transfer.r3cor"))?;
    let (_,om,oxm):(Vec<Meta>,Vec<Meta>,Vec<Meta>)=read(&old_qa.join("metadata.r3b"))?;
    if old.validation.len()!=512 || ox.validation.len()!=128 || om.len()!=512 || oxm.len()!=128 {return Err(bad("original QA panel counts"));}
    if [&q.train,&q.validation,&x.validation,&old.validation,&ox.validation].into_iter().flatten()
        .any(|e|e.request.limits.max_tokens!=128 || e.request.limits.timeout_ms!=120000) {return Err(bad("QA original 128-token contract"));}
    Ok((q,x,tm,dm,xm,old,ox,om,oxm))
}
fn qa_pool(parent:&Path,balanced:&Path,old_qa:&Path,tok:&ByteBpe)->Result<(data::native::Corpus,Vec<Meta>,Vec<Meta>,binary::Value)> {
    let old=historical_plan(parent)?;let r=verified_corpus(&parent.join("corpus.r3cor"),&old.corpus)?;let (mut rm,rd,_)=verified_metadata(parent,&old)?;
    if r.train.len()!=4608 || r.validation.len()!=1536 {return Err(bad("retention source pool"));}
    if r.train.iter().chain(&r.validation).any(|e|e.request.limits.max_tokens!=32 || e.request.limits.timeout_ms!=120000) {return Err(bad("retention original 32-token contract"));}
    let(q,x,qm,dm,xm,oq,ox,om,oxm)=qa_sources(balanced,old_qa)?;
    let (variant,mut vm)=qa_variant(&q.train,&qm)?;let mut train=r.train;train.extend(q.train.clone());train.extend(variant);
    rm.extend(qm.iter().cloned().map(|mut m| {m.source_id=Some(m.id.clone());m.template=format!("Q0/{}",m.template);m}));rm.append(&mut vm);
    data::validate_episodes(&train)?;
    let mut train_inputs=BTreeSet::new();let mut train_families=BTreeSet::new();let mut train_scenes=BTreeSet::new();
    for e in &train {
        let p=tok.prepare_with_framing(&e.request,neural::Framing::QuestionEvidence,2048,"split-audit")?;
        train_inputs.insert(digest(&p.token_ids)?);train_families.insert(e.family.clone());
        if !e.request.evidence.items.is_empty(){train_scenes.insert(digest(&e.request.evidence)?);}
    }
    for es in [&r.validation,&q.validation,&x.validation,&oq.validation,&ox.validation] {
        qa_token_cost(es,&[],tok)?;
        for e in es {
            let p=tok.prepare_with_framing(&e.request,neural::Framing::QuestionEvidence,2048,"split-audit")?;
            if train_inputs.contains(&digest(&p.token_ids)?) || train_families.contains(&e.family)
                || (!e.request.evidence.items.is_empty() && train_scenes.contains(&digest(&e.request.evidence)?)) {return Err(bad("retained QA heldout leakage"));}
        }
    }
    for (es,ms) in [(&q.validation,&dm),(&x.validation,&xm),(&oq.validation,&om),(&ox.validation,&oxm)] {if es.iter().zip(ms).any(|(e,m)|e.id!=m.id || m.bucket>=8){return Err(bad("QA evaluation metadata identity"));}}
    let rows=qa_tape(&rm)?;let costs=qa_token_cost(&train,&rows,tok)?;
    let manifest=data::CorpusManifest{version:1,scope:"project-owned retained value/citation and educational balanced QA".into(),permission:"synthetic project-owned".into(),generator:QA_DATA.into(),seed:20260921,split_rule:"preserved source splits; Q1 train-only intent-equivalent wording; retention and two QA panels disjoint".into(),train:data::native::split("train",&train),validation:data::native::split("validation",&r.validation)};
    Ok((data::native::from_episodes(manifest,train,r.validation)?,rm,rd,costs))
}
fn qa_plan(old:&Plan,study:&Path,s:&binary::Value,metadata:&[Meta])->Result<Plan> {
    let mut p=old.clone();let state:TrainingState=binary::from_value(s["parent_training_state"].clone())?;
    let get=|k:&str|s[k].as_str().map(str::to_owned).ok_or_else(||bad("retained QA policy field"));
    p.source=get("source")?;p.binary=get("binary")?;p.initial=get("parent_physical")?;p.initial_weights=get("parent_weights")?;
    p.corpus=get("corpus")?;p.transfer=get("transfer")?;p.metadata=get("metadata")?;
    p.config.seq_len=512;p.config.lr=3e-5;p.config.warmup=0;p.config.max_steps=state.step+if p.tiny {2}else{4096};
    p.config.budget_start_step=state.step;p.config.budget_start_tokens=state.consumed_tokens;p.config.max_tokens=state.consumed_tokens+18_000_000;
    let o=p.identifiable.as_mut().ok_or_else(||bad("retained QA parent profile"))?;
    o.study=study.into();o.dataset=QA_DATA.into();o.rows.truncate(state.step);
    if o.rows.len()!=state.step {return Err(bad("retained QA consumed parent tape"));}
    o.rows.extend(qa_tape(metadata)?.into_iter().take(if p.tiny {2}else{4096}));p.train_order=digest(&o.rows)?;
    p.order=vec![(0..3*POOL+2*QA_POOL).collect()];
    let f=p.fork.as_mut().ok_or_else(||bad("retained QA fork"))?;
    f.study=study.into();f.study_hash=file_hash(&study.join("selection.r3b"))?;f.parent_policy=digest(old)?;
    f.parent_state=digest(&state)?;f.parent_adam=get("parent_adam")?;f.origin_step=state.step;f.origin_input=state.consumed_tokens;f.origin_target=state.target_tokens;
    f.original_corpus=p.corpus.clone();f.constant_lr=3e-5;f.target_limit=2_400_000;
    p.evaluation=evaluation(&p);Ok(p)
}
pub(in super::super::super) fn qa_prepare(previous:&Path,balanced:&Path,old_qa:&Path,output:&Path,parent_review:&Path,tiny:bool)->Result<()> {
    if tiny!=cfg!(all(test,feature="test-support")) {return Err(bad("retained QA production/TINY separation"));}
    let previous=previous.canonicalize()?;let balanced=balanced.canonicalize()?;let old_qa=old_qa.canonicalize()?;let output=std::path::absolute(output)?;
    let parent=previous.join(MEAN_ARMS[1]);let old=historical_plan(&parent)?;let(end,decision)=close(&parent,&old)?;
    let l=checkpoint::load(&parent.join(&end.checkpoint),Device::Cpu,true)?;let state=l.manifest.training.as_ref().ok_or_else(||bad("retained QA parent Adam"))?;
    if !precision(&old) || old.tiny!=tiny || (!tiny && decision["eligible"]!=true) || end.resume || end.phase.as_deref()!=Some("Finished")
        || (!tiny && (state.step!=11264 || end.stop!="CANDIDATE_FIXED_AT_11264"))
        || state.sampler_state!=state.step as u64 || state.resume_binding!=Some(old.binding(state,&l.tokenizer)?)
        || old.config.lr!=3e-5 || !answer_mean(&old) || old.config.microbatch!=8 || old.config.accumulation!=1
        || old.framing()!=neural::Framing::QuestionEvidence {return Err(bad("accepted ANSWER11264 parent required"));}
    let accepted=if tiny {binary::record!({"fixture":true})}else{verified_confirmation(&previous,&old,&end,&decision)?};
    if !tiny && accepted["value_citation_baseline_verified"]!=true {return Err(bad("accepted parent citation scope required"));}
    let(c,tm,dm,costs)=qa_pool(&parent,&balanced,&old_qa,&l.tokenizer)?;
    let mut sources=BTreeMap::new();for (label,path) in [("balanced",&balanced),("old_qa",&old_qa)] {for name in ["corpus.r3cor","transfer.r3cor","metadata.r3b"] {sources.insert(format!("{label}/{name}"),file_hash(&path.join(name))?);}}
    let mut s=binary::record!({"contract":QA_CONTRACT,"source":source_digest()?,"binary":file_hash(&std::env::current_exe()?)?,"parent":parent,
        "parent_endpoint":end,"parent_physical":end.checkpoint_hash,"parent_policy":file_hash(&parent.join("plan.r3b"))?,"parent_terminal":file_hash(&terminal_path(&parent,&end)?)?,
        "parent_training_state":state,"parent_adam":optimizer_hash(&l.optimizer)?,"parent_weights":l.model.weight_hash()?,"parent_acceptance":accepted,
        "parent_review":parent_review.canonicalize()?,"parent_review_hash":file_hash(parent_review)?,"balanced":balanced,"old_qa":old_qa,"sources":sources,"planned_costs":costs,
        "historical_resume":false,"new_authorization":QA_CONTRACT,"origin":state.step,"max_steps":state.step+4096,"physical_rows":20992,"Q0_exposures":8192,"Q1_exposures":8192,
        "objective":checkpoint::ANSWER_MEAN_OBJECTIVE,"actual_lr_bits":3e-5f64.to_bits(),"retention_limits":32,"QA_limits":128,"training_length":512,"model_context":2048});
    std::fs::create_dir(&output)?;let root=output.join(MEAN_ARMS[1]);std::fs::create_dir(&root)?;
    data::native::write(&root.join("corpus.r3cor"),&c,true)?;copy_native(&root.join("corpus.r3cor"),&root.join("transfer.r3cor"))?;
    write(&root.join("metadata.r3b"),&(tm.clone(),dm.clone(),dm))?;
    copy_native(&parent.join(&end.checkpoint),&root.join("initial.r3m"))?;copy_native(&parent.join("tokenizer.r3b"),&root.join("tokenizer.r3b"))?;
    for name in ["corpus","transfer","metadata"] {s[name]=binary::record!(file_hash(&root.join(format!("{name}.{}",if name=="metadata" {"r3b"}else{"r3cor"})))?);}
    write(&output.join("selection.r3b"),&s)?;let p=qa_plan(&old,&output,&s,&tm)?;write(&root.join("plan.r3b"),&p)?;qa_verify_plan(&root,&p)?;
    if !p.parent_entry(&root.join("initial.r3m"),&l)? {return Err(bad("retained QA parent entry"));}
    publish_confirmed(&output.join("preparation.r3b"),&binary::record!({"contract":QA_CONTRACT,"source":p.source,"binary":p.binary,"selection":file_hash(&output.join("selection.r3b"))?,
        "arms":{(MEAN_ARMS[1]):{"policy":file_hash(&root.join("plan.r3b"))?,"initial":p.initial,"corpus":p.corpus,"tape":p.train_order}},"optimizer":0,"generation":0,"teacher":0}))?;
    println!("RETAINED_QA_PREPARED rows20992 updates0 generations0 teacher0 costs_input={} target={} padding={} A_PENDING",costs["input"],costs["target"],costs["padding"]);Ok(())
}
fn qa_verify_plan(root:&Path,p:&Plan)->Result<()> {
    let study=&own(p).study;let s:binary::Value=read(&study.join("selection.r3b"))?;
    let path=|k:&str|s[k].as_str().map(Path::new).ok_or_else(||bad("retained QA bound path"));
    let parent=path("parent")?;let old=historical_plan(parent)?;let end:Segment=binary::from_value(s["parent_endpoint"].clone())?;
    let (tm,_,_)=verified_metadata(root,p)?;
    if s["contract"]!=QA_CONTRACT || root!=study.join(MEAN_ARMS[1]) || *p!=qa_plan(&old,study,&s,&tm)?
        || s["parent_terminal"]!=file_hash(&terminal_path(parent,&end)?)? || digest(&read_confirmed::<Segment>(&terminal_path(parent,&end)?)?)?!=digest(&end)?
        || s["parent_policy"]!=file_hash(&parent.join("plan.r3b"))? || s["parent_review_hash"]!=file_hash(path("parent_review")?)?
        || p.initial!=file_hash(&parent.join(&end.checkpoint))? || p.initial!=file_hash(&root.join("initial.r3m"))? {return Err(bad("retained QA immutable lineage/policy"));}
    for label in ["balanced","old_qa"] {for name in ["corpus.r3cor","transfer.r3cor","metadata.r3b"] {
        if s["sources"][&format!("{label}/{name}")]!=file_hash(&path(label)?.join(name))? {return Err(bad("retained QA frozen source changed"));}
    }}
    let tok=ByteBpe::load(&root.join("tokenizer.r3b"))?;let c=verified_corpus(&root.join("corpus.r3cor"),&p.corpus)?;
    if tok.id()!=p.tokenizer || c.train.len()!=20992 || p.config.seq_len!=512 || p.config.lr!=3e-5 || p.framing()!=neural::Framing::QuestionEvidence {return Err(bad("retained QA input configuration"));}
    Ok(())
}
fn qa_bound_panel(p:&Plan,label:&str,transfer:bool)->Result<(Vec<Episode>,Vec<Meta>)> {
    let s:binary::Value=read(&own(p).study.join("selection.r3b"))?;
    let dir=Path::new(s[label].as_str().ok_or_else(||bad("QA source path"))?);
    let name=if transfer {"transfer.r3cor"}else{"corpus.r3cor"};
    let expected=s["sources"][&format!("{label}/{name}")].as_str().ok_or_else(||bad("QA source digest"))?;
    let c=verified_corpus(&dir.join(name),expected)?;
    let bytes=std::fs::read(dir.join("metadata.r3b"))?;
    if s["sources"][&format!("{label}/metadata.r3b")]!=neural::hash(&bytes){return Err(bad("QA metadata changed"));}
    let(_,dm,xm):(Vec<Meta>,Vec<Meta>,Vec<Meta>)=binary::from_slice(&bytes)?;
    Ok((c.validation,if transfer {xm}else{dm}))
}
fn qa_base_panels(root:&Path,p:&Plan,step:usize)->Result<Vec<Panel>> {
    if !p.evaluation_due(step){return Err(bad("retained QA unregistered evaluation"));}
    let c=verified_corpus(&root.join("corpus.r3cor"),&p.corpus)?;let(tm,dm,_)=verified_metadata(root,p)?;
    let full=full_evaluation(p,step);let n=if p.tiny {4}else if full {512}else{64};
    let mut out=[("value",0),("citation",512),("renamed",1024)].into_iter().map(|(name,start)|(format!("{name}{n}"),c.validation[start..start+n].to_vec(),dm[start..start+n].to_vec())).collect::<Vec<_>>();
    for label in ["old_qa","balanced"] {
        let(es,ms)=qa_bound_panel(p,label,false)?;
        let(es,ms)=if full {(es,ms)}else{subset(&es,&ms,if p.tiny {2}else{8})};
        out.push((format!("qa-{label}-primary"),es,ms));
        if full {let(es,ms)=qa_bound_panel(p,label,true)?;out.push((format!("qa-{label}-transfer"),es,ms));}
    }
    if full {
        let offset=3*POOL+if step-p.origin_step()>2048 {QA_POOL}else{0};
        let(es,ms)=subset(&c.train[offset..offset+QA_POOL],&tm[offset..offset+QA_POOL],16);
        let seen=own(p).rows[p.origin_step()..step].iter().flatten().copied().collect::<BTreeSet<_>>();
        if ms.iter().any(|m|tm.iter().position(|x|x.id==m.id).is_none_or(|i|!seen.contains(&i))){return Err(bad("QA train diagnostic contains unexposed variant"));}
        out.push(("qa-train128".into(),es,ms));
    }
    Ok(out)
}
fn qa_score(root:&Path,p:&Plan,step:usize,panel:&Panel)->Result<binary::Value> {
    let(name,es,ms)=panel;let tok=ByteBpe::load(&root.join("tokenizer.r3b"))?;
    let summary=audit_panel(root,p,step,name,es,ms,&tok)?;
    let raw=binary::read_value_records(&root.join(format!("eval-{step:04}-{name}.r3rows")))?;
    qa_rows_score(panel,&raw[1..],&tok,p.tiny,&summary.model,&summary.raw_hash)
}
fn qa_rows_score(panel:&Panel,rows:&[binary::Value],tok:&ByteBpe,tiny:bool,model:&str,raw_hash:&str)->Result<binary::Value> {
    let(name,es,ms)=panel;
    if rows.len()!=es.len()||ms.len()!=es.len(){return Err(bad("QA score complete panel"));}
    for ((e,m),r)in es.iter().zip(ms).zip(rows){if m.id!=e.id||r["id"]!=e.id||r["expected"]!=e.answer{return Err(bad("QA score content binding"));}verify_generated(r,tok)?;}
    let mut groups=vec![[0usize;8];8];let mut exact=vec![];let(mut eos,mut errors,mut length,mut runtime,mut utf8,mut outside)=(0,0,0,0,0,0);let mut g_types=BTreeMap::<String,[usize;2]>::new();
    for ((e,m),r) in es.iter().zip(ms).zip(rows) {
        let good=recovery::strict_answer_match(r["actual"].as_str(),&e.answer,r["finish_reason"]=="stop",!r["error"].is_null())&&r["generation_completed"]==true;
        exact.push(good);let stop=r["finish_reason"]=="stop";let err=!r["error"].is_null();
        eos+=usize::from(stop);errors+=usize::from(!stop||err);length+=usize::from(r["finish_reason"]=="length");
        utf8+=usize::from(r["error_class"]=="strict_utf8");runtime+=usize::from(err&&r["error_class"]!="strict_utf8");
        let expected=citations(&e.answer)?;let ids=r["actual"].as_str().and_then(|s|citations(s).ok());
        let external=has_outside_id(e,r["actual"].as_str());outside+=usize::from(external);
        let g=&mut groups[m.bucket];g[0]+=1;g[1]+=usize::from(good);g[2]+=usize::from(!stop||err);
        g[3]+=usize::from(!expected.is_empty() && ids.as_ref()==Some(&expected));g[4]+=usize::from(external);
        g[5]+=usize::from(ids.as_ref().is_some_and(|ids|!ids.is_empty()&&!external&&*ids!=expected));g[6]+=usize::from(ids.is_none());g[7]+=usize::from(ids.as_ref().is_some_and(Vec::is_empty));
        if m.bucket==6 {let key=e.answer.clone();let row=g_types.entry(key).or_default();row[0]+=1;row[1]+=usize::from(good);}
    }
    let mut pairs=vec![[0usize;2];8];let mut resolutions=BTreeMap::<String,[usize;4]>::new();
    let mut old_relations=vec![[0usize;10];8];let mut old_all4=vec![[0usize;2];8];
    if name.contains("balanced") || name=="qa-train128" {
        for(i,mm)in ms.chunks_exact(2).enumerate(){if mm[0].base!=mm[1].base || mm[0].view!=0 || mm[1].view!=1 || mm[0].bucket!=mm[1].bucket {return Err(bad("QA complementary panel order"));}let g=&mut pairs[mm[0].bucket];g[0]+=1;g[1]+=usize::from(exact[2*i]&&exact[2*i+1]);
            if mm[0].bucket==6 {let r=resolutions.entry(es[2*i].answer.clone()).or_default();r[0]+=1;r[1]+=usize::from(exact[2*i]);r[2]+=usize::from(exact[2*i+1]);r[3]+=usize::from(exact[2*i]&&exact[2*i+1]);}
        }
    }else{
        let mut bases=BTreeMap::<&str,Vec<usize>>::new();for(i,m)in ms.iter().enumerate(){bases.entry(&m.base).or_default().push(i);}
        for ix in bases.values(){if (!tiny&&ix.len()!=4)||![2,4].contains(&ix.len())||ix.iter().enumerate().any(|(j,&i)|ms[i].view!=j||ms[i].bucket!=ms[ix[0]].bucket){return Err(bad("old QA four-view identity"));}
            let at=ix[0];let g=&mut old_relations[ms[at].bucket];g[0]+=1;
            for j in 1..ix.len(){g[2*j-1]+=1;g[2*j]+=usize::from(exact[at]&&exact[ix[j]]);}
            g[7]+=usize::from(es[at].answer!=es[ix[1]].answer);
            if ix.len()==4 {g[8]+=usize::from(es[at].request.evidence!=es[ix[2]].request.evidence);g[9]+=usize::from(es[at].request.input!=es[ix[3]].request.input);
                let a=&mut old_all4[ms[at].bucket];a[0]+=1;a[1]+=usize::from(ix.iter().all(|&i|exact[i]));}
        }
    }
    let fixed=es.iter().zip(ms).filter(|(_,m)|m.bucket==7).map(|(e,_)|e.answer.as_str()).collect::<BTreeSet<_>>();
    let mut h=[0usize;3];let mut non_h=[0usize;2];
    for ((r,m),good) in rows.iter().zip(ms).zip(&exact) {let emission=r["actual"].as_str().is_some_and(|s|fixed.contains(s));if m.bucket==7 {h[0]+=1;h[1]+=usize::from(emission);h[2]+=usize::from(*good);}else{non_h[0]+=1;non_h[1]+=usize::from(emission);}}
    let parse_failures=groups.iter().map(|g|g[6]).sum::<usize>();
    let mut out=binary::record!({"scorer":QA_SCORER,"parse_failure_rows":parse_failures,"model":model,"raw":raw_hash,"cases":digest(es)?,"total":es.len(),"full":exact.iter().filter(|x|**x).count(),"exact":exact,
        "EOS":eos,"errors":errors,"length":length,"runtime_errors":runtime,"UTF8_errors":utf8,"outside_id":outside,"buckets":groups,"QUERY_BOTH_planned_correct":pairs,"G_subtypes_planned_correct":g_types,
        "G_resolution_planned_unresolved_resolved_both":resolutions,"H_fixed_planned_emitted_strict":h,"non_H_fixed_planned_emitted":non_h,
        "bucket_fields":["planned","full","errors","exact_nonempty_support","outside_id","wrong_provided_id","parse_failure","empty_citation"],"ALL4":"NOT_DEFINED","scope":"old four views or balanced complementary two views; strict complete answer"});
    if name.contains("old_qa") {out["QUERY_BOTH_planned_correct"]=binary::record!("NOT_DEFINED");out["ALL4"]=binary::record!(old_all4);
        out["old_view_pairs"]=binary::record!(old_relations);out["old_view_pair_fields"]=binary::record!(["bases","value01_planned","value01_both","order02_planned","order02_both","wording03_planned","wording03_both","changed_gold01","changed_evidence02","changed_query03"]);}
    Ok(out)
}
// Historical metrics are immutable. This command validates their raw/native
// bindings, checks every unchanged field, then publishes a separate correction.
pub(in super::super::super) fn qa_recount(study:&Path,output:&Path)->Result<()> {
    if output.exists(){return Err(bad("recount output already exists"));}
    let study=study.canonicalize()?;let root=study.join(MEAN_ARMS[1]);let p=historical_plan(&root)?;
    if !retained_qa(&p){return Err(bad("historical QA recount profile"));}
    let ends=history(&root,&p)?;let end=ends.last().ok_or_else(||bad("no historical endpoint"))?;
    if end.phase.as_deref()!=Some("Finished"){return Err(bad("historical endpoint incomplete"));}
    let mut protected=BTreeMap::<PathBuf,String>::new();
    for name in ["plan.r3b","corpus.r3cor","metadata.r3b","tokenizer.r3b"] {
        let path=root.join(name);protected.insert(path.clone(),file_hash(&path)?);
    }
    let selection=own(&p).study.join("selection.r3b");protected.insert(selection.clone(),file_hash(&selection)?);
    let mut results=vec![];let(mut generated,mut qa_rows,mut corrected_rows)=(0,0,0);
    for step in p.evaluation.train_steps.iter().copied().filter(|&s|s<=end.step) {
        let path=root.join(format!("citation-decision-{step:04}.r3b"));
        protected.insert(path.clone(),file_hash(&path)?);
        let old:binary::Value=read_confirmed(&path)?;
        if old["policy"]!=digest(&p)? || old["step"]!=step {return Err(bad("historical decision binding"));}
        for panel in qa_base_panels(&root,&p,step)? {
            let (name,es,ms)=&panel;
            let rawpath=root.join(format!("eval-{step:04}-{name}.r3rows"));
            for suffix in [".r3rows","-teachers.r3rows",".r3b"] {
                let path=root.join(format!("eval-{step:04}-{name}{suffix}"));protected.insert(path.clone(),file_hash(&path)?);
            }
            let raw=binary::read_value_records(&rawpath)?;
            let native=PathBuf::from(raw[0]["checkpoint"].as_str().ok_or_else(||bad("historical native path"))?);
            protected.insert(native.clone(),file_hash(&native)?);
            let new=read_score(&root,&p,step,&panel)?;let mut legacy=new.clone();let mut rows=vec![];
            if name.starts_with("qa-") {
                qa_rows+=es.len();let mut counted=0;let mut buckets=vec![0usize;8];
                for(i,((e,m),r))in es.iter().zip(ms).zip(&raw[1..]).enumerate() {
                    let text=r["actual"].as_str();let strict=text.and_then(|s|citations(s).ok());
                    let ids=text.map(individually_valid_ids).unwrap_or_default();
                    let external=has_outside_id(e,text);let prior=strict.as_ref().is_some_and(|ids|ids.iter().any(|id|!e.request.evidence.items.iter().any(|v|v.event_id==*id)));
                    counted+=usize::from(prior);buckets[m.bucket]+=usize::from(prior);corrected_rows+=usize::from(external!=prior);
                    rows.push(binary::record!({"ordinal":i,"id":e.id,"bucket":m.bucket,"strict_ids":strict,"individually_valid_ids":ids,
                        "outside_row":external,"previous_outside_row":prior,"parse_failure":strict.is_none(),"empty_citation":strict.as_ref().is_some_and(Vec::is_empty)}));
                }
                let map=legacy.as_object_mut().ok_or_else(||bad("historical score object"))?;map.remove("scorer");map.remove("parse_failure_rows");
                legacy["outside_id"]=binary::record!(counted);for(b,n)in buckets.into_iter().enumerate(){legacy["buckets"][b][4]=binary::record!(n);}
            }
            if old["panels"][name]!=legacy{return Err(bad("historical score differs beyond the explicit citation correction"));}
            generated+=es.len();
            results.push(binary::record!({"step":step,"panel":name,"decision":protected[&path],"raw":protected[&rawpath],"physical":protected[&native],"previous":old["panels"][name],"corrected":new,"citation_rows":rows}));
        }
    }
    for(path,hash)in &protected{if file_hash(path)?!=*hash{return Err(bad("historical input changed during recount"));}}
    let report=binary::record!({"mode":"READ_ONLY_HISTORICAL_REANALYSIS","scorer":QA_SCORER,"source":source_digest()?,"binary":file_hash(&std::env::current_exe()?)?,
        "original_source":p.source,"original_binary":p.binary,"policy":digest(&p)?,"protected":protected,"panels":results,"generated_rows":generated,"qa_rows":qa_rows,
        "corrected_outside_rows":corrected_rows,"new_optimizer":0,"new_generation":0,"new_teacher":0,"historical_candidate_promotion":"NOT_AUTHORIZED","last_step":end.step,"original_stop":end.stop});
    publish_confirmed(output,&report)?;
    println!("QA_RECOUNT raw={generated} QA={qa_rows} corrected_outside_rows={corrected_rows} new_model_calls0 output={}",output.display());Ok(())
}
fn qa_dev_pass(scores:&BTreeMap<String,binary::Value>)->Result<bool> {
    for label in ["old_qa","balanced"] {for split in ["primary","transfer"] {
        let key=format!("qa-{label}-{split}");let s=scores.get(&key).ok_or_else(||bad("missing full QA panel"))?;
        let (total,min)=if split=="primary"{(512,487)}else{(128,116)};
        if s["scorer"]!=QA_SCORER || s["parse_failure_rows"]!=0 || s["total"]!=total || s["full"].as_u64().is_none_or(|n|n<min) || s["EOS"]!=total || s["errors"]!=0 || s["outside_id"]!=0 {return Ok(false);}
        if split=="primary" {
            for b in 0..8 {if s["buckets"][b][0]!=64 || s["buckets"][b][1].as_u64().is_none_or(|n|n<58){return Ok(false);}}
            if label=="balanced" {for b in 2..6 {if s["QUERY_BOTH_planned_correct"][b][0]!=32 || s["QUERY_BOTH_planned_correct"][b][1].as_u64().is_none_or(|n|n<29){return Ok(false);}}}
        }
    }}Ok(true)
}
fn qa_guard(root:&Path,p:&Plan,step:usize,panels:&[Panel])->Result<binary::Value> {
    let tok=ByteBpe::load(&root.join("tokenizer.r3b"))?;let mut before=[0usize;3];let mut previous=None;
    for at in p.evaluation.train_steps.iter().copied().filter(|&n|n<=step) {
        let cases=if at==step {panels.to_vec()}else{base_panels(root,p,at)?};let mut screens=vec![];let mut hashes=vec![];let mut after=[0usize;3];let mut stop=None;
        for (i,panel) in cases[..3].iter().enumerate() {
            let path=root.join(format!("eval-{at:04}-{}.r3rows",panel.0));let raw=binary::read_value_records(&path)?;let n=if p.tiny{4}else{64};
            let score=if i==0 {orbit_score(&panel.1[..n],&panel.2[..n],&raw[1..n+1],&tok)?}else{score_citation(&panel.1[..n],&panel.2[..n],&raw[1..n+1],&tok)?.joint};
            let(a,s)=if p.tiny{(0,None)}else{retention_decision(score.full,score.all4,score.errors,before[i])};after[i]=a;if s.is_some(){stop=s;}
            screens.push(score);hashes.push(file_hash(&path)?);
        }
        let g=binary::record!({"before":before,"after":after,"screens":screens,"raw":hashes,"previous":previous,"stop":stop,"origin_counts_as_warning":false});
        if at==step{return Ok(g);}
        let path=root.join(format!("citation-decision-{at:04}.r3b"));let d:binary::Value=read_confirmed(&path)?;
        if d["policy"]!=digest(p)? || d["step"]!=at || d["guard"]!=g || stop.is_some(){return Err(bad("retained QA guard chain mismatch or prior stop"));}
        before=after;previous=Some(file_hash(&path)?);
    }Err(bad("retained QA guard unregistered step"))
}
fn qa_decision(root:&Path,p:&Plan,step:usize)->Result<binary::Value> {
    let cases=panels(root,p,step)?;let scores=cases.iter().map(|panel|Ok((panel.0.clone(),read_score(root,p,step,panel)?))).collect::<Result<BTreeMap<_,_>>>()?;
    let models=scores.values().map(|s|s["model"].as_str().ok_or_else(||bad("QA model identity"))).collect::<Result<BTreeSet<_>>>()?;
    if models.len()!=1{return Err(bad("mixed QA endpoint models"));}
    let guard=qa_guard(root,p,step,&cases)?;let regression=guard["stop"].is_string();
    let dev=full_evaluation(p,step)&&dev_pass(&scores)?&&qa_dev_pass(&scores)?;
    let mut fits=dev;
    if dev {for(name,total,full,qb,all4)in[("old512",512,508,252,124),("new1024",1024,1016,504,248),("VC0train1536",1536,1524,756,372),("VC1train1536",1536,1524,756,372)] {
        let s:OrbitScore=binary::from_value(scores.get(name).ok_or_else(||bad("QA conditional fit missing"))?["joint"].clone())?;fits&=pass(&s,total,full,qb,all4);
    }}
    let eligible=dev&&fits&&!regression;let(action,extend)=if regression{(guard["stop"].as_str().unwrap().to_owned(),false)}else if eligible{(format!("CANDIDATE_FIXED_AT_{step}"),false)}else if step==p.config.max_steps{("STUDY_COMPLETE_QUALITY_FAIL".into(),false)}else{("CONTINUE_WITHIN_REGISTERED_CAP".into(),true)};
    Ok(binary::record!({"policy":digest(p)?,"step":step,"model":models.into_iter().next(),"panels":scores,"guard":guard,"development":dev,"fit":fits,"eligible":eligible,"action":action,"extend":extend,"stop":(!extend).then_some(action)}))
}
fn qa_parent_cases(root:&Path,p:&Plan,name:&str)->Result<(Vec<Episode>,Option<Vec<binary::Value>>)> {
    if name=="balanced" || name=="transfer" {
        let (es,ms)=qa_bound_panel(p,"balanced",name=="transfer")?;
        return Ok((if p.tiny{subset(&es,&ms,2).0}else{es},None));
    }
    if !["value","citation"].contains(&name){return Err(bad("retained QA parent panel"));}
    let c=verified_corpus(&root.join("corpus.r3cor"),&p.corpus)?;let start=if name=="value"{0}else{512};let n=if p.tiny{4}else{16};
    let s:binary::Value=read(&own(p).study.join("selection.r3b"))?;let parent=Path::new(s["parent"].as_str().ok_or_else(||bad("QA parent path"))?);
    let raw=binary::read_value_records(&parent.join(format!("eval-{:04}-{name}{}.r3rows",p.origin_step(),if p.tiny{4}else{512})))?;
    Ok((c.validation[start..start+n].to_vec(),Some(raw[1..n+1].to_vec())))
}
fn qa_authorize(root:&Path,p:&Plan)->Result<()> {
    qa_verify_plan(root,p)?;
    for panel in ["value","citation","balanced","transfer"] {
        let(es,expected)=qa_parent_cases(root,p,panel)?;
        observed(&own(p).study,&format!("qa-parent-{panel}"),p,&p.initial,&es,expected.is_some())?;
    }
    if !p.tiny {
        let r:binary::Value=read_confirmed(&own(p).study.join("review-a.r3b"))?;
        let path=Path::new(r["s4_seal_path"].as_str().ok_or_else(||bad("S4 independent seal missing"))?);
        if r["s4_seal_hash"]!=file_hash(path)? {return Err(bad("S4 independent seal changed"));}
        let seal:binary::Value=read(path)?;
        if seal["count"]!=200 || seal["status"]!="SEALED" || seal["opened_for_model"]!=false {return Err(bad("S4 must be prepared before learning"));}
        for (location,hash) in [("fixture_path","fixture_sha256"),("controls_path","controls_sha256")] {
            let file=Path::new(seal[location].as_str().ok_or_else(||bad("S4 canonical file missing"))?);
            if seal[hash]!=file_hash(file)? {return Err(bad("S4 sealed file changed"));}
        }
    }
    Ok(())
}
pub(in super::super::super) fn qa_parent(study:&Path,name:&str)->Result<()> {
    let study=study.canonicalize()?;let root=study.join(MEAN_ARMS[1]);let p=plan_read(&root)?;
    if !retained_qa(&p){return Err(bad("QA observation profile"));}expansion_review(&p)?;
    let(es,expected)=qa_parent_cases(&root,&p,name)?;let count=es.len();
    orbit_observe(&study,&format!("qa-parent-{name}"),&root.join("initial.r3m"),&es,
        &binary::record!({"policy":digest(&p)?,"checkpoint":p.initial}),expected.as_deref(),observation_control(&p,count,0)?)?;
    let rows=observed(&study,&format!("qa-parent-{name}"),&p,&p.initial,&es,expected.is_some())?;
    let exact=es.iter().zip(&rows).filter(|(e,r)|recovery::strict_answer_match(r["actual"].as_str(),&e.answer,r["finish_reason"]=="stop",!r["error"].is_null())).count();
    println!("QA_PARENT panel={name} full={exact}/{count} generation={count} teacher0 optimizer0");Ok(())
}
pub(in super::super::super) fn qa_report(study:&Path)->Result<()> {
    let study=study.canonicalize()?;let root=study.join(MEAN_ARMS[1]);let p=historical_plan(&root)?;
    if !retained_qa(&p){return Err(bad("retained QA report scope"));}
    let h=history(&root,&p)?;let Some(end)=h.last() else {println!("QA_STUDY NOT_RUN");return Ok(());};
    for step in p.evaluation.train_steps.iter().copied().filter(|&s|s<=end.step) {
        if step==end.step && end.phase.as_deref()==Some("EvaluationPending"){continue;}
        let d=qa_decision(&root,&p,step)?;
        if read_confirmed::<binary::Value>(&root.join(format!("citation-decision-{step:04}.r3b")))?!=d {return Err(bad("QA report/raw decision mismatch"));}
        for panel in qa_base_panels(&root,&p,step)? {let s=&d["panels"][&panel.0];println!("QA_PANEL step={step} panel={} score={}",panel.0,if panel.0.starts_with("qa-"){binary::record!({"full":s["full"],"total":s["total"],"EOS":s["EOS"],"length":s["length"],"errors":s["errors"],"runtime":s["runtime_errors"],"UTF8":s["UTF8_errors"],"outside":s["outside_id"],"buckets":s["buckets"],"QUERY_BOTH":s["QUERY_BOTH_planned_correct"],"ALL4":s["ALL4"],"old_view_pairs":s["old_view_pairs"],"G_resolution":s["G_resolution_planned_unresolved_resolved_both"],"H_fixed":s["H_fixed_planned_emitted_strict"],"non_H_fixed":s["non_H_fixed_planned_emitted"]})}else{binary::record!({"full":s["joint"]["full"],"total":s["joint"]["total"],"QB":s["joint"]["query_both"],"ALL4":s["joint"]["all4"],"support":s["citation_support_correct"],"outside":s["valid_outside_id"]})});}
    }
    let t=trace(&root,&p,end)?;
    println!("QA_TRACE updates={} input={} target={} exposure={} unique={} usage={:?} durable={} step={} resume={} stop={} S4=NOT_ACCEPTED S5=NOT_ACCEPTED S6=NOT_ACCEPTED Goal1=false",t["updates"],t["input"],t["target"],t["tasks_V_VC0_VC1_Q0_Q1_samples_input_target"],t["unique_V_VC0_VC1_Q0_Q1"],work(&p)?,end.checkpoint_hash,end.step,end.resume,end.stop);
    if !end.resume&&end.phase.as_deref()==Some("Finished"){let(e,d)=close(&root,&p)?;println!("QA_COMPARISON {}",comparison(&study,&p,&e,&d)?);}
    Ok(())
}
pub(in super::super::super) fn qa_review(root:&Path,label:&str)->Result<()> {
    if !["old_qa","balanced"].contains(&label){return Err(bad("QA reviewer panel"));}
    let root=root.canonicalize()?;let p=historical_plan(&root)?;
    if !retained_qa(&p){return Err(bad("QA reviewer profile"));}
    let(end,_)=close(&root,&p)?;
    if !["STUDY_COMPLETE_QUALITY_FAIL","SEVERE_RETENTION_REGRESSION","PERSISTENT_RETENTION_REGRESSION"].contains(&end.stop.as_str())
        && end.stop!=format!("CANDIDATE_FIXED_AT_{}",end.step) {return Err(bad("QA reviewer requires complete quality endpoint"));}
    let panel=qa_base_panels(&root,&p,end.step)?.into_iter().find(|v|v.0==format!("qa-{label}-primary")).ok_or_else(||bad("QA review raw missing"))?;
    let scored=qa_score(&root,&p,end.step,&panel)?;
    let raw=binary::read_value_records(&root.join(format!("eval-{:04}-{}.r3rows",end.step,panel.0)))?;
    // Exactly two frozen rows per bucket, wrong rows first; evaluation-only.
    let mut indices=vec![];for bucket in 0..8 {
        let mut candidates=panel.2.iter().enumerate().filter(|(_,m)|m.bucket==bucket).map(|(i,_)|i).collect::<Vec<_>>();
        candidates.sort_by_key(|&i|(scored["exact"][i]==true,i));
        if candidates.len()<2{return Err(bad("QA review bucket incomplete"));}indices.extend_from_slice(&candidates[..2]);
    }
    let es=indices.iter().map(|&i|panel.1[i].clone()).collect::<Vec<_>>();let expected=indices.iter().map(|&i|raw[i+1].clone()).collect::<Vec<_>>();
    orbit_observe(&own(&p).study,&format!("qa-review-{label}"),&root.join(&end.checkpoint),&es,
        &binary::record!({"policy":digest(&p)?,"checkpoint":end.checkpoint_hash,"sample_indices":indices}),Some(&expected),observation_control(&p,16,0)?)?;
    println!("QA_REVIEW panel={label} generation16 teacher0 optimizer0 matched16 candidate_permission=false");Ok(())
}
