# B0 storage format v1

## R3TOK v1 train derivative

This immutable prototype is compiled from the verified R3ER owned train split,
after the BASE/SPAN study. Existing study preparation is not automatically changed.
Source JSON and the full episode snapshot remain the provenance and reprocessing
inputs. R3TOK contains tokens and sparse answer-byte roles, not a replacement for
original text, memory or model weights. It uses the existing bounded Reader and
create-new/fsync publisher; no JSON payload or Rust struct memory image is stored.

All fixed integers are little-endian. Header320 bytes:

| Offset | Bytes | Meaning |
| --- | ---: | --- |
| 0 | 8 | R3TOK plus3 zero bytes |
| 8 | 2 | wire1 |
| 10 | 1 | byte order1 |
| 11 | 1 | token width2 or4 from actual vocabulary/IDs |
| 12 | 4 | header length320 |
| 16 | 8 | exact total length |
| 24 | 4 | sample count |
| 28 | 4 | sequence bound |
| 32 | 8 | total tokens |
| 40/48/56 | 8 each | index/token/span offsets |
| 64 | 192 | six SHA256: physical snapshot, typed ordered train contents, ordered ordinals, tokenizer semantic identity, framing+sequence, bound role/run policy |
| 256 | 32 | body SHA256 |
| 288 | 32 | first288 header bytes SHA256 |

Body: token offsets u64[N+1], response_start u32[N], source ordinal u32[N],
curriculum u8[N], supported-role flag u8[N], span offsets u64[N+1], packed u16/u32
tokens, then spans(start u32,end u32,role u8). Span offsets count intervals. Bounds
are128MiB/16384 samples/8192 sequence/32 spans per sample. Exact adjacent sections,
offset order, BOS/EOS/assistant boundary, vocabulary and target control IDs are
checked. The reader consumes the same owned bytes it hashed; no retokenization
fallback. Dense role fractions reconstruct from stored intervals and original answer
bytes with the same tokenizer. Model/sequence/policy mismatch requires a new compile.

`compile.pending` is durable before artifact publication. Directory shared/exclusive
OS locks serialize compile/reader; raw/Zstd and complete.bin are immutable. complete.bin
is72 bytes: R3TKDONE plus raw/cold physical SHA256 (cold zero if not smaller). Pending
or missing/corrupt completion blocks consumption. Pending unlink after durable outputs
is the commit boundary; later directory cleanup sync failure warns, as for verification
authorization. Failed artifacts are preserved. Checksums detect corruption, not hostile
re-signing by a writer with filesystem access; OS permissions remain a trust boundary.

The prototype uses indexed packed sample access followed by the existing Rust batch()
and Candle tensors. It verifies all sample IDs/order/targets/masks/roles, and five actual
draws. Disk u16 does not halve tensor RAM. Zstd3 is a whole-container cold derivative:
it must fully decompress before random sample access. Both are retained for measurement;
neither changes current production inference, DB, `.r3m` or historical runs.

## Train-only objective records

R3ER kind19 explicitly extends the owned input snapshot with objective revision1,
BASE/SPAN mode, exact annotation digest and ordered train probe ordinals. The
annotation uses answer-byte intervals and the unchanged full answer tokenization;
it is neither a product inference input nor a tokenizer mapping. Kinds1/13 remain
unchanged. A nondefault optimizer continuation requires its bound native run policy;
the unchanged `.r3m` itself does not identify the loss used to train it. Generic
resume from a registered objective directory is rejected in favor of native run.

Kind20 contains bounded typed own-model train-probe rows: native/source/policy
binding, ordinal, actual target/correct counts, first-error role, CE/objective and
role mass/NLL. Started and cumulative partial rows are immutable; an unfinished
probe is not silently retried. Kind21 extends the segment with up to512 actual LR
bit patterns and per-update preparation/forward/backward/optimizer seconds,
gradient norm, update norm, objective and clip flag. Old kind14 retains its256
bound and original meaning. These records use fixed typed fields, not JSON payloads.
The model tensor format, inference equation ID, tokenizer mapping and user DB are
unchanged. Train cache measurements belong to a later source than this experiment.

