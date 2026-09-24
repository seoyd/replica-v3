# Teacher device closure — independent review

Date: 2026-09-24. Contract: R3-TEACHER-DEVICE-CLOSURE-1.0.

**Independent diagnostic closure PASS.** Actual Metal teacher192 completed,
independent teacher24 matched, and both64-row output reproductions matched.
This accepts the completed diagnosis; model quality remains FAIL, Muon adoption
NO and Goal1 NOT_ACCEPTED. The old training and diagnosis failures are unchanged.

| Acceptance layer | Current verdict |
| --- | --- |
| A_DELTA | PASS |
| D2_IMPORTED_INTEGRITY | VERIFIED |
| ACTUAL_TEACHER_CALLER | PASS |
| TEACHER192_COMPLETE / TEACHER24_PARITY | PASS192/192 / PASS24/24 |
| REPRODUCTION_A / REPRODUCTION_M | PASS64/64 / PASS64/64 |
| NEW_CALLS_WITHIN_BUDGET | PASS128 generation /216 teacher; caps fully spent |
| ORIGINALS_PRESERVED | PASS for1,856 scoped references |
| SUCCESSOR_DIAGNOSIS | PASS |
| ORIGINAL_TRAINING / PREDECESSOR_DIAGNOSIS | FAILED_UNCHANGED / FAILED_UNCHANGED |
| MODEL_QUALITY / MUON_ADOPTION | FAIL / NO |
| S4 / S5 / S6 / GOAL1 | NOT_ACCEPTED |

## Source and policy

- Reviewed source: `1344ae3b4e2f359db81d889e1c2c24b2356e7b17`.
- Comparison baseline: `f5895057ba40e1758c72bf7dba5e37fef21d6445`.
- Old D2/D3 producer: `4850195a1b5cb1d51f7f49fccae14ed194243f56`.
- Frozen executor SHA256:
  `c6b89acddb2587abbef941ddc40813362330866e3a719bd84faa7f64a3c6907d`.
- Test executable SHA256:
  `e8d147b940e0439f451bdaa2a58750acda1c118ad3fc5f5a51572f39e2e16ac0`.
- New plan physical SHA256:
  `5b2368f13073d41ced50ecff960ee20c3653065b9cee07a2eefb8889e24c4b4a`.
- New policy:
  `0b1b017796960c4d0c84814252101a49e6950ccc57118a50ded2aa7a5e6c740c`.
- New run: `artifacts/teacher-device-20260924-run/`.
- Independent evidence: `artifacts/teacher-device-20260924-review/`.

Native physical SHA256 values are parent
`15015900a3c91a6b8ed87e9c2ecb8f7faf124849e5dcf8f40203cddf267e30d9`, A512
`a549392f3f12ebdea95616d6d66043f9b88e1e56ce40ab74d6159304069a8da0`, and M512
`bca802ec29fe4c92a977cc501a928e1f91fc4bf3e4caf60969a1817bf5a9974c`.
Their steps are14336/14848/14848, with local512 for both endpoints.
The corpus SHA256 is
`96e36843c792354bff98195e95db3ec98c4d0dde534aaa619787b5259665fa3c`;
metadata SHA256 is
`21b022c372f28d35ea2f2ce106c4b8a1f5b94e15a8e57d9b7fabb97e80c4f8af`;
tokenizer semantic identity is
`c32ffe6d80251e7eb55607711c78c293d80fc4ede199dd1673868aadfa14be41`.
The actual runtime is Metal0/F32, device `Metal { gpu_id: 4294968525 }`,
macOS27.0 build26A428 and the unchanged `metal-f32-sum-contiguous-v1` patch.

The changed source is confined to `src/muon_diagnosis.rs`, the existing teacher
prefix collector in `src/fresh.rs`, and forward observability in
`src/quality_recovery.rs`. Model math, tokenizer, generation policy, product,
vendor and Cargo lock have no delta. The reviewer found a reachable new empty
prefix regression during preparation; it was corrected before model execution.
No remaining confirmed source defect was found. The actual caller evidence
below independently closes the previous helper-only verification gap.

