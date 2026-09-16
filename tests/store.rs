use replica_v3::{Error, event::*, retrieval::Search, store::Store};
use rusqlite::Connection;
fn fact(value: &str, context: &str, previous: Option<i64>) -> Event {
    let mut e = Event::observation("scope", "session", "user", value.as_bytes().to_vec());
    e.kind = Kind::Fact {
        slot: Slot {
            entity: "robot".into(),
            predicate: "direction".into(),
            context: context.into(),
        },
        previous,
        restored_from: None,
        valid_from: Some(100),
        valid_until: None,
    };
    e
}
#[test]
fn lifecycle_restart_idempotency_and_late_arrival() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("memory.db");
    let mut s = Store::init(&path).unwrap();
    assert!(Store::init(&path).is_err());
    let mut raw = Event::observation("scope", "session", "user", b"RIGHT\0\n".to_vec());
    raw.observed_at = Some(900);
    raw.request_key = Some([1; 16]);
    let observation = s.append(raw.clone()).unwrap();
    assert_eq!(s.append(raw.clone()).unwrap(), observation);
    raw.payload = b"wrong".to_vec();
    assert!(matches!(s.append(raw), Err(Error::Conflict(_))));
    let first = s.append(fact("RIGHT", "hall", None)).unwrap();
    std::thread::sleep(std::time::Duration::from_millis(2));
    let mut correction = fact("LEFT", "hall", Some(first.id));
    correction.observed_at = Some(100);
    let second = s.append(correction).unwrap();
    assert!(second.observed_at < observation.observed_at);
    let third = s
        .restore_fact(fact("", "hall", Some(second.id)), first.id)
        .unwrap();
    assert_eq!(third.payload, b"RIGHT");
    let slot = first.kind.slot().unwrap();
    assert_eq!(
        s.current("scope", slot, None, 100).unwrap(),
        Some(third.clone())
    );
    assert_eq!(
        s.current("scope", slot, Some(first.recorded_at), 100)
            .unwrap(),
        Some(first.clone())
    );
    assert!(s.current("scope", slot, None, 99).unwrap().is_none());
    s.append(fact("LEFT", "other", None)).unwrap();
    assert_eq!(s.head("scope", slot).unwrap().unwrap().id, third.id);
    assert!(s.append(fact("bad", "hall", Some(first.id))).is_err());
    assert!(
        s.restore_fact(fact("", "other", Some(third.id + 1)), first.id)
            .is_err()
    );
    assert!(
        s.restore_fact(fact("", "hall", Some(third.id)), 9999)
            .is_err()
    );
    assert_eq!(
        s.history("scope", slot).unwrap(),
        vec![first.clone(), second, third.clone()]
    );
    drop(s);
    let s = Store::open(&path).unwrap();
    assert_eq!(s.get(observation.id).unwrap(), observation);
    assert_eq!(s.get(third.id).unwrap(), third);
    s.doctor(true).unwrap();
    let conn = Connection::open(&path).unwrap();
    assert!(
        conn.execute("UPDATE records SET body=x'00' WHERE id=1", [])
            .is_err()
    );
    assert!(conn.execute("DELETE FROM records", []).is_err());
}
#[test]
fn concurrent_correction_has_one_winner() {
    let d = tempfile::tempdir().unwrap();
    let p = d.path().join("db");
    let mut s = Store::init(&p).unwrap();
    let first = s.append(fact("RIGHT", "c", None)).unwrap();
    drop(s);
    let barrier = std::sync::Arc::new(std::sync::Barrier::new(2));
    let handles: Vec<_> = (0..2)
        .map(|n| {
            let p = p.clone();
            let barrier = barrier.clone();
            std::thread::spawn(move || {
                let mut s = Store::open(p).unwrap();
                barrier.wait();
                s.append(fact(&format!("value{n}"), "c", Some(first.id)))
            })
        })
        .collect();
    let results: Vec<_> = handles.into_iter().map(|h| h.join().unwrap()).collect();
    assert_eq!(results.iter().filter(|r| r.is_ok()).count(), 1);
    assert_eq!(
        results
            .iter()
            .filter(|r| matches!(r, Err(Error::Conflict(_))))
            .count(),
        1
    );
}
#[test]
fn rebuild_open_wal_backup_and_corruption() {
    let d = tempfile::tempdir().unwrap();
    let p = d.path().join("db");
    let mut s = Store::init(&p).unwrap();
    let e = s.append(fact("오른쪽 경로", "c", None)).unwrap();
    let q = Search::new("scope", "오른쪽");
    let before = s.search(&q).unwrap();
    let backup = d.path().join("backup");
    s.backup(&backup).unwrap();
    assert!(s.backup(&backup).is_err());
    let restored = d.path().join("restored");
    Store::restore(&backup, &restored).unwrap();
    assert_eq!(Store::open(restored).unwrap().get(e.id).unwrap(), e);
    let conn = Connection::open(&p).unwrap();
    conn.execute_batch("DROP TABLE record_fts; DROP TABLE record_meta; DROP TABLE current_heads; DROP TABLE relations; DROP TABLE request_state; DROP TABLE results; DROP TABLE projection_state;").unwrap();
    assert!(Store::open(&p).is_err());
    drop(s);
    let mut s = Store::open_for_maintenance(&p, true).unwrap();
    s.reindex().unwrap();
    assert_eq!(s.get(e.id).unwrap(), e);
    assert_eq!(
        s.search(&q).unwrap().items[0].event_id,
        before.items[0].event_id
    );
    s.doctor(true).unwrap();
    conn.execute(
        "UPDATE record_meta SET source='tampered' WHERE id=?1",
        [e.id],
    )
    .unwrap();
    assert!(matches!(s.get(e.id), Err(Error::Corrupt(_))));
    s.reindex().unwrap();
    conn.execute_batch(
        "DROP TRIGGER immutable_update; UPDATE records SET body=x'0000' WHERE id=1;",
    )
    .unwrap();
    assert!(s.doctor(true).is_err());
    assert!(s.reindex().is_err());
}

