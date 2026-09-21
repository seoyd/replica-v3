**Independent review A: PASS. Endpoint recount and reproduction: PASS. Both model candidates: FAIL.**

This review applies R3-FOUNDATION-ORBIT-1.0 to source
`13969c6fe2b7b9232297a0796b037b202add350b`, relative to
`a4267d308c233f7bab47e1c4bcf39f4f4e21d3cb`. The implementation-report HEAD was
`2e0497fdb4804257f72569406b5289515e5d69e9`; an actual `git ls-remote` confirmed
that full SHA on origin/main before this report was published. Candidate-to-report
changes were confined to two documentation files. This subsequent commit contains
only this review report; it is not a new source candidate.

**Confirmed code findings: 0 within the reviewed scope.** No Severity High,
Medium or Low issue or production patch is proposed. Quality failures below are
observed model results, not evidence of a newly established implementation defect.
This verdict does not certify unrelated product paths or general model capability.

The review read the changed binding/fresh/quality-recovery paths and their direct
trainer, native reader, scorer, authorization, publication and process tests.
In particular, `src/binding.rs` implements request-only label resolution,
value-only assignment expansion, semantic split validation, finite tapes,
four-way scoring and candidate gates. `src/fresh.rs` dispatches the commands and
keeps teacher observation separate from generation; `src/quality_recovery.rs`
records gold/foil logits only after unrestricted generation. The product library
does not import the training generator. No external model, API, new SMALL
training, previous stabilization suite or PyTorch comparison was used.

**Evidence executed in this review** is under
`artifacts/foundation-independent-20260921-Yw3DU8/`. An exact candidate archive,
separate Cargo target, TMPDIR, fixtures, native outputs and logs were used.
Previously supplied independent Rust readers were inspected, copied and rebuilt
against that archive. Their only adaptations were source/output paths and removal
of a pre-training absence assertion for this retrospective preparation review.
A separate intervention reader independently checked request labels, native-input
collisions and all paired tape slots. Existing reported results were not treated
as fresh execution.

The source/tests/Cargo preservation manifest contains 40 files, and the original
study plus parent preservation manifest contains 14,820 files. Before/after
manifests compare identically. The original executable and contract input also
rehash unchanged. No original plan, checkpoint, corpus, raw row, receipt or
authorization file was rewritten. Existing untracked `.DS_Store` was preserved.
Only this aggregate report is published; helper sources, fixtures and raw outputs
remain local.

**Review A — preparation and direct execution.**

|Check|Independently observed result|
|---|---|
|Semantic skeletons|train128 / dev128 / confirmation64; pairwise disjoint|
|Old opposite-assignment dev|Its skeletons equal old train; none occur in new dev|
|Native common pool|512 train rows and 512 dev rows, identical between arms|
|Actual exposure|FIXED256 unique rows×16; BOTH512 unique rows×8|
|All512 tape rows|Same four skeletons and query order at every update; each skeleton visited16 times|
|Intervention|2,048 slots identical; 2,048 change only both evidence-value token spans and target digit; EOS unchanged|
|Actual input identity|No contradictory targets for an identical native input among1,024 train/dev requests|
|Token counts|Per arm input593,920 / target8,192 / padding0; equal counts do not mean identical input or target tokens|
|Framing|Prompt+digit+EOS146 tokens; two provided records, none excluded; same training/generation framing|
|Initial state|Exact old untrained A initial, common tokenizer/model, no training state; fresh Adam-zero digest independently recomputed|
|Controls|Physical order64/64; first/last record, smaller event ID and smaller value rules each256/512|

Train key frequencies are [25,25,26,26,26,25,25,26,26,26], and value
frequencies [26,26,26,25,25,26,26,26,25,25]. Dev key frequencies are
[25,26,25,25,26,26,25,26,26,26], and value frequencies
[26,26,26,25,26,26,25,25,26,25]. These are measured near-balanced marginals;
they are not claimed to be identical. Individual digits or edges shared across
disjoint skeletons are permitted by this contract.

The reviewer read the sealed confirmation corpus only for A-stage integrity,
grouping and split/ID-disjointness checks. Its examples and answers are not
reported, used for candidate selection or evaluated by a model. This is distinct
from running conditional confirmation.

