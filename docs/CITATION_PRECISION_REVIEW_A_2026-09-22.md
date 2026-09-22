# Citation precision — independent preparation review A

Date: 2026-09-22. Contract: R3-CITATION-PRECISION-1.0.

**ACTUAL_PREPARATION_PREREVIEW: PASS.** This accepts the bounded preparation and
changed execution boundaries for the single precision fork. It does not accept
model quality, confirmation, S4/S5/S6 or Goal1. New SMALL optimizer, generation
and teacher calls during this review: **0 / 0 / 0**.

Reviewed final source: `3290fec66db372c5ab427bf2e632bf573824ce3d`.
The three substantial regressions ran on the preceding frozen source
`368254091a56e7a6f00c01ee2e68d9caef3e15df`; after a confirmed token-cap omission,
the final delta received its own independent exact-source regression. This is
not described as a full quick run on the final revision. The earlier preparation
was preserved and received no approval. Approval applies only to the final study.

Product source/tests/Cargo and original artifacts were read-only for the reviewer.
Source copies, build targets, fixtures and Rust readers were isolated under
`artifacts/citation-precision-20260922-review/`. The only published reviewer file
is this report. The final study's explicit approval receipt is local evidence.

## Source and execution identities

| Item | SHA256 or Git identity |
|---|---|
| Final reviewed source | `3290fec66db372c5ab427bf2e632bf573824ce3d` |
| Training source digest | `121483eb645720cfa8369693ae25fe05bf54782773fc9a3e508dd793fdb244c3` |
| Frozen production executable | `0020e08670d6e66de5b4c2759fac21f012f2bd610c98474580af32fee525132f` |
| Candidate diff from report HEAD88c7c181 | `e5fdeca775adfd8c8296b95f48be9a334d78ba86ed751b5dd067ed03527cee3d` |
| Three-test source | `368254091a56e7a6f00c01ee2e68d9caef3e15df` |
| Three-test training source digest | `704b745d307c05562cce02b7e9f5a94668787859eebebd9911561da59e85c0e0` |
| Three-test checker source digest | `a95e0eb36b6bb644f31754e67d58cffa83beb80b18f45e3dae901b1aaff2ed2d` |
| Three-test executable | `fbf467bf4dcebc4e527feb566969f9da8e8a03aa4490745bad8842a878114623` |
| Checker executable | `e2cfe90ad48311ebe0ba303472715b63e6853520c0a508c76f7039d986c51155` |
| Final cap-test executable | `83682e4cc53831263998992870a3d0d15a1f69f0baf0fd1c65d53dddffe2ee0c` |
| Final source-only delta from368254 | `2f705433dabefe064bf447d62c1327f68282b588ef1d5c6a7ca5527c115dd4a2` |

The production binary is `artifacts/citation-precision-20260922-final-executable`.
The final delta changes only the precision branch in `binding::remaining`, its
direct zero-call regression, and implementation status. Unchanged execution
tests were not repeated to consume the shared TINY budget. The original test
binary and checker were preserved separately before the final cap build.

## Actual independent commands and outcomes

Installed Rust/Cargo1.98.1, locked/offline, Accelerate/F32, one compute thread.
`CARGO_TARGET_DIR`, `TMPDIR` and `R3_CONT_TEST_ROOT` pointed into reviewer scratch.
No dependency/toolchain/model installation or external inference was performed.

From the exact368254 `A-source` copy:

```text
cargo build --locked --offline --release --features accelerate --bin replica-check
replica-check --output <review>/A-quick quick --citation-precision
```

The checker actually ran:

```text
cargo test --locked --offline --features accelerate,test-support --release --bin replica-train citation_precision_ -- --nocapture --test-threads=1
```

Executed3, passed3, failed0, exit0; test runtime481.46s, cargo command including
compilation516.001364583s. Checker confirmed its source remained unchanged.
The three fully qualified tests were:

- `training::fresh::identifiable::binding::citation::tests::citation_precision_process`
- `training::fresh::identifiable::binding::citation::tests::citation_precision_tape_schedule_and_samples`
- `training::tests::citation_precision_adam_lr_f64_reference`

From the final3290fec `A-final-source` copy:

```text
cargo test --locked --offline --features accelerate,test-support --release --bin replica-train training::fresh::identifiable::binding::citation::tests::citation_precision_executed_token_caps -- --exact --nocapture --test-threads=1
```

Executed1, passed1, failed0, exit0; test2.89s, compile32.49s. All model calls0.
There were no zero-test passes or independent whole-suite retries.

Actual evidence, relative to the reviewer root:

