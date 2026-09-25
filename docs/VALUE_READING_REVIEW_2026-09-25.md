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
