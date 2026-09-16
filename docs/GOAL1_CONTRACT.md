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
* S5: own artifact replaces the external model in the existing worker/app path; CLI for
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

## R3-CUSTOMIZE-AND-DIAGNOSE-1.0

이번 승인 범위는 기존 v3/S4 WIP를 보존한 제한 진단과 저장·연산 교체 경계 구현이다.
앞의 Goal 1 품질 기준을 낮추거나 이 계약 완료로 Goal 1 완료를 대체하지 않는다.
프로젝트 소스·학습·변환·평가 도구는 Rust이며 외부 모델/학습 tokenizer/teacher/API,
가짜 loss/가짜 모델 대체/답변 하드코딩을 금지한다. 기존 범용 crate와 시스템 backend는
native 하위 의존까지 공개하고 재사용한다. 대규모 QA 학습, 크기 증가, LR/seed sweep,
무제한 데이터 확대, 범용 DB/텐서 framework 재작성은 이번 범위가 아니다.

진행 DAG: `P0 → {P1,P2}; P2 → {P3,P4,P5}; {P1,P2,P3,P4,P5} → P6`.
heavy 작업은 한 번에 하나이며 P1 실행 중 source를 변경하지 않는다. 진단의 품질 실패는
정직한 결과로 닫을 수 있지만 품질 PASS로 부르지 않는다. DELIVERABLE_VERIFIED,
MODEL_QUALITY_PASS, GOAL1_READY를 구분한다. 독립 검토는 INDEPENDENT_PENDING이다.

- **P0**: 실제 HEAD/dirty scope/hash/환경/features와 run별 parent, 시작·종료 step,
  input/target, sampler, 종료 이유를 조사한다. 미확인 값은 UNKNOWN이다. stale 실행
  상태를 정정하고 사용자 DB/credentials를 탐색하지 않는다. 각 활성 crate의 exact
  version, 직접/전이, feature, 실제 사용, native 의존, 비용/대체 판단을 표로 남긴다.
  inference/resume/기억/개발 cache를 분리한다. 모든 tensor 이름·shape·dtype·bytes·역할,
  safetensors JSON header와 외부 JSON, 실제 read/copy/hash/RSS를 계상한다. 합성 DB의
  canonical/FTS/관계/dbstat/freelist/WAL/SHM/backup을 중복 합산하지 않고 FULL을 유지한다.
- **P1**: 기존 실패16을 원본 byte와 4그룹×4변형 그대로 동결한다. serialized input만
  읽는 별도 validator로 answer 유일성, 역할/token IDs, evidence status/time, citation/EOS를
  확인하고 모호함은 DATA_AMBIGUITY로 남긴다. 기존 반복 검증셋은 DEVELOPMENT이다.
  새 그룹64는 학습 전에 ID/값/순서와 group split을 동결하고 성공 후 단 한 번 평가한다.
  학습 전 prompt ID, batch1/그룹, cached/uncached/chunk, 255/256/257과 실제 경계,
  causal/padding/local/QK/head/RoPE, label shift/target mask/EOS/분모/gradient를 검사한다.
  첫 정답 분기 logit/probability와 teacher-forced/greedy를 구별하며 argmax/heatmap만으로
  입력 무시/근거 사용을 단정하지 않는다. support-only는 길이와 distractor 혼동을 명시한다.
  R-A는 SMALL random seed17, R-B는 provenance가 확인된 일반 QA checkpoint이며 v12
  auxiliary 후보는 제외한다. 같은 새 Adam, LR .001/warmup20, 동일 구조/tokenizer/F32로
  update마다 16개를 정확히 한번씩 노출한다. 그룹 보존 microbatch≤8와 target-token 가중
  accumulation을 사용한다. run당 1,000 updates/3,200,000 input tokens/45분 중 먼저 닿는
  상한에서 정상 저장·종료한다. eval0,100,...에서 전체답/value/recordID/EOS와 그룹을
  분리 기록한다. 마지막 두 평가와 새 프로세스16/16·4/4가 진단 통과 조건이다. 실패 후
  명확한 한 원인 수정 R-C 최대1회만 가능하며 근거 없는 반복은 금지한다.
- **P2**: 수식, 실행 backend, artifact, evidence의 네 책임만 작게 구분한다. OperatorSpec의
  family/equation/parameter/state/numeric ID와 training/backward/prefill/decode/cache/dtype/
  device/context 능력을 명시한다. semantic architecture, kernel, weights, tokenizer semantic,
  wire, cache schema/실제 history binding은 별개다. 동등 구현 둘을 실제 호출하고 weight,
  gradient/cache parity를 검사한다. 동일 shape의 다른 수식은 새 ID/재평가, attention→SSM은
  converter 또는 random-init가 필요하다. evidence는 모델 독립이다. SMALL/TINY는 유지하고
  experimental config에 명시 상한을 둔다. 미사용이 입증된 legacy direct dependency만 제거한다.
