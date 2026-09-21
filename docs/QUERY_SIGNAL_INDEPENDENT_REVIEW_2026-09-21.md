# Query signal — 독립 A 재검토와 기존 실행 결과 검산

검토 기준은 R3-QUERY-SIGNAL-CONVERGENCE-1.0이다. **A 전체 수용은 FAIL이다.**
기존 512/1024 판정 수정과 정상 관측 비개입 회귀는 통과했지만, 순수 시간 종료로
한 번도 호출되지 않은 관측이 다음 process의 재개를 영구 차단하는 Medium 결함
1건을 독립 fixture에서 재현했다. 제품 코드는 수정하지 않았다.

이미 끝난 SMALL 연구의 raw와 사용량은 다시 계산했다. 해당 실행에서 이 결함이
발생했다는 증거는 없으며, 품질 미달의 원인으로 주장하지 않는다. 이번 A가
미수용이므로 새 B 16회 재생성과 confirmation으로 진행하지 않았다. 기존 검토자의
16회 기록은 보존하고 읽기 검산했으며 이번 신규 실행으로 합산하지 않았다.

## 식별과 범위

|항목|값|
|---|---|
|REVIEWED_SOURCE|a8c7f0fa261b2bf6804e02783d6f78bdcb0c7b28|
|비교 기준 source|32df6dcb1c86020b9fc5402ff3b8bc32f3180f0e|
|구현 REPORT_SOURCE / 검토 시작 HEAD|327beac84d6c76d0da6dfc7ce676b32eada528f0|
|Compiled source digest|1892e1c5575658e41c6318b32018e1a8f3d052e325931b778da999c57fae2f7e|
|기존 production executable의 실제 재계산 SHA256|c037ec25664e4a2945628bff0724a2ddf96a4da7e5a1a2f648a327857305d49f|
|원래 실행 FEATURES / ENV|production accelerate, test-support 없음; CPU/F32/Accelerate/thread1|
|이번 직접 회귀 FEATURES / ENV|accelerate,test-support; Rust/Cargo1.98.1; locked/offline; compute thread1|
|독립 process fault binary SHA256|210e9d44d32f2a8560633bd49ec4ac4fa2327406b4b70f128b0910ae91c0c834|
|독립 unit fault binary SHA256|d5fdef29aa753d955311f2bf3dd9777526d61ddfed97b8634e41bac4f1c0f6c4|

Candidate 이후 시작 HEAD까지 변경은 상태/계획 문서 2개뿐이었다. 원본 src와
격리 candidate 사본의 src는 byte diff가 없었다. 기존 production binary는 hash와
기존 실행 연결을 확인했으며 이번에 SMALL 추론/학습용으로 실행하지 않았다.

격리 작업 위치는 로컬 `artifacts/query-signal-independent-20260921-EpXxfa`이다.
`source/`는 candidate production source의 정확한 사본에 독립 reader example만
추가한 것이다. `fault-source/`에는 검토 전용 test와 기존 test-support deadline
hook에 환경변수를 연결한 fixture만 추가했다. hook은 실제 stop/budget 검사에서
시간이 만료되게 하며 production 판정·관측·저장·재개 논리를 변경하지 않는다.
두 overlay diff와 실행 binary를 scratch에 보존했다. 원본 source/tests/Cargo,
corpus/checkpoint/raw/기존 종료 decision과 수용 receipt는 수정하지 않았다.

정상 TINY 실행에 사용한 candidate test binary는 후속 fault build로 같은 격리
target에서 교체되어 그 최초 binary hash는 별도 보존하지 못했다. 해당 실행의
candidate source, build 명령/로그, 자식 process 로그와 checkpoint/receipt는
보존했다. 위 fault binary hash를 정상 TINY binary hash로 대신 사용하지 않는다.

## 발견 이슈 QS-R1

**Severity: Medium**

**Location:** `src/binding.rs:2345`의 `signal_authorize()`, 특히 2352–2355.
연결 경로는 `src/fresh.rs:397`의 `SignalObservation::start()`, 406의 `measure()`,
456의 `finish()`, `src/training.rs:1447`의 관측 실패 정리,
`src/fresh.rs:1774`의 순수 시간 종료/재개 판정이다.

