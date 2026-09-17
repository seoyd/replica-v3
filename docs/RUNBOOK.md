# Replica v3 B0 runbook

## Explicit experimental graph journal

Product `ask` still uses SQLite. These commands only use the supplied snapshot and
journal; they do not initialize, migrate or dual-write the operating database.
Build with installed stable Rust and the existing offline lock. `test-support` is
only for fault-injection regressions, never enable it in a product release.

```sh
replica-v3 journal --snapshot SNAPSHOT.r3a --path NEW.r3j init --store-id 00112233445566778899aabbccddeeff00
replica-v3 journal --snapshot SNAPSHOT.r3a --path NEW.r3j append --request 01010101010101010101010101010101 --events EVENT.rpv3 --codec raw
replica-v3 journal --snapshot SNAPSHOT.r3a --path NEW.r3j inspect
replica-v3 archive --path SNAPSHOT.r3a --journal NEW.r3j show 1 --canonical
replica-v3 archive --path SNAPSHOT.r3a --journal NEW.r3j history --scope SCOPE --entity ENTITY --predicate PREDICATE --context CONTEXT
replica-v3 journal --snapshot SNAPSHOT.r3a --path DAMAGED.r3j recover --output RECOVERED.r3j
```

`--events` accepts exact assigned RPV3 envelopes, in commit order, at most256 events
and4MiB per transaction. `archive show --canonical` exports an exact envelope;
ordinary `show` retains its original payload output. Supply new IDs/times with the
existing Rust Event/codec API. This is an explicit experimental append/import API,
not an automatic source-DB replication service. The snapshot's existing IDs cannot
be appended again. Same request plus identical encoded content returns the prior
commit; a changed body conflicts. Scope/head/reference checks run before append.

EOF tail is retained and queries expose only the verified prefix, with a diagnostic.
The writer refuses a damaged tail. Explicit recovery creates a new file and binds
the old file hash and verified byte range; it never truncates the original. Readers
hold shared locks and the single writer holds an exclusive lock. File sync is not
claimed equivalent to SQLite's macOS fullfsync or a tested power-loss guarantee.
Journal views share archive's bounded history/current/graph semantics and lack its
unsupported full SQLite FTS/BM25 facilities. Derived indexes rebuild in RAM.

Direct commands: `cargo test --locked --offline --features accelerate,test-support
--test journal -- --test-threads=1`; existing quick also selects these regressions.
Measurement-only `validate journal-measure NEW_DIRECTORY 1000|10000|100000` creates
synthetic stores. `validate native-storage-measure SOURCE.r3m NEW_DIRECTORY` measures
native model and cold probes. Neither changes defaults or the source artifact.

This project is Rust only. The user's later instructions override the MLX suggestion
in the implementation instruction: use the installed stable Rust (verified 1.98.1), no Python bridge, no external inference API,
no canned product answers, and defer model/runtime installation until a later task.
Only Cargo build dependencies were fetched. No model or runtime installer was run.

## Offline development harness

`AGENTS.md` maps the existing source and permanent contracts. The checker uses the
existing Cargo targets with `accelerate,test-support`; the product release does not
enable test-support. The user-updated installed stable toolchain is used without
installing tools or changing Cargo.lock. SQLite/Accelerate native dependencies remain.

```sh
cargo build --locked --offline --features accelerate --bin replica-check
VECLIB_MAXIMUM_THREADS=1 RAYON_NUM_THREADS=1 target/debug/replica-check quick --output NEW_CHECK_DIRECTORY
target/debug/replica-check model --help
target/debug/replica-check release --help
```

Each output directory must be new. Command argv, stdout/stderr, test counts,
failure/skip status, source digest and deadlines are retained there. Zero executed
tests, child failure, cancellation, timeout or source changes fail the check.
The structural/module/dependency scan is a heuristic, not a whole-program proof.
Quick runs direct boundary/numeric/checkpoint/prompt/memory regressions and never
starts SMALL training or evaluates model quality. Model checks require explicit
checkpoint, original corpus, transfer corpus and split; they generate in separate
fresh processes and retain all rows, including errors. Missing input is BLOCKED.
Release additionally requires actual artifact-bound S4/S5/S6 evidence; unavailable
or not-yet-verifiable downstream evidence cannot yield a release PASS. Quick PASS
does not grant model quality or independent Goal1 acceptance.

Diagnostic QA support ablation now requires the full entity/context in the original
question. Numeric-only auxiliary tasks use an explicit separate mode. Malformed
diagnostic input fails before model load/output creation. C/W close requires both
original terminal receipts, complete watch/train panels and native artifact/clock
bindings. Missing legacy fields are unverified; fresh replay cannot cure a prior stop.

## Bounded copy curriculum continuation

`replica-train recovery progress-prepare --a0 A0_DIRECTORY --harness QUICK_SUMMARY --output NEW_PAIR`
registers both LR policies and the common immutable parent/tape before any updates.
Use the same frozen executable with `recovery progress-arm --experiment NEW_PAIR --arm C`
then `--arm L`, one process at a time, VECLIB_MAXIMUM_THREADS=1/RAYON_NUM_THREADS=1.
Only a clean TIME_BUDGET receipt permits explicit `--resume LATEST_SEGMENT_DIRECTORY`.
`recovery progress-close --experiment NEW_PAIR` recounts actual trace exposure and paired
raw scores; it never opens sealed cases or grants Goal1 readiness. Existing closed1024
receipts remain immutable; new outputs must not reuse their directories.

After A1 closes with `next=A2_CONDITIONAL`, use the newly verified frozen executable:
`recovery progress-renewal --experiment A1_PAIR --harness NEW_QUICK_SUMMARY --output NEW_A2_PAIR --seed SEED`.
This creates both balanced corpora and registers F/N before training. Run `progress-arm`
with `--arm F`, then `--arm N`, against that new pair, and finish with `progress-close`.
The prior A1 update/time ledger remains part of the same2048-update/120-minute H3 budget.
Never resume an old closed arm or reset its LR/Adam when registering this conditional pair.

