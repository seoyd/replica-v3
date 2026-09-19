# Replica v3 working map

## Scope and authority

Continue the existing repository; preserve user changes and original artifacts.
The user's current instructions take precedence over this map.
On 2026-09-19 the user explicitly authorized deletion of all local learned data,
corpus, checkpoints, original user data, long-term memory and execution logs,
while retaining Rust source and Git history. That data reset is complete.
Historical artifact paths below and in linked documents are no longer available.
Do not resume an old experiment, reconstruct its receipts or restore deleted data
from Git as part of ordinary work. The requested removal of product/training/tool
JSON use and direct JSON dependency removal is implemented. SQLite is retained
under the latest user scope. Transitive JSON dependencies inside existing generic
libraries are permitted. Use R3BIN for former text metadata/IPC/record streams;
reuse R3MODEL/R3CORP/R3TOK/R3ER for their existing domain schemas.
The earlier storage freeze does not restrict that explicitly requested removal.
Permanent acceptance criteria: `docs/GOAL1_CONTRACT.md`.
The fresh joint4096 baseline is closed and its new artifacts are preserved.
The exposure/phrase study closed at2048 additional updates per arm: C primary425/512,
transfer44/128; P primary425/512, transfer79/128. Both failed the development gate.
Final200 remains unopened; no further learning is authorized by these closed plans.
Its two explicit forks used the same parent, constant LR3e-5,
unchanged Adam/tokenizer/response CE and eight-bucket case order. P changes only
one of two question exposures per case. Do not reinitialize or resume the old
closed run. The active plan defines repair gates, budgets and conditional S4/S5/S6.
The selector-consistency study is a new explicit P6144 fork: C-KEEP/S-SELECT retain
P's wording policy and differ only in C/D/E selector exposure. Its own policy caps
each arm at2048 updates and the pair at4096; old run permissions remain unchanged.
Active stages, budgets and stop conditions: `docs/QUALITY_RECOVERY_PLAN.md`.
Observed results and historical failures: `docs/EXPERIMENT_STATUS.md`.
Architecture and artifact semantics: `docs/NATIVE_MODEL.md`.
Operational commands: `docs/RUNBOOK.md`.
Keep temporary instructions local; do not link them from delivered sources/docs.

## Code map

`src/lib.rs` is the product library root.
`src/main.rs` owns the product CLI.
`src/model.rs` owns native worker transport and inference.
`src/neural.rs` owns project tokenizer framing and mapping.
`src/neural/transformer.rs` owns the native model and bounded KV.
`src/neural/artifact.rs` owns native inference/resume serialization.
`src/neural/checkpoint.rs` owns training state; retired text import fails explicitly.
`src/binary.rs` owns typed R3BIN metadata and record-stream serialization.
`src/store.rs`, `event.rs`, `codec.rs` own immutable memory and history.
`src/retrieval.rs`, `app.rs` own evidence packing and committed responses.
`src/train_main.rs` is the separate training executable.
`src/data.rs` is training-only; generators never enter the product library.
`src/training.rs` owns the trainer and diagnostic corpus evaluator.
`src/quality_recovery.rs` owns bounded diagnostics and receipt checks.
`src/check_main.rs` runs existing checks and propagates failures.
`tests/` contains direct Rust regressions; `examples/validate.rs` has native probes.

## Commands

Use the user-updated installed stable Rust (verified 1.98.1) and existing Cargo lock.
Do not install a toolchain or update dependencies as part of checks.
Default builds and tests use `--locked --offline`.
On the current M4, use `--features accelerate` and one compute thread.
Build the checker with `cargo build --locked --offline --features accelerate --bin replica-check`.
Run `target/debug/replica-check quick --output NEW_DIRECTORY` before learning.
For the fresh baseline use `target/debug/replica-check --output NEW_DIRECTORY quick --fresh`;
this runs only the relevant independent numeric/corpus/native-process regressions.
Use `replica-check model --help` for explicit artifact/corpus validation.
Use `replica-check release --help` for receipt-bound release verification.
Run only directly relevant tests during implementation.
The release gate is the explicit place for all non-heavy v3 regressions.
A filter that executes zero tests is not a passing check.
Missing dependency cache or model evidence is BLOCKED, not PASS.

## Immutable boundaries

All new product, training, evaluation and development code is Rust.
Existing locked generic crates and native system dependencies remain permitted.
Do not use external model weights, learned tokenizers, teachers or model APIs.
Do not download models or substitute an external local model server.
Do not route training generators, expected answers or ablation oracles into inference.
Do not repair generated numbers, force EOS or use lossy UTF-8 decoding.
H0–H6 preserve SMALL topology, tokenizer mapping, F32 and backend.
Do not change SQLite, archive, tensor kernels or storage before S4 passes.
S6 permits only the contracted INT4 implementation and measurements.

## Execution and evidence

One owned heavy process at a time; no background automatic learning.
Freeze source/binary while learning and retain the exact identity of each run.
Use the shared cancellation/deadline path through evaluation and finalization.
Save consistent optimizer state on stop; do not start further work after cancellation.
Preserve failed and partial receipts; missing eligibility fields fail closed.
Keep code verification, skill quality and Goal1 readiness separate.
Report actual updates, tokens, exposure, elapsed time and observed memory.
Never replace failed heldout results with an oracle-assisted score.
Advance only when the previous stage's actual gate passes.
Do not extend a failed run beyond its registered budget.

## Preservation and publication

Use disposable test DBs, never the user's operating DB.
Preserve corpus, checkpoints, Adam, raw failures and uncommitted work.
Do not reset, clean, stash, rebase or force-push away user work.
Store large experiment evidence under ignored `artifacts/`.
Stage named related source/tests/status docs only after the stage closes.
Do not publish checkpoints, corpus, private logs, DBs or temporary instructions.
Normal push to the verified branch is authorized; compare the actual remote SHA.
Independent acceptance remains external; never self-grant GOAL1_ACCEPTED.
