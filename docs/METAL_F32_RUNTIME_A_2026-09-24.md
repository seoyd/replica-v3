# Independent Metal F32 Runtime A

Date: 2026-09-24. Contract: `R3-METAL-F32-BASELINE-1.0`.

**Runtime A: NOT_ACCEPTED. M2 numerical gate: FAIL / BLOCKED.** The independent
reviewer reproduced an incorrect Metal middle-axis reduction against a scalar
oracle. M3 generation/cache/worker acceptance, M4 learning/restart, M5 performance
and Runtime B are **NOT_RUN**. No model-quality or speed claim follows from the
partial forward results. The separate LegacyB completion remains valid.

## Candidate and actual independent execution

Reviewed source was HEAD `da1987d3a5a6ba74c1d25eec121fb483c97d24f9` plus frozen
`artifacts/metal-f32-20260924-evidence/m2-candidate.diff`, SHA256
`94a967e91410548bc8788c4ad8647607409efb9a148891357d817ca80c91a5a7`.
The current eight-file diff matched those bytes. The source identity is separate
from the later report/publication commit.

Frozen test executable:
`artifacts/metal-f32-20260924-evidence/replica-train-m2-tests`, SHA256
`3bd26da935b4c55df7c771a28fa9add80ace9c3719781955f3ff0df41c567544`.
The reviewer verified its hash and reused it without rebuilding dependencies.
Exactly one independent test invocation was made:

```sh
VECLIB_MAXIMUM_THREADS=1 RAYON_NUM_THREADS=1 OMP_NUM_THREADS=1 \
artifacts/metal-f32-20260924-evidence/replica-train-m2-tests \
--ignored --exact training::metal_tests::metal_f32_middle_axis_sum_oracle \
--nocapture --test-threads=1
```

**Actual result: 1 test, 0 passed, 1 failed, exit 101**, reported duration 4.57 s.
This is successful reproduction of a numerical defect, not a passing numeric
test. No retry or tolerance change was made. CPU and actual Metal device
`gpu_id=4294968525` were used; the fixture did not silently fall back to CPU.

The F32 input has shape `[8,4,2,12,8]`, contiguous strides
`[768,192,96,8,1]`, and values `((i % 41) - 20) / 32`.
The scalar oracle explicitly adds the two axis-2 elements for each output.
`sum_keepdim(2)` should produce `[8,4,1,12,8]`.

| Result | CPU | Metal |
|---|---:|---:|
| First differing flat index | None | 0 |
| Oracle value at index 0 | -0.8125 | -0.8125 |
| Actual value at index 0 | -0.8125 | -0.3125 |
| Maximum absolute error | 0 | 1.65625 |
| p99 absolute error | 0 | 1.53125 |
| RMS error | 0 | 0.6269952080567337 |

The registered `1e-4 + 1e-3 * abs(reference)` limit fails. These differences
are not a bitwise-only discrepancy or a near-zero cosine convention.
The observed boundary is the installed Candle 0.11.0 Metal F32 reduction on
this shape; it does not prove that every Metal operation or device is faulty.

Independent usage: two primitive reductions, **0 native-model forward,
0 backward, 0 optimizer updates, 0 generation and 0 teacher calls**. The fixture
uses synthetic tensors and reads no original model or private corpus.

## Existing direct evidence reviewed, not rerun

The implementation logs retain these distinct outcomes:

| Test/boundary | Observed result |
|---|---|
| TINY padded forward `[8,12,264]` | Within pointwise limit; max error `1.7881393432617188e-7` |
| TOKEN / ANSWER CE | Within limit; absolute errors `0` / `9.5367431640625e-7` |
| Same TINY embedding gradient | FAIL: NRMSE `0.4792736070923524`, cosine `0.8789800447152862` |
| Isolated TOKEN/ANSWER loss gradients and repeated-index embedding gradient | PASS, one test |
| RMS and RoPE primitive forward/backward | Passed before the next failing primitive |
| `repeat_kv` backward `[8,4,12,8]` | FAIL: NRMSE `1.488532390725811`, cosine `-0.007109491795689133` |
| Device/cache/kernel/Adam rejection test | PASS, one test; unchanged weights and zero moments |

