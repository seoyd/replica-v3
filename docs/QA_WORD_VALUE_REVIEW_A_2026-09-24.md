# Independent A — QA word values

**INDEPENDENT_A: PASS.** The bounded word-value preparation and changed execution
boundaries are accepted. SMALL optimizer/generation/teacher calls are **0/0/0**
at this review. Model quality, B, QA640, S4/S5/S6 and Goal1 are not accepted.

Reviewed final source: `c1f82a719dcd72ecca2e7b8024384562423f37c2`.
Source digest: `2811ed5b1abbd2799925979b1ccabd38fa854537b38b4744946eeec5647ec877`.
Diff base: `e2e354a7f24babb61d13ac2f1266c1cfd12d7b0e`.
The report publication commit is separate from this reviewed source. Reviewer
writes are limited to isolated evidence, this report and the explicit A receipt.

| Final artifact | SHA256 |
|---|---|
| `candidate-final.diff` | `0af038a8cbbce5413f78959587ca0bf77164d90dbcbfdc9465ab349efbac4dfa` |
| Production `replica-train-final` | `d8e20a6b304fc9d1575b7ab43dca557f276486c3515a60b790f5d0468940b3b4` |
| `replica-train-tests-final` | `1fa71c04826415e5a0184709daf597380922100d3bfb83b2d0ada8a00f1f4457` |
| Preparation | `4e24d661b530df45477d731ff683bcf8c6d66b4d54f34d4079b2198df6a0891c` |
| Selection | `dc33cd128c9adf7652e21a68d9e1bb219b54722998c88404006e9c4177f8de48` |
| `ANSWER-MEAN/plan.r3b` | `b2db7c745bd811e13d8c051bc60a33df050fd20224efbfa27303cef381fb739c` |
| Corpus | `96e36843c792354bff98195e95db3ec98c4d0dde534aaa619787b5259665fa3c` |
| Metadata | `21b022c372f28d35ea2f2ce106c4b8a1f5b94e15a8e57d9b7fabb97e80c4f8af` |
| Parent14336 and initial native, physical file | `15015900a3c91a6b8ed87e9c2ecb8f7faf124849e5dcf8f40203cddf267e30d9` |

**Independent executions.** Existing source-bound locked/offline release
Accelerate test binaries were hash-checked and executed, without an unnecessary
Cargo rebuild. Exact test names share prefix
`training::fresh::identifiable::binding::citation::word::tests::`.

| Test | Executed source / binary | Result | Wall seconds | TINY optimizer / generation / teacher |
|---|---|---|---:|---|
| `word_native_process` | `915e2f04f3294cccc73f0d12f058589e6408a462` / `08bb95b9223c003731dc7bdf4f30f05fa65f46fdad8f919f53a09707e867c341` | 1 PASS, exit0 | 94.87 | 6 /120 /120 |
| `word_scope_never_confirmation` | final `c1f82a7` / final test binary above | 1 PASS, exit0 | 30.92 | 0 /0 /0 |

Both used `--ignored --exact --nocapture --test-threads=1` and compute threads1.
The process used `R3_WORD_PARENT=artifacts/instruction-bridge-completion-20260924-review/A-process/eval-only/ANSWER-MEAN`
and a new `R3_WORD_PROCESS_ROOT=artifacts/qa-word-value-20260924-review/A-process`
(absolute paths at execution). The zero-call test used the actual SMALL parent
plan under `artifacts/instruction-bridge-completion-20260924-study-final/ANSWER-MEAN`.

Continuous2, new-process1+1 and evaluation-only2+0 produce the same weights,
Adam, clock and raw. The resumed final-fit evaluation preserves its RETURNED
prefix and performs no optimizer step. Wrong LR, sequence, profile and tape
bindings are rejected; terminal resume and premature QA admission remain blocked.
These are native TINY execution tests, using the preserved EOS fixture, not
SMALL quality evidence.

The later source change adds the word profile to the existing confirmation deny
condition and a direct predicate test. The independent final-source test checks
TINY/SMALL with eligible true/false: all four are denied before seal access.
The earlier process result is reused only for unchanged training/evaluation
paths; it is not described as a process run of the final binary.

The source-bound implementer evidence `direct-final.log` is reused: three tests
passed, exit0,327.52s (`word_data_tape_boundaries`, `word_reader_score_gate`,
`word_raw_endpoint_gates`). These check the actual review4/word4 masks and
batch8 versus4+4 gradient reduction, strict string/ID scoring, and typed
writer→reader→decision early pass/continue/fit failure/final failed-dev fit/B
admission. Three synthetic backward calculations are separate from native model
calls. No unrelated suite or earlier accepted experiment was repeated.

