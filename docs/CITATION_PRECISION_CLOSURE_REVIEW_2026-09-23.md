# Citation precision — independent closure readback

2026-09-23 · R3-CITATION-PRECISION-1.0. **Review PASS within the contracted scope.**
No confirmed product-code defect was found. General QA quality remains **FAIL**;
S4/S5/S6 and Goal1 are not accepted. No product patch or additional stabilization
test is requested by this review.

Reviewed source: `3290fec66db372c5ab427bf2e632bf573824ce3d`. At review start,
HEAD and actual remote main were `0143b90bf489851d8d2face1907500994e6e139a`.
Their product source/tests/Cargo are identical to the reviewed candidate.
This publication is report-only; its commit is separate from the source SHA.

The current contract was read in full. Production review covered the precision
parent/fork, tape, LR-to-Adam call, scheduled evaluation, remaining-token caps,
terminal/resume state, reproduction selection and confirmation chain. Existing
[A](CITATION_PRECISION_REVIEW_A_2026-09-22.md) and
[B](CITATION_PRECISION_REVIEW_B_2026-09-23.md) acceptance was reused after identity
checks; completed model work was not repeated.

A's three numeric/tape/process tests ran on source `3682540`; the affected
executed-token-cap regression ran on final source `3290fec`. These are preserved
historical dynamic results, not newly executed tests on this review turn.
B's complete raw/teacher/native/trace verification and actual V16/VC16 fresh-process
parity remain bound to the unchanged candidate. The parent reproduction included
its wrong orbits; the final candidate had no wrong development rows to select.

| Separate verdict | Result |
|---|---|
| Code / independent A / independent B | PASS, existing acceptance preserved |
| Training | Completed at the contracted early stop: 768 updates, step11264, resume=false |
| Development and fit | Existing joint gates PASS; unused768 updates remain unused |
| Same-candidate citation confirmation | Independent saved-raw readback PASS |
| General QA640 | Executed quality FAIL: primary0/512, transfer0/128 |
| S4/S5/S6 / Goal1 | NOT_ACCEPTED / not ready or accepted |

Confirmation was already completed once after B. This review only read its original
512 rows. Independently resolved request gold, strict token decoding, complete
string/EOS, support ID, FULL/QB/SB/ALL4 and stored scores agree. Both V and VC are
256/256 FULL, 128/128 QB and SB, 64/64 ALL4, EOS256 and errors0. VC support256,
outside IDs0. The B report hash, eligible decision, fixed checkpoint, original
seal claim and all512 RETURNED prepared/resolved pairs agree. One complete
confirmation command recorded generation512, teacher0, tokens4864, active30.997563s.
No candidate replacement, new seal execution, missing-receipt synthesis or output
correction occurred. Acceptance remains limited to the recorded one-digit value,
two-record selection and fixed citation grammar.

QA's existing native reader was rerun against scratch copies of the two R3ER files.
The new recount is byte-identical to the original. EOS440, length-stop200,
runtime/UTF-8 errors0; all640 answers remain incorrect. Empty citation sets in G/H
do not count as correct answers. QA used the separately approved original128-token
limit. Its original combined control active time remains UNKNOWN, not zero.
This is a quality observation, not evidence of a new code regression or an
isolated causal effect of LR.

Actual commands executed from the repository root, with logs in
`artifacts/citation-precision-20260922-review/closure-check/`:

```sh
VECLIB_MAXIMUM_THREADS=1 OMP_NUM_THREADS=1 RAYON_NUM_THREADS=1 target/release/qa-native-recount artifacts/citation-precision-20260922-study-final/ANSWER-MEAN/segment-0003/final artifacts/citation-precision-20260922-review/closure-check/qa640 artifacts/fresh-joint-20260919 artifacts/citation-precision-20260922-review/A-final-source
artifacts/citation-precision-20260922-review/closure-check/confirmation-reader
```

Both final commands exited0. The Rust confirmation reader reused the existing
independent recount functions and already-built candidate dependencies, offline.
Its first run exited101 on a reviewer-only assumption of four segment files;
the actual single command has two, start and finish. That failed scratch log is
preserved; correcting this reader assertion did not modify product or evidence.
New optimizer/generation/teacher/diagnostic-backward calls in this review: **0/0/0/0**.
The QA reader's printed generation640 describes existing raw, not fresh calls.
Historical contract totals remain SMALL768 optimizer /7680 generation /6464
teacher; TINY58 optimizer. No new test invocation is counted as dynamic PASS.

| Evidence | SHA-256 |
|---|---|
| Frozen production executable | `0020e08670d6e66de5b4c2759fac21f012f2bd610c98474580af32fee525132f` |
| Confirmation reader source | `7603c01b692afd31a157a9444b4e5d73855a58b5a2f1c816eb88e6a6e21f18bd` |
| Confirmation reader executable | `b8e6ac8dd3be029bade959167f30a55cc37f87d722c08ea69bd7fb240bdccbee` |
| Independent confirmation recount | `43961133a4499585928785d6986d6346a89fe69daaf2dda509c1f1ac0b0b71be` |
| QA reader executable | `c6a34bde5280fd4969c4328f781c7644af4f3ccc592f53389b07123183c44404` |
| Original and repeated QA recount | `1aec55f53eeb5f8bbaa4ac98b8018000ff298ddf50485c6fe738ac875990651b` |
| Protected-file manifest | `a6a64277f30cda4004f97feb1da40fa9d0aa2531a989e9be96601e519354cfa4` |

The1092 consumed/protected file hashes match before and after readback, including
the original27 A-protected paths and the B-bound raw/teacher files. Study and
original seal-owner file inventories are unchanged. Current product source,
tests and Cargo remain unchanged; the pre-existing untracked `.DS_Store` was
preserved. Original A/B reports were not edited because their hashes are bound
into execution evidence. Only this report is published; scratch and originals
remain local. The authorized review scope is closed without further learning,
duplicate model reproduction or speculative repair requests.
