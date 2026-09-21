# Citation continuation — independent B, 2026-09-22

**RESULT: EXECUTION_AND_RAW_REPRODUCTION_VERIFIED. MODEL_QUALITY: FAIL.**
The permitted ANSWER continuation finished its 3,040 new updates at absolute
step 7,424. Independent recount agrees with every saved evaluation. The final
value/citation/renamed-ID development gate fails; this review does not grant
candidate eligibility, further learning, confirmation access, or Goal1 acceptance.
No new source defect was established in this B review.

Reviewed product source: `c9c121077bbe1aadc546e344ab237d38728e3d3e`.
Source digest: `65cee12e4d3c0c4149a7b02c7026614eb93206c2c61f6bdcab97ce33d92bd0c8`.
This report is published after the implementation status commit
`2c9557c156a2196bd4f1089c3fe2d95675772b6f`; its report commit is distinct from
reviewed source. The immutable independent A report and approval were rehashed,
not rewritten or reissued. A's actual process tests are not counted as new B tests.

## Actual checks and authority

The reviewer performed two production `fresh answer-mean-review` commands,
with the frozen executable and the registered absolute ANSWER-MEAN root: first
without `--citation` for V16, then with it for VC16. Both returned exit 0 and
`matched16 generation16 optimizer0 teacher0 candidate_permission=false`.
There were exactly **32 new SMALL generations, 0 new teacher calls, 0 optimizer
updates, 0 TINY calls, and no confirmation calls** in B. These fixed prefixes
verify reproducibility; their 16/16 correctness does not estimate the full panel.

The independent Rust reader reused the earlier B reader's strict scorer,
training-only label resolver, native loader, tokenizer and prepared/resolved
journal verification. It read all 3,040 trace rows, 11 saved native endpoints,
all seven scheduled decisions, all 4,096 generation rows and 4,096 separate
teacher rows. Its standalone build and execution both returned exit 0. The
separate pure parity/usage/seal reader also built and returned exit 0. No Cargo
or broad suite was run for B; a zero-test run is not claimed as a PASS.
Existing immutable Rust rlibs were linked locally; no dependencies were fetched.
The readers' training-only resolver and expected answers never enter inference.

The checks covered exact source/policy/tokenizer/corpus/tape binding, parent
physical bytes, finite native weights/Adam and update diagnostics, step/sampler
and token clocks, all ordered episode IDs/content/answers, prompt digests,
normal greedy token decoding, EOS/errors, full/joint and citation fields,
independent teacher statistics, final/native model agreement, cumulative
usage, and the absence of missing/extra evaluations and unresolved attempts.
All **8,192 prepared/resolved call pairs** belong to the expected generation or
teacher row, case, model, panel, policy, ordinal and attempt zero. No UNKNOWN,
missing result or runtime error was found. Normal length endings remain in the
full denominator: VC64 at 4,416 has two such rows, EOS 62/64, full 0/64 and
runtime errors 0. They were not regenerated or converted into successes.

## Actual learning and durable state

| Item | Independently verified result |
| --- | --- |
| Inherited / new / total citation updates | 32 / 3,040 / 3,072 |
| Absolute update range | 4,385 through 7,424; next update not executed |
| Durable segment endpoints | 4,385; 4,416; 4,480; 4,608; 4,736; 5,120; 5,632; 5,888; 6,400; 6,912; 7,424 |
| First saved / fresh process input | 4,385 saved; next segment binds that exact file and resumes at 4,385 |
| New input / target / padding tokens | 3,623,680 / 231,040 / 97,280 |
| Final cumulative input / target tokens | 8,710,144 / 303,104 |
| V / VC0 / VC1 draws | 12,160 / 6,080 / 6,080 |
| Batch / loss / LR | 8 = V4 + VC0 2 + VC1 2; answer-mean CE; constant 0.0003 |
| Resume objective | family 6, normalizer 2, first-target weight 1; QE; same tokenizer and original tape |
| Discarded/uncommitted optimizer/input/target | 0 / 0 / 0 from exact trace and control agreement |
| Training generation / teacher calls | 4,096 / 4,096 |
| Final terminal | `Finished`, `FINAL_QUALITY_FAIL_AT_7424`, `resume=false` |