`replica-train recovery progress-baseline` is the no-update A0 entry for controlled H3
progress. Explicit inputs are the H2 baseline, stopped1024 segment directory, native inference
file, H3 corpus, exact-source passing quick receipt, new output and fixed development seed.
It replays olddev once, measures original400, verifies final failing prefixes, records
tokenizer/QK/exposure measurements and freezes/evaluates CROSS-DEV. It never opens the old
seal or edits parent receipts. A0 `a0_pass` is input/measurement readiness, not model quality.
Use `--help`; the active plan defines separate new experiment forks and their bounds.

`replica-train recovery baseline`, `skill-prepare` and `skill-run` reuse the native trainer,
corpus and control paths. Inspect their `--help` for explicit paths; use only audited frozen
inputs and a passing quick summary for the exact current source. The run freezes checkpoint,
corpus, tape, binary, source, LR policy, token/update/time limits and sampler lineage before
the first update. Source and binary must remain unchanged while it runs. No ordinary QA
support oracle or field-only output is used. Exact commands/results live in the status document.

The native resume contains inherited Adam and cumulative clock. Its adjacent policy retains
the stage's constant LR; the ordinary `train --resume` path refuses to reinterpret it as a
cosine schedule. Only `skill-run --resume` accepts a matching clean TIME_BUDGET recovery,
using the same root output/tape and remaining cumulative budget. Cancellation, quality guard,
incomplete checkpoint or identity mismatch cannot resume. Do not copy a stopped artifact to
a new output to reset its allowance. Stopped files remain available for explicit inspection
and native inference export; successful loading does not make them quality candidates.

An explicitly authorized continuation after a reported quality stop uses a new output and
`--renew-from-quality-stop`; this is not plain resume or an automatic budget reset. It binds
the preserved stop's policy/result/native state and inherits all prior updates/tokens/time,
Adam, tape, LR and ordinary-watch baseline. Cancellation/resource/integrity/incomplete stops
are ineligible. The separately approved `--finish-copy-budget` amendment accepts only the
eligible stopped512-update state, allows at most1024total, and records intermediate UTF-8
counts with a stop on two consecutive increases. Control/empty and other guards remain;
final accuracy/zero-error acceptance is unchanged. Clean time-split resume persists the
previous UTF-8 count and growth streak, without a new allowance. See the active plan.

The final H3 run is CLOSED_BUDGET/QUALITY_FAIL at1024updates: full208/256, entity233,
event249, UTF-8 errors2, watch18/32. The original128/512 QUALITY_GUARD receipts remain
immutable. Final candidate/resume eligibility is false; plain resume and another explicit
finish request were both rejected through the actual CLI before any generation/update.
No further learning or H4–H8 execution is scheduled. S5/S6 receipt verifiers and release
acceptance remain pending their quality prerequisites; the release checker fails closed.

`recovery skill-diagnose` performs bounded no-update checks of the original stopped H3
segment: trace/tape/token exposure, stratified exposed/unexposed training panels, and cached
versus full-prefix logits of recorded failures. These panels are not heldout quality gates.
`recovery skill-recount --evaluation PATH --output NEW_FILE` only derives scores from
preserved dev/watch raw rows; no model or optimizer calls and no candidate promotion.
Both reuse the existing Rust recovery CLI. Detailed local paths and reproduction commands
are in EXPERIMENT_STATUS.md. Keep native checkpoints together with their adjacent policy
and receipts; moving only the file is not a verified way to preserve the stage's LR policy.

## Build and targeted verification

```sh
cargo build --release --locked --bin replica-v3
cargo test --locked --features test-support --test codec --test store --test retrieval --test runtime --test cli
cargo fmt --all -- --check
cargo check --locked
cargo clippy --locked --all-targets --features test-support -- -D warnings
```

All application code forbids unsafe Rust; library internals (SQLite, zstd, Candle)
are outside that restriction. No v1/v2 dependency/build is needed. The
`test-support` feature enables explicit fault hooks and a separate Rust test
worker binary. Default release builds contain neither fault hooks nor that binary.
Test double text exists only in tests. Zero-test unit/doc targets are not counted.

## Data and commands

Use `target/release/replica-v3`. Default data is outside the repository at
`~/Library/Application Support/Replica-v3/memory.db`; `--db` always overrides it.
Run examples with a new disposable path. `init`, backup, and restore never overwrite
an existing destination. Existing unreadable DBs fail; they do not become empty DBs.
New data directories use 0700 and DB files 0600. Existing parent directories are
not chmod'ed. SQLite sidecars inherit DB permissions. Binary records, FTS and WAL
are plaintext; OS filesystem access is the protection. No encryption claim.

```sh
replica_bin=target/release/replica-v3
replica_dir=$(mktemp -d /tmp/replica-v3-demo.XXXXXX)
replica_db="$replica_dir/memory.db"
printf '두 번째 원문\n' > "$replica_dir/input.txt"
"$replica_bin" --db "$replica_db" init
printf '원본 명령: 오른쪽으로 가\n' | "$replica_bin" --db "$replica_db" record --scope demo --session day1 --request-key 00000000000000000000000000000001
"$replica_bin" --db "$replica_db" record --scope demo --file "$replica_dir/input.txt"
"$replica_bin" --db "$replica_db" fact add --scope demo --entity robot --predicate direction --context hall --text RIGHT
# Use the IDs printed by the preceding commands for expected-head and target.
"$replica_bin" --db "$replica_db" fact correct --scope demo --entity robot --predicate direction --context hall --expected-head 3 --text LEFT
"$replica_bin" --db "$replica_db" fact restore --scope demo --entity robot --predicate direction --context hall --expected-head 4 --target 3
"$replica_bin" --db "$replica_db" fact current --scope demo --entity robot --predicate direction --context hall
"$replica_bin" --db "$replica_db" fact current --scope demo --entity robot --predicate direction --context hall --as-of 1789516800000 --valid-at 1789516800000
"$replica_bin" --db "$replica_db" fact history --scope demo --entity robot --predicate direction --context hall
"$replica_bin" --db "$replica_db" relation add --scope demo --kind precedes --from 3 --to 4 --evidence 3,4 --text '기록 순서; 원인 미확인'
"$replica_bin" --db "$replica_db" search --scope demo --query '오른쪽 지시'
"$replica_bin" --db "$replica_db" search --scope demo --session day1 --query '명령' --history --lexical-only
"$replica_bin" --db "$replica_db" search --scope demo --entity robot --predicate direction --context hall --query RIGHT
"$replica_bin" --db "$replica_db" show 1 > "$replica_dir/original.txt"
"$replica_bin" --db "$replica_db" show 1 --metadata
"$replica_bin" --db "$replica_db" doctor --full
"$replica_bin" --db "$replica_db" reindex
"$replica_bin" --db "$replica_db" backup "$replica_dir/backup.db"
"$replica_bin" --db "$replica_dir/restored.db" restore "$replica_dir/backup.db"
```

