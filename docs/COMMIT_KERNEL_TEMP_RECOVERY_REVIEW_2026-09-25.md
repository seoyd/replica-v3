# Commit-kernel temporary-file recovery: independent review

TEMP_RECOVERY_COMPONENT / COMPONENT_REPAIR: **PASS**.
INDEPENDENT_DYNAMIC: **PASS** for the bounded native checks below. No blocking
finding was found in the allocation repair or reviewed caller paths.
This closes the earlier same-process recovery defect for this candidate; it does
not grant product integration or broader acceptance.

SOURCE_SHA (base): `da96082121e6b760e0b87884813a81eed249feac`.
REVIEWED_SHA: `1168878146450ba4858cfa05b7bf3a4e9d8b46ea`.
Prior review/report commit: `9c1edb1fbf0684a370e1b723046216121432b0cd`.
The [prior review](COMMIT_KERNEL_EXTERNAL_REVIEW_2026-09-25.md) remains preserved:
its reader exited 0 because it asserted the defect, not successful recovery.
That RED evidence was reused and was not rerun on the old source.

## Source assessment and independent expectations

The four changed Rust files were reviewed against the prior reporting commit,
plus the direct shared callers. The only production change is in
`durable.rs`: a private allocator tries at most 1,024 checked serial increments
and exclusive `create_new` opens. Only the open result's `AlreadyExists` retries.
Other open errors and all errors after successful open return immediately;
the loop cannot repeat a business operation. Size validation precedes allocation.
The new `external.rs` code is entirely `cfg(test)` and uses the existing private
transaction for the ACK fault; no public API or production transaction changed.

Endpoint, DurableKernel and correction DurableStore retain their owned writer
lock, canonical-only decoding/validation, and poison-on-uncertain-save behavior.
Allocation neither reads nor modifies a collided file. The existing order remains
open, write, file sync, close, canonical rename, directory sync, then caller reply.
R3BIN encoding, canonical filenames, snapshot versions, effect identifiers,
receipt linkage and the non-idempotent UNKNOWN_EFFECT rule are unchanged.

Expected receiver values were independently derived from declared inputs:
initial 0 plus effect delta 7 is 7 with one receipt; replay adds nothing;
a separate delta 11 produces 18; a child-process delta -3 produces 15 and three
ledger entries. The assertions compare exact effect/receipt fields, sequence and
saved state, rather than accepting a producer-written expected-results file.
Sender input delta 9 independently requires receiver value 9, one attempt,
ACK status and the identical receiver ledger receipt after reopening. The kernel
fixture's first operation has ID 1, nonce 11 and delta 5 from initial 0; this
test checks COMMIT then REPLAY, a single ledger entry, and native validated child
readback. It does not execute the fixture's second independent operation.

## New dynamic evidence

Evidence root: `artifacts/commit-kernel-temp-20260925/review/`.
All invocations used an atomically created fresh subdirectory, the crate working
directory `crates/replica-commit-kernel`, and the immutable binaries in
`artifacts/commit-kernel-temp-20260925/bin/`. For each row the exact executable
command was `bin/<binary> --exact <test> --ignored --nocapture`, with the binary
path resolved absolutely. `VECLIB_MAXIMUM_THREADS=1` and each of
`R3_TEMP_EVIDENCE`, `R3_KERNEL_E_EVIDENCE`, `R3_KERNEL_PROCESS_ROOT`,
`R3_KERNEL_D_EVIDENCE` pointed to that row's absolute evidence subdirectory.
Each directory retains `command.txt`, `run.log`, and `exit.txt`.

| Subdirectory / binary | Exact test | New result and scope |
| --- | --- | --- |
| allocation / unit | `durable::temp_recovery_tests::allocation_boundaries` | 1/1, exit 0; real 1,024-collision exhaustion, overflow, size-before-open, OS NotFound propagation without retry, post-open AlreadyExists without retry, successful hook order, dangling symlink preservation |
| sender / unit | `external::temp_recovery_tests::sender_begin_ack_recovery` | 1/1, exit 0; begin and ACK pre-rename faults, poison, same-PID reopen/save, stale bytes, attempt/receipt linkage, UNKNOWN_EFFECT block; 1 child |
| receiver / external | `temp_recovery_receiver` | 1/1, exit 0; 7 hook boundaries: temp-open, write, file-sync, close, rename, directory-sync, reply; 14 children |
| kernel / durable | `temp_recovery_kernel` | 1/1, exit 0; same-PID write-hook failure/reopen/COMMIT/reopen/REPLAY, stale bytes; 1 child |
| correction / correction | `native_batch_failure_and_restart` | 1/1, exit 0; existing atomic batch/lock/native readback/failure-reopen/corrupt-canonical smoke; 1 child |