The actual tape equals the original tape's suffix after its consumed first
32 citation updates. Each update has 1,192 input tokens and 76 target tokens,
including EOS. Each V task has 2 targets; each VC task has 17. Independent
arithmetic reproduces the token CE and answer-mean CE diagnostics, and the
training objective uses the eight-example denominator. Trace step and sampler
clocks remain cumulative; all 136 Adam moment tensors are finite. Parent
weights/Adam were loaded from the exact preserved ANSWER4384 file, not reset.

New exposure histograms are V: 128 cases seven times and 1,408 eight times;
each VC pool: 64 cases three times and 1,472 four times. Including the inherited
32 updates gives every V case eight exposures and every VC0/VC1 case four.
No added data, reordered tape, tokenizer, objective coefficient, LR search or
extra optimizer step was observed.

The actual retention decisions recompute from the first fixed V64 rows:

| Step | V full / ALL4 | Warning streak before → after |
| --- | --- | --- |
| 4,416 | 59/64; 12/16 | 0 → 1 |
| 4,480 | 62/64; 14/16 | 1 → 0 |
| 4,608 | 63/64; 15/16 | 0 → 0 |
| 4,736 | 64/64; 16/16 | 0 → 0 |
| 5,120 | 63/64; 15/16 | 0 → 0 |
| 5,888 | 64/64; 16/16 | 0 → 0 |
| 7,424 | 63/64; 15/16 | 0 → 0 |

Every previous-decision digest and raw-panel digest agrees. V runtime/EOS
errors are zero at these steps. The unchanged original 60/64 and 12/16 parent
baseline does not add a warning. No severe or persistent-retention stop was
triggered. The historical parent's different `QUALITY_REGRESSION` and
`resume=false` remain untouched.

One earlier relative-root command failed before admission with
`continuation immutable parent/policy`. The preserved log contains no
`TRAIN_END`; its reported wall time is 3.64 seconds. The source checks that
root before segment registration, and the first admitted segment begins from
the unchanged 4,384 parent. The implementer reported exit 1 and zero model calls
for that pre-admission failure; B verified its log and boundary rather than
rerunning it. Subsequent 11 command logs contain their expected `TRAIN_END`,
and their saved controls/terminals agree. The implementer's exit-0 reports for
those original training processes are distinguished from the reviewer's own
exit-0 commands.

## Final quality, one model at step 7,424

QB = both queries correct for a given assignment; SB = both assignments correct
for a given query; ALL4 = all four questions for the same semantic orbit.
Strict full answers require normal EOS and no generation error. No trimming,
number repair, forced output or oracle-selected support was used.

| Panel | Full | QB | SB | ALL4 | First value | Exact syntax | Correct support | Foreign provided-ID failure | EOS / runtime errors |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| V dev | 511/512 | 255/256 | 255/256 | 127/128 | 511/512 | N/A | N/A | N/A | 512/512; 0 |
| VC dev | 492/512 | 236/256 | 241/256 | 113/128 | 509/512 | 512/512 | 495/512 | 17/512 | 512/512; 0 |
| ID-only renamed dev | 495/512 | 240/256 | 243/256 | 116/128 | 510/512 | 512/512 | 497/512 | 15/512 | 512/512; 0 |
| Fixed VC train128 | 127/128 | 63/64 | 63/64 | 31/32 | 128/128 | 128/128 | 127/128 | 1/128 | 128/128; 0 |

All final citation outputs contain one parseable eight-digit ID. The foreign
ID count measures parsed IDs absent from the supplied two records; provided
and correct-support counts coincide here. VC has three wrong first values
and 17 ID errors; renamed dev has two wrong first values and 15 ID errors.
These are separate field counts, not interchangeable first-error categories.
ID-renaming paired exact correctness is **477/512**; no identical citation was
retained when the expected ID changed. This diagnostic is not SWAP_BOTH.

