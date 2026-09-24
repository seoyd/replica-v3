# Independent A — protected adaptation and precision preparation

**INDEPENDENT_A: PASS.** The bounded q/v adapter preparation and changed native
execution boundaries pass this review. This is permission to execute the
registered experiment, not model-quality, FP4-quality, S6 or Goal1 acceptance.
Independent SMALL optimizer/generation/teacher calls are **0/0/0**.

Reviewed source is the frozen worktree at base
`d45c3e0bed2052e7b683408ff375ef4113b324e3` plus the preserved patch below.
Its compile-time source digest is
`fa0080d2abbc7376f89c0df73560b4522756e779366f3d10607f95000fe7b952`.
The implementer will publish these reviewed bytes in a separate source commit;
the publication SHA is not substituted for the reviewed source identity.
No product source, original data or earlier acceptance record was changed by
this reviewer.

| Frozen item | SHA256 |
|---|---|
| `candidate-worktree.patch` | `3844867659a86c070e12ae195d1ad4ce448c139bb067a396cd1e4ca450d7eda9` |
| Production `replica-train` | `7d707564bbeb54fa8d77695e782633139740ba64534ee72661cd395c64c444fd` |
| `replica-train-tests` | `41d968d6f798f2f7b6316a84628fb0ef5e71bb6b5532123d99ce05b4581326b9` |
| Preparation | `266a47f994ee046f288518d092ae9f7d4af13ccd19738bac58e8dc67eed66ffc` |
| Selection | `d8a0cd0e31615ef0d7d840582be4798a4c237e1be79685caf69033666c1dc647` |
| `ANSWER-MEAN/plan.r3b` | `e09d8abaa640877fbd551f002ddfe9c038bdd98d3dbeb59dc43dcb34b9ac0830` |
| Initial delta | `de0695b649614e3443fa12c2a936620ededabad49a319725bb9e3da1d1e5bd5e` |

Artifacts are under `artifacts/protected-adaptation-20260924-evidence/`,
`artifacts/protected-adaptation-20260924-study-final/` and the independent
`artifacts/protected-adaptation-20260924-review/`.

**Executed independently.** The exact frozen test executable ran
`training::fresh::identifiable::binding::citation::adapt::tests::adapter_native_process`
once with `--ignored --exact --nocapture --test-threads=1`, Accelerate and compute
threads1. `R3_ADAPTER_WORD` points to the preserved
`artifacts/qa-word-value-20260924-tiny-process-02/continuous`; the new
`R3_ADAPTER_TEST_ROOT` is the independent review's `process/` directory.
Result: **1 passed, 0 failed, exit0, 31.74s; optimizer8, generation72, teacher72**.
`A-process.log` SHA256:
`325c6a9a01190be03ebcf13d7ec70f030b9a346ddb998aee65df85ab2f8d7fd3`.

Continuous2 versus new-process1+1 and final evaluation-only resume preserve
weights, Adam, clock and raw. The completed returned prefix is unchanged and
evaluation-only performs optimizer0. Wrong base/tokenizer/framing/objective,
rank/scaling/registry, metadata/clock/lineage, missing/truncated/trailing delta
and generic trainer entry are rejected. Closed-run continuation, extra parent
dev generation and QA entry are rejected.

The preserved EOS fixture has zero output projections and consequently zero
q/v gradient; it verifies execution, not learning. The separate random native
fixture verifies nonzero updates, initially zero A gradient and subsequent
nonzero A gradient, frozen base, and identical delta/Adam/state after2 versus
1+1. It also checks the same actual batch8 versus4+4 ANSWER reduction. Its four
optimizer steps use six backward evaluations, including two extra microbatch
comparisons; those comparisons add no optimizer step or model-generation call.

Canonical terminal/control receipts and numeric checkpoints independently give:

| Run | Result | TINY optimizer / generation / teacher |
|---|---|---|
| Implementer process01 | Preserved failed zero-gradient expectation | 2 /36 /36 |
| Implementer process02 | PASS | 8 /72 /72 |
| Independent process | PASS | 8 /72 /72 |
| **Total / remaining** | Limits32 /192 /192 | **18 /180 /180; remaining14 /12 /12** |

**Reused evidence.** The unchanged-path implementer release tests are retained
as implementer evidence, not described as new independent executions:
`core-tests-01.log` 2/2 PASS (scalar gradient/frozen registry and prefill/cache
identity; eight synthetic backwards, three native full and six cached forwards,
generation/optimizer0); `native-regression-01.log` 3/3 PASS (original native
format and corruption/publication boundaries); `fp4-codec-01.log` 1/1 PASS
(16 codes, signed zero, nearest-even ties, saturation, subnormal, padding,
checksum/malformed input and a reference layer; max absolute error0.044921875).
All used the locked/offline release build. Earlier accepted learning/QA/B
experiments and whole suites were not rerun.

