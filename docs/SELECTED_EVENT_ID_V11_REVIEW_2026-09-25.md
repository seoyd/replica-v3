# Selected event ID protocol 1.1 — independent review

Independent A is **PASS** for frozen source `2e98f6a834ffe440ce2f20dec4361dee8818ef29` and policy `e0a8d176266d932a3a20a86e5c6d4b66ed03434abc776a5cd5b8652ed7ec0e7f`. Baseline, conditional learning and independent B were pending at this stage; model quality and Goal1 were not accepted.

Final outcome: input length **PASS**, execution **A/B PASS**, bounded model quality **FAIL / NO_CLEAR_SIGNAL**, **Goal1=false**. Both arms stopped at the common +64 endpoint after I crossed the registered retention-error threshold. The A evidence below describes the earlier admission; final execution and B evidence follow it.

The separate Rust reader verified all 22,272 derived episodes, exact request transformations, source/native bindings, split disjointness, unchanged review rows, registered tape and costs. All six length summaries passed: FULL is 218 prompt + 15 gold + EOS = 234 tokens; ID_ONLY is 233 + 8 + EOS = 242. Each mode contains 1,536 train, 192 dev and 192 renamed records, always two provided and zero excluded evidence records. The maximum remains 256. The earlier 1.0 failure and its artifacts are preserved separately.

| Per-arm registered 128-update cost | F | I |
|---|---:|---:|
| Input tokens | 208,128 | 212,224 |
| Target tokens including EOS | 14,976 | 11,392 |
| Padding | 30,464 | 34,560 |

The arms share batch order and exposure. Target simplification also changes ANSWER weighting and token cost: I adds 4,096 input tokens and removes 3,584 target tokens. Train has 192 semantic groups and dev has 48 disjoint groups; renamed reuses those 48 heldout groups. On the fixed train128 panel, 42 records have parent/new exposure 1/1 and 86 have 2/0.

Independent actual Metal F32 TINY passed eight updates and two teachers, with fresh-process split runs matching uninterrupted weights, complete Adam m/v, clocks, cursor and token counters exactly. Model/Adam clocks advanced 14848/512 to 14850/514. Identical tapes plus cursor 2 preserve the next batch. Moved natives load; wrong arm/runtime reject. Both actual teacher rows passed independent device/dtype/shape, gold-shift, digest, finite-NLL and role checks; the pure reader inspected 25 target tokens without model calls. ID_ONLY has eight ID positions and EOS; FULL keeps MIXED spans explicit.

Root and reviewer together consumed **16 TINY optimizer/backward updates, four teacher calls and zero generation calls**. TINY updates are exhausted. Independent TINY wall time was 18.18 seconds, maximum RSS 1,795,833,856 bytes. A conservatively charges 32 seconds to the active budget, covering both TINY runs and the relevant numerical/readback checks. Frozen-source direct request/clock, scorer/gate and actual final-only no-call process evidence also passed and was reused without repetition.

Immutable evidence is under `artifacts/selected-event-id-20260924-review-v11/`: `phase-a.md`, `preparation-readback.r3b`, `tiny-teacher-readback.r3b`, raw TINY native/process evidence and the typed `review-a.r3b`. Matching root direct-test evidence and frozen binaries are under `artifacts/selected-event-id-v11-20260924-evidence/`. Executor SHA-256 is `69e3fac7b7ab9d43e3eca5886c7c3738fc7aaef64b80fbd2ae03b3c175c22d30`; test binary SHA-256 is `e2253e69cbe0c031bf6846537ff8db88c2a8fe55dcfdb2a1d9eb57d795133bd6`.

**Independent B, completed.** The separate reader verified baseline and final raw results, strict decoding/scoring, native configuration and clocks, tape, exposure, costs, protective decisions and paired comparisons. F/I first saved model14849/Adam513/cursor1 and continued in a fresh process. Their common final clocks are model14912/Adam576/cursor64. Optimizer origin14336 and study start14848 remain distinct. Terminal segment001 is successful with resume=false. F's endpoint physical SHA is `9ea752b455d0222cdbc414f799fd86e8a3aff1f31d3893f3571f7e215b4a3ca4`; I's is `865a6d8d4273e1e868b7e192ce53611ad018699fb884992ef2687e0e4efbb844`.

