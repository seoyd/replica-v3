# FULL response fit — independent review

## Final authorized posthoc B

**Posthoc A/B execution and parity PASS; model development and full-train fit FAIL.**
Source `af8ad53bf52e592f531b2f29ea84670ac201e618` and executor
`052c9a7d150b97f5a0b16ac2f7968a721cc529688946163e0e72de06403f54d4`
remained frozen. The original bounded run remains incomplete at committed3069,
discarded3/backward3072, model17981/Adam3645/cursor3069. Its original
INTEGRITY_FAIL, success=false/resume=false/fit=false terminal is unchanged.
The separately authorized diagnostic evaluation completed all seven panels and
teacher64; independent B completed64 generations and teacher8, optimizer/backward0.
This neither completes3072 training nor grants candidate/S4/Goal1 acceptance.

The independent Rust reader verified all3520 main raw rows, strict UTF-8, actual
EOS, full/value/support/outside/malformed metrics, QB/SB/ALL4, request/evidence,
native/tokenizer/source binding and each RETURNED resolution. All stored scores
agree. B's fixed normal32 quota8/6/6/6/6 and extra16 outside-ID failures plus16
query mates total64; all raw tokens/text/EOS/error outcomes reproduce exactly.
Those mates are correctly answered rows, not32 additional errors. B scores are
not added to accuracy denominators. Teacher gold/argmax/roles match and maximum
absolute NLL reproduction difference is0.

| Final3069 panel | FULL | QB / SB / ALL4 | Whole value / support | Outside |
|---|---:|---:|---:|---:|
| Value | 512/512 | 256 /256 /128 | N/A | 0 |
| Citation | 510/512 | 254 /254 /126 | 512 /510 | 2 |
| S1Q1 | 478/512 | 222 /231 /103 | 511 /479 | 33 |
| FULL word | 29/192 | 0 /0 /0 | 38 /140 | 52 |
| FULL renamed | 45/192 | 4 /0 /0 | 55 /154 | 38 |
| FULL train | 485/1536 | 32 /1 /0 | 494 /1508 | 28 |
| OLD_FULL | 1/64 | 0 /0 /0 | 12 /2 | 62 |

Every row has actual EOS; malformed, other-provided-ID and generation errors
are0 throughout these final panels. Value retention passes; citation fails its
outside-ID condition and S1Q1 fails retention. Development and full-train gates
fail. OLD_FULL measures the original system separately. Dev/renamed share48
semantic groups; they are not384 independent groups.

Full train by value: 왼쪽79/384, 오른쪽166/384, 직진146/384, 대기94/384.
The six pair totals and two ID versions are:

| Pair | Total /256 | ID0 /128 | ID1 /128 |
|---|---:|---:|---:|
| 0–1 | 86 | 41 | 45 |
| 0–2 | 64 | 34 | 30 |
| 0–3 | 69 | 35 | 34 |
| 1–2 | 103 | 53 | 50 |
| 1–3 | 79 | 39 | 40 |
| 2–3 | 84 | 41 | 43 |

ID-version totals are243/768 and242/768. Exact exposure strata are prior0/new8:
396/1280, prior1/new7:5/12, prior1/new8:84/244. The committed/discarded and
different-system semantic exposures remain separate as recorded below. Original
train128 has different coverage and is not this1536 panel's paired baseline.
Against identical original F64 requests, gain/loss is word29/0 and renamed45/0
on192 each, OLD_FULL1/0 on64, S1Q10/3 on the common64, value/citation0/0 on64.

Final teacher train/dev ID tokens are256/256 and248/256, while whole8-digit IDs
are32/32 and24/32. Clean FORMAT160/160, CLOSE32/32 and EOS32/32 pass in each
split. MIXED is6/32 and4/32; VALUE-only has no denominator (N/A), and whole
FORMAT span coverage is excluded because of that crossing token. ID NLL sums
are1.0303948453/24.2042868587; MIXED sums44.3223524094/45.3629331589.
The sample contains only 왼쪽/오른쪽16 each per split, and all train32 have
new exactFULL8/priorFULL0. First teacher errors are trainMIXED26/NONE6 and
devMIXED28/ID1/NONE3. Free-generation first differences in word/renamed/train
are MIXED154/137/1042, ID9/10/9, exact29/45/485. Gold-prefix suffix accuracy
does not correct a wrong generated value. Valid text lengths are33 or36 bytes:
word136/56 rows, renamed126/66, train1002/534, respectively.

