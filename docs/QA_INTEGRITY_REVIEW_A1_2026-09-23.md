# QA citation integrity — independent repair review A1

Date:2026-09-23. Scope:R3-QA-INTEGRITY-AND-BRIDGE-1.0, I1/I1b.
A1: PASS. Product/source/model originals and previous acceptance reports are
read-only. This is a repair and historical-metric review, not model-quality
acceptance or permission to extend a closed learning run.

## Bound source and RED evidence

Base source:`3a5f54ed91dffd8a5782e148c56a56dddf2244c8`.
Reviewed repair source:`281b8591145bb3dfc674eb52163cc5a7b459b200`.
Report HEAD at review start:`337052d0789910c9adea86a44e9ab08ce9eed391`.
The exact two-file repair was frozen before independent execution:

| Artifact | SHA-256 |
|---|---|
| src/value_citation.rs |`35def2998f6e4862d388b5cc0161d8d3c358d52679db261215694285c6977da6`|
| src/fresh.rs |`2e6c6e719687b4802b9f08a00f54b5dc83abf84f6a92f52b9492e5785fe7e854`|
| Candidate source diff |`2a8e6378f9f578687cc47fa6e909748c1f7f7abdd48a9cf910b1f17f1736137b`|
| New production executable |`a1970d4abeef9a0b8552cb2867ba5b728253e202c83939bf3bf94da6eb3026da`|
| New test executable |`bd508f3c318fa36adee0bdd2623139d02e399ab6012e777fd7ef129b2de3a85d`|

The existing RED was verified and reused, not rerun. Its recheck source/tests/Cargo
match the baseline except two preserved test-only fixture hunks. It records
normal-EOS FULL511/512, actual outside1, scored outside0, parse-failure1 and
eligibletrue, then fails the required outside assertion:1 failed,exit101,67.99s.
Its fixture diff90462697422630429f97dbcd37b2b8f28a6a5d7c7c609fc7bc386795e3c9c064,
binary983f31dd93e4c901f3796b0564f41894028c13de2cef413c5ed5a952bd878547 and
logbb2efa10b1baac09295c991b6e40a346911e992e6c49aa317396f2a21f038af0 match the
preserved independent follow-up. The old A FAIL/B metric FAIL remains an accurate
historical finding; this review closes only the verified repair scope.

The repaired scorer keeps the product whole-response parser strict. It separately
extracts each positive-i64 citation, counts outside references by row and bucket,
and requires outside0 plus parse_failure_rows0 in qa_dev_pass. Exact support is
still strict and nonempty; parse failures are distinct from valid empty citation
sets. Normal no-citation G/H answers remain subject to their actual answer text.
No partial-extraction helper enters product answer approval or generation.

## Actual GREEN and independent recount

The frozen test executable was copied to isolated A1-bin and hash checked.
It was built by the implementer with locked/offline Accelerate/test-support;
this review independently executed that newly built binary, not a new Cargo
rebuild or reused old test result. Both commands used isolated source/TMPDIR,
thread1 and exact names under
`training::fresh::identifiable::binding::citation::tests`:

| Direct test | Actual result |
|---|---|
| retained_qa_citation_grammar_boundaries |1 passed,exit0,0.01s|
| retained_qa_full_raw_gate |1 passed,exit0,204.99s|

The boundary test covers malformed references before/after a valid outside ID,
provided+malformed, malformed-only, duplicates/multiple IDs, unclosed/nested
markers, zero/negative/overflow IDs and Korean UTF-8 text. Strict successes must
have the same individually extracted set. The full typed raw writer→reader→audit→
qa_score→qa_dev_pass→qa_decision path covers positive G/H, missing/failed fit,
QA bucket/pair/EOS/outside gates and normal512 rows with one malformed output.
These are synthetic evaluator fixtures, not model generation or quality evidence.