## Verification publication authorization v2

R3ER kind17 is the new intent (explicit scope plus the existing typed fields);
kind18 binds the intent FileRef and physical final digest as publication Pending.
Kinds10/15 remain legacy read-only; no markers are backfilled into old experiments.
Readers take a shared OS lock on the immutable intent inode; the finalizer holds
an exclusive lock. Intent/pending/partial raw are preserved after failures.

| Boundary | Return/consumer meaning |
| --- | --- |
| Intent only, missing final, or Pending present | Incomplete/UNKNOWN; no approval or retry |
| Final link visible but file/directory publication returned Err | Pending remains; same blocked outcome in a fresh process |
| Final file and directory sync succeeded; Pending unlink fails | Error, Pending remains, blocked |
| Pending unlink succeeds under writer lock | Authorization linearization point; final was already durable |
| Following cleanup directory sync fails | Committed success with explicit cleanup warning; crash may resurrect Pending and block conservatively |

Cleanup cannot delete raw/final evidence. No failure log write is required to retain
the earlier durable blocker. This is cooperative file/OS-lock serialization, not a
multi-file transaction or a guarantee against arbitrary device power-loss behavior.

## R3ER verification intent and outcome

PV01 adds bounded typed record kinds10(verification start),11(entry reservation or
returned EvalRow),12(final outcome) to the existing R3ER envelope/codec. Kind9 retains
its old preflight-proof meaning. Model/native/journal/SQLite formats are unchanged.
The start binds the canonical root, pair/source/binary, two input/command/native file
references and ordered cases. The existing create-new hard-link publisher elects one
writer after file sync and directory sync; observed work begins only after success.
No per-token journal is added. Each entry reservation precedes the API, and its returned
raw row is published before the next cancellation check. A reservation is not an actual call.

Started → immutable returned rows → provisional proof → final(success with proof ref).
Consumers share read_preflight_outcome; only the complete binding grants current success.
Failed and incomplete attempts cannot retry or authorize another arm. Missing finalization
reports durable lower bounds, reserved limits and UNKNOWN tail/time/tokens, never actual0.
Known failures preserve actual entries/completions/interruption/tokens and original/save
errors. Final elapsed measures through prior proof/publication work to the final decision;
the last immutable file cannot include its own fsync duration. Budget conservatively adds
the120s publication reservation once, separately from observed time. Stdout records elapsed
after final publication. Cooperative cancel is sealed immediately before finite publication;
fsync/tensor preemption and multi-file atomic transactions are not claimed.

Old proofs without intent are LEGACY_SUCCESS_WITHOUT_ATTEMPT_INTENT in explicit read-only
audits. They are not upgraded or granted new verification permission. Current work permits
new save-verification fixtures only for TINY; the completed SMALL4 test is reused as history.

The bounded cooldown study uses kind13 for the same owned input snapshot with explicit
K/D policy and run purpose. Source, binary, parent and registered root join its binding.
Kind14 adds actual f64 LR bits to the existing segment, checked against each cumulative
update before resume. Native .r3m and old kind4 are unchanged. Kind15 is a scoped use
of the same verification intent (parent16 or conditional fresh1168); kind16 extends the
existing proof to up to three panels. All scopes use the same row/finalization/consumer.
Entry/final readers bound counts by their intent; old kind9/10 retain their old decoding.
No JSON state, tensor change, new publisher, or automatic retry is introduced.

## Experimental R3JRN v1 (fixed before implementation)

Separate explicit snapshot+journal paths; product ask remains SQLite. R3ARCH and
RPV3 bytes are unchanged. One exclusive OS file lock for the writer; readers hold
a shared lock. Max100,000 total events,256MiB canonical and512MiB decoded event
bytes; max256 events/4MiB decoded transaction, max512MiB journal. No rotation/GC.

All offsets below are bytes, integers unsigned LE; reserved bytes must be zero.

