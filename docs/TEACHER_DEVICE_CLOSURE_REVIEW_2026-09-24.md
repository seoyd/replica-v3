# Teacher device closure — independent review

Date: 2026-09-24. Contract: R3-TEACHER-DEVICE-CLOSURE-1.0.

**A-delta PASS; actual teacher caller and complete diagnosis are pending.**
This stage admits only the three registered first teacher rows. No teacher
forward, generation, optimizer or backward was executed by this review yet.
The old training and old diagnosis failures remain unchanged.

| Acceptance layer | Current verdict |
| --- | --- |
| A_DELTA | PASS |
| D2_IMPORTED_INTEGRITY | VERIFIED |
| ACTUAL_TEACHER_CALLER | NOT_RUN |
| TEACHER192_COMPLETE / TEACHER24_PARITY | NOT_RUN / NOT_RUN |
| REPRODUCTION_A / REPRODUCTION_M | NOT_RUN / NOT_RUN |
| NEW_CALLS_WITHIN_BUDGET | 0 generation / 0 teacher; caps128 /216 |
| ORIGINALS_PRESERVED | Scoped original references match |
| SUCCESSOR_DIAGNOSIS | PARTIAL; actual gates pending |
| ORIGINAL_TRAINING / PREDECESSOR_DIAGNOSIS | FAILED_UNCHANGED / FAILED_UNCHANGED |
| MODEL_QUALITY / MUON_ADOPTION | FAIL / NO |
| S4 / S5 / S6 / GOAL1 | NOT_ACCEPTED |

## Source and policy

- Reviewed source: `1344ae3b4e2f359db81d889e1c2c24b2356e7b17`.
- Comparison baseline: `f5895057ba40e1758c72bf7dba5e37fef21d6445`.
- Old D2/D3 producer: `4850195a1b5cb1d51f7f49fccae14ed194243f56`.
- Frozen executor SHA256:
  `c6b89acddb2587abbef941ddc40813362330866e3a719bd84faa7f64a3c6907d`.
- Test executable SHA256:
  `e8d147b940e0439f451bdaa2a58750acda1c118ad3fc5f5a51572f39e2e16ac0`.
- New plan physical SHA256:
  `5b2368f13073d41ced50ecff960ee20c3653065b9cee07a2eefb8889e24c4b4a`.
- New policy:
  `0b1b017796960c4d0c84814252101a49e6950ccc57118a50ded2aa7a5e6c740c`.
- New run: `artifacts/teacher-device-20260924-run/`.
- Independent evidence: `artifacts/teacher-device-20260924-review/`.

The changed source is confined to `src/muon_diagnosis.rs`, the existing teacher
prefix collector in `src/fresh.rs`, and forward observability in
`src/quality_recovery.rs`. Model math, tokenizer, generation policy, product,
vendor and Cargo lock have no delta. The reviewer found a reachable new empty
prefix regression during preparation; it was corrected before model execution.
No remaining confirmed source defect was found. This is not yet proof that the
corrected actual teacher caller completes.

## Independently executed A-delta evidence

The reviewer authored two negative tests against the production reader and
executed each once on the final test executable. They reject changed D2 rows,
missing/duplicate rows, incomplete/pending resolutions, mixed cancellation or
UNKNOWN/nonfinite/I/O failures disguised as the known device failure, old failed
root reentry, old rows relabelled with a new source, and changed parent/A/M,
step, tokenizer, runtime or teacher selection. Synthetic journals are confined
to the new scratch root; models and corpus are read in place.

| Exact test | Result | Wall seconds | Maximum RSS bytes |
| --- | --- | ---: | ---: |
| `independent_successor_rejects_d2_and_failure_forgery` | 1/1 PASS, exit0 |27.87|3,890,855,936|
| `independent_successor_rejects_identity_changes` | 1/1 PASS, exit0 |30.12|4,892,590,080|

The implementer's focused prefix fixture checks initial empty legacy prefix,
first1/full64 immutable identity, a fresh-process reader, exhausted-budget
no-call reading and duplicate rejection. Its evidence is identified separately
from the reviewer-executed tests; no numerical test-count threshold is used.

The independent Rust plan reader newly checked all1,856 scoped references,
case/ordinal/RETURNED bindings and complete D2 lane finals. It independently
reconstructed teacher64, check8 and replay32 normal+32 failure extras per arm,
verified parent14336/A512/M512 metadata, content and physical identities,
SMALL/F32/framing, and runtime parity except for the new binary. It exited0 in
2.06s, maximum RSS231,489,536 bytes. Both selected replay unions contain64 cases;
M retains two strict-UTF8 cases. All model counters remain zero.

The old independent reader was also run anew against the failed predecessor
before preparation, exit0, wall1.50s. It verified original586+missing54 =
composite640 and train256, hence896 unique reused rows. Both train FULL scores
remain0/128: A whole-value50/support0; M whole-value0/support2. Per arm42 cases
were exposed once and86 twice. These scores and historical5,604 newly generated
tokens are not new successor model work.

The historical failed teacher-row digest remains
`814debc4b71bc32598257aa6fd3416c61393fa0f1bd9955d98f1420c704479a3`;
the failed observation003 remains
`525b82accad293c8a37eeddbf6d2c68d6074290feca2fc1ac2ab2e14b27d6edb`.
Historical usage is generation310, teacher attempts1/results0, active57.566452876s.
No old failure or UNKNOWN is converted into successful evidence.

## Remaining actual gates

The three formal ordinal-zero rows must complete the real Metal F32 teacher
forward, save native row/resolution evidence and pass a model-free fresh reader.
Their1/64 coverage must remain incomplete until the other63 rows per model are
appended. A separate A-caller receipt must record the actual three calls.
Only then may teacher192, independent teacher24 and A/M replay at most128 close.
The final role, span, free-prefix and next-change conclusions await those gates.

The reviewer used the repository's Replica Lean Development and Karpathy
Guidelines skills. Only new reviewer Rust tools, scratch evidence and this report
were written by the reviewer; original reports, source artifacts and untracked
`.DS_Store` were preserved. Report publication identity is separate from the
reviewed execution source.
