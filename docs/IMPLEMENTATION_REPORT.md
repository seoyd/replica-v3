# Implementation report

MODE: IMPLEMENT  
CONTRACT: B0-CONTRACT-1.0  
TARGET: /Users/seo/Projects/Replica-v3  
SOURCE_BASELINE: local v2 6da531adebde3c4d001a313b2cdd180ae783353f; remote HEAD and contract baseline 063be980ab0c8233497dcc29fd7035f9f22074c6  
IMPLEMENTATION_STATUS: PARTIAL  
VALIDATION_STATUS: DETERMINISTIC_TESTS_VERIFIED; REAL_MODEL_VERIFIED=NO (BLOCKED_MODEL); TARGET_M4_VERIFIED=storage/CLI only; full model path=NOT_RUN  
B0_READY: NO

Workspace initially contains only the implementation instruction (42295 bytes) and empty the review instruction;
not a Git repository, no HEAD or tracked/dirty distinction. No applicable
AGENTS.md found in workspace or ancestors. v2 is read-only at
/Users/seo/Projects/Replica-v2, initial untracked .DS_Store, the implementation instruction, the review instruction,
untitled.md; tracked files clean. v1 /Users/seo/Projects/Replica HEAD
f1990c51004a7256bb6610cd7d9b5cea7641eeab has pre-existing modifications to
resources/reports/v0_84_682_parent_bound_small_replica_deliberation_final_report.md
and src/kernel/tests/core_kernel_parent_bound_small_replica_deliberation_lock_contracts_v0_84_682_test.zig,
and untracked untitled.md. All are preserved.

Host: Mac mini Mac16,10, Apple M4, 24 GB, macOS 27.0 build 26A428,
Darwin 27.0.0 arm64. Rust 1.98.0 (88d9e12ae 2026-08-18), LLVM 22.1.8,
cargo 1.98.0 (797e8a9bc 2026-08-05), aarch64-apple-darwin. Pin the installed
1.98.0 locally; do not change global toolchain. Full outputs in logs/environment.txt.

No local model or MLX installation found in standard caches, model directories,
project/download trees, or system Python 3.9.6. Model ID/revision/quantization/
license: UNAVAILABLE, not inferred. User was asked for an existing path.
User subsequently requires Rust only: select one Rust Candle Qwen2 GGUF CPU adapter.
No Python bridge or Python implementation is created. Actual load remains BLOCKED_MODEL.
Archive replica-main(260508).zip not found in workspace, Projects or Downloads;
the contract's archive hash is not independently verified. No ZIP code executed.
SQLite product runtime is bundled 3.50.2, not system Python SQLite 3.54.0.
Actual options include ENABLE_FTS5 and ENABLE_DBSTAT_VTAB; full list is in
logs/measurements.txt and logs/cli-smoke.txt. Queried PRAGMAs: WAL, foreign_keys=1,
synchronous=2 (FULL), busy_timeout=5000, fullfsync=1, application_id=1380996659,
user_version=1. These flags are not a physical power-loss test.


## 1. Implemented execution paths and user steering

All implementation and test worker code is Rust, pinned to installed 1.98.0.
A standalone Rust package was necessary because this workspace had no source.
The later user instructions take precedence: Rust only, latest installed Rust,
no external API/canned answers, and **do not install a model/runtime in this task**.
No Python bridge, model download, runtime installer, global toolchain change,
remote write, commit, push, or change to v1/v2 source was performed. Cargo fetched
only build dependencies and locked them locally; no weights were installed.

Production commands are wired: init, record, fact add/correct/restore/current/history
(including as_of), relation add, search, show, doctor, reindex, backup and restore.
The ask path validates → commits raw question → retrieves a bounded snapshot →
invokes one owned Rust model worker → validates citations → commits answer or
failure → displays only after commit. A per-DB OS lock prevents concurrent
inference without holding a DB write transaction. A committed result is replayed
without a second model call. Test doubles are only in test sources, behind the
explicit test-support binary feature. The real smoke uses LocalModel only.

## 2. Selective v2 reuse

