# Metal F32 fresh-optimizer comparison — independent B

**B_INTEGRITY_REPRODUCTION: BLOCKED_RUNTIME.** The preserved study stopped at
A512/M512 with `success=false`, `resume=false`. An independent read-only Rust
recount verifies the available state, trace and raw observations; it does not
complete the missing evaluation or authorize another call. The accepted numerical
A remains separate from this incomplete B. No quality or Goal1 acceptance follows.

## Identity and scope

The actual learning source is `6d71d0418222039c7e23440cffaf78c09a26ce7f`.
Its `src/muon.rs` SHA256 is
`1b559900bfd8ada7f010b1a77ddbcfc457f6b835017047ba77045b10b46a93fc`;
the preserved executable at
`artifacts/muon-quality-20260924-evidence/replica-train` hashes to
`1efbb40511e7f19f2b448e582d17f5b9b77d9b852f787cd28f2a686a4adccfc7`.
The subsequent output-classification correction is reviewed separately below.
It was not used to produce these model checkpoints or raw rows.

Original study: `artifacts/muon-quality-20260924-study/`.
Independent evidence: `artifacts/muon-quality-20260924-review-b/`.
Preparation, parent, corpus, tokenizer, tape, runtime, roles and numerical bounds
retain the identities recorded in `docs/MUON_QUALITY_REVIEW_A_2026-09-24.md`.
In particular, plan file SHA256 is
`76b8c24f50406b4a6861c42bb04d8decda2519f87c57a4003ad71e5abc223c5b`
and policy digest is
`6133bfd80aebf586b409fc67e6f4f78a8444df98d50c6e10685c83250d871399`.

Product source, original data and study records were read-only for this reviewer.
Independent B issued **0 SMALL/TINY updates, 0 generation, 0 teacher calls**.
Loading native files on CPU to inspect bytes/state is not an inference fallback.
No old Metal suite, protected generation, LegacyB or FP4 test was repeated.

## Confirmed output-error classification defect

Severity: **Medium**, original learning source `src/muon.rs::generated` and
`decision`. The collector treated every non-null output `error` as an execution
failure. This includes an otherwise normally returned native generation whose
bytes fail strict UTF-8 decoding. Separately, the guard used `total - EOS` instead
of the scorer's error/non-EOS union, which would omit a decode error after EOS.
These are two effects of the same output-versus-execution classification mismatch.

The actual trigger is M512 `renamed`, ordinal9,
`qa-word-value-v1/renamed/2/id0/1`:

- `generation_started=true`, `generation_completed=true`, finish `stop`, EOS
  present, 17 raw tokens; `generation_error=null`, `command_stop=null`.
- `actual=null`; `error` and `decode_error` both contain
  `model: native tokenizer: invalid/incomplete output UTF-8`.
- Raw row digest:
  `12aaf54b365d9b2a5f0b90919fd171e53c334c35ea4eed98519405c762f2e783`.
- Tokens: `384,54,40,99,287,66,64,465,54,287,61,65,57,57,58,101,2`.

The independent reader decoded these original tokens and confirmed the failure.
The original collector raised `Error::Invalid`, subsequently classified as
`INTEGRITY_FAIL`. The impact is premature execution closure, not an inflated
quality score. There is no evidence here of Metal NaN, kernel failure, corrupted
state or an incorrect Muon equation. Invalid model output remains a real quality
failure; it must not be repaired, lossily decoded or counted exact.

The recommended narrow repair is to validate raw/Generated identity on both new
and reused RETURNED rows, admit only proven output-decode failure to the quality
scorer, and use its union of error or non-EOS rows for the guard. Native execution,
nonfinite data, cancellation, UNKNOWN and storage errors must still fail closed.
The preserved terminal is not retroactively changed by this repair.

During review of the repair, a second, **delta-only regression** was found before
publication: unconditional rejection of `generation_error` on reused rows would
block the already-supported command-capped native timeout. A resolved returned
timeout is different from an unknown call. The narrow repair must preserve the
exact proven TIME_BUDGET case without admitting request timeout or mixed errors.
Its final verification is recorded separately below.

## Independently verified execution and state

`failed_recount.rs` reads the original native corpus and metadata, retokenizes each
consumed tape batch, checks input/target/padding and row order, then verifies every
trace's local/model clock, actual LR and sample exposures. It loads both endpoints
with training state, verifies optimizer family/roles/keys/shapes and finite weights
and moments, and checks native physical/content identities.

