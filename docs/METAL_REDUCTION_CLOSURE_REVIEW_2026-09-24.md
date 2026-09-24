# Metal reduction repair — independent closure review

Contract: R3-METAL-REDUCTION-REPAIR-1.0. Date: 2026-09-24.

**RESULT: PASS within the registered Runtime A/B boundary.** Nine newly executed
candidate tests passed. A new, model-free B audit reproduced the existing audit
and readback receipts byte-for-byte, including wrong outputs. No confirmed defect
requiring a product patch was found. This closes the requested review scope;
it does not accept general QA, S4/S5/S6, Goal1 or further SMALL training.
Historical model-quality failures remain unchanged; this is numerical/runtime acceptance.


| Decision | Result and evidence type |
|---|---|
| Runtime A / public sum / Broadcast VJP / QKV gradients / TINY update | PASS — actual new CPU/Metal tests |
| Runtime B / protected quality / native restart / cache | PASS — source review, identity-bound existing execution and new pure readback |
| Original strided shader fixed | **false** — replaced by equivalent GPU copy plus contiguous reduction |
| Actual Metal device / CPU fallback | registry4294968525, F32 / 0 in tested GPU paths |
| Training admission | Registered parent/runtime only; generic Metal Adam remains blocked |
| SMALL48 completed | Existing 48 updates verified; new reviewer SMALL updates0 |
| Performance | Existing measured gain on the specified workloads; no new benchmark |
| LegacyB | Existing acceptance preserved; new calls0 |
| General QA / S4,S5,S6 / Goal1 | NOT_ESTABLISHED / NOT_ACCEPTED / NOT_ACCEPTED |

## Source and execution identity

Reviewed and newly compiled source: **cef33c777a1db32e3937046d401a9927bbf87046**.
The initial report HEAD and actual remote main were
`d815161724fadef4402fb0af59479c0bf033faa8`. The differences after the candidate
were only two report documents. This review's publication commit is distinct
from the reviewed/executed source; its exact commit and remote match are reported
in the publication receipt and final handoff, not embedded recursively here.

The G4/G5 source is identical to candidate code: `4556b6d` plus retained tracked
diff SHA256 `80abfe67a9b6a4b781acb5d77b7b27cb746c54d1463d74cd699fe9133acc90d5`
and new module SHA256
`b8a0289d09c4b53add05cca263e5479d22d8e5b13ed92a8acd2afa5567c131f3`.
These were recomputed from the candidate, not inferred from filenames.

| Identity | SHA256 |
|---|---|
| Locked upstream Candle0.11.0 archive | `5ecb245093b0f791b89d3420c3df9c6d49c60ab63ba54db896bf8a3baf486706` |
| Candle patch | `34faeb37febb834770c5679093b2f22d26c6eff01f40154a7854691b76ca43b2` |
| Patched Metal source | `20ccaadf0f1b097b366636d48dbb79f4009d66da4abfcaefc37f2d6bd6b833d6` |
| Cargo.lock | `a911931fc3baca7367a34c129673640661a8fc3d41458d49e4eb744ae5454154` |
| Newly built training test executable | `1be702e8a05917909f5c0a0d51cf0c39a4cdf3a0982e87d54f0185b5b4d29f84` |
| Newly built native test executable | `46c9f38f22b2edbc0d50255d04dbf7c9c4b1b4bd0cf7cf37f2cc1d72b82376b5` |
| New pure Rust reader | `ba375aafb5c68b830cc1822ad86ee9d62601bb8dffa3834d3c6f1a372b3bd32b` |
| Reader source | `b0a04f75f8baf17ae657376ebe7c74b9b9bc05bae4f4359fbb5d77785db861f5` |
| Existing G45 trainer, reused evidence | `95c94f2892bcb3475f98fead9a1f2cb44bc9b0c3d31c4a8b295d7c074da4e1ba` |
| Existing G45 worker, reused evidence | `3b7d342b2ba65e3f76b16d8eca245c5378a98d5795a77e6de5993438dd385d1a` |

