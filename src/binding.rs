//! Training-only finite selection study. Reuses fresh plans, native corpora,
//! the decoder/trainer, immutable call receipts and the strict panel scorer.
use super::*;
const DATA: &str = "binding-learnability-v1";
const CONTRACT_ID: &str = "R3-BINDING-LEARNABILITY-1.0";
const ARMS: [&str; 4] = ["A", "B", "C", "D"];
const ORBIT_DATA: &str = "foundation-orbit-v1";
const ORBIT_CONTRACT: &str = "R3-FOUNDATION-ORBIT-1.0";
const ORBIT_ARMS: [&str; 2] = ["FIXED", "BOTH"];
const SHORT_SYSTEM: &str = "근거에 따라 답하라.";
const VALUE_QUERY: &str = "현재 값의 숫자 하나만 답하라.";
const CITATION_QUERY: &str = "현재 값을 인용과 함께 써라.";
type Skeleton = [u8; 4];

pub(in super::super) fn is(p: &Plan) -> bool {
    p.identifiable.as_ref().is_some_and(|x| x.dataset == DATA || x.dataset == ORBIT_DATA)
}
fn is_orbit(p: &Plan) -> bool { p.identifiable.as_ref().is_some_and(|x| x.dataset == ORBIT_DATA) }
fn arms(p: &Plan) -> &'static [&'static str] { if is_orbit(p) { &ORBIT_ARMS } else { &ARMS } }
pub(in super::super) fn evaluation_for(p: &Plan) -> EvaluationPolicy {
    let mut e = evaluation(p.tiny);
    if is_orbit(p) { e.teacher_limit = 3072; e.primary_min = 488; }
    e
}
fn own(p: &Plan) -> &Policy {
    p.identifiable.as_ref().expect("binding policy")
}
fn canonical(mut s: Skeleton) -> Skeleton {
    if s[0] > s[1] {
        s.swap(0, 1);
    }
    if s[2] > s[3] {
        s.swap(2, 3);
    }
    s
}
fn orbit(s: Skeleton) -> BTreeSet<Skeleton> {
    (0..10)
        .map(|n| canonical(s.map(|v| (v + n) % 10)))
        .collect()
}
fn rank(s: Skeleton) -> String {
    neural::hash(format!("17/binding/skeleton/{s:?}").as_bytes())
}
fn universe() -> Vec<Skeleton> {
    let mut out = vec![];
    for a in 0..10 {
        for b in a + 1..10 {
            for c in 0..10 {
                for d in c + 1..10 {
                    if ![a, b].contains(&c) && ![a, b].contains(&d) {
                        out.push([a, b, c, d]);
                    }
                }
            }
        }
    }
    out
}
fn skeletons() -> Vec<Skeleton> {
    let reserved = orbit([0, 1, 3, 4]);
    let mut reps = BTreeSet::new();
    for s in universe() {
        let o = orbit(s);
        if o.len() == 10 && o.is_disjoint(&reserved) {
            reps.insert(*o.first().unwrap());
        }
    }
    let mut reps: Vec<_> = reps.into_iter().collect();
    reps.sort_by_key(|&s| rank(s));
    let mut out = vec![];
    for s in reps.into_iter().take(12) {
        out.extend(orbit(s));
    }
    for n in 0..10 {
        if n != 0 && n != 5 {
            out.push(canonical([n, (n + 1) % 10, (n + 3) % 10, (n + 4) % 10]));
        }
    }
    out.sort_by_key(|&s| rank(s));
    out
}
fn names(seed: u64, scenes: &[Skeleton]) -> Result<Vec<String>> {
    let mut rng = stream(seed, "independent-name-digits");
    let mut codes = vec![];
    for _ in 0..5 {
        let mut digits: Vec<_> = (b'0'..=b'9').collect();
        shuffle(&mut digits, &mut rng);
        let prefix = &digits[..4];
        codes.push(String::from_utf8([prefix, prefix].concat()).map_err(|_| bad("name bytes"))?);
        codes.push(String::from_utf8(digits[..8].to_vec()).map_err(|_| bad("name bytes"))?);
    }
    if codes.iter().collect::<BTreeSet<_>>().len() != 10 {
        return Err(bad("finite name collision"));
    }
    let mut counts = [0; 10];
    for s in scenes {
        counts[s[0] as usize] += 1;
        counts[s[1] as usize] += 1;
    }
    let mut low: Vec<_> = (0..10).filter(|&i| counts[i] == 25).collect();
    let mut high: Vec<_> = (0..10).filter(|&i| counts[i] == 26).collect();
    shuffle(&mut low, &mut rng);
    shuffle(&mut high, &mut rng);
    if low.len() != 4 || high.len() != 6 {
        return Err(bad("key marginal balance"));
    }
    let order = [
        low[0], high[0], high[1], low[1], low[2], high[2], high[3], low[3], high[4], high[5],
    ];
    let mut out = vec![String::new(); 10];
    for (code, key) in codes.into_iter().zip(order) {
        out[key] = code;
    }
    Ok(out)
}
fn resolve_request(q: &ModelRequest) -> Result<String> {
    let mut words = q.input.splitn(3, ' ');
    let entity = words.next().ok_or_else(|| bad("binding query entity"))?;
    let context = words.next().ok_or_else(|| bad("binding query context"))?;
    let mode = words.next().ok_or_else(|| bad("binding query mode"))?;
    if ![VALUE_QUERY, CITATION_QUERY].contains(&mode) {
        return Err(bad("binding output request"));
    }
    let matches = q
        .evidence
        .items
        .iter()
        .map(|r| Ok((r, parsed_record(r)?)))
        .collect::<Result<Vec<_>>>()?
        .into_iter()
        .filter(|(_, (e, c, _))| *e == entity && *c == context)
        .collect::<Vec<_>>();
    if matches.len() != 1 {
        return Err(bad("binding query must identify one record"));
    }
    let (r, (_, _, v)) = matches[0];
    Ok(if mode == VALUE_QUERY {
        v.into()
    } else {
        format!("{v}입니다. [event:{}]", r.event_id)
    })
}
fn generate(arm: &str, split: usize, mapping: &[String]) -> Result<(Vec<Episode>, Vec<Meta>)> {
    let selected = skeletons();
    let mut scenes = if split < 2 {
        selected.clone()
    } else {
        let mut v = universe()
            .into_iter()
            .filter(|s| !selected.contains(s))
            .collect::<Vec<_>>();
        v.sort_by_key(|&s| rank(s));
        v.truncate(64);
        v
    };
    let mut order = vec![false; scenes.len()];
    for x in order.iter_mut().skip(scenes.len() / 2) {
        *x = true;
    }
    shuffle(&mut order, &mut stream(17 ^ split as u64, "physical-order"));
    let assignment = balanced_factor(scenes.len(), 2, 17, "value-assignment");
    let mut ids = stream(17 ^ split as u64, "event-ids");
    let mut used = BTreeSet::new();
    let mut es = vec![];
    let mut ms = vec![];
    for (i, s) in scenes.drain(..).enumerate() {
        let keys = [s[0], s[1]];
        let values = if (assignment[i] ^ usize::from(split == 1)) == 0 {
            [s[2], s[3]]
        } else {
            [s[3], s[2]]
        };
        let names = keys.map(|k| {
            format!(
                "장치{}",
                if ["B", "D"].contains(&arm) {
                    mapping[k as usize].clone()
                } else {
                    k.to_string()
                }
            )
        });
        let event = [
            10_000_000 + (ids.next_u64() % 90_000_000) as i64,
            10_000_000 + (ids.next_u64() % 90_000_000) as i64,
        ];
        if event.iter().any(|&id| !used.insert(id)) {
            return Err(bad("finite event collision"));
        }
        let mut records = (0..2)
            .map(|j| {
                let mut r = record(
                    &names[j],
                    "구역0",
                    &values[j].to_string(),
                    event[j],
                    0,
                    "current",
                );
                r.source = "s".into();
                r.recorded_at = 0;
                r
            })
            .collect::<Vec<_>>();
        if order[i] {
            records.reverse();
        }
        let base = format!("{DATA}/{split}/{i}");
        for view in 0..2 {
            let id = format!("{base}/{view}");
            let q = ModelRequest {
                request_id: String::new(),
                system: SHORT_SYSTEM.into(),
                input: format!(
                    "{} 구역0 {}",
                    names[view],
                    if ["A", "B"].contains(&arm) {
                        VALUE_QUERY
                    } else {
                        CITATION_QUERY
                    }
                ),
                evidence: EvidenceBundle {
                    items: records.clone(),
                    ..Default::default()
                },
                limits: GenerationLimits {
                    context_tokens: 2048,
                    max_tokens: 32,
                    timeout_ms: 120_000,
                },
            };
            let answer = if ["A", "B"].contains(&arm) {
                values[view].to_string()
            } else {
                format!("{}입니다. [event:{}]", values[view], event[view])
            };
            if resolve_request(&q)? != answer {
                return Err(bad("independent binding label"));
            }
            es.push(Episode {
                id: id.clone(),
                category: 0,
                family: base.clone(),
                binding: format!("{:?}", [(keys[0], values[0]), (keys[1], values[1])]),
                sequence: base.clone(),
                request: q,
                answer,
            });
            ms.push(Meta {
                id,
                base: base.clone(),
                template: if ["A", "B"].contains(&arm) { "V" } else { "VC" }.into(),
                bucket: 0,
                view,
                split: split.to_string(),
                entities: names.to_vec(),
                source_id: None,
                query_context: Some("구역0".into()),
            });
        }
    }
    Ok((es, ms))
}
fn validate(es: &[Episode], ms: &[Meta], tok: &ByteBpe, count: usize) -> Result<binary::Value> {
    if es.len() != count || ms.len() != count || count % 2 != 0 {
        return Err(bad("binding panel count"));
    }
    let framed = samples(es, tok, 256)?;
    let mut scene_ids = BTreeSet::new();
    let mut max_len = 0;
    let mut key_counts = BTreeMap::new();
    for (pair, meta) in es.chunks_exact(2).zip(ms.chunks_exact(2)) {
        if pair[0].request.evidence != pair[1].request.evidence
            || pair[0].answer == pair[1].answer
            || meta[0].base != meta[1].base
            || meta[0].view != 0
            || meta[1].view != 1
        {
            return Err(bad("binding complementary views"));
        }
        let mut scene = pair[0]
            .request
            .evidence
            .items
            .iter()
            .map(|r| {
                let (e, _, v) = parsed_record(r)?;
                Ok((e.to_owned(), v.to_owned()))
            })
            .collect::<Result<Vec<_>>>()?;
        scene.sort();
        if !scene_ids.insert(digest(&scene)?) {
            return Err(bad("duplicate canonical scene"));
        }
        for (e, _) in scene {
            bump(&mut key_counts, e);
        }
        let mut fixed = pair[1].request.clone();
        fixed.input = pair[0].request.input.clone();
        if resolve_request(&fixed)? == pair[1].answer {
            return Err(bad("question-ignore negative control"));
        }
    }
    for ((e, m), s) in es.iter().zip(ms).zip(&framed) {
        if m.bucket != 0
            || m.id != e.id
            || resolve_request(&e.request)? != e.answer
            || !e.request.request_id.is_empty()
            || e.request.system != SHORT_SYSTEM
        {
            return Err(bad("binding owned request metadata"));
        }
        let p = tok.prepare(&e.request, 2048, "binding-prepare")?;
        if p.provided.len() != 2
            || !p.excluded.is_empty()
            || p.token_ids != s.tokens[..s.response_start]
            || s.tokens.len() > 256
            || s.tokens.last() != Some(&EOS)
        {
            return Err(bad("BLOCKED_FIXTURE: binding framing/length"));
        }
        let actual = tok.decode(&s.tokens[s.response_start..s.tokens.len() - 1])?;
        if actual != e.answer {
            return Err(bad("answer roundtrip"));
        }
        let first = tok.encode(&e.answer.as_bytes()[..1])?;
        if first.len() != 1 || first[0] != s.tokens[s.response_start] {
            return Err(bad("framed first digit token"));
        }
        max_len = max_len.max(s.tokens.len());
    }
    Ok(
        binary::record!({"rows":count,"bases":scene_ids.len(),"scenes":scene_ids,"key_frequency":key_counts,"max_prompt_target_eos":max_len,"provided":2,"excluded":0,"label_integrity":"VERIFIED","query_necessary":true}),
    )
}
fn tape(steps: usize) -> Vec<[usize; 8]> {
    let mut out = vec![];
    for epoch in 0..steps.div_ceil(32) {
        let mut bases = (0..128).collect::<Vec<_>>();
        shuffle(&mut bases, &mut stream(17 ^ epoch as u64, "paired-tape"));
        for chunk in bases.chunks_exact(4) {
            let mut row = [0; 8];
            for (j, b) in chunk.iter().enumerate() {
                row[j * 2] = b * 2;
                row[j * 2 + 1] = b * 2 + 1;
            }
            out.push(row);
        }
    }
    out.truncate(steps);
    out
}
fn config(tiny: bool) -> TrainConfig {
    TrainConfig {
        lr: 3e-4,
        warmup: if tiny { 2 } else { 32 },
        max_steps: if tiny { 2 } else { 512 },
        max_tokens: 5_000_000,
        seq_len: 256,
        microbatch: 8,
        accumulation: 1,
        validate_every: 128,
        seed: 17,
        ..Default::default()
    }
}
fn architecture(vocab: usize, tiny: bool) -> Config {
    let mut c = if tiny {
        Config::tiny(vocab)
    } else {
        Config::small(vocab)
    };
    if tiny {
        c.context = 2048;
        c.window = 256;
        c.profile = "NATIVE_TRPP_EXPERIMENTAL_V1".into();
    }
    c
}
fn native_corpus(train: Vec<Episode>, dev: Vec<Episode>) -> Result<data::native::Corpus> {
    let manifest=data::CorpusManifest{version:1,scope:"project-owned synthetic selection diagnostic".into(),permission:"synthetic project-owned".into(),generator:DATA.into(),seed:17,split_rule:"canonical sorted key/value scene; opposite assignments in disjoint full scenes; individual edges/digits and K8 names shared".into(),train:data::native::split("train",&train),validation:data::native::split("validation",&dev)};
    data::native::from_episodes(manifest, train, dev)
}
pub(in super::super) fn evaluation(tiny: bool) -> EvaluationPolicy {
    let steps = if tiny {
        vec![0, 2]
    } else {
        vec![0, 128, 256, 512]
    };
    EvaluationPolicy {
        screen_steps: steps.clone(),
        train_steps: steps,
        primary_steps: vec![],
        transfer_steps: vec![],
        teacher_steps: vec![],
        generation_limit: 4096,
        teacher_limit: 4096,
        active_seconds: 7200,
        primary_min: 244,
        bucket_min: 0,
        transfer_min: 0,
        ..Default::default()
    }
}
pub(in super::super) fn prepare(parent: &Path, output: &Path, tiny: bool) -> Result<()> {
    if cfg!(feature = "test-support") && !tiny {
        return Err(bad("production binding binary required"));
    }
    let parent_plan: Plan = read(&parent.join("plan.r3b"))?;
    let tok = ByteBpe::load(&parent.join("tokenizer.r3b"))?;
    if tok.id() != parent_plan.tokenizer {
        return Err(bad("tokenizer parent identity"));
    }
    for d in b'0'..=b'9' {
        let t = tok.encode(&[d])?;
        if t.len() != 1 || tok.decode(&t)?.as_bytes() != [d] {
            return Err(bad("ASCII digit token"));
        }
    }
    let sc = skeletons();
    let mapping = names(17, &sc)?;
    let new_names = names(18, &sc)?;
    if mapping.iter().any(|x| new_names.contains(x)) {
        return Err(bad("new name overlap"));
    }
    std::fs::create_dir(output)?;
    let c = architecture(tok.vocab_size(), tiny);
    let model = Transformer::init(c, 17, Device::Cpu)?;
    #[cfg(test)]
    if tiny {
        numeric_token_fixture(&model, EOS)?;
    }
    let adam = Adam::new(&model.vars)?;
    if adam.moments.values().any(|t| {
        t.flatten_all()
            .and_then(|v| v.to_vec1::<f32>())
            .map_or(true, |v| v.iter().any(|&v| v.to_bits() != 0))
    }) {
        return Err(bad("fresh Adam"));
    }
    let mut arms = BTreeMap::new();
    let rows = tape(config(tiny).max_steps);
    for arm in ARMS {
        let root = output.join(arm);
        std::fs::create_dir(&root)?;
        let (train, tm) = generate(arm, 0, &mapping)?;
        let (dev, dm) = generate(arm, 1, &mapping)?;
        let (probe, xm) = generate(arm, 2, &new_names)?;
        let tr = validate(&train, &tm, &tok, 256)?;
        let dv = validate(&dev, &dm, &tok, 256)?;
        let xp = validate(&probe, &xm, &tok, 128)?;
        if tr["key_frequency"] != dv["key_frequency"] {
            return Err(bad("train/dev name frequency"));
        }
        let a: BTreeSet<String> = binary::from_value(tr["scenes"].clone())?;
        let b: BTreeSet<String> = binary::from_value(dv["scenes"].clone())?;
        if !a.is_disjoint(&b) {
            return Err(bad("canonical scene split leakage"));
        }
        let corpus = native_corpus(train.clone(), dev.clone())?;
        let x = native_corpus(train.clone(), probe)?;
        data::native::write(&root.join("corpus.r3cor"), &corpus, true)?;
        data::native::write(&root.join("transfer.r3cor"), &x, true)?;
        let native = data::native::read(&root.join("corpus.r3cor"))?;
        if digest(&native.train)? != digest(&train)? || digest(&native.validation)? != digest(&dev)?
        {
            return Err(bad("native owned episode roundtrip"));
        }
        tok.save(&root.join("tokenizer.r3b"))?;
        write(&root.join("metadata.r3b"), &(tm, dm, xm))?;
        let manifest = checkpoint::initialized(&model, &tok, 17, source_digest()?)?;
        checkpoint::save(
            &root.join("initial.r3m"),
            &model,
            &tok,
            manifest,
            &BTreeMap::new(),
        )?;
        let loaded = checkpoint::load(&root.join("initial.r3m"), Device::Cpu, false)?;
        if loaded.model.weights_content_id()? != model.weights_content_id()?
            || loaded.manifest.training.is_some()
        {
            return Err(bad("initial native identity"));
        }
        let plan = Plan {
            identifiable: Some(Policy {
                study: output.to_owned(),
                arm: arm.into(),
                dataset: DATA.into(),
                rows: rows.clone(),
            }),
            schema: Some(2),
            fork: None,
            paired: None,
            training_values: None,
            grounding: None,
            revision: REVISION.into(),
            source: source_digest()?,
            binary: file_hash(&std::env::current_exe()?)?,
            corpus: file_hash(&root.join("corpus.r3cor"))?,
            transfer: file_hash(&root.join("transfer.r3cor"))?,
            tokenizer: tok.id(),
            initial: file_hash(&root.join("initial.r3m"))?,
            initial_weights: model.weight_hash()?,
            config: config(tiny),
            architecture: model.config.clone(),
            sampler: "query-pair-tape-v1".into(),
            order: vec![(0..256).collect()],
            train_order: digest(&rows)?,
            split_policy: corpus.manifest.split_rule.clone(),
            data_seed: 17,
            model_seed: 17,
            tiny,
            metadata: file_hash(&root.join("metadata.r3b"))?,
            evaluation: evaluation(tiny),
        };
        write(&root.join("plan.r3b"), &plan)?;
        verify_plan(&root, &plan)?;
        arms.insert(arm,binary::record!({"policy":file_hash(&root.join("plan.r3b"))?,"initial":plan.initial,"weights":model.weights_content_id()?,"architecture":model.config.semantic_id()?,"corpus":plan.corpus,"transfer":plan.transfer,"metadata":plan.metadata,"train":tr,"dev":dv,"new_name_probe":xp}));
    }
    publish_confirmed(
        &output.join("preparation.r3b"),
        &binary::record!({"contract":CONTRACT_ID,"dataset":DATA,"source":source_digest()?,"binary":file_hash(&std::env::current_exe()?)?,"arms":arms,"key_mapping":mapping,"new_key_mapping":new_names,
        "skeletons":sc,"universe_scenes":2520,"tape":digest(&rows)?,"tokenizer":tok.id(),"parent_policy":file_hash(&parent.join("plan.r3b"))?,"parameters":model.config.parameters(),"tiny_numeric_tensor_fixture":tiny&&cfg!(test),"adam_zero_hash":optimizer_hash(&adam.moments)?,"adam_clock":0,"new_optimizer":0,"generation":0,"teacher":0,"final200":"NOT_CREATED_NOT_IN_SCOPE"}),
    )?;
    println!(
        "BINDING_PREPARED arms=4 train=256 dev=256 SMALL_UPDATES=0 GENERATION=0 TEACHER=0 review=PENDING"
    );
    Ok(())
}
pub(in super::super) fn verify_plan(root: &Path, p: &Plan) -> Result<()> {
    if is_orbit(p) { return orbit_verify_plan(root, p); }
    let o = own(p);
    if o.dataset != DATA
        || !ARMS.contains(&o.arm.as_str())
        || root != o.study.join(&o.arm)
        || p.config != config(p.tiny)
        || p.model_seed != 17
        || p.data_seed != 17
        || p.fork.is_some()
        || p.paired.is_some()
        || p.grounding.is_some()
        || p.training_values.is_some()
        || p.order != vec![(0..256).collect::<Vec<_>>()]
        || o.rows != tape(p.config.max_steps)
        || p.architecture != architecture(p.architecture.vocab, p.tiny)
    {
        return Err(bad("binding policy/tape scope"));
    }
    let tok = ByteBpe::load(&root.join("tokenizer.r3b"))?;
    if tok.id() != p.tokenizer {
        return Err(bad("binding tokenizer"));
    }
    let c = verified_corpus(&root.join("corpus.r3cor"), &p.corpus)?;
    let x = verified_corpus(&root.join("transfer.r3cor"), &p.transfer)?;
    let (tm, dm, xm) = verified_metadata(root, p)?;
    validate(&c.train, &tm, &tok, 256)?;
    validate(&c.validation, &dm, &tok, 256)?;
    validate(&x.validation, &xm, &tok, 128)?;
    let mapping = names(17, &skeletons())?;
    let probe_names = names(18, &skeletons())?;
    for (split, es, ms, mapping) in [
        (0, &c.train, &tm, &mapping),
        (1, &c.validation, &dm, &mapping),
        (2, &x.validation, &xm, &probe_names),
    ] {
        let expected = generate(&o.arm, split, mapping)?;
        if digest(&(es, ms))? != digest(&expected)? {
            return Err(bad("frozen binding scene/label/metadata mismatch"));
        }
    }
    if digest(&x.train)? != digest(&c.train)? {
        return Err(bad("probe train pool mismatch"));
    }
    Ok(())
}
fn panel_cases(
    root: &Path,
    p: &Plan,
    step: usize,
) -> Result<Vec<(String, Vec<Episode>, Vec<Meta>)>> {
    let c = verified_corpus(&root.join("corpus.r3cor"), &p.corpus)?;
    let (tm, dm, _) = verified_metadata(root, p)?;
    let nt = if is_orbit(p) {
        if p.tiny { 4 } else if step == 512 { 512 } else { 64 }
    } else if p.tiny {
        4
    } else if step == 512 {
        256
    } else {
        32
    };
    let nd = if is_orbit(p) {
        if p.tiny { 4 } else if step == 512 { 512 } else { 64 }
    } else if p.tiny {
        4
    } else if step == 512 {
        256
    } else {
        64
    };
    Ok(vec![
        (
            format!("train{nt}"),
            c.train[..nt].to_vec(),
            tm[..nt].to_vec(),
        ),
        (
            format!("dev{nd}"),
            c.validation[..nd].to_vec(),
            dm[..nd].to_vec(),
        ),
    ])
}
pub(in super::super) fn evaluate(
    p: &Plan,
    root: &Path,
    path: &Path,
    step: usize,
    control: &mut recovery::RunControl,
) -> Result<Option<String>> {
    for (name, es, ms) in panel_cases(root, p, step)? {
        evaluate_panel(p, root, path, step, &name, &es, &ms, control)?;
    }
    Ok(None)
}
pub(in super::super) fn audit(
    root: &Path,
    p: &Plan,
    first: usize,
    last: usize,
) -> Result<(usize, BTreeMap<String, PanelResult>)> {
    let tok = ByteBpe::load(&root.join("tokenizer.r3b"))?;
    let mut count = 0;
    let mut out = BTreeMap::new();
    for &step in p
        .evaluation
        .train_steps
        .iter()
        .filter(|&&s| s >= first && s <= last)
    {
        for (name, es, ms) in panel_cases(root, p, step)? {
            let s = audit_panel(root, p, step, &name, &es, &ms, &tok)?;
            count += es.len();
            out.insert(format!("eval-{step:04}-{name}"), s);
        }
    }
    Ok((count, out))
}
fn gate(root: &Path, p: &Plan) -> Result<bool> {
    if is_orbit(p) { return orbit_gate(root, p); }
    if p.tiny {
        return Ok(false);
    }
    let h = history(root, p)?;
    let Some(last) = h.last() else {
        return Ok(false);
    };
    if last.step != p.config.max_steps || last.phase.as_deref() != Some("Finished") || last.resume {
        return Ok(false);
    }
    let (_, rs) = audit(root, p, last.step, last.step)?;
    let both = |s: &PanelResult| s.paired_both.map(|x| x.iter().sum::<usize>()).unwrap_or(0);
    let train = &rs["eval-0512-train256"];
    let dev = &rs["eval-0512-dev256"];
    Ok(train.exact >= 254
        && both(train) >= 126
        && dev.exact >= 244
        && both(dev) >= 116
        && train.errors == 0
        && dev.errors == 0)
}
pub(in super::super) fn authorize(root: &Path, p: &Plan) -> Result<()> {
    let study = &own(p).study;
    let prep: binary::Value = read_confirmed(&study.join("preparation.r3b"))?;
    let review: binary::Value = read_confirmed(&study.join("review-a.r3b"))?;
    if review["verdict"] != "PASS"
        || review["preparation"] != file_hash(&study.join("preparation.r3b"))?
        || review["source"] != p.source
        || review["report_hash"]
            != file_hash(Path::new(
                review["report_path"]
                    .as_str()
                    .ok_or_else(|| bad("review path"))?,
            ))?
        || prep["source"] != p.source
        || prep["binary"] != p.binary
        || prep["arms"][own(p).arm.as_str()]["policy"] != file_hash(&root.join("plan.r3b"))?
    {
        return Err(bad("independent binding review required"));
    }
    for &arm in arms(p) {
        let r = study.join(arm);
        let plan = plan_read(&r)?;
        let h = history(&r, &plan)?;
        if h.last().is_some_and(|s| {
            !s.resume && (s.step != plan.config.max_steps || s.phase.as_deref() != Some("Finished"))
        }) {
            return Err(bad("binding failed peer"));
        }
    }
    if is_orbit(p) {
        return orbit_authorize(root, p);
    }
    if own(p).arm != "A" {
        let a = study.join("A");
        let ap = plan_read(&a)?;
        if !gate(&a, &ap)? || !parity_verified(&a, &ap)? {
            return Err(bad("NOT_RUN_PREREQUISITE: A positive control"));
        }
    }
    Ok(())
}
fn work(p: &Plan) -> Result<(f64, usize, usize, u64, u64)> {
    let mut out = (0., 0, 0, 0u64, 0u64);
    for &arm in arms(p) {
        let root = own(p).study.join(arm);
        for i in 0..128 {
            let end = root.join(format!("segment-{i:04}-finished.r3b"));
            if !end.exists() {
                break;
            }
            let s: Segment = read_confirmed(&end)?;
            let c: binary::Value = read(&root.join(format!("segment-{i:04}/train-control.r3b")))?;
            out.0 += s.elapsed;
            out.1 += s.generations;
            out.2 += s.teachers;
            out.3 += c["executed_input_tokens_including_uncommitted"]
                .as_u64()
                .ok_or_else(|| bad("input UNKNOWN"))?;
            out.4 += c["executed_target_tokens_including_uncommitted"]
                .as_u64()
                .ok_or_else(|| bad("target UNKNOWN"))?;
        }
        for observation in ["parity", "probe"] {
            if root.join(format!("{observation}-started.r3b")).exists() {
                let r: binary::Value =
                    read_confirmed(&root.join(format!("{observation}-finished.r3b")))
                        .map_err(|_| bad("observation usage UNKNOWN"))?;
                out.0 += r["control"]["elapsed_seconds"]
                    .as_f64()
                    .ok_or_else(|| bad("parity time UNKNOWN"))?;
                out.1 += r["control"]["generation_calls"]
                    .as_u64()
                    .ok_or_else(|| bad("parity calls UNKNOWN"))? as usize;
                out.2 += r["control"]["teacher_calls"]
                    .as_u64()
                    .ok_or_else(|| bad("observation teachers UNKNOWN"))?
                    as usize;
                if !r["error"].is_null() {
                    return Err(bad("failed observation remains sticky"));
                }
            }
        }
    }
    if is_orbit(p) {
        for name in ["swap", "confirmation", "review-FIXED", "review-BOTH"] {
            let study = &own(p).study;
            if study.join(format!("{name}-started.r3b")).exists() {
                let r: binary::Value = read_confirmed(&study.join(format!("{name}-finished.r3b")))
                    .map_err(|_| bad("orbit observation usage UNKNOWN"))?;
                out.0 += r["control"]["elapsed_seconds"].as_f64().ok_or_else(||bad("orbit time UNKNOWN"))?;
                out.1 += r["control"]["generation_calls"].as_u64().ok_or_else(||bad("orbit calls UNKNOWN"))? as usize;
                out.2 += r["control"]["teacher_calls"].as_u64().ok_or_else(||bad("orbit teacher UNKNOWN"))? as usize;
                if !r["error"].is_null() || r["control"]["terminal_reason"] != "COMPLETED" {
                    return Err(bad("orbit failed observation remains sticky"));
                }
            }
        }
    }
    Ok(out)
}
pub(in super::super) fn usage(p: &Plan) -> Result<(f64, usize, usize)> {
    let w = work(p)?;
    Ok((w.0, w.1, w.2))
}
pub(in super::super) fn remaining(p: &Plan, target: bool) -> Result<u64> {
    let w = work(p)?;
    let (cap, n) = if is_orbit(p) {
        if target { (20_000u64, w.4) } else { (2_000_000u64, w.3) }
    } else if target {
        (1_000_000u64, w.4)
    } else {
        (5_000_000u64, w.3)
    };
    cap.checked_sub(n)
        .ok_or_else(|| bad("binding token budget exhausted"))
}
fn parity_verified(root: &Path, p: &Plan) -> Result<bool> {
    let r: binary::Value = read_confirmed(&root.join("parity-finished.r3b"))?;
    let h = history(root, p)?;
    let Some(last) = h.last() else {
        return Ok(false);
    };
    Ok(r["matched"] == 16
        && r["policy"] == digest(p)?
        && r["checkpoint"] == last.checkpoint_hash
        && r["error"].is_null()
        && r["raw"] == file_hash(&root.join("parity.r3rows"))?
        && r["control"]["generation_calls"] == 16
        && r["control"]["teacher_calls"] == 0
        && r["control"]["terminal_reason"] == "COMPLETED")
}
fn observation_control(
    p: &Plan,
    generations: usize,
    teachers: usize,
) -> Result<recovery::RunControl> {
    let (elapsed, g, t) = usage(p)?;
    if elapsed >= p.evaluation.active_seconds as f64
        || g + generations > p.evaluation.generation_limit
        || t + teachers > p.evaluation.teacher_limit
    {
        return Err(bad("binding observation budget exhausted"));
    }
    let cancel = std::sync::Arc::new(AtomicBool::new(false));
    let signal = cancel.clone();
    ctrlc::set_handler(move || signal.store(true, Ordering::Relaxed))
        .map_err(|e| bad(&e.to_string()))?;
    let mut control = recovery::RunControl::new(
        cancel,
        std::time::Duration::from_secs_f64(
            (p.evaluation.active_seconds as f64 - elapsed).min(p.evaluation.segment_seconds as f64),
        ),
        16 * 1024 * 1024,
    )?;
    control.set_call_limits(generations, teachers);
    Ok(control)
}
pub(in super::super) fn parity(root: &Path) -> Result<()> {
    let p = plan_read(root)?;
    if is_orbit(&p) { return orbit_parity(root, &p, false); }
    if !is(&p) || own(&p).arm != "A" || p.tiny {
        return Err(bad("SMALL A parity only"));
    }
    authorize(root, &p)?;
    let h = history(root, &p)?;
    let last = h.last().ok_or_else(|| bad("parity endpoint missing"))?;
    if last.step != p.config.max_steps || last.phase.as_deref() != Some("Finished") {
        return Err(bad("parity requires final endpoint"));
    }
    let c = verified_corpus(&root.join("corpus.r3cor"), &p.corpus)?;
    let (_, dm, _) = verified_metadata(root, &p)?;
    let path = root.join(&last.checkpoint);
    let l = checkpoint::load(&path, Device::Cpu, false)?;
    audit_panel(root, &p, 512, "dev256", &c.validation, &dm, &l.tokenizer)?;
    let old = binary::read_value_records(&root.join("eval-0512-dev256.r3rows"))?;
    let binding = binary::record!({"policy":digest(&p)?,"checkpoint":last.checkpoint_hash,"cases":digest(&&c.validation[..16])?});
    let mut control = observation_control(&p, 16, 0)?;
    write(&root.join("parity-started.r3b"), &binding)?;
    let mut f = std::fs::OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(root.join("parity.r3rows"))?;
    let mut matched = 0;
    let result = (|| -> Result<()> {
        for (i, e) in c.validation[..16].iter().enumerate() {
            let attempt = prepare_call(root, "parity16", "generation", &binding, e, i)?;
            let row = match recovery::observe_generation(&l, e, &e.request, &mut control, false) {
                recovery::ObservedCall::Returned(r) => r,
                recovery::ObservedCall::NotInvoked(_) => {
                    resolve_call(&attempt, None, &mut control)?;
                    return Err(bad("parity not invoked"));
                }
            };
            append_row(&mut f, &row)?;
            resolve_call(&attempt, Some(&row), &mut control)?;
            verify_generated(&row, &l.tokenizer)?;
            for key in [
                "raw_tokens",
                "actual",
                "error",
                "finish_reason",
                "generation_completed",
            ] {
                if row[key] != old[i + 1][key] {
                    return Err(bad("fresh process output parity"));
                }
            }
            matched += 1;
            control.stop_result()?;
        }
        Ok(())
    })();
    if let Err(e) = &result {
        control.classify_error(e);
    }
    let result = result.and(control.seal_terminal());
    publish_confirmed(
        &root.join("parity-finished.r3b"),
        &binary::record!({"policy":digest(&p)?,"checkpoint":last.checkpoint_hash,"matched":matched,"control":control.receipt(),"error":result.as_ref().err().map(ToString::to_string),"raw":file_hash(&root.join("parity.r3rows"))?}),
    )?;
    println!("BINDING_PARITY matched={matched}/16 optimizer=0 teacher=0");
    result
}
pub(in super::super) fn probe(root: &Path) -> Result<()> {
    let p = plan_read(root)?;
    if !is(&p) || p.tiny || !["B", "D"].contains(&own(&p).arm.as_str()) {
        return Err(bad("new-name probe K8 only"));
    }
    authorize(root, &p)?;
    let h = history(root, &p)?;
    let last = h.last().ok_or_else(|| bad("probe endpoint"))?;
    if last.step != 512 || last.phase.as_deref() != Some("Finished") || last.resume {
        return Err(bad("probe final endpoint required"));
    }
    let (_, panels) = audit(root, &p, 512, 512)?;
    let dev = &panels["eval-0512-dev256"];
    if dev.exact < 244
        || dev.paired_both.map_or(0, |n| n.iter().sum::<usize>()) < 116
        || dev.errors != 0
    {
        return Err(bad("NOT_RUN_PREREQUISITE: K8 development gate"));
    }
    let x = verified_corpus(&root.join("transfer.r3cor"), &p.transfer)?;
    let (_, _, ms) = verified_metadata(root, &p)?;
    let mut control = observation_control(&p, 128, 128)?;
    write(
        &root.join("probe-started.r3b"),
        &binary::record!({"policy":digest(&p)?,"checkpoint":last.checkpoint_hash,"dataset":p.transfer,"cases":digest(&x.validation)?,"generation_limit":128,"teacher_limit":128}),
    )?;
    let result = evaluate_panel(
        &p,
        root,
        &root.join(&last.checkpoint),
        512,
        "new-names128",
        &x.validation,
        &ms,
        &mut control,
    );
    if let Err(e) = &result {
        control.classify_error(e);
    }
    let terminal = control.seal_terminal();
    let result = result.and_then(|s| terminal.map(|_| s));
    publish_confirmed(
        &root.join("probe-finished.r3b"),
        &binary::record!({"policy":digest(&p)?,"checkpoint":last.checkpoint_hash,"result":result.as_ref().ok(),"control":control.receipt(),"error":result.as_ref().err().map(ToString::to_string)}),
    )?;
    let s = result?;
    println!(
        "BINDING_NEW_NAMES full={}/{} both={:?} errors={} optimizer=0",
        s.exact, s.total, s.paired_both, s.errors
    );
    Ok(())
}
pub(in super::super) fn report(root: &Path) -> Result<()> {
    let p = plan_read(root)?;
    if !is(&p) {
        return Err(bad("binding report scope"));
    }
    let h = history(root, &p)?;
    let last = h.last().ok_or_else(|| bad("no binding execution"))?;
    let (_, rs) = audit(root, &p, 0, last.step)?;
    for (k, s) in &rs {
        let raw = binary::read_value_records(&root.join(format!("{k}.r3rows")))?;
        let tok = ByteBpe::load(&root.join("tokenizer.r3b"))?;
        let mut first = 0;
        let mut sides = [[0, 0]; 2];
        for (i, r) in raw[1..].iter().enumerate() {
            let expected = r["expected"].as_str().ok_or_else(|| bad("expected"))?;
            let id = tok.encode(&expected.as_bytes()[..1])?[0];
            first += usize::from(r["raw_tokens"][0] == id);
            sides[i % 2][1] += 1;
            sides[i % 2][0] += usize::from(r["exact_match"] == true);
        }
        println!(
            "BINDING_PANEL {k} full={}/{} value_first_token={first} body={} citation={} eos={} errors={} both={:?} sides={sides:?} CE={:?}",
            s.exact,
            s.total,
            s.field_exact,
            if is_orbit(&p) || ["A", "B"].contains(&own(&p).arm.as_str()) {
                String::from("NOT_APPLICABLE")
            } else {
                s.citation_exact.to_string()
            },
            s.eos,
            s.errors,
            s.paired_both,
            s.ce
        );
        let teachers = binary::read_value_records(&root.join(format!("{k}-teachers.r3rows")))?;
        let mut positions = BTreeMap::<usize, (f64, usize, usize)>::new();
        let mut divergence = BTreeMap::<usize, usize>::new();
        for (r, t) in raw[1..].iter().zip(&teachers[1..]) {
            let t = &t["teacher"]["target_token_observation"];
            let gold: Vec<u32> = binary::from_value(t["gold"].clone())?;
            let nll: Vec<f64> = binary::from_value(t["nll"].clone())?;
            let predicted: Vec<u32> = binary::from_value(t["argmax"].clone())?;
            if gold.len() != nll.len()
                || gold.len() != predicted.len()
                || nll.iter().any(|n| !n.is_finite())
            {
                return Err(bad("per-token teacher integrity"));
            }
            for (i, (&g, &n)) in gold.iter().zip(&nll).enumerate() {
                let z = positions.entry(i).or_default();
                z.0 += n;
                z.1 += usize::from(g == predicted[i]);
                z.2 += 1;
            }
            let output: Vec<u32> = binary::from_value(r["raw_tokens"].clone())?;
            if let Some(at) = gold
                .iter()
                .zip(&output)
                .position(|(a, b)| a != b)
                .or_else(|| (gold.len() != output.len()).then_some(gold.len().min(output.len())))
            {
                *divergence.entry(at).or_default() += 1;
            }
        }
        let ce = positions
            .into_iter()
            .map(|(i, (sum, correct, n))| (i, (sum / n as f64, correct, n)))
            .collect::<BTreeMap<_, _>>();
        println!(
            "BINDING_TOKEN_TEACHER {k} per_position_mean_CE_correct_total={ce:?} free_generation_first_divergence={divergence:?}"
        );
    }
    if root.join("probe-started.r3b").exists() {
        let r: binary::Value = read_confirmed(&root.join("probe-finished.r3b"))?;
        if !r["error"].is_null() || r["control"]["terminal_reason"] != "COMPLETED" {
            return Err(bad("new-name probe incomplete"));
        }
        let x = verified_corpus(&root.join("transfer.r3cor"), &p.transfer)?;
        let (_, _, ms) = verified_metadata(root, &p)?;
        let tok = ByteBpe::load(&root.join("tokenizer.r3b"))?;
        let s = audit_panel(root, &p, 512, "new-names128", &x.validation, &ms, &tok)?;
        if r["result"] != binary::record!(s) || r["checkpoint"] != last.checkpoint_hash {
            return Err(bad("probe endpoint/recount mismatch"));
        }
        println!(
            "BINDING_PROBE_RECOUNT full={}/128 both={:?} errors={}",
            s.exact, s.paired_both, s.errors
        );
    }
    let mut exposure = vec![0usize; if is_orbit(&p) {512} else {256}];
    let mut committed = 0;
    let mut committed_tokens = (0u64, 0u64);
    let mut executed = (0u64, 0u64, 0u64);
    for (i, _) in h.iter().enumerate() {
        let dir = root.join(format!("segment-{i:04}"));
        let c: binary::Value = read(&dir.join("train-control.r3b"))?;
        executed.0 += c["executed_input_tokens_including_uncommitted"]
            .as_u64()
            .ok_or_else(|| bad("input UNKNOWN"))?;
        executed.1 += c["executed_target_tokens_including_uncommitted"]
            .as_u64()
            .ok_or_else(|| bad("target UNKNOWN"))?;
        executed.2 += c["executed_padding_tokens"]
            .as_u64()
            .ok_or_else(|| bad("padding UNKNOWN"))?;
        if dir.join("updates.r3rows").exists() {
            for r in binary::read_value_records(&dir.join("updates.r3rows"))? {
                committed += 1;
                committed_tokens.0 += r["input"]
                    .as_u64()
                    .ok_or_else(|| bad("committed input UNKNOWN"))?;
                committed_tokens.1 += r["target"]
                    .as_u64()
                    .ok_or_else(|| bad("committed target UNKNOWN"))?;
                let draw: Vec<usize> = binary::from_value(r["sample_indices"].clone())?;
                if r["step"] != committed
                    || draw != p.training_draw(committed - 1)
                    || r["lr_bits"] != p.learning_rate(committed).to_bits()
                {
                    return Err(bad("actual update/tape/LR mismatch"));
                }
                for index in draw {
                    exposure[index] += 1;
                }
                if committed == 1 {
                    println!("BINDING_FIRST_BATCH {}", r["tasks"]);
                }
            }
        }
    }
    if committed != last.step
        || committed_tokens.0 > executed.0
        || committed_tokens.1 > executed.1
        || committed_tokens.1 != last.target
        || last.input != executed.0
    {
        return Err(bad("update trace/counter mismatch"));
    }
    println!(
        "BINDING_USAGE updates={committed} unique_bases={} exposures_min_max={}/{} committed_input={} target={} actual_input={} target={} padding={} discarded_input={} target={} generation={} teacher={} seconds={}",
        exposure
            .chunks_exact(if is_orbit(&p) {4} else {2})
            .filter(|n| n.iter().any(|&x| x > 0))
            .count(),
        exposure.iter().min().unwrap(),
        exposure.iter().max().unwrap(),
        committed_tokens.0,
        committed_tokens.1,
        executed.0,
        executed.1,
        executed.2,
        executed.0 - committed_tokens.0,
        executed.1 - committed_tokens.1,
        h.iter().map(|s| s.generations).sum::<usize>(),
        h.iter().map(|s| s.teachers).sum::<usize>(),
        h.iter().map(|s| s.elapsed).sum::<f64>()
    );
    println!(
        "BINDING_END arm={} step={} checkpoint={} physical={} gate={} resume={} stop={} MODEL_QUALITY=DIAGNOSTIC_ONLY GOAL1=false",
        own(&p).arm,
        last.step,
        last.checkpoint,
        last.checkpoint_hash,
        gate(root, &p)?,
        last.resume,
        last.stop
    );
    if is_orbit(&p) {
        println!("ORBIT_EXPOSURE {}", orbit_exposure(&own(&p).rows[..committed])?);
        orbit_report_panels(root, &p, last.step)?;
    }
    Ok(())
}