I's S1Q1 has FULL57/64, EOS60/64 and four completed length-limit returns. The error/non-EOS union of four triggers SEVERE_RETENTION even though FULL exceeds the preserved A512 reference54/64. F has64/64 on V, VC and S1Q1; I has64/64 on V and VC. No +65 update occurred. Final evaluation and teachers completed at +64 for both arms; the quality stop is not a runtime or integrity failure.

| Actual +64 training cost | F | I |
|---|---:|---:|
| Updates / backward calls | 64 / 64 | 64 / 64 |
| Input tokens | 104,832 | 106,880 |
| Target tokens including EOS | 7,488 | 5,696 |
| Padding | 14,464 | 16,512 |
| Review / word exposures | 256 / 256 | 256 / 256 |
| Unique training records | 512 | 512 |

Fixed train128 exposure remains 42 records with parent/new 1/1 and 86 with 2/0. These parent exposures describe A512's earlier training tape. Neither the train panel nor teacher gold prefixes are heldout free-generation successes. Word and renamed each contain192 rows from the same48 heldout semantic groups; their384 rows are not384 independent cases. Existing key pairs and vocabulary can overlap training, while evaluated event IDs and record combinations are held out.

The frozen executor's `fresh muon review --root artifacts/selected-event-id-v11-20260924-study --arm F` and then `--arm I` both exited0, with one owned process and one compute thread. Wall times were14.84/15.98 seconds; maximum RSS was1,180,254,208/1,191,362,560 bytes. Each pre-call manifest fixed32 normal cases (V4, VC4, S1Q1 4, FULL word8, ID word8, FULL renamed2, ID renamed2) and32 additional failures/mates. Independent selection reconstruction matched those manifests. No strict-UTF8 failures existed; F extras contained16 malformed and16 valid-wrong cases, while I extras contained24 malformed, five valid-wrong and three successful mates.

All128 fresh generations reproduced raw tokens, text, EOS, completion and decode/error states exactly. All32 fresh teacher rows matched gold, argmax and roles; maximum absolute NLL difference was0 against the unchanged1e-5 bound. The pure teacher reader also checked all256 main rows, including actual Metal U32 inputs/F32 logits, shapes, exact gold shift, prompt/input digests and finite NLL. Wrong answers reproduced exactly remain wrong answers.

| Heldout free generation, word / renamed | Parent | F | I |
|---|---:|---:|---:|
| FULL exact | 0 / 0 | 0 / 0 | 0 / 0 |
| FULL value correct | 45 / 47 | 40 / 50 | 0 / 0 |
| ID_ONLY exact | 0 / 0 | 0 / 0 | 0 / 0 |
| ID_ONLY malformed | 192 / 192 | 192 / 192 | 192 / 192 |

Each denominator above is192. QUERY_BOTH, value-swap joint correctness and ALL4 are zero for all modes, variants and models; all I-versus-parent/F paired exact counts are `[both=0, gain=0, loss=0, neither=192]`. F/I train128 exact is also0/128 for both modes. Original OLD_FULL64 exact is0/64 in both arms; value correctness is F5/64 and I0/64. I's occasional citation support does not make its malformed FULL output correct.

ID_ONLY produces no eight-digit-only heldout response. For word dev, the actual raw-token length histograms (including EOS; `length:count`) are:

- Parent: `2:18, 4:4, 7:170`.
- F: `2:10, 11:16, 12:6, 13:22, 14:50, 15:16, 16:19, 17:2, 18:51`.
- I: `3:7, 4:55, 10:15, 11:16, 14:10, 15:4, 16:55, 17:4, 18:22, 19:2, 20:2`.

