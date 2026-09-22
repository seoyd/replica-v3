# Citation fidelity consolidation — independent B, 2026-09-22

**INDEPENDENT_B: PARTIAL.** Native lineage, all saved evaluation/teacher rows, optimizer usage and fixed-prefix fresh parity were independently verified. The final model fails the registered citation gate: two generated citation IDs are outside the provided records. The fresh V16/VC16 sample contains no wrong cases, so the required wrong-case fresh reproduction is **NOT_RUN_PENDING_AUTHORIZATION**. This report does not grant complete B acceptance, candidate permission, confirmation permission or Goal1 acceptance.

| Judgment | Result |
| --- | --- |
| Reviewed code / preparation A | Existing independent A PASS preserved; no new confirmed product defect found in B |
| Final native / Adam / clock / objective / tape | VERIFIED |
| Saved generation3904 and teacher3904 recount | PASS; all stored decisions agree |
| Fresh fixed-prefix V16 / VC16 | PASS,16/16 matches each,32 new generations |
| Fresh reproduction of final wrong cases | NOT_RUN_PENDING_AUTHORIZATION; neither fixed prefix includes a wrong row |
| Independent B overall | PARTIAL because of that reproduction gap |
| Registered3072-update execution | COMPLETE; no extension or rerun |
| Citation quality / candidate | FAIL / NONE |
| Full train4608 / citation confirmation / QA640 | NOT_RUN; prerequisite development gate failed |
| Original scalar4352 acceptance | PRESERVED; not reused for model selection |
| S4 / S5 / S6 / Goal1 | NOT_ACCEPTED; GOAL1_READY=false, GOAL1_ACCEPTED=false |

Reviewed source is `820100a6a27eebefe0ba723fc03948ea6f2a0abb`. The implementation-result report HEAD before this report is `3884c9929e013a832446e31937c645e0c206ce05`; neither report commit changes product source. Scope is R3-CITATION-FIDELITY-CONSOLIDATION-1.0. Sources, tests, Cargo, original model/corpus/raw/failures and previous approvals remained read-only. Reviewer writes were isolated Rust readers, logs and binary recounts, the existing CLI's separate reviewer observations, and this report. No external model, teacher, API or dependency download was used.

## Frozen identities and endpoint

| Identity | SHA-256 |
| --- | --- |
| Compile-time training source digest | `c8ee368de6ef5bcdb31ddcf85d42cb6485fe5278d2658e4119f0f13ba8047fcc` |
| Frozen production executable | `b0f974de8ffe5ff9aa0be81978c57b8b21dc07ac96182ea55b4aacbaf93f108c` |
| Candidate diff | `7f34d56b3a2793a167ba8280c3b4bc95e5778851241501f7f258e553ac9aeb0f` |
| Preparation | `0d90de3e17d3a94e6bb8878277d8aaa0bbaa3a4b3be70ead74c39443ee926a8b` |
| Selection | `a7b8eb3f18d85571ac4d35af09b3ed77311b95c0827d765a083ed6c2752b3d20` |
| Native plan physical bytes | `f03f2ec7e49a0c474c5f9b702520bc42fcb8a0c26764170725ca3fb2bfd6e292` |
| Plan content digest | `2c768c57a77e34f29cbbef784384b60d7597333ea0b61e321189b406e590d967` |
| Tape content | `1ac2793203f5ef3da42d6c37ae8b3c09c13b5baf82da201dd0f66ed88c213e58` |
| Parent7424 physical checkpoint | `8c2e9b0f670be09acbae40efb6e2d4cb4e5a3c01315b0d7e3a51aa467270ec47` |
| Final10496 physical checkpoint | `7845eb2e66bf333f07a4fde28d80601f418f212912c7c7b58fc41424646b5ff6` |
| Final manifest weight payload | `2ae7311273497a45adccfe609e10091ef302421e5fa963146dac9561e38098a4` |
| Final tensor content | `c80af6c77b83b5e135930e1621ec4e7cb85ad376378ff83f87a02e877b480f17` |
| Final evaluator model identity | `df3649f9b64f1cf315af1ec86fcc12bd52cc6e80d68e0c0f6ff899c690641a8e` |
| Final136 Adam tensors | `542468bdda23af463502e59c4b96e6b6ee31a33f40673742cd06710b67f96438` |
| Final training state | `b5e4af0761727e2587b581cf84194885759d6a4c67dd6b364d8da55a6f462972` |
| Final segment terminal | `35674c4800e6cc86edcf91f73665b9ea69b2c1105f6c284267aa1950856098e9` |

