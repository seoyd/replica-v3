# Implementation report

Current native contract result: **PARTIAL**. S0–S3 are verified and published;
S4's first actual5000-step run failed required heldout quality; bounded continuation
is in progress. S5 native CLI integration is implemented but its quality gate failed;
S6 remains unimplemented. See the continuation observations below. The B0 report
immediately below is historical.

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
User subsequently requires Rust only: select one Rust Candle external pretrained CPU adapter.
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
former external model library (now removed), tokenizers, minijinja, sha2. tempfile is test/example
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

Selected adapter: Rust Candle 0.11.0 CPU, external pretrained adapter (now removed). Exact installed model,
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

### S1 — implemented and executed

RV-01: `store::result` is the common canonical kind/input/scope/session verifier.
Both replay paths reject projection corruption without generation or canonical writes;
append also checks newly submitted links before returning an existing result. Terminal
success is restricted to AssistantAnswer. CLI corruption fixture emits no stdout.

RV-02: production worker uses PreparedPrompt after disabling truncation/padding only
in memory. Exact complete IDs, artifact hashes, token digest and evidence partition
are bound in PromptReceipt and independently recomputed by LocalModel at the parent
boundary. Whole low-ranked evidence drops for byte/token caps. Required-only overflow
is ContextTooSmall. Tests use a locally constructed real tokenizer, not FakeModel
for tokenization. Null/left/right truncation, padding, exact/+1, Unicode/NUL/emoji,
evidence tails/removal, oversized bytes and receipt tampering are exercised. The
pre-fix executed failure first exposed padding; silent truncation was source-confirmed
and the same final test exercises both directions at 1024. Original files are not edited.

RV-03: origin scope/session/time/snapshot SQL filters precede LIMIT; limit+1 sentinel,
unique scheduling, hop/output/visited flags and separate fetched/eligible/visited
counters. Hop uncertainty is conservative for an unvisited but possibly ineligible
neighbor. Complete short/cyclic searches can still return truncated=false. Tests
isolate filtered origins, duplicate and distinct 256/257/258 neighbors, 4/5 hops,
8/9 evidence, snapshot and the existing 10003-event lexical/graph comparison.

RV-04: startup and head each compare a single SQL observation. Two-connection
channel schedules test writer commits after that observation, with genuine watermark
and head damage as negative controls. Hooks are thread-local, feature gated and absent
from the default release; existing concurrent correction remains covered.

RV-05: an actual canonical SELECT pins the transaction before Backup::new, through
copy, doctor and comparison. Writer schedules before copy and after Done both restore
the pinned original bytes. Existing destinations never overwrite. Failures explicitly
retain an untrusted destination and report its path (an allowed failure-artifact policy),
including injected copy/validation errors and corrupt-source rejection.

Evidence: logs/goal1-rv01-red.txt through goal1-rv05-red.txt record executed failures
against baseline behavior with only extraction/scheduling instrumentation. No isolated
baseline worktree was used. logs/goal1-s1-direct.txt, goal1-s1-extra.txt and
 goal1-s1-visited.txt are targeted repetitions, not extra unique tests. Final gate:
`cargo test --locked --features test-support --test codec --test store --test retrieval
--test runtime --test cli -- --nocapture`: **22 tests passed**, none ignored (CLI 3,
codec 2, retrieval 5, runtime 6, store 6). fmt/check/clippy -D warnings and default
release bin/example build passed. Initial clippy type-complexity failure is retained;
a test-only alias fixed it. Fingerprint: logs/goal1-s1-source-digest.txt.

S1=VERIFIED within these boundaries; native-tokenizer boundary extensions remain S2.
Native training/quality/restart/quantization: NOT_RUN. No external model executed.
GOAL1_READY_FOR_REVIEW=NO; GOAL1_ACCEPTED=NO; INDEPENDENT_REVIEW=PENDING.
S0 published commit: 7d831d08bd42410f9a38836c5ef2b1671f023b4d, origin/main matched.

### S2 — implemented and executed

New responsibilities: src/data.rs and src/train_main.rs are training-only corpus/CLI;
src/neural.rs is shared reversible byte BPE and trusted role framing. Cargo.toml adds
one Rust training binary, with no new dependency. src/lib.rs exposes the own tokenizer.
Tests: tests/native.rs (2) and tests/training.rs (1), **3 actual tests passed** in
logs/goal1-s2-final-tests.txt. Tests cover byte/UTF-8 strictness, reserved markers, full
length, exact/+1 limits, complete evidence drops, malformed metadata, create-new files,
fresh-process identical IDs/roundtrip and corpus split leakage rejection. Clippy all
targets/features and default release trainer build passed. Initial crate API/serde
compile/test failures are retained in the earlier S2 logs; final passing run is distinct.

Actual release commands generated 2000 train/200 validation synthetic episodes and
trained the own BPE to 648 tokens (requested cap 4096, no artificial merges). Counts,
hashes, exact commands and local ignored artifact locations are in NATIVE_MODEL.md,
RUNBOOK.md and logs/goal1-s2-{corpus,tokenizer,tokenizer-manifest}.txt. No local user
corpus was provided or collected. DATA_SCOPE=SYNTHETIC_ONLY. OWN_TOKENIZER_TRAINED=YES;
OWN_MODEL_TRAINED=NO; TRAIN_PIPELINE_VERIFIED=NO; GOAL1_TASK_PASS=NOT_RUN.
T-N06 corpus/tokenizer lineage is implemented; model initialization/training lineage
remains pending S3/S4. S1 published 4edd62cafb92d077667155105f233ce1e88c4e3d and
origin/main matched. No S3–S6 success or independent acceptance is claimed.

### S3 in progress

S2 published 6ded741b6e994b3b43db9f0582eee850f0bcea3d; origin/main matched.
New source owns tensor-level decoder, bounded KV, masked loss, AdamW state and strict
safetensors checkpoint. CPU tiny forward/gradient/cache and fresh-process 6-step vs
3+resume-to-6 tests executed (logs/goal1-s3-direct.txt). These are numeric pipeline
checks, not memory QA or model quality. Small-profile timing probe is running; S3
is not yet marked complete. No external model/weight/API has been used.

### S3 — implemented and executed

Changed: src/neural/{transformer,checkpoint}.rs, src/training.rs, src/train_main.rs,
shared neural/lib modules; Cargo adds only already-cached candle-nn 0.11.0 and
safetensors 0.8.0 (lockfile adds direct edges). examples/validate.rs adds an explicit
own-model numeric boundary command. No external model classes initialize the native
model. Source/lineage and exact mathematical conventions are in NATIVE_MODEL.md.

Final directly related gate: **11 tests passed** (logs/goal1-s3-exit-tests.txt),
including independent operation references, finite difference, masks, real greedy
logits, KV chunks/rollover/reset/identity, malformed tensor artifacts, optimizer state,
exact fresh-process resume and cancellation after an observed real step. Clippy all
 targets/features and release trainer/example builds passed. Initial clippy migration
to Rust 1.98 as_chunks is recorded; no dependency/runtime installer was needed.

Actual SMALL probe: 9,546,432 parameters, random seed17, 2 optimizer steps, 622 input
 tokens / 26 supervised targets. Nonzero gradients and changed weight hashes observed.
Validation was 200 separate episodes, not final test; loss 6.68363942 -> 6.23209999.
This does not establish answer accuracy. Exact probe source fingerprint and environment:
logs/goal1-s3-probe-source-digest.txt, goal1-s3-probe-host.txt; probe init/run logs retained.
Final release numeric boundary execution uses that immutable initialized artifact with
current implementation: logs/goal1-s3-small-boundaries.txt (six measured lengths through
2048, largest error 1.252e-6). It is not six independent language tests.

INIT_FROM_RANDOM=YES (seed/config/hash/observed nonzero initialization).
TRAIN_PIPELINE_VERIFIED=YES (CPU numerical/state checks); OWN_MODEL_TRAINED=PROBE_ONLY.
GOAL1_TASK_PASS=NOT_RUN; REAL_NATIVE_MODEL_SMOKE=NOT_RUN; M4_CPU=NUMERIC_PROBE_PASS;
M4_METAL=NOT_RUN; QUANT_EXPORT_TEST=NOT_RUN; QUANT_DEFAULT=FP32.
S3=VERIFIED; S4 actual overfit/full training/heldout quality remains next.
INDEPENDENT_REVIEW=PENDING; GOAL1_READY_FOR_REVIEW=NO; GOAL1_ACCEPTED=NO.

### Native closure after the bounded S4 attempt

```text
MODE: IMPLEMENT
CONTRACT: GOAL1-NATIVE-TRPP-1.0
BASELINE_SHA: 436ed1d1cdc9efa18c3728bc75fe972b8a5fab14
FINAL_CODE_SHA: 23cc5b0178048eb7b23775fefb3d9282c1f91152 (last verified/published phase only)
REMOTE_BRANCH: origin/main at https://github.com/seoyd/replica-v3.git
REMOTE_SHA: 23cc5b0178048eb7b23775fefb3d9282c1f91152; MATCH=YES (actual final ls-remote)
S4_WORKSPACE: unpublished; source manifest SHA47a8e46a40fd8d59d9318a796a0aa488c380711fb2836d5a4cb6afba6624f8e9
RESULT: PARTIAL
```

