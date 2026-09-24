# Diagnostic repair and bounded quality recovery

## Active: CPU reference preservation and opt-in Metal F32

Execution closure: M1 is independently `B_COVERAGE_COMPLETE` (16 new,42 reused
physical observations; fixed normal32/failure32 overlap6). M2 reached actual
Metal F32 but failed the preregistered gradient tolerance. Scalar-oracle
middle-axis sum and GQA repeat backward reproduced the mismatch; independent
Runtime A is NOT_ACCEPTED. The shared Adam entry rejects Metal before mutation.
M3~M5 remain NOT_RUN, optimizer budget used0. This is a bounded negative outcome,
not an accepted Metal runtime or permission to substitute a backend/kernel.
The planned requirements below are preserved; unfinished descriptors/resume and
quality/performance tests are explicitly listed in EXPERIMENT_STATUS.

`R3-METAL-F32-BASELINE-1.0` preserves source849d8bb, accepted11264,
base14336, the failed adapter14464, all prior raw and immutable reviews.
Latest closure a9ae778 retains A and the42 replay/raw evidence but rejects
complete B coverage: fixed normal10 is short of32. M1 selects first complete
query pairs8/6/6/6/6 from V/VC/S1Q1/word/renamed, independently of correctness.
Only the difference from existing RETURNED cases is generated (cap22,
total unique64); failed32 and original producers remain separately bound.
An immutable study-root claim precedes supplemental calls; incomplete/UNKNOWN
coverage cannot authorize a retry or full acceptance. The failed model stays
failed and cannot resume.

Then add explicit CPU(default) / Metal0 F32 execution with locked Candle0.11.0,
no CPU fallback or equation/tokenizer/native content-hash reinterpretation.
Runtime device identity is separate from preserved math/model identity and
binds cache and new training provenance. Required operators, actual tensors,
gradients/Adam, synchronization, host transfers and native reload are verified.
Direct numerical tests and independent Runtime A precede protected P128
(11264 V64/VC64) and challenge C64 (14336 S1Q1/word32 each) on CPU and Metal,
plus worker4 per backend. Protected CPU-correct answers/support/EOS may not
be lost. Token mismatches and quality changes are separate results.

Registered tolerances: finite forward/logits/KV abs<=1e-4+1e-3*abs(reference);
CE abs<=1e-5+1e-3*abs(reference); gradient tensor NRMSE<=1e-2 and cosine>=.999
where nonzero, near-zero max_abs<=1e-6; one-update delta NRMSE<=2e-2,
near-zero max_abs<=1e-7. NRMSE denominator is max(reference L2,1e-6*sqrt(N)).
Integer inputs/masks/cursor and original tensor bytes must match exactly.
NaN/Inf fail; thresholds are not changed after observing results.

Only after numerical/protected M3 gates pass, run the same14336 weights/Adam,
word mixed batch8 tape, ANSWER CE and LR3e-5: CPU16, Metal16, Metal1+15 in a
new process. Maximum48 SMALL updates, diagnostic scope only. Compare each
step and identical-backend restart; save only source reference and needed1/16
states. Each endpoint gets16 fixed generations. Bench8 fixed requests with
warmup1/measured3 per backend, synchronize before/after timings. Distinguish
load/compile/prefill/decode/TTFT/IPC/validation, current/peak RSS and Metal
allocator metrics. Numerical pass does not imply speed gain (target1.2x).

Overall caps: SMALL generation704, teacher256, optimizer48, diagnostic
backward32; TINY optimizer64/generation96/teacher96. Primitive ops separate.
Active7200s, segment900s, evidence1.5GiB, build-growth upper bound8GiB;
UNKNOWN/cancel/numeric/storage errors block dependent work. One heavy process,
incremental0, compatible target reuse. No inventory/cleanup/deletion/move,
FP4 rerun, half-precision training, Muon, new attention or quality fine-tuning.
Missing Metal dependencies may be obtained from the registry without a Candle
upgrade, with lock changes reported. Independent LegacyB, Runtime A/B and
code/numerical/protected-quality/speed verdicts are distinct. CPU remains the
default; GENERAL_QA_IMPROVED=NOT_ESTABLISHED and GOAL1_READY=false.

## Closed execution: frozen-base q/v adaptation and independent precision probe

The actual adapter run stopped at128 new updates / effective14464 with
`SEVERE_RETENTION_REGRESSION`, `resume=false`; S1Q1 FULL fell from64 to17/64.
Unused optimizer allowance is closed. Zero-delta16 parity passed, but active
retention and word learning did not. The separate11264 FP4 reference completed
native16 plus dequantized256 calls with no measured count regression on this
sample. Its codec/storage result is not GPU FP4, S6 or Goal1 acceptance.
Independent B passed the intact endpoint audit and42/42 fresh reproduction;
it did not accept model quality. The owned-root measurement is81,403,358 bytes,
with retained touched build outputs bounded by67,335,036 bytes; deleted bytes0.
The registered policy below is preserved, not reopened.

`R3-PROTECTED-ADAPTATION-PRECISION-1.0` preserves accepted11264, normal
bridge14336 and failed14464. The latter remains closed. This is a new14336
fork with frozen base tensors, q/v rank8/alpha8/dropout0 in all layers,
normal A(std=1/sqrt(hidden), seed20260924), zero B, fresh adapter-only Adam,
constant LR3e-4, decay0 and no warmup. Historical full-tune changed different
parameters/Adam/LR/decay; it is not a one-variable causal comparison.
ANSWER CE, QE, tokenizer, original word7680/validation3456 and the exact first
1024 updates of its tape are unchanged. Shared inputs use immutable path/hash
references. No old model/data copying, inventory or cleanup is authorized.

The actual trainable registry is A/B only (SMALL59904 parameters). Native
delta records bind the base file/content/config/tokenizer/framing, tensor
registry, adapter/Adam, objective, policy, tape and cursor. Effective step is
base14336+updates; fresh Adam bias correction uses updates. No automatic merge.
Generic full-tune resume rejects this profile. The same adapter is active for
every input in training, prefill and cached generation.

Direct numerical/native-process tests and independent A precede zero-delta16
parity and learning. Save/restart after update1. At64/128/256/512 evaluate
V64,VC64,S1Q1-64,word64,renamed64. Final1024 evaluates six old512 panels,
word192/renamed192 and exposed train128. Retain the existing parent-relative
severe/persistent guard. Only a jointly passing development endpoint permits
word fit1536; old/word/fit gates are unchanged. Normal completed or guard-stopped
integrity endpoints permit bounded independent B reproduction, irrespective
of candidate eligibility. Cancel/UNKNOWN/I/O remain blocking. No QA640,
confirmation or S4/S5/S6 is authorized by this study.

P caps:1024 SMALL optimizer,8192 generation,8192 teacher,4M input/400k target,
7200 active seconds,900 per segment. Shared TINY caps:32/192/192 including
failed and reviewer calls. No diagnostic SMALL backward or automatic search.
New immutable evidence/models/data cap1GiB; mutable build growth cap8GiB;
free space must retain16GiB plus planned writes. One heavy process; locked
offline builds reuse the managed target with CARGO_INCREMENTAL=0. Preserve
actual final executables and receipts; no savings are claimed for existing
release incremental defaults.

Q independently uses accepted11264 only. `R3-FP4-E2M1-B32-F32S-EXPERIMENTAL`
packs q/k/v/o/gate/up/down in row blocks32 with F32 scale; tied embedding and
norms remain F32. This is neither MXFP4 nor NVFP4. Dequantized tensors execute
the existing F32 CPU/Accelerate path. Primitive codec tests precede native16
parity plus dequantized256 normal generations on metadata-first V128/VC128.
Optimizer/teacher0,900s cap. Report output regressions, actual bytes and
process peakRSS including dequantization; no FP4 GPU/training/S6 claim.
Code, frozen-base identity, active retention, word learning and precision
results are separate verdicts. GOAL1_READY remains false.

## Closed scope: four-word QA value selection

The registered run stopped at additional128 / absolute14464 with
`SEVERE_RETENTION_REGRESSION`, `resume=false`. Source/preparation A passed;
word FULL0/64 and renamed FULL0/64 did not pass. The S1Q1 retention panel
fell to24/64. Remaining optimizer allowance is not permission to resume.
Fit, fresh B reproduction, QA640 and all acceptance stages remain NOT_RUN
after this guard stop. Raw and checkpoints are preserved; independent B is a
read-only stop/integrity audit. The original registered design below is retained.

`R3-QA-WORD-VALUE-1.0` uses the normally completed bridge14336 weights,
ANSWER objective/family6, Adam moments and clock, tokenizer, QE, batch8 and
constant LR3e-5. Previous failures/resume=false and accepted11264/4352 remain
immutable. CLEANUP_EXECUTION=CANCELLED_BY_USER; APPLIED=false. No inventory,
deletion, movement, compression, hardlink substitution, cargo clean or rollback.
Previously accepted no-call finalization and audit evidence remain preserved.

The training-only extension selects and cites four existing QA values (왼쪽,
오른쪽, 직진, 대기). It retains two current records, 장치0–9, 구역0, equal
timestamps, eight-digit positive IDs, QA system and the existing S1Q1 question.
For each of six word pairs, rank45 key pairs by domain/seed20260924:32 train,
8 dev,5 unused. Train192 groups ×2 ID versions ×2 assignments ×2 questions
produces1536 rows; dev48 groups ×4 produces192, with an ID-only renamed192.
Unused groups and previous seals are not opened. The original bridge6144 train
and six512 dev episodes remain unchanged in the appended native corpus.

One registered fork permits3072 optimizer updates,11264 generations and10240
teacher samples, input8M/target1M tokens, active10800s, segments900s, inference
12GiB/training16GiB. TINY direct regressions have separate caps64/512/512.
Each batch consumes four reviews from successive halves of the actual1536-step
parent tape and two new complete query pairs from different groups. New1536
rows receive one exposure every384 updates, at most eight cycles. New answers
use an unrestricted whole UTF-8 string scorer; historical digit metrics and
normal greedy decoding remain unchanged. Full input≤256, generation≤32;
EOS belongs to ANSWER CE, prompt/padding do not.

Direct data/scorer/gradient/native-process checks and independent A precede
parent16 parity plus new384 generation-only observations. If both new gates
already pass, no training is needed. Otherwise save/restart after update1;
evaluate five64 panels at128/512, six512 old panels plus two192 new panels and
actually exposed train128 at1536/3072. Fit1536 runs at most once: at1536 only
when both development gates pass, otherwise at3072. Failed early fit closes
the study. Fixed parent64 retention drops≥16 or≥4 non-EOS/error rows stop;
drop>4 FULL or>2 ALL4 at two consecutive scheduled evaluations also stops.
Cancel, UNKNOWN, storage/integrity errors remain blocking.

Each word dev requires FULL183/192, QB/SB88/96, ALL4 44/48, value/support190,
EOS192 and zero outside-ID/parse/generation errors. Fit requires1524/1536,
QB/SB756 and ALL4 372 with zero errors. Old six-panel gates remain FULL488,
QB/SB232, ALL4 116; citation value/support508 and zero errors. Independent B
recounts all raw and reproduces at most64 cases. Only normal complete endpoints
with B integrity PASS may run original QA640 once (original128-token limits),
even when quality fails. No confirmation, S4 seal, S5/S6 or Goal1 acceptance
is granted by this development study. No automatic data/LR/loss/seed extension.

## Closed scope: artifact hygiene and no-call finalization

R3-ARTIFACT-HYGIENE-AND-FINALIZATION-1.0 permits zero new SMALL/TINY
optimizer, generation, teacher or diagnostic backward calls. Previous bridge
quality failure, QA640 and accepted11264 remain unchanged. A fully RETURNED QA
journal can finish at the active-time cap through the existing diagnostic caller:
verify historical producer/model/policy/cases, per-call resolutions and pure-time
segment accounting before publishing final/score. Partial work retains original
source and model-budget admission. Missing/UNKNOWN/cancel/mixed/storage failures
remain blocking. Finalizer provenance and bounded management work (900 seconds,
512 MiB of observation files) are separate from preserved model active usage.
Interrupted management publication is fail-closed; complete final/score reentry
only verifies immutable results. Limits are cooperative, not tensor/fsync preemption.

The narrow four-row RETURNED process regression and independent A passed without
model calls. Independent B verified the corrected dry-run plan; original deletion
is not authorized. Artifacts were inventoried once with no-follow metadata;
known references and selected exact hashes inform an explicit dry-run plan.
Keep originals, native/Adam/raw/policies/receipts, exact binaries and dependencies,
unique readers/patches/logs, unknown trees and root target. No original deletion,
move, hardlink substitution or archive overwrite is authorized. A later apply
requires explicit approval of the exact plan digest and fresh identity/reference/
writer checks. This task creates no new quality, confirmation, S4/S5/S6 or Goal1
acceptance and does not reopen any closed experiment budget.

## Closed execution: instruction bridge completion and diagnostic QA

The registered study executed1536 updates to14336. Independent A and B passed
their code/preparation and execution/integrity scopes. Development failed with
one outside-ID row in each bridge panel; fit recorded1535/1536 with one preserved
native TIME_BUDGET return. An evaluation-only process completed the remainder
with zero optimizer updates. The final endpoint is non-resumable and ineligible.
The separately authorized original QA640 diagnostic completed at0/640, EOS300,
length340 and runtime/UTF-8 errors0. Independent verification of the new640 rows
passed; the addendum is tracked in the status document. No additional learning,
model selection, confirmation or S4
is authorized by this closed plan. The registered policy below is retained as
the execution specification, not a new grant of unused call or training budget.

R3-INSTRUCTION-BRIDGE-COMPLETION-1.0 registers a new fork of the preserved
bridge12800 endpoint. Its original BRIDGE_DEVELOPMENT_FAIL/resume=false and
accepted11264 remain immutable. Preserve weights, 136 Adam tensors, cumulative
clock, tokenizer, QE, ANSWER CE/family6/normalizer2, first-target1, batch8 and
constant LR3e-5. Copy corpus6144/metadata/transfer byte-for-byte and append the
exact consumed1536-update bridge suffix once. Expected new sample/input/target/
padding counts are12288/2064384/162816/233472. No new data or parameter search.

Preparation and independent A precede32 metadata-only parent parity generations
and learning. Check six64 panels at13056 and six512 panels at13568/14336.
At13568, development passage triggers the single1536-row train fit: passage
fixes the candidate; fit failure closes normally. Otherwise continue only to14336,
where fit runs once even on development failure. Preserve the existing strict
development/fit thresholds and severe/consecutive retention guards described
below. Final normal failure is FINAL_BRIDGE_QUALITY_FAIL_AT_14336. Store/reload
at12801, scheduled evaluations and14080; no optimizer calls during evaluation-only
time resumption. Partial/RETURNED evaluation and fit rows are reused unchanged.

After independent B verifies a normal complete13568/14336 endpoint, diagnostic
oldQA640 is authorized even when bridge quality failed. This requires all six
full panels and fit, excludes cancellation/UNKNOWN/corruption/storage/guard stops,
and grants no candidate, confirmation, S4 or training authority. Reuse the exact
original512+128 inputs,128 output tokens/context2048/timeout120000; retain every
returned row and resume only clean time-limited segments. B audits only the newly
added QA results afterward. The used citation confirmation remains closed.

Reuse `fresh qa-bridge-prepare --continue-from OLD_STUDY --output NEW_STUDY`,
`qa-bridge-review --parent`, `fresh run`, `qa-bridge-report`, and
`qa-bridge-qa --diagnostic` (plus `--transfer`). The original candidate-only QA
command retains its eligibility requirement. Diagnostic collection shares the
existing segmented confirmation collector, with separate bindings and artifacts.

New limits:1536 SMALL optimizer/9216 generation/8192 teacher/diagnostic backward0;
input4M/target300K, active7200s, command900s, inference/training12/16GiB.
TINY limits64 optimizer/1024 generation/1024 teacher. No automatic extension.
Independent A covers policy/tape/native resume and diagnostic permission/failure
boundaries. B reproduces32 normal plus at most32 failed-pair rows. Code, bridge
quality, general QA and S4/S5/S6/Goal1 are separate verdicts.

## Closed learning budget: QA integrity and bounded instruction bridge

A1 and A2 independently accepted the scorer repair and bridge preparation.
Frozen source `8522d026512cfb7b6e4b6c32142c5d68b77c9f2f` executed exactly1536
new updates from11264 to12800 with the registered tape/Adam/LR. Final retention
V/VC/ID each scored512/512; the three bridge panels scored511/511/508 but each
retained one valid outside ID. The endpoint is BRIDGE_DEVELOPMENT_FAIL/resume=false.
Conditional fit and oldQA640 are NOT_RUN because development failed. No further
optimizer calls or automatic extension are authorized. Independent B accepted
execution/raw integrity and44/44 fresh-process reproductions, including all six
wrong rows; bridge quality remains FAIL. Narrow improvement is not Goal1 acceptance.
The registration below remains the unchanged basis for this run.

R3-QA-INTEGRITY-AND-BRIDGE-1.0 supersedes the retained-QA metric acceptance,
following the independently reproduced malformed/outside-ID undercount. Keep
the accepted11264 model and all15360 raw, failures and closed decisions immutable.
First repair diagnostic ID extraction and the actual QA candidate gate: strict
whole-response parsing remains unchanged; every individually valid positive i64
reference is counted against provided evidence, even beside malformed references.
Outside rows and parse-failure rows must both be zero for a new QA candidate.
Publish a versioned read-only recount of7168 raw rows, including3328 QA rows;
do not rewrite old summaries or grant retrospective candidate eligibility.
Independent A1 must verify the full writer/reader/audit/scorer/gate regression,
positive G/H cases and the four historical missed outside rows.

After A1, observe the same first four citation-dev bases (16 rows) on11264 under
short/QA system and original/current-alias wording. Reuse VC16 parent parity as
S0Q0: V16+VC16 parity and three variants16 each consume80 generations, no teacher
or optimizer. Keep evidence, answers, IDs, order, clocks and generation32 fixed.
If every variant reaches FULL15/16, QB/SB7/8 and ALL4 3/4 with no outside, parse or
generation error, close NO_TRAINING_JUSTIFIED_BY_THIS_PROBE. Otherwise prepare
all three variants, not only failures, for independent A2 before learning.

The conditional bridge preserves ANSWER11264, Adam/clock, QE, tokenizer, TR++,
ANSWER CE/first-target1, constant LR3e-5 and train512. Select64 train bases by
metadata hash only, all four views and both ID versions:512 source rows produce
1536 bridge rows. Retain4608 original rows. Batch8 is V2/VC2/bridge4, always query
pairs. At1536 updates V receives3072 exposures, each VC version1536 and bridge6144.
No confirmation or sealed S4 examples enter these data. Audit full tape, target
tokens, no dropped evidence, native/cache binding and fresh-process resume.

At +1 save/reload; +256 evaluates six64-row panels; +768/+1536 evaluates retention
V/VC/ID and three bridge variants, each512. Save intervals are at most512 updates.
V dev requires FULL488/QB232/SB232/ALL4116/errors0. Each citation panel also needs
value/support508, outside0 and parse0. If development passes, bridge train1536 fit
runs once: FULL1524/QB756/ALL4372, no outside/parse/generation errors. Fit failure
closes BRIDGE_FIT_NOT_MET; a passing +768 endpoint stops learning immediately.
Retention64 warns below FULL60 or ALL412; two consecutive scheduled warnings for
the same panel stop. FULL<=48 or errors>=4 stop immediately. Preserve normal
time-only evaluation resumption and all cancellation/storage/UNKNOWN blockers.

Limits:1536 SMALL optimizer,9216 generations,8192 teacher forwards, diagnostic
backward0, input4M/target300K, active7200s, command900s, inference/training12/16GiB.
TINY limits64 optimizer/1024 generation/1024 teacher. No budget extension/search.
Independent B audits trace/native/raw and at most64 fixed normal/wrong replays.
Only bridge development+fit+B passage permits one oldQA640 check under its original
128-token contract. This is development transfer, not independent confirmation;
S4 remains sealed and S4/S5/S6/Goal1 remain NOT_ACCEPTED in this scope.

Execution uses `fresh qa-bridge-probe-*`, `qa-bridge-prepare`, the existing
`fresh run`, and `qa-bridge-report`. Preparation binds the original QA path for
the conditional scope check. `qa-bridge-review` selects32 metadata-only rows
across all six panels (8/4/4/8/4/4), and `--errors` selects at most32 additional
rows in failed query pairs; the latter is explicitly post-hoc diagnostic evidence.
`qa-bridge-qa` requires the complete development/fit endpoint and a bound B report;
`--transfer` selects the original128-row transfer split. No used confirmation is
opened. Bridge episodes retain the existing citation family prefix so the same
teacher path records actual target-token NLL, with variant/source lineage appended.

## Closed: retained value/citation capability and balanced QA transfer

Execution closed at15360 after4096 new optimizer updates under frozen source
`3a5f54ed91dffd8a5782e148c56a56dddf2244c8`. The final terminal is
PERSISTENT_RETENTION_REGRESSION/resume=false and the full learning budget is
consumed. V/VC/renamed dev FULL is512/505/496 out of512. Old QA is27/512 and24/128;
balanced QA is13/512 and3/128, with A–F all0. No candidate qualified. Conditional
retention fit, sealed S4 and S5 were not run. Existing11264 acceptance is unchanged;
S6 and Goal1 remain unaccepted. No further learning is authorized by this plan.
The following records the executed policy and unchanged gates.

R3-RETAINED-QA-TRANSFER-1.0 starts a separate ANSWER11264-bound research run.
The accepted parent, consumed citation confirmation, old QA640 observation and
all failed studies remain immutable. Product source baseline is
`3290fec66db372c5ab427bf2e632bf573824ce3d`. No reinitialization, external model,
teacher/API, tokenizer/core/storage change or new loss is authorized.

Reuse parent V1536/VC0 1536/VC1 1536 plus accepted balanced QA8192. Q0 preserves
its original requests; Q1 changes only the task clause to an existing train-only
phrase of the same intent. The physical pool is20992. Batch8 has V2 and VC2
(VC0/VC1 alternate), then two complete complementary QA pairs. Bucket pairs
cycle AB/CD/EF/GH, with one fixed512-base permutation per bucket. First2048
updates expose every Q0 once; next2048 expose every Q1 once in the same order.
R exposure is16384 (V8192, VC0/VC1 each4096), with disclosed nonuniform case
counts; QA exposure is16384. Count target tokens separately from the ANSWER
objective's eight-example denominator; this does not imply equal gradient norms.

Preserve weights, Adam moments and cumulative clock, ANSWER family6/normalizer2,
QE, first-target1 and constant LR3e-5. Train length512 retains full evidence;
model context remains2048. Retention requests keep generation32, QA128, timeout
120000ms. Verify actual training/generation prompt IDs and excluded0. No expected
answer or training resolver enters inference.

Before SMALL: directly test mixed data/tape/CE/masks/8-versus4+4, native TINY2
versus fresh-process1+1, final evaluation-only resumption and quality-failure
reviewer access. Use the existing quick runner's retained-QA filter and separately
counted process receipt. Independent A binds frozen source, preparation and an
independently prepared, unopened S4 seal. The new study verifies actual sealed
files. Initial observations are parent V16+VC16 parity and balanced512+128 once;
reuse the previous old QA640 raw without regeneration.

Save after new1 and at intervals no larger than512. At new128/512/1024/3072,
evaluate V64/VC64/ID64, old QA64 and balanced QA64 (320). At2048/4096 evaluate
retention3x512, old512+128, balanced512+128 and exposed QAtrain128 (2944).
Only joint development passage permits full same-checkpoint retention fit4608.
Each retention screen independently stops at FULL<=48 or errors>=4; FULL<60
or ALL4<12 warns, with two distinct consecutive warnings stopping that panel.
Initial low QA is a new-task gap, not regression of accepted retention ability.
Cancel, nonfinite, I/O, binding and UNKNOWN remain blocking. Pure time stops use
the existing cursor; completed rows are never regenerated. Final-step resume
performs evaluation with optimizer0.

Common development gate: retention V512 FULL488/QB232/ALL4116/errors0; VC/ID
add first-value508/support508/outside0. Both distinct QA primary512 panels need
FULL487 and each bucket58/64; each transfer128 needs116, normal EOS/errors0/
outside0. Balanced C/D/E/F each need QUERY_BOTH29/32. G/H require actual correct
content, never an empty-citation shortcut. Retention fit old512 needs508/QB252/
ALL4124, new1024 needs1016/QB504/ALL4248, each VC1536 needs1524/QB756/ALL4372,
all errors0. QAtrain128 is diagnostic only. First eligible2048 endpoint fixes
the candidate; otherwise stop at4096, with quality failure and resume=false if
the gate still fails. No automatic seed/LR/data search or extension.

Independent B audits raw/teacher/trace/native and at most64 fresh reproductions.
Only the fixed eligible candidate proceeds to sealed S4:190/200, each category
36/40, accepted-invalid-citation0. Random/no-evidence/value-swap controls each
use a fixed balanced40 separately from main200, with retrieval-only comparison.
Only S4 passage permits existing app/worker/isolated Store S5 memory/correction/
restart smoke, at most20 generations. S6 is NOT_ACCEPTED until separately proved;
neither narrow acceptance nor QA training makes Goal1 accepted.

New SMALL limits:4096 optimizer, input18M, target2.4M, generation24576,
self-teacher18432, diagnostic backward0, active14400s, command900s+cleanup120s,
training/inference RSS16/12GiB. TINY shared limit96 optimizer/768 generation/
768 teacher, finite-difference8. Include known failed costs; UNKNOWN is not zero.
One owned heavy process, no background learning. Preserve raw and scoped commits;
publish only source, tests and anonymous status, then verify actual remote SHA.

## Closed: citation precision from complete ANSWER10496

Learning is closed at11264 after768 new updates. All three dev512 panels and
the same-checkpoint full fit4608 pass; the candidate is fixed with extend=false.
The remaining768 updates are unused. Independent B passed, and the original
unused citation confirmation passed V256/256 and VC256/256, with support256,
outside ID0 and errors0. Accept only two current records, one-digit key/value,
fixed grammar and eight-digit event IDs on the fixed11264 checkpoint.
The original QA512+128 observation then completed once, with teacher/optimizer0.
The user explicitly authorized its original128-token limits; these do not change
the citation32-token policy. QA scored0/640, with EOS440 and length200; the broad
QA gap remains. No further learning is authorized by this closed study.
S4/S5/S6 and Goal1 remain unaccepted; no product model pointer is changed.

R3-CITATION-PRECISION-1.0 preserves the accepted scalar4352 and all closed
fidelity A/B evidence. The specific failed10496 parent physical SHA256 is
`7845eb2e66bf333f07a4fde28d80601f418f212912c7c7b58fc41424646b5ff6`.
Its Finished / FINAL_QUALITY_FAIL_AT_10496 / resume=false remains unchanged.
This is a separate parent-bound study: the initial native is a bit-identical copy,
including136 Adam tensors and cumulative clock. Only actual LR changes3e-4 to
constant3e-5. ANSWER_MEAN family6/normalizer2, first-target1, QE, tokenizer,
F32/Accelerate/thread1, model shape, beta/eps/clip/decay and data remain fixed.
No warmup, optimizer reset, reinitialization or second LR/seed search is permitted.

The unchanged4608-row corpus and dev/metadata are native byte copies. Append only
the first1536 draws of the original3072 citation cycle after the preserved10496
prefix. First10497 consumes cycle0 and last12032 consumes1535. Batch8 is
V4/VC0 2/VC1 2, actual target76 and loss denominator8. Preparation must verify
input1830912/target116736/padding49152 and V6144/VC0 3072/VC1 3072 exposures;
the768-update point costs exactly half. These are planned costs until executed.

Direct checks cover the actual LR route, an independent F64 Adam first-delta
comparison with inherited nonzero moments, native2 versus new-process1+1,
policy mismatch rejection, tape/exposure, final evaluation-only resumption,
and the new seal lineage. Existing verified stabilization is not repeated.
Independent A precedes source/binary freeze and learning. Parent parity uses
V16/VC16, each at most four complete four-view orbits: wrong orbits first in
metadata order, then correct orbits. Freeze selection before invocation and
use normal greedy without passing expected answers to generation. Previous
wrong4/B32 runs are historical evidence, not calls repeated by this plan.

Save first10497, then10752,11264,11776,12032, with registered command time
segments as needed. Screen10752 is V64/VC64/ID64. At11264 and12032 evaluate
V512/VC512/ID512 and diagnostic train128. Only a common dev pass permits full
fit4608 of that same checkpoint. First dev+fit pass fixes the candidate and
ends learning; final12032 always has extend=false. Saved-only11776 has no score.
Final-step TIME_BUDGET resumes remaining evaluation with optimizer0. Cancel,
nonfinite/I/O/binding/data errors and UNKNOWN remain blocking. The V64 guard
is unchanged: FULL<=48 or errors>=4 stops; FULL<60 or ALL4<12 warns, and two
distinct consecutive warnings stop. Immutable decisions count once.

All previous full-fit/dev gates remain, including each citation dev's support508,
first-value508 and outside-record ID0. Report FULL/QB/SB/ALL4, grammar, provided
wrong ID, outside ID and EOS separately. train128 is not full-fit acceptance.
B verifies native/trace/raw/teacher and reproduces V16/VC16 with the same orbit
rule. Only an eligible candidate plus B PASS may open the existing unused
citation seal once (V256/VC256), using the existing same-candidate time protocol.
Confirmation gates remain FULL244/QB116/ALL458/errors0 on each; citation also
first-value254/support254/outside0. Confirmation PASS accepts only the bounded
value/citation scope and permits existing primary512+transfer128 once, teacher0.
Missing/incompatible QA is explicitly BLOCKED/NOT_COMPARABLE, never replaced.
S4 final200, product memory S5, INT4/M4 S6 and Goal1 remain separate.

New caps: SMALL1536 optimizer, input2000000/target130000, generation14336,
teacher13120, diagnostic backward0; active7200s, command900s, cleanup120s,
training16GiB/inference12GiB. TINY direct+independent+failed work is limited to
64 updates/1200 generations/1024 teachers/16 scalar finite differences.
Report actual costs, last durable step and stop reason. This is a post-hoc
development study with no simultaneous high-LR control; improvement would not
by itself establish the independent causal effect of LR. Never add dev errors
to learning or repair generated event IDs. Preserve originals and publish only
verified source/tests and small reports; no private/raw/model/seal artifacts.

## Closed learning budget: citation fidelity consolidation from ANSWER7424

Source820100a passed independent A, preserved parent parity32/32, and completed
all3072 new updates in9 segments through10496. The final saved native is
`7845eb2e66bf333f07a4fde28d80601f418f212912c7c7b58fc41424646b5ff6`,
Finished / FINAL_QUALITY_FAIL_AT_10496 / resume=false. No learning budget remains.
At8960 the full panels were V512/512, citation509/512, renamed512/512, but the
citation panel had3 outside-record IDs. At10496 they are510/510/512, with
selected citation support510/512 and2/0 outside IDs. All final panels have
EOS512/errors0; both citation first values are512. The outside-ID0 requirement
fails, so conditional full-fit4608, candidate selection, confirmation and QA640
are NOT_RUN. Fixed train128 success is not a full fit acceptance.
Independent B verified all3904 raw/teacher rows,3072 update traces and9 native
checkpoints. Fresh-process V16/VC16 match32/32, but the existing fixed prefix
contains only correct outputs. The initial
[independent B report](CITATION_FIDELITY_REVIEW_B_2026-09-22.md), publication
893d974d61b34d5c88ac2479f334487da4141f22, was PARTIAL for missing wrong-case
fresh reproduction. That original coverage limit remains historical evidence.

The user subsequently authorized exactly four additional wrong-case generations.
The reviewer fixed V23/V100/VC477/VC479 before execution, then reproduced all4
with the same10496 checkpoint and unchanged native normal greedy path in a new
process. Raw token/byte/text/EOS/finish/error equality is4/4; actual calls4,
tokens38, active0.862545250s, optimizer/teacher/TINY0, retries0, exit0.
These are separate observations; original raw, terminal and quality failure
are unchanged. The pure readback also exited0; current independent B verification
is PASS. The report-only addendum commit is40e9c9977042e0696b532a92a7c6c8544f87babc,
with report SHA25631068df393e3e35938501ae1700f61ed866a1bac7de00c7171572e01a182ce38.
No further calls or learning are authorized by the completed check.
S4/S5/S6 and Goal1 remain unaccepted. The original4352 acceptance and
all7424/failure evidence are preserved.
The registered design below explains the completed bounded run.

R3-CITATION-FIDELITY-CONSOLIDATION-1.0 is a new explicit fork, preserving the
accepted scalar4352 and complete failed ANSWER4384/7424 endpoints. The exact
7424 parent physical hash is
`8c2e9b0f670be09acbae40efb6e2d4cb4e5a3c01315b0d7e3a51aa467270ec47`.
Existing continuation A/B acceptance is closed; only this new parent's repeated
tape, schedule, native binding and seal linkage require direct regression/A.
No new loss, model, tokenizer, data, storage/SQLite or repair framework is planned.

P0 recounts existing5888/7424 raw and teachers without model calls, separating
value, selected/other-provided/outside/unparseable ID and descriptive digit errors.
P1 binds the same native4608 train/1536 dev, metadata, tokenizer, ANSWER_MEAN
family6/normalizer2, QE, F32/Accelerate/thread1, inherited Adam and constant3e-4.
The inherited tape prefix stays intact. The original citation suffix3072 is
appended exactly once: absolute7425 consumes suffix0,10496 consumes3071.
Batch8 is V4+VC0 2+VC1 2; EOS contributes to each answer's actual target mean.
Expected new costs: input3661824/target233472/padding98304; exposures V12288,
VC0/VC1 each6144. Half1536 updates is exactly half, V each4/VC each2.
Actual target76/batch and objective denominator8 remain separate.

After independent A and parent V16/VC16 parity, P2 saves7425 and restores in a
new process. Screens7680/8192/9728 use V64/VC64/ID64. Endpoints8960/10496 use
V512/VC512/ID512 and the existing citation train128. Safe saves8704/9472/10240
are not quality observations. Only a development-pass endpoint evaluates full
train4608. The first joint dev+fit pass fixes a candidate; otherwise stop10496.
Final-step evaluation-only continuation adds no optimizer update. Pure time
pause is resumable; cancellation, numerical/storage/binding failure or UNKNOWN
remains blocking. Fixed V64 FULL<=48 or errors>=4 stops; FULL<60 or ALL4<12 warns,
and two distinct consecutive warnings stop. Immutable guard decisions count once.

Unchanged gates: V old512 full508/QB252/ALL4124, new1024 1016/504/248;
VC0/VC1 train1536 each1524/756/372 (combined3048/1512/744), errors0.
All dev512 full488/QB232/ALL4116, EOS512/errors0; both citation dev panels also
require first-value508/support508/valid outside ID0. Grammar/ID_BOTH/SWAP_BOTH
are reported separately. Normal greedy raw is never corrected or constrained.