- `A-quick/summary.r3b`: `c860783bcdda7e72479c3a891b6a560d9eab52ab6938d4a585a2eb922fa413c6`.
- `A-quick/command-00.stdout`: `fc8f3b116769725833850afcd5c45526c75850b5e0c885ee96b328c21fa046f2`.
- `A-cap-test.stdout`: `e199c5bca19bb6edd9fc9b5d8daf1a1185d1e3c60787e8c7b7afc88fc87efe31`.
- `A-actual-usage.r3b`: `c6583fd88e61743fb75ecaaa638883d526c0fa69c31a821e636022dbada7fd6b`.
- `A-preparation-audit.r3b`: `2bfa846a63f7622c588b4b48a5296ecec2b6810c506106881086b833c7f9a463`.
- `A-protected-manifest.r3b`: `5cf08e3af05e67947e2715e91a385d5c5b71f4f8184079c2ee105c3095cdd579`.

## Boundary and numerical findings

The implementation initially generated non-prefix reviewer samples but the
confirmation consumer still expected prefix16. The reviewer identified the
source mismatch before this execution. A common `endpoint_reproduction` now
supplies both generation and receipt consumption. The actual typed-receipt test
rejects prefix consumption, accepts the bound non-prefix sample, and rejects
changed expected content. `verify_review_b` accepts the same model/policy/report
and rejects changed model, policy or report. These are explicit synthetic
zero-model records, not measured SMALL quality.

During A, the implementer identified a second real omission: precision inherited
the older4,000,000 input/260,000 target execution caps. The reviewer independently
confirmed it in source. `caps-before.log` records the implementer's actual failing
regression (exit101,4,000,000 versus2,000,000); `caps-after.log` records its pass.
The final independent cap test exercised the actual `remaining_input` and
`remaining_targets` path with committed0 but known executed work: zero,
1,192/76 uncommitted tokens, exact2,000,000/130,000, each over-limit, and UNKNOWN.
The final precision-only branch enforces2,000,000/130,000; old profiles retain
their prior caps. UNKNOWN is rejected rather than counted as zero.

The actual random TINY Adam check used identical nonzero moments and gradients
at cumulative clock10497. All27,136 coordinates had inherited nonzero moments.
Updated moments match across the two LRs; an independent F64 reference includes
clipping, bias correction and LR-scaled weight decay. Maximum absolute delta
error was1.1578788545701219e-7 within predeclared absolute/relative F32 bounds.
The first theoretical delta scales by1/10; this proves neither a trajectory
ratio nor better model quality.

Actual process tests established random2 versus new-process1+1 weights/Adam/state
identity. EOS fixtures separately exercised complete continuous2, split1+1 and
evaluation-only2+0 paths. The random seed's real control-token generation failure
remains a failure: NOT_INVOKED timeout then a new process returns the same failed
row, and an already RETURNED ordinal cannot be regenerated. No output was forced
to make that negative case pass. At teacher12/12, a durable final teacher row
followed by time stop resumed in a fresh process with optimizer/generation/
teacher0 and reached Finished. Generic resume with same/different LR is rejected
through an actual child process. Normal native inference loading remains usable.

Typed raw/teacher/native fixtures traversed `panels`, `read_score`, `audit_panel`
and `evaluation_result`: dev pass with missing fit is rejected; dev+fit pass fixes
11264; fit failure permits only bounded continuation; final failure at12032 has
extend=false. These fixtures are not substitutes for later full model evaluation.
The new profile's schedule, save-only11776, every new-step LR bit pattern,
max-step refusal, same seal ownership and malformed lineage refusal were checked.

## Actual final preparation and parent

Approved local study: `artifacts/citation-precision-20260922-study-final/`.
Preparation SHA256:
`798ab26a728c374a93271c5a51be3a236acffb1cdf596e39d29a938205a50120`.
Policy physical SHA256:
`08254fdc34e17692910f2d04f30b8ca276cd847573fc293b64697da96ccddf0c`.
Tape content SHA256:
`d23d41239a5e57bf5f30cf17af2e373a902fc72d76183da9d6dcca32ebb3cd78`.

The reader used existing binary/native readers and tokenizer preparation. It
performed no generation, teacher or optimizer calls. The original consumed17
protected files, preserved earlier preparation5 and final preparation5 retained
their hashes (27 paths). The whole historical archive was not redundantly
rehashed. Sealed case bodies were not decoded.

Parent `citation-fidelity-20260922-study/ANSWER-MEAN/segment-0008/final` is exactly
the new initial file, physical SHA256
`7845eb2e66bf333f07a4fde28d80601f418f212912c7c7b58fc41424646b5ff6`.
The parent remains Finished / FINAL_QUALITY_FAIL_AT_10496 / resume=false.

