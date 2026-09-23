# Independent closure — QA word values

**CODE / A: PASS, reusing verified unchanged execution evidence. B RAW / GUARD / USAGE: PASS, independently recounted this run. No confirmed code defect was found in the reviewed scope.** The model failed the retention guard; this is not acceptance of word quality or the full B reproduction stage. No product patch is requested.

Reviewed candidate: `c1f82a719dcd72ecca2e7b8024384562423f37c2`.
Review-start report HEAD and directly checked remote main:
`e1eff24e7f105c2421d5f74a0b3ca45e62e06c93`.
Candidate diff SHA-256:
`0af038a8cbbce5413f78959587ca0bf77164d90dbcbfdc9465ab349efbac4dfa`.
Frozen production binary SHA-256:
`d8e20a6b304fc9d1575b7ab43dca557f276486c3515a60b790f5d0468940b3b4`.
The commit adding this report is a separate report-only publication; its full
commit and actual remote SHA are supplied after publication.

## Requirements and production path

The shared contract and reviewer instructions for R3-QA-WORD-VALUE-1.0 were
read. Review was limited to `word_value.rs` and its changed citation, binding,
fresh-command and teacher-observation connections and direct tests. The
repository lean-development skill was applied. Product source/tests/Cargo match
the candidate. Cleanup remains cancelled, with no rollback of accepted
finalization, inventory, deletion, movement, compression or new cleanup work.

The word generator and request-only resolver remain in the training executable;
the product inference path receives neither expected answers nor a word/ID
output restriction. The complete-string scorer separates FULL/EOS, value,
individual citation support, malformed syntax and independently valid outside
IDs. `word_pass` and the retained joint gate consume those distinct counts.

The frozen split uses 192 train and 48 heldout semantic groups, with 30 unused;
train1536 and dev192/renamed192 are distinct from the retained train6144/dev3072.
Renamed dev shares the 48 scenes and changes IDs only. The 3072-row tape splits
each parent tape row into two four-example reviews and adds two distinct word
query pairs. ANSWER reduction still divides by examples, with prompt/padding
excluded and EOS included; token accounting remains separate.

The child policy preserves parent14336 weights/Adam/clock/QE/tokenizer, LR3e-5
and the parent's closed state. Evaluation scheduling, fixed64 retention guards,
single conditional fit and terminal-state restrictions were traced through the
actual caller. Word QA requires a normal complete scheduled endpoint, its
bound independent B and reproduction evidence. Confirmation rejects the word
profile before reading a seal. No S4 or additional-learning authority is added.

## A evidence reused

The [independent A report](QA_WORD_VALUE_REVIEW_A_2026-09-24.md), its preparation
reader, logs, receipts and frozen executables were checked against the candidate.
The data/scorer/gradient/typed-gate results in `direct-final.log` contain three
actual passing tests, not a zero-test invocation. The independent native process
test contains one pass for continuous2, fresh-process1+1 and evaluation-only2+0,
with identical weights/Adam/clock/raw. The final confirmation-denial test also
contains one pass.

The process test ran on `915e2f04f3294cccc73f0d12f058589e6408a462`; its binary
hash is `08bb95b9223c003731dc7bdf4f30f05fa65f46fdad8f919f53a09707e867c341`.
The subsequent candidate changes only word confirmation denial and its direct
test, whose final binary hashes to
`1fa71c04826415e5a0184709daf597380922100d3bfb83b2d0ada8a00f1f4457`.
This preserves the scope of the earlier process proof instead of claiming it
ran on the final binary. No A test or earlier accepted model observation was
repeated in this closure. Existing TINY totals remain14/276/276; the three
previous synthetic backward calculations are not new model work.

## B executed this run

The existing independent Rust raw reader/scanner was inspected and copied into
`artifacts/qa-word-value-20260924-review-closure/`. Only its output directory was
changed. Product scoring functions were not substituted for its request-only
answer check or separate outside-ID scanner. The original reader and receipts
were preserved.

Installed Rust1.98.1 compiled the isolated reader against existing locked
release rlibs (`replica_v3`, `candle_core`, `serde`, `sha2`, `zstd`), offline and
without Cargo/dependency changes. **Build exit0; reader exit0.** Runtime used
`VECLIB_MAXIMUM_THREADS=1 OMP_NUM_THREADS=1 RAYON_NUM_THREADS=1`.

