use replica_v3::{
    archive::Archive,
    codec::{self, Compression},
    event::*,
    journal::Journal,
    retrieval::{GraphDirection, Search},
    store::Store,
};
use std::{fs, process::Command};

fn observation(id: i64, text: &[u8]) -> Event {
    let mut e = Event::observation("scope", "session", "synthetic", text.to_vec());
    e.id = id;
    e.recorded_at = id * 100;
    e
}
fn body(e: &Event) -> Vec<u8> {
    codec::encode(e, Compression::Raw).unwrap()
}
fn empty(root: &std::path::Path) -> (std::path::PathBuf, std::path::PathBuf) {
    let db = root.join("empty.db");
    let store = Store::init(&db).unwrap();
    let snapshot = root.join("snapshot.r3a");
    store.export_archive(&snapshot, Compression::Raw).unwrap();
    let path = root.join("store.r3j");
    Journal::create(&snapshot, &path, [7; 16]).unwrap();
    (snapshot, path)
}
#[test]
fn journal_exact_history_graph_restart_matches_sqlite_and_archive() {
    let d = tempfile::tempdir().unwrap();
    let db = d.path().join("source.db");
    let mut sql = Store::init(&db).unwrap();
    let snapshot = d.path().join("empty.r3a");
    sql.export_archive(&snapshot, Compression::Raw).unwrap();
    let slot = Slot {
        entity: "표지".into(),
        predicate: "방향".into(),
        context: "통로".into(),
    };
    let fact = |payload: &str, previous, restored_from| {
        let mut e =
            Event::observation("scope", "session", "synthetic", payload.as_bytes().to_vec());
        e.kind = Kind::Fact {
            slot: slot.clone(),
            previous,
            restored_from,
            valid_from: Some(10),
            valid_until: None,
        };
        e
    };
    let a = sql.append(fact("오른쪽\0 원문", None, None)).unwrap();
    std::thread::sleep(std::time::Duration::from_millis(2));
    let b = sql.append(fact("왼쪽 정정", Some(a.id), None)).unwrap();
    let mut retract = Event::observation("scope", "session", "synthetic", b"retract".to_vec());
    retract.kind = Kind::Retraction {
        slot: slot.clone(),
        previous: b.id,
        valid_from: Some(10),
        valid_until: None,
    };
    std::thread::sleep(std::time::Duration::from_millis(2));
    let c = sql.append(retract).unwrap();
    std::thread::sleep(std::time::Duration::from_millis(2));
    let restored = sql
        .append(fact("오른쪽\0 원문", Some(c.id), Some(a.id)))
        .unwrap();
    for _ in 0..2 {
        sql.append(Event::observation(
            "scope",
            "session",
            "same-user",
            b"same payload".to_vec(),
        ))
        .unwrap();
    }
    for relation in [
        RelationKind::Precedes,
        RelationKind::Supports,
        RelationKind::CausalHypothesis,
    ] {
        let mut e = Event::observation("scope", "session", "source", b"explicit relation".to_vec());
        e.kind = Kind::Relation {
            relation,
            from: 5,
            to: 6,
            evidence: vec![a.id],
        };
        sql.append(e).unwrap();
    }
    let canonical = d.path().join("full.r3a");
    sql.export_archive(&canonical, Compression::Raw).unwrap();
    let full = Archive::open(&canonical).unwrap();
    let bodies = (1..=9)
        .map(|id| full.canonical_body(id).unwrap())
        .collect::<Vec<_>>();
    let expected_text = [
        "오른쪽\0 원문",
        "왼쪽 정정",
        "retract",
        "오른쪽\0 원문",
        "same payload",
        "same payload",
        "explicit relation",
        "explicit relation",
        "explicit relation",
    ];
    for level in [None, Some(1), Some(3)] {
        let path = d.path().join(format!("{level:?}.r3j"));
        Journal::create(&snapshot, &path, [8; 16]).unwrap();
        let mut j = Journal::open(&snapshot, &path, true).unwrap();
        let commit = j.append([1; 16], bodies.clone(), level).unwrap();
        assert_eq!(j.append([1; 16], bodies.clone(), level).unwrap(), commit);
        let mut changed = bodies.clone();
        changed[0] = body(&observation(1, b"different"));
        assert!(j.append([1; 16], changed, level).is_err());
        assert!(Journal::open(&snapshot, &path, true).is_err());
        drop(j);
        let j = Journal::open(&snapshot, &path, false).unwrap();
        let view = j.view();
        assert!(j.tail().is_none());
        assert_eq!(view.event_count(), 9);
        for (i, text) in expected_text.iter().enumerate() {
            let e = view.get(i as i64 + 1).unwrap();
            assert_eq!(e.payload, text.as_bytes());
            assert_eq!(e.scope, "scope");
            assert_eq!(e.session, "session");
            assert_eq!(view.canonical_body(e.id).unwrap(), bodies[i]);
            assert_eq!(e, sql.get(e.id).unwrap());
        }
        assert_eq!(
            view.history("scope", &slot)
                .unwrap()
                .iter()
                .map(|e| e.id)
                .collect::<Vec<_>>(),
            vec![1, 2, 3, 4]
        );
        for (at, want) in [
            (a.recorded_at, Some(1)),
            (b.recorded_at, Some(2)),
            (c.recorded_at, None),
            (restored.recorded_at, Some(4)),
        ] {
            assert_eq!(
                view.current("scope", &slot, Some(at), 10)
                    .unwrap()
                    .map(|e| e.id),
                want
            );
        }
        assert!(view.current("scope", &slot, None, 9).unwrap().is_none());
        let mut q = Search::new("scope", "원문");
        q.history = true;
        q.valid_at = 10;
        for direction in [
            GraphDirection::Both,
            GraphDirection::Outgoing,
            GraphDirection::Incoming,
        ] {
            assert_eq!(
                view.directed_graph(&q, &[1, 5], direction).unwrap(),
                sql.directed_graph(&q, &[1, 5], direction).unwrap()
            );
            assert_eq!(
                view.directed_graph(&q, &[1, 5], direction).unwrap(),
                full.directed_graph(&q, &[1, 5], direction).unwrap()
            );
        }
        q.session = Some("hidden".into());
        assert!(
            view.directed_graph(&q, &[1], GraphDirection::Both)
                .unwrap()
                .items
                .is_empty()
        );
        assert_eq!(
            view.relation_inventory().collect::<Vec<_>>(),
            full.relation_inventory().collect::<Vec<_>>()
        );
    }
}
#[test]
fn journal_invalid_batch_never_commits_prefix() {
    let d = tempfile::tempdir().unwrap();
    let (snapshot, path) = empty(d.path());
    let mut j = Journal::open(&snapshot, &path, true).unwrap();
    let a = observation(1, b"same");
    j.append([1; 16], vec![body(&a)], None).unwrap();
    let before = fs::read(&path).unwrap();
    let b = observation(2, b"same");
    let mut edge = observation(3, b"bad relation");
    edge.kind = Kind::Relation {
        relation: RelationKind::Supports,
        from: 1,
        to: 99,
        evidence: vec![1],
    };
    assert!(
        j.append([2; 16], vec![body(&b), body(&edge)], None)
            .is_err()
    );
    assert_eq!(j.view().event_count(), 1);
    edge.kind = Kind::Relation {
        relation: RelationKind::Supports,
        from: 1,
        to: 2,
        evidence: vec![1],
    };
    edge.scope = "other".into();
    assert!(
        j.append([2; 16], vec![body(&b), body(&edge)], None)
            .is_err()
    );
    let slot = Slot {
        entity: "e".into(),
        predicate: "p".into(),
        context: "c".into(),
    };
    let mut wrong = observation(2, b"head");
    wrong.kind = Kind::Fact {
        slot,
        previous: Some(1),
        restored_from: None,
        valid_from: None,
        valid_until: None,
    };
    assert!(j.append([2; 16], vec![body(&wrong)], None).is_err());
    let mut duplicate = b.clone();
    duplicate.request_key = Some([9; 16]);
    let mut other = observation(3, b"same");
    other.request_key = duplicate.request_key;
    assert!(
        j.append([2; 16], vec![body(&duplicate), body(&other)], None)
            .is_err()
    );
    assert_eq!(fs::read(&path).unwrap(), before);
    j.append([3; 16], vec![body(&b)], None).unwrap();
    assert_eq!(j.view().event_count(), 2);
}
#[test]
fn journal_snapshot_profiles_preserve_large_single_event() {
    let d = tempfile::tempdir().unwrap();
    let mut s = Store::init(d.path().join("large.db")).unwrap();
    s.set_compression(Compression::Raw);
    let e = s
        .append(Event::observation("s", "t", "u", vec![b'x'; 200_000]))
        .unwrap();
    for block in [65536, 262144] {
        for level in [None, Some(1), Some(3)] {
            let p = d.path().join(format!("{block}-{level:?}"));
            let info = s.export_archive_profile(&p, block, level).unwrap();
            assert_eq!(info.blocks, 1);
            let a = Archive::open(&p).unwrap();
            assert_eq!(a.get(e.id).unwrap(), e);
            assert_eq!(
                a.canonical_body(e.id).unwrap(),
                codec::encode(&e, Compression::Raw).unwrap()
            );
        }
    }
    assert!(
        s.export_archive_profile(&d.path().join("invalid"), 4096, Some(1))
            .is_err()
    );
    assert!(
        s.export_archive_profile(&d.path().join("invalid"), 65536, Some(9))
            .is_err()
    );
}
#[test]
fn journal_tail_corruption_recovery_preserves_source_and_prefix() {
    let d = tempfile::tempdir().unwrap();
    let (snapshot, path) = empty(d.path());
    let mut j = Journal::open(&snapshot, &path, true).unwrap();
    let first = j
        .append([1; 16], vec![body(&observation(1, b"first"))], None)
        .unwrap();
    j.append([2; 16], vec![body(&observation(2, b"second"))], Some(3))
        .unwrap();
    drop(j);
    let source = fs::read(&path).unwrap();
    for (i, cut) in [
        first.end as usize + 1,
        first.end as usize + 127,
        first.end as usize + 130,
        source.len() - 1,
    ]
    .into_iter()
    .enumerate()
    {
        let p = d.path().join(format!("cut-{i}"));
        fs::write(&p, &source[..cut]).unwrap();
        let mut r = Journal::open(&snapshot, &p, false).unwrap();
        assert!(r.tail().unwrap().incomplete);
        assert_eq!(r.view().event_count(), 1);
        assert!(Journal::open(&snapshot, &p, true).is_err());
        let recovered = d.path().join(format!("recovered-{i}"));
        r.recover_to(&recovered).unwrap();
        drop(r);
        assert_eq!(fs::read(&p).unwrap(), source[..cut]);
        let mut fresh = Journal::open(&snapshot, &recovered, true).unwrap();
        assert!(fresh.tail().is_none());
        assert_eq!(fresh.view().get(1).unwrap().payload, b"first");
        fresh
            .append(
                [3; 16],
                vec![body(&observation(2, b"explicit recovery"))],
                None,
            )
            .unwrap();
        let header = fs::read(&recovered).unwrap();
        assert_ne!(&header[112..144], &[0; 32]);
        assert_eq!(
            u64::from_le_bytes(header[144..152].try_into().unwrap()),
            first.end
        );
    }
    let corrupt = d.path().join("middle");
    let mut bytes = source.clone();
    bytes[224 + 128 + 4 + 20] ^= 1;
    fs::write(&corrupt, &bytes).unwrap();
    let r = Journal::open(&snapshot, &corrupt, false).unwrap();
    assert!(!r.tail().unwrap().incomplete);
    assert_eq!(r.last().sequence, 0);
    assert_eq!(r.view().event_count(), 0);
    let repeated = d.path().join("duplicate-sequence");
    let mut bytes = source[..first.end as usize].to_vec();
    bytes.extend(&source[224..first.end as usize]);
    fs::write(&repeated, bytes).unwrap();
    let r = Journal::open(&snapshot, &repeated, false).unwrap();
    assert_eq!(r.last().sequence, 1);
    assert!(!r.tail().unwrap().incomplete);
    let short = d.path().join("short-header");
    fs::write(&short, &source[..223]).unwrap();
    assert!(Journal::open(&snapshot, &short, false).is_err());
    let mut sql = Store::init(d.path().join("other.db")).unwrap();
    sql.append(Event::observation("s", "t", "u", b"other".to_vec()))
        .unwrap();
    let other = d.path().join("other.r3a");
    sql.export_archive(&other, Compression::Raw).unwrap();
    assert!(Journal::open(&other, &path, false).is_err());
    assert_eq!(fs::read(&path).unwrap(), source);
}
#[cfg(feature = "test-support")]
#[test]
fn journal_process_sync_before_ack_restart_idempotency() {
    let d = tempfile::tempdir().unwrap();
    let (snapshot, path) = empty(d.path());
    let event = d.path().join("event.rpv3");
    fs::write(&event, body(&observation(1, "새 원문\0".as_bytes()))).unwrap();
    let invoke = || {
        let mut c = Command::new(env!("CARGO_BIN_EXE_replica-v3"));
        c.args([
            "journal",
            "--snapshot",
            snapshot.to_str().unwrap(),
            "--path",
            path.to_str().unwrap(),
            "append",
            "--request",
            "01010101010101010101010101010101",
            "--events",
            event.to_str().unwrap(),
        ]);
        c
    };
    let out = invoke()
        .env("R3JRN_TEST_CRASH", "after-sync")
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(91));
    assert!(out.stdout.is_empty());
    let durable = fs::read(&path).unwrap();
    let out = invoke().output().unwrap();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(out.stdout.starts_with(b"ACK"));
    assert_eq!(fs::read(&path).unwrap(), durable);
    let out = Command::new(env!("CARGO_BIN_EXE_replica-v3"))
        .args([
            "--db",
            d.path().join("nonexistent.db").to_str().unwrap(),
            "archive",
            "--path",
            snapshot.to_str().unwrap(),
            "--journal",
            path.to_str().unwrap(),
            "show",
            "1",
        ])
        .output()
        .unwrap();
    assert!(out.status.success());
    assert_eq!(out.stdout, "새 원문\0".as_bytes());
    assert!(!d.path().join("nonexistent.db").exists());
    let exported = Command::new(env!("CARGO_BIN_EXE_replica-v3"))
        .args([
            "archive",
            "--path",
            snapshot.to_str().unwrap(),
            "--journal",
            path.to_str().unwrap(),
            "show",
            "1",
            "--canonical",
        ])
        .output()
        .unwrap();
    assert!(exported.status.success());
    assert_eq!(exported.stdout, fs::read(&event).unwrap());
    let writer = Journal::open(&snapshot, &path, true).unwrap();
    assert!(!invoke().output().unwrap().status.success());
    drop(writer);
}