| header offset | size | meaning |
| --- | ---: | --- |
| 0 | 8 | `R3JRN\0\0\0` |
| 8 | 2 | version1 |
| 10 | 1 | event reader schema2 (accepts wire1/2) |
| 11 | 1 | byte order1 (LE) |
| 12 | 4 | header length224 |
| 16 | 16 | caller-provided store identity |
| 32 | 32 | exact base snapshot SHA256 |
| 64 | 32 | base canonical source digest |
| 96 | 8 | max raw transaction4MiB |
| 104 | 8 | max file512MiB |
| 112 | 32 | recovery source file hash; zero for new store |
| 144 | 8 | recovery source verified prefix end; zero for new store |
| 152 | 40 | reserved zero |
| 192 | 32 | SHA256(header[0..192]) |

| transaction header offset | size | meaning |
| --- | ---: | --- |
| 0 | 8 | `R3TXN\0\0\0` |
| 8 | 8 | full frame length:128+stored+56 |
| 16 | 8 | consecutive commit sequence, starts1 |
| 24 | 16 | request key; unique per journal |
| 40 | 32 | previous frame digest; zero for first |
| 72 | 4 | event count1..256 |
| 76 | 4 | raw body length |
| 80 | 4 | stored body length |
| 84 | 1 | codec0 raw /1 zstd single bounded frame |
| 85 | 11 | reserved zero |
| 96 | 32 | SHA256(exact raw transaction body) |
| 128 | stored | body: each u32 length + exact RPV3 event bytes |
| 128+stored | 8 | trailer `R3COMMIT` |
| +8 | 8 | repeated full frame length |
| +16 | 8 | repeated sequence |
| +24 | 32 | SHA256(transaction header + stored body) |

Append validates the complete ordered batch with existing Event/link/head/request
rules before writing; file sync succeeds before derived view/ACK. Same request
and exact encoded body returns the existing commit, conflict otherwise. Assigned
ID/time are supplied by this explicit import API and included in content identity.
Complete prefix only is replayed. Partial EOF header/body/trailer is preserved,
reported, and disallows append; malformed complete frame/checksum/sequence stops
at that offset with no resync. Recovery explicitly publishes a new journal with
source hash/prefix range and copies verified frames; old bytes remain unchanged.
No real power-loss proof: std sync_all differs from SQLite macOS fullfsync.
DURABILITY_NOT_EQUIVALENT. Checksums are not authentication. Derived metadata,
versions and bounded BFS reuse Archive; committed journal bodies are a bounded
in-memory overlay. No duplicate payload persisted in a second index.

Snapshot export measurement permits64KiB/256KiB blocks and raw/zstd1/zstd3 inside
existing R3ARCH wire1 and2MiB block maximum. A single larger event occupies its
own bounded block. Default archive export remains2MiB/zstd3. No dictionary.

Model storage remains R3MODEL wire1 raw F32. Timing fields are process-local
instrumentation, not new wire fields. Measurement-only R3COLD wrapper is rejected
by the product loader; only byte-exact restored .r3m files are loaded.

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
Request key = optional exactly 16 bytes, explicit CLI keys use32 hex digits.
Chat generates a fresh OS-random key for each submitted line. Its LF/CRLF delimiter
is framing; body bytes, including a final CR without LF at EOF, are preserved.
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
Native model revisions include hashes of actual weights, own tokenizer and architecture configuration; historical revision strings remain readable.
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

## 자체 모델 container v1 (현재 기본 경로)

사건 RPV3 codec과 별개의 `R3MODEL\0` 모델 파일이다. tensor dtype은 F32,
integer는 명시 little-endian 또는 기존 canonical ULEB128이다. Rust 구조체/enum의
메모리 layout을 저장하지 않는다. 파일 하나에 config/tokenizer/lineage/directory/body가
있으며 JSON 문자열, JSON header, 압축 JSON을 넣지 않는다.

64-byte prefix:

| offset | bytes | 의미 |
|---:|---:|---|
| 0 | 8 | ASCII R3MODEL + NUL |
| 8 | 2 | LE wire version=1 |
| 10 | 1 | kind: 1 INFERENCE, 2 RESUME |
| 11 | 1 | flags=0, 이 버전은 LE만 지원 |
| 12 | 4 | LE header length, 최대2MiB |
| 16 | 8 | LE 전체 파일 길이, 최대192MiB |
| 24 | 32 | header SHA-256 |
| 56 | 8 | reserved=0 |

