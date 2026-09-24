# Metal F32 Runtime B — 2026-09-24

Contract: R3-METAL-REDUCTION-REPAIR-1.0. Independent reviewer: citation_continuation_review. Runtime B PASS for the registered Metal F32 execution and state-integrity boundary. No general QA, Goal1 acceptance or further learning is granted.

## Reviewed identity

The reviewed source is `cef33c777a1db32e3937046d401a9927bbf87046`. Its Rust bytes are identical to the executed G4/G5 snapshot: commit `4556b6dacd853d7a07fec18d83db666a3851dd01` plus the frozen tracked diff `80abfe67a9b6a4b781acb5d77b7b27cb746c54d1463d74cd699fe9133acc90d5` and new `metal_runtime.rs` snapshot `b8a0289d09c4b53add05cca263e5479d22d8e5b13ed92a8acd2afa5567c131f3`. These are source identities, not this report's eventual publication commit.

| Artifact | SHA256 |
|---|---|
| Frozen trainer, `replica-train-g45` | `95c94f2892bcb3475f98fead9a1f2cb44bc9b0c3d31c4a8b295d7c074da4e1ba` |
| Frozen worker, `replica-v3-g45` | `3b7d342b2ba65e3f76b16d8eca245c5378a98d5795a77e6de5993438dd385d1a` |
| Exact G45 project library, debug accelerate+metal | `fdc57b3fcee72525c2d015a00d9cbca38e6d0b204c45d673560f29204f9c78a5` |
| Independent pure reader executable | `54df96283a5bf787fd4383a97fcb265fbcdc44a57cbcb295bad029a4663c13f8` |
| Independent fresh32/readback executable | `7270021fbeba8ea575a71a4263eedd27a387c528d4408993c32069912934c637` |
| Independent audit receipt | `ccd238da1e6f2e9690c44b3e0c18b7f6678d478c13c45bae51f76df47f8aaada` |

The exact library was reused with `rustc`, without Cargo/dependency builds. `build-exact.sh`, `build-fresh.sh` and `identities.sha256` retain the linked dependencies and commands. The earlier release-library exploratory reader is excluded from final acceptance. Reviewer compiler failures and the initial p95 index-definition discrepancy are preserved, all with zero model calls. The final quantile uses nearest-rank `ceil(0.95*n)-1`, index22 for n24; no producer values or tolerance were changed.

Evidence is local under `artifacts/metal-reduction-20260924-runtime-b/`; original runtime records remain under `artifacts/metal-reduction-20260924-runtime/`. Raw contents are not published here.

## Independent checks

`review-exact audit`: exit0. Independently recomputed canonical request/prompt/token/text/finish fields and scores against native corpus episodes, start/end hashes, entered/returned work, same tape indices and response masks including EOS, native state, complete budget accounting, and benchmark quantiles. No forward/backward/generation/teacher/optimizer call occurred in this readback.

`replica-train-g45 fresh metal-runtime train --root artifacts/metal-reduction-20260924-runtime --arm cpu16 --until 16`: exit0 in a new process. It read the completed proof and reported `durable_step=14352 new_optimizer0 new_generation0`; no original finished record was replaced. Preservation is checked again around fresh reproduction.

| Existing execution | Independently confirmed result |
|---|---|
| Protected11264, CPU/Metal128 each | 128/128 each; 0 CPU-correct losses; raw tokens identical; historical CPU parity |
| Challenge14336, CPU/Metal64 each | 32/64 each; raw tokens identical |
| Actual worker path | CPU4 + Metal4 returned; backend/model/prepared prompt bound; cancellation/zero deadline precede request entry |
| Metal full/chunk/token cache | Lengths1,127,128,255,256,257,512,2048; 2,086 diagnostic forward calls, generation/backward0 |
| CPU16 / Metal16 / Metal1+15 | Exactly48 optimizer updates, all three stop at14352 |
| Native restart | Metal continuous/split weights, 136 Adam tensors, step, cursor, input and target counts exact |
| Endpoint generation | 12/16 for each arm; all48 raw token/text/finish results identical across arms |
| First SMALL numeric gate | Each Metal arm:68 gradient vectors and204 first delta/Adam vectors satisfy the recorded numerical limits |

Cache maximum absolute errors were 6.1988831e-6 for full/chunk and 9.0599060e-6 for token mode. CPU/Metal loss absolute drift over16 draws was at most8.5830688e-6. First/last response-mean CE: CPU5.485620975→2.667597771; Metal5.485620499→2.667594433. No16-step tensor drift is judged against the first-update tolerance.

Each arm consumes128 examples, input23,776, target1,872 and padding1,952. Combined:384 exposures, input71,328, target5,616, padding5,856. LR3e-5, fresh schedule reset0, unchanged inherited Adam, ANSWER mean CE, batch8, tokenizer/QE, and the registered first16 draws are verified.

| Final native | Physical SHA256 |
|---|---|
| CPU16 | `98c2a733929b093cdab9671b57f7fc6e857e9503e4703b21c2b25d6743c5d7ba` |
| Metal16 | `e2ebc993e2abe482989fef2059be00ce13e8dd4dbadfa78ff1b1cd9a8f589006` |
| Metal1+15 | `c724c45f51df98c30844f583740036a7a0e53083019637c7336a37202d9f9b33` |

Metal tensor content `73a1a79e45378e126fa5c694b977cbec3c7b060280168a46907556105aff8c5b` and sorted Adam digest `2bffe4653d1eb072b4f206fe35009d65139dfb130845e9dde15bdb823cef534d` match exactly between continuous/split. Different whole-file hashes retain distinct arm policy provenance. Each native is114,181,184 bytes; all six saved models total685,087,104 bytes.

