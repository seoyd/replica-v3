//! Explicit training tool only. This module is not linked by the inference library.
#[path = "native_corpus.rs"]
pub mod native;
use replica_v3::{
    Error, Result,
    event::GenerationLimits,
    model::{ModelRequest, SYSTEM},
    neural::{ByteBpe, hash, read_bounded, write_new},
    retrieval::{Evidence, EvidenceBundle},
};
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeSet,
    path::{Path, PathBuf},
};
const MAX_CORPUS: usize = 64 * 1024 * 1024;
pub const GENERATOR_REVISION: &str = "educational-korean-v1";
#[cfg(test)]
mod harness_tests {
    use super::*;
    #[test]
    fn conditional_panel_facts_swap_order_and_independent_oracle() {
        let (cases, foils) = conditional_panel(&[], 19317).unwrap();
        assert_eq!(cases.len(), 144);
        assert_eq!(foils.len(), 144);
        for base in cases.as_chunks::<6>().0 {
            for (view, e) in base.iter().enumerate() {
                let status = if view == 1 { "superseded" } else { "current" };
                let target = e
                    .request
                    .evidence
                    .items
                    .iter()
                    .find(|r| {
                        r.version_status == status && r.original_excerpt.contains(" 이동 지시는 ")
                    })
                    .unwrap();
                assert_eq!(
                    e.answer,
                    format!("{} [event:{}]", target.original_excerpt, target.event_id)
                );
                assert_eq!(
                    e.request.evidence.items.len(),
                    if view == 5 { 3 } else { 2 }
                );
                assert_eq!(target.recorded_at, if view == 1 { 100 } else { 200 });
                assert!(!e.request.input.contains(&e.answer));
                assert!(
                    e.request
                        .input
                        .contains(e.binding.split('/').next().unwrap())
                );
            }
            assert_ne!(base[0].answer, base[1].answer);
            assert_ne!(base[0].answer, base[2].answer);
            for v in [3, 4, 5] {
                assert_eq!(base[0].answer, base[v].answer);
            }
            assert_eq!(
                base[0].request.evidence.items[0].event_id,
                base[2].request.evidence.items[0].event_id
            );
            assert_eq!(
                base[0].request.evidence.items[0].event_id,
                base[3].request.evidence.items[1].event_id
            );
            assert_ne!(base[0].request.input, base[4].request.input);
        }
        let mut seen = BTreeSet::new();
        for e in &cases {
            assert!(seen.insert(e.id.clone()));
        }
        let (again, again_foils) = conditional_panel(&[], 19317).unwrap();
        assert_eq!(native::ordered_bytes(&cases), native::ordered_bytes(&again));
        assert_eq!(foils, again_foils);
    }
    #[test]
    #[ignore = "isolated by bounded parent to avoid an in-process non-progress loop"]
    fn empty_key_child() {
        let value = replace_training_literals("가", &[(String::new(), String::new())]);
        assert!(
            format!("{value:?}").starts_with("Err("),
            "empty key accepted"
        );
    }
    #[test]
    fn harness_m01_empty_key_is_bounded_error() {
        use std::{
            process::{Command, Stdio},
            time::{Duration, Instant},
        };
        let mut child = Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "data::harness_tests::empty_key_child",
                "--ignored",
            ])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .unwrap();
        let start = Instant::now();
        loop {
            if let Some(status) = child.try_wait().unwrap() {
                assert!(status.success());
                break;
            }
            if start.elapsed() > Duration::from_secs(2) {
                child.kill().unwrap();
                child.wait().unwrap();
                panic!(
                    "empty replacement did not consume input or return an error within watchdog"
                );
            }
            std::thread::sleep(Duration::from_millis(10));
        }
    }
    #[test]
    fn harness_m01_literal_progress_unicode_prefix_conflict_and_bound() {
        let pairs = |pairs: &[(&str, &str)]| {
            pairs
                .iter()
                .map(|(a, b)| (a.to_string(), b.to_string()))
                .collect::<Vec<_>>()
        };
        assert_eq!(
            replace_training_literals(
                "구역1 구역10 😀가",
                &pairs(&[("구역1", "구역10"), ("구역10", "구역1"), ("가", "나")])
            )
            .unwrap(),
            "구역10 구역1 😀나"
        );
        assert_eq!(
            replace_training_literals(
                "가나다",
                &pairs(&[("가", "가"), ("가", "가"), ("나", "다"), ("다", "나")])
            )
            .unwrap(),
            "가다나"
        );
        assert!(replace_training_literals("", &pairs(&[("", "x")])).is_err());
        assert!(replace_training_literals("가", &pairs(&[("가", "나"), ("가", "다")])).is_err());
        assert!(replace_training_literals("xx", &[("x".into(), "x".repeat(256 * 1024))]).is_err());
        assert_eq!(
            replace_training_literals("😀가", &pairs(&[("가", "")])).unwrap(),
            "😀"
        );
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Episode {
    pub id: String,
    pub category: usize,
    pub family: String,
    pub binding: String,
    pub sequence: String,
    pub request: ModelRequest,
    pub answer: String,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Split {
    pub file: String,
    pub sha256: String,
    pub bytes: usize,
    pub documents: usize,
    pub tokens: Option<usize>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CorpusManifest {
    pub version: u32,
    pub scope: String,
    pub permission: String,
    pub generator: String,
    pub seed: u64,
    pub split_rule: String,
    pub train: Split,
    pub validation: Split,
}
pub fn load_split_legacy(root: &Path, split: &Split) -> Result<Vec<Episode>> {
    if split.file.contains('/') || split.file.contains('\\') || split.file.starts_with('.') {
        return Err(Error::Invalid("split filename".into()));
    }
    let bytes = read_bounded(&root.join(&split.file), MAX_CORPUS)?;
    if hash(&bytes) != split.sha256 || bytes.len() != split.bytes {
        return Err(Error::Corrupt("corpus hash/size".into()));
    }
    let docs: Vec<Episode> = serde_json::from_slice(&bytes)?;
    if docs.len() != split.documents {
        return Err(Error::Invalid("corpus count".into()));
    }
    validate_episodes(&docs)?;
    Ok(docs)
}
pub(crate) fn validate_episodes(docs: &[Episode]) -> Result<()> {
    if docs.is_empty() || docs.len() > 100_000 {
        return Err(Error::Invalid("corpus count".into()));
    }
    let mut ids = BTreeSet::new();
    for e in docs {
        if !ids.insert(&e.id)
            || e.category > 4
            || e.request.input.len() > 262144
            || e.answer.len() > 262144
        {
            return Err(Error::Invalid("corpus episode/size".into()));
        }
        e.request.limits.validate()?;
        replica_v3::event::check_refs(
            &e.request
                .evidence
                .items
                .iter()
                .map(|e| e.event_id)
                .collect::<Vec<_>>(),
        )?;
        let ids: Vec<_> = e
            .request
            .evidence
            .items
            .iter()
            .map(|e| e.event_id)
            .collect();
        if replica_v3::app::citations(&e.answer)?
            .iter()
            .any(|id| !ids.contains(id))
        {
            return Err(Error::Invalid("training citation outside evidence".into()));
        }
    }
    Ok(())
}
pub fn load_legacy(root: &Path) -> Result<(CorpusManifest, Vec<Episode>, Vec<Episode>)> {
    let manifest: CorpusManifest =
        serde_json::from_slice(&read_bounded(&root.join("manifest.json"), 65536)?)?;
    if manifest.version != 1 {
        return Err(Error::Invalid("corpus version".into()));
    }
    let train = load_split_legacy(root, &manifest.train)?;
    let validation = load_split_legacy(root, &manifest.validation)?;
    check_split(&train, &validation)?;
    Ok((manifest, train, validation))
}
pub fn load(root: &Path) -> Result<(CorpusManifest, Vec<Episode>, Vec<Episode>)> {
    let c = native::read(root)?;
    Ok((c.manifest, c.train, c.validation))
}
pub(crate) fn check_split(train: &[Episode], validation: &[Episode]) -> Result<()> {
    for field in [0, 1, 2, 3] {
        let value = |e: &Episode| match field {
            0 => e.id.clone(),
            1 => e.binding.clone(),
            2 => e.family.clone(),
            _ => e.sequence.clone(),
        };
        let a: BTreeSet<_> = train.iter().map(value).collect();
        if validation.iter().map(value).any(|v| a.contains(&v)) {
            return Err(Error::Invalid(
                "episode/binding/template/sequence split leakage".into(),
            ));
        }
    }
    Ok(())
}
fn evidence(id: i64, text: String, status: &str) -> Evidence {
    Evidence {
        event_id: id,
        original_excerpt: text,
        excerpt_truncated: false,
        source: "synthetic".into(),
        recorded_at: id,
        observed_at: None,
        version_status: status.into(),
        retrieval_reason: "lexical".into(),
        relation_path: vec![],
    }
}
// Training-only renderer. Final test questions and expected outputs are constructed
// independently by the evaluation tool after the candidate is fixed.
fn synthetic(count: usize, seed: u64, validation: bool) -> Vec<Episode> {
    (0..count)
        .map(|i| {
            let category = i % 5;
            let n = (i as u64)
                .wrapping_mul(6364136223846793005)
                .wrapping_add(seed);
            let pool = if validation { "검증" } else { "연습" };
            let entity = format!("{pool}{}호", (n >> 16) % 41);
            let context = format!("구역{}", (n >> 8) % 7);
            let value = ["오른쪽", "왼쪽", "직진", "대기"][(n % 4) as usize];
            let old = ["북쪽", "남쪽", "동쪽", "서쪽"][((n >> 4) % 4) as usize];
            let id = 1 + ((n >> 24) % 800) as i64;
            let family = format!("{pool}-{category}-{}", i % 3);
            let mut items = vec![evidence(
                id,
                format!("{entity}의 {context} 이동 지시는 {value}이다."),
                "current",
            )];
            let (question, answer) = match category {
                0 => {
                    items.push(evidence(
                        id + 1,
                        "별도 장비의 이동 지시는 대기이다.".into(),
                        "current",
                    ));
                    (
                        format!("{entity} {context} 현재 이동 지시는 무엇인가?"),
                        format!("{value}입니다. [event:{id}]"),
                    )
                }
                1 => {
                    items[0].event_id = id + 1;
                    items[0].recorded_at = id + 1;
                    items.insert(
                        0,
                        evidence(
                            id,
                            format!("{entity}의 {context} 이전 지시는 {old}였다."),
                            "superseded",
                        ),
                    );
                    if i % 3 == 0 {
                        items[1].version_status = "superseded".into();
                        items.push(evidence(
                            id + 2,
                            format!("{entity}의 {context} 지시를 {old}으로 복원했다."),
                            "current",
                        ));
                        (
                            format!("{entity} {context} 복원 후 현재 이동 지시는?"),
                            format!("{old}입니다. [event:{}]", id + 2),
                        )
                    } else if i % 2 == 0 {
                        (
                            format!("{entity} {context} 정정 전 과거 지시는?"),
                            format!("{old}입니다. [event:{id}]"),
                        )
                    } else {
                        (
                            format!("{entity} {context} 정정 후 현재 지시는?"),
                            format!("{value}입니다. [event:{}]", id + 1),
                        )
                    }
                }
                2 => {
                    items.push(evidence(
                        id + 1,
                        format!("{entity}의 다른 구역 이동 지시는 {old}이다."),
                        "current",
                    ));
                    (
                        format!("{entity}는 {context}에서 어느 쪽으로 가야 하나?"),
                        format!("{value}입니다. [event:{id}]"),
                    )
                }
                3 => (
                    format!("{entity} {context} 기록에서 이동 값을 찾아줘."),
                    format!("{value}입니다. [event:{id}]"),
                ),
                _ => {
                    if i % 2 == 0 {
                        items.clear();
                        (
                            format!("{entity}의 사고 원인은 무엇인가?"),
                            "근거가 없어 알 수 없습니다.".into(),
                        )
                    } else {
                        items.push(evidence(
                            id + 1,
                            format!(
                                "{entity}는 지시를 실행한 뒤 사고가 났다. 원인은 조사되지 않았다."
                            ),
                            "current",
                        ));
                        (
                            format!("{entity}의 {value} 지시가 사고 원인으로 확정됐나?"),
                            format!("원인은 확정되지 않았습니다. [event:{}]", id + 1),
                        )
                    }
                }
            };
            let question = if validation {
                // Different question family and episode shape, not merely a seed
                // change or a differently named split containing the same scenes.
                items.push(evidence(
                    id + 10,
                    "검증용 다른 설비는 점검 중이며 이동값은 알려지지 않았다.".into(),
                    "current",
                ));
                match category {
                    0 => format!(
                        "지금 유효한 자료를 기준으로 {context}에 있는 {entity}의 방향을 말해줘."
                    ),
                    1 if i % 3 == 0 => {
                        format!("{entity}를 복구한 기록 이후 {context}의 유효한 방향은?")
                    }
                    1 if i % 2 == 0 => format!("{context}에서 {entity}를 바꾸기 이전 값이 궁금해."),
                    1 => format!("{entity} 변경 기록 중 {context}에 적용되는 최신 값은?"),
                    2 => format!(
                        "다른 구역은 제외하고 {context}의 {entity}에게 적용되는 지시만 알려줘."
                    ),
                    3 => format!("제시된 문서에서 {entity}와 {context}에 해당하는 방향을 읽어줘."),
                    _ => format!("{entity}의 사고가 앞선 지시 때문이라고 결론지을 증거가 있니?"),
                }
            } else {
                question
            };
            if i % 3 == 0 {
                items.reverse();
            }
            Episode {
                id: format!("{pool}-{seed}-{i}"),
                category,
                family,
                binding: format!("{entity}/{context}/{value}"),
                sequence: format!("{pool}-{category}-{}-{}", i % 3, items.len()),
                request: ModelRequest {
                    request_id: format!("training-{i}"),
                    system: SYSTEM.into(),
                    input: question,
                    evidence: EvidenceBundle {
                        items,
                        ..Default::default()
                    },
                    limits: GenerationLimits {
                        max_tokens: 128,
                        context_tokens: 2048,
                        timeout_ms: 120000,
                    },
                },
                answer,
            }
        })
        .collect()
}
fn save_split(root: &Path, name: &str, docs: &[Episode]) -> Result<Split> {
    let bytes = serde_json::to_vec(docs)?;
    if bytes.len() > MAX_CORPUS {
        return Err(Error::Invalid("corpus byte budget".into()));
    }
    let file = format!("{name}.json");
    write_new(&root.join(&file), &bytes)?;
    Ok(Split {
        file,
        sha256: hash(&bytes),
        bytes: bytes.len(),
        documents: docs.len(),
        tokens: None,
    })
}
// Curriculum and QA data remain training-tool code. Neither final-test expectations
// nor a renderer can be invoked by a native inference worker.
fn curriculum(count: usize, seed: u64, validation: bool, balanced: bool) -> Vec<Episode> {
    use replica_v3::neural::transformer::Rng;
    let mut rng = Rng::new(seed ^ if validation { 0x9173 } else { 0x4137 });
    let pool = if validation { "validation" } else { "train" };
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
    let mut out = Vec::new();
    for i in 0..count {
        let copy = i % 6 == 5;
        let category = if copy { 3 } else { i % 6 };
        let variant = (rng.next_u64() % 3) as usize;
        let style = if balanced {
            (rng.next_u64() % 6) as usize
        } else {
            variant
        };
        let digit = rng.next_u64() % 4;
        let start = [1, 10, 100, 1000][digit as usize];
        let id = start + (rng.next_u64() % (start * 9) as u64) as i64;
        let other_id = if balanced {
            let base = [1, 10, 100, 1000][(rng.next_u64() % 4) as usize];
            let candidate = base + (rng.next_u64() % (base * 9) as u64) as i64;
            if candidate == id {
                candidate + 1
            } else {
                candidate
            }
        } else {
            id + 1
        };
        let value_index = (rng.next_u64() % values.len() as u64) as usize;
        let value = values[value_index];
        let other = values[(value_index + 1 + (rng.next_u64() % 7) as usize) % values.len()];
        let prefix = ["장치", "설비", "센서", "장비"][(rng.next_u64() % 4) as usize];
        // Disjoint entity sets with the same range/length distribution. Both the
        // target and distractor share split parity, so parity cannot select an answer.
        let parity = if validation { 2 } else { 1 };
        let number = if balanced {
            500_000 + 2 * ((i * 7919 + seed as usize) % 100_000) + parity
        } else if validation {
            80000 + i
        } else {
            1000 + i
        };
        let entity = format!("{prefix}{number}");
        let distractor_number = if balanced {
            let candidate = 500_000 + 2 * (rng.next_u64() as usize % 100_000) + parity;
            if candidate == number {
                500_000 + (candidate + 2 - 500_000 - parity) % 200_000 + parity
            } else {
                candidate
            }
        } else {
            number + 30000
        };
        let distractor = format!("{prefix}{distractor_number}");
        let (context, other_context) = if balanced {
            let first = (rng.next_u64() % 39) as usize;
            let second = (first + 1 + (rng.next_u64() % 38) as usize) % 39;
            let render = |n: usize| format!("{}{}", ["구역", "통로", "현장"][n / 13], n % 13);
            (render(first), render(second))
        } else {
            (
                format!(
                    "{}{}",
                    ["구역", "통로", "현장"][(rng.next_u64() % 3) as usize],
                    rng.next_u64() % 13
                ),
                format!("별관{}", rng.next_u64() % 9),
            )
        };
        let full_system = !copy || if balanced { variant == 2 } else { i % 18 == 17 };
        let mut items = vec![evidence(
            id,
            format!("{entity}의 {context} 이동 지시는 {value}이다."),
            "current",
        )];
        let (mut question, answer, mut family) = if copy {
            let q = match (validation, variant) {
                (false, 0) => "제공된 사건 번호만 숫자로 그대로 적어줘.",
                (false, 1) => "기록의 이동 값만 그대로 적어줘.",
                (false, _) => "이 기록의 이동 지시는 무엇인가? 사건을 인용해줘.",
                (true, 0) => "자료의 사건 ID 숫자를 복사해줘.",
                (true, 1) => "자료 속 방향 값만 복사해줘.",
                (true, _) => "자료에 명시된 지시와 근거를 답해줘.",
            };
            let a = match variant {
                0 => id.to_string(),
                1 => value.to_string(),
                _ => format!("{value}입니다. [event:{id}]"),
            };
            (q.to_string(), a, format!("copy/{pool}/{variant}"))
        } else {
            let (q, a) = match category {
                0 | 3 => {
                    if category == 0 {
                        items.push(evidence(
                            other_id,
                            format!("{distractor}의 {context} 이동 지시는 {other}이다."),
                            "current",
                        ));
                    }
                    let q = if validation {
                        format!(
                            "제공 자료에 따르면 {context}에서 {entity}가 따라야 할 방향은 어느 쪽이지?"
                        )
                    } else {
                        match variant {
                            0 => format!("{entity} {context} 현재 이동 지시는 무엇인가?"),
                            1 => format!(
                                "지금 유효한 {entity}의 {context} 방향을 근거와 함께 알려줘."
                            ),
                            _ => format!("{context} 기록에서 {entity}에게 지정된 이동 값은?"),
                        }
                    };
                    (q, format!("{value}입니다. [event:{id}]"))
                }
                1 => {
                    items[0].event_id = id + 1;
                    items.insert(
                        0,
                        evidence(
                            id,
                            if balanced {
                                format!("{entity}의 {context} 이동 지시는 {other}이다.")
                            } else {
                                format!("{entity}의 {context} 이전 지시는 {other}였다.")
                            },
                            "superseded",
                        ),
                    );
                    let (query, expected, cited) = match variant {
                        0 => {
                            items[1].version_status = "superseded".into();
                            items.push(evidence(
                                id + 2,
                                if balanced {
                                    format!("{entity}의 {context} 이동 지시는 {other}이다.")
                                } else {
                                    format!("{entity}의 {context} 지시를 {other}으로 복원했다.")
                                },
                                "current",
                            ));
                            (
                                if validation {
                                    "복구까지 반영한 현재 방향을 답해줘."
                                } else {
                                    "복원 후 현재 이동 지시는?"
                                },
                                other,
                                id + 2,
                            )
                        }
                        1 => (
                            if validation {
                                "변경이 있기 이전의 방향을 답해줘."
                            } else {
                                "정정 전 과거 이동 지시는?"
                            },
                            other,
                            id,
                        ),
                        _ => (
                            if validation {
                                "정정 내용을 반영한 지금의 방향을 답해줘."
                            } else {
                                "정정 후 최신 이동 지시는?"
                            },
                            value,
                            id + 1,
                        ),
                    };
                    (
                        format!("{entity} {context} {query}"),
                        format!("{expected}입니다. [event:{cited}]"),
                    )
                }
                2 => {
                    items.push(evidence(
                        other_id,
                        format!("{entity}의 {other_context} 이동 지시는 {other}이다."),
                        "current",
                    ));
                    let q = if validation {
                        format!(
                            "{entity}에 대해 {other_context}가 아닌 {context}의 지시를 구분해 답해줘."
                        )
                    } else {
                        match variant {
                            0 => format!("{entity}는 {context}에서 어느 방향으로 가야 하나?"),
                            1 => format!(
                                "다른 현장과 구분하여 {entity}의 {context} 이동 값을 알려줘."
                            ),
                            _ => {
                                format!("{other_context} 말고 {context}의 {entity} 지시만 답해줘.")
                            }
                        }
                    };
                    (q, format!("{value}입니다. [event:{id}]"))
                }
                _ => {
                    if variant == 0 {
                        items.clear();
                        if (i / 6).is_multiple_of(2) {
                            items.push(evidence(
                                id + 7,
                                format!("{distractor}의 점검은 끝났지만 사고 자료는 없다."),
                                "observation_or_interpretation",
                            ));
                        }
                        (
                            if validation {
                                format!("{entity}에 일어난 사고의 원인이 자료로 확인되는가?")
                            } else {
                                format!("{entity}의 사고 원인은 무엇인가?")
                            },
                            "근거가 없어 알 수 없습니다.".into(),
                        )
                    } else {
                        items.push(evidence(
                            id + 1,
                            format!("{entity}는 {value} 지시를 실행했다."),
                            "observation_or_interpretation",
                        ));
                        items.push(evidence(
                            id + 2,
                            format!("이후 {entity}의 사고가 기록되었다. 원인은 확인되지 않았다."),
                            "observation_or_interpretation",
                        ));
                        (
                            if validation {
                                format!(
                                    "{entity} 지시와 실행, 사고가 순서대로 기록되면 인과관계도 확정된 것인가?"
                                )
                            } else if variant == 1 {
                                format!("{entity}의 앞선 이동 지시가 사고의 원인으로 확정됐나?")
                            } else {
                                format!("{entity}의 시간 순서만으로 사고 원인을 알 수 있나?")
                            },
                            format!("원인은 확정되지 않았습니다. [event:{}]", id + 2),
                        )
                    }
                }
            };
            (q, a, format!("qa/{pool}/{category}/{variant}"))
        };
        if balanced {
            question = if copy {
                match (validation, variant, style % 2) {
                    (true, 0, _) => "제시된 근거의 event 번호만 적어줘.",
                    (true, 1, _) => "여기서 이동 방향 값 하나만 적어줘.",
                    (true, _, _) => "이동 지시를 문장으로 답하고 해당 사건을 인용해줘.",
                    (false, 0, 0) => "자료의 사건 ID만 숫자로 복사해줘.",
                    (false, 0, _) => "제공된 사건 번호만 그대로 적어줘.",
                    (false, 1, 0) => "기록의 이동 값만 그대로 적어줘.",
                    (false, 1, _) => "자료 속 방향 값 하나만 복사해줘.",
                    (false, _, 0) => "이 기록의 이동 지시는 무엇인가? 사건을 인용해줘.",
                    (false, _, _) => "명시된 이동 방향과 사건 인용을 문장으로 답해줘.",
                }
                .into()
            } else {
                match category {
                    0 | 3 => {
                        if validation {
                            format!("알려진 기록으로 {entity}의 {context} 방향을 확인해줘.")
                        } else {
                            match style {
                                0 => format!("{entity} {context} 현재 이동 지시는 무엇인가?"),
                                1 => format!(
                                    "지금 유효한 {entity}의 {context} 방향을 근거와 함께 알려줘."
                                ),
                                2 => format!("{context} 기록에서 {entity}에게 지정된 이동 값은?"),
                                3 => format!(
                                    "{entity}가 {context}에서 따라야 할 방향은 어느 쪽이지?"
                                ),
                                4 => format!(
                                    "제공된 자료에서 {context}의 {entity} 이동 방향을 찾아줘."
                                ),
                                _ => format!(
                                    "다른 장비 말고 {entity}의 {context} 현재 지시를 답해줘."
                                ),
                            }
                        }
                    }
                    1 => {
                        let temporal = match (validation, variant, style % 3) {
                            (true, 0, _) => "되돌린 기록까지 포함해 현재 적용될 방향은?",
                            (true, 1, _) => "현재값 대신 정정 이전 기록의 방향만 답하라.",
                            (true, _, _) => "변경된 기록을 적용한 현재 방향을 답하라.",
                            (false, 0, 0) => "복원 후 현재 이동 지시는?",
                            (false, 0, 1) => "다시 복구한 뒤 유효한 방향은?",
                            (false, 0, _) => "되돌린 다음 지금의 이동 값을 알려줘.",
                            (false, 1, 0) => "정정 전 과거 이동 지시는?",
                            (false, 1, 1) => "변경 이전에는 어느 방향이었지?",
                            (false, 1, _) => "현재 말고 처음 기록했던 방향을 알려줘.",
                            (false, _, 0) => "정정 후 최신 이동 지시는?",
                            (false, _, 1) => "변경을 반영한 지금의 방향은?",
                            (false, _, _) => "과거 기록 말고 현재 유효한 지시를 알려줘.",
                        };
                        format!("{entity} {context} {temporal}")
                    }
                    2 => {
                        if validation {
                            format!(
                                "자료에 {other_context}도 나오지만 {context}에 해당하는 {entity}의 방향을 읽어줘."
                            )
                        } else {
                            match style % 3 {
                                0 => format!("{entity}는 {context}에서 어느 방향으로 가야 하나?"),
                                1 => format!(
                                    "다른 현장과 구분하여 {entity}의 {context} 이동 값을 알려줘."
                                ),
                                _ => format!(
                                    "{other_context} 말고 {context}의 {entity} 지시만 답해줘."
                                ),
                            }
                        }
                    }
                    _ if variant == 0 => {
                        if validation {
                            format!("현재 근거로 {entity}의 사고 원인을 알 수 있는지 답해줘.")
                        } else {
                            match style % 3 {
                                0 => format!("{entity}의 사고 원인은 무엇인가?"),
                                1 => format!("{entity}의 원인으로 확정한 기록이 있어?"),
                                _ => format!("자료로 {entity}의 사고 원인을 확인할 수 있어?"),
                            }
                        }
                    }
                    _ => {
                        if validation {
                            format!("{entity}에 일어난 순서와 확정 원인을 구분하여 말해줘.")
                        } else {
                            match style % 3 {
                                0 => {
                                    format!("{entity}의 앞선 이동 지시가 사고의 원인으로 확정됐나?")
                                }
                                1 => format!("{entity}의 시간 순서만으로 사고 원인을 알 수 있나?"),
                                _ => format!(
                                    "{entity}의 지시와 실행 다음 사고가 기록되면 원인도 확인되는가?"
                                ),
                            }
                        }
                    }
                }
            };
            family = format!(
                "{}{pool}/{category}/{variant}/{style}",
                if copy {
                    "copy/balanced/"
                } else {
                    "qa/balanced/"
                }
            );
        }
        // Timestamp is independent of citation ID, as in the real memory store.
        let recorded = 1_780_000_000_000i64 + (rng.next_u64() % 1_000_000) as i64;
        for (n, e) in items.iter_mut().enumerate() {
            e.recorded_at = recorded
                + if balanced && (category == 0 || category == 2) {
                    (rng.next_u64() % 10_000) as i64
                } else {
                    n as i64
                };
            e.observed_at = (i % 3 == 0).then_some(recorded - 1000);
            e.source = ["user", "sensor", "manual"][(rng.next_u64() % 3) as usize].into();
        }
        for n in (1..items.len()).rev() {
            items.swap(n, (rng.next_u64() % (n + 1) as u64) as usize);
        }
        out.push(Episode {
            id: format!(
                "{}-{pool}-{seed}-{i}",
                if balanced { "balanced" } else { "curriculum" }
            ),
            category,
            family,
            binding: if balanced {
                format!("{entity}/{context}/{value}")
            } else {
                format!("{pool}/{entity}/{context}/{value}")
            },
            sequence: if balanced {
                hash(
                    &serde_json::to_vec(&(&question, &items))
                        .expect("serializable synthetic episode"),
                )
            } else {
                format!("{pool}/{category}/{variant}/{}", items.len())
            },
            request: ModelRequest {
                request_id: format!("corpus-{pool}-{i}"),
                system: if full_system {
                    SYSTEM.into()
                } else {
                    String::new()
                },
                input: question,
                evidence: EvidenceBundle {
                    items,
                    ..Default::default()
                },
                limits: GenerationLimits {
                    max_tokens: 128,
                    context_tokens: 2048,
                    timeout_ms: 120000,
                },
            },
            answer,
        });
    }
    out
}
// Targeted training exercises for the observed multi-record binding error. This
// code is training-only, and does not run before/after product generation.
fn grounding(count: usize, seed: u64, validation: bool) -> Vec<Episode> {
    use replica_v3::neural::transformer::Rng;
    let mut rng = Rng::new(seed ^ if validation { 0x1459 } else { 0x2567 });
    let mut episodes = curriculum(count, seed, validation, true);
    let directions = [
        "오른쪽",
        "왼쪽",
        "직진",
        "대기",
        "북쪽",
        "남쪽",
        "동쪽",
        "서쪽",
    ];
    for (index, episode) in episodes.iter_mut().enumerate() {
        let fields: Vec<_> = episode.binding.split('/').collect();
        let (entity, context, value) = (fields[0], fields[1], fields[2]);
        let items = &mut episode.request.evidence.items;
        if episode.family.starts_with("copy/") {
            let mode = index / 6 % 4;
            let first = items[0].clone();
            let mut other = first.clone();
            let id = 1 + (rng.next_u64() % 9999) as i64;
            other.event_id = if id == first.event_id { id + 1 } else { id };
            other.recorded_at -= 100 + (rng.next_u64() % 5000) as i64;
            let alternate = directions
                .iter()
                .position(|v| *v == value)
                .expect("own corpus value");
            let alternate =
                directions[(alternate + 1 + (rng.next_u64() % 7) as usize) % directions.len()];
            let digits = entity
                .chars()
                .position(|c| c.is_ascii_digit())
                .expect("own entity number");
            let prefix: String = entity.chars().take(digits).collect();
            let number: usize = entity
                .chars()
                .skip(digits)
                .collect::<String>()
                .parse()
                .expect("own entity number");
            let distractor = format!(
                "{prefix}{}",
                500_000 + (number - 500_000 + 40_000) % 200_000
            );
            let context_prefix: String = context
                .chars()
                .take_while(|c| !c.is_ascii_digit())
                .collect();
            let context_number: usize = context[context_prefix.len()..]
                .parse()
                .expect("own context number");
            let different_context = format!(
                "{context_prefix}{}",
                (context_number + 1 + (rng.next_u64() % 12) as usize) % 13
            );
            other.original_excerpt =
                format!("{distractor}의 {context} 이동 지시는 {alternate}이다.");
            match mode {
                0 => {
                    episode.request.input = if validation {
                        format!("{entity} {context}에 해당하는 사건 ID 숫자만 답하라.")
                    } else {
                        format!("다른 장비와 구분하여 {entity}의 {context} 사건 번호만 복사해줘.")
                    };
                    episode.answer = first.event_id.to_string();
                }
                1 => {
                    episode.request.input = if validation {
                        format!("이동 값이 {value}인 자료의 event 번호는?")
                    } else {
                        format!("{value} 지시가 적힌 사건의 번호만 숫자로 적어줘.")
                    };
                    episode.answer = first.event_id.to_string();
                }
                2 => {
                    other.original_excerpt =
                        format!("{entity}의 {different_context} 이동 지시는 {alternate}이다.");
                    episode.request.input = if validation {
                        format!("{entity}의 {context} 자료에 적힌 방향 값만 복사하라.")
                    } else {
                        format!(
                            "{different_context}가 아닌 {context}에서 {entity}의 이동 값만 적어줘."
                        )
                    };
                    episode.answer = value.into();
                }
                _ => {
                    other.original_excerpt =
                        format!("{entity}의 {context} 이동 지시는 {alternate}이다.");
                    other.version_status = "superseded".into();
                    other.event_id = first.event_id;
                    items[0].event_id += 1;
                    episode.request.input = if validation {
                        "현재 적용되는 자료의 사건 ID만 답하라.".into()
                    } else {
                        "superseded된 과거 자료 말고 current인 사건 번호만 복사해줘.".into()
                    };
                    episode.answer = items[0].event_id.to_string();
                }
            }
            items.push(other);
            if rng.next_u64().is_multiple_of(2) {
                items.reverse();
            }
            episode.request.system.clear();
            episode.family = format!(
                "copy/grounding/{}/{mode}",
                if validation { "validation" } else { "train" }
            );
        } else if episode.category == 4 && items.len() == 3 && rng.next_u64().is_multiple_of(2) {
            let mut chronology: Vec<_> = items.iter().collect();
            chronology.sort_by_key(|item| item.recorded_at);
            episode.request.input = if validation {
                format!(
                    "{entity}에 대해 자료로 확인할 수 있는 사건 순서와 원인 판단의 한계를 함께 답해줘."
                )
            } else {
                format!(
                    "{entity}의 지시부터 실행과 사고까지 확인된 일과 원인의 불확실성을 설명해줘."
                )
            };
            episode.answer = format!(
                "지시 [event:{}] 뒤 실행 [event:{}], 이후 사고 [event:{}]가 기록되었습니다. 원인은 확정되지 않았습니다.",
                chronology[0].event_id, chronology[1].event_id, chronology[2].event_id
            );
            episode.family = format!(
                "qa/grounding/{}/causal-sequence",
                if validation { "validation" } else { "train" }
            );
        } else if episode.category == 1 && items.len() == 3 && rng.next_u64().is_multiple_of(2) {
            let first = items
                .iter()
                .min_by_key(|item| item.recorded_at)
                .expect("three records");
            // Restoration copies immutable original bytes; asking about the first
            // version must still cite the first event, not the restored event.
            episode.request.input = if validation {
                format!("{entity} {context}의 복원본 대신 최초 버전에 적힌 방향을 말하라.")
            } else {
                format!("{entity} {context}의 현재 복구된 기록 말고 처음 사건의 지시와 근거는?")
            };
            let value = first
                .original_excerpt
                .split("이동 지시는 ")
                .nth(1)
                .expect("own sentence")
                .strip_suffix("이다.")
                .expect("own sentence");
            episode.answer = format!("{value}입니다. [event:{}]", first.event_id);
            episode.family = format!(
                "qa/grounding/{}/initial-before-restore",
                if validation { "validation" } else { "train" }
            );
        }
        episode.id = format!(
            "grounding-{}-{seed}-{index}",
            if validation { "validation" } else { "train" }
        );
        episode.sequence =
            hash(&serde_json::to_vec(&(&episode.request.input, items)).expect("synthetic episode"));
    }
    episodes
}
// One-pass replacement avoids cascading label/ID substitutions. Training-only;
// neither the inference library nor the independent final renderer calls this.
pub(crate) fn replace_training_literals(
    text: &str,
    replacements: &[(String, String)],
) -> Result<String> {
    // A request-sized upper bound, checked before every append including identity replacements.
    const MAX_REPLACED_BYTES: usize = 256 * 1024;
    let mut keys = std::collections::BTreeMap::new();
    for (old, new) in replacements {
        if old.is_empty() || keys.insert(old, new).is_some_and(|prior| prior != new) {
            return Err(Error::Invalid(
                "empty or conflicting training replacement key".into(),
            ));
        }
    }
    if text.len() > MAX_REPLACED_BYTES {
        return Err(Error::Invalid("training replacement output bound".into()));
    }
    let mut ordered: Vec<_> = keys.into_iter().collect();
    ordered.sort_by_key(|(old, _)| std::cmp::Reverse(old.len()));
    let mut result = String::with_capacity(text.len());
    let mut remaining = text;
    while !remaining.is_empty() {
        let (consumed, replacement) = if let Some((old, new)) = ordered
            .iter()
            .find(|(old, _)| remaining.starts_with(old.as_str()))
        {
            (old.len(), new.as_str())
        } else {
            let next = remaining.chars().next().expect("nonempty text");
            (next.len_utf8(), &remaining[..next.len_utf8()])
        };
        if result
            .len()
            .checked_add(replacement.len())
            .is_none_or(|n| n > MAX_REPLACED_BYTES)
        {
            return Err(Error::Invalid("training replacement output bound".into()));
        }
        result.push_str(replacement);
        remaining = &remaining[consumed..]; // Keys are nonempty; unmatched UTF-8 consumes one character.
    }
    Ok(result)
}
// Four different evidence bindings per base scene discourage question-only
// memorization. Every variant is still a real, explicitly stored training episode.
fn counterfactual(count: usize, seed: u64, validation: bool) -> Result<Vec<Episode>> {
    use replica_v3::neural::transformer::Rng;
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
    let mut rng = Rng::new(seed ^ if validation { 0x4812 } else { 0x8193 });
    let mut out = Vec::with_capacity(count);
    for mut base in grounding(count.div_ceil(4), seed, validation) {
        // Reporting known chronology and its uncertainty is valid even when the
        // question asks only about the cause. It must cite all three actual events.
        if base.category == 4 && base.request.evidence.items.len() == 3 {
            let mut rows: Vec<_> = base.request.evidence.items.iter().collect();
            rows.sort_by_key(|e| e.recorded_at);
            base.answer = format!(
                "지시 [event:{}] 뒤 실행 [event:{}], 이후 사고 [event:{}]가 기록되었습니다. 원인은 확정되지 않았습니다.",
                rows[0].event_id, rows[1].event_id, rows[2].event_id
            );
        }
        let mut permutation = values;
        for index in (1..permutation.len()).rev() {
            permutation.swap(index, (rng.next_u64() % (index + 1) as u64) as usize);
        }
        let mut used_ids = BTreeSet::new();
        for variant in 0..4 {
            if out.len() == count {
                break;
            }
            let mut episode = base.clone();
            let mut replacements: Vec<_> = values
                .iter()
                .enumerate()
                .map(|(i, old)| {
                    (
                        old.to_string(),
                        permutation[(i + variant) % values.len()].to_string(),
                    )
                })
                .collect();
            let old_ids: Vec<_> = episode
                .request
                .evidence
                .items
                .iter()
                .map(|e| e.event_id)
                .collect();
            let mut ids = std::collections::BTreeMap::new();
            for old in old_ids {
                let mut new = 1 + (rng.next_u64() % 9999) as i64;
                while !used_ids.insert(new) {
                    new = new % 9999 + 1;
                }
                ids.insert(old, new);
                replacements.push((format!("[event:{old}]"), format!("[event:{new}]")));
            }
            let offset = (rng.next_u64() % 100_000_000) as i64;
            for item in &mut episode.request.evidence.items {
                item.original_excerpt =
                    replace_training_literals(&item.original_excerpt, &replacements)?;
                item.event_id = ids[&item.event_id];
                item.recorded_at += offset;
                item.observed_at = item.observed_at.map(|t| t + offset);
                item.source = ["sensor", "manual", "user"][(rng.next_u64() % 3) as usize].into();
            }
            episode.answer = if let Ok(id) = episode.answer.parse::<i64>() {
                ids[&id].to_string()
            } else {
                replace_training_literals(&episode.answer, &replacements)?
            };
            episode.request.input =
                replace_training_literals(&episode.request.input, &replacements)?;
            episode.binding = replace_training_literals(&episode.binding, &replacements)?;
            let items = &mut episode.request.evidence.items;
            for index in (1..items.len()).rev() {
                items.swap(index, (rng.next_u64() % (index + 1) as u64) as usize);
            }
            episode.id = format!("counterfactual/{}/{variant}", base.id);
            episode.family = format!("{}/counterfactual/{variant}", base.family);
            episode.request.request_id = episode.id.clone();
            episode.sequence = hash(
                &serde_json::to_vec(&(&episode.request.input, items)).expect("synthetic variant"),
            );
            out.push(episode);
        }
    }
    Ok(out)
}
// Training-only factorization: generate the supporting event before its value.
// The product decoder still predicts every token; there is no output formatter.
fn evidence_first(count: usize, seed: u64, validation: bool) -> Result<Vec<Episode>> {
    let mut episodes = counterfactual(count, seed, validation)?;
    for episode in &mut episodes {
        if episode.category < 4 && !episode.family.starts_with("copy/") {
            let (value, citation) = episode
                .answer
                .split_once("입니다. ")
                .expect("own single-fact training answer");
            episode.answer = format!("기록 {citation}의 방향은 {value}입니다.");
        }
    }
    Ok(episodes)
}
// Whole-record targets keep entity, context and value together before the citation.
// These are supervised training targets, never a product answer formatter.
fn record_copy(count: usize, seed: u64, validation: bool) -> Result<Vec<Episode>> {
    let mut episodes = evidence_first(count, seed, validation)?;
    for (index, episode) in episodes.iter_mut().enumerate() {
        let mut fields = episode.binding.split('/');
        let entity = fields.next().expect("own entity binding");
        let context = fields.next().expect("own context binding");
        if episode.family.starts_with("copy/") {
            let prefix = format!("{entity}의 {context} 이동 지시는 ");
            let record = episode
                .request
                .evidence
                .items
                .iter()
                .find(|item| {
                    item.version_status == "current" && item.original_excerpt.starts_with(&prefix)
                })
                .expect("own auxiliary record");
            episode.request.input = if validation {
                format!(
                    "제시된 자료 중 {entity} {context}에 지금 적용되는 문장을 그대로 적은 뒤 근거를 달아줘."
                )
            } else {
                format!(
                    "{entity}의 {context}에서 현재 유효한 사건의 원문을 빠짐없이 쓰고 그 사건을 인용해줘."
                )
            };
            episode.answer = format!("{} [event:{}]", record.original_excerpt, record.event_id);
        } else if !validation
            && episode.family.contains("/initial-before-restore/")
            && (index / 24).is_multiple_of(2)
        {
            episode.request.input = format!(
                "{entity}의 {context} 자료에서 최초로 작성된 이동 지시를 근거와 함께 답해줘."
            );
        }
        if episode.category < 4 && !episode.family.starts_with("copy/") {
            let cited = replica_v3::app::citations(&episode.answer)
                .expect("own single-fact training citation");
            let record = episode
                .request
                .evidence
                .items
                .iter()
                .find(|item| item.event_id == cited[0])
                .expect("own supported training answer");
            episode.answer = format!("{} [event:{}]", record.original_excerpt, record.event_id);
        }
        episode.sequence = hash(
            &serde_json::to_vec(&(&episode.request.input, &episode.request.evidence.items))
                .expect("synthetic record-copy episode"),
        );
    }
    Ok(episodes)
}
// Short auxiliary supervision for the first context-dependent token. Ordinary QA
// is unchanged, and the product decoder cannot call this training-only renderer.
fn entity_cue(count: usize, seed: u64, validation: bool) -> Result<Vec<Episode>> {
    use replica_v3::neural::transformer::Rng;
    let mut episodes = record_copy(count, seed, validation)?;
    let names = ["장치", "설비", "센서", "장비"];
    let mut rng = Rng::new(seed ^ if validation { 0x42c14be7 } else { 0x7160d351 });
    let mut offset = 0;
    for (index, episode) in episodes.iter_mut().enumerate() {
        if index.is_multiple_of(4) {
            offset = (rng.next_u64() % names.len() as u64) as usize;
        }
        if !episode.family.starts_with("copy/") {
            continue;
        }
        let old_name: String = episode
            .binding
            .chars()
            .take_while(|c| !c.is_ascii_digit())
            .collect();
        let name = names[(offset + index % 4) % names.len()];
        let replacements = [(old_name, name.to_owned())];
        for item in &mut episode.request.evidence.items {
            item.original_excerpt =
                replace_training_literals(&item.original_excerpt, &replacements)?;
        }
        episode.binding = replace_training_literals(&episode.binding, &replacements)?;
        episode.request.input = if validation {
            "이 자료가 부르는 대상의 종류 이름만 적어줘."
        } else {
            "제공된 원문에서 숫자 앞에 적힌 대상의 분류명만 그대로 복사해줘."
        }
        .into();
        episode.answer = name.into();
        episode.family.push_str("/entity-cue");
        episode.sequence = hash(
            &serde_json::to_vec(&(&episode.request.input, &episode.request.evidence.items))
                .expect("synthetic entity-cue episode"),
        );
    }
    Ok(episodes)
}
// Direct supervision of source fields; ordinary QA and product inference stay unchanged.
fn field_cue(count: usize, seed: u64, validation: bool) -> Result<Vec<Episode>> {
    let mut episodes = entity_cue(count, seed, validation)?;
    let mut auxiliary = 0;
    for episode in &mut episodes {
        if !episode.family.starts_with("copy/") {
            continue;
        }
        let group = auxiliary / 4;
        // Rotate across the four underlying record layouts as well as target fields.
        let mode = (group + group / 4) % 4;
        auxiliary += 1;
        if mode == 0 {
            continue; // Preserve the already verified name-copy task and its wording.
        }
        let mut fields = episode.binding.split('/');
        let entity = fields.next().expect("own entity");
        let context = fields.next().expect("own context");
        let number = entity.trim_start_matches(|c: char| !c.is_ascii_digit());
        let prefix = format!("{entity}의 {context} 이동 지시는 ");
        let record = episode
            .request
            .evidence
            .items
            .iter()
            .find(|e| e.version_status == "current" && e.original_excerpt.starts_with(&prefix))
            .expect("own supported auxiliary record");
        let (question, answer, field) = match mode {
            1 => (
                if validation {
                    format!("{entity}({context})의 식별 숫자 부분만 보여줘.")
                } else {
                    format!("{context} 자료의 대상 {entity}에서 분류명 뒤 숫자만 원문대로 적어줘.")
                },
                number.to_owned(),
                "number",
            ),
            2 => (
                if validation {
                    format!("이 자료 중 {entity}에 대해 지정한 {context} 장소 문자열만 답하라.")
                } else {
                    let other = episode
                        .request
                        .evidence
                        .items
                        .iter()
                        .filter_map(|e| {
                            e.original_excerpt
                                .split_once("의 ")?
                                .1
                                .split_once(" 이동 지시는 ")
                                .map(|(c, _)| c)
                        })
                        .find(|c| *c != context);
                    if let Some(other) = other {
                        format!(
                            "{other}도 함께 적혀 있지만, {entity}의 {context} 위치 이름만 복사하라."
                        )
                    } else {
                        format!("제공 기록에서 {entity}의 {context} 위치 이름 전체만 복사하라.")
                    }
                },
                context.to_owned(),
                "context",
            ),
            _ => (
                if validation {
                    format!(
                        "번호 {number}인 대상의 {context}에 현재 적용되는 방향 값 한 단어만 답하라."
                    )
                } else {
                    format!(
                        "식별 숫자가 {number}인 대상의 {context} current 원문에서 '이동 지시는' 다음 방향 단어만 복사해줘."
                    )
                },
                record
                    .original_excerpt
                    .strip_prefix(&prefix)
                    .expect("own prefix")
                    .strip_suffix("이다.")
                    .expect("own suffix")
                    .to_owned(),
                "value",
            ),
        };
        episode.request.input = question;
        episode.answer = answer;
        episode.family.push_str(&format!("/{field}"));
        episode.sequence = hash(
            &serde_json::to_vec(&(&episode.request.input, &episode.request.evidence.items))
                .expect("synthetic field-cue episode"),
        );
    }
    Ok(episodes)
}
// Training-only wording diversity and controlled value changes. A value quartet
// shares the complete background and changes only its current supporting value.
fn field_pairs(count: usize, seed: u64) -> Result<Vec<Episode>> {
    use replica_v3::neural::transformer::Rng;
    let mut episodes = field_cue(count, seed, false)?;
    let mut rng = Rng::new(seed ^ 0x312bc745);
    for group in episodes.chunks_mut(4) {
        if !group[0].family.starts_with("copy/") {
            continue;
        }
        let base = group[0].clone();
        let form = rng.next_u64() % 8;
        let field = base.family.rsplit('/').next().expect("own field");
        if field == "value" {
            let mut parts = base.binding.split('/');
            let entity = parts.next().expect("own entity");
            let context = parts.next().expect("own context");
            let prefix = format!("{entity}의 {context} 이동 지시는 ");
            let target = base
                .request
                .evidence
                .items
                .iter()
                .position(|e| {
                    e.version_status == "current" && e.original_excerpt.starts_with(&prefix)
                })
                .expect("own supported value");
            for episode in group.iter_mut() {
                episode.request.evidence = base.request.evidence.clone();
                episode.request.evidence.items[target].original_excerpt =
                    format!("{prefix}{}이다.", episode.answer);
                episode.binding = format!("{entity}/{context}/{}", episode.answer);
            }
        }
        for episode in group {
            let mut parts = episode.binding.split('/');
            let entity = parts.next().expect("own entity");
            let context = parts.next().expect("own context");
            let number = entity.trim_start_matches(|c: char| !c.is_ascii_digit());
            episode.request.input = match field {
                "entity-cue" => match form {
                    0 => episode.request.input.clone(),
                    1 => "자료에 등장한 대상의 분류 이름만 알려줘.".into(),
                    2 => "기록 속 대상이 어떤 종류인지 이름만 복사하라.".into(),
                    3 => "원문의 숫자 바로 앞 명칭만 답해줘.".into(),
                    4 => "대상의 종류를 나타내는 단어 하나만 적어줘.".into(),
                    5 => "여기 나온 대상은 무슨 분류명으로 불리는가? 분류명만 답하라.".into(),
                    6 => "자료가 사용하는 대상의 종류 명칭만 보여줘.".into(),
                    _ => "제공된 원문에 적힌 대상 이름 중 숫자를 제외한 이름만 답하라.".into(),
                },
                "number" => match form {
                    0 => episode.request.input.clone(),
                    1 => format!("{entity}의 {context} 자료에서 식별 번호만 복사해줘."),
                    2 => format!("{context}의 {entity}에서 숫자 부분만 적어줘."),
                    3 => format!("대상 {entity}({context})의 번호 숫자만 답하라."),
                    4 => format!(
                        "{entity}의 이름 뒤 식별 숫자는? {context} 기록을 보고 숫자만 답해줘."
                    ),
                    5 => format!("{context} 자료에 나오는 {entity}의 식별 숫자만 보여줘."),
                    6 => format!("{entity}({context})를 식별하는 번호만 원문대로 적어줘."),
                    _ => format!(
                        "{context} 기록의 대상 {entity}에서 종류 이름을 빼고 숫자만 답하라."
                    ),
                },
                "context" => match form {
                    0 => episode.request.input.clone(),
                    1 => format!("{entity}의 {context} 자료에서 지정한 장소 이름만 적어줘."),
                    2 => format!("대상 {entity}에 대해 요청한 {context} 위치 문자열만 보여줘."),
                    3 => format!("{entity}({context})의 장소 명칭만 복사해줘."),
                    4 => format!("이 기록에서 {entity}의 {context}라는 위치 이름 전체를 답하라."),
                    5 => format!("{entity}의 자료 중 {context} 장소 이름만 원문대로 답해줘."),
                    6 => {
                        format!("{context}에 관한 {entity} 기록이다. 지정된 장소 문자열만 적어줘.")
                    }
                    _ => format!("여러 기록 중 {entity}의 {context} 위치 이름만 답하라."),
                },
                "value" => match form {
                    0 => base.request.input.clone(),
                    1 => format!("번호 {number}인 대상의 {context}에서 현재 방향 단어만 적어줘."),
                    2 => format!("{context} 자료 중 번호 {number}의 current 이동 방향만 답하라."),
                    3 => format!(
                        "식별 번호 {number}, 장소 {context}: 현재 적용되는 이동 지시의 값만 복사해줘."
                    ),
                    4 => {
                        format!("{context}의 번호 {number}에 지금 적용되는 방향 한 단어만 보여줘.")
                    }
                    5 => format!(
                        "이 자료에서 식별 숫자 {number}에 대해 {context}의 현재 방향 값만 적어줘."
                    ),
                    6 => format!(
                        "번호 {number}의 {context} 이동 지시는 현재 어느 방향인가? 방향만 답하라."
                    ),
                    _ => format!(
                        "{context}에서 대상 번호 {number}의 current 기록이 지정한 방향 단어만 복사하라."
                    ),
                },
                _ => unreachable!("own auxiliary field"),
            };
            episode.sequence = hash(
                &serde_json::to_vec(&(&episode.request.input, &episode.request.evidence.items))
                    .expect("synthetic paired field episode"),
            );
        }
    }
    Ok(episodes)
}
// A two-by-two counterfactual: same evidence, two requested targets; then swap
// only the two source values. Correct generation must use both question and evidence.
fn query_pairs(count: usize, seed: u64) -> Result<Vec<Episode>> {
    let mut episodes = field_pairs(count, seed)?;
    for group in episodes.chunks_mut(4) {
        if !group[0].family.starts_with("copy/") || !group[0].family.ends_with("/value") {
            continue;
        }
        let base = group[0].clone();
        let parts: Vec<_> = base.binding.split('/').collect();
        let target_prefix = format!("{}의 {} 이동 지시는 ", parts[0], parts[1]);
        let records = &base.request.evidence.items;
        assert_eq!(records.len(), 2, "own two-record auxiliary layout");
        let target = records
            .iter()
            .position(|e| {
                e.version_status == "current" && e.original_excerpt.starts_with(&target_prefix)
            })
            .expect("own supported value");
        let other = 1 - target;
        let parsed: Vec<_> = records
            .iter()
            .map(|record| {
                let (entity, rest) = record
                    .original_excerpt
                    .split_once("의 ")
                    .expect("own record entity");
                let (context, value) = rest
                    .split_once(" 이동 지시는 ")
                    .expect("own record context");
                (
                    entity,
                    context,
                    value.strip_suffix("이다.").expect("own record value"),
                )
            })
            .collect();
        assert_ne!(parsed[target].2, parsed[other].2, "own distinct directions");
        let number = |s: &str| {
            s.trim_start_matches(|c: char| !c.is_ascii_digit())
                .to_owned()
        };
        let mut replacements = vec![
            (number(parsed[target].0), number(parsed[other].0)),
            (parsed[target].1.to_owned(), parsed[other].1.to_owned()),
        ];
        if records[other].version_status == "superseded" {
            replacements.extend([
                ("current".into(), "superseded".into()),
                ("현재".into(), "과거".into()),
                ("지금".into(), "예전에".into()),
            ]);
        }
        let other_question = replace_training_literals(&base.request.input, &replacements)?;
        assert_ne!(
            other_question, base.request.input,
            "own distinct requested targets"
        );
        for (variant, episode) in group.iter_mut().enumerate() {
            let selected = if variant % 2 == 0 { target } else { other };
            let swapped = variant >= 2;
            episode.request.evidence = base.request.evidence.clone();
            if swapped {
                for (i, record) in episode.request.evidence.items.iter_mut().enumerate() {
                    record.original_excerpt = format!(
                        "{}의 {} 이동 지시는 {}이다.",
                        parsed[i].0,
                        parsed[i].1,
                        parsed[1 - i].2
                    );
                }
            }
            episode.request.input = if selected == target {
                base.request.input.clone()
            } else {
                other_question.clone()
            };
            episode.answer = parsed[if swapped { 1 - selected } else { selected }]
                .2
                .to_owned();
            episode.binding = format!(
                "{}/{}/{}",
                parsed[selected].0, parsed[selected].1, episode.answer
            );
            episode.sequence = hash(
                &serde_json::to_vec(&(&episode.request.input, &episode.request.evidence.items))
                    .expect("synthetic query-value pair"),
            );
        }
    }
    Ok(episodes)
}
pub fn prepare(
    root: &Path,
    seed: u64,
    documents: usize,
    local: &[PathBuf],
    profile: &str,
) -> Result<()> {
    if !(50..=20_000).contains(&documents) {
        return Err(Error::Invalid("documents 50..20000".into()));
    }
    let (mut train, validation, revision) = match profile {
        "v1" => (
            synthetic(documents, seed, false),
            synthetic((documents / 10).max(50), seed, true),
            GENERATOR_REVISION,
        ),
        "curriculum" => (
            curriculum(documents, seed, false, false),
            curriculum((documents / 10).clamp(50, 400), seed, true, false),
            "educational-korean-curriculum-v2",
        ),
        "balanced" => (
            curriculum(documents, seed, false, true),
            curriculum((documents / 10).clamp(50, 400), seed, true, true),
            "educational-korean-balanced-v3",
        ),
        "grounding" => (
            grounding(documents, seed, false),
            grounding((documents / 10).clamp(50, 400), seed, true),
            "educational-korean-grounding-v4",
        ),
        "counterfactual" => (
            counterfactual(documents, seed, false)?,
            counterfactual((documents / 10).clamp(50, 400), seed, true)?,
            "educational-korean-counterfactual-v5",
        ),
        "evidence-first" => (
            evidence_first(documents, seed, false)?,
            evidence_first((documents / 10).clamp(50, 400), seed, true)?,
            "educational-korean-evidence-first-v6",
        ),
        "record-copy" => (
            record_copy(documents, seed, false)?,
            record_copy((documents / 10).clamp(50, 400), seed, true)?,
            "educational-korean-record-copy-v8",
        ),
        "entity-cue" => (
            entity_cue(documents, seed, false)?,
            entity_cue((documents / 10).clamp(50, 400), seed, true)?,
            "educational-korean-entity-cue-v9",
        ),
        "field-cue" => (
            field_cue(documents, seed, false)?,
            field_cue((documents / 10).clamp(50, 400), seed, true)?,
            "educational-korean-field-cue-v10",
        ),
        "field-pairs" => (
            field_pairs(documents, seed)?,
            field_cue((documents / 10).clamp(50, 400), seed, true)?,
            "educational-korean-field-pairs-v11",
        ),
        "query-pairs" => (
            query_pairs(documents, seed)?,
            field_cue((documents / 10).clamp(50, 400), seed, true)?,
            "educational-korean-query-pairs-v12",
        ),
        _ => return Err(Error::Invalid("corpus profile".into())),
    };
    // Only explicitly supplied files. Each entire document belongs to one split;
    // no private DB discovery, recursive directory walk or automatic ingestion.
    for (i, p) in local.iter().enumerate() {
        let bytes = read_bounded(p, 262144)?;
        let text =
            String::from_utf8(bytes.clone()).map_err(|_| Error::Invalid("corpus UTF-8".into()))?;
        train.push(Episode {
            id: hash(&bytes),
            category: 4,
            family: "local-document".into(),
            binding: format!("local-{i}"),
            sequence: "local-document".into(),
            request: ModelRequest {
                request_id: format!("local-{i}"),
                system: String::new(),
                input: text,
                evidence: EvidenceBundle::default(),
                limits: GenerationLimits {
                    max_tokens: 128,
                    context_tokens: 2048,
                    timeout_ms: 120000,
                },
            },
            answer: String::new(),
        });
    }
    check_split(&train, &validation)?;
    let manifest=CorpusManifest{version:1,scope:if local.is_empty(){"SYNTHETIC_ONLY"}else{"SYNTHETIC_AND_EXPLICIT_LOCAL"}.into(),permission:"project-generated; supplied paths explicitly authorized for training".into(),generator:revision.into(),seed,split_rule:"episode first; disjoint entity binding, template family and sequence; final test created independently".into(),train:native::split("train",&train),validation:native::split("validation",&validation)};
    native::write(
        root,
        &native::from_episodes(manifest.clone(), train, validation.clone())?,
        true,
    )?;
    println!("{}", serde_json::to_string_pretty(&manifest)?);
    if matches!(
        profile,
        "counterfactual" | "evidence-first" | "record-copy" | "entity-cue" | "field-cue"
    ) {
        println!(
            "synthetic_base_scenes_train={} synthetic_base_scenes_validation={} variants_per_base_max=4; variants are not independent questions",
            documents.div_ceil(4),
            validation.len().div_ceil(4)
        );
    }
    Ok(())
}
/// Full-answer contrasts from existing training scenes; the development split stays unchanged.
/// Keep the full training population, but make two-record direction QA depend on
/// both the requested target and its record's value. Never used by the worker.
pub fn binding_pairs(source: &Path, output: &Path) -> Result<()> {
    let (mut manifest, mut train, validation) = load(source)?;
    if !train.len().is_multiple_of(4) {
        return Err(Error::Invalid(
            "binding pairs require complete existing quartets".into(),
        ));
    }
    let mut changed = 0;
    for group in train.chunks_mut(4) {
        let base = group[0].clone();
        if base.family.starts_with("copy/") || !matches!(base.category, 0 | 2) {
            continue;
        }
        if group
            .iter()
            .any(|e| e.category != base.category || e.request.input != base.request.input)
            || base.request.evidence.items.len() != 2
            || base
                .request
                .evidence
                .items
                .iter()
                .any(|r| r.version_status != "current" || r.excerpt_truncated)
        {
            return Err(Error::Invalid("binding pair source layout".into()));
        }
        let parsed: Vec<_> = base
            .request
            .evidence
            .items
            .iter()
            .map(|r| {
                r.original_excerpt
                    .split_once("의 ")
                    .and_then(|(entity, rest)| {
                        rest.split_once(" 이동 지시는 ")
                            .and_then(|(context, value)| {
                                value
                                    .strip_suffix("이다.")
                                    .map(|value| (entity, context, value))
                            })
                    })
                    .ok_or_else(|| Error::Invalid("binding pair source grammar".into()))
            })
            .collect::<Result<_>>()?;
        if parsed.iter().any(|(entity, context, value)| {
            [*entity, *context, *value]
                .iter()
                .any(|field| field.is_empty() || !field.chars().all(char::is_alphanumeric))
        }) {
            return Err(Error::Invalid(
                "binding pair empty or malformed fact atom".into(),
            ));
        }
        let cited = replica_v3::app::citations(&base.answer)?;
        let target = base
            .request
            .evidence
            .items
            .iter()
            .position(|r| cited == [r.event_id])
            .ok_or_else(|| Error::Invalid("binding pair single source citation".into()))?;
        if base.answer
            != format!(
                "{} [event:{}]",
                base.request.evidence.items[target].original_excerpt, cited[0]
            )
            || !base.request.input.contains(parsed[target].0)
            || !base.request.input.contains(parsed[target].1)
            || parsed[0].2 == parsed[1].2
        {
            return Err(Error::Invalid("binding pair source target".into()));
        }
        let other = 1 - target;
        let mut replacements = Vec::new();
        for field in 0..2 {
            let a = if field == 0 {
                parsed[target].0
            } else {
                parsed[target].1
            };
            let b = if field == 0 {
                parsed[other].0
            } else {
                parsed[other].1
            };
            if a != b {
                replacements.push((a.to_owned(), b.to_owned()));
                replacements.push((b.to_owned(), a.to_owned()));
            }
        }
        // Longest-first prevents context1 from matching the prefix of context10.
        replacements.sort_by_key(|(old, _)| std::cmp::Reverse(old.len()));
        let other_question = replace_training_literals(&base.request.input, &replacements)?;
        if other_question == base.request.input {
            return Err(Error::Invalid("binding pair questions must differ".into()));
        }
        for (variant, episode) in group.iter_mut().enumerate() {
            let selected = if variant % 2 == 0 { target } else { other };
            let swapped = variant >= 2;
            episode.request.evidence = base.request.evidence.clone();
            if swapped {
                for (i, record) in episode.request.evidence.items.iter_mut().enumerate() {
                    record.original_excerpt = format!(
                        "{}의 {} 이동 지시는 {}이다.",
                        parsed[i].0,
                        parsed[i].1,
                        parsed[1 - i].2
                    );
                }
            }
            episode.request.input = if selected == target {
                base.request.input.clone()
            } else {
                other_question.clone()
            };
            let record = &episode.request.evidence.items[selected];
            episode.answer = format!("{} [event:{}]", record.original_excerpt, record.event_id);
            episode.binding = format!(
                "{}/{}/{}",
                parsed[selected].0,
                parsed[selected].1,
                parsed[if swapped { 1 - selected } else { selected }].2
            );
            episode.family.push_str("/query-value-pair");
            episode.sequence = hash(&serde_json::to_vec(&(
                &episode.request.input,
                &episode.request.evidence.items,
            ))?);
        }
        changed += group.len();
    }
    if changed == 0 {
        return Err(Error::Invalid("no supported binding pair quartets".into()));
    }
    check_split(&train, &validation)?;
    let parent_train = manifest.train.sha256.clone();
    manifest.generator.push_str("/full-binding-pairs-v1");
    manifest.split_rule = format!(
        "Same training count; {changed} existing two-current-record QA0/QA2 cases become query/value pairs; other cases and validation bytes unchanged; parent train={parent_train}; {}",
        manifest.split_rule
    );
    native::write(
        output,
        &native::from_episodes(manifest.clone(), train, validation)?,
        true,
    )?;
    println!("{}", serde_json::to_string_pretty(&manifest)?);
    Ok(())
}
pub fn qa_pairs(source: &Path, output: &Path, groups: usize) -> Result<()> {
    let (mut manifest, original, validation) = load(source)?;
    if !(1..=128).contains(&groups)
        || manifest.generator != "educational-korean-query-pairs-v12"
        || !original.len().is_multiple_of(4)
    {
        return Err(Error::Invalid(
            "QA pairs require query-pairs corpus and 1..128 groups".into(),
        ));
    }
    let blocks = original.as_chunks::<4>().0;
    let paired: Vec<_> = blocks
        .iter()
        .filter(|g| {
            g.iter()
                .all(|e| e.family.starts_with("copy/") && e.family.ends_with("/value"))
        })
        .take(groups)
        .collect();
    let ordinary: Vec<_> = blocks
        .iter()
        .filter(|g| {
            g.iter()
                .all(|e| !e.family.starts_with("copy/") && !e.answer.is_empty())
        })
        .take(groups)
        .collect();
    if paired.len() != groups || ordinary.len() != groups {
        return Err(Error::Invalid(
            "insufficient existing QA pair groups".into(),
        ));
    }
    let mut train = Vec::with_capacity(groups * 16);
    for (index, (paired, ordinary)) in paired.into_iter().zip(ordinary).enumerate() {
        for (kind, group) in [("binding", paired), ("ordinary", ordinary)] {
            let mut base = group.clone();
            if kind == "binding" {
                for e in &mut base {
                    if e.request.evidence.items.len() != 2 {
                        return Err(Error::Invalid(
                            "QA pair requires two full source records".into(),
                        ));
                    }
                    // Supervised target construction only. The worker imports neither
                    // this module nor these expected answers/record-selection labels.
                    let mut matching = Vec::new();
                    for r in &e.request.evidence.items {
                        let fields = r
                            .original_excerpt
                            .split_once("의 ")
                            .and_then(|(entity, rest)| {
                                rest.split_once(" 이동 지시는 ")
                                    .and_then(|(context, value)| {
                                        value
                                            .strip_suffix("이다.")
                                            .map(|value| (entity, context, value))
                                    })
                            })
                            .ok_or_else(|| Error::Invalid("QA pair source grammar".into()))?;
                        if r.excerpt_truncated {
                            return Err(Error::Invalid("QA pair truncated source".into()));
                        }
                        if fields.2 == e.answer {
                            matching.push((r, fields.0, fields.1));
                        }
                    }
                    if matching.len() != 1 {
                        return Err(Error::Invalid("QA pair ambiguous source value".into()));
                    }
                    let (record, entity, context) = matching[0];
                    let prefix = format!("{entity}의 {context} 이동 지시는 ");
                    if e.request
                        .evidence
                        .items
                        .iter()
                        .filter(|r| {
                            r.version_status == record.version_status
                                && r.original_excerpt.starts_with(&prefix)
                        })
                        .count()
                        != 1
                    {
                        return Err(Error::Invalid("QA pair ambiguous requested record".into()));
                    }
                    e.category = if e
                        .request
                        .evidence
                        .items
                        .iter()
                        .any(|r| r.version_status == "superseded")
                    {
                        1
                    } else if e
                        .request
                        .evidence
                        .items
                        .iter()
                        .all(|r| r.original_excerpt.starts_with(&format!("{entity}의 ")))
                    {
                        2
                    } else {
                        0
                    };
                    let when = match record.version_status.as_str() {
                        "current" => "현재",
                        "superseded" => "과거",
                        _ => return Err(Error::Invalid("QA pair source status".into())),
                    };
                    e.request.input = match index % 3 {
                        0 => format!("{entity} {context} {when} 이동 지시는 무엇인가?"),
                        1 => format!("{context}에서 {entity}의 {when} 방향과 근거를 알려줘."),
                        _ => format!(
                            "{entity}의 {context} {when} 기록에 적힌 지시를 사건과 함께 답해줘."
                        ),
                    };
                    e.request.system = SYSTEM.into();
                    e.answer = format!("{} [event:{}]", record.original_excerpt, record.event_id);
                    e.family = format!("qa/binding-pairs/train/{index}");
                }
            }
            for reversed in [false, true] {
                for mut e in base.clone() {
                    e.id = format!("qa-pairs/{kind}/{reversed}/{}", e.id);
                    e.request.request_id = e.id.clone();
                    if reversed {
                        e.request.evidence.items.reverse();
                    }
                    e.sequence = hash(&serde_json::to_vec(&(
                        &e.request.input,
                        &e.request.evidence.items,
                    ))?);
                    train.push(e);
                }
            }
        }
    }
    check_split(&train, &validation)?;
    manifest.split_rule = format!(
        "DEVELOPMENT; {groups} existing query/value quartets converted to full QA and {groups} unchanged ordinary QA quartets, each in both evidence orders; no new base scenes; eight consecutive cases per sampling group; parent train={} validation={}; validation unchanged; {}",
        manifest.train.sha256, manifest.validation.sha256, manifest.split_rule
    );
    manifest.generator.push_str("/qa-binding-pairs-v1");
    native::write(
        output,
        &native::from_episodes(manifest.clone(), train, validation)?,
        true,
    )?;
    println!("{}", serde_json::to_string_pretty(&manifest)?);
    Ok(())
}
/// A diagnostic subset of existing QA bytes, never a new heldout benchmark.
pub fn qa_subset(source: &Path, output: &Path, count: usize) -> Result<()> {
    if !(16..=32).contains(&count) {
        return Err(Error::Invalid(
            "memorization subset requires 16..32 QA".into(),
        ));
    }
    let (mut manifest, train, validation) = load(source)?;
    let select = |episodes: Vec<Episode>| -> Result<Vec<Episode>> {
        let selected: Vec<_> = episodes
            .into_iter()
            .filter(|e| !e.answer.is_empty() && !e.family.starts_with("copy/"))
            .take(count)
            .collect();
        if selected.len() != count {
            return Err(Error::Invalid("insufficient ordinary QA for subset".into()));
        }
        Ok(selected)
    };
    let train = select(train)?;
    let validation = select(validation)?;
    check_split(&train, &validation)?;
    manifest.split_rule = format!(
        "MEMORIZATION_DIAGNOSTIC_ONLY; first {count} nonempty non-copy QA from each existing split; episodes unchanged; parent train={} validation={}; {}",
        manifest.train.sha256, manifest.validation.sha256, manifest.split_rule
    );
    native::write(
        output,
        &native::from_episodes(manifest.clone(), train, validation)?,
        true,
    )?;
    println!("{}", serde_json::to_string_pretty(&manifest)?);
    Ok(())
}
/// Materialized H3 education, isolated from inference. Reuses the existing corpus format.
pub fn copy_curriculum(source: &Path, output: &Path, seed: u64) -> Result<()> {
    use replica_v3::neural::transformer::Rng;
    let (parent, original, validation) = load_legacy(source)?;
    let mut rng = Rng::new(seed);
    let shuffle = |indices: &mut Vec<usize>, rng: &mut Rng| {
        for i in (1..indices.len()).rev() {
            indices.swap(i, (rng.next_u64() % (i + 1) as u64) as usize);
        }
    };
    let mut anchors = Vec::new();
    let mut bases = BTreeSet::new();
    for (category, count) in [410, 410, 410, 409, 409].into_iter().enumerate() {
        let mut indices: Vec<_> = original
            .iter()
            .enumerate()
            .filter(|(_, e)| e.category == category && !e.family.starts_with("copy/"))
            .map(|(i, _)| i)
            .collect();
        shuffle(&mut indices, &mut rng);
        let selected: Vec<_> = indices
            .into_iter()
            .filter(|i| {
                bases.insert(
                    original[*i]
                        .id
                        .rsplit_once('/')
                        .map_or(original[*i].id.as_str(), |(base, _)| base)
                        .to_owned(),
                )
            })
            .take(count)
            .collect();
        if selected.len() != count {
            return Err(Error::Invalid(
                "distinct ordinary anchor bases unavailable".into(),
            ));
        }
        anchors.extend(selected.into_iter().map(|i| original[i].clone()));
    }
    let original_entities: BTreeSet<_> = original
        .iter()
        .chain(&validation)
        .flat_map(|e| e.request.evidence.items.iter())
        .filter_map(|r| {
            r.original_excerpt
                .split_once("의 ")
                .map(|(entity, _)| entity.to_owned())
        })
        .collect();
    let prefixes = ["장치", "설비", "센서", "장비"];
    let directions = [
        "오른쪽",
        "왼쪽",
        "직진",
        "대기",
        "북쪽",
        "남쪽",
        "동쪽",
        "서쪽",
    ];
    let bucket = |entity: &str| -> usize {
        usize::from_str_radix(&hash(entity.as_bytes())[..8], 16).expect("hex digest") % 3
    };
    let mut splits = Vec::new();
    for (split_index, (split, base_count)) in [("train", 512), ("dev", 64), ("seal", 64)]
        .into_iter()
        .enumerate()
    {
        let mut episodes = Vec::new();
        for base in 0..base_count {
            let digits = base % 8 + 1;
            let shape = (base / 8) % 4;
            let mut pair = None;
            for _ in 0..10_000 {
                let prefix = prefixes[(rng.next_u64() % 4) as usize];
                let mut number: Vec<u8> = (0..digits)
                    .map(|_| b'0' + (rng.next_u64() % 10) as u8)
                    .collect();
                if shape == 1 && digits > 1 {
                    number[0] = b'0';
                }
                if shape == 2 {
                    let d = number[0];
                    number.fill(d);
                }
                let entity = format!("{prefix}{}", std::str::from_utf8(&number).expect("digits"));
                if bucket(&entity) != split_index || original_entities.contains(&entity) {
                    continue;
                }
                for difference in 1..10 {
                    let mut near = number.clone();
                    near[digits - 1] = b'0' + (number[digits - 1] - b'0' + difference) % 10;
                    let other = format!("{prefix}{}", std::str::from_utf8(&near).expect("digits"));
                    if bucket(&other) == split_index && !original_entities.contains(&other) {
                        pair = Some((entity.clone(), other));
                        break;
                    }
                }
                if pair.is_some() {
                    break;
                }
            }
            let (entity, renamed) =
                pair.ok_or_else(|| Error::Invalid("copy identifier allocation exhausted".into()))?;
            let context = format!("구역{}", 1 + rng.next_u64() % 999_999);
            let event_id = (1 + rng.next_u64() % 999_999) as i64;
            let recorded_at = (rng.next_u64() % 1_000_000_000) as i64;
            let kind = if base.is_multiple_of(2) {
                "direction"
            } else {
                "short-value"
            };
            let value = if kind == "direction" {
                directions[(rng.next_u64() % 8) as usize].to_string()
            } else {
                format!("경로{}", rng.next_u64() % 100_000)
            };
            let mut changed = if kind == "direction" {
                directions[(rng.next_u64() % 8) as usize].to_string()
            } else {
                format!("경로{}", rng.next_u64() % 100_000)
            };
            if changed == value {
                changed = if kind == "direction" {
                    directions[(directions.iter().position(|v| *v == value).unwrap() + 1) % 8]
                        .into()
                } else {
                    format!("경로{}", 100_000 + rng.next_u64() % 100_000)
                };
            }
            for view in 0..4 {
                let entity = if view == 1 { &renamed } else { &entity };
                let value = if view == 2 { &changed } else { &value };
                let record = format!("{entity}의 {context} 이동 지시는 {value}이다.");
                let id = format!("skill-H3/{split}/{seed}/{base}/{view}");
                let mut item = evidence(event_id, record.clone(), "current");
                item.recorded_at = recorded_at;
                let question = if view == 3 {
                    "제공된 유일한 기록의 원문을 빠짐없이 쓰고 그 사건을 인용해줘.".into()
                } else {
                    format!(
                        "{entity}의 {context}에서 현재 유효한 사건의 원문을 빠짐없이 쓰고 그 사건을 인용해줘."
                    )
                };
                let request = ModelRequest {
                    request_id: id.clone(),
                    system: SYSTEM.into(),
                    input: question,
                    evidence: EvidenceBundle {
                        items: vec![item],
                        ..Default::default()
                    },
                    limits: GenerationLimits {
                        context_tokens: 2048,
                        max_tokens: 128,
                        timeout_ms: 120000,
                    },
                };
                episodes.push(Episode {
                    id,
                    category: 0,
                    family: format!(
                        "skill/H3/{split}/digits-{digits}/{kind}/shape-{shape}/view-{view}"
                    ),
                    binding: format!("{entity}/{context}/{value}"),
                    sequence: hash(&serde_json::to_vec(&(
                        &request.input,
                        &request.evidence.items,
                    ))?),
                    request,
                    answer: format!("{record} [event:{event_id}]"),
                });
            }
        }
        splits.push(episodes);
    }
    for a in 0..3 {
        for b in a + 1..3 {
            check_split(&splits[a], &splits[b])?;
        }
    }
    // Identifier partition is independent of values/context and applies to every view.
    for (index, episodes) in splits.iter().enumerate() {
        if episodes
            .iter()
            .any(|e| bucket(e.binding.split('/').next().unwrap_or_default()) != index)
        {
            return Err(Error::Corrupt("copy identifier split".into()));
        }
    }
    let mut train = anchors;
    train.extend(splits[0].clone());
    check_split(&train, &splits[1])?;
    check_split(&train, &splits[2])?;
    std::fs::create_dir(output)?;
    let manifest = CorpusManifest {
        version: 1,
        scope: "SYNTHETIC_ONLY".into(),
        permission: "project-generated H3 and explicitly retained ordinary anchors".into(),
        generator: "educational-copy-H3-v1".into(),
        seed,
        split_rule: format!(
            "anchor first2048 (categories410/410/410/409/409), focus last2048; dev/seal64 bases x4; hash-partitioned complete entity strings; parent={}",
            parent.train.sha256
        ),
        train: save_split(output, "train", &train)?,
        validation: save_split(output, "validation", &splits[1])?,
    };
    let seal = save_split(output, "seal", &splits[2])?;
    write_new(
        &output.join("seal-manifest.json"),
        &serde_json::to_vec_pretty(&seal)?,
    )?;
    write_new(
        &output.join("manifest.json"),
        &serde_json::to_vec_pretty(&manifest)?,
    )?;
    println!(
        "H3_materialized anchors=2048 focus=2048 dev=256 seal=256 seed={seed} optimizer_updates=0"
    );
    Ok(())
}
/// One development-only crossed panel. Does not read or evaluate the sealed corpus.
pub fn crossed_copy_development(
    prior: &[Episode],
    seed: u64,
) -> Result<(Vec<Episode>, serde_json::Value)> {
    crossed_copy_panel(prior, seed, 128, false)
}
/// Frozen development contrasts only. Labels stay outside ModelRequest.
pub fn conditional_panel(prior: &[Episode], seed: u64) -> Result<(Vec<Episode>, Vec<String>)> {
    use replica_v3::neural::transformer::Rng;
    let mut rng = Rng::new(seed);
    let mut used: BTreeSet<String> = prior
        .iter()
        .flat_map(|e| e.request.evidence.items.iter())
        .filter_map(|e| {
            e.original_excerpt
                .split_once("의 ")
                .map(|(s, _)| s.to_owned())
        })
        .collect();
    let mut cases = Vec::new();
    let mut foils = Vec::new();
    for base in 0..24 {
        let digits = [2, 4, 6, 8][base % 4];
        let repeated = (base / 4) % 2 == 1;
        let kind = (base / 4 + base / 8) % 2;
        let mut entity = None;
        for _ in 0..10000 {
            let d = (rng.next_u64() % 10) as u8 + b'0';
            let mut number = vec![d; digits];
            if !repeated {
                for (i, n) in number.iter_mut().enumerate() {
                    *n = b'0' + ((d - b'0' + i as u8) % 10);
                }
            }
            let s = format!(
                "{}{}",
                ["장치", "설비", "센서", "장비"][(rng.next_u64() % 4) as usize],
                std::str::from_utf8(&number).unwrap()
            );
            if !used.contains(&s)
                && usize::from_str_radix(&hash(s.as_bytes())[..8], 16).unwrap() % 3 != 2
            {
                entity = Some(s);
                break;
            }
        }
        let entity = entity
            .ok_or_else(|| Error::Invalid("conditional panel unseen stratum capacity".into()))?;
        used.insert(entity.clone());
        let context = format!("구역{}", rng.next_u64() % 99999);
        let values = if kind == 0 {
            let directions = [
                "오른쪽",
                "왼쪽",
                "직진",
                "대기",
                "북쪽",
                "남쪽",
                "동쪽",
                "서쪽",
            ];
            let i = (rng.next_u64() % 8) as usize;
            [
                directions[i].to_string(),
                directions[(i + 1 + (rng.next_u64() % 7) as usize) % 8].to_string(),
            ]
        } else {
            let n = rng.next_u64() % 100000;
            [
                format!("경로{n:05}"),
                format!("경로{:05}", (n + 1 + rng.next_u64() % 99999) % 100000),
            ]
        };
        let current = 1 + (rng.next_u64() % 900000) as i64;
        let past = current + 1000000;
        for view in 0..6 {
            let swap = usize::from(view == 2);
            let selected = usize::from(view == 1);
            let mut records = Vec::new();
            for (i, id) in [current, past].into_iter().enumerate() {
                let original =
                    format!("{entity}의 {context} 이동 지시는 {}이다.", values[i ^ swap]);
                let mut record =
                    evidence(id, original, if i == 0 { "current" } else { "superseded" });
                record.recorded_at = if i == 0 { 200 } else { 100 };
                record.observed_at = Some(record.recorded_at - 1);
                records.push(record);
            }
            let answer = format!(
                "{} [event:{}]",
                records[selected].original_excerpt, records[selected].event_id
            );
            let foil = format!(
                "{} [event:{}]",
                records[1 - selected].original_excerpt,
                records[1 - selected].event_id
            );
            if view == 3 {
                records.reverse();
            }
            if view == 5 {
                records.push(evidence(
                    current + 2000000,
                    "기록 보관실의 점검 시간은 오후 세 시이다.".into(),
                    "current",
                ));
            }
            let input = match view {
                1 => format!(
                    "{entity}의 {context}에서 과거에 유효했던 사건의 원문을 빠짐없이 쓰고 그 사건을 인용해줘."
                ),
                4 => format!(
                    "{context}에 있는 {entity}에 대해 지금 적용되는 기록은 무엇인가요? 해당 원문 전체와 사건 인용을 알려주세요."
                ),
                _ => format!(
                    "{entity}의 {context}에서 현재 유효한 사건의 원문을 빠짐없이 쓰고 그 사건을 인용해줘."
                ),
            };
            let id = format!("conditional-development/{seed}/{base}/{view}");
            cases.push(Episode {
                id: id.clone(),
                category: 0,
                family: format!(
                    "conditional/digits-{digits}/repeated-{repeated}/kind-{kind}/view-{view}"
                ),
                binding: format!("{entity}/{context}/{}", values[selected ^ swap]),
                sequence: format!("conditional/{seed}/{base}"),
                answer,
                request: ModelRequest {
                    request_id: id,
                    system: SYSTEM.into(),
                    input,
                    evidence: EvidenceBundle {
                        items: records,
                        ..Default::default()
                    },
                    limits: GenerationLimits {
                        context_tokens: 2048,
                        max_tokens: 128,
                        timeout_ms: 30000,
                    },
                },
            });
            foils.push(foil);
        }
    }
    validate_episodes(&cases)?;
    Ok((cases, foils))
}
fn crossed_copy_panel(
    prior: &[Episode],
    seed: u64,
    bases: usize,
    training: bool,
) -> Result<(Vec<Episode>, serde_json::Value)> {
    use replica_v3::neural::transformer::Rng;
    use serde_json::json;
    let prefixes = ["장치", "설비", "센서", "장비"];
    let mut seen = BTreeSet::new();
    for e in prior {
        seen.insert(e.binding.split('/').next().unwrap_or_default().to_owned());
        for item in &e.request.evidence.items {
            if let Some((entity, _)) = item.original_excerpt.split_once("의 ") {
                seen.insert(entity.to_owned());
            }
        }
    }
    // Reserve the old seal's entire ID partition without reading its cases or answers.
    let available = |entity: &str| {
        !seen.contains(entity)
            && usize::from_str_radix(&hash(entity.as_bytes())[..8], 16).unwrap() % 3 != 2
    };
    let one_digit: Vec<_> = prefixes
        .iter()
        .flat_map(|p| (0..10).map(move |n| format!("{p}{n}")))
        .filter(|e| available(e))
        .collect();
    let mut meta = json!({"role":"DEVELOPMENT_ONLY","generator":"cross-copy-v1","seed":seed,
        "planned_bases":bases,"planned_views":bases*4,"one_digit_unseen_unreserved":one_digit,
        "reserved_old_seal":"entire hash partition2; cases unopened","max_attempts_per_base":10000,
        "one_digit_patterns":"general/repeated/alternating/adjacent coincide; not independent conditions",
        "split_scope":"full entity absent from supplied train/development; old seal namespace reserved",
        "status":"CAPACITY","optimizer_updates":0});
    let directions = [
        "오른쪽",
        "왼쪽",
        "직진",
        "대기",
        "북쪽",
        "남쪽",
        "동쪽",
        "서쪽",
    ];
    let mut rng = Rng::new(seed);
    let mut out = Vec::new();
    for base in 0..bases {
        let digits = (base % 128) / 16 + 1;
        let kind = (base / 8) % 2;
        let pattern = (base / 2) % 4;
        let mut pair = None;
        for _ in 0..10_000 {
            let prefix = prefixes[(rng.next_u64() % 4) as usize];
            let mut number: Vec<u8> = (0..digits)
                .map(|_| b'0' + (rng.next_u64() % 10) as u8)
                .collect();
            let first = number[0];
            match pattern {
                1 => number.fill(first),
                2 => {
                    let second = b'0' + ((first - b'0' + 1 + (rng.next_u64() % 9) as u8) % 10);
                    for (i, n) in number.iter_mut().enumerate() {
                        *n = if i % 2 == 0 { first } else { second };
                    }
                }
                3 if digits > 1 => {
                    number[1] = first;
                }
                _ => {}
            }
            if pattern == 0 && digits > 1 && base % 2 == 0 {
                number[0] = b'0';
            }
            let entity = format!("{prefix}{}", std::str::from_utf8(&number).unwrap());
            if !available(&entity) {
                continue;
            }
            for delta in 1..10 {
                let mut near = number.clone();
                near[digits - 1] = b'0' + (near[digits - 1] - b'0' + delta) % 10;
                let renamed = format!("{prefix}{}", std::str::from_utf8(&near).unwrap());
                if available(&renamed) {
                    pair = Some((entity.clone(), renamed));
                    break;
                }
            }
            if pair.is_some() {
                break;
            }
        }
        let Some((entity, renamed)) = pair else {
            meta["failed_stratum"] =
                json!({"digits":digits,"kind":kind,"pattern":pattern,"base":base});
            meta["constructed_views_not_a_gate"] = json!(out.len());
            // Never silently shrink the requested panel or count seen IDs as unseen.
            return Ok((Vec::new(), meta));
        };
        let context = format!("구역{}", 1 + rng.next_u64() % 999_999);
        let mut event_id = (1 + rng.next_u64() % 999_999) as i64;
        if training && base % 4 == 1 {
            // A disclosed training counterexample: the same number in two fields,
            // with the remaining examples independently assigned. Never an inference rule.
            event_id = context
                .strip_prefix("구역")
                .unwrap()
                .parse()
                .map_err(|_| Error::Invalid("context integer".into()))?;
        }
        let recorded_at = (rng.next_u64() % 1_000_000_000) as i64;
        let value = if kind == 0 {
            directions[(rng.next_u64() % 8) as usize].to_owned()
        } else {
            format!("경로{}", rng.next_u64() % 100_000)
        };
        let changed = if kind == 0 {
            let offset = 1 + (rng.next_u64() % 7) as usize;
            directions[(directions.iter().position(|v| *v == value).unwrap() + offset) % 8]
                .to_owned()
        } else {
            format!(
                "경로{}",
                (value.strip_prefix("경로").unwrap().parse::<u64>().unwrap()
                    + 1
                    + rng.next_u64() % 99_999)
                    % 100_000
            )
        };
        for view in 0..4 {
            let name = if view == 1 { &renamed } else { &entity };
            let val = if view == 2 { &changed } else { &value };
            let original = format!("{name}의 {context} 이동 지시는 {val}이다.");
            let mut item = evidence(event_id, original.clone(), "current");
            item.recorded_at = recorded_at;
            let namespace = if training {
                "train-renewal"
            } else {
                "development"
            };
            let id = format!("cross-H3/{namespace}/{seed}/{base}/{view}");
            let input = if view == 3 {
                "제공된 유일한 기록의 원문을 빠짐없이 쓰고 그 사건을 인용해줘.".into()
            } else {
                format!(
                    "{name}의 {context}에서 현재 유효한 사건의 원문을 빠짐없이 쓰고 그 사건을 인용해줘."
                )
            };
            let request = ModelRequest {
                request_id: id.clone(),
                system: SYSTEM.into(),
                input,
                evidence: EvidenceBundle {
                    items: vec![item],
                    ..Default::default()
                },
                limits: GenerationLimits {
                    context_tokens: 2048,
                    max_tokens: 128,
                    timeout_ms: 120000,
                },
            };
            out.push(Episode {
                id,
                category: 0,
                family: if training {format!("renewal/H3/train/digits-{digits}/kind-{kind}/pattern-{pattern}/replica-{}/view-{view}",base%2)}
                    else {format!("cross/H3/digits-{digits}/kind-{kind}/pattern-{pattern}/replica-{}/view-{view}",base%2)},
                binding: format!("{name}/{context}/{val}"),
                sequence: hash(&serde_json::to_vec(&(
                    &request.input,
                    &request.evidence.items,
                ))?),
                request,
                answer: format!("{original} [event:{event_id}]"),
            });
        }
    }
    check_split(prior, &out)?;
    meta["status"] = json!("MATERIALIZED_NOT_VALIDATED");
    meta["bases"] = json!(bases);
    meta["views"] = json!(out.len());
    meta["unique_entities"] = json!(
        out.iter()
            .map(|e| e.binding.split('/').next().unwrap())
            .collect::<BTreeSet<_>>()
            .len()
    );
    if training {
        meta["role"] = json!("TRAINING_ONLY");
        meta["generator"] = json!("controlled-renewal-H3-v1");
        meta["counterexample_same_number_context_event"] =
            json!("base%4==1; others independently sampled");
    }
    Ok((out, meta))
}
/// F is a fixed balanced quarter of N. Both use the same grammar and 128-stratum schedule.
pub fn renewed_copy_curricula(
    original: &(CorpusManifest, Vec<Episode>, Vec<Episode>),
    heldout: &[Episode],
    output: &Path,
    seed: u64,
) -> Result<serde_json::Value> {
    use serde_json::json;
    let (parent, old, dev) = original;
    if old.len() != 4096 || dev.len() != 256 {
        return Err(Error::Invalid("original H3 curriculum required".into()));
    }
    let (focus, meta) = crossed_copy_panel(heldout, seed, 512, true)?;
    if focus.len() != 2048 {
        return Err(Error::Invalid(format!("renewal capacity: {meta}")));
    }
    check_split(old, &focus)?;
    check_split(&focus, dev)?;
    let mut report = json!({"generator":meta,"original_train_hash":parent.train.sha256,"seed":seed,"arms":{},"optimizer_updates":0});
    for (arm, count) in [("F", 512), ("N", 2048)] {
        let dir = output.join(format!("corpus-{arm}"));
        std::fs::create_dir(&dir)?;
        let mut train = old[..2048].to_vec();
        train.extend_from_slice(&focus[..count]);
        check_split(&train, heldout)?;
        let manifest=CorpusManifest {version:1,scope:parent.scope.clone(),permission:"project synthetic training; original anchors preserved".into(),
            generator:"controlled-renewal-H3-v1".into(),seed,split_rule:"holdout full entity excluded; old seal partition reserved; original train binding/raw disjoint; base views kept together".into(),
            train:save_split(&dir,"train",&train)?,validation:save_split(&dir,"validation",dev)?};
        write_new(
            &dir.join("manifest.json"),
            &serde_json::to_vec_pretty(&manifest)?,
        )?;
        report["arms"][arm] = json!({"corpus":dir,"focus_views":count,"focus_bases":count/4,"train_hash":manifest.train.sha256,"dev_hash":manifest.validation.sha256});
    }
    Ok(report)
}
pub fn tokenizer(root: &Path, output: &Path, vocab: usize) -> Result<()> {
    let (mut manifest, train, validation) = load(root)?;
    let docs: Vec<Vec<u8>> = train
        .iter()
        .map(|e| {
            let mut s = e.request.system.clone();
            s.push_str(&e.request.input);
            for v in &e.request.evidence.items {
                s.push_str(&replica_v3::neural::evidence_text(v));
            }
            s.push_str(&e.answer);
            s.into_bytes()
        })
        .collect();
    let tok = ByteBpe::train(&docs, &manifest.train.sha256, vocab)?;
    let tokens: usize = docs
        .iter()
        .map(|d| tok.encode(d).map(|v| v.len()))
        .collect::<Result<Vec<_>>>()?
        .iter()
        .sum();
    tok.save(output)?;
    let count = |episodes: &[Episode]| -> Result<usize> {
        episodes
            .iter()
            .map(|e| {
                if e.answer.is_empty() {
                    return Ok(tok.encode(e.request.input.as_bytes())?.len());
                }
                let prompt = tok.prepare(&e.request, 2048, "tokenization-statistics")?;
                Ok(prompt.token_ids.len() + tok.encode(e.answer.as_bytes())?.len() + 1)
            })
            .sum()
    };
    manifest.train.tokens = Some(count(&train)?);
    manifest.validation.tokens = Some(count(&validation)?);
    write_new(
        &output.with_extension("manifest.json"),
        &serde_json::to_vec_pretty(&serde_json::json!({
            "tokenizer_sha256":tok.id(),"corpus":manifest,"training_text_bytes":docs.iter().map(Vec::len).sum::<usize>(),
            "training_text_tokens":tokens,"actual_vocab":tok.vocab_size()
        }))?,
    )?;
    println!(
        "tokenizer_sha256={} train_sha256={} documents={} bytes={} actual_tokens={} actual_vocab={}",
        tok.id(),
        manifest.train.sha256,
        docs.len(),
        docs.iter().map(Vec::len).sum::<usize>(),
        tokens,
        tok.vocab_size()
    );
    Ok(())
}
