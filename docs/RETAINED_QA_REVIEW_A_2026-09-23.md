# Retained QA transfer — independent preparation review A

Date: 2026-09-23. Contract: R3-RETAINED-QA-TRANSFER-1.0.

**PREPARATION_REVIEW_A: PASS.** This accepts the bound curriculum and execution
preconditions, not its unmeasured SMALL quality. No SMALL optimizer, generation or
teacher calls were made by this review. S4/S5/S6 and Goal1 remain unaccepted.
Product source, original artifacts and closed experiment decisions were read-only.

Reviewed source: `3a5f54ed91dffd8a5782e148c56a56dddf2244c8`.
Its product/source diff against the reviewed worktree was empty at execution.
The source snapshot is local `artifacts/retained-qa-20260923-review/A-source`.
This report is published in a separate report-only commit; that commit is not the
reviewed source commit.

## Identities and preparation

| Artifact | SHA-256 |
|---|---|
| Plan source digest | `7199cdbe4ebc0c018adf0ba70c3b1b1c874adc5a5411aa9b492bea3d2a721ca4` |
| Production executable | `a833ff1919fb166d7a493e60c1f4c34f9697a7e82ed34ba80853e72222c357d7` |
| Preparation | `6d06c25f4abcb1f7aff5b0b4b79ecf138d741244127ee722f753a36b57d265a2` |
| Selection | `5aef761f98041138ad6479eb26bcd72900e0a76f4bee50cac45415cf7a680f53` |
| Child plan | `745702185996f5a7c450bf84ca0a7041f158deae08af593b50d9368f35c1d6ac` |
| Child corpus | `64c0fa67ab256081095c20717b521a00843f70149cb8315be96671dffba3117d` |
| Full tape | `e644ac1df9b40ed64cdff74a4854e3ad62f8a5e19ce59ad08f86ccc9b0752665` |
| Parent11264 / child initial native | `c47e34c7ac4f88dd08b5e719bf4d0364f139ac5002bf4572fe55b37dac823925` |
| Parent136 Adam tensors | `f4402d071c11273d4c12cb482904b5fc51373a7ad4c4ffa4b90cb562c643df64` |
| Parent training state | `7681f5e97b9988bfdb0566e714350e7a932e60076cd0a0eea609253850f37f0b` |
| Bound prior independent report | `3a9d5ca14f945723de47e9ef6ea6c1d41882d3e4681c6ce16cefa9661d2fd770` |

Actual preparation is
`artifacts/retained-qa-20260923-study-final`; the model/plan/corpus are under its
`ANSWER-MEAN` directory. The executable is
`artifacts/retained-qa-20260923-executable`.

The independent Rust reader loaded the actual native parent with Adam, checked
all136 finite optimizer tensors and clock/sampler11264, and compared the exact
child-initial bytes. ANSWER objective family6/normalizer2/first-target1, QE framing,
tokenizer, architecture, optimizer parameters, batch8/accumulation1 and LR3e-5 are
retained. The fork starts at11264 and stops at15360; the old parent remains
Finished/CANDIDATE_FIXED_AT_11264/resume=false.

The accepted balanced source is the existing LOCAL5 data under
`artifacts/identifiable-baseline-20260921-r3`. Its8192 train rows,512 primary and128
transfer are separate from old QA512/128. The existing source acceptance was
reused; no old experiment was rerun or reconstructed. The prepared4608 retention
rows and1536 retention dev rows were compared against the original parent corpus.

All8192 Q0 rows equal their balanced source. All8192 Q1 rows preserve their
non-question content, family, base/view, target and generation limits, have unique
IDs/source references, and use the intended two train phrases selected by the
registered base/intent hash. The4096-update tape was independently read in full:
V2+VC2+two complete QA pairs, alternating VC0/VC1, AB/CD/EF/GH bucket rotation,
all512 distinct bases per bucket in the first epoch, and the same base order in
the Q1 epoch. Q0 and Q1 each receive exactly one exposure per row.

| Pool | Exposures | Input tokens | Target tokens |
|---|---:|---:|---:|
| V | 8192 | 1187840 | 16384 |
| VC0 | 4096 | 626688 | 69632 |
| VC1 | 4096 | 626688 | 69632 |
| Q0 | 8192 | 1720935 | 149526 |
| Q1 | 8192 | 1844427 | 149526 |
| Total | 32768 | 6006578 | 454700 |