header 순서는 operator family/equation/parameter/state/numeric 문자열, bounded config,
architecture semantic digest, model content digest, source/initial-weight/status,
init seed/trained steps/diagnostic flag, optional legacy identity, tokenizer,
optional training state, tensor directory이다. 문자열/bytes는 canonical 길이+원문이다.
기존 JSON config/tokenizer/tensor file digest의 migration map은 명시 importer가
원본 checksum/shape/mapping을 검증한 후 보존한다. native loader가 JSON을 재구성해
이 map을 만드는 것은 아니다. legacy wire identity와 새 semantic identity는 별개다.

config는 profile 문자열 다음 vocab/layers/hidden/heads/kv_heads/head_dim/ffn/
local_layers/window/context varint, eps/rope_theta F64 LE이다. semantic ID는 profile
label을 제외하고 수식 및 수치 설정을 포함한다. 알려지지 않은 수식/state를 같은
shape라는 이유로 허용하지 않는다. 기존 SMALL/TINY 및 명시 experimental 상한만 허용한다.

tokenizer schema1은 특수 ID0..7, byte 원문 처리, ASCII 비문자/각 숫자의 독립
segmentation을 고정한다. train digest/legacy wire digest/semantic mapping digest,
vocab count, ID 순서의 raw token bytes, ordered merge count와 두 입력 token ID를
저장한다. 특수 token의 raw bytes는 비어 있고 문자 token은1..32bytes, vocab264..4096.
256 byte fallback, 고유 vocab, merge의 앞선 구성요소/고유 결과/도달 가능한 vocab을
검증한다. direct BPE builder를 사용하며 load 시 JSON parser를 사용하지 않는다.

각 tensor descriptor는 name(한 번), dtype1=F32, rank/dims, offset/length LE u64,
codec0=raw, payload SHA-256이다. 이름은 정렬된 정확한 parameter schema 집합이며
rank1..3, shape/dtype/count/expected byte product를 allocation 전에 확인한다.
첫 payload 및 각 후속 payload는64-byte 경계에 시작하며 padding은0..63개의0 byte.
마지막 payload 직후 EOF이고 trailing bytes, 빈 tensor, overlap, duplicate, 잘못된
정렬/shape/checksum/nonfinite를 거부한다. raw body는 contiguous F32 LE이며 whole-file
압축을 하지 않는다. checksum은 손상 검출이고 인증/암호화가 아니다.

INFERENCE에는 model weights/config/exact tokenizer와 필요한 출처·실제 update 수·
진단 여부만 있으며 Adam/config scheduler/sampler 상태는 없다. RESUME에는 동일 모델과
각 parameter의 Adam m/v, TrainConfig 전 필드, step, input/target budget, u64 sampler,
corpus/validation/previous-corpora hashes, initial/parent weight lineage, loss,
contrast16 여부와 중단 상태가 있다. F64/u64는 JSON 숫자로 우회하지 않는다.
contrast16은 고정16개 전체 통과 sampler의 기존 필드명이다. 명시적인 순서 반전 진단은
같은16개의 두 view인32개를 허용하며 microbatch×accumulation과 corpus hash로 구별한다.
16/32 외 크기는 거부한다. wire 배치는 그대로이며 구형 reader는32개 state를 거부한다.
optimizer step 경계에서만 trainer가 저장하고 accumulation 중간 gradient는 저장하지
않는다. SIGKILL 후 최신 미공개 gradient까지 복구된다는 의미가 아니다.

inference view는 resume 파일에서도 Adam payload를 seek로 건너뛰며 읽거나 할당하지
않는다. 따라서 그 view는 미사용 Adam payload의 checksum까지 검사했다고 주장하지
않는다. resume view와 export의 공개 전 검증은 모든 tensor를 검사한다. inference-only
파일을 resume으로 열면 명시 오류다. model revision은 optimizer 내용과 독립된 model
content/architecture/tokenizer semantic ID로 구성한다.

