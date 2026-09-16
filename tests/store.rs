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
