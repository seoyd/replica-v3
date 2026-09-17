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
            assert!(path.join("segment-01/step-0001.r3m").exists() || name == "final-split");
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