See REUSE.md for full commit/path/symbol/license rows. Adapted canonical LEB128
boundaries; used independent typed events/checked reads and the crc32c crate.
Whole BrainGraph/policy/store modules, atomic pack-file replacement, authority seals,
MIR audits and follow-up planners were excluded. There is no v2 path dependency.

## 3. Files and dependencies

Added Cargo.toml/Cargo.lock/rust-toolchain.toml/.gitignore; 8 src Rust modules;
one SQL migration; five integration test files and one Rust child-test fixture;
one explicit release measurement/real-smoke example; five documentation files
and actual logs under docs/logs. No existing v3 source was replaced. the implementation instruction and
the review instruction were not edited. A user untitled.md appeared during the task and was left
untouched. The initially observed v2 untracked .DS_Store was absent from the final
status; this agent performed no v2 writes/deletions. Tracked v1/v2 statuses and
HEADs remained as recorded above. No private DB was opened by the implementation.

Direct dependencies: rusqlite (bundled SQLite, backup, hooks), crc32c, zstd,
thiserror, clap, serde/serde_json (ephemeral IPC only), ctrlc, candle-core and
candle-transformers 0.11.0, tokenizers, minijinja, sha2. tempfile is test/example
only. There is no async framework, HTTP API client or graph/vector server.
Final dependency versions/checksums are in Cargo.lock. Schema constants, protocol
limits, system instructions and independent fixture expectations are not invented
answers, benchmark outcomes or product fallback text.

## 4. G0–G5 and verification evidence

PLAN.md contains the required DAG and exact node states. G0–G3 are VERIFIED;
G4's Rust adapter and G5's CLI/application are implemented, with G4/G5 marked
BLOCKED for required real-model validation deferred by the user. B0_READY remains NO.

| Contract test | Actual execution / scope | Evidence |
|---|---|---|
| T01 | literal independent bytes, UTF-8/NUL, boundaries, canonical varints, malformed envelope/tag/checksum, bounded zstd + trailing frame rejection | tests/codec.rs; logs/final-tests.txt |
| T02 | separate CLI processes, exact byte/ID recovery, repeated utterances and request replay/conflicts | tests/cli.rs; logs/final-tests.txt |
| T03 | actual child SIGKILL before COMMIT, after COMMIT, before stdout ack; retry has one committed record | tests/cli.rs; logs/final-tests.txt |
| T04 | RIGHT→LEFT→restore RIGHT, history/current/as_of, late observed time, distinct contexts, stale head, concurrent winner, invalid restore/ref | tests/store.rs; logs/final-tests.txt |
| T05 | dropped derived tables, atomic rebuild, metadata/canonical corruption rejection, open-WAL backup and restore with exact rows | tests/store.rs; logs/final-tests.txt; logs/cli-smoke.txt |
| T06 | 10003 events: old evidence before 10000 distractors, repeated directions/path names, sessions/scopes, same top-k/data; lexical 1/2 vs graph 2/2 | tests/retrieval.rs; logs/final-tests.txt; logs/retrieval-regression.txt |
| T07 | missing model, Rust test-child early exit/timeout/2 MiB stderr/oversize/invalid IPC/cancel, bad citation, SQLite COMMIT failure | tests/runtime.rs; logs/runtime-regression.txt; logs/retrieval-regression.txt |
| T08 | five fixed Korean prompts and restart/citation runner implemented; **NOT_RUN / BLOCKED_MODEL** | examples/validate.rs smoke; user deferred installation |
| T09 | inert injection/SQL/shell data, no created marker, no canonical rewriting, cross-scope rejection, cyclic graph bounds | tests/runtime.rs, tests/retrieval.rs; logs/final-tests.txt + regression log |
| T10 | release M4 storage/CLI measurements completed; actual model load/first token/generation and model RSS NOT_RUN | logs/measurements.txt |

The comprehensive v3 run executed **12 actual integration tests, all passed**.
Zero-test Rust unit/doc harnesses are not counted. Subsequent FTS execution-plan
fix reran the two retrieval and three affected runtime tests, all passed. These
are repetitions of existing tests, not five additional unique tests. fmt, locked
check, clippy with -D warnings and release build passed. No v1/v2 tests/audits ran.
Manual release CLI lifecycle through restore/full doctor also succeeded.