공개 절차: 동일 parent의 create_new 소유 temp → 전체 작성 → 전체 readback 검증 →
file.sync_all → hard_link no-clobber → parent.sync_all. 일반 오류 시 자신의 temp만
정리한다. SIGKILL은 미공개 temp를 남길 수 있지만 기존 목적지/source를 덮어쓰지 않는다.
공개 후 directory sync 오류는 목적지가 이미 존재할 수 있다. OS/device 한계를 넘는
power-loss proof를 주장하지 않는다. 배포 경로는 native-only이고 legacy 디렉터리 탐색
fallback은 없다. `model import-legacy --kind inference|resume`만 명시 변환 경로다.

## 사건 schema2와 읽기 전용 evidence archive v1

기존 schema1 사건의 bytes는 그대로 유지한다. 명시 취소 Kind5와 DependsOn7/Retracts8
관계 tag만 schema2를 사용한다. 취소는 같은 slot/validity의 현재 Fact ID를 참조하는
새 immutable 사건이며 이전 원문을 지우지 않는다. 최신 취소 시 current는 None이고
이전 값으로 자동 fallback하지 않는다. restore는 실제 과거 Fact의 원문을 새 사건으로
복원하고 취소 head 및 복원 대상에 각각 계보 edge를 남긴다. 관측시각은 기록순서와
별개다. Precedes/DependsOn/Supports/Contradicts/CausalHypothesis는 다른 관계이며
사고가 뒤따랐다는 이유로 원인을 확정하지 않는다. 구형 reader는 새 schema2를 거부한다.

archive는 R3ARCH + NUL2개 prefix144 + 독립 block들 + directory/index의 단일 파일이다.
운영 SQLite를 자동 변경하거나 대체하지 않는다. Export는 count/max ID를 읽은 한 SQL
read transaction을 유지하고 모든 canonical ID/body를 재인코딩 없이 그대로 복사한다.
작성은 모델과 동일한 create_new temp/전체 검증/fsync/no-clobber 공개를 재사용한다.

| offset | bytes | 의미 (정수 little-endian) |
|---:|---:|---|
| 0 | 8 | magic |
| 8 | 2 | archive version1 |
| 10 | 1 | block 선택: raw0, zstd가 작을 때만 사용1 |
| 11 | 1 | 지원 canonical schema2 (기존 schema1 포함) |
| 12 | 4 | directory+index 길이 |
| 16 | 8 | 전체 파일 길이 |
| 24 | 8 | directory 시작 offset |
| 32 | 8 | snapshot 최대 사건 ID; empty0 |
| 40 | 8 | snapshot 최종 recorded_at; empty i64::MIN |
| 48 | 4 | 사건 수 |
| 52 | 4 | block 수 |
| 56 | 8 | canonical envelope+encoded body 총 bytes |
| 64 | 8 | 원문 payload 총 bytes |
| 72 | 32 | 순서대로 id:i64/encoded_length:u64/body를 해시한 source SHA-256 |
| 104 | 32 | prefix[0..104], prefix[136..144], directory의 SHA-256 |
| 136 | 8 | 압축 해제한 canonical event body 총 bytes |

각 block descriptor52 bytes는 offset:u64/stored:u32/raw:u32/codec:u8/reserved3zero/
raw SHA-256이다. record index20 bytes는 id:i64/block:u32/offset:u32/length:u32이다.
순차 block 영역 및 순차 record coverage를 완전히 검사하고 gap/overlap/중복 ID/trailing
bytes를 거부한다. 각 block은 최대2MiB이며 event를 쪼개지 않는다. zstd level3은 기존
의존을 쓰며 한 frame만 허용, window≤2MiB, 선언된 출력+1byte에서 해제를 제한한다.
사전 dictionary와 payload dedup은 없다. 전역 파일 압축도 없다.

