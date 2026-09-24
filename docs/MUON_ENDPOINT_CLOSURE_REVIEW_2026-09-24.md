# Muon endpoint diagnosis — independent closure review

Contract: R3-MUON-ENDPOINT-DIAGNOSIS-1.0. Date: 2026-09-24.

**Correction boundary PASS; D2 evidence VERIFIED; full diagnostic B BLOCKED_RUNTIME.**
The reviewed candidate has no additional confirmed defect requiring a patch.
The completed raw observations remain usable, but the first D3 teacher attempt
failed and closed the diagnostic plan. Unused call capacity is not retry authority.
No model reproduction, teacher forward, optimizer update or backward was executed
by this review. Neither failed run was reopened.

| Acceptance layer | Verdict |
|---|---|
| CODE_DELTA_REVIEW | PASS for the reviewed correction boundary; actual corrected teacher forward NOT_RUN |
| Existing A7 / Metal / unchanged A-delta fixtures | EXISTING_REUSED after identity and relevant source comparison |
| ORIGINAL_STUDY_STATE | FAILED_UNCHANGED |
| ORIGINAL_B_FULL | BLOCKED_AS_RECORDED |
| POSTHOC_AUTHORIZATION | Original separate observation plan remains bound; its failed execution is closed |
| D2 composite / train raw integrity | VERIFIED by a newly executed model-free independent reader |
| POSTHOC_EVIDENCE_COMPLETE | false; overall PARTIAL, not a completed diagnostic PASS |
| GENERATIVE_PARITY / independent teacher checks | BLOCKED_RUNTIME / NOT_RUN |
| M512_RENAMED_ACTUAL_ROWS | Historical10 plus new54 = composite64; historical panel remains10/64 |
| TEACHER_CONDITION / VALUE, FORMAT, ID, EOS, MIXED teacher metrics | Gold prefix only; no completed observations, metrics unavailable |
| MODEL_QUALITY / MUON_ADOPTION / GOAL1_READY | FAIL / NO / false |

## Reviewed and executed identities

- Reviewed correction source: `f5895057ba40e1758c72bf7dba5e37fef21d6445`.
- Actual D2/D3 producer: `4850195a1b5cb1d51f7f49fccae14ed194243f56`.
- Starting report HEAD: `18d83a69a255df6d9cfe6479425cfb29d31d49ad`.
  Its source/tests/Cargo/vendor match the correction candidate.
- Old learning source: `6d71d0418222039c7e23440cffaf78c09a26ce7f`.
- Diagnostic executor SHA256:
  `bb149a438d8aefad8a119048c026868502073ea14a2c9acf38a4ef0a1c04bb68`.
- Corrected candidate test executable SHA256:
  `fe1720ae7f9769fc5c7366e394c2d985b716700bf5fca39311be0d35c1ee1207`.
- Independent reader source / executable SHA256:
  `f1019c1b27e27d2ac8da63b282eb89772a2cd04d69b3abf45b6766193fc195b4` /
  `82707649664b2a4b9a87f3b37edf32d1fadba82fbd76d893c5d9090d81babc35`.
- Diagnostic plan physical SHA256:
  `49f9aaf5633fcdb7249985105b71c4672330c0208e60739fdb4eaf7ab2d5b920`.
- Diagnostic policy digest:
  `56b0583a7d9acb1b2d1ffe54142490b94c9388409d77715aa23ff2c46a5a054d`.

The exact existing locked/offline release executables were reused; no build,
dependency resolution or registry mutation was needed. The input test used actual
Metal and CPU tensor allocation, not CPU model inference. One-thread compute
environment and a scratch TMPDIR were used. Existing A7 and Metal receipts are
reused, not counted as new tests. Lock and patched Metal source still hash to
`a911931fc3baca7367a34c129673640661a8fc3d41458d49e4eb744ae5454154` and
`20ccaadf0f1b097b366636d48dbb79f4009d66da4abfcaefc37f2d6bd6b833d6`.
The optimizer, neural implementation, Cargo and vendor have no delta from the
previous accepted source in this change.

## Code and failure-path conclusions

The execution source allocated teacher input on CPU while the model was on Metal.
That confirmed historical integration defect is corrected at
`src/quality_recovery.rs:1420,1541`: the shifted U32 input uses `l.model.device`.
The regression checks actual device, dtype, shape, full response shift, empty
input and context limits. It does not execute the corrected model forward and
does not retroactively approve the failed D3 call.

The production path is `fresh muon diagnose` -> bound plan/ref checks ->
`segments` -> `observe` -> existing generation or teacher collection.
`src/muon_diagnosis.rs:29` accepts only the checked old RETURNED decode boundary;
different step, mixed runtime failures and unresolved calls are not equivalent
authorization. `load` checks frozen source/runtime and admission, while
`segments` at line152 rejects missing terminals and non-time/sticky failures.
`observe` invokes that check before opening another observation. Original
training/review control is unchanged; no global INTEGRITY_FAIL override exists.

