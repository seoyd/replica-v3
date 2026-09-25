//! Small semantic mutation gate. The real source and frozen fixture are read-only.
//! One scratch crate and its isolated build target are reused sequentially.
use std::{fs, path::PathBuf, process::Command};

#[test]
#[ignore = "two compiled external retry/absence guard mutants"]
fn external_guards_are_killed() {
    let (root, scratch) = prepare_scratch();
    fs::create_dir_all(scratch.join("tests/data")).unwrap();
    for p in [
        "tests/external.rs",
        "tests/data/commit_kernel_v1_2I_vectors.json",
        "tests/data/commit_kernel_v1_2J_vectors.json",
        "tests/data/commit_kernel_v1_2M_vectors.json",
    ] {
        fs::copy(root.join(p), scratch.join(p)).unwrap();
    }
    let source = fs::read_to_string(root.join("src/external.rs")).unwrap();
    let mut summary = String::new();
    for (name, from, to, test, ignored) in [
        (
            "open_absence",
            "if o.closed_world && o.watermark_closed {",
            "if true {",
            "frozen_j_twelve_cases_and_attribution_boundaries",
            false,
        ),
        (
            "non_idempotent_retry",
            "if t.status == TaskStatus::InFlight && !self.receiver_idempotent {",
            "if false {",
            "i_directed_restart_and_non_idempotent_unknown",
            true,
        ),
    ] {
        assert_eq!(source.matches(from).count(), 1);
        fs::write(
            scratch.join("src/external.rs"),
            source.replacen(from, to, 1),
        )
        .unwrap();
        fs::write(
            scratch.join(format!("{name}.mutation.txt")),
            format!("FROM {from}\nTO {to}\n"),
        )
        .unwrap();
        let mut command = Command::new(env!("CARGO"));
        command
            .current_dir(&scratch)
            .env("CARGO_INCREMENTAL", "0")
            .env("CARGO_TARGET_DIR", scratch.join("target"))
            .env("CARGO_PROFILE_DEV_DEBUG", "0")
            .env("CARGO_PROFILE_TEST_DEBUG", "0")
            .env("R3_KERNEL_E_EVIDENCE", scratch.join(name))
            .args([
                "test",
                "--locked",
                "--offline",
                "--test",
                "external",
                "--",
                "--exact",
                test,
            ]);
        if ignored {
            command.arg("--ignored");
        }
        let out = command.output().unwrap();
        fs::write(scratch.join(format!("{name}.stdout")), &out.stdout).unwrap();
        fs::write(scratch.join(format!("{name}.stderr")), &out.stderr).unwrap();
        assert_eq!(out.status.code(), Some(101));
        assert!(String::from_utf8(out.stdout)
            .unwrap()
            .contains(&format!("test {test} ... FAILED")));
        summary.push_str(&format!(
            "{name}: compiled_semantic_failure=true exit=101 test={test}\n"
        ));
    }
    fs::write(scratch.join("summary.txt"), summary).unwrap();
    assert_eq!(
        fs::read_to_string(root.join("src/external.rs")).unwrap(),
        source
    );
}

#[test]
#[ignore = "two compiled correction closure/cache mutants; explicit scratch root"]
fn correction_closure_mutants_are_killed() {
    let (root, scratch) = prepare_scratch();
    fs::create_dir_all(scratch.join("tests/data")).unwrap();
    for path in [
        "tests/correction.rs",
        "tests/data/commit_kernel_v1_2K_vectors.json",
        "tests/data/commit_kernel_v1_2L_vectors.json",
    ] {
        fs::copy(root.join(path), scratch.join(path)).unwrap();
    }
    let source = fs::read_to_string(root.join("src/correction.rs")).unwrap();
    let cases = [
        ("direct_head_only", "for d in r.deps.iter().rev() {", "for d in r.deps.iter().rev().take(0) {", vec!["--test", "correction"], "frozen_provenance_and_directed_closure"),
        ("cache_truth", "let mut todo = vec![(key.clone(), false)];", "if self.cache.get(key) == Some(&Validity::Active) { return true; } let mut todo = vec![(key.clone(), false)];", vec!["--lib"], "correction::tests::lying_and_half_propagated_cache_has_no_authority"),
    ];
    let mut summary = String::new();
    for (name, from, to, target, test) in cases {
        assert_eq!(source.matches(from).count(), 1);
        fs::write(
            scratch.join("src/correction.rs"),
            source.replacen(from, to, 1),
        )
        .unwrap();
        fs::write(
            scratch.join(format!("{name}.mutation.txt")),
            format!("FROM {from}\nTO {to}\n"),
        )
        .unwrap();
        let out = Command::new(env!("CARGO"))
            .current_dir(&scratch)
            .env("CARGO_INCREMENTAL", "0")
            .env("CARGO_TARGET_DIR", scratch.join("target"))
            .env("CARGO_PROFILE_DEV_DEBUG", "0")
            .env("CARGO_PROFILE_TEST_DEBUG", "0")
            .args(["test", "--locked", "--offline"])
            .args(target)
            .args(["--", "--exact", test])
            .output()
            .unwrap();
        fs::write(scratch.join(format!("{name}.stdout")), &out.stdout).unwrap();
        fs::write(scratch.join(format!("{name}.stderr")), &out.stderr).unwrap();
        assert_eq!(out.status.code(), Some(101));
        assert!(String::from_utf8(out.stdout)
            .unwrap()
            .contains(&format!("test {test} ... FAILED")));
        summary.push_str(&format!(
            "{name}: compiled_semantic_failure=true exit=101 test={test}\n"
        ));
    }
    fs::write(scratch.join("summary.txt"), summary).unwrap();
    assert_eq!(
        fs::read_to_string(root.join("src/correction.rs")).unwrap(),
        source
    );
}

