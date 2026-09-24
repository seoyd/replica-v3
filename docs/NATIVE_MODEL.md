# Native model implementation

The2026-09-24 protected adaptation profile adds only q/v rank8 updates to the
existing F32 model: `xWᵀ + (8/8)(xAᵀ)Bᵀ`. It keeps base tensors constant and
registers only A/B for gradients and Adam. All input types share that adapter,
including prefill and cached decode. Frozen base identity and active output
retention are separate tests. See the active recovery plan for bounded budgets;
this profile is not automatically the product default or an accepted model.

Its R3BIN delta stores typed base file/content/architecture/tokenizer/framing
identities, rank/alpha/seed, exact q/v shapes, little-endian F32 A/B and moments,
their hashes and the existing training manifest. The loader reuses common native
metadata validation, then validates the adapter registry and effective/base/Adam
clocks. It loads original inference tensors without historical full Adam.
Missing delta, changed base, wrong objective/profile and pending publication
fail closed. Full merged weights are not emitted.

The separate `R3-FP4-E2M1-B32-F32S-EXPERIMENTAL` research tool packs selected
matrices in32-value row blocks with F32 scale and nearest-even-code ties using
F64 distances. It preserves signed zero, uses scale1 for zero blocks and minimum
positive subnormal scale on underflow. Norms and the single tied embedding stay
F32. This is neither MXFP4 nor NVFP4. Inference fully dequantizes to the ordinary
F32 CPU path; it provides no FP4 GPU or training performance claim.

Current model on2026-09-19: the fresh joint baseline trained4096 updates from
seed17/random weights and zero Adam. Own train-only tokenizer vocab562;
SMALL parameters9,513,408; CPU F32 Accelerate thread1. Existing6-layer TR++
(pre/QK RMSNorm, RoPE, GQA8/2, SwiGLU, tied embeddings, local5/global1) is unchanged.
The bounded run ended with primary392/512 and transfer40/128, both below gates.
The current native artifact is `artifacts/fresh-joint-20260919/segment-0005/final`.
No accepted S4 candidate, S5/S6 result or Goal1 readiness follows. Source/weights/
physical hashes and exact counts are recorded in EXPERIMENT_STATUS.

2026-09-19 active storage update: the user deleted all local learned data and
retained Rust source/Git history. The run results below are historical, not
currently available artifacts. Project serialization now uses R3BIN for former
JSON metadata/tokenizer files/IPC/diagnostics; native model weights and Adam remain
R3MODEL. SQLite is retained for memory, not tensors. Text checkpoint import is
retired with an error. See STORAGE_FORMAT.md for active wire boundaries. This
migration does not train a replacement SMALL model or improve measured quality.

Contract: GOAL1-NATIVE-TRPP-1.0. Verified through S3; S4 learning remains under
investigation after task-quality failures. No final acceptance has been declared.
The following observations are scoped to their named phase, not overall task success.

Historical continuation record (these artifacts were deleted by user instruction):

| Run | Actual bounded training | Validation-only generation | Acceptance |
|---|---|---|---|
| first SMALL |5000 updates, own vocab648|independent final automatic1/200; semantic0/200|FAILED, exposed fixture is regression-only|
| curriculum v2 |5000 updates, own train-only vocab801|selected4500:229/400|insufficient; no new final test|
| balanced v3 |5000 additional updates from v2 step4500|selected9250:250/400|insufficient; no new final test|
| grounding v4 |5000 additional updates from9250; budget ended at14250|selected10500:188/400|insufficient; no new final test|
| counterfactual v5 |2925 additional updates; agent stopped after validation deterioration|selected12500:221/400|insufficient; no new final test|
| evidence-first v6 |4053 additional updates; predeclared validation stop at16553|selected14500:228/400, QA macro0.591544; terminal215/400|insufficient; no new final test|
| record-copy v8 |user-directed stop19271; terminal selected under original rule|terminal126/400, QA macro0.361029; zero generation failures|not qualified; checkpoint preserved|
| entity-cue v9 |100-update auxiliary diagnostic, then bounded500-update mixed QA|auxiliary64/64; mixed selected19750:219/400, QA macro0.467096; terminal211/400|not qualified; remaining number/context/value binding errors|
| field-cue v10 |500-update auxiliary cap reached20250|terminal name0/16, number0/16, context0/16, value9/16|field gate FAILED; no mixed-QA continuation|
| field-pairs v11 |500-update revised auxiliary cap reached20750|original-question42/64: name14, number7, context12, value9 of16 each; training59/64, oracle-question55/64|field gate FAILED; no automatic continuation|
| query-pairs v12 |STOPPED: 1,000-update 예산 종료21,750; 현재 NOT_RUNNING|기존 원본 검증45/400: 보조45/64, 일반 QA0/336; 신규 재평가 아님|QUALITY_FAIL; 전체 목표 기준 유지|
| SMALL32-QA memorization |random initialization; stopped508 after stable exact memorization|400/500 updates32/32 each; fresh terminal reload32/32 train and0/32 validation|memorization PASS only; original task gate still unmet|