S0–S3 implementation and executed RV01–05/numerical regressions are detailed above.
Current phase/trace state is PLAN.md; model mathematics, artifact paths, measurements,
diagnosis and quality limitations are in NATIVE_MODEL.md. Source manifests exclude
reports/logs to avoid self-reference. The launch source is separately reconstructible
from the recorded S3 commit plus logs/goal1-s4-training-source.patch; its two changed
source hashes matched the manifest captured before the run. The final workspace also
contains later diagnostic/evaluator code, which did not modify the running trainer.

S4 changes: src/{train_main,training}.rs add explicit restricted-environment RSS control
and corpus generation diagnostics; examples/validate.rs owns independent heldout
construction, actual neural evaluation, retrieval-only and paired-output comparisons.
No training renderer is linked into inference. All actual outputs originate from native
logits, with strict UTF-8 and provided-citation rejection; no answer repair/fallback.

Actual execution: tiny offline500-step overfit completed; SMALL random initialization
seed17 trained5000 steps,1,622,873 consumed inputs and56,407 supervised targets. Final
loss0.57465279, validation1.76776054; minimum validation1.25634503 selected step750
before test creation. The full run's final checkpoint is artifacts/goal1-small-train/final;
the selected checkpoint is artifacts/goal1-small-train/step-000750. These are not the
same artifact. Raw step/loss/gradient/weight-delta/checkpoint records, budget termination,
source identities and external time/RSS output are retained under logs/goal1-s4-*.

Final heldout200 cases: automatic counts[0,0,0,0,1], with40 per category; target FAILED.
199 outputs cited events outside the actually supplied evidence. The sole automatic
hit is also rejected by implementer semantic inspection: case198 output was
`오른쪽입니다. 수 없습니다. 수 없어 없어 알 수 확정되지 않았습니다284]`.
There is no supplied direction evidence for that case; its repetition/stray fragment
is not a readable supported answer. Semantic accepted count0/200; raw automatic1/200
is preserved. This observed false positive remains a limitation of the keyword rubric,
not an independent approval. No checkpoint was reselected after seeing final results.

Comparisons: random0/200; evidence removed1/200; value-swapped1/200 (the same invalid
automatic hit). In160 changed-value pairs,147 actual responses were identical and0
pairs had both answers correct. Actual retrieval-only top-1 value hits133/160 is a
different metric, not neural answer accuracy. Four model variants reuse200 base cases,
not800 independent questions. The final data were generated after training and read
only by the evaluation tool; they were never supplied to the trainer/tokenizer.

Validation-based diagnosis before final testing: step750 had0/25 exact validation
answers and3/25 train-format answers (only empty-evidence uncertainty). Cached/reference
logits on trained weights matched within4.053116e-6 through2048 tokens. Thus actual
binding/citation failures already occur before heldout wording or DB retrieval; passing
tensor/optimizer tests and falling teacher-forced loss did not establish useful task
learning. The fixed5000-step budget is exhausted. No endless training, threshold waiver,
test-driven answer table, external model, or fake fallback was introduced. Further data/
optimization work remains; any use of these test failures requires a new final holdout.

```text
EXTERNAL_PRETRAINED_WEIGHTS_USED: NO
EXTERNAL_MODEL_OR_API_USED: NO
RUNTIME_OFFLINE_TEST: NOT_RUN (integrated native ask); actual native training/evaluation under deny-network: PASS
INIT_FROM_RANDOM: YES (seed17, initialized weight hash9a6850f46a7bce368ad34fa996122af86d8372c3edcb2aa931dda6aa5f1d0894 and actual changed weights)
OWN_TOKENIZER_TRAINED: YES (train-only648-token byte BPE; artifact/hash in model card)
OWN_MODEL_TRAINED: YES (actual bounded run above, quality failed)
TRAIN_PIPELINE_VERIFIED: YES (CPU numerical/state correctness)
GOAL1_TASK_PASS: NO
MODEL_QUALITY_SCOPE: SYNTHETIC_ONLY; failed narrow Korean memory-QA, no general-language claim
REAL_NATIVE_MODEL_SMOKE: NOT_RUN (the required five integrated memory queries)
RESTART_MEMORY_AND_CHECKPOINT: NOT_RUN (combined native path; standalone exact checkpoint resume passed)
M4_CPU: PASS (actual numerical/training/generation execution only); M4_METAL: NOT_RUN
QUANT_EXPORT_TEST: NOT_RUN; QUANT_DEFAULT: FP32 (only implemented native precision)
MLA / MTP / FP4 / LATENT_THOUGHT / LOW_BIT_KV: NOT_IMPLEMENTED
TEMP_INSTRUCTION_DECOUPLED: YES
INDEPENDENT_REVIEW: PENDING
GOAL1_READY_FOR_REVIEW: NO
GOAL1_ACCEPTED: NO
```

ACTUAL_TEST_COUNT:34 distinct passing tests across relevant phase gates:22 storage/
retrieval/runtime/CLI/codec,11 native/trainer,1 evaluation-rubric test. Exact names and
results are in goal1-s1-tests.txt, goal1-s3-exit-tests.txt and goal1-s4-pair-regression.txt.
S2's3 tests are included in later native/trainer counts. The4 RSS-related reruns, later
rubric reruns and trained-cache length probes are not added as new distinct tests.
Ignored/zero-test harnesses are not counted. Model quality failures above are separate
from passing deterministic tests. Relevant fmt/check/clippy/release builds passed;
earlier compile/test/platform failures remain in their original logs. No unrelated
v1/v2 or long whole-project verification was run after the required S1 v3 gate.

HARD_BLOCKERS: S4 mandatory task quality failed; S5's own-model worker, generate/ask/chat,
newly stored-memory questions/fresh restarts and native failure integration are not
implemented. S6's actual INT4 export/load/comparison, final integrated M4 gate and final
phase publication are also outstanding. The historical external model adapter was
never used as this task's model or as a fallback; the continuation below removes it.
NONBLOCKING_LIMITATIONS: no Metal/GPU allocation/true OS-cold/power-loss claim; numeric
measurements do not certify semantic quality. Memory canonical schema/original bytes
remain unchanged. User instruction originals, private data and ignored artifacts stay
local; no external model installation resumed.

Verified phase publications (normal push, remote SHA matched after each):

| Phase | Commit |
|---|---|
| S0 | 7d831d08bd42410f9a38836c5ef2b1671f023b4d |
| S1 | 4edd62cafb92d077667155105f233ce1e88c4e3d |
| S2 | 6ded741b6e994b3b43db9f0582eee850f0bcea3d |
| S3 | 23cc5b0178048eb7b23775fefb3d9282c1f91152 |

S4 source/report/logs and ignored checkpoints remain in the workspace, uncommitted
and unpushed, following the failed-phase publication rule. No independent acceptance
is inferred from those four earlier commits.

Final contract comparison: reviewed the complete current instruction again. RV01–05,
own corpus/tokenizer, native decoder/KV/backward/optimizer/strict checkpoint and actual
bounded training are implemented/executed. Required task quality is FAILED; native
memory/CLI/restart and low-bit closure are UNIMPLEMENTED, not waived or renamed success.

## Continuation after the failed S4 run

The preceding closure describes the first bounded experiment, not the current
completion state. S4 is again IMPLEMENTING. The exposed final fixture remains a
regression artifact. No new final-test score or Goal 1 pass is declared.

An explicitly generated curriculum corpus has12000 train/400 validation episodes,
independent timestamp/citation IDs, shuffled evidence, varied questions and a
training-only copy stage. Own BPE has801 tokens; SMALL has9605184 parameters.
The CPU Accelerate tensor backend passed the existing numerical/gradient/cache and
resume tests before adoption. The ten-step offline probe observed20.07s wall and
1431371776B maximum RSS. The same checkpoint/optimizer continues toward the frozen
5000-step/20M-token budget; see logs/goal1-s4-v2-operating-config.txt. Diagnostics run
alongside training are not controlled latency comparisons.

The user's subsequent explicit removal instruction also removed the former external
model adapter and dependency immediately. The owned Rust worker now loads our native
checkpoint and calls the same Transformer generation loop used by evaluation.
`ask --checkpoint` verifies the parent's complete prompt/evidence receipt and exact
checkpoint revision. Random/unupdated checkpoints, invalid UTF-8, empty output and
output-limit termination cannot become product answers. Product defaults are256 new
tokens/context2048; old canonical metadata remains readable. This is implementation
progress, not S5 acceptance: final quality, five new-memory questions and restarts
still have to pass. The architecture is the specified modern GQA/QK-RMSNorm/RoPE/
SwiGLU/local-global decoder; no completed external model class is linked.

The old adapter's names and installation instructions were removed from current
source/documents at the user's request. Historical build log library names were
redacted; their observed outcomes and timings were retained. Original user instruction
files and Git history were not rewritten. The former template loader and dependency
were also removed. The truncation regression now exercises the native product loader:
serialized foreign tokenizer states and injected truncation/padding fields are
explicitly rejected, while valid own BPE preserves the complete question and tail.
Its six runtime tests passed in goal1-native-only-no-template-tests.txt.

Current removal gate:25 directly related tests passed (trainer1, CLI3, codec2,
native7, runtime6, training5, grader1); release native product/trainer/evaluator built.
See logs/goal1-native-only-{check,tests,build,dependencies}.txt. The new curriculum
resume test crosses the sampling-stage boundary in a new process and reproduces
identical model/optimizer bytes. Its first fixture was missing an evidence counter;
that failure is preserved in goal1-s4-v2-curriculum-resume-test.txt and the corrected
execution is in goal1-native-only-tests.txt. The stricter grader rejects the prior
observed malformed-citation false positive. This gate does not certify model quality.

