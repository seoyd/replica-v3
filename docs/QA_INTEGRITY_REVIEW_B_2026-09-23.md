# Independent B — bounded QA instruction bridge

**INDEPENDENT_B: PASS.** This accepts execution and result integrity;
the final model has **BRIDGE_DEVELOPMENT_FAIL**, with no eligible candidate.
Conditional train fit and old QA640 were not run. S4 remains unopened;
S5/S6/Goal1 are not accepted. The protected11264 acceptance and closed A1/A2
reviews remain unchanged.

Reviewed learning source: `8522d026512cfb7b6e4b6c32142c5d68b77c9f2f`.
Source digest: `9b46c11e60cf4ecbd44a77d28c82f70b59dc94895e5a2a81149c2f0576c500b4`.
The reviewer changed no product source, tests, Cargo files or original artifacts.
The report commit is separate from the reviewed source. All new reader code is
Rust, confined to ignored reviewer scratch; original raw contents are not published.

| Bound artifact | SHA256 |
|---|---|
| Production `executable-bridge` | `341f018c30aa57feaf6ae1a8e50ddadedb1c91492a998da0f76bdd12b2ea6394` |
| Study `preparation.r3b` | `7c8fc48fef59c47ab065e0f563eff300040a41344d89ce5823d2648acea1c378` |
| `ANSWER-MEAN/plan.r3b` | `13d292570d17b39c7e69b502a29cc2dfeacf0c41bb93dbae672349c9b864c261` |
| Final native physical bytes | `80970cee2ca6dbeb258bc7f202e612f6e178c9185c590acc23c99b6941a98d73` |
| Native manifest weights hash | `451a2b28b1282b6cc39525fbcb34cfbbf95150025c84b28a6d99f64705ec8f43` |
| Evaluator model identity | `ce54b5a22d252115df9290151b4dd9d30f62fa2db478bfd7020c89b74c2cf926` |
| Tensor content identity | `dd6bb75420c26c74a34744f4233939bbe213c3a664b23417173478662dace154` |
| Ordered136 Adam tensors | `cc340e07fcb5c8dd667bb72d160a528f2131d214e9cac56d3fa9674522ad00e7` |
| Training state digest | `5d2501c16bda5974172cdbb332ad782e3f92708fffcdd013fff9d72cf14b638f` |
| Final segment terminal | `2a81c3973318da799111225416989ee24b3c10a2879a040fe05a02849c9ad594` |

Physical bytes, manifest weights, evaluator model and tensor-content identities
use different representations; these hashes are not interchangeable.

**Executed independently.** `read_B_bridge.rs` reused the frozen native corpus,
tokenizer, checkpoint and typed record readers. A separate byte-scanning ID
implementation, retained from A1, independently counted valid positive IDs even
when another citation is malformed. No model forward/backward/optimizer call was
made by either reader. B did not rerun the closed A1/A2 tests or claim zero tests
as a new test PASS. The production native generation commands are listed below.

The reader verified five saved segment/native/Adam/clock/control chains, every
one of1536 LR/tape/update records, all18 panels (six64 and twice six512),
6528 generation rows,6528 teacher rows, and13056 prepared/resolved call pairs.
Actual tokens, strict UTF-8, EOS/errors, question/evidence/prompt hashes, expected
training-only labels, summary scores, teacher target masks/NLLs, guard streaks
and same-model final decisions agree. There were no missing rows or pending calls.
The complete histories and source-bound A2 evidence were preserved.

Training used the registered native11264 parent, unchanged QE/tokenizer/Adam,
constant LR3e-5, family6/normalizer2 ANSWER CE, EOS included and prompt/padding
excluded from loss. Each update's objective denominator is8 while token usage
remains the actual target-token count. Exposure was V3072, VC01536, VC11536 and
bridge6144:12288 examples total. Input2064384, target162816 and padding233472
match preparation exactly. The first11265 checkpoint was saved and the following
segment resumed its native state in a new process. Final12800 is
`Finished / BRIDGE_DEVELOPMENT_FAIL / resume=false`; existing terminal records
were not changed or supplemented to make the run eligible.

