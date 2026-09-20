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
        "train.r3b" => replica_v3::binary::to_vec(&corpus.train).unwrap(),
        "validation.r3b" => replica_v3::binary::to_vec(&corpus.validation).unwrap(),
        "manifest.r3b" => replica_v3::binary::to_vec(&corpus.manifest).unwrap(),
        _ => panic!("unknown test split"),
    }
}
#[test]
fn harness_m04_cli_close_reports_stopped_arm_before_any_replay() {
    let dir = tempfile::tempdir().unwrap();
    for name in ["C", "W"] {
        let path = dir.path().join(name);
        std::fs::create_dir(&path).unwrap();
        std::fs::write(path.join("result.r3b"), replica_v3::binary::to_vec(&replica_v3::binary::record!({"reason":"CANCELLED","comparison_eligible":false,"final_evaluation_complete":false})).unwrap()).unwrap();
    }
    let output = dir.path().join("closed");
    let result = Command::new(env!("CARGO_BIN_EXE_replica-train"))
        .args([
            "recovery",
            "--archived-controls",
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
    let error = String::from_utf8(result.stderr).unwrap();
    assert!(error.contains("INELIGIBLE_OR_UNVERIFIED_ARM"), "{error}");
    let summary: replica_v3::binary::Value =
        replica_v3::binary::from_slice(&std::fs::read(output.join("summary.r3b")).unwrap()).unwrap();
    assert_eq!(summary["comparison_eligible"], false);
    assert_eq!(summary["candidate_eligible"], false);
    assert_eq!(summary["model_calls"], 0);
    assert_eq!(summary["arm_terminals"][0]["reason"], "CANCELLED");
}
#[test]
fn harness_m01_m02_malformed_cli_rejects_before_output_or_model_load() {
    use replica_v3::binary::{Value, record};
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
    let original_train = native_assertion_bytes(&source, "train.r3b");
    let original_validation = native_assertion_bytes(&source, "validation.r3b");
    let original_manifest: Value =
        replica_v3::binary::from_slice(&native_assertion_bytes(&source, "manifest.r3b")).unwrap();
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
        let mut rows: Vec<Value> = replica_v3::binary::from_slice(if qa {
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
            row["request"]["input"] = record!(format!(
                "{}의 {context} 이동 지시와 근거는?",
                if qa { "장비31" } else { entity }
            ));
            row["binding"] = record!(format!("{entity}/{context}/{value}"));
            let records = row["request"]["evidence"]["items"].as_array_mut().unwrap();
            records.truncate(2);
            records[0]["original_excerpt"] =
                record!(format!("{entity}의 {context} 이동 지시는 {value}이다."));
            records[0]["version_status"] = record!("current");
            records[1]["original_excerpt"] = record!("장비32의 구역2 이동 지시는 서쪽이다.");
            records[1]["version_status"] = record!("current");
            let answer = format!(
                "{} [event:{}]",
                records[0]["original_excerpt"].as_str().unwrap(),
                records[0]["event_id"]
            );
            row["answer"] = record!(answer);
        }
        let changed = replica_v3::binary::to_vec(&rows).unwrap();
        let split = if qa { "validation" } else { "train" };
        let fixture = data::native::from_episodes(
            replica_v3::binary::from_value(original_manifest.clone()).unwrap(),
            replica_v3::binary::from_slice(if qa { &original_train } else { &changed }).unwrap(),
            replica_v3::binary::from_slice(if qa { &changed } else { &original_validation }).unwrap(),
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
            replica_v3::binary::from_slice::<Value>(&native_assertion_bytes(
                &corpus,
                &format!("{split}.r3b")
            ))
            .unwrap(),
            replica_v3::binary::from_slice::<Value>(&changed).unwrap()
        );
        assert_eq!(std::fs::read(&corpus).unwrap(), before);
    }
    assert_eq!(
        native_assertion_bytes(&source, "train.r3b"),
        original_train
    );
    assert_eq!(
        native_assertion_bytes(&source, "validation.r3b"),
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
    let validation_bytes = native_assertion_bytes(&corpus, "validation.r3b");
    let validation: Vec<replica_v3::binary::Value> = replica_v3::binary::from_slice(&validation_bytes).unwrap();
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
        let rows: Vec<replica_v3::binary::Value> = replica_v3::binary::read_records(&output).unwrap();
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
        native_assertion_bytes(&corpus, "validation.r3b")
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
        native_assertion_bytes(&source, "validation.r3b"),
        native_assertion_bytes(&output, "validation.r3b")
    );
    let original: Vec<replica_v3::binary::Value> =
        replica_v3::binary::from_slice(&native_assertion_bytes(&source, "train.r3b")).unwrap();
    let train: Vec<replica_v3::binary::Value> =
        replica_v3::binary::from_slice(&native_assertion_bytes(&output, "train.r3b")).unwrap();
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
                replica_v3::binary::record!(reversed),
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
        replica_v3::binary::from_slice::<Vec<replica_v3::binary::Value>>(&native_assertion_bytes(
            &output,
            "train.r3b"
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
        read("field-pairs", "validation.r3b"),
        read("query-pairs", "validation.r3b")
    );
    let before: Vec<replica_v3::binary::Value> =
        replica_v3::binary::from_slice(&read("field-pairs", "train.r3b")).unwrap();
    let after: Vec<replica_v3::binary::Value> =
        replica_v3::binary::from_slice(&read("query-pairs", "train.r3b")).unwrap();
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
        read("field-cue", "validation.r3b"),
        read("field-pairs", "validation.r3b")
    );
    let before: Vec<replica_v3::binary::Value> =
        replica_v3::binary::from_slice(&read("field-cue", "train.r3b")).unwrap();
    let after: Vec<replica_v3::binary::Value> =
        replica_v3::binary::from_slice(&read("field-pairs", "train.r3b")).unwrap();
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
    for split in ["train.r3b", "validation.r3b"] {
        let read = |profile: &str| -> Vec<replica_v3::binary::Value> {
            replica_v3::binary::from_slice(&native_assertion_bytes(&dir.path().join(profile), split))
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
    for split in ["train.r3b", "validation.r3b"] {
        let read = |profile: &str| -> Vec<replica_v3::binary::Value> {
            replica_v3::binary::from_slice(&native_assertion_bytes(&dir.path().join(profile), split))
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
    for split in ["train.r3b", "validation.r3b"] {
        let read = |profile: &str| -> Vec<replica_v3::binary::Value> {
            replica_v3::binary::from_slice(&native_assertion_bytes(&dir.path().join(profile), split))
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
                    assert_eq!(split, "train.r3b", "validation QA stays independent");
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
    for split in ["train.r3b", "validation.r3b"] {
        let read = |profile: &str| -> Vec<replica_v3::binary::Value> {
            replica_v3::binary::from_slice(&native_assertion_bytes(&dir.path().join(profile), split))
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
    let read = |file| -> Vec<replica_v3::binary::Value> {
        replica_v3::binary::from_slice(&native_assertion_bytes(&corpus, file)).unwrap()
    };
    let train = read("train.r3b");
    let validation = read("validation.r3b");
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
    let read = |name| -> Vec<replica_v3::binary::Value> {
        replica_v3::binary::from_slice(&native_assertion_bytes(&corpus, name)).unwrap()
    };
    let train = read("train.r3b");
    let validation = read("validation.r3b");
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
    let read = |file| -> Vec<replica_v3::binary::Value> {
        replica_v3::binary::from_slice(&native_assertion_bytes(&corpus, file)).unwrap()
    };
    let train = read("train.r3b");
    let validation = read("validation.r3b");
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
            let number = |row: &replica_v3::binary::Value| -> u64 {
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
    for filename in ["train.r3b", "validation.r3b"] {
        let rows: Vec<replica_v3::binary::Value> =
            replica_v3::binary::from_slice(&native_assertion_bytes(&corpus, filename)).unwrap();
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
    let tokenizer = d.path().join("tokenizer.r3b");
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
    let record_bytes: replica_v3::binary::Value = replica_v3::binary::from_slice(&out.stdout).unwrap();
    assert_eq!(record_bytes["ids"], replica_v3::binary::record!(tok.encode(raw).unwrap()));
    assert_eq!(record_bytes["roundtrip_sha256"], replica_v3::neural::hash(raw));
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
    // All persisted training metadata is native; tensor checkpoints retain R3MODEL.
    for root in [&full, &partial, &resumed] {
        let control: replica_v3::binary::Value = replica_v3::binary::from_slice(
            &std::fs::read(root.join("train-control.r3b")).unwrap(),
        ).unwrap();
        assert!(control.is_object());
        for entry in std::fs::read_dir(root).unwrap() {
            let path = entry.unwrap().path();
            if path.is_file() {
                let bytes = std::fs::read(&path).unwrap();
                assert!(bytes.starts_with(b"R3MODEL") || bytes.starts_with(b"R3BIN"), "{}", path.display());
                assert!(!matches!(path.extension().and_then(|s| s.to_str()), Some("json" | "jsonl")));
                if bytes.starts_with(b"R3BIN") {
                    replica_v3::binary::from_slice::<replica_v3::binary::Value>(&bytes).unwrap();
                }
            }
        }
    }
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
    let episodes = |split: &str, values: &[&str]| -> Vec<replica_v3::binary::Value> {
        values.iter().enumerate().map(|(i, value)| {
            let input = format!("source {value}");
            let identity = format!("{split}-{i}");
            replica_v3::binary::record!({"id":identity,"category":3,"family":format!("{}/{split}",if i % 2 == 0 {"copy"} else {"qa"}),"binding":identity,"sequence":hash(input.as_bytes()),"request":{"request_id":identity,"system":"","input":input,"evidence":replica_v3::retrieval::EvidenceBundle::default(),"limits":{"max_tokens":8,"context_tokens":64,"timeout_ms":5000}},"answer":value})
        }).collect()
    };
    let train = replica_v3::binary::to_vec(&episodes("train", &["12", "left", "47", "right"])).unwrap();
    let validation = replica_v3::binary::to_vec(&episodes("validation", &["98", "up"])).unwrap();
    let split = |name: &str, bytes: &[u8], count: usize| {
        std::fs::write(corpus.join(format!("{name}.r3b")), bytes).unwrap();
        replica_v3::binary::record!({"file":format!("{name}.r3b"),"sha256":hash(bytes),"bytes":bytes.len(),"documents":count,"tokens":null})
    };
    let manifest = replica_v3::binary::record!({"version":1,"scope":"SYNTHETIC_ONLY","permission":"test fixture","generator":"resume-boundary-fixture","seed":29,"split_rule":"disjoint episodes and families","train":split("train", &train, 4),"validation":split("validation", &validation, 2)});
    std::fs::write(
        corpus.join("manifest.r3b"),
        replica_v3::binary::to_vec(&manifest).unwrap(),
    )
    .unwrap();
    let native = data::native::from_episodes(
        replica_v3::binary::from_value(manifest.clone()).unwrap(),
        replica_v3::binary::from_slice(&train).unwrap(),
        replica_v3::binary::from_slice(&validation).unwrap(),
    ).unwrap();
    let train_hash = native.manifest.train.sha256.clone();
    let corpus = dir.path().join("source.r3c");
    data::native::write(&corpus, &native, false).unwrap();
    let tok = ByteBpe::train(&[b"source 12 left 47 right".to_vec()], &train_hash, 280).unwrap();
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
    let exposure: replica_v3::binary::Value = replica_v3::binary::from_slice(&exposure.stdout).unwrap();
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
    let exposure: replica_v3::binary::Value = replica_v3::binary::from_slice(&exposure.stdout).unwrap();
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
        replica_v3::binary::to_value(&grouped_full.manifest.training).unwrap(),
        replica_v3::binary::to_value(&grouped_resumed.manifest.training).unwrap()
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
    let exposure: replica_v3::binary::Value = replica_v3::binary::from_slice(&result.stdout).unwrap();
    assert_eq!(exposure["draws"], 4);
    assert_eq!(exposure["sampler_state_matches"], true);
    for (i, draws) in [2, 0, 2, 0].into_iter().enumerate() {
        assert_eq!(exposure["prefix"][i]["draws"], draws);
    }
    let changed = dir.path().join("changed-corpus");
    std::fs::create_dir(&changed).unwrap();
    let next_train =
        replica_v3::binary::to_vec(&episodes("next-train", &["31", "down", "68", "back"])).unwrap();
    let next_validation =
        replica_v3::binary::to_vec(&episodes("next-validation", &["85", "north"])).unwrap();
    let mut next_manifest = manifest.clone();
    for (name, bytes, count) in [
        ("train", &next_train, 4),
        ("validation", &next_validation, 2),
    ] {
        std::fs::write(changed.join(format!("{name}.r3b")), bytes).unwrap();
        next_manifest[name] = replica_v3::binary::record!({"file":format!("{name}.r3b"),"sha256":hash(bytes),"bytes":bytes.len(),"documents":count,"tokens":null});
    }
    std::fs::write(
        changed.join("manifest.r3b"),
        replica_v3::binary::to_vec(&next_manifest).unwrap(),
    )
    .unwrap();
    let native = data::native::from_episodes(
        replica_v3::binary::from_value(next_manifest.clone()).unwrap(),
        replica_v3::binary::from_slice(&next_train).unwrap(),
        replica_v3::binary::from_slice(&next_validation).unwrap(),
    ).unwrap();
    let next_train_hash = native.manifest.train.sha256.clone();
    let changed = dir.path().join("changed-source.r3c");
    data::native::write(&changed, &native, false).unwrap();
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
    assert_eq!(state.corpus_hash, next_train_hash);
    assert_eq!(state.previous_corpora, [train_hash]);
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
    let _lines = observer.join().unwrap();
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
    let receipt: replica_v3::binary::Value = replica_v3::binary::from_slice(&std::fs::read(output.join("train-control.r3b")).unwrap()).unwrap();
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

#[test]
fn fresh_balanced_two_updates_match_fresh_process_resume_and_reject_unbound() {
    use candle_core::Device;
    use replica_v3::neural::checkpoint;
    let d=tempfile::tempdir().unwrap();
    let call=|args:&[&str],success:bool| {
        let out=Command::new(env!("CARGO_BIN_EXE_replica-train")).args(args)
            .env("VECLIB_MAXIMUM_THREADS","1").env("RAYON_NUM_THREADS","1").env("R3_FRESH_FIXTURE_EOS","1").output().unwrap();
        assert_eq!(out.status.success(),success,"{}\n{}",String::from_utf8_lossy(&out.stdout),String::from_utf8_lossy(&out.stderr));
    };
    let full=d.path().join("full");let split=d.path().join("split");
    for root in [&full,&split] {call(&["fresh","fixture","--output",root.to_str().unwrap()],true);}
    call(&["fresh","fixture-full","--root",full.to_str().unwrap()],true);
    call(&["fresh","run","--root",split.to_str().unwrap()],true);
    let first=split.join("segment-0000/final");
    call(&["train","--resume",first.to_str().unwrap(),"--corpus",split.join("corpus.r3cor").to_str().unwrap(),"--output",d.path().join("forbidden").to_str().unwrap()],false);
    assert!(!d.path().join("forbidden").exists());
    call(&["fresh","run","--root",split.to_str().unwrap()],true);
    let a=checkpoint::load(&full.join("segment-0000/final"),Device::Cpu,true).unwrap();
    let b=checkpoint::load(&split.join("segment-0001/final"),Device::Cpu,true).unwrap();
    assert_eq!(a.model.weight_hash().unwrap(),b.model.weight_hash().unwrap());
    for (k,t) in &a.optimizer {assert_eq!(t.flatten_all().unwrap().to_vec1::<f32>().unwrap(),b.optimizer[k].flatten_all().unwrap().to_vec1::<f32>().unwrap());}
    let sa=a.manifest.training.unwrap();let sb=b.manifest.training.unwrap();
    assert_eq!((sa.step,sa.sampler_state,sa.consumed_tokens,sa.target_tokens),(2,2,sb.consumed_tokens,sb.target_tokens));
    assert_eq!(sa.config,sb.config);assert_eq!(sb.step,2);assert_eq!(sb.sampler_state,2);
    call(&["fresh","fixture-eval","--root",split.to_str().unwrap()],true);
    let raw=split.join("eval-0002-fixture.r3rows");let before=std::fs::read(&raw).unwrap();
    call(&["fresh","fixture-eval","--root",split.to_str().unwrap()],true);
    assert_eq!(before,std::fs::read(&raw).unwrap());
    // Reuse this completed disposable run for raw corruption/missing-obligation checks.
    let raw = split.join("eval-0002-dev512.r3rows");
    let original = std::fs::read(&raw).unwrap();
    for fault in ["truncated", "trailing", "missing"] {
        match fault {
            "truncated" => std::fs::write(&raw, &original[..original.len()-1]).unwrap(),
            "trailing" => { let mut bytes=original.clone(); bytes.push(0); std::fs::write(&raw, bytes).unwrap(); }
            _ => std::fs::remove_file(&raw).unwrap(),
        }
        call(&["fresh","report","--root",split.to_str().unwrap()],false);
        std::fs::write(&raw,&original).unwrap();
    }
    let teacher = split.join("eval-0002-transfer128-teachers.r3rows");
    let original = std::fs::read(&teacher).unwrap();
    std::fs::remove_file(&teacher).unwrap();
    call(&["fresh","report","--root",split.to_str().unwrap()],false);
    std::fs::write(teacher, original).unwrap();
    let checkpoint = split.join("segment-0001/final");
    let original = std::fs::read(&checkpoint).unwrap();
    std::fs::copy(split.join("initial.r3m"), &checkpoint).unwrap();
    call(&["fresh","report","--root",split.to_str().unwrap()],false);
    std::fs::write(checkpoint, original).unwrap();
    call(&["fresh","run","--root",split.to_str().unwrap()],false);
    let p=split.join("plan.r3b");let mut policy:replica_v3::binary::Value=replica_v3::binary::from_slice(&std::fs::read(&p).unwrap()).unwrap();
    policy["config"]["seed"]=replica_v3::binary::record!(30);
    std::fs::write(&p,policy.to_vec().unwrap()).unwrap();
    call(&["fresh","run","--root",split.to_str().unwrap()],false);
    println!("FRESH_TINY_OPTIMIZER_CALLS=4 fresh_process_resume=EXACT missing_policy=REJECTED");
}

#[test]
fn fresh_fx01_post_publication_failure_blocks_new_process() {
    for fault in ["finished-file-sync","finished-dir-sync","pending-write","finished-pending-only"] {
    let d=tempfile::tempdir().unwrap();let root=d.path().join("run");
    let call=|args:&[&str],fault:Option<&str>| {let mut c=Command::new(env!("CARGO_BIN_EXE_replica-train"));c.args(args).env("VECLIB_MAXIMUM_THREADS","1").env("RAYON_NUM_THREADS","1");if let Some(f)=fault{c.env("R3_FRESH_TEST_STOP",f);}c.output().unwrap()};
    assert!(call(&["fresh","fixture","--output",root.to_str().unwrap()],None).status.success());
    let r=call(&["fresh","run","--root",root.to_str().unwrap()],Some(fault));
    assert!(!r.status.success(),"post-publication fault must fail");
    let final_path=root.join("segment-0000-finished.r3b");
    assert_eq!(final_path.exists(),fault.ends_with("sync"));
    let r=call(&["fresh","run","--root",root.to_str().unwrap()],None);
    assert!(!r.status.success());assert!(!root.join("segment-0001").exists());
    assert!(!String::from_utf8_lossy(&r.stdout).contains("TRAIN_START"));
    if final_path.exists(){
        let bytes=std::fs::read(&final_path).unwrap();std::fs::write(&final_path,&bytes[..12]).unwrap();
        std::fs::remove_file(root.join("segment-0000-finished.pending.r3b")).unwrap();
        assert!(!call(&["fresh","run","--root",root.to_str().unwrap()],None).status.success());
        assert!(!root.join("segment-0001").exists());
    }
    }
    println!("TINY_OPTIMIZER_CALLS=4 final_sync_pending_truncation_new_process=BLOCKED failed_retry_optimizer_generation=0");
}

#[test]
fn fresh_fx04_checkpoint_timeouts_keep_final_evaluation_pending() {
    use candle_core::Device;
    use replica_v3::{binary, neural::checkpoint};
    for fault in [
        "before_training_checkpoint",
        "training_checkpoint_saved",
        "training_optimizer_returned",
    ] {
        let d = tempfile::tempdir().unwrap();
        let root = d.path().join("run");
        let call = |args: &[&str], stop: bool| {
            let mut c = Command::new(env!("CARGO_BIN_EXE_replica-train"));
            c.args(args)
                .env("VECLIB_MAXIMUM_THREADS", "1")
                .env("RAYON_NUM_THREADS", "1")
                .env("R3_FRESH_FIXTURE_EOS", "1");
            if stop {
                c.env("R3_FRESH_TRAIN_STOP", fault);
            }
            let o = c.output().unwrap();
            assert!(
                o.status.success(),
                "{} {}",
                String::from_utf8_lossy(&o.stdout),
                String::from_utf8_lossy(&o.stderr)
            );
        };
        call(
            &["fresh", "fixture", "--output", root.to_str().unwrap()],
            false,
        );
        call(
            &["fresh", "fixture-full", "--root", root.to_str().unwrap()],
            true,
        );
        let s: binary::Value =
            binary::from_slice(&std::fs::read(root.join("segment-0000-finished.r3b")).unwrap())
                .unwrap();
        assert_eq!(s["phase"], "EvaluationPending", "{fault}");
        assert_eq!(s["resume"], true);
        let before = checkpoint::load(&root.join("segment-0000/final"), Device::Cpu, true).unwrap();
        call(&["fresh", "run", "--root", root.to_str().unwrap()], false);
        let after = checkpoint::load(&root.join("segment-0001/final"), Device::Cpu, true).unwrap();
        assert_eq!(
            before.model.weight_hash().unwrap(),
            after.model.weight_hash().unwrap()
        );
        assert_eq!(before.manifest.training, after.manifest.training);
        for (k, t) in &before.optimizer {
            assert_eq!(
                t.flatten_all().unwrap().to_vec1::<f32>().unwrap(),
                after.optimizer[k]
                    .flatten_all()
                    .unwrap()
                    .to_vec1::<f32>()
                    .unwrap()
            );
        }
        let c: binary::Value = binary::from_slice(
            &std::fs::read(root.join("segment-0001/train-control.r3b")).unwrap(),
        )
        .unwrap();
        assert_eq!(c["optimizer_calls"], 0);
        assert_eq!(c["generation_calls"], 24);
        assert_eq!(c["teacher_calls"], 24);
    }
    println!("TINY_UPDATES=6 GENERATION=72 TEACHER=72 FX04_PROCESS_CASES=3");
}

#[test]
fn fresh_fx03_final_step_resumes_only_remaining_evaluation() {
    let d=tempfile::tempdir().unwrap();let root=d.path().join("run");
    let call=|args:&[&str],fault:Option<&str>| {let mut c=Command::new(env!("CARGO_BIN_EXE_replica-train"));c.args(args).env("VECLIB_MAXIMUM_THREADS","1").env("RAYON_NUM_THREADS","1");if let Some(f)=fault{c.env("R3_FRESH_TEST_STOP",f);}let o=c.output().unwrap();assert!(o.status.success(),"{}\n{}",String::from_utf8_lossy(&o.stdout),String::from_utf8_lossy(&o.stderr));};
    call(&["fresh","fixture","--output",root.to_str().unwrap()],None);
    call(&["fresh","fixture-full","--root",root.to_str().unwrap()],Some("final-row-1"));
    assert!(root.join("eval-0002-train64.r3rows").exists(),"TINY must execute shared evaluation");
    let before=replica_v3::neural::checkpoint::load(&root.join("segment-0000/final"),candle_core::Device::Cpu,true).unwrap();
    call(&["fresh","run","--root",root.to_str().unwrap()],None);
    let after=replica_v3::neural::checkpoint::load(&root.join("segment-0001/final"),candle_core::Device::Cpu,true).unwrap();
    assert_eq!(before.model.weight_hash().unwrap(),after.model.weight_hash().unwrap());
    assert_eq!(before.manifest.training,after.manifest.training);
    for (k,t) in &before.optimizer{assert_eq!(t.flatten_all().unwrap().to_vec1::<f32>().unwrap(),after.optimizer[k].flatten_all().unwrap().to_vec1::<f32>().unwrap());}
}

#[test]
fn fresh_fx03_middle_last_summary_and_unknown_process_boundaries() {
    use replica_v3::{binary,neural::checkpoint};use candle_core::Device;
    for fault in ["final-train64-row-4","final-transfer128-row-8","final-transfer128-summary"] {
        let d=tempfile::tempdir().unwrap();let root=d.path().join("run");
        let call=|args:&[&str],fault:Option<&str>,ok:bool|{let mut c=Command::new(env!("CARGO_BIN_EXE_replica-train"));c.args(args).env("VECLIB_MAXIMUM_THREADS","1").env("RAYON_NUM_THREADS","1");if let Some(f)=fault{c.env("R3_FRESH_TEST_STOP",f);}let o=c.output().unwrap();assert_eq!(o.status.success(),ok,"{}\n{}",String::from_utf8_lossy(&o.stdout),String::from_utf8_lossy(&o.stderr));o};
        call(&["fresh","fixture","--output",root.to_str().unwrap()],None,true);
        call(&["fresh","fixture-full","--root",root.to_str().unwrap()],Some(fault),true);
        let before=checkpoint::load(&root.join("segment-0000/final"),Device::Cpu,true).unwrap();
        let paths:Vec<_>=std::fs::read_dir(&root).unwrap().map(|e|e.unwrap().path()).filter(|p|p.extension().is_some_and(|x|x=="r3rows")).collect();
        let prefixes:Vec<_>=paths.iter().map(|p|(p.clone(),std::fs::read(p).unwrap())).collect();
        call(&["fresh","run","--root",root.to_str().unwrap()],None,true);
        let after=checkpoint::load(&root.join("segment-0001/final"),Device::Cpu,true).unwrap();
        assert_eq!(before.manifest.training,after.manifest.training);assert_eq!(before.model.weight_hash().unwrap(),after.model.weight_hash().unwrap());
        for(k,t)in &before.optimizer{assert_eq!(t.flatten_all().unwrap().to_vec1::<f32>().unwrap(),after.optimizer[k].flatten_all().unwrap().to_vec1::<f32>().unwrap());}
        for(p,old)in prefixes{assert!(std::fs::read(p).unwrap().starts_with(&old));}
        let receipt=|n|->binary::Value{binary::from_slice(&std::fs::read(root.join(format!("segment-{n:04}/train-control.r3b"))).unwrap()).unwrap()};
        let a=receipt(0);let b=receipt(1);assert_eq!(b["optimizer_calls"],0);assert_eq!(b["final_evaluation_complete"],true);
        assert_eq!(a["generation_calls"].as_u64().unwrap()+b["generation_calls"].as_u64().unwrap(),24);
        assert_eq!(a["teacher_calls"].as_u64().unwrap()+b["teacher_calls"].as_u64().unwrap(),24);
        call(&["fresh","run","--root",root.to_str().unwrap()],None,false);
    }
    println!("TINY_OPTIMIZER_CALLS=6 distinct_process_boundaries=3 completed_prefix_unchanged=true");
}

#[test]
fn fresh_explicit_fork_matches_continuous_and_split_native_resume() {
    use candle_core::Device;
    use replica_v3::{binary, neural::checkpoint};
    let d = tempfile::tempdir().unwrap();
    let parent = d.path().join("parent");
    let study = d.path().join("study");
    let call = |args: &[&str]| {
        let out = Command::new(env!("CARGO_BIN_EXE_replica-train"))
            .args(args)
            .env("VECLIB_MAXIMUM_THREADS", "1")
            .env("RAYON_NUM_THREADS", "1")
            .env("R3_FRESH_FIXTURE_EOS", "1")
            .output()
            .unwrap();
        assert!(
            out.status.success(),
            "{}\n{}",
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        );
    };
    call(&["fresh", "fixture", "--output", parent.to_str().unwrap()]);
    call(&["fresh", "fixture-full", "--root", parent.to_str().unwrap()]);
    call(&[
        "fresh",
        "study-prepare",
        "--parent",
        parent.to_str().unwrap(),
        "--output",
        study.to_str().unwrap(),
    ]);
    call(&["fresh", "study-observe", "--root", study.to_str().unwrap()]);
    let c = study.join("C-REPEAT");
    let p = study.join("P-PHRASE");
    call(&["fresh", "fixture-full", "--root", p.to_str().unwrap()]);
    let initial = checkpoint::load(&c.join("initial.r3m"), Device::Cpu, true).unwrap();
    let other = checkpoint::load(&p.join("initial.r3m"), Device::Cpu, true).unwrap();
    assert_eq!(
        initial.model.weight_hash().unwrap(),
        other.model.weight_hash().unwrap()
    );
    assert_eq!(initial.manifest.training, other.manifest.training);
    for (k, t) in &initial.optimizer {
        assert_eq!(
            t.flatten_all().unwrap().to_vec1::<f32>().unwrap(),
            other.optimizer[k]
                .flatten_all()
                .unwrap()
                .to_vec1::<f32>()
                .unwrap()
        );
    }
    for root in [&c, &p] {
        let plan: binary::Value =
            binary::from_slice(&std::fs::read(root.join("plan.r3b")).unwrap()).unwrap();
        assert_eq!(plan["config"]["max_steps"], 4);
        for n in 0..2 {
            let path = root.join(format!("segment-{n:04}/updates.r3rows"));
            if !path.exists() {
                continue;
            }
            for row in binary::read_value_records(&path).unwrap() {
                assert_eq!(row["lr_bits"], 3e-5f64.to_bits());
            }
        }
    }
    // P is a real completed parent. Exercise both selector arms through the same native path.
    let selector = d.path().join("selector");
    call(&[
        "fresh",
        "study-prepare",
        "--selector",
        "--parent",
        p.to_str().unwrap(),
        "--output",
        selector.to_str().unwrap(),
    ]);
    call(&[
        "fresh",
        "study-observe",
        "--root",
        selector.to_str().unwrap(),
    ]);
    let c = selector.join("C-KEEP");
    let selected = selector.join("S-SELECT");
    call(&["fresh", "run", "--root", c.to_str().unwrap()]);
    call(&["fresh", "run", "--root", c.to_str().unwrap()]);
    call(&[
        "fresh",
        "fixture-full",
        "--root",
        selected.to_str().unwrap(),
    ]);
    call(&[
        "fresh",
        "study-report",
        "--root",
        selector.to_str().unwrap(),
    ]);
    // Same policy C in a second disposable study provides continuous versus1+1 equality.
    let other_study = d.path().join("continuous");
    call(&[
        "fresh",
        "study-prepare",
        "--selector",
        "--parent",
        p.to_str().unwrap(),
        "--output",
        other_study.to_str().unwrap(),
    ]);
    call(&[
        "fresh",
        "study-observe",
        "--root",
        other_study.to_str().unwrap(),
    ]);
    let full = other_study.join("C-KEEP");
    call(&["fresh", "fixture-full", "--root", full.to_str().unwrap()]);
    let a = checkpoint::load(&full.join("segment-0000/final"), Device::Cpu, true).unwrap();
    let b = checkpoint::load(&c.join("segment-0001/final"), Device::Cpu, true).unwrap();
    assert_eq!(
        a.model.weight_hash().unwrap(),
        b.model.weight_hash().unwrap()
    );
    for (k, t) in &a.optimizer {
        assert_eq!(
            t.flatten_all().unwrap().to_vec1::<f32>().unwrap(),
            b.optimizer[k]
                .flatten_all()
                .unwrap()
                .to_vec1::<f32>()
                .unwrap()
        );
    }
    let a = a.manifest.training.unwrap();
    let b = b.manifest.training.unwrap();
    assert_eq!(
        (a.step, a.sampler_state, a.consumed_tokens, a.target_tokens),
        (b.step, b.sampler_state, b.consumed_tokens, b.target_tokens)
    );
    println!(
        "TINY_OPTIMIZER_CALLS=10 TINY_GENERATIONS=167 TINY_TEACHERS=167 actual_selector_parent_fork=VERIFIED constant_LR_bits=VERIFIED continuous_vs_fresh_resume=EXACT"
    );
}

#[test]
fn fresh_value_exposure_restores_identical_native_state() {
    value_exposure_native_resume(false, false, false);
}

#[test]
fn fresh_value_coverage_restores_identical_native_state() {
    value_exposure_native_resume(true, false, false);
}

#[test]
fn fresh_cover_phrase_restores_identical_native_state() {
    value_exposure_native_resume(true, true, false);
}

#[test]
fn fresh_value_diversity_restores_identical_native_state() {
    value_exposure_native_resume(false, false, true);
}

#[test]
fn fresh_four_view_coverage_restores_identical_native_state() {
    value_exposure_native_resume(true, false, true);
}

fn value_exposure_native_resume(coverage: bool, wording: bool, diversity: bool) {
    use replica_v3::{binary,neural::checkpoint};
    use candle_core::Device;
    let d=tempfile::tempdir().unwrap();
    let (mode,flag,steps,cycle)=if coverage&&diversity {("COVER4","--diverse-pair-values",16,4)}else if diversity {("DIVERSE","--diverse-pair-values",8,2)}else if coverage {("COVER","--cover-value-pairs",8,4)}else{("VALUE","--alternate-pair-values",4,2)};
    let call=|args:&[&str],success:bool| {
        let out=Command::new(env!("CARGO_BIN_EXE_replica-train")).args(args)
            .env("VECLIB_MAXIMUM_THREADS","1").env("RAYON_NUM_THREADS","1")
            .env("R3_FRESH_FIXTURE_VALUE_DONORS",if diversity {"1"}else{"0"})
            .env("R3_FRESH_FIXTURE_EOS","1").output().unwrap();
        println!("VALUE_TEST_COMMAND {args:?} exit={}\n{}\n{}",out.status,String::from_utf8_lossy(&out.stdout),String::from_utf8_lossy(&out.stderr));
        assert_eq!(out.status.success(),success);
    };
    let parent=d.path().join("parent");let phrase=d.path().join("phrase");let selector=d.path().join("selector");
    call(&["fresh","fixture","--output",parent.to_str().unwrap()],true);
    call(&["fresh","fixture-full","--root",parent.to_str().unwrap()],true);
    call(&["fresh","study-prepare","--parent",parent.to_str().unwrap(),"--output",phrase.to_str().unwrap()],true);
    call(&["fresh","study-observe","--root",phrase.to_str().unwrap()],true);
    let p=phrase.join("P-PHRASE");
    call(&["fresh","fixture-full","--root",p.to_str().unwrap()],true);
    call(&["fresh","study-prepare","--selector","--parent",p.to_str().unwrap(),"--output",selector.to_str().unwrap()],true);
    call(&["fresh","study-observe","--root",selector.to_str().unwrap()],true);
    let roots=[d.path().join("continuous"),d.path().join("split")];
    for (i,root) in roots.iter().enumerate() {
        let source=selector.join("S-SELECT");
        let mut args=vec!["fresh","paired-prepare","--parent",p.to_str().unwrap(),"--source-data",source.to_str().unwrap(),
            "--output",root.to_str().unwrap(),flag];
        if coverage&&diversity {args.push("--cover-value-pairs");}
        call(&args,true);
        let arm=root.join(mode);
        let plan:binary::Value=binary::from_slice(&std::fs::read(arm.join("plan.r3b")).unwrap()).unwrap();
        let rows=plan["paired"]["rows"].as_array().unwrap();assert_eq!(rows.len(),steps);
        assert_ne!(rows[0].as_array().unwrap()[..6],rows[cycle].as_array().unwrap()[..6]);
        if coverage {assert_ne!(rows[0].as_array().unwrap()[..6],rows[2].as_array().unwrap()[..6]);}
        if diversity {
            assert!(plan["training_values"].as_str().is_some());
            let native=std::fs::read(arm.join("training-values.r3cor")).unwrap();
            assert_eq!(plan["training_values"],replica_v3::neural::hash(&native));
            // The real run must reject changed input before any new process calls.
            std::fs::write(arm.join("training-values.r3cor"),b"invalid fixture").unwrap();
            call(&["fresh","run","--root",arm.to_str().unwrap()],false);
            assert!(!arm.join("segment-0000-started.r3b").exists());
            std::fs::write(arm.join("training-values.r3cor"),native).unwrap();
        }
        if i==0 {call(&["fresh","fixture-full","--root",arm.to_str().unwrap()],true);}
        else {for _ in 0..2 {call(&["fresh","run","--root",arm.to_str().unwrap()],true);}}
        call(&["fresh","paired-report","--root",root.to_str().unwrap()],true);
    }
    let a=checkpoint::load(&roots[0].join(mode).join("segment-0000/final"),Device::Cpu,true).unwrap();
    let b=checkpoint::load(&roots[1].join(mode).join("segment-0001/final"),Device::Cpu,true).unwrap();
    assert_eq!(a.model.weight_hash().unwrap(),b.model.weight_hash().unwrap());
    for (name,t) in &a.optimizer {assert_eq!(t.flatten_all().unwrap().to_vec1::<f32>().unwrap(),b.optimizer[name].flatten_all().unwrap().to_vec1::<f32>().unwrap());}
    let x=a.manifest.training.as_ref().unwrap();let y=b.manifest.training.as_ref().unwrap();
    assert_eq!((x.step,x.sampler_state,x.consumed_tokens,x.target_tokens),(y.step,y.sampler_state,y.consumed_tokens,y.target_tokens));
    assert_eq!(x.step,4+steps);assert_eq!(x.resume_binding.as_ref().unwrap().family,4);
    assert_eq!(x.config.lr.to_bits(),3e-5f64.to_bits());assert_eq!(x.config.first_target_weight,1.);
    assert!(checkpoint::ResumeBinding::require_default(x,&a.tokenizer).is_err());
    let bad=d.path().join("mixed");
    call(&["fresh","paired-prepare","--parent",p.to_str().unwrap(),"--source-data",selector.join("S-SELECT").to_str().unwrap(),
        "--output",bad.to_str().unwrap(),flag,"--fit-seen-pairs"],false);assert!(!bad.exists());
    if coverage && !diversity {
        call(&["fresh","paired-prepare","--parent",p.to_str().unwrap(),"--source-data",selector.join("S-SELECT").to_str().unwrap(),
            "--output",bad.to_str().unwrap(),flag,"--alternate-pair-values"],false);assert!(!bad.exists());
        fn hashes(root:&std::path::Path)->std::collections::BTreeMap<std::path::PathBuf,String> {
            let mut out=std::collections::BTreeMap::new();
            for e in std::fs::read_dir(root).unwrap() {
                let p=e.unwrap().path();if p.is_dir(){out.extend(hashes(&p));}
                else{out.insert(p.clone(),replica_v3::neural::hash(&std::fs::read(p).unwrap()));}
            }out
        }
        let follow=[d.path().join("exposure-continuous"),d.path().join("exposure-split")];
        let continuation_flag=if wording {"--selector-phrase-exposure"}else{"--fixed-cover-exposure"};
        for (i,root) in follow.iter().enumerate() {
            let parent=roots[i].join(mode);let before=hashes(&parent);
            call(&["fresh","paired-continue","--parent",parent.to_str().unwrap(),"--output",bad.to_str().unwrap(),
                "--frozen-executable",env!("CARGO_BIN_EXE_replica-train")],false);assert!(!bad.exists());
            call(&["fresh","paired-continue","--parent",parent.to_str().unwrap(),"--output",root.to_str().unwrap(),
                "--frozen-executable",env!("CARGO_BIN_EXE_replica-train"),continuation_flag],true);
            assert_eq!(before,hashes(&parent));
            let arm=root.join(mode);
            let old:binary::Value=binary::from_slice(&std::fs::read(parent.join("plan.r3b")).unwrap()).unwrap();
            let new:binary::Value=binary::from_slice(&std::fs::read(arm.join("plan.r3b")).unwrap()).unwrap();
            if wording {
                assert_ne!(old["paired"]["rows"],new["paired"]["rows"]);
                assert_eq!(old["paired"]["rows"][4],new["paired"]["rows"][4]);
                assert_eq!(old["paired"]["rows"][7],new["paired"]["rows"][7]);
            } else {assert_eq!(old["paired"]["rows"],new["paired"]["rows"]);}
            assert_eq!(new["paired"]["first_step"],12); // Explicit TINY cycle, not the SMALL cursor.
            for key in ["tokenizer","corpus","transfer","order","metadata"] {assert_eq!(old[key],new[key]);}
            if i==0 {call(&["fresh","fixture-full","--root",arm.to_str().unwrap()],true);}
            else {for _ in 0..2 {call(&["fresh","run","--root",arm.to_str().unwrap()],true);}}
            let finished=hashes(root);
            call(&["fresh","paired-report","--root",root.to_str().unwrap()],true);assert_eq!(finished,hashes(root));
            call(&["fresh","paired-continue","--parent",arm.to_str().unwrap(),"--output",bad.to_str().unwrap(),
                "--frozen-executable",env!("CARGO_BIN_EXE_replica-train"),continuation_flag],false);assert!(!bad.exists());
            assert_eq!(before,hashes(&parent));
        }
        let a=checkpoint::load(&follow[0].join("COVER/segment-0000/final"),Device::Cpu,true).unwrap();
        let b=checkpoint::load(&follow[1].join("COVER/segment-0001/final"),Device::Cpu,true).unwrap();
        assert_eq!(a.model.weight_hash().unwrap(),b.model.weight_hash().unwrap());
        for (k,t) in &a.optimizer {assert_eq!(t.flatten_all().unwrap().to_vec1::<f32>().unwrap(),b.optimizer[k].flatten_all().unwrap().to_vec1::<f32>().unwrap());}
        let x=a.manifest.training.as_ref().unwrap();let y=b.manifest.training.as_ref().unwrap();
        assert_eq!((x.step,x.sampler_state,x.consumed_tokens,x.target_tokens),(y.step,y.sampler_state,y.consumed_tokens,y.target_tokens));
        assert_eq!(x.step,20);assert_eq!(x.resume_binding.as_ref().unwrap().family,4);
        assert_eq!(x.config.lr.to_bits(),3e-5f64.to_bits());assert_eq!(x.config.first_target_weight,1.);
        assert!(checkpoint::ResumeBinding::require_default(x,&a.tokenizer).is_err());
    }
    let mut used=[0u64;3];
    fn count(root:&std::path::Path,used:&mut [u64;3]) {
        for entry in std::fs::read_dir(root).unwrap() {
            let path=entry.unwrap().path();if path.is_dir(){count(&path,used);}
            else if path.file_name().unwrap()=="train-control.r3b" {
                let row:binary::Value=binary::from_slice(&std::fs::read(path).unwrap()).unwrap();
                for (i,key) in ["optimizer_calls","generation_calls","teacher_calls"].iter().enumerate(){used[i]+=row[*key].as_u64().unwrap();}
            }
        }
    }
    count(d.path(),&mut used);println!("VALUE_TINY_TRAIN_CONTROLS optimizer={} generation={} teacher={}; setup observations counted separately",used[0],used[1],used[2]);
}

#[test]
fn fresh_paired_policies_match_continuous_and_split_processes() {
    use replica_v3::{binary, neural::checkpoint};
    use candle_core::Device;
    let d = tempfile::tempdir().unwrap();
    let call = |args: &[&str], fault: Option<&str>, success: bool| {
        let mut c = Command::new(env!("CARGO_BIN_EXE_replica-train"));
        c.args(args).env("VECLIB_MAXIMUM_THREADS", "1").env("RAYON_NUM_THREADS", "1").env("R3_FRESH_FIXTURE_EOS", "1");
        if let Some(f) = fault { c.env("R3_FRESH_TEST_STOP", f); }
        let out = c.output().unwrap();
        println!("PAIRED_TEST_COMMAND {args:?} fault={fault:?} exit={}\n{}\n{}", out.status, String::from_utf8_lossy(&out.stdout), String::from_utf8_lossy(&out.stderr));
        assert_eq!(out.status.success(),success,"{}\n{}",String::from_utf8_lossy(&out.stdout),String::from_utf8_lossy(&out.stderr));
        out
    };
    let parent=d.path().join("parent");let phrase=d.path().join("phrase");let selector=d.path().join("selector");
    call(&["fresh","fixture","--output",parent.to_str().unwrap()],None,true);
    call(&["fresh","fixture-full","--root",parent.to_str().unwrap()],None,true);
    call(&["fresh","study-prepare","--parent",parent.to_str().unwrap(),"--output",phrase.to_str().unwrap()],None,true);
    call(&["fresh","study-observe","--root",phrase.to_str().unwrap()],None,true);
    let p=phrase.join("P-PHRASE");
    call(&["fresh","fixture-full","--root",p.to_str().unwrap()],None,true);
    call(&["fresh","study-prepare","--selector","--parent",p.to_str().unwrap(),"--output",selector.to_str().unwrap()],None,true);
    call(&["fresh","study-observe","--root",selector.to_str().unwrap()],None,true);
    let roots=[d.path().join("continuous"),d.path().join("split")];
    for root in &roots {
        call(&["fresh","paired-prepare","--parent",p.to_str().unwrap(),"--source-data",selector.join("S-SELECT").to_str().unwrap(),"--output",root.to_str().unwrap()],None,true);
    }
    for arm in ["SPACED","ADJACENT"] {
        let a=roots[0].join(arm);let b=roots[1].join(arm);
        call(&["fresh","fixture-full","--root",a.to_str().unwrap()],None,true);
        call(&["fresh","run","--root",b.to_str().unwrap()],None,true);
        call(&["fresh","run","--root",b.to_str().unwrap()],None,true);
        let a=checkpoint::load(&a.join("segment-0000/final"),Device::Cpu,true).unwrap();
        let b=checkpoint::load(&b.join("segment-0001/final"),Device::Cpu,true).unwrap();
        assert_eq!(a.model.weight_hash().unwrap(),b.model.weight_hash().unwrap());
        for (k,t) in &a.optimizer { assert_eq!(t.flatten_all().unwrap().to_vec1::<f32>().unwrap(),b.optimizer[k].flatten_all().unwrap().to_vec1::<f32>().unwrap()); }
        let x=a.manifest.training.unwrap();let y=b.manifest.training.unwrap();
        assert_eq!((x.step,x.sampler_state,x.consumed_tokens,x.target_tokens),(y.step,y.sampler_state,y.consumed_tokens,y.target_tokens));
        assert_eq!(x.step,6);
    }
    fn hashes(root:&std::path::Path)->std::collections::BTreeMap<std::path::PathBuf,String> {
        let mut out=std::collections::BTreeMap::new();
        for e in std::fs::read_dir(root).unwrap() { let p=e.unwrap().path(); if p.is_dir(){out.extend(hashes(&p));}else{out.insert(p.clone(),replica_v3::neural::hash(&std::fs::read(p).unwrap()));} }out
    }
    for root in &roots {
        let before=hashes(root);
        call(&["fresh","paired-report","--root",root.to_str().unwrap()],None,true);
        assert_eq!(before,hashes(root));
    }
    let follow=[d.path().join("follow-continuous"),d.path().join("follow-split")];
    for (i,root) in follow.iter().enumerate() {
        let old=hashes(&roots[i]);
        call(&["fresh","paired-continue","--parent",roots[i].join("ADJACENT").to_str().unwrap(),
            "--output",root.to_str().unwrap(),"--frozen-executable",env!("CARGO_BIN_EXE_replica-train")],None,true);
        assert_eq!(old,hashes(&roots[i]));
        let arm=root.join("ADJACENT");
        if i==0 { call(&["fresh","fixture-full","--root",arm.to_str().unwrap()],None,true); }
        else { for _ in 0..2 {call(&["fresh","run","--root",arm.to_str().unwrap()],None,true);} }
        let before=hashes(root);
        call(&["fresh","paired-report","--root",root.to_str().unwrap()],None,true);
        assert_eq!(before,hashes(root));
    }
    let a=checkpoint::load(&follow[0].join("ADJACENT/segment-0000/final"),Device::Cpu,true).unwrap();
    let b=checkpoint::load(&follow[1].join("ADJACENT/segment-0001/final"),Device::Cpu,true).unwrap();
    assert_eq!(a.model.weight_hash().unwrap(),b.model.weight_hash().unwrap());
    for (k,t) in &a.optimizer {assert_eq!(t.flatten_all().unwrap().to_vec1::<f32>().unwrap(),b.optimizer[k].flatten_all().unwrap().to_vec1::<f32>().unwrap());}
    assert_eq!(a.manifest.training.unwrap().step,8);
    assert_eq!(b.manifest.training.unwrap().step,8);
    let weighted=[d.path().join("first4-continuous"),d.path().join("first4-split")];
    for (i,root) in weighted.iter().enumerate() {
        call(&["fresh","paired-prepare","--parent",p.to_str().unwrap(),"--source-data",selector.join("S-SELECT").to_str().unwrap(),
            "--output",root.to_str().unwrap(),"--first-target-four"],None,true);
        assert!(!root.join("SPACED").exists());
        let arm=root.join("ADJACENT");
        if i==0 {call(&["fresh","fixture-full","--root",arm.to_str().unwrap()],None,true);}
        else {for _ in 0..2 {call(&["fresh","run","--root",arm.to_str().unwrap()],None,true);}}
        let before=hashes(root);
        call(&["fresh","paired-report","--root",root.to_str().unwrap()],None,true);
        assert_eq!(before,hashes(root));
    }
    let a=checkpoint::load(&weighted[0].join("ADJACENT/segment-0000/final"),Device::Cpu,true).unwrap();
    let b=checkpoint::load(&weighted[1].join("ADJACENT/segment-0001/final"),Device::Cpu,true).unwrap();
    assert_eq!(a.model.weight_hash().unwrap(),b.model.weight_hash().unwrap());
    for (k,t) in &a.optimizer {assert_eq!(t.flatten_all().unwrap().to_vec1::<f32>().unwrap(),b.optimizer[k].flatten_all().unwrap().to_vec1::<f32>().unwrap());}
    for model in [&a,&b] {
        let state=model.manifest.training.as_ref().unwrap();
        assert_eq!(state.step,6);
        assert_eq!(state.config.first_target_weight,4.);
        assert_eq!(state.resume_binding.as_ref().unwrap().first_target_weight_bits,4f64.to_bits());
    }
    let control=checkpoint::load(&roots[0].join("ADJACENT/segment-0000/final"),Device::Cpu,true).unwrap();
    assert_ne!(a.model.weight_hash().unwrap(),control.model.weight_hash().unwrap());
    let lr_roots=[d.path().join("lr-continuous"),d.path().join("lr-split")];
    for (i,root) in lr_roots.iter().enumerate() {
        call(&["fresh","paired-prepare","--parent",p.to_str().unwrap(),"--source-data",selector.join("S-SELECT").to_str().unwrap(),
            "--output",root.to_str().unwrap(),"--learning-rate-threefold"],None,true);
        assert!(!root.join("SPACED").exists());
        let arm=root.join("ADJACENT");
        if i==0 {call(&["fresh","fixture-full","--root",arm.to_str().unwrap()],None,true);}
        else {for _ in 0..2 {call(&["fresh","run","--root",arm.to_str().unwrap()],None,true);}}
        let before=hashes(root);
        call(&["fresh","paired-report","--root",root.to_str().unwrap()],None,true);
        assert_eq!(before,hashes(root));
    }
    let a=checkpoint::load(&lr_roots[0].join("ADJACENT/segment-0000/final"),Device::Cpu,true).unwrap();
    let b=checkpoint::load(&lr_roots[1].join("ADJACENT/segment-0001/final"),Device::Cpu,true).unwrap();
    assert_eq!(a.model.weight_hash().unwrap(),b.model.weight_hash().unwrap());
    for (k,t) in &a.optimizer {assert_eq!(t.flatten_all().unwrap().to_vec1::<f32>().unwrap(),b.optimizer[k].flatten_all().unwrap().to_vec1::<f32>().unwrap());}
    for model in [&a,&b] {
        let state=model.manifest.training.as_ref().unwrap();
        assert_eq!(state.step,6);
        assert_eq!(state.config.lr.to_bits(),9e-5f64.to_bits());
        assert_eq!(state.config.first_target_weight,1.);
        assert_eq!(state.resume_binding.as_ref().unwrap().first_target_weight_bits,1f64.to_bits());
    }
    assert_ne!(a.model.weight_hash().unwrap(),control.model.weight_hash().unwrap());
    let rejected=d.path().join("combined-intervention-rejected");
    call(&["fresh","paired-prepare","--parent",p.to_str().unwrap(),"--source-data",selector.join("S-SELECT").to_str().unwrap(),
        "--output",rejected.to_str().unwrap(),"--first-target-four","--learning-rate-threefold"],None,false);
    assert!(!rejected.exists());
    let co_roots=[d.path().join("co-continuous"),d.path().join("co-split")];
    for (i,root) in co_roots.iter().enumerate() {
        call(&["fresh","paired-prepare","--parent",p.to_str().unwrap(),"--source-data",selector.join("S-SELECT").to_str().unwrap(),
            "--output",root.to_str().unwrap(),"--co-batch"],None,true);
        assert!(!root.join("ADJACENT").exists());
        let arm=root.join("COBATCH");
        if i==0 {call(&["fresh","fixture-full","--root",arm.to_str().unwrap()],None,true);}
        else {for _ in 0..2 {call(&["fresh","run","--root",arm.to_str().unwrap()],None,true);}}
        let before=hashes(root);
        call(&["fresh","paired-report","--root",root.to_str().unwrap()],None,true);
        assert_eq!(before,hashes(root));
    }
    let a=checkpoint::load(&co_roots[0].join("COBATCH/segment-0000/final"),Device::Cpu,true).unwrap();
    let b=checkpoint::load(&co_roots[1].join("COBATCH/segment-0001/final"),Device::Cpu,true).unwrap();
    assert_eq!(a.model.weight_hash().unwrap(),b.model.weight_hash().unwrap());
    for (k,t) in &a.optimizer {assert_eq!(t.flatten_all().unwrap().to_vec1::<f32>().unwrap(),b.optimizer[k].flatten_all().unwrap().to_vec1::<f32>().unwrap());}
    let state=a.manifest.training.as_ref().unwrap();let old=control.manifest.training.as_ref().unwrap();
    assert_eq!(state.config,old.config);assert_eq!(state.step,old.step);assert_eq!(state.target_tokens,old.target_tokens);
    assert_eq!(state.consumed_tokens,old.consumed_tokens);
    assert_eq!(state.resume_binding.as_ref().unwrap().first_target_weight_bits,1f64.to_bits());
    assert_ne!(a.model.weight_hash().unwrap(),control.model.weight_hash().unwrap());
    let mixed=d.path().join("mixed-packing-rejected");
    call(&["fresh","paired-prepare","--parent",p.to_str().unwrap(),"--source-data",selector.join("S-SELECT").to_str().unwrap(),
        "--output",mixed.to_str().unwrap(),"--co-batch","--learning-rate-threefold"],None,false);
    assert!(!mixed.exists());
    let side_roots=[d.path().join("side-continuous"),d.path().join("side-split")];
    for (i,root) in side_roots.iter().enumerate() {
        call(&["fresh","paired-prepare","--parent",p.to_str().unwrap(),"--source-data",selector.join("S-SELECT").to_str().unwrap(),
            "--output",root.to_str().unwrap(),"--sidewise-contrast"],None,true);
        let arm=root.join("SIDE");
        if i==0 {call(&["fresh","fixture-full","--root",arm.to_str().unwrap()],None,true);}
        else {for _ in 0..2 {call(&["fresh","run","--root",arm.to_str().unwrap()],None,true);}}
        let before=hashes(root);
        call(&["fresh","paired-report","--root",root.to_str().unwrap()],None,true);
        assert_eq!(before,hashes(root));
    }
    let side=checkpoint::load(&side_roots[0].join("SIDE/segment-0000/final"),Device::Cpu,true).unwrap();
    let split=checkpoint::load(&side_roots[1].join("SIDE/segment-0001/final"),Device::Cpu,true).unwrap();
    assert_eq!(side.model.weight_hash().unwrap(),split.model.weight_hash().unwrap());
    for (k,t) in &side.optimizer {assert_eq!(t.flatten_all().unwrap().to_vec1::<f32>().unwrap(),split.optimizer[k].flatten_all().unwrap().to_vec1::<f32>().unwrap());}
    let state=side.manifest.training.as_ref().unwrap();let old=a.manifest.training.as_ref().unwrap();
    assert_eq!(state.config,old.config);assert_eq!(state.step,old.step);
    assert_eq!(state.target_tokens,old.target_tokens);assert_eq!(state.consumed_tokens,old.consumed_tokens);
    let binding=state.resume_binding.as_ref().unwrap();
    assert_eq!(binding.family,4);assert_eq!(binding.normalizer,4);
    assert_eq!(binding.span_alpha_bits,Some(0.1f64.to_bits()));assert!(binding.annotation.is_some());
    assert!(checkpoint::ResumeBinding::require_default(state,&side.tokenizer).is_err());
    assert_ne!(side.model.weight_hash().unwrap(),a.model.weight_hash().unwrap());
    let repeat_roots=[d.path().join("repeat-continuous"),d.path().join("repeat-split")];
    for (i,root) in repeat_roots.iter().enumerate() {
        call(&["fresh","paired-prepare","--parent",p.to_str().unwrap(),"--source-data",selector.join("S-SELECT").to_str().unwrap(),
            "--output",root.to_str().unwrap(),"--repeat-pair-block"],None,true);
        let arm=root.join("REPLAY");
        let plan:binary::Value=binary::from_slice(&std::fs::read(arm.join("plan.r3b")).unwrap()).unwrap();
        assert_eq!(plan["config"]["max_steps"],8);
        let rows=plan["paired"]["rows"].as_array().unwrap();assert_eq!(rows.len(),4);
        assert_eq!(rows[0].as_array().unwrap()[..6],rows[2].as_array().unwrap()[..6]);
        if i==0 {call(&["fresh","fixture-full","--root",arm.to_str().unwrap()],None,true);}
        else {for _ in 0..2 {call(&["fresh","run","--root",arm.to_str().unwrap()],None,true);}}
        let before=hashes(root);call(&["fresh","paired-report","--root",root.to_str().unwrap()],None,true);assert_eq!(before,hashes(root));
    }
    let repeat=checkpoint::load(&repeat_roots[0].join("REPLAY/segment-0000/final"),Device::Cpu,true).unwrap();
    let split=checkpoint::load(&repeat_roots[1].join("REPLAY/segment-0001/final"),Device::Cpu,true).unwrap();
    assert_eq!(repeat.model.weight_hash().unwrap(),split.model.weight_hash().unwrap());
    for (k,t) in &repeat.optimizer {assert_eq!(t.flatten_all().unwrap().to_vec1::<f32>().unwrap(),split.optimizer[k].flatten_all().unwrap().to_vec1::<f32>().unwrap());}
    let state=repeat.manifest.training.as_ref().unwrap();assert_eq!(state.step,8);
    assert_eq!(state.resume_binding.as_ref().unwrap().family,4);
    assert_eq!(state.resume_binding.as_ref().unwrap().normalizer,4);
    assert_eq!(state.config.lr.to_bits(),3e-5f64.to_bits());assert_eq!(state.config.first_target_weight,1.);
    assert!(checkpoint::ResumeBinding::require_default(state,&repeat.tokenizer).is_err());
    let mixed=d.path().join("mixed-recurrence-rejected");
    call(&["fresh","paired-prepare","--parent",p.to_str().unwrap(),"--source-data",selector.join("S-SELECT").to_str().unwrap(),
        "--output",mixed.to_str().unwrap(),"--repeat-pair-block","--sidewise-contrast"],None,false);
    assert!(!mixed.exists());
    let wide_roots=[d.path().join("wide-continuous"),d.path().join("wide-split")];
    for (i,root) in wide_roots.iter().enumerate() {
        call(&["fresh","paired-prepare","--parent",p.to_str().unwrap(),"--source-data",selector.join("S-SELECT").to_str().unwrap(),
            "--output",root.to_str().unwrap(),"--repeat-two-blocks"],None,true);
        let arm=root.join("WIDE");
        let plan:binary::Value=binary::from_slice(&std::fs::read(arm.join("plan.r3b")).unwrap()).unwrap();
        assert_eq!(plan["config"]["max_steps"],12);
        let rows=plan["paired"]["rows"].as_array().unwrap();assert_eq!(rows.len(),8);
        assert_eq!(rows[0].as_array().unwrap()[..6],rows[4].as_array().unwrap()[..6]);
        assert_eq!(rows[2].as_array().unwrap()[..6],rows[6].as_array().unwrap()[..6]);
        assert_ne!(rows[0].as_array().unwrap()[..6],rows[2].as_array().unwrap()[..6]);
        if i==0 {call(&["fresh","fixture-full","--root",arm.to_str().unwrap()],None,true);}
        else {for _ in 0..2 {call(&["fresh","run","--root",arm.to_str().unwrap()],None,true);}}
        let before=hashes(root);call(&["fresh","paired-report","--root",root.to_str().unwrap()],None,true);assert_eq!(before,hashes(root));
    }
    let wide=checkpoint::load(&wide_roots[0].join("WIDE/segment-0000/final"),Device::Cpu,true).unwrap();
    let split=checkpoint::load(&wide_roots[1].join("WIDE/segment-0001/final"),Device::Cpu,true).unwrap();
    assert_eq!(wide.model.weight_hash().unwrap(),split.model.weight_hash().unwrap());
    for (k,t) in &wide.optimizer {assert_eq!(t.flatten_all().unwrap().to_vec1::<f32>().unwrap(),split.optimizer[k].flatten_all().unwrap().to_vec1::<f32>().unwrap());}
    let state=wide.manifest.training.as_ref().unwrap();let other=split.manifest.training.as_ref().unwrap();
    assert_eq!((state.step,state.sampler_state,state.consumed_tokens,state.target_tokens),(other.step,other.sampler_state,other.consumed_tokens,other.target_tokens));
    assert_eq!(state.step,12);assert_eq!(state.resume_binding.as_ref().unwrap().family,4);
    assert_eq!(state.resume_binding.as_ref().unwrap().normalizer,4);
    assert_eq!(state.config.lr.to_bits(),3e-5f64.to_bits());assert_eq!(state.config.first_target_weight,1.);
    assert!(checkpoint::ResumeBinding::require_default(state,&wide.tokenizer).is_err());
    let mixed=d.path().join("mixed-recurrence-width-rejected");
    call(&["fresh","paired-prepare","--parent",p.to_str().unwrap(),"--source-data",selector.join("S-SELECT").to_str().unwrap(),
        "--output",mixed.to_str().unwrap(),"--repeat-two-blocks","--repeat-pair-block"],None,false);
    assert!(!mixed.exists());
    let mixed=d.path().join("mixed-side-rejected");
    let fit_roots=[d.path().join("fit-continuous"),d.path().join("fit-split")];
    for (i,root) in fit_roots.iter().enumerate() {
        call(&["fresh","paired-prepare","--parent",p.to_str().unwrap(),"--source-data",selector.join("S-SELECT").to_str().unwrap(),
            "--output",root.to_str().unwrap(),"--fit-seen-pairs"],None,true);
        let arm=root.join("FIT");
        let plan:binary::Value=binary::from_slice(&std::fs::read(arm.join("plan.r3b")).unwrap()).unwrap();
        assert_eq!(plan["config"]["max_steps"],8);
        let rows=plan["paired"]["rows"].as_array().unwrap();assert_eq!(rows.len(),4);
        assert_eq!(rows[0].as_array().unwrap()[..6],rows[2].as_array().unwrap()[..6]);
        if i==0 {call(&["fresh","fixture-full","--root",arm.to_str().unwrap()],None,true);}
        else {for _ in 0..2 {call(&["fresh","run","--root",arm.to_str().unwrap()],None,true);}}
        let before=hashes(root);call(&["fresh","paired-report","--root",root.to_str().unwrap()],None,true);assert_eq!(before,hashes(root));
    }
    let fit=checkpoint::load(&fit_roots[0].join("FIT/segment-0000/final"),Device::Cpu,true).unwrap();
    let split=checkpoint::load(&fit_roots[1].join("FIT/segment-0001/final"),Device::Cpu,true).unwrap();
    assert_eq!(fit.model.weight_hash().unwrap(),split.model.weight_hash().unwrap());
    assert_eq!(fit.model.weight_hash().unwrap(),repeat.model.weight_hash().unwrap());
    for (k,t) in &fit.optimizer {
        assert_eq!(t.flatten_all().unwrap().to_vec1::<f32>().unwrap(),split.optimizer[k].flatten_all().unwrap().to_vec1::<f32>().unwrap());
        assert_eq!(t.flatten_all().unwrap().to_vec1::<f32>().unwrap(),repeat.optimizer[k].flatten_all().unwrap().to_vec1::<f32>().unwrap());
    }
    let state=fit.manifest.training.as_ref().unwrap();let other=split.manifest.training.as_ref().unwrap();
    assert_eq!((state.step,state.sampler_state,state.consumed_tokens,state.target_tokens),(other.step,other.sampler_state,other.consumed_tokens,other.target_tokens));
    assert_eq!(state.step,8);assert_eq!(state.resume_binding.as_ref().unwrap().family,4);
    assert_eq!(state.resume_binding.as_ref().unwrap().normalizer,4);
    assert_eq!(state.config.lr.to_bits(),3e-5f64.to_bits());assert_eq!(state.config.first_target_weight,1.);
    assert!(checkpoint::ResumeBinding::require_default(state,&fit.tokenizer).is_err());
    let mixed_fit=d.path().join("mixed-fit-rejected");
    call(&["fresh","paired-prepare","--parent",p.to_str().unwrap(),"--source-data",selector.join("S-SELECT").to_str().unwrap(),
        "--output",mixed_fit.to_str().unwrap(),"--fit-seen-pairs","--repeat-pair-block"],None,false);
    assert!(!mixed_fit.exists());
    call(&["fresh","paired-prepare","--parent",p.to_str().unwrap(),"--source-data",selector.join("S-SELECT").to_str().unwrap(),
        "--output",mixed.to_str().unwrap(),"--sidewise-contrast","--pair-contrast"],None,false);
    assert!(!mixed.exists());
    let contrast_roots=[d.path().join("contrast-continuous"),d.path().join("contrast-split")];
    for (i,root) in contrast_roots.iter().enumerate() {
        call(&["fresh","paired-prepare","--parent",p.to_str().unwrap(),"--source-data",selector.join("S-SELECT").to_str().unwrap(),
            "--output",root.to_str().unwrap(),"--pair-contrast"],None,true);
        let arm=root.join("CONTRAST");
        if i==0 {call(&["fresh","fixture-full","--root",arm.to_str().unwrap()],None,true);}
        else {for _ in 0..2 {call(&["fresh","run","--root",arm.to_str().unwrap()],None,true);}}
        let before=hashes(root);
        call(&["fresh","paired-report","--root",root.to_str().unwrap()],None,true);
        assert_eq!(before,hashes(root));
    }
    let contrast=checkpoint::load(&contrast_roots[0].join("CONTRAST/segment-0000/final"),Device::Cpu,true).unwrap();
    let split=checkpoint::load(&contrast_roots[1].join("CONTRAST/segment-0001/final"),Device::Cpu,true).unwrap();
    assert_eq!(contrast.model.weight_hash().unwrap(),split.model.weight_hash().unwrap());
    for (k,t) in &contrast.optimizer {assert_eq!(t.flatten_all().unwrap().to_vec1::<f32>().unwrap(),split.optimizer[k].flatten_all().unwrap().to_vec1::<f32>().unwrap());}
    let state=contrast.manifest.training.as_ref().unwrap();let old=a.manifest.training.as_ref().unwrap();
    assert_eq!(state.config,old.config);assert_eq!(state.step,old.step);
    assert_eq!(state.target_tokens,old.target_tokens);assert_eq!(state.consumed_tokens,old.consumed_tokens);
    let binding=state.resume_binding.as_ref().unwrap();
    assert_eq!(binding.family,3);assert_eq!(binding.normalizer,3);
    assert_eq!(binding.span_alpha_bits,Some(0.1f64.to_bits()));assert!(binding.annotation.is_some());
    assert!(checkpoint::ResumeBinding::require_default(state,&contrast.tokenizer).is_err());
    assert_ne!(contrast.model.weight_hash().unwrap(),a.model.weight_hash().unwrap());
    let mixed=d.path().join("mixed-contrast-rejected");
    call(&["fresh","paired-prepare","--parent",p.to_str().unwrap(),"--source-data",selector.join("S-SELECT").to_str().unwrap(),
        "--output",mixed.to_str().unwrap(),"--pair-contrast","--co-batch"],None,false);
    assert!(!mixed.exists());
    // A saved quality stop permits the other preauthorized arm; failed publication does not.
    let quality=d.path().join("quality");
    call(&["fresh","paired-prepare","--parent",p.to_str().unwrap(),"--source-data",selector.join("S-SELECT").to_str().unwrap(),"--output",quality.to_str().unwrap()],None,true);
    call(&["fresh","fixture-full","--root",quality.join("SPACED").to_str().unwrap()],Some("paired-quality"),true);
    call(&["fresh","run","--root",quality.join("ADJACENT").to_str().unwrap()],None,true);
    call(&["fresh","run","--root",quality.join("ADJACENT").to_str().unwrap()],None,true);
    let failure=d.path().join("failure");
    call(&["fresh","paired-prepare","--parent",p.to_str().unwrap(),"--source-data",selector.join("S-SELECT").to_str().unwrap(),"--output",failure.to_str().unwrap()],None,true);
    call(&["fresh","run","--root",failure.join("SPACED").to_str().unwrap()],Some("finished-file-sync"),false);
    let out=call(&["fresh","run","--root",failure.join("ADJACENT").to_str().unwrap()],None,false);
    assert!(!failure.join("ADJACENT/segment-0000-started.r3b").exists());
    assert!(!String::from_utf8_lossy(&out.stdout).contains("TRAIN step="));
    let mut counts=[0u64;3];
    fn count(root:&std::path::Path,totals:&mut[u64;3]) {
        for e in std::fs::read_dir(root).unwrap(){let p=e.unwrap().path();if p.is_dir(){count(&p,totals);}else if p.file_name().unwrap()=="train-control.r3b"{let r:binary::Value=binary::from_slice(&std::fs::read(&p).unwrap()).unwrap();for (i,k) in ["optimizer_calls","generation_calls","teacher_calls"].iter().enumerate(){totals[i]+=r[*k].as_u64().unwrap();}}}
    }
    count(d.path(),&mut counts);
    println!("TINY_TRAIN_CONTROLS_UPDATES_GENERATIONS_TEACHERS={counts:?} observation_generations=27 observation_teachers=27 fresh_process_policy_parity=EXACT pure_report=UNCHANGED");
}

#[test]
fn fresh_eos_deadline_process_and_sync_failure_stay_distinct() {
    use replica_v3::{binary,neural::{EOS,checkpoint}};use candle_core::Device;
    for sync_failure in [false,true]{
        let d=tempfile::tempdir().unwrap();let root=d.path().join("run");
        let call=|args:&[&str],deadline:bool,ok:bool|{let mut c=Command::new(env!("CARGO_BIN_EXE_replica-train"));c.args(args).env("VECLIB_MAXIMUM_THREADS","1").env("RAYON_NUM_THREADS","1").env("R3_FRESH_FIXTURE_EOS","1");
            if deadline{c.env("R3_FRESH_TEST_STOP","final-row-1");if sync_failure{c.env("R3_FRESH_PUBLISH_FAULT","finished-dir-sync");}}
            let o=c.output().unwrap();assert_eq!(o.status.success(),ok,"{}\n{}",String::from_utf8_lossy(&o.stdout),String::from_utf8_lossy(&o.stderr));o};
        call(&["fresh","fixture","--output",root.to_str().unwrap()],false,true);
        call(&["fresh","fixture-full","--root",root.to_str().unwrap()],true,!sync_failure);
        let raw=root.join("eval-0002-train64.r3rows");let before=std::fs::read(&raw).unwrap();
        let rows=binary::read_value_records(&raw).unwrap();assert_eq!(rows.len(),2);assert_eq!(rows[1]["generation_completed"],true);
        assert_eq!(rows[1]["raw_tokens"],binary::record!([EOS]));assert_eq!(rows[1]["command_stop"],"TIME_BUDGET");assert_eq!(rows[1]["exact_match"],false);
        let native=checkpoint::load(&root.join("segment-0000/final"),Device::Cpu,true).unwrap();
        let out=call(&["fresh","run","--root",root.to_str().unwrap()],false,!sync_failure);
        if sync_failure{assert!(!root.join("segment-0001").exists());assert!(!String::from_utf8_lossy(&out.stdout).contains("TRAIN_START"));}
        else{let end=checkpoint::load(&root.join("segment-0001/final"),Device::Cpu,true).unwrap();assert_eq!(native.manifest.training,end.manifest.training);assert_eq!(native.model.weight_hash().unwrap(),end.model.weight_hash().unwrap());assert!(std::fs::read(raw).unwrap().starts_with(&before));
            let a:binary::Value=binary::from_slice(&std::fs::read(root.join("segment-0000/train-control.r3b")).unwrap()).unwrap();let b:binary::Value=binary::from_slice(&std::fs::read(root.join("segment-0001/train-control.r3b")).unwrap()).unwrap();assert_eq!(a["generation_calls"],1);assert_eq!(a["teacher_calls"],0);assert_eq!(b["generation_calls"],23);assert_eq!(b["optimizer_calls"],0);}
    }
    println!("TINY_OPTIMIZER_CALLS=4 actual_EOS_deadline_resume=PASS TIME_PLUS_SYNC_FAILURE=BLOCKED GENERATIONS=25 TEACHERS=24");
}

#[test]
fn fresh_fx06_native_timeout_process_resume_preserves_failed_rows() {
    use candle_core::Device;
    use replica_v3::{binary, neural::checkpoint};
    for (panel, ordinal, tokens) in [("train64", 0, 0), ("train64", 0, 1), ("transfer128", 7, 1)] {
        let d = tempfile::tempdir().unwrap();
        let root = d.path().join("run");
        let call = |action: &str, stop: bool| {
            let mut c = Command::new(env!("CARGO_BIN_EXE_replica-train"));
            c.args([
                "fresh",
                action,
                if action == "fixture" {
                    "--output"
                } else {
                    "--root"
                },
                root.to_str().unwrap(),
            ])
            .env("VECLIB_MAXIMUM_THREADS", "1")
            .env("RAYON_NUM_THREADS", "1")
            .env(
                "R3_FRESH_FIXTURE_EOS",
                if tokens == 0 { "1" } else { "token" },
            );
            if stop {
                c.env(
                    "R3_FRESH_CALL_STOP",
                    format!("eval-0002-{panel}/generation/{ordinal}/native-timeout-{tokens}"),
                );
            }
            let out = c.output().unwrap();
            assert!(
                out.status.success(),
                "{}\n{}",
                String::from_utf8_lossy(&out.stdout),
                String::from_utf8_lossy(&out.stderr)
            );
        };
        call("fixture", false);
        call("fixture-full", true);
        let read = |p: &std::path::Path| -> binary::Value {
            binary::from_slice(&std::fs::read(p).unwrap()).unwrap()
        };
        let control = |segment| read(&root.join(format!("segment-{segment:04}/train-control.r3b")));
        let a = control(0);
        assert_eq!(a["observed_conditions"], binary::record!(["TIME_BUDGET"]));
        assert_eq!(a["final_evaluation_complete"], false);
        let terminal = read(&root.join("segment-0000-finished.r3b"));
        assert_eq!(terminal["phase"], "EvaluationPending");
        assert_eq!(terminal["resume"], true);
        let raw = root.join(format!("eval-0002-{panel}.r3rows"));
        let prefix = std::fs::read(&raw).unwrap();
        let rows = binary::read_value_records(&raw).unwrap();
        assert_eq!(rows.len(), ordinal + 2); // header and completed failed call
        let row = rows.last().unwrap();
        assert_eq!(row["generation_started"], true);
        assert_eq!(row["generation_completed"], false);
        assert_eq!(row["exact_match"], false);
        assert_eq!(row["eos"], false);
        assert_eq!(row["error_class"], "timeout");
        assert_eq!(row["timeout_cap_source"], "command");
        assert!(
            row["effective_timeout_ms"].as_u64().unwrap()
                < row["original_timeout_ms"].as_u64().unwrap()
        );
        let ids = row["raw_tokens"].as_array().unwrap();
        assert_eq!(ids.len(), tokens);
        assert!(
            ids.iter()
                .all(|id| id.as_u64().unwrap() >= replica_v3::neural::SPECIALS as u64)
        );
        let resolved = read(&root.join(format!(
            "eval-0002-{panel}-generation-{ordinal:04}-000-resolved.r3b"
        )));
        assert_eq!(resolved["state"], "RETURNED");
        assert_eq!(resolved["calls"], 1);
        let before = checkpoint::load(&root.join("segment-0000/final"), Device::Cpu, true).unwrap();
        call("run", false); // separate process; no optimizer is permitted here
        let after = checkpoint::load(&root.join("segment-0001/final"), Device::Cpu, true).unwrap();
        assert_eq!(
            before.model.weight_hash().unwrap(),
            after.model.weight_hash().unwrap()
        );
        assert_eq!(before.manifest.training, after.manifest.training);
        for (k, t) in &before.optimizer {
            assert_eq!(
                t.flatten_all().unwrap().to_vec1::<f32>().unwrap(),
                after.optimizer[k]
                    .flatten_all()
                    .unwrap()
                    .to_vec1::<f32>()
                    .unwrap()
            );
        }
        assert!(std::fs::read(raw).unwrap().starts_with(&prefix));
        let b = control(1);
        assert_eq!(b["optimizer_calls"], 0);
        assert_eq!(b["final_evaluation_complete"], true);
        assert_eq!(
            a["generation_calls"].as_u64().unwrap() + b["generation_calls"].as_u64().unwrap(),
            24
        );
        assert_eq!(
            a["teacher_calls"].as_u64().unwrap() + b["teacher_calls"].as_u64().unwrap(),
            24
        );
        if panel == "transfer128" {
            assert_eq!(b["generation_calls"], 0);
            assert_eq!(b["teacher_calls"], 1);
        }
        let summary = read(&root.join(format!("eval-0002-{panel}.r3b")));
        assert_eq!(summary["total"], 8);
        assert!(summary["exact"].as_u64().unwrap() < 8);
        assert!(summary["errors"].as_u64().unwrap() >= 1);
        assert!(
            !root
                .join(format!(
                    "eval-0002-{panel}-generation-{ordinal:04}-001-prepared.r3b"
                ))
                .exists()
        );
        println!(
            "NATIVE_TIMEOUT panel={panel} ordinal={ordinal} prefix_tokens={tokens} generation_entries=24 teacher_entries=24 optimizer=2 resume_optimizer=0 duplicates=0"
        );
    }
}

#[test]
fn fresh_fx06_timeout_study_usage_and_mixed_failure_process() {
    use replica_v3::binary;
    let d = tempfile::tempdir().unwrap();
    let parent = d.path().join("parent");
    let study = d.path().join("study");
    let call = |args: &[&str], fault: Option<bool>, success: bool| {
        let mut c = Command::new(env!("CARGO_BIN_EXE_replica-train"));
        c.args(args)
            .env("VECLIB_MAXIMUM_THREADS", "1")
            .env("RAYON_NUM_THREADS", "1")
            .env("R3_FRESH_FIXTURE_EOS", "1");
        if let Some(mixed) = fault {
            c.env(
                "R3_FRESH_CALL_STOP",
                "eval-0004-train64/generation/0/native-timeout-0",
            );
            if mixed {
                c.env("R3_FRESH_RESOLUTION_FAIL", "1");
            }
        }
        let o = c.output().unwrap();
        assert_eq!(
            o.status.success(),
            success,
            "{}\n{}",
            String::from_utf8_lossy(&o.stdout),
            String::from_utf8_lossy(&o.stderr)
        );
        String::from_utf8(o.stdout).unwrap()
    };
    call(
        &["fresh", "fixture", "--output", parent.to_str().unwrap()],
        None,
        true,
    );
    call(
        &["fresh", "fixture-full", "--root", parent.to_str().unwrap()],
        None,
        true,
    );
    call(
        &[
            "fresh",
            "study-prepare",
            "--parent",
            parent.to_str().unwrap(),
            "--output",
            study.to_str().unwrap(),
        ],
        None,
        true,
    );
    call(
        &["fresh", "study-observe", "--root", study.to_str().unwrap()],
        None,
        true,
    );
    let arm = study.join("C-REPEAT");
    call(
        &["fresh", "fixture-full", "--root", arm.to_str().unwrap()],
        Some(false),
        true,
    );
    call(
        &["fresh", "run", "--root", arm.to_str().unwrap()],
        None,
        true,
    );
    let other = study.join("P-PHRASE");
    // A report requires each arm to have started; one saved update has no due panel.
    call(
        &["fresh", "run", "--root", other.to_str().unwrap()],
        None,
        true,
    );
    // Usage validation ran on the resumed arm; close still refuses an incomplete peer.
    call(
        &["fresh", "study-report", "--root", study.to_str().unwrap()],
        None,
        false,
    );
    let read = |p: &std::path::Path| -> binary::Value {
        binary::from_slice(&std::fs::read(p).unwrap()).unwrap()
    };
    let a = read(&arm.join("segment-0000-finished.r3b"));
    let b = read(&arm.join("segment-0001-finished.r3b"));
    assert_eq!(a["phase"], "EvaluationPending");
    assert_eq!(
        a["generations"].as_u64().unwrap() + b["generations"].as_u64().unwrap(),
        24
    );
    assert_eq!(
        a["teachers"].as_u64().unwrap() + b["teachers"].as_u64().unwrap(),
        24
    );
    assert_eq!(
        read(&arm.join("segment-0001/train-control.r3b"))["optimizer_calls"],
        0
    );
    // A real returned timeout plus failed resolution must block the other arm's history.
    call(
        &["fresh", "fixture-full", "--root", other.to_str().unwrap()],
        Some(true),
        false,
    );
    let failed = read(&other.join("segment-0001/train-control.r3b"));
    assert!(
        failed["observed_conditions"]
            .as_array()
            .unwrap()
            .contains(&binary::record!("TIME_BUDGET"))
    );
    assert!(
        failed["observed_conditions"]
            .as_array()
            .unwrap()
            .contains(&binary::record!("INTEGRITY_FAIL"))
    );
    call(
        &["fresh", "run", "--root", other.to_str().unwrap()],
        None,
        false,
    );
    call(
        &["fresh", "study-report", "--root", study.to_str().unwrap()],
        None,
        false,
    );
    assert!(!other.join("segment-0002").exists());
    println!(
        "TINY_OPTIMIZER=6 GENERATION_ENTRIES=65 RETURNED=65 TEACHERS=64 timeout_study_accounting=VERIFIED mixed_resolution_failure=BLOCKED"
    );
}
#[test]
fn fresh_fx05_not_invoked_and_unknown_process_boundaries() {
    use candle_core::Device;
    use replica_v3::{binary, neural::checkpoint};
    let cases = [
        ("generation", 0, "case_started"),
        ("generation", 3, "prompt_prepared"),
        ("generation", 7, "case_started"),
        ("teacher", 0, "teacher_started"),
        ("teacher", 3, "teacher_forward"),
        ("teacher", 7, "teacher_forward"),
    ];
    let kill_only=std::env::var("R3_FRESH_TEST_KILL_ONLY").as_deref()==Ok("1");
    // The bounded stabilization run covers all boundary kinds without repeating
    // the first/middle/last positions of every pre-call hook.
    let compact = std::env::var("R3_FRESH_TEST_COMPACT").as_deref() == Ok("1");
    for (kind, ordinal, boundary) in cases.into_iter().filter(|&(kind, ordinal, boundary)| {
        !kill_only && (!compact || (kind == "generation" && ordinal == 0)
            || boundary == "prompt_prepared" || (kind == "teacher" && ordinal == 7))
    }) {
        let d = tempfile::tempdir().unwrap();
        let root = d.path().join("run");
        let call = |action: &str, stop: bool| {
            let mut c = Command::new(env!("CARGO_BIN_EXE_replica-train"));
            c.args([
                "fresh",
                action,
                if action == "fixture" {
                    "--output"
                } else {
                    "--root"
                },
                root.to_str().unwrap(),
            ])
            .env("VECLIB_MAXIMUM_THREADS", "1")
            .env("RAYON_NUM_THREADS", "1")
            .env("R3_FRESH_FIXTURE_EOS", "1");
            if stop {
                c.env(
                    "R3_FRESH_CALL_STOP",
                    format!("eval-0002-train64/{kind}/{ordinal}/{boundary}"),
                );
            }
            let o = c.output().unwrap();
            assert!(
                o.status.success(),
                "{}\n{}",
                String::from_utf8_lossy(&o.stdout),
                String::from_utf8_lossy(&o.stderr)
            );
        };
        call("fixture", false);
        call("fixture-full", true);
        let receipt = |n| -> binary::Value {
            binary::from_slice(
                &std::fs::read(root.join(format!("segment-{n:04}/train-control.r3b"))).unwrap(),
            )
            .unwrap()
        };
        let a = receipt(0);
        assert_eq!(a["final_evaluation_complete"], false);
        assert_eq!(
            a["generation_calls"],
            ordinal + usize::from(kind == "teacher")
        );
        assert_eq!(a["teacher_calls"], ordinal);
        let raw = root.join("eval-0002-train64.r3rows");
        let prefix = std::fs::read(&raw).unwrap();
        let resolution = root.join(format!(
            "eval-0002-train64-{kind}-{ordinal:04}-000-resolved.r3b"
        ));
        let r: binary::Value = binary::from_slice(&std::fs::read(&resolution).unwrap()).unwrap();
        assert_eq!(r["state"], "NOT_INVOKED");
        assert_eq!(r["resumable"], true);
        assert_eq!(r["calls"], 0);
        let before = checkpoint::load(&root.join("segment-0000/final"), Device::Cpu, true).unwrap();
        call("run", false);
        let after = checkpoint::load(&root.join("segment-0001/final"), Device::Cpu, true).unwrap();
        assert_eq!(
            before.model.weight_hash().unwrap(),
            after.model.weight_hash().unwrap()
        );
        assert_eq!(before.manifest.training, after.manifest.training);
        for (k, t) in &before.optimizer {
            assert_eq!(
                t.flatten_all().unwrap().to_vec1::<f32>().unwrap(),
                after.optimizer[k]
                    .flatten_all()
                    .unwrap()
                    .to_vec1::<f32>()
                    .unwrap()
            );
        }
        let b = receipt(1);
        assert_eq!(b["optimizer_calls"], 0);
        assert_eq!(b["final_evaluation_complete"], true);
        assert_eq!(
            a["generation_calls"].as_u64().unwrap() + b["generation_calls"].as_u64().unwrap(),
            24
        );
        assert_eq!(
            a["teacher_calls"].as_u64().unwrap() + b["teacher_calls"].as_u64().unwrap(),
            24
        );
        assert!(std::fs::read(raw).unwrap().starts_with(&prefix));
    }
    for fault in [
        "kill-generation",
        "kill-teacher",
        "resolution-sync",
        "cancel-time",
        "identity",
        "required-teacher",
    ].into_iter().filter(|fault|(!kill_only || fault.starts_with("kill-")) && (!compact || *fault != "required-teacher")) {
        let d = tempfile::tempdir().unwrap();
        let root = d.path().join("run");
        let call = |action: &str, inject: bool| {
            let mut c = Command::new(env!("CARGO_BIN_EXE_replica-train"));
            c.args([
                "fresh",
                action,
                if action == "fixture" {
                    "--output"
                } else {
                    "--root"
                },
                root.to_str().unwrap(),
            ])
            .env("VECLIB_MAXIMUM_THREADS", "1")
            .env("RAYON_NUM_THREADS", "1")
            .env("R3_FRESH_FIXTURE_EOS", "1");
            if inject {
                match fault {
                    "kill-generation" => {
                        c.env("R3_FRESH_KILL_ENTERED", "generation");
                    }
                    "kill-teacher" => {
                        c.env("R3_FRESH_KILL_ENTERED", "teacher");
                    }
                    "required-teacher" => {}
                    _ => {
                        c.env(
                            "R3_FRESH_CALL_STOP",
                            "eval-0002-train64/generation/0/case_started",
                        );
                    }
                }
                if fault == "resolution-sync" {
                    c.env("R3_FRESH_RESOLUTION_FAIL", "1");
                }
                if fault == "cancel-time" {
                    c.env("R3_FRESH_CALL_CANCEL", "1");
                }
            }
            c.output().unwrap()
        };
        assert!(call("fixture", false).status.success());
        let first = call("fixture-full", true);
        if fault.starts_with("kill-") {assert_eq!(first.status.code(),Some(86),"native entry hook must execute");}
        assert_eq!(
            first.status.success(),
            matches!(fault, "identity" | "required-teacher"),
            "fault={fault} {}",
            String::from_utf8_lossy(&first.stderr)
        );
        if fault == "identity" {
            let path = root.join("eval-0002-train64-generation-0000-000-prepared.r3b");
            let mut v: binary::Value = binary::from_slice(&std::fs::read(&path).unwrap()).unwrap();
            v["ordinal"] = binary::record!(1);
            std::fs::write(path, binary::to_storage_vec(&v).unwrap()).unwrap();
        }
        if fault == "required-teacher" {
            std::fs::remove_file(root.join("eval-0002-transfer128-teachers.r3rows")).unwrap();
        }
        let o = call("run", false);
        assert!(!o.status.success(), "{fault}");
        assert!(
            !String::from_utf8_lossy(&o.stdout).contains("TRAIN_START"),
            "{fault}"
        );
    }
    println!("TINY_OPTIMIZER_CALLS={} successful_no_call_resumes={} zero_optimizer_resume=true kill_entry_exit86=VERIFIED",if kill_only{4}else if compact{16}else{24},if kill_only{0}else if compact{3}else{6});
}
