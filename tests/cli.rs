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
fn native_generate_and_chat_eof_quit_cancel_have_bounded_side_effects() {
    use replica_v3::store::Store;
    use std::io::Read;
    let dir = tempfile::tempdir().unwrap();
    let absent_db = dir.path().join("unused-db");
    let out = cli(
        &absent_db,
        &[
            "generate",
            "--checkpoint",
            "absent-native",
            "--text",
            "question",
        ],
        b"",
    );
    assert!(!out.status.success());
    assert!(out.stdout.is_empty());
    assert!(String::from_utf8_lossy(&out.stderr).contains("MISSING_NATIVE_CHECKPOINT"));
    assert!(!absent_db.exists());
    let path = dir.path().join("memory");
    Store::init(&path).unwrap();
    let args = ["chat", "--scope", "s", "--checkpoint", "absent-native"];
    for input in [b"".as_slice(), b"/quit\n"] {
        let out = cli(&path, &args, input);
        assert!(
            out.status.success(),
            "{}",
            String::from_utf8_lossy(&out.stderr)
        );
        assert!(out.stdout.is_empty());
        assert_eq!(Store::open(&path).unwrap().count().unwrap(), 0);
    }
    let mut child = Command::new(env!("CARGO_BIN_EXE_replica-v3"))
        .arg("--db")
        .arg(&path)
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let mut prompt = [0u8; 2];
    child
        .stderr
        .as_mut()
        .unwrap()
        .read_exact(&mut prompt)
        .unwrap();
    assert_eq!(&prompt, b"> "); // Handler is installed and foreground awaits input.
    assert!(
        Command::new("kill")
            .args(["-INT", &child.id().to_string()])
            .status()
            .unwrap()
            .success()
    );
    let deadline = Instant::now() + Duration::from_secs(3);
    while child.try_wait().unwrap().is_none() {
        if Instant::now() >= deadline {
            child.kill().unwrap();
            child.wait().unwrap();
            panic!("chat cancellation blocked on stdin");
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    let out = child.wait_with_output().unwrap();
    assert!(!out.status.success());
    assert!(out.stdout.is_empty());
    assert!(String::from_utf8_lossy(&out.stderr).contains("cancelled"));
    assert_eq!(Store::open(&path).unwrap().count().unwrap(), 0);
    let original = "  새 질문 😀\r".as_bytes();
    for _ in 0..2 {
        let out = cli(&path, &args, original);
        assert!(!out.status.success());
        assert!(out.stdout.is_empty());
    }
    let store = Store::open(path).unwrap();
    assert_eq!(store.count().unwrap(), 4); // Each fresh input plus its recorded setup failure.
    let first = store.get(1).unwrap();
    let second = store.get(3).unwrap();
    assert_eq!(first.payload, original);
    assert_eq!(second.payload, original);
    assert!(first.request_key.is_some());
    assert_ne!(first.request_key, second.request_key);
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

#[test]
fn rv01_corrupt_result_has_no_success_stdout_or_write() {
    use replica_v3::{event::*, store::Store};
    let d = tempfile::tempdir().unwrap();
    let p = d.path().join("db");
    let mut s = Store::init(&p).unwrap();
    let mut q = Event::observation("s", "default", "user", b"question".to_vec());
    q.kind = Kind::Observation {
        question: Some(GenerationLimits::default()),
    };
    q.request_key = Some([1; 16]);
    let q = s.append(q).unwrap();
    let obs = s
        .append(Event::observation(
            "s",
            "default",
            "user",
            b"wrong".to_vec(),
        ))
        .unwrap();
    let conn = rusqlite::Connection::open(&p).unwrap();
    conn.execute(
        "INSERT INTO results VALUES(?1,?2)",
        rusqlite::params![q.id, obs.id],
    )
    .unwrap();
    let out = cli(
        &p,
        &[
            "ask",
            "--scope",
            "s",
            "--request-key",
            "01010101010101010101010101010101",
            "--checkpoint",
            "absent",
            "--text",
            "question",
        ],
        b"",
    );
    assert!(!out.status.success());
    assert!(out.stdout.is_empty());
    assert!(String::from_utf8_lossy(&out.stderr).contains("corruption"));
    assert_eq!(s.count().unwrap(), 2);
}
