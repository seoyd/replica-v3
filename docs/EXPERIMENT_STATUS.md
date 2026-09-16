# 진단 및 구현 상태

계약: R3-CUSTOMIZE-AND-DIAGNOSE-1.0. 2026-09-17.
모든 성공 표시는 구현자 확인이며 INDEPENDENT_PENDING이다.

현재 신경망 학습 프로세스는 NOT_RUNNING이다. 기존 v12는 step 21,750에서
BUDGET_REACHED로 종료했고, 신규 contrast R-A/R-B는 각각600/200 updates에서
두 번 연속 memorization 통과 후 정상 종료했다. 대규모 QA 학습은 재개하지 않았다.
이전 문서의 실행 중 표현은 종료 로그/manifest보다 오래된 상태였다.

| NODE | STATUS | SOURCE_CHANGED | EXECUTED_COMMANDS | OBSERVED_RESULTS | FILE_HASHES | LIMITATIONS | NEXT_DEPENDENCY |
|---|---|---|---|---|---|---|---|
| P0 | VERIFIED / PUBLISHED | examples/validate.rs 감사 명령; 한국어 조사/상태 문서; 선행 WIP 테스트의 import/최신 Rust lint 수정 | offline tree/metadata, release build, storage-audit/load-audit/measure; 직접 회귀38개+trainer unit7개; fmt/all-target clippy; git push/ls-remote | tensor 204개 및 합성 SQLite 실측; 회귀45개 통과; 원격 full SHA 일치 | ce48514ab5fc8e76a9552ce5fabe7ce1617ff4ac | 이전 S4 WIP를 새 성과로 계산하지 않음 | P1, P2 |
| P1 | VERIFIED; 새64 전이 EXPERIMENT_FAILED | src/contrast.rs(training 전용), train_main/training/checkpoint/data | freeze/check/train/evaluate; 신규 unit2개+기존 trainer7개; checkpoint2개/resume1개; fmt/clippy | R-A600/R-B200에서 두 평가 및 fresh reload16/16·4/4; 새64 단1회0/64·0/16 | 아래 P1 source/fixture/binary/final/log hash | memorization만 통과; S4 품질 FAIL; 추가 학습 없음 | P2 |
| P2 | VERIFIED | neural/transformer.rs, neural.rs, tests/native.rs, examples/validate.rs | native9개, fresh-process resume1개, contrast16 재생성, kernel-profile/compare | 기본 수식·학습 gradient 보존; 의미/커널/내용/cache 분리 | 아래 P2/P4 기록 | Candle Tensor 결합 유지; legacy wire ID는 명시 보존 | P3, P5 |
| P3 | VERIFIED | native artifact, checkpoint/default worker/trainer, codec publication, 직접 테스트/문서 | 명시 import/export;384개 fresh-process 상대 대조;binary resume/cancel/kill 및 손상검사;접근 차단 생성 | 모델68+Adam136 bit/state 동일;JSON/DB 없는 실제 생성;inference38,432,768 B | 아래 P3 원본/변환/실행 hash | 품질 미승격;F16/INT4 미구현;IPC/corpus JSON 유지 | P5, P6 |
| P4 | VERIFIED / CANDIDATE_REJECTED | P2의 실제 GEMV dispatch와 비교 명령 | 동일 SMALL, warmup3/n31, CPU F32 단일 thread | 수치/생성 동일 허용오차 통과; decode/전체 생성 악화 | 아래 P2/P4 기록 | 후보 한 개만 시험, 기본 reference 유지 | P6 |
| P5 | VERIFIED | archive.rs, event/codec/store/retrieval/CLI, 관련 테스트 및 측정 도구 | 직접 회귀22개+archive unit3개; 동일10,000건/329관계 raw/zstd 비교; DB 차단 새 process 조회 | 원문/ID/관계 전부 일치; 지원 lexical6/6, 미지원4 별도; DB-free 조회 | 아래 P5 hash/실측 | 읽기 전용; FTS5/live write 미구현; 무작위 조회 SQL보다 느림 | P6 |
| P6 | NOT_STARTED | 없음 | NOT_RUN | 신규 계약 최종 대조 전 | 해당 없음 | 독립 검토 없음 | P1~P5 |

`P0 → {P1,P2}; P2 → {P3,P4,P5}; {P1,P2,P3,P4,P5} → P6`.
한 번에 하나의 heavy 작업만 실행하고 P1 실행 동안 source를 변경하지 않는다.
P1의 품질 FAIL은 독립적인 무손실 저장/보존 구현을 막지 않는다.

## 이전 상태와 이번 기여 구분

BASE_HEAD: `23cc5b0178048eb7b23775fefb3d9282c1f91152`.
BASE_DIRTY_SCOPE_DIGEST: `5b315b8423be0ce4d0e5e605deba5e90a95310b59906a30a23199dd508612ba1`.
시작 git-status digest: `1226021b1f9f1831c39b87537b97e4c624456d00db220f9d9e38fc0594fd4b13`.
로컬의 기존 corpus/학습/평가/CLI 변경은 이전 S4 WIP이다. 이전 제거된
candle-transformers/minijinja도 새 작업 성과가 아니다. 원문 로그와 이전 checkpoint는
불변으로 보존하며 원격 HEAD로 되돌리지 않았다.

v12 validation raw summary를 다시 읽어 확인한 값은 45/400이고 그중 copy 45/64,
일반 QA 0/336이다. 이는 기존 실행 기록 검증이며 새 generation 재실행은 아니다.
v12 train prefix 400의 copy 값 항목은 8/16이었다. 모델 품질은 여전히 FAIL이다.
QA32의 32/32는 기존 별도 memorization 실험이고 새로운 contrast16 결과가 아니다.
support-only 결과는 길이/방해 근거 수까지 바뀌므로 기록 선택 하나가 유일한 원인이라고
확정할 수 없다. 64/336/400 반복 검증 자료는 DEVELOPMENT이다.

## 현재 필수 결과 필드