Actual planned padding is1574558. Retention per-row exposure is V5–6 and VC2–3;
it is not uniform. All20992 sequences were tokenized: maximum length281,
excluded evidence0, train/generation prompt equality, prompt/padding exclusion
and EOS inclusion. The input/target caps remain18M/2.4M. Full prepared-prompt,
family and nonempty-scene overlap against retention and both fixed QA evaluation
sources was0. Shared vocabulary and answer grammar are not presented as leakage.
Twenty bound source/native/metadata/report/seal files had unchanged hashes after
this readback.

## Direct evidence and review findings

Two SOURCE_READ findings were fixed before this candidate was frozen:

1. QA reporting now separates G subtype/unresolved/resolved/both, H fixed-sentence
   emissions including emissions on non-H inputs, and old QA value/order/wording
   view-pair results. Old four-view ALL4 and the actual changed-input denominators
   are explicit; balanced two-view ALL4 remains undefined.
2. The new full QA and conditional-fit path has a typed raw writer→reader→audit→
   decision regression, covering missing mandatory panels, failed fit, bucket/pair
   gates, EOS/outside IDs, early candidate fixing and final extend=false.

The final-source implementation quick receipt at
`artifacts/retained-qa-20260923-quick-03` was checked, not re-executed by this
review: three commands, each one test and exit0, source_unchanged=true.
Its tests were `retained_qa_tape_input_objective`, `retained_qa_full_raw_gate`, and
`token_cache_actual_native_framing_separates_same_tokenizer`. They cover mixed
ANSWER gradients8 versus4+4, real masks, exact full2944+conditional4608 typed
panels, and native framing/cache rejection. Synthetic correct raw is a test
oracle, not generated quality evidence; the typed full-panel test has model
calls0. Existing unchanged numerical/timeout/confirmation acceptance was reused.

The independent reviewer **executed** exactly one final-source native process
test. The already-completed TINY ancestry under
`artifacts/retained-qa-20260923-tiny-01` was read-only; it was not retrained.
The final-source test binary was copied to independent `A-bin`, hash-checked
before and after, and executed in fresh subprocesses/new `A-fixture` and TMPDIR:

```text
R3_QA_PREVIOUS_FIXTURE=<repo>/artifacts/retained-qa-20260923-tiny-01
R3_QA_TEST_ROOT=<repo>/artifacts/retained-qa-20260923-review/A-fixture
TMPDIR=<repo>/artifacts/retained-qa-20260923-review/A-tmp
VECLIB_MAXIMUM_THREADS=1 OMP_NUM_THREADS=1
<A-bin>/replica-train-tests --exact \
  training::fresh::identifiable::binding::citation::tests::retained_qa_native_process \
  --nocapture
```

Result: **1 passed,0 failed,exit0,133.56s wall time**. Test binary SHA:
`5085c33979bdb7d0fc72559d42827e7389583cfabba428257bb5eae6534a2137`.
This was independent execution of the frozen compiled binary, not an independent
full Cargo rebuild. The underlying build used locked/offline dependencies and
Accelerate/test-support. The independent preparation reader used the current
Accelerate production rlib instead of rebuilding unchanged model code.

The new two-update endpoint equals the preserved continuous endpoint in model
tensors, Adam, clock/sampler, consumed input/target and raw outputs. A durable QA
row followed by TIME_BUDGET retained EvaluationPending; a new process finished
the remaining evaluation with optimizer0 and an unchanged raw prefix. Wrong
length/LR policies were rejected. The normal quality-fail endpoint permitted
read-only retention/QA reproduction and rejected training/confirmation; its
final decision was extend=false/resume=false. This EOS TINY fixture proves path
and resume behavior, not SMALL quality. The actual process uses reduced screen
panels; the full-panel obligations are separately exercised by the typed test.

Independent canonical readback matched every prepared/resolved call and raw row
with control/terminal counts and update traces:

| Execution | TINY optimizer | Generation | Teacher |
|---|---:|---:|---:|
| Implementation original ancestry/process | 22 | 400 | 236 |
| Implementation final-source reuse/process | 2 | 124 | 44 |
| Independent A reuse/process | 2 | 124 | 44 |
| Contract total so far | 26 | 648 | 324 |