```sh
rustc --edition=2021 artifacts/qa-word-value-20260924-review-closure/read_B.rs --extern replica_v3=target/release/deps/libreplica_v3-5f74ea5b6a70fcf8.rlib --extern candle_core=target/release/deps/libcandle_core-c172507719469e7f.rlib --extern sha2=target/release/deps/libsha2-961a6422faaaf604.rlib --extern serde=target/release/deps/libserde-2c76a0458361e7f8.rlib --extern zstd=target/release/deps/libzstd-44b73ca4f092671b.rlib -L dependency=target/release/deps -o artifacts/qa-word-value-20260924-review-closure/read_B
VECLIB_MAXIMUM_THREADS=1 OMP_NUM_THREADS=1 RAYON_NUM_THREADS=1 artifacts/qa-word-value-20260924-review-closure/read_B
```

This actual execution checked all720 stored generation rows,320 teacher rows,
128 committed updates, both native states/finite Adam,640 evaluation call
journals, source/policy/model/prompt bindings, strict token decoding and EOS,
complete query/assignment pairs, error rows and exposure/token accounting.
Teacher gold-prefix NLL remains diagnostic; its value-overlap tokens are not
reported as a pure value-byte loss. No forward, generation, teacher, optimizer
or backward computation was invoked.

| Completed14464 screen | FULL /64 | Value | Correct support | Outside-ID / malformed rows |
|---|---:|---:|---:|---:|
| Numeric value |64|64|—|—|
| Numeric citation |62|63|63|1 /0|
| S1Q1 |24|64|24|39 /1|
| Word |0|12|0|60 /4|
| Word renamed |0|12|0|64 /0|

Each screen has EOS64/64 and runtime errors0. S1Q1 falls from parent FULL64
to24 and ALL4 from16 to3 on the same fixed64 cases. The loss of40 exact answers
exceeds the immediate16-answer retention guard. Word0 did not itself trigger
the stop. The final native step14464 is Finished / SEVERE_RETENTION_REGRESSION /
resume=false, physical hash
`152c18bc1656e2fbbb7889fd3bbae826b15d33abb5d012fc4ab6985b3302d6f5`.

Historical SMALL usage independently recomputed here is optimizer128,
generation720 (parent400 + endpoint320), teacher320; training input186752,
target14976, padding19072; generated tokens11655; active159.388163415 seconds.
**This closure adds optimizer/generation/teacher/backward0/0/0/0.** Native reads
and stored-raw rescoring are not model observations or new learning.

The new recount and protected manifest are byte-identical to the previous
[B guard-closure evidence](QA_WORD_VALUE_REVIEW_B_2026-09-24.md), confirmed by
`cmp` exit0 for both files. Consumed protected hashes match before/after.

| Local closure evidence | SHA-256 |
|---|---|
| `read_B.rs` | `72ca6f05d95d91d6afc73b84aa3b95b346b4688bb81b49465665a8a31619feca` |
| Reader executable | `a936d162e806366490415677aa4e63ac88cb2ba187dd94ae2370dd858893a649` |
| Reused independent scanner source | `08c5a0afddaca4b5d2952510f0f0c8aa57c5a517792ecc2c54ef9d10d0cc39b8` |
| `B-recount.r3b` | `7308c08c3eb96707fc3e02273d8999a62ebacdae3543c6ec2d9882ccb3a122a7` |
| `B-protected-manifest.r3b` | `605e9d55152ac56acf66046ff3c74b9f324b181aa6b68f1874da9c5093ce10e0` |
| `recount.log` | `62411cb3a59659e954ac1365ca031de366646ff8759ee40001794a343f511872` |

## Separate final decisions

| Scope | Verdict |
|---|---|
| Code / preparation A | PASS, unchanged execution evidence reused |
| B stored-run integrity / guard / usage | PASS, actual independent readback this run |
| Full B fresh-process generation | NOT_RUN, early retention-guard endpoint; no B acceptance/QA receipt issued |
| New word quality | Not accepted; observed word/renamed FULL0/64 |
| Full word dev192 / train-fit1536 / retained six-panel joint gate | NOT_REACHED, not PASS |
| QA640 | NOT_RUN, guard stop has no subsequent QA authority |
| Confirmation / S4/S5/S6 / Goal1 | NOT_ACCEPTED; seals unopened |
| Cleanup / rollback | CANCELLED /0; prior accepted finalization preserved |

No current code-backed defect or necessary additional regression was found.
The early failure is preserved, and its unused budget is not permission to
resume, replace the candidate or add another study. Only this report is
published; original product/tests/Cargo, models/corpus/raw, earlier reports and
the existing untracked `.DS_Store` remain unchanged.
