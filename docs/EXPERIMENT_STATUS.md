# 진단 및 구현 상태

## G2 — C50 저장 실패로 pair 중단, QUALITY_INCONCLUSIVE

2026-09-18 / EXECUTED_THIS_RUN. F512 부모의 anchor4/focus4 대6/2를 명시
R3ER kind6으로 등록했다. 기존 parent Adam/step/tokenizer/정상 greedy를 유지하고
LR actual1e-4를 사용했다. 원래 parent config의 lr 필드는3e-5이나 기존 F와 이번
runner는 별도로 검증한 constant LR을 Adam에 전달한다. 새 warmup/reset은 없다.
공통 anchor2048개는 원래 parent corpus의 ID와 전체 내용 hash로, focus512개는
F training provenance로 검증했다. 같은 두 master stream의 prefix와 실제 배치
4:4/6:2, input/target 분모를 직접 회귀로 검산했다. ordinary/CROSS/dev와 분리했다.

C50은 실제256 SMALL updates, absolute24054, 생성1168/teacher0 후 저장 실패했다.
terminal은 IntegrityFail/resume=false/complete=false/save_error를 보존한다.
부모 max_steps23798을 새 fork metadata에 그대로 사용한 구현 결함으로 native
validator가 저장을 거부했다. 이는 이번 runner의 결함이며 과거 학습 부진의 원인이라는
증거가 아니다. A75와 추가 SMALL 실행은 중단했다. 원본 F512·Adam·corpus는 불변이다.
새 weights/Adam endpoint는 저장되지 못했으므로 재개·fresh-process 검증 불가능하다.

| C50 +256 in-memory 관측 | full | entity | event | errors |
| --- | ---: | ---: | ---: | ---: |
| dev256 | 244 | 250 | 251 | 0 |
| CROSS512 | 464 | 503 | 498 | 0 |
| ordinary400 | 233 | 별도 raw | 별도 raw | 0 |

ordinary QA179/336,aux54/64,watch19/32. 원래 부모242/459/176과 비교한 부분 관측이며
재현 가능한 endpoint 또는512 사전 비교 결과가 아니다. RAW native binding은 MISSING.
H3/S4 품질 PASS 없음, seal NOT_OPENED, S5/S6 NOT_RUN, Goal1_READY/ACCEPTED=false.
MODEL_PAIR=QUALITY_INCONCLUSIVE, ANCHOR_PRESERVED=UNRESOLVED, 다음 비율/LR 가설은
이 중단 실험으로 선택하지 않는다. 먼저 저장 가능한 fork 경계가 필요하다. 새 학습0.

수정은 새 fork 예산 metadata를 등록/첫 작업 전에 endpoint까지 검사하고, pair의
저장 실패·손상·취소가 다른 arm에서도 중단을 유지하도록 했다. 기존 실패 실행은
재작성/재등록하지 않는다. exhausted-parent native writer 실패→budget 수정 후
writer/reader weights·Adam·state 일치 회귀 PASS(optimizer/generation0). 비율 회귀도
PASS. 학습 전 quick59 PASS, 이후 직접 회귀2 PASS와 clippy PASS. TINY 누적45,
scalar0. 새 binary TINY generation/teacher74/74; quick의 다른 legacy generation은
별도 전역 계측하지 않았으므로 전체 generation 총수로 오해하지 않는다.

실행 source는 G1 SHA0a13ac64b10ae189788fc999058403aaa1e947ad + 동결 diff,
evaluator digest c4d42ad8919c2e86682e03f4a9bb69045376e72f527651c1979ef6bc07e6a447,
binary af63cdd67dd92c6755e78c65f9627aa9048fd3a1f976e7582643677ba297cb2a.
원자료는 local `artifacts/native-storage-quality-20260918/`의 `g2-source.diff`,
`g2-source-binary.sha256`, `g2-C50.log`, `anchor-pair/{C50,A75}/inputs.r3er`,
`anchor-pair/C50/segment-00/`이다. 모델/원문/로그는 Git에 게시하지 않는다.
실제 command: frozen `g2-train recovery native run --root .../anchor-pair/C50`.
첫 prepare의 corpus provenance 경로 오류와 개발 compile 실패도 별도 로그로 보존했다.
G2는 실행 실패 상태로 닫고 승인된 독립 저장 작업 G3/G4를 계속한다.

## G1 — 최종 모델과 저장된 stop 경계 검증

2026-09-18 / EXECUTED_THIS_RUN. CL-01/CL-02 RED→GREEN, CODE_CLOSE=VERIFIED.
실제 같은 shape/tokenizer/step의 B weight 한 값을 바꾸고 정상 native 저장한 뒤
B CROSS를 A terminal에 연결하면 기준 source는 정상 comparison을 발행했다.
다른 회귀의 실제 child는2 TINY updates 후 모든 panel을 생성하고 cleanup 취소
terminal만 durable하게 저장한 뒤 exit91했다. close-stop이 없는 새 process의
기준 close도 정상 comparison을 발행했다. 기존 F/N에 발생했다는 주장은 아니다.

수정은 공통 close에서 실제 terminal과 모든 final panel의 weights/tokenizer/equation/
numeric-policy/step을 대조하고 검증된 ancestry의 저장된 stop을 정상 close 전에
거부한다. 합법적인 중간 TimeBudget-only 재개와 정상 오답 close는 유지한다.
평가 참조는 inference 전용 파일도 허용하며 학습 재개의 Adam 검사는 유지한다.
같은 weights를 다른 metadata/physical hash의 RESUME 및 INFERENCE로 저장한 positive도
통과했다. 기존 semantic architecture ID는 numeric policy까지 포함한다(SOURCE_READ).

직접 unit7 PASS, inference positive 포함 추가1 PASS, process2 PASS.
기존 raw/checkpoint/final 시간 분할의 model/Adam/tape/guard/f64 bits 동일성을 유지했다.
신규 SMALL0/TINY14/scalar0, generation40/자체 teacher40: RED process2updates/6calls,
GREEN 기존process10/28과 저장취소process2/6. 이전에 검증한 bootstrap을 읽기 전용으로
재사용해 새 bootstrap 학습0이다. 구성 native/row fixture는 optimizer/generation0.
증거: `artifacts/native-storage-quality-20260918/g1-{red-model,red-cancel,green-unit,green-process,inference-positive}.log`.
하네스는 기존 binary test filter에 저장취소 process 회귀를 함께 실행하도록 수정했다.
MODEL_PAIR=NOT_RUN; 저장 경계 수리가 모델 품질의 개선을 뜻하지 않는다. NEXT=G2.

## G0 — native storage/quality 시작 상태

2026-09-18 / R3-NATIVE-STORAGE-QUALITY-1.0 / EXECUTED_THIS_RUN.
HEAD d19f39972ab70799527b567d5ad35a02c76ead5c, source
b18ec31bb78a8d0ef0adf985b46697a2d4993474. tracked clean, 기존 untracked487개 보존.
Rust/cargo1.98.1, Apple M4/24GiB/macOS27.0(26A428), Accelerate/threads1/offline lock.
소유 학습/benchmark process 없음. 기존 native/frozen6개와 data/policy/raw192개
보존 manifest의 실제 SHA 검증 모두 OK. SMALL/TINY/scalar updates0, generation/teacher0.
근거는 local `artifacts/native-storage-quality-20260918/{entry-status,native-entry,data-entry}.txt`.

SOURCE_READ: INFERENCE/RESUME 분리와 Adam seek skip은 기존 R3MODEL 구현,
원문 사건은 RPV3, readonly graph snapshot은 R3ARCH, 실험 기록은 R3ER다.
제품 IPC/기존 corpus·명시 import/개발자 출력 JSON은 남아 있다. 이 기능들은 새 구현으로
세지 않는다. G1 close의 terminal model/stop 검증 후에만 G2 승인 pair를 실행한다.
G3 저장 probe와 G4 별도 journal은 품질 미달을 성공으로 대체하지 않는다.
G1/G2/G3/G4=NOT_RUN, H3/S4 미통과, GOAL1_ACCEPTED=false, independent pending.

## J0–J5 최종 — BINARY_EVAL_RESUME_VERIFIED, 모델 품질 판정 불변

2026-09-18 / R3-BINARY-EVAL-RESUME-1.0 / IMPLEMENTER_REPORT.
RESULT=PASS_IN_REPAIR_SCOPE. 실제 평가 저장→TIME_BUDGET→별도 OS process/segment
재개→guard 정확히 한 번 적용→close를 native TINY로 검증했다. 아래 과거 기록은
해당 시점의 결과로 보존하며, 그때의 미실행 항목을 이번 실행으로 소급 덮지 않는다.

| 최종 필드 | 결과 |
| --- | --- |
| BR01_BINARY_FLOAT_IDENTITY | VERIFIED: 문제 f64를 실제 writer/reader/decision/close까지 bit 단위로 보존 |
| BR02_NATIVE_CHECKPOINT_BINDING | VERIFIED: 실제 파일 SHA/model/step/Adam/계보 검증, filename fallback 없음 |
| END_TO_END_NEW_SEGMENT_CLOSE | VERIFIED: 서로 다른 OS process와 segment에서 연속 실행과 동일 |
| HARNESS_BOUNDARIES / EVAL_GUARD_EXACTLY_ONCE | VERIFIED / VERIFIED |
| FROZEN_INPUT_BINDING / PANEL_COMPLETENESS | VERIFIED / VERIFIED: 소유 binary snapshot 및 전체 raw 재채점 |
| RAW_SCORE_AGREEMENT | VERIFIED_FOR_CURRENT_FILES: F512/N512/N256 기존 점수 일치 |
| HISTORICAL_RECEIPT_BINDING | F/N ABSENT; N256은 기존 사후 관측 binding, 과거 사전등록 증명 아님 |
| HISTORICAL_FLOAT_BITS | UNKNOWN / LEGACY_ROUNDTRIP_UNPROVEN |
| HISTORICAL_CANDIDATE_PROMOTION | NOT_AUTHORIZED, 원래 종료/부적격/resume=false 보존 |
| MODEL_OUTPUT_PARITY | F512 normal greedy dev16+ordinary16,32/32 일치 |
| MODEL_QUALITY_IMPROVED / H3_PASS / H3_SEAL | NOT_CLAIMED / NO / NOT_OPENED |
| S4 / S5 / S6 | NOT_PASSED / NOT_RUN_THIS_SCOPE / NOT_RUN_THIS_SCOPE |
| GOAL1_READY / GOAL1_ACCEPTED / INDEPENDENT_REVIEW | false / false / INDEPENDENT_PENDING |

### 구현과 검증한 경계

새 `src/experiment_record.rs`는 학습 실행기의 한정된 schema 전용 내부 모듈이다.
평가/input snapshot/decision/native reference/segment/comparison을 typed struct/enum과
custom binary R3ER v1로 저장한다. JSON 문자열·동적 map을 binary에 담지 않으며
새 identity는 exact binary bytes를 해시한다. f32/f64 LE bits, 명시적 error/tag,
길이 상한, 중복/누락/trailing 검사와 기존 immutable publisher를 사용한다.
`src/codec.rs`는 기존 Reader/varint/publisher의 접근 범위만 열었다. memory wire
알고리즘과 `.r3m`, tokenizer mapping, Cargo.lock, 모델 수식/학습 조건/DB는 불변이다.

`src/quality_recovery.rs`는 기존 CLI에 `recovery native`를 연결하고 strict answer
predicate와 기존 guard 계산을 공통 사용한다. 새 controller는 binary 소유 입력만
소비한다. 기존 corpus/frozen 검증과 scorer/native loader/RunControl을 재사용하며,
완료 평가와 guard-before에서 pending 판정을 복구한 뒤에만 다음 optimizer를 허용한다.
native 참조는 파일 위치와 내용 identity를 별도로 확인한다. 원 segment final과 후속
segment step의 정당한 재참조를 연결하되 다른 run/model/tokenizer/step/Adam/counter,
root escape/없는 참조/같은 step의 모호한 raw는 거부한다.

close는 frozen snapshot→완전한 raw→실제 decode/EOS/error→strict score→guard/native/
종료 계보를 검증한 후 comparison을 발행한다. 정상 오답은 candidate=false이며,
손상·불완전·취소는 별도 immutable close-stop이고 정상 결과를 발행하지 않는다.
품질/취소/save-error가 시간 종료와 함께 발생해도 재개 자격은 되살아나지 않는다.
각 파일의 durable publication을 다중 파일 transaction이라고 부르지 않는다. 동기
tensor/fsync 강제 선점은 없으며 generation/teacher/update/save/close 안전 경계에서
공통 command1800초·cleanup120초를 확인한다. 이번 SMALL run/resume은 허용하지 않는다.

새 `tests/experiment_record.rs`가 production 함수를 쓰는 실제 자식 process 회귀를
실행한다. test-support의 작은 spec/fault만 사용하며 모델은 실제 random-init TINY다.
`src/check_main.rs`는 해당 직접 unit 및 subprocess 회귀를 기존 quick에 추가했다.
영구 신규 파일은 이 두 Rust 파일뿐이다. 기존 docs4개를 갱신했고 새 framework,
외부 bridge, Python/외부 모델/API, 제품 정답 분기나 FakeModel을 추가하지 않았다.

### 실행한 테스트, 호출 수와 실패 보존

| 실행 | 관측 결과 / 증거 |
| --- | --- |
| 기준 source62147dc의 격리 RED-BR01 | 실제 JSON writer→reader에서 f64 bits4576864117419147264→4576864117419147263, guard identity 실패 |
| 같은 기준의 RED-BR02 | 실제 별도 process close가 원 segment의 없는 step 파일을 요구해 NotFound; 두 논리 결함 재현 |
| binary 직접 unit | 초기2 PASS, 보완 후3 PASS; 마지막 최신 source는5 PASS,0 FAILED |
| 실제 subprocess 회귀 | 직접1 PASS×2, 안정 quick 안에서1 PASS; 연속/평가 직후/체크포인트 직후/최종 평가 시간 분할 |
| 안정 quick | 한 번 실행,53 PASS; fmt/check/all-targets/clippy -D warnings PASS |
| 최종 소스 확인 | 직접5 PASS, clippy all-targets -D warnings PASS, release build PASS; quick 재실행 없음 |
| 최종 executable close | OS 수준 JSON/JSONL read 차단 상태에서 실제 F512 전체 close PASS |

연속/세 분할 경로는 같은 TINY 부모에서 각각 실제2 updates 후 absolute step26,
model60c60eba458a00ec2e1dbc3df55a6d4ffc2848111a25aa3d292846da43765dec,
같은 Adam/cursor/input·target 소비/raw tokens/guard에 도달했다. 판정2건이 각각 한 번
반영됐고 문제 f64 bits4576864117419147264도 동일하다. 취소+시간 및 품질+취소+시간
각 경로는1 update 뒤 중단, 이후 optimizer0, 재개 거부다. 직접 close의10개 구성
fixture는 정상 정답/오답과 누락·다른 case·잘못된 native/guard/decision·모호성·최종
취소를 검사한다. 이 구성 fixture의 optimizer/generation은0이다. RED의128/512
라벨은 실제 학습 횟수로 세지 않았다.

SMALL/TINY/SCALAR_UPDATES=0/97/0. TINY97은 bootstrap24×2=48, 실제 subprocess
회귀10×3=30, 기존 quick native Adam 회귀6+10+3=19의 합이며128 상한 이내다.
SMALL 학습 input/target tokens와 신규 학습 노출은0/0/0이다. 새 binary TINY 경로는
generation92/같은 자체 모델 teacher92(bootstrap8+subprocess84), SMALL은32/0이다.
기존 quick의 다른 legacy/native generation 회귀 총수는 별도 계측하지 않았다.
따라서124를 전체 테스트의 전역 generation 총수라고 주장하지 않는다. 외부 teacher0.
원자료 import/재채점/성능 측정/최종 close의 generation/optimizer는 모두0이다.

실패와 재시도도 보존했다. 첫 TINY fixture 준비는 corpus 계보 검사에서 optimizer0으로
실패했고 previous-corpora 연결을 고친 새 출력 경로에서 준비했다. 개발 중 compile
오류는 실행 테스트/RED로 세지 않는다. N256 최초 import는 inline score가 없는 사후
schema 때문에 `INTEGRITY_FAIL: score auxiliary`로 중단됐다. 원본 점수 오류로 해석하지
않고 별도 result의 required-field 검증을 추가한 뒤 같은 원자료를 다시 감사했다.
실패 시 정상 output root나 모델 호출은 없었다. 누락/null을 default로 통과시키지 않는다.
최종 추가 회귀는 읽은 bytes와 그 digest를 함께 소유하고, 이후 파일 변경을 이전
parsed 값의 provenance로 잘못 붙이지 않는다는 점도 확인한다.

주요 실행 명령은 기존 checker `quick --output artifacts/binary-eval-resume-20260918/quick-final`,
`cargo test --locked --offline --features accelerate,test-support --bin replica-train binary_tests`,
`cargo test --locked --offline --features accelerate,test-support --test experiment_record binary_real_process_resume_and_close`,
`replica-train recovery native import/parity/bench/close`다. test threads1과
VECLIB_MAXIMUM_THREADS=1/RAYON_NUM_THREADS=1을 사용했다. 각 quick의 정확한 argv/exit/count는
`quick-final/summary.json` 및 `command-*.json`에 있다. source tree hash도 같이 보존했다.
이는 개발자 검증 기록이며 새 canonical controller 입력은 아니다.

### 같은 기존 raw의 재검산과 출력 동등성

아래 표는 DERIVED_FROM_EXISTING_LOGS이며 재채점 실행 자체는 EXECUTED_THIS_RUN이다.
새 학습이나 N256 추가 생성으로 얻은 개선 수치가 아니다. dev256/watch32/CROSS512/
ordinary400를 각각 전체 ID/내용/순서와 해당 exact checkpoint step에 연결했다.

| checkpoint / absolute step | dev full/entity/event /256 | CROSS full/entity/event /512 | ordinary QA /336 | aux /64 | watch /32 |
| --- | --- | --- | ---: | ---: | ---: |
| F512 /23798 | 242/251/251 | 459/501/498 | 176 | 56 | 15 |
| N512 /23798 | 236/252/245 | 449/504/500 | 174 | 55 | 16 |
| N256 POST_HOC /23542 | 252/255/256 | 457/487/508 | 161 | 57 | 16 |
| 유지된 gate | >=244/254/254 | >=487/507/507 | >=178 | 별도 | 별도 guard |

최종 생성 오류는 모두0이다. N256은 dev만 통과하고 CROSS·ordinary는 실패한다.
선택/winner/resume/product pointer는 바꾸지 않았다. F/N HISTORICAL_RECEIPT_BINDING은
ABSENT이고 CURRENT_RAW_RECOUNT는 VERIFIED다. N256의 기존 사후 binding도 원래의
DEVELOPMENT 증거 수준 그대로다. 이번 binary 변환 해시는 현재 감사의 binding이다.
serde_json1.0.151 default/std/alloc, float_roundtrip 미사용, converter source와 기존
raw file SHA를 보존하며 과거 in-memory float bits가 복구됐다고 주장하지 않는다.

F512 parity는 점수를 보기 전에 category/family/base/ID에서 dev16·ordinary16을
고정하고 같은 native checkpoint/tokenizer/normal greedy로32번 생성했다. raw token,
EOS/text/error/prompt identity32/32 일치, teacher0, 재시도0이다. 새 parity panel tags는
전체 QA/dev gate를 대체하지 않는다. `F512-parity32/inputs.r3er`가 선택을 기록하고
`j4-parity32.log`와 `parity.r3er`가 실행 결과를 기록한다.

### 새 canonical binary와 잔존 JSON

CANONICAL_DIAGNOSTIC_JSON_WRITES=0, 새 native reader/resume/close LEGACY_JSON_READS=0.
TINY 재개 회귀는 `.r3er/.r3m`만 가진 디렉터리에서 완료했다. 실제 F512 close는
추가로 OS sandbox에서 모든 `.json/.jsonl` file-read-data를 거부하고 검증했다.
실제 policy JSON의 cat은 Operation not permitted, native close는 PASS였다.
원본 JSON을 rename/delete하지 않았다.

첫 sandbox close는 기존 RSS 관측용 `/bin/ps`가 OS 제한으로 실패해
RESOURCE_OBSERVATION_FAILED로 정직하게 중단했다. 실패 root의 close-stop은 보존했다.
그다음 profile은 `/bin/ps` 자식만 no-sandbox 실행을 허용하고 native process의 JSON
읽기 차단을 유지했다. RSS gate를 끄지 않았다. 새 root에서 통과했으며 마지막 최신
executable도 별도 root로 같은 차단 close를 통과했다. 이것은 새 canonical 경로의
JSON 비의존 증거이며 기존 프로젝트 전체에서 JSON이 사라졌다는 주장이 아니다.

| REMAINING_JSON_BY_ROLE | 현재 역할과 분리 경계 |
| --- | --- |
| 제품 model IPC | `model.rs` request/response framing 유지, 새 진단 상태 입력 아님 |
| 기존 의미 ID | config/request/native prompt/tokenizer 호환 ID를 opaque typed field로 보존, 새 binary digest와 구분 |
| corpus/frozen | `data.rs` 및 기존 recovery 입력의 JSON 유지; 명시 importer에서 검증 후 owned binary snapshot으로 변환 |
| 과거 checkpoint/tokenizer | 기존 명시 legacy importer만 사용; `.r3m`/mapping 불변 |
| 기존 recovery commands | 옛 이름의 JSON writer/reader 유지; native reader의 자동 fallback은 없음 |
| teacher 계산 adapter | 기존 같은 모델 계산의 transient Value를 즉시 typed record로 변환, JSON blob/hash로 저장하지 않음 |
| checker/developer reports | quick command/summary JSON, 사람용 출력 및 비정규 measurement 유지 |
| 새 명시 legacy audit | F/N 성공 각1회, N256 사전검사 실패1회+수정 후 성공1회; 원본 수정/소급 승격 없음 |
| benchmark reference | typed-json/legacy-row-json 이름의 `.dat`, 각15 durable 쓰기(워밍업 포함); canonical 상태로 사용하지 않음 |

LEGACY_JSON_READS는 명시 import/benchmark/기존 quick 경로에서 발생했다. 모든 파일 읽기
시스템콜을 전역 계측하지 않았으므로 가짜 총횟수를 기재하지 않는다. importer의 파싱과
해시 대상은 동일하게 소유한 실제 bytes다. native canonical의0과 legacy 전역 미계측을
구분한다. 향후 JSON 이관 순서는 필요 시 corpus의 명시 snapshot 입력, 기존 recovery
진입점 폐기/전환, 개발자 receipt, 마지막으로 별도 IPC 호환성 검토다. 이번에는 실행하지
않으며 model/tokenizer ID나 사용자 DB를 바꾸는 계획으로 자동 확장하지 않는다.

### 동등 데이터 저장 크기·속도

EXECUTED_THIS_RUN: Apple M4, RAM25769803776B(24GiB), macOS27.0 build26A428,
Rust/cargo1.98.1 release, CPU/Accelerate, compute threads1. warmup5, timed10, 압축 없음.
F512 dev256, raw output tokens9542. binary evaluation84869B, 같은 typed fields의 JSON
426543B(JSON digest는 byte array)다. legacy rows942922B와 active owned cases264019B+
binary panel84869B=348888B의 차이는 schema dedup+codec이며 순수 codec 효과가 아니다.
full inputs.r3er4006623B는 미사용 train/다른 panel까지 포함하므로 active panel 비교에서
분리했다. input/expected/derived output는 immutable case/tokenizer에서 재구성하고,
기존 teacher/timing/raw 진단 필드를 삭제해서 크기를 줄이지 않았다.

| 작업, ms median/p95 | binary | 같은 typed fields JSON | legacy row JSON |
| --- | ---: | ---: | ---: |
| encode | 0.597584/0.983916 | 0.425917/0.500792 | 0.742958/0.804042 |
| decode | 0.384750/0.615542 | 1.969084/2.591625 | 2.578916/3.055708 |
| hash | 0.150959/0.158792 | 0.747625/1.788500 | 1.656250/2.498417 |
| verify | 95.605542/111.046500 | 2.723291/4.281583 | 23.898791/25.165792 |
| durable write+sync+readback+directory sync+reload | 8.413250/8.593208 | 8.500917/8.631708 | 7.475000/8.326792 |

binary가 작고 이 측정에서 decode/hash가 빠르지만 encode는 typed JSON보다 느리다.
durable save는 일관된 속도 우위가 없다. verify는 binary의 전체 native panel 재채점,
typed JSON의 syntax/hash, legacy JSON의 기존 verifier/scorer로 작업량이 다르므로
semantic 검증 속도 배율을 계산하지 않는다. control/RSS 확인은 timed region 밖이다.
관측 종료 RSS285984KiB는 peak가 아니고 allocation profiler도 사용하지 않았다.
encode에는 검증용 temporary allocation이 포함되며 clone/peak 최소화를 달성했다고
주장하지 않는다. 원시 ns 값과 한계는 `storage-measurement/measurement.txt`에 있다.

### 재현 identity, 원본 경로와 보존

SOURCE_COMMIT=b18ec31bb78a8d0ef0adf985b46697a2d4993474,
origin/main의 같은 full SHA를 정상 push 후 직접 확인했다. J1–J3 source는
feec27439dbc0704335ad58c7bfca7cc8733f968이다. 이 절을 추가한 commit은 REPORT_COMMIT이며
소스 변경 없이 보고서만 게시한다. 최종 report/remote full SHA는 게시 응답에 별도 기록한다.
source 기준62147dc5854ca07a4c0785c0f006734d62074200, 시작 report HEAD
d075382539cc5f8bf7ba6de01b8d98a8d7aa7ffb와 이후 로컬 원본을 보존했다.

실행별 소스/바이너리를 구별한다. J3 quick worktree digest는
535ea2275e6f21616d188432d5841f956a067e1095b42b36ca51ca2bca97970f,
debug binary는83cd09c480c602c60ae7b27c8c9bde39197c4e3a051cde832ea2cecfcb9d7df3이다.
J4 parity/benchmark executable은5fea4ee9ec4f21bbee392b7a0b5cfe60c363c2308ad6cc45848f8d45c3a1973f,
그때 experiment_record.rs는f8d7e5c08632e6493256ac2f72bc59fcd7601dfe1cf1a763b0fe64d43754176a다.
J4 이후 importer의 exact consumed bytes 회귀를 추가했다. 최종 같은 파일 SHA는
572096dad474830638986a8e8719f6ac3639e8a3af53e6cb743af03a02f601cf,
최종 release executable은b661425d7970d7adf97533b09944bd728559b5b8918153d3776ec1f54528d6c2다.
마지막 direct5/clippy/build/JSON-denied close는 최종 소스에 해당한다. 생성/모델 경로는
그 수정에서 변경하지 않았으나 J4 parity를 최종 executable로 실행했다고 바꾸어 쓰지 않는다.

| 기존 모델 | 로컬 상대 경로 | physical SHA256 |
| --- | --- | --- |
| F512 | artifacts/h3-controlled-20260917/a2/F/segment-00-0000/final | bdc28e3f4b31ad5d600cf1c6ec0615b591deed9e26b069d9167f6a175f0a0e16 |
| N512 | artifacts/h3-controlled-20260917/a2/N/segment-00-0000/final | 734ec32aa4bfad8bce9ecf2387eae81f988cd9643b3257612f7527733d4f0bca |
| N256 | artifacts/h3-controlled-20260917/a2/N/segment-00-0000/step-0256 | 3556caaf40cef53183eb25c1c4162a6b7050ffe623cc785d4a810359400e9153 |

F model565a40a33ee896916a421dbee538d3f126789a67ee184039b28e7cc4224a375a,
N model870facdd31aca3a251dfe4b111745281724517b673f2a4d38aa6267d982594ea,
N256 modelf3802303a462575441849e91d1b00c5fe51e7cd77383cc28f533637eccf6eb1f,
공통 tokenizer652718e4864c2f06af3a2c67dac0a5174d5e2feb1387667173902da9a8a295d4.
F/N corpus는 `artifacts/h3-controlled-20260917/a2/corpus-F/`,
`artifacts/h3-controlled-20260917/a2/corpus-N/`의 manifest/train/validation.json이다.
policy는 `artifacts/h3-controlled-20260917/a2/{F,N}/policy.json`,
raw는 각 arm의 `segment-00-0000/{eval-0512,cross,ordinary400,result}.json`이다.
N256 dev는 같은 N segment의 `eval-0256.json`, 기존 사후 CROSS/ordinary/result는
`artifacts/state-data-result-20260917/reaudit-bound/N256/`이다.
ordinary=`artifacts/s4-completion-20260917/binding-corpus/validation.json`, SHA
572b0d9d797feb2c31fa8713566853aff91ae6b994d736c399e8bad0cb3fb004.
CROSS=`artifacts/h3-controlled-20260917/a0/cross-development.json`, SHA
c27bc3e35343c3c046fa2b9c944aae0601912f086fa2a739ed7900f0a6dbf28a.
frozen=`artifacts/harness-goal1-20260917/h2-baseline/frozen.json`, SHA
d3531005d3de1e76337bbdbee0454a9a0640bfaa26e8558aed2b61ca4fbf1cb7.

새 증거 root는 `artifacts/binary-eval-resume-20260918/`이다. 원자료/모델/로그는 local only다.
`import-F512/`, `import-N512/`, `import-N256/`, `F512-parity32/`, `storage-measurement/`,
`json-denied-final/`에 typed 결과가 있다. `red-br01.log`, `red-br02.log`,
`j3-e2e-01.log`, `j3-e2e-02.log`, `quick-final/`, `j4-F512.log`, `j4-N512.log`,
`j4-N256.log`, `j4-N256-repaired.log`, `j4-parity32.log`, `j4-storage-measurement.log`,
`j5-direct-tests.log`, `j5-clippy.log`, `j5-release-build.log`, `j5-json-denied-close.log`를
보존했다. `j4-observation-source.sha256`, `j4-final-source.sha256`,
`final-source-binary.sha256`와 보존 실행파일이 각 관측을 식별한다.
시작/종료의 기존 manifest 재검증에서 native/frozen6개와 data/policy/raw192개 모두
SHA 일치했다. 사용자 DB/종료 records/dirty 작업은 건드리지 않았고 명시 파일만 stage했다.

### NEXT_MODEL_TEST_PROPOSAL — anchor 샘플 비율 한 변수, 실행하지 않음

현재 관측은 copy 성능 변화와 ordinary 저하가 함께 있음을 보여준다. 원인은 UNRESOLVED다.
N256의 dev252가 CROSS/ordinary 수용으로 이어지지 않아 best-dev 선택만으로 회복을
보장할 수 없다. tokenizer/temperature/LR 또는 망각이 원인이라고 확정하지 않는다.

