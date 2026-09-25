# Fresh TR++ / Full GRU execution

Contract: R3-QUALITY-GRU-EXECUTION-1.0. Execution closed on 2026-09-26.
Preparation and independent A PASS. TR++ completed3072 updates and its final
evaluation; GRU saved630 updates at TIME_BUDGET. The complete paired study is
PARTIAL. Neither model has quality acceptance, product integration or Goal1.
The single [independent B report](QUALITY_GRU_REVIEW_B_2026-09-26.md) concludes
PARTIAL_RESOURCE_LIMIT: TR++ B PASS, GRU recorded integrity PASS with final
evaluation/reproduction NOT_RUN. Native B report SHA256:
`c225a9efe09ac1d17f3ce035cf1b0643878806a308f02d99ef6e0451188b533c`.

## Frozen implementation and preparation

Original candidate `fa54bb4f582b905fdcae4a4915d7ab22f6ff3840` received independent
A FAIL: reproduction selection consulted correctness, and safe cancellation
discarded earlier completed updates in the segment. Both findings and the first
review's actual two TINY updates remain preserved. The narrow repair is
`2667c27f4353dd8d2a684daa74e38e3af0311826`; its remote main SHA was verified.

The repaired runner uses metadata-only first16 word rows for B. It saves complete
weights/Adam/cursor at safe stop boundaries, retains sticky cancellation, and
does not publish potentially partial optimizer mutations. An explicit revision
references the original unused step0 models and shared cache; it verifies the old
plan, bound FAIL report and native identities before rebinding new execution
policy/runtime. No initial model was rerolled or copied for this repair.

Independent cores: unchanged SMALL TR++9,513,408 parameters and reset-after full
GRU9,811,072 parameters (four layers, H640, embedding384, separate dense r/z/n
gates). The common initial embedding has equal bytes in independent mutable
tensors. Other tensors follow the registered separate initialization rules.
There is no GRU/TR hybrid or Commit Kernel integration.

One existing FULL corpus and token cache serve both cores: train7680,
validation3456, review4/word4 batches, original3072-update cycle, maximum encoded
training length205. Each full arm exposes word1536 eight times and review rows
12288 times in total; input4,534,272 and supervised target359,424 tokens. The
shared answer-mean CE includes EOS and excludes prompt/padding. Fresh AdamW,
LR3e-4 with128-commit warmup, Metal F32 and normal unrestricted greedy are used.
Training dictionaries/scoring oracles do not enter generation.

## Direct and independent evidence

Direct tests passed: CPU numeric3 (including44-coordinate f64 gate VJP), Metal
numeric1 (maximum observed gradient NRMSE3.50e-7), old native compatibility1,
answer-loss1, shared greedy1, strict output bytes1. These original direct logs
retain their original development binary identities; they are reused evidence,
not falsely relabelled as runs of the repaired binary.

The repaired actual Metal cancellation regression committed1 then saved/reloaded
clock1 with CANCELLED/resume=false. Independent A independently decoded it and
ran8 new TINY updates: TR++ and GRU each continuous2 versus1 plus a new-process1.
Weights/Adam/cursor matched exactly (NRMSE0); wrong core/tokenizer/clock, legacy
loader and truncated native were rejected. GRU state restored in a new process,
then rejected a changed-weight or different-request snapshot. Independent
read-only native audit verified zero initial moments, equal initial embeddings,
parameter counts and all referenced original hashes.

TINY cumulative27/32 (initial direct16, failed A2, repair cancellation1,
repaired independent A8). The eight independent updates consumed input64 and
target40, recorded update time1.375193 seconds and maximum observed update RSS
130624KiB. This is not total CLI wall time or total GPU memory. Teacher0.

## Reproduction identities and local evidence

- Repaired binary SHA256: `406f44ed0fdd0f1b15fce0328b89f25f998ae21efcb4d923a394594d81881550`.
- Cargo.lock SHA256: `a911931fc3baca7367a34c129673640661a8fc3d41458d49e4eb744ae5454154`.
- Revised physical plan SHA256: `3f8f14e1cbaa6f35375dad9309b36626f2f1bf0762c3d0f2378927d28753b730`.
- Preparation SHA256: `7c9d0c1315de9b6493ade66dfd606c677980b639f5f0daeb49f0054e64aff664`.
- Semantic source: `1470a6ba4a503c19e15eb29fd2a85d0b3b4b68007522a5f13e3f9fc442bd4da0`.
- Semantic policy: `6f7b0e42db213a1b154ec2b0671b482ee3d25966ba7e446d2d4a1746055b7cd8`.
- Independent A native report SHA256: `212377d90ca57b1c5bbbd926cf2d53a5c3ef8b16b9f601707d4c86008dd81ce0`.
- No-call final report stdout SHA256: `5ba5dc7276f47feacc692f38d1c0a48e6293b26914648d4e91052bf3f38066a5`.

Local evidence is under ignored `artifacts/quality-gru-20260925/`: original and
repair candidate patches; preserved binaries; paired-study and paired-study-repair
plans/preparation; direct logs; review-a FAIL and review-a-repair PASS reports,
Rust readers, six independent Metal process logs and two parity logs. Original
model/corpus/raw and private logs are not published. Recovered specification
SHA256 is `c6af6422c544e98526295aabb7d6b72523d273b95d827eb148eb3275f77742be`;
historical research numbers are not claimed as new Replica results.

