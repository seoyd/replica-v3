# Selected event ID protocol 1.1 — independent review

Independent A is **PASS** for frozen source `2e98f6a834ffe440ce2f20dec4361dee8818ef29` and policy `e0a8d176266d932a3a20a86e5c6d4b66ed03434abc776a5cd5b8652ed7ec0e7f`. Baseline, conditional learning and independent B were pending at this stage; model quality and Goal1 were not accepted.

The separate Rust reader verified all 22,272 derived episodes, exact request transformations, source/native bindings, split disjointness, unchanged review rows, registered tape and costs. All six length summaries passed: FULL is 218 prompt + 15 gold + EOS = 234 tokens; ID_ONLY is 233 + 8 + EOS = 242. Each mode contains 1,536 train, 192 dev and 192 renamed records, always two provided and zero excluded evidence records. The maximum remains 256. The earlier 1.0 failure and its artifacts are preserved separately.

| Per-arm registered 128-update cost | F | I |
|---|---:|---:|
| Input tokens | 208,128 | 212,224 |
| Target tokens including EOS | 14,976 | 11,392 |
| Padding | 30,464 | 34,560 |

The arms share batch order and exposure. Target simplification also changes ANSWER weighting and token cost: I adds 4,096 input tokens and removes 3,584 target tokens. Train has 192 semantic groups and dev has 48 disjoint groups; renamed reuses those 48 heldout groups. On the fixed train128 panel, 42 records have parent/new exposure 1/1 and 86 have 2/0.

Independent actual Metal F32 TINY passed eight updates and two teachers, with fresh-process split runs matching uninterrupted weights, complete Adam m/v, clocks, cursor and token counters exactly. Model/Adam clocks advanced 14848/512 to 14850/514. Identical tapes plus cursor 2 preserve the next batch. Moved natives load; wrong arm/runtime reject. Both actual teacher rows passed independent device/dtype/shape, gold-shift, digest, finite-NLL and role checks; the pure reader inspected 25 target tokens without model calls. ID_ONLY has eight ID positions and EOS; FULL keeps MIXED spans explicit.

Root and reviewer together consumed **16 TINY optimizer/backward updates, four teacher calls and zero generation calls**. TINY updates are exhausted. Independent TINY wall time was 18.18 seconds, maximum RSS 1,795,833,856 bytes. A conservatively charges 32 seconds to the active budget, covering both TINY runs and the relevant numerical/readback checks. Frozen-source direct request/clock, scorer/gate and actual final-only no-call process evidence also passed and was reused without repetition.

Immutable evidence is under `artifacts/selected-event-id-20260924-review-v11/`: `phase-a.md`, `preparation-readback.r3b`, `tiny-teacher-readback.r3b`, raw TINY native/process evidence and the typed `review-a.r3b`. Matching root direct-test evidence and frozen binaries are under `artifacts/selected-event-id-v11-20260924-evidence/`. Executor SHA-256 is `69e3fac7b7ab9d43e3eca5886c7c3738fc7aaef64b80fbd2ae03b3c175c22d30`; test binary SHA-256 is `e2253e69cbe0c031bf6846537ff8db88c2a8fe55dcfdb2a1d9eb57d795133bd6`.
