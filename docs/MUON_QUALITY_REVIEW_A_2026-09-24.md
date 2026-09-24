# Metal F32 fresh-optimizer comparison — independent A

**A_ACCEPTED: PASS** for `R3-METAL-F32-MUON-QUALITY-1.0`, before any SMALL
quality update. This is numerical, native-state and bounded-caller acceptance;
model quality, independent B, general QA and Goal1 remain NOT_RUN/not accepted.
The reviewer executed the tests below independently. Product source and original
models/corpus/raw were read-only during this review.

## Reviewed identity

The candidate is based on `52e718eab0ad01396faf85aa6a803c751e8b1330` with the
explicit source changes retained in
`artifacts/muon-quality-20260924-review-a/reviewed-tracked.diff`
(SHA256 `6564e27044e95aea7dcae83be307e1388c5c41fbfab1ba49ae3957b3960c3534`)
and new `src/muon.rs`
(SHA256 `1b559900bfd8ada7f010b1a77ddbcfc457f6b835017047ba77045b10b46a93fc`).
The compile-time candidate source digest is
`2bed8a035ab609350fc8b8db4ea4d8406665b705ecad97f18a96145f3f7415d2`.
The subsequent source/report publication SHA is a separate Git identity; this
review does not assert that the pre-publication HEAD already contains the change.

| Artifact | SHA256 |
|---|---|
| Release trainer | `1efbb40511e7f19f2b448e582d17f5b9b77d9b852f787cd28f2a686a4adccfc7` |
| Release test executable | `48bf5733c1ddbaf0b684e80a833b88ffbb1667e01223e07da215e23aa4f5fe58` |
| Exact release library used by independent readers | `527ccd0ede9a3baa1edafd50e9a01c97d399e027de3e9c52f10408a3e629877d` |
| Cargo.lock | `a911931fc3baca7367a34c129673640661a8fc3d41458d49e4eb744ae5454154` |
| Reused Candle0.11.0 Metal patch | `34faeb37febb834770c5679093b2f22d26c6eff01f40154a7854691b76ca43b2` |
| Native study plan file | `76b8c24f50406b4a6861c42bb04d8decda2519f87c57a4003ad71e5abc223c5b` |
| Preparation receipt | `af501f44eeaab09acbff770a5e2b9e12d3268c1fa8b99620805c9e6587657f82` |
| Independent oracle executable | `d2db47cd05cbb93d2469e288c46d0f5eb9793d6225e07a8f4d0c59de6d0001ac` |
| Independent preparation reader | `5c20235d7c538c54ec3b55f00dce6107d6e9a5631c8db7dbad2497cf5ff8bf85` |

Policy digest:
`6133bfd80aebf586b409fc67e6f4f78a8444df98d50c6e10685c83250d871399`.
Prepared inputs are under `artifacts/muon-quality-20260924-study/`;
review evidence is under `artifacts/muon-quality-20260924-review-a/`.
Build/test source was frozen for this execution. Installed Rust1.98.1, the existing
locked/offline release `accelerate,metal` package graph, incremental disabled,
and one compute thread were used. No dependency or external model was installed.

## Source and preparation findings

The production optimizer uses sum momentum `M=.95*M+G`, Nesterov `H=G+.95*M`,
F32 Frobenius normalization, the specified five polynomial iterations, tall-matrix
transpose/back, and `0.2*sqrt(max(m,n))` once on the gradient update. Decay uses the
unscaled LR. CPU is only the explicit oracle/reference; the registered study uses
actual Metal F32. No BF16 conversion, CPU NS fallback, extra clipping, output
repair or per-task routing was found.

Roles are derived from Config: all q/k/v/o/gate/up/down matrices use Muon in M;
embedding and all norm gains use aux AdamW. A uses AdamW for every parameter.
Coverage is exact, with one update of the tied embedding. Both states allocate
zeros, while learned parent weights remain unchanged. The first fresh Adam bias
correction uses local step1, separately from model step14337. Native resume v3
binds the explicit optimizer family, role map, constants, local clock and runtime;
Muon tensors use `muon.m.*`, not fabricated Adam keys. Historical native v1/v2
meaning is unchanged, and generic training rejects bound research optimizers.

The independent reader opened the original parent, native corpus and metadata,
recomputed all1024 tape rows and tokenized training costs, and checked native
policy coefficients and role descriptors. Observed:

- Parent step14336, no adapter; physical SHA
  `15015900a3c91a6b8ed87e9c2ecb8f7faf124849e5dcf8f40203cddf267e30d9`.
- Weight content SHA
  `45d0c75ef6eca2901a9357fb26e1e4b0ccf8f812b2592d2febea43b2d46cf7a5`.
- Corpus SHA
  `96e36843c792354bff98195e95db3ec98c4d0dde534aaa619787b5259665fa3c`;
  train7680 and validation3456.
