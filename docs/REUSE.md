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
