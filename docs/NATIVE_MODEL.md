# Native model implementation

Contract: GOAL1-NATIVE-TRPP-1.0. Current completed phase: S2 (data/tokenizer).
No neural training or task-quality result is implied by tokenizer completion.

The training executable is `replica-train`; its private `data` module owns synthetic
answer rendering. The inference library/worker does not import it. Product prompts
use ByteBpe::prepare and PreparedPrompt: trusted BOS/role/end IDs are inserted as
integers, while every untrusted byte is encoded through BPE and can only produce
IDs >=8. The raw strings `<assistant>` and reserved Unicode spellings never acquire
control meaning. Decode to text rejects invalid/incomplete UTF-8 instead of replacing
bytes. The byte API can roundtrip all 256 byte values. Canonical memory is unchanged.

Tokenizer v1 maps byte b to U+0100+b, learns BPE solely from train documents with
min_frequency=2 and max merged length=32 bytes. ASCII separators and each decimal
digit form separate segments (all retained). Eight reserved symbols use U+E000..E007,
outside the mapped alphabet: PAD/BOS/EOS/system/user/evidence/assistant/end. No
pretrained vocabulary, added-token recognizer, normalizer, truncation or padding.
Vocab is contiguous and 264..4096, never padded with fake merges. Artifact load checks
size, schema, all byte fallbacks, IDs, token lengths and merge consistency. SHA-256
binds exact artifact bytes and source train split. Config/weights binding is S3.

Corpus v1 is bounded, explicit training data, not canonical personal memory. Each
synthetic episode carries category/binding/template-family/sequence IDs. Train and
validation have disjoint entity pools, different question families and episode shapes;
validation is not used to train tokenizer merges. Local documents are read only when
explicitly passed as --local; each whole document enters train with its content hash
as ID. No recursive collection/automatic personal-memory training. Per-split hashes,
serialized bytes/document counts, generator revision and seed are in manifest.json.
Counts requiring tokenization are added to the tokenizer's adjacent manifest, retaining
all original corpus metadata. No final test was used for tokenizer/model selection.
The independent final evaluation renderer remains to be implemented in S4.

Observed S2 artifact (synthetic only, seed 41): 2000 train / 200 validation episodes;
train/validation serialized bytes 2,830,265 / 343,636; actual vocab 648. Text supplied
to BPE training: 1,416,629 bytes, 341,878 tokens. Complete framed sequences including
answers/EOS: 650,955 train / 96,548 validation tokens, before training batching.
These are corpus counts, not consumed training tokens. Exact hashes/logs are in
logs/goal1-s2-tokenizer.txt and logs/goal1-s2-tokenizer-manifest.txt.

Actual checkpoint/training/heldout/KV/quant/offline-device measurements: NOT_RUN.
MLA, MTP, FP4, latent thought, low-bit KV: NOT_IMPLEMENTED. No broad intelligence claim.

## S3 native decoder and training state

The generic Candle tensor/Var operations in neural/transformer.rs own every block.
SMALL is exactly 6x384, Q8/KV2/head48, FFN1024, five local256 layers + one global2048,
pre-RMSNorm, headwise learned QK-RMSNorm before RoPE, attention scale 1/sqrt(48),
bias-free SwiGLU/projections and tied embedding/output. All reductions/softmax/CE,
master weights and Adam moments are F32. Dropout is zero. SMALL has **9,546,432**
parameters with the actual 648-token vocabulary. TINY_NUMERIC_TEST_ONLY is a distinct
2x32 numerical fixture profile, never reported as the SMALL trained model.

Masks use absolute positions, causal/local bounds and explicit padding. Fully masked
rows have zero attention. Persistent K/V retains KV heads only; temporary head expansion
is separate. Chunked prefill (128 tokens) evicts local history after each whole chunk,
retaining <=256; global cache keeps all <=2048 tokens. Cache binds weights/config,
tokenizer and scope, and resets explicitly. This bounded concat implementation is not
claimed optimal. Real SMALL reference/cache probes at 255/256/257/513/1024/2048 tokens
had max error <=1.252e-6; at 2048 retained [256,256,256,256,256,2048], K/V 2,555,904
bytes and largest cached attention tensor 8,388,608 bytes. The latter is not total
workspace/allocator peak. Raw observations: logs/goal1-s3-small-boundaries.txt.

Teacher forcing uses input[:-1] -> labels[1:]. SFT loss counts response/EOS targets
only, LM loss counts valid next tokens; prompt/padding never enter the denominator.
Microbatch gradients accumulate weighted by actual target count, then normalize,
clip by global norm and apply project-owned AdamW with warmup/cosine LR. Scalar
reference tests check AdamW/clipping; finite-difference probes check embedding,
attention and FFN gradients. CPU is explicit. Metal forward/backward/step: NOT_RUN;
no GPU throughput claim or silent device fallback.

Checkpoint directories are create-new and immutable from the trainer's perspective.
They contain tensor-only safetensors, own tokenizer, and a bounded manifest published
last with an atomic no-clobber hard link. Incomplete directories lack manifest.json;
previous checkpoints are never replaced. Strict load validates exact tensor set,
names, shapes, F32 dtype, finite values, hashes, tokenizer, architecture and state.
No missing tensors are randomly filled. Resume includes Adam m/v/step, LR configuration,
sampler RNG position, data hashes and consumed/target token counts. CPU 6 uninterrupted
steps versus 3 steps + fresh-process resume to 6 produced identical tensor-file hashes.
An observed optimizer step followed by SIGINT preserved a loadable boundary checkpoint
and nonzero cancellation exit. Nonfinite step computations reject before weight update;
initial/periodic boundary checkpoints remain available. SIGKILL/OOM can leave only the
last published checkpoint, not an exact snapshot of uncommitted gradient accumulation.

Observed SMALL CPU probe: 2 steps, 622 consumed input tokens / 26 supervised targets,
train loss 6.58497810 then 6.48413992; independent 200-episode validation loss
6.68363942 -> 6.23209999. First gradient norm 29.29214996 and weight delta L2 0.03082849;
initial and updated hashes differ (raw log). Total 48.757 s includes two full validation
passes and checkpoint writes. Largest post-step RSS sample 772,352 KiB; transient peak
and GPU/shared allocations NOT_MEASURED. This is a probe, not a quality pass.

S3 direct exit: 11 actual tests (7 native, 3 process-training, 1 trainer unit) passed.
No heldout task quality, real memory integration or quantization acceptance yet.