`--text`, `--file`, or stdin preserves exact UTF-8 bytes, including NUL/newlines;
`show` without `--metadata` writes only exact bytes, with no added newline. Scope,
source, session and slot components are explicit text identities. Request keys are
caller-chosen 16 bytes written as 32 hex digits, not generated UUIDs. Same key in a
scope with identical typed content returns the same event; changed content conflicts.
Without a key, repeated observations are separate events. `ask` requires a key.
A committed failed ask remains a failed result under that key; use a new key for an
intentional new attempt. A crash with input only can regenerate, but a committed
answer is returned without another model call. There is no exactly-once inference
claim across process crashes. An OS file lock serializes generation for the DB,
without keeping a SQLite write transaction open. The `.inference.lock` file is
only a process mutex, contains no canonical memory, and is released by exit/kill.

Fact versions share a fixed validity interval. `--valid-from` and `--valid-until`
are UTC Unix milliseconds; when used, repeat the same interval in corrections and
restores. Unknown start + known end is rejected. At a recorded-time snapshot the
latest ID wins ties, then its validity interval is checked; expired latest versions
do not resurrect previous values. Interval changes/branch merges are unsupported
and rejected. Use another context for another independent slot. Raw observations
and model answers are never overwritten by fact corrections.

Search is lexical, not semantic or Korean morphological search. It applies scope,
optional session/slot and recorded-time `--after`/`--before` bounds. Default excludes
questions, answers, failures and superseded/expired facts. `--history` includes them.
FTS takes literal escaped terms; operators are not executable syntax. A query
containing only 1–2 character terms uses at most 64 filtered prefix checks, returning
NarrowScope for a larger candidate set. Results include provenance, version status,
reason, paths, excerpt truncation and budget truncation. Long queries search the
version-1 normalized first 16 KiB per record; exact originals remain available.
`ask` derives at most 32 terms / 4096 bytes for retrieval and marks a shortened query
truncated; the full input still reaches persistence and the model context check.
Scope/snapshot filters also apply to graph expansion. Stored edges do not discover
or prove causality. A 100 ms budget can return fewer/no results with truncated=true.
FTS rank controls seed order; ties use SQLite FTS ordering, not a promised semantic
ranking. 64 candidates, 8 final evidence, 4 hops, 256 visited nodes are hard bounds.

## Native Rust model worker

The worker loads this project's checkpoint directory: manifest.json,
weights.safetensors and its own trained tokenizer.json. It uses the same native
Transformer and prompt framing as training. No completed external model module,
external tokenizer configuration or chat template is a product dependency.
Missing directories return MISSING_NATIVE_CHECKPOINT after the input is committed.
Artifacts remain read-only; exact weight/tokenizer hashes identify each response.
This interface is implemented; S4 task quality and S5 acceptance remain pending.

```sh
"$replica_bin" --db "$replica_db" ask --scope demo --session day2 \
  --request-key 00000000000000000000000000000002 \
  --checkpoint /path/to/our/trained/checkpoint \
  --text '오른쪽으로 가라는 기록은 무엇인가?'
```

The same executable launches one Rust foreground `__model-worker` child using
explicit argv. No shell, socket, HTTP client, cloud fallback, downloader, SQL/file
command dispatch or tool execution is exposed to generated text. The worker receives
bounded prompt data, never a Store or DB path. This is an application capability
boundary, not a same-UID OS sandbox against malicious model/runtime code.

Defaults: 256 generated tokens, 2048 native context,
180 s startup/load, 120 s generation; CLI can lower generation limits. Context is
counted with the own byte BPE after trusted role/evidence framing. Lowest
rank evidence is removed first if needed; provided/excluded IDs are persisted.
A too-large user/system prompt fails before inference. Greedy decoding runs once,
uses actual model logits, and stops at the model EOS; reaching the token limit
before EOS is an explicit failure. No hidden CoT
extraction or reflection loop. No automatic continuation or retry.

IPC is u32-LE length + transient JSON: request <=524288 bytes, response <=262144
bytes. stderr retains <=65536 bytes while draining all additional bytes. Dedicated
finite reader/writer threads avoid pipe deadlock; cancellation/timeout/failure kills
and waits for the owned child and joins its threads. Generation input/output token
counts and timings come from actual operations. No usage estimates are invented.
Answers are buffered, citation IDs `[event:N]` checked against the actual supplied
bundle, and committed before display. Bad citation output is preserved as Failure.
A missing citation displays an uncertainty notice. ID validity does not verify the
meaning of the answer. Commit failures never print a successful final answer;
if a failure event also cannot commit, stderr says `NOT persisted`.

## Explicit measurements and real smoke

```sh
cargo build --release --locked --example validate
target/release/examples/validate measure
# Execute with an actually trained native checkpoint:
target/release/examples/validate smoke /path/to/our/trained/checkpoint target/release/replica-v3 artifacts/new-native-smoke
target/release/examples/validate native-failures /path/to/our/trained/checkpoint target/release/replica-v3 artifacts/new-native-failures
```

