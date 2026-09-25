---
name: replica-lean-development
description: Reduce repeated exploration, tests, logs, and reporting while implementing or reviewing Replica v3. Apply only to this repository's development and experiments; preserve explicit requirements, evidence, and execution budgets.
---

# Replica Lean Development

Use this skill for work in this repository, including resumed tasks. Reduce wasted
work, never required evidence. It does not change the model's reasoning setting,
grant execution permission, or define model quality gates.

## Authority and scope

- Current explicit user instructions and the active contract take precedence.
  Honor requested full document reads and final requirement comparisons. Otherwise
  compare the known document hash/version, read changed or relevant sections, and
  expand only when requirements conflict or dependencies are unclear. If prior
  contents are unavailable, read enough to establish the complete applicable scope.
- Reuse AGENTS.md as the map and the existing active plan/status as evidence.
  Keep temporary handoff documents out of source, builds, permanent links, and
  runtime/authorization inputs. Do not copy their entire contents into new files.
- Preserve local changes, originals, failed records, and closed run decisions.
  Never use this skill to reset data, relax a gate, extend a budget, or self-approve
  an independent review. Keep code, execution, model quality, and Goal1 separate.

## Read and change only what the task needs

- Start with the request, current diff/status, and the known relevant symbols.
  Use scoped `rg` and bounded line reads; expand to callers, tests, then subsystem
  only when an unresolved question requires it. Do not inventory the repository.
- Reuse already read unchanged content. Re-read when the file changed, the context
  is missing, a required full read applies, or a concrete inconsistency warrants it.
- Batch independent reads. Retain a compact checkpoint in the existing status or
  handoff: source/artifact identity, verified facts, actual costs, last durable
  state, remaining dependency. Do not create a new tracking framework.
- Reuse existing code, helpers, standard library and locked dependencies before
  adding files. Avoid unrelated cleanup, formatting, abstraction and refactoring.
- Do not reopen an accepted design without new evidence or a changed requirement.
  Reserve deeper analysis for ambiguous failures, numerical/state integrity,
  incompatible serialization and interactions; routine edits need scoped reasoning.

## Tests, failures and experiments

- Run the smallest meaningful affected test, then required integration/process
  regressions. Run a broader suite only for a concrete shared-path risk or an
  explicitly required gate. Completion alone is not a reason for all tests.
- Reuse passing results tied to the unchanged source/binary and inputs. A source
  change needs affected-path verification, not automatic repetition of every test.
  Never claim an old result ran on new source. Zero tests and NOT_RUN are not PASS.
- On failure, capture expected/actual, first divergence and smallest actual-path
  reproduction. Read the first meaningful error and relevant context; avoid
  repeatedly dumping duplicate errors or analyzing one cause as many defects.
- Keep raw logs in the existing evidence directory. Surface only test name/count,
  exit, relevant source/binary hash and failure excerpt; preserve warnings and
  limitations without printing entire successful logs or full raw panels.
- Follow the registered experiment exactly. Cheap representative probes help only
  where the contract permits them; do not replace mandated SMALL, full panels,
  independent checks or fresh-process runs with TINY/synthetic substitutes.
- Count failed work and UNKNOWN, obey cancellation/deadlines and one heavy process.
  Confirm durable terminal/cursor and prior calls before resuming; do not regenerate
  completed rows to recover an interrupted report. No duplicate independent review
  or extra agent unless explicitly required/authorized and useful.

## Communication and stopping

- Use brief updates with newly observed facts, meaningful milestones, failures or
  changed next dependency. Follow required update cadence; avoid repeating the
  whole plan, unchanged counters, or polling the same log unnecessarily.
- Use the user's requested final format. Otherwise use RESULT, CHANGED, TESTED,
  ISSUES and one necessary NEXT. Link concrete candidate/diff/evidence when asked.
  Expand explanation for consequential decisions or unexpected experimental results.
- When a stage passes, continue already authorized dependent stages. Stop at the
  task's actual completion or a contractual blocker/limit, not merely a useful
  intermediate result. Do not add speculative improvements after completion.
- If missing information could change correctness, preservation, interpretation or
  release eligibility, obtain it. Do not save tokens by guessing.

## Model, effort and measured cost

- Use only settings actually supported by the current account/client. These are
  project recommendations, not capability rankings or guaranteed savings:
  verified CLI/hash/count/publication: Luna High or Sol Low; bounded Rust edits:
  Sol Medium; loss/gradient/checkpoint/stop state: Sol High; independent design,
  formula and acceptance: Astra High once. Use a supported equivalent if absent.
- Default xhigh/Max/Ultra off when a real setting control is available. Escalate
  for the same unresolved reproduction after two fixes, nonlocal state corruption
  or an independent numerical-oracle mismatch. Narrow xhigh follows an unresolved
  High investigation, never a routine compile typo or report-format correction.
- Prompt wording does not change settings. Record requested and observed model /
  effort separately; without a live control report UNCHANGED or UNKNOWN. Do not
  overwrite global configuration, install services, use API keys or lower guards.
- When delegation is explicitly authorized or required, use one implementer then
  one independent reviewer sequentially. Use a concise handoff with source/parent,
  invariants, changed paths, exact commands, results, budgets and next action.
  Do not create a standing router/management agent or self-issue reviewer PASS.
- Normal learning needs no repeated reasoning session. Keep logs on disk and
  re-enter at scheduled evaluation, command exit or actual error; obey required
  progress cadence without dumping raw output. Keep normal progress within eight
  lines unless the user's request or a consequential failure requires detail.
- Reuse unchanged source/binary/input-bound checks; never reduce required dynamic
  verification or run a duplicate closure review without a new mismatch/change.
- Record observed stage model/effort, agent and tool-call counts, rework and result
  in the existing status/report when available. Codex context/input/output/
  reasoning/cache usage is separate from Replica input/target/generation tokens.
  Missing usage is UNKNOWN; do not infer savings percentages or add these totals.
- Keep a running experiment alive across routing changes unless its own stop
  condition or the user requires cancellation. No automatic budget extension.

## Artifact hygiene in this repository

- Copy only required source/fixtures for a review; read original evidence in place.
  Reuse a managed Cargo target sequentially after checking toolchain, target,
  features and profile. Verify exact source/binary identity before using results.
- Preserve the actual executable, dynamic dependencies, unique readers/patches,
  commands, environment, logs and raw locations. Keep compiler intermediates
  separate and nominate only proven disposable files after the task closes.
- Inventory once and reuse metadata. A target/scratch name is not deletion proof.
  Default to a path/hash-bound dry-run plan; deletion needs explicit approval of
  that plan and fresh identity/reference/writer checks. Preserve unknown files.
- A Git commit replaces a source snapshot only when its exact tree, local diff
  and untracked inputs are preserved. It does not restore model/raw evidence.
  Never rewrite an immutable report or receipt to describe cleanup.