The v2 continuation reached its explicit5000-step budget, consuming4579449 input
and183311 optimized target tokens. Final validation loss was0.19971792; the lowest
validation loss selected step4500,0.1825343128, weights SHA-256
`a9accb03c9f97fe78ca71ab4805527037b7096ff1e1c1eaf2fadf68b5d138802`.
Autoregressive validation was229/400 exact (copy42/66; task categories26/67,
18/67,39/67,62/67,42/66). No new final holdout was consumed. The resumed run took
2498.14s, maximum RSS3499409408B, footprint3149253848B (probe separate; builds and
diagnostics overlapped, so not a controlled throughput benchmark).

A validation-based data correction removes magnitude/identity/context shortcuts,
preserves original text through correction/restore, and keeps bare-copy targets
separate from product citation instructions. The balanced v3 corpus has12000/400
train/validation episodes with disjoint six-digit identities beyond the v2 ranges.
It uses the frozen own v2 tokenizer. Explicit `--extend-steps --replace-corpus`
preserves optimizer/sampler state, records previous training corpus hashes, requires
a new source fingerprint and limits the additional run to5000 steps/20M tokens.
Exact fresh-process resume, changed-corpus rejection, bounded extension and corpus
balance tests passed. No automatic training repeat was introduced. See
logs/goal1-balanced-lineage-tests.txt and goal1-balanced-corpus-final-test.txt.
The v3 ten-step offline probe observed23.69s and2992570368B RSS. Its frozen operating
configuration and exact source archive fingerprint are in
logs/goal1-s4-v3-operating-config.txt. Training is ongoing; no final quality claim.
Old training startup logs called the currently loaded tensor hash `initial_weight_hash`;
the immutable manifest's `initial_weight_hash` remains the original random model.
Current source now prints both `loaded_weight_hash` and `random_initial_weight_hash`
to remove this ambiguity; old raw logs remain unchanged.

The external-model removal is exercised by the real product: an intermediate v3
step5000 checkpoint answered14 requests under deny-network in separate CLI processes.
The seven questions cover new current facts, correction current/past, restored
current/initial past, context separation and causal uncertainty; all facts were
created after that checkpoint. Quality was4/14, so this is a FAILED diagnostic,
not S5 completion. All14 completed keys replayed the exact committed result with an
absent model path, and19 original events plus full doctor remained valid. Raw model
answers and provenance are retained in
logs/goal1-native-cli-smoke-diagnostic-5000-fixed.txt and its ignored artifact directory.
The first harness attempt omitted mandatory relation evidence and stopped before
inference; its failure remains in goal1-native-cli-smoke-diagnostic-5000.txt. That
fixture was corrected rather than weakening Store validation. Causal uncertainty
alone does not satisfy the smoke request to also state the confirmed sequence.

`validate native-failures` additionally executed five actual native-path scenarios
with the intermediate trained checkpoint: actual fresh random artifact rejection,
truncated tensor rejection, generation timeout, SIGINT after the question commit,
and deferred foreign-key failure triggered only when a validated AssistantAnswer is
committed. All five preserved input/original memory, emitted no success stdout and
replayed the same Failure with an absent checkpoint. Their raw outputs are in
logs/goal1-native-failures-5000.txt (2.01s total,303693824B maximum RSS).
The old `checkpoint_sha256` field in that log identifies the trained reference even
for the deliberately random/corrupt control; current source makes this explicit as
`trained_reference_sha256` and separately logs the selected artifact path. The
transport's synthetic-worker stderr/abnormal-exit tests remain separate unit evidence,
not a substitute for these native executions or for task quality.

The later instruction to remove every remaining former-model reference supersedes
literal name preservation in the ignored local instruction originals: the remaining
names were replaced by generic external-model wording without changing their
requirements. This is a later intentional local edit, not part of the earlier
byte-preserving S0 index removal. The files remain local/ignored; Git history remains
unchanged. The generic tensor dependency's build metadata can still contain an unused
foreign tokenizer preset symbol; it is not in our source or linked release product.
Final build-artifact cleanup must remove that metadata while retaining tested native
executables separately. No external tokenizer preset is called by the own BPE path.

The six cached generic tensor-library metadata/object files containing the unused
preset name were removed after the latest targeted build. A case-insensitive scan
of the entire current working tree, including ignored artifacts and executables but
excluding Git history, then found no remaining former-model name/content or filenames.
This does not edit global Cargo registry sources; subsequent Cargo compilation may
recreate generic dependency metadata, which must be cleared again before delivery.
The current tested native executables and own trained weights were retained.
The actual native load-timeout check also passed in goal1-native-failures-load.txt;
all six native failure scenarios passed, with five application failures replayed
unchanged. Same scenarios rerun are not counted as new independent tests.

The balanced v3 run completed its additional5000 updates at global step9500,
9317067 cumulative input tokens and364580 cumulative optimized target tokens.
Within this run that is5251027 inputs/201829 targets, including the initial ten-step
probe. Available corpus:12000 distinct episodes/3157726 framed tokens;20000 sampled
episode draws with replacement. Available unique corpus size is not a claim that
every episode was sampled. The main resumed process took2880.51s, maximum
RSS3862134784B and footprint3631632032B; probe23.69s is separate. Final safetensors
SHA-256 `a2deb1136c333ed22967ec356c3585d223ccec2742e713995afeb6859e38cf39`.
The preset minimum-validation-loss rule selected step9250, loss0.1142376273,
SHA-256 `97e91910cb8ea9dcc465b4389b2e1c4f311e37882a6fdd19a6aafe8c528a7d94`.
Actual autoregressive validation:250/400 exact, copy53/66, task categories16/67,
32/67,16/67,67/67,66/66. This is insufficient; no new final holdout was run.
Raw selected checkpoint and generation logs: goal1-s4-v3-selected-candidate.json
and goal1-s4-v3-validation-9250-full.{jsonl,txt}. Final trained lineage remains the
same own random initialization; unselected later updates are recorded as executed
computation, not silently inserted into the selected checkpoint's lineage.

The next explicit v4 experiment addresses multi-record selection (entity, value,
context, status), initial-versus-restored citations, and confirmed event sequence
with causal uncertainty. Generator and expected labels remain training-tool-only.
Its new regression and existing six training tests passed; explicit microbatch and
copy-curriculum changes preserve exact fresh-process optimizer resume and reject
out-of-budget options. Corpus seed97 has12000/400 episodes. Probe configuration,
source archive and preserved Rust binary hashes are in goal1-s4-v4-probe-config.txt.
The510-update probe includes500 copy exercises followed by ten ordinary QA updates
at microbatch8; those updates count toward the same5000-step/20M-input-token budget.
S4 remains IMPLEMENTING, S5 quality remains FAILED, and S6 remains unimplemented.

The v4 probe completed510 actual updates (500 copy-only plus ten QA) in241.19s;
maximum RSS6749798400B, footprint6358128592B. New-split validation loss decreased
2.51375464->1.26574518; this is not task accuracy or a comparison with the old corpus.
Continuation settings are frozen in goal1-s4-v4-operating-config.txt and the same
optimizer/sampler checkpoint is now being resumed toward global14250. No automatic
training restart or final-test selection occurs.

Actual CLI interfaces were also exercised offline with the frozen v3 step9250:
`generate` returned an uncertainty answer with no citations and did not create the
specified unused DB path; `chat` answered two differently worded questions about a
newly stored fact with the correct value and [event:1]. Both answers committed, prior
questions/answers were not retrieved, explicit quit ended the session and full doctor
passed with five records. See goal1-native-generate-9250*.txt and
 goal1-native-chat-9250*.txt. These interface checks do not certify the outstanding
multi-evidence task quality.

The v4 step10000 validation diagnostic generated50/120 exact responses (copy11/20,
task categories1/20,7/20,3/20,20/20,8/20). These new grounding tasks differ from the
earlier corpus; the scores are not a before/after comparison on one fixed test.
The diagnostic tool now measures teacher-forced negative log likelihood only AFTER
each actual autoregressive response. Gold tokens never enter that generation call.
Repeating the same checkpoint/input retained exactly50/120 matches. Mean first-target
NLL versus subsequent-target NLL was1.191/0.241 for current facts with distractors,
0.826/0.110 for versions, and0.952/0.373 for contexts. This identifies substantial
early response uncertainty; it does not establish the sole cause of generation
errors. Raw outputs: goal1-s4-v4-prefix-10000.{jsonl,txt}; diagnostic clippy/build
passed. The running trainer uses the separately preserved, unchanged v4 binary.

After that diagnostic build, three regenerated generic tensor-library metadata
files were removed again. The complete working-tree content/name scan, including
ignored files and excluding Git history, again found zero former-model references.

The v4 step11000 generation diagnostic scored53/120 on the same validation subset
(copy11/20; task categories2/20,8/20,5/20,20/20,7/20); its full400-case teacher-forced
validation loss was0.2457917173. The earlier step10000 training-subset diagnostic
scored65/120, distinct from50/120 validation. These observations do not satisfy the
task gate. The bounded v4 process remains active; no fresh final test has been run.

The independent evaluation tool now prepares a version2 fixture with a recorded
OS-random seed after candidate selection. Its constructor is separate from the
training corpus and exposed version1 regression constructor. Required counts and
quality thresholds remain200 cases,40/category,190 overall and36/category; distractor
identity/context, random ID versus chronological order, immutable restored text,
first-versus-current citations, and confirmed event sequence are exercised. Gold
fields remain outside ModelRequest. Actual final fixture creation/generation is
NOT_RUN. Two direct rubric/fixture regressions, clippy and release build passed;
see goal1-s4-fresh-holdout-tests-fixed.txt, goal1-s4-fresh-holdout-clippy.txt and
goal1-s4-fresh-holdout-build.txt. The first failed compilation log is retained.