The V development condition passes. VC fails ALL4 (113 < 116), support
(495 < 508) and the zero-foreign-ID condition. Renamed dev fails support
(497 < 508) and zero foreign IDs. Thus the **joint development gate fails**
even though both citation panels exceed 488 full answers. At 5,888 the saved
joint gate also failed. The conditional full training panels (`old512`,
`new1024`, `VC0train1536`, `VC1train1536`) were correctly **NOT_RUN**; train128
is a fixed diagnostic subset and does not establish the full fit gate.

The parent VC64 had full 0, first value 50, EOS 64 and syntax/support 0.
Final citation grammar and full answers visibly improve in the measured
panels, while remaining ID/support mistakes still prevent acceptance. This
is neither unchanged quality nor a completed citation baseline. A narrow
next necessary quality task is reliable generation of the selected record's
provided ID while retaining scalar selection: 17 and 15 final dev rows cite
foreign IDs despite valid syntax. This is an observed failure boundary, not
proof of a particular attention, tokenizer or optimizer cause, and no further
experiment was started or authorized by this B result.

The existing separate gold-prefix teacher rows were independently rechecked;
there were **zero new teacher forwards/backwards**. NLLs are diagnostic token
means and do not replace normal greedy metrics or the answer-mean training
objective.

| Final panel | First-value NLL | After-value EOS minus citation logit | Grammar-prefix NLL | ID-token NLL | Gold-prefix EOS NLL |
| --- | --- | --- | --- | --- | --- |
| V | 0.0055406050 | +14.9336468834 | N/A | N/A | 0.00000687360 |
| VC | 0.0194566795 | −12.4198790209 | 0.00003307188 | 0.0164094868 | 0.00000316905 |
| ID renamed | 0.0141608123 | −12.3931354638 | 0.00003462385 | 0.0115123997 | 0.00000307266 |

Each citation development panel contains 3,072 grammar-prefix and 4,096 ID
teacher tokens. Each ID position below has **512** contributing observations.
All target/EOS identities, byte-span overlaps and full-precision saved means
were verified before these rounded reporting values were produced.

| ID digit position | VC gold-prefix token NLL | Renamed gold-prefix token NLL |
| --- | --- | --- |
| 1 | 0.0011393536 | 0.0024816892 |
| 2 | 0.0110495947 | 0.0042820448 |
| 3 | 0.0020368693 | 0.0050651583 |
| 4 | 0.0382836232 | 0.0084554717 |
| 5 | 0.0150579902 | 0.0114216872 |
| 6 | 0.0193962026 | 0.0100499619 |
| 7 | 0.0270502624 | 0.0358560589 |
| 8 | 0.0172619983 | 0.0144871260 |

## Calls, timing, identities and preserved evidence

Training raw contains 49,504 free-generation tokens. The implementer's parent
parity used 32 generations and 64 tokens; B's final parity used 32 generations
and 304 tokens. The study's combined observed count is **3,040 updates,
4,160 generations, 4,096 teachers and 49,872 free-generation tokens**.
These are not training target-token counts. B adds no TINY optimizer usage;
the previously accepted aggregate TINY budget remains 128 updates.

Saved segment elapsed time sums to 1,619.194935375 seconds. Parent observation
control time is 2.45858 seconds and B observation control time is 2.798803834
seconds, giving 1,624.452319209 seconds in that explicitly defined sum. It does
not include compilation, pure readers, pre-admission failure, all surrounding
CLI validation or wall-clock idle time. The original `/usr/bin/time` logs
report maximum process RSS up to 7,225,114,624 bytes, below the 16 GiB limit.
This is not a storage/backend performance benchmark.

