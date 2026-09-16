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
| P2 | NOT_STARTED | 없음 | NOT_RUN | 기존 reference 유지 | 해당 없음 | 새 연산 경계 미구현 | P0 |
| P3 | NOT_STARTED | 없음 | NOT_RUN | JSON+safetensors 현재 경로 | 해당 없음 | native binary 미구현 | P2 |
| P4 | NOT_STARTED | 없음 | NOT_RUN | 후보 미측정 | 해당 없음 | 기본값 승격 없음 | P2 |
| P5 | NOT_STARTED | 없음 | NOT_RUN | 운영 SQLite 유지 | 해당 없음 | archive 미구현 | P2 |
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
NATIVE_BINARY_INFERENCE: NOT_IMPLEMENTED
EXACT_RESUME_BINARY: NOT_IMPLEMENTED
JSON_FREE_DEFAULT_ARTIFACT: NO
LEGACY_JSON_USAGE: 모델/tokenizer manifest, safetensors header, config hash, IPC, corpus, logs
INFERENCE_BYTES: model raw 38,420,736 B; 독립 inference artifact 아직 없음
RESUME_BYTES: 기존 세 파일 115,316,831 B
TOKENIZER_META_BYTES: 기존 tokenizer JSON 22,483 B
KERNEL_REFERENCE: 기존 Candle/Accelerate F32
KERNEL_CANDIDATE: NOT_IMPLEMENTED
KERNEL_ADOPTION: KEEP_REFERENCE
DB_FREE_ARCHIVE_VERIFIED: NO
LIVE_SQLITE_REPLACEMENT: NO
CODE_REVIEW_STATUS: INDEPENDENT_PENDING
S4_QUALITY: FAIL
S5_INTEGRATION: 미완료
S6_QUANT: 미구현
GOAL1_READY: NO
COMMIT / REMOTE_SHA: P0 `ce48514ab5fc8e76a9552ce5fabe7ce1617ff4ac` / 동일 SHA 확인

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
