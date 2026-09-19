//! Training-only fresh corpus/policy. No renderer or oracle enters product inference.
use super::*;
use clap::Subcommand;
use replica_v3::{
    binary,
    event::GenerationLimits,
    model::ModelRequest,
    neural::transformer::Config,
    retrieval::{Evidence, EvidenceBundle},
};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, io::Write, path::PathBuf};
const REVISION: &str = "joint-educational-v1";
const SYSTEM: &str = "제공된 기록과 질문만으로 답하세요. 요구한 원문 또는 값을 쓰고 근거를 [event:번호]로 인용하세요. 근거가 없거나 모호하면 구별해서 유보하세요. 순서만으로 원인을 단정하지 마세요.";
#[derive(Subcommand)]
pub enum Command {
    Prepare {
        #[arg(long)]
        output: PathBuf,
    },
    Run {
        #[arg(long)]
        root: PathBuf,
    },
    Report {
        #[arg(long)]
        root: PathBuf,
    },
    #[cfg(feature = "test-support")]
    Fixture {
        #[arg(long)]
        output: PathBuf,
    },
    #[cfg(feature = "test-support")]
    FixtureFull {
        #[arg(long)]
        root: PathBuf,
    },
    #[cfg(feature = "test-support")]
    FixtureEval {
        #[arg(long)]
        root: PathBuf,
    },
}
fn bad(s: &str) -> Error {
    Error::Invalid(format!("fresh: {s}"))
}
fn write<T: Serialize>(p: &Path, v: &T) -> Result<()> {
    neural::write_new(p, &binary::to_storage_vec(v)?)
}
fn read<T: serde::de::DeserializeOwned>(p: &Path) -> Result<T> {
    Ok(binary::from_slice(&neural::read_bounded(
        p,
        128 * 1024 * 1024,
    )?)?)
}
fn file_hash(p: &Path) -> Result<String> {
    Ok(neural::hash(&neural::read_bounded(p, 512 * 1024 * 1024)?))
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Plan {
    revision: String,
    source: String,
    binary: String,
    corpus: String,
    transfer: String,
    tokenizer: String,
    initial: String,
    initial_weights: String,
    pub(super) config: TrainConfig,
    architecture: Config,
    sampler: String,
    order: Vec<Vec<usize>>,
    train_order: String,
    split_policy: String,
    data_seed: u64,
    model_seed: u64,
    tiny: bool,
    metadata: String,
    evaluation: EvaluationPolicy,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
struct EvaluationPolicy {
    screen_steps: Vec<usize>,
    primary_steps: Vec<usize>,
    transfer_steps: Vec<usize>,
    train_steps: Vec<usize>,
    generation_limit: usize,
    teacher_limit: usize,
    active_seconds: u64,
    segment_seconds: u64,
    cleanup_seconds: u64,
    primary_min: usize,
    bucket_min: usize,
    transfer_min: usize,
    regression_exact_loss: usize,
    regression_ce_ratio: f64,
}
impl Default for EvaluationPolicy {
    fn default() -> Self {
        Self {
            screen_steps: vec![0, 128, 512, 1024, 2048, 3072, 4096],
            primary_steps: vec![1024, 2048, 4096],
            transfer_steps: vec![2048, 4096],
            train_steps: vec![0, 1024, 2048, 4096],
            generation_limit: 4096,
            teacher_limit: 6000,
            active_seconds: 10800,
            segment_seconds: 900,
            cleanup_seconds: 120,
            primary_min: 487,
            bucket_min: 58,
            transfer_min: 116,
            regression_exact_loss: 12,
            regression_ce_ratio: 1.2,
        }
    }
}
#[derive(Clone, Serialize, Deserialize)]
struct Meta {
    id: String,
    base: String,
    template: String,
    bucket: usize,
    view: usize,
    split: String,
    entities: Vec<String>,
}
fn digest<T: Serialize>(v: &T) -> Result<String> {
    Ok(neural::hash(&binary::to_vec(v)?))
}
fn entity_pool(split: usize, length: usize) -> Vec<String> {
    // Full entity hash partitions a finite namespace. One-digit names are reused
    // within their split, never falsely described as thousands of unique names.
    let capacity = 10usize.pow(length as u32).min(2048);
    (0..capacity)
        .map(|n| format!("장치{n:0length$}"))
        .filter(|s| {
            u64::from_str_radix(&neural::hash(s.as_bytes())[..8], 16).unwrap() % 3 == split as u64
        })
        .collect()
}
fn record(entity: &str, context: &str, value: &str, id: i64, time: i64, status: &str) -> Evidence {
    Evidence {
        event_id: id,
        original_excerpt: format!("{entity}의 {context} 값은 {value}이다."),
        excerpt_truncated: false,
        source: "교육 기록".into(),
        recorded_at: time + 100,
        observed_at: Some(time),
        version_status: status.into(),
        retrieval_reason: "lexical".into(),
        relation_path: vec![],
    }
}
fn parsed_record(e: &Evidence) -> Result<(&str, &str, &str)> {
    let (entity, rest) = e
        .original_excerpt
        .split_once("의 ")
        .ok_or_else(|| bad("record grammar"))?;
    let (context, rest) = rest
        .split_once(" 값은 ")
        .ok_or_else(|| bad("record grammar"))?;
    let value = rest
        .strip_suffix("이다.")
        .filter(|v| !v.is_empty())
        .ok_or_else(|| bad("record grammar"))?;
    Ok((entity, context, value))
}
// Independent constrained resolver consumes serialized request only: no Episode
// binding, bucket, selected index, seed or expected answer is available here.
fn resolve(q: &ModelRequest) -> Result<String> {
    let mut words = q.input.splitn(3, ' ');
    let entity = words.next().ok_or_else(|| bad("question entity"))?;
    let context = words.next().ok_or_else(|| bad("question context"))?;
    let task = words.next().ok_or_else(|| bad("question grammar"))?;
    let full = matches!(
        task,
        "기록 원문 전체를 인용과 함께 써라."
            | "원문 전체와 인용을 답하라."
            | "해당 기록의 문장 전체를 근거 번호와 함께 알려줘."
    );
    let past = matches!(
        task,
        "정정 전 과거 값을 인용과 함께 써라."
            | "이전 값을 인용해 답하라."
            | "바꾸기 전의 값을 근거 번호와 함께 알려줘."
    );
    let cause = matches!(
        task,
        "기록의 시간순서만으로 원인이 확정되는가?"
            | "앞선 기록이 뒤 기록의 원인이라고 확정할 수 있는가?"
            | "먼저 일어났다는 사실이 원인임을 입증하는가?"
    );
    let current = matches!(
        task,
        "현재 값을 인용과 함께 써라."
            | "현재 값과 인용을 답하라."
            | "지금 적용할 값을 근거 번호와 함께 알려줘."
            | "복원 후 현재 값을 인용과 함께 써라."
            | "복원된 값과 인용을 답하라."
            | "다시 복원한 뒤의 값을 근거 번호와 함께 알려줘."
    );
    if !(full || past || cause || current) {
        return Err(bad("unsupported question grammar"));
    }
    let mut matched = vec![];
    for e in &q.evidence.items {
        let (name, ctx, value) = parsed_record(e)?;
        if name == entity && ctx == context {
            matched.push((e, value));
        }
    }
    if q.evidence.items.is_empty() {
        return Ok("근거가 없습니다.".into());
    }
    if matched.is_empty() {
        return Ok("요청한 대상의 근거가 없습니다.".into());
    }
    if cause {
        return Ok("시간순서만으로 원인은 확정되지 않습니다.".into());
    }
    matched.retain(|(e, _)| {
        if past {
            e.version_status == "superseded"
        } else {
            e.version_status == "current"
        }
    });
    if matched.is_empty() {
        return Ok("요청한 대상의 근거가 없습니다.".into());
    }
    let latest = matched
        .iter()
        .map(|(e, _)| e.observed_at.unwrap_or(e.recorded_at))
        .max()
        .unwrap();
    matched.retain(|(e, _)| e.observed_at.unwrap_or(e.recorded_at) == latest);
    if matched.len() != 1 {
        return Ok("근거가 모호하여 확정할 수 없습니다.".into());
    }
    let (e, value) = matched[0];
    Ok(if full {
        format!("{} [event:{}]", e.original_excerpt, e.event_id)
    } else {
        format!("{value}입니다. [event:{}]", e.event_id)
    })
}
fn generate(bases: usize, split: usize, seed: u64) -> Result<(Vec<Episode>, Vec<Meta>)> {
    let pools: Vec<_> = (1..=8).map(|len| entity_pool(split, len)).collect();
    if pools.iter().any(Vec::is_empty) {
        return Err(bad("finite entity namespace exhausted"));
    }
    let mut rng = Rng::new(seed);
    let mut episodes = vec![];
    let mut metadata = vec![];
    for bucket in 0..8 {
        for base in 0..bases {
            let pool = &pools[base % 8];
            let entity = &pool[rng.next_u64() as usize % pool.len()];
            let other_pool = &pools[(base + 1) % 8];
            let other = &other_pool[rng.next_u64() as usize % other_pool.len()];
            let context = format!("구역{}", rng.next_u64() % 1000);
            let other_context = format!("구역{}", 1000 + rng.next_u64() % 1000);
            let a = 1 + (rng.next_u64() % (10u64.pow((base % 8 + 1) as u32) - 1)) as i64;
            let b = if a == 1 || a % 2 == 0 { a + 1 } else { a - 1 };
            let c = a + 100_000_000;
            let time = 1000 + (rng.next_u64() % 100_000) as i64;
            let value = if base % 2 == 0 {
                ["왼쪽", "오른쪽", "직진", "대기"][base % 4].to_string()
            } else {
                format!(
                    "{:0width$}",
                    rng.next_u64() % 10u64.pow((base % 8 + 1) as u32),
                    width = base % 8 + 1
                )
            };
            let alt = if base % 2 == 0 {
                ["대기", "직진", "오른쪽", "왼쪽"][base % 4].to_string()
            } else {
                format!("{:0width$}", (rng.next_u64() % 9) + 1, width = base % 8 + 1)
            };
            let family = neural::hash(format!("{entity}/{context}/{time}/{a}/{bucket}").as_bytes());
            for view in 0..4 {
                let (v, w) = if view == 1 {
                    (&alt, &value)
                } else {
                    (&value, &alt)
                };
                let mut items = vec![record(entity, &context, v, a, time, "current")];
                let mut selected = 0;
                match bucket {
                    0 | 1 => {}
                    2 => items.push(record(other, &context, w, b, time, "current")),
                    3 => items.push(record(entity, &other_context, w, b, time, "current")),
                    4 => {
                        items[0].observed_at = Some(time - 1);
                        items[0].original_excerpt =
                            record(entity, &context, w, a, time, "current").original_excerpt;
                        items.push(record(entity, &context, v, b, time, "current"));
                        selected = 1;
                    }
                    5 => {
                        items[0].version_status = "superseded".into();
                        items.push(record(entity, &context, w, b, time + 1, "superseded"));
                        items.push(record(entity, &context, v, c, time + 2, "current"));
                        selected = if base % 2 == 0 { 1 } else { 2 };
                    }
                    6 => match base % 3 {
                        0 => items.clear(),
                        1 => items[0] = record(other, &context, w, a, time, "current"),
                        _ => items.push(record(entity, &context, w, b, time, "current")),
                    },
                    7 => items.push(record(entity, &context, w, b, time + 1, "current")),
                    _ => unreachable!(),
                }
                let past = bucket == 5 && base % 2 == 0;
                let form = if split == 2 {
                    2
                } else if view == 3 {
                    1
                } else {
                    0
                };
                let tasks = if bucket == 0 {
                    [
                        "기록 원문 전체를 인용과 함께 써라.",
                        "원문 전체와 인용을 답하라.",
                        "해당 기록의 문장 전체를 근거 번호와 함께 알려줘.",
                    ]
                } else if past {
                    [
                        "정정 전 과거 값을 인용과 함께 써라.",
                        "이전 값을 인용해 답하라.",
                        "바꾸기 전의 값을 근거 번호와 함께 알려줘.",
                    ]
                } else if bucket == 5 {
                    [
                        "복원 후 현재 값을 인용과 함께 써라.",
                        "복원된 값과 인용을 답하라.",
                        "다시 복원한 뒤의 값을 근거 번호와 함께 알려줘.",
                    ]
                } else if bucket == 7 {
                    [
                        "기록의 시간순서만으로 원인이 확정되는가?",
                        "앞선 기록이 뒤 기록의 원인이라고 확정할 수 있는가?",
                        "먼저 일어났다는 사실이 원인임을 입증하는가?",
                    ]
                } else {
                    [
                        "현재 값을 인용과 함께 써라.",
                        "현재 값과 인용을 답하라.",
                        "지금 적용할 값을 근거 번호와 함께 알려줘.",
                    ]
                };
                let answer = if bucket == 6 {
                    [
                        "근거가 없습니다.",
                        "요청한 대상의 근거가 없습니다.",
                        "근거가 모호하여 확정할 수 없습니다.",
                    ][base % 3]
                        .to_string()
                } else if bucket == 7 {
                    "시간순서만으로 원인은 확정되지 않습니다.".into()
                } else if bucket == 0 {
                    format!(
                        "{} [event:{}]",
                        items[selected].original_excerpt, items[selected].event_id
                    )
                } else {
                    format!(
                        "{}입니다. [event:{}]",
                        if past { w } else { v },
                        items[selected].event_id
                    )
                };
                // Transfer contains both unseen wording and unseen record-count combinations.
                if split == 2 && base >= 2 && bucket < 6 && items.len() < 3 {
                    items.push(record(other, &other_context, w, c + 1, time, "current"));
                }
                if view == 2 || (base + bucket) % 2 == 0 {
                    items.reverse();
                }
                let id = format!("{family}-{view}");
                let request = ModelRequest {
                    request_id: id.clone(),
                    system: SYSTEM.into(),
                    input: format!("{entity} {context} {}", tasks[form]),
                    evidence: EvidenceBundle {
                        items,
                        ..Default::default()
                    },
                    limits: GenerationLimits {
                        context_tokens: 2048,
                        max_tokens: 128,
                        timeout_ms: 120000,
                    },
                };
                if resolve(&request)? != answer {
                    return Err(bad("independent label disagreement"));
                }
                metadata.push(Meta {
                    id: id.clone(),
                    base: family.clone(),
                    template: format!(
                        "{}-{form}",
                        if bucket == 0 {
                            "full"
                        } else if past {
                            "past"
                        } else if bucket == 7 {
                            "cause"
                        } else {
                            "current"
                        }
                    ),
                    bucket,
                    view,
                    split: ["train", "primary", "transfer"][split].into(),
                    entities: vec![entity.clone(), other.clone()],
                });
                episodes.push(Episode {
                    id,
                    category: [3, 3, 0, 2, 1, 1, 4, 4][bucket],
                    family: family.clone(),
                    binding: format!("{entity}/{other}"),
                    sequence: format!("{entity}/{context}/{time}"),
                    request,
                    answer,
                });
            }
        }
    }
    data::validate_episodes(&episodes)?;
    Ok((episodes, metadata))
}
fn corpus(train: Vec<Episode>, dev: Vec<Episode>, seed: u64) -> Result<data::native::Corpus> {
    let m=data::CorpusManifest {version:1,scope:"educational synthetic Korean only".into(),permission:"synthetic project-owned".into(),generator:REVISION.into(),seed,split_rule:"full-entity SHA256 namespace mod3; base/scene disjoint; primary grammar shared; transfer phrasing heldout".into(),train:data::native::split("train",&train),validation:data::native::split("validation",&dev)};
    data::native::from_episodes(m, train, dev)
}
impl Plan {
    pub(super) fn binding(
        &self,
        s: &TrainingState,
        t: &ByteBpe,
    ) -> Result<checkpoint::ResumeBinding> {
        let mut b = checkpoint::ResumeBinding::default_for(s, t);
        b.execution = 1;
        b.policy = checkpoint::ResumeBinding::digest_bytes(&binary::to_vec(self)?);
        b.provenance = b.policy;
        b.train_order = checkpoint::ResumeBinding::digest_bytes(self.train_order.as_bytes());
        Ok(b)
    }
    pub(super) fn draw(&self, step: usize) -> Vec<usize> {
        let epoch = step / self.order[0].len();
        let position = step % self.order[0].len();
        (0..8)
            .map(|bucket| {
                let order = &self.order[bucket];
                let bases = order.len() / 4;
                let mut permutation: Vec<_> = (0..bases).collect();
                let mut r = Rng::new(
                    self.config.seed
                        ^ (bucket as u64 + 1).wrapping_mul(0x9e3779b97f4a7c15)
                        ^ (epoch as u64).wrapping_mul(0x517cc1b727220a95),
                );
                for i in (1..bases).rev() {
                    let j = r.next_u64() as usize % (i + 1);
                    permutation.swap(i, j);
                }
                // Each pass uses a different view; adjacent draws use distinct bases.
                order[permutation[position % bases] * 4 + (position / bases + epoch + bucket) % 4]
            })
            .collect()
    }
}
fn prepare(root: &Path, tiny: bool) -> Result<()> {
    std::fs::create_dir(root)?;
    let (train, tm) = generate(if tiny { 2 } else { 256 }, 0, 20260919)?;
    let (dev, dm) = generate(if tiny { 1 } else { 16 }, 1, 20260920)?;
    let (transfer, xm) = generate(if tiny { 1 } else { 4 }, 2, 20260921)?;
    let c = corpus(train.clone(), dev.clone(), 20260919)?;
    let x = corpus(train.clone(), transfer.clone(), 20260919)?;
    data::native::write(&root.join("corpus.r3cor"), &c, true)?;
    data::native::write(&root.join("transfer.r3cor"), &x, true)?;
    write(&root.join("metadata.r3b"), &(tm.clone(), dm, xm))?;
    data::tokenizer(
        &root.join("corpus.r3cor"),
        &root.join("tokenizer.r3b"),
        4096,
    )?;
    let tok = ByteBpe::load(&root.join("tokenizer.r3b"))?;
    let mut prompts = HashMap::new();
    for es in [&train, &dev, &transfer] {
        let framed = samples(es, &tok, 512)?;
        for (e, s) in es.iter().zip(&framed) {
            let p = tok.prepare(&e.request, 2048, "length-check")?;
            if s.tokens.len() > 513
                || p.provided.len() != e.request.evidence.items.len()
                || !p.excluded.is_empty()
            {
                return Err(bad("full evidence/sequence length"));
            }
            if prompts
                .insert(s.tokens[..s.response_start].to_vec(), e.answer.clone())
                .is_some_and(|old| old != e.answer)
            {
                return Err(bad("conflicting target for identical prompt"));
            }
        }
        println!(
            "FRESH_DATA examples={} input_target_tokens={} max_sequence={}",
            es.len(),
            framed.iter().map(|s| s.tokens.len()).sum::<usize>(),
            framed.iter().map(|s| s.tokens.len()).max().unwrap()
        );
    }
    let mut config = TrainConfig {
        lr: 3e-4,
        warmup: 128,
        max_steps: 4096,
        max_tokens: 20_000_000,
        microbatch: 8,
        accumulation: 1,
        validate_every: 512,
        seed: 29,
        ..Default::default()
    };
    let mut architecture = if tiny {
        Config::tiny(tok.vocab_size())
    } else {
        Config::small(tok.vocab_size())
    };
    if tiny {
        architecture.context = 512;
        architecture.profile = "NATIVE_TRPP_EXPERIMENTAL_V1".into();
        config.max_steps = 2;
        config.warmup = 0;
        config.validate_every = 2;
    }
    let estimate =
        config.microbatch * architecture.layers * architecture.heads * 512 * 512 * 4 * 12
            + architecture.parameters() * 4 * 16;
    if estimate > 16usize * 1024 * 1024 * 1024 {
        config.microbatch = 2;
        config.accumulation = 4;
    }
    config.validate(architecture.context)?;
    let source = source_digest()?;
    let model = Transformer::init(architecture.clone(), 17, Device::Cpu)?;
    let m = checkpoint::initialized(&model, &tok, 17, source.clone())?;
    checkpoint::save(&root.join("initial.r3m"), &model, &tok, m, &BTreeMap::new())?;
    let order: Vec<Vec<usize>> = (0..8)
        .map(|b| {
            tm.iter()
                .enumerate()
                .filter_map(|(i, m)| (m.bucket == b).then_some(i))
                .collect()
        })
        .collect();
    let plan = Plan {
        revision: REVISION.into(),
        source,
        binary: file_hash(&std::env::current_exe()?)?,
        corpus: file_hash(&root.join("corpus.r3cor"))?,
        transfer: file_hash(&root.join("transfer.r3cor"))?,
        tokenizer: tok.id(),
        initial: file_hash(&root.join("initial.r3m"))?,
        initial_weights: model.weight_hash()?,
        config,
        architecture,
        sampler: "bucket-base-permutation-v1".into(),
        train_order: digest(&order)?,
        order,
        split_policy: c.manifest.split_rule,
        data_seed: 20260919,
        model_seed: 17,
        tiny,
        metadata: file_hash(&root.join("metadata.r3b"))?,
        evaluation: EvaluationPolicy::default(),
    };
    write(&root.join("plan.r3b"), &plan)?;
    let check: Plan = read(&root.join("plan.r3b"))?;
    if check != plan {
        return Err(bad("plan readback"));
    }
    println!(
        "FRESH_PREPARED root={} vocab={} parameters={} initial_weights={} plan={} model=NOT_TRAINED planning_bytes={estimate} optimizer=0",
        root.display(),
        tok.vocab_size(),
        model.config.parameters(),
        plan.initial_weights,
        digest(&plan)?
    );
    Ok(())
}
fn source_digest() -> Result<String> {
    let mut bytes = vec![];
    // Compile-time source identity; no temporary instruction is a runtime input.
    for source in [
        include_bytes!("fresh.rs").as_slice(),
        include_bytes!("training.rs").as_slice(),
        include_bytes!("data.rs").as_slice(),
        include_bytes!("native_corpus.rs").as_slice(),
        include_bytes!("neural.rs").as_slice(),
        include_bytes!("neural/transformer.rs").as_slice(),
        include_bytes!("neural/artifact.rs").as_slice(),
        include_bytes!("neural/checkpoint.rs").as_slice(),
        include_bytes!("../Cargo.lock").as_slice(),
    ] {
        bytes.extend_from_slice(source);
    }
    Ok(neural::hash(&bytes))
}
pub fn execute(command: Command) -> Result<()> {
    match command {
        Command::Prepare { output } => prepare(&output, false),
        Command::Run { root } => run(&root, false),
        Command::Report { root } => report(&root),
        #[cfg(feature = "test-support")]
        Command::Fixture { output } => prepare(&output, true),
        #[cfg(feature = "test-support")]
        Command::FixtureFull { root } => run(&root, true),
        #[cfg(feature = "test-support")]
        Command::FixtureEval { root } => {
            let p = plan_read(&root)?;
            if !p.tiny {
                return Err(bad("numerical fixture required"));
            }
            let history = history(&root, &p)?;
            let last = history
                .last()
                .ok_or_else(|| bad("fixture missing checkpoint"))?;
            let (_, _, dev) = data::load(&root.join("corpus.r3cor"))?;
            let (_, dm, _): (Vec<Meta>, Vec<Meta>, Vec<Meta>) = read(&root.join("metadata.r3b"))?;
            let mut control = recovery::RunControl::command(false)?;
            evaluate_panel(
                &p,
                &root,
                &root.join(&last.checkpoint),
                last.step,
                "fixture",
                &dev[..1],
                &dm[..1],
                &mut control,
            )?;
            println!("FIXTURE_EVAL {}", control.receipt());
            Ok(())
        }
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
struct Segment {
    policy: String,
    checkpoint: String,
    checkpoint_hash: String,
    step: usize,
    input: u64,
    target: u64,
    elapsed: f64,
    generations: usize,
    teachers: usize,
    resume: bool,
    stop: String,
}
fn plan_read(root: &Path) -> Result<Plan> {
    let p: Plan = read(&root.join("plan.r3b"))?;
    if p.revision != REVISION
        || p.source != source_digest()?
        || p.binary != file_hash(&std::env::current_exe()?)?
        || p.sampler != "bucket-base-permutation-v1"
        || p.order.len() != 8
        || p.train_order != digest(&p.order)?
    {
        return Err(bad("source/binary/sampler plan mismatch"));
    }
    for (name, expected) in [
        ("corpus.r3cor", &p.corpus),
        ("transfer.r3cor", &p.transfer),
        ("initial.r3m", &p.initial),
        ("metadata.r3b", &p.metadata),
    ] {
        if &file_hash(&root.join(name))? != expected {
            return Err(bad("frozen input changed"));
        }
    }
    if p.evaluation != EvaluationPolicy::default() {
        return Err(bad("unsupported evaluation policy"));
    }
    Ok(p)
}
fn history(root: &Path, p: &Plan) -> Result<Vec<Segment>> {
    let mut out = vec![];
    for i in 0..128 {
        if !root.join(format!("segment-{i:04}-started.r3b")).exists() {
            return Ok(out);
        }
        let s: Segment = read(&root.join(format!("segment-{i:04}-finished.r3b")))
            .map_err(|_| bad("started segment without completion: usage UNKNOWN, retry blocked"))?;
        if s.policy != digest(p)? || file_hash(&root.join(&s.checkpoint))? != s.checkpoint_hash {
            return Err(bad("segment binding"));
        }
        out.push(s);
    }
    Err(bad("segment count bound"))
}
fn run(root: &Path, uninterrupted_fixture: bool) -> Result<()> {
    let p = plan_read(root)?;
    let previous = history(root, &p)?;
    if uninterrupted_fixture && !p.tiny {
        return Err(bad("fixture requires TINY"));
    }
    if previous.last().is_some_and(|s| !s.resume) {
        return Err(bad("closed run cannot resume"));
    }
    let elapsed: f64 = previous.iter().map(|s| s.elapsed).sum();
    let generations: usize = previous.iter().map(|s| s.generations).sum();
    let teachers: usize = previous.iter().map(|s| s.teachers).sum();
    if elapsed >= p.evaluation.active_seconds as f64
        || generations >= p.evaluation.generation_limit
        || teachers >= p.evaluation.teacher_limit
    {
        return Err(bad("total budget exhausted"));
    }
    let step = previous.last().map_or(0, |s| s.step);
    let input = root.join(
        previous
            .last()
            .map_or("initial.r3m", |s| s.checkpoint.as_str()),
    );
    let index = previous.len();
    let output = root.join(format!("segment-{index:04}"));
    let stop_after = if step == 0 && !uninterrupted_fixture {
        1
    } else {
        p.config.max_steps
    };
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
    control.set_call_limits(
        p.evaluation.generation_limit - generations,
        p.evaluation.teacher_limit - teachers,
    );
    let timer = Instant::now();
    write(
        &root.join(format!("segment-{index:04}-started.r3b")),
        &binary::record!({"policy":digest(&p)?,"checkpoint":input,"physical":file_hash(&input)?,"step":step,"prior_seconds":elapsed,"generation_limit":4096-generations,"teacher_limit":6000-teachers}),
    )?;
    let r = super::train_with_policy(
        Run {
            checkpoint: &input,
            corpus: Some(&root.join("corpus.r3cor")),
            output: &output,
            resume: !previous.is_empty(),
            numeric_probe: false,
            config: p.config.clone(),
            stop_after: Some(stop_after),
            measure_rss: true,
            extend_steps: None,
            extend_microbatch: None,
            extend_sample_group_size: None,
            extend_curriculum_steps: None,
            extend_first_target_weight: None,
            extend_lr: None,
            extend_warmup: None,
            source_id: None,
            replace_corpus: false,
        },
        &mut control,
        Some((&p, root)),
    );
    let receipt: binary::Value = read(&output.join("train-control.r3b"))?;
    let path = output.join("final");
    let (m, _) = checkpoint::metadata(&path)?;
    let s = m.training.ok_or_else(|| bad("missing state"))?;
    let stop = receipt["fresh_stop"]
        .as_str()
        .or_else(|| receipt["reason"].as_str())
        .ok_or_else(|| bad("missing stop"))?
        .to_string();
    let resume = s.step < p.config.max_steps
        && s.consumed_tokens < p.config.max_tokens
        && receipt["fresh_stop"].is_null()
        && receipt["save_error"].is_null()
        && (r.is_ok()
            || (stop == "TIME_BUDGET"
                && receipt["observed_conditions"]
                    .as_array()
                    .is_some_and(|v| v.iter().all(|x| x == "TIME_BUDGET"))));
    let segment = Segment {
        policy: digest(&p)?,
        checkpoint: path.strip_prefix(root).unwrap().to_string_lossy().into(),
        checkpoint_hash: file_hash(&path)?,
        step: s.step,
        input: s.consumed_tokens,
        target: s.target_tokens,
        elapsed: timer.elapsed().as_secs_f64(),
        generations: receipt["generation_calls"]
            .as_u64()
            .ok_or_else(|| bad("missing generation usage"))? as usize,
        teachers: receipt["teacher_calls"]
            .as_u64()
            .ok_or_else(|| bad("missing teacher usage"))? as usize,
        resume,
        stop,
    };
    write(
        &root.join(format!("segment-{index:04}-finished.r3b")),
        &segment,
    )?;
    println!(
        "FRESH_SEGMENT step={} input={} target={} generations={} teachers={} elapsed={} durable={} resume={} stop={}",
        segment.step,
        segment.input,
        segment.target,
        segment.generations,
        segment.teachers,
        segment.elapsed,
        segment.checkpoint,
        segment.resume,
        segment.stop
    );
    if resume && segment.stop == "TIME_BUDGET" {
        Ok(())
    } else {
        r
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
struct PanelResult {
    step: usize,
    panel: String,
    model: String,
    dataset: String,
    exact: usize,
    total: usize,
    errors: usize,
    eos: usize,
    buckets: [usize; 8],
    denominators: [usize; 8],
    base4: usize,
    raw_hash: String,
    ce: Option<f64>,
    citation_exact: usize,
    field_exact: usize,
    invalid_utf8: usize,
    length_ended: usize,
    strata: BTreeMap<String, [usize; 2]>,
}
fn score(rows: &[binary::Value], episodes: &[Episode], meta: &[Meta]) -> Result<PanelResult> {
    if rows.len() != episodes.len() || meta.len() != episodes.len() {
        return Err(bad("incomplete evaluation"));
    }
    let mut s = PanelResult {
        step: 0,
        panel: String::new(),
        model: String::new(),
        dataset: String::new(),
        exact: 0,
        total: rows.len(),
        errors: 0,
        eos: 0,
        buckets: [0; 8],
        denominators: [0; 8],
        base4: 0,
        raw_hash: String::new(),
        ce: None,
        citation_exact: 0,
        field_exact: 0,
        invalid_utf8: 0,
        length_ended: 0,
        strata: BTreeMap::new(),
    };
    let mut bases: BTreeMap<&str, (usize, usize)> = BTreeMap::new();
    for ((row, e), m) in rows.iter().zip(episodes).zip(meta) {
        if row["id"] != e.id
            || row["expected"] != e.answer
            || row["question"] != e.request.input
            || row["generated_evidence"] != binary::record!(e.request.evidence)
        {
            return Err(bad("raw input/content mismatch"));
        }
        let eos = row["finish_reason"] == "stop" && row["generation_completed"] == true;
        let error = !row["error"].is_null() || !eos;
        let exact = !error && row["actual"] == e.answer;
        if row["exact_match"] != exact {
            return Err(bad("raw score mismatch"));
        }
        s.exact += usize::from(exact);
        s.errors += usize::from(error);
        s.eos += usize::from(eos);
        s.denominators[m.bucket] += 1;
        s.buckets[m.bucket] += usize::from(exact);
        s.invalid_utf8 += usize::from(row["error_class"] == "strict_utf8");
        s.length_ended += usize::from(row["finish_reason"] == "length");
        if let Some(actual) = row["actual"].as_str() {
            s.citation_exact += usize::from(citations(actual).ok() == citations(&e.answer).ok());
            let field = |s: &str| s.split_once(" [event:").map_or(s, |(a, _)| a).to_string();
            s.field_exact += usize::from(field(actual) == field(&e.answer));
        }
        for key in [
            format!("template:{}", m.template),
            format!("records:{}", e.request.evidence.items.len()),
            format!(
                "entity_digits:{}",
                m.entities[0].chars().filter(char::is_ascii_digit).count()
            ),
            format!(
                "prompt_length_64_bin:{}",
                row["prompt_length"].as_u64().unwrap_or(0) / 64
            ),
        ] {
            let count = s.strata.entry(key).or_default();
            count[0] += usize::from(exact);
            count[1] += 1;
        }
        let b = bases.entry(&m.base).or_default();
        b.0 += 1;
        b.1 += usize::from(exact);
    }
    s.base4 = bases.values().filter(|&&(n, k)| n == 4 && k == 4).count();
    Ok(s)
}
fn verify_generated(row: &binary::Value, tok: &ByteBpe) -> Result<()> {
    let raw = row["raw_tokens"]
        .as_array()
        .ok_or_else(|| bad("missing raw tokens"))?;
    let ids = raw
        .iter()
        .map(|x| {
            x.as_u64()
                .and_then(|n| u32::try_from(n).ok())
                .ok_or_else(|| bad("raw token"))
        })
        .collect::<Result<Vec<_>>>()?;
    if row["finish_reason"] == "stop" && ids.last() != Some(&EOS) {
        return Err(bad("EOS receipt mismatch"));
    }
    if let Some(actual) = row["actual"].as_str() {
        let n = ids.len() - usize::from(ids.last() == Some(&EOS));
        if tok.decode(&ids[..n])? != actual {
            return Err(bad("raw text mismatch"));
        }
    }
    Ok(())
}
fn evaluate_panel(
    p: &Plan,
    root: &Path,
    path: &Path,
    step: usize,
    name: &str,
    episodes: &[Episode],
    meta: &[Meta],
    control: &mut recovery::RunControl,
) -> Result<PanelResult> {
    let prefix = format!("eval-{step:04}-{name}");
    let raw = root.join(format!("{prefix}.r3rows"));
    let summary = root.join(format!("{prefix}.r3b"));
    let dataset = digest(&episodes)?;
    let l = checkpoint::load(path, Device::Cpu, false)?;
    let model = l.model.weight_hash()?;
    if l.manifest.training.as_ref().is_none_or(|s| {
        s.step != step || s.resume_binding.as_ref() != p.binding(s, &l.tokenizer).ok().as_ref()
    }) {
        return Err(bad("evaluation checkpoint step/policy"));
    }
    let binding = binary::record!({"policy":digest(p)?,"step":step,"model":model,"dataset":dataset,"tokenizer":l.tokenizer.id(),"panel":name,"planned":episodes.len(),"decoding":"normal-greedy-strict-utf8-eos"});
    let header = binary::record!({"binding":binding,"checkpoint":path,"physical":file_hash(path)?});
    let mut existing = if raw.exists() {
        binary::read_value_records(&raw)?
    } else {
        vec![]
    };
    if let Some(old) = existing.first() {
        let original = Path::new(
            old["checkpoint"]
                .as_str()
                .ok_or_else(|| bad("missing evaluation checkpoint"))?,
        );
        if old["binding"] != binding
            || old["physical"] != file_hash(original)?
            || checkpoint::metadata(original)?.0.model_content_digest
                != l.manifest.model_content_digest
        {
            return Err(bad("evaluation header mismatch"));
        }
    }
    let rows = if existing.is_empty() {
        vec![]
    } else {
        existing.drain(1..).collect::<Vec<_>>()
    };
    if rows.len() > episodes.len() {
        return Err(bad("extra rows"));
    }
    for row in &rows {
        verify_generated(row, &l.tokenizer)?;
    }
    if summary.exists() {
        let s: PanelResult = read(&summary)?;
        let check = score(&rows, episodes, meta)?;
        if s.raw_hash != file_hash(&raw)?
            || s.exact != check.exact
            || s.model != model
            || s.dataset != dataset
        {
            return Err(bad("evaluation summary disagreement"));
        }
        return Ok(s);
    }
    let mut f = std::fs::OpenOptions::new()
        .create_new(!raw.exists())
        .append(true)
        .open(&raw)?;
    if existing.is_empty() {
        binary::write_value_record(&mut f, &header)?;
        f.sync_all()?;
    }
    let mut rows = rows;
    let pending = root.join(format!("{prefix}-pending.r3b"));
    if pending.exists() {
        return Err(bad("unreturned generation usage UNKNOWN; do not retry"));
    }
    for e in episodes.iter().skip(rows.len()) {
        control.check("fresh_panel_next")?;
        write(
            &pending,
            &binary::record!({"model":model,"case":e.id,"row":rows.len()}),
        )?;
        let row = recovery::evaluate_one(&l, e, &e.request, control);
        binary::write_value_record(&mut f, &row)?;
        f.flush()?;
        f.sync_all()?;
        std::fs::remove_file(&pending)?;
        verify_generated(&row, &l.tokenizer)?;
        rows.push(row);
        control.check("fresh_panel_row_durable")?;
    }
    let mut s = score(&rows, episodes, meta)?;
    s.step = step;
    s.panel = name.into();
    s.model = model;
    s.dataset = dataset;
    s.raw_hash = file_hash(&raw)?;
    // Same 64-case development panel supplies a stable teacher curve.
    let mut nll = 0.;
    let mut targets = 0.;
    for row in &rows {
        let t = &row["teacher_forced_diagnostic_after_generation"];
        if let (Some(n), Some(v)) = (
            t["target_tokens_including_eos"].as_u64(),
            t["mean_nll"].as_f64(),
        ) {
            nll += n as f64 * v;
            targets += n as f64;
        }
    }
    if targets > 0. {
        s.ce = Some(nll / targets);
    }
    write(&summary, &s)?;
    println!(
        "FRESH_EVAL step={step} panel={name} exact={}/{} errors={} eos={} buckets={:?} base4={} model={}",
        s.exact, s.total, s.errors, s.eos, s.buckets, s.base4, s.model
    );
    Ok(s)
}
pub(super) fn evaluate_boundary(
    p: &Plan,
    root: &Path,
    path: &Path,
    step: usize,
    control: &mut recovery::RunControl,
) -> Result<Option<String>> {
    if p.tiny {
        return Ok(None);
    }
    if ![0, 128, 512, 1024, 1536, 2048, 2560, 3072, 3584, 4096].contains(&step) {
        return Ok(None);
    }
    let (_, train, dev) = data::load(&root.join("corpus.r3cor"))?;
    let (_, _, transfer) = data::load(&root.join("transfer.r3cor"))?;
    let (tm, dm, xm): (Vec<Meta>, Vec<Meta>, Vec<Meta>) = read(&root.join("metadata.r3b"))?;
    let subset = |es: &[Episode], ms: &[Meta]| {
        let ids: Vec<_> = (0..8)
            .flat_map(|b| {
                ms.iter()
                    .enumerate()
                    .filter(move |(_, m)| m.bucket == b)
                    .take(8)
                    .map(|(i, _)| i)
            })
            .collect();
        (
            ids.iter().map(|&i| es[i].clone()).collect::<Vec<_>>(),
            ids.iter().map(|&i| ms[i].clone()).collect::<Vec<_>>(),
        )
    };
    let (probe, pm) = subset(&train, &tm);
    // Train generation is fixed64 at0/1024/2048/4096; auxiliary probes never select labels.
    if p.evaluation.train_steps.contains(&step) {
        evaluate_panel(p, root, path, step, "train64", &probe, &pm, control)?;
    }
    let mut screen = None;
    if p.evaluation.screen_steps.contains(&step) {
        let (es, ms) = subset(&dev, &dm);
        screen = Some(evaluate_panel(
            p, root, path, step, "screen64", &es, &ms, control,
        )?);
    }
    if step.is_multiple_of(512) {
        let l = checkpoint::load(path, Device::Cpu, false)?;
        let teacher = root.join(format!("teacher-{step:04}.r3b"));
        if !teacher.exists() {
            let sampled = samples(&probe, &l.tokenizer, 512)?;
            let loss = validation_loss(&l.model, &sampled, control)?;
            write(
                &teacher,
                &binary::record!({"step":step,"model":l.model.weight_hash()?,"panel":"train64-fixed","dataset":digest(&probe)?,"ce":loss,"teacher_calls":sampled.len()}),
            )?;
        }
    }
    let mut full = None;
    if p.evaluation.primary_steps.contains(&step) {
        full = Some(evaluate_panel(
            p, root, path, step, "dev512", &dev, &dm, control,
        )?);
    }
    if p.evaluation.transfer_steps.contains(&step) {
        let transfer = evaluate_panel(p, root, path, step, "transfer128", &transfer, &xm, control)?;
        let f = full.as_ref().unwrap();
        if f.exact >= p.evaluation.primary_min
            && f.buckets.iter().all(|&x| x >= p.evaluation.bucket_min)
            && f.errors == 0
            && transfer.exact >= p.evaluation.transfer_min
            && transfer.errors == 0
        {
            return Ok(Some("DEVELOPMENT_PASS_FINAL_NOT_RUN".into()));
        }
    }
    if step == 1024 && full.as_ref().is_some_and(|f| f.exact == 0) {
        let t: PanelResult = read(&root.join("eval-1024-train64.r3b"))?;
        let a: binary::Value = read(&root.join("teacher-0000.r3b"))?;
        let b: binary::Value = read(&root.join("teacher-1024.r3b"))?;
        if t.exact == 0 && b["ce"].as_f64() >= a["ce"].as_f64() {
            return Ok(Some("NO_LEARNING_SIGNAL".into()));
        }
    }
    if step == 2048 {
        let f = full.as_ref().unwrap();
        let prev: PanelResult = read(&root.join("eval-1024-dev512.r3b"))?;
        if f.buckets.iter().filter(|&&x| x == 0).count() >= 4 && f.exact <= prev.exact {
            return Ok(Some("NO_TRANSFER_SIGNAL".into()));
        }
    }
    if let Some(s) = screen {
        if step >= 1024 {
            let mut streak = 0;
            let mut best = 0;
            let mut best_ce = None;
            for n in [0, 128, 512, 1024, 2048, 3072, 4096]
                .into_iter()
                .filter(|&n| n <= step)
            {
                let f = root.join(format!("eval-{n:04}-screen64.r3b"));
                if !f.exists() {
                    continue;
                }
                let r: PanelResult = read(&f)?;
                if best >= 32
                    && r.exact + 12 <= best
                    && r.ce.zip(best_ce).is_some_and(|(a, b)| a >= b * 1.2)
                {
                    streak += 1;
                } else {
                    streak = 0;
                }
                if r.exact > best {
                    best = r.exact;
                    best_ce = r.ce;
                }
            }
            if streak >= 2 {
                return Ok(Some("QUALITY_REGRESSION".into()));
            }
        }
        let _ = s;
    }
    Ok(None)
}
fn report(root: &Path) -> Result<()> {
    let p = plan_read(root)?;
    let h = history(root, &p)?;
    for s in &h {
        println!(
            "segment step={} input={} target={} gen={} teacher={} resume={} stop={} durable={}",
            s.step, s.input, s.target, s.generations, s.teachers, s.resume, s.stop, s.checkpoint
        );
    }
    println!(
        "SMALL_UPDATES={} GENERATIONS={} TEACHERS={} ACTIVE_SECONDS={} S4=NOT_ACCEPTED GOAL1_READY=false",
        if p.tiny {
            0
        } else {
            h.last().map_or(0, |s| s.step)
        },
        h.iter().map(|s| s.generations).sum::<usize>(),
        h.iter().map(|s| s.teachers).sum::<usize>(),
        h.iter().map(|s| s.elapsed).sum::<f64>()
    );
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;
    #[test]
    fn fresh_accumulation_and_metadata_boundary() {
        let tok =
            ByteBpe::train(&[b"abc123".to_vec()], &neural::hash(b"fresh numeric"), 264).unwrap();
        for digit in b'0'..=b'9' {
            assert_eq!(tok.encode(&[digit, digit]).unwrap().len(), 2);
        }
        let model = Transformer::init(Config::tiny(264), 17, Device::Cpu).unwrap();
        let inputs = vec![
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
        let grad = |ids: &[usize]| {
            let b = batch(&inputs, ids, &Device::Cpu).unwrap();
            let (_, loss, n) =
                response_loss(&model.forward(&b.input, Some(&b.valid)).unwrap(), &b, 1.).unwrap();
            let g = loss.backward().unwrap();
            (
                model
                    .vars
                    .iter()
                    .map(|(k, v)| (k.clone(), g.get(v).unwrap().detach()))
                    .collect::<BTreeMap<_, _>>(),
                n,
            )
        };
        let (full, n) = grad(&[0, 1]);
        let (a, na) = grad(&[0]);
        let (b, nb) = grad(&[1]);
        assert_eq!(n, na + nb);
        for (k, g) in &full {
            let micro = ((&a[k] * na as f64).unwrap() + (&b[k] * nb as f64).unwrap()).unwrap();
            let error = (g - (micro / n as f64).unwrap())
                .unwrap()
                .abs()
                .unwrap()
                .max_all()
                .unwrap()
                .to_scalar::<f32>()
                .unwrap();
            assert!(error < 3e-5, "{k} {error}");
        }
        let d = tempfile::tempdir().unwrap();
        let mut m = checkpoint::initialized(
            &model,
            &tok,
            17,
            neural::hash(b"synthetic-count-not-executed"),
        )
        .unwrap();
        let mut state = TrainingState {
            resume_binding: None,
            contrast16: false,
            parent_checkpoint_hash: None,
            config: TrainConfig {
                max_steps: 4096,
                warmup: 128,
                seq_len: 64,
                seed: 29,
                ..Default::default()
            },
            step: 4096,
            consumed_tokens: 20_000_000,
            target_tokens: 1_000_000,
            sampler_state: 4096,
            corpus_hash: tok.train_hash.clone(),
            validation_hash: neural::hash(b"synthetic-dev"),
            previous_corpora: vec![],
            initial_weight_hash: model.weight_hash().unwrap(),
            train_loss: Some(0.12345678901234567),
            validation_loss: None,
        };
        state.resume_binding = Some(checkpoint::ResumeBinding::default_for(&state, &tok));
        m.training = Some(state.clone());
        m.status = "BUDGET_REACHED".into();
        let adam = Adam::new(&model.vars).unwrap();
        checkpoint::save(
            &d.path().join("synthetic4096.r3m"),
            &model,
            &tok,
            m,
            &adam.moments,
        )
        .unwrap();
        let loaded =
            checkpoint::load(&d.path().join("synthetic4096.r3m"), Device::Cpu, true).unwrap();
        assert_eq!(loaded.manifest.training.unwrap(), state);
        println!("SYNTHETIC_METADATA_STEP=4096 OPTIMIZER_CALLS=0 backward_calls=3");
    }
    #[test]
    fn fresh_data_disjoint_resolver_and_balanced_epoch() {
        let (train, tm) = generate(256, 0, 20260919).unwrap();
        let (dev, dm) = generate(16, 1, 20260920).unwrap();
        let (transfer, xm) = generate(4, 2, 20260921).unwrap();
        assert_eq!((train.len(), dev.len(), transfer.len()), (8192, 512, 128));
        data::check_split(&train, &dev).unwrap();
        data::check_split(&train, &transfer).unwrap();
        let entities = |m: &[Meta]| {
            m.iter()
                .flat_map(|m| m.entities.iter().cloned())
                .collect::<BTreeSet<_>>()
        };
        assert!(entities(&tm).is_disjoint(&entities(&dm)));
        assert!(entities(&tm).is_disjoint(&entities(&xm)));
        assert!(entities(&dm).is_disjoint(&entities(&xm)));
        let mut prompts = HashMap::new();
        for e in train.iter().chain(&dev).chain(&transfer) {
            assert_eq!(resolve(&e.request).unwrap(), e.answer);
            let key = digest(&(
                e.request.system.clone(),
                e.request.input.clone(),
                e.request.evidence.clone(),
            ))
            .unwrap();
            assert!(
                prompts
                    .insert(key, e.answer.clone())
                    .is_none_or(|old| old == e.answer)
            );
            let mut reversed = e.request.clone();
            reversed.evidence.items.reverse();
            assert_eq!(resolve(&reversed).unwrap(), e.answer);
        }
        let mut wrong = train[0].request.clone();
        wrong.input.push_str("unsupported");
        assert!(resolve(&wrong).is_err());
        wrong = train[0].request.clone();
        wrong.evidence.items[0].original_excerpt.push('!');
        assert!(resolve(&wrong).is_err());
        let p = Plan {
            revision: REVISION.into(),
            source: String::new(),
            binary: String::new(),
            corpus: String::new(),
            transfer: String::new(),
            tokenizer: String::new(),
            initial: String::new(),
            initial_weights: String::new(),
            config: TrainConfig {
                seed: 29,
                ..Default::default()
            },
            architecture: Config::tiny(264),
            sampler: "bucket-base-permutation-v1".into(),
            order: (0..8)
                .map(|b| {
                    tm.iter()
                        .enumerate()
                        .filter_map(|(i, m)| (m.bucket == b).then_some(i))
                        .collect()
                })
                .collect(),
            train_order: String::new(),
            split_policy: String::new(),
            data_seed: 20260919,
            model_seed: 17,
            tiny: true,
            metadata: String::new(),
            evaluation: EvaluationPolicy::default(),
        };
        for epoch in 0..4 {
            let mut seen = BTreeSet::new();
            let mut last = [
                String::new(),
                String::new(),
                String::new(),
                String::new(),
                String::new(),
                String::new(),
                String::new(),
                String::new(),
            ];
            for step in epoch * 1024..(epoch + 1) * 1024 {
                let draw = p.draw(step);
                assert_eq!(draw.len(), 8);
                for (b, i) in draw.into_iter().enumerate() {
                    assert_eq!(tm[i].bucket, b);
                    assert!(seen.insert(i));
                    assert_ne!(last[b], tm[i].base);
                    last[b] = tm[i].base.clone();
                }
            }
            assert_eq!(seen.len(), 8192);
        }
        println!("SYNTHETIC_CORPUS_ONLY optimizer=0 examples=8832 epochs_checked=4");
    }
}