향후 별도 승인 후보는 같은 F512 부모·기존 F corpus·Adam·LR·tokenizer·normal greedy를
유지하고 batch8의 anchor/focus 비율만 기존4/4 대6/2로 비교하는 것이다. 각 arm 최대128
updates, 자동 연장 없음. 자료는 늘리지 않는다. 실행 전 두 tape와 실제 input/target
token 예산을 등록하고 길이 조건을 맞춘다. 동일 step을 동일 token 노출이라고 부르지
않으며 비교 가능한 노출 설계가 안 되면 시작하지 않는다. 기존 dev256/CROSS512/
ordinary336+aux64를 시작·최종에만 확인하고 guard/cancel/deadline과 기존 최종 gate를
그대로 적용한다. 정상 greedy 전체 조건을 모두 만족해야 후보 자격을 검토한다.
이는 미실행 가설/대조 제안 하나이며 예산 승인·학습 예약·과거 run 재개가 아니다.
NEXT_DEPENDENCY=independent source/evidence review와 별도 다음 실험 승인.

## J1–J3 종료 — binary 저장에서 별도 process 재개·close까지 검증

2026-09-18 / EXECUTED_THIS_RUN. J4 원자료/parity/성능 검증은 다음 단계다.
BR01_BINARY_FLOAT_IDENTITY=VERIFIED, BR02_NATIVE_CHECKPOINT_BINDING=VERIFIED,
END_TO_END_NEW_SEGMENT_CLOSE=VERIFIED. 모델 품질 개선/Goal1 승격은 주장하지 않는다.
테스트 source worktree digest535ea2275e6f21616d188432d5841f956a067e1095b42b36ca51ca2bca97970f,
debug binary83cd09c480c602c60ae7b27c8c9bde39197c4e3a051cde832ea2cecfcb9d7df3.
Rust/cargo1.98.1, Accelerate, compute threads1, locked/offline. 원본 보존, SMALL optimizer0.

실제 random-init TINY 준비24회×2와 연속/분할/취소 회귀10회×3, 기존 quick의
native Adam 회귀19회를 합쳐 TINY97/128, scalar0이다. 첫 준비/회귀 이후 input binding을
보완하면서 새 준비 디렉터리와 동일 회귀를 다시 실행했다. 실패한 fixture 초기화1건은
native corpus lineage 검사에서 중단됐고 optimizer0이었다. 컴파일 실패를 실행 테스트로
합산하지 않았다. 새 binary 경로의 실제 생성92/자체 teacher92; 기존 quick의 다른
generation 회귀 호출은 별도 총계 계측하지 않았으며 이92에 포함하지 않는다.

직접 unit2 PASS 후 추가 close 음성 fixture를 포함한 unit3 PASS, subprocess 회귀1 PASS를
두 차례 실행했다. 안정된 worktree에서 quick은 한 번 실행,53 tests PASS이며 fmt/check/
clippy -D warnings도 PASS다. 소스 정적 검사나 과거 실행 결과를 이53에 더하지 않았다.
증거: `artifacts/binary-eval-resume-20260918/j3-unit3.log`, `j3-unit-final.log`,
`j3-e2e-01.log`, `j3-e2e-02.log`, `quick-final/`, `quick-e2e/`.

연속/평가 저장 직후 종료/checkpoint 저장 직후 종료/최종 평가 시간 분할은 서로 다른
OS process·segment에서 완료했다. 같은 부모에서 실제2 updates 후 모델
60c60eba458a00ec2e1dbc3df55a6d4ffc2848111a25aa3d292846da43765dec,
Adam/cursor/token 소비/raw tokens/guard가 같고 판정은2건씩 정확히 한 번 적용됐다.
문제 f64의4576864117419147264 bits는 raw writer→reader→decision→close에서 보존됐다.
원 segment의 없는 step 파일을 요구하지 않으며 실제 final 또는 후속 segment의 명시
native 참조를 검증한다. 취소+시간, 품질+취소+시간은 다음 optimizer0, resume=false다.
complete wrong-output fixture는 integrity 오류 없이 candidate=false로 닫고, 누락/중복/
다른 case/손상 native·raw·decision/최종 취소는 정상 comparison 발행을 거부한다.

새 canonical loader의 JSON sidecar read/write는0이며, 격리 회귀 root에는 `.r3er`와
`.r3m`만 존재한다. transient 기존 teacher 계산 adapter와 호환용 config/request ID의
JSON 사용은 남아 있다. 기존 이름의 legacy recovery/corpus/harness writer도 그대로이며,
새 native loader가 이 경로로 fallback하지 않는다. 저장 형식과 잔존 경계는 기존
STORAGE_FORMAT/RUNBOOK에 기록했다. J4와 독립 검토는 아직 NOT_RUN/INDEPENDENT_PENDING.

## 현재 J0 종료 — binary 평가/재개 수리의 두 결함 재현

2026-09-18 / R3-BINARY-EVAL-RESUME-1.0 / EXECUTED_THIS_RUN, INDEPENDENT_PENDING.
HEADd075382539cc5f8bf7ba6de01b8d98a8d7aa7ffb, sourced621 기준은
62147dc5854ca07a4c0785c0f006734d62074200이며 차이는 상태 보고서뿐이다.
main/origin seoyd/replica-v3, tracked clean, 기존 untracked 작업 보존. Rust/cargo1.98.1.
시작 시 소유 모델 프로세스 없음. SMALL/TINY/scalar optimizer0, 생성/teacher0.

격리된 기준 source의 실제 writer→파일→reader→guard consumer에서 문제 f64의 bits가
4576864117419147264→4576864117419147263으로 바뀌고 evaluation decision digest 오류가
발생했다(RED-BR01). 같은 기준의 별도 OS process에서 검증한 원 final step128→새
segment step128/guard 복구→준비된 마지막 상태→실제 close가 원 segment의 없는 step
파일을 요구해 NotFound로 실패했다(RED-BR02). 준비된128/512 라벨은 실제 updates가
아니다. BR01은1 실패, BR02는 부모/자식 각각1 실패이며 의도한 consumer assertion이다.
컴파일/의존/timeout 실패를 버그 재현으로 세지 않았다. J1–J3 구현/통합 실행은 아직 NOT_RUN.

로컬 증거 root=`artifacts/binary-eval-resume-20260918/`: `base-red/`, `red-target/`,
`red-br01.log`, `red-br02.log`, `entry-status.txt`, `serde-json-features.txt`,
`native-entry-verification.txt`, `data-entry-verification.txt`.
보존한 기준 release executable `base-train` SHA=
80b78a1716c303524c925a69ad6d069a40076a864ac10e9c7a76673cc7502a53.
기존 보존 manifest의6개 항목(실제 native5개+frozen1개)과 데이터/정책/raw192개 모두 OK.
원본이나 종료 flags를 수정하지 않았다. 원격 source와 로컬 격리 시험 수정은 구분한다.

REMAINING_JSON_LEDGER (SOURCE_READ):

| 역할 | 현재 실제 호출 / 이번 경계 |
| --- | --- |
| model artifact | `neural/artifact.rs`의 native .r3m은 이미 binary; config/source legacy ID는 유지 |
| tokenizer | `neural.rs`의 자체 학습·legacy importer 및 `transformer.rs` config ID는 JSON 호환 사용; mapping 불변 |
| evidence/memory | canonical 사건은 `codec.rs` binary/SQLite; request/evidence의 IPC 직렬화는 별도 |
| corpus | `data.rs`의 manifest/train/validation JSON: 명시 read-only import 후 owned binary snapshot으로 이동 |
| evaluator | `quality_recovery.rs`의 평가 rows/teacher 및 JSON 재직렬화 identity: 이번 typed binary 전환 대상 |
| stop/resume/close | policy/frozen 최소 snapshot, decision/native link/terminal/comparison: 이번 canonical 전환 대상 |
| production IPC | `model.rs` framing/request/response와 request digest의 JSON: 이번 범위 밖, 유지 |
| developer output | `check_main.rs` quick/release receipt 및 CLI 표시의 JSON: 잔존; 신규 binary 제어 입력으로 사용 금지 |
| legacy import | `checkpoint.rs`의 명시 legacy manifest/tokenizer reader, 과거 recovery reader: read-only 감사 경계 |

실제 lock의 serde_json은1.0.151이고 feature는 alloc/default/std다. float_roundtrip은
활성화하지 않았다. 전역 reader feature를 바꾸어 과거 identity를 재정의하지 않는다.
다음 경계는 고정 schema·LE float bits·exact bytes identity와 실제 native 참조다.
모델 출력/품질/H3/S4/S5/S6/Goal1 개선은 이번 J0로 주장하지 않는다.

## B0–B6 종료: 진단 경계 검증 완료, H3 품질 미달 유지

2026-09-18 / R3-H3-STATE-DATA-RESULT-1.0 / IMPLEMENTER_REPORT.
이번 승인 범위의 결과는 DIAGNOSTIC_BOUNDARIES_VERIFIED다. 신규 학습은 하지 않았다.
독립 검토는 INDEPENDENT_PENDING이며 GOAL1_ACCEPTED=false다. 아래 기존 기록은 보존한다.

| 판정 | 이번 결과와 한계 |
| --- | --- |
| CODE_VERDICT / HARNESS_BOUNDARIES | PASS / DIAGNOSTIC_BOUNDARIES_VERIFIED |
| EVAL_GUARD_EXACTLY_ONCE | VERIFIED: 실제 평가 저장/중단/재개 경계의 RED→GREEN 및 정상 TINY 연속성 |
| FROZEN_INPUT_BINDING / PANEL_COMPLETENESS | VERIFIED / VERIFIED: 공통 소유 데이터와 완전한 원문 panel을 먼저 검증 |
| RAW_SCORE_AGREEMENT / HISTORICAL_RESULTS_CONFIRMED | VERIFIED / YES_FOR_CURRENT_FILES: F/N 현재 원자료 재채점 일치 |
| HISTORICAL_RECEIPT_BINDING | ABSENT: 현재 해시가 과거 쓰기 시점의 검증 증거를 만들지는 않음 |
| SOURCE_PROVENANCE | 기존 policy/source 값 보존; 현재 파일 binding은 이번 실행으로 검증 |
| HISTORICAL_CANDIDATE_PROMOTION | NOT_AUTHORIZED; 원래 selected/candidate/resume/종료 기록 불변 |
| N256_POSTHOC_STATUS | EXECUTED_THIS_RUN / DEVELOPMENT: dev raw 재사용, CROSS/ordinary 새 무학습 관측 |
| MODEL_QUALITY_RECOVERED / H3_PASS | NO / NO: 모든 개발 panel을 함께 통과한 모델 없음 |
| H3_SEAL | NOT_OPENED |
| S4 / S5 / S6 / GOAL1_READY | NOT_PASSED / NOT_RUN_THIS_SCOPE / NOT_RUN_THIS_SCOPE / false |
| INDEPENDENT_REVIEW | INDEPENDENT_PENDING |

수정한 세 경계는 `src/quality_recovery.rs`에 있다. 평가 receipt는 run/policy/model/step/
panel/evaluation 및 guard before/after/applied identity를 연결한다. 신규 optimizer 전에
미처리 판정을 복구하고 같은 평가를 다시 소비하지 않는다. 완성된 판정의 짧은 복구는
중단 안전 경계에서 수행하며, 동기 tensor/fsync 내부의 강제 선점은 보장하지 않는다.
서로 다른 파일 쓰기를 하나의 transaction이라고 주장하지 않는다. 단일 command 작업
1800초와 cleanup120초 한계를 유지하며 취소·시간·품질 사유를 함께 보존한다.

`progress_baseline`, `progress_prepare`, `progress_renewal`, 신규/시간 재개 `progress_arm`,
최종 ordinary 및 `progress_close`가 같은 frozen 검증을 거친 소유 episode를 소비한다.
`replay_cases`도 기존 frozen corpus 검증을 재사용한다. `src/data.rs`의 기존 renewal
생성기는 검증된 snapshot을 받아 경로를 다시 읽지 않는다. 최종/중간 raw의 순서·전체
내용·정답·normal greedy token/strict UTF-8/EOS·native step·종료 receipt를 검증하고
기존 scorer로 재계산한 뒤 비교와 자격을 결정한다. `src/check_main.rs`에는 해당 직접
회귀 필터만 추가했다. 최종 후보 자격은 ordinary/watch의 생성 정상성도 별도로 요구한다.
ordinary 정답 수가 높아도 length 종료/빈 출력/오류를 후보 통과로 숨기지 않는다.
모델 수식/커널/tokenizer/SQLite/native tensor 포맷은 변경하지 않았다.

### 같은 모델·같은 panel의 실제 결과

| checkpoint | 누적 step | dev full/entity/event (각 /256) | CROSS full/entity/event (각 /512) | ordinary QA /336 | aux /64 |
| --- | ---: | --- | --- | ---: | ---: |
| F512 | 23798 | 242 / 251 / 251 | 459 / 501 / 498 | 176 | 56 |
| N512 | 23798 | 236 / 252 / 245 | 449 / 504 / 500 | 174 | 55 |
| N256 POST_HOC | 23542 | 252 / 255 / 256 | 457 / 487 / 508 | 161 | 57 |
| 변경하지 않은 기준 | — | >=244 / >=254 / >=254 | >=487 / >=507 / >=507 | >=178 | 별도 보고 |

F/N 행과 N256 dev는 DERIVED_FROM_EXISTING_LOGS: 전체 raw를 현재 frozen/native와 검산했다.
N256 CROSS512+ordinary400는 EXECUTED_THIS_RUN: 같은 기존 step-0256 가중치로 한 번 생성했다.
dev와 다른 step의 CROSS를 혼합하지 않았다. 모든 표의 최종 생성 오류는0이다.
N256 dev의 높은 값만으로 수용하지 않는다. CROSS entity487/512와 ordinary161/336도
각 기준에 못 미친다. N256 ordinary는 N512의174보다 낮으므로 early stopping으로 일반
QA가 복구된다고 해석할 수도 없다. ordinary의 aux64는 QA336 분모에 합산하지 않았다.

이 checkpoint는 기존 dev 결과를 본 뒤 선정한 사후 진단 대상이다. 과거 F/N 실험의 사전등록 승자가 아니며, 이번에는 seal·제품 승격·학습 재개를 하지 않는다.

F/N dev base4/4는56/64,53/64; CROSS는 둘 다106/128이다. N256은 dev61/64,
CROSS104/128이다. N을 treatment로 한 재계산 paired counts의 순서는
both-wrong/gain/loss/both-correct다: CROSS [36,17,27,432], ordinary400 [155,13,16,216].
dev gain4/loss10, base gain3/loss6이다. 순위 selected=F를 품질 합격으로 바꾸지 않았다.
N256 CROSS 첫 차이 분류는 format32/entity10/context6/citation4/value3이며,
전체 필드 오답 수와는 다른 통계다. ordinary QA 범주는12/68,19/68,5/68,62/68,63/64다.

### 실행·테스트·보존

B4 recount는7.024249917초, 모델 generation/teacher0이었다. F16/N16은 실행 전에
고정한 dev 정답8+오답8을 각 새 프로세스로 정상 생성했고 raw token/bytes/prompt/
actual/error/EOS/finish 차이는 각각0이었다. 실행 시간은4.028350750초와3.860064667초다.
N256은 기존의 완전한 dev256을 검증 후 재사용하고 나머지912문항만 생성했다.
N256 작업 시간120.021135916초, terminal=COMPLETED, observed_conditions=[]였다.

NEW_SMALL_UPDATES=0, 새 학습 input/target tokens=0/0, 새 학습 노출=0.
NEW_GENERATIONS=944/1200, 같은 자체 모델의 무학습 teacher calls=944이며 외부 teacher는 없다.
실패한 최초 큰 보고서 consumer는 generation0에서 거부됐다. 별도 재채점은 동일 원자료만
읽었으며 그 실패 출력도 보존했다. 추가 sweep, retry generation 또는 예산 연장은 없다.
NEW_TINY_UPDATES=82, scalar updates=0: 직접 연속성 회귀3+3와 네 quick 실행 각각19.
TINY generation은 통과한 테스트 목록과 해당 소스 경로로 재집계한120회(각 quick30),
teacher108회(각27)다. 별도로 이미 취소된 native 생성 API 진입4회가 있으며 forward와
토큰 생성은0이다. 새 경계 fixture 및 마지막 관측 중단 회귀의 generation/teacher는0이다.
이 TINY 수치는 실행 로그와 SOURCE_READ를 결합한 재집계이며 SMALL 관측과 합산하지 않는다.
모의128/256/512 step 라벨을 optimizer 실행 수로 세지 않았다.

RED는 수정 전 실제 평가 저장 직후 시간 종료의 판정 누락1건 및 기준 소스 close의
빈 CROSS/변경 ordinary 수용2건이다. 컴파일 실패를 RED로 세지 않았다. GREEN은
정상 close와 시간 분할 model/Adam/sampler 정합성을 포함한 직접 state/data 회귀8개다.
영향 quick 마지막 실행은47 PASS였다. 이후의 작은 수정은 직접8개와 fmt/clippy/release로
확인했고, 마지막 관측 메타데이터 보강은 별도 실제 진입점 회귀1개 PASS(6.37초),
fmt/clippy/release PASS로 확인했다. quick의 source digest를 최종 source로 소급 바꾸지 않는다.
관측 회귀는 F16/N256에서 첫 생성 전 TIME_BUDGET, raw 해시와 불완전 종료 보존,
자동 재시도 거부를 확인했다. 최종 후보 조건 회귀1개도 PASS(8.62초): 합성 ordinary
QA335/336에서 length 종료가 있는 후보를 실제 close가 거부하고, 후보가 아닌 완전한
오답 재채점은400 분모로 허용했다. 추가 optimizer/generation/teacher는0이었다.
그 수정 후에도 fmt/clippy/release가 통과했다. 전체 무관한 테스트 suite나 모델 재평가는
추가 실행하지 않았다. 두 후속 회귀는 이전 quick47/직접8과 별도 실행이다.

원본 native6개와 데이터/정책/raw192개 보존 manifest 검증이 모두 OK였다.
기존 untracked487개 목록도 동일하다. 운영 모델 포인터와 DB를 건드리지 않았으며,
종료 시 소유 모델/학습/평가 프로세스는 없다. 신규 영구 파일은0개다.

### source 및 로컬 검토 경로

REVIEW_BASE=bf705ad823d132a2f91e3df7323d49c3c23e2ee4.
최종 SOURCE_COMMIT=62147dc5854ca07a4c0785c0f006734d62074200; 정상 push 후 실제
origin/main 전체 SHA 일치를 확인했다. 이 최신 절은 그 뒤의 별도 보고서 commit에 담는다.
최종 source digest=f0a3f9d5a423d7fdba865963d28ae0788057de2be7b0fd085061de45f7c4d569.
최종 release train SHA=80b78a1716c303524c925a69ad6d069a40076a864ac10e9c7a76673cc7502a53;
이 바이너리는 마지막 관측 receipt/후보 조건 보강 후 빌드했으며 추가 SMALL 관측에는
사용하지 않았다. 해당 보강이 현재 F/N/N256 raw 점수를 바꾸지는 않는다.

실제 B4/B5 관측 code commit=adbd83ba99a6790fd53a011fb74f40f3dbe923c2,
source digest=e41f53941abc1db93e8b7aa6364b1b2276175e7adf7e9040156080fd44cbfcae,
고정 executable=`artifacts/state-data-result-20260917/repair-train-bound`, SHA=
a2f4020b9312b822dcfff0d4296fa2f94e493d3856ed92e693fcdbd5987afeb7.
N256 실행 당시 HEAD=f41b53adb817115808332e1636c91699037215cd는 B4 보고서 commit이며
관측 코드는 adbd83b와 동일하다. Rust/cargo1.98.1, locked/offline Accelerate,
VECLIB_MAXIMUM_THREADS=1/RAYON_NUM_THREADS=1이었다. 변경 executable의 동적 의존은
기존 system/Foundation/Accelerate/iconv/objc이며 외부 모델 bridge는 없다.

기존 N256 registration의 `selection` 문자열에는 F16/N16 설명이 잘못 복사돼 있다.
그 파일의 kind/mode/indices=[]/상한912와 실제 완전한 CROSS512/ordinary400 및 결과로
실행 범위를 확인했다. 원본을 고치지 않았다. 최종 소스에서는 설명을 분기하고 공통
writer로 evaluator source, registration/재사용 dev/raw 파일 해시를 종료 receipt에
직접 연결한다. 기존 관측의 해시는 별도 `observation-closure.sha256`에 현재 시점 binding으로
남겼다. 이를 과거 receipt 보강 또는 마지막 바이너리로 재실행한 증거라고 부르지 않는다.

아래 경로는 저장소 루트 `/Users/seo/Projects/Replica-v3` 기준이며 원문은 로컬에만 있다.

| 자료 | 실제 경로 / SHA256 |
| --- | --- |
| A0 | `artifacts/h3-controlled-20260917/a0/summary.json` / afadb76c0898deb0762bfbaddd88f16e9d633c16ca144ef3e541ddd5ef293c4a |
| frozen | `artifacts/harness-goal1-20260917/h2-baseline/frozen.json` / d3531005d3de1e76337bbdbee0454a9a0640bfaa26e8558aed2b61ca4fbf1cb7 |
| original H3 train/dev | `artifacts/harness-goal1-20260917/h3-corpus/{train,validation}.json`; dev ca5c86f91b4defe471bf1c6c2931bcc97ac0b88e084bd5506b65b0101ebccd89 |
| CROSS | `artifacts/h3-controlled-20260917/a0/cross-development.json` / c27bc3e35343c3c046fa2b9c944aae0601912f086fa2a739ed7900f0a6dbf28a |
| ordinary400 | `artifacts/s4-completion-20260917/binding-corpus/validation.json` / 572b0d9d797feb2c31fa8713566853aff91ae6b994d736c399e8bad0cb3fb004 |
| F train | `artifacts/h3-controlled-20260917/a2/corpus-F/train.json` / fe36d1a44a0dd1b6370777c23d588f1c928c0fd242ff186f66985ce027723cde |
| N train | `artifacts/h3-controlled-20260917/a2/corpus-N/train.json` / 41aa4ead3f90d7a3e869022105ebc41e00656ddd52aff879ce9eb4766929676a |
| F512 native | `artifacts/h3-controlled-20260917/a2/F/segment-00-0000/final` / bdc28e3f4b31ad5d600cf1c6ec0615b591deed9e26b069d9167f6a175f0a0e16 |
| N512 native | `artifacts/h3-controlled-20260917/a2/N/segment-00-0000/final` / 734ec32aa4bfad8bce9ecf2387eae81f988cd9643b3257612f7527733d4f0bca |
| N256 native | `artifacts/h3-controlled-20260917/a2/N/segment-00-0000/step-0256` / 3556caaf40cef53183eb25c1c4162a6b7050ffe623cc785d4a810359400e9153 |
| N256 model content | f3802303a462575441849e91d1b00c5fe51e7cd77383cc28f533637eccf6eb1f |
| N policy | `artifacts/h3-controlled-20260917/a2/N/policy.json` / 7ed8feeef47a05203e8982ac68aee5921d1b700835cc63e7feced0e9072db5fa |
| tokenizer semantic | 652718e4864c2f06af3a2c67dac0a5174d5e2feb1387667173902da9a8a295d4 |

원래 F/N policy와 raw는 `artifacts/h3-controlled-20260917/a2/{F,N}/` 아래에 있으며
각 `segment-00-0000/{eval-0000,eval-0128,eval-0256,eval-0512,cross,ordinary400,result}.json`
및 `trace.jsonl`을 보존했다. L 부모는 `artifacts/h3-controlled-20260917/a1/L/segment-00-0000/final`,
native SHA20996fa3c4ac2a0445014bf1c0fd394cefbc34c9f64f74c4dce62ea81e17a4eb다.
최신 관측 root=`artifacts/state-data-result-20260917/reaudit-bound`이며
`reaudit.json`, `F16/{registration,raw,result}.json`, `N16/{registration,raw,result}.json`,
`N256/{registration,cross,ordinary,result}.json`이 있다.
reaudit SHA97b3fba4d8ed58dc4279d862061019d44ccd5d861fc2694fe1e00410aa057bb2,
N256 result SHAfea60391dcaa0eadf9518d8a4ae33dd3c4aa9a20540794a293733d5c425d921a다.

실제 실행한 명령은 다음과 같다. 출력은 이미 존재하므로 재실행/덮어쓰기 지시가 아니다.

```sh
VECLIB_MAXIMUM_THREADS=1 RAYON_NUM_THREADS=1 artifacts/state-data-result-20260917/repair-train-bound recovery progress-reaudit --experiment artifacts/h3-controlled-20260917/a2 --output artifacts/state-data-result-20260917/reaudit-bound
VECLIB_MAXIMUM_THREADS=1 RAYON_NUM_THREADS=1 artifacts/state-data-result-20260917/repair-train-bound recovery progress-reaudit --experiment artifacts/h3-controlled-20260917/a2 --output artifacts/state-data-result-20260917/reaudit-bound --observe F16
VECLIB_MAXIMUM_THREADS=1 RAYON_NUM_THREADS=1 artifacts/state-data-result-20260917/repair-train-bound recovery progress-reaudit --experiment artifacts/h3-controlled-20260917/a2 --output artifacts/state-data-result-20260917/reaudit-bound --observe N16
VECLIB_MAXIMUM_THREADS=1 RAYON_NUM_THREADS=1 artifacts/state-data-result-20260917/repair-train-bound recovery progress-reaudit --experiment artifacts/h3-controlled-20260917/a2 --output artifacts/state-data-result-20260917/reaudit-bound --observe N256
```

로그 root=`artifacts/state-data-result-20260917`: `b1-red.log`, `b2-b3-baseline-red.log`,
`boundaries-final.log`, `quick-bound/summary.json`, `observation-metadata-test.log`,
`closure-{fmt,clippy,release}.log`, `candidate-generation-test.log`,
`candidate-{fmt,clippy,release}.log`, `b4-bound-{recount,F16,N16}.log`, `b5-N256.log`.
`preserved-{native,data}.sha256` 및 `*-preservation-final.log`는 원본 보존 근거,
`entry-status.txt`와 untracked 목록은 시작 상태, `observation-closure.sha256`는 이번
관측 파일의 현재 해시 목록이다. 실패/중간 로그를 지우거나 공개 업로드하지 않았다.

### 이번에 답한 질문과 다음 단일 가설

| 질문 | 확인한 근거 / 결론 |
| --- | --- |
| 기존 F/N 점수는 raw와 같은가 | 현재 frozen·native·모든 final raw 재계산 일치. 당시 receipt binding은 부재 |
| 시간 재개가 guard를 건너뛸 수 있었나 | 실제 경계 RED로 재현했고 pending/applied 회귀 및 정상 연속성 GREEN으로 수리 확인 |
| N 중간 향상이 다른 자료에도 있었나 | 같은 N256 dev252/CROSS457/ordinary161. 전체 gate 개선으로 일반화할 수 없음 |
| 복사 증가와 일반 QA 하락이 함께 있었나 | 같은 frozen에서 A0 dev208/ordinary178 대비 F242/176, N236/174. trade-off 관측이며 망각 원인 확정 아님 |
| 다음에 바꿀 한 변수는 무엇인가 | 원인은 UNRESOLVED. 기존 ordinary anchor의 학습 노출 비율을 단일 비교 변수로 검토할 근거가 생김 |

다음 가설은 미등록/미실행이다. 향후 별도 승인·명시적 fork에서 같은 부모/Adam/LR/
tokenizer/normal greedy/자료와 총 draw 수를 고정하고 ordinary anchor draw 비율 하나만
대조할 수 있다. 제안 예산은 두 arm 각 최대128 updates(합256), dev/watch의0/64/128
평가와 끝점 CROSS/ordinary 한 번이며 경고·중단·최종 품질 기준은 유지한다. anchor 증가가
복사 품질을 악화시키거나 일반 QA를 보존하지 못하면 음성 결과로 닫는다. 이번 실행0이며
새 예산·자료·seal 접근·학습 재개를 등록하거나 예약하지 않았다. tokenizer/temperature/LR가
현재 실패의 원인이라고 확정하지 않는다.

NODE=B6, STATE=CLOSED_DIAGNOSTIC_BOUNDARIES_VERIFIED, RAW_RECOUNT_STATUS=VERIFIED,
STOP_REASON=AUTHORIZED_DIAGNOSTIC_SCOPE_COMPLETE, NEXT_DEPENDENCY=INDEPENDENT_REVIEW.
원자료 신뢰성 확인·도구 수리와 모델 품질 미달을 별도로 판정한 상태다.

## B4 실제 F/N 재검산 및 새 프로세스 대조 완료

2026-09-18 / EXECUTED_THIS_RUN + DERIVED_FROM_EXISTING_LOGS, INDEPENDENT_PENDING.
Source adbd83ba99a6790fd53a011fb74f40f3dbe923c2 was pushed and directly matched origin/main.
The corrected reaudit is4,350,443bytes, readable by the unchanged bounded reader. Recount
elapsed7.024249917s, generation0/teacher0. F dev242/entity251/event251 of256,
CROSS459/entity501/event498 of512, ordinary176/336, auxiliary56/64; N236/252/245,
449/504/500,174/336,55/64. Raw scores, field/base scores and paired counts match the
existing reports. Historical panel receipt binding=ABSENT; current raw recount=VERIFIED.
Current native, tokenizer, clock, frozen/corpus/policy and parent physical hashes agree.
This establishes current consistency, not a newly invented historical write-time binding.

F16 and N16 each registered first8 exact+first8 incorrect dev cases before free generation.
Separate processes each executed16 generation and16 same-model teacher calls; raw token,
bytes, prompt, actual/error, EOS and finish differences0. Elapsed4.028350750s and3.860064667s.
The first oversized-report refusal generated0 cases and remains in the prior output/logs.
SMALL optimizer0; cumulative TINY optimizer82/scalar0 from bounded regression runs only.
All original final candidate/resume=false records and quality failures remain intact.

Evidence: `artifacts/state-data-result-20260917/reaudit-bound/reaudit.json`,
`F16/{registration,raw,result}.json`, `N16/{registration,raw,result}.json`, and
`b4-bound-{recount,F16,N16}.log` in the experiment evidence root. The code, source digest and
frozen executable identities are recorded in the preceding repair section below.
Next: only the existing N256 post-hoc DEVELOPMENT observation; no learning or seal opening.

## 경계 수리 후속 검증 및 B4 진입

2026-09-18. First source publication c7810ee4c896ef616349b7034385c64e926e91b0 matched
origin/main. The first actual F/N recount verified existing scores and ABSENT historical
panel bindings, generation/teacher0. Its report duplicated verified raw payloads and grew
to19,961,942bytes; the following F16 command correctly refused it before model generation
at the existing16MiB reader boundary. That report and failure log remain immutable.
The fix keeps the owned rows through paired scoring, then publishes only their original
file references, hashes and bindings. Resume also verifies the immutable raw files linked
by prior segment receipts; the common input loader verifies physical parent/receipt hashes.
Newly written development-candidate eligibility is derived from complete safe quality gates;
historical false eligibility and all seal/Goal1 restrictions remain unchanged.