UNKNOWN/pending0; finite-difference0; SMALL calls0. Independent process input2898,
target277,padding718,raw generated tokens124. Its recorded segment/observation
active times were5.811601791/3.388138625s, distinct from total test wall time.
Remaining TINY caps are70 updates/120 generation/444 teacher, not automatic
authorization to consume them.

One reader setup failure is preserved: reusing an older cached A-target rlib
rejected the parent as OBJECTIVE_POLICY_BINDING_MISMATCH before any model calls.
Re-linking the unchanged reader to the current production rlib passed. No native
validator was weakened, checkpoint rewritten or model retried. Production rlib
`libreplica_v3-5f74ea5b6a70fcf8.rlib` SHA:
`a9126c1a260edacd3a54ab89d7341366649ad1ca479e5df21c8da5699d0b8df0`.

## Independent S4 preparation

S4 was initially NOT_CREATED. The independently prepared current fixture is
existing typed R3BIN Holdoutv2 under
`artifacts/retained-qa-20260923-review/S4-sealed/derived-time-repair-01`.
It has exactly200 cases, five categories×40 and three predeclared balanced
control subsets40 each. It is SEALED and **NOT_OPENED_FOR_MODEL**.

| Current artifact | SHA-256 |
|---|---|
| Seal manifest | `573a155ff785d2a6e057427884ddbeb16bb390ab9e9a9d3800901d48829671ca` |
| final200.r3b | `fdb26c93974c657982e0e59bb334227b0d1fe50592f3c1222270051e320e886a` |
| controls.r3b | `eed036a0d1ca5ee0d0d32d39318b8cda7207c3a641d9246ce9503416728c0720` |

One generated scene set/seed was retained throughout preparation. Initial ID
overlap, an exhausted same-digit mapping attempt, a rejected empty-train R3CORP
wrapper and a discovered timestamp shortcut were preserved as failed preparation
evidence. Narrow deterministic ID remapping and timestamp permutation corrected
those newly prepared fixtures before model opening, without resampling scenes,
changing answers, model calls or overwriting the failed files. The current
independent label and inverse-content checks passed. Full prompt, scene, event-ID,
digit-masked wording and family overlap checks against the fixed six source
corpora passed. The old accepted confirmation was used only as a read-only
overlap source, never as new training or candidate selection data.

Limits are explicit:40 no-value cases have no value to swap; the value-swap
control subset contains32 changed-value and8 no-value cases. Twenty single-record
cases have trivial record selection, reported separately. The nuisance-rule
audit is not proof of absence of every possible shortcut. Current seal hashes
and both canonical files were verified again during A; answers were not exposed
to the model or training agent. Preparation details/failure artifacts stay local.

## Local evidence and authorization boundary

Independent evidence root: `artifacts/retained-qa-20260923-review`.

| Evidence | SHA-256 |
|---|---|
| A-process.stdout | `7904b5b5116af1276ea2de1be951ce74ad472144a5b4e33d715a58ee78118d87` |
| A-preparation-audit.r3b | `eadb4da007c020d414ba996211395df8b6c81c321ed3966d06c3a25c2a512442` |
| A-actual-usage.r3b | `56f17fb5c12f45b9cb9970e83f514ef63e062ad5062b83eb6d1942c2adfe0dad` |
| A-protected-manifest.r3b | `b58514e367ef6ae16fa27f4f7fe6b93d7e14ca083d9b64314a8ce12d66caad9c` |
| read_A_retained.rs | `4c8e40cc51ddd5ca6f832c84ef7b8476b9a0bf4e696a7f06d299c0a55d199352` |
| read_A_retained_release | `cbca3ab1bcb4385888223f9517e3d6a3d1ef230bc3e649c7c623bef802279292` |
| read_A_usage | `0db5556ae5181790989eb0590369ed2c87959a267beaf7e046c526b6f3c1825e` |

The remaining registered work is the parent32 parity plus balanced640 baseline,
the bounded child learning/evaluation, and independent endpoint B. Independent
S4 may open only after its fixed candidate gates pass; S5/S6 remain conditional.
A permits that existing plan to proceed. It grants no quality pass, extra budget,
new experiment, model promotion or Goal1 acceptance.