RESULT: PARTIAL
FINAL_SOURCE_IDENTITY: 아직 최종 아님
DEPENDENCY_AUDIT: 실측 표는 DEPENDENCY_STORAGE_AUDIT.md
UNUSED_DEPENDENCIES_REMOVED: 이번 P0 없음; 이전 S4 WIP 별도
EXTERNAL_WEIGHTS_USED: NO
EXTERNAL_MODEL_API_USED: NO
PROJECT_SOURCE_LANGUAGE: RUST
NATIVE_TRANSITIVE_DEPENDENCIES: SQLite C, zstd C, onig C, Accelerate/OS
CONTRAST16_TRAIN_EM: R-A16/16, R-B16/16; 두 평가 및 fresh reload 확인
CONTRAST16_GROUP_ALL_CORRECT: R-A4/4, R-B4/4
CONTRAST64_HELDOUT_EM: R-B0/64, 그룹0/16; 단1회 실행, FAIL
QA_GENERAL_DEVELOPMENT: 기존 v12 기록 0/336
NATIVE_BINARY_INFERENCE: VERIFIED; 기본 CLI/worker 실제 생성, 품질은 미달
EXACT_RESUME_BINARY: VERIFIED; 6 연속 vs3+3, weights/Adam/RNG/config/loss/logits 일치
JSON_FREE_DEFAULT_ARTIFACT: YES; 단일 자체 binary, legacy 자동 fallback 없음
LEGACY_JSON_USAGE: 명시 importer/audit의 구형 모델/tokenizer/safetensors, legacy lineage hash; tokenizer 학습 도구, IPC, corpus, logs
INFERENCE_BYTES: 38,432,768 B (model raw38,420,736 +header12,026 +padding6)
RESUME_BYTES: 115,285,248 B (model38,420,736 +Adam76,841,472 +header23,008 +padding32)
TOKENIZER_META_BYTES: native6,324 B; 기존 tokenizer JSON22,483 B
KERNEL_REFERENCE: candle-linear-v1, 실제 이번 측정 CPU/gemm F32 (Accelerate feature 미활성)
KERNEL_CANDIDATE: rust-f32-decode-gemv-v1, inference-only
KERNEL_ADOPTION: KEEP_REFERENCE
DB_FREE_ARCHIVE_VERIFIED: YES; 동일 snapshot 원문10,000/10,000·관계329/329
LIVE_SQLITE_REPLACEMENT: NO
CODE_REVIEW_STATUS: INDEPENDENT_PENDING
S4_QUALITY: FAIL
S5_INTEGRATION: 미완료
S6_QUANT: 미구현
GOAL1_READY: NO
COMMIT / REMOTE_SHA: P0 `ce48514ab5fc8e76a9552ce5fabe7ce1617ff4ac`, P1 `8e0a264f3b0996d9a7ca632903e727aef2c5e773` / 각각 동일 원격 SHA 확인
P2/P4 COMMIT / REMOTE_SHA: `29df8c8a0c8f4a6d328c45046446f4418c102600` / 동일 SHA 확인
P3 COMMIT / REMOTE_SHA: `4edf7159518108be302813d704ca0ff5db449395` / 동일 SHA 확인

## 원본 run별 manifest 대조

아래 수치는 Rust u64로 원본을 읽어 기록한다. JSON 도구의 float 변환으로 sampler
정밀도를 잃지 않는다. 각 weights hash를 실제 파일과 대조한다. 시작 token/step은
각 run의 budget_start 필드이며 전체 lifetime이나 다른 branch와 합산하지 않는다.
별도 probe 이후 재개한 실행은 같은 명시 예산 범위 내 update 수로 표시한다.

<!-- lineage-table -->
| run 경로 | 상태 | 시작 step | 종료 step | updates | 시작 input | 종료 input | 종료 target | sampler u64 | weights SHA-256 |
|---|---|---:|---:|---:|---:|---:|---:|---:|---|
| artifacts/goal1-small-qa32-train/final | CANCELLED | 0 | 508 | 508 | 0 | 1217005 | 106609 | 12730656644794535281 | fdfadcb740e49bfdf62f65e5a2414f54c5010c2c8d46543732dd9b7f5d1bad1f |
| artifacts/goal1-small-train/final | BUDGET_REACHED | 0 | 5000 | 5000 | 0 | 1622873 | 56407 | 3134908853477931577 | 4204371e1672571987192029d08772651fee67884c730261041d0d7d2a240369 |
| artifacts/goal1-small-v10-field-train/final | BUDGET_REACHED | 19750 | 20250 | 500 | 32501322 | 33215855 | 2232750 | 4585745042110082553 | 307864aaeb32d2997e09c5f4fee508233058933fbf3b8f958de1754c76706e92 |
| artifacts/goal1-small-v11-field-train/final | BUDGET_REACHED | 20250 | 20750 | 500 | 33215855 | 33898268 | 2246990 | 5212726812805668865 | 4478c75414aae9da3995dd2605b5e7daecf39a5c8a0782f7beaa3b073ad321d8 |
| artifacts/goal1-small-v12-field-train/final | BUDGET_REACHED | 20750 | 21750 | 1000 | 33898268 | 35265622 | 2275614 | 6466690354196841489 | aed892878e3f974ca8989a73bac4e2c72ff3a6a4a71f708f94a6e35a4a5b789c |
| artifacts/goal1-small-v2-train/final | BUDGET_REACHED | 0 | 5000 | 5000 | 0 | 4579449 | 183311 | 12539635413911726257 | 335da0ec201ce8d0910a95e408fe2020ff3ceb535b4ec1baaca8822f4f866291 |
| artifacts/goal1-small-v3-train/final | BUDGET_REACHED | 4500 | 9500 | 5000 | 4066040 | 9317067 | 364580 | 5378563212722728257 | a2deb1136c333ed22967ec356c3585d223ccec2742e713995afeb6859e38cf39 |
| artifacts/goal1-small-v4-train/final | BUDGET_REACHED | 9250 | 14250 | 5000 | 9053171 | 19412103 | 790543 | 11384108196141042809 | 14030f7ac0732324ec9a39e2a7fd7e9d97f8efbbfdc0484f1b232ed92f8aae6c |
| artifacts/goal1-small-v5-train/final | CANCELLED | 10500 | 13425 | 2925 | 11303424 | 17777824 | 793490 | 10935377324292083473 | 069db87393acb0e773c1fe5d5142b1e3dce01959b4f1c85f91732552248d9183 |
| artifacts/goal1-small-v6-train/final | CANCELLED | 12500 | 16553 | 4053 | 15722033 | 24838251 | 1308288 | 5079172076085679057 | a31a2978e82c4d0b4d5d8adf458334e69643c79de8d90c7de3927553779e83e4 |
| artifacts/goal1-small-v8-train/final | CANCELLED | 14500 | 19271 | 4771 | 20220437 | 31494447 | 2140718 | 14875340930758921089 | 611a277296d6e474a7fe841821ac3c04137686d0801655be5f87055b4d45efac |
| artifacts/goal1-small-v9-cue-train/final | BUDGET_REACHED | 19271 | 19371 | 100 | 31494447 | 31633012 | 2142318 | 4308879903089659169 | a7183bbd3dbf6978748687e7ba11b7fb62412a5d25581812e96d07f30526c0cd |
| artifacts/goal1-small-v9-mixed-train/final | BUDGET_REACHED | 19371 | 19871 | 500 | 31633012 | 32780250 | 2242995 | 6816806985872004417 | 1708fbee7152b5db658686f6ddd9a33bffcf8acea1ac45200d3ff91da6bdfe4a |
| artifacts/goal1-small-v9-mixed-train/step-019750 | TRAINING | 19371 | 19750 | 379 | 31633012 | 32501322 | 2218428 | 2077817959327737305 | 0d60afec7dd18a8a68c77e6da43b3c46076d6720609f00d6e2656a5ccf9ae2b9 |
| artifacts/goal1-small-v6-train/step-014500 | TRAINING | 12500 | 14500 | 2000 | 15722033 | 20220437 | 991264 | 12638071737532215433 | 166c58986c4f408da383ac7ea98f2b7d5996e0e20f1cfe9c164e36eb4210fc93 |
| artifacts/goal1-small-v2-init | RANDOM_INITIALIZED | 0 | 0 | 0 | 0 | 0 | 0 | 해당 없음 | d4e75cd8cbd8f3ff76a2c9fdc300561a3c65319e6c0210d4215275159fda2cc3 |
| run 경로 | 상태 | 시작 step | 종료 step | updates | 시작 input | 종료 input | 종료 target | sampler u64 | weights SHA-256 |
|---|---|---:|---:|---:|---:|---:|---:|---:|---|
| artifacts/goal1-small-init | RANDOM_INITIALIZED | 0 | 0 | 0 | 0 | 0 | 0 | 해당 없음 | 6888e5990dfa93504bf250ba169f4a95361555f4b20d7d81a55493bcd4dd9e21 |
| artifacts/goal1-small-v2-train/step-004500 | TRAINING | 0 | 4500 | 4500 | 0 | 4066040 | 162751 | 11285671872520553633 | a9accb03c9f97fe78ca71ab4805527037b7096ff1e1c1eaf2fadf68b5d138802 |
| artifacts/goal1-small-v3-train/step-009250 | TRAINING | 4500 | 9250 | 4750 | 4066040 | 9053171 | 354468 | 4751581442027141945 | 97e91910cb8ea9dcc465b4389b2e1c4f311e37882a6fdd19a6aafe8c528a7d94 |
| artifacts/goal1-small-v4-train/step-010500 | TRAINING | 9250 | 10500 | 1250 | 9053171 | 11303424 | 437794 | 11021399148983005065 | 79c7073e3af095e23a5bedd52921dadb7c90db20304b785989c6d254da5e5d35 |
| artifacts/goal1-small-v5-train/step-012500 | TRAINING | 10500 | 12500 | 2000 | 11303424 | 15722033 | 680823 | 2606363406402834441 | b3a455f15c23efabe1bc3614500efef6024d260d060961a006aefa36fd6df2a3 |