Every available panel row is bound to its frozen request, answer, prompt digest,
provided evidence, model, policy and tokenizer. Immutable prepared/resolved
RETURNED pairs and row hashes are checked. The reader independently computes
strict text/EOS/full, full-string value, exact support, individually valid outside
IDs, parse failures and QB/SB/ALL4, comparing all complete saved scores. Incomplete
rows are reported at their actual numerator/denominator. Earlier raw results do
not claim to be fresh generation on the endpoint.

| Segment | A local | M local | Generation | Result |
|---|---:|---:|---:|---|
| 000 | 1 | 1 | 0 | Explicit +1 save, resumable |
| 001 | 512 | 436 | 1,600 | Pure TIME_BUDGET; resumable |
| 002 | 512 | 512 | 266 | Preserved INTEGRITY_FAIL; not resumable |

The separate parent parity observation accounts for another16 generations.
At A128, S1Q1 FULL51/64 and ALL4 10/16 started the weak-retention streak. At A512,
54/64 and10/16 caused `PERSISTENT_RETENTION`; A did not continue. M proceeded only
to the same local512 endpoint. Its512 panel failure prevented publication of its
complete decision/fit; the recorded evaluation cursor remains128. M512's complete
S1Q1 panel already has a severe retention signal, but this report does not create
or apply a historical decision that was never published.

Each arm committed512 updates,4,096 exposures,3,584 unique train rows,
753,536 input tokens,59,904 targets and71,296 padding tokens. Both use exactly the
same prefix, batch8 and actual LR3e-5. Native absolute step is14848; optimizer local
step and tape cursor are512, not14848. The total is1,024 committed updates and
8,192 exposures, not1,024 updates applied to one model.

M's local437 in segment001 additionally performed one forward/backward with
1,400 input and102 target tokens before the time boundary, **optimizer0**. Its
immutable `step-437-segment-001-discarded.r3b` is retained; the next segment
committed that cursor once. This work is separate from committed token totals.

| Durable native | Bytes | Optimizer tensors | Physical SHA256 |
|---|---:|---:|---|
| `A/segment-001-step-512.r3m` |114,182,528|136|`a549392f3f12ebdea95616d6d66043f9b88e1e56ce40ab74d6159304069a8da0`|
| `M/segment-002-step-512.r3m` |77,020,608|94|`bca802ec29fe4c92a977cc501a928e1f91fc4bf3e4caf60969a1817bf5a9974c`|

Weight content IDs are A
`0b0e02f7a5872cc1a383d3a5ad2d9e8f4218ca86991d5b7be7e9fc7af8b6bec5`
and M `ac0f30f8f18dc34c473e0547d92cbd60f7b24b7daddcd5e2c943ba05906de74d`.
The preserved final segment SHA256 is
`7eda204d83b5f32f77635ac6b0929a73988b1747ba1315fc08dbe374b6a9c7af`.

## Same-step observations, not completed quality acceptance

All complete rows below have denominator64. QB and SB each have denominator32;
ALL4 has denominator16. Value is the entire generated value string, not its first
token. Outside-ID and parse counts overlap and must not be added as disjoint errors.

| Arm512 / panel | FULL | QB / SB / ALL4 | Value / support | Outside / parse | EOS / errors |
|---|---:|---|---|---|---|
| A V |64|32 /32 /16|—|—|64 /0|
| A VC |63|31 /31 /15|63 /64|0 /0|64 /0|
| A S1Q1 |54|22 /26 /10|64 /54|9 /1|64 /0|
| A word |0|0 /0 /0|14 /0|64 /0|64 /0|
| A renamed |0|0 /0 /0|15 /0|64 /0|64 /0|
| M V |64|32 /32 /16|—|—|64 /0|
| M VC |64|32 /32 /16|64 /64|0 /0|64 /0|
| M S1Q1 |40|17 /19 /8|64 /40|13 /11|57 /7|
| M word |0|0 /0 /0|0 /0|20 /50|52 /12|

M renamed has only **10/64 RETURNED**, not a completed score:0 exact,0 value,
0 support,0 outside,10 parse,4 EOS and7 error/non-EOS rows,264 output tokens.
Its whole-panel QB/SB/ALL4 and paired comparison are INCOMPLETE, not0% acceptance.

At32 and128, both word and renamed FULL/ALL4 were also0. The paired ALL4 counts
for every completed common word/renamed panel are
both0/M gain0/M loss0/neither16. Word and renamed share the same semantic bases;
they are not independent samples and are not pooled for a larger denominator.
M128 retained S1Q1 64/64 while A had51/64; by512 the corresponding complete
observations were40/64 and54/64. That transient retention difference is not a
demonstrated new-task learning benefit or a general optimizer ranking.

