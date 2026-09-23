# Independent B — word-value guard closure

**RAW / GUARD / USAGE: PASS (independent read-only execution).**
**Full B fresh-process reproduction: NOT_RUN — guard-stop scope.**
This is a verified early retention failure, not acceptance of the model or the
complete B reproduction stage. No new B acceptance/QA authorization receipt is
issued. Word/joint development is not accepted; scheduled full dev and fit were
NOT_REACHED. QA640, confirmation, S4/S5/S6 and Goal1 remain unaccepted.

Reviewed source: `c1f82a719dcd72ecca2e7b8024384562423f37c2`.
Source digest: `2811ed5b1abbd2799925979b1ccabd38fa854537b38b4744946eeec5647ec877`.
Frozen production binary SHA256:
`d8e20a6b304fc9d1575b7ab43dca557f276486c3515a60b790f5d0468940b3b4`.
The publication commit is separate from the reviewed source. Existing A report,
A receipt, product source, corpus, raw and closed endpoints are unchanged.

**Observed stop.** The registered14336 parent fork saved its first update14337,
then continued in a new process for127 updates. At14464 the completed five
scheduled64-row panels triggered `SEVERE_RETENTION_REGRESSION`. The terminal is
`Finished`, `resume=false`; no additional optimizer or model work followed.

| Fixed retention panel | Parent FULL / ALL4 |14464 FULL / ALL4|Lost exact answers|Guard|
|---|---:|---:|---:|---|
| Numeric value |64 /16|64 /16|0|No warning|
| Numeric citation |64 /16|62 /14|2|No warning|
| S1Q1 instruction |64 /16|24 /3|40|Immediate severe stop|

All three parent comparisons use the same original first64 cases. Their parents
are fully correct in that fixed subset; historical errors outside it are not
counted as newly lost answers. The warning streak is0/0/1, and the independent
calculation confirms the immediate16-answer-drop rule. Word0 itself did not
trigger this guard.

| Completed14464 panel |FULL|QUERY_BOTH|SWAP_BOTH|ALL4|Value|Support|Outside-ID rows|Malformed rows|
|---|---:|---:|---:|---:|---:|---:|---:|---:|
| Value64 |64|32|32|16|64|—|—|—|
| Citation64 |62|30|30|14|63|63|1|0|
| S1Q1-64 |24|9|10|3|64|24|39|1|
| Word64 |0|0|0|0|12|0|60|4|
| Word-renamed64 |0|0|0|0|12|0|64|0|

Every panel has EOS64/64 and runtime errors0. The independently implemented
individual-ID scanner preserves outside-ID counts even when syntax is malformed;
the two columns can overlap. S1Q1 retains the correct first numeric value on all64
cases while selected-event support falls to24. This observation does not prove
a particular internal mechanism or identify a sufficient remedy.

Parent observations were preserved and audited: metadata/failed-pair parity
matched16; word dev and renamed dev are each0/192, with outside-ID rows20 and28.
Those two panels share48 semantic groups. They are not384 independent scenes.
The new endpoint has only the64-row screens above; it has no final192-row word
dev result, train-fit result or general QA result.

**Stored teacher evidence.** All320 teacher rows were bound to their actual
cases/model/prompt and target tokens; finite per-token NLL and recorded CE were
recomputed without forward/backward execution. For the word screens:

| Panel |First/value-overlap token NLL|Citation token NLL|Gold-prefix EOS NLL|Value / citation tokens|
|---|---:|---:|---:|---:|
| Word64 |1.4810933731|1.3731370272|0.0000293864|64 /896|
| Word-renamed64 |1.4821339864|1.3792395631|0.0000444059|64 /896|

All64 value tokens in each panel cross the value-byte boundary, so this is the
existing overlapping-token diagnostic, not a pure value-byte loss. Gold/foil
first tokens differ on all64 rows of each screen; shared-first count is0. Teacher
results are diagnostics and do not replace normal generation scores.

**Actual usage and lineage.** The complete128-row committed trace matches the
registered tape prefix, LR3e-5 bits, ANSWER objective denominator8 and actual
masked input/target lengths. No gradient/objective normalization was substituted
for token accounting. Both saved native states have136 finite Adam tensors,
objective family6/normalizer2, unchanged QE/tokenizer and cumulative clocks.