The801-token model has9,605,184 parameters and retains the required SMALL architecture.
All these runs use our random initialization lineage and CPU/Accelerate FP32; none uses
an external weight, tokenizer artifact, teacher or inference API. Corpus versions are
separate declared splits with retained history; v3–v6 freeze the v2 training tokenizer.
Validation losses from different corpora or answer distributions are not directly
comparable quality scores. Full manifests, measured resource use, raw outputs and
failed attempts are recorded in IMPLEMENTATION_REPORT.md and its referenced logs.

Native generate/ask/chat and fresh-process replay are connected to the trained logits
path. Intermediate CLI quality was4/14, while committed same-key replay was14/14.
These observations establish neither the required memory-answer quality nor S5 exit.
The next independent final fixture constructor is ready and its two direct regressions
passed; actual fixture creation/evaluation remains pending a frozen validation-selected
candidate. Overall task quality and S6 packed quantization are still unverified.

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
The independent S4 evaluation renderer lives only in examples/validate.rs, outside
the training generator and product inference library.

Observed S2 artifact (synthetic only, seed 41): 2000 train / 200 validation episodes;
train/validation serialized bytes 2,830,265 / 343,636; actual vocab 648. Text supplied
to BPE training: 1,416,629 bytes, 341,878 tokens. Complete framed sequences including
answers/EOS: 650,955 train / 96,548 validation tokens, before training batching.
These are corpus counts, not consumed training tokens. Exact hashes/logs are in
logs/goal1-s2-tokenizer.txt and logs/goal1-s2-tokenizer-manifest.txt.

At the S2 exit, checkpoint/training/heldout/KV/quant/offline measurements were NOT_RUN;
later observations follow below. Heldout quality is not inferred from numeric tests.
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

## S4 operating record (executed, quality failed)

CPU tiny overfit used the same real Transformer/backward/AdamW/checkpoint path. The
network-disabled 500-step run consumed 5,498 input/target tokens on two repeated toy
sequences: initial validation loss 6.48106397, final 0.07490649. It is intentionally
overfit, not independent accuracy. Its final artifact is
`artifacts/goal1-tiny-overfit-offline/final`; raw results are in
logs/goal1-s4-tiny-offline.txt. An earlier sandbox run failed with EPERM because it
attempted to execute ps; its log and incomplete output directory are retained.

Explicit --no-rss handles that observed platform restriction: sampled RSS is None,
not zero. External /usr/bin/time -l measures resident high water and memory footprint
separately. An architecture/batch tensor planning guard is an estimate, not an observed
peak. The ordinary RSS sampling/stop path remains enabled without the option.

The fixed SMALL run uses the existing 2000/200 episode corpus and own 648-token BPE,
random initialization seed17, CPU F32, length512, 5000-step/20M-input-token bounds,
microbatch1/accumulation1, LR0.001/warmup100 and validation every250. Configuration was
recorded before the run/final test in logs/goal1-s4-operating-config.txt. Checkpoints
are selected only by validation loss. The launch source manifest is
logs/goal1-s4-training-source-digest.txt. Apply goal1-s4-training-source.patch to
commit 23cc5b0178048eb7b23775fefb3d9282c1f91152 to reproduce those source bytes;
both reconstructed changed files matched the pre-launch SHA-256 entries exactly.
Subsequent evaluator-only additions do not change the running training executable.

The separate validation generation diagnostic at step750 had 0/25 exact matches;
outputs often had an incorrect value or an unprovided event ID. This is evidence of
failed task learning/generalization, not a final heldout result. Numeric cached/full
forward parity on that trained checkpoint still passed all six lengths through2048
(maximum observed logits difference 4.053116e-6). Evidence: logs/goal1-s4-validation-750.*
and goal1-s4-trained-cache-parity.txt. No quality waiver or scripted answer is applied.
The same checkpoint's first25 train episodes had3 exact matches, all empty-evidence
uncertainty responses (logs/goal1-s4-train-diagnostic-750.*). Failures therefore already
occur on training-format questions, before introducing heldout wording or DB retrieval.
This diagnostic does not prove a single cause; it identifies evidence/value/ID learning
as unresolved despite the passing numerical pipeline checks.