## Bounded timing observations

These are the actual debug accelerate+metal execution profile on the recorded macOS27.0/26A428 device. The generation benchmark uses the unchanged14336 parent and8 fixed cases, one warmup plus3 measured repetitions each (n24/backend). It is not a general hardware throughput claim. End-to-end excludes model load; TTFT includes first token selection after prefill. RSS is the recorded process observation, not a measured allocator peak.

| Measurement | CPU | Metal |
|---|---:|---:|
| Load seconds | 8.641 | 7.906 |
| End-to-end median / p95 seconds | 0.9683 /1.2453 | 0.5298 /0.5868 |
| End-to-end min–max seconds | 0.2635–1.3307 | 0.0965–0.6289 |
| TTFT median / p95 ms | 255 /310 | 63 /70 |
| Prefill median / p95 ms | 255 /310 | 62 /69 |
| Decode median / p95 ms | 706 /940 | 455.5 /512 |
| Median ms per actual generated token | 64.578 | 31.378 |
| Recorded RSS KiB | 198,448 | 104,672 |
| Median training ms/update, excluding named CPU-reference interval | 12,882.219 | 806.691 |

Metal split median training time is819.180ms/update. First-update timing includes additional comparison overhead outside the separately named reference interval; medians are observations, not pure-kernel timings. Per-case repetitions and range are retained in `audit.r3b`. Shader preparation is included in warmup and not isolated; actual model-level canonicalization copy bytes and GPU allocator peak remain UNKNOWN. The earlier exact primitive copy measurement24,576 bytes is not extrapolated to a model.

## Usage and scope

Before independent fresh32:504 SMALL generations =384 P/C +8 workers +48 endpoints +64 benchmark;48 SMALL optimizer updates;2 additional diagnostic backwards over16 teacher-forced samples. Raw generation tokens6,274. Recorded action time1,276.712816s; including120s numerical reserve1,396.712816s, below7,200s. All registered commands finish within900s. There are no pending/UNKNOWN training or generation calls in the audited runtime.

The preceding independent Runtime A remains unchanged; its numerical tests were not repeated. New reviewer TINY training/generation, SMALL optimizer and teacher calls are0. Full general QA, S4/S5/S6, model promotion and Goal1 acceptance are not established by this bounded runtime comparison.

## Independent fresh32 completion

`review-fresh reproduce` and `review-fresh readback` both exited0. A single new process executed exactly CPU16 endpoint16 + Metal16 endpoint16, using the original fixed validation indices0..4,512..516,2560..2564,3072..3076. All32 raw token sequences, text, EOS/finish and error fields reproduced exactly, including4 previously incorrect answers per arm. Both scores remain12/16. There was no retry, optimizer, teacher, output repair or expected-answer input to generation: only the ordinary prepared prompt IDs and unchanged request limits enter `Transformer::generate_observed`. Gold/reference output is compared after RETURNED.

Prepared and returned receipts close all32 ordinals; UNKNOWN0. The32 new calls generated436 tokens in58.950814292s. Original native/raw/start/end/policy hashes were checked before and after; the no-call finalization changed no original bytes. Current source diff and new module hashes also remain exact.

| Final evidence | SHA256 |
|---|---|
| Fresh preparation | `48e3b33bd186cc4e8fe5e22f984ae242f7abb93ed70a11ef68c5673dc19a38c6` |
| Fresh raw | `9c5e448cacc609b462fd3d20099148fde0c139c4b81777a063e16d0046882cbb` |
| Fresh finished | `76136862c00ec9cbf6085d4fd58aaa6d354953c22273fe4f17fca67f05321092` |
| Post-run preservation verification | `100339e9b22a25d67183fc342bd576795fa7a259c134d315b5e7f638f1107074` |
| Independent final readback | `c39b51e1816f29ef4a2f46ca960078a8b11794f60a4aa0adb85d07d0c8ede5ea` |

Final SMALL totals are536 generations,6,710 generated tokens,48 optimizer updates and2 diagnostic backwards over16 teacher-forced samples. No automatic extension was performed. The recorded runtime action time plus independent fresh process is1,335.663630291s, or1,455.663630291s including the explicitly reserved120s. Pure reader/compiler/file-verification work is not represented as model computation.

CPU-versus-Metal16 endpoint differences below are observations only, computed from the saved tensors without forward/backward. RMS and NRMSE columns are the maximum *per-tensor* value in each group, not pooled whole-model statistics. Near-zero NRMSE uses the recorded1e-6×sqrt(N) norm floor.

| Tensor group | Count | Maximum absolute difference | Maximum tensor RMS | Maximum tensor NRMSE |
|---|---:|---:|---:|---:|
| Weights | 68 | 2.458692e-7 | 2.580957e-8 | 2.755422e-7 |
| Adam m | 68 | 6.742775e-7 | 1.002723e-7 | 3.770204e-5 |
| Adam v | 68 | 7.312337e-10 | 7.617197e-11 | 1.414114e-5 |

No confirmed implementation defect remains within this reviewed execution boundary. Protected generation, real worker/cache, bound native training/resume, the first numerical update and independent output reproduction are accepted for this exact profile. Challenge32/64 and endpoint12/16 are preserved quality limitations. They do not establish broader language, general QA, S4/S5/S6 or Goal1 success.

Scoped reviewer artifact measurement:98 regular-file paths,42,436,331 logical bytes under the new Runtime B root. This counts the shared hardlinked audit binary at both paths, so it is conservative path-total storage, not allocated bytes or build growth. No original artifact inventory or deletion was performed. Build net growth remains UNKNOWN; no Cargo rebuild occurred in B.
