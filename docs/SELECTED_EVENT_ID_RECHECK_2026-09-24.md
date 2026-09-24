# Selected-event ID protocol independent A/B recheck

2026-09-24 · `R3-SELECTED-EVENT-ID-PROTOCOL-1.0`

**A: BLOCKED_INPUT_LENGTH / NOT_ACCEPTED. B: NOT_RUN.** The unchanged
prescribed ID_ONLY requests require 259 tokens including the target and EOS,
against the registered limit of 256. The preparation correctly rejects this
input before publishing a study. No new code defect was established in the
reviewed preparation boundary; this is not an overall code or execution PASS.

The complete common contract and reviewer instructions were read. The
repository's replica-lean-development workflow was applied. Matching completed
evidence was reused, rather than executing the same input audit or previously
accepted model tests again. Product sources, tests, Cargo files, originals and
earlier reports were not edited. No new study or substitute admission receipt
was manufactured.

## Confirmed blocker

- **Severity:** Medium — registered input-contract incompatibility.
- **Location:** Common contract sections 4.1 and 4.4;
  `src/muon.rs`, `event_prepare`, lines 107–118. Study publication begins only
  at line 144, after the bounds checks.
- **Evidence:** With the prescribed neutral system, ID task, original tokenizer
  and both evidence records, ID_ONLY is prompt250 + target8 + EOS1 =259.
  The existing independent Rust reader checked all 1,920 transformed word
  cases; every one exceeds256. FULL is234 for all 1,920 cases. The reader
  derives the selected ID from request entity/context and the unique current
  record, cross-checking the original FULL label without calling the candidate
  transformation. Original evidence is unchanged; excluded records=0.
- **Impact:** Neither F/I preparation nor independent A can be admitted under
  the current contract. No baseline, training endpoint or B raw exists for this
  study. This does not alter any previous accepted boundary or prior failure.
- **Reproduction:** The first rejected case is source train index6144,
  `qa-word-value-v1/train/0/id0/0`, at mode1. The preserved preparation error and
  independent bounds receipt have the same source/input identity as the current
  candidate. Their execution was reused, not repeated in this review.
- **Required resolution:** Explicitly reconcile the prescribed request and
  fixed sequence limit in the contract before regenerating preparation. No
  particular new limit is approved here. Do not remove evidence, truncate the
  target, silently change instructions/tokenization, or relax the rejecting
  check. This is not a request for a nominal product patch.

## Evidence classification

| Boundary | Classification and result |
|---|---|
| Current source, binaries, protected inputs and prior receipt identity | EXECUTED_THIS_REVIEW: SHA256 matches |
| Request-only labels, record preservation and lengths | REUSED_EXECUTED_EVIDENCE: independent bounds reader checked3,840 transformed requests; length gate FAIL |
| Request relations and native clock descriptor validation | REUSED_EXECUTED_EVIDENCE: exact candidate test1/1 PASS, exit0; no model calls |
| Preparation rejection before output publication | SOURCE_ONLY plus matching preserved failure log; no study directory currently exists |
| Full prepared split/tape/exposure and target-cost verification | NOT_RUN: no prepared study |
| Strict output scorer → gate malformed/EOS/UTF-8 regression | NOT_RUN: fixture requires the missing prepared study |
| Metal TINY continuous2 vs split1+fresh-process1, each arm | NOT_RUN: fixture requires the missing prepared study |
| Actual inherited Adam513 update, native restart and mode-specific teacher | NOT_RUN; descriptor assertions are not optimizer/forward evidence |
| New dispatcher RETURNED-only finalization | NOT_RUN: prepared-study fixture unavailable |
| B raw recount, normal32/failure≤32 per arm, teacher8/mode/arm | NOT_RUN: no baseline or F/I endpoints |

The existing request test checks value swapping preserves ID_ONLY's selected ID
while FULL's word changes, query swapping changes the selected record,
renaming changes its ID, and record order does not change the selection.
These are request/label checks. ID_ONLY value-swap invariance is **not** success
at generating the swapped word value. The descriptor test verifies inherited
origin14336/local512/study14848 and rejects mismatched families/clocks; it does
not prove an actual Adam update or native process restart.

After a revised input contract produces a valid study, the still-unexecuted
requirements above remain necessary: full split/tape audit; the actual strict
scorer/gate fixture; both arms' continuous/split Metal fixture within the
independent8-update budget; actual teacher and changed finalization boundaries.
B follows only actual baseline/endpoints. Old Metal/Muon/teacher closure,
QA640, FP4 and completed generation must not be repeated for this blocker.

## Source and reused execution identity

