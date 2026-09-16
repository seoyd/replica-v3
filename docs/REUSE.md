# Selective reuse

Read local v2 Cargo.toml, src/lib.rs, snapshot codec/reader/CRC helpers,
local_store bounded reads, README.md and WORK.md. Remote read-only ls-remote
reports contract baseline; local HEAD is later. No v2 build/path dependency.

| source_repo | commit | path | symbol | reused_or_reimplemented | reason | destination | test | license |
|---|---|---|---|---|---|---|---|---|
| seoyd/replica-v2 | 6da531adebde3c4d001a313b2cdd180ae783353f | src/snapshot.rs | canonical_varint, decode_canonical_varint | adapted | isolated 10-byte canonical LEB128, replace error/constant names | src/codec.rs | tests/codec.rs | MIT per source Cargo.toml; no standalone license file present |
| seoyd/replica-v2 | 6da531adebde3c4d001a313b2cdd180ae783353f | src/snapshot.rs | crc32c, Reader::exact | independently implemented/library | standard CRC32C crate and checked slice reader avoid BrainGraph dependency | src/codec.rs | tests/codec.rs | v2 MIT metadata; dependency license in Cargo metadata |
| seoyd/replica-v2 | 6da531adebde3c4d001a313b2cdd180ae783353f | src/local_store.rs | read_bounded_file_bytes, persist_encoded | independently implemented / persistence excluded | bounded read retained as a principle; SQLite owns atomic commit/backup, no file WAL | src/main.rs, src/model.rs, src/store.rs | tests/cli.rs, tests/runtime.rs, tests/store.rs | v2 MIT metadata |
| seoyd/replica-v2 | 6da531adebde3c4d001a313b2cdd180ae783353f | src/snapshot.rs | encode_evidence, decode_evidence | independently implemented | preserve provenance/time semantics with explicit linear slots; avoid policy/core types | src/event.rs, src/store.rs | tests/store.rs | v2 MIT metadata |

No BrainCore, super::* imports, authority seal, MIR audits or atomic snapshot
replacement copied. CRC crate uses the standard polynomial; no copied CRC table.


The native model owns its Transformer blocks, training loop and generation loop.
Only generic Candle tensor/autograd/activation operations are reused. The former
external model adapter and its dependency have been removed. Own byte BPE learns
from the declared training corpus; its artifact is bundled with our checkpoint.
No Python or C++ worker or bridge was created.

## 읽기 전용 archive에서 재사용한 v3 의미

이번에는 v2를 새로 열거나 추가 소스를 가져오지 않았다. 위에 기록한 v2 출처와 별개로
현재 v3 4edf7159518108be302813d704ca0ff5db449395의 event/codec/store/retrieval을
기준으로 확장했다. Event::edges와 순수 linkage validator는 SQL projection과 archive가
함께 쓰고, 기존 bounded BFS는 EvidenceRead의 SQL/archive 입력으로 실행한다. 의미/한도는
공유하되 SQLite FTS5는 archive가 구현했다고 표시하지 않는다. 기존 v3 tests/codec.rs의
독립33-byte literal을 archive 독립249-byte fixture 안에 그대로 넣어 검증했다.

새 module은 읽기 전용 binary directory/block/index 수명과 검증을 맡는 archive.rs 한 개다.
일반 DB framework, v2 BrainGraph/policy/BC loop, 모델 정답 selector는 가져오지 않았다.
취소와 DependsOn의 명시 tag는 새 schema2이며 과거와의 byte compatibility를 가정하지 않는다.
