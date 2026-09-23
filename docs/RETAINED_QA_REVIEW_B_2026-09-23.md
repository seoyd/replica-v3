# Retained QA transfer — independent endpoint review B

Date: 2026-09-23. Contract: R3-RETAINED-QA-TRANSFER-1.0.

**B_INTEGRITY: PASS.** The bounded learning run is complete, but its
joint quality gate failed. This review checks the actual execution and evidence;
it does not promote the failed endpoint. Independent A and the accepted11264
value/citation baseline remain preserved. Product source and original artifacts
were read-only; no new training, TINY or teacher calls were performed by B.

Reviewed source: `3a5f54ed91dffd8a5782e148c56a56dddf2244c8`.
Source digest: `7199cdbe4ebc0c018adf0ba70c3b1b1c874adc5a5411aa9b492bea3d2a721ca4`.
Frozen executable: `artifacts/retained-qa-20260923-executable`, SHA-256
`a833ff1919fb166d7a493e60c1f4c34f9697a7e82ed34ba80853e72222c357d7`.
The report is published separately from the reviewed source.

## Execution, native state and evidence

The existing independent Rust reader was minimally adapted to the retained-QA
panels and linked against the unchanged Accelerate production rlib verified in
review A. It independently read all4096 actual update rows,10 segment terminals,
all7168 generation rows and7168 teacher rows, and14336 prepared/resolved call
pairs. All raw panels were complete and bound to their actual same-step native,
policy, tokenizer, request, normal-greedy framing and strict UTF-8/EOS output.
Frozen expected answers, raw token decoding, summary metrics, per-bucket/pair
scores, all guard transitions and final eligibility were recomputed and matched.
No saved exact-match flags or old aggregate scores substituted for recounting.

All4096 updates use the frozen full tape and bit-exact LR3e-5, with ANSWER
family6/normalizer2/first-target1 and unchanged QE framing/tokenizer. The actual
per-example CE was recombined using the answer-mean denominator8 separately from
its token-weighted diagnostic CE. Every actual input/target/padding count matched
tokenized samples; the native Adam136 tensors were finite and the clock, sampler,
objective and cumulative usage matched each durable checkpoint.

| Actual work | Count |
|---|---:|
| New SMALL updates |4096|
| Origin → endpoint |11264 →15360|
| Training examples |32768|
| Input / target / padding tokens |6006578 /454700 /1574558|
| V / VC0 / VC1 exposures |8192 /4096 /4096|
| Q0 / Q1 exposures |8192 /8192|
| Evaluation generation / teacher |7168 /7168|
| Evaluation generated tokens |100751|
| Committed/discarded updates |4096 /0|
| Known uncommitted input/target |0 /0|
| Pending or UNKNOWN evaluation calls |0|

The planned exposure and cost totals exactly match independent A. Q0/Q1 each
expose every8192 row once; retention per-row repeats remain V5–6/VC2–3. The10
training/evaluation segments record3941.807455625 active seconds. Wall time of
reader/build/close work is not model inference time. The preserved relative-root
CLI error occurred before segment admission, with no optimizer/generation/teacher
calls. Registered absolute-root commands then used the same plan; the error was
not repaired into a successful run.

Final native:
`artifacts/retained-qa-20260923-study-final/ANSWER-MEAN/segment-0009/final`.

| Final identity | SHA-256 |
|---|---|
| Physical native file |`6b95de2c4ead5203d7acdbda7a76544b3307a0bb2d1787de8f2a3be787f08b3d`|
| Manifest weights |`4f75192796125320073f7bcc674f9e9b17d3c1957a9e027eebe7710926a198b0`|
| Tensor content |`493281a262fb80c6d6954f01f816748403a67c7350ecd4788541dc00a82f7e54`|
| Evaluator model |`79c5f1ddd527b8943d6e5868506fcd566c618f6ab763bbaa3ea99786701ef88d`|
| Adam136 content |`21bbf82bbb66c64ae203743d50d2709130e5533d9440627b8e59da4822b5102e`|
| Training state |`1d6e0760cd9c2db8c3f4e9f2bb0e6fd8165e5be8d119f1ce9483ad3d2b06d7b5`|
| Final terminal |`6a23d5758a96fe02d21160beb1bea1063afd9ad29ab2cbcf3652771433c41a34`|

