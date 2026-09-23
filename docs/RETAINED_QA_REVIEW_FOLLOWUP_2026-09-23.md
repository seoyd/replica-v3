# Retained QA — independent defensive follow-up

2026-09-23 · R3-RETAINED-QA-TRANSFER-1.0.
**A acceptance: FAIL (confirmed scoring/eligibility defect). B metric validation:
FAIL (four observed undercounts).** Execution and preserved model evidence are
separate from these failures. No product patch was applied.

Reviewed candidate: `3a5f54ed91dffd8a5782e148c56a56dddf2244c8`.
Review-start report HEAD: `a010c6f418b765a4cd85acf2cf8a698f55de591b`.
Product source/tests/Cargo are identical between those commits and remain unchanged.
Production executable SHA-256:
`a833ff1919fb166d7a493e60c1f4c34f9697a7e82ed34ba80853e72222c357d7`.
This report's publication commit is distinct from the reviewed source.

The complete current shared contract and review prompt were read. The existing
11264 acceptance, confirmation and prior stabilization were preserved. Existing
retained-QA A/B reports remain immutable historical records; this report corrects
their scope of acceptance using new evidence.

## Confirmed issue — malformed citation suppresses real outside IDs

- **Severity:** Medium.
- **Location:** `src/value_citation.rs:3753–3757`, `qa_score`; downstream
  `qa_dev_pass` at3791 and `qa_decision` at3820–3825. The existing independent
  `read_B_retained.rs::qa_count` repeats the same aggregation error.
- **Evidence:** `citations(actual).ok()` discards the entire parsed ID set if any
  citation is malformed. `ids.as_ref().is_some_and(...)` then reports no outside
  ID. `src/app.rs::citations` deliberately returns an error on a malformed member.
  The QA gate relies on `outside_id == 0`, while allowing some strict answer errors.
- **Problem:** A malformed citation is not evidence that the output contains no
  other valid reference outside the provided records. The contract requires those
  references to remain counted. Strict FULL already treats this answer as wrong;
  that does not repair the independent outside-zero gate.
- **Impact:** QA reports undercount outside IDs, and an otherwise qualifying
  endpoint can be marked eligible despite an outside citation. Product response
  validation was not changed or shown to accept such an answer. The actual final
 15360 endpoint remains ineligible for other quality failures.
- **Reproduction:** A reviewer-only extension to the existing complete typed
  writer→reader→audit→scorer→decision fixture placed one valid outside reference
  plus a malformed citation in a single normal-EOS QA output, within max_tokens128.
  All production functions were unchanged. It produced `actual_outside_rows=1`,
  `outside_id=0`, `FULL=511/512`, `parse_failures=1`, `eligible=true`.
  The required outside-count assertion failed: one test failed, exit101,67.99s.
- **Recommended Fix:** In `qa_score`, count individually valid outside references
  even when a different citation fails parsing; keep parse failures and strict
  answer correctness separate. The existing `valid_outside_ids` helper at718
  demonstrates the intended independent check. Feed the corrected count into
  per-bucket totals and `qa_dev_pass`. Correct the independent QA reader too;
  preserve original raw/decisions and publish a separately identified corrected
  recount instead of overwriting execution evidence. Do not repair generated text.

The original run contains four affected rows, with zero-based raw row indices:

| Step / panel | Outside rows recorded → independently counted | Rows |
|---|---:|---|
|11392 balanced primary64|14 →16|6,7|
|12288 old QA primary64|31 →32|11|
|13312 old QA primary512|281 →282|99|

Final15360 QA counts and FULL scores are unchanged. Old QA remains27/512 and24/128;
balanced QA remains13/512 and3/128. No candidate, S4 or later-stage permission is
created by this correction. Model joint quality remains FAIL.

## Executed, reused and not run

- **Executed:** locked/offline Accelerate build in isolated source/target/TMPDIR;
  `retained_qa_tape_input_objective`:1 passed,exit0,8.32s. It checks mixed lengths,
  all4096 fixture draws, ANSWER scalar/gradient8 versus4+4, prompt/padding masks
  and EOS. Its fixture token totals are not the SMALL run's measured totals.
