# Independent Runtime A: Metal reduction repair

Date: 2026-09-24. Contract: `R3-METAL-REDUCTION-REPAIR-1.0`.

**RUNTIME_A: PASS for G2/G3's tested F32 numerical boundary.** The original
public sum, representative layouts, analytic broadcast VJP, all-variable GQA,
whole TINY gradients and first Adam update passed independent actual execution.
Overall runtime completion remains **PARTIAL**: G4/G5, protected SMALL quality,
registered Metal training/restart and performance are not accepted by this A.
GENERAL_QA_IMPROVED is NOT_ESTABLISHED; S4/S5/S6/Goal1 remain unaccepted.

## Reviewed source and dependency

Executed source: HEAD `37e3f4d86c3ee3488b4f6e9838319a1d0d2acc2b` plus staged
`artifacts/metal-reduction-20260924/g3-stats-candidate.diff`, SHA256
`3eab6a19ae8b963ba73d5a56ed5cca6d629f5546b4a36e46d345e65e0ccab4e7`.
The final staged diff matched this identity after the tests. This source identity
is separate from the subsequent publication commit and report hash.

During source review, the reviewer found missing p99/RMS vector statistics.
The implementer added p99, RMS and the near-zero floor before independent
execution; numerical tolerances and operation paths were unchanged. No test was
run by this reviewer on the superseded G3 binary.

| Item | SHA256 |
|---|---|
| Published Candle core 0.11.0 archive | `5ecb245093b0f791b89d3420c3df9c6d49c60ab63ba54db896bf8a3baf486706` |
| Root `Cargo.lock` | `a911931fc3baca7367a34c129673640661a8fc3d41458d49e4eb744ae5454154` |
| `vendor/candle-core-0.11.0/replica-metal-sum.patch` | `34faeb37febb834770c5679093b2f22d26c6eff01f40154a7854691b76ca43b2` |
| Patched `src/metal_backend/mod.rs` | `20ccaadf0f1b097b366636d48dbb79f4009d66da4abfcaefc37f2d6bd6b833d6` |
| Replica `src/training.rs` | `59280f98d5983cbc11fccd13c0cfa1b857b670a5d54020b6c11adfc5268b3095` |
| Frozen `replica-train-g3-stats-tests` | `f9cf303e5d32c01c4e76cf457f5bf4c7607c03ed118a293d8171e308362cd811` |
| Frozen `native-g3-tests` | `b8496d8d95b70f5e74e66b8e8849aa9fdda9512e2753ad42df8fddb7789e6ff4` |

The retained archive checksum matches the original locked package. Its upstream
revision is `31f35b147389700ed2a178ee66a91c3cc25cc80d`; Apache-2.0 LICENSE is
preserved. A scoped registry/vendor comparison found only the Metal source
delta, two patch/provenance documents, and registry's `.cargo-ok` marker. The
registry Metal source also matched the published archive bytes. Existing package
graph evidence resolves both Replica and candle-nn 0.11.0 to the single vendored
candle-core 0.11.0. No dependency upgrade, registry edit or duplicate vendor copy
was performed by the reviewer.

## Source boundary

`MetalStorage::reduce_op` preserves the existing contiguous path. Only a nonempty
F32 Sum that would enter the strided path materializes the existing logical
preserved/reduced axis permutation on the GPU, including original offset and
zero strides. It then dispatches the same operation using physically contiguous
storage and suffix axes. The recursive invocation therefore reaches the
unchanged contiguous branch. Allocation element/byte products are checked.

Public Tensor sum and the unchanged Broadcast backward both reach this boundary;
the latter still calls `grad.sum_keepdim(sum_dims)`. Replica's `repeat_kv` is
unchanged. There is no fixture-value branch, CPU reduction/upload, detach, mean
substitution, gradient replacement or new shader. CPU/CUDA, other dtypes and
min/max/arg operations are outside the patch condition.

**ORIGINAL_STRIDED_SHADER_FIXED=false.** This is a correct GPU replacement path,
with an additional full logical-input F32 allocation and GPU copy when selected.
Allocation size follows actual shape; speed/allocator peak for production
workloads is not measured by this numerical A.

## Independent actual tests

Seven tests ran sequentially, once each, with
`VECLIB_MAXIMUM_THREADS=1 RAYON_NUM_THREADS=1 OMP_NUM_THREADS=1`,
`--exact --nocapture --test-threads=1`. The six training tests additionally used
`--ignored`; their full names have prefix `training::metal_tests::`.
Frozen executables were used directly without a new build. Every invocation
ran exactly one test and exited 0; no zero-test or historical result is counted.

| Test suffix / native test name | Result | Test seconds | Wall seconds |
|---|---|---:|---:|
| `metal_f32_middle_axis_sum_oracle` | 1/1 PASS | 3.87 | 4.55 |
| `metal_f32_reduction_layouts` | 1/1 PASS | 0.08 | 0.08 |
| `metal_f32_repeat_and_qkv_vjp` | 1/1 PASS | 3.92 | 3.92 |
| `metal_f32_attention_backward_boundary` | 1/1 PASS | 0.05 | 0.05 |
| `metal_f32_tiny_forward_gradient_update` | 1/1 PASS | 1.74 | 1.74 |
| `metal_f32_admission_is_fail_closed` | 1/1 PASS | 0.09 | 0.09 |
| `native_checkpoint_roundtrip_and_corruption_rejection` | 1/1 PASS | 0.14 | 0.57 |

Summed reported durations: 9.89 test seconds / 11.00 process-wall seconds.
These include test/process/device setup and are not a speed benchmark.
Actual Metal registry ID was `4294968525`, dtype F32; CPU fallback count was 0.

