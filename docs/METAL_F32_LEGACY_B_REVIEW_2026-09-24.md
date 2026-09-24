# Metal F32 baseline: independent LegacyB completion

Date: 2026-09-24. Contract: `R3-METAL-F32-BASELINE-1.0`, M1 only.

**Verdict: B_COVERAGE_COMPLETE.** The missing normal coverage was independently
completed and verified. The protected-adaptation model remains **QUALITY FAIL**,
`SEVERE_RETENTION_REGRESSION`, step 14464, `resume=false`. This report grants no
training, QA, confirmation, runtime A/B, S4/S5/S6 or Goal1 acceptance.

The earlier independent B executed 42 cases: 10 normal and 32 failure cases. Its
matching observations remain valid, but its normal-32 coverage was incomplete,
as recorded by the preserved closure review at
`a9ae778bf74e9acf3ee8317178073928271cc167`. The original A, B and closure reports
were not rewritten. This is a separate completion record.

## Reviewed identity

The executed candidate was HEAD `a9ae778bf74e9acf3ee8317178073928271cc167` plus
`artifacts/metal-f32-20260924-evidence/m1-candidate.diff`, SHA256
`020208ad8b7979961b72d2b50e3f69a3af2f09825c5edbe92b06b951cf936583`.
Its source edits were limited to `src/fresh.rs` and `src/word_value.rs`.
Source identity is independent of the later report/publication commit.

| Item | SHA256 |
|---|---|
| `src/fresh.rs` | `6c18a20b4f2571fa4bb18e7b7cf6068ce7041a3fba6fb48c033e77719a2266ea` |
| `src/word_value.rs` | `09130fca1a704f88cf2b8246850ae829b316f424f828d3cfd9886b6096da08ce` |
| Frozen `replica-train-m1` | `ebd143525598ec322feded1524f61b8ceb8d5df94506879c0fc34e89f615d68d` |
| Frozen `replica-train-m1-tests` | `fdc0ee13893aefb817c88e981771cfa9dc60baaa3f95fa9429f59fe5af20193b` |
| Final adapter checkpoint, physical file | `de9064bffc95aa8f09e5653c2878c0ee44ad8bd17349f746a2e9409532bcb9a2` |
| Referenced base 14336, physical file | `15015900a3c91a6b8ed87e9c2ecb8f7faf124849e5dcf8f40203cddf267e30d9` |

## Actual independent execution

The reviewer invoked the frozen production binary once, with one compute thread:

```sh
VECLIB_MAXIMUM_THREADS=1 RAYON_NUM_THREADS=1 OMP_NUM_THREADS=1 \
artifacts/metal-f32-20260924-evidence/replica-train-m1 \
fresh adapter-review-complete \
--study /Users/seo/Projects/Replica-v3/artifacts/protected-adaptation-20260924-study-final \
--output /Users/seo/Projects/Replica-v3/artifacts/metal-f32-20260924-legacy-b
```

Exit 0: `B_COVERAGE_COMPLETE normal32 failures32 reused16 new16 unique58`.
The identical command with `--check` subsequently exited 0, with no new calls.
The independent Rust reader also exited 0. It reused the existing native corpus,
tokenizer, typed record and durability codecs; it performed no model calls.
Its first compilation omitted the existing `sha2` extern and failed before
execution. Adding that link argument produced a successful build; both logs
are preserved. No dependency installation or product changes were required.

The source-bound implementation regression
`training::fresh::identifiable::binding::citation::word::tests::adapter_normal_completion_boundaries`
was reused as implementation evidence: 1 test, PASS, exit 0, model calls 0.
It was not rerun or represented as an independent test execution. Its reported
67.10 seconds overlaps part of a compiler operation and is not a performance
measurement. No zero-test result is counted as PASS.

## Coverage, provenance and usage

Normal cases are fixed metadata prefixes `[8,6,6,6,6]`, including complete
oriented query pairs, independent of correctness. Normal does not mean correct.