These hashes identify different domains; the manifest weight hash is not the physical file hash. Native states at7425,7680,8192,8704,8960,9472,9728,10240,10496 were loaded and checked against segment admission, parent physical bytes, terminal, source, policy, tokenizer, objective and raw evaluation bindings. Each contains136 finite Adam tensors and the expected accumulated clock. Final step and sampler are10496, cumulative input12,371,968 and target536,576. The last durable native is `ANSWER-MEAN/segment-0008/final`.

The original7424 failure remains unchanged. The new final terminal is `Finished / FINAL_QUALITY_FAIL_AT_10496 / resume=false`. Normal quality failure permits read-only reviewer reproduction but does not reopen learning or confer candidate eligibility.

## Actual execution and independent recount

Existing Rust native/binary readers, tokenizer and scoring definitions were reused in isolated helpers. Both helpers compiled successfully against the existing locked dependency artifacts; no model code was modified. Pure recount loaded native states but made zero model forward, generation, teacher or optimizer calls. Reader execution and both fresh native commands exited0. There was no test-filter/zero-test substitution for these checks.

The independent reader verified:

- Exactly3072 committed optimizer rows, their ordered original suffix draws, LR3e-4 and unchanged ANSWER objective family6/normalizer2, first-target1, QE and Adam. Every8-row batch has V4/VC0 2/VC1 2. Actual target76 and reduction denominator8 remain distinct; prompt/padding are not targets and EOS is included. Stored per-example losses agree with the declared answer-mean reduction.
- All24,576 exposures remain in the unchanged train pool: V12,288, VC0 6,144 and VC1 6,144. Each V row occurs8 times and each VC0/VC1 row4 times. No development error was inserted into learning.
- All3904 generated rows and3904 saved teacher rows, expected order/content, checkpoint identity, raw tokens/bytes, strict decode, EOS, error status and score denominators. All7808 prepared/resolved call pairs are present, bound and attempt0, with no pending, UNKNOWN or unaccounted row.
- All required scheduled panels and decisions, including preservation streak0 before/after each scheduled evaluation. Runtime errors are0. The final quality stop agrees with the actual final model and final raw panels.
- Stored teacher token NLL/positions and after-value signals, using existing raw only. No new teacher calculation replaced or supplemented the observations.
- Before/after hashes of15,830 existing study files, plus the16,704-file protected original/A manifest. All matched during pure recount. Existing A report and approval remain unchanged. Fresh reproduction added only its separate reviewer observation files.

The new cycle consumed input3,661,824, target233,472 and padding98,304. Nine segment controls sum to1749.725134418s. These are recorded active-control seconds, not end-to-end command/build/hash time.

## Same-checkpoint quality

QB is QUERY_BOTH, SB is SWAP_BOTH. Both use256 pairs for512-row panels; ALL4 uses128 orbits. The128-row train probe has64 pairs and32 orbits. All listed panels finish with EOS on every row and runtime errors0.

| Step / panel | Strict FULL | QB | SB | ALL4 | Correct value | Correct support | Outside ID |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| 8960 V512 | 512/512 | 256/256 | 256/256 | 128/128 | 512 | — | — |
| 8960 VC512 | 509/512 | 253/256 | 254/256 | 126/128 | 512 | 509 | 3 |
| 8960 renamed512 | 512/512 | 256/256 | 256/256 | 128/128 | 512 | 512 | 0 |
| 8960 train128 | 128/128 | 64/64 | 64/64 | 32/32 | 128 | 128 | 0 |
| 10496 V512 | 510/512 | 254/256 | 254/256 | 126/128 | 510 | — | — |
| 10496 VC512 | 510/512 | 254/256 | 255/256 | 127/128 | 512 | 510 | 2 |
| 10496 renamed512 | 512/512 | 256/256 | 256/256 | 128/128 | 512 | 512 | 0 |
| 10496 train128 | 128/128 | 64/64 | 64/64 | 32/32 | 128 | 128 | 0 |