`measure` creates temporary synthetic DBs and separately reports raw/auto-zstd,
10000-event batch imports, 20 durable single commits, 128 reads, 100 warm lexical
queries, reindex, DB/WAL/SHM/FTS pages, reopen-read and RSS. Reopen is not OS-cold.
Min/median/max with sample counts are observations; no P99 confidence claims.
The real smoke creates OS-seeded facts after loading checkpoint metadata and uses
the actual CLI for writes, corrections, restores, relations and answers. Seven
questions cover five required categories, then repeat in fresh processes with new
request keys. Same-key retries use an absent checkpoint to verify zero model loads.
It preserves every command/output, fixture, expected value, actual text/provenance
and grading result under a new output directory. Failed quality exits nonzero;
separate semantic review is still required. It never edits generated answers. The historical B0
model smoke was NOT_RUN; current native acceptance is tracked separately. Power-loss, physical disk-full,
true cold-cache and GPU/shared-memory measurements were not performed.

`native-failures` checks fresh random-model rejection, corrupt artifact rejection,
generation timeout, synchronized CLI cancellation and an actual deferred SQLite
COMMIT failure after native generation. Every failed request must preserve its input,
emit no success stdout, and replay its Failure without loading a model. Its artifacts
are disposable synthetic DBs; it never installs triggers in an existing user DB.

## Current native Goal 1 work

The current model contract is [GOAL1-NATIVE-TRPP-1.0](GOAL1_CONTRACT.md). Do not obtain external weights
or tokenizer artifacts. Native training/tooling progress is tracked in PLAN.md.
S1 result/provenance/search/snapshot regressions are verified; neural model work is
not yet complete. Failed backups report an untrusted retained destination path; inspect
that artifact separately and choose a fresh path for a new backup attempt.

### Own corpus and byte tokenizer (S2)

```sh
cargo build --release --locked --bin replica-train
mkdir -p artifacts
target/release/replica-train corpus prepare --output artifacts/goal1-corpus --documents 2000 --seed 41
target/release/replica-train tokenizer train --corpus artifacts/goal1-corpus --output artifacts/goal1-tokenizer.json
# Inspect a supplied local byte fixture in a new process:
target/release/replica-train tokenizer inspect --tokenizer artifacts/goal1-tokenizer.json --file /explicit/fixture
```

Outputs are create-new; use a fresh output path when repeating. `artifacts/` is
ignored and never staged. `corpus prepare --local /explicit/file` additionally reads
only the specified authorized UTF-8 document, without normalizing its bytes. Omit it
for SYNTHETIC_ONLY. Corpus serialization is training data, not a personal memory DB.
Tokenizer's adjacent `.manifest.json` records actual sequence/token statistics.
No external weights/tokenizer/model API is used. Actual training evidence is recorded
in NATIVE_MODEL.md; it is distinct from final task-quality acceptance.

### Native initialization / training / resume (S3)

```sh
# source_id is a SHA-256 of an explicitly recorded source manifest, not a temporary instruction.
source_id=$(shasum -a 256 docs/logs/goal1-s3-source-digest.txt | awk '{print $1}')
target/release/replica-train model init --tokenizer artifacts/goal1-tokenizer.json --output artifacts/small-init --profile small --seed 17 --source-id "$source_id"
target/release/replica-train model inspect --checkpoint artifacts/small-init
target/release/replica-train train --checkpoint artifacts/small-init --corpus artifacts/goal1-corpus --output artifacts/training-run
# Resume uses the checkpoint's optimizer/schedule/sampler config and exact corpus hashes:
target/release/replica-train train --resume artifacts/training-run/final --corpus artifacts/goal1-corpus --output artifacts/resumed-run
```

CPU/F32 is explicit. Default horizon 5000 steps, 20M input tokens, length512,
microbatch1/accumulation4, clip1, LR0.001, warmup100, validation every100. Overrides
are finite CLI config; --stop-after is an intentional optimizer-boundary stop within
that horizon. Resume continues the saved horizon; it is not a new schedule or weight-only
fine-tune. --numeric-probe with a tiny artifact is an explicit training-loop check,
not a memory QA dataset. Only training tooling can reach its toy samples. A final test
must never select the checkpoint. Ctrl-C publishes a boundary checkpoint and exits
with cancellation; process death retains the last published checkpoint. Training RSS
is sampled after steps with a 16 GiB stop guard, not a precise transient-peak profiler.

### Native training and evaluation in a restricted process (S4)

The actual SMALL run is in `artifacts/goal1-small-train`. Its immutable initialization
is `artifacts/goal1-small-init`. Model/optimizer/corpus artifacts remain local and ignored.
Use fresh output paths when reproducing; existing directories are never overwritten.

```sh
# macOS deny-network execution; time runs outside the restricted child.
/usr/bin/time -l /usr/bin/sandbox-exec -p '(version 1)(allow default)(deny network*)' \
  target/release/replica-train train --checkpoint artifacts/goal1-small-init \
  --corpus artifacts/goal1-corpus --output artifacts/another-small-run \
  --steps 5000 --lr 0.001 --warmup 100 --accumulation 1 --validate-every 250 --no-rss
# Validation-only diagnostic: this command is never the final heldout score.
target/release/replica-train evaluate --checkpoint artifacts/goal1-small-train/step-000750 \
  --corpus artifacts/goal1-corpus --output artifacts/validation-generation.jsonl --limit 25
```

The optional `--split train` explicitly labels a training-set diagnostic. It cannot be
reported as validation or final heldout accuracy; `validation` remains the default.

`--no-rss` is explicit because this sandbox denies spawning ps. Logs report None for
sampled RSS. External time's maximum resident set size and memory footprint are distinct
observations; neither is a measured GPU allocation. The tensor planning guard is retained.
Native generation diagnostics run one greedy logits loop with a fresh bounded cache per
question, never the answer renderer. Validation expected text is used only after generation.

