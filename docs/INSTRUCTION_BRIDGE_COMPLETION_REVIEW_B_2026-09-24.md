# Independent B — instruction bridge completion

**INDEPENDENT_B: PASS** for execution and result integrity of the closed14336
endpoint. **BRIDGE_DEVELOPMENT_PASS=false; BRIDGE_FIT_PASS=false.** No candidate,
learning resume, confirmation, S4/S5/S6 or Goal1 acceptance follows from this review.
The complete known-result endpoint qualifies only for the separately authorized
old QA640 diagnostic. That diagnostic has not run at this report's publication.
Its later review must be a separate addendum; this receipt-bound report is immutable.

Reviewed source: `c502e66872ad6abdf463990c5992061d3ec3a19c`.
Source digest: `dab51655d7ad6a5bd4c9f9ffcd0906efa80f9f4971dc93719bd234909021e434`.
Product source/tests/Cargo have no diff against that source. Only ignored Rust
reviewer helpers, new reviewer observations and this report were written.
The report commit is separate from the reviewed source.

| Identity | SHA256 |
|---|---|
| Frozen production executable | `ffffb7f2f2dab68f04a73d579f65fe8d55b9177c00244705d7bd12788c472713` |
| Preparation | `755c60e07729eb4f3712ce1574ab05a2396024e07e2b2ae40a7d4fe483d151a5` |
| Native plan | `9fd65bb560f985f0ce325fe31ad0931592e2af3463b46ea283004ff7f96f20f1` |
| Final physical file | `15015900a3c91a6b8ed87e9c2ecb8f7faf124849e5dcf8f40203cddf267e30d9` |
| Native manifest weights | `9cbcac14496c39929b233ce204d4b9650fe775112d3b0900cf50ae8f4b74e28c` |
| Evaluator model | `b4afc6efd0cbd4e306128d75f23b7be4dcd356cc4a23de99bbeff311c3f4fc3b` |
| Tensor content | `45d0c75ef6eca2901a9357fb26e1e4b0ccf8f812b2592d2febea43b2d46cf7a5` |
| Ordered136 Adam tensors | `f0868369435813856aca46b5f01ccf090d6e0e14735244ecc6db93915f7ac4e0` |
| Training state | `70f50b56ff06e49d46bd0733c7665ceb66e7f239077480bfd007485bad994544` |
| Final segment terminal | `4a5f5cf215f3a2f2a9eba8ba5d50f464b6915be9b3d2b9393139d32c38fd588d` |

The physical file, manifest weights, evaluator model and tensor identities use
different representations. Both segment0004/final and segment0005/final have the
same physical hash above; the TRAIN_END weights digest is not their file hash.

**Independent executed checks.** Existing Rust readers were adapted only for the
new12800 origin, actual endpoint, parent32 and final fit1536. They used frozen
native corpus/tokenizer/checkpoint/record readers and the independent strict and
individual-ID scanners retained from the accepted parser review. No reader made
a model forward, teacher or optimizer call. Closed A and historical reviews were
not repeated; no zero-test invocation is counted as a test PASS.

The reader checked six segment/native/Adam/clock/control chains, all1536 update
records, all19 panels,8064 generation rows,8064 teacher rows and16128 matching
prepared/resolved records. Actual prompts, raw tokens/strict UTF-8/EOS/errors,
training-only labels, dataset/model hashes, teacher masks/NLLs, summaries, guard
streaks and final decision agree. Every expected row and terminal exists; no
UNKNOWN, pending record, mixed cancellation/I/O condition or missing panel remains.
The25 A-protected and120 consumed-artifact files match their before/after hashes.

Training retained native12800 weights/Adam, QE/tokenizer, ANSWER CE family6/
normalizer2, first-target1, batch8 and constant LR3e-5. Trace objectives use the
eight-answer denominator while token accounting uses actual target masks, with
EOS included and prompt/padding excluded. The exact1536-update suffix yielded
V3072/VC01536/VC11536/bridge6144 exposures, input2064384, target162816 and
padding233472, matching preparation. Final14336 is
`Finished / FINAL_BRIDGE_QUALITY_FAIL_AT_14336 / resume=false`.

**Same-model final recount.** EOS512 and runtime errors0 hold for each dev panel.
All six dev parse-failure counts are0. The three retention panels have no gain or
loss against parent12800: each remains512/512. Outside rows are counted per panel;
the three expressions are not independent population samples.

| Panel | FULL | QUERY_BOTH | SWAP_BOTH | ALL4 | First value | Exact support | Valid outside rows |
|---|---:|---:|---:|---:|---:|---:|---:|
| V /512 |512|256|256|128|512|—|0|
| VC /512 |512|256|256|128|512|512|0|
| Renamed /512 |512|256|256|128|512|512|0|
| S1Q0 /512 |511|255|255|127|512|511|1|
| S0Q1 /512 |511|255|255|127|512|511|1|
| S1Q1 /512 |508|252|252|124|510|510|1|
| Bridge fit /1536 |1535|767|767|383|1535|1535|0|

