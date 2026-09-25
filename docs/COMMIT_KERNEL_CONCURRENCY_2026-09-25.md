# Commit Kernel C execution — 2026-09-25

RESULT: PASS for the standalone in-memory C boundary. D/E and product integration
are NOT_TESTED. No model quality, power-loss or external-effect claim is made.
GRU remains entirely separate from TR++ architecture, weights, state, training
and inference. Neither was changed or invoked in this work.

## Inputs and semantics

Starting source/report HEAD: `4ac820c4f8d2f0d45fcd4fb2433a3d73dab700b3`.
Prior A source: `17fe36ca5bada6411a44abc70de46991069c5597`.
Prior B source: `d520e272dd4cd8d8a83ec7e23b3bf752eb4929c9`.

User archive: `/Users/seo/Downloads/replica_commit_kernel_v1_2B_E_F_bundle.tar.gz`,
14017 bytes, SHA256
`abcf7183bac79e7b0ae0426900fedbb31c32bb3f4d85d8eccd8a4121432fb389`.
All nine supplied manifest hashes matched. Only regular files/directories were
extracted under `artifacts/commit-kernel-c-20260925/` (hereafter evidence root).
The original archive and earlier evidence remain intact.

The bundle labels E Python as a reconstruction of the original session code;
it is not a preserved original source file. F Go sources/reports are preserved
original artifacts. Neither Python nor Go was executed. E weak interleavings
were transcribed into Rust; strong outcomes use the actual F authority/state.
F reads the current opposite bit at commit; E's weak toy reads stale state.
These models must not be described as identical differential implementations.
F source defaults and its stored report use different attempt counts; this
Rust run is its own declared workload, not a reproduction of Go's106551 events.

F DELTA increments object version; A_OFF/B_OFF conditionally increment predicate
version. Frozen B leaves those versions unchanged and is not rewritten. C has
its own state/digest identity and preserves checked arithmetic, actual payload
digest calculation, reviewed digest, capability membership and commit-time
parent/policy/capability/object/predicate checks. Commit, nonce and receipt/ledger
mutation are serialized under one authority mutex. Overflow changes no canonical
effect/nonce/ledger; the audit sequence still records the rejected request.
Trusted controls are separate from proposal submission; no untrusted command
adapter is exposed. Pending queues/tickets are not durable state.

## Actual tests

All Cargo commands below run in `crates/replica-commit-kernel` with
`CARGO_INCREMENTAL=0 CARGO_TARGET_DIR=../../target`, locked/offline. Toolchain
is rustc1.98.1, cargo1.98.1, aarch64-apple-darwin, macOS27.0/26A428; prior full
toolchain output is retained in the A/B evidence. No dependency or lock update.

| Check | Actual result |
| --- | --- |
| `cargo test --locked --offline --lib concurrency::tests::overflow_rejects_without_partial_canonical_effect -- --exact` | 1 test PASS, exit0;11 canonical overflow cases plus sequence overflow |
| `cargo test --locked --offline --test concurrency -- --test-threads=1` | 5 tests PASS, exit0; final execution0.75s |
| `cargo test --locked --offline --test mutations concurrency_guards_are_killed -- --exact --ignored` | 1 test PASS,11 compiled mutants detected; each selected child test fails semantically with exit101 |
| `cargo test --locked --offline --test direct_negative --test preflight --test reference_vectors` | 10+1+1 tests PASS, exit0; frozen32 vectors included |

Final concurrency command sets
`R3_KERNEL_C_EVIDENCE=/Users/seo/Projects/Replica-v3/artifacts/commit-kernel-c-20260925/final-isolated`.
Final mutation command sets
`R3_KERNEL_MUTATION_ROOT=/Users/seo/Projects/Replica-v3/artifacts/commit-kernel-c-20260925/mutations-isolated`.
Child mutants use that scratch's isolated target, with dev/test debug symbols0.
Their source/patch/stdout/stderr/summary remain there. Compiler failure is not
counted as semantic detection. No zero-test target is counted as PASS.

| Exhaustive scenario | Weak violations | Actual F violations |
| --- | --- | --- |
| write skew |18/20|0/20|
| phantom |2/4|0/4|
| same operation retry |4/6|0/6|
| same nonce |4/6|0/6|
| same object |4/6|0/6|
| cancel/revoke |4/12|0/12|