| Parent identity | Verified value |
|---|---|
| Step / sampler | 10496 /10496 |
| Cumulative input /target | 12,371,968 /536,576 |
| Native objective | family6, normalizer2, first-target1, QE |
| Manifest weight payload | `2ae7311273497a45adccfe609e10091ef302421e5fa963146dac9561e38098a4` |
| Evaluator identity | `df3649f9b64f1cf315af1ec86fcc12bd52cc6e80d68e0c0f6ff899c690641a8e` |
| Tensor content | `c80af6c77b83b5e135930e1621ec4e7cb85ad376378ff83f87a02e877b480f17` |
| 136 finite Adam tensors | `542468bdda23af463502e59c4b96e6b6ee31a33f40673742cd06710b67f96438` |
| Training state | `b5e4af0761727e2587b581cf84194885759d6a4c67dd6b364d8da55a6f462972` |
| Bound parent B report | `31068df393e3e35938501ae1700f61ed866a1bac7de00c7171572e01a182ce38` |

Original corpus4608, dev1536, metadata, tokenizer, architecture and fixed optimizer
coefficients remain byte/value identical. Parent LR bits4554169646866313825
(3e-4) deliberately become child4539475662290099561 (3e-5); the initial native
itself retains its historical parent binding. First child storage binds the new
policy. The immutable tape prefix10496 is unchanged; the appended1536 draws are
exactly original citation indices0..1535 (original absolute4352..5887).

Every actual train answer mask excludes prompt/padding and includes EOS. Value
rows have2 target tokens; citation rows17. Each batch has V4/VC0 2/VC1 2,
input1192, target76 and padding32. ANSWER loss averages8 answers, independently
of the76-token usage denominator.

| Prepared interval | Updates | Samples | Input | Target | Padding | Per-row V/VC0/VC1 exposures |
|---|---:|---:|---:|---:|---:|---|
|10496→11264|768|6144|915456|58368|24576|2 /1 /1|
|10496→12032|1536|12288|1830912|116736|49152|4 /2 /2|

These are independently computed **planned costs**, not executed SMALL updates.
Both train/development inputs stay unchanged; no dev error ID enters the tape.
The permitted schedule and limits match the plan:10752 screen;11264/12032 full
panels; conditional fit4608;1536 updates;2m/130k input/target; generation14336,
teacher13120; active7200s, command900s, cleanup120s; training16GiB/inference12GiB.

Existing parent quality is reused evidence, not a new experiment: V510/512,
VC510/512 with outside2, renamed512/512. The current raw error memberships and
labels agree with the frozen inputs. The future parity selections are fixed
four-view orbits: V indices20..23,100..103,0..3,4..7; VC476..479,0..3,4..7,8..11.
These include the known wrong rows23/100 and477/479. This read-only selection
check generated nothing and does not estimate model quality.

Six registered linked studies plus the original citation owner remain bound to
the same unused seal
`dc99a793cdc79f448a8a159bbff436e110f58887600a35ff622ea36aec4ca3d0`.
Every link's preparation/status matched; no candidate, claim, attempt, raw or
result execution record was present under those owners. Scalar4352 acceptance
and its separate used confirmation remain unchanged. Citation confirmation is
NOT_OPENED; no receipt was reconstructed to manufacture historical completeness.

## Actual usage and conclusion

| Execution | TINY optimizer | Generation | Teacher | FD |
|---|---:|---:|---:|---:|
| Implementer direct process + numerical check |29|210|140|0|
| Independent frozen process + numerical check |29|210|140|0|
| Cap RED/GREEN/independent and pure readers |0|0|0|0|
| **Contract total** |**58/64**|**420/1200**|**280/1024**|**0/16**|

The reader independently reconciled22 control-file updates plus4 random updates
and3 numerical updates per main execution. All210 generation and140 teacher raw
rows per process matched prepared/resolved call records; one confirmed NOT_INVOKED
attempt per process, UNKNOWN0, pending0. Actual generation tokens were210 each.
Each process's tracked trainer input26096/target1432/padding576 is separate from
the random/numerical diagnostics. Their standalone optimizer elapsed time was
not separately recorded; it is not reported as zero. Independent known segment
time17.383023583s, observation3.320048291s and random-evaluation0.134613209s are
not presented as full training performance or total wall time.

Retained implementer helper-compile and synthetic-native construction failures
had no model calls; the production validators were not weakened. Old quick
results are labeled with their actual source, and existing whole fmt/clippy
debt remains documented by the implementer rather than hidden by this review.

No confirmed unresolved issue remains in this changed preparation scope.
INDEPENDENT_A=PASS; PARENT10496_PRESERVED=true; BASELINE4352_PRESERVED=true;
TRAINING_EXECUTED=NOT_RUN by this reviewer; LAST_DURABLE_STEP=parent10496;
DEV_JOINT/FIT/B for the new model=NOT_RUN; CANDIDATE_FIXED=false;
CONFIRMATION=NOT_OPENED; VALUE_CITATION_SCOPE_ACCEPTED=false;
QA_GAP/S4/S5/S6=NOT_RUN; GOAL1_READY=false; GOAL1_ACCEPTED=false.

After the bound approval, the implementer may perform the authorized parent32
parity and the single bounded precision fork. No second LR/seed, extra training,
confirmation or model-quality approval is granted by this preparation review.
Report-only commit/remote identity is recorded separately from reviewed source.