**Actual preparation audit.** The independent Rust reader reuses the existing
native corpus/checkpoint/codec readers, with no forward, teacher or optimizer.
The exact14336 base physical hash remains
`15015900a3c91a6b8ed87e9c2ecb8f7faf124849e5dcf8f40203cddf267e30d9`.
Registered original references, protected11264 and failed14464 also match their
preserved hashes. Base inference loading allocates no historical Adam. The base
has68 tensor entries; trainable registry is only24 A/B entries and48 fresh Adam
entries. The model step is14336; the adapter Adam update clock is0, so its first
bias-correction clock is1. Tensor counts are not optimizer clocks.

Rank8/alpha8, seed20260924, deterministic normal A and zero B match the registered
initialization. Actual shapes give59,904 adapter parameters,239,616 F32 weight
bytes and479,232 Adam bytes. The compressed native initial delta is141,811 bytes
and references the original base; it does not contain another full base or Adam.
Base content/config/tokenizer/QE and combined content identities, raw tensor
shapes/hashes, family6/normalizer2/first-target1, LR3e-4/decay0/warmup0, policy
and source bindings all agree.

Corpus7680/dev3456, metadata and tokenizer are immutable hash-bound references
to the accepted word preparation. The actual consumed tape is precisely its
first1,024 updates, batch8:8,192 exposures comprising V1,024 / VC0 512 /
VC1 512 / bridge2,048 / word4,096. Independent prompt/mask calculation agrees
with the registered costs: **input1,512,064 / target119,808 /
padding138,368**. Prompt and padding are excluded from target counts; EOS is
included. The remaining full tape is not scheduled for execution.

Policy caps are1,024 new optimizer steps, generation/teacher8,192 each,
input4,000,000, target400,000 and active7,200s, with900s commands. Screens occur
at+64/+128/+256/+512 and full dev at+1,024. Planned maximum including zero-B
parity16, screens1,280, full3,584, conditional fit1,536 and reviewer64 is6,480
generations and6,400 teachers. Conditional fit requires development acceptance;
retention failure still stops. The same active adapter serves every input.
Training resolver/expected answers are not routed into generation. QA640,
confirmation and S4/S5/S6 remain denied. Source review found no remaining
confirmed blocker in these changed boundaries.

The independent preparation receipt is `A-preparation-audit.r3b`, SHA256
`115cc120ba8e470b8b0c8312da8e696017ce083b64bf4963b1f3f5a06034f066`;
usage receipt `A-tiny-accounting.r3b`, SHA256
`26b9028d9e43824e1d8e6f793574e65cfb5d709573090a8f9e19e891a56b8d50`.
Reader source SHA256 `889e3434d7466f6263c1b4bba32b9ad1b3eb8c809f502c4e524fc22f15c7fc24`,
executable `809a55d5dd0fb4b4f58aebe200186e6d0b0ab517cb25c0a32e5f68ce1575e37f`,
production rlib `16c298d95b7c022a5fb32beb67e1389c4b5e5459cbb720001bfa630a49984e91`.
Reader build and execution both exit0; no Cargo/dependency rebuild was needed.

**Resource and acceptance limits.** The implementer's bounded new-owned-roots
measurement records53,311,319 logical bytes/1,081 files, excluding shared
symlinks. Touched retained crate build outputs total67,335,036 bytes/46 files;
net cache growth is UNKNOWN because no prior per-file size baseline exists.
This is a retained touched-output upper bound, not savings. Evidence:
`owned-bytes-before-learning.txt`, SHA256
`a2e9aef96c252cf51ad402a3a23d7ed643d802345bdc12f21ad5148d64f960ed`.
No old inventory, deletion, compression or cleanup was performed; deleted bytes0.

P model learning, active retention and zero-B SMALL parity remain **NOT_RUN**
at this A. Q is the separate `R3-FP4-E2M1-B32-F32S-EXPERIMENTAL` storage format
with F32 dequantized CPU/Accelerate execution. Codec evidence passes; its
11264 parity16 and paired256 quality, dequantization time and OS peakRSS remain
**NOT_RUN**. It is neither MXFP4/NVFP4 nor FP4 compute/RAM/S6 acceptance.
No unknown native call or automatic retry occurred. Initial delta is the last
durable SMALL artifact. The independent A receipt binds this immutable report
and preparation; downstream quality and independent B are separate gates.