중간 checkpoint의 TRAINING은 저장 당시 상태이며 현재 process 실행 표시가 아니다.
다음 parent는 보존된 run 시작 설정/선택 기록 및 위 manifest hash 대조를 함께 확인했다.
작업 중 파일 이름으로 parent를 자동 추정하거나 서로 다른 branch의 step을 합하지 않는다.

| run_id | parent_checkpoint_hash | 시작 target | 추가 input | 추가 target | 종료 이유 |
|---|---|---:|---:|---:|---|
| v1 | 6888e5990dfa93504bf250ba169f4a95361555f4b20d7d81a55493bcd4dd9e21 | 0 | 1622873 | 56407 | BUDGET_REACHED |
| v2 | d4e75cd8cbd8f3ff76a2c9fdc300561a3c65319e6c0210d4215275159fda2cc3 | 0 | 4579449 | 183311 | BUDGET_REACHED |
| v3 | a9accb03c9f97fe78ca71ab4805527037b7096ff1e1c1eaf2fadf68b5d138802 | 162751 | 5251027 | 201829 | BUDGET_REACHED |
| v4 | 97e91910cb8ea9dcc465b4389b2e1c4f311e37882a6fdd19a6aafe8c528a7d94 | 354468 | 10358932 | 436075 | BUDGET_REACHED |
| v5 | 79c7073e3af095e23a5bedd52921dadb7c90db20304b785989c6d254da5e5d35 | 437794 | 6474400 | 355696 | CANCELLED; 검증 악화 후 중단 기록 |
| v6 | b3a455f15c23efabe1bc3614500efef6024d260d060961a006aefa36fd6df2a3 | 680823 | 9116218 | 627465 | CANCELLED; 검증 중단 기록 |
| v8 | 166c58986c4f408da383ac7ea98f2b7d5996e0e20f1cfe9c164e36eb4210fc93 | 991264 | 11274010 | 1149454 | CANCELLED; 사용자 중단 기록 |
| v9-cue | 611a277296d6e474a7fe841821ac3c04137686d0801655be5f87055b4d45efac | 2140718 | 138565 | 1600 | BUDGET_REACHED |
| v9-mixed | a7183bbd3dbf6978748687e7ba11b7fb62412a5d25581812e96d07f30526c0cd | 2142318 | 1147238 | 100677 | BUDGET_REACHED |
| v10 | 0d60afec7dd18a8a68c77e6da43b3c46076d6720609f00d6e2656a5ccf9ae2b9 | 2218428 | 714533 | 14322 | BUDGET_REACHED |
| v11 | 307864aaeb32d2997e09c5f4fee508233058933fbf3b8f958de1754c76706e92 | 2232750 | 682413 | 14240 | BUDGET_REACHED |
| v12 | 4478c75414aae9da3995dd2605b5e7daecf39a5c8a0782f7beaa3b073ad321d8 | 2246990 | 1367354 | 28624 | BUDGET_REACHED |
| QA32 | d4e75cd8cbd8f3ff76a2c9fdc300561a3c65319e6c0210d4215275159fda2cc3 | 0 | 1217005 | 106609 | CANCELLED; 기억32/32 후 중단 기록 |

원본 run별 실제 timestamp와 별도 probe/main 시간은 원래 로컬 로그에 남아 있다.
모든 row는 위 artifact의 현재 SHA 검증을 포함하지만 과거 학습 전 과정을 재실행한 것은 아니다.

## P0 검증과 기여

최초 직접 회귀 build는 tests/runtime.rs의 Ordering 경로 누락으로 실패했다.
해당 참조를 완전 경로로 수정 후 native7, training15, runtime6, retrieval6, cli4,
합계38개가 통과했다. trainer unit7개도 통과했다. all-target clippy에서 기존 grouped
sampler 테스트의 chunks_exact 고정 크기 lint를 발견하여 Rust 1.98 as_chunks로 바꿨다.
최초 실패 로그를 보존하며 재실행은 새로운 테스트 수로 중복 계산하지 않는다.

이번 신규 소스는 examples/validate.rs의 storage-audit, checkpoint-load-audit,
lineage-audit와 합성 DB measure 계상 보완, 위 두 테스트 호환 수정이다.
함께 전송하는 이전 S4 WIP는 Cargo 설정, native CLI/worker, corpus/학습/평가,
그 직접 회귀이며 6,034행 추가/542행 삭제의 기존 기여로 구분한다. 감사 명령이 이
WIP의 metadata/TrainConfig를 사용하므로 실제 빌드 가능한 선행 상태를 함께 보존한다.
큰 모델/자료/원문 로그는 전송에서 제외한다. 이 commit은 S4/Goal1 품질 합격이 아니다.

P0 소스 목록 digest: `e36fd1992b984e448750d18cafe052f54ec59b7418a0903477fe48acac05fa99`.
대상은 Cargo.toml/Cargo.lock/rust-toolchain.toml, src, tests, examples, migrations의 Rust/SQL
파일을 경로순으로 SHA-256 계상한 목록이다. 문서 자체를 포함하는 순환 hash는 만들지 않는다.
수정 후 all-target clippy와 fmt가 통과했고 grouped sampler 직접 회귀도 통과했다.
DELIVERABLE_VERIFIED(P0)=YES, MODEL_QUALITY_PASS=NO, GOAL1_READY=NO.

P0 raw 증거 SHA-256:

| 증거 | SHA-256 |
|---|---|
| target-filtered dependency metadata | 51959b60e4bd642fc825a5ef68cbebf2ab35425e3224c700316a7681123424b9 |
| dependency feature tree | 0e53b19f8a533da96681289397767ce927b4aa86951a32b39316f748a34f5b79 |
| tensor inventory와 read/hash/load/save 측정 | 7817f1d885a5c53a0d875eb290438ba1ee68f097bfa1004156615f218ee19978 |
| synthetic SQLite 측정 | f77d453e23ec852bf99463ccd061aacc5dee73431134dc7d1fa8b003913d2cd5 |
| 감사 harness examples/validate.rs | 0b8440ebea89cf86544c9d96facfd56eea72b7075a76c24c5cc858116b4cfbbf |

## P1 실행 전 동결

새 파일 src/contrast.rs는 training-only 진단 책임을 기존 1,400행 trainer에서 분리한다.
제품 library/worker는 이 모듈이나 gold validator를 import하지 않는다. 기존 Sample,
batch, target loss, Adam, Transformer, tokenizer, checkpoint를 재사용한다.

원본16은 v12 train 앞400개 중 copy/value인 4개 quartet이다. base ID는23/41/59/77이다.
원본 generation log와 question/evidence/answer를 대조했고 별도 serialized-input
validator가 target을 유일하게 재구성했다. source/binding/gold를 runtime 모델에 추가하지 않는다.
기존 원문, 질문, 순서, ID, 시간/status와 token IDs/role span을 로컬 freeze 로그에 보존했다.

새64는 학습 전에 seed917031로 만든 별도16그룹이다. 전체 원본 corpus의 ID 상한 밖
새 사건 ID, 새로운 대상 번호/장소/값과 그룹별 고정 순서 반전을 사용했다. 새 값은
`경로<정수>` 형식으로 기존 방향 단어에 비해 복사와 새 token 조합 부담도 달라진다.
따라서 성공하더라도 Goal1 heldout가 아니며 별도 전이 조건으로 보고한다. 아직 평가하지 않았다.

| 동결 대상 | SHA-256 |
|---|---|
| train16 직렬화 | 25247ee85541def3b5efedb391fcf922cbade6574a538f2f8b7bc885f7c69d3d |
| heldout64 직렬화 | 9bfd298901814b2491721225da803c2e98597b58b6247062448a90b8a02c9a0e |
| 전체 freeze 파일 | ad0e51190eb976ed1766008a808b33159dc6a0ead6f099f748f7576facfa4ba8 |
| 기존 원본 generation log | fa9a43c7da60ebc6aa6061c75b5f6337fc0555d95bcb1ae90dec770301ad5071 |
| P1 Rust/Cargo source 목록 | 07ed0ef5290c6201bd28c6f12a477748daf173dd963ce9f64351f0190da94578 |
| P1 실행 binary | 73c2a44ce4e89836816196c5a2853876ba87836478dcacabb37851f017a2e9b6 |

사전 허용 logit 절대 오차는 5e-4다. 실제 SMALL seed17와 일반 QA19750에서 train/generate
prompt ID, 독립 role/shift/mask/분모, 단건/4행 batch, 127/128/257 chunk cache와 uncached
전체 greedy, 255/256/257의 독립 causal/local mask 및 cache parity, 모든68 parameter
gradient의 finite/nonzero를 확인했다. 출력 projection gradient는 tied embedding에 합쳐진다.
random preflight 9.30s/최대 RSS955,383,808 B, QA preflight2.68s/1,010,466,816 B.
이는 학습 성공이나 단일 오류 원인 확정을 뜻하지 않는다.

R-A는 seed17 SMALL을 실제 재초기화하고 기존 random artifact의 model digest와 대조한다.
R-B는 마지막 일반 QA 선택 checkpoint19750이며 v12 auxiliary-only는 제외한다.
두 run 모두 Adam m/v를 새로0으로 초기화한다. step/input/target도 새 run 기준0이며
부모 checkpoint SHA를 TrainingState에 별도로 저장한다. 일반 랜덤 sampler로 이 진단
checkpoint를 재개하려 하면 명시 거부한다.

동일 설정: F32 CPU/Accelerate, LR.001, warmup20, W1, seq512, microbatch4×accumulation4,
group4, seed17. 각 update에 모든16개를 정확히1회씩 노출하고 그룹 순서만 shuffle한다.
각 microbatch mean gradient에 target 수를 곱해 합산한 뒤 전체 target 수로 나눈다.
종전 v12의 micro8×1, with-replacement group sampler, LR.0003/warm100/W8과 다르다.
run별 최대1,000 updates/3,200,000 input/45분이며 더 이른 상한에서 정상 final을 저장한다.
평가는0,100,... 전체16개, 마지막 두 평가16/16·4/4와 fresh reload를 통과해야 진단 합격이다.
답변/value/EOS와 값으로 추정한 record ID를 분리한다. 원본은 citation을 요구하지 않아
citation 점수는 NOT_REQUESTED이며 추정 record를 실제 출력 citation으로 바꾸어 보고하지 않는다.
source는 실제 실행 동안 편집하지 않고 모델/자료/로그는 로컬에만 보존한다.

## P1 실제 실행 결과

두 run 모두 `VECLIB_MAXIMUM_THREADS=1 RAYON_NUM_THREADS=1`로 실행했다.
학습 전후 Rust/Cargo source 목록은 byte-identical이었다. 원본 initial/QA/v12 checkpoint
hash도 보존했다. heavy 작업을 병렬 실행하지 않았고 새로운 학습 source를 중간 교체하지 않았다.
각 update의 실제 input은2,690, target은EOS 포함32다. prompt는162~172 tokens였다.
따라서 이 자료에서 짧은 답변 길이만 보고 context를 추론하지 않았으며 실제 prompt도 계상했다.

| optimizer update | R-A 전체 EM / 그룹 | R-B 전체 EM / 그룹 |
|---:|---|---|
| 0 | 0/16, 0/4 | 0/16, 0/4 |
| 100 | 10/16, 1/4 | 16/16, 4/4 |
| 200 | 10/16, 1/4 | 16/16, 4/4 |
| 300 | 12/16, 2/4 | 정상 종료 후 미실행 |
| 400 | 13/16, 3/4 | 미실행 |
| 500 | 16/16, 4/4 | 미실행 |
| 600 | 16/16, 4/4 | 미실행 |
| 별도 새 프로세스 final 복원 | 16/16, 4/4 | 16/16, 4/4 |

| run_id | parent checkpoint SHA | updates | input / target | 최종 sampler u64 | 종료 이유 |
|---|---|---:|---|---|---|
| R-A | d4e75cd8cbd8f3ff76a2c9fdc300561a3c65319e6c0210d4215275159fda2cc3 | 600 | 1,614,000 / 19,200 | 8507264816735876025 | TWO_EVALUATIONS_PASS_PENDING_FRESH_RELOAD; 이후 복원 통과 |
| R-B | 0d60afec7dd18a8a68c77e6da43b3c46076d6720609f00d6e2656a5ccf9ae2b9 | 200 | 538,000 / 6,400 | 15133584321384993097 | TWO_EVALUATIONS_PASS_PENDING_FRESH_RELOAD; 이후 복원 통과 |

