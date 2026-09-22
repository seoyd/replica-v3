# Citation fidelity consolidation — independent A, 2026-09-22

**ACTUAL_PREPARATION_PREREVIEW: PASS.** Candidate code, preparation and the bounded continuation are accepted within this review scope. New SMALL training, model quality, confirmation and Goal1 are not accepted by this result.

| Judgment | Result |
| --- | --- |
| Code / independent A | PASS — no remaining confirmed issue in the changed paths |
| Parent and preparation | PASS — exact ANSWER7424 native, Adam, source, report, corpus and tape |
| Final exact-source direct regressions | 3 passed / 0 failed; zero-test result not used |
| New SMALL optimizer / generation / teacher | 0 / 0 / 0 in this review |
| Current7424 citation quality | FAIL, preserved; outside-record IDs17/15 |
| Original scalar4352 acceptance | PRESERVED; not regenerated or repurposed |
| Citation confirmation | NOT_OPENED; no candidate or confirmation attempt created |
| S4 / S5 / S6 / Goal1 | NOT_ACCEPTED; GOAL1_READY=false, GOAL1_ACCEPTED=false |

Reviewed source: `820100a6a27eebefe0ba723fc03948ea6f2a0abb`. The report commit is separate from this source commit. Scope is R3-CITATION-FIDELITY-CONSOLIDATION-1.0. Original product sources, tests, checkpoints, corpus, raw, failures, seal and earlier reviews were read-only. Only isolated review source/target/TMPDIR/fixtures/readers/logs, this report and the explicit new study A approval were written. No external model, teacher, API, tokenizer mapping or downloaded dependencies were used.

## Source and executable evidence

The exact Git candidate was archived into a separate source directory. Its product, tests and Cargo contents match the current source. The checker independently reported unchanged source before/after the actual test command.

| Identity | SHA-256 |
| --- | --- |
| Compile-time training source digest | `c8ee368de6ef5bcdb31ddcf85d42cb6485fe5278d2658e4119f0f13ba8047fcc` |
| Checker source/Cargo/tests/examples digest | `ac16db47423ff478d0f7fb03c265943ef748029c09204900c76e8f68468dd99b` |
| Production executable, without test-support | `b0f974de8ffe5ff9aa0be81978c57b8b21dc07ac96182ea55b4aacbaf93f108c` |
| Independently built test executable | `c75d39208799b21707c122fbe1998e1b5d76ad9629e5f8cf9b00d3654d9c6277` |
| Independently built checker | `700f13be2400632f62fde8519f92891fe1361ea17d80c85ef4692c39d51423de` |
| Candidate diff, previous HEAD0294a383→820100a6 | `7f34d56b3a2793a167ba8280c3b4bc95e5778851241501f7f258e553ac9aeb0f` |

Training-source and checker-source digests cover different defined byte sequences and are not interchangeable. The first preparation reader incorrectly compared these two domains and stopped before native load; its source, binary and exit101 log are preserved. The reviewer-only comparison was corrected to independently verify each definition. The successful repeat used zero model calls. No product change or TINY test retry followed this bookkeeping error.

## Actual changed-path tests

Rust1.98.1 / Cargo1.98.1; existing lock and offline cache. Commands ran in isolated `A-source`, with absolute `CARGO_TARGET_DIR=A-target`, `TMPDIR=A-tmp`, `R3_CONT_TEST_ROOT=A-fixture`, and VECLIB/OMP/RAYON threads1:

```text
cargo build --locked --offline --release --features accelerate --bin replica-check
replica-check --output A-quick quick --citation-fidelity
```

The checker executed:

```text
cargo test --locked --offline --features accelerate,test-support --release --bin replica-train citation_fidelity_ -- --nocapture --test-threads=1
```

Exact executed tests:

- `training::fresh::identifiable::binding::citation::tests::citation_fidelity_process`
- `training::fresh::identifiable::binding::citation::tests::citation_fidelity_tape_and_schedule`
- `training::recovery::tests::citation_fidelity_inference_budget`

All3 passed, exit0. Test harness303.05s; Cargo command including compilation332.666193791s. Checker build8.40s and test compilation29.12s are separate from model evaluation timing. Checker summary: `CHECKED_SCOPE_PASS`, source_unchanged=true, actual_small_updates0.

The actual process regression used the existing native evaluator and same prepare/parent/arm paths. It verified:

- The original continuation profile still creates and completes the required TINY parent; a wrong parent profile is rejected before new study publication.
- The new profile retains the parent tape prefix and appends its own original citation suffix. Absolute start/end/evaluation boundaries include7425,8960,10496 and evaluation-only reentry at the final step.
- Native continuous2 versus new-process1+1 and evaluation-only2+0 preserve endpoint weights and Adam. Separate random seed93 continuous2 and1+1 preserve weights/Adam/training state without substituting an EOS model for that comparison.
- Random native generation's real control-token failure remains a failure. A confirmed first-call NOT_INVOKED TIME_BUDGET resumes in a new process and produces the same returned failure once; the returned ordinal cannot be regenerated. UNKNOWN/control-token failure is not turned into resumable success.
- After the last teacher row is RETURNED and durable at teacher12/12, a TIME_BUDGET leaves EvaluationPending. A fresh process completes with optimizer0/generation0/teacher0, the exact same native physical hash, then Finished and the actual negative quality verdict. No extra optimizer step completes evaluation.
- The sealed ancestor remains the same. A malformed confirmation-link in the disposable fixture is rejected and restoration returns to the original binding. No independent admission in the fixture is a hard refusal, not implicit approval. A closed negative run cannot resume. Its reviewer V/VC reproduction is allowed separately from candidate eligibility.
- The 12GiB inference limit is applied to fidelity evaluation and observation. `restrict_rss` cannot increase an existing limit. The direct boundary test accepts12GiB and rejects12GiB+1KiB as RESOURCE_LIMIT with zero generation/teacher calls. This is a boundary test, not a claim that12GiB was allocated or a resource benchmark.

The discovered exact-teacher-cap admission problem was checked dynamically after the narrow fix. The final source permits equality only in EvaluationPending; the pending endpoint uses the current step and reserves zero remaining calls. RunControl continues to reject any additional invocation. Existing cancel, I/O, UNKNOWN, mixed-error and quality-stop mechanisms were not weakened. Unchanged former confirmation test suites were not rerun as new evidence.

## Parent and native preparation

The new initial file is byte-identical to the original final7424 checkpoint. The original `Finished / FINAL_QUALITY_FAIL_AT_7424 / resume=false` terminal and its comparison remain unchanged.

| Artifact | SHA-256 |
| --- | --- |
| Parent/native initial physical bytes | `8c2e9b0f670be09acbae40efb6e2d4cb4e5a3c01315b0d7e3a51aa467270ec47` |
| Parent manifest weight payload | `66c16ddf00396df00fae1266c628c95384a94a2a2f195eabc4eb30352634b875` |
| Parent tensor content | `d9ed4e38e815949a3297b17d059e2de738b501fd5e5ecfef76eb2b6480daaa11` |
| Parent evaluator model identity | `1d071953549a7b5da4cc88c25f364b104a824f6d8232255ccc1448306eb84225` |
| Parent136 Adam tensors | `3a2c0e4846b62c1d40ab051e8222964b0346baffe8b83ee43c3929c93fb42006` |
| Parent training state | `6f96023a880e9e69ddc310e685d440ed3c45c104e4a36d4be668d997643cc505` |
| Bound independent parent report | `c167a27f4fe12a9e80e660dcb225adf398e532bb8ec8899fcb9972cced7ea8af` |
| New preparation | `0d90de3e17d3a94e6bb8878277d8aaa0bbaa3a4b3be70ead74c39443ee926a8b` |
| New selection | `a7b8eb3f18d85571ac4d35af09b3ed77311b95c0827d765a083ed6c2752b3d20` |
| New native plan | `f03f2ec7e49a0c474c5f9b702520bc42fcb8a0c26764170725ca3fb2bfd6e292` |
| New tape content | `1ac2793203f5ef3da42d6c37ae8b3c09c13b5baf82da201dd0f66ed88c213e58` |

The native reader verified finite136 Adam tensors, step/sampler7424, cumulative input8,710,144 and target303,104, family6/normalizer2 answer-mean objective, first-target1 and QE. Native policy, source, tokenizer, corpus and parent report links agree. Architecture, Adam configuration, LR3e-4, batch8/accumulation1, zero warmup, seed, loss and tokenizer are unchanged.

The4,608 train rows,1,536 development rows, native metadata, transfer and tokenizer files match the parent's files byte for byte. There is no new dataset or restored historical receipt. The new tape has10,496 rows: its first7,424 rows equal the parent tape and rows7,424–10,495 equal the parent's original citation rows4,352–7,423, in order. This is one new complete3,072-update citation cycle, not the previous continuation's index32 suffix.

All4,608 train samples were independently framed/tokenized using the existing tokenizer. Shifted target masks exclude prompt/padding and include EOS. Each V answer has2 target tokens and each VC answer17. Every batch has V4/VC0 2/VC1 2,76 actual target tokens and answer-mean objective denominator8; these denominators are not interchanged. Request framing and answer lengths reproduce input145(V)/153(VC).

| New cycle | Updates | Input | Targets | Padding | V each | VC0 each | VC1 each |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| Through8960 | 1536 | 1,830,912 | 116,736 | 49,152 | 4 | 2 | 2 |
| Through10496 | 3072 | 3,661,824 | 233,472 | 98,304 | 8 | 4 | 4 |

No train index leaves the unchanged train pool. The existing dev/confirmation separation is retained. The conditional full train fit is not mistaken for training exposure. Planned generation14,336/teacher13,120, active10,800s, command900s and cleanup120s match the registered plan. Parent parity32 and reviewer parity32, candidate-only confirmation512 and conditional QA640 remain included in the generation ceiling.

## P0 retained evidence and interpretation

The separate P0 Rust reader reused the existing native loader, tokenizer, scorer, raw and saved teacher fields. It independently recounted the5888/7424 development panels without any new forward/generation/teacher/optimizer call. Labels were checked from the two provided records; no resolver was connected to inference.