Source baseline: `b9cf630a1b9a6dc91ecbe144006e7d06a7d9191e`.
Starting report-only HEAD: `9252cb731dba006f5acdba986bb602bbd98bfb0d`.
The reviewed candidate is **uncommitted source**, not that HEAD's source tree.
Its working diff exactly matches the preserved candidate patch:

| Item | SHA256 |
|---|---|
| `src/muon.rs` | `7611a5ae20f80687f87c87c0ee3e52be92724a172bee538d8ee3e3d5b97bdf83` |
| `src/muon_diagnosis.rs` | `58ae80a91d71d77039f010f0e672dfa3f8646e4b24feef64084a1540486765ce` |
| `src/neural/checkpoint.rs` | `10bae1c179d652ddd6808708161532500d127418eb40e9c35f101b46b4bd0968` |
| Candidate patch | `10d9d33b32d04ccd0396a28a9e8b1c942f1c4db1574de8eba6d13c876d06e89a` |
| Preserved executor | `277d3ad75fc1a02d836e343bad85decbb0301837c34f7b34ada2dd7b6879f9f1` |
| Preserved test executable | `2bbfb9731fee56e78fc648e70604d7e93f0b98dbc32df778a3f36039b6e78be5` |
| Independent bounds-reader source | `de9c9eb51cba9c4262eff59c8811f5b5c19c75d44b434e758bf62f0e554ba671` |
| Independent bounds-reader binary | `2716e284259237bd99451b4222bb72e7dca65c064ef7ec45391b19564704f310` |
| Bounds receipt | `d9e5998fd8bc90f7e77fb784887633000e7ff902f6360400a1cf30fd4fd1a8c4` |

Reused logs: `artifacts/selected-event-id-20260924-evidence/prepare-02.log`,
`test-final-request-clock.log`, and
`artifacts/selected-event-id-20260924-review/bounds-01.log`.
The earlier preparation report is preserved unchanged. Old closure evidence is
reused through its existing accepted report and unchanged composite identity;
no new whole-artifact inventory or replay was performed.

## Preservation, usage and disposition

| Protected original | SHA256 |
|---|---|
| Original word corpus | `96e36843c792354bff98195e95db3ec98c4d0dde534aaa619787b5259665fa3c` |
| Original tokenizer | `ec945ee5f3cbd87992bdfa13f199de2a671b94b64337dff04d85e01d982ab9ef` |
| A512 native, including inherited optimizer | `a549392f3f12ebdea95616d6d66043f9b88e1e56ce40ab74d6159304069a8da0` |
| Closed teacher composite | `3c35b40dbabac5eb80f5e9b284b6f5025a27b1127b65b90aaa2f2f318f88e99c` |

The scoped21-file preservation manifest is
`artifacts/selected-event-id-20260924-recheck/preserved.sha256`; it includes
the three candidate files, Cargo files, consumed originals, reused binaries,
receipt/logs and prior reports. Its post-review verification is retained beside
it. This verifies the named inputs, not a claim of a new whole-repository audit.

New model construction/forward/backward, TINY/SMALL optimizer, generation and
teacher calls/examples/tokens: **all0**. New test executions0; new builds0.
No model active time, discarded updates or new checkpoints. Model/data artifact
bytes added0; only this report and small hash-verification evidence are added.
The previous23-file/45,885,599-byte preparation footprint is reused as reported,
not measured again. Last durable remains A512 model14848/Adam512.

`CODE_A=BLOCKED_INPUT_LENGTH`; `EXECUTION_COMPLETE=false`;
`F_I_ENDPOINT=NOT_RUN`; `OPTIMIZER_CLOCK=NOT_RUN` for new updates;
`EXPOSURE_PARITY=NOT_RUN`; `TARGET_COST_DIFFERENCE=NOT_RUN` for actual tape;
`ID_ONLY_DEV/RENAMED`, `FULL_DEV/RENAMED`, new `OLD_FULL64`, `RETENTION`,
`TRAIN_SEEN`, `TEACHER_CONDITION`, `B_RECOUNT/PARITY` all `NOT_RUN`.
`NEXT_SIGNAL=INPUT_CONTRACT_BLOCKED`; new `MODEL_QUALITY=NOT_MEASURED`;
previous model quality FAIL remains unchanged; `GOAL1_READY=false`.

This recheck closes at the confirmed preparation blocker; A/B are not accepted.
Only this report is eligible for commit/push. Its containing commit is the
report SHA, separate from the uncommitted candidate identity above. Actual
remote publication is verified after committing and reported in the delivery;
the pre-publication remote was exactly the starting report-only HEAD.