B independently audits the complete endpoint, including a quality failure,
and reproduces V16/VC16. Only eligible+B permits the existing unused V256/VC256
citation confirmation, one candidate with the accepted time-segment protocol.
Each confirmation panel requires full244/QB116/ALL458/errors0; citation also
first-value254/support254/outside0. Only confirmation PASS permits existing
QA primary512/transfer128 once, generation640/teacher0. Missing/incompatible
QA is BLOCKED_LEGACY_QA_INPUT/NOT_COMPARABLE. This accepts only value+citation;
S4/S5/S6 and Goal1 remain separate. No subsequent training is implied.

New caps: SMALL3072 updates, input4000000/target260000, generation14336,
teacher13120, diagnostic backward0; TINY64 updates/generation1024/teacher1024,
finite difference0. Active10800 seconds, command900/cleanup120, training16GiB
and inference12GiB. Failed/partial work counts; UNKNOWN is never zeroed. Freeze
source/binary for learning and preserve all historical failures and originals.

## Closed learning budget: citation continuation from complete ANSWER4384

The c9c1210 source passed independent A and ran all3040 newly authorized updates
from4384 to7424. First update4385 was saved and restored in a fresh process.
All11 segments completed normally; the final endpoint is
`FINAL_QUALITY_FAIL_AT_7424 / Finished / resume=false`. There is no remaining
learning authorization. Physical final hash:
`8c2e9b0f670be09acbae40efb6e2d4cb4e5a3c01315b0d7e3a51aa467270ec47`.

At7424, V dev511/512 (ALL4127), citation492/512 (ALL4113) and renamed495/512
(ALL4116) all have normal EOS512/errors0. Citation support495 and renamed497
miss508; valid foreign IDs17/15 miss0. Citation ALL4113 misses116. Thus no
eligible candidate exists despite better value/citation output than at4384.
Conditional full-train4608, confirmation512 and broader QA640 are NOT_RUN because
their common development prerequisite failed. Independent B verified all3040
updates,11 checkpoints and4096 generation/teacher rows, then reproduced V16/VC16
in fresh processes (32/32 parity). Execution/raw/reproduction passed; citation
quality failed. The [B report](CITATION_CONTINUATION_REVIEW_B_2026-09-22.md) was
published separately at79cc30c20a6dd50ddbe4c93b04a4b4fbbb8e446d, with that full
remote SHA verified. It does not promote the endpoint or reopen its budget.
Final SMALL usage is3040 updates/4160 generations/4096 teacher calls. Code and
independent execution acceptance do not imply S4/S5/S6 or Goal1 acceptance.

The next required quality step is to obtain an independently eligible value+
citation candidate before S4. The remaining observed gap is exact event-ID
binding/copying across assignment/query variants; its cause is not established.
Any further experiment needs a new explicit plan and budget. No new learning,
data/LR/loss/core sweep or protected4352 recheck is authorized here.

The registered policy below is retained as the explanation of this closed run.

R3-CITATION-CONTINUATION-1.0 preserves the accepted scalar4352 and both
TOKEN/ANSWER4384 QUALITY_REGRESSION endpoints. The latest independent rereview
accepted their numerical and execution evidence but found same-candidate
confirmation reentry incomplete. This narrowly scoped repair precedes learning.
No historical failure, receipt, unused budget or resume=false is changed.

Confirmation binds the immutable candidate, physical native/model/step, policy,
source/binary, tokenizer/framing, development result, B approval/report and the
existing citation seal. Candidate publication is confirmed before reading sealed
answers. A claim under the actual seal owner excludes another study. The same
candidate may reuse confirmed RETURNED rows, including wrong, length and returned
command-timeout rows. Only pure command TIME_BUDGET may continue. Cancellation,
request timeout, mixed error, pending publication or an unresolved call/command
remain blocking. Existing prepared/resolved records, native collector, scorer and
confirmed no-clobber publisher are reused. All segment costs count; only remaining
unreturned rows are reserved. Complete reports are pure reads and never generate
missing evidence. Small protocol fixtures are not model-quality acceptance.

The sole new arm copies complete ANSWER4384 weights/Adam and the exact objective
family6/normalizer2. It consumes original citation suffix indices32..3071, keeping
QE, tokenizer, F32/Accelerate/thread1, LOCAL5, CE answer mean, first-target1,
LR3e-4, batch8, accumulation1, clipping/betas/decay and actual Adam clock unchanged.
Native policy binding changes explicitly at this new parent-bound entry. The
3072-row historical suffix is not restarted. New updates are capped at3040;
new input3623680/target231040/padding97280 and24320 samples are planned. These
comprise V12160 and VC0/VC1 each6080; inherited32 updates remain separate.

Preservation checks use the original first16 V orbits. FULL<=48/64 or scorer
errors>=4 stops immediately. FULL<60 or ALL4<12 is a warning; two consecutive
distinct scheduled warnings stop for persistent retention regression. Equal60/12
resets the streak. Immutable raw/step/decision links prevent repeated application
on resume. The parent does not count as a warning. The old immediate ALL4-drop
guard remains unchanged in historical studies. Early citation0 alone is not a stop.

Save4385 then restart a fresh process. Evaluate V64/VC64 at4416/4480/4608;
add ID64 at4736/5120. At5888/7424 evaluate V512/VC512/ID512 and fixed VC-train128;
conditionally evaluate the complete4608 training panel if development passes.
Safe saves5632/6400/6912 keep segments<=512 updates. Last-step evaluation may
resume with zero additional optimizer calls. First eligible5888 fixes immediately;
otherwise7424 is final, without automatic extension or new LR/loss/seed search.

Final gates remain: V old512 full508/QB252/ALL4124, new1024 1016/504/248,
dev512 488/232/116; VC0 and VC1 train1536 each1524/756/372; VC and renamed
dev512 each488/232/116 plus first-value508/support508, no valid foreign citation,
EOS512/errors0. B independently recounts the actual endpoint and reproduces
V16/VC16, including a complete negative quality endpoint. Only eligible+B may
open the existing unused citation V256/VC256 confirmation once. Each requires
full244/QB116/ALL458/errors0, and citation first-value254/support254/foreign0.
After confirmation passes only, run preserved primary512/transfer128 once as a
no-learning scope check, or report missing/incompatible inputs explicitly.

Caps include all failed work: SMALL3040 updates, input4000000/target260000,
generation16384/teacher16384, diagnostic backward0; TINY128 updates,
generation2048/teacher1024, finite-difference0. Worst-case planned generation14528
and teacher13312 include parent32, B32, confirmation512 and QA gap640.
Active10800s, segment900s plus cleanup120s, RSS16GiB. Unknown usage is not zero.
Use `replica-check quick --citation-continuation` for directly changed boundaries.
Prepare with `fresh citation-continue-prepare`, perform independently reviewed
parent checks using `fresh citation-continue-parent [--citation]`, then existing
`fresh run`, `fresh answer-mean-report`, `fresh answer-mean-review` and
`fresh citation-confirm` commands. A is required before parent generation/learning.
S4 independent200, S5 actual memory/correction/restart, S6 INT4/M4 and Goal1
remain separate and unaccepted even if this narrow citation baseline passes.

## Closed at retention guard: answer-mean citation reduction from accepted4352

Actual frozen source `70d933b4b798b9d73be5b5f99db4db1c980889b9` ran both arms
for32 updates each, using1 plus a fresh-process31. TOKEN exactly reproduced
the prior128 raw outputs, tensor and Adam; its negative quality endpoint
permitted the registered ANSWER fork. ANSWER improved V FULL41→60/EOS49→64
at the same32-update budget, but V ALL4 fell from parent16 to12 and triggered
the unchanged >=4 regression guard. VC FULL/syntax/selected citation remained0.
Both final4384 endpoints are durable, Finished/QUALITY_REGRESSION/resume=false.
No128-or-later training is authorized by this closed run. No candidate exists;
the unused citation confirmation remains unopened. Independent A/B passed;
B recounted all256 raw/teacher rows and reproduced each arm's V16/VC16 once
(64 fresh generations). Final new SMALL64 updates/336 generations/256 teacher
calls are complete with UNKNOWN0. B does not reopen training or grant quality.
The following preregistered policy is retained to explain this result.

R3-ANSWER-MEAN-CITATION-1.0 preserves the accepted scalar4352 and the complete
VALUE-CITATION4384 negative endpoint. The latest independent review reproduced
that endpoint's V16/VC16; the historical CLI refusal remains a separate fact.
No previous failure, unused budget or acceptance record is reopened.

The only intervention is CE reduction. TOKEN retains sum of response NLL divided
by actual supervised token count. ANSWER uses the mean of each example's mean
response NLL. Actual binary response masks determine lengths; prompt/padding
are excluded and EOS included. Microbatch gradients are multiplied by their
normalizer and divided by the sum of normalizers: tokens for TOKEN, examples
for ANSWER. Logged token CE, answer CE, effective objective and normalizer are
distinct from actual input/target/padding usage. Scalar V/VC contributions do
not claim per-task gradient norms or Adam displacement.

Native ResumeBinding family6 denotes `response_ce_answer_mean_v1`, normalizer2,
first-target1, no span/annotation/auxiliary loss, execution1 and the exact
policy/corpus/tape/tokenizer/framing/config digests. This extends an unused tag
without changing tensor fields or descriptor widths. Existing readers reject
unknown family6; new readers preserve existing family1 semantics. Inference
can load the native model, while generic TOKEN resume rejects a moved ANSWER
file before optimizer execution. The parent-to-ANSWER change is an explicit
new objective fork retaining the parent Adam, not an implicit default resume.

Both arms copy the same4352 native and unchanged train4608/development1536
corpus, metadata, tokenizer and finite tape. TOKEN-CONTROL uses the first32
rows; ANSWER-MEAN uses up to3072. Constant LR3e-4, batch8/accumulation1,
V4/VC4, QE, LOCAL5/F32/Accelerate/thread1 and basic CE remain fixed.
Every batch contains76 actual targets: V4*2 and VC4*17. Its scalar mode weights
are2/19 and17/19 for TOKEN, one half each for ANSWER. General loss code never
uses those task lengths as constants. Full planned costs are input3699968,
target235904,padding99328 for3104 updates; all are recomputed from native data.

Direct checks cover independent F64 loss/logit gradients, empty/nonfinite masks,
EOS and padding, random TINY gradient/Adam under8/4+4/3+5/2+6 partitions,
native moved-objective rejection and actual new-process2 versus1+1 plus
last-step evaluation-only2+0. Random native continuation and the existing EOS
tensor evaluation fixture are disclosed separately. Scalar tolerance2e-6 and
native gradient/Adam tolerance2e-5 are fixed before execution; identical-path
resume and TOKEN raw/tensor/Adam comparisons require exact equality.
Use `replica-check quick --answer-mean` for only these boundaries.

Independent A precedes SMALL. `fresh answer-mean-prepare --previous PATH --output PATH`
binds the unchanged previous study and unused citation seal without creating
new examples or a new seal. After A, `fresh answer-mean-parent --study PATH`
checks the fixed parent16; `fresh run --root PATH/TOKEN-CONTROL` executes1 then31
in separate processes. Only complete verified negative quality termination,
matching original raw128 and tensor/Adam, permits ANSWER. Cancellation, storage,
binding, UNKNOWN and incomplete peer state remain blocking.

ANSWER similarly saves after1 and continues to32. Keep the original V64 guard:
FULL loss>=16, ALL4 loss>=4 or scorer errors>=4 stops for QUALITY_REGRESSION.
Early citation0/length alone does not stop. Subsequent evaluation is at
+128/384/768/1536/3072, with the previous panels and all unchanged joint gates;
safe non-evaluation saves keep segments <=512 updates. Eligible1536 stops
immediately; otherwise the registered maximum3072 is final. No new ratio,
LR/seed/loss/core exploration or automatic extension follows a miss.

Caps: new SMALL3104, input4000000,target260000, generation/teacher16384 each,
diagnostic backward0, active10800s, segment900s plus cleanup120s, RSS16GiB.
Direct plus independent TINY caps are96 updates,1024 generation/teacher each,
finite-difference64 coordinates, including failed tests. Zero-test filters
are not verification. Reports are pure readers and never synthesize evidence.

Independent B recounts both actual endpoints and uses
`fresh answer-mean-review --root ARM_PATH [--citation]` for V16/VC16 even on a
complete QUALITY_REGRESSION. This is reproduction only; it grants neither
resume nor candidate rights and leaves the old citation parity policy intact.
Malformed citation prefixes are separated from parsed IDs outside supplied
evidence. Only eligible ANSWER plus B may use the explicitly linked, previously
unused citation V256/VC256 seal once through the existing confirmation path.
Its immutable source bytes remain in the earlier study. The old scalar seal
is never used to select this model. Any success covers only two current records,
single-digit key/value and fixed eight-digit event citations; S4/S5/S6 and
Goal1 remain separate and unaccepted.

## Closed at retention guard: value and selected-event citation from4352

Actual execution on frozen source `04b5624f0855d9f1ff35c1a8a4a48df459ef3d8b`
stopped at4384 after32 new updates. Same-model V64 fell from FULL64/ALL4 16
to FULL41/ALL4 5 with15 generation errors; all three registered retention
conditions fired. The endpoint is durable, QUALITY_REGRESSION/resume=false,
candidate=null. Citation64 FULL0/value47/support0 is not acceptance.
There is no authorization to spend the remaining3040 updates, repeat the run,
change LR/seed/loss, or open confirmation. The budget, gates and conditional
later stages below describe the registered plan; they were not all executed.
Read-only independent outcome review remains separate from quality acceptance.
The original scalar4352 and its accepted confirmation are preserved.

R3-VALUE-CITATION-BRIDGE-1.0 preserves the accepted REBIND4352 native,
Adam136, tokenizer562, QE, LOCAL5/F32/Accelerate/thread1 and every previous
failure/receipt. Its two repairs separate normal quality-failure reviewer
reproduction from candidate/resume authority, and show confirmation's current
verified state separately from its immutable historical comparison. Historical
readers retain the recorded source/binary binding. Missing/pending/corrupt
evidence never triggers generation or replacement records.

After direct regressions and independent A, the single VALUE-CITATION fork uses
the same4352 weights/Adam, CE first-target1, constant LR3e-4, batch8/accumulation1.
The native train pool retains1536 value-only rows exactly, adds1536 citation rows
with original event IDs, and1536 citation rows differing only in two newly
assigned eight-digit IDs and their gold citation. The existing request grammar
and full answer `{value}입니다. [event:{id}]` are retained. No product resolver,
answer repair, vocabulary, core, auxiliary loss or storage change is permitted.

Each batch has four distinct bases and both queries: two V bases and two VC
bases. At3072 updates V rows receive8 exposures each and VC rows4 each, totaling
24576 samples. Every batch has one VC0 base and one VC1 base, two queries each.
The finite tape rotates V base pairs, offsets citation bases by192, and rotates
assignment and ID variant clocks; no model score chooses rows.
VC0/VC1 exposures are equal. Actual target/input/padding costs and
per-row exposures are prepared and bound; token weighting is not asserted to be
50:50 merely because sample counts are. Native parent_entry binds the new
corpus/tape and budget without resetting the parent Adam clock.
Existing sample traces provide CE/input/targets and scheduled token NLL.
Gradient norm and actual delta are measured for the whole batch. Separate
V/VC gradients are NOT_MEASURED; diagnostic backward remains0.

Vdev512 remains byte-identical. VCdev512 changes only the output request/gold;
VC-ID512 renames event IDs only. Their128 skeletons are shared, not1536 independent
scenes. New confirmation is independently reserved before learning:64 skeletons
excluded from train384, dev128 and previously used confirmation64, with new IDs
excluded from all train/dev IDs. V256 and VC256 use those same new64 skeletons.
The old confirmation is read only for preservation/report integrity.

First save is4353, followed by chunks of at most512 updates. Screen at+32/+128
uses V64/VC64; +384/+768 adds VC-ID64. At+1536/+3072 use all three dev512 panels
and fixed VCtrain128. A joint dev pass triggers Vtrain1536 and VCtrain3072 at
that same checkpoint before eligibility. The fixed citation train sample is the
first64 metadata rows of VC0 and first64 of VC1, retained in that order. It is a
training diagnostic, not heldout evidence. A first train miss preserves the result
and may continue only to the registered maximum7424. Evaluation-only time resume
adds no optimizer step and never retries a returned failure row.

Required FULL/QB/ALL4 minima: V old512 508/252/124, V new1024 1016/504/248,
Vdev512 488/232/116; VCtrain3072 3048/1512/744 and each variant1536
1524/756/372. Each citation dev512 independently requires488/232/116 plus
value508 and selected-event support508, no outside ID, no generation error and
all512 EOS. Report SB and ID_BOTH separately. Strict full strings, raw token
decode, normal greedy/EOS and sealed expected answers determine correctness.
Other provided-event citations remain support errors.

From+32, V64 FULL loss>=16, ALL4 loss>=4 or errors>=4 versus the fixed parent
sample stops for retention. Early low citation scores/length endings alone do
not stop learning; numeric, storage, binding, cancellation and UNKNOWN failures
remain sticky. Caps: new SMALL3072, input6300000, targets800000, generation16384,
teacher samples16384, diagnostic backward0, active10800s, segment900s plus120s
cleanup, RSS16GiB. Direct and independent TINY caps are optimizer96 and
generation/teacher1024 each, including failed tests. No automatic extension,
seed/LR search, new data or reinitialization is authorized.

Independent B recounts raw/native/trace and reproduces V16/VC16 even for a
normal final quality failure; this grants no confirmation or training authority.
An eligible5888 or7424 candidate can use the new V256/VC256 confirmation once,
after B. Both require FULL244/QB116/ALL4 58 and errors0; VC also requires value254,
support254 and outside-ID0. Only the value+eight-digit citation scope can then
be registered. The protected scalar4352 remains separately accepted. S4/S5/S6
and Goal1 are not granted by this study. A possible subsequent name/evidence
length study is planning only, with no learning under this budget.

## Closed: REBIND4352 minimal binding baseline accepted

Execution source `c52f7fbd1a60464cc55961b48d9b73952ee625b6` and its frozen
production binary completed768 additional updates from REBIND3584. Independent
A verified the actual preparation and TINY process boundaries. The4352 endpoint
passed all joint gates: old FULL512/QB256/SB256/ALL4 128,
new1024/512/512/256, dev508/252/252/125; errors0 and all EOS. It closed
CANDIDATE_FIXED_AT_4352/resume=false. No4864/5120 or extra candidate was run.

Independent B recounted full raw/native/Adam/trace and reproduced16/16 outputs.
The single original sealed confirmation then passed FULL254/256,QB126/128,
SB126/128,ALL4 62/64,errors0/EOS256. The confirmed result marks
MINIMAL_BINDING_BASELINE_ACCEPTED=true for two records, one-digit key/value,
fixed grammar and new semantic skeletons only. It does not grant S4/S5/S6 or
GOAL1_READY/ACCEPTED, and it does not change the product's default model pointer.

Actual new training consumption is768 updates,6144 sample exposures,
input890880/target12288/padding0, every1536 train row4 times; Adam clock/sampler
continues to4352. New SMALL generation2544 and teacher2240 include parent16,
scheduled2240, implementer16, independent16 and confirmation256. There is no
new optimizer/teacher in confirmation and no diagnostic backward. Unspent budget
is closed rather than rolled into another experiment. Existing parents and all
failed/closed records remain unchanged. Versioned source/model/data/recipe/raw
and independent evidence identities are registered in EXPERIMENT_STATUS.

One proposed next scope is to vary key length alone while retaining record count,
value grammar and normal generation, with a separate prepared train/heldout split
and the accepted short-key baseline checked alongside it. This proposal is
NOT_RUN; citation, time selection, wider language and new cores are not combined
with it. There is no automatic B/C/D training under this completed plan.

The following is the completed study's frozen design, not new run permission.

R3-REBIND-CONSOLIDATION-1.0 is a new, single REBIND-CONTINUE study from the
closed REBIND3584 endpoint. The previous expansion A/B and QS-R1 acceptance
remain closed. Neither the historical FINAL_QUALITY_FAIL/resume=false nor its
spent budget changes. There is no REPEAT training, reinitialization, new corpus,
loss, tokenizer, framing, architecture or learning-rate search.

The full native parent carries weights, Adam136, clock/sampler3584 and cumulative
input4157440/target57344. The new policy retains the byte-identical native corpus,
metadata and tokenizer, QE framing, ordinary digit+EOS CE/first-target1, batch8,
constant LR3e-4 and fresh budget origin3584 without resetting Adam or warming up.
It appends the original actually consumed tape suffix[2048..3584) once. The old
prefix is lineage only. Each of1536 train rows receives8 additional exposures:
12288 samples, planned input1781760/target24576/padding0. Midpoint exposure is
counted from the tape, not assumed uniform. Old512 and new1024 are both train;
dev512 remains unseen in learning, but previously observed in research.

The only chunks are first saved3585,3840,4352,4864,5120. At3840 evaluate the
registered first16 skeletons/four views per old/new/dev panel. At4352 and5120
evaluate old512/new1024/dev512 in full;4864 saves without a quality evaluation.
Each generated row has the existing same-model teacher observation, digit/EOS
full-vocabulary NLL and gold/foil c/d, with no extra backward or foil forward.
Parent first16 dev parity is mandatory after independent A. Last-step pure time
expiry may resume evaluation with optimizer0 and preserved Returned rows.

One checkpoint must simultaneously meet old FULL508/QB252/ALL4 124,
new FULL1016/QB504/ALL4 248, dev FULL488/QB232/ALL4 116, errors0 and full EOS.
SB is reported without adding a new gate. Full gate at4352 fixes that candidate
and stops learning; normal miss continues only within the registered cap. At5120
the result is a fixed candidate or final quality failure, always extend=false.
Unsupported steps, incomplete rows and mixed models fail closed. Same weights,
policy and step in different physical step/final native files are permitted.
Three-panel errors>=16 or simultaneous FULL and ALL4 drops of>=10 percentage
points in at least two panels stop QUALITY_REGRESSION. Integer comparisons use
the same parent cases: registered64 for the screen, complete panels otherwise.

New ceilings: optimizer1536, executed input2100000/target30000 including discarded
work, generation4800, teacher samples4608, active7200 seconds, segment900 plus
cleanup120, RSS16GiB. TINY implementer and independent checks together are capped
at optimizer64/generation256/teacher512. Cancellation, nonfinite values, I/O,
unreturned UNKNOWN and integrity failures remain sticky. No automatic rollover.

C0 verifies parent/native/Adam/raw/tape and preserves their actual hashes. C1
tests full native tape and policy boundaries, real TINY continuous2 versus fresh
process1+1 and final evaluation2+0, and all stage/gate boundaries. Independent A
reviews the frozen changed source and actual preparation before SMALL learning.
The existing scorer/publisher/RunControl and native formats remain in use.
Pure recount reports parent FULL/ALL4 gain/loss, NLL, c/d, usage and terminal.

A qualified candidate receives implementer16 and independent B16 parity. Only
after B accepts can the existing sealed confirmation256 run once, teacher0 and
optimizer0, requiring FULL244/QB116/ALL4 58, errors0/EOS256. This alone permits
MINIMAL_BINDING_BASELINE_ACCEPTED for two records, one-digit key/value and fixed
grammar. S4/S5/S6/Goal1 remain separate and unaccepted. Failure closes the study;
no alternate candidate, new confirmation or follow-up training is authorized.

## Closed: learned binding expansion — transfer improved, full gates unmet

Execution source b5e0d504a6a23ea8df9a7a1236690361cd13c86b was independently
approved for the QS-R1 repair and actual preparation, normally pushed and frozen.
REPEAT and REBIND each completed1536 new updates, absolute2048→3584, in five
fresh processes including the first saved update. Actual traces confirm
REPEAT old512×24 and REBIND1536×8, with identical per-slot targets, unchanged
LR3e-4/Adam/CE/QE/tokenizer and input1781760/target24576 per arm. No discarded
input/target or extra optimizer updates were observed. This plan has no budget left.

Final FULL/QB/SB/ALL4: REPEAT old509/253/253/125, new694/227/289/83,
dev292/84/110/26; REBIND old507/251/251/123, new1004/492/492/236,
dev476/220/223/99. Old/dev denominators are512/256/256/128; new denominators
are1024/512/512/256. Errors0 and EOS complete on every panel. Native bindings,
actual traces and all raw scores were independently recounted. These are same-step
3584 outcomes, not a combination of intermediate best panels.

REBIND improves unseen development selection but misses old/new/dev joint gates.
REPEAT passes old fit and fails dev. Both close FINAL_QUALITY_FAIL, Finished,
resume=false. No candidate is eligible, confirmation remains NOT_OPENED,
minimal baseline/S4/S5/S6/Goal1 remain unaccepted. Narrow progress is preserved;
no additional training, candidate sweep or automatic continuation is authorized.
Final parity/review details and physical identities are in EXPERIMENT_STATUS.

The following is the completed study's frozen design, not new run permission.

R3-LEARNED-BINDING-EXPANSION-1.0 preserves every old run, including the
QE2048 FINAL_QUALITY_FAIL/resume=false endpoint. The latest independent query
signal review rejected QS-R1 despite the earlier A/B reports. Those historical
reports describe different review dates; old model scores remain valid evidence.
This plan authorizes a repair and one comparison, not reopening that old budget.

E0 binds the actual QE2048 native file, tensor content, Adam, clock2048, tokenizer,
corpus, metadata and raw. E1 repairs only observer no-call deadline admission:
policy/sample/update/pre-update native binding, durable entry counters, uncommitted
update, unchanged saved native state and pure TIME_BUDGET must agree. A confirmed
NOT_INVOKED_TIME_PAUSE creates a new immutable attempt linked to the old finish.
Missing/corrupt finish, orphan attempts, cancellation, I/O, nonfinite values,
mixed conditions and partial/unreturned calls block; post-commit interruption
cannot repeat that update. Legacy successes remain readable, failures immutable.
Actual TINY subprocess RED/GREEN and independent fault tests precede learning.

E2 preserves old512 rows byte/order/target and adds two key pairs for each of128
old skeletons. The common pool has1536 rows: old512, new1 512, new2 512. Keys alone
change in the query and two record excerpts. Values, IDs/time/source/status,
physical order, system, limits, digit target and EOS remain identical per slot.
External episode/sequence/base/source-reference metadata is unique. Hypothetical
records never enter operating memory. Finite ranked candidates exclude old train128,
dev128 and confirmation64. Only the public reservation algorithm and seal digest
are used; confirmation examples/answers stay unopened. Capacity shortage blocks.
Actual token checks require146 prompt+target+EOS,145 input,144 response start,
identical two target tokens/masks, two provided records and zero excluded records.
Request-only resolution, swap negatives and conflicting-token-prompt checks apply.

Both arms inherit the same QE2048 weights/Adam and tokenizer562, LOCAL5,
CPU/F32/Accelerate/thread1, QE framing and ordinary full-vocabulary digit+EOS CE
with first-target1. LR3e-4 is constant, warmup/reset0, original clip/decay/betas.
Each adds1536 optimizer updates, absolute2048→3584. The512-step BOTH tape repeats
three times. REPEAT uses old512 rows24 times; REBIND cycles each logical slot
old/new1/new2 by visit modulo3 and uses1536 rows8 times. Both consume12288 examples,
input1781760/target24576. The actual key/edge frequency change is reported as part
of the coverage intervention. REPEAT's unused new pool is not called train fit.
Batch8 remains four logical slots with both queries, identical targets and shape.

Independent A must approve actual repair source and preparation. Commit/push and
freeze source/binary before SMALL. Parent dev16 parity and parent new64 generation
plus teacher run once. Parent old/dev64 use verified historical raw. Save the first
new update and continue in a fresh process. Segments remain at most512 updates.
Evaluate old64/new64/dev64 at absolute2304 and2816; final3584 evaluates
old512/new1024/dev512 at the same saved endpoint. Final evaluation can resume
without additional optimizer work. New observer-gradient probes are0.

Only two consecutive scheduled screens with both old FULL<32/64 and dev FULL<16/64
stop QUALITY_REGRESSION. Otherwise ordinary low scores do not trigger early
extension or search; execute the registered final subject to safety/budget.
Cancellation, NaN/Inf, shape/data/policy/source/storage errors and UNKNOWN block
the peer. A normally completed low-quality REPEAT permits REBIND. Stopped or
unequal endpoints cannot support a matched superiority claim.

Final old fit for both: FULL>=508/512, QUERY_BOTH>=252/256, ALL4>=124/128.
REBIND new fit: FULL>=1016/1024, QB>=504/512, ALL4>=248/256. REPEAT new is diagnostic.
Common dev: FULL>=488/512, QB>=232/256, ALL4>=116/128. All require errors0/normal EOS.
Only a final3584 candidate meeting all its gates is eligible. Tie order is dev
ALL4/QB/FULL, old FULL, then REPEAT. Recount full/QB/SB/ALL4, foil/other, digit/EOS
NLL and c/d separately; paired gain/loss uses128 whole dev skeletons.
Own and independent endpoint16 parity, independent B, then one fixed candidate
may open the existing confirmation256 once: FULL>=244, QB>=116, ALL4>=58, errors0.
Failure authorizes no alternate candidate, seed/LR/loss exploration or extension.
Narrow binding acceptance never grants S4/S5/S6 or Goal1 acceptance.

New SMALL cap3072; committed input3563520/target49152; executed including discarded
input4300000/target60000; generation5504; teacher/sample rows5120; extra gS backward0.
Planned generation5280 includes scheduled4864,parent new64,parent parity32,
two-arm own/review parity64,conditional confirmation256. TINY caps128 optimizer,
512 generation,512 teacher rows including failures; scalar work is separate.
Unknown killed work stays UNKNOWN, not zero. One heavy process, active10800s,
segment900s+cleanup120s, trainer16GiB. Compile time is not model performance.
Only related Rust source/tests and minimal plan/status are published; original
models, raw, corpus, Adam, DBs and scratch remain local. Report execution source,
report commit and actual remote SHA separately.

## Closed: query signal convergence — delayed train fit, joint gate unmet

Source a8c7f0fa261b2bf6804e02783d6f78bdcb0c7b28 was independently approved,
pushed and frozen before the single QE continuation. Actual new1536 updates
completed as1/511/512/512 in separate processes, from preserved QE512 to2048.
Input1781760/target24576,24 additional visits per row and every tape/Adam clock
were verified. ConstantLR3e-4, inherited Adam, QE/BOTH/CE and the production
binary remained fixed. No EQ, new seed, warmup, reset or additional search ran.

Final train FULL472/512, QUERY_BOTH216/256, SWAP_BOTH217/256, ALL4 96/128;
dev294/512,77/256,101/256,21/128. Both have errors0 and normal EOS but fail
the registered full gates. Final decision is FINAL_QUALITY_FAIL, Finished,
resume=false. The old512 decision remains closed and unchanged. No further
learning is authorized by this completed plan.

Train query discrimination improved after1024; full train mean d rose from
0.001144 at512 to3.602308 at2048. Dev mean d also rose to1.074871 but dev digit
NLL worsened from0.959152 to1.400022, with46 other-value responses. Thus train
selection and dev success remain distinct. Observation-only gS/Adam-delta probes
did not change CE or optimizer state; local linear agreement is not long-term
causal proof. Complete distributions and actual counts are in the status record.

Parent, implementer endpoint and independent endpoint fresh-process parity
each match16/16. Production raw/decision recount succeeds. New SMALL totals
2352 generation,2432 teacher/sample rows,16 diagnostic microbatch forwards,
6 diagnostic backwards; no unknown/discarded work. Confirmation remains sealed
and NOT_RUN_PREREQUISITE. CODE_LOW_FIX and observation noninterference pass;
TRAIN_BINDING/DEV_BINDING fail; minimal baseline,S4/S5/S6 and Goal1 remain
unaccepted. Product default is unchanged. Original files and failed tests remain.

The following is the closed study's registered design, not a new run permission.

R3-QUERY-SIGNAL-CONVERGENCE-1.0 preserves source32df6dcb1c86020b9fc5402ff3b8bc32f3180f0e
and the closed QE/EQ comparison. Its old decision and all endpoint receipts are
immutable. This is a separate parent-bound study from the actual QE512 native
weights/Adam, not permission to resume the old framing experiment. Product QE,
tokenizer562, LOCAL5, F32/Accelerate/thread1, BOTH512 corpus and basic response CE
remain unchanged. The only new training intervention is additional fixed exposure.

Q0 verifies physical/content/state/Adam hashes and original preservation. Q1 fixes
the final1024 action: extend is always false, with CANDIDATE_AT_1024 or
FINAL_QUALITY_FAIL_AT_1024. The512 action/gates/selection order remain unchanged.
Q2 independently recounts existing first16 skeletons at0/128/256/512 and the full
512 endpoints separately. For the same assignment's two queries, u0=za(q0)-zb(q0),
u1=za(q1)-zb(q1), c=(u0+u1)/2, d=(u0-u1)/2. Report both margins, positive pairs,
four-positive skeletons, strict generation scores, first-digit/EOS NLL and binary
NLL. Labels/evidence establish orientation; first-token and EOS losses are separate.

The fixed first4 training skeletons (16 rows) measure S=mean(d), equivalent to
mean oriented gold-minus-foil margin over16 rows. At local1/8/32, a separate
autograd graph computes gS using two batches of8 and denominator16. The existing
Adam callback only reads the actual post-clip/moment/decay delta. gS never enters
CE, moments, RNG or sampling. Read S and first-digit/EOS NLL before/after; local128
uses the same observation without backward. Record first-order residual rather
than calling the dot product causal proof. TINY observer-off/on and fresh-process
resume must preserve weights/Adam/clock/tokens/raw bitwise. Interrupted observation
is retained and blocks retry; unknown work is not zero.

Independent A must approve the actual source, directly affected checks, parent,
corpus/tape and immutable preparation before SMALL work. First additional update
is saved, then a fresh process continues. Register absolute512→2048 with no reset
or warmup, constantLR3e-4, original beta.9/.999/epsilon1e-8/decay.01/clip1,
batch8/accumulation1/seq256. Repeat the original512 tape three times: expected
new input1,781,760, targets24,576 and24 exposures per training row. Adam and sampler
clocks remain absolute. Each segment has at most512 updates; evaluation-only
continuation adds zero optimizer calls and reuses durable returned rows.

