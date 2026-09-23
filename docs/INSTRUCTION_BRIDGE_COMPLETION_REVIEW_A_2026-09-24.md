# Independent A — instruction bridge completion

**INDEPENDENT_A: PASS.** The changed parent/fork, final-fit and diagnostic-QA
boundaries, and the actual preparation are accepted for this bounded study.
SMALL optimizer/generation/teacher calls remain0 at this review. This is neither
model quality acceptance nor B/QA640/S4/S5/S6/Goal1 acceptance.

Reviewed source: `c502e66872ad6abdf463990c5992061d3ec3a19c`.
Source digest: `dab51655d7ad6a5bd4c9f9ffcd0906efa80f9f4971dc93719bd234909021e434`.
Diff base: `a09273df7613af61184afb0164a19c54b190bb30`.
Reviewer changes are confined to scratch evidence and this report; product
source/tests/Cargo and original artifacts remain unchanged. The report commit
is separate from the reviewed source.

| Bound artifact | SHA256 |
|---|---|
| `candidate.diff` | `cc89fc7676fe02204b66b3a14caf956f9d0a112d5ef1f6dcf879467f422660fe` |
| Frozen production executable | `ffffb7f2f2dab68f04a73d579f65fe8d55b9177c00244705d7bd12788c472713` |
| Source-bound test binary, independently copied/executed | `530a7e52a98a19b4f1134505b2cdeb691114189595e4819a9ddc0f773443eff7` |
| Study preparation | `755c60e07729eb4f3712ce1574ab05a2396024e07e2b2ae40a7d4fe483d151a5` |
| Study selection | `e6c49f2394614c6fcfff8b5083065716632ff61e2cca1f7d3ce1ddbbf0fa7c55` |
| `ANSWER-MEAN/plan.r3b` | `9fd65bb560f985f0ce325fe31ad0931592e2af3463b46ea283004ff7f96f20f1` |
| Exact12800 parent and copied initial native | `80970cee2ca6dbeb258bc7f202e612f6e178c9185c590acc23c99b6941a98d73` |

**Executed independently.** The implementer's newly built locked/offline release
Accelerate/test-support binary was copied and hash-checked. This reports actual
independent execution of that binary, not an additional Cargo build. The fixed
source was separately archived under `A-source`; `TMPDIR` and all fixtures were
isolated, with compute threads1.

| Exact suffix under `training::fresh::identifiable::binding::citation::tests` | Result | Wall seconds | New optimizer / generation / teacher |
|---|---|---:|---|
| `bridge_completion_raw_gate` |1 PASS, exit0|139.37|0 /0 /0|
| `bridge_completion_native_process` |1 PASS, exit0|86.32|TINY6 /136 /84|

Both commands used `A-bin/replica-train-tests --exact FULL_TEST_NAME --nocapture
--test-threads=1`; the process command additionally used `--ignored` and:

```text
R3_BRIDGE_TEST_PARENT=/Users/seo/Projects/Replica-v3/artifacts/qa-integrity-bridge-20260923-tiny-02/continuous/ANSWER-MEAN
R3_BRIDGE_QA_FIXTURE=/Users/seo/Projects/Replica-v3/artifacts/fresh-joint-20260919
R3_BRIDGE_TEST_ROOT=/Users/seo/Projects/Replica-v3/artifacts/instruction-bridge-completion-20260924-review/A-process
```

The typed writer→reader→scorer→decision fixture checks intermediate/final quality
failure, final fit despite failed dev, missing/failed fit, malformed and valid
outside-ID simultaneous counts, and different native weights at the same step.
The actual process test compares continuous2, new-process1+1 and final
evaluation-only2+0 weights/Adam/clock/sampler/raw. A fault during final fit leaves
EvaluationPending; the new process preserves RETURNED rows and adds optimizer0.
Diagnostic QA resumes the same cursor after two returned rows, completes only
the remaining two, and its completed reentry makes no new call. Missing command
final, corrupt receipt, cancelled command and mixed cancellation per-call receipt
are rejected in separate processes. Candidate-only QA, confirmation and training
resume remain rejected for the complete negative endpoint. TINY/record fixtures
are execution evidence, not SMALL quality evidence.