The old A diagnostic used exactly the first eight stored train bases, both
queries. An additional reader checked the old raw checkpoint/model/step/policy,
request/prompt and returned-call bindings and decoded the selected original
tokens. Original FULL14/16 and QUERY_BOTH6/8 become swap FULL4/16 and
QUERY_BOTH0/8; conditional success on the original-both subset is0/6. Same
output9/16; swapped gold/foil/other/error =4/11/1/0. These are
DERIVED_EXISTING_RAW, with no new swap generation. A low diagnostic score is not
an execution-integrity failure or proof of an internal mechanism.

Three unique candidate tests ran successfully, exit0, zero-test invocations0:

- `foundation_t1_t2_t3_values_split_and_finite_tape`
- `foundation_t4_t6_native_process_resume_and_peer_authorization`
- `foundation_t5_strict_orbit_scores_and_seen_mask`

They exercise native data/tape and batch construction, malformed assignments and
metadata, the512th tape row, strict scorer/seen-mask cases and both-arm TINY
continuous2 versus fresh-process1+1. Weights, Adam, step, sampler and input/target
clocks match. Normally completed low-quality FIXED permits BOTH; cancellation
and UNKNOWN block progression. New TINY usage was optimizer8, generation64,
teacher64; scalar/finite-difference0. Initial helper compilation mistakes were
retained in logs and occurred before model calls; they are not product failures
or passing test executions.

REVIEW_A_READY_TO_TRAIN=PASS for the registered preparation. The real study was
already closed at512 updates per arm, so this retrospective acceptance does not
reopen it or authorize more learning.

**Saved-endpoint results — independently recounted.**

All16 scheduled panels were reread:2,816 generated rows and2,816 teacher rows,
with native checkpoint/policy/dataset/tokenizer, prompt, prepared/returned call,
token-to-bytes, strict UTF-8, EOS, summary and teacher checks. Errors remain in the
denominators. The two512-update traces and both terminal checkpoints were also
checked. Evaluation checkpoint files and terminal files have different physical
hashes but identical corresponding native model content and step512.

|Arm / panel|FULL|QUERY_BOTH|SWAP_BOTH|ORBIT_ALL4|Errors|
|---|---:|---:|---:|---:|---:|
|FIXED common train|274/512|88/256|42/256|0/128|0|
|BOTH exposed train|256/512|0/256|31/256|0/128|0|
|FIXED new dev|236/512|46/256|19/256|1/128|0|
|BOTH new dev|255/512|1/256|28/256|0/128|0|

FIXED exposed train fit is213/256, QUERY_BOTH85/128; its unexposed half is61/256,
QUERY_BOTH3/128. Its common-pool274/512 must not be called exposed train fit.
BOTH gives the same output to both queries for every one of256 train pairs and
253/256 dev pairs. Its final errors are all the other provided value. These are
observations on these endpoints, not a global impossibility claim.

On the same new dev, FIXED→BOTH gains143 and loses124 FULL answers, net19/512
(3.7109375 percentage points). At128 orbit units the descriptive SE is
1.5211013 percentage points, normal-approximation95% interval
[0.7295790,6.6922960]. QUERY_BOTH falls46→1 and ALL4 falls1→0. One seed and
a finite digit alphabet limit this inference;512 rows are not independent units.
No improvement percentage is computed against old A's different dev split.

|Arm / panel|Digit full-vocabulary NLL|EOS NLL|Gold/foil renormalized NLL|
|---|---:|---:|---:|
|FIXED train|1.592714805|0.000456483|1.470620619|
|FIXED dev|2.042247317|0.000457582|1.858239618|
|BOTH train|0.924160954|0.000329529|0.848874227|
|BOTH dev|0.959151574|0.000328444|0.848460167|

The last column was recomputed with stable softplus of the negative recorded
gold-minus-foil raw-logit difference. Only this binary conditional quantity uses
ln2 as a uniform reference. Actual decoding remained full-vocabulary greedy.

**New endpoint reproduction — EXECUTED_THIS_REVIEW.** Each arm's first16 stored
dev rows was generated once in a separate process through the exact candidate's
normal `Transformer::generate_observed` path and original tokenizer. Golds and
metadata did not select evidence, logits or outputs. Tokens, strict decoded text,
finish/error and completion agree16/16 for each arm, including wrong answers.
The standalone reviewer wrapper writes only new scratch receipts and imposes
16-generation/zero-teacher/zero-optimizer limits and cancellation/deadline checks.
It does not rerun the original once-only CLI or overwrite its receipts.

