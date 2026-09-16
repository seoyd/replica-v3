# Native Goal 1 work graph

Contract: [GOAL1-NATIVE-TRPP-1.0](GOAL1_CONTRACT.md).
Baseline: `436ed1d1cdc9efa18c3728bc75fe972b8a5fab14` on `main`.
Historical B0 execution is recorded in IMPLEMENTATION_REPORT.md; it is not current
verification. Source inspection and execution are separate evidence levels.

`S0 → S1 → S2 → S3 → S4 → S5 → S6 → independent review`

| Node | State | Paths / checks / evidence | Remaining exit condition |
|---|---|---|---|
| S0 | VERIFIED | GOAL1_CONTRACT.md, logs/goal1-environment.txt; local instruction bytes unchanged by index-only removal | published 7d831d0; remote matched |
| S1 | VERIFIED | src/{store,app,retrieval,model}.rs; tests/{store,runtime,retrieval,cli}.rs; logs/goal1-s1-tests.txt; logs/goal1-s1-source-digest.txt | phase commit/push |
| S2 | VERIFIED | src/{data,neural,train_main}.rs; tests/{native,training}.rs; logs/goal1-s2-final-tests.txt, goal1-s2-tokenizer.txt; goal1-s2-source-digest.txt | phase commit/push |
| S3 | VERIFIED | src/neural/{transformer,checkpoint}.rs, src/training.rs; logs/goal1-s3-exit-tests.txt, goal1-s3-small-boundaries.txt; goal1-s3-source-digest.txt | phase commit/push |
| S4 | NOT_STARTED | actual training and independent evaluation | T-N07, fixed quality thresholds |
| S5 | NOT_STARTED | src/{model,app,main}.rs and CLI | T-I01–03, actual restart |
| S6 | NOT_STARTED | own quantization, measurement and docs | T-N08, T-I04, T-D02 |

| Trace | Required observation | State |
|---|---|---|
| T-R01 | canonical result binding, no model call on corruption | VERIFIED |
| T-R02 | full prompt IDs and actual evidence partition | VERIFIED |
| T-R03 | each search budget plus complete short search | VERIFIED |
| T-R04 | synchronized startup/head commits vs genuine corruption | VERIFIED |
| T-R05 | pinned backup under writes, restored bytes | VERIFIED |
| T-N01 | own tokenizer roundtrip, train-only hash, no truncation | VERIFIED |
| T-N02 | native forward/masks/GQA/RoPE/QK norm/SwiGLU gradients | VERIFIED (CPU numerical/state boundary) |
| T-N03 | bounded prefill/decode cache and reference parity | VERIFIED (CPU numerical/state boundary) |
| T-N04 | actual gradient, weight update, masked loss | VERIFIED (CPU numerical/state boundary) |
| T-N05 | strict checkpoint load and fresh-process resume | VERIFIED (CPU numerical/state boundary) |
| T-N06 | corpus/split/random-init/training lineage | VERIFIED (own corpus/init/probe lineage; final training pending) |
| T-N07 | >=200 heldout generations and category quality | NOT_STARTED |
| T-N08 | packed inference, quality and memory comparison | NOT_STARTED |
| T-I01 | five own-model memory queries and fresh restart | NOT_STARTED |
| T-I02 | timeout/cancel/citations/COMMIT failure | VERIFIED |
| T-I03 | raw bytes/history/restore/as_of/reindex/backup | VERIFIED |
| T-I04 | offline runtime and measured M4 backend | NOT_STARTED |
| T-D01 | temporary instructions decoupled, originals preserved | VERIFIED |
| T-D02 | phase code identity equals remote branch | VERIFIED (S0; later phases pending) |
