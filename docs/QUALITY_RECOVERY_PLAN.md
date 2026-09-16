# R3-S4-QUALITY-RECOVERY-1.0

2026-09-17. 기준 HEAD `9fb5059696553b41d9a3190856888a929d0931be`, origin
`https://github.com/seoyd/replica-v3.git`, main. 시작 tracked dirty 없음.
Rust/Cargo1.98.0, macOS27 arm64, Apple M4/24GiB. CPU/Accelerate F32,
VECLIB_MAXIMUM_THREADS=1/RAYON_NUM_THREADS=1. 시작 시 프로젝트 학습/추론/빌드 없음.
기존 SMALL/801BPE, native artifact/SQLite format, 기본 decoding은 고정한다.
외부 모델/teacher/API, gold 기반 제품 선택, lossy decode, EOS 억제, 자료 확대는 금지한다.

## 순서와 종료 기준

Q0 실제 기준점/원본 동결 → Q1 무학습 replay/오류 보존 → Q2 데이터·prompt·수치 경계 →
결함이면 Q3A 최소 수정/causal replay, 아니면 Q3B 한 요인 제한 대조 → Q4 재검증/닫기.
독립 검토는 PENDING이며 이 라운드 종료는 Goal1 완료가 아니다.
품질 기준은 기존 독립200개/5분류40개씩/전체95%/각90%/잘못된 인용 승인0을 유지한다.
개발 근거가 부족하면 final은 NOT_RUN_NOT_ELIGIBLE. 제품 모델 포인터를 바꾸지 않는다.

명시 로컬 자료:

- parent 원본: artifacts/goal1-resume-20260917/u2-parent-resume.r3m
- 동일 가중치·U2 정책 시작 상태: artifacts/goal1-resume-20260917/u2-probe/start
- 실패: artifacts/goal1-resume-20260917/u2-to-20000/final
- +20: artifacts/goal1-resume-20260917/u2-probe/final
- parent corpus: artifacts/goal1-corpus-v9; U2: artifacts/goal1-resume-20260917/u2-corpus-fixed
- 기존400 실제 로그: 같은 디렉터리의 u2-start-validation.jsonl/u2-20000-validation.jsonl
- 새 산출물: artifacts/quality-recovery-20260917 (기존 파일 create_new, 덮어쓰기 없음)

원본 frozen 실행 파일은 baseline-replica-train으로 복사했다. 파일/hash/metadata는 실제
Rust native loader로 registry에 기록한다. physical/tensor/model-content/tokenizer wire/
semantic ID를 구분한다. 과거 기록은 REPOSITORY_REPORT, 이번 실제 읽기/실행과 구별한다.

## 실행 예산 (실행 전에 고정)

무학습0updates. Q1 watch32는 category별7/7/6/6/6, 각 category에서 base scene이
중복되지 않는 첫 항목을 metadata 순서로 선택한다. 실패 panel≤16은 기존 실패 로그에서
UTF8/빈응답/값·숫자/인용 오류 유형을 순회해 고정한다. 점수를 보고 panel을 교체하지 않는다.
기존400 로그를 검산하며 신뢰할 수 있으면 전체 replay 대신 panel만 실행한다.
각 generation은 원래 request의 고정 max_tokens/timeout/context를 사용한다.
무학습 CLI command 상한15분, 라운드 실제 실행 합산 상한90분, inference12GiB/training16GiB.
일반 검사 abs1e-4+rel1e-3, 사후 tolerance 확대 없음. nonfinite/data ambiguity/parity 실패는
학습 실험 시작 전에 중단하고 관련 경계만 수정한다.
실제 batch의 선택2좌표 central difference는 F32 차분 cancellation을 감안해 사전
step0.002, abs0.002+rel0.05로 고정한다. optimizer update 없이 원래 좌표 복원/hash 대조.

결함이 확정되지 않으면 근거 있는 최대3개 개입+control, 각≤50updates. 평가0/10/25/50,
watch32와 사전 고정 노출 표본만 사용한다. 1arm당 입력≤200,000/target≤50,000,
운영≤15분. 이는 기존250updates/약284s를 근거로 잡은 상한이며 완료시간 약속이 아니다.
watch에서 parent보다 정답4개 이상 감소 또는 새 UTF8/control/empty 오류2개 이상 상태가
연속 두 평가면 해당 arm 종료. nonfinite/시간/메모리/token cap은 즉시 boundary 종료.
방향·안전성 근거가 있을 때만 후보1개 confirmation2회, 사전 별도 tape, 각≤100updates/
입력400,000/target100,000/20분. 총≤400updates, 숨은 probe/수정 전후 update도 포함한다.
50updates에서 control 하락이 재현되지 않으면 자동 연장하지 않고 CLOSED_NEGATIVE.

## 가설 ledger (결과 관측 전)