Final V has one foil value and one other digit. Final VC has two grammatically valid answers with correct values but IDs outside both provided records, rather than the other provided record's ID. Those two rows occupy one ALL4 orbit and are not two independent scenes. Renamed512 has no error. ID_BOTH improves from509/512 at8960 to510/512 at10496; repeated unchanged IDs across the renaming panels are0. Citation grammar is valid on all512 rows of each final citation panel.

The registered outside-ID requirement is0. Therefore neither full endpoint passes development, and full train4608 is NOT_RUN at both endpoints. The perfect128-row train diagnostic does not imply full-train acceptance. No candidate was fixed and no confirmation or QA640 execution was authorized by these results.

Against parent7424 on exactly matched episode content:

| Panel | Parent FULL → final | FULL gain / loss | Parent ALL4 → final | Orbit gain / loss |
| --- | --- | --- | --- | --- |
| V512 | 511 → 510 | 1 / 2 | 127 → 126 | 0 / 1 |
| VC512 | 492 → 510 | 20 / 2 | 113 → 127 | 15 / 1 |
| Renamed512 | 495 → 512 | 17 / 0 | 116 → 128 | 12 / 0 |

Citation improvement is observed, but it is not monotonic across all tasks and does not meet the final zero-outside-ID condition. These observations do not establish a specific attention, storage or backend cause.

## Saved teacher diagnostics, no new teacher calls

The following are independent aggregates of the final512 saved rows per panel. First-value/EOS/signal counts are512 each, grammar tokens3072, ID tokens4096 and suffix tokens7680. NLL is reported separately from normal greedy exact match; a small gold-prefix NLL does not repair the two generated wrong IDs.

| Saved measurement | VC512 | Renamed512 |
| --- | ---: | ---: |
| First-value NLL | 0.002247643884494277 | 0.0023827187673166605 |
| Grammar-prefix NLL | 0.000014265072450577362 | 0.000014435458006722968 |
| Eight ID positions mean NLL | 0.0016746284308734039 | 0.0006931849000466617 |
| Gold-prefix EOS NLL | 0.000005135996587135594 | 0.00000508104876351112 |
| After-value EOS minus citation logit | -16.402466016616543 | -16.324956505351945 |
| Citation suffix mean NLL | 0.0009000624079923044 | 0.0003766612305582839 |

| ID position, each count512 | VC NLL | Renamed NLL |
| --- | ---: | ---: |
| 1 | 0.00019741739807166425 | 0.00015752468551433196 |
| 2 | 0.0007060598952564057 | 0.0005837284581833302 |
| 3 | 0.0020875547663452565 | 0.001732450875692404 |
| 4 | 0.0020333017405831377 | 0.0005270880075602946 |
| 5 | 0.0023195857390443386 | 0.0002695873168319096 |
| 6 | 0.0004392354672244636 | 0.0002527504371601097 |
| 7 | 0.0053947845767705616 | 0.0013349219676515034 |
| 8 | 0.00021908786369140287 | 0.0006874274517794099 |

## Fresh native reproduction and its limit

The frozen production executable was run in two new processes with VECLIB/OMP/RAYON threads1:

```text
artifacts/citation-fidelity-20260922-executable fresh answer-mean-review --root /Users/seo/Projects/Replica-v3/artifacts/citation-fidelity-20260922-study/ANSWER-MEAN
artifacts/citation-fidelity-20260922-executable fresh answer-mean-review --root /Users/seo/Projects/Replica-v3/artifacts/citation-fidelity-20260922-study/ANSWER-MEAN --citation
```

Both exited0 and reported matched16, generation16, optimizer0, teacher0, candidate_permission=false. They used existing normal greedy QE generation, context2048/max_new32, strict UTF-8 and real EOS. The independent reader verified exact raw-token/byte/text/EOS/finish/error/prompt-digest equality to the saved endpoint rows, and confirmed that teacher diagnostics were NOT_RUN in these observations.

