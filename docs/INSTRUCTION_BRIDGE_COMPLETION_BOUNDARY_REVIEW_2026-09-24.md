# Independent boundary addendum — instruction bridge completion

**Changed-boundary acceptance: FAIL — one Medium code defect.** The earlier
source-bound A process tests remain valid evidence for their exercised cases;
this additional review identifies an uncovered finalization boundary. Existing
A/B/QA reports and their bound receipts have not been rewritten.

Reviewed source: `c502e66872ad6abdf463990c5992061d3ec3a19c`.
Source digest: `dab51655d7ad6a5bd4c9f9ffcd0906efa80f9f4971dc93719bd234909021e434`.
HEAD before this report: `a14fde15dfb3e26803fe8a61ada469c18bf65e8e`;
source/tests/Cargo are unchanged from the reviewed candidate. This report commit
is separate from that source.

## Finding: completed diagnostic QA cannot finalize at the active-time cap

- **Severity:** Medium.
- **Location:** `src/value_citation.rs`, `bridge_diagnostic_qa`, lines4294–4305;
  `src/binding.rs`, `observation_control`, lines1040–1045.
- **Evidence:** The caller calculates `remaining` from returned raw rows, then
  unconditionally calls `observation_control(&p, remaining, 0)` before entering
  `segmented_collect`, validating the completed observation and publishing its
  score. `observation_control` rejects `elapsed >= active_seconds`, including
  `remaining == 0`. Therefore a complete returned prefix cannot reach no-call
  finalization when the cumulative active-time limit has been reached.
- **Problem:** The bounded continuation contract permits finalization at cap
  equality when no additional model call is required. The new diagnostic path
  applies the admission test for new model work to this finalization operation.
- **Impact:** A permitted QA diagnostic can remain without its aggregate final
  record/score after the last row was durably returned and the command then
  encountered pure TIME_BUDGET. Reentry cannot finish it within the registered
  policy. This blocks result completion; it does not silently promote a model.
  The actual14336 execution finished with active2532.134917416/7200s, so this
  finding does not invalidate its completed B or QA640 results.
- **Reproduction:** A reviewer-only fixture copied the already accepted four-row
  TINY QA observation, preserved every raw row and RETURNED resolution, omitted
  the aggregate final/score, and set only the copied last segment's elapsed and
  stop fields to represent a pure deadline after its fourth durable return.
  Production `segmented_usage` validated its prefix/call records; production
  `usage` returned `(7200, 4, 0)`. Calling the same production budget function
  used at4296 with `remaining=0` returned
  `invalid input: fresh: binding observation budget exhausted`.
  The contract assertion failed: **1 executed, 0 passed, 1 failed, exit101**.
  This is a dynamic budget-boundary unit reproduction plus static caller tracing,
  not a claim that the entire diagnostic CLI was rerun at the time cap.
- **Recommended Fix:** In `bridge_diagnostic_qa` and the segmented finalization
  path, separate validating/finalizing a complete RETURNED observation from
  admission of new model calls. Before taking that path, validate the complete
  count, model/case binding, raw/prefix hashes, per-call resolutions and segment
  termination. Permit only the resulting no-call finalization at the exhausted
  time cap. Keep nonzero remaining calls, UNKNOWN, cancellation, corruption and
  missing evidence blocked; do not extend the budget or candidate/confirmation
  authority. Merely removing the time comparison would also leave an exhausted
  RunControl at finalization and is insufficient. No product patch was applied.

## Execution and reused evidence

The candidate was archived to
`artifacts/instruction-bridge-completion-20260924-review/cap-boundary/source`.
Only a `#[cfg(test)]` module inclusion and its new fixture file were added to the
scratch copy. Production function bodies were unchanged; product source/tests/
Cargo and original artifacts were not edited.

The command was run from that copy with isolated target/TMPDIR, compute threads1:

```text
cargo test --locked --offline --release --features accelerate,test-support --bin replica-train returned_qa_at_active_cap_can_finalize_without_new_calls -- --nocapture --test-threads=1
```

Compilation took37.77s; the one executed test took0.02s and produced the expected
RED result above. New optimizer/generation/teacher/backward calls: **0/0/0/0**.
No native model was loaded for this fixture. Its source and raw-copy hashes
distinguish the added test from the unchanged candidate production code.

The existing independent A gate/process logs, A preparation, B raw/trace/native
recount,44-call reproduction and QA640 addendum were reused after verifying
their recorded hashes and candidate/binary/preparation/report bindings. They
were not rerun or counted as new dynamic tests. The diagnostic reader compiled
and exited0; it checked1422 unique files from the overlapping A/B/QA consumed
manifests in one pass, with no model calls. Existing complete results remain:

| Scope | Verdict |
|---|---|
| Current changed-boundary code acceptance | FAIL: the Medium finding above |
| Existing B execution/metrics/reproduction | PASS, reused bound evidence |
| Existing QA640 result integrity | PASS, reused bound evidence |
| Training |1536 updates complete, final14336, closed; no repeat authorized |
| Bridge development | FAIL: S1Q0/S0Q1/S1Q1 FULL511/511/508, outside rows1/1/1 |
| Bridge fit | FAIL: FULL1535/1536, one preserved RETURNED timeout/parse failure |
| General QA | FAIL: FULL0/640, EOS300, length340 |
| Protected11264 / parent12800 | Acceptance / historical quality failure preserved |
| New confirmation / S4 / S5 / S6 / Goal1 | No new confirmation; S4 unopened; no acceptance |

Existing usage stays SMALL1536 optimizer/8780 generation/8064 teacher,
TINY26/586/360. Original evidence and `.DS_Store` were preserved. Only this
additional review report is published; historical receipt-bound reports remain
unchanged.

## Necessary regression coverage after repair

- **Unit / boundary:** zero remaining calls at the active-time cap can finalize;
  one remaining call is still denied. Cover a deadline observed immediately after
  the last durable RETURNED row.
- **Integration / regression:** run the diagnostic caller in a fresh process
  from that complete TIME_BUDGET state; final/score publication succeeds with
  optimizer0/generation0/teacher0, unchanged raw, and idempotent completed reentry.
- **Malformed / failure path:** the new no-call branch must reject missing or
  extra rows, mismatched hashes, UNKNOWN and cancellation before publication.
  Exercise the new branch specifically; broad historical stabilization is not
  requested.

Local evidence root:
`artifacts/instruction-bridge-completion-20260924-review/cap-boundary/`.

| Evidence | SHA256 |
|---|---|
| Frozen production binary | `ffffb7f2f2dab68f04a73d579f65fe8d55b9177c00244705d7bd12788c472713` |
| Candidate plus test-only module binary | `021e9268b186199e4dc5da4581559f6d0036da5b47acfd53b797c6e466108375` |
| `source/src/review_budget_boundary.rs` | `0dbff93de3758bb26b4c74a8458bbe69a5884fda5209da4bfb54dd708bb14109` |
| `boundary-test.log` | `1c087d8450ca73a81ef92c70d2eeda500f068d3eab55fe142fc5d550925ef204` |
| `verify_reused.rs` | `c2303cef12432c53353cfe5da5eb45f8428383bb817a5b98a8b7e6b7dd87240a` |
| `verify_reused` executable | `3d4321abfa0d872737beff58b8d0fd23c06b2d87290e93049b38ee7fd0599e05` |
| `reused-evidence.log` | `65f83ee8597fed52b484abfab99cd822cb76eec30c71dd0d96461bbce9eefc7c` |