## Independently executed A-delta evidence

The reviewer authored two negative tests against the production reader and
executed each once on the final test executable. They reject changed D2 rows,
missing/duplicate rows, incomplete/pending resolutions, mixed cancellation or
UNKNOWN/nonfinite/I/O failures disguised as the known device failure, old failed
root reentry, old rows relabelled with a new source, and changed parent/A/M,
step, tokenizer, runtime or teacher selection. Synthetic journals are confined
to the new scratch root; models and corpus are read in place.

| Exact test | Result | Wall seconds | Maximum RSS bytes |
| --- | --- | ---: | ---: |
| `independent_successor_rejects_d2_and_failure_forgery` | 1/1 PASS, exit0 |27.87|3,890,855,936|
| `independent_successor_rejects_identity_changes` | 1/1 PASS, exit0 |30.12|4,892,590,080|

The implementer's focused prefix fixture checks initial empty legacy prefix,
first1/full64 immutable identity, a fresh-process reader, exhausted-budget
no-call reading and duplicate rejection. Its evidence is identified separately
from the reviewer-executed tests; no numerical test-count threshold is used.

The independent Rust plan reader newly checked all1,856 scoped references,
case/ordinal/RETURNED bindings and complete D2 lane finals. It independently
reconstructed teacher64, check8 and replay32 normal+32 failure extras per arm,
verified parent14336/A512/M512 metadata, content and physical identities,
SMALL/F32/framing, and runtime parity except for the new binary. It exited0 in
2.06s, maximum RSS231,489,536 bytes. Both selected replay unions contain64 cases;
M retains two strict-UTF8 cases. This A-delta stage made zero model calls.

The old independent reader was also run anew against the failed predecessor
before preparation, exit0, wall1.50s. It verified original586+missing54 =
composite640 and train256, hence896 unique reused rows. Both train FULL scores
remain0/128: A whole-value50/support0; M whole-value0/support2. Per arm42 cases
were exposed once and86 twice. These scores and historical5,604 newly generated
tokens are not new successor model work.

The historical failed teacher-row digest remains
`814debc4b71bc32598257aa6fd3416c61393fa0f1bd9955d98f1420c704479a3`;
the failed observation003 remains
`525b82accad293c8a37eeddbf6d2c68d6074290feca2fc1ac2ab2e14b27d6edb`.
Historical usage is generation310, teacher attempts1/results0, active57.566452876s.
No old failure or UNKNOWN is converted into successful evidence.

## Actual caller and fresh readback

All three formal ordinal-zero rows completed on the registered Metal device.
Their full64-case headers are unchanged; each prefix terminal records1/64 and
work_remaining=true, with no complete lane final or false timeout. Each actual
observation records one teacher call and zero generation/optimizer/backward.
Active time was parent0.735365875s, A0.697272459s and M0.62630425s, total2.058942584s.
The three rows contain48 canonical target tokens including EOS.

The independently authored metadata-only Rust reader ran in a fresh process,
exit0, wall1.65s, maximum RSS165,183,488 bytes. It allocated no model or GPU
tensors. It rebuilt prompt and full-answer shift digests, checked canonical
gold/EOS/raw-byte roles, target alignment and finite NLL, and verified actual
Metal U32 input, Metal F32 logits, shape and finite-logit evidence. Native
architecture/tokenizer/content/physical identity and steps match; row/case/
ordinal/prepared/RETURNED hashes and prefix/segment finals agree. Native bytes
and all20 run file names/contents remained unchanged. Its new model calls are0.
Incorrect argmax tokens remain unmodified quality data.

The producer's fresh-process readback digest is
`957f2855675e73ab9568dba141dd6475cc7eda2ca505828405dc9eff12a8ab4f`.
Independent native receipts are A-delta
`5bae6e9bf9f90672be450b3546d2c72b66c527c907844b7ed71a6659c5263e53`
and A-caller
`16efeb95aae29450259488bf48135783697b275cfde41c032d2555c1fe0ab504`.
They bind immutable phase notes/evidence; the caller receipt records the actual
three calls and the first-row digests, without treating the growing raw stream
as an immutable whole-file prefix hash.