| Artifact identity | SHA-256 |
| --- | --- |
| Frozen training/inference executable | `9f150bf0d5855c2956db8802ee91f3038389765afaaeddf1d336cd2868981d31` |
| Preparation | `5be75f097064ac5156409adaab0ebfe23c5321e59534fc0aa35422f7af8205f2` |
| Selection | `8bd2b0eb35016689ce44aa5e093dae690a2ecaf4dda2841a76e24a2d0fb313a7` |
| Native plan physical bytes | `ea42ffe7b08e2ef3c6c70b57a319ba70bb95cc85069a8a81868b3d3d5a6d9026` |
| Plan canonical content digest | `f0577429e1956666ae3aad3c5a0ebf39ed47e8b3865e8b01225820a3774700d7` |
| Corpus | `80b43942dc4854e114558f17a2b5d71e1ad0c71685511ded7c9b4457d4091c81` |
| Tokenizer mapping identity | `ec945ee5f3cbd87992bdfa13f199de2a671b94b64337dff04d85e01d982ab9ef` |
| Original tape digest | `13e249a6035a6edb06e0e816687135577ce61db9e3e2d228034849d843029f65` |
| Exact 4,384 parent / new initial file | `838754d751a71e4fe7971a4127a76c17a0ed05c4d8efdf524c79248324c7a1dd` |
| First durable 4,385 physical file | `d30773494b513ca4c6c8c3634f2903459574d7d5085b62c9f1dd3f7c264f6bd4` |
| Final 7,424 physical resume file | `8c2e9b0f670be09acbae40efb6e2d4cb4e5a3c01315b0d7e3a51aa467270ec47` |
| Final manifest serialized weights section | `66c16ddf00396df00fae1266c628c95384a94a2a2f195eabc4eb30352634b875` |
| Final tensor-content identity | `d9ed4e38e815949a3297b17d059e2de738b501fd5e5ecfef76eb2b6480daaa11` |
| Final evaluation model identity | `1d071953549a7b5da4cc88c25f364b104a824f6d8232255ccc1448306eb84225` |
| Final Adam136 digest | `3a2c0e4846b62c1d40ab051e8222964b0346baffe8b83ee43c3929c93fb42006` |
| Final training-state digest | `6f96023a880e9e69ddc310e685d440ed3c45c104e4a36d4be668d997643cc505` |
| Final evaluation checkpoint physical file | `ad545fa171f5877b5d8b325d00fb1e35370e2000132540b4cb33e9baf8653461` |
| Final value512 raw | `2a2e96176891533038c3b521d383aefcbbb9b987702e38fce495060959689265` |
| Final citation512 raw | `8c3573fca42976eb7e3dd055aaa8fc802d567299d2f7bafbdcc96241d1c390ca` |
| Final renamed512 raw | `a9dc18374afa9b4d72a742dccf7291eee22fe19e2c21aad3be646b000bd8ffd3` |
| Final train128 raw | `ff42e1eb4d99ada4a6e19f6da190e2bf4daffa89ca319b3d7cd35c332aa68a36` |
| Final immutable decision | `b053b54b1c3f30155aaa0eb4b0b4bcd24afee1508de8c83362952aa4a8a9e6ea` |

The `TRAIN_END sha256` label denotes the manifest weights section, not the
physical native file. The final evaluation file and final resume file have
different physical hashes but identical verified model/tensor content at the
same step; each panel binds its actual evaluation file, and fresh B inference
loads the terminal's final resume file. The implementer's earlier chat label
that called `66c16...` a physical hash was a reporting mistake, not a changed
checkpoint.

Authorized local evidence paths are under
`artifacts/citation-continuation-20260922-study-02/ANSWER-MEAN/` (native corpus,
metadata, tokenizer, plan, initial, segment starts/controls/traces/finals,
evaluation rows, separate teacher rows, call journals and decisions),
`artifacts/citation-continuation-20260922-study-02/` (preparation, selection,
immutable A approval and new parent/B observation receipts), and
`artifacts/citation-continuation-20260922-review/` (commands, scratch readers,
independent recounts and implementation diff). The frozen executable is
`artifacts/citation-continuation-20260922-executable-02`.
These local original artifacts and raw bodies are not published by this report.

