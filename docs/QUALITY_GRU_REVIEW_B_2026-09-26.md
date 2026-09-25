# Independent B — fresh TR++ / Full GRU

RESULT: **PARTIAL_RESOURCE_LIMIT**. TRPP's completed 3072 endpoint passes independent B recount, native verification and fixed16 reproduction. GRU's recorded execution, saved630 native and scheduled512 results pass read-only integrity checks. GRU630 final evaluation, old QA640 and B reproduction are **NOT_RUN** because630 is not a registered evaluation endpoint and its time budget is exhausted. The complete paired-study contract has not passed.

| Judgment | Result |
| --- | --- |
| CODE / independent A | Previously passed for frozen repaired source; unchanged paths reused |
| B execution | TRPP PASS; GRU read-only integrity PASS, final reproduction incomplete |
| Study completion | PARTIAL:3072 versus630 commits; common evaluated endpoint512 |
| Model quality | TRPP FAIL; both512 panels insufficient; GRU630 quality unmeasured |
| Resource | Cooperative deadline overruns observed; hard1800/GRU7200 caps NOT_MET |
| Product integration / Goal1 | NOT_RUN / NOT_ACCEPTED |

Execution source is `2667c27f4353dd8d2a684daa74e38e3af0311826`. Later reporting commits are separate. Source, Cargo files and the frozen binary were unchanged during this review. Binary SHA256 is `406f44ed0fdd0f1b15fce0328b89f25f998ae21efcb4d923a394594d81881550`; policy `6f7b0e42db213a1b154ec2b0671b482ee3d25966ba7e446d2d4a1746055b7cd8`; physical plan `3f8f14e1cbaa6f35375dad9309b36626f2f1bf0762c3d0f2378927d28753b730`. The preserved reset-after specification provenance and initial-state review remain the independent A evidence; historical GRU results are not included in this study.

Independent Rust readers decoded the existing native formats and tokenizer bytes. They recomputed complete-answer/EOS, full-value, selected-support, malformed/outside-ID, orbit and QA-bucket metrics without calling the product scorer. They checked batch order against the frozen tape, input/target counts against the shared packed samples, every commit/Adam clock and LR, native hashes/tensor registries/finite weights and moments, and every returned call's prepared/resolved/raw hashes. All recorded score fields checked agreed. No UNKNOWN or duplicate RETURNED calls were found among3504 observations. Frozen corpus/label preparation evidence was reused; this is not a newly generated label oracle.

| Actual execution | TRPP | GRU |
| --- | ---: | ---: |
| Parameters | 9,513,408 | 9,811,072 |
| Commit / backward / discarded | 3072 /3072 /0 | 630 /634 /4 |
| Input tokens | 4,534,272 | 930,420 |
| Target tokens, including EOS | 359,424 | 73,710 |
| Last recorded response CE | 0.092760 | 1.266699 |
| Generation calls, including B | 3248 | 256 |
| Generated tokens, including B | 76,498 | 3,252 |
| Accounted segment seconds | 4545.310953 | 7232.802929 |
| Maximum observed update RSS, KiB | 1,176,432 | 1,397,680 |
| Maximum recorded decode state bytes | 1,312,512 KV | 10,240 hidden |

Losses are last-batch observations at different endpoints, not comparable full-corpus loss estimates. Trace-derived per-row exposure vectors are retained in the native recounts; TRPP completed the3072 tape with12,288 word exposures and12,288 review exposures. GRU consumed exactly its630-update prefix. Equal parameter scale and shared data do not imply equal compute or elapsed time.

At the shared512 endpoint both arms have input753,536 and target59,904 tokens. FULL results on the same224 cases were:

| Panel | Denominator | TRPP512 | GRU512 |
| --- | ---: | ---: | ---: |
| Numeric value | 32 | 16 | 4 |
| Citation | 32 | 0 | 0 |
| S1Q1 | 32 | 0 | 0 |
| Word | 64 | 0 | 0 |
| Renamed word | 64 | 1 | 0 |

ALL4 was0 in every512 panel. GRU's citation/S1Q1 rows were all malformed complete-answer grammar despite EOS32/32; its word and renamed panels each had value32/64, support0/64 and outside IDs64/64. These are actual failed outputs, retained without repair. The512 results are burn-in observations, not an impossibility test or a quality-based early stop.

TRPP's normal3072 endpoint completed all2752 prescribed rows. Value/support/outside columns below are whole-value and parsed-support diagnostics, not substitutes for strict FULL:

| Panel | FULL | Whole value | Selected support | Outside IDs | QUERY_BOTH / SWAP_BOTH / ALL4 |
| --- | ---: | ---: | ---: | ---: | --- |
| Numeric | 266/512 | 266/512 | N/A | 0/512 | 13/256;31/256;0/128 |
| Citation | 47/512 | 251/512 | 105/512 | 307/512 | 0/256;2/256;0/128 |
| S1Q1 | 48/512 | 252/512 | 92/512 | 334/512 | 0/256;6/256;0/128 |
| Word | 30/192 | 99/192 | 58/192 | 95/192 | 1/96;2/96;0/48 |
| Renamed | 28/192 | 97/192 | 60/192 | 93/192 | 1/96;1/96;0/48 |
| Representative train | 45/192 | 96/192 | 91/192 | 74/192 | 0/96;0/96;0/48 |

