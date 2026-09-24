# Independent B — protected adaptation; FP4 storage with F32 inference

**INDEPENDENT_B: PASS for execution/evidence integrity and bounded output
reproduction. P MODEL_QUALITY: FAIL. Q: no observed regression on the fixed
256-case paired sample. Goal1 remains incomplete.** No quality gate was relaxed.

Reviewed product source is `849d8bb39bebfd9a6e0194f60ad2680b63e2adf1`, source
digest `fa0080d2abbc7376f89c0df73560b4522756e779366f3d10607f95000fe7b952`.
The frozen production executable is
`7d707564bbeb54fa8d77695e782633139740ba64534ee72661cd395c64c444fd`.
Source/Cargo remain identical to that commit. The independent A report and its
receipt are unchanged. This B changes only independent local evidence and this
report; publication is a separate report commit.

**P closure and reproduction.** Original raw/control/native/trace records show
128 optimizer updates, ending at effective step14464 / adapter clock128.
All three segments saved successfully with no cancellation, UNKNOWN, nonfinite,
storage or trace error. The terminal is Finished, resume=false,
`SEVERE_RETENTION_REGRESSION`. No further training is permitted by this run.

| Final fixed64 panel | FULL | QB / SB / ALL4 | Value / support | Outside / parse | EOS |
|---|---:|---|---|---|---:|
| V | 63 | 31 /31 /15 | 63 /N/A | N/A | 64 |
| VC | 54 | 22 /26 /10 | 63 /55 | 9 /0 | 64 |
| S1Q1 | 17 | 1 /5 /0 | 53 /21 | 40 /2 | 64 |
| Word | 0 | 0 /0 /0 | 0 /0 | 0 /64 | 64 |
| Word renamed | 0 | 0 /0 /0 | 0 /0 | 0 /64 | 52 |

The parent fixed64 V/VC/S1Q1 scores were64/64 and ALL4 16/16. At+64 they were
64/64/62 with ALL4 16/16/15, with no warning. At+128 S1Q1 loses47 full answers
and16 ALL4 groups, meeting the severe stop rule. The three warning counters
become0/1/1; immediate severe failure does not require a second warning.

The renamed panel's twelve reported `errors` are **normal length stops**, each
generation_completed=true, finish=length,32 output tokens and error/error_class
null. Runtime/UTF8/UNKNOWN errors are0. They remain quality failures: no EOS was
forced and no output was corrected. All640 evaluation generations and640
teachers have matching prepared/RETURNED journals, with no gaps or reruns.

The independent reviewer first checked admission without model calls, then ran
the frozen executable once:

```text
replica-train fresh word-review --study ABSOLUTE_REGISTERED_STUDY
VECLIB_MAXIMUM_THREADS=1 RAYON_NUM_THREADS=1 OMP_NUM_THREADS=1
```

The existing selector chooses42 rows: the first complete query pair from each
of five panels plus32 additional failed-pair rows. All42 outputs, raw token
sequences, finishes and error states match the saved endpoint, including26
incorrect answers. Exit0; generation42, teacher/optimizer0;662 output tokens,
3.835571292s registered active time. It is neither a fixed32-row test nor a
rerun of all failures. The original length-stopped rows were audited but were
not in this predetermined42-row reproduction. No automatic retry occurred.

The independent reader recalculates every actual128 tape draw, mask-derived
input/target/padding cost, ANSWER denominator8 and finite training loss, and
validates all640 raw/teacher rows at both checkpoints. Actual exposure is
V128 / VC0 64 / VC1 64 / bridge256 / word512. A/B gradient, before/after and
delta norms are finite in all128 saved traces; first A gradient is0, first B
gradient/update is nonzero and the second A gradient/update is nonzero. These
are recorded optimizer observations, not new backward calculations.

| P accounting | Optimizer | Generation | Teacher | Output tokens | Active seconds |
|---|---:|---:|---:|---:|---:|
| Training/evaluation plus zero-B parity16 | 128 | 656 | 640 | 9,827 | 200.642085917 |
| Independent fresh reproduction | 0 | 42 | 0 | 662 | 3.835571292 |
| **P total** | **128** | **698** | **640** | **10,489** | **204.477657209** |

Training consumption is separately **input186,752 / target14,976 /
padding19,072**. No discarded or UNKNOWN execution is present. The full
+1,024 development endpoint and conditional word-fit1536 were not reached.
QA640, confirmation and S4/S5/S6 were not run.

**Base versus active model.** The14336 base physical hash remains
`15015900a3c91a6b8ed87e9c2ecb8f7faf124849e5dcf8f40203cddf267e30d9`.
The delta contains24 adapter tensors /59,904 parameters /239,616 F32 weight
bytes and48 Adam tensors /479,232 bytes. No historical base Adam is updated.
Final delta is733,966 physical bytes:

- Physical: `de9064bffc95aa8f09e5653c2878c0ee44ad8bd17349f746a2e9409532bcb9a2`.
- Combined evaluator/manifest weights: `c68d65f8ec67e61c6599425887a83e8ee14bd5cd10d9d1b411d7f04657760190`.
- Combined tensor content: `cea58acd66923bf43dceba678906e492668ad2b45cda4cf64e8cc98ba8e4b7df`.
- Adapter Adam: `261b34c551f85f43cf13f0c762fad3fe17c40a2179a783ba84f9b520c9a50ede`.

