# B0 implementation DAG

```text
G0 source/environment
 ├─ G1 typed events / binary codec
 │    └─ G2 SQLite / recovery / backup
 │          └─ G3 versions / search / relations
 └─ G4 local model adapter
       G2 + G3 + G4 → G5 CLI / restart / fixtures / measurements
```

| Node | Status | Files | Tests/evidence |
|---|---|---|---|
| G0 | VERIFIED | docs/IMPLEMENTATION_REPORT.md, docs/REUSE.md | docs/logs/environment.txt, baseline reads; archive absent is recorded |
| G1 | VERIFIED | src/event.rs, src/codec.rs | tests/codec.rs; docs/logs/final-tests.txt |
| G2 | VERIFIED | src/store.rs, migrations/001_init.sql | tests/store.rs, tests/cli.rs; docs/logs/final-tests.txt, docs/logs/cli-smoke.txt |
| G3 | VERIFIED | src/retrieval.rs, src/store.rs | tests/retrieval.rs; docs/logs/retrieval-regression.txt, docs/logs/measurements.txt |
| G4 | BLOCKED | src/model.rs (Rust adapter implemented) | tests/runtime.rs passed; actual model absent/installation deferred by user |
| G5 | BLOCKED | src/app.rs, src/main.rs, examples/validate.rs (implemented) | CLI/restart/failures/measurements verified; real 5-query smoke deferred |

G4/G5 BLOCKED denotes actual model verification, not a fake replacement path.
No B0_READY claim. The user's subsequent Rust-only/latest installed Rust/no external
API/no canned result instructions and explicit model installation deferral are
included in the final implementation report.
