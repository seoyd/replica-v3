# Commit Kernel Rust execution — 2026-09-25

RESULT: PARTIAL for the complete A–E/product-integration roadmap.
GATE: v1.3A execution PASS; v1.3B scoped process/disk execution PASS.
These are local Rust execution results, not independent product acceptance,
model-quality improvement, or Goal1 acceptance. C/D/E and TR++ routing remain
NOT_TESTED. Earlier synthetic/Go research numbers were supplied by the user;
they were not reproduced or substituted for these Rust results.

## Source and identities

| Item | Exact identity |
| --- | --- |
| R0 fixture repair / independent narrow review | e4dc41a09c83f506579de4dfcc67214113570477 |
| v1.3A source commit | 17fe36ca5bada6411a44abc70de46991069c5597 |
| v1.3A repository tree | 099971b8a6298a55cb80ffc8fd9ad043eafd9d78 |
| v1.3A crate file/SHA256 manifest hash | be378db5e57b26292cc864e6d4d318f4d7ec26d22b83795bd9f11f32efa8cbdd |
| v1.3B source commit | d520e272dd4cd8d8a83ec7e23b3bf752eb4929c9 |
| v1.3B repository tree | 2a5682884dfaec73111113d7d61e4fea015c9c1c |
| v1.3A crate lock SHA256 | 90a95f6ddd4f17003affa5a2789bcda152852d0c8bed220e524ae9b9582e514b |
| v1.3B crate lock SHA256 | 57a5361c426c69ada27eaecc320a1d0516b02c75c721e9867a4093ddb88eb007 |
| Unchanged product Cargo.lock SHA256 | a911931fc3baca7367a34c129673640661a8fc3d41458d49e4eb744ae5454154 |
| Supplied archive SHA256 | e2ea033cd8a1ee19b9a596062bb978e9138087b18ceb34e16ef085df09d7f9f8 |
| Fixture expected SHA256 | cd5f0533762fe68da3c3da01410c87a240a224db33350c52f3d4c339984c0323 |
| Fixture actual SHA256 | cd5f0533762fe68da3c3da01410c87a240a224db33350c52f3d4c339984c0323 |

All three source commits above were normally pushed and their complete SHA
matched `git ls-remote origin refs/heads/main` immediately afterward. This report
is a later documentation commit; its HEAD is not the executable source identity.
The original archive remains in the user's Downloads directory. Original source
extraction and all run evidence are under `artifacts/commit-kernel-20260925/`.
No original models, Adam, raw, failed outcomes, native policies or user memory
were changed. User `.DS_Store` remains unstaged.

RUSTC: 1.98.1, commit48a229ceaefd4985c50990b14116b6d856af0985, LLVM22.1.8.
CARGO: 1.98.1 (797e8a9bc,2026-08-05).
TARGET: aarch64-apple-darwin / arm64.
MACOS: 27.0, build26A428.

## Implemented boundary

The supplied source is imported as `crates/replica-commit-kernel`, an independent
crate. It is not a product/training dependency and does not construct a model.
The frozen JSON fixture and canonical state/effect JSON digest are kept for
reference-protocol compatibility; they do not replace persistent user memory.
The supplied archive had no Cargo.lock. Its initial lock was generated offline;
the product lock was preserved. B adds a dev-only dependency on the existing
Replica library to use its R3BIN codec and existing patched Candle resolution.
The runtime library itself remains independent of the model library.

The original unmodified candidate compiled and passed its two integration tests
and32 CLI vectors in both profiles. New direct negatives then found:

1. Preflight trusted the manifest's digest string without hashing the actual
   ordered names. Renaming a vector and its manifest entry together was accepted.
   The fix hashes the exact LF-terminated ordered manifest and checks its count.
2. Malformed events could parse with a missing envelope and panic at apply.
   The fix rejects missing/null/extra envelopes and unknown event types during
   parsing; direct typed invalid events reject without mutation or panic.

Original failure logs are retained as `direct-original.log` (7 pass/1 fail) and
`direct-event-before.log` (8 pass/2 fail). No fixture or expected result changed.
CLI output now includes each actual final-state hash and full mismatch details.
`candidate-from-archive.patch` preserves the v1.3A source diff from the archive.

B adds `durable.rs`: a single-writer OS file lock, host-supplied codec,
full snapshot replay validation, temporary-file write, file sync, close, atomic
rename, directory sync, then reply. A snapshot includes initial state, events,
journal/receipts, final state, ledger and nonces. The tested host codec is the
existing `replica_v3::binary` canonical R3BIN codec. No new codec or JSON store
was introduced. Any uncertain save poisons the live handle until disk-only open.
Restore never promotes a newer temp or backup over a corrupt committed file.
The first version accepts a pristine initial ledger/nonce state; it is not a
migration path for unrelated existing stores. Events/snapshot bytes are bounded.

## Tests and commands

Commands ran from `crates/replica-commit-kernel`, unless noted. All result logs
and explicit `.exit` files are in the evidence root. `CARGO_INCREMENTAL=0` was
set. A used `CARGO_TARGET_DIR=../../artifacts/commit-kernel-20260925/target`.
B reused `CARGO_TARGET_DIR=../../target` sequentially. No toolchain/dependency
download, model, generation, teacher, optimizer or backward call was made.

```sh
cargo test --locked --offline
cargo test --locked --offline --release
cargo run --locked --offline -- tests/data/commit_kernel_reference_vectors_v1_2B.json
cargo run --locked --offline --release -- tests/data/commit_kernel_reference_vectors_v1_2B.json
R3_KERNEL_MUTATION_ROOT=/Users/seo/Projects/Replica-v3/artifacts/commit-kernel-20260925/mutations-final cargo test --locked --offline --test mutations -- --ignored --exact guards_are_killed_by_frozen_vectors
R3_KERNEL_PROCESS_ROOT=/Users/seo/Projects/Replica-v3/artifacts/commit-kernel-20260925/durable-process-final cargo test --locked --offline --test durable_process -- --ignored --skip process_worker --test-threads=1
cargo test --locked --offline --test direct_negative --test preflight --test reference_vectors
```