**Evidence:**

1. `start()`가 먼저 `signal-probe-…-started.r3b`를 발행한다.
2. `measure()`는 `begin_teacher_rows(8)`의 stop 검사 이후에만 forward 수를
   증가시키고 모델을 호출한다. 이 검사에서 시간이 끝나면 실제 호출은 0이다.
3. 학습 정리 경로는 이 `TIME_BUDGET`도 `finish(Some(error))`로 저장한다.
4. 외부 segment 경로는 순수 `TIME_BUDGET`, 저장 성공, 안전한 state를 확인하고
   `TrainingPending / resume=true`로 저장한다.
5. 다음 진입의 `signal_authorize()`는 관측의 `error`가 non-null이면 호출/commit
   여부나 순수 시간 종료를 구분하지 않고 `signal observation failed`로 거부한다.

**Problem:** 계약의 순수 TIME_BUDGET 재개·미호출 구분과 실제 admission이 어긋난다.
관측도 optimizer도 호출되지 않고 이전 checkpoint를 정상 보존한 경우인데
영구 실패가 된다. segment가 재개 가능하다고 표시한 상태와 다음 process의
행동도 불일치한다.

**Impact:** 새 query-signal 연구에서 local 1/8/32/128 관측의 시작 기록 이후,
첫 sample-forward 진입 전에 시간 한도가 소진되는 경계에 영향을 준다.
현재 완료된 SMALL 연구의 점수나 데이터 손상 문제로 확대하지 않는다.
저장된 모델을 정상 후속 실행에 사용할 수 없는 복구/상태 전이 결함이다.

**Reproduction — 이번 직접 실행:**

- 보존된 TINY 부모에서 scratch에 새 parent-bound signal policy를 준비했다.
- 새 process에서 기존 `before_teacher_budget` stop 검사에 deadline을 주입했다.
- 첫 process는 exit0, `checkpoint_saved=true`, step2,
  `TrainingPending / resume=true / TIME_BUDGET`이었다.
- optimizer/generation/teacher/diagnostic forward/backward는 모두 0이었다.
  observed condition은 TIME_BUDGET 하나뿐이며 모델·Adam을 변경하지 않았다.
- 다음 새 process는 `signal observation failed`로 실패했다.
  두 번째 segment 시작 기록이나 추가 모델 호출은 없었다.
- `logs/fault-process.log`, `logs/fault-child-{0,1}.{stdout,stderr}`에 보존했다.
  별도 `logs/not-invoked-fault.log`에서도 같은 admission 거부를 확인했다.

재현 test의 PASS는 결함 조건의 재현 성공이며, 재개 기능 PASS가 아니다.

**Recommended Fix:** `SignalObservation`의 완료 기록과 `signal_authorize()`에서
실제 호출/optimizer commit 상태를 기준으로 미호출 시간 종료와 실패/UNKNOWN을
구별한다. 안전한 무호출 TIME_BUDGET은 보존한 checkpoint에서 재개할 수 있게
하고, 기존 관측 기록을 덮어쓰지 않도록 후속 attempt의 identity를 명시한다.
segment의 `resume` 표시와 admission 조건도 일치시킨다. 이미 반환된 실패나
미반환 UNKNOWN, 취소, 저장 실패를 일반적으로 재시도 가능하게 바꾸면 안 된다.
optimizer가 이미 commit된 이후의 관측 중단은 별도 상태로 다뤄 동일 update를
다시 수행하지 않도록 해야 한다. 직접 패치는 적용하지 않았다.

## 직접 실행한 검증

|검증|이번 결과|
|---|---|
|512/1024 모든 full/signal 조합과 unsupported step|지정 test 1 PASS|
|실제 binary publisher/reader comparison, 혼합/누락/잘못된 후보와 row budget|지정 test 1 PASS|
|최종 record 0/1/2/128/255/256/257/511/512 writer/reader|지정 test 1 PASS|
|TINY observer off2 / observed 새 process1+1 / 평가 전용2+0|지정 test 1 PASS; weights/Adam/clock/tokens/dev raw bitwise 일치|
|관측 첫 호출 전 시간 종료의 unit admission|결함 재현|
|관측 첫 호출 전 시간 종료의 실제 segment·다음 새 process|결함 재현|