| Panel | Normal | Reused RETURNED | New calls | FULL | EOS |
|---|---:|---:|---:|---:|---:|
| Value | 8 | 2 | 6 | 8 | 8 |
| Citation | 6 | 4 | 2 | 4 | 6 |
| S1Q1 | 6 | 6 | 0 | 2 | 6 |
| Word | 6 | 2 | 4 | 0 | 6 |
| Word, renamed IDs | 6 | 2 | 4 | 0 | 4 |
| Total | 32 | 16 | 16 | 14 | 30 |

The retained failure set contains 32 rows. Its overlap with normal is 6 rows:
`32 + 32 - 6 = 58` unique observations, exactly the original 42 plus 16 new calls.
The two renamed normal cases ending at length 32 were retained as completed
length failures. No case was removed, regenerated to seek EOS, or postprocessed.

Every new output matched its original evaluation row, including token sequence,
strict decoded text, finish/error state, question/evidence and prompt digest.
The reader checked original/new producer separation, exact episode identities,
checkpoint/step, policy bindings, prepared/resolved `RETURNED` hashes, complete
pairs, uniqueness and final raw receipts. Expected answers were used only for
post-generation comparison and scoring, not supplied to the generator.

New usage was **16 generation calls, 224 output tokens, 2.10707125 active seconds,
0 teacher calls and 0 optimizer updates**. No UNKNOWN or pending call remained.
The original 42 calls used 662 output tokens; combined review evidence is 58
calls and 886 tokens. The prior P+B+Q accounting of 970 SMALL generations and
13,073 output tokens becomes 986 and 13,297 respectively. Historical TINY usage
is unchanged. M1 consumes 16 new generations of the current contract's budget;
it does not reset or extend any earlier budget.

## Durable evidence and preservation

The study-root supplement registration was published before execution. It binds
the old source/binary/raw/finished receipt, the new source/binary, checkpoint,
selection, reused IDs, missing IDs and separate output location. The original
42 are not attributed to the new producer. `--check` and independent readback
verified the same registration and coverage without reopening model work.

| Artifact | SHA256 |
|---|---|
| Original `word-review.r3rows` | `642a56426887e0b0cc2676f7beb455bfd03bbdc4d30ded64e1469c5778ea2148` |
| Original `word-review-finished.r3b` | `1c875b1e35b7e460675dd041d136684c9b422cbf2304b67af2e6ce1127efcfd6` |
| Study `word-review-supplement.r3b` | `9e82d175f786621433af2eb2f0763bd00628e0a79732e2fba74fb0dc9ef12d22` |
| New `normal-supplement.r3rows` | `09da60697ef1493cda849d7a02c34d98b0c544b286eb8009abdc7d53b0315404` |
| New `normal-supplement-finished.r3b` | `3bb8ddc6cf501febe70c8938165e4ca2db123021c31aeb1ccb25ed2a5f0659b9` |
| New `coverage.r3b` | `a017be828b2d467a7df0e3c1b2e374f0d8b6d951c3b12b5cd53b6c4db2264c6c` |
| Independent `recount.rs` | `efce1d4369d0ace5be4a70701c1657a8df2dcc020a2978cd73bb7127dbf7d2d1` |
| Independent reader executable | `4d606f7328081be29d232cafe2cd3e289f7793c98e0d86e824148b696c9f6f64` |
| Independent `independent-coverage.r3b` | `ec739b3a471b65c2865554ebed1c68b4e987e7c8439e499a8aa77b134cdbd4f6` |

New production evidence resides in `artifacts/metal-f32-20260924-legacy-b/`;
reviewer commands, compiler logs, readback and before/after checks reside in
`artifacts/metal-f32-20260924-legacy-b-review/`. The named preservation manifests
passed before and after: original 42/raw/finished, adapter, base 14336, protected
11264, frozen source/diff and the three prior reports retained their exact hashes.
No inventory, cleanup, historical model replay or original artifact replacement
was performed. Runtime Metal A/B remains separate and NOT_RUN by this review.