The final evaluation renderer has five categories of40 cases. Its question/value/time
expectations are independent of the training generator and remain outside the product
library. Scoring checks required values, temporal citation IDs, contradictory values,
blanket uncertainty, missing/invalid citations and causal overclaims. Actual Korean
outputs still require separate semantic inspection; these checks are not a language
understanding proof. Comparisons use the random artifact, removed evidence, swapped
values with identical questions, and actual Store::search without gold-ID seeds.
The last comparison measures top-1 retrieved value, explicitly not answer accuracy.

The run ended normally at its fixed5000-step budget after1686.704 s, consuming1,622,873
input tokens and56,407 supervised targets. Last train loss was0.57465279; final validation
loss1.76776054. There were5000 sampled episodes with replacement from2000 distinct
corpus episodes (average2.5 draws per episode, not2.5 complete deterministic epochs).
Minimum validation loss1.25634503 selected step750 before final fixture creation;
that selected artifact consumed243,473 inputs/8,425 targets. Final and selected artifacts
are different: final tensor SHA4204371e1672571987192029d08772651fee67884c730261041d0d7d2a240369,
selected SHA73689b0104c750f3aec94079f5eed81fbadfacca523f860169da0aa756f44487.
Selection evidence is logs/goal1-s4-selected-candidate.txt. All later checkpoints and
failures remain local; none was substituted based on final-test results.

Heldout seed872341 was rendered only after training/selection. Its SHA-256 is
1a0a41758064ebc01d13cd4f983f26a167e2af992bb5a956c546994bf8dcda8c. Each category has40
cases. The trained automatic counts were[0,0,0,0,1], total1/200 (0.5%), versus required
190/200 and36/40 each. Of200 outputs,199 had unprovided event citations and were rejected.
The one automatic hit (case198) was also semantically invalid: it asserted an unrelated
direction, repeated broken fragments and ended in a stray number/bracket. Implementer
inspection accepts0/200; independent review remains pending. The raw automatic1 is
preserved rather than silently relabeled. This exposes a false positive in the simple
uncertainty-keyword rubric; the rubric is not a semantic validator or production guard.

| Input/model comparison | Automatic correct /200 | Provided-citation rejections | Interpretation |
|---|---:|---:|---|
| Selected trained | 1 | 199 | sole automatic hit fails semantic inspection |
| Random initialization | 0 | 0 |150 invalid UTF-8 outputs; other outputs unrelated/repetitive |
| Evidence removed | 1 | 117 |same invalid automatic hit; no demonstrated evidence benefit |
| Correct evidence value swapped | 1 | 199 |147/160 known-value pairs unchanged; both-correct pairs0 |

These are four variants of200 cases, not800 independent test questions. Complete
actual tokens/text/provided/excluded/citations/digests/timings and failures are in
logs/goal1-s4-heldout-{trained,random,no-evidence,value-swap}.jsonl and accompanying
text logs. Source/evaluation fingerprint: logs/goal1-s4-final-source-digest.txt.
Actual Store::search top-1 value hits were133/160 known-value cases, with no gold-ID
query seeds. This baseline stores raw observations, not canonical version chains;
it does not certify version-aware answer accuracy. It did not generate an answer.

Trained evaluation ran in one new process with CPU/F32 and fresh KV per question,
network denied. Model load380.750 ms;200 generations totaling2363 tokens including EOS,
27.001 s including prefill (87.52 aggregate tokens/s). TTFT min/median/max34/103/149 ms;
generation58/136/184 ms. All200 stopped at EOS. These throughput numbers describe failed
answers, not useful-answer performance. Filesystem caches were not flushed; no true
OS-cold claim. Random/no-evidence runs overlapped briefly and their raw timings are
not a controlled latency comparison. Trained and value-swap runs were separate.

External time measured training maximum RSS1,676,378,112 bytes and peak memory
footprint1,504,954,072 bytes; trained inference RSS449,429,504 bytes and footprint
446,841,600 bytes. Do not add these metrics. GPU/shared allocations were not measured.
Selected safetensors are114,575,488 bytes including master weights, two Adam moments
and headers; tokenizer16,636 bytes; manifest12,943 bytes. Raw model F32 weights alone
are38,185,728 bytes. There is no native-memory DB measurement yet.

S4=FAILED; TRAIN_PIPELINE_VERIFIED=YES; GOAL1_TASK_PASS=NO. This bounded run is preserved,
not published as a completed phase. Validation diagnostics identify failed value/ID
learning even on train questions; cached/full parity passed, and final evaluation fed
evidence directly, so these failures occur without DB retrieval. Data/optimization
improvement remains necessary; no single cause is claimed proven. The final fixture
is now exposed and must become a regression set if a later change uses these observations.
Any later candidate requires fresh independent final evaluation.