Posthoc main+B usage is3584generation/72teacher,51132 generated tokens.
Combined with the preserved original run:6864generation/136teacher,95162
generated tokens and2176 teacher targets. Optimizer3069/backward3072 and
TINY8/8 remain unchanged. Stored main554.91381525s plus B12.489815s adds
567.40363025s to prior3035.100428542s: **3602.504058792s total active**.
The later console main snapshot554.946385583s differs by32.570333ms; it is not
the immutable accounting value. Independent B process exited0, wall25.46s,
maxRSS1734836224bytes/footprint1912686800bytes. Pure main/B/teacher readbacks
exited0 in2.06/0.81/0.47s and made no model calls.

Last durable native physical SHA256 remains
`f61db3b873859f2aac5083ace36385067f8cbb4d250cdd4d364d0eea289ea55c`.
Eleven original consumed refs and five posthoc origin/descriptor refs match their
before hashes. Scoped snapshot epoch1790296419: posthoc study7388files/
16402911 logical bytes, posthoc evidence23/42931565, current review107/31038067.
Allocated bytes respectively40218624/42995712/31297536. Original study plus
posthoc study and registration total1061310924 logical bytes, within the
implementation's scoped model/raw allowance. **The broader historical three-root
footprint1106603756 already exceeded1GiB; this is not a whole-scope storage PASS.**
Executables and other test fixtures remain preserved, later report/receipt growth
is excluded, and build peak is UNKNOWN. No whole inventory or cleanup occurred.

One unexecuted next hypothesis: in a newly registered controlled study, change
only first-target weight1→2 to test whether more relative weight on the first
MIXED value/format decision improves FULL train and paired heldout scores.
This is neither a pure VALUE-only signal nor a confirmed cause/fix. Lack of
preregistered heldout/paired improvement or worse retention would falsify the
proposal. No new learning is authorized or executed here.

Evidence under `artifacts/full-response-fit-20260925-review/`:
`posthoc-main-readback.r3b`, `posthoc-b-readback.r3b`,
`posthoc-teacher-readback.r3b`, `posthoc-details.r3b`, their named logs,
`posthoc-b-phase.md` and `posthoc-review-b.r3b`.
Receipt SHA256 is `0c4f06fb30d8224b9f3ad13019cd7a65b172c1e8446e5d1057423be68dbda27f`.
The actual B command was
`replica-train fresh muon full-fit-posthoc --root artifacts/full-response-fit-posthoc-20260925-study --phase review`
using the frozen posthoc executable and VECLIB/OMP/RAYON thread limits1.
Exact commands and source/binary/log hashes are bound by the B receipt; old
partial sources and receipts retain their original hashes. Reviewed source and
the subsequent report-only publication commit remain distinct.

**CODE_A=PASS; FULL_FIT_EXECUTION=original incomplete/posthoc complete;
B_RECOUNT_PARITY=PASS; CANDIDATE_DEVELOPMENT=NOT_ACCEPTED;
ORIGINAL_RUNS_UNCHANGED=verified; GOAL1_READY=false.**

## Separately authorized posthoc A

The user subsequently authorized no-learning final-native evaluation and B,
retaining the old cap/failure rather than completing the original3072 run.
Independent A passed the new boundary on source
af8ad53bf52e592f531b2f29ea84670ac201e618. Descriptor policy/physical is
b8b3e02a3464632b108d5710ff7815e979c99a82a9aa19e39f8a5ebb866057b1;
executor052c9a7d150b97f5a0b16ac2f7968a721cc529688946163e0e72de06403f54d4.
Receipt `artifacts/full-response-fit-20260925-review/posthoc-a/review-a.r3b`,
physical3d52de63ae888bd9a1a5cc5a06c101cdab406d73e8dc7d22a86841b69466d2c6.

Actual frozen CLI processes rejected duplicate sibling registration, existing
outside output, a new child inside the original, and using the posthoc root
for ordinary training. Each expected exit1 preserved the original/posthoc file
maps and all checked hashes, without new pending/attempt/admission records.
The independent Rust reader bound old native/clock/config and consumed tokens,
trace digest and prior3280 generation/64 teacher/3035.100428542seconds to the
new descriptor. Original3069/model17981/Adam3645 and failed resume=false are
unchanged. New collector byte allowance is28833811 after original logical
bytes and registration; broader evidence-footprint excess is not hidden.

The reviewer reused final-source direct scope and four-row RETURNED process
evidence (6.57s/8.00s, one test each plus included child), without rerunning old
Metal/scorer/optimizer verification. Independent readback plus four CLI checks
took18.86s, all new forward/generation/teacher/optimizer/backward0. The source
review caught original-internal output before writes; it was fixed before
source freeze and the dynamic case above verifies it. This A accepts only the
new observation boundary; original run completion and model quality remain
unaccepted. Exact commands and case-log hashes are in the local
`posthoc-a/phase-a.md` and `posthoc-a/readback.r3b` beneath the review root.

