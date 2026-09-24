# Muon quality comparison — independent candidate re-review

Contract: R3-METAL-F32-MUON-QUALITY-1.0. Date: 2026-09-24.

**A_NEW_OPTIMIZER_BOUNDARY: PASS. B_FULL: BLOCKED_RUNTIME. Overall: PARTIAL.**
The final candidate's optimizer, native resume and corrected output-error boundary
passed new execution. No new confirmed current-code defect requires a patch.
The existing A512/M512 study remains failed and closed; this review neither
repairs its terminal nor authorizes consuming its unused training/call budget.
Retention and word-learning quality are unsuccessful. General QA, S4/S5/S6 and
Goal1 remain unaccepted.

| Field | Verdict |
|---|---|
| CODE_AND_NUMERIC / A | PASS for the tested new optimizer/native/caller boundaries |
| PARENT_WEIGHTS_IDENTICAL / OPTIMIZER_RESET_SCOPE | Verified same learned14336 weights; all optimizer states fresh |
| PARAM_GROUP_COVERAGE | Hidden q/k/v/o/gate/up/down only; embedding/norms aux AdamW |
| MUON_NUMERIC_VARIANT | F32 sum momentum, Nesterov, NS5, match-RMS shape scale as specified |
| RUNTIME_BINDING / RESUME_PARITY | PASS in actual TINY Metal and new-process tests |
| B_AVAILABLE_RAW_STATE_TRACE | VERIFIED by a new pure recount |
| B_INTEGRITY_REPRODUCTION | BLOCKED_RUNTIME; fresh normal/failure reproduction NOT_RUN |
| RETENTION / WORD_LEARNING | FAIL / no successful word selection demonstrated |
| RESEARCH_RECOMMENDATION | No automatic extension or Muon quality adoption |
| GOAL1_READY | false |

## Identities and scope

Reviewed/current executed test source:
`6e336ea3db65054cd95c9800aa46008cae932f41`.
Actual historical learning source:
`6d71d0418222039c7e23440cffaf78c09a26ce7f`.
The latter produced the preserved incomplete study before the output-error repair.
The current candidate and actual remote main matched before report publication.
This report-only publication commit is separate; its exact commit and remote match
are recorded in the final handoff and local publication receipt.

The unchanged Metal reduction acceptance was reused after checking the exact
patch `34faeb37febb834770c5679093b2f22d26c6eff01f40154a7854691b76ca43b2`,
patched source `20ccaadf0f1b097b366636d48dbb79f4009d66da4abfcaefc37f2d6bd6b833d6`
and lock `a911931fc3baca7367a34c129673640661a8fc3d41458d49e4eb744ae5454154`.
No old Metal reduction suite,48-step backend training, LegacyB or FP4 was rerun.

| Newly executed artifact | SHA256 |
|---|---|
| Exact candidate test binary | `e0c6fb7268e6d33cb73b6c55d1e1033ebed5405cd76b859096ee8dd879764633` |
| Independent scalar oracle | `123ddc25342c2fd84222ce2895d7bf111adf1fc085374c38de05a1459aedbea9` |
| Oracle source | `196193b9719a840478fc645513c0313999209c0672f0ac512b7623a45b06ca3d` |
| Candidate arithmetic extraction | `b7e0356ab336ac98fe8e6a73fd8bea8a6c73152d07db844856018568b1b711c6` |
| Preparation reader | `84a821852ac03e1f357d24e976725952419b5382be670bf5d4de612b06ba365d` |
| Raw/state/trace reader | `351ee88670635b44ef459c28c855e49257b4a1e6ad534814cc06c4cba6466efd` |
| Raw reader source | `ee955ba8f9343df6face9823f227bbea95b0955b81a133ce891479fb768cdbad` |

