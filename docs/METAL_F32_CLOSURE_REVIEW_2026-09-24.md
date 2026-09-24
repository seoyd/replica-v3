# Independent closure — LegacyB and Metal F32 runtime

**LegacyB: PASS / closed. Runtime A: FAIL / NOT_ACCEPTED.
Runtime B: BLOCKED by Runtime A; NOT_RUN.** The known Metal numerical defect
remains confirmed; no additional product defect was established in this review.
Adapter model quality remains FAIL. Protected output quality, performance gain,
S4/S5/S6 and Goal1 are not accepted.

**Reviewed and executed identities.** Reviewed candidate:
`f4ae62b647ffecfb89129e775d687c218f410ab3`.
The complete combined shared contract and reviewer prompt for
R3-METAL-F32-BASELINE-1.0 were read; their local content SHA256 is
`010af8fc6c76fed34c54e2fbd3ea33c20de3876bac55eec25659baf167d0ca82`.
No temporary instruction was added to a build, policy or published source link.
The repository review skill was applied to reuse unchanged execution evidence.

LegacyB executed source is the M1 source published in
`da1987d3a5a6ba74c1d25eec121fb483c97d24f9`; its changed files `fresh.rs` and
`word_value.rs` remain byte-identical in this candidate. Runtime A's executed
worktree was that M1 commit plus `m2-candidate.diff`; the current Git source/test/
Cargo diff independently hashes to the exact same
`94a967e91410548bc8788c4ad8647607409efb9a148891357d817ca80c91a5a7`.
This new review report is a separate report-only publication, not a new product
candidate or a newly executed Metal binary.

| Frozen execution evidence | Rechecked SHA256 |
|---|---|
| M1 `replica-train-m1` | `ebd143525598ec322feded1524f61b8ceb8d5df94506879c0fc34e89f615d68d` |
| M2 final tests | `3bd26da935b4c55df7c771a28fa9add80ace9c3719781955f3ff0df41c567544` |
| M2 initial numeric tests | `47452e4820d7734a49133f34212f1050e69b917b634f3da22af25910cb07f719` |
| M2 product executable | `d48dcce4ea2c6953213bd89db49e04e3fc18c0f5433c8750e88c56fd8cdf226c` |
| Current Cargo.lock | `52f4672ddd7d140e4fad7f55cf7a42c11c0224ec03b5ead8a208924d4bf95e6d` |
| Existing independent Runtime A report | `b95903374acb5dba023beaaa1a3251b0b73587df957c953ef01af34d7faef31f` |
| Existing independent GPU failure log | `87bb7210713f7f23eeb40c916ef50784985e564c0343433a3ade8047e1e12808` |

**LegacyB completion.** The real adapter-only completion path now selects fixed
metadata prefixes V8/VC6/S1Q1 6/word6/renamed6, preserving oriented complete query
pairs independently of correctness. The old general word selector is unchanged;
adapter entry through its old replay command is rejected.

The independent reader was copied to isolated scratch with only its output
receipt destination changed, compiled with existing locked libraries and run
once. Build and execution both exited0. It reads actual corpus/metadata and
raw, verifies old/new source and binary identities, model/step/request/prompt,
prepared/resolved RETURNED records and output equality, and independently
computes the normal set, reuse, difference and unique calls.

| Quantity | Verified result |
|---|---:|
| Fixed normal coverage |32 |
| Retained failure coverage |32 |
| Normal/failure overlap |6 |
| Old RETURNED observations preserved |42 |
| Normal rows reused from those observations |16 |
| Supplement originally generated |16 |
| Unique physical observations |58 |
| Normal FULL / EOS |14 /30 |

Thus `32 + 32 - 6 = 42 + 16 = 58 <= 64`. The supplement contains exactly the
set difference, within the22-call limit. It adds224 recorded output tokens and
2.10707125 recorded active seconds, teacher/optimizer0. These are existing
execution costs, not calls made by this review. Two returned length failures
remain failures. Original32 failure rows were not regenerated or relabeled.

The new readback receipt is byte-identical to the earlier independent result:
`ec739b3a471b65c2865554ebed1c68b4e987e7c8439e499a8aa77b134cdbd4f6`.
The study registration and separate closure retain original producer provenance,
`B_COVERAGE_COMPLETE`, model_quality=FAIL and resume=false. The earlier Medium
normal-sample finding is resolved in this scope; A and the failed14464 model
are not reopened.

**Confirmed issue — Metal middle-axis reduction violates the numeric contract.**

- **Severity:** Medium. This is the already reported numerical blocker, not a
  newly discovered training-state corruption.
- **Location:** `src/neural/transformer.rs:428`, `repeat_kv`, called by
  `gqa_attention`; installed Candle0.11.0 `backprop.rs:479` Broadcast backward
  and `metal_backend/mod.rs:299`, `reduce_op`. The minimal regression is
  `src/training.rs`, `metal_f32_middle_axis_sum_oracle`.
- **Evidence:** `repeat_kv` broadcasts a group axis before reshape. Its Broadcast
  derivative invokes `grad.sum_keepdim(sum_dims)`. The preserved same-source
  GPU regression uses F32 `[8,4,2,12,8]`, contiguous strides
  `[768,192,96,8,1]`, and reduces axis2. Scalar/CPU output0 is `-0.8125`;
  Metal returns `-0.3125`. Max error is1.65625 and RMS0.6269952080567337.
  The test synchronizes the actual Metal device before reading values.
