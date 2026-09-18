#![cfg(feature = "test-support")]
//! Real native subprocess regression. Test-only tiny fixture; never model quality evidence.
use std::{
    fs,
    path::{Path, PathBuf},
    process::{Command, Output},
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
    let out = command.output().unwrap();
    fs::write(log, [out.stdout.as_slice(), out.stderr.as_slice()].concat()).unwrap();
    print!("{}", String::from_utf8_lossy(&out.stdout));
    assert_eq!(
        out.status.success(),
        success,
        "args={args:?}\n{}",
        String::from_utf8_lossy(&out.stderr)
    );
    out
}
fn p(p: &Path) -> &str {
    p.to_str().unwrap()
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
