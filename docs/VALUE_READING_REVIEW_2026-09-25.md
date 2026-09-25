# Value-reading isolation — independent A/B review

## A: prepared source, inputs and zero-call report boundary

**A_CODE_AND_INPUT = FAIL** for prepared source commit
`b0a68b60d40017ed2ec1216011542c0f415257ce`. Input construction and binding passed;
the report loses prior clean-TIME segments. No A authorization receipt was
issued, and no new model generation, teacher, optimizer or backward was called.
This is a diagnostic finalization defect, not a model-quality result.

The immutable v1 plan is
`artifacts/value-reading-20260925-main/value-reading-plan.r3b`, physical SHA256
`6814889c550f9f9064ca5aa5cdc69af069695b00f4744d20b81d5b5502248dd7`, canonical
policy `b3cf7393301ebeddbf60f9ffe0f7ccd16d60bc526e5f8c3f6acc119d9f03591b`,
source composite `70ad4fd0ba5dd05b8a6e9e12616c185f102bc6ca3c79f6ce45540459ddaccf91`.
The examined release executable is SHA256
`5d5c790e124ad5ebec585ca967016165921aa15f046b0581312f255d7abce8da`.
Original failed v7 and completed evaluation-only B remain separate and unchanged.

The independent Rust reader at
`artifacts/value-reading-20260925-main/independent_reader_a_v1.rs` uses the existing
binary codec, canonical native corpus reader, and tokenizer. Its semantic resolver
and metadata selection do not call the candidate builder. It compared all 96 O
episodes byte-for-byte with the bound native corpus and metadata, independently
reproduced the stable selected indices from the returned 192 per split, and
checked O raw request/answer/evidence/prompt/tokenizer/model bindings. Both splits
have six word pairs, 12 distinct semantic bindings with four views each, and each
word occurs 12 times. Train has 24 ID0 and 24 ID1 rows; dev uses ID0 only. These
are 12 correlated scene groups per split, not 48 independent scenes.

For all 288 requests the independent request-only resolver found exactly one
current entity/context record. U changes only the other record's value bytes;
S preserves the chosen evidence item exactly and removes the other item. All
other request fields and targets remain unchanged. Measured prompt lengths are
O/U 218 and S 167; target including EOS is 16 throughout, hence totals 234/183.
Provided counts are 2/2/1, excluded is zero. Roundtrips and training/generation
prefixes agree. C native physical SHA256 is
`1f5656d5c033a5cba3d271c3939aed66aaa14f4a00b11cc30f8081af5db28a90`, content
`a2a2b721c5982d883f12c7d6ae07969d3fb80827265655d3450eb3d9d8c0d851`, with
model18237/Adam3901. Native, tokenizer and three executable physical identities
were read and verified. No training loader or model computation was needed.

Existing unchanged-identity direct tests are reused: actual selection 1/1 in
37.91s, two-row boundary fixture 1/1 in 21.48s, and six-panel report/preflight
1/1 in 18.90s. Their synthetic outputs are not model quality. These cover the
255/256/257 bounds, malformed/ambiguous records, wrong RETURNED denominators,
strict invalid UTF-8, malformed citation scanning, model mismatch, missing
resolution and UNKNOWN. Prior numerical, Metal, teacher and optimizer reviews
were not repeated.

The independent actual-CLI counterexample uses preserved synthetic evidence at
`artifacts/value-reading-20260925-main/independent-time-fixture`. U-train has a
clean TIME segment with 16 returned calls and 300 seconds, followed by success
with 32 calls and 5 seconds. Together with the other panel and parity receipts,
the full history has 200 calls and 325 seconds. The frozen command
`target/release/replica-train fresh muon value-reading --root artifacts/value-reading-20260925-main/independent-time-fixture --phase report`
exited 0 and published **184 calls / 25 seconds**, retaining all 48 U-train rows.
Its report SHA256 is
`dd8e003d5caed94b054e62619768b92b361c83809e93e547e0c9b9e066eae4d6`.
The fixture has 192 synthetic U/S RETURNED rows and a parity receipt; no actual
parity raw is present. The old report accepts that missing binding too. Required
repair: verify every contiguous segment and its start/terminal link, sum all
segments, reconcile successful call usage with durable RETURNED observations
including parity, and reject missing, cancelled, corrupt or UNKNOWN histories.
Revalidate only this affected boundary after repair; do not repeat model runs.

