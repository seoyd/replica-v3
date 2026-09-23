# Artifact hygiene and finalization — independent closure

**FINALIZATION_CODE / INDEPENDENT_A: PASS. ARTIFACT_AUDIT / PLAN_REVIEW: PASS for the fixed dry-run plan below.** No confirmed defect remains in this reviewed scope. No additional product patch or repeated stabilization test is required. This review grants no deletion or model-quality acceptance.

The complete shared contract and reviewer instructions for R3-ARTIFACT-HYGIENE-AND-FINALIZATION-1.0 were read. The repository's lean-development workflow was applied: unchanged, identity-bound process evidence was reused; the closure adds a small independent Rust readback, without repeating model reproduction, QA640, the inventory traversal, or candidate/model content hashing.

| Identity | Value |
|---|---|
| Reviewed final candidate | `e2e354a7f24babb61d13ac2f1266c1cfd12d7b0e` |
| Finalization source within candidate | `1fb463dbdca34e3119124f3d9918310840c5494e` |
| Review-start report HEAD and directly checked remote main | `2219403b828447fea8b28ccc05e61de6ebc2f77a` |
| Actual candidate diff SHA-256 | `0104827ab321f32d10e3f0c25a98c53eae143e3cc217145562802177f6eae84d` |
| Accepted dry-run plan SHA-256 | `429ff481d8f9538bc97bda286f33d8e70b605ab686619ad7a4d12342f14ae531` |

The commit adding this report is a report-only publication, not the reviewed source. Its full commit and actual remote SHA are delivered after publication. Product source/tests/Cargo remain byte-identical to the candidate; the existing untracked `.DS_Store` was preserved.

## A — finalization

The changed production path is `bridge_diagnostic_qa` → `bridge_diagnostic_output` → `segmented_returned` / confirmed publication / strict scoring, with `observation_control` still required for remaining model work. The no-call branch checks producer/input/native identity, segment histories, every RETURNED resolution and the raw prefix before finalizing. It has no inference entry, preserves active usage, and records the new finalizer separately. Candidate/S4 authority remains false. Management uses the existing 900-second control and bounded observation inputs; UNKNOWN and pending publication remain sticky.

The [existing independent A report](ARTIFACT_HYGIENE_REVIEW_A_2026-09-24.md) records one successful test, exit 0, with **15 actual fresh child processes** invoking the shared production output caller. Its frozen binary SHA-256 `8150a21412d692325a6700fe0cd9a0ef2ef51278d7e6ee62596997f4412c4080`, source files and parent log were rehashed and matched. This is reused dynamic evidence, not 15 newly executed processes in this closure.

The existing cases cover RETURNED4/remaining0/active7200 finalization, unchanged reentry, remaining1 rejection, missing/UNKNOWN resolution, other model/policy, reordered/truncated/extra/duplicate raw, cancellation, TIME mixed with I/O, and pending state after final-publication sync failure. The model-entry fault guard is present. The closure reader checked all 15 child logs, absence of finals in rejection fixtures, and the preserved pending marker after sync failure.

It also read the successful four-row raw and actual RETURNED resolutions, verified final/score/start/receipt hash bindings, active7200, historical generation count4, new finalization calls0, producer/finalizer separation, and unchanged original fixture raw. **New A process tests: 0; new model/teacher/optimizer/backward calls: 0.** Existing completed model reproduction and QA640 were not repeated.

## B — dry-run protection and accounting

The [existing independent B report](ARTIFACT_HYGIENE_PLAN_REVIEW_B_2026-09-24.md) and its original/delta Rust readers were inspected. Their executable/source and receipt hashes matched; the corrected-plan receipt binds the current plan and prior full inventory/classification recount. The historical owner-name error is corrected in this plan; the original plan and report remain preserved.

The new reader checked every exact candidate-shard entry and recomputed candidate counts/unique-inode totals. Candidates are exclusively non-executable, single-link regular compiler `.o`, `dep-graph.bin`, `query-cache.bin`, and `work-products.bin` files under the three closed precision/answer-mean/query-signal incremental roots. Whole directories, source copies, executables/deps, readers/patches/logs, native/Adam, raw/corpus and unknown files are excluded.