The TINY test failed before the update phase, so neither of its planned optimizer
updates occurred. The primitive sequence stopped at `repeat_kv`; later GQA,
linear and SiLU checks in that test are not counted as executed. `repeat_kv`
expands the head-group axis through broadcast before reshaping; the separately
reproduced reduction error identifies a relevant failing primitive boundary,
without claiming an exhaustive explanation of every full-model gradient error.

Three directly related CPU implementation regressions each passed one test:
`native_kv_chunk_rollover_parity_reset_and_identity`,
`native_checkpoint_roundtrip_and_corruption_rejection`, and
`missing_model_and_commit_failure_keep_input_without_success`.
They are reused implementation evidence, not independent reruns. No zero-test
result is counted as PASS. They do not establish the unexecuted fixed SMALL
generation or complete worker comparison.

## Admission, preservation and incomplete scope

Source inspection confirms CPU remains the default. Explicit Metal creation
fails if unavailable, without CPU fallback. Worker runtime metadata reports the
requested backend and is checked by the parent; historical model/semantic
identity and `MIXER.numeric_policy` are unchanged. Cache device identity is
checked, CPU-only `RustDecodeGemv` is rejected on Metal, and native save begins
with device synchronization.

All Adam entry variants reach `step_with_rate`. Its Metal-variable rejection is
before gradient reduction, moment construction or weight mutation. The existing
direct admission test verifies this rejection and state preservation. Metal
training/backward capabilities are not advertised as accepted. The production
CLI/backend wiring as a whole is **not independently accepted** by this report.

The full execution descriptor, trainer policy bound to device on resume, and
Metal native roundtrip/restart are **not implemented or not verified to the
contract's acceptance level**. Long-context/cache boundaries, batch8/4+4 gradient
comparison, accepted Adam updates, protected SMALL generation, worker process
comparisons and performance remain incomplete. The failed numerical prerequisite
blocks these downstream stages; it is not replaced by forward-only agreement.

No original checkpoint, corpus, tokenizer, failed run or previous report was
modified by the reviewer. No source repair, fallback backend, new kernel,
alternative optimizer or wider experiment was attempted. M2 new SMALL generation,
teacher and optimizer usage is zero; the contract's only new SMALL generations
so far are the separately recorded 16 LegacyB completion calls. Existing QA and
Goal1 failures remain unchanged.

## Evidence identities

All implementation logs are under `artifacts/metal-f32-20260924-evidence/`.

| Artifact | SHA256 |
|---|---|
| `m2-tiny-numeric.log` | `0b3574e7dcf388ef299ae886a4bb648d47b7613106ea6d46791807b6bb4a6bd3` |
| `m2-backward-boundary.log` | `fd62e2122f099ddf845c5b260b2b2f45ea44bcb22a596382b9663ef3498ba6a9` |
| `m2-attention-boundary.log` | `4b40090a0ca54ff0cf8b4d3fbdd3c420914ac3baf6b2e042301725c7bb793dfc` |
| `m2-sum-oracle.log` | `fad5fdbd2882d6dde586f6761bf2d7ab9b490c0a9abb9288f08e24d406015d7a` |
| `m2-admission.log` | `0104f363220ac55c0bebfb2f411fe10438a52e7b300d52136113e7efc74125e0` |
| Independent `artifacts/metal-f32-20260924-runtime-a/sum-oracle-independent.log` | `87bb7210713f7f23eeb40c916ef50784985e564c0343433a3ade8047e1e12808` |

Runtime A issued no acceptance receipt. The preserved failing test and this
report close the present review as NOT_ACCEPTED; downstream authorization
cannot be inferred from the presence of the Metal feature or device flag.
