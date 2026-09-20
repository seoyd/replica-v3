//! Training-only finite selection study. Reuses fresh plans, native corpora,
//! the decoder/trainer, immutable call receipts and the strict panel scorer.
use super::*;
const DATA: &str = "binding-learnability-v1";
const CONTRACT_ID: &str = "R3-BINDING-LEARNABILITY-1.0";
const ARMS: [&str; 4] = ["A", "B", "C", "D"];
const SHORT_SYSTEM: &str = "근거에 따라 답하라.";
const VALUE_QUERY: &str = "현재 값의 숫자 하나만 답하라.";
const CITATION_QUERY: &str = "현재 값을 인용과 함께 써라.";
type Skeleton = [u8; 4];

pub(in super::super) fn is(p: &Plan) -> bool {
    p.identifiable.as_ref().is_some_and(|x| x.dataset == DATA)
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
    let nt = if p.tiny {
        4
    } else if step == 512 {
        256
    } else {
        32
    };
    let nd = if p.tiny {
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
    for arm in ARMS {
        let r = study.join(arm);
        let plan = plan_read(&r)?;
        let h = history(&r, &plan)?;
        if h.last().is_some_and(|s| {
            !s.resume && (s.step != plan.config.max_steps || s.phase.as_deref() != Some("Finished"))
        }) {
            return Err(bad("binding failed peer"));
        }
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
    for arm in ARMS {
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
    Ok(out)
}
pub(in super::super) fn usage(p: &Plan) -> Result<(f64, usize, usize)> {
    let w = work(p)?;
    Ok((w.0, w.1, w.2))
}
pub(in super::super) fn remaining(p: &Plan, target: bool) -> Result<u64> {
    let w = work(p)?;
    let (cap, n) = if target {
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
            if ["A", "B"].contains(&own(&p).arm.as_str()) {
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
    let mut exposure = [0usize; 256];
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
            .chunks_exact(2)
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
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
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
