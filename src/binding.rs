//! Training-only finite selection study. Reuses fresh plans, native corpora,
//! the decoder/trainer, immutable call receipts and the strict panel scorer.
use super::*;
#[path = "value_citation.rs"]
pub(in super::super) mod citation;
const DATA: &str = "binding-learnability-v1";
const CONTRACT_ID: &str = "R3-BINDING-LEARNABILITY-1.0";
const ARMS: [&str; 4] = ["A", "B", "C", "D"];
const ORBIT_DATA: &str = "foundation-orbit-v1";
const ORBIT_CONTRACT: &str = "R3-FOUNDATION-ORBIT-1.0";
const ORBIT_ARMS: [&str; 2] = ["FIXED", "BOTH"];
const FRAME_DATA: &str = "causal-framing-v1";
const FRAME_CONTRACT: &str = "R3-CAUSAL-FRAMING-BASELINE-1.0";
const FRAME_ARMS: [&str; 2] = ["QE", "EQ"];
const SIGNAL_DATA: &str = "query-signal-convergence-v1";
const SIGNAL_CONTRACT: &str = "R3-QUERY-SIGNAL-CONVERGENCE-1.0";
const EXPANSION_DATA: &str = "learned-binding-expansion-v1";
const EXPANSION_CONTRACT: &str = "R3-LEARNED-BINDING-EXPANSION-1.0";
const EXPANSION_ARMS: [&str; 2] = ["REPEAT", "REBIND"];
const CONSOLIDATION_DATA: &str = "rebind-consolidation-v1";
const CONSOLIDATION_CONTRACT: &str = "R3-REBIND-CONSOLIDATION-1.0";
const CONTINUE_ARM: &str = "REBIND-CONTINUE";
const SHORT_SYSTEM: &str = "근거에 따라 답하라.";
const VALUE_QUERY: &str = "현재 값의 숫자 하나만 답하라.";
const CITATION_QUERY: &str = "현재 값을 인용과 함께 써라.";
type Skeleton = [u8; 4];