**Actual preparation.** The read-only Rust reader loaded the preserved native
12800 and verified136 finite Adam tensors, cumulative clock/tokens, unchanged
QE/tokenizer/model/config, LR3e-5 bits and objective family6/normalizer2. Its
original Finished/BRIDGE_DEVELOPMENT_FAIL/resume=false and independent B/report
binding are unchanged. Adam digest is
`cc340e07fcb5c8dd667bb72d160a528f2131d214e9cac56d3fa9674522ad00e7`.

Corpus6144/dev3072, metadata, transfer and tokenizer are byte-identical to the
parent. All12800 prefix rows and all1536 repeated suffix rows match, including
an independent comparison with the registered bridge ordering. Actual prompt/
target masks include EOS and exclude prompt/padding; V/VC0/VC1/bridge exposures
are3072/1536/1536/6144. Prepared cost is input2064384, target162816, padding233472.
The new origin is12800, cap14336, evaluations13056/13568/14336; no old calls are
charged as new work. These are prepared costs, not executed SMALL updates.

Original old-QA primary512/transfer128/metadata hashes match the parent selection,
with unchanged ordered cases and context2048/max128/timeout120000. The accepted
prior11264 QA audit and its two physical raw hashes were reused, not recounted or
regenerated. Parent12800 QA remains NOT_RUN. No used confirmation or S4 body was
opened. All consumed protected-file hashes matched after verification.

**Failed work retained.** Implementation process01/02/03/04/05 actually consumed
TINY0/24/0,2/48/24,6/108/84,6/134/84,6/136/84 respectively; only05 passed.
Failures were preparation policy-hash representation, omitted TINY fit routing,
test-process duplicate Ctrl-C handler and a wrong zero-based fault-prefix
expectation. Their receipts/logs remain preserved. Independent receipt recount
gives implementation20/450/276 plus this review6/136/84 = **26/586/360**, within
64/1024/1024. This review did not repeat previous A1/A2/B, probe80 or replay44.
The independent native test's recorded active time is13.333390667s; its86.32s
whole-process wall is separate. Reader model calls and diagnostic backward are0.

Local roots: implementation `artifacts/instruction-bridge-completion-20260924-implementation/`;
prepared study `artifacts/instruction-bridge-completion-20260924-study-final/`;
independent evidence `artifacts/instruction-bridge-completion-20260924-review/`.
Large native/raw/fixture content stays local.

| Independent evidence | SHA256 |
|---|---|
| `A-gate.log` | `0155808b6c4f3d39f6dfbe7ee638f0455b01c9a2b4bd8125e05cdef700512812` |
| `A-process.log` | `0595a9a0f14ec1f673c5b20801f6ad6f653f60bb88b9fc38e977d41bf8860646` |
| `read_A.rs` / executable | `ea9397ecf3eb01751b7ebd4444252a10da211e4278c1f1fa093c2c540a0e748d` / `f0d3892d75884c053751b2f14034ad4eeb28b24bf607d8c2b3e23607e113c7bb` |
| Reused native production rlib | `a9126c1a260edacd3a54ab89d7341366649ad1ca479e5df21c8da5699d0b8df0` |
| `A-preparation-audit.r3b` | `c67ae5febc415889c1c6f29018a0645fb8d54aa545941ad6e9c1eba916473b69` |
| `A-protected-manifest.r3b` | `decda3e21ebea57c8a43fd040750fc2c7712fbd839d602482e76bc49ddc24e2f` |
| `A-tiny-accounting.r3b` | `a4916937d27020920d60841ac9dbe05179dd78d7a05956cf9370e337206674ee` |

The confirmed `review-a.r3b` binds this report's exact bytes, source and
preparation. No additional code finding blocks this bounded preparation.
Parent32 parity and authorized learning remain the implementer's next work;
endpoint B and diagnostic QA are separate later stages. S4/S5/S6/Goal1 remain
NOT_ACCEPTED/false, and unused budget is not permission to extend the study.
