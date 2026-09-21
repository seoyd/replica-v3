# Citation continuation — independent A, 2026-09-22

**INDEPENDENT_A: PASS. ACTUAL_PREPARATION_PREREVIEW: PASS.**

Reviewed source: `c9c121077bbe1aadc546e344ab237d38728e3d3e`.
Source digest: `65cee12e4d3c0c4149a7b02c7026614eb93206c2c61f6bdcab97ce33d92bd0c8`.
This is independent execution of the confirmation/resume/accounting boundaries and verification of the specific ANSWER4384 continuation preparation. It does not accept model quality, B, citation confirmation, S4/S5/S6 or Goal1. Parent32 fresh parity remains the next prerequisite before SMALL updates.

The earlier candidate `493025591f1616ab73d86820e7d7bfec5622c31a` was not approved: an actual resolution-publication I/O fault exposed incorrect known-token accounting. Its preparation and all failure evidence remain intact and receive no approval. The final source fixes that boundary only; the reviewer did not edit product source or existing tests.

## Executed source and commands

Each candidate was archived from its exact Git commit into a separate reviewer-owned source directory. Builds used installed Rust/Cargo1.98.1, the existing lock and offline dependency cache, Accelerate and one compute thread. Original source, models, corpus and raw were read-only. Large evidence stays under ignored `artifacts/citation-continuation-20260922-review/`.

Final commands, with `CARGO_TARGET_DIR=A2-target`, `R3_CONT_TEST_ROOT=A2-fixture` and the isolated source as cwd:

```text
cargo build --locked --offline --release --features accelerate --bin replica-check
replica-check --output A2-quick quick --citation-continuation
```

The checker actually ran:

```text
cargo test --locked --offline --features accelerate,test-support --release --bin replica-train citation_continuation_ -- --nocapture --test-threads=1
```

Final build and quick both exited0. **2 tests passed, 0 failed, 155 filtered; source unchanged.** Test execution took304.40 seconds; compile time is separate. The two test names are:

- `training::fresh::identifiable::binding::citation::tests::citation_continuation_guard_and_record_boundaries`
- `training::fresh::identifiable::binding::citation::tests::citation_continuation_process`

No zero-test result was accepted. Existing loss-math tests were not rerun.

| Evidence | SHA256 |
|---|---|
| Final independent test binary | `0160323272e1730c2ce237ea6ff8b6563d26d7e2a9c9926f329c21b0455f04fd` |
| Independent checker binary | `c5287c821190876e99f0d81356a1a8898cda7c00141a39d7a1143c5f2ac5ed13` |
| `A2-quick/summary.r3b` | `6b57a4fbcd7315e37dd7388072678688ee7d3fe306633e33992614029cac1a1d` |
| `A2-usage-recount.log` | `dbae43df9bad23568d0b972d682256969aa19f77743a12b76186a23ab7fcd9af` |
| `A2-preparation-recount.log` | `f9fcc51ebd28e7b8bd5d67fd89864f2972545702e5629f0d43a9228cb87075b5` |

## Verified boundaries

The actual TINY process tests use the production preparation, collector, per-call journal, native checkpoint and evaluator. The explicit tiny confirmation admission is8 rows and does not relax SMALL quality gates. Synthetic writer/publisher/reader fixtures use a fixed512 expected panel: complete512 succeeds;0/1/511 fail final completion. Valid partial prefixes remain resumable only under the specified state contract.

Candidate publication, definite pre-call TIME, returned-row TIME, last-row TIME and final-only completion survive fresh processes without regenerating returned rows. Continuous and segmented confirmation tokens/rows/denominators match. Repeated complete confirm/report calls are pure reads. EOS-fixture continuous2, split1+1 and evaluation-only2+0 have identical native weights/Adam/cursor and output evidence.

The separate random seed93 TINY comparison preserves its real control-token failure. Continuous2 and fresh-process1+1 native files, weights, Adam and training state match. Definite no-call TIME followed by a new process produces the same raw token/error/cursor as continuous evaluation, with no additional optimizer. Reentry after RETURNED is blocked with zero generation. This is failure-path/state equivalence; random evaluation is not falsely labeled a completed positive-quality panel. EOS-fixture normal completion is reported separately.

Additional independently prepared scratch tests exercised40 rejected malformed full confirm/report invocations and4 unchanged/restored positive invocations. Each made zero model calls and left fixture files unchanged. Covered boundaries include candidate checkpoint/source/binary/policy/B/seal, another seal-owner claim, mixed and non-time terminal/per-call conditions, missing/pending/UNKNOWN resolution, missing/duplicate/reordered/truncated raw, attempt gaps and extra ordinals. All980 fixture files matched their pristine copies after restoration. These tests used the unchanged final-consumer bodies from493; the subsequent c9 change only moves returned-call bookkeeping before fallible publication and adds its direct regression.

A reviewer-only test attachment, without changing any product function body, additionally exercised actual native command timeouts at0 and1 generated tokens. Each failed returned row remains intact while the next process finishes only remaining rows; subsequent complete reentry generates0. Actual length outputs are also reused without repair. An I/O pending resolution and cancellation remain sticky. This attachment used7 actual generations, optimizer0 and teacher0.

The actual `continuation_guard` consumed copied real plan/corpus/metadata and synthetic V64 raw: first59 creates warning1; re-reading the same step does not increment it; the next scheduled warning creates2 and stops; normal64 resets to0. Altered prior receipt counts or a prior stop are rejected. This is a raw/receipt chain test, not only a standalone threshold assertion, and used no model calls.

