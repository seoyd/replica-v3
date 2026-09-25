use replica_commit_kernel::{run_vector, vectors::load_frozen_bundle};

#[test]
fn frozen_v1_2b_vectors_match_exactly() {
    let bundle = load_frozen_bundle("tests/data/commit_kernel_reference_vectors_v1_2B.json")
        .expect("frozen bundle must pass preflight");
    assert_eq!(bundle.directed_vectors.len(), 32);

    for vector in &bundle.directed_vectors {
        let got = run_vector(vector);
        assert_eq!(
            got.journal, vector.expected_journal_v1_2b,
            "journal mismatch: {}",
            vector.name
        );
        assert_eq!(
            got.final_state, vector.expected_final_state_v1_2b,
            "state mismatch: {}",
            vector.name
        );
        assert_eq!(
            got.state_sha256, vector.expected_state_sha256_v1_2b,
            "hash mismatch: {}",
            vector.name
        );
    }
}