기존 TINY test는 dev raw를 직접 비교한다. 기존 보존된 TINY 결과의 train/dev raw
일치도 reader로 재확인했지만, 이를 이번 새 TINY test가 양쪽 raw를 모두 비교한
것으로 표현하지 않는다. 새 test의 수치 fixture는 EOS 편향으로 S/gS가 0이므로
nonzero SMALL gradient를 독립 재생성했다는 근거로 쓰지 않는다.

초기 TINY 실행은 검토자의 retained-parent 경로 지정 오류로 모델 호출 전 실패했다.
그 로그를 보존하고 실제 존재하는 부모 경로로 새 scratch에서 실행했다. Reader
example build도 잘못된 working directory로 한 번 실패한 뒤 격리 source에서
성공했다. 두 검토 설정 실패를 제품 결함이나 테스트 PASS로 합산하지 않았다.

이번 신규 비용은 TINY optimizer6, generation24, teacher/sample56이다.
정상 training forward/backward6/6, 별도 진단 microbatch forward4/backward2이며
진단 sample-forward32는 위 teacher56에 이미 포함된다. fault 재현은 모두 0이다.
신규 SMALL optimizer/generation/teacher/diagnostic forward/backward는 모두 0이다.
scalar finite-difference도 0이며 이미 사용된 16회를 반복하지 않았다.
기존 실패 시험까지 재계산한 TINY13/56/200에 이번6/24/56을 더하면
optimizer19/generation80/teacher256이다. teacher 상한에 도달하여 더 실행하지 않았다.

## 수학·준비자료·기존 결과 재검산

검토한 기존 Rust reader를 scratch에서 사용했고, raw 채점 reader에는 strict
token→text/Generated receipt 일치 검사를 추가하여 별도로 빌드했다. raw의
`exact_match` 값을 신뢰해서 점수를 합산하지 않았다. 모델 호출은 0이다.

- 새 initial은 지정 QE512 parent와 전체 native bytes가 같고 Adam/state를
  포함한다. step/cursor512, inherited input593920/target8192와 QE framing을 확인했다.
- 실제 source/binary/selection/preparation/plan/native binding, corpus·metadata·
  tokenizer 사본, 기존 CLOSE_NO_JOINT_SIGNAL decision hash를 확인했다.
- 학습은 원래 512 tape의 suffix 1536개, 각 slot batch8, constant LR3e-4였다.
  trace 전수의 absolute clock, LR bits, draw, input/target, CE/objective를 대조했다.
  새 각 행24회, 부모 포함32회이며 부모512 updates를 신규로 합산하지 않았다.
- 동일 evidence의 q0/q1, 서로 다른 gold와 각 assignment의 orientation을 확인했다.
  `c=(m0-m1)/2`, `d=(m0+m1)/2`를 저장된 gold/foil logits로 재계산했다.
  고정64와 전수512의 분모를 구분했다.
- L2의 d 미분은 `-(sigmoid(-c-d)+sigmoid(c-d))/2`이므로 d=0에서 -1/2이다.
  이는 독립 logit 좌표의 이진 진단이며 full-vocabulary CE parameter gradient로
  해석하지 않았다. optimizer는 기존 CE map을 사용하고 관측 gS를 합치지 않는다.
- 기존 QE/EQ 0/128/256/512 raw와 새 QE 768/1024/1536/2048 raw/teacher를 재집계했다.
  EOS·finish·error·raw token·case/prompt/native·호출 원장 연결을 별도 검사했다.

|같은 endpoint / 전수 panel|FULL /512|QUERY_BOTH /256|SWAP_BOTH /256|ALL4 /128|
|---|---:|---:|---:|---:|
|QE1024 train|269|17|35|1|
|QE1024 dev|253|10|31|0|
|QE2048 train|472|216|217|96|
|QE2048 dev|294|77|101|21|

최종 errors는 양쪽 0이다. 최종 양수 margin pair는 train218/dev91로 실제
QUERY_BOTH216/77과 다르다. 최종 full-vocab digit NLL은 train0.222022513,
dev1.400022346이다. 이진 진단이나 EOS 개선을 정상 생성 품질로 대체하지 않았다.