At v4 step12000, generation remained53/120 validation (copy11; task categories6,
4,4,20,8 out of20 each), versus86/120 training (copy19; task categories5,13,9,20,20).
Full validation CE was0.2359288556. Evaluation source digest manifest SHA-256
`7bab310d565b4b840bd277cd2a47a5219a0a7290f5908fad615ea5602ac12dbe` and binary
hashes are recorded in goal1-s4-fresh-evaluation-{source,binary}-digest.txt.
These hashes precede the subsequent training-objective option; the ongoing v4 run
still uses its separately archived original source and binary.

A generic optional first-target optimization weight was added for a subsequent
explicit experiment. Weight1 retains ordinary CE; weightW adds(W-1) times the first
supervised token's NLL for each sequence, divided by the actual target-token count.
Raw training/validation CE and the weighted objective are logged separately. No
token value, question, seed, expected answer or runtime renderer selects the weight.
The configuration is checkpointed; old artifacts default to1. Independent scalar
CE/gradient checks include prompt/PAD zero gradients and unequal response lengths.
Two trainer unit tests and seven directly related training integration tests passed,
including fresh-process exact optimizer/sampler resume with weight7, and clippy/build
passed. Evidence: goal1-s4-first-target-objective-{tests,clippy,build}.txt. A SMALL
run with this objective has NOT_RUN; the fixed v4 training is not modified.

V4 step13000 reached95/120 training-subset exact matches but52/120 validation
(copy12/20; task categories3/20,5/20,5/20,20/20,7/20). Full validation CE was
0.2294172062. This widening gap motivates explicit evidence variation; it does not
prove a single internal cause. The new training-only counterfactual profile reuses
grounding scenes, supplies four different evidence bindings to the same ordinary
question, remaps citation IDs without cascading replacements, randomizes row order
and metadata, and preserves original/restored text equality. Three-event causal
records consistently teach confirmed chronology plus uncertainty. The independent
final renderer is unchanged and has not been executed on a model.

The direct regression checks four different values/citations for identical questions,
correct evidence references, unchanged restored bytes, sequence citation order and
disjoint train/validation fields. It passed, as did clippy/release build:
goal1-s4-counterfactual-{corpus-test,clippy,build}.txt. The forthcoming experiment
remains bounded and must use its own probe/frozen settings; no quality improvement
is claimed for code or generated data alone.

V4 terminated normally at its declared budget: global14250, cumulative19412103
input/790543 target tokens; this run added5000 updates,10358932 input and436075
target tokens. Its40000 sampled episodes include repetitions from12000 available
episodes/3263429 framed input tokens. Main continuation6166.81s, maximum RSS
7256440832B and peak footprint7315331648B; the separately recorded510-update
probe took241.19s and is included in the5000-update budget. Final validation CE
0.2325415179693259 was not the selection minimum. All probe/main saved candidates
were considered; step10500 minimized plain validation CE at0.20714910425821254,
weights SHA25679c7073e3af095e23a5bedd52921dadb7c90db20304b785989c6d254da5e5d35.
Its400 actual validation generations scored188/400: copy33/66 and five QA groups
20/67,21/67,17/67,67/67,30/66; zero generation transport failures. This is insufficient
quality, with no new final test consumed. Evidence: goal1-s4-small-v4-train.txt,
goal1-s4-v4-selected-candidate.json, goal1-s4-v4-validation-10500-full.{jsonl,txt}.

The next explicit experiment uses counterfactual seed113, train SHA256
7167d3081cae443a524059b4db511120cf5add2dbbbe4be7e1dc15a09a7180b0 and validation
SHA256a8c1fd108ef91e6834bd73e65dcb284c5894a3bcf5b3114c111f4f1a6b9135b3.
There are3000/100 base scenes with four evidence variants each, not12000/400
independent questions. A20-update SMALL probe was started from the selected v4
checkpoint with first-target weight8, no copy-only curriculum, and the same frozen
tokenizer. Its5000-update/20M-input-token total budget includes that probe.
Settings/source/archive identities and actual preflight are in
goal1-s4-v5-probe-config.txt and goal1-s4-v5-training-artifact-digest.txt.

That20-update probe completed in46.91s, maximum RSS6666878976B and footprint
6598694648B, consuming44082 input and2420 target tokens. All20 updates logged
finite gradients and nonzero weight changes. New-split unweighted validation CE
0.39841044→0.3169290049 is not generation quality. The actual checkpoint at10520,
weights SHA2566bcf31fac7706fd8924f18c506fe1081b40959d5ae45bdeaa90360c563ad3d57,
is being resumed with unchanged settings/source/data through absolute15500 or its
20M additional input budget. The probe is counted in that budget. See
goal1-s4-small-v5-probe.txt and goal1-s4-v5-operating-config.txt. Available framed
tokens are3336602 train and115670 validation before sampling/repetition.

At v5 global11000 (500 additional updates), full400-case plain validation CE was
0.16385287. Actual autoregressive diagnostics on120 episodes scored46/120 validation
(copy6/20; QA groups5,6,5,20,4 of20 each) and61/120 training (copy13/20; QA groups
6,8,6,20,8). These prefixes contain30 base scenes with four evidence variants each;
they are not120 independent questions or the final heldout test. Wrong direction
choices and incorrect event selection remain visible in the saved raw outputs.
Evidence: goal1-s4-v5-{validation,training}-11000.{jsonl,txt}. Both diagnostic processes
ran alongside training; their timings are not isolated inference benchmarks.

At v5 global12000, unweighted validation CE was0.1193614975367042. The same120
validation episodes scored63/120 (copy9/20; QA5,7,5,20,17), versus72/120 training
(copy13/20; QA8,6,5,20,20). Confirmed-sequence responses improved in this diagnostic,
while multi-record value and version selection still failed. This remains a
validation diagnostic, not a final task score or a phase pass. Raw generations:
goal1-s4-v5-{validation,training}-12000.{jsonl,txt}; checkpoint SHA256
f770421c04c13f3d39b60090eb965a1dd4c63cc701a490168ceabd865b24b83c.

V5 global13000 scored61/120 validation (copy9/20; QA5,7,3,20,17), versus79/120
training (copy16/20; QA7,8,9,20,19); plain validation CE0.12489444167023746.
The same-prefix diagnostic still shows wrong value and event selection. Checkpoint
SHA256758d6aaf8ed3b9464bc6ca872afc1f2d1302b71ebd7b72a951a30a8ff01e11ca;
raw outputs goal1-s4-v5-{validation,training}-13000.{jsonl,txt}.

A subsequent training-only `evidence-first` profile changes single-fact answer order
to cite the selected event before its direction value. Every generated token still
comes from logits in the product path; there is no answer formatter or citation repair.
It preserves input questions, evidence bytes/metadata, supported values/citations,
copy exercises and confirmed-sequence/uncertainty targets. The independent regression
checks these properties and passed; the directly touched counterfactual regression
also passed. Rust1.98 clippy initially flagged a fixed-size test iterator; it was changed
to as_chunks and clippy/release build passed. Initial failed lint output is retained
in goal1-s4-evidence-first-clippy.txt; successful logs use -clippy-fixed.txt.

Prepared v6 seed157 has20000 training episodes/5000 base scenes and400 validation
episodes/100 base scenes, with four evidence variants per base. Train SHA256
37e0c684529a74742786a12f394228ad40efb13a6dd8ddc7dc5de85a6dc0bd02, validation
SHA256ac30dacea6985503abfad1bcde314a0e322cfa3b247687246dcaf5ba17461549.
Its frozen source digest isca26983c4c7337993a86bf2fbdb6d40168e961223d1c317bc0fcbd50def3998f;
preserved binary/archive identities are in goal1-s4-v6-training-artifact-digest.txt.
Actual SMALL v6 training is NOT_RUN. Running v5 retains its separately preserved
binary/source/data/configuration; a new run requires the current budget result and
validation-only candidate selection, then its own measured probe.

V5 was deliberately stopped by the agent after its validation CE rose for three
successive saved checkpoints following the12500 minimum. This was an adaptive
validation-based decision, not a predeclared stopping rule or user cancellation;
see goal1-s4-v5-validation-stop-decision.txt. SIGINT ended at optimizer boundary13425,
statusCANCELLED/exit1, retaining a valid final checkpoint SHA256
069db87393acb0e773c1fe5d5142b1e3dce01959b4f1c85f91732552248d9183.
It used2925 additional updates,6474400 input and355696 target tokens, including the
20-update probe;23400 draws from12000 episodes/3000 base scenes are not23400 unique
questions. Main4024.74s, maximum RSS7274741760B, footprint7311514128B; probe46.91s
is separate. The5000-update maximum was not exhausted.

Minimum plain validation CE over all saved v5 candidates selected12500,
CE0.10438712087039331, weights SHA256
b3a455f15c23efabe1bc3614500efef6024d260d060961a006aefa36fd6df2a3.
Full validation generation221/400: copy31/64 and QA18/68,23/68,20/68,68/68,61/64.
This is insufficient; no new independent final fixture was created. Evidence:
goal1-s4-v5-selected-candidate.json and goal1-s4-v5-validation-12500-full.{jsonl,txt}.

