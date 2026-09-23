# Independent A1 recheck — QA integrity and bounded bridge

**A1: PASS. No additional confirmed code defect in the reviewed scope.**
The repaired scorer and the actual raw-to-decision path reject malformed
citation rows, including rows that also contain valid outside IDs. No product
patch or additional repair is requested.

Reviewed source: `8522d026512cfb7b6e4b6c32142c5d68b77c9f2f`.
Source digest: `9b46c11e60cf4ecbd44a77d28c82f70b59dc94895e5a2a81149c2f0576c500b4`.
Repository HEAD before this report: `1ad0e8a77326de6c1eaf0c82d51877d470e1749c`;
its source/tests/Cargo are identical to the reviewed candidate.
This report's commit is separate from the reviewed source.

The complete shared contract and review prompt for
`R3-QA-INTEGRITY-AND-BRIDGE-1.0` were read. The local input document SHA256 was
`8f923dc80957442ed57ea5155d2897f2bab159059809abe53a36e7ced3b4ae96`.
Review covered the repaired citation scanner, historical reanalysis,
raw/scorer/gate integration, and the directly changed bridge preparation,
evaluation, termination and reviewer-admission paths.

| Verdict | Evidence and scope |
|---|---|
| Repair A1 | PASS: two tests newly executed on the exact candidate, plus new independent historical recount |
| Historical correction | PASS: four omissions confirmed; final15360 metrics and closed failure preserved |
| New-training preparation A2 | Existing PASS preserved; source-bound process/input evidence hash-verified, not rerun |
| B result integrity and reproduction | Existing PASS preserved; evidence/native/raw manifests hash-verified; prior44-call reproduction not repeated |
| Bridge training | Existing1536-update run complete at12800; no new training in this review |
| Bridge model quality | FAIL: valid outside-ID rows remain in all three bridge panels; candidate ineligible |
| Conditional fit / old QA640 / S4 | NOT_RUN; no new candidate, confirmation or seal use |
| Protected11264 / S5 / S6 / Goal1 | Prior11264 acceptance preserved; no new S5/S6/Goal1 acceptance |

## New execution

Exact source/tests/Cargo were archived into
`artifacts/qa-integrity-bridge-20260923-review/recheck/source` and compared against
the unchanged product files. Cargo target and TMPDIR were isolated under the
same `recheck` directory. The existing locked dependencies and Accelerate backend
were used offline with one compute thread.

Each command used `cargo test --locked --offline --release --features
accelerate,test-support --bin replica-train FILTER -- --nocapture
--test-threads=1` from that source directory. Environment variables were
`CARGO_TARGET_DIR=.../recheck/target`, `TMPDIR=.../recheck/tmp`,
`VECLIB_MAXIMUM_THREADS=1`, `OMP_NUM_THREADS=1`.

| Filter | Result | Test duration |
|---|---|---:|
| `retained_qa_citation_grammar_boundaries` | 1 passed, 0 failed, exit0 | 0.00s |
| `retained_qa_full_raw_gate` | 1 passed, 0 failed, exit0 | 198.08s |

The first command compiled the candidate in33.37s. Test binary SHA256:
`7f877e55d6808915037e90c8b1a45f8f8c38f5bb4c820e47a10ca1820bcb1489`.
Neither test performed optimizer, generation or teacher calls. The native
fixtures and their synthetic reservation data were confined to scratch.

The second test exercised the real typed writer, reader, raw audit,
`qa_score`, `qa_dev_pass` and `qa_decision`. Its two malformed cases both had
FULL511/512 and EOS512/512:

| Case | Valid outside rows | Parse-failure rows | Candidate eligible |
|---|---:|---:|---|
| Malformed plus valid outside ID | 1 | 1 | false |
| Malformed only | 0 | 1 | false |

The same test verified passing G/H cases, missing mandatory evidence,
failed conditional fit and bucket/pair/EOS boundaries. Grammar coverage included
ordering, duplicate IDs, nested/unclosed starts, overflow, nonpositive IDs,
UTF-8 surroundings, plus signs, leading zeros and missing text. Prior RED
evidence remains preserved; it was not rerun or counted as a current PASS.

Production code references are `src/value_citation.rs`:
`individually_valid_ids`/`has_outside_id` at732/738,
`read_score` at989, `qa_score` at4261, `qa_dev_pass` at4366 and
`qa_decision` at4394. Strict product parsing remains unchanged. Individual
positive-ID enumeration is independent of strict parse success; the gate also
requires parse-failure rows0. Bridge decisions independently require valid
outside rows0 and parse-failure rows0 at4012–4028.