The production READ_ONLY_HISTORICAL_REANALYSIS is a new native file:
`artifacts/qa-integrity-bridge-20260923-implementation/historical-recount.r3b`,
SHA`a3a60bedabea8aaca7300c2ded28ceb2b9daffd1467d93c2d45539e70f22f0af`.
It verifies original native/raw/summary bindings, compares all legacy fields other
than the declared correction, and does not rewrite old decisions or receipts.

Independent recount: PASS,exit0. The Rust reader uses its own byte-window scanner,
checked decimal arithmetic and separate strict parser rather than the product
partial-ID extractor. It compares per-row valid IDs/strict parse/outside/empty,
bucket totals and final aggregates to production output; counts use the actual
provided ID set, with excluded IDs checked separately. Raw tokens are decoded and
FULL/EOS independently recalculated. Only consumed raw/teacher/metadata/policy,
terminal/native and protected11264 files are hash checked; no repository-wide
inventory or old learning/confirmation test is rerun.

| Step / panel | Outside correction | Zero-based raw rows |
|---|---:|---|
|11392 balanced primary64|14→16|6,7|
|12288 old QA primary64|31→32|11|
|13312 old QA primary512|281→282|99|

All7168 generation rows and3328 QA rows were actually read. The independent
per-row comparison matched the production correction, including exactly four
undercounts and20 parse-failure rows. No index-specific scoring rule was used.
FULL/EOS and all other unchanged bucket fields matched the preserved prior raw
recount; strict parse results also matched the unchanged product strict parser.
Final15360 remains PERSISTENT_RETENTION_REGRESSION/resume=false and ineligible.
Final quality remains old QA27/512+24/128 and balanced13/512+3/128. Final QA
outside counts remain428/92 and463/114, all parse failures0; the fixed train probe
remains21/128, outside98, EOS127/length1. All A–F remain0. The repair cannot turn
these into model-quality acceptance.

## Preservation, scope and remaining work

New SMALL/TINY optimizer, generation, teacher and diagnostic backward calls:0.
No original raw/teacher/checkpoint/decision/A/B report is modified. The accepted
11264 baseline and used confirmation remain preserved; S4 stays unopened.
A1 does not authorize a new quality candidate, historical promotion, S4/S5/S6 or
Goal1. Input-factor observations and any conditional training require their own
registered stages and independent A2 preparation.

Reviewer setup note: the initial source archive named absent directory sql and
was rejected before extraction/build. It was corrected to the actual tracked
migrations include; both model and build calls were0. The failure is preserved in
A1-source-setup-01.log. No production validator or source was changed by review.
At receipt publication, the worktree already contained subsequent bridge edits.
The initial publisher rejected that current-source hash and emitted no receipt.
The finalized A1 binding verifies the isolated tested snapshot against committed
repair281b859 instead; subsequent worktree changes are outside this acceptance.

Evidence directory:`artifacts/qa-integrity-bridge-20260923-review`.
The final report commit will be separate from the reviewed source commit.

| Independent evidence | SHA-256 |
|---|---|
| Grammar log |`1217793b3c44c4074f76d9009c2c1a7a872c1661fb53477e336eddc2a96d4ffc`|
| Full raw gate log |`e6409c00b5823e29905d132ed9107aae427516028475f4bb65860857122ed36a`|
| Independent recount |`6f6bfea806803aaf6a3030daa8a55f328e72363c732fcfbbe9601b73a08a9ea6`|
| Consumed preservation manifest |`538c8d27e68dbc4f8f4c208d8b0d15d12a4467393b316ec9277b8f7dfcb4f2f9`|
| Independent Rust source |`e035d38f5f1dc719397689b4a4636e27e42d0f551267a5861f3c2c38e1157cd1`|
| Independent reader executable |`d0c0c89fe50880cf9fcde07817b2a47d794c2e93304e2fb5c8072aceeb59aa79`|
| Recount stdout |`0184f372fad213a1222474fd0309ecef1a78202d2c97da5d312a88782c3bd737`|