// A semantic skeleton is the sorted key set plus sorted value set. It is NOT
// orbit(), the older digit-rotation helper above.
fn semantic_skeleton(e: &Episode) -> Result<Skeleton> {
    if e.request.evidence.items.len() != 2 { return Err(bad("orbit requires two records")); }
    let mut s = [0;4];
    for (i,r) in e.request.evidence.items.iter().enumerate() {
        let (key, context, value) = parsed_record(r)?;
        let key = key.strip_prefix("장치").ok_or_else(||bad("K1 key"))?;
        if key.len()!=1 || value.len()!=1 || !key.as_bytes()[0].is_ascii_digit()
            || !value.as_bytes()[0].is_ascii_digit() || context!="구역0" {
            return Err(bad("K1 digit/context scope"));
        }
        s[i]=key.as_bytes()[0]-b'0'; s[i+2]=value.as_bytes()[0]-b'0';
    }
    if s.iter().collect::<BTreeSet<_>>().len()!=4 { return Err(bad("four distinct key/value digits")); }
    Ok(canonical(s))
}
fn value_offset(r: &Evidence) -> Result<usize> {
    let (_,_,value)=parsed_record(r)?;
    let n=r.original_excerpt.find(" 값은 ").ok_or_else(||bad("value span"))?+" 값은 ".len();
    if value.len()!=1 || &r.original_excerpt[n..] != format!("{value}이다.") {
        return Err(bad("exact value byte span"));
    }
    Ok(n)
}
fn swap_values(e: &Episode) -> Result<Episode> {
    semantic_skeleton(e)?;
    if resolve_request(&e.request)? != e.answer { return Err(bad("original orbit label")); }
    let values=e.request.evidence.items.iter().map(|r| parsed_record(r).map(|v|v.2.to_owned())).collect::<Result<Vec<_>>>()?;
    let mut out=e.clone();
    for (i,r) in out.request.evidence.items.iter_mut().enumerate() {
        let n=value_offset(r)?;
        r.original_excerpt.replace_range(n..n+1,&values[1-i]);
    }
    out.answer=resolve_request(&out.request)?;
    if out.answer==e.answer { return Err(bad("swap did not change gold")); }
    Ok(out)
}
fn verify_swap(a: &Episode,b: &Episode,tok: &ByteBpe) -> Result<()> {
    let expected=swap_values(a)?;
    if digest(&expected.request)? != digest(&b.request)? || expected.answer != b.answer { return Err(bad("values-only intervention mismatch")); }
    let pa=tok.prepare(&a.request,2048,"orbit-span")?;
    let pb=tok.prepare(&b.request,2048,"orbit-span")?;
    if pa.provided.len()!=2 || pb.provided.len()!=2 || !pa.excluded.is_empty() || !pb.excluded.is_empty()
        || pa.token_ids.len()!=pb.token_ids.len() {return Err(bad("value swap framing")); }
    let mut allowed=vec![];
    let ids=&pa.token_ids;
    for (record_index,start) in ids.iter().enumerate().filter_map(|(i,&t)|(t==neural::EVIDENCE_ROLE).then_some(i+1)).enumerate() {
        let end=start+ids[start..].iter().position(|&t|t==neural::END_ROLE).ok_or_else(||bad("evidence framing end"))?;
        let bytes=tok.decode_bytes(&ids[start..end])?;
        let r=&a.request.evidence.items[record_index];
        let excerpt=r.original_excerpt.as_bytes();
        let locations=bytes.windows(excerpt.len()).enumerate().filter_map(|(i,w)|(w==excerpt).then_some(i)).collect::<Vec<_>>();
        if locations.len()!=1 {return Err(bad("unique excerpt in actual prompt")); }
        let at=locations[0]+value_offset(r)?;
        let mut offset=0;
        let mut found=None;
        for j in start..end {
            let decoded=tok.decode_bytes(&[ids[j]])?;
            if offset==at && decoded.len()==1 {found=Some(j);}
            offset+=decoded.len();
        }
        allowed.push(found.ok_or_else(||bad("value not standalone framed token"))?);
    }
    let changed=pa.token_ids.iter().zip(&pb.token_ids).enumerate().filter_map(|(i,(a,b))|(a!=b).then_some(i)).collect::<Vec<_>>();
    if changed!=allowed || changed.len()!=2 {return Err(bad("input difference outside two value spans")); }
    let sa=samples(std::slice::from_ref(a),tok,256)?.remove(0);
    let sb=samples(std::slice::from_ref(b),tok,256)?.remove(0);
    if sa.tokens.len()!=sb.tokens.len() || sa.tokens.len()>256
        || sa.response_start!=sb.response_start || sa.tokens.len()-sa.response_start!=2
        || sa.tokens.last()!=Some(&EOS) || sb.tokens.last()!=Some(&EOS)
        || sa.tokens[sa.response_start]==sb.tokens[sb.response_start] {
        return Err(bad("digit+EOS target intervention"));
    }
    Ok(())
}
pub(in super::super) fn orbit_foil(e: &Episode) -> Result<String> {
    semantic_skeleton(e)?;
    let gold=resolve_request(&e.request)?;
    if gold!=e.answer {return Err(bad("orbit teacher gold"));}
    e.request.evidence.items.iter().map(|r|parsed_record(r).map(|v|v.2.to_owned()))
        .collect::<Result<Vec<_>>>()?.into_iter().find(|v|v!=&gold).ok_or_else(||bad("orbit foil"))
}
fn expand_assignments(es: &[Episode], ms: &[Meta], split: &str) -> Result<(Vec<Episode>,Vec<Meta>)> {
    if es.len()!=ms.len() || es.len()%2!=0 {return Err(bad("paired source count"));}
    let mut out=vec![]; let mut meta=vec![];
    for (base,(pair,pm)) in es.chunks_exact(2).zip(ms.chunks_exact(2)).enumerate() {
        if pair[0].request.evidence!=pair[1].request.evidence || pm[0].base!=pm[1].base
            || pair[0].answer==pair[1].answer {return Err(bad("paired source identity"));}
        let b=format!("{ORBIT_DATA}/{split}/{base}");
        for assignment in 0..2 {
            for q in 0..2 {
                let mut e=if assignment==0 {pair[q].clone()} else {swap_values(&pair[q])?};
                e.id=format!("{b}/{assignment}/{q}");e.family=b.clone();e.sequence=b.clone();
                e.binding=digest(&e.request.evidence)?;
                let mut m=pm[q].clone();m.id=e.id.clone();m.base=b.clone();m.view=assignment*2+q;
                m.split=split.into();m.template="V".into();m.source_id=Some(pair[q].id.clone());
                out.push(e);meta.push(m);
            }
        }
    }
    Ok((out,meta))
}
fn heldout_skeletons(excluded: &[Skeleton],count:usize) -> Vec<Skeleton> {
    // Finite deterministic greedy marginal balancing, hash tie break; no model
    // scores, repeated seed search, or unbounded rejection sampling.
    let mut candidates=universe().into_iter().filter(|s|!excluded.contains(s)).collect::<Vec<_>>();
        let mut frequency=[[0usize;10];2];let mut panel=vec![];
        for _ in 0..count {
            let i=(0..candidates.len()).min_by_key(|&i| {
                let s=candidates[i];
                ((0..4).map(|j|2*frequency[j/2][s[j] as usize]+1).sum::<usize>(),rank(s))
            }).unwrap();
            let s=candidates.remove(i);
            for j in 0..4 {frequency[j/2][s[j] as usize]+=1;}
            panel.push(s);
        }
    panel
}
fn heldout_cases(sc: &[Skeleton], split: &str, train: &[Episode]) -> Result<(Vec<Episode>,Vec<Meta>)> {
    let mut order=vec![false;sc.len()];
    for x in order.iter_mut().skip(sc.len()/2) {*x=true;}
    shuffle(&mut order,&mut stream(17,&format!("{ORBIT_DATA}/{split}/physical")));
    let mut used=train.iter().flat_map(|e|e.request.evidence.items.iter().map(|r|r.event_id)).collect::<BTreeSet<_>>();
    let mut ids=stream(17,&format!("{ORBIT_DATA}/{split}/event"));
    let mut es=vec![];let mut ms=vec![];
    for (i,s) in sc.iter().enumerate() {
        let mut records=train[0].request.evidence.items.clone();
        for j in 0..2 {
            let id=10_000_000+(ids.next_u64()%90_000_000) as i64;
            if !used.insert(id) {return Err(bad("finite heldout event collision"));}
            records[j].event_id=id;
            records[j].original_excerpt=format!("장치{}의 구역0 값은 {}이다.",s[j],s[j+2]);
        }
        if order[i] {records.reverse();}
        let base=format!("foundation-heldout/{split}/{i}");
        for q in 0..2 {
            let mut e=train[q].clone();
            e.id=format!("{base}/{q}");e.family=base.clone();e.sequence=base.clone();
            e.request.evidence.items=records.clone();
            e.request.input=format!("장치{} 구역0 {VALUE_QUERY}",s[q]);
            e.answer=resolve_request(&e.request)?;e.binding=digest(&e.request.evidence)?;
            ms.push(Meta{id:e.id.clone(),base:base.clone(),template:"V".into(),bucket:0,view:q,split:split.into(),
                entities:vec![format!("장치{}",s[0]),format!("장치{}",s[1])],source_id:None,query_context:Some("구역0".into())});
            es.push(e);
        }
    }
    expand_assignments(&es,&ms,split)
}
fn orbit_validate(es:&[Episode],ms:&[Meta],tok:&ByteBpe,count:usize) -> Result<binary::Value> {
    if es.len()!=count || ms.len()!=count || count%4!=0 {return Err(bad("orbit complete ALL4 panel"));}
    let mut pair_meta=ms.to_vec();
    for m in &mut pair_meta {m.base=format!("{}/{}",m.base,m.view/2);m.view%=2;}
    let framing=validate(es,&pair_meta,tok,count)?;
    let mut skeleton_ids=BTreeSet::new();let mut events=BTreeSet::new();
    let mut frequency=[[0usize;10];2];let mut physical=[0usize;2];
    let mut rules=BTreeMap::<String,[usize;2]>::new();
    for (quad,meta) in es.chunks_exact(4).zip(ms.chunks_exact(4)) {
        let skeleton=semantic_skeleton(&quad[0])?;
        if !skeleton_ids.insert(skeleton) {return Err(bad("repeated semantic skeleton"));}
        for j in 0..4 {frequency[j/2][skeleton[j] as usize]+=1;}
        for r in &quad[0].request.evidence.items {if !events.insert(r.event_id) {return Err(bad("duplicate skeleton event"));}}
        physical[usize::from(parsed_record(&quad[0].request.evidence.items[0])?.0
            != format!("장치{}",skeleton[0]))]+=1;
        for j in 0..4 {
            if semantic_skeleton(&quad[j])?!=skeleton || meta[j].base!=meta[0].base || meta[j].view!=j
                || quad[j].family!=meta[j].base || meta[j].id!=quad[j].id {return Err(bad("ALL4 metadata/request grouping"));}
            if j<2 {verify_swap(&quad[j],&quad[j+2],tok)?;}
            for (name,index) in [("first_record",0),("last_record",1),("smaller_event_id",
                usize::from(quad[j].request.evidence.items[0].event_id>quad[j].request.evidence.items[1].event_id))] {
                let v=parsed_record(&quad[j].request.evidence.items[index])?.2;
                let r=rules.entry(name.into()).or_default();r[0]+=usize::from(v==quad[j].answer);r[1]+=1;
            }
            let smaller=quad[j].request.evidence.items.iter().min_by_key(|r|parsed_record(r).unwrap().0).unwrap();
            let r=rules.entry("smaller_key".into()).or_default();r[0]+=usize::from(parsed_record(smaller)?.2==quad[j].answer);r[1]+=1;
        }
    }
    Ok(binary::record!({"framing":framing,"skeletons":skeleton_ids,"key_value_frequencies":frequency,"physical_order":physical,"query_ignore_rules":rules,"rows":count,"orbits":count/4}))
}
fn orbit_rows(arm:&str,steps:usize) -> Vec<[usize;8]> {
    let mut visits=[0usize;128];
    tape(steps).into_iter().map(|old| {
        let mut row=[0;8];
        for j in 0..4 {
            let b=old[2*j]/2;
            let assignment=if arm=="BOTH" {(visits[b]+usize::from(b>=64))%2} else {0};
            row[2*j]=b*4+assignment*2;row[2*j+1]=row[2*j]+1;visits[b]+=1;
        }
        row
    }).collect()
}
fn orbit_exposure(rows:&[[usize;8]]) -> Result<binary::Value> {
    let mut counts=vec![0usize;512];let mut visits=vec![0usize;128];let mut assignments=vec![[0usize;2];128];
    for row in rows {
        let mut seen=BTreeSet::new();
        for pair in row.chunks_exact(2) {
            if pair[0]>=512 || pair[0]%2!=0 || pair[1]!=pair[0]+1 || !seen.insert(pair[0]/4) {return Err(bad("four skeleton/query pairs per batch"));}
            visits[pair[0]/4]+=1;assignments[pair[0]/4][(pair[0]%4)/2]+=1;
            counts[pair[0]]+=1;counts[pair[1]]+=1;
        }
    }
    Ok(binary::record!({"consumed_examples":rows.len()*8,"per_id_count":counts,"exposed_unique_ids":counts.iter().enumerate().filter_map(|(i,&n)|(n>0).then_some(i)).collect::<Vec<_>>(),"per_skeleton_visits":visits,"per_assignment_count":assignments}))
}
fn orbit_config(tiny:bool) -> TrainConfig {let mut c=config(tiny);c.max_tokens=2_000_000;c}
fn orbit_native(train:Vec<Episode>,dev:Vec<Episode>) -> Result<data::native::Corpus> {
    let manifest=data::CorpusManifest{version:1,scope:"project-owned finite K1 binding".into(),permission:"synthetic project-owned".into(),
        generator:ORBIT_DATA.into(),seed:17,split_rule:"disjoint sorted key-set/value-set skeletons; both assignments and queries; individual digits/edges may overlap".into(),
        train:data::native::split("train",&train),validation:data::native::split("validation",&dev)};
    data::native::from_episodes(manifest,train,dev)
}
fn copy_native(source:&Path,dest:&Path) -> Result<()> {
    replica_v3::codec::publish_new(dest,|f,_| {std::io::copy(&mut std::fs::File::open(source)?,f)?;Ok(())})
}
fn orbit_parent(parent:&Path) -> Result<(Plan,data::native::Corpus,Vec<Meta>,ByteBpe)> {
    let raw:Plan=read(&parent.join("plan.r3b"))?;
    let p=plan_read_bound(parent,&raw.source,&raw.binary)?;
    if own(&p).dataset!=DATA || own(&p).arm!="A" {return Err(bad("preserved original A required"));}
    let corpus=verified_corpus(&parent.join("corpus.r3cor"),&p.corpus)?;
    let (tm,_,_)=verified_metadata(parent,&p)?;
    let tok=ByteBpe::load(&parent.join("tokenizer.r3b"))?;
    Ok((p,corpus,tm,tok))
}
pub(in super::super) fn orbit_prepare(parent:&Path,output:&Path,tiny:bool) -> Result<()> {
    if cfg!(feature="test-support") && !tiny {return Err(bad("production orbit binary required"));}
    let (old,original,om,tok)=orbit_parent(parent)?;
    if old.tiny!=tiny {return Err(bad("orbit initial profile"));}
    if !output.exists() {std::fs::create_dir(output)?;}
    let initial=checkpoint::load(&parent.join("initial.r3m"),Device::Cpu,false)?;
    if initial.manifest.training.is_some() || !initial.optimizer.is_empty()
        || initial.model.weight_hash()?!=old.initial_weights || initial.tokenizer.id()!=old.tokenizer
        || initial.model.config!=old.architecture {return Err(bad("untrained original initial required"));}
    let adam=Adam::new(&initial.model.vars)?;
    if adam.moments.values().any(|t|t.flatten_all().and_then(|x|x.to_vec1::<f32>()).map_or(true,|v|v.iter().any(|x|x.to_bits()!=0))) {return Err(bad("fresh Adam zero"));}
    let (train,tm)=expand_assignments(&original.train,&om,"train")?;
    let train_skeletons=original.train.chunks_exact(2).map(|p|semantic_skeleton(&p[0])).collect::<Result<Vec<_>>>()?;
    let dev_sc=heldout_skeletons(&train_skeletons,128);
    let (dev,dm)=heldout_cases(&dev_sc,"dev",&original.train)?;
    let tr=orbit_validate(&train,&tm,&tok,512)?;let dv=orbit_validate(&dev,&dm,&tok,512)?;
    let selection=binary::record!({"contract":ORBIT_CONTRACT,"parent":parent,"parent_policy":digest(&old)?,"parent_corpus":old.corpus,"parent_metadata":old.metadata,
        "initial":old.initial,"initial_content":initial.model.weights_content_id()?,"train":train_skeletons,"dev":dev_sc,
        "confirmation_count":64,"confirmation_policy":"same finite selector over universe excluding train+dev; independent seal only",
        "selection":"finite incremental squared marginal frequency; fixed binding rank tie break; no model input","source":source_digest()?});
    write(&output.join("selection.r3b"),&selection)?;
    let c=orbit_native(train.clone(),dev.clone())?;
    let mut plans=BTreeMap::new();
    for arm in ORBIT_ARMS {
        let root=output.join(arm);std::fs::create_dir(&root)?;
        data::native::write(&root.join("corpus.r3cor"),&c,true)?;
        copy_native(&root.join("corpus.r3cor"),&root.join("transfer.r3cor"))?; // existing loader slot; never confirmation
        tok.save(&root.join("tokenizer.r3b"))?;
        write(&root.join("metadata.r3b"),&(tm.clone(),dm.clone(),dm.clone()))?;
        copy_native(&parent.join("initial.r3m"),&root.join("initial.r3m"))?;
        let rows=orbit_rows(arm,orbit_config(tiny).max_steps);
        let mut p=old.clone();p.source=source_digest()?;p.binary=file_hash(&std::env::current_exe()?)?;
        p.identifiable=Some(Policy{study:output.to_owned(),arm:arm.into(),dataset:ORBIT_DATA.into(),rows:rows.clone()});
        p.corpus=file_hash(&root.join("corpus.r3cor"))?;p.transfer=file_hash(&root.join("transfer.r3cor"))?;
        p.metadata=file_hash(&root.join("metadata.r3b"))?;p.initial=file_hash(&root.join("initial.r3m"))?;
        p.config=orbit_config(tiny);p.order=vec![(0..512).collect()];p.train_order=digest(&rows)?;
        p.split_policy=c.manifest.split_rule.clone();p.evaluation=evaluation_for(&p);
        write(&root.join("plan.r3b"),&p)?;
        orbit_verify_plan(&root,&p)?;
        let frames=samples(&train,&tok,256)?;
        let input=rows.iter().flatten().map(|&i|frames[i].tokens.len()-1).sum::<usize>();
        plans.insert(arm,binary::record!({"policy":file_hash(&root.join("plan.r3b"))?,"corpus":p.corpus,"metadata":p.metadata,"initial":p.initial,
            "initial_content":initial.model.weights_content_id()?,"input":input,"target":rows.len()*16,"padding":0,"exposure":orbit_exposure(&rows)?}));
    }
    publish_confirmed(&output.join("preparation.r3b"),&binary::record!({"contract":ORBIT_CONTRACT,"dataset":ORBIT_DATA,"source":source_digest()?,
        "binary":file_hash(&std::env::current_exe()?)?,"arms":plans,"selection":file_hash(&output.join("selection.r3b"))?,"train":tr,"dev":dv,
        "tokenizer":tok.id(),"initial_content":initial.model.weights_content_id()?,"adam_zero":optimizer_hash(&adam.moments)?,
        "confirmation":"RESERVED_FOR_INDEPENDENT_SEAL","parent":parent,"new_updates":0,"generation":0,"teacher":0}))?;
    println!("ORBIT_PREPARED FIXED256x16 BOTH512x8 skeletons128 updates=0 independent_review=PENDING");
    Ok(())
}
fn orbit_verify_plan(root:&Path,p:&Plan) -> Result<()> {
    let o=own(p);
    if !ORBIT_ARMS.contains(&o.arm.as_str()) || root!=o.study.join(&o.arm) || p.config!=orbit_config(p.tiny)
        || p.order!=vec![(0..512).collect::<Vec<_>>()] || o.rows!=orbit_rows(&o.arm,p.config.max_steps)
        || p.architecture!=architecture(p.architecture.vocab,p.tiny) || p.fork.is_some()
        || p.paired.is_some() || p.training_values.is_some() || p.grounding.is_some() || p.model_seed!=17 || p.data_seed!=17 {
        return Err(bad("orbit policy/tape scope"));
    }
    let selection:binary::Value=read(&o.study.join("selection.r3b"))?;
    let parent=Path::new(selection["parent"].as_str().ok_or_else(||bad("orbit source parent"))?);
    let (old,original,om,tok)=orbit_parent(parent)?;
    if selection["parent_policy"]!=digest(&old)? || selection["parent_corpus"]!=old.corpus
        || selection["parent_metadata"]!=old.metadata || selection["initial"]!=p.initial
        || p.initial!=old.initial || p.initial_weights!=old.initial_weights || p.tokenizer!=tok.id()
        || selection["source"]!=p.source {return Err(bad("orbit original binding"));}
    let expected_train=expand_assignments(&original.train,&om,"train")?;
    let train_sc=original.train.chunks_exact(2).map(|p|semantic_skeleton(&p[0])).collect::<Result<Vec<_>>>()?;
    let dev_sc=heldout_skeletons(&train_sc,128);
    if selection["train"]!=binary::record!(train_sc) || selection["dev"]!=binary::record!(dev_sc)
        || selection["confirmation_count"]!=64 || selection["confirmation_policy"]!="same finite selector over universe excluding train+dev; independent seal only" {
        return Err(bad("semantic split selection"));
    }
    let expected_dev=heldout_cases(&dev_sc,"dev",&original.train)?;
    let corpus=verified_corpus(&root.join("corpus.r3cor"),&p.corpus)?;
    let (tm,dm,xm)=verified_metadata(root,p)?;
    if digest(&(&corpus.train,&tm))?!=digest(&expected_train)? || digest(&(&corpus.validation,&dm))?!=digest(&expected_dev)?
        || digest(&xm)?!=digest(&dm)? || p.transfer!=p.corpus {return Err(bad("frozen orbit pool differs from original/split"));}
    orbit_validate(&corpus.train,&tm,&tok,512)?;orbit_validate(&corpus.validation,&dm,&tok,512)?;
    let frames=samples(&corpus.train,&tok,256)?;
    let old_samples=samples(&original.train,&tok,256)?;
    let old_rows=tape(p.config.max_steps);let mut input=0;
    for (step,row) in o.rows.iter().enumerate() {
        for j in 0..8 {
            let index=row[j];let old_index=old_rows[step][j];
            if index/4!=old_index/2 || index%2!=old_index%2 {return Err(bad("original base/query order changed"));}
            let base=&frames[index/4*4+index%2];
            if base.tokens!=old_samples[old_index].tokens {return Err(bad("original FIXED encoded order"));}
            if frames[index].tokens.len()!=base.tokens.len() {return Err(bad("unequal token cost"));}
            input+=frames[index].tokens.len()-1;
        }
    }
    if o.rows.len()*16>20_000 || input*2>2_000_000 {return Err(bad("orbit planned budget"));}
    Ok(())
}