## Completed main and independent reproduction

The remaining189 teacher rows appended on the same source and full64 manifest.
The independent full reader checked all192 rows/3,072 target tokens, exit0 in
1.35s, maximum RSS166,821,888 bytes. Its receipt comparer verifies the accepted
first three rows are identical and each continuation used exactly63 calls.
Every teacher row has canonical gold/EOS, matching NLL/argmax lengths, finite
scalars and the original case/model/input/runtime binding.

The reviewer then executed the five actual reproduction lanes in separate
processes using the same frozen executable and one compute thread. The fixed
teacher indices were0,1,16,17,32,33,60,61; gold and argmax matched exactly,
and maximum absolute NLL difference was **0** across384 target tokens, within
the unchanged1e-5 limit. Raw-byte role identity follows the same independently
checked canonical targets. No teacher/generation failure or UNKNOWN occurred.

| Reviewer lane | Calls | Active seconds | Wall seconds | Maximum RSS bytes |
| --- | ---: | ---: | ---: | ---: |
| check-parent |8 teacher|1.526647500|5.82|860,717,056|
| check-A |8 teacher|1.478943458|5.57|744,194,048|
| check-M |8 teacher|1.491064792|5.60|823,115,776|
| replay-A |64 generation|9.655673250|19.27|2,635,595,776|
| replay-M |64 generation|11.912395292|21.81|2,612,117,504|

Each replay has32 metadata-prefix normal cases (V8/VC6/S1Q1-6/word6/renamed6)
and32 additional deterministic failure cases, union64. Normal does not mean
correct; selection retains complete query pairs and skips normal overlap when
adding failures. No mate is missing or fabricated. The explicit normal and
failure-extra ID lists are recorded separately in `final-verification.log` and
are bound by the native B receipt. A produced924 tokens; M1,181. All128 matched
the original raw tokens, text/null, EOS/length and strict decoding states.
M retains two strict-UTF8 failures and13 length endings in this selected sample.
These reproduce incorrect outputs and do not estimate general accuracy.

## Independently recounted role and prefix findings

Each train/dev split contains32 gold-prefix examples, with16 target tokens per
example. Every answer has one MIXED token crossing VALUE/FORMAT, five clean
FORMAT tokens, eight ID tokens, CLOSE and EOS. **VALUE has no pure-token
denominator**, so a VALUE0% teacher score would be invalid. Full VALUE and
FORMAT span coverage is0/32 per split because of the crossing token; clean
ID span coverage is32/32, with **zero fully correct eight-digit IDs in every
model/split**. MIXED is retained as its own role.

Entries below are correct tokens/denominator; parentheses give mean NLL.
All counts, NLL sums/means and span counts were independently aggregated from
raw arrays and exactly matched the native composite across all six splits.

| Model / split | MIXED | FORMAT | ID | CLOSE | EOS |
| --- | ---: | ---: | ---: | ---: | ---: |
| parent / train |0/32 (19.486)|50/160 (5.163)|24/256 (12.405)|0/32 (13.915)|0/32 (11.621)|
| parent / dev |0/32 (17.943)|47/160 (5.308)|20/256 (12.741)|0/32 (12.425)|0/32 (12.099)|
| A / train |12/32 (1.434)|160/160 (0.000247)|48/256 (2.266)|32/32 (0.00320)|32/32 (0.000344)|
| A / dev |5/32 (1.450)|160/160 (0.000347)|34/256 (2.257)|32/32 (0.0196)|32/32 (0.000328)|
| M / train |0/32 (3.733)|140/160 (0.272)|34/256 (4.996)|2/32 (3.992)|32/32 (0.00253)|
| M / dev |0/32 (3.447)|140/160 (0.281)|24/256 (5.345)|8/32 (2.960)|32/32 (0.00522)|

