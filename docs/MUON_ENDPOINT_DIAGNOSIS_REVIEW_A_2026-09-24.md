# Muon endpoint diagnosis — independent A-delta

**CODE_DELTA_REVIEW: PASS** for `R3-MUON-ENDPOINT-DIAGNOSIS-1.0`.
The new evaluation-only registration may proceed within its own fixed budgets.
This acceptance does not reopen the original failed study, accept its incomplete
B, or assert new model quality. The independent reviewer made no product change.

## Reviewed identities

- Reviewed source: `4850195a1b5cb1d51f7f49fccae14ed194243f56`.
- New `src/muon_diagnosis.rs` SHA256:
  `9d95d076a01e2dfb1865d709c9afc51325b4883972c480d489fa792f8d1a1967`.
- Execution binary, `artifacts/muon-endpoint-20260924-diagnosis/replica-train`:
  `bb149a438d8aefad8a119048c026868502073ea14a2c9acf38a4ef0a1c04bb68`.
- Exact test binary:
  `b2dead1c0c6ebc920f0d6aaaf0e1a4495ec88900ed411ef9279111175e5eba87`.
- Registered plan, `artifacts/muon-endpoint-20260924-diagnosis/run/diagnostic-plan.r3b`:
  `49f9aaf5633fcdb7249985105b71c4672330c0208e60739fdb4eaf7ab2d5b920`.
- Policy digest:
  `56b0583a7d9acb1b2d1ffe54142490b94c9388409d77715aa23ff2c46a5a054d`.

The earlier prepare-probe is not the registered execution plan. The old learning
source remains `6d71d0418222039c7e23440cffaf78c09a26ce7f`, and old endpoints/raw
remain in `artifacts/muon-quality-20260924-study/`. Existing Metal/numerical/native
A evidence is reused; none of its optimizer or backend trials was rerun.

## Production-path review

The `fresh muon diagnose` entry point uses existing native inference loading,
normal generation, teacher observation, immutable call records, scorer and
RunControl. It has no optimizer/backward action and does not weaken the original
study's `control()` or `resume=false` barrier. Each new lane binds model, cases,
new source/runtime and a separate budget. Inference loads do not import Adam
tensors for an update. The old optimizer-state audit is retained as provenance.

Preparation checks the exact original failure shape and its normally returned
strict UTF-8 error, physical endpoint identities, model/local clocks, parent,
corpus, tape, raw/call resolutions and original panel scores. An arbitrary
INTEGRITY_FAIL, a different step, cancellation, mixed failure, pending or UNKNOWN
call is not equivalent authorization. The original partial panel's next call is
also checked rather than assuming an absent row proves a call was never entered.

Original raw is referenced, not copied or appended. Existing returned wrong rows,
including the UTF-8 row, are retained. New generation targets exactly the remaining
54 rows and the two train128 panels. Generation and teacher collection are
separate, avoiding the old train evaluator's automatic teacher side effect.
Composite records explicitly carry historical/new provenance.

Teacher forcing reuses the existing causal forward and target observations.
The reviewed shift predicts `gold[j]` at `prompt.len()-1+j`; only the preceding
gold prefix is visible. Role attribution uses the bytes of the actual encoded
answer, with crossing tokens marked MIXED and EOS separate. Token divergence
does not override strict text correctness. Free/teacher argmax comparison is
limited to the first difference, where the earlier token histories still agree.

Narrow source-review findings were addressed before the frozen candidate:

- Seen/unseen support now uses the existing individually-valid-ID extractor,
  keeping malformed syntax independent of valid support/outside IDs.
- Failure replay includes an available query mate, with strict UTF-8 failures
  first in fixed ordering and deduplication against the normal32.
- Final close requires all11 successful lane finals and their bound raw. An
  intermediate pure report cannot prematurely freeze a composite that subsequent
  authorized replay would invalidate.
- Error subtypes and selected teacher word/length/exposure distributions are
  explicit; overlapping error counts are not added as disjoint categories.

There is no unresolved confirmed defect in the reviewed new boundary. This is
not a claim that later model output or fresh-process parity must pass.

## Independently executed direct tests

Each test below ran exactly once on the stated final test binary with `--exact
--nocapture --test-threads=1`; artifact-dependent tests additionally used
`--ignored`. Each top-level invocation executed1 test, passed1, failed0, exit0.
The child is included in the parent process test, not counted as another named
top-level test. All use the existing locked/offline release build and one compute
thread (`VECLIB_MAXIMUM_THREADS`, `RAYON_NUM_THREADS`, `OMP_NUM_THREADS` set to1).

All suffixes are under `training::fresh::muon::diagnosis::tests::`:

| Test | Direct evidence | Wall seconds |
|---|---|---:|
| `original_decode_failure_is_not_general_failure_permission` | Exact old decode boundary allowed; old study still closed; different step/mixed failure rejected |1.44|
| `changed_refs_reject_before_model_call` | Parent/corpus/A/M hash changes and pending file rejected |8.01|
| `teacher_roles_scalar_and_mixed_alignment` | Synthetic scalar-logit NLL/role counts, byte crossing/MIXED, EOS and malformed target rejection |0.11|
| `teacher_shift_matches_full_answer_and_causal_positions` | Actual tokenizer/sample input and target-prefix positions; no forward |1.73|
| `failure_selection_keeps_correct_mates_and_invalid_utf8` | Error-first selection, correct mate retention and normal overlap |0.01|
| `not_invoked_returned_unknown_and_pure_reader` | Proven no-call time return versus UNKNOWN; cancel block; pure reader does not create missing records |4.36|
| `completed_observation_process_at_exhausted_budget` | Actual observation caller in a new process at exhausted1800s; synthetic RETURNED completion, final reuse and no extra calls |9.55|

Total independent direct-test wall time:25.21s. The process fixture is explicitly
synthetic and is not a native generation result. Its preserved path is
`/var/folders/dc/80zzf7_s6158zsmc7tk9yjzm0000gn/T/endpoint-process-52132-1790249838421950000`.
The numerical fixture is scalar-only; no NS, optimizer, backward or TINY model
trial was run. There were no zero-test PASS claims.

## Independent preparation readback

The reviewer-authored Rust `prepare_reader.rs` reads the registered native plan,
corpus, tokenizer and metadata independently. It rehashes all1,215 scoped original
references, recomputes case/metadata/request/RETURNED identities and strict decode,
and recomputes the manifest's selected train/teacher cases and consumed tape
exposures. It ran once, exit0,1.33s read-only wall time. Compilation and file audit
time are separate from model active time.

Observed:

- Original A320 + M266 =586 RETURNED; exactly1 strict UTF-8 error, the preserved
  M renamed ordinal9. New missing set is precisely54, not64 or a retry of the error.
- Original terminal remains `success=false`, `resume=false`, INTEGRITY_FAIL.
  The registered endpoints equal its A/M references; model step14848/local512.
- Each selected train128 row was actually seen:42 rows once and86 rows twice in
  the first512 tape updates. This is a selected128-row diagnostic, not full train
  fit, and does not assume all corpus entries were exposed.
- Teacher64/model = train32 + dev32. Each split has16 `오른쪽` and16 `왼쪽`
  examples;3/2 characters,9/6 value bytes,16 answer targets including EOS.
  The fixed teacher manifest does **not** represent every word type in the corpus.
  This limitation must remain explicit in subsequent causal interpretation.
- Independent usage for this A-delta: **optimizer0, backward0, generation0,
  teacher0**. The plan preserves438 generation,216 teacher-example,1800s active,
  900s segment and256MiB immutable-evidence caps for the separate diagnosis.

Evidence is under `artifacts/muon-endpoint-20260924-review-a/`:

| Evidence | SHA256 |
|---|---|
| `prepare_reader.rs` |`18e97f62c33c29e01ffbf7a09ee25e9b1d3dfa62b4d700ae5102ac2ec2afbe0f`|
| `prepare-reader` |`a290115a9daccb799cb472819c07c03306e189588f4b3c81e3f7fbd4ea28b218`|
| `prepare-reader.log` |`6430a3937c8dc556e034b979c211d10f7f6e568ca4c94781cb25fbf6b2dd99ac`|
| `completed_observation_process_at_exhausted_budget.log` |`f2c7e67db3d5863abecc62ed6b949786919dcae19fe316ca59846c29059017b5`|

The other six logs use their exact test suffix as filename in the same directory.
The original failed study, models, Adam, raw and user changes were not edited.
No deletion, model export, archive/DB redesign, dependency update or external API
was used. Only new reviewer source, executable, fixture, log and report artifacts
were created; no full source/model/artifact copy was made.

## Acceptance boundary

| Layer | Verdict |
|---|---|
| CODE_DELTA_REVIEW / new A |PASS|
| ORIGINAL_STUDY_STATE |FAILED_UNCHANGED|
| ORIGINAL_B_FULL |BLOCKED_AS_RECORDED|
| POSTHOC_AUTHORIZATION |Bound evaluation-only plan accepted|
| POSTHOC_EVIDENCE_COMPLETE / new B |NOT_RUN at A publication|
| GENERATIVE_PARITY / teacher forward results |NOT_RUN at A publication|
| MODEL_QUALITY / MUON_ADOPTION / GOAL1_READY |No new acceptance / not adopted / false|

This accepts the registered D2–D4 observations under their fixed limits. It does
not permit new training, changes to the original terminal, additional QA640,
confirmation/S4, FP4, automatic retries after parity failure or budget expansion.
Source4850195 and the later report/publication commit are separate identities.
The implementer records the normal push and actual full remote SHA after staging
the named related files; this reviewer does not claim a future report SHA.