Native step/sampler15360, cumulative input19294002 and target1049644. The final
state is Finished/resume=false, stop=PERSISTENT_RETENTION_REGRESSION. Renamed
retention had warning streak1 at14336 and2 at15360; the other streaks remained0.
The4096-update cap was also exhausted. Recount does not clear this stop or restart
the experiment. The original11264 checkpoint and its accepted scope are intact.

## Same-endpoint quality

| Final panel | Exact / total | QUERY_BOTH | ALL4 | Outside ID |
|---|---:|---:|---:|---:|
| Value |512/512|256/256|128/128|0|
| Citation |505/512|250/256|123/128|7|
| ID-renamed citation |496/512|241/256|119/128|16|
| Old QA primary |27/512|not defined|6/128|428|
| Old QA transfer |24/128|not defined|6/32|92|
| Balanced QA primary |13/512|1/256|not defined|463|
| Balanced QA transfer |3/128|0/64|not defined|114|
| Fixed Q1 train probe |21/128|8/64|not defined|98|

All final heldout rows ended with normal EOS and had runtime/UTF-8 errors0. The
train probe had127 EOS and one normal length termination, retained in the full
128 denominator. Low QA scores therefore cannot be dismissed as only truncation
or runtime failure. All final A–F buckets scored0: old/balanced primary0/64 each,
transfer0/16 each, train probe0/16 each. G/H alone account for the observed QA
correct answers. The Q1 train probe is a fixed128 subset, not full-train accuracy.

Balanced C/D/E/F QUERY_BOTH was0/32 each. In balanced primary G, unresolved
no-evidence cases were11/11 correct, but the corresponding resolved cases0/11;
ambiguous and wrong-target subtypes were0. All32 G resolution pairs failed.
Balanced transfer similarly had three correct unresolved no-evidence cases,
resolved0, and all8 pairs failed. H fixed-output exact/emission counts were2/64
in balanced primary and0/16 in transfer; non-H fixed emissions were0. Old QA H was
0/64 primary and16/16 transfer. Old four-view value/order/wording pair counts and
actual changed-input denominators were separately recomputed; they are not
relabeled as balanced two-view selector success.

Against the same preserved11264 retention raw, value lost0 answers, citation
lost7 and renamed citation lost16; there were no gains because the parent was
already perfect on those panels. At the earlier13312 endpoint the QA scores were
old64/512+11/128 and balanced71/512+19/128; the later decrease is an observation,
not proof of a specific forgetting mechanism. No intermediate endpoint is
promoted by this review.

Stored teacher rows were independently reaggregated without new forward calls.
Final token-mean CE: value0.000590056, citation0.002819296,
renamed0.008384029, old primary1.643313457, old transfer1.767091800,
balanced primary1.477424332, balanced transfer1.520032677,
train probe1.227834128. Retention token-level gold/NLL observations were checked;
QA token-position observations were absent in the original rows, so only their
recorded mean/counts were reaggregated. Missing position diagnostics were not
invented or replaced by additional teacher calls.

## Independent fresh-process reproduction

Four separate invocations of the frozen executable completed with exit0:

```text
fresh answer-mean-review --root <absolute-study>/ANSWER-MEAN
fresh answer-mean-review --root <absolute-study>/ANSWER-MEAN --citation
fresh retained-qa-review --root <absolute-study>/ANSWER-MEAN --panel old_qa
fresh retained-qa-review --root <absolute-study>/ANSWER-MEAN --panel balanced
```

Each invocation used VECLIB_MAXIMUM_THREADS=1 and OMP_NUM_THREADS=1. The existing
production normal native generator received the request, not expected answers,
an answer resolver or output corrections. Expected raw was used only after
generation to compare the result. No retries or additional samples were run.

| Fresh panel | Matched | Correct | Wrong reproduced | Tokens | Active seconds |
|---|---:|---:|---:|---:|---:|
| V prefix16 |16/16|16|0|32|1.295569083|
| VC prefix16 |16/16|16|0|272|1.631737833|
| Old QA two per bucket, wrong first |16/16|0|16|208|1.643699334|
| Balanced QA two per bucket, wrong first |16/16|0|16|289|1.810504416|