| Check | Actual outcome / evidence |
| --- | --- |
| A debug tests | 12 passed:10 direct-negative functions + identity1 + vector-replay1; `final-debug-test.log`, exit0 |
| A release tests | Same12 passed; `final-release-test.log`, exit0 |
| A debug CLI | 32 distinct vectors,0 failures; `final-debug-run.log`, exit0 |
| A release CLI | Same32 names and final-state hashes; `final-release-run.log`, exit0 |
| Manifest | All32 required names executed exactly once; no duplicates; computed name digest and frozen bundle identity checked |
| Semantic mutation | 11/11 compiled mutants produce CLI exit1 and failed vectors; `mutations-final/summary.txt`; test exit0 |
| B process tests | 3 passed, child-worker entry filtered out; `b-process-final.log`, exit0 |
| B A-regression | Direct10 + identity1 + frozen32-vector replay1 pass; `b-a-regression.log`, exit0 |
| B mutation-harness regression | 11/11 compiled semantic mutants detected on B source; `b-mutations.log` and `mutations-b/summary.txt`, test exit0 |

Zero-test library, binary and documentation targets are not counted as passing
tests. The mutation test was explicitly run; its default ignored listing is
not counted as execution. `debug-vectors.txt` and `release-vectors.txt` contain
the actual32 names and final-state SHA256 values plus the summary; byte comparison
was identical. The frozen JSON manifest is the exact full name list.

The mutation gate separately disables effect digest, trusted capability,
parent generation, policy epoch, capability epoch, object dependency,
predicate dependency, operation mismatch, nonce exclusivity, ledger sort and
nonce sort. Semantic failure counts respectively:2,3,1,1,1,1,1,1,1,4,4.
No compiler failure is counted as a killed mutant.

Direct tests separately cover signed extrema, all five epoch overflows,
no-effect/no-receipt rejection, replay versus mismatched operation/nonce,
unknown/duplicate fields, negative/overflow/float/null numeric input, empty and
duplicate suites, format/version/suite mismatch and exact manifest identity.

B prefix and suffix ran in separate actual processes and match continuous
journal/state/receipt/ledger/nonce results. Seven children received actual
SIGKILL9; seven retries ran in fresh processes:

| Kill boundary | Actual fresh-process retry | Final effect |
| --- | --- | --- |
| temp open | COMMIT | Once |
| write | COMMIT | Once |
| file sync | COMMIT | Once |
| close | COMMIT | Once |
| rename | REPLAY | Once |
| directory sync | REPLAY | Once |
| before reply | REPLAY | Once |

All seven have ledger1/nonce1. Sixteen child processes total were used by these
three tests:2 prefix/suffix,7 killed writers,7 retries. The failure test also
checks single-writer lock rejection, poisoned handles after an injected
post-rename save failure, successful disk-only reopen, and five corruptions:
truncation, missing ledger, missing nonce, missing receipt and duplicate nonce.
A valid newer temp exists during corruption checks; committed corruption still
fails closed. Raw child stdout/stderr, R3BIN snapshots and crash summary remain
under `durable-process-final/`.

One preliminary B dependency-resolution check used `cargo check --offline --tests`
without `--locked` to add the required dev-only native-codec dependency. This is
recorded as an invocation deviation, not concealed as locked execution. All
acceptance tests afterward used the resulting lock with `--locked --offline`.
No product lock or vendor source was changed. An initial format invocation had
an incorrect relative manifest path and was corrected; it was not a test PASS.

## Binary identities and limits

- A debug CLI:944eafc13c73d366dc5d64c74a6761cbefef1108292d491ab3b1bd0323c4b927.
- A release CLI:540884429598296a94a207bd03ad71187aac0c13221a6d73fce611ad78044f3b.
- B final process-test executable:46a55fd39a54deb1819ca99e49a39851c7f287e773ce2cc80f29a6b0e708cbaa,
  4517888 bytes; preserved as `durable-process-test`.
- The original value-reading test executable remains at its referenced path with
  SHA256aff1a2a21dd5bab8f534f5e790b54570c162d1c9284dec5a46319fdaf7ed6c4a.

No cleanup, model/corpus/vendor copying, FP4 experiment, GRU implementation or
training occurred. Incremental builds were disabled; only the small supplied
candidate and required mutation source were copied. Scoped build/evidence
measurements are recorded in the status document; peak transient allocation,
Codex token usage and live root model/effort setting are UNKNOWN. The requested
routing did not change the root session's configuration. R0 used one Sol/Medium
implementer followed by one Sol/Medium independent narrow reviewer; kernel
implementation and execution were performed by the root, not represented as
independent acceptance.

## Remaining protocol boundary

The supplied frozen `two_valid_operations` vector accepts two different
operations with unchanged object/predicate versions. `Kernel::apply` increments
those versions only for explicit OBJECT_CHANGE/PREDICATE_CHANGE events. This
does not define the later research's automatic write-conflict/predicate update
policy. Adding increments on every COMMIT would change the frozen semantics.
The original v1.2E/F concurrency source/vectors or explicit update rules have
been requested before C. They were not included in the supplied v1.3A archive.

NEXT_GATE: v1.3C after that protocol input is resolved. C/D/E, queue fairness,
versioned correction closure, external effect/compensation and the Replica
vertical slice are NOT_TESTED. B covers real process crashes, not physical power
loss or universal external exactly-once. No hallucination/intelligence claim
or product-version promotion follows from these results.