At768/1536 evaluate the same train64/dev64; at1024/2048 evaluate train512/dev512.
Both full panels must pass at the same checkpoint: train FULL>=508, QUERY_BOTH>=252,
ALL4>=124; dev FULL>=488, QUERY_BOTH>=232, ALL4>=116; errors0 and normal EOS.
At1024 success fixes the candidate early; otherwise continue to the registered2048
unless safety/budget stops. Two consecutive scheduled checks with no QB/ALL4 gain
and fixed-train64 first-digit NLL>=1.5 times the parent stop QUALITY_DIVERGENCE.
Current and parent fixed subsets are identical even when the full512 is evaluated.
Final failure is FINAL_QUALITY_FAIL. No EQ, seed/LR/loss/core search or extension.

Caps: new SMALL1536, executed input2,100,000/target30,000 including discarded work,
generation2816, teacher/sample-forward rows3072. Diagnostic microbatch forwards16,
backwards16 maximum, scalar finite-difference evaluations16; TINY optimizer64,
generation256/teacher256, including failed tests. One heavy process; active7200s,
segment900s plus cleanup120s; training16GiB, inference12GiB. Model calls, sample
rows and additional diagnostic backwards are separately counted. Compile time is
not inference performance. Cancellation, nonfinite/disconnected gradients, storage,
policy/data/source errors and UNKNOWN remain sticky.

Parent16 parity, final implementer16 and independent16 have separate receipts.
Independent B recounts native state, tape, raw/teacher and final parity. Only an
eligible fixed candidate may open the old sealed256 once: FULL>=244,
QUERY_BOTH>=116, ALL4>=58, errors0. Otherwise it remains NOT_RUN_PREREQUISITE.
Narrow selection acceptance, S4/S5/S6 and Goal1 are separate; product default and
GOAL1_ACCEPTED are never changed by this study. Commit/push named related source,
tests and minimal status only; freeze source/binary before learning.

## Closed: causal framing baseline — no joint signal at512

Frozen source32df6dcb1c86020b9fc5402ff3b8bc32f3180f0e was independently approved,
pushed and used unchanged for QE/EQ first1 plus new-process511 updates each.
Both512 endpoints are durable. Actual pair1024 updates, input1187840/target16384,
padding/discarded0; no conditional additional updates were executed.

QE train/dev FULL256/255, QUERY_BOTH0/1, SWAP_BOTH31/28, ALL4 0/0.
EQ train/dev FULL255/253, QUERY_BOTH3/4, SWAP_BOTH43/54, ALL4 0/0.
FULL denominator512, pair denominators256, ALL4 denominator128; final errors0.
Neither arm satisfies the joint extension rule or the full candidate gate.
The immutable paired decision is CLOSE_NO_JOINT_SIGNAL. Existing512 segment
TrainingPending/resume=true/TRAINING records remain untouched; the study decision
rejects further optimizer calls. Unused conditional budget is not new authority.

QE reproduces old BOTH512 tensor/Adam bitwise and all1408 recorded generation rows.
Implementer and independent fresh-process parity each match16/16 per arm. Normal
EOS and successful execution do not establish conditional selection. Confirmation
remains unopened/NOT_RUN_PREREQUISITE; MINIMAL_BASELINE is not established,
S4/S5/S6 are not accepted, GOAL1_READY/GOAL1_ACCEPTED remain false. Product default
QE and all original artifacts remain preserved. Independent A/B and production
comparison agree on execution/raw integrity PASS and development quality FAIL.
Final SMALL usage is1024 updates/2896 generation/2816 teacher; all calls returned,
not-invoked/unknown0. TINY26/208/208 remains separate, with its exact decode-forward
count unavailable from retained temporary evidence. No new test run substitutes
for that missing measurement. The status record retains all evidence paths/hashes.

The following is the completed comparison's fixed design, not permission to resume.

R3-CAUSAL-FRAMING-BASELINE-1.0 continues source13969c6fe2b7b9232297a0796b037b202add350b
and independently reviewed report4ba5a3d713f4de15c2fde3aa01021c61c17255c9.
All old endpoints, failures, Adam, tokenizer and data remain immutable. The only
intervention is question/evidence block order. QE retains native-role-bytes-v1;
EQ uses native-role-bytes-evidence-question-v1. System, record internals/order,
question bytes, answer and EOS are unchanged. The shared serializer is used by
samples, generation, teacher and verifier. Native resume binding, cache keys and
raw policy bind the actual framing. Generic product inference and exports that
lose the descriptor reject EQ. Product default remains QE.

F0 verifies the preserved BOTH corpus, initial tensor, tape and old512 endpoint.
F1 runs only directly affected literal block/mask/cache/binding tests and actual
EQ TINY continuous2 versus new-process1+1, including evaluation-only continuation.
F2 independently approves identical corpus512train/512dev, initial tensor,
fresh Adam-zero, tokenizer562 and paired tape before source/push/binary freeze.
Each row has146 tokens including digit+EOS. Both arms use SMALL9,513,408,
LOCAL5, CPU/F32/Accelerate/thread1, first-target1 CE, batch8/accumulation1,
seq256, warmup32 then constant3e-4, beta.9/.999, epsilon1e-8, decay.01, clip1.
No seed/LR/loss/core/storage search or automatic extension is authorized.

F3 runs QE and EQ with first1 durable update then511 in a fresh process.
Steps0/128/256 evaluate the fixed64train+64dev;512 evaluates all512+512.
First512 tape equals old BOTH exactly; each unique row appears8 times.
QE512 weights, Adam, cursor, counters and1024 raw outputs must reproduce old
BOTH512 bitwise. A mismatch blocks continuation and is preserved.

F4 is decided only after both verified512 endpoints. Any full gate stops at512.
Otherwise extend both by512 only if either arm has train QUERY_BOTH>=64 and
ALL4>=16, or dev QUERY_BOTH>=32 and ALL4>=8, with zero final generation errors
and valid execution. Repeat the identical512 tape without clock/Adam reset.
An immutable paired decision is revalidated against original512 raw on resume.
No joint signal closes both at512. At1024 evaluate all train/dev once, with no
additional intermediate panels. Segment arrays never exceed512 updates.

Full gate: train FULL>=508/512, QUERY_BOTH>=252/256, ALL4>=124/128;
dev FULL>=488/512, QUERY_BOTH>=232/256, ALL4>=116/128; errors0 and normal EOS.
F5 independently recounts all raw/teacher and regenerates first16 per arm.
Select only eligible candidates by dev ALL4, then QUERY_BOTH, then FULL, then QE.
Fix one candidate before independent one-time opening of the old sealed256:
FULL>=244, QUERY_BOTH>=116/128, ALL4>=58/64, errors0. Without eligibility it stays
unopened. Compare paired skeleton units128, not512 independent rows. Report
SWAP_BOTH, gold/foil/other/errors, identical query outputs, digit/EOS full-vocab
NLL and separately normalized gold/foil NLL; only the latter compares with ln2.

Caps: SMALL optimizer2048, input3,000,000, target40,000; generation5632,
self-teacher5120; TINY optimizer128/generation512/teacher512. Active7200seconds,
segment900 plus cleanup120, one heavy process. Record committed/discarded/unknown
work and durable state. Cancellation, numerical/storage/policy errors and UNKNOWN
block continuation; initial random zero quality is not a runtime failure.
Evaluation-only continuation adds no optimizer calls. Existing pending/confirmed
publication and RunControl remain authoritative. Closed old runs are not reopened.
Code execution acceptance, narrow selection baseline, S4/S5/S6 and Goal1 remain
separate; neither framing is automatically promoted to product default.

## Closed: foundation orbit — bounded comparison complete, binding baseline not established

Source13969c6fe2b7b9232297a0796b037b202add350b was independently preparation-bound,
pushed and frozen before the two real1+511-update runs. Both512 endpoints ended
normally with durable weights/Adam and resume=false/BUDGET_REACHED. The new
SMALL1024-update budget is exhausted; no continuation or unspent-budget transfer.

Final common train/dev FULL is FIXED274/236 and BOTH256/255 (each512 rows).
FIXED exposed fit213/256 is separate from its common train-orbit score.
Dev QUERY_BOTH46→1 and ALL4 1→0 fail the joint gate. BOTH answers identically
across253/256 dev query pairs, despite normal digit+EOS and no final generation
errors. Paired full gain143/loss124 does not establish binding quality. Independent
trace/Adam/raw/teacher recount and fresh-process parity are execution evidence;
they do not grant a model-quality pass.

No arm meets the registered train/dev thresholds, so confirmation is
NOT_RUN_PREREQUISITE and remains sealed from implementer evaluation. No candidate
was promoted. MINIMAL_BINDING_BASELINE_VERIFIED=false; S4/S5/S6 not accepted;
GOAL1_READY/GOAL1_ACCEPTED=false. All original and new failed-quality artifacts
are preserved. A later proposal requires a separate, evidence-based contract;
this closed comparison does not authorize another LR/seed/core/loss search.

The following is the completed study's fixed design and acceptance definition.

R3-FOUNDATION-ORBIT-1.0 preserves closed A512/LOCAL5/GLOBAL6 models, all source,
raw failures and native state. Base source a4267d308c233f7bab47e1c4bcf39f4f4e21d3cb;
base report76f296c95c2c66b0536bc4ce71a766a52b5f251d. No storage/core/tokenizer,
loss, LR search or old-run restart. Existing Rust trainer, immutable receipts,
RunControl, scorer and preparation authorization are reused.

D0 verifies existing A initial/final, corpus/tape/metadata/raw and independent
evidence. D1 takes the first8 train bases in saved order, both queries, and changes
only two evidence value digits. Query/IDs/order/metadata/system/limits are identical.
Reuse original raw; new normal-greedy generation16/teacher0/optimizer0. Report
original/swapped full/both, same output, gold/foil/other and swapped both within
the originally-both subset with its actual denominator. No score-based reselection.
This small observation does not impose an arbitrary score gate before learning.

Semantic skeleton means sorted key set plus sorted value set, four distinct
digits, excluding assignment/query/IDs/order/time. It differs from the old
digit-rotation helper. Finite universe1260 skeletons/2520 mapping scenes.
Retain actual old train128 skeletons and variant0 requests. Variant1 changes only
two value bytes and target digit. Same512-row pool in each arm:128×2assignments×2queries.
FIXED consumes256 original rows16 times; BOTH consumes512 rows8 times.
Unique-example/repetition differences are part of the assignment intervention.

Same old A512-update base/query order,4 skeletons×2queries/batch8,16 visits/base.
BOTH alternates assignment with64/64 initial directions; FIXED always variant0.
Actual input differences only at two framed value spans, target digit only,
same EOS/length/padding/tokens. Unselected pool rows never enter a gradient.
New dev128 skeletons×4=512 excludes all train skeletons. Independent confirmation
reserves another64×4=256. Finite greedy incremental squared marginal frequency
with fixed hash tie break selects each split without model scores/seed search.
Report actual frequencies/rule baselines. Individual digits/edges may overlap;
full skeletons cannot. IDs are split-independent; both assignments share IDs/order/
times. Physical and query positions balance. All actual prompt+target/EOS<=256,
provided2/excluded0. Independent sealing creates confirmation content before
training. Ordinary prepare/verify sees only its reservation policy/count and
later file digests. Existing transfer loader slot contains common dev, never
confirmation. Native confirmation train slot preserves the common training pool.

D2 independent actual preparation review precedes source/binary freeze, normal
commit/push and full remote SHA verification. D3 uses preserved untrained A
initial in both arms, fresh Adam0/clock0; no new random search. Same LOCAL5 SMALL,
tokenizer, CPU/F32/Accelerate/thread1, response CE/first-target1, accumulation1,
Adam beta.9/.999/epsilon1e-8/decay.01/clip1, warmup32→3e-4 constant. Each512
includes first1-save→new process511 without clock/warmup replay. Normal low-score
FIXED completion permits BOTH; cancel, I/O/numerical/integrity failure, UNKNOWN
or unfinished command blocks the peer. No automatic retry or extension.

Each arm step0/128/256 evaluates train16 skeletons×4 and dev16×4; step512 evaluates
train512+dev512. Each1408 generation/teacher; two arms1024 updates/target16384.
Total input<=2M/target<=20k including discarded work; generation<=4096
(planned2816+swap16+implementer parity32+reviewer parity32+confirmation256=3152);
self-teacher<=3072. TINY separately<=128 optimizer/512 generation/512 teacher.
Scalar/finite-difference calls separately counted. Active7200s, segment900s/
cleanup120s. Initial random EOS failures remain in raw; only accepted pure-time
continuation resumes, without regenerating returned rows.

D4 independently recounts FULL, QUERY_BOTH256, SWAP_BOTH256, ALL4 128 per512,
actual exposed train fit, gold/foil/other/error, same outputs and digit/EOS NLL.
Raw gold/foil delta and softplus(-delta) are separate binary-renormalized
diagnostics; only that NLL compares to ln2. No decoding restriction. Same-dev
gain/loss and uncertainty use128 skeleton units, not512 independent rows.
Old dev55/256 is a different split, not the new improvement baseline.

Candidate gate at one saved endpoint: common train512 full>=508/queryboth>=252/
ALL4>=124, dev512 full>=488/queryboth>=232/ALL4>=116, errors0/EOS all, fresh
parity16/16 and independent recount. FIXED exposed256 fit is separate; its
candidate gate is not reduced to256. If both qualify choose dev ALL4, FULL,
recorded segment cost, then deterministic arm name. Durably fix candidate before
reading independent confirmation; never switch candidate after its result.
D5 sealed256 requires full>=244/queryboth>=116/ALL4>=58/errors0. Passing grants
MINIMAL_BINDING_BASELINE_VERIFIED only for K1-V/two records/familiar digits/new
key-value sets. Failure closes the study. B/C/D/new names/fullQA/S4/memory/S5/
INT4/S6/Goal1 remain separate. No operating-model promotion or budget transfer.

## Closed: bounded selection learnability — A positive control not established

R3-BINDING-LEARNABILITY-1.0 completed its authorized A512 branch. Independent
preparation A/B PASS was bound to the exact native preparation before source
a4267d308c233f7bab47e1c4bcf39f4f4e21d3cb was pushed and remotely verified.
The production executable stayed frozen throughout the real1+511 updates.
Final train213/256, both85/128; dev55/256, both1/128; errors0. Native fresh-process
fixed16 parity16/16 and independent raw/receipt recount passed. A fails both
registered train and unseen-combination gates: POSITIVE_CONTROL_NOT_ESTABLISHED.
B/C/D and new-name probes are NOT_RUN_PREREQUISITE. The unused1536 updates are
not transferable and this is not a new continuation permission.

Actual committed/executed input593920, target8192, padding/discarded0,128 bases,
each example16 exposures;512 optimizer calls,816 generation including parity,
800 self-teacher. The final native is Finished/resume=false/BUDGET_REACHED.
Every final wrong output fails at its first value token and still has normal EOS;
most choose the other record's value. Learning execution/storage correctness did
not establish the minimum compositional selection capability. Preserve all raw.
S4/S5/S6 and Goal1 remain unaccepted; no final200 was created or opened.

The following is the completed specification, not authorization to rerun it.

Preserve the closed LOCAL5/GLOBAL6 runs, weights, Adam, tokenizer, raw failures,
and the historical GROUND cancellation. This is a separate training-only study,
not a continuation or product replacement. No new core, loss, tokenizer mapping,
backend, storage or user DB change. Rust, locked/offline Cargo, CPU F32/Accelerate,
one compute thread and one owned heavy process. External model/teacher/API0.

L0 separates observed failures from hypotheses with independent math review A
and data-design review B. Astra was actually invoked; Fable was unavailable and
has a handoff packet, not a fabricated review. Actual prepared corpus/native/tape
and source must independently PASS before L2. Source is committed/pushed and the
production executable frozen before any SMALL optimizer call.

L1 prepares A(short key,value), B(eight-digit key,value), C(short key,value+event
citation), D(eight-digit key,value+event citation). All four share the same seed17
random LOCAL5 SMALL tensors, fresh Adam0 and retained tokenizer mapping. The V/VC
comparison changes the explicit output instruction together with its answer
contract. It is not a pure character-length intervention. V citation is N/A.

Enumerate2520 canonical two-record key/value scenes from ten digits; choose128
train bases and128 dev bases, each with two opposite queries on identical
evidence. Four digits per base are distinct. Split identity excludes row/event
IDs and physical order. Full scenes are disjoint; individual edges/digits are
shared. Twelve full digit-rotation orbits plus eight reserved rotations balance
key/value marginals, and opposite assignments separate train/dev full scenes.
K8 maps the same ten keys to ten fixed names in both splits, with balanced
repeated digits/common prefixes. Event IDs use an independent stream and are
identical across arms. Source/status/times/context are common. All actual
prompt+answer+EOS lengths must fit256, provided=2/excluded=0; training and native
generation prompt IDs must match. Request-only label checks never enter inference.

The explicit512-row tape draws four different bases and both views per batch8.
Each32 updates is an epoch;512 updates give each of256 examples16 exposures.
Default masked response CE/first-target1; no auxiliary. LR32 linear warmup to
3e-4 then constant, Adam beta.9/.999, epsilon1e-8, decay.01, clip1, fresh clock0.
The first actual update is saved and resumed in a new process within512, not
replayed. Per-target CE uses already computed logits and adds no model forward.

L2 runs A only. Step0/128/256 evaluate frozen train32+dev64; step512 evaluates
train256+dev256. Thus800 generation and800 self-teacher calls per arm. Require
train full>=254/256 and both>=126/128; dev full>=244/256 and both>=116/128;
final errors0, normal greedy/actual EOS/strict UTF-8, and fixed16 post-load parity.
Failing full train closes POSITIVE_CONTROL_NOT_ESTABLISHED; train passing but dev
failing closes MINIMAL_COMPOSITION_NOT_ESTABLISHED. B/C/D remain NOT_RUN_PREREQUISITE.
Only A passing permits L3 B/C/D, each from the common initial tensor, each512.
The same full/both criteria apply to all arms. B/D passing the basic gate permits
one128-row new-name probe each; it never changes selection or authorizes more
training. Final200 is not created or used in this study.

Total SMALL<=2048 optimizer updates, input5M/target1M including discarded work;
generation<=4096/self-teacher<=4096, active7200s, segment900s/cleanup120s. Include
post-load16, independent at most16 per arm, and new-name probes in the ledger.
TINY direct regressions separately cap optimizer128/generation512/teacher512.
No seed/LR exploration or unused-budget transfer. NaN/Inf, leakage, identity or
storage errors, UNKNOWN and cancellation stop. Initial low accuracy and length
termination remain failure rows without a quality stop; control-token execution
errors retain the existing integrity stop. Clean time continuation only follows
the accepted pending/cursor rules. No accepted stabilization suite is repeated.

L4 independently re-scores saved raw and native identity, reports first value
token/body/citation/EOS/both and query sides, actual step/input/target/padding,
per-example exposure, last durable checkpoint and stop. Short diagnostic success
does not satisfy H3, S4, S5, S6 or Goal1. No operating model pointer changes.
GOAL1_READY/ACCEPTED remain false until their separate full contracts pass.

## Closed: identifiable baseline R3–R5, registered budget used, quality not met

The authorized replacement at `artifacts/identifiable-baseline-20260921-r3/`
completed4096 updates per arm,8192 total, using frozen source
936d42258a6143ae664fca71ef8294b760b1dffe and the retained production binary.
Independent A accepted the replacement preparation before learning. At1024 the
independent matched review verified GLOBAL6 fixed-train CE decreased11.1023%
from512; this satisfied the existing OR signal and allowed both arms to continue
to4096 without changing data, seed, LR, loss, tokenizer or Adam. It was not a
quality acceptance. Both final native endpoints are Finished/resume=false with
BUDGET_REACHED; no learning or automatic search remains authorized by this plan.

LOCAL5 final train31/64, primary199/512, transfer53/128; GLOBAL6 final28/64,
189/512,50/128. Both primary C/D/E both0/96 and fixed-train both0/12, with no
final generation errors. Both fail the registered development gate. Preserve
both artifacts; do not promote GLOBAL6 or claim LOCAL5 generally superior from
one seed and the small active mask difference. Existing-reference candidate
evaluation, final200/S4, S5 and S6 remain NOT_RUN due to the failed prerequisite.
GOAL1_READY=false and GOAL1_ACCEPTED=false. Detailed raw/usage/cost verification
and the bounded R5 decision are recorded in the latest experiment status.

The policy below is the completed study's specification, not a renewed budget.
The failed r2 attempt remains immutable and is not resumed. The same seed17
tensor values, tokenizer mapping, eight-task episodes, finite tape and schedule
were retained in the separately approved replacement.
Previously consumed optimizer/input/target/generation/teacher in the failed
attempt were0; its0.912484459s failed command and all prior preparation/parity
costs remain separately reported. Never reinterpret that failure as success.

R3-IDENTIFIABLE-BASELINE-1.0 authorizes a separate study. The GROUND cancellation
below remains historical and immutable: step6634, resume=false, unused790 not
reusable. No new auxiliary objective, P-weight continuation or backend change.

R0 preserves native/checkpoint/provenance; R1 reads existing frozen corpora and
executed draws without model calls; R2 prepares balanced native data and the
local/global control. An independent review A must PASS the exact preparation
before R3. The implementer must not manufacture that decision. R4 compares equal
steps and independently re-scores raw; R5 records the bounded design decision.

The new dataset has8192 train,512 primary,128 transfer examples, eight tasks and
two views per family. C/D keep identical evidence and change the query; E swaps
observed timestamps only. G changes missing-to-present evidence, the requested
entity from absent to present, or tied-to-ordered observation times. H swaps
observation times while retaining causal uncertainty. No pair repeats the same
model input. All names in a pair have equal digit length. Value
types share one distribution; event ranks, physical order, entity/value lengths
and numeric streams are separately assigned. Context keys are hash-partitioned
before families across the three splits; both record sides use the same finite
key pool, stratified below/above1000 and equally assigned to either query side.
The declared marginal cross-tables must match exactly. This does not claim
independence of every high-order combination. Existing train grammar only; full
key/value bindings and scenes cannot cross splits. Native reload, independent
request-only labels, full evidence and prompt+target/EOS<=512 are mandatory.

LOCAL5 is Config::small; GLOBAL6 changes local_layers to0 and uses the existing
experimental profile. Random seed17 tensors are initialized once and copied
with from_tensors. Tokenizer mapping alone comes from P6144. Fresh Adam zero
moments/clock, CPU/F32/Accelerate/thread1, batch8 (one case per task), adjacent
complementary views. Response+EOS CE, first-target1. LR3e-4, warmup128, cosine4096
to3e-5, beta0.9/0.999, epsilon1e-8, decay0.01, clip1. No SIDE/GROUND/SPAN.

Each arm<=4096 updates/20M input tokens; pair<=8192/40M. Segment900s/cleanup120s,
total active14400s; generation6400/teacher6400 including parity and later gates.
Reserve64 generations for independent final verification. The first real update
is saved and followed by a new process, within each arm's budget. No extra
SMALL preflight training. An absent/failed review, UNKNOWN usage, cancellation,
IO/numeric error or peer failure blocks learning. Input allowance includes
uncommitted work. At1024 both arms stop for matched-budget signal inspection;
partial evaluation cannot advance the optimizer beyond that checkpoint.

Evaluation:0 train32+screen32;256 screen64;512 and1024 train64+primary512+transfer128;
2048 screen64;4096 full panels again. Each arm2304 generations. Extend both after
1024 only if one arm improves512→1024 primary by16, CDE both by8, or fixed train
CE by10%. Otherwise INCONCLUSIVE_AT_REGISTERED_BUDGET; no automatic search.
The existing regression rule uses an attained best>=32/64, loss>=12 and
CE>=1.2 times the CE at that best, at two consecutive observations. Full-panel
raw/teacher rows supply the same fixed64 screen without extra model calls.
Step0 has32 cases and is excluded from this comparison. This preserves the
attained-best guard rather than comparing random initialization with P425.

New gate: primary>=487/512, every task>=58/64, transfer>=116/128, errors0,
CDE both>=87/96. Old primary/transfer gates and selector both>=173/192 remain
separate requirements for promotion. Then independent final200>=190 and each
category>=36/40, invalid accepted citations0; existing S5 memory/restart and S6
grouped INT4/M4 gates. Neither code/data checks nor the new dataset replace them.
If actual mask difference is zero, omit GLOBAL6 and do not transfer its budget.
The current prepared data has a small nonzero mask treatment, not evidence of
question information being inaccessible through the residual/global layer.

The corrected preparation received independent A PASS. Its first LOCAL5 command
then failed at step0 native save: the new state omitted the reused tokenizer's
source-corpus provenance. Optimizer/input/target/generation/teacher were all0;
GLOBAL6 was not started. Preserve that failed study and its missing final native;
do not rewrite admission/terminal records or silently restart it.

The narrow repair records the mapping's source corpus in the existing lineage
slot only when constructing an explicitly mapping-reusing new state. This slot
does not assert that the random weights learned the tokenizer's old corpus.
Existing resume and checkpoint validators remain unchanged; a missing lineage
on resume is still rejected. Direct TINY step0 save/new-process reload and
negative-provenance checks exercise the boundary without optimizer calls.
That earlier r2 comparison was NOT_RUN because it failed before update1. The
separately registered and independently accepted r3 replacement above completed
the actual comparison; it does not make the failed root resumable. Existing
failed studies and the following historical entries remain unchanged.

## Paused by user: GROUND cancelled after490 updates

The user requested stopping work and a detailed report. SIGINT reached the
existing RunControl; final native step6634 and actual usage were saved, command
exit1/CANCELLED/Failed/resume=false. Goal status is paused. No new training,
diagnostic, automatic retry or extension is registered by this report.
Unused790 updates are not automatically reusable. Existing parents/failed raw/
policies remain intact. Current source42af900c2f52264ed629e95560f63e7ae493108a.

Actual SMALL490 updates,838299 input/60205 target including discarded1688/132;
1160 generation/1160 own-teacher plus the earlier16 parity generations. Last
complete quality panel is+256: train49/64, primary350/512, transfer67/128,
flip8/192, both0/192, errors0. The cancelled490 endpoint is NOT_EVALUATED.
Compared with matched COVER4+256, primary347→350, transfer67→67,
flip10→8 and both0→0 do not demonstrate joint quality recovery. Final200 remains
NOT_CREATED/NOT_OPENED; S4/S5/S6 and Goal1 acceptance remain unmet.
The latest status section contains the source/raw/checkpoint identities, actual
counts, prior intervention table, known limitations and independent-review handoff.

## Prior registration: bounded train-only record grounding

Under the user's continuing one-variable authorization, prepare GROUND from the
same P6144 parent as COVER4. This is a hypothesis about learning record selection,
not a diagnosed attention implementation defect. The fixed-weight observation
below rejects question masking for its24 inputs, while development first-token
correctness remains3/12. Correct train outputs also lack uniform selected-record
attention dominance; attention magnitude alone does not prove causal use.

One variable only: retain COVER4's exact data/pool/tape/32 scenes/four values/
anchors/Adam/tokenizer/LR3e-5/first-target1 response CE/SIDE auxiliary0.1; add a
fixed training-only selected-record attention penalty. At each C/D/E sample's
first response position, use the last layer's existing masked attention weights.
Let A be attention summed across all query heads and tokens in the selected
record; B is the corresponding sum across both supplied records. Add
`0.1 * mean_selector(-ln(max(A / B, 1e-8)))` to the existing objective. No selector
sample means exact zero additional loss. Full record-content spans exclude role
markers; tokens/padding/masks and every generation computation stay unchanged.
Use train-only resolver/citation labels; never pass labels, selected-slot metadata
or restricted evidence to inference. No attention head, coefficient, LR or seed
sweep. Last-layer choice and coefficient are fixed before results, not optimized
on these observations. This experiment may fail and does not lower acceptance.

Bind objective family and exact annotation/policy in the existing native resume
identity; do not change tensor/storage layout or permit silent old-loss resume.
Reuse forward_observed inside the actual training pass, not an extra teacher
model. Direct scalar/gradient/mask/annotation/no-selector tests and exact TINY
continuous versus fresh-process resume must pass before any SMALL run. Verify
normal inference parity, same parent/Adam/data/tape, and freeze source/binary.
Preparation caps TINY64 optimizer/768 generation/768 own-teacher, scalar16;
actual direct forward/backward calls are separately reported. No new framework.

Preparation is now verified, including two preserved failed process executions.
Actual TINY totals50 optimizer/333 generation/333 teacher; scalar optimizer0,
direct random-native forward2/backward1 and scalar backward4. Corrected process
regression uses2 versus1+1 updates and the same16-row four-view tape; the prior
16 versus1+15 state comparison also completed before its separate, invalid
EOS-fixture weight-difference assertion failed. The final test instead checks
real objective inclusion and independently proves random-native gradients.
Production P16 parity matches16/16; no SMALL optimizer calls yet.

Proposed registered SMALL cap1280 updates,12M input/1M target including discarded
work,12000 generation/10000 own-teacher,21600s active,900s command/120s cleanup.
Same screens32/64/128/768 and full256/1280 as the retained COVER4 control; all
existing retention/no-progress/numerical/stop and joint gates apply. Only a new
explicit parent-bound root may execute after implementation checks. No training
has been registered or executed yet; no closed budget is extended. A joint pass
freezes the candidate for the unchanged independent S4/S5/S6 path.

## Closed: fixed DIVERSE selector-attention diagnosis

After COVER4's negative matched comparison, use the preserved DIVERSE7424
weights once to separate observable input visibility/first-token behavior from
unproven learning hypotheses. No optimizer or new autoregressive generation.
Select the first two actually exposed view0 train pairs and first two view0
development pairs per C/D/E, by existing tape/metadata order,24 cases total.
Bind the original train48 and development/selector raw, native physical/model,
tokenizer, policy and exact owned requests before calls. Reuse the existing
forward observer and RunControl/call publisher in an ignored diagnostic test.

Per case, observe last-prompt-query attention to trusted question/record0/
record1/system/framing token regions; then perform one unchanged reference
forward. Record all head/layer masses, allowed-token counts, entropy and logits.
Require exact observed/reference logits and argmax agreement with existing
normal-greedy first tokens. Expected selected-slot/first-token annotations are
diagnostic-only and never enter model input. Attention mass is not causal proof.

Caps SMALL optimizer0, generation0, prompt-only teacher forwards48 (24 observed
and24 reference), active900s/cleanup120s; TINY optimizer/generation/teacher0,
scalar0. Direct existing native observer test may perform TINY forward/backward
comparisons, reported separately. One process, no automatic retry, checkpoint
sweep or new training policy. Pending/failed attempts remain immutable; source
and diagnostic binary freeze before model observations. Production code stays
unchanged. Joint/S4/S5/S6/Goal1 gates remain unmet and unchanged.

Initial preparation rejected a merged-pool container ID against the earlier
unmerged train raw before output creation or model calls. Preserve the failure;
correct only the association to unique exact native-prompt digest plus expected
answer, keeping raw/model/content validation. Directly test duplicate and wrong
content rejection. One corrected preparation uses a new output path; no attempted
model call is retried and the48-forward cap remains unchanged.

Corrected source58a6f97583a545b50ace20c7a936af16ad769276 observation completed:
24 cases,48 prompt-only forwards,9752 processed prompt tokens,2.6192165s control
time, optimizer/generation0. All24 observed/reference last-token logits exactly
match and agree with the previous normal-greedy first tokens. Prompts186–241
tokens; question visibility is nonzero in all48 heads/layers per case.
Train first/full11/12 and both5/6; development first3/12, prior full2/12,
both0/6. These selected cases are diagnostic, not population quality estimates.
All raw/case/native/call/usage bindings verify; no numerical/cancel/UNKNOWN error.
The initial pre-call failure remains separate. No more model calls are authorized
by this closed diagnostic. Its evidence motivates the single grounding hypothesis
above without claiming an established attention or storage cause.

## Current: COVER4 closed; joint quality not recovered

COVER4 completed1280 updates at7424 and closed BUDGET_REACHED/Finished,
resume=false. Same-source pure report verifies native state, tape, raw, usage
and terminal binding. Primary375/512, transfer67/128, flipped15/192, both2/192
(C0/D2/E0), generation errors0. Matched DIVERSE control is399/70/4/0;
primary gain/loss24/48, transfer4/7. Both admission and joint gates fail.
The observed trade-off does not establish a transferable selector or justify
reopening this budget. Final200 NOT_CREATED/NOT_OPENED; S4/S5/S6 and Goal1 unmet.

Actual SMALL1280 optimizer,2160 generation,2144 own-teacher includes entry P16.
Actual input2190995/target157238 includes discarded1743/138; active1683.29954275s.
TINY36/129/129 and scalar0 are separate. No posthoc generation in this study.
The next action is a bounded fixed-weight diagnosis grounded in actual failed
selector inputs; no further learning is registered by this closure.

## Closed: four-view scene coverage, bounded COVER4 preparation

The user's continuing one-variable quality authorization covers a new P6144
research, COVER4, with retained DIVERSE1280 as the matched control. Actual
DIVERSE trained-view full178/192 versus development both0/192 motivates testing
independent scene coverage rather than adding another memorization-only score.
Only recurring C/D/E scenes increase8→32 per task; keep the same four value
views/native training pool, phrase/selector side order, co-batch8 and exact
other-five-task draws. Use the existing cover and diverse preparation options
together as this explicitly defined policy. All other mixed interventions fail.
The unchanged finite3840 tape consumes1280 updates; each of256 sides per C/D/E
receives5 exposures instead of64 sides×20. Same update/draw budget, not equal
FLOPs. Original parent/Adam/tokenizer/LR3e-5/first-target1 CE/SIDE family4
auxiliary0.1/F32/Accelerate/thread1 remain fixed. No old run is reopened.

SMALL cap1280 optimizer,12M input/1M target including discarded work,
generation12000/own-teacher10000, active21600s, command900s/cleanup120s.
Keep screens32/64/128/768 and complete256/1280, existing retention/no-progress
and all unchanged joint gates. Stop at first joint pass and hand off independent
final200; failure grants no automatic extension. No new heldout or donor values.

Reuse direct maximum-tape/native-pool tests and actual TINY continuous16 versus
fresh1+15 with all four views. TINY caps64 optimizer/768 generation/768 teacher,
scalar0, no automatic retry after unknown use. Verify exact Adam/weights/cursor,
pre-call pool-tamper rejection, immutable parent and other-task slots. Reuse
existing pure mapping regression; no general harness or storage change.
After checks, freeze/publish source and executable, run same-source P16 parity,
verify physical inputs against DIVERSE, then register a new root and start SMALL.
These were the preparation caps; the completed execution is recorded above.

## Current: actual training-view fitting verified; scene transfer remains unresolved

Fixed DIVERSE7424 has full178/192 across its four actual training views and
both83/96 pairs. Every side had20 executed exposures. Original/view1/view2/view3
full44/42/45/47 of48 and both21/18/21/23 of24; generation errors0.
This includes143 new observations and one verified train64 reuse; original48
is retained evidence. Diagnostic calls are exhausted and its optimizer count0.
These are trained scenes, not development192 or independent acceptance.
Development both0/192 and recombination both4/24 remain unchanged.

