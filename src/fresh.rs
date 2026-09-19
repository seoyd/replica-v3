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
    /// Register an explicit exposure/wording study from a verified completed parent.
    StudyPrepare {
        #[arg(long)]
        parent: PathBuf,
        #[arg(long)]
        output: PathBuf,
        /// Preserve the completed P wording policy and compare selector exposure.
        #[arg(long)]
        selector: bool,
    },
    /// Fixed same-weight parity and familiar-wording diagnostic, no optimizer calls.
    StudyObserve {
        #[arg(long)]
        root: PathBuf,
    },
    /// Pure read-only parent/C/P comparison; never creates missing observations.
    StudyReport {
        #[arg(long)]
        root: PathBuf,
        /// Read a closed study using its exact retained executable identity.
        /// This grants no permission to resume training or observations.
        #[arg(long)]
        frozen_executable: Option<PathBuf>,
    },
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
    let bytes = binary::to_storage_vec(v)?;
    replica_v3::codec::publish_new(p, |f, owned| {
        f.write_all(&bytes)?;
        if std::fs::read(owned)? != bytes {
            return Err(bad("publication readback"));
        }
        Ok(())
    })
}
fn pending_path(p: &Path) -> PathBuf {
    p.with_extension("pending.r3b")
}
fn publish_confirmed<T: Serialize>(p: &Path, v: &T) -> Result<()> {
    let pending = pending_path(p);
    #[cfg(feature = "test-support")]
    if p.file_name()
        .is_some_and(|n| n.to_string_lossy().ends_with("-finished.r3b"))
        && std::env::var("R3_FRESH_TEST_STOP").as_deref() == Ok("pending-write")
    {
        return Err(std::io::Error::other("injected pending write failure").into());
    }
    write(
        &pending,
        &binary::record!({"schema":2,"destination":p,"content":digest(v)?}),
    )?;
    #[cfg(feature = "test-support")]
    if p.file_name()
        .is_some_and(|n| n.to_string_lossy().ends_with("-finished.r3b"))
        && std::env::var("R3_FRESH_TEST_STOP").as_deref() == Ok("finished-pending-only")
    {
        return Err(std::io::Error::other("injected stop after durable pending").into());
    }
    write(p, v)?;
    // All required file/directory syncs completed before releasing the interlock.
    // A crash can resurrect this unlink and conservatively block, never approve a failed sync.
    std::fs::remove_file(pending)?;
    Ok(())
}
fn read_confirmed<T: serde::de::DeserializeOwned>(p: &Path) -> Result<T> {
    if pending_path(p).exists() {
        return Err(bad(
            "publication pending: optimizer/generation=0; retry blocked",
        ));
    }
    read(p)
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
fn verified_corpus(path: &Path, expected: &str) -> Result<data::native::Corpus> {
    let c = data::native::read(path)?;
    if hex(&c.physical) != expected {
        return Err(bad("owned native corpus differs from frozen bytes"));
    }
    Ok(c)
}
fn verified_metadata(root: &Path, p: &Plan) -> Result<(Vec<Meta>, Vec<Meta>, Vec<Meta>)> {
    let bytes = neural::read_bounded(&root.join("metadata.r3b"), 128 * 1024 * 1024)?;
    if neural::hash(&bytes) != p.metadata {
        return Err(bad("owned metadata differs from plan"));
    }
    Ok(binary::from_slice(&bytes)?)
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Plan {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    schema: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    fork: Option<Fork>,
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
struct Fork {
    study: PathBuf,
    study_hash: String,
    arm: String,
    parent_policy: String,
    parent_state: String,
    parent_adam: String,
    origin_step: usize,
    origin_input: u64,
    origin_target: u64,
    original_corpus: String,
    tokenizer_training_hash: String,
    variants: Option<String>,
    variant_metadata: Option<String>,
    alternate_first: Vec<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    selector: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    selector_metadata: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    flip_first: Vec<Option<bool>>,
    constant_lr: f64,
    target_limit: u64,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
struct EvaluationPolicy {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    teacher_steps: Vec<usize>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    fixture_per_bucket: Option<usize>,
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
            teacher_steps: vec![],
            fixture_per_bucket: None,
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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    source_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    query_context: Option<String>,
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
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
enum Intent {
    Full,
    Current,
    Previous,
    Restored,
    Cause,
}
fn phrases(i: Intent) -> [&'static str; 2] {
    match i {
        Intent::Full => [
            "기록에 적힌 문장을 빠짐없이 쓰고 인용하라.",
            "기록 원문을 온전히 옮기고 근거 번호를 덧붙여라.",
        ],
        Intent::Current => [
            "유효한 값을 답하고 기록 번호를 인용하라.",
            "지금의 값에 해당하는 근거를 함께 제시하라.",
        ],
        Intent::Previous => [
            "정정 이전의 값을 답하고 기록 번호를 인용하라.",
            "수정하기 이전 값과 그 근거 번호를 제시하라.",
        ],
        Intent::Restored => [
            "복원 이후 유효한 값을 기록 번호와 함께 답하라.",
            "되살린 기록의 현재 값과 근거 번호를 제시하라.",
        ],
        Intent::Cause => [
            "시간상 앞섰다는 이유만으로 원인을 결론 내릴 수 있는가?",
            "시간의 선후만으로 인과관계를 확정할 수 있는가?",
        ],
    }
}
fn question_intent(input: &str) -> Result<(&str, &str, Intent)> {
    let mut words = input.splitn(3, ' ');
    let e = words.next().ok_or_else(|| bad("question entity"))?;
    let c = words.next().ok_or_else(|| bad("question context"))?;
    let t = words.next().ok_or_else(|| bad("question task"))?;
    for (i, old) in [
        (
            Intent::Full,
            [
                "기록 원문 전체를 인용과 함께 써라.",
                "원문 전체와 인용을 답하라.",
                "해당 기록의 문장 전체를 근거 번호와 함께 알려줘.",
            ],
        ),
        (
            Intent::Current,
            [
                "현재 값을 인용과 함께 써라.",
                "현재 값과 인용을 답하라.",
                "지금 적용할 값을 근거 번호와 함께 알려줘.",
            ],
        ),
        (
            Intent::Previous,
            [
                "정정 전 과거 값을 인용과 함께 써라.",
                "이전 값을 인용해 답하라.",
                "바꾸기 전의 값을 근거 번호와 함께 알려줘.",
            ],
        ),
        (
            Intent::Restored,
            [
                "복원 후 현재 값을 인용과 함께 써라.",
                "복원된 값과 인용을 답하라.",
                "다시 복원한 뒤의 값을 근거 번호와 함께 알려줘.",
            ],
        ),
        (
            Intent::Cause,
            [
                "기록의 시간순서만으로 원인이 확정되는가?",
                "앞선 기록이 뒤 기록의 원인이라고 확정할 수 있는가?",
                "먼저 일어났다는 사실이 원인임을 입증하는가?",
            ],
        ),
    ] {
        if old.contains(&t) || phrases(i).contains(&t) {
            return Ok((e, c, i));
        }
    }
    Err(bad("unsupported question grammar"))
}
fn familiar(i: Intent) -> &'static str {
    match i {
        Intent::Full => "기록 원문 전체를 인용과 함께 써라.",
        Intent::Current => "현재 값을 인용과 함께 써라.",
        Intent::Previous => "정정 전 과거 값을 인용과 함께 써라.",
        Intent::Restored => "복원 후 현재 값을 인용과 함께 써라.",
        Intent::Cause => "기록의 시간순서만으로 원인이 확정되는가?",
    }
}
// Independent constrained resolver consumes serialized request only: no Episode
// binding, bucket, selected index, seed or expected answer is available here.
fn resolve(q: &ModelRequest) -> Result<String> {
    let (entity, context, intent) = question_intent(&q.input)?;
    let full = intent == Intent::Full;
    let past = intent == Intent::Previous;
    let cause = intent == Intent::Cause;
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
                    source_id: None,
                    query_context: None,
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
    #[cfg(feature = "test-support")]
    pub(super) fn is_tiny(&self) -> bool {
        self.tiny
    }
    pub(super) fn training_corpus(
        &self,
        path: &Path,
    ) -> Result<(data::CorpusManifest, Vec<Episode>, Vec<Episode>)> {
        let c = verified_corpus(path, &self.corpus)?;
        Ok((c.manifest, c.train, c.validation))
    }
    pub(super) fn source_identity(&self) -> &str {
        &self.source
    }
    pub(super) fn remaining_targets(&self, root: &Path) -> Result<Option<u64>> {
        let Some(f) = &self.fork else { return Ok(None) };
        let mut used = 0u64;
        // run() already validated history before publishing this command's start.
        for i in 0..128 {
            let end = root.join(format!("segment-{i:04}-finished.r3b"));
            if !end.exists() {
                break;
            }
            let _: Segment = read_confirmed(&end)?;
            let c: binary::Value = read(&root.join(format!("segment-{i:04}/train-control.r3b")))?;
            used = used
                .checked_add(
                    c["executed_target_tokens_including_uncommitted"]
                        .as_u64()
                        .ok_or_else(|| bad("target usage UNKNOWN"))?,
                )
                .ok_or_else(|| bad("target usage overflow"))?;
        }
        Ok(Some(
            f.target_limit
                .checked_sub(used)
                .ok_or_else(|| bad("target budget exhausted"))?,
        ))
    }
    pub(super) fn learning_rate(&self, step: usize) -> f64 {
        self.fork
            .as_ref()
            .map_or_else(|| self.config.learning_rate(step), |f| f.constant_lr)
    }
    pub(super) fn origin_step(&self) -> usize {
        self.fork.as_ref().map_or(0, |f| f.origin_step)
    }
    pub(super) fn additional_samples(&self, root: &Path, tok: &ByteBpe) -> Result<Vec<Sample>> {
        let Some(f) = &self.fork else {
            return Ok(vec![]);
        };
        if f.variants.is_none() {
            return Ok(vec![]);
        }
        let c = verified_corpus(&root.join("variants.r3cor"), f.variants.as_ref().unwrap())?;
        let mut out = samples(&c.train, tok, self.config.seq_len)?;
        if let Some(h) = &f.selector {
            let c = verified_corpus(&root.join("selectors.r3cor"), h)?;
            out.extend(samples(&c.train, tok, self.config.seq_len)?);
        }
        Ok(out)
    }
    pub(super) fn training_draw(&self, step: usize) -> Vec<usize> {
        let mut ids = self.draw(step);
        if let Some(f) = &self.fork
            && f.variants.is_some()
        {
            let n = self.order.iter().map(Vec::len).sum::<usize>();
            let epoch = (step - f.origin_step) / self.order[0].len();
            for i in &mut ids {
                let original = *i;
                if f.alternate_first[*i] ^ (epoch % 2 == 1) {
                    *i += n;
                }
                if f.selector.is_some()
                    && f.flip_first[original].is_some_and(|bit| bit ^ (epoch % 2 == 1))
                {
                    *i += 2 * n;
                }
            }
        }
        ids
    }
    pub(super) fn parent_entry(&self, path: &Path, l: &checkpoint::Loaded) -> Result<bool> {
        let Some(f) = &self.fork else {
            return Ok(false);
        };
        let Some(s) = &l.manifest.training else {
            return Err(bad("fork missing parent state"));
        };
        let own_policy = digest(self)?;
        if s.step != f.origin_step
            || s.resume_binding
                .as_ref()
                .is_some_and(|b| hex(&b.policy) == own_policy)
        {
            return Ok(false);
        }
        if file_hash(path)? != self.initial
            || digest(s)? != f.parent_state
            || optimizer_hash(&l.optimizer)? != f.parent_adam
            || l.model.weight_hash()? != self.initial_weights
            || l.tokenizer.train_hash != f.tokenizer_training_hash
            || s.resume_binding
                .as_ref()
                .is_none_or(|b| hex(&b.policy) != f.parent_policy)
        {
            return Err(bad("explicit parent fork binding mismatch"));
        }
        Ok(true)
    }
    pub(super) fn evaluation_due(&self, step: usize) -> bool {
        [
            &self.evaluation.screen_steps,
            &self.evaluation.primary_steps,
            &self.evaluation.transfer_steps,
            &self.evaluation.train_steps,
            &self.evaluation.teacher_steps,
        ]
        .iter()
        .any(|s| s.contains(&step))
    }
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
    #[cfg(feature = "test-support")]
    if tiny
        && matches!(
            std::env::var("R3_FRESH_FIXTURE_EOS").as_deref(),
            Ok("1" | "token")
        )
    {
        // Numerical tensor fixture only. The token is selected by real logits/greedy,
        // not by a decoder override; ordinary SMALL preparation cannot enter it.
        for (name, v) in &model.vars {
            let mut data = vec![
                if name.ends_with("norm") || name == "embedding" {
                    1f32
                } else {
                    0.
                };
                v.elem_count()
            ];
            if name == "embedding" {
                let h = model.config.hidden;
                let id = if std::env::var("R3_FRESH_FIXTURE_EOS").as_deref() == Ok("token") {
                    tok.encode(b"x")?[0] as usize
                } else {
                    EOS as usize
                };
                data[id * h..(id + 1) * h].fill(2.);
            }
            v.set(&Tensor::from_vec(data, v.dims(), &Device::Cpu)?)?;
        }
    }
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
        schema: Some(2),
        fork: None,
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
        evaluation: if tiny {
            EvaluationPolicy {
                screen_steps: vec![],
                primary_steps: vec![2],
                transfer_steps: vec![2],
                train_steps: vec![2],
                teacher_steps: vec![],
                fixture_per_bucket: Some(1),
                ..EvaluationPolicy::default()
            }
        } else {
            EvaluationPolicy {
                teacher_steps: (0..=4096).step_by(512).collect(),
                ..EvaluationPolicy::default()
            }
        },
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
        include_bytes!("quality_recovery.rs").as_slice(),
        include_bytes!("codec.rs").as_slice(),
        include_bytes!("binary.rs").as_slice(),
        include_bytes!("train_main.rs").as_slice(),
        include_bytes!("../Cargo.lock").as_slice(),
    ] {
        bytes.extend_from_slice(source);
    }
    Ok(neural::hash(&bytes))
}
pub fn execute(command: Command) -> Result<()> {
    match command {
        Command::StudyPrepare { parent, output, selector } => study_prepare(&parent, &output, selector),
        Command::StudyObserve { root } => study_observe(&root),
        Command::StudyReport {
            root,
            frozen_executable,
        } => study_report(&root, frozen_executable.as_deref()),
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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    schema: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    start_hash: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    control_hash: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    phase: Option<String>,
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
    plan_read_bound(
        root,
        &source_digest()?,
        &file_hash(&std::env::current_exe()?)?,
    )
}
fn plan_read_bound(root: &Path, source: &str, executable: &str) -> Result<Plan> {
    let p: Plan = read(&root.join("plan.r3b"))?;
    if p.revision != REVISION
        || p.source != source
        || p.binary != executable
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
    let expected = if let Some(f) = &p.fork {
        let study: Study = read(&f.study.join("study.r3b"))?;
        if file_hash(&f.study.join("study.r3b"))? != f.study_hash
            || study.source != p.source
            || study.binary != p.binary
            || study.parent_step != f.origin_step
            || study.parent_model != p.initial_weights
            || study.parent_file != p.initial
            || study.parent_adam != f.parent_adam
            || !study.tie_break.contains(&f.arm)
            || f.constant_lr.to_bits() != 3e-5f64.to_bits()
            || p.config.max_steps != f.origin_step + study.updates
            || p.config.budget_start_step != f.origin_step
            || p.config.budget_start_tokens != f.origin_input
            || p.config.warmup != 0
            || p.config.lr != f.constant_lr
            || p.config.first_target_weight != 1.
            || p.corpus != f.original_corpus
            || p.tokenizer != study.tokenizer
        {
            return Err(bad("unsupported explicit fork policy"));
        }
        if let Some(h) = &f.variants
            && &file_hash(&root.join("variants.r3cor"))? != h
        {
            return Err(bad("frozen question variants changed"));
        }
        if let Some(h) = &f.variant_metadata
            && &file_hash(&root.join("question-variants.r3b"))? != h
        {
            return Err(bad("frozen variant metadata changed"));
        }
        for (name, h) in [
            ("selectors.r3cor", &f.selector),
            ("selector-metadata.r3b", &f.selector_metadata),
        ] {
            if let Some(h) = h
                && &file_hash(&root.join(name))? != h
            {
                return Err(bad("frozen selector data changed"));
            }
        }
        let mut e = fork_evaluation(f.origin_step, study.updates, p.tiny);
        if study.schema == 3 {
            e.generation_limit = 4608;
        }
        e
    } else if p.tiny {
        EvaluationPolicy {
            screen_steps: vec![],
            primary_steps: vec![2],
            transfer_steps: vec![2],
            train_steps: vec![2],
            teacher_steps: vec![],
            fixture_per_bucket: Some(1),
            ..EvaluationPolicy::default()
        }
    } else {
        EvaluationPolicy {
            teacher_steps: (0..=4096).step_by(512).collect(),
            ..EvaluationPolicy::default()
        }
    };
    if p.schema != Some(2) || p.evaluation != expected {
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
        let start = root.join(format!("segment-{i:04}-started.r3b"));
        let s: Segment = read_confirmed(&root.join(format!("segment-{i:04}-finished.r3b")))
            .map_err(|_| bad("started segment without completion: usage UNKNOWN, retry blocked"))?;
        if s.policy != digest(p)? || file_hash(&root.join(&s.checkpoint))? != s.checkpoint_hash {
            return Err(bad("segment binding"));
        }
        let started: binary::Value = read(&start)?;
        let input = Path::new(
            started["checkpoint"]
                .as_str()
                .ok_or_else(|| bad("start checkpoint"))?,
        );
        let control_path = root.join(format!("segment-{i:04}/train-control.r3b"));
        let c: binary::Value = read(&control_path)?;
        if s.schema != Some(2)
            || s.start_hash.as_ref() != Some(&file_hash(&start)?)
            || s.control_hash.as_ref() != Some(&file_hash(&control_path)?)
            || started["policy"] != s.policy
            || started["physical"] != file_hash(input)?
            || c["checkpoint_saved"] != true
            || !c["save_error"].is_null()
        {
            return Err(bad("unconfirmed/failed segment; optimizer/generation=0"));
        }
        let (m, _) = checkpoint::metadata(&root.join(&s.checkpoint))?;
        let state = m
            .training
            .ok_or_else(|| bad("segment missing native state"))?;
        let expected_prior = out.last().map_or(p.initial.as_str(), |prior: &Segment| {
            prior.checkpoint_hash.as_str()
        });
        if started["physical"] != expected_prior
            || s.step != state.step
            || s.input != state.consumed_tokens
            || s.target != state.target_tokens
            || s.generations as u64
                != c["generation_calls"]
                    .as_u64()
                    .ok_or_else(|| bad("generation usage UNKNOWN"))?
            || s.teachers as u64
                != c["teacher_calls"]
                    .as_u64()
                    .ok_or_else(|| bad("teacher usage UNKNOWN"))?
            || !s.elapsed.is_finite()
            || s.elapsed < 0.
            || (s.resume && !matches!(s.stop.as_str(), "TIME_BUDGET" | "TRAINING"))
        {
            return Err(bad("segment native/counter/chain mismatch"));
        }
        if c["final_evaluation_complete"] == true && p.evaluation_due(s.step) {
            let (_, panels) = audit_panels_range(root, p, s.step, s.step)?;
            let native = checkpoint::load(&root.join(&s.checkpoint), Device::Cpu, false)?;
            let model = native.model.weight_hash()?;
            if panels.values().any(|panel| panel.model != model) {
                return Err(bad("completed panel differs from segment checkpoint"));
            }
        }
        if s.phase.as_deref() == Some("Finished")
            && p.evaluation_due(s.step)
            && c["final_evaluation_complete"] != true
        {
            return Err(bad("Finished with outstanding evaluation"));
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
    let (elapsed, generations, teachers) = if let Some(f) = &p.fork {
        study_usage(&f.study, true)?
    } else {
        (
            previous.iter().map(|s| s.elapsed).sum(),
            previous.iter().map(|s| s.generations).sum(),
            previous.iter().map(|s| s.teachers).sum(),
        )
    };
    if elapsed >= p.evaluation.active_seconds as f64
        || generations >= p.evaluation.generation_limit
        || teachers >= p.evaluation.teacher_limit
    {
        return Err(bad("total budget exhausted"));
    }
    let step = previous.last().map_or(p.origin_step(), |s| s.step);
    let input = root.join(
        previous
            .last()
            .map_or("initial.r3m", |s| s.checkpoint.as_str()),
    );
    let index = previous.len();
    let output = root.join(format!("segment-{index:04}"));
    let stop_after = if previous.is_empty() && !uninterrupted_fixture {
        p.origin_step() + 1
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
        &binary::record!({"schema":2,"policy":digest(&p)?,"checkpoint":input,"physical":file_hash(&input)?,"step":step,"prior_seconds":elapsed,"generation_limit":p.evaluation.generation_limit-generations,"teacher_limit":p.evaluation.teacher_limit-teachers}),
    )?;
    let r = if step == p.config.max_steps {
        // No trainer/Adam call on evaluation-only continuation, including final row publication.
        std::fs::create_dir(&output)?;
        let evaluated = evaluate_boundary(&p, root, &input, step, &mut control);
        if let Err(e) = &evaluated {
            control.classify_error(e);
        }
        let bytes = std::fs::read(&input)?;
        let saved = replica_v3::codec::publish_new(&output.join("final"), |f, _| {
            f.write_all(&bytes)?;
            Ok(())
        });
        if let Err(e) = &saved {
            control.classify_error(e);
        }
        let _ = control.seal_terminal();
        let mut receipt = control.receipt();
        receipt["reason"] = binary::record!(control.reason().unwrap_or("BUDGET_REACHED"));
        receipt["checkpoint_saved"] = binary::record!(saved.is_ok());
        receipt["save_error"] = binary::record!(saved.as_ref().err().map(ToString::to_string));
        receipt["work_error"] = binary::record!(evaluated.as_ref().err().map(ToString::to_string));
        receipt["fresh_stop"] = binary::record!(evaluated.as_ref().ok().cloned().flatten());
        receipt["final_evaluation_complete"] = binary::record!(evaluated.is_ok());
        receipt["optimizer_calls"] = binary::record!(0);
        receipt["executed_input_tokens_including_uncommitted"] = binary::record!(0);
        receipt["executed_target_tokens_including_uncommitted"] = binary::record!(0);
        receipt["executed_padding_tokens"] = binary::record!(0);
        write(&output.join("train-control.r3b"), &receipt)?;
        control.stop_result().and(evaluated.map(|_| ())).and(saved)
    } else {
        super::train_with_policy(
            Run {
                checkpoint: &input,
                corpus: Some(&root.join("corpus.r3cor")),
                output: &output,
                resume: !previous.is_empty() || p.fork.is_some(),
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
        )
    };
    let receipt: binary::Value = read(&output.join("train-control.r3b"))?;
    let path = output.join("final");
    let (m, _) = checkpoint::metadata(&path)?;
    let s = m.training.ok_or_else(|| bad("missing state"))?;
    let stop = receipt["fresh_stop"]
        .as_str()
        .or_else(|| receipt["reason"].as_str())
        .ok_or_else(|| bad("missing stop"))?
        .to_string();
    let evaluation_pending =
        p.evaluation_due(s.step) && receipt["final_evaluation_complete"] != true;
    let clean_time = matches!(&r,Err(Error::Model(message)) if message=="TIME_BUDGET")
        && receipt["reason"] == "TIME_BUDGET"
        && receipt["save_error"].is_null()
        && receipt["observed_conditions"]
            .as_array()
            .is_some_and(|v| !v.is_empty() && v.iter().all(|x| x == "TIME_BUDGET"));
    let resume = (s.step < p.config.max_steps || evaluation_pending)
        && s.consumed_tokens < p.config.max_tokens
        && receipt["fresh_stop"].is_null()
        && receipt["save_error"].is_null()
        && receipt["token_budget_reached"] != true
        && (r.is_ok() || clean_time);
    let segment = Segment {
        schema: Some(2),
        start_hash: Some(file_hash(
            &root.join(format!("segment-{index:04}-started.r3b")),
        )?),
        control_hash: Some(file_hash(&output.join("train-control.r3b"))?),
        phase: Some(
            if !resume && r.is_err() && !clean_time {
                "Failed"
            } else if evaluation_pending {
                "EvaluationPending"
            } else if resume {
                "TrainingPending"
            } else {
                "Finished"
            }
            .into(),
        ),
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
    publish_confirmed(
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
    if clean_time { Ok(()) } else { r }
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    teachers_completed: Option<usize>,
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
        teachers_completed: None,
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
        if let Some(context)=&m.query_context {
            let count=s.strata.entry(format!("query_context:{context}")).or_default();
            count[0]+=usize::from(exact);count[1]+=1;
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
    if row["row_version"] == 2 {
        if row["generation_completed"] == true && row["generation"].is_null() {
            return Err(bad("completed generation without Generated receipt"));
        }
        if row["generation_completed"] == true {
            let n = ids.len() - usize::from(ids.last() == Some(&EOS));
            if row["generation"]["tokens"] != binary::record!(&ids[..n])
                || row["generation"]["generated"] != ids.len()
                || row["generation"]["finish"] != row["finish_reason"]
            {
                return Err(bad("Generated/raw token mismatch"));
            }
            match tok.decode(&ids[..n]) {
                Ok(text) => {
                    if row["actual"] != text || !row["error"].is_null() {
                        return Err(bad("decoded text/error mismatch"));
                    }
                }
                Err(_) => {
                    if !row["actual"].is_null() || row["error"].is_null() {
                        return Err(bad("missing strict decode failure"));
                    }
                }
            }
        }
        let exact = row["generation_completed"] == true
            && row["error"].is_null()
            && row["finish_reason"] == "stop"
            && ids.last() == Some(&EOS)
            && row["actual"]
                .as_str()
                .is_some_and(|s| !s.is_empty() && row["expected"] == s);
        if row["exact_match"] != exact {
            return Err(bad("strict raw verdict"));
        }
    }
    if let Some(actual) = row["actual"].as_str() {
        let n = ids.len() - usize::from(ids.last() == Some(&EOS));
        if tok.decode(&ids[..n])? != actual {
            return Err(bad("raw text mismatch"));
        }
    }
    Ok(())
}
fn append_row(f: &mut std::fs::File, row: &binary::Value) -> Result<()> {
    binary::write_value_record(f, row)?;
    f.flush()?;
    f.sync_all()?;
    Ok(())
}
// Immutable prepared/resolved pairs: absence of a resolution is never evidence
// that a native call did not happen. Reuse is permitted only for proven no-call timeouts.
fn call_attempt(
    root: &Path,
    prefix: &str,
    kind: &str,
    binding: &binary::Value,
    e: &Episode,
    ordinal: usize,
    returned: Option<&binary::Value>,
) -> Result<Option<PathBuf>> {
    for attempt in 0..128 {
        let path = root.join(format!(
            "{prefix}-{kind}-{ordinal:04}-{attempt:03}-prepared.r3b"
        ));
        let identity = binary::record!({"schema":1,"binding":binding,"kind":kind,"case":digest(e)?,"ordinal":ordinal,"attempt":attempt});
        if !path.exists() {
            if returned.is_some() {
                return Err(bad("returned row without call resolution"));
            }
            return Ok(Some(path));
        }
        let old: binary::Value = read(&path)?;
        if old != identity {
            return Err(bad("call attempt identity mismatch"));
        }
        let resolved = path.with_file_name(
            path.file_name()
                .unwrap()
                .to_string_lossy()
                .replace("-prepared", "-resolved"),
        );
        if !resolved.exists() || pending_path(&resolved).exists() {
            return Err(bad(
                "entered or unreturned call usage UNKNOWN; retry blocked",
            ));
        }
        let r: binary::Value = read_confirmed(&resolved)?;
        if r["prepared"] != file_hash(&path)? {
            return Err(bad("call resolution binding"));
        }
        if r["state"] == "RETURNED" {
            let row = returned.ok_or_else(|| bad("returned call missing durable row"))?;
            if row["attempt"] != path.file_name().unwrap().to_string_lossy().as_ref()
                || r["row"] != digest(row)?
            {
                return Err(bad("returned call/raw mismatch"));
            }
            if root
                .join(format!(
                    "{prefix}-{kind}-{ordinal:04}-{:03}-prepared.r3b",
                    attempt + 1
                ))
                .exists()
            {
                return Err(bad("attempt after returned observation"));
            }
            return Ok(None);
        }
        if r["state"] != "NOT_INVOKED" || r["resumable"] != true || r["calls"] != 0 {
            return Err(bad("non-resumable call attempt"));
        }
    }
    Err(bad("call attempt limit"))
}
fn prepare_call(
    root: &Path,
    prefix: &str,
    kind: &str,
    binding: &binary::Value,
    e: &Episode,
    ordinal: usize,
) -> Result<PathBuf> {
    let path = call_attempt(root, prefix, kind, binding, e, ordinal, None)?
        .ok_or_else(|| bad("call already returned"))?;
    let name = path.file_name().unwrap().to_string_lossy();
    let attempt: usize = name
        .rsplit('-')
        .nth(1)
        .ok_or_else(|| bad("attempt name"))?
        .parse()
        .map_err(|_| bad("attempt number"))?;
    write(
        &path,
        &binary::record!({"schema":1,"binding":binding,"kind":kind,"case":digest(e)?,"ordinal":ordinal,"attempt":attempt}),
    )?;
    Ok(path)
}
fn resolve_call(
    path: &Path,
    row: Option<&binary::Value>,
    control: &recovery::RunControl,
) -> Result<()> {
    let c = control.receipt();
    let pure_time = c["observed_conditions"] == binary::record!(["TIME_BUDGET"]);
    let dest = path.with_file_name(
        path.file_name()
            .unwrap()
            .to_string_lossy()
            .replace("-prepared", "-resolved"),
    );
    let value = binary::record!({"prepared":file_hash(path)?,"state":if row.is_some(){"RETURNED"}else{"NOT_INVOKED"},"row":row.map(digest).transpose()?,"calls":usize::from(row.is_some()),"resumable":row.is_none()&&pure_time,"control":c});
    #[cfg(feature = "test-support")]
    if std::env::var("R3_FRESH_RESOLUTION_FAIL").as_deref() == Ok("1") {
        write(&pending_path(&dest), &value)?;
        return Err(std::io::Error::other("injected resolution sync failure").into());
    }
    publish_confirmed(&dest, &value)
}
#[cfg(feature="test-support")]
fn call_fixture(
    l: &checkpoint::Loaded,
    control: &mut recovery::RunControl,
    prefix: &str,
    kind: &str,
    ordinal: usize,
) {
    if l.model.config.hidden == 32 && l.model.config.layers == 2 {
        control.fixture_boundary = std::env::var("R3_FRESH_CALL_STOP").ok().and_then(|s| {
            s.strip_prefix(&format!("{prefix}/{kind}/{ordinal}/"))
                .map(str::to_owned)
        });
        if let Some(tokens) = control
            .fixture_boundary
            .as_deref()
            .and_then(|s| s.strip_prefix("native-timeout-"))
            .and_then(|s| s.parse().ok())
        {
            control.fixture_native_timeout(tokens);
        }
    }
}
fn teacher_prefix(
    root: &Path,
    prefix: &str,
    binding: &binary::Value,
    l: &checkpoint::Loaded,
    episodes: &[Episode],
    control: &mut recovery::RunControl,
) -> Result<Vec<binary::Value>> {
    let raw = root.join(format!("{prefix}-teachers.r3rows"));
    let pending = root.join(format!("{prefix}-teacher-pending.r3b"));
    if pending.exists() {
        return Err(bad("unreturned teacher usage UNKNOWN; retry blocked"));
    }
    let mut entries = if raw.exists() {
        binary::read_value_records(&raw)?
    } else {
        vec![]
    };
    if entries.first().is_some_and(|v| v != binding) {
        return Err(bad("teacher binding"));
    }
    let rows = if entries.is_empty() {
        vec![]
    } else {
        entries.drain(1..).collect::<Vec<_>>()
    };
    if rows.len() > episodes.len() {
        return Err(bad("teacher cursor beyond completed prefix"));
    }
    for (i, (r, e)) in rows.iter().zip(episodes).enumerate() {
        if binding["call_protocol"] == 1 || !r["attempt"].is_null() {
            call_attempt(root, prefix, "teacher", binding, e, i, Some(r))?;
        }
        if r["ordinal"] != i
            || r["id"] != e.id
            || r["case"] != digest(e)?
            || r["teacher"]["mean_nll"]
                .as_f64()
                .is_none_or(|v| !v.is_finite())
            || r["teacher"]["target_tokens_including_eos"]
                .as_u64()
                .is_none_or(|n| n == 0)
        {
            return Err(bad("teacher prefix integrity"));
        }
    }
    let mut f = std::fs::OpenOptions::new()
        .create_new(!raw.exists())
        .append(true)
        .open(&raw)?;
    if entries.is_empty() {
        append_row(&mut f, binding)?;
        std::fs::File::open(root)?.sync_all()?;
    }
    let mut rows = rows;
    for (i, e) in episodes.iter().enumerate().skip(rows.len()) {
        control.check("fresh_teacher_next")?;
        let attempt = prepare_call(root, prefix, "teacher", binding, e, i)?;
        #[cfg(feature = "test-support")]
        call_fixture(l, control, prefix, "teacher", i);
        let t = match recovery::fresh_teacher(l, e, control) {
            recovery::ObservedCall::NotInvoked(result) => {
                if let Err(ref error) = result {
                    control.classify_error(error);
                }
                resolve_call(&attempt, None, control)?;
                result?;
                return Err(bad("teacher not invoked without failure"));
            }
            recovery::ObservedCall::Returned(result) => result,
        };
        let value = match &t {
            Ok(v) => v.clone(),
            Err(error) => binary::record!({"error":error.to_string()}),
        };
        let row = binary::record!({"ordinal":i,"id":e.id,"case":digest(e)?,"teacher":value,"attempt":attempt.file_name().unwrap().to_string_lossy()});
        append_row(&mut f, &row)?;
        resolve_call(&attempt, Some(&row), control)?;
        t?;
        rows.push(row);
        control.check("fresh_teacher_durable")?;
    }
    Ok(rows)
}
fn teacher_ce(rows: &[binary::Value]) -> Result<f64> {
    let mut sum = 0.;
    let mut n = 0u64;
    for r in rows {
        let t = &r["teacher"];
        let k = t["target_tokens_including_eos"]
            .as_u64()
            .ok_or_else(|| bad("teacher count"))?;
        sum += t["mean_nll"].as_f64().ok_or_else(|| bad("teacher NLL"))? * k as f64;
        n += k;
    }
    if n == 0 || !sum.is_finite() {
        return Err(bad("incomplete teacher CE"));
    }
    Ok(sum / n as f64)
}
#[allow(clippy::too_many_arguments)] // One shared evaluator; caller owns the fixed cases and metadata.
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
    let binding = binary::record!({"call_protocol":1,"policy":digest(p)?,"step":step,"model":model,"dataset":dataset,"tokenizer":l.tokenizer.id(),"panel":name,"planned":episodes.len(),"decoding":"normal-greedy-strict-utf8-eos"});
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
    for (i, row) in rows.iter().enumerate() {
        if binding["call_protocol"] == 1 || !row["attempt"].is_null() {
            call_attempt(
                root,
                &prefix,
                "generation",
                &binding,
                &episodes[i],
                i,
                Some(row),
            )?;
        }
        verify_generated(row, &l.tokenizer)?;
    }
    score(&rows, &episodes[..rows.len()], &meta[..rows.len()])?;
    if pending_path(&summary).exists() {
        return Err(bad("panel publication unconfirmed"));
    }
    let mut f = std::fs::OpenOptions::new()
        .create_new(!raw.exists())
        .append(true)
        .open(&raw)?;
    if existing.is_empty() {
        append_row(&mut f, &header)?;
        std::fs::File::open(root)?.sync_all()?;
    }
    let mut rows = rows;
    let pending = root.join(format!("{prefix}-pending.r3b"));
    if pending.exists() {
        return Err(bad("unreturned generation usage UNKNOWN; do not retry"));
    }
    let mut teachers = teacher_prefix(
        root,
        &prefix,
        &binding,
        &l,
        &episodes[..rows.len()],
        control,
    )?;
    for e in episodes.iter().skip(rows.len()) {
        control.check("fresh_panel_next")?;
        let attempt = prepare_call(root, &prefix, "generation", &binding, e, rows.len())?;
        #[cfg(feature = "test-support")]
        call_fixture(&l, control, &prefix, "generation", rows.len());
        #[cfg(feature = "test-support")]
        {
            control.fixture_post_generation_deadline = p.tiny
                && step == p.config.max_steps
                && name == "train64"
                && std::env::var("R3_FRESH_TEST_STOP")
                    .is_ok_and(|v| v == format!("final-row-{}", rows.len() + 1));
            control.fixture_post_generation_deadline |= p.tiny
                && step == p.config.max_steps
                && std::env::var("R3_FRESH_TEST_STOP")
                    .is_ok_and(|v| v == format!("final-{name}-row-{}", rows.len() + 1));
        }
        let mut row = match recovery::observe_generation(&l, e, &e.request, control, false) {
            recovery::ObservedCall::Returned(row) => row,
            recovery::ObservedCall::NotInvoked(row) => {
                resolve_call(&attempt, None, control)?;
                control.stop_result()?;
                return Err(bad(&format!("generation not invoked: {}", row["error"])));
            }
        };
        row["attempt"] = binary::record!(attempt.file_name().unwrap().to_string_lossy());
        append_row(&mut f, &row)?;
        resolve_call(&attempt, Some(&row), control)?;
        verify_generated(&row, &l.tokenizer)?;
        rows.push(row);
        control.check("fresh_panel_row_durable")?;
        teachers = teacher_prefix(
            root,
            &prefix,
            &binding,
            &l,
            &episodes[..rows.len()],
            control,
        )?;
    }
    let mut s = score(&rows, episodes, meta)?;
    s.step = step;
    s.panel = name.into();
    s.model = model;
    s.dataset = dataset;
    s.raw_hash = file_hash(&raw)?;
    s.ce = Some(teacher_ce(&teachers)?);
    s.teachers_completed = Some(teachers.len());
    #[cfg(feature = "test-support")]
    if p.tiny
        && step == p.config.max_steps
        && std::env::var("R3_FRESH_TEST_STOP").is_ok_and(|v| v == format!("final-{name}-summary"))
    {
        control.fixture_deadline();
        control.check("fresh_summary_pending")?;
    }
    if summary.exists() {
        let old: PanelResult = read_confirmed(&summary)?;
        if old != s {
            return Err(bad("evaluation summary disagreement"));
        }
        return Ok(s);
    }
    publish_confirmed(&summary, &s)?;
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
    if !p.evaluation_due(step) {
        return Ok(None);
    }
    let (_, train, dev) = p.training_corpus(&root.join("corpus.r3cor"))?;
    let transfer = verified_corpus(&root.join("transfer.r3cor"), &p.transfer)?.validation;
    let (tm, dm, xm) = verified_metadata(root, p)?;
    let subset = |es: &[Episode], ms: &[Meta]| {
        let ids: Vec<_> = (0..8)
            .flat_map(|b| {
                ms.iter()
                    .enumerate()
                    .filter(move |(_, m)| m.bucket == b)
                    .take(p.evaluation.fixture_per_bucket.unwrap_or(8))
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
    if p.evaluation.teacher_steps.contains(&step) {
        let l = checkpoint::load(path, Device::Cpu, false)?;
        let teacher = root.join(format!("teacher-{step:04}.r3b"));
        let binding = binary::record!({"call_protocol":1,"policy":digest(p)?,"step":step,"model":l.model.weight_hash()?,"dataset":digest(&probe)?});
        if teacher.exists() {
            let old: binary::Value = read_confirmed(&teacher)?;
            if old["binding"] != binding {
                return Err(bad("fixed teacher binding"));
            }
        } else {
            let (loss, calls, reused) = if p.evaluation.train_steps.contains(&step) {
                let panel: PanelResult =
                    read_confirmed(&root.join(format!("eval-{step:04}-train64.r3b")))?;
                (
                    panel
                        .ce
                        .ok_or_else(|| bad("required train teacher incomplete"))?,
                    0,
                    true,
                )
            } else {
                let ts = teacher_prefix(
                    root,
                    &format!("teacher-{step:04}"),
                    &binding,
                    &l,
                    &probe,
                    control,
                )?;
                (teacher_ce(&ts)?, ts.len(), false)
            };
            publish_confirmed(
                &teacher,
                &binary::record!({"binding":binding,"step":step,"model":l.model.weight_hash()?,"panel":"train64-fixed","dataset":digest(&probe)?,"ce":loss,"teacher_calls":calls,"reused_complete_train_panel":reused}),
            )?;
        }
    }
    let mut full = None;
    if p.evaluation.primary_steps.contains(&step) {
        let (dev, dm) = if p.tiny {
            subset(&dev, &dm)
        } else {
            (dev.clone(), dm.clone())
        };
        full = Some(evaluate_panel(
            p, root, path, step, "dev512", &dev, &dm, control,
        )?);
    }
    if p.fork.is_some()
        && let Some(full) = &full
    {
        let prefix = format!("eval-{step:04}-dev512");
        let rows = binary::read_value_records(&root.join(format!("{prefix}.r3rows")))?;
        let tr = binary::read_value_records(&root.join(format!("{prefix}-teachers.r3rows")))?;
        let (es, ms) = subset(&dev, &dm);
        let mut selected = vec![];
        let mut ts = vec![];
        for e in &es {
            let n = rows[1..]
                .iter()
                .position(|r| r["id"] == e.id)
                .ok_or_else(|| bad("screen subset missing"))?;
            selected.push(rows[n + 1].clone());
            ts.push(tr[n + 1].clone());
        }
        let mut r = score(&selected, &es, &ms)?;
        r.step = step;
        r.panel = "screen64".into();
        r.model = full.model.clone();
        r.dataset = digest(&es)?;
        r.raw_hash = full.raw_hash.clone();
        r.ce = Some(teacher_ce(&ts)?);
        r.teachers_completed = Some(ts.len());
        let dest = root.join(format!("eval-{step:04}-screen64.r3b"));
        if dest.exists() {
            let old: PanelResult = read_confirmed(&dest)?;
            if old != r {
                return Err(bad("reused screen summary mismatch"));
            }
        } else {
            publish_confirmed(&dest, &r)?;
        }
        screen = Some(r);
    }
    if p.evaluation.transfer_steps.contains(&step) {
        let (transfer, xm) = if p.tiny {
            subset(&transfer, &xm)
        } else {
            (transfer.clone(), xm.clone())
        };
        let transfer = evaluate_panel(p, root, path, step, "transfer128", &transfer, &xm, control)?;
        let f = full.as_ref().unwrap();
        if step == p.config.max_steps
            && p.fork
                .as_ref()
                .is_some_and(|f| matches!(f.arm.as_str(), "C-KEEP" | "S-SELECT"))
        {
            let (es, ms) = selector_panel(&dev, &dm, p.tiny)?;
            evaluate_panel(p, root, path, step, "selector192", &es, &ms, control)?;
        }
        if (p.fork.is_none() || step == p.config.max_steps)
            && f.exact >= p.evaluation.primary_min
            && f.buckets.iter().all(|&x| x >= p.evaluation.bucket_min)
            && f.errors == 0
            && transfer.exact >= p.evaluation.transfer_min
            && transfer.errors == 0
        {
            return Ok(Some("DEVELOPMENT_PASS_FINAL_NOT_RUN".into()));
        }
    }
    if let Some(f) = &p.fork {
        if matches!(f.arm.as_str(), "C-KEEP" | "S-SELECT")
            && !p.tiny
            && full
                .as_ref()
                .is_some_and(|panel| panel.exact + 64 <= 425 || panel.errors >= 8)
        {
            return Ok(Some("QUALITY_REGRESSION_PRIMARY".into()));
        }
        if let Some(current) = screen {
            let parent: binary::Value = read(&f.study.join("parent-audit.r3b"))?;
            let baseline: PanelResult = binary::from_value(
                parent["panels"][format!("eval-{:04}-screen64", f.origin_step).as_str()].clone(),
            )
            .or_else(|_| {
                binary::from_value(
                    parent["panels"][format!("eval-{:04}-dev512", f.origin_step).as_str()].clone(),
                )
            })?;
            let mut best = baseline.exact;
            let mut ce = baseline.ce;
            let mut streak = 0;
            let mut steps = p.evaluation.screen_steps.clone();
            steps.extend(&p.evaluation.primary_steps);
            steps.sort();
            steps.dedup();
            for n in steps.into_iter().filter(|&n| n <= step) {
                let r: PanelResult =
                    read_confirmed(&root.join(format!("eval-{n:04}-screen64.r3b")))?;
                if r.exact + 24 <= best || r.errors >= 8 {
                    return Ok(Some("QUALITY_REGRESSION_SEVERE".into()));
                }
                if let Some((a, b)) = r.ce.zip(ce) {
                    if r.exact + 12 <= best && a >= b * 1.2 {
                        streak += 1;
                    } else {
                        streak = 0;
                    }
                }
                if r.exact > best {
                    best = r.exact;
                    ce = r.ce;
                }
            }
            if streak >= 2 {
                return Ok(Some("QUALITY_REGRESSION".into()));
            }
            println!(
                "FORK_SCREEN step={step} exact={}/{} errors={} complete_teacher={:?}",
                current.exact, current.total, current.errors, current.teachers_completed
            );
        }
        return Ok(None);
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
            h.last().map_or(0, |s| s.step - p.origin_step())
        },
        h.iter().map(|s| s.generations).sum::<usize>(),
        h.iter().map(|s| s.teachers).sum::<usize>(),
        h.iter().map(|s| s.elapsed).sum::<f64>()
    );
    Ok(())
}
fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}
fn optimizer_hash(m: &BTreeMap<String, Tensor>) -> Result<String> {
    use sha2::{Digest, Sha256};
    let mut h = Sha256::new();
    for (name, t) in m {
        h.update(name.as_bytes());
        for x in t.flatten_all()?.to_vec1::<f32>()? {
            h.update(x.to_bits().to_le_bytes());
        }
    }
    Ok(hex(&h.finalize()))
}
fn subset(es: &[Episode], ms: &[Meta], per_bucket: usize) -> (Vec<Episode>, Vec<Meta>) {
    let ids: Vec<_> = (0..8)
        .flat_map(|b| {
            ms.iter()
                .enumerate()
                .filter(move |(_, m)| m.bucket == b)
                .take(per_bucket)
                .map(|(i, _)| i)
        })
        .collect();
    (
        ids.iter().map(|&i| es[i].clone()).collect(),
        ids.iter().map(|&i| ms[i].clone()).collect(),
    )
}
fn inventory(root: &Path) -> Result<Vec<(PathBuf, String, u64)>> {
    fn walk(root: &Path, p: &Path, out: &mut Vec<(PathBuf, String, u64)>) -> Result<()> {
        for e in std::fs::read_dir(p)? {
            let e = e?;
            if e.file_type()?.is_dir() {
                walk(root, &e.path(), out)?;
            } else {
                out.push((
                    e.path()
                        .strip_prefix(root)
                        .map_err(|_| bad("inventory path"))?
                        .into(),
                    file_hash(&e.path())?,
                    e.metadata()?.len(),
                ));
            }
        }
        Ok(())
    }
    let mut out = vec![];
    walk(root, root, &mut out)?;
    out.sort_by(|a, b| a.0.cmp(&b.0));
    Ok(out)
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Study {
    schema: u32,
    source: String,
    binary: String,
    parent: PathBuf,
    parent_plan: String,
    parent_checkpoint: PathBuf,
    parent_file: String,
    parent_model: String,
    parent_adam: String,
    parent_state: String,
    parent_step: usize,
    tokenizer: String,
    updates: usize,
    tiny: bool,
    inventory: Vec<(PathBuf, String, u64)>,
    diagnostic_hash: String,
    parent_audit_hash: String,
    tie_break: Vec<String>,
}
fn fork_evaluation(origin: usize, updates: usize, tiny: bool) -> EvaluationPolicy {
    let offsets = if tiny { vec![] } else { vec![256, 512, 1536] };
    let endpoints = if tiny {
        vec![origin + updates]
    } else {
        vec![origin + 1024, origin + updates]
    };
    let mut teacher_steps: Vec<_> = offsets
        .iter()
        .map(|n| origin + n)
        .chain(endpoints.iter().copied())
        .collect();
    teacher_steps.sort();
    EvaluationPolicy {
        screen_steps: offsets.into_iter().map(|n| origin + n).collect(),
        primary_steps: endpoints.clone(),
        transfer_steps: endpoints.clone(),
        train_steps: endpoints.clone(),
        teacher_steps,
        fixture_per_bucket: tiny.then_some(1),
        active_seconds: 14400,
        ..EvaluationPolicy::default()
    }
}
// Pure reader for both explicitly historical and current producer records. This
// never relaxes plan_read's current-source check used to authorize learning.
fn audit_panels(root: &Path, p: &Plan) -> Result<(usize, BTreeMap<String, PanelResult>)> {
    audit_panels_through(root, p, p.config.max_steps)
}
fn audit_panels_through(
    root: &Path,
    p: &Plan,
    last: usize,
) -> Result<(usize, BTreeMap<String, PanelResult>)> {
    audit_panels_range(root, p, 0, last)
}
#[allow(clippy::too_many_arguments)] // One existing panel identity and owned expected cases.
fn audit_panel(
    root: &Path,
    p: &Plan,
    step: usize,
    name: &str,
    es: &[Episode],
    ms: &[Meta],
    tok: &ByteBpe,
) -> Result<PanelResult> {
    let key = format!("eval-{step:04}-{name}");
    let raw = root.join(format!("{key}.r3rows"));
    let rows = binary::read_value_records(&raw)?;
    let header = rows.first().ok_or_else(|| bad("empty raw"))?;
    let cp = Path::new(
        header["checkpoint"]
            .as_str()
            .ok_or_else(|| bad("raw checkpoint"))?,
    );
    let l = checkpoint::load(cp, Device::Cpu, false)?;
    let state = l
        .manifest
        .training
        .as_ref()
        .ok_or_else(|| bad("raw state"))?;
    if header["physical"] != file_hash(cp)?
        || header["binding"]["policy"] != digest(p)?
        || header["binding"]["dataset"] != digest(&es)?
        || header["binding"]["step"] != step
        || header["binding"]["planned"] != es.len()
        || header["binding"]["model"] != l.model.weight_hash()?
        || state.step != step
        || state.resume_binding.as_ref() != Some(&p.binding(state, tok)?)
    {
        return Err(bad("raw lineage/dataset mismatch"));
    }
    for (i, (r, e)) in rows[1..].iter().zip(es).enumerate() {
        if header["binding"]["call_protocol"] == 1 || !r["attempt"].is_null() {
            call_attempt(root, &key, "generation", &header["binding"], e, i, Some(r))?;
        }
        verify_generated(r, tok)?;
        let prompt = tok.prepare(
            &e.request,
            l.model.config.context as u32,
            &l.model.config.id()?,
        )?;
        if r["native_prompt_digest"] != prompt.token_digest {
            return Err(bad("raw actual framing mismatch"));
        }
    }
    let mut s = score(&rows[1..], es, ms)?;
    s.step = step;
    s.panel = name.into();
    s.model = l.model.weight_hash()?;
    s.dataset = digest(&es)?;
    s.raw_hash = file_hash(&raw)?;
    let teacher_rows = if p.schema == Some(2) {
        let t = binary::read_value_records(&root.join(format!("{key}-teachers.r3rows")))?;
        if t.len() != es.len() + 1 || t[0] != header["binding"] {
            return Err(bad("teacher complete binding"));
        }
        for (i, (r, e)) in t[1..].iter().zip(es).enumerate() {
            if header["binding"]["call_protocol"] == 1 || !r["attempt"].is_null() {
                call_attempt(root, &key, "teacher", &header["binding"], e, i, Some(r))?;
            }
            if r["id"] != e.id || r["ordinal"] != i || r["case"] != digest(e)? {
                return Err(bad("teacher case"));
            }
        }
        s.teachers_completed = Some(es.len());
        t[1..].to_vec()
    } else {
        rows[1..]
            .iter()
            .map(|r| binary::record!({"teacher":r["teacher_forced_diagnostic_after_generation"]}))
            .collect()
    };
    s.ce = Some(teacher_ce(&teacher_rows)?);
    let saved: PanelResult = if p.schema == Some(2) {
        read_confirmed(&root.join(format!("{key}.r3b")))?
    } else {
        read(&root.join(format!("{key}.r3b")))?
    };
    if s != saved {
        return Err(bad("raw independently recomputed summary mismatch"));
    }

    Ok(s)
}
fn audit_panels_range(
    root: &Path,
    p: &Plan,
    first: usize,
    last: usize,
) -> Result<(usize, BTreeMap<String, PanelResult>)> {
    let (_, tr, dv) = p.training_corpus(&root.join("corpus.r3cor"))?;
    let tx = verified_corpus(&root.join("transfer.r3cor"), &p.transfer)?.validation;
    let (tm, dm, xm) = verified_metadata(root, p)?;
    let tok = ByteBpe::load(&root.join("tokenizer.r3b"))?;
    if tok.id() != p.tokenizer {
        return Err(bad("parent tokenizer"));
    }
    let mut count = 0;
    let mut results = BTreeMap::new();
    for (steps, name, es, ms, sub) in [
        (&p.evaluation.train_steps, "train64", &tr, &tm, true),
        (&p.evaluation.screen_steps, "screen64", &dv, &dm, true),
        (&p.evaluation.primary_steps, "dev512", &dv, &dm, p.tiny),
        (
            &p.evaluation.transfer_steps,
            "transfer128",
            &tx,
            &xm,
            p.tiny,
        ),
    ] {
        let (es, ms) = if sub {
            subset(es, ms, p.evaluation.fixture_per_bucket.unwrap_or(8))
        } else {
            (es.clone(), ms.clone())
        };
        for &step in steps.iter().filter(|&&step| step >= first && step <= last) {
            let key = format!("eval-{step:04}-{name}");
            let s = audit_panel(root, p, step, name, &es, &ms, &tok)?;
            count += es.len();
            results.insert(key, s);
        }
    }
    if p.schema == Some(2) {
        for &step in p
            .evaluation
            .teacher_steps
            .iter()
            .filter(|&&s| s >= first && s <= last)
        {
            let key = format!("teacher-{step:04}");
            let summary: binary::Value = read_confirmed(&root.join(format!("{key}.r3b")))?;
            let (probe, _) = subset(&tr, &tm, p.evaluation.fixture_per_bucket.unwrap_or(8));
            if summary["binding"]["policy"] != digest(p)?
                || summary["binding"]["step"] != step
                || summary["binding"]["dataset"] != digest(&probe)?
            {
                return Err(bad("teacher obligation binding"));
            }
            if results
                .values()
                .filter(|r| r.step == step)
                .any(|r| summary["binding"]["model"] != r.model)
            {
                return Err(bad("fixed teacher differs from panel model"));
            }
            let ce = if p.evaluation.train_steps.contains(&step) {
                results
                    .get(&format!("eval-{step:04}-train64"))
                    .and_then(|s| s.ce)
                    .ok_or_else(|| bad("missing required panel teacher"))?
            } else {
                let ts = binary::read_value_records(&root.join(format!("{key}-teachers.r3rows")))?;
                if ts.len() != probe.len() + 1 || ts[0] != summary["binding"] {
                    return Err(bad("incomplete fixed teacher"));
                }
                for (i, (r, e)) in ts[1..].iter().zip(&probe).enumerate() {
                    if r["ordinal"] != i || r["id"] != e.id || r["case"] != digest(e)? {
                        return Err(bad("fixed teacher case"));
                    }
                    if ts[0]["call_protocol"] == 1 || !r["attempt"].is_null() {
                        call_attempt(root, &key, "teacher", &ts[0], e, i, Some(r))?;
                    }
                }
                teacher_ce(&ts[1..])?
            };
            if summary["ce"] != binary::record!(ce) {
                return Err(bad("required teacher summary"));
            }
        }
    }
    if first <= p.config.max_steps
        && last >= p.config.max_steps
        && p.fork
            .as_ref()
            .is_some_and(|f| matches!(f.arm.as_str(), "C-KEEP" | "S-SELECT"))
    {
        let (es, ms) = selector_panel(&dv, &dm, p.tiny)?;
        let s = audit_panel(root, p, p.config.max_steps, "selector192", &es, &ms, &tok)?;
        count += es.len();
        results.insert(format!("eval-{:04}-selector192", p.config.max_steps), s);
    }
    Ok((count, results))
}
fn changed_question(e: &Episode, suffix: &str) -> Result<Episode> {
    let (entity, context, intent) = question_intent(&e.request.input)?;
    let mut out = e.clone();
    out.request.input = format!("{entity} {context} {suffix}");
    if question_intent(&out.request.input)?.2 != intent
        || resolve(&e.request)? != e.answer
        || resolve(&out.request)? != e.answer
    {
        return Err(bad("question-only meaning changed"));
    }
    let mut check = out.clone();
    check.request.input = e.request.input.clone();
    if digest(&check)? != digest(e)? {
        return Err(bad("non-question change"));
    }
    Ok(out)
}
fn validate_framed(es: &[Episode], tok: &ByteBpe) -> Result<()> {
    let seq = samples(es, tok, 512)?;
    let mut seen = HashMap::new();
    for (e, s) in es.iter().zip(seq) {
        let p = tok.prepare(&e.request, 2048, "input-check")?;
        if s.tokens.len() > 513
            || p.provided.len() != e.request.evidence.items.len()
            || !p.excluded.is_empty()
        {
            return Err(bad("question length/evidence"));
        }
        if seen
            .insert(s.tokens[..s.response_start].to_vec(), e.answer.clone())
            .is_some_and(|a| a != e.answer)
        {
            return Err(bad("conflicting framed targets"));
        }
    }
    Ok(())
}
fn flip_selection(e: &Episode, m: &Meta) -> Result<(Episode, Meta)> {
    if !(2..=4).contains(&m.bucket) {
        return Err(bad("selector bucket"));
    }
    let mut out = e.clone();
    let mut meta = m.clone();
    let (entity, context, intent) = question_intent(&e.request.input)?;
    if intent != Intent::Current || e.request.evidence.items.len() != 2 {
        return Err(bad("selector grammar/count"));
    }
    let items = &e.request.evidence.items;
    if items[0].event_id == items[1].event_id
        || items.iter().any(|r| {
            r.version_status != "current"
                || r.observed_at.is_none()
                || r.observed_at.unwrap() > r.recorded_at
        })
    {
        return Err(bad("selector status/id/time"));
    }
    let (a, b) = (parsed_record(&items[0])?, parsed_record(&items[1])?);
    let suffix = e
        .request
        .input
        .splitn(3, ' ')
        .nth(2)
        .ok_or_else(|| bad("selector suffix"))?;
    match m.bucket {
        2 => {
            if a.0 == b.0 || a.1 != b.1 || a.1 != context {
                return Err(bad("entity selector fields"));
            }
            let other = if a.0 == entity {
                b.0
            } else if b.0 == entity {
                a.0
            } else {
                return Err(bad("query entity absent"));
            };
            out.request.input = format!("{other} {context} {suffix}");
        }
        3 => {
            if a.0 != b.0 || a.0 != entity || a.1 == b.1 {
                return Err(bad("context selector fields"));
            }
            let other = if a.1 == context {
                b.1
            } else if b.1 == context {
                a.1
            } else {
                return Err(bad("query context absent"));
            };
            out.request.input = format!("{entity} {other} {suffix}");
        }
        4 => {
            if a.0 != entity
                || b.0 != entity
                || a.1 != context
                || b.1 != context
                || items[0].observed_at == items[1].observed_at
            {
                return Err(bad("time selector fields"));
            }
            out.request.evidence.items[0].observed_at = items[1].observed_at;
            out.request.evidence.items[1].observed_at = items[0].observed_at;
            if out
                .request
                .evidence
                .items
                .iter()
                .any(|r| r.observed_at.unwrap() > r.recorded_at)
            {
                return Err(bad("swapped observation in future"));
            }
        }
        _ => unreachable!(),
    }
    // Request-only resolver supplies targets. An independent field/time selection
    // verifies the chosen citation; original expected is never a selector input.
    out.answer = resolve(&out.request)?;
    let (q, c, _) = question_intent(&out.request.input)?;
    let mut matching = out
        .request
        .evidence
        .items
        .iter()
        .filter(|r| parsed_record(r).is_ok_and(|(n, ctx, _)| n == q && ctx == c))
        .collect::<Vec<_>>();
    matching.sort_by_key(|r| r.observed_at);
    let selected = matching.last().ok_or_else(|| bad("no selected evidence"))?;
    let expected = format!(
        "{}입니다. [event:{}]",
        parsed_record(selected)?.2,
        selected.event_id
    );
    if out.answer != expected
        || citations(&out.answer)? == citations(&resolve(&e.request)?)?
        || resolve(&e.request)? != e.answer
    {
        return Err(bad("independent selector label"));
    }
    out.id = format!("{}/selector", e.id);
    meta.source_id = Some(e.id.clone());
    meta.id = out.id.clone();
    meta.query_context = Some(c.into());
    meta.entities[0] = q.into();
    Ok((out, meta))
}
fn selector_panel(es: &[Episode], ms: &[Meta], tiny: bool) -> Result<(Vec<Episode>, Vec<Meta>)> {
    let mut out = vec![];
    let mut metadata = vec![];
    for (e, m) in es
        .iter()
        .zip(ms)
        .filter(|(_, m)| (2..=4).contains(&m.bucket))
    {
        if tiny && m.view != 0 {
            continue;
        }
        let (f, fm) = flip_selection(e, m)?;
        let (reverse, _) = flip_selection(&f, &fm)?;
        if digest(&reverse.request)? != digest(&e.request)? || reverse.answer != e.answer {
            return Err(bad("selector involution"));
        }
        out.push(f);
        metadata.push(fm);
    }
    Ok((out, metadata))
}
fn study_prepare(parent: &Path, output: &Path, selector: bool) -> Result<()> {
    let started = Instant::now();
    let parent = parent.canonicalize()?;
    let p: Plan = read(&parent.join("plan.r3b"))?;
    for (name, h) in [
        ("corpus.r3cor", &p.corpus),
        ("transfer.r3cor", &p.transfer),
        ("metadata.r3b", &p.metadata),
        ("initial.r3m", &p.initial),
    ] {
        if &file_hash(&parent.join(name))? != h {
            return Err(bad("parent frozen input mismatch"));
        }
    }
    if selector
        && p.fork
            .as_ref()
            .is_none_or(|f| f.arm != "P-PHRASE" || f.variants.is_none())
    {
        return Err(bad(
            "selector study requires the preserved P phrase endpoint",
        ));
    }
    if !selector && p.fork.is_some() {
        return Err(bad(
            "study requires original joint parent, not another fork",
        ));
    }
    let mut last = None;
    for i in 0..128 {
        let started = parent.join(format!("segment-{i:04}-started.r3b"));
        if !started.exists() {
            break;
        }
        let path = parent.join(format!("segment-{i:04}-finished.r3b"));
        let s: Segment = if p.schema == Some(2) {
            read_confirmed(&path)?
        } else {
            read(&path)?
        };
        if s.policy != digest(&p)? || s.checkpoint_hash != file_hash(&parent.join(&s.checkpoint))? {
            return Err(bad("historical segment binding"));
        }
        last = Some(s);
    }
    let end = last.ok_or_else(|| bad("missing completed parent"))?;
    if end.resume || end.step != p.config.max_steps || end.stop != "BUDGET_REACHED" {
        return Err(bad("parent is not a closed endpoint"));
    }
    let cp = parent.join(&end.checkpoint);
    let l = checkpoint::load(&cp, Device::Cpu, true)?;
    let s = l
        .manifest
        .training
        .clone()
        .ok_or_else(|| bad("parent Adam state"))?;
    if s.step != end.step
        || s.sampler_state != s.step as u64
        || s.config != p.config
        || s.resume_binding.as_ref() != Some(&p.binding(&s, &l.tokenizer)?)
        || l.tokenizer.id() != p.tokenizer
        || s.config.first_target_weight != 1.
    {
        return Err(bad("parent objective/sampler/tokenizer"));
    }
    let inv = inventory(&parent)?;
    let (raw_count, mut panels) = if selector {
        audit_panels_range(&parent, &p, s.step, s.step)?
    } else {
        audit_panels(&parent, &p)?
    };
    if selector {
        let full = &panels[&format!("eval-{:04}-dev512", s.step)];
        let old: PanelResult =
            read_confirmed(&parent.join(format!("eval-{:04}-screen64.r3b", s.step)))?;
        if old.model != full.model || old.raw_hash != full.raw_hash {
            return Err(bad("parent screen model/raw"));
        }
        panels.insert(format!("eval-{:04}-screen64", s.step), old);
    }
    let eval = checkpoint::load(
        &parent
            .join(end.checkpoint)
            .parent()
            .unwrap()
            .join(format!("step-{:06}", s.step)),
        Device::Cpu,
        true,
    )?;
    if eval.model.weight_hash()? != l.model.weight_hash()?
        || optimizer_hash(&eval.optimizer)? != optimizer_hash(&l.optimizer)?
        || eval.manifest.training != l.manifest.training
    {
        return Err(bad("parent final/evaluated checkpoint differ"));
    }
    let (manifest, train, dev) = data::load(&parent.join("corpus.r3cor"))?;
    let (_, _, transfer) = data::load(&parent.join("transfer.r3cor"))?;
    let (tm, dm, xm): (Vec<Meta>, Vec<Meta>, Vec<Meta>) = read(&parent.join("metadata.r3b"))?;
    let (diagnostic_source, diagnostic_meta) = if p.tiny {
        subset(&transfer, &xm, 1)
    } else {
        (transfer.clone(), xm.clone())
    };
    let mut diagnostic = diagnostic_source
        .iter()
        .map(|e| {
            let i = question_intent(&e.request.input)?.2;
            changed_question(e, familiar(i))
        })
        .collect::<Result<Vec<_>>>()?;
    let held_suffixes: std::collections::BTreeSet<_> = transfer
        .iter()
        .map(|e| e.request.input.splitn(3, ' ').nth(2).unwrap().to_owned())
        .collect();
    let mut diagnostic_meta = diagnostic_meta;
    if selector {
        (diagnostic, diagnostic_meta) = selector_panel(&dev, &dm, p.tiny)?;
    }
    let variants = if selector {
        let f = p.fork.as_ref().unwrap();
        if f.variant_metadata.as_ref() != Some(&file_hash(&parent.join("question-variants.r3b"))?) {
            return Err(bad("parent phrase metadata"));
        }
        verified_corpus(&parent.join("variants.r3cor"), f.variants.as_ref().unwrap())?.train
    } else {
        train
            .iter()
            .zip(&tm)
            .map(|(e, m)| {
                let i = question_intent(&e.request.input)?.2;
                let h = u64::from_str_radix(&neural::hash(m.base.as_bytes())[..8], 16)
                    .map_err(|_| bad("base digest"))?;
                let suffix = phrases(i)[h as usize % 2];
                if held_suffixes.contains(suffix) {
                    return Err(bad("heldout suffix copied into train"));
                }
                changed_question(e, suffix)
            })
            .collect::<Result<Vec<_>>>()?
    };
    if variants.len() != train.len() {
        return Err(bad("unaligned P variants"));
    }
    validate_framed(
        &train.iter().chain(&variants).cloned().collect::<Vec<_>>(),
        &l.tokenizer,
    )?;
    validate_framed(&diagnostic, &l.tokenizer)?;
    let mut alternate = vec![false; train.len()];
    for b in 0..8 {
        let mut bases: Vec<_> = tm
            .iter()
            .filter(|m| m.bucket == b)
            .map(|m| m.base.clone())
            .collect();
        bases.sort();
        bases.dedup();
        bases.sort_by_key(|base| neural::hash(format!("29/{base}").as_bytes()));
        let chosen: std::collections::BTreeSet<_> = bases.iter().take(bases.len() / 2).collect();
        for (i, m) in tm.iter().enumerate().filter(|(_, m)| m.bucket == b) {
            alternate[i] = chosen.contains(&m.base);
        }
    }
    if selector {
        alternate = p.fork.as_ref().unwrap().alternate_first.clone();
    }
    let mut flip_first = vec![None; train.len()];
    if selector {
        for bucket in 2..=4 {
            for style in [false, true] {
                let mut bases = tm
                    .iter()
                    .enumerate()
                    .filter(|(i, m)| m.bucket == bucket && alternate[*i] == style)
                    .map(|(_, m)| m.base.clone())
                    .collect::<Vec<_>>();
                bases.sort();
                bases.dedup();
                bases.sort_by_key(|b| neural::hash(format!("selector/29/{b}").as_bytes()));
                let half = bases[..bases.len() / 2]
                    .iter()
                    .collect::<std::collections::BTreeSet<_>>();
                for (i, m) in tm
                    .iter()
                    .enumerate()
                    .filter(|(i, m)| m.bucket == bucket && alternate[*i] == style)
                {
                    flip_first[i] = Some(half.contains(&m.base));
                }
            }
        }
    }
    let mut selector_rows = vec![];
    let mut selector_meta = vec![];
    if selector {
        for (style, es) in [&train, &variants].into_iter().enumerate() {
            for (e, m) in es.iter().zip(&tm) {
                let (mut row, mut meta) = if (2..=4).contains(&m.bucket) {
                    let pair = flip_selection(e, m)?;
                    let (back, _) = flip_selection(&pair.0, &pair.1)?;
                    if digest(&back.request)? != digest(&e.request)? || back.answer != e.answer {
                        return Err(bad("train selector involution"));
                    }
                    pair
                } else {
                    (e.clone(), m.clone())
                };
                row.id = format!("{}/selector-style-{style}", e.id);
                meta.id = row.id.clone();
                meta.source_id = Some(e.id.clone());
                selector_rows.push(row);
                selector_meta.push(meta);
            }
        }
        validate_framed(
            &train
                .iter()
                .chain(&variants)
                .chain(&selector_rows)
                .cloned()
                .collect::<Vec<_>>(),
            &l.tokenizer,
        )?;
    }
    std::fs::create_dir(output)?;
    let output = output.canonicalize()?;
    let peer = if selector && !p.tiny {
        let path = parent
            .parent()
            .ok_or_else(|| bad("parent study directory"))?
            .join("C-REPEAT");
        let plan: Plan = read(&path.join("plan.r3b"))?;
        if plan.corpus != p.corpus
            || plan.transfer != p.transfer
            || plan.tokenizer != p.tokenizer
            || plan.config.max_steps != s.step
        {
            return Err(bad("historical C/P endpoint mismatch"));
        }
        let before = inventory(&path)?;
        let (count, result) = audit_panels_range(&path, &plan, s.step, s.step)?;
        if before != inventory(&path)? {
            return Err(bad("historical peer changed"));
        }
        Some(binary::record!({"path":path,"raw_count":count,"panels":result,"inventory":before}))
    } else {
        None
    };
    write(
        &output.join("parent-audit.r3b"),
        &binary::record!({"raw_count":raw_count,"panels":panels,"peer":peer,"parent_policy":digest(&p)?,"historical_receipts":"READ_ONLY_VERIFIED_NOT_REISSUED"}),
    )?;
    write(
        &output.join("diagnostic.r3b"),
        &(diagnostic.clone(), diagnostic_meta),
    )?;
    let study = Study {
        schema: if selector { 3 } else { 2 },
        source: source_digest()?,
        binary: file_hash(&std::env::current_exe()?)?,
        parent: parent.clone(),
        parent_plan: file_hash(&parent.join("plan.r3b"))?,
        parent_checkpoint: cp.clone(),
        parent_file: file_hash(&cp)?,
        parent_model: l.model.weight_hash()?,
        parent_adam: optimizer_hash(&l.optimizer)?,
        parent_state: digest(&s)?,
        parent_step: s.step,
        tokenizer: p.tokenizer.clone(),
        updates: if p.tiny { 2 } else { 2048 },
        tiny: p.tiny,
        inventory: inv,
        diagnostic_hash: file_hash(&output.join("diagnostic.r3b"))?,
        parent_audit_hash: file_hash(&output.join("parent-audit.r3b"))?,
        tie_break: if selector {
            vec!["C-KEEP".into(), "S-SELECT".into()]
        } else {
            vec!["C-REPEAT".into(), "P-PHRASE".into()]
        },
    };
    write(&output.join("study.r3b"), &study)?;
    for arm in study.tie_break.iter().map(String::as_str) {
        let root = output.join(arm);
        std::fs::create_dir(&root)?;
        for name in [
            "corpus.r3cor",
            "transfer.r3cor",
            "metadata.r3b",
            "tokenizer.r3b",
        ] {
            let bytes = std::fs::read(parent.join(name))?;
            neural::write_new(&root.join(name), &bytes)?;
        }
        neural::write_new(&root.join("initial.r3m"), &std::fs::read(&cp)?)?;
        let variant_hash = if arm == "P-PHRASE" || selector {
            let c = data::native::from_episodes(manifest.clone(), variants.clone(), dev.clone())?;
            data::native::write(&root.join("variants.r3cor"), &c, true)?;
            let aligned=variants.iter().zip(&tm).enumerate().map(|(n,(e,m))|->Result<binary::Value>{let(_,_,i)=question_intent(&e.request.input)?;
                Ok(binary::record!({"source_episode_id":e.id,"base":m.base,"intent":i,"variant_id":phrases(i).iter().position(|t|e.request.input.ends_with(t)).ok_or_else(||bad("variant id"))?,"alternate_first_epoch":alternate[n]}))}).collect::<Result<Vec<_>>>()?;
            write(&root.join("question-variants.r3b"), &aligned)?;
            Some(file_hash(&root.join("variants.r3cor"))?)
        } else {
            None
        };
        let variant_metadata = if variant_hash.is_some() {
            Some(file_hash(&root.join("question-variants.r3b"))?)
        } else {
            None
        };
        let (selector_hash, selector_metadata) = if arm == "S-SELECT" {
            let c =
                data::native::from_episodes(manifest.clone(), selector_rows.clone(), dev.clone())?;
            data::native::write(&root.join("selectors.r3cor"), &c, true)?;
            let owned = data::native::read(&root.join("selectors.r3cor"))?;
            for e in &owned.train {
                if resolve(&e.request)? != e.answer {
                    return Err(bad("native selector label verification"));
                }
            }
            write(&root.join("selector-metadata.r3b"), &selector_meta)?;
            (
                Some(file_hash(&root.join("selectors.r3cor"))?),
                Some(file_hash(&root.join("selector-metadata.r3b"))?),
            )
        } else {
            (None, None)
        };
        let mut plan = p.clone();
        plan.schema = Some(2);
        plan.source = study.source.clone();
        plan.binary = study.binary.clone();
        plan.initial = study.parent_file.clone();
        plan.initial_weights = study.parent_model.clone();
        plan.config.budget_start_step = s.step;
        plan.config.budget_start_tokens = s.consumed_tokens;
        plan.config.max_steps = s.step + study.updates;
        plan.config.max_tokens = s.consumed_tokens + 9_000_000;
        plan.config.lr = 3e-5;
        plan.config.warmup = 0;
        plan.config.validate_every = if p.tiny { 2 } else { 256 };
        plan.evaluation = fork_evaluation(s.step, study.updates, p.tiny);
        if selector {
            plan.evaluation.generation_limit = 4608;
        }
        plan.fork = Some(Fork {
            study: output.clone(),
            study_hash: file_hash(&output.join("study.r3b"))?,
            arm: arm.into(),
            parent_policy: digest(&p)?,
            parent_state: digest(&s)?,
            parent_adam: study.parent_adam.clone(),
            origin_step: s.step,
            origin_input: s.consumed_tokens,
            origin_target: s.target_tokens,
            original_corpus: p.corpus.clone(),
            tokenizer_training_hash: l.tokenizer.train_hash.clone(),
            variants: variant_hash,
            variant_metadata,
            alternate_first: alternate.clone(),
            selector: selector_hash,
            selector_metadata,
            flip_first: if selector { flip_first.clone() } else { vec![] },
            constant_lr: 3e-5,
            target_limit: 1_000_000,
        });
        let original = samples(&train, &l.tokenizer, 512)?;
        let variant = samples(&variants, &l.tokenizer, 512)?;
        let mut all = original.clone();
        all.extend(plan.additional_samples(&root, &l.tokenizer)?);
        let mut counts = vec![[0usize; 2]; train.len()];
        let mut epoch_counts = [[[0usize; 2]; 8]; 2];
        let mut flips = 0usize;
        let mut flip_counts = vec![[0usize; 2]; train.len()];
        let mut strata: BTreeMap<(usize, usize, bool, usize), [usize; 2]> = BTreeMap::new();
        let mut input = 0usize;
        let mut targets = 0usize;
        for step in s.step..plan.config.max_steps {
            let ids = plan.draw(step);
            let draw = plan.training_draw(step);
            for (bucket, (&i, &j)) in ids.iter().zip(&draw).enumerate() {
                let selected = (j / train.len()) % 2;
                let flipped = usize::from(j >= 2 * train.len());
                flips += flipped;
                flip_counts[i][flipped] += 1;
                if (2..=4).contains(&bucket) {
                    strata
                        .entry((
                            (step - s.step) / plan.order[0].len(),
                            bucket,
                            selected == 1,
                            tm[i].view,
                        ))
                        .or_default()[flipped] += 1;
                }
                counts[i][selected] += 1;
                epoch_counts[(step - s.step) / plan.order[0].len()][bucket][selected] += 1;
                let a = &original[i];
                let b = &variant[i];
                if a.tokens[a.response_start..] != b.tokens[b.response_start..] {
                    return Err(bad("variant changed target tokens"));
                }
                let row = &all[j];
                input += row.tokens.len() - 1;
                targets += row.tokens.len() - row.response_start;
            }
        }
        if !p.tiny
            && counts.iter().any(|c| {
                *c != if arm == "P-PHRASE" || selector {
                    [1, 1]
                } else {
                    [2, 0]
                }
            })
        {
            return Err(bad("two-epoch exposure contract"));
        }
        if !p.tiny
            && epoch_counts.iter().flatten().any(|c| {
                *c != if arm == "P-PHRASE" || selector {
                    [512, 512]
                } else {
                    [1024, 0]
                }
            })
        {
            return Err(bad("per-bucket epoch wording balance"));
        }
        if !p.tiny
            && arm == "S-SELECT"
            && (flips != 3072
                || strata.values().any(|x| x[0] != x[1])
                || flip_counts.iter().zip(&tm).any(|(c, m)| {
                    *c != if (2..=4).contains(&m.bucket) {
                        [1, 1]
                    } else {
                        [2, 0]
                    }
                }))
        {
            return Err(bad("selector balanced exposure contract"));
        }
        if input > 9_000_000 || targets > 1_000_000 {
            return Err(bad("registered token budget insufficient"));
        }
        write(&root.join("plan.r3b"), &plan)?;
        write(
            &root.join("registration.r3b"),
            &binary::record!({"policy":digest(&plan)?,"counts":counts,"epoch_variant_counts":epoch_counts,"selector_draws":flips,"selector_counts":flip_counts,"selector_strata":strata.into_iter().collect::<Vec<_>>(),"planned_input":input,"planned_target":targets,"origin_step":s.step,"end_step":plan.config.max_steps,"lr_bits":3e-5f64.to_bits(),"optimizer_calls":0}),
        )?;
        println!(
            "FORK_REGISTERED arm={arm} origin={} end={} planned_input={input} planned_target={targets} parent={} optimizer=0",
            s.step, plan.config.max_steps, study.parent_model
        );
    }
    if inventory(&parent)? != study.inventory {
        return Err(bad("parent changed during read-only registration"));
    }
    publish_confirmed(
        &output.join("study-ready.r3b"),
        &binary::record!({"study":file_hash(&output.join("study.r3b"))?,"C":file_hash(&output.join(&study.tie_break[0]).join("plan.r3b"))?,"P":file_hash(&output.join(&study.tie_break[1]).join("plan.r3b"))?,"elapsed_seconds":started.elapsed().as_secs_f64()}),
    )?;
    println!(
        "PARENT_RAW_RESCORE={raw_count} VERIFIED initial_weights_Adam=SHARED new_updates=0 diagnostic_planned={}",
        diagnostic.len()
    );
    Ok(())
}
fn study_read(root: &Path) -> Result<Study> {
    study_read_bound(root, None)
}
fn study_read_bound(root: &Path, frozen_executable: Option<&Path>) -> Result<Study> {
    let s: Study = read(&root.join("study.r3b"))?;
    let names = match s.schema {
        2 => ["C-REPEAT", "P-PHRASE"],
        3 => ["C-KEEP", "S-SELECT"],
        _ => return Err(bad("study schema")),
    };
    if s.tie_break != names {
        return Err(bad("study arm identity"));
    }
    let ready: binary::Value = read_confirmed(&root.join("study-ready.r3b"))?;
    if ready["study"] != file_hash(&root.join("study.r3b"))?
        || ready["C"] != file_hash(&root.join(&s.tie_break[0]).join("plan.r3b"))?
        || ready["P"] != file_hash(&root.join(&s.tie_break[1]).join("plan.r3b"))?
    {
        return Err(bad("study registration incomplete/changed"));
    }
    let executable = frozen_executable.map_or_else(std::env::current_exe, |p| Ok(p.to_path_buf()))?;
    if ![2, 3].contains(&s.schema)
        || (frozen_executable.is_none() && s.source != source_digest()?)
        || s.binary != file_hash(&executable)?
        || s.parent_plan != file_hash(&s.parent.join("plan.r3b"))?
        || s.parent_file != file_hash(&s.parent_checkpoint)?
        || s.diagnostic_hash != file_hash(&root.join("diagnostic.r3b"))?
        || s.parent_audit_hash != file_hash(&root.join("parent-audit.r3b"))?
    {
        return Err(bad("frozen study binding"));
    }
    Ok(s)
}
fn paired(a: &[binary::Value], b: &[binary::Value]) -> Result<binary::Value> {
    if a.len() != b.len() {
        return Err(bad("paired denominator"));
    }
    let (mut gain, mut loss) = (0, 0);
    let mut body = [0usize; 2];
    let mut citation = [0usize; 2];
    for (a, b) in a.iter().zip(b) {
        if a["id"] != b["id"]
            || a["expected"] != b["expected"]
            || a["generated_evidence"] != b["generated_evidence"]
        {
            return Err(bad("paired source changed"));
        }
        gain += usize::from(a["exact_match"] != true && b["exact_match"] == true);
        loss += usize::from(a["exact_match"] == true && b["exact_match"] != true);
        let body_ok = |r: &binary::Value| {
            r["actual"]
                .as_str()
                .zip(r["expected"].as_str())
                .is_some_and(|(x, y)| x.split(" [event:").next() == y.split(" [event:").next())
        };
        let citation_ok = |r: &binary::Value| {
            r["actual"]
                .as_str()
                .zip(r["expected"].as_str())
                .is_some_and(|(x, y)| citations(x).ok() == citations(y).ok())
        };
        body[0] += usize::from(!body_ok(a) && body_ok(b));
        body[1] += usize::from(body_ok(a) && !body_ok(b));
        citation[0] += usize::from(!citation_ok(a) && citation_ok(b));
        citation[1] += usize::from(citation_ok(a) && !citation_ok(b));
    }
    Ok(
        binary::record!({"paired":a.len(),"gain":gain,"loss":loss,"body_gain_loss":body,"citation_gain_loss":citation}),
    )
}
fn selector_pairs(
    original: &[binary::Value],
    flipped: &[binary::Value],
    es: &[Episode],
    ms: &[Meta],
) -> Result<binary::Value> {
    if flipped.len() != es.len() || es.len() != ms.len() {
        return Err(bad("selector pair denominator"));
    }
    let mut outcomes = [0usize; 4];
    let mut same = 0;
    let mut citation = [0usize; 4];
    let mut by_bucket: BTreeMap<usize, [usize; 4]> = BTreeMap::new();
    let mut base: BTreeMap<&str, (usize, usize)> = BTreeMap::new();
    let mut original_full = 0;
    let mut flip_full = 0;
    let mut original_body = 0;
    let mut flip_body = 0;
    let mut original_citation = 0;
    for ((row, e), m) in flipped.iter().zip(es).zip(ms) {
        let source = m
            .source_id
            .as_ref()
            .ok_or_else(|| bad("selector source case"))?;
        let old = original
            .iter()
            .find(|r| r["id"] == *source)
            .ok_or_else(|| bad("selector original missing"))?;
        if row["id"] != e.id || row["expected"] != e.answer {
            return Err(bad("selector paired content"));
        }
        let a = old["exact_match"] == true;
        let b = row["exact_match"] == true;
        let index = match (a, b) {
            (true, true) => 0,
            (true, false) => 1,
            (false, true) => 2,
            (false, false) => 3,
        };
        outcomes[index] += 1;
        by_bucket.entry(m.bucket).or_default()[index] += 1;
        original_full += usize::from(a);
        flip_full += usize::from(b);
        same += usize::from(old["actual"].is_string() && old["actual"] == row["actual"]);
        let fields = |v: &binary::Value| {
            v.as_str()
                .unwrap_or("")
                .split_once(" [event:")
                .map(|(b, _)| b.to_owned())
        };
        original_body += usize::from(
            fields(&old["actual"]).is_some() && fields(&old["actual"]) == fields(&old["expected"]),
        );
        flip_body += usize::from(
            fields(&row["actual"]).is_some() && fields(&row["actual"]) == fields(&row["expected"]),
        );
        original_citation += usize::from(
            old["actual"].as_str().and_then(|s| citations(s).ok())
                == old["expected"].as_str().and_then(|s| citations(s).ok()),
        );
        let got = row["actual"]
            .as_str()
            .and_then(|s| citations(s).ok())
            .unwrap_or_default();
        let expected = citations(&e.answer)?;
        let c = if got.is_empty() {
            3
        } else if got == expected {
            0
        } else if got
            .iter()
            .all(|id| e.request.evidence.items.iter().any(|r| r.event_id == *id))
        {
            1
        } else {
            2
        };
        citation[c] += 1;
        let x = base.entry(&m.base).or_default();
        x.0 += 1;
        x.1 += usize::from(a && b);
    }
    Ok(
        binary::record!({"planned":es.len(),"pair_order":["both","original_only","flipped_only","neither"],"pairs":outcomes,"bucket_pairs":by_bucket,"same_output":same,
        "original_full":original_full,"flipped_full":flip_full,"original_body":original_body,"flipped_body":flip_body,"original_citation":original_citation,
        "flipped_citation_order":["selected","other_provided","invented","none"],"flipped_citations":citation,"both_base4":base.values().filter(|&&(n,k)|n==4&&k==4).count()}),
    )
}
fn fragments(tok: &ByteBpe, es: &[Episode]) -> Result<binary::Value> {
    let ids: std::collections::BTreeSet<_> = (0..tok.vocab_size() as u32)
        .filter(|&i| {
            tok.decode_bytes(&[i])
                .is_ok_and(|b| std::str::from_utf8(&b).is_err())
        })
        .collect();
    let mut prompt = 0;
    let mut answer = 0;
    let mut pt = 0;
    let mut at = 0;
    for e in es {
        let p = tok.prepare(&e.request, 2048, "wording-diagnostic")?;
        let a = tok.encode(e.answer.as_bytes())?;
        pt += p.token_ids.len();
        at += a.len();
        prompt += p.token_ids.iter().filter(|i| ids.contains(i)).count();
        answer += a.iter().filter(|i| ids.contains(i)).count();
    }
    Ok(
        binary::record!({"examples":es.len(),"prompt_tokens":pt,"answer_tokens":at,"fragment_prompt":prompt,"fragment_answer":answer,"fragment_pieces":ids.len()}),
    )
}
fn study_observe(root: &Path) -> Result<()> {
    let s = study_read(root)?;
    if root.join("observation-started.r3b").exists() {
        return Err(bad("observation already attempted; no automatic retry"));
    }
    let p: Plan = read(&s.parent.join("plan.r3b"))?;
    let (_, train, dev) = data::load(&s.parent.join("corpus.r3cor"))?;
    let (_, _, transfer) = data::load(&s.parent.join("transfer.r3cor"))?;
    let (_, dm, _): (Vec<Meta>, Vec<Meta>, Vec<Meta>) = read(&s.parent.join("metadata.r3b"))?;
    let (diagnostic, xm): (Vec<Episode>, Vec<Meta>) = read(&root.join("diagnostic.r3b"))?;
    let tok = ByteBpe::load(&s.parent.join("tokenizer.r3b"))?;
    let (probe, pm) = subset(&dev, &dm, if p.tiny { 1 } else { 2 });
    let mut control = recovery::RunControl::command(false)?;
    control.set_call_limits(if s.schema == 3 { 208 } else { 192 }, 6000);
    write(
        &root.join("observation-started.r3b"),
        &binary::record!({"study":file_hash(&root.join("study.r3b"))?,"parity_ids":probe.iter().map(|e|&e.id).collect::<Vec<_>>(),"generation_limit":if s.schema==3{208}else{192},"teacher_policy":"required per returned case; separate durable records"}),
    )?;
    let result = (|| -> Result<binary::Value> {
        let parity = evaluate_panel(
            &p,
            root,
            &s.parent_checkpoint,
            s.parent_step,
            "parity",
            &probe,
            &pm,
            &mut control,
        )?;
        let original = binary::read_value_records(
            &s.parent
                .join(format!("eval-{:04}-dev512.r3rows", s.parent_step)),
        )?;
        let current = binary::read_value_records(
            &root.join(format!("eval-{:04}-parity.r3rows", s.parent_step)),
        )?;
        for row in &current[1..] {
            let old = original[1..]
                .iter()
                .find(|r| r["id"] == row["id"])
                .ok_or_else(|| bad("parity missing case"))?;
            for field in [
                "raw_tokens",
                "actual",
                "error",
                "finish_reason",
                "eos_index",
            ] {
                if old[field] != row[field] {
                    return Err(bad("same-weight output parity changed"));
                }
            }
        }
        let diagnostic_name = if s.schema == 3 {
            "selector192"
        } else {
            "familiar-wording"
        };
        let diagnostic_result = evaluate_panel(
            &p,
            root,
            &s.parent_checkpoint,
            s.parent_step,
            diagnostic_name,
            &diagnostic,
            &xm,
            &mut control,
        )?;
        let orig = binary::read_value_records(&s.parent.join(format!(
            "eval-{:04}-{}.r3rows",
            s.parent_step,
            if s.schema == 3 {
                "dev512"
            } else {
                "transfer128"
            }
        )))?;
        let altered = binary::read_value_records(&root.join(format!(
            "eval-{:04}-{diagnostic_name}.r3rows",
            s.parent_step
        )))?;
        let comparison = if s.schema == 3 {
            selector_pairs(&orig[1..], &altered[1..], &diagnostic, &xm)?
        } else {
            paired(&orig[1..], &altered[1..])?
        };
        Ok(
            binary::record!({"parity":parity,"diagnostic_label":if s.schema==3{"SELECTOR_DEVELOPMENT_DIAGNOSTIC"}else{"FAMILIAR_WORDING_DIAGNOSTIC_NOT_PRODUCT_SCORE"},"diagnostic":diagnostic_result,"paired":comparison,
            "fragments":{"train":fragments(&tok,&train)?,"primary":fragments(&tok,&dev)?,"transfer":fragments(&tok,&transfer)?,"familiar":fragments(&tok,&diagnostic)?}}),
        )
    })();
    if let Err(e) = &result {
        control.classify_error(e);
    }
    let _ = control.seal_terminal();
    let record = binary::record!({"study":file_hash(&root.join("study.r3b"))?,"start":file_hash(&root.join("observation-started.r3b"))?,"result":result.as_ref().ok(),"error":result.as_ref().err().map(ToString::to_string),"control":control.receipt()});
    publish_confirmed(&root.join("observation-finished.r3b"), &record)?;
    println!("STUDY_OBSERVATION {record}");
    if inventory(&s.parent)? != s.inventory {
        return Err(bad("parent changed during observation"));
    }
    control.stop_result().and(result.map(|_| ()))
}
fn study_usage(root: &Path, require_observation: bool) -> Result<(f64, usize, usize)> {
    let s = study_read(root)?;
    study_usage_bound(root, require_observation, &s)
}
fn study_usage_bound(
    root: &Path,
    require_observation: bool,
    s: &Study,
) -> Result<(f64, usize, usize)> {
    let ready: binary::Value = read_confirmed(&root.join("study-ready.r3b"))?;
    let mut elapsed = ready["elapsed_seconds"]
        .as_f64()
        .ok_or_else(|| bad("prepare time UNKNOWN"))?;
    let mut generation = 0;
    let mut teacher = 0;
    if require_observation || root.join("observation-started.r3b").exists() {
        let r: binary::Value = read_confirmed(&root.join("observation-finished.r3b"))?;
        if !r["error"].is_null()
            || r["study"] != file_hash(&root.join("study.r3b"))?
            || r["start"] != file_hash(&root.join("observation-started.r3b"))?
            || r["control"]["terminal_reason"] != "COMPLETED"
        {
            return Err(bad("study observation failed/incomplete; blocked"));
        }
        let parent: Plan = read(&s.parent.join("plan.r3b"))?;
        let corpus = verified_corpus(&s.parent.join("corpus.r3cor"), &parent.corpus)?;
        let (_, dm, _) = verified_metadata(&s.parent, &parent)?;
        let tok = ByteBpe::load(&s.parent.join("tokenizer.r3b"))?;
        let (es, ms) = subset(&corpus.validation, &dm, if s.tiny { 1 } else { 2 });
        let parity = audit_panel(root, &parent, s.parent_step, "parity", &es, &ms, &tok)?;
        let (es, ms): (Vec<Episode>, Vec<Meta>) = read(&root.join("diagnostic.r3b"))?;
        let diag = audit_panel(
            root,
            &parent,
            s.parent_step,
            if s.schema == 3 {
                "selector192"
            } else {
                "familiar-wording"
            },
            &es,
            &ms,
            &tok,
        )?;
        if r["result"]["parity"] != binary::to_value(parity)?
            || r["result"]["diagnostic"] != binary::to_value(diag)?
        {
            return Err(bad("observation raw summary"));
        }
        elapsed += r["control"]["elapsed_seconds"]
            .as_f64()
            .ok_or_else(|| bad("observation time UNKNOWN"))?;
        generation = r["control"]["generation_calls"]
            .as_u64()
            .ok_or_else(|| bad("generation UNKNOWN"))? as usize;
        teacher = r["control"]["teacher_calls"]
            .as_u64()
            .ok_or_else(|| bad("teacher UNKNOWN"))? as usize;
    }
    for arm in &s.tie_break {
        let a = root.join(arm);
        let p = plan_read_bound(&a, &s.source, &s.binary)?;
        for segment in history(&a, &p)? {
            if require_observation && segment.phase.as_deref() == Some("Failed") {
                return Err(bad("study arm command failed; study blocked"));
            }
            elapsed += segment.elapsed;
            generation += segment.generations;
            teacher += segment.teachers;
        }
    }
    Ok((elapsed, generation, teacher))
}
fn study_report(root: &Path, frozen_executable: Option<&Path>) -> Result<()> {
    let s = study_read_bound(root, frozen_executable)?;
    let (elapsed, generation, teacher) = study_usage_bound(root, false, &s)?;
    let mut results = BTreeMap::new();
    let mut eligible = vec![];
    let mut all_endpoints = true;
    for arm in &s.tie_break {
        let a = root.join(arm);
        let p = plan_read_bound(&a, &s.source, &s.binary)?;
        let h = history(&a, &p)?;
        let last = h.last().ok_or_else(|| bad("arm has not run"))?;
        if last.resume || last.phase.as_deref() != Some("Finished") {
            return Err(bad("arm incomplete"));
        }
        all_endpoints &= last.step == p.config.max_steps;
        let (rows, panels) = audit_panels_through(&a, &p, last.step)?;
        let exposure = audit_updates(&a, &p, &h)?;
        let full = panels.get(&format!("eval-{:04}-dev512", p.config.max_steps));
        let transfer = panels.get(&format!("eval-{:04}-transfer128", p.config.max_steps));
        if let (Some(f), Some(t)) = (full, transfer)
            && f.exact >= p.evaluation.primary_min
            && f.buckets.iter().all(|&k| k >= p.evaluation.bucket_min)
            && t.exact >= p.evaluation.transfer_min
            && f.errors == 0
            && t.errors == 0
        {
            eligible.push((
                arm.clone(),
                f.buckets.iter().min().copied().unwrap_or(0),
                f.exact,
                t.exact,
            ));
        }
        results.insert(arm.clone(),binary::record!({"panels":panels,"raw_rows":rows,"exposure":exposure,"last_step":last.step,"new_updates":last.step-s.parent_step,"durable":last.checkpoint,"stop":last.stop}));
    }
    let mut comparisons = BTreeMap::new();
    let c_arm = &s.tie_break[0];
    let p_arm = &s.tie_break[1];
    if all_endpoints
        && (results[c_arm]["exposure"]["case_order"] != results[p_arm]["exposure"]["case_order"]
            || (s.schema == 2
                && results[c_arm]["exposure"]["target_order"]
                    != results[p_arm]["exposure"]["target_order"])
            || (s.schema == 3
                && (results[c_arm]["exposure"]["phrase_order"]
                    != results[p_arm]["exposure"]["phrase_order"]
                    || results[c_arm]["exposure"]["unchanged_five_tasks"]
                        != results[p_arm]["exposure"]["unchanged_five_tasks"])))
    {
        return Err(bad("actual C/P case/target exposures differ"));
    }
    for panel in ["train64", "dev512", "transfer128"] {
        if !all_endpoints {
            continue;
        }
        let parent = binary::read_value_records(
            &s.parent
                .join(format!("eval-{:04}-{panel}.r3rows", s.parent_step)),
        )?;
        let c = binary::read_value_records(&root.join(c_arm).join(format!(
            "eval-{:04}-{panel}.r3rows",
            s.parent_step + s.updates
        )))?;
        let p = binary::read_value_records(&root.join(p_arm).join(format!(
            "eval-{:04}-{panel}.r3rows",
            s.parent_step + s.updates
        )))?;
        comparisons.insert(panel,binary::record!({"control":c_arm,"treatment":p_arm,"parent_to_control":paired(&parent[1..],&c[1..])?,"parent_to_treatment":paired(&parent[1..],&p[1..])?,"control_to_treatment":paired(&c[1..],&p[1..])?}));
    }
    let mut selector_comparison = BTreeMap::new();
    if s.schema == 3 && all_endpoints {
        let (es, ms): (Vec<Episode>, Vec<Meta>) = read(&root.join("diagnostic.r3b"))?;
        for arm in &s.tie_break {
            let a = root.join(arm);
            let step = s.parent_step + s.updates;
            let original =
                binary::read_value_records(&a.join(format!("eval-{step:04}-dev512.r3rows")))?;
            let flipped =
                binary::read_value_records(&a.join(format!("eval-{step:04}-selector192.r3rows")))?;
            selector_comparison.insert(
                arm,
                selector_pairs(&original[1..], &flipped[1..], &es, &ms)?,
            );
        }
    }
    eligible.sort_by(|a, b| {
        b.1.cmp(&a.1)
            .then(b.2.cmp(&a.2))
            .then(b.3.cmp(&a.3))
            .then(a.0.cmp(&b.0))
    });
    println!(
        "STUDY_REPORT {}",
        binary::record!({"execution_source":s.source,"execution_binary":s.binary,"report_source":source_digest()?,"report_binary":file_hash(&std::env::current_exe()?)?,"read_only_frozen_execution":frozen_executable.is_some(),"results":results,"paired":comparisons,"selector":selector_comparison,"arm_order":s.tie_break,"candidate":eligible.first().map(|x|&x.0),"generation":generation,"teacher":teacher,"active_seconds":elapsed,"result":if !all_endpoints{"STUDY_INCONCLUSIVE_UNEQUAL_BUDGET"}else if eligible.is_empty(){"STUDY_COMPLETE_QUALITY_FAIL"}else{"DEVELOPMENT_PASS_FINAL_NOT_RUN"},"FINAL200":"NOT_OPENED","GOAL1_READY":false})
    );
    if inventory(&s.parent)? != s.inventory {
        return Err(bad("parent preservation inventory mismatch"));
    }
    Ok(())
}
fn audit_updates(root: &Path, p: &Plan, h: &[Segment]) -> Result<binary::Value> {
    let f = p
        .fork
        .as_ref()
        .ok_or_else(|| bad("update audit requires explicit fork"))?;
    let (_, episodes, _) = p.training_corpus(&root.join("corpus.r3cor"))?;
    let tok = ByteBpe::load(&root.join("tokenizer.r3b"))?;
    let mut framed = samples(&episodes, &tok, p.config.seq_len)?;
    framed.extend(p.additional_samples(root, &tok)?);
    let mut steps = 0;
    let mut input = 0u64;
    let mut target = 0u64;
    let mut actual_input = 0u64;
    let mut actual_target = 0u64;
    let mut padding = 0u64;
    let mut order = vec![];
    let mut target_order = vec![];
    let mut phrase_order = vec![];
    let mut unchanged = vec![];
    let mut flips = 0usize;
    let mut counts = vec![[0usize; 2]; episodes.len()];
    for (i, segment) in h.iter().enumerate() {
        let path = root.join(format!("segment-{i:04}/updates.r3rows"));
        if path.exists() {
            for r in binary::read_value_records(&path)? {
                let step = f.origin_step + steps;
                let ids = p.draw(step);
                let draw = p.training_draw(step);
                let expected_input = draw
                    .iter()
                    .map(|&j| (framed[j].tokens.len() - 1) as u64)
                    .sum::<u64>();
                let expected_target = draw
                    .iter()
                    .map(|&j| (framed[j].tokens.len() - framed[j].response_start) as u64)
                    .sum::<u64>();
                if r["step"] != step + 1
                    || r["draw"] != binary::record!(ids)
                    || r["sample_indices"] != binary::record!(draw)
                    || r["input"] != expected_input
                    || r["target"] != expected_target
                    || r["lr_bits"] != f.constant_lr.to_bits()
                {
                    return Err(bad("actual sampler/target/LR trace mismatch"));
                }
                for (bucket, (&j, &selected)) in ids.iter().zip(&draw).enumerate() {
                    let phrase = (selected / episodes.len()) % 2;
                    counts[j][phrase] += 1;
                    phrase_order.push(phrase);
                    flips += usize::from(selected >= 2 * episodes.len());
                    if !(2..=4).contains(&bucket) {
                        unchanged.push(digest(&framed[selected].tokens)?);
                    }
                    target_order
                        .push(framed[selected].tokens[framed[selected].response_start..].to_vec());
                }
                order.push(ids);
                input += expected_input;
                target += expected_target;
                steps += 1;
            }
        }
        if segment.step != f.origin_step + steps {
            return Err(bad("checkpoint step differs from update trace"));
        }
        let c: binary::Value = read(&root.join(format!("segment-{i:04}/train-control.r3b")))?;
        actual_input += c["executed_input_tokens_including_uncommitted"]
            .as_u64()
            .ok_or_else(|| bad("input UNKNOWN"))?;
        actual_target += c["executed_target_tokens_including_uncommitted"]
            .as_u64()
            .ok_or_else(|| bad("target UNKNOWN"))?;
        padding += c["executed_padding_tokens"]
            .as_u64()
            .ok_or_else(|| bad("padding UNKNOWN"))?;
        if segment.input != f.origin_input + actual_input
            || segment.target != f.origin_target + target
        {
            return Err(bad("native counter/trace disagreement"));
        }
    }
    if !p.tiny
        && f.origin_step + steps == p.config.max_steps
        && counts
            .iter()
            .any(|x| *x != if f.variants.is_some() { [1, 1] } else { [2, 0] })
    {
        return Err(bad("completed study exposure contract"));
    }
    if !p.tiny
        && f.origin_step + steps == p.config.max_steps
        && f.arm == "S-SELECT"
        && flips != 3072
    {
        return Err(bad("actual selector exposures"));
    }
    Ok(
        binary::record!({"updates":steps,"case_order":digest(&order)?,"target_order":digest(&target_order)?,"phrase_order":digest(&phrase_order)?,"unchanged_five_tasks":digest(&unchanged)?,"unchanged_digest_kind":"ordered-native-token-row-sha256-v1","selector_draws":flips,"committed_input":input,"committed_target":target,
        "actual_input":actual_input,"actual_target":actual_target,"discarded_input":actual_input-input,"discarded_target":actual_target-target,"padding":padding,
        "case_count":counts.len(),"case_exposure_hash":digest(&counts)?,"original_draws":counts.iter().map(|x|x[0]).sum::<usize>(),"variant_draws":counts.iter().map(|x|x[1]).sum::<usize>()}),
    )
}
#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;
    #[test]
    fn fresh_exposure_digest_preserves_order_above_aggregate_codec_limit() {
        // The actual two-epoch five-task report contains more than one million
        // tokens. Keep the codec limit and bind each bounded native row first.
        let rows: Vec<Vec<u32>> = (0..10_240).map(|n| vec![n as u32; 128]).collect();
        assert!(digest(&rows).is_err());
        let mut hashes: Vec<_> = rows.iter().map(digest).collect::<Result<_>>().unwrap();
        let bound = digest(&hashes).unwrap();
        let record = binary::record!({"unchanged_five_tasks":bound,"rows":rows.len()});
        let bytes = binary::to_storage_vec(&record).unwrap();
        assert_eq!(binary::from_slice::<binary::Value>(&bytes).unwrap(), record);
        hashes.swap(0, 1);
        assert_ne!(digest(&hashes).unwrap(), bound);
        hashes.swap(0, 1);
        let mut changed = rows[0].clone();
        changed[0] += 1;
        hashes[0] = digest(&changed).unwrap();
        assert_ne!(digest(&hashes).unwrap(), bound);
    }
    #[test]
    fn selector_involution_labels_balance_and_negative_cases() {
        let (train, ms) = generate(256, 0, 20260919).unwrap();
        let mut counts = [0usize; 8];
        for (e, m) in train
            .iter()
            .zip(&ms)
            .filter(|(_, m)| (2..=4).contains(&m.bucket))
        {
            let (flipped, fm) = flip_selection(e, m).unwrap();
            let (back, _) = flip_selection(&flipped, &fm).unwrap();
            assert_eq!(digest(&e.request).unwrap(), digest(&back.request).unwrap());
            assert_eq!(e.answer, back.answer);
            assert_eq!(flipped.answer, resolve(&flipped.request).unwrap());
            assert_ne!(
                citations(&e.answer).unwrap(),
                citations(&flipped.answer).unwrap()
            );
            assert_eq!(fm.base, m.base);
            assert_eq!(fm.view, m.view);
            assert_eq!(
                fm.entities[0],
                question_intent(&flipped.request.input).unwrap().0
            );
            if m.bucket < 4 {
                assert_eq!(
                    digest(&e.request.evidence).unwrap(),
                    digest(&flipped.request.evidence).unwrap()
                );
            } else {
                assert_eq!(e.request.input, flipped.request.input);
                let mut restored = flipped.request.evidence.clone();
                for (r, o) in restored.items.iter_mut().zip(&e.request.evidence.items) {
                    r.observed_at = o.observed_at;
                }
                assert_eq!(
                    digest(&restored).unwrap(),
                    digest(&e.request.evidence).unwrap()
                );
            }
            counts[m.bucket] += 1;
        }
        assert_eq!(counts, [0, 0, 1024, 1024, 1024, 0, 0, 0]);
        let index = ms.iter().position(|m| m.bucket == 4).unwrap();
        let e = &train[index];
        let m = &ms[index];
        for fault in 0..6 {
            let mut wrong = e.clone();
            match fault {
                0 => wrong.request.input.clear(),
                1 => {
                    wrong.request.evidence.items[1].event_id =
                        wrong.request.evidence.items[0].event_id
                }
                2 => wrong.request.evidence.items[1].version_status = "superseded".into(),
                3 => {
                    wrong.request.evidence.items[1].observed_at =
                        wrong.request.evidence.items[0].observed_at
                }
                4 => wrong.request.evidence.items[1].original_excerpt.clear(),
                _ => wrong.request.evidence.items[1].observed_at = None,
            };
            assert!(flip_selection(&wrong, m).is_err(), "fault={fault}");
        }
        println!(
            "SELECTOR_NATIVE_REQUEST_INVOLUTION=3072 INVALID_CASES=6 OPTIMIZER=0 GENERATION=0"
        );
    }
    #[test]
    #[ignore = "explicit preserved parent/output paths; at most 16 SMALL generations, no teacher or training"]
    fn stabilization_fixed_parent_parity() -> Result<()> {
        let parent = PathBuf::from(
            std::env::var("R3_PARITY_PARENT")
                .map_err(|_| bad("explicit parity parent required"))?,
        );
        let output = PathBuf::from(
            std::env::var("R3_PARITY_OUTPUT")
                .map_err(|_| bad("explicit new parity output required"))?,
        );
        std::fs::create_dir(&output)?;
        let before = inventory(&parent)?;
        let p: Plan = read(&parent.join("plan.r3b"))?;
        let dev = verified_corpus(&parent.join("corpus.r3cor"), &p.corpus)?.validation;
        let (_, dm, _) = verified_metadata(&parent, &p)?;
        let (es, _) = subset(&dev, &dm, 2);
        if es.len() != 16 || p.tiny {
            return Err(bad("parity fixed panel"));
        }
        // Freeze metadata-selected IDs before history/audit reads any old output.
        let selection = binary::record!({"ids":es.iter().map(|e|&e.id).collect::<Vec<_>>(),"cases":digest(&es)?,"policy":digest(&p)?,"step":p.config.max_steps,"tokenizer":p.tokenizer,"source":source_digest()?,"binary":file_hash(&std::env::current_exe()?)?,"generation_limit":16,"teacher_limit":0});
        write(&output.join("selection.r3b"), &selection)?;
        let h = history(&parent, &p)?;
        let last = h.last().ok_or_else(|| bad("missing parent endpoint"))?;
        if last.step != p.config.max_steps || last.resume {
            return Err(bad("parity requires a closed final parent"));
        }
        let checkpoint = parent.join(&last.checkpoint);
        let l = checkpoint::load(&checkpoint, Device::Cpu, false)?;
        if l.tokenizer.id() != p.tokenizer {
            return Err(bad("parity tokenizer"));
        }
        let old_panel = audit_panel(&parent, &p, last.step, "dev512", &dev, &dm, &l.tokenizer)?;
        if old_panel.model != l.model.weight_hash()? {
            return Err(bad("parity endpoint model"));
        }
        let original = binary::read_value_records(
            &parent.join(format!("eval-{:04}-dev512.r3rows", last.step)),
        )?;
        let mut raw = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(output.join("parity.r3rows"))?;
        append_row(
            &mut raw,
            &binary::record!({"selection":file_hash(&output.join("selection.r3b"))?,"checkpoint":checkpoint,"physical":file_hash(&checkpoint)?,"model":l.model.weight_hash()?}),
        )?;
        let mut control = recovery::RunControl::command(false)?;
        control.set_call_limits(16, 0);
        let mut matched = 0;
        let mut tokens = 0;
        let result = (|| -> Result<()> {
            for e in &es {
                let row = match recovery::observe_generation(&l, e, &e.request, &mut control, false)
                {
                    recovery::ObservedCall::Returned(row) => row,
                    recovery::ObservedCall::NotInvoked(_) => {
                        control.stop_result()?;
                        return Err(bad("parity call not invoked"));
                    }
                };
                append_row(&mut raw, &row)?;
                control.stop_result()?;
                verify_generated(&row, &l.tokenizer)?;
                let old = original[1..]
                    .iter()
                    .find(|r| r["id"] == e.id)
                    .ok_or_else(|| bad("parity ID absent"))?;
                for field in [
                    "raw_tokens",
                    "actual",
                    "error",
                    "finish_reason",
                    "eos_index",
                    "native_prompt_digest",
                ] {
                    if old[field] != row[field] {
                        return Err(bad("normal output parity changed"));
                    }
                }
                tokens += row["raw_tokens"]
                    .as_array()
                    .ok_or_else(|| bad("parity tokens"))?
                    .len();
                matched += 1;
            }
            Ok(())
        })();
        if let Err(error) = &result {
            control.classify_error(error);
        }
        let _ = control.seal_terminal();
        let same = before == inventory(&parent)?;
        publish_confirmed(
            &output.join("result.r3b"),
            &binary::record!({"selection":file_hash(&output.join("selection.r3b"))?,"raw":file_hash(&output.join("parity.r3rows"))?,"control":control.receipt(),"matched":matched,"tokens":tokens,"originals_unchanged":same,"error":result.as_ref().err().map(ToString::to_string),"quality_improvement":"NOT_CLAIMED"}),
        )?;
        result?;
        if !same {
            return Err(bad("parity parent changed"));
        }
        println!(
            "PARENT_PARITY matched={matched}/16 raw_tokens={tokens} SMALL_OPTIMIZER=0 SMALL_TEACHER=0 ORIGINALS_UNCHANGED={same}"
        );
        Ok(())
    }
    #[test]
    fn fresh_strict_rows_roundtrip_command_stop_and_errors() {
        let (episodes, meta) = generate(1, 0, 20260919).unwrap();
        let e = &episodes[0];
        let m = &meta[0];
        let tok = ByteBpe::train(
            &[e.answer.as_bytes().to_vec()],
            &neural::hash(b"strict test"),
            264,
        )
        .unwrap();
        let d = tempfile::tempdir().unwrap();
        for case in 0..6 {
            let text = if case == 1 { "wrong" } else { &e.answer };
            let mut ids = if case == 3 {
                tok.encode(&[255]).unwrap()
            } else {
                tok.encode(text.as_bytes()).unwrap()
            };
            let eos = case != 2 && case != 4;
            let body = ids.clone();
            if eos {
                ids.push(EOS);
            }
            let complete = case != 4;
            let finish = if eos {
                "stop"
            } else if complete {
                "length"
            } else {
                "timeout"
            };
            let error = match case {
                3 => Some("invalid/incomplete output UTF-8"),
                4 => Some("native generation timeout"),
                _ => None,
            };
            let expected_exact = case == 0 || case == 5;
            let row = binary::record!({"row_version":2,"id":e.id,"question":e.request.input,"generated_evidence":e.request.evidence,"expected":e.answer,
                "generation_completed":complete,"generation":if complete{binary::record!({"tokens":body,"finish":finish,"generated":ids.len()})}else{binary::Value::Null},
                "raw_tokens":ids,"actual":if error.is_none(){Some(text)}else{None},"error":error,"error_class":if case==3{Some("strict_utf8")}else{None},
                "finish_reason":finish,"exact_match":expected_exact,"command_stop":if case==5{Some("TIME_BUDGET")}else{None}});
            let path = d.path().join(format!("case-{case}.r3rows"));
            let mut f = std::fs::File::create(&path).unwrap();
            append_row(&mut f, &row).unwrap();
            let loaded = binary::read_value_records(&path).unwrap();
            verify_generated(&loaded[0], &tok).unwrap();
            let scored = score(&loaded, std::slice::from_ref(e), std::slice::from_ref(m)).unwrap();
            assert_eq!(scored.exact, usize::from(expected_exact));
            let mut false_row = loaded[0].clone();
            false_row["exact_match"] = binary::record!(!expected_exact);
            assert!(verify_generated(&false_row, &tok).is_err());
        }
        println!("STRICT_ROW_CASES=6 OPTIMIZER=0 GENERATION=0 independent_oracle=true");
    }
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
                max_steps: 8192,
                budget_start_step: 6144,
                warmup: 0,
                seq_len: 64,
                seed: 29,
                ..Default::default()
            },
            step: 8192,
            consumed_tokens: 20_000_000,
            target_tokens: 1_000_000,
            sampler_state: 8192,
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
            &d.path().join("synthetic6144.r3m"),
            &model,
            &tok,
            m,
            &adam.moments,
        )
        .unwrap();
        let loaded =
            checkpoint::load(&d.path().join("synthetic6144.r3m"), Device::Cpu, true).unwrap();
        assert_eq!(loaded.manifest.training.unwrap(), state);
        let path = d.path().join("trace.r3rows");
        let expected=(6145..=8192).map(|step|binary::record!({"step":step,"sampler":step,"lr_bits":3e-5f64.to_bits(),"draw":[0,1,2,3,4,5,6,7],"input":1234,"target":128})).collect::<Vec<_>>();
        replica_v3::codec::publish_new(&path, |f, _| {
            for row in &expected {
                binary::write_value_record(f, row)?;
            }
            Ok(())
        })
        .unwrap();
        assert_eq!(binary::read_value_records(&path).unwrap(), expected);
        println!("SYNTHETIC_METADATA_STEP=8192 TRACE_ROWS=2048 OPTIMIZER_CALLS=0 backward_calls=3");
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
            schema: None,
            fork: None,
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