Final full quick-bound:47 passing executions,0 failed/ignored, source unchanged at
a3d4df39357e10a23bb3fbe5d62b2d85a4ce574651093d997b05484526ae6463.
Subsequent small changes (derived eligibility, evidence-text/high-gate negatives) passed
all8 direct state_data regressions plus final fmt/clippy and release builds. The final
source digest is e41f53941abc1db93e8b7aa6364b1b2276175e7adf7e9040156080fd44cbfcae;
do not label the earlier quick receipt as identical to this later source.
Actual TINY optimizer total82: direct continuity3+3, four quick runs each19.
Scalar0, SMALL0. Repeated checks followed concrete fixes; their counts are executions,
not distinct tests or model-quality measurements. Final binary
`artifacts/state-data-result-20260917/repair-train-bound` has SHA
a2f4020b9312b822dcfff0d4296fa2f94e493d3856ed92e693fcdbd5987afeb7.
Final dynamic dependencies remain existing system/Foundation/Accelerate/iconv libraries.
The new output directory for B4/B5 is `artifacts/state-data-result-20260917/reaudit-bound`;
no failed output or original experiment is overwritten.

## 현재: 상태·자료·raw 판정 경계 수리 — B0/B1/B2/B3 검증 종료

2026-09-18 / R3-H3-STATE-DATA-RESULT-1.0 / IMPLEMENTER_REPORT, INDEPENDENT_PENDING.
Entry source bf705ad823d132a2f91e3df7323d49c3c23e2ee4, main, origin seoyd/replica-v3;
tracked clean, original untracked work retained. No active model process at entry.
Installed Rust/cargo1.98.1, locked offline Accelerate builds. New SMALL optimizer calls0.
Source, native model content/physical hashes and execution binaries remain separate identities.

SD-01: the actual progress-arm evaluation boundary now records a policy/model/step/panel/
evaluation key, durable guard-before, computed guard-after and the applied identity.
Pending complete decisions reconcile before the next optimizer, including time resume;
applied decisions validate without incrementing streaks again. Zero/parent evaluation is
excluded from streak consumption. Partial, ambiguous legacy or conflicting digest/model/
clock state fails closed. Quality/time/cancel observations remain sticky. Immutable raw,
decision sidecar and terminal digest are linked; separate writes are not called a filesystem
transaction. Existing cooperative tensor/fsync limits still apply.

SD-02: baseline, prepare, renewal, new/resumed arm, final ordinary and close now share
frozen-byte/manifest/policy validation and owned data. Ordinary336+aux64/watch32, H3 dev256,
CROSS512, A0/frozen, source/current train/dev and A2 parent comparison identities are checked.
The existing renewal generator takes the already verified corpus snapshot rather than
reopening its path; generation/data grammar is unchanged. replay_cases reuses the same
frozen corpus binding. No user corpus, model, storage format or model mathematics changed.

SD-03: complete ordered raw rows bind original question/evidence/answer/metadata, prepared
prompt, tokenizer-decoded bytes, EOS/finish/error and component scores before rescoring.
First byte/field mismatch is independently recalculated from raw bytes. New panel/file
receipts bind source split, model, tokenizer, policy, evaluator source and native step.
Intermediate dev/watch and final CROSS/ordinary are verified before ranking. Original
endpoint cancellation/incompleteness/quality stops cannot be waived by close. Strict close
rejects absent receipts; explicit read-only reaudit records current bindings without
backfilling historical files. Failed close emits a separate failure receipt, never success.

RED: evaluation raw-save→TIME_BUDGET lost the pending guard in the unchanged extracted
orchestration (b1-red.log). A separate baseline-source checkout with fixture helpers and
regressions, leaving its close implementation unchanged, accepted both empty CROSS arrays
and changed ordinary content (b2-b3-baseline-red.log,2 failed assertions). These are dynamic
reproductions, not compilation failures and not evidence of actual F/N score contamination.
GREEN: seven direct state/data/panel regression tests pass, including positive complete close,
normal TINY native time-split model/Adam/sampler parity, corrupt pending decisions, duplicate
application, simultaneous stops, all entry data rejection, renewal pre-generation rejection,
raw tampering and sticky read-only legacy handling. Fixture labels128/256/512 are not actual
optimizer calls. This run's direct regressions used3 actual TINY optimizer calls; SMALL0.
The final quick harness executes46 passing tests,0 failed/ignored, with source unchanged;
its TINY optimizer calls are19 (existing LR10+renewal6 and new time-split3), scalar0.
Total through B3: actual TINY22, scalar0, SMALL0. Initial fixture-validation/clippy failures
are retained; they are not classified as the original boundary defects.

Quick source digest f0884561e1c9091d7a905ab836393bc41c1d82910661899ed75b6aec2d33cb1c.
Evidence root: `artifacts/state-data-result-20260917/` (local only), including entry-status,
preserved native/data SHA manifests, all RED/GREEN logs, isolated baseline-source reproduction
and quick/summary.json. CODE/HARNESS checked scope PASS; B4 actual F/N recount and bounded
fresh-process observations remain next, followed only conditionally by N256 post-hoc diagnosis.
H3 quality, seal, S4/S5/S6 and Goal1 are not promoted by these tests. No new learning experiment.

현재 작업: R3-H3-CONTROLLED-PROGRESS-1.0의 A0→C/L→조건부 F/N→품질 수용.
2026-09-17. 이전 R3-CUSTOMIZE-AND-DIAGNOSE-1.0 실행 이력은 아래 보존한다.
모든 성공 표시는 구현자 확인이며 INDEPENDENT_PENDING이다.

## 현재: A0/A1/A2 검증 종료 — H3 품질 미달, 새 예산 종료

EXECUTED_THIS_RUN / IMPLEMENTER_REPORT. 현재 실행 중인 학습은 없다. 새 SMALL optimizer
호출은 C/L/F/N 각512, 총2048회로 상한에 도달했다. 합계 input4,903,380/target554,581이며
이는 네 독립 arm의 합계다. 각 F/N 모델이 물려받은 새 학습은 L512+자신512=1024회다.
옛 H3 1024와 이전 종료/resume=false는 변경하지 않았다. A1 publication
5c52c27fc56e07dd05d44e2398f5caf1a741b6df는 실제 origin/main과 일치했다.

| A2 normal greedy | 선택 부모 L | F 고정 focus | N 갱신 focus |
| --- | ---: | ---: | ---: |
| old dev full | 227/256 | 242/256 | 236/256 |
| entity / event | 246 / 253 | 251 / 251 | 252 / 245 |
| dev errors | 3 | 0 | 0 |
| CROSS full | 289/512 | 459/512 | 449/512 |
| CROSS entity / context / value / event | 321 / — / — / 507 | 501 / 490 / 502 / 498 | 504 / 481 / 491 / 500 |
| CROSS errors | 0 | 0 | 0 |
| CROSS base4/4 | — | 106/128 | 106/128 |
| ordinary | 185/336 | 176/336 | 174/336 |
| auxiliary | 53/64 | 56/64 | 55/64 |
| watch | 17/32 | 15/32 | 16/32 |

F dev0/128/256/512=227/217/242/242; N=227/244/252/236이다. N256은 기존 dev에서
full252/entity255/event256/오류0을 충족했지만 최종512에서 유지하지 못했다. 그 중간 결과를
최종 PASS로 대체하거나 별도 미등록 후보 선택에 쓰지 않았다. 원래 ordinary 하한178/336을
F/N 모두 잃었다. 양쪽 raw-development gate=false/candidate=false/resume=false,
terminal=SCREENING_BUDGET_REACHED/control=COMPLETED다. comparison의 selected_arm=F는
안전한 endpoint 중 old-full 순위일 뿐 승인/배포가 아니며 next=H3_BUDGET_CLOSED다.

N 대 F 최종 paired old-dev 획득4/상실10, base4/4 획득3/상실6이다. CROSS 획득17/상실27,
base4/4 획득9/상실9다. 고정 focus보다 갱신 focus가 최종 full-answer를 개선했다는 증거는
이번 비교에서 없다. 초기 향상과 후기 하락의 인과 원인을 LR/temperature/tokenizer 중 하나로
확정하지 않는다. F의 CROSS459/512는 A0의210/512보다249개 높지만 수용치487에는 미달한다.
양쪽 UTF-8/empty/control/length 오류0이어도 숫자·맥락·값·인용의 전체 정답 기준은 별도다.

CROSS 길이1..8의 full/64: F=51/60/60/56/62/64/54/52, N=56/59/57/53/60/60/59/45.
방향/짧은경로 full/256: F236/223, N231/218. 일반/반복/교대/인접중복 base-pattern의
view full/128: F116/107/122/114, N111/103/114/121. 1자리 패턴 중복과 rename-view의
모양 변경은 A0에 공개한 그대로이며 독립 표본 수로 부풀리지 않는다. 각 raw score에 모든
stratum, 첫 불일치 field, base, EOS, token, teacher 통계를 보존했다.

A2의 두 군은 L의 같은 native/Adam/RNG에서 시작했고 LR 시계513..1024를1e-4로 이어갔다.
새 warmup/Adam reset/decay/tokenizer/temperature 개입은 없다. anchor draw2048은 동일하다.
F focus128bases/512views/384bindings를4회 소비, N512bases/2048views/1536bindings를1회 소비했다.
view0/3은 같은 원문·binding의 서로 다른 질문이므로2048개의 독립 binding이라고 하지 않는다.
F/N의 전체 unique bases/views는2176/2560 및2560/4096이다. N은 F보다 실제 input50/target21
token 적다(F1,225,844/138,636; N1,225,794/138,615). padding/정답 절단으로 맞추지 않았다.
F는 N materialization의 균형 첫128base 부분집합이며, 두 군의 분포·stratum/view schedule은 같다.

자료 seed917260419, generator controlled-renewal-H3-v1. 새 training 사례는 새 binding/raw이며
entity 문자열은 이전 training에서 본 것을 재사용할 수 있다. 생성기의 exclusion 입력은
기존 dev+CROSS+ordinary heldout이다. 모든 heldout full entity와 옛 seal hash namespace는
제외했으며 seal 사례는 읽지 않았다. 단일 자리17개 가용 ID도 이 heldout 예약에 대한 값이다.
따라서 이를 '이전 training에서 모두 처음 보는 full entity'라고 주장하지 않는다.
F/N corpus의 각 focus view는 독립 serialized validator를 통과했다(max framed309tokens).
전체 원문/target/manifest와 update/slot/base/view/sequence 좌표는 로컬에 동결했다.

F931.79s/max RSS6,688,325,632B, N932.20s/max RSS6,686,916,608B; 별도 peak memory footprint
F7,253,449,208B/N7,259,560,440B다. A0+네 arm의 학습/평가 관측 시간은 약65.2분으로120분
이내이며 모든 command1800초/cleanup120초 이내다. 각 arm1776 generation+1776 teacher calls,
이번 A0 포함 총8272 generation이다. 모든 실제 SMALL update와 실패 사전검사 기록을 보존했다.

A2 실행 source HEAD5c52c27fc56e07dd05d44e2398f5caf1a741b6df + 관련 미커밋 소스,
digest9e5c7503651c7908a2f30514be58df689154cd6af8c70f01ac40ab5aee28c9c6.
동결 binary `artifacts/h3-controlled-20260917/a2-train-frozen`, SHA
8504af591a9eb675b0c8aae23c66cff527485bdefaca00f8c6020b1a5d83fbea.
실제 자료: `artifacts/h3-controlled-20260917/a2/corpus-F` 및 `corpus-N`.
Train SHA F=fe36d1a44a0dd1b6370777c23d588f1c928c0fd242ff186f66985ce027723cde,
N=41aa4ead3f90d7a3e869022105ebc41e00656ddd52aff879ce9eb4766929676a.
dev SHA는 기존ca5c86f91b4defe471bf1c6c2931bcc97ac0b88e084bd5506b65b0101ebccd89 그대로다.
Checkpoint/raw/trace: `artifacts/h3-controlled-20260917/a2/{F,N}/segment-00-0000/`.
F final native SHA bdc28e3f4b31ad5d600cf1c6ec0615b591deed9e26b069d9167f6a175f0a0e16,
model565a40a33ee896916a421dbee538d3f126789a67ee184039b28e7cc4224a375a.
N final native SHA734ec32aa4bfad8bce9ecf2387eae81f988cd9643b3257612f7527733d4f0bca,
model870facdd31aca3a251dfe4b111745281724517b673f2a4d38aa6267d982594ea.
Policies, coordinates, generated audit and comparison reside in the same A2 root;
console/time logs are `artifacts/h3-controlled-20260917/a2-{F,N,close}.log`.

검증: A2 직접 progress7 PASS, 추가 renewal/native2 PASS, quick-a239 executions PASS
(0 failed/ignored), 최종 fmt/check/clippy 및 release 제품/학습/checker build PASS.
처음 자료 회귀는 테스트용264-byte tokenizer의 context 한계로 실패했고, 기존801-token
교육자료 tokenizer fixture 방식을 재사용해 수정했다. 실제 SMALL tokenizer는 변경하지 않았다.
이 실패와 clippy 실패 로그도 보존했다. 이번 수치 회귀 실제 TINY updates 총82/SMALL0이며
제품 품질을 TINY 결과로 대신하지 않는다. 과거34개 수치 시험은 이번 횟수에 합산하지 않았다.
기존 untracked487개와 원본 보존 manifest를 최종 대조했다. 임시 지시문/원문/weights/DB는
publish 대상이 아니며 source·문서 신규 파일0개, 로컬 실험 artifact만 생성했다.

최종 요구사항 대조 및 판정:

- CODE/HARNESS_VERDICT=CHECKED_SCOPE_PASS: Rust-only, 기존 하네스/RunControl/저장/검사기 재사용.
  explicit fork, native Adam/LR clock/cursor parity, 누락·중단 fail-closed, serialized 자료·분모 검증 완료.
- REPORTED_CLAIMS_VERIFIED=A0 재현/원자료/노출/토큰/QK 제한 관측 완료; 가설을 원인으로 확정하지 않음.
- LR_EFFECT=조건부 관측: L 최종 old-dev+2/CROSS+30/ordinary+7, auxiliary-9 versus C; 일반 우월성 미확정.
- RENEWAL_EFFECT=최종 개선 미입증: N이 F보다 old-dev6/CROSS10/ordinary2개 낮음; 중간 향상은 보존.
- QK_CAUSE_LEVEL=OBSERVATIONAL_ONLY. RMS로±7을 강제하지 않았고 decay/iid 산식을 원인 증명에 쓰지 않음.
- RAW_H3_PASS=FAIL. UTF8_GUARD_ONLY_RESULT=NOT_RUN (선택 사항, normal greedy 유지).
- ANCHOR_PRESERVED=C/L PASS, F/N FAIL (하한178). H3_SEAL_PASS=NOT_RUN_SEALED.
- A3 새 프로세스 후보 수용=NOT_RUN_PREREQUISITE; A0 부모 재현/수치 native 재개 검증과 구분.
- H4–H8/S4/S5/S6=NOT_RUN_PREREQUISITE; GOAL1_IMPLEMENTER_READY=false,
  GOAL1_ACCEPTED=false; INDEPENDENT_REVIEW=PENDING. 추가 학습/봉인 개봉/배포 없음.

후속 판단 근거만 남긴다: 마지막 F/N은 UTF-8 오류0이지만 CROSS 전체 정답과 ordinary 보존이
부족했고 N의 중간 dev 성능이 최종에 하락했다. tokenizer/temperature 또는 다른 LR/retention
정책을 바꾸려면 이 기록을 바탕으로 새 통제 계약이 필요하며 이번에는 시행하지 않았다.

## A1 C/L 비교 종료 기록

EXECUTED_THIS_RUN. C/L 각각512 NEW updates, input1,225,871/target138,665를 소비했다.
동일 native 부모·Adam·누적 step22774·RNG와 실제512회 sample 순서/token 분모가 확인됐다.
각 군4096draws=anchor2048+focus2048, unique2560bases/4096views다. 두 독립 모델의 노출을
한 모델의 추가 epoch로 합산하지 않는다. 이번 새 SMALL 누계1024, 잔여는 조건부 F/N 각512다.

| normal greedy | A0 parent | C constant3e-5 | L ramp64→1e-4 |
| --- | ---: | ---: | ---: |
| old dev full | 208/256 | 225/256 | 227/256 |
| entity / event | 233 / 249 | 248 / 250 | 246 / 253 |
| dev errors | 2 | 2 | 3 |
| CROSS full | 210/512 | 259/512 | 289/512 |
| CROSS entity / event | 262 / 492 | 299 / 502 | 321 / 507 |
| CROSS errors | 1 | 1 | 0 |
| ordinary | 178/336 | 178/336 | 185/336 |
| auxiliary | 63/64 | 62/64 | 53/64 |
| watch | 18/32 | 18/32 | 17/32 |

C dev0/128/256/512=208/209/208/225; L=208/160/221/227. L의128회 하락 경고는256회에
해제됐다. 양쪽 SCREENING_BUDGET_REACHED/control COMPLETED, resume=false,
comparison eligible=true/candidate eligible=false다. 취소/무한값/시간초과/자료 변경 관측은 없다.
C 941.34s/max RSS6,699,188,224B; L 939.41s/max RSS6,648,381,440B. 별도 peak memory
footprint는 C7,204,084,168B/L7,208,016,328B다. 각1776 generation+1776 teacher calls.

동일512회의 L 대 C old-dev 득실18/16, base4/4 득실6/6; CROSS 득실45/15, base11/5다.
작은 old-full 차이로 일반적인 LR 우월성은 주장하지 않는다. C도 LR 변경 없이 추가 노출로
개선됐다. L의 auxiliary 하락도 보존한다. 두 군 모두 H3 수용 FAIL, 봉인 NOT_OPENED다.
A2 부모는 계약의 old-full 우선 순서로 L을 선택했다. 이는 S4/Goal1 승격이 아니다.

실행 source digest e61ead8af945c5ab44d0909244a01442cee013e0f05f3d820680f671bb451826,
source HEAD60a4f703b2824be971f267f7936876c205608d51 + 관련 미커밋 변경.
동결 binary `artifacts/h3-controlled-20260917/a1-ready-frozen`, SHA
519379ce072029a6dafe4177b5326ee4de08a59708a2db8afa6032121548a84f.
C/L checkpoint·trace·raw: `artifacts/h3-controlled-20260917/a1/{C,L}/segment-00-0000/`.
C native SHA d9bfabf9d3ae64d22fe0a9c102acab86ac4721ac33b1a1c0dc3cd0608234779e,
model506076cf584b43af22bab18819a0cd38232922bd1bbeed68ff9b4bc608fa4d6a.
L native SHA20996fa3c4ac2a0445014bf1c0fd394cefbc34c9f64f74c4dce62ea81e17a4eb,
model eb9ef9f8612a5d9e43acb202945ac195b3deac4acbf8d45b1f56c4f735bcd601,
Adam f4a37d163a1060a6aff14962401684e8e2c07920afb6934b372d07bf712bb8a2.
Pair audit: `artifacts/h3-controlled-20260917/a1/comparison.json`; logs `a1-C.log`, `a1-L.log`,
`a1-close.log` in the parent experiment directory. H3 corpus/CROSS remain the A0 files below.

검증: 직접 progress6 tests, quick-a1-ready37 executions, 기존 constant-policy ordinary
resume 거부1 test, release 제품/학습/checker build 모두 PASS. SMALL 테스트 업데이트0.
새 native LR 재개 회귀는 실행당 TINY10 updates로 직접1회/quick3회 총40회다.
초기 compile/test 실패와 두 번의 updates0 사전등록 실패 로그도 보존했다. 사전등록 실패는
진단 JSON reader의 train_loss 1ULP 반올림 차이였다. 기록 대조에 같은 JSON 읽기 규칙을
적용하고 실제 상태는 native에서 읽어 hash로 대조한다. 가중치/Adam 저장 형식은 변경하지
않았고 LR trace에는 실제 f64 bits도 기록한다. 계측/UTF8 정책은 학습 중 변경하지 않았다.

## A0 기준점 검증 완료 기록

2026-09-17, EXECUTED_THIS_RUN. 시작 HEAD=be6e2b7a7f59e444ffa25d8bae9e1af34d418d66,
추적 파일 clean, 기존 untracked487개 보존. rustc/cargo1.98.1, locked/offline,
CPU/Accelerate/F32, VECLIB_MAXIMUM_THREADS=1/RAYON_NUM_THREADS=1. 아래 이전1024 품질
실패와 예산 종료/resume=false는 그대로다. 이번 새 SMALL/TINY/scalar updates는 모두0이다.

A0는 정상 H3 dev256을 새 프로세스에서208/256으로 재현했다. 이전 raw와 ID/질문/근거/
expected/raw token/prompt/provided/excluded/finish/EOS/error/EM 차이0건이다.
같은1024 parent의 원래 ordinary는178/336, auxiliary63/64, 범주별23/68·28/68·9/68·56/68·62/64다.
ordinary watch는 같은400 결과의 고정 subset18/32다. 이후 anchor 보존 하한은max(175,178)=178이다.
새 normal CROSS-DEV는210/512, entity262/context365/value463/event492, UTF-8 1/오류1,
base4/4=38/128이다. 이 패널은 DEVELOPMENT이며 봉인/최종 시험이 아니다.
전체 H3 품질 PASS는 아직 없고 C/L·F/N·S4/S5/S6는 이 A0 기록에서 NOT_RUN이다.

자료: CROSS는128base×4views, 길이1~8×방향/경로 각32views로 한 번 동결했다.
seed917260317, SHA c27bc3e35343c3c046fa2b9c944aae0601912f086fa2a739ed7900f0a6dbf28a.
기존 train/dev 전체 entity를 제외하고 과거 seal의 hash namespace 전체를 예약해 사례는 열지 않았다.
1자리 잔여 공간은3개 ID, 같은 접두어의 재명명 쌍을 만들 수 있는 것은2개였다. 이를 서로 다른
context/value의 base들에서 재사용했다. 전체 unique entities207/bindings384이며512독립 ID가 아니다.
1자리 패턴들은 서로 겹친다. pattern은 원형 base 분류이며 ID 재명명 view에서 모양이 달라질 수 있다.
독립 검사기는 직렬화된 유일 current 원문과 질문에서 답/인용을 도출하고 split·대조·빈/모호 입력을
검사한다. 전체 field가 같아서 맞힐 수 있는 자료로 대체하지 않았다. 생성기 status와 별도로
cross-manifest의 validation.status=SERIALIZED_INPUT_VERIFIED이며 A0 실제 종료도 COMPLETED다.

보고 주장 재검산(DERIVED_FROM_LOGS): focus pool512base/2048views, 실제4096draws,
각view2회; anchor2048base/2048views, 실제4096draws, 각view2회. 총 input2,451,742/
target277,330,1024updates다. focus 한 pool의 available input584,031/target76,277과
실제 두 epoch 소비량은 구분한다. 기록된 LR 합0.03071999999999948,
decay-only product0.9996928471350713이며 과거 이력/gradient 효과를 포함한 측정은 아니다.

tokenizer801=reserved8+base256+learned537. learned 중 standalone valid383,
incomplete prefix66, invalid standalone88로 단독 UTF-8이 아닌 조각154개가 확인됐다.
base256은 valid128/incomplete51/invalid77. train/dev4352문항 gold roundtrip을 검사하고
조각별 prompt/target 빈도와 전후 token 분포를 기록했다. 임의 생성 연결의 실패와 codec 결함은 다르다.
보존된 문항별 teacher count와 자유 생성 교차표는 teacher-all-correct208/free-correct208,
teacher-not-all-correct48/free-wrong48, 불일치0이다. 평균 정확도의 거듭제곱을 원인 증거로 쓰지 않았다.

최종 UTF-8 실패53/0·53/1의 모든57/54 raw prefix를 실제 cached/full로 대조했다.
허용오차 위반0, 기록된 argmax/token/byte hash 일치, byte46 invalid_sequence를 각각 재현했다.
M은 성공1건과 반복숫자 오류1건의 선택된 실제 prefix에서 각6층×8heads의 gain 원소 min/max/RMS,
post-RoPE Q/K L2, mask 제외 finite logits/gap/entropy/top5 key를 기록했다.
예를 들어 첫 사례 layer0/head0의 q gain max1.265428/k gain max1.290802이며 보수적
상한은11.316647이다. RMS로±7 고정 상한을 만들지 않는다. 관찰한 attention은 원인 확정이 아니다.
U는 선택적 실험으로 NOT_RUN, 기본 decoding 변경0이다.

검증: 신규 직접 회귀3개와 관련 기존1개 통과; quick 총34회 테스트 실행(기존1개 중복 호출 포함),
fmt/check/clippy 통과. 관찰 hook off/on forward·gradient bit parity와 cached parity를 확인했다.
테스트 fixture 작성 중 compile 오류 및 부정확한 EM fixture 실패 로그도 보존했고 수정 후 통과했다.
실제 A0 generation1168/own-teacher1168, 추가 cache/QK forward는 별도이며 optimizer0.
wall167.74s, maximum RSS1,067,630,592B, 별도 peak memory footprint961,889,672B.
command1800s 내 COMPLETED, 중단 조건0, 기존 model/Adam/정책/corpus/봉인 bytes 보존 확인.

증거 루트=artifacts/h3-controlled-20260917. a0/{summary.json,identity-tokenizer-exposure.json,
normal-dev.json,ordinary400.json,watch32.json,final-prefix-parity.json,qk-observations.json,
cross-development.json,cross-manifest.json,cross-parent.json}, a0.log, quick-a0/summary.json,
a0-direct-tests*.log, a0-product-build.log, preserved.sha256를 보존했다.
실행 binary=a0-train-frozen SHA df45a04bf660b7b3df1db6f880f4b78d21525d7c84424ae3cc5603798c9fe652.
source digest=bf19a01b13f07f83b3d775909fd1ce9328dd71f480b75ae49e3b35270862457d.
parent physical48480b5f7fb740b7a7163d298372e957d931dcc0138dbdca1de9ef8e6ff479a7,
model f5d875bae16494a24ad7628f1628706f9e800abb1eddc03b9425ba1f55a8182d,
Adam tensor hash028e81b5d62283b83c566994e68a612f81d95eddac00030e72355e9a32650964.
SOURCE_VERIFIED/CODE_VERIFIED는 모델 품질과 별개이며 독립 수용은 PENDING이다.

## 이전: H3 1,024updates 종료 — 개선 관측, 품질 미달 / PARTIAL

2026-09-17, EXECUTED_THIS_RUN. 사용자가 승인한 마지막512회를 실행해 H3 누적1,024회에서
종료했다. 전체 답변은512회의0/256에서768회123/256,1,024회208/256(81.25%)으로
개선됐다. 따라서 이번 결과를 “변화 없음”으로 보고하지 않는다. 다만 full244/256,
entity254/256, event254/256, 생성 오류0이라는 합격 조건에는 미달이다.
STATE=NOT_RUNNING, STOP_REASON=SCREENING_BUDGET_REACHED, candidate/resume=false.
명령 exit0과 control COMPLETED는 정해진 실행이 끝났다는 뜻이며 품질 PASS가 아니다.
H4~H8/S4/S5/S6·Goal1은 완료되지 않았다. 추가 학습·자동 재개는 설정하지 않았다.

이 절은 페이블 등 다음 검토자에게 전달할 기술 인계 보고서다. 외부로 직접 전송하지 않았다.
원본·실패·중간 결과는 아래 이전 기록과 로컬 artifact에 보존한다. SOURCE_ONLY,
DERIVED_FROM_EXISTING_LOGS, EXECUTED_THIS_RUN을 구분하며 근본 원인은 UNRESOLVED다.

### 판정과 이번 변경 범위

| 항목 | 판정 | 근거와 한계 |
|---|---|---|
| H0/H1 M01~M04 | CODE_VERIFIED, 이전 공개 유지 | 기존 red4/green/정상·중단 회귀; 이번 quick에서도 관련 경계 통과 |
| H2 기준점 | VERIFIED | V1000·자료 고정, 기존400 재집계, 실제 watch32/ABA; 아래 이전 상세 기록 |
| 하네스 | QUICK_VERIFIED / MODEL_PATH_VERIFIED | quick30, 최종 native 정상2/재시작2/전이2 생성; 전체 release 수용은 미완 |
| H3 COPY | QUALITY_FAIL / 예산 종료 | dev208/256, entity233, event249, UTF-8 2; seal 미개봉 |
| H4 선택 / H5 질문 전이 / H6 전체 QA | NOT_RUN_PREREQUISITE | H3 실제 합격 부모 없음 |
| FULL_QA_DEV | 최종 artifact에서는 NOT_RUN | H2 V1000의 ordinary175/336·aux64/64를 H3 모델 점수로 옮기지 않음 |
| S4 최종 | NOT_RUN_PREREQUISITE | 새 독립200 미실행; 과거 노출 final은 개발자료로만 보존 |
| S5 공식 기억/재시작 | NOT_RUN_PREREQUISITE | 기존 기능·중간 smoke 이력은 있으나 최종 품질 수용 아님 |
| S6 / INT4 채택 | NOT_IMPLEMENTED / NO | 선행 S4/S5 미충족; 새 quantization/kernel 변경 없음 |
| TARGET_M4 | CPU/Accelerate 학습 실측 | Metal/INT4 배포 성능 수용은 NOT_RUN |
| Goal1 | IMPLEMENTER_READY=NO / ACCEPTED=NO | INDEPENDENT_REVIEW=PENDING |

이번 변경 파일은 src/quality_recovery.rs, src/check_main.rs와 기존 상태/계획/실행 문서다.
기존 recovery CLI에 무학습 skill-diagnose, 원시 평가 재집계 skill-recount, 명시적 품질 중단
갱신과 승인된 마지막 예산 정책을 붙였다. 기존 RunControl/Adam/native/evaluator를 재사용했다.
진단·계속 실행의 실제 production training CLI에 연결했으며 별도 framework는 만들지 않았다.
기존 source/단계 구현과 구분해, 이번에는 Transformer·tokenizer·loss/Adam 수식·DB·저장 포맷을
바꾸지 않았다. 새 영구 파일0, 자료 확대0, 외부 모델/teacher/모델 API 사용0이다.

### 동일 H3 dev의 실제 변화

아래 모든 전체 답변 점수는 normal greedy의 전체 문자열·인용·EOS 기준이며 오류도256에 포함한다.
0/128은 보존 로그,256/512/768/1024는 이번 실제 생성이다. 보조 필드는 같은 raw rows 재집계다.

| H3 updates | 0 | 128 | 256 | 512 | 768 | 1024 |
|---|---:|---:|---:|---:|---:|---:|
| 전체 답변 /256 | 0 | 0 | 0 | 0 | 123 | 208 |
| entity 전체 /256 | 0 | 16 | 47 | 97 | 186 | 233 |
| context /256 | — | 0 | 0 | 3 | 181 | 239 |
| value /256 | — | 98 | 128 | 127 | 216 | 243 |
| event ID /256 | 0 | 0 | 0 | 113 | 249 | 249 |
| invalid UTF-8 | 26 | 2 | 0 | 4 | 1 | 2 |
| length 종료 포함 생성 오류 | 28 | 19 | 13 | 4 | 1 | 2 |
| 기존 ordinary watch /32 | 16 | 14 | 15 | 20 | 19 | 18 |

