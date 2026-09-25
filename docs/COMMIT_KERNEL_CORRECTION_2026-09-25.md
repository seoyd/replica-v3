# Commit Kernel D execution — 2026-09-25

RESULT: PASS for the standalone K/L correction profile's declared gates.
No model, GRU/TR++ integration, external-effect execution or power-loss claim.
C source/remote baseline: `6fc23152ba6d5c8eadadc999c6f6ea11a59ce496`,
tree `c7dc0f8f719a77ce82cdd83d66c57a8f498e318b`; the actual remote main SHA was
queried and matched before starting D. Prior A/B/C failures and evidence remain.

## Input provenance

User archive: `/Users/seo/Downloads/replica_commit_kernel_v1_2K_L_bundle.tar.gz`.
SHA256: `79173b43efe00370f1b86e6bc14692ba231a2a70ccf6adc3736f857f92d71af6`.
All nine manifest entries matched. Extraction contains only regular files and
directories under `artifacts/commit-kernel-d-20260925/` (the evidence root).
K Python/L Go source and reports are preserved original artifacts. Their two
scenario JSON files were reconstructed later, not saved during the original run.
No Python/Go code was executed, and no original random tape was reconstructed.

K source SHA256: `be739a0210d11e5a27a0a42988e35d1f607cf91403fea482cd8216e054d6c899`.
L source SHA256: `d41e51a4843982c0788b77d47c9d6a89de8bfe23b334b191ebe9188821105ca4`.
K fixture: `5c540883cfc656c9d0b953b2002ff62fe17ce24543e52146f69e398712ae7f1b`.
L fixture: `066e571ead5fe48fc87293dee8e592012069009e4a985da5c35d6446842d3618`.
Fixtures are hash-pinned in direct tests; directed scenarios are implemented in
Rust assertions, not a general JSON execution interpreter.

## Implementation and explicit limits

`correction::Store` preserves record versions, payload/status and exact
dependencies. Iterative current-closure validation ignores cache. Reverse edges
propagate invalidation to reachable descendants. Historical reads remain
available and confer no current mutation authority. Already-executed action
metadata produces `CompensationRequired`, without executing compensation or
upgrading epistemic labels. The tests'40 such records are synthetic flags.

Rust rejects duplicate/missing/cyclic dependencies, regressed/dangling heads,
oversized fields, self-history dependency and invalid version lineage. Compared
with the permissive research append helper, this profile explicitly requires
initial version1 and subsequent version+1 with matching correction_of. These
are port input hardening rules, not a claim of identical behavior for every
malformed Python/Go input. Maximum records200000, native snapshot64MiB; existing
R3BIN item/depth limits still apply independently and are not enlarged.

`DurableStore` has one exclusive disk writer; batches append records and switch
heads in one native snapshot. It reuses B's extracted `persist_bytes` helper:
new temp, full write, file sync, close, atomic rename, directory sync, then reply.
I/O uncertainty poisons the live handle until disk-only reopen. Restore validates
records/heads and rebuilds reverse/cache indexes, ignoring temps and caches.
The cache-half/cache-rename process probes write a test-only advisory sidecar
after canonical save; production correctness does not depend on persisting it.

This is a separate K/L profile. It is not yet C's queued proposal/nonce/receipt
state or Replica's operating memory. All APIs are trusted local Rust calls;
no neural proposal or external transport adapter has been added. No GRU/TR++
weights, inference, training, tokenizer, model checkpoints or root lock changed.

## Actual execution

Cargo workdir: `crates/replica-commit-kernel`. Environment:
`CARGO_INCREMENTAL=0 CARGO_TARGET_DIR=../../target`. Toolchain/OS reuse the
verified A/B setup: rustc/cargo1.98.1, aarch64-apple-darwin, macOS27.0/26A428.

| Command | Result |
| --- | --- |
| `cargo test --locked --offline --test correction -- --include-ignored --skip process_worker --test-threads=1 --nocapture` | Final6 tests PASS, exit0,9.47s |
| `cargo test --locked --offline --lib correction::tests::lying_and_half_propagated_cache_has_no_authority -- --exact` |1 PASS, exit0|
| `cargo test --locked --offline --test mutations correction_closure_mutants_are_killed -- --exact --ignored` |1 PASS;2 compiled semantic mutants detected,23.27s|
| `cargo test --locked --offline --test durable_process -- --ignored --skip process_worker --test-threads=1` |B shared-writer regression3 PASS, exit0,0.40s|