Base bytes preservation passes; active retention and word learning fail. A
constant base does not imply constant behavior of base plus adapter. This run
also changed trainable parameter space, Adam, LR and decay relative to the
historical full-tune failure; these results do not isolate a single cause.

**Q independent verification.** Q uses the accepted11264 physical parent
`c47e34c7ac4f88dd08b5e719bf4d0364f139ac5002bf4572fe55b37dac823925`,
not the failed adapter. Original bytes and Adam are preserved. The reader
checks the actual packed registry, all scales/codes/tails/checksums, unchanged
F32 embedding/norm tensors, tensorwise max/MSE and saved normal-greedy rows.
No Q generation was repeated by the reviewer.

| Fixed sample | Float FULL / QB / SB / ALL4 | FP4→F32 FULL / QB / SB / ALL4 | EOS / outside / parse | Delta |
|---|---|---|---|---|
| V128 | 128 /64 /64 /32 | 128 /64 /64 /32 | 128 /0 /0 | 0 |
| VC128 | 128 /64 /64 /32 | 128 /64 /64 /32 | 128 /0 /0 | 0 |

Citation value/support are128/128. Original parity16 also matches. Total Q
generation272, teacher/optimizer0, output2,584 tokens, registered active
17.771682542s, UNKNOWN/runtime0. This sample has no observed regression; it
does not establish future errors0, full QA quality or S6 acceptance.

Format is **R3-FP4-E2M1-B32-F32S-EXPERIMENTAL**: four-bit codes with one F32 scale
per32 elements. It is not MXFP4/NVFP4. Actual quantized parameters are
9,289,728 of9,513,408;223,680 remain F32. Nibbles occupy4,644,864 bytes,
scales1,161,216 and untouched F32 tensors894,720. The one derived file is
6,713,446 bytes. The saved normalized-absolute-value>6 clipping counter96,830
matches independent F32-scale reconstruction. No source weight is overwritten.

The previously executed synthetic3×32 scalar/layer regression is reused as
primitive evidence. To connect this to the real11264 tensors, the reviewer
also computes only `layer.0.q` on the fixed non-answer vector
`x[i]=((17*i mod31)-15)/16`: two F32 matrix products,384 outputs, maximum
absolute output error0.16097934544086456 and RMS0.038130663026841184.
Whole-model forward, generation, teacher, optimizer and calibration are0 for
this numeric check. It has no independently chosen quality tolerance.

| Q command | `/usr/bin/time -l` wall seconds | Peak RSS bytes |
|---|---:|---:|
| Prepare | 67.35 | 4,259,446,784 |
| Float parity16 | 37.62 | 4,829,315,072 |
| FP4-dequantized256 | 52.97 | 5,069,979,648 |

Recorded dequantization plus base verification is1,075.513166ms; the observation
and finalization interval is16,266.596916ms. End-of-run RSS256,688KiB is a
different measurement from OS peak RSS. Computation uses dequantized **F32 CPU
Accelerate**, with no FP4 kernel, training, RAM-saving or speedup acceptance.

**Evidence and limits.** Independent tools reused existing readers/scorers and
the shared compiled library; Rust builds and reader executions exited0. No
closed A test, earlier learning experiment, teacher or full model forward was
rerun. The P inspector, P recount/reproduction readback and Q reader all exit0.
Their source/executables/logs are retained under
`artifacts/protected-adaptation-20260924-review/`; original journals remain in
the registered study and Q roots.

| Independent / frozen evidence | SHA256 |
|---|---|
| P `B-recount.r3b` | `93a917ac81215edc709f22db578ead460daef06e589c9ee21f1df672d92bd427` |
| `B-reproduction.r3b` | `622e99babdec639a21b783c81ef9b79212b674d42137bc66d9fec062ee16ba48` |
| `B-reproduction.log` | `7b277e52d0f20dccb218cbcd21bedddc7c08251244ccacce08f3985065138184` |
| P reader source / executable | `dfce0d25e225266bca97291b0611aab55efb9cc7aa96dc19084639952ba27085` / `d677069292fb0309e87d68be71ffcc715668c7e0c02beb1f635a831d91f12157` |
| `Q-recount.r3b` | `3dc17a0e601a7271f25969c90db8b9b50683361cfef47ab278827d9d41650dfc` |
| Q reader source / executable | `675bdc57dbe56ba88612fd4925e930f6564a676a874bb77d7803dca2d877f286` / `d581455d9ff44ac8cc5ee4be7c47e5dbbda73ed808e042f38cc5a60a12353ba2` |
| Q `model.r3b` | `5b9b502f5ea3afc6175d9b07e9311806241b3560269846f1729d4f1674bca37b` |
| Q `preparation.r3b` | `ca500de8201d44885bb336302542e6ead7683949ddc1b4a0b7466a2cc4c5da00` |
| Q original `result.r3b` | `8cbfff3837f6806ff6d04b2e30b6f10ac9b6f2667e43d50c056a4a950059cbeb` |

Final SMALL P+B+Q totals are **optimizer128 / generation970 / teacher640** and
**13,073 output tokens**. TINY remains the previously closed18/180/180; no
new TINY work was performed in B. Deleted/moved/recompressed original bytes0.
New experiment/cache size accounting belongs to the implementer's bounded
owned-root ledger; this review does not re-inventory or claim reclaimed space.
No further model calls, training, fit, QA or confirmation are authorized by
this report. Final model verdict stays FAIL, and Goal1 remains false.