**Final12800 independent raw recount.** Every row below uses the same saved model.
EOS512/512 and generation errors0 apply to every panel. All parse-failure counts
are0. FULL accuracy alone is insufficient: each citation panel also requires
outside-ID rows0.

| Panel | FULL /512 | QUERY_BOTH /256 | SWAP_BOTH /256 | ALL4 /128 | First value /512 | Exact support /512 | Valid outside rows |
|---|---:|---:|---:|---:|---:|---:|---:|
| V |512|256|256|128|512|—|0|
| VC |512|256|256|128|512|512|0|
| Renamed ID |512|256|256|128|512|512|0|
| S1Q0 |511|255|255|127|512|511|1|
| S0Q1 |511|255|255|127|512|511|1|
| S1Q1 |508|252|252|124|510|510|1|

The +768/12032 bridge FULL511/511/506 also failed outside-ID0. Continued training
was within the original1536-update cap; no extension or new policy was used.
Retention V/VC/renamed stayed512/512 at the final endpoint, with no correctness
gain or loss against the protected11264 on these three panels. Phrase transfer
improved over the factor probe, but the remaining citation errors prevent the
registered development acceptance. No claim about general QA follows from this.

The six final wrong rows are disjointly classified as three valid outside-ID
rows, one wrong provided event with the correct value, and two wrong-value rows
with the correct selected event. Malformed citation, generation failure and
format-only error counts are0. The outside rows are S1Q0 index416, S0Q1 index416
and S1Q1 index477; S1Q1 index416 cites the wrong provided record, while indices135
and425 have wrong values. These are zero-based local panel ordinals, not new
training labels or product answer rules.

Saved teacher rows were independently reaggregated without new forwards. Their
NLLs do not replace greedy correctness or the outside-ID gate:

| Final panel | First value NLL | Gold-prefix EOS NLL | Citation suffix NLL | Citation ID NLL |
|---|---:|---:|---:|---:|
| V |0.00007562|0.000005153|—|—|
| VC |0.00059100|0.000005082|0.00030163|0.00055316|
| Renamed ID |0.00064572|0.000004989|0.00017514|0.00031623|
| S1Q0 |0.00017067|0.000001249|0.00065559|0.00121802|
| S0Q1 |0.00010324|0.000001573|0.00310834|0.00581597|
| S1Q1 |0.01699685|0.000001178|0.00522610|0.00978384|

**Fresh-process reproduction: PASS.** Both commands exited0. Normal32 matched
32/32 (all correct); the six failed query pairs matched12/12, including all six
wrong answers and their six correct counterparts. Every selected original token,
byte sequence, prompt digest, EOS, finish and error field matched. Prepared and
resolved call records contain44 RETURNED calls, attempt0, with no retry or
UNKNOWN. Normal generation consumed424 tokens/2.5841795s; failed-pair generation
consumed204 tokens/1.447022375s. No additional teacher or optimizer calls occurred.

The frozen executable was invoked once per mode, sequentially with
`VECLIB_MAXIMUM_THREADS=1 RAYON_NUM_THREADS=1`:

```text
artifacts/qa-integrity-bridge-20260923-implementation/executable-bridge fresh qa-bridge-review --study /Users/seo/Projects/Replica-v3/artifacts/qa-integrity-bridge-20260923-study-final
artifacts/qa-integrity-bridge-20260923-implementation/executable-bridge fresh qa-bridge-review --study /Users/seo/Projects/Replica-v3/artifacts/qa-integrity-bridge-20260923-study-final --errors
```

Normal indices use the predeclared six-panel8/4/4/8/4/4 coverage. Error indices
are separate complete failed query pairs, excluding normal indices; they are a
reproduction diagnostic, not an accuracy estimate. Expected answers are checked
after generation; the frozen generator receives only the request and model.
The existing RunControl900s, inference12GiB and native normal-greedy path remain
in force. No teacher, optimizer, retry or new candidate selection is authorized
by these reviewer observations.