Old and new rows retain separate source/hash/ordinal references. Missing-only
selection preserves the original invalid UTF-8 row. Train generation does not
invoke automatic teacher collection. The scorer retains strict text/EOS and
whole-word/support/outside/parse distinctions and complete pair denominators.
The teacher shift and byte-span role logic preserve gold-prefix conditioning,
MIXED tokens and EOS. Token divergence is separate from final text correctness.
Unchanged A-delta fixture evidence is reused for these boundaries.

No new current-code issue was confirmed. The remaining verification gap is the
actual corrected teacher caller followed by complete D3 and fresh B observations;
an input-allocation test alone is not that evidence. The closed failed plan does
not authorize filling this gap. No nominal patch or additional learning is requested.

## Executed in this review

Evidence root: `artifacts/muon-endpoint-20260924-closure-review/`.
Commands below ran from the repository root; all model counters are zero.

| Command / evidence log | Actual result |
|---|---|
| `target/release/deps/replica_train-a3e8f3bc867d1194 --exact training::recovery::tests::teacher_input_uses_native_device_and_response_shift --ignored --nocapture --test-threads=1` / `teacher-input-device.log` | 1 test, 1 PASS, exit0, reported test time0.01s |
| `artifacts/muon-endpoint-20260924-review-b/recount artifacts/muon-endpoint-20260924-diagnosis/run --failed-partial` / `recount-partial.log` | exit0, wall1.10s; reader code inspected, existing reader executed anew |
| `artifacts/muon-endpoint-20260924-diagnosis/replica-train fresh muon diagnose observe --root artifacts/muon-endpoint-20260924-diagnosis/run --lane replay-A` / `failed-observe.log` | expected exit1, wall0.89s, sticky execution failure before new observation/model call |
| Same executor, `fresh muon diagnose close --root artifacts/muon-endpoint-20260924-diagnosis/run` / `failed-close.log` | expected exit1, wall0.78s, no successful composite published |

The last two commands execute source4850195's unchanged state boundary, not a
corrected teacher forward. Both return
`diagnostic UNKNOWN/cancel/sticky execution failure`. These are two negative
process checks, not two additional Rust tests or failed model attempts.

## Independent raw recount

The reader verifies the plan's1,215 scoped original references, case/RETURNED
bindings, strict decoding, completed D2 lane finals, scores, exposure and failed
teacher/terminal records. It recomputes from raw rather than accepting summary
flags. New composite640 and train256 include13,946 output tokens; this includes
8,342 historical tokens and5,604 newly produced D2 tokens.

| Panel | A FULL | M FULL | A whole-value / support | M whole-value / support |
|---|---:|---:|---:|---:|
| V64 |64/64|64/64|n/a|n/a|
| VC64 |63/64|64/64|63/64|64/64|
| S1Q1-64 |54/64|40/64|64/54|64/40|
| word64 |0/64|0/64|14/0|0/0|
| renamed64 composite |0/64|0/64|15/0|0/0|
| word train128 |0/128|0/128|50/0|0/2|

Value/support columns are two counts, not a ratio. Complete dev panels have
QUERY_BOTH/SWAP_BOTH denominators32 and ALL4 denominator16; train denominators
are64/64/32. Word, renamed and train joint scores are zero for both models.
The historical M10/64 is never treated as a complete zero-score panel.

TRAIN_SEEN_EXPOSURES: per arm42 cases seen once and86 seen twice, all128 seen,
214 sampled exposures. No unseen sample is relabeled as trained. Both train
FULL scores are zero; failure cannot be attributed solely to unseen generalization.
Word and renamed share semantic bases and are not pooled as independent samples.
M renamed contains two strict UTF-8 failures and twelve length endings; outside
ID18 and parse48 are overlapping categories. Neither error repair nor lossy
decoding raises the official score. Gold-prefix role results are unavailable,
so prefix dependence versus intrinsic ID prediction remains unresolved.

Historical diagnosis usage is generation310 (54+128+128), teacher attempts1,
completed teacher examples0, active57.566452876s, optimizer/backward0.
This review adds generation0, teacher0, optimizer0, backward0. Reviewer normal/
failure replay128 and teacher24 remain NOT_RUN. S4/confirmation, QA640, FP4,
S5/S6, model export and learning were not performed.

## Preservation and publication

Before/after manifests verify162 tracked source/tests/Cargo/vendor files,
all639 files and membership in the diagnostic run, and17 reused logs/reports.
The prior19-entry scoped preservation ledger also matches, including both
endpoint checkpoints and the original failed terminal. This is not a whole
artifact-store audit. No missing teacher, observation004, replay or composite
record was created. The independent reader checks those absences explicitly.
Product files, original reports, source artifacts and untracked `.DS_Store`
were preserved. Logs and preservation manifests are local ignored evidence.
No build was performed, so new build bytes are0; the review evidence is small
text only, below the256MiB cap, with exact local size recorded in `footprint.txt`.

Only this report is published. Its report commit and actual remote SHA are
recorded after the normal push in the final handoff and local publication
receipt; they are separate from the reviewed/executed source SHAs above.