The independent reader compared every raw token, strict byte/text decode, EOS,
finish, error, prompt digest and returned-call receipt against the original row.
Retention prefix samples did not contain citation failures; their full raw
failures were recounted, while actual wrong-case reproduction was32 QA rows.
These selected16-row panels do not estimate population quality. The preserved
sample indices are bound in each started receipt and in `B-parity-usage.r3b`.

B made exactly64 generation calls,801 generated tokens and6.381510666 active
seconds, with teacher/optimizer/TINY0. Including the original672 parent calls,
the contract totals are7904 generation,7168 teacher,4096 optimizer and147080
generated tokens,4070.374851875 recorded active seconds. Parent usage was read
from its existing receipts, not regenerated. Native/corpus/plan and all30243
pre-review study files plus the20 A-protected inputs had unchanged hashes after
these new observations. New observation receipts were written only to the
designated study paths.

A reviewer CLI spelling error (`--panel old-qa`) was rejected by argument parsing
with exit2 before admission and any model calls. Its failed log is preserved;
the corrected registered `old_qa` invocation above is separately recorded. This
is not a discarded generation or a successful first attempt.

## Decision and local evidence

The full joint development gate failed at13312 and15360. Conditional full-fit4608,
S4 final200 and its controls, S5 and S6 were NOT_RUN_PREREQUISITE. The independently
prepared S4 seal remains NOT_OPENED_FOR_MODEL; past citation confirmation was not
reused as a new candidate test. There is no eligible candidate or new quality
acceptance.

| Decision | Result |
|---|---|
| BASELINE11264_PRESERVED |PASS|
| PREPARATION_VERIFIED |previous independent A preserved|
| TRAINING_EXECUTED |PASS —4096/4096|
| OLD_CAPABILITY_RETAINED |FAIL joint guard; value-only retained|
| QA_PRIMARY / TRANSFER |FAIL for both fixed sources|
| BALANCED_QUERY_CONDITION_USED |NOT_ESTABLISHED; C–F paired correctness0|
| B_INTEGRITY |PASS — execution/evidence scope only|
| S4 / S5 / S6 |NOT_RUN_PREREQUISITE|
| GOAL1_READY / GOAL1_ACCEPTED |false /false|

Local evidence root: `artifacts/retained-qa-20260923-review`; run originals remain
under `artifacts/retained-qa-20260923-study-final`. No raw text, corpus, checkpoint,
Adam or temporary instructions are published with this report.

The pure readback command was `read_B_retained` with one compute thread and
exit0, output `B-recount-01.stdout`. `B-recount.r3b` SHA-256:
`ede2770071d6e0d91a74e1e49e66886bfd70c42965cd58acd735d775b2d47533`.
One scratch-reader delimiter compilation error was corrected before the first
readback; it made no model calls and its log remains preserved. No product
validator, source, checkpoint or result was changed. Existing A/TINY acceptance
was reused; no regression suite was rerun for this read-only B review.

| Independent readback artifact | SHA-256 |
|---|---|
| Rust reader source |`da8ed0f46e1871ad138f7c4e159899143300e30823ed303cf68cf31e744b77e9`|
| Reader executable |`6cf0e361b9b09fd37e4cfcc06625248a1f2170be06716e8f9a13c43df1f2ee53`|
| Recount stdout |`640e2c75799d6c2ed48c143d7df594f74cd7281e93b5b680ed4dcc2fe3bac9dd`|
| Original study preservation manifest |`fc82e4d142cf08a4be8a231132b05878eb6828a7ee43f12b7bb66f0d6d8159c2`|
| Fresh64/usage readback |`2fe392cf2f8e5460d111025cd7a14a3b0b6de7ddfb6f8de519b9532d89f4a8b9`|
| Fresh64 readback stdout |`4f2da195236278db571ca6dd8f6deaf1cb9e5d17ad941f1463fc87cef2fa4f12`|

No full Cargo rebuild was required. The small read-only Rust helper used the
existing current Accelerate production rlib SHA-256
`a9126c1a260edacd3a54ab89d7341366649ad1ca479e5df21c8da5699d0b8df0`.
All generation used the independently hash-checked frozen production executable,
not the reviewer helper and not a substitute output generator.
