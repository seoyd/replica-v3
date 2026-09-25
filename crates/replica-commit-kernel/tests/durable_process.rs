use replica_commit_kernel::{
    durable::{DurableKernel, Snapshot, SnapshotCodec},
    kernel::Kernel,
    vectors::load_frozen_bundle,
};
use std::{
    fs, io,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    time::{Duration, Instant},
};

struct Native;
impl SnapshotCodec for Native {
    fn encode(&self, s: &Snapshot) -> io::Result<Vec<u8>> {
        replica_v3::binary::to_vec(s).map_err(io::Error::other)
    }
    fn decode(&self, bytes: &[u8]) -> io::Result<Snapshot> {
        replica_v3::binary::from_canonical_slice(bytes).map_err(io::Error::other)
    }
}
fn fixture() -> replica_commit_kernel::vectors::Bundle {
    load_frozen_bundle("tests/data/commit_kernel_reference_vectors_v1_2B.json").unwrap()
}
fn root(name: &str) -> PathBuf {
    let base = PathBuf::from(
        std::env::var_os("R3_KERNEL_PROCESS_ROOT").expect("explicit new evidence root"),
    );
    fs::create_dir_all(&base).unwrap();
    base.join(name)
}
fn child(store: &Path, mode: &str, count: usize, crash: Option<&str>) -> Command {
    let mut cmd = Command::new(std::env::current_exe().unwrap());
    cmd.args(["--ignored", "--exact", "process_worker", "--nocapture"])
        .env("R3_KERNEL_STORE", store)
        .env("R3_KERNEL_MODE", mode)
        .env("R3_KERNEL_COUNT", count.to_string())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    if let Some(point) = crash {
        cmd.env("R3_KERNEL_CRASH", point);
    }
    cmd
}

#[test]
#[ignore = "invoked only as a fresh child process by durable boundary tests"]
fn process_worker() {
    let store = PathBuf::from(std::env::var_os("R3_KERNEL_STORE").expect("child path"));
    let mode = std::env::var("R3_KERNEL_MODE").unwrap();
    let count: usize = std::env::var("R3_KERNEL_COUNT").unwrap().parse().unwrap();
    let b = fixture();
    let v = b
        .directed_vectors
        .iter()
        .find(|v| v.name == "two_valid_operations")
        .unwrap();
    let mut kernel = if mode == "prefix" {
        DurableKernel::create(&store, v.initial_state_v1_2b.clone(), Native).unwrap()
    } else {
        DurableKernel::open(&store, Native).unwrap()
    };
    let events: Vec<_> = v
        .events
        .iter()
        .filter(|e| e.kind == "ATTEMPT")
        .cloned()
        .collect();
    if mode == "suffix" {
        for e in events.iter().skip(count) {
            kernel.apply(e).unwrap();
        }
    } else if mode == "crash" {
        let point = std::env::var("R3_KERNEL_CRASH").unwrap();
        kernel
            .apply_with_hook(&events[0], &mut |at| {
                if at == point {
                    fs::write(store.join("reached"), at)?;
                    loop {
                        std::thread::sleep(Duration::from_secs(1));
                    }
                }
                Ok(())
            })
            .unwrap();
    } else {
        for e in events.iter().take(count) {
            kernel.apply(e).unwrap();
        }
    }
    println!(
        "child mode={mode} journal={} ledger={} nonces={}",
        kernel.snapshot().unwrap().journal.len(),
        kernel.snapshot().unwrap().state.operation_ledger.len(),
        kernel.snapshot().unwrap().state.used_nonces.len()
    );
}

#[test]
#[ignore = "explicit evidence root; real fresh processes, R3BIN, no model calls"]
fn prefix_restart_matches_continuous() {
    let b = fixture();
    let v = b
        .directed_vectors
        .iter()
        .find(|v| v.name == "two_valid_operations")
        .unwrap();
    let events: Vec<_> = v
        .events
        .iter()
        .filter(|e| e.kind == "ATTEMPT")
        .cloned()
        .collect();
    let mut reference = Kernel::new(v.initial_state_v1_2b.clone());
    let journal: Vec<_> = events.iter().map(|e| reference.apply(e)).collect();
    let state = reference.into_state();
    let store = root("prefix-restart");
    for (mode, count) in [("prefix", 1), ("suffix", 1)] {
        let result = child(&store, mode, count, None).output().unwrap();
        fs::write(store.join(format!("{mode}.stdout")), result.stdout).unwrap();
        fs::write(store.join(format!("{mode}.stderr")), result.stderr).unwrap();
        assert!(result.status.success(), "{mode}: {:?}", result.status);
    }
    let reopened = DurableKernel::open(&store, Native).unwrap();
    assert_eq!(reopened.snapshot().unwrap().state, state);
    assert_eq!(reopened.snapshot().unwrap().journal, journal);
    assert!(fs::read(store.join("kernel.state.r3b"))
        .unwrap()
        .starts_with(b"R3BIN\0\0\0"));
}

