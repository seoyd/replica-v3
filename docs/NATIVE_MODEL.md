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