The trained/unseen scene gap motivates a single next coverage comparison:
increase independent C/D/E scenes from8 to32 while retaining the four value
combinations, P6144/Adam/LR/objective/tokenizer and exact other-task draws.
Do not infer a unique architecture/storage defect or that repeating eight scenes
alone will achieve the unchanged joint gates. A new bounded registration and
direct input/tape/resume checks must precede any such learning. Existing closed
DIVERSE/VALUE/COVER endpoints and their no-progress decisions remain immutable.
No additional optimizer run is registered by this diagnostic closure.

## Closed: fixed DIVERSE actual-view fitting diagnosis

Read-only tape/raw audit found48 actually trained sides per C/D/E value view,
twenty exposures each. Prior original48 covers view0; train64 overlaps just two
view0 and one view1 side. Views1/2/3 therefore have47/48/48 unobserved sides.
Register one fixed DIVERSE7424 diagnostic under the user's continuing quality
investigation authorization. Do not reopen its optimizer budget or native state.

Use the actual hash-bound training-values pool and first eight executed pairs
per C/D/E, retaining exact request/answer/phrase/selector identity. Observe views
1,2,3 once each; reuse the one identical input/expected answer already present in
the same native endpoint's verified train64 raw, keeping its original provenance.
Do not create a new call row for reused evidence. Report completed new calls and
full48-case derived scores separately. View0's44/48 and all original raw remain.

SMALL optimizer0, generation143/own-model teacher143 maximum; TINY/scalar0.
One process at a time, command900s/cleanup120s, active1800s, no automatic retry or
checkpoint sweep. Source change is confined to the existing ignored diagnostic
and direct pure mapping tests. Production inference/trainer/model/data unchanged.
Source/executable and selection/native hashes freeze before observations.
This measures fitting of actual training views, not heldout quality or approval.
No new learning is registered; use the results to justify a next single variable.

Pre-call view1 preparation failed because the diagnostic incorrectly required
evaluation and final native files to have identical physical hashes. No output
directory or model call was created. Preserve that failure; narrowly reuse the
existing audit_panel physical/state checks and terminal model/step comparison.
Freeze the corrected executable and use new `view1-verified` through
`view3-verified` outputs once each. This explicit preparation correction adds no
generation/teacher budget and permits no retry of an attempted model call.

## Current: value diversity closed; selection remains unresolved

DIVERSE reached its1280-update cap at7424 with primary399/512, transfer70/128,
flipped4/192, both0/192 and generation errors0. It closed Finished with
NO_FURTHER_PROGRESS/resume=false. Matched VALUE control is393/72/2/0;
primary paired gain/loss21/15, transfer0/2. Neither meets admission or joint gates.
Same-scene original fitting drops48→44/48, while fixed recombined inputs improve
only22→23/48 and both3→4/24. All diagnostic raw/case/call bindings verify.
This does not establish transferable selection or a storage/backend defect.

Actual SMALL1280 optimizer/2256 generation/2240 own-model teacher include the
entry16 parity and endpoint96 diagnostic calls. TINY56/462/462 is separate.
No further execution is authorized by this closed registration. Preserve all
previous failures and native states. Final200 NOT_CREATED/NOT_OPENED; joint
development/S4/S5/S6/Goal1 remain unmet. Any next one-variable study requires a
separate bounded policy grounded in these results; no new study is registered.
Before choosing it, separate insufficient fitting of the four trained views
from transfer to unseen scenes using existing raw and the frozen draw metadata.
Do not infer that one extra correct diagnostic case resolves the failure or
automatically reopen the existing no-progress endpoint.

## Closed: bounded training value-association diversity

The user's continuing one-variable authorization covers DIVERSE, a new explicit
P6144 research with the retained VALUE1280 as matched parent/update control.
Do not resume or modify any closed VALUE/COVER/wording/diagnostic receipt.
Parent is the preserved P-PHRASE6144, physical
c4d6989fbcaad60516053dda206caf43abc08c4b388c3f765e4c446864f60e20.
The native reader and same-source parent16 parity must prove its exact weights,
Adam, tokenizer and objective before registering a SMALL run.

Only train value-association diversity changes: same first8 recurring scenes
per C/D/E, selector sides, phrase exposure, anchors and batch8/co-batch tape;
four combinations instead of VALUE's original/exchanged two. Additional views2
and3 clone the original view0 request and change respectively record slot0 or1
to a distinct owned-train value of the same numeric/text class and numeric width.
Choose the first eligible value in sorted owned-train values, independent of
model output. Retain the other record value, ID/order/status/times/question and
selected citation. Derive labels with the existing training-only request resolver.
These single-record changes cannot duplicate the prior diagnostic inputs that
changed both records to values disjoint from the original pair. No heldout text
or answers are consulted or used for training.

Reuse R3CORP for a separate typed training-values pool, bound in the existing Plan
and native resume policy. The trainer and actual-update audit use that same pool;
train64/primary/transfer/selector evaluations retain their original frozen files.
Original/phrase/selector sources and tokenizer remain unmodified. Only indices
for C/D/E views2/3 use the new content; other5 tasks stay byte-identical.
The finite3840 tape is frozen, but this study consumes only1280 updates. Each
selector side/value combination receives20 exposures, versus40 in VALUE; same
total draws, fewer repeats per combination. Do not call this matched FLOPs.

Keep LR3e-5, first-target1 response CE, SIDE family4 auxiliary0.1, original Adam,
F32/Accelerate/thread1/model. SMALL max1280 updates,12M actual input/1M target,
generation12000/own-model teacher10000, active21600s, command900s/cleanup120s.
Same screens32/64/128/768 and full256/1280, same retention/no-progress/error gates.
No automatic extension. Joint development/S4/S5/S6 criteria remain unchanged.
After closure, reuse original/value-recombination diagnostic once at the same
fixed endpoint, never promoting train-scene scores to heldout quality.

Direct regression caps: TINY96 optimizer/768 generation/768 teacher, scalar0.
Use the existing actual native continuous8 versus fresh1+7 path, plus unchanged
VALUE continuous4 versus1+3. The new TINY fixture uses4 owned train bases rather
than2 solely to supply a third text value; product SMALL still uses the unchanged
256-base corpus. Pure native roundtrip, one-record-change invariants, full3840
tape bounds/exposures and pre-call tamper rejection must pass. Freeze and publish
source, verify remote SHA, then register a new root before any SMALL learning.

## Current diagnosis: correspondence fails on new values in familiar scenes

Fixed VALUE7424 original48 remains exact with byte-identical raw token output.
Changing only to disjoint values already present elsewhere in owned training
drops full48→22/48 and both24→3/24, while citation remains45/48. Sixteen errors
copy the other supplied record's value, whereas only1 repeats the complete old
selected answer. First teacher/greedy argmax agrees48/48. All generation errors0,
SMALL optimizer0;96 generation/96 teacher completed within the registered cap.
These observed cases support testing value-to-record correspondence across more
training combinations, not simply changing storage, backend or question wording.
They do not prove a unique mathematical cause or establish heldout improvement.

Next narrow hypothesis is training value-association diversity at fixed scene,
selector pair, phrase exposure, anchor rows, parent/Adam/LR/objective/tokenizer.
A separate bounded policy must retain a matched control, verify changed-value
labels from requests, freeze train-only donors and keep this diagnostic out of
training. Do not reopen the closed VALUE/wording budgets or treat these48 cases
as a substitute for primary/transfer/selector/final200. No new optimizer run is
registered by this diagnostic closure. Goal1 gates remain unmet and unchanged.

## Closed: fixed VALUE endpoint, unseen value-combination diagnostic

Register a read-only diagnostic before any further learning. Parent is the closed
VALUE7424 native at `artifacts/pair-value-20260920/VALUE/segment-0002/final`,
physical09b0f972cebf37cd0ea1006a1a606176282c4a79d722374aa305ca9949a1e1b1.
Its original and exchanged train views previously scored48/48 each, both24/24,
while development both remained0/192. This diagnostic separates fitting those
two values from copying other supplied values under the same trained selectors.

Reuse `paired_seen_train_diagnostic`, actual native generation, RunControl and
binary call/panel receipts. Recheck the same original48 with the new diagnostic
binary against retained raw tokens. Then change only the values in both records
of those48 cases: choose the first two-value donor in the owned train order with
distinct values, neither in the old pair, same numeric/text class and numeric
width. Preserve entity/context/record ID/order/status/times/question and citation;
request-only resolver verifies the new labels before calls. No expected answer
or resolver is an inference input. Text lengths may differ; not matched FLOPs.
Reject any changed prompt already present in the combined owned train sources.
New cases remain diagnostic development data and do not enter training/final200.

Only the VALUE7424 model is observed: optimizer0, at most96 SMALL generation and
96 own-model teacher calls including the48 original parity cases; TINY/scalar0.
One process at a time, command900s/cleanup120s, total active1800s, no automatic
retry or checkpoint sweep. Use one new evidence root
`artifacts/value-combination-20260920-evidence/`. Pure fixture checks execute no
model calls. Failed preparation/calls and unknown usage remain preserved/blocking.
Report full/body/citation/EOS, both, old-answer repetition and actual token use.
Success on these combinations is not heldout quality; failure is evidence for
value/selector transfer work, not proof of one optimizer or tokenizer defect.
No new learning is registered here; all prior terminal budgets remain closed.

## Current result: wording helps some development cases; joint gate remains unmet

The registered selector-wording study is closed at9984 after2560 new updates,
BUDGET_REACHED/Finished/resume=false. Final primary386/512, transfer67/128,
flipped33/192, both12/192 (C5/D7/E0), generation errors0. Same-parent/budget
fixed-exposure control is376/64/20/9. Paired gains/losses are35/25 primary,
8/5 transfer and5/2 both. View3 improves flipped0→9/48 and both0→3/48, but
numeric both is1/96 and time selection both0/64. Parent retention remains below
392/68. These limited changes do not satisfy admission or the joint gate.

The cap was reached before final evaluation finished. The preserved pure time
stop saved9984; a new process completed evaluation with optimizer0 and identical
native checkpoint bytes. All raw/call/teacher/native/counter audits pass. Actual
SMALL2560 optimizer/3160 generation/3128 own-model teacher include both entry
parities; separate TINY92/495/495. No additional C++ calls. The existing closed
experiments, source data, failed setup and parent state remain unchanged.

Do not extend or restart this study, substitute an intermediate best score, or
interpret storage parity as model quality. Current evidence still leaves value
and time-selection generalization unresolved. The trained original/value-swapped
diagnostics use two trained value views, so their fitting scores do not prove
unseen-value conditional copying. A further hypothesis requires its own bounded
registration and evidence; no next optimizer call is authorized by this closure.
Final200 remains NOT_CREATED/NOT_OPENED; S4/S5/S6 preconditions and Goal1 unmet.

## Closed: one bounded selector-wording exposure study

The user's continuing one-variable authorization covers a new schema17 research
from the intact COVER7424 parent, physical
9845a43db02f197fca6026db058c503c1fa8856ffd673267904103c03b2e98e0.
Retained schema16 fixed exposure is the matched-parent/budget control. Neither
closed run resumes or changes its candidate status. New root is
`artifacts/selector-phrase-20260920-final/`; internal arm remains COVER to reuse its
unchanged objective. Register only once after direct checks and parent16 parity.
The earlier preparation root is retained with optimizer0: final Clippy found a
new style warning, fixed by the equivalent integer divisibility method before
learning. It is not an additional learning arm. Two parent16 source-parity runs
are counted32 total against the same generation allowance; only the final root
may perform the2560 optimizer calls. No model execution failure is reopened.

Only question-wording exposure changes. Keep the same32 C/D/E scenes per bucket,
both value views, paired selector sides and all anchor update/slot rows. In the
remaining tape1280..3840, cycles satisfying `(cursor/64)%4==0` substitute owned
view3 for view0, using that scene's original question wording and preserving the
side. This changes1920 of20,480 draws; all answers, evidence, order and times are
identical for every replacement. Each original-value side receives10 existing
wording and10 view3 exposures; exchanged-value sides retain20. P wording remains
in the other cycles; no balanced three-wording exposure is claimed. Use only
already-owned train episodes, not development questions, new facts or a new
corpus. Native readers and request-only label checks verify each replacement.

Keep weights/Adam/tokenizer/F32/model, LR3e-5, first-target1 CE and SIDE family4
auxiliary0.1. Max2560 SMALL optimizer,7424→9984; actual input12M/target1M,
generation12000/own-model teacher10000, active21600s, command900s/cleanup120s.
Carry the same prior retention streak and full-point history as schema16.
Same screens32/64/128/512/1536 and full1024/2048/2560. Numeric/save/cancel/
UNKNOWN and no-progress guards remain. No automatic extension. Same checkpoint
must meet the unchanged joint gate before independent final200/S4/S5/S6.

Direct regression limits: TINY96 optimizer/768 generation/768 teacher, scalar0.
Reuse actual COVER setup and continuous8 versus fresh1+7 for existing and changed
tapes. TINY starts a new eight-row fixture cycle explicitly; SMALL preserves the
full tape cursor. Pure full3840 writer/reader and changed-question invariants
must pass. Source/executable freeze and publication precede actual SMALL learning.
Final comparisons use complete same-step raw and strict full/citation/EOS;
wording-stratum gains alone cannot replace overall quality or numeric selection.

## Current diagnosis: selection does not transfer across wording and numeric values

Read-only recount of the closed COVER7424 and9984 endpoints confirms their
primary392→376, flipped15→20 and both3→9. At9984, both is9/96 for nonnumeric
values and0/96 for numeric values. View3 has0/48 flipped and0/48 both, whereas
views0/1/2 have8/7/5 flipped and4/4/1 both. All required raw/teacher/call/native
bindings and request-only labels validate; no model calls or optimizer updates.
View2 duplicates view0 input/order in24/48 scenes; it is not always an order
reversal. Preserve the frozen panel and its denominator. These are correlated
development strata, not independent tests or proof of a tokenizer/format cause.

The narrow next hypothesis is missing selector wording exposure: reuse the
owned train view3 wording on a deterministic subset of the same COVER scenes,
preserving facts, answers, value-view exposure, evidence order, phrase diversity,
anchors, parent, Adam, LR and loss. A separate bounded registration and direct
input/resume checks must precede any optimizer call; compare with the retained
same-parent fixed-exposure control. Do not add order/value/architecture changes
or repeat the closed experiment unchanged. No new learning is registered by
this diagnostic entry. Numeric-value generalization remains unresolved.

## Current result: exposure helps fitting but fails development recovery

The separate2560-update research is closed at9984, BUDGET_REACHED/Finished/
resume=false. Same trained subset improves original23→44/48 and exchanged24→45/48,
both5→20 and4→22/24, errors0. It still misses47/48 and23/24 fitting gates.
Development primary392→376/512, transfer68→64/128, both3→9/192, all errors0.
Other-five-task primary297→300/320; original selection95→76/192 accounts for
the aggregate decline. Both C3/D6/E0, independent bases4, no complete base4.
This is limited selection change with worse total development, not adoption.

Actual2560 optimizer,3240 generations and3224 own-model teachers include entry
parity and final diagnostics; direct TINY48/312/312 are separate. Pure native
recount and strict raw/call verification pass. Model quality and Goal1 do not.
No C++ calls or changes to original parents/closed budgets occurred. Do not
resume this endpoint or automatically add exposure. The next unresolved question
is transfer of conditional selection beyond the trained scenes while retaining
original answers; no next learning intervention is registered here. Keep the
unchanged joint/S4/S5/S6 gates and final200 NOT_CREATED/NOT_OPENED.

## Closed: fixed COVER exposure, separate bounded research

Under the user's continuing one-variable authorization, test exposure duration
from the intact closed COVER7424 endpoint. Parent physical SHA256
9845a43db02f197fca6026db058c503c1fa8856ffd673267904103c03b2e98e0,
modeld8b2472422605e9305f2a25d8c39cbac555161d65d1303ca4b149c392dc60403.
This is not G3 admission or adoption of its failed development scores. Preserve
its BUDGET_REACHED/Finished/resume=false and every old study. The new schema16
registration binds that parent/Adam/terminal, the actual preceding decisions,
fresh16 parity and unchanged originalP6144 retention baseline. It must reject
failed, incomplete, wrong-policy parents and a second continuation of itself.

Only duration changes: at most2560 additional SMALL updates,7424→9984, taking
the original COVER tape cursor1280→3840. Keep all3840 rows and first_step6144;
the new run does not rewind the tape. Same32 train bases per C/D/E, both owned
value views,6 selector slots every second update, original anchor rows, original
P phrase policy, LR3e-5, first-target1 CE, SIDE family4 auxiliary0.1, weights,
Adam, tokenizer, F32/backend and model. Each of384 selector sides gains20
exposures, for30 total. No new corpus or heldout training, C++ calls or framework.
Duration changes cumulative anchor exposure too; it is not a matched-compute
comparison. Narrow VALUE40 exposures remain a separate historical observation.

Limits:2560 optimizer,12M actual input/1M target including discarded work,
12000 generation/10000 own-model teacher,21600 active seconds, command900s and
cleanup120s. Direct TINY cap48 optimizer/384 generation/384 teacher, scalar0;
its explicitly shortened eight-row continuation shifts first_step at a new
eight-update cycle, while a pure full-size test checks the unchanged SMALL cursor.
Run affected VALUE and COVER continuous/split native paths, direct tape/guard
checks and fresh parent16 parity before source freeze/registration/learning.
Save and restore the first actual update within this budget.

New screens32/64/128/512/1536, full1024/2048/2560. Keep original screen baseline,
carry prior guard streak and all prior full-point progress into the existing
plateau rule; never reset a failed guard. Numeric/save/cancel/UNKNOWN failures
remain blocking. At a completed final endpoint, run original48/value48 fitting
diagnostics once each. These use the retained subset, not all96 COVER pairs.
All old fitting, joint development and independent S4/S5/S6 gates stay unchanged.
First joint pass freezes the candidate and proceeds to the authorized gates.
Otherwise close honestly at the registered stop, without automatic extension.

## Current result: wider scenes remain underfit at the registered exposure

COVER completed1280 updates and all final observations. Primary392/512,
transfer68/128, flipped15/192, both3/192 across2 bases, errors0. Retained VALUE
was393/72/2/0. Both gains are limited and do not satisfy retention or the joint
gate. The same first8 train pairs per bucket score original23/48/both5/24 and
value-exchanged24/48/both4/24, versus VALUE48/48/both24/24 on each. Ten exposures
per side across the wider pool did not establish fitting, whereas40 across the
narrower pool did. This does not isolate insufficient duration from interference,
data coverage or shared model/objective limitations.

The next single-variable hypothesis may test exposure at this same fixed scene
pool, because its trained examples remain inaccurate. It requires a separate
bounded registration, unchanged data/objective/LR/Adam/tokenizer, explicit parent
lineage and retention guards. This closure grants no automatic extension and
does not reopen COVER, change its resume=false or promote its checkpoint. Do not
change several factors to chase the three development pairs. Final200 remains
NOT_CREATED/NOT_OPENED; S4/S5/S6 and Goal1 are unfulfilled.

## Closed: COVER, one bounded scene-coverage intervention

Under the continuing one-variable authorization, register COVER from the original
P6144, compared with retained VALUE at matched256/1280 updates. Increase the
recurring prefix from8 to32 paired train scenes per C/D/E. Both existing value
views remain trained; each64-update pass alternates original/exchanged values.
This pool-size change necessarily reduces exact-side exposures40→10 and lengthens
the value-view recurrence16→64. These are disclosed consequences, not a claim
of equal per-case exposure. Use no new corpus, no heldout training and no new
model, LR, loss, Adam, tokenizer, decoding or storage setting. Preserve all
other-five-task update/slot rows, SIDE family4 auxiliary0.1 and LR3e-5.

The hypothesis is transfer across more independent scenes after successful
fitting of both value views. It is not evidence that coverage is the unique cause.
Retain all closed studies and decisions; no C++ calls or old resume are needed.
Max1280 SMALL optimizer,12M actual input/1M target including discarded work,
12000 generation/10000 own-model teacher,21600 active seconds,900s command/
120s cleanup. Direct TINY cap40 optimizer/384 generation/384 teacher, scalar0.
Run full3840-row tape roundtrip/bounds/exposure checks and both affected TINY
continuous/split paths (COVER8 versus1+7, retained VALUE4 versus1+3).
Production P16 precedes registration, commit/push and source/binary freeze.
Save/reload the first actual update within the1280 allowance.

Keep screens32/64/128/768 and full256/1280, original retention/plateau/numeric/
storage/cancel/UNKNOWN guards and joint development/S4/S5/S6 gates. No automatic
extension or additional arm. At a fully completed endpoint reuse the two48-case
diagnostics once each,96 generation/teacher total within the budget; these are
the retained first8 train pairs per bucket, not a census of all96 COVER pairs.
Their47/48 full and23/24 both thresholds concern fitting only. Any joint pass
freezes the candidate and proceeds to the authorized independent gates.

## Current result: changed-value fitting succeeds, transfer remains unresolved

VALUE completed its registered1280 updates and is closed at7424,
Finished/resume=false/NO_FURTHER_PROGRESS. Original48 and exchanged48 both score
48/48, both24/24, errors0 in fresh processes. The old FIT control scores48/48
and10/48 respectively. These are training scenes: VALUE explicitly trained both
views,40 exposures per side, whereas FIT trained only the original80 times.
This proves fitting of the introduced variation, not unseen-value generalization.

The same VALUE endpoint scores primary393/512, transfer72/128, flipped2/192,
both0/192, errors0. Matched FIT was398/76/2/0; parent was425/79/2/0. The narrow
intervention did not recover joint development quality and is not adopted.
Pure native report and strict raw/call-receipt recount pass. No execution,
numeric, storage or UNKNOWN failure was observed; neither this result nor the
closed Rust/LibTorch parity proves all shared model/data assumptions correct.

The next useful single-variable hypothesis is coverage of independent scenes
while preserving the learned original/value-exchanged pairing. It must explicitly
measure the coverage/repetition tradeoff against these retained controls; simply
adding more exposure to the same96 sides is not supported as a quality solution.
This is a hypothesis, not a registered experiment or a proven root cause. Do not
reopen VALUE, FIT or the C++ comparison. A new bounded policy must precede any
further optimizer calls under the user's continuing one-variable authorization.
Final200 remains NOT_CREATED/NOT_OPENED; S4/S5/S6 and Goal1 remain unmet.

## Closed: one value-exposure intervention from original P6144

Register VALUE under the continuing one-variable authorization, comparing with
the retained matched-budget FIT execution. The fixed FIT model fits48/48 but
scores10/48 when only supplied values exchange, including24 old exact answers.
Change only C/D/E value-view exposure: the same first8 paired train bases per
bucket alternate existing view0/view1 every16 updates. Retain the exact anchor
slots, question/phrase/selector sides/IDs/order/times, original P6144/Adam,
tokenizer, SIDE family4 auxiliary0.1, first-target1 CE, LR3e-5 and native model.
Registration verifies each changed owned request differs only in its values.
No corpus generation/editing or old terminal/budget changes. At1280 there are
40 exposures of each of96 sides, versus FIT80 of each of48: that coverage versus
exact repetition tradeoff is explicit, not a second hidden hyperparameter.

Cap1280 SMALL optimizer,12M actual input/1M target including discarded work,
12000 generations/10000 own-model teachers,21600 active seconds,900s command/
120s cleanup. Direct TINY cap24 optimizer/256 generation/256 teacher, scalar0.
Reuse full3840-row writer/reader/tape checks and an actual TINY continuous4 versus
fresh1+3 across the value boundary. P16 production parity precedes source/binary
freeze; first actual update is saved and restored within the1280 allowance.
Same screens32/64/128/768, full256/1280, retention/plateau/unsafe-failure guards.
No extension or LR/seed/tokenizer/architecture search. No C++ calls are needed.

At a complete final endpoint run the existing original48 and exchanged48
diagnostics once each, at most96 generations/96 required teachers within these
limits. Reading-change success requires47/48 full and23/24 both on each panel,
errors0; it remains separate from all unchanged joint development/S4/S5/S6 gates.
If quality guards stop early, report the incomplete evidence without reopening.

## Closed: fixed-weight value exchange on the fitted train scenes

Executed48 normal generations/48 own-model teachers, optimizer0. Original
fitted answers48/48 become10/48 after exchanging only the two evidence values,
both0/24, EOS48/errors0.24/48 repeat the old exact answer;38/48 retain the correct
citation. First teacher token agrees with generation48/48, gold16/48. These
observations expose failure to follow changed values even in familiar scenes.
They do not establish a defect in a particular tensor equation or decoder.
Next investigate one training variable: alternate the already-owned value-swap
view with the original view, keeping the same fitted bases/conditions and all
other training settings. No old budget or endpoint is resumed.

Before registering another optimizer intervention, distinguish learned answer
associations from reading changed evidence. On the closed FIT7424 checkpoint,
select the same first8 fitted pairs per C/D/E and use their already-owned view1
instead of view0. Both supplied values exchange; question, entities, contexts,
IDs, record order/status/times and phrase stay identical. Expected values change
with the records; citations stay the same. Reject a no-op or any other semantic
input change before calling a model. This is a diagnostic on train scenes, not
new heldout data or a candidate selection score.

Reuse `paired_seen_train_diagnostic` and the actual existing evaluation/RunControl
path, with an explicit value-swap mode and a new output root. No new corpus,
model/core/storage policy, framework, external model or product oracle.
Max48 SMALL normal generations and48 own-model teachers, optimizer0/TINY0,
one900s command/120s cleanup, no automatic retry. Preserve the closed FIT and
LibTorch budgets. Compare with the already verified normal48/48 raw without
regenerating it. Pure validation tests exercise both sides/phrase styles and
reject wrong labels, reordered records, changed times/questions and missing
records. Result determines the next single-variable hypothesis; no new learning
is registered by this observation. Development/independent gates stay unchanged.

## Current evidence: fitting is possible, joint transfer remains unresolved

The registered FIT experiment completed1280 updates at7424 and is closed.
The same48 seen answers are all correct, both24/24, normal EOS/errors0 after80
exposures per side. Independent raw recount verifies the exact cases and receipts.
Development is primary398/512, transfer76/128, flipped2/192, both0/192; no joint
pass or candidate adoption. Final NO_FURTHER_PROGRESS, Finished/resume=false.
Do not extend this recurrence run or repeat the closed C++ comparison.

The actual native6400 weights and both Adam moments are bit-identical to the
Rust diagnostic endpoint used in the LibTorch comparison. Thus its matching112
outputs are connected to the production trainer. These observations support
investigating learned-case to new-scene selection transfer and existing-capability
retention; they do not uniquely identify data, objective or capacity as the cause.
Increasing exposure on this narrow set succeeded at fitting without satisfying
the development task. A future hypothesis must distinguish that transfer gap
with one registered variable, preserve these results and the joint gate, and
not assume a new framework, LR/tokenizer change or reinitialization is a remedy.
No further experiment is registered by this closure. Goal1 remains unfinished.

## Closed: native execution of the registered FIT learnability experiment

The isolated Rust/LibTorch comparison is closed. Its same112 outputs and close
numeric trajectories narrow the evidence against a backend-specific explanation;
they do not prove the shared model/data/objective correct. Each256 endpoint still
missed10/48 seen answers, six solely on citation, with fixed dev45/64 versus
parent48/64. Use the already prepared FIT policy below in the native production
trainer, beginning from original P6144, not the closed comparison outputs.
The user's continuing one-variable improvement authorization applies to this
separate registered experiment. No old receipt, terminal or budget is reopened.

The1280 ceiling and all retention/numeric/storage/cancel guards below remain.
This is the planned recurrence intervention versus preserved SIDE/REPLAY, not
a new period/LR/seed search. The comparison's256 calls are historical diagnostic
usage and are reported separately. No C++ calls or new corpus are needed here.

## Closed: removable independent LibTorch comparison

Both independent backends completed256 diagnostic updates from P6144 and112
normal generations each. Numerical one-step parity passed; all112 generated
token sequences match. Both score train38/48, both15/24 and fixed dev45/64.
The same parent dev64 raw recount is48/64. Train errors include six correct
values with wrong citations; development C/D/E contains seven unsupported
abstentions despite unique matching evidence. These are observed output
failures, not a proven root cause in any shared equation or data distribution.
SMALL514/TINY2 optimizer,224 generation,0 teacher; budget closed without
retry/extension or product adoption. Comparison execution PASS, quality
recovery NOT_ESTABLISHED, final200 unopened and Goal1 not ready. The deferred
FIT plan below had no SMALL execution at comparison closure. The active section
above registers its separate native execution; neither comparison arm resumes.

The user explicitly authorized an isolated C++ LibTorch reference while keeping
the product Rust-only, with deletion of the comparison directory having no effect
on the product. All comparison code/library/builds/exports stay under ignored
`artifacts/libtorch-parity-20260920/`; no Cargo or runtime linkage is added.
No external learned model/tokenizer/teacher is used. Existing artifacts remain
read-only. This narrow C++ diagnostic authorization does not change product scope.

First compare one actual TINY and one P6144 SMALL optimizer step in each backend,
using identical native weights/Adam and exact framed input/target/masks. Independently
implemented ATen forward/autograd/Adam is checked against the actual Rust functions.
Fixed element tolerances precede observations. Numerical PASS grants only the next
diagnostic: at most256 updates per backend from original P6144, same first16 C/D/E
recurrence and step-specific other-five-task rows, SIDE family4/CE/LR3e-5 unchanged.
Export all256 batches; never repeat the other five tasks' first16 batches.

Total SMALL cap514 (two numerical steps plus256 per backend), TINY2, scalar0.
Then at most112 generations per backend: same seen48 and frozen primary screen64.
Strict normal greedy/EOS/UTF-8/full-answer scoring; no new heldout or final200.
No own-model teacher calls, automatic retries, sweep or training extension.
900s command/120s cleanup, one heavy process; cancel/error/UNKNOWN blocks dependent
work. Numerical disagreement is investigated before learning. Preserve incomplete
outputs and actual attempted/committed/discarded counts. This comparison cannot
promote a product candidate or declare S4/Goal1. The separate FIT1280 implementation
below was deferred during this diagnostic and has its own registration and budget.

## Registered FIT policy: fixed seen-pair learnability diagnostic

Under the user's continuing one-variable authorization, register one FIT arm
from P6144. Completed REPLAY/WIDE observations scored18/48 and19/48 on the
same seen train sides, with both3/24 and1/24. Check fitting before further
coverage changes; this does not prove a broken gradient or capacity limit.

Keep SIDE family4 auxiliary0.1 plus first-target1 response CE, LR3e-5, Adam,
tokenizer, model, native pools, batch8 and exact other-five-task update/slot rows.
Only repeat the first16 adjacent C/D/E updates: eight pairs per bucket, the same
24 pairs/48 sides already diagnosed. At1280 each side has80 exposures. This is
a train-only fitting test with narrower base/view/phrase coverage; full-block
phrase/first-side balance is not claimed for this prefix. Both sides and total
eight-task counts remain balanced. No new corpus or label enters inference.

Maximum1280 SMALL updates,12M actual input/1M target including discarded work,
12000 generations/10000 own-model teachers,21600 active seconds,900s command/
120s cleanup. TINY allowance96/768/768 optimizer/generation/teacher, scalar0.
Reuse full3840-row writer-reader and TINY continuous4 versus fresh1+3; verify
recurrence, exact anchors, objective binding and wrong-policy rejection.
Production P16 parity precedes source freeze and first-update save/fresh restore.
Existing screens32/64/128/768 and full256/1280, retention/plateau/unsafe-failure
guards and joint quality thresholds remain unchanged. No automatic extension.

After the final saved endpoint, run the existing same48 normal generations and
48 own-model teachers once, within the above call budget. Train fitting success
requires at least47/48 exact and23/24 both with errors0; this cannot grant any
heldout/H3/S4/Goal1 promotion. If a safety/retention guard stops learning early,
report actual exposure and incomplete fitting evidence without resetting it.
Only the unchanged joint gate permits independent final200/S4 and S5/S6.

## Closed: normal generation on the already exposed pairs, no new learning

Completed96 generations/96 teachers, optimizer0. On identical24 train pairs,
REPLAY18/48 full, both3/24; WIDE19/48, both1/24. Normal generation and teacher
first argmax agree for all96 sides. REPLAY14 first-token errors plus16 errors
after a correct first token; WIDE15 plus14. Both have normal EOS/errors0.
This demonstrates incomplete learning of these seen cases, not solely heldout
transfer failure. Three/five exposures do not establish a numerical defect or
an architectural capacity limit. A next diagnostic should test learnability of
these exact paired conditions with substantially repeated exposure, keeping
the model/objective/Adam/LR fixed; no additional coverage/period sweep.

The closed REPLAY/WIDE trials retain weak heldout both6/5 respectively. Before
another training intervention, distinguish inability on seen examples from
transfer failure using the existing `paired_seen_train_diagnostic` normal mode.
For each closed7424 endpoint, select the same first8 seen original/flip pairs
per C/D/E from the first frozen block (24 pairs/48 sides). These are the same
cases used in earlier margin observations: REPLAY5 exposures, WIDE3. No new
examples, alternate seeds, candidate adoption or replay of earlier failed runs.

Two new diagnostic roots, maximum96 normal greedy generations and96 required
own-model teachers total, optimizer0/TINY0, one900s command/120s cleanup each,
1800 active seconds total, no retries. Production features and unchanged
tokenizer/native weights/policy. Preserve full raw/call/finish receipts and
check identical case identities/expected content across the two observations.
Recount whole-answer both, same-output, EOS/errors and first-token teacher
agreement separately. This train-only diagnostic cannot pass development/S4.
Do not choose another learning variable until the actual results are available.

## Closed: widen the recurring pair pool, one sampling variable

Completed1280 updates at7424: primary375/512, transfer69/128, flipped32/192,
both5/192 (D5 across3 bases), errors0. REPLAY was369/64/25/6: primary/transfer
increased but both did not. Original78/192, same output141/192. No joint pass,
no adoption; Finished/BUDGET_REACHED, resume=false. Pure report verified all
raw/native/usage bindings. Do not extend the period sweep or reopen this budget.

Register one WIDE arm from the unchanged P6144 parent under the user's continuing
one-variable authorization. Preserve the closed REPLAY trial and compare its
matched6400/7424 endpoints read-only. Its train24 paired positive margins rose
4→9 and heldout both2→6, while primary/transfer declined. This motivates a
recurrence/coverage test; it does not identify a unique cause or prove a remedy.

Only the recurring C/D/E prefix grows from256 to512 updates. Retain the SIDE
family4 auxiliary0.1, response CE first-target1, batch8, constant LR3e-5, Adam,
tokenizer, native pools, model and exact nonselector slots. At1280, each C/D/E
has512 exact cases from256 bases: first256 cases appear3 times and the next256
twice. REPLAY had256 cases/128 bases five times. Changed token totals are part
of this sampling tradeoff; no new corpus or unseen question text is introduced.
The first512 rows equal the nonrecurring SIDE tape. No period or seed sweep.