Build: `CARGO_INCREMENTAL=0 VECLIB_MAXIMUM_THREADS=1 cargo build --locked --offline --features accelerate,metal --bin replica-train`.
Run the preserved repaired binary with `fresh core-comparison`; `prepare`,
`revise`, `admit`, `execute`, `tiny` and `verify-tiny` are the relevant subcommands.
Exact original command outputs remain next to each local evidence file.

## Actual execution and quality

Both fresh baselines completed32 calls: FULL0/32 and ALL4 0/8 for each;
TR++ generated1024 tokens and GRU1012. Both saved first commit/Adam clock1,
input1400/target102. At the common512 checkpoint each consumed input753536 and
target59904. The following are actual unrestricted generations, not teacher
scores or retained11264 scores.

| Same checkpoint512 | TR++ FULL | GRU FULL |
|---|---:|---:|
| V |16/32|4/32|
| VC |0/32|0/32|
| S1Q1 |0/32|0/32|
| word |0/64|0/64|
| renamed |1/64|0/64|

ALL4 is0 in every common512 panel. TR++ continued to1536: FULL V16/32,
VC0/32, S1Q1 1/32, word5/64, renamed9/64; ALL4 remains0. Low intermediate
scores did not trigger the protected-parent retention guard.

TR++ completed3072 commits/backwards, input4,534,272, target359,424 and24576
example exposures. Its final2752-row evaluation completed across two processes:
2649 then103 new calls. Returned rows were reused, including one time-limited
failure; evaluation continuation added no optimizer update.

| TR++3072 panel | FULL | QUERY_BOTH | SWAP_BOTH | ALL4 |
|---|---:|---:|---:|---:|
| V |266/512|13/256|31/256|0/128|
| VC |47/512|0/256|2/256|0/128|
| S1Q1 |48/512|0/256|6/256|0/128|
| word |30/192|1/96|2/96|0/48|
| renamed |28/192|1/96|1/96|0/48|
| train |45/192|0/96|0/96|0/48|

Original QA primary0/512 and transfer0/128 (total0/640), including fixed-refusal
H0/64 and transfer-H0/16. This is model quality FAIL, not S4/Goal1 acceptance.
TR++ final word whole-value99/192, exact support58/192, valid outside-ID95/192;
renamed whole-value97/192, exact support60/192, outside-ID93/192. Both panels
have EOS192/192 and malformed0. GRU512 word and renamed each have whole-value
32/64, support0/64, outside-ID64/64, EOS64/64 and malformed0. Thus a valid
format/EOS or a matching value does not imply correct evidence selection.
Per-bucket counts are preserved in independent B evidence.

GRU committed630 updates with634 backwards (four deadline-discarded backwards),
input930420, target73710 and5040 committed example exposures. It saved the
consistent final weights/Adam at630 after TIME_BUDGET. The last evaluated GRU
checkpoint is512; its scores above are not630 scores. GRU1536/3072, final fit,
QA640 and final-endpoint B generation are NOT_RUN. The frozen runner's B mode
requires a scheduled endpoint; no unscheduled model calls or budget extension
were introduced to fill this gap.

Final natives under `paired-study-repair/`:

- `TRPP/segment-006-3072.r3model`: SHA256 `f58e6564134fccf7926df1f2d077396fe08aac67cf84ad8bbb88324872a7440f`.
- `GRU/segment-007-630.r3model`: SHA256 `302d42f42cd7e4344edb9d9f62f319d7ac485a5f9bfe1df18ae2837e4f406c54`.

The source remained `2667c27f4353dd8d2a684daa74e38e3af0311826` throughout.
`final-candidate.patch` combines the source changes from the prior report HEAD;
SHA256 `7c7bf7474de60a542b64be18361845fa5b8a5322e4213a6f668ac203eaba9a69`.

## Cost and limits

TR++ consumed3248 generation calls/76498 generated tokens including independent
B16, and4545.310953208 recorded active seconds. GRU consumed256 calls/3252
generated tokens and7232.802929 recorded active seconds; teacher0 for both.
Training is SMALL3072+630=3702 commits and
3072+634=3706 backwards; the separate TINY27 is not included in these totals.

Deadlines are cooperative. Some1800-second segments completed an already-entered
backward and durable finalization after the deadline (for example1833.286
seconds). The final GRU segment recorded1284.409 seconds against about1251.605
remaining. Actual elapsed time is charged; no further model work follows its
TIME_BUDGET. Strict wall-time compliance is NOT_MET, not resource PASS. These
unseparated execution/finalization timings do not prove a new invocation began
after the deadline. No timing threshold was relaxed or failed receipt edited.

TR++ maximum observed training RSS1,176,432KiB; observed decode KV cache
1,312,512 bytes and attention workspace1,048,576 bytes. GRU logical per-request
hidden state is10240 bytes; maximum observed training RSS1,397,680KiB. Hidden
state is separate from model/Adam/activation/RSS memory.
Total device memory and Codex token usage are UNKNOWN. Different endpoint work
and unmeasured GPU totals preclude a general speed or memory-efficiency claim.

Shared target/debug changed from11,494,112 to11,636,104KiB: growth145399808 bytes,
below4GiB. New evidence at final GRU save was1,677,497,033 bytes, below2GiB;
The no-model-call final report exited0 and counted1,677,622,356 bytes at its
measurement, before its own stdout and the last independent report were closed.
Original models/raw/Adam and the user's untracked change remain preserved.
No LR/seed/data/architecture search, cleanup or automatic extension was run.