After training stops, select and record the minimum-validation-loss checkpoint before
creating or executing the final fixture. The example is the separate final test tool:

```sh
cargo build --release --locked --offline --features accelerate --example validate
target/release/examples/validate holdout-prepare artifacts/goal1-independent-final-v2.json
# Replace SELECTED with the previously fixed candidate; do not select it from test results.
target/release/examples/validate evaluate SELECTED artifacts/goal1-independent-final-v2.json trained artifacts/heldout-v2-trained.jsonl
# RANDOM_INIT must be the candidate's own initialization, with the same tokenizer/config.
target/release/examples/validate evaluate RANDOM_INIT artifacts/goal1-independent-final-v2.json random artifacts/heldout-v2-random.jsonl
target/release/examples/validate evaluate SELECTED artifacts/goal1-independent-final-v2.json no-evidence artifacts/heldout-v2-no-evidence.jsonl
target/release/examples/validate evaluate SELECTED artifacts/goal1-independent-final-v2.json value-swap artifacts/heldout-v2-value-swap.jsonl
target/release/examples/validate retrieval-baseline artifacts/goal1-independent-final-v2.json artifacts/heldout-v2-retrieval.jsonl
```

Trained-mode evaluation preserves all results and exits nonzero when the quality gate
fails. Baseline modes may finish successfully with poor scores; inspect
`task_target_pass` and actual counts. Failed generations stay in the200-case denominator.
The current fixture writer creates version2 using a recorded OS-random seed, independent
of the training generator and the exposed version1 regression fixture. It covers four
version questions, matched context distractors, new bindings, insufficient evidence and
confirmed chronology with causal uncertainty. Counterfactual pairs change only the cited
value; they do not mutate the canonical memory store. Semantic review of actual Korean
outputs remains required, including citation-to-event attribution in sequence answers.
No final-test result may train/select a candidate; if it informs a later change, use a
new independent final test for that model. Version1 artifacts remain readable.

Recorded attempt: the5000-step run completed, but selected step750 scored only1/200
automatically; its sole automatic hit failed semantic inspection. The full comparison
is in NATIVE_MODEL.md and raw goal1-s4 logs. This is **not an accepted memory model**.
That S4 experiment failed and remains unpublished. A continuation is now in progress;
native ask/generate/chat are implemented, while S5 actual quality/restart acceptance
and S6 quantization/integrated closure remain outstanding.
Do not resume the historical external-model installation path to bypass this failure.

### Current continuation interfaces

The preceding S4 failure describes the first experiment. The next bounded run uses
`artifacts/goal1-corpus-v2`, `goal1-tokenizer-v2.json` and `goal1-small-v2-train`.
Its fixed parameters and actual ten-step probe are recorded in
logs/goal1-s4-v2-operating-config.txt. `--features accelerate` selects the verified
CPU SGEMM backend; it is not Metal and does not load a completed external model.

The external model/template loaders have been removed. Current native commands:

```sh
cargo build --release --locked --offline --features accelerate --bin replica-v3 --bin replica-train --example validate
target/release/replica-v3 generate --checkpoint /path/to/our/trained/checkpoint --text '질문'
target/release/replica-v3 --db /path/to/memory.db ask --scope demo \
  --request-key 00000000000000000000000000000003 \
  --checkpoint /path/to/our/trained/checkpoint --history --text '정정 전 방향은?'
target/release/replica-v3 --db /path/to/memory.db chat --scope demo --session conversation \
  --checkpoint /path/to/our/trained/checkpoint --history
```

`generate` opens no memory DB. `ask` and each submitted chat line use the same
persist/search/model/validate/commit path. Chat assigns a fresh OS-random request key,
ends on EOF or `/quit`, and observes Ctrl-C even while waiting for input. It does not
implicitly include previous generated answers as evidence. `--history` supplies fact
versions while excluding previous questions/terminal results before retrieval limits.
General `search --history` retains its existing all-history behavior. All three native
interfaces default to256 new tokens/context2048 and require an actually updated own
checkpoint. An unqualified checkpoint is not made task-correct by these interfaces.

Ordinary `train --resume` keeps the saved budget/schedule/data exactly. A separate
bounded run may be requested explicitly with `--resume CHECKPOINT --extend-steps N
--source-id SOURCE_MANIFEST_SHA256`:1..5000 additional steps and20M additional input
tokens, optimizer/RNG retained, saved base LR/warmup restarted for the new horizon.
Global update/token counters never reset. Its manifest records the budget origin.
It requires a fresh output directory. Use `--stop-after` with an absolute global step
for a short probe, then ordinary resume of that probe's final checkpoint. This option
does not trigger automatic repetition or change the corpus, quality bar or final test.

