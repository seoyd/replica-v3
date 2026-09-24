# Muon endpoint diagnosis: independent partial B review

Date: 2026-09-24. Contract: R3-MUON-ENDPOINT-DIAGNOSIS-1.0.

**D2 evidence VERIFIED; full B BLOCKED_RUNTIME.** The first D3 teacher call failed
before producing logits. This report does not approve a continuation, candidate,
Muon adoption, S4/S5/S6, or Goal1. The original failed study and the new failed
diagnosis remain closed. No B model replay or teacher check was executed.

## Identities and scope

- Observed execution source: `4850195a1b5cb1d51f7f49fccae14ed194243f56`.
- Frozen executor SHA256: `bb149a438d8aefad8a119048c026868502073ea14a2c9acf38a4ef0a1c04bb68`.
- Plan file SHA256: `49f9aaf5633fcdb7249985105b71c4672330c0208e60739fdb4eaf7ab2d5b920`.
- Policy: `56b0583a7d9acb1b2d1ffe54142490b94c9388409d77715aa23ff2c46a5a054d`.
- Source digest: `7b7a4a7a24f0410cefad313d46d7f0811152a0df9779584a9ce9c2a73cad6699`.
- Reviewed correction source: `f5895057ba40e1758c72bf7dba5e37fef21d6445`;
  its verification below uses no model calls. It did not produce the D2 outputs.
- Actual recorded runtime: Metal0, `Metal { gpu_id: 4294968525 }`, F32,
  macOS 27.0 build 26A428, existing `metal-f32-sum-contiguous-v1` patch.

The independent Rust reader checked the plan's 1,215 scoped references, frozen
corpus/tokenizer/metadata, original and new row bindings, resolutions, complete
D2 lane finals, strict decoding, metrics, pair denominators, training exposure,
and the failed teacher/segment records. It did not load a model for inference.
Original 586 RETURNED rows were reused; only the missing 54 were newly generated
to complete the two endpoints' 640-panel diagnostic. A/M remain the saved
absolute-step 14848, local-update 512 models.

## Confirmed issue and correction

**Severity: Medium.** In execution source `4850195`,
`src/quality_recovery.rs::teacher_observation` constructed the teacher input on
`Device::Cpu` (around line 1425). The caller had loaded the model on Metal.
`src/neural/transformer.rs::forward_inner` rejects inputs whose device differs
from the model (current lines 746–757). Thus this Metal teacher path fails
deterministically before attention/linear computation, with:

`invalid input: native input shape/dtype/device/context/cache identity`

The first failed case was `qa-word-value-v1/train/0/id0/0`. Its journal records
one entered teacher attempt and no successful teacher result. This is an input
device integration defect, not evidence of a Muon formula error, nonfinite GPU
arithmetic, or corrupted weights. The earlier A-delta tests covered shift/roles,
receipts, and no-call state boundaries but missed this actual device connection;
that earlier report is preserved, not retroactively rewritten.

The correction in `f589505` makes production `teacher_observation` use
`teacher_forward_input(..., &l.model.device)` (current line 1420). The helper
at line 1541 constructs the full shifted U32 input on the actual model device,
rejects empty/oversized input, and preserves the original causal target positions.
No model math, gradient, optimizer, or generation policy changed.

The independent exact regression
`training::recovery::tests::teacher_input_uses_native_device_and_response_shift`
passed **1/1, exit 0, 0.01 s**. It checks CPU and actual Metal tensors, shape,
dtype, device, full answer shift, and empty/context boundary rejection.
New model/teacher/backward/optimizer calls: **0**.
Corrected file SHA256:
`d3493dbc42887737a7c5faa53ad48b6c2f14b3a398b1f49e0f384ed04845d7dc`;
test binary SHA256:
`fe1720ae7f9769fc5c7366e394c2d985b716700bf5fca39311be0d35c1ee1207`.
Actual corrected teacher execution remains **NOT_RUN**. The failed plan was not
reopened to test it.

## Independently recounted results

Each development panel contains 64 rows; QUERY_BOTH/SWAP_BOTH denominators are 32,
ALL4 is 16. Train panels contain 128 rows, with pair denominators 64 and ALL4 32.
For value-only panels, separate citation-parser value/support metrics are not
applicable; their exact outputs are represented by FULL.

