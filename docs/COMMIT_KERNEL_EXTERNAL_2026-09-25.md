# Commit Kernel E local execution — 2026-09-25

RESULT: PASS for the declared I/J/M component gates. Overall product integration
is PARTIAL: A/B, C, D and E are still separate profiles. No external service,
model, teacher, optimizer, backward or GRU/TR++ mixture was used.

Baseline source: `1a382e10839d838a16e64995a6f4fb5de887106b`.
Tree: `5daa507c67b346dde8be88b7edf5010dc3d2183b`.
All original model/Adam/raw/failures, accepted11264 and the user's `.DS_Store`
remain unchanged. Earlier component gates are reused; no unchanged model gate
was repeated and no independent reviewer PASS is asserted.

## Preparation and provenance

Archive: `/Users/seo/Downloads/replica_commit_kernel_v1_2I_J_M_bundle.tar.gz`.
SHA256 `a3d1c40ce774ee8f5c7520285a91ed060f98da9237561390d223a754a7cb0256`.
All16 manifest file hashes matched. Only regular files/directories were extracted
under `artifacts/commit-kernel-e-20260925/` (the evidence root).
Go/Python source and reports are original saved artifacts. I/J/M vector JSON
files were reconstructed after the research runs. No Go/Python code was run,
and synthetic research rates were not relabeled as Rust or Replica quality.

| Input | SHA256 |
| --- | --- |
| I Go source | `27e9e6d0078441fc3453adf39767d8a53f4856bc6f016c499bc18447d1c31277` |
| J reference | `619bc869d1661f61abf45676944d3171cacbb9031d7d51ad18cbdfb3554c77bf` |
| M Go source | `c884f16ce473bb6979af6572511c5a556e8aa5bb9bad1940bf01419b9421d670` |
| I fixture | `e5d1085efc4e04befec961ebe0db752c3ef339f72170efe3511a5fe4ee64978c` |
| J fixture | `bc12952014ff9959242d83af5ec84199d40cfe3f493db60c498de522b06041a1` |
| M fixture | `50f71ac203f58d61ebb82ba2261324cbaf063116569ac36e87c935d8f95cbb42` |

## Implementation boundaries

`external::Endpoint` serializes typed sender/receiver state through host-supplied
R3BIN encoding and the existing atomic-file helper. Exclusive file locks protect
each writer. Sender attempts/InFlight are durable before transport. Receiver
mutation, payload-bound idempotency ledger and terminal receipt share a durable
snapshot. Receiver restore replays its actual effect history. Poisoned save,
corrupt committed state, overflow, wrong receipt/receiver and changed payload
under the same ID fail closed. No temp promotion or JSON persistent store.

Reconciliation requires attributable, fresh, authoritative per-effect evidence.
An open-world absence is UNKNOWN. A closed-world absence also needs a closed
delivery watermark: ordinary receiver queries return watermark_closed=false.
The local test host closes it only after all relevant child processes have
terminated and no request can arrive later. Models/retrieved text cannot supply
this trust. This test transport does not establish a watermark for real networks.

The port uses stricter bounded rules where the toy source was permissive:
payload/receiver binding, checked arithmetic, record validation, finite task/
history limits and explicit compensation policy. Same validity notification is
idempotent; only one unresolved compensation cycle is admitted. Restoration
binds a particular compensated task, preventing a historical ACK from causing
an extra restore during a later cancelled cycle. Conflicting attributable digest
evidence remains DISPUTED even alongside PARTIAL (the source's precedence could
hide that conflict). These differences are disclosed, not asserted to reproduce
every malformed-input behavior of the original references.

Compensation/restore deltas are both explicitly declared host inputs, not inferred
inverses. No declaration gives manual intervention. In-flight reversals reconcile
before cancelling or restoring; late ACK handling reloads current sender state.
This numerical reference does not implement real bank/email/reservation actions.

## Actual commands and results

Workdir: `crates/replica-commit-kernel`.
Environment: `CARGO_INCREMENTAL=0 CARGO_TARGET_DIR=../../target`.
Verified toolchain unchanged: rustc/cargo1.98.1, aarch64-apple-darwin,
macOS27.0/26A428. No dependency update; no GPU/model execution or speed claim.

| Command suffix after `cargo test --locked --offline --test external` | Result |
| --- | --- |
| `frozen_j_twelve_cases_and_attribution_boundaries -- --exact` |1 test PASS;12 frozen cases plus identity/freshness/conflict boundaries|
| `-- --include-ignored --skip process_worker --skip bounded_stress_with_real_processes --test-threads=1 --nocapture` |Initial direct4 tests PASS,1.92s (before adding later exact tests)|
| `bounded_stress_with_real_processes -- --exact --ignored --nocapture` |Initial parent test FAIL at log-name collision,39.47s; preserved|
| Same exact stress command with explicit prior-panel reuse |PASS,20.91s; completed I panels read-only, new I80/M60|
| `late_ack_uses_current_world -- --exact --ignored` |PASS,0.16s|
| `evidence_child_counts -- --exact --ignored --nocapture` |PASS; read-only actual child-exit aggregation|

Direct environment `R3_KERNEL_E_EVIDENCE` points to evidence root/direct-process.
Initial stress points to stress-process. Completion points to stress-completion
with `R3_KERNEL_E_REUSE_I` set to the absolute stress-process path. Reuse checks
all150 sender receipts against receiver receipts and all600 saved deliveries'
effect IDs/digests/value/outcomes; it does not regenerate any completed delivery.
Late-ACK evidence is under late-ack; count-reader environment is the evidence root.