#[derive(Debug,Serialize,Deserialize,PartialEq)]
struct OrbitScore {
    total:usize, full:usize, query_both:usize, swap_both:usize, all4:usize,
    eos:usize, errors:usize, gold_foil_other_malformed:[usize;4],
    same_across_queries:usize, same_across_assignments:usize,
    exact:Vec<bool>,
}
fn orbit_score(es:&[Episode],ms:&[Meta],rows:&[binary::Value],tok:&ByteBpe) -> Result<OrbitScore> {
    if es.len()%4!=0 || rows.len()!=es.len() {return Err(bad("incomplete orbit scoring panel"));}
    let s=score(rows,es,ms)?;
    let mut out=OrbitScore{total:s.total,full:s.exact,query_both:0,swap_both:0,all4:0,eos:s.eos,errors:s.errors,
        gold_foil_other_malformed:[0;4],same_across_queries:0,same_across_assignments:0,exact:vec![]};
    for (e,r) in es.iter().zip(rows) {
        verify_generated(r,tok)?;
        let exact=r["error"].is_null() && r["generation_completed"]==true && r["finish_reason"]=="stop" && r["actual"]==e.answer;
        out.exact.push(exact);
        let actual=r["actual"].as_str();
        let kind=if exact {0} else if !r["error"].is_null() || r["finish_reason"]!="stop" || r["generation_completed"]!=true
            || actual.is_none_or(|v|v.len()!=1 || !v.as_bytes()[0].is_ascii_digit()) {3}
            else if actual==Some(orbit_foil(e)?.as_str()) {1} else {2};
        out.gold_foil_other_malformed[kind]+=1;
    }
    for (q,r) in out.exact.chunks_exact(4).zip(rows.chunks_exact(4)) {
        out.query_both+=usize::from(q[0]&&q[1])+usize::from(q[2]&&q[3]);
        out.swap_both+=usize::from(q[0]&&q[2])+usize::from(q[1]&&q[3]);
        out.all4+=usize::from(q.iter().all(|&v|v));
        for (a,b) in [(0,1),(2,3)] {out.same_across_queries+=usize::from(r[a]["actual"].is_string() && r[a]["actual"]==r[b]["actual"]);}
        for (a,b) in [(0,2),(1,3)] {out.same_across_assignments+=usize::from(r[a]["actual"].is_string() && r[a]["actual"]==r[b]["actual"]);}
    }
    Ok(out)
}
fn orbit_panel(root:&Path,p:&Plan,step:usize,name:&str) -> Result<OrbitScore> {
    let (_,es,ms)=panel_cases(root,p,step)?.into_iter().find(|(n,_,_)|n==name).ok_or_else(||bad("orbit panel name"))?;
    let tok=ByteBpe::load(&root.join("tokenizer.r3b"))?;
    audit_panel(root,p,step,name,&es,&ms,&tok)?;
    let rows=binary::read_value_records(&root.join(format!("eval-{step:04}-{name}.r3rows")))?;
    orbit_score(&es,&ms,&rows[1..],&tok)
}
fn orbit_candidate_scores(tr:&OrbitScore,dv:&OrbitScore) -> bool {
    tr.total==512 && dv.total==512 && tr.full>=508 && tr.query_both>=252 && tr.all4>=124
        && dv.full>=488 && dv.query_both>=232 && dv.all4>=116
        && tr.errors==0 && dv.errors==0 && tr.eos==512 && dv.eos==512
}
fn orbit_gate(root:&Path,p:&Plan) -> Result<bool> {
    if p.tiny {return Ok(false);}
    let h=history(root,p)?;
    if h.last().is_none_or(|s|s.step!=512 || s.resume || s.phase.as_deref()!=Some("Finished") || s.stop!="BUDGET_REACHED") {return Ok(false);}
    Ok(orbit_candidate_scores(&orbit_panel(root,p,512,"train512")?,&orbit_panel(root,p,512,"dev512")?))
}
fn orbit_report_panels(root:&Path,p:&Plan,last:usize) -> Result<()> {
    for &step in p.evaluation.train_steps.iter().filter(|&&s|s<=last) {
        for (name,es,_) in panel_cases(root,p,step)? {
            let s=orbit_panel(root,p,step,&name)?;
            let seen=own(p).rows[..step].iter().flatten().copied().collect::<BTreeSet<_>>();
            let exposed=(0..s.total).filter(|i|name.starts_with("train") && seen.contains(i)).collect::<Vec<_>>();
            let exposed_full=exposed.iter().filter(|&&i|s.exact[i]).count();
            println!("ORBIT_PANEL step={step} arm={} panel={name} full={}/{} query_both={}/{} swap_both={}/{} all4={}/{} classes={:?} same_queries={} same_assignments={} exposed_fit={}/{}",
                own(p).arm,s.full,s.total,s.query_both,s.total/2,s.swap_both,s.total/2,s.all4,s.total/4,s.gold_foil_other_malformed,
                s.same_across_queries,s.same_across_assignments,exposed_full,exposed.len());
            let t=binary::read_value_records(&root.join(format!("eval-{step:04}-{name}-teachers.r3rows")))?;
            let tok=ByteBpe::load(&root.join("tokenizer.r3b"))?;
            let mut sum=[0f64;3];
            for (r,e) in t[1..].iter().zip(&es) {
                let observation=&r["teacher"]["target_token_observation"];
                let n:Vec<f64>=binary::from_value(observation["nll"].clone())?;
                let gold=tok.encode(e.answer.as_bytes())?[0];let foil=tok.encode(orbit_foil(e)?.as_bytes())?[0];
                let binary=&r["teacher"]["conditional_foil"];
                let g=binary["gold_logit"].as_f64().ok_or_else(||bad("missing gold logit"))?;
                let f=binary["foil_logit"].as_f64().ok_or_else(||bad("missing foil logit"))?;
                let d=g-f;let nll=(-d).max(0.)+(-d.abs()).exp().ln_1p();
                if n.len()!=2 || n.iter().any(|v|!v.is_finite()) || !g.is_finite() || !f.is_finite()
                    || observation["gold"]!=binary::record!([gold,EOS]) || binary["gold"]!=gold || binary["foil"]!=foil
                    || binary["margin"]!=binary::record!(d) || binary["binary_nll"]!=binary::record!(nll) {
                    return Err(bad("full-vocabulary/renormalized NLL integrity"));
                }
                sum[0]+=n[0];sum[1]+=n[1];sum[2]+=nll;
            }
            println!("ORBIT_NLL step={step} panel={name} value_full_vocab={} eos_full_vocab={} gold_foil_binary={} count={} binary_uniform_reference={}",
                sum[0]/s.total as f64,sum[1]/s.total as f64,sum[2]/s.total as f64,s.total,std::f64::consts::LN_2);
        }
    }
    Ok(())
}
// One bounded observation path for swap/parity/confirmation. Start precedes
// model loading; every returned call is durable, and a missing final is UNKNOWN.
fn orbit_observe(root:&Path,name:&str,path:&Path,es:&[Episode],identity:&binary::Value,
    previous:Option<&[binary::Value]>,mut control:recovery::RunControl) -> Result<Vec<binary::Value>> {
    let binding=binary::record!({"identity":identity,"checkpoint":file_hash(path)?,"cases":digest(&es)?,"source":source_digest()?,
        "binary":file_hash(&std::env::current_exe()?)?,"decoding":"normal-greedy-strict-utf8-eos"});
    write(&root.join(format!("{name}-started.r3b")),&binding)?;
    let raw=root.join(format!("{name}.r3rows"));
    let mut rows=vec![];let mut matched=0;
    let result=(||->Result<()> {
        let mut f=std::fs::OpenOptions::new().create_new(true).write(true).open(&raw)?;
        let l=checkpoint::load(path,Device::Cpu,false)?;
        append_row(&mut f,&binding)?;
        for (i,e) in es.iter().enumerate() {
            control.check("orbit_next_generation")?;
            let attempt=prepare_call(root,name,"generation",&binding,e,i)?;
            let mut row=match recovery::observe_generation(&l,e,&e.request,&mut control,false) {
                recovery::ObservedCall::NotInvoked(_)=>{resolve_call(&attempt,None,&control)?;return Err(bad("orbit not invoked"));},
                recovery::ObservedCall::Returned(row)=>row,
            };
            row["attempt"]=binary::record!(attempt.file_name().unwrap().to_string_lossy());
            append_row(&mut f,&row)?;resolve_call(&attempt,Some(&row),&control)?;
            verify_generated(&row,&l.tokenizer)?;
            rows.push(row);
            if let Some(old)=previous {
                for key in ["raw_tokens","actual","error","finish_reason","generation_completed"] {
                    if old.get(i).is_none_or(|r|r[key]!=rows[i][key]) {return Err(bad("orbit endpoint parity mismatch"));}
                }
                matched+=1;
            }
            control.stop_result()?;
        }
        Ok(())
    })();
    if let Err(e)=&result {control.classify_error(e);}
    let result=result.and(control.seal_terminal());
    let raw_hash=if raw.exists(){Some(file_hash(&raw)?)}else{None};
    publish_confirmed(&root.join(format!("{name}-finished.r3b")),&binary::record!({
        "policy":identity["policy"],"checkpoint":binding["checkpoint"],"binding":binding,"completed":rows.len(),"matched":matched,
        "control":control.receipt(),"error":result.as_ref().err().map(ToString::to_string),"raw":raw_hash}))?;
    result?;
    Ok(rows)
}
fn swap_control() -> Result<recovery::RunControl> {
    let flag=std::sync::Arc::new(AtomicBool::new(false));let signal=flag.clone();
    ctrlc::set_handler(move||signal.store(true,Ordering::Relaxed)).map_err(|e|bad(&e.to_string()))?;
    let mut c=recovery::RunControl::new(flag,std::time::Duration::from_secs(900),16*1024*1024)?;
    c.set_call_limits(16,0);Ok(c)
}
pub(in super::super) fn orbit_swap(parent:&Path,output:&Path) -> Result<()> {
    let (p,corpus,ms,tok)=orbit_parent(parent)?;
    if p.tiny || cfg!(feature="test-support") {return Err(bad("production A512 swap only"));}
    let h=history(parent,&p)?;let end=h.last().ok_or_else(||bad("preserved A endpoint missing"))?;
    if end.step!=512 || end.resume || end.phase.as_deref()!=Some("Finished") {return Err(bad("A512 closed endpoint required"));}
    let cases=corpus.train[..16].to_vec();let metadata=ms[..16].to_vec();
    let swapped=cases.iter().map(swap_values).collect::<Result<Vec<_>>>()?;
    for (a,b) in cases.iter().zip(&swapped) {verify_swap(a,b,&tok)?;}
    std::fs::create_dir(output)?;
    let identity=binary::record!({"contract":ORBIT_CONTRACT,"parent":parent,"policy":digest(&p)?,"checkpoint":end.checkpoint_hash,
        "selection":"first eight train metadata bases, both views; no score selection","ids":cases.iter().map(|e|&e.id).collect::<Vec<_>>(),
        "original_cases":digest(&cases)?,"swapped_cases":digest(&swapped)?,"original_raw":file_hash(&parent.join("eval-0512-train256.r3rows"))?});
    write(&output.join("swap-selection.r3b"),&identity)?;
    audit_panel(parent,&p,512,"train256",&corpus.train,&ms,&tok)?;
    let old=binary::read_value_records(&parent.join("eval-0512-train256.r3rows"))?;
    let original=score(&old[1..17],&cases,&metadata)?;
    let rows=orbit_observe(output,"swap",&parent.join(&end.checkpoint),&swapped,&identity,None,swap_control()?)?;
    let result=score(&rows,&swapped,&metadata)?;
    let mut original_both=0;let mut conditional_both=0;let mut swapped_both=0;let mut classes=[0;4];
    for i in 0..8 {
        let a=old[i*2+1]["exact_match"]==true && old[i*2+2]["exact_match"]==true;
        let b=rows[i*2]["exact_match"]==true && rows[i*2+1]["exact_match"]==true;
        original_both+=usize::from(a);swapped_both+=usize::from(b);conditional_both+=usize::from(a&&b);
    }
    for (e,r) in swapped.iter().zip(&rows) {
        let kind=if r["exact_match"]==true {0} else if r["error"].is_null() && r["finish_reason"]=="stop" && r["actual"]==orbit_foil(e)? {1}
            else if r["error"].is_null() && r["finish_reason"]=="stop" && r["actual"].as_str().is_some_and(|s|s.len()==1 && s.as_bytes()[0].is_ascii_digit()) {2} else {3};
        classes[kind]+=1;
    }
    let same=rows.iter().zip(&old[1..17]).filter(|(a,b)|a["actual"].is_string() && a["actual"]==b["actual"]).count();
    let summary=binary::record!({"identity":identity,"original_full":original.exact,"original_both":original_both,"swapped_full":result.exact,
        "swapped_both":swapped_both,"original_both_subset_swapped_both":conditional_both,"subset_denominator":original_both,
        "subset_status":if original_both==0 {"NOT_IDENTIFIABLE"}else{"SMALL_FIXED_DIAGNOSTIC"},
        "same_output":same,"gold_foil_other_malformed":classes,"raw":file_hash(&output.join("swap.r3rows"))?,
        "generation":16,"teacher":0,"optimizer":0});
    publish_confirmed(&output.join("swap-result.r3b"),&summary)?;
    println!("ORBIT_SWAP16 {summary}");Ok(())
}
fn verify_seal(study:&Path) -> Result<binary::Value> {
    let receipt:binary::Value=read_confirmed(&study.join("confirmation-seal.r3b"))?;
    if receipt["preparation"]!=file_hash(&study.join("preparation.r3b"))?
        || receipt["selection"]!=file_hash(&study.join("selection.r3b"))? || receipt["count"]!=256
        || receipt["corpus"]!=file_hash(&study.join("sealed/confirmation.r3cor"))?
        || receipt["metadata"]!=file_hash(&study.join("sealed/metadata.r3b"))? {
        return Err(bad("confirmation seal binding"));
    }
    Ok(receipt)
}
pub(in super::super) fn orbit_seal(study:&Path) -> Result<()> {
    let p=plan_read(&study.join("FIXED"))?;
    if !is_orbit(&p) {return Err(bad("orbit seal scope"));}
    for arm in ORBIT_ARMS {if study.join(arm).join("segment-0000-started.r3b").exists() {return Err(bad("confirmation must precede training"));}}
    let selection:binary::Value=read(&study.join("selection.r3b"))?;
    let train_sc:Vec<Skeleton>=binary::from_value(selection["train"].clone())?;
    let dev_sc:Vec<Skeleton>=binary::from_value(selection["dev"].clone())?;
    let sc=heldout_skeletons(&[train_sc.clone(),dev_sc.clone()].concat(),64);
    let c=verified_corpus(&study.join("FIXED/corpus.r3cor"),&p.corpus)?;
    let tok=ByteBpe::load(&study.join("FIXED/tokenizer.r3b"))?;
    let (es,ms)=heldout_cases(&sc,"confirmation",&c.train)?;
    let stats=orbit_validate(&es,&ms,&tok,256)?;
    if sc.iter().any(|s|train_sc.contains(s)||dev_sc.contains(s)) {return Err(bad("confirmation skeleton leak"));}
    let all_other_ids=c.train.iter().chain(&c.validation).flat_map(|e|e.request.evidence.items.iter().map(|r|r.event_id)).collect::<BTreeSet<_>>();
    if es.iter().flat_map(|e|e.request.evidence.items.iter()).any(|r|all_other_ids.contains(&r.event_id)) {return Err(bad("confirmation event collision"));}
    std::fs::create_dir(study.join("sealed"))?;
    data::native::write(&study.join("sealed/confirmation.r3cor"),&orbit_native(c.train,es)?,true)?;
    write(&study.join("sealed/metadata.r3b"),&ms)?;
    publish_confirmed(&study.join("confirmation-seal.r3b"),&binary::record!({"preparation":file_hash(&study.join("preparation.r3b"))?,
        "selection":file_hash(&study.join("selection.r3b"))?,"corpus":file_hash(&study.join("sealed/confirmation.r3cor"))?,
        "metadata":file_hash(&study.join("sealed/metadata.r3b"))?,"count":256,"skeletons":64,"skeleton_digest":digest(&sc)?,
        "max_prompt_target_eos":stats["framing"]["max_prompt_target_eos"],"new_optimizer":0,"generation":0,"teacher":0}))?;
    println!("ORBIT_CONFIRMATION_SEALED skeletons64 rows256 model_calls0");Ok(())
}
fn orbit_authorize(_root:&Path,p:&Plan) -> Result<()> {
    let study=&own(p).study;let _:binary::Value=verify_seal(study)?;
    let prep:binary::Value=read_confirmed(&study.join("preparation.r3b"))?;
    if prep["selection"]!=file_hash(&study.join("selection.r3b"))? {return Err(bad("reviewed selection changed"));}
    for arm in ORBIT_ARMS {
        let root=study.join(arm);let peer=plan_read(&root)?;let h=history(&root,&peer)?;
        if h.last().is_some_and(|s| !orbit_peer_can_proceed(s)) {return Err(bad("orbit cancelled/failed peer"));}
    }
    if !p.tiny {
        let swap:binary::Value=read_confirmed(&study.join("swap-finished.r3b"))?;
        let summary:binary::Value=read_confirmed(&study.join("swap-result.r3b"))?;
        let selection:binary::Value=read(&study.join("selection.r3b"))?;
        if !swap["error"].is_null() || swap["control"]["terminal_reason"]!="COMPLETED"
            || swap["control"]["generation_calls"]!=16 || swap["control"]["teacher_calls"]!=0
            || swap["raw"]!=file_hash(&study.join("swap.r3rows"))? || summary["raw"]!=swap["raw"]
            || summary["identity"]["parent"]!=selection["parent"] || summary["identity"]["policy"]!=selection["parent_policy"] {
            return Err(bad("original swap observation missing/inconsistent"));
        }
    }
    if own(p).arm=="BOTH" {
        let root=study.join("FIXED");let fixed=plan_read(&root)?;let h=history(&root,&fixed)?;
        if h.last().is_none_or(|s|s.step!=fixed.config.max_steps || s.resume || s.phase.as_deref()!=Some("Finished") || s.stop!="BUDGET_REACHED") {
            return Err(bad("normal FIXED endpoint required before BOTH"));
        }
    }
    Ok(())
}
fn orbit_peer_can_proceed(s:&Segment) -> bool {
    ["TRAINING","TIME_BUDGET","BUDGET_REACHED"].contains(&s.stop.as_str())
        && (s.phase.as_deref()!=Some("Finished") || s.stop=="BUDGET_REACHED")
}
pub(in super::super) fn orbit_parity(root:&Path,p:&Plan,reviewer:bool) -> Result<()> {
    if !is_orbit(p) || p.tiny {return Err(bad("SMALL orbit parity scope"));}
    authorize(root,p)?;
    let h=history(root,p)?;let end=h.last().ok_or_else(||bad("orbit endpoint missing"))?;
    if end.step!=512 || end.resume || end.phase.as_deref()!=Some("Finished") {return Err(bad("orbit final endpoint"));}
    let c=verified_corpus(&root.join("corpus.r3cor"),&p.corpus)?;
    let (_,dm,_)=verified_metadata(root,p)?;let tok=ByteBpe::load(&root.join("tokenizer.r3b"))?;
    audit_panel(root,p,512,"dev512",&c.validation,&dm,&tok)?;
    let raw=binary::read_value_records(&root.join("eval-0512-dev512.r3rows"))?;
    let destination=if reviewer {own(p).study.clone()}else{root.to_owned()};
    let name=if reviewer {format!("review-{}",own(p).arm)}else{"parity".into()};
    let identity=binary::record!({"policy":digest(p)?,"checkpoint":end.checkpoint_hash,"selection":"first16 frozen dev rows",
        "reviewer":reviewer,"cases":digest(&&c.validation[..16])?});
    orbit_observe(&destination,&name,&root.join(&end.checkpoint),&c.validation[..16],&identity,Some(&raw[1..17]),observation_control(p,16,0)?)?;
    println!("ORBIT_PARITY arm={} reviewer={reviewer} matched16/16 optimizer0 teacher0",own(p).arm);Ok(())
}
fn orbit_comparison(study:&Path) -> Result<binary::Value> {
    let mut scores=vec![];let mut endpoints=BTreeMap::new();let mut candidates=vec![];
    for arm in ORBIT_ARMS {
        let root=study.join(arm);let p=plan_read(&root)?;
        authorize(&root,&p)?;
        let h=history(&root,&p)?;let last=h.last().ok_or_else(||bad("missing pair endpoint"))?;
        if last.step!=512 || last.resume || last.stop!="BUDGET_REACHED" || last.phase.as_deref()!=Some("Finished") {return Err(bad("pair incomplete"));}
        let tr=orbit_panel(&root,&p,512,"train512")?;let dv=orbit_panel(&root,&p,512,"dev512")?;
        let parity=parity_verified(&root,&p)?;
        if orbit_candidate_scores(&tr,&dv)&&parity {candidates.push((arm,dv.all4,dv.full,h.iter().map(|s|s.elapsed).sum::<f64>()));}
        endpoints.insert(arm,binary::record!({"policy":digest(&p)?,"checkpoint":last.checkpoint_hash,"train":tr,"dev":dv,"parity":parity,
            "exposure":orbit_exposure(&own(&p).rows)?}));
        scores.push(dv);
    }
    let mut gain=0;let mut loss=0;let mut differences=vec![];
    for (a,b) in scores[0].exact.chunks_exact(4).zip(scores[1].exact.chunks_exact(4)) {
        for (&a,&b) in a.iter().zip(b) {gain+=usize::from(!a&&b);loss+=usize::from(a&&!b);}
        differences.push((b.iter().filter(|&&v|v).count() as f64-a.iter().filter(|&&v|v).count() as f64)/4.);
    }
    let mean=differences.iter().sum::<f64>()/128.;
    let se=(differences.iter().map(|d|(d-mean).powi(2)).sum::<f64>()/127./128.).sqrt();
    candidates.sort_by(|a,b|b.1.cmp(&a.1).then(b.2.cmp(&a.2)).then(a.3.total_cmp(&b.3)).then(a.0.cmp(b.0)));
    Ok(binary::record!({"contract":ORBIT_CONTRACT,"source":source_digest()?,"preparation":file_hash(&study.join("preparation.r3b"))?,
        "endpoints":endpoints,"common_dev_gain":gain,"common_dev_loss":loss,"orbit_full_fraction_difference_mean":mean,
        "orbit_unit_standard_error":se,"normal_approx_95_interval":[mean-1.96*se,mean+1.96*se],"independent_units":128,
        "uncertainty_scope":"paired orbit means; normal approximation, one seed; not 512 independent rows",
        "selected":candidates.first().map(|x|x.0),"selection_rule":"eligible only; dev ALL4 then FULL then recorded segment cost then arm name",
        "minimal_baseline_verified":false,"confirmation":"PENDING_ONLY_IF_ELIGIBLE","goal1_ready":false}))
}
pub(in super::super) fn orbit_compare(study:&Path) -> Result<()> {
    let comparison=orbit_comparison(study)?;
    println!("ORBIT_COMPARISON {comparison}");
    Ok(())
}
pub(in super::super) fn orbit_confirm(study:&Path) -> Result<()> {
    let comparison=orbit_comparison(study)?;
    let arm=comparison["selected"].as_str().ok_or_else(||bad("NOT_RUN_PREREQUISITE: no baseline candidate"))?;
    let review:binary::Value=read_confirmed(&study.join("review-b.r3b"))?;
    if review["verdict"]!="PASS" || review["preparation"]!=file_hash(&study.join("preparation.r3b"))?
        || review["endpoints"]!=comparison["endpoints"]
        || review["report_hash"]!=file_hash(Path::new(review["report_path"].as_str().ok_or_else(||bad("result review path"))?))? {
        return Err(bad("independent endpoint recount required"));
    }
    let root=study.join(arm);let p=plan_read(&root)?;let h=history(&root,&p)?;let end=h.last().unwrap();
    let seal=verify_seal(study)?;
    // Candidate is fixed durably before reading the sealed examples or answers.
    write(&study.join("confirmation-candidate.r3b"),&binary::record!({"arm":arm,"checkpoint":end.checkpoint_hash,
        "comparison":comparison,"seal":file_hash(&study.join("confirmation-seal.r3b"))?,"review":file_hash(&study.join("review-b.r3b"))?}))?;
    let c=verified_corpus(&study.join("sealed/confirmation.r3cor"),seal["corpus"].as_str().ok_or_else(||bad("seal corpus"))?)?;
    let ms:Vec<Meta>=read(&study.join("sealed/metadata.r3b"))?;let tok=ByteBpe::load(&root.join("tokenizer.r3b"))?;
    orbit_validate(&c.validation,&ms,&tok,256)?;
    let identity=binary::record!({"policy":digest(&p)?,"checkpoint":end.checkpoint_hash,"candidate":file_hash(&study.join("confirmation-candidate.r3b"))?});
    let rows=orbit_observe(study,"confirmation",&root.join(&end.checkpoint),&c.validation,&identity,None,observation_control(&p,256,0)?)?;
    let s=orbit_score(&c.validation,&ms,&rows,&tok)?;
    let pass=s.full>=244&&s.query_both>=116&&s.all4>=58&&s.errors==0&&s.eos==256;
    publish_confirmed(&study.join("confirmation-result.r3b"),&binary::record!({"arm":arm,"checkpoint":end.checkpoint_hash,
        "score":s,"minimal_binding_baseline_verified":pass,"scope":"K1-V finite digits; two records; new key/value sets",
        "goal1_ready":false,"s4":false,"s5":false,"s6":false}))?;
    println!("ORBIT_CONFIRMATION arm={arm} full={} query_both={} all4={} errors={} minimal_baseline={pass} GOAL1=false",s.full,s.query_both,s.all4,s.errors);Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn orbit_fixture(root:&Path) -> Result<PathBuf> {
        let parent=parent(root)?;let old=root.join("old");
        prepare(&parent,&old,true)?;
        let study=root.join("new");
        orbit_prepare(&old.join("A"),&study,true)?;
        orbit_seal(&study)?;fixture_review(&study)?;
        Ok(study)
    }
    #[test]
    fn foundation_t1_t2_t3_values_split_and_finite_tape() -> Result<()> {
        let temp=tempfile::tempdir()?;let study=orbit_fixture(temp.path())?;
        let p=plan_read(&study.join("FIXED"))?;
        let tok=ByteBpe::load(&study.join("FIXED/tokenizer.r3b"))?;
        let c=verified_corpus(&study.join("FIXED/corpus.r3cor"),&p.corpus)?;
        let (tm,_,_)=verified_metadata(&study.join("FIXED"),&p)?;
        assert_eq!(universe().len(),1260);
        for quad in c.train.chunks_exact(4) {
            verify_swap(&quad[0],&quad[2],&tok)?;verify_swap(&quad[1],&quad[3],&tok)?;
        }
        let mut missing=c.train[0].clone();missing.request.input.clear();assert!(swap_values(&missing).is_err());
        let mut same=c.train[0].clone();same.request.evidence.items[1]=same.request.evidence.items[0].clone();assert!(swap_values(&same).is_err());
        let mut half=c.train[2].clone();half.request.evidence.items[1]=c.train[0].request.evidence.items[1].clone();
        assert!(verify_swap(&c.train[0],&half,&tok).is_err());
        let mut id=c.train[2].clone();id.request.evidence.items[0].event_id+=1;
        assert!(verify_swap(&c.train[0],&id,&tok).is_err());
        let mut reordered=c.train[0].clone();reordered.id="different-id".into();reordered.request.evidence.items.reverse();
        assert_eq!(semantic_skeleton(&reordered)?,semantic_skeleton(&c.train[0])?);
        let mut broken=tm.clone();broken[2].base="another-group".into();
        assert!(orbit_validate(&c.train,&broken,&tok,512).is_err());
        let select:binary::Value=read(&study.join("selection.r3b"))?;
        let tr:Vec<Skeleton>=binary::from_value(select["train"].clone())?;
        let dv:Vec<Skeleton>=binary::from_value(select["dev"].clone())?;
        assert!(select["confirmation"].is_null());
        let cf=heldout_skeletons(&[tr.clone(),dv.clone()].concat(),64);
        assert_eq!((tr.len(),dv.len(),cf.len()),(128,128,64));
        assert!(tr.iter().all(|s|!dv.contains(s)&&!cf.contains(s)));assert!(dv.iter().all(|s|!cf.contains(s)));
        let frames=samples(&c.train,&tok,256)?;
        let mut observed=vec![];
        for arm in ORBIT_ARMS {
            let rows=orbit_rows(arm,512);let exposure=orbit_exposure(&rows)?;
            let count:Vec<usize>=binary::from_value(exposure["per_id_count"].clone())?;
            assert_eq!(count.iter().sum::<usize>(),4096);
            assert_eq!(count.iter().filter(|&&n|n>0).count(),if arm=="FIXED"{256}else{512});
            assert!(count.iter().enumerate().all(|(i,&n)|n==if arm=="FIXED"{if i%4<2{16}else{0}}else{8}));
            let visits:Vec<usize>=binary::from_value(exposure["per_skeleton_visits"].clone())?;
            assert!(visits.iter().all(|&v|v==16));
            let input=rows.iter().flatten().map(|&i|frames[i].tokens.len()-1).sum::<usize>();
            observed.push(input);
            let path=study.join(format!("{arm}-512.r3rows"));
            let records=rows.iter().enumerate().map(|(i,r)|binary::record!({"step":i+1,"draw":r})).collect::<Vec<_>>();
            replica_v3::codec::publish_new(&path,|f,_|{for r in &records{binary::write_value_record(f,r)?;}Ok(())})?;
            assert_eq!(binary::read_value_records(&path)?,records);
            assert_eq!(records.last().unwrap()["step"],512);
            let mut bad=rows.clone();bad[511][0]=512;assert!(orbit_exposure(&bad).is_err());
            let mut invalid=p.clone();invalid.identifiable.as_mut().unwrap().rows[0][0]^=2;
            assert!(orbit_verify_plan(&study.join("FIXED"),&invalid).is_err());
            let b=batch(&frames,&rows[511],&Device::Cpu)?;
            let input=b.input.to_vec2::<u32>()?;let targets=b.target.to_vec2::<u32>()?;let masks=b.mask.to_vec2::<f32>()?;
            for (j,&i) in rows[511].iter().enumerate() {
                assert_eq!(&input[j][..frames[i].tokens.len()-1],&frames[i].tokens[..frames[i].tokens.len()-1]);
                assert_eq!(&targets[j][..frames[i].tokens.len()-1],&frames[i].tokens[1..]);
                assert_eq!(masks[j].iter().sum::<f32>(),2.);
                assert_eq!(targets[j][frames[i].tokens.len()-2],EOS);
                if arm=="FIXED"{assert!(i%4<2);}
            }
        }
        assert_eq!(observed[0],observed[1]);
        let fixed=orbit_rows("FIXED",512);let both=orbit_rows("BOTH",512);
        assert_eq!(fixed.iter().flatten().zip(both.iter().flatten()).filter(|(a,b)|a!=b).count(),2048);
        println!("FOUNDATION_T1_T2_T3 native pool512 split128/128/64 FIXED256x16 BOTH512x8 equal_input={} target8192 optimizer0 generation0 teacher0",observed[0]);
        Ok(())
    }
    #[test]
    fn foundation_t5_strict_orbit_scores_and_seen_mask() -> Result<()> {
        let temp=tempfile::tempdir()?;let study=orbit_fixture(temp.path())?;
        let p=plan_read(&study.join("FIXED"))?;let tok=ByteBpe::load(&study.join("FIXED/tokenizer.r3b"))?;
        let c=verified_corpus(&study.join("FIXED/corpus.r3cor"),&p.corpus)?;let (tm,_,_)=verified_metadata(&study.join("FIXED"),&p)?;
        let es=&c.train[..8];let ms=&tm[..8];
        let make=|e:&Episode,text:&str,finish:&str|->Result<binary::Value>{
            let mut ids=tok.encode(text.as_bytes())?;if finish=="stop"{ids.push(EOS);}
            Ok(binary::record!({"id":e.id,"expected":e.answer,"question":e.request.input,"generated_evidence":e.request.evidence,
                "actual":text,"raw_tokens":ids,"finish_reason":finish,"generation_completed":true,"error":null,
                "exact_match":text==e.answer && finish=="stop"}))
        };
        let correct=es.iter().map(|e|make(e,&e.answer,"stop")).collect::<Result<Vec<_>>>()?;
        let s=orbit_score(es,ms,&correct,&tok)?;assert_eq!((s.full,s.query_both,s.swap_both,s.all4),(8,4,4,2));
        let mut mixed=correct.clone();
        mixed[1]=make(&es[1],&orbit_foil(&es[1])?,"stop")?;
        mixed[4]=make(&es[4],"x","stop")?;
        mixed[5]=make(&es[5],&es[5].answer,"length")?;
        let s=orbit_score(es,ms,&mixed,&tok)?;
        assert_eq!((s.full,s.query_both,s.swap_both,s.all4),(5,2,1,0));
        assert_eq!(s.gold_foil_other_malformed,[5,1,0,2]);
        assert_eq!(s.exact.iter().enumerate().filter(|(i,_)|i%4<2).filter(|(_,v)|**v).count(),1);
        assert!(orbit_score(es,ms,&mixed[..7],&tok).is_err());
        let mut lie=mixed.clone();lie[1]["exact_match"]=binary::record!(true);
        assert!(orbit_score(es,ms,&lie,&tok).is_err());
        assert!(!orbit_candidate_scores(&s,&s));
        println!("FOUNDATION_T5 independent literal full/query/swap/all4/classes/seen/incomplete PASS model_calls0");Ok(())
    }
    #[test]
    fn foundation_t4_t6_native_process_resume_and_peer_authorization() -> Result<()> {
        const CHILD:&str="R3_FOUNDATION_PROCESS_CHILD";
        if let Ok(root)=std::env::var(CHILD) {return run(Path::new(&root),std::env::var("R3_FOUNDATION_CONTINUOUS").as_deref()==Ok("1"));}
        let temp=tempfile::tempdir()?;
        for name in ["continuous","segmented"] {let base=temp.path().join(name);std::fs::create_dir(&base)?;orbit_fixture(&base)?;}
        let mut total=(0,0,0);
        for arm in ORBIT_ARMS {
            let mut ends=vec![];
            for name in ["continuous","segmented"] {
                let study=temp.path().join(name).join("new");let root=study.join(arm);let p=plan_read(&root)?;
                if arm=="FIXED" {assert!(authorize(&study.join("BOTH"),&plan_read(&study.join("BOTH"))?).is_err());}
                for _ in 0..if name=="continuous"{1}else{2} {
                    let out=std::process::Command::new(std::env::current_exe()?).args(["--exact","training::fresh::identifiable::binding::tests::foundation_t4_t6_native_process_resume_and_peer_authorization","--nocapture"])
                        .env(CHILD,&root).env("R3_FOUNDATION_CONTINUOUS",if name=="continuous"{"1"}else{"0"})
                        .env("VECLIB_MAXIMUM_THREADS","1").env("OMP_NUM_THREADS","1").output()?;
                    print!("{}",String::from_utf8_lossy(&out.stdout));
                    assert!(out.status.success(),"{}",String::from_utf8_lossy(&out.stderr));
                    assert!(String::from_utf8_lossy(&out.stdout).contains("1 passed"));
                }
                let h=history(&root,&p)?;let end=h.last().unwrap();
                assert_eq!(end.step,2);assert!(!end.resume);assert!(orbit_peer_can_proceed(end));
                let mut stop=end.clone();stop.stop="CANCELLED".into();assert!(!orbit_peer_can_proceed(&stop));
                stop.stop="UNKNOWN".into();assert!(!orbit_peer_can_proceed(&stop));
                assert!(!gate(&root,&p)?);
                if arm=="FIXED" {authorize(&study.join("BOTH"),&plan_read(&study.join("BOTH"))?)?;}
                let l=checkpoint::load(&root.join(&end.checkpoint),Device::Cpu,true)?;
                let state=l.manifest.training.as_ref().unwrap();
                ends.push((l.model.weights_content_id()?,optimizer_hash(&l.optimizer)?,state.step,state.sampler_state,state.consumed_tokens,state.target_tokens));
                total.0+=state.step;total.1+=h.iter().map(|s|s.generations).sum::<usize>();total.2+=h.iter().map(|s|s.teachers).sum::<usize>();
                report(&root)?;
            }
            assert_eq!(ends[0],ends[1]);
        }
        assert_eq!(total,(8,64,64));
        let study=temp.path().join("continuous/new");let fixed=plan_read(&study.join("FIXED"))?;
        // A started, unreturned later segment must not become a fresh attempt.
        write(&study.join("BOTH/segment-0001-started.r3b"),&binary::record!({"started":true}))?;
        assert!(authorize(&study.join("FIXED"),&fixed).unwrap_err().to_string().contains("UNKNOWN"));
        println!("FOUNDATION_T4_T6 continuous2 vs newprocess1+1 both arms weights/Adam/clock/tape SAME; low-quality FIXED permits BOTH, cancelled/UNKNOWN BLOCK; optimizer={} generation={} teacher={}",total.0,total.1,total.2);
        Ok(())
    }
    fn parent(root: &Path) -> Result<PathBuf> {
        let p = root.join("parent");
        super::super::super::prepare(&p, true)?;
        Ok(p)
    }
    fn fixture_review(study: &Path) -> Result<()> {
        let path = study.join("fixture-review.txt");
        std::fs::write(&path, b"TINY fixture authorization; no SMALL approval")?;
        publish_confirmed(
            &study.join("review-a.r3b"),
            &binary::record!({"verdict":"PASS","preparation":file_hash(&study.join("preparation.r3b"))?,"source":source_digest()?,"report_hash":file_hash(&path)?,"report_path":path}),
        )
    }
    #[test]
    fn binding_data_tape_native_batch_contract() -> Result<()> {
        let temp = tempfile::tempdir()?;
        let parent = parent(temp.path())?;
        let tok = ByteBpe::load(&parent.join("tokenizer.r3b"))?;
        assert_eq!(universe().len() * 2, 2520);
        assert_eq!(skeletons().len(), 128);
        let study = temp.path().join("study");
        prepare(&parent, &study, true)?;
        let mut hashes = BTreeSet::new();
        let mut previous = None;
        for arm in ARMS {
            let root = study.join(arm);
            let p = plan_read(&root)?;
            let l = checkpoint::load(&root.join("initial.r3m"), Device::Cpu, false)?;
            hashes.insert(l.model.weights_content_id()?);
            let c = verified_corpus(&root.join("corpus.r3cor"), &p.corpus)?;
            let (tm, dm, _) = verified_metadata(&root, &p)?;
            let frames = samples(&c.train, &tok, 256)?;
            let b = batch(&frames, &tape(1)[0], &Device::Cpu)?;
            let input = b.input.to_vec2::<u32>()?;
            let labels = b.target.to_vec2::<u32>()?;
            let masks = b.mask.to_vec2::<f32>()?;
            let first = b.first_target_mask.to_vec2::<f32>()?;
            for (row, &i) in tape(1)[0].iter().enumerate() {
                let s = &frames[i];
                assert_eq!(
                    &input[row][..s.tokens.len() - 1],
                    &s.tokens[..s.tokens.len() - 1]
                );
                assert_eq!(&labels[row][..s.tokens.len() - 1], &s.tokens[1..]);
                assert_eq!(
                    masks[row].iter().sum::<f32>() as usize,
                    s.tokens.len() - s.response_start
                );
                assert_eq!(first[row].iter().sum::<f32>(), 1.);
                assert_eq!(labels[row][s.tokens.len() - 2], EOS);
                for (j, &m) in masks[row].iter().enumerate() {
                    assert_eq!(
                        m,
                        if j + 1 >= s.response_start && j + 1 < s.tokens.len() {
                            1.
                        } else {
                            0.
                        }
                    );
                }
            }
            let latent = c
                .train
                .iter()
                .chain(&c.validation)
                .map(|e| {
                    e.request
                        .evidence
                        .items
                        .iter()
                        .map(|r| {
                            Ok((
                                r.event_id,
                                parsed_record(r)?.2.to_string(),
                                r.source.clone(),
                                r.observed_at,
                                r.recorded_at,
                            ))
                        })
                        .collect::<Result<Vec<_>>>()
                })
                .collect::<Result<Vec<_>>>()?;
            if let Some(old) = &previous {
                assert_eq!(&latent, old);
            }
            previous = Some(latent);
            let mut wrong = c.train.clone();
            wrong[0].answer = wrong[1].answer.clone();
            assert!(validate(&wrong, &tm, &tok, 256).is_err());
            let mut missing = c.train.clone();
            missing[0].request.input.clear();
            assert!(validate(&missing, &tm, &tok, 256).is_err());
            let mut q = c.train[0].request.clone();
            q.evidence.items.reverse();
            assert_eq!(resolve_request(&q)?, c.train[0].answer);
            assert_ne!(digest(&c.train)?, digest(&c.validation)?);
            let a: BTreeSet<String> =
                binary::from_value(validate(&c.train, &tm, &tok, 256)?["scenes"].clone())?;
            let d: BTreeSet<String> =
                binary::from_value(validate(&c.validation, &dm, &tok, 256)?["scenes"].clone())?;
            assert!(a.is_disjoint(&d));
        }
        assert_eq!(hashes.len(), 1);
        fixture_review(&study)?;
        let tiny = plan_read(&study.join("A"))?;
        assert!(!gate(&study.join("A"), &tiny)?);
        assert!(parity(&study.join("A")).is_err());
        assert!(authorize(&study.join("B"), &plan_read(&study.join("B"))?).is_err());
        let mut exhausted = tiny;
        exhausted.evaluation.generation_limit = 0;
        assert!(observation_control(&exhausted, 16, 0).is_err());
        let mut p: Plan = read(&study.join("A/plan.r3b"))?;
        p.config = config(false);
        let rows = tape(512);
        let trace=rows.iter().enumerate().map(|(i,r)|binary::record!({"step":i+1,"draw":r,"lr":p.learning_rate(i+1),"lr_bits":p.learning_rate(i+1).to_bits()})).collect::<Vec<_>>();
        let path = study.join("tape512.r3b");
        publish_confirmed(&path, &trace)?;
        let restored: Vec<binary::Value> = read_confirmed(&path)?;
        assert_eq!(restored, trace);
        let stream = study.join("updates512.r3rows");
        replica_v3::codec::publish_new(&stream, |f, _| {
            for row in &trace {
                binary::write_value_record(f, row)?;
            }
            Ok(())
        })?;
        assert_eq!(binary::read_value_records(&stream)?, trace);
        let mut malformed = std::fs::read(&stream)?;
        malformed.pop();
        let broken = study.join("truncated.r3rows");
        std::fs::write(&broken, &malformed)?;
        assert!(binary::read_value_records(&broken).is_err());
        let mut exposures = [0; 256];
        for (epoch, chunk) in rows.chunks_exact(32).enumerate() {
            let mut seen = BTreeSet::new();
            for row in chunk {
                for pair in row.chunks_exact(2) {
                    assert_eq!(pair[0] % 2, 0);
                    assert_eq!(pair[1], pair[0] + 1);
                    seen.insert(pair[0] / 2);
                }
                for &i in row {
                    exposures[i] += 1;
                }
            }
            assert_eq!(seen.len(), 128);
            assert_eq!(p.epoch((epoch + 1) * 32), epoch + 1);
        }
        assert!(exposures.iter().all(|&n| n == 16));
        assert_eq!(p.learning_rate(1).to_bits(), (3e-4f64 / 32.).to_bits());
        assert_eq!(p.learning_rate(32), 3e-4);
        assert_eq!(p.learning_rate(512), 3e-4);
        println!(
            "BINDING_DATA all4x(train256/dev256/probe128) label/frame/native=PASS batch=8 tape=512 updates=0 generations=0 teachers=0"
        );
        Ok(())
    }
    #[test]
    fn binding_tiny_continuous_vs_new_process_resume() -> Result<()> {
        const CHILD: &str = "R3_BINDING_RESUME_CHILD";
        if let Ok(root) = std::env::var(CHILD) {
            return run(
                Path::new(&root),
                std::env::var("R3_BINDING_CONTINUOUS").as_deref() == Ok("1"),
            );
        }
        let temp = tempfile::tempdir()?;
        let parent = parent(temp.path())?;
        for name in ["continuous", "segmented"] {
            let study = temp.path().join(name);
            prepare(&parent, &study, true)?;
            fixture_review(&study)?;
        }
        for (name, continuous) in [
            ("continuous", true),
            ("segmented", false),
            ("segmented", false),
        ] {
            let out=std::process::Command::new(std::env::current_exe()?).args(["--exact","training::fresh::identifiable::binding::tests::binding_tiny_continuous_vs_new_process_resume","--nocapture"]).env(CHILD,temp.path().join(name).join("A")).env("R3_BINDING_CONTINUOUS",if continuous{"1"}else{"0"}).env("VECLIB_MAXIMUM_THREADS","1").env("OMP_NUM_THREADS","1").output()?;
            print!("{}", String::from_utf8_lossy(&out.stdout));
            assert!(
                out.status.success(),
                "{}",
                String::from_utf8_lossy(&out.stderr)
            );
            assert!(String::from_utf8_lossy(&out.stdout).contains("1 passed"));
        }
        let mut ends = vec![];
        let mut total = (0, 0, 0);
        for name in ["continuous", "segmented"] {
            let root = temp.path().join(name).join("A");
            let p = plan_read(&root)?;
            let h = history(&root, &p)?;
            let end = h.last().unwrap();
            assert_eq!(end.step, 2);
            assert!(!end.resume);
            for step in [0, 2] {
                for name in ["train4", "dev4"] {
                    let rows = binary::read_value_records(
                        &root.join(format!("eval-{step:04}-{name}-teachers.r3rows")),
                    )?;
                    for r in &rows[1..] {
                        let t = &r["teacher"];
                        let v = &t["target_token_observation"];
                        let nll: Vec<f64> = binary::from_value(v["nll"].clone())?;
                        assert_eq!(nll.len(), 2);
                        assert_eq!(v["gold"][1], EOS);
                        assert!(
                            (nll.iter().sum::<f64>() / 2. - t["mean_nll"].as_f64().unwrap()).abs()
                                < 1e-12
                        );
                    }
                }
            }
            let trace = binary::read_value_records(&root.join("segment-0000/updates.r3rows"))?;
            for task in trace[0]["tasks"].as_array().unwrap() {
                let nll = task["target_token_observation"].as_array().unwrap();
                assert_eq!(nll.len(), 2);
                let mean = nll.iter().map(|v| v["nll"].as_f64().unwrap()).sum::<f64>() / 2.;
                assert!((mean - task["ce"].as_f64().unwrap()).abs() < 1e-5);
            }
            let l = checkpoint::load(&root.join(&end.checkpoint), Device::Cpu, true)?;
            let initial = checkpoint::load(&root.join("initial.r3m"), Device::Cpu, false)?;
            assert_ne!(
                l.model.weights_content_id()?,
                initial.model.weights_content_id()?
            );
            assert_ne!(
                optimizer_hash(&l.optimizer)?,
                optimizer_hash(&Adam::new(&initial.model.vars)?.moments)?
            );
            let s = l.manifest.training.unwrap();
            assert_eq!(s.step, 2);
            total.0 += s.step;
            total.1 += h.iter().map(|s| s.generations).sum::<usize>();
            total.2 += h.iter().map(|s| s.teachers).sum::<usize>();
            ends.push((
                l.model.weights_content_id()?,
                optimizer_hash(&l.optimizer)?,
                s.step,
                s.sampler_state,
                s.consumed_tokens,
                s.target_tokens,
            ));
            report(&root)?;
        }
        assert_eq!(ends[0], ends[1]);
        assert_eq!(total, (4, 32, 32));
        println!(
            "BINDING_TINY_RESUME weights/Adam/clock/cursor/input/target=IDENTICAL optimizer={} generation={} teacher={}",
            total.0, total.1, total.2
        );
        Ok(())
    }
}