Seven real barrier races each ran128 rounds. Same-op:128 COMMIT+128 REPLAY.
Same-nonce/write-skew/same-object: each128 COMMIT+128 expected rejection.
Insert/cancel/revoke races: respectively3/2/9 COMMIT and125/126/119 rejection;
each was checked against actual serialized order, not a predetermined schedule.
Queue caps32/64/64, High mutation floor,65/25/10 service,64 borrowed Low slots
and1001 identical pending retries were tested through the authority.

Final stress:32 workers ×256 proposals =8192 attempts, plus512 controls.
8548 actual executions (156 queued duplicates coalesced),725 distinct commits.
Every captured response and final state matches serial replay of the actual
execution order; replay mismatch0, duplicate effect0, repeated nonce commit0.
Native R3BIN execution trace was round-tripped and preserved. This replay uses
the same transition function; independent protection comes from the directed
outcome/invariant checks and guard mutations, not a second reference engine.
Final state digest:
`699522a3d4de25643708069402ff31ec62631b939d30b2e10930f5aab7345da4`.

## Failures preserved and identities

Initial test compilation failed on ambiguous `sum()` typing; `sum::<usize>()`
fixed it. It executed0 tests and is not a PASS. Later, scratch mutants and normal
tests shared a target directory and identical package/target names. The last
operation-dedup mutant overwrote the normal executable while Cargo considered
the original target fresh. Final check failed3/5 with REJECT_NONCE replacing
REPLAY; the root source still contained the correct guard.

`concurrency-final.log` and `concurrency-mutant-collision-test` preserve that
failure; wrong executable SHA256:
`fa1217ec6c9d6250d8ce1806cdfad7c4175b375a79ff97cc414dc7e7a415b33a`.
The harness now isolates mutant targets. Original source/test mtimes were
refreshed to force recompilation without changing their content. All11 mutants
and the normal5-test gate then passed their intended checks. The final normal
binary exactly matches the earlier passing run02 binary. Earlier failed or
partial evidence directories are not rewritten.

| File | SHA256 |
| --- | --- |
| `src/concurrency.rs` | `67ae7869d67b6d6dd67b80b643dd9258f6e51083343de02251cca774c9e685f6` |
| `tests/concurrency.rs` | `26ffdfec9246247594dcb6efc89da8a40d7136b91a76d8e71a67ec5df0246546` |
| `tests/mutations.rs` | `f73c2df825439d7c0deecd1d1e7e3f6731ac0bc6fc93f9afd8e2c6231e20c73f` |
| E fixture | `b5ef799f90fcebd8ab4f917f1a77e6e7e318360cb2b38eb23752139f1f81dce9` |
| `concurrency-final-test` | `a8610a9065b05ed93dfa0974cdc270979af348fd2fd7cb0b5de652a2e681dad2` |
| `mutations-final-test` | `80975f777e85b0525a26f6003d1780f746beed9ad01acfbf70c49919af0cc190` |
| Root lock | `a911931fc3baca7367a34c129673640661a8fc3d41458d49e4eb744ae5454154` |
| Crate lock | `57a5361c426c69ada27eaecc320a1d0516b02c75c721e9867a4093ddb88eb007` |

Full candidate paths, hashes and final logs are in the evidence root:
`source-binary-lock.sha256`, `primitive-final.log`, `a-regression.log`,
`concurrency-final-isolated.log`, `mutations-isolated.log` and matching exit files.
`final-isolated/` contains exhaustive/queue/barrier/stress summaries and
`actual-executions.r3b`. Git's named stage commit supplies the candidate diff;
original model/corpus/raw and temporary instructions are not published.

## Limits and next gate

At the scoped measurement before this report, C evidence had1942 files and
563163604 logical bytes (including scratch build outputs and preserved binaries).
Shared debug allocation was11412448KiB,16612KiB above the prior11395836KiB.
Peak/transient usage and Codex token usage are UNKNOWN. No cleanup was performed.
No sub-agent or independent-review PASS is claimed. Current session settings
were not changed. Generation/teacher/optimizer/backward counts are all0.

B disk durability was not rerun; C is currently in-memory and has no combined
concurrent crash/restore evidence. Service-share tests assume a progressing
host pump; they are not OS scheduling latency guarantees. Correction closure,
external effects and model integration remain later gates. Original K/L inputs
have been requested before porting their precise version/closure semantics.
This work does not approve any GRU/TR++ mixture or change protected11264.
