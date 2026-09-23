# Independent B addendum — old QA640 diagnostic

**QA_DIAGNOSTIC_INTEGRITY: PASS. QA_QUALITY: FAIL,0/640.** This reviews only the
new640 saved rows and their execution records. It adds no model, teacher,
optimizer, backward or TINY calls, and does not repeat the initial B or change its
receipt. No candidate, confirmation, S4/S5/S6 or Goal1 acceptance is granted.

Reviewed source remains `c502e66872ad6abdf463990c5992061d3ec3a19c` and the frozen
production executable remains
`ffffb7f2f2dab68f04a73d579f65fe8d55b9177c00244705d7bd12788c472713`.
The diagnostic used the same final14336 physical native
`15015900a3c91a6b8ed87e9c2ecb8f7faf124849e5dcf8f40203cddf267e30d9`.
Its `FINAL_BRIDGE_QUALITY_FAIL_AT_14336 / Finished / resume=false` remains unchanged.

The initial [B report](INSTRUCTION_BRIDGE_COMPLETION_REVIEW_B_2026-09-24.md),
published at `f8bb36e990e80ae27098a3d1bff1d44872d8515f`, remains hash
`96f9f65046cdf073130b542f7af4a410f3fc44bcd883f0b33f4e45d28dc9f809`.
The bound `review-b.r3b` remains
`41694366b243fc648f33e8e1eeb509b9b4bec003165cdcf6aacd37e2f46d5852`.
This addendum has its own report commit, separate from the reviewed source.

**Actual diagnostic and independent verification.** The implementer's two frozen
`fresh qa-bridge-qa --study ABS --diagnostic` commands, with `--transfer` for the
second panel, exited0. Each has one completed segment;640 generation calls are
recorded, teacher/optimizer0. CPU/F32/Accelerate, one compute thread, normal
greedy/strict UTF-8/EOS, QE, context2048/max128/timeout120000ms were retained.
Original system, query, evidence, answers, order and tokenizer inputs match the
prepared old corpus/transfer/metadata hashes and ordered-case identities.
Expected answers did not enter native generation.

The independent Rust reader reused the existing QA counter and separately
implemented strict/individual-ID scanners. It verified all640 raw rows,1280
prepared/RETURNED records, segment start/final and aggregate final records,
prefix/file hashes, token usage, prompt/byte/token binding and stored scores.
All calls are attempt0, RETURNED once, with no pending/UNKNOWN, retry, runtime
error, malformed UTF-8 or missing row. Length stops remain failed full answers,
even though the generation call itself returned normally. The decoder outputs
and native/initial-B/source identities were preserved.

| Panel | FULL | EOS | Length stop | Runtime / UTF-8 | Outside-ID rows | Parse-failure rows | Generated tokens | Active seconds |
|---|---:|---:|---:|---:|---:|---:|---:|---:|
| Primary512 |0/512|225|287|0 /0|1|17|47311|127.017503208|
| Transfer128 |0/128|75|53|0 /0|0|2|10203|30.230867291|
| Combined |0/640|300|340|0 /0|1|19|57514|157.248370499|

Exact nonempty support and wrong-provided-support counts are0 in every bucket.
The outside ID is a primary B-bucket row; no outside row also has a malformed
citation. Strict-empty citation sets occur in494 primary and126 transfer rows;
an empty set is not a correct answer, including in G/H.

| Bucket | Primary FULL /64 | Primary EOS | Primary length | Primary parse | Transfer FULL /16 | Transfer EOS | Transfer length | Transfer parse |
|---|---:|---:|---:|---:|---:|---:|---:|---:|
| A |0|48|16|2|0|8|8|0|
| B |0|41|23|2|0|11|5|0|
| C |0|21|43|2|0|9|7|2|
| D |0|23|41|4|0|8|8|0|
| E |0|16|48|1|0|9|7|0|
| F |0|5|59|1|0|1|15|0|
| G |0|47|17|3|0|16|0|0|
| H |0|24|40|2|0|13|3|0|

