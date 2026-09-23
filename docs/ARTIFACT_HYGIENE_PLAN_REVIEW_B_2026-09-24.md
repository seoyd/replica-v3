# Artifact hygiene — independent cleanup-plan review B

**Verdict: PASS for the corrected dry-run plan.** No deletion is authorized or performed. This is a review of the proposed plan, not an approval to apply it.

## Reviewed scope and actual work

The reviewer read `examples/artifact_audit.rs`, the three closed-review provenance reports, and the actual R3BIN inventory/plan shards. A small isolated Rust reader reused the existing `replica_v3::binary` codec. It ran once, **exit 0 in 36.139 seconds**, with no model, training, teacher, backward, archive, or deletion calls.

The reader verified every inventory shard and its corresponding classification shard, all candidate shards, immutable metadata fields, candidate-to-classification equality, unique inode accounting, report/keeper hashes, registered reference records, and candidate exclusion of the two protected final checkpoints. It also rechecked the current no-follow path, device/inode, size, mtime, mode, and nlink of every candidate. It did not repeat the implementer's 10.6 GB candidate-content hashing or the full inventory traversal.

| Evidence | SHA-256 |
|---|---|
| Original cleanup plan | `137251b71697e2516183462b8ac47a9ed33b52709fa2924816794271ec270f5f` |
| Corrected cleanup plan | `429ff481d8f9538bc97bda286f33d8e70b605ab686619ad7a4d12342f14ae531` |
| Independent reader source | `d68480b812ca4d21784d1ffa128fca37c9bf2bf73ac7b583ffeaf39b5af01878` |
| Independent reader executable | `e9bcaaf76a35e04113cc12dc8c018e312d343a16251ae92662f616920d43e6fd` |
| Independent plan recount receipt | `acd568a4a78e7cc0a455dc14b212c073fc3cb4709fc4adb34bdf4f5c4f3f23eb` |
| Original audit executable | `7bbb424282c63143a1017bbc6b13dde85bc0851147ee2d01e6a3105461fcce98` |
| Corrected audit source | `5990b8bbd15ecaf7d35899df3fafb4e099b91ea34584f361872d7b506e13f02d` |
| Corrected audit executable | `038897b3468b957d31f568cc66723221a1bcf689539fb56853be333732656359` |
| Independent delta reader source | `ae38049b30a181cfe610475bf2f90ecaf5fe3d2e11d1d64531e85dcf86a6b5de` |
| Independent delta receipt | `1f353f9c858ee41a2c145becd8fbe6d84b34ec8b577c604abfe327e111ba58db` |

Local evidence: `artifacts/artifact-hygiene-20260924-independent-b/`, including `read_plan.rs`, `recount.log`, `plan-recount.r3b`, `read_delta.rs`, `delta-recount.log`, and `delta-recount.r3b`. The readers were built directly with the existing locked dependency artifacts; no dependency acquisition or Cargo rebuild was needed. An initial delta-reader compile had a Rust slice-reference type error; its log remains, the scratch helper was corrected, and the following build and execution both exited 0. This did not modify the audit source or consume model calls.

## Verified amounts

| Measure | Actual recorded value |
|---|---:|
| Inventory entries | 895,166 |
| Regular files | 860,719 |
| Logical bytes | 184,316,744,031 |
| Unique-inode regular bytes | 183,211,608,097 |
| Unique-inode allocation estimate | 185,742,196,736 |
| Proposed files | 18,141 |
| Proposed unique logical bytes | 10,606,310,195 |
| Proposed allocation estimate | 10,643,812,352 |
| All other kept logical bytes | 173,710,433,836 |
| Selected duplicate upper bound | 114,181,184 |
| Applied deletion / actual reclaimed bytes | 0 / 0 |

The candidate list contains only non-executable, single-link regular `.o`, `dep-graph.bin`, `query-cache.bin`, and `work-products.bin` files inside three explicitly reviewed, closed Cargo incremental directories:

| Review owner | Files | Logical bytes |
|---|---:|---:|
| Citation precision A | 6,330 | 3,789,518,236 |
| Answer-mean citation A | 6,288 | 3,588,388,091 |
| Query-signal independent review | 5,523 | 3,228,403,868 |

These are exact listed files, not whole-directory deletion proposals. Source copies, executable/deps/rlib/dylib files, unique readers, patches, logs, checkpoints, corpus, original failures, and all unfamiliar entries are outside the candidate list. The repository's current root target and Cargo home are not inventory deletion targets. Candidate files have recorded whole-file hashes from the implementer's audit; those historical hashes are reused here, not represented as fresh reviewer content hashes.

The two final checkpoint files remain KEEP and are not purge candidates. Their independently checked metadata and prior whole-file hash evidence identify equal 114,181,184-byte files at distinct inodes, SHA-256 `15015900a3c91a6b8ed87e9c2ecb8f7faf124849e5dcf8f40203cddf267e30d9`. This limited duplicate upper bound is not a model-weight comparison, not added to the purge candidate amount, and not permission to unlink or hardlink either path.

## Confirmed classification finding

The original tool's `OWNERS` list named `instruction-bridge-20260923-study` for the prior bridge study, while the actual step-12800 parent is `qa-integrity-bridge-20260923-study-final`. As a result, that required hot parent did not receive its explicit HOT_PROTECTED classification or the intended known-owner metadata scan. Default KEEP still protected its bytes, so this did not insert a checkpoint into the candidate list or cause any deletion.

The implementer preserved the original plan and issued a derived plan at `artifacts/artifact-hygiene-20260924-plan-final/`. It corrects the actual step-12800 owner and the accepted step-4352 owner (`rebind-consolidation-20260921-study`). Candidate and native whole-file hashes retain their prior-plan provenance; candidate hash timestamps are unchanged, while native reuse records the prior audit's start time. Neither is described as a newly read content hash. Current metadata is rechecked before reuse.

The independent delta reader completed **exit 0 in 3.721 seconds**. All 18,141 exact candidate entries, their original hashes, identity, keeper/provenance, and amounts remain unchanged; only their hash-origin description now records reuse. It checked the 120 changed classification shards against the original and verified all other shard hashes unchanged. The corrected owner subtrees contain 26,322 and 9,641 entries respectively, now HOT_PROTECTED/KEEP. All 20 current known-owner reference records were hash-checked; none refer to a candidate unit. The two final checkpoint exclusions, inventory identity, no-apply state, and duplicate bound are unchanged. No new source defect or plan-safety blocker remains in this scope.

Final kept classification amounts are HOT_PROTECTED **8,617,790,378 bytes**, COLD_EVIDENCE **84,687,522,825 bytes**, and UNKNOWN_OR_ACTIVE **80,405,120,633 bytes**. They sum to the kept amount above. Historical classification mistakes are not erased by the derived result.

## Application limits

The tool has only inventory/analyze commands. `PURGE_AUTHORITY=NOT_AUTHORIZED`, `APPLIED=false`, and reclaimed bytes are zero. The audit's recorded allocation is a stat-based estimate; APFS clones, snapshots, compression, and open files prevent treating it as guaranteed physical recovery. The duplicate survey covers only the two stated checkpoints, not all artifact contents.

Archive probing was not run: the available native probe is not a bounded generic archive/restore tool, and no new archive engine was introduced. The audit preserves unknown subtrees instead of treating their names as deletion permission.

Any later application requires the user's approval of the exact corrected manifest and a fresh no-follow identity/content/reference/keeper/live-writer check. Changed entries or new directory contents must be rejected rather than expanding the list. This review grants no deletion authority, new model work, or change to existing quality and Goal1 verdicts.