- **P3**: JSON 없는 자체 binary 하나의 format에서 INFERENCE와 RESUME을 타입/CLI로
  구분한다. 전자는 model/config/exact tokenizer/lineage, 후자는 Adam 두 moment와 모든
  최신 TrainConfig/state/RNG/scheduler/budget/group/중단 상태를 추가한다. source 품질은
  상속하며 freeze/optimizer 경계에서만 저장한다. legacy는 불변 explicit import 전용이다.
  기본 runtime은 native load→실제 generation이며 JSON 재구성 우회를 금지한다.
  기존 canonical integer/bytes codec을 별도 model schema에 재사용한다. magic/version/kind/
  flags/LE, bounded header/directory/count/total, schema/equation/tokenizer/lineage, tensor
  dictionary/dtype/rank/dims/offset/length/codec/digest, 정렬을 명세한다. contiguous F32 raw가
  기본이며 inference가 Adam을 읽거나 할당하지 않게 한다. 중복/overlap/shape overflow/
  unknown kind/dtype/truncation/trailing/checksum/길이/merge 오류는 할당 전 거부한다.
  byte vocab/ordered merge rank/special IDs/numeric segmentation을 그대로 보존한다.
  semantic/wire ID 변경은 검증한 migration map을 기록한다. 같은 FS create_new temp,
  write+validate+sync, no-clobber publish, parent sync를 사용하고 자기 temp만 정리한다.
  literal fixture/hand-encoded bytes, bitwise tensor/token parity, 새 프로세스 logits/text,
  JSON/safetensors/DB 없는 생성, interrupted publication, 연속 N-step 대비 binary split
  resume의 weights/Adam/sampler/loss/생성을 검증한다. inference로 resume은 오류다.
  F16/INT4는 필수 F32 이후 선택 사항이고 미구현이어도 이번 F32 deliverable은 가능하다.
  저장 양자화만을 packed kernel 가속/S6 완료라고 부르지 않는다.
- **P4**: 같은 SMALL/weights/prompt에서 prefill/decode별 shape/stride/dtype/thread/warmup/n/
  복사·할당을 측정한다. GQA 확장 복사, decode GEMV, RoPE/KV 재사용 중 실측 한 후보만
  Rust 또는 동등 연산 재배치로 시험한다. 기본은 reference이며 inference-only 연산은 학습
  거부/명시 fallback한다. 사전 tolerance, hand/random/zero/odd/tail/noncontiguous 검증,
  필요 backward/finite-difference, logit/cache parity와 같은 M4 조건 micro/E2E 측정으로
  채택 또는 KEEP_REFERENCE_OR_REJECT를 결정한다. 새 C/C++/Metal shader는 금지한다.
- **P5**: 기존 canonical event/codec/관계를 재사용하고 immutable 원문/ID/source/observed/
  valid/current/retract/supersede/restore/causal hypothesis 의미를 보존한다. SQLite의 일관된
  snapshot 전체 id/body를 byte-cap block+offset index의 binary archive로 export한다.
  header에는 source/snapshot/count/codec/length/hash, block integrity와 bounded decode를
  둔다. raw/zstd는 실제 측정 후 선택한다. Rust reader는 SQLite를 열지 않고 get/history/
  as_of/current와 scope/time/cycle/hop/visit 제한 incoming/outgoing graph, truncated를 제공한다.
  index는 archive에서 재생성한다. 정확히 같은 의미의 lexical subset만 비교하며 BM25/trigram
  미지원은 NOT_COMPARABLE로 센다. 반복 한국어/긴/난수/정정/철회/restore/다른 context/
  무관 지시를 포함한 같은 10,000 사건으로 bytes/IDs/관계/검색, 새 process archive-only,
  durable export/index build/cold-process·warm latency/RSS/DB open·closed/backup을 측정한다.
  LIVE_SQLITE_REPLACEMENT=NO; read-only snapshot은 live append/transaction/recovery 대체가 아니다.
- **P6**: 영향받은 RV/경계와 direct tests, fmt/check/clippy/release, 실제 native 생성/export
  parity/exact binary resume/contrast16/DB-free/kernel 실행을 대조한다. 단계별 실제 명령,
  source/hash, 관측/한계/다음 의존을 한국어로 보고한다. 미실행을 성공으로 쓰지 않는다.
  이전 WIP를 새 기여로 계산하지 않는다. 검증된 관련 소스/작은 fixture/계약/익명 통계만
  commit하고 기존 origin/main에 정상 push한 뒤 full remote SHA를 확인한다. 충돌은
  PUSH_BLOCKED이며 force/reset/stash/clean/rebase로 덮지 않는다. 큰 모델/원문 로그/사용자
  DB/개인자료/임시 지시문/target은 올리지 않는다. P1 실패, 저장 완료, Goal1 완료는 별개다.

단계 상태와 필수 결과 필드는 [EXPERIMENT_STATUS.md](EXPERIMENT_STATUS.md), 의존성과
저장 실측은 [DEPENDENCY_STORAGE_AUDIT.md](DEPENDENCY_STORAGE_AUDIT.md)에 유지한다.