These six panels had EOS on every row, zero malformed rows and zero UTF-8/length errors. Four-view rows share an orbit; the pair/orbit denominators are not independent extra examples. Representative train192 does not replace a full1536-row fit gate. Final quality fails the frozen criteria; no thresholds were relaxed.

Old QA primary was0/512 and transfer0/128. Each of the eight buckets was0/64 primary and0/16 transfer. Their observed error counts by bucket A–H were primary `[57,45,3,11,6,8,35,10]` and transfer `[8,5,0,0,0,0,12,0]`. Primary had EOS337, length175, outside14, malformed5; transfer had EOS103, length24, one preserved command-timeout row, outside8 and malformed1. Both had UTF-8 errors0 and exact nonempty support0. Fixed-refusal H contributed0/64 and0/16; fixed-refusal emissions in non-H were also0/448 and0/112. All three G refusal subtypes had strict successes0. The final evaluation's2649-returned/103-returned time split retained its timeout row and did not regenerate it.

The only new B model command was:

```text
VECLIB_MAXIMUM_THREADS=1 artifacts/quality-gru-20260925/replica-train-repair fresh core-comparison execute --root artifacts/quality-gru-20260925/paired-study-repair --core TRPP --phase review
```

It exited0, generating exactly16 responses/256 tokens in14.215020 seconds with optimizer0 and teacher0. Independent raw comparison verified the first four frozen word orbits, global indices1536..1551: correct2 and incorrect14 all matched their original token IDs, bytes, text, error, finish and completion fields. The16 reproductions add nothing to the accuracy denominator. Replay probes word cases only; read-only recount covers all returned panels. GRU received no B model calls. TINY use remained27/32 from A; B added none.

Resource accounting is explicit. GRU exceeded7200 accounted seconds by32.802929; the four TIME_BUDGET receipts show cooperative deadline overruns33.298567,31.653196,36.038804 and32.819268 seconds. TRPP's final-evaluation TIME_BUDGET receipt exceeded its segment deadline by0.048172 seconds. Source/trace inspection shows no new forward or generation after an observed stop: already-entered computation is disposed before optimizer, then consistent checkpoint/receipt publication runs. The evidence does not separate those two overrun components precisely. These facts do not establish a hard-cap PASS. All actual segment seconds were charged forward; paired time was11,778.113882 seconds, calls3504/8192 and tokens79,750/1,048,576. Pre-entry ledger/input/native validation is outside this segment timer; total CLI wall time was not independently measured.

RSS is sampled host RSS, not a guaranteed process peak or GPU total. TRPP's recorded KV tensor bytes and maximum attention-workspace metric1,048,576 are separate quantities. GRU's10,240 bytes equal4×640×F32 for one request's hidden state, excluding weights, training activations and allocator storage. Device total memory and physical footprint are UNKNOWN; no unified-memory quantities are added together. Successful generation rows reported prefill/decode sums185.125/1812.215 seconds for TRPP before B, and202.351/22.487 for GRU; panel sizes, output lengths and endpoints differ, so these are not a matched speed benchmark. Scoped artifact logical size at the GRU audit was1,677,592,665 bytes, below2GiB; later small report files are outside that measurement timestamp. The same `du -sk target/debug` method measured11,494,112→11,636,104KiB, growth145,399,808 bytes, below4GiB.

Final native SHA256 values are TRPP `f58e6564134fccf7926df1f2d077396fe08aac67cf84ad8bbb88324872a7440f` at3072 and GRU `302d42f42cd7e4344edb9d9f62f319d7ac485a5f9bfe1df18ae2837e4f406c54` at630. Both native archives independently passed schema/checksum/state/tensor-size checks, finite model/Adam values, nonnegative Adam variance and exact clocks.

Evidence is preserved locally under `artifacts/quality-gru-20260925/review-b/`: `TRPP-segments9-recount.r3b`, `GRU-segments8-recount.r3b`, both `*-native-*.r3b` and `*-attempt-audit.r3b`, `TRPP-QA-fixed.r3b`, `TRPP-replay-audit.r3b`, reader sources/binaries and logs. The single final native B report is `report.r3b`, SHA256 `c225a9efe09ac1d17f3ce035cf1b0643878806a308f02d99ef6e0451188b533c`. The first reader's overly narrow NOT_RUN assertion was corrected to accept the preserved NOT_RUN_TIME_BUDGET status; its failed log remains. This was a reader issue, not a model rerun.

Original source/models/corpus/raw failures and the previous A FAIL remain preserved. This reviewer changed only new review evidence and this report, and did not commit or push. Publication is separate from execution-source identity. Codex usage/model effort is unobserved or unchanged. This single-seed, single-LR partial protocol does not establish general architecture superiority, promote either core, authorize more learning, or grant Goal1 acceptance.