V6 then began its20-update probe from that selected checkpoint. Before training,
goal1-s4-v6-probe-config.txt declared a new candidate rule: full400-case validation
generation at probe end, each1000 additional updates and termination; maximize the
five QA groups' macro-average exact accuracy, then lower plain CE and earlier step
for ties. Copy tasks are diagnostic. Two consecutive scheduled generation evaluations
without an improved best QA macro-average trigger an optimizer-boundary stop. The
maximum remains5000 additional updates/20M input tokens, including the probe.
The trainer's min-CE best.json is a loss diagnostic; it does not override the stated
generation-based candidate selection. Earlier v1–v5 selections remain unchanged.
No final-test output informs this rule and no required acceptance threshold is lowered.

The v6 probe completed20 actual updates in46.39s with maximum RSS7009402880B,
footprint6940793096B, consuming44143 input and3116 target tokens. Every update
logged finite gradients/nonzero weight changes. New-corpus plain validation CE
3.46425379→1.7211333301 does not measure task success. Checkpoint12520 weights
SHA256402850bfd1b6b2b6a0d2bdc6bdb3d07e584e22d1ecc6257b68144a3c7e3b6e09.
Full validation generation66/400: copy30/64, QA0/68,0/68,0/68,0/68,36/64, giving
QA macro-average0.1125. Thirteen invalid/incomplete UTF-8 outputs were rejected.
These failures remain in goal1-s4-v6-validation-12520-full.{jsonl,txt}; they were not
converted to fallback answers. No quality PASS is claimed at this early format-learning
point. Continuation now uses the frozen goal1-s4-v6-operating-config.txt; actual
available framed tokens5643338 train/112772 validation before repetitions.

The first scheduled v6 generation evaluation at13500 scored205/400: copy29/64,
QA18/68,27/68,13/68,68/68,50/64, macro-average0.5268382352941177. No generation
failures occurred. This is the current best of the predeclared generation candidates
(probe12520 and13500); task quality still fails. Plain CE0.15308541 is a separate
diagnostic. The next scheduled original-question evaluation is14500.

A training-tool-only oracle paraphrase diagnostic then replaced QA0/QA2 questions
with known training wording, retaining already-explicit entity/context, exact evidence,
expected answer and provided IDs. It accepts no expected value or citation ID in its
question helper, and rejects absent or partial numeric targets. The product worker and
independent final evaluator do not call it. Original questions and actual generated
questions are both logged; oracle_question_ablation=true excludes these scores from
candidate selection and task acceptance. Its direct unit regression, clippy and build
passed; the initial test filter matched zero tests and is not counted as verification.
Actual one-test execution is in goal1-s4-question-ablation-test-fixed.txt.

At the same frozen13500 checkpoint, original QA0/QA2 scored31/136 versus41/136 with
known wording (24/68 and17/68). All136 pairs retained identical evidence/expected/
provided IDs;26 stayed correct,15 improved,5 regressed,44 outputs changed. Rephrasing
alone therefore did not resolve the grounding failures. This is diagnostic evidence,
not a higher reported task score. Raw outputs and pair checks are in
goal1-s4-v6-question-ablation-13500.{jsonl,txt} and
goal1-s4-v6-question-ablation-comparison.json. Diagnostic source digest SHA256
547c4b6b6f62f5aaaab14bebbc5abcba04d4420daaced2ed12727ba778214ecf is separate from
the unchanged running v6 trainer/archive.

The second scheduled v6 evaluation at14500 scored228/400: copy30/64 and
QA36/68,27/68,17/68,68/68,50/64; zero generation failures. QA macro-average is
3218/5440=0.5915441176470588, compared with13500's2866/5440. These exact integer
numerators express the same predeclared macro-average over four68-case groups and
one64-case group; they are not extra correct cases. It is a strict improvement, so
the consecutive non-improvement count is zero and14500 is the current candidate.
Plain CE0.14638046 is retained separately. See goal1-s4-v6-validation-14500-full.*.
The next scheduled original-question evaluation is15500; the independent final test
remains unused and acceptance thresholds remain unchanged.

At15500, the third scheduled v6 evaluation scored210/400: copy22/64 and
QA32/68,29/68,19/68,68/68,40/64, with zero generation failures. The QA macro-average
3048/5440=0.5602941176470588 did not exceed14500's3218/5440. The predeclared
consecutive non-improvement counter is therefore one; the next scheduled evaluation
is16500. Plain CE0.1647298240 remains diagnostic. Raw outputs are retained in
goal1-s4-v6-validation-15500-full.{jsonl,txt}; no new final test was consumed.

A separate training-only `record-copy` corpus profile now replaces copy-family
auxiliary targets with the complete selected current record's original bytes and
citation. It also varies some training questions about the earliest pre-restoration
record, retaining their original supported answers and historical citations. Main QA
answers, all source records, and validation QA wording remain unchanged. The direct
regression `record_copy_auxiliary_targets_keep_source_bytes_and_temporal_qa` passed
(one test, goal1-s4-record-copy-corpus-test.txt). This is data preparation code only;
no model has yet been trained on this profile, and the running v6 trainer remains its
previously frozen binary. No product answer formatter or quality claim was added.

The auxiliary-only preparation was frozen under v7 (seed191,20000/400 episodes) but
was NOT trained. A subsequent read-only decomposition of the15500 validation outputs
found QA0 correct citations67/68 but correct values33/68:35 responses selected the
right event while giving the wrong value. QA1 had37 correct citations/50 correct
values and QA2 had35/31; exact scores remained32/68,29/68,19/68 respectively. See
goal1-s4-v6-validation-15500-error-components.json. These are overlapping error
components, not additional successful cases or evidence of product acceptance.

Before any new training, the record-copy revision was therefore extended to ordinary
single-fact QA: its supervised target now contains the supporting original sentence
(entity, context and value together), followed by the same historical/current citation.
Chronology/uncertainty targets and validation QA questions remain unchanged. This
is generator revision v8; the untrained v7 artifact/source snapshot remains preserved.
The product decoder still predicts all output tokens from logits with no answer repair.

Revision v8's direct regression `record_copy_targets_keep_source_bytes_and_temporal_qa`
passed (one test); targeted clippy, release build and diff whitespace checks passed.
The earlier auxiliary-only test is the predecessor of this same regression, not an
additional independent test. Logs: goal1-s4-record-copy-full-target-{test,clippy,build}.txt.
Prepared v8 seed191 has20000 train/400 validation episodes (5000/100 base scenes,
four variants each); train SHA2561bd03e69e03f3f74f4812bcdd8f0e1741117976dd875fa708ad39168bd041295,
validation5033c05420da336f8b234d420e757bfd144fcefbbdb6a2c8d284d844cf1c908c.
Source manifest SHA256ffb75e3cc9c9c1cde54206f1b35bcdda82b9e1ac5c10e68dc641cb4d99938ab2,
preserved binary6d81bfaf204f0359e0738e46a94f1e103179ef801982937ec9ceda93418d6c5e;
full identities are in goal1-s4-v8-training-artifact-digest.txt. Preparation is not
training: no update or quality result on v8 is claimed at this point. The installed
compiler was reconfirmed as Rust1.98.0/aarch64-apple-darwin and Cargo1.98.0.

The scheduled v6 step16500 evaluation scored211/400: copy24/64 and
QA36/68,31/68,9/68,68/68,43/64; zero generation failures. Its QA macro3035/5440
did not exceed14500's3218/5440, the second consecutive scheduled non-improvement.
The predeclared stop rule was applied via SIGINT to the owned trainer, ending at
optimizer boundary16553 with CANCELLED status. This is4053 additional updates,
9116218 input tokens and627465 target tokens in v6, including its20-update probe;
32424 actual episode draws from20000 episodes/5000 base scenes. Terminal cumulative
input24838251/target1308288, validation CE0.1515147931; weights SHA256
a31a2978e82c4d0b4d5d8adf458334e69643c79de8d90c7de3927553779e83e4.
Main continuation elapsed5764.37s, maximum RSS7494729728B, peak footprint7420615280B;
the probe's46.39s and separate resource measurements remain recorded above.
This was an intentional validation stop, not budget completion or user cancellation.
The required terminal generation evaluation is running before candidate selection.
See goal1-s4-v6-validation-stop-decision.txt and goal1-s4-small-v6-train.txt.

Terminal v6 generation scored215/400, copy24/64 and QA30/68,29/68,12/68,68/68,52/64,
zero generation failures; macro3108/5440=0.5713235294. Applying the declared rule to
all six observed candidates selected step14500 (228/400, macro3218/5440), weights
166c58986c4f408da383ac7ea98f2b7d5996e0e20f1cfe9c164e36eb4210fc93; cumulative
input20220437/target991264, sampler12638071737532215433. Full candidate observations
are in goal1-s4-v6-selected-candidate.json. S4 quality is still insufficient; no new
independent final fixture, completed-phase commit or push has been made.

The v8 probe completed20 real updates through14520:46128 input/4817 target tokens,
51.46s, maximum RSS6675300352B and peak footprint7027497296B. All recorded gradients
were finite and weight changes nonzero. Plain validation CE on the new full-record
targets fell4.91431155→2.4847969999; this is not a task-quality result. Available framed
tokens before repetitions are5936389 train/122804 validation. Checkpoint weights
78c2a12ba019cf43994a785b068ed0e66331e9ce00a175afa45073dbd1829fed retain optimizer/
sampler; cumulative input20266565/target996081. Probe full generation scored9/400:
copy0/64, QA0/68,0/68,0/68,0/68,9/64, macro0.028125. Ten invalid UTF-8 outputs and
one generated control token were explicitly rejected. Raw failures are retained in
goal1-s4-v8-validation-14520-full.{jsonl,txt}; no fallback or quality PASS was recorded.
After this actual resource probe, the bounded continuation uses the frozen settings
in goal1-s4-v8-operating-config.txt, including probe work in the total budget. The next
scheduled original-question full validation is15500. No new final test has been used.