각 사례 노출은 R-A600회/R-B200회로 정확히 동일하다. optimizer moment를 부모에서
가져오지 않았다. 실제 wall time은449.85s/150.29s, 최대 RSS는1,135,984,640 B /
1,112,670,208 B였다. 모두 예산 안에 종료했고 checkpoint의 DIAGNOSTIC_COMPLETE는
일반 모델 품질 합격을 뜻하지 않는다.

최종 두 평가와 fresh reload에서 전체답/value/값으로 추정한 record는 각각16/16,
EOS도16/16이다. 정답 형식 자체가 값 하나이므로 value exact와 전체 string exact는
같고 전체 정합에는 EOS를 추가 요구한다. citation 요구는 없으며 출력 citation 점수를
임의로 만들지 않았다. teacher-forced와 실제 greedy prefix의 첫 분기 logit/probability는
각 raw 평가 row에 별도로 보존했다. argmax만으로 입력을 무시한다고 판정하지 않는다.

사전 동결64는 R-B final에서 한 번만 실행했다. 전체답/value/추정 record0/64,
완전한 그룹0/16, EOS64/64였다. 실제 wall3.30s, 최대 RSS538,689,536 B.
새 문자열 값을 복사하지 못하고 훈련된 방향 단어를 출력했다. 새 값·대상·장소·순서를
함께 바꾼 전이 조건이므로 단일 실패 원인을 확정하지 않는다. 이 결과로 추가 tuning,
자료 확장, LR sweep, R-C를 실행하지 않았다.

이전 실패 v12 가중치에도 추가 no-training check를 실행했고 통과했다(2.69s,
RSS1,002,979,328 B; 257-token cache 오차1.1444092e-5). 이는 기존 FAIL을 지우지 않는다.
검사한 범위에서 shifting/masking/cache/gradient의 특정 구현 오류는 관측하지 않았다.
16개 암기 가능성과 새로운 사실에 대한 일반화는 별개이며, 큰 QA 실패의 원인 전체가
해결됐다는 결론은 내리지 않는다.

최종 snapshot SHA-256:

| 경로 | weights SHA-256 |
|---|---|
| artifacts/customize-20260917/r-a/final | 0cfcbcdd0740411fc55a150837be8b75147428d12460edcef968da73bed0bec6 |
| artifacts/customize-20260917/r-b/final | f50a002154eb746eaea7eda2fd69addf9b03143aa213f0bd8ab55a7c01caf680 |

검증 명령은 동결 executable `artifacts/customize-20260917/contrast-trainer`의
`contrast train --fixture .../contrast-frozen.json --checkpoint <원본> --output <신규>
--source-id 07ed0ef5290c6201bd28c6f12a477748daf173dd963ce9f64351f0190da94578
--start random|qa`, 별도 `contrast evaluate --fixture ... --checkpoint <각 final>` 및
R-B의 단 한 번 `contrast evaluate ... --heldout`이다. 이 executable과 corpus/로그는
로컬 증거이며 원격에는 Rust 구현과 익명 통계만 올린다.

| 로컬 raw 증거 | SHA-256 |
|---|---|
| r-a-train.txt | 9d63af70a55ee2b355bd06a9b56f51914f3397a9232c1bbc6770855e7fce4ded |
| r-a-reload.txt | c48bc43dd3c17976fa44f24774a14b9c362f975d0dc35bd189a04052e55150b6 |
| r-b-train.txt | f439a9c8e05f6e91e884d63b904849ffbf0e252c3db1e93932c8f215ba23cc80 |
| r-b-reload.txt | b040616826503fdee4799123082e94e2aed81feacc1ca7210630749397af3839 |
| r-b-heldout-once.txt | b42b975648d4d661c9fbd567bdff09b33c122abe92f6d2bf17bb8b2efe3fb272 |

DELIVERABLE_VERIFIED(P1)=YES. CONTRAST16_MEMORIZATION=PASS.
MODEL_QUALITY_PASS=NO. S4_QUALITY=FAIL. GOAL1_READY=NO.

## P2/P4 수식 경계와 단일 커널 실험

SMALL의 RMSNorm, QK norm, RoPE, causal local/global GQA, SwiGLU, tied embedding을
그대로 유지했다. `gqa_attention`은 기존 수식의 호출 경계이며 `Kernel`은 실제 linear
실행을 선택한다. `OperatorSpec`에 수식/parameter/state/numeric 버전을 명시했다.
Capabilities의 candidate training/backward/prefill은 false이며, 모델의 해당 경로는
명시적으로 기존 미분 가능한 reference를 호출한다. inference 후보의 Vec 변환을
학습 graph에 삽입하지 않는다. Candle Tensor와 autograd 전체는 여전히 의존한다.

기존 `Config.id()`/tokenizer JSON hash/`weight_hash()`는 legacy 이력 비교를 위해
보존했다. 별도 architecture semantic ID는 수식과 수치 config를 canonical bytes로
해시하며 profile 이름·JSON 공백과 무관하다. weights content ID는 이름·shape·F32
값을 해시한다. tokenizer semantic ID는 특수/ASCII segmentation 버전, byte mapping,
ordered merge rank를 해시한다. 기존 tokenizer 학습이나 token ID 변경은 없다.
BPE 구성은 direct builder로 옮겼으며 JSON 모델을 다시 구성하지 않는다.

cache는 architecture/weights/tokenizer/state schema/scope에 결합하고 실제 입력
u32 LE 이력을 해시한다. reset은 이력도 초기화한다. 동등 kernel ID는 cache 의미에
넣지 않아 같은 상태로 reference/candidate를 비교할 수 있다. cache는 원래 디스크에
직렬화하지 않았으며 프로세스 내 이전 cache를 새 구현에 이식하지 않는다.
명시 experimental profile은 SMALL 상한 이내 숫자 검사를 통과해야 한다.
SMALL/TINY의 이름으로 다른 config를 허용하지 않고, 이번 학습 구조도 바꾸지 않았다.

호환성: (A) 같은 수식 kernel은 tolerance/cache parity를 만족하면 가중치 재사용 가능.
학습 후보라면 gradient parity도 필요하다. 이번 후보는 inference-only이고 training
fallback의 출력·gradient가 bitwise 동일함을 검사했다. (B) 같은 shape여도 다른 수식은
새 equation ID, cache 무효화, 재평가/필요시 재학습이 필요하다. (C) SSM/recurrent
state로 자동 weight/cache 변환하지 않는다. (D) evidence 원문/버전/관계는 모델 ID와
독립이며 파생 embedding/index만 모델 버전별 재구성 대상이다.

실측 source는 R-B final weights
`f50a002154eb746eaea7eda2fd69addf9b03143aa213f0bd8ab55a7c01caf680`,
169-token 고정 입력 digest는
`2fa057cc01ce08261f7791771a7a1e9f2c8fa127bb8e305b14dffa35fc9c7724`.
CPU/gemm F32, VECLIB_MAXIMUM_THREADS=1/RAYON_NUM_THREADS=1, M4에서 측정했다.
profile warmup3/n15: prefill total 중앙값44.4315ms, linear33.5910ms;
decode total2.1656ms, linear1.2093ms. timer/shape 관측 overhead는 total에 포함되고
linear 구간에는 포함되지 않는다. 이후 성능 비교에는 관측기를 사용하지 않았다.