pub(in super::super) fn is(p: &Plan) -> bool {
    citation::is(p) || p.identifiable
        .as_ref()
        .is_some_and(|x| [DATA, ORBIT_DATA, FRAME_DATA, SIGNAL_DATA, EXPANSION_DATA, CONSOLIDATION_DATA].contains(&x.dataset.as_str()))
}
pub(in super::super) fn is_consolidation(p: &Plan) -> bool {
    p.identifiable.as_ref().is_some_and(|x| x.dataset == CONSOLIDATION_DATA)
}
pub(in super::super) fn is_expansion(p: &Plan) -> bool {
    p.identifiable.as_ref().is_some_and(|x| x.dataset == EXPANSION_DATA)
}
pub(in super::super) fn is_signal(p: &Plan) -> bool {
    p.identifiable.as_ref().is_some_and(|x| x.dataset == SIGNAL_DATA)
}
pub(in super::super) fn is_framing(p: &Plan) -> bool {
    p.identifiable
        .as_ref()
        .is_some_and(|x| x.dataset == FRAME_DATA)
}
fn is_orbit(p: &Plan) -> bool {
    citation::is(p) || is_consolidation(p) || is_expansion(p) || is_signal(p) || is_framing(p)
        || p.identifiable
            .as_ref()
            .is_some_and(|x| x.dataset == ORBIT_DATA)
}
fn arms(p: &Plan) -> &'static [&'static str] {
    if citation::is(p) { return citation::arms(p); }
    if is_consolidation(p) {
        &[CONTINUE_ARM]
    } else if is_expansion(p) {
        &EXPANSION_ARMS
    } else if is_signal(p) {
        &["QE"]
    } else if is_framing(p) {
        &FRAME_ARMS
    } else if is_orbit(p) {
        &ORBIT_ARMS
    } else {
        &ARMS
    }
}
pub(in super::super) fn evaluation_for(p: &Plan) -> EvaluationPolicy {
    if citation::is(p) { return citation::evaluation(p); }
    let mut e = evaluation(p.tiny);
    if is_consolidation(p) {
        e.screen_steps.clear();
        e.train_steps = if p.tiny { vec![4] } else { vec![3840, 4352, 5120] };
        e.generation_limit = 4800;
        e.teacher_limit = 4608;
        e.active_seconds = 7200;
        return e;
    }
    if is_expansion(p) {
        e.screen_steps.clear();
        e.train_steps = vec![2304, 2816, 3584];
        e.generation_limit = 5504;
        e.teacher_limit = 5120;
        e.active_seconds = 10800;
        return e;
    }
    if is_signal(p) {
        e.screen_steps.clear();
        e.train_steps = if p.tiny {vec![4]} else {vec![768,1024,1536,2048]};
        e.generation_limit = 2816;
        e.teacher_limit = 3072;
        e.active_seconds = 7200;
        return e;
    }
    if is_orbit(p) {
        e.teacher_limit = 3072;
        e.primary_min = 488;
    }
    if is_framing(p) {
        e.teacher_limit = 5120;
        e.generation_limit = 5632;
        e.train_steps.push(if p.tiny { 4 } else { 1024 });
    }
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
            framing: None,
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
    if citation::is(p) { return citation::verify_plan(root,p); }
    if is_consolidation(p) { return consolidation_verify_plan(root, p); }
    if is_expansion(p) { return expansion_verify_plan(root, p); }
    if is_signal(p) { return signal_verify_plan(root, p); }
    if is_framing(p) {
        return framing_verify_plan(root, p);
    }
    if p.framing.is_some() {
        return Err(bad("legacy plan cannot select a new framing"));
    }
    if is_orbit(p) {
        return orbit_verify_plan(root, p);
    }
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
    if citation::is(p) { return citation::panels(root,p,step); }
    let c = verified_corpus(&root.join("corpus.r3cor"), &p.corpus)?;
    let (tm, dm, _) = verified_metadata(root, p)?;
    if is_consolidation(p) {
        if !p.evaluation.train_steps.contains(&step) {
            return Err(bad("unsupported consolidation evaluation step"));
        }
        let (old, new) = if p.tiny { (4, 4) } else if step == 3840 { (64, 64) } else { (512, 1024) };
        let start = if p.tiny { 0 } else { 512 };
        if c.train.len() != if p.tiny {512} else {1536} || c.validation.len() != 512
            || tm.len() != c.train.len() || dm.len() != 512 {
            return Err(bad("consolidation complete frozen pool required"));
        }
        return Ok(vec![(format!("old{old}"), c.train[..old].to_vec(), tm[..old].to_vec()),
            (format!("new{new}"), c.train[start..start+new].to_vec(), tm[start..start+new].to_vec()),
            (format!("dev{old}"), c.validation[..old].to_vec(), dm[..old].to_vec())]);
    }
    if is_expansion(p) {
        let final_step = step == p.config.max_steps;
        let old = if final_step {512} else {64};
        let new = if final_step {1024} else {64};
        if c.train.len()!=1536 || tm.len()!=1536 || c.validation.len()!=512 || dm.len()!=512 {
            return Err(bad("expansion complete pool required"));
        }
        return Ok(vec![(format!("old{old}"), c.train[..old].to_vec(), tm[..old].to_vec()),
            (format!("new{new}"), c.train[512..512+new].to_vec(), tm[512..512+new].to_vec()),
            (format!("dev{old}"), c.validation[..old].to_vec(), dm[..old].to_vec())]);
    }
    let nt = if is_orbit(p) {
        if p.tiny {
            4
        } else if step >= 512 && (!is_signal(p) || [1024,2048].contains(&step)) {
            512
        } else {
            64
        }
    } else if p.tiny {
        4
    } else if step == 512 {
        256
    } else {
        32
    };
    let nd = if is_orbit(p) {
        if p.tiny {
            4
        } else if step >= 512 && (!is_signal(p) || [1024,2048].contains(&step)) {
            512
        } else {
            64
        }
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
    if citation::is(p) { return citation::evaluate(p,root,path,step,control); }
    for (name, es, ms) in panel_cases(root, p, step)? {
        evaluate_panel(p, root, path, step, &name, &es, &ms, control)?;
    }
    if is_consolidation(p) { return consolidation_decision(root, p, step); }
    if is_signal(p) { return signal_evaluation_decision(root, p, step); }
    if is_expansion(p) { return expansion_evaluation_decision(root, p, step); }
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
    if citation::is(p) { return citation::gate(root,p); }
    if is_consolidation(p) {
        let h = history(root, p)?;
        return Ok(h.last().is_some_and(|s| [4352,5120].contains(&s.step)
            && s.stop == format!("CANDIDATE_FIXED_AT_{}", s.step) && !s.resume
            && s.phase.as_deref() == Some("Finished")));
    }
    if is_expansion(p) {
        let h=history(root,p)?;
        return Ok(h.last().is_some_and(|s| s.step==3584 && s.stop=="CANDIDATE_FIXED" && !s.resume));
    }
    if is_signal(p) {
        let h = history(root,p)?;
        return Ok(h.last().is_some_and(|s| s.stop == "CANDIDATE_FIXED" && !s.resume));
    }
    if is_framing(p) {
        let h = history(root, p)?;
        let Some(s) = h.last() else { return Ok(false) };
        return Ok(!p.tiny
            && [512, 1024].contains(&s.step)
            && orbit_candidate_scores(
                &orbit_panel(root, p, s.step, "train512")?,
                &orbit_panel(root, p, s.step, "dev512")?,
            ));
    }
    if is_orbit(p) {
        return orbit_gate(root, p);
    }
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
    if citation::is(p) { return citation::authorize(root,p); }
    if is_consolidation(p) {
        if root.canonicalize()? != study.join(CONTINUE_ARM).canonicalize()? {
            return Err(bad("consolidation registered root"));
        }
        consolidation_parent_receipt(p)?;
        return Ok(());
    }
    if is_signal(p) { return signal_authorize(root,p); }
    if is_expansion(p) { return expansion_authorize(root,p); }
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
    if is_framing(p) {
        return framing_authorize(root, p);
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
        for name in [
            "swap",
            "legacy",
            "confirmation",
            "review-FIXED",
            "review-BOTH",
            "review-QE",
            "review-EQ",
            "review-REPEAT", "review-REBIND", "parent-new", "review-parent", "review-REBIND-CONTINUE",
            "citation-parent", "citation-review-value", "citation-review-citation",
            "citation-parity-value", "citation-parity-citation",
            "mean-parent", "mean-review-TOKEN-CONTROL-value", "mean-review-TOKEN-CONTROL-citation",
            "mean-review-ANSWER-MEAN-value", "mean-review-ANSWER-MEAN-citation",
            "continuation-parent-value", "continuation-parent-citation",
            "qa-parent-value", "qa-parent-citation", "qa-parent-balanced", "qa-parent-transfer",
            "qa-review-old_qa", "qa-review-balanced",
            "qa-factor-value", "qa-factor-citation", "qa-factor-S1Q0", "qa-factor-S0Q1", "qa-factor-S1Q1",
            "bridge-review-normal", "bridge-review-errors", "bridge-qa-primary", "bridge-qa-transfer", "bridge-parent-parity",
        ] {
            let study = &own(p).study;
            if study.join(format!("{name}-started.r3b")).exists() {
                if ["confirmation","bridge-qa-primary","bridge-qa-transfer"].contains(&name) && read::<binary::Value>(&study.join(format!("{name}-started.r3b")))?["segments"] == 1 {
                    let (elapsed, calls, _) = segmented_usage(study,name)?;
                    out.0 += elapsed; out.1 += calls;
                    continue;
                }
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
    if citation::instruction_bridge(p) {
        let probe=citation::bridge_probe_usage(p)?;out.0+=probe.0;out.1+=probe.1;out.2+=probe.2;
    }
    Ok(out)
}
pub(in super::super) fn usage(p: &Plan) -> Result<(f64, usize, usize)> {
    let w = work(p)?;
    Ok((w.0, w.1, w.2))
}
pub(in super::super) fn remaining(p: &Plan, target: bool) -> Result<u64> {
    let w = work(p)?;
    let (cap, n) = if citation::instruction_bridge(p) {
        if target {(300_000u64,w.4)}else{(4_000_000u64,w.3)}
    } else if citation::retained_qa(p) {
        if target {(2_400_000u64,w.4)} else {(18_000_000u64,w.3)}
    } else if citation::precision(p) {
        if target {(130_000u64,w.4)} else {(2_000_000u64,w.3)}
    } else if citation::is_mean(p) {
        if target {(260_000u64,w.4)} else {(4_000_000u64,w.3)}
    } else if citation::is(p) {
        if target {(800_000u64,w.4)} else {(6_300_000u64,w.3)}
    } else if is_consolidation(p) {
        if target {(30_000u64,w.4)} else {(2_100_000u64,w.3)}
    } else if is_expansion(p) {
        if target {(60_000u64,w.4)} else {(4_300_000u64,w.3)}
    } else if is_signal(p) {
        if target {(30_000u64,w.4)} else {(2_100_000u64,w.3)}
    } else if is_framing(p) {
        if target {
            (40_000u64, w.4)
        } else {
            (3_000_000u64, w.3)
        }
    } else if is_orbit(p) {
        if target {
            (20_000u64, w.4)
        } else {
            (2_000_000u64, w.3)
        }
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
        (if citation::instruction_bridge(p)||citation::retained_qa(p)||citation::fidelity(p)||citation::precision(p) {12}else{16}) * 1024 * 1024,
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

// Confirmation and explicitly admitted QA diagnostics permit time segments.
// Existing confirmation names and canonical bytes remain unchanged.
// Every command has a durable start
// and final, in addition to the existing per-call prepared/resolved protocol.
// A lost command final is UNKNOWN, even when the raw prefix looks complete.
fn confirmation_usage(root: &Path) -> Result<(f64, usize, usize)> {
    segmented_usage(root,"confirmation")
}
fn segmented_usage(root: &Path,name:&str) -> Result<(f64, usize, usize)> {
    let mut elapsed = 0.; let mut calls = 0; let mut next = 0;
    let mut recorded=BTreeMap::<String,String>::new();
    let binding: binary::Value = read_confirmed(&root.join(format!("{name}-started.r3b")))?;
    for i in 0..128 {
        let start = root.join(format!("{name}-segment-{i:03}-started.r3b"));
        if !start.exists() {
            for entry in std::fs::read_dir(root)? {
                let filename = entry?.file_name().to_string_lossy().into_owned();
                if let Some(index) = filename.strip_prefix(&format!("{name}-segment-")).and_then(|s|s.get(..3)).and_then(|s|s.parse::<usize>().ok()) {
                    if index >= i { return Err(bad("confirmation segment gap")); }
                }
            }
            if recorded!=segmented_call_files(root,name)? {return Err(bad("confirmation unaccounted call records; usage UNKNOWN"));}
            return Ok((elapsed, calls, next));
        }
        let s: binary::Value = read_confirmed(&start)?;
        let end: binary::Value = read_confirmed(&root.join(format!("{name}-segment-{i:03}-finished.r3b")))
            .map_err(|_|bad("confirmation command usage UNKNOWN"))?;
        if s["binding"] != digest(&binding)? || s["index"] != i || end["start"] != file_hash(&start)? {
            return Err(bad("confirmation segment identity"));
        }
        let c = &end["control"];
        let clean = c["terminal_reason"] == "COMPLETED" && c["observed_conditions"] == binary::record!([]) && end["error"].is_null();
        let timed = c["terminal_reason"] == "TIME_BUDGET" && c["observed_conditions"] == binary::record!(["TIME_BUDGET"]);
        if !clean && !timed { return Err(bad("confirmation non-time failure remains sticky")); }
        let seconds = c["elapsed_seconds"].as_f64().filter(|s|s.is_finite() && *s>=0.).ok_or_else(||bad("confirmation elapsed UNKNOWN"))?;
        let n = c["generation_calls"].as_u64().ok_or_else(||bad("confirmation calls UNKNOWN"))? as usize;
        if s["returned_before"]!=calls || end["returned_before"]!=s["returned_before"] || c["teacher_calls"] != 0 || end["returned_before"].as_u64().and_then(|b|b.checked_add(n as u64)) != end["returned_after"].as_u64() {
            return Err(bad("confirmation returned/call accounting"));
        }
        let raw=root.join(format!("{name}.r3rows"));
        let records=if raw.exists(){binary::read_value_records(&raw)?}else{vec![]};
        let after=calls+n;
        let prefix=if records.is_empty() && after==0 {&[][..]}else{records.get(..after+1).ok_or_else(||bad("confirmation segment prefix missing"))?};
        if end["prefix"]!=digest(&prefix)? {return Err(bad("confirmation segment/raw prefix changed"));}
        let current:BTreeMap<String,String>=binary::from_value(end["call_records"].clone())?;
        if recorded.iter().any(|(k,v)|current.get(k)!=Some(v)) {return Err(bad("confirmation call history changed"));}
        recorded=current;
        let tokens=prefix.iter().skip(calls+1).map(|r|r["raw_tokens"].as_array().map(Vec::len).ok_or_else(||bad("confirmation token usage UNKNOWN"))).collect::<Result<Vec<_>>>()?.iter().sum::<usize>();
        if end["generated_tokens"]!=tokens {return Err(bad("confirmation token usage mismatch"));}
        elapsed += seconds; calls += n; next = i+1;
    }
    Err(bad("confirmation segment limit"))
}
fn confirmation_call_files(root:&Path)->Result<BTreeMap<String,String>> {
    segmented_call_files(root,"confirmation")
}
fn segmented_call_files(root:&Path,name:&str)->Result<BTreeMap<String,String>> {
    let mut files=BTreeMap::new();
    for entry in std::fs::read_dir(root)? {
        let entry=entry?;let filename=entry.file_name().to_string_lossy().into_owned();
        if filename.starts_with(&format!("{name}-generation-")) {files.insert(filename,file_hash(&entry.path())?);}
    }
    Ok(files)
}
fn confirmation_prefix(root: &Path, binding: &binary::Value, es: &[Episode], tok: &ByteBpe) -> Result<Vec<binary::Value>> {
    segmented_prefix(root,"confirmation",binding,es,tok)
}
fn segmented_prefix(root: &Path,name:&str, binding: &binary::Value, es: &[Episode], tok: &ByteBpe) -> Result<Vec<binary::Value>> {
    let raw = root.join(format!("{name}.r3rows"));
    let rows = if raw.exists() {
        let mut rows = binary::read_value_records(&raw)?;
        if rows.first() != Some(binding) || rows.len()>es.len()+1 { return Err(bad("confirmation raw header/count")); }
        rows.remove(0); rows
    } else { vec![] };
    for (i,e) in es.iter().enumerate() {
        let row = rows.get(i);
        call_attempt(root,name,"generation",binding,e,i,row)?;
        if let Some(row)=row {
            if row["id"]!=e.id || row["question"]!=e.request.input || row["expected"]!=e.answer
                || row["generated_evidence"]!=binary::to_value(&e.request.evidence)? {
                return Err(bad("confirmation row/case content"));
            }
            verify_generated(row,tok)?;
        }
    }
    // Reject unknown/extra ordinals rather than ignoring files beyond the panel.
    for entry in std::fs::read_dir(root)? {
        let filename=entry?.file_name().to_string_lossy().into_owned();
        if let Some(s)=filename.strip_prefix(&format!("{name}-generation-")) {
            let parts=s.split('-').collect::<Vec<_>>();
            if parts.len()!=3 || !["prepared.r3b","resolved.r3b"].contains(&parts[2]) {return Err(bad("confirmation unexpected or pending call file"));}
            let i=parts.first().and_then(|n|n.parse::<usize>().ok()).ok_or_else(||bad("confirmation call filename"))?;
            let a=parts.get(1).and_then(|n|n.parse::<usize>().ok()).ok_or_else(||bad("confirmation attempt filename"))?;
            if i>=es.len() {return Err(bad("confirmation extra ordinal"));}
            let prepared=root.join(format!("{name}-generation-{i:04}-{a:03}-prepared.r3b"));
            if !prepared.exists() || (a>0 && !root.join(format!("{name}-generation-{i:04}-{:03}-prepared.r3b",a-1)).exists()) {
                return Err(bad("confirmation attempt gap"));
            }
            if let Some(row)=rows.get(i) {
                let last=row["attempt"].as_str().ok_or_else(||bad("confirmation returned attempt missing"))?;
                if prepared.file_name().unwrap().to_string_lossy().as_ref()>last {return Err(bad("confirmation attempt after RETURNED"));}
            }
            if parts.last()==Some(&"resolved.r3b") {
                let r:binary::Value=read_confirmed(&root.join(&filename))?;
                let conditions=&r["control"]["observed_conditions"];
                if *conditions!=binary::record!([]) && *conditions!=binary::record!(["TIME_BUDGET"]) {
                    return Err(bad("confirmation call has a sticky non-time failure"));
                }
                if r["state"]=="NOT_INVOKED" && r["control"]["observed_conditions"]!=binary::record!(["TIME_BUDGET"]) {
                    return Err(bad("confirmation no-call is not pure time"));
                }
            }
        }
    }
    Ok(rows)
}
fn confirmation_collect(root:&Path,path:&Path,es:&[Episode],tok:&ByteBpe,identity:&binary::Value,
    control:recovery::RunControl) -> Result<Vec<binary::Value>> {
    segmented_collect(root,"confirmation",path,es,tok,identity,control)
}
fn segmented_collect(root:&Path,name:&str,path:&Path,es:&[Episode],tok:&ByteBpe,identity:&binary::Value,
    mut control:recovery::RunControl) -> Result<Vec<binary::Value>> {
    if !["confirmation","bridge-qa-primary","bridge-qa-transfer"].contains(&name){return Err(bad("unregistered segmented observation"));}
    let binding=binary::record!({"segments":1,"identity":identity,"checkpoint":file_hash(path)?,"cases":digest(&es)?,"source":source_digest()?,
        "binary":file_hash(&std::env::current_exe()?)?,"decoding":"normal-greedy-strict-utf8-eos"});
    let started=root.join(format!("{name}-started.r3b"));
    if started.exists() { if read_confirmed::<binary::Value>(&started)?!=binding {return Err(bad("confirmation binding changed"));} }
    else {publish_confirmed(&started,&binding)?;}
    let (prior_elapsed,prior_calls,index)=segmented_usage(root,name)?;
    let mut rows=segmented_prefix(root,name,&binding,es,tok)?;
    if prior_calls!=rows.len() {return Err(bad("confirmation calls/raw disagreement"));}
    let final_path=root.join(format!("{name}-finished.r3b"));
    if final_path.exists() {
        let r:binary::Value=read_confirmed(&final_path)?;
        if rows.len()!=es.len() || r["binding"]!=binding || r["raw"]!=file_hash(&root.join(format!("{name}.r3rows")))?
            || r["control"]["generation_calls"]!=prior_calls || r["control"]["elapsed_seconds"]!=prior_elapsed {
            return Err(bad("confirmation final binding"));
        }
        return Ok(rows);
    }
    let before=rows.len();
    let start=root.join(format!("{name}-segment-{index:03}-started.r3b"));
    publish_confirmed(&start,&binary::record!({"binding":digest(&binding)?,"index":index,"returned_before":before,
        "reserved_generation_upper":es.len()-before,"unfinalized_usage":"UNKNOWN; automatic retry prohibited"}))?;
    let raw=root.join(format!("{name}.r3rows"));
    let result=(||->Result<()> {
        let exists=raw.exists();
        let mut f=std::fs::OpenOptions::new().write(true).append(exists).create_new(!exists).open(&raw)?;
        if !exists {append_row(&mut f,&binding)?;std::fs::File::open(root)?.sync_all()?;}
        if before<es.len() {
            #[cfg(feature="test-support")]
            if std::env::var_os("R3_FINALIZATION_NO_MODEL").is_some() {return Err(bad("FORBIDDEN_MODEL_ENTRY"));}
            let l=checkpoint::load(path,Device::Cpu,false)?;
            for (i,e) in es.iter().enumerate().skip(before) {
                control.check("confirmation_next_generation")?;
                let attempt=prepare_call(root,name,"generation",&binding,e,i)?;
                #[cfg(feature="test-support")]
                call_fixture(&l,&mut control,name,"generation",i);
                let mut row=match recovery::observe_generation(&l,e,&e.request,&mut control,false) {
                    recovery::ObservedCall::NotInvoked(_)=>{resolve_call(&attempt,None,&control)?;control.stop_result()?;return Err(bad("confirmation unexplained no-call"));},
                    recovery::ObservedCall::Returned(row)=>row,
                };
                row["attempt"]=binary::record!(attempt.file_name().unwrap().to_string_lossy());
                // A returned call consumes tokens even if its durable resolution
                // fails. Accounting does not authorize reuse of a pending call.
                rows.push(row);
                let row=rows.last().unwrap();
                append_row(&mut f,row)?;resolve_call(&attempt,Some(row),&control)?;
                verify_generated(row,&tok)?;
                control.check("confirmation_row_durable")?;
            }
        }
        control.check("confirmation_before_final")?;
        Ok(())
    })();
    if let Err(e)=&result {control.classify_error(e);}
    let result=result.and(control.seal_terminal());
    publish_confirmed(&root.join(format!("{name}-segment-{index:03}-finished.r3b")),&binary::record!({
        "start":file_hash(&start)?,"returned_before":before,"returned_after":rows.len(),"control":control.receipt(),
        "raw":if raw.exists(){Some(file_hash(&raw)?)}else{None},"prefix":digest(&if raw.exists(){binary::read_value_records(&raw)?}else{vec![]})?,
        "call_records":segmented_call_files(root,name)?,
        "generated_tokens":rows[before..].iter().map(|r|r["raw_tokens"].as_array().map(Vec::len).ok_or_else(||bad("confirmation raw tokens missing"))).collect::<Result<Vec<_>>>()?.iter().sum::<usize>(),
        "error":result.as_ref().err().map(ToString::to_string)}))?;
    result?;
    let (elapsed,calls,_)=segmented_usage(root,name)?;
    publish_confirmed(&final_path,&binary::record!({"policy":identity["policy"],"checkpoint":binding["checkpoint"],"binding":binding,
        "completed":rows.len(),"matched":0,"control":{"terminal_reason":"COMPLETED","observed_conditions":[],"generation_calls":calls,
        "teacher_calls":0,"elapsed_seconds":elapsed},"generated_tokens":rows.iter().map(|r|r["raw_tokens"].as_array().map(Vec::len).unwrap_or(0)).sum::<usize>(),
        "error":null,"raw":file_hash(&raw)?}))?;
    Ok(rows)
}
// Read-only admission for already RETURNED observations. This does not grant a
// new segment, call, candidate or training budget. The producer identity stays
// historical; a later finalizer is recorded separately by the caller.
fn segmented_returned(root:&Path,name:&str,binding:&binary::Value,es:&[Episode],tok:&ByteBpe)
    ->Result<(Vec<binary::Value>,f64)> {
    if read_confirmed::<binary::Value>(&root.join(format!("{name}-started.r3b")))?!=*binding {
        return Err(bad("finalization producer/input binding changed"));
    }
    let(elapsed,calls,_)=segmented_usage(root,name)?;
    let rows=segmented_prefix(root,name,binding,es,tok)?;
    if rows.len()!=es.len() || calls!=es.len() {return Err(bad("finalization has remaining calls"));}
    for row in &rows {
        let attempt=row["attempt"].as_str().ok_or_else(||bad("finalization returned attempt missing"))?;
        let r:binary::Value=read_confirmed(&root.join(attempt.replace("-prepared","-resolved")))?;
        if r["state"]!="RETURNED" || r["calls"]!=1 {return Err(bad("finalization requires accounted RETURNED calls"));}
    }
    Ok((rows,elapsed))
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
// A projection of present evidence, never a replacement for the immutable
// comparison that authorized opening the seal. This path performs no writes or
// native calls, including when a publication or an observation is incomplete.
fn confirmation_current(study: &Path, p: &Plan, end: &Segment, comparison: &binary::Value) -> binary::Value {
    match confirmation_current_checked(study, p, end, comparison) {
        Ok(value) => value,
        Err(e) => {
            let access = matches!(&e, replica_v3::Error::Io(io)
                if !matches!(io.kind(), std::io::ErrorKind::InvalidData | std::io::ErrorKind::NotFound));
            binary::record!({"status":if access {"NOT_VERIFIABLE"} else {"INTEGRITY_FAIL"},
                "error":e.to_string(),"minimal_binding_baseline_verified":false,"goal1_ready":false})
        }
    }
}
fn confirmation_current_checked(study: &Path, p: &Plan, end: &Segment, comparison: &binary::Value) -> Result<binary::Value> {
    // read_dir, rather than exists(), distinguishes inaccessible evidence from
    // an observed empty namespace. Call journals also count as execution traces.
    let mut names = BTreeSet::new();
    for entry in std::fs::read_dir(study)? {
        names.insert(entry?.file_name().to_string_lossy().into_owned());
    }
    let state = |status: &str| binary::record!({"status":status,"observed":!["NOT_OPENED","NOT_ELIGIBLE"].contains(&status),"checkpoint":end.checkpoint_hash,
        "minimal_binding_baseline_verified":false,"goal1_ready":false});
    let traces = names.iter().any(|n| n.starts_with("confirmation-") || n == "confirmation.r3rows");
    if !traces {
        return Ok(state(if comparison["selected"].is_null() {"NOT_ELIGIBLE"} else {"NOT_OPENED"}));
    }
    if !names.contains("confirmation-candidate.r3b") {
        return Err(bad("confirmation execution without fixed candidate"));
    }
    let root = study.join(&own(p).arm);
    let candidate: binary::Value = read(&study.join("confirmation-candidate.r3b"))?;
    let selection: binary::Value = read(&study.join("selection.r3b"))?;
    let seal_root = Path::new(selection["seal_root"].as_str().ok_or_else(||bad("confirmation seal root"))?);
    let seal = verify_seal(seal_root)?;
    let review: binary::Value = read_confirmed(&study.join("review-b.r3b"))?;
    if comparison["selected"] != own(p).arm || candidate != binary::record!({"arm":own(p).arm,
        "checkpoint":end.checkpoint_hash,"framing":p.framing(),"comparison":comparison,
        "seal":file_hash(&seal_root.join("confirmation-seal.r3b"))?,"review":file_hash(&study.join("review-b.r3b"))?})
        || review["verdict"] != "PASS" || review["endpoints"] != comparison["endpoints"]
        || review["preparation"] != file_hash(&study.join("preparation.r3b"))?
        || review["report_hash"] != file_hash(Path::new(review["report_path"].as_str().ok_or_else(||bad("review report path"))?))?
        || file_hash(&root.join(&end.checkpoint))? != end.checkpoint_hash {
        return Err(bad("confirmation candidate/model/review binding"));
    }
    let corpus = verified_corpus(&seal_root.join("sealed/confirmation.r3cor"), seal["corpus"].as_str().ok_or_else(||bad("seal corpus"))?)?;
    let metadata: Vec<Meta> = read(&seal_root.join("sealed/metadata.r3b"))?;
    let tok = ByteBpe::load(&root.join("tokenizer.r3b"))?;
    if tok.id() != p.tokenizer {return Err(bad("confirmation tokenizer binding"));}
    orbit_validate(&corpus.validation, &metadata, &tok, 256)?;
    let identity = binary::record!({"policy":digest(p)?,"checkpoint":end.checkpoint_hash,
        "candidate":file_hash(&study.join("confirmation-candidate.r3b"))?});
    let binding = binary::record!({"identity":identity,"checkpoint":end.checkpoint_hash,
        "cases":digest(&corpus.validation)?,"source":p.source,"binary":p.binary,"decoding":"normal-greedy-strict-utf8-eos"});
    let start = names.contains("confirmation-started.r3b");
    let raw = names.contains("confirmation.r3rows");
    let finish = names.contains("confirmation-finished.r3b");
    let result = names.contains("confirmation-result.r3b");
    if !start {
        if raw || finish || result || names.iter().any(|n|n.starts_with("confirmation-generation-")) {
            return Err(bad("confirmation output without start"));
        }
        return Ok(state("PENDING/INCOMPLETE"));
    }
    if read::<binary::Value>(&study.join("confirmation-started.r3b"))? != binding {
        return Err(bad("confirmation start identity"));
    }
    let rows = if raw {binary::read_value_records(&study.join("confirmation.r3rows"))?} else {vec![]};
    if (!rows.is_empty() && rows.first() != Some(&binding)) || rows.len() > 257 {
        return Err(bad("confirmation raw header/count"));
    }
    let actual = rows.get(1..).unwrap_or(&[]);
    // Validate even a partial prefix against sealed gold, not self-reported
    // expected answers. No missing row or call resolution is manufactured.
    score(actual, &corpus.validation[..actual.len()], &metadata[..actual.len()])?;
    for (i,(row,e)) in actual.iter().zip(&corpus.validation).enumerate() {
        verify_generated(row,&tok)?;
        let prompt=tok.prepare_with_framing(&e.request,p.framing(),p.architecture.context as u32,&p.architecture.id()?)?;
        if row["native_prompt_digest"]!=prompt.token_digest || row["framing"]!=p.framing().id() {
            return Err(bad("confirmation raw tokenizer/framing mismatch"));
        }
        if let Some(attempt) = row["attempt"].as_str() {
            let resolved = attempt.replace("-prepared", "-resolved");
            if names.contains(attempt) && (!names.contains(&resolved)
                || names.contains(&resolved.replace(".r3b", ".pending.r3b"))) {
                return Ok(state("UNKNOWN/FAILED_EXECUTION"));
            }
        }
        call_attempt(study,"confirmation","generation",&binding,e,i,Some(row))?;
    }
    let uncertain = names.iter().any(|n|n.starts_with("confirmation-") && n.ends_with("pending.r3b"));
    if uncertain {return Ok(state("UNKNOWN/FAILED_EXECUTION"));}
    if !finish {
        if result {return Err(bad("confirmation result without terminal"));}
        return Ok(state(if names.iter().any(|n|n.starts_with("confirmation-generation-") && n.ends_with("prepared.r3b")
            && !names.contains(&n.replace("-prepared", "-resolved"))) {"UNKNOWN/FAILED_EXECUTION"} else {"PENDING/INCOMPLETE"}));
    }
    let finished: binary::Value = read_confirmed(&study.join("confirmation-finished.r3b"))?;
    if finished["binding"] != binding || finished["policy"] != digest(p)? || finished["checkpoint"] != end.checkpoint_hash
        || finished["completed"] != actual.len() || finished["raw"] != if raw {binary::record!(file_hash(&study.join("confirmation.r3rows"))?)} else {binary::Value::Null}
        || finished["matched"] != 0 || finished["control"]["generation_calls"] != actual.len()
        || finished["control"]["teacher_calls"] != 0
        || finished["control"]["elapsed_seconds"].as_f64().is_none_or(|v|!v.is_finite() || v<0.) {
        return Err(bad("confirmation terminal/raw usage binding"));
    }
    if !finished["error"].is_null() || finished["control"]["terminal_reason"] != "COMPLETED"
        || finished["control"]["observed_conditions"] != binary::record!([]) {
        if result {return Err(bad("confirmation result after failed execution"));}
        return Ok(state("UNKNOWN/FAILED_EXECUTION"));
    }
    if actual.len() != 256 {return Err(bad("completed confirmation has missing rows"));}
    if !result {return Ok(state("PENDING/INCOMPLETE"));}
    let score = orbit_score(&corpus.validation,&metadata,actual,&tok)?;
    let pass = score.full>=244 && score.query_both>=116 && score.all4>=58 && score.errors==0 && score.eos==256;
    let saved: binary::Value = read_confirmed(&study.join("confirmation-result.r3b"))?;
    let scope = "K1-V finite digits; two records; new key/value sets";
    if saved != binary::record!({"arm":own(p).arm,"checkpoint":end.checkpoint_hash,"score":score,
        "minimal_binding_baseline_verified":pass,"scope":scope,"goal1_ready":false,"s4":false,"s5":false,"s6":false}) {
        return Err(bad("confirmation stored result differs from strict sealed raw recount"));
    }
    Ok(binary::record!({"status":if pass {"COMPLETED_PASS"} else {"COMPLETED_QUALITY_FAIL"},
        "observed":true,"checkpoint":end.checkpoint_hash,"score":score,"scope":scope,"minimal_binding_baseline_verified":pass,
        "historical_comparison":digest(comparison)?,"confirmation_result":file_hash(&study.join("confirmation-result.r3b"))?,
        "goal1_ready":false,"s4":false,"s5":false,"s6":false}))
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
fn orbit_peer_can_proceed(s: &Segment) -> bool {
    ["TRAINING", "TIME_BUDGET", "BUDGET_REACHED"].contains(&s.stop.as_str())
        && (s.phase.as_deref() != Some("Finished") || s.stop == "BUDGET_REACHED")
}
pub(in super::super) fn orbit_parity(root: &Path, p: &Plan, reviewer: bool) -> Result<()> {
    if !is_orbit(p) || (p.tiny && !(is_consolidation(p) && cfg!(feature="test-support"))) {
        return Err(bad("SMALL orbit parity scope"));
    }
    authorize(root, p)?;
    let h = history(root, p)?;
    let end = h.last().ok_or_else(|| bad("orbit endpoint missing"))?;
    if is_consolidation(p) {
        let (_, _, decision) = consolidation_close_bound(root, p.clone())?;
        consolidation_reproduction_endpoint(end, &decision, reviewer, p.tiny)?;
    } else if is_expansion(p) {
        if end.step!=3584 || end.resume || end.phase.as_deref()!=Some("Finished")
            || !["CANDIDATE_FIXED","FINAL_QUALITY_FAIL"].contains(&end.stop.as_str()) {
            return Err(bad("expansion final endpoint required"));
        }
    } else if is_signal(p) {
        if ![1024,2048].contains(&end.step) || end.resume || end.phase.as_deref()!=Some("Finished")
            || !["CANDIDATE_FIXED","FINAL_QUALITY_FAIL"].contains(&end.stop.as_str()) {
            return Err(bad("signal final endpoint required"));
        }
    } else if is_framing(p) {
        framing_final_step(&own(p).study)?;
    } else if end.step != 512 || end.resume || end.phase.as_deref() != Some("Finished") {
        return Err(bad("orbit final endpoint"));
    }
    let c = verified_corpus(&root.join("corpus.r3cor"), &p.corpus)?;
    let (_, dm, _) = verified_metadata(root, p)?;
    let tok = ByteBpe::load(&root.join("tokenizer.r3b"))?;
    let (panel, cases, metadata) = if p.tiny {
        panel_cases(root,p,end.step)?.into_iter().find(|(n,_,_)|n.starts_with("dev"))
            .ok_or_else(||bad("explicit TINY development panel absent"))?
    } else {("dev512".into(),c.validation,dm)};
    audit_panel(root, p, end.step, &panel, &cases, &metadata, &tok)?;
    let raw =
        binary::read_value_records(&root.join(format!("eval-{:04}-{panel}.r3rows", end.step)))?;
    let count=cases.len().min(16);
    let destination = if reviewer {
        own(p).study.clone()
    } else {
        root.to_owned()
    };
    let name = if reviewer {
        format!("review-{}", own(p).arm)
    } else {
        "parity".into()
    };
    let identity = binary::record!({"policy":digest(p)?,"checkpoint":end.checkpoint_hash,"selection":"first16 frozen dev rows",
        "reviewer":reviewer,"cases":digest(&&cases[..count])?});
    orbit_observe(&destination,&name,&root.join(&end.checkpoint),&cases[..count],&identity,Some(&raw[1..count+1]),observation_control(p,count,0)?)?;
    println!("ORBIT_PARITY arm={} reviewer={reviewer} matched{count}/{count} optimizer0 teacher0",own(p).arm);Ok(())
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
pub(in super::super) fn orbit_confirm(study: &Path) -> Result<()> {
    let consolidation = study.join(CONTINUE_ARM).join("plan.r3b").exists();
    let expansion = study.join("REPEAT/plan.r3b").exists();
    let framing = study.join("QE/plan.r3b").exists();
    let signal = framing && is_signal(&plan_read(&study.join("QE"))?);
    let comparison = if consolidation {
        consolidation_comparison(study)?
    } else if expansion {
        expansion_comparison(study)?
    } else if signal {
        signal_comparison(study)?
    } else if framing {
        framing_comparison(study)?
    } else {
        orbit_comparison(study)?
    };
    let arm = comparison["selected"]
        .as_str()
        .ok_or_else(|| bad("NOT_RUN_PREREQUISITE: no baseline candidate"))?;
    let review: binary::Value = read_confirmed(&study.join("review-b.r3b"))?;
    if review["verdict"] != "PASS"
        || review["preparation"] != file_hash(&study.join("preparation.r3b"))?
        || review["endpoints"] != comparison["endpoints"]
        || review["report_hash"]
            != file_hash(Path::new(
                review["report_path"]
                    .as_str()
                    .ok_or_else(|| bad("result review path"))?,
            ))?
    {
        return Err(bad("independent endpoint recount required"));
    }
    let root = study.join(arm);
    let p = plan_read(&root)?;
    let h = history(&root, &p)?;
    let end = h.last().unwrap();
    let selection: binary::Value = if framing || expansion || consolidation {
        read(&study.join("selection.r3b"))?
    } else {
        binary::Value::Null
    };
    let seal_root = if framing || expansion || consolidation {
        Path::new(
            selection["seal_root"]
                .as_str()
                .ok_or_else(|| bad("seal root"))?,
        )
    } else {
        study
    };
    let seal = verify_seal(seal_root)?;
    // Candidate is fixed durably before reading the sealed examples or answers.
    write(
        &study.join("confirmation-candidate.r3b"),
        &binary::record!({"arm":arm,"checkpoint":end.checkpoint_hash,
        "framing":p.framing(),"comparison":comparison,"seal":file_hash(&seal_root.join("confirmation-seal.r3b"))?,"review":file_hash(&study.join("review-b.r3b"))?}),
    )?;
    let c = verified_corpus(
        &seal_root.join("sealed/confirmation.r3cor"),
        seal["corpus"].as_str().ok_or_else(|| bad("seal corpus"))?,
    )?;
    let ms: Vec<Meta> = read(&seal_root.join("sealed/metadata.r3b"))?;
    let tok = ByteBpe::load(&root.join("tokenizer.r3b"))?;
    orbit_validate(&c.validation, &ms, &tok, 256)?;
    let identity = binary::record!({"policy":digest(&p)?,"checkpoint":end.checkpoint_hash,"candidate":file_hash(&study.join("confirmation-candidate.r3b"))?});
    let rows = orbit_observe(
        study,
        "confirmation",
        &root.join(&end.checkpoint),
        &c.validation,
        &identity,
        None,
        observation_control(&p, 256, 0)?,
    )?;
    let s = orbit_score(&c.validation, &ms, &rows, &tok)?;
    let pass =
        s.full >= 244 && s.query_both >= 116 && s.all4 >= 58 && s.errors == 0 && s.eos == 256;
    publish_confirmed(
        &study.join("confirmation-result.r3b"),
        &binary::record!({"arm":arm,"checkpoint":end.checkpoint_hash,
        "score":s,"minimal_binding_baseline_verified":pass,"scope":"K1-V finite digits; two records; new key/value sets",
        "goal1_ready":false,"s4":false,"s5":false,"s6":false}))?;
    println!("ORBIT_CONFIRMATION arm={arm} full={} query_both={} all4={} errors={} minimal_baseline={pass} GOAL1=false",s.full,s.query_both,s.all4,s.errors);Ok(())
}

fn framing_config(tiny: bool) -> TrainConfig {
    let mut c = orbit_config(tiny);
    c.max_steps = if tiny { 4 } else { 1024 };
    c.max_tokens = 3_000_000;
    c
}
fn framing_rows(tiny: bool) -> Vec<[usize; 8]> {
    let rows = orbit_rows("BOTH", if tiny { 2 } else { 512 });
    [rows.clone(), rows].concat()
}
fn framing_parent(parent: &Path) -> Result<Plan> {
    let raw: Plan = read(&parent.join("plan.r3b"))?;
    let p = plan_read_bound(parent, &raw.source, &raw.binary)?;
    if own(&p).dataset != ORBIT_DATA || own(&p).arm != "BOTH" {
        return Err(bad("preserved BOTH reference required"));
    }
    Ok(p)
}
fn framing_equivalence(es: &[Episode], tok: &ByteBpe, seq: usize) -> Result<()> {
    let qe = samples_with_framing(es, tok, seq, neural::Framing::QuestionEvidence)?;
    let eq = samples_with_framing(es, tok, seq, neural::Framing::EvidenceQuestion)?;
    for ((e, a), b) in es.iter().zip(&qe).zip(&eq) {
        let mut starts = a.tokens[..a.response_start]
            .iter()
            .enumerate()
            .filter_map(|(i, &v)| {
                [
                    neural::SYSTEM_ROLE,
                    neural::USER_ROLE,
                    neural::EVIDENCE_ROLE,
                    neural::ASSISTANT_ROLE,
                ]
                .contains(&v)
                .then_some((i, v))
            })
            .collect::<Vec<_>>();
        if starts.iter().map(|x| x.1).collect::<Vec<_>>()
            != [
                neural::SYSTEM_ROLE,
                neural::USER_ROLE,
                neural::EVIDENCE_ROLE,
                neural::EVIDENCE_ROLE,
                neural::ASSISTANT_ROLE,
            ]
            || a.tokens.len() != 146
            || a.tokens.len() != b.tokens.len()
            || a.response_start != b.response_start
        {
            return Err(bad("frame block/target boundary"));
        }
        starts.push((a.response_start, EOS));
        let expected = [
            &a.tokens[..starts[1].0],
            &a.tokens[starts[2].0..starts[4].0],
            &a.tokens[starts[1].0..starts[2].0],
            &a.tokens[starts[4].0..],
        ]
        .concat();
        if expected != b.tokens {
            return Err(bad("EQ must be exact block permutation including target"));
        }
        for (framing, s) in [
            (neural::Framing::QuestionEvidence, a),
            (neural::Framing::EvidenceQuestion, b),
        ] {
            let p = tok.prepare_with_framing(&e.request, framing, 2048, "frame-check")?;
            if p.provided.len() != 2
                || !p.excluded.is_empty()
                || p.token_ids != s.tokens[..s.response_start]
                || s.tokens[s.response_start..] != [tok.encode(e.answer.as_bytes())?[0], EOS]
            {
                return Err(bad("frame train/generation/target parity"));
            }
        }
    }
    Ok(())
}
pub(in super::super) fn framing_prepare(parent: &Path, output: &Path, tiny: bool) -> Result<()> {
    if cfg!(feature = "test-support") && !tiny {
        return Err(bad("production framing binary required"));
    }
    let old = framing_parent(parent)?;
    if old.tiny != tiny {
        return Err(bad("framing initial profile"));
    }
    let c = verified_corpus(&parent.join("corpus.r3cor"), &old.corpus)?;
    let tok = ByteBpe::load(&parent.join("tokenizer.r3b"))?;
    let initial = checkpoint::load(&parent.join("initial.r3m"), Device::Cpu, false)?;
    if initial.manifest.training.is_some()
        || initial.model.weight_hash()? != old.initial_weights
        || initial.model.config != old.architecture
        || initial.tokenizer.id() != old.tokenizer
    {
        return Err(bad("untrained common A tensor required"));
    }
    framing_equivalence(
        &[c.train.clone(), c.validation.clone()].concat(),
        &tok,
        old.config.seq_len,
    )?;
    let adam = Adam::new(&initial.model.vars)?;
    if adam.moments.values().any(|t| {
        t.flatten_all()
            .and_then(|x| x.to_vec1::<f32>())
            .map_or(true, |v| v.iter().any(|x| x.to_bits() != 0))
    }) {
        return Err(bad("Adam must start at zero"));
    }
    let seal = verify_seal(&own(&old).study)?; // hashes only, no sealed examples read
    std::fs::create_dir(output)?;
    write(
        &output.join("selection.r3b"),
        &binary::record!({"contract":FRAME_CONTRACT,"parent":parent,"parent_policy":file_hash(&parent.join("plan.r3b"))?,
        "corpus":old.corpus,"metadata":old.metadata,"initial":old.initial,"tokenizer":old.tokenizer,
        "seal_root":own(&old).study,"seal":file_hash(&own(&old).study.join("confirmation-seal.r3b"))?,"sealed_corpus":seal["corpus"],
        "extension_rule":"512: any full gate stops; else train QB>=64 AND ALL4>=16 OR dev QB>=32 AND ALL4>=8 permits both additional512",
        "source":source_digest()?}),
    )?;
    let mut plans = BTreeMap::new();
    for (arm, frame) in [
        ("QE", neural::Framing::QuestionEvidence),
        ("EQ", neural::Framing::EvidenceQuestion),
    ] {
        let root = output.join(arm);
        std::fs::create_dir(&root)?;
        for name in [
            "corpus.r3cor",
            "transfer.r3cor",
            "metadata.r3b",
            "tokenizer.r3b",
            "initial.r3m",
        ] {
            copy_native(&parent.join(name), &root.join(name))?;
        }
        let rows = framing_rows(tiny);
        let mut p = old.clone();
        p.framing = Some(frame);
        p.config = framing_config(tiny);
        p.source = source_digest()?;
        p.binary = file_hash(&std::env::current_exe()?)?;
        p.identifiable = Some(Policy {
            study: output.to_owned(),
            arm: arm.into(),
            dataset: FRAME_DATA.into(),
            rows: rows.clone(),
        });
        p.train_order = digest(&rows)?;
        p.evaluation = evaluation_for(&p);
        write(&root.join("plan.r3b"), &p)?;
        framing_verify_plan(&root, &p)?;
        let f = samples_with_framing(&c.train, &tok, p.config.seq_len, frame)?;
        let n = if tiny { 2 } else { 512 };
        plans.insert(arm,binary::record!({"policy":file_hash(&root.join("plan.r3b"))?,"framing":frame.id(),"corpus":p.corpus,"metadata":p.metadata,
            "initial":p.initial,"initial_content":initial.model.weights_content_id()?,"input_first_stage":rows[..n].iter().flatten().map(|&i|f[i].tokens.len()-1).sum::<usize>(),
            "target_first_stage":n*16,"tape":p.train_order,"exposure_first_stage":orbit_exposure(&rows[..n])?}));
    }
    publish_confirmed(
        &output.join("preparation.r3b"),
        &binary::record!({"contract":FRAME_CONTRACT,"source":source_digest()?,"binary":file_hash(&std::env::current_exe()?)?,
        "arms":plans,"selection":file_hash(&output.join("selection.r3b"))?,"parameters":initial.model.config.parameters(),"tokenizer":tok.id(),
        "initial_content":initial.model.weights_content_id()?,"adam_zero":optimizer_hash(&adam.moments)?,"optimizer":0,"generation":0,"teacher":0,
        "confirmation":"PRIOR_SEAL_UNOPENED","max_updates":if tiny {8}else{2048},"stage_updates":if tiny {2}else{512}}),
    )?;
    println!(
        "FRAMING_PREPARED same corpus/tensor/tape QE/EQ only block order; optimizer0 generation0 teacher0 REVIEW_PENDING"
    );
    Ok(())
}
fn framing_verify_plan(root: &Path, p: &Plan) -> Result<()> {
    let o = own(p);
    let expected = match o.arm.as_str() {
        "QE" => neural::Framing::QuestionEvidence,
        "EQ" => neural::Framing::EvidenceQuestion,
        _ => return Err(bad("unknown framing arm")),
    };
    if root != o.study.join(&o.arm)
        || p.framing != Some(expected)
        || p.config != framing_config(p.tiny)
        || o.rows != framing_rows(p.tiny)
        || p.fork.is_some()
        || p.paired.is_some()
        || p.grounding.is_some()
        || p.training_values.is_some()
    {
        return Err(bad("framing policy/tape mismatch"));
    }
    let s: binary::Value = read(&o.study.join("selection.r3b"))?;
    let parent = Path::new(s["parent"].as_str().ok_or_else(|| bad("framing parent"))?);
    let old = framing_parent(parent)?;
    if s["parent_policy"] != file_hash(&parent.join("plan.r3b"))?
        || s["source"] != p.source
        || p.corpus != old.corpus
        || p.metadata != old.metadata
        || p.initial != old.initial
        || p.initial_weights != old.initial_weights
        || p.tokenizer != old.tokenizer
        || p.architecture != old.architecture
        || p.order != old.order
        || p.transfer != old.transfer
        || p.model_seed != old.model_seed
        || p.data_seed != old.data_seed
        || p.split_policy != old.split_policy
    {
        return Err(bad("framing frozen parent mismatch"));
    }
    for (name, hash) in [
        ("corpus.r3cor", &old.corpus),
        ("transfer.r3cor", &old.transfer),
        ("metadata.r3b", &old.metadata),
        ("initial.r3m", &old.initial),
    ] {
        if file_hash(&root.join(name))? != *hash {
            return Err(bad("framing copied bytes mismatch"));
        }
    }
    let tok = ByteBpe::load(&root.join("tokenizer.r3b"))?;
    if tok.id() != old.tokenizer {
        return Err(bad("framing tokenizer mapping mismatch"));
    }
    let c = verified_corpus(&root.join("corpus.r3cor"), &p.corpus)?;
    framing_equivalence(&[c.train, c.validation].concat(), &tok, p.config.seq_len)
}
fn framing_authorize(_root: &Path, p: &Plan) -> Result<()> {
    let study = &own(p).study;
    let s: binary::Value = read(&study.join("selection.r3b"))?;
    let seal_root = Path::new(
        s["seal_root"]
            .as_str()
            .ok_or_else(|| bad("framing seal root"))?,
    );
    if s["seal"] != file_hash(&seal_root.join("confirmation-seal.r3b"))?
        || verify_seal(seal_root)?["corpus"] != s["sealed_corpus"]
    {
        return Err(bad("framing seal changed"));
    }
    let prep: binary::Value = read_confirmed(&study.join("preparation.r3b"))?;
    if prep["selection"] != file_hash(&study.join("selection.r3b"))? {
        return Err(bad("framing reviewed selection changed"));
    }
    for arm in FRAME_ARMS {
        let r = study.join(arm);
        let peer = plan_read(&r)?;
        let h = history(&r, &peer)?;
        if h.last().is_some_and(|s| !orbit_peer_can_proceed(s)) {
            return Err(bad("framing failed peer"));
        }
    }
    if !p.tiny {
        let legacy: binary::Value = read_confirmed(&study.join("legacy-finished.r3b"))?;
        let parent = Path::new(s["parent"].as_str().ok_or_else(|| bad("legacy parent"))?);
        let old = framing_parent(parent)?;
        let h = history(parent, &old)?;
        let end = h.last().ok_or_else(|| bad("legacy final endpoint"))?;
        if legacy["matched"] != 16
            || legacy["completed"] != 16
            || legacy["binding"]["source"] != p.source
            || legacy["binding"]["binary"] != p.binary
            || legacy["binding"]["identity"]["parent"] != s["parent"]
            || legacy["binding"]["identity"]["framing"] != neural::PROMPT_FORMAT
            || legacy["checkpoint"] != end.checkpoint_hash
            || end.step != 512 || end.resume
            || !legacy["error"].is_null()
            || legacy["control"]["generation_calls"] != 16
            || legacy["control"]["teacher_calls"] != 0
            || legacy["control"]["terminal_reason"] != "COMPLETED"
            || legacy["raw"] != file_hash(&study.join("legacy.r3rows"))?
        {
            return Err(bad("QE legacy parity required"));
        }
    }
    Ok(())
}
fn framing_action(step: usize, full: bool, signal: bool) -> Result<(&'static str, bool)> {
    match (step, full, signal) {
        (512, true, _) => Ok(("CANDIDATE_AT_512", false)),
        (512, false, true) => Ok(("EXTEND_BOTH_TO_1024", true)),
        (512, false, false) => Ok(("CLOSE_NO_JOINT_SIGNAL", false)),
        (1024, true, _) => Ok(("CANDIDATE_AT_1024", false)),
        (1024, false, _) => Ok(("FINAL_QUALITY_FAIL_AT_1024", false)),
        _ => Err(bad("unsupported framing decision endpoint")),
    }
}
fn framing_endpoint_score(requested:usize,actual:usize,pending:bool,normal:bool,
    tr:&OrbitScore,dv:&OrbitScore)->Result<(bool,bool)> {
    if requested!=actual || pending || !normal || tr.total!=512 || dv.total!=512
        || tr.errors!=0 || dv.errors!=0 || tr.eos!=512 || dv.eos!=512 {
        return Err(bad("matched framing endpoint incomplete"));
    }
    Ok((orbit_candidate_scores(tr,dv),(tr.query_both>=64&&tr.all4>=16)||(dv.query_both>=32&&dv.all4>=8)))
}

// A new, explicitly parent-bound study. Historical plans are read with their
// retained identities only here; they never authorize execution by this binary.
fn signal_parent(root: &Path) -> Result<(Plan, Segment)> {
    let raw: Plan = read(&root.join("plan.r3b"))?;
    let p = plan_read_bound(root, &raw.source, &raw.binary)?;
    if !is_framing(&p) || own(&p).arm != "QE" || p.framing() != neural::Framing::QuestionEvidence {
        return Err(bad("preserved QE parent required"));
    }
    let h = history(root,&p)?;
    let end = h.last().ok_or_else(||bad("QE parent endpoint missing"))?.clone();
    if end.step != if p.tiny {2} else {512} || end.phase.as_deref()!=Some("TrainingPending")
        || end.stop!="TRAINING" || !end.resume {
        return Err(bad("QE staged parent identity"));
    }
    if !p.tiny {
        let d: binary::Value = read_confirmed(&own(&p).study.join("framing-decision.r3b"))?;
        if d["decision"]!="CLOSE_NO_JOINT_SIGNAL" || d["extend"]!=false || d["source"]!=p.source
            || d["endpoints"]["QE"]["checkpoint"]!=end.checkpoint_hash
            || d["endpoints"]["QE"]["step"]!=512 {
            return Err(bad("closed historical decision binding"));
        }
    }
    Ok((p,end))
}
// This fork only broadens key-pair coverage. Historical terminal receipts are
// read under their retained source identities, never reopened for execution.
fn expansion_parent(root: &Path) -> Result<(Plan, Segment)> {
    let raw: Plan = read(&root.join("plan.r3b"))?;
    let p = plan_read_bound(root, &raw.source, &raw.binary)?;
    let end = history(root, &p)?
        .last()
        .cloned()
        .ok_or_else(|| bad("expansion parent missing"))?;
    if !is_signal(&p)
        || p.tiny
        || own(&p).arm != "QE"
        || end.step != 2048
        || end.resume
        || end.phase.as_deref() != Some("Finished")
        || end.stop != "FINAL_QUALITY_FAIL"
        || p.framing() != neural::Framing::QuestionEvidence
    {
        return Err(bad("closed QE2048 weights/Adam required"));
    }
    let d: binary::Value = read_confirmed(&root.join("signal-decision-2048.r3b"))?;
    if d != signal_evaluation_result(root, &p, 2048)? {
        return Err(bad("preserved parent decision/raw mismatch"));
    }
    Ok((p, end))
}
fn rekey(e: &Episode, s: Skeleton) -> Result<Episode> {
    let old = semantic_skeleton(e)?;
    if old[2..] != s[2..] || canonical(s) != s || s.iter().collect::<BTreeSet<_>>().len() != 4 {
        return Err(bad("rekey must preserve values"));
    }
    let mut out = e.clone();
    let map = |text: &str| -> Result<String> {
        let offset = "장치".len();
        let digit = text.as_bytes().get(offset).ok_or_else(|| bad("key span"))?;
        if !text.starts_with("장치") {
            return Err(bad("key prefix"));
        }
        let side = old[..2]
            .iter()
            .position(|v| v + b'0' == *digit)
            .ok_or_else(|| bad("key outside skeleton"))?;
        let mut v = text.to_owned();
        v.replace_range(offset..offset + 1, &s[side].to_string());
        Ok(v)
    };
    out.request.input = map(&e.request.input)?;
    for r in &mut out.request.evidence.items {
        r.original_excerpt = map(&r.original_excerpt)?;
    }
    if resolve_request(&out.request)? != e.answer || semantic_skeleton(&out)? != s {
        return Err(bad("rekey resolver/target"));
    }
    Ok(out)
}
fn verify_rekey(a: &Episode, b: &Episode, tok: &ByteBpe) -> Result<()> {
    let expected = rekey(a, semantic_skeleton(b)?)?;
    if digest(&expected.request)? != digest(&b.request)? || a.answer != b.answer {
        return Err(bad("rekey changed metadata/value/target"));
    }
    let sa = samples(std::slice::from_ref(a), tok, 256)?.remove(0);
    let sb = samples(std::slice::from_ref(b), tok, 256)?.remove(0);
    let p = tok.prepare(&b.request, 2048, "rekey")?;
    if sa.tokens.len() != 146
        || sb.tokens.len() != 146
        || sa.response_start != 144
        || sb.response_start != 144
        || sa.tokens[144..] != sb.tokens[144..]
        || sa.tokens[145] != EOS
        || p.provided.len() != 2
        || !p.excluded.is_empty()
    {
        return Err(bad("rekey actual token shape/target"));
    }
    let mut allowed = BTreeSet::new();
    for (i, &role) in sa.tokens[..144].iter().enumerate() {
        if ![neural::USER_ROLE, neural::EVIDENCE_ROLE].contains(&role) {
            continue;
        }
        let start = i + 1;
        let end = start
            + sa.tokens[start..144]
                .iter()
                .position(|&v| v == neural::END_ROLE)
                .ok_or_else(|| bad("key role end"))?;
        let bytes = tok.decode_bytes(&sa.tokens[start..end])?;
        let needle = "장치".as_bytes();
        let spans = bytes
            .windows(needle.len())
            .enumerate()
            .filter_map(|(n, w)| (w == needle).then_some(n + needle.len()))
            .collect::<Vec<_>>();
        if spans.len() != 1 {
            return Err(bad("unique key span per role"));
        }
        let mut offset = 0;
        for j in start..end {
            let piece = tok.decode_bytes(&[sa.tokens[j]])?;
            if offset == spans[0] && piece.len() == 1 && piece[0].is_ascii_digit() {
                allowed.insert(j);
            }
            offset += piece.len();
        }
    }
    let changed = sa
        .tokens
        .iter()
        .zip(&sb.tokens)
        .enumerate()
        .filter_map(|(i, (x, y))| (x != y).then_some(i))
        .collect::<BTreeSet<_>>();
    if allowed.len() != 3 || changed.is_empty() || !changed.is_subset(&allowed) {
        return Err(bad("non-key token intervention"));
    }
    Ok(())
}
fn expansion_pool(parent: &Path, old: &Plan) -> Result<(Vec<Episode>, Vec<Meta>, binary::Value)> {
    let c = verified_corpus(&parent.join("corpus.r3cor"), &old.corpus)?;
    let (tm, dm, _) = verified_metadata(parent, old)?;
    if c.train.len() != 512 || c.validation.len() != 512 {
        return Err(bad("expansion parent pool size"));
    }
    let tok = ByteBpe::load(&parent.join("tokenizer.r3b"))?;
    let previous: binary::Value = read(&own(old).study.join("selection.r3b"))?;
    let seal_root = Path::new(
        previous["seal_root"]
            .as_str()
            .ok_or_else(|| bad("reserved seal path"))?,
    );
    expansion_pool_from(&c, tm, &dm, &tok, seal_root)
}
fn expansion_pool_from(
    c: &data::native::Corpus,
    tm: Vec<Meta>,
    dm: &[Meta],
    tok: &ByteBpe,
    seal_root: &Path,
) -> Result<(Vec<Episode>, Vec<Meta>, binary::Value)> {
    orbit_validate(&c.train, &tm, tok, 512)?;
    orbit_validate(&c.validation, dm, tok, 512)?;
    let tr = c
        .train
        .as_chunks::<4>()
        .0
        .iter()
        .map(|q| semantic_skeleton(&q[0]))
        .collect::<Result<Vec<_>>>()?;
    let dev = c
        .validation
        .as_chunks::<4>()
        .0
        .iter()
        .map(|q| semantic_skeleton(&q[0]))
        .collect::<Result<Vec<_>>>()?;
    // Public finite rank algorithm and the existing seal digest only. No
    // confirmation episode, answer or metadata is opened for preparation.
    let seal: binary::Value = read_confirmed(&seal_root.join("confirmation-seal.r3b"))?;
    let reserved = heldout_skeletons(&[tr.clone(), dev.clone()].concat(), 64);
    if seal["skeleton_digest"] != digest(&reserved)? || seal["count"] != 256 {
        return Err(bad("reserved skeleton digest"));
    }
    let mut used = tr
        .iter()
        .chain(&dev)
        .chain(&reserved)
        .copied()
        .collect::<BTreeSet<_>>();
    let mut candidates = universe()
        .into_iter()
        .filter(|s| !used.contains(s))
        .collect::<Vec<_>>();
    candidates.sort_by_key(|&s| rank(s));
    let mut capacity = BTreeMap::new();
    for s in &tr {
        let key = format!("{},{}", s[2], s[3]);
        let needed = tr.iter().filter(|v| v[2..] == s[2..]).count() * 2;
        let available = candidates.iter().filter(|v| v[2..] == s[2..]).count();
        if available < needed {
            return Err(bad("BLOCKED_DATA_CAPACITY"));
        }
        capacity.insert(
            key,
            binary::record!({"available":available,"required":needed}),
        );
    }
    let mut es = c.train.clone();
    let mut ms = tm;
    let mut chosen = vec![];
    for group in 1..=2 {
        for (base, &s) in tr.iter().enumerate() {
            let next = *candidates
                .iter()
                .find(|v| v[2..] == s[2..] && !used.contains(*v))
                .ok_or_else(|| bad("BLOCKED_DATA_CAPACITY"))?;
            used.insert(next);
            chosen.push(next);
            let key = format!("{EXPANSION_DATA}/new{group}/{base}");
            for view in 0..4 {
                let old_e = &c.train[base * 4 + view];
                let mut e = rekey(old_e, next)?;
                e.id = format!("{key}/{view}");
                e.family = key.clone();
                e.sequence = key.clone();
                e.binding = digest(&e.request.evidence)?;
                let mut m = ms[base * 4 + view].clone();
                m.id = e.id.clone();
                m.base = key.clone();
                m.split = "train".into();
                m.entities = vec![format!("장치{}", next[0]), format!("장치{}", next[1])];
                m.source_id = Some(format!("{}/{group}", old_e.id));
                verify_rekey(old_e, &e, tok)?;
                es.push(e);
                ms.push(m);
            }
        }
    }
    let mut audits = vec![];
    for group in 0..3 {
        audits.push(orbit_validate(
            &es[group * 512..(group + 1) * 512],
            &ms[group * 512..(group + 1) * 512],
            tok,
            512,
        )?);
    }
    let mut prompts = BTreeMap::new();
    let mut edges = BTreeMap::new();
    for e in &es {
        let s = samples(std::slice::from_ref(e), tok, 256)?.remove(0);
        if prompts
            .insert(s.tokens[..s.response_start].to_vec(), e.answer.clone())
            .is_some_and(|v| v != e.answer)
        {
            return Err(bad("conflicting identical prompt"));
        }
        for r in &e.request.evidence.items {
            let (k, _, v) = parsed_record(r)?;
            bump(&mut edges, format!("{k}->{v}"));
        }
    }
    if es.iter().map(|e| &e.id).collect::<BTreeSet<_>>().len() != 1536
        || ms.iter().map(|m| &m.id).collect::<BTreeSet<_>>().len() != 1536
    {
        return Err(bad("expansion duplicate external ID"));
    }
    Ok((
        es,
        ms,
        binary::record!({"capacity":capacity,"old":tr,"new":chosen,"dev":dev,"reserved_digest":seal["skeleton_digest"],
        "seal_root":seal_root,"seal":file_hash(&seal_root.join("confirmation-seal.r3b"))?,"sealed_corpus":seal["corpus"],
        "groups":audits,"edge_frequency_pool":edges,"input_length":145,"target_length":2,"rows":1536,"changed":"query/record key digits only"}),
    ))
}
fn expansion_rows(old: &Plan, arm: &str) -> Result<Vec<[usize; 8]>> {
    if !EXPANSION_ARMS.contains(&arm) || own(old).rows.len() != 2048 {
        return Err(bad("expansion tape parent"));
    }
    let base = &own(old).rows[..512];
    let mut visits = [0usize; 128];
    let mut out = own(old).rows.clone();
    for local in 0..1536 {
        let mut row = base[local % 512];
        for pair in row.as_chunks_mut::<2>().0 {
            let b = pair[0] / 4;
            if b >= 128 || pair[0] % 2 != 0 || pair[1] != pair[0] + 1 {
                return Err(bad("parent paired slot"));
            }
            let group = if arm == "REBIND" { visits[b] % 3 } else { 0 };
            visits[b] += 1;
            pair[0] += group * 512;
            pair[1] += group * 512;
        }
        out.push(row);
    }
    if visits.iter().any(|&n| n != 48) {
        return Err(bad("expansion slot visits"));
    }
    Ok(out)
}
fn expansion_exposure(
    rows: &[[usize; 8]],
    arm: &str,
    es: &[Episode],
    tok: &ByteBpe,
) -> Result<binary::Value> {
    if rows.len() != 1536 || es.len() != 1536 {
        return Err(bad("expansion tape/pool counts"));
    }
    let ss = samples(es, tok, 256)?;
    let mut counts = vec![0usize; 1536];
    let mut assignments = vec![[0usize; 2]; 384];
    let mut input = 0;
    let mut target = 0;
    let mut edges = BTreeMap::new();
    let mut keys = BTreeMap::new();
    for row in rows {
        let mut logical = BTreeSet::new();
        for pair in row.as_chunks::<2>().0 {
            if pair[0] >= 1536
                || pair[0] % 2 != 0
                || pair[1] != pair[0] + 1
                || !logical.insert(pair[0] % 512 / 4)
            {
                return Err(bad("expansion paired batch"));
            }
            assignments[pair[0] / 4][pair[0] % 4 / 2] += 1;
            for &i in pair {
                if ss[i].tokens.len() != 146 || ss[i].tokens[144..] != ss[i % 512].tokens[144..] {
                    return Err(bad("paired target/shape mismatch"));
                }
                counts[i] += 1;
                input += 145;
                target += 2;
                for r in &es[i].request.evidence.items {
                    let (k, _, v) = parsed_record(r)?;
                    bump(&mut keys, k.to_owned());
                    bump(&mut edges, format!("{k}->{v}"));
                }
            }
        }
    }
    for (i, &n) in counts.iter().enumerate() {
        if n != if arm == "REPEAT" {
            if i < 512 { 24 } else { 0 }
        } else {
            8
        } {
            return Err(bad("expansion exact per-row exposure"));
        }
    }
    if assignments.iter().enumerate().any(|(b, &n)| {
        n != if arm == "REPEAT" {
            if b < 128 { [24, 24] } else { [0, 0] }
        } else {
            [8, 8]
        }
    }) || input != 1_781_760
        || target != 24_576
    {
        return Err(bad("expansion target/input/assignment budget"));
    }
    Ok(
        binary::record!({"updates":1536,"rows":12288,"unique":counts.iter().filter(|&&n|n>0).count(),"counts":counts,
        "per_assignment":assignments,"input":input,"target":target,"key_frequency":keys,"edge_frequency":edges}),
    )
}
fn expansion_plan(
    old: &Plan,
    state: &TrainingState,
    study: &Path,
    arm: &str,
    s: &binary::Value,
) -> Result<Plan> {
    let mut p = old.clone();
    p.source = s["source"]
        .as_str()
        .ok_or_else(|| bad("expansion source"))?
        .into();
    p.binary = s["binary"]
        .as_str()
        .ok_or_else(|| bad("expansion binary"))?
        .into();
    p.initial = s["parent_endpoint"]["checkpoint_hash"]
        .as_str()
        .ok_or_else(|| bad("expansion physical"))?
        .into();
    p.initial_weights = s["parent_weights"]
        .as_str()
        .ok_or_else(|| bad("expansion weights"))?
        .into();
    p.config.budget_start_step = 2048;
    p.config.budget_start_tokens = state.consumed_tokens;
    p.config.max_steps = 3584;
    p.config.max_tokens = state.consumed_tokens + 4_300_000;
    p.config.warmup = 0;
    p.config.lr = 3e-4;
    let rows = expansion_rows(old, arm)?;
    p.identifiable = Some(Policy {
        study: study.into(),
        arm: arm.into(),
        dataset: EXPANSION_DATA.into(),
        rows: rows.clone(),
    });
    p.train_order = digest(&rows)?;
    p.order = vec![(0..1536).collect()];
    p.corpus = s["corpus"]
        .as_str()
        .ok_or_else(|| bad("expansion corpus"))?
        .into();
    p.metadata = s["metadata"]
        .as_str()
        .ok_or_else(|| bad("expansion metadata"))?
        .into();
    p.fork = Some(Fork {
        study: study.into(),
        study_hash: file_hash(&study.join("selection.r3b"))?,
        arm: arm.into(),
        parent_policy: digest(old)?,
        parent_state: digest(state)?,
        parent_adam: s["parent_adam"]
            .as_str()
            .ok_or_else(|| bad("expansion Adam"))?
            .into(),
        origin_step: 2048,
        origin_input: state.consumed_tokens,
        origin_target: state.target_tokens,
        original_corpus: old.corpus.clone(),
        tokenizer_training_hash: ByteBpe::load(
            &Path::new(s["parent"].as_str().unwrap()).join("tokenizer.r3b"),
        )?
        .train_hash,
        variants: None,
        variant_metadata: None,
        alternate_first: vec![],
        selector: None,
        selector_metadata: None,
        flip_first: vec![],
        constant_lr: 3e-4,
        target_limit: 60_000,
    });
    p.evaluation = evaluation_for(&p);
    Ok(p)
}
pub(in super::super) fn expansion_prepare(parent: &Path, output: &Path) -> Result<()> {
    if cfg!(feature = "test-support") {
        return Err(bad("production expansion binary required"));
    }
    let (parent, output) = (parent.canonicalize()?, std::path::absolute(output)?);
    let (old, end) = expansion_parent(&parent)?;
    let cp = parent.join(&end.checkpoint);
    let l = checkpoint::load(&cp, Device::Cpu, true)?;
    let state = l
        .manifest
        .training
        .as_ref()
        .ok_or_else(|| bad("expansion Adam state"))?;
    let (es, tm, data) = expansion_pool(&parent, &old)?;
    let c = verified_corpus(&parent.join("corpus.r3cor"), &old.corpus)?;
    let (_, dm, xm) = verified_metadata(&parent, &old)?;
    let native = orbit_native(es.clone(), c.validation)?;
    std::fs::create_dir(&output)?;
    let first = output.join("REPEAT");
    std::fs::create_dir(&first)?;
    data::native::write(&first.join("corpus.r3cor"), &native, true)?;
    write(&first.join("metadata.r3b"), &(tm, dm, xm))?;
    let selection = binary::record!({"contract":EXPANSION_CONTRACT,"source":source_digest()?,"binary":file_hash(&std::env::current_exe()?)?,
        "parent":parent,"parent_policy":file_hash(&parent.join("plan.r3b"))?,"parent_source":old.source,"parent_binary":old.binary,
        "parent_endpoint":end,"parent_content":l.model.weights_content_id()?,"parent_weights":l.model.weight_hash()?,
        "parent_adam":optimizer_hash(&l.optimizer)?,"parent_state":digest(state)?,"old_decision":file_hash(&parent.join("signal-decision-2048.r3b"))?,
        "corpus":file_hash(&first.join("corpus.r3cor"))?,"metadata":file_hash(&first.join("metadata.r3b"))?,
        "data":data,"seal_root":data["seal_root"],"seal":data["seal"],"sealed_corpus":data["sealed_corpus"],
        "origin":2048,"maximum":3584,"new_updates":3072,"observer_backwards":0,"counter_offset":0});
    write(&output.join("selection.r3b"), &selection)?;
    let mut arms = BTreeMap::new();
    for arm in EXPANSION_ARMS {
        let root = output.join(arm);
        if arm != "REPEAT" {
            std::fs::create_dir(&root)?;
            for name in ["corpus.r3cor", "metadata.r3b"] {
                copy_native(&first.join(name), &root.join(name))?;
            }
        }
        for name in ["transfer.r3cor", "tokenizer.r3b"] {
            copy_native(&parent.join(name), &root.join(name))?;
        }
        copy_native(&cp, &root.join("initial.r3m"))?;
        let p = expansion_plan(&old, state, &output, arm, &selection)?;
        write(&root.join("plan.r3b"), &p)?;
        expansion_verify_plan(&root, &p)?;
        if !p.parent_entry(&root.join("initial.r3m"), &l)? {
            return Err(bad("expansion native parent readback"));
        }
        let exposure = expansion_exposure(&own(&p).rows[2048..], arm, &es, &l.tokenizer)?;
        arms.insert(arm,binary::record!({"policy":file_hash(&root.join("plan.r3b"))?,"tape":p.train_order,
            "corpus":p.corpus,"metadata":p.metadata,"initial":p.initial,"content":selection["parent_content"],
            "adam":selection["parent_adam"],"tokenizer":p.tokenizer,"exposure":exposure}));
    }
    publish_confirmed(
        &output.join("preparation.r3b"),
        &binary::record!({"contract":EXPANSION_CONTRACT,
        "source":selection["source"],"binary":selection["binary"],"selection":file_hash(&output.join("selection.r3b"))?,
        "arms":arms,"data":data,"optimizer":0,"generation":0,"teacher":0}),
    )?;
    println!(
        "EXPANSION_PREPARED pool1536 old512 new1024 dev512 repeat512x24 rebind1536x8 updates3072 REVIEW_PENDING"
    );
    Ok(())
}
fn expansion_verify_plan(root: &Path, p: &Plan) -> Result<()> {
    let o = own(p);
    let s: binary::Value = read(&o.study.join("selection.r3b"))?;
    let parent = Path::new(
        s["parent"]
            .as_str()
            .ok_or_else(|| bad("expansion parent"))?,
    );
    let (old, end) = expansion_parent(parent)?;
    let (m, _) = checkpoint::metadata(&parent.join(&end.checkpoint))?;
    let state = m
        .training
        .as_ref()
        .ok_or_else(|| bad("expansion parent state"))?;
    let expected = expansion_plan(&old, state, &o.study, &o.arm, &s)?;
    if *p != expected
        || root != o.study.join(&o.arm)
        || s["contract"] != EXPANSION_CONTRACT
        || s["parent_endpoint"] != binary::record!(end)
        || s["parent_policy"] != file_hash(&parent.join("plan.r3b"))?
        || s["parent_source"] != old.source
        || s["parent_binary"] != old.binary
        || s["old_decision"] != file_hash(&parent.join("signal-decision-2048.r3b"))?
    {
        return Err(bad("expansion policy/parent/tape mismatch"));
    }
    let (es, ms, data) = expansion_pool(parent, &old)?;
    let c = verified_corpus(&root.join("corpus.r3cor"), &p.corpus)?;
    let parent_c = verified_corpus(&parent.join("corpus.r3cor"), &old.corpus)?;
    let (tm, dm, xm) = verified_metadata(root, p)?;
    let (_, parent_dm, parent_xm) = verified_metadata(parent, &old)?;
    if data != s["data"]
        || digest(&es)? != digest(&c.train)?
        || digest(&ms)? != digest(&tm)?
        || digest(&c.validation)? != digest(&parent_c.validation)?
        || digest(&(dm, xm))? != digest(&(parent_dm, parent_xm))?
        || file_hash(&root.join("initial.r3m"))? != end.checkpoint_hash
        || file_hash(&root.join("tokenizer.r3b"))? != file_hash(&parent.join("tokenizer.r3b"))?
    {
        return Err(bad("expansion frozen owned inputs"));
    }
    let tok = ByteBpe::load(&root.join("tokenizer.r3b"))?;
    expansion_exposure(&o.rows[2048..], &o.arm, &c.train, &tok)?;
    Ok(())
}
fn expansion_review(p: &Plan) -> Result<()> {
    let study = &own(p).study;
    let r: binary::Value = read_confirmed(&study.join("review-a.r3b"))?;
    if r["verdict"] != "PASS"
        || r["source"] != p.source
        || r["preparation"] != file_hash(&study.join("preparation.r3b"))?
        || r["report_hash"]
            != file_hash(Path::new(
                r["report_path"]
                    .as_str()
                    .ok_or_else(|| bad("expansion review path"))?,
            ))?
    {
        return Err(bad("independent expansion A required"));
    }
    Ok(())
}
fn expansion_parent_receipt(p: &Plan, name: &str, n: usize, t: usize) -> Result<binary::Value> {
    let study = &own(p).study;
    let r: binary::Value = read_confirmed(&study.join(format!("{name}-finished.r3b")))?;
    if r["checkpoint"] != p.initial
        || r["binding"]["source"] != p.source
        || r["binding"]["binary"] != p.binary
        || !r["error"].is_null()
        || r["control"]["terminal_reason"] != "COMPLETED"
        || r["control"]["generation_calls"] != n
        || r["control"]["teacher_calls"] != t
        || (name == "legacy"
            && (r["matched"] != 16 || r["raw"] != file_hash(&study.join("legacy.r3rows"))?))
    {
        return Err(bad("expansion parent observation incomplete"));
    }
    if name == "parent-new" {
        let path = study.join("eval-2048-new64.r3rows");
        let rows = binary::read_value_records(&path)?;
        let header = rows.first().ok_or_else(|| bad("parent observation header"))?;
        if r["raw"] != file_hash(&path)? || header["physical"] != p.initial
            || header["binding"]["model"] != p.initial_weights {
            return Err(bad("parent new64 differs from fixed native"));
        }
    }
    Ok(r)
}
fn expansion_authorize(root: &Path, p: &Plan) -> Result<()> {
    let study = &own(p).study;
    expansion_review(p)?;
    if root.canonicalize()? != study.join(&own(p).arm).canonicalize()? {
        return Err(bad("expansion registered root"));
    }
    let prep: binary::Value = read_confirmed(&study.join("preparation.r3b"))?;
    if prep["selection"] != file_hash(&study.join("selection.r3b"))? {
        return Err(bad("expansion reviewed selection changed"));
    }
    for arm in EXPANSION_ARMS {
        let r = study.join(arm);
        let pp = plan_read(&r)?;
        let h = history(&r, &pp)?;
        if h.last().is_some_and(|s| {
            s.phase.as_deref() == Some("Failed")
                || ![
                    "TRAINING",
                    "TIME_BUDGET",
                    "CANDIDATE_FIXED",
                    "FINAL_QUALITY_FAIL",
                ]
                .contains(&s.stop.as_str())
                || (!s.resume && (s.step != 3584 || s.phase.as_deref() != Some("Finished")))
        }) {
            return Err(bad("expansion failed peer remains closed"));
        }
        if own(p).arm == "REBIND"
            && arm == "REPEAT"
            && h.last().is_none_or(|s| {
                s.step != 3584 || s.resume || s.phase.as_deref() != Some("Finished")
            })
        {
            return Err(bad("normal REPEAT endpoint required"));
        }
    }
    expansion_parent_receipt(p, "legacy", 16, 0)?;
    let r = expansion_parent_receipt(p, "parent-new", 64, 64)?;
    let s: binary::Value = read(&study.join("selection.r3b"))?;
    let parent = Path::new(s["parent"].as_str().unwrap());
    let (old, _) = expansion_parent(parent)?;
    let c = verified_corpus(&root.join("corpus.r3cor"), &p.corpus)?;
    let (tm, _, _) = verified_metadata(root, p)?;
    let stats = signal_stats_cases(
        study,
        &old,
        2048,
        "new64",
        &c.train[512..576],
        &tm[512..576],
        64,
    )?;
    if r["statistics"] != stats || r["raw"] != stats["raw"] {
        return Err(bad("expansion parent new64/raw mismatch"));
    }
    Ok(())
}
pub(in super::super) fn expansion_parent_observe(study: &Path, new_pool: bool) -> Result<()> {
    let root = study.join("REPEAT");
    let p = plan_read(&root)?;
    if !is_expansion(&p) {
        return Err(bad("expansion parent observation scope"));
    }
    expansion_review(&p)?;
    let s: binary::Value = read(&study.join("selection.r3b"))?;
    let parent = Path::new(s["parent"].as_str().unwrap());
    let (old, end) = expansion_parent(parent)?;
    let c = verified_corpus(&root.join("corpus.r3cor"), &p.corpus)?;
    let (tm, dm, _) = verified_metadata(&root, &p)?;
    let tok = ByteBpe::load(&root.join("tokenizer.r3b"))?;
    if !new_pool {
        audit_panel(parent, &old, 2048, "dev512", &c.validation, &dm, &tok)?;
        let raw = binary::read_value_records(&parent.join("eval-2048-dev512.r3rows"))?;
        orbit_observe(
            study,
            "legacy",
            &root.join("initial.r3m"),
            &c.validation[..16],
            &binary::record!({"policy":digest(&p)?,"parent":parent,"checkpoint":end.checkpoint_hash}),
            Some(&raw[1..17]),
            observation_control(&p, 16, 0)?,
        )?;
        println!("EXPANSION_PARENT_PARITY matched16/16 optimizer0 teacher0");
        return Ok(());
    }
    expansion_parent_receipt(&p, "legacy", 16, 0)?;
    let mut control = observation_control(&p, 64, 64)?;
    let binding = binary::record!({"source":p.source,"binary":p.binary,"policy":digest(&p)?,"checkpoint":p.initial,
        "parent_policy":digest(&old)?,"cases":digest(&&c.train[512..576])?,"selection":"first16 new skeletons, both assignments/queries"});
    write(&study.join("parent-new-started.r3b"), &binding)?;
    // The copied native is still bound to its historical policy at step2048.
    // Reuse the exact evaluator with that policy, in a new observation root.
    let result = (|| -> Result<binary::Value> {
        copy_native(&root.join("tokenizer.r3b"), &study.join("tokenizer.r3b"))?;
        evaluate_panel(
            &old,
            study,
            &root.join("initial.r3m"),
            2048,
            "new64",
            &c.train[512..576],
            &tm[512..576],
            &mut control,
        )?;
        signal_stats_cases(
            study,
            &old,
            2048,
            "new64",
            &c.train[512..576],
            &tm[512..576],
            64,
        )
    })();
    if let Err(e) = &result {
        control.classify_error(e);
    }
    let terminal = control.seal_terminal();
    let result = result.and_then(|stats| terminal.map(|()| stats));
    let statistics = result.as_ref().ok();
    let raw = study.join("eval-2048-new64.r3rows");
    publish_confirmed(
        &study.join("parent-new-finished.r3b"),
        &binary::record!({"binding":binding,"checkpoint":p.initial,
        "statistics":statistics,"raw":if raw.exists(){Some(file_hash(&raw)?)}else{None},
        "control":control.receipt(),"error":result.as_ref().err().map(ToString::to_string)}),
    )?;
    println!("EXPANSION_PARENT_NEW64 {}", result?);
    Ok(())
}
pub(in super::super) fn expansion_endpoint(p: &Plan, step: usize, pending: bool) -> Result<usize> {
    if !is_expansion(p) || !(2048..=3584).contains(&step) {
        return Err(bad("expansion cursor"));
    }
    if pending && p.evaluation_due(step) {
        return Ok(step);
    }
    [2304, 2816, 3328, 3584]
        .into_iter()
        .find(|&s| s > step)
        .ok_or_else(|| bad("expansion budget closed"))
}
fn expansion_evaluation_result(root: &Path, p: &Plan, step: usize) -> Result<binary::Value> {
    let mut panels = BTreeMap::new();
    for (name, es, _) in panel_cases(root, p, step)? {
        panels.insert(name.clone(), signal_stats(root, p, step, &name, es.len())?);
    }
    let final_step = step == 3584;
    let mut streak = 0;
    for s in [2304, 2816].into_iter().filter(|&s| s <= step) {
        let old = orbit_panel(root, p, s, "old64")?;
        let dev = orbit_panel(root, p, s, "dev64")?;
        streak = if old.full < 32 && dev.full < 16 {
            streak + 1
        } else {
            0
        };
    }
    let eligible = if final_step {
        let old = orbit_panel(root, p, step, "old512")?;
        let new = orbit_panel(root, p, step, "new1024")?;
        let dev = orbit_panel(root, p, step, "dev512")?;
        orbit_candidate_scores(&old, &dev)
            && (own(p).arm == "REPEAT"
                || (new.full >= 1016
                    && new.query_both >= 504
                    && new.all4 >= 248
                    && new.errors == 0
                    && new.eos == 1024))
    } else {
        false
    };
    let stop = if streak >= 2 {
        Some("QUALITY_REGRESSION")
    } else if final_step {
        Some(if eligible {
            "CANDIDATE_FIXED"
        } else {
            "FINAL_QUALITY_FAIL"
        })
    } else {
        None
    };
    Ok(
        binary::record!({"policy":digest(p)?,"step":step,"panels":panels,"regression_streak":streak,"eligible":eligible,"stop":stop}),
    )
}
fn expansion_evaluation_decision(root: &Path, p: &Plan, step: usize) -> Result<Option<String>> {
    let d = expansion_evaluation_result(root, p, step)?;
    let path = root.join(format!("expansion-decision-{step:04}.r3b"));
    if path.exists() {
        let r: binary::Value = read_confirmed(&path)?;
        if r != d {
            return Err(bad("expansion decision/raw mismatch"));
        }
    } else {
        publish_confirmed(&path, &d)?;
    }
    println!("EXPANSION_DECISION {d}");
    Ok(d["stop"].as_str().map(str::to_owned))
}
fn expansion_comparison(study: &Path) -> Result<binary::Value> {
    let mut endpoints = BTreeMap::new();
    let mut scores = vec![];
    let mut candidates = vec![];
    for arm in EXPANSION_ARMS {
        let root = study.join(arm);
        let p = plan_read(&root)?;
        authorize(&root, &p)?;
        let h = history(&root, &p)?;
        let end = h.last().ok_or_else(|| bad("expansion endpoint absent"))?;
        if end.step != 3584 || end.resume || end.phase.as_deref() != Some("Finished") {
            return Err(bad("expansion matched endpoint incomplete"));
        }
        let d = expansion_evaluation_result(&root, &p, 3584)?;
        let saved: binary::Value = read_confirmed(&root.join("expansion-decision-3584.r3b"))?;
        if d != saved || d["stop"] != end.stop {
            return Err(bad("expansion terminal/raw disagreement"));
        }
        let old = orbit_panel(&root, &p, 3584, "old512")?;
        let new = orbit_panel(&root, &p, 3584, "new1024")?;
        let dev = orbit_panel(&root, &p, 3584, "dev512")?;
        let parity = parity_verified(&root, &p)?;
        if d["eligible"] == true && parity {
            candidates.push((arm, dev.all4, dev.query_both, dev.full, old.full));
        }
        endpoints.insert(
            arm,
            binary::record!({"policy":digest(&p)?,"checkpoint":end.checkpoint_hash,"step":3584,
            "old":old,"new":new,"dev":dev,"eligible":d["eligible"],"parity":parity}),
        );
        scores.push(dev);
    }
    let mut gain = 0;
    let mut loss = 0;
    let mut differences = vec![];
    for (a, b) in scores[0]
        .exact
        .as_chunks::<4>()
        .0
        .iter()
        .zip(scores[1].exact.as_chunks::<4>().0)
    {
        gain += usize::from(!a.iter().all(|&v| v) && b.iter().all(|&v| v));
        loss += usize::from(a.iter().all(|&v| v) && !b.iter().all(|&v| v));
        differences.push(
            (b.iter().filter(|&&v| v).count() as f64 - a.iter().filter(|&&v| v).count() as f64)
                / 4.,
        );
    }
    candidates.sort_by(|a, b| {
        b.1.cmp(&a.1)
            .then(b.2.cmp(&a.2))
            .then(b.3.cmp(&a.3))
            .then(b.4.cmp(&a.4))
            .then_with(|| {
                if a.0 == "REPEAT" {
                    std::cmp::Ordering::Less
                } else {
                    std::cmp::Ordering::Greater
                }
            })
    });
    Ok(
        binary::record!({"contract":EXPANSION_CONTRACT,"source":source_digest()?,"preparation":file_hash(&study.join("preparation.r3b"))?,
        "endpoints":endpoints,"selected":candidates.first().map(|v|v.0),"dev_all4_gain":gain,"dev_all4_loss":loss,
        "dev_skeleton_full_fraction_differences":differences,"independent_units":128,
        "selection_rule":"eligible final3584 only; dev ALL4/QB/FULL, old FULL, REPEAT tie",
        "minimal_baseline_verified":false,"confirmation":"NOT_OPENED","goal1_ready":false}),
    )
}
pub(in super::super) fn expansion_report(study: &Path) -> Result<()> {
    let mut complete = true;
    for arm in EXPANSION_ARMS {
        let root = study.join(arm);
        let p = plan_read(&root)?;
        let h = history(&root, &p)?;
        let Some(end) = h.last() else {
            complete = false;
            println!("EXPANSION_RESULT arm={arm} NOT_RUN");
            continue;
        };
        for &step in p.evaluation.train_steps.iter().filter(|&&s| s <= end.step) {
            if step == end.step && end.phase.as_deref() == Some("EvaluationPending") {
                continue;
            }
            let d = expansion_evaluation_result(&root, &p, step)?;
            let r: binary::Value =
                read_confirmed(&root.join(format!("expansion-decision-{step:04}.r3b")))?;
            if d != r {
                return Err(bad("expansion report/raw disagreement"));
            }
            println!("EXPANSION_PANEL arm={arm} {d}");
        }
        println!(
            "EXPANSION_RESULT arm={arm} step={} new_updates={} durable={} stop={} resume={} usage={:?}",
            end.step,
            end.step - 2048,
            end.checkpoint_hash,
            end.stop,
            end.resume,
            usage(&p)?
        );
        complete &= end.step == 3584 && !end.resume && end.phase.as_deref() == Some("Finished");
    }
    if complete {
        println!("EXPANSION_COMPARISON {}", expansion_comparison(study)?);
    }
    Ok(())
}

// A single continuation of a closed endpoint, with no retrospective admission.
fn consolidation_parent(root: &Path, tiny: bool) -> Result<(Plan, Segment)> {
    if tiny && !cfg!(test) {
        return Err(bad("TINY consolidation is a direct test fixture"));
    }
    let raw: Plan = read(&root.join("plan.r3b"))?;
    let p = plan_read_bound(root, &raw.source, &raw.binary)?;
    let h = history(root, &p)?;
    let end = h
        .last()
        .cloned()
        .ok_or_else(|| bad("consolidation parent absent"))?;
    let valid = if tiny {
        own(&p).dataset == ORBIT_DATA
            && own(&p).arm == "BOTH"
            && end.step == 2
            && end.stop == "BUDGET_REACHED"
    } else {
        is_expansion(&p)
            && own(&p).arm == "REBIND"
            && end.step == 3584
            && end.stop == "FINAL_QUALITY_FAIL"
            && p.config.lr == 3e-4
            && p.config.warmup == 0
    };
    if !valid
        || p.tiny != tiny
        || end.resume
        || end.phase.as_deref() != Some("Finished")
        || p.framing() != neural::Framing::QuestionEvidence
        || p.config.first_target_weight != 1.
        || own(&p).rows.len() != end.step
    {
        return Err(bad("closed REBIND3584 weights/Adam required"));
    }
    if !tiny {
        let d: binary::Value = read_confirmed(&root.join("expansion-decision-3584.r3b"))?;
        if d != expansion_evaluation_result(root, &p, 3584)? {
            return Err(bad("consolidation parent decision/raw mismatch"));
        }
    }
    Ok((p, end))
}
fn consolidation_rows(old: &Plan) -> Result<Vec<[usize; 8]>> {
    let (origin, suffix) = if old.tiny { (2, 0) } else { (3584, 2048) };
    if own(old).rows.len() != origin {
        return Err(bad("consolidation tape origin"));
    }
    Ok([
        own(old).rows.clone(),
        own(old).rows[suffix..origin].to_vec(),
    ]
    .concat())
}
fn consolidation_trace(root: &Path, p: &Plan, end: &Segment) -> Result<binary::Value> {
    let mut cursor = p.origin_step();
    let mut input = 0u64;
    let mut target = 0u64;
    let mut files = BTreeMap::new();
    for index in 0..128 {
        let terminal = root.join(format!("segment-{index:04}-finished.r3b"));
        if !terminal.exists() {
            break;
        }
        let segment: Segment = read_confirmed(&terminal)?;
        let path = root.join(format!("segment-{index:04}/updates.r3rows"));
        if path.exists() {
            for row in binary::read_value_records(&path)? {
                cursor += 1;
                if cursor > own(p).rows.len()
                    || row["step"] != cursor
                    || row["sampler"] != cursor
                    || row["sample_indices"] != binary::record!(p.training_draw(cursor - 1))
                    || row["lr_bits"] != p.learning_rate(cursor).to_bits()
                    || row["input"] != 1160
                    || row["target"] != 16
                    || ["ce", "objective", "grad_norm", "delta_norm"]
                        .iter()
                        .any(|key| row[*key].as_f64().is_none_or(|v| !v.is_finite()))
                {
                    return Err(bad("actual continuation trace/tape/LR/token mismatch"));
                }
                input += 1160;
                target += 16;
            }
            files.insert(index, file_hash(&path)?);
        }
        if cursor != segment.step {
            return Err(bad("missing/extra committed update trace"));
        }
    }
    let (origin_input, origin_target) = p
        .fork
        .as_ref()
        .map_or((0, 0), |f| (f.origin_input, f.origin_target));
    if cursor != end.step
        || origin_input.checked_add(input) != Some(end.input)
        || origin_target.checked_add(target) != Some(end.target)
    {
        return Err(bad("trace cumulative native counters"));
    }
    Ok(
        binary::record!({"files":files,"origin":p.origin_step(),"last":cursor,
        "updates":cursor-p.origin_step(),"input":input,"target":target}),
    )
}
fn consolidation_baselines(parent: &Path, old: &Plan, step: usize) -> Result<binary::Value> {
    let tok = ByteBpe::load(&parent.join("tokenizer.r3b"))?;
    let mut out = BTreeMap::new();
    for (name, es, ms) in panel_cases(parent, old, step)? {
        audit_panel(parent, old, step, &name, &es, &ms, &tok)?;
        let raw_path = parent.join(format!("eval-{step:04}-{name}.r3rows"));
        let raw = binary::read_value_records(&raw_path)?;
        let full = orbit_score(&es, &ms, &raw[1..], &tok)?;
        let n = if old.tiny { 4 } else { 64 };
        let screen = orbit_score(&es[..n], &ms[..n], &raw[1..n + 1], &tok)?;
        let keys = if old.tiny && name == "train4" {
            vec!["old", "new"]
        } else if name.starts_with("old") {
            vec!["old"]
        } else if name.starts_with("new") {
            vec!["new"]
        } else {
            vec!["dev"]
        };
        for key in keys {
            out.insert(key, binary::record!({"panel":name,"raw":file_hash(&raw_path)?,
                "teacher":file_hash(&parent.join(format!("eval-{step:04}-{name}-teachers.r3rows")))?,
                "full":full,"screen":screen,"cases":digest(&es)?,"screen_cases":digest(&&es[..n])?,
                "screen_ids":es[..n].iter().map(|e|&e.id).collect::<Vec<_>>(),
                "selection":"first16 metadata skeletons, both assignments and queries; TINY first1"}));
        }
    }
    Ok(binary::record!(out))
}
fn consolidation_plan(
    old: &Plan,
    state: &TrainingState,
    study: &Path,
    s: &binary::Value,
) -> Result<Plan> {
    let text = |key: &str| -> Result<String> {
        Ok(s[key]
            .as_str()
            .ok_or_else(|| bad("consolidation identity absent"))?
            .into())
    };
    let mut p = old.clone();
    p.source = text("source")?;
    p.binary = text("binary")?;
    p.initial = s["parent_endpoint"]["checkpoint_hash"]
        .as_str()
        .ok_or_else(|| bad("parent physical"))?
        .into();
    p.initial_weights = text("parent_weights")?;
    let origin = if old.tiny { 2 } else { 3584 };
    p.config.budget_start_step = origin;
    p.config.budget_start_tokens = state.consumed_tokens;
    p.config.max_steps = if old.tiny { 4 } else { 5120 };
    p.config.max_tokens = state
        .consumed_tokens
        .checked_add(2_100_000)
        .ok_or_else(|| bad("input budget overflow"))?;
    p.config.warmup = 0;
    p.config.lr = 3e-4;
    let rows = consolidation_rows(old)?;
    p.train_order = digest(&rows)?;
    p.identifiable = Some(Policy {
        study: study.into(),
        arm: CONTINUE_ARM.into(),
        dataset: CONSOLIDATION_DATA.into(),
        rows,
    });
    let tok = ByteBpe::load(&Path::new(&text("parent")?).join("tokenizer.r3b"))?;
    p.fork = Some(Fork {
        study: study.into(),
        study_hash: file_hash(&study.join("selection.r3b"))?,
        arm: CONTINUE_ARM.into(),
        parent_policy: digest(old)?,
        parent_state: digest(state)?,
        parent_adam: text("parent_adam")?,
        origin_step: origin,
        origin_input: state.consumed_tokens,
        origin_target: state.target_tokens,
        original_corpus: old.corpus.clone(),
        tokenizer_training_hash: tok.train_hash,
        variants: None,
        variant_metadata: None,
        alternate_first: vec![],
        selector: None,
        selector_metadata: None,
        flip_first: vec![],
        constant_lr: 3e-4,
        target_limit: 30_000,
    });
    p.evaluation = evaluation_for(&p);
    Ok(p)
}
pub(in super::super) fn consolidation_prepare(
    parent: &Path,
    output: &Path,
    tiny: bool,
) -> Result<()> {
    if cfg!(feature = "test-support") && !tiny {
        return Err(bad("production consolidation binary required"));
    }
    let (parent, output) = (parent.canonicalize()?, std::path::absolute(output)?);
    let (old, end) = consolidation_parent(&parent, tiny)?;
    let cp = parent.join(&end.checkpoint);
    let l = checkpoint::load(&cp, Device::Cpu, true)?;
    let state = l
        .manifest
        .training
        .as_ref()
        .ok_or_else(|| bad("parent Adam state"))?;
    if state.step != end.step
        || state.sampler_state != end.step as u64
        || state.resume_binding.as_ref() != Some(&old.binding(state, &l.tokenizer)?)
    {
        return Err(bad("parent native clock/cursor/policy"));
    }
    let baseline = consolidation_baselines(&parent, &old, end.step)?;
    let parent_trace = consolidation_trace(&parent, &old, &end)?;
    let prior: binary::Value = read(&own(&old).study.join("selection.r3b"))?;
    let seal_root = if tiny {
        own(&old).study.to_string_lossy().into_owned()
    } else {
        prior["seal_root"]
            .as_str()
            .ok_or_else(|| bad("existing seal root"))?
            .into()
    };
    let selection = binary::record!({"contract":CONSOLIDATION_CONTRACT,"source":source_digest()?,
        "binary":file_hash(&std::env::current_exe()?)?,"parent":parent,"parent_policy":file_hash(&parent.join("plan.r3b"))?,
        "parent_source":old.source,"parent_binary":old.binary,"parent_endpoint":end,
        "parent_content":l.model.weights_content_id()?,"parent_weights":l.model.weight_hash()?,
        "parent_adam":optimizer_hash(&l.optimizer)?,"parent_state":digest(state)?,
        "parent_terminal":file_hash(&parent.join(format!("segment-{:04}-finished.r3b",history(&parent,&old)?.len()-1)))?,
        "old_decision":if tiny {None}else{Some(file_hash(&parent.join("expansion-decision-3584.r3b"))?)},
        "corpus":old.corpus,"metadata":old.metadata,"baseline":baseline,"parent_trace":parent_trace,"seal_root":seal_root,
        "seal":file_hash(&Path::new(&seal_root).join("confirmation-seal.r3b"))?,
        "origin":end.step,"maximum":if tiny {4}else{5120},"new_updates":if tiny {2}else{1536},
        "input_limit":2100000,"target_limit":30000,"observer_backwards":0});
    std::fs::create_dir(&output)?;
    write(&output.join("selection.r3b"), &selection)?;
    let root = output.join(CONTINUE_ARM);
    std::fs::create_dir(&root)?;
    for name in [
        "corpus.r3cor",
        "metadata.r3b",
        "transfer.r3cor",
        "tokenizer.r3b",
    ] {
        copy_native(&parent.join(name), &root.join(name))?;
    }
    copy_native(&cp, &root.join("initial.r3m"))?;
    let p = consolidation_plan(&old, state, &output, &selection)?;
    write(&root.join("plan.r3b"), &p)?;
    consolidation_verify_plan(&root, &p)?;
    if !p.parent_entry(&root.join("initial.r3m"), &l)? {
        return Err(bad("native continuation parent readback"));
    }
    let c = verified_corpus(&root.join("corpus.r3cor"), &p.corpus)?;
    let exposure = if tiny {
        binary::record!({"updates":2,"explicit_fixture":true})
    } else {
        expansion_exposure(&own(&p).rows[3584..], "REBIND", &c.train, &l.tokenizer)?
    };
    let mut counts = vec![0usize; c.train.len()];
    for row in own(&p)
        .rows
        .iter()
        .skip(p.origin_step())
        .take(if tiny { 1 } else { 768 })
    {
        for &i in row {
            counts[i] += 1;
        }
    }
    let arm = binary::record!({"policy":file_hash(&root.join("plan.r3b"))?,"tape":p.train_order,
        "corpus":p.corpus,"metadata":p.metadata,"initial":p.initial,"content":selection["parent_content"],
        "adam":selection["parent_adam"],"tokenizer":p.tokenizer,"exposure":exposure,"midpoint_counts":counts});
    publish_confirmed(
        &output.join("preparation.r3b"),
        &binary::record!({"contract":CONSOLIDATION_CONTRACT,
        "source":p.source,"binary":p.binary,"selection":file_hash(&output.join("selection.r3b"))?,
        "arms":BTreeMap::from([(CONTINUE_ARM,arm)]),"optimizer":0,"generation":0,"teacher":0}),
    )?;
    println!(
        "CONSOLIDATION_PREPARED origin={} max={} tape={} corpus={} REVIEW_PENDING",
        p.origin_step(),
        p.config.max_steps,
        p.train_order,
        p.corpus
    );
    Ok(())
}
fn consolidation_verify_plan(root: &Path, p: &Plan) -> Result<()> {
    let o = own(p);
    if p.origin_step() != if p.tiny { 2 } else { 3584 }
        || p.config.max_steps != if p.tiny { 4 } else { 5120 }
        || p.config.lr != 3e-4
        || p.config.warmup != 0
        || o.rows.len() != p.config.max_steps
    {
        return Err(bad("consolidation absolute clock/LR/ceiling"));
    }
    let s: binary::Value = read(&o.study.join("selection.r3b"))?;
    let parent = Path::new(
        s["parent"]
            .as_str()
            .ok_or_else(|| bad("consolidation parent root"))?,
    );
    let (old, end) = consolidation_parent(parent, p.tiny)?;
    let (m, _) = checkpoint::metadata(&parent.join(&end.checkpoint))?;
    let state = m
        .training
        .as_ref()
        .ok_or_else(|| bad("parent state missing"))?;
    if *p != consolidation_plan(&old, state, &o.study, &s)?
        || root != o.study.join(CONTINUE_ARM)
        || s["contract"] != CONSOLIDATION_CONTRACT
        || s["parent_endpoint"] != binary::record!(end)
        || s["parent_source"] != old.source
        || s["parent_binary"] != old.binary
        || s["parent_policy"] != file_hash(&parent.join("plan.r3b"))?
        || s["parent_state"] != digest(state)?
        || s["corpus"] != old.corpus
        || s["metadata"] != old.metadata
        || s["parent_terminal"]
            != file_hash(&parent.join(format!(
                "segment-{:04}-finished.r3b",
                history(parent, &old)?.len() - 1
            )))?
        || (!p.tiny && s["old_decision"] != file_hash(&parent.join("expansion-decision-3584.r3b"))?)
        || s["baseline"] != consolidation_baselines(parent, &old, end.step)?
        || s["parent_trace"] != consolidation_trace(parent, &old, &end)?
    {
        return Err(bad("consolidation frozen parent/policy/tape mismatch"));
    }
    for name in [
        "corpus.r3cor",
        "metadata.r3b",
        "transfer.r3cor",
        "tokenizer.r3b",
    ] {
        if file_hash(&root.join(name))? != file_hash(&parent.join(name))? {
            return Err(bad("consolidation changed owned input"));
        }
    }
    if file_hash(&root.join("initial.r3m"))? != end.checkpoint_hash
        || s["seal"]
            != file_hash(
                &Path::new(s["seal_root"].as_str().ok_or_else(|| bad("seal root"))?)
                    .join("confirmation-seal.r3b"),
            )?
    {
        return Err(bad("consolidation native/seal identity"));
    }
    let c = verified_corpus(&root.join("corpus.r3cor"), &p.corpus)?;
    let tok = ByteBpe::load(&root.join("tokenizer.r3b"))?;
    if !p.tiny {
        expansion_exposure(&o.rows[3584..], "REBIND", &c.train, &tok)?;
    }
    Ok(())
}
fn consolidation_parent_receipt(p: &Plan) -> Result<()> {
    let study = &own(p).study;
    let r: binary::Value = read_confirmed(&study.join("legacy-finished.r3b"))?;
    let n = if p.tiny { 4 } else { 16 };
    if r["checkpoint"] != p.initial
        || r["policy"] != digest(p)?
        || r["matched"] != n
        || !r["error"].is_null()
        || r["raw"] != file_hash(&study.join("legacy.r3rows"))?
        || r["control"]["terminal_reason"] != "COMPLETED"
        || r["control"]["generation_calls"] != n
        || r["control"]["teacher_calls"] != 0
    {
        return Err(bad("consolidation actual parent parity required"));
    }
    Ok(())
}
pub(in super::super) fn consolidation_parent_parity(study: &Path) -> Result<()> {
    let root = study.join(CONTINUE_ARM);
    let p = plan_read(&root)?;
    if !is_consolidation(&p) {
        return Err(bad("consolidation parity scope"));
    }
    expansion_review(&p)?;
    let s: binary::Value = read(&study.join("selection.r3b"))?;
    let parent = Path::new(s["parent"].as_str().ok_or_else(|| bad("parent path"))?);
    let (old, end) = consolidation_parent(parent, p.tiny)?;
    let c = verified_corpus(&root.join("corpus.r3cor"), &p.corpus)?;
    let name = if p.tiny { "dev4" } else { "dev512" };
    let raw =
        binary::read_value_records(&parent.join(format!("eval-{:04}-{name}.r3rows", end.step)))?;
    let n = if p.tiny { 4 } else { 16 };
    orbit_observe(
        study,
        "legacy",
        &root.join("initial.r3m"),
        &c.validation[..n],
        &binary::record!({"policy":digest(&p)?,"parent_policy":digest(&old)?,"checkpoint":p.initial}),
        Some(&raw[1..n + 1]),
        observation_control(&p, n, 0)?,
    )?;
    println!("CONSOLIDATION_PARENT_PARITY matched{n}/{n} optimizer0 teacher0");
    Ok(())
}
pub(in super::super) fn consolidation_endpoint(
    p: &Plan,
    step: usize,
    pending: bool,
) -> Result<usize> {
    if !is_consolidation(p) || step < p.origin_step() || step > p.config.max_steps {
        return Err(bad("consolidation cursor"));
    }
    if pending && p.evaluation_due(step) {
        return Ok(step);
    }
    let endpoints = if p.tiny {
        vec![4]
    } else {
        vec![3840, 4352, 4864, 5120]
    };
    endpoints
        .into_iter()
        .find(|&s| s > step)
        .ok_or_else(|| bad("consolidation budget closed"))
}
fn consolidation_action(
    step: usize,
    eligible: bool,
    regression: bool,
) -> Result<(&'static str, bool)> {
    if ![3840, 4352, 5120].contains(&step) {
        return Err(bad("unsupported consolidation decision step"));
    }
    if regression {
        return Ok(("QUALITY_REGRESSION", false));
    }
    Ok(match (step, eligible) {
        (4352, true) => ("CANDIDATE_FIXED_AT_4352", false),
        (5120, true) => ("CANDIDATE_FIXED_AT_5120", false),
        (5120, false) => ("FINAL_QUALITY_FAIL_AT_5120", false),
        _ => ("CONTINUE_WITHIN_REGISTERED_CAP", true),
    })
}
fn consolidation_regression(now: &[OrbitScore], parent: &[OrbitScore]) -> Result<bool> {
    if now.len() != 3 || parent.len() != 3 {
        return Err(bad("three complete consolidation panels required"));
    }
    let mut drops = 0;
    for (a, b) in now.iter().zip(parent) {
        if a.total != b.total
            || a.total == 0
            || a.total % 4 != 0
            || a.exact.len() != a.total
            || b.exact.len() != b.total
        {
            return Err(bad("consolidation denominator/raw mismatch"));
        }
        drops += usize::from(
            b.full.saturating_sub(a.full) * 10 >= a.total
                && b.all4.saturating_sub(a.all4) * 10 >= a.total / 4,
        );
    }
    Ok(now.iter().map(|s| s.errors).sum::<usize>() >= 16 || drops >= 2)
}
fn consolidation_eligible(scores: &[OrbitScore]) -> bool {
    scores.len() == 3
        && orbit_candidate_scores(&scores[0], &scores[2])
        && scores[1].total == 1024
        && scores[1].full >= 1016
        && scores[1].query_both >= 504
        && scores[1].all4 >= 248
        && scores[1].errors == 0
        && scores[1].eos == 1024
}
fn consolidation_gains(now: &OrbitScore, parent: &OrbitScore) -> Result<binary::Value> {
    if now.exact.len() != parent.exact.len() {
        return Err(bad("trajectory case count"));
    }
    let (mut gain, mut loss, mut all_gain, mut all_loss) = (0, 0, 0, 0);
    for (&a, &b) in parent.exact.iter().zip(&now.exact) {
        gain += usize::from(!a && b);
        loss += usize::from(a && !b);
    }
    for (a, b) in parent
        .exact
        .as_chunks::<4>()
        .0
        .iter()
        .zip(now.exact.as_chunks::<4>().0)
    {
        let (a, b) = (a.iter().all(|&v| v), b.iter().all(|&v| v));
        all_gain += usize::from(!a && b);
        all_loss += usize::from(a && !b);
    }
    Ok(
        binary::record!({"full_gain":gain,"full_loss":loss,"all4_gain":all_gain,"all4_loss":all_loss,
        "independent_units":now.total/4,"comparison":"same cases, parent trajectory; not equal-budget A/B"}),
    )
}
fn consolidation_evaluation_result(root: &Path, p: &Plan, step: usize) -> Result<binary::Value> {
    let s: binary::Value = read(&own(p).study.join("selection.r3b"))?;
    let mut panels = BTreeMap::new();
    let mut scores = vec![];
    let mut parent = vec![];
    let mut models = BTreeSet::new();
    for (name, es, _) in panel_cases(root, p, step)? {
        let mut stats = signal_stats(root, p, step, &name, es.len())?;
        let score: OrbitScore = binary::from_value(stats["score"].clone())?;
        let key = if name.starts_with("old") {
            "old"
        } else if name.starts_with("new") {
            "new"
        } else {
            "dev"
        };
        let screen = step == 3840 || p.tiny;
        if stats["cases"] != s["baseline"][key][if screen { "screen_cases" } else { "cases" }] {
            return Err(bad("parent/current case order mismatch"));
        }
        let base: OrbitScore =
            binary::from_value(s["baseline"][key][if screen { "screen" } else { "full" }].clone())?;
        stats["parent_change"] = consolidation_gains(&score, &base)?;
        let raw = binary::read_value_records(&root.join(format!("eval-{step:04}-{name}.r3rows")))?;
        models.insert(
            raw[0]["binding"]["model"]
                .as_str()
                .ok_or_else(|| bad("model absent"))?
                .to_owned(),
        );
        panels.insert(name, stats);
        scores.push(score);
        parent.push(base);
    }
    if models.len() != 1 {
        return Err(bad("mixed consolidation panel models"));
    }
    let regression = consolidation_regression(&scores, &parent)?;
    let eligible = !p.tiny && step != 3840 && consolidation_eligible(&scores);
    let (action, extend) = if p.tiny {
        ("FINAL_QUALITY_FAIL", false)
    } else {
        consolidation_action(step, eligible, regression)?
    };
    let stop = (!extend).then_some(action);
    Ok(
        binary::record!({"policy":digest(p)?,"step":step,"model":models.into_iter().next(),"panels":panels,
        "regression":regression,"eligible":eligible&&!regression,"action":action,"extend":extend,"stop":stop}),
    )
}
fn consolidation_decision(root: &Path, p: &Plan, step: usize) -> Result<Option<String>> {
    let d = consolidation_evaluation_result(root, p, step)?;
    let path = root.join(format!("consolidation-decision-{step:04}.r3b"));
    if path.exists() {
        if read_confirmed::<binary::Value>(&path)? != d {
            return Err(bad("consolidation decision/raw mismatch"));
        }
    } else {
        publish_confirmed(&path, &d)?;
    }
    println!("CONSOLIDATION_DECISION {d}");
    Ok(d["stop"].as_str().map(str::to_owned))
}
fn consolidation_close(root: &Path) -> Result<(Plan, Segment, binary::Value)> {
    let p = plan_read(root)?;
    consolidation_close_bound(root, p)
}
// Historical readers retain the recorded source/binary identity. Training and
// candidate publication still enter through plan_read's current-build binding.
pub(in super::super) fn historical_plan(root: &Path) -> Result<Plan> {
    let root = root.canonicalize()?;
    let p: Plan = read(&root.join("plan.r3b"))?;
    plan_read_bound(&root, &p.source, &p.binary)
}
fn consolidation_reproduction_endpoint(end: &Segment, d: &binary::Value, reviewer: bool, tiny: bool) -> Result<()> {
    let candidate = !tiny && [4352, 5120].contains(&end.step)
        && end.stop == format!("CANDIDATE_FIXED_AT_{}", end.step) && d["eligible"] == true;
    let quality_failure = reviewer && end.step == (if tiny {4} else {5120})
        && end.stop == (if tiny {"FINAL_QUALITY_FAIL"} else {"FINAL_QUALITY_FAIL_AT_5120"}) && d["eligible"] == false;
    if end.resume || end.phase.as_deref() != Some("Finished") || !(candidate || quality_failure)
        || d["step"] != end.step || d["stop"] != end.stop || d["action"] != end.stop
        || d["extend"] != false || d["regression"] != false {
        return Err(bad("normal complete consolidation endpoint required; reproduction grants no candidate/resume permission"));
    }
    Ok(())
}
fn consolidation_close_bound(root: &Path, p: Plan) -> Result<(Plan, Segment, binary::Value)> {
    if !is_consolidation(&p) {
        return Err(bad("consolidation close scope"));
    }
    authorize(root, &p)?;
    let h = history(root, &p)?;
    let end = h
        .last()
        .cloned()
        .ok_or_else(|| bad("consolidation endpoint absent"))?;
    if end.resume || end.phase.as_deref() != Some("Finished") || !p.evaluation_due(end.step) {
        return Err(bad("consolidation endpoint incomplete"));
    }
    let d = consolidation_evaluation_result(root, &p, end.step)?;
    consolidation_trace(root, &p, &end)?;
    if d != read_confirmed::<binary::Value>(
        &root.join(format!("consolidation-decision-{:04}.r3b", end.step)),
    )? || d["stop"] != end.stop
    {
        return Err(bad("consolidation terminal/raw disagreement"));
    }
    Ok((p, end, d))
}
fn consolidation_comparison(study: &Path) -> Result<binary::Value> {
    let root = study.join(CONTINUE_ARM);
    let (p, end, d) = consolidation_close(&root)?;
    consolidation_comparison_bound(study, &p, &end, &d)
}
fn consolidation_comparison_bound(study: &Path, p: &Plan, end: &Segment, d: &binary::Value) -> Result<binary::Value> {
    let root = study.join(CONTINUE_ARM);
    let parity = if d["eligible"] == true {
        parity_verified(&root, &p)?
    } else {
        false
    };
    let endpoint = binary::record!({"policy":digest(&p)?,"checkpoint":end.checkpoint_hash,"step":end.step,
        "decision":d,"parity":parity});
    Ok(
        binary::record!({"contract":CONSOLIDATION_CONTRACT,"source":p.source,
        "preparation":file_hash(&study.join("preparation.r3b"))?,"endpoints":BTreeMap::from([(CONTINUE_ARM,endpoint)]),
        "selected":if d["eligible"]==true&&parity {Some(CONTINUE_ARM)}else{None},
        "minimal_baseline_verified":false,"confirmation":"NOT_OPENED","goal1_ready":false}),
    )
}
pub(in super::super) fn consolidation_report(study: &Path) -> Result<()> {
    let root = study.join(CONTINUE_ARM);
    let p = historical_plan(&root)?;
    let h = history(&root, &p)?;
    let Some(end) = h.last() else {
        println!("CONSOLIDATION NOT_RUN");
        return Ok(());
    };
    for &step in p.evaluation.train_steps.iter().filter(|&&s| s <= end.step) {
        if step == end.step && end.phase.as_deref() == Some("EvaluationPending") {
            continue;
        }
        let d = consolidation_evaluation_result(&root, &p, step)?;
        if d != read_confirmed::<binary::Value>(
            &root.join(format!("consolidation-decision-{step:04}.r3b")),
        )? {
            return Err(bad("consolidation report/raw disagreement"));
        }
        println!("CONSOLIDATION_PANEL {d}");
    }
    println!(
        "CONSOLIDATION_RESULT step={} new_updates={} durable={} stop={} resume={} usage={:?}",
        end.step,
        end.step - p.origin_step(),
        end.checkpoint_hash,
        end.stop,
        end.resume,
        work(&p)?
    );
    if !end.resume && end.phase.as_deref() == Some("Finished") {
        let (_, checked_end, decision) = consolidation_close_bound(&root, p.clone())?;
        let comparison = consolidation_comparison_bound(study, &p, &checked_end, &decision)?;
        println!("CONSOLIDATION_HISTORICAL_COMPARISON {comparison}");
        println!("CONSOLIDATION_CONFIRMATION_CURRENT {}", confirmation_current(study, &p, &checked_end, &comparison));
    }
    Ok(())
}

fn signal_config(old:&Plan,state:&TrainingState) -> TrainConfig {
    let mut c=old.config.clone();
    c.budget_start_step=state.step;c.budget_start_tokens=state.consumed_tokens;
    c.max_steps=if old.tiny {4}else{2048};
    c.max_tokens=state.consumed_tokens+2_100_000;
    c.warmup=0;c.lr=3e-4;
    c
}
pub(in super::super) fn signal_prepare(parent:&Path,output:&Path,tiny:bool)->Result<()> {
    if cfg!(feature="test-support") && !tiny {return Err(bad("production signal binary required"));}
    let (old,end)=signal_parent(parent)?;
    if old.tiny!=tiny {return Err(bad("signal parent profile"));}
    let cp=parent.join(&end.checkpoint);
    let l=checkpoint::load(&cp,Device::Cpu,true)?;
    let state=l.manifest.training.as_ref().ok_or_else(||bad("parent Adam required"))?;
    let previous:binary::Value=read(&own(&old).study.join("selection.r3b"))?;
    let mut p=old.clone();
    p.source=source_digest()?;p.binary=file_hash(&std::env::current_exe()?)?;
    p.initial=end.checkpoint_hash.clone();p.initial_weights=l.model.weight_hash()?;
    p.config=signal_config(&old,state);
    let rows=(0..p.config.max_steps).map(|i|own(&old).rows[i%end.step]).collect::<Vec<_>>();
    p.identifiable=Some(Policy{study:output.into(),arm:"QE".into(),dataset:SIGNAL_DATA.into(),rows:rows.clone()});
    p.train_order=digest(&rows)?;p.evaluation=evaluation_for(&p);
    std::fs::create_dir(output)?;
    let selection=binary::record!({"contract":SIGNAL_CONTRACT,"source":p.source,"binary":p.binary,
        "historical_mode":"READ_ONLY_PARENT","parent":parent,"parent_policy":file_hash(&parent.join("plan.r3b"))?,
        "parent_source":old.source,"parent_binary":old.binary,"parent_endpoint":end,
        "parent_content":l.model.weights_content_id()?,"parent_adam":optimizer_hash(&l.optimizer)?,"parent_state":digest(state)?,
        "old_decision":if tiny {None}else{Some(file_hash(&own(&old).study.join("framing-decision.r3b"))?)},
        "seal_root":previous["seal_root"],"seal":previous["seal"],"sealed_corpus":previous["sealed_corpus"],
        "new_updates":p.config.max_steps-end.step,"first_new_tape_index":0,"max_input":2_100_000,"max_target":30_000,
        "observation":"fixed first4 train skeletons, local1/8/32/128; separate graphs; no CE change",
        "stop":"1024 full gate fixes candidate; otherwise registered2048 unless safety/divergence/budget"});
    write(&output.join("selection.r3b"),&selection)?;
    p.fork=Some(Fork{study:output.into(),study_hash:file_hash(&output.join("selection.r3b"))?,arm:"QE".into(),
        parent_policy:digest(&old)?,parent_state:digest(state)?,parent_adam:optimizer_hash(&l.optimizer)?,
        origin_step:state.step,origin_input:state.consumed_tokens,origin_target:state.target_tokens,
        original_corpus:old.corpus.clone(),tokenizer_training_hash:l.tokenizer.train_hash.clone(),
        variants:None,variant_metadata:None,alternate_first:vec![],selector:None,selector_metadata:None,flip_first:vec![],
        constant_lr:3e-4,target_limit:30_000});
    let root=output.join("QE");std::fs::create_dir(&root)?;
    for name in ["corpus.r3cor","transfer.r3cor","metadata.r3b","tokenizer.r3b"] {
        copy_native(&parent.join(name),&root.join(name))?;
    }
    copy_native(&cp,&root.join("initial.r3m"))?;
    write(&root.join("plan.r3b"),&p)?;
    signal_verify_plan(&root,&p)?;
    if !p.parent_entry(&root.join("initial.r3m"),&l)? {return Err(bad("signal parent readback"));}
    let c=verified_corpus(&root.join("corpus.r3cor"),&p.corpus)?;
    let ss=samples_with_framing(&c.train,&l.tokenizer,p.config.seq_len,p.framing())?;
    let input=rows[end.step..].iter().flatten().map(|&i|ss[i].tokens.len()-1).sum::<usize>();
    let target=rows[end.step..].len()*16;
    if !tiny && (input!=1_781_760 || target!=24_576) {return Err(bad("signal actual planned token budget"));}
    publish_confirmed(&output.join("preparation.r3b"),&binary::record!({"contract":SIGNAL_CONTRACT,
        "source":p.source,"binary":p.binary,"selection":p.fork.as_ref().unwrap().study_hash,
        "arms":{"QE":{"policy":file_hash(&root.join("plan.r3b"))?,"tape":p.train_order,"corpus":p.corpus,
        "metadata":p.metadata,"tokenizer":p.tokenizer,"initial":p.initial,"content":selection["parent_content"],
        "new_input":input,"new_target":target,"new_updates":rows.len()-end.step}},"optimizer":0,"generation":0,"teacher":0}))?;
    println!("SIGNAL_PREPARED parent={} new_updates={} input={input} target={target} REVIEW_PENDING",end.step,rows.len()-end.step);
    Ok(())
}
fn signal_verify_plan(root:&Path,p:&Plan)->Result<()> {
    let o=own(p);let f=p.fork.as_ref().ok_or_else(||bad("signal explicit fork required"))?;
    let s:binary::Value=read(&o.study.join("selection.r3b"))?;
    let parent=Path::new(s["parent"].as_str().ok_or_else(||bad("signal parent path"))?);
    let (old,end)=signal_parent(parent)?;
    let (m,_)=checkpoint::metadata(&parent.join(&end.checkpoint))?;
    let state=m.training.as_ref().ok_or_else(||bad("signal parent state"))?;
    let expected_rows=(0..p.config.max_steps).map(|i|own(&old).rows[i%end.step]).collect::<Vec<_>>();
    let mut expected=old.clone();
    expected.source=p.source.clone();expected.binary=p.binary.clone();expected.initial=end.checkpoint_hash.clone();
    expected.initial_weights=p.initial_weights.clone();expected.config=signal_config(&old,state);
    expected.identifiable=Some(Policy{study:o.study.clone(),arm:"QE".into(),dataset:SIGNAL_DATA.into(),rows:expected_rows.clone()});
    expected.train_order=digest(&expected_rows)?;expected.fork=Some(Fork{study:o.study.clone(),study_hash:file_hash(&o.study.join("selection.r3b"))?,arm:"QE".into(),
        parent_policy:digest(&old)?,parent_state:digest(state)?,parent_adam:s["parent_adam"].as_str().ok_or_else(||bad("parent Adam digest"))?.into(),
        origin_step:state.step,origin_input:state.consumed_tokens,origin_target:state.target_tokens,original_corpus:old.corpus.clone(),
        tokenizer_training_hash:ByteBpe::load(&parent.join("tokenizer.r3b"))?.train_hash,
        variants:None,variant_metadata:None,alternate_first:vec![],selector:None,selector_metadata:None,flip_first:vec![],constant_lr:3e-4,target_limit:30_000});
    expected.evaluation=evaluation_for(&expected);
    if *p!=expected || root!=o.study.join("QE") || s["contract"]!=SIGNAL_CONTRACT || s["source"]!=p.source
        || s["binary"]!=p.binary || s["parent_policy"]!=file_hash(&parent.join("plan.r3b"))?
        || s["parent_source"]!=old.source || s["parent_binary"]!=old.binary || s["parent_endpoint"]!=binary::record!(end)
        || s["parent_state"]!=f.parent_state || f.study_hash!=file_hash(&o.study.join("selection.r3b"))? {
        return Err(bad("signal frozen parent/policy/tape mismatch"));
    }
    if !p.tiny && s["old_decision"]!=file_hash(&own(&old).study.join("framing-decision.r3b"))? {return Err(bad("historical decision changed"));}
    for (name,h) in [("corpus.r3cor",&p.corpus),("transfer.r3cor",&p.transfer),("metadata.r3b",&p.metadata),("initial.r3m",&p.initial)] {
        if file_hash(&root.join(name))?!=*h {return Err(bad("signal owned bytes mismatch"));}
    }
    if file_hash(&root.join("tokenizer.r3b"))?!=file_hash(&parent.join("tokenizer.r3b"))? {return Err(bad("signal tokenizer bytes changed"));}
    Ok(())
}
fn signal_authorize(root:&Path,p:&Plan)->Result<()> {
    let study=&own(p).study;
    if root.canonicalize()?!=study.join(&own(p).arm).canonicalize()? {return Err(bad("signal registered root mismatch"));}
    let prep:binary::Value=read_confirmed(&study.join("preparation.r3b"))?;
    if prep["selection"]!=file_hash(&study.join("selection.r3b"))? {return Err(bad("reviewed signal selection changed"));}
    let h=history(root,p)?;
    for local in if p.tiny {vec![1]}else{vec![1,8,32,128]} {
        let step=p.origin_step()+local;
        signal_probe_next(root,p,step,h.last().map_or(p.origin_step(),|s|s.step))?;
    }
    if h.last().is_some_and(|s|s.phase.as_deref()==Some("Failed") || !["TRAINING","TIME_BUDGET","CANDIDATE_FIXED","BUDGET_REACHED","FINAL_QUALITY_FAIL"].contains(&s.stop.as_str())) {
        return Err(bad("signal failed endpoint remains closed"));
    }
    if !p.tiny {
        let r:binary::Value=read_confirmed(&study.join("legacy-finished.r3b"))?;
        if r["checkpoint"]!=p.initial || r["binding"]["source"]!=p.source || r["binding"]["binary"]!=p.binary
            || r["matched"]!=16 || r["completed"]!=16 || !r["error"].is_null()
            || r["control"]["generation_calls"]!=16 || r["control"]["teacher_calls"]!=0
            || r["control"]["terminal_reason"]!="COMPLETED" || r["raw"]!=file_hash(&study.join("legacy.r3rows"))? {
            return Err(bad("signal parent parity required"));
        }
    }
    Ok(())
}
pub(in super::super) fn signal_parent_parity(study:&Path)->Result<()> {
    let root=study.join("QE");let p=plan_read(&root)?;
    let review:binary::Value=read_confirmed(&study.join("review-a.r3b"))?;
    if !is_signal(&p) || p.tiny || review["verdict"]!="PASS" || review["source"]!=p.source
        || review["preparation"]!=file_hash(&study.join("preparation.r3b"))?
        || review["report_hash"]!=file_hash(Path::new(review["report_path"].as_str().ok_or_else(||bad("review path"))?))? {
        return Err(bad("signal independent A required"));
    }
    let selection:binary::Value=read(&study.join("selection.r3b"))?;
    let parent=Path::new(selection["parent"].as_str().ok_or_else(||bad("parent path"))?);
    let (old,end)=signal_parent(parent)?;
    let c=verified_corpus(&root.join("corpus.r3cor"),&p.corpus)?;
    let (_,dm,_)=verified_metadata(parent,&old)?;
    let tok=ByteBpe::load(&root.join("tokenizer.r3b"))?;
    audit_panel(parent,&old,end.step,"dev512",&c.validation,&dm,&tok)?;
    let raw=binary::read_value_records(&parent.join("eval-0512-dev512.r3rows"))?;
    orbit_observe(study,"legacy",&root.join("initial.r3m"),&c.validation[..16],
        &binary::record!({"policy":digest(&p)?,"parent":parent,"checkpoint":p.initial}),Some(&raw[1..17]),observation_control(&p,16,0)?)?;
    println!("SIGNAL_PARENT_PARITY matched16/16 optimizer0 teacher0");Ok(())
}
pub(in super::super) fn signal_endpoint(p:&Plan,step:usize,pending:bool)->Result<usize> {
    let origin=p.origin_step();
    if step<origin || step>p.config.max_steps {return Err(bad("signal cursor out of range"));}
    if pending && p.evaluation_due(step) {return Ok(step);}
    if p.tiny {return Ok(p.config.max_steps);}
    [1024,1536,2048].into_iter().find(|&s|s>step).ok_or_else(||bad("signal budget closed"))
}
fn signal_margins(m0:f64,m1:f64)->Result<[f64;4]> {
    if !m0.is_finite() || !m1.is_finite() {return Err(bad("nonfinite discrimination logits"));}
    Ok([(m0-m1)/2.,(m0+m1)/2.,m0,m1])
}
fn signal_stats(root:&Path,p:&Plan,step:usize,name:&str,count:usize)->Result<binary::Value> {
    let (_,es,ms)=panel_cases(root,p,step)?.into_iter().find(|(n,_,_)|n==name).ok_or_else(||bad("signal panel"))?;
    signal_stats_cases(root,p,step,name,&es,&ms,count)
}
#[allow(clippy::too_many_arguments)] // The same frozen panel scorer also reads the parent new64 observation.
fn signal_stats_cases(root:&Path,p:&Plan,step:usize,name:&str,es:&[Episode],ms:&[Meta],count:usize)->Result<binary::Value> {
    if count>es.len() || count==0 || count%4!=0 {return Err(bad("signal panel denominator"));}
    let tok=ByteBpe::load(&root.join("tokenizer.r3b"))?;
    audit_panel(root,p,step,name,es,ms,&tok)?;
    let teachers=binary::read_value_records(&root.join(format!("eval-{step:04}-{name}-teachers.r3rows")))?;
    let raw=binary::read_value_records(&root.join(format!("eval-{step:04}-{name}.r3rows")))?;
    let score=orbit_score(&es[..count],&ms[..count],&raw[1..count+1],&tok)?;
    let mut margins=vec![];let mut nll=[0.;3];
    for (i,e) in es[..count].iter().enumerate() {
        if resolve_request(&e.request)?!=e.answer || teachers[i+1]["id"]!=e.id {return Err(bad("signal orientation label"));}
        let t=&teachers[i+1]["teacher"];let cf=&t["conditional_foil"];
        let gold=tok.encode(e.answer.as_bytes())?[0];let foil=tok.encode(orbit_foil(e)?.as_bytes())?[0];
        let g=cf["gold_logit"].as_f64().ok_or_else(||bad("gold logit missing"))?;
        let f=cf["foil_logit"].as_f64().ok_or_else(||bad("foil logit missing"))?;
        let n:Vec<f64>=binary::from_value(t["target_token_observation"]["nll"].clone())?;
        let margin=g-f;let bn=(-margin).max(0.)+(-margin.abs()).exp().ln_1p();
        if n.len()!=2 || n.iter().any(|v|!v.is_finite()) || !margin.is_finite()
            || cf["gold"]!=gold || cf["foil"]!=foil || cf["margin"]!=binary::record!(margin)
            || cf["binary_nll"]!=binary::record!(bn) || t["target_token_observation"]["gold"]!=binary::record!([gold,EOS]) {
            return Err(bad("signal teacher binding/NLL"));
        }
        margins.push(margin);nll[0]+=n[0];nll[1]+=n[1];nll[2]+=bn;
    }
    let mut cs=vec![];let mut ds=vec![];let mut mins=vec![];let mut positive=0;
    for i in (0..count).step_by(2) {
        if ms[i].base!=ms[i+1].base || es[i].request.evidence!=es[i+1].request.evidence
            || es[i].answer!=orbit_foil(&es[i+1])? || es[i].request.input==es[i+1].request.input {
            return Err(bad("signal query/evidence orientation mismatch"));
        }
        let [c,d,m0,m1]=signal_margins(margins[i],margins[i+1])?;
        cs.push(c);ds.push(d);mins.push(m0.min(m1));positive+=usize::from(m0>0.&&m1>0.);
    }
    let dist=|v:&[f64]| {let mut s=v.to_vec();s.sort_by(f64::total_cmp);binary::record!({"min":s[0],"q25":s[(s.len()-1)/4],"median":s[(s.len()-1)/2],"q75":s[(s.len()-1)*3/4],"max":s[s.len()-1],"mean":s.iter().sum::<f64>()/s.len() as f64,"mean_abs":s.iter().map(|x|x.abs()).sum::<f64>()/s.len() as f64})};
    Ok(binary::record!({"step":step,"panel":name,"count":count,"cases":digest(&&es[..count])?,"score":score,
        "c":dist(&cs),"d":dist(&ds),"min_margin":dist(&mins),"positive_pairs":positive,
        "all4_positive":margins.chunks_exact(4).filter(|v|v.iter().all(|&m|m>0.)).count(),
        "value_nll":nll[0]/count as f64,"eos_nll":nll[1]/count as f64,"binary_nll":nll[2]/count as f64,
        "raw":file_hash(&root.join(format!("eval-{step:04}-{name}.r3rows")))?,
        "teacher":file_hash(&root.join(format!("eval-{step:04}-{name}-teachers.r3rows")))?}))
}
fn signal_evaluation_result(root:&Path,p:&Plan,step:usize)->Result<binary::Value> {
    let mut panels=BTreeMap::new();
    for (name,es,_) in panel_cases(root,p,step)? {
        panels.insert(name.clone(),signal_stats(root,p,step,&name,es.len())?);
    }
    let nt=if [1024,2048].contains(&step){512}else{64};
    let tr=signal_stats(root,p,step,&format!("train{nt}"),64)?;
    let dv=signal_stats(root,p,step,&format!("dev{nt}"),64)?;
    let selection:binary::Value=read(&own(p).study.join("selection.r3b"))?;
    let parent=Path::new(selection["parent"].as_str().ok_or_else(||bad("signal parent"))?);
    let (old,_)=signal_parent(parent)?;
    let baseline=signal_stats(parent,&old,512,"train512",64)?;
    let preceding=p.evaluation.train_steps.iter().copied().take_while(|&s|s<step).collect::<Vec<_>>();
    let mut prior=baseline.clone();let mut streak=0usize;
    for s in preceding.iter().copied().chain(std::iter::once(step)) {
        let n=if [1024,2048].contains(&s){512}else{64};
        let current=if s==step {tr.clone()}else{signal_stats(root,p,s,&format!("train{n}"),64)?};
        let no_gain=current["score"]["query_both"].as_u64()<=prior["score"]["query_both"].as_u64()
            && current["score"]["all4"].as_u64()<=prior["score"]["all4"].as_u64();
        let bad_nll=current["value_nll"].as_f64().unwrap()>=1.5*baseline["value_nll"].as_f64().unwrap();
        streak=if no_gain&&bad_nll {streak+1}else{0};prior=current;
    }
    let full=nt==512&&orbit_candidate_scores(&orbit_panel(root,p,step,"train512")?,&orbit_panel(root,p,step,"dev512")?);
    let stop=if full {Some("CANDIDATE_FIXED")}else if streak>=2 {Some("QUALITY_DIVERGENCE")}
        else if step==p.config.max_steps {Some("FINAL_QUALITY_FAIL")}else{None};
    Ok(binary::record!({"policy":digest(p)?,"step":step,"panels":panels,"fixed_train64":tr,"fixed_dev64":dv,
        "parent_fixed_train64":baseline,"divergence_streak":streak,"full_gate":full,"stop":stop}))
}
fn signal_evaluation_decision(root:&Path,p:&Plan,step:usize)->Result<Option<String>> {
    if p.tiny {return Ok(None);}
    let decision=signal_evaluation_result(root,p,step)?;
    let path=root.join(format!("signal-decision-{step:04}.r3b"));
    if path.exists() {let old:binary::Value=read_confirmed(&path)?;if old!=decision {return Err(bad("signal decision/raw mismatch"));}}
    else {publish_confirmed(&path,&decision)?;}
    println!("SIGNAL_DECISION {decision}");
    Ok(decision["stop"].as_str().map(str::to_owned))
}
pub(in super::super) fn signal_report(study:&Path)->Result<()> {
    let p=plan_read(&study.join("QE"))?;
    if !is_signal(&p) {return Err(bad("signal report scope"));}
    let h=history(&study.join("QE"),&p)?;
    let end=h.last().ok_or_else(||bad("signal endpoint missing"))?;
    audit(&study.join("QE"),&p,p.origin_step()+1,end.step)?;
    orbit_report_panels(&study.join("QE"),&p,end.step)?;
    if !p.tiny {
        for &step in p.evaluation.train_steps.iter().filter(|&&s|s<=end.step) {
            let actual=signal_evaluation_result(&study.join("QE"),&p,step)?;
            let recorded:binary::Value=read_confirmed(&study.join(format!("QE/signal-decision-{step:04}.r3b")))?;
            if actual!=recorded {return Err(bad("signal report decision/raw mismatch"));}
            println!("SIGNAL_DIAGNOSTIC {actual}");
        }
    }
    println!("SIGNAL_RESULT step={} new_updates={} durable={} stop={} resume={} usage={:?} Goal1=false",end.step,end.step-p.origin_step(),end.checkpoint_hash,end.stop,end.resume,usage(&p)?);
    Ok(())
}
fn signal_comparison(study:&Path)->Result<binary::Value> {
    let root=study.join("QE");let p=plan_read(&root)?;authorize(&root,&p)?;
    let h=history(&root,&p)?;let end=h.last().ok_or_else(||bad("signal final missing"))?;
    if p.tiny || end.resume || end.phase.as_deref()!=Some("Finished") || ![1024,2048].contains(&end.step)
        || !["CANDIDATE_FIXED","FINAL_QUALITY_FAIL"].contains(&end.stop.as_str()) {return Err(bad("signal final incomplete"));}
    let tr=orbit_panel(&root,&p,end.step,"train512")?;let dv=orbit_panel(&root,&p,end.step,"dev512")?;
    let eligible=end.stop=="CANDIDATE_FIXED"&&orbit_candidate_scores(&tr,&dv)&&parity_verified(&root,&p)?;
    Ok(binary::record!({"contract":SIGNAL_CONTRACT,"source":p.source,"preparation":file_hash(&study.join("preparation.r3b"))?,
        "endpoints":{"QE":{"policy":digest(&p)?,"checkpoint":end.checkpoint_hash,"step":end.step,"train":tr,"dev":dv}},
        "selected":if eligible {Some("QE")}else{None},"minimal_baseline_verified":false,"goal1_ready":false}))
}
pub(in super::super) fn signal_probe_cases(p:&Plan,root:&Path,step:usize,tok:&ByteBpe)->Result<Option<(Vec<Sample>,Vec<u32>,bool)>> {
    if !signal_probe_due(p,step) {return Ok(None);}
    let local=step.checked_sub(p.origin_step()).ok_or_else(||bad("signal probe cursor"))?;
    let c=verified_corpus(&root.join("corpus.r3cor"),&p.corpus)?;
    let (ms,_,_)=verified_metadata(root,p)?;
    for (i,e) in c.train[..16].iter().enumerate() {
        if resolve_request(&e.request)?!=e.answer || ms[i].base!=ms[i/4*4].base {return Err(bad("signal probe fixed skeleton/label"));}
    }
    let samples=samples_with_framing(&c.train[..16],tok,p.config.seq_len,p.framing())?;
    let foils=c.train[..16].iter().map(|e|Ok(tok.encode(orbit_foil(e)?.as_bytes())?[0])).collect::<Result<Vec<_>>>()?;
    Ok(Some((samples,foils,local!=128)))
}
pub(in super::super) fn signal_probe_due(p:&Plan,step:usize)->bool {
    #[cfg(feature="test-support")]
    if p.tiny && std::env::var("R3_SIGNAL_OBSERVER_OFF").as_deref()==Ok("1") {return false;}
    is_signal(p)&&step.checked_sub(p.origin_step()).is_some_and(|local|
        if p.tiny {local==1}else{[1,8,32,128].contains(&local)})
}
fn framing_stage(study: &Path, step: usize, latest: bool) -> Result<binary::Value> {
    framing_action(step, false, false)?;
    let mut endpoints = BTreeMap::new();
    let mut full = false;
    let mut signal = false;
    for arm in FRAME_ARMS {
        let root = study.join(arm);
        let p = plan_read(&root)?;
        authorize(&root, &p)?;
        let h = history(&root, &p)?;
        let s = if latest {
            h.last()
        } else {
            h.iter()
                .rev()
                .find(|s| s.step == step && s.phase.as_deref() != Some("EvaluationPending"))
        }
        .ok_or_else(|| bad("matched framing endpoint missing"))?;
        let tr = orbit_panel(&root, &p, step, "train512")?;
        let dv = orbit_panel(&root, &p, step, "dev512")?;
        let (f,j)=framing_endpoint_score(step,s.step,s.phase.as_deref()==Some("EvaluationPending"),orbit_peer_can_proceed(s),&tr,&dv)?;
        full|=f;signal|=j;
        endpoints.insert(arm,binary::record!({"checkpoint":s.checkpoint_hash,"step":step,"policy":digest(&p)?,"framing":p.framing(),"train":tr,"dev":dv}));
    }
    let (decision, extend) = framing_action(step, full, signal)?;
    Ok(binary::record!({"endpoints":endpoints,"full_gate":full,"joint_signal":signal,
        "extend":extend,"decision":decision,"source":source_digest()?}))
}
pub(in super::super) fn framing_decide(study: &Path) -> Result<()> {
    framing_legacy_reproduction(study)?;
    let decision = framing_stage(study, 512, true)?;
    publish_confirmed(&study.join("framing-decision.r3b"), &decision)?;
    println!("FRAMING_DECISION {}", decision["decision"]);
    Ok(())
}
fn framing_decision(study: &Path) -> Result<binary::Value> {
    let decision: binary::Value = read_confirmed(&study.join("framing-decision.r3b"))?;
    if decision != framing_stage(study, 512, false)? {
        return Err(bad("extension decision/raw endpoint binding"));
    }
    Ok(decision)
}
pub(in super::super) fn framing_run_endpoint(
    _root: &Path,
    p: &Plan,
    step: usize,
    pending: bool,
) -> Result<usize> {
    let midpoint = if p.tiny { 2 } else { 512 };
    if step < midpoint || (step == midpoint && pending) {
        return Ok(midpoint);
    }
    let decision = framing_decision(&own(p).study)?;
    if decision["extend"] != true {
        return Err(bad(
            "FRAMING_STUDY_CLOSED: no authorized additional updates",
        ));
    }
    Ok(p.config.max_steps)
}
fn framing_final_step(study: &Path) -> Result<usize> {
    let d = framing_decision(study)?;
    let step = if d["extend"] == true { 1024 } else { 512 };
    framing_stage(study, step, true)?;
    Ok(step)
}
pub(in super::super) fn framing_legacy_parity(study: &Path) -> Result<()> {
    let p = plan_read(&study.join("QE"))?;
    let review: binary::Value = read_confirmed(&study.join("review-a.r3b"))?;
    if review["verdict"] != "PASS"
        || review["preparation"] != file_hash(&study.join("preparation.r3b"))?
        || review["source"] != p.source
        || review["report_hash"]
            != file_hash(Path::new(
                review["report_path"]
                    .as_str()
                    .ok_or_else(|| bad("review report"))?,
            ))?
    {
        return Err(bad("actual framing review required"));
    }
    let s: binary::Value = read(&study.join("selection.r3b"))?;
    let parent = Path::new(s["parent"].as_str().ok_or_else(|| bad("parent"))?);
    let old = framing_parent(parent)?;
    let h = history(parent, &old)?;
    let end = h.last().ok_or_else(|| bad("legacy endpoint"))?;
    if end.step != 512 || end.resume {
        return Err(bad("closed BOTH512 required"));
    }
    let c = verified_corpus(&parent.join("corpus.r3cor"), &old.corpus)?;
    let (_, ms, _) = verified_metadata(parent, &old)?;
    let tok = ByteBpe::load(&parent.join("tokenizer.r3b"))?;
    audit_panel(parent, &old, 512, "dev512", &c.validation, &ms, &tok)?;
    let raw = binary::read_value_records(&parent.join("eval-0512-dev512.r3rows"))?;
    orbit_observe(
        study,
        "legacy",
        &parent.join(&end.checkpoint),
        &c.validation[..16],
        &binary::record!({"parent":parent,"checkpoint":end.checkpoint_hash,"framing":neural::PROMPT_FORMAT}),
        Some(&raw[1..17]),
        observation_control(&p, 16, 0)?,
    )?;
    println!("FRAMING_LEGACY_PARITY matched16/16 optimizer0 teacher0");
    Ok(())
}
pub(in super::super) fn framing_compare(study: &Path) -> Result<()> {
    println!("FRAMING_COMPARISON {}", framing_comparison(study)?);
    Ok(())
}
fn framing_legacy_reproduction(study: &Path) -> Result<()> {
    let selection: binary::Value = read(&study.join("selection.r3b"))?;
    let old_root = Path::new(
        selection["parent"]
            .as_str()
            .ok_or_else(|| bad("legacy parent"))?,
    );
    let qe = study.join("QE");
    let old = framing_parent(old_root)?;
    let p = plan_read(&qe)?;
    let old_history = history(old_root, &old)?;
    let new_history = history(&qe, &p)?;
    let old_end = old_history
        .iter()
        .rev()
        .find(|s| s.step == 512 && s.phase.as_deref() == Some("Finished"))
        .ok_or_else(|| bad("old512"))?;
    let new_end = new_history
        .iter()
        .rev()
        .find(|s| s.step == 512 && s.phase.as_deref() != Some("EvaluationPending"))
        .ok_or_else(|| bad("QE512"))?;
    let a = checkpoint::load(&old_root.join(&old_end.checkpoint), Device::Cpu, true)?;
    let b = checkpoint::load(&qe.join(&new_end.checkpoint), Device::Cpu, true)?;
    let sa = a
        .manifest
        .training
        .as_ref()
        .ok_or_else(|| bad("legacy Adam state"))?;
    let sb = b
        .manifest
        .training
        .as_ref()
        .ok_or_else(|| bad("QE Adam state"))?;
    if a.model.weights_content_id()? != b.model.weights_content_id()?
        || optimizer_hash(&a.optimizer)? != optimizer_hash(&b.optimizer)?
        || (
            sa.step,
            sa.sampler_state,
            sa.consumed_tokens,
            sa.target_tokens,
        ) != (
            sb.step,
            sb.sampler_state,
            sb.consumed_tokens,
            sb.target_tokens,
        )
    {
        return Err(bad(
            "QE_LEGACY_REPRODUCTION bitwise weights/Adam/cursor/token mismatch",
        ));
    }
    for name in ["train512", "dev512"] {
        let (_, es, ms) = panel_cases(old_root, &old, 512)?
            .into_iter()
            .find(|(n, _, _)| n == name)
            .unwrap();
        audit_panel(old_root, &old, 512, name, &es, &ms, &a.tokenizer)?;
        audit_panel(&qe, &p, 512, name, &es, &ms, &b.tokenizer)?;
        let ra = binary::read_value_records(&old_root.join(format!("eval-0512-{name}.r3rows")))?;
        let rb = binary::read_value_records(&qe.join(format!("eval-0512-{name}.r3rows")))?;
        for (i, (a, b)) in ra[1..].iter().zip(&rb[1..]).enumerate() {
            for key in [
                "raw_tokens",
                "actual",
                "error",
                "finish_reason",
                "generation_completed",
            ] {
                if a[key] != b[key] {
                    return Err(bad(&format!("QE_LEGACY_REPRODUCTION {name} row{i} {key}")));
                }
            }
        }
    }
    println!("QE_LEGACY_REPRODUCTION weights/Adam/cursor/tokens BITWISE_EQUAL raw1024/1024 calls0");
    Ok(())
}
fn framing_comparison(study: &Path) -> Result<binary::Value> {
    let step = framing_final_step(study)?;
    framing_legacy_reproduction(study)?;
    let mut result = framing_stage(study, step, true)?;
    let mut scores = vec![];
    let mut candidates = vec![];
    for arm in FRAME_ARMS {
        let root = study.join(arm);
        let p = plan_read(&root)?;
        let tr = orbit_panel(&root, &p, step, "train512")?;
        let dv = orbit_panel(&root, &p, step, "dev512")?;
        if !parity_verified(&root, &p)? {
            return Err(bad("framing final first16 parity required"));
        }
        if orbit_candidate_scores(&tr, &dv) {
            candidates.push((arm, dv.all4, dv.query_both, dv.full));
        }
        scores.push(dv);
    }
    let mut gain = 0;
    let mut loss = 0;
    let mut differences = vec![];
    for (a, b) in scores[0]
        .exact
        .as_chunks::<4>()
        .0
        .iter()
        .zip(scores[1].exact.as_chunks::<4>().0)
    {
        for (&a, &b) in a.iter().zip(b) {
            gain += usize::from(!a && b);
            loss += usize::from(a && !b);
        }
        differences.push(
            (b.iter().filter(|&&v| v).count() as f64 - a.iter().filter(|&&v| v).count() as f64)
                / 4.,
        );
    }
    let mean = differences.iter().sum::<f64>() / 128.;
    let se = (differences.iter().map(|d| (d - mean).powi(2)).sum::<f64>() / 127. / 128.).sqrt();
    candidates.sort_by(|a, b| {
        b.1.cmp(&a.1)
            .then(b.2.cmp(&a.2))
            .then(b.3.cmp(&a.3))
            .then((a.0 != "QE").cmp(&(b.0 != "QE")))
    });
    result["contract"] = binary::record!(FRAME_CONTRACT);
    result["step"] = binary::record!(step);
    result["preparation"] = binary::record!(file_hash(&study.join("preparation.r3b"))?);
    result["selected"] = binary::record!(candidates.first().map(|x| x.0));
    result["selection_rule"] =
        binary::record!("eligible only; dev ALL4 then QUERY_BOTH then FULL then QE");
    result["paired_dev"] = binary::record!({"gain":gain,"loss":loss,"orbit_mean_difference":mean,"standard_error":se,
        "normal_approx_95_interval":[mean-1.96*se,mean+1.96*se],"independent_units":128,"scope":"paired skeleton means; one seed"});
    result["confirmation"] = binary::record!(if candidates.is_empty() {
        "NOT_RUN_PREREQUISITE"
    } else {
        "PENDING_INDEPENDENT_B"
    });
    result["minimal_baseline_verified"] = binary::record!(false);
    result["goal1_ready"] = binary::record!(false);
    verify_framing_comparison(&result)?;
    Ok(result)
}
fn verify_framing_comparison(result:&binary::Value)->Result<()> {
    let step=result["step"].as_u64().ok_or_else(||bad("comparison step missing"))? as usize;
    let mut full=false;let mut signal=false;let mut candidates=vec![];
    for arm in FRAME_ARMS {
        let e=&result["endpoints"][arm];
        let actual=e["step"].as_u64().ok_or_else(||bad("comparison endpoint missing"))? as usize;
        let tr:OrbitScore=binary::from_value(e["train"].clone())?;
        let dv:OrbitScore=binary::from_value(e["dev"].clone())?;
        let (f,s)=framing_endpoint_score(step,actual,false,true,&tr,&dv)?;
        full|=f;signal|=s;
        if f {candidates.push((arm,dv.all4,dv.query_both,dv.full));}
    }
    candidates.sort_by(|a,b|b.1.cmp(&a.1).then(b.2.cmp(&a.2)).then(b.3.cmp(&a.3)).then((a.0!="QE").cmp(&(b.0!="QE"))));
    let (decision,extend)=framing_action(step,full,signal)?;
    if result["full_gate"]!=full || result["joint_signal"]!=signal || result["decision"]!=decision
        || result["extend"]!=extend || result["selected"]!=binary::record!(candidates.first().map(|x|x.0)) {
        return Err(bad("comparison action/candidate endpoint mismatch"));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    // Synthetic gold rows exercise the reader, not model quality. The separate
    // child invocations below use untouched random TINY weights and real greedy.
    #[test]
    fn value_citation_v1_confirmation_and_reviewer_process() -> Result<()> {
        const CHILD: &str = "R3_VALUE_CITATION_V1_CHILD";
        if let Ok(dir) = std::env::var(CHILD) {
            let dir = Path::new(&dir);
            let (p,end,comparison): (Plan,Segment,binary::Value) = read(&dir.join("reader-input.r3b"))?;
            let result = confirmation_current(dir,&p,&end,&comparison);
            assert_eq!(result["status"], "COMPLETED_PASS");
            consolidation_reproduction_endpoint(&end,&binary::record!({"step":5120,"eligible":false,
                "action":"FINAL_QUALITY_FAIL_AT_5120","stop":"FINAL_QUALITY_FAIL_AT_5120","extend":false,"regression":false}),true,false)?;
            let c: Vec<Episode> = read(&dir.join("native-cases.r3b"))?;
            let previous = binary::read_value_records(&dir.join("native-parent.r3rows"))?;
            let control = recovery::RunControl::new(std::sync::Arc::new(AtomicBool::new(false)),std::time::Duration::from_secs(60),16*1024*1024)?;
            let observed=orbit_observe(dir,"native-review",&dir.join(own(&p).arm.as_str()).join(&end.checkpoint),&c,
                &binary::record!({"policy":digest(&p)?}),Some(&previous[1..]),control);
            let parent:binary::Value=read_confirmed(&dir.join("native-parent-finished.r3b"))?;
            let current:binary::Value=read_confirmed(&dir.join("native-review-finished.r3b"))?;
            assert_eq!(parent["error"],current["error"]);
            assert_eq!(current["matched"],1);
            assert_eq!(current["control"]["generation_calls"],1);
            println!("V1_CHILD random_TINY generation1 teacher0 optimizer0 result={observed:?}");
            return Ok(());
        }
        let temp = if let Ok(path)=std::env::var("R3_VALUE_CITATION_V1_EVIDENCE") {
            tempfile::tempdir_in(path)?
        } else {tempfile::tempdir()?};
        let fixture_root=temp.path().to_path_buf();
        let original = orbit_fixture(&fixture_root)?;
        let mut p = plan_read(&original.join("BOTH"))?;
        let dir = temp.path().join("projection");std::fs::create_dir(&dir)?;
        p.identifiable.as_mut().unwrap().study = dir.clone();
        let root = dir.join("BOTH");std::fs::create_dir(&root)?;
        std::fs::copy(original.join("BOTH/tokenizer.r3b"),root.join("tokenizer.r3b"))?;
        let loaded = checkpoint::load(&original.join("BOTH/initial.r3m"),Device::Cpu,false)?;
        let model = replica_v3::neural::transformer::Transformer::init(p.architecture.clone(),71,Device::Cpu)?;
        neural::artifact::save(&root.join("random.r3m"),&model,&loaded.tokenizer,loaded.manifest.clone(),&loaded.optimizer)?;
        let mut end = Segment {schema:None,start_hash:None,control_hash:None,phase:Some("Finished".into()),
            policy:digest(&p)?,checkpoint:"random.r3m".into(),checkpoint_hash:file_hash(&root.join("random.r3m"))?,
            step:5120,input:0,target:0,elapsed:0.,generations:0,teachers:0,resume:false,stop:"FINAL_QUALITY_FAIL_AT_5120".into()};
        let fail = binary::record!({"step":5120,"eligible":false,"action":end.stop,"stop":end.stop,"extend":false,"regression":false});
        consolidation_reproduction_endpoint(&end,&fail,true,false)?;
        assert!(consolidation_reproduction_endpoint(&end,&fail,false,false).is_err());
        for reason in ["CANCELLED","UNKNOWN","QUALITY_REGRESSION","IO_ERROR"] {
            let mut stopped=end.clone();stopped.stop=reason.into();
            assert!(consolidation_reproduction_endpoint(&stopped,&fail,true,false).is_err());
        }
        for phase in ["EvaluationPending","Failed"] {
            let mut stopped=end.clone();stopped.phase=Some(phase.into());
            assert!(consolidation_reproduction_endpoint(&stopped,&fail,true,false).is_err());
        }
        end.step=4352;end.stop="CANDIDATE_FIXED_AT_4352".into();
        let good=binary::record!({"step":4352,"eligible":true,"action":end.stop,"stop":end.stop,"extend":false,"regression":false});
        consolidation_reproduction_endpoint(&end,&good,false,false)?;
        let mut mismatch=good.clone();mismatch["step"]=binary::record!(5120);
        assert!(consolidation_reproduction_endpoint(&end,&mismatch,true,false).is_err());
        end.step=5120;end.stop="FINAL_QUALITY_FAIL_AT_5120".into();
        let comparison=binary::record!({"selected":"BOTH","endpoints":{"fixture":"explicit synthetic reader spec; no candidate authority"}});
        assert_eq!(confirmation_current(&dir,&p,&end,&comparison)["status"],"NOT_OPENED");
        assert_eq!(confirmation_current(&dir,&p,&end,&binary::record!({"selected":null}))["status"],"NOT_ELIGIBLE");
        write(&dir.join("selection.r3b"),&binary::record!({"seal_root":original}))?;
        write(&dir.join("preparation.r3b"),&binary::record!({"fixture":true}))?;
        let report=dir.join("fixture-review.txt");std::fs::write(&report,b"synthetic reader fixture, not acceptance")?;
        write(&dir.join("review-b.r3b"),&binary::record!({"verdict":"PASS","preparation":file_hash(&dir.join("preparation.r3b"))?,
            "endpoints":comparison["endpoints"],"report_path":report,"report_hash":file_hash(&report)?}))?;
        let candidate=binary::record!({"arm":"BOTH","checkpoint":end.checkpoint_hash,"framing":p.framing(),"comparison":comparison,
            "seal":file_hash(&original.join("confirmation-seal.r3b"))?,"review":file_hash(&dir.join("review-b.r3b"))?});
        write(&dir.join("confirmation-candidate.r3b"),&candidate)?;
        assert_eq!(confirmation_current(&dir,&p,&end,&comparison)["status"],"PENDING/INCOMPLETE");
        let seal=verify_seal(&original)?;
        let corpus=verified_corpus(&original.join("sealed/confirmation.r3cor"),seal["corpus"].as_str().unwrap())?;
        let ms:Vec<Meta>=read(&original.join("sealed/metadata.r3b"))?;
        let tok=&loaded.tokenizer;
        let identity=binary::record!({"policy":digest(&p)?,"checkpoint":end.checkpoint_hash,"candidate":file_hash(&dir.join("confirmation-candidate.r3b"))?});
        let binding=binary::record!({"identity":identity,"checkpoint":end.checkpoint_hash,"cases":digest(&corpus.validation)?,
            "source":p.source,"binary":p.binary,"decoding":"normal-greedy-strict-utf8-eos"});
        write(&dir.join("confirmation-started.r3b"),&binding)?;
        assert_eq!(confirmation_current(&dir,&p,&end,&comparison)["status"],"PENDING/INCOMPLETE");
        let mut raw=vec![binding.clone()];
        for (i,e) in corpus.validation.iter().enumerate() {
            let tokens=tok.encode(e.answer.as_bytes())?;let mut ids=tokens.clone();ids.push(EOS);
            let prepared=prepare_call(&dir,"confirmation","generation",&binding,e,i)?;
            let row=binary::record!({"row_version":2,"id":e.id,"question":e.request.input,"generated_evidence":e.request.evidence,
                "expected":e.answer,"actual":e.answer,"raw_tokens":ids,"finish_reason":"stop","generation_completed":true,
                "generation":{"tokens":tokens,"generated":ids.len(),"finish":"stop"},"error":null,"exact_match":true,
                "native_prompt_digest":tok.prepare_with_framing(&e.request,p.framing(),p.architecture.context as u32,&p.architecture.id()?)?.token_digest,
                "framing":p.framing().id(),
                "attempt":prepared.file_name().unwrap().to_string_lossy()});
            write(&prepared.with_file_name(prepared.file_name().unwrap().to_string_lossy().replace("-prepared","-resolved")),
                &binary::record!({"prepared":file_hash(&prepared)?,"state":"RETURNED","row":digest(&row)?,"fixture":"synthetic strict-gold oracle; no model call"}))?;
            raw.push(row);
        }
        let raw_path=dir.join("confirmation.r3rows");
        let save_raw=|rows:&[binary::Value]|->Result<()> {let mut f=std::fs::File::create(&raw_path)?;for row in rows {append_row(&mut f,row)?;}Ok(())};
        save_raw(&raw)?;
        let finished=binary::record!({"binding":binding,"policy":digest(&p)?,"checkpoint":end.checkpoint_hash,"completed":256,"matched":0,
            "raw":file_hash(&raw_path)?,"error":null,"control":{"generation_calls":256,"teacher_calls":0,"elapsed_seconds":0.,"terminal_reason":"COMPLETED","observed_conditions":[]}});
        write(&dir.join("confirmation-finished.r3b"),&finished)?;
        let s=orbit_score(&corpus.validation,&ms,&raw[1..],tok)?;
        let result=binary::record!({"arm":"BOTH","checkpoint":end.checkpoint_hash,"score":s,"minimal_binding_baseline_verified":true,
            "scope":"K1-V finite digits; two records; new key/value sets","goal1_ready":false,"s4":false,"s5":false,"s6":false});
        write(&dir.join("confirmation-result.r3b"),&result)?;
        let snapshot=||inventory(&dir);
        let before=snapshot()?;
        assert_eq!(confirmation_current(&dir,&p,&end,&comparison)["status"],"COMPLETED_PASS");
        assert_eq!(before,snapshot()?);
        let mut failed_raw=raw.clone();
        let mut preserved_resolutions=vec![];
        for row in &mut failed_raw[1..17] {
            let text=if row["actual"]=="0" {"1"} else {"0"};
            let tokens=tok.encode(text.as_bytes())?;let mut ids=tokens.clone();ids.push(EOS);
            row["actual"]=binary::record!(text);row["raw_tokens"]=binary::record!(ids);
            row["generation"]=binary::record!({"tokens":tokens,"generated":ids.len(),"finish":"stop"});row["exact_match"]=binary::record!(false);
            let path=dir.join(row["attempt"].as_str().unwrap().replace("-prepared","-resolved"));
            preserved_resolutions.push((path.clone(),std::fs::read(&path)?));
            let mut r:binary::Value=read(&path)?;r["row"]=binary::record!(digest(row)?);
            std::fs::remove_file(&path)?;write(&path,&r)?;
        }
        save_raw(&failed_raw)?;
        let mut terminal=finished.clone();terminal["raw"]=binary::record!(file_hash(&raw_path)?);
        std::fs::remove_file(dir.join("confirmation-finished.r3b"))?;write(&dir.join("confirmation-finished.r3b"),&terminal)?;
        let mut failed_result=result.clone();failed_result["score"]=binary::record!(orbit_score(&corpus.validation,&ms,&failed_raw[1..],tok)?);
        failed_result["minimal_binding_baseline_verified"]=binary::record!(false);
        std::fs::remove_file(dir.join("confirmation-result.r3b"))?;write(&dir.join("confirmation-result.r3b"),&failed_result)?;
        assert_eq!(confirmation_current(&dir,&p,&end,&comparison)["status"],"COMPLETED_QUALITY_FAIL");
        for (path,bytes) in preserved_resolutions {std::fs::write(path,bytes)?;}
        save_raw(&raw)?;
        for (name,value) in [("confirmation-finished.r3b",&finished),("confirmation-result.r3b",&result)] {
            std::fs::remove_file(dir.join(name))?;write(&dir.join(name),value)?;
        }
        for (file,original_value,key,value) in [
            ("confirmation-result.r3b",result.clone(),"minimal_binding_baseline_verified",binary::record!(false)),
            ("confirmation-candidate.r3b",candidate.clone(),"checkpoint",binary::record!("wrong-model")),
            ("confirmation-candidate.r3b",candidate.clone(),"seal",binary::record!("wrong-seal")),
        ] {
            let path=dir.join(file);let bytes=std::fs::read(&path)?;let mut changed=original_value;changed[key]=value;
            std::fs::remove_file(&path)?;write(&path,&changed)?;
            assert_eq!(confirmation_current(&dir,&p,&end,&comparison)["status"],"INTEGRITY_FAIL");
            std::fs::write(path,bytes)?;
        }
        let mut duplicate=raw.clone();duplicate[2]=duplicate[1].clone();save_raw(&duplicate)?;
        assert_eq!(confirmation_current(&dir,&p,&end,&comparison)["status"],"INTEGRITY_FAIL");
        save_raw(&raw[..256])?;assert_eq!(confirmation_current(&dir,&p,&end,&comparison)["status"],"INTEGRITY_FAIL");save_raw(&raw)?;
        write(&pending_path(&dir.join("confirmation-result.r3b")),&result)?;
        assert_eq!(confirmation_current(&dir,&p,&end,&comparison)["status"],"UNKNOWN/FAILED_EXECUTION");
        std::fs::remove_file(pending_path(&dir.join("confirmation-result.r3b")))?;
        let cases=corpus.validation[..1].to_vec();write(&dir.join("native-cases.r3b"),&cases)?;
        let ctl=recovery::RunControl::new(std::sync::Arc::new(AtomicBool::new(false)),std::time::Duration::from_secs(60),16*1024*1024)?;
        let observed=orbit_observe(&dir,"native-parent",&root.join(&end.checkpoint),&cases,&binary::record!({"policy":digest(&p)?}),None,ctl);
        println!("V1_PARENT random_TINY generation1 result={observed:?}");
        write(&dir.join("reader-input.r3b"),&(p,end,comparison))?;
        let status=std::process::Command::new(std::env::current_exe()?).arg("value_citation_v1_confirmation_and_reviewer_process")
            .arg("--nocapture").env(CHILD,&dir).env("VECLIB_MAXIMUM_THREADS","1").env("OMP_NUM_THREADS","1").status()?;
        assert!(status.success());
        println!("V1 actual random TINY parent1+fresh_process1 generation2 teacher0 optimizer0; generated errors retained; synthetic5120 metadata is not training; strict-reader fault cases PASS");
        if std::env::var_os("R3_VALUE_CITATION_V1_EVIDENCE").is_some() {println!("V1_EVIDENCE {}",temp.keep().display());}
        Ok(())
    }
    #[test]
    fn consolidation_t1_actual_suffix_and_native_plan() -> Result<()> {
        let tmp = tempfile::tempdir()?;
        let study = orbit_fixture(tmp.path())?;
        let root = study.join("BOTH");
        let mut old = plan_read(&root)?;
        let c = verified_corpus(&root.join("corpus.r3cor"), &old.corpus)?;
        let (tm, dm, _) = verified_metadata(&root, &old)?;
        let tok = ByteBpe::load(&root.join("tokenizer.r3b"))?;
        let (es, _, _) = expansion_pool_from(&c, tm, &dm, &tok, &study)?;
        old.tiny = false;
        old.identifiable.as_mut().unwrap().rows = (0..2048)
            .map(|i| orbit_rows("BOTH", 512)[i % 512])
            .collect();
        old.identifiable.as_mut().unwrap().rows = expansion_rows(&old, "REBIND")?;
        let rows = consolidation_rows(&old)?;
        assert_eq!(rows.len(), 5120);
        assert_eq!(rows[..3584], own(&old).rows);
        assert_eq!(rows[3584..], own(&old).rows[2048..3584]);
        let path = tmp.path().join("actual-tape.r3b");
        publish_confirmed(&path, &rows)?;
        let back: Vec<[usize; 8]> = read_confirmed(&path)?;
        let native = tmp.path().join("actual-corpus.r3cor");
        data::native::write(&native, &orbit_native(es, c.validation)?, true)?;
        let corpus = data::native::read(&native)?;
        let exposure = expansion_exposure(&back[3584..], "REBIND", &corpus.train, &tok)?;
        assert_eq!(exposure["input"], 1781760);
        assert_eq!(exposure["target"], 24576);
        assert!(
            exposure["counts"]
                .as_array()
                .unwrap()
                .iter()
                .all(|n| *n == 8)
        );
        let mut wrong = back[3584..].to_vec();
        wrong[0].swap(0, 1);
        assert!(expansion_exposure(&wrong, "REBIND", &corpus.train, &tok).is_err());
        let mut changed = corpus.train.clone();
        changed[512].answer = "99".into();
        assert!(expansion_exposure(&back[3584..], "REBIND", &changed, &tok).is_err());
        old.identifiable.as_mut().unwrap().rows.pop();
        assert!(consolidation_rows(&old).is_err());
        println!(
            "CONSOLIDATION_T1 native tape writer/publisher/reader exact1536 suffix 12288 samples input1781760 target24576 optimizer0 generation0 teacher0"
        );
        Ok(())
    }
    #[test]
    fn consolidation_t3_gate_and_actions() -> Result<()> {
        let make = || {
            let mut n = endpoint_fixture(508, 252, 124);
            n.total = 1024;
            n.full = 1016;
            n.query_both = 504;
            n.all4 = 248;
            n.eos = 1024;
            n.exact = vec![false; 1024];
            vec![
                endpoint_fixture(508, 252, 124),
                n,
                endpoint_fixture(488, 232, 116),
            ]
        };
        assert!(consolidation_eligible(&make()));
        for panel in 0..3 {
            for field in 0..3 {
                for delta in [-1i32, 0, 1] {
                    let mut s = make();
                    let v = match field {
                        0 => &mut s[panel].full,
                        1 => &mut s[panel].query_both,
                        _ => &mut s[panel].all4,
                    };
                    *v = (*v as i32 + delta) as usize;
                    assert_eq!(consolidation_eligible(&s), delta >= 0);
                }
            }
        }
        for panel in 0..3 {
            let mut s = make();
            s[panel].errors = 1;
            assert!(!consolidation_eligible(&s));
            s = make();
            s[panel].eos -= 1;
            assert!(!consolidation_eligible(&s));
            s = make();
            s[panel].total -= 1;
            assert!(!consolidation_eligible(&s));
        }
        assert!(!consolidation_eligible(&make()[..2]));
        assert_eq!(
            consolidation_action(4352, true, false)?,
            ("CANDIDATE_FIXED_AT_4352", false)
        );
        assert_eq!(
            consolidation_action(4352, false, false)?,
            ("CONTINUE_WITHIN_REGISTERED_CAP", true)
        );
        assert_eq!(
            consolidation_action(5120, true, false)?,
            ("CANDIDATE_FIXED_AT_5120", false)
        );
        assert_eq!(
            consolidation_action(5120, false, false)?,
            ("FINAL_QUALITY_FAIL_AT_5120", false)
        );
        assert_eq!(
            consolidation_action(3840, true, false)?,
            ("CONTINUE_WITHIN_REGISTERED_CAP", true)
        );
        for step in [0, 3584, 3841, 4351, 4353, 4864, 5119, 5121] {
            assert!(consolidation_action(step, true, false).is_err());
        }
        for step in [3840, 4352, 5120] {
            assert_eq!(
                consolidation_action(step, true, true)?,
                ("QUALITY_REGRESSION", false)
            );
        }
        let parent = make();
        let mut now = make();
        now[0].errors = 15;
        assert!(!consolidation_regression(&now, &parent)?);
        now[1].errors = 1;
        assert!(consolidation_regression(&now, &parent)?);
        for total in [64, 512, 1024] {
            let m = || {
                (0..3)
                    .map(|_| OrbitScore {
                        total,
                        full: total,
                        query_both: total / 2,
                        swap_both: total / 2,
                        all4: total / 4,
                        eos: total,
                        errors: 0,
                        gold_foil_other_malformed: [total, 0, 0, 0],
                        same_across_queries: 0,
                        same_across_assignments: 0,
                        exact: vec![true; total],
                    })
                    .collect::<Vec<_>>()
            };
            let baseline = m();
            let mut now = m();
            for n in now.iter_mut().take(2) {
                n.full -= total.div_ceil(10);
                n.all4 -= (total / 4).div_ceil(10);
            }
            assert!(consolidation_regression(&now, &baseline)?);
            now[1].all4 += 1;
            assert!(!consolidation_regression(&now, &baseline)?);
        }
        println!(
            "CONSOLIDATION_T3 old/new/dev exact gate boundaries actions4352/5120 regression rational thresholds optimizer0 generation0 teacher0"
        );
        Ok(())
    }
    #[test]
    fn consolidation_t2_native_process_resume() -> Result<()> {
        const CHILD: &str = "R3_CONSOLIDATION_CHILD";
        if let Ok(root) = std::env::var(CHILD) {
            if std::env::var("R3_CONSOLIDATION_REVIEW").as_deref() == Ok("1") {
                let root=Path::new(&root);let p=plan_read(root)?;
                assert!(!gate(root,&p)?);
                assert!(orbit_parity(root,&p,false).is_err());
                orbit_parity(root,&p,true)?;
                let comparison=consolidation_comparison(&own(&p).study)?;
                assert!(comparison["selected"].is_null());
                assert!(orbit_confirm(&own(&p).study).is_err());
                assert!(run(root,false).is_err());
                return Ok(());
            }
            if std::env::var("R3_CONSOLIDATION_PARITY").as_deref() == Ok("1") {
                return consolidation_parent_parity(Path::new(&root));
            }
            return run(
                Path::new(&root),
                std::env::var("R3_CONSOLIDATION_CONTINUOUS").as_deref() == Ok("1"),
            );
        }
        let tmp = tempfile::tempdir()?;
        let base = std::env::var_os("R3_CONSOLIDATION_TEST_ROOT")
            .map(PathBuf::from)
            .unwrap_or_else(|| tmp.path().join("evidence"));
        std::fs::create_dir(&base)?;
        let base = base.canonicalize()?;
        let child = |root: &Path, continuous: bool, fault: bool, label: &str| -> Result<()> {
            let mut c = std::process::Command::new(std::env::current_exe()?);
            c.args(["--exact","training::fresh::identifiable::binding::tests::consolidation_t2_native_process_resume","--nocapture"])
                .env(CHILD,root).env("R3_CONSOLIDATION_CONTINUOUS",if continuous{"1"}else{"0"})
                .env("VECLIB_MAXIMUM_THREADS","1").env("OMP_NUM_THREADS","1");
            if label.ends_with("parity") {
                c.env("R3_CONSOLIDATION_PARITY", "1");
            }
            if label.ends_with("review") {c.env("R3_CONSOLIDATION_REVIEW","1");}
            if fault {
                c.env(
                    "R3_FRESH_CALL_STOP",
                    "eval-0004-old4/generation/1/fresh_panel_row_durable",
                );
            }
            let out = c.output()?;
            std::fs::write(base.join(format!("{label}.stdout")), &out.stdout)?;
            std::fs::write(base.join(format!("{label}.stderr")), &out.stderr)?;
            assert!(
                out.status.success(),
                "{label}: {} {}",
                String::from_utf8_lossy(&out.stdout),
                String::from_utf8_lossy(&out.stderr)
            );
            assert!(String::from_utf8_lossy(&out.stdout).contains("1 passed"));
            Ok(())
        };
        let retained_parent = std::env::var_os("R3_CONSOLIDATION_TEST_PARENT").map(PathBuf::from);
        let parent = if let Some(parent) = &retained_parent {
            parent.clone()
        } else {
            let study = orbit_fixture(&base)?;
            for arm in ["FIXED", "BOTH"] {
                child(&study.join(arm), true, false, &format!("parent-{arm}"))?;
            }
            study.join("BOTH")
        };
        let mut endpoints = vec![];
        let mut raws = vec![];
        let mut usage = if retained_parent.is_some() {
            [0usize; 3]
        } else {
            [4, 32, 32]
        };
        for name in ["continuous", "split", "evaluation-only"] {
            let out = base.join(name);
            consolidation_prepare(&parent, &out, true)?;
            fixture_review(&out)?;
            child(&out, false, false, &format!("{name}-parity"))?;
            usage[1] += 4;
            let root = out.join(CONTINUE_ARM);
            let p = plan_read(&root)?;
            for field in 0..6 {
                let mut wrong = p.clone();
                match field {
                    0 => wrong.fork.as_mut().unwrap().origin_step += 1,
                    1 => wrong.config.lr = 1e-4,
                    2 => wrong.config.max_steps += 1,
                    3 => wrong.corpus = "changed".into(),
                    4 => wrong.initial = "changed".into(),
                    _ => wrong.identifiable.as_mut().unwrap().rows[2].swap(0, 1),
                }
                assert!(consolidation_verify_plan(&root, &wrong).is_err());
            }
            child(
                &root,
                name != "split",
                name == "evaluation-only",
                &format!("{name}-0"),
            )?;
            let mut retained = None;
            if name == "evaluation-only" {
                let h = history(&root, &p)?;
                assert_eq!(h[0].step, 4);
                assert_eq!(h[0].phase.as_deref(), Some("EvaluationPending"));
                let path = root.join("eval-0004-old4.r3rows");
                let rs = binary::read_value_records(&path)?;
                assert_eq!(rs.len(), 3);
                retained = Some(rs);
            }
            if name != "continuous" {
                child(&root, true, false, &format!("{name}-1"))?;
            }
            let h = history(&root, &p)?;
            let end = h.last().unwrap();
            assert_eq!(end.step, 4);
            assert!(!end.resume);
            if let Some(prefix) = retained {
                let rs = binary::read_value_records(&root.join("eval-0004-old4.r3rows"))?;
                assert_eq!(rs[..3], prefix);
                let ctl: binary::Value = read(&root.join("segment-0001/train-control.r3b"))?;
                assert_eq!(ctl["optimizer_calls"], 0);
                assert_eq!(ctl["generation_calls"], 10);
            }
            let loaded = checkpoint::load(&root.join(&end.checkpoint), Device::Cpu, true)?;
            let state = loaded.manifest.training.as_ref().unwrap();
            endpoints.push((
                loaded.model.weights_content_id()?,
                optimizer_hash(&loaded.optimizer)?,
                state.step,
                state.sampler_state,
                state.consumed_tokens,
                state.target_tokens,
            ));
            let mut output = vec![];
            for panel in ["old4", "new4", "dev4"] {
                let rs =
                    binary::read_value_records(&root.join(format!("eval-0004-{panel}.r3rows")))?;
                output.extend(rs[1..].iter().map(|r| {
                    binary::record!([
                        r["raw_tokens"],
                        r["actual"],
                        r["error"],
                        r["finish_reason"],
                        r["generation_completed"]
                    ])
                }));
            }
            raws.push(output);
            consolidation_close(&root)?;
            if name == "continuous" {
                child(&root,false,false,"normal-quality-failure-review")?;
                usage[1]+=4;
                let same = root.join("same-content-distinct-file.r3m");
                let mut manifest = loaded.manifest.clone();
                manifest.source_id = neural::hash(b"TINY physical copy provenance");
                neural::artifact::save(
                    &same,
                    &loaded.model,
                    &loaded.tokenizer,
                    manifest.clone(),
                    &loaded.optimizer,
                )?;
                assert_ne!(file_hash(&same)?, end.checkpoint_hash);
                let (panel, es, ms) = panel_cases(&root, &p, 4)?.remove(0);
                let mut ctl = recovery::RunControl::new(
                    std::sync::Arc::new(AtomicBool::new(false)),
                    std::time::Duration::from_secs(30),
                    16 * 1024 * 1024,
                )?;
                ctl.set_call_limits(0, 0);
                evaluate_panel(&p, &root, &same, 4, &panel, &es, &ms, &mut ctl)?;
                let different = root.join("different-model.r3m");
                let model = replica_v3::neural::transformer::Transformer::init(
                    p.architecture.clone(),
                    18,
                    Device::Cpu,
                )?;
                neural::artifact::save(
                    &different,
                    &model,
                    &loaded.tokenizer,
                    manifest,
                    &loaded.optimizer,
                )?;
                assert!(
                    evaluate_panel(&p, &root, &different, 4, &panel, &es, &ms, &mut ctl).is_err()
                );
                assert_eq!(ctl.receipt()["generation_calls"], 0);
                assert_eq!(ctl.receipt()["teacher_calls"], 0);
            }
            assert!(consolidation_endpoint(&p, 4, false).is_err());
            assert_eq!(consolidation_endpoint(&p, 4, true)?, 4);
            assert!(consolidation_evaluation_result(&root, &p, 4352).is_err());
            // A saved summary without its complete raw cannot close.
            let raw = root.join("eval-0004-new4.r3rows");
            let held = root.join("held-new4.r3rows");
            std::fs::rename(&raw, &held)?;
            assert!(consolidation_close(&root).is_err());
            std::fs::rename(&held, &raw)?;
            assert!(run(&root, false).is_err());
            for (i, s) in h.iter().enumerate() {
                let control: binary::Value =
                    read(&root.join(format!("segment-{i:04}/train-control.r3b")))?;
                usage[0] += control["optimizer_calls"].as_u64().unwrap() as usize;
                usage[1] += s.generations;
                usage[2] += s.teachers;
            }
        }
        assert!(endpoints.windows(2).all(|v| v[0] == v[1]));
        assert!(raws.windows(2).all(|v| v[0] == v[1]));
        assert_eq!(
            usage,
            if retained_parent.is_some() {
                [6, 52, 36]
            } else {
                [10, 84, 68]
            }
        );
        println!(
            "CONSOLIDATION_T2 actual TINY native EOS tensor fixture (not unmodified random weights), real forward/Adam: 2 vs 1+1 vs 2+0 weights/Adam/clock/cursor/input/target/full_raw_EQUAL retained_returned_rows2 evaluation_resume_optimizer0; optimizer{} generation{} teacher{} evidence={}",
            usage[0],
            usage[1],
            usage[2],
            base.display()
        );
        Ok(())
    }
    fn endpoint_fixture(full:usize,qb:usize,all4:usize)->OrbitScore {
        OrbitScore{total:512,full,query_both:qb,swap_both:0,all4,eos:512,errors:0,
            gold_foil_other_malformed:[full,512-full,0,0],same_across_queries:0,same_across_assignments:0,exact:vec![false;512]}
    }
    #[test]
    fn query_signal_comparison_and_scalar_boundaries()->Result<()> {
        let tr=endpoint_fixture(508,252,124);let dv=endpoint_fixture(488,232,116);
        for requested in [512,1024] {
            let encoded=binary::to_vec(&(requested,&tr,&dv))?;
            let (actual,t,d):(usize,OrbitScore,OrbitScore)=binary::from_slice(&encoded)?;
            let (full,joint)=framing_endpoint_score(requested,actual,false,true,&t,&d)?;
            assert!(full&&joint);assert_eq!(framing_action(requested,full,joint)?.1,false);
            assert!(framing_endpoint_score(requested,actual,true,true,&t,&d).is_err());
            assert!(framing_endpoint_score(requested,actual,false,false,&t,&d).is_err());
            assert!(framing_endpoint_score(requested,1536-actual,false,true,&t,&d).is_err());
            let mut missing=endpoint_fixture(508,252,124);missing.total=511;
            assert!(framing_endpoint_score(requested,actual,false,true,&missing,&d).is_err());
        }
        for (index,threshold) in [(0,508),(1,252),(2,124),(3,488),(4,232),(5,116)] {
            for delta in [-1i32,0,1] {
                let mut values=[508,252,124,488,232,116];values[index]=(threshold as i32+delta) as usize;
                let t=endpoint_fixture(values[0],values[1],values[2]);let d=endpoint_fixture(values[3],values[4],values[5]);
                assert_eq!(orbit_candidate_scores(&t,&d),delta>=0);
            }
        }
        for delta in [-1i32,0,1] {
            let t=endpoint_fixture(256,(64+delta) as usize,16);let d=endpoint_fixture(256,0,0);
            assert_eq!(framing_endpoint_score(512,512,false,true,&t,&d)?.1,delta>=0);
            assert_eq!(framing_action(1024,false,delta>=0)?,("FINAL_QUALITY_FAIL_AT_1024",false));
        }
        let sp=|v:f64|v.max(0.)+(-v.abs()).exp().ln_1p();
        let loss=|c:f64,d:f64|(sp(-c-d)+sp(c-d))/2.;
        let h=1e-5;let mut calculations=0;
        for c in [-4.,-1.,0.,3.] {
            let derivative=(loss(c,h)-loss(c,-h))/(2.*h);calculations+=2;
            assert!((derivative+0.5).abs()<1e-9);
        }
        let a=signal_margins(2.,-1.)?;assert_eq!(a,[1.5,0.5,2.,-1.]);
        assert_eq!(signal_margins(-2.,1.)?,a.map(|v|-v));
        let z=[3.,1.,2.,4.];let shifted=z.map(|v|v+17.);
        assert_eq!(signal_margins(z[0]-z[1],z[3]-z[2])?,signal_margins(shifted[0]-shifted[1],shifted[3]-shifted[2])?);
        let ms=(0..16).map(|i|i as f64-5.).collect::<Vec<_>>();
        let global=ms.iter().sum::<f64>()/16.;
        assert_eq!(global,ms.chunks_exact(8).map(|v|v.iter().sum::<f64>()/16.).sum::<f64>());
        assert_eq!(global,ms.chunks_exact(2).map(|m|signal_margins(m[0],m[1]).unwrap()[1]).sum::<f64>()/8.);
        assert!(signal_margins(f64::NAN,0.).is_err());
        println!("QUERY_SIGNAL_SCALAR finite_difference_evaluations={calculations} model_calls=0 synthetic_endpoint_steps=512,1024 optimizer=0");
        Ok(())
    }
    #[test]
    fn query_signal_comparison_published_endpoint_and_row_budget()->Result<()> {
        let temp=tempfile::tempdir()?;
        let tr=endpoint_fixture(508,252,124);let dv=endpoint_fixture(488,232,116);
        let endpoint=binary::record!({"step":1024,"train":tr,"dev":dv});
        let result=binary::record!({"step":1024,"endpoints":{"QE":endpoint,"EQ":endpoint},
            "full_gate":true,"joint_signal":true,"decision":"CANDIDATE_AT_1024","extend":false,"selected":"QE"});
        let path=temp.path().join("comparison.r3b");publish_confirmed(&path,&result)?;
        let readback:binary::Value=read_confirmed(&path)?;verify_framing_comparison(&readback)?;
        let before=file_hash(&path)?;
        for (field,value) in [("step",binary::record!(512)),("extend",binary::record!(true)),
            ("decision",binary::record!("CANDIDATE_AT_512")),("selected",binary::record!("EQ"))] {
            let mut invalid=readback.clone();invalid[field]=value;assert!(verify_framing_comparison(&invalid).is_err());
        }
        for value in [binary::Value::Null,binary::record!({"step":512,"train":tr,"dev":dv})] {
            let mut invalid=readback.clone();invalid["endpoints"]["EQ"]=value;assert!(verify_framing_comparison(&invalid).is_err());
        }
        assert_eq!(before,file_hash(&path)?);
        let mut control=recovery::RunControl::new(std::sync::Arc::new(AtomicBool::new(false)),std::time::Duration::from_secs(30),16*1024*1024)?;
        control.set_call_limits(0,16);control.begin_teacher_rows(8)?;control.begin_teacher_rows(8)?;
        assert_eq!(control.teacher_calls,16);assert!(control.begin_teacher_rows(1).is_err());assert_eq!(control.teacher_calls,16);
        println!("QUERY_SIGNAL_COMPARISON writer/publisher/reader1024 endpoint/candidate/missing/mixed rejection; reserved_teacher_rows16 simulated_model_calls0");
        Ok(())
    }
    #[test]
    fn query_signal_no_call_time_pause_process() -> Result<()> {
        const NAME: &str = "training::fresh::identifiable::binding::tests::query_signal_no_call_time_pause_process";
        if let Ok(root) = std::env::var("R3_SIGNAL_PAUSE_CHILD") {
            return run(Path::new(&root), true);
        }
        let base = PathBuf::from(
            std::env::var_os("R3_SIGNAL_PAUSE_ROOT")
                .ok_or_else(|| bad("explicit retained evidence root required"))?,
        );
        let parent = PathBuf::from(
            std::env::var_os("R3_SIGNAL_TEST_PARENT")
                .ok_or_else(|| bad("explicit retained TINY parent required"))?,
        );
        std::fs::create_dir(&base)?;
        let child = |root: &Path, stop: bool, label: &str| -> Result<()> {
            let mut command = std::process::Command::new(std::env::current_exe()?);
            command
                .args(["--exact", NAME, "--nocapture"])
                .env("R3_SIGNAL_PAUSE_CHILD", root)
                .env("VECLIB_MAXIMUM_THREADS", "1")
                .env("OMP_NUM_THREADS", "1");
            if stop {
                command.env("R3_SIGNAL_CALL_STOP", "before_teacher_budget");
            }
            let out = command.output()?;
            std::fs::write(base.join(format!("{label}.stdout")), &out.stdout)?;
            std::fs::write(base.join(format!("{label}.stderr")), &out.stderr)?;
            assert!(
                out.status.success(),
                "{label}: {} {}",
                String::from_utf8_lossy(&out.stdout),
                String::from_utf8_lossy(&out.stderr)
            );
            Ok(())
        };
        let mut endpoints = vec![];
        let mut raws = vec![];
        let mut usage = [0u64; 3];
        for name in ["paused", "continuous"] {
            let study = base.join(name);
            signal_prepare(&parent, &study, true)?;
            fixture_review(&study)?;
            let root = study.join("QE");
            let p = plan_read(&root)?;
            if name == "paused" {
                child(&root, true, "pause")?;
                let h = history(&root, &p)?;
                assert_eq!(h.len(), 1);
                let s = &h[0];
                assert_eq!(s.step, p.origin_step());
                assert!(s.resume);
                assert_eq!(s.stop, "TIME_BUDGET");
                let c: binary::Value = read(&root.join("segment-0000/train-control.r3b"))?;
                for key in [
                    "optimizer_calls",
                    "teacher_calls",
                    "generation_calls",
                    "diagnostic_microbatch_forwards",
                    "diagnostic_backwards",
                ] {
                    assert_eq!(c[key], 0, "{key}");
                }
                assert_eq!(c["observed_conditions"], binary::record!(["TIME_BUDGET"]));
                let started = root.join(format!(
                    "signal-probe-{:04}-started.r3b",
                    p.origin_step() + 1
                ));
                let finished = root.join(format!(
                    "signal-probe-{:04}-finished.r3b",
                    p.origin_step() + 1
                ));
                let hashes = (file_hash(&started)?, file_hash(&finished)?);
                child(&root, false, "resume")?;
                assert_eq!(hashes, (file_hash(&started)?, file_hash(&finished)?));
            } else {
                child(&root, false, "continuous")?;
            }
            let h = history(&root, &p)?;
            let end = h.last().unwrap();
            assert_eq!(end.step, p.config.max_steps);
            assert!(!end.resume);
            let l = checkpoint::load(&root.join(&end.checkpoint), Device::Cpu, true)?;
            let s = l.manifest.training.as_ref().unwrap();
            endpoints.push((
                l.model.weights_content_id()?,
                optimizer_hash(&l.optimizer)?,
                s.step,
                s.sampler_state,
                s.consumed_tokens,
                s.target_tokens,
            ));
            for panel in ["train4", "dev4"] {
                let rows = binary::read_value_records(
                    &root.join(format!("eval-{:04}-{panel}.r3rows", end.step)),
                )?;
                raws.push(
                    rows[1..]
                        .iter()
                        .map(|r| r["raw_tokens"].clone())
                        .collect::<Vec<_>>(),
                );
            }
            for (i, s) in h.iter().enumerate() {
                let c: binary::Value =
                    read(&root.join(format!("segment-{i:04}/train-control.r3b")))?;
                usage[0] += c["optimizer_calls"].as_u64().unwrap();
                usage[1] += s.generations as u64;
                usage[2] += s.teachers as u64;
            }
        }
        assert_eq!(endpoints[0], endpoints[1]);
        assert_eq!(raws[..2], raws[2..]);
        assert_eq!(usage, [4, 16, 80]);
        println!(
            "QS_R1_PROCESS no_call_time_pause_new_attempt weights_Adam_clock_sampler_tokens_train_dev_raw_EQUAL actual_optimizer={} generation={} teacher_rows={} evidence={}",
            usage[0],
            usage[1],
            usage[2],
            base.display()
        );
        Ok(())
    }

    #[test]
    fn query_signal_tiny_observer_and_process_resume()->Result<()> {
        const CHILD:&str="R3_SIGNAL_TEST_CHILD";
        if let Ok(root)=std::env::var(CHILD) {
            return run(Path::new(&root),std::env::var("R3_SIGNAL_CONTINUOUS").as_deref()==Ok("1"));
        }
        let temp=tempfile::tempdir()?;
        let base=std::env::var_os("R3_SIGNAL_TEST_ROOT").map(PathBuf::from).unwrap_or_else(||temp.path().join("evidence"));
        std::fs::create_dir(&base)?;
        let retained_parent=std::env::var_os("R3_SIGNAL_TEST_PARENT").map(PathBuf::from);
        let parent_root=if let Some(parent)=&retained_parent {parent.clone()}else{
            let old=orbit_fixture(&base)?;let frames=base.join("frames");
            framing_prepare(&old.join("BOTH"),&frames,true)?;fixture_review(&frames)?;frames.join("QE")
        };
        let child=|root:&Path,continuous:bool,off:bool,stop:bool,label:&str|->Result<()> {
            let mut cmd=std::process::Command::new(std::env::current_exe()?);
            cmd.args(["--exact","training::fresh::identifiable::binding::tests::query_signal_tiny_observer_and_process_resume","--nocapture"])
                .env(CHILD,root).env("R3_SIGNAL_CONTINUOUS",if continuous{"1"}else{"0"})
                .env("R3_SIGNAL_OBSERVER_OFF",if off{"1"}else{"0"}).env("VECLIB_MAXIMUM_THREADS","1").env("OMP_NUM_THREADS","1");
            if stop {cmd.env("R3_FRESH_TRAIN_STOP","training_checkpoint_saved");}
            let out=cmd.output()?;
            std::fs::write(base.join(format!("{label}.stdout")),&out.stdout)?;
            std::fs::write(base.join(format!("{label}.stderr")),&out.stderr)?;
            assert!(out.status.success(),"{label}: {} {}",String::from_utf8_lossy(&out.stdout),String::from_utf8_lossy(&out.stderr));
            assert!(String::from_utf8_lossy(&out.stdout).contains("1 passed"));
            Ok(())
        };
        if retained_parent.is_none() {child(&parent_root,true,true,false,"parent2")?;}
        let (oldp,_)=signal_parent(&parent_root)?;
        let oldh=history(&parent_root,&oldp)?;
        let mut generations=if retained_parent.is_none(){oldh.iter().map(|s|s.generations).sum::<usize>()}else{0};
        let mut teacher_rows=if retained_parent.is_none(){oldh.iter().map(|s|s.teachers).sum::<usize>()}else{0};
        let mut updates=if retained_parent.is_none(){2}else{0};let mut endpoints=vec![];let mut raws=vec![];
        // A retained test parent avoids repeating already-counted fixture work.
        for name in ["off","segmented","evaluation-only"] {
            let study=base.join(name);signal_prepare(&parent_root,&study,true)?;fixture_review(&study)?;
            let root=study.join("QE");let p=plan_read(&root)?;
            assert_eq!(own(&p).rows[..2],own(&p).rows[2..4]);assert_eq!(p.learning_rate(3),3e-4);
            let mut wrong=p.clone();wrong.fork.as_mut().unwrap().origin_step+=1;assert!(signal_verify_plan(&root,&wrong).is_err());
            wrong=p.clone();wrong.framing=Some(neural::Framing::EvidenceQuestion);assert!(signal_verify_plan(&root,&wrong).is_err());
            let continuous=name!="segmented";let off=name!="segmented";
            child(&root,continuous,off,name=="evaluation-only",&format!("{name}-0"))?;
            if name=="segmented"||name=="evaluation-only" {
                let h=history(&root,&p)?;
                if name=="evaluation-only" {assert_eq!(h.last().unwrap().phase.as_deref(),Some("EvaluationPending"));}
                child(&root,continuous,off,false,&format!("{name}-1"))?;
            }
            let h=history(&root,&p)?;let end=h.last().unwrap();assert_eq!(end.step,4);assert!(!end.resume);
            let l=checkpoint::load(&root.join(&end.checkpoint),Device::Cpu,true)?;
            let s=l.manifest.training.as_ref().unwrap();
            endpoints.push((l.model.weights_content_id()?,optimizer_hash(&l.optimizer)?,s.step,s.sampler_state,s.consumed_tokens,s.target_tokens));
            let raw=binary::read_value_records(&root.join("eval-0004-dev4.r3rows"))?;
            raws.push(raw[1..].iter().map(|r|r["raw_tokens"].clone()).collect::<Vec<_>>());
            for (i,seg) in h.iter().enumerate() {
                let control:binary::Value=read(&root.join(format!("segment-{i:04}/train-control.r3b")))?;
                updates+=control["optimizer_calls"].as_u64().unwrap() as usize;
                teacher_rows+=seg.teachers;generations+=seg.generations;
                if name=="evaluation-only"&&i==1 {assert_eq!(control["optimizer_calls"],0);}
            }
            if !off {
                for step in [3] {
                    let r:binary::Value=read_confirmed(&root.join(format!("signal-probe-{step:04}-finished.r3b")))?;
                    assert_eq!(r["diagnostic_backwards"],2);assert_eq!(r["sample_forwards"],32);assert!(r["error"].is_null());
                }
            }
        }
        assert!(endpoints.windows(2).all(|v|v[0]==v[1]));assert!(raws.windows(2).all(|v|v[0]==v[1]));
        assert!(updates<=64&&generations<=256&&teacher_rows<=256);
        println!("QUERY_SIGNAL_TINY actual observeroff2/observed_newprocess1+1/evalonly2+0 BITWISE_WEIGHTS_ADAM_CLOCK_TOKENS_RAW_EQUAL optimizer={updates} generation={generations} teacher_sample_rows={teacher_rows} retained_parent={} evidence={}",retained_parent.is_some(),base.display());
        Ok(())
    }
    #[test]
    fn framing_final_action_is_endpoint_bound() -> Result<()> {
        for signal in [false, true] {
            assert_eq!(framing_action(512, true, signal)?, ("CANDIDATE_AT_512", false));
            assert_eq!(framing_action(1024, true, signal)?, ("CANDIDATE_AT_1024", false));
            assert_eq!(framing_action(1024, false, signal)?, ("FINAL_QUALITY_FAIL_AT_1024", false));
        }
        assert_eq!(framing_action(512, false, false)?, ("CLOSE_NO_JOINT_SIGNAL", false));
        assert_eq!(framing_action(512, false, true)?, ("EXTEND_BOTH_TO_1024", true));
        for step in [0, 511, 513, 1023, 1025] { assert!(framing_action(step, true, true).is_err()); }
        Ok(())
    }
    use super::*;
    #[test]
    fn framing_t1_t2_t3_exact_blocks_and_policy() -> Result<()> {
        let temp = tempfile::tempdir()?;
        let old = orbit_fixture(temp.path())?;
        let study = temp.path().join("frames");
        framing_prepare(&old.join("BOTH"), &study, true)?;
        let root = study.join("EQ");
        let p = plan_read(&root)?;
        let tok = ByteBpe::load(&root.join("tokenizer.r3b"))?;
        let c = verified_corpus(&root.join("corpus.r3cor"), &p.corpus)?;
        let mut req = c.train[0].request.clone();
        req.system = "S".into();
        req.input = "Q".into();
        let system = [
            vec![BOS, neural::SYSTEM_ROLE],
            tok.encode(b"S")?,
            vec![neural::END_ROLE],
        ]
        .concat();
        let question = [
            vec![neural::USER_ROLE],
            tok.encode(b"Q")?,
            vec![neural::END_ROLE],
        ]
        .concat();
        let mut evidence = vec![];
        for e in &req.evidence.items {
            evidence.push(neural::EVIDENCE_ROLE);
            evidence.extend(tok.encode(neural::evidence_text(e).as_bytes())?);
            evidence.push(neural::END_ROLE);
        }
        let qe = tok.prepare(&req, 2048, "fixture")?;
        let eq =
            tok.prepare_with_framing(&req, neural::Framing::EvidenceQuestion, 2048, "fixture")?;
        assert_eq!(
            qe.token_ids,
            [
                system.clone(),
                question.clone(),
                evidence.clone(),
                vec![neural::ASSISTANT_ROLE]
            ]
            .concat()
        );
        assert_eq!(
            eq.token_ids,
            [system, evidence, question, vec![neural::ASSISTANT_ROLE]].concat()
        );
        assert_eq!(qe.provided, eq.provided);
        assert_eq!(qe.provided.len(), 2);
        assert!(eq.excluded.is_empty());
        assert_ne!(qe.token_digest, eq.token_digest);
        assert_ne!(qe.config_id, eq.config_id);
        assert!(!eq.token_ids.contains(&EOS));
        framing_equivalence(
            &[c.train.clone(), c.validation].concat(),
            &tok,
            p.config.seq_len,
        )?;
        let qe_samples = samples(&c.train, &tok, p.config.seq_len)?;
        let eq_samples = samples_with_framing(&c.train, &tok, p.config.seq_len, p.framing())?;
        let a = batch(&qe_samples, &[0, 1], &Device::Cpu)?;
        let b = batch(&eq_samples, &[0, 1], &Device::Cpu)?;
        assert_eq!(a.mask.to_vec2::<f32>()?, b.mask.to_vec2::<f32>()?);
        assert_eq!(
            a.first_target_mask.to_vec2::<f32>()?,
            b.first_target_mask.to_vec2::<f32>()?
        );
        for (x, y) in qe_samples.iter().zip(&eq_samples) {
            assert_eq!(x.tokens[x.response_start..], y.tokens[y.response_start..]);
        }
        assert!(neural::Framing::from_digest([0; 32]).is_err());
        assert_eq!(
            neural::Framing::from_digest(p.framing().digest())?,
            p.framing()
        );
        let mut wrong = p.clone();
        wrong.framing = Some(neural::Framing::QuestionEvidence);
        assert!(framing_verify_plan(&root, &wrong).is_err());
        let rows = framing_rows(false);
        assert_eq!(rows.len(), 1024);
        assert_eq!(rows[..512], rows[512..]);
        assert_eq!(rows[..512], orbit_rows("BOTH", 512));
        assert_eq!(framing_run_endpoint(&root, &p, 0, false)?, 2);
        let mut large = p.clone();
        large.tiny = false;
        assert_eq!(framing_run_endpoint(&root, &large, 511, false)?, 512);
        assert_eq!(framing_run_endpoint(&root, &large, 512, true)?, 512);
        assert!(framing_run_endpoint(&root, &large, 512, false).is_err());
        println!(
            "FRAMING_T1_T2_T3 literal arrays, exact permutation, masks/targets, unknown/mismatch, repeated512 tape/staged evaluation PASS optimizer0 generation0 teacher0"
        );
        Ok(())
    }
    #[test]
    fn framing_t4_eq_native_new_process_resume() -> Result<()> {
        const CHILD: &str = "R3_FRAMING_PROCESS_CHILD";
        if let Ok(root) = std::env::var(CHILD) {
            return run(
                Path::new(&root),
                std::env::var("R3_FRAMING_CONTINUOUS").as_deref() == Ok("1"),
            );
        }
        let temp = tempfile::tempdir()?;
        let mut ends = vec![];
        let mut raw = vec![];
        let mut total = (0, 0, 0);
        for name in ["continuous", "segmented", "evaluation-only"] {
            let base = temp.path().join(name);
            std::fs::create_dir(&base)?;
            let old = orbit_fixture(&base)?;
            let study = base.join("frames");
            framing_prepare(&old.join("BOTH"), &study, true)?;
            fixture_review(&study)?;
            let root = study.join("EQ");
            let p = plan_read(&root)?;
            for i in 0..if name == "continuous" { 1 } else { 2 } {
                let mut cmd = std::process::Command::new(std::env::current_exe()?);
                cmd.args(["--exact","training::fresh::identifiable::binding::tests::framing_t4_eq_native_new_process_resume","--nocapture"])
                    .env(CHILD,&root).env("R3_FRAMING_CONTINUOUS",if name=="segmented"{"0"}else{"1"})
                    .env("VECLIB_MAXIMUM_THREADS","1").env("OMP_NUM_THREADS","1");
                if name == "evaluation-only" && i == 0 {
                    cmd.env("R3_FRESH_TRAIN_STOP", "training_checkpoint_saved");
                }
                let out = cmd.output()?;
                print!("{}", String::from_utf8_lossy(&out.stdout));
                assert!(
                    out.status.success(),
                    "{}",
                    String::from_utf8_lossy(&out.stderr)
                );
                assert!(String::from_utf8_lossy(&out.stdout).contains("1 passed"));
                if name == "evaluation-only" && i == 0 {
                    assert_eq!(
                        history(&root, &p)?.last().unwrap().phase.as_deref(),
                        Some("EvaluationPending")
                    );
                }
            }
            let h = history(&root, &p)?;
            let end = h.last().unwrap();
            assert_eq!(end.step, 2);
            assert!(end.resume);
            assert_eq!(end.phase.as_deref(), Some("TrainingPending"));
            assert_eq!(end.stop, "TRAINING");
            assert!(framing_run_endpoint(&root, &p, 2, false).is_err());
            let l = checkpoint::load(&root.join(&end.checkpoint), Device::Cpu, true)?;
            assert_eq!(l.manifest.framing()?, neural::Framing::EvidenceQuestion);
            assert!(l.manifest.require_default_framing().is_err());
            assert!(neural::artifact::export_inference(&base.join("export.r3m"), &l).is_err());
            let moved = base.join("moved.r3m");
            std::fs::copy(root.join(&end.checkpoint), &moved)?;
            let moved = checkpoint::load(&moved, Device::Cpu, true)?;
            assert_eq!(moved.manifest.framing()?, p.framing());
            let blocked = base.join("blocked-generic-resume");
            let error = train(Run {
                checkpoint: &root.join(&end.checkpoint), corpus: Some(&root.join("corpus.r3cor")),
                output: &blocked, resume: true, numeric_probe: false, config: p.config.clone(),
                stop_after: None, measure_rss: false, extend_steps: None, extend_microbatch: None,
                extend_sample_group_size: None, extend_curriculum_steps: None,
                extend_first_target_weight: None, extend_lr: None, extend_warmup: None,
                source_id: None, replace_corpus: false,
            }, std::sync::Arc::new(AtomicBool::new(false))).unwrap_err();
            assert!(error.to_string().contains("OBJECTIVE_POLICY_UNSUPPORTED"));
            assert!(!blocked.exists());
            let state = l.manifest.training.as_ref().unwrap();
            assert!(checkpoint::ResumeBinding::require_default(state, &l.tokenizer).is_err());
            ends.push((
                l.model.weights_content_id()?,
                optimizer_hash(&l.optimizer)?,
                state.step,
                state.sampler_state,
                state.consumed_tokens,
                state.target_tokens,
            ));
            raw.push(
                binary::read_value_records(&root.join("eval-0002-dev4.r3rows"))?[1..]
                    .iter()
                    .map(|r| r["raw_tokens"].clone())
                    .collect::<Vec<_>>(),
            );
            total.0 += state.step;
            total.1 += h.iter().map(|s| s.generations).sum::<usize>();
            total.2 += h.iter().map(|s| s.teachers).sum::<usize>();
            if name == "evaluation-only" {
                let r: binary::Value = read(&root.join("segment-0001/train-control.r3b"))?;
                assert_eq!(r["optimizer_calls"], 0);
            }
            for (panel, es, ms) in panel_cases(&root, &p, 2)? {
                audit_panel(&root, &p, 2, &panel, &es, &ms, &l.tokenizer)?;
            }
        }
        assert!(ends.windows(2).all(|w| w[0] == w[1]));
        assert!(raw.windows(2).all(|w| w[0] == w[1]));
        println!(
            "FRAMING_T4 actual EQ continuous2/newprocess1+1/evaluation-only weights Adam clock cursor token raw SAME; optimizer={} generation={} teacher={}",
            total.0, total.1, total.2
        );
        Ok(())
    }
    pub(super) fn orbit_fixture(root: &Path) -> Result<PathBuf> {
        let parent = parent(root)?;
        let old = root.join("old");
        prepare(&parent, &old, true)?;
        let study = root.join("new");
        orbit_prepare(&old.join("A"), &study, true)?;
        orbit_seal(&study)?;
        fixture_review(&study)?;
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
    fn learned_expansion_pool_tape_and_native_panels() -> Result<()> {
        let temp = tempfile::tempdir()?;
        let study = orbit_fixture(temp.path())?;
        let root = study.join("BOTH");
        let mut p = plan_read(&root)?;
        let c = verified_corpus(&root.join("corpus.r3cor"), &p.corpus)?;
        let (tm, dm, _) = verified_metadata(&root, &p)?;
        let tok = ByteBpe::load(&root.join("tokenizer.r3b"))?;
        let (es, ms, selection) = expansion_pool_from(&c, tm.clone(), &dm, &tok, &study)?;
        assert_eq!(digest(&&es[..512])?, digest(&c.train)?);
        assert_eq!(digest(&&ms[..512])?, digest(&tm)?);
        assert_eq!(selection["rows"], 1536);
        p.identifiable.as_mut().unwrap().rows = (0..2048)
            .map(|i| orbit_rows("BOTH", 512)[i % 512])
            .collect();
        let repeat = expansion_rows(&p, "REPEAT")?;
        let rebind = expansion_rows(&p, "REBIND")?;
        let mut exposures = vec![];
        for (arm, rows) in [("REPEAT", &repeat), ("REBIND", &rebind)] {
            exposures.push(expansion_exposure(&rows[2048..], arm, &es, &tok)?);
            assert_eq!(rows[..2048], own(&p).rows);
        }
        assert_eq!(exposures[0]["unique"], 512);
        assert_eq!(exposures[1]["unique"], 1536);
        assert_eq!(exposures[0]["input"], exposures[1]["input"]);
        assert_eq!(exposures[0]["target"], exposures[1]["target"]);
        let ss = samples(&es, &tok, 256)?;
        for (a, b) in repeat[2048..].iter().zip(&rebind[2048..]) {
            for (&x, &y) in a.iter().zip(b) {
                assert_eq!(x, y % 512);
                assert_eq!(ss[x].tokens[144..], ss[y].tokens[144..]);
            }
        }
        let native = temp.path().join("pool.r3cor");
        data::native::write(&native, &orbit_native(es.clone(), c.validation)?, true)?;
        let readback = data::native::read(&native)?;
        assert_eq!(digest(&readback.train)?, digest(&es)?);
        for count in [64, 512, 1024, 1536] {
            let path = temp.path().join(format!("panel-{count}.r3rows"));
            let mut f = std::fs::OpenOptions::new()
                .create_new(true)
                .write(true)
                .open(&path)?;
            append_row(
                &mut f,
                &binary::record!({"expected":count,"ids":digest(&es[..count].iter().map(|e|&e.id).collect::<Vec<_>>())?}),
            )?;
            for e in &es[..count] {
                let mut ids = tok.encode(e.answer.as_bytes())?;
                ids.push(EOS);
                append_row(
                    &mut f,
                    &binary::record!({"id":e.id,"expected":e.answer,"question":e.request.input,
                    "generated_evidence":e.request.evidence,"actual":e.answer,"raw_tokens":ids,"finish_reason":"stop",
                    "generation_completed":true,"error":null,"exact_match":true}),
                )?;
            }
            f.sync_all()?;
            drop(f);
            let rows = binary::read_value_records(&path)?;
            let score = orbit_score(&es[..count], &ms[..count], &rows[1..], &tok)?;
            assert_eq!(
                (
                    score.total,
                    score.full,
                    score.query_both,
                    score.swap_both,
                    score.all4
                ),
                (count, count, count / 2, count / 2, count / 4)
            );
            let receipt = temp.path().join(format!("panel-{count}.r3b"));
            publish_confirmed(&receipt, &score)?;
            assert_eq!(read_confirmed::<OrbitScore>(&receipt)?, score);
            assert!(orbit_score(&es[..count], &ms[..count], &rows[1..count], &tok).is_err());
            let mut invalid = rows[1..].to_vec();
            invalid.swap(0, 1);
            assert!(orbit_score(&es[..count], &ms[..count], &invalid, &tok).is_err());
            invalid[0] = invalid[1].clone();
            assert!(orbit_score(&es[..count], &ms[..count], &invalid, &tok).is_err());
        }
        let mut bad_case = es[512].clone();
        bad_case.request.evidence.items[0].event_id += 1;
        assert!(verify_rekey(&es[0], &bad_case, &tok).is_err());
        bad_case = es[512].clone();
        bad_case.answer = orbit_foil(&bad_case)?;
        assert!(verify_rekey(&es[0], &bad_case, &tok).is_err());
        p.identifiable.as_mut().unwrap().dataset = EXPANSION_DATA.into();
        p.config.max_steps = 3584;
        p.evaluation = evaluation_for(&p);
        for (step, pending, next) in [
            (2049, false, 2304),
            (2304, false, 2816),
            (2816, false, 3328),
            (3328, false, 3584),
            (3584, true, 3584),
        ] {
            assert_eq!(expansion_endpoint(&p, step, pending)?, next);
        }
        assert!(expansion_endpoint(&p, 3584, false).is_err());
        let loaded = checkpoint::load(&root.join("initial.r3m"), Device::Cpu, false)?;
        let mut control = recovery::RunControl::new(std::sync::Arc::new(AtomicBool::new(false)),
            std::time::Duration::from_secs(30), 16 * 1024 * 1024)?;
        control.set_call_limits(0, 1);
        let teacher = match recovery::fresh_teacher_with_foil(&loaded, &es[512], Some(&orbit_foil(&es[512])?), &mut control) {
            recovery::ObservedCall::Returned(result) => result?,
            recovery::ObservedCall::NotInvoked(_) => return Err(bad("new family teacher not invoked")),
        };
        assert_eq!(control.teacher_calls, 1);
        assert_eq!(teacher["target_token_observation"]["gold"], binary::record!([tok.encode(es[512].answer.as_bytes())?[0], EOS]));
        assert!(teacher["conditional_foil"]["gold_logit"].as_f64().is_some_and(f64::is_finite));
        println!("EXPANSION_NATIVE pool1536 targets_equal tape1536x8 REPEAT512x24 REBIND1536x8 panels64/512/1024/1536 strict_writer_reader_score; TINY_teacher1 optimizer0 generation0");
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
    pub(super) fn fixture_review(study: &Path) -> Result<()> {
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