1,024회의 ID 자리별 일치는1113/1152, 모든4변형을 맞힌 base는43/64다.
각 ID 길이1~8자리의 전체 답변은32·26·30·27·27·25·27·14 / 각각32다.
방향값116/128, 짧은 경로값92/128; 명시 대상 질문158/192, 유일한 원문만 묻는 질문50/64다.
8자리와 짧은 경로값이 상대적으로 어렵다. 다만 현재 자료는 홀수 ID 길이가 방향값,
짝수 길이가 경로값이므로 길이와 값 종류 효과를 독립적인 원인으로 단정할 수 없다.
전체 entity 문자열·base·binding은 split 간 분리했으나 개별 숫자 token은 기존 vocabulary를
재조합한 것이다. 새 언어·훈련 밖 길이·범용 한국어 지능 시험으로 확대 해석하지 않는다.

dev teacher-forced micro CE는0/128/512/768/1024에서 각각
7.199205 /1.102823 /0.455337 /0.065120 /0.020953이다.
마지막 teacher token accuracy=0.994654, 실제 마지막 학습 batch CE=0.009906이다.
teacher는 동일한 자체 모델에 정답 접두어를 넣어 측정한 진단이며 외부 teacher가 아니다.
이 값은 자유 생성의 전체 답변 정확도와 다르다. 한 token 오류로도 전체 EM은 실패하며,
오류 이후에는 정답 접두어를 제공하는 진단과 실제 생성 경로가 달라질 수 있다.
최종 train 전체 exact-match는 측정하지 않았다. 마지막 batch loss를 train 정확도로 부르거나
“train도 낮으므로 일반화 문제가 아니다”라고 이 결과만으로 판정할 수 없다.

### 남은 실패48개와 확인된 생성 경계

첫 raw byte 불일치의 위치는 entity16 / context8 / value11 / citation4 / format9다.
이는 첫 오류 기준의 배타적48개 분류이며, 전체 답변의 각 필드 오류 수와는 다르다.
예를 들어 entity 전체 오답23개에는 먼저 형식이 틀리거나 UTF-8 decoding이 실패한 행도 포함된다.
정답 접두어 진단 필드 안에 저장됐지만 이 분류 자체는 실제 free-running raw bytes와 gold를 비교한다.

최종 eval-1024.json에서 직접 확인한 사례:

| 사례 | 정답의 해당 부분 | 실제 출력의 해당 부분 | 다른 관측 |
|---|---|---|---|
| digits-2, base1/view1 | 센서91 | 센서99 | 구역99077·경로25914·event115726은 일치 |
| digits-2, base1/view3 | 구역99077 | 구역990777 | 유일한 원문 요청; entity/value/citation 일치 |
| digits-8, base7/view1 | 장비07589721 | 장비07589712 | 숫자 순서 교환; 구역·값·인용 일치 |
| base53/view0·view1 | 경로55798 | strict UTF-8 오류 | ID53/0,53/1 모두 EOS를 생성했지만 유효한 문자열이 아님 |

마지막 두 오류는 skill-H3/dev/917260311/53/0 및 /53/1이다.
raw bytes의46번 위치에서 invalid_sequence(error_len2)가 발생했고, EOS는 각각
raw index56/53에 있다. 빈 답·control·timeout은0이며 오류2건을 분모에서 제거하지 않았다.
53/1은 첫 token 차이가 index22: gold197, 실제61, 동일 접두어 teacher argmax도61이다.
이미 생성된 token423의 bytes EA B2 다음에 필요한 BD 대신 숫자 token의 byte35가 나와
UTF-8 연속 바이트 조건을 깨뜨렸다. raw 기록에는 이후 반복 문구도 남아 있다.
이는 해당 실패가 출력 끝의 단순 잘림이라는 설명과 맞지 않는다. 잘못된 부분을 lossy decode나
숫자 교정으로 수선하지 않았다. gold의 tokenizer roundtrip은 이 사례에서 true다.

학습 전128 상태의 별도 무학습 진단에서는 노출 train32와 미노출 train32가 모두0/32였다.
길이별4개씩 검사했고 각각 entity3/32·event0/32였다. 미노출 view가 이미 노출된 base와
연관될 수 있으므로 두 패널 모두 heldout은 아니다. trace128의 입력308,182/target34,911과
실제 sample/tape/index/RNG를 대조했고 일치했다.
당시 새 UTF-8 실패49/2의 기록된42개 생성 위치 전체에서 cached/full-prefix logits가
허용오차 내였고 greedy argmax와 보존 raw token도 일치했다. 잘못된 byte46까지 재현됐다.
따라서 그 사례의 cache 경로 차이는 관측되지 않았다. 이것이 모든 checkpoint/질문의
cache 또는 gradient가 무결하다는 증명은 아니다. 최종53/0·53/1의 전 prefix parity는 미실행이다.
무학습 진단 명령은 exit0/중단 조건0이지만 해당 summary의 terminal_reason은 null이다.
이 진단 summary를 정상 종료한 학습 stage receipt나 품질 승격 증거로 사용하지 않는다.

### 현재 모델·자료·학습·제품 설계

| 경계 | 실제 구현 |
|---|---|
| 모델 | NATIVE_TRPP_G1_SMALL,9,605,184 parameters, actual vocab801,6 layers×hidden384 |
| attention | Q8/KV2/head48 GQA; learned headwise QK-RMSNorm 후 absolute RoPE(theta10000) |
| block | pre-RMSNorm(eps1e-6), bias-free projections, SwiGLU FFN1024, residual, tied embedding/output |
| 문맥/KV | 첫5층 local256+마지막1층 global2048; chunk128 prefill, absolute causal/padding mask; KV에는 KV heads만 유지 |
| tokenizer | 프로젝트 train 자료로 학습한 reversible256-byte BPE;801실제 token,8reserved; 각 숫자는 독립 segment; 정규화·외부 학습 mapping 없음 |
| 학습 target | input[:-1]→labels[1:]; response+EOS만 loss, prompt/pad 제외; first target weight8 유지 |
| optimizer | 자체 AdamW beta1=.9/beta2=.999/eps1e-8/decay.01/clip1; V1000 moments와 누적 clock 유지 |
| H3 LR/batch | 실제 next nominal LR0.00004751892132506239에서 min(...,3e-5)=3e-5 고정; batch8/accumulation1/group1 |
| 실행 | CPU/Accelerate F32, Apple M4, compute threads1; Metal 학습/추론 수용 아님 |
| 생성 | 자체 native loader→자체 Transformer logits→greedy→EOS/strict UTF-8; 정답/field-only/oracle 출력 없음 |
| 저장 | inference38,432,768B와 Adam 포함 resume115,285,248B를 분리한 native binary |
| 기억 | SQLite에 원문·사건·버전·관계·FTS·요청/응답 이력; 학습 가중치를 SQLite에 저장하지 않음 |

모델은2017 원형을 그대로 구현한 구조가 아니며 위 GQA/RMSNorm/QK norm/RoPE/SwiGLU/
local-global attention을 사용한다. 이 사실만으로 최신 대형 모델 수준의 능력이나 구조의 최적성을
주장하지 않는다. 이번 품질 개선 과정에서 구조를 교체하지 않았다.
src/neural/transformer.rs가 수식/커널 경계와 KV, src/neural.rs가 tokenizer와 prompt를 소유한다.
src/training.rs가 실제 batch/loss/Adam, src/quality_recovery.rs가 제한 실행·평가·중단을 담당한다.
src/model.rs의 native worker와 app/retrieval/store 경로가 제품 입력→근거→모델→응답 commit에
연결돼 있다. 합성 정답 builder는 별도 training binary의 data 모듈에 있으며 제품 library에
포함되지 않는다. H3의 단일 근거는 처음부터 별도로 만든 과제이며 정상 QA에서 정답 근거를
oracle로 골라 남긴 결과가 아니다. S5의 새 사실 저장 후 공식 품질 시험은 아직 통과하지 않았다.

고정 H3 train4096=기존 일반 QA anchor2048+focus2048(512base×4views),
dev256/seal256은 각각64base×4views다. anchor 다섯 범주410/410/410/409/409.
seed917260311, entity-prefix는 장치/설비/센서/장비, ID1~8자리·leading-zero·반복·근접 차이.
context/event ID는 독립 생성하고, record 시각도 ID와 독립 생성한다.
4views는 원형/이름만 변경/값만 변경/유일한 원문 질문이다. source 전체와 실제 citation+EOS를
답해야 한다. 입력 구조로 gold 유일성을 별도 검사하며 원래 RF 문법 검사는 anchor에 유지한다.
최대 sequence426으로 training 한도 안에 들며 잘라서 학습하지 않았다.
512updates마다 pool별2048views를 중복 없이 노출한다. 매 batch는 서로 다른 base의
anchor4+focus4;1,024updates는 같은 동결 pool의 두 epoch다. 자료 추가·tokenizer 재학습 없음.
기존 corpus와 실패 checkpoint를 수정하지 않았고 seal은 구조 감사 외 모델 호출0이다.

native artifact는 src/neural/artifact.rs의 R3MODEL\0/version1 형식으로,
LE scalar/canonical ULEB128 metadata와 F32 tensors, 정확한 tokenizer/config/lineage를 저장한다.
JSON header가 아니다. inference는 Adam/정확한 resume 상태를 제외하고 resume는 이를 보존한다.
legacy safetensors/JSON import 경로는 별도로 존재하지만 이번 실제 산출물은 native binary다.
corpus·policy·raw evaluation 로그의 JSON은 그대로 보존한다. 이를 가중치의 JSON 저장과 혼동하지 않는다.
상수 LR 정책은 sidecar에 기록하며 native TrainConfig의 일반 schedule 필드는 schema 보존을 위해
남아 있다. skill-run은 상수 LR를 명시 적용하고 일반 trainer는 옆 정책을 확인해 잘못된 재개를
거부한다. checkpoint만 떼어 옮긴 뒤 일반 cosine resume을 해도 같다고 보장하지 않는다.

검증한 설치 도구는 rustc1.98.1(48a229cea), cargo1.98.1(797e8a9bc), stable이다.
새 설치·의존 갱신 없이 --locked --offline을 사용했다. 실제 direct dependencies는
candle-core/candle-nn0.11.0, clap4.6.7, crc32c0.6.8, ctrlc3.5.2, rusqlite0.37.0,
safetensors0.8.0, serde1.0.229, serde_json1.0.151, sha2 0.10.9, thiserror2.0.20,
tokenizers0.22.2, zstd0.13.3, dev tempfile3.27.0이다. 기존 generic crate 알고리즘과
자체 학습 mapping을 사용한다. SQLite/zstd/Accelerate 등 native 하위 의존까지 순수 Rust라는
주장은 하지 않는다. 프로젝트 제품·진단·변환·테스트 도구 소스는 Rust다.

### 실행 규칙 변경·예산·검증

128 중단 후 무학습 진단을 먼저 수행했다. 계속 진행 요청에 따라 새 output에서128→512를
명시 갱신했으나 새 UTF-8 3건과 전체 정답 증가0으로 다시 QUALITY_GUARD였다.
이후 사용자가 명시 승인한1024 상한에 한해 primary gain≥4 연장 조건을 면제하고,
중간 UTF-8 총수가 연속 두 평가에서 증가하면 중단하는 규칙을 적용했다.
새 정책은4→1→2개, 증가 streak0→1이어서 중간 중단 조건을 충족하지 않았다.
최종 오류0 조건은 그대로이므로2개 오류가 남은 모델은 합격하지 않았다.
이전128/512의 실패 receipt를 COMPLETED로 고치지 않았다. ordinary watch/resource/cancel/
nonfinite/control/empty guards, 전체1,024updates/6Minput/1.5Mtarget/60분 상한도 유지했다.

이번 추가 SMALL896(384+512), TINY0, scalar0. H3 전체 SMALL1024이며 누적 모델
step21750→22774다. 이번 입력2,143,560/target242,419, H3 전체2,451,742/277,330.
H3 총 노출은anchor4096+focus4096=8192views; 각 pool의 각 view를 정확히 두 epoch 노출했다.
이번 평가 generation/own-teacher는256/512/768/1024의1152쌍, 무학습 패널64쌍,
종료 후 native reload/model 하네스6쌍이다. cache parity의 별도 forward는 이 생성 수와 다르다.
원래128 실행의576쌍까지 포함하면 H3 정기평가는1728쌍이다.

| 실행 | 실제 추가 updates | time -l wall | maximum RSS | 별도 peak memory footprint |
|---|---:|---:|---:|---:|
| 무학습 진단 | 0 | 12.16s | 649,117,696B | 진단 log 참조 |
| 128→512 | 384 | 623.75s | 6,889,472,000B | 7,301,388,768B |
| 512→1024 | 512 | 753.14s | 6,802,964,480B | 7,231,150,536B |

마지막 work751.628283s/cleanup0.841718s, 전체 H3 누적 stage elapsed1631.647288s.
command900s/cleanup120s와 stage3600s 안에서 종료했고 time split은 필요하지 않았다.
운영 DB 접근·추가 자료 수집·모델 자동 다운로드는 하지 않았다.

직접 새 회귀3개: 노출 패널/precancel, explicit renewal 자격·실패 보존·다른 terminal 거부,
UTF-8 연속 증가/재개 상태/strict 기본 규칙. 실제3/3 통과, optimizer0.
같은 source의 quick30/30, fmt/check/clippy 통과; 필터0개를 성공으로 세지 않았다.
최종 native inference export→fresh load→정상2/재시작2/전이2 실제 생성 통과,
normal/restart 출력 동일, raw EOS/strict UTF-8/nonempty 검증; 이6개는 품질 합격 근거가 아니다.
1,024 종료물의 plain resume 및 추가 finish-copy-budget 요청은 실제 CLI exit1로 거부했다.
두 경우 generation/teacher/optimizer0, 새 run 디렉터리 생성 없이 거부했다.
현재 source digest는 학습 전 quick과 종료 후 model 하네스에서 동일하다.
test-support 없는 제품 release 빌드도 통과했다(release-final.log). 전체 release 수용 suite는
선행 품질 미달로 실행하지 않았고, S5/S6 전용 receipt verifier는 미구현 상태다.

### 재현 경로·source·checkpoint 식별

저장소 루트는 /Users/seo/Projects/Replica-v3다. 아래 경로는 이 루트 기준이며,
큰 corpus/checkpoint/raw 로그는 ignored artifacts에 있고 GitHub에 게시하지 않는다.
BASE_SHA=5879a3c6642211e78a0d19a91679babba5f8a6c1.
이번 시작 HEAD=f964a2a0308dd64852fd3d33e83e8e431373ab17; 그 위 로컬 수정으로 실행했다.
FINAL_CODE_SHA=d31e53eda459af447eed10d87617e4ee4d11ead3.
정상 push 뒤 실제 origin/main의 동일 full SHA를 확인했다(REMOTE_SHA_MATCH=YES).
이번 코드·상태 변경 diff는 f964a2a0308dd64852fd3d33e83e8e431373ab17..d31e53eda459af447eed10d87617e4ee4d11ead3이다.
이 publication 필드의 후속 commit은 보고서 전용이며 학습 source/binary 및 코드 commit과 구분한다.
독립 검토는 아직 없으며 검토 기준5879a3c6642211e78a0d19a91679babba5f8a6c1의 SOURCE_ONLY 이력을 보존한다.
실행 당시 코드 identity는 commit SHA 대신 다음 실제 source manifest/binary hash로 고정했다.

| identity | SHA256 |
|---|---|
| 마지막 실행 source digest | a20105cd8ef2160c565265f0ddb7d3d3f0b001e1678e540c651bb4dcdee85b4a |
| finish-train-frozen binary | 577374f833e618e915fc39ed05ccd9d3eb52f6f375d3d374e59a36428ddd029b |
| H3 train bytes | b246c2508c6af4141416a17fcd6b6ee222e4bca6a9a3f5331c5913f87760430a |
| H3 dev bytes | ca5c86f91b4defe471bf1c6c2931bcc97ac0b88e084bd5506b65b0101ebccd89 |
| 1024 eval raw JSON | 72ef3944d658cae8cd6d268784ea5158037a6d3aac750cf303452c92714c9d72 |
| 1024 native resume physical | 48480b5f7fb740b7a7163d298372e957d931dcc0138dbdca1de9ef8e6ff479a7 |
| 1024 inference physical | c4c0c35a631a43fe5cf0d5f4956c6bc46644efb69b818ee558468f46a38aa268 |
| 양쪽 동일 model content | f5d875bae16494a24ad7628f1628706f9e800abb1eddc03b9425ba1f55a8182d |

R=artifacts/goal1-continue-20260917, B=artifacts/harness-goal1-20260917로 읽는다.
이는 경로 약어이며 다음 명령에는 실제 경로를 썼다.

| 자료 | 실제 위치 |
|---|---|
| 원래 parent resume / inference | artifacts/s4-completion-20260917/binding-v-1000/final / binding-v1000-inference.r3m(같은 상위 폴더) |
| 원래 corpus / binding corpus | artifacts/goal1-corpus-v9 / artifacts/s4-completion-20260917/binding-corpus |
| 기준점 / H3 corpus | B/h2-baseline / B/h3-corpus(train.json,validation.json,prepared.json,manifest.json) |
| 원래128 stop | B/h3-run/segment-00-0000/{trace.jsonl,eval-0128.json,result.json,final} |
| 무학습 진단 | R/h3-diagnostic/{summary.json,exposed_train_views.json,unexposed_train_views.json} |
| 512 stop | R/h3-renewed/segment-00-0128/{trace.jsonl,eval-0256.json,eval-0512.json,result.json,final} |
| 최종1024 | R/h3-final-budget/segment-00-0512/{trace.jsonl,eval-0768.json,eval-1024.json,result.json,final} |
| 실행 정책 | R/h3-final-budget/policy.json; 이전 각 run의 policy도 보존 |
| 최종 inference | R/h3-1024-inference.r3m |
| 전체 실행 stdout/RSS | R/h3-diagnostic.log, h3-renewed.log, h3-final-budget.log |
| raw 기반 재집계 | R/recount-0128.json, recount-0256.json, recount-0512.json, recount-0768.json, recount-1024.json |
| 테스트/로드/재시작 | R/finish-policy-tests.log, quick-finish-frozen/summary.json, model-1024/summary.json, release-final.log |
| 거부 회귀 | R/resume-1024-rejected.log, renew-1024-rejected.log |

원래 V1000 resume physical bce08fe79cccdfa44ec8fbc0abc7c27f6248611cabbac4b1a92c20bfe8221cc0,
V1000 model f93483799d3cc2bfa80d55708ed758eb9f13dae6536cf1c42676dc346eabb89d.
512 stop physical de0410b709e2c22945f8de4bb655856783ecb0322610be682c65a904eef00eca,
model68abff547bb16edb4af90c9091cfbfd18c3b23089538c5827f262d22727e8719.
원본 train SHA3ac32fca9d954f98f9b3a40c785d939010ee16ddc0bb91a73c49e7269391957a,
binding train809b4559c44f3a9e46d68afd6038f9a0f434d3b1274fdc6264d6071e58344d77,
두 validation572b0d9d797feb2c31fa8713566853aff91ae6b994d736c399e8bad0cb3fb004.
이 원본 해시, 이전128/512 policy/result/checkpoint 보존 manifest를 종료 후 다시 대조했다.
기존 미추적 파일 목록487개도 시작 목록과 byte-for-byte 동일하다. 이번 변경 외 사용자 작업과
운영 DB를 staging하지 않는다. 최종1024 보존 hash 목록은 R/final-1024.sha256이다.

무학습 재집계 명령(새 output 이름 필요):

```sh
artifacts/goal1-continue-20260917/finish-train-frozen recovery skill-recount \
  --evaluation artifacts/goal1-continue-20260917/h3-final-budget/segment-00-0512/eval-1024.json \
  --output artifacts/goal1-continue-20260917/reviewer-recount-1024.json
```

실제로 실행한 마지막 학습 명령은 아래와 같다. 이는 실행 이력이며 예산 갱신/추가 재학습 지시가 아니다.
그대로 다시 실행하면 기존 output/중단 자격 검사가 거부해야 하며 실패 receipt를 지워 재시도하지 않는다.

```sh
VECLIB_MAXIMUM_THREADS=1 RAYON_NUM_THREADS=1 /usr/bin/time -l \
  artifacts/goal1-continue-20260917/finish-train-frozen recovery skill-run \
  --baseline artifacts/harness-goal1-20260917/h2-baseline \
  --corpus artifacts/harness-goal1-20260917/h3-corpus \
  --output artifacts/goal1-continue-20260917/h3-final-budget \
  --harness artifacts/goal1-continue-20260917/quick-finish-frozen/summary.json \
  --renew-from-quality-stop artifacts/goal1-continue-20260917/h3-renewed/segment-00-0128/final \
  --finish-copy-budget
```

실제 launch 경로는 당시 target/release/replica-train이었고 위 frozen 파일과 byte hash가 같다.
이후 model 하네스 빌드와 제품 release 빌드가 target binary를 바꾸므로 실행 재현에는 frozen을 쓴다.
무학습 생성 재현은 기존 replica-check model에 위 inference, binding-corpus, h3-corpus,
--split validation --limit 2 및 새 --output을 지정한다. seal은 이 명령의 입력이 아니다.

### 다음 검토자에게 남기는 질문과 한계

확인된 사실은 “고정 구조/자료/Adam/LR로 두 번째 epoch에서 새 복사 dev가 크게 개선됐지만
전체 정확성과 strict UTF-8은 미달”이다. 최초128개의 무학습 trace/cache 진단에서는 결함을
특정하지 못했다. 현재 증거로 optimizer/LR, label shift, causal mask, tokenizer, tied embedding
중 하나를 근본 원인이라고 선언하지 않는다. 기존 수치/gradient/재개 회귀 통과도 품질 보장이 아니다.

검토 우선순위는 최종48개 실패의 첫 divergence와 입력 위치를 source에 대조하는 것이다.
특히 반복/긴 ID의 자리 교환·중복, 경로값에서 UTF-8 부분 token 뒤 잘못된 숫자를 고르는 사례,
전체 생성과 gold-prefix 진단 차이를 확인할 수 있다. 최종53/0·53/1의 cached/full parity와
train 정확도는 추가 관측이 필요한 경계이며 이번에 실행했다고 보고하지 않는다.
짧은 값/길이 상관, inherited Adam과 낮은 고정 LR에서의 제한된 노출, free-running 오류 전파는
가능한 설명이지만 분리 실험을 하지 않아 인과 결론이 없다. H3가 단일 근거라서 아직 여러 근거
선택과 질문 표현 전이의 개선 여부도 알 수 없다. watch18/32만으로 원래336 전체 복구를 주장할 수 없다.
이 보고서는 추가 자료 확대·LR/seed sweep·새 구조·무기한 학습을 자동 승인하지 않는다.

## 이전: H3 128updates 중단 — 당시 QUALITY_GUARD / PARTIAL

2026-09-17, EXECUTED_THIS_RUN. H0~H2 검증 후 H3 복사 학습을 실행했으나,
128updates 평가에서 새 UTF-8 오류가 발생했다. 전체 답변은0/256으로 품질 미달이다.
전체 오류 수가 줄었어도 새 오류를 허용하지 않는 사전 중단 조건을 그대로 적용했다.
학습은 NOT_RUNNING이며 H4~H8을 시작하지 않는다. 도구 수정만 수행한 종료도,
예산 소진도, S4 완료도 아니다. 중단 artifact와 원본 V1000은 모두 보존했다.

| NODE | STATE / SCOPE | 관측 및 다음 조건 |
|---|---|---|
| H0/H1 | CODE_VERIFIED / 네 진단 경계 | M01~M04 수정 전 red4, 수정 후 직접/정상/CLI 회귀. 하네스는 아래 실행 범위만 검증 |
| H2 | VERIFIED / 원자료·실모델 기준점 | 전체 자료 감사, 기존400 재집계, 실제 watch32+ABA, H3 split 동결 |
| H3 | STOPPED_QUALITY_GUARD / 실제 SMALL128 | dev0/256, 새 UTF-8 오류1; skill 미통과, 후보·resume 부적격 |
| H4/H5/H6 | NOT_RUN_PREREQUISITE | H3 dev와 봉인 전이 PASS 없음; 전체 QA 복구·독립 최종 시험 없음 |
| H7/H8 | NOT_RUN_PREREQUISITE | S4/S5 선행 품질 미충족; 새 기억 수용·INT4 구현/측정 없음 |

BASE_SHA=5879a3c6642211e78a0d19a91679babba5f8a6c1.
H0/H1 publication=c04c695ecafab0ee65cba090fdb8c7f54dc5a348,
H2 publication=5ce3863bfc697501cf8c6d5c86e08940eac30ac6; 각각 정상 push와 remote SHA 일치 확인.
H3 실행 source는 H2 위의 로컬 구현이며 훈련 중 수정하지 않았다.
실행 source manifest digest=6daf324fb943ce39bae98dd26f93d60d423a8973bbe2b6a8a3080c4b8d40cee1,
binary SHA256=567a67900e75571306aac1425f1aaca1d87d2632fa09ed16a02f62a0073950a8.
정확한 실행 binary는 artifacts/harness-goal1-20260917/h3-train-frozen에 보존했다.
종료 후 checker의 raw/EOS 검증만 강화했다.
FINAL_CODE_SHA=719f331c88c6aff37cf463ca8cd96d084d7fbee7,
REMOTE_SHA_MATCH=YES: 정상 push 후 실제 origin/main에서 같은 full SHA를 확인했다.
이 publication 필드 갱신은 후속 보고서 전용 commit이며 코드 실행 SHA와 구별한다.
직전 전달 검토의 source 기준은5879a3c6642211e78a0d19a91679babba5f8a6c1(SOURCE_ONLY)이고,
이번 변경의 독립 검토는 아직 수행하지 않았다. 리뷰 diff base는
52aaccdab8153ea34d14db5026fad810f8dcdf26이며 이번 구현 diff base와 구별한다.

### 구현 및 직접 검증 범위

기존 recovery CLI에 skill-run을 연결하고 기존 batch/loss/Adam/native/RunControl을 재사용했다.
새 학습률 진입점은 상수 LR만 주입하며 clip/decay/moments/bias correction 수식은 유지한다.
V1000 Adam·step21750·tokenizer·가중치와 first_target_weight8을 이어받았다.
LR=min(실제 next LR0.00004751892132506239,0.00003)=0.00003을128회 모두 사용했다.
새 corpus 계보와 constant 정책은 기존 native 상태 및 sidecar에 기록한다. 일반 trainer가
이 상태를 cosine 정책으로 잘못 재개하지 못하게 거부한다. native 포맷은 바꾸지 않았다.
512update epoch마다 anchor2048/focus2048을 각각 중복 없이 노출하는 사전 tape를 사용한다.
각 batch는 서로 다른 base의 anchor4+focus4이며 index/ID/RNG/pool/token/실제 loss를 남긴다.
같은 tape·source·binary·checkpoint·clock·남은 예산으로만 TIME_BUDGET plain resume이 가능하다.
취소·품질 중단·무결성 오류는 resume/후보 수용을 허용하지 않는다.

H3 직접 회귀5개 통과: constant Adam의 독립 scalar 기준, native 저장/재개 및 precancel,
1024-entry tape의 혼합·중복·계보·재개 위치, 전체 답변/EOS/entity/citation gate,
일반 trainer의 constant 정책 오해석 거부. 기존 Adam/teacher-forcing 회귀1개도 통과했다.
실제 optimizer 합계는 TINY6/scalar6이다(실패했던 첫 시험 실행 포함). fixture의
clock18/19 또는51/52를 그만큼 수행한 학습으로 세지 않는다. H2 독립 자료 회귀1개는
optimizer0이며 원본 bytes·split·정답 유일성을 검사했다.
학습 전 quick26/fmt/check/clippy/release 통과. 종료 후 native inference export→fresh load→
정상2/재시작2/전이2의 실제 생성6회 및 teacher6회를 실행했다. raw token/EOS/nonempty/
오류 검사를 통과하고 정상/재시작 출력이 일치했다. 틀린 정답을 저장·로딩 성공과 혼동하지
않는다. 강화한 checker 회귀는 오류·빈 답·EOS·EM receipt 불일치를 거부한다.
최종 quick27/fmt/check/clippy와 test-support 없는 제품 release 빌드도 통과했다.
quick-final/summary.json의 source digest는
aab8636c95f588d917dd357913a3be768020049d6482aa727e6c47953e1a0ffb이며 model 하네스와 같다.
실패0/ignored0, 추가 optimizer0; release-final.log에 빌드 기록을 남겼다.

HARNESS_VERDICT=QUICK_VERIFIED / MODEL_PATH_VERIFIED(6 generated rows), QUALITY_NOT_GRANTED.
release 전체 테스트 및 S4/S5/S6 수용 검사는 NOT_RUN_PREREQUISITE다. release 진입점은
미완료/누락 evidence를 거부하며, S5/S6 전용 receipt 검증기는 해당 선행 단계 통과 후 구현할
부분으로 남아 있다. 현재 하네스 전체 release 수용까지 구현 완료됐다고 주장하지 않는다.
구조 검색은 휴리스틱이며 외부 의존·gold 격리에 대한 보안 증명이 아니다.

### H3 실제 결과와 예산

| 지표 | 시작0updates | 종료128updates |
|---|---:|---:|
| dev 전체 답변 EM+EOS | 0/256 | 0/256 |
| 전체 entity / event ID | 0/256 / 0/256 | 16/256 / 0/256 |
| ID 자리별 일치 | 37/1152 | 305/1152 |
| 모든 view가 맞은 base | 0/64 | 0/64 |
| invalid UTF-8 / control / empty | 26 / 0 / 0 | 2 / 0 / 0 |
| length 종료를 포함한 생성 오류 사례 | 28/256 | 19/256 |
| ordinary watch | 16/32 | 14/32 |
| teacher-forced dev micro CE / token accuracy | 7.199205 / 0.305136 | 1.102823 / 0.629560 |

1~8자리 각32개, 방향값128/짧은값128, 대상명 질문192/원문만 묻는 질문64의 전체 EM은
모두0이다. context/value, 첫 불일치, 원문/질문/인용/raw token은 각 평가 rows에 보존했다.
digit·entity·teacher 변화는 관측됐지만 전체 답변 개선이나 S4 복구로 판정하지 않는다.
새 UTF-8 오류 ID는 skill-H3/dev/917260311/49/2다. watch 감소2는 연속 감소3 중단 조건에
해당하지 않는다. 실제 STOP_REASON은 새 UTF-8 오류에 의한 QUALITY_GUARD다.
봉인256은 구조 감사만 했으며 모델 생성·점수 계산·후보 선택에 사용하지 않았다.
256/512update 평가, second epoch, 원본400 후보 평가, seal은 실행하지 않았다.

