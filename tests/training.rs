use std::process::Command;
// Train-only reader shared by CLI regressions; it is not exported by inference.
#[allow(dead_code)]
#[path = "../src/data.rs"]
mod data;
// Preserve the existing field-by-field assertions in older corpus regressions.
// These bytes exist only in test memory; every source is read by the real native reader.
fn native_assertion_bytes(path: &std::path::Path, split: &str) -> Vec<u8> {
    let corpus = data::native::read(path).unwrap();
    match split {
        "train.json" => serde_json::to_vec(&corpus.train).unwrap(),
        "validation.json" => serde_json::to_vec(&corpus.validation).unwrap(),
        "manifest.json" => serde_json::to_vec(&corpus.manifest).unwrap(),
        _ => panic!("unknown test split"),
    }
}
#[test]
fn harness_m04_cli_close_reports_stopped_arm_before_any_replay() {
    let dir = tempfile::tempdir().unwrap();
    for name in ["C", "W"] {
        let path = dir.path().join(name);
        std::fs::create_dir(&path).unwrap();
        std::fs::write(path.join("result.json"), br#"{"reason":"CANCELLED","comparison_eligible":false,"final_evaluation_complete":false}"#).unwrap();
    }
    let output = dir.path().join("closed");
    let result = Command::new(env!("CARGO_BIN_EXE_replica-train"))
        .args([
            "recovery",
            "close",
            "--fixture",
            "absent-fixture",
            "--control",
            dir.path().join("C").to_str().unwrap(),
            "--treatment",
            dir.path().join("W").to_str().unwrap(),
            "--legacy-tokenizer",
            "absent-tokenizer",
            "--worker-binary",
            "absent-worker",
            "--source-id",
            &"1".repeat(64),
            "--output",
            output.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert!(!result.status.success());
    assert!(
        String::from_utf8(result.stderr)
            .unwrap()
            .contains("INELIGIBLE_OR_UNVERIFIED_ARM")
    );
    let summary: serde_json::Value =
        serde_json::from_slice(&std::fs::read(output.join("summary.json")).unwrap()).unwrap();
    assert_eq!(summary["comparison_eligible"], false);
    assert_eq!(summary["candidate_eligible"], false);
    assert_eq!(summary["model_calls"], 0);
    assert_eq!(summary["arm_terminals"][0]["reason"], "CANCELLED");
}
#[test]
fn harness_m01_m02_malformed_cli_rejects_before_output_or_model_load() {
    use serde_json::{Value, json};
    use std::{
        process::Stdio,
        time::{Duration, Instant},
    };
    let dir = tempfile::tempdir().unwrap();
    let source = dir.path().join("source");
    let prepared = Command::new(env!("CARGO_BIN_EXE_replica-train"))
        .args([
            "corpus",
            "prepare",
            "--profile",
            "entity-cue",
            "--documents",
            "240",
            "--seed",
            "317",
            "--output",
            source.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert!(prepared.status.success());
    let original_train = native_assertion_bytes(&source, "train.json");
    let original_validation = native_assertion_bytes(&source, "validation.json");
    let original_manifest: Value =
        serde_json::from_slice(&native_assertion_bytes(&source, "manifest.json")).unwrap();
    for (case, entity, context, value) in [
        ("entity", "", "구역1", "동쪽"),
        ("context", "센서31", "", "동쪽"),
        ("value", "센서31", "구역1", ""),
        ("space", "  ", "구역1", "동쪽"),
        ("delimiter", "센서/31", "구역1", "동쪽"),
        ("qa-name", "센서31", "구역1", "동쪽"),
    ] {
        let corpus = dir.path().join(case);
        let qa = case == "qa-name";
        let mut rows: Vec<Value> = serde_json::from_slice(if qa {
            &original_validation
        } else {
            &original_train
        })
        .unwrap();
        let start = rows
            .iter()
            .position(|e| e["category"] == 0 && !e["family"].as_str().unwrap().starts_with("copy/"))
            .unwrap();
        for row in &mut rows[start..start + if qa { 1 } else { 4 }] {
            row["request"]["input"] = json!(format!(
                "{}의 {context} 이동 지시와 근거는?",
                if qa { "장비31" } else { entity }
            ));
            row["binding"] = json!(format!("{entity}/{context}/{value}"));
            let records = row["request"]["evidence"]["items"].as_array_mut().unwrap();
            records.truncate(2);
            records[0]["original_excerpt"] =
                json!(format!("{entity}의 {context} 이동 지시는 {value}이다."));
            records[0]["version_status"] = json!("current");
            records[1]["original_excerpt"] = json!("장비32의 구역2 이동 지시는 서쪽이다.");
            records[1]["version_status"] = json!("current");
            let answer = format!(
                "{} [event:{}]",
                records[0]["original_excerpt"].as_str().unwrap(),
                records[0]["event_id"]
            );
            row["answer"] = json!(answer);
        }
        let changed = serde_json::to_vec(&rows).unwrap();
        let split = if qa { "validation" } else { "train" };
        let fixture = data::native::from_episodes(
            serde_json::from_value(original_manifest.clone()).unwrap(),
            serde_json::from_slice(if qa { &original_train } else { &changed }).unwrap(),
            serde_json::from_slice(if qa { &changed } else { &original_validation }).unwrap(),
        )
        .unwrap();
        data::native::write(&corpus, &fixture, true).unwrap();
        let before = std::fs::read(&corpus).unwrap();
        let output = dir.path().join(format!("{case}-output"));
        let err_path = dir.path().join(format!("{case}.stderr"));
        let mut command = Command::new(env!("CARGO_BIN_EXE_replica-train"));
        if qa {
            command.args([
                "evaluate",
                "--checkpoint",
                "nonexistent-model-must-not-be-read",
                "--corpus",
                corpus.to_str().unwrap(),
                "--single-qa-record",
                "--limit",
                "1",
            ]);
        } else {
            command.args([
                "corpus",
                "binding-pairs",
                "--source",
                corpus.to_str().unwrap(),
            ]);
        }
        let mut child = command
            .args(["--output", output.to_str().unwrap()])
            .stdout(Stdio::null())
            .stderr(std::fs::File::create(&err_path).unwrap())
            .spawn()
            .unwrap();
        let began = Instant::now();
        let status = loop {
            if let Some(status) = child.try_wait().unwrap() {
                break status;
            }
            if began.elapsed() > Duration::from_secs(5) {
                child.kill().unwrap();
                child.wait().unwrap();
                panic!("unbounded malformed CLI: {case}");
            }
            std::thread::sleep(Duration::from_millis(10));
        };
        assert!(!status.success(), "{case}");
        let error = std::fs::read_to_string(err_path).unwrap();
        assert!(
            error.contains(if qa {
                "INVALID_DIAGNOSTIC_INPUT"
            } else {
                "empty or malformed fact atom"
            }),
            "{case}: {error}"
        );
        assert!(!output.exists(), "malformed input published output");
        assert_eq!(
            serde_json::from_slice::<Value>(&native_assertion_bytes(
                &corpus,
                &format!("{split}.json")
            ))
            .unwrap(),
            serde_json::from_slice::<Value>(&changed).unwrap()
        );
        assert_eq!(std::fs::read(&corpus).unwrap(), before);
    }
    assert_eq!(
        native_assertion_bytes(&source, "train.json"),
        original_train
    );
    assert_eq!(
        native_assertion_bytes(&source, "validation.json"),
        original_validation
    );
}
#[test]
fn ordinary_qa_ablation_cli_keeps_gold_and_distinguishes_question_from_record() {
    use replica_v3::neural::{
        ByteBpe, checkpoint, hash,
        transformer::{Config, Transformer},
    };
    let dir = tempfile::tempdir().unwrap();
    let corpus = dir.path().join("corpus");
    let tokenizer = dir.path().join("tokenizer");
    let artifact = dir.path().join("random-native");
    let run = |args: &[&str]| {
        let out = Command::new(env!("CARGO_BIN_EXE_replica-train"))
            .args(args)
            .output()
            .unwrap();
        assert!(
            out.status.success(),
            "{}",
            String::from_utf8_lossy(&out.stderr)
        );
        out
    };
    run(&[
        "corpus",
        "prepare",
        "--profile",
        "entity-cue",
        "--documents",
        "240",
        "--seed",
        "317",
        "--output",
        corpus.to_str().unwrap(),
    ]);
    run(&[
        "tokenizer",
        "train",
        "--corpus",
        corpus.to_str().unwrap(),
        "--output",
        tokenizer.to_str().unwrap(),
    ]);
    let tok = ByteBpe::load(&tokenizer).unwrap();
    // Use the real native forward path with small test tensors and a full prompt.
    // Quality is evaluated separately with the unchanged trained SMALL artifact.
    let mut config = Config::tiny(tok.vocab_size());
    config.profile = "NATIVE_TRPP_EXPERIMENTAL_V1".into();
    config.context = 2048;
    let model = Transformer::init(config, 73, candle_core::Device::Cpu).unwrap();
    checkpoint::save(
        &artifact,
        &model,
        &tok,
        checkpoint::initialized(&model, &tok, 73, hash(b"ablation regression")).unwrap(),
        &Default::default(),
    )
    .unwrap();
    let artifact_hash = hash(&std::fs::read(&artifact).unwrap());
    let validation_bytes = native_assertion_bytes(&corpus, "validation.json");
    let validation: Vec<serde_json::Value> = serde_json::from_slice(&validation_bytes).unwrap();
    let cases: Vec<_> = validation
        .iter()
        .filter(|r| {
            matches!(r["category"].as_u64(), Some(0 | 2))
                && !r["family"].as_str().unwrap().starts_with("copy/")
        })
        .take(8)
        .collect();
    assert!(cases.iter().any(|r| r["category"] == 2));
    for (name, question, record) in [
        ("question", true, false),
        ("record", false, true),
        ("both", true, true),
    ] {
        let output = dir.path().join(name);
        let mut args = vec![
            "evaluate",
            "--checkpoint",
            artifact.to_str().unwrap(),
            "--corpus",
            corpus.to_str().unwrap(),
            "--output",
            output.to_str().unwrap(),
            "--limit",
            "8",
        ];
        if question {
            args.push("--known-question-form");
        }
        if record {
            args.push("--single-qa-record");
        }
        run(&args);
        let rows: Vec<serde_json::Value> = std::fs::read_to_string(output)
            .unwrap()
            .lines()
            .map(|s| serde_json::from_str(s).unwrap())
            .collect();
        assert_eq!(rows[0]["oracle_question_ablation"], question);
        assert_eq!(rows[0]["oracle_record_selection"], record);
        assert_eq!(rows[0]["oracle_field_task_label"], false);
        assert_eq!(rows.last().unwrap()["terminal_reason"], "COMPLETED");
        assert_eq!(rows.last().unwrap()["candidate_eligible"], false);
        let generated: Vec<_> = rows.iter().filter(|r| r.get("id").is_some()).collect();
        assert_eq!(generated.len(), cases.len());
        for (case, row) in cases.iter().zip(generated) {
            assert_eq!(row["id"], case["id"]);
            assert_eq!(row["expected"], case["answer"]);
            assert_eq!(row["question"], case["request"]["input"]);
            assert_eq!(row["evidence"], case["request"]["evidence"]);
            assert_eq!(row["generation_started"], true);
            assert_eq!(row["generated_question"] != row["question"], question);
            if record {
                let selected = row["generated_evidence"]["items"].as_array().unwrap();
                assert_eq!(selected.len(), 1);
                let cited = replica_v3::app::citations(case["answer"].as_str().unwrap()).unwrap();
                let expected_record = case["request"]["evidence"]["items"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .find(|r| r["event_id"] == cited[0])
                    .unwrap();
                assert_eq!(&selected[0], expected_record);
            } else {
                assert_eq!(row["generated_evidence"], row["evidence"]);
            }
        }
    }
    for incompatible in ["--single-current-record", "--known-field-question-form"] {
        let rejected = dir.path().join("rejected");
        let out = Command::new(env!("CARGO_BIN_EXE_replica-train"))
            .args([
                "evaluate",
                "--checkpoint",
                artifact.to_str().unwrap(),
                "--corpus",
                corpus.to_str().unwrap(),
                "--output",
                rejected.to_str().unwrap(),
                "--single-qa-record",
                incompatible,
            ])
            .output()
            .unwrap();
        assert!(!out.status.success());
        assert!(!rejected.exists());
    }
    assert_eq!(
        validation_bytes,
        native_assertion_bytes(&corpus, "validation.json")
    );
    assert_eq!(artifact_hash, hash(&std::fs::read(artifact).unwrap()));
}
#[test]
fn full_population_binding_pairs_require_question_and_value_without_split_growth() {
    let dir = tempfile::tempdir().unwrap();
    let source = dir.path().join("source");
    let output = dir.path().join("paired");
    let run = |args: &[&str]| {
        let out = Command::new(env!("CARGO_BIN_EXE_replica-train"))
            .args(args)
            .output()
            .unwrap();
        assert!(
            out.status.success(),
            "{}",
            String::from_utf8_lossy(&out.stderr)
        );
    };
    run(&[
        "corpus",
        "prepare",
        "--profile",
        "entity-cue",
        "--documents",
        "384",
        "--seed",
        "317",
        "--output",
        source.to_str().unwrap(),
    ]);
    let raw = std::fs::read(&source).unwrap();
    let original_corpus = data::native::read(&source).unwrap();
    let original = &original_corpus.train;
    run(&[
        "corpus",
        "binding-pairs",
        "--source",
        source.to_str().unwrap(),
        "--output",
        output.to_str().unwrap(),
    ]);
    assert_eq!(std::fs::read(&source).unwrap(), raw);
    let paired_corpus = data::native::read(&output).unwrap();
    assert_eq!(
        data::native::ordered_bytes(&original_corpus.validation),
        data::native::ordered_bytes(&paired_corpus.validation)
    );
    let paired = &paired_corpus.train;
    assert_eq!(paired.len(), original.len());
    let mut changed = 0;
    for (old, new) in original.chunks(4).zip(paired.chunks(4)) {
        if !matches!(old[0].category, 0 | 2) || old[0].family.starts_with("copy/") {
            assert_eq!(
                data::native::ordered_bytes(old),
                data::native::ordered_bytes(new)
            );
            continue;
        }
        changed += 4;
        assert_eq!(new[0].request.input, new[2].request.input);
        assert_eq!(new[1].request.input, new[3].request.input);
        assert_ne!(new[0].request.input, new[1].request.input);
        assert_eq!(new[0].request.evidence, new[1].request.evidence);
        assert_eq!(new[2].request.evidence, new[3].request.evidence);
        assert_ne!(new[0].answer, new[1].answer);
        assert_ne!(new[0].answer, new[2].answer);
        for (before, after) in old.iter().zip(new) {
            assert_eq!(before.id, after.id);
            assert_eq!(before.request.system, after.request.system);
            assert_eq!(before.request.limits, after.request.limits);
        }
        for row in new {
            let question = row.request.input.as_str();
            let affirmative = question.split_once(" 말고 ").map_or(question, |(_, s)| s);
            let records = &row.request.evidence.items;
            // Resolve from serialized query/records, not the generator's selected index,
            // label, binding metadata, or the other rows in this quartet.
            let selected: Vec<_> = records
                .iter()
                .filter(|r| {
                    let (entity, rest) = r.original_excerpt.split_once("의 ").unwrap();
                    let context = rest.split_once(" 이동 지시는 ").unwrap().0;
                    affirmative.contains(entity)
                        && (row.category == 0
                            || ["에서", " 이동", "의"]
                                .iter()
                                .any(|suffix| affirmative.contains(&format!("{context}{suffix}"))))
                })
                .collect();
            assert_eq!(selected.len(), 1, "{question}");
            let event = selected[0];
            assert_eq!(
                row.answer,
                format!("{} [event:{}]", event.original_excerpt, event.event_id)
            );
        }
        for (a, b) in new[0]
            .request
            .evidence
            .items
            .iter()
            .zip(&new[2].request.evidence.items)
        {
            let mut b = b.clone();
            b.original_excerpt = a.original_excerpt.clone();
            assert_eq!(
                *a, b,
                "value swaps must preserve ID/status/time/order metadata"
            );
        }
    }
    assert!(changed > 0);
    assert!(changed < paired.len());
    assert_eq!(
        paired_corpus.manifest.train.sha256,
        replica_v3::neural::hash(&data::native::ordered_bytes(paired))
    );
    // A malformed/non-quartet source is rejected before the output is created.
    let rejected = dir.path().join("rejected");
    let result = Command::new(env!("CARGO_BIN_EXE_replica-train"))
        .args([
            "corpus",
            "binding-pairs",
            "--source",
            output.to_str().unwrap(),
            "--output",
            rejected.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert!(!result.status.success());
    assert!(!rejected.exists());
}
#[test]
fn full_qa_pairs_keep_split_and_bind_question_value_citation_in_both_orders() {
    let d = tempfile::tempdir().unwrap();
    let source = d.path().join("source");
    let output = d.path().join("paired");
    let run = |args: &[&str]| {
        let out = Command::new(env!("CARGO_BIN_EXE_replica-train"))
            .args(args)
            .output()
            .unwrap();
        assert!(
            out.status.success(),
            "{}",
            String::from_utf8_lossy(&out.stderr)
        );
    };
    run(&[
        "corpus",
        "prepare",
        "--profile",
        "query-pairs",
        "--documents",
        "3840",
        "--seed",
        "317",
        "--output",
        source.to_str().unwrap(),
    ]);
    run(&[
        "corpus",
        "qa-pairs",
        "--source",
        source.to_str().unwrap(),
        "--output",
        output.to_str().unwrap(),
        "--groups",
        "8",
    ]);
    assert_eq!(
        native_assertion_bytes(&source, "validation.json"),
        native_assertion_bytes(&output, "validation.json")
    );
    let original: Vec<serde_json::Value> =
        serde_json::from_slice(&native_assertion_bytes(&source, "train.json")).unwrap();
    let train: Vec<serde_json::Value> =
        serde_json::from_slice(&native_assertion_bytes(&output, "train.json")).unwrap();
    assert_eq!(train.len(), 128);
    for group in train.as_chunks::<8>().0 {
        for i in 0..4 {
            let a = &group[i];
            let b = &group[i + 4];
            assert_eq!(a["answer"], b["answer"]);
            assert_eq!(a["request"]["input"], b["request"]["input"]);
            assert_eq!(a["request"]["system"], b["request"]["system"]);
            let mut reversed = a["request"]["evidence"]["items"]
                .as_array()
                .unwrap()
                .clone();
            reversed.reverse();
            assert_eq!(
                serde_json::json!(reversed),
                b["request"]["evidence"]["items"]
            );
            if let Some(id) = a["id"]
                .as_str()
                .unwrap()
                .strip_prefix("qa-pairs/ordinary/false/")
            {
                let old = original.iter().find(|e| e["id"] == id).unwrap();
                assert_eq!(a["answer"], old["answer"]);
                assert_eq!(a["request"]["input"], old["request"]["input"]);
                assert_eq!(a["request"]["evidence"], old["request"]["evidence"]);
            } else {
                assert_eq!(a["request"]["system"], replica_v3::model::SYSTEM);
                let question = a["request"]["input"].as_str().unwrap();
                let wanted_status = if question.contains("과거") {
                    "superseded"
                } else {
                    "current"
                };
                // Derive the support from the serialized question/records, independently of
                // the answer, binding, generator choice, and position within this group.
                let support: Vec<_> = a["request"]["evidence"]["items"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .filter(|r| {
                        let raw = r["original_excerpt"].as_str().unwrap();
                        let (entity, rest) = raw.split_once("의 ").unwrap();
                        let (context, _) = rest.split_once(" 이동 지시는 ").unwrap();
                        question.contains(entity)
                            && question.contains(context)
                            && r["version_status"] == wanted_status
                    })
                    .collect();
                assert_eq!(support.len(), 1);
                let id = support[0]["event_id"].as_i64().unwrap();
                let answer = a["answer"].as_str().unwrap();
                assert_eq!(replica_v3::app::citations(answer).unwrap(), [id]);
                assert_eq!(
                    answer.strip_suffix(&format!(" [event:{id}]")),
                    support[0]["original_excerpt"].as_str()
                );
                assert_eq!(group[0]["request"]["input"], group[2]["request"]["input"]);
                assert_eq!(group[1]["request"]["input"], group[3]["request"]["input"]);
                assert_ne!(group[0]["request"]["input"], group[1]["request"]["input"]);
                assert_eq!(
                    group[0]["request"]["evidence"],
                    group[1]["request"]["evidence"]
                );
                assert_eq!(
                    group[2]["request"]["evidence"],
                    group[3]["request"]["evidence"]
                );
            }
        }
    }
    let out = Command::new(env!("CARGO_BIN_EXE_replica-train"))
        .args([
            "corpus",
            "qa-pairs",
            "--source",
            source.to_str().unwrap(),
            "--output",
            output.to_str().unwrap(),
            "--groups",
            "129",
        ])
        .output()
        .unwrap();
    assert!(!out.status.success());
    assert_eq!(
        train,
        serde_json::from_slice::<Vec<serde_json::Value>>(&native_assertion_bytes(
            &output,
            "train.json"
        ))
        .unwrap()
    );
}
#[test]
fn query_pairs_require_question_and_evidence_with_validation_unchanged() {
    let dir = tempfile::tempdir().unwrap();
    for profile in ["field-pairs", "query-pairs"] {
        let result = Command::new(env!("CARGO_BIN_EXE_replica-train"))
            .args([
                "corpus",
                "prepare",
                "--profile",
                profile,
                "--documents",
                "3840",
                "--seed",
                "317",
                "--output",
            ])
            .arg(dir.path().join(profile))
            .output()
            .unwrap();
        assert!(
            result.status.success(),
            "{}",
            String::from_utf8_lossy(&result.stderr)
        );
    }
    let read =
        |profile: &str, split: &str| native_assertion_bytes(&dir.path().join(profile), split);
    assert_eq!(
        read("field-pairs", "validation.json"),
        read("query-pairs", "validation.json")
    );
    let before: Vec<serde_json::Value> =
        serde_json::from_slice(&read("field-pairs", "train.json")).unwrap();
    let after: Vec<serde_json::Value> =
        serde_json::from_slice(&read("query-pairs", "train.json")).unwrap();
    let mut layouts = std::collections::BTreeSet::new();
    for (old, group) in before
        .as_chunks::<4>()
        .0
        .iter()
        .zip(after.as_chunks::<4>().0)
    {
        let family = group[0]["family"].as_str().unwrap();
        if !(family.starts_with("copy/") && family.ends_with("/value")) {
            assert_eq!(old, group, "other training tasks remain unchanged");
            continue;
        }
        layouts.insert(family.split('/').nth(3).unwrap().to_owned());
        assert_eq!(old[0], group[0], "original target is retained");
        assert_eq!(
            group[0]["request"]["evidence"],
            group[1]["request"]["evidence"]
        );
        assert_eq!(
            group[2]["request"]["evidence"],
            group[3]["request"]["evidence"]
        );
        assert_ne!(group[0]["request"]["input"], group[1]["request"]["input"]);
        assert_eq!(group[0]["request"]["input"], group[2]["request"]["input"]);
        assert_eq!(group[1]["request"]["input"], group[3]["request"]["input"]);
        assert_ne!(group[0]["answer"], group[1]["answer"]);
        assert_eq!(group[0]["answer"], group[3]["answer"]);
        assert_eq!(group[1]["answer"], group[2]["answer"]);
        for (previous, episode) in old.iter().zip(group) {
            for key in ["id", "family", "category"] {
                assert_eq!(previous[key], episode[key]);
            }
            let question = episode["request"]["input"].as_str().unwrap();
            let parts: Vec<_> = episode["binding"].as_str().unwrap().split('/').collect();
            assert!(question.contains(parts[0].trim_start_matches(|c: char| !c.is_ascii_digit())));
            assert!(question.contains(parts[1]));
            let past = ["과거", "예전에", "superseded"]
                .iter()
                .any(|s| question.contains(s));
            let evidence = episode["request"]["evidence"]["items"].as_array().unwrap();
            let matching: Vec<_> = evidence
                .iter()
                .filter(|r| {
                    r["version_status"] == if past { "superseded" } else { "current" }
                        && r["original_excerpt"]
                            .as_str()
                            .unwrap()
                            .starts_with(&format!("{}의 {} 이동 지시는 ", parts[0], parts[1]))
                })
                .collect();
            assert_eq!(matching.len(), 1);
            assert_eq!(
                matching[0]["original_excerpt"],
                format!(
                    "{}의 {} 이동 지시는 {}이다.",
                    parts[0],
                    parts[1],
                    episode["answer"].as_str().unwrap()
                )
            );
            assert!(!question.contains(episode["answer"].as_str().unwrap()));
            for (a, b) in old[0]["request"]["evidence"]["items"]
                .as_array()
                .unwrap()
                .iter()
                .zip(evidence)
            {
                let mut a = a.clone();
                let mut b = b.clone();
                a.as_object_mut().unwrap().remove("original_excerpt");
                b.as_object_mut().unwrap().remove("original_excerpt");
                assert_eq!(a, b, "record metadata and order stay fixed");
            }
        }
    }
    assert_eq!(layouts.len(), 4);
}
#[test]
fn field_pairs_change_only_training_questions_and_selected_value() {
    use std::collections::{BTreeMap, BTreeSet};
    let dir = tempfile::tempdir().unwrap();
    for profile in ["field-cue", "field-pairs"] {
        let result = Command::new(env!("CARGO_BIN_EXE_replica-train"))
            .args([
                "corpus",
                "prepare",
                "--profile",
                profile,
                "--documents",
                "3840",
                "--seed",
                "313",
                "--output",
            ])
            .arg(dir.path().join(profile))
            .output()
            .unwrap();
        assert!(
            result.status.success(),
            "{}",
            String::from_utf8_lossy(&result.stderr)
        );
    }
    let read =
        |profile: &str, split: &str| native_assertion_bytes(&dir.path().join(profile), split);
    assert_eq!(
        read("field-cue", "validation.json"),
        read("field-pairs", "validation.json")
    );
    let before: Vec<serde_json::Value> =
        serde_json::from_slice(&read("field-cue", "train.json")).unwrap();
    let after: Vec<serde_json::Value> =
        serde_json::from_slice(&read("field-pairs", "train.json")).unwrap();
    let mut forms = BTreeMap::<String, BTreeSet<String>>::new();
    let mut paired = 0;
    for (old_group, group) in before.chunks(4).zip(after.chunks(4)) {
        if !group[0]["family"].as_str().unwrap().starts_with("copy/") {
            assert_eq!(old_group, group, "ordinary training QA is unchanged");
            continue;
        }
        let field = group[0]["family"]
            .as_str()
            .unwrap()
            .rsplit('/')
            .next()
            .unwrap();
        for (old, new) in old_group.iter().zip(group) {
            for key in ["id", "family", "category", "answer"] {
                assert_eq!(old[key], new[key]);
            }
            for key in ["system", "request_id", "limits"] {
                assert_eq!(old["request"][key], new["request"][key]);
            }
            if field != "value" {
                assert_eq!(old["binding"], new["binding"]);
                assert_eq!(old["request"]["evidence"], new["request"]["evidence"]);
            }
            let parts: Vec<_> = new["binding"].as_str().unwrap().split('/').collect();
            let number = parts[0].trim_start_matches(|c: char| !c.is_ascii_digit());
            let question = new["request"]["input"].as_str().unwrap();
            let normalized = question
                .replace(parts[0], "ENTITY")
                .replace(parts[1], "CONTEXT")
                .replace(number, "NUMBER");
            forms.entry(field.into()).or_default().insert(normalized);
            let target_prefix = format!("{}의 {} 이동 지시는 ", parts[0], parts[1]);
            let record = new["request"]["evidence"]["items"]
                .as_array()
                .unwrap()
                .iter()
                .find(|e| {
                    e["version_status"] == "current"
                        && e["original_excerpt"]
                            .as_str()
                            .unwrap()
                            .starts_with(&target_prefix)
                })
                .unwrap();
            assert!(
                record["original_excerpt"]
                    .as_str()
                    .unwrap()
                    .contains(new["answer"].as_str().unwrap())
            );
            if field == "value" {
                assert!(!question.contains(new["answer"].as_str().unwrap()));
                assert_eq!(
                    record["original_excerpt"],
                    format!("{target_prefix}{}이다.", new["answer"].as_str().unwrap())
                );
            }
        }
        if field == "value" {
            paired += 1;
            let answers: BTreeSet<_> = group
                .iter()
                .map(|e| e["answer"].as_str().unwrap())
                .collect();
            assert_eq!(answers.len(), 4);
            let base = &old_group[0];
            let parts: Vec<_> = base["binding"].as_str().unwrap().split('/').collect();
            let expected_text = format!(
                "{}의 {} 이동 지시는 {}이다.",
                parts[0],
                parts[1],
                base["answer"].as_str().unwrap()
            );
            for episode in group {
                assert_eq!(episode["request"]["input"], group[0]["request"]["input"]);
                let mut normalized = episode["request"]["evidence"].clone();
                let matches: Vec<_> = normalized["items"]
                    .as_array_mut()
                    .unwrap()
                    .iter_mut()
                    .filter(|e| {
                        e["version_status"] == "current"
                            && e["original_excerpt"]
                                .as_str()
                                .unwrap()
                                .starts_with(&format!("{}의 {} 이동 지시는 ", parts[0], parts[1]))
                    })
                    .collect();
                assert_eq!(matches.len(), 1);
                for record in matches {
                    record["original_excerpt"] = expected_text.clone().into();
                }
                assert_eq!(
                    normalized, base["request"]["evidence"],
                    "all IDs, times, status, ordering and distractors remain identical"
                );
            }
        }
    }
    assert!(paired > 0);
    assert_eq!(forms.len(), 4);
    assert!(
        forms.values().all(|v| v.len() >= 6),
        "each field needs varied training wording: {forms:?}"
    );
}
#[test]
fn field_cue_targets_are_supported_and_ordinary_qa_is_unchanged() {
    let dir = tempfile::tempdir().unwrap();
    for profile in ["entity-cue", "field-cue"] {
        let result = Command::new(env!("CARGO_BIN_EXE_replica-train"))
            .args([
                "corpus",
                "prepare",
                "--profile",
                profile,
                "--documents",
                "3840",
                "--seed",
                "307",
                "--output",
            ])
            .arg(dir.path().join(profile))
            .output()
            .unwrap();
        assert!(
            result.status.success(),
            "{}",
            String::from_utf8_lossy(&result.stderr)
        );
    }
    for split in ["train.json", "validation.json"] {
        let read = |profile: &str| -> Vec<serde_json::Value> {
            serde_json::from_slice(&native_assertion_bytes(&dir.path().join(profile), split))
                .unwrap()
        };
        let before = read("entity-cue");
        let after = read("field-cue");
        let mut covered = std::collections::BTreeSet::new();
        let mut field_layouts = std::collections::BTreeSet::new();
        for (old, new) in before.iter().zip(&after) {
            if !old["family"].as_str().unwrap().starts_with("copy/") {
                assert_eq!(old, new, "ordinary QA, including validation, is immutable");
                continue;
            }
            for key in ["id", "binding", "category"] {
                assert_eq!(old[key], new[key]);
            }
            for key in ["system", "evidence", "limits", "request_id"] {
                assert_eq!(old["request"][key], new["request"][key]);
            }
            let field = new["family"].as_str().unwrap().rsplit('/').next().unwrap();
            covered.insert(field.to_owned());
            field_layouts.insert((
                field.to_owned(),
                old["family"]
                    .as_str()
                    .unwrap()
                    .split('/')
                    .nth(3)
                    .unwrap()
                    .to_owned(),
            ));
            let mut binding = new["binding"].as_str().unwrap().split('/');
            let entity = binding.next().unwrap();
            let context = binding.next().unwrap();
            let records = new["request"]["evidence"]["items"].as_array().unwrap();
            let supporting = records
                .iter()
                .find(|r| {
                    r["version_status"] == "current"
                        && r["original_excerpt"]
                            .as_str()
                            .unwrap()
                            .starts_with(&format!("{entity}의 {context} 이동 지시는 "))
                })
                .unwrap();
            let answer = new["answer"].as_str().unwrap();
            match field {
                "entity-cue" => assert_eq!(old, new),
                "number" => {
                    assert!(answer.bytes().all(|b| b.is_ascii_digit()));
                    assert_eq!(
                        entity.strip_prefix(old["answer"].as_str().unwrap()),
                        Some(answer)
                    );
                }
                "context" => assert_eq!(answer, context),
                "value" => {
                    assert_eq!(
                        supporting["original_excerpt"],
                        format!("{entity}의 {context} 이동 지시는 {answer}이다.")
                    );
                    assert!(
                        !new["request"]["input"].as_str().unwrap().contains(answer),
                        "value answers must come from evidence"
                    );
                }
                _ => panic!("unexpected auxiliary field"),
            }
        }
        assert_eq!(
            covered,
            ["entity-cue", "number", "context", "value"]
                .map(str::to_owned)
                .into()
        );
        assert_eq!(
            field_layouts.len(),
            16,
            "each field occurs with all four existing record layouts"
        );
        for group in after.as_chunks::<4>().0 {
            if group[0]["family"].as_str().unwrap().ends_with("/value") {
                assert!(
                    group
                        .iter()
                        .all(|e| e["request"]["input"] == group[0]["request"]["input"])
                );
                assert_eq!(
                    group
                        .iter()
                        .map(|e| e["answer"].as_str().unwrap())
                        .collect::<std::collections::BTreeSet<_>>()
                        .len(),
                    4,
                    "the same value question must require different evidence-supported answers"
                );
            }
        }
    }
}
#[test]
fn qa_memorization_subset_preserves_episodes_and_split_boundaries() {
    let dir = tempfile::tempdir().unwrap();
    let source = dir.path().join("source");
    let subset = dir.path().join("subset");
    let binary = env!("CARGO_BIN_EXE_replica-train");
    let prepared = Command::new(binary)
        .args([
            "corpus",
            "prepare",
            "--profile",
            "record-copy",
            "--documents",
            "240",
            "--seed",
            "293",
            "--output",
        ])
        .arg(&source)
        .output()
        .unwrap();
    assert!(
        prepared.status.success(),
        "{}",
        String::from_utf8_lossy(&prepared.stderr)
    );
    let run = |count: &str| {
        Command::new(binary)
            .args(["corpus", "subset", "--source"])
            .arg(&source)
            .arg("--output")
            .arg(&subset)
            .args(["--count", count])
            .output()
            .unwrap()
    };
    assert!(!run("33").status.success());
    assert!(
        !subset.exists(),
        "reject invalid count before creating files"
    );
    let selected = run("32");
    assert!(
        selected.status.success(),
        "{}",
        String::from_utf8_lossy(&selected.stderr)
    );
    let source_bytes = std::fs::read(&source).unwrap();
    let original = data::native::read(&source).unwrap();
    let selected = data::native::read(&subset).unwrap();
    let mut ids = std::collections::BTreeSet::new();
    for (old, rows, old_split, new_split) in [
        (
            &original.train,
            &selected.train,
            &original.manifest.train,
            &selected.manifest.train,
        ),
        (
            &original.validation,
            &selected.validation,
            &original.manifest.validation,
            &selected.manifest.validation,
        ),
    ] {
        let expected: Vec<_> = old
            .iter()
            .filter(|e| !e.answer.is_empty() && !e.family.starts_with("copy/"))
            .take(32)
            .cloned()
            .collect();
        assert_eq!(
            data::native::ordered_bytes(rows),
            data::native::ordered_bytes(&expected)
        );
        assert_eq!(new_split.documents, 32);
        assert_eq!(
            new_split.sha256,
            replica_v3::neural::hash(&data::native::ordered_bytes(rows))
        );
        assert!(selected.manifest.split_rule.contains(&old_split.sha256));
        for episode in rows {
            assert!(ids.insert(episode.id.clone()));
        }
    }
    assert_eq!(std::fs::read(&source).unwrap(), source_bytes);
    let before = std::fs::read(&subset).unwrap();
    assert!(
        !run("16").status.success(),
        "existing subset is never overwritten"
    );
    assert_eq!(before, std::fs::read(&subset).unwrap());
}
#[test]
fn entity_cue_auxiliary_pairs_require_evidence_and_preserve_ordinary_qa() {
    let dir = tempfile::tempdir().unwrap();
    for profile in ["record-copy", "entity-cue"] {
        let output = Command::new(env!("CARGO_BIN_EXE_replica-train"))
            .args([
                "corpus",
                "prepare",
                "--profile",
                profile,
                "--documents",
                "240",
                "--seed",
                "271",
                "--output",
            ])
            .arg(dir.path().join(profile))
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
    let names: std::collections::BTreeSet<_> = ["장치", "설비", "센서", "장비"].into();
    let mut copy_questions = Vec::new();
    for split in ["train.json", "validation.json"] {
        let read = |profile: &str| -> Vec<serde_json::Value> {
            serde_json::from_slice(&native_assertion_bytes(&dir.path().join(profile), split))
                .unwrap()
        };
        let original = read("record-copy");
        let revised = read("entity-cue");
        assert_eq!(original.len(), revised.len());
        let mut copied = 0;
        for (before, after) in original.iter().zip(&revised) {
            if !before["family"].as_str().unwrap().starts_with("copy/") {
                assert_eq!(
                    before, after,
                    "all ordinary QA, including validation, stays unchanged"
                );
                continue;
            }
            let answer = after["answer"].as_str().unwrap();
            assert!(names.contains(answer));
            let question = after["request"]["input"].as_str().unwrap();
            assert!(
                names.iter().all(|name| !question.contains(name)),
                "the question must not supply the label"
            );
            let old_binding = before["binding"].as_str().unwrap();
            let old_name: String = old_binding
                .chars()
                .take_while(|c| !c.is_ascii_digit())
                .collect();
            assert_eq!(
                after["binding"].as_str().unwrap().strip_prefix(answer),
                old_binding.strip_prefix(&old_name)
            );
            let old_rows = before["request"]["evidence"]["items"].as_array().unwrap();
            let new_rows = after["request"]["evidence"]["items"].as_array().unwrap();
            assert_eq!(old_rows.len(), new_rows.len());
            for (old, new) in old_rows.iter().zip(new_rows) {
                for field in [
                    "event_id",
                    "source",
                    "recorded_at",
                    "observed_at",
                    "version_status",
                ] {
                    assert_eq!(old[field], new[field]);
                }
                assert_eq!(
                    new["original_excerpt"]
                        .as_str()
                        .unwrap()
                        .strip_prefix(answer),
                    old["original_excerpt"]
                        .as_str()
                        .unwrap()
                        .strip_prefix(&old_name),
                    "only the supported name changes, retaining the remaining original bytes"
                );
            }
            if copied == 0 {
                copy_questions.push(question.to_owned());
            }
            copied += 1;
        }
        assert!(copied > 0);
        for group in revised.as_chunks::<4>().0 {
            if !group[0]["family"].as_str().unwrap().starts_with("copy/") {
                continue;
            }
            let answers: std::collections::BTreeSet<_> = group
                .iter()
                .map(|e| e["answer"].as_str().unwrap())
                .collect();
            assert_eq!(answers, names);
            assert!(
                group
                    .iter()
                    .all(|e| e["request"]["input"] == group[0]["request"]["input"])
            );
        }
    }
    assert_eq!(copy_questions.len(), 2);
    assert_ne!(
        copy_questions[0], copy_questions[1],
        "train and validation use different auxiliary wording"
    );
}
#[test]
fn record_copy_targets_keep_source_bytes_and_temporal_qa() {
    let dir = tempfile::tempdir().unwrap();
    for profile in ["evidence-first", "record-copy"] {
        let output = Command::new(env!("CARGO_BIN_EXE_replica-train"))
            .args([
                "corpus",
                "prepare",
                "--profile",
                profile,
                "--documents",
                "960",
                "--seed",
                "229",
                "--output",
            ])
            .arg(dir.path().join(profile))
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
    let mut copied = 0;
    let mut rephrased = 0;
    for split in ["train.json", "validation.json"] {
        let read = |profile: &str| -> Vec<serde_json::Value> {
            serde_json::from_slice(&native_assertion_bytes(&dir.path().join(profile), split))
                .unwrap()
        };
        let original = read("evidence-first");
        let revised = read("record-copy");
        assert_eq!(original.len(), revised.len());
        for (before, after) in original.iter().zip(&revised) {
            for field in ["evidence", "limits", "system", "request_id"] {
                assert_eq!(before["request"][field], after["request"][field]);
            }
            for field in ["id", "binding", "category", "family"] {
                assert_eq!(before[field], after[field]);
            }
            let answer = after["answer"].as_str().unwrap();
            let rows = after["request"]["evidence"]["items"].as_array().unwrap();
            let cited = replica_v3::app::citations(answer).unwrap();
            if after["family"].as_str().unwrap().starts_with("copy/") {
                assert_eq!(cited.len(), 1);
                let record = rows.iter().find(|e| e["event_id"] == cited[0]).unwrap();
                assert_eq!(record["version_status"], "current");
                let text = record["original_excerpt"].as_str().unwrap();
                assert_eq!(
                    answer.strip_prefix(text).unwrap(),
                    format!(" [event:{}]", cited[0])
                );
                let mut fields = after["binding"].as_str().unwrap().split('/');
                let question = after["request"]["input"].as_str().unwrap();
                for target in [fields.next().unwrap(), fields.next().unwrap()] {
                    assert!(text.contains(target));
                    assert!(question.contains(target));
                }
                copied += 1;
            } else {
                assert_eq!(
                    cited,
                    replica_v3::app::citations(before["answer"].as_str().unwrap()).unwrap()
                );
                if after["category"].as_u64().unwrap() < 4 {
                    assert_eq!(cited.len(), 1);
                    let record = rows.iter().find(|e| e["event_id"] == cited[0]).unwrap();
                    assert_eq!(
                        answer
                            .strip_prefix(record["original_excerpt"].as_str().unwrap())
                            .unwrap(),
                        format!(" [event:{}]", cited[0])
                    );
                } else {
                    assert_eq!(before["answer"], after["answer"]);
                }
                if before["request"]["input"] != after["request"]["input"] {
                    assert_eq!(split, "train.json", "validation QA stays independent");
                    assert_eq!(after["category"], 1);
                    assert!(
                        after["family"]
                            .as_str()
                            .unwrap()
                            .contains("/initial-before-restore/")
                    );
                    assert_eq!(cited.len(), 1);
                    let first = rows
                        .iter()
                        .min_by_key(|r| r["recorded_at"].as_i64().unwrap())
                        .unwrap();
                    assert_eq!(
                        first["event_id"], cited[0],
                        "the requested historical version stays unchanged"
                    );
                    rephrased += 1;
                }
            }
        }
        for group in revised.as_chunks::<4>().0 {
            assert!(
                group
                    .iter()
                    .all(|e| e["request"]["input"] == group[0]["request"]["input"])
            );
        }
    }
    assert!(copied > 0);
    assert!(rephrased > 0);
}
#[test]
fn evidence_first_training_preserves_questions_records_and_supported_answers() {
    let dir = tempfile::tempdir().unwrap();
    for profile in ["counterfactual", "evidence-first"] {
        let output = Command::new(env!("CARGO_BIN_EXE_replica-train"))
            .args([
                "corpus",
                "prepare",
                "--profile",
                profile,
                "--documents",
                "240",
                "--seed",
                "229",
                "--output",
            ])
            .arg(dir.path().join(profile))
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
    for split in ["train.json", "validation.json"] {
        let read = |profile: &str| -> Vec<serde_json::Value> {
            serde_json::from_slice(&native_assertion_bytes(&dir.path().join(profile), split))
                .unwrap()
        };
        let original = read("counterfactual");
        let revised = read("evidence-first");
        assert_eq!(original.len(), revised.len());
        let mut reordered = 0;
        for (before, after) in original.iter().zip(&revised) {
            for field in ["request", "id", "binding", "family", "sequence", "category"] {
                assert_eq!(
                    before[field], after[field],
                    "only training answer order changes"
                );
            }
            let a = before["answer"].as_str().unwrap();
            let b = after["answer"].as_str().unwrap();
            if before["category"].as_u64().unwrap() >= 4
                || before["family"].as_str().unwrap().starts_with("copy/")
            {
                assert_eq!(a, b, "copy and uncertainty/chronology tasks are preserved");
                continue;
            }
            let ids = replica_v3::app::citations(b).unwrap();
            assert_eq!(ids, replica_v3::app::citations(a).unwrap());
            assert_eq!(ids.len(), 1);
            let record = after["request"]["evidence"]["items"]
                .as_array()
                .unwrap()
                .iter()
                .find(|r| r["event_id"] == ids[0])
                .unwrap();
            let text = record["original_excerpt"].as_str().unwrap();
            let value = text
                .split("이동 지시는 ")
                .nth(1)
                .unwrap()
                .strip_suffix("이다.")
                .unwrap();
            assert!(
                b.contains(value),
                "same value must be supported by the cited record"
            );
            let cite_end = b.find(']').unwrap();
            assert!(
                b.find(value).unwrap() > cite_end,
                "value follows its event citation"
            );
            reordered += 1;
        }
        assert!(reordered > 0);
    }
}
#[test]
fn counterfactual_corpus_requires_evidence_for_identical_questions() {
    let dir = tempfile::tempdir().unwrap();
    let corpus = dir.path().join("counterfactual");
    let output = Command::new(env!("CARGO_BIN_EXE_replica-train"))
        .args([
            "corpus",
            "prepare",
            "--profile",
            "counterfactual",
            "--documents",
            "240",
            "--seed",
            "811",
            "--output",
        ])
        .arg(&corpus)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        String::from_utf8_lossy(&output.stdout).contains("variants are not independent questions")
    );
    let read = |file| -> Vec<serde_json::Value> {
        serde_json::from_slice(&native_assertion_bytes(&corpus, file)).unwrap()
    };
    let train = read("train.json");
    let validation = read("validation.json");
    assert_eq!(train.len(), 240);
    assert_eq!(validation.len(), 50);
    let mut changed_positions = std::collections::BTreeSet::new();
    for group in train.as_chunks::<4>().0 {
        let category = group[0]["category"].as_u64().unwrap();
        let copy = group[0]["family"].as_str().unwrap().starts_with("copy/");
        if category < 4 && !copy {
            let mut values = std::collections::BTreeSet::new();
            let mut citations = std::collections::BTreeSet::new();
            for episode in group {
                assert_eq!(episode["request"]["input"], group[0]["request"]["input"]);
                let answer = episode["answer"].as_str().unwrap();
                let cited = replica_v3::app::citations(answer).unwrap();
                assert_eq!(cited.len(), 1);
                citations.insert(cited[0]);
                let value = answer.split("입니다.").next().unwrap();
                values.insert(value);
                let rows = episode["request"]["evidence"]["items"].as_array().unwrap();
                let position = rows.iter().position(|e| e["event_id"] == cited[0]).unwrap();
                if category == 0 {
                    changed_positions.insert(position);
                }
                assert!(
                    rows[position]["original_excerpt"]
                        .as_str()
                        .unwrap()
                        .contains(value)
                );
            }
            assert_eq!(
                values.len(),
                4,
                "one question must have four different evidence-derived values"
            );
            assert_eq!(
                citations.len(),
                4,
                "one question must not predict a fixed event ID"
            );
        }
    }
    assert_eq!(changed_positions, [0, 1].into());
    for episode in train.iter().chain(&validation) {
        let answer = episode["answer"].as_str().unwrap();
        let rows = episode["request"]["evidence"]["items"].as_array().unwrap();
        let cited = replica_v3::app::citations(answer).unwrap();
        assert!(
            cited
                .iter()
                .all(|id| rows.iter().any(|e| e["event_id"] == *id))
        );
        if episode["category"] == 4 && rows.len() == 3 {
            assert_eq!(cited.len(), 3);
            assert!(answer.contains("확정되지"));
            let mut chronological: Vec<_> = rows.iter().collect();
            chronological.sort_by_key(|e| e["recorded_at"].as_i64().unwrap());
            let positions: Vec<_> = chronological
                .iter()
                .map(|e| answer.find(&format!("[event:{}]", e["event_id"])).unwrap())
                .collect();
            assert!(positions.windows(2).all(|p| p[0] < p[1]));
        }
        if episode["category"] == 1 && rows.len() == 3 {
            let mut chronological: Vec<_> = rows.iter().collect();
            chronological.sort_by_key(|e| e["recorded_at"].as_i64().unwrap());
            assert_eq!(
                chronological[0]["original_excerpt"],
                chronological[2]["original_excerpt"]
            );
        }
    }
    for field in ["id", "binding", "family", "sequence"] {
        let seen: std::collections::BTreeSet<_> =
            train.iter().map(|e| e[field].as_str().unwrap()).collect();
        assert!(
            validation
                .iter()
                .all(|e| !seen.contains(e[field].as_str().unwrap()))
        );
    }
}
#[test]
fn curriculum_corpus_keeps_copy_training_explicit_and_time_independent() {
    let dir = tempfile::tempdir().unwrap();
    let corpus = dir.path().join("curriculum");
    let out = Command::new(env!("CARGO_BIN_EXE_replica-train"))
        .args([
            "corpus",
            "prepare",
            "--profile",
            "curriculum",
            "--documents",
            "120",
            "--seed",
            "953",
            "--output",
        ])
        .arg(&corpus)
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let read = |name| -> Vec<serde_json::Value> {
        serde_json::from_slice(&native_assertion_bytes(&corpus, name)).unwrap()
    };
    let train = read("train.json");
    let validation = read("validation.json");
    assert_eq!(train.len(), 120);
    let copy: Vec<_> = train
        .iter()
        .filter(|e| e["family"].as_str().unwrap().starts_with("copy/"))
        .collect();
    assert_eq!(copy.len(), 20);
    assert!(copy.iter().any(|e| {
        e["answer"]
            .as_str()
            .unwrap()
            .bytes()
            .all(|b| b.is_ascii_digit())
    }));
    for e in train.iter().chain(&validation) {
        let ids: Vec<_> = e["request"]["evidence"]["items"]
            .as_array()
            .unwrap()
            .iter()
            .map(|r| {
                assert_ne!(r["recorded_at"], r["event_id"]);
                r["event_id"].as_i64().unwrap()
            })
            .collect();
        for cited in replica_v3::app::citations(e["answer"].as_str().unwrap()).unwrap() {
            assert!(ids.contains(&cited));
        }
    }
    for field in ["id", "family", "binding", "sequence"] {
        let train_fields: std::collections::BTreeSet<_> =
            train.iter().map(|e| e[field].as_str().unwrap()).collect();
        assert!(
            validation
                .iter()
                .all(|e| !train_fields.contains(e[field].as_str().unwrap()))
        );
    }
}
#[test]
fn balanced_corpus_varies_distractor_identity_context_and_version_order() {
    let dir = tempfile::tempdir().unwrap();
    let corpus = dir.path().join("balanced");
    let out = Command::new(env!("CARGO_BIN_EXE_replica-train"))
        .args([
            "corpus",
            "prepare",
            "--profile",
            "balanced",
            "--documents",
            "600",
            "--seed",
            "177",
            "--output",
        ])
        .arg(&corpus)
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let read = |file| -> Vec<serde_json::Value> {
        serde_json::from_slice(&native_assertion_bytes(&corpus, file)).unwrap()
    };
    let train = read("train.json");
    let validation = read("validation.json");
    let mut signs = std::collections::BTreeSet::new();
    let mut id_signs = std::collections::BTreeSet::new();
    let mut context_prefixes = std::collections::BTreeSet::new();
    let mut restored = 0;
    for episode in &train {
        if episode["family"].as_str().unwrap().starts_with("copy/") {
            assert_eq!(
                !episode["request"]["system"].as_str().unwrap().is_empty(),
                !replica_v3::app::citations(episode["answer"].as_str().unwrap())
                    .unwrap()
                    .is_empty(),
                "product citation instructions must not be paired with bare-copy targets"
            );
        }
        let rows = episode["request"]["evidence"]["items"].as_array().unwrap();
        if episode["category"] == 0 {
            let target =
                replica_v3::app::citations(episode["answer"].as_str().unwrap()).unwrap()[0];
            let correct = rows.iter().find(|r| r["event_id"] == target).unwrap();
            let other = rows.iter().find(|r| r["event_id"] != target).unwrap();
            let number = |row: &serde_json::Value| -> u64 {
                row["original_excerpt"]
                    .as_str()
                    .unwrap()
                    .split('의')
                    .next()
                    .unwrap()
                    .chars()
                    .filter(char::is_ascii_digit)
                    .collect::<String>()
                    .parse()
                    .unwrap()
            };
            signs.insert(number(correct).cmp(&number(other)));
            assert!(number(correct) >= 500_000 && number(other) >= 500_000);
            id_signs.insert(
                correct["event_id"]
                    .as_i64()
                    .unwrap()
                    .cmp(&other["event_id"].as_i64().unwrap()),
            );
        }
        if episode["category"] == 2 {
            for row in rows {
                let context = row["original_excerpt"]
                    .as_str()
                    .unwrap()
                    .split_whitespace()
                    .nth(1)
                    .unwrap();
                context_prefixes.insert(
                    context
                        .chars()
                        .take_while(|c| !c.is_ascii_digit())
                        .collect::<String>(),
                );
            }
        }
        if episode["category"] == 1 && rows.len() == 3 {
            let current = rows
                .iter()
                .find(|r| r["version_status"] == "current")
                .unwrap();
            assert!(rows.iter().any(|r| r["event_id"] != current["event_id"]
                && r["original_excerpt"] == current["original_excerpt"]));
            restored += 1;
        }
    }
    assert_eq!(
        signs,
        [std::cmp::Ordering::Less, std::cmp::Ordering::Greater].into()
    );
    assert_eq!(id_signs, signs);
    assert_eq!(
        context_prefixes,
        ["구역".to_string(), "통로".to_string(), "현장".to_string()].into()
    );
    assert!(restored > 0);
    for field in ["id", "binding", "family", "sequence"] {
        let seen: std::collections::BTreeSet<_> =
            train.iter().map(|e| e[field].as_str().unwrap()).collect();
        assert!(
            validation
                .iter()
                .all(|e| !seen.contains(e[field].as_str().unwrap()))
        );
    }
}

#[test]
fn grounding_corpus_teaches_binding_and_supported_sequence_without_runtime_renderer() {
    let dir = tempfile::tempdir().unwrap();
    let corpus = dir.path().join("grounding");
    let result = Command::new(env!("CARGO_BIN_EXE_replica-train"))
        .args([
            "corpus",
            "prepare",
            "--profile",
            "grounding",
            "--documents",
            "600",
            "--seed",
            "271",
            "--output",
        ])
        .arg(&corpus)
        .output()
        .unwrap();
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    let mut modes = std::collections::BTreeSet::new();
    let mut sequence = 0;
    let mut restored_past = 0;
    for filename in ["train.json", "validation.json"] {
        let rows: Vec<serde_json::Value> =
            serde_json::from_slice(&native_assertion_bytes(&corpus, filename)).unwrap();
        for row in rows {
            let family = row["family"].as_str().unwrap();
            let evidence = row["request"]["evidence"]["items"].as_array().unwrap();
            let answer = row["answer"].as_str().unwrap();
            if family.starts_with("copy/grounding/") {
                let mode = family.rsplit('/').next().unwrap();
                modes.insert(mode.to_string());
                assert_eq!(evidence.len(), 2);
                assert_eq!(row["request"]["system"], "");
                let binding: Vec<_> = row["binding"].as_str().unwrap().split('/').collect();
                let expected_text = format!(
                    "{}의 {} 이동 지시는 {}이다.",
                    binding[0], binding[1], binding[2]
                );
                let target = evidence
                    .iter()
                    .find(|r| r["original_excerpt"] == expected_text)
                    .unwrap();
                let distractor = evidence
                    .iter()
                    .find(|r| r["event_id"] != target["event_id"])
                    .unwrap();
                assert_ne!(target["original_excerpt"], distractor["original_excerpt"]);
                if mode == "2" {
                    assert_eq!(answer, binding[2]);
                } else {
                    assert_eq!(
                        answer.parse::<i64>().unwrap(),
                        target["event_id"].as_i64().unwrap()
                    );
                }
                if mode == "3" {
                    assert_eq!(target["version_status"], "current");
                    assert_eq!(distractor["version_status"], "superseded");
                    assert!(
                        target["recorded_at"].as_i64().unwrap()
                            > distractor["recorded_at"].as_i64().unwrap()
                    );
                }
            } else if family.ends_with("causal-sequence") {
                sequence += 1;
                let mut ids: Vec<_> = evidence
                    .iter()
                    .map(|e| e["event_id"].as_i64().unwrap())
                    .collect();
                ids.sort();
                assert_eq!(replica_v3::app::citations(answer).unwrap(), ids);
                assert!(
                    ["지시", "실행", "사고", "확정되지"]
                        .iter()
                        .all(|word| answer.contains(word))
                );
            } else if family.ends_with("initial-before-restore") {
                restored_past += 1;
                let first = evidence
                    .iter()
                    .min_by_key(|e| e["recorded_at"].as_i64().unwrap())
                    .unwrap();
                assert_eq!(
                    replica_v3::app::citations(answer).unwrap(),
                    [first["event_id"].as_i64().unwrap()]
                );
                assert!(evidence.iter().any(|e| e["version_status"] == "current"
                    && e["original_excerpt"] == first["original_excerpt"]
                    && e["event_id"] != first["event_id"]));
            }
        }
    }
    assert_eq!(
        modes,
        ["0".into(), "1".into(), "2".into(), "3".into()].into()
    );
    assert!(sequence > 0 && restored_past > 0);
}
#[test]
fn corpus_and_tokenizer_use_train_only_and_reject_split_leakage() {
    let d = tempfile::tempdir().unwrap();
    let corpus = d.path().join("corpus");
    let tokenizer = d.path().join("tokenizer.json");
    let run = |args: &[&str]| {
        Command::new(env!("CARGO_BIN_EXE_replica-train"))
            .args(args)
            .output()
            .unwrap()
    };
    let out = run(&[
        "corpus",
        "prepare",
        "--output",
        corpus.to_str().unwrap(),
        "--documents",
        "100",
        "--seed",
        "917",
    ]);
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let out = run(&[
        "tokenizer",
        "train",
        "--corpus",
        corpus.to_str().unwrap(),
        "--output",
        tokenizer.to_str().unwrap(),
    ]);
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let tok = replica_v3::neural::ByteBpe::load(&tokenizer).unwrap();
    let input = d.path().join("raw");
    let raw = "새 문장\0😀 <assistant>\n".as_bytes();
    std::fs::write(&input, raw).unwrap();
    let out = run(&[
        "tokenizer",
        "inspect",
        "--tokenizer",
        tokenizer.to_str().unwrap(),
        "--file",
        input.to_str().unwrap(),
    ]);
    assert!(out.status.success());
    let json: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(json["ids"], serde_json::json!(tok.encode(raw).unwrap()));
    assert_eq!(json["roundtrip_sha256"], replica_v3::neural::hash(raw));
    let original_bytes = std::fs::read(&corpus).unwrap();
    let original = data::native::read(&corpus).unwrap();
    let train = data::native::ordered_bytes(&original.train);
    assert_eq!(tok.train_hash, replica_v3::neural::hash(&train));
    // Native construction rejects leaked split content before publishing or tokenization.
    let leaked =
        data::native::from_episodes(original.manifest, original.train.clone(), original.train);
    assert!(leaked.unwrap_err().to_string().contains("leakage"));
    assert_eq!(std::fs::read(&corpus).unwrap(), original_bytes);
}

#[test]
fn native_training_resume_is_identical_in_fresh_processes() {
    use candle_core::Device;
    use replica_v3::neural::{
        ByteBpe, checkpoint, hash,
        transformer::{Config, Transformer},
    };
    let d = tempfile::tempdir().unwrap();
    let initial = d.path().join("initial");
    let tok = ByteBpe::train(
        &["가 나 다 오른쪽 왼쪽".as_bytes().to_vec(); 1],
        &hash(b"numeric training fixture"),
        300,
    )
    .unwrap();
    let model = Transformer::init(Config::tiny(tok.vocab_size()), 73, Device::Cpu).unwrap();
    checkpoint::save(
        &initial,
        &model,
        &tok,
        checkpoint::initialized(&model, &tok, 73, hash(b"fixture source")).unwrap(),
        &Default::default(),
    )
    .unwrap();
    let full = d.path().join("full");
    let partial = d.path().join("partial");
    let resumed = d.path().join("resumed");
    let run = |args: &[&str]| {
        let out = Command::new(env!("CARGO_BIN_EXE_replica-train"))
            .args(args)
            .output()
            .unwrap();
        assert!(
            out.status.success(),
            "{}\n{}",
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        );
        out
    };
    let common = [
        "--numeric-probe",
        "--seq-len",
        "64",
        "--steps",
        "6",
        "--warmup",
        "0",
        "--accumulation",
        "2",
        "--validate-every",
        "6",
        "--lr",
        "0.002",
    ];
    let mut args = vec![
        "train",
        "--checkpoint",
        initial.to_str().unwrap(),
        "--output",
        full.to_str().unwrap(),
    ];
    args.extend(common);
    run(&args);
    args[4] = partial.to_str().unwrap();
    args.extend(["--stop-after", "3"]);
    run(&args);
    let resume_path = partial.join("final");
    run(&[
        "train",
        "--resume",
        resume_path.to_str().unwrap(),
        "--output",
        resumed.to_str().unwrap(),
        "--numeric-probe",
    ]);
    let a = checkpoint::load(&full.join("final"), Device::Cpu, true).unwrap();
    let b = checkpoint::load(&resumed.join("final"), Device::Cpu, true).unwrap();
    assert_eq!(a.manifest.weights_sha256, b.manifest.weights_sha256);
    assert_eq!(a.optimizer.len(), a.model.vars.len() * 2);
    for (name, t) in &a.optimizer {
        assert_eq!(
            t.flatten_all().unwrap().to_vec1::<f32>().unwrap(),
            b.optimizer[name]
                .flatten_all()
                .unwrap()
                .to_vec1::<f32>()
                .unwrap(),
            "{name}"
        );
    }
    for (name, t) in &a.model.vars {
        assert_eq!(
            t.flatten_all().unwrap().to_vec1::<f32>().unwrap(),
            b.model.vars[name]
                .flatten_all()
                .unwrap()
                .to_vec1::<f32>()
                .unwrap(),
            "{name}"
        );
    }
    let probe = candle_core::Tensor::new(&[[8u32, 9, 10]], &Device::Cpu).unwrap();
    assert_eq!(
        a.model
            .forward(&probe, None)
            .unwrap()
            .flatten_all()
            .unwrap()
            .to_vec1::<f32>()
            .unwrap(),
        b.model
            .forward(&probe, None)
            .unwrap()
            .flatten_all()
            .unwrap()
            .to_vec1::<f32>()
            .unwrap()
    );
    let a_state = a.manifest.training.unwrap();
    let b_state = b.manifest.training.unwrap();
    assert_eq!(a_state.step, 6);
    assert_eq!(a_state.sampler_state, b_state.sampler_state);
    assert_eq!(a_state.consumed_tokens, b_state.consumed_tokens);
    assert_eq!(a_state.target_tokens, b_state.target_tokens);
    assert_eq!(a_state.validation_loss, b_state.validation_loss);
    assert_eq!(a_state.train_loss, b_state.train_loss);
    assert_eq!(a_state.config, b_state.config);
    assert_ne!(a.model.weight_hash().unwrap(), model.weight_hash().unwrap());
    // A new bounded run is explicit; ordinary resume above preserves the original
    // schedule exactly. Extension must preserve the actual optimizer/RNG at entry.
    let full_final = full.join("final");
    let extended = d.path().join("extended");
    let source_id = hash(b"explicit extension source fixture");
    run(&[
        "train",
        "--resume",
        full_final.to_str().unwrap(),
        "--output",
        extended.to_str().unwrap(),
        "--numeric-probe",
        "--extend-steps",
        "4",
        "--extend-lr",
        "0.001",
        "--extend-warmup",
        "2",
        "--source-id",
        &source_id,
    ]);
    let entry = checkpoint::load(&extended.join("start"), Device::Cpu, true).unwrap();
    assert_eq!(entry.manifest.weights_sha256, a.manifest.weights_sha256);
    for (name, moment) in &a.optimizer {
        assert_eq!(
            moment.flatten_all().unwrap().to_vec1::<f32>().unwrap(),
            entry.optimizer[name]
                .flatten_all()
                .unwrap()
                .to_vec1::<f32>()
                .unwrap()
        );
    }
    let entry_state = entry.manifest.training.unwrap();
    assert_eq!(entry_state.config.lr, 0.001);
    assert_eq!(entry_state.config.warmup, 2);
    assert_eq!(entry_state.config.learning_rate(7), 0.0005);
    assert_eq!(entry_state.config.learning_rate(8), 0.001);
    assert_eq!(entry_state.sampler_state, a_state.sampler_state);
    assert_eq!(entry_state.config.budget_start_step, 6);
    assert_eq!(
        entry_state.config.budget_start_tokens,
        a_state.consumed_tokens
    );
    let extended = checkpoint::load(&extended.join("final"), Device::Cpu, true).unwrap();
    assert_eq!(extended.manifest.source_id, source_id);
    let extended_state = extended.manifest.training.unwrap();
    assert_eq!(extended_state.step, 10);
    assert!(extended_state.consumed_tokens > a_state.consumed_tokens);
    assert_ne!(extended.manifest.weights_sha256, a.manifest.weights_sha256);
    assert_eq!(
        extended_state.initial_weight_hash,
        a_state.initial_weight_hash
    );
    for budget in ["0", "5001"] {
        let out = Command::new(env!("CARGO_BIN_EXE_replica-train"))
            .args([
                "train",
                "--resume",
                full_final.to_str().unwrap(),
                "--output",
            ])
            .arg(d.path().join(format!("invalid-{budget}")))
            .args([
                "--numeric-probe",
                "--extend-steps",
                budget,
                "--source-id",
                &source_id,
            ])
            .output()
            .unwrap();
        assert!(!out.status.success());
        assert!(!d.path().join(format!("invalid-{budget}")).exists());
    }
    for (flag, value) in [
        ("--extend-microbatch", "0"),
        ("--extend-microbatch", "9"),
        ("--extend-first-target-weight", "0"),
        ("--extend-first-target-weight", "17"),
        ("--extend-first-target-weight", "NaN"),
        ("--extend-lr", "0"),
        ("--extend-lr", "NaN"),
        ("--extend-warmup", "5"),
        ("--extend-curriculum-steps", "5"),
    ] {
        let output = d.path().join(format!("invalid-{flag}-{value}"));
        let out = Command::new(env!("CARGO_BIN_EXE_replica-train"))
            .args([
                "train",
                "--resume",
                full_final.to_str().unwrap(),
                "--output",
            ])
            .arg(&output)
            .args([
                "--numeric-probe",
                "--extend-steps",
                "4",
                "--source-id",
                &source_id,
                flag,
                value,
            ])
            .output()
            .unwrap();
        assert!(!out.status.success());
        assert!(!output.exists());
    }
}

#[test]
fn curriculum_resume_crosses_sampling_boundary_in_fresh_process() {
    use candle_core::Device;
    use replica_v3::neural::{
        ByteBpe, checkpoint, hash,
        transformer::{Config, Transformer},
    };
    let dir = tempfile::tempdir().unwrap();
    let corpus = dir.path().join("corpus");
    std::fs::create_dir(&corpus).unwrap();
    let episodes = |split: &str, values: &[&str]| -> Vec<serde_json::Value> {
        values.iter().enumerate().map(|(i, value)| {
            let input = format!("source {value}");
            let identity = format!("{split}-{i}");
            serde_json::json!({"id":identity,"category":3,"family":format!("{}/{split}",if i % 2 == 0 {"copy"} else {"qa"}),"binding":identity,"sequence":hash(input.as_bytes()),"request":{"request_id":identity,"system":"","input":input,"evidence":replica_v3::retrieval::EvidenceBundle::default(),"limits":{"max_tokens":8,"context_tokens":64,"timeout_ms":5000}},"answer":value})
        }).collect()
    };
    let train = serde_json::to_vec(&episodes("train", &["12", "left", "47", "right"])).unwrap();
    let validation = serde_json::to_vec(&episodes("validation", &["98", "up"])).unwrap();
    let split = |name: &str, bytes: &[u8], count: usize| {
        std::fs::write(corpus.join(format!("{name}.json")), bytes).unwrap();
        serde_json::json!({"file":format!("{name}.json"),"sha256":hash(bytes),"bytes":bytes.len(),"documents":count,"tokens":null})
    };
    let manifest = serde_json::json!({"version":1,"scope":"SYNTHETIC_ONLY","permission":"test fixture","generator":"resume-boundary-fixture","seed":29,"split_rule":"disjoint episodes and families","train":split("train", &train, 4),"validation":split("validation", &validation, 2)});
    std::fs::write(
        corpus.join("manifest.json"),
        serde_json::to_vec(&manifest).unwrap(),
    )
    .unwrap();
    let tok = ByteBpe::train(&[b"source 12 left 47 right".to_vec()], &hash(&train), 280).unwrap();
    let model = Transformer::init(Config::tiny(tok.vocab_size()), 29, Device::Cpu).unwrap();
    let initial = dir.path().join("initial");
    checkpoint::save(
        &initial,
        &model,
        &tok,
        checkpoint::initialized(&model, &tok, 29, hash(b"curriculum resume fixture")).unwrap(),
        &Default::default(),
    )
    .unwrap();
    let run = |name: &str, resume: bool, stop: bool| {
        let mut command = Command::new(env!("CARGO_BIN_EXE_replica-train"));
        command
            .arg("train")
            .arg(if resume { "--resume" } else { "--checkpoint" })
            .arg(if resume {
                dir.path().join("partial/final")
            } else {
                initial.clone()
            })
            .arg("--corpus")
            .arg(&corpus)
            .arg("--output")
            .arg(dir.path().join(name));
        if !resume {
            command.args([
                "--steps",
                "4",
                "--seq-len",
                "64",
                "--warmup",
                "0",
                "--microbatch",
                "2",
                "--accumulation",
                "1",
                "--validate-every",
                "4",
                "--curriculum-steps",
                "2",
            ]);
        }
        if stop {
            command.args(["--stop-after", "2"]);
        }
        let out = command.output().unwrap();
        assert!(
            out.status.success(),
            "{}\n{}",
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        );
    };
    run("full", false, false);
    run("partial", false, true);
    run("resumed", true, false);
    let full = checkpoint::load(&dir.path().join("full/final"), Device::Cpu, true).unwrap();
    let resumed = checkpoint::load(&dir.path().join("resumed/final"), Device::Cpu, true).unwrap();
    assert_eq!(
        full.manifest.weights_sha256,
        resumed.manifest.weights_sha256
    );
    let a = full.manifest.training.unwrap();
    let b = resumed.manifest.training.unwrap();
    assert_eq!(b.config.curriculum_steps, 2);
    assert_eq!(b.step, 4);
    assert_eq!(a.sampler_state, b.sampler_state);
    assert_eq!(a.consumed_tokens, b.consumed_tokens);
    assert_eq!(a.target_tokens, b.target_tokens);
    let audit = |end: &str| {
        Command::new(env!("CARGO_BIN_EXE_replica-train"))
            .arg("sampling-exposure")
            .arg("--start")
            .arg(dir.path().join("full/start"))
            .arg("--end")
            .arg(dir.path().join(end))
            .arg("--corpus")
            .arg(&corpus)
            .args(["--limit", "4"])
            .output()
            .unwrap()
    };
    let exposure = audit("partial/final");
    assert!(
        exposure.status.success(),
        "{}",
        String::from_utf8_lossy(&exposure.stderr)
    );
    let exposure: serde_json::Value = serde_json::from_slice(&exposure.stdout).unwrap();
    assert_eq!(exposure["draws"], 4);
    assert_eq!(exposure["sampler_state_matches"], true);
    for i in [1, 3] {
        assert_eq!(
            exposure["prefix"][i]["draws"], 0,
            "copy-only phase cannot draw ordinary QA"
        );
    }
    let exposure = audit("full/final");
    assert!(exposure.status.success());
    let exposure: serde_json::Value = serde_json::from_slice(&exposure.stdout).unwrap();
    assert_eq!(exposure["draws"], 8);
    let mut corrupted =
        checkpoint::load(&dir.path().join("full/final"), Device::Cpu, true).unwrap();
    corrupted.manifest.training.as_mut().unwrap().sampler_state = a.sampler_state ^ 1;
    checkpoint::save(
        &dir.path().join("corrupt-rng"),
        &corrupted.model,
        &corrupted.tokenizer,
        corrupted.manifest,
        &corrupted.optimizer,
    )
    .unwrap();
    let rejected = audit("corrupt-rng");
    assert!(!rejected.status.success());
    assert!(String::from_utf8_lossy(&rejected.stderr).contains("replayed sampler does not match"));
    // An explicit grouped extension must keep optimizer and sampler restart exact,
    // including the transition from the filtered copy pool to the full corpus.
    let source = hash(b"grouped sampler extension fixture");
    let grouped = |name: &str, extend: bool, source_id: bool, stop: bool, resume: bool| {
        let mut command = Command::new(env!("CARGO_BIN_EXE_replica-train"));
        command
            .arg("train")
            .arg("--resume")
            .arg(dir.path().join(if resume {
                "grouped-partial/final"
            } else {
                "full/final"
            }))
            .arg("--corpus")
            .arg(&corpus)
            .arg("--output")
            .arg(dir.path().join(name));
        if !resume {
            command.args(["--extend-sample-group-size", "2"]);
            if extend {
                command.args(["--extend-steps", "4", "--extend-curriculum-steps", "2"]);
            }
            if source_id {
                command.args(["--source-id", &source]);
            }
        }
        if stop {
            command.args(["--stop-after", "6"]);
        }
        command.output().unwrap()
    };
    for (name, extend, source_id) in [
        ("grouped-no-extension", false, true),
        ("grouped-no-source", true, false),
    ] {
        assert!(
            !grouped(name, extend, source_id, false, false)
                .status
                .success()
        );
        assert!(!dir.path().join(name).exists());
    }
    for (name, stop, resume) in [
        ("grouped-full", false, false),
        ("grouped-partial", true, false),
        ("grouped-resumed", false, true),
    ] {
        let result = grouped(name, !resume, !resume, stop, resume);
        assert!(
            result.status.success(),
            "{}\n{}",
            String::from_utf8_lossy(&result.stdout),
            String::from_utf8_lossy(&result.stderr)
        );
    }
    let grouped_full =
        checkpoint::load(&dir.path().join("grouped-full/final"), Device::Cpu, true).unwrap();
    let grouped_resumed =
        checkpoint::load(&dir.path().join("grouped-resumed/final"), Device::Cpu, true).unwrap();
    assert_eq!(
        grouped_full.manifest.weights_sha256,
        grouped_resumed.manifest.weights_sha256
    );
    assert_eq!(
        serde_json::to_value(&grouped_full.manifest.training).unwrap(),
        serde_json::to_value(&grouped_resumed.manifest.training).unwrap()
    );
    assert_eq!(
        grouped_full
            .manifest
            .training
            .unwrap()
            .config
            .sample_group_size,
        2
    );
    let result = Command::new(env!("CARGO_BIN_EXE_replica-train"))
        .arg("sampling-exposure")
        .arg("--start")
        .arg(dir.path().join("grouped-full/start"))
        .arg("--end")
        .arg(dir.path().join("grouped-partial/final"))
        .arg("--corpus")
        .arg(&corpus)
        .args(["--limit", "4"])
        .output()
        .unwrap();
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    let exposure: serde_json::Value = serde_json::from_slice(&result.stdout).unwrap();
    assert_eq!(exposure["draws"], 4);
    assert_eq!(exposure["sampler_state_matches"], true);
    for (i, draws) in [2, 0, 2, 0].into_iter().enumerate() {
        assert_eq!(exposure["prefix"][i]["draws"], draws);
    }
    let changed = dir.path().join("changed-corpus");
    std::fs::create_dir(&changed).unwrap();
    let next_train =
        serde_json::to_vec(&episodes("next-train", &["31", "down", "68", "back"])).unwrap();
    let next_validation =
        serde_json::to_vec(&episodes("next-validation", &["85", "north"])).unwrap();
    let mut next_manifest = manifest.clone();
    for (name, bytes, count) in [
        ("train", &next_train, 4),
        ("validation", &next_validation, 2),
    ] {
        std::fs::write(changed.join(format!("{name}.json")), bytes).unwrap();
        next_manifest[name] = serde_json::json!({"file":format!("{name}.json"),"sha256":hash(bytes),"bytes":bytes.len(),"documents":count,"tokens":null});
    }
    std::fs::write(
        changed.join("manifest.json"),
        serde_json::to_vec(&next_manifest).unwrap(),
    )
    .unwrap();
    let change_source = hash(b"corpus change source fixture");
    let next_run = |name: &str, explicit: bool, partial: bool, resume: bool| {
        let mut command = Command::new(env!("CARGO_BIN_EXE_replica-train"));
        command
            .arg("train")
            .arg("--resume")
            .arg(dir.path().join(if resume {
                "changed-partial/final"
            } else {
                "full/final"
            }))
            .arg("--corpus")
            .arg(&changed)
            .arg("--output")
            .arg(dir.path().join(name));
        if !resume {
            command.args(["--extend-steps", "2", "--source-id", &change_source]);
            command.args(["--extend-microbatch", "2", "--extend-curriculum-steps", "1"]);
            command.args(["--extend-first-target-weight", "7"]);
        }
        if explicit {
            command.arg("--replace-corpus");
        }
        if partial {
            command.args(["--stop-after", "5"]);
        }
        command.output().unwrap()
    };
    assert!(
        !next_run("rejected-change", false, false, false)
            .status
            .success()
    );
    assert!(!dir.path().join("rejected-change").exists());
    for (name, partial, resume) in [
        ("changed-full", false, false),
        ("changed-partial", true, false),
        ("changed-resumed", false, true),
    ] {
        let out = next_run(name, !resume, partial, resume);
        assert!(
            out.status.success(),
            "{}\n{}",
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        );
    }
    let entry =
        checkpoint::load(&dir.path().join("changed-full/start"), Device::Cpu, true).unwrap();
    assert_eq!(entry.manifest.weights_sha256, full.manifest.weights_sha256);
    let next = checkpoint::load(&dir.path().join("changed-full/final"), Device::Cpu, true).unwrap();
    let resumed =
        checkpoint::load(&dir.path().join("changed-resumed/final"), Device::Cpu, true).unwrap();
    assert_eq!(
        next.manifest.weights_sha256,
        resumed.manifest.weights_sha256
    );
    assert_eq!(next.tokenizer.id(), tok.id());
    let state = next.manifest.training.unwrap();
    assert_eq!(state.step, 6);
    assert_eq!(state.config.microbatch, 2);
    assert_eq!(state.config.curriculum_steps, 5);
    assert_eq!(state.config.first_target_weight, 7.);
    assert_eq!(state.corpus_hash, hash(&next_train));
    assert_eq!(state.previous_corpora, [hash(&train)]);
}

#[test]
fn native_training_cancel_keeps_optimizer_boundary_checkpoint() {
    use candle_core::Device;
    use replica_v3::neural::{
        ByteBpe, checkpoint, hash,
        transformer::{Config, Transformer},
    };
    use std::{
        io::{BufRead, BufReader},
        process::Stdio,
        time::Duration,
    };
    let d = tempfile::tempdir().unwrap();
    let initial = d.path().join("initial");
    let output = d.path().join("cancelled");
    let tok = ByteBpe::train(
        &["가 나 다 오른쪽 왼쪽".as_bytes().to_vec(); 1],
        &hash(b"numeric training fixture"),
        300,
    )
    .unwrap();
    let model = Transformer::init(Config::tiny(tok.vocab_size()), 73, Device::Cpu).unwrap();
    checkpoint::save(
        &initial,
        &model,
        &tok,
        checkpoint::initialized(&model, &tok, 73, hash(b"fixture source")).unwrap(),
        &Default::default(),
    )
    .unwrap();
    let mut child = Command::new(env!("CARGO_BIN_EXE_replica-train"))
        .args([
            "train",
            "--checkpoint",
            initial.to_str().unwrap(),
            "--output",
            output.to_str().unwrap(),
            "--numeric-probe",
            "--seq-len",
            "64",
            "--accumulation",
            "1",
        ])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let reader = child.stdout.take().unwrap();
    let (send, receive) = std::sync::mpsc::channel();
    let observer = std::thread::spawn(move || {
        let mut lines = Vec::new();
        for line in BufReader::new(reader).lines() {
            let line = line.unwrap();
            if line.starts_with("step=") {
                let _ = send.send(());
            }
            lines.push(line);
        }
        lines
    });
    if receive.recv_timeout(Duration::from_secs(10)).is_err() {
        child.kill().unwrap();
        child.wait().unwrap();
        panic!("no actual optimizer step observed");
    }
    assert!(
        Command::new("kill")
            .args(["-INT", &child.id().to_string()])
            .status()
            .unwrap()
            .success()
    );
    let result = child.wait_with_output().unwrap();
    let lines = observer.join().unwrap();
    assert!(!result.status.success());
    assert!(String::from_utf8_lossy(&result.stderr).contains("cancelled"));
    let restored = checkpoint::load(&output.join("final"), Device::Cpu, true).unwrap();
    assert_eq!(restored.manifest.status, "CANCELLED");
    let state = restored.manifest.training.unwrap();
    assert!(state.step >= 1);
    eprintln!(
        "cancel integration actual TINY optimizer updates={}",
        state.step
    );
    let receipt: serde_json::Value = serde_json::from_str(
        lines
            .iter()
            .find_map(|line| line.strip_prefix("TRAIN_CONTROL "))
            .unwrap(),
    )
    .unwrap();
    assert_eq!(receipt["reason"], "CANCELLED");
    assert_eq!(receipt["checkpoint_saved"], true);
    assert_eq!(receipt["work_budget_seconds"], 900.);
    assert_eq!(
        receipt["teacher_calls"], 2,
        "cancel must not launch final validation"
    );
    assert_eq!(receipt["final_evaluation_complete"], false);
    assert_eq!(
        state.validation_loss, None,
        "previous-step CE must not label new weights"
    );
    assert!(!restored.optimizer.is_empty());
}

#[cfg(feature = "test-support")]
#[test]
fn native_export_kill_before_publication_preserves_source_and_allows_retry() {
    use replica_v3::neural::{
        ByteBpe, checkpoint, hash,
        transformer::{Config, Transformer},
    };
    use std::{
        process::Stdio,
        time::{Duration, Instant},
    };
    let d = tempfile::tempdir().unwrap();
    let source = d.path().join("source.r3m");
    let destination = d.path().join("inference.r3m");
    let marker = d.path().join("marker");
    let tok = ByteBpe::train(
        &[b"export fixture fixture".to_vec()],
        &hash(b"export fixture"),
        280,
    )
    .unwrap();
    let model =
        Transformer::init(Config::tiny(tok.vocab_size()), 23, candle_core::Device::Cpu).unwrap();
    checkpoint::save(
        &source,
        &model,
        &tok,
        checkpoint::initialized(&model, &tok, 23, hash(b"source")).unwrap(),
        &Default::default(),
    )
    .unwrap();
    let original = std::fs::read(&source).unwrap();
    let command = || {
        let mut cmd = Command::new(env!("CARGO_BIN_EXE_replica-train"));
        cmd.args(["model", "export-inference", "--checkpoint"])
            .arg(&source)
            .arg("--output")
            .arg(&destination);
        cmd
    };
    let mut child = command()
        .env("REPLICA_TEST_PAUSE", "artifact_before_publish")
        .env("REPLICA_TEST_MARKER", &marker)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let deadline = Instant::now() + Duration::from_secs(10);
    while !marker.exists() {
        if let Some(status) = child.try_wait().unwrap() {
            panic!("export exited before marker: {status}");
        }
        if Instant::now() > deadline {
            child.kill().unwrap();
            child.wait().unwrap();
            panic!("export marker timeout");
        }
        std::thread::sleep(Duration::from_millis(5));
    }
    child.kill().unwrap();
    let result = child.wait_with_output().unwrap();
    assert!(!result.status.success());
    assert!(result.stdout.is_empty());
    assert!(!destination.exists());
    assert_eq!(std::fs::read(&source).unwrap(), original);
    let result = command().output().unwrap();
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    let reloaded = checkpoint::load(&destination, candle_core::Device::Cpu, false).unwrap();
    assert_eq!(
        reloaded.model.weights_content_id().unwrap(),
        model.weights_content_id().unwrap()
    );
    assert_eq!(std::fs::read(&source).unwrap(), original);
}
