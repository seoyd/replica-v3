# Citation precision — independent B

Review began2026-09-22 and completed2026-09-23. Scope:
R3-CITATION-PRECISION-1.0. **INDEPENDENT_B: PASS.** The fixed11264 candidate's
native state, complete saved evaluation, teacher observations, optimizer trace,
usage and fresh V16/VC16 reproduction were independently verified.

The same checkpoint passes the registered development and full-train gates.
This permits the contracted single citation confirmation after the bound B
receipt; it does not mean confirmation or Goal1 has passed. Confirmation remains
NOT_OPENED at review close; QA640 and S4/S5/S6 are NOT_RUN. GOAL1_READY=false and
GOAL1_ACCEPTED=false.

Product source/tests/Cargo and original evidence remained read-only. Existing
Rust readers, binary/native loaders, scorer definitions and production reviewer
commands were reused. No new training, TINY work, teacher computation, model
API, dependency download or output correction was performed. Reviewer generation
was exactly32 calls. The completed value16 and pure recount were retained across
an agent usage interruption; only the absent citation16 was subsequently run.

## Frozen identities

| Item | Verified identity |
|---|---|
| Reviewed product source | `3290fec66db372c5ab427bf2e632bf573824ce3d` |
| Training source digest | `121483eb645720cfa8369693ae25fe05bf54782773fc9a3e508dd793fdb244c3` |
| Frozen executable | `0020e08670d6e66de5b4c2759fac21f012f2bd610c98474580af32fee525132f` |
| Preparation | `798ab26a728c374a93271c5a51be3a236acffb1cdf596e39d29a938205a50120` |
| Plan physical file | `08254fdc34e17692910f2d04f30b8ca276cd847573fc293b64697da96ccddf0c` |
| Plan content digest | `6908abb68b12b0d54205da4c666c7b9d618fac77e84ddc7da7fe5d7286fff775` |
| Tape content digest | `d23d41239a5e57bf5f30cf17af2e373a902fc72d76183da9d6dcca32ebb3cd78` |
| Final11264 physical native | `c47e34c7ac4f88dd08b5e719bf4d0364f139ac5002bf4572fe55b37dac823925` |
| Manifest weight payload | `053442c31c032b800501a8633999a3285bb87c5e4b6f2eea146fc905678d9a11` |
| Tensor content | `aa6c466ddaee33f3775e674fd0f489fd48d12cffd10b620d3e74ec5b8719edf2` |
| Evaluator model identity | `ea1d7fa166f4178e08d894613ff75a9ba3e87c93cfd85814361a7b83ea2bc173` |
| 136 finite Adam tensors | `f4402d071c11273d4c12cb482904b5fc51373a7ad4c4ffa4b90cb562c643df64` |
| Final training state | `7681f5e97b9988bfdb0566e714350e7a932e60076cd0a0eea609253850f37f0b` |
| Final decision | `7f09b2683a7941fd282e61fe6a63e99cac34fcb9b5df7d7b8458d80a09563626` |
| Final terminal | `f3e2baf9b15e84074b57fc033be7ba4c658bfdea60b8672d0f046e85c415ba90` |

The study is `artifacts/citation-precision-20260922-study-final/`; final native is
`ANSWER-MEAN/segment-0003/final`. It is Finished /
CANDIDATE_FIXED_AT_11264 / resume=false. Clock and sampler are11264, cumulative
input13,287,424 and target594,944. ANSWER family6/normalizer2/first-target1, QE,
the tokenizer and architecture remain unchanged. The historical10496 failure,
its native and Adam, scalar4352 acceptance and prior A/B reports remain preserved.

## Actual execution and independent checks

Exactly768 updates ran at LR3e-5, bits4539475662290099561. Every trace draw matches
the registered first768 citation draws at the correct absolute cursor. Each batch
is V4/VC0 2/VC1 2 with8 answer-mean terms and76 target tokens. Per-example losses,
ANSWER reduction, objective values and usage denominators were independently
recomputed. EOS belongs to targets; prompt and padding do not.