상한은100,000사건, canonical encoded256MiB, decoded512MiB, directory8MiB,
파일264MiB+144이다. 개별 event는 기존 codec 상한을 검사한다. 선언된 길이/count/offset을
allocation 전에 검사하고 전체 canonical linkage, ID/시간순서/request/result 중복/
원문 합계/source digest도 확인한다. SHA/CRC는 손상 검출이며 인증이 아니다.
prefix/source/block digest64+32×block_count bytes는 각 항목 크기에 이미 포함된다.

읽기 경로는 SQLite를 열지 않는다. slot version, metadata, 양방향 adjacency는 파일에서
재구성하는 메모리 index이고 원문의 대체물이 아니다. 한 block 캐시만 유지한다. get,
history, current(as_of recorded/valid_at), outgoing/incoming/both 탐색을 제공한다.
SQL과 archive는 같은 bounded BFS를 사용한다: 후보64, 방문256, hop4, 출력8,
협력적100ms deadline. scope/session/기록시각/snapshot과 edge origin 제한을 적용한다.

lexical은 모든 단어가3자 미만인 기존 정규화 원문 prefix의 전체 query substring
경로만 동일하다. 그 외는 SQLite FTS5 trigram/BM25를 구현하지 않았으므로
UNSUPPORTED/NOT_COMPARABLE이다. append/live transaction/동시 쓰기/recovery는
NOT_IMPLEMENTED이다. archive와 SQLite 모두 명시된 각 snapshot의 원본 data source이며
archive가 운영 DB의 대체 backup/recovery라고 주장하지 않는다.

## Training evaluation records: R3ER v1

The training executable's `recovery native` commands use custom, uncompressed binary
records. Product memory and native `.r3m` weights/Adam/tokenizer formats are unchanged.
`experiment_record.rs` uses the existing bounded Reader, canonical varints and immutable
same-filesystem publisher. It is not imported by product inference.

Header: magic `R3ER` (4 bytes), version u16 LE=1, record kind u8, byte-order u8=1,
payload length u64 LE, SHA256 (32 bytes), followed by the exact typed body. The hash
covers `R3ER-record-v1\0`, the kind byte and the body. The entire file has a separate
physical SHA256 in every FileRef. Evaluation content excludes mutable checkpoint
locators, decisions and its own digest. No JSON serialization participates in this hash.

| Kind | Ordered typed body |
| --- | --- |
| 1 Inputs | contract, run/policy/frozen/source digests, parent native reference, historical/TINY flags, original-file provenance, owned Episodes, train ordinals, panel specs, immutable batch tape, evaluation steps, LR policy/offset, baseline counters, QA floor |
| 2 Evaluation | run/input/source/model/tokenizer/architecture digests, absolute step/new updates, panel kind, expected count, ordered rows |
| 3 Decision | run/input/dev/watch content digests, absolute step, guard-before/after counters, applied and quality-stop flags |
| 4 Segment | run/input/segment, optional parent terminal and durable native, evaluation/native mappings, decision refs, guard, all observed stop reasons, completion/resume/candidate/save-error, actual update/generation/teacher counts, elapsed/cleanup scalars, actual draws |
| 5 Comparison | run/input/terminal binding, independently recomputed panel scores, guard, candidate/historical flags |

Fixed digests are 32 bytes. Other unsigned integers use canonical unsigned LEB128;
signed timestamps/IDs use zigzag. Text is length-prefixed strict UTF-8. Vectors retain
order, lengths precede allocation, option/boolean tags accept only 0/1. Scalar tags
0/1 hold IEEE754 f32/f64 LE bits, preserving signed zero and subnormals. Tags2/3 preserve
nonfinite failure bits; successful teacher/diagnostic scalars must be finite.
Unknown version/kind/option tags, overflow, extra trailing bytes and duplicate membership
are rejected. Bounds: file128MiB, owned cases16384, panel rows512, tokens per row2048,
text262144 bytes, evidence IDs256, tape512, terminal chain64. Actual row limits also
must match the owned request and tokenizer preparation. Production panels remain
Dev256/Watch32/Cross512/Ordinary400, with ordinary336 QA and64 auxiliary frozen at import.
Panel tags0–3 identify these four panels. Tags4/5 are explicit DevParity16/OrdinaryParity16;
they cannot satisfy the ordinary H3/S4 close gates. TINY test specs are restricted to
native TINY profiles and cannot become product candidates.