실제 linear는 매 forward43회. B=1, prefill T=M=169, decode T=M=1이며
(N,K,호출수)는 (1024,384,12), (384,1024,6), (384,384,12), (801,384,1),
(96,384,12)이다. input stride는 [T*K,K,1], weight stride [K,1]. reference는
flatten과 weight transpose view를 사용하며 backend 내부 scratch 할당량은 UNKNOWN이다.
GQA의 repeat_kv는 물리적인 head 복사, local mask는 dense QK 계산, KV cat/evict는
복사, RoPE sin/cos는 매 호출 생성임을 소스에서 확인했지만 추가 후보는 구현하지 않았다.

후보는 Rust CPU F32 contiguous M=1 GEMV 하나다. weight cache 없이 매 linear마다
weight N*K, input K를 Vec로 복사하고 output N을 할당한다. 명시 user-space buffer
바이트는 (N*K+K+N)*4이며 backend/allocator의 숨은 비용까지 0이라고 주장하지 않는다.
0 값, 독립 손계산, random/cancellation, 홀수/tail, 잘못된 dtype/shape/비연속 및
빈 차원 거부를 검사했다. dot 오차는 f64 독립 기준과
2*K*F32epsilon*sum(abs(products))+1e-6의 사전 상한을 사용했다.
모델 logit 허용치는 사전 5e-4이며 실제 SMALL decode 최대차3.8146973e-6이었다.
TINY local eviction cache와 reference 학습 gradient 비교도 통과했다.

| warmup3/n31 중앙값 ms | reference | Rust GEMV |
|---|---:|---:|
| 단일 gate, M1/N1024/K384 | 0.0468 | 0.1540 |
| 전체 decode | 2.1729 | 4.7344 |
| 전체 prefill | 44.7017 | 44.8708 |
| 실제 greedy 생성, chunk128 prefill 포함 | 50.0514 | 53.0012 |

생성 token/EOS 결과는 동일했다. `KERNEL_ADOPTION=KEEP_REFERENCE_OR_REJECT`.
마이크로 수치만으로 채택하지 않았으며 속도 악화로 기본값을 유지한다. 이 실험은
품질 검증이나 S4 승격이 아니다. 로컬 원본 결과는 customize-20260917의
p2-kernel-profile/compare, p2-native-tests, p2-resume, p2-contrast16-regression에 보존한다.

P2/P4 파일 SHA-256:
- neural.rs: `8b9ba9b472e55db3c342b7c1af4d11384cee247d452995e05e148c3f8a63c587`
- transformer.rs: `6218161c869c442ee9003f3cb7e121e2053f1a6bf3ec18d2d378b487659e0395`
- native tests: `251aaac8f620196ae8f80cd0fafacf35583240ab074de09904666e08d5f970af`
- validate: `23b600d2eea572c5dc3c19aff186f21d0fea627fdf5fb10be36a7bf910d82397`
- profile 결과: `8ca5179a331c72a29cfde4951c43ef792d4afeb04398f9b95f9819174e712fca`
- candidate 결과: `aa7d7d1b711246c930bdafd32edaa455b28e5c9783fb36a95899548b4ae50e94`
- 기본 contrast16 재생성: `149ecca544eb34afaa72ff87c25b1d52e8eff674d67bd2fc7a2b55b8908faaad`

실행 명령은 `validate kernel-profile|kernel-compare CHECKPOINT FIXTURE`,
`replica-train contrast evaluate --fixture FIXTURE --checkpoint CHECKPOINT`,
`cargo test --offline --locked --test native`,
`cargo test --offline --locked --test training native_training_resume_is_identical_in_fresh_processes`,
`cargo fmt --check`, `cargo clippy --offline --locked --all-targets -- -D warnings`이다.
직접 회귀10개, 기존 contrast16 16/16·그룹4/4 및 fmt/clippy가 통과했다.
새64는 다시 실행하지 않았다. 최초 계측 example의 잘못된 필드명으로 발생한 compile
실패는 수정했으며 최종 build는 통과했다. 이 실패 로그도 로컬에 보존한다.

고정 P1 실행 파일과 새 실행 파일을 같은 단일 thread 설정으로 각각 별도 실행해
contrast16을 대조했다. 생성 token IDs/문자열/EOS와 prompt 길이는 모두 같다.
branch logit gap은 최대 5.7220459e-6 차이가 관측돼 사전 5e-4 이내이며, 재빌드 전후
logit의 bitwise 동일성으로 보고하지 않는다. 같은 새 backend에서 uninterrupted/split
resume는 기존 회귀의 weight file hash·sampler·token budget·loss exact 비교를 통과했다.

## P3 native binary 저장·실행 검증

기본 `checkpoint::save/load/metadata`, trainer의 start/step/final, product worker가
단일 R3MODEL v1 파일을 사용한다. inference와 resume은 artifact kind 및 CLI로 구분한다.
기존 JSON+safetensors 디렉터리는 명시 legacy importer/audit만 읽는다. 원본 파일과
운영 DB는 교체/삭제하지 않았다. 모델 wire 책임만 neural/artifact.rs에 분리하고
기존 integer/bytes codec, BPE builder, Transformer/Adam/state를 재사용했다.

가중치 math/config/token ID와 source quality는 보존한다. 별도 `trained_steps`와
`diagnostic_only`, source/initial/parent/legacy migration identity가 inference에도 남는다.
변환으로 승인되지 않은 legacy quality를 승격하지 않으므로 이번 모든 변환은 diagnostic이다.
모델 runtime revision은 model content digest와 tokenizer/architecture semantic ID이다.
기존 `Manifest.weights_sha256` 필드는 legacy에서 safetensors file hash였고, native에서는
정렬된 tensor별 이름+payload digest의 집합 hash다. 실제 container file SHA는 아래에
별도로 보고한다. 추론의 model content digest는 optimizer 내용과 독립이다.

source는 v12 final step21,750, weights file SHA
`aed892878e3f974ca8989a73bac4e2c72ff3a6a4a71f708f94a6e35a4a5b789c`로
변환 후에도 원본 hash 일치를 확인했다. 새 artifacts는 모두 로컬만 보존한다.
- inference file SHA: `b642e96d5a23945af172ac79f782192c18190eb50e0be1546b141eef570ac6c8`
- resume file SHA: `d3c4646474f4417561e0f90a5d1e56d4c29a445b7a47716a0e2ae99caefaffde`
- model content ID: `a13c3ed54c964b889a67c446036bb2acb4d1210e0605cb06a327bc1a3de653b9`
- architecture semantic ID: `41e9889d768173eda5373878e87a3d83a35a541e6769b969d12d234d37cfa3ab`
- tokenizer semantic ID: `652718e4864c2f06af3a2c67dac0a5174d5e2feb1387667173902da9a8a295d4`

| 새 프로세스 단일 실측 | 내부 측정 ms | 실제 파일 read B | 최대 RSS B |
|---|---:|---:|---:|
| inference load | 307.014 | 38,432,768 | 85,966,848 |
| resume의 inference view | 270.086 | 38,443,776 | 83,165,184 |
| full resume load | 408.609 | 115,285,248 | 160,464,896 |

