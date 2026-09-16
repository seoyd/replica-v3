use replica_v3::{Error, event::*, retrieval::Search, store::Store};
#[test]
fn independent_ten_thousand_event_fixture_and_graph_comparison() {
    let d = tempfile::tempdir().unwrap();
    let mut s = Store::init(d.path().join("db")).unwrap();
    let target = s
        .append(Event::observation(
            "factory",
            "incident",
            "operator",
            "오른쪽으로 가".as_bytes().to_vec(),
        ))
        .unwrap();
    let mut events = Vec::new();
    for i in 0..10_000 {
        let scope = if i % 7 == 0 { "private" } else { "factory" };
        let direction = ["LEFT", "RIGHT", "직진", "취소"][i % 4];
        let text = format!("보라터널 공용 경로 {direction} 반복 관측 {i}");
        events.push(Event::observation(
            scope,
            &format!("shift-{}", i % 13),
            "fixture",
            text.into_bytes(),
        ));
    }
    s.import(events).unwrap();
    let mut broad = Search::new("factory", "보라터널");
    broad.graph = false;
    let bounded = s.search(&broad).unwrap();
    assert!(!bounded.items.is_empty());
    assert!(bounded.truncated);
    let accident = s
        .append(Event::observation(
            "factory",
            "incident",
            "sensor",
            "보라터널 사고 당시 이동 기록".as_bytes().to_vec(),
        ))
        .unwrap();
    let mut relation = Event::observation(
        "factory",
        "incident",
        "user",
        "기록 시점상 지시가 사고보다 앞섰음. 원인은 미확인."
            .as_bytes()
            .to_vec(),
    );
    relation.kind = Kind::Relation {
        relation: RelationKind::Precedes,
        from: target.id,
        to: accident.id,
        evidence: vec![target.id, accident.id],
    };
    s.append(relation).unwrap();
    // The oracle IDs stay here, never passed as retrieval seeds or query hints.
    let expected = [target.id, accident.id];
    let mut q = Search::new("factory", "보라터널 사고 당시 이동 지시");
    q.session = Some("incident".into());
    q.graph = false;
    let lexical = s.search(&q).unwrap();
    q.graph = true;
    let graph = s.search(&q).unwrap();
    assert!(lexical.items.iter().any(|e| e.event_id == accident.id));
    assert!(!lexical.items.iter().any(|e| e.event_id == target.id));
    for id in expected {
        assert!(graph.items.iter().any(|e| e.event_id == id), "missing {id}");
    }
    assert!(graph.items.len() <= 8 && graph.visited <= 256);
    assert!(
        graph
            .items
            .iter()
            .flat_map(|e| &e.relation_path)
            .all(|step| step.relation == RelationKind::Precedes)
    );
    println!(
        "T06 canonical_count={} top_k=8 lexical_recall={}/2 graph_recall={}/2 stored_precedes_only=true semantic_causality_model=NOT_RUN",
        s.count().unwrap(),
        lexical
            .items
            .iter()
            .filter(|e| expected.contains(&e.event_id))
            .count(),
        graph
            .items
            .iter()
            .filter(|e| expected.contains(&e.event_id))
            .count()
    );
}
#[test]
fn short_query_scope_sql_syntax_and_cyclic_budget() {
    let d = tempfile::tempdir().unwrap();
    let mut s = Store::init(d.path().join("db")).unwrap();
    let events = (0..300)
        .map(|i| {
            Event::observation(
                "scope",
                "s",
                "user",
                format!("방 {i} 무한순환 검색").into_bytes(),
            )
        })
        .collect();
    let events = s.import(events).unwrap();
    assert!(matches!(
        s.search(&Search::new("scope", "방")),
        Err(Error::NarrowScope)
    ));
    let mut relations = Vec::new();
    for pair in events.windows(2) {
        let mut e = Event::observation("scope", "s", "user", b"edge".to_vec());
        e.kind = Kind::Relation {
            relation: RelationKind::Supports,
            from: pair[0].id,
            to: pair[1].id,
            evidence: vec![pair[0].id],
        };
        relations.push(e);
    }
    let mut e = Event::observation("scope", "s", "user", b"cycle".to_vec());
    e.kind = Kind::Relation {
        relation: RelationKind::Supports,
        from: events[299].id,
        to: events[0].id,
        evidence: vec![events[0].id],
    };
    relations.push(e);
    s.import(relations).unwrap();
    let r = s.search(&Search::new("scope", "무한순환")).unwrap();
    assert!(r.truncated);
    assert!(r.visited <= 256);
    assert!(r.items.len() <= 8);
    assert!(r.items.iter().all(|e| e.relation_path.len() <= 4));
    for query in [
        "\" OR 1=1; DROP TABLE records; --",
        "$(touch /tmp/never-execute)",
        "***\"",
        "AND OR NOT",
    ] {
        s.search(&Search::new("scope", query)).unwrap();
    }
    assert_eq!(s.count().unwrap(), 600);
    let e = s
        .append(Event::observation(
            "private",
            "s",
            "user",
            "전용정보".as_bytes().to_vec(),
        ))
        .unwrap();
    let r = s.search(&Search::new("scope", "전용정보")).unwrap();
    assert!(r.items.is_empty());
    let mut bad = Event::observation("scope", "s", "user", b"cross scope".to_vec());
    bad.kind = Kind::Relation {
        relation: RelationKind::Supports,
        from: events[0].id,
        to: e.id,
        evidence: vec![events[0].id],
    };
    assert!(s.append(bad).is_err());
    let mut q = Search::new("private", "전용");
    assert_eq!(s.search(&q).unwrap().items[0].event_id, e.id);
    q.query = "\0".into();
    assert!(s.search(&q).is_err());
}