Train-fit128, its teacher/NLL diagnostics, the1024 full panels, the prescribed
fresh-process B normal32 plus failure sample, general QA, confirmation and
S4/S5/S6 are **NOT_RUN/BLOCKED_RUNTIME**. The independent reader is not a
replacement for fresh-process output reproduction. It does not certify B PASS.

## Update size, time and owned bytes

Both first updates saw the same ANSWER loss5.4856205 and unclipped gradient norm
11.872930745856488, with clip factor0.0842252028084055. Their actual deltas differ.
The trace records synchronized gradient/update computation, not an equal-FLOPs
comparison or an equal-update-norm experiment.

| Local step | A ANSWER CE | M ANSWER CE | A delta L2 | M delta L2 |
|---|---:|---:|---:|---:|
|1|5.4856205|5.4856205|0.0905808754|0.0086761903|
|32|3.433977|4.9374285|0.0543529528|0.0076416444|
|128|1.0789397|3.6715553|0.0665643384|0.0084599856|
|512|0.6339637|1.5967574|0.0258762621|0.0208809383|

`trace-footprint.log` independently aggregates the saved per-parameter
gradient/momentum/delta/weight norms and delta/weight ranges at1/32/128/512.
At step1, M's hidden42 delta L2 is0.00575365295 and aux26 is0.00649397840;
at512 these are0.00554923407 and0.02013006674. Hidden momentum L2 is
0.7660591886 at1 and8.9817140928 at512. These observed sum-momentum and aux Adam
quantities are not interchangeable algorithms or proof of a learning-rate cause.

| Synchronized median seconds, n=512 | A | M |
|---|---:|---:|
| Forward/backward |0.548857375|0.549039896|
| Optimizer |0.1154697085|0.1706262915|
| Whole step |0.668259333|0.722869146|

These are actual Metal F32 release measurements from this study, with the retained
Candle patch and one compute thread. Optimizer timing includes its registered
finite checks, scalar readback and NS5 work; there is no isolated NS matmul timing.
NS temporary GPU peak and peak process RSS are **UNKNOWN** in these records.
`rss_observation_enabled=true` is not a peak measurement. The implementer reports
one live `ps` sample, RSS864,432KiB for the frozen trainer PID11765 at elapsed14:29;
this is IMPLEMENTER_REPORT, not this reviewer's independent sample or a peak.
An old debug/CPU speed
ratio is not reused. Initialization, native saves and evaluation are not silently
included in or equated to the per-step medians.

Study accounting:1,882 generations,0 teachers,26,982 generated raw tokens,
1,043.33592275 active seconds, including numerical A admission39.01s and parent
parity2.170716833s. The three training segments consumed1,002.155205917s.
Arithmetic remaining budgets are1,024 updates,4,518 generations and6,400 teachers;
the sticky terminal forbids consuming them. They are not a resume authorization.

A read-only Rust count scoped only to this new study found4,879 regular files,
468,258,526 logical bytes:5 native files459,426,880B;36 raw/trace files5,538,402B;
4,838 metadata files3,293,244B. This is logical size, not allocated disk blocks or
an inventory of original artifacts. No deletion/move/compression was performed.

## Independent execution evidence and correction delta

Read-only `failed-recount` ran once, exit0,5.53s wall, verifying29 complete panels,
1,866 panel RETURNED rows and the16-row baseline accounting. The retained first
compile attempt failed on a scratch-only Result conversion and executed no model;
the corrected reader compiled against the existing locked release libraries.
`trace-footprint` also completed exit0 with no model calls. Compilation and read-only
audit time are separate from model active time.

| Evidence under `artifacts/muon-quality-20260924-review-b/` | SHA256 |
|---|---|
| `failed_recount.rs` |`d2a616a4fa2dff0db255b2ed93550bd45db1d0f95b9d5b1e1ce7ae07b414c490`|
| `failed-recount` |`11cb0cd2b04af0725fd160ec461117f25b519d165745a9dd374df51d7ddaaf6a`|
| `failed-recount.log` |`ee82b8ea2c425350a80a78caefaa16240e35049ce5a03212a81a042fdb7df552`|
| `trace_footprint.rs` |`157c4f34a3bd5d596e4d21485df23ad5ec8ce59dd50301e1d6a97d3323db1e08`|
| `trace-footprint` |`e235ca560d50dcc5f64f3d022c682191fd24102cd5e24f8ca2d48b7adb0bfc5b`|
| `trace-footprint.log` |`f78c2888e6eede0569e9130f3135a80195509a3728ecc6399f73d26d2e705cbf`|

