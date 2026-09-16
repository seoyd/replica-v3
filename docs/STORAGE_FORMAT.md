# B0 storage format v1

Frozen before implementation. Canonical memory is one SQLite database, application_id
0x52505633, user_version=1, projection/normalization version=1. No dictionary or
payload deduplication. records(id INTEGER PRIMARY KEY, body BLOB) is append-only.
IDs are positive <= i64::MAX, allocated under BEGIN IMMEDIATE from max(id)+1.
Timestamps are signed UTC Unix milliseconds; recorded_at is max(clock, previous
recorded_at), with EventId breaking same-millisecond ties. observed_at is optional.

Envelope (20 bytes): magic `RPV3`, u16 LE version=1, u8 codec (0 raw, 1 zstd),
u8 flags=0, u32 LE raw_length, u32 LE stored_length, u32 LE CRC32C of raw body.
Body and stored lengths <=1,048,576. Decode checks lengths before allocation;
zstd accepts exactly one frame with no trailing data, reads at most raw_length+1,
and limits window_log to 20. Encode tries
zstd level 3 only at body length >=256 and uses it only if strictly smaller.
Checksum detects damage; it is neither authentication nor encryption.

All variable integers are canonical unsigned LEB128 (max 10 bytes). Signed i64
uses zigzag. Bytes/text = varint length then exact bytes; text must be UTF-8.
Option = u8 0 absent / 1 present then value. Booleans = u8 0/1 only.
Lists = varint count then entries, max 8 IDs unless stated otherwise. Strings
source/scope/session/slot components <=1024 bytes; payload <=262144 bytes.
Request key = optional exactly 16 caller-supplied bytes, CLI 32 hex digits.
No UUID version is implied; keys are unique within scope. Same key requires
identical typed content excluding assigned ID/recorded_at, otherwise Conflict.

Body common order: id, recorded_at(zigzag), observed_at(option zigzag), source(text),
scope(text), session(text), request_key(option 16 bytes), payload(bytes), kind(u8).
Kinds and following fields:

- 0 Observation: question(option GenerationLimits).
- 1 Fact: slot(entity/text,predicate/text,context/text), previous(option ID),
  restored_from(option ID), valid_from(option zigzag), valid_until(option zigzag).
  Scope comes from the common header. Slot SQL key = four length-framed texts
  scope/entity/predicate/context, never separator concatenation.
- 2 Relation: type(u8), from(ID), to(ID), evidence_refs(list IDs).
- 3 AssistantAnswer: input(ID), evidence_refs(list IDs actually cited),
  provided_refs(list IDs actually in model prompt), excluded_refs(list IDs),
  model_id(text), model_revision(text), runtime_revision(text), quantization(text),
  license(text), finish_reason(text), input_tokens(option varint),
  output_tokens(option varint), limits(GenerationLimits), load_ms(varint),
  first_token_ms(option varint), generation_ms(varint), retrieval_truncated(bool).
- 4 Failure: input(ID), code(text). Payload holds the diagnostic or exact rejected
  model output, never a successful answer. One terminal result per input.

GenerationLimits: max_tokens(varint), context_tokens(varint), timeout_ms(varint).
B0 maxima/defaults: 512 new tokens, 8192 context, 120000 ms generation.
Relation tags: 0 used_evidence, 1 precedes, 2 supports, 3 contradicts,
4 causal_hypothesis, 5 supersedes, 6 restores. Last two and used_evidence are
transaction-derived from canonical facts/answers; explicit attempts are rejected.
There is no confirmed causal edge. All references must exist, have smaller IDs,
and belong to the same scope. A relation event has nonempty explicit evidence.
Answers/failures refer to a question Observation. Answers never become facts.

## Linear version/time semantics

Each explicit slot has one linear lineage. Initial fact has no previous/restored
reference; corrections require the expected current head. Restore appends a new
version with the exact target payload and both previous/restored references.
All versions of a lineage share one validity interval: changing it is explicitly
unsupported (use a separate context). Unknown valid_from means unspecified
validity and requires absent valid_until. Absent valid_until is open-ended only
when valid_from is known. Zero is a real timestamp. Intervals are [from, until).
At a recorded-time snapshot, select the last recorded version (ID tie-break),
then test its interval at the requested valid time. An expired latest version
does not resurrect an older one. as_of excludes later recorded events regardless
of observed_at. Late arrivals preserve original event time. history returns all
versions in ID/record order. No similarity-based correction or interval merging.

## Derived state and bounds

record_meta, current_heads, relations, request_state and record_fts are disposable.
Normalization v1 is Unicode lowercase on a UTF-8-safe first 16384 bytes; it does
not normalize canonical text. FTS5 trigram stores that prefix again (measured
separately). Retrieval excerpts are UTF-8-safe first 4096 bytes and carry an
explicit excerpt_truncated flag. Exact bytes are available through show.
Search uses parameterized filters, quoted literal FTS terms (up to 32); no query
operators are accepted as syntax. Short-only queries scan at most 64 filtered
prefixes, and return NarrowScope if more are eligible. Default search excludes
questions and answers and superseded/expired facts. history opts into all kinds.
Time filters apply to recorded time. All expansion paths preserve every filter.
Budgets: 64 seeds, 8 evidence, 4 hops, 256 visited nodes, 100 ms SQLite/traversal
budget, bounded neighbor fetch. Budget exhaustion reports truncated=true.

SQLite uses WAL, synchronous=FULL, foreign_keys=ON, busy_timeout=5000 ms and
macOS fullfsync=ON; actual values are queried and checked. Inference never holds
a write transaction. Canonical and all projections commit atomically. Normal
startup checks identity/version/quick_check/projection watermark; full doctor
replays canonical records and compares all derived rows. Reindex runs one
transaction. Backup uses SQLite online backup into a create-new destination,
then full validation and exact canonical comparison. Restore uses the same
path and never overwrites. DB/sidecars are plaintext and rely on OS permissions;
new directories are 0700, new DB files 0600. Same-UID/root edits are not prevented.


## Runtime/projection clarifications after integration

A per-DB `.inference.lock` is an empty OS mutex, not a second commit store. The
canonical DB path is resolved before choosing that lock. An active generation
fails another generation attempt explicitly; the input remains committed.
Question inputs retain full bytes while retrieval derives a bounded prefix and
marks truncation. Source/model metadata is text inside explicit binary fields.
Model revisions include hashes of actual weights/tokenizer/chat-template bytes.
FTS is deliberately the outer SQL loop (CROSS JOIN) and seed rank uses SQLite's
FTS rank order; do not add a metadata-driven join or secondary sort that forces
an unbounded pre-LIMIT sort. Scope and snapshot checks still precede admission.

Goal 1 S1: terminal replay verifies canonical input, question kind, scope and session.
Startup/head comparison values come from one SQL observation. Online backup pins a
canonical read snapshot before copying and retains it through validation. A failed
create-new backup is retained as an explicitly untrusted artifact; the error includes
its path. Existing destinations are never overwritten/deleted. Retrieval now reports
sentinel/hop/output limits even when duplicate/filter rejection prevents queue growth;
fetched candidate/edge and eligible counters are transient diagnostics, not new schema.
