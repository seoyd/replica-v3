# Teacher device closure — independent candidate recheck

Contract: R3-TEACHER-DEVICE-CLOSURE-1.0. Date: 2026-09-24.

**SUCCESSOR_DIAGNOSIS: PASS. No additional confirmed current-code defect.**
The accepted actual caller and independent reproduction evidence match the
candidate and raw records. A newly executed independent recount confirms them.
Model quality remains FAIL; Muon adoption remains NO; S4/S5/S6/Goal1 are not
accepted. Original training and predecessor diagnosis remain FAILED_UNCHANGED.

This review did not repeat the completed model calls. The existing policy had
already consumed all216 teacher forwards and128 generations, including the
independent24/128. Those results are identity-checked reused execution evidence,
not new forwards attributed to this review. No budget was extended or reset.

| Scope | Verdict and evidence class |
|---|---|
| A_DELTA | PASS; unchanged source and recorded negative fixtures reused |
| ACTUAL_TEACHER_CALLER | PASS; actual first3 forward/record/readback evidence reused and checked against current full raw |
| D2_IMPORTED_INTEGRITY | VERIFIED; newly executed independent recount of896 reused rows |
| TEACHER192_COMPLETE | PASS; newly executed independent target/device/role/native reader |
| TEACHER24_PARITY | PASS; completed independent forwards reused, raw comparison executed anew |
| REPRODUCTION_A / M | PASS64/64 each; completed model reproductions reused, raw parity and selection rechecked |
| Completed-lane no-call reuse / D2 regeneration denial | PASS in new production processes |
| ORIGINALS_PRESERVED | PASS within the scoped references and before/after manifests |
| New model generation / teacher / optimizer / backward | 0 /0 /0 /0 |
| New model reproduction in this recheck | NOT_RUN; existing required reproduction was already completed |

## Source and artifact identities

- Reviewed and frozen execution source: `1344ae3b4e2f359db81d889e1c2c24b2356e7b17`.
- Starting report HEAD: `a823452d36b27dd4f3896934965044a05af2c997`;
  source/tests/Cargo/vendor have no change from the candidate.
- Comparison baseline: `673883ceea433ca351c1629edac9d365d3945e7a`;
  teacher device repair was `f5895057ba40e1758c72bf7dba5e37fef21d6445`.
- Predecessor D2/D3 producer: `4850195a1b5cb1d51f7f49fccae14ed194243f56`.
- Frozen executor SHA256:
  `c6b89acddb2587abbef941ddc40813362330866e3a719bd84faa7f64a3c6907d`.
- Plan SHA256 / policy:
  `5b2368f13073d41ced50ecff960ee20c3653065b9cee07a2eefb8889e24c4b4a` /
  `0b1b017796960c4d0c84814252101a49e6950ccc57118a50ded2aa7a5e6c740c`.
- Final native composite SHA256:
  `3c35b40dbabac5eb80f5e9b284b6f5025a27b1127b65b90aaa2f2f318f88e99c`.
- Parent/A/M native steps:14336/14848/14848, A/M local512.
  Their physical hashes and content/tokenizer/framing identities match the
  preserved plan and independent metadata reader.

The existing exact offline executables were reused without rebuilding. Model
math, tokenizer, generation, optimizer, Cargo lock and Metal vendor patch are
unchanged in this delta. Existing A7/Metal/Muon suites were not repeated.

## Reviewed production boundaries

`verified_d2` and `predecessor` in `src/muon_diagnosis.rs:142,180` validate the
three completed predecessor lanes and the separately failed teacher attempt.
Original `segments` failure handling is retained. Successor provenance references
the old D2 producer; it does not relabel old rows or reopen either failed run.
The new observation path at line396 rejects D2 regeneration, binds fixed
manifests and requires A-caller admission before the remaining work.

`teacher_prefix_until` in `src/fresh.rs:2686` preserves the full64-case header
and immutable row ordinals while collecting a deliberate1-row prefix. It neither
fabricates a timeout nor declares that prefix a complete lane. Empty legacy
input remains supported. Continuation skips the durable first row.

The actual teacher path uses the repaired native-device input, calls
`model.forward`, checks F32/finite logits and stores token NLL/argmax plus input
and device identity. `validate_forward` at `src/muon_diagnosis.rs:368` checks
target shift and native observations. Acceptance here rests on actual recorded
forwards and independent check24, not only helper tensor allocation.

The direct negative and prefix/process fixtures in the unchanged candidate were
read and their identity-bound evidence reused. They cover changed D2 rows,
identity/step/runtime, pending or incomplete resolutions, mixed failure causes,
old-root reentry, prefix/full distinction and duplicate rejection. No remaining
confirmed issue justifies a patch or repeating the old numerical suites.