ACTUAL_UPDATES: SMALL128 / TINY6 / scalar6. SMALL cumulative21750→21878.
DATA_EXPOSURES: anchor512+focus512=1024 views, 각 pool에서 epoch 내 중복0;
dev256+watch32를0/128에서 생성해576generation/576teacher calls.
INPUT/TARGET_TOKENS=308,182/34,911(실제 SMALL 학습 소비).
ELAPSED: 작업255.289821s + cleanup0.857474s; stage256.147300s, time -l wall256.88s.
MEMORY: maximum RSS6,649,823,232B, 별도 peak memory footprint7,245,420,952B.
CPU/Accelerate/F32, Apple M4, compute threads1. 남은 예산은 품질 중단 후 소비하지 않았다.
종료 후 resume 요청은 실제 CLI에서 exit1로 거부, 추가 optimizer/model calls0.

### 로컬 재현 근거와 별도 판정

아래 경로는 artifacts/harness-goal1-20260917/ 기준이다. 큰 파일은 게시하지 않는다.

- COMMAND: h3-train-frozen recovery skill-run --baseline h2-baseline --corpus h3-corpus
  --output h3-run --harness quick-h3-frozen/summary.json (각 경로에 위 prefix 적용).
- h3-run.log; h3-run/policy.json SHA f6621c38afa34edd4be6fbcc2762711174a7656e06b1668b9873f977120609d4.
- h3-run/segment-00-0000/: trace.jsonl128행, eval-0000.json, eval-0128.json, result.json,
  step-0000/step-0128/final. final native resume115,285,248B,
  SHA83b250bd02190d6bcc065f1f2558947784994cc2a0631ea98c82bc0a2c27f077.
- h3-stopped-inference.r3m: Adam 없는 native38,432,768B,
  SHA138311f492b114cfb948e1718c58e6f5a5643a353880a23a42201b77bf7aab2b.
  resume/inference model content는 동일한 e3e5fb69a6c389b29dacdd4599245c9d181b606e1ce4cce8305147818cb51267.
- h3-final-inspect.txt, h3-export.log, h3-inference-inspect.txt, h3-resume-rejected.log,
  model-harness/summary.json 및 normal/restart/transfer.jsonl, h3-corpus/prepared.json.

H2_BASELINE/FULL_QA_DEV: 원본400 기존 로그 재집계175/336, auxiliary64/64,
각 category25/68·24/68·10/68·54/68·62/64. H3 종료 artifact에 이 점수를 붙이지 않는다.
S4_FINAL=NOT_RUN_PREREQUISITE; 과거 final은 노출 이력 때문에 재사용하지 않으며 아래
사전 동결 절차를 유지한다. S5_OFFICIAL=NOT_RUN_PREREQUISITE.
S6_IMPLEMENTATION=NOT_RUN_PREREQUISITE, INT4_ADOPTED=NO,
TARGET_M4=S6_NOT_RUN(H3 CPU 학습 실측만 있음, Metal/저비트 성능 주장은 없음).
ROOT_CAUSE_CLAIM=CONFIRMED_AT_BOUNDARY(M01~M04만); 모델 품질 실패의 근본 원인 UNRESOLVED.
GOAL1_IMPLEMENTER_READY=NO, INDEPENDENT_REVIEW=PENDING, GOAL1_ACCEPTED=NO.

FAILED_ATTEMPTS: H1 red4와 초기 checker/fixture/컴파일 실패, H3 tape의 base 중복 음성시험
실패 및 metric 합성 fixture 형식 실패를 보존했다. fixture 실패는 모델 결함 재현이 아니다.
실제 H3 품질 중단도 실패 기록이며 성공 실행으로 덮어쓰지 않는다.
PRESERVED_ARTIFACTS: 기존487 untracked, 원본 corpus/validation/checkpoint/Adam/raw/final,
사용자 로컬 지시문. original V1000 resume/export 보존 해시를 종료 후 다시 확인했다.
기존 untracked487개 전체 경로 목록이 시작 목록과 일치하고 원본/변환 train·validation
네 파일의 실제 SHA도 H2 동결 값과 일치함을 최종 확인했다.
LIMITATIONS: 제한된 합성 복사 문법이며 범용 언어 능력을 대표하지 않는다. 방향/짧은값은
ID 길이 층과 결합돼 있어 독립적인 두 요인 효과를 주장하지 않는다. 결과를 보고 자료를
다시 만들거나 같은 seal로 반복 평가하지 않았다.
NEXT_DEPENDENCY: 새 오류 원인과 복사 실패에 대한 별도 조사/실행 계약이 필요하다.
현재 계약의 중단 상태를 유지하며 자동 학습/자료 확장/다음 단계 실행은 없다.

CHANGED_FILES(전체 H0~H3): Cargo.toml, rust-toolchain.toml, AGENTS.md,
src/check_main.rs, src/data.rs, src/contrast.rs, src/training.rs, src/quality_recovery.rs,
tests/training.rs, docs/QUALITY_RECOVERY_PLAN.md, docs/EXPERIMENT_STATUS.md,
docs/RUNBOOK.md, docs/PLAN.md. NEW_FILES=AGENTS.md, src/check_main.rs 두 개.
Cargo.lock/모델 core/SQLite/archive/저장 포맷/tokenizer 변경 없음.

## 이전 단계: H0 하네스 / H1 네 경계 수정 / H2 동결 — 2026-09-17

H0/H1 publication: c04c695ecafab0ee65cba090fdb8c7f54dc5a348,
정상 push 뒤 실제 origin/main SHA 일치 확인.

H2 EXECUTED_THIS_RUN: baseline command12.52s, RSS977,059,840bytes(time -l).
V1000 native resume115,285,120B와 inference38,432,768B를 실제 로드해 같은 model content
f93483799d3cc2bfa80d55708ed758eb9f13dae6536cf1c42676dc346eabb89d 확인.
resume SHA bce08fe79cccdfa44ec8fbc0abc7c27f6248611cabbac4b1a92c20bfe8221cc0,
inference SHA f498afc20e3b3af192c55c8a69bede346735963a73c849a11a14c5131cf8c094.
step21750, 원래 Adam/RNG 보존. 실제 next LR0.00004751892132506239;
H3 고정 설계 LR0.00003. H2 종료 시 새 optimizer update0.
원본/변환 train 각20,000(ordinary16,668/aux3,332), validation400(336/64)을 전체 감사:
contradicted/unsupported/ambiguous0; 보조 과제는 감사 OUT_OF_SCOPE로 별도 표시.
기존 로그400 재집계 ordinary175/336, aux64/64; 범주25/68,24/68,10/68,54/68,62/64.
fixed ordinary watch32 실제16/32, 기존 raw/output/prompt/evidence receipt 차이0건.
새 generation33회(watch32+기존 A-B-A cache 회귀1), teacher33회, weights 변화 없음.
명령/registry/raw/감사: artifacts/harness-goal1-20260917/h2-baseline 및 h2-baseline.log.

H2 split 동결: h3-corpus, seed917260311, train4096=anchor2048+focus2048,
dev256/seal256(각64base×4). anchor 분류410/410/410/409/409, 모두 다른 기존 base.
완전한 entity 문자열 hash partition 및 기존 corpus 식별자 제외, split 간 identifier/
binding/base 교차0. 정답은 각 단일 원문 전체+실제citation이며 ID부재의 원문복사 질문도 포함.
원문/대상 변경, 값 변경, ID없는 질문의 대조를 독립 validator가 확인.
학습 최대426tokens(EOS포함), 1epoch 계획input1,225,871/target138,665(아직 소비0).
train hash b246c2508c6af4141416a17fcd6b6ee222e4bca6a9a3f5331c5913f87760430a,
dev hash ca5c86f91b4defe471bf1c6c2931bcc97ac0b88e084bd5506b65b0101ebccd89.
seal은 구조 감사만 실행, 모델 생성/점수/선택에 미사용. 원본 자료 변경 없음.
직접 구조 회귀1 통과(잘못된 정답·split 유출·추가 근거 거부, optimizer0).

최종 평가 노출: 기존 final200의 과거1/200 실패 및 rubric 보정/검토가 이미 공개 이력에
남아 있어 독립 최종 자료로 재사용하지 않는다. 현재 goal1-final-heldout 파일 SHA는
1a0a41758064ebc01d13cd4f983f26a167e2af992bb5a956c546994bf8dcda8c이고,
이번에는 hash만 확인했으며 사례 내용은 열지 않았다. 미노출 보장은 할 수 없으므로 폐기하지
않고 개발 이력으로 보존한다. H6 dev gate 통과 후 기존 independent v2 renderer와 OS seed로
새200을 단 한 번 동결하고 실제 train/dev identifier·장면 중복을 검사한다. 그 final 실패 뒤
동일 세트 재튜닝/seed 재추첨은 없다. 지금 final 생성/평가 NOT_RUN_PREREQUISITE.

BASE_SHA=5879a3c6642211e78a0d19a91679babba5f8a6c1, main,
origin=https://github.com/seoyd/replica-v3.git. 시작 local/remote SHA 일치 직접 확인.
시작 tracked clean, 기존 untracked487개와 corpus/checkpoint/raw 실패 기록 보존.
사용자가 설치한 stable rustc1.98.1(48a229cea)/cargo1.98.1(797e8a9bc)을 직접 확인하고
저장소의1.98.0 고정을 installed stable로 변경했다. 설치/의존성 업데이트 없음.
실제장치 Apple M4/25,769,803,776bytes, CPU/Accelerate/F32,
VECLIB_MAXIMUM_THREADS=1/RAYON_NUM_THREADS=1. 운영 DB 사용 없음.

NODE=H0/H1, STATE=EXECUTED_THIS_RUN, SCOPE=code/diagnostic boundaries,
ACTUAL_UPDATES: SMALL0/TINY0/scalar0; learning INPUT/TARGET_TOKENS=0/0.
현재 baseline 모델은 과거 V1000이며 이전175/336 수치는 아직 DERIVED다.
아래 실험 이력의 품질 실패는 변경하지 않는다.

- M01: empty/conflicting replacement key 거부, longest-first/비연쇄/UTF-8 소비,
  출력256KiB 상한/checked length, binding atom 문법 검사, 모든 호출자 Result 전파.
- M02: ExactEntity/ExplicitNumericField 모드, 질문·근거 전체 대상/맥락 검증,
  숫자 충돌 거부, INVALID_DIAGNOSTIC_INPUT을 모델 로드·출력 생성 전에 반환.
- M03: 관련 사고 원문 전체의 제한 문법 분류 및 사례/사건 ID별 분류 기록;
  supported+unsupported 혼합, 관련성 미상, 복수/동률 근거를 승인하지 않음.
- M04: arm terminal→평가→native checkpoint→close gate의 부적격 상태 보존.
  완료 watch32/train16, source/fixture/tape/clock/artifact/evaluation hash 결합.
  누락된 legacy 필드는 UNKNOWN_UNVERIFIED. 취소·save error·불일치 파일은 승격 불가.

직접 red4: 수정 전 실제4실패, bounded child2초로 빈 키 무진행 재현(OOM 없음).
green: checker quick의26회귀 통과(하네스3, M01~04 7, 기존RF10, malformed/close CLI2,
기존 numeric/checkpoint/prompt/memory 각1), fmt/check/clippy 및 release 빌드 통과.
정상 full-population binding 변환/원본·validation 보존과 native QA ablation CLI의 기존
통합2개도 통과(각0.92s/3.08s). 최종 명령별 기록은 quick-h1-final에 보존했다.
M04의50clock은 명시적인 테스트 상태 fixture이며50학습을 수행한 것이 아니다.
실제 TINY logits/teacher 경계 회귀와 테스트용 native checkpoint 저장·로드를 사용했다.
새 검사기 첫 실행의 clap global 인자 구성 실패, 새 checkpoint fixture의 corpus lineage
거부 및 Result 호출자 연결 컴파일 실패도 로컬 실패 로그에 보존했다.

로컬 근거: artifacts/harness-goal1-20260917/ (baseline/preserved hashes, red/green,
quick 명령별 로그). quick-h1-final은 source digest를 기록하며 실패/0-test를 숨기지 않는다.
H1/H2 당시 HARNESS_VERDICT=QUICK_VERIFIED; model/release 품질 증거 NOT_RUN.
H2=VERIFIED: V1000 resume/export content, 원본/변환 corpus 감사와400재집계/watch32 및 split 동결.
H2 종료 시 H3~H8=NOT_RUN_PREREQUISITE. S4/S5/S6=NO, GOAL1_IMPLEMENTER_READY=NO,
ROOT_CAUSE_CLAIM=CONFIRMED_AT_BOUNDARY_ONLY(과거 모델 붕괴 원인은 UNRESOLVED),
INDEPENDENT_REVIEW=PENDING, GOAL1_ACCEPTED=NO.

NEW_FILES: AGENTS.md, src/check_main.rs. Native/core/storage/tokenizer 변경 없음.
후속 실제 실행과 publication은 상단의 현재 상태에 기록한다.

## 이전: S4 V1000 종료, 품질 미달 — 2026-09-17

사용자가 S4 품질과 Goal1 전체 완료를 다시 명시했다. 기존 제한 진단 완료를 최종 완료로
대체하지 않는다. 새 V의 실행 전 계획은 QUALITY_RECOVERY_PLAN의 S4 completion V에
기록했다. 실제1000updates에서 종료했고 남은1000updates는 실행하지 않았다.
이미 닫힌 T를 단순 연장하지 않고, 동일 출발점/optimizer/sampling/LR에서 학습 자료의
질문·값 결합 하나를 바꾸는 비교다. S4/S5/S6 품질 판정은 아직 NO다.

읽기 전용 진단: 기존 T500 가중치로 원본 train 앞400을 실제 생성해 일반239/336,
보조64/64를 관측했다. 이는 전체train 정확도가 아니다. ordinary97실패 중85개는 첫 오답이
value에서 발생했다. validation QA0/2의 known-question ablation은69/136으로, 원문35/136보다
높았다. 특히 QA2는11→40/68이었다. oracle 질문 치환 점수를 후보 품질로 사용하지 않는다.
원래 validation과 최종200자료를 재작성하지 않았다.

기존 Rust data/CLI에 full-population binding-pairs 변환을 추가했다. 원본 train20000중
QA0/QA2의6668개를 같은 근거/다른 질문과 값 교환 대조로 바꾸고, 나머지13332개와
validation400의 바이트를 유지했다. 실제 출력 전체를 별도 Rust 읽기 도구로 검산했다.
신규 scene/자료 수 증가0, 원본 덮어쓰기0. 직접 CLI 회귀1개와 check/clippy/release 통과.
초기 clippy의 테스트 반복문 경고는 보존하고 수정했다. 아직 단계 완료로 commit하지 않았다.

V 출발 artifact는 artifacts/goal1-renewed-20260917/parent-p-1000/final(step20750)이다.
새 corpus는 artifacts/s4-completion-20260917/binding-corpus이며 train SHA256은
809b4559c44f3a9e46d68afd6038f9a0f434d3b1274fdc6264d6071e58344d77,
validation은 이전572b0d9d797feb2c31fa8713566853aff91ae6b994d736c399e8bad0cb3fb004다.
실행 source manifest digest는9158a9c3b69b576d2e0d6995b617ccce2434334d620c013289b57555d601159b,
trainer binary는c0766a7f02b594e450439b02edb00d9eed165e2c476be3655e595984dde4ffc6이다.
실제 초기 validation CE0.13134989는 P1000과 일치했다. batch8/group1/acc1/first-target8,
Adam/RNG 유지, LR0.00008650922011682288/warmup0/cosine2000을 저장 상태로 확인했다.
이 실행 중 source/binary는 고정한다. 큰 자료·체크포인트·원문 로그는 로컬만 보존한다.

V500 완료: step21250, 입력1,165,743/target104,272tokens, wall832.17s,
maximum RSS6,438,551,552bytes.900초 작업 상한 안에서 최종 validation과 native 저장을
완료했고 TRAIN_CONTROL의 work_error/save_error는 null이었다. 새 프로세스 validation400은
ordinary165/336, auxiliary64/64, QA0~4는23/68·20/68·7/68·53/68·62/64이며 생성/UTF-8/
빈 응답 오류0이다. 같은500updates의 T169/336보다 낮아 효과를 확인하지 못했다.
V500 physical SHA256은5a7a78d750c31939a187bf12016969fb2fd950b93d2d42a047b08496f37f857c,
weight manifest SHA256은12b2b0af9b2e5682e21bec0a9ee50550253c8547162012b875534b08d42c51b4다.
원래 P1000보다 ordinary1개 높고 사전 중단 조건에 닿지 않아 두 번째500 구간을
plain resume했다.

V1000 완료: step21750, 두 번째 구간 입력1,164,641/target104,048tokens,
wall739.23s/maximum RSS7,039,188,992bytes. 최종 validation과 native 저장을 모두 완료했다.
새 process validation400은 ordinary175/336(52.08%), auxiliary64/64이며 QA0~4는
25/68·24/68·10/68·54/68·62/64, 생성/UTF-8/빈 응답 오류0이다. T500 최고169/336보다
6문항(1.79%p) 높다. 동일1000updates의 T169/336과 비교하면 gain26/loss20이며, 질문·근거·
정답·제공ID·분모400이 모두 같은 것을 별도 Rust 집계로 검산했다. 작은 개발 점수 개선을
S4 통과나 단일 원인 규명으로 확대하지 않는다. ordinary macro는 V0.5261029412,
동일step T0.5084558824다. 특히 QA2는10/68로 계속 실패했다.

V1000 physical SHA256은bce08fe79cccdfa44ec8fbc0abc7c27f6248611cabbac4b1a92c20bfe8221cc0,
weight manifest SHA256은61b2795a13996768ae743fb83591bb6b6da4948cbcf73beb1d9d83bf866bf8a4다.
두 구간 합계 실제1000updates/입력2,330,384/target208,320tokens, wall1571.40s다.
V 내부 및 동일 원본 QA 개발 기준 후보는 V1000이나 최종 시험 적격은 아니다.
사용자의 반복 개선 실패에 대한 방향 질문 후, V1000 결과를 보기 전에 미달 시 현 구간에서
닫도록 상한을 줄였다. 계획된 상한을 다 소진하기 위한 V1500/V2000 자동 연장은 하지 않는다.
학습은 NOT_RUNNING이며 S4_QUALITY_PASS=NO, S5/S6=NOT_RUN_PREREQUISITE,
GOAL1_READY=NO, INDEPENDENT_PENDING이다. final200과 운영 모델은 변경하지 않았다.

직접 CLI 회귀1개와 fmt/check/clippy/release가 통과했다. 실행 소스29개와 원본17개,
기존 P/T checkpoints8개의 보존 해시를 재확인했다. 새 영구 파일/의존성은0이다.
실행자료는 artifacts/s4-completion-20260917에 남으며 소스·테스트·작은 상태 문서만 게시한다.
V 소스·검증 기록 commitffcf698930fec454f415738078945a0a1358da90를 정상 push했고,
origin/main의 동일 full SHA를 직접 확인했다.

### V1000 가중치 고정: 질문 표현과 근거 선택 분리

기존 보조 value-task의 oracle selector를 일반 QA0/QA2에도 적용할 수 있도록 기존
evaluate CLI에 --single-qa-record를 추가했다. 기존 질문 치환과 조합할 수 있으며,
원문/생성용 질문·근거를 구분해 기록한다. 정답 내용은 모델 입력이나 selector에 넣지 않는다.
full entity가 질문에 있으면 숫자가 같아도 다른 분류명의 원문은 선택하지 않는다.
product worker/retrieval/모델 수식·tokenizer·prompt 기본 형식에는 변경이 없다.
동일 evaluate_one/RunControl을 사용하며 oracle 결과는 후보·최종 품질로 승인하지 않는다.

| 같은 QA136의 조건 | QA0 /68 | QA2 /68 | 전체 /136 |
|---|---:|---:|---:|
| 원래 질문·두 근거 — 기존 V1000 로그 재사용 | 25 | 10 | 35 |
| 익숙한 질문 표현만 | 24 | 40 | 64 |
| 정답 근거 하나만 | 57 | 29 | 86 |
| 두 변경을 함께 적용 | 56 | 59 | 115 |

새 생성은 사전 범위408건이고 모두 완료, 생성/UTF-8/빈 응답 오류0, optimizer updates0이다.
세 command wall17.61/14.89/14.68s, 최대 RSS612,401,152bytes다. 별도 Rust 집계가 동일
weights/split/ID/원래 질문·근거/정답, 선택된 원문 bytes, 질문·근거 변경의 독립성을 검산했다.
둘을 적용하고도 남은21개 오답의 첫 차이는 entity19/format2였다. 예를 들어 원문의
장치690440을 장치690540으로 생성했다. 이미 학습된 모델이 새로운 식별자를 정확히
복사하지 못하는 사례다. 모든 오답이 이것만으로 설명된다는 주장은 하지 않는다.

관측 해석: QA0은 방해 근거 제거에, QA2는 질문 표현과 방해 근거 모두에 민감하다.
지원 근거만 남기면 길이/위치도 달라지므로 attention 또는 데이터 한 원인을 확정하지 않는다.
115/136은 oracle-assisted 진단이며 S4 점수175/336을 대체하지 않는다. 이후 학습은 자동
재개하지 않는다. 다음 방향은 기존 실패 원문을 기준으로 새 식별자의 정확한 복사, 질문에
따른 근거 선택, 질문 표현 전이를 분리한 학습 설계이며, 고정16/32 재암기나 같은 LR/step
연장은 아니다. 세 요인을 한꺼번에 바꾸거나 최종 heldout를 학습에 사용하지 않는다.

직접 CLI 회귀1개와 기존 ablation helper3개, fmt/check/clippy/release가 통과했다.
초기 CLI 회귀는 random SMALL로 실행해 오래 걸려218.34s에 소유 child에 SIGINT로 중단했다.
이 FAILED 로그는 실제 수정 전 결함 재현으로 세지 않는다. 최종 회귀는 기존 bounded
experimental config의 작은 native tensor/context2048로24회 생성해3.22s에 통과했다.
테스트는 실제 native 연산이며 FakeModel을 쓰지 않았고 optimizer 호출은0이다.
실제 품질 분리는 동일한 학습 완료 SMALL V1000에서만 실행했다.

진단 source manifest digest42b9115b17c6f5e039b5595bd80dc73c1927912bf8e8a9fe4df1f6b2d01bd012,
binary6910b44af8a079adb953174581b2db81656619bf3d00feed74aaab93f9042bf9다.
훈련 소스 digest9158a9c와 혼동하지 않는다. 소스29개 및 V artifacts2개의 보존 해시도 일치했다.

기존 native exporter로 V1000 추론용 파일을 별도로 생성·재로드했다. resume115,285,120B,
inference38,432,768B이며 후자는 training=null/Adam 없음/trained_steps21750이다.
양쪽 actual model weight hash는f93483799d3cc2bfa80d55708ed758eb9f13dae6536cf1c42676dc346eabb89d로
같다. 추론 파일 physical SHA256은f498afc20e3b3af192c55c8a69bede346735963a73c849a11a14c5131cf8c094,
경로는 artifacts/s4-completion-20260917/binding-v1000-inference.r3m이다.
native tensor-body manifest hash는 Adam 유무에 따라 다르며 실제 model weight hash와 구분한다.
diagnostic_only=true를 유지했고 운영 모델 채택/INT4 구현/S5 합격으로 표시하지 않는다.

최종 상태: 도구 구현·자료 변환/무학습 비교 검증 완료, 개발 QA의 제한된 개선 관측,
기존 U2 붕괴의 단일 원인 UNRESOLVED, S4/S5/S6/Goal1 미완, 독립 검토 PENDING.
새 영구 파일0. checkpoint/corpus/원문 로그/임시 입력은 로컬에 보존한다.

## 이전: 일반 학습 경로의 취소·종료 보완 — 2026-09-17

**일반 trainer의 중단 경계를 추가 수정했다. S4·Goal1 전체 판정은 PARTIAL이다.**
출발 SHA는32a633200950c820c22d50508b65c814bbd2837c이며 원격 일치와 tracked clean을
확인했다. 기존 RF-01~03은 진단 경로에 구현돼 있었지만, 재개한 일반 `train`은 최초/주기/
최종 validation에서 공통 제어를 쓰지 않았다. 이미 취소된 경우도 validation을 실행했고,
마지막 저장 이후 취소를 성공 종료로 놓칠 수 있었다. 기존 품질 하락과의 인과는 UNKNOWN이다.

- 구현: 기존 RunControl에 CLI의 실제 Arc 취소 flag와900초 deadline을 연결했다.
  validation teacher 전후, microbatch, optimizer 원자 경계 전후, 저장과 terminal까지
  확인한다. stop 뒤에는 추가 평가 없이 native checkpoint 보존만 수행한다.
- 중단 상태: 완료 update의 weights/Adam/sampler를 보존한다. 실제 완료한 gradient 계산의
  입력 토큰은 취소되더라도 소비 예산에 남긴다. 새 weights에서 validation을 완료하지
  못하면 이전 step의 CE를 현재 값으로 저장하지 않는다. --no-rss도 명시적으로 유지한다.
- 보고: TRAIN_CONTROL에 첫 사유/관측 조건, teacher 호출 수, 실제 실행 입력량, 최종 평가
  완료 여부, 저장 성공/실패, work/cleanup/overrun을 기록한다. 저장 후 관측된 취소는
  native 파일을 다시 쓰지 않고 최종 receipt와 실패 exit로 표시한다. 동기 tensor/fsync를
  강제 중단하는 hard deadline은 아니다. 기존 status vocabulary와 binary 형식은 불변이다.
- 범위: src/training.rs, src/train_main.rs, src/quality_recovery.rs, tests/training.rs 및
  기존 상태/계획 문서만 수정했다. 새 소스·fixture 파일·의존성·framework는 없다.

수정 전 실제 테스트4개 실패를 red.txt에 보존했다. 그중 취소 전/teacher 중/최종 종료
누락3개는 유효한 결함 재현이다. 나머지의 소비 토큰0 기대는 실제 실행 예산을 지우는
잘못된 assertion이므로 철회했다. 이 초기 판단을 원래 코드의 토큰 계수 결함으로 보고하지
않는다. 최종 accumulation 회귀는 실제 소비량 보존과 weights/Adam/RNG 불변을 함께 검사한다.

새 회귀5개와 기존 RF 회귀10개, gold independence/causal positive/tape-clock/분모4개,
실제 CLI 취소·새 process exact-resume2개, 총21distinct tests가 통과했다. 일반 학습
positive와 native 저장→재로드를 포함한다. 최종 검산 로그는 artifacts/training-stop-20260917에
보존한다. 원래 RF01/RF02 감사 parser나 데이터는 수정하지 않았으며 아래 R4의4496자료
감사/기존400 재집계/66생성 parity는 이전 실행 결과로 유지한다.

이번 SMALL optimizer updates는0이다. TINY는35updates다: 수정 전 teacher-cancel 실패
재현1 + exact-resume16×2회 + 실제 SIGINT 중단1×2회다. 그 외 새5회귀/RF/gold/집계
시험의 optimizer 호출은0이다. 재실행은 테스트 개수에 중복 합산하지 않고 실제 update에는
합산했다. 기존3700SMALL 학습을 이번 실행으로 중복 계산하지 않는다.
원본 corpus/checkpoint/실패로그와 운영 DB는 그대로 보존하고, 모델 배포를 바꾸지 않았다.

최종 fmt/check/clippy(-D warnings)/release는 Rust1.98.0, offline/locked/accelerate에서
전부 통과했다. 원본17파일과 추가 P/T8체크포인트의 SHA256이 기존 manifest와 일치했고,
487untracked 파일을 보존했다. source manifest29항목의 digest는
4983c44575e8113b9848bee1566193648e35ca2a2a8adb07c0147bcc63b58503,
실제 release trainer SHA256은
850d69d653bc796de717fe98f908511f8607780d05185e0b63c30fe4d99d1415다.
검증된 소스·테스트·두 상태 문서만 출판 대상으로 삼는다. 원자료/모델/실행 로그는 로컬에
보존한다. 새 영구 파일은0이며 이 증거 디렉터리의 로컬 로그만 추가됐다.
검토 소스·검증 기록 commit9a5d14adf9af456ab03dc3a61d5506f21ebe83aa를 origin/main에
정상 push했고 동일한 원격 full SHA를 직접 확인했다. 이 출판 확인 문구는 후속 문서 기록이다.

| 구분 | 판정 |
|---|---|
| 진단 도구 RF-01~03 / 일반 trainer stop | 구현자 검증 PASS / PASS |
| 데이터 감사 | 이전 CHECKED_BOUNDARIES_PASS의4496/4092범위 유지; 새 감사 아님 |
| 모델 성능 복구 / 원인 | NOT_ESTABLISHED / UNRESOLVED |
| S4 최종 품질 | NO — 이전 최고 개발 QA169/336; final heldout 미실행 |
| S5 새 사실·재시작 최종 수용 / S6 | NOT_RUN_PREREQUISITE / NOT_RUN_PREREQUISITE |
| Goal1 완료 / 독립 검토 | NO / PENDING |

## 이전: Goal1 재개 — 2026-09-17

**현재 실행은 종료됐다. RESULT=PARTIAL, S4·Goal1은 미완료다.** 갱신 승인 후 실제
SMALL3700updates/TINY16을 실행했다. 같은 원래 validation의 최고 일반 QA는
169/336(50.30%)이며, 보조 복사64/64를 더한233/400을 S4 품질로 대체하지 않는다.
T의 마지막 세 평가가 최고 후보를 넘지 못해 사전 중단 조건으로 닫았다. 아래 소스 수정과
실제 학습·native 저장·재로드·생성은 검증됐지만 품질95%/분류별90%는 통과하지 못했다.
새 final200·새 사실 기반 S5·S6는 선행 품질 조건 미충족으로 미실행이다. 학습 권한을
다시 미승인으로 바꾸는 것이 아니라, 실패한 제한 실행을 자동 연장하지 않는 상태다.

사용자가 추가 학습을 포함한 S4·Goal1 계속 진행을 명시했다. 출발 SHA는
`52e8b88cd38e88b4ba563a27d47cfe0bc9543f83`이며 tracked dirty 없음/remote 일치를 확인했다.
아래 RF-01~03의0update 예산은 종료된 라운드의 사실로 유지한다. 새 단계의 제한 C/L
LR-policy 비교 계획은 QUALITY_RECOVERY_PLAN.md에 사전 기록했다. 관련7tests/clippy/release
통과 후 source manifest `e4afa6f3e4d4ed6660a64cfe1c5ff28c4b804054686462249fcc5c39b899112a`로
실행했다. C는200updates에서 QUALITY_GUARD로 종료, checkpoint_saved=true다.
watch0/10/25/50/100/150/200은18/17/16/17/16/11/11 of32였다. C20 가중치는 보존된
원래 U2+20과 정확히 일치했다. C의 입력489342/target45146tokens, work256.176s다.
L은250updates를 완료했다. watch0/10/25/50/100/150/200/250은18/15/15/18/17/15/14/15
of32였다. 입력613756/target57272tokens, work315.723s, SCREENING_BUDGET_REACHED,
checkpoint_saved=true다. 신규 SMALL 합계450updates이며 numeric/TINY optimizer는0이다.
S4_QUALITY_PASS=NO, GOAL1_READY=NO를 유지한다.
실행 증거는 artifacts/goal1-renewed-20260917에 별도로 보존하며 원본을 덮어쓰지 않는다.

