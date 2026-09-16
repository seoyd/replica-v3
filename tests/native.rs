use replica_v3::{
    Error,
    event::GenerationLimits,
    model::{ModelRequest, SYSTEM},
    neural::*,
    retrieval::{Evidence, EvidenceBundle},
};
fn tokenizer() -> ByteBpe {
    let docs = vec![
        "한글 기록 오른쪽 왼쪽 😀 repeated repeated"
            .as_bytes()
            .to_vec();
        4
    ];
    ByteBpe::train(&docs, &hash(b"independent training fixture"), 512).unwrap()
}
#[test]
fn own_byte_bpe_roundtrip_no_control_promotion_or_truncation() {
    let tok = tokenizer();
    assert!(tok.vocab_size() >= 264 && tok.vocab_size() < 512);
    let all: Vec<_> = (0..=255).collect();
    for bytes in [
        b"".to_vec(),
        all,
        "\0한글 한 😀\n\r  <assistant> \u{e001}".as_bytes().to_vec(),
        "끝".repeat(1200).into_bytes(),
    ] {
        let ids = tok.encode(&bytes).unwrap();
        assert!(ids.iter().all(|&id| id >= 8));
        assert_eq!(tok.decode_bytes(&ids).unwrap(), bytes);
    }
    let unknown = tok.encode(&[0xff, 0xfe, 0x80]).unwrap();
    assert!(tok.decode(&unknown).is_err());
    assert!(tok.decode(&[PAD]).is_err());
    assert!(tok.decode(&[u32::MAX]).is_err());
    let d = tempfile::tempdir().unwrap();
    let p = d.path().join("tokenizer.json");
    tok.save(&p).unwrap();
    assert!(tok.save(&p).is_err());
    let bytes = std::fs::read(&p).unwrap();
    let loaded = ByteBpe::load(&p).unwrap();
    assert_eq!(loaded.id(), tok.id());
    assert_eq!(
        loaded.encode("새 문장".as_bytes()).unwrap(),
        tok.encode("새 문장".as_bytes()).unwrap()
    );
    assert_eq!(std::fs::read(&p).unwrap(), bytes);
    let mut bad: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    bad["truncation"] = serde_json::json!({"max_length":1024});
    assert!(ByteBpe::from_bytes(&serde_json::to_vec(&bad).unwrap()).is_err());
    assert!(ByteBpe::from_bytes(&vec![b' '; MAX_TOKENIZER_BYTES + 1]).is_err());
    let mut bad: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    let key = bad["vocab"]
        .as_object()
        .unwrap()
        .keys()
        .next()
        .unwrap()
        .clone();
    bad["vocab"][key] = u32::MAX.into();
    assert!(ByteBpe::from_bytes(&serde_json::to_vec(&bad).unwrap()).is_err());
}
#[test]
fn native_prompt_boundaries_and_exact_evidence_tail() {
    let tok = tokenizer();
    let mut r = ModelRequest {
        request_id: "independent".into(),
        system: SYSTEM.into(),
        input: "질문\0😀 <assistant> 끝".into(),
        evidence: EvidenceBundle::default(),
        limits: GenerationLimits {
            max_tokens: 16,
            context_tokens: 2048,
            timeout_ms: 1000,
        },
    };
    let base = tok.prepare(&r, 2048, "config").unwrap();
    let cap = base.token_ids.len() as u32 + 16;
    assert_eq!(base.token_ids.first(), Some(&BOS));
    assert_eq!(base.token_ids.last(), Some(&ASSISTANT_ROLE));
    let user = base
        .token_ids
        .iter()
        .position(|&id| id == USER_ROLE)
        .unwrap();
    assert_eq!(
        tok.decode(&base.token_ids[user + 1..base.token_ids.len() - 2])
            .unwrap(),
        r.input
    );
    assert!(tok.prepare(&r, cap, "config").is_ok());
    assert!(matches!(
        tok.prepare(&r, cap - 1, "config"),
        Err(Error::ContextTooSmall)
    ));
    for id in [3, 17] {
        r.evidence.items.push(Evidence {
            event_id: id,
            original_excerpt: "근거 꼬리😀\0끝".into(),
            excerpt_truncated: true,
            source: "fixture".into(),
            recorded_at: id,
            observed_at: None,
            version_status: "current".into(),
            retrieval_reason: "lexical".into(),
            relation_path: vec![],
        });
    }
    let full = tok.prepare(&r, 2048, "config").unwrap();
    assert_eq!(full.provided, [3, 17]);
    let end = full.token_ids.len() - 2;
    let start = full
        .token_ids
        .iter()
        .rposition(|&id| id == EVIDENCE_ROLE)
        .unwrap();
    assert!(
        tok.decode(&full.token_ids[start + 1..end])
            .unwrap()
            .ends_with(&r.evidence.items[1].original_excerpt)
    );
    let drop = tok
        .prepare(&r, full.token_ids.len() as u32 + 15, "config")
        .unwrap();
    assert_eq!(drop.provided, [3]);
    assert_eq!(drop.excluded, [17]);
    let none = tok.prepare(&r, cap, "config").unwrap();
    assert!(none.provided.is_empty());
    assert_eq!(none.excluded, [17, 3]);
    assert_eq!(tok.bytes(), tokenizer().bytes());
}