Maximum1280 new SMALL updates,12M actual input/1M target,12000 generations/
10000 own-model teachers,21600 active seconds,900s command/120s cleanup.
TINY allowance96 optimizer/768 generation/768 teacher; scalar optimizer0.
Verify the full3840-row writer-reader, partial final cycle, exact anchors,
recurrence counts and wrong-policy rejection. Actual TINY continuous8 versus
fresh1+7 crosses its4-update recurrence; production P16 parity precedes freeze.
Save/restore the first real update within the same1280 budget. Screens32/64/128/
768 and full train64/primary512/transfer128/selector192 at256/1280. Existing
retention/plateau/cancel/numeric/storage/UNKNOWN guards and joint gates remain.
Stop at first joint pass for independent final200/S4 and conditional S5/S6,
otherwise close at the registered stop/cap. Never reopen an old failed budget.

## Closed: fixed train margins after recurrence, no learning

Completed96 teachers, optimizer0/generation0; pure raw/call/receipt recount and
case/parent equality passed. REPLAY both-positive train margins9/24 versus SIDE4;
both>=1 is5 versus1. On46 token0-divergence sides, gold vocabulary argmax32 versus26,
both-first-gold pairs9 versus4; two nonzero-divergence sides are not inferred.
This supports a recurrence/coverage tradeoff test; it is not whole-answer train
success or an explanation of every heldout failure. Closed run budgets stay closed.

Use the existing teacher-only seen-pair diagnostic once for P6144 and REPLAY7424,
same first8 actually seen pairs per C/D/E as SIDE. New root, at most96 own-model
teacher calls, optimizer0/generation0,900s command/120s cleanup, no retries.
Pure recount and case/parent equality check precede interpretation. Exact tape
recount confirms these48 sides each had5 exposures rather than SIDE's1. Compare
training margins separately from heldout generation; do not declare a generalization
solution or cause from teacher accuracy. No new training before this observation.

## Closed: exact-pair recurrence at unchanged SIDE objective

Completed1280 updates at7424: primary369/512, transfer64/128, flipped25/192,
both6/192 (C1/D5/E0,5 bases), errors0. Matched SIDE372/70/25/2; original70
versus72, same output126 versus168. Train64 improved43→48, but development/
transfer fell. No joint pass or adoption. Finished/BUDGET_REACHED, resume=false.
Pure raw/native/usage report passed. Preserve the endpoint and its failure.

Register one REPLAY arm from P6144 under the continuing user authorization.
Retained SIDE6400/7424 is the matched control. Pure tape recount found each of
the48 probed train sides was seen exactly once in1280 updates, despite weak
conditional teacher discrimination. Test recurring exact cases before further
loss changes. This is a sampling hypothesis, not a proven cause of failure.

Only C/D/E selection in the finite co-batch tape changes: repeat their first
256-update block throughout the trial. Other five tasks retain their exact
update/slot sample IDs; same eight-task counts, batch8 and pair co-location.
The first256 updates are identical to SIDE. At1280 each C/D/E bucket has256
exact samples seen5 times, rather than1280 samples seen once. This reduces
distinct base/view coverage (128 bases/view0 instead of256 bases/multiple views)
and changes actual token/padding totals. It is a recurrence/diversity tradeoff,
not isolated causal proof about repetition. Phrase forms and both selector sides
remain balanced. No new corpus/labels, answer-conditioned sampler or oracle.

Same parent weights/Adam/tokenizer/model, LR3e-5, family4 sidewise auxiliary0.1
and response CE. Reuse the existing native descriptor/tape/policy/RunControl;
no storage/core changes. Maximum1280 new SMALL updates,12M input/1M target
including discarded work,12000 generation/10000 own-model teacher,21600 active
seconds,900s command/120s cleanup. TINY64 updates/768 generation/768 teacher.
Test3840-row writer-reader and exact recurrence/anchor preservation, then actual
TINY continuous4 versus fresh1+3 so recurrence crosses its2-update fixture block.
P16 production parity before source freeze; save/restore the first real update.
Screens32/64/128/768 and full train64/primary512/transfer128/selector192 at256/1280.
Existing retention/plateau/unsafe-failure guards and joint quality thresholds
remain intact. Stop at first joint pass for independent final200/S4 and conditional
S5/S6, otherwise close at the registered stop/cap. No old budget or failed run resumes.

## Closed: fixed train-pair margins after sidewise learning, no updates

Completed96 teachers, generation0/optimizer0, errors0; pure recount passed.
SIDE and CONTRAST both have4/24 train pairs with both margins positive. SIDE
has1/24 both>=1 and0 aggregate-pass/wrong-side pairs. This changes margin
calibration without solving conditional choice. On46 first-token divergence
sides, positive-margin but wrong vocabulary argmax occurs1 time in SIDE and2
in CONTRAST; third-token preference is not the dominant failure in this sample.
Pure tape recount found all48 fixed train sides had exactly one exposure during
1280 updates. Each C/D/E bucket exposed1280 distinct exact samples once, with
2816 other exact samples unused. This is exact-sample exposure, not unique base
count: all256 bases per bucket were represented. Consider recurrence separately
from marginal loss weighting; no causal claim or tiny-set quality substitution.

Use the existing optional teacher-only seen-pair diagnostic once in a new root
for SIDE7424 and P6144: first8 actually seen train pairs per C/D/E, same selection
as the earlier CONTRAST diagnosis. At most96 own-model teacher calls, generation0,
optimizer0, one900s command/120s cleanup, no retry. Independently recount the raw
and compare with retained CONTRAST margins on identical cases. Distinguish
counterpart margin from vocabulary argmax and from normal generation accuracy;
do not infer a unique cause from any one teacher statistic. No new learning yet.

## Closed: separate per-side margins at unchanged co-batch exposure

Completed1280 updates at7424: primary372/512, transfer70/128, flipped25/192,
both2/192 (D2 across2 bases), errors0. Original72 and same output168/192.
Matched CONTRAST was366/68/39/0, original64 and same145. The modest primary/
transfer gains and two both-correct pairs do not meet the unchanged joint gate;
SIDE is not adopted. BUDGET_REACHED, resume=false. Raw/native/teacher/usage
recount passed. Inspect the fixed train margins before choosing another variable.

Register one SIDE trial from P6144 under the user's continuing one-variable
authorization. Reuse retained CONTRAST6400/7424 as the equal-budget control;
never resume that closed trial. The96-teacher fixed train diagnosis found4/24
pairs with sum margin>=1 but one side<=0. This motivates testing aggregation,
not a claim that it uniquely explains heldout failure.

Keep the same parent weights/Adam, tokenizer, all data and exact3840-row co-batch
tape, batch8, LR3e-5, clipping/decay, first-target1 CE and decoding. Let da/db
be each side's correct-minus-counterpart logit at the first divergent target
under the common gold prefix. Replace only the auxiliary with
`0.1 * mean_pair (softplus(1-da)+softplus(1-db))/2`, added to unchanged response
CE. Margin1 now applies separately to each side, not their sum. This changes
gradient magnitude and no longer cancels common token bias. Coefficient remains
0.1; no tuning sweep, extra forwards or product annotations. Anchor-only batches
add zero. Native objective family4/normalizer4 binds this equation, fixed
coefficient and training pair-table digest in the existing descriptor layout.
Family3 and all original checkpoints retain their original meaning.

Maximum1280 new SMALL updates,12M input/1M target including discarded work,
12000 generation/10000 own-model teacher,21600 active seconds;900s command/
120s cleanup. TINY64 updates/768 generation/768 teacher, scalar optimizer0.
Verify independent scalar loss/gradients, balanced versus wrong-side equal-sum
case, finite extremes/malformed pairs, full finite tape, native objective binding
and continuous2 versus fresh1+1. Production P16 parity precedes source freeze.
Save the first real update and restore in a new process within this budget.
Screens32/64/128/768; full train64/primary512/transfer128/selector192 at256/1280.
Existing retention/plateau/cancel/numeric/storage/UNKNOWN guards and all joint
quality gates remain unchanged. First joint pass freezes the candidate for
independent S4 and conditional S5/S6; otherwise close at the registered stop/cap.
No retroactive winner selection or extension of this trial.

## Closed: fixed train-pair margin diagnosis, no learning

Completed96 own-model teachers, optimizer0/generation0. Independent raw recount
passed. At CONTRAST7424,8/24 seen pairs meet sum>=1;4 of those have one side<=0.
Only4/24 have both sides positive and0/24 both>=1. Parent P6144 has0/24 in
all three positive criteria. This demonstrates a limitation of sum aggregation
on these fixed train pairs, not the sole cause of heldout failure. A separate
bounded experiment may test per-side aggregation while retaining other settings.

Before selecting the next single learning variable, use the existing seen-pair
diagnostic's optional teacher-only mode. Select the first8 actually seen pairs
per C/D/E from the first frozen tape block, before model calls. Compare P6144 and
the closed CONTRAST7424 on those same24 pairs/48 sides using existing native
loading, foil teacher and durable call helpers. New optimizer0, generation0,
own-model teacher at most96, one900s command/120s cleanup, no automatic retry.
Inputs/original checkpoints/receipts stay read-only; all outputs use a new root.
For each side, record correct-versus-counterpart logit margin at their first
divergent token under the shared gold prefix. Recount both-positive, both>=1,
sum>=1 and sum>=1 with a nonpositive side. These are train-only teacher metrics,
not generation accuracy or heldout quality. Compare the actual measurements
before changing the equation; no coefficient/margin sweep is authorized here.
The teacher's old `objective` field is per-example response CE, not family3's
batch auxiliary; new records state this scope explicitly. Historical raw stays intact.

## Closed: paired discrimination objective at fixed co-batch exposure

Completed1280 updates at7424: primary366/512, transfer68/128, flipped39/192,
both0/192, errors0. Original64/192 and same output145/192. The matched pure-CE
co-batch control was371/60/21/0, original77 and same output179. The intermediate
both3/192 at256 did not persist; do not select it retrospectively. Stop
BUDGET_REACHED, resume=false. This objective is not adopted as a quality fix.
Before another intervention, inspect fixed seen train-pair margins without updates;
the sum margin can mathematically be positive while one side is still incorrect.
This is a limitation to investigate, not an established cause of these scores.

The user's continuing one-variable authorization permits one new CONTRAST trial
from P6144, retaining the exact COBATCH sample tape, batch8, Adam, LR3e-5,
tokenizer, native data, clipping/decay and model. The closed COBATCH control at
6400/7424 is reused read-only. No new examples, framing, decoding or product oracle.
COBATCH ended both0/192 and identical normal outputs179/192. Existing target pairs
differ first at token0 in172 cases and token1 in20; these observations motivate
explicit conditional discrimination, not a proven cause or an assumed solution.

Only the training objective changes. For each co-batched C/D/E original/flip pair,
find the first distinct target token after their shared gold prefix. With correct
tokens ya/yb and their respective native logits za/zb at that position, define
`d = (za[ya]-za[yb]) + (zb[yb]-zb[ya])`. Optimize existing response CE plus
`0.1 * mean_pair log(1+exp(1-d))`; use stable log-sum-exp, fixed margin1 and
coefficient0.1. Anchor-only batches add zero. The CE denominator stays the actual
response tokens, and CE is logged separately from the objective. The auxiliary
has its own pair mean and changes gradient magnitude; it is not a mere rescaling
of first-target CE. A common token preference at both inputs cancels in d.
No extra model forward/teacher call and no gradient from evaluation data.

Existing native resume descriptor family3/normalizer3 identifies this exact
equation, stores coefficient0.1 in its existing optional coefficient slot, and
binds the training pair table, tape and policy. No tensor/file layout changes.
Generic/default-loss resume rejects it; native inference receives no annotation.
The caller verifies complete C/D/E pairs, distinct targets, shared prefix,
token/position bounds and EOS. Product logits/greedy/strict UTF-8 stay unchanged.

Maximum1280 new SMALL updates,12M input/1M target including discarded work,
12000 generation/10000 own-model teacher,21600 active seconds,900s command/
120s cleanup. TINY64 updates/768 generation/768 teacher, scalar optimizer0.
Verify independent scalar loss/gradients, common-bias invariance, extreme finite
logits, malformed pairs,3840-row tape, native binding and actual continuous2
versus fresh1+1; then production P16 parity before source freeze and learning.
Save/resume the first actual update. Screens32/64/128/768 and full train64,
primary512,transfer128,selector192 at256/1280. Original retention/plateau,
cancel/numeric/storage/UNKNOWN guards and final joint thresholds remain unchanged.
Stop on first joint pass for independent S4 and conditional S5/S6; otherwise close
at the registered stop/cap. No coefficient/margin search or combined intervention.

## Closed: joint pair packing at unchanged batch8

Completed1280 new updates at7424: primary371/512, transfer60/128, flipped21/192,
both0/192, generation errors0. Matched ADJACENT control was366/73/33/2.
Stop NO_FURTHER_PROGRESS with the optimizer cap exhausted, resume=false.
Batch grouping alone did not recover joint quality and is not adopted. Preserve
all native/raw evidence. Further single-variable research needs a separate
parent-bound registration under the user's continuing authorization.

The user authorized continuing evidence-based one-variable research. Neither
first-target4 nor LR9e-5 recovered joint selection; keep their failures closed.
Register `fresh paired-prepare --co-batch` as one new COBATCH arm from P6144,
with the retained coefficient1/LR3e-5 ADJACENT control at matched6400/7424.
Hypothesis: putting both query sides into one gradient may reduce preference
oscillation across updates. This is not a proven optimizer defect. The LR trial
still produced the same normal output on172/192 opposite-query pairs and both0.

Only batch grouping changes. For each pair of adjacent eight-case tape rows,
put the six original/flip C/D/E cases into one eight-case batch. Two remaining
slots take the first side of two other tasks, rotating through A/B/F/G/H by pair
ordinal. The other three first-side cases and all five second-side cases form
the next eight-case batch. Every two updates retain the exact16-sample multiset;
every256 updates retain256 draws per task, phrase and side counts and the same
request/target bytes. Per-update task composition and padding do change; the
five other tasks are no longer updatewise identical. Report this explicitly.
Each sample still has independent attention; the opposite answer is never in
its prompt. No paired oracle is exposed to generation. This is not batch16 or
gradient accumulation. Native shapes/formats and the CE equation are unchanged;
batch grouping naturally changes each batch's target-token normalizer/gradient.

Same parent weights/Adam/tokenizer, seed, model, microbatch8/accumulation1,
constant LR3e-5, first-target1, clipping/decay, corpus and whole-answer evaluation.
No coefficient4/LR9e-5 combination, reset, new wording/data or decoding change.
Existing native finite tape and sample table, RunControl and scorer are reused.
Maximum1280 SMALL updates,12M input/1M target including discarded work,
12000 generation/10000 own-model teacher,21600 active seconds,900s commands/
120s cleanup. TINY64 updates/512 generation/512 teacher; scalar optimizer0.
First actual update saved and resumed. Screens +32/64/128/768; full train64,
primary512,transfer128,selector192 at+256/+1280. Original retention/plateau,
cancel/numeric/storage/UNKNOWN guards and final joint thresholds remain intact.
Verify exact two-update multiset, co-location, true task metadata,3840-row native
readback and TINY continuous2 versus fresh1+1 before SMALL; verify P16 parity.
Stop at the first joint pass for independent S4 and conditional S5/S6, otherwise
close at the registered stop or cap without extending this run.

## Closed: bounded learning-rate intervention from P6144

Completed1280 updates at7424: primary330/512, transfer60/128, flipped35/192,
both0/192, generation errors0. The matched LR3e-5 control was366/73/33/2.
Stop NO_FURTHER_PROGRESS and exhausted optimizer budget; resume=false. LR9e-5
is not adopted. The native endpoint, original raw and failed decisions are preserved.

The user's continuing one-variable authorization permits this new experiment;
the old paired/follow-through/first-target4 failures remain closed and read-only.
Register `fresh paired-prepare --learning-rate-threefold` in a new root. Change
only constant LR3e-5→9e-5 relative to the retained coefficient1 control, using the
same P6144 weights/Adam, tokenizer, native corpus, ADJACENT tape, batch8, eight
tasks, clipping/decay and first-target1 response CE. Do not combine weight4, reset
Adam, reinitialize, search other rates, change wording/data or alter generation.

Hypothesis: adaptation may be limited by the continuation update size. Retained
selector teachers at9984 still miss the first target on99/192 cases at coefficient1;
weight4 reduces that to78 yet reduces primary368→339. Increasing its loss weight
did not recover joint quality. This motivates a separate LR test, not proof of
a low-LR defect. Compare only matched cursors6400 and7424 with retained control
sources `2dfa56f` and `cd52b21`; an earlier or unequal-budget stop is reported as
such. The control is existing evidence, not a freshly executed second arm.

New cap1280 SMALL updates (6144→7424),12M actual input/1M target including
discarded work,12000 generation/10000 own-model teacher,21600 active seconds,
900s commands/120s cleanup. TINY cap64 updates/512 generation/512 teacher,
scalar optimizer0. Validate native LR/objective and exact continuous2 versus
fresh-process1+1, full finite tape and schedule, and production P16 parity before
SMALL. Save the first actual update and resume in a fresh process within the cap.
Screens at additional32/64/128/768; full train64/primary512/transfer128/selector192
at256/1280. Same parent retention, numerical/storage/cancel/UNKNOWN guards and
two-full-panel stagnation rule. A joint pass freezes the first candidate and
activates independent S4 and conditional S5/S6. Otherwise close without extending
this budget. There is no third LR value or automatic longer continuation in this
registration; any later hypothesis needs its own evidence and bounded registration.
All original joint, final200 and Goal1 thresholds remain unchanged.

## Closed: first-target coefficient, one-variable research

Completed3840 updates at9984: primary339/512, transfer67/128, flipped67/192,
both7/192, errors0. The coefficient1 retained control was368/67/56/4 at the
same step. Joint quality failed and coefficient4 is not adopted. All artifacts
and resume=false are preserved. Subsequent one-variable research uses a new
preregistered root under the user's continuing authorization; no old budget opens.

The user's2026-09-20 authorization permits grounded, bounded experiments changing
one variable at a time after the fixed-condition follow-through. Register one new
P6144 fork with retained weights/Adam/tokenizer and the identical3840-row ADJACENT
tape, native corpus/phrases/selectors, eight tasks, batch8 and constant LR3e-5.
Only the existing first supervised token coefficient changes1→4. Do not restart
the closed runs, reset Adam, collect data, alter generation or change storage/core.
The objective is `(sum(response NLL) + 3*sum(first-target NLL))/actual target count`;
prompt/PAD stay masked, EOS stays supervised, reported CE stays unweighted.
This also changes the objective's gradient magnitude; it is not a pure LR change.

Evidence: seen train-pair underfitting and high first-target NLL in retained
own-model teacher rows, documented in EXPERIMENT_STATUS. This does not prove a
loss defect. Compare with the preserved coefficient1 ADJACENT lineage at matched
absolute6400/7424/8448/9984, not with a different parent or unequal-budget endpoint.
The control spans the original256 and follow-through3584; its files stay read-only.
Reuse the existing native objective binding, Plan/Fork/finite tape and evaluator.
P16 production parity, scalar weighted gradients, max-tape readback and actual
TINY continuous2 versus fresh-process1+1 must pass before new SMALL learning.

New caps: SMALL3840 updates,12M input/1M target including discarded work,
12000 generation/10000 own-model teacher,21600 active seconds;900s command plus
120s cleanup. TINY regression cap64 updates/512 generation/512 teacher, scalar
optimizer0. These are new research budgets, not amendments to old closed budgets.
The first real update is saved and resumed in a fresh process inside this cap.
Screens at absolute6176/6208/6272/6912/7936/8960/9472; full train64/primary512/
transfer128/selector192 at6400/7424/8448/9984. Joint quality, original P retention
guards, two-full-panel stagnation, cancel/numeric/storage/UNKNOWN interlocks and
strict normal greedy/EOS/UTF-8 remain unchanged. No retries after unsafe failure.
Stop on the first joint pass and proceed to the authorized independent S4/S5/S6.
A loss or first-token improvement alone never qualifies a model.

## Closed: explicitly requested bounded exposure follow-through

The follow-through completed3584 updates at absolute9984, with primary368/512,
transfer67/128, flipped56/192 and both4/192 (C1/D3/E0), generation errors0.
The final native weights/Adam and all raw are preserved, resume=false. This is
not joint quality recovery. The user subsequently explicitly authorized further
grounded, one-variable experiments while retaining the quality thresholds,
Rust-only implementation and external-model prohibition. A subsequent experiment
must register its own bounded policy and exact parent before any learning; no old
budget, failed gate or terminal is reopened.

On2026-09-20 the user requested continued improvement toward Goal1 after the
paired study failed its entry gate. This authorizes a new bounded research fork;
it does not change the old gate result, terminal records or model acceptance.
Start from the completed ADJACENT6400 native weights/Adam and consume its exact
remaining finite tape, cursor256 through3839, at most3584 new updates. The old
SPACED256+ADJACENT256 and this continuation together remain within4096 updates.
No new data, reinitialization, ratio, LR, loss, tokenizer, model or storage change.
This is an exposure-length hypothesis, not evidence of a superior policy.

A fixed train-only diagnostic on the already consumed first8 pairs per C/D/E
returned12/48 full, both0/24, same output23/24, EOS48/errors0, CE0.1501907592.
It used48 production generations/48 own-model teachers and zero optimizer calls.
It demonstrates failure on those seen pairs, not a proven optimizer/core defect.

Reuse the existing continuation evaluation schedule: screen64+flip24 at additional
512/1536/2560/3072; full train64/primary512/transfer128/selector192 at1024/2048/3584.
Keep the original P screen retention guards and every numeric/cancel/storage/UNKNOWN
interlock. Two complete panels without improving any best primary/transfer/both
score stop the research. First joint development pass stops learning and activates
the previously specified independent S4 and subsequent S5/S6; no self-acceptance.
The joint/final/product/quant thresholds below are unchanged.

The new policy has its own source/executable/parent/authorization binding and
create-new root. It never edits the old resume=false or ranks this unequal-length
endpoint as the prior comparison winner. Existing closed artifacts remain read-only.
Fresh SMALL limits are3584 optimizer,12M input/1M target,12k generation/10k teacher,
21600 active seconds,900-second commands/120-second cleanup. Actual totals across
both research roots and the train-only diagnostic must also be reported.

## Closed: paired exposure from the preserved P6144 parent

G2 closed on2026-09-20 at256 updates per arm,512 total, with no admissible
learner. SPACED primary375/512, transfer65/128, flip5/192, both0/192;
ADJACENT primary367/512, transfer70/128, flip15/192, both0/192. All generated
rows have normal EOS and no error. Both6400 native checkpoints and Adam remain
preserved with resume=false. Equal actual information/target exposure and raw
scores passed the read-only recount. This is execution completion, not quality
recovery. G3's3584 conditional updates are NOT_AUTHORIZED_BY_GATE; G4–G7/S4/S5/S6
and Goal1 remain unexecuted/unaccepted. Do not reopen these closed arms.

R3-PAIRED-LEARNING-TO-GOAL1-1.0 reuses independently accepted R2/R3 evidence.
Compare only SPACED128 versus ADJACENT1, each256 updates from the same P6144
weights/Adam/tokenizer and eight-task train pool. LR3e-5, first-target1 CE,
batch8 and F32/CPU/Accelerate/thread1 stay fixed. Each256 block contains128
distinct training bases per C/D/E bucket, both sides once,64 original-first/64
flipped-first and64 familiar/64 P phrase forms. Other five tasks are identical
update by update; full-block multisets match. The native policy binds the finite
3840-step tape and request/target index table; absolute Adam clock is preserved.

G0 parent/provenance/current-source P16; G1 tape and TINY continuous2 versus1+1
per policy, plus the retained final-step evaluation-only regression; G2 actual
sequential training. Screen64+flip24 at +32/64/128; train64+primary512+transfer128
+selector192 at +256. Pair originals reuse the same endpoint's primary raw.
Strict generation failures remain in denominators; required teachers are retained.

Safe quality stop: screen errors>=8 or parent screen loss>=16; loss>=8 at two
consecutive checkpoints. The peer may continue its original budget. Cancellation,
unknown usage, storage/numeric/integrity failure blocks both. Pure time stops use
accepted cursor semantics. Read-only report recounts raw before ranking.

G3 entry requires completed256, primary>=417, transfer>=75, all full panels
error-free, both>=12/192, each C/D/E both>=1 and at least4 independent bases.
Rank both, primary, transfer, then arm name (ADJACENT before SPACED). If neither
qualifies, close without extension. Otherwise register continuation from6400 for
at most3584 further updates using the frozen pool/tape. Screen checkpoints are
+512/1536/2560/3072; full checkpoints +1024/2048/3584. Stop on joint pass,
retention guard, or two full checkpoints with no improvement in any best
primary/transfer/both score.

Total caps: SMALL4096 optimizer calls, actual input12M/target1M including discarded
work, generation12000/teacher10000, active21600s, command900s plus cleanup120s.
TINY counters are separate: updates128, generation/teacher768 each. Joint gate:
primary487/512, each task58/64, transfer116/128, both173/192, each C/D/E56/64,
generation errors0. Fix the first passing candidate and check fresh32 parity.
Independent final200/S4, actual product memory/correction/restart/S5 and grouped
INT4/M4/S6 are conditional authorized successors. Goal1 independent acceptance
cannot be self-granted. G0/G1 verification completed before G2 on2026-09-20.
At G0/G1, new SMALL updates were0; production P16 matched16/16 with160 raw tokens
and no teacher. G2 usage and closure are recorded above.
Direct TINY/process, finite tape, accumulation/mask/causal and evaluation-only
resume checks passed. Failed test attempts remain in the local evidence ledger.

## Closed implementation: equal-step posthoc selector observation; R3 accepted

R3-ACCEPTANCE-AND-QUALITY-CLOSURE-1.0. R0–R2 independently accepted source
`abd967980645a878f22069708daa9fbfce70bf87`; report commit
`898f57d7dd8109386f23549d391618c558097774` records15 tests and P16 parity.
Implementer Q0 verified that report's test log and executable hashes without
rerunning its model calls. The accepted generation/timeout code stays frozen.

Q0 is complete: both native step7168 artifacts, their primary512 raw/teachers,
and the existing selector192 mapping pass the existing native/policy/scorer checks.
An explicit ignored test in the existing training test module connects the generic
loader, generation observer, call receipts and confirmed publisher without teacher
or optimizer calls. Registration, observation and pure reporting are separate modes.
No product CLI or model semantics change. Its test-only diff requires reviewer R3.

Q1: metadata-fixed C6/D5/E5 parity per model, then192 existing selector cases per
model, once. Cap416 SMALL generations, teacher/backward/optimizer0. Persist returned
failures and resume only remaining observations after a confirmed pure time stop;
unknown entry, cancellation or storage failure blocks. Q2 verifies raw/decode/labels,
both-correct and paired gain/loss. R3 independently checks these new observations.
No training, candidate promotion, old run resume or final200 opening is authorized.

Q1/Q2 completed on2026-09-20 using frozen candidate
`e71345e66e18d221db2e5cb43fb67569c6eace8e`. Parity16/16 each;416 total generations,
5,998 raw tokens, EOS416/errors0/UNKNOWN0, teacher/backward/optimizer0. C7168 versus
S7168 original/flipped full155/8 versus70/28, both-correct0/0. Flipped gain/loss24/4,
both gain/loss0/0. Original and flipped native prompts differ on every pair; all
retain both evidence records. These results support a limited change in answer
preference with retention cost, not joint selection or a proven underlying cause.
The4,129 bound inputs are unchanged; raw and failed historical study remain intact.
No more implementation-side generations are authorized by this completed observation.
The separate R3 reviewer has at most32 generations and must review the test-only diff
and pure recount. One proposed future ordering/spacing comparison is documented in
EXPERIMENT_STATUS; it is not authorized or executed. H3/S4/Goal1 remain false.

## Closed: existing execution path stabilization

Closed on2026-09-19: STABILIZATION_SCOPE_VERIFIED. Source
`abd967980645a878f22069708daa9fbfce70bf87` was pushed and remote-verified.
Direct lifecycle regressions passed; frozen-source P6144 parity16 matched all raw
outputs. C/S recount confirmed the existing unequal-budget result with no candidate.
Original inventory is unchanged. New SMALL updates0/generation16/teacher0; TINY
optimizer55/generation507/teacher482 including failed test execution. Do not extend
this run or resume S. Model quality and Goal1 remain unaccepted. Details and local
evidence paths are in EXPERIMENT_STATUS.

R3-EXISTING-PATH-STABILIZATION-1.0, source reference
`b0e38e18cb66b87bdaf1f657b8e74332723cb996`. Freeze features and learning.
Fix only native command-capped timeout classification, preserving returned failure
rows, strict denominators, pending evaluation and mixed-error/UNKNOWN interlocks.
Reuse the current native evaluator, confirmed publisher, call resolutions and scorer.
The original and effective timeout and cap source are captured before entry. Only
the exact native timeout with a shorter command cap adds TIME_BUDGET alone; request
timeout remains a failure. A returned failure consumes its ordinal even with zero
tokens. Remaining teacher/evaluation can finish without another optimizer update.

S0 preservation → S1 narrow repair → S2 direct native/process lifecycle regressions
→ S3 read-only C/S recount and fixed P6144 parity16 → S4 source/report publication.
SMALL optimizer/backward/updates0, generation≤16, teacher0. TINY optimizer≤128,
generation/teacher≤512 each; count failed tests and known entries, not just receipts.
Missing post-kill work is UNKNOWN and blocks retry. Use explicitly scoped existing
tests, not the entire quick suite. No C/S continuation, corpus changes or new study.
Preserve the closed S stop, unequal-budget C/S result and unopened final200.
Stabilization acceptance cannot promote S4/S5/S6 or Goal1. Stop after the contracted
lifecycle matrix, parity and original-preserving recount are verified.

## Historical: selector consistency from the preserved P6144 parent

Closed on2026-09-19. C-KEEP completed2048 new updates at8192 (primary420/512,
transfer80/128). S-SELECT stopped after1024 at7168 (primary350/512, transfer67/128)
with QUALITY_REGRESSION_PRIMARY:75 fewer primary answers than parent425. Both
resume=false. STUDY_INCONCLUSIVE_UNEQUAL_BUDGET; no eligible candidate or budget
extension. S's remaining1024 and final selector panel are NOT_RUN_QUALITY_STOP.
Final200 remains unopened; Goal1 S4/S5/S6 are not accepted. The policy below records
the closed run, not permission to restart. EXPERIMENT_STATUS gives the actual traces,
raw recount and separate learning/report source identities. A report-only bounded
exposure digest repair was verified after learning ended; it changes no run record.

R3-SELECTOR-CONSISTENCY-1.0. Preserve P6144, its original/phrase training data,
tokenizer, Adam and all closed C/P observations. No reinitialization or storage,
SQLite, architecture, tokenizer or loss changes. The source reference is
e97e2c665c5de29a1a6a84b85864016406f92cf3; prior report is e9645747a54a45cb864394840ff8c4a21ccf0ae9.

S0 parent/input verification → S1 evaluation obligation and actual call boundaries
→ S2 direct TINY processes and parent parity16 → S3 selector diagnostic192 and
training registration → S4 C-KEEP/S-SELECT → S5 endpoint recount. Only a joint
development pass authorizes fixed-candidate parity64, final200 and existing S5/S6.

Checkpoint completion is separate from due evaluation completion. Optimizer-returned,
pre-checkpoint and post-checkpoint timeout preserve EvaluationPending. Evaluation-only
continuation changes no weights/Adam/clock/counters. Consumers require complete raw,
required teacher rows and summaries. Prepared call records and immutable resolutions
bind model/policy/panel/case/cursor/attempt; only confirmed NotInvoked plus pure
TIME_BUDGET permits another attempt at the same cursor. Returned rows remain intact,
including known errors. Missing resolution is UNKNOWN, never zero usage. Cancellation,
I/O and identity errors block. Publication uses the existing confirmed publisher;
cooperative checks cannot preempt a synchronous tensor call or infer crash usage.

Both arms retain P's original/phrase1:1 exposure and sampler epochs6/7. C keeps the
selection relation; S flips C/D/E once per case over two exposures. C changes only
query entity, D only query context, E only the two observed_at values. Request-only
labels, independent field/time checks and involution are training/diagnostic tooling,
never product inference. Flip assignment balances bucket/epoch/phrase/view; A/B/F/G/H
remain byte/token-identical. S has3072 flipped draws among16384 total.

Same P6144 weights/Adam/tokenizer, first-target1 CE, fixed LR3e-5. Each arm2048 updates
ends8192; total4096 SMALL, input9M/target1M per arm including discarded work. Shared
generation4608/teacher6000/active14400s, segment900s plus cleanup120s. TINY update128,
generation768/teacher768 and scalar32 are separate limits. First1 save/process restart
is inside the learning budget. No automatic extension, third arm or source mutation.

Reuse parent endpoint raw704 and historical C704; parity16 plus selector192 new calls.
At+256/512/1536 screen64, +1024/2048 train64+primary512+transfer128 with fixed teacher64;
screen is reused from full primary when present. Final also measures selector192 per
arm. Screen loss24/errors8 is severe; loss12 and CE×1.2 twice stops. Primary loss64
from425 or errors8 stops. Quality stop of one arm permits the unchanged other arm;
cancel/data/numeric/I/O/usage failure blocks the study. Intermediate best is not a winner.

Same final endpoint must achieve primary487/512, every bucket58/64, transfer116/128,
generation errors0. Selector pairs are diagnostic, never a replacement gate. Failure
closes with no candidate/final200. Tie-break lowest bucket, primary, transfer, then C.
Final200/S4 and S5/S6 criteria below remain unchanged; independent acceptance is external.

## Historical: exposure and question-phrasing comparison

Closed at each2048 new updates on2026-09-19. C primary425/512, transfer44/128;
P primary425/512, transfer79/128, both generation errors0. Development gate FAILED,
candidate=null, final200 NOT_OPENED; S5/S6 NOT_RUN_PREREQUISITE. Both arms end at
absolute6144 with BUDGET_REACHED/resume=false. The policy below records the completed
comparison and authorizes no further learning or replacement arm. Last durable is
each arm's `segment-0003/final` under `artifacts/fresh-exposure-phrase-20260919/`.
Source e97e2c665c5de29a1a6a84b85864016406f92cf3; EXPERIMENT_STATUS records raw recount,
actual exposure/token/call usage, parameter updates and endpoint hashes.

R3-FRESH-EXPOSURE-PHRASE-1.0, source reference d126aff35d85cebd1bf2b40bc6bc0a5943a3084b,
baseline report a8376ff03ced939710f946609170d2fb6b798452. Preserve the completed
fresh4096 run, all native source data, Adam/tokenizer and original raw observations.
No reset, deleted-parent recovery, storage redesign or SQLite migration.

