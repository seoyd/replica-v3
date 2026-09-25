# Native temp allocation recovery — 2026-09-25

Implementation scope: R3-COMMIT-KERNEL-TEMP-RECOVERY-1.0.
Base product source: `da96082121e6b760e0b87884813a81eed249feac`.
Prior independent failure report: `9c1edb1fbf0684a370e1b723046216121432b0cd`.
Repaired source: `1168878146450ba4858cfa05b7bf3a4e9d8b46ea` (also verified on remote main).
This document's reporting commit is separate from that reviewed source.

## Implementation and direct result

TEMP_RECOVERY_COMPONENT: PASS. The separate
[independent review](COMMIT_KERNEL_TEMP_RECOVERY_REVIEW_2026-09-25.md) also passed:
five parent tests and17 child executions, counted separately from the direct runs.
`durable::persist_bytes` now obtains its handle/path through a private allocator:
checked serial increment, same-directory exclusive `create_new`, at most1,024
candidates. Only the open operation's `AlreadyExists` advances to another name.
Other open errors and all post-open errors propagate without repeating the save.
Exhaustion and overflow fail before hooks/write/publish. Existing stale files are
never read as recovery state, overwritten, deleted or promoted.

Writer locks, native encoding, canonical names, write/file-sync/close/rename/
directory-sync order, poison and disk-only reopen remain unchanged. Endpoint's
production code is unchanged; its appended private tests inject an ACK transaction
fault without extending the public API. Effect IDs, nonce/receipt meaning,
UNKNOWN_EFFECT, compensation and restore identifiers are unchanged.

| Actual direct invocation | Result |
| --- | --- |
| `durable::temp_recovery_tests::allocation_boundaries` |1 PASS: actual1,024 collisions, exhaustion, overflow, size precheck, OS not-found propagation, post-open AlreadyExists not retried, Unix dangling symlink preserved|
| `external::temp_recovery_tests::sender_begin_ack_recovery` |1 PASS: begin and ACK pre-rename errors, same-PID reopen/save, receipt connection, child readback, non-idempotent UNKNOWN blocks resend|
| `temp_recovery_receiver` |1 PASS:7 hook boundaries, same-PID reopen/save/reopen, second independent save, native/arbitrary stale bytes unchanged, payload-bound replay;14 fresh children replay or save|
| `temp_recovery_kernel` |1 PASS: actual R3BIN shared caller, same-PID failed write then COMMIT/REPLAY;1 child|
| `native_batch_failure_and_restart` |1 PASS: existing small correction caller smoke;1 child; no large correction campaign|

Five parent invocations,17 child test executions, all exit0. They are not22 unique
test definitions. The receiver hook points are temp-open/write/file-sync/close/
rename/directory-sync/reply. Injected hook failures occur after the named operation;
they do not establish actual disk-full, failing sync syscalls or power-loss safety.
Post-rename/reply failures restore the new canonical and replay the exact receipt;
pre-rename failures restore the old canonical, then successfully commit once.

Locked/offline narrow build, library check, clippy with `-D warnings`, changed-file
rustfmt check and staged diff check passed. Tests were listed before exact filters;
zero-test filters are not counted. One initial shell redirection used the wrong cwd
and exited1 before Cargo started; the corrected invocation is the recorded build.
The earlier defect reader's exit0 asserted broken recovery: reused RED evidence,
not a recovery PASS. The old150/600/80/60 stress runs are historical, not rerun.

## Evidence, preservation and cost

Local evidence: `artifacts/commit-kernel-temp-20260925/`.
`candidate.patch`, `commands.txt`, `identity.sha256`, `toolchain.txt`, build/check/
clippy/fmt logs and exits, and `direct/` retain exact execution and native snapshots.
`bin/` holds the four frozen executable copies; no source/model/vendor/target tree
was copied. The receiver log records stale paths, full-byte comparisons and SHA256.
Sender and kernel also compare preserved stale bytes and canonical state directly.
Original E executable and both lockfiles still match their prior hashes;
correction source is unchanged. External source hash changes only because of the
new cfg(test) block; that deliberate test addition is not an original-data change.
Existing models, Adam, memory, failed raw, fixtures and `.DS_Store` were not touched.

Build target `target/debug`:11,479,384→11,494,172 KiB allocated (+14,788 KiB), measured
around this narrow build/check. Evidence before independent review:1,198 files,
18,506,610 logical bytes, below256MiB; final scoped measurement is retained locally.
No global artifact inventory or cleanup occurred. Peak RSS and Codex usage UNKNOWN.
Root agent setting UNCHANGED/UNKNOWN; independent reviewer requested Astra High.
Model/teacher/forward/backward/optimizer/GPU calls:0. External service/operating DB
calls:0. Git publication only; local file/process tests are not network effects.

## Remaining integration boundary

PRODUCT_MEMORY_INTEGRATION: NOT_RUN. GRU_SPEC: NOT_PROVIDED; no core experiment.
Code recovery does not promote model quality, S4/S5/S6 or Goal1.
The independent kernel, correction and external endpoints still need a separately
approved product transaction design: validated proposal/effect, current dependency
closure, operation/nonce/receipt and actual memory mutation must share one durable
commit domain. Committing the kernel and then writing SQLite separately would not
provide that atomicity. External delivery follows an authorized committed intent
through an outbox/reconciliation boundary; ambiguous non-idempotent delivery remains
UNKNOWN_EFFECT. This repair grants no product-memory integration authority.

Execution identity (SHA256): unit executable
`17a7e375e2b941339f69c02bb236b32be75ecdaa5456bc64b6d9f2dfeaf8c3d2`;
external executable
`83e04c4288a1eda75e4f8c8801506c138d823e3a4b2793fdd7757d08ffe2cebb`;
durable executable
`a8ffcbafb8dc3dd017da52b199348963814399b5912f1dd02ce0b647b82e9fd1`;
correction executable
`95523658adcdb95c2ab593b40d1666d2fcc27c59a98284b8934d836ebf81537d`.
Crate lock: `57a5361c426c69ada27eaecc320a1d0516b02c75c721e9867a4093ddb88eb007`.
Toolchain: rustc/cargo1.98.1, aarch64-apple-darwin; macOS27.0 build26A428.
Only ordinary local file semantics were exercised; filesystem/power-loss behavior
outside these tests is unproven.