`--replace-corpus --corpus NEW_CORPUS` is allowed only with that explicit extension.
It retains the own tokenizer and prior corpus hashes, and updates the declared current
training/validation hashes. Ordinary resume rejects an undeclared data change.
`--extend-microbatch N` (1..8) and `--extend-curriculum-steps N` may be specified only
at this new run boundary; the latter counts additional initial copy-training steps.
All changes are checkpointed and a subsequent ordinary resume uses them exactly.
`--first-target-weight W` defaults to1 for fresh training. An explicit new run may
set `--extend-first-target-weight W` (finite1..16). This optional objective gives
each sequence's first supervised token weightW and all other supervised tokens
weight1, divided by the actual target-token count. Prompt/PAD positions stay masked.
`loss` and checkpoint/validation CE remain unweighted; `objective` is reported
separately and supplies the actual backward pass. Old artifacts default to weight1.
This changes optimization only; model architecture, logits decoding and answer
validation are unchanged. Rebuild the CLI/evaluator when using newly written artifacts.
The `grounding` corpus profile adds multi-record selection exercises and supported
sequence/uncertainty answers. It is a training-only generator, not an inference path.
The `counterfactual` profile makes up to four evidence variants of each grounding
scene. Values, citation IDs, record metadata and evidence order vary while ordinary
QA questions stay fixed. Complete three-event causal records always teach the known
sequence plus uncertainty. A12000/400-episode split has3000/100 base scenes, not12000/400
independent questions. The manifest records the generator revision; the command also
prints base-scene counts. The final evaluation constructor is separate from this code.
The `evidence-first` profile retains those questions/records and supported values,
but teaches single-fact QA answers to generate the supporting event citation before
the direction value. Copy and sequence/uncertainty tasks retain their original targets.
This is a training-only change in response order, not an inference formatter or
post-generation citation repair. It needs a declared new corpus/run and actual
quality evaluation; preparing it alone does not improve or qualify a checkpoint.
The `record-copy` profile teaches the supporting record's complete original sentence
followed by its citation for single-fact QA and copy-family auxiliary exercises.
Entity, context and value therefore appear together in the supervised target. Some
training-only questions about the earliest restored history
use another wording; their answers and requested version stay unchanged. Validation
QA wording and source records are preserved. This profile also reports base scenes
separately from its four counterfactual variants; it does not run in the product.
The `entity-cue` profile preserves every ordinary `record-copy` QA episode at the
same seed/size. It changes only copy-family auxiliary exercises: four evidence
variants use four different subject-type names, and the neutral question asks for
that name alone. The question contains none of the labels. It supplies short first-
token supervision through the existing copy curriculum; it is not a runtime classifier
or answer table. Record IDs, times, statuses and remaining original bytes are retained.

`replica-train evaluate --known-question-form --split validation --limit N ...`
is an oracle paraphrase diagnostic, limited to the eligible QA0/QA2 episodes.
It uses corpus metadata only for the entity/context already explicit in the original
question and substitutes known training wording. It does not supply an expected value
or citation ID, and leaves evidence unchanged. Logs retain both questions and mark
`oracle_question_ablation=true`. Its score is neither final-task quality nor an eligible
v6 candidate-selection measurement. The product worker never runs this helper.

For a learning-path memorization diagnostic, `replica-train corpus subset --source
<existing-corpus> --output <new-directory> --count 32` preserves the first32 ordinary
QA in each existing split (allowed count16..32). It keeps the original episode bytes,
excludes copy auxiliaries, records parent hashes, and never overwrites an existing
corpus. Use the existing SMALL trainer/checkpoint/evaluate commands on the subset;
record any tokenizer/corpus continuation explicitly. `evaluate --split train --limit 32`
checks actual autoregressive entire-answer/EOS accuracy on the trained examples.
Its teacher-forced diagnostic is computed afterward and is not a generation score.
A memorization pass cannot satisfy the independent heldout quality gate. The current
bounded random-init recipe and probe are in logs/goal1-s4-qa32-config.txt and
logs/goal1-s4-qa32-operating-config.txt; broad-corpus training is paused during diagnosis.

`replica-train sampling-exposure --start <checkpoint> --end <checkpoint> --corpus
<unchanged-corpus> --limit 400` reconstructs sampler exposure within a single recorded
QA training run. It checks the final RNG state, supports the recorded copy curriculum,
and reports exposure histograms and prefix IDs/counts. LM chunked documents and changed
run configs/corpora are rejected. This is deterministic replay, not a per-draw log or
a claim that every member of the training pool has actually been trained on.

The training-only `field-cue` corpus profile extends `entity-cue` auxiliary examples
with identifying digits, context, and evidence-value extraction. It preserves all
ordinary QA and balances fields across existing record layouts. These auxiliary
scores diagnose learning of individual fields; they cannot replace full QA or final
independent heldout scoring. Values must be read from evidence; number/context targets
may already be explicit in the requested identifier and serve as copying exercises.

`evaluate --known-field-question-form --split validation --limit 64` is an oracle
auxiliary-task ablation. It keeps evidence and targets unchanged but supplies the
known field-task identity and training-style question wording; original questions
remain in the log. Its scores cannot select a candidate or satisfy auxiliary/final
gates. It is mutually exclusive with `--known-question-form` and never affects the
product worker, default evaluation, or independent final test.

The training-only `field-pairs` profile preserves `field-cue` validation bytes and
ordinary training QA, diversifies auxiliary training wording, and makes each value
quartet differ only in the selected current record's direction. It keeps background
records, ordering, IDs, times and status fixed inside that quartet.
`train --sample-group-size N` samples whole consecutive blocks of N from the eligible
pool. N must divide the microbatch and pool size; default1 preserves the prior RNG
sequence. Corpus producers are responsible for arranging semantically related blocks.
Changing a resumed run requires `--extend-sample-group-size N` with explicit extension
budget and source identity. The saved setting is used by training, restart and sampler
exposure replay. Grouping changes batch sampling only; attention remains independent
between batch rows, and no expected answer reaches generation.

`evaluate --single-current-record --split validation --limit 16` is an oracle
evidence-selection diagnostic limited to auxiliary value tasks. It selects exactly
one current record using numeric entity/context identifiers already explicit in the
question. Gold values are not read by selection. It may be combined with
`--known-field-question-form` to separate question transfer from distractor selection.
Original and generated evidence are both logged and the run is marked
`oracle_record_selection`; its score cannot satisfy any quality gate or select a
candidate. Product retrieval, generation and default evaluation are unchanged.

The training-only `query-pairs` profile changes only `field-pairs` value auxiliary
quartets. Two questions request different records from identical evidence; a second
pair swaps only the records' direction values. IDs, source, times, order and statuses
stay fixed. Superseded/current layouts explicitly request past/current respectively.
Thus neither ignoring the question nor ignoring evidence can answer all four cases.
Validation and all other training tasks are preserved. Group size4 keeps each complete
two-by-two contrast in the actual minibatch; no oracle selector is used in training.

## 제한 contrast16 진단

`replica-train contrast freeze --corpus <기존 query-pairs corpus> --log <원본 train
generation JSONL> --tokenizer <기존 자체 tokenizer> --output <새 freeze 파일>`은
원본 앞400개에서 실패16을 추출·대조하고 새로운64를 학습 전에 함께 동결한다.
기존 파일은 덮지 않는다. `contrast check --fixture <freeze> --checkpoint <원본>`은
실제 model의 prompt/gradient/batch/cache 경로를 검사한다.