An owned Episode contains the exact request, evidence, answer and metadata. EvalRow
references its ordinal/content digest and records typed prompt hashes/length/provided/
excluded IDs, raw tokens, EOS position, finish/error/completion/interruption, timeout,
generation timing/KV/workspace/retention observations and all existing teacher diagnostic
values. Actual text and bytes are reconstructed using the bound tokenizer, without
trimming or lossy decode. Gold is used only after native generation. Integer counters,
entity/event/base4/4 and QA/auxiliary scores are independently derived from fixed panels.

CheckpointRef binds relative locator, physical SHA256, run/segment, actual model/tokenizer/
architecture, absolute step, optional Adam digest and input/target/sampler counters.
Absence of Adam denotes an inference-only reference and cannot resume. The resolver
loads the actual native file, checks these identities and reads its physical hash again.
Root escape and symlink escape are rejected; there is no filename or JSON fallback.
Normal close compares every final panel's resolved model/tokenizer/equation/numeric
policy/absolute step to the resolved terminal endpoint. Physical file differences and
inference versus resume packaging do not imply different evaluation models. Adam is
required for resume, not inference-only evaluation references. Stored stop reasons in
the verified terminal ancestry block normal close independently of close-stop files;
only a proven resumed intermediate TimeBudget-only stop is allowed.
Original segment final and later segment step files can have different physical hashes
while binding the same evaluation model/step. Explicit lineage permits that re-reference.

Evaluation, decision, native preservation and terminal publication are separate durable
writes, not a multi-file transaction. Completed pending decisions are resolved before
another update; applied decisions must reproduce the same before/after state exactly.
Earlier evaluation decisions cannot be omitted when a later optimizer clock is observed.
Time-only resumable stops preserve the tape cursor. Cancellation, quality, integrity,
partial evaluation and save errors cannot become clean time resumes. A close failure
publishes a separate immutable `close-stop.r3er`; it blocks a later normal close.
Cooperative checks cover generation, teacher, update, preservation and close. Synchronous
tensor work and fsync are not forcibly preempted; command1800s and cleanup120s are bounds
checked at safe boundaries, not hard real-time guarantees.

Legacy JSON is read only by explicit import/audit and noncanonical measurement tools.
Original file digests and policy/data/native lineage are retained in the owned snapshot;
imported evaluation source fields identify their exact historical raw file. The import
report records converter source, serde_json1.0.151 default/std/alloc and the absence of
float_roundtrip. Historical in-memory float bits remain UNKNOWN/LEGACY_ROUNDTRIP_UNPROVEN.
A current binary hash does not retroactively establish an absent historical receipt.
Imported terminals describe the completed read-only audit, never the eligibility of the
old run; original flags/files remain unchanged and historical candidate/resume are false.

The explicit importer obtains each parsed legacy record and its provenance digest from
the same owned file bytes. It never substitutes a later re-read's hash for the bytes it
parsed. Final-arm inline summaries and the existing post-hoc result-file schema are
separate validated cases; missing/null required fields remain errors. The latter does
not restore historical float bits or preregister an observed intermediate checkpoint.

Measured F512 dev256 (9542 output tokens): binary evaluation84869B, same typed-fields
JSON426543B. Legacy rows942922B versus active owned cases plus binary evaluation348888B
is a schema-deduplication-and-codec comparison. Full input snapshot4006623B also holds
unused training/other panels. All existing diagnostic fields are retained. Encode,
verification and durable write were not uniformly faster; EXPERIMENT_STATUS records
the exact workload, timing table and RSS limitations. Benchmark JSON is noncanonical.