New SMALL generation32, teacher0, optimizer0, returned32, mismatch0, retry0.
These are additional, explicitly requested review calls, separate from the32
review calls already recorded in the old study ledger. Recorded new wrapper
elapsed is0.953750167s and0.931695583s, including load; these are not resource
benchmark or S6 measurements.

Existing raw receipts independently sum to SMALL optimizer1,024,
input1,187,840, target16,384, generation2,896, teacher2,816; discarded input,
discarded target and padding are0. Existing generation and teacher calls are all
RETURNED, with NOT_INVOKED0/UNKNOWN0. Including this review's separate32 SMALL
generations gives2,928 generations; the original ledger remains immutable.
TINY calls remain separately reported. This review's total new generation across
SMALL and TINY is96, teacher64 and optimizer8, all optimizer work TINY.

Both candidates fail train FULL508/QUERY_BOTH252/ALL4 124 and dev
FULL488/QUERY_BOTH232/ALL4 116. CONFIRMATION=NOT_RUN_PREREQUISITE;
MINIMAL_BINDING_BASELINE_VERIFIED=false. S4/S5/S6 remain NOT_ACCEPTED,
GOAL1_READY=false and GOAL1_ACCEPTED=false. No conditional confirmation, new
SMALL learning, seed/LR/loss search or old-run extension was performed.

**Verification checklist.**

|Category|This review|
|---|---|
|Unit|Literal FULL/QUERY_BOTH/SWAP_BOTH/ALL4, exposed mask and false exact-match rejection passed|
|Integration|Native writer→reader→trainer→save→fresh-process resume passed on TINY; SMALL saved endpoints recounted and reproduced|
|Regression|All512 paired tape rows, original A variant0 preservation and first1/resumed511 trace/clock checks passed|
|Malformed / boundary|Missing query, duplicate record, partial swap, changed ID, broken metadata, out-of-range final tape index and incomplete panel fixtures passed; independent contradictory-target check rejected the negative fixture|
|Failure path|Low-quality normal FIXED continuation, cancelled/UNKNOWN peer rejection and unmatched started-receipt rejection passed; no broad storage/backend fault suite was repeated|

No further test or patch is a prerequisite within this closed scope. Future
changes to these boundaries require their relevant direct regressions; this is
not authorization to repeat the closed quality experiment.

Reproduction uses Rust1.98.1, existing locked dependencies, offline Accelerate
and one compute thread. The direct test invocation, from the exact source copy,
was:

```sh
env TMPDIR=../tmp CARGO_TARGET_DIR=../target VECLIB_MAXIMUM_THREADS=1 OMP_NUM_THREADS=1 RAYON_NUM_THREADS=1 cargo test --release --locked --offline --features accelerate --bin replica-train training::fresh::identifiable::binding::tests::foundation_ -- --nocapture --test-threads=1
```

Local helper sources and build/run logs retain the exact recount and reproduction
commands. Model reproduction requires a new explicitly authorized output and
budget; it must not be repeated merely to reproduce this report. Important hashes:

|Evidence|SHA256|
|---|---|
|Compiled candidate source digest|3ad507da91a8fcc32ac0fb490a720c7cc8d21c5157102e9a5b1fcaadcf11e74a|
|Original production executable|5745c440536e5046518dad97b450a78a3c8d9f14e5680a88a2b382e809f3fea1|
|Rebuilt direct-test binary|9dd95fd3c3dbe4254ebb7f4ecc724b6d912a82ffe97b9d3a67bdbfc7b229425c|
|Reviewer generation binary|f25c98fadb6b999a04c3dd4323a33ce2ea5c1610125d87c598ec721f9974fe57|
|FIXED terminal native file|7500f025bdf1ac9513c7e1c638028cbdd8b4480b9d12742bc214b8ba73ee2743|
|BOTH terminal native file|44c8c105f7e240a90e18028face8f404782fe9b7e69ab6b993fbaa512e18c364|
|Local EVIDENCE_SHA256SUMS|549dd0478fa208bc7ddb9a48012770c746bfa43aec2960a1ed01c11d52023ef4|
|Source preservation manifest|ce375f102c3a62afba04572fa049f5ba9c3495ee6ca01622a26d5a2abadff369|
|Original preservation manifest|e8e5af7aa8bbed67562149d1619797397d8f963101b01e7eb9cc99153c1f2fc1|

CODE / DATA_SPLIT / TAPE / RESUME / RAW_INTEGRITY pass within this review.
Training fit and unseen joint selection fail their declared gates. Reproduction
and execution acceptance do not establish a learned baseline or Goal1 readiness.
