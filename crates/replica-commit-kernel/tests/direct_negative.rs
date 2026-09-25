use replica_commit_kernel::kernel::{effect_digest, Envelope, Event, Kernel, State};
use replica_commit_kernel::vectors::{load_frozen_bundle, preflight, Bundle};

fn bundle() -> Bundle {
    load_frozen_bundle("tests/data/commit_kernel_reference_vectors_v1_2B.json").unwrap()
}
fn sample() -> (State, Envelope) {
    let b = bundle();
    let v = &b.directed_vectors[0];
    (
        v.initial_state_v1_2b.clone(),
        v.events[0].env.clone().unwrap(),
    )
}
fn attempt(env: Envelope) -> Event {
    Event {
        kind: "ATTEMPT".into(),
        env: Some(env),
    }
}
fn reject_unchanged(state: State, event: Event, expected: &str) {
    let mut k = Kernel::new(state);
    let before = k.state().clone();
    let result = k.apply(&event);
    assert_eq!(result.outcome, expected);
    assert!(result.receipt.is_none());
    assert_eq!(
        k.state(),
        &before,
        "rejection mutated state, nonce or ledger"
    );
}

#[test]
fn effect_payload_tamper_is_not_a_checksum_only_test() {
    let (s, mut e) = sample();
    e.effect.delta += 1;
    reject_unchanged(s, attempt(e), "REJECT_EFFECT_DIGEST");
}
#[test]
fn registry_authorization_is_required() {
    let (mut s, e) = sample();
    s.authorized_capability_ids.clear();
    reject_unchanged(s, attempt(e), "REJECT_CAPABILITY");
    let (_, e) = sample();
    let mut input = serde_json::to_value(e).unwrap();
    input["scope_ok"] = true.into();
    assert!(serde_json::from_value::<Envelope>(input).is_err());
}
#[test]
fn signed_overflow_has_no_effect_or_receipt() {
    for (value, delta) in [(i64::MAX, 1), (i64::MIN, -1)] {
        let (mut s, mut e) = sample();
        s.value = value;
        e.effect.delta = delta;
        e.reviewed_effect_digest = effect_digest(&e.effect);
        reject_unchanged(s, attempt(e), "REJECT_OVERFLOW");
    }
}
#[test]
fn all_five_epoch_overflows_are_atomic() {
    for kind in [
        "PARENT_REUSE",
        "REVOKE",
        "POLICY_UPDATE",
        "OBJECT_CHANGE",
        "PREDICATE_CHANGE",
    ] {
        let (mut s, _) = sample();
        s.parent_active = false;
        match kind {
            "PARENT_REUSE" => s.parent_generation = u64::MAX,
            "REVOKE" => s.capability_epoch = u64::MAX,
            "POLICY_UPDATE" => s.policy_epoch = u64::MAX,
            "OBJECT_CHANGE" => s.object_version = u64::MAX,
            _ => s.predicate_version = u64::MAX,
        }
        reject_unchanged(
            s,
            Event {
                kind: kind.into(),
                env: None,
            },
            "REJECT_OVERFLOW",
        );
    }
}
#[test]
fn operation_replay_mismatch_and_nonce_are_distinct() {
    let (s, e) = sample();
    let mut k = Kernel::new(s);
    let committed = k.apply(&attempt(e.clone()));
    assert_eq!(committed.outcome, "COMMIT");
    let state = k.state().clone();
    assert_eq!(k.apply(&attempt(e.clone())).outcome, "REPLAY");
    assert_eq!(k.state(), &state);
    let mut changed = e.clone();
    changed.effect.delta += 1;
    changed.reviewed_effect_digest = effect_digest(&changed.effect);
    reject_unchanged(state.clone(), attempt(changed), "REJECT_OP_MISMATCH");
    let mut second = e;
    second.operation_id += 1;
    reject_unchanged(state, attempt(second), "REJECT_NONCE");
}
#[test]
fn parser_rejects_unknown_duplicate_fields_and_bad_integer_types() {
    let (_, e) = sample();
    let text = serde_json::to_string(&e).unwrap();
    let unknown = text.replacen('{', "{\"unknown\":1,", 1);
    let duplicate = text.replacen('{', "{\"nonce\":3,", 1);
    assert!(serde_json::from_str::<Envelope>(&unknown).is_err());
    assert!(serde_json::from_str::<Envelope>(&duplicate).is_err());
    for invalid in ["-1", "18446744073709551616", "1.5", "null"] {
        let text = text.replace("\"nonce\":11", &format!("\"nonce\":{invalid}"));
        assert!(
            serde_json::from_str::<Envelope>(&text).is_err(),
            "{invalid}"
        );
    }
}
#[test]
fn preflight_rejects_empty_duplicate_wrong_format_version_and_suite() {
    let mut b = bundle();
    b.directed_vectors.clear();
    assert!(preflight(&b).is_err());
    let mut b = bundle();
    b.directed_vectors[1].name = b.directed_vectors[0].name.clone();
    assert!(preflight(&b).is_err());
    for field in ["format", "version", "suite"] {
        let mut b = bundle();
        match field {
            "format" => b.format.push('x'),
            "version" => b.version.push('x'),
            _ => b.suite_id.push('x'),
        }
        assert!(preflight(&b).is_err());
    }
}
#[test]
fn preflight_recomputes_exact_manifest_instead_of_trusting_digest_label() {
    let mut b = bundle();
    let old = b.manifest.required_vector_names[0].clone();
    b.manifest.required_vector_names[0] = "unregistered_vector".into();
    b.directed_vectors
        .iter_mut()
        .find(|v| v.name == old)
        .unwrap()
        .name = "unregistered_vector".into();
    assert!(
        preflight(&b).is_err(),
        "matching renamed sets must not retain the frozen manifest identity"
    );
    let mut b = bundle();
    b.manifest
        .required_vector_names
        .push(b.manifest.required_vector_names[0].clone());
    assert!(
        preflight(&b).is_err(),
        "duplicate manifest entries must not disappear in a set"
    );
}

#[test]
fn event_schema_rejects_missing_null_extra_and_unknown_events() {
    for input in [
        r#"{"type":"ATTEMPT"}"#,
        r#"{"type":"ATTEMPT","env":null}"#,
        r#"{"type":"CANCEL","env":null}"#,
        r#"{"type":"UNREGISTERED"}"#,
    ] {
        assert!(serde_json::from_str::<Event>(input).is_err(), "{input}");
    }
    let (_, e) = sample();
    let extra = format!(
        r#"{{"type":"CANCEL","env":{}}}"#,
        serde_json::to_string(&e).unwrap()
    );
    assert!(serde_json::from_str::<Event>(&extra).is_err());
}

#[test]
fn public_apply_rejects_invalid_event_without_panic_or_mutation() {
    let (s, _) = sample();
    reject_unchanged(
        s.clone(),
        Event {
            kind: "ATTEMPT".into(),
            env: None,
        },
        "REJECT_EVENT",
    );
    reject_unchanged(
        s,
        Event {
            kind: "UNKNOWN".into(),
            env: None,
        },
        "REJECT_EVENT",
    );
}