| Independent evidence in review directory | SHA-256 |
| --- | --- |
| `read_B_continuation.rs` | `3857917b1b372a65bff70c0cd5946cec0c72c18c9eda65035624002802e2d2ac` |
| `read_B_continuation` | `cb67453b369ed7231893311d8d33c51075ad3127e954cabe5ef60500223aa500` |
| `B-recount-01.log` | `589c0c51857476976a51530361a1e2f1a982f93b2b208dde0048d7ab307a515c` |
| `B-recount-scores.r3b` | `ecc3fbbb5586b250bfe877edabb7174128c730479526003da0dae4edcc5b91ab` |
| `read_B_parity.rs` | `6055dd55f1fb5c8a1ee97a4e9c3327c41d6e02482dee7e1404b504e89d4af187` |
| `read_B_parity` | `60fc66220f119ba5e59505b7b545a631983ee26f96cf3eda2815e667cb3185f8` |
| `B-parity-recount.log` | `bbede3d61a958900cae95fa66690b478f89fe7af372fc513d59ee1fd45b5b4e4` |
| `implementation.diff` (initial HEAD `a4e113ee3a04fbc176cf6690c6ec9c3f70f66374` to reviewed source) | `52aea6f4a827d58755c908359e9e1d43d78ae95e79336d414ca2f7d43b00d5b7` |
| `B-value16.stdout` and `B-citation16.stdout` individually | `c052aeb2ed2b03fdca2c8242ca653d1d8c01b1d66b07ce1f1136c5b42c7dca6d` |

The native reader links the immutable earlier pure Rust library
`artifacts/answer-mean-citation-20260922-rereview-01/target/release/deps/libreplica_v3-93d50c7b7eeb4433.rlib`,
SHA `60c6929aedae8697379847f0953b535f7eba1e43fe936c7c8f014170e8b448a5`;
its unchanged codecs are used only to read existing schemas. Actual generation
uses the candidate executable identified above. Commands are reproducible as:

```text
VECLIB_MAXIMUM_THREADS=1 OMP_NUM_THREADS=1 artifacts/citation-continuation-20260922-executable-02 fresh answer-mean-review --root /Users/seo/Projects/Replica-v3/artifacts/citation-continuation-20260922-study-02/ANSWER-MEAN
VECLIB_MAXIMUM_THREADS=1 OMP_NUM_THREADS=1 artifacts/citation-continuation-20260922-executable-02 fresh answer-mean-review --root /Users/seo/Projects/Replica-v3/artifacts/citation-continuation-20260922-study-02/ANSWER-MEAN --citation
VECLIB_MAXIMUM_THREADS=1 artifacts/citation-continuation-20260922-review/read_B_continuation
artifacts/citation-continuation-20260922-review/read_B_parity
```

The original protected roots contain the same 11,607 files and exact contents
as the pre-work manifest, digest
`4a1b33d4f543b5de04f02a07808b85c871f412ffa8b00e78d8210ff6bf8dded4`.
This includes original 4,384 failures/Adam/raw and accepted scalar 4,352
confirmation. A report hash remains
`7428b1052369901f0d3faa27bf408e4243e7335d36f3ab9d8ba35fe66038c709`;
A approval remains
`dd48089004e5f18ebcf33661eb1c5eae444d28db3a3cd455a4adce456f9847e2`.
The citation seal owner and all three linked study registrations were read;
all have no confirmation attempt. Sealed corpus bytes were hash-checked only,
never decoded into questions or supplied to a model. No previous terminal,
comparison, A approval, candidate or raw row was rewritten.

**Final judgments:** code repair remains independently A-accepted; B actual
execution/raw/native/reproduction checks PASS; citation joint quality FAIL;
candidate NONE; confirmation NOT_OPENED; conditional full-train and full-QA640
NOT_RUN because their prerequisite did not pass; S4/S5/S6 NOT_ACCEPTED;
GOAL1_READY=false and GOAL1_ACCEPTED=false. This normal negative endpoint is
closed; no automatic extension or replacement study was started.

NEW FILES: this report and ignored reviewer Rust/readout/observation artifacts.
Product source/tests/Cargo files changed by B: NONE. Only this report is staged
for publication; its commit/remote SHA is delivered separately after push.