## Executed now, without model calls

New local evidence: `artifacts/teacher-device-20260924-closure-review/`.
Reader code was inspected before execution. All commands below used existing
executables; production processes used one compute thread.

| Command, relative to repository root | Exit / wall seconds |
|---|---|
| `artifacts/teacher-device-20260924-review/recount-plan artifacts/teacher-device-20260924-run --require-closed` |0 /1.21|
| `artifacts/teacher-device-20260924-review/caller-reader artifacts/teacher-device-20260924-run complete artifacts/teacher-device-20260924-closure-review/full-readback.r3b` |0 /1.26|
| `artifacts/teacher-device-20260924-review/verify-final artifacts/teacher-device-20260924-review/caller-readback.r3b artifacts/teacher-device-20260924-closure-review/full-readback.r3b artifacts/teacher-device-20260924-run` |0 /0.01|
| Frozen executor, `fresh muon diagnose observe --root artifacts/teacher-device-20260924-run --lane check-parent`, then separate processes for `check-A`, `check-M`, `replay-A`, `replay-M` |Each0;3.61/3.60/3.74/8.78/8.61|
| Same observe command with `--lane missing` |Expected1 /3.57; D2 regeneration forbidden|
| Frozen executor, `fresh muon diagnose report --root artifacts/teacher-device-20260924-run` |0 /21.20|

The frozen executor is `artifacts/teacher-device-20260924-evidence/replica-train`.
All five completed observations returned `REUSED_COMPLETE ... new_calls0`.
These are fresh-process reuse checks, not another152 model calls. No Rust unit
tests were rerun, and zero tests are not counted as a new test PASS.

## Recounted conclusions

The reader rechecked1,856 scoped references and all896 reused D2 rows:
historical586 plus missing54 gives composite640, with train256 separate.
Historical M renamed10/64 remains partial in the predecessor; its completed
successor composite includes the additional54 with distinct provenance.

All192 main teacher rows have canonical gold/EOS, aligned prompt/target digests,
finite NLL, valid argmax and Metal U32 input/F32 logits evidence. The newly
generated model-free full-readback receipt is byte-identical to the previous
independent receipt: SHA256
`a3d33a89b2d97629b179c60542155b3b9eb05da8bac7cd74e9dfb17a35896990`.
The original three accepted row hashes match the full192; each continuation
consumed63 forwards. First3 were included in192, not regenerated.

Independent check24 preserves gold/argmax exactly; maximum absolute NLL
difference is0 over384 targets, within1e-5. Both replay unions contain normal32
and failure-extra32, with no fabricated mate or duplicate. All128 raw outputs
match, including M's two invalid-UTF8 outputs and13 length endings. Wrong outputs
remain wrong. The replay sample is not used to estimate model accuracy.

Both free train panels remain FULL0/128. A whole-value/support counts are50/0;
M counts are0/2. Per arm42 cases were seen once and86 twice. The teacher train32
subset consists of twice-seen cases. Independent role/span aggregates match all
six model/split groups. VALUE has no pure-token denominator because of MIXED
VALUE/FORMAT tokens; this is not VALUE accuracy0%. Clean eight-digit ID spans
have coverage32/32 and full-span correctness0/32 in every model/split.
High FORMAT/EOS scores do not establish successful free generation.

Successor usage remains teacher216, generation128, target tokens3,456,
generated tokens2,105, active40.235394543s, failed0/UNKNOWN0. Historical D2
generation310 and failed teacher1 remain separate. This review adds no model
usage; reader/process wall times above are not GPU execution time.

## Preservation, remaining scope and publication

Before/after checks match162 source/tests/Cargo/vendor files, all733 successor
run files including membership, and68 reused source/log/receipt/report files.
The pure production report returns the unchanged content digest
`963b88385a4c7b696756f85dcb905bf44e8ca8371629f656f9eaf9526d2fa126`.
The run still occupies1,450,079 logical bytes. Only new reviewer logs/manifests,
one independent readback receipt and this report were written. No original
report, model, Adam, corpus, raw or user `.DS_Store` was modified. No source or
vendor copy, new build, cleanup, external model/API or learning was performed.

The bounded diagnosis is closed. The existing proposal to compare an ID-only
response target with the original response is still a proposal, not permission
to train. A lack of disjoint heldout exact-ID improvement would argue against
it; even improvement would not establish full QA. Its target-length/objective
confound remains explicit. S4/S5/S6/Goal1 remain NOT_ACCEPTED.

Only this new report is committed and pushed. Report commit and actual remote
SHA are recorded in the final handoff and local publication receipt, separately
from the reviewed/executed candidate. Original reports are preserved.