The six completed dev errors are three valid outside-ID rows, one wrong provided
support with the correct value, and two wrong-value rows with the correct support.
S1Q0/S0Q1 outside index416; S1Q1 outside477, wrong support416, wrong values135/425.
These local zero-based ordinals are diagnostic provenance, not product rules.

**Returned timeout, preserved without regeneration.** Fit ordinal1354 returned
`model: native generation timeout`, class `timeout`, after7 tokens in segment0004.
It has one attempt0, one RETURNED resolution, resumable=false for that call and
only TIME_BUDGET in the recorded conditions. The known partial output contributes
one runtime failure and one strict parse failure; fit EOS is1535/1536. The fit
gate therefore fails despite satisfying its numerical FULL/joint thresholds.
This is not a normally completed semantic error or an unknown call.

Segment0004 closed EvaluationPending/TIME_BUDGET. The fresh segment0005 executed
optimizer0/generation181/teacher182 and completed all remaining rows, preserving
the failed row and the same native/Adam/state. It did not retry ordinal1354.
The registered endpoint is thus complete with a known failed result, rather than
a partial panel. Six dev panels, full fit and a normal quality-failure terminal
permit the contracted DIAGNOSTIC_ONLY QA640 after this B; no quality criterion
or candidate-required access is relaxed.

**Fresh reproduction: PASS.** Before calls, the reader froze the six-panel
metadata selection32 and six complete failed query pairs12. Both frozen CLI
commands exited0:32/32 and12/12 original outputs matched, including all six
completed wrong answers and their counterparts. The fit timeout was excluded
from error replay and remains unreplaced. All selected token/byte/prompt/EOS/error
fields and RETURNED receipts match;44 calls, attempt0, no retry. Expected answers
were used only after generation; the native generator receives the request/model.

```text
executable-bridge fresh qa-bridge-review --study /Users/seo/Projects/Replica-v3/artifacts/instruction-bridge-completion-20260924-study-final
executable-bridge fresh qa-bridge-review --study /Users/seo/Projects/Replica-v3/artifacts/instruction-bridge-completion-20260924-study-final --errors
```

Here executable-bridge is the frozen file under
`artifacts/instruction-bridge-completion-20260924-implementation/`.
Both commands used CPU/F32/Accelerate, one compute thread, unchanged QE,
normal greedy/strict UTF-8/EOS, context2048/max32 and existing RunControl limits.
Normal32 used424 tokens/2.696577792s; failed-pair12 used204 tokens/1.512511875s.
B added teacher0/optimizer0/TINY0. Error-selected results are not an accuracy estimate.

Saved teacher first-value/gold-prefix-EOS/citation-suffix/ID NLLs were independently
reaggregated without new forwards. For S1Q0 they are0.0000982691/0.000000963453/
0.000428573/0.000795140; S0Q1:0.0000570914/0.00000122865/0.00319636/0.00598418;
S1Q1:0.0178263/0.000000931788/0.00537595/0.0100700. They do not replace greedy gates.

**Usage before QA640.** SMALL optimizer1536/1536; generation8140/9216
(parent32+evaluation8064+B44); teacher8064/8192; diagnostic backward0.
Input2064384/4000000, target162816/300000; active2374.886546917/7200s.
B generated628 tokens in4.209089667s. The inherited failure-inclusive TINY ledger
remains26 optimizer/586 generation/360 teacher; B does not reopen it.
Remaining budget is not authorization for additional learning or replay.

Both final Rust readers compiled and exited0 using existing locked dependency
artifacts. Initial helper linkage omissions and one assertion that expected a
literal TIME_BUDGET error string were preserved as reviewer setup failures, with
model calls0; actual native error text and typed TIME_BUDGET receipt were then
verified. There was no product change or model-command retry.

Local evidence root: `artifacts/instruction-bridge-completion-20260924-review/`.
Original bodies, models and logs remain local and are not published.

| Evidence | SHA256 |
|---|---|
| `B-recount.r3b` | `0d53e203407d3f5585d00e2853e4e10884baf5be96d4af9d3457322199d53bfb` |
| `B-protected-manifest.r3b` | `f2edfe0345f4c19e3726979b6badc95823d1642da41d038a6f866b91eb663c8d` |
| `B-runtime-boundary.r3b` | `93af5e72411a3708d690be8c79ca7acb0d9c83be6941fc9be59994c3f71c3a61` |
| `B-parity-usage.r3b` | `4e3076461decd1e7013159b5eca62eddd1b665b242d5743d9753988ccbd2c885` |
| Recount Rust source | `b7b7823dcc25a1eeac4314b67d956eb10d0ec9582263c7806ebfd297cc30a1ca` |
| Recount executable | `f3f5d95d2c9c8e2917083fdc47757ab2c38bffbc2baa7a481f7e08be147f3f82` |
| Existing production rlib | `a9126c1a260edacd3a54ab89d7341366649ad1ca479e5df21c8da5699d0b8df0` |

QA640 remains NOT_RUN here. Retention train4608, confirmation and S4 are NOT_RUN;
existing protected11264 acceptance and all previous failures remain unchanged.