`contrast train --fixture <freeze> --checkpoint <원본> --output <새 directory>
--source-id <실제 소스 SHA-256> --start random|qa`는 고정 SMALL, 새 Adam,
micro4×accumulation4, update마다16개 전체, 최대1,000 updates/320만input/45분이다.
random은 seed17 재초기화 hash를 대조하고 qa는 auxiliary-only checkpoint를 거부한다.
두 번 연속16/16·4/4 후 종료하며, `contrast evaluate --fixture <freeze> --checkpoint
<final>`의 새 프로세스 복원이 별도로 필요하다. 모든 평가/노출/counter는 실제 값이다.
훈련 checkpoint는 `contrast16` sampler 표시를 가지므로 보통의 랜덤 trainer 재개와
혼용할 수 없다. source 품질은 DIAGNOSTIC_ONLY이며 서비스 합격 모델이 아니다.

새64의 `contrast evaluate ... --heldout`은 암기 통과 뒤 사전 선택한 한 후보에 한 번만
실행한다. 이 결과를 보고 추가 학습하지 않는다. 현재 실행 결과는16개 암기 통과,
새64는0/64이며 Goal1은 미완이다. 세부 계보와 hash는 EXPERIMENT_STATUS.md에 있다.

`contrast transfer --fixture <freeze> --checkpoint <native>`는 학습16에서 원본,
ID/순서/entity 길이/context/값을 한 요소씩 바꿔 생성하는 DEVELOPMENT 진단이다.
새64를 다시 평가하지 않는다. `contrast train ... --start diagnostic --both-orders`는
명시한 학습된 contrast checkpoint에서 새 Adam으로 원본16+순서반전16을 학습한다.
32개 전체1회/update, micro4×accumulation8, 기존1,000updates/320만input/45분 한도를
유지한다. 두 번 연속32/32·8/8묶음 뒤 종료하며, export한 inference 파일의 transfer에서
원본/순서반전 각각16/16·4/4를 새 process로 확인한다.4개 base scene의 변형을
독립32개 scene으로 부르지 않는다. 일반 QA/인용/새 사실 품질은 별도 조건이다.

평가 도구는 Adam/state 유무가 아닌 native artifact의 `trained_steps`로 초기/학습
가중치를 구별한다. inference-only 파일도 `evaluate ... trained ...`에 사용할 수 있고,
학습된 파일을 random baseline으로 평가하면 거부한다. 품질 gate는 그대로 적용한다.

`corpus qa-pairs --source <기존 query-pairs corpus> --output <새 directory>
--groups <1..128>`는 새base를 만들지 않는 제한적인 일반QA 대조자료다. N개의 기존
질문/값 quartet을 원문/인용을 생성하는 일반QA로 변환하고 N개의 기존 일반QA quartet을
그대로 유지한다. 각각 두 근거 순서로8개, 총16N개다. validation bytes는 그대로라서
새blind test가 아니다. 묶음 순서를 유지하는 microbatch8/sample_group_size8을 명시한다.
이 자료가 학습됐다고 간주하지 말고 실제 sampler 노출 수와 생성 결과를 함께 확인한다.
기존 학습을 이어갈 때는 native resume의 명시적 extension/replace-corpus를 사용하며,
tokenizer는 새로 학습하지 않는다. 자원probe의 updates도 같은 실행 예산에 포함한다.

2026-09-17 현재 이U2 실험은250updates 뒤 일반QA155/336→56/336으로 악화돼
중단했으며 사용자 요청으로 전체 구현/학습도 PAUSED다. 위 명령은 기능 설명으로,
학습 재개 지시나 추천 설정이 아니다. 코드 게시와 학습 재개를 구별한다.

## Native binary 기본 artifact (현재)

기존 학습 디렉터리는 원본으로 보존한다. 아래 명령은 새 목적지 파일을 만들며
기존 파일/디렉터리를 덮어쓰지 않는다. 파일 확장자는 식별 기준이 아니며 magic과
schema를 검사한다. 초기화/학습의 start/step/final도 이제 단일 binary 파일이다.

```sh
cargo run --offline --locked --release --bin replica-train -- model import-legacy --source OLD_DIRECTORY --output MODEL.r3m --kind inference
cargo run --offline --locked --release --bin replica-train -- model import-legacy --source OLD_DIRECTORY --output RESUME.r3m --kind resume
cargo run --offline --locked --release --bin replica-train -- model export-inference --checkpoint RESUME.r3m --output INFERENCE.r3m
cargo run --offline --locked --release --bin replica-v3 -- generate --checkpoint MODEL.r3m --text '질문'
cargo run --offline --locked --release --example validate -- native-load-audit MODEL.r3m inference
cargo run --offline --locked --release --example validate -- native-load-audit RESUME.r3m resume
```

위 generate는 DB를 열지 않는다. ask/chat은 원래 SQLite 기억 검색을 유지한다.
worker와 학습 재개는 binary만 읽으며 구형 디렉터리를 자동으로 열지 않는다.
inference 파일의 resume은 오류다. legacy 변환물은 진단/품질 미확인 상태를 유지한다.
저장 성공으로 모델 품질 또는 Goal1 통과를 선언하지 않는다. corpus/IPC/출력 로그의
JSON과 모델 artifact의 JSON 제거는 별개다. 이 작업에서 tokenizer를 다시 학습하지 않는다.

행렬 비교는 `validate kernel-profile MODEL.r3m FIXTURE` 및
`validate kernel-compare MODEL.r3m FIXTURE`이다. 기본은 candle-linear-v1;
Rust decode GEMV는 더 느린 것으로 관측돼 기본값으로 채택하지 않았다.

## DB 없이 읽는 evidence archive

