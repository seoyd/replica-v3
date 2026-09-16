//! Explicit training tool only. This module is not linked by the inference library.
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
#[derive(Debug, Serialize, Deserialize)]
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
pub fn load_split(root: &Path, split: &Split) -> Result<Vec<Episode>> {
    if split.file.contains('/') || split.file.contains('\\') || split.file.starts_with('.') {
        return Err(Error::Invalid("split filename".into()));
    }
    let bytes = read_bounded(&root.join(&split.file), MAX_CORPUS)?;
    if hash(&bytes) != split.sha256 || bytes.len() != split.bytes {
        return Err(Error::Corrupt("corpus hash/size".into()));
    }
    let docs: Vec<Episode> = serde_json::from_slice(&bytes)?;
    if docs.len() != split.documents || docs.is_empty() || docs.len() > 100_000 {
        return Err(Error::Invalid("corpus count".into()));
    }
    let mut ids = BTreeSet::new();
    for e in &docs {
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
    Ok(docs)
}
pub fn load(root: &Path) -> Result<(CorpusManifest, Vec<Episode>, Vec<Episode>)> {
    let manifest: CorpusManifest =
        serde_json::from_slice(&read_bounded(&root.join("manifest.json"), 65536)?)?;
    if manifest.version != 1 {
        return Err(Error::Invalid("corpus version".into()));
    }
    let train = load_split(root, &manifest.train)?;
    let validation = load_split(root, &manifest.validation)?;
    check_split(&train, &validation)?;
    Ok((manifest, train, validation))
}
fn check_split(train: &[Episode], validation: &[Episode]) -> Result<()> {
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
pub fn prepare(root: &Path, seed: u64, documents: usize, local: &[PathBuf]) -> Result<()> {
    if !(50..=20_000).contains(&documents) {
        return Err(Error::Invalid("documents 50..20000".into()));
    }
    let (mut train, validation) = (
        synthetic(documents, seed, false),
        synthetic((documents / 10).max(50), seed, true),
    );
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
    std::fs::create_dir(root)?;
    let manifest=CorpusManifest{version:1,scope:if local.is_empty(){"SYNTHETIC_ONLY"}else{"SYNTHETIC_AND_EXPLICIT_LOCAL"}.into(),permission:"project-generated; supplied paths explicitly authorized for training".into(),generator:GENERATOR_REVISION.into(),seed,split_rule:"episode first; disjoint entity binding, template family and sequence; final test created independently".into(),train:save_split(root,"train",&train)?,validation:save_split(root,"validation",&validation)?};
    write_new(
        &root.join("manifest.json"),
        &serde_json::to_vec_pretty(&manifest)?,
    )?;
    println!("{}", serde_json::to_string_pretty(&manifest)?);
    Ok(())
}
pub fn tokenizer(root: &Path, output: &Path, vocab: usize) -> Result<()> {
    let (mut manifest, train, validation) = load(root)?;
    let docs: Vec<Vec<u8>> = train
        .iter()
        .map(|e| {
            let mut s = e.request.system.clone();
            s.push_str(&e.request.input);
            for v in &e.request.evidence.items {
                s.push_str(&v.original_excerpt);
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
