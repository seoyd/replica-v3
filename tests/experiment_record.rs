#![cfg(feature = "test-support")]
//! Real native subprocess regression. Test-only tiny fixture; never model quality evidence.
use std::{
    fs,
    path::{Path, PathBuf},
    process::{Command, Output, Stdio},
    time::{Duration, Instant},
};
fn call(args: &[&str], stop: Option<&str>, success: bool, log: &Path) -> Output {
    let mut command = Command::new(env!("CARGO_BIN_EXE_replica-train"));
    command
        .args(["recovery", "native"])
        .args(args)
        .env("VECLIB_MAXIMUM_THREADS", "1")
        .env("RAYON_NUM_THREADS", "1")
        .env_remove("R3ER_TEST_STOP");
    if let Some(stop) = stop {
        command.env("R3ER_TEST_STOP", stop);
    }
    let file = fs::File::create(log).unwrap();
    let mut child = command
        .stdout(Stdio::from(file.try_clone().unwrap()))
        .stderr(Stdio::from(file))
        .spawn()
        .unwrap();
    let deadline = Instant::now() + Duration::from_secs(120);
    let status = loop {
        if let Some(status) = child.try_wait().unwrap() {
            break status;
        }
        if Instant::now() >= deadline {
            let _ = child.kill();
            let _ = child.wait();
            panic!("owned test child exceeded deadline: {args:?}");
        }
        std::thread::sleep(Duration::from_millis(10));
    };
    let out = Output {
        status,
        stdout: fs::read(log).unwrap(),
        stderr: Vec::new(),
    };
    print!("{}", String::from_utf8_lossy(&out.stdout));
    assert_eq!(
        out.status.success(),
        success,
        "args={args:?}\n{}",
        String::from_utf8_lossy(&out.stdout)
    );
    out
}
fn p(p: &Path) -> &str {
    p.to_str().unwrap()
}
fn copy_fixture(from: &Path, to: &Path) {
    fs::create_dir(to).unwrap();
    for entry in fs::read_dir(from).unwrap() {
        let entry = entry.unwrap();
        let path = to.join(entry.file_name());
        if entry.file_type().unwrap().is_dir() {
            copy_fixture(&entry.path(), &path);
        } else {
            fs::copy(entry.path(), path).unwrap();
        }
    }
}
fn evidence_manifest(root: &Path) -> Vec<(PathBuf, bool, u64, Vec<u8>)> {
    use sha2::{Digest, Sha256};
    fn walk(root: &Path, dir: &Path, out: &mut Vec<(PathBuf, bool, u64, Vec<u8>)>) {
        for entry in fs::read_dir(dir).unwrap() {
            let entry = entry.unwrap();
            let m = fs::symlink_metadata(entry.path()).unwrap();
            assert!(!m.is_symlink());
            out.push((
                entry.path().strip_prefix(root).unwrap().to_path_buf(),
                m.is_dir(),
                if m.is_dir() { 0 } else { m.len() },
                if m.is_dir() {
                    vec![]
                } else {
                    Sha256::digest(fs::read(entry.path()).unwrap()).to_vec()
                },
            ));
            if m.is_dir() {
                walk(root, &entry.path(), out);
            }
        }
    }
    let mut out = vec![];
    walk(root, root, &mut out);
    out.sort();
    out
}
#[test]
fn conditional_root_registration_survives_missing_child() {
    let d = tempfile::tempdir().unwrap();
    let bootstrap = PathBuf::from(std::env::var_os("R3ER_TEST_BOOTSTRAP").unwrap());
    let root = d.path().join("plan");
    call(
        &[
            "fixture-conditional",
            "--from",
            p(&bootstrap),
            "--output",
            p(&root),
        ],
        None,
        true,
        &d.path().join("prepare.log"),
    );
    call(
        &["conditional-run", "--root", p(&root), "--model", "0"],
        Some("pv-start-kill"),
        false,
        &d.path().join("start.log"),
    );
    fs::rename(root.join("model-0"), d.path().join("quarantine")).unwrap();
    let preserved = evidence_manifest(&root);
    for model in ["0", "1"] {
        let out = call(
            &["conditional-run", "--root", p(&root), "--model", model],
            None,
            false,
            &d.path().join("retry.log"),
        );
        let log = String::from_utf8_lossy(&out.stdout);
        assert!(!log.contains("VERIFICATION_GENERATION_API_ENTRY="));
        assert!(!log.contains("CONDITIONAL_MODEL_LOAD="));
        assert!(!root.join(format!("model-{model}")).exists());
    }
    assert_eq!(preserved, evidence_manifest(&root));
    let root = d.path().join("sync-failure");
    call(
        &[
            "fixture-conditional",
            "--from",
            p(&bootstrap),
            "--output",
            p(&root),
        ],
        None,
        true,
        &d.path().join("prepare-sync.log"),
    );
    let out = call(
        &["conditional-run", "--root", p(&root), "--model", "0"],
        Some("conditional-registration-sync"),
        false,
        &d.path().join("sync.log"),
    );
    assert!(root.join("attempt-0.r3er").exists());
    assert!(!String::from_utf8_lossy(&out.stdout).contains("CONDITIONAL_MODEL_LOAD="));
    for model in ["0", "1"] {
        call(
            &["conditional-run", "--root", p(&root), "--model", model],
            None,
            false,
            &d.path().join("sync-retry.log"),
        );
        assert!(!root.join(format!("model-{model}")).exists());
    }
    println!("ROOT_REGISTRATION_REGRESSION TINY_UPDATES=0 SMALL_UPDATES=0");
}
#[test]
fn bridge_registered_observation_rejects_prestart_copy_in_fresh_processes() {
    let d = tempfile::tempdir().unwrap();
    let bootstrap = PathBuf::from(std::env::var_os("R3ER_TEST_BOOTSTRAP").unwrap());
    let root = d.path().join("observation");
    call(
        &[
            "fixture-bridge",
            "--from",
            p(&bootstrap),
            "--output",
            p(&root),
            "--observation",
        ],
        None,
        true,
        &d.path().join("prepare.log"),
    );
    assert!(root.with_extension("r3er").is_file());
    let copied = d.path().join("copied-observation");
    copy_fixture(&root, &copied);
    let original = evidence_manifest(&root);
    let before = evidence_manifest(&copied);
    let study = d.path().join("must-not-be-created");
    for args in [
        vec!["bridge-observe", "--root", p(&copied)],
        vec!["bridge-observe-report", "--root", p(&copied)],
        vec![
            "bridge-prepare",
            "--observation",
            p(&copied),
            "--output",
            p(&study),
        ],
    ] {
        let out = call(&args, None, false, &d.path().join("rejected.log"));
        let log = String::from_utf8_lossy(&out.stdout);
        assert!(log.contains("REGISTERED_OBSERVATION_ROOT_MISMATCH"));
        assert!(!log.contains("GENERATION_API_ENTRY="));
        assert!(!log.contains("TEACHER_API_ENTRY="));
        assert_eq!(before, evidence_manifest(&copied));
        assert_eq!(original, evidence_manifest(&root));
        assert!(!study.exists());
    }
    let alias = root.join(".");
    call(
        &["bridge-observe", "--root", p(&alias)],
        None,
        true,
        &d.path().join("observe.log"),
    );
    let complete = evidence_manifest(&root);
    call(
        &["bridge-observe-report", "--root", p(&alias)],
        None,
        true,
        &d.path().join("report.log"),
    );
    assert_eq!(complete, evidence_manifest(&root));
    println!(
        "REGISTERED_ROOT_REAL_LAYOUT TINY_UPDATES=0 GENERATIONS=2 TEACHERS=2 REJECTED_COPY_CALLS=0"
    );
}
#[test]
fn bounded_screen_registered_tiny_first_save_fresh_resume_and_close() {
    let d = tempfile::tempdir().unwrap();
    let bootstrap = PathBuf::from(std::env::var_os("R3ER_TEST_BOOTSTRAP").unwrap());
    let obs = d.path().join("observation");
    let root = d.path().join("T-SCREEN");
    call(
        &[
            "fixture-bridge",
            "--from",
            p(&bootstrap),
            "--output",
            p(&obs),
            "--observation",
        ],
        None,
        true,
        &d.path().join("prepare.log"),
    );
    call(
        &["bridge-observe", "--root", p(&obs)],
        None,
        true,
        &d.path().join("observe.log"),
    );
    call(
        &[
            "fixture-screen",
            "--observation",
            p(&obs),
            "--output",
            p(&root),
        ],
        None,
        true,
        &d.path().join("screen.log"),
    );
    let original = evidence_manifest(&obs);
    let first = call(
        &["run", "--root", p(&root)],
        None,
        true,
        &d.path().join("first.log"),
    );
    assert!(String::from_utf8_lossy(&first.stdout).contains("COMMAND_FINALIZATION=TimePause"));
    let second = call(
        &[
            "run",
            "--root",
            p(&root),
            "--resume",
            "segment-00/terminal.r3er",
        ],
        None,
        true,
        &d.path().join("resume.log"),
    );
    let text = String::from_utf8_lossy(&second.stdout);
    assert!(text.contains("T_SCREEN_RESULT updates=1"));
    assert!(text.contains("COMMAND_FINALIZATION=Complete"));
    let before = evidence_manifest(&root);
    call(
        &[
            "close",
            "--root",
            p(&root),
            "--terminal",
            "segment-01/terminal.r3er",
        ],
        None,
        true,
        &d.path().join("close.log"),
    );
    assert_eq!(original, evidence_manifest(&obs));
    assert_eq!(before, evidence_manifest(&root));
    let copied = d.path().join("copied");
    copy_fixture(&root, &copied);
    let out = call(
        &[
            "run",
            "--root",
            p(&copied),
            "--resume",
            "segment-00/terminal.r3er",
        ],
        None,
        false,
        &d.path().join("moved.log"),
    );
    assert!(!String::from_utf8_lossy(&out.stdout).contains("ACTUAL_TINY_UPDATE="));
    println!(
        "BOUNDED_SCREEN_PROCESS TINY_UPDATES=2 SMALL_UPDATES=0 PRODUCTION_REPLACEMENT=true EXPLICIT_PANEL_SPEC=1+1+1+1"
    );
}
#[test]
fn bridge_observation_teacher_receipt_survives_fresh_read_and_cannot_retry() {
    let d = tempfile::tempdir().unwrap();
    let bootstrap = PathBuf::from(std::env::var_os("R3ER_TEST_BOOTSTRAP").unwrap());
    let root = d.path().join("observation");
    call(
        &[
            "fixture-bridge",
            "--from",
            p(&bootstrap),
            "--output",
            p(&root),
            "--observation",
        ],
        None,
        true,
        &d.path().join("prepare.log"),
    );
    let observed = call(
        &["bridge-observe", "--root", p(&root)],
        None,
        true,
        &d.path().join("observe.log"),
    );
    let log = String::from_utf8_lossy(&observed.stdout);
    assert!(log.contains("TEACHERS=2"));
    let final_bytes = fs::read(root.join("preflight-final.r3er")).unwrap();
    let manifest = evidence_manifest(&root);
    let report = call(
        &["bridge-observe-report", "--root", p(&root)],
        None,
        true,
        &d.path().join("report.log"),
    );
    assert!(
        String::from_utf8_lossy(&report.stdout)
            .contains("NEW_GENERATIONS=0 NEW_TEACHERS=0 NEW_SMALL_UPDATES=0")
    );
    assert_eq!(manifest, evidence_manifest(&root));
    for name in [
        "parent-probe-start.r3er",
        "parent-probe-entry-00.r3er",
        "parent-probe-row-00.r3er",
        "parent-probe-final.r3er",
    ] {
        let bytes = fs::read(root.join(name)).unwrap();
        for missing in [true, false] {
            if missing {
                fs::remove_file(root.join(name)).unwrap();
            } else {
                fs::write(root.join(name), b"corrupt teacher evidence").unwrap();
            }
            let before = evidence_manifest(&root);
            let out = call(
                &["bridge-observe-report", "--root", p(&root)],
                None,
                false,
                &d.path().join("negative-report.log"),
            );
            assert!(
                !String::from_utf8_lossy(&out.stdout)
                    .contains("VERIFICATION_GENERATION_API_ENTRY=")
            );
            let study = d.path().join("must-not-exist");
            call(
                &[
                    "bridge-prepare",
                    "--observation",
                    p(&root),
                    "--output",
                    p(&study),
                ],
                None,
                false,
                &d.path().join("negative-prepare.log"),
            );
            assert!(!study.exists());
            assert_eq!(before, evidence_manifest(&root));
            fs::write(root.join(name), &bytes).unwrap();
        }
    }
    call(
        &["bridge-observe", "--root", p(&root)],
        None,
        false,
        &d.path().join("retry.log"),
    );
    assert_eq!(
        final_bytes,
        fs::read(root.join("preflight-final.r3er")).unwrap()
    );
    let study = d.path().join("study");
    call(
        &[
            "bridge-prepare",
            "--observation",
            p(&root),
            "--output",
            p(&study),
        ],
        None,
        true,
        &d.path().join("admission.log"),
    );
    let mut updates = 0;
    for arm in ["C-COPYMATCH", "T-TEMPORAL"] {
        let dir = study.join(arm);
        for resume in [false, true] {
            let mut args = vec!["run", "--root", p(&dir)];
            if resume {
                args.extend(["--resume", "segment-00/terminal.r3er"]);
            }
            let out = call(&args, None, true, &d.path().join("run.log"));
            updates += String::from_utf8_lossy(&out.stdout)
                .matches("ACTUAL_TINY_UPDATE=")
                .count();
        }
    }
    assert_eq!(updates, 4);
    let before = evidence_manifest(&study);
    call(
        &["anchor-report", "--root", p(&study)],
        None,
        true,
        &d.path().join("pair-report.log"),
    );
    assert_eq!(before, evidence_manifest(&study));
    assert_eq!(manifest, evidence_manifest(&root));
    println!("PRODUCTION_LAYOUT_E2E TINY_UPDATES={updates} SMALL_UPDATES=0");
    println!(
        "BRIDGE_OBSERVATION_ONLY TINY_GENERATIONS=2 TINY_TEACHERS=2 TINY_UPDATES=0 SMALL_UPDATES=0"
    );
}
#[test]
fn bridge_teacher_failure_is_sticky_and_report_does_not_repair() {
    let d = tempfile::tempdir().unwrap();
    let bootstrap = PathBuf::from(std::env::var_os("R3ER_TEST_BOOTSTRAP").unwrap());
    for mode in [
        "bridge-teacher-cancel",
        "conditional-teacher-error",
        "pv-final-write-fail",
    ] {
        let root = d.path().join(mode);
        call(
            &[
                "fixture-bridge",
                "--from",
                p(&bootstrap),
                "--output",
                p(&root),
                "--observation",
            ],
            None,
            true,
            &d.path().join("prepare.log"),
        );
        call(
            &["bridge-observe", "--root", p(&root)],
            Some(mode),
            false,
            &d.path().join("observe.log"),
        );
        assert!(root.join("parent-probe-entry-00.r3er").exists());
        let before = evidence_manifest(&root);
        // A complete raw-only report may describe a failed command, but never grants admission.
        call(
            &["bridge-observe-report", "--root", p(&root)],
            None,
            mode == "pv-final-write-fail",
            &d.path().join("report.log"),
        );
        assert_eq!(before, evidence_manifest(&root));
        let study = d.path().join(format!("study-{mode}"));
        call(
            &[
                "bridge-prepare",
                "--observation",
                p(&root),
                "--output",
                p(&study),
            ],
            None,
            false,
            &d.path().join("admission.log"),
        );
        call(
            &["bridge-observe", "--root", p(&root)],
            None,
            false,
            &d.path().join("retry.log"),
        );
        assert!(!study.exists());
        assert_eq!(before, evidence_manifest(&root));
    }
    println!("BRIDGE_FAILURE_REGRESSION TINY_UPDATES=0 SMALL_UPDATES=0");
}
#[test]
fn bridge_policy_two_updates_equal_one_plus_one_in_fresh_processes() {
    let d = tempfile::tempdir().unwrap();
    let bootstrap = PathBuf::from(std::env::var_os("R3ER_TEST_BOOTSTRAP").unwrap());
    let a = d.path().join("continuous");
    let b = d.path().join("split");
    let mut updates = 0;
    for (root, continuous) in [(&a, true), (&b, false)] {
        let mut args = vec![
            "fixture-bridge",
            "--from",
            p(&bootstrap),
            "--output",
            p(root),
        ];
        if continuous {
            args.push("--continuous");
        }
        call(&args, None, true, &d.path().join("prepare.log"));
        let out = call(
            &["run", "--root", p(root)],
            None,
            true,
            &d.path().join("run.log"),
        );
        updates += String::from_utf8_lossy(&out.stdout)
            .lines()
            .filter(|l| l.starts_with("ACTUAL_TINY_UPDATE="))
            .count();
        if !continuous {
            assert!(String::from_utf8_lossy(&out.stdout).contains("resume=true complete=false"));
            let out = call(
                &[
                    "run",
                    "--root",
                    p(root),
                    "--resume",
                    "segment-00/terminal.r3er",
                ],
                None,
                true,
                &d.path().join("resume.log"),
            );
            updates += String::from_utf8_lossy(&out.stdout)
                .lines()
                .filter(|l| l.starts_with("ACTUAL_TINY_UPDATE="))
                .count();
        }
    }
    assert_eq!(updates, 4);
    call(
        &["fixture-check", "--roots", p(&a), "--roots", p(&b)],
        None,
        true,
        &d.path().join("check.log"),
    );
    println!(
        "BRIDGE_FRESH_PROCESS NUMERIC_WEIGHT_ADAM_LR_PARITY=true TINY_UPDATES={updates} SMALL_UPDATES=0"
    );
}
#[test]
fn verification_published_final_sync_error_cannot_authorize_fresh_process() {
    let d = tempfile::tempdir().unwrap();
    let bootstrap = PathBuf::from(std::env::var_os("R3ER_TEST_BOOTSTRAP").unwrap());
    let root = d.path().join("publication");
    call(
        &[
            "fixture-preflight",
            "--from",
            p(&bootstrap),
            "--output",
            p(&root),
        ],
        None,
        true,
        &d.path().join("prepare.log"),
    );
    let mut updates = 0;
    for (arm, resume) in [
        ("preflight-A", None),
        ("preflight-B", None),
        ("preflight-B", Some("segment-00/terminal.r3er")),
    ] {
        let dir = root.join(arm);
        let mut args = vec!["run", "--root", p(&dir)];
        if let Some(r) = resume {
            args.extend(["--resume", r]);
        }
        let out = call(&args, None, true, &d.path().join("train.log"));
        updates += String::from_utf8_lossy(&out.stdout)
            .lines()
            .filter(|l| l.starts_with("ACTUAL_TINY_UPDATE="))
            .count();
    }
    println!("PF_PUBLISH_ACTUAL_TINY_UPDATES={updates} SMALL_UPDATES=0");
    let prepared = root;
    for (mode, success, final_exists) in [
        ("pv-final-sync-fail", false, true),
        ("pv-final-sync-and-record-fail", false, true),
        ("pv-pending-kill", false, false),
        ("pv-pending-release-write-fail", false, true),
        ("pv-cleanup-write-fail", true, true),
    ] {
        let root = d.path().join(mode);
        copy_fixture(&prepared, &root);
        let out = call(
            &["save-preflight-verify", "--root", p(&root)],
            Some(mode),
            success,
            &d.path().join("fault.log"),
        );
        if mode.starts_with("pv-final-sync") {
            assert!(String::from_utf8_lossy(&out.stdout).contains("AFTER final hard_link"));
        }
        let final_bytes = fs::read(root.join("preflight-final.r3er")).ok();
        assert_eq!(final_bytes.is_some(), final_exists);
        if success {
            assert!(
                String::from_utf8_lossy(&out.stdout)
                    .contains("PUBLICATION_COMMITTED=true CLEANUP_WARNING=")
            );
            let out = call(
                &["save-preflight-verify", "--root", p(&root)],
                None,
                true,
                &d.path().join("positive.log"),
            );
            assert!(
                !String::from_utf8_lossy(&out.stdout)
                    .contains("VERIFICATION_GENERATION_API_ENTRY=")
            );
            continue;
        }
        for args in [
            vec!["save-preflight-verify", "--root", p(&root)],
            vec!["run", "--root", p(&root.join("C50"))],
            vec!["anchor-report", "--root", p(&root)],
        ] {
            let out = call(&args, None, false, &d.path().join("consumer.log"));
            let text = String::from_utf8_lossy(&out.stdout);
            assert!(!text.contains("VERIFICATION_GENERATION_API_ENTRY="));
            assert!(!text.contains("ACTUAL_TINY_UPDATE="));
        }
        assert_eq!(
            final_bytes,
            fs::read(root.join("preflight-final.r3er")).ok()
        );
        assert!(root.join("preflight-publication-pending.r3er").is_file());
        let pending = root.join("preflight-publication-pending.r3er");
        let mut bytes = fs::read(&pending).unwrap();
        *bytes.last_mut().unwrap() ^= 1;
        fs::write(&pending, bytes).unwrap();
        call(
            &["save-preflight-verify", "--root", p(&root)],
            None,
            false,
            &d.path().join("corrupt-pending.log"),
        );
    }
}
#[test]
fn cooldown_actual_lr_continuous_and_fresh_process_resume() {
    let d = tempfile::tempdir().unwrap();
    let bootstrap =
        PathBuf::from(std::env::var_os("R3ER_TEST_BOOTSTRAP").expect("reuse TINY bootstrap"));
    let root = d.path().join("cooldown");
    call(
        &[
            "fixture-cooldown",
            "--from",
            p(&bootstrap),
            "--output",
            p(&root),
        ],
        None,
        true,
        &d.path().join("prepare.log"),
    );
    let continuous = root.join("continuous");
    let split = root.join("split");
    let a = call(
        &["run", "--root", p(&continuous)],
        None,
        true,
        &d.path().join("continuous.log"),
    );
    let b = call(
        &["run", "--root", p(&split)],
        None,
        true,
        &d.path().join("first.log"),
    );
    assert!(split.join("segment-00/final.r3m").is_file());
    let c = call(
        &[
            "run",
            "--root",
            p(&split),
            "--resume",
            "segment-00/terminal.r3er",
        ],
        None,
        true,
        &d.path().join("resume.log"),
    );
    let rates = |out: &Output| {
        String::from_utf8_lossy(&out.stdout)
            .lines()
            .filter(|l| l.starts_with("ACTUAL_TINY_UPDATE="))
            .map(|l| l.split("LR_BITS=").nth(1).unwrap().parse::<u64>().unwrap())
            .collect::<Vec<_>>()
    };
    let mut split_rates = rates(&b);
    split_rates.extend(rates(&c));
    assert_eq!(rates(&a), split_rates);
    assert_eq!(split_rates.len(), 2);
    assert_eq!(split_rates[0], 1e-4f64.to_bits());
    assert!(f64::from_bits(split_rates[1]) < 1e-4);
    call(
        &[
            "fixture-check",
            "--roots",
            p(&continuous),
            "--roots",
            p(&split),
        ],
        None,
        true,
        &d.path().join("check.log"),
    );
    println!(
        "COOLDOWN_NUMERIC TINY_UPDATES={} GENERATIONS=0 SCALAR_OPTIMIZER=0",
        rates(&a).len() + split_rates.len()
    );
}
#[test]
fn conditional_plan_sticky_failure_and_normal_wrong_completion() {
    let d = tempfile::tempdir().unwrap();
    let bootstrap = PathBuf::from(std::env::var_os("R3ER_TEST_BOOTSTRAP").unwrap());
    let mut generations = 0;
    for mode in [
        "pv-first-token",
        "pv-row-deadline",
        "conditional-teacher-error",
        "pv-final-write-fail",
        "conditional-final-sync",
        "pv-start-kill",
        "normal",
    ] {
        let root = d.path().join(mode);
        call(
            &[
                "fixture-conditional",
                "--from",
                p(&bootstrap),
                "--output",
                p(&root),
            ],
            None,
            true,
            &d.path().join("prepare.log"),
        );
        let first = call(
            &["conditional-run", "--root", p(&root), "--model", "0"],
            (mode != "normal").then_some(mode),
            mode == "normal",
            &d.path().join("first.log"),
        );
        generations += String::from_utf8_lossy(&first.stdout)
            .matches("VERIFICATION_GENERATION_API_ENTRY=")
            .count();
        if mode == "normal" {
            assert!(String::from_utf8_lossy(&first.stdout).contains("CONDITIONAL_EXACT=0/6"));
        }
        let original = root.join("model-0");
        let preserved = fs::read_dir(&original)
            .unwrap()
            .map(|e| {
                let e = e.unwrap();
                (e.file_name(), fs::read(e.path()).unwrap())
            })
            .collect::<Vec<_>>();
        let second = call(
            &["conditional-run", "--root", p(&root), "--model", "1"],
            None,
            mode == "normal",
            &d.path().join("second.log"),
        );
        let log = String::from_utf8_lossy(&second.stdout);
        let calls = log.matches("VERIFICATION_GENERATION_API_ENTRY=").count();
        generations += calls;
        if mode == "normal" {
            assert_eq!(calls, 6);
        } else {
            assert_eq!(calls, 0);
            assert!(!root.join("model-1").exists());
            assert!(!log.contains("TRAIN_PROBE_API_ENTRY="));
            let retry = call(
                &["conditional-run", "--root", p(&root), "--model", "0"],
                None,
                false,
                &d.path().join("retry.log"),
            );
            assert!(
                !String::from_utf8_lossy(&retry.stdout)
                    .contains("VERIFICATION_GENERATION_API_ENTRY=")
            );
        }
        if matches!(
            mode,
            "pv-first-token" | "pv-row-deadline" | "conditional-teacher-error"
        ) {
            assert!(original.join("generation-000.r3er").is_file());
        }
        if mode == "conditional-final-sync" {
            assert!(original.join("publication-pending.r3er").is_file());
            assert!(original.join("final.r3er").is_file());
        }
        for (name, bytes) in preserved {
            assert_eq!(fs::read(original.join(name)).unwrap(), bytes);
        }
    }
    let root = d.path().join("locked");
    call(
        &[
            "fixture-conditional",
            "--from",
            p(&bootstrap),
            "--output",
            p(&root),
        ],
        None,
        true,
        &d.path().join("prepare.log"),
    );
    let lock = fs::File::open(root.join("plan.r3er")).unwrap();
    lock.lock().unwrap();
    call(
        &["conditional-run", "--root", p(&root), "--model", "0"],
        None,
        false,
        &d.path().join("locked.log"),
    );
    assert!(!root.join("model-0").exists());
    println!("CONDITIONAL_STICKY_TINY_GENERATIONS={generations} TINY_UPDATES=0 SMALL_UPDATES=0");
}
#[test]
fn native_resume_ignores_ambient_policy_json() {
    use replica_v3::neural::checkpoint;
    let d = tempfile::tempdir().unwrap();
    let bootstrap = PathBuf::from(std::env::var_os("R3ER_TEST_BOOTSTRAP").unwrap());
    let fixture = d.path().join("fixture");
    call(
        &[
            "fixture-native-corpus",
            "--from",
            p(&bootstrap),
            "--output",
            p(&fixture),
        ],
        None,
        true,
        &d.path().join("prepare.log"),
    );
    let parent =
        checkpoint::load(&fixture.join("parent.r3m"), candle_core::Device::Cpu, true).unwrap();
    let step = parent.manifest.training.as_ref().unwrap().step;
    let mut expected = None;
    for (name, policy) in [
        ("absent", None),
        (
            "irrelevant",
            Some(br#"{"stage":"H3","constant_lr":0.0001,"entry":"EXPERIMENT_FORK"}"#.as_slice()),
        ),
        ("malformed", Some(b"{not-json".as_slice())),
    ] {
        let root = d.path().join(name);
        fs::create_dir_all(root.join("segment")).unwrap();
        let path = root.join("segment").join(if name == "malformed" {
            "renamed.r3m"
        } else {
            "parent.r3m"
        });
        fs::copy(fixture.join("parent.r3m"), &path).unwrap();
        if let Some(bytes) = policy {
            fs::write(root.join("policy.json"), bytes).unwrap();
        }
        let out = root.join("trained");
        let result = Command::new(env!("CARGO_BIN_EXE_replica-train"))
            .args([
                "train",
                "--resume",
                p(&path),
                "--corpus",
                p(&fixture.join("source.r3c")),
                "--output",
                p(&out),
                "--stop-after",
                &(step + 1).to_string(),
            ])
            .env("VECLIB_MAXIMUM_THREADS", "1")
            .env("RAYON_NUM_THREADS", "1")
            .output()
            .unwrap();
        print!(
            "{}{}",
            String::from_utf8_lossy(&result.stdout),
            String::from_utf8_lossy(&result.stderr)
        );
        assert!(result.status.success(), "{name}");
        let actual = checkpoint::load(&out.join("final"), candle_core::Device::Cpu, true).unwrap();
        assert_eq!(actual.manifest.trained_steps, step + 1);
        let state = (
            actual.model.weight_hash().unwrap(),
            actual.manifest.training,
            actual
                .optimizer
                .iter()
                .map(|(n, t)| {
                    (
                        n.clone(),
                        t.flatten_all().unwrap().to_vec1::<f32>().unwrap(),
                    )
                })
                .collect::<Vec<_>>(),
        );
        if let Some(prior) = &expected {
            assert_eq!(prior, &state);
        }
        expected = Some(state);
        assert_eq!(
            fs::read(&path).unwrap(),
            fs::read(fixture.join("parent.r3m")).unwrap()
        );
    }
    println!("AMBIENT_RESUME_TINY_UPDATES=3 SMALL_UPDATES=0");
}
#[test]
fn objective_policy_same_filename_different_weights_rejected_before_work() {
    use candle_core::{Device, Tensor};
    use replica_v3::neural::{artifact, checkpoint};
    let d = tempfile::tempdir().unwrap();
    let bootstrap = PathBuf::from(std::env::var_os("R3ER_TEST_BOOTSTRAP").unwrap());
    let root = d.path().join("native");
    call(
        &[
            "fixture-native-corpus",
            "--from",
            p(&bootstrap),
            "--output",
            p(&root),
        ],
        None,
        true,
        &d.path().join("prepare.log"),
    );
    let parent = root.join("parent.r3m");
    let renamed = d.path().join("renamed.r3m");
    fs::copy(&parent, &renamed).unwrap();
    let loaded = checkpoint::load(&renamed, Device::Cpu, true).unwrap();
    checkpoint::ResumeBinding::require_default(
        loaded.manifest.training.as_ref().unwrap(),
        &loaded.tokenizer,
    )
    .unwrap();
    let original_model = loaded.model.weight_hash().unwrap();
    assert_eq!(fs::read(&parent).unwrap(), fs::read(&renamed).unwrap());

    // A valid alternative artifact, not a checksum-corrupted byte stream.
    // Only this isolated fixture is replaced; the registered parent stays fixed.
    let var = loaded.model.vars.values().next().unwrap();
    let mut values = var.flatten_all().unwrap().to_vec1::<f32>().unwrap();
    values[0] += 0.25;
    var.set(&Tensor::from_vec(values, var.dims(), &Device::Cpu).unwrap())
        .unwrap();
    let alternate = d.path().join("alternate.r3m");
    artifact::save(
        &alternate,
        &loaded.model,
        &loaded.tokenizer,
        loaded.manifest.clone(),
        &loaded.optimizer,
    )
    .unwrap();
    fs::copy(&alternate, &parent).unwrap();
    let valid_alternate = checkpoint::load(&parent, Device::Cpu, true).unwrap();
    assert_ne!(valid_alternate.model.weight_hash().unwrap(), original_model);
    assert_eq!(valid_alternate.manifest.training, loaded.manifest.training);
    let out = call(
        &["run", "--root", p(&root)],
        None,
        false,
        &d.path().join("rejected.log"),
    );
    let log = String::from_utf8_lossy(&out.stdout);
    assert!(log.contains("native physical digest"), "{log}");
    for forbidden in [
        "ACTUAL_TINY_UPDATE=",
        "ACTUAL_SMALL_UPDATE=",
        "TRAIN_PROBE_API_ENTRY=",
    ] {
        assert!(!log.contains(forbidden), "{log}");
    }
    assert!(!root.join("segment-00").exists());
    assert!(!root.join("comparison.r3er").exists());
    println!("OBJECTIVE_REPLACEMENT_TINY_UPDATES=0 SMALL_UPDATES=0 GENERATIONS=0 TEACHERS=0");
}
#[test]
fn objective_policy_native_fresh_resume_and_zero_span_parity() {
    let d = tempfile::tempdir().unwrap();
    let bootstrap = PathBuf::from(std::env::var_os("R3ER_TEST_BOOTSTRAP").unwrap());
    let mut roots = Vec::new();
    let mut updates = 0;
    for span in [false, true] {
        let root = d.path().join(if span { "span" } else { "base" });
        call(
            &[
                "fixture-fork",
                "--from",
                p(&bootstrap),
                "--output",
                p(&root),
                "--objective-span",
                if span { "true" } else { "false" },
            ],
            None,
            true,
            &d.path().join("prepare.log"),
        );
        let out = call(
            &["run", "--root", p(&root)],
            span.then_some("raw:1"),
            true,
            &d.path().join("run.log"),
        );
        updates += String::from_utf8_lossy(&out.stdout)
            .lines()
            .filter(|l| l.starts_with("ACTUAL_TINY_UPDATE="))
            .count();
        if span {
            let out = call(
                &[
                    "run",
                    "--root",
                    p(&root),
                    "--resume",
                    "segment-00/terminal.r3er",
                ],
                None,
                true,
                &d.path().join("resume.log"),
            );
            updates += String::from_utf8_lossy(&out.stdout)
                .lines()
                .filter(|l| l.starts_with("ACTUAL_TINY_UPDATE="))
                .count();
        }
        assert!(root.join("train-final-final.r3er").is_file());
        // Move only the file: loss authorization must not depend on adjacent policy names.
        let isolated = d.path().join(if span {
            "moved-span"
        } else {
            "moved-native-base"
        });
        fs::create_dir(&isolated).unwrap();
        let endpoint = root.join(if span {
            "segment-01/step-0002.r3m"
        } else {
            "segment-00/step-0002.r3m"
        });
        let moved = isolated.join("renamed.r3m");
        fs::copy(&endpoint, &moved).unwrap();
        let result = Command::new(env!("CARGO_BIN_EXE_replica-train"))
            .args([
                "train",
                "--resume",
                p(&moved),
                "--numeric-probe",
                "--output",
                p(&isolated.join("forbidden")),
            ])
            .env("VECLIB_MAXIMUM_THREADS", "1")
            .env("RAYON_NUM_THREADS", "1")
            .output()
            .unwrap();
        assert!(!result.status.success());
        assert!(String::from_utf8_lossy(&result.stderr).contains("OBJECTIVE_POLICY_UNSUPPORTED"));
        assert!(!isolated.join("forbidden").exists());
        assert_eq!(fs::read(endpoint).unwrap(), fs::read(moved).unwrap());
        roots.push(root);
    }
    call(
        &[
            "fixture-check",
            "--roots",
            p(&roots[0]),
            "--roots",
            p(&roots[1]),
        ],
        None,
        true,
        &d.path().join("parity.log"),
    );
    assert_eq!(updates, 4);
    println!("OBJECTIVE_PROCESS_TINY_UPDATES={updates} SMALL_UPDATES=0");
}
#[test]
fn native_corpus_standalone_default_two_vs_fresh_one_one() {
    use replica_v3::neural::checkpoint;
    let d = tempfile::tempdir().unwrap();
    let bootstrap = PathBuf::from(std::env::var_os("R3ER_TEST_BOOTSTRAP").unwrap());
    let isolated = d.path().join("isolated");
    call(
        &[
            "fixture-native-corpus",
            "--from",
            p(&bootstrap),
            "--output",
            p(&isolated),
        ],
        None,
        true,
        &d.path().join("prepare.log"),
    );
    let parent_step = checkpoint::metadata(&isolated.join("parent.r3m"))
        .unwrap()
        .0
        .training
        .unwrap()
        .step;
    assert!(fs::read_dir(&isolated).unwrap().all(|e| matches!(
        e.unwrap().path().extension().and_then(|s| s.to_str()),
        Some("r3c" | "r3m" | "r3er")
    )));
    // macOS denies the setuid /bin/ps used by the unchanged RSS gate inside
    // sandbox-exec. Keep that gate: exercise an isolated native-only root and
    // verify its contents, rather than disable resource observation for a test.
    let mut compile = Command::new(env!("CARGO_BIN_EXE_replica-train"));
    compile
        .args([
            "recovery",
            "native",
            "token-cache-compile",
            "--root",
            p(&isolated),
            "--output",
            p(&isolated.join("cache")),
        ])
        .env("VECLIB_MAXIMUM_THREADS", "1")
        .env("RAYON_NUM_THREADS", "1");
    let compiled = compile.output().unwrap();
    assert!(
        compiled.status.success(),
        "{}",
        String::from_utf8_lossy(&compiled.stderr)
    );
    let run = |source: &Path, out: &Path, stop: bool| {
        let mut c = Command::new(env!("CARGO_BIN_EXE_replica-train"));
        c.args([
            "train",
            "--resume",
            p(source),
            "--corpus",
            p(&isolated.join("source.r3c")),
            "--output",
            p(out),
        ])
        .env("VECLIB_MAXIMUM_THREADS", "1")
        .env("RAYON_NUM_THREADS", "1");
        if stop {
            c.args(["--stop-after", &(parent_step + 1).to_string()]);
        }
        let o = c.output().unwrap();
        print!(
            "{}{}",
            String::from_utf8_lossy(&o.stdout),
            String::from_utf8_lossy(&o.stderr)
        );
        assert!(o.status.success());
        let updates = String::from_utf8_lossy(&o.stdout)
            .lines()
            .filter(|line| line.starts_with("step="))
            .count();
        assert_eq!(
            updates,
            if out.file_name().unwrap() == "continuous" {
                2
            } else {
                1
            }
        );
    };
    let a = isolated.join("continuous");
    let b = isolated.join("one");
    let c = isolated.join("resumed");
    run(&isolated.join("parent.r3m"), &a, false);
    run(&isolated.join("parent.r3m"), &b, true);
    let moved = isolated.join("only-checkpoint.r3m");
    fs::copy(b.join("final"), &moved).unwrap();
    run(&moved, &c, false);
    let x = checkpoint::load(&a.join("final"), candle_core::Device::Cpu, true).unwrap();
    let y = checkpoint::load(&c.join("final"), candle_core::Device::Cpu, true).unwrap();
    assert_eq!(
        x.model.weight_hash().unwrap(),
        y.model.weight_hash().unwrap()
    );
    assert_eq!(x.manifest.training, y.manifest.training);
    for (name, t) in &x.optimizer {
        assert_eq!(
            t.flatten_all().unwrap().to_vec1::<f32>().unwrap(),
            y.optimizer[name]
                .flatten_all()
                .unwrap()
                .to_vec1::<f32>()
                .unwrap()
        );
    }
    let evaluated = Command::new(env!("CARGO_BIN_EXE_replica-train"))
        .args([
            "evaluate",
            "--checkpoint",
            p(&c.join("final")),
            "--corpus",
            p(&isolated.join("source.r3c")),
            "--output",
            p(&isolated.join("evaluation.r3er")),
            "--limit",
            "1",
        ])
        .env("VECLIB_MAXIMUM_THREADS", "1")
        .env("RAYON_NUM_THREADS", "1")
        .output()
        .unwrap();
    assert!(
        evaluated.status.success(),
        "{}",
        String::from_utf8_lossy(&evaluated.stderr)
    );
    assert!(
        fs::read(isolated.join("evaluation.r3er"))
            .unwrap()
            .starts_with(b"R3ER")
    );
    println!(
        "NATIVE_DEFAULT_STANDALONE_PARITY=PASS TINY_OPTIMIZER_CALLS=4 SMALL_OPTIMIZER_CALLS=0"
    );
    let pair = d.path().join("native-pair");
    fs::create_dir(&pair).unwrap();
    for (name, split) in [("legacy-reference", false), ("native", true)] {
        let dir = pair.join(name);
        let mut args = vec![
            "fixture-native-corpus",
            "--from",
            p(&bootstrap),
            "--output",
            p(&dir),
        ];
        if split {
            args.push("--split");
        }
        call(&args, None, true, &d.path().join("pair-prepare.log"));
        call(
            &[
                "token-cache-compile",
                "--root",
                p(&dir),
                "--output",
                p(&dir.join("cache")),
            ],
            None,
            true,
            &d.path().join("pair-cache.log"),
        );
        call(
            &["run", "--root", p(&dir)],
            None,
            true,
            &d.path().join("pair-run.log"),
        );
        if split {
            call(
                &[
                    "run",
                    "--root",
                    p(&dir),
                    "--resume",
                    "segment-00/terminal.r3er",
                ],
                None,
                true,
                &d.path().join("pair-resume.log"),
            );
        }
    }
    call(
        &["path-verify", "--root", p(&pair)],
        None,
        true,
        &d.path().join("pair-verify.log"),
    );
    println!("NATIVE_CACHE_PROCESS_PARITY=PASS ADDITIONAL_TINY_OPTIMIZER_CALLS=4");
}
#[test]
fn objective_policy_nonzero_span_continuous_and_process_resume() {
    let d = tempfile::tempdir().unwrap();
    let bootstrap = PathBuf::from(std::env::var_os("R3ER_TEST_BOOTSTRAP").unwrap());
    let mut roots = Vec::new();
    let mut updates = 0;
    for split in [false, true] {
        let root = d.path().join(if split { "split" } else { "continuous" });
        call(
            &[
                "fixture-fork",
                "--from",
                p(&bootstrap),
                "--output",
                p(&root),
                "--objective-span",
                "true",
                "--nonzero-role",
            ],
            None,
            true,
            &d.path().join("prepare.log"),
        );
        let o = call(
            &["run", "--root", p(&root)],
            split.then_some("raw:1"),
            true,
            &d.path().join("run.log"),
        );
        updates += String::from_utf8_lossy(&o.stdout)
            .lines()
            .filter(|s| s.starts_with("ACTUAL_TINY_UPDATE="))
            .count();
        if split {
            let o = call(
                &[
                    "run",
                    "--root",
                    p(&root),
                    "--resume",
                    "segment-00/terminal.r3er",
                ],
                None,
                true,
                &d.path().join("resume.log"),
            );
            updates += String::from_utf8_lossy(&o.stdout)
                .lines()
                .filter(|s| s.starts_with("ACTUAL_TINY_UPDATE="))
                .count();
        }
        roots.push(root);
    }
    call(
        &[
            "fixture-check",
            "--roots",
            p(&roots[0]),
            "--roots",
            p(&roots[1]),
        ],
        None,
        true,
        &d.path().join("parity.log"),
    );
    let a = replica_v3::neural::checkpoint::load(
        &roots[0].join("segment-00/step-0002.r3m"),
        candle_core::Device::Cpu,
        true,
    )
    .unwrap();
    let b = replica_v3::neural::checkpoint::load(
        &roots[1].join("segment-01/step-0002.r3m"),
        candle_core::Device::Cpu,
        true,
    )
    .unwrap();
    assert_eq!(a.manifest.training, b.manifest.training);
    assert_eq!(updates, 4);
    println!("NONZERO_SPAN_RESUME=PASS TINY_UPDATES={updates} SMALL_UPDATES=0");
}
#[test]
fn restart_and_cooldown_partial_panels_resume_to_close() {
    let d = tempfile::tempdir().unwrap();
    let bootstrap = PathBuf::from(std::env::var_os("R3ER_TEST_BOOTSTRAP").unwrap());
    let mut updates = 0;
    for flag in ["--restart-spec", "--cooldown-panel"] {
        for boundary in [128, 256] {
            for prefix in [0, 1, 2] {
                let root = d.path().join(format!("{flag}-{boundary}-{prefix}"));
                call(
                    &[
                        "fixture-fork",
                        "--from",
                        p(&bootstrap),
                        "--output",
                        p(&root),
                        flag,
                        "--panel-rows",
                        "3",
                    ],
                    None,
                    true,
                    &d.path().join("prepare.log"),
                );
                let out = call(
                    &["run", "--root", p(&root)],
                    Some(&format!("partial-dev:{boundary}:{prefix}")),
                    true,
                    &d.path().join("pause.log"),
                );
                updates += String::from_utf8_lossy(&out.stdout)
                    .lines()
                    .filter(|l| l.starts_with("ACTUAL_TINY_UPDATE="))
                    .count();
                assert!(
                    String::from_utf8_lossy(&out.stdout).contains("COMMAND_FINALIZATION=TimePause")
                );
                let raw_name = format!(
                    "segment-00/dev-{:04}.r3er",
                    if boundary == 128 { 1 } else { 2 }
                );
                let original = fs::read(root.join(&raw_name)).unwrap();
                let out = call(
                    &[
                        "run",
                        "--root",
                        p(&root),
                        "--resume",
                        "segment-00/terminal.r3er",
                    ],
                    None,
                    true,
                    &d.path().join("resume.log"),
                );
                updates += String::from_utf8_lossy(&out.stdout)
                    .lines()
                    .filter(|l| l.starts_with("ACTUAL_TINY_UPDATE="))
                    .count();
                assert_eq!(original, fs::read(root.join(raw_name)).unwrap());
                call(
                    &[
                        "close",
                        "--root",
                        p(&root),
                        "--terminal",
                        "segment-01/terminal.r3er",
                    ],
                    None,
                    true,
                    &d.path().join("close.log"),
                );
                println!(
                    "PARTIAL_PROCESS contract={flag} logical={boundary} prefix={prefix} CLOSED=true TINY_UPDATES_CUMULATIVE={updates}"
                );
            }
        }
    }
    assert_eq!(updates, 24);
    println!("PARTIAL_RESUME_ACTUAL_TINY_UPDATES={updates} SMALL_UPDATES=0");
}
#[test]
fn binary_preflight_attempt_failures_usage_and_single_writer() {
    fn copy_dir(from: &Path, to: &Path) {
        fs::create_dir(to).unwrap();
        for e in fs::read_dir(from).unwrap() {
            let e = e.unwrap();
            let dest = to.join(e.file_name());
            if e.file_type().unwrap().is_dir() {
                copy_dir(&e.path(), &dest);
            } else {
                fs::copy(e.path(), dest).unwrap();
            }
        }
    }
    let d = tempfile::tempdir().unwrap();
    let bootstrap = PathBuf::from(
        std::env::var_os("R3ER_TEST_BOOTSTRAP").expect("reuse registered TINY bootstrap"),
    );
    let prepared = d.path().join("prepared");
    call(
        &[
            "fixture-preflight",
            "--from",
            p(&bootstrap),
            "--output",
            p(&prepared),
        ],
        None,
        true,
        &d.path().join("prepare.log"),
    );
    let mut updates = 0;
    for (arm, resume) in [
        ("preflight-A", None),
        ("preflight-B", None),
        ("preflight-B", Some("segment-00/terminal.r3er")),
    ] {
        let path = prepared.join(arm);
        let mut args = vec!["run", "--root", p(&path)];
        if let Some(r) = resume {
            args.extend(["--resume", r]);
        }
        let out = call(
            &args,
            None,
            true,
            &d.path()
                .join(format!("prepare-{arm}-{}.log", resume.is_some())),
        );
        updates += String::from_utf8_lossy(&out.stdout)
            .lines()
            .filter(|l| l.starts_with("ACTUAL_TINY_UPDATE="))
            .count();
    }
    assert_eq!(updates, 4);
    let prestart = d.path().join("prestart-rejected");
    copy_dir(&prepared, &prestart);
    let rejected = call(
        &["save-preflight-verify", "--root", p(&prestart)],
        Some("pv-start-write-fail"),
        false,
        &d.path().join("prestart.log"),
    );
    assert!(!prestart.join("preflight-start.r3er").exists());
    assert!(
        !String::from_utf8_lossy(&rejected.stdout).contains("VERIFICATION_GENERATION_API_ENTRY=")
    );
    assert!(!String::from_utf8_lossy(&rejected.stdout).contains("SAVE_CAPABLE="));
    let mut entries = 0;
    for (mode, expected, final_file, proof) in [
        ("pv-first-token", 1, true, false),
        ("pv-nonfinite", 1, true, false),
        ("pv-after-row", 1, true, false),
        ("pv-row-deadline", 1, true, false),
        ("pv-start-kill", 0, false, false),
        ("pv-raw-write-fail", 1, true, false),
        ("pv-proof-write-fail", 2, true, false),
        ("pv-proof-kill", 2, false, true),
        ("pv-final-write-fail", 2, false, true),
        ("pv-after-row-final-write-fail", 1, false, false),
    ] {
        let root = d.path().join(mode);
        copy_dir(&prepared, &root);
        let out = call(
            &["save-preflight-verify", "--root", p(&root)],
            Some(mode),
            false,
            &d.path().join(format!("{mode}.log")),
        );
        assert!(root.join("preflight-start.r3er").is_file());
        assert_eq!(root.join("preflight-final.r3er").is_file(), final_file);
        assert_eq!(root.join("preflight-proof.r3er").is_file(), proof);
        let text = String::from_utf8_lossy(&out.stdout);
        if final_file {
            assert!(
                text.contains(&format!("GENERATION_API_ENTRIES={expected}")),
                "{text}"
            );
        }
        if matches!(
            mode,
            "pv-first-token" | "pv-nonfinite" | "pv-after-row" | "pv-row-deadline"
        ) {
            assert!(root.join("preflight-row-00.r3er").is_file());
            assert!(!root.join("preflight-row-01-start.r3er").exists());
        }
        let before = fs::read(root.join("preflight-start.r3er")).unwrap();
        for args in [
            vec!["save-preflight-verify", "--root", p(&root)],
            vec!["run", "--root", p(&root.join("C50"))],
            vec!["anchor-report", "--root", p(&root)],
        ] {
            let retry = call(&args, None, false, &d.path().join("retry.log"));
            let text = String::from_utf8_lossy(&retry.stdout);
            assert!(!text.contains("ACTUAL_TINY_UPDATE="));
            assert!(!text.contains("VERIFICATION_BEGIN_DURABLE=true"));
            if !final_file {
                assert!(text.contains("UNKNOWN_TAIL=true"), "{text}");
            }
        }
        assert_eq!(before, fs::read(root.join("preflight-start.r3er")).unwrap());
        let actual = text
            .lines()
            .filter(|l| l.starts_with("VERIFICATION_GENERATION_API_ENTRY="))
            .count();
        assert_eq!(actual, expected);
        entries += actual;
    }
    let root = d.path().join("positive");
    copy_dir(&prepared, &root);
    let out = call(
        &["save-preflight-verify", "--root", p(&root)],
        None,
        true,
        &d.path().join("positive.log"),
    );
    assert!(String::from_utf8_lossy(&out.stdout).contains("GENERATION_API_ENTRIES=2"));
    entries += String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter(|l| l.starts_with("VERIFICATION_GENERATION_API_ENTRY="))
        .count();
    let proof = fs::read(root.join("preflight-proof.r3er")).unwrap();
    let final_bytes = fs::read(root.join("preflight-final.r3er")).unwrap();
    let out = call(
        &["save-preflight-verify", "--root", p(&root)],
        None,
        true,
        &d.path().join("readonly.log"),
    );
    assert!(String::from_utf8_lossy(&out.stdout).contains("ACTUAL_NEW_GENERATIONS=0"));
    assert_eq!(proof, fs::read(root.join("preflight-proof.r3er")).unwrap());
    assert_eq!(
        final_bytes,
        fs::read(root.join("preflight-final.r3er")).unwrap()
    );
    let mut corrupt = final_bytes.clone();
    *corrupt.last_mut().unwrap() ^= 1;
    fs::write(root.join("preflight-final.r3er"), corrupt).unwrap();
    call(
        &["save-preflight-verify", "--root", p(&root)],
        None,
        false,
        &d.path().join("corrupt-final.log"),
    );
    fs::write(root.join("preflight-final.r3er"), final_bytes).unwrap();
    fs::copy(
        root.join("preflight-A/parent.r3m"),
        root.join("preflight-A/segment-00/final.r3m"),
    )
    .unwrap();
    call(
        &["save-preflight-verify", "--root", p(&root)],
        None,
        false,
        &d.path().join("wrong-native.log"),
    );

    let root = d.path().join("race");
    copy_dir(&prepared, &root);
    let race_log = d.path().join("race.log");
    let file = fs::File::create(&race_log).unwrap();
    let mut child = Command::new(env!("CARGO_BIN_EXE_replica-train"))
        .args([
            "recovery",
            "native",
            "save-preflight-verify",
            "--root",
            p(&root),
        ])
        .env("VECLIB_MAXIMUM_THREADS", "1")
        .env("RAYON_NUM_THREADS", "1")
        .env("R3ER_TEST_STOP", "pv-hold-start")
        .stdout(Stdio::from(file.try_clone().unwrap()))
        .stderr(Stdio::from(file))
        .spawn()
        .unwrap();
    let deadline = Instant::now() + Duration::from_secs(10);
    while !root.join("preflight-start.r3er").exists() && Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(5));
    }
    assert!(root.join("preflight-start.r3er").is_file());
    call(
        &["save-preflight-verify", "--root", p(&root)],
        None,
        false,
        &d.path().join("race-second.log"),
    );
    loop {
        if let Some(status) = child.try_wait().unwrap() {
            assert!(
                status.success(),
                "{}",
                fs::read_to_string(&race_log).unwrap()
            );
            break;
        }
        if Instant::now() > deadline {
            let _ = child.kill();
            let _ = child.wait();
            panic!("race child timeout");
        }
        std::thread::sleep(Duration::from_millis(5));
    }
    entries += fs::read_to_string(&race_log)
        .unwrap()
        .lines()
        .filter(|l| l.starts_with("VERIFICATION_GENERATION_API_ENTRY="))
        .count();
    println!(
        "PV01_ACTUAL_TINY_UPDATES={updates} VERIFICATION_GENERATION_ENTRIES={entries} TEACHERS=0 FOLLOWUP_OPTIMIZER_ENTRIES=0"
    );
}
#[test]
fn binary_save_preflight_and_partial_panel_process_resume() {
    let d = tempfile::tempdir().unwrap();
    let bootstrap = if let Some(path) = std::env::var_os("R3ER_TEST_BOOTSTRAP") {
        PathBuf::from(path)
    } else {
        let path = d.path().join("bootstrap");
        call(
            &["fixture", "--output", p(&path)],
            None,
            true,
            &d.path().join("bootstrap-prepare.log"),
        );
        call(
            &["run", "--root", p(&path)],
            None,
            true,
            &d.path().join("bootstrap-run.log"),
        );
        path
    };
    let root = d.path().join("preflight");
    call(
        &[
            "fixture-preflight",
            "--from",
            p(&bootstrap),
            "--output",
            p(&root),
        ],
        None,
        true,
        &d.path().join("prepare.log"),
    );
    for arm in ["preflight-A", "preflight-B"] {
        call(
            &["run", "--root", p(&root.join(arm))],
            None,
            true,
            &d.path().join(format!("{arm}.log")),
        );
    }
    call(
        &[
            "run",
            "--root",
            p(&root.join("preflight-B")),
            "--resume",
            "segment-00/terminal.r3er",
        ],
        None,
        true,
        &d.path().join("resume.log"),
    );
    let out = call(
        &["save-preflight-verify", "--root", p(&root)],
        None,
        true,
        &d.path().join("verify.log"),
    );
    assert!(String::from_utf8_lossy(&out.stdout).contains("quality_eligible=false"));
    let partial = d.path().join("partial");
    call(
        &[
            "fixture-fork",
            "--from",
            p(&bootstrap),
            "--output",
            p(&partial),
            "--restart-spec",
        ],
        None,
        true,
        &d.path().join("partial-prepare.log"),
    );
    call(
        &["run", "--root", p(&partial)],
        Some("partial-dev"),
        true,
        &d.path().join("partial.log"),
    );
    let raw = fs::read(partial.join("segment-00/dev-0001.r3er")).unwrap();
    let terminal = fs::read(partial.join("segment-00/terminal.r3er")).unwrap();
    call(
        &[
            "run",
            "--root",
            p(&partial),
            "--resume",
            "segment-00/terminal.r3er",
        ],
        None,
        true,
        &d.path().join("partial-resume.log"),
    );
    assert_eq!(
        raw,
        fs::read(partial.join("segment-00/dev-0001.r3er")).unwrap()
    );
    assert_eq!(
        terminal,
        fs::read(partial.join("segment-00/terminal.r3er")).unwrap()
    );
    assert!(partial.join("comparison.r3er").is_file());
    call(
        &[
            "close",
            "--root",
            p(&partial),
            "--terminal",
            "segment-01/terminal.r3er",
        ],
        None,
        true,
        &d.path().join("partial-close.log"),
    );
    println!("PREFLIGHT_REGRESSION_TINY_UPDATES=6 SMALL_UPDATES=0 GENERATIONS=8 TEACHERS=6");
}
#[test]
fn binary_post_terminal_failure_blocks_pair_and_report() {
    let d = tempfile::tempdir().unwrap();
    let bootstrap = if let Some(p) = std::env::var_os("R3ER_TEST_BOOTSTRAP") {
        PathBuf::from(p)
    } else {
        let p = d.path().join("bootstrap");
        call(
            &["fixture", "--output", p.to_str().unwrap()],
            None,
            true,
            &d.path().join("fixture.log"),
        );
        call(
            &["run", "--root", p.to_str().unwrap()],
            None,
            true,
            &d.path().join("bootstrap.log"),
        );
        p
    };
    for (i, fault) in [
        "cancel-close",
        "error-close",
        "cancel-close-stop-write-fail",
        "cancel-close-command-write-fail",
        "cancel-close-stop-write-fail-command-write-fail",
        "command-write-fail",
    ]
    .into_iter()
    .enumerate()
    {
        let pair = d.path().join(format!("pair-{i}"));
        call(
            &[
                "fixture-pair",
                "--from",
                p(&bootstrap),
                "--output",
                p(&pair),
            ],
            None,
            true,
            &d.path().join(format!("prepare-{i}.log")),
        );
        let c = pair.join("C50");
        let a = pair.join("A75");
        let out = call(
            &["run", "--root", p(&c)],
            Some(fault),
            false,
            &d.path().join(format!("run-{i}.log")),
        );
        assert!(String::from_utf8_lossy(&out.stdout).contains("complete=true"));
        assert!(String::from_utf8_lossy(&out.stdout).contains("NEW_TINY_UPDATES=2"));
        let terminal = fs::read(c.join("segment-00/terminal.r3er")).unwrap();
        if fault.contains("stop-write-fail") {
            assert!(!c.join("close-stop.r3er").exists());
        }
        if fault.contains("command-write-fail") {
            assert!(!c.join("segment-00/command.r3er").exists());
        }
        assert_eq!(
            c.join("comparison.r3er").exists(),
            fault == "command-write-fail"
        );
        let out = call(
            &["run", "--root", p(&a)],
            None,
            false,
            &d.path().join(format!("other-{i}.log")),
        );
        assert!(!String::from_utf8_lossy(&out.stdout).contains("ACTUAL_TINY_UPDATE"));
        assert!(!a.join("segment-00").exists());
        let before = fs::read_dir(&c).unwrap().count();
        let out = call(
            &["anchor-report", "--root", p(&pair)],
            None,
            false,
            &d.path().join(format!("report-{i}.log")),
        );
        assert!(String::from_utf8_lossy(&out.stdout).contains("MODEL_PAIR=FAILED_OR_INCOMPLETE"));
        assert!(!String::from_utf8_lossy(&out.stdout).contains("candidate=true"));
        assert_eq!(fs::read_dir(&c).unwrap().count(), before);
        call(
            &[
                "close",
                "--root",
                p(&c),
                "--terminal",
                "segment-00/terminal.r3er",
            ],
            None,
            false,
            &d.path().join(format!("close-{i}.log")),
        );
        assert_eq!(
            terminal,
            fs::read(c.join("segment-00/terminal.r3er")).unwrap()
        );
    }
    println!("POST_TERMINAL_TINY_UPDATES=12 GENERATIONS=36 TEACHERS=36 OTHER_ARM_UPDATES=0");
}
#[test]
fn binary_stored_cleanup_cancel_fresh_process_close() {
    let temporary = tempfile::tempdir().unwrap();
    let root = std::env::var_os("R3ER_CANCEL_TEST_OUTPUT")
        .map(PathBuf::from)
        .unwrap_or_else(|| temporary.path().join("cancel"));
    fs::create_dir(&root).unwrap();
    let bootstrap = if let Some(path) = std::env::var_os("R3ER_TEST_BOOTSTRAP") {
        PathBuf::from(path)
    } else {
        let path = root.join("bootstrap");
        call(
            &["fixture", "--output", p(&path)],
            None,
            true,
            &root.join("fixture.log"),
        );
        call(
            &["run", "--root", p(&path)],
            None,
            true,
            &root.join("bootstrap.log"),
        );
        path
    };
    let run = root.join("run");
    call(
        &["fixture-fork", "--from", p(&bootstrap), "--output", p(&run)],
        None,
        true,
        &root.join("fork.log"),
    );
    let out = call(
        &["run", "--root", p(&run)],
        Some("stored-cleanup-cancel"),
        false,
        &root.join("cancel.log"),
    );
    assert_eq!(out.status.code(), Some(91));
    let output = String::from_utf8_lossy(&out.stdout);
    assert!(output.contains("NEW_TINY_UPDATES=2"));
    assert!(output.contains("Cancelled"));
    assert!(output.contains("complete=true"));
    assert!(output.contains("NATIVE_FINAL_REUSED="));
    assert!(!run.join("segment-00/final.r3m").exists());
    assert!(!run.join("close-stop.r3er").exists());
    assert!(!run.join("comparison.r3er").exists());
    let terminal = fs::read(run.join("segment-00/terminal.r3er")).unwrap();
    call(
        &[
            "close",
            "--root",
            p(&run),
            "--terminal",
            "segment-00/terminal.r3er",
        ],
        None,
        false,
        &root.join("fresh-close.log"),
    );
    assert!(!run.join("comparison.r3er").exists());
    assert_eq!(
        terminal,
        fs::read(run.join("segment-00/terminal.r3er")).unwrap()
    );
    assert!(!run.join("segment-01").exists());
    println!(
        "STORED_CANCEL_TINY_UPDATES=2 (plus bootstrap24 only when generated) GENERATIONS=6 TEACHERS=6 FRESH_CLOSE_UPDATES=0"
    );
}
#[test]
fn binary_real_process_resume_and_close() {
    let temporary = tempfile::tempdir().unwrap();
    let root = std::env::var_os("R3ER_TEST_OUTPUT")
        .map(PathBuf::from)
        .unwrap_or_else(|| temporary.path().join("e2e"));
    fs::create_dir(&root).unwrap();
    let bootstrap = if let Some(path) = std::env::var_os("R3ER_TEST_BOOTSTRAP") {
        PathBuf::from(path)
    } else {
        let path = root.join("bootstrap");
        call(
            &["fixture", "--output", p(&path)],
            None,
            true,
            &root.join("fixture.log"),
        );
        call(
            &["run", "--root", p(&path)],
            None,
            true,
            &root.join("bootstrap.log"),
        );
        path
    };
    let mut normal = Vec::new();
    for (name, stop) in [
        ("continuous", None),
        ("after-raw", Some("raw:1")),
        ("after-checkpoint", Some("checkpoint:1")),
        ("final-split", Some("checkpoint:2")),
    ] {
        let path = root.join(name);
        call(
            &[
                "fixture-fork",
                "--from",
                p(&bootstrap),
                "--output",
                p(&path),
            ],
            None,
            true,
            &root.join(format!("{name}-fork.log")),
        );
        let original = fs::read(path.join("parent.r3m")).unwrap();
        call(
            &["run", "--root", p(&path)],
            stop,
            true,
            &root.join(format!("{name}-first.log")),
        );
        if stop.is_some() {
            assert!(!path.join("comparison.r3er").exists());
            let raw = fs::read(path.join("segment-00/dev-0001.r3er")).unwrap();
            let terminal = fs::read(path.join("segment-00/terminal.r3er")).unwrap();
            if name == "after-raw" {
                assert!(!path.join("segment-00/step-0001.r3m").exists());
            }
            call(
                &[
                    "run",
                    "--root",
                    p(&path),
                    "--resume",
                    "segment-00/terminal.r3er",
                ],
                None,
                true,
                &root.join(format!("{name}-resume.log")),
            );
            assert_eq!(
                raw,
                fs::read(path.join("segment-00/dev-0001.r3er")).unwrap()
            );
            assert_eq!(
                terminal,
                fs::read(path.join("segment-00/terminal.r3er")).unwrap()
            );
            assert!(!path.join("segment-01/dev-0001.r3er").exists());
            // The resumed segment verifies/references the already durable identical state.
            assert!(!path.join("segment-01/step-0001.r3m").exists());
            if name == "final-split" {
                assert!(!path.join("segment-01/step-0002.r3m").exists());
            }
        }
        assert_eq!(original, fs::read(path.join("parent.r3m")).unwrap());
        assert!(path.join("comparison.r3er").exists());
        // Native-only directory proof: no corpus/policy/receipt JSON is copied into this root.
        fn binary_only(root: &Path) {
            for e in fs::read_dir(root).unwrap() {
                let p = e.unwrap().path();
                if p.is_dir() {
                    binary_only(&p)
                } else {
                    assert!(
                        matches!(p.extension().and_then(|x| x.to_str()), Some("r3er" | "r3m")),
                        "{p:?}"
                    );
                }
            }
        }
        binary_only(&path);
        normal.push(path);
    }
    let mut args = vec!["fixture-check"];
    for root in &normal {
        args.extend(["--roots", p(root)]);
    }
    call(&args, None, true, &root.join("parity-check.log"));
    for (name, untrained) in [("cancel", false), ("quality-and-cancel", true)] {
        let path = root.join(name);
        // A reused bootstrap can itself be a trained fork. Its parent is then not
        // random-init; construct only the untrained fixture (zero optimizer calls).
        let random_init = root.join("random-init");
        let input = if untrained {
            call(
                &["fixture", "--output", p(&random_init)],
                None,
                true,
                &root.join("random-init.log"),
            );
            &random_init
        } else {
            &bootstrap
        };
        let mut args = vec!["fixture-fork", "--from", p(input), "--output", p(&path)];
        if untrained {
            args.push("--untrained");
        }
        call(&args, None, true, &root.join(format!("{name}-fork.log")));
        let out = call(
            &["run", "--root", p(&path)],
            Some("cancel-checkpoint:1"),
            false,
            &root.join(format!("{name}.log")),
        );
        let output = String::from_utf8_lossy(&out.stdout);
        assert!(output.contains("NEW_TINY_UPDATES=1"));
        assert!(output.contains("Cancelled"));
        assert!(output.contains("TimeBudget"));
        assert!(output.contains("resume=false"));
        if untrained {
            assert!(output.contains("QualityGuard"));
        }
        assert!(!path.join("comparison.r3er").exists());
        call(
            &[
                "run",
                "--root",
                p(&path),
                "--resume",
                "segment-00/terminal.r3er",
            ],
            None,
            false,
            &root.join(format!("{name}-resume-rejected.log")),
        );
        assert!(!path.join("segment-01").exists());
    }
    println!(
        "E2E_ACTUAL_TINY_UPDATES=10 (plus bootstrap24 only when generated); SCALAR=0; SMALL=0; GENERATIONS=28; TEACHERS=28; four distinct complete lineages, two sticky stops"
    );
}