G's no-evidence, missing-target and ambiguous-evidence subtypes score0/24,0/20,
0/20 on primary and0/8,0/4,0/4 on transfer. This original four-view dataset has
no separately defined unresolved/resolved complementary G-pair denominator;
the resolution map is empty, not a pass. H fixed-answer emissions and strict
answers are0/64 and0/16; non-H fixed-answer emissions are0/448 and0/112.
Excluding H, full accuracy remains0/448 and0/112.

**Old-view joint denominators.** For each bucket, primary has16 bases and
16 planned pairs for each relation01/02/03; transfer has4 bases and4 pairs each.
Every pair-both score is0; ALL4 is0/16 or0/4 per bucket. QUERY_BOTH is not defined
as the newer complementary-query metric here. The actual changes within these
old views were also counted, so an unchanged input is not represented as a
successful counterfactual test:

| Buckets | Primary changed gold01 / evidence02 / question03 | Transfer changed gold01 / evidence02 / question03 |
|---|---|---|
| A–B, each |16 /0 /16|4 /1 /0|
| C–F, each |16 /8 /16|4 /2 /0|
| G |0 /2 /16|0 /0 /0|
| H |0 /8 /16|0 /2 /0|

The accepted protected11264 [closure review](CITATION_PRECISION_CLOSURE_REVIEW_2026-09-23.md)
and its existing recount are reused:0/640, EOS440, length200, runtime/UTF-8 errors0.
The same input hashes and ordered cases are verified; old model generation and
raw recount were not repeated. Final14336 remains0/640, with EOS140 fewer and
length stops140 more. Parent12800 QA was never measured, so this comparison
does not isolate the effect of the latest1536 updates. The old combined active
time remains UNKNOWN and is not substituted into the new run's measured totals.

**Closed usage after this diagnostic.** SMALL optimizer1536/1536,
generation8780/9216, teacher8064/8192, additional backward0; input2064384/4000000,
target162816/300000, padding233472. Generated tokens179324 and registered
active2532.134917416/7200s include this QA's57514 tokens/157.248370499s.
TINY remains26 optimizer/586 generation/360 teacher. No remaining numerical
headroom authorizes another training, replay, candidate or wording change.

The Rust helper compiled and the final reader exited0. Its first read-only run
failed a reviewer-only assumption that the provided-ID list was already sorted;
the corrected set comparison preserves evidence order and all original files.
That failed log is retained, with model calls0. No product validator or score was
relaxed. No new test suite or model reproduction was run for this addendum.

Local evidence: `artifacts/instruction-bridge-completion-20260924-review/`;
raw/segment/score records remain under the existing `instruction-bridge-completion-20260924-study-final`.
Original raw contents and models are not published.

| Evidence | SHA256 |
|---|---|
| Primary raw | `55eb005ffff9f69fa94e27ff1d55cfd982d1f8144743c08c6a617e57871cf867` |
| Transfer raw | `9b997776870a17a2d243251e03aade274fdfa7d494e7c1471978c7f68fc3bc8a` |
| Primary score | `5d34a65de515d5ac7f6557cdc52e2d261eb081a3b71515d4760665790f20f982` |
| Transfer score | `1f0e1d3ceff53eca9e5d988d0f4f9f65be0f4dd6c7559b8f40ed69402b25b43a` |
| Independent addendum recount | `d8cdfdfff7935021526ae9f1b79f8e69f33fd051286df4481d1e86e6e1b7d918` |
| Consumed-file preservation manifest | `ca341ba808f26fab53f66784c5268eb917bd58f43ba137cb3e3447aaf7a81a10` |
| Independent reader source | `95388ccd01d32892ccd27413b7a99a55af951aafe0bf1afa4033ce41451781f9` |
| Independent reader executable | `b74f0f9199edd45aaa27bf341d1fb75fa9949e2d037d1ed31abbe88f3772020a` |
| Final reader log | `bcd8e1b5e0fb78c4bbf5be9cbb96e91cb3fa49cfe3820e3d5d27063aeac87f52` |
| Reused protected11264 recount | `1aec55f53eeb5f8bbaa4ac98b8018000ff298ddf50485c6fe738ac875990651b` |