헤더/정렬 padding을 포함한 read_exact 요청 byte를 세었다. inference view 두 경우
모두 Adam payload read/allocation은0이고 model68개만 할당한다. 실제 allocator 내부
총 할당 횟수/숨은 backend scratch는 별도 계측하지 않았으며 RSS로 대신 표시했다.
OS cache를 flush하지 않았으므로 cold-process이지 physical cold-disk 측정이 아니다.
P0 legacy inference는115,280,520 B 전체 tensor file을 읽고 RSS203,505,664 B였다.

명시 legacy import의 load+export 총 내부 시간은 inference728.596ms/RSS208,748,544 B,
resume1108.679ms/RSS359,972,864 B였다. native resume에서 inference를 내보내는 별도
실행은 source load286.849ms, 작성·전체 readback 검증·sync·공개276.508ms,
전체 프로세스 최대 RSS91,652,096 B였다. 이 출력은 legacy에서 직접 변환한 inference와
파일 bytes가 완전히 같았다. write 단계의 검증 메모리와 loader 메모리를 섞지 않는다.

모든 모델68 tensor의 name/shape/F32 to_bits, Adam136 tensor 및 TrainingState/TrainConfig
전 필드를 원본과 직접 비교해 일치했다. u64 sampler 정밀도를 유지한다. tokenizer의
mapping/ordered ranks, all256 bytes 및 한글/조합문자/숫자/control-spelling token IDs와
raw-byte roundtrip도 일치했다. native tokenizer section6324 B는 기존 JSON22483 B보다
작으며, 전체 inference overhead12032 B 중 일부이다.

별도 프로세스의 같은 CPU/gemm F32/단일 thread reference에서 다음 개발 입력을
legacy와 native로 각각 실행했다. prompt digest, 전체 마지막-position logits,
generated token IDs/text/finish를 기록한 JSONL 파일이 cmp로 완전히 같았다.
| 상대 회귀 입력 | 수 | 양쪽 결과 SHA-256 |
|---|---:|---|
| 기존 contrast16 train | 16 | 69d3869077bc25a0216203f82c6885aadfcb7676470afeb31e338bf3d41d7d4f |
| 기존 QA32 train 입력 | 32 | 736c2d5a1f86c85523bd68249f24ed6bba8c8931809230a9afab0c3226ec328f |
| 기존 일반 QA development | 336 | 07942b62849e4df8d304c50fb655bdb32a7b6fd21b0fed4fd2967c6ff3891da2 |

세 행 모두 같은 v12 source를 사용한 RELATIVE_REGRESSION_ONLY이다. QA32 전용 모델의
32/32 memorization이나 R-B의16/16을 v12 품질로 옮겨 계산하지 않는다. 새64는 재실행하지
않았다. 일반 QA 대조 시작 시 기준 프로세스 종료 확인을 잘못 처리해 비교 프로세스가
잠시 겹쳤다. 비교 프로세스 PID89773만 즉시 종료했고 부분 로그는 aborted-overlap으로
보존했다. 기준의336개 완료를 확인한 뒤 비교336개를 단독으로 다시 실행했다. 해당
실행의 시간을 성능 결과로 사용하지 않는다.

macOS sandbox-exec에서 JSON/JSONL/safetensors/DB/SQLite 읽기와 network를 deny했다.
실제 JSON, safetensors, 합성 SQLite file을 cat한 negative controls는 모두
Operation not permitted였다. 같은 제한에서 기본 `replica-v3 generate`가 native 파일을
읽고 child worker로 실제 생성했으며 지정한 absent DB는 생성되지 않았다.
'안녕하세요' 입력에 출력은 '장비631장비', EOS stop이었다. 이는 품질 FAIL의 실제
예시이며 저장 구현 성공과 분리한다. JSON IPC 출력은 모델 artifact가 아니다.

직접 테스트: 독립 complete literal container decode/hand encoder bytes, raw bit/state
roundtrip, 손상 header/kind/flags/version/dtype/duplicate/shape/offset/trailing/truncation/
length/merge/payload/nonfinite 거부, inference Adam skip/resume 손상 거부, no-clobber,
공개 전 실패 cleanup을 검사했다. 6-update 연속 vs3+3 fresh-process resume에서
weights/Adam/config/sampler/token budget/loss/logits가 exact 일치했다. curriculum 및
sample_group 전환 회귀, 취소 시 optimizer 경계 저장, 실제 child SIGKILL의 목적지
미공개·source 불변·정상 재시도도 통과했다. SIGKILL이 남긴 소유 temp는 fixture
디렉터리와 함께 정리하며 임의 temp/원본을 삭제하지 않는다.

실행: `model import-legacy --kind inference|resume`, `model export-inference`,
`validate native-load-audit`, `validate export-parity`,
`validate artifact-probe PATH legacy|native CASES LIMIT`, sandbox default generate,
`cargo test --lib neural::artifact`, `cargo test --test native`,
`cargo test --features test-support --test training native_`,
`cargo test --test training curriculum_resume_crosses_sampling_boundary_in_fresh_process`,
fmt/clippy/release build. 직접 테스트의 반복 실행을 새 테스트 수로 합산하지 않는다.
최초 literal fixture의 기대 token range/string length 오타와 clippy4건은 수정했으며
실패 로그도 로컬에 보존했다. F16/INT4/packed runtime은 NOT_IMPLEMENTED, S6 완료가 아니다.

## P5 동일 원문·관계의 읽기 전용 archive 검증

SOURCE_CHANGED: src/archive.rs(new), event/codec/store/retrieval/lib/main, examples/validate,
tests/codec/store/cli 및 관련 기존 문서. 기존 SQL projection/validation/BFS를 재사용하고
명시 취소·복원 head·DependsOn을 확장했다. schema1 bytes는 유지한다.
DELIVERABLE_VERIFIED=YES, MODEL_QUALITY_PASS=NO, LIVE_SQLITE_REPLACEMENT=NO.

fixture는 seed917055로 만든 정확히10,000사건이며 학습 corpus가 아니다. 초기654건에
정정/취소/복원/다른 context/무관 지시/실행/사고/명시 관계/cycle/chain/wide graph,
나머지9,346건에 긴 한국어10건, 반복2,334건, 난수ASCII2,333건, 기타4,669건을 담았다.
원문을 요약하거나 사건 ID를 재배정하지 않았다. SQL canonical body 자체는 기존 Auto
event 압축이고, raw archive라는 말은 그 body 바깥 block을 추가 압축하지 않는다는 뜻이다.

모든10,000개의 ID/body bytes/Event 객체와 전체329개 sparse edges를 양쪽에서 대조했다.
timeline/current9개, lexical/combined 지원6개, 방향·session·time 조합9개 및 cycle/
hop4/visited256/snapshot edge 제한과 truncated가 일치했다. 미지원 lexical4개는
오른쪽/원인 미확정/명시관계/반복 한국어이며 별도 NOT_COMPARABLE로 기록했다.
prefix 부분문자열과 FTS5 trigram/BM25를 같은 기능으로 부르지 않는다.
graph seed는 fixture에서 명시한 endpoint이고 신경모델 benchmark의 정답 사건 공급이 아니다.