The separate read-only inherited exposure audit is bound to the v1 plan by
`artifacts/value-reading-20260925-main/exposure-audit.txt`, SHA256
`4ff5d04ef2053f3f851341e6b25f0610abf4b0cc72e4b76cc8623ca950827f8d`.
Across selected train48, prior F64 exact exposures sum 6, FULL-fit3069 exact
exposures 384, and C256 exact exposures 40. Earlier other-system exposures sum
62; C256 other rows with the same semantic binding sum 280. Forty-six selected
rows have nine cumulative exact exposures and two have eight. The original plan
fields intentionally describe only C256 (40 rows once and eight zero); its
`semantic_other_exposure` means same-binding other rows in that suffix, not total
inherited or earlier other-system exposure.

Reader executable SHA256 at the counterexample is
`43d51e6701cae76fc03d6420706701dada4508c4df3b5b5d25fd9da05fb88134`; reader source
SHA256 is `8526ad5b909511c6a80a57d45a5319ec3e3afe796c3ad9c3c62637cf470412d7`.
It was linked with installed Rust against existing release dependencies, without
Cargo rebuilds, model/corpus copies or source-tree copies. Reviewer model/effort
was requested/configured Astra High through the agent tool; root live settings
remain separate and unchanged. Usage totals unavailable from the client are
UNKNOWN. The earlier 2,298,192-byte input-only intermediate reader executable was
overwritten during development; its input checks remain in the preserved final
v1 reader. Full preservation of every reader revision is not claimed. Shared
release net growth observed before this review is 23416 KiB
allocated; full build growth and peak remain UNKNOWN. No overall cost PASS.

B_RAW_AND_PARITY = NOT_RUN. DIAGNOSTIC_EXECUTION = PARTIAL (preparation only).
NEW_GENERATION / REUSED_ORIGINAL / NEW_TEACHER / OPTIMIZER / BACKWARD =
0 / 96 bound existing O rows / 0 / 0 / 0. ORIGINAL_TASK_FULL retains C word29/192.
ORIGINAL_C_W_QUALITY = FAIL_UNCHANGED; QUALITY_PROMOTION = NONE;
S4/S5/S6/GOAL1 = NOT_ACCEPTED. U/S scores and paired changes remain unobserved.

## Same A, affected repair validation

The user authorized an immutable-scope cap of **160 MiB**, preserving existing
artifacts and build outputs. Generation216, tokens6912, active900 seconds,
segment300 seconds and teacher/optimizer/backward zero are unchanged. The v1
plan and counterexample remain immutable; no model-call budget was reset.

The repair validates the complete contiguous segment history, rejects orphan or
unresolved starts/finishes and invalid stop conditions, binds each lane marker to
its last verified successful segment, and counts all clean-TIME segments.
Reporting also verifies all eight parity RETURNED rows and their resolutions,
and reconciles the main call count to 200. Revised runtime capture verifies all
old profile fields apart from the new executable hash. This also repairs the
preliminary revision's stale runtime-binary field before any model call.

Affected actual-CLI validation **PASS** on `src/muon.rs` SHA256
`1d6fcc81fdde7718203c13b2aaef4a5827625de1cc069a161c7dac6b5ab24c4a` and
`artifacts/value-reading-20260925-main/replica-train-value-v2-small`, SHA256
`bbedb487c576e6c614cffaa2944f52c236d4de56407e320ec18d76b2b40cafb8`
(15,054,160 bytes). The v2 plan SHA256 is
`6809d89ee31f05262e187849feff9488e0f960f5f2e5a2af41f9708469479723`, policy
`d503a7dc7db584419955b877e3b21522e25087b0956d4e05e76f9141c44fcc0d`, source
composite `80661ade54bdad172b54d51df1beebb6f570277681132eeea73c6ffc43b29eb5`.
The independent identity comparison found every prior input, native, tokenizer,
length, exposure and non-storage budget field unchanged. The full 288-case input
review is reused from v1, not repeated.

The independent fixture at
`artifacts/value-reading-20260925-main/independent-time-fixture-v2` includes 200
synthetic RETURNED observations and a clean TIME split. Running the actual v2
report CLI produced **200 calls / 325 seconds**. Re-reading produced the same
report without adding files. Nine actual CLI negatives all exited 1: missing
parity, corrupt parity, missing resolution, missing segment start, UNKNOWN,
cancellation, underreported calls, mismatched final marker and pending
publication. Each failure retained the saved successful report; restoring the
fixture made its final no-call reread succeed. These are synthetic boundary
observations, not model generations or quality scores.

