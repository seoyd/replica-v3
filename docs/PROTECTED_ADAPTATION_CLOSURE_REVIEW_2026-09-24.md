# Protected adaptation / FP4 — independent contract closure review

**RESULT: FAIL for the complete B reproduction requirement. One Medium finding.**
The prior A acceptance remains valid. Existing P raw/native/trace integrity and
42-row reproduction remain supported. P model quality remains FAIL. Q codec and
its fixed paired256 evidence remain supported; Goal1 is NOT_ACCEPTED.
No product patch or new model execution was performed.

**Identity and review scope.** REVIEWED_SOURCE and IMPLEMENTATION_EXECUTION_SOURCE
are both `849d8bb39bebfd9a6e0194f60ad2680b63e2adf1`; the source digest is
`fa0080d2abbc7376f89c0df73560b4522756e779366f3d10607f95000fe7b952`.
The incoming implementation-report HEAD is
`a86a10a81bfae84bd96f570d8628b0f01b362f4c`. This new report is published separately;
its publication commit is the REPORT_SHA, not a new source candidate.

The entire combined shared contract and reviewer prompt for
R3-PROTECTED-ADAPTATION-PRECISION-1.0 were read. Their local content digest is
`8ffedd3361ee5557ac57b2488901008015300f697dd6a9997ef4581d4dc856b7`.
Review covered the changed q/v forward path, adapter-only gradients/Adam,
native delta loading/publication, training clocks/objective, preparation/tape,
evaluation/stop/reviewer entry and experimental FP4 path, with directly linked
tests. No unrelated security or storage review was added. The repository's
replica-lean-development workflow was applied to reuse unchanged evidence.

**Finding 1 — B's fixed normal sample contains10 rows rather than32.**

- **Severity:** Medium.
- **Location:** `src/word_value.rs:418`, `review_cases`, especially425–429;
  `review` at431–435. The new adapter profile enters this existing selector
  through `word::is` and the `fresh word-review` CLI.
- **Evidence:** The reviewer contract's B section requires normal32 and up to32
  failed-pair rows as separate diagnostics, within64 calls. `if at < 2` adds
  only the first pair from each panel. The actual14464 adapter endpoint has
  five64-row panels, so the fixed normal subset is2×5=10. The failure branch
  independently appends32 rows. The command succeeds when those42 rows match;
  it does not check the required normal count. Existing B report lines48–53
  explicitly describe42 rows, and its opening PASS covers that bounded replay.
  That narrower result is insufficient for the complete B contract.
- **Problem:** The inherited word-study sampling rule was reused by the adapter
  study without implementing its normal32 requirement. Correct execution of
  the selected42 outputs does not establish execution of the missing22 normal
  samples. Failure-selected rows do not substitute for the fixed normal sample.
- **Impact:** B's declared coverage is incomplete. This does not invalidate the
 640-row raw/teacher audit, the42 observed matches or the genuine model-quality
  failure. It does not change candidate, resume, confirmation or Goal1 authority.
- **Reproduction:** At the preserved14464 endpoint, enumerate the five final
  panels, take each first complete query pair and then the existing32 failed
  rows. The independent Rust checker reads the original raw and saved replay,
  matches all42 IDs/tokens/text/finish/error states, and reports
  `required_normal32 actual_normal10 failed_pair_rows32 missing_normal22`.
  It exits2 intentionally for the unmet requirement, with model calls0.
- **Recommended Fix:** Give this adapter profile a deterministic normal32
  selection independent of correctness, retaining complete pair orientation
  and the contracted panel coverage. Keep failed pairs separately identified,
  bounded by32 and the total64-call budget. Bind selection/counts into the
  existing execution identity and require them when accepting the replay.
  Preserve the prior word profile's accepted behavior and all existing
  RETURNED records. Any completion must reuse those records and generate only
  the missing authorized sample; do not rerun training or silently replace
  the old42-row receipt. The old immutable report remains intact; this report
  supplies the narrower contract verdict.

**Verification performed now versus evidence reused.**

| Area | Evidence class | Result |
|---|---|---|
| Current product source/Cargo/tests versus849d8bb | New static/Git comparison | Identical; no tracked product changes |
| Source/binary/data/report/receipt identity | New Rust execution, exit0 | 198 named files checked before/after; unchanged |
| B selector versus actual returned rows | New Rust execution, exit2 |42 production rows match; normal sample10 instead of32 |
| q/v scalar arithmetic, A/B gradients, frozen registry, cached/prefill path | Reused unchanged executable evidence |2 direct tests PASS; not rerun |
| TINY native/process and ANSWER batch8 versus4+4 | Reused independent A | Continuous2 versus fresh-process1+1 and evaluation-only PASS; not rerun |
| Native corruption/publication regressions | Reused unchanged evidence |3 tests PASS; not rerun |
| FP4 scalar/matrix/codec regression | Reused unchanged evidence |1 test PASS; not rerun |
| P raw640, teacher640, trace128, native endpoints | Reused independent B after identity checks | Evidence integrity PASS |
| P fresh reproduction | Reused same-source B |42/42, including26 incorrect answers; required normal coverage FAIL |
| Q paired V128/VC128 and native parity16 | Reused same-source evidence | No observed paired regression; not rerun |
| Additional normal22 reproduction | NOT_RUN | Product selector requirement remains unsatisfied |
| Full+1024 dev / conditional fit1536 / QA640 / confirmation / S4/S5/S6 | NOT_RUN | Not inferred from this review |