**Closed usage.** Actual totals include the immutable factor probe80, all training
evaluations6528, and reviewer44. The two pure readers compiled and exited0; the
small summary reader only displays existing audit records. Product tests were
not repeated. Remaining numerical headroom is not authorization for another run.

| Usage | Actual | Registered cap |
|---|---:|---:|
| SMALL optimizer updates |1536|1536|
| SMALL generation calls |6652|9216|
| SMALL teacher forwards |6528|8192|
| Additional diagnostic backward |0|0|
| Training input tokens |2064384|4000000|
| Training target tokens |162816|300000|
| Registered command active seconds |2072.37719825|7200|
| TINY optimizer / generation / teacher, reused closed A2 ledger |14 /168 /168|64 /1024 /1024|

Generated tokens total96717:94656 from training evaluations,1433 from the factor
probe,628 from this review. Training/evaluation segment active time is
2059.343500083s; factor probe9.002496292s; reviewer observations4.031201875s.
Build time and pure readback/scoring overhead are excluded from these existing
RunControl counters and are not described as model execution time. All five
learning commands and both reviewer commands exited0. B added no TINY work.
The28 A2-protected files and116 files in the B consumed-artifact manifest hash
identically before and after review, including original parent and closed run
evidence. Product source/tests/Cargo have no diff against the reviewed source.

**Local execution evidence.** Study root:
`artifacts/qa-integrity-bridge-20260923-study-final/`; final native:
`ANSWER-MEAN/segment-0004/final`. Implementation logs/executable are under
`artifacts/qa-integrity-bridge-20260923-implementation/`.
Independent Rust readers, binaries, commands, logs and readbacks are under
`artifacts/qa-integrity-bridge-20260923-review/`.

| Independent evidence | SHA256 |
|---|---|
| `read_B_bridge.rs` | `75a2cfa7cffe19c2d8735295e541fc8b677297862d85d86a706795d09d3b7a1c` |
| `read_B_bridge` executable | `c2bf03cb2033b95f2ae75f5a67b9ab00dc5310c64c647bdf917141a249271b39` |
| Production rlib reused by readers | `a9126c1a260edacd3a54ab89d7341366649ad1ca479e5df21c8da5699d0b8df0` |
| `B-recount.r3b` | `446c8eb8e93a1eacdb23d33be008fead8e4d95022e88ba1eb41fc71e62647d2f` |
| `B-protected-manifest.r3b` | `f3726b6e63da1383fc9b87bb0d79ea4c44bf238a92473e92c3a6aa3ffa0e33ab` |
| `B-parity-usage.r3b` | `d8ec475afd3223e1bde4b85ee28837b4c7925fa4caccbab21a88b14a3d932193` |
| `read_B_parity.rs` | `0c50e20dab0ae5ae3acc8ef7d4f90f1fbc11bb64b5d3cbecce998d66733f5baa` |
| `read_B_parity` executable | `45311609a633f7f2cae0eb8297c5cfcafaf5c0f1fd4fecc9fe82ffd727886889` |
| `B-normal-command.log` | `df379c036d74debef62c8cbe8152202e5d84564a644221111186698442d3beca` |
| `B-errors-command.log` | `37946a28b7122809dbd21d9ace434eb4f8187177a756bf03e51734e64805a79a` |

`B-summary.log` displays the already-saved `independent_failed_rows` classification.
The parity log's optional `FINAL_CLASS` display used an absent shorter key and
printed empty maps; this display was not an acceptance assertion. The original
audit classifications and outside/parse counts were already correct and remain
unchanged. The summary reader corrected that display without another model call
or raw recount. Full canonical audit bytes, not that optional printout, are the
classification evidence.

The native B receipt binds the report's exact bytes, preparation and immutable
12800 policy/checkpoint/decision. Its integrity verdict cannot override the
development failure or admit QA640. No independent confirmation was created;
the existing S4 seal was not opened. Further experiments are outside this review.
