# Replica commit-kernel frozen contract map (v1.2B → v1.2M)

The supplied v1.3A archive implemented only the v1.2B 32-vector core boundary.
The local port additionally verifies the v1.3B single-writer R3BIN disk/process
boundary. The following map preserves the later research requirements; it is
not a claim that all research stages have been implemented or accepted in Rust.

## v1.2B — production boundary core
- Kernel computes the actual effect digest from the effect payload.
- Capability authorization is derived from trusted kernel state, never a proposal boolean.
- All value/epoch arithmetic is checked; overflow rejects without consuming nonce/ledger.
- Operation-ID replay and nonce exclusivity are commit-boundary rules.

## v1.2C–H — durability / concurrency / liveness
- Effect + ledger + nonce + receipt form one durable local transition.
- Restore trusts committed durable state, not stale temporary files.
- Commit-time object/predicate/parent/capability revalidation is mandatory.
- Single commit authority is the first implementation target; reads/neural work may be parallel.
- Queueing is bounded and risk-budgeted; unbounded FIFO is forbidden.

## v1.2I–J — external effects / UNKNOWN_EFFECT
- Automatic retry is safe only against a receiver with verified durable idempotency.
- Ambiguous non-idempotent delivery becomes UNKNOWN_EFFECT, not guessed success/failure.
- Aggregate state changes alone are not attributable terminal evidence.

## v1.2K–L — versioned correction
- Corrections append immutable versions; old versions are retained.
- Derived nodes bind exact dependency versions.
- Use/commit validates the complete dependency closure against current durable heads.
- STALE caches are advisory only.
- Correction record + head switch share one atomic durable boundary.

## v1.2M — compensation
- Executed external actions are never erased when invalidated.
- Explicit compensators become immutable version-bound follow-up tasks.
- Ambiguous compensation enters RECONCILE_REQUIRED.
- Retry uses the same stable effect ID.
- If no explicit safe compensator exists, use MANUAL_INTERVENTION_REQUIRED.

## Port sequence
1. v1.3A: compile + exact 32/32 v1.2B vectors (this crate).
2. v1.3B: real Rust disk durability / crash-restart.
3. v1.3C: real Rust concurrent single-commit authority.
4. v1.3D: Rust versioned correction closure.
5. v1.3E: Rust outbox / reconciliation / compensation.