- **Executed:** the negative full raw gate fixture described above,1 failed,
  exit101. Its added code is confined to existing `cfg(test)` fixture/assertions;
  the candidate's production logic is unchanged. Synthetic raw is not a model
  output, an attack against a service or quality evidence.
- **Executed:** an independent Rust reader decoded all7168 saved generated rows,
  checked strict FULL/EOS flags and all3328 QA rows for individually valid outside
  IDs. Original raw/teacher hashes match the prior B receipt. All4096 trace steps,
  draw indices, LR bits and denominator8 were checked; all10 native/terminal
  identities remain bound to prior B. Final reader exit0; undercounted rows4.
- **Reused and verified:** source-bound preparation/split/native/Adam and actual
  fresh-process resume evidence. A source matches current source; its test binary
  remains `5085c33979bdb7d0fc72559d42827e7389583cfabba428257bb5eae6534a2137`.
  Training resolver/annotation stay outside the product library and normal
  generation receives request tokens and request limits, not expected answers.
- **Reused and re-read:** the completed64 fresh-process outputs. A separate reader
  matched all64 raw outputs/bytes/EOS/errors to originals, including32 QA mistakes,
  exit0. These are historical generations, not newly generated outputs.
- **Not run:** duplicate TINY process work, SMALL learning, new64 reproduction,
  old confirmation, S4/S5/S6. No eligible endpoint exists; no seal was opened.

New optimizer/generation/teacher/finite-difference calls: **0/0/0/0**.
The synthetic-logit gradient test is not a model backward or an optimizer update.
Historical SMALL4096 updates, generation7904 and teacher7168 remain unchanged.
No missing receipt was synthesized. Study file inventory and consumed protected
source/corpus/native/raw/teacher/trace hashes were unchanged. Existing A/B reports,
the original11264 acceptance and the untracked `.DS_Store` were preserved.

Two reviewer setup failures are retained and are not product findings: the first
scratch build lacked the tracked SQL include (exit101; copied unchanged from the
same candidate), and the first readback used a one-based tape index (exit101;
corrected only in the reader). Neither performed model calls.

## Necessary regression checks

- **Unit / malformed:** valid outside ID plus malformed citation in both orders;
  valid provided ID plus malformed citation; malformed-only and no-citation text.
  Preserve separate parse-failure, support, outside and strict-answer counts.
- **Integration / failure path:** the complete raw→QA scorer→candidate fixture
  above must report outside1 and eligible=false even with FULL511/512 and normal EOS.
- **Regression:** recount the four preserved original rows and correct their
  panel/bucket counts; retain the final failed endpoint, no-resume decision and
  existing raw. No new learning or whole stabilization rerun is needed.

Local evidence: `artifacts/retained-qa-20260923-review/recheck/`.
The actual commands used `cargo test --locked --offline --release --features
accelerate,test-support --bin replica-train` with the named filters,
`-- --nocapture --test-threads=1`; the negative fixture alone sets
`R3_REVIEW_MALFORMED=1`. Both use isolated CARGO_TARGET_DIR/TMPDIR and one compute
thread. `audit-reader` and `parity-reader` only read originals and write scratch.

| Evidence | SHA-256 |
|---|---|
| Fixture-only diff |`90462697422630429f97dbcd37b2b8f28a6a5d7c7c609fc7bc386795e3c9c064`|
| Reviewer test executable |`983f31dd93e4c901f3796b0564f41894028c13de2cef413c5ed5a952bd878547`|
| Negative fixture log |`bb2efa10b1baac09295c991b6e40a346911e992e6c49aa317396f2a21f038af0`|
| Independent raw reader source |`9095a3be40bc436721dfb7c95b97cbf531e13294524d72cdc0d996cdff3a7bd2`|
| Independent raw reader executable |`f507e88f43246b592c253dc23a25ec81466352524e0c2e38dd542c68f13c1350`|
| Independent raw audit |`97ddbbcb5accb5a7a2382eba171526c0b5e56ab7beb470d8caf7424acc96df81`|
| Consumed protected manifest |`12f7cb0e09e53e5d56b5bc9844d45f9612ceaf0e53d026dc60574fc02cc02aba`|

Only this report is published. The review closes with one confirmed root cause;
execution preservation is distinct from failed metric/gate correctness and failed
model quality. S4/S5/S6 and Goal1 remain unaccepted.