| Executed train stream |Exposure|Input tokens|Target tokens|Padding tokens|
|---|---:|---:|---:|---:|
| V |128|18,560|256|7,168|
| VC0 |64|9,792|1,088|3,072|
| VC1 |64|9,792|1,088|3,072|
| Numeric bridge |256|45,696|4,352|5,760|
| Word |512|102,912|8,192|0|
| Total |1,024|186,752|14,976|19,072|

SMALL totals remain **optimizer128 / generation720 / teacher320**, including
parent400 plus evaluation320. Recorded active time is159.388163415s. Preserved
generation raw contains11,655 tokens including EOS: parent7,426 and endpoint4,229.
The final native cumulative counters are input17,602,944/target935,552 and
step/sampler14464. These are distinct from this study's new usage.

Final checkpoint:
`artifacts/qa-word-value-20260924-study-final/ANSWER-MEAN/segment-0001/final`.

| Identity | SHA256 / content identity |
|---|---|
| Physical final file | `152c18bc1656e2fbbb7889fd3bbae826b15d33abb5d012fc4ab6985b3302d6f5` |
| Manifest weights | `15e2606fd8d1bc0305473a64fbe9877ffa6231c99f3be317e6c4e90938b86256` |
| Tensor content | `226ea8e7d3db98107651900bbe24a102753210344878a6e1a50d22f1778272f5` |
| Evaluator model | `3ce7523f25797bc52ebf0e6b61a9c180a3da1d5efaae707f56013ffa64515afc` |
| Adam136 tensor digest | `6cae12632e43e132df667507781de8064cd59b791a71da139a2e7e43438ada5c` |
| Native training-state digest | `3c015054f9e4d53fa01be24c75d9d94b88fba9266acbcf11a8d324b194587423` |
| Final terminal | `b8ceb5604e98665916e9867c6b351ddcbcfa4a69567a8286c97e954cb87052ed` |

The initial relative-root CLI attempt failed before admission because the plan
registers an absolute root. Its original `train-segment-0000.log` is preserved;
it created no segment/entry/command and used optimizer/generation/teacher0.
The separately logged absolute-root execution accounts for both real segments.

**Independent execution evidence.** The Rust reader reuses the previous native
corpus/checkpoint reader and accepted independent ID scanner, with only the
new word score/teacher spans and current path/guard accounting added. It checks
all720 new raw rows,320 teacher rows,128 updates, both native states, case/policy/
model hashes and640 evaluation prepared/resolved pairs. Missing/extra evaluation
streams and pending records are rejected. Consumed protected hashes match before
and after. No repository inventory, test suite or historical QA was repeated.

The first scratch compile failed because two reused helper functions were
private; no execution occurred. The corrected compile and `read_B` exited0.
Build used the existing locked release rlibs and Accelerate, with runtime compute
threads1. Reviewer optimizer/generation/teacher/backward calls are **0/0/0/0**;
TINY remains the accepted total14/276/276. No B64 calls were consumed.

Evidence root: `artifacts/qa-word-value-20260924-review/`. Private row content
remains there, not in this report.

| Independent artifact | SHA256 |
|---|---|
| `read_B.rs` | `1a6ed9ebdf3497dc0543682086c031a8efe07c59a485ca2e75e9e1459ccc9f28` |
| Reused reader/scanner source | `08c5a0afddaca4b5d2952510f0f0c8aa57c5a517792ecc2c54ef9d10d0cc39b8` |
| Reader executable | `77844241ffd488d6422962252a33be600d782da40b68abc923e4c8f1d004b526` |
| `B-recount-01.log` | `62411cb3a59659e954ac1365ca031de366646ff8759ee40001794a343f511872` |
| `B-recount.r3b` | `7308c08c3eb96707fc3e02273d8999a62ebacdae3543c6ec2d9882ccb3a122a7` |
| `B-protected-manifest.r3b` | `605e9d55152ac56acf66046ff3c74b9f324b181aa6b68f1874da9c5093ce10e0` |

The ordinary B reproduction allowance is not extended to this guard stop merely
because the CLI can admit a read-only endpoint. Fresh reproduction is explicitly
NOT_RUN. The run stopped before either normal full endpoint; fit, QA640 and
further learning are prohibited here. This preserves the measured negative
result and the previous accepted models without granting new model acceptance.