```sh
replica-v3 --db SOURCE.db archive --path NEW.r3a export --compression zstd
replica-v3 archive --path NEW.r3a inspect
replica-v3 archive --path NEW.r3a show 1
replica-v3 archive --path NEW.r3a history --scope S --entity E --predicate P --context C
replica-v3 archive --path NEW.r3a current --scope S --entity E --predicate P --context C --as-of 1000 --valid-at 900
replica-v3 archive --path NEW.r3a graph --scope S --seeds 1,2 --direction incoming --history
replica-v3 archive --path NEW.r3a search --scope S --query '쪽' --history
```

export만 source SQLite를 연다. 다른 archive 명령은 --db/default DB와 무관하게
archive 파일만 읽는다. show 기본 출력은 원문 bytes이며 --metadata로 사건 필드를 본다.
source DB/기존 archive를 덮어쓰지 않는다. 다른 시점의 최신 DB와 비교할 때는 snapshot
ID/기록시각/source identity를 먼저 맞춘다. current는 기록시점 as_of와 유효시점 valid_at을
구분한다. 새 취소는 fact retract이며 기존 fact write와 같은 scope/slot/expected-head/
validity 검증을 거친다. 운영 기억 추가·ask/chat은 계속 SQLite를 사용한다.

실측10,000건에서 zstd는 raw보다988,222B(10.32%) 작고, block 교체가 발생하는
get/graph 중앙값은 약6% 느렸다. 저장용 읽기 전용 archive 기본값은 zstd이며 속도/메모리
우선이면 --compression raw를 명시한다. SQLite FTS 및 live writes의 대체 완료가 아니다.
validate archive-measure NEW_DIRECTORY는 사용자 DB가 아닌 별도10,000건 합성 fixture만
만들어 비교한다. validate archive-read-probe PATH sqlite|archive는 새 프로세스 open과
첫 조회를 측정하며 OS cache를 강제로 비우지 않는다. 상세 실측은 EXPERIMENT_STATUS.md.
# Read-only state/data/result repair observations

After the boundary quick checks pass, use the existing training executable's explicit
legacy mode. Preserve original experiment directories; the output must be new.

```sh
VECLIB_MAXIMUM_THREADS=1 RAYON_NUM_THREADS=1 target/release/replica-train recovery progress-reaudit --experiment artifacts/h3-controlled-20260917/a2 --output artifacts/state-data-result-20260917/reaudit
VECLIB_MAXIMUM_THREADS=1 RAYON_NUM_THREADS=1 target/release/replica-train recovery progress-reaudit --experiment artifacts/h3-controlled-20260917/a2 --output artifacts/state-data-result-20260917/reaudit --observe F16
VECLIB_MAXIMUM_THREADS=1 RAYON_NUM_THREADS=1 target/release/replica-train recovery progress-reaudit --experiment artifacts/h3-controlled-20260917/a2 --output artifacts/state-data-result-20260917/reaudit --observe N16
VECLIB_MAXIMUM_THREADS=1 RAYON_NUM_THREADS=1 target/release/replica-train recovery progress-reaudit --experiment artifacts/h3-controlled-20260917/a2 --output artifacts/state-data-result-20260917/reaudit --observe N256
```

Reaudit checks immutable policies, frozen inputs, native endpoints and all existing complete
raw panels, then independently rescores. It grants no historical candidate/resume permission.
Observations require the completed reaudit receipt. F16/N16 register the first eight exact
and eight incorrect dev rows before generation. N256 is a single post-hoc DEVELOPMENT
observation of the existing step-0256 native artifact, reusing validated dev and generating
only missing CROSS512/ordinary400. Each observation directory is create-new; failed observations
block further generation. Maximum new calls944 with reused dev, SMALL optimizer0; no checkpoint
sweep, seal opening, model installation, budget renewal or operating pointer change.

## Binary evaluation and resume repair

Use `replica-train recovery native --help` for the new canonical diagnostic path.
`native import --policy POLICY --endpoint NATIVE --evaluation DEV_WATCH_RAW --cross CROSS_RAW
--ordinary ORDINARY_RAW --output NEW_ROOT` performs an explicit read-only historical audit,
verifies frozen owned data and endpoint/native/step lineage, then writes `inputs.r3er`,
raw binary panels, terminal and comparison into a new root. Originals are never updated.
`native close --root ROOT --terminal terminal.r3er` uses only owned binary records and
native files. Complete wrong outputs produce candidate=false; incomplete/corrupt panels
produce an integrity/stop record, not a successful comparison.

`native run --root ROOT` and `--resume segment-00/terminal.r3er` use explicit terminal
lineage and the same native trainer. This repair enables optimizer execution only for
registered TINY regression specs; SMALL updates and historical resume remain disabled.
With `test-support`, `native fixture` creates a real random-init numeric fixture and
`fixture-fork` clones its verified native parent for continuous/split-process regressions.
These test-only commands and fault hooks do not exist in the product/default release.
Quick now includes the direct binary unit and subprocess tests. Its optional
`R3ER_TEST_BOOTSTRAP` identifies an already-created, matching TINY fixture to avoid
repeating preparation; `R3ER_TEST_OUTPUT` must be a new directory. Report actual updates
from child logs, including fixture preparation and failed runs, against the128-call cap.

`native parity --root IMPORTED_F512 --output NEW_ROOT` freezes16 dev and16 ordinary
indices from category/family/base/ID metadata before inspecting outputs. It copies the
same native checkpoint, runs normal greedy once per selected case and compares raw tokens,
EOS, text, errors and prompt identity with preserved records. Parity panels have separate
kinds and never replace full QA/H3 quality evaluation. `native bench --root IMPORTED_ROOT
--terminal terminal.r3er --output NEW_ROOT` reports uncompressed size and timing measurements;
these developer measurements are not machine decision input.

The older named recovery/corpus/harness commands retain their legacy JSON interfaces;
they are not an automatic fallback from native readers. Product JSON IPC and opaque
legacy config/request IDs remain. This is a bounded evaluation/control migration, not
whole-project JSON removal or model quality acceptance.
