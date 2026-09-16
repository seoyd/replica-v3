use std::{
    io::Write,
    path::Path,
    process::{Command, Stdio},
    time::{Duration, Instant},
};
fn cli(path: &Path, args: &[&str], stdin: &[u8]) -> std::process::Output {
    let mut child = Command::new(env!("CARGO_BIN_EXE_replica-v3"))
        .arg("--db")
        .arg(path)
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child.stdin.take().unwrap().write_all(stdin).unwrap();
    child.wait_with_output().unwrap()
}
#[test]
fn independent_process_restart_bytes_ids_and_request_replay() {
    let d = tempfile::tempdir().unwrap();
    let p = d.path().join("db");
    assert!(cli(&p, &["init"], b"").status.success());
    let original = "\0\n 한글 한 😀 \"\n ".as_bytes();
    let args = [
        "record",
        "--scope",
        "scope",
        "--request-key",
        "00000000000000000000000000000001",
    ];
    let saved = cli(&p, &args, original);
    assert!(
        saved.status.success(),
        "{}",
        String::from_utf8_lossy(&saved.stderr)
    );
    let id = String::from_utf8(saved.stdout.clone()).unwrap();
    assert_eq!(cli(&p, &["show", id.trim()], b"").stdout, original);
    assert_eq!(cli(&p, &args, original).stdout, saved.stdout);
    let different = cli(&p, &["record", "--scope", "scope"], original);
    assert!(different.status.success());
    assert_ne!(different.stdout, saved.stdout);
    assert!(!cli(&p, &args, b"different").status.success());
    assert!(cli(&p, &["doctor", "--full"], b"").status.success());
    let absent = d.path().join("missing");
    assert!(!cli(&absent, &["show", "1"], b"").status.success());
    assert!(!absent.exists());
}
#[cfg(feature = "test-support")]
#[test]
fn kill_before_after_commit_and_before_ack() {
    use replica_v3::store::Store;
    for point in ["before_commit", "after_commit", "before_display"] {
        let d = tempfile::tempdir().unwrap();
        let p = d.path().join("db");
        Store::init(&p).unwrap();
        let marker = d.path().join("marker");
        let args = [
            "record",
            "--scope",
            "scope",
            "--request-key",
            "00000000000000000000000000000001",
            "--text",
            "durable",
        ];
        let mut child = Command::new(env!("CARGO_BIN_EXE_replica-v3"))
            .arg("--db")
            .arg(&p)
            .args(args)
            .env("REPLICA_TEST_PAUSE", point)
            .env("REPLICA_TEST_MARKER", &marker)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        let deadline = Instant::now() + Duration::from_secs(5);
        while !marker.exists() {
            if let Some(status) = child.try_wait().unwrap() {
                panic!("child exited {status}");
            }
            assert!(Instant::now() < deadline, "hook timeout");
            std::thread::sleep(Duration::from_millis(5));
        }
        child.kill().unwrap();
        let out = child.wait_with_output().unwrap();
        assert!(!out.status.success());
        assert!(out.stdout.is_empty());
        let s = Store::open(&p).unwrap();
        assert_eq!(
            s.count().unwrap(),
            if point == "before_commit" { 0 } else { 1 }
        );
        s.doctor(true).unwrap();
        drop(s);
        let ack = cli(&p, &args, b"");
        assert!(ack.status.success());
        let s = Store::open(&p).unwrap();
        assert_eq!(s.count().unwrap(), 1);
        assert_eq!(s.get(1).unwrap().payload, b"durable");
    }
}