| Step / panel | Strict full | Correct provided ID | Other provided ID | Outside ID | Unparseable | Value wrong with correct ID |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| 5888 VC512 | 474 | 486 | 0 | 26 | 0 | 12 |
| 5888 renamed512 | 488 | 498 | 0 | 14 | 0 | 10 |
| 7424 VC512 | 492 | 495 | 0 | 17 | 0 | 3 |
| 7424 renamed512 | 495 | 497 | 0 | 15 | 0 | 2 |

All four panels have512 EOS, valid citation grammar and zero runtime errors. Every outside-ID row has the correct value. At7424, the32 outside-ID errors occupy30 paired slots/20 semantic orbits, with2 slots failing in both panels; they are not32 independent scenes. VC errors are Hamming distance1×15 and4×2; renamed errors distance1×11 and2×4. Error IDs match neither a full training ID nor the earlier pre-renaming IDs. Full denominator prefix/repeated-digit and target/other similarity tables, paired gains/losses and saved teacher/NLL statistics are retained in P0-audit.r3b. They describe the error distribution; they do not establish a particular attention or memory mechanism.

Between5888 and7424, strict-full gains/losses are V4/1, VC37/19 and renamed19/12. Current7424 V511/512 and ALL4 127/128 does not waive current citation support/outside-ID failures. Saved teacher gold-prefix statistics do not replace free generation metrics. The earlier scalar4352 confirmation254/256, ALL4 62/64 and its original hash are preserved.

The original citation seal is unchanged (`dc99a793cdc79f448a8a159bbff436e110f58887600a35ff622ea36aec4ca3d0`). Four linked studies, including this new study, bind to that owner with VERIFIED_UNUSED status. No confirmation candidate, started/returned/final artifact or owner claim exists. The seal body was not decoded; only its existing receipt/link/hash and absence of execution were checked.

## Actual usage and preservation

Independent native journal readers cross-checked control counts, optimizer traces, prepared/resolved records, raw rows, parent observations and random native state transitions. Counts include the failed implementer run. No repeated full TINY suite was used by independent A.

| Current-contract execution | TINY updates | Generation | Teacher | Test outcome |
| --- | ---: | ---: | ---: | --- |
| Implementer quick01 | 14 | 120 | 92 | 1 pass / 1 fail; test assertion stopped after split2 |
| Implementer quick02 | 24 | 190 | 128 | 2 pass / 0 fail |
| Independent exact-source A | 24 | 190 | 128 | 3 pass / 0 fail |
| Total | 62 / limit64 | 500 / limit1024 | 348 / limit1024 | Failed work included |

quick01's missing fixture admission correctly raised IoNotFound; its test had incorrectly expected Ok(false). The failed fixture and actual14 updates were preserved. Independent A did not count that run as a passing test. Both complete process runs contain20 optimizer updates in normal training controls and4 separately evidenced random-model updates. Each has190 raw generated tokens, one confirmed NOT_INVOKED call, no UNKNOWN/unresolved/pending call, and28 started/finished pairs. Independent A's known segment/observation/random-evaluation elapsed times are15.466177790/2.824006624/0.132819291s. Random optimizer timing is not separately recorded and is not fabricated as0; actual whole test timing is reported above.

Preparation audit verified16,704 source-artifact/report files before and after its read-only execution. Their manifest hash is `8a03d32dba9108ba49493ca8b7ee500f3c6f5f0f28b34c9b98f3c2e80ab6e804`. All matched. The explicit new A receipt is the only subsequent authorized study addition. Prior A/B reports and approvals remain unchanged.

## Review evidence and disposition

All detailed evidence is local under `artifacts/citation-fidelity-20260922-review/`; original corpus, model, raw and temporary instructions are not published.

| Evidence | SHA-256 |
| --- | --- |
| A-quick/summary.r3b | `c907612e254b3564b9344b5030afc1a19dba906a722addfc7ed36964d48e7a6a` |
| A-quick/command-00.r3b | `eb7070ab92aa1ad9d549760bc56ebb0f91c160a1251d4e4bc5bdf2dd95d9518d` |
| A-preparation-audit.r3b | `1a621ce5590b7b0634ef43da8be55f5ac112471747a78a6985e0521415cb8a4a` |
| A-actual-usage.r3b | `5bbdcf048cf99c2ee762e578786a6f5e5f0a74641aec0e568204b4f0597d563f` |
| P0-audit.r3b | `b2871d5f653915f04859b9410bdce9a0de450910333d32cb8daea1e5ac99bb11` |
| read_A_fidelity.rs | `b71b87e05bf2ad7c9ab9dab8fc243af2a6433c7a928253f5a1561539eedbdd81` |
| read_A_usage.rs | `71f484aab7ee6208638e2cfc968dc3b27bf7244ec82bab4a558a41c38f36cdd3` |

**Independent A is closed as PASS for the source/preparation above.** The implementer may perform the authorized parent32 parity and registered3072-update continuation, respecting actual safety, quality and budget conditions. Independent B and any confirmation depend on the resulting actual endpoint. No new learning result or Goal1 acceptance is implied by this report.