The existing review path selects the first16 cases in each panel. **Both fixed prefixes are FULL16/16 and EOS16/16. Neither contains either final wrong V row or either final wrong VC row.** Consequently this is prefix32 parity evidence, not wrong-case fresh reproduction. All wrong saved rows were included in the3904-row recount, but the contract's wrong-case fresh reproduction remains NOT_RUN_PENDING_AUTHORIZATION. The allocated B32 calls have been consumed. A request for four additional, fixed wrong-case calls is pending; no answer is interpreted as neither permission nor refusal. No extra calls, sample substitution, hidden retry or product change was made to fill this gap. Complete B acceptance is not granted.

| Actual current-study usage | Updates | Generations | Teachers | Generated tokens | Active seconds |
| --- | ---: | ---: | ---: | ---: | ---: |
| Training and scheduled evaluations | 3072 | 3904 | 3904 | 48128 | 1749.725134418 |
| Implementer parent V16+VC16, independently read back | 0 | 32 | 0 | 304 | 2.898363249 |
| Independent B final V16+VC16 | 0 | 32 | 0 | 304 | 2.928262667 |
| Total | 3072 | 3968 | 3904 | 48736 | 1755.551760334 |

No new TINY, finite-difference, backward or teacher work occurred in B. The separately closed changed-path verification remains62 TINY updates/500 generations/348 teachers, including the original failed fixture run; it was not repeated or counted as SMALL training.

## Preservation, evidence and disposition

The original scalar4352 confirmation hash remains `9446035282e93bf9c94bf7c56056dc977f6b85c998b0372e1f7a5a6781a4810f`, with its accepted254/256 and ALL4 62/64 unchanged. The citation seal remains `dc99a793cdc79f448a8a159bbff436e110f58887600a35ff622ea36aec4ca3d0`; owner and linked-root records show no execution. Its body was not decoded. No confirmation candidate, attempt, raw or result was created. Original failures, parent states and earlier A/B acceptances remain unchanged.

Local permitted evidence root is `artifacts/citation-fidelity-20260922-review/`. The actual study is `artifacts/citation-fidelity-20260922-study/`; raw panels, separate teacher panels and decisions are under `ANSWER-MEAN/`. No original corpus, checkpoint, raw body or seal body is published with this report.

| Local evidence | SHA-256 |
| --- | --- |
| B-recount.r3b | `888d9e55c26b5e3260b3ac9665c98b2c12f0660634fcad65ad18ea3baed8e53c` |
| B-parity-usage.r3b | `570ab4fb56d828611e2bbecc03bb5864cde64ae642c61fc9c94e4df1c362c5ad` |
| read_B_fidelity.rs | `e89bc92c5fe1610dd39ed9c191106019e61c74d2a289fe34e54673333c189a42` |
| read_B_fidelity executable | `f8f579b4ed85e552f6b5ddd24e060ad1c94707e3bcc9ef5655ba4bfae4b3737b` |
| read_B_parity.rs | `af107db22974e14d1dc00463af16536cfde3c54165ce719ab98e6b69f50c7892` |
| read_B_parity executable | `ac169ab7515f9fb9124a6870f9e4adccd00cae37be1d18d91cc81789a53e2089` |
| Fresh final V16 raw | `238194e90cc2f0039c235b80943e47e4fecb6bc12959c22174f97e1a3f96f63e` |
| Fresh final VC16 raw | `63d098f988aade1966decda20f5567a0333a5ae5a7cbdcf079e9fc5d646e3245` |
| Existing A report, unchanged | `fddbf168acb7b6d8b99fe908b7177ece1c9645fdb47068e19191fcb3160de162` |

Build logs are `read_B_fidelity-build-01.log` and `read_B_parity-build-01.log`; pure execution logs are `B-recount-01.log/.stderr` and `B-parity-usage-01.log/.stderr`. Actual native logs are `B-native-value16.stdout/.stderr` and `B-native-citation16.stdout/.stderr`. Exit0 was observed for both reader builds, both reader runs and both native commands. The review published no additional model approval or learning permission.

**The run is closed as a preserved negative quality result. B is PARTIAL solely for the unexecuted wrong-case fresh reproduction; the reported raw recount and fixed-prefix parity are actual PASS results.** Existing A remains closed, and this report neither automatically extends learning nor treats storage/execution correctness as model or Goal1 acceptance.
