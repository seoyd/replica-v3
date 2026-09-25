use crate::kernel::{Event, JournalEntry, State};
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

pub const FORMAT: &str = "replica.commit-kernel.reference-vectors";
pub const VERSION: &str = "1.2B";
pub const SUITE_ID: &str = "commit-kernel-production-boundary-v1.2B";
pub const VECTOR_COUNT: usize = 32;
pub const FROZEN_BUNDLE_SHA256: &str =
    "cd5f0533762fe68da3c3da01410c87a240a224db33350c52f3d4c339984c0323";
pub const REQUIRED_NAMES_SHA256: &str =
    "c200d5187070bb9484eca455b340c20283ea156b3be3603eb8134562cf936614";

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Bundle {
    pub format: String,
    pub version: String,
    pub suite_id: String,
    pub manifest: Manifest,
    pub directed_vectors: Vec<Vector>,
    // Metadata fields intentionally modeled so deny_unknown_fields can be used.
    pub date: String,
    pub profile: serde_json::Value,
    pub semantics_note: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Manifest {
    pub base_vector_count: usize,
    pub hardening_vector_count: usize,
    pub required_vector_names: Vec<String>,
    pub required_vector_names_sha256: String,
    pub source_v1_2a_sha256: String,
    pub vector_count: usize,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Vector {
    pub name: String,
    pub events: Vec<Event>,
    pub initial_state_v1_2b: State,
    pub expected_journal_v1_2b: Vec<JournalEntry>,
    pub expected_final_state_v1_2b: State,
    pub expected_state_sha256_v1_2b: String,
}

pub fn load_frozen_bundle(path: impl AsRef<Path>) -> Result<Bundle, String> {
    let raw = fs::read(path.as_ref()).map_err(|e| format!("read bundle: {e}"))?;
    let actual_sha = hex::encode(Sha256::digest(&raw));
    if actual_sha != FROZEN_BUNDLE_SHA256 {
        return Err(format!(
            "bundle sha mismatch: expected {FROZEN_BUNDLE_SHA256}, got {actual_sha}"
        ));
    }

    let bundle: Bundle = serde_json::from_slice(&raw).map_err(|e| format!("parse bundle: {e}"))?;
    preflight(&bundle)?;
    Ok(bundle)
}

pub fn preflight(bundle: &Bundle) -> Result<(), String> {
    if bundle.format != FORMAT {
        return Err(format!("format mismatch: {}", bundle.format));
    }
    if bundle.version != VERSION {
        return Err(format!("version mismatch: {}", bundle.version));
    }
    if bundle.suite_id != SUITE_ID {
        return Err(format!("suite mismatch: {}", bundle.suite_id));
    }
    if bundle.manifest.vector_count != VECTOR_COUNT || bundle.directed_vectors.len() != VECTOR_COUNT
    {
        return Err(format!(
            "vector count mismatch: manifest={}, actual={}",
            bundle.manifest.vector_count,
            bundle.directed_vectors.len()
        ));
    }
    if bundle.manifest.base_vector_count != 22 || bundle.manifest.hardening_vector_count != 10 {
        return Err("base + hardening count mismatch".to_string());
    }
    if bundle.manifest.required_vector_names_sha256 != REQUIRED_NAMES_SHA256 {
        return Err("required name manifest hash mismatch".to_string());
    }
    // Frozen manifest canonicalization: ordered UTF-8 names, each followed by LF.
    // Compare actual bytes, not an untrusted digest label or deduplicated set.
    let mut names = Sha256::new();
    for name in &bundle.manifest.required_vector_names {
        names.update(name.as_bytes());
        names.update(b"\n");
    }
    if bundle.manifest.required_vector_names.len() != VECTOR_COUNT
        || hex::encode(names.finalize()) != REQUIRED_NAMES_SHA256
    {
        return Err("actual required name manifest mismatch".to_string());
    }

    let expected: BTreeSet<_> = bundle
        .manifest
        .required_vector_names
        .iter()
        .cloned()
        .collect();
    let actual: BTreeSet<_> = bundle
        .directed_vectors
        .iter()
        .map(|v| v.name.clone())
        .collect();
    if expected.len() != VECTOR_COUNT || actual.len() != VECTOR_COUNT {
        return Err("duplicate or missing vector names".to_string());
    }
    if actual != expected {
        return Err("required vector names do not match actual suite".to_string());
    }
    Ok(())
}