At the first experiment's closure, native memory integration and quantization were
unimplemented. The continuation has replaced the former external adapter with the
own-model worker and added native generate/chat/history interfaces. S5 acceptance
still requires the final qualified model and actual memory/restart trials. S6 is
not implemented. There is no accepted product model or Goal1 pass.

## Objective-owned RESUME (2026-09-18)

The current writer uses R3MODEL wire2; the reader explicitly supports wire1 and2.
The same F32 weights and tokenizer mapping remain the inference model identity.
RESUME v2 requires a typed ResumeBinding in TrainingState, with semantic revision,
response-CE, normalized-span, summed-pair or separate-side divergence family, exact first-target/alpha IEEE bits,
normalizer/annotation, ordered training source, tokenizer/framing/config/corpus,
and executed policy/provenance digests. The descriptor has a domain-separated
digest; it does not recursively hash its containing checkpoint. Adam and all
step/token/sampler/config fields retain the existing encoding and validation.

Generic training accepts only an exact supported default descriptor. Native study
resume compares its full registered descriptor, including actual continuation
policy; nondefault checkpoints cannot silently become default after file moves.
Wire1 has unknown objective, not implicit default. Inference and read-only recount
continue to work. `recovery native upgrade-resume` needs explicit, verified original
policy/native/terminal/close lineage; it writes a new file and preserves the old
physical hash. Default and SPAN migrations are distinct from resume permission.
A bound native policy remains unsupported by generic train even if its objective
family is response CE; use the matching authorized native path.

No topology, QK/RoPE/GQA/SwiGLU/KV equation, tokenizer mapping or optimizer formula
changes accompany this format version. Storage parity is not intelligence recovery.

## 현재 제한 진단·native 저장 경로 (2026-09-17)

위 run/JSON+safetensors 설명은 당시 S3/S4 기록이다. 현재 기본 runtime/trainer 저장은
STORAGE_FORMAT.md의 단일 자체 binary이다. INFERENCE는 F32 가중치+config+자체 tokenizer+
출처만, RESUME은 같은 모델에 Adam m/v와 정확한 scheduler/sampler/RNG/data/budget 상태를
추가한다. SQLite는 모델 tensor 저장에 쓰지 않는다. 구형 checkpoint는 보존하고 명시
import 명령에서만 읽는다. native runtime은 JSON을 재구성해 parser에 넣지 않는다.

이번 SMALL 수식은 바꾸지 않았다: pre-RMSNorm/QK-RMSNorm/RoPE/GQA/SwiGLU/tied embedding,
local5/global1이다. semantic equation/parameter/state ID와 kernel ID를 분리했다.
같은 계산의 tiled/fused kernel은 검증된 오차/gradient/cache 호환 범위에서 weight를
재사용할 수 있다. 같은 shape라도 다른 수식은 새 equation ID·cache 폐기·재평가가 필요하고,
attention→SSM 같은 변경은 weight/state 직접 이식이 보장되지 않는다. 원문 evidence는
이들과 독립적으로 남는다. 현재 interface는 여전히 Candle Tensor를 받으므로 Candle 전체를
교체했다고 주장하지 않는다. CPU F32 decode 전용 자체 Rust GEMV 한 후보만 실측했고,
느려서 기본은 기존 differentiable reference다. 학습/prefill은 reference를 유지한다.

고정16개는 R-A random seed17에서600 updates, R-B 일반 QA parent에서200 updates로
마지막 두 평가와 새 process 각각16/16·그룹4/4를 통과했다. 같은 구조/tokenizer/LR와
새 Adam을 썼으며 update마다16개 전체를 포함했다. R-B의 학습 전 동결 새64 평가 단1회는
0/64·그룹0/16이었다. 암기는 됐지만 새 ID/값/형태 전이는 실패했으며 단일 원인을 확정하지
않는다. 추가 대규모 학습/자료 확장은 없고 S4=QUALITY_FAIL, GOAL1_READY=NO다.

상세 의존·기존 tensor inventory는 DEPENDENCY_STORAGE_AUDIT.md, 새 저장/실측/진단과
기여·한계·정확한 hash는 EXPERIMENT_STATUS.md에 기록했다. 외부 모델/학습 tokenizer/
teacher/API는 쓰지 않았고 프로젝트 소스는 Rust다. SQLite/zstd/onig의 native C 하위
의존은 유지하며 순수 Rust 의존 트리로 표현하지 않는다.
