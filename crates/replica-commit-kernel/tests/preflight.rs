use replica_commit_kernel::vectors::{load_frozen_bundle, FROZEN_BUNDLE_SHA256, VECTOR_COUNT};
use sha2::{Digest, Sha256};
use std::fs;

#[test]
fn frozen_bundle_identity_is_pinned() {
    let path = "tests/data/commit_kernel_reference_vectors_v1_2B.json";
    let raw = fs::read(path).expect("bundle readable");
    assert_eq!(hex::encode(Sha256::digest(&raw)), FROZEN_BUNDLE_SHA256);
    let bundle = load_frozen_bundle(path).expect("preflight passes");
    assert_eq!(bundle.directed_vectors.len(), VECTOR_COUNT);
}