**CODE_A=PASS** for source `80ec58a37bf50d41a919a0ddfed86b4c969fd4b8`, policy `403e9c50e054048f5967fb60ff5ea9731b8f987a019dc8c68794187c97322858`. This accepts the registered single-F continuation's input/state/caller boundaries. At A admission, SMALL learning and parent parity had not run; model quality, development and Goal1 were not accepted.

The original F64 is unchanged: physical `9ea752b455d0222cdbc414f799fd86e8a3aff1f31d3893f3571f7e215b4a3ca4`, model14912/Adam576/cursor64, optimizer origin14336. Independent native tensor readback verified full weights and Adam digest `8a37026b1b815dcf2360745dec7c3c15184e2fabd9504e6b191a206ee9291431`. New cursor0 is distinct from all three historical counters. Hyperparameters, LR3e-5, ANSWER mean CE, existing FULL corpus/tokenizer and review inputs remain fixed.

The independent Rust reader verified the original 3,072-row native tape, old Muon prefix1,024, actual F64 consumption512..575 and new rotation576..3071/0..575. It calculated input4,927,488/target359,424/padding798,720 and maximum shifted length233. Train192 covers six word pairs, four semantic groups per pair, two ID versions and four views. Train has192 semantic groups, dev48 disjoint groups, and renamed shares those48 heldout groups.

| Word-row exposure identity | Actual preparation count |
|---|---|
| New exact FULL, planned full cycle | All1,536 rows ×8 |
| Prior exact FULL in F64 | 1,280 rows ×0;256 ×1 |
| Earlier A512, semantic match with different prompt | 1,024 rows ×1;512 ×2 |
| Cumulative exact FULL after a complete cycle | 1,280 rows ×8;256 ×9 |

These are planned new exposures until the actual trace is available. Review exposures differ: successive1,024-row source blocks total2,048/1,536/1,024/1,024/2,560/4,096. Existing validation bounds and unchanged scorer/Metal/teacher evidence were reused; no ID_ONLY or prior full diagnostics were rerun.

Independent actual Metal TINY used the production load/update/save path for continuous2 versus1+fresh-process1. All weights, Adam m/v, LR, clocks, cursor and next tape row matched; final model14914/Adam578/cursor2 points next to source578. Independent native/trace readback also passed. Root and reviewer used four updates/backward each: total8, generation0, teacher0. Independent wall time was19.56seconds including child, maximum RSS605,945,856bytes. Together with direct5.34seconds, A charges24.90seconds. Child updates are not counted twice. Pure preparation1.89seconds, native readback0.29seconds and negative metadata checks1.00second are separate file work.

The frozen direct caller fixtures passed final3072, early joint acceptance1536, train-fail continuation1536, and retention stop768 with train192/OLD_FULL64 but no unscheduled teacher. They verified trace3072/panel1536, prefix reuse, wrong-model/oversize rejection and idempotent guard rereads with zero model calls. Their403.80-second outer run constructed/read synthetic files; it is not3,072 TINY updates. These immutable fixtures were reused rather than written again.

Independent isolated metadata fixtures invoked the actual dispatcher for UNKNOWN, cancelled control and wrong native physical hash. Each exited101 with its expected rejection before a new segment/attempt/native; the supervising check passed with all model calls0. The initial TINY fixture origin mismatch was fixed before source freeze and both actual runs validated the correction. The standalone reader's first compile missed the existing zstd extern; that log is preserved and its corrected compile/readback passed. No confirmed blocker remains.

Evidence is under `artifacts/full-response-fit-20260925-review/`: immutable `phase-a.md`, `preparation-readback.r3b`, `tiny-readback.r3b`, `blocked-dispatch/readback.r3b`, actual logs and native A receipt. Matching direct evidence is under `artifacts/full-response-fit-20260925-evidence/`. Frozen executor SHA256: `f1aa135e0eb71bafdf9006c5de7f655cda84335b4661bd335ab67f7303fe7495`; tests: `390c049588468f240aba9f65eda8a7496fc3516437ad58e7dd337041a99e422e`. Original corpus, native states, terminal decisions, failures and prior reports were read-only.

## Historical original-cap closure — before posthoc authorization

