# Full response fit — independent candidate recheck, 2026-09-25

No new confirmed implementation defect was found in the reviewed scope. Original A remains accepted; the separately authorized final-native observation A/B remains accepted for execution and evidence integrity. Original training completion and model quality remain unaccepted. This review closes the supplied candidate scope and requires no product patch or additional model execution.

| Verdict | Result and limit |
|---|---|
| Original single-F preparation / numeric / process A | PASS, existing matching evidence reused |
| Posthoc observation boundary A | PASS, existing matching evidence reused and current code inspected |
| Posthoc raw / state / usage / reproduction B | PASS, fresh independent Rust readbacks agree with existing results |
| Original planned training completion | INCOMPLETE: 3,069 committed updates plus 3 discarded backwards exhaust 3,072 backwards |
| FULL train / development / retention quality | FAIL |
| Candidate eligibility / renewed learning | False / not authorized |
| S4 / S5 / S6 / Goal1 | NOT_RUN here; no promotion |

## Identity and preservation

Reviewed candidate: `af8ad53bf52e592f531b2f29ea84670ac201e618`. Original training executed source `80ec58a37bf50d41a919a0ddfed86b4c969fd4b8`; the new final-native observation executed the reviewed candidate. Before publication local HEAD and actual remote `main` both equaled `867d22dd09a99e4f2870a865f5f8a15b3d2c7935`. Candidate-to-HEAD source/tests/Cargo/vendor diff and tracked working changes were empty. The only candidate change since the training source is `src/muon.rs`, +220/-4. The delivered patch SHA-256 is `ac36455fd9e1a0eaf42e1494c8ecaa53f1210c967036e265fed37705f745251b`.

Posthoc executor SHA-256: `052c9a7d150b97f5a0b16ac2f7968a721cc529688946163e0e72de06403f54d4`; test binary: `c6268c9bd3e67d8e2cf689805e353bcf22f614047e089292698a5e09ab209e18`. Original training executor/test identities remain in the preserved A report. Posthoc descriptor policy: `b8b3e02a3464632b108d5710ff7815e979c99a82a9aa19e39f8a5ebb866057b1`.

The independent Rust identity checker verified 72 distinct referenced files, including original A evidence, its 11 protected input references, posthoc A/B evidence, direct test logs, executables, the patch, original partial readback, terminal chain and final native. Receipt source/runtime/policy bindings agree. Posthoc A receipt SHA-256 is `3d52de63ae888bd9a1a5cc5a06c101cdab406d73e8dc7d22a86841b69466d2c6`; B is `0c4f06fb30d8224b9f3ad13019cd7a65b172c1e8446e5d1057423be68dbda27f`. All 72 recorded hashes also passed the after-check. This is a check of consumed references, not a new inventory of all artifacts.

Product source/tests/Cargo/vendor, models, Adam, corpus, original raw and prior reports were preserved. No model/corpus copy, cleanup or seal access occurred. Scratch outputs are under `artifacts/full-response-fit-20260925-recheck`; only this report is published. The publication commit containing this report is the report SHA, distinct from either executed source. Its actual remote match is checked after the normal push and returned with the final result.

## Requirement-to-path review

The `R3-FULL-RESPONSE-FIT-1.0` contract and full reviewer instructions were read. The separately authorized final-native diagnostic scope is recorded in the current quality plan. Original failed execution is preserved; its final checkpoint is not relabeled as a completed 3,072-update endpoint.

The reviewed production paths are `fit_prepare` / `fit_verify_inputs`, `load_arm` / `train_update` / `save_arm`, single-arm `fit_train`, scheduled evaluation/guard/finalization, `fit_trace`, and the new `fit_posthoc_*` / `fit_replay` paths in `src/muon.rs`. Directly attached process, scope, large-count and inherited-state tests were inspected. Unchanged F/I, Metal, Muon, scorer and teacher implementations were not reopened as new suites.

The reused A evidence establishes parent14912/Adam576/origin14336, first new Adam577, unchanged weights/full Adam/LR3e-5, native state binding and actual Metal continuous2 versus1+fresh-process1 equality. The registered tape rotates the original 3,072 rows from index576, wraps after3071 and ends575. All1,536 word rows have eight planned new exact FULL exposures. Earlier A512 exposures with a different system are semantic exposures only. Original FULL corpus, review bytes, tokenizer, prompt/target identities, train192 selection and disjoint train192/dev48 semantic groups remain bound; renamed dev shares the same48 dev groups.

Actual final cursor is3069, model17981/Adam3645. Native SHA-256 is `f61db3b873859f2aac5083ace36385067f8cbb4d250cdd4d364d0eea289ea55c`. The three deadline-discarded backwards count toward the cap. Actual new word exposures are1,524 rows at8 and12 rows at7; cumulative exact FULL is1,292 rows at8 and244 rows at9. A full cycle was planned but not completed. Terminal `INTEGRITY_FAIL`, success=false, resume=false, fit=false and last evaluated2304 remain intact.

The posthoc descriptor binds that exact original plan/terminal/trace/native and inherits prior calls/time. Its admission exception requires precisely the verified backward-cap failure; cancellation, mixed failure, UNKNOWN and other terminal errors remain excluded. It is a separate typed descriptor, not a training Study. Its sibling registration prevents a second output root from resetting allowance. The original failure remains blocking for ordinary training. Reproduction reuses completed RETURNED rows, including wrong answers, and completed observations allow pure readback only. Posthoc completion explicitly leaves candidate eligibility and planned3072 completion false.

## Current execution and evidence reuse

No new GPU test, forward, generation, teacher, optimizer or backward ran in this recheck. Existing successful model executions are reused after identity verification; they are not represented as newly executed tests. No zero-test invocation is counted as PASS.

