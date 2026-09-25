# Replica Commit Kernel — Rust v1.3A candidate

The supplied archive was uncompiled in its research environment. On the Replica
Mac, the original candidate passed all32 frozen vectors in debug and release.
Direct negative tests then exposed a manifest-preflight gap and malformed-event
panic. This port fixes those boundaries without changing any frozen expectation.
Gate v1.3A execution passes:32 vectors/profile,10 direct negative test functions,
and11 compiled semantic guard mutants detected. This is not durability,
concurrency, natural-language quality or product integration acceptance.

## Scope

v1.3A intentionally ports only the frozen **v1.2B 32-vector core commit boundary**. Later v1.2C–M contracts are listed in `docs/FROZEN_CONTRACT_B_TO_M.md` and remain future Rust gates.

## First target-machine gate

```bash
rustc -Vv
cargo -V
cargo test --locked --offline
cargo test --locked --offline --release
cargo run --locked --offline -- tests/data/commit_kernel_reference_vectors_v1_2B.json
cargo run --locked --offline --release -- tests/data/commit_kernel_reference_vectors_v1_2B.json
```

Expected:
- tests pass;
- CLI prints 32 `PASS <vector>` lines;
- final line is `vectors=32 failures=0`;
- exit code 0.

Then preserve:
- `rustc -Vv`
- `cargo -V`
- generated `Cargo.lock`
- command stdout/stderr
- git commit/SHA of this port.

This is an independent crate, not a dependency of Replica's product or training
binary. Its supplied JSON fixture and canonical JSON digest compatibility are
reference-protocol inputs, not a persistent user-memory format. The main project's
Cargo manifest/lock, model paths and binary storage are unchanged. A separate lock
was generated offline because the supplied archive had no lockfile; retain it.

For mutations, set `R3_KERNEL_MUTATION_ROOT` to a new scratch directory and run
`cargo test --locked --offline --test mutations -- --ignored --exact guards_are_killed_by_frozen_vectors`.
The test preserves the original source/fixture and requires a compiled CLI exit1
with actual failed vectors; a compiler failure is not a detected mutant.

## Frozen bundle identity

SHA-256:

`cd5f0533762fe68da3c3da01410c87a240a224db33350c52f3d4c339984c0323`