| Endpoint / panel | FULL | QUERY_BOTH | SWAP_BOTH | ALL4 | String value | Exact support | Outside ID | Parse error | EOS | Generation errors |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| A / value | 64 | 32 | 32 | 16 | — | — | 0 | 0 | 64 | 0 |
| A / citation | 63 | 31 | 31 | 15 | 63 | 64 | 0 | 0 | 64 | 0 |
| A / S1Q1 | 54 | 22 | 26 | 10 | 64 | 54 | 9 | 1 | 64 | 0 |
| A / word | 0 | 0 | 0 | 0 | 14 | 0 | 64 | 0 | 64 | 0 |
| A / renamed | 0 | 0 | 0 | 0 | 15 | 0 | 64 | 0 | 64 | 0 |
| M / value | 64 | 32 | 32 | 16 | — | — | 0 | 0 | 64 | 0 |
| M / citation | 64 | 32 | 32 | 16 | 64 | 64 | 0 | 0 | 64 | 0 |
| M / S1Q1 | 40 | 17 | 19 | 8 | 64 | 40 | 13 | 11 | 57 | 7 |
| M / word | 0 | 0 | 0 | 0 | 0 | 0 | 20 | 50 | 52 | 12 |
| M / renamed | 0 | 0 | 0 | 0 | 0 | 0 | 18 | 48 | 52 | 14 |
| A / train128 | 0 | 0 | 0 | 0 | 50 | 0 | 128 | 0 | 128 | 0 |
| M / train128 | 0 | 0 | 0 | 0 | 0 | 2 | 64 | 66 | 115 | 13 |

Other-provided-ID counts were zero throughout. Outside-ID and parse-error counts
may overlap; they are not exclusive categories. M renamed has two strict UTF-8
failures and twelve length endings; its 14-error union preserves both. Other M
generation errors in the table are length endings. The historical invalid UTF-8
row remains unchanged. No generation runtime error was found in these D2 rows.

Each train128 sample set contains 42 cases exposed once and 86 exposed twice by
the original 512-update tape. All are seen cases. Both have zero FULL and zero
joint scores, so these samples do not support an explanation solely in terms of
held-out generalization. A has some correct word values but no exact support;
M has no correct word values in these samples. These are bounded observations,
not a whole-corpus fit claim or a proven learning mechanism. Word and renamed
panels share bases and must not be pooled as independent examples.

## Usage and terminal integrity

| New lane | Generation calls | Teacher attempts / results | Active seconds |
| --- | ---: | ---: | ---: |
| missing | 54 | 0 / 0 | 19.268115625 |
| train-A | 128 | 0 / 0 | 17.898014583 |
| train-M | 128 | 0 / 0 | 19.727726584 |
| teacher-parent | 0 | 1 / 0 | 0.672596084 |
| Total | 310 | 1 / 0 | 57.566452876 |

New output tokens: **5,604** (missing 1,074; train A 2,050; train M 2,480).
Reused 586 rows: **8,342** output tokens. Composite640 plus train256 therefore
contain 13,946 output tokens; that sum is not newly generated usage.
Optimizer/backward calls are zero. The reviewer made no generation/teacher calls.

`observation-003-finished.r3b` is `success=false`, `resume=false`, INTEGRITY_FAIL.
Its SHA256 is `525b82accad293c8a37eeddbf6d2c68d6074290feca2fc1ac2ab2e14b27d6edb`.
The failed teacher-row digest is
`814debc4b71bc32598257aa6fd3416c61393fa0f1bd9955d98f1420c704479a3`.
No completed teacher-parent lane final, later observation, or successful final
composite was manufactured. The original study's failed terminal still hashes to
`7eda204d83b5f32f77635ac6b0929a73988b1747ba1315fc08dbe374b6a9c7af`.

Teacher192, reviewer replay128, and reviewer teacher24 are **BLOCKED_RUNTIME**.
Arithmetic unused capacity is 128 generation and 215 teacher attempts; it is
**not permission to retry** this failed plan. Teacher NLL, free-vs-teacher
divergence, and role-specific teacher conclusions are unavailable. Any future
actual teacher diagnosis requires a separately authorized run; no extra learning
or optimizer change is justified by these missing measurements.

## Local evidence

- Frozen execution and plan: `artifacts/muon-endpoint-20260924-diagnosis/replica-train`
  and `artifacts/muon-endpoint-20260924-diagnosis/run/`.
- Independent reader source: `artifacts/muon-endpoint-20260924-review-b/recount.rs`,
  SHA256 `f1019c1b27e27d2ac8da63b282eb89772a2cd04d69b3abf45b6766193fc195b4`.
- Pure reader command: `recount artifacts/muon-endpoint-20260924-diagnosis/run --failed-partial`;
  exit 0, 1.57 s. Log: `artifacts/muon-endpoint-20260924-review-b/recount-partial.log`,
  SHA256 `47e4a4719c2faff5c93acd279ec35119d48393641c90d644116f352b8d17bd25`.
- Exact input primitive test log: `artifacts/muon-endpoint-20260924-review-b/teacher-input-device.log`,
  SHA256 `dc29fd717fe5d5e1c7cf717593aef05e1129a2a445e07c0f1a72d7f18ced4036`.

The reviewer changed no product source or original artifact. Only isolated Rust
reader/build/log evidence and this report were written. Correction verification
is PASS at its input primitive boundary; complete diagnosis/B acceptance is not.
Protected11264 and prior narrow acceptances remain unchanged; Goal1 remains false.