fn prepare_scratch() -> (PathBuf, PathBuf) {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let scratch = PathBuf::from(
        std::env::var_os("R3_KERNEL_MUTATION_ROOT").expect("explicit new scratch root"),
    );
    fs::create_dir(&scratch).expect("scratch must not exist");
    fs::create_dir(scratch.join("src")).unwrap();
    for path in [
        "Cargo.lock",
        "src/lib.rs",
        "src/vectors.rs",
        "src/main.rs",
        "src/durable.rs",
        "src/kernel.rs",
        "src/concurrency.rs",
        "src/correction.rs",
        "src/external.rs",
    ] {
        fs::copy(root.join(path), scratch.join(path)).unwrap();
    }
    let repo = root.join("../..").canonicalize().unwrap();
    let manifest = fs::read_to_string(root.join("Cargo.toml"))
        .unwrap()
        .replace("\"../..\"", &serde_json::to_string(&repo).unwrap())
        .replace(
            "\"../../vendor/candle-core-0.11.0\"",
            &serde_json::to_string(&repo.join("vendor/candle-core-0.11.0")).unwrap(),
        );
    fs::write(scratch.join("Cargo.toml"), manifest).unwrap();
    (root, scratch)
}

#[test]
#[ignore = "explicit scratch path required; compiles eleven semantic guard mutants"]
fn guards_are_killed_by_frozen_vectors() {
    let (root, scratch) = prepare_scratch();
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

#[test]
#[ignore = "explicit scratch root; C guard mutants must execute and fail real tests"]
fn concurrency_guards_are_killed() {
    let (root, scratch) = prepare_scratch();
    fs::create_dir(scratch.join("tests")).unwrap();
    fs::copy(
        root.join("tests/concurrency.rs"),
        scratch.join("tests/concurrency.rs"),
    )
    .unwrap();
    let source = fs::read_to_string(root.join("src/concurrency.rs")).unwrap();
    let boundary = "commit_time_invalidation_and_replay_preserve_original_guards";
    let preservation = "preservation_guards_and_real_queue_boundaries";
    let races = "seven_barrier_races_use_the_actual_authority";
    let mutations = [
        (
            "effect_digest",
            "p.reviewed_effect_digest != digest",
            "false",
            preservation,
        ),
        (
            "capability_registry",
            "!self.capabilities.contains(&p.capability_id)",
            "false",
            preservation,
        ),
        ("parent_active", "!s.parent_active", "false", boundary),
        (
            "parent_generation",
            "p.expected.parent_generation != s.parent_generation",
            "false",
            boundary,
        ),
        (
            "policy",
            "p.expected.policy_epoch != s.policy_epoch",
            "false",
            boundary,
        ),
        (
            "capability_epoch",
            "p.expected.capability_epoch != s.capability_epoch",
            "false",
            boundary,
        ),
        (
            "object",
            "p.expected.object_version != s.object_version",
            "false",
            boundary,
        ),
        (
            "predicate",
            "p.expected.predicate_version != s.predicate_version",
            "false",
            races,
        ),
        (
            "operation_mismatch",
            "entry.effect_digest != digest",
            "false",
            preservation,
        ),
        (
            "nonce",
            "self.nonces.contains_key(&p.nonce)",
            "false",
            races,
        ),
        (
            "operation_dedup",
            "self.ledger.get(&p.operation_id)",
            "self.ledger.get(&u64::MAX)",
            boundary,
        ),
    ];
    let mut summary = String::new();
    for (name, from, to, test) in mutations {
        let count = source.matches(from).count();
        assert!(count > 0, "mutation {name} exists");
        fs::write(scratch.join("src/concurrency.rs"), source.replace(from, to)).unwrap();
        fs::write(
            scratch.join(format!("{name}.mutation.txt")),
            format!("count={count}\nFROM {from}\nTO {to}\n"),
        )
        .unwrap();
        let out = Command::new(env!("CARGO"))
            .current_dir(&scratch)
            .env("CARGO_INCREMENTAL", "0")
            // Same package/target names can overwrite the real test executable in
            // a shared target while Cargo retains its original freshness stamp.
            .env("CARGO_TARGET_DIR", scratch.join("target"))
            .env("CARGO_PROFILE_DEV_DEBUG", "0")
            .env("CARGO_PROFILE_TEST_DEBUG", "0")
            .env_remove("R3_KERNEL_C_EVIDENCE")
            .args([
                "test",
                "--locked",
                "--offline",
                "--test",
                "concurrency",
                "--",
                "--exact",
                test,
            ])
            .output()
            .unwrap();
        fs::write(scratch.join(format!("{name}.stdout")), &out.stdout).unwrap();
        fs::write(scratch.join(format!("{name}.stderr")), &out.stderr).unwrap();
        let stdout = String::from_utf8(out.stdout).unwrap();
        assert_eq!(out.status.code(), Some(101), "{name}");
        assert!(
            stdout.contains(&format!("test {test} ... FAILED")),
            "compiled semantic failure required: {name}"
        );
        assert!(stdout.contains("test result: FAILED"), "{name}");
        summary.push_str(&format!(
            "{name}: compiled_semantic_failure=true exit=101 test={test}\n"
        ));
    }
    fs::write(scratch.join("summary.txt"), summary).unwrap();
    assert_eq!(
        fs::read_to_string(root.join("src/concurrency.rs")).unwrap(),
        source
    );
}