G0 verify the real parent → G1 publication/generation/evaluation-resume boundaries
→ G2 read-only raw recount, same-weight16 parity and familiar-wording128 diagnostic
→ G3 immutable C/P registration → G4 two bounded arms → G5 endpoint comparison.
Only a joint development pass permits G6 independent200, S5 and S6.

New plan/segment schema2 uses existing R3BIN. A durable pending interlock precedes
finished publication; publisher file/directory syncs and readback precede release.
Consumers verify the start, finished/control hashes, actual checkpoint and counters.
Failure needs no additional error file to remain blocked. The last unlink may
reappear after a crash and conservatively block; multiple files are not one atomic
transaction. Legacy finished records are read-only parent evidence, never upgraded.

Raw row version2 records a returned Generated result independently from command_stop.
EOS plus strict text can be correct even when the command deadline is observed
after return. Length/error/empty output remains wrong. Required teacher observations
have a separate durable prefix and are resumed without repeating completed generation.
An outstanding started call without a returned row is UNKNOWN and blocks retry.
Optimizer completion and EvaluationPending are separate: last-step evaluation-only
continuation calls no trainer/Adam and preserves the native checkpoint bytes.

Both arms inherit the same4096 parent and moments/absolute clock. C repeats original
train8192 for sampler epochs4/5. P keeps cases/evidence/labels/order and uses original
and alternate training question once each, balanced within each bucket/epoch.
Only the task suffix changes; supported Full/Current/Previous/Restored/Cause intents
come from the request, never the target. Two project-owned alternate suffixes per
intent are checked against exact heldout suffixes and an independent resolver.
No new tokenizer mapping, model equation, output format or loss weighting.

Constant actual LR3e-5; ordinary response+EOS CE, first-target1; no warmup restart.
Each arm ends at absolute6144, max2048 new updates/input9M/target1M. Shared new
generation4096/teacher6000/active14400s caps include observation and all segments;
commands900s plus cleanup120s. TINY updates128/scalar32 are separate numeric limits.
First actual update of each arm is saved before fresh-process continuation.
No third arm, automatic extension or source change during learning.

Parent train64/primary512/transfer128 are verified raw reuse. At+256/+512/+1536,
screen64; at+1024/+2048, train64+primary512+transfer128. Endpoint screen is the exact
subset of full primary raw, not another generation. Fixed train64 teacher NLL is
measured at each boundary, reusing the completed train panel when available.
All teacher rows are required before a panel CE or boundary is complete.
Severe screen loss>=24 or errors>=8 stops the arm. Two consecutive losses>=12
from parent47/current best with CE>=1.2x stop it. Quality stops do not alter the
other arm; cancellation/I/O/integrity failure blocks automatic study continuation.

Only the+2048 endpoint is the primary comparison. Gates remain primary>=487/512,
each bucket>=58/64, transfer>=116/128 and generation errors0 simultaneously.
Tie-break: lowest bucket accuracy, primary, transfer, then C before P. Familiar
wording is diagnostic only. Failure closes the study without further learning.
Final200 remains unopened until a qualifying candidate is frozen; then>=190/200,
each of5 categories>=36/40, no generation errors or accepted bad citations.
Only actual S4 pass permits S5 memory/restart and S6 grouped INT4 comparison,
with extra training0/generation<=1024. Independent acceptance remains external.

## Active: fresh joint baseline after the complete data reset

Closed at4096 updates on2026-09-19: primary392/512, transfer40/128, errors0
at the final checkpoint. Development gates FAILED. Independent200 NOT_OPENED;
S5/S6 NOT_RUN. The registered policy below records this completed attempt and
does not authorize a retry, continuation or new initialization. Last durable
`artifacts/fresh-joint-20260919/segment-0005/final`, BUDGET_REACHED, resume=false.
See EXPERIMENT_STATUS for source identity, actual usage and independent raw recount.

R3-FRESH-JOINT-BASELINE-1.0, reference1885626a4f84ec79137e4e79414269a019de7e3f.
This replaces the old parent-dependent continuation instructions for new work.
No deleted model/corpus/receipt is restored. No new storage/SQLite/kernel work.
Use existing native formats and the ordinary trainer, CPU F32 Accelerate thread1.

F0 current state → F1 focused numeric/process checks → F2 native joint corpus and
independent constrained resolver → F3 train-only tokenizer/fresh plan → F4 one
bounded run → F5 fixed development candidate/independent200 → conditional F6 S5/S6.
Eight equally sampled buckets cover full copy, explicit field, entity selection,
context, current/valid time, past/correction/restoration, missing/ambiguous evidence,
and unconfirmed causality. Train8192/primary512/transfer128; base views stay together.
Full entity SHA256 partition is disjoint; primary grammar is shared deliberately.
Transfer reports unseen phrasing separately from record-count combinations.

Seeds model17/data20260919/sampler29. SMALL topology unchanged; tokenizer trained
once on all train text, maximum4096 actual vocabulary. Default response+EOS CE,
first-target1, no SPAN/curriculum. Fresh Adam .9/.999/eps1e-8, decay.01, clip1;
LR3e-4, warmup128, cosine final.1, absolute clock. Batch8, or fixed2x4 only if
the pre-run memory planning guard exceeds16GiB. One draw per bucket/update,
base permutation per epoch and rotating views, no replacement within1024 updates.
Plan hash and exact order/config/tokenizer/source are bound inside native checkpoints.
Generic resume cannot silently substitute the default sampler/objective.

Limits: SMALL4096 updates/input20M/active10800s, segment900s plus cleanup120s;
generation4096/own teacher6000, TINY128. First1 save and fresh-process resume are
inside the4096 budget. Only clean time pauses resume; missing completion is UNKNOWN
and blocks retry. No source/data/seed/LR/loss changes after execution freeze.

Screen64 at0/128/512/1024/2048/3072/4096. Primary512 at1024/2048/4096;
transfer128 at2048/4096. Fixed train64 generation at0/1024/2048/4096 and its
teacher CE initially/every512. Errors count in full denominators. Initial random
invalid output is not a numeric failure. Nonfinite/data/I/O/cancel/resource errors
stop. Two screen regressions of>=12 below a best>=32 plus CE>=1.2x stop;
1024 dev0 with no train signal and2048 at least4 empty buckets without progress
close the run. End4096 is final; no automatic extension or replacement attempt.

Same-checkpoint development gate: primary>=487/512, every bucket>=58/64,
transfer>=116/128, errors0 and no accepted invalid citations. Freeze a qualifying
2048/4096 candidate before independent200 (>=190, each of5 categories>=36/40).
No final data generation/reading by trainer before that gate. S4 pass alone permits
normal worker/memory/restart S5 and separate groupedINT4 S6 (loss<=2pp, additional
generation<=1024, training0). No independent acceptance is self-granted.

The data-reset and storage-only phases below are completed history.

## Completed: native project serialization after the data reset

On 2026-09-19 the user authorized deleting all learned artifacts, corpora, raw
user data, long-term memory and execution logs, retaining Rust source and Git
history. The local artifacts, logs and build output directories have been removed.
No default operating memory directory existed. The protocols below are historical
and do not authorize restoring their deleted inputs or starting another old arm.
No new training or quality improvement occurred during this reset.

The latest user request removes project JSON use in product, training, tools and
tests. SQLite remains for memory transactions/indexes. Direct serde_json and
safetensors dependencies are removed; the existing generic libraries' transitive
dependencies remain permitted. R3BIN replaces text metadata, standalone tokenizer
serialization, IPC and diagnostic record streams. R3MODEL/R3CORP/R3TOK/R3ER keep
their domain-specific binary layouts. No JSON parser, JSON writer or text fallback
is used by project code. Text checkpoint import is retired explicitly.
Old JSON instructions/paths below describe historical interfaces, not supported
inputs to the new binary paths. Do not recreate deleted runs or their bindings.
Focused TINY serialization/process tests are allowed; no SMALL quality training
or experiment restart is part of this migration. Storage verification cannot
establish model quality or Goal1 readiness.

The follow-up storage optimization rechecked that artifacts/logs/default memory
remain absent and uses only temporary synthetic records for measurement. Owned
R3BIN records avoid redundant tree copies. Large metadata/row storage can use
bounded lossless Zstd, while canonical hashes, IPC and tokenizer bytes stay raw.
The common panel publisher binds the actual stored bytes. Stream expansion/item
budgets are cumulative; legacy raw storage is still readable. Benchmark and
integrity results are separate from learning; no optimizer update is authorized
or performed for this optimization. Build caches are not learned/user data.

## Closed at8: user-authorized retention retry01 after the save-reason repair

Retry01 ran from execution HEAD49f0e6e693f8dc6b43b6b85a458e5f0877ef924a with
unchanged candidate source c2ce9c3ce1c456b54702ee6611f26157794836e1. Parent parity8,
first1 save and fresh-process Adam/state resume passed. It reached8, saved24318,
then completed the frozen224 screen: OLD9/64, CROSS7/64, QA18/32, NEW0/64,
errors18. The OLD/CROSS immediate retention guard stopped the attempt. Final
command remains Failed/QualityGuard, error=None, resume=false/candidate=false.
Fresh pure report verified this outcome and preserved all89 attempt files.
No16/32/64/128 or full1424 runs, no second automatic retry, no source changes
while learning. New SMALL8, generation232, diagnostic forwards24/backwards0.
The paragraph below records the user's authorization of this now-closed attempt.

On 2026-09-19 the user explicitly requested another execution after correcting
the error. This authorizes one separate retry01 from the same A75-R24310 parent,
not resume of the failed step24311 run. The original terminal, command, checkpoint,
raw and consumed update1 remain immutable. Use the corrected candidate
c2ce9c3ce1c456b54702ee6611f26157794836e1 and a new registered root under
artifacts/retention-first-retry01-20260919/. No model/trainer change accompanies it.

The retry keeps Q4/R2/T2, the exact native pools/tape, LR1e-4, loss/first-target8,
Adam/clock, tokenizer, F32/backend and all retention gates specified below.
The new attempt has at most128 SMALL updates including first1/fresh resume,
generations4096, diagnostic forwards256/backwards8, command1800s/model wall7200s
and cleanup120s reservations. Keep prior costs separate; related TINY reruns plus
the previous61 remain below the original128 limit. No further automatic attempt
or budget extension follows a new failure. Verify corrected-source direct checks
before freezing the source/binary and registering the attempt. Record actual
counts, saved native identities and quality stops independently from code PASS.

## Closed after execution failure at1: R3-RETENTION-FIRST-1.0

The single R-REPLAY used source56f3f0eb3029dc549eee4cfd3bd052ffc4cb6243.
Parent parity8 passed; one SMALL update consumed input2563/target238 and Q4/R2/T2.
The first probe checkpoint call then supplied an unsupported termination string.
The command failed with IntegrityFail/non-resume, preserving a failure checkpoint
at24311. Post-update probes and all8/16/32/64/128 quality panels are NOT_RUN.
Source c2ce9c3ce1c456b54702ee6611f26157794836e1 reuses the existing native screening
status and passes a direct TINY writer/fresh-loader regression (optimizer0). It
does not change the failed run or authorize its remaining127 updates. No new attempt
or post-fix SMALL execution occurred. Retention/new skill remain unmeasured and
H3/S4/S5/S6/Goal1 remain unpassed. The protocol below records the closed study.

Base source eb66357fbee97de67b637ec0b1986e636ebd372e; prior report
dbd0eb3fcaf377524574670cc72e325a81f5959c. Preserve T32/C512 and all old failures.
P0 identities → P1 audit publication/ancestor command repairs → P2 pure T32
recount, parent parity8 and native training coverage → P3 registered R-REPLAY
and read-only observer parity → P4 first1/fresh resume, screen8/16/32 → gated64/128.
No legacy JSON/SQLite cleanup, codec redesign or old experiment continuation.

The sole new intervention is Q4/R2/T2 versus historical T32 Q6/T2. Keep the first
four Q and last two T positions from the old tape, replacing only positions4/5
with existing full-copy train examples (at least64 bases; at most512 views).
Keep A75-R24310, native Adam/clock, LR1e-4, default loss/first-target weight8,
tokenizer and F32 Accelerate threads1. No validation/conditional/sealed labels in
training. Freeze provenance, exact prompt conflicts, tape/token counts and inputs.

Screen remains OLD64/CROSS64/QA32/NEW64. Stop for OLD/CROSS loss>=12 once,
errors>=5 and parent+4, QA loss>=4 twice, or execution/data/storage/numeric error.
32→64 requires OLD>=60/CROSS>=56/QA>=18/errors<=2; 64→128 additionally requires
NEW>=4 and base4>=1. At128, only retention plus NEW>=8/base4>=2/errors0 permits
the same checkpoint's full1424 panels. These are spending gates, not H3/S4 PASS.

Ceilings: one R attempt128 SMALL updates, generations4096, own diagnostic
forwards256/backwards8 batches, TINY128; model wall7200s, command1800s and120s
cleanup reservation. First1 is included, no extra SMALL save preflight, no retries
or recycled remainder after stop. Observer measures true Adam delta/norm/clip and
fixed8 train probes at0/1/8 without changing optimizer or sampler; optional dot
product may be explicitly skipped. Publish actual counts and saved identities.
H3, S4/S5/S6, Goal1 and external acceptance remain separate and unpassed.

## Closed at32: R3-QUALITY-RECOVERY-BOUNDED-BRIDGE-1.0

Source eb66357fbee97de67b637ec0b1986e636ebd372e. FB01/FB02 direct regressions,
C512 full-state/raw audit, fresh64 and numeric60 passed. The single T-SCREEN then
executed first1+fresh31 and saved step24342 before evaluating224 frozen cases.
OLD64→16, CROSS60→11, QA20→12, NEW0→0; errors0→28. The registered immediate
quality stop fired. Terminal and command preserve QualityGuard/non-resume/Failed,
and a fresh read-only report verified that research outcome. No64/128/256/512 run,
no old C/T repair, no further registration and no use of the unspent480 updates.
New SMALL32, generation288, own forwards60, TINY63 (including standalone resume3).
H3/S4/S5/S6/Goal1 are unpassed.
The protocol below records this closed study and does not authorize another attempt.

Base source e08687eff768dd8b6426accebdfe148d3ce8b556, report HEAD
92c6ca1e0f12135c83a4f3bc334039332672bb83. Preserve the failed C512 publication,
T0, successful replacement-01, native corpora/checkpoints and original raw.
R0 preservation → R1 shared512 terminal capacity and registered observation root
→ R2 direct Record/publisher/process regressions → R3 diagnostic-only C512 audit
→ conditionally one new T-SCREEN → R6 bounded result/publication. No C retraining,
old terminal reconstruction, replacement-02 or restored historical eligibility.

R3 loads the entire native/Adam/objective state, binds original panel memberships,
and rescores olddev256/CROSS512/ordinary400/newdev256/conditional144 independently.
Exact prompt-token conflicts block learning. Metadata-selected normal generation
is limited to32 cases each for parent/C; cached/full comparisons use only the still
correct free prefix before the first error, with abs1e-4 + rel1e-3 per-logit tolerance
and a declared near-tie rule. This diagnostic cannot issue a resume certificate.

Only verified inputs, successful original parent evidence, numeric parity and the
two repaired boundaries authorize a separate T-SCREEN from A75-R24310, retaining
the existing T corpus, Adam, tokenizer, default loss/first weight8, constant1e-4 LR,
F32 CPU Accelerate threads1 and batch8=anchor6/focus2. The first1/fresh resume counts
toward512. Freeze OLD64/CROSS64/QA32/NEW64 by metadata before learning. At32/64/128,
stop for OLD/CROSS loss>=12 once or>=8 twice, QA loss>=4 twice, or total errors>=5
and>=parent+4. Runtime/data/save/nonfinite/cancel errors stop immediately. At128
and256, extend only for NEW gain>=8 and base4>=2, OLD/CROSS loss<=4, QA loss<=2,
and no outstanding execution error. Otherwise close the research honestly.
At512, evaluate all1568 final cases; reuse ordinary watch rows without double calls.

New ceilings: SMALL512, TINY128, generation4096, own diagnostic/teacher192
(R3 forward<=64), model wall7200s, each command1800s with cleanup reservation120s.
Compile/test time is separate. No automatic budget renewal, historical C promotion,
best-midpoint selection or additional H4–H8 training. H3 joint, S4/S5/S6 and Goal1
remain separate and unpassed until their actual independent acceptance criteria.
The closed attempt below retains its original failed status and consumed budget.

## Closed failed attempt: R3-BRIDGE-EVIDENCE-RESTART-1.0

B1/B2 direct boundary repairs passed at frozen source
e08687eff768dd8b6426accebdfe148d3ce8b556. The single replacement-01 parent
observation, pure report and preparation passed. C executed512 updates, saved
step24822 and returned every final generation panel and own teacher64, then
terminal serialization failed (`R3ER: count bound`). The kind14 decoder bounds
the actual LR vector at256 while the fresh resume segment accumulated511 rates.
This additional storage defect remains unfixed in the frozen execution source.
T did not start. No successful terminal/command/close certificate exists for C512;
a fresh read-only pair report rejects the missing terminal without changing files.

STUDY_STATE=FAILED_PUBLICATION_AFTER_C512, comparison incomplete, SMALL512/TINY92,
new SMALL completed generations2176/own teacher128. Known returned generation
tokens75,647; missing canonical command accounting remains UNKNOWN. Preserved C512
raw shows newdev4/256, olddev6/256, CROSS4/512, QA188/336, aux24/64; these are
observations, not endpoint acceptance. The final native and all failed evidence
remain local. No source mutation to continue this study, rewind to segment00,
second replacement, budget extension or T admission is authorized. A future
repair must first exercise the actual terminal writer/reader at256/257/511/512
rates; any subsequent learning requires a separate decision. The intended
protocol below records this closed attempt, not an active execution authorization.

Base source2a1010105e0914e4c84cc94b7da707171baaecbe, report
5833ee9f6790b7c0c9489b648e9f4f2e0b229acd. The failed original parent observation
remains FAILED_PUBLICATION/INTERRUPTED_UNKNOWN and cannot resume. This contract
authorizes exactly one separate replacement-01 after the direct repairs pass.

B0 preserve identities/raw → B1 root registration and pure teacher verifier →
B2 actual separate-file TINY observation/approval/fresh resume/close → freeze source
and binary → B3 register replacement, reuse identical data and A75-R24310, observe
sanity64/dev256 and own teacher64 once → B4 C/T first1+fresh511 each → B5 recount.
Missing registered children block retry and later models. Successful zero scores
permit admission; cancellation, nonfinite values, integrity/publication failure or
unknown completion block it. No automatic replacement-02 or repaired old final.

The only learning variable stays the nonselected entity in existing C/T corpora.
Both arms retain the native parent/Adam/tokenizer/default loss/first weight8,
constant actualLR1e-4, F32 CPU Accelerate threads1, same6:2 tape,512 updates.
New budgets: SMALL1024, additional parity SMALL0, TINY128; generation4608 maximum
(320 parent +576 mid +3136 final +conditional48 =4080 planned), own teacher192,
input2M/target500k per arm, model7200s, command1800s/cleanup120s. Previously spent
generation320/teacher64 and unknown tail remain historical, never zeroed or reused.

R3ER registration/proof variants retain explicit file references and typed data.
Reports use the same pure teacher verifier as scope3 approval and perform no
collection or publication. Test-sized denominators retain the production separate
start/entry/row/final layout; synthetic64-row schema tests are not64 forwards.
Root registration only detects missing children while the root evidence survives;
it cannot detect deletion/rollback of the entire root and all external evidence.

Primary final512: newdev256 T−C paired gain/loss and64 base×4-view joint correctness.
Signal: T>=192, T−C>=26, base4>=40, newdev errors0, QA>=178. H3 joint remains
olddev244/entity254/event254, CROSS487/entity507/event507, QA178, required errors0,
same durable native endpoint. Report aux64 and original conditional144 separately.
Signal and H3 are separate; seal/S4/S5/S6/Goal1 acceptance are not authorized here.
Only a joint/signal candidate permits metadata-selected fresh48 within this budget.

## Closed attempt: R3-QUALITY-FIRST-BRIDGE-1.0

Base source a89fbc977ed9421489a43fc4b4eb6f42a5ebbd25, report
91ce87ba3286637e9372f7e70f337b73de1b0aaf. Preserve all prior raw, corpus,
checkpoint, failed studies and local changes. Rust1.98.1/offline lock, F32 CPU
Accelerate/one thread. Existing native storage, SQLite compatibility and IPC stay fixed.

Q0 identity → Q1 ambient-policy resume / plan-wide sticky failures / native caller
regressions → Q2 coverage and frozen16×4 bridge sanity → Q3 registered matched
data and native resume smoke → Q4 C-COPYMATCH versus T-TEMPORAL → Q5 raw recount
and transfer/preservation → Q6 publication. No storage framework or new codec.

Both arms use the verified A75-R24310 migrated v2 parent, retained Adam/tokenizer,
default response loss and first-target weight, constant actualLR1e-4, batch8
with the same ordinary anchor6/focus2 tape. Only the nonselected record's entity
differs: alias in C, same entity in T. Two records, same query, answer, values,
IDs/status/time/order; token length parity is measured. Focus128bases×4views,
each view exactly twice across512 updates. T development64 unseen bases×4views.
Seal namespace stays reserved without opening its files. Original conditional144
is reused only for development transfer, not copied into training.

Budgets: quality SMALL1024 (each arm1+fresh511), extra SMALL parity0;
TINY optimizer128 total, scalar counted separately; generations4608 maximum
(4080 planned including conditional candidate48); own teacher192 maximum.
Each arm input2M/target500k tokens; pair7200seconds, command1800/cleanup120.
Cancel/nonfinite/data/source/save failure blocks the other arm; only validated
pure-time pause can resume. No re-registration, automatic extension or best-midpoint
selection. Normal wrong output is distinct from execution failure.

Mid256: newdev256/watch32; saved final512: oldH3dev256/CROSS512/ordinary400,
newdev256 and conditional144. Parent newdev256 once; existing old panels reused
only after identity checks. Teacher metadata train32/dev32 for parent/C/T.
Limited signal requires T newdev>=192, T−C>=26, all4-view correct>=40/64,
newdev errors0 and QA>=178; disclose every oldcopy/entity/event/aux result.
H3 joint stays dev244/entity254/event254, CROSS487/entity507/event507, QA178,
required-panel errors0 and durable same endpoint. Signal and H3 are separate.
S4/S5/S6/seal/Goal1 are not automatically authorized by this study.

### Current stop: parent observation publication failed

Q1 repairs are published at10a968ec1eb0b253f21ebc318f6973d2da5ac6fa.
Q2 native paired data and actual parent observations exist. Q3 native TINY
continuous2 versus fresh1+1 is numerically equal. Q4 C/T training is NOT_RUN:
the parent observation attempted320 generations and64 own teacher forwards,
then its final record failed the inherited `teachers == 0` decoder invariant.
This is an implementation/publication failure, not evidence of a failed optimizer
or a completed quality experiment. The start, every returned row, panels, teacher
rows and provisional proof remain immutable. No positive final certificate exists.

The corrected decoder permits at most64 teacher observations; the bound reader
requires zero for old verification scopes and the exact expected count for the
bridge observation scope. Real TINY subprocess observation/teacher/fresh read
and no-retry regressions exercise this path. Read-only recount cannot issue a
new positive final or reopen the failed attempt. A fresh SMALL study remains
blocked; no automatic replacement root, budget reuse or extra optimizer calls.

Implementation stays in the existing data generator and R3ER runner: one native
paired corpus per arm, verified origins, shared tape/LR/Adam, first-update save,
partial evaluation continuation, six required final panels and typed probes.
No new module/codec/DB/IPC was added. The SMALL registration and512-update paths
are implemented but have not been executed in this attempt; do not call Q3/Q4
accepted or STUDY_COMPLETE. The candidate48 branch is NOT_RUN_NO_CANDIDATE.

One next hypothesis remains untested: competitive temporal-record exposure can
improve same-slot selection while preserving ordinary QA. The current single-record
sanity control also fails, so temporal selection alone is not an established cause.
Any replacement study needs explicit new authorization and must retain this stop.

## Closed: R3-NATIVE-CORPUS-OBJECTIVE-BINDING-1.0

N0–N5 implementation and bounded observations completed. Exact objective resume,
native source/default paths and SMALL input parity verified. Conditional full0/144
for each fixed model is a negative diagnostic, not recovery. Actual quality SMALL0,
parity SMALL4, generation320, own teacher288, TINY optimizer108, scalar0. Warm3 and
fresh-process3 preparation measurements completed; no overall training throughput
experiment. No remaining model-call budget or implicit next learning authorization.
One aux-preservation hypothesis is proposed only in EXPERIMENT_STATUS. H3/S4/S5/S6
and Goal1 remain unaccepted, seal unopened. Details below retain the closed scope.

Base source af63823346ebb6e11cc23451a8ba0c704eb34039, report
5c35c8c452f6d6790f121af2e2a9e018cc01641c. Preserve all historical studies,
unknown/failed usage and terminal eligibility. SPAN is not adopted or repeated.
N0 identity/preservation → N1 checkpoint-owned objective → N2 lossless native
source → N3 real default train/eval/cache and process parity → N4 fixed-weight
conditional diagnosis → N5 equivalent preparation measurements and publication.

Budgets: quality SMALL0; PATH_PARITY SMALL4 (legacy2 versus native1+fresh1);
generation320 (version parity16+16, conditional144+144); own teacher288;
TINY optimizer128 across failed and successful regressions. Command1800s and
cleanup120s, one heavy process, no source edits during learning/measurements.
Cancel, nonfinite, changed source/data, sync/checkpoint failure stop subsequent
model work. Unknown killed-process usage is not zero. No sealed-panel opening.

Required boundaries: standalone v1 resumes fail unknown; proven original default
and span policies migrate to new v2 without changing weights/Adam/tokenizer/clock;
generic default and supported span native process resume remain exact. Default
source APIs consume R3CORP with no JSON fallback, cache uses verified owned train
only, and every token/target/mask/role/order compares to the explicit legacy import.
The SMALL parity endpoints are not quality candidates or future study parents.

The conditional panel is frozen before outputs: 24 dependent bases × current,
past question, value swap, evidence order, unused wording, unrelated distractor.
Two fixed endpoints only: A75-R24310 and B-BASE24822. Preserve full raw tokens,
strict bytes/EOS/error and binary usage. Teacher gold/foil first-divergence margin
is distinct from normal greedy. Report joint/paired correct results and error
strata, recount prior role probes and train task exposures; propose only one next
hypothesis or UNKNOWN. No automatic learning authorization follows diagnosis.

Warm3/fresh-process3 compare equal source JSON/raw/Zstd3; separate train-only cache
table. Record read/hash/decode/tokenize/batch/encode/verify/durable publish, bytes
and RSS. Fresh process is not cold OS cache. Preserve originals in retained totals;
startup improvement is neither training throughput nor model-quality recovery.
Code/format support, diagnostics, H3/S4/S5/S6 and Goal1 are separate verdicts.

Current call map: `data::load` → owned native corpus; `load_legacy` → explicit
historical/import only. `corpus prepare/binding-pairs/qa-pairs/subset` emit native;
generic train/evaluate and tokenizer training read native. `native path-prepare`
binds imported full source to the verified migrated parent and old ordered tape;
`native run` consumes native source plus R3TOK and original batch/optimizer.
`upgrade-resume` checks original binary policy/terminal/native/close before writing
v2. Older JSON recovery controls require `--legacy-json`; unsupported old optimizer
entry points reject before learning. Existing checker supports scoped
`quick --native-corpus`; this does not claim all historical tests were rerun.

## Closed: R3-DATA-BINARY-AND-TARGET-LOSS-1.0

E1/E2 direct process repairs verified; E3 objective implemented; E4 completed
BASE512/SPAN512 with both joint gates failed; E5 cache prototype and equivalent
batch measurements verified. SMALL1024, generation2928, own teacher192, TINY109.
No budget remains for SMALL learning and no joint candidate authorizes confirmation
or seal. The failed quality result and original records remain immutable. E6 publishes
the verified implementation, measurements and negative study; independent review and
H3/S4/S5/S6/Goal1 acceptance remain outstanding. The following records the closed scope.

Base1e25be2cef801dd67a6515e5c33823a0d1b06a92; preserve closed studies and all originals.
Evidence: `artifacts/data-binary-target-loss-20260918/`. Rust/Cargo1.98.1, offline lock,
CPU Accelerate/F32/threads1, one heavy process. E0 identity → E1 publication pending
→ E2 partial-panel resume → E3 train-only role objective/probe64 → E4 BASE/SPAN
→ E5 independent token-cache parity/measurement → E6 recount/publication.

E1 requires durable Pending before final publication, serialized readers, and no
authorization after an observed commit failure. E2 allows explicit RESTART/COOLDOWN
pure-time pauses with unchanged complete binary row prefixes, panel/model/step/policy
identity and exactly-once complete guard decisions. New objective resume must explicitly
use the same capability. Failed/unknown/ambiguous states never authorize another arm.

E3/E4 parent is A75-R24310, physical SHA
50b927dd41771c39ca5e7138ca28aaf8193a96460eff9a2aae92ed444015f09c.
Same Adam, exact6:2 tape/batch8, actualLR1e-4, tokenizer/model/F32/clip/decay/greedy.
Change only target role overlap weighting: b=1+r; per episode Z=sum(a)/sum(a*b);
objective=sum(Z*a*b*CE)/actual target count. The original first-target weight stays
in a, prompt/padding remain zero, EOS remains supervised. Use full-answer token raw
byte overlap; preserve per-episode weight mass. Annotations stay training-only.
Unsupported ordinary grammar retains baseline with coverage disclosed; all focus
grammar must validate. Concrete label contradictions stop DATA_CONTRACT_FAIL.
Train64 teacher probes are fixed by metadata before outcomes, never dev-failure selection.

The current implementation sorts train ordinals by SHA256(seed17, category, family,
scene, ID), takes64 and stores that exact order in the objective policy. BASE and
SPAN use one binary, the same pool/tape/parent and constant actualLR bits. First
target weight is8 in the observed parent. Per-update phase times, gradient/update
norm and clip flag are typed segment metrics; native save/reload and panel time
are separate observed log measurements. The train-only module uses the existing
limited renderer grammar, checks copied fields/cited source, and falls back only
for unsupported ordinary forms. No annotation is passed to greedy generation.

Commands reuse `replica-train recovery native`: `objective-prepare --parent-arm
PATH --expected-parent SHA --output NEW_ROOT`, `cooldown-verify --root NEW_ROOT`
for the shared parent16 finalizer, `run --root NEW_ROOT/B-BASE` then `S-SPAN`,
and `anchor-report --root NEW_ROOT` for raw/paired recount. Only a clean authorized
time pause may use `run --resume segment-NN/terminal.r3er`. Failed/unknown command
or probe state blocks retries and the other arm. `cooldown-verify --confirmation`
uses the same consumer and only runs for a joint candidate. These command names
reuse the existing bounded implementation, not the closed K/D authorization.

Budget: SMALL1024 (BASE512+SPAN512), generation4096, own-model teacher256 (planned192),
TINY optimizer128 including failed/quick runs. Each command1800s/study7200s/cleanup120s.
Parent parity16; each mid256 dev/watch288; each final512 dev/CROSS/ordinary1168;
one joint-candidate fresh1168 only if eligible. Final watch is derived. Final absolute
step24822; no intermediate selection, old-study restart, or automatic budget extension.
Joint gates: dev244/entity254/event254; CROSS487/entity507/event507; ordinaryQA178;
zero errors and all native/panel/stop bindings. Existing guards stay fixed. No joint
candidate means no seal/H4–H8/S4–S6 advancement. Goal1 remains separate.

E5 follows the study on a separately identified source. Compile a bounded immutable
typed cache from owned train snapshots, preserving tokens/masks/response_start/order
and source/tokenizer/framing/policy hashes. Measure JSON, R3ER, raw u16/u32 cache and
Zstd3: retained-total bytes, read/hash/parse/tokenize/first batch (warm3/fresh-process3),
and consumed tape batch preparation5. Fresh process is not cold OS cache. No raw
deletion, silent rebuild, model format/DB/journal changes, or storage-to-quality claims.
Commit/push only verified closed stages and verify actual full remote SHA.

## Closed: R3-PREFLIGHT-ONCE-AND-COOLDOWN-1.0

P0–P5 completed within SMALL512/generation2928/TINY85/scalar-optimizer0. PV01 and
real native one-update/fresh-process resume were verified. Final K: dev238/CROSS458/
QA177; D:237/459/181, each one dev UTF-8 error. Complete raw recount and separate
QA/aux paired counts agree with saved results. STUDY_COMPLETE_QUALITY_FAIL; fresh
confirmation NOT_RUN_NO_JOINT_CANDIDATE, seal NOT_OPENED, Goal1 not ready.
Original63 registered file hashes and487 untracked files were preserved. No automatic
learning extension or candidate promotion. The specification below records this closed
study, not permission for another fork. See EXPERIMENT_STATUS for source/binary/evidence.

Base source50a0fb72f552cc130fbd5598019222c7a7bd95fe, report198a1a032368f023d81ab744e59eccb1cd6b6e53.
P0 preserves the previous preflight, completed C50-R/A75-R, failed pair and raw inputs.
New evidence root: `artifacts/preflight-cooldown-20260918/`, originals.sha256.
P1 records verification intent before observed work, each returned raw row, and a shared
success/failure outcome with known/reserved/unknown usage. Missing finalization blocks
retry and subsequent arms; legacy successes remain explicitly read-only observations.
P2 reuses real TINY process fixtures for cancel/deadline/publication/kill/concurrency,
positive parity, training resume and close regressions; final quick once before learning.
Then P3 registers one A75-R24310 fork: K-KEEP constant1e-4 versus D-DECAY single cosine
1e-4→1e-5 at local updates1..256. Same Adam,6:2 pools/tape, tokenizer/model/F32/CPU
Accelerate/threads1. P4 saves first update and resumes in a fresh process, evaluates
dev/watch at128 and full dev/CROSS/ordinary at256. P5 recounts fixed endpoints and
paired outcomes. No old ratio rerun, SMALL4 preflight, journal/DB/format/core changes.
Limits: SMALL512, TINY/scalar128, input1M/target250K per arm, generation4096, teacher0;
model7200s, command1800s, cleanup120s. Parent parity16, two mid288, two final1168,
conditional one candidate fresh1168. Joint gates/guard remain fixed, H3 seal NOT_OPENED.
No joint candidate means study complete/quality fail and no further learning. Verified
stages explicitly commit/push with remote SHA checks; independent acceptance stays pending.