#[test]
#[ignore = "seven actual child-process SIGKILL points; no power-loss claim"]
fn seven_sigkill_boundaries_restore_old_or_new() {
    let b = fixture();
    let v = b
        .directed_vectors
        .iter()
        .find(|v| v.name == "two_valid_operations")
        .unwrap();
    let event = v.events.iter().find(|e| e.kind == "ATTEMPT").unwrap();
    let mut summary = String::new();
    for point in [
        "temp-open",
        "write",
        "file-sync",
        "close",
        "rename",
        "directory-sync",
        "reply",
    ] {
        let store = root(&format!("crash-{point}"));
        drop(DurableKernel::create(&store, v.initial_state_v1_2b.clone(), Native).unwrap());
        let mut process = child(&store, "crash", 1, Some(point)).spawn().unwrap();
        let start = Instant::now();
        while !store.join("reached").exists() {
            if let Some(status) = process.try_wait().unwrap() {
                panic!("child exited before {point}: {status}");
            }
            if start.elapsed() > Duration::from_secs(10) {
                process.kill().unwrap();
                process.wait().unwrap();
                panic!("timeout reaching {point}");
            }
            std::thread::sleep(Duration::from_millis(5));
        }
        process.kill().unwrap(); // Unix Child::kill sends SIGKILL.
        let output = process.wait_with_output().unwrap();
        use std::os::unix::process::ExitStatusExt;
        assert_eq!(output.status.signal(), Some(9));
        fs::write(store.join("child.stdout"), output.stdout).unwrap();
        fs::write(store.join("child.stderr"), output.stderr).unwrap();
        // The retry also runs in a new process, with no inherited kernel state.
        let result = child(&store, "retry", 1, None).output().unwrap();
        fs::write(store.join("retry.stdout"), result.stdout).unwrap();
        fs::write(store.join("retry.stderr"), result.stderr).unwrap();
        assert!(result.status.success(), "retry {point}");
        let kernel = DurableKernel::open(&store, Native).unwrap();
        let s = kernel.snapshot().unwrap();
        let expected = if ["rename", "directory-sync", "reply"].contains(&point) {
            "REPLAY"
        } else {
            "COMMIT"
        };
        assert_eq!(s.journal.last().unwrap().outcome, expected, "{point}");
        assert_eq!(s.state.operation_ledger.len(), 1);
        assert_eq!(s.state.used_nonces.len(), 1);
        assert_eq!(
            s.state.value,
            v.initial_state_v1_2b.value + event.env.as_ref().unwrap().effect.delta
        );
        summary.push_str(&format!(
            "{point}: signal=9 retry={expected} ledger=1 nonce=1 effect_once=true\n"
        ));
    }
    fs::write(root("crash-summary.txt"), summary).unwrap();
}

#[test]
#[ignore = "native malformed snapshot and save-failure boundaries"]
fn corruption_and_save_failures_fail_closed() {
    let b = fixture();
    let v = &b.directed_vectors[0];
    let event = &v.events[0];
    let store = root("failure-path");
    let mut k = DurableKernel::create(&store, v.initial_state_v1_2b.clone(), Native).unwrap();
    assert!(
        DurableKernel::open(&store, Native).is_err(),
        "single writer lock"
    );
    assert!(k
        .apply_with_hook(event, &mut |at| if at == "rename" {
            Err(io::Error::other("injected dirsync boundary failure"))
        } else {
            Ok(())
        })
        .is_err());
    assert!(k.snapshot().is_err());
    assert!(k.apply(event).is_err());
    drop(k);
    let mut k = DurableKernel::open(&store, Native).unwrap();
    assert_eq!(k.apply(event).unwrap().outcome, "REPLAY");
    let valid = k.snapshot().unwrap().clone();
    drop(k);
    let committed = store.join("kernel.state.r3b");
    let original = fs::read(&committed).unwrap();
    fs::write(store.join("newer.tmp"), &original).unwrap();
    for kind in [
        "truncated",
        "missing-ledger",
        "missing-nonce",
        "missing-receipt",
        "duplicate-nonce",
    ] {
        let mut bad = valid.clone();
        match kind {
            "missing-ledger" => bad.state.operation_ledger.clear(),
            "missing-nonce" => bad.state.used_nonces.clear(),
            "missing-receipt" => bad.journal[0].receipt = None,
            "duplicate-nonce" => bad.state.used_nonces.push(bad.state.used_nonces[0]),
            _ => (),
        }
        let bytes = if kind == "truncated" {
            original[..original.len() / 2].to_vec()
        } else {
            Native.encode(&bad).unwrap()
        };
        fs::write(&committed, bytes).unwrap();
        assert!(
            DurableKernel::open(&store, Native).is_err(),
            "{kind} must not promote a valid temp"
        );
    }
    fs::write(&committed, &original).unwrap();
    assert_eq!(
        DurableKernel::open(&store, Native)
            .unwrap()
            .snapshot()
            .unwrap(),
        &valid
    );
}