It freshly checked no-follow metadata for **15 representatives**, covering each candidate type and each unit's largest file, all **20 registered reference records**, and the **three keeper source/Cargo/report bindings**. Optional incremental state can be rebuilt using the retained source/lock and recorded commands; byte-identical executable reproduction is not promised, and the existing executable/dependency files remain keepers. The prior complete metadata/reference/classification review is reused, not described as a new check of every live filesystem entry. Unknown subtrees remain protected.

| Recorded inventory/plan amount | Bytes |
|---|---:|
| Logical regular bytes | 184,316,744,031 |
| Unique-inode logical bytes | 183,211,608,097 |
| Unique-inode allocation estimate | 185,742,196,736 |
| Protected/kept logical bytes | 173,710,433,836 |
| Proposed 18,141 files: unique logical bytes | 10,606,310,195 |
| Proposed allocation estimate | 10,643,812,352 |
| Two protected native files: selected duplicate upper bound | 114,181,184 |
| Actual deleted / reclaimed bytes | 0 / 0 |

HOT_PROTECTED 8,617,790,378, COLD_EVIDENCE 84,687,522,825 and UNKNOWN_OR_ACTIVE 80,405,120,633 bytes sum to the kept total. The duplicate amount compares whole native files, including Adam, and is not added to purge candidates. Both native paths remain KEEP. Logical sizes and stat allocation are not guaranteed APFS physical recovery; clone/snapshot/open-file effects remain unmeasured. These are the preserved inventory's measurements, not a new volume scan.

**PURGE_AUTHORITY=NOT_AUTHORIZED; APPLIED=false.** The tool exposes inventory/analyze only. PLAN PASS does not authorize original deletion, unlink/hardlink substitution, archive conversion or another manifest. Any future application needs separate approval of the exact plan and fresh identity/hash/reference/keeper/writer checks. Apply fault tests are NOT_RUN because no apply implementation is being accepted. ARCHIVE_PROBE remains NOT_RUN: no suitable existing bounded archive/restore tool was established.

## This closure's execution and preservation

The isolated reader is `artifacts/artifact-hygiene-20260924-review-closure/verify.rs`. Installed Rust 1.98.1 compiled it against the existing locked `replica_v3` and `sha2` rlibs, without Cargo rebuild or dependency/network acquisition. Build exit 0; reader exit 0; measured reader time **0.535 seconds**. The reader and log are retained locally.

```sh
rustc --edition=2021 artifacts/artifact-hygiene-20260924-review-closure/verify.rs --extern replica_v3=target/release/deps/libreplica_v3-5f74ea5b6a70fcf8.rlib --extern sha2=target/release/deps/libsha2-961a6422faaaf604.rlib -L dependency=target/release/deps -o artifacts/artifact-hygiene-20260924-review-closure/verify
VECLIB_MAXIMUM_THREADS=1 OMP_NUM_THREADS=1 artifacts/artifact-hygiene-20260924-review-closure/verify
```

| New evidence | SHA-256 |
|---|---|
| Reader source | `79940cc1ef0d7cb40f97e937d3407d331551708529112c161e352fb1a9b9bc8f` |
| Reader executable | `8ec9fe6c186038752354ef9b3080427da8aeff576fc31fb7785b17b2edaf3396` |
| `verification.r3b` | `361c0ade58c30379987e6d7e8c639bd88d10a05f08f189611be0f221dfc0ea03` |
| `verification.log` | `644eab2cd96569f78051399f8c3b2281a677fe97cad57a4145d60cebf2c6d80c` |

All 113 consumed reader inputs retained their before/after hashes. Reader byte accounting is 4,108,761 logical bytes plus 2,620,703 bytes for preservation rehash; it is not physical I/O or whole-command accounting. Cleanup-candidate and native file contents were not rehashed. Only new scratch evidence and this report were written; no original model, corpus, raw, seal, receipt or prior report was modified. Whole-volume peak/free-space measurement was not repeated.

**MODEL_CALLS / UPDATES: 0 / 0. PROTECTED11264_ACCEPTANCE: PRESERVED. BRIDGE_QUALITY: existing FAIL. OLD_QA: existing 0/640, not a new measurement. S4/S5/S6/GOAL1: NOT_ACCEPTED.** The reviewed A boundary and B dry-run plan are closed; no further regression or nominal repair is required by the verified scope.
