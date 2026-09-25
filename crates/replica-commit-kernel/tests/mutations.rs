//! Small semantic mutation gate. The real source and frozen fixture are read-only.
//! One scratch crate and shared build target are reused sequentially.
use std::{fs, path::PathBuf, process::Command};

#[test]
#[ignore = "explicit scratch path required; compiles eleven semantic guard mutants"]
fn guards_are_killed_by_frozen_vectors() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let scratch = PathBuf::from(
        std::env::var_os("R3_KERNEL_MUTATION_ROOT").expect("explicit new scratch root"),
    );
    fs::create_dir(&scratch).expect("scratch must not exist");
    fs::create_dir(scratch.join("src")).unwrap();
    for path in [
        "Cargo.toml",
        "Cargo.lock",
        "src/lib.rs",
        "src/vectors.rs",
        "src/main.rs",
    ] {
        fs::copy(root.join(path), scratch.join(path)).unwrap();
    }
    let source = fs::read_to_string(root.join("src/kernel.rs")).unwrap();
    let fixture = root.join("tests/data/commit_kernel_reference_vectors_v1_2B.json");
    let mutations = [
        (
            "effect_digest",
            "env.reviewed_effect_digest != actual_digest",
            "false",
        ),
        (
            "capability",
            "binary_search(&env.capability_id)",
            "binary_search(&1)",
        ),
        (
            "parent_generation",
            "env.parent_generation != self.state.parent_generation",
            "false",
        ),
        (
            "policy_epoch",
            "env.policy_epoch != self.state.policy_epoch",
            "false",
        ),
        (
            "capability_epoch",
            "env.capability_epoch != self.state.capability_epoch",
            "false",
        ),
        (
            "object_dependency",
            "env.object_version != self.state.object_version",
            "false",
        ),
        (
            "predicate_dependency",
            "env.predicate_version != self.state.predicate_version",
            "false",
        ),
        (
            "operation_mismatch",
            "existing.effect_digest == actual_digest",
            "true",
        ),
        ("nonce", "self.nonces.contains(&env.nonce)", "false"),
        (
            "ledger_sort",
            "self.ledger.values().cloned().collect()",
            "self.ledger.values().rev().cloned().collect()",
        ),
        (
            "nonce_sort",
            "self.nonces.iter().copied().collect()",
            "self.nonces.iter().rev().copied().collect()",
        ),
    ];
    let mut summary = String::new();
    for (name, from, to) in mutations {
        assert_eq!(
            source.matches(from).count(),
            1,
            "unambiguous mutation {name}"
        );
        fs::write(scratch.join("src/kernel.rs"), source.replacen(from, to, 1)).unwrap();
        fs::write(
            scratch.join(format!("{name}.mutation.txt")),
            format!("FROM {from}\nTO {to}\n"),
        )
        .unwrap();
        let out = Command::new(env!("CARGO"))
            .current_dir(&scratch)
            .env("CARGO_INCREMENTAL", "0")
            .env("CARGO_TARGET_DIR", scratch.join("target"))
            .args(["run", "--locked", "--offline", "--"])
            .arg(&fixture)
            .output()
            .unwrap();
        fs::write(scratch.join(format!("{name}.stdout")), &out.stdout).unwrap();
        fs::write(scratch.join(format!("{name}.stderr")), &out.stderr).unwrap();
        let stdout = String::from_utf8(out.stdout).unwrap();
        assert_eq!(
            out.status.code(),
            Some(1),
            "must be a semantic failure, not compile failure: {name}"
        );
        assert!(
            stdout.lines().any(|line| line.starts_with("FAIL ")),
            "{name}"
        );
        assert!(stdout.contains("vectors=32 failures="), "{name}");
        summary.push_str(&format!(
            "{name}: semantic_detected=true exit=1 {}\n",
            stdout.lines().last().unwrap()
        ));
    }
    fs::write(scratch.join("summary.txt"), summary).unwrap();
    assert_eq!(
        fs::read_to_string(root.join("src/kernel.rs")).unwrap(),
        source
    );
}