### C/L 제한 비교 종료 — 검증된 음성 결과

| 관측 | C | L |
|---|---:|---:|
| 실행 updates / 종료 model step | 200 / 19950 | 250 / 20000 |
| 동일200updates watch / 사전 train16 | 11/32 / 8/16 | 14/32 / 11/16 |
| 동일200updates entity / context / value | 14/26 / 21/26 / 15/26 | 19/26 / 21/26 / 16/26 |
| 동일200updates citation exact | 25/32 | 26/32 |
| process wall / maximum RSS bytes | 257.51s / 6333513728 | 316.71s / 6620741632 |
| 최종 reason | QUALITY_GUARD | SCREENING_BUDGET_REACHED |

LR 정책을 바꾼 군에서200update의 일부 오류가 줄었지만 parent18/32를 회복하지 못했다.
원래 하락의 단일 원인을 확정하거나 L을 품질 통과 모델로 선정하지 않는다.
C는 중단돼250 결과가 없으며 L250과 새로운 C250을 비교했다고 쓰지 않는다.
이전 C/W 또는 QA32/contrast16을 반복하지 않았다. 새 final heldout도 실행하지 않았다.

Rust `recovery schedule-report`가 fixture/초기상태/config 차이, 전체 계획 tape,
각 실제 trace의 LR/model·optimizer·schedule clock/입력·target 수/사례 순서를 검산했다.
기존 평가 row의 정답·오류로 watch/train16 및 entity/context/value/citation을 재집계했다.
`schedule-comparison.json` 및 `schedule-comparison-detailed.json` 모두 읽기 전용 결과다.
이 재집계의 새 모델 호출/optimizer update는0이다. train16은 전체 train 점수가 아니다.

실제 학습 소스 digest는 위 e4afa6f다. 이후 기존 row의 component/train16 집계만 보강한
최종 전달 digest는 `687e1ecdcd8665f97c6bd3db97a1dbaa675ab8a600915a1882a4754411046181`이다.
직접8tests와 fmt/check/clippy/release가 통과했고 원본17개 파일의 해시가 계속 일치했다.
소스는 기존 quality_recovery.rs만 변경했고 checkpoint schema/기본 trainer 수식은 유지했다.
결과는 C/L 진단 완료이며 S4 완료가 아니다. 다음 조치는 별도 사전 계획으로 기록한다.
C/L 소스·기록 commit `79ac7866635960ced9a4619be98a2382438c498d`를 정상 push하고
origin/main의 같은 full SHA를 직접 확인했다.

B도250update 상한에서 종료했다. L 설정에서 sample_group_size8→1만 바꾸고 이전 L을
control로 재사용했다. watch0/10/25/50/100/150/200/250은18/15/16/15/14/16/17/15 of32다.
L/B 마지막 watch는 둘 다15/32이고 고정 train16은11/16 대10/16이었다. B를 연장하지 않는다.
최종 B의 entity21/26/context21/26/value18/26/citation26/32, 새 생성 오류0이다.
입력613827/target58199tokens, wall395.09s/max RSS6660702208bytes, 정상 상한 종료 및
checkpoint 저장을 확인했다. 이번 재개 후 총 SMALL updates700, numeric/TINY optimizer0이다.

B 실제 source digest `6661a8b2b7e3212b01d84cb979f530cb579d74b388a84aa891540bb5e6b518f6`,
binary `ce7939cf9f34f00e5163515ab757c921f570848e7b5218dd10f308c6a8cbf2ef`다. 각 policy의
RNG/tape/ID/step/LR를 실제 원문과 재검산하고, 시작 watch raw 출력 일치와 동일 train16을
확인했다. 실제 draw와 노출은 서로 달라 동일 tape/동일 multiset 효과로 확대하지 않는다.
직접6tests/fmt/check/clippy/release와 실제 group-comparison 검산이 통과했다.
판정은 GROUP_POLICY_COMPARISON_VERIFIED / RECOVERY_NOT_ESTABLISHED이며 S4는 미완이다.
다음 검토에서 원래 일반 QA corpus의 원문이 v8부터 유지된 사실을 확인했으므로,
parent의 최근379update만으로 모든 QA의 학습 노출을 계산하거나 노출 부족을 원인으로
단정하지 않는다. 새 후속 학습은 원래 corpus의 분포를 유지하는 별도 계획으로 고정한다.
B 소스·기록 commit `0dfc918271eb83d293662cbc751bb52fd0b52f10` 정상 push 및 원격 일치를
확인했다. 다음 P는 원래 parent/corpus의 최대1000updates를250단위로 검증하며 진행한다.
P의 원래 parent/corpus validation400 재생성은219/400, ordinary155/336, copy64/64였고
분류별13/68·22/68·5/68·54/68·61/64, 생성/UTF-8 오류0으로 기존 결과와 일치했다.
실제 wall44.83s/max RSS974307328bytes,400개 완료 및 terminal COMPLETED를 확인했다.
P 첫250update는365.96s/input577458/target50665tokens에서 정상 구간 종료·저장됐다.
이후 validation은226/400, ordinary162/336, copy64/64, 분류별16/68·24/68·5/68·54/68·63/64,
생성/UTF-8 오류0이었다(wall44.71s). 두 번째250도 plain exact resume으로 완료했다.
구간 입력587192/target52835tokens, wall367.40s/max RSS6494011392bytes였고, 누적500의
validation은214/400, ordinary150/336, copy64/64,14/68·21/68·4/68·48/68·63/64,
생성/UTF-8 오류0이었다(wall44.22s). 세 번째250은 입력582618/target51722tokens,
wall360.23s/max RSS6930546688bytes에서 완료·저장됐다. 누적750의 validation은226/400,
ordinary162/336, copy64/64,19/68·22/68·6/68·52/68·63/64, 생성/UTF-8 오류0이었다
(wall44.25s). 이 시점에는 전체 QA와 category macro가250과 같아250 후보를 유지했다.
마지막250은 input576646/target50497tokens, wall368.28s/max RSS6503333888bytes,
step20750/BUDGET_REACHED에서 정상 저장됐다. 마지막 validation은228/400,
ordinary164/336, copy64/64,21/68·22/68·6/68·52/68·63/64, 생성/UTF-8 오류0이었다
(wall45.23s). category macro와 ordinary EM이 개선돼 사전 규칙상P1000을 개발 후보로
선택했다. 독립 최종 평가 후보 자격은 없으며 운영 DB/모델을 교체하지 않았다.
명시 LR/warmup 옵션과 Adam/RNG 보존·native exact
resume 회귀, 기존 엄격 집계 회귀, fmt/clippy/release가 통과했다. resume 테스트의 TINY
optimizer16회는 SMALL 학습과 별도로 계상한다. P 실행 source manifest는
`848df720b46efbec53627a8074f2549b2509e9bd96ca735a766920498650665b`, 실행 binary는
`de55611f45af58fd2c5ecd0ce3bff6d82093e354ae53f55c8d12c34c40571c06`이다.

### P 단계 종료 — 실제 학습 완료, S4 품질 미달

P 총1000SMALL updates/input2323914/target205719tokens, 학습 OS wall1461.87s,
최대 RSS6930546688bytes였다. 매 구간900s/전체3600s, input20M/RSS16GiB 상한 안에서
완료했다. baseline 포함5×400=2000개의 실제 generation/teacher 평가를 새 프로세스로
완료했고 모두 full panel/COMPLETED/오류0이었다. 평가 wall 합계223.24s다.
P250/500/750/1000은 각각162/150/162/164 of336이었다. 최종164/336은 parent155/336보다
9개 많지만 전체95%·분류별90% 기준을 통과하지 못했다. qa-2는6/68에 머물렀다.
teacher-forced token accuracy97.01%를 전체 답변 정확도48.81%로 바꿔 보고하지 않는다.

이번 재개 누계: SMALL1700(C200+L250+B250+P1000), TINY optimizer16.
실제 직접 회귀는9distinct tests(P 관련2포함), fmt/check/clippy/release PASS다.
P 실행 source manifest 전항목과 binary가 그대로이며 원본17파일 해시도 모두 일치했다.
신규 소스 파일/의존/구조/저장 schema/자료 확장 없음. 새 native checkpoint/raw log는
로컬 artifacts/goal1-renewed-20260917 아래 보존한다. 모델 품질 실패를 도구 실패나
Goal1 성공으로 바꾸지 않는다. REPAIR_VERIFIED=YES, DATA_AUDIT는 이전 제한 범위 유지,
RECOVERY_ESTABLISHED=NO, S4_QUALITY_PASS=NO, S5/S6 미완, GOAL1_READY=NO다.

| P 구간 | artifact physical SHA256 | manifest weights SHA256 |
|---|---|---|
| 250 | 1d9c1170e287c0ae383f6885a3d6158c3e8bb79774eddbd1bd7aadbd36a6df83 | 18064a0e0b6e8f031300aa6fe7fc31761be1f1ff788290e8d25b997a3a92f770 |
| 500 | da2e6cb23e9b033e879da879bd202c0d579874ec29147d763a35b316d84a83de | c04c35ed93a544a9c71f49826b0ad6e68139e6ff7eab12422db37bade7fef392 |
| 750 | 35472867a39207877e007cb203fd0f54197424b01ed187787f349e790aa4cae4 | 7d99a382615221b1854b03cfb4aa47ec7e59db4498d6e717b80f0610d3113c17 |
| 1000 | e99fbfbfda47fdf0c29112c8b9a30b127476b7ee5219aeecdbab71b374832b40 | d19919907db369f20f25e1f9885837bf27b7372dd21a528468e0c55cca06b3a9 |

각 artifact는 artifacts/goal1-renewed-20260917/parent-p-N/final이다. 실제 corpus는
artifacts/goal1-corpus-v9, 각 원문 generation ledger는 parent-validation-N.jsonl,
학습 로그는 parent-p-N.txt, native inspect는 parent-N-manifest.txt다. 기존 parent,
실패U2 및 원본 corpus를 덮어쓰지 않았다. P의 검증된 소스·기록을
`aa99fbfd7913276f1cd4e9e56f72dc215fa541a1`로 정상 push하고 실제 원격 SHA 일치를 확인했다.
다음 T는 P의 관측된 개선을 출발점으로 하는 별도 최대2000update 학습이다. 실행 source와
binary는 P와 동일하다. 독립 최종200은 NOT_RUN_NOT_ELIGIBLE.

T 첫500update는 input1165810/target104294tokens, wall690.79s/max RSS6952665088bytes,
step21250에서 정상 구간 종료·저장됐다. validation은233/400, ordinary169/336,
copy64/64,24/68·22/68·11/68·49/68·63/64, 생성/UTF-8 오류0, wall43.67s였다.
시작P1000의164/336보다5개 늘었지만 S4는 여전히 미달이다. 실제 loss0.12035898이며
두 번째500update 구간도 plain exact resume으로 완료했다. input1164716/target104076tokens,
wall696.89s/max RSS6674530304bytes, step21750에서 저장됐고 validation은233/400,
ordinary169/336, copy64/64,19/68·24/68·12/68·52/68·62/64, 생성/UTF-8 오류0이었다
(wall43.64s). EM은 같지만 category macro가 소폭 낮아T500 후보를 유지한다. 비개선1회,
세 번째500도 input1158552/target101901tokens, wall690.79s/max RSS6543327232bytes,
step22250에서 완료·저장했다. validation은231/400, ordinary167/336, copy64/64,
17/68·24/68·11/68·56/68·59/64, 생성/UTF-8 오류0, wall43.37s였다.
마지막500은 input1156825/target102655tokens, wall694.82s/max RSS6769770496bytes,
step22750/BUDGET_REACHED에서 저장했다. validation은230/400, ordinary166/336,
copy64/64,16/68·25/68·10/68·56/68·59/64, 생성/UTF-8 오류0, wall43.39s였다.
최고 후보 이후 비개선3회가 되어 T를 닫았다. 신규 누계SMALL3700/TINY16이다.

### T 단계 종료와 최종 대조

T는 총2000updates/input4645903/target412926tokens, 학습 OS wall2773.29s,
최대 RSS6952665088bytes였다. 각 구간900s/전체3600s, input20M/RSS16GiB 한도를
지켰다. 새 full400 평가4회=1600generation을 모두 완료했고 평가 wall 합계174.07s,
생성/UTF-8/빈응답 오류0이었다. P와 합쳐 새 full validation 생성은9×400=3600회다.
P/T 실행 중 Rust 소스와 binary는 동일했고, 최종29항목 source manifest와 원본17파일
hash가 전부 일치했다. 실제 train/evaluate 프로세스도 더 이상 실행 중이지 않다.

| T 누적 updates | 일반 QA / 보조 | qa-0 / qa-1 / qa-2 / qa-3 / qa-4 | 판정 |
|---|---|---|---|
| 0 (P1000 재사용) | 164/336 / 64/64 | 21/68 / 22/68 / 6/68 / 52/68 / 63/64 | 기준 |
| 500 | 169/336 / 64/64 | 24/68 / 22/68 / 11/68 / 49/68 / 63/64 | 개발 후보 선택 |
| 1000 | 169/336 / 64/64 | 19/68 / 24/68 / 12/68 / 52/68 / 62/64 | category macro 하락 |
| 1500 | 167/336 / 64/64 | 17/68 / 24/68 / 11/68 / 56/68 / 59/64 | 비개선2회 |
| 2000 | 166/336 / 64/64 | 16/68 / 25/68 / 10/68 / 56/68 / 59/64 | 비개선3회 + update cap |

최고 개발 후보는 artifacts/goal1-renewed-20260917/transfer-t-500/final, model step21250이다.
원래 parent에서 선택된 계보의 추가 update는1500(P1000+T500)이고, 실제 모든 분기에서
실행한 신규3700update와 구분한다. 원래 parent155/336 대비14개 개선은 같은 개발 split의
관측일 뿐이다. 이전 U2 하락의 단일 원인 확정, 새로운 heldout 전이, S4 성공이 아니다.
최고 후보의 qa-2는11/68로 여전히 낮고, 최종 checkpoint가 최고 후보도 아니다.
운영 DB/모델을 자동 교체하지 않았으며 모든 원본과 실패 후보를 그대로 보존했다.

| T artifact | physical SHA256 | manifest weights SHA256 |
|---|---|---|
| 500/final | 146dae22aa2792480a189d2404a5040f2fced81c97da8c8520435bca6433969b | dc0bf31c43156917a85ef0540b7fe97eb385aa1a06a7e7d1bdc8cc7ae2be9046 |
| 1000/final | b6ab47d467add76ea685e52b4ad92e9f8cbacf18d5ea92060d5e01070ce1a383 | bcabf1ebb40adc7f90e441027d79a8470aac58a1aacaab8342c26f85f8c8d4e4 |
| 1500/final | 834e0cc08b1d4bc8271e5078586deb50f0ab6e920ca8dabdaebcb3ae0d1a7ae8 | c5260e38bdec238e9239b6bef73333990161f4252f07dc7f68fa4418b0b13e98 |
| 2000/final | 1205aeac9b9ad12b1b392ae996960dac3095a5d93cdcbb016c55e931c100437a | e0b6e62f274cbec353615b9ed3b3e236f6ef949d295eb930de697d2597c3b86e |

위 경로의 공통 prefix는 artifacts/goal1-renewed-20260917/transfer-t-다. 실제 corpus는
artifacts/goal1-corpus-v9, raw 학습 로그는 transfer-t-N.txt, raw 생성 ledger는
transfer-validation-N.jsonl, native inspect 기록은 transfer-N-manifest.txt다.
소스 commit은 aa99fbfd7913276f1cd4e9e56f72dc215fa541a1이며 T 중 소스 변경은0이다.
실행 source/binary digest는 위 P와 동일하다. 큰 artifact/corpus/raw는 로컬 보존하고,
마지막 publication은 상태 문서만 갱신한다. 독립 검토는 PENDING이다.

| 요구/판정 | 최종 상태 |
|---|---|
| 갱신된 학습 권한과 한도 | 사용자 승인 반영; 개별 계획 상한/중단 기준 준수 |
| RF-01~03 도구 수정 | 이전 검증·출판 유지; 관련 취소/clock/집계 회귀 통과 |
| 데이터 감사 | 이전 ordinary4092/4496 제한 범위 유지; 전체20,000 의미 감사로 확대 주장하지 않음 |
| Rust-only/자체 모델/외부 teacher·API·답변 하드코딩 금지 | 유지; 잠긴 Rust1.98/CPU·Accelerate/F32 실행 |
| 최소 수정/재사용/실제 경로 | 기존 CLI·trainer·native artifact·평가 재사용; 새 소스 파일/의존 없음 |
| 저장·구조·원본 보존 | format/topology/tokenizer/DB 불변; 원본17파일 및 source29항목 hash 일치 |
| 검증 | 직접9distinct tests, fmt/check/clippy/release; P/T 실제 학습→binary 저장→새 process 생성 |
| 개발 QA 개선 | YES, 같은 split155→169/336; 보조와 오류를 별도 계상 |
| 성능 하락 원인 / 복구 확정 | UNRESOLVED / NOT_ESTABLISHED |
| S4 최종 품질 / 새 facts S5 / S6 | NO / NOT_RUN_PREREQUISITE / NOT_RUN_PREREQUISITE |
| Goal1 완료 / 독립 승인 | NO / PENDING |
| 현재 상태 | NOT_RUNNING; T THREE_CONSECUTIVE_NON_IMPROVEMENTS, native terminal BUDGET_REACHED |

같은 실패 실행을 자동 연장하지 않는다. 남은 문제는 질문·근거 선택 일반화이며 원인 미확정이다.
그 다음 조치는 추가 update 수를 임의로 늘리기 전에, 기존 실패 row에서 일반화 경계를
구분할 수 있는 한 가지 검증 가능한 가설을 정하는 일이다. 아직 새 실험은 등록·실행하지 않았다.

## 이전: 세 진단 경계 수정 — 2026-09-17

**RF-01~03 진단 경계의 수정과 제한 검증을 완료했다. 모델 품질은 회복되지 않았다.**
동일 원자료 감사에서 지원 범위 내 label 모순은 발견하지 못했다. 기존400 재집계와
parent/실패 모델의 제한 replay는 이전 기록과 일치했다. 신규 SMALL 학습은0update다.
기준 `3b671bf695ae86511273c4139d43d75bd976e490`의 독립 source 판정은
CODE_VERDICT=FAIL로 보존하며, 아래 새 수정의 PASS는 구현자 검증이다.

| 판정 키 | 이번 실행 결과 |
|---|---|
| RESULT / REPAIR_IMPLEMENTER_VERDICT | VERIFIED_REPAIR / PASS |
| INDEPENDENT_REVIEW | PENDING — 새 소스의 독립 검토 미실행 |
| RF01_SPLIT_BINDING | PASS |
| RF02_TARGET_SEMANTICS | PASS — 현재 합성 QA의 제한 문법 |
| RF03_EVAL_STOP | PASS — 협력적 취소/deadline; hard kill 아님 |
| DATA_AUDIT | CHECKED_BOUNDARIES_PASS — ordinary4092, 보조404 의미 제외 |
| SEMANTIC_SCOPE | scanned4496 / validated4092 / contradicted0 / unsupported0 / ambiguous0 / out_of_scope404 |
| HISTORICAL_LEDGER_RECOUNT | PASS — 기존400 및 C/W trace 읽기·재집계 |
| NATIVE_REPLAY_PARITY | PASS — parent32+ABA1, 실패32+ABA1; 총66회 |
| SOURCE_SHA_AND_WORKTREE_DIGEST | 검토 소스e4e08fc0b8e43071437b40082fb098d4df815878; 아래 실행/전달 digest 구분 |
| SMALL_OPTIMIZER_UPDATES_THIS_ROUND | 0 |
| NUMERIC_TEST_OPTIMIZER_UPDATES | 0 — scalar Adam/accumulation 및 TINY 학습·resume 시험 미실행 |
| MODEL_WEIGHTS_CHANGED | NO — 보존 원본17파일의 시작/종료 해시 일치 |
| ROOT_CAUSE_EVIDENCE | UNRESOLVED — 검산 범위 내 새 자료 결함 미확인 |
| REGRESSION_RECOVERED / TRANSFER_IMPROVED | NO / NOT_TESTED |
| S4_QUALITY_PASS / GOAL1_READY | NO / NO |
| NEXT_TRAINING_AUTHORIZED | NO |

### 수정 경계와 실행 회귀

`src/quality_recovery.rs`의 기존 replay/scan/평가/arm 종료 경로와
`src/training.rs`의 직접 평가 호출자를 수정했다. synthetic fixture는 기존 테스트 모듈
안에 두었다. 새 소스·dependency·영구 문서·checkpoint schema는 추가하지 않았다.
Rust1.98.0/Cargo.lock/Accelerate를 유지했고 외부 모델·teacher·API를 사용하지 않았다.
여기서 teacher 진단은 동일 로컬 모델의 gold-prefix forward이며 외부 teacher가 아니다.

RF-01은 all에서 loader가 검증한 동일 소유 validation과 manifest를 보존해 frozen hash를
대조한다. 불일치는 모델 로드·호출·output 생성 전에 거부한다. watch/failures는 frozen의
episode만 사용한다. `source_validation_hash`는 유래이며 `actual_split_hash`는 all의 검증된
현재 split이다. 실제 panel 내용은 `evaluated_cases_hash`로 구분한다. 이 값은 모든 필드를
포함한 순서 있는 `Vec<Episode>`의 compact `serde_json::to_vec` 바이트 SHA256이다.
`ordered_ids_hash`는 순서만 식별한다. 옛 `split_hash`는 panel 자체가 아닌 유래400의
hash였으며 옛 로그는 수정하지 않았다. 알 수 없는 panel도 오류로 거부한다.