The original shape `[8,4,2,12,8]`, dyadic input, independent scalar oracle and
public `sum_keepdim(2)` call are preserved. Both CPU and patched Metal exactly
matched every output, including first value `-0.8125`; max/p99/RMS error were 0.
An added exact-equality assertion strengthens this dyadic fixture without
extending bitwise requirements to general F32 reductions. Earlier RED evidence
remains unchanged.

The 22-case layout table covers original/production shapes, representative
batch/group/sequence/head dimensions, first/middle/last and multi/all axes,
transpose, unsorted axes, narrow offset 21, zero-stride broadcast, singleton,
empty axes and scalar shapes. The independent f64 scalar oracle indexes the
original source buffer through each view's stride/offset; it does not use the
product materialization helper. Duplicate/invalid axes and malformed views are
rejected. The existing Metal zero-byte constructor rejection is preserved and
is not counted as a successfully computed empty Metal sum.

Analytic repeat VJPs use distinct K/V upstream patterns and groups 2 and 4;
all dyadic results match exactly. Actual GQA makes Q, K and V separately
trainable. Observed Q/K/V gradient NRMSE values were approximately
`9.3813e-8 / 8.7988e-8 / 4.1754e-8`, with cosine above `0.99999999999999`.
The primitive sequence completed RMS, RoPE, repeat, GQA, linear and SiLU.

All 24 TINY parameter tensors passed for both TOKEN and ANSWER gradients.
Padded microbatches used actual target denominators 21/41 and 20/41 for TOKEN,
and 4/8 each for ANSWER, on both devices. All accumulated gradients passed the
same registered bounds. ANSWER embedding gradient NRMSE was `1.8023965e-7`,
compared with the preserved unpatched failure near 0.479; this is a numerical
repair comparison, not evidence about old CPU model quality.

The same Adam body performed one CPU and one test-admitted Metal update at
clock 1. All 24 weight deltas and 48 m/v tensors passed their applicable bounds.
Measured gradient norms were CPU `2.777685288999244` / Metal
`2.7776853049488834`; update norms were `0.0049414033136463776` /
`0.004941403451688341`. Clipping uses the unchanged shared rule/configuration.
Per-tensor norm, max, p99, RMS, near-zero floor and cosine are retained in raw
test logs; near-zero cosine is not fabricated as 1.

## Usage, preservation and remaining gates

Independent usage was **TINY optimizer 2, whole-model forward 19, whole-model
backward 12; generation 0, teacher 0, SMALL calls 0**. Separate primitive VJP
backwards were 10, plus 12 in the six-operation chain. The explicit successful
sum-call accounting was 63: original 2 + layouts 39 + VJP losses 10 + chain
losses 12. This counts public calls, including empty-axis identity cases, not
GPU kernel dispatches or reductions inside whole-model backward. Four invalid
axis calls rejected before reduction are separate; no UNKNOWN or retry occurred.
CPU native roundtrip used only a new temporary fixture, with no forward/update.

The production Adam guard remained closed and was tested before mutation;
only the test invoked the private shared update body after its gradient gates.
Runtime A accepts this exact patch/F32 scope, not generic Metal training.
G4 must bind the admitted runtime/profile/parent and reject mismatched resume
before weights or moments change. No environment bypass was introduced.

The existing named preservation manifest passed: original 42-row LegacyB raw
and receipt, failed adapter 14464, base 14336 and protected 11264 retained their
hashes. The reviewer made no product edits, model loads from those originals,
new source/vendor copies, cleanup or legacy regenerations. LegacyB's prior PASS
is preserved with zero new LegacyB calls.

CPU scalar/reference comparisons, TINY numerical parity and native corruption
rejection passed in this scope. Historical SMALL CPU output, full cache boundary
coverage, native Metal restart, protected quality, SMALL48, same-Metal restart
and workload performance remain **NOT_RUN by this A**. Their later execution and
independent B remain required; this report does not substitute for them.

## Local execution evidence

Evidence root: `artifacts/metal-reduction-20260924-runtime-a/`.

| Log | SHA256 |
|---|---|
| `01-public-sum.log` | `0fc36ccf63a72de29f62d07943d7cc44daf004d77e196602bba85c3fcc5e36ff` |
| `02-layouts.log` | `86fa1018a1aca00c9c329ca4aad39e1a471052add57b4259e842987b2854b85f` |
| `03-vjp.log` | `23fe70b1a11472a42457d2d54dc201e0dd1a3bcce987bbfbb08710ce8f646e66` |
| `04-op-chain.log` | `6d0f3eb9c32593641382fc8f74e6f1b5dc45e6c4ae179c4d7d77ce0966968cd1` |
| `05-tiny.log` | `a34e8bd864eed11e1102446d88b4dc80233b63ff71e583c1d844404b74f14436` |
| `06-admission.log` | `04c6992ec7f8d1f6d76c16564ec22a3ea1de0fc946e8165b48dd41ca377752fe` |
| `07-native.log` | `6857cabe0c84b9b7eca14427aad2b66c6f692b71351e98559ae1eb1e86c61014` |
| `upstream-files.log` | `6e4e8ac8a81be34905fa9ec4dc1afbd7cb9d1569d8d30a4b42fc3d5f8100eaef` |
| `protected-after.log` | `aea0b595f8611832d53b7a117754616af17f7a88c22b5ab46dd1bba366f57674` |

Publication/remote verification is performed separately by the implementer;
the reviewer has not committed, pushed or claimed a remote SHA for this report.