The first scheduled v8 generation evaluation at15500 scored86/400: copy3/64 and
QA0/68,14/68,1/68,13/68,55/64; zero generation failures. Macro1383/5440=0.2542279412
exceeds the probe's153/5440, so15500 is the current v8 candidate and the declared
consecutive non-improvement counter is zero. Plain validation CE0.1833399122 is
diagnostic only. This new whole-record format is not directly comparable with v6's
short-answer exact score; neither run qualifies the task.

Read-only component checks retained all68 examples per single-fact QA group,
including malformed responses. For QA0, correct subject/citation/value counts were
1/61/33; for QA1,21/41/45; for QA2,6/37/32; for the single-record QA3,13/68/68.
Thus copying the full sentence has exposed incorrect subject names/numbers even
when its value and event are right. Training prefix counts across ordinary single-fact
QA were3216/3340/3324/3456 for the four generated names, not a single-name corpus.
These component counts overlap and do not replace exact or semantic acceptance.
Raw outputs: goal1-s4-v8-validation-15500-full.{jsonl,txt}; decomposition:
goal1-s4-v8-validation-15500-error-components.json. Continue the previously frozen
bounded run; the next full original-question evaluation is16500. No final test used.

The second scheduled v8 evaluation at16500 scored87/400: copy3/64 and
QA0/68,3/68,5/68,16/68,60/64, zero generation failures. Macro1404/5440=0.2580882353
is a strict but small increase over15500's1383/5440, so the declared non-improvement
counter remains zero and16500 is the current v8 candidate. Plain CE0.1811850063
does not establish task quality. No final test or phase acceptance follows. A separate
400-episode training-prefix generation diagnostic was started on this same checkpoint
to compare observed training behavior; it is not eligible for candidate selection and
does not represent all20000 training episodes. The next scheduled validation is17500.

The400-episode training-prefix diagnostic at16500 scored107/400: copy11/64 and
QA8/68,8/68,7/68,9/68,64/64, with zero generation failures. Failure therefore also
occurs on these training examples; this observation alone does not isolate a cause.
For single-record QA3, mean teacher-forced first-target NLL was1.4856133 versus
0.0006042 over the remaining target tokens; validation gave1.4233570 versus0.0519951.
These diagnostic NLLs are computed only after actual generation and supply no answer
tokens to that generation. See goal1-s4-v8-training-16500.* and
goal1-s4-v8-first-token-diagnostic-16500.json. No training score enters selection.

A new direct regression, `masked_first_response_learns_context_and_generates_without_future_labels`,
uses the real tiny transformer, the trainer's masked loss and Adam, and actual cached
generation. Four prompts have an identical ending but different earlier context cues;
the cue lies before the local window.200 real updates reduced first-target NLL
5.5729113→0.020560432, and all four prompt-only generations selected the correct first
token. The test passed in5.43s (goal1-s4-first-response-context-regression.txt).
It checks this numerical learning path, not SMALL Korean quality or generalization.
Only a test was added; the currently running v8 production binary/source snapshot,
objective, data, budget and candidate-selection rules are unchanged.

The subsequent clippy check initially found an unused import left by the native-worker
test migration and Rust1.98's constant-chunk iterator lint in the existing first-target
scalar regression. The orphan import was removed and that test iterator changed to
`as_chunks`; neither changes production behavior. The scalar loss/gradient regression
was rerun successfully, and clippy then passed. Both initial lint failures and corrected
results remain in goal1-s4-first-response-context-clippy{,-fixed}.txt; the repeated
scalar regression is not counted as a new independent test. No full test suite ran.

The v8 step17500 evaluation scored81/400: copy6/64 and QA0/68,0/68,4/68,18/68,53/64;
zero generation failures. Macro1253/5440=0.2303308824 is below16500's1404/5440.
The declared non-improvement counter is one;16500 remains the candidate, and18500
is the next scheduled full evaluation. Plain CE0.1831025870 is separate. No final
holdout or phase-quality acceptance was claimed.

A training-only `entity-cue` profile (revision v9) was prepared for the observed
first-token failure. It changes copy-family auxiliary episodes to a neutral question
and a one-name target; each four-variant group includes all four names in its evidence.
The question supplies none of the labels. All ordinary QA remains identical to v8.
The direct regression `entity_cue_auxiliary_pairs_require_evidence_and_preserve_ordinary_qa`
passed (one test,0.68s), followed by clippy and release build; logs are
goal1-s4-entity-cue-{corpus-test,clippy,build}.txt. The running v8 binary/data are unchanged.

Prepared v9 uses seed191 and20000/400 episodes. Exact comparison with v8 verified
all16668 ordinary train and336 ordinary validation episodes unchanged. Auxiliary
labels are balanced:833 train and16 validation episodes per name. These are the same
ordinary QA examples, not a new independent corpus. See goal1-s4-v9-ordinary-qa-identity.json.
Train SHA2563ac32fca9d954f98f9b3a40c785d939010ee16ddc0bb91a73c49e7269391957a;
validation572b0d9d797feb2c31fa8713566853aff91ae6b994d736c399e8bad0cb3fb004.
Source manifest f8ae00f7b7363a765213d5b193254ad897a3eb970bc708d09dd9f50062e582e6,
preserved binary37ec1f4e30baba063de7e4df8d15a074f14cdb41a560c0505788031d925e7aab;
archive and full identities are in goal1-s4-v9-training-artifact-digest.txt. No v9 model
has been trained yet; the next experiment's parent/configuration is not yet frozen.

At18500, v8 scored96/400: copy7/64 and QA1/68,10/68,2/68,19/68,57/64;
zero generation failures. Macro1481/5440=0.2722426471 exceeds the previous best
1404/5440, so18500 becomes the current candidate and the predeclared consecutive
non-improvement counter resets to zero. Required task quality is still unmet.
The same frozen run continues to its19500 maximum; there is no automatic extension.
Raw outputs are in goal1-s4-v8-validation-18500-full.{jsonl,txt}.

The user's training-accuracy diagnostic now takes priority over another broad-corpus
run. At v8 step16500, actual generation on the first400 training episodes was107/400,
versus87/400 validation. Low training accuracy motivates a32-QA memorization check;
it does not identify a specific implementation defect by itself. No v9 training ran.
The verified owned v8 process received SIGINT and saved an optimizer-boundary final
checkpoint at19271 (CANCELLED), not the planned19500. Actual additional updates4771,
input11274010 and target1149454, including the20-update probe. Main run7019.95s,
maximum RSS7550763008B, peak footprint7414242024B; probe51.46s is separate. Final
weight-file SHA256611a277296d6e474a7fe841821ac3c04137686d0801655be5f87055b4d45efac.
Terminal validation CE0.14228163735 is not a generation score; terminal full generation
has not yet run. Best observed scheduled generation remains18500's96/400, not a pass.

`replica-train corpus subset` reuses the existing validated loader/writer to preserve
32 ordinary QA verbatim from each existing train/validation split. It excludes copy
auxiliaries, records parent hashes, rejects invalid counts and existing destinations,
and never changes product inference. A direct subset regression passed (0.67s), then
clippy and release build passed. The existing independent shifting/masking/Adam
reference regression also passed. Evaluation now records answer-tokenizer roundtrip,
training/generation prompt-ID equality and teacher-forced token accuracy after the
actual autoregressive result; these diagnostics never supply gold tokens to generation.

The SMALL32-QA run uses the frozen v8 QA bytes and own BPE801 with9,605,184 parameters,
random weights (same recorded seed17 initial hash), and zero Adam state. A zero-update
setup checkpoint retains tokenizer lineage; no weight training on v2 occurred. Explicit
corpus replacement selects the32-QA set. The real trainer and generator are unchanged.
The predeclared1000-update cap,20-update probe, LR/masking/batch settings and exact
answer-plus-EOS acceptance rule are in logs/goal1-s4-qa32-config.txt. Source/binary
identities are in goal1-s4-qa32-artifact-digest.txt. This is a memorization diagnostic,
not independent task acceptance; stable training-set exact-match results are pending.

SMALL32-QA memorization completed without a model/optimizer/decoder implementation
change. Actual whole-answer plus citation plus EOS exact matches: random0/32,
20 updates0/32,100→10/32,200→27/32,300→28/32,400→32/32,500→32/32. The predeclared
two-successive-checkpoint rule triggered SIGINT; final state508 was saved with real
Adam/RNG. A new process loaded that checkpoint and again generated32/32 exact
training answers, zero generation failures. All32 actual final outputs were inspected.
Separate validation QA scored0/32, zero generation failures; this is expected evidence
of memorization without demonstrated generalization, not a task-quality pass.

Final all32 teacher-forced train CE0.0003541936956034617; latest minibatch CE
0.00034103356 is a different statistic. First target correct32/32, exact train/inference
prompt IDs32/32, tokenizer roundtrip32/32. These diagnostics were computed only after
each real generation. Actual508 updates used1217005 input and106609 supervised target
tokens,4064 sampled episodes from32 available (9593 unique framed tokens). Probe25.90s
and resumed training596.79s were measured separately; main peak RSS4701061120B,
footprint4594159720B. Checkpoint file hash
fdfadcb740e49bfdf62f65e5a2414f54c5010c2c8d46543732dd9b7f5d1bad1f.