## Executed finding and final repair

On493, the I/O fault returned four real native tokens before resolution publication failed. Raw was preserved and retry blocked, but segment `generated_tokens` incorrectly said0 because `rows.push` followed the fallible write. Severity was Medium: inaccurate known usage, without candidate laundering. The final source retains returned rows before append/resolution while preserving pending/non-time rejection.

The final frozen quick explicitly ran resolution failure and a fresh-process retry. Raw token count and final usage now agree; the returned count is1, `generation_calls=1`, and the pending resolution remains ineligible. The retry adds0 calls. Final I/O receipt hash: `6dce26705b88bc40fe0367f72302d9b6d290de922dcdc37cf1c6434d3e65810e`. The original four-token failing fixture is preserved, not rewritten.

Reviewer execution mistakes are retained rather than omitted: an extra-ordinal expectation looked for a later error string while the product correctly rejected it earlier as unaccounted usage; cancellation returned lowercase `cancelled` rather than the expected uppercase string. Exact remaining checks subsequently verified rejection. An overly broad `reviewer_` filter also started one existing test; it was stopped during synthetic setup and its directory preserved. No native-parent/review call or optimizer entry had occurred. That test is STOPPED, not PASS; its separate elapsed time is UNKNOWN. No missing receipt was reconstructed. Scratch reader compilation/selection errors likewise carry no model calls and are not counted as product-test passes.

## Actual usage

| Execution | TINY optimizer | Generation | Teacher |
|---|---:|---:|---:|
| Initial independent frozen quick on493 | 20 | 174 | 104 |
| Additional native fault tests | 0 | 7 | 0 |
| Final independent frozen quick on c9 | 20 | 175 | 104 |
| **Independent A total** | **40** | **356** | **208** |

Pure readers, synthetic records, malformed full-entry tests and approval publication add0 calls. New SMALL updates/generation/teacher and scalar finite differences are0. The two quick runs independently account for total training input47,424 and target2,560. Pending I/O calls are included in actual generation use; they are not treated as reusable confirmed rows.

Including the implementer's preserved accounting, the entire repair has **TINY128 updates, generation1149, teacher704, FD0**. The TINY optimizer cap is exhausted; no further optimizer test is authorized within this budget. The implementer's first failed process accounting includes its explicitly labeled source-derived counts and missing-canonical-evidence limitations. This review does not turn those limitations into new verified receipts.

## Actual preparation accepted

Approved root: `artifacts/citation-continuation-20260922-study-02`. Earlier `artifacts/citation-continuation-20260922-study` remains unapproved and unchanged.

| Binding | SHA256 |
|---|---|
| Production executable-02 | `9f150bf0d5855c2956db8802ee91f3038389765afaaeddf1d336cd2868981d31` |
| Preparation | `5be75f097064ac5156409adaab0ebfe23c5321e59534fc0aa35422f7af8205f2` |
| Selection | `8bd2b0eb35016689ce44aa5e093dae690a2ecaf4dda2841a76e24a2d0fb313a7` |
| ANSWER-MEAN plan | `ea42ffe7b08e2ef3c6c70b57a319ba70bb95cc85069a8a81868b3d3d5a6d9026` |
| Exact4384 native parent/initial | `838754d751a71e4fe7971a4127a76c17a0ed05c4d8efdf524c79248324c7a1dd` |
| Parent tensor content | `31525bbc55d7cde77fcdbe6d8870b33a61b5b838ea412440b0dce28791aad692` |
| Parent Adam136 | `c0bee7aba8609c9f888760e7713eb56acac98dc0e9944ffeac617e725e60b4c9` |
| Unchanged corpus | `80b43942dc4854e114558f17a2b5d71e1ad0c71685511ded7c9b4457d4091c81` |

Pure Rust readers verified native family6/normalizer2, first-target1, QE, unchanged tokenizer/model/Adam, batch8 and LR3e-4. The original7424-row tape is identical; only suffix index32 through3071 remains for this new fork:3040 updates, input3,623,680, target231,040, padding97,280. V/VC0/VC1 draws are12,160/6,080/6,080. Next absolute update is4385; planned evaluations are4416/4480/4608/4736/5120/5888/7424. Upper planned generation/teacher budgets are14,528/13,312.

Original ANSWER4384 V64 independently recounts full60, QUERY_BOTH28, SWAP_BOTH28, ALL4=12, EOS64 and generation errors0; four wrong rows affect four distinct orbits. VC64 full0/value50/syntax0/support0/EOS64 is unchanged. This preserves the original QUALITY_REGRESSION and resume=false rather than repairing their historical flags.

All11,607 protected original files match their prior manifest digest `4a1b33d4f543b5de04f02a07808b85c871f412ffa8b00e78d8210ff6bf8dded4`. The three registered links were checked, including the failed-source preparation; all citation confirmation histories remain unused. The seal body was hashed but not decoded. Scalar4352 acceptance remains historical evidence, never a new-model selection panel.

Next authorized step is parent16-value plus16-citation parity, then the fixed3040-update continuation only if that prerequisite passes. B and any eligible citation confirmation require their own later evidence. **GOAL1_READY=false; GOAL1_ACCEPTED=false.** Report publication has its own commit SHA, separate from the reviewed source above.