## Independent historical recount

The frozen production `executable-bridge` ran `fresh retained-qa-recount
--study artifacts/retained-qa-20260923-study-final --output
artifacts/qa-integrity-bridge-20260923-review/recheck/historical-recount.r3b`,
exit0. Its binary SHA256 is
`341f018c30aa57feaf6ae1a8e50ddadedb1c91492a998da0f76bdd12b2ea6394`.
It published a separate correction; no original decision or score was replaced.

The independent Rust `audit.rs` was copied with only its output directory
redirected to new scratch, compiled and executed against that correction,
exit0. Its scanner uses byte windows and checked decimal arithmetic, independently
of the product ID extractor. All7168 generation rows, including3328 QA rows,
were read; actual provided IDs, empty exclusions, decoded tokens, FULL/EOS,
strict support, parse failures, bucket fields and production correction rows
were compared. All20 parse failures were separately counted.

| Step / QA panel | Previously counted outside rows | Corrected | Omitted row indices, zero-based |
|---|---:|---:|---|
| 11392 / balanced primary64 | 14 | 16 | 6, 7 |
| 12288 / old primary64 | 31 | 32 | 11 |
| 13312 / old primary512 | 281 | 282 | 99 |

Final15360 FULL/outside metrics were unchanged. The original
`PERSISTENT_RETENTION_REGRESSION`, `resume=false` and ineligible state remain.
New optimizer/generation/teacher calls for this entire recheck: **0/0/0**.

## Evidence reuse, preservation and closure

The unchanged candidate's existing A2 continuous/split/evaluation-only process
and mixed-objective evidence was reused under
`QA_INTEGRITY_REVIEW_A2_2026-09-23.md`. Existing B evidence under
`QA_INTEGRITY_REVIEW_B_2026-09-23.md` retains the6528 raw/teacher-row audit and
44/44 fresh-process reproduction, including all six original wrong answers.
Those executions were not repeated or presented as newly executed tests.

Evidence hashes, the A1/A2/B protected manifests of84/28/116 files and original
native artifacts were rechecked. These counts overlap and are not unique-file
totals. Before/after verification logs were byte-identical. File inventories of
the historical QA, protected precision and bridge study roots were unchanged;
no missing QA/confirmation evidence was synthesized. Source/tests/Cargo remain
unchanged. Existing `.DS_Store` was left untouched.

Bridge12800 has FULL511/511/508 and valid outside rows1/1/1 on S1Q0/S0Q1/S1Q1.
The gate correctly records `BRIDGE_DEVELOPMENT_FAIL`, `resume=false`, no eligible
candidate. Prior retention panels remain512/512. Existing usage stays
SMALL1536 optimizer,6652 generation,6528 teacher; TINY14/168/168.
Quality failure does not invalidate the independently verified result integrity.

No additional unit, integration, regression, malformed/boundary or failure-path
test is requested for this closed A1 scope. The relevant executed checks and
unchanged source-bound evidence cover the identified repair. This report does
not reopen the closed study or authorize further learning, promotion or S4.

Local evidence is under
`artifacts/qa-integrity-bridge-20260923-review/recheck/`; only this report is
published.

| New evidence | SHA256 |
|---|---|
| `grammar.log` | `dc3ec972e625ebd9318e186a080aa19dffca64c7dfd1fa95a5808edbab14b741` |
| `gate.log` | `38f5ec1711fe5af2973a990e2fae99be07d47aa55dad8b751829b62076b9df9f` |
| `historical-recount.r3b` | `0f7c2b0ad6a8343a29da08d92ffd863187aec8e872eb34246a0a0422115b2e1b` |
| `audit.rs` | `090b6510acded010fc5cdbcaff8026ae8fd8beecf837e4b0f89c295a4a02aec5` |
| `audit` executable | `ab8f5ccdef80b8d29310b37f84623bb94590808d8629f5596436ca1e99dd6ac2` |
| `A1-independent-recount.r3b` | `c5dde3bfb3b8be05ed7d9fbeb5dfab125d54e2d72e6da9911d4e0e450802faf6` |
| `A1-consumed-manifest.r3b` | `ee8223b4c474ff667806a314f2987dfc421c2523ada7950d5e13c1001ff7a8a9` |
| `scanner.log` | `0184f372fad213a1222474fd0309ecef1a78202d2c97da5d312a88782c3bd737` |
| `preservation-after.log` | `ff72f4cc53b40de4064c211e30e1ad9709364ebde318f3a9e3406107fe794ca4` |