Original policy `6133bfd80aebf586b409fc67e6f4f78a8444df98d50c6e10685c83250d871399`,
plan file `76b8c24f50406b4a6861c42bb04d8decda2519f87c57a4003ad71e5abc223c5b`,
parent native `15015900a3c91a6b8ed87e9c2ecb8f7faf124849e5dcf8f40203cddf267e30d9`,
and corpus `96e36843c792354bff98195e95db3ec98c4d0dde534aaa619787b5259665fa3c`
were independently rechecked. The learned parent content is
`45d0c75ef6eca2901a9357fb26e1e4b0ccf8f812b2592d2febea43b2d46cf7a5`.
Historical trainer remains
`1efbb40511e7f19f2b448e582d17f5b9b77d9b852f787cd28f2a686a4adccfc7`.

## New A execution

A minimal source copy was extracted from the candidate; product source/tests/Cargo
and the single vendor were unchanged. Rust1.98.1, locked/offline release,
accelerate+metal, incremental disabled, matching shared target cache, scratch
TMPDIR and one compute thread were used. Build exited0 in33.36s. Every exact test
below executed one top-level test, passed1/failed0, exit0. Child processes are
included, not counted again as separate top-level tests.

- `small_shape_ns_cpu_metal`: six Q/K/FFN-shaped numerical fixtures, no SMALL model.
- `roles_and_rejection`: hidden-role coverage; missing/NaN gradient, missing state,
  unsupported optimizer and incorrect clock rejection before mutation.
- `answer_accumulation_updates`: actual ANSWER batch8 versus4+4, four TINY updates.
- `native_policy_and_completed_no_call`: native family/role/clock/runtime rejection;
  immutable RETURNED reuse, missing/UNKNOWN/cancel boundaries, no model call.
- `native_new_process_restart`: A and M each saved at1, continued to2 and restored
  at1 in a separate process for the second update. Weights/state/local clock/cursor
  are exact; six total TINY updates. Its stale ignore-description says8, but the
  executed code and actual invocation count are6.
- `final_evaluation_only_process`: actual train caller in a fresh process with
  exhausted time and synthetic complete RETURNED fixtures; no new model call.
- `returned_decode_quality_guard`: actual collector/scorer/guard accepts only
  raw-proven strict UTF-8 output failure as wrong quality; EOS decode error plus
  three length rows gives severe error union4. Runtime/cancel/mixed errors reject;
  exact returned command-timeout rows with0/1 tokens reuse without regeneration.

A reviewer-owned f64 scalar-loop oracle was also executed anew on CPU and actual
Metal F32 (registry4294968525),16 primitive fixtures. It covers square/wide/tall
matrices, two updates with nonzero history, mixed/rank-deficient gradients,
zero-gradient remaining momentum, zero H and fresh Adam clock1. The scalar
polynomial is independent of Candle/product NS. Candidate tensor helpers were
extracted verbatim, with only Adam's namespace renamed for linkage; production
optimizer/native tests above exercise their actual caller separately. Norm floor,
pointwise/update bounds are unchanged; near-zero cosine is not fabricated.

Both states allocate zeros while preserving learned weights. The first local
Adam clock is1, not14337. NS normalization/transposition, sum momentum, Nesterov,
five polynomial iterations, shape scaling once and unscaled-LR decay match the
contract. All-role coverage includes one tied embedding update; ndim alone does
not select Muon. No BF16/CPU NS fallback was found or executed. Native v3 validates
the optimizer descriptor and exact state key set; generic training rejects a
research optimizer without its bound caller.

The preparation reader re-tokenized all1024 tape draws:8192 exposures,5120 unique
rows, input1,512,064/target119,808/padding138,368 per planned arm. Review4/word4,
sequence256, existing train7680/validation3456, content/token hashes and inherited
beta/eps/clip/decay are verified. These are prepared costs, not actual1024-step
training. The original tape's stored absolute-prefix representation was verified
against the explicit local1024 plan; trainer uses `s.tape[p.local]`.

## B recount and remaining boundary