This checks the actual SMALL architecture, shared forward/cache/generation, real
masked training and tied weights from random initialization. It does not prove every
possible training bug absent. Diagnostic LR0.001/warmup20 differs from broad v8's
LR0.0003/warmup100; neither tokenizer nor network structure was changed. Existing
large-corpus quality remains inadequate, and no v9/broad-corpus run has restarted.
The next read-only diagnostic distinguishes membership of the20000-example training
pool from actual sampled exposure at the step16500 training-prefix evaluation.

A read-only `sampling-exposure` diagnostic reuses the model's actual RNG and recorded
start/end states. It verifies unchanged run config and corpus, replays sampling,
and rejects a final RNG-state mismatch. This is deterministic reconstruction checked
against saved state, not a historical per-draw execution log. The existing independent
curriculum/resume fixture now checks copy-only selection, real total draws across the
boundary, and rejection of a deliberately altered final RNG state; it passed in2.11s.
Clippy/build passed. Identities: goal1-s4-sampling-exposure-artifact-digest.txt.

For v8 start14500→16500,16000 draws from20000 episodes left9045 unselected. Among
the400 episodes previously scored as training-prefix,189 hadzero draws,145 hadone,
51 hadtwo,14 hadthree, andone hadfour. Actual generation was55/211 on selected
examples and52/189 on unselected examples, so exposure count alone does not explain
the failure. Calling107/400 the training-prefix score is accurate; treating it as
accuracy on400 repeatedly learned examples would be inaccurate. At the v8 terminal
19271,38168 draws still left3002/20000 unselected (63/400 prefix). In the32-QA diagnostic,
all32 episodes had103–150 draws, total4064. Three replays matched their saved RNG states.
Raw counts and joins: goal1-s4-v8-sampling-exposure-16500.json,
goal1-s4-v8-training-16500-exposure-accuracy.json,
goal1-s4-v8-terminal-sampling-exposure.json and
goal1-s4-qa32-terminal-sampling-exposure.json. No broad-corpus weight training restarted.

The remaining v8 terminal validation was executed after the priority memorization
check. Step19271 scored126/400: copy7/64, QA15/68,11/68,10/68,23/68,60/64; no generation
failures. Macro1964/5440=0.3610294118 selects the terminal checkpoint under the original
v8 candidate rule, including early terminals. This remains far below required quality.
All six actual candidates and the selected manifest are retained in
goal1-s4-v8-selected-candidate.json; no new final test or publication occurred.

After the real SMALL32-QA pass and exposure audit, only a100-update auxiliary diagnostic
was declared on the already prepared v9 corpus. It tests the observed first-token
weakness with direct evidence-name targets and first-target weight8, retaining the
existing model/Adam, LR0.0003/warmup100, batch8 and all ordinary QA bytes. There is no
broad-corpus training restart. An actual20-update resource probe is included in the100
cap; predeclared auxiliary validation gate58/64 exact outputs precedes considering any
separate mixed-QA experiment. This diagnostic stops after100 updates even if the gate
fails. Config/parent/source/data/budget are in goal1-s4-v9-cue-probe-config.txt. Results
remain pending; memorization success is not converted into a Goal1 quality pass.

The100-update cue diagnostic's start scored119/400: auxiliary0/64, unchanged ordinary
QA counts15/68,11/68,10/68,23/68,60/64. The20-update resource probe scored81/400:
auxiliary26/64, ordinary0/68,0/68,0/68,0/68,55/64, withtwo generation failures. This
shows interference with the old response format and is not a successful candidate.
The remaining80 updates ran under the original100-update cap; terminal generation
is being measured before applying the predeclared58/64 auxiliary continuation gate.

The cue diagnostic stopped at the declared100-update cap,19371; added138565 actual
input tokens and1600 targets across800 draws. Main69.52s plus probe34.82s; main peak
RSS2050736128B, footprint1875675440B. Artifact SHA256
a7183bbd3dbf6978748687e7ba11b7fb62412a5d25581812e96d07f30526c0cd.
Terminal held auxiliary generation64/64 with EOS passed the predeclared58/64 gate.
Ordinary QA was0/336 with11 generation failures, because most answers stopped after
the name. The measured first-target NLL for unchanged QA0/1/2 was approximately
0.000183/0.000266/0.000216, QA3 0.0341, while QA4 deteriorated to11.1656. This supports
specific subject-name learning plus response-format interference, not full-task success.

A separate500-update maximum mixed-QA experiment was then explicitly declared to
restore original answers while retaining first-target weight8. Its source/data/model
remain unchanged; initial copy-only curriculum is disabled. The20-update probe is
inside its500-update budget, and a predeclared comparison with the pre-cue v8 macro
controls early stopping. Configuration and observations are recorded in
logs/goal1-s4-v9-mixed-probe-config.txt; no full-goal or independent-quality claim is made.

The mixed v9 run reached its500-update cap at19871, adding1147238 input and100677
target tokens across4000 draws, including the20-update probe. Main835.29s plus probe
55.06s; main maximum RSS7248887808B and footprint7247288848B. Final weights
1708fbee7152b5db658686f6ddd9a33bffcf8acea1ac45200d3ff91da6bdfe4a.
Actual original-question validation:19391→107/400(copy58),19500→198/400(copy64),
19750→219/400(copy64),19871→211/400(copy64). The two scheduled observations exceeded
the pre-cue macro, so the declared early-stop condition did not trigger. The cap did;
there is no automatic extension. One premature terminal evaluation failed before the
final artifact was published; its failure is retained as
logs/goal1-s4-v9-mixed-validation-final-before-checkpoint.txt. The later real evaluation
loaded the published artifact; no failed attempt was counted as a success.

Under the predeclared ordinary-QA macro rule,19750 is selected: QA13/68,22/68,5/68,
54/68,61/64; macro2541/5440=0.4670955882, zero generation failures. All candidate
states and raw scores are in goal1-s4-v9-mixed-selected-candidate.json. This is still
below the unchanged quality requirements, and no fresh final test was consumed.

A read-only field decomposition of19750's272 QA0–3 outputs preserves all68 examples
per category, including unparseable failures (all actual rows parsed here). Target
name is correct68/68 in each category. QA0: number30/68, context68, value29, citation68.
QA1: number61, context68, value37, citation41. QA2: number58, context21, value35,
citation37. QA3: number54, context68, value68, citation68. Thus first-name learning
transferred, but number copying and multi-record context/value selection still fail.
No claim that a tokenizer/optimizer bug was found or fixed is made. The training QA2
pool does include1104/3332 explicit distractor-context questions; complete absence of
such training examples is not the cause. See goal1-s4-v9-mixed-validation-19750-components.json.

A training-only `field-cue` profile now reuses `entity_cue` and all existing records to
supervise name, identifying digits, context string, and current evidence value directly.
Every ordinary training/validation QA remains byte-for-byte unchanged. Name auxiliaries
retain the verified wording. Fields rotate across all four existing record layouts,
rather than coupling one field permanently to one layout. Value questions use the
stable numeric identity so each quartet has the same question and four different
supported direction answers. Number/context copying can use the explicit requested
identifier; value answers are absent from the question and require evidence.

The initial direct regression failed because rotating subject names also changed the
value questions within a quartet. That preparation defect was fixed before any v10
training. Both the failed test and corrected runs are retained. The strengthened
regression covers all16 field/layout pairs in both splits, supported targets, unchanged
ordinary QA, intact evidence metadata, and same-question/different-value dependence.
It passed in2.49s, followed by clippy/build. Logs are goal1-s4-field-cue-{test,test-fixed,
coverage-test,clippy,build}.txt. This is one evolving regression, not three independent
tests. There is no product inference hook, lookup answer, or model-architecture change.

Actual v10 preparation (seed191,20000/400 rows) verified ordinary QA deep equality:
16668 train and336 validation unchanged. Training auxiliary name836, number832,
context832, value832; validation16 per field, four examples of every field/layout
pair. Corpus train14ddbd7c3e9a499a0dd198715bf2917ddff28c6fc2258da253da3e9a8e521ca0;
validationf8d18fe3f6bd2b14045218139eafabec41486420779295fe1273a12c8d697864.
Exact identity/coverage is in goal1-s4-v10-ordinary-qa-identity.json. Frozen source
51a23d328c9711445e188c19fbc644f9a195001ee6fcacc38ec4c9156f2a51d7 and preserved binary
47bdc356e140c7d397e7f6ca91bea61eb228d1b16b20c7ae7ac683f872ed867b are recorded with
the source archive in goal1-s4-v10-training-artifact-digest.txt.

A separate500-update auxiliary-only diagnostic was declared from the selected v9
step19750, including a20-update resource probe. Each field must reach15/16 exact held
auxiliary answers before considering any separate mixed-QA run; failed fields cannot
be averaged away. Ordinary QA and final heldout requirements remain unchanged.
The precise cap/source/data/parent/config/gate is in goal1-s4-v10-field-probe-config.txt.
No v10 result or quality pass is claimed yet.

The v10 field20-update probe completed in30.43s, maximum RSS2064203776B and footprint
1999915480B. It consumed28567 input/549 target tokens with finite gradients and real
updates. Weighted objective and plain CE are recorded separately; pre-clipping norm
55.3424 at the final probe step is not a claim of unclipped execution. Actual start
validation171/400(copy16), probe107/400(copy4), no generation failures. The low probe
scores are retained; no acceptance is inferred from teacher-forced loss. Per-field
counts are saved as goal1-s4-v10-field-validation-{start,probe}-fields.json.
The remaining480 updates use the exact saved optimizer/RNG and original cap; operating
facts are frozen in goal1-s4-v10-field-operating-config.txt. Terminal field gates and
ordinary QA generation remain pending.