기존 실제 원장은 optimizer1536, input1781760, target24576,
generation2352, teacher/sample2432와 일치했다. generation은 예정2304와
기존 parent/구현자/검토자 parity 각각16의 합이다. teacher는 평가2304와 관측128이다.
별도 진단은 microbatch forward16/backward6이다. 새로운 run이나 16회 재생성은
기존 원장에 끼워 넣지 않았다. 모든 기존 생성2352/평가 teacher2304의 resolved
state는 RETURNED였으며 UNKNOWN0이었다. 최종 Finished/resume=false/
FINAL_QUALITY_FAIL과 후속 segment 부재를 확인했다.

## 판정과 보존

|구분|이번 판정|
|---|---|
|SOURCE / READY / A 전체|FAIL — QS-R1 Medium 1건|
|LOW_STAGE_FIX|PASS — 기존 Low 수정 확인|
|QUERY_SIGNAL_MATH|SOURCE + DERIVED_EXISTING_RAW 확인; 새 finite-difference0|
|OBSERVER_NONINTERFERENCE|정상 TINY 수치 경로 PASS; 시간 종료 admission은 FAIL|
|DATA / LEGACY_PRESERVED|고정 부모·자료·tape 확인, 보존 manifest 50개 전후 모두 일치|
|PREP_AND_RESUME|준비와 정상1+1/평가 전용 재개 PASS; 무호출 TIME_BUDGET 재개 FAIL|
|ACTUAL_UPDATE_BUDGET / RAW_SCORE_INTEGRITY|기존 전수 원장/teacher/raw 재검산 일치|
|STUDY_EXECUTION|기존 유한 연구1536 신규 updates 완료 확인; 이번 SMALL 학습0|
|TRAIN_BINDING / UNSEEN_BINDING|모두 FAIL — 사전 gate 미달|
|B 새 process16 / B 전체 수용|NOT_RUN_A_FINDING / 이번 수용 부여 안 함|
|CONFIRMATION|NOT_RUN_PREREQUISITE; 내용 미개봉|
|MINIMAL_BASELINE|NOT_ESTABLISHED|
|S4/S5/S6/GOAL1|NOT_ACCEPTED; 이번 근거로 완료 선언 안 함|

보존 검증은 사전 제공된 50개 원본 manifest 범위다. 전체 artifacts 디렉터리를
새로 전수 해시한 것으로 확대하지 않는다. 제품 소스 변경 0, 기존 사용자
`.DS_Store` 보존, 새 영구 변경은 이 검토보고서 하나다. 원본 검토 수용 receipt를
수정하거나 과거 검토를 이번 실행 PASS로 바꾸지 않았다. 기존 연구를 다시 열거나
실패 품질을 근거로 학습 가능성 전체를 부정하지 않는다.

## 수정 후 필요한 직접 테스트

- unit: 관측 미호출 TIME_BUDGET과 RETURNED failure/UNKNOWN을 구분하고,
  state·호출/commit count와 admission 결론이 일치하는지 검사한다.
- integration: 이번 두 process 재현을 사용해 안전한 미호출 종료 후 재개가
  성공하며 이전 모델/Adam·절대 clock을 정확히 계승하는지 확인한다.
- regression: 512/1024 action, on/off2, 1+1, 평가 전용 optimizer0을 유지한다.
- malformed / boundary: 첫 forward 직전, microbatch 사이, optimizer commit
  전후, 관측 완료 기록 직전의 각 경계를 구분한다. 누락/손상 receipt는 거부한다.
- failure path: 취소·미반환 UNKNOWN·저장 실패의 차단을 유지하고, 이미 반환된
  실패를 좋은 결과로 바꾸기 위한 재호출이나 동일 update 중복 수행을 금지한다.

관련 source를 수정하거나 SMALL 재학습할 권한을 이 보고서가 추가하지 않는다.
새 검토보고서의 commit 및 실제 remote SHA는 게시 후 사용자에게 별도로 전달하며,
위 reviewed source나 구현 보고서 SHA와 혼동하지 않는다.