Fault injection for the final DB failure test is a deferred foreign-key violation
created by a test-only AFTER INSERT trigger, failing SQLite COMMIT and rolling
back the candidate answer and projections. This is **not disk-full/I/O injection**.
Child SIGKILL is not a power-cut test. Runtime doubles do not verify actual model
quality, prompt-injection resistance of generated language, or causal reasoning.
The relation test checks stored `precedes` semantics; it does not establish a cause.

## 5. Model boundary

Selected adapter: Rust Candle 0.11.0 CPU, Qwen2-family GGUF. Exact installed model,
weight path/revision, actual quantization/license: UNAVAILABLE. No claim of a loaded
model or GPU inference. Runtime derives these fields from actual local metadata
and hashes when files are supplied. Original tokenizer/chat-template files remain
read-only. B0 limits: 512 new tokens, min(8192, actual supported context), 180 s
startup/load, 120 s generation; 524288 request bytes, 262144 response bytes,
65536 retained stderr bytes. Token counting uses the loaded tokenizer; evidence is
dropped from lowest rank first and excluded IDs are persisted. No external API,
cloud fallback, tools, training, hidden CoT extraction or repeated model cycle.

## 6. Actual release measurements

Host is the recorded M4/24 GB. Model excluded. Each raw/compressed case has 10020
records and 4,839,200 exact payload bytes. Fixture is synthetic and repetitive;
these figures are not a personal-data compression promise. Full raw output and
SQLite options: logs/measurements.txt.

| Measurement | Raw | Auto zstd |
|---|---:|---:|
| Batch import 10000, ms | 257.8053 | 290.8102 |
| Single FULL commit, n=20, min/median/max ms | 3.2037 / 3.9914 / 17.0543 | 3.8787 / 4.0017 / 15.0926 |
| Warm exact read, n=128, min/median/max ms | 0.0061 / 0.0063 / 0.0164 | 0.0077 / 0.0079 / 0.0318 |
| Warm lexical search, n=100, min/median/max ms | 6.8523 / 6.9562 / 8.5271 | 7.0727 / 7.1869 / 8.7037 |
| Reindex 10020, ms | 252.1975 | 261.6515 |
| DB bytes | 20,746,240 | 16,515,072 |
| WAL bytes before close | 16,558,312 | 15,223,432 |
| SHM bytes | 32,768 | 32,768 |
| Total physical DB+WAL+SHM before close | 37,337,320 | 31,771,272 |
| FTS pages incl duplicated text, already inside DB | 12,611,584 | 12,611,584 |
| Reopen + first read, ms (not OS-cold) | 77.4304 | 77.7592 |

After closing the final connection, WAL was 0 bytes in both cases (separately
reported in the raw log). Current process RSS after measurement was 27,488 KiB;
it is not GPU/shared memory and contains no loaded model. True OS-cold, GPU memory,
model load/first response/generation: NOT_RUN. No P99/99%-recall/intelligence claims.
All broad-query measurements report truncated=true because 10000 matches exceed
64 candidates; they still return bounded results after the fixed FTS plan.

An initial measurement used two-character terms and correctly returned NarrowScope.
A subsequent broad query hit the 100 ms guard due to a metadata-first SQLite plan.
The actual EXPLAIN plan and minimal reproduction identified repeated FTS matching;
CROSS JOIN now preserves FTS-first traversal and removes the full-result sort.
The budget was not enlarged and fixture size was not reduced. Initial failure,
pre-fix measurements and plan evidence remain in logs/measurements-initial.txt,
logs/measurements-before-cross-join.txt and logs/fts-plan*.txt. The temporary plan
probe source was removed after diagnosis; production regression assertions remain.

## 7. Remaining work / blockers

- **User-deferred model installation/selection:** load one supplied compatible
  local model, confirm its exact identity/license/quantization, run and inspect the
  five Korean smoke outputs, restart citations, and record actual model timings/RSS.
- Actual tokenizer/template/model compatibility and language-level evidence
  faithfulness remain unverified without that model; no substitute answer was used.