Renamed raw-length ranges are parent2–7, F2–18 and I2–19; the complete byte/character/token histograms are preserved in `closure-readback.r3b`. Representative output shapes are the parent's Korean answer plus incomplete citation, F's Korean answer plus repeated digits/brackets, and I's digits with extra closing brackets or Korean text. I word responses include35 ASCII non-digit-only strings and157 strings containing non-ASCII characters; renamed has35/154 plus three wrong-length digit strings. All these ID_ONLY panels eventually emit EOS, but at the wrong content/length. The first wrong gold-token position is0 for all parent/F rows; I is0/1 on156/36 word rows and168/24 renamed rows. Token position is distinct from decoded byte or character length.

| Gold-prefix teacher correctness | F train | F dev | I train | I dev |
|---|---:|---:|---:|---:|
| ID_ONLY ID tokens | 4/256 | 2/256 | 10/256 | 29/256 |
| ID_ONLY EOS | 0/32 | 0/32 | 0/32 | 0/32 |
| FULL ID tokens | 54/256 | 46/256 | 48/256 | 48/256 |
| FULL FORMAT tokens | 160/160 | 160/160 | 127/160 | 134/160 |
| FULL CLOSE | 32/32 | 30/32 | 28/32 | 26/32 |
| FULL EOS | 32/32 | 32/32 | 32/32 | 32/32 |
| FULL MIXED tokens | 11/32 | 6/32 | 0/32 | 0/32 |

Complete ID-span correctness is 0/32 in every arm/mode/split. FULL has no pure VALUE-token denominator: the value boundary crosses a MIXED token, so it is not reported as 0%. ID_ONLY has only ID and EOS roles. Under gold prefix, I's first error lies in the ID span in all 32 train/dev ID_ONLY rows, and in the first MIXED token in all 32 train/dev FULL rows. These observations diagnose the failed response contract; they do not increase free-generation scores.

**Execution accounting and preservation.** SMALL used128 optimizer/backward updates,3,456 generations and288 teachers:3,328 main generations plus128 independent replays,256 main teachers plus32 independent checks. Generated tokens total46,948 including EOS (1,975 in B); teacher targets total3,600 including EOS (400 in B). TINY remains separately16 updates/backward, four teachers and zero generations. Independent B performed no optimizer/backward calls. Charged A plus immutable run journals total656.538534833 seconds; call, time and update caps were respected. The old teacher216/replay128 and old1.0 blocked preparation were not rerun.

Protected source corpus/tokenizer/native references were independently checked before and after B; the typed readbacks are byte-identical to each other and the A preparation readback. Original failures, native weights/full Adam, raw failures and old reports remain preserved. B evidence is the existing scratch's `phase-b.md`, `endpoint-readback.r3b`, `replay-readback.r3b`, `teacher-final-readback.r3b`, `closure-readback.r3b` and `review-b.r3b`, with exact commands/exits in the corresponding logs.

Receipt SHA-256: A `bdab0859fe2a2568f240c11d6cc5f7317e167e5d5c68771243f8d37773129035`; B `df20736d138c4ceb85371097fdbdab6b19cdb0ea0bd2f94a23e6fd83ff9ea99c`. The source identity remains the frozen implementation commit; subsequent report-only commits do not change it.

The scoped snapshot contains13,540 files and548,945,643 logical bytes: new study7,713/474,782,053; evidence17/42,541,651; review77/18,488,541 (including independent TINY); root final-dispatch temporary fixture5,694/9,033,222; root TINY temporary fixture39/4,100,176. This snapshot precedes the small B phase-note/receipt and later report additions; those are distinct from the measured counts. It remains below1GiB. No old archive or global inventory was performed, and the shared Cargo target was reused.

One proposed next variable, **not executed**: lower LR from3e-5 to1e-5 while retaining inherited A512 Adam, both protocols, corpus, tape and protective rules. The hypothesis is that less retention damage may allow the output contract to improve before the stop. This experiment does not establish LR as the cause; another protective stop or no heldout exact-ID improvement would falsify the proposal's utility. No further learning is authorized by this review. The fixed signal thresholds are operational criteria, not statistical significance or proof of a fundamental128-update learning limit.