| 저장 항목 | 실제 bytes |
|---|---:|
| 원문 payload 합 | 14,166,104 |
| 압축 해제 canonical event body 합 | 14,726,981 |
| source의 canonical envelope+encoded body 합 | 9,377,174 |
| raw archive block 합 (5개) | 9,377,174 |
| zstd archive block 합 (5개 모두 압축) | 8,388,952 |
| prefix / block directory / record index / dictionary | 144 / 260 / 200,000 / 0 |
| source+header+block SHA 필드 (위 크기에 포함) | 224 |
| raw archive 전체 | 9,577,578 |
| zstd archive 전체 | 8,589,356 |
| SQL main 열린/닫힌 상태 | 78,901,248 / 78,901,248 |
| SQL WAL 열린/닫힌 상태 | 80,459,512 / 0 |
| SQL SHM 열린/닫힌 상태 | 163,840 / 0 |
| 별도 검증 backup | 78,901,248 |
| FTS에 중복 저장된 정규화 prefix bytes | 13,169,934 |
| records pages | 9,646,080 |
| FTS content/data/docsize/idx/config pages | 14,540,800 / 51,359,744 / 102,400 / 233,472 / 4,096 |
| metadata / current_heads / relations pages | 643,072 / 98,304 / 12,288 |

dbstat 전체 표는 로컬 측정 로그에 있고 SQL page 항목은 main 크기에 이미 포함된다.
DB+WAL+SHM 열린 총159,524,600 B와 닫힌78,901,248 B를 구분한다.
archive의 훨씬 작은 크기는 FTS 및 운영 transaction/index 기능 차이도 포함한다.
동일 정보 보존은 확인했지만 같은 SQL 기능 전체를 같은 비용으로 제공한다는 뜻이 아니다.

| 작업 | SQLite | raw archive | zstd archive |
|---|---:|---:|---:|
| durable export+readback 검증 ms | 해당 없음 | 121.133 | 133.034 |
| 동일 process open / index rebuild ms | 해당 없음 | 54.653 / 53.965 | 54.843 / 54.157 |
| 새 process open / index rebuild ms | 529.266 / 기존 projection | 61.395 / 60.536 | 62.589 / 61.820 |
| 새 process 첫 get+graph ms | 0.347 | 3.845 | 4.018 |
| get p50 / p95 ms | 0.008083 / 0.014208 | 3.477875 / 3.526167 | 3.684792 / 3.798625 |
| graph p50 / p95 ms | 0.141875 / 0.152709 | 3.486125 / 3.586542 | 3.697750 / 3.813625 |
| current p50 / p95 ms | 0.008958 / 0.010125 | 0.000459 / 0.000584 | 0.000500 / 0.000708 |
| 새 process peak RSS bytes | 7,258,112 | 14,974,976 | 17,858,560 |

warm 각 항목3회 warmup+101표본, 같은 조회 순서/get IDs/graph scope와 seed/current slot.
각 루프는 get→graph→current라 한 block 캐시가 교체되는 부하도 포함한다.
archive는 block read/checksum/해제 때문에 get/graph가 SQL보다 느리고 RSS도 더 컸다.
OS cache는 비우지 않았으며 cold disk 측정이 아니다. SQL startup quick_check와 archive
전체 decode/index rebuild는 각 구현의 실제 production open 비용이지 같은 내부 작업이 아니다.
통합 생성/검증/backup 벤치마크 peak RSS224,706,560 B, wall11.52s는 reader 단독 RSS가 아니다.
backup의 pinned copy+전체검증5,187.738ms도 archive export와 기능이 다르다.

zstd는 raw보다988,222 B/10.32% 작고 get/graph p50 약6% 느렸다. 저장용 read-only
snapshot의 기본값은 zstd로 정했으며 raw 선택을 유지한다. 추가 block-size/codec sweep은
하지 않았다. dictionary/dedup/append e/s/concurrency/recovery는 NOT_IMPLEMENTED다.

새 CLI process에 DB/SQLite/JSON/safetensors 파일 read 및 network를 차단하고 archive show와
incoming graph를 실제 실행했다. 같은 sandbox의 source.db cat은 Operation not permitted.
24-byte 원문은 NUL까지 그대로 나왔고 지정한 p5-must-not-open.db는 생성되지 않았다.
통합 테스트에서는 별도 source DB를 숨긴 뒤 읽고, export 후 source append와 독립된
snapshot을 확인했다. 동시 writer hook은 export의 snapshot을 먼저 pin한 뒤 append하여
archive1/source2건을 검증했다. 운영 DB에는 쓰지 않았다.

EXECUTED_COMMANDS:
- cargo test --offline --locked --features test-support --test codec --test store --test retrieval --test cli: 22개 통과.
- cargo test --offline --locked --lib archive::tests: 3개 통과 (독립 literal, 손상/상한, zstd extra-frame/expansion).
- cargo build --offline --locked --release --bins --example validate.
- /usr/bin/time -l validate archive-measure artifacts/customize-20260917/p5-memory-bench-fixed.
- /usr/bin/time -l validate archive-read-probe PATH sqlite|archive: 각 저장 형태 별도 process.
- sandbox-exec로 기본 CLI archive show/graph와 DB 접근 실패 대조.
- fmt/all-target clippy, direct check/release: 완료 로그 기준.

실패도 보존했다. RV03은 처음 방향을 하나의 SQL OR parameter로 묶어 방문 예산 회귀가
났고 원래 양방향 endpoint 조건을 복원했다. 이후 deadline 내 반복 prepare 비용으로
249/256이 관측되어 동일 조건 prepare_cached로 수정했다. 한도/기대값을 낮추지 않았고
최종 기존 retrieval6개 모두 통과했다. 첫 benchmark는 지원되는 2자 단어 query를 미지원
목록에 잘못 넣어 중단됐고 새 경로에서 수정 재실행했다. 이전 파일을 삭제하지 않았다.
Rust1.98 clippy의 고정 chunks_exact 두 건도 as_chunks로 수정했다.

FILE_HASHES:
- source snapshot identity: 411fa074426045d38e7085b3a4ccedc70b3a3d160d6628f694f3b51157d0bb2a
- raw.r3a: 8b918ebdfc20a083d13f9d2cf9fe587bdba98d04f8eceb55d703009a226409a9
- zstd.r3a: d17fac53b8451cbd859bff4203d0c35ce7217f372dff08864c4b3670acdb4a90
- p5-memory-measure-fixed.txt: eec7d120951753576a171bd6a0391c3088f12169f1540c84c68f756c12fa1cd1
- archive-only 원문: f3dc049d050e387d40dc97efc59399cf9d7ef93cfb39c34dc9ef2ed981512b21

LIMITATIONS: 읽기 전용 snapshot, FTS5 미지원, 한 block 캐시, device power-loss 보장 아님.
원문/관계 보존 성공은 신경모델 기록 선택 및 새값 전이 실패를 해결한 결과가 아니다.
NEXT_DEPENDENCY: P6 최종 회귀·실제 생성·계약 대조·원격 일치 확인.