- Tape SHA `d959ffffa8cccfd2c78dfd342fcdbcf89a2760b8fafae24ab11aea31b4293477`,
  exactly the original word cycle's first1024 updates, review4/word4 throughout.
- Per arm:8192 sample exposures,5120 unique rows, input1,512,064,
  target119,808, padding138,368. Row exposure histogram:
  2560 unseen,3072 once,1024 twice,1024 three times.
- Role digests A `c2ed5fd1963da7926d8bdc39a85dfd8f9095706890c59c4450e9a2f323ea01bb`,
  M `56cb45132a3607127d78ca78d3dc76769d9ec3512787a454f14cc023eb00f183`.

Three narrow review observations were addressed before the frozen execution:
zero-gradient/nonzero-history now exercises the actual proposal; baseline/B
observation callers use immutable time segments instead of trying to overwrite
an existing start; scorer tokenizer identity and full saved metric checks are
explicit. The latter observation-caller changes were source-reviewed; the actual
new-process terminal-caller execution is separately identified below. No unresolved
confirmed optimizer/state defect remains in this reviewed scope.

## Executed direct evidence

Each exact test below ran once with `--exact --ignored --nocapture --test-threads=1`
on `target/release/deps/replica_train-a3e8f3bc867d1194`. Every top-level invocation
executed1 test, passed1, failed0, exit0. Child-process results are not counted as
additional independent test names.

| Exact suffix under `training::fresh::muon::tests::` | Evidence |
|---|---|
| `scalar_oracle_cpu_metal_ns_and_updates` |37 primitive fixtures; scalar/CPU/Metal, first2 updates, zero/nonzero history and all-zero input; unchanged bounds |
| `small_shape_ns_cpu_metal` |6 fixtures, actual SMALL Q/K/FFN shapes; no SMALL model update |
| `roles_and_rejection` |Role coverage; missing/nonfinite gradient, missing state, unsupported family/clock reject before mutation |
| `answer_accumulation_updates` |ANSWER batch8 versus4+4; both optimizers;4 actual TINY optimizer calls |
| `native_policy_and_completed_no_call` |Family/role/clock/runtime malformed state rejection; RETURNED reuse; missing/UNKNOWN/cancel block; calls0 |
| `native_new_process_restart` |Both A and M: actual2 versus saved1+fresh-process1; weights/state/clock/cursor exact;6 actual TINY optimizer calls |
| `final_evaluation_only_process` |Actual `train` caller in a fresh process at exhausted7200s; final fit/score completes with generation/teacher/optimizer0; durable native unchanged |

The finalization fixture contains explicitly synthetic completed RETURNED and
teacher rows, not claimed model observations. It tests accounting and publication,
while the separate native restart test supplies actual TINY update/process evidence.
Completed wrong rows are reused; cancellation, absent rows and unresolved attempts
remain blocked in the direct boundary test.

The reviewer additionally wrote an independent f64 scalar-loop oracle in
`independent_oracle.rs`. Frozen `norm/ns5/muon_proposal` bodies and the actual Adam
proposal body were copied into a minimal scratch harness with SHA-bound source
provenance, not substituted into product code. This independently authored oracle
checks square/wide/tall matrices, two successive updates, zero second gradient
with nonzero history, momentum/state and delta.12 CPU/Metal fixtures passed,
exit0. Pointwise `1e-4+1e-3*abs(ref)`, meaningful-update NRMSE<=.02/cosine>=.999,
and near-zero floor/max-abs bounds were unchanged. This source-body numerical
check complements, rather than replaces, the actual linked trainer tests above.

On actual SMALL-shaped matrices, observed update max error was at most
`9.313225746154785e-10`; FFN NS max error was `2.123415470123291e-7`.
The preparation reader also passed, exit0. Its first compile attempt omitted
existing sha2/zstd extern flags and did not execute; that failure log is retained.
The corrected compile reused the same existing libraries, with no source change.

## Usage, preservation and limits

Reviewer execution: TINY optimizer10, primitive55, SMALL optimizer0, normal
generation0, teacher0. The implementer's earlier10 TINY/43 primitive plus this
review total20 TINY/98 primitive, below32/128. Seven process/numerical invocations
plus the independent oracle used27.57s measured process wall time (child time is
already included). Implementer direct numerical/process usage11.44s makes the
A admission accounting39.01s. Pure preparation readback1.48s and compilation are
separate; synthetic fixture filesystem time is conservatively included in27.57s.
No old Metal suite, LegacyB, protected generation or quality training was repeated.

The existing five-file preservation manifest rehashed successfully after review,
including protected11264, original14336, failed adapter14464 and its raw/terminal.
No original file, product source or user change was edited by this reviewer.
No deletion, artifact sweep, compression, network model/API or CPU fallback occurred.

Acceptance authorizes only the bound study's next planned steps: same-parent
normal16 parity, fresh-state checks, then the registered same-step A/M comparison
under its guard and total budgets. It does not forecast quality, approve a winning
optimizer, accept independent B, or change `GOAL1_READY=false`.