Command: `artifacts/value-reading-20260925-main/independent_reader artifacts/value-reading-20260925-main/independent-time-fixture-v2 cli-test`.
Log `artifacts/value-reading-20260925-main/independent-repair-cli.txt` SHA256
`834fb90ab63bbe768ecf9bae6c4dbeac4eaad486c72ab83a7dc2c9aa54af4eb2`.
The final independent reader is 2,882,624 bytes, SHA256
`5ae1587f51b6df1ab7645713775cfc50cba26d9a995fa8e75235e04837781acf`; source
`independent_reader.rs` SHA256
`b9bc0b700c711ef968190371d8e3ea5472703d49c0f116937783f93be62735c3`.
It also contains the separate semantic scorer and fixed B selection readback.
The used v1 reader and exact source remain preserved separately.

**A_CODE_AND_INPUT = PASS** for repaired source commit
`c173e13cc39a6c6bdc3dd2fa1cff7633d54b9bc1` (remote equality checked by the
implementing/root workflow). After that freeze, the independent reader published
`artifacts/value-reading-20260925-main/review-a.r3b`, SHA256
`66ce75ef25d1eb92a5015e8e605158a753f13a47807abea7095964b4830e2d01`.
The receipt binds contract, exact source composite, plan policy and physical
plan, including all inputs; it does not hash this mutable A/B report as authority.
Command: `artifacts/value-reading-20260925-main/independent_reader artifacts/value-reading-20260925-main admit`.
New model calls still equal zero at A issuance. The registered parity8 and
U/S192 may now execute within unchanged call/time limits; B remains pending.

## B: actual observations and one fixed reproduction

**B_RAW_AND_PARITY = PASS. DIAGNOSTIC_EXECUTION = COMPLETE.** Source commit
`c173e13cc39a6c6bdc3dd2fa1cff7633d54b9bc1` and executable `bbedb487…cafb8`
remained unchanged. Independent semantic scoring checked all 96 reused O rows
and 192 new U/S rows against their requests, targets, tokenizer, strict decoded
tokens, actual EOS, provided IDs and raw bindings. All six score panels and their
word, pair, semantic-base and paired aggregates matched the canonical report.

The first independent B read exposed a reviewer-only accounting error: its
`errors` counter counted exceptions but omitted completed outputs without EOS.
S-train has six length-ended outputs. The used reader and source were preserved
as `independent_reader_pre_b[.rs]`; only the independent reader was corrected to
count abnormal finish/completion/EOS as well as exceptions. No product source,
model output or denominator changed, and no B generation had occurred. The
corrected complete read passed. These 12 S length-ended rows remain RETURNED;
they were not regenerated as replacement answers.

All denominators below are 48. `Errors` includes a missing normal EOS, even when
the inference call returned without an exception. Whole-value tests the complete
value prefix; it can be correct while the overall response syntax is malformed.

| Panel | FULL | Whole value | Exact support | Actual EOS | Errors | Malformed | Outside ID |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| O train | 16 | 16 | 48 | 48 | 0 | 0 | 0 |
| U train | 16 | 16 | 48 | 48 | 0 | 0 | 0 |
| S train | 0 | 8 | 0 | 42 | 6 | 48 | 4 |
| O dev | 10 | 12 | 36 | 48 | 0 | 0 | 12 |
| U dev | 10 | 12 | 36 | 48 | 0 | 0 | 12 |
| S dev | 0 | 10 | 0 | 42 | 6 | 48 | 6 |

Paired counts are **both correct / gain / loss / both wrong**, always with O as
the reference. B reproductions are excluded from these accuracy denominators.

| Change | FULL | Whole value |
| --- | --- | --- |
| O→U train | 16 / 0 / 0 / 32 | 16 / 0 / 0 / 32 |
| O→U dev | 10 / 0 / 0 / 38 | 12 / 0 / 0 / 36 |
| O→S train | 0 / 0 / 16 / 32 | 1 / 7 / 15 / 25 |
| O→S dev | 0 / 0 / 10 / 38 | 1 / 9 / 11 / 27 |

U leaves FULL and whole-value correctness unchanged in every row, hence in every
word and all 12 semantic groups per split. S has FULL zero in each of the four
words and every semantic group; its value gains and losses are mixed and its
aggregate value count is lower. The exact independently checked word/group
matrices are preserved in `artifacts/value-reading-20260925-main/independent-score.r3b`,
SHA256 `4d67772a1182b99c64140bf52f2325de01c024d40e74233a634bae7baf091a42`.
O/U train classify 16 wrong answers as the original foil word and 16 as another
word; dev has 12 foil and 24 other-word answers. S has 48 malformed outputs per
split, so none is reassigned to a well-formed foil/other-word category. S's
outside-ID reference is its one actual provided ID; its lower outside count does
not establish improved support, which is zero.

