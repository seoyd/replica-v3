# Independent A2 — retained instruction bridge

**A2: PASS.** This accepts the prepared, bounded bridge study and its directly
changed execution paths. New SMALL training and bridge quality are **NOT_RUN** at
this review. Protected11264 acceptance remains preserved; S4/S5/S6 and Goal1 are
not accepted by this review. A1 remains closed under its separate report.

Reviewed source: `8522d026512cfb7b6e4b6c32142c5d68b77c9f2f`.
Source digest: `9b46c11e60cf4ecbd44a77d28c82f70b59dc94895e5a2a81149c2f0576c500b4`.
The reviewer changed no product source, tests, Cargo files or original artifacts.
This report's commit is separate from the reviewed source.

| Bound artifact | SHA256 |
|---|---|
| Production `artifacts/qa-integrity-bridge-20260923-implementation/executable-bridge` | `341f018c30aa57feaf6ae1a8e50ddadedb1c91492a998da0f76bdd12b2ea6394` |
| Source-bound test binary, independently copied and executed | `7f877e55d6808915037e90c8b1a45f8f8c38f5bb4c820e47a10ca1820bcb1489` |
| Study `preparation.r3b` | `7c8fc48fef59c47ab065e0f563eff300040a41344d89ce5823d2648acea1c378` |
| Study `selection.r3b` | `0277faab9674a59c02d3f5a7cb52c52f7c4a74c8ef24579e7a03ecd0a83091f0` |
| `ANSWER-MEAN/plan.r3b` | `13d292570d17b39c7e69b502a29cc2dfeacf0c41bb93dbae672349c9b864c261` |
| `ANSWER-MEAN/corpus.r3cor` | `0e5728231f4f7f9fb8d3971f2df926067a97a6ec89434d0973bb47ec69e34879` |
| Protected11264 and exact copied initial native | `c47e34c7ac4f88dd08b5e719bf4d0364f139ac5002bf4572fe55b37dac823925` |

Study root: `artifacts/qa-integrity-bridge-20260923-study-final/`.
Independent evidence root: `artifacts/qa-integrity-bridge-20260923-review/`.
The latter contains `A2-source`, `A2-bin`, `A2-tmp`, `A2-process`, logs and Rust
readers. These local artifacts and raw contents are not published in Git.

**Executed independently.** The implementation's newly built locked/offline,
release, Accelerate/test-support test binary was copied and hash-checked. These
are independent executions of that exact binary, not a claim of a second Cargo
build. Each filter executed one test; zero-test results were not accepted.

| Exact test suffix under `training::fresh::identifiable::binding::citation::tests` | Result | Wall time | New optimizer / generation / teacher |
|---|---|---|---|
| `qa_bridge_raw_conditional_fit` | 1 PASS, exit0 | 102.91s | 0 / 0 / 0 |
| `qa_bridge_native_process` | 1 PASS, exit0 | 356.30s | TINY6 / 72 / 72 |

Commands used the copied `A2-bin/replica-train-tests`, `--exact`, `--nocapture`,
`--test-threads=1`; the process test additionally used `--ignored`.
`TMPDIR` pointed to `A2-tmp`, compute threads were1, and the process root was a
new `A2-process`. The explicit preserved TINY parent was
`artifacts/retained-qa-20260923-tiny-01/precision/ANSWER-MEAN`.
No old ancestry training was repeated.

The raw fixture uses the existing typed writer, audit and real bridge decision:
six512-row panels, absent conditional fit rejected,13 wrong fit rows cause
`BRIDGE_FIT_NOT_MET`, perfect fit fixes12032, and a severe first64 guard prevents
fit even when whole-panel FULL488 passes. These are synthetic boundary fixtures,
not model quality evidence.

The native test compares continuous2, new-process1+1 and evaluation-only2+0.
Weights, Adam, clock and output tokens/text/finish/error agree. The evaluation-only
fault first saved step12 with18 generation/17 teacher calls and TIME_BUDGET;
the new process finished the remaining6/7 with optimizer0, preserving the returned
prefix. Wrong parent/LR/sequence policy and closed-run reentry were rejected;
the complete negative endpoint remained reviewer-readable, while QA and old
confirmation admission were rejected.