The same independent reviewer subsequently read the actual bounded run, using
standalone Rust readers and the existing locked dependencies. Product source
still matches80ec58a37bf50d41a919a0ddfed86b4c969fd4b8. The frozen executor hash
also matches. Raw/state audit exited0 in4.17s; teacher readback exited0 in0.49s.
All new audit generation/teacher/optimizer/backward counts are0. These are
scratch readbacks, not a canonical completion or B acceptance receipt.

The original terminal is success=false/resume=false/fit=false and records
INTEGRITY_FAIL with direct cause `FULL_FIT backward budget exhausted`.
Committed3069 plus discarded3 equals the unchanged backward cap3072. No
numerical corruption was found by this audit. Final model/Adam/cursor are
17981/3645/3069; last evaluated cursor2304. Native physical
f61db3b873859f2aac5083ace36385067f8cbb4d250cdd4d364d0eea289ea55c and all
19026816 finite Adam scalars were verified;11 protected references match A.
The failed terminal was neither edited nor treated as a normal final endpoint.

All scheduled256/768/1536/2304 raw panels independently match their strict
scores, EOS/error decoding, policy/model/request binding and guard chain.
Parent16 raw parity also agrees. S1Q1 warned at1536 (fixed64 FULL57/ALL411)
and cleared at2304 (61/13); there was no retention stop. At1536 word/renamed/
train192 had FULL31/32/30, whole-value50/43/51, support91/126/123 and
outside-ID101/66/69 respectively. ALL4 was0 in each. Representative train
word accuracy was 왼쪽4/48, 오른쪽3/48, 직진21/48, 대기2/48; its six word-pair
totals were3/4/3/9/0/11 of32. This is not full train1536 acceptance.

Teacher64 at1536 is only a readback of the existing gold-prefix computation.
Train/dev ID tokens242/256 and232/256 coexist with full eight-digit ID20/32
and14/32. Clean FORMAT160/160, CLOSE32/32 and EOS32/32 pass in each split;
MIXED4/32 and5/32 do not. VALUE-only is N/A and whole FORMAT spans are
excluded because of boundary MIXED, distinct from the clean FORMAT tokens.
The sample covers only 왼쪽/오른쪽16 each per split; train32 had new exact
exposure4 and prior FULL0. Free word/renamed/train192 first differences are
MIXED142/149/141, ID19/11/21 and exact31/32/30. These diagnostics do not
establish a tokenizer defect or substitute for final free generation.

Verified actual costs including discarded work are input4927268,
target359394 and padding798940. Generation3280 returned44030tokens;
teacher64 used1024target tokens. Active accounting3035.100428542s includes
A24.90s; TINY remains8 optimizer/backward and0 generation/teacher. New
committed word exposure is1524×8 and12×7; cumulative exact FULL is1292×8
and244×9. Discarded backward exposures are separate. The final native's
quality, full train1536 and OLD_FULL64 are unmeasured.

**B_FINAL/B_PARITY=NOT_RUN_SCOPE_BLOCKED.** Normal32/failure reproduction
and final teacher8 were not run. A passing partial raw audit cannot authorize
new calls after this failure, complete missing panels, or accept a candidate.
No final3069 score is inferred from an earlier checkpoint. GOAL1_READY=false.

Scoped snapshot epoch1790272255 measured study9848files/1044907746bytes,
evidence17/42505633 and review72/19190377 logical bytes. Their sum1106603756
exceeds1GiB by32861932; only the implementation's owned-study check is below
that limit. Allocated bytes are1081413632/42545152/19378176 respectively.
Later report growth and other direct-fixture roots are excluded, and build
peak is UNKNOWN. No cleanup or global inventory was performed. The next
proposed action is separately authorized no-learning final-native evaluation,
not another training intervention; this audit did not grant or execute it.

The scratch note `artifacts/full-response-fit-20260925-review/partial-audit.md`
contains the exact reader commands, binary/source hashes and raw-log paths;
SHA2560bcc0159506dae1257aa0be4fab13e32692e57c2b7ba6f848ca61064083974c3.
Readbacks: partial-readback.r3b
2c6df18b61722bbfa359d5c049522b6a9edbce0f5439b772c274b5f6d20c4769;
teacher-partial-readback.r3b
598d9ec730173d61a296d8c63dc266071833fe6e38ca7f695674ea5db4d7fa10;
partial-details.r3b
2409fd55f0b414756efacf48691931b1bed2f7bdf5eecb3e7919eff58cdc9c6d.
Final reader compile/execution commands exited0. The first teacher-reader
compile omitted the existing sha2 extern and failed; its log is preserved,
and the corrected link command passed. This was a reviewer command failure,
not a product-code defect. No further verification was added after closure.