The single fresh-process B execution used one thread for Accelerate/Rayon and
the frozen v2 executable's `fresh muon value-reading --phase review` command.
Normal O/U/S each used train indices 0, 1, 9 and 17, covering all four words;
additional first eligible failure/query-mate pairs were U 2/3 and S 4/5. This is
16 distinct requests selected by the previously frozen rule, not 16 extra test
cases for accuracy. Exit was 0. Independent readback exactly matched raw token
vectors, text, EOS/finish, error and completion to the corresponding observations.
All six actual lanes' case digests, preparation/resolution records, native/source
bindings and complete segment histories were checked; each terminal was success
with no stop condition. No completed request was retried.

New generation calls are **216 = parity8 + main192 + B16**, with 96 original O
rows reused. New teacher/optimizer/backward calls are **0/0/0**. Actual generation
tokens total **3655/6912** and new input tokens total **41886**; U/S main192 alone
accounts for 3233 generated and 36960 input tokens. Sum of all six canonical
control receipts is **36.698421167 active seconds / 900**. Actual command wall
times are main **39.33 seconds** plus B **3.83 seconds**, totaling **43.16 seconds**.
The immutable B receipt's `command_wall_seconds=39.33` is main-only: the reader
matched the main logs' BSD time layout but omitted B's POSIX `real 3.83` line.
The correction here comes directly from the preserved command log; the receipt
is retained unchanged. Maximum observed RSS is **294158336 bytes**, and maximum
observed process memory footprint is **549159896 bytes**. Whole-system/build
peak and total build growth remain UNKNOWN, so overall COST is not PASS.

The task root plus every distinct externally located used executable measured
**158881534 bytes** before writing the B receipt and completing this report,
within the authorized 160 MiB cap. This count retains the failed v1 fixture,
used reader versions and unused build outputs; it does not treat original
referenced model/corpus as new copies. Final publication accounting adds the
receipt, remaining log bytes and this Markdown report. Direct final hashing
confirmed the original native, tokenizer, both O raw panels, original/evaluation
plans and immutable v1 plan unchanged.

Final publication ledger: task-root files **47373569 bytes** (including the B
receipt and completed logs), three distinct used executables outside that root
**111510896 bytes**, and this review Markdown **18022 bytes**: total
**158902487 bytes / 167772160 bytes**. The final report is counted explicitly;
referenced original model/corpus are preserved in place rather than copied.

Independent final readback log:
`artifacts/value-reading-20260925-main/independent-b-final.txt`, SHA256
`2b067f8b29d2db7b23aed6043872e89ac7f361a463c1f0c4e531a05e57aa771f`.
Final reader source SHA256 is
`6c575c68c21d4a6a2c86990d82963181ee3e0e3d0f116026c52dcc34923dddc1` and executable
SHA256 is `4b7dc8dcabe5aec26519fec4e6772b4493a24d87a98e5f9d170d9edc3b941543`.
Canonical main report SHA256 is
`d0f8a566000748c4c81978c7397b6906a848dd6b408b05f48150fd96d17342c6`.
The small immutable B receipt `artifacts/value-reading-20260925-main/review-b.r3b`
has SHA256 `c7bfdafb5f143ef0fa6e5c8f83e2a4ebd38bc4e256ed23f69562d91c6b5dc717`.
It binds the plan, source, A receipt, canonical report, independent score and all
six generated raw streams. No additional closure review or model call is needed.

CAUSAL_SCOPE: removing value competition did not improve this sample. Removing
the other record also changed count, length, relative position and input
distribution, and destabilized full-answer formatting. Selection removal alone
was insufficient; these data identify no particular circuit or single cause and
do not establish that copying is impossible. The 48 correlated views per split
are a small diagnostic, not 48 independent scenes.

One next hypothesis is that producing the full citation-bearing answer burdens
word-value readout. A separately authorized **FULL versus explicit VALUE_ONLY
output-contract comparison** could change that single contract while fixing
the model and evidence; it must retain separate train/dev and word/group
reporting. The hypothesis is weakened if VALUE_ONLY fails to improve heldout
whole-value correctness consistently across words/groups, or improves only
familiar train scenes. The new instruction is itself a distribution change,
so even improvement would not establish a mechanism. This is a proposal only;
no learning, new teacher/probe, loss/LR/seed search or extra generation is opened.
The original FULL quality requirement remains unchanged.

Final: A_CODE_AND_INPUT PASS; B_RAW_AND_PARITY PASS; DIAGNOSTIC_EXECUTION COMPLETE;
ORIGINAL_C_W_QUALITY FAIL_UNCHANGED; ORIGINAL_TASK_FULL C word29/192 retained;
QUALITY_PROMOTION NONE; QUALITY_APPROVED false; DIAGNOSTIC_ONLY true;
S4/S5/S6/GOAL1 NOT_ACCEPTED. W weight2 remains unadopted.