| Check performed now | Exit | Evidence / result |
|---|---:|---|
| Standalone Rust identity reader | 0 | `identity-final-02.log`, 72 references and source/runtime/policy bindings |
| Main raw/native readback | 0 | `main.log`, all3,520 generation rows independently rescored |
| Existing B raw parity/readback | 0 | `replay.log`, 64 recorded generations, quotas/mates and usage verified |
| Main/B teacher readback | 0 | `teacher.log`, 64+8 recorded examples, role/gold/argmax/NLL checked |
| Three binary readback comparisons | 0 each | Current main/replay/teacher outputs byte-identical to previous independent readbacks |
| Preservation hash check | 0 | `preservation-after.log`, all72 references unchanged |

The identity reader was compiled offline from scratch `identity.rs` with the already cached replica_v3 rlib. Its source SHA-256 is `0aacc648d0fc49047d480f7bae63bfa620e6e18f5fc884dce799cc5d910d8b92`; executable `identity-final` is `aa4507954352dcfb8c42ba99de2af735f742aea09a879c92d5ba45c2f80096f6`. An initial final-reader launch raced its still-running compile and exited127 before executing; that log is preserved and is not PASS. Compilation completed, and the subsequent reader exited0. No product command or model was involved in that launch failure.

The verified existing independent `recount-posthoc` and `teacher-posthoc` executables were used for pure readbacks. Commands, with the single-thread environment `VECLIB_MAXIMUM_THREADS=1 OMP_NUM_THREADS=1 RAYON_NUM_THREADS=1`, were:

```text
artifacts/full-response-fit-20260925-recheck/identity-final artifacts/full-response-fit-20260925-recheck/protected-final.sha256
artifacts/full-response-fit-20260925-review/recount-posthoc artifacts/full-response-fit-posthoc-20260925-study artifacts/full-response-fit-20260925-recheck/main.r3b --posthoc
artifacts/full-response-fit-20260925-review/recount-posthoc artifacts/full-response-fit-posthoc-20260925-study artifacts/full-response-fit-20260925-recheck/replay.r3b --posthoc --only-independent
artifacts/full-response-fit-20260925-review/teacher-posthoc artifacts/full-response-fit-posthoc-20260925-study artifacts/full-response-fit-20260925-recheck/teacher.r3b --posthoc --with-independent
shasum -a 256 -c artifacts/full-response-fit-20260925-recheck/protected-final.sha256
```

Measured final identity/main/replay/teacher reader wall times were1.47/1.91/0.57/0.18s. These are file verification time, not GPU time or new experiment calls. The three generated readback SHA-256 values are respectively `f7f33553e9c7bb3b5e207a2c07162bb3cfc48889a0d3f840ddc1b9e9b8197dd9`, `423e3a038397d3a391a9d7108b2df217c8ec6c186cbb60a8f96d4a639cf409e6`, `9731937b98f83c24978f0b9d00ff76d85c3492701d54dd18734e4d4e2d00ea4f`.

## B result and quality separation

| Final3069 panel | FULL | QUERY_BOTH | SWAP_BOTH | ALL4 | Outside-ID rows |
|---|---:|---:|---:|---:|---:|
| V |512/512|256|256|128|0|
| VC |510/512|254|254|126|2|
| S1Q1 |478/512|222|231|103|33|
| FULL word dev |29/192|0|0|0|52|
| FULL renamed dev |45/192|4|0|0|38|
| FULL train |485/1536|32|1|0|28|
| OLD_FULL |1/64|0|0|0|62|

Every generation row reached EOS; malformed/error counts are0. Valid output syntax therefore does not imply correct value or support. Outside IDs remain scored as failures, without correction. Word/renamed/train value-correct counts are38/55/494 and support-correct counts140/154/1508. Train per-word correctness is left79, right166, straight146, wait94, each denominator384. Train fit, heldout development and retention all fail.

Previously executed B used one fresh model process for normal32 plus extra32: sixteen outside-ID failures and their sixteen correct query mates. The current readback reconfirmed exact token/text/EOS/finish parity64/64. B rows do not enlarge quality denominators. Main teacher64 and B teacher8 agree on gold/argmax/roles, maximum absolute NLL delta0. Teacher gold-prefix ID scores are not free-generation success; pure VALUE coverage is absent because of MIXED tokens, not zero accuracy on a measured VALUE-only set.

Recorded original generation3280/teacher64 plus posthoc main3520/64 and B64/8 yields cumulative generation6864/teacher136, generation tokens95162 and teacher targets2176. Stored active time is3602.504058792s within7200. Posthoc optimizer/backward0; current recheck adds0 to every model-call counter. Original TINY total8 optimizer/backward remains unchanged and was not repeated.

Scoped original-study plus posthoc-study/registration logical bytes were1,061,310,924 at the preserved snapshot, below1GiB. The earlier broader three-root footprint1,106,603,756 already exceeded1GiB and remains disclosed; this is not a whole-scope storage PASS. Executable/build/test evidence is separately scoped in the authorized posthoc plan. No new global size audit was performed.

## Findings and necessary follow-up

Confirmed new code issues: **none**. No speculative Severity item, nominal product patch or additional learning is requested.

Existing identity-matched tests cover unit guard/legacy encoding, native inheritance and new-process integration, single-arm/large-count regressions, wrong-model/oversize rejection, and UNKNOWN/cancellation/corruption/RETURNED/missing-row failure paths. Their actual prior execution evidence is retained. This recheck found no additional necessary test within the supplied change scope. New GPU tests, training, additional generation, prior full suites, QA640 and seal/S4/S5/S6 runs were NOT_RUN here. Original execution incompleteness and model quality FAIL remain separate from posthoc execution/evidence PASS.
