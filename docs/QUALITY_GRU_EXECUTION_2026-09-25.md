# Fresh TR++ / Full GRU execution

Contract: R3-QUALITY-GRU-EXECUTION-1.0. Preparation and independent A PASS;
SMALL training is in progress. Model quality, independent B, product integration
and Goal1 are not accepted by this report.

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
- Semantic source: `1470a6ba4a503c19e15eb29fd2a85d0b3b4b68007522a5f13e3f9fc442bd4da0`.
- Semantic policy: `6f7b0e42db213a1b154ec2b0671b482ee3d25966ba7e446d2d4a1746055b7cd8`.
- Independent A native report SHA256: `212377d90ca57b1c5bbbd926cf2d53a5c3ef8b16b9f601707d4c86008dd81ce0`.

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

## Execution boundary

Both fresh baselines completed32 calls: FULL0/32 and ALL4 0/8 for each;
TR++ generated1024 tokens and GRU1012. Both saved actual first commit/Adam clock1,
input1400/target102. TR++ then resumed in a new process to512, input753536 and
target59904. Its224-row intermediate evaluation completed: FULL V16/32,
VC0/32, S1Q1 0/32, word0/64, renamed1/64; ALL4 all0. This is not quality PASS.
GRU's corresponding training is in progress. Final3072 panels, QA640 and B have
not run. Low intermediate quality does not trigger the old protected-parent
retention guard; registered safety/resource limits still apply.

Caps remain3072 commits/3088 backwards and7200 active seconds per core,
8192 generation calls,1048576 generated tokens,2GiB new immutable evidence and
4GiB shared build growth. Device total memory and Codex token usage are UNKNOWN.
No automatic LR/seed/data/architecture search or budget extension is authorized.
