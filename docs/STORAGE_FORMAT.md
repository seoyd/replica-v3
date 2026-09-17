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