The registered D policy is `1e-5 + 0.5*(1e-4-1e-5)*(1+cos(PI*(n-1)/255))`.
Local n never replaces the inherited Adam step24310+n. LR-policy changes also change
the cumulative effect of the unchanged decoupled decay coefficient; this is not a
claim about gradient step-size alone. The previous512 A75 draws were exactly reproduced,
then the next256 frozen (input627563,target65772 each). Parent QA181 is observed; the
unchanged acceptance floor178 retains its original provenance. Mid128 is not a winner.

## Closed: R3-DURABILITY-PAIR-RESTART-1.0

D0–D4 execution is closed on source50a0fb72f552cc130fbd5598019222c7a7bd95fe.
Journal retry ACK and command finalization regressions passed; real SMALL continuous2
versus fresh-process1+1 matched exactly. The separate C50-R/A75-R attempt completed
512 updates each with verified native endpoints at24310 and complete raw recount.
C50-R: dev241/256, CROSS461/512, ordinary170/336, one dev generation error.
A75-R: dev240/256, CROSS462/512, ordinary181/336, one dev generation error.
Both joint gates fail. MODEL_PAIR=STUDY_COMPLETE_QUALITY_FAIL; H3 seal NOT_OPENED.
New SMALL1028/1028, generation4676/7500, teacher0; TINY79/128, scalar0.
No automatic extension, intermediate-point promotion, further candidate generation,
H4–H8 execution or Goal1 acceptance. Original failure pair11/11 hashes unchanged.
The retained specification below records this completed attempt, not another run grant.

D0 starts at source2e4121035aabd11918d64875285661ad8e3a8f91 and report
1ea5abe9ae63e12ee8a7e9e8e6731b6b274d07fb. Preserve the failed
`artifacts/native-storage-quality-20260918/anchor-pair/` (C50 updates256,
endpoint absent, A75 not run). New evidence belongs only under
`artifacts/durability-pair-restart-20260918/`; original-pair.sha256 binds the
read-only files. This replacement attempt is distinct; no old flags/budget are reset.

D0 identities → D1 journal retry sync and command outcome → D2 native TINY and
SMALL save/resume preflight → D3 C50-R then A75-R → D4 endpoint recount/publication.
Change existing journal/experiment_record, direct tests/checker and these existing
documents only. No new framework, dependencies, model equations, tokenizer or DB.
Runtime verified Apple M4/24GiB, installed Rust/Cargo1.98.1, Accelerate CPU/F32,
VECLIB_MAXIMUM_THREADS=1/RAYON_NUM_THREADS=1; locked/offline builds.

D1 follows run_native → immutable terminal → close_native → command finalization
→ shared effective outcome in anchor_budget/anchor_report. A comparison alone is
provisional. Missing/failed finalization blocks another arm and normal pair approval;
only a verified time-only pause permits the same arm to resume. Journal retry ACK
requires a successful sync_all on that call, including after replay.

D2 uses the same F512 parent and first two registered training batches: continuous2
versus split1/save/new-process/load/1, total SMALL4 and optional generation≤8.
Compare exact weights/Adam/clock/sampler/tokens/LR; no preflight quality claim or use
as a training parent. Failure closes the preflight with no automatic retry.
D3 is a single newly bound attempt with replacement receipt hashes, same F51223798,
pools2048/512, tape seeds, inherited Adam, actual constantLR1e-4; C50-R4/4 versus
A75-R6/2. Each512 updates, 2Minput/500Ktarget, eval256/512 on dev256/CROSS512/ordinary400,
watch32 derived. Save/verify before full evaluation. New SMALL≤1028 including preflight;
TINY/scalar≤128 including quick. Model commands total≤7200s, each≤1800s plus cleanup120s;
generation≤7500, teacher0. One heavy process, frozen source/binary while learning.
Joint gates and existing guards remain unchanged; stop/integrity/save/close failure
halts the pair, completed low quality permits the other arm. No automatic extension,
seal opening, model promotion or Goal1 acceptance. Closed stages commit/push explicitly
with full remote SHA verification; originals/private artifacts never enter Git.

## Closed: R3-NATIVE-STORAGE-QUALITY-1.0

Execution closure: G1 boundaries verified; G2 halted at C50+256 on native save
validation failure, A75 NOT_RUN, QUALITY_INCONCLUSIVE. Do not restart this pair or
reuse its remaining budget automatically. The fork-budget boundary has a direct
zero-update native save/reload regression. Original failure/terminal remain intact.
G3 native storage/cold exact probes verified; raw F32 remains the default. G4 is an
explicit single-writer prototype, independent of model quality. H3/S4 remain below
acceptance, seal NOT_OPENED, S5/S6 and Goal1 acceptance remain unfulfilled.

Source b18ec31bb78a8d0ef0adf985b46697a2d4993474, report
d19f39972ab70799527b567d5ad35a02c76ead5c. Preserve original artifacts and closed runs.
Development DAG: G0 identities → G1 close boundaries → G2 anchor pair → G3 model
storage measurement → G4 experimental journal → G5 publication/independent review.
Product data DAG: original event → durable record → derived lookup/index → model
evidence input → native generation → validated result event.

G1 changes only experiment_record, its existing process tests and necessary checker
filters. Test terminal-model agreement with valid alternate native files and stored
stop rejection after a new process, preserving existing time-split regressions.
Use direct `cargo test --locked --offline --features accelerate,test-support` filters.
G2 extends the existing typed native runner with explicit experiment authorization;
it never resumes an old ineligible run. Register C50/A75 from the exact F512 native
parent, same pools/Adam/LR1e-4, batch8 anchor4/focus4 versus6/2. Each≤512 updates,
2M input/500K supervised tokens; pair≤1024 updates and7200 command seconds, each
segment1800s plus cleanup120s. Total SMALL generations≤7500, teachers default0
(at most64 explicit diagnostic calls). Evaluate shared0 and each256/512 on full
dev256/CROSS512/ordinary400; derive watch32 from ordinary. Existing joint gates and
guard policy stay fixed; stop/cancel/budget remains sticky. No automatic extension.
Only actual joint PASS permits one full fresh-process candidate check and the existing
seal procedure. H4–H8 require their actual remaining authority; no budget reset.

G3 reuses neural/artifact and existing Rust validation executable for F512 raw F32
INFERENCE/RESUME and lossless zstd1/3 probes. Report component bytes, actual I/O,
durable publication/load timings, bit/output parity, at least3 warm and fresh-process
runs and memory limitations. No production codec/quantization or tensor changes.
G4 permits one cohesive journal module/test, narrow archive/event-view reuse, explicit
experimental CLI, and existing Rust measurement probes. Fix R3JRN layout before code;
test single-writer lock, exact append/restart/idempotency, durable-prefix recovery,
history/current/relations and same-data SQL/archive parity. Synthetic1K/10K/up to100K,
raw/zstd1/zstd3 and64/256KiB snapshot blocks; disclose search/durability differences.
The operating SQLite path and original evidence stay unchanged. No new daemon/engine.

One heavy process, Rust1.98.1/Accelerate/threads1/offline lock, source frozen during
learning. Record actual optimizer/generation/teacher counters, exposure, native/raw
hashes and stop conditions. Closed stages receive explicit source/docs commits and
normal pushes with full remote SHA verification; independent acceptance stays pending.

## Active: R3-BINARY-EVAL-RESUME-1.0

Base source62147dc5854ca07a4c0785c0f006734d62074200, reportd075382539cc5f8bf7ba6de01b8d98a8d7aa7ffb.
J0–J5 are closed in repair scope, with independent review pending. Baseline failures
were reproduced; separate-process native TINY save/time-stop/resume/close and the
stable candidate quick53 passed. Final importer changes passed five direct tests and
native close with JSON reads denied. F/N/N256 current raw recount agrees; F512 parity
is32/32. SMALL optimizer0, TINY97/128, scalar0; no further executions are scheduled.
The final source is b18ec31bb78a8d0ef0adf985b46697a2d4993474, verified on origin/main.
J1–J3 implement typed, schema-specific binary snapshots,
evaluation payloads, decisions, explicit native references, segment and close receipts.
Use the current native model loader/save, varint primitives, RunControl and scoring rules.
New native control must not reopen JSON sidecars or hash JSON reserialization. Preserve
the existing opaque model/tokenizer identities, weights, optimizer mathematics and DB.
Verify continuous versus separate-process/segment TINY evaluation, time stop, resume and
close, including the problematic diagnostic f64 and sticky cancellation/quality failures.
TINY/scalar optimizer budget128 including failed runs; new SMALL optimizer0.
After these boundaries pass, J4 imports/recounts existing F/N/N256 raw once and permits
only F512 dev16+ordinary16 normal-greedy parity, selected by metadata/ID before output.
No N256 regeneration, checkpoint sweep, learning extension, seal or candidate promotion.
Compare equal-content uncompressed records with warmup5/timed20 or fewer, including
durable write/sync/reload; distinguish schema deduplication from codec effects.
J5 records observed identities, remaining JSON by role and one unexecuted model-test
proposal in EXPERIMENT_STATUS. The repair verdict is BINARY_EVAL_RESUME_VERIFIED;
model quality is unchanged. The retained execution specification above is closed,
not authorization for another import/parity/learning run or an automatic budget reset.

## Closed: R3-H3-STATE-DATA-RESULT-1.0

B0–B6 diagnostic scope is verified by the implementer; independent review is pending.
F/N immutable raw recount agrees with the existing scores, with historical receipt
binding ABSENT and current raw VERIFIED. F16/N16 fresh-process raw differences are zero.
N256 post-hoc DEVELOPMENT: dev252/256, CROSS457/512, ordinary161/336, auxiliary57/64;
only dev meets its gate. New SMALL optimizer0, generations944, same-model teacher944.
N256 reused its verified dev256 raw and generated CROSS512+ordinary400 once. Original
failure/stop/resume records, weights, Adam and datasets remain unchanged; seal NOT_OPENED.
This closes DIAGNOSTIC_BOUNDARIES_VERIFIED, not H3/S4/S5/S6 or Goal1 acceptance.
No further generation, checkpoint search, budget extension or learning is scheduled.
The following execution specification is retained as the closed diagnostic scope.

Continue from bf705ad823d132a2f91e3df7323d49c3c23e2ee4. New SMALL optimizer calls are zero;
the closed C/L and F/N budgets, failed quality gates and resume=false remain immutable.
Repair only the shared evaluation/guard transition, frozen owned-data loading, and
complete raw-panel/receipt verification followed by independent strict rescoring.
Use existing RunControl, scorer, native loader and checker; retain model mathematics,
tokenizer, normal greedy/strict UTF-8, tensor/storage formats and original artifacts.

B0 preserves actual source/artifact identities. B1 must reproduce time interruption after
raw publication and after checkpoint/decision publication, reconcile pending decisions
before another optimizer call, avoid duplicate streak application, retain simultaneous
quality/time/cancel reasons, and demonstrate normal short TINY native continuity.
B2 binds A0/frozen/ordinary, source and current arm corpora, CROSS and policy lineage at
baseline, renewal, new/resumed arm, final ordinary and close. Evaluators and the renewal
generator consume the verified owned snapshots. B3 requires ordered complete dev256,
watch32, CROSS512 and ordinary336+aux64; raw tokenizer/EOS/error consistency, independent
field/base/strict rescoring and endpoint status precede any comparison or eligibility.
Malformed/contradictory receipts are integrity failures; partial execution is incomplete;
ordinary model errors remain in the full denominator. Historical missing bindings are
explicit READ_ONLY_REAUDIT, never backfilled receipts or retroactive candidate promotion.

After boundary regressions pass, B4 recounts unchanged F/N panels and permits one fresh
process per final model with at most16 predetermined dev cases. B5 optionally inspects
only the existing N256 native checkpoint, after lineage/preservation checks: reuse valid
dev256 raw and generate its missing CROSS512/ordinary400 once. This is post-hoc DEVELOPMENT,
not a preregistered winner, resume authorization, seal opening or product promotion.
Aggregate new generation cap1200 (retries included), optimizer0; report teacher calls
separately. Each command≤1800s plus cleanup≤120s, one heavy process, no automatic extension.
The implementation below reuses dev and hence needs at most944 new generations.

B6 records source/dirty/binary/native/raw provenance, RED/GREEN and positive regressions,
derived versus newly executed observations, any post-hoc limits and one unresolved next
hypothesis. Publish only related sources/tests/docs with verified remote SHA. Boundary
verification, historical raw agreement, H3/S4/S5/S6 and Goal1 remain separate verdicts;
independent review is pending. No new learning experiment is authorized by this repair.

## Closed: R3-H3-CONTROLLED-PROGRESS-1.0

A0/A1/A2 are implemented, checked and executed. All four arms reached512 new updates;
total2048 updates/4,903,380input/554,581target, approximately65.2minutes of A0+arm execution.
Final F dev242/CROSS459/ordinary176 and N236/449/174 both fail raw H3 and original-QA
retention. The once-only seal remains unopened. NEXT=H3_BUDGET_CLOSED, no automatic
extension, no H4–H8/S4–S6 execution or Goal1 readiness. N256 dev252 did not survive to512
and is not substituted for final acceptance. Full results/provenance are in EXPERIMENT_STATUS.
The registered execution contract below is retained as history, not pending authorization.

Start at report HEAD be6e2b7a7f59e444ffa25d8bae9e1af34d418d66, code d31e53eda459af447eed10d87617e4ee4d11ead3.
Preserve the1024-update H3 native/Adam and its failed quality/closed budget/resume=false receipts.
A0 performs no updates: verify native lineage, recount raw256 and token exposure, reproduce
normal dev256 once, measure original ordinary336/aux64 once, compare both final invalid
prefixes cached/full, inspect byte fragments and bounded actual QK/decay measurements.
Freeze one balanced DEVELOPMENT CROSS panel128bases×4views; report bounded ID capacity
and split collisions honestly. No sealed generation or independent-final selection in A0.

After A0 passes, new explicit EXPERIMENT_FORK C/L share parent weights/Adam/RNG, data and
draw tape. C constant3e-5; L ramps by j/64 from3e-5 to1e-4, then holds. Each≤512new updates,
2Minput/500Ktarget. Evaluate olddev/watch at0/128/256/512; final CROSS512/original400.
Only safe finite improving endpoints permit the conditional F/N comparison: identical
balanced focus distribution/anchor/LR/Adam, fixed512views versus fresh2048views, each≤512.
No Adam reset, topology/tokenizer/decay/temperature/storage/kernel changes. U prefix-safe
decoding is optional, diagnostic-only and cannot replace NORMAL_GREEDY acceptance.

New H3 total≤2048updates/8Minput/2Mtarget/120min, command≤1800s plus cleanup≤120s,
training16GiB/inference12GiB. Freeze source/binary during each run; one heavy process.
Cancel/nonfinite/data/hash/cache/leakage/resource/budget failures stop cooperatively.
Two consecutive olddev drops≥26, watch drops≥4, error increases≥6, or single error rate≥20%
stop that arm. All thresholds are relative to its registered parent. Final errors still0.
Comparison uses common exposure prefixes; unequal endpoints cannot establish LR superiority.

A3 requires normal olddev full244/entity254/event254 of256, CROSS ceil95%/99%/99%, errors0,
ordinary≥max(175,A0 parent), fresh-process raw parity, then once-only sealed256 acceptance.
Only actual A3 PASS continues H4→H5→H6/S4→H7/S5→H8/S6 under their existing contracts below.
No automatic budget renewal; failed stages preserve evidence and block dependents.
Closed A0: normal dev208/256 reproduces every raw/prompt/finish field; ordinary178/336,
aux63/64; crossed development210/512. Parent/model/Adam/raw/corpus preserved, final two
invalid prefixes reproduce cached/full, bounded QK and tokenizer/exposure diagnostics saved.
Quick34/direct4 and release pass; SMALL/TINY/scalar updates0. NEXT=A1 independent C/L forks.

A1 execution registration: `recovery progress-prepare` binds both C/L policies to the same
A0 native parent, Adam content, cumulative step22774, inherited RNG and first512 draws of
the existing skill tape. It records every input/target denominator before training.
`progress-arm` permits only its registered arm and latest clean TIME_BUDGET continuation;
ordinary trainer resume remains rejected. Native budget metadata extends for the explicit
fork; optimizer hyperparameters and original corpus remain unchanged. LR comes from the
sidecar policy, not the ordinary cosine config. The policy clock is retained across segments.
Guards count all generation errors across dev256+watch32 (denominator288); parent208/18/errors2.
Closed/failed segment calls and elapsed time count toward the global budget; an unclosed
segment blocks continuation. `progress-close` checks actual traces and compares common
prefixes before any conditional F/N or fresh-process/sealed acceptance. No seal is opened
by these commands. A0 publication60a4f703b2824be971f267f7936876c205608d51 matched origin/main.

Closed A1: both512 updates at identical actual exposure. C dev225/CROSS259/ordinary178;
L dev227/CROSS289/ordinary185, auxiliary53 (C62/parent63). Neither meets raw H3 acceptance.
L is the conditional A2 parent by the registered old-full ranking, not a claim of general
LR superiority. Both native endpoints/Adam/raw receipts remain immutable. NEXT=A2 F/N
with inherited L policy (already past its64-update ramp), identical anchor/distribution,
at most512 updates each. No sealed/final input has been opened.

A2 registration uses `progress-renewal`: the selected L native endpoint/Adam/RNG is
immutable and both new arms continue LR clock513..1024 at1e-4, cumulative Adam23287..23798.
Balanced128-base strata repeat four times in the schedule. F reuses128bases×4views;
N supplies512bases×4views, each view once. F is the first balanced quarter of the N
materialization, allowing the renewal contrast without changing grammar/distribution.
Original anchor2048views and their draw order are identical. Full heldout entities are
excluded and the old seal's entire namespace is reserved; existing train bindings/raw
are also excluded from new focus. Counterexamples include leading-zero IDs, one-digit
ID changes, value changes, and disclosed context/event numeric equality on base%4==1.
Actual target/input token differences are reported rather than padded or truncated.
Persisted corpus, generator revision/seed/namespace/base/view/update/slot coordinates,
native cursor and tape hashes make resumed sampling exact. The independent serialized
validator checks every new focus view before learning. Final stop eligibility fails closed
when any terminal/cleanup/checkpoint eligibility field is missing.

## Previous: R3-HARNESS-TO-GOAL1-1.0

Renewed continuation after publication f964a2a0308dd64852fd3d33e83e8e431373ab17:
the user again requests H4–H8 and S4/S5/S6/Goal1. First inspect the actual stopped H3
without updates: verify its trace against the frozen tape and native counters; generate
32 exposed and32 unexposed training views, explicitly not heldout; compare cached/full
logits on the actual newly invalid generation prefix. Reuse recorded dev0/128 rows and
keep seal closed. This diagnostic has the common900s/12GiB limits, no optimizer calls,
no corpus/weights/tokenizer/decoding changes. Any repair must follow observed evidence;
the failed run and its QUALITY_GUARD receipt remain immutable. No prerequisite or final
quality threshold is waived by the request to finish. Record findings before choosing
any further learning intervention; do not rerun the same stopped experiment blindly.

Observed diagnostic: exposed32 and unexposed32 training views both0/32; each entity3/32,
event0/32. Unexposed views can share an exposed base and are not heldout. The newly invalid
dev case reproduces every recorded greedy token with both cached and full-prefix logits;
strict decoding sees the same invalid byte46. Trace input/target counts match actual samples.
This does not identify a numerical/cache defect or establish that another512 steps will work.

One explicit renewed H3 attempt is authorized by the user's continuation request after the
reported stop. Keep the original stop receipt/checkpoint immutable and start a new output
with `--renew-from-quality-stop`. This is distinct from the still-forbidden plain resume of
a quality stop. Inherit the128-update checkpoint, Adam, LR3e-5, original tape position and
the original total budgets (including prior updates/tokens/time); do not reset the allowance.
The previously observed invalid IDs are acknowledged in the new policy, not erased from
past scores. Any additional new UTF-8/control/empty still stops, as does the original watch
guard against16/32. Acceptance still requires zero errors, full244/256 and entity/event254/256,
ordinary retention and a once-only seal. At512 total updates, extend only for the originally
required≥4 primary gain since the preceding dev evaluation. No second explicit renewal is
automatic, no cancellation/resource/integrity stop is eligible, and no failed parent moves
to H4. A new failure closes this renewed attempt with the next stages unexecuted.

Observed first renewal:512total updates, dev0/256, entity97/256, context3/256,
event113/256, value127/256, watch20/32. Four invalid UTF-8 cases (three new) stopped
the run. Additional384updates consumed917,689input/103,754target tokens; wall623.75s.
No second epoch was launched under that policy; original and renewed stops remain intact.

Explicit user amendment after seeing those results: complete up to1024 total H3 updates
(512 further), despite the primary-gain extension condition. Keep corpus/tape/Adam/LR,
all aggregate token/time caps, ordinary-watch/resource/nonfinite/cancel/control/empty
guards and final acceptance unchanged. For this final continuation only, intermediate
UTF-8 counts are fully retained and stop on two consecutive evaluation-to-evaluation
increases. Persist the previous count and growth streak across a clean TIME_BUDGET resume.
Use a new output and explicit `--renew-from-quality-stop ... --finish-copy-budget`;
only an eligible stopped512-update state can start it. No automatic further renewal,
no budget reset, and no changes to earlier receipts. Seal remains closed until dev passes.
This amendment supersedes only H3's two intermediate rules in the original plan below.
The user also requests a detailed technical handoff if the final continuation still fails:
record design, actual failures, reproduction, artifact/source identities, tested boundaries
and unresolved hypotheses separately. Do not present a hypothesis as a confirmed cause.

Closed final continuation:1024total H3 updates, dev208/256 (768:123/256), entity233,
context239, value243, event249; UTF-8 errors2, control/empty0, watch18/32. The policy's
UTF-8 counts4→1→2 give growth streak1, so no intermediate guard fired; the unchanged
final zero-error and accuracy gates still FAIL. Terminal SCREENING_BUDGET_REACHED,
comparison=true, candidate/resume=false, NOT_RUNNING. Additional512updates consumed
1,225,871input/138,665target tokens; total2,451,742/277,330, stage elapsed1631.647288s.
No seal/original400 candidate score or H4–H8 execution. Do not automatically extend.
Quick30/direct3, native export/fresh-process generation6, terminal resume/renewal
rejection and production release build passed. Full release/S5/S6 remain unverified.
Detailed results, design, failures, limits and exact evidence paths are in
EXPERIMENT_STATUS.md; learning improved substantially but Goal1 remains incomplete.

Baseline5879a3c6642211e78a0d19a91679babba5f8a6c1; tracked clean at entry. The current
user authorizes H0 harness → H1 four boundary repairs → H2 frozen V1000 baseline →
H3 copy → H4 selection → H5 paraphrase → H6 normal QA/final S4 → H7 memory S5 → H8 INT4 S6.
Actual prerequisite PASS advances without another permission request. A failed skill gate,
exhausted budget, invalid data or cancellation preserves evidence and stops dependent stages.
Code verification, model quality and independent acceptance are separate.

H0–H2 SMALL updates0. H3–H6 each512 updates; a second512 is allowed only when dev primary
improves≥4/256 and all preservation guards hold. Stage caps1024updates/6Minput/1.5Mtarget/
60minutes; aggregate4096updates/24Minput/6Mtarget/180minutes including training/evaluation.
Commands≤900s plus≤120s cleanup, training16GiB/inference12GiB; one owned heavy process.
Keep tokenizer/SMALL/F32/backend/first-target weight and inherited Adam/cumulative clock.
New constant LR=min(V1000 actual next LR,3e-5), with no warmup/schedule rise or optimizer reset.
Materialized focus2048 plus original ordinary QA anchor2048, batch8/acc1=4+4 distinct bases;
without-replacement epoch/tape with exact index/RNG/pool/exposure records. No silent truncation.
Evaluate dev256/watch32/previous-skill32 at0/128/256/512 and conditionally768/1024.
Two consecutive watch drops≥3 or prior-skill drops≥2 stop; new UTF-8/control/empty stops.
New data have disjoint identifiers/bindings/base-scene lineage across train/dev/seal; dev/seal
each64bases×4views. Seal256 is opened once after development passes. Preserve original QA400
and require ordinary≥V1000 before promoting a specialised parent. No final-test selection.

H1: reject non-progressing/ambiguous/oversized literal replacement and malformed fact atoms;
use explicit exact-entity versus numeric-field ablation modes; account for every relevant
causal record before semantic acceptance; propagate arm incompleteness/provenance through
close and candidate gates. Reuse existing helpers/tests/RunControl and native formats.
H2: verify V1000 resume/inference content, original and binding corpus bytes, recount original
400 and replay fixed ordinary32; audit semantics and final-test exposure without retraining.
H3 gates dev/seal full-answer≥95%, entity/event-ID≥99%, malformed0 and original QA retention.
H4 adds support≥95% and whole-group≥90%; H5 adds group/new-expression-strata≥90%.
H6 retains normal ordinary≥95%, each category≥90%, invalid citation acceptance0, then
independent final≥200 with40/category and unchanged95%/90% gates. H7/H8 use fresh saved facts,
restart/fault paths, then grouped INT4 W4A16 and actual M4 measurements,≤90min each.
No product pointer promotion or independent acceptance is implied by these checks.

H0/H1: direct red4 → green boundary/integration checks; quick26 and two existing positive
CLI regressions pass, as do fmt/check/clippy/release. SMALL/TINY/scalar updates0.
H2: both full training corpora/validation audited; original400 re-counted175/336+64/64;
watch32=16/32 with zero raw/output differences (33calls including ABA). H3 train/dev/seal
materialized and independently checked before learning. Old final artifacts are exposure-unknown
or already inspected; preserve them as development history. Freeze one fresh independent200
only after H6 dev passes, exclude actual train/dev scenes/identifiers, never tune on that final.
H3 implementation uses the existing loss/Adam/native/RunControl with explicit constant LR,
a precomputed4+4/no-repeat tape, and artifact/source/clock-bound TIME_BUDGET-only resume.
The original execution closed at128updates (cumulative21878),308,182input/34,911target tokens,
wall256.88s, maximum RSS6,649,823,232B. Dev full-answer0→0/256, entity0→16/256,
event0→0/256, watch16→14/32. Invalid UTF-826→2 in total, but one case was newly invalid:
the registered QUALITY_GUARD stops after that evaluation and before any further update.
Candidate/resume are ineligible; a real resume request was rejected. No retry, second epoch,
seal score, candidate original400 evaluation or downstream learning is allowed under this
closed run. The remaining numerical budget does not override a stop condition.
H4–H8/S4–S6=NOT_RUN_PREREQUISITE, Goal1 ready/accepted=NO, independent review=PENDING.
Quick and the6-generation model/restart path are verified only in their executed scope;
full release and S5/S6-specific receipt verification remain pending those prerequisites.
Current: H3 CLOSED_BUDGET / QUALITY_FAIL / NOT_RUNNING at1024 under the explicit amendment
above; original128/512 stops and all failed artifacts remain preserved.
Historical plans/results below remain preserved and are not new execution evidence.

## S4 completion continuation V — authorized 2026-09-17

User again explicitly requires S4 quality and Goal1 completion. Baseline52aaccdab8153ea34d14db5026fad810f8dcdf26.
The former RF-only update cap and closed T run remain historical; authorization includes
new bounded learning, then S5/S6 in order, without lowering acceptance criteria. Do not
repeat tiny memorization, externalize inference, or alter the current model/storage formulas.

Observed before this learning: T500's original train prefix400 gives239/336 ordinary QA
and64/64auxiliary;97ordinary failures include85 first value errors. Train QA0/1/2 are37/68,
36/68,34/68, while single-record QA3 and causal QA4 are68/68 and64/64. On unchanged validation,
known-question-only ablation gives69/136 QA0/2 instead of35/136, including QA2 11→40/68.
These are diagnostics, not final test scores: both multi-record value binding and question
expression transfer remain. No new label contradiction or core numerical defect was found.

One learning-data intervention: full-population question/value pairs. Reuse each existing
four-case ordinary QA0/QA2 block, holding both records fixed for two different requested
targets, then swapping only the two source values. Derive training labels from those actual
records. Keep episode count/base scenes, all other categories/auxiliary cases, and validation
bytes unchanged. Preserve the original corpus and publish only the Rust transformer/tests.
This addresses absence of same-evidence/opposite-question supervision in ordinary QA without
the earlier U2 population reduction. It does not claim data design is the sole root cause.

Start V from preserved P1000 step20750, exactly as the historical T control. Same Adam/RNG,
batch8/group1/accumulation1/first-target8, CPU/Accelerate/F32 and tokenizer. Same explicit
LR0.00008650922011682288/warmup0/cosine2000 extension, with subsequent plain resumes. Only
the new declared train split changes; index draws and schedule remain comparable, while
actual samples/input-token counts necessarily differ. New source has the verified ordinary
trainer stop checks; they do not change finite uninterrupted optimizer arithmetic.

V cap2000updates/20Minput tokens, four≤500-update segments, each work≤900s plus preservation,
training RSS16GiB; one heavy process. Evaluate the original validation400 every500 and
compare to the existing T rows at matching steps. Stop for nonfinite/cancel/resource limits,
≥0.10 ordinary macro loss or≥2new generation errors at two evaluations, or three consecutive
evaluations without a new best. No automatic extension of a failed V. Use validation-only
selection; do not run final200 until a candidate passes the unchanged development gate.
Track actual train versus validation outcomes without calling ablations candidate scores.
Local evidence: artifacts/s4-completion-20260917. Original checkpoints/corpus/logs remain intact.

During V's second segment, the user asked whether repeated attempts were improving and
requested a direction if quality remains unchanged. V500 was165/336 versus T500169/336.
Decision made before observing V1000: finish/preserve/evaluate the current segment; if it
still fails the original development quality gate, do not launch V1500/V2000. The2000 cap
is an upper bound, not a reason to consume it. Reuse existing question/record ablation
evidence first, fill only missing no-update comparisons, then identify a specific boundary
before any next training. This is an early bounded closure, not a revised acceptance gate.

V observed closure:1000updates,2,330,384input/208,320target tokens, training wall1571.40s,
maximum RSS7,039,188,992bytes. V500 ordinary165/336; V1000 ordinary175/336 versus T1000
169/336 on identical400 inputs/targets. Auxiliary64/64 and generation failures0 in both V
evaluations. V1000 has a small development gain; QA2 remains10/68, so the original gate
fails. V1500/V2000 were not run under the early closure decision recorded above. Preserve
both native artifacts and raw failures; no final test, memorization retest or new optimizer run.

Missing no-update comparison: existing record-only ablation handles auxiliary value tasks,
not ordinary QA0/QA2. Extend that same evaluator/selector for these two categories, with
explicit oracle metadata and the existing shared cancellation/deadline. At frozen V1000,
reuse the original136 answers from validation400, then run known-question, record-only,
and their combination once each (408 additional generations maximum;900s/12GiB each).
Keep gold and selected record bytes fixed; selecting support also changes input length and
distractor count, so the difference is not proof of a sole attention cause. No training,
new scenes, final-test use, product selector, or quality promotion from oracle scores.
Verify CLI category filtering, exact original/modified request receipts, flag conflicts,
full entity/context matching and source/checkpoint preservation before the real comparison.

Observed no-update closure: ordinary QA0/QA2 original35/136; known-question64/136;
support-only86/136; both115/136. QA0 counts25/24/57/56, QA2 counts10/40/29/59, denominator68
each. All408 new generations completed without generation/UTF-8/empty errors, total47.18s.
Separate Rust row checks confirm identical original input/gold/weights and unchanged selected
record bytes. The remaining21 first errors are19entity/2format. Oracle intervention identifies
sensitivities, not a sole root cause or accepted model. No more learning is launched here.
Next direction: design for unseen identifier copying and question-conditioned record selection,
then language-expression transfer with independent train/evaluation bindings. Do not repeat
fixed tiny memorization, silently expand data, or substitute the oracle in product inference.
Any future learning must declare its one intervention, bounded comparison and stop conditions
before running. Goal1 acceptance thresholds remain unchanged and incomplete.

Current: ordinary trainer cancellation follow-through verified; C/L/B/P/T/V learning runs
remain CLOSED, NOT_RUNNING. S4 quality and Goal1 remain incomplete. T closed after three
consecutive evaluations without a new best and its2000-update cap. No automatic extension
of this negative closure.

## Ordinary trainer stop boundary — 2026-09-17

Baseline32a633200950c820c22d50508b65c814bbd2837c. On continuation, inspection of the actual
`train` caller found that RF-03's existing diagnostic RunControl was not used by ordinary
training validation/finalization. An already-set flag still allowed initial validation,
teacher validation had no intermediate cancellation/deadline check, and final validation
could run after cancellation. This is a control defect; no evidence links it to the prior
QA regression. Scope: reuse the same RunControl in this caller, preserve training arithmetic,
schedule/Adam/tokenizer/corpus/native schema and all original artifacts, no SMALL learning.

The CLI passes its actual Arc flag. A900-second command deadline covers load/preparation,
validation teacher calls, microbatches, the atomic optimizer boundary, checkpoint save and
terminal emission. The existing --no-rss opt-out remains explicit in the receipt; it does
not turn observation failure into a zero measurement. Stop skips subsequent validation and
updates, preserves the consistent native state, and reports the first reason independently
of cleanup/save errors. Ordinary successful resume retains exact weights/Adam/RNG/loss.

Track the step of the last completed validation so stale CE is not attached to newly updated
weights after interruption. Keep actual consumed input tokens, including completed gradient
work in an aborted accumulation, while weights/Adam/sampler remain at the completed update.
The initial proposed zero-consumption assertion was incorrect and withdrawn; the raw failed
test is preserved. This is not reported as an original token-accounting defect. Receipts show
executed inputs including uncommitted work, final-validation completeness, save/error details,
and work/cleanup/overrun time. Native status vocabulary is unchanged; cancellation observed
after publication is reported by TRAIN_CONTROL/exit status without rewriting that artifact.

Verification: pre-cancel/deadline, interrupted real TINY teacher, aborted accumulation budget
accounting, final-save/terminal cancellation, cleanup collision, normal completion/no-rss,
actual CLI SIGINT, and fresh-process exact resume. Test-only hooks reuse the existing private
control; no alternate model or scheduler. Recheck the existing RF/gold-independence/ledger/
tape-clock cases and pinned offline fmt/check/clippy/release. Only directly related tests run.
Original R4 audit/recount/66-generation parity are historical completed evidence, not rerun or
counted as new results. Preserve source/test logs under artifacts/training-stop-20260917.

S4 remains below the unchanged95%/90% gates. No new corpus, memorization retest, C/W repeat,
SMALL update, final heldout, S5 acceptance or S6 work is added by this control repair. Existing
training authorization remains recorded; the exhausted negative T run is not automatically
extended. Repair completion, data-audit scope, quality recovery and Goal1 are separate verdicts.