All32 selected teacher train cases had two actual exposures in each A/M512
update tape. The parent uses the same diagnostic manifest; this does not claim
that its weights received those two exposures. The broader reused train128
still has42 once-exposed and86 twice-exposed cases per arm; both FULL
scores remain0/128, with A whole-value50/support0 and M whole-value0/support2.
Thus A learned some word values and clean formatting, while neither model shows
complete ID prediction in these teacher samples. These bounded observations do
not establish permanent inability to learn, a sole mechanism, or independence
between word and renamed versions of the same semantic scenes.

For A, the first free-token mismatch is MIXED20/ID12 on train and MIXED27/ID5
on dev; these counts exactly match its first teacher-argmax mismatch roles.
For M, both train and dev first mismatch at MIXED32/32. At that first divergence,
the teacher argmax equals the free token in64/64 cases per arm. Prefixes after
that point differ and are not compared as equivalent inputs. Token mismatch
does not override final-text scoring. Gold-prefix ID results do not replace free
support scores or prove that exposure bias or forgetting is the single cause.

## Closure, usage and preservation

The independent closed reader exited0 in1.04s. A separate fresh production
report ran inside a before/after snapshot: all733 new-run file names and hashes
were unchanged, with zero new model calls at fully spent call caps. That report
took21.46s and maximum RSS4,217,012,224 bytes for pure data/hash validation;
it did not create missing evidence. The composite was published only by explicit
close, and its SHA256 is
`3c35b40dbabac5eb80f5e9b284b6f5025a27b1127b65b90aaa2f2f318f88e99c`.

Successor usage is generation128/128 with2,105 generated tokens, teacher216/216
one-example forwards with3,456 target tokens (main3,072 plus independent384),
optimizer0/backward0, failed0/UNKNOWN0. Model-active time is40.235394543s against
1,800s; the largest segment is11.912395292s against900s. Reviewer T4 accounts
for generation128/teacher24 and26.064724292 active seconds. Compile, hash and
pure-reader times are separate. Historical generation310/5,604 new tokens and
failed teacher1 are retained separately; reused historical8,342 tokens are
not added to successor generation cost. The896 reused D2 rows are not claimed
as regenerated rows.

At the native B receipt checkpoint, logical evidence bytes totalled36,892,790:
run733 files/1,450,079 bytes; execution evidence24 files/22,247,885 bytes;
review690 files/13,194,826 bytes, including narrow synthetic journals. This is
below256MiB. Later identity/publication metadata are separately retained.
The implementation reported scoped Cargo target allocation growth516KiB;
reviewer tools compiled directly to scratch without writing the Cargo target.
No model, Adam, corpus or vendor copy/cleanup was performed. Scoped refs still
match, and original failed terminals and reports were not rewritten.

Independent B receipt SHA256:
`9b9fc816e863415e2991245798aeded6c776663c137851f4662830a33752ee76`.
It records SUCCESSOR_DIAGNOSIS=PASS while preserving original failures,
MODEL_QUALITY=FAIL, MUON_ADOPTION=NO and S4/S5/S6/GOAL1=NOT_ACCEPTED.

## One next quality variable, not executed

The evidence supports testing **an ID-only training response target** against
the original word-plus-ID response, from the same preserved A512 state with
the same data/framing/LR and Adam. A proposed separate study would cap each arm
at128 updates,256 total, keep disjoint heldout data and retain original full
response metrics. This changes target content and effective answer-length CE
weighting; it is not a pure isolation of selector mechanics. If the registered
budget yields no heldout exact-ID gain, the proposed change lacks support.
Even improved ID-only output would not establish full QA or Goal1 readiness.
No such learning was authorized or executed in this closure.

Muon adoption remains unsupported by the actual quality comparisons. Previously
recorded first-update norms were A0.0905808754 and M0.0086761903; these are reused
historical observations, not new update measurements. Together with the different
optimizer costs, they limit equal-step algorithm comparisons and do not justify
an automatic tenfold LR increase. Neither arm passed the full-response gate,
and this configuration offers no measured quality benefit for adopting Muon.

The reviewer used the repository's Replica Lean Development and Karpathy
Guidelines skills. Only new reviewer Rust tools, scratch evidence and this report
were written by the reviewer; original reports, source artifacts and untracked
`.DS_Store` were preserved. Report publication identity is separate from the
reviewed execution source.