The v10 field run reached its500-update cap at20250. Added714533 input tokens and
14322 targets across4000 auxiliary draws; main273.67s plus probe30.43s. Main maximum
RSS2340536320B, footprint2029324784B. Terminal weight file
307864aaeb32d2997e09c5f4fee508233058933fbf3b8f958de1754c76706e92.
At20000, actual held auxiliary scores were name0/16, number3/16, context0/16, value4/16;
at20250: name0/16, number0/16, context0/16, value9/16. The terminal full400 had9 exact
answers and4 generation failures, all ordinary QA incorrect. The predeclared field
gate FAILED. This checkpoint will not be continued into mixed QA, and no automatic
longer run is authorized by the diagnostic. Raw outputs and complete per-field counts
remain in goal1-s4-v10-field-validation-{20000,final}{,-fields}.jsonl/json and text logs.
Actual outputs often answered a different field (e.g. a place or digits for a name
question). Read-only training-prefix evaluation was launched to distinguish learning
from transfer to different question expressions. No further weight training is running.

Read-only v10 training-prefix evaluation generated55/64 auxiliary answers exactly:
name16/16, number16/16, context16/16, value7/16. The other336 ordinary QA were not the
current copy-only training task; the combined63/400 is not an auxiliary learning score.
RNG replay matched the saved state. Even zero-draw prefix examples had name4/4,
number6/6 andcontext3/3 correct, showing transfer within the familiar training wording.

A strictly diagnostic `--known-field-question-form` flag replaces only the auxiliary
question with known training wording, using an explicit oracle field-task label and
identifiers already present in the original question. No gold value, evidence body,
or expected answer is passed to the rephrasing helper. Numeric partial identifiers
are rejected at both ends. Two direct helper regressions (one existing, one new) passed,
then clippy/build passed; boundary-hardening reruns are not additional independent tests.
The flag requires validation, conflicts with the older QA0/2 ablation, and marks its
header/summary as oracle diagnostics. It is absent from product inference and final
heldout scoring. Source/binary identities are in goal1-s4-field-ablation-artifact-digest.txt.

Actual unchanged-weight/unchanged-evidence/unchanged-target ablation on64 held examples
scored57/64: name16/16, number16/16, context16/16, value9/16, zero generation failures.
Original questions scored9/64. This isolates the question-expression failure for three
fields; value selection remains poor even with familiar wording. These oracle-assisted
results do NOT satisfy the failed original-question field gate and cannot select an
accepted model. All64 IDs/evidence/provided/expected comparisons are recorded in
goal1-s4-v10-field-question-ablation-identity.json. No additional weight training ran
during the ablation; the failed original results remain unchanged.

A revised training-only diagnostic is prepared from these two observed failures.
The `field-pairs` profile leaves validation bytes and ordinary training QA unchanged,
varies auxiliary training questions, and holds all background evidence/metadata fixed
within each four-answer value group. Only the selected current record's direction
changes. This is a training-data hypothesis, not an established root-cause fix.

The real trainer now supports explicit grouped sampling; default1 preserves the old
RNG sequence exactly, and older manifests default to1. The group setting is checkpointed
and shared with sampling-exposure replay. New unit coverage checks old draws, complete
blocks, invalid pools/configurations without RNG mutation and legacy manifests.
The existing fresh-process curriculum regression now also covers explicit grouped
extension, copy/full-pool boundary, bit-identical resumed weights/state, exposure
replay, and rejection without extension/source identity. Both existing default and
grouped resume regressions passed. A separate data regression passed byte-identical
validation/ordinary-QA preservation, supported targets, varied wording, and target-only
changes across four-answer groups. No model architecture or product generation changes.
Initial clippy flagged the enlarged CLI enum; bounding the new extension group argument
to the existing maximum microbatch8 removed excess option storage. Clippy and the
affected CLI resume regression then passed. Failed output remains recorded separately.

The revised field-pairs diagnostic is now executing under a separately declared
500-update/20M-input cap, not as an automatic extension of the failed v10 gate.
Frozen source15404d32a430f4cddb4042247c132cded560c4e057ca7577c7c994e8bb987732
and executableb7e0eb0c8e3d362e030632aa303e295dda19dbbe406b86affabf748d1f9814e7
are recorded in goal1-s4-v11-training-artifact-digest.txt. Actual full-corpus comparison
confirmed16668 ordinary train QA and all400 validation examples unchanged; validation
bytes/hash are identical. Train hash17550609d30a5f4b2ee8495d7e9b10215ebab7c5e67d88a6eeb9524426300640.
The probe completed20 actual updates20250→20270,27261 input/588 target tokens,
32.49s, maximum RSS2115141632B and footprint2224130376B. Remaining480 updates use
unchanged settings and original questions. The unchanged field gate is15/16 per field;
no mixed-QA continuation is permitted on another failed gate. S4 remains unqualified.

The v11 cap completed at20750, weights4478c75414aae9da3995dd2605b5e7daecf39a5c8a0782f7beaa3b073ad321d8,
input33898268/target2246990/RNG5212726812805668865. Actual500 updates consumed682413
input/14240 target tokens,4000 auxiliary sample draws. Sampler replay matched the saved
state; groups advance RNG once per quartet, unlike default individual sampling.
Main run258.85s, RSS2809200640B, footprint2505263984B; probe32.49s is separate.
Original-question full400 validation at20500 scored24/400 (24/64 auxiliaries); terminal
scored42/400 (42/64 auxiliaries), ordinary QA0/336, one ordinary generation failure.
Terminal field scores name14/16, number7/16, context12/16, value9/16 FAIL the declared
15/16-per-field gate. No mixed QA or automatic longer run follows this failed result.

Read-only terminal training-prefix evaluation scored59/64 auxiliary cases:
name16,number16,context14,value13 of16 each,0 generation failures. Oracle-question
ablation scored55/64: name16,number14,context16,value9,0 failures. Evidence/expected
identity was compared for all64. These diagnostics do not satisfy the original gate.
All7 original-question value errors exactly reproduced a distractor record's direction:
3 superseded records,1 other context and3 other numeric identifiers. Value copying and
record selection will be separated with an explicitly oracle evidence ablation; no
new training run is declared by this analysis.

The actual read-only evidence ablation at the same v11 terminal weights scored15/16
with original questions, versus9/16 with distractors. Combining it with the already
declared question ablation scored16/16. Both had zero generation failures. All targets,
questions (record-only case), original records and selected-record bytes were compared;
see goal1-s4-v11-field-record-ablation-comparison.json. A new direct helper regression
verified explicit identifiers, exact numeric/context matching, current status,
value-independent selection, and rejection of missing/ambiguous targets without mutation.
This uses an explicitly oracle selector outside the product path, is marked in logs,
and cannot pass a gate or select an accepted model. Frozen source/executable hashes
are recorded in goal1-s4-record-ablation-artifact-digest.txt.

The next data hypothesis addresses question-dependent record selection specifically:
`query-pairs` uses the same evidence with two different requested targets, followed by
the same questions with only the two direction values exchanged. Ordinary training QA,
other auxiliary tasks and full validation remain unchanged. A direct regression passed
all four layouts, exact background metadata preservation, question/value dependencies,
supported current/past targets and unchanged validation. Initial clippy requested the
current Rust fixed-array chunk API in that regression; it was adjusted and rerun.

The query-pairs v12 sourcecebc2a97e879322cfd65cc5525349dbe6bebfc82bae95e699eb4140e23c7e3c9
and executableffee858d12dd63313e2a84f573b790692eb2818e110f9165c0b456a1903e1500
were frozen before the new run (archive/hash logs retained). Actual comparison verified
all19168 non-value-auxiliary train cases and all400 validation bytes unchanged; only832
value auxiliary rows changed. Train775c820cbb2167438389616589046da68aabb0a34e943013d36a244895b77741.
The new declared cap is1000 updates/20M input tokens, copy-only, grouped4/microbatch8,
LR0.0003/warm100/W8, starting20750. No oracle selector enters training. The20-update
probe completed20770 with27412 input/508 target tokens,34.27s,RSS1999962112B,
footprint2220591744B. Remaining980 updates keep source/data/parameters/gate fixed,
maximum21750/input53898268. Full400 original-question validation is predeclared at
20770,21250 and21750. Every field must still reach15/16 before considering mixed QA;
the original full S4 quality gate is unchanged. The previous v11 failed result remains.

## 현재 상태 정정 및 제한 진단 계약

2026-09-17 실제 종료 manifest와 실행 로그를 확인했다. 위 v12 실행 계획은 과거 기록이다.
v12는 21,750 step, input 35,265,622, target 2,275,614, sampler
6,466,690,354,196,841,489에서 BUDGET_REACHED로 종료했고 현재 NOT_RUNNING이다.
원본 validation 결과는 보조45/64와 일반 QA0/336이다. S4_QUALITY=FAIL,
GOAL1_READY=NO를 유지한다. 별도 새로운 실행으로 재통과를 주장하지 않는다.

현재 범위는 R3-CUSTOMIZE-AND-DIAGNOSE-1.0이다. 상세 실측은
[의존성·저장 감사](DEPENDENCY_STORAGE_AUDIT.md), 실행/미실행 및 단계 구분은
[실험 상태](EXPERIMENT_STATUS.md)에 기록한다. 이전 S4 WIP와 새 감사 변경은 별도 기여다.