Observed closure:21distinct directly related tests passed, plus pinned offline fmt/check/
clippy/release. SMALL updates0; TINY35 including one failed-red optimizer update and two
16-update exact-resume/one-update SIGINT test executions. Original17files and P/T8native
checkpoints match their preserved SHA256 manifests; existing487untracked files remain.
No new source/dependency/fixture file. Source manifest digest:
4983c44575e8113b9848bee1566193648e35ca2a2a8adb07c0147bcc63b58503.

## Renewed Goal 1 continuation — authorized 2026-09-17

User explicitly renewed the training restriction and requested continuation through S4
and Goal1. Baseline `52e8b88cd38e88b4ba563a27d47cfe0bc9543f83`; the closed zero-update repair
below remains historical. No original artifact/data/raw log is replaced. Rust-only,
own-model, no external teacher/API, actual measurement and unchanged quality thresholds
remain required. S4 quality, S5 new-memory/restart and S6 INT4 must pass in order.

First experiment, registered before execution: schedule/LR policy C versus L, starting
from the same frozen U2 start (same general parent weights/Adam/step19750/sampler). Keep
corpus, full250-draw tape, batch8/group8/accumulation1, first-target weight8, tokenizer,
architecture and backend unchanged. C retains original warmup100/peak0.0003/cosine1000
schedule. L starts at the parent step19750 endpoint LR derived from its saved config
(approximately0.00008650922), removes warmup and uses the same1000-update cosine horizon.
This is one LR-policy intervention; no change to Adam arithmetic or optimizer clock.
The saved native TrainConfig fully expresses either schedule without schema changes.

Each arm: at most250 new updates, input1,000,000/target250,000 tokens, work900s, RSS16GiB;
total at most500SMALL updates. Evaluate fixed watch32/train16 at0/10/25/50/100/150/200/250.
Stop on nonfinite/cancel/resource/time/token limits, or parent-relative watch loss≥4EM
or ≥2new generation errors at two consecutive evaluations. A stopped arm is not a
completed250-update result. Existing20-step control identity is checked, with no extra
training. Preserve every result, including failures. No C/W repeat, new corpus, tiny
memorization, final heldout or INT4 implementation during this causal comparison.

Evidence for LR sensitivity requires C deterioration and better L scores at matching
exposure/steps, with unchanged tape/weights-at-start/moments/clock. Failure to establish
this stays UNRESOLVED; no automatic repeat/sweep or promotion to a quality claim.
The next bounded action will be chosen from observed failures and recorded before it
runs. Later training runs retain the original≤5000step/≤20Mtoken limits and explicit
stopping criteria. Final heldout remains unopened until a validation-selected candidate
is ready. Passing a diagnosis does not waive overall95%/each-category90%/invalid-citation0.

Local evidence root: artifacts/goal1-renewed-20260917. Existing recovery arm/trainer/native
checkpoint/evaluation helpers are reused; no new training framework. C/W defaults remain
50steps; explicit --max-updates is bounded to250 and the old W arm remains limited to50.
Actual results and source identities are recorded in EXPERIMENT_STATUS.md after execution.

Observed closure: C stopped at200updates under QUALITY_GUARD (watch11/32), L completed250
(watch15/32). Same200update comparison was C11/32 versus L14/32; train16 C8/16 versus
L11/16. Actual policies/tapes/clocks/LR and row recount verified. New SMALL updates450,
numeric/TINY optimizer0. Source/data/code checks passed; quality recovery NOT_ESTABLISHED.
C/L is closed without an automatic extension, and all original evidence remains intact.

Next registered experiment B: the original general-QA parent used sample_group_size1,
whereas U2 changed to8 and each batch repeated related views of one scene. C/L alone
did not recover parent quality; C's largest observed field loss was entity accuracy
(21/26→14/26), and L still ended below baseline. Test the sampling policy as the next
single intervention, using the same initial parent/Adam/corpus and L's LR policy, but
sample_group_size1 instead of8. Keep batch8/accumulation1/first-target8 and every other
TrainConfig field unchanged. The prior L control is reused, not trained again.

B cap:250additional updates, input1M/target250k, work900s, RSS16GiB. Same watch32 and the
same predeclared train16, evaluations0/10/25/50/100/150/200/250, same quality/nonfinite/
resource/cancel stop criteria. No corpus/architecture/tokenizer/Adam/schema changes.
Changing the sampler necessarily changes the actual tape and RNG advancement, so
this is a sampling-policy comparison, not a matched-multiset batch-reordering claim.
Replay each recorded sampler/tape against its own policy; verify matching initial raw
watch outputs and identical LR/optimizer clocks at common steps. Source/binary IDs
differ because B support was added after L; retain both identities and original logs.
Train16 remains fixed from the original group8 selection, with actual B exposures counted.
No further extension of a failing B arm; any subsequent action requires new observed
evidence and its own recorded budget. No final heldout until validation quality is ready.

B observed closure:250updates, watch15/32 and train16 10/16; final L was15/32 and11/16.
Actual sampler/tape/clock/LR, same initial watch raw generation and fixed train16 verified.
B used613827input/58199target tokens,395.09s wall, maximum RSS6660702208bytes. B is not
extended and no quality recovery is claimed. Renewed SMALL total700updates so far.
Direct6tests/fmt/check/clippy/release passed. The next learning plan will keep the original
parent corpus rather than the narrowed U2 distribution. Ordinary QA bytes already existed
in v8, so recent379updates alone do not establish insufficient lifetime training exposure.

Next registered learning stage P (stable original corpus): restore the preserved general
QA parent as the starting artifact, without changing that artifact or deploying it.
Use artifacts/goal1-corpus-v9 unchanged (train20000, ordinary16668; validation400,
ordinary336/aux64). This tests learning on the original distribution after C/L/B on U2
failed to recover quality; it does not assume lifetime underexposure or a confirmed bug.

P cap:1000new SMALL updates and20Minput tokens total, four segments of at most250updates.
Retain Adam, initial sampler, batch8/group1/accumulation1/first-target8, tokenizer and
architecture. Explicit continuation LR0.00008650922011682288, warmup0, cosine1000;
subsequent segments are plain exact resume with no schedule reset. The existing native
TrainConfig stores this policy. CLI --extend-lr/--extend-warmup applies only at the first
explicit extension; ordinary resume keeps its saved policy. No new corpus/heldout is made.

At baseline and after each250, generate existing validation400 once and score ordinary
QA separately from auxiliary tasks. Select by ordinary five-category macro, then ordinary
EM, then earlier step. A run is stopped for nonfinite/RSS16GiB/cancel or token/update cap;
each segment is monitored with a900s wall budget and an owned-process SIGINT at that
boundary, permitting consistent checkpoint cleanup. No hard-preemption claim. Stop if
ordinary macro drops≥0.10 or new generation errors≥2 on two consecutive evaluations;
also stop after three consecutive evaluations without improvement. Do not extend a
negative result automatically. Maximum training wall budget3600s plus measured cleanup
and evaluations; inference commands retain their900s/12GiB bounds.

S4 final evaluation remains NOT_RUN until a validation-selected candidate reaches at
least95% ordinary QA overall,90% per category and zero invalid-citation acceptance.
Existing development400 remains development, and the final200 constructor is not used
to generate training data. New direct native resume test executes16TINY optimizer steps
(full6 + split3/3 + extension4); these are separate from SMALL training totals.

P observed closure:1000updates completed with all four native saves and five fresh-process
validation400 evaluations. Ordinary QA155→162→150→162→164 of336; auxiliary64/64 at every
evaluation. Macro/EM selectsP1000. Generation errors0 throughout; this remains below S4.
Input2323914/target205719tokens, training wall1461.87s, maximum RSS6930546688bytes; all
registered limits respected. Original17file hashes and executed source/binary remain equal.
Renewed totals1700SMALL and16TINY optimizer updates. No corpus/architecture/storage change.

Next registered learning stage T: P improved ordinary validation by9answers, while the
remaining errors are concentrated in question/evidence selection. This observation supports
a bounded continuation from the selectedP1000, not a claim that an implementation defect
was identified. Preserve the same original corpus and every model/Adam/sampler/objective
setting. Extend by at most2000SMALL updates/20Minput tokens, four500-update segments;
first extension explicitly uses LR0.00008650922011682288/warmup0/cosine2000. This raises
the endpoint LR at a declared new learning boundary; subsequent500 segments plain-resume
that schedule without resets. It is continued learning, not a matched-step causal comparison.

T starts at model step20750. Check existing validation400 after each500, baseline reused
fromP1000 with identical source/binary/input. Select ordinary five-category macro, then EM,
then earlier step; keep every candidate. Stop after3consecutive evaluations without a new
best, or macro drop≥0.10/new generation errors≥2 relative to baseline on two consecutive
evaluations. Each training segment≤900s plus consistent cleanup, total≤3600s; inference
900s/12GiB, training16GiB, nonfinite/cancel stop immediately at existing safe boundaries.
No sweep, new corpus, memorization retest, storage/kernel/topology change or final-heldout
selection. Negative T is not automatically extended. S4 eligibility/quality gates are unchanged.

T observed closure:2000updates/input4645903/target412926tokens, training wall2773.29s and
maximum RSS6952665088bytes. Four500-update native saves and fresh-process400 evaluations
completed without generation/UTF-8/empty errors. Ordinary QA164→169→169→167→166 of336;
auxiliary64/64 throughout. Macro selectsT500 (T1000 has equal EM but a lower macro).
The last three evaluations did not beat the best, so the predeclared plateau stop applies;
native terminal is BUDGET_REACHED at step22750. No additional run is scheduled.

Renewed total3700SMALL updates (C200+L250+B250+P1000+T2000), plus16TINY exact-resume
test updates. Best developer candidate T500 step21250 has169/336 versus original155/336,
but four of five categories remain below90%; S4 overall95% is not met. Final200 remains
NOT_RUN_NOT_ELIGIBLE, S5/S6 prerequisites unmet, Goal1 not complete. The observed modest
development improvement does not establish the cause of the old U2 regression or new-fact
transfer. Preserve every original/checkpoint/log; no source/corpus/storage change during T.
Executed29source-manifest entries, binary and17original hashes still match. Full evidence,
physical artifact IDs and separate verdicts are recorded in EXPERIMENT_STATUS.md.

## R3-S4-DIAGNOSTIC-REPAIR-1.0 — closed repair round

2026-09-17, baseline main `3b671bf695ae86511273c4139d43d75bd976e490`.
Origin `https://github.com/seoyd/replica-v3.git`. Tracked tree initially clean; 487 older
untracked logs preserved. Rust/Cargo1.98.0, macOS27/M4/24GiB, CPU/Accelerate/F32.
Shell thread variables were unset; executions explicitly set both VECLIB_MAXIMUM_THREADS
and RAYON_NUM_THREADS to1. No project training/worker process was running.

Independent source review identified RF-01 split binding, RF-02 target semantics and
RF-03 evaluation cancellation/deadline defects. Baseline independent CODE_VERDICT=FAIL;
the older CHECKED_BOUNDARIES_PASS below is a historical implementer scope verdict.
Source repair is separate from model quality. SMALL new optimizer updates=0. No C/W
training, final heldout, data expansion, model/optimizer/tokenizer/kernel/storage changes.

R0 preserve identities → R1 frozen/current split binding + red/green → R2 limited target
grammar + red/green → R3 shared cooperative budget + red/green → R4 read-only data audit,
historical recount, limited native parity → R5 direct checks, report, commit/push.
All named artifacts from the previous round are AVAILABLE. Original byte hashes and
initial untracked inventory are in local artifacts/diagnostic-repair-20260917/.

R1: all validates the loader-owned snapshot before model/output access; panels use their
frozen episodes. Ordered IDs are not a content hash. Hash ordered serialized Episode
content with SHA256 over compact serde_json::to_vec, including request/limits/answer.
R2: derive obligations from question/evidence/status/time only, parse target claims, then
check facts/citation/order/uncertainty. Unknown forms remain UNSUPPORTED_FORM; no corpus
edits or new out-of-scope exclusions. Audit U2 train2048, parent first2048 and validation400
(ordinary336/auxiliary64); existing copy/* auxiliary semantics remain OUT_OF_SCOPE.
R3: one command flag/deadline, stop before/after large synchronous boundaries and before
teacher/next case/update. Record partial counts and first observed stop. Native weights
retain atomic optimizer boundaries. Cleanup is separate; synchronous compute/fsync can
overrun a cooperative deadline. No hard-kill claim or new scheduler.

Real SMALL generation cap70 calls (parent/failed watch32 plus one ABA each; optional
two fixed failures per model); command work cap900s, aggregate native verification1800s.
No full400/C/W generation reruns. Related deterministic tests only; TINY/scalar optimizer
calls counted separately. Source/artifacts/raw logs remain preserved; only changed Rust,
small synthetic tests and anonymous status docs are published after verification.

### Closed verification and requirement cross-check

RESULT=VERIFIED_REPAIR, REPAIR_IMPLEMENTER_VERDICT=PASS, new INDEPENDENT_REVIEW=PENDING.
RF01_SPLIT_BINDING/PASS, RF02_TARGET_SEMANTICS/PASS, RF03_EVAL_STOP/PASS. These are
implementation boundary results; REGRESSION_RECOVERED=NO, TRANSFER_IMPROVED=NOT_TESTED,
S4_QUALITY_PASS=NO, GOAL1_READY=NO and NEXT_TRAINING_AUTHORIZED=NO remain separate.

| Node | READ/SOURCE | IMPLEMENTED | EXECUTED / OBSERVED | BOUNDARY / LIMITATION |
|---|---|---|---|---|
| R0 | Baseline/source manifest, existing checkpoint/corpus/raw records available | Preservation manifest and untracked inventory, local only | Original17file start/end hashes equal; Rust1.98/M4 observed | No reset, model replacement, training or toolchain change |
| R1 | Existing replay/data loader and panels | Owned all snapshot, pre-model hash validation, explicit panel dispatch, complete-content digest | Baseline red; 3 direct regressions pass, including unchanged positives/0calls/old-output preservation | Legacy split_hash means source split, not subset content; no checkpoint identity change |
| R2 | Existing request/support/scan and corpus grammar | Separate request obligation, whole target grammar and semantic decision; scan→audit statuses | Baseline red; independent positive/negative/unknown fixtures and existing richer-causal regression pass | Training-only limited grammar, no answer-fed support selection or product oracle |
| R3 | evaluate_one/teacher/replay/close/arm/training caller | Shared cooperative control, remaining timeout, partial counts, latched stop, final preservation/terminal checks | Baseline-body pre-cancel red; 5 new tests pass, including real random TINY generation and final-state interruption without updates | Synchronous tensor/fsync/worker IPC can overrun; no hard-kill guarantee |
| R4 | Same frozen U2/parent/validation, original400 and C/W traces | Existing audit extended; small Rust recount command; optional replay reference check | Data scope4092/4496 validated; existing400/C/W recount PASS; parent/failed66native calls, raw parity differences0 | No additional failures/C-W/full400 generation; auxiliary404 remains outside semantic scope |
| R5 | Final source diff and all contract requirements | Existing plan/status updated with evidence and separate verdicts | 18 distinct related tests, fmt/check/clippy/release PASS; source normal push and remote SHA verified | Independent review pending; following documentation-only commit records publication |

RF-01 content encoding is SHA256 over compact serde_json::to_vec of the ordered complete
Vec<Episode>. IDs alone are not content identity. All records retain expected/source hashes
separately; panel evaluations use only frozen episodes. Same-ID question/evidence/answer
changes, count changes, and invalid manifest fail before a model call or output mutation.

RF-02 validates structured entity/context/value, no-evidence uncertainty, unknown-cause
accident claims and requested chronology including order/uncertainty. Contradiction,
unsupported form, ambiguous evidence and predeclared auxiliary scope are separate states.
The initial real audit exposed two unsupported existing source forms (movement-value
question and completed-inspection/no-accident record); only these forms and independent
positive fixtures were added. Initial incomplete logs are preserved. No labels, source
records, episode IDs or exclusion rules were changed to obtain the final audit result.

| Final audit scope | scanned | ordinary | validated | contradicted | unsupported | ambiguous | out-of-scope |
|---|---:|---:|---:|---:|---:|---:|---:|
| U2 all train | 2048 | 2048 | 2048 | 0 | 0 | 0 | 0 |
| Parent first2048 | 2048 | 1708 | 1708 | 0 | 0 | 0 | 340 |
| Validation400 | 400 | 336 | 336 | 0 | 0 | 0 | 64 |
| Total | 4496 | 4092 | 4092 | 0 | 0 | 0 | 404 |

DATA_AUDIT=CHECKED_BOUNDARIES_PASS at this scope, not a full-parent/auxiliary audit.
Prefix/collision/checked entity overlap findings=0. ROOT_CAUSE_EVIDENCE=UNRESOLVED.
Historical scores are recalculated from original rows including errors: parent155/336+
16/64=171/400; failed56/336+0/64=56/400; previous C/W each17/32. Missing old error receipts30
remain UNKNOWN. HISTORICAL_LEDGER_RECOUNT=PASS is not new400 generation or new C/W work.

RF-03 simultaneous observation priority is cancel, deadline, RSS observation failure,
then RSS limit. The first cause is retained even if cleanup fails; observed conditions
are also recorded. Generation/teacher/next-case/ABA/update stop together. Native optimizer
updates remain atomic, and preservation only saves the last consistent state. The final
sidecar distinguishes checkpoint save status/reason from a stop observed after saving.
Work time, cleanup time and deadline overrun are reported separately. The terminal check
is the completion decision; signals after it do not retroactively change that decision.

Actual native parity: fixed parent32+ABA1 and failed32+ABA1, total66generation calls and
10.97s OS wall time, no timeout/interruption. Parent18/32, failed5/32, same raw tokens/text/
errors/prompt/provided/excluded, both ABA equal. Parent/failed max RSS769654784/787087360
bytes. Original requests, tokenizer, weights and CPU/Accelerate/F32/two thread limits1
were unchanged. NATIVE_REPLAY_PARITY=PASS; no claim of recovered intelligence follows.

SMALL_OPTIMIZER_UPDATES_THIS_ROUND=0 and NUMERIC_TEST_OPTIMIZER_UPDATES=0. Related tests
used TINY forward/generation without training. Scalar Adam, accumulated-gradient and
exact-resume training tests were intentionally not run. Historical100C/W updates are not
this round's work. No final heldout, contrast16/QA32 memorization, corpus expansion or
new numeric/kernel experiments were performed.

Local evidence root: artifacts/diagnostic-repair-20260917. Red proofs are rf01-red.txt,
rf02-red.txt, rf03-red.txt. RF-03 red used a test adapter into the unchanged baseline
evaluation body because that API originally had no budget parameter. The accidental
initial RF-01 zero-test filter is excluded. Final10new+7existing+1decode tests passed;
offline/locked pinned-feature fmt/check/clippy/release also passed. R4 evidence is
data-audit-final.json, historical-recount.json and parent-watch/failed-watch.jsonl/.txt.

Executed parity source-manifest SHA256:
`cb930b1a7a69010eeea032aa6e0d7e9db25a0faebdb7607e40984657393220ed`.
Final delivery source-manifest SHA256:
`aa978d4d9b74c4ad7491001c74df8bafd3ff05a81b10fdcacb53f155363fafee`.
Manifests hash sorted Rust sources/tests/examples plus Cargo.toml/Cargo.lock and are
distinct from commit SHAs. After actual parity, the normal terminal reason was made
explicit (COMPLETED instead of null), with zero-call/normal-terminal assertions added.
Final direct checks cover this bookkeeping change; SMALL parity was not repeated.
The original66call logs retain their executed null/no-stop terminal metadata unchanged.
Full source/binary/artifact identities and actual paths are in EXPERIMENT_STATUS.md.

Published review source commit: `e4e08fc0b8e43071437b40082fb098d4df815878`.
Normal push to origin/main succeeded; git ls-remote returned that exact full SHA.
The publication receipt is a following documentation-only commit, whose final SHA is
reported separately after push; there are no source edits after the reviewed source commit.

Only next hypothesis proposed: retained-Adam schedule restart/LR effect. NOT_RUN and no
new training authorization. A separately approved C/L comparison would retain the same
general-QA parent weights/moments/optimizer clock/tape/batch/first-target weight, vary only
the schedule/LR policy and explicitly budget the post-warmup100 through historical250
range. Nonfinite/resource/time/token conditions stop immediately; watch loss of at least
4EM or at least2new errors on two consecutive evaluations stops the arm. This is not an
automatic extension or another default50update run. C/W's negative result does not rule
out LR, batch or longer-range causes. The present repair closes without that experiment.

## Previous R3-S4-QUALITY-RECOVERY-1.0 (historical)

2026-09-17. 기준 HEAD `9fb5059696553b41d9a3190856888a929d0931be`, origin
`https://github.com/seoyd/replica-v3.git`, main. 시작 tracked dirty 없음.
Rust/Cargo1.98.0, macOS27 arm64, Apple M4/24GiB. CPU/Accelerate F32,
VECLIB_MAXIMUM_THREADS=1/RAYON_NUM_THREADS=1. 시작 시 프로젝트 학습/추론/빌드 없음.
기존 SMALL/801BPE, native artifact/SQLite format, 기본 decoding은 고정한다.
외부 모델/teacher/API, gold 기반 제품 선택, lossy decode, EOS 억제, 자료 확대는 금지한다.

## 순서와 종료 기준

Q0 실제 기준점/원본 동결 → Q1 무학습 replay/오류 보존 → Q2 데이터·prompt·수치 경계 →
결함이면 Q3A 최소 수정/causal replay, 아니면 Q3B 한 요인 제한 대조 → Q4 재검증/닫기.
독립 검토는 PENDING이며 이 라운드 종료는 Goal1 완료가 아니다.
품질 기준은 기존 독립200개/5분류40개씩/전체95%/각90%/잘못된 인용 승인0을 유지한다.
개발 근거가 부족하면 final은 NOT_RUN_NOT_ELIGIBLE. 제품 모델 포인터를 바꾸지 않는다.

명시 로컬 자료:

- parent 원본: artifacts/goal1-resume-20260917/u2-parent-resume.r3m
- 동일 가중치·U2 정책 시작 상태: artifacts/goal1-resume-20260917/u2-probe/start
- 실패: artifacts/goal1-resume-20260917/u2-to-20000/final
- +20: artifacts/goal1-resume-20260917/u2-probe/final
- parent corpus: artifacts/goal1-corpus-v9; U2: artifacts/goal1-resume-20260917/u2-corpus-fixed
- 기존400 실제 로그: 같은 디렉터리의 u2-start-validation.jsonl/u2-20000-validation.jsonl
- 새 산출물: artifacts/quality-recovery-20260917 (기존 파일 create_new, 덮어쓰기 없음)

원본 frozen 실행 파일은 baseline-replica-train으로 복사했다. 파일/hash/metadata는 실제
Rust native loader로 registry에 기록한다. physical/tensor/model-content/tokenizer wire/
semantic ID를 구분한다. 과거 기록은 REPOSITORY_REPORT, 이번 실제 읽기/실행과 구별한다.

## 실행 예산 (실행 전에 고정)

무학습0updates. Q1 watch32는 category별7/7/6/6/6, 각 category에서 base scene이
중복되지 않는 첫 항목을 metadata 순서로 선택한다. 실패 panel≤16은 기존 실패 로그에서
UTF8/빈응답/값·숫자/인용 오류 유형을 순회해 고정한다. 점수를 보고 panel을 교체하지 않는다.
기존400 로그를 검산하며 신뢰할 수 있으면 전체 replay 대신 panel만 실행한다.
각 generation은 원래 request의 고정 max_tokens/timeout/context를 사용한다.
무학습 CLI command 상한15분, 라운드 실제 실행 합산 상한90분, inference12GiB/training16GiB.
일반 검사 abs1e-4+rel1e-3, 사후 tolerance 확대 없음. nonfinite/data ambiguity/parity 실패는
학습 실험 시작 전에 중단하고 관련 경계만 수정한다.
실제 batch의 선택2좌표 central difference는 F32 차분 cancellation을 감안해 사전
step0.002, abs0.002+rel0.05로 고정한다. optimizer update 없이 원래 좌표 복원/hash 대조.

결함이 확정되지 않으면 근거 있는 최대3개 개입+control, 각≤50updates. 평가0/10/25/50,
watch32와 사전 고정 노출 표본만 사용한다. 1arm당 입력≤200,000/target≤50,000,
운영≤15분. 이는 기존250updates/약284s를 근거로 잡은 상한이며 완료시간 약속이 아니다.
watch에서 parent보다 정답4개 이상 감소 또는 새 UTF8/control/empty 오류2개 이상 상태가
연속 두 평가면 해당 arm 종료. nonfinite/시간/메모리/token cap은 즉시 boundary 종료.
방향·안전성 근거가 있을 때만 후보1개 confirmation2회, 사전 별도 tape, 각≤100updates/
입력400,000/target100,000/20분. 총≤400updates, 숨은 probe/수정 전후 update도 포함한다.
50updates에서 control 하락이 재현되지 않으면 자동 연장하지 않고 CLOSED_NEGATIVE.

## 가설 ledger (결과 관측 전)

| ID | 근거/예측 | 반증·최소 확인 | 허용 변경/현재 판정 |
|---|---|---|---|
| H-RECEIPT | evaluate의 decode 오류 가지가 Generated를 버림 | malformed byte fixture에서 IDs/종료 보존 검사 | 평가 receipt 보강; 품질 하락 원인과 별개 |
| H-DATA | answer로 support를 찾는 U2 생성 경로 | 질문/원문/status만으로 독립 검산, 동일 token prompt/다른 target 탐지 | 재현 결함만 수정; UNRESOLVED |
| H-PROMPT | train은 answer 길이, inference는 고정 예약 | 실제 train/generate IDs·제외 근거 대조 | 실제 불일치만 수정; UNRESOLVED |
| H-NUMERIC | 생성/cache/batch 경계 가능성 | 동일 prefix full/chunk/cache/batch/causal; 기존 독립 loss/Adam reference | 실제 위반만 수정; UNRESOLVED |
| H-LR | extension은 moments 유지, warmup/cosine 재시작 | 세 clock 및 실제LR/update trace, C 대비 L | Q2 후 arm 선택; UNRESOLVED |
| H-BATCH | 8개 같은 scene view 집중 | 같은 tape multiset, batch 경계만 교체 C/B | Q2 후 선택; UNRESOLVED |
| H-FIRST | 첫 target 가중치8 | 첫 token 분포/실제 objective, C/W | Q2 후 선택; UNRESOLVED |

D(기존 QA 혼합)/O(moment+optimizer clock 초기화)는 먼저 구현하지 않는다. Q2 근거가
위 후보보다 강할 때만 선택 사유를 고정한다. O는 clock 분리가 필요하며 기존 resume와
동일하다고 부르지 않는다. 이미 통과한 random QA32/contrast16 암기를 반복하지 않는다.

## 파일 경계와 검증

기존 training 평가/trainer/batch/Adam/native 모델을 재사용한다. 진단 orchestration은
training 전용 모듈1개로 격리한다. 기존 training.rs는 이미 평가·훈련·회귀를 포함하므로
registry/panel/audit를 제품 library에 넣거나 새 framework로 만들지 않는다.
새 영구 문서는 이 계획1개만 만들고 결과는 EXPERIMENT_STATUS.md에 기록한다.
실제 생성 오류 보존/ledger 분모/독립 support/packing/관련 numeric를 직접 검사한다.
최종 fmt/check/clippy/release 및 해당 회귀만 실행한다. 학습 실행 동안 소스를 동결한다.
공개 소스/테스트/익명 집계만 단계별 명시 commit/push, 원격 full SHA 대조.
원본 corpus/DB/model/Adam/raw log/임시 입력/target은 로컬 보존한다.

## Q0~Q2 관측과 Q3 선택 (학습 전)

원본 log336+64 집계와 artifact hash가 일치했다. 고정 watch32 parent18/32→실패5/32,
실패선정16 parent12/16→실패0/16; 각각 기존 출력 차이0, A→B→A 동일. UTF8 실패표본4개는
모두 invalid_sequence이며 length2/EOS2다. 같은 가중치를 쓰는 계측 수정으로 품질 회복은 없다.
초기 독립 validator가 인과 불확실성 답변의 단일 사건 인용과 상세 chronology 인용을 혼동해
U2 64/parent bounded120개를 표시했다. 원문/시각을 확인해 둘 다 뒷받침되는 응답 형식임을
분리하고 독립 합성 회귀를 추가했다. corpus를 고치거나 해당 항목을 제외하지 않았다.
U2 전2,048과 parent 앞2,048에서 token prompt 모순/prefix 불일치0; 부모의 보조340개는
의미 유일성 검사 밖이라고 별도 계상했다. U2의1,776prompt는256을 넘으므로 짧다는 추측을
하지 않는다. 실제 두 checkpoint의34개 수치 비교와 실제8행 loss/68gradient/2좌표 차분은 통과.

Q3는 C와 W 두 arm만 선택한다. 근거: 새 first-EOS12건과 학습 first-target8배 목적이
실제 존재한다. first token은 대상 종류 등 공통 문두이므로 기록 선택을 뜻하지 않는다.
W는 이 첫 토큰 가중치만8→1로 바꿔 빈 응답/후속 생성 하락과의 관련성을 반증 가능한
조건으로 확인한다. 일반 batch에서 가중 항의 영향이 작았으므로 큰 개선을 예상하지 않는다.
C/W 각각50updates, 동일 parent weights/Adam/optimizer clock/schedule/tape/자료/CPU/F32.
plan의 원래 cap을 그대로 적용한다. C20의 model hash가 원래 U2+20과 같은지도 확인한다.
추가 optimizer reset, LR/seed sweep, B/D 자료 재배열/확대는 실행하지 않는다.
L 미선택: parent LR0.0000865092, U2 +1/.000003→+50/.00015→+100/.0003의 재시작과
실제 log 일치는 확인했지만 아직 인과 증거가 아니다. 이번의 직접적인 first-EOS 경계부터
한 요인만 시험한다. LR 일정 continuation은 필요시 다음 한 개의 실험 후보로 남긴다.
첫 screening이 원인을 지지하지 않으면 자동 extension 없이 닫는다. 확인 실험은 아직
선택/실행하지 않았으며 총 업데이트 예산은 늘리지 않는다.

## Q4 종료 — 실제 결과 및 요구사항 대조

현재 NODE=Q4, STATUS=CLOSED_NEGATIVE / NOT_RUNNING. 제한 진단은 종료했고,
모델 회복의 원인은 UNRESOLVED다. 상세 수치와 판정은
[실험 상태](EXPERIMENT_STATUS.md)의 첫 보고에 기록한다.

| 경계 | 실제 구현/검증 | 판정 및 제한 |
|---|---|---|
| Q0 원본/계보 | 명시된 parent/start/+20/+250 native loader registry, physical/content/tokenizer/Adam/RNG/clock, source 및 binary hash | 기존 artifact 보존; 다른 run으로 대체하지 않음 |
| Q1 분모와 replay | 기존400 raw ledger 검산, 고정watch32 및 실패16, A→B→A, gold와 독립 생성 | 기존 출력 차이0; 실패선정16은 품질 추정 표본이 아님 |
| Q1 오류 receipt | 실제 generation loop observer, strict decode 실패에도 ID/bytes/EOS/finish 보존 | 독립 failing test 수정 전 실패/수정 후 통과; 학습 원인과 별개 |
| Q2 자료·입력 | U2전2048/parent앞2048, 질문/원문/status/time만으로 support, 역순·citation·값·prepared prefix·split 검사 | 확인 범위 PASS; parent 보조340 의미 검산 밖 |
| Q2 분포 | base scene/질문형/값 multiset/ID순서/token prompt 각각 집계 | U2는256 base scenes, 142숫자정규화 질문형, 58값 multiset, 1121 ID순서, 1894 token prompts |
| Q2 수치 | parent/failed 각각34개 full/cache/chunk/padding/causal 대조, 실제8행 shift/mask/EOS/68gradient 및2좌표 차분 | 사전 tolerance 내; 전체 연산의 무결성을 증명했다고 확대하지 않음 |
| Q2 optimizer | 독립 f64 Adam3step, 서로 다른 target 수 accumulation, 실제 parameter group norms | moment/clock 유지, schedule 재시작과 실제LR 일치; LR 인과성 미확정 |
| Q3 C/W | 같은 parent/tape/moments/LR, first-target 가중치만8→1, 각50updates | 모두watch18→17→16→17/32; C20은 과거+20과 model hash 정확히 일치 |
| Q4 재시작/제품 연결 | C/W native fresh load32개씩, worker4개 성공/오류 대조, legacy/native tokenizer byte mapping | raw IDs/문장/오류/prompt parity 통과; 제품 기본 모델 승격 없음 |
| Q4 후보/전이/최종 | 후보 적격 없음, confirmation/후보400/새전이/final200 미실행 | NOT_RUN_NOT_ELIGIBLE; S4/S5/S6/Goal1 합격 아님 |
| 종료 검증 | 직접 회귀13개, fmt/check/clippy/release, 원문·실패 로그 유지 | 독립 검토 PENDING; 공개 범위는 Rust 및 상태 문서 |

SMALL 실제 추가 업데이트는 C50+W50=100, 총input244,760/target22,600이다.
각각 같은19,750에서19,800으로 분기했으며19,850까지 연속 학습한 것이 아니다.
기존 exact-resume 회귀의 TINY12updates와 독립 scalar Adam3step을5회 실행한15회는
별도 테스트 계산으로 기록한다. 이를 합쳐도 총127updates로400 상한 이내다.
최초 C 시도는 native status vocabulary 검증에서0updates로 중단했다. 상세 종료 이유는
sidecar에 두고 기존 native status만 사용하도록 고쳤으며, 실패 디렉터리도 보존했다.
실제 C/W 프로세스74.63/75.50초, max RSS5.38/5.69GiB로 사전 상한 이내였다.

H-RECEIPT만 CONFIRMED_IMPLEMENTATION_DEFECT다. H-DATA/H-PROMPT/H-NUMERIC은
검사 범위 위반 없음, H-FIRST는 NOT_SUPPORTED_WITHIN_BUDGET이다.
H-LR/H-BATCH 및 학습 하락의 근본 원인은 UNRESOLVED다. C의50updates에서 원래
250update 붕괴가 재현되지 않아 REGRESSION_NOT_REPRODUCED_WITHIN_BUDGET으로 닫는다.
50updates 결과로250updates 현상을 반증하지 않는다. 확인 실험은 실행하지 않았다.

다음 한 개의 실험 제안은 C 대비 부모 마지막 LR을 유지하는 L이다. 기존 schedule의
warmup100을 실제로 지나는 범위를 별도 예산으로 먼저 고정해야 한다. 같은 parent,
moments/optimizer clock, tape/목적함수로 LR만 대조한다. 이는 다음 라운드 제안이며
이번 라운드에서 자동 연장하거나 실행하지 않는다.
