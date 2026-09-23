# Artifact hygiene — independent finalization review A

**Verdict: PASS for the H1/H2 no-call finalization boundary.** This review does not accept model quality, Goal1, artifact deletion, or the separate inventory/apply implementation.

The reviewer independently read the changed production path and ran the exact process regression once. Product source, original observations, models, policies, and earlier acceptance reports were not modified. No new build, model load for computation, generation, teacher call, optimizer update, or backward pass was performed.

## Reviewed identity

Reviewed source commit: `1fb463dbdca34e3119124f3d9918310840c5494e`. The test ran against the following frozen source bytes over base `02677fca16b2f2ecfe3c290223e21fb4257c68c7`; those two files were committed unchanged. The report's publication commit is separate from this reviewed source.

| Item | SHA-256 |
|---|---|
| `src/binding.rs` | `a754436cfe18cf4602f7bd1e8356be93da6fa2a7bdcc1fc0637d711402b56679` |
| `src/value_citation.rs` | `cbb8eedcdd30599b1cf842a4b9d8488f12d5af92c38648188b13f38d8a8026b2` |
| Executed test binary | `8150a21412d692325a6700fe0cd9a0ef2ef51278d7e6ee62596997f4412c4080` |
| Independent execution log | `dec0f572ebbe9612e44992d60da3e2f324e8719773aecf5d89275cb7ab6ad316` |

The executable was the implementer's newly built `target/release/deps/replica_train-337b3b476be81dd6`, hashed before independent execution. Reusing this exact executable avoids a redundant build; its earlier implementer result is not counted as the independent test.

## Actual execution

```sh
R3_RETURNED_FIXTURE="$PWD/artifacts/instruction-bridge-completion-20260924-review/A-process/eval-only" \
R3_RETURNED_OUTPUT="$PWD/artifacts/artifact-hygiene-20260924-independent-a" \
VECLIB_MAXIMUM_THREADS=1 OMP_NUM_THREADS=1 \
target/release/deps/replica_train-337b3b476be81dd6 \
training::fresh::identifiable::binding::citation::tests::bridge_returned_finalization_process \
--exact --nocapture --test-threads=1
```

Result: **exit 0; 1 passed, 0 failed; 15 child processes; 11.24 seconds**. Each child executes the same exact test entry and invokes the shared production output caller. New SMALL/TINY optimizer, generation, teacher, and backward counts are all **0**. A test-only entry guard makes attempted native model entry an explicit failure.

The test reads the historical endpoint, policy, independent B receipt, and original four-row journal in place. Only the QA journal is copied into new isolated directories. Its copied segment elapsed time is adjusted so verified prior work plus copied observation usage equals **7200/7200 seconds**. This is a boundary fixture, not measured historical model time.

| Boundary | Independent result |
|---|---|
| Four accounted RETURNED rows, zero remaining work, no aggregate final/score, active cap reached | Strict reader and fresh-process production caller finalize successfully |
| Reentry after completion | Raw/final/score hashes unchanged; no new calls |
| Coherent three-returned-row prefix, one remaining, cap reached | Rejected with `budget exhausted` before model entry |
| Missing resolution or unknown unfinished segment | Rejected; no final published |
| Different model/policy, reordered/truncated/extra/duplicate raw | Rejected; no final published |
| Cancellation or TIME_BUDGET mixed with I/O failure | Rejected; no final published |
| Final publication followed by injected sync failure | Published file and pending marker retained; fresh-process retry rejected as publication UNKNOWN |

## Production-path findings

`bridge_diagnostic_qa` retains the study lock, registered endpoint/admission checks, independent B validation, and input identity checks. It calls `bridge_diagnostic_output`, the same function exercised by the fresh child processes.

The zero-call branch uses `segmented_returned`, existing segment/call/prefix readers, and strict raw scoring. Complete counts alone are insufficient: case order, requests, producer identity, checkpoint, call records, RETURNED resolution, and token/text receipts must agree. Returned failures are not regenerated or removed from the rows. Candidate and S4 authority remain false.

Historical producer source/binary are preserved. New finalization records identify the current finalizer separately, retain original active usage, and record management time separately. Management is bounded by 900 seconds, 512 MiB of unique observation input files, and 16,384 directory entries; the byte figure is explicitly not physical I/O. Known pure-time over-cap history is distinguished from unexplained over-cap accounting. Remaining model work still requires the original current-source admission and unchanged model budget checks.

Confirmed publication and pending-state rejection remain active. The successful reentry validates existing bytes instead of rewriting them. No confirmed defect remained in this reviewed boundary.

## Evidence and limits

Local independent evidence is under `artifacts/artifact-hygiene-20260924-independent-a/`; the parent log is `artifacts/artifact-hygiene-20260924-independent-a.log`. Original and successful copied raw both hash to `259dc4cdd338922afd98714c91c2743ba311334165f212c6aafa87d2111d4c52`.

| Successful copied artifact | SHA-256 |
|---|---|
| Aggregate final | `b60aaf8845d38ec9b4d4734263c3485bff15986e2e2440e30f1ca312912066be` |
| Strict score | `966a17e1551cc79e1d78384e8a8a117ea0c16f2a6c807ebfa39e52533271bec6` |
| Separate finalization receipt | `4666f04af3e5854bf3dff618f260a4afd763094f8cb6adba9b5257dba94ffc6a` |

The cap and negative mutations are confined to disposable copies of actual prior RETURNED evidence. They establish state-transition and publication behavior, not new model quality. Existing B reproduction and QA640 were not repeated. No artifact removal, archive conversion, inventory acceptance, or deletion authority is granted by this report. Preserve this exact test executable and its source identities with the review evidence.