The implementation's directly related request invariants, complete tape/mixed
ANSWER8 versus4+4 gradient/EOS mask and three checker tests were reused as
implementation evidence, not independently rerun. The checker tests are
`retained_qa_tape_input_objective`, `retained_qa_full_raw_gate` and
`token_cache_actual_native_framing_separates_same_tokenizer`. The final checker
record reports three executed tests, three PASS. The new fit branch was tested
independently above.

**Actual input audit.** `read_A2_bridge.rs` reused native readers and loaded the
protected checkpoint without forward/backward/optimizer work. It independently
verified136 finite Adam tensors, step/sampler11264, family6/normalizer2, QE,
unchanged model/tokenizer/optimizer settings, exact LR3e-5 bits and first-target1.
Adam hash: `f4402d071c11273d4c12cb482904b5fc51373a7ad4c4ffa4b90cb562c643df64`.

All4608 retained rows remain identical. Hash-selected64 train bases produce512
VC0/VC1 source rows and three input variants,1536 bridge rows. Restoring only the
declared system/task-clause and bookkeeping fields reproduces each original
episode exactly. All target answers, records, IDs, ordering, values and times
remain unchanged. The3072 development rows include three dependent512-row bridge
panels; there are128 independent held-out semantic bases. No train/dev semantic
or prepared-prompt overlap was found. Used confirmation and S4 bodies were not
opened or used for selection.

The full1536-update suffix and unchanged11264 prefix were checked. Each batch
is V2/VC2/bridge4 with complete query pairs and distinct bridge bases. Exact
exposures are V3072, VC01536, VC11536, bridge6144, total12288. Training/generation
prompt parity, no evidence exclusion, target masks excluding prompt/padding and
including EOS were checked using the actual tokenizer. Planned costs are
input2,064,384, target162,816, padding233,472; maximum sequence205. They fit the
registered4,000,000/300,000 limits. The schedule is11520/12032/12800 with the
explicit11265 save/restart boundary,512-update segment ceiling and no extension.

Existing probe80 raw rows and terminal usage were read without regeneration.
The original citation result is16/16, S1Q0 is15/16, and both Q1 variants are0/16;
the latter independently establish the training trigger. The unchanged oldQA640
source is frozen at `artifacts/fresh-joint-20260919`, retaining128-token inputs.
Source review confirms it is admitted only after bridge dev/fit and a matching
independent B, with the same endpoint and verified reviewer observations.
No new confirmation route is admitted.

**Failure accounting and preservation.** The implementation's first TINY attempt
failed on missing teacher details after2/24/24; it remains INTEGRITY_FAIL. Its
corrected second attempt used6/72/72. Including this review, canonical totals are
**TINY14 updates,168 generation,168 teacher**. New SMALL usage in this review is0.
The earlier raw-fixture budget configuration failure used no model calls. The
reviewer's initial reader compile failed on two `&&str` indexing errors; the
corrected compile and both read-only invocations exited0. Those logs are retained.
No model trial was repeated to repair the reader.

All28 consumed/protected files matched before and after the input audit. Native
model loads are not generation calls. The Rust reader linked the existing
Accelerate production rlib
`a9126c1a260edacd3a54ab89d7341366649ad1ca479e5df21c8da5699d0b8df0`.

| Independent evidence | SHA256 |
|---|---|
| `A2-fit.log` | `ad74d22d25728ff22036a6126a95a406c502cdf3daedfd0e851bb1e126556623` |
| `A2-process.log` | `15ff64d671c78934ad0b9df4c1ac502a9aded948ba24f703fe3e60ff87816129` |
| `read_A2_bridge.rs` | `bc62b3aa2f34f4a82df0cb45b5543992a7121ab77f6a95eab5ba77a2d223d0f8` |
| `read_A2_bridge` executable | `a42440e9c1f9202b85283598009a13db16fdc4dbeb449b20068bc3f4f8b505d1` |
| `A2-preparation-audit.r3b` | `58d904b3fd7fa94eae1f2476177f78f957e851298fe74be7f75a73dc0c925a8f` |
| `A2-protected-manifest.r3b` | `d6153cf80331dd32a674739bf36a3372017bbbcf10022c5187cc2ee59b98920c` |
| `A2-tiny-accounting.r3b` | `7d3e8e4028f068d6a97d0df2d794353f81d4b0123d72fbbd9358d4590dba80d4` |

The separately published study `review-a.r3b` binds this report's exact bytes,
source digest and preparation hash. It authorizes only the registered bridge
study; it does not grant model quality, B, general QA, S4/S5/S6 or Goal1 acceptance.