Final process environment:
`R3_KERNEL_D_EVIDENCE=/Users/seo/Projects/Replica-v3/artifacts/commit-kernel-d-20260925/final-process`.
Mutation environment uses `R3_KERNEL_MUTATION_ROOT` at the evidence root's
`mutations/`; scratch has its own target and debug-symbols0. Each mutant's child
test executes and fails with101; compile failure is not detection. The mutations
are direct-head-only and trusting ACTIVE cache. Root source remains unchanged.
B process root is `b-shared-writer/`; this rerun is justified by the shared helper
extraction, not a repeat of unchanged model acceptance.

Final checks:

- Five directed K semantic cases, historical record preservation, malformed
  snapshots and no-partial-mutation rejection passed.
- 100000 in-memory descendants:99960 stale,40 compensation flags. Unrelated
  branches:25000 invalidated and25000 current.5000 deterministic correction/use
  interleavings: stale5000 rejected and fresh5000 committed. This is a constructed
  Rust workload, not the original race tape or its136 external flags.
- Native truth/cache rebuild:36336 records, canonical bytes identical after
  restore. This subset is declared separately from the100000-node fanout.
- Eight real correction SIGKILL boundaries: before rename OLD; rename or later
  NEW. Five recompute boundaries: before rename no fresh action, rename or later
  fresh action current; old action invalid throughout the corrected world.
- Actual split-head-first mutant produces a dangling head rejected on restore.
  Malformed/lying cache, exclusive-writer collision, rejected batch, save failure,
  poisoned handle and corrupt committed state all fail closed as intended.
- Additional200 real SIGKILL runs use a fixed round-robin point schedule:
  OLD100/NEW100/OTHER0. This is not the original random OLD98/NEW102.

The initial process suite and final source-bound suite each used214 actual D
SIGKILLs (8 correction+5 recompute+1 split mutant+200 repeated). Both raw roots
are preserved. B regression used another7. Zero model generation, teacher,
optimizer or backward calls occurred. No independent reviewer PASS is claimed.

## Failures and immutable identities

Initial direct run passed2 tests and failed the combined stress test when trying
to encode the entire100002-record fanout in one R3BIN value (`value bounds`).
The codec was not weakened. Tests now separate the100000-node memory gate from
the declared36336-record native-rebuild gate. The failure is retained in
`direct.log`; corrected direct and final logs remain separate. A wrong relative
format-command path also failed; formatting was rerun from the crate workdir.

| Candidate/evidence | SHA256 |
| --- | --- |
| `src/correction.rs` | `e700318bafd55e479ffac75ab626f013e67110bb3f3fce64210d7b1a32a58c40` |
| `src/durable.rs` | `957fd10e1121e5ba082eeaaa309a1b9606771678b6b7bd570931fbfe7bb74d14` |
| `tests/correction.rs` | `e1deb99d4477c4548236953d77a8bda35f1c2a7b9cf435f0e98328f4315cc43e` |
| `tests/mutations.rs` | `4dc2e1042ddea9b5c2a9e007167217b0132d9cb57af0d4ddc64b42889a0b2915` |
| `correction-final-test` | `97478fc3b398dda404709352cbe6122e0047510abcce025d3a99b3a545569db6` |
| `b-regression-test` | `7584701473f79f8f9a13b2b4b610df8254b4a36e6be5a7ca34799e961818df76` |
| `mutations-final-test` | `0a7f954c8d462b8c75a6b5186aa600213e167597ba1c9aab7739d87fe7af0490` |

Root lock remains `a911931fc3baca7367a34c129673640661a8fc3d41458d49e4eb744ae5454154`;
crate lock remains `57a5361c426c69ada27eaecc320a1d0516b02c75c721e9867a4093ddb88eb007`.
Exact hashes/paths are also in `source-binary-lock.sha256`. Final log/exit files,
process summaries and compiled mutant outputs are under the evidence root.
Git's named stage commit supplies the candidate diff; original models/raw and
temporary instructions are not published.

Scoped measurement before this report:6592 files,553371561 logical bytes,
557988KiB allocated. Shared debug11441076KiB,28628KiB above C's final measurement.
Peak/transient allocation and Codex usage remain UNKNOWN. No cleanup, model
copy, inventory outside this scope or model-setting change occurred.

Next: original I/J/M source/scenarios for external-effect/compensation E were
requested. E and eventual C/D/product integration remain NOT_TESTED. Process
SIGKILL evidence is not power-loss proof, and consistency is not language truth.
