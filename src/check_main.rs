#![forbid(unsafe_code)]
//! Thin offline runner for the existing v3 checks. Receipts are observations, not approval.
use clap::{Parser, Subcommand};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    fs::{self, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::{Duration, Instant},
};
type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;
const FEATURES: &str = "accelerate,test-support";

#[derive(Parser)]
#[command(name = "replica-check")]
struct Cli {
    #[command(subcommand)]
    command: Checks,
    #[arg(long, global = true)]
    output: Option<PathBuf>,
}
#[derive(Subcommand)]
enum Checks {
    /// Offline source checks and direct regressions; no quality evaluation or learning.
    Quick {
        /// Direct source-corpus/objective/resume regressions only; no unrelated storage suites.
        #[arg(long)]
        native_corpus: bool,
        /// Only the changed bridge receipt/resume and direct corpus boundaries; leaves the full native suite intact.
        #[arg(long, conflicts_with = "native_corpus")]
        bridge_receipts: bool,
    },
    /// Explicit normal/transfer generation and fresh-process restart, never a quality waiver.
    Model {
        #[arg(long)]
        checkpoint: PathBuf,
        #[arg(long)]
        corpus: PathBuf,
        #[arg(long)]
        transfer_corpus: PathBuf,
        #[arg(long, value_parser = ["validation"])]
        split: String,
        #[arg(long)]
        limit: usize,
    },
    /// Non-heavy v3 tests and artifact-bound S4/S5/S6 evidence. Missing evidence blocks release.
    Release {
        #[arg(long)]
        checkpoint: PathBuf,
        #[arg(long)]
        s4: PathBuf,
        #[arg(long)]
        s5: PathBuf,
        #[arg(long)]
        s6: PathBuf,
    },
}
fn bytes_hash(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
fn file_hash(path: &Path) -> Result<String> {
    Ok(bytes_hash(&fs::read(path)?))
}
fn write_new(path: &Path, value: &Value) -> Result<()> {
    let mut f = OpenOptions::new().write(true).create_new(true).open(path)?;
    serde_json::to_writer_pretty(&mut f, value)?;
    f.write_all(b"\n")?;
    f.sync_all()?;
    Ok(())
}
fn source_files(directory: &Path, out: &mut Vec<PathBuf>) -> Result<()> {
    for entry in fs::read_dir(directory)? {
        let path = entry?.path();
        if path.is_dir() {
            source_files(&path, out)?;
        } else if path.extension().is_some_and(|e| e == "rs") {
            out.push(path);
        }
    }
    Ok(())
}
fn source_identity() -> Result<(String, Vec<PathBuf>)> {
    let mut files = vec![
        "Cargo.toml".into(),
        "Cargo.lock".into(),
        "rust-toolchain.toml".into(),
    ];
    for dir in ["src", "tests", "examples"] {
        source_files(Path::new(dir), &mut files)?;
    }
    files.sort();
    let mut h = Sha256::new();
    for p in &files {
        h.update(p.to_string_lossy().as_bytes());
        h.update([0]);
        h.update(fs::read(p)?);
        h.update([0]);
    }
    Ok((format!("{:x}", h.finalize()), files))
}
// Cargo's actual executed test counts, not the requested filter or exit code alone.
fn test_counts(text: &str) -> Result<[u64; 3]> {
    let mut counts = [0; 3];
    for line in text.lines().filter(|l| l.starts_with("test result:")) {
        for (index, label) in ["passed", "failed", "ignored"].iter().enumerate() {
            let words: Vec<_> = line.split_whitespace().collect();
            if let Some(i) = words.iter().position(|w| w.trim_end_matches(';') == *label) {
                counts[index] += words
                    .get(i.wrapping_sub(1))
                    .ok_or("test count missing")?
                    .parse::<u64>()?;
            }
        }
    }
    if counts[0] + counts[1] == 0 {
        return Err("ZERO_TESTS_EXECUTED".into());
    }
    Ok(counts)
}
struct Runner {
    output: PathBuf,
    cancel: Arc<AtomicBool>,
    records: Vec<Value>,
}
impl Runner {
    fn run(&mut self, program: &str, args: &[&str], tests: bool, timeout: Duration) -> Result<()> {
        let n = self.records.len();
        let stdout = self.output.join(format!("command-{n:02}.stdout"));
        let stderr = self.output.join(format!("command-{n:02}.stderr"));
        let start = Instant::now();
        let mut command = Command::new(program);
        command
            .args(args)
            .env("CARGO_NET_OFFLINE", "true")
            .env("VECLIB_MAXIMUM_THREADS", "1")
            .env("RAYON_NUM_THREADS", "1")
            .stdout(Stdio::from(
                OpenOptions::new()
                    .create_new(true)
                    .write(true)
                    .open(&stdout)?,
            ))
            .stderr(Stdio::from(
                OpenOptions::new()
                    .create_new(true)
                    .write(true)
                    .open(&stderr)?,
            ));
        #[cfg(unix)]
        {
            use std::os::unix::process::CommandExt;
            command.process_group(0);
        }
        let mut child = command.spawn()?;
        let mut stopped = None;
        let status = loop {
            if let Some(status) = child.try_wait()? {
                break status;
            }
            if self.cancel.load(Ordering::Relaxed) || start.elapsed() >= timeout {
                stopped = Some(if self.cancel.load(Ordering::Relaxed) {
                    "CANCELLED"
                } else {
                    "TIME_BUDGET"
                });
                #[cfg(unix)]
                {
                    let _ = Command::new("/bin/kill")
                        .args(["-TERM", "--", &format!("-{}", child.id())])
                        .status();
                }
                let cleanup = Instant::now();
                while cleanup.elapsed() < Duration::from_secs(5) && child.try_wait()?.is_none() {
                    std::thread::sleep(Duration::from_millis(25));
                }
                #[cfg(unix)]
                {
                    let _ = Command::new("/bin/kill")
                        .args(["-KILL", "--", &format!("-{}", child.id())])
                        .status();
                }
                let _ = child.kill();
                break child.wait()?;
            }
            std::thread::sleep(Duration::from_millis(25));
        };
        let stdout_text = fs::read_to_string(&stdout)?;
        let stderr_text = fs::read_to_string(&stderr)?;
        let counts = if tests {
            Some(test_counts(&stdout_text))
        } else {
            None
        };
        let passed = status.success()
            && stopped.is_none()
            && counts
                .as_ref()
                .is_none_or(|c| c.as_ref().is_ok_and(|n| n[1] == 0));
        let blocked_cache = stderr_text.contains("offline")
            && (stderr_text.contains("failed to download")
                || stderr_text.contains("no matching package"));
        let row = json!({"command":program,"args":args,"target":std::env::consts::ARCH,"features":FEATURES,
            "status":if blocked_cache {"BLOCKED_DEPENDENCY_CACHE"} else if passed {"PASS"} else {"FAIL"},
            "exit_code":status.code(),"stop_reason":stopped,"elapsed_seconds":start.elapsed().as_secs_f64(),
            "executed_tests":counts.as_ref().and_then(|c| c.as_ref().ok()).map(|n|n[0]+n[1]),
            "failed_tests":counts.as_ref().and_then(|c| c.as_ref().ok()).map(|n|n[1]),
            "skipped_tests":counts.as_ref().and_then(|c| c.as_ref().ok()).map(|n|n[2]),
            "count_error":counts.as_ref().and_then(|c|c.as_ref().err()).map(ToString::to_string),
            "stdout":stdout,"stderr":stderr});
        write_new(&self.output.join(format!("command-{n:02}.json")), &row)?;
        println!(
            "command={n} status={} executed_tests={}",
            row["status"], row["executed_tests"]
        );
        self.records.push(row);
        if passed {
            Ok(())
        } else {
            Err(if blocked_cache {
                "BLOCKED_DEPENDENCY_CACHE"
            } else {
                "COMMAND_FAILED"
            }
            .into())
        }
    }
    fn cargo(&mut self, action: &str, tail: &[&str], tests: bool) -> Result<()> {
        let mut args = vec![action, "--locked", "--offline", "--features", FEATURES];
        args.extend_from_slice(tail);
        self.run("cargo", &args, tests, Duration::from_secs(900))
    }
}
fn boundaries(files: &[PathBuf]) -> Result<Value> {
    for root in ["src/lib.rs", "src/main.rs", "src/model.rs"] {
        let code = fs::read_to_string(root)?;
        for forbidden in [
            "mod data;",
            "mod training;",
            "mod quality_recovery;",
            "crate::data::",
            "crate::training::",
        ] {
            if code.contains(forbidden) {
                return Err(format!("product boundary: {root}: {forbidden}").into());
            }
        }
    }
    let mut inspected = files.to_vec();
    for entry in fs::read_dir("docs")? {
        let p = entry?.path();
        if p.extension().is_some_and(|e| e == "md") {
            inspected.push(p);
        }
    }
    inspected.push("AGENTS.md".into());
    for p in &inspected {
        if p == Path::new("src/check_main.rs") {
            continue;
        } // Only the checker may name denied instruction files.
        let text = fs::read_to_string(p)?;
        if ["imsi1.md", "imsi2.md", "01_IMPLEMENTATION_PROMPT.md"]
            .iter()
            .any(|name| text.contains(name))
        {
            return Err(format!("temporary instruction reference: {}", p.display()).into());
        }
    }
    let lock = fs::read_to_string("Cargo.lock")?;
    for name in ["hf-hub", "reqwest", "openai", "ollama-rs"] {
        if lock.contains(&format!("name = \"{name}\"")) {
            return Err(format!("external loader/API dependency needs review: {name}").into());
        }
    }
    Ok(
        json!({"inspected_files":inspected.len(),"result":"CHECKED_BOUNDARIES_PASS","limitations":"literal module/dependency checks are heuristics, not a whole-program proof; locked Candle/tokenizers and native SQLite/Accelerate are allowed"}),
    )
}
fn evaluation_rows(path: &Path) -> Result<Vec<Value>> {
    let all: Vec<Value> = fs::read_to_string(path)?
        .lines()
        .map(serde_json::from_str)
        .collect::<std::result::Result<_, _>>()?;
    let terminal = all.last().ok_or("empty evaluation")?;
    if terminal["terminal"] != true
        || terminal["final_evaluation_complete"] != true
        || terminal["comparison_eligible"] != true
    {
        return Err("incomplete evaluation".into());
    }
    let header = all.first().ok_or("missing header")?;
    if [
        "oracle_question_ablation",
        "oracle_field_task_label",
        "oracle_record_selection",
    ]
    .iter()
    .any(|k| header[*k] != false)
    {
        return Err("oracle evaluation is not normal QA".into());
    }
    let rows: Vec<_> = all.into_iter().filter(|r| r.get("id").is_some()).collect();
    if rows.is_empty() {
        return Err("empty evaluation rows".into());
    }
    for row in &rows {
        let raw = row["raw_tokens"]
            .as_array()
            .ok_or("missing raw generation tokens")?;
        let eos = row["eos_index"]
            .as_u64()
            .and_then(|n| usize::try_from(n).ok());
        if !row["error"].is_null()
            || row["generation_completed"] != true
            || row["actual"].as_str().is_none_or(|s| s.trim().is_empty())
            || row["finish_reason"] != "stop"
            || eos.is_none_or(|i| {
                i + 1 != raw.len() || raw.get(i) != Some(&json!(replica_v3::neural::EOS))
            })
        {
            return Err(
                "generation failed strict UTF-8/control/nonempty/EOS checks; raw rows retained"
                    .into(),
            );
        }
        if row["exact_match"] != (row["actual"] == row["expected"]) {
            return Err("exact-match receipt mismatch".into());
        }
    }
    Ok(rows)
}
// Receipts must contain independently inspectable per-case observations, not a PASS flag.
fn release_evidence(checkpoint: &Path, stages: [(&str, &Path); 3]) -> Result<Value> {
    let weight = file_hash(checkpoint).map_err(|e| format!("BLOCKED_INPUT checkpoint: {e}"))?;
    let mut reports = Vec::new();
    for (stage, path) in stages {
        let receipt: Value = serde_json::from_slice(
            &fs::read(path).map_err(|e| format!("BLOCKED_INPUT {stage}: {e}"))?,
        )?;
        if receipt["checkpoint_file_sha256"] != weight
            || receipt["comparison_eligible"] != true
            || receipt["final_evaluation_complete"] != true
            || !receipt["error"].is_null()
            || receipt.get("error").is_none()
            || receipt["terminal_reason"] != "COMPLETED"
        {
            return Err(format!("UNVERIFIED {stage} eligibility/provenance").into());
        }
        let rows_path = PathBuf::from(receipt["rows_path"].as_str().ok_or("missing rows path")?);
        if receipt["rows_sha256"] != file_hash(&rows_path)? {
            return Err("release rows hash mismatch".into());
        }
        let rows = evaluation_rows(&rows_path)?;
        if stage == "S4" {
            for category in 0..5 {
                let group: Vec<_> = rows.iter().filter(|r| r["category"] == category).collect();
                if group.len() < 40
                    || group
                        .iter()
                        .filter(|r| {
                            r["exact_match"] == true
                                && r["finish_reason"] == "stop"
                                && r["eos_index"].is_number()
                        })
                        .count()
                        * 10
                        < group.len() * 9
                {
                    return Err("S4 category gate".into());
                }
            }
            if rows
                .iter()
                .filter(|r| {
                    r["exact_match"] == true
                        && r["finish_reason"] == "stop"
                        && r["eos_index"].is_number()
                })
                .count()
                * 100
                < rows.len() * 95
            {
                return Err("S4 overall gate".into());
            }
        } else {
            // No pre-S4 implementation of future acceptance contracts is presumed.
            return Err(format!("UNVERIFIED {stage}: stage-specific evidence verifier not implemented before prerequisite PASS").into());
        }
        if rows
            .iter()
            .any(|r| !r["error"].is_null() || r["actual"].as_str().is_none_or(str::is_empty))
        {
            return Err("release generation failure".into());
        }
        reports.push(json!({"stage":stage,"receipt_hash":file_hash(path)?,"rows":rows.len()}));
    }
    Ok(json!(reports))
}
fn execute(cli: &Cli, r: &mut Runner, files: &[PathBuf]) -> Result<()> {
    write_new(&r.output.join("boundaries.json"), &boundaries(files)?)?;
    match &cli.command {
        Checks::Quick {
            native_corpus,
            bridge_receipts,
        } => {
            r.run(
                "cargo",
                &["fmt", "--all", "--", "--check"],
                false,
                Duration::from_secs(120),
            )?;
            r.cargo("check", &["--all-targets"], false)?;
            r.cargo("clippy", &["--all-targets", "--", "-D", "warnings"], false)?;
            if *bridge_receipts {
                // Each CLI fixture installs its process cancellation handler once.
                r.cargo(
                    "test",
                    &[
                        "--bin",
                        "replica-train",
                        "artifact_input_conflict_",
                        "--",
                        "--test-threads=1",
                    ],
                    true,
                )?;
                r.cargo(
                    "test",
                    &[
                        "--bin",
                        "replica-train",
                        "segment_capacity_",
                        "--",
                        "--test-threads=1",
                        "--nocapture",
                    ],
                    true,
                )?;
                r.cargo(
                    "test",
                    &[
                        "--bin",
                        "replica-train",
                        "bridge_",
                        "--",
                        "--test-threads=1",
                    ],
                    true,
                )?;
                for filter in [
                    "bridge_",
                    "conditional_",
                    "native_resume_ignores_ambient_policy_json",
                    "verification_published_final_sync_error",
                    "binary_preflight_attempt_failures_usage_and_single_writer",
                    "restart_and_cooldown_partial_panels_resume_to_close",
                    "objective_policy_native_fresh_resume_and_zero_span_parity",
                ] {
                    r.cargo(
                        "test",
                        &[
                            "--test",
                            "experiment_record",
                            filter,
                            "--",
                            "--test-threads=1",
                            "--nocapture",
                        ],
                        true,
                    )?;
                }
                for filter in [
                    "full_population_binding_pairs_require_question_and_value_without_split_growth",
                    "qa_memorization_subset_preserves_episodes_and_split_boundaries",
                ] {
                    r.cargo(
                        "test",
                        &[
                            "--test",
                            "training",
                            filter,
                            "--",
                            "--exact",
                            "--test-threads=1",
                        ],
                        true,
                    )?;
                }
                return Ok(());
            }
            if *native_corpus {
                r.cargo(
                    "test",
                    &["--lib", "neural::artifact::tests", "--", "--test-threads=1"],
                    true,
                )?;
                for filter in [
                    "native_corpus_",
                    "conditional_panel_",
                    "bridge_pairs_serialized_temporal_oracle_and_negative_facts",
                    "token_cache_",
                    "partial_prefix_rejects",
                ] {
                    r.cargo(
                        "test",
                        &["--bin", "replica-train", filter, "--", "--test-threads=1"],
                        true,
                    )?;
                }
                for filter in [
                    "native_corpus_standalone_default",
                    "native_resume_ignores_ambient_policy_json",
                    "conditional_plan_sticky_failure",
                    "bridge_",
                    "objective_policy_",
                    "verification_published_final_sync_error",
                    "restart_and_cooldown_partial_panels",
                ] {
                    r.cargo(
                        "test",
                        &[
                            "--test",
                            "experiment_record",
                            filter,
                            "--",
                            "--test-threads=1",
                            "--nocapture",
                        ],
                        true,
                    )?;
                }
                for filter in [
                    "full_population_binding_pairs_require_question_and_value_without_split_growth",
                    "qa_memorization_subset_preserves_episodes_and_split_boundaries",
                    "full_qa_pairs_keep_split_and_bind_question_value_citation_in_both_orders",
                    "harness_m01_m02_malformed_cli_rejects_before_output_or_model_load",
                    "query_pairs_require_question_and_evidence_with_validation_unchanged",
                    "field_pairs_change_only_training_questions_and_selected_value",
                    "field_cue_targets_are_supported_and_ordinary_qa_is_unchanged",
                    "entity_cue_auxiliary_pairs_require_evidence_and_preserve_ordinary_qa",
                    "record_copy_targets_keep_source_bytes_and_temporal_qa",
                    "evidence_first_training_preserves_questions_records_and_supported_answers",
                    "counterfactual_corpus_requires_evidence_for_identical_questions",
                    "curriculum_corpus_keeps_copy_training_explicit_and_time_independent",
                    "balanced_corpus_varies_distractor_identity_context_and_version_order",
                    "grounding_corpus_teaches_binding_and_supported_sequence_without_runtime_renderer",
                    "corpus_and_tokenizer_use_train_only_and_reject_split_leakage",
                ] {
                    r.cargo(
                        "test",
                        &[
                            "--test",
                            "training",
                            filter,
                            "--",
                            "--exact",
                            "--test-threads=1",
                        ],
                        true,
                    )?;
                }
                return Ok(());
            }
            r.cargo(
                "test",
                &["--lib", "neural::artifact::tests", "--", "--test-threads=1"],
                true,
            )?;
            r.cargo(
                "test",
                &["--lib", "journal::tests", "--", "--test-threads=1"],
                true,
            )?;
            r.cargo(
                "test",
                &["--example", "validate", "cold_probe_exact_and_corrupt"],
                true,
            )?;
            for (target, filter) in [
                ("replica-check", "harness_"),
                ("replica-train", "harness_m"),
                ("replica-train", "repair_rf"),
                ("replica-train", "skill_"),
                ("replica-train", "progress_"),
                ("replica-train", "state_data_"),
                ("replica-train", "binary_tests"),
            ] {
                r.cargo(
                    "test",
                    &["--bin", target, filter, "--", "--test-threads=1"],
                    true,
                )?;
            }
            for (target, filter) in [
                ("training", "harness_m"),
                ("native", "native_numeric_references"),
                ("native", "progress_attention_observer"),
                ("native", "legacy_checkpoint_import_roundtrip"),
                ("runtime", "rv02_whole_evidence"),
                ("store", "lifecycle_restart"),
                ("experiment_record", "binary_"),
                ("journal", "journal_"),
                ("store", "retraction_restore_and_db_free_archive"),
            ] {
                r.cargo(
                    "test",
                    &["--test", target, filter, "--", "--test-threads=1"],
                    true,
                )?;
            }
        }
        Checks::Model {
            checkpoint,
            corpus,
            transfer_corpus,
            split,
            limit,
        } => {
            if *limit == 0
                || !checkpoint.is_file()
                || !corpus.join("manifest.json").is_file()
                || !transfer_corpus.join("manifest.json").is_file()
            {
                return Err("BLOCKED_INPUT explicit checkpoint/corpus/transfer split".into());
            }
            r.cargo("build", &["--release", "--bin", "replica-train"], false)?;
            let mut previous = None;
            for (name, dataset) in [
                ("normal", corpus),
                ("restart", corpus),
                ("transfer", transfer_corpus),
            ] {
                let out = r.output.join(format!("{name}.jsonl"));
                let args = vec![
                    "evaluate".into(),
                    "--checkpoint".into(),
                    checkpoint.display().to_string(),
                    "--corpus".into(),
                    dataset.display().to_string(),
                    "--split".into(),
                    split.clone(),
                    "--limit".into(),
                    limit.to_string(),
                    "--output".into(),
                    out.display().to_string(),
                ];
                r.run(
                    "target/release/replica-train",
                    &args.iter().map(String::as_str).collect::<Vec<_>>(),
                    false,
                    Duration::from_secs(900),
                )?;
                let rows = evaluation_rows(&out)?;
                if rows.len() != *limit {
                    return Err("generation denominator mismatch".into());
                }
                let signatures: Vec<_> = rows
                    .iter()
                    .map(|row| {
                        json!([
                            row["id"],
                            row["raw_tokens"],
                            row["actual"],
                            row["error"],
                            row["eos_index"],
                            row["finish_reason"]
                        ])
                    })
                    .collect();
                if name == "restart" && previous.as_ref() != Some(&signatures) {
                    return Err("fresh restart mismatch".into());
                }
                previous = Some(signatures);
            }
        }
        Checks::Release {
            checkpoint,
            s4,
            s5,
            s6,
        } => {
            r.cargo("test", &["--all-targets", "--", "--test-threads=1"], true)?;
            let evidence = release_evidence(checkpoint, [("S4", s4), ("S5", s5), ("S6", s6)])?;
            write_new(&r.output.join("release-evidence.json"), &evidence)?;
        }
    }
    Ok(())
}
fn main() {
    let cli = Cli::parse();
    let result = (|| -> Result<()> {
        let (source, files) = source_identity()?;
        let source_files: std::collections::BTreeMap<_, _> = files
            .iter()
            .map(|p| Ok((p.display().to_string(), file_hash(p)?)))
            .collect::<Result<_>>()?;
        let output = cli
            .output
            .as_ref()
            .ok_or("--output NEW_DIRECTORY is required")?;
        fs::create_dir(output)?;
        let cancel = Arc::new(AtomicBool::new(false));
        let flag = cancel.clone();
        ctrlc::set_handler(move || flag.store(true, Ordering::Relaxed))?;
        let mut runner = Runner {
            output: output.clone(),
            cancel,
            records: Vec::new(),
        };
        let outcome = execute(&cli, &mut runner, &files);
        let unchanged = source_identity()?.0 == source;
        let summary = json!({"source_digest":source,"source_files":source_files,"source_unchanged":unchanged,"commands":runner.records,
            "result":if outcome.is_ok() && unchanged {"CHECKED_SCOPE_PASS"} else {"FAIL_OR_BLOCKED"},
            "error":outcome.as_ref().err().map(ToString::to_string),"actual_small_updates":0,
            "quality":"NOT_GRANTED_BY_HARNESS","goal1_accepted":false,"not_run":"dependent commands after first failure; S4/S5/S6 unless explicit release evidence passes"});
        write_new(&output.join("summary.json"), &summary)?;
        outcome?;
        if !unchanged {
            return Err("source changed during checks".into());
        }
        Ok(())
    })();
    if let Err(error) = result {
        eprintln!("{error}");
        std::process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn harness_counts_reject_zero_and_keep_failures() {
        assert!(
            Cli::try_parse_from(["replica-check", "quick", "--output", "new-directory"]).is_ok()
        );
        assert!(
            test_counts("test result: ok. 0 passed; 0 failed; 0 ignored; 17 filtered out").is_err()
        );
        assert_eq!(
            test_counts("test result: FAILED. 2 passed; 1 failed; 3 ignored; 0 measured").unwrap(),
            [2, 1, 3]
        );
        assert_eq!(test_counts("test result: ok. 2 passed; 0 failed; 0 ignored\ntest result: ok. 3 passed; 0 failed; 1 ignored").unwrap(), [5,0,1]);
    }
    #[test]
    #[ignore = "bounded child fixture, called only by runner regression"]
    fn child_fixture() {
        std::thread::sleep(Duration::from_secs(30));
    }
    #[test]
    fn harness_runner_observes_exit_zero_filter_and_deadline() {
        let dir = tempfile::tempdir().unwrap();
        let mut runner = Runner {
            output: dir.path().into(),
            cancel: Arc::new(AtomicBool::new(false)),
            records: Vec::new(),
        };
        let exe = std::env::current_exe().unwrap();
        assert!(
            runner
                .run(
                    exe.to_str().unwrap(),
                    &["--exact", "no_such_test"],
                    true,
                    Duration::from_secs(5)
                )
                .is_err()
        );
        assert_eq!(runner.records[0]["count_error"], "ZERO_TESTS_EXECUTED");
        assert!(
            runner
                .run(
                    exe.to_str().unwrap(),
                    &["--invalid-test-runner-option"],
                    false,
                    Duration::from_secs(5)
                )
                .is_err()
        );
        assert!(
            runner
                .run(
                    exe.to_str().unwrap(),
                    &[
                        "--exact",
                        "tests::harness_counts_reject_zero_and_keep_failures"
                    ],
                    true,
                    Duration::from_secs(5)
                )
                .is_ok()
        );
        runner.cancel.store(true, Ordering::Relaxed);
        assert!(
            runner
                .run(
                    exe.to_str().unwrap(),
                    &["--exact", "tests::child_fixture", "--ignored"],
                    false,
                    Duration::ZERO
                )
                .is_err()
        );
        assert_eq!(runner.records.last().unwrap()["stop_reason"], "CANCELLED");
        runner.cancel.store(false, Ordering::Relaxed);
        assert!(
            runner
                .run(
                    exe.to_str().unwrap(),
                    &["--exact", "tests::child_fixture", "--ignored"],
                    false,
                    Duration::from_millis(50)
                )
                .is_err()
        );
        assert_eq!(runner.records.last().unwrap()["stop_reason"], "TIME_BUDGET");
    }
    #[test]
    fn harness_missing_release_artifact_blocks() {
        let dir = tempfile::tempdir().unwrap();
        assert!(
            release_evidence(
                &dir.path().join("absent"),
                [("S4", dir.path()), ("S5", dir.path()), ("S6", dir.path())]
            )
            .is_err()
        );
    }
    #[test]
    fn harness_model_rows_require_actual_eos_and_keep_failures() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("rows.jsonl");
        let header = json!({"header":true,"oracle_question_ablation":false,"oracle_field_task_label":false,"oracle_record_selection":false});
        let row = json!({"id":"independent/0","actual":"a","expected":"a","exact_match":true,"raw_tokens":[8,2],"eos_index":1,"finish_reason":"stop","generation_completed":true,"error":null});
        let terminal =
            json!({"terminal":true,"final_evaluation_complete":true,"comparison_eligible":true});
        let write =
            |row: &Value| fs::write(&path, format!("{header}\n{row}\n{terminal}\n")).unwrap();
        write(&row);
        assert_eq!(evaluation_rows(&path).unwrap().len(), 1);
        for (key, value) in [
            ("error", json!("invalid UTF-8")),
            ("finish_reason", json!("length")),
            ("eos_index", Value::Null),
            ("actual", json!("")),
            ("raw_tokens", json!([8, 3])),
            ("generation_completed", json!(false)),
        ] {
            let mut bad = row.clone();
            bad[key] = value;
            write(&bad);
            assert!(evaluation_rows(&path).is_err(), "{key}");
        }
    }
}
