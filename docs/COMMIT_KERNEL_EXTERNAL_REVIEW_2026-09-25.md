# Commit Kernel E independent review — 2026-09-25

RESULT: FAIL for recovery after a pre-rename save failure. One Medium defect is
confirmed through the actual `Endpoint` → `persist_bytes` → R3BIN disk path.
No product patch was applied. The review is closed with this finding; this is
not an acceptance of the candidate as defect-free.

Reviewed source: `da96082121e6b760e0b87884813a81eed249feac`.
The reporting commit is the commit adding this document, not the reviewed source.
Normal report-only publication and actual remote identity are reported separately.
The component remains separate from product SQLite/inference; no broader product
or model-quality acceptance is inferred.

## Confirmed finding

**Severity: Medium — stale temporary file prevents same-process write recovery.**

**Location:** `crates/replica-commit-kernel/src/external.rs:681`,
`Endpoint::open` (serial reset at line696), and
`crates/replica-commit-kernel/src/durable.rs:175`, `persist_bytes` (lines186–201).

**Evidence:** opening an endpoint resets its snapshot serial to zero. Saving uses
`external.<pid>.<serial>.tmp` with `create_new(true)`. An error after opening the
temporary file and before rename leaves that filename present. The endpoint is
poisoned and requires reopening, but reopening in the same process resets the
counter and attempts the same filename. The resulting `AlreadyExists` again
poisons the endpoint.

**Problem / impact:** a recoverable write or sync failure can prevent subsequent
sender or receiver state changes in the same process even after the underlying
I/O condition clears. Committed state remains valid; this is a recovery/liveness
failure, not demonstrated data corruption or duplicate delivery. The shared
helper predates E, but the newly introduced E endpoint uses it directly. A fresh
process with a different PID is outside this exact collision condition.

**Reproduction:** the isolated Rust fixture creates and reopens a receiver,
injects an error at the `write` hook, drops the poisoned endpoint, and twice
reopens and retries the same effect in that process. Both retries returned
`AlreadyExists` (OS code17), while the restored committed value remained0.
This exercises the production library with the actual R3BIN codec, not a mocked
persistence helper. The reproducer exits0 only after asserting the defective
behavior; exit0 is not a product-recovery PASS.

**Recommended fix:** change temporary-file allocation in `persist_bytes` so a
pre-existing temporary filename causes selection of another unused name while
holding the existing writer lock. Preserve exclusive creation and atomic rename;
do not truncate, promote or blindly delete stale temporary files. An endpoint
reopen must not make a stale PID/serial name permanently block progress. The
same correction should cover callers sharing this helper.

**Missing regression:** `tests/external.rs::native_guards_and_history_fail_closed`
checks poisoning and the committed value after reopening, but does not perform
a subsequent successful write. Add that final step for a pre-rename fault.
A bounded collision test should preserve the stale file and verify a new save
succeeds; retain existing post-rename uncertainty checks. Sender begin/ACK and
receiver apply should remain atomic across the same recovery boundary. No
unrelated malformed-input or full-suite rerun is requested.

## Actual execution and reused evidence

Evidence root: `artifacts/commit-kernel-e-20260925-review/`.

| Check | This review's result |
| --- | --- |
| Read changed E code, direct sender/receiver callers, reconciliation/compensation tests and shared persistence | Static review completed |
| Earlier VALUE_READING fixture repair | Narrow test-diff review plus reuse of existing exact 1/1 boundary and 1/1 full-report logs; not rerun |
| Existing E source/test/final-test identity | SHA256 matched the implementation manifest |
| Fresh locked/offline isolated library build | PASS, exit0 (`build.log`) |
| Initial standalone reader link attempts | NOT EXECUTED: exit1 due to cached/new serde dependency identity mismatch; logs retained |
| Recompile exact candidate library using existing compatible cached dependencies; compile independent Rust reader | PASS, exit0 (`build-cached-deps.log`, `reader-build-final.log`) |
| New `reopen` fixture execution | One execution, exit0; asserts two failed recovery attempts (`reopen.log`) |
| Original crate source/tests/manifests/locks and retained E final-test executable | All20 selected SHA256 entries unchanged (`protected-after.log`) |
| Model/teacher/generation/optimizer/backward/GPU calls | 0 |
| Prior stress/mutation/process campaigns | Reused provenance only; not repeated or counted as this review's dynamic PASS |
| GRU_SPEC / CORE_TRAINING | NOT_PROVIDED / NOT_RUN; no user GRU specification was read |
| Model quality / Goal1 | Not evaluated or promoted |

The original failing stress report is preserved. No all-artifact rescan, original
artifact mutation, cleanup, product/Cargo/test edit or external service test was
performed. The existing untracked `.DS_Store` is preserved. The only permanent
addition is this report; fixtures and build outputs remain ignored local evidence.

## Identity and reproducibility

- `external.rs`: `80249feee3bbb607907c924a49b24b9335b0adf280399ba911c8d184f950a815`
- Existing final-test executable: `d5f56c3caaff8a5a49c6028f98cc3e2150b44ae852bbf108bd5043f81ab488a8`
- Independent Rust reader source: `eed4f8a6aa6296cb56199902fc1ad687c6a7ba2e9d3b652a08bc5c460a16257f`
- Executed reader: `2ebcd4db960f9f50f457828479cc333436f9cbf23a559fb9f5041f961cb4c3bc`
- Fresh candidate library: `7332b6095e0011632a4c9d152ec909215118c0cfda4dd417e9130345dc5ab51b`
- Reused product codec library: `82cedaec6f798d0f9539311fdd27a55bbfc6a1df3d17aa4ee95ab6138e0809dd`

Full build commands are in the local `commands.txt`; build failures are retained
separately from successful builds. `reader-identity.sha256` binds the executed
reader/library files. `protected-before.sha256` and `protected-after.log` bind
the selected unchanged originals. This is a scoped preservation check, not a
claim to have hashed every model/corpus/raw artifact. No new model evidence was
created. Peak RSS and Codex token usage were not measured.