R3ER kind6 is an explicitly authorized anchor-pair input snapshot. Existing kind1
bytes and readers remain unchanged; older readers reject kind6. Its body is the
existing input body followed by pair/baseline/expected-parent SHA256s, anchor count
u8 and two bounded vectors of training ordinals. Its input identity has the separate
`R3ER-anchor-input-v1` domain. The approved purpose is R3-NATIVE-STORAGE-QUALITY-1.0,
F512 step23798 with Adam, C50(4/4) or A75(6/2),512 updates,2M input/500K target tokens.
Preparation verifies exact original anchor contents and frozen pool membership before
publishing both registrations. Corpus provenance uses explicit read-only legacy import;
run/evaluation/resume/close consume the owned binary input only. Source and executable
hashes must match registration before training. Historical imports still cannot resume.

Pool shuffles are deterministic training-only master streams, separately seeded from
the inherited sampler. Both ratios use the same per-pool sequence and reset to that
pool's original order before each reshuffle cycle. The immutable full tape records
every batch ordinal, actual input/target denominator and continuation cursor. Different
ratios imply different draw/token totals; no padding or token-equality claim is made.
Dev/CROSS/ordinary are evaluated together at256/512 with normal greedy and no teacher;
watch rows are the exact registered ordinary subset. The same existing trainer/loss/
Adam retains absolute clock and the inherited constant1e-4 LR.

Each command publishes an additional immutable segment-shaped `command.r3er` after
close/stop, recording elapsed command time through that boundary. It does not replace
the original terminal. Pair budget counts these records across both arms; an unclosed
command blocks automatic retry. Pair caps are1024 updates,7500 generations and7200s;
segment1800s with bounded120s cleanup. A terminal's clean time resume cannot reset them.
# Command finalization and journal retry acknowledgement

R3ER kind8 is a replacement-study input snapshot. It explicitly encodes an
authorization-present byte, the existing typed snapshot, then purpose0 Anchor,
1 SaveContinuous, or2 SaveSplit. The new contract binds source and read-only origin
FileRefs into input identity. Preflight has exactly two frozen draws, no quality
evaluation schedule and candidate=false. Kind9 is its save/resume proof: pair,
source and binary digests; two command FileRefs; two parity-panel FileRefs; exact
elapsed Scalar and actual generation count. It is emitted only after both complete
native endpoints, state and output equality are verified. Old readers reject these
kinds; no historical input/command is silently upgraded.

Replacement evaluation checkpoints are saved and reloaded before expensive panels.
A clean time-only partial panel may continue in a new immutable record at the same
model/step, retaining the exact completed row prefix and preserving the old raw.
An interrupted last row can be attempted again and is counted again. Changed complete
observations, mismatched prefixes, cancelled/integrity-failed ancestry remain errors.
This continuation never permits optimizer progress past a pending guard decision.

R3ER v1 record kind7 is a typed CommandOutcome: run digest32, binding digest32,
terminal FileRef, optional comparison FileRef, status byte (0 Complete,1 TimePause,
2 Failed), bounded unique stop tags, optional UTF-8 error, exact Scalar elapsed.
Actual work counters come from the referenced immutable terminal, not a second
counter copy. Command records never replace terminals. Historical kind4 command
copies are not upgraded to positive finalization. Unknown kinds/statuses fail closed.

The run writes terminal, verifies/publishes provisional comparison, then observes
the final cooperative cancellation/deadline boundary and publishes command outcome.
Separate durable writes are not a transaction. Only a verified Complete outcome
bound to the actual terminal and comparison authorizes normal pair progress.
Missing/failed finalization, close-stop or stored stop prevents approval; even a
comparison without command outcome is provisional. TimePause permits only the same
arm's verified continuation. Read-only report never repairs or finalizes an arm.
The command decision seals before its finite publication; synchronous sync/tensor
operations cannot be forcibly preempted. A failed publication returns an error.

R3JRN v1 layout is unchanged. Both new append and identical-content retry call the
same journal-local sync_all helper before ACK. Retry first poisons the writer;
success clears only that temporary poison, returns the original Commit and adds no
frame/view/sequence. Sync failure keeps the instance poisoned until explicit reopen,
whose retry must sync again. Replay/last() proves observed bytes, not a prior durable
ACK. This is a sync_all software boundary, not proof of power-loss durability or
equivalence with SQLite/macOS fullfsync. Previous throughput comparisons remain historical.