#[cfg(feature = "test-support")]
#[test]
fn rv04_atomic_startup_and_head_with_writer_and_corrupt_control() {
    use replica_v3::store::with_sync_hook;
    for point in ["startup_observed", "head_observed"] {
        let d = tempfile::tempdir().unwrap();
        let p = d.path().join("db");
        let mut s = Store::init(&p).unwrap();
        let first = s.append(fact("RIGHT", "c", None)).unwrap();
        let slot = first.kind.slot().unwrap().clone();
        let (go, receive) = std::sync::mpsc::channel();
        let (done, wait) = std::sync::mpsc::channel();
        let mut writer = Store::open(&p).unwrap();
        let h = std::thread::spawn(move || {
            receive.recv().unwrap();
            writer.append(fact("LEFT", "c", Some(first.id))).unwrap();
            done.send(()).unwrap();
        });
        let mut fired = false;
        with_sync_hook(
            move |at| {
                if at == point && !fired {
                    fired = true;
                    go.send(()).unwrap();
                    wait.recv().unwrap();
                }
                Ok(())
            },
            || {
                if point == "startup_observed" {
                    Store::open(&p).unwrap();
                } else {
                    assert_eq!(s.head("scope", &slot).unwrap().unwrap().payload, b"RIGHT");
                }
            },
        );
        h.join().unwrap();
        assert_eq!(s.head("scope", &slot).unwrap().unwrap().payload, b"LEFT");
        let conn = Connection::open(&p).unwrap();
        conn.execute("UPDATE projection_state SET last_id=0", [])
            .unwrap();
        assert!(matches!(Store::open(&p), Err(Error::Corrupt(_))));
        s.reindex().unwrap();
        conn.execute("UPDATE current_heads SET event_id=?1", [first.id])
            .unwrap();
        assert!(matches!(s.head("scope", &slot), Err(Error::Corrupt(_))));
    }
}

#[cfg(feature = "test-support")]
#[test]
fn rv05_pinned_backup_before_copy_and_after_done() {
    use replica_v3::store::with_sync_hook;
    for point in ["backup_snapshot", "backup_done"] {
        let d = tempfile::tempdir().unwrap();
        let p = d.path().join("db");
        let mut s = Store::init(&p).unwrap();
        let first = s.append(fact("RIGHT\0\n", "c", None)).unwrap();
        let (go, receive) = std::sync::mpsc::channel();
        let (done, wait) = std::sync::mpsc::channel();
        let mut writer = Store::open(&p).unwrap();
        let h = std::thread::spawn(move || {
            receive.recv().unwrap();
            writer
                .append(Event::observation(
                    "scope",
                    "session",
                    "user",
                    b"later".to_vec(),
                ))
                .unwrap();
            done.send(()).unwrap();
        });
        let b = d.path().join("backup");
        with_sync_hook(
            move |at| {
                if at == point {
                    go.send(()).unwrap();
                    wait.recv().unwrap();
                }
                Ok(())
            },
            || s.backup(&b),
        )
        .unwrap();
        h.join().unwrap();
        let backup = Store::open(&b).unwrap();
        backup.doctor(true).unwrap();
        assert_eq!(backup.count().unwrap(), 1);
        assert_eq!(backup.get(first.id).unwrap(), first);
        assert_eq!(s.count().unwrap(), 2);
        let restored = d.path().join("restored");
        Store::restore(&b, &restored).unwrap();
        assert_eq!(
            Store::open(&restored).unwrap().get(first.id).unwrap(),
            first
        );
        assert!(s.backup(&b).is_err());
        assert_eq!(backup.count().unwrap(), 1);
    }
}

#[cfg(feature = "test-support")]
#[test]
fn rv05_failed_artifact_is_explicit_and_existing_destination_preserved() {
    use replica_v3::store::with_sync_hook;
    let d = tempfile::tempdir().unwrap();
    let p = d.path().join("db");
    let mut s = Store::init(&p).unwrap();
    s.append(fact("bytes", "c", None)).unwrap();
    let existing = d.path().join("existing");
    std::fs::write(&existing, b"sentinel").unwrap();
    assert!(s.backup(&existing).is_err());
    assert_eq!(std::fs::read(&existing).unwrap(), b"sentinel");
    for point in ["backup_step", "backup_done"] {
        let target = d.path().join(point);
        let error = with_sync_hook(
            move |at| {
                if at == point {
                    Err(Error::Invalid("injected backup failure".into()))
                } else {
                    Ok(())
                }
            },
            || s.backup(&target),
        )
        .unwrap_err();
        assert!(error.to_string().contains("untrusted destination retained"));
        assert!(error.to_string().contains(target.to_str().unwrap()));
        assert!(target.exists());
        assert!(s.backup(&target).is_err());
    }
    Connection::open(&p)
        .unwrap()
        .execute_batch("UPDATE record_meta SET source='corrupt'")
        .unwrap();
    let target = d.path().join("corrupt-backup");
    assert!(
        s.backup(&target)
            .unwrap_err()
            .to_string()
            .contains("untrusted destination retained")
    );
    assert!(target.exists());
}