`cargo test --locked --offline --test mutations external_guards_are_killed -- --exact --ignored`
passed with `R3_KERNEL_MUTATION_ROOT` at evidence root/mutations (21.20s).
Mutants use an isolated target, incremental0 and debug-symbols0. Open absence and
non-idempotent retry guards were separately disabled. Both compiled and failed
the selected semantic test with101. Compiler failure/zero tests are not PASS.

Observed component results:

- I: idempotent receiver/sender crashes replay the same saved receipt once.
  Non-idempotent ambiguity is UNKNOWN_EFFECT with no automatic resend. Explicit
  unsafe test retry duplicates5→10; production sender retains5. Receiver restart
  dedup and wrong-payload rejection pass.
- Receiver effect-first/ledger-first test mutants are killed after saving their
  split state. Strict validation rejects both; a weak accept/retry calculation
  shows duplicate10 or lost0. The weak retry is a negative-control calculation,
  not a second faulty production receiver execution.
- 150 unique effects all ACKED:20 receiver-after-apply kills,20 sender-after-ACK,
  20 withheld-ACK deadlines,17 prepared and17 in-flight kills. Timeout waits a
  real30ms deadline, then terminates the stalled local receiver for cleanup.
- 600 deliveries:120 COMMIT/480 REPLAY,120 effects.80 distinct effects recover
  with up to8 simultaneous sender processes; all80 sender/receiver records agree.
- Seven M directed scenarios, no-compensator/manual behavior, late ACK and
  historical-cycle preservation pass. Three M unsafe behaviors are explicit
  controls: changed compensation ID applies twice; blind cancellation/restoration
  yields wrong numeric state before the safe reconciliation path corrects it.
- 60 compensation/restoration cycles:120 ACKED tasks/receiver receipts; final10,
  duplicate0/lost0. Fault schedule:22 receiver crashes,29 sender ACK crashes,
  12 before-send crashes. This is a declared deterministic Rust schedule, not a
  reconstruction of the original random event tape.

## Preserved failure and exact execution identities

Initial parallel I80 failed because two test threads generated the same
PID+clock log-directory name (`AlreadyExists`, parent test exit101). The native
protocol was unchanged. Test call IDs now use an atomic sequence. The partial
I80 state remains in stress-process and is not reused for acceptance. Completed
I150/I600 were revalidated read-only; remaining panels used a new evidence root.
A relative executable-copy command also failed; it was rerun from repository
root before any executable was replaced.

Production `src/external.rs` SHA256 is identical across direct-process, failed
stress, completion and late-ACK gates (the earlier standalone J test preceded
the final sender-state validation checks):
`80249feee3bbb607907c924a49b24b9335b0adf280399ba911c8d184f950a815`.

| Actual executable / final source | SHA256 |
| --- | --- |
| Initial direct and failed stress `external-test` | `c9fc567d551b402a8036b6b13941aeb92929c35a7811b0793216aca64d7cfcca` |
| Corrected stress `external-completion-test` | `f8724ca5b2711805878aafc1810368aea0c10886a5e1140117a229b1c7abd1d0` |
| Late ACK / count reader `external-final-test` | `d5f56c3caaff8a5a49c6028f98cc3e2150b44ae852bbf108bd5043f81ab488a8` |
| Mutation runner | `5e628bb5a179e04443bcc0ca96ed2e21a0b5f49b4d69929759298726880d5dfb` |
| Final `tests/external.rs` | `c31abd6275ebe584098b34246c3311b4c7bb00625e13a91071251e8a57c5ba7c` |
| Final `tests/mutations.rs` | `6ba04a8644948e5d2a9dc6b414a092aa9e1c830c38a3132dd1a4dfa7e0c65d21` |

Root lock: `a911931fc3baca7367a34c129673640661a8fc3d41458d49e4eb744ae5454154`.
Crate lock: `57a5361c426c69ada27eaecc320a1d0516b02c75c721e9867a4093ddb88eb007`.
`source-binary-lock.sha256` records exact paths. Every test log has a separate
exit file; sender/receiver calls retain native requests, returned receipts,
stdout/stderr and actual exit/signal. Candidate diff is bound by the stage commit.

Read-only Rust exit aggregation: direct19 sender/23 receiver;
initial stress258/824; completion287/275; late ACK1/2. Total565 sender+1124
receiver,198 intentional SIGKILLs. Compiled mutation controls add4+4 children
and2 kills. Overall569 sender,1128 receiver,200 SIGKILLs; unexpected child exits0.
The parent test collision remains a failure despite child exits being expected.
`actual-child-counts.txt` contains main-lane counts; mutation exit files are
separate. Model/teacher/optimizer/backward counts are all0.

Scoped measurement before this report:9438 files,558883044 logical bytes,
575144KiB allocated. Shared debug11482264KiB,41188KiB growth from D. Peak RSS,
transient allocation and Codex usage are UNKNOWN. No cleanup/global inventory,
model-setting change, external API or independent-review claim.

## Remaining integration boundary

E transport is file/child IPC using native bytes, not HTTP or a live service.
Authority/freshness/watermark come from the trusted local test host. Neither
process SIGKILL nor a local idempotent receiver proves real power-loss or universal
external exactly-once. Synthetic J population error/coverage rates were not rerun.

The next product slice must join approved proposal/effect, current dependency
closure, operation/nonce/receipt and operating-memory mutation in a coherent
durable boundary. Simply writing to SQLite after a separate kernel commit is not
atomic integration. Existing C/D/E component profiles alone do not provide that
join. No product-memory or TR++ inference/training path has been connected;
GRU remains separate. Model quality and Goal1 are not advanced by this report.