Actual input915456, target58368, padding24576; samples6144. V rows each occur2
times (3072 exposures), VC0/VC1 each1 time (1536 each). No discarded optimizer
work or unaccounted training row was found. The remaining768 registered updates
were not executed after the early joint pass.

| Segment | New updates | Saved step | Final phase |
|---|---:|---:|---|
|0000|1|10497|TrainingPending|
|0001|255|10752|TrainingPending|
|0002|512|11264|EvaluationPending / TIME_BUDGET|
|0003|0|11264|Finished / CANDIDATE_FIXED_AT_11264|

The final native physical bytes, tensors, Adam and state are identical across
segments0002 and0003. Segment0002 generated4475 rows and completed4474 teacher
observations; the new process completed the remaining1797/1798 with optimizer0.
No finished row was regenerated. The900-second cooperative stop was followed by
safe preservation; segment0002's recorded elapsed time is901.126545583s.
Cleanup is part of the existing bounded path, not an extra training budget.

The relative-root command initially failed with `continuation immutable
parent/policy` before admission. Its retained log is `segment-0000.log`; the
registered absolute-root command used a separate log. It created no segment,
optimizer, generation or teacher evidence and is not counted as learning.

The independent reader checked all6464 generation rows and6464 teacher rows,
frozen IDs/order/content/expected answers, prompt digest, checkpoint/model/step,
raw tokens and strict decoding, actual EOS and error status. All12,928
prepared/resolved pairs are complete, attempt0, with no missing/pending/UNKNOWN
record. Stored exact flags, summaries, guard chain, dev/fit gates and final action
agree with independent recalculation. Runtime errors0. The recorded guard streak
remains0. Before/after hashes of25,999 existing study files and the27 consumed
protected input paths match; fresh reproduction adds only its own observation
files. No whole historical archive or closed prior review was re-executed.

## Same-checkpoint results

FULL is strict complete answer equality; QB/SB are query/swap pairs. Citation
support means the selected event, not merely an ID present in the evidence.
All rows below reached EOS and runtime errors0.

| Step / panel | FULL | QB | SB | ALL4 | Correct value | Correct support | Outside ID |
|---|---:|---:|---:|---:|---:|---:|---:|
|10752 V64|64/64|32/32|32/32|16/16|64|—|—|
|10752 VC64|64/64|32/32|32/32|16/16|64|64|0|
|10752 renamed64|64/64|32/32|32/32|16/16|64|64|0|
|11264 V512|512/512|256/256|256/256|128/128|512|—|—|
|11264 VC512|512/512|256/256|256/256|128/128|512|512|0|
|11264 renamed512|512/512|256/256|256/256|128/128|512|512|0|
|11264 train probe128|128/128|64/64|64/64|32/32|128|128|0|
|11264 old512 train|512/512|256/256|256/256|128/128|512|—|—|
|11264 new1024 train|1024/1024|512/512|512/512|256/256|1024|—|—|
|11264 VC0 train1536|1536/1536|768/768|768/768|384/384|1536|1536|0|
|11264 VC1 train1536|1536/1536|768/768|768/768|384/384|1536|1536|0|

Development and full fit are both PASS on11264. Combined VC train is3072/3072,
QB1536/1536 and ALL4768/768. Development ID_BOTH512/512; unchanged cited ID across
renaming0. Against parent10496 with identical cases, V gains2/loses0 and ALL4
gains2/loses0; VC gains2/loses0 and ALL4 gains1/loses0; renamed512 is unchanged.
These repeated development results do not establish an isolated causal LR effect:
additional exposure also occurred, and no simultaneous high-LR control was run.

Existing saved teacher observations were re-aggregated with zero new teacher
calls. They remain gold-prefix diagnostics, separate from free generation:

| Final panel | First-value NLL | Gold-prefix EOS NLL | After-value EOS minus citation logit | Grammar-prefix NLL | ID-token NLL |
|---|---:|---:|---:|---:|---:|
|VC512|0.000549505819|0.000004355320|-16.3442424664|0.000014170370|0.001142767701|
|Renamed512|0.000565064672|0.000004308987|-16.2634458649|0.000014275102|0.000610347561|

Each panel has3072 grammar tokens and4096 ID tokens; each of8 ID positions has
512 observations. The position NLL arrays are preserved in `B-recount.r3b` and
its log. No token metric replaces strict FULL, joint accuracy or zero outside ID.

## Fresh reproduction and usage

The frozen production executable ran these existing commands in fresh processes,
one at a time, with compute threads1 and the existing900s/12GiB RunControl:

```text
fresh answer-mean-review --root <absolute study-final>/ANSWER-MEAN
fresh answer-mean-review --root <absolute study-final>/ANSWER-MEAN --citation
```

Both completed with matched16, generation16, teacher0, optimizer0. Native tokens,
bytes, actual answer, EOS/finish, errors and prompt identity match the original
raw. Final dev has no wrong rows, so the registered selection rule correctly
chooses indices0..15 for both panels. There are no omitted final errors to
reproduce. This sample is a parity check, not a second quality estimate.

Parent parity evidence was read rather than re-executed: its error-first orbits
include all four original wrong rows. Both parent samples have14/16 correct
answers but16/16 parity, preserving wrong outputs exactly. Expected answers and
scoring/orbit selection remain outside the generator's request-only native path.
The B continuation reused the completed value receipt and made no duplicate call.

| Recorded cost | Optimizer | Generation | Teacher | Generated tokens |
|---|---:|---:|---:|---:|
|Training/evaluation segments|768|6464|6464|78208|
|Implementer parent parity|0|32|0|304|
|Independent B parity|0|32|0|304|
|Total before confirmation|768|6528|6464|78816|

Segment elapsed sum1412.590066334s; parent parity3.116021333s; B parity3.000510375s;
recorded active total1418.706598042s. Reader/build/report wall time is excluded.
Previously established TINY contract usage stays58/420/280, FD0; B added none.

Local execution evidence is under `artifacts/citation-precision-20260922-review/`:
`read_B_precision.rs`, `B-recount.log`, `read_B_parity.rs`, `B-parity.log`,
`B-value.stdout/stderr`, `B-citation.stdout/stderr`, and `B-report.stdout/stderr`.
Readers and native commands exited0; reader compilation retains unused helper
warnings from reuse. The one missing-comparison inspection produced a read-only
NotFound diagnostic, not a model failure. Source verification confirmed that
`comparison()` returns a pure value; no canonical comparison file is required or
created. The existing report command read the results successfully, and the B
receipt uses that exact endpoint value without changing original results.

- `B-recount.r3b`: `b3b81eb7e43f4195f7fef4615123f56c79c434ced5a89bc4b41eb6eafa6c9558`.
- `B-parity-usage.r3b`: `f51b625a87e821f325c546011e28a7fd3920aec38660e824bb9e8d9ce92ff0f7`.
- Fresh value raw: `4370f369f3069685cfff70b1999cc5507be46d809fe46a7ef09ed435c8262211`.
- Fresh citation raw: `17231824177f386433064f74504a50ad24d585aaaf63c4d406662dbf68079999`.

No confirmed issue remains in B's actual integrity/reproduction scope. The same
unused citation seal remains bound across its seven registered owner/link roots;
its body has not been opened. The existing publisher binds this report to the
same preparation and final comparison endpoints. After that receipt, the
implementer may execute the already-authorized single same-candidate citation
confirmation. Only its actual result can accept the limited value/citation
scope. Further learning and S4/S5/S6/Goal1 acceptance are not authorized by B.

Only this report is published by the reviewer; report commit and remote identity
are separate from the reviewed product source. The repository's lean-development
skill was applied to reuse completed work without omitting required evidence.