**Actual preparation audit.** An independent Rust reader reused the native
corpus/checkpoint/codec readers and loaded the exact parent, without forward,
generation, teacher or optimizer execution. The parent has **136 Adam tensors**;
136 is not a clock. Its cumulative model step and sampler state are14336, and
the next Adam bias-correction clock is14337. Finite Adam content, native
objective family6/normalizer2/first-target1, QE, tokenizer mapping, LR3e-5 bits
and the original closed failure/B bindings all match the new fork.

Original train6144/dev3072 and their metadata are unchanged. Independent hash
ordering gives192 train,48 dev and30 unused semantic groups, disjoint per the
six word-pair partitions. The new train1536 and dev192/renamed192 have unique
resolvable answers, four query/assignment views, balanced selected record and
ID-size preference, unchanged S1Q1 system/question, and fixed current timestamps.
Renamed dev changes only IDs. All960 new event IDs are unique eight-digit IDs
outside the4771 explicitly registered historical reservations. Only public ID
reservation metadata was used; sealed answer bodies remain unopened.

The complete3072-update tape was checked against both halves of the original
1536-update suffix and all eight384-update word cycles. Review exposures are
V3072, VC0/VC1 each1536, bridge6144; word exposure12288. Train size7680 and
dev size3456 are exact. Every actual prompt/target round-trips through the same
tokenizer, uses the same train/generation framing and both evidence records;
target masks include EOS and exclude prompt/padding. Prepared cost is
**input4,534,272 / target359,424 / padding417,792**, maximum sequence205≤256.
These are planned costs, not executed SMALL work.

Native budgets bind origin14336, maximum17408,3072 updates,8m input/1m target,
11264 generation/10240 teacher,10800 active seconds and900-second segments.
Source inspection retains the12GiB inference/16GiB training limits, fixed
retention guards, same-checkpoint joint gates, fit-once and independent-B-before-QA
conditions. The scalar/citation/word generators and label resolver remain in the
training executable; normal product generation receives no expected answer.

Two preparation findings were corrected before acceptance: the new family's
missing per-token teacher observation, and scorer coupling of value/support to
whole-answer syntax. Current value uses the entire exact value string; syntax,
individual valid IDs, selected support and outside/malformed counts are separate.
The subsequently found confirmation admission omission is closed as above.

**Preserved failures and cost.** Implementer process01 failed at final16 with
INTEGRITY_FAIL after2/36/36; its raw/receipts remain. Process02 passed with6/120/120.
This independent process used6/120/120. Actual control/terminal recount therefore
gives **TINY14/276/276**, within64/512/512. The reader's first compile omitted
the existing zstd extern and failed before execution; the corrected compile and
both preparation/count readers exited0. No failed or zero-test result is PASS.
The original study remains unapproved; only the new final study receives A.

Consumed protected-file hashes matched before/after the audit. No deletion,
movement, compression or new inventory occurred. Recorded pre-preparation free
space91,834,804KiB exceeds the registered12GiB output allowance plus16GiB margin;
these are a space plan and observed free space, not measured future peak usage.

Local evidence roots: `artifacts/qa-word-value-20260924-evidence/`,
`artifacts/qa-word-value-20260924-review/`; accepted preparation root:
`artifacts/qa-word-value-20260924-study-final/`. Raw content stays local.

| Independent evidence | SHA256 |
|---|---|
| `A-process.log` | `b6aa27c03507ef5d6cd0d7b05405fe6dd7e24a117380ed1d60564470200f2df2` |
| `A-confirmation-deny.log` | `b278bc311e102c03a84cdaf90f71d359e8f386cc29e59b093be847364f343e17` |
| `read_A.rs` / executable | `0adbd0fbc15cf9a719a7614d5a26eb1ff77660e3bedf1f706c50b3c0a4799b5e` / `02d457321208b7df1a140582f1dc623967639e84207178b2423df5a0e46f568d` |
| Reused native production rlib | `a9126c1a260edacd3a54ab89d7341366649ad1ca479e5df21c8da5699d0b8df0` |
| `A-preparation-audit.r3b` | `cd7e9ceb2dbc0a43f65d6e4056661cec66ff14f4824f364b0e0cae7af80f1a3d` |
| `A-tiny-accounting.r3b` | `5c5e5d1bf3e2d017f591278b3c1a8c804992021db79a289503d483d14ecf36f8` |

The next authorized stage is the fixed parent400 observations, then only the
registered conditional learning/evaluation path. Confirmation/S4 remain
prohibited. A passing code/preparation review does not predict word learning or
change any earlier quality failure.