| ID | 근거/예측 | 반증·최소 확인 | 허용 변경/현재 판정 |
|---|---|---|---|
| H-RECEIPT | evaluate의 decode 오류 가지가 Generated를 버림 | malformed byte fixture에서 IDs/종료 보존 검사 | 평가 receipt 보강; 품질 하락 원인과 별개 |
| H-DATA | answer로 support를 찾는 U2 생성 경로 | 질문/원문/status만으로 독립 검산, 동일 token prompt/다른 target 탐지 | 재현 결함만 수정; UNRESOLVED |
| H-PROMPT | train은 answer 길이, inference는 고정 예약 | 실제 train/generate IDs·제외 근거 대조 | 실제 불일치만 수정; UNRESOLVED |
| H-NUMERIC | 생성/cache/batch 경계 가능성 | 동일 prefix full/chunk/cache/batch/causal; 기존 독립 loss/Adam reference | 실제 위반만 수정; UNRESOLVED |
| H-LR | extension은 moments 유지, warmup/cosine 재시작 | 세 clock 및 실제LR/update trace, C 대비 L | Q2 후 arm 선택; UNRESOLVED |
| H-BATCH | 8개 같은 scene view 집중 | 같은 tape multiset, batch 경계만 교체 C/B | Q2 후 선택; UNRESOLVED |
| H-FIRST | 첫 target 가중치8 | 첫 token 분포/실제 objective, C/W | Q2 후 선택; UNRESOLVED |

D(기존 QA 혼합)/O(moment+optimizer clock 초기화)는 먼저 구현하지 않는다. Q2 근거가
위 후보보다 강할 때만 선택 사유를 고정한다. O는 clock 분리가 필요하며 기존 resume와
동일하다고 부르지 않는다. 이미 통과한 random QA32/contrast16 암기를 반복하지 않는다.

## 파일 경계와 검증

기존 training 평가/trainer/batch/Adam/native 모델을 재사용한다. 진단 orchestration은
training 전용 모듈1개로 격리한다. 기존 training.rs는 이미 평가·훈련·회귀를 포함하므로
registry/panel/audit를 제품 library에 넣거나 새 framework로 만들지 않는다.
새 영구 문서는 이 계획1개만 만들고 결과는 EXPERIMENT_STATUS.md에 기록한다.
실제 생성 오류 보존/ledger 분모/독립 support/packing/관련 numeric를 직접 검사한다.
최종 fmt/check/clippy/release 및 해당 회귀만 실행한다. 학습 실행 동안 소스를 동결한다.
공개 소스/테스트/익명 집계만 단계별 명시 commit/push, 원격 full SHA 대조.
원본 corpus/DB/model/Adam/raw log/임시 입력/target은 로컬 보존한다.

## Q0~Q2 관측과 Q3 선택 (학습 전)

원본 log336+64 집계와 artifact hash가 일치했다. 고정 watch32 parent18/32→실패5/32,
실패선정16 parent12/16→실패0/16; 각각 기존 출력 차이0, A→B→A 동일. UTF8 실패표본4개는
모두 invalid_sequence이며 length2/EOS2다. 같은 가중치를 쓰는 계측 수정으로 품질 회복은 없다.
초기 독립 validator가 인과 불확실성 답변의 단일 사건 인용과 상세 chronology 인용을 혼동해
U2 64/parent bounded120개를 표시했다. 원문/시각을 확인해 둘 다 뒷받침되는 응답 형식임을
분리하고 독립 합성 회귀를 추가했다. corpus를 고치거나 해당 항목을 제외하지 않았다.
U2 전2,048과 parent 앞2,048에서 token prompt 모순/prefix 불일치0; 부모의 보조340개는
의미 유일성 검사 밖이라고 별도 계상했다. U2의1,776prompt는256을 넘으므로 짧다는 추측을
하지 않는다. 실제 두 checkpoint의34개 수치 비교와 실제8행 loss/68gradient/2좌표 차분은 통과.

Q3는 C와 W 두 arm만 선택한다. 근거: 새 first-EOS12건과 학습 first-target8배 목적이
실제 존재한다. first token은 대상 종류 등 공통 문두이므로 기록 선택을 뜻하지 않는다.
W는 이 첫 토큰 가중치만8→1로 바꿔 빈 응답/후속 생성 하락과의 관련성을 반증 가능한
조건으로 확인한다. 일반 batch에서 가중 항의 영향이 작았으므로 큰 개선을 예상하지 않는다.
C/W 각각50updates, 동일 parent weights/Adam/optimizer clock/schedule/tape/자료/CPU/F32.
plan의 원래 cap을 그대로 적용한다. C20의 model hash가 원래 U2+20과 같은지도 확인한다.
추가 optimizer reset, LR/seed sweep, B/D 자료 재배열/확대는 실행하지 않는다.
L 미선택: parent LR0.0000865092, U2 +1/.000003→+50/.00015→+100/.0003의 재시작과
실제 log 일치는 확인했지만 아직 인과 증거가 아니다. 이번의 직접적인 first-EOS 경계부터
한 요인만 시험한다. LR 일정 continuation은 필요시 다음 한 개의 실험 후보로 남긴다.
첫 screening이 원인을 지지하지 않으면 자동 extension 없이 닫는다. 확인 실험은 아직
선택/실행하지 않았으며 총 업데이트 예산은 늘리지 않는다.