The first narrow correction revision (`src/muon.rs` SHA
`bbc74a06e1c612936355789921ba8008bccd90eac43c2fcf0206a3b672cb9698`)
was independently exercised by the exact ignored test
`training::fresh::muon::tests::returned_decode_quality_guard`,1 passed/0 failed,
exit0,2.75s wall. It explicitly uses synthetic returned rows, not model generation:
strict invalid UTF-8 with EOS plus3 length rows produces union4/severe, validates
strict bytes, rejects runtime/cancel fields and preserves immutable reused raw.
This first test PASS was not used as the final delta verdict because the
timeout-resume regression described above was found afterward.

**FINAL_DELTA: PASS** for the narrow reviewed boundary. Final `src/muon.rs` SHA256:
`ba40871565d9a54a2a2c5782f857e54fc888dd1eb5390ec56abe55de70a8f62a`.
Final exact-test binary SHA256:
`e0c6fb7268e6d33cb73b6c55d1e1033ebed5405cd76b859096ee8dd879764633`.
It is preserved at
`artifacts/muon-quality-20260924-evidence/replica-train-tests-timeout-final`.
The reviewer executed that same named test once on the final binary:
1 passed/0 failed/190 filtered, exit0,2.73s wall, all model calls0.
`returned_decode_quality_guard-timeout-preserved.log` retains the result,
SHA256 `6e74fac9f0d222de23ca7a459908277cfa5798184ee513c5ee41e774c0a325b0`.
Invocation used `--exact --ignored --nocapture --test-threads=1`,
`R3_MUON_PREPARATION` bound to this study, and
`VECLIB_MAXIMUM_THREADS=1 RAYON_NUM_THREADS=1`.

The final helper accepts only a verified completed strict-UTF8 output error or
the explicit proven command-capped returned timeout; the latter requires the
exact native timeout/error fields, unfinished/null generated result, TIME_BUDGET
stop/interruption, positive effective cap below the request cap, and matching raw
count. The collector still verifies the immutable call resolution before reuse.
The new synthetic0/1-token timeout fixtures reuse the original raw at zero call
budget and verify no file change; request-cap/mixed-stop fields remain rejected.
Both fresh and cached collector paths call the same helper. Cancellation/UNKNOWN
and nonfinite optimizer rejection retain their previously accepted boundaries;
no old model-generation suite was repeated to claim new evidence.

The first and final independent delta invocations consumed2.75+2.73=5.48s of
no-call fixture wall time. The implementer reports19.55s for its preceding
RED/GREEN/native/finalization checks plus2.63s for the final timeout fixture,
22.18s separately. Neither quantity is added to the historical study receipt or
reported as new training/generation. These direct changes do not reopen the study.

Existing five-file protected-artifact hashes were rechecked successfully after the
read-only recount, including protected11264, original14336, failed adapter14464
and its raw/terminal. The review did not modify any original record. Independent
review execution did not call an external model, modify product source, update an
optimizer or generate another answer.

## Separate verdicts

- **CODE_AND_NUMERIC / A_ACCEPTED:** original numerical/state acceptance retained;
  original collector classification defect confirmed; final narrow delta PASS.
- **RESET_FAIRNESS / ACTUAL_STATE_COUNTS:** VERIFIED for the consumed same512 prefix.
- **B_INTEGRITY_REPRODUCTION:** BLOCKED_RUNTIME; available raw integrity VERIFIED,
  missing fit and fresh reproduction remain missing.
- **SAME_STEP_EFFECT:** reported only for completed common512 panels; incomplete
  renamed rows are not silently made comparable.
- **RETENTION / WORD_LEARNING:** A weak-retention stop; M512 complete S1Q1 severe
  signal, word FULL/ALL4 zero. No successful new-word selection demonstrated.
- **TIME/COST:** observed scope above; peak RSS/NS temporary peak UNKNOWN.
- **RESEARCH_RECOMMENDATION:** no Muon quality adoption or automatic extension.
  One nominal LR, seed, fresh-state fine-tuning comparison does not establish that
  Muon is generally better or worse; no hidden LR search was performed.
- **GENERAL_QA / S4 / S5 / S6 / GOAL1:** NOT_RUN/not accepted by this study.

Reviewed learning source, correction source and subsequent report publication SHA
are distinct identities. Publication and actual remote SHA are recorded by the
implementer after this report closes; this report does not invent a future SHA.
