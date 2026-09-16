# Replica v3 B0 runbook

This project is Rust only. The user's later instructions override the MLX suggestion
in the implementation instruction: use installed Rust 1.98.0, no Python bridge, no external inference API,
no canned product answers, and defer model/runtime installation until a later task.
Only Cargo build dependencies were fetched. No model or runtime installer was run.

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

## Local Rust model adapter (installation deferred)

One adapter: Candle 0.11.0, Qwen2-family GGUF, CPU. No GPU performance claim.
Use one existing compatible local GGUF plus its original tokenizer.json and string
chat_template in tokenizer_config.json. Files must be explicitly supplied. No model
is bundled, downloaded or inferred from a public name. Missing files return
BLOCKED_MODEL after the input is committed. Unsupported architecture/template or
missing required GGUF metadata is an explicit error, not a fallback answer.
Weights/config/tokenizer are read-only. Metadata identifies model name, tensor
quantization types and license (UNSPECIFIED_LOCAL_METADATA if absent); SHA-256
fingerprints of weights, tokenizer and template identify the exact local revision.
The declared upstream model license must be checked when a model is later supplied.

```sh
"$replica_bin" --db "$replica_db" ask --scope demo --session day2 \
  --request-key 00000000000000000000000000000002 \
  --model /existing/local/model.gguf \
  --tokenizer /existing/local/tokenizer.json \
  --tokenizer-config /existing/local/tokenizer_config.json \
  --text '오른쪽으로 가라는 기록은 무엇인가?'
```

The same executable launches one Rust foreground `__model-worker` child using
explicit argv. No shell, socket, HTTP client, cloud fallback, downloader, SQL/file
command dispatch or tool execution is exposed to generated text. The worker receives
bounded prompt data, never a Store or DB path. This is an application capability
boundary, not a same-UID OS sandbox against malicious model/runtime code.

Defaults/maxima: 512 generated tokens, 8192 context or lower actual GGUF limit,
180 s startup/load, 120 s generation; CLI can lower generation limits. Context is
counted with the local tokenizer after rendering the original chat template. Lowest
rank evidence is removed first if needed; provided/excluded IDs are persisted.
A too-large user/system prompt fails before inference. Greedy decoding runs once,
uses actual model logits, and stops at the model EOS or output limit. No hidden CoT
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
# Execute only after the user resumes model work and supplies installed files:
target/release/examples/validate smoke /existing/model.gguf /existing/tokenizer.json /existing/tokenizer_config.json
```

`measure` creates temporary synthetic DBs and separately reports raw/auto-zstd,
10000-event batch imports, 20 durable single commits, 128 reads, 100 warm lexical
queries, reindex, DB/WAL/SHM/FTS pages, reopen-read and RSS. Reopen is not OS-cold.
Min/median/max with sample counts are observations; no P99 confidence claims.
The real smoke contains five fixed Korean questions, reopens between turns, checks
actual source citations and prints unmodified model text/provenance for human
inspection. It does not replace answers with expected strings. Not run in this task
because the user deferred model installation. Power-loss, physical disk-full,
true cold-cache and GPU/shared-memory measurements were not performed.