- **Problem:** The result exceeds the registered
  `1e-4 + 1e-3 * abs(reference)` tolerance. At output0 the allowed error is
  0.0009125, versus actual0.5. This is not a demand for bitwise equality.
  Related recorded `repeat_kv` gradient NRMSE1.488532391 and full TINY
  embedding-gradient NRMSE0.479273607 also fail the gradient gate. These
  symptoms are grouped under the demonstrated reduction boundary; the review
  does not claim it exhaustively explains every gradient difference.
- **Impact:** Metal backward/update acceptance is blocked. The shared Adam
  entry at `src/training.rs:310` rejects Metal Vars before moment or weight
  mutation. No SMALL/TINY optimizer update was accepted. This does not
  retroactively explain prior CPU quality failures or prove all Metal
  operations wrong. Inference protection/worker/restart acceptance remains
  unverified, rather than inferred from the small forward test.
- **Reproduction:** The exact frozen test invocation is
  `replica-train-m2-tests --ignored --exact training::metal_tests::metal_f32_middle_axis_sum_oracle --nocapture --test-threads=1`,
  with Accelerate/Rayon/OMP threads1. The preserved independent run executed
  one test, exit101, 0 passed/1 failed, on Metal gpu_id4294968525. Its source,
  binary and log identities were rechecked here; the GPU run was not repeated.
- **Recommended Fix:** Keep the current Metal optimizer rejection. Any later
  authorized repair must correct the GPU reduction's axis/stride mapping:
  sum only the broadcast group dimension while preserving batch/head/token/
  channel coordinates. The concrete repair boundary is the selected Metal
  strided reduction or a separately verified, mathematically equivalent GPU
  reduction used by `repeat_kv` backward. Do not modify masks/answers, raise
  tolerances, silently compute on CPU or remove the guard to obtain PASS.
  No dependency/kernel/product patch is applied or required to close this
  blocked review; a repair belongs to a subsequent authorized scope.

**Runtime findings and acceptance limits.** CPU remains the default. Metal
selection is explicit and failure to open the device is an error, not fallback.
Input/model/cache device checks and rejection of CPU-only RustDecodeGemv are
present. Worker runtime_revision reflects the selected backend and is checked
by its parent. Native save synchronizes before serialization/publication.
Historical architecture/weight identity and numeric policy are unchanged.

Generation checks logits and reads argmax back before recording first-token and
completion timing. The new status report does not claim queue dispatch time as
GPU throughput. The synchronized benchmark was not run; no prefill/decode,
TTFT, speedup or allocator-peak acceptance is inferred. The complete runtime
provenance descriptor and backend-bound trainer resume remain incomplete and
are disclosed as such.

| Scope | Verdict | Evidence class |
|---|---|---|
| LegacyB coverage | PASS, closed | New readback plus identity-bound prior CLI execution |
| Small padded Metal forward and CE | PASS for recorded fixture only | Reused implementation evidence |
| Metal backward/reduction | FAIL | Reused independent actual-GPU failure, identities verified |
| Device/cache/kernel and Adam rejection | PASS for tested admission boundary | Reused direct test plus current source review |
| Metal update / same-backend restart | BLOCKED / NOT_RUN | No accepted update or restart evidence |
| Runtime A overall | NOT_ACCEPTED | Numerical prerequisite failed |
| Runtime B | BLOCKED / NOT_RUN | No downstream generation authorized by failed A |
| Protected P128/C64 output quality | NOT_RUN | No quality-preservation PASS |
| CPU fixed SMALL outputs / full worker comparison | NOT_RUN | Existing narrow CPU regressions are not substitutes |
| Performance gain / mixed-precision execution | NOT_ESTABLISHED | No benchmark or reduced-precision run |
| Adapter quality / S4/S5/S6/Goal1 | FAIL / NOT_ACCEPTED | Coverage repair grants no model promotion |

**Necessary validation after an authorized numerical repair.** Reuse the
existing failing scalar-reduction, `repeat_kv` gradient and native TINY tests;
run those affected boundaries after the fix. Exercise group factors2/4 and
padding/layout boundaries, then apply the existing ANSWER gradient and
first-update criteria before considering backend-bound restart. The unchanged
LegacyB tests/coverage, historical128-step learning, FP4256, QA640 and full
artifact audit need no repetition. No new speculative test suite is requested.

**New execution and preservation.** This review made zero GPU primitive,
model-forward, backward, generation, teacher and optimizer calls. It ran one
Rust raw/receipt readback, not a new model test. Existing GPU failures and CPU
regressions were reused after identity checks; NOT_RUN was not counted as PASS.
The current contract's recorded generation usage remains16 for M1, with
SMALL/TINY optimizer0. No original source/tests/Cargo/model/Adam/corpus/raw or
previous report was changed; the named original preservation manifest passes.
No whole artifact traversal, cleanup, move, deletion or compression occurred.

New source/log/receipt files are in
`artifacts/metal-f32-20260924-closure-review/`. Reader source SHA256 is
`69cfa3540173cb7763855d144063e318113d579c17c5fcfa2245e4c7c47decfc`,
reader executable `530f4abc1fa8585034cb94d835e0d16275eebbf1f9dffd061e1a3de875833da0`.
The unchanged CPU reader rlib is
`16c298d95b7c022a5fb32beb67e1389c4b5e5459cbb720001bfa630a49984e91`;
it is used for native data/record reading, not represented as a Metal build.
No Cargo rebuild or dependency retrieval occurred in this review.

Only this new report is published. Its commit and the actual remote main SHA
are reported separately from the reviewed/executed source. LegacyB is closed;
Runtime A is closed as NOT_ACCEPTED and Runtime B remains blocked. No additional
GPU execution, learning, quality promotion or confirmation is started.