The shared registry Metal source matches the retained published archive bytes
(`015ef2c128a54ea4fb2cdb5705ee26101509de7e17182e4877fc50de1b167758`).
Registry/vendor comparison found only that source delta, patch/provenance files
and the registry marker. The package graph resolves Replica and candle-nn to the
same vendored candle-core. The single vendor directory was referenced read-only;
it was not duplicated or modified. Upstream revision remains
`31f35b147389700ed2a178ee66a91c3cc25cc80d`; license is preserved.

One old checksum-manifest pathname needed interpretation: its `review.rs` entry
predates fresh32 additions. The recorded checksum matches the preserved
`review-audit-source.rs`; the frozen audit executable and all other entries
matched. The current reader source and fresh executable have separate identities.
This was resolved from retained bytes, without changing old records.

## Production boundary and newly executed A

Inspection covered `MetalStorage::reduce_op`, public sum/Broadcast backprop,
unchanged `repeat_kv`, actual GQA, Adam's guard/shared update body, native runtime
profile, worker deadline/accounting, decoder timing and bounded runtime commands.
Only nonempty F32 Sum entering the strided branch takes the GPU permutation/copy
and suffix-reduction replacement. Offset and zero strides are preserved; checked
allocation and the contiguous base case prevent an unbounded recursive retry.
There is no CPU tensor reduction, fixture-specific return, detached backward or
changed head grouping. Other dtype/op and existing zero-buffer errors retain
their prior dispatch semantics. The original strided shader is unchanged.

A source copy was extracted from the exact commit; source/tests equality was
checked afterwards. Rust1.98.1, `--locked --offline --features accelerate,metal`,
`CARGO_INCREMENTAL=0`, existing matching target cache, scratch TMPDIR and one
compute thread were used. No product source, tests or Cargo file was changed.
The no-run build exited0 in23.97s. The following tests each ran once, one test per
process, with `--exact --nocapture --test-threads=1`; six numerical/admission tests
also used `--ignored`. Zero-test invocations were not counted.

| Actual test | Count / exit | Wall seconds |
|---|---|---:|
| metal_f32_middle_axis_sum_oracle | 1 passed /0 | 4.44 |
| metal_f32_reduction_layouts | 1 passed /0 | 0.08 |
| metal_f32_repeat_and_qkv_vjp | 1 passed /0 | 3.81 |
| metal_f32_attention_backward_boundary | 1 passed /0 | 0.06 |
| metal_f32_tiny_forward_gradient_update | 1 passed /0 | 1.70 |
| metal_f32_admission_is_fail_closed | 1 passed /0 | 0.08 |
| native_checkpoint_roundtrip_and_corruption_rejection | 1 passed /0 | 0.52 |
| external_call_and_no_call_finalization_fail_closed | 1 passed /0 | 0.00 |
| runtime_profile_is_execution_only_and_fail_closed | 1 passed /0 | 15.29 |

The original dyadic oracle matched exactly on CPU and Metal. Layout22 covers
production shape, first/middle/last/multiple axes, transpose, narrow offset,
broadcast zero stride and malformed/empty cases. Analytic K/V VJPs for groups2/4
matched exactly. Actual GQA Q/K/V gradient NRMSE was approximately
9.38e-8/8.80e-8/4.18e-8, with cosine above0.99999999999999.

All24 TINY parameter gradients passed TOKEN/ANSWER and padded microbatch4+4
checks with their actual denominators. First Adam weights and48 m/v tensors
passed. CPU/Metal gradient norms were2.777685288999244/2.7776853049488834;
update norms0.0049414033136463776/0.004941403451688341. No tolerance changed.
Invalid backend/profile/cache and unadmitted Adam were rejected; completed
no-call TIME finalization was allowed while cancellation/call-limit failure
remained blocked. These scoped tests do not replace every historical fault suite.

## Newly read B evidence and preserved prior executions

The Rust reader reuses the reviewed independent scorer/state-check logic, writes
only to this new scratch root, and adds a direct comparison of every retained
fresh32 row with original endpoint raw and the native corpus, including wrong
answers. It performed no model forward/backward, generation, teacher or optimizer
call. It exited0 after223.59s of file/native readback; this is not GPU execution
time. Its freshly emitted receipts exactly match prior bytes:

- Audit: `ccd238da1e6f2e9690c44b3e0c18b7f6678d478c13c45bae51f76df47f8aaada`.
- Readback: `c39b51e1816f29ef4a2f46ca960078a8b11794f60a4aa0adb85d07d0c8ede5ea`.

Protected CPU/Metal128 scored128/128 with identical tokens/EOS and no historical
CPU-correct loss. Challenge64 scored32/64 each. Worker4+4, cache boundaries through
2048 and their original successful execution receipts remain verified.
CPU16/Metal16/Metal1+15 consumed exactly48 updates; same parent14336, inherited
Adam, LR3e-5, ANSWER/batch8 and first16 tape entries. Total input71,328,
target5,616 and padding5,856 were recomputed. All endpoints are step14352.
Metal continuous/split weights,136 Adam tensors, cursor and trace loss/clip are
exactly equal. Cross-backend first-step metrics pass;16-step drift is reported
separately, without imposing a new bitwise requirement.

All three endpoint panels remain12/16. Prior fresh32 matches32/32 original outputs:
24 correct and8 wrong,436 tokens. It was **not regenerated this run**. The
previously completed fresh process, no-call close and actual1+15 restart are
reused evidence after identity checks; no new SMALL training is claimed.

Existing synchronized timing was independently recomputed from raw: generation
median0.968259s CPU versus0.529780s Metal (1.83x), n24/backend after warmup;
update median12,882.219ms versus806.692ms (15.97x). This applies only to the fixed
debug build, one CPU thread and recorded request/batch lengths. Device syncs bracket
measured work. First-update comparison overhead remains in the tail; load,
TTFT/prefill/decode and actual output lengths are separate. GPU allocator peak,
model-wide canonicalization copy bytes and isolated shader preparation remain
UNKNOWN. The existing primitive temporary/copy measurement is24,576B.

## Usage, preservation and closure

New review usage: **TINY optimizer2, whole-model forward19/backward12;
primitive successful sums63 plus4 rejected invalid-axis calls, VJP backwards22**.
New generation/teacher/SMALL optimizer/SMALL backward/LegacyB calls are all0.
The nine test processes total25.98s; profile/CPU/file work is included, so this
is a conservative wall duration, not measured GPU time.

Updated contract ledger: TINY optimizer6/64, generation4/96, diagnostic
forward57/64, backward36/64; explicit successful reductions225/256 and VJP66/192.
SMALL totals stay536/584 generations,48/48 optimizer updates and2 diagnostic
backwards over16 teacher-forced samples,6,710 generated tokens. No UNKNOWN or
retry was added. Conservatively adding all25.98s to the prior total including
its120s reserve gives1,481.643630291s, below7,200s; compile/readback are separate.

All539 paths in the existing scoped preservation map matched before and after.
The earlier five protected originals, old failures and hash-bound A/B reports
also match. Product/tests/Cargo/vendor remain unchanged; existing `.DS_Store`
is preserved. No original artifact deletion, inventory, model copy, whole QA640,
FP4256, LegacyB, seal or unrelated regression suite was performed.

New scratch logical size at measurement was about98.43MB, allocated98.66MB,
including94,625,680B of frozen executables. Vendor is not counted again.
Together with the previous conservative approximately1.06GB path-total this
remains below1.5GiB. Matching target allocated size rose from10,499,092KiB to
10,548,284KiB (+49,192KiB) during this isolated build. This is this invocation's
observed target delta; whole-contract build growth remains UNKNOWN without an
original baseline. It is not reclaimed space. No clean or dependency update ran.

Evidence is local under `artifacts/metal-reduction-20260924-closure-review/`:
`commands.txt`, build/test logs, identity checks, `reader.rs`, `b-reaudit.log`,
new audit/readback receipts and preservation checks. The existing Runtime A/B
reports and original runtime evidence remain intact. Only this report is to be
committed/pushed. No additional mandatory test or product repair is identified
within the completed scope.
