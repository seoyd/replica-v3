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
        let mut args = vec![
            "fixture-fork",
            "--from",
            p(&bootstrap),
            "--output",
            p(&path),
        ];
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