## Q4 종료 — 실제 결과 및 요구사항 대조

현재 NODE=Q4, STATUS=CLOSED_NEGATIVE / NOT_RUNNING. 제한 진단은 종료했고,
모델 회복의 원인은 UNRESOLVED다. 상세 수치와 판정은
[실험 상태](EXPERIMENT_STATUS.md)의 첫 보고에 기록한다.

| 경계 | 실제 구현/검증 | 판정 및 제한 |
|---|---|---|
| Q0 원본/계보 | 명시된 parent/start/+20/+250 native loader registry, physical/content/tokenizer/Adam/RNG/clock, source 및 binary hash | 기존 artifact 보존; 다른 run으로 대체하지 않음 |
| Q1 분모와 replay | 기존400 raw ledger 검산, 고정watch32 및 실패16, A→B→A, gold와 독립 생성 | 기존 출력 차이0; 실패선정16은 품질 추정 표본이 아님 |
| Q1 오류 receipt | 실제 generation loop observer, strict decode 실패에도 ID/bytes/EOS/finish 보존 | 독립 failing test 수정 전 실패/수정 후 통과; 학습 원인과 별개 |
| Q2 자료·입력 | U2전2048/parent앞2048, 질문/원문/status/time만으로 support, 역순·citation·값·prepared prefix·split 검사 | 확인 범위 PASS; parent 보조340 의미 검산 밖 |
| Q2 분포 | base scene/질문형/값 multiset/ID순서/token prompt 각각 집계 | U2는256 base scenes, 142숫자정규화 질문형, 58값 multiset, 1121 ID순서, 1894 token prompts |
| Q2 수치 | parent/failed 각각34개 full/cache/chunk/padding/causal 대조, 실제8행 shift/mask/EOS/68gradient 및2좌표 차분 | 사전 tolerance 내; 전체 연산의 무결성을 증명했다고 확대하지 않음 |
| Q2 optimizer | 독립 f64 Adam3step, 서로 다른 target 수 accumulation, 실제 parameter group norms | moment/clock 유지, schedule 재시작과 실제LR 일치; LR 인과성 미확정 |
| Q3 C/W | 같은 parent/tape/moments/LR, first-target 가중치만8→1, 각50updates | 모두watch18→17→16→17/32; C20은 과거+20과 model hash 정확히 일치 |
| Q4 재시작/제품 연결 | C/W native fresh load32개씩, worker4개 성공/오류 대조, legacy/native tokenizer byte mapping | raw IDs/문장/오류/prompt parity 통과; 제품 기본 모델 승격 없음 |
| Q4 후보/전이/최종 | 후보 적격 없음, confirmation/후보400/새전이/final200 미실행 | NOT_RUN_NOT_ELIGIBLE; S4/S5/S6/Goal1 합격 아님 |
| 종료 검증 | 직접 회귀13개, fmt/check/clippy/release, 원문·실패 로그 유지 | 독립 검토 PENDING; 공개 범위는 Rust 및 상태 문서 |

SMALL 실제 추가 업데이트는 C50+W50=100, 총input244,760/target22,600이다.
각각 같은19,750에서19,800으로 분기했으며19,850까지 연속 학습한 것이 아니다.
기존 exact-resume 회귀의 TINY12updates와 독립 scalar Adam3step을5회 실행한15회는
별도 테스트 계산으로 기록한다. 이를 합쳐도 총127updates로400 상한 이내다.
최초 C 시도는 native status vocabulary 검증에서0updates로 중단했다. 상세 종료 이유는
sidecar에 두고 기존 native status만 사용하도록 고쳤으며, 실패 디렉터리도 보존했다.
실제 C/W 프로세스74.63/75.50초, max RSS5.38/5.69GiB로 사전 상한 이내였다.

H-RECEIPT만 CONFIRMED_IMPLEMENTATION_DEFECT다. H-DATA/H-PROMPT/H-NUMERIC은
검사 범위 위반 없음, H-FIRST는 NOT_SUPPORTED_WITHIN_BUDGET이다.
H-LR/H-BATCH 및 학습 하락의 근본 원인은 UNRESOLVED다. C의50updates에서 원래
250update 붕괴가 재현되지 않아 REGRESSION_NOT_REPRODUCED_WITHIN_BUDGET으로 닫는다.
50updates 결과로250updates 현상을 반증하지 않는다. 확인 실험은 실행하지 않았다.

다음 한 개의 실험 제안은 C 대비 부모 마지막 LR을 유지하는 L이다. 기존 schedule의
warmup100을 실제로 지나는 범위를 별도 예산으로 먼저 고정해야 한다. 같은 parent,
moments/optimizer clock, tape/목적함수로 LR만 대조한다. 이는 다음 라운드 제안이며
이번 라운드에서 자동 연장하거나 실행하지 않는다.
