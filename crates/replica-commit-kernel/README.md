# Replica Commit Kernel — Rust A/B/C/D/E component gates

The supplied archive was uncompiled in its research environment. On the Replica
Mac, the original candidate passed all32 frozen vectors in debug and release.
Direct negative tests then exposed a manifest-preflight gap and malformed-event
panic. This port fixes those boundaries without changing any frozen expectation.
Gate v1.3A execution passes:32 vectors/profile,10 direct negative test functions,
and11 compiled semantic guard mutants detected. This is not durability,
concurrency, natural-language quality or product integration acceptance.

## Scope

v1.3A ports the frozen **v1.2B 32-vector core commit boundary**. v1.3B adds
the R3BIN single-writer disk/process boundary described below. v1.3C adds a
separate in-memory F concurrency profile. v1.3D adds the K/L correction profile
below. v1.3E adds the local I/J/M external-effect profile below. These component
gates do not yet constitute an integrated product commit authority.

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

Gate v1.3B adds a single-writer durable snapshot with a host-supplied native codec.
Tests use the existing `replica_v3::binary` R3BIN codec through a dev-only
dependency; no model is constructed. The snapshot atomically contains initial
state, event history, journal/receipts and final state/ledger/nonces. Restore
replays and verifies the full state. Save errors poison the live handle until
disk-only reopen. Temps/backups are not recovery authority. Actual SIGKILL tests
cover seven save/reply boundaries; this is not a power-loss proof.

Run these tests with a new `R3_KERNEL_PROCESS_ROOT` directory:
`cargo test --locked --offline --test durable_process -- --ignored --skip process_worker --test-threads=1`.

For mutations, set `R3_KERNEL_MUTATION_ROOT` to a new scratch directory and run
`cargo test --locked --offline --test mutations -- --ignored --exact guards_are_killed_by_frozen_vectors`.
The test preserves the original source/fixture and requires a compiled CLI exit1
with actual failed vectors; a compiler failure is not a detected mutant.

## C: separate concurrency profile

`concurrency::Authority` serializes commit-time validation and mutation, with
bounded Critical/High/Low queues (32/64/64), a 65/25/10 service cycle, empty-share
borrowing and identical pending-proposal coalescing. The host pumps
`service_one`; a pending ticket is not a durable receipt. Proposal priority has
a High floor; controls are a separate trusted-host API, not user authority.

The supplied F protocol advances object version on DELTA and predicate version
on conditional A_OFF/B_OFF. It is separate from the frozen A/B protocol, whose
32-vector expectations and version semantics remain unchanged. Rust retains
checked arithmetic, trusted capability membership and reviewed-effect binding.
E's reconstructed weak reference and F's preserved source are distinct models;
matching safety goals does not imply identical state hashes or workloads.

Run `cargo test --locked --offline --test concurrency -- --test-threads=1`.
Set `R3_KERNEL_C_EVIDENCE` to a new directory to preserve actual R3BIN execution
traces. For C mutations, use a new `R3_KERNEL_MUTATION_ROOT` and
`cargo test --locked --offline --test mutations concurrency_guards_are_killed -- --exact --ignored`.
Mutants use their own target directory: sharing same-name package outputs can
overwrite a normal executable without invalidating its Cargo freshness record.

C has no disk-restore adapter yet; B process durability and C concurrency PASS
are not combined concurrent durability. No model/generation/training integration
is enabled. GRU must remain separate from TR++ weights, state, training and
inference; this crate adds neither core nor a bridge between them.

## D: versioned correction profile

`correction::Store` keeps immutable version records, current heads, exact
dependencies and a rebuilt reverse/cache index. `record` is a historical read;
`is_current` traverses the full closure at current use and does not trust cache.
Appending a correction invalidates reachable descendants only. An already
executed action is marked `CompensationRequired`; no external action is performed.
Status/payload are retained evidence, not a semantic truth judgement.

`correction::DurableStore` writes record additions and head switches together
using B's atomic native-file helper and an exclusive writer lock. A batch fails
without publishing a partial state. Uncertain I/O poisons the live handle until
disk-only restore. Native decoding is supplied by the host; tests use R3BIN.
Missing, duplicate, cyclic or regressed state fails closed. Temps and sidecar
caches are never recovery authority. This separate K/L profile is not yet
connected to C's proposal/receipt protocol or product memory.

Run `cargo test --locked --offline --test correction` for directed/stress checks.
For actual child-process tests, set `R3_KERNEL_D_EVIDENCE` to a new directory and
run `cargo test --locked --offline --test correction -- --ignored --skip process_worker --test-threads=1`.
With a new mutation root, run
`cargo test --locked --offline --test mutations correction_closure_mutants_are_killed -- --exact --ignored`.
The two supplied K/L JSON scenario files were reconstructed after the research
run; their original source/report provenance and this Rust execution are distinct.

## E: local external-effect profile

`external::Endpoint` persists typed sender/receiver snapshots through the same
atomic native-file helper. Sender admission is durable before transport. An
ambiguous non-idempotent send becomes `UnknownEffect` and cannot automatically
retry. An idempotent receiver binds stable effect ID to payload, applying effect,
ledger and receipt together; the receipt returned on replay is the saved one.
Restore replays receiver history and verifies value/ledger/receipt consistency.

`reconcile` accepts attributable per-effect evidence from a trusted host adapter.
Stale/non-authoritative/other-effect observations and open-world absence do not
become terminal truth. Closed absence additionally requires a closed delivery
watermark; the receiver's ordinary query does not claim that watermark. Evidence
history is retained. A boolean supplied by a model is not authority proof.

Compensation is a separately declared numerical effect with a different stable
ID; retries keep that ID. An ambiguous in-flight reversal requires reconciliation.
Only proven application permits restoration. Without a declared compensator,
the sender records manual intervention. One unresolved compensation cycle is
allowed at a time; this bounded profile does not invent arbitrary inverses.
Late ACK handling reads current sender state. Earlier completed compensation
does not create an extra restore in a later cancelled cycle.

These tests use local child processes and R3BIN request/response files, not live
external services. The closed-absence probe joins every old sender/receiver
before claiming the delivery watermark. No general network exactly-once or
external compensation guarantee follows.

Run the frozen12 reconciliation cases with
`cargo test --locked --offline --test external frozen_j_twelve_cases_and_attribution_boundaries -- --exact`.
For local process gates, set `R3_KERNEL_E_EVIDENCE` to a new directory and run
`cargo test --locked --offline --test external -- --include-ignored --skip process_worker --skip evidence_child_counts --test-threads=1`.
`R3_KERNEL_E_REUSE_I`, if explicitly set, points to completed I150/I600 evidence
for read-only native/receipt revalidation; it never replaces the remaining tests.
With a new mutation root, run
`cargo test --locked --offline --test mutations external_guards_are_killed -- --exact --ignored`.

A/B, C, D and E remain separate typed profiles. Combining authorization,
correction closure, durable receipts and existing product memory is a subsequent
integration boundary, not implied by these standalone PASS results. GRU/TR++
remain separate and neither is invoked by the kernel tests.

## Frozen bundle identity

SHA-256:

`cd5f0533762fe68da3c3da01410c87a240a224db33350c52f3d4c339984c0323`