RF-02는 request의 질문·원문·status·시각으로 의무를 유도하고, target 문장을 별도로
파싱해 대상/위치/값/인용/시간순서/인과 불확실성을 검사한다. 근거 없음, 원인 미확인,
chronology, 구조화 QA의 긍정/부정 사례를 구분한다. 올바른 ID만 인용하거나 불확실성
문구 뒤에 확정 원인을 덧붙여도 통과하지 못한다. 비지원 문법과 모호한 근거는 각각
UNSUPPORTED_FORM/AMBIGUOUS_EVIDENCE이며 승인하지 않는다. 기존 copy/* 보조 범위를
늘리지 않았다. 제품 retrieval/ask/worker에는 이 checker나 gold를 연결하지 않았다.

RF-03은 command의 동일 flag/deadline을 생성·teacher·watch/train panel·replay·close·마지막
평가·저장·종료에 전달한다. generation timeout은 원래 한도와 남은 command budget 중
작은 값이며 적용값을 별도 receipt에 남긴다. 1ms 미만 잔여에는 새 작업을 시작하지 않는다.
중단 후 teacher/다음 case/ABA/update는 실행하지 않고, 마지막 일관 상태 저장만 허용한다.
planned/attempted/completed/not-run/interrupted ID와 실제 사유를 기록하며 부분 결과는
comparison/candidate 부적격이다. 오류 생성은 완료 panel의 EM 분모에 그대로 남긴다.
마지막 terminal 확인 뒤 발생한 cancel은 이미 닫힌 판정을 소급 변경하지 않는다.

동시 조건의 우선순위는 cancel→deadline→RSS 관측 실패→RSS 초과다. 첫 사유를 래치하고
나머지 관측 조건도 보존한다. save 실패는 `checkpoint_saved=false`와 `save_error`로 별도
기록한다. 기존 native status vocabulary를 유지하므로 저장 직후 stop이 관측되면
`checkpoint_save_status_reason`과 최종 sidecar의 terminal reason이 다를 수 있다.
판정에는 최종 sidecar를 사용한다. work/cleanup/overrun 시간을 분리한다. 동기 tensor 연산,
파일 sync, 기존 worker IPC는 중간 강제 중단을 보장하지 않아 deadline을 넘을 수 있다.

| 회귀 | 수정 전 실제 실패 | 수정 후 실제 통과 |
|---|---|---|
| RF-01 | 동일 ID/current 변경을 옛 replay가 실행함: rf01-red.txt | hash/count/manifest 불일치, output 보존, 0호출, panel 내용, 정상 all: 신규3test |
| RF-02 | no-evidence 확정 원인을 scan이 승인함: rf02-red.txt | 실제 scan→audit positive/negative/unknown 및 기존 chronology positive: 신규2test+기존1test |
| RF-03 | pre-cancel인데 옛 평가가 생성/teacher를 수행함: rf03-red.txt | 실제 TINY shared flag, 생성 중단, 시간 경계, 최종 평가/저장/terminal: 신규5test |

RF-03 red는 budget 입력이 없던 기준 함수에 test-only 연결용 wrapper를 두어
**수정하지 않은 기준 평가 본문**을 호출한 실행 실패다. 컴파일 실패를 red로 세지 않았다.
RF-01 초기 exact filter의0test 실행도 성공·실패 근거에서 제외했다. 실패/WIP 로그는
모두 로컬 보존한다. 실제 TINY는 무작위 초기화된 모델의 forward/generate만 사용했다.
최종 n=50 상태는 평가/종료 helper에 직접 진입시켜 검사했으며50회 학습을 돌리지 않았다.

직접 테스트는 신규10개, 기존 gold independence/support/causal/분모/byte/control/tape-clock
7개, decode receipt1개로 **서로 다른18개 전부 통과**했다. 관련 변경 후 재실행은 이 수에
중복 합산하지 않는다. `cargo fmt --all -- --check`, offline/locked
`cargo check --all-targets --features accelerate`,
`cargo clippy --all-targets --features accelerate -- -D warnings`,
`cargo build --release --features accelerate --bins`도 통과했다. 전체 suite는 실행하지 않았다.

### 동일 자료 감사와 과거 결과 재집계

아래 감사는 tokenization/정적 자료 검산이며 모델 forward·gradient·generation은0이다.

| 범위 | scanned | ordinary / validated | contradicted | unsupported | ambiguous | out_of_scope |
|---|---:|---:|---:|---:|---:|---:|
| U2 train 전체 | 2048 | 2048 / 2048 | 0 | 0 | 0 | 0 |
| Parent train 앞2048 | 2048 | 1708 / 1708 | 0 | 0 | 0 | 340 |
| Validation400 | 400 | 336 / 336 | 0 | 0 | 0 | 64 |
| 합계 | 4496 | 4092 / 4092 | 0 | 0 | 0 | 404 |

prefix 불일치/동일 token prompt의 다른 target/검산한 구조화 entity의 split 교집합도0이다.
이 결과는 parent 전체20000 및 범위 밖404의 의미 정확성을 보증하지 않는다.
첫 실제 감사에서는 기존의 “이동 값” 질문형과 점검 완료·사고 자료 부재의 원문형을
checker가 해석하지 못했다. 초기 U2 unsupported88/ambiguous24, parent184/40,
validation0/8을 원본 ID·이유와 함께 `data-audit.json`에 보존했다. 실제 source/원문을
확인해 이 두 제한 형식과 독립 positive 회귀만 보완했다. 원문/정답/제외 범위는 불변이다.
현재 판정은 `data-audit-final.json`이다. 초기 미완료 감사의 stderr가 INTEGRITY_FAIL로
분류되던 것도 AUDIT_INCOMPLETE로 분리했으며 초기 로그를 고쳐 쓰지 않았다.

새 Rust `recovery recount`는 기존400 raw hash, fixture/split/checkpoint binding,
ID 중복·누락, 질문·근거·target, C/W policy/tape/실제 trace/세 clock/LR를 검증했다.
row의 actual/expected/error/finish로 점수를 다시 계산했다. C/W는 기존50+50 trace를
읽었으며 이번 학습 횟수로 합산하지 않는다.

| 읽어서 재집계한 기존 결과 | Parent | U2+250 | 기존 C50 / W50 |
|---|---:|---:|---:|
| 일반 QA | 155/336 | 56/336 | 해당 없음 |
| 보조 | 16/64 | 0/64 | 해당 없음 |
| 전체 EM+EOS | 171/400 | 56/400 | 해당 없음 |
| watch32 | 해당 없음 | 해당 없음 | 17/32 / 17/32 |

이400개를 새로 생성하지 않았다. 과거30개 UTF-8 오류는 raw receipt가 없으므로 해당
finish를 UNKNOWN으로 유지한다. frozen label 기준 점수라는 의미도 그대로 기록했다.
이번 checker의 지원 범위에서는 추가 label 모순이 확인되지 않았다.

### 제한 무학습 native replay와 식별자

기존 watch32를 parent/실패 각각 한 번 재생하고 각1회 ABA만 추가했다. reference에는
이전 `artifacts/quality-recovery-20260917/parent-watch.jsonl`과 `failed-watch.jsonl`을
지정했다. optional failures 추가, C/W replay, 전체400 생성은 실행하지 않았다.

| 이번 실제 생성 | Parent | U2+250 실패 |
|---|---:|---:|
| generation calls (32+ABA1) | 33 | 33 |
| 전체 panel EM+EOS | 18/32 | 5/32 |
| invalid UTF-8 / empty / first-EOS | 0 / 0 / 0 | 4 / 1 / 1 |
| planned / attempted / completed / not-run | 32 / 32 / 32 / 0 | 32 / 32 / 32 / 0 |
| 기존 raw tokens/text/error/prompt/provided/excluded 차이 | 0 | 0 |
| ABA | 동일 | 동일 |
| command wall / OS maximum RSS | 4.73s / 769654784 bytes | 6.24s / 787087360 bytes |

합계66generation/10.97s다. command work≤900s, 합계≤1800s, 생성≤70 한도 안에서
완료했다. CPU/Accelerate/F32, M4, 두 thread 환경변수1로 이전 실행과 맞췄다.
생성이 오류로 끝난4건도 분모32에서 제외하지 않았다. 부분 완료나 timeout은 없었다.

기준 source HEAD는 `3b671bf695ae86511273c4139d43d75bd976e490`이다. 실행 source digest는
정렬한 `src/tests/examples`의 Rust 파일과 Cargo.toml/Cargo.lock별 SHA256 manifest의
SHA256이며 git commit과 구별한다.

| 식별 대상 | SHA256 |
|---|---|
| 66회 replay 실행 source manifest | cb930b1a7a69010eeea032aa6e0d7e9db25a0faebdb7607e40984657393220ed |
| 해당 실행 binary | 3a62e251031772dc722454c3e57ba51f25d0c2dcf58f1be754df5ad8b2855178 |
| 최종 전달 source manifest | aa978d4d9b74c4ad7491001c74df8bafd3ff05a81b10fdcacb53f155363fafee |
| 최종 release binary | 9ce7198c5b8bcb8512558798ed14020b204dafa0ec3fef914a80f924b5f35778 |
| 원래 frozen fixture | 48a356ca63c8d24fdd2da57ecc25391847127f2f814bc8f90102b994a87e74b4 |
| Parent physical checkpoint | 1bcae73d7f2f7f501acc43e3958d66781b22d139533b12d6e369624171356849 |
| 실패 physical checkpoint | 34c2ef0630b6afe1df1b4b901a8679d4461edb041e4885b812fe91935eb1ad2c |
| U2 train content | 7c19a05354ddbed179b7ccb2f9dc667ad71a98d2f4bd426a9e74e0ffd8046b70 |
| U2 validation content | f8d18fe3f6bd2b14045218139eafabec41486420779295fe1273a12c8d697864 |

제한 replay 이후 정상 terminal의 무중단 사유를 null 대신 COMPLETED로 명시하고,
회귀에서 실제0호출/정상 종료 assertion을 보강했다. 생성 계산·입력 변경은 없다.
최종 소스에서 직접 회귀와 fmt/check/clippy/release를 완료했으며 SMALL replay를 반복하지
않았다. 기존66회의 원시 로그에는 실행 당시 null과 complete=true가 그대로 남아 있다.
이전 실행 binary와 최종 전달 binary를 동일하다고 보고하지 않는다.

주요 로컬 위치는 다음과 같다. 모든 경로의 기준은 저장소 root다.

| 내용 | 보존 위치 |
|---|---|
| Parent native | artifacts/goal1-resume-20260917/u2-parent-resume.r3m |
| 실패 native | artifacts/goal1-resume-20260917/u2-to-20000/final |
| Parent corpus / U2 corpus | artifacts/goal1-corpus-v9 / artifacts/goal1-resume-20260917/u2-corpus-fixed |
| 기존400 parent raw | artifacts/goal1-resume-20260917/u2-start-validation.jsonl |
| 기존400 실패 raw | artifacts/goal1-resume-20260917/u2-20000-validation.jsonl |
| 기존 frozen | artifacts/quality-recovery-20260917/frozen.json |
| 기존 C / W | artifacts/quality-recovery-20260917/arm-c-fixed / artifacts/quality-recovery-20260917/arm-w |
| 이번 실행 증거 root | artifacts/diagnostic-repair-20260917 |
| 감사 / 재집계 | 해당 root의 data-audit-final.json / historical-recount.json |
| 실제 replay / 자원·command receipt | 해당 root의 parent-watch.jsonl / failed-watch.jsonl 및 각각 .txt |
| 회귀 / 빌드 | 해당 root의 rf01-red.txt, rf02-red.txt, rf03-red.txt, final-repair-tests.txt, existing-direct-tests.txt, decode-receipt-test.txt, final-*.txt |
| 소스·원본 보존 증거 | 해당 root의 native-parity-source.sha256, delivery-source.sha256, originals.sha256, originals-end-check.txt, initial-untracked.txt |

### 종료와 다음 가설의 경계

R0~R4와 R5의 직접 검증을 완료했다. 기존 미추적487파일과 raw/private 증거는 로컬에
보존한다. 공개 범위는 수정한 Rust2파일과 기존 상태/계획 문서2파일뿐이다.
모델/Adam/DB/corpus/raw log는 staging하지 않았다. 새 독립 source 검토는 PENDING이다.

검토 source commit은 `e4e08fc0b8e43071437b40082fb098d4df815878`이다.
`git push origin main` 성공 후 `git ls-remote origin refs/heads/main`으로 같은 full SHA를
직접 확인했다. 기준 대비 diff는
`git diff 3b671bf695ae86511273c4139d43d75bd976e490 e4e08fc0b8e43071437b40082fb098d4df815878`
이다. 이 확인 기록은 후속 docs-only commit에 담는다. 그 최종 문서 commit SHA와 최종
remote HEAD 일치는 전달 리포트에 별도 기록하며 source SHA와 혼동하지 않는다.

다음에 제안하는 가설은 **Adam moments를 유지한 schedule restart의 LR 영향** 하나다.
이번에는 NOT_RUN이며 재학습 권한도 없다. 별도 승인 시 같은 일반 QA parent의 weights,
moments, optimizer clock, 자료·tape·batch·first-target weight를 맞춘 C/L 두 군에서
schedule/LR 정책만 비교하고, 현상이 생겼던 warmup100 이후~250update 범위를 별도 예산으로
정해야 한다. nonfinite/시간·메모리·token 한도는 즉시 중단하고, parent 대비 watch EM4개
이상 감소 또는 새 오류2개 이상이 연속 두 평가면 중단한다. 이는 자동 연장이나 기존50회
재실행 승인이 아니다. C/W 음성 결과는 LR·batch·250update 원인을 배제하지 않는다.

## 이전 제한 품질 회복 진단 종료 — 2026-09-17

**원래 U2+250의 성능 하락은 재현됐지만 학습 원인은 미확정이다.** 평가 시 UTF-8
decode 실패로 생성 ID와 종료 정보를 잃던 결함을 수정했다. 이것은 관측 결함이며
가중치나 답변 품질을 회복시키는 수정은 아니다. 같은 일반 QA parent에서 첫 target
가중치만 대조한 C50/W50은 모두 watch17/32로 종료했다. 추가 학습은 실행하지 않는다.

| 판정 키 | 결과 |
|---|---|
| RESULT | COMPLETE_DIAGNOSIS — 정해진 범위의 음성 결과로 종료; 품질 목표 기준 PARTIAL |
| EXECUTION | Q4 CLOSED_NEGATIVE / NOT_RUNNING |
| CODE_VERDICT | CHECKED_BOUNDARIES_PASS; 생성 receipt 보존 결함 수정 |
| ARTIFACT_REPLAY | 원본 parent/+20/+250 정확히 식별; 고정 panel의 기존 출력 차이0 |
| REGRESSION_REPRODUCED | 원래+250 저장 모델 YES; 새 C50에서는 REGRESSION_NOT_REPRODUCED_WITHIN_BUDGET |
| ROOT_CAUSE_EVIDENCE | 관측 결함 CONFIRMED_IMPLEMENTATION_DEFECT; H-FIRST NOT_SUPPORTED_WITHIN_BUDGET; 학습 하락 UNRESOLVED |
| REGRESSION_RECOVERED | NO — 후보 없음; parent로의 rollback도 실행하지 않음 |
| TRANSFER_IMPROVED | NOT_ESTABLISHED; 새 전이 시험 미실행 |
| S4_QUALITY_PASS | NO; final200 NOT_RUN_NOT_ELIGIBLE |
| S5_OFFICIAL_PASS / S6_INT4_PASS | NO / NO |
| GOAL1_READY / INDEPENDENT_REVIEW | NO / PENDING |
| TOTAL_OPTIMIZER_UPDATES_THIS_ROUND | SMALL100; 별도 exact-resume TINY12 및 scalar Adam15; 전체127 |
| DATA_EXPOSURE_AND_SPLIT | U2 train2048 중 각 arm384개 view 실제 노출; watch32/실패16은 DEVELOPMENT, 새 final 아님 |

### 실제 비교점 및 재현 범위

시작 source HEAD는 `9fb5059696553b41d9a3190856888a929d0931be`이며 tracked dirty는
없었다. Rust/Cargo1.98.0, macOS27 arm64 Apple M4/24GiB, CPU/Accelerate F32,
VECLIB_MAXIMUM_THREADS=1/RAYON_NUM_THREADS=1로 실행했다. 모델은 기존
9,605,184-parameter SMALL/자체801 byte BPE다. 외부 모델·teacher·모델 API를 사용하지 않았다.

| artifact | 로컬 명시 경로 | 누적 model/optimizer step | schedule step |
|---|---|---:|---:|
| GENERAL_QA_PARENT | artifacts/goal1-resume-20260917/u2-parent-resume.r3m | 19750/19750 | 379 |
| U2_POLICY_START | artifacts/goal1-resume-20260917/u2-probe/start | 19750/19750 | 0 |
| U2_AFTER_20 | artifacts/goal1-resume-20260917/u2-probe/final | 19770/19770 | 20 |
| U2_AFTER_250 | artifacts/goal1-resume-20260917/u2-to-20000/final | 20000/20000 | 250 |

아래는 기존400개 raw outputs를 새 ledger로 **재집계**한 수치다. 전체400을 새로 생성한
결과가 아니다. 실제 checkpoint/validation hash, 질문·근거·target을 검산하고, 새 프로세스의
watch32와 실패선정16 자유 생성은 과거 출력과 모두 일치했다. 오류를 분모에서 빼지 않았다.
현재400은 QA336+보조64이며, 과거 다른 보조64를 포함한219/400과 비교하지 않는다.

| 같은 DEVELOPMENT400 | Parent | U2+250 실패 | 회복 후보 |
|---|---:|---:|---|
| 일반 QA | 155/336 | 56/336 | 선정 없음 / NOT_RUN |
| 보조 | 16/64 | 0/64 | NOT_RUN |
| 전체 full-answer EM+EOS | 171/400 | 56/400 | NOT_RUN |
| teacher-forced micro token CE | 0.270056009 | 0.331130866 | NOT_RUN |
| teacher-forced case macro CE | 1.070589435 | 1.266940633 | NOT_RUN |
| teacher-forced token accuracy | 95.2622% | 92.3495% | NOT_RUN |
| invalid UTF-8 / 빈 응답 | 0 / 0 | 30 / 12 | NOT_RUN |
| 기록된 first-EOS / control / timeout | 0 / 0 / 0 | 12 / 0 / 0 | NOT_RUN |
| 전체400 record/value/citation 분리 점수 | 과거 로그에 없음; 아래 panel 참조 | 과거 로그에 없음; 아래 panel 참조 | NOT_RUN |
| 새 전이 점수 | 이번 미실행 | 이번 미실행 | NOT_RUN |

과거 decode 오류30건은 generation receipt가 없으므로 전체30건의 종료 원인을
추정해서 채우지 않았다. 새 실패선정 panel의 UTF-8 오류4건은 모두 중간 invalid_sequence,
error_len=1, valid_up_to=75/22/253/78이었다. length 종료2건과 EOS 종료2건이다.
incomplete_tail과 invalid_sequence 구분 자체는 별도 독립 byte fixture로 검증했다.

| 새 자유 생성: 고정 watch32 | Parent | U2+250 | C50 | W50 |
|---|---:|---:|---:|---:|
| full-answer EM+EOS | 18/32 | 5/32 | 17/32 | 17/32 |
| token CE (micro) | 0.093050201 | 0.180677370 | 0.074909880 | 0.074871167 |
| teacher-forced token accuracy | 97.3399% | 93.8916% | 97.3399% | 97.3399% |
| entity | 21/26 | 10/26 | 21/26 | 21/26 |
| context | 23/26 | 18/26 | 23/26 | 23/26 |
| value | 16/26 | 19/26 | 19/26 | 19/26 |
| gold citation IDs exact | 28/32 | 17/32 | 25/32 | 25/32 |
| citation IDs가 제공 근거 안에 있음 | 31/32 | 21/28 decoded | 29/32 | 29/32 |
| invalid UTF-8 / empty / first-EOS | 0/0/0 | 4/1/1 | 0/0/0 | 0/0/0 |

26은 구조화된 entity/context/value 필드가 있는 답변 수다. 실패의 인용 포함 여부는
decode 실패4건을 UNKNOWN으로 따로 남기며, 전체 EM 분모는 계속32다. 제공된 ID를
인용하는 것만으로 올바른 support 선택은 아니다. 빈 인용도 포함 검사에서 참일 수 있다.
위 분리 점수는 full-answer 실패를 대체하지 않는다. 별도 실패선정16은 parent12/16,
실패0/16이며, 실패를 골라 만든 표본이라 전체 품질 추정에 사용하지 않는다.

### 확인한 경계와 최소 수정

`src/neural/transformer.rs`는 기존 generate loop에서 선택한 token을 관측하는 callback만
추가했다. 제품 generate는 같은 loop를 no-op observer로 호출한다. topology, 연산,
greedy/EOS 정책은 그대로다. `src/training.rs`의 기존 평가가 공유 receipt 함수를 호출해
decode 실패에도 IDs/bytes/finish를 보존한다. Adam도 같은 수식에서 update를 관측한다.
`src/train_main.rs`에 recovery CLI를 연결했다. 새 `src/quality_recovery.rs`는 훈련 전용으로
registry/ledger/audit/제한 실행을 묶으며 제품 library는 이 모듈이나 gold에 의존하지 않는다.

U2전2048과 parent앞2048의 실제 prepared token prefix를 대조했다. parent의 일반 QA1708은
질문/원문/status/time만 사용하는 독립 support 검사도 수행했고, 보조340은 의미 검사
범위 밖으로 명시했다. 동일 token prompt/다른 target, prefix 불일치, 검산 대상 의미 모순,
전체 train/validation의 구조화 entity 교집합은0이었다. final 자료는 만들거나 읽지 않았다.
초기 validator가 사고 단일 인용과 근거 있는 chronology 인용을 혼동해 U2 64/parent120건을
표시했다. 실제 원문과 시각을 확인하고 독립 회귀로 validator 경계만 고쳤다. 자료를
제외하거나 정답/원문을 수정하지 않았으며 초기 실패 로그도 보존했다.

| 자료 고유 수 | U2전2048 | Parent앞2048 |
|---|---:|---:|
| base scene | 256 | 512 |
| 원문 질문 / 숫자 run을 #로 접은 질문형 | 384 / 142 | 428 / 189 |
| 구조화 값 / 근거 값 multiset(빈 목록 포함) | 8 / 58 | 8 / 82 |
| 근거 ID 순서 / 최종 token prompt | 1121 / 1894 | 1970 / 1997 |

숫자 정규화는 분포 집계만을 위한 정의다. 제품 입력을 바꾸지 않는다. U2의1776 prompt가
window256보다 길었고 최대371이었다. 따라서 모든 입력이 짧아 window를 넘지 않는다고
설명할 수 없다. 다만 parent/실패 각각34개 actual full/cache/chunk127·128·129,
alone/batch, causal255·256·257 검사에서 사전 abs1e-4+rel1e-3 위반은0이었다.
최대 absolute 차이는 parent2.38419e-5/실패1.52588e-5였다.

실제8행 batch의224 target에 shift/mask/EOS를 검산하고68개 gradient가 finite임을 확인했다.
embedding/final norm 선택2좌표 central difference는 사전 step0.002/abs0.002+rel0.05 내였다.
독립 f64 Adam3step은 m/v, bias correction, decay, epsilon, global clipping을 검사했다.
서로 다른 target 길이 accumulation도 통과했다. clip은 target 수로 가중 평균한 gradient에
한 번 적용한다. 첫1/5/20 update의 attention/QK norm/tied embedding/FFN별 gradient/update/
weight 비율을 저장했다. 이 제한 검사로 모든 학습 결함을 배제했다고 주장하지 않는다.

LR는 artifact 계산값 DERIVED_CONFIG와 기존 실제 trace를 구분해 대조했다. parent 마지막
0.0000865092201은 실제 출력0.00008651과 맞았다. U2는 moments와 optimizer19750을
유지하며 schedule0부터 재시작했다. +1/+5/+20/+50/+100의 계산값은 각각
0.000003/0.000015/0.000060/0.000150/0.000300이며 실제 trace와 일치했다.
schedule0은 실행 update가 아니다. 이 사실만으로 LR가 하락 원인이라고 결론내리지 않는다.

### 한 요인 실험과 재시작 검증

사전 선택한 C와 W만 실행했다. W는 first-target weight8→1 하나를 바꿨다. 같은 parent의
weights/tokenizer/Adam, group8/micro8/acc1, RNG/tape, LR/clock을 유지했다.
C20의 model hash는 보존된 원래 U2+20과 **정확히 일치**했다. 종료 검산에서도50개
실제 trace의 사례 순서/토큰 수/RNG/LR/세 clock이 C/W에서 일치했다.

| 관측 | C | W |
|---|---:|---:|
| 신규 updates / 최종 누적 step | 50 / 19800 | 50 / 19800 |
| watch0→10→25→50 | 18→17→16→17 /32 | 18→17→16→17 /32 |
| 사전 고정 train16의 EM0→10→25→50 | 9→11→11→10 /16 | 9→11→11→10 /16 |
| input / supervised target tokens | 122380 / 11300 | 122380 / 11300 |
| 실제 추출 / 고유 view | 400 / 384 | 400 / 384 |
| 프로세스 wall time / OS max RSS | 74.63s / 5778407424 bytes | 75.50s / 6104776704 bytes |
| watch 새 generation 오류/empty | 0 / 0 | 0 / 0 |

train16은 tape 첫 고유16개를 실행 전에 고정했다. 평가0에서는 이번 segment 노출0,
10/25/50에서는 각각1회였고 과거 parent 노출은 이 숫자에 합치지 않는다. 전체train 정확도로
확대하지 않는다. 최초 C 시도는0update에서 native status 검증 오류로 끝났고 복구 가능한
원본/실패 로그를 보존했다. 상세 이유를 sidecar에 두고 기존 native status를 사용하도록
고친 뒤 위 C50을 실행했다. 저장 포맷 변경은 없었다.

두50update 실행은 각각 parent19750에서 분기했다. 신규 총100을19750→19850 연속 실행으로
표시하지 않는다. 고정 panel에서4개 이상 악화/새 오류2개 이상이 연속 두 평가라는 중단
조건은 발생하지 않았다. 50update에서250update 붕괴가 재현되지 않았으므로 예산을 늘리지
않았다. W 효과도 확인되지 않아 confirmation 두 회, 후보 전체400, 새 전이, final200은
실행하지 않았다. 새 heldout를 소모하거나 기준을 낮추지 않았다.

C/W native checkpoint를 새 프로세스에서 각각32개 다시 생성해 raw IDs/text/error/prompt
digest가 step50 기록과 같았다. A→B→A도 같았다. 기존 제품 worker로 C/W 성공 응답2건과
실패 artifact의 length+UTF8/first-EOS 빈 응답2건을 대조했다. 제품은 원래대로 길이 초과를
decode보다 먼저 거부했고 빈 응답도 거부했다. legacy/native tokenizer의 semantic ID와
이64+2건 generated ID→byte mapping은 일치했다. 제품 DB나 기본 checkpoint 포인터는
교체하지 않았다. JSON/JSONL은 로컬 진단 로그이며 모델은 기존 native binary다.

### 실행 근거, 검증 및 남은 한계

모든 새 원시 증거는 로컬 `artifacts/quality-recovery-20260917/`에 보존한다. 공개되는
이 문서는 익명 집계이며 corpus/원시 prompt·출력/모델/Adam/운영DB는 게시하지 않는다.
주요 CLI는 `replica-train recovery freeze`, `replay`, `audit`, `numeric`, `arm`, `close`다.
각 path와 source digest는 고정 fixture/policy/header에 기록됐으며 새 출력은 create_new다.

| 실행/증거 | 결과 |
|---|---|
| frozen.json, parent/failed-watch.jsonl, parent/failed-failures.jsonl | 원본 registry/400집계/48개씩 replay, 이전 출력 차이0 |
| data-audit-delivery.json | 최종 분포 포함 U2전2048/parent앞2048 검사 PASS |
| parent-numeric.json / failed-numeric.json | 각34개 수치 대조 및 실제 batch gradient/finite difference PASS |
| arm-c-fixed, arm-w의 policy/trace/eval/result 및 native final | C50/W50 실학습, 저장/종료, 표본·clock 대조 PASS |
| closure-final/summary.json 및 C-fresh/W-fresh.jsonl | 동일64개 fresh생성, 제품worker4개, tokenizer mapping PASS |
| recovery_* 직접 unit9개 | ledger, UTF8, control, 독립 support/chronology, Adam, accumulation, 한 요인, gold 비유입 PASS |
| decode_failure_preserves_generated_tokens_and_eos_receipt | 수정 전 실제 FAIL, 수정 후 PASS |
| first_target_objective_matches_scalar_ce_and_gradients_without_mask_leakage | 독립 목적함수/gradient PASS |
| native_local_global_mask_and_greedy_generation_boundaries | 해당 generation 경계 PASS |
| native_training_resume_is_identical_in_fresh_processes | TINY6 대 3+3, weights/moments/RNG/loss/logits 일치 PASS |
| cargo fmt --all -- --check; check/clippy --all-targets; build --release --bins | offline/locked/features accelerate, clippy -D warnings, 모두 PASS |

직접 테스트는 서로 다른13개다. 전체 suite/DB/backup 성능 검사를 반복하지 않았다.
테스트의TINY12updates와 scalar Adam3step×5회=15는 실제SMALL100과 구분한다.
데이터 집계 보완 후 read-only audit만 다시 실행했으며 학습을 다시 하지 않았다.

기준 physical parent hash는
`1bcae73d7f2f7f501acc43e3958d66781b22d139533b12d6e369624171356849`,
실패 hash는 `34c2ef0630b6afe1df1b4b901a8679d4461edb041e4885b812fe91935eb1ad2c`다.
순수 tensor content digest는 parent
`1533f7fa235837810e883e92412e1fdf4e4ef81b78dd80391c86d95717a276c8`, 실패
`c3b56a58da8ee44d31124db92903b97879af07e53e1e97ecc9651c654a96f9d2`다.
기존 호환 필드 model_content_hash는 architecture를 함께 묶는 weight_hash이며,
순수 tensor digest인 manifest.model_content_digest와 같은 값으로 취급하지 않는다.
tokenizer semantic hash는 `652718e4864c2f06af3a2c67dac0a5174d5e2feb1387667173902da9a8a295d4`다.

학습 당시 source manifest digest는
`6737851fceaed2b61b0e0fe8a06f094edea3fc56aa9f6d61d51f632a6d4c681e`, 실행 binary는
`9c707fae42e97d9158dbfb88ed13fce0ec9bd445835daae4734c083a11a293f8`다.
종료 재생의 source는 `b326397506d80177f7544288cf4e7ed61d49ff1a703b84e522b653f536b969e5`,
binary는 `37a7d1b98e8f71201d0d09a0d792b699f02885c0a88e190df526260a83761b55`다.
최종 gold 비유입 회귀와 분포 집계를 더한 게시 source digest는
`fe87a7de13b457b888ffc7f68d810767dedcd99a7e2e0ba5325403d080d26a34`, binary는
`4901d4e14969798c4bef801eaea143e5f8cadff44246ad962cdec05a3729ab26`다.
학습 도중 소스를 바꾸지 않았고, 게시 소스로 학습했다고 과거 run을 재표기하지 않는다.

다음에 필요한 단일 실험은 같은 parent/moments/tape에서 LR만 고정 continuation으로
대조하는 C/L이다. warmup100을 지나는 별도 한정 범위를 먼저 정해야 하며 이번에는
실행하지 않는다. 현재의 결과만으로 LR, 데이터 부족, 용량 부족, tokenizer, forgetting
중 하나를 근본 원인으로 단정할 수 없다. [진단 계획과 최종 대조](QUALITY_RECOVERY_PLAN.md)에
검사 범위와 조건부 미실행을 기록했다. 이전 수준 복구·새 전이·S4·Goal1은 모두 남아 있다.

## 이전 중단 시점 보고 — 2026-09-17, 아래는 재개 전 이력

RESULT: PARTIAL. EXECUTION: PAUSED_BY_USER / NOT_RUNNING.
MODEL_QUALITY_PASS: NO. S4_QUALITY: FAIL. GOAL1_READY: NO.
사용자의 중단·보고 요청 뒤에는 학습/추론 프로세스가 없음을 확인했다. 이후 GitHub 게시
요청으로 기존 구현과 실제 관측 결과를 보존한다. 이 게시를 학습 재개나 단계 합격으로
해석하지 않는다. 새 학습·모델 변경·추가 과적합 진단은 실행하지 않았다.

실제 gradient 학습과 작은 고정 자료 암기는 확인됐다. 그러나 새 값/근거에 대한 전이와
일반 QA는 품질 기준을 충족하지 못했고, 최근 U2의 실제 답변 정확도는 크게 악화됐다.
유용한 일반 지능이나 서비스 가능한 기억 응답 모델을 완성했다고 말할 근거가 없다.

### 구현돼 있는 것과 남은 것

| 영역 | 구현/관측 사실 | 한계 |
|---|---|---|
| 모델 | Rust 자체 decoder, 9,605,184 parameters; 6 layers/hidden384, Q8/KV2 GQA, pre-RMSNorm, QK-RMSNorm, RoPE, SwiGLU, bias-free, tied embedding/output, local5+global1 | 현대적인 구성 요소를 사용했다는 사실이 답변 품질을 보장하지 않음; 기본 수식은 이번 U2에서 변경하지 않음 |
| tokenizer | 학습 split만으로 만든 byte BPE; 현재801 vocabulary; 자체 byte framing/숫자 분절/native 저장, 범용 tokenizers crate의 BPE 학습 알고리즘 재사용 | 외부 학습 tokenizer는 사용하지 않음; 목표 상한4096과 실제801을 구별 |
| trainer/KV | 실제 backward/AdamW, target mask/shift/accumulation, causal/local KV, RNG와 optimizer 저장·복원 | 관련 회귀가 통과했어도 모든 학습 오류를 배제한 것은 아님 |
| 저장 | 실제 native binary 기본 저장/로드/생성, inference와 resume 분리, 새 process의 정확한 resume 검증 | F32이며 INT4는 미구현; 저장 성공은 품질 성공과 별개 |
| 기억 | 기존 SQLite 원문/사건/버전/관계/검색과 native worker/CLI 연결; 동일 snapshot DB-free archive 조회 | 운영 SQLite 유지; 실제 새 사실/재시작의 최종 품질 합격 미완 |
| 연산 경계 | 의미/커널 교체 경계와 실제 Rust GEMV 후보 비교 | 후보가 더 느려 기각, 기존 기본 커널 유지 |
| 최종 목표 | S0~S3 및 별도 P0~P6 구현 검증 기록 보존 | S4 품질 FAIL, S5 정식 합격 미완, S6 INT4 미구현, 독립 검토 미실시 |

프로젝트 소스와 학습/변환/평가 도구는 Rust이며 외부 pretrained model/teacher/model API는
사용하지 않았다. 범용 Candle/autograd/tokenizers/rusqlite 등의 crate는 사용한다.
SQLite/zstd 및 선택한 OS Accelerate의 native 하위 의존까지 Rust라는 뜻은 아니다.
상세 의존성과 실제 저장 구분은 [의존·저장 감사](DEPENDENCY_STORAGE_AUDIT.md)에 있다.

### 학습됐다는 근거와 전이 실패

| 진단 | 실제 결과 | 해석 |
|---|---|---|
| 과거 full QA32 | 400/500step32/32, final fresh reload32/32; 별도32는0/32 | 전체 문장 암기 가능, 새 QA 일반화 미확인 |
| P1 contrast16 | random R-A600 / QA parent R-B200updates에서 각각16/16, fresh reload도 통과 | 작은 고정 입력은 학습 가능 |
| P1 사전 동결 새64 | 한 번 실행0/64 | 새 장면 전이 실패; 재학습 선택용으로 쓰지 않음 |
| U1 이전 근거 순서 반전 | 원본16/16인데 순서 반전0/16 | 고정 사례에서도 순서에 민감 |
| U1 원본+반전32 학습 | 200/300updates 연속32/32; exported inference fresh reload 원본/반전 각각16/16 | 네 base scene의32개 view 암기만 통과 |
| U1 별도 요소 변형 | 사건ID9/16, 알려진 값 교체6/16, 새로운 숫자 값0/16 | 순서 보강을 넓은 근거 이해로 일반화하지 못함 |

따라서 "gradient가 전혀 작동하지 않는다"는 결론도, "작은 자료를 외웠으므로 지능이
완성됐다"는 결론도 관측과 맞지 않는다. 현재 합성 한국어 자료의 제한된 테스트만으로
범용 언어 능력을 평가하거나 주장하지 않는다.

### 최근 일반 QA 실행 U2 — 개선 실패

기존 일반 QA parent step19,750에서 시작해 새 자료2,048개로 총250updates만 실행했다.
step20,000은 누적 표시이며 이번에20,000updates를 추가한 것이 아니다. U1 한 단어
진단 가중치를 일반 QA 모델로 연장하지 않았다. 별도 경로에 기존 Adam/RNG를 유지했고
변경한 corpus와8개 묶음 sampling을 명시했다. 기존 체크포인트는 모두 보존했다.

| 같은 입력의 실제 자유 생성 | 시작19,750 | +20updates | +250updates/중단 |
|---|---:|---:|---:|
| 새 train 앞128 전체 답변 정확도 | 55/128 (42.97%) | 54/128 (42.19%) | 31/128 (24.22%) |
| 일반 QA 개발검증336 | 155/336 (46.13%) | 153/336 (45.54%) | 56/336 (16.67%) |
| 보조과제64 | 16/64 (25.00%) | 16/64 (25.00%) | 0/64 (0.00%) |
| 개발검증 전체400 | 171/400 (42.75%) | 169/400 (42.25%) | 56/400 (14.00%) |
| 개발검증 token CE | 0.27005602 | 0.24824657 | 0.33113087 |

위400개는 반복 사용한 DEVELOPMENT이며 새 독립 최종 시험이 아니다. 최종 후보를
선정할 품질에 도달하지 못해 이번 U2의 새 final heldout는 실행하지 않았다. 기존 v9의
다른 보조64를 포함한219/400과 현재171/400을 같은 전체 평가로 비교하면 안 된다.

250updates 후 개발검증30/400에서 `native tokenizer: invalid/incomplete output UTF-8`
오류가 발생했다. 이는 실제 생성된 byte열의 디코딩 거부이며 원인 자체는 아직 분리하지
못했다. 오류와 별도로 개발검증12건, train 표본8건은 정상 종료했지만 답변이 빈 문자열이었다.
무근거 질문에서 답변이 사라졌고, 대상 숫자·방향값 혼합과 문구 반복도 관측됐다.
오류/빈 답변은 모두 정답에 포함하지 않았다. 정답/기록선택 oracle은 사용하지 않았다.

마지막 train loss0.008424405는 마지막 batch의 teacher-forced token loss다. 전체
train의 답변 정확도가 아니다. 정답 앞부분을 주는 token 예측과 자기 출력으로 이어 쓰는
자유 생성은 다르고, 숫자/값/인용 한 곳의 오류로도 전체 답변은 틀린다. 낮은 마지막 loss를
성공으로 판단할 수 없다. 검증 CE와 실제 답변 지표는 모두 악화됐다.

sampler replay 결과 전체2,048개 중1,352개만 이번run에서 한 번 이상 뽑혔다. 평가한
train128의64개는 미추출이고64개는1~3회 추출됐다. 이64개 중22개만 맞았다(34.375%).
이는 "전체 학습자료를 충분히 반복 학습한 뒤에도24%"라는 실험은 아니다. 그렇더라도
이전 능력 하락·빈 응답·디코딩 오류는 분명한 실패이므로 남은750updates를 실행하지 않았다.

### 원인에 대한 판단과 중단 범위

확인된 사실은 순서 민감성, 새로운 값/ID 전이 실패, 최근 자료·sampling 변경 후 일반 QA
성능 하락이다. 자료 대조를 강화하면 일반 QA가 좋아질 것이라는 U2 가설은 이번 실행에서
지지되지 않았다. 저장/회귀 테스트 성공을 모델 품질 개선으로 대신하지 않는다.

미확정 가설은 좁은 자료의 상관관계 암기, 같은 장면8개 묶음의 gradient 편향/분산,
기존 optimizer 상태와 변경된 자료 분포의 상호작용, 첫 target 가중치의 영향이다.
어느 하나를 근본 원인으로 확정하지 않았다. UTF-8 오류가 tokenizer 구현 결함인지,
생성된 byte 조합/종료 문제인지도 추가 분리 없이는 단정하지 않는다. 과거 shift/mask/
gradient/cache 검사 통과는 해당 범위의 근거이며 모든 수치·학습 문제의 부재 증명이 아니다.

재개한다면 현재 전체 QA 형식의16~32개를 고정해 실제 노출·token loss·자유 생성·새
process 복원을 함께 보는 진단이 우선 후보다. 이는 제안이며 이번 중단 후 시작하지 않았다.
추가 학습·자료 확대·모델/optimizer 변경·INT4 작업은 진행하지 않는다.

### 게시 범위와 검증

U1 코드/테스트/문서는 `e365b090f3916f1c04f1886d74de2d979e9e34fb`로 이미 게시했고
당시 원격 SHA 일치를 확인했다. 이번 스냅샷에는 U2의 `src/data.rs`, `src/train_main.rs`,
`tests/training.rs`와 상태/계획/실행 문서를 담는다. 기존 역사 문서/이미 추적된 로그의
명칭 정리3건도 보존하며 신규 모델 구현 성과로 세지 않는다. 새 소스 파일은 없다.
기존 미추적 원시 로그, corpus, 가중치/Adam, 운영 DB, 임시 지시문, target은 로컬 보존한다.

U2 직접 회귀 `full_qa_pairs_keep_split_and_bind_question_value_citation_in_both_orders`
1개 통과: split bytes, 질문에서 독립 유도한 원문/인용, 근거 역순, 기존 QA 보존, 입력 한도.
중단 전 fmt/check/Accelerate clippy/release build 통과를 확인했다. 테스트는 자료 변환의
정확성을 검증하며 모델 답변 품질을 보증하지 않는다. 게시를 위해 학습/평가를 재실행하지
않고 저장된 로그를 대조한다. 원격 전송 결과는 게시 후 실제 full SHA로 별도 보고한다.

## P0~P6 종료 시점의 기록 (아래 수치는 해당 과거 단계 기준)

당시 신경망 학습 프로세스는 NOT_RUNNING이었다. 기존 v12는 step 21,750에서
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
| P6 | VERIFIED / IMPLEMENTER_VERIFIED | kernel stride 및 artifact equation/state 거부 회귀 보강, 현재 보고 정리 | 직접43+resume/cancel/kill/group4개, fmt/check/clippy/release; fresh native contrast16/생성/archive/kernel | 필수 구현·실행 최종 대조 완료; 새값 전이 실패 유지 | 아래 최종 source/binary/log SHA | INDEPENDENT_PENDING; Goal1 미완 | 없음: 이번 계약 종료, 대규모 학습 재개 없음 |

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

## P0~P6 종료 시점 필수 결과 필드

RESULT: COMPLETE_THIS_CONTRACT
DELIVERABLE_VERIFIED: YES
MODEL_QUALITY_PASS: NO
FINAL_SOURCE_IDENTITY: 63117c99c09430e2b497e9c2b3f24fe65e0e1ff3f28654d18eb2aa1b0dadf706
DEPENDENCY_AUDIT: 실측 표는 DEPENDENCY_STORAGE_AUDIT.md
UNUSED_DEPENDENCIES_REMOVED: 이번 P0 없음; 이전 S4 WIP 별도
EXTERNAL_WEIGHTS_USED: NO
EXTERNAL_MODEL_API_USED: NO
PROJECT_SOURCE_LANGUAGE: RUST
NATIVE_TRANSITIVE_DEPENDENCIES: SQLite C, zstd C, onig C, OS; Accelerate는 선택 feature, 최종 실행은 CPU/gemm
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
P5 COMMIT / REMOTE_SHA: `5802ac56ccdc2c08c1a99002e6367bc5f8d89d50` / 동일 SHA 확인
P6 COMMIT / REMOTE_SHA: 이 보고를 포함하는 최종 commit 전송 후 full SHA를 최종 응답 및 로컬 p6-publication.txt에 기록한다. 자기 commit SHA를 문서에 순환 삽입하지 않는다.

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

## P6 최종 대조와 종료

이번 계약의 필수 구현·실행은 완료했다. RESULT=COMPLETE_THIS_CONTRACT는 진단과 저장
구현 계약의 완료이며 모델 품질 통과나 Goal1 완료가 아니다. DELIVERABLE_VERIFIED=YES,
MODEL_QUALITY_PASS=NO, GOAL1_READY=NO, 독립 검토는 INDEPENDENT_PENDING이다.
대규모 학습/자료 확장은 재개하지 않았다. 마지막 학습은 P1의 제한 R-A600/R-B200이고
P6는 보존한 가중치의 변환/재생성/수치 회귀이다.

SOURCE_CHANGED는 tests/native.rs의 shape가 유효한 비연속 stride 거부 검사,
neural/artifact.rs의 독립 fixture equation/state ID 손상 검사와 현재 문서 보완이다.
공식 project source는 Rust1.98.0/edition2024이며 새로운 의존은 없다.
수식/토크나이저 mapping/default reference는 P2/P3 검증 이후 바뀌지 않았다.

| 요구사항 | 실제 연결 및 관측 | 최종 판정 |
|---|---|---|
| 로컬 WIP·checkpoint 보존 | P0 시작 diff/파일 digest, 원본 init/일반 QA parent/v12/R-A/R-B weights 및 freeze SHA 재검증 | 보존; reset/stash/clean/모델 설치 없음 |
| 의존·JSON·tensor·SQLite 상세 조사 | DEPENDENCY_STORAGE_AUDIT의 활성175개/204tensor/FTS·WAL·SHM·cache 구분 | 완료; 각 crate 비용 미측정은 UNKNOWN |
| 고정16 입력독립 검증·제한학습 | 자체 input-only validator; shift/mask/denom/68gradient/batch/cache; 같은 LR/새 Adam, full16/update | R-A600/R-B200, 두 평가+fresh16/16·4/4 |
| 새로운64 사전동결·분리 | P1에서 R-B 한 번 평가 후 추가 tuning 없음 | 0/64·0/16 EXPERIMENT_FAILED 유지 |
| 수식/커널/내용/토크나이저/cache 경계 | 실제 dispatch, OperatorSpec/Capabilities, bounded experimental config, unknown equation/state 거부 | 기존 SMALL 의미 보존 |
| inference/resume native binary | 기본 checkpoint/worker/trainer가 실제 binary 사용; 명시 legacy import | JSON 없는 자체 format, Adam 분리 |
| 바이너리 정확성·내구성 | 독립 literal/encoder bytes, 손상/상한, F32 bits/merge token IDs, no-clobber/kill/retry | 검증; power-loss proof 아님 |
| 동일 checkpoint 상대 평가 | P3 fresh-process contrast16+QA32+일반QA336의 logits/생성384입력 일치 | RELATIVE_REGRESSION_ONLY; 품질 미승격 |
| 실제 JSON 없는 모델 생성 | P6 sandbox가 JSON/safetensors/DB/network 접근 차단; 기본 generate 성공 | 실제 logits 경로, 응답 품질 미달 |
| 정확한 재개 | 실제 TINY 6연속 vs3+fresh3, Adam/weights/RNG/config/loss/logits; group boundary; 취소/kill | 4개 직접 회귀 재통과 |
| 실측 병목 한 후보 | 같은 SMALL/prompt CPU/gemm F32 단일thread warm3/n31, Rust GEMV | 수치 통과, 느려서 KEEP_REFERENCE |
| 그래프 의미·DB-free archive | 동일10,000원문/329edge, 정정/취소/restore/as_of/scope/time/cycle/caps, 독립 literal와 손상 | DB_FREE_ARCHIVE_VERIFIED |
| 정확한 비교 범위 | lexical/combined 지원6개, FTS 미지원4개 별도; raw/zstd byte·시간·RSS | SQLite live/FTS 대체 완료 아님 |
| 기존 RV01~05와 실행 경계 | runtime/CLI/store/retrieval/codec 직접 회귀 | 원문·answer commit·취소·timeout·replay·scope·재시작 보존 |
| 단계 전송·기여 구분 | P0/P1/P2+P4/P3/P5 각 정상 push/full 원격 SHA 확인; P6 종료 후 동일 절차 | force/대형 artifact push 없음 |

최종 직접 테스트는 library6 + native9 + runtime6 + cli5 + codec3 + store8 +
retrieval6 = 43개, 별도 binary resume/cancel/kill3 + group resume1 = 4개로 **47개**다.
이전 P0/P1/P3/P5 반복 통과를 여기에 다시 더하지 않는다. 0 tests는 집계하지 않는다.
cargo test --offline --locked --features test-support --lib --test native --test runtime
--test cli --test codec --test store --test retrieval, training native_ 및 지정 group resume
filter를 실행했다. fmt --check, all-target check/clippy -D warnings, release bins/validate
빌드도 통과했다. v1/v2 전체 테스트나 별도 대규모 학습을 실행하지 않았다.

P6 R-B legacy→새 native resume은115,285,184 B, trained_steps200, diagnostic_only=true이다.
모델68개와 Adam136개의 이름/shape/F32 bit 및 TrainingState/TrainConfig 전 필드를 대조했다.
다시 읽은 native의 고정16 생성은16/16·4/4, EOS16/16이며 기존 P1 fresh reload와
ID/생성 token IDs/문자열/finish가 모두 같았다. 빌드 간 정답-대조 logit gap 차이 최대
5.7220459e-6은 사전 tolerance 이내이고 bitwise logits 동일이라고 부르지 않는다.
P3 동일-build legacy/native384입력의 완전 일치와는 다른 비교다.

원본 train 로그 마지막 loss는 R-A600=0.000222076, R-B200=0.000059094였다.
보존 manifest 값은 각각0.00022207557049114257 / 0.00005909441824769601로,
로그의9자리 반올림과 일치한다. 해당 snapshot SHA와 token budget/노출 수를 재확인했고,
R-B는 binary 변환 후 이 loss를 포함한 전체 상태도 exact 비교했다. 이 값은 당시 step의
실제 teacher-forced 학습 loss이며 새64 생성 정확도나 새로 실행한 optimizer step이 아니다.
원본·신규 로그는 로컬에 남기고 보고에는 익명 통계/hash만 넣었다.

P6 기본 generate는 P3와 동일한 v12 inference artifact/입력/상한으로 새 process에서
실행했다. 장비631장비, stop, output6tokens로 이전과 같았으며 제대로 된 인사 응답은
아니다. load315ms/first-token30ms/generation40ms는 단일 warm-OS 관측이고 성능 보장 아님.
같은 sandbox의 archive show는 NUL 포함24bytes 원문을 이전 결과와 cmp로 대조했다.
두 명령 모두 지정한 p6-must-not-open.db를 만들지 않았다.

P6 동일 kernel 후보 재측정 (warm3/n31, CPU/gemm F32, VECLIB1/RAYON1):
reference/candidate median ms는 micro0.0470/0.1533, decode2.1386/4.6219,
prefill43.6446/43.6050, 전체생성48.9697/51.6462다. 최대 logit 차이3.8146973e-6
(기준5e-4), 생성 token/EOS 일치. prefill/학습은 기존 differentiable reference이고
후보는 decode만 쓴다. 후보를 기본값으로 올리지 않았으며 새 후보/추가 tuning은 없다.

FINAL_SOURCE_IDENTITY는 git ls-files의 Cargo.toml/Cargo.lock/src/tests/examples 각 파일에
shasum -a256을 적용한 순서 있는 목록 자체의 SHA다. 문서·학습자료·모델·target은 별개다.
목록은 로컬 p6-source-files.sha256이며 공개 commit의 source bytes를 식별한다.

| P6 파일/증거 | SHA-256 |
|---|---|
| 최종 source 목록 | 63117c99c09430e2b497e9c2b3f24fe65e0e1ff3f28654d18eb2aa1b0dadf706 |
| R-B native resume | 946f41b0370e85f589e7920ec2769ea9d766d5b4253edced9458262cc9c7d37a |
| contrast16 native raw 출력 | 5ad6fca5bf1dd9d868a12b31673658121b5f33348802b8448fa8598f46fcc57a |
| kernel 실측 | 26e81ece4111512a0707f333b85107eda24585f1e882e7aaabdaa0a1c584325c |
| 직접43개 테스트 로그 | b3544795556da9d7acd9d50fe60e3c75db1225275465731b35cff4fbbd52db39 |
| binary resume/cancel/kill 로그 | 6f6b9feab2078c992ff8ab67a2376d2d09aa02c6140b0047f03761691e84839a |
| group resume 로그 | 1c70604fb57493efde0b9ad59111c3dd18a5da7173d7f6eab5ea55874ed19f59 |
| 실제 generate stdout | 44f764fdd92addb56a3ec01b503fb6797d3007fa542db59a72b09cec3b6a9df4 |
| 실제 archive 원문 | f3dc049d050e387d40dc97efc59399cf9d7ef93cfb39c34dc9ef2ed981512b21 |
| release replica-v3 | 06ee2bd0860f93693dc6ab1f9d0278359a819540a421d5874e687f5829f0a3f0 |
| release replica-train | c26dab84d16d57d9d020ab6347913b58475ee4499af080a77655d3f8a5e9da38 |
| release validate | 034fcbf65b582812d1626f1821d9520ebe6e1bc5e43b731e2fec2a3b39660dfb |

새 tracked 파일은 소스3개(contrast.rs: 제품에서 분리된 제한 진단, neural/artifact.rs:
모델 전용 binary, archive.rs: DB-free evidence 수명/검증)와 지정 보고2개다.
그 밖의 보고서는 기존 문서에 합쳤다. 기존 사용자 미커밋 문서3개와 untracked 과거 로그는
이번 commit에 섞지 않고 유지했다. 모델/optimizer/corpus/DB/raw 로그/임시 입력/target도
commit하지 않았다.

LIMITATIONS / REMAINING:
이번 좁은 계약의 필수 구현 미완 항목은 없다. 새64 전이는 실패했고 기존 일반 QA0/336,
S4 품질 FAIL, S5 정식 품질 합격 미완, S6 quant 미구현, 독립 검토 미실시는 그대로다.
F16/INT4/packed runtime, live SQLite 대체, 완전한 자체 tensor/autograd는 이번 완료 주장이
아니다. CPU RSS와 실제 read bytes는 측정했지만 allocator 내부 모든 임시 allocation을
추적하거나 GPU 메모리/성능·진짜 cold disk·전원 장애 내구성을 검증하지 않았다.
NEXT_DEPENDENCY: 없음. 별도 방향 결정 없이 대규모 학습/새 architecture 실험을 시작하지 않는다.

## Goal1 재개: 학습16의 요소별 전이 진단

사용자가 Goal1까지 계속 진행하도록 요청했다. 위 P0~P6의 종료 결과/실패는 그 시점 기록으로
유지하고 S4→S5→S6를 다시 진행한다. 전체95%/분류90%/잘못된 인용 승인0 기준은 그대로다.
현재 base는122101c7c19efce94e0660538f0930889937934d이며 기존 사용자 문서 WIP3개를 보존한다.
Rust1.98, 외부 모델/API/답변 하드코딩 금지, native binary 및 기존 SMALL 수식 유지다.

신규 transfer 도구는 frozen train16에 한 요소만 바꾸는 DEVELOPMENT 진단이다.
원래 새64는 평가하지 않았고 새로운 blind test로도 부르지 않는다. input-only validator로
정답 유일성을 다시 확인하며 원본 freeze/가중치를 변경하지 않는다. 체크포인트는 P6 R-B
native resume, 실제 CPU/gemm F32 단일thread이다.128개 생성17.92s, 추가학습0updates.

| 변경 요소 (각16) | exact | quartet all-correct |
|---|---:|---:|
| 원본 | 16/16 | 4/4 |
| 사건 ID만 변경 | 14/16 | 2/4 |
| 근거 순서만 반전 | 0/16 | 0/4 |
| 같은 길이 entity 숫자 변경 | 12/16 | 2/4 |
| entity 숫자를8자리로 변경 | 11/16 | 1/4 |
| context 이름만 변경 | 14/16 | 2/4 |
| 방향 값 순환 치환 | 4/16 | 0/4 |
| 새 숫자 경로 값 | 0/16 | 0/4 |

순서만 바꿔도 전부 실패하므로 상태/질문 의미에 대한 순서 불변성이 확보되지 않았다.
모든 실패의 유일한 원인이라고 단정하지 않는다. 숫자 경로 값은 학습16 정답에 없던13개
token ID를 포함한다. 방향 순환에서도2개가 그 정답 집합에 없었다. source/ID/context/value
동시 변경 결과만으로 일반화를 하나의 지표로 설명하지 않는다.

다음 U1은 이 순서 의존만 교정하는32-case 진단으로 사전 고정한다. 원본16+순서반전16,
R-B parent·tokenizer·SMALL·F32 불변, 새 Adam/LR.001/warmup20, micro4×accumulation8,
update마다32개 전체1회, 최대1,000updates/320만input/45분(먼저 도달)이다. 32개두평가
연속완전정합 뒤 fresh process 원본/반전 재검증이 조건이며 Goal1 pass는 아니다.
한 번에heavy작업 하나, 학습 중 source를 고정한다. 원본16 및 새64를 덮어쓰지 않는다.
NODE=S4/U1, STATUS=VERIFIED_DIAGNOSTIC; SOURCE_CHANGED=contrast.rs/train_main.rs 및
neural/checkpoint.rs/artifact.rs. 관련 contrast unit3개와 artifact 회귀1개 통과,
check/clippy/release build 완료. 학습 중에는 코드 및 source hash를 고정한다.

첫 실행은0update에서 저장 state의16개 고정 guard에 의해 거부됐다. 기존16개와 명시적
32개만 허용하도록 검사하고12/24/36개는 거부하는 binary roundtrip 회귀를 추가했다.
실패 출력은 보존하고 다른 출력 경로로 실행했다. 현재 source 목록 SHA는
176560efbb50aec08b06500fb409d9cb2dfa188583dd087829fd65c8c467f3e6이며,
32개 학습 사례 SHA는3643f50b9716726f7542b284466ec327f0e5d18ae58ed2d6661be246c84de94c다.

실행: `replica-train contrast train --fixture <frozen> --checkpoint <R-B-native-resume>
--output <new-output> --source-id <source-sha> --start diagnostic --both-orders`.
실제 backend는CPU/gemm F32, VECLIB1/RAYON1이다. 과거P1의 R-A/R-B 학습은
CPU/Accelerate였다. 이번 순서 변경 전후 추론은 모두CPU/gemm으로 측정했지만
이전 run과 학습 속도/최적화 경로까지 동일한 비교라고 부르지 않는다.

0update는16/32·4/8묶음,100update는29/32·6/8묶음·EOS32/32다.
100update 입력538,000/target6,400토큰, loss0.069477400이다. 원본16과 순서반전16은
독립32개 scene이 아니라 동일4개 scene의8개 변형이다. 인용 생성은 요청하지 않았다.

U1 종료:200/300update 모두32/32·8/8·EOS32/32.300updates, input1,614,000,
target19,200, sampler8507264816735876025, 마지막loss0.000048603,
종료이유TWO_EVALUATIONS_PASS_PENDING_FRESH_RELOAD. 실제학습1,367.92s,
최대RSS1,015,283,712B. 원본R-B 파일SHA 불변을 재확인했다.
RESUME115,285,184B를 INFERENCE38,432,768B로 export했고, 새로운process에서
원본16/16·4/4 및 순서반전16/16·4/4를 통과했다. source quality는DIAGNOSTIC_ONLY다.

| U1 inference 재로딩의 개발 조건 | exact | quartet all-correct |
|---|---:|---:|
| 원본 | 16/16 | 4/4 |
| 사건 ID만 변경 | 9/16 | 0/4 |
| 근거 순서만 반전 | 16/16 | 4/4 |
| 같은 길이 entity 숫자 변경 | 11/16 | 1/4 |
| entity 숫자8자리 | 8/16 | 0/4 |
| context 이름 변경 | 14/16 | 2/4 |
| 방향 값 순환 | 6/16 | 0/4 |
| 새 숫자 경로 값 | 0/16 | 0/4 |

재로딩128개 생성17.82s/최대RSS205,783,040B. 순서 학습은 통과했지만 다른 요소의
전이가 악화된 항목도 있다. 일반화 해결·일반 QA 성공·Goal1 완료가 아니다.
새64/최종heldout를 추가 실행하거나 후보 선택에 사용하지 않았다.

평가 도구의 오류도 수정했다: Adam 없는 native inference를 미학습으로 오인하던 검사를
보존된trained_steps 기준으로 바꿨다. 직접 회귀1개는 TINY의 실제1회 gradient update→
optimizer 없는binary→평가의품질실패 도달 및 random/trained 역분류 거부를 확인한다.
의도적으로 긴 입력을 거부하는 이 unit의200개는 모델 품질 평가 실적으로 세지 않는다.
이번 직접회귀는 contrast3+artifact1+평가기1=5개이고 fmt/check/clippy/release 통과다.
처음 test명 filter가0개를 선택한 로그는 보존했으며, 완전한 test경로로 실행한1개만 센다.

| U1 파일 | SHA-256 |
|---|---|
| resume | e1933e202b13aed509fca9aef4f4fc661fb355aa5c666e404334b6cf215c89ae |
| inference | 06760d59145b862ef51367e2ee9620923c7f231bb80578293a5cf293a109985b |
| 학습 raw 결과 | 46f097878d33013cf7754c43151adfff0cac3698509a4bc1356e1e9585fd9644 |
| inference 요소별 raw 결과 | a6554d621f70a8141f1ad1c7f1c8b4fcf046e5d4c2655b48bba052ffe17af132 |
| 평가기 수정 포함 최종 source 목록 | 357f0c827aa383b00ae38e521be19c30b4af01a0e1bcc8263dccb3d1c62ee08d |

DELIVERABLE_VERIFIED=U1; MODEL_QUALITY_PASS=NO; GOAL1_READY=NO;
S4_QUALITY=FAIL; S5/S6의 선행 품질 조건 미충족; INDEPENDENT_PENDING.
다음은 이32개를 더 오래 학습하는 작업이 아니라, 근거값과 사건번호를 함께 생성하는
일반 QA의 대조자료/실제 생성 오류를 교정하는 제한된 단계다. 기존 수식/저장과 checkpoint를
보존하며 새 변경·실행 예산·개발검증을 먼저 명시한다.

## S4/U2: 기존 장면의 일반 QA 대조 묶음 (실행 전 고정)

BASE_HEAD=e365b090f3916f1c04f1886d74de2d979e9e34fb, 정상push/원격SHA 일치 확인.
U1은 작은 순서 대조를 암기했지만 ID/값 전이가 부족하다. U2는 기존 일반 QA가 서로 다른
근거의 값과 인용을 섞는 문제를 대상으로 하며, U1의 방향 한 단어 모델을 서비스 후보로
연장하지 않는다. 기존 validation으로 선택해 보존한 v9 step19,750을 native resume으로
명시 import한다. Git/source/기존artifact를 과거로 되돌리지 않는다.

새 `corpus qa-pairs`는 기존 query-pairs 학습20,000개에서128개 질문/값 quartet을
일반 QA 질문+원문/인용 target으로 바꾸고128개 일반 QA quartet을 함께 유지한다.
각 quartet의 두 근거 순서를 포함한8개씩, 총2,048개/256개 base 묶음이다.
원문/사건ID/status/time/값 자체나 새base를 만들지 않고, 기존 전체답 문장 형식을 유지한다.
질문은 선택할 entity/context/status를 담으며 model input에 gold ID/답을 넣지 않는다.
선택 label은 training 전용 module에만 있다. 같은 entity/context/status가 중복되어
요청한 기록이 모호하면 거부한다. 기존 QA quartet의 질문/정답/근거는 그대로다.
0/1개 근거의 역순 view는 내용이 같으므로 서로 다른 관측 장면으로 세지 않는다.

train 분류별720/464/464/200/200, validation400은 이전 DEVELOPMENT bytes와 같다.
처음 만든 초안은 보조과제의 category3 표시를 계승했으므로 사용하지 않았다. 실제 과제의
대상/버전/장소 분류로 바꾼 별도 fixed corpus를 사용한다. 검증 자료를 학습에 섞지 않는다.

| 실행 전 고정 identity | SHA-256 |
|---|---|
| source 목록 | 4be449d0505e1fb85152bbc5112ac3c6088587307b758f8de53fce600dcb5c86 |
| parent native resume | 1bcae73d7f2f7f501acc43e3958d66781b22d139533b12d6e369624171356849 |
| fixed train2,048 | 7c19a05354ddbed179b7ccb2f9dc667ad71a98d2f4bd426a9e74e0ffd8046b70 |
| 동일 validation400 | f8d18fe3f6bd2b14045218139eafabec41486420779295fe1273a12c8d697864 |

실제 기존 SMALL/F32/801BPE/Adam/RNG를 유지한다. CPU/Accelerate,
VECLIB1/RAYON1, LR.0003/warmup100, micro8/accumulation1, first_target_weight8을
그대로 쓰며, 추가로 명시한 sample_group_size8로 대조 묶음 전체를 뽑는다. 기존 일반
trainer의 explicit extension/corpus replacement를 재사용한다. 시작step19,750,
input32,501,322, sampler2077817959327737305. 최대추가1,000updates/2,000만input/
45분 중 먼저 닿는 예산, 수치 이상/16GiB 초과는 중단한다.20updates probe를 먼저
저장하고 이 동일 예산/state 안에서만 재개한다. 무제한 extension/LR·seed sweep은 없다.

고정start와 probe의 train앞128/validation400 실제 생성을 비교하고, 이후 저장된
step20,000/20,250/20,500/20,750에서 validation의 일반QA macro로만 후보를 선택한다.
동률은 일반CE가 낮은 것, 다시 동률이면 앞선step이다. 보조64와 일반336을 분리해
보고한다. 일반QA 전체95%/각분류90%도 못 넘으면 새 최종heldout를 소모하지 않는다.
새 최종fixture는 후보가 고정되고 개발 품질을 통과한 뒤 생성한다. 원래Goal1 조건은
그대로이며 대조 묶음의 암기만으로 완료하지 않는다.

NODE=S4/U2, STATUS=EXPERIMENT_FAILED / PAUSED_BY_USER; 최종20,000step에서 중단.
아래20update 기록은 중간 관측이며 현재 상태는 뒤의250update 종료 기록을 따른다.
소스: data.rs/train_main.rs, 직접 통합회귀: tests/training.rs의 full_qa_pairs...
1개 통과. validation byte동일성, 원문/인용/질문 유일성, 순서 대조, 기존 일반QA 보존,
한도 거부를 검사했고 fmt/check/Accelerate clippy/release build를 실행했다.

20update probe는step19,770에서 명시 stop_after로 정상 저장했다. 추가input50,084/
target4,696, 실행39.32s, OS maximum RSS5,641,568,256B이며 단계 사이 ps의 최고
1,429,680KiB보다 높다. sampled RSS만 최대 메모리로 보고하지 않는다. 종료이유TRAINING,
optimizer_boundary exact resume 가능, validation CE0.27005602→0.2482465661이다.

| 실제 greedy DEVELOPMENT | start19,750 | probe19,770 |
|---|---:|---:|
| 새train 앞128 전체답 | 55/128 | 54/128 |
| 기존일반QA | 155/336 | 153/336 |
| 기존보조 | 16/64 | 16/64 |
| 일반QA0 | 13/68 | 13/68 |
| 일반QA1 | 22/68 | 22/68 |
| 일반QA2 | 5/68 | 9/68 |
| 일반QA3 | 54/68 | 50/68 |
| 일반QA4 | 61/64 | 59/64 |

이 시점에서 품질 개선을 주장하지 않는다. 저장된sampler의 결정적 replay로 총160개가
한 번씩 노출됐고1,888개는 이번run에 미노출임을 확인했다. 평가한train 앞128 중에는
8개만1회 노출,120개 미노출이다. corpus에 속한다는 사실과 이번run에서 학습했다는
사실을 구별한다. 이후 명시 중단점20,000까지 진행한 결과는 다음과 같다.

### U2 최종250update 관측 및 보존

20update probe 이후230updates를 같은 예산/Adam/RNG로 재개해step20,000에서
`--stop-after 20000`으로 정상 저장·종료했다. 로그의 `reason=TRAINING`은 이 명시
중단점의 내부 상태이며 프로세스가 계속 실행 중이라는 뜻이 아니다. 이후 새 process에서
고정 checkpoint의 train128/development400 생성을 완료했다. 결과는 문서 첫 표와 같고
일반QA 분류별4/68,13/68,5/68,26/68,8/64; 보조0/64다. 품질 하락으로 남은750updates와
나머지 사전 후보 지점20,250/20,500/20,750은 실행하지 않았다.

추가 input613,756/target57,272, 누적 input33,115,078/target2,275,700.
학습 두 구간39.32s+244.90s=284.22s이며 평가 시간은 제외한다. 두 번째 구간 OS maximum
RSS6,547,439,616B(약6.10GiB), swaps0; sampled max1,567,936KiB와 구별한다.
NaN/OOM을 종료 원인으로 보고하지 않는다. 마지막 plain train CE0.0084244050,
validation CE0.3311308720; `task_quality=NOT_EVALUATED`인 trainer 로그와 별도로
위 실제 생성 품질은 FAIL이다.

전체 exposure histogram은0회696/1회856/2회376/3회88/4회32, 총2,000draws다.
train 앞128은0회64/1회48/2회8/3회8, 각각 정답9/18/2/2다. saved sampler state와
결정적 replay가 일치하며 과거 parent 전체 lifetime의 노출 수로 확대하지 않는다.

아래 파일은 `artifacts/goal1-resume-20260917/` 아래 로컬에 보존한다. 전체 파일 SHA와
모델 tensor 내용 digest를 혼용하지 않는다. final의 tensor digest는
`dc2842a267f29d87b64f63a142310fb1735c4e43c727956c0fb1e5f9934ee0f0`이다.

| 로컬 보존 파일 | 전체 파일 SHA-256 |
|---|---|
| u2-to-20000/final | 34c2ef0630b6afe1df1b4b901a8679d4461edb041e4885b812fe91935eb1ad2c |
| u2-to-20000.txt | 5fb86cb2999582b46c52464c4048853a8536a16e41707065510d273779529c00 |
| u2-20000-train.jsonl | 40b278503f18c2deffe797c91a4e7fbcf69c763abded72b268133d6f57586be6 |
| u2-20000-validation.jsonl | 3605f11e505cb87e0ee280ade6e129a26755bccb909dd17e586c765b6fbc804e |
| u2-20000-exposure.txt | e73209cd51127694cd16be0c55456b3874c1eb934feef785a762cad076835147 |

parent checkpoint, P1/U1, U2 start/probe/final은 덮어쓰거나 삭제하지 않았다.