New preparation readback exited0 in1.42s; new failed-study recount exited0 in5.45s.
Both were model-free. The latter verifies29 complete panels, the final10-row
partial panel, immutable entered/RETURNED bindings, strict token decode,
value/support/outside-ID/parse/EOS, QB/SB/ALL4, all consumed trace rows and native
state. Word/renamed share semantic bases; incomplete renamed observations are
not treated as a complete independent comparison.

A and M each committed512 updates,4096 exposures/3584 unique rows,
input753,536/target59,904/padding71,296. Local optimizer clock/cursor512 corresponds
to model step14848. Combined1024 updates, generation1882 (1866 evaluation rows
plus baseline16), teacher0, raw tokens26,982. A discarded pre-optimizer M437
forward/backward is separately preserved; it is not another committed update.
Both arms retain the same tape, reset scope and nominal LR3e-5, not identical
update norms or FLOPs.

At512, V is64/64 each, VC63/64 versus64/64, S1Q154/64 versus40/64; word FULL is0/64
for both. A renamed is0/64; M renamed is incomplete10/64. A's128/512 warnings are
distinct scheduled observations and stop it at512. No better arm is extended.
Synchronized median optimizer time is0.1154697085s A versus0.1706262915s M;
whole observed step0.668259333s versus0.722869146s. These are the same release/Metal
study, not the old CPU/debug speedup. Peak RSS/GPU/NS temporary peak remain UNKNOWN.

The original M512 strict-UTF8 returned error was misclassified by learning source
6d71d04, leaving `success=false`, `resume=false`, `INTEGRITY_FAIL`. Current candidate
corrects that classification, but does not rewrite the old terminal. The available
partial evidence is consistent; full B remains blocked. Word train128, teacher,
normal32/failure<=32 fresh reproduction and full1024 panels remain NOT_RUN.
No incomplete evaluation was filled in and no failed study was reopened.

The read-only selection probe reports normal32 plus failure32, unique64. Normal
prefixes are complete query pairs. Twelve selected failure rows lack their mate
on A512 data. The initial suspicion that this necessarily violated the contract
was withdrawn after checking the surrounding wording: complete-pair coverage is
mandatory for normal32; failure-pair selection is qualified as where possible.
This observation is not a confirmed defect, not a reason to require a nominal
patch, and not evidence that B reproduction ran. The actual closed-study Review
command was never invoked.

## Costs, preservation and publication

New reviewer usage: **TINY updates10, primitive fixtures22 (six actual shapes plus
sixteen independent oracle fixtures), SMALL training/backward0, generation0,
teacher0**. Cumulative study verification totals now TINY30/32 and primitive120/128.
Seven test processes plus the oracle used39.00s wall, including fixture/native
I/O; this is not claimed as pure GPU time. Compilation and pure readers are
separate. Historical study usage remains unchanged. Adding prior post-run fixture
27.66s and this39.00s to historical1043.33592275s gives a conservative1109.99592275s,
below7200s, without modifying any original receipt.

The before/after hashes match for study plan/preparation/fresh-state, all original
segment terminals and five native files, frozen executables and bound A/B reports.
All available raw bindings and returned resolutions were rechecked. The five-file
older protection ledger also matches. Product/tests/Cargo/vendor are unchanged;
`.DS_Store` is preserved. No cleanup, full original artifact inventory, model/API
access, new dependency, SMALL retraining or seal access occurred.

New review root at measurement:34,022,845 logical bytes /38,166,528 allocated bytes,
including isolated fixtures and sources; vendor symlink is not followed. Matching
release target grew from1,204,520 to1,204,524KiB during this invocation. This is a
scoped observed build delta, not whole-contract growth or reclaimed space.

Only this report is published. Evidence and exact commands remain local under
`artifacts/muon-quality-20260924-closure-review/`. No additional optimizer repair
or rerun of the accepted numerical scope is requested. Full B requires an eligible,
complete endpoint under a separately valid execution decision; this review does
not grant one or request that the preserved failed study be retried.
