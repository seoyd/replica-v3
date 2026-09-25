# First-decision independent review — 2026-09-25

## A — FAIL: pair input and native corpus binding disagree

Independent session `first_decision_review_a` reviewed contract
`R3-COST-BOUNDED-FIRST-DECISION-1.0` and candidate
`ff4f8e90726959732cfbace1533b0b93a4d698ef`, relative to product base
`af8ad53bf52e592f531b2f29ea84670ac201e618`. Requested reviewer setting was
Astra High; actual model/effort and account token usage are UNKNOWN.
Product source and original artifacts were read only. No A admission receipt
was issued. The selected v6 study is not accepted for learning.

**P1 — W's native corpus identity and TINY input still select historical I.**
`src/muon.rs:event_inputs` indexes `event.corpus[arm]`; `load_arm` uses this
arm-dependent corpus to set and verify native train/validation hashes. In
contrast, `first_train` builds both C/W batches from `event_inputs(s,0)`, the
FULL corpus. W consequently declares the historical ID_ONLY corpus while its
production training would consume FULL. `event_tiny_steps` uses
`event_inputs(s,arm)`, so its W test actually trains ID_ONLY and does not
exercise the production W input. Both direct and independent within-arm
restart assertions can pass while this contract violation remains.

The independent reader measured these two-step traces from actual Metal runs:

| Arm | Input tokens | Target tokens including EOS | Padding |
| --- | ---: | ---: | ---: |
| C | 3276 | 234 | 452 |
| W | 3340 | 178 | 516 |

The required equality assertion failed with exit101 in
`artifacts/first-decision-20260925-review/a-pair-consistency.log`.
Minimum correction: resolve FULL corpus0 consistently for first-decision
preparation, load/resume bindings, production batches and TINY caller; assert
identical prompt/target tokens, roles, tape and costs across C/W before calls.
Then actual corrected Metal execution must be independently verified. The
current call cap is exhausted, so that execution requires additional explicit
authorization; existing ID_ONLY W results cannot substitute for it.

The earlier cost findings are preserved in `a-static-dc346a0.md` and
`a-static-fe4ae28.md` under the same review root. Static review verified their
targeted corrections: reserve8 stops before another backward, saved lower or
unequal endpoints can finish observation, cost/quality reasons are separate,
and cost-final128 binds the unchanged scheduled decision's hash. Final-source
implementer logs `cost128-002.log` and `cost-stop-005.log` report actual small
caller PASS with zero model calls; A reused these source-bound checks rather
than repeating them. This does not resolve the corpus discrepancy above.

## Executed evidence and limits

Rust/cargo1.98.1; locked offline release with accelerate,metal and
CARGO_INCREMENTAL=0. The no-run build reused the shared target in0.29s.
VECLIB_MAXIMUM_THREADS=1 and OMP_NUM_THREADS=1 were set for test execution.

The existing `first_decision_f64_gradient_masks_and_microbatch` executed one
test, exit0. Its actual loss/gradient fixture covers weight1 delegation,
weight2 normalization, EOS/masks, review/word separation and4+4 example
accumulation. Independent Rust f64 arithmetic checked360 finite differences
on eight mixed-length2/16 answers: maximum error4.584e-11; normalized weight2
was not twice whole loss. The independent reader also verified all256 tape
rows and2048 exposures, source573..828, role-map digest and real parent native
model17981/Adam3645. The plan records input411360/target29952/padding65824 per
arm; acceptance of production/native consistency nevertheless fails above.

`first_decision_tiny_process_and_binding` executed one outer test and two fresh
children, all exit0. It used8 actual optimizer/backward calls, no generation
or teacher. Within each arm, continuous2 and1+fresh-process1 had exact
weights/m/v/protocol and matching objective semantics, cursor and token
counters; first Adam clock3646 and final3647 were verified. Existing negative
checks rejected wrong parent/arm/role map and absent objective. Continuous and
split fixtures have distinct policy hashes because their study roots differ;
the independent reader verified each against its own plan before comparing
the remaining binding fields. Its earlier overstrict whole-binding assertion
failure is retained in `a-check.log`; `a-check-002.log` resolves that reader
error. The subsequent cross-arm input assertion remains a product/test defect.

Across A's8 calls, actual input13232/target824/padding1936 were consumed. Direct8
plus independent8 exhaust the combined TINY16 optimizer/backward cap. No
SMALL, generation, teacher, extra TINY or B calls were made. TINY wall time was
12.25s; observed maximum RSS3410345984bytes, peak memory footprint3364834928bytes.
These are process observations, not a whole-system/build peak. Shared-build
growth and peak remain UNKNOWN; the prior supplied whole-scope109293379bytes
predates A's new evidence/helper and is not presented as a final measurement.
Standalone objective wall time0.01s and reader attempts0.60/0.49/0.49s remain
separate from model calls. No storage budget PASS is self-granted.

## Identities and disposition

Selected root: `artifacts/first-decision-20260925-study-v6`.
Plan SHA256 `3b99f321bf846ce9d5a708c1f2d93dbc6f3092b6a3794d5a6322c01925b26f51`.
Policy `4beb6cd490dd81cfb666327081b7977fde218344dfe715870091ce62f44714f2`.
Runtime source digest `a87008aff43d937b6968695a2f1f4a76baff038531baafee751d6cfa51971fa1`.
Main executable SHA256 `1c9d1c2234a03802c8be473bb365eae055a8afb8d594991a0e964f3128d75bfd`.
Test executable `target/release/deps/replica_train-a3e8f3bc867d1194`, SHA256
`fd91fb0e056974b8bfb6c1d9545bdf7d927dcd850832b264ee5ab3d7fe1fa3c9`.

Review-root SHA256 receipts:

- `a-tiny.log`: `c462853329054910d95399d6a3fac5c0e93d131434ca5c0829b1f3b23b64f887`.
- `a-objective.log`: `473121a7d9e51497e5adbd0349de2b1705b403ad4d16404985ed08116bd3eb72`.
- `a-pair-consistency.log`: `9211a13d35aad476f869447d66058ccdb7b05fb41707af7a84cdab19a505457f`.
- Independent reader `a-check.rs`: `4f4232f12d401c385dd0dbe984f838cee427dfd499902cb0b9b67723fe513c0e`.
- Reader executable `a-check`: `198a985557cbac60b46b71965968da26e2399d2cfa5465fa94904f67cb06e2d6`.

CODE FAIL; EXECUTION acceptance FAIL despite passing within-arm tests;
RAW_INTEGRITY FAIL for cross-arm input identity; COST TINY cap exhausted,
storage/build final measurement UNKNOWN; QUALITY NOT_RUN;
FOLLOWUP_SIGNAL NOT_ASSESSED; GOAL1 NOT_ACCEPTED. B is NOT_RUN. Product repair
can proceed without model calls, but this A does not authorize learning or
reuse the mismatched-input test as corrected execution evidence.