These are **5 distinct parent tests**, independently executed once here after
the implementer's separate executions. There were **17 actual child executions**:
14 receiver workers, 1 sender reader, 1 kernel worker and 1 correction worker.
Every child log reports exactly 1 passed test; all child executions succeeded.
Those four child helper selections and seven receiver boundary cases are not
counted as additional distinct parent tests. Parent harness times were 0.10,
0.08, 0.79, 0.06 and 0.04 seconds respectively (1.07 seconds summed, rounded).

| Required property | Independent conclusion |
| --- | --- |
| TEMP_ALLOCATION / NON_COLLISION_ERROR_PROPAGATION | PASS: bounded open-only retry; NotFound kind and raw OS error unchanged, serial advanced once, no hook |
| SAME_PID_REOPEN_SAVE | PASS: receiver, sender begin/ACK and DurableKernel perform actual native saving after reopening |
| STALE_TEMP_PRESERVED | PASS: full byte comparisons; receiver logs also record length/SHA256 for fault-created stale files; valid old R3BIN and arbitrary stale bytes ignored |
| COMMIT/ACK_ORDER | PASS: pre-rename canonical unchanged, failed operation returns Err and poisons; no Effect returned before begin persistence; reopened ACK matches receipt |
| POST_RENAME_UNCERTAINTY | PASS: rename/directory-sync/reply fault returns Err and poisons even though canonical contains new state; replay returns the same receipt with one effect |
| SENDER_RECEIVER_REPLAY / NATIVE_READBACK | PASS: same-PID replay and separate-process native readback; receiver child also saves a new effect |
| Shared correction caller | Existing native smoke PASS; post-failure successful new save was not separately tested for this caller |

The failure hooks run after their named I/O steps; they are not measured disk
write/sync failures. Real OS error testing here is at open, plus the Unix symlink
collision. This is not a disk-full, power-loss, real network exactly-once, or
actual PID-reuse experiment. Recovery beyond 1,024 occupied candidates is not
promised. Existing large I/J/M, stress and mutation campaigns were not rerun or
counted as new PASS evidence.

## Identity, preservation and cost

EXECUTABLE_HASH (SHA256):

| Reused immutable binary | SHA256 |
| --- | --- |
| unit | `17a7e375e2b941339f69c02bb236b32be75ecdaa5456bc64b6d9f2dfeaf8c3d2` |
| external | `83e04c4288a1eda75e4f8c8801506c138d823e3a4b2793fdd7757d08ffe2cebb` |
| durable | `a8ffcbafb8dc3dd017da52b199348963814399b5912f1dd02ce0b647b82e9fd1` |
| correction | `95523658adcdb95c2ab593b40d1666d2fcc27c59a98284b8934d836ebf81537d` |

LOCK_HASH: root `Cargo.lock`
`a911931fc3baca7367a34c129673640661a8fc3d41458d49e4eb744ae5454154`;
crate `Cargo.lock`
`57a5361c426c69ada27eaecc320a1d0516b02c75c721e9867a4093ddb88eb007`.
The implementation build receipt identifies Rust 1.98.1 on aarch64-apple-darwin,
unoptimized test profile. That build provenance and binaries were reused, not
rebuilt by this reviewer. The source matched REVIEWED_SHA and all ten manifest
entries (four binaries, four changed Rust files, two locks) matched before and
after execution. `dynamic-libraries.txt` records executable library dependencies.

Product source, original evidence and retained failures were read-only. Additional
before/after hashes checked the user `.DS_Store`, unchanged correction test and
kernel input fixture. These scoped checks do not claim an all-artifact inventory.
Only fresh isolated test fixtures/logs and this report were written. No product
database, model, corpus, checkpoint, dependency or toolchain was modified.

At the post-test measurement, this review's raw regular files totaled **41,847
logical bytes** (1,186 files); the entire task evidence root totaled **18,548,412
logical bytes** (2,382 files), before this report and later publication records.
The report is additional text outside that root. Both figures exclude directory
and filesystem allocation overhead. The shared 256 MiB logical-evidence allowance
has ample remaining space at that measurement. This reviewer ran no build and
created no new executable; global target growth was not independently measured
(UNKNOWN). Peak RSS and Codex token/cache usage are UNKNOWN. Requested review
setting was Astra High; actual model/effort could not be verified through a live
control and is UNKNOWN. One independent reviewer agent; no delegated subagents,
test rework or failed review test invocation.

New generation/teacher/forward/backward/optimizer/GPU calls: **0**.
External service/network effects and operating database writes: **0**.
E_COMPONENT_INTEGRITY is preserved within the reviewed allocation and native
sender/receiver/shared-caller boundary. PRODUCT_MEMORY_INTEGRATION: **NOT_RUN**.
GRU_SPEC: **NOT_PROVIDED**. CORE_TRAINING, MODEL_QUALITY, S4/S5/S6 and GOAL1 receive
no promotion from this component repair. Publication belongs to a separate
report-only commit; this reviewer did not commit or push.
