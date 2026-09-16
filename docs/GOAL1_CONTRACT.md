# GOAL1-NATIVE-TRPP-1.0

Extend the existing Rust 1.98 v3 memory application. This contract replaces B0's
external pretrained model/no training scope. Historical failures and measurements
remain historical; independent acceptance is pending.

Required path: authorized local corpus (synthetic educational Korean when none is
supplied), train-only byte BPE, random native Transformer initialization, actual
backward/AdamW training, durable checkpoint, fresh-process restoration, persisted
question, bounded retrieval, one neural generation, checked provenance, committed
answer, then display. Memory stays in the existing binary SQLite records. Explicit
corrections/restores never replace original bytes. Model output has no tools or
memory write authority. Failure has no canned answer or automatic retry loop.

No external weights, adapters, embeddings, model classes, teacher, inference API,
server, automatic downloads, Python trainer/bridge, or private corpus collection.
Locked generic Rust tensor/autograd/tokenizer/SQLite dependencies are permitted.
Training generators and independent expected answers stay outside the inference
dependency graph. User conversations are not automatically trained.

Required phases, in order:

* S0: capture actual environment/baseline, preserve local changes, establish this
  independent permanent contract, separate temporary instructions from delivery.
* S1: canonical result kind/input/scope/session validation on both replay paths;
  full untruncated prompt packing with exact evidence partition; truthful search
  cap flags; atomic startup/head observations; pinned online backup snapshot and
  safe failure handling. Direct independent regressions plus v3 regression gate.
* S2: bounded corpus manifest with permission, hashes, bytes/tokens/counts and
  document/episode split before chunking; separate entity/value/template/sequence
  heldouts; own reversible 256-byte BPE, target vocab 4096, trusted role IDs, no
  normalization/truncation/padding, strict decode and artifact binding.
* S3: native decoder, 6 layers, hidden 384, 8 query/2 KV heads (48/head), FFN 1024
  SwiGLU, pre-RMSNorm and headwise QK-RMSNorm, absolute RoPE, bias-free projections,
  tied embedding/output, five local layers (window 256) and one global layer,
  context 2048, default 256 new tokens, training length 512. Test masks/GQA/actual
  gradients, reference/bounded KV parity, masked CE denominator, clipping/AdamW/
  schedule/accumulation, strict atomic tensor checkpoints including optimizer and
  sampler state, fresh-process exact resume and malformed artifact rejection.
* S4: actual tiny overfit and small-profile learning, validation-only selection,
  then >=200 independent autoregressive heldout answers, 40 per memory category.
  Targets: >=95% overall, >=90% each, zero accepted invalid citations. Compare
  random, no evidence, value swaps and retrieval-only. Report synthetic scope and
  failures; never waive the quality threshold or select checkpoints on final test.
* S5: own artifact replaces external GGUF in the existing worker/app path; CLI for
  corpus/tokenizer/model init/inspect/train/resume/evaluate/generate/ask/chat; five
  actual memory/version/context/causality questions and fresh-process restart,
  replay, invalid citations and runtime/commit faults. Preserve existing commands.
* S6: grouped INT4 W4A16 export/load, block numeric reference and actual inference;
  float comparison (adoption requires <=2 percentage-point accuracy loss), tensor/
  cache/working-memory sizes, M4 load/TTFT/throughput, network-disabled execution,
  final requirement comparison and verified publication.

Start with FP32 weights/state and CPU correctness; select Metal only after complete
forward/backward/update probes. Initial guards: inference 12 GiB, training 16 GiB;
per run <=5000 optimizer steps and <=20 million consumed tokens. Probe before fixing
operating configuration. Cancellation/nonfinite/budget/OOM leaves recoverable state
and an explicit reason; no unbounded training. Checkpoint includes architecture,
tokenizer/weights/source/corpus hashes, lineage, tensor names/shapes/dtypes and exact
training state. Unknown/missing/mismatched/nonfinite tensors must fail closed.

MLA, MTP, FP4, latent thought and low-bit KV remain explicitly unsupported unless
separately implemented/tested. INT4 is not FP4. No broad intelligence claim.

PLAN.md owns S0–S6 and T-R01–05, T-N01–08, T-I01–04, T-D01–02 status/evidence.
Only actual executed tests/measurements count. Publish verified phase source/tests/
docs by explicit staging and non-force push to the verified repository branch;
never publish private corpus/DB/weights or incomplete phases as completed.
Independent review remains PENDING and GOAL1_ACCEPTED=NO until granted externally.