- Physical power-loss, disk-full/I/O faults, true OS-cold cache and GPU/shared-memory
  profiling were not performed; they are not reported as passed tests.

Optional payload deduplication, chat UI, embeddings, vector servers, interval-merge
engines and future architectures were not added. Their absence is not represented
as a required B0 implementation failure.

## 8. Preservation and final requirement comparison

Only the initially empty v3 source area and owned synthetic temporary paths were
written. Existing imsi documents, v1/v2 source, model files and private databases
were not changed. No checkout/reset/clean/merge/commit/push was used. Source baseline
reads were read-only; final tracked statuses matched the starting state.

After implementation, the implementation instruction was reread from start to finish and compared:

| Requirements | Implemented boundary / final status |
|---|---|
| R01, R12 | Actual M4/24 GB environment and measured logs; model and test doubles separated |
| R02 | Standalone package/lock/toolchain; no v2 build |
| R03–R05 | Exact original UTF-8 bytes, typed binary codec, bounded raw/zstd; JSON only ephemeral IPC |
| R06–R08 | Append-only corrections/restores, fixed interval/as_of semantics, one DB transaction, rebuildable projections |
| R09–R10 | One Rust local-model adapter, no output execution/external API, explicit budgets; real model user-deferred |
| R11 | Distinct relation enum; no confirmed-cause promotion; actual model semantics NOT_RUN |
| R13–R14 | Synthetic temporary tests, unchanged existing source, recorded selective reuse/license metadata |
| R15 | Real-model completion is not claimed: PARTIAL and B0_READY=NO |
| R16 | Stops at B0 scope; no additional core/UI/trainer/agent work |
| G0–G3 | Implemented and exercised through public APIs/CLI |
| G4–G5 | Production adapter/CLI implemented; actual local model and complete M4 inference path deferred |
| Minimum CLI | All required commands wired; actual ask success awaits a model, missing-model failure is tested |
| T01–T07, T09 | Relevant deterministic tests executed; mock/hardware/fault boundaries explicitly stated above |
| T08, model portion of T10 | Runner present, NOT_RUN due to user's explicit deferral |

This is an implementation report, not an independent reviewer's approval.

## 9. Reproduction commands

From /Users/seo/Projects/Replica-v3, use RUNBOOK.md commands. Exact performed gates:

```sh
cargo test --locked --features test-support -- --nocapture
cargo test --locked --features test-support --test retrieval --test runtime
cargo fmt --all -- --check
cargo check --locked
cargo clippy --locked --all-targets --features test-support -- -D warnings
cargo build --release --locked --bin replica-v3 --example validate
target/release/examples/validate measure
```

The first command was run once as the comprehensive v3 gate; the second was the
narrow regression after the measured FTS-plan correction. Earlier targeted failed
and successful attempts are retained without inflating final test counts. Do not
run `validate smoke` until the deferred model work is resumed with real local files.

## 10. Package identity

This workspace has no Git history/diff. docs/logs/package-digest.txt records SHA-256
for final source/config/tests/docs (excluding this self-referencing report and logs),
plus a digest of that sorted manifest. It also fingerprints the unchanged imsi files.

## Native Goal 1 execution (2026-09-16)

MODE: IMPLEMENT; CONTRACT: GOAL1-NATIVE-TRPP-1.0.
BASELINE_SHA: 436ed1d1cdc9efa18c3728bc75fe972b8a5fab14 (local and origin/main matched).
The preceding report describes historical B0 only. Its model deferral is superseded
by the native from-scratch task; no external weights/API are authorized. Temporary
instruction references were edited for separation, preserving historical values and
failure facts. Original reports/manifests remain available at the baseline commit.
S0: installed Rust/Cargo 1.98.0, aarch64 Apple M4 24 GiB confirmed; starting user
changes preserved, both temporary input byte hashes unchanged by index-only removal.
S1–S6: NOT_STARTED. Actual current tests/training/inference: NOT_RUN.
GOAL1_ACCEPTED=NO; INDEPENDENT_REVIEW=PENDING; GOAL1_READY_FOR_REVIEW=NO.