New readers were compiled locally with installed Rust1.98.1 and the existing
locked release rlib. No Cargo/dependency download or rebuild occurred. The rlib
SHA256 is `16c298d95b7c022a5fb32beb67e1389c4b5e5459cbb720001bfa630a49984e91`.
Scratch executables, sources, build logs, actual command/exit record and receipts
are under `artifacts/protected-adaptation-20260924-closure-review/`.

| New local evidence | SHA256 |
|---|---|
| `verify.rs` | `822b88b78af1a5f17d5da8afcdf76584b74eb3dfd66e8f12e9494c1cd4f99b24` |
| `verify` | `b8bbed1d41502c1d76f582c6633402005f48ea175d486f9f9ff08de00f7fbab4` |
| `identity.r3b` | `99381ce9d05cc3017a731e4fa4709e2f8bc71c9aefd1ea7893a42b34fbad0eab` |
| `sampling.rs` | `77e7a1c33c6f2f5fbafc525ecd34b3c50f81e4fe3346aca8fe1a8478b6c24d0b` |
| `sampling` | `9a4847b398f5b04719b993686fb633abca1ab4af4c1d140c610a306d1dbcc22e` |
| `sampling.r3b` | `38b27659153100d6abbc07a385325fb144164178c755e31722f4cbc1bcace07d` |

The identity reader's `existing_B=PASS_REUSED_IDENTITY_VERIFIED` means the old
receipt and its narrower result are unchanged. The subsequent sampling check
establishes the full-contract FAIL; these are different checks. No zero-test
invocation or old log was counted as a newly executed model/test PASS.

**Separate decisions.**

| Decision | Result and limit |
|---|---|
| A / adapter implementation integrity | PASS retained; no newly confirmed q/v, gradient, base or resume defect |
| Complete changed code/reviewer-contract scope | FAIL: normal32 selection missing |
| B raw/trace/native integrity | PASS, reused evidence with identities rechecked |
| Complete B acceptance | FAIL: normal22 missing |
| Frozen base preservation | PASS; original14336 weights/Adam file remains unchanged |
| Active retention / word quality | FAIL: at+128, S1Q1 FULL17/64 and both word panels0/64 |
| Q codec / numeric measurement | PASS within experimental E2M1 storage scope |
| Q model delta | Measured: V128 and VC128 both128/128, delta0 on this fixed sample |
| Q execution | Dequantized F32 CPU/Accelerate; native FP4 speedup/RAM/S6 not established |
| Delta storage | Payload239,616B weights +479,232B Adam; actual final file733,966B, referencing one original base |
| Goal1 | NOT_ACCEPTED |

The same adapter is active for old and new questions. Base-weight immutability
therefore does not imply active-output preservation. At the closed endpoint,
VC support55/64/outside9 and S1Q1 support21/64/outside40 remain failures; outputs
were not repaired. Original native14336 physical hash remains
`15015900a3c91a6b8ed87e9c2ecb8f7faf124849e5dcf8f40203cddf267e30d9`.
Final delta physical hash remains
`de9064bffc95aa8f09e5653c2878c0ee44ad8bd17349f746a2e9409532bcb9a2`.
P stopped after128 updates with `SEVERE_RETENTION_REGRESSION`, resume=false;
this run cannot continue or promote a candidate.

The FP4 artifact remains6,713,446B. Its F32 scale/embedding overhead and
F32-dequantized computation are distinct from its four-bit stored codes.
The paired256 result is a measured subset, not general losslessness or Goal1.
No full artifact inventory, cleanup, deletion or recompression was performed.

**Usage.** This review adds optimizer0, generation0, teacher0, whole-model
forward0 and backward0, for both SMALL and TINY. Preserved totals remain SMALL
optimizer128/generation970/teacher640, output13,073 tokens, and TINY18/180/180.
The existing independent TINY process consumed8/72/72; it was not repeated.
The source, original receipts and original reports were not rewritten. Only
this new report is eligible for publication; local scratch is ignored.

**Necessary tests after a fix.**

- Unit: exact normal32 count, complete pairs, deterministic selection independent
  of correctness; failed rows separately bounded by32.
- Integration/regression: the real adapter `word-review` route on the actual
  five-panel shape must validate normal and failure counts, preserve the older
  word-study profile and enforce the64-call cap.
- Boundary/failure path: zero failures, more than32 failed rows, too few complete
  normal pairs and a valid42-row receipt with insufficient normal coverage.
  Reject insufficient coverage; do not publish complete B acceptance.
- Completion: preserve/reuse already RETURNED rows and verify only the missing
  authorized sample. No repeated SMALL learning or unrelated suite is needed.

No other confirmed issue was found in the reviewed scope. The review closes
with the finding above; no additional speculative repair is requested.
