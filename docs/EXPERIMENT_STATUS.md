# 진단 및 구현 상태

## 2026-09-21 REBIND consolidation — C0/C1 검증, 새 A 전

기존 source `b5e0d504a6a23ea8df9a7a1236690361cd13c86b`, 보고 HEAD
`b20a48c0edf505a39ce688cbc42a624960488c68`에서 단일 REBIND-CONTINUE를
준비한다. 과거 독립 A/B·QS-R1 수용은 보존하며 새 수리나 두 군 재실험이 아니다.
기존 REBIND3584의 FINAL_QUALITY_FAIL/resume=false는 그대로다.

C0 독립 pure reader 실행2개 exit0, 신규 모델 호출0. 부모 physical
`385600c2ad9ae99e496fa1b404aec192e2cb0cf1770fc1f6071b0bc75c469f11`,
tensor content `d2420af178ea4d948aa2cb7d99bd233e62bc5a13e05c6106c991a82edeb0d063`,
evaluation weight `543c242c03e142433414bfa912c8a6cfa8116cb5d5497f427eb2c1e64c1bd9c0`,
Adam136 `cd4d9ab576b59ddf48b667140a21dfa2b81f4924615a4e5b4652218f64de2d04`.
실제 step/sampler3584, input4157440/target57344와 전량 raw/teacher를 검산했다.
FULL/QB/SB/ALL4는 old507/251/251/123, new1004/492/492/236,
dev476/220/223/99. 첫16 skeleton의 고정64는 old64/32/32/16,
new63/31/31/15, dev59/27/27/11이며 새 생성 결과가 아니다.

이번 실행: SMALL updates/generation/teacher 모두0; 새 A와 학습 NOT_RUN.
기존 하네스의 새 단일 군 dispatch/부모 native binding/실제 tape suffix/LR/
4352 조기 후보 고정/5120 최종 종료/부모 대비 회귀 판정을 연결했다.
old/new/dev는 같은 endpoint에서 재채점하며 원래 두 군의 의미는 유지한다.

직접 `quick --rebind-consolidation`: cargo check exit0, T1/T3/T2 각각 실제1개
통과(합3/3), checker exit0. T1은 native 전체1536 suffix/12288 노출과
input1781760/target24576를 확인한다. T2는 EOS tensor fixture의 실제 native
forward/Adam으로 연속2, 새 process1+1, 평가 중단 뒤2+0의 weights/Adam/
clock/cursor/token/raw 일치를 확인했다. Returned2행을 유지하고 재개 generation10,
optimizer0으로 나머지 평가를 끝냈다. 같은 content의 다른 물리 native는 허용하고
다른 모델과 누락 raw는 거부했다. 해당 최종 T2 비용은 optimizer6/generation48/
teacher36이며 기존 test parent를 읽기 전용 재사용했다. SMALL 품질 증거가 아니다.

최초 T2는 macOS 임시 경로 별칭으로 준비에서 실패해 fixture 경로를 정규화했다.
후속 T2는 수치·process PASS였지만 준비 부모의 step0 평가 카운터를 누락한
표시를 실제 child receipt 기준으로 보정했다(실제10/80/68). 첫 실패 비용4/32/32와
최종6/48/36을 포함해 현재 TINY 총20/160/136이다. 이후 추가 native-copy 시험의
첫 컴파일 실패(private counter 접근)는 기존 public receipt 사용으로 고쳤다.
해당 컴파일 실패의 실행 테스트0·모델 호출0은 PASS에 세지 않았다.

release build/check exit0. 전체 fmt exit1은 기존 포맷 차이, strict clippy exit101은
기존 진단32건이며 이번 추가 진단은 제거했다. 관련 없는 전체 포맷/리팩터링을 하지
않았다. 테스트 binary SHA256 `53b3a130da5ecc56959f77401cf5cc7cb1d2ff789582bf0b1b9b1287ebb85a9c`,
production binary `eae1892f6a692c4bf4dbbf46e8d81348ef5f034748daa88172555cd1f7c200da`.
새 source/preparation을 동결한 후 독립 A와 실제 학습으로 이어간다.
최소 기준선/S4/S5/S6/Goal1 미수용.
허용된 로컬 근거는 `artifacts/rebind-consolidation-20260921-review/`의
`C0-INDEPENDENT-PARENT-REVIEW.md`, baseline, 71개 객체 보존 manifest다.
원본 모델·corpus·raw·사용자 자료는 게시하지 않는다.

## 2026-09-21 Learned binding expansion — 실행 완료, 전이 개선·최종 기준 미달

**STUDY_COMPLETE_QUALITY_FAIL**. 실행 source는
`b5e0d504a6a23ea8df9a7a1236690361cd13c86b`이며 정상 push 후 원격 전체 SHA
일치를 직접 확인하고 SMALL 실행 동안 고정했다. 아래 준비 단계의 NOT_RUN은
당시 상태이며, 이번 두 군은 각각 새1536회, 합3072회를 실제 실행했다.
두 final3584 모두 `Finished / FINAL_QUALITY_FAIL / resume=false`다.
이전 QE2048과 실패·Adam·raw·원문은 보존했다. 모델·loss·LR·tokenizer·framing
탐색이나 추가 재초기화는0이다.

### 같은 checkpoint의 전수 정상 생성

|모델 / panel|FULL|QUERY_BOTH|SWAP_BOTH|ALL4|Gold / foil / other|EOS / errors|
|---|---:|---:|---:|---:|---|---|
|부모2048 old512|472/512|216/256|217/256|96/128|472 /38 /2|512 /0|
|부모2048 dev512|294/512|77/256|101/256|21/128|294 /172 /46|512 /0|
|REPEAT3584 old512|509/512|253/256|253/256|125/128|509 /3 /0|512 /0|
|REPEAT3584 new1024|694/1024|227/512|289/512|83/256|694 /173 /157|1024 /0|
|REPEAT3584 dev512|292/512|84/256|110/256|26/128|292 /139 /81|512 /0|
|REBIND3584 old512|507/512|251/256|251/256|123/128|507 /5 /0|512 /0|
|REBIND3584 new1024|1004/1024|492/512|492/512|236/256|1004 /20 /0|1024 /0|
|REBIND3584 dev512|476/512|220/256|223/256|99/128|476 /25 /11|512 /0|

부모 old/dev는 기존 raw의 재검산이며 이번에 다시 생성한 전수 평가는 아니다.
부모 new1024는 NOT_RUN이다. 이번 고정 parent new64는 FULL43/64, QB16/32,
SB16/32, ALL4 4/16, EOS64/errors0이며 두 군에 공유했다.
REPEAT new1024는 미학습 준비 pool이므로 train fit으로 부르지 않는다.

같은 dev128 skeleton의 paired 비교에서 REBIND는 REPEAT 대비 row 정답
194개 증가·10개 감소, ALL4 73개 증가·0개 감소다. FULL 차이는184/512,
ALL4 차이는73/128이다. 전체512행을 독립표본으로 취급하지 않았다.
REBIND는 실제로 넓어진 결합을 배웠고 미학습 dev에서도 공동 정답이 증가했다.
다만 이는 이번 고정 seed·자료·예산의 비교이며 내부 회로나 일반적인 언어 능력의
증명이 아니다. REPEAT도 old fit은 개선됐으므로 '학습을 전혀 못 한다'는 해석은
맞지 않는다. REBIND의 old FULL은 REPEAT보다2개 낮고 공동 기준은 미달이다.

REPEAT old gate는 PASS지만 dev gate는 FAIL이다. REBIND old는
FULL507<508/QB251<252/ALL4 123<124, new는1004<1016/492<504/236<248,
dev는476<488/220<232/99<116이다. 후보를 선정하지 않았고 confirmation은
**NOT_OPENED**다. 중간 최고점 선택·다른 후보 재시험·추가학습은0이다.

### 손실·질문 신호를 정상 생성과 분리

|모델 / panel|Mean abs(c)|Mean d|Mean min margin|Digit NLL|EOS NLL|Binary NLL|
|---|---:|---:|---:|---:|---:|---:|
|부모 old512|1.503517|3.602308|2.098791|0.222023|0.00002404|0.198583|
|부모 dev512|2.155598|1.074871|-1.080727|1.400022|0.00002422|1.090279|
|부모 new64|1.931728|2.380773|0.449044|0.737585|0.00002481|0.555790|
|REPEAT old512|1.360741|9.833626|8.472885|0.013446|0.00000748|0.012323|
|REPEAT new1024|3.399134|4.427685|1.028551|1.798513|0.00000713|1.077615|
|REPEAT dev512|3.794303|2.912587|-0.881716|2.503964|0.00000740|1.728593|
|REBIND old512|1.807615|8.328381|6.520766|0.052767|0.00001376|0.047435|
|REBIND new1024|1.863043|8.175983|6.312940|0.057155|0.00001377|0.053586|
|REBIND dev512|2.294474|6.828476|4.534001|0.248019|0.00001394|0.201576|

이 수치는 실제 teacher raw의 full-vocabulary 첫 digit/EOS NLL 및 gold/foil
margin 재집계다. Binary NLL은 두 후보를 재정규화한 별도 값이다. 관측 gradient나
gS backward를 optimizer에 섞지 않았고 이번 SMALL 추가 관측 backward는0이다.
Dev의 같은 query 출력은 부모120쌍→REPEAT85/REBIND25, 같은 assignment 출력은
80→55/20이다. REBIND의 dev NLL·공동 정답 개선을 함께 관측했지만 최종 gate를
대신하지 않는다. 분포 전체와 pair별 값은 독립 scratch TSV/log에 보존한다.

### 실제 단계·사용량·재시작

|군 / absolute step|old64 FULL/QB/SB/ALL4|new64 FULL/QB/SB/ALL4|dev64 FULL/QB/SB/ALL4|
|---|---|---|---|
|REPEAT2304|60/28/28/13|41/14/19/6|40/13/15/4|
|REPEAT2816|64/32/32/16|44/15/19/7|43/13/17/4|
|REBIND2304|59/27/27/11|51/19/20/8|44/13/16/4|
|REBIND2816|60/28/28/12|58/26/27/12|50/18/22/6|

위 중간값은64/32/32/16 분모이고 최종 전수와 섞지 않는다. 각 군은 별도 process의
2049→2304→2816→3328→3584 다섯 segment를 거쳤다. 첫1회는1536에 포함한다.
실제 첫 batch가 같아 두 군2049의 loss0.1987816244와 weight delta L2
0.20340192 및 weights hash가 일치했다. 이후 새 key 노출 tape가 달라진다.
모든 신규2049..3584 trace에 gap/중복 없이 LR3e-4, Adam 누적 clock·sampler,
sample_indices·target·EOS·input shape가 고정 정책과 일치한다.

* REPEAT: actual old512×24회/new0회; input1,781,760/target24,576.
* REBIND: actual1536행×8회; input1,781,760/target24,576.
* 합3072 optimizer, committed/executed input3,563,520/target49,152;
  discarded input0/target0, padding0. 부모 누적 input2,375,680/target32,768은
  신규 예산에 합산하지 않는다. 각 final native 누적은4,157,440/57,344다.
* 예정 평가 generation4864/teacher4864행과 parent generation80/teacher64행을
  실행했다. 구현자 endpoint parity32회도 모두16/16 일치, teacher/optimizer0이다.
  독립 endpoint parity32회도 각각16/16, exit0이다. 총 신규 SMALL generation
  5008/5504, teacher4928/5120, optimizer3072/3072이며 재시도0이다.
  이 teacher 경로는 매번 batch1의 한 sample을 한 forward로 평가한다. 따라서
  확인된 teacher4928행은 teacher forward4928회이며, gold/foil logits는 같은
  반환 tensor에서 읽는다. 추가 foil forward나 관측 backward는0이다.
* TINY는 optimizer15/generation56/teacher 예산차감249행. teacher241행은 확인,
  kill 미반환8행은 UNKNOWN으로 보존·보수 차감했다. scalar finite-difference0.
* 정상 반환된 모든SMALL평가에 오류0. final3584에서 학습 budget을 닫았으며
  시간/취소/NaN/I/O/UNKNOWN으로 이어간 예산이나 재시도는 없다.
* Runtime work ledger의 segment/observation receipt elapsed 합은1681.6739795초,
  내부 RunControl elapsed 합은1679.553183293초로 active10800초 상한 이내다.
  사전 파일/정책 검산·컴파일을 포함한 전체 wall time 또는 처리속도 benchmark로
  해석하지 않는다. 학습 로그의 최대 sampled RSS는
  6,612,720KiB이며16GiB 상한 이내다. 강제 선점/전체 process peak 측정은 아니다.

### Native identity와 실제 실행 증거

|Identity|REPEAT3584|REBIND3584|
|---|---|---|
|Physical native SHA256|13d75c269771743ad3df58bd8508265a8c855248d90106ba6db45cc8d94f7908|385600c2ad9ae99e496fa1b404aec192e2cb0cf1770fc1f6071b0bc75c469f11|
|Evaluation weight ID|f274862ba90f28158aa28e0deb1bfad5418781e5c916a1977c153b5634f7a496|543c242c03e142433414bfa912c8a6cfa8116cb5d5497f427eb2c1e64c1bd9c0|
|Adam136 digest|cd1aeaaf280b64cca0012d949c8d03989f02368680fcc0c227d75052b2fdd392|cd4d9ab576b59ddf48b667140a21dfa2b81f4924615a4e5b4652218f64de2d04|

최종 파일은 각 arm의 `segment-0004/final`이다. TRAIN_END의 `sha256` 표시는
manifest weights digest이며 위 physical file hash와 다르다. 기존 .r3m schema는
바꾸지 않았다. Source digest `95e07dba3de9008bb7c38bfa6031878a5ee4cb87398a2cf672408fb9fcfd3a2e`,
실행물 SHA256 `8719097f96d04bfaf2af15a1288544c88ed8bfa03e04d2890bb86d00b947ebe4`는
준비·독립 A·실행 동안 동일했다. test-support 없이 release로 빌드했다.

실제명령은 고정 실행물의 `fresh expansion-prepare`, `fresh expansion-parent`
(legacy16 및 `--new-pool`64), 각 군 `fresh run --root ARM`5회,
`fresh binding-parity --root ARM`, 독립 `fresh orbit-review-parity`, 최종
`fresh expansion-report --study STUDY`다. 모두 exit0이며
VECLIB_MAXIMUM_THREADS=1/OMP_NUM_THREADS=1, Rust1.98.1, locked/offline/accelerate다.
E1 RED의 의도된exit101과 기존 fmt/clippy FAIL은 아래 준비 검증 기록에 별도 보존했다.
최종 production report가 실제 양군 raw/teacher/종료/policy/native/parity를 다시
검산하고 dev ALL4 gain73/loss0, selected=null, confirmation=NOT_OPENED,
goal1_ready=false를 반환했다. 이는 새 generation/teacher가 없는 read 검산이다.
실제 stdout 전체는 `artifacts/binding-expansion-20260921-evidence/final-report.log`다.
해당 로그 SHA256는
`e9a9a0b0b12f94914623aa8e266aed52cfe021f2afb359d49982bb8153ea181e`다.

현재 변경 source는 기존 `src/fresh.rs`, `training.rs`, `binding.rs`,
`quality_recovery.rs`, `check_main.rs` 다섯 파일과 기존 계획·상태 문서다.
새 tracked source 파일은0이다. 실제 candidate diff는
`artifacts/binding-expansion-20260921-evidence/candidate.diff`이며 SHA256
`277a8e044ed2b59a186e4c73e9a40e05d47646f8f620ce091fc4aed5720536c0`이다.
인가된 준비·raw·checkpoint·원본 읽기 경로는 바로 아래 준비 기록에 명시했다.
개인 DB·봉인 confirmation·임시 지시문·원본/모델은 게시하지 않는다.

QS_R1_SOURCE/DYNAMIC와 INDEPENDENT_A는PASS, 실제 신규 학습·전수 재채점은완료다.
독립 E4/B numerical 검산과 final parity는PASS지만 후보자격은양군FAIL이다.
최종 독립 보고서는
`artifacts/binding-expansion-20260921-math-review/E4-B-FINAL-REVIEW.md`, SHA256
`d2b6bb872affc99177c915384656613897714da73dd831c258aaa37bff7f1a6f`다.
독립 실제32 generation과 별도 pure panel/native/trace/paired/usage 검산을
완료했으며 품질 승인용 review-b receipt는 발행하지 않았다.
UNSEEN_BINDING은개선관측/최종gateFAIL, MINIMAL_BASELINE=NOT_ESTABLISHED,
S4/S5/S6=NOT_ACCEPTED, GOAL1_READY=false, GOAL1_ACCEPTED=false다.

마지막 요구 대조에서 E0 원본/부모22개 hash 보존, E1 실제 no-call pause
RED/GREEN·실패 차단, E2 원자료/key-only/token/tape/policy·독립A,
E3 두 군 실제1536회·첫 저장/새 process·정해진 screen/전수 평가,
E4 실제 raw 재채점·노출·같은3584 비교를 확인했다. E5는 공동 gate 미달에 따른
NOT_RUN_PREREQUISITE이며 누락된 학습으로 채우지 않는다. 추가 source/model 변경은0,
임시 전달문을 source/빌드/영구 문서의 의존으로 추가하지 않았다.

## 2026-09-21 Learned binding expansion — 수리·준비 검증, 학습 전

R3-LEARNED-BINDING-EXPANSION-1.0은 종료된 QE2048의 weights/Adam을 읽는
별도 연구다. 이전 FINAL_QUALITY_FAIL/resume=false 및 모든 원자료는 보존한다.
이전 A/B PASS 보고와 최신 독립 QS-R1 미수용 보고는 다른 시점의 증거다.
이번 수리가 과거 점수의 원인을 입증하거나 모델 품질을 회복했다는 뜻은 아니다.

### 이번 실행 증거

* QS-R1 수정 전 실제 subprocess 시험은 exit101로 재현했다. 관측 첫 forward
  이전 순수 TIME_BUDGET은0회 호출인데 다음 process admission이 거부됐다.
* 수정 후 동일 직접 시험1 PASS: 새 immutable attempt로 재개하고 연속 실행과
  weights/Adam/clock/sampler/input/target 및 train/dev raw가 같았다.
* 독립 검토자도 RED, GREEN 및 실패 경계10개를 실제 실행했다. finish 누락/손상,
  orphan, 잘못된 samples, advanced cursor, timeout+cancel, timeout+I/O,
  partial return, commit 이후 중단, entered/unreturned kill은 차단됐다.
  독립 E1 보고 SHA256는
  `af01cb4c579f0e87bb3c6c49943d562044b0737b4e7529de50f0f0d7e17c1837`이다.
* 기존 observer off/on·새 process1+1·평가만 재개2+0 회귀1 PASS.
* 새 native pool/tape/panel writer→publisher→reader→strict score 시험 PASS.
  64/512/1024/1536행, 누락/중복/순서 변경, key 외 변경, target 불일치를 검사했다.
  새 family의 실제 TINY teacher1행에서 target NLL/gold/foil logits도 확인했다.
* 기존 checker `quick --binding-expansion`은 check 및 지정 test3개를 실행해
  CHECKED_SCOPE_PASS, source_unchanged=true다. 0-test를 PASS로 세지 않았다.
  release build/check와 diff whitespace 검사도 통과했다. fmt 전체 검사는 기존
  compact source 포맷 부채로 실패했다. strict clippy는 기존32개 진단으로 실패했고,
  새 구현 블록의 경고는 수정했다. 무관한 전체 소스를 재포맷/리팩터링하지 않았다.

여기까지 실제 합계 TINY optimizer15/generation56/teacher 예산차감249행이다.
teacher241행은 반환/종료 기록으로 확인됐고, kill 시험의 미반환8행은 UNKNOWN으로
보존하고 보수적으로 차감했다. scalar finite-difference0, SMALL optimizer/
generation/teacher0이다. 단순 native load/init, fixture writer와 raw 재집계를
모델 호출로 세지 않는다.

독립 A는 실제 preparation의 전수 data/token/target/tape/parent native·Adam 검산과
독립 E1 동적 증거 결속 후 PASS를 발행했다. 보고서는
`artifacts/binding-expansion-20260921-math-review/ACTUAL-A-PREPARATION-REVIEW.md`,
SHA256 `80aae5d9a68cc4a2306c912cd656b4b977f64307f5d543f2fc6564761e8a6d2e`이며,
확정된 `review-a.r3b` SHA256는
`00cd957b44a8889b49302918b6dba4080cf1f574f55649f42e8facd96b3d9a98`이다.
이 검토의 모델 호출은0이다. 이후 부모 관측/학습/품질 수용은 별도 기록한다.

### 실제 준비 identity와 로컬 인계 경로

|준비 항목|SHA256 또는 실제 값|
|---|---|
|Compiled source digest|95e07dba3de9008bb7c38bfa6031878a5ee4cb87398a2cf672408fb9fcfd3a2e|
|Production executable|8719097f96d04bfaf2af15a1288544c88ed8bfa03e04d2890bb86d00b947ebe4|
|Preparation|1288fad5dca2b2d317c279b20b33ac9a25e01c3ac5efd9de8413caaf4967dba7|
|Selection|2117510c87a7e9345e86578389645ac6f667aaf8f3bcac207c10a6b3122079dc|
|Shared native corpus|c0462f4861a1c67d9509305e9841bc6a07753a213ef92d253dbe918d7126e06a|
|Shared metadata|c3428b8ee012d95b210350e12700e77821640f3cfb4d21438fb33fb6c8a816fc|
|REPEAT policy / tape|e45bbb02476d9fe7df279694a1f7970cf1c7a8d566c5f54736f1c1af033be2d9 / d4cf2c9f1ad0040e5f3fc1720ff1d2feb3498bedc642071a98091ae9a0531210|
|REBIND policy / tape|f3741bc854f9e1ea0b9cf24bb8e92362043a70b1542f5bb574cf4abcf6f1c4ca / 2ce8a84f5f0198fd8da137ecd6912e4682588a5c064b473508277344ba9b2089|
|Common parent physical|e9b6c796bf7eccf292b46b040592450339dbe679014cbee46c5bf2952c9e1ab0|
|Common parent Adam|5d3b93835657a75ae44b27a27fdd105a8258a5fec6db2760bac89c5b30c66762|

공통 pool1536은 원래512행을 보존하고 key-only1024행을 추가했다. public finite
reservation을 제외한 후보940개에서 필요한256개를 고정 순서로 선택했고 모든
value-pair의 용량이 충분했다. confirmation 내용/정답은 열지 않았다. 같은1536
pool에서 REPEAT512행×24회, REBIND1536행×8회이며 각1536 updates의 예정
input1781760/target24576은 actual token/tape 전수검사 결과다. 아직 학습량이 아니다.

인가된 로컬 검토 범위는 `artifacts/binding-expansion-20260921-study/`의
preparation/selection, 두 arm의 native corpus/metadata/plan/initial 및 이후 생성할
해당 연구 raw/checkpoint다. 실행물은 `artifacts/binding-expansion-20260921-executable`,
명령·검사 증거는 `artifacts/binding-expansion-20260921-evidence/`, checker 결과는
`artifacts/binding-expansion-20260921-quick/`, 독립 보고는
`artifacts/binding-expansion-20260921-review/`와
`artifacts/binding-expansion-20260921-math-review/`에 있다. 이 artifact들은 게시하지 않는다.
이전 parent `artifacts/query-signal-20260921-study/QE/`는 읽기 전용이다.
운영 DB/사용자 원문과 봉인 confirmation 내용은 이 인계의 열람 범위가 아니다.

## 2026-09-21 Query signal2048 — 제한 실행 완료, 학습 적합 개선·공동 품질 미달

R3-QUERY-SIGNAL-CONVERGENCE-1.0의 실제 source는
**a8c7f0fa261b2bf6804e02783d6f78bdcb0c7b28**이다. 독립 A를 받은 뒤 정상 push와
원격 SHA 일치를 확인했고, 아래 SMALL 실행 동안 source/binary 변경은0이다.
1024에서 full gate가 미달해 사전등록한2048까지 진행했다. 최종
**FINAL_QUALITY_FAIL / Finished / resume=false**이며 추가 예산은 없다.
실행·저장 성공과 모델 품질을 구분한다. 아래 준비 기록의 NOT_RUN은 당시 상태다.

|실제 identity|값|
|---|---|
|Source / compiled-source digest|a8c7f0fa261b2bf6804e02783d6f78bdcb0c7b28 / 1892e1c5575658e41c6318b32018e1a8f3d052e325931b778da999c57fae2f7e|
|Production executable SHA256|c037ec25664e4a2945628bff0724a2ddf96a4da7e5a1a2f648a327857305d49f|
|Parent QE512 physical SHA256|e6895cbe02ab5ecdef0f300d953b60749358e0d6f8ba2d4d31344eece8f15fbb|
|BOTH corpus physical SHA256|e73fddef722c4cd1c249d9896b49dd7e1dcae96bc39ad0eb1e3438ee6c5a51ac|
|Tokenizer content ID|ec945ee5f3cbd87992bdfa13f199de2a671b94b64337dff04d85e01d982ab9ef|
|Plan physical SHA256|c7a6d0605e31e7714b15f3391250636dc7f6ed52934bdc0d3a1db74d62d0ab7c|
|Canonical native/receipt policy digest|a427cf693094f790736874c5bb2b005bb998e20b1dcf9dae916ae75fbf97c04d|
|Serialized2048-row tape digest|df3ea0fd4867e3df355a9badc9ad68fe737d1c269b9ab790b52767461b6f9cd8|
|Framing / objective|QE native-role-bytes-v1 / unchanged full-vocabulary digit+EOS CE, first-target1|
|Tape / optimizer|original512 tape repeated3 times; actual1536 traces; inherited Adam clock512→2048; constantLR3e-4|
|Final native physical SHA256|e9b6c796bf7eccf292b46b040592450339dbe679014cbee46c5bf2952c9e1ab0|
|Final tensor content ID|341577e026a62675c670199f59e0c198203684f983a65d7d0f2eed6c7157b231|
|Final evaluation weight ID|0111e4df7f0efe06aa9147b660da1e39221ec25daf92a785ec1f4a0cceb354fc|
|Final Adam136 digest|5d3b93835657a75ae44b27a27fdd105a8258a5fec6db2760bac89c5b30c66762|
|Final decision SHA256|8daf8ca1c5715a61960a7d7e55b706d1303b2be686f01789960fa5fc5eb3995d|
|Final segment receipt SHA256|8112280bb73907c378a283af9f25a6702a225eaaef6e958ae98ceb5b3ad6e70f|

### 같은 endpoint의 전수 결과

|Step / panel|FULL /512|QUERY_BOTH /256|SWAP_BOTH /256|ALL4 /128|Gold / foil / other|EOS / errors|
|---|---:|---:|---:|---:|---|---|
|512 parent train|256|0|31|0|256 /256 /0|512 /0|
|512 parent dev|255|1|28|0|255 /257 /0|512 /0|
|1024 train|269|17|35|1|269 /243 /0|512 /0|
|1024 dev|253|10|31|0|253 /259 /0|512 /0|
|2048 train|472|216|217|96|472 /38 /2|512 /0|
|2048 dev|294|77|101|21|294 /172 /46|512 /0|

Train gate는 FULL508/QB252/ALL4 124, dev gate는488/232/116이며 둘 다 미달이다.
512의 원래 결과는 기존 raw를 재사용했다. 이번 최종 정상 생성은 별도 연구의
저장된 동일2048 모델이다. 원본 framing512 decision과 EQ 결과를 수정하지 않았다.

아래는 같은 metadata 고정64만의 경과다. 위 전수와 분모를 섞지 않는다.

|Step|Train FULL /64, QB /32, SB /32, ALL4 /16|Dev FULL /64, QB /32, SB /32, ALL4 /16|
|---|---|---|
|512 parent|32 /0 /5 /0|31 /0 /7 /0|
|768|32 /0 /3 /0|32 /1 /4 /0|
|1024|33 /1 /4 /0|34 /3 /6 /0|
|1536|52 /20 /22 /7|36 /8 /11 /2|
|2048|59 /27 /27 /12|40 /11 /14 /3|

### 질문 신호와 실제 갱신

|전수 panel|Mean abs(c)|Mean d|Mean min margin|Digit NLL|EOS NLL|Binary NLL|
|---|---:|---:|---:|---:|---:|---:|
|512 train|0.969794|0.001144|-0.968650|0.924161|0.000330|0.848874|
|512 dev|0.957095|0.000483|-0.956613|0.959152|0.000328|0.848460|
|1024 train|0.815976|0.076844|-0.739132|0.805237|0.000093|0.775571|
|1024 dev|0.802243|-0.005127|-0.807371|0.854602|0.000092|0.809708|
|2048 train|1.503517|3.602308|2.098791|0.222023|0.000024|0.198583|
|2048 dev|2.155598|1.074871|-1.080727|1.400022|0.000024|1.090279|

이번에는 train의 질문 판별·공동 정답이 늦게 크게 늘었다. 따라서512의 낮은
공동 정답만으로 학습 불가능이나 구조 한계를 확정할 수 없다. Dev 공동 정답도
1→77, ALL4 0→21로 늘었지만 train보다 크게 낮고, digit NLL은 부모보다 악화됐다.
전체 후보 밖으로 간 정상 오답도 dev46개다. EOS만 좋아진 결과는 아니지만,
일반화 기준선은 여전히 미확립이다. 동일 정책의 시간·노출 관측이며 QE/EQ의
동등 예산 비교나 장기 인과 증명이 아니다.

2048의 두 margin 양수는 train218쌍/dev91쌍, 실제 QUERY_BOTH는216/77이다.
양수 all4는97/28, 실제 ALL4는96/21이다. 두 후보 diagnostic을 정상 전체
vocabulary 점수로 대체하지 않았다. c/d/m0/m1/최소 margin의 전체 분포·orientation은
독립 scratch의 panel-2048.log와 pair-details TSV에 보존했다.

|새 update / 절대 step|S before → after|gS·delta|실제 S 변화 − 1차 근사|
|---|---|---:|---:|
|1 /513|0.002266467 → 0.002301425|0.0000374422|-0.0000024841|
|8 /520|0.002264887 → 0.002313793|0.0000614670|-0.0000125613|
|32 /544|0.002262443 → 0.002295613|0.0000323299|0.0000008401|
|128 /640|0.003302604 → 0.003284752|NOT_RUN backward|NOT_APPLICABLE|

고정 train16의 별도 graph를 사용했고 관측 gradient를 optimizer에 넣지 않았다.
첫 세 국소 update의 dot/실제 S 변화는 양수이나 이것이2048까지의 원인을
증명하지는 않는다. 독립 검토는 저장된 요약의 수치·trace delta norm 연결과
source를 검산했다. 원 gradient tensor를 새 forward로 재생성한 검증은 아니다.

### 실제 사용량·검증·남은 범위

SMALL은 old512 + **new1536 = absolute2048**. 네 새 process의 optimizer는
1/511/512/512이며 첫1회 저장 후 복원했다. 새 committed/executed input
**1,781,760**, target**24,576**, padding/discarded/uncommitted0이다. 각 train행
24회 추가 노출, 부모 포함32회이며 tape 순서를 실제 trace와 대조했다.

예정 평가 generation2304 + parent16 + 구현자 final16 + 독립 final16 = **2352**.
Teacher/sample-forward **2432** = 평가2304 + train 관측128이다. 후자는16개
microbatch forward, 추가 backward6이며 정상 training forward/backward1536과
별도다. 모든 생성/teacher 호출은 반환됐고 UNKNOWN0이다. TINY 누적13 updates,
generation56, teacher sample200 및 scalar finite-difference16은 실패 시험까지
포함한 별도 회귀 비용이다. SMALL/TINY를 합쳐 모델 학습량이라고 하지 않는다.

예정 학습/평가/관측의 source-derived forward10,768, backward1,542도 독립
검산했다. Generation 경로의 두 prefill과 한 decode, teacher 한 forward를
실제 반환 token/행 수에 적용한 호출 집계이며 kernel 계측은 아니다.
부모/최종/독립 parity와 TINY는 이 subtotal에 포함하지 않는다.
Parity48회까지 포함한 SMALL source-derived forward는10,912, backward는1,542다.

학습 segment의 receipt elapsed 합864.304263166초는 평가·저장을 포함한다.
최대 관측 RSS2,762,864KiB, 각 segment900초+cleanup120초와 총7200초 내 종료했다.
이것은 학습만의 처리량이나 전체 forward별 성능 측정이 아니다. 마지막 durable
checkpoint는 `artifacts/query-signal-20260921-study/QE/segment-0003/final`이다.
세 parity command까지 합한 실제 receipt elapsed는868.223972583초다.

직접 관련 고유 회귀5개 최종 PASS, 독립 순수 지정 실행3개 각각1 PASS.
초기 compile 실패와 첫 TINY process 실패는 보존했고 PASS로 세지 않았다.
실제 네 학습 command, 부모16/최종16/독립16 parity, production signal-report는
모두 exit0이며 parity는 각각16/16이다. Source 수리와 독립 A 이후의 실제
SMALL 실행에서 취소·저장 실패·NaN·UNKNOWN은 없었다.

독립 B는 실제 raw/native/tape/호출 원장과 최종 재생성을 검증해
INDEPENDENT_RESULT_INTEGRITY=PASS, QUALITY_GATE=FAIL로 닫았다.
`artifacts/query-signal-20260921-review/FINAL_B_REPORT.md` SHA256은
c27c14e3aeb657a17dba2171f7af43a95965afb60b0077b631d7effb628ba445,
실제 review-b.r3b는1cb68d99b5069c88123fe1f0e1df6de78e453d2eec5c4832fccbd24fa7ffa9e7이다.
별도 수학 검산 FINAL2048-MATH-REVIEW.md의 SHA256은
c8e43d369051fb9a9fdd372db10e6dd94e0b0f01ecd933c0c2768a6f57f8df37이다.
두 검토 모두 새 SMALL 학습0이며 독립 생성은 B의 지정16회뿐이다.

CODE_LOW_FIX=PASS, OBSERVATION_NONINTERFERENCE=PASS, STUDY_EXECUTION=COMPLETE.
TRAIN_BINDING=FAIL, DEV_BINDING=FAIL, CONFIRMATION=NOT_RUN_PREREQUISITE(미개봉),
MINIMAL_BASELINE=NOT_ESTABLISHED, S4/S5/S6=NOT_ACCEPTED,
GOAL1_READY=false, GOAL1_ACCEPTED=false. 제품 기본 모델은 교체하지 않았다.

실제 candidate diff, 준비물·native·raw·로그·검토 위치는 로컬
`artifacts/query-signal-20260921-evidence/HANDOFF.md`에 정리한다. 새 추적 파일은0이며
source4개와 기존 상태/계획2개만 변경했다. 모델·corpus·큰 raw·scratch는 게시하지 않는다.

## 2026-09-21 Query signal — QE512 보존 확인 및 제한 continuation 준비

R3-QUERY-SIGNAL-CONVERGENCE-1.0은 기존 framing512 종료를 고치지 않는 별도
연구다. 기준 source32df6dcb1c86020b9fc5402ff3b8bc32f3180f0e 이후 시작 HEAD는
a9c652e2eccbd0067d17d33ea4c3e1f7fd9d673a이며 기존 변경은 상태/계획 문서2개뿐이다.
사용자의 미추적 `.DS_Store`와 모든 원본을 보존했다. 설치된 Rust/Cargo1.98.1,
locked/offline/Accelerate를 사용하고 모델·의존성을 내려받지 않았다.

Q0 독립 확인은 실제 QE512 physical
e6895cbe02ab5ecdef0f300d953b60749358e0d6f8ba2d4d31344eece8f15fbb,
tensor content7d5dce3d545f7fc860de99113e7a74760b450e91f4de60d20cfff8fa60895e6e,
Adam eb258addfd2e5841b0ce946e232cc70cdde1a7f0cc8e324cc9f84bcdbd3eb64e,
step/cursor512·QE·tokenizer562·기존 corpus/tape를 검산했다. 직접 관련50파일의
보존 manifest를 남겼다. 이전 CLOSE_NO_JOINT_SIGNAL과 segment 종료는 불변이다.

Q1은 실제 endpoint를 받는 순수 판정으로1024의 extend=false와 최종 문자열을
연결했다.512의 기존 truth table·품질 gate·candidate 순서는 유지한다.
작은1024 endpoint fixture는 실제 publisher/reader/comparison 검증을 통과하며
누락·mixed step·잘못된 candidate/action을 거부한다. 이 fixture는1024 학습이 아니다.

Q2 기존 raw 독립 재집계는 추가 모델 호출0이다. 각 고정64와512 전수를 나눴다.
QE train512는 mean|c|0.969794, mean|d|0.014082, 양 margin 양수0/256이다.
EOS/후보 밖 확률의 감소와 두 질문 공동 정답의 부재를 구분한다. 이를 단일 원인이나
구조적 불가능의 증명으로 쓰지 않는다. Scalar finite-difference는 총16계산,
모델 호출0이며 d=0 미분−0.5·orientation·상수 이동·분모를 확인했다.

첫 실제 TINY process 시험은 관측이 stop_after보다 먼저 시작되어 미실행 다음
step의 probe를 남기는 오류를 재현했다. 실패와 원본을 보존하고 관측 시작을
중단 검사 뒤로 이동했다.8행 관측의 budget 예약도 backend1회와 혼동하지 않도록
원자적8행 예약으로 수정했다. 독립 source 검토의 두 finding을 함께 보존했다.
수정 후 observer-off2/observed1+1/평가-only2+0의 weights·Adam·clock·token·raw가
bitwise 일치했다. 기존 EOS 중심 numeric fixture의 S/gS=0은 품질 증거가 아니다.
TINY 누적 실제 optimizer13/generation56/teacher sample-forward200, 관측
microbatch18·추가 backward10이다. 이 값은 실패 시험을 포함하며 SMALL과 별도다.

직접 실행: endpoint truth table, scalar/경계 fixture, 실제 published comparison,
TINY process 재개, 기존 record capacity writer→publisher→reader(0..512 명시
개수)만 실행했다. 처음 미완성 소스 compile과 TINY 실패를 PASS에 합산하지 않았다.
새 source/준비물 독립 A는 실제 native payload(Adam 포함)·tape·counts와 순수
회귀3개를 별도로 확인해 PASS했다. 보고서 ACTUAL_PREREVIEW.md SHA256은
b65acead143def18385805723a6c9e5d75b77937bf90b0330e34d35195bf1cd4다.
Compiled source digest1892e1c5575658e41c6318b32018e1a8f3d052e325931b778da999c57fae2f7e,
production executable c037ec25664e4a2945628bff0724a2ddf96a4da7e5a1a2f648a327857305d49f,
preparation a5e49a4bd49ecc3e2abfcf8b237047f108bd1286336831866f7e64ab0bf4c2b0,
QE policy c7a6d0605e31e7714b15f3391250636dc7f6ed52934bdc0d3a1db74d62d0ab7c에 묶였다.
실제 SMALL 실행은 아직 NOT_RUN이다. Confirmation은 미개봉이며 S4/S5/S6 및
GOAL1_READY/GOAL1_ACCEPTED=false를 유지한다. 이 상태 commit 이후 source를 동결한다.

로컬 인가 증거: `artifacts/query-signal-20260921-evidence/`,
`artifacts/query-signal-20260921-review/`,
`artifacts/query-signal-20260921-math-review/`,
`artifacts/query-signal-20260921-tiny-01/`(실패 포함),
`artifacts/query-signal-20260921-tiny-02/`(수정 후 통과).
원본 checkpoint/corpus/raw/실행 바이너리는 게시하지 않는다.

## 2026-09-21 Causal framing512 — 제한 학습 종료, 공동 선택 품질 미달

R3-CAUSAL-FRAMING-BASELINE-1.0의 독립 준비 A 후 QE/EQ를 각각 실제512회
학습했다. 동일한 무학습 A tensor/fresh Adam에서 첫1회 저장→새 process511회를
수행했으며 네 학습 command 모두 exit0이다. Source candidate는
**32df6dcb1c86020b9fc5402ff3b8bc32f3180f0e**이고 학습 전 정상 push/원격 SHA
일치를 확인했다. 이후 source 변경0이며 동결 production executable SHA256은
**4a77b920e898016e1d1da9daf8414fa062871831f0c1b2f4d75c1b982d18228d**다.
학습 기능은 test-support 없이 Rust1.98.1/locked/offline/Accelerate로 빌드했다.
이 결과를 추가하는 문서 commit은 실행 source와 별도다.

### 같은 saved endpoint의 실제 결과

|Arm / panel|FULL /512|QUERY_BOTH /256|SWAP_BOTH /256|ALL4 /128|Gold / foil / other|EOS / errors|
|---|---:|---:|---:|---:|---|---|
|QE train|256|0|31|0|256 /256 /0|512 /0|
|QE dev|255|1|28|0|255 /257 /0|512 /0|
|EQ train|255|3|43|0|255 /257 /0|512 /0|
|EQ dev|253|4|54|0|253 /259 /0|512 /0|

모든 최종 출력은 정상 digit+EOS이며 오답은 다른 제공 기록의 값이다.
두 군 모두 train부터 ALL4=0이다. 따라서 새로운 조합으로의 일반화 문제만으로
설명할 수 없고, 이번 예산에서 학습 자료의 공동 선택 조건도 달성하지 못했다.
전체 정답 약50%·정상 EOS·학습 command 성공을 선택 능력의 성공으로 바꾸지 않는다.
같은 두 query의 출력이 같은 쌍은 QE train256/256·dev253/256,
EQ train249/256·dev245/256이다. EQ에서 일부 응답이 바뀌었으나 공동 gate는 미달이다.

|고정64 표본 step|QE train FULL|QE dev FULL|EQ train FULL|EQ dev FULL|
|---|---:|---:|---:|---:|
|0|0|0|0|0|
|128|8|6|9|8|
|256|29|26|32|30|

Step0의 네64행 panel은32-token 길이 종료/EOS 미학습으로 각각 오류64를 기록했다.
이는 반환된 실제 초기 출력이며 최종 오류0으로 덮어쓰지 않았다. 이후 평가의
생성 오류는0이다. 초기 오류는 NaN·저장 실패·사용자 취소 또는 미반환이 아니었다.

|Arm / panel|Digit full-vocab NLL|EOS NLL|Gold/foil binary NLL|
|---|---:|---:|---:|
|QE train|0.9241609537|0.0003295292|0.8488742270|
|QE dev|0.9591515741|0.0003284439|0.8484601666|
|EQ train|0.8915289384|0.0003977159|0.8147429237|
|EQ dev|0.9479311485|0.0003984837|0.8596551532|

ln2와 비교 가능한 값은 마지막 이진 재정규화 열뿐이다. EOS NLL 감소가
정답 digit 선택의 성공을 뜻하지 않는다. Dev의 EQ−QE paired gain134/loss136,
FULL 차이−2/512=−0.390625 percentage points. 비교 단위는128 skeleton이며
표준오차0.732844674pp, 단일 seed의 정규근사95% 구간은[−1.82700056,+1.04575056]pp다.
QUERY_BOTH gain4/loss1, SWAP_BOTH gain36/loss10, ALL4 gain0/loss0이다.
512행을 독립 표본으로 취급하지 않으며 입력 순서의 총효과에서 causal 경로,
RoPE 거리, recency 중 하나의 기전을 분리했다고 주장하지 않는다.

### 종료·보존·실제 사용량

양군512 전수와 QE의 이전 BOTH 재현을 확인한 production `framing-decide`는
exit0, **CLOSE_NO_JOINT_SIGNAL**이다. 어느 군도 train QB>=64 및 ALL4>=16,
또는 dev QB>=32 및 ALL4>=8 조건을 만족하지 않았다. 조건부 추가512×2는
**실행0**이다. full gate도 미달하여 confirmation은 NOT_RUN_PREREQUISITE이고
미개봉 상태를 유지한다. LR/seed/loss/core 탐색·세 번째 arm·기존 run 재개0이다.

원 segment의 TrainingPending/resume=true/TRAINING은 단계512의 저장 상태다.
이를 Finished로 고치지 않았다. 별도 immutable framing-decision.r3b가
이 연구의 추가 update 권한을 차단하며 SHA256은
**1ca5796529ac6b2b015724b05a1b6c77e9c3065346917894172b5aaeea9782ab**다.
기존 예산이나 실패·종료 receipt는 수정하지 않았다.

각 arm 실제512 committed optimizer, input593920/target8192, unique train512의
각8회 노출이다. 합계 **1024 updates / input1,187,840 / target16,384**이며
padding·discarded·uncommitted input/target 모두0이다. 실제 draw/LR f64 bits,
Adam clock, clipping, CE와 objective 동일성, finite gradient/delta를 독립 검산했다.
warmup32 이후3e-4 고정이며 새 process에서 schedule/Adam을 초기화하지 않았다.

각 arm 평가 generation1408/teacher1408, 합계2816/2816은 모두 RETURNED이며
NOT_INVOKED0/UNKNOWN0이다. 초기 길이 종료도 RETURNED에 포함한다.
기존 BOTH parity16, 구현자 QE/EQ parity 각16, 독립 B QE/EQ parity 각16은
별도 실제 generation80이며 모두16/16 raw 일치, optimizer/teacher0이다.
따라서 SMALL 총 generation2896/teacher2816, optimizer1024다.
Scalar/finite-difference update는0이다. TINY는 실패한 직접 회귀의 사용량도
포함해 optimizer26/generation208/teacher208이며 SMALL과 합산하지 않는다.

내부 kernel counter를 새로 계측하지 않았다. 보존 raw와 실제 호출 경로에서
도출한 SMALL forward는 generation16368(평가16128+parity240), teacher2816,
training1024로 총20208, backward1024다. DERIVED_FROM_EXISTING_LOGS/source이며
측정 counter라고 하지 않는다. TINY는 training forward/backward26씩,
prefill416/teacher208은 소스·로그에서 도출되지만 임시 per-token raw가 남지 않아
decode-forward 정확 횟수는 UNKNOWN이다. TINY total forward는650+미확인 decode로
남기며650회로 확정하지 않는다. 이는 미반환 모델 호출 UNKNOWN과 다른 계측 한계다.
독립 부록 artifacts/causal-framing-20260921-math-review/TINY-CALL-ACCOUNTING-APPENDIX.md
SHA256 f82f83159829d12f79d1a6f96ba0735306f30b456b11bf82dee9851351dbd053에
실패 회귀·평가-only까지 포함한 실제 호출 합계와 근거 한계를 기록했다.

네 학습 command wall은 QE31.46+351.95초, EQ37.15+361.17초, 합계781.73초다.
이 시간은 모델 준비·저장·평가를 포함하며 순수 optimizer 성능 측정이 아니다.
Receipt가 계상한 네 segment active 합은739.773222626초다.
학습 command 최대 RSS는 QE1,841,004,544/EQ3,518,906,368bytes로 관측됐다.
이를 단일 통제 자원 benchmark나 S6 수용으로 사용하지 않는다. 재채점·자료
검증·재생성 command 시간은 학습 시간과 별도로 실행 로그에 남긴다.

### 재현·독립 검산과 실제 경로

QE512는 이전 BOTH512와 actual tensor content 및136 Adam tensor가 bitwise
동일하다. Step/sampler/input/target도 같다. Production 검산의 최종1024개 raw가
일치했고, 독립 비교에서는 중간 평가까지 포함한1408개 raw의 tokens/actual/error/
finish/completed가 전부 일치했다. 파일 전체 hash는 새 framing policy 때문에
다르며 tensor content 일치와 구분한다. 새 QE/EQ corpus·tokenizer는 원본과
byte 동일하고 token block 순서만 다르다. 최종 모델은 다음 파일에 보존한다.

|Arm|실제 마지막 durable 경로|Native file SHA256|Tensor content ID|
|---|---|---|---|
|QE|artifacts/causal-framing-20260921-study/QE/segment-0001/final|e6895cbe02ab5ecdef0f300d953b60749358e0d6f8ba2d4d31344eece8f15fbb|7d5dce3d545f7fc860de99113e7a74760b450e91f4de60d20cfff8fa60895e6e|
|EQ|artifacts/causal-framing-20260921-study/EQ/segment-0001/final|3ff8a4249ad833d586a205abef7f1a7b4522826e21bf291e3e4abb1974065163|7e717911aaefbfdd3df5d81e1110ef8f62bd061fcdf63b2da7ddf0ec3389beec|

`eval-0512-{train512,dev512}.r3rows`와 같은 접두어의 teacher/receipt/native가
각 arm에 있다. Trace는 각 segment의 updates.r3rows, 과정 로그와 source diff는
artifacts/causal-framing-20260921-evidence에 보존한다. Native corpus.r3cor,
metadata.r3b, tokenizer.r3b, initial.r3m, plan.r3b 및 study preparation/review-a/
decision은 artifacts/causal-framing-20260921-study에서 읽을 수 있다.
이 원자료·모델·scratch는 Git 게시 대상이 아니다.

독립 수학 검산은 다섯 Rust reader가 모두 exit0이며 모델 호출0이다. 전체 raw,
teacher, gold/foil logits/NLL, trace, native/Adam, 출력 동일 쌍을 직접 확인했다.
보고서는 artifacts/causal-framing-20260921-math-review/FINAL-RESULT-MATH-REVIEW.md,
SHA256 **2b2b07562950946845ce0ee80c47b6d3ca24890f802fef8eb15e40203bc7221a**다.
별도 독립 binding 검토도 양군 각2segment/8panel의 모든 준비→반환 journal과
frame/policy/native/model-step 연결을 확인했고 원래 orbit5713/5713 hash를 보존했다.
최종 독립 B도 재생성 QE/EQ 각16/16과 production framing-compare 단1회 exit0을
확인했다. 비교는 selected=None, extension=false, confirmation=NOT_RUN_PREREQUISITE이며
독립 재채점과 일치한다. B 보고는
artifacts/causal-framing-20260921-review/b/FINAL_B_REPORT.md,
SHA256 **366c7f7ed003e845338570b4a60dc88efee2060fad3f83e44d5cb4f2d9ded2a8**이고
실제 review-b.r3b SHA256은
**b5301d0fccdf78bce3b1a15ecb789768f6068d73a902d88fe324e2298ab859bc**다.
무결성 PASS와 품질 FAIL을 분리하며 원 final receipt hash도 불변이다.
모든 SMALL 호출을 합산한 receipt elapsed는746.402500209초다. 이 값에는
CLI 사전검증 전체 시간이 포함되지 않으며 command wall과 구분한다.
실제 source diff·준비자료·명령·로그 경로를 모은 로컬 인계 문서는
artifacts/causal-framing-20260921-evidence/HANDOFF.md다.

코드 범위와 frame/resume binding, 직접 회귀는 PASS이며 TRAIN_FIT 및
UNSEEN_BINDING은 FAIL이다. MINIMAL_BASELINE은 미확립이고 S4/S5/S6는 미수용,
GOAL1_READY=false, GOAL1_ACCEPTED=false다. 제품 기본 QE나 기존 모델 포인터를
바꾸지 않았다. EQ를 기본 생성이나 framing 정보가 사라지는 inference export로
조용히 전달하지 않는다. 원본 기억·SQLite·모델 수식·tokenizer mapping은 그대로다.

|판정|결과|
|---|---|
|CODE_SCOPE / FRAME_PARITY / RESUME_BINDING|PASS|
|RAW_INTEGRITY / INDEPENDENT_A / INDEPENDENT_B|PASS|
|TRAIN_FIT / UNSEEN_BINDING|FAIL|
|CONDITIONAL_EXTENSION|NOT_RUN_NO_JOINT_SIGNAL; 추가 update0|
|CONFIRMATION|NOT_RUN_PREREQUISITE; 봉인 미개봉|
|MINIMAL_BASELINE|NOT_ESTABLISHED|
|S4 / S5 / S6|NOT_ACCEPTED|
|GOAL1_READY / GOAL1_ACCEPTED|false / false|
|TINY_DECODE_FORWARD_COUNT|UNKNOWN; 임시 raw 미보존 계측 한계|

다음 가설은 하나만 제안하며 실행하지 않았다. 같은 BOTH 구성·framing·수식·
optimizer에서 학습 skeleton 수만 줄인 한정 비교로 네 조건을 함께 적합할 수
있는지 확인하는 것이다. 자료 다양성과 사례당 노출도 함께 바뀌는 한계를 공개해야
하며, 현재 결과만으로 특정 회로나 backend 결함·구조적 불가능성을 단정하지 않는다.
새 학습은 별도 승인·사전 예산이 필요하다.

## 2026-09-21 Causal framing — F0/F1 구현·직접 검증, F2 준비

R3-CAUSAL-FRAMING-BASELINE-1.0. 시작 HEAD는
4ba5a3d713f4de15c2fde3aa01021c61c17255c9이며 source13969c6… 이후 차이는
기존 독립 보고/상태/계획 문서뿐이었다. 기존 BOTH corpus·initial·tokenizer·
모델/Adam·raw와 미추적 사용자 파일을 보존했다. 이 절의 시점에는 신규
SMALL optimizer/generation/teacher 모두0이며 독립 준비 A는 PASS로 닫혔다.

QE의 기존 PROMPT_FORMAT/기본 제품 입력은 유지했다. EQ는 독립 typed ID
native-role-bytes-evidence-question-v1을 사용한다. 같은 serializer를
train sample·normal greedy·teacher·raw 검산에 연결했고, native resume의
framing digest와 실제 실행을 비교한다. Cache key/scope도 실제 framing을
사용한다. EQ 파일의 폴더 이동은 의미를 바꾸지 않으며, 일반 QE resume와
descriptor를 잃는 제품 직접 생성·inference export는 EQ를 거부한다.
모델 수식·tokenizer mapping·저장 형식·SQLite는 변경하지 않았다.

독립 source 검토에서 두 연결 누락을 실행 전에 수정했다. EQ teacher의
training-prompt 검산도 실제 framing을 사용하며, 전체 한도보다 이른 단계의
평가-only 완료는 TRAINING으로 기록해 history의 resume 판정과 일치한다.
기존 실패나 종료 기록을 수정한 작업은 아니다.

직접 검증은 Rust1.98.1/locked/offline/Accelerate로 수행했다.
`framing-stage-verified.log`는 관련4test PASS이고, EQ 실제 TINY를 전체4/
단계2 명시 spec으로 continuous2, 새 process1+1, checkpoint 시간 종료 후
평가-only0 optimizer로 비교했다. Weights/Adam/cursor/token/raw가 일치하고
추가 단계는 decision 부재로 차단됐다. 이 실행 자체의 optimizer6,
generation48, teacher48은 품질 학습과 별도다. Native prompt 회귀1 PASS,
Record writer→publisher→reader의0/1/2/128/255/256/257/511/512 경계1 PASS.
잘못된 exact filter의0-test는 PASS에서 제외했다. 최초 TINY 검증 파일명
오류와 stage<max에서 주입되지 않던 fixture 실패도 로그에 보존했다.
구현자 전체 TINY 실행 사용량은 optimizer20/generation160/teacher160;
컴파일 실패·metadata/자료 검산·cache 시험은 모델 호출0이다.
독립 검토자가 같은 정확한 EQ TINY 시험을 별도로1회 통과해 optimizer6/
generation48/teacher48을 추가했다. 이 단계 전체 TINY 합계는26/208/208이다.
독립 실제 준비 보고 SHA256은
26739fd7eac67d0280876143a35b0b116af749cc1154c2ec9f9e069fdc0d5e0e,
review-a.r3b는5077559ad6fd4c6d4a9a197ac28da1bb09a8bf111fe1377a5b23d453e242837a.
Source digest acdc8119b6a62137410bdd3240058376c934bd90b903af1b4b1e179ef895b6fd와
아래 preparation/executable에 결속되며 원본5713/5713 hash 보존도 확인했다.
독립 수학/자료 검산 보고557f5e291feffd6fd6bf6650f29404e38c9f7aff52e866b7a4a5d135f0659682도 PASS다.

Check/release/clippy는 종료0. Clippy의 기존29경고는 별도 부채이며 새 경고는
없다. 전체 변경 파일 fmt check는 기존 compact 코드의 형식 차이를 남긴다.
무관한 전체 재포맷은 하지 않았고 `git diff --check`는 통과했다.

준비 경로는 artifacts/causal-framing-20260921-study. preparation.r3b SHA256
fbe06036068067eb9d67323b53eec19049afbb088dcb3e15ed08a9d1fdff324f.
Production executable artifacts/causal-framing-20260921-executable SHA256
4a77b920e898016e1d1da9daf8414fa062871831f0c1b2f4d75c1b982d18228d.
Test binary SHA256 5d4061c0767678a71c60043588685aaccf361c7f325fff2ac4415ad8eb54ec08.
두 군은 같은 native train512/dev512, 보존 A initial/fresh Adam-zero,
tokenizer562와 BOTH 첫512 tape를 사용하며 필요시 그 tape만 한 번 반복한다.
준비 과정에서 전수1024행의 정확 block permutation, 길이146, target 동일,
근거 제공2/제외0을 검사했다. 미사용 confirmation 내용은 열지 않았다.

구현/실행 증거는 artifacts/causal-framing-20260921-evidence,
독립 근거는 artifacts/causal-framing-20260921-review 및
artifacts/causal-framing-20260921-math-review에 보존한다. Source code는
기존 neural/training/fresh/binding/model/cache/checkpoint/artifact 모듈을
수정했으며 새 제품 source 파일·framework는0이다. 이 준비/검증 결과는
새 품질 관측이 아니다. MINIMAL_BASELINE/S4/S5/S6/GOAL1은 미수용이다.

## 2026-09-21 Foundation orbit512 비교 종료 — 공동 선택 기준선 미확립

R3-FOUNDATION-ORBIT-1.0의 준비·검토 A 후 FIXED/BOTH를 각각 실제512회
학습했다. 각 군은 같은 보존 A initial에서 fresh Adam으로 시작해1회 저장,
새 process511회를 수행했다. 네 학습 command는 모두 exit0이며 두 최종
endpoint는 Finished/resume=false/BUDGET_REACHED다. 신규 SMALL optimizer는
합계1024로 상한을 전부 사용했다. 추가 학습·seed/LR 탐색·기존 run 재개는0.

**핵심 판정:** 정상 실행/자료/재채점과 모델 품질을 구분한다. BOTH의 새 dev
FULL은 FIXED보다19개 많지만 QUERY_BOTH는46→1, ALL4는1→0이다.
두 군 모두 train/dev 후보 조건에 미달했다. `orbit-compare`의 selected는
Null이며 confirmation은 NOT_RUN_PREREQUISITE다. 구현자는 봉인된 평가
본문/정답을 읽지 않았다. MINIMAL_BINDING_BASELINE_VERIFIED=false,
S4/S5/S6=NOT_ACCEPTED, GOAL1_READY=false, GOAL1_ACCEPTED=false다.

### 실제 코드·실행 신원

Source candidate **13969c6fe2b7b9232297a0796b037b202add350b**를 학습 전에
정상 push했고 실제 원격 main SHA 일치를 확인했다. 학습 중 source diff0.
변경 source는 src/binding.rs, src/fresh.rs, src/quality_recovery.rs이며,
기존 data/tape 검증·trainer·RunControl·native reader·publisher·scorer를
사용하는 training-only 비교 경로다. 새 source 파일0, 새 core/framework0.
모델 수식·tokenizer·optimizer·기본 loss·storage/SQLite·제품 생성은 그대로다.
후속 이 상태/계획 갱신은 report-only commit이며 학습 source와 구분한다.

동결 production executable은 artifacts/foundation-orbit-20260921-executable,
SHA256 **5745c440536e5046518dad97b450a78a3c8d9f14e5680a88a2b382e809f3fea1**.
Compiled source digest
3ad507da91a8fcc32ac0fb490a720c7cc8d21c5157102e9a5b1fcaadcf11e74a.
Test-support 없이 Rust1.98.1/locked/offline/Accelerate로 빌드했다.
LOCAL5 SMALL9,513,408 parameters, CPU/F32/thread1, tokenizer562, seq256,
batch8/accumulation1, 기본 CE/first-target1, Adam beta.9/.999/eps1e-8,
decay.01/clip1, warmup32→LR3e-4 고정을 양쪽에서 유지했다.

같은 initial physical SHA256
a58b1d3ff6ac7e4597f886b0450f84d0230911ab900a5c367547f7198a15faaa,
tokenizer SHA256 ec945ee5f3cbd87992bdfa13f199de2a671b94b64337dff04d85e01d982ab9ef.
동일 common corpus physical SHA256은
e73fddef722c4cd1c249d9896b49dd7e1dcae96bc39ad0eb1e3438ee6c5a51ac다.
모두 이전 학습된 A512가 아닌 A의 무학습 initial을 사용했다.

Train/dev는 각128개 서로 다른 semantic skeleton×4이며 두 집합은 겹치지
않는다. Digit0~9의 train key 빈도는[25,25,26,26,26,25,25,26,26,26],
value는[26,26,26,25,25,26,26,26,25,25]다. Dev key는
[25,26,25,25,26,26,25,26,26,26], value는[26,26,26,25,26,26,25,25,26,25].
각 기대25.6에서 절대편차 최대0.6이며 physical order64/64다. 질문 없이
첫/마지막 기록, 작은 event ID, 작은 value를 고르는 독립 규칙은 각각
256/512다. 실제 prompt+digit+EOS는 모두146tokens, 제공2/제외0이다.
이는 label 일치와 자료 균형 검증이며 모델이 그 규칙을 배웠다는 뜻은 아니다.

### 같은 endpoint의 실제 전수 결과

|Arm / panel|FULL|QUERY_BOTH|SWAP_BOTH|ALL4|Gold / foil / other / malformed|EOS / errors|
|---|---:|---:|---:|---:|---|---|
|FIXED common train-orbit|274/512|88/256|42/256|0/128|274 /224 /14 /0|512 /0|
|FIXED unseen dev|236/512|46/256|19/256|1/128|236 /252 /24 /0|512 /0|
|BOTH exposed train-orbit|256/512|0/256|31/256|0/128|256 /256 /0 /0|512 /0|
|BOTH unseen dev|255/512|1/256|28/256|0/128|255 /257 /0 /0|512 /0|

FIXED의 실제 exposed fit은213/256·queryboth85/128이고, 미노출 반대 배정은
61/256·queryboth3/128이다. common train274/512를 exposed fit이라고 하지
않는다. FIXED256 unique×16, BOTH512 unique×8; 공통128 skeleton은 각16번
방문했다. 두 군 모두 총4096행을 소비했고512 tape의 base/query 순서와
LR bits·Adam clock은 독립 검산에서 일치했다. 각input593920/target8192,
합계input1,187,840/target16,384, padding/폐기/미커밋 input·target 모두0.

각 train 후보조건 full>=508/queryboth>=252/all4>=124와 dev 조건
full>=488/queryboth>=232/all4>=116을 모두 충족하지 못했다. 정상 종료/EOS나
새 process 재현 PASS는 이 품질 조건을 대신하지 않는다.

|고정64 표본 step|FIXED train FULL/query/swap/all4|FIXED dev|BOTH train|BOTH dev|
|---|---|---|---|---|
|0|0/0/0/0|0/0/0/0|0/0/0/0|0/0/0/0|
|128|9/0/0/0|8/0/1/0|8/0/0/0|6/0/0/0|
|256|29/1/3/0|29/0/4/0|29/0/1/0|26/0/0/0|

표의 분모는 FULL64, query/swap32, all4 16이다. Step0의 각64 오류/EOS0도
분모와 raw에 보존했다. Step128/256의 각 표본은 EOS64/오류0이다. 이 표본과 최종
512 전수를 같은 분모로 이어 붙여 개선률을 만들지 않는다.

### 효과와 해석의 한계

같은 dev FIXED→BOTH의 FULL gain143/loss124, +19/512=+3.7109375%p다.
128개 orbit별 FULL 비율 차이로 계산한 SE1.5211013%p, 정규근사95% 구간은
[0.7295790,6.6922960]%p다. 한 seed·작은 고정 alphabet에서의 paired 관측이며
512개 독립 문항이나 넓은 언어 일반화의 신뢰구간으로 해석하지 않는다.
QUERY_BOTH gain0/loss45, ALL4 gain0/loss1이므로 FULL 증가를 공동 선택
능력 회복이라고 부를 수 없다.

BOTH는 train query256쌍 전부, dev256쌍 중253쌍에서 두 질문에 같은 값을
냈다. FIXED는 각각96/256·135/256이었다. 같은 query의 배정 변경에도 같은
출력인 쌍은 FIXED train180/dev187, BOTH train194/dev199(각분모256)다.
BOTH 최종 오답은 모두 다른 제공 기록의 값이었다. 두 자료의 정답 검증과
실제 정상 생성 실패가 함께 확인됐으며, 후처리/EOS/잘못된 숫자 문자열로
점수를 보정하지 않았다. 이 특정 endpoint의 질문 구별 실패를 관측한 것이지,
모든 모델/분포에서 질문을 전혀 읽지 않는다는 명제나 내부 회로 증명이 아니다.

|Arm / panel|첫 digit full-vocab NLL|EOS full-vocab NLL|gold/foil 재정규화 NLL|
|---|---:|---:|---:|
|FIXED train|1.592714804543|0.000456482669|1.470620619273|
|FIXED dev|2.042247317189|0.000457582499|1.858239618418|
|BOTH train|0.924160953742|0.000329529201|0.848874226970|
|BOTH dev|0.959151574061|0.000328443898|0.848460166582|

동일 self-teacher forward의 첫 분기 raw gold−foil logits에서 stable
softplus(−delta)를 재계산했다. ln2=0.69314718056은 마지막 열의 두 후보
대조에만 사용한다. 모든 최종 teacher 첫 argmax와 free generation 첫 token은
일치했다. Full-vocab NLL 감소나 token 정확도 거듭제곱으로 FULL을 설명하지
않는다. 양쪽 배정 노출만 바꾸는512회 비교는 선택 학습을 확립하지 못했다.
더 긴 학습/다른 설계의 가능성이나 실패 원인을 이 한 비교로 확정하지 않는다.

독립 native 검사는 FIXED512와 기존 A512의 실제 parameter content,
Adam136 tensors 전체 F32 bits, tokenizer/architecture/clock/cursor/input/target이
동일함을 확인했다. 새 생성이나 재학습 없이 읽기만 수행했다. 이는 FIXED
학습 경로의 보존 증거이며 옛 dev55/256과 새 dev236/512의 성능 비교가 아니다.

### 저장·명령·검증 자료

마지막 durable은 각 study/{FIXED,BOTH}/segment-0001/final, step512다.
실제 native 전체 파일 SHA256:

- FIXED:7500f025bdf1ac9513c7e1c638028cbdd8b4480b9d12742bc214b8ba73ee2743.
- BOTH:44c8c105f7e240a90e18028face8f404782fe9b7e69ab6b993fbaa512e18c364.

최종 평가에 사용한 중간 저장 파일과 final 파일의 physical hash는 서로
다르다. Native tensor-content/step/정책을 독립 연결해 같은 모델임을 확인했다.
TRAIN_END의 sha256 표시는 기존 manifest.weights_sha256이며 위 전체 파일
해시와 구분한다. 기록된 두 segment 시간은 FIXED357.211796초,
BOTH357.996675751초로 합715.208471751초다. 모델 load·평가·저장을 포함하고
컴파일/독립 reader 시간과 구분한다. 각900초/cleanup120초 제한 내 종료했다.
Sampled peak RSS는 FIXED1,185,952KiB, BOTH2,414,912KiB다. 동일 내구성의
독립 성능 benchmark가 아니며 메모리/저장 최적화나 S6 통과 증거가 아니다.

아래 명령은 모두 동결 executable로 실제 실행했다. 모든 모델 process에
VECLIB_MAXIMUM_THREADS=1 OMP_NUM_THREADS=1 RAYON_NUM_THREADS=1을 적용했다.
각 run은1회 segment와511회 segment의 별도 process로 두 번 실행했다.
각 report는 기존 raw 읽기이고, parity는 고정 첫16개 새 생성이다.

```text
artifacts/foundation-orbit-20260921-executable fresh orbit-swap --parent /Users/seo/Projects/Replica-v3/artifacts/binding-learnability-20260921-study-r1/A --output /Users/seo/Projects/Replica-v3/artifacts/foundation-orbit-20260921-study
artifacts/foundation-orbit-20260921-executable fresh orbit-prepare --parent /Users/seo/Projects/Replica-v3/artifacts/binding-learnability-20260921-study-r1/A --output /Users/seo/Projects/Replica-v3/artifacts/foundation-orbit-20260921-study
artifacts/foundation-orbit-20260921-executable fresh run --root /Users/seo/Projects/Replica-v3/artifacts/foundation-orbit-20260921-study/FIXED
artifacts/foundation-orbit-20260921-executable fresh run --root /Users/seo/Projects/Replica-v3/artifacts/foundation-orbit-20260921-study/BOTH
artifacts/foundation-orbit-20260921-executable fresh binding-report --root /Users/seo/Projects/Replica-v3/artifacts/foundation-orbit-20260921-study/FIXED
artifacts/foundation-orbit-20260921-executable fresh binding-report --root /Users/seo/Projects/Replica-v3/artifacts/foundation-orbit-20260921-study/BOTH
artifacts/foundation-orbit-20260921-executable fresh binding-parity --root /Users/seo/Projects/Replica-v3/artifacts/foundation-orbit-20260921-study/FIXED
artifacts/foundation-orbit-20260921-executable fresh binding-parity --root /Users/seo/Projects/Replica-v3/artifacts/foundation-orbit-20260921-study/BOTH
artifacts/foundation-orbit-20260921-executable fresh orbit-compare --study /Users/seo/Projects/Replica-v3/artifacts/foundation-orbit-20260921-study
```

위 실제 명령은 exit0이다. 준비 단계의 상대경로 실패1건은 아래 절과 원본
swap16.log에 보존했다. Checkpoint나 정책 파일을 수정해 통과시키지 않았다.
구현자 parity는 각16/16이며 실제 원자료/실패 포함 raw token·EOS가 같다.
시험은 아래 T1~T6를 덮는 고유3건만 실행해 PASS/exit0였고 zero-test0이다.
test binary SHA2569dd95fd3c3dbe4254ebb7f4ecc724b6d912a82ffe97b9d3a67bdbfc7b229425c.
TINY optimizer8/generation64/teacher64, scalar/finite-difference0으로 별도다.

```text
cargo test --release --locked --offline --features accelerate --bin replica-train training::fresh::identifiable::binding::tests::foundation_ -- --nocapture --test-threads=1
```

검토자도 동일 frozen executable의 fresh orbit-review-parity --root에 같은
FIXED/BOTH 절대경로를 주어 순차 실행했다. 각 exit0/16개 생성/16개 일치,
teacher0/optimizer0이며 재시도0이다. 독립 raw/teacher/receipt/usage 재검산에서
학습평가 generation2816 + swap16 + 구현자parity32 + 검토자parity32 =2896,
self-teacher2816, optimizer1024를 확인했다. 모든 generation2896/teacher2816은
RETURNED, NOT_INVOKED0/UNKNOWN0이다. 정상 생성 완료와 정답 여부는 별도다.
Swap 및4개 parity까지 포함한 recorded elapsed 합722.039185168초로,
active7200초 상한 내다. 컴파일·보고서 작성·독립 raw reader 시간은 이 모델
실행/관측 receipt 합에 넣지 않았다. 미사용 호출량을 추가 진단에 쓰지 않았다.

수학 최종 독립 보고 artifacts/foundation-orbit-20260921-math-review/FINAL-RESULT-REVIEW.md의
SHA256은944af34eb63a0def9a98a519420efb7fcf1bf4fa4bd1feac2046f38d3e3baa20다.
이 검토자는 추가 모델 호출0으로 양군 trace1024·최종 raw/teacher2048을
검산했다. 별도 데이터 검토자의 실제32회 재생성과 구분한다.
데이터 최종 독립 보고 artifacts/foundation-orbit-20260921-review/d4/REPORT.md는
SHA2568056a45a204ddd29e653923cb7b4c91f92fcd229239e5a9aef88fa6cca2baf0f다.
실제16개 예정 panel의 raw/teacher2816행씩,2개 usage/endpoint와 관측 ledger를
검산하고 독립32개 출력을 재생성했다. INDEPENDENT_RECOUNT_INTEGRITY=PASS,
FIXED_BASELINE_CANDIDATE=FAIL, BOTH_BASELINE_CANDIDATE=FAIL이다.
원 A의 지정20개 corpus/checkpoint/Adam/raw/종료/parity 파일은 사전 hash와
모두 일치했고, frozen source/executable도 실행 후 다시 일치했다.

CODE_SCOPE=IMPLEMENTED_AND_DIRECT_TESTED; INDEPENDENT_DATA_REVIEW=PASS;
VALUE_SWAP16=EXECUTED; TRAIN_FIT_FIXED=213/256_EXPOSED;
TRAIN_FIT_BOTH=256/512; UNSEEN_RECOMBINATION=FAIL; MODEL_EFFECT=MIXED_FULL_AND_WORSE_JOINT;
CONFIRMATION=NOT_RUN_PREREQUISITE; MINIMAL_BINDING_BASELINE_VERIFIED=false;
S4/S5/S6=NOT_ACCEPTED; GOAL1_READY=false; GOAL1_ACCEPTED=false.
최신 요구사항 전체와 구현/실행/조건부 미실행을 종료 시 다시 대조했다.
인가 범위 구현·제한 비교는 완료됐고, 남은 품질 목표를 성공으로 바꾸지 않았다.

실제 준비물/원자료는 프로젝트 내부의 아래 허용 경로에 보존했다.

- artifacts/foundation-orbit-20260921-study/: preparation/selection/review-a,
  두 arm의 corpus.r3cor/metadata.r3b/plan.r3b/tokenizer.r3b/initial.r3m,
  0·128·256·512 raw/teacher/receipt, updates trace와 최종 weights+Adam.
- artifacts/foundation-orbit-20260921-evidence/: 실제 source diff,
  source-commit.txt/source-remote.txt, frozen source/binary hashes, 직접 tests,
  build/swap/prepare/4개 segment/report/parity/comparison stdout.
- artifacts/foundation-orbit-20260921-review/ 및
  artifacts/foundation-orbit-20260921-math-review/: 독립 준비·결과 reader,
  보고서/해시/실행 로그. 이 원자료와 리뷰 전문은 Git에 게시하지 않는다.

## 2026-09-21 Foundation orbit 준비 — D0~D2 검증, 학습 전 source 동결

R3-FOUNDATION-ORBIT-1.0으로 별도 자료 개입을 준비한다. 시작 HEAD는
76f296c95c2c66b0536bc4ce71a766a52b5f251d이고 기준 sourcea4267d3 이후 변경은
상태/계획 문서2개뿐이었다. 미추적 .DS_Store를 보존했으며 학습 process0,
Rust1.98.1/locked/offline/Accelerate/thread1을 확인했다.
기존 A512와 모든 역사 run의 종료/예산/가중치/원자료를 수정하지 않는다.

D0 independent native reader의 source/data 검산은 EXECUTED_THIS_REVIEW다.
최종 A 파일0630aa0ad8a576351df0beb7a7467e3610715daffb94ab69eb2ad099b05566ba와
지정 원본/기존 독립 보고의 hash가 일치했다. 실제 A train256 모두 framed
prompt+digit+EOS146이다. 저장 순서 첫8 base 원 raw는 full14/16,both6/8;
값만 교환할 때 actual prompt 차이는 index87/139의 두 digit뿐임을 확인했다.
아직 swap generation 점수가 아니며 모델/optimizer/teacher 호출0.
계획 FIXED256unique×16/BOTH512unique×8의 동일 input593920/target8192는
무학습 tape 계산값이며 실제 학습 사용량과 구분한다.

독립 D0/설계 보고는 artifacts/foundation-orbit-20260921-review/D0_DESIGN_REPORT.md,
SHA25663a2aa8409b9cf019f4ef786f2f8b1af6e033cee85c3348a90d2021c126afe75.
실제 준비물 사전 수용은 별도다. 수학 초안 검토는 SOURCE_ONLY이며
artifacts/foundation-orbit-20260921-math-review/DRAFT-SOURCE-REVIEW.md,
SHA256d7ddb26a2dcfe3521681a79d39ebb59c3d44fa9001282bdb6dbc9dd65b07d9e9.
사용자 전달 PM 해석은 USER_SUPPLIED_REVIEW로 구분한다. 별도 전문을 실제로
읽었다고 주장하지 않으며 접근 가능한 지정 프로젝트 리뷰만 재사용한다.

직접 회귀 T1~T6를 세 고유 테스트로 실행했다:
foundation_t1_t2_t3_values_split_and_finite_tape,
foundation_t4_t6_native_process_resume_and_peer_authorization,
foundation_t5_strict_orbit_scores_and_seen_mask.
Release/locked/offline/accelerate replica-train의 정확한 foundation_ 필터에서
3 PASS/exit0, 0-test0이다. 실제 TINY 두 arm의 연속2 vs 새process1+1에
optimizer8/generation64/teacher64를 썼고 scalar/finite-difference0이다.
자료·마지막512 tape/native batch·gold/foil/형식/미완료/ALL4·인가 검사를 했다.
초기3건은 confirmation의 빈 train 슬롯을 기존 native reader가 거부해
실행 전 실패했다(model calls0). 기존 common train 슬롯을 보존하도록 고쳤다.
dev/step0 exposed 표시와 일반 준비 경로의 confirmation 내용 목록 노출도
독립 지적 후 실제 tape prefix 및 reviewer 전용 생성으로 수정했다.
초기 실패 로그를 버리거나 통과 수에 합산하지 않았다.

Production executable artifacts/foundation-orbit-20260921-executable의 SHA256은
5745c440536e5046518dad97b450a78a3c8d9f14e5680a88a2b382e809f3fea1이다.
Test-support/fixture 없이 빌드했다. 첫 swap CLI는 상대 parent와 원본 plan의
절대 study 경로 불일치로 자료/모델 호출 전 exit1이었다(output root 미생성).
원본에 등록된 절대경로를 사용했고 원본 plan·검증 규칙은 수정하지 않았다.

D1 실제 값 교환16: original full14/16,both6/8 → swapped full4/16,both0/8.
원래 both였던6개 중 swap both0/6, 동일 output9/16. Gold4/foil11/기타숫자1/
malformed+error0이며 generation16/teacher0/optimizer0. Query/IDs/order/time를
유지한 고정 표본의 관측이다. 전체 원인이나 질문을 전혀 쓰지 않는다는
전역 단정을 하지 않는다. Raw SHA256
b635ae79cbf4b442cb17abb275d6f1a7e1271fc006093e341cb724d77e0bcaa6.

실제 준비 root: artifacts/foundation-orbit-20260921-study.
Preparation SHA256 a93c975fb97a02d6914b6f0b6c951e5253e8153c5f9d0bf039c5549451fb5c1d.
FIXED policy file bb604a5f3627cdac5b2dc0474e0297e8a2f138b0ce6ebd74fef50ebc47444df1,
BOTH c4849e0311836b4d408a841034b4ac317f23fceba948c70a3f9044c53a12c644.
두 initial 파일은 원 A의 a58b1d3ff6ac7e4597f886b0450f84d0230911ab900a5c367547f7198a15faaa를
그대로 복사했다. Metadata에 새 seed나 초기값 탐색은 없다.
독립 reviewer가 별도 confirmation256을 학습 전에 봉인했다.
본문/정답은 구현자에게 전달하지 않으며 아직 모델로 평가하지 않았다.
이 시점 새 SMALL optimizer0/generation16/teacher0이며 실제 학습 결과는
별도로 기록한다. 모든 실제 명령/초기 실패/통과 stdout은
artifacts/foundation-orbit-20260921-evidence에 보존한다.

D2 실제 준비물 독립 검산 두 건은 모두 PASS다. 데이터 검토의
artifacts/foundation-orbit-20260921-review/ACTUAL_PREREVIEW.md SHA256은
4130e75f57fa8e82dad2c72e2c4348a9ca4f464ce82474a9f261d4aa32bc6cfd이며,
수학/실제 tape 검토의 artifacts/foundation-orbit-20260921-math-review/FINAL-PREPARED-REVIEW.md는
a568b8ea01719ffb44f638e9383c86ed4ec728142ffea854cdce2d17383968db다.
기존 reviewer publisher로 두 보고서와 실제 preparation/source digest를
review-a.r3b에 결속했다(exit0, physical SHA256
5471605289c438fef7d39301247ae6dfb0e9c734b1045b471d1f40a1c95156b3).
독립 reviewer의 신규 optimizer/generation/teacher는 모두0이다.
Confirmation seal SHA256은
55524d144ec83d584703b42d7a952774fb5b8c48a4d5430ea78a9aaf68d36056이며,
구현자는 봉인 내용/정답을 읽지 않았다. Confirmation 모델 평가는 NOT_RUN이다.
동결 compiled source digest는
3ad507da91a8fcc32ac0fb490a720c7cc8d21c5157102e9a5b1fcaadcf11e74a다.
이는 자료/실행 준비 수용이며 모델 품질이나 Goal1 수용이 아니다.

## 2026-09-21 최소 선택 A512 종료 — 실행 검산 PASS, 양성대조 미확립

R3-BINDING-LEARNABILITY-1.0의 L0/L1 독립 사전검토 후 A(K1-V)를 실제512회
학습했다. 첫1회를 저장하고 새 process에서511회를 이어갔다. 두 segment는
정상 종료했으며 최종512 checkpoint는 Finished/resume=false/BUDGET_REACHED다.
Train213/256·both85/128, dev55/256·both1/128로 등록 합격선에 못 미쳤다.
따라서 POSITIVE_CONTROL_NOT_ESTABLISHED로 닫고 B/C/D 및 새 이름 probe는
NOT_RUN_PREREQUISITE다. 미사용1536 update를 재사용하거나 정책을 바꾸지 않았다.

**실제 source/candidate:** a4267d308c233f7bab47e1c4bcf39f4f4e21d3cb.
학습 전에 정상 push했고 `git ls-remote origin refs/heads/main`이 같은 전체
SHA임을 확인했다. Base report e57294c2c829abcc9f5ef15f320f6c27351f2229 이후
source diff는 새 training-only binding.rs와 기존 fresh/identifiable 경로,
trainer/teacher 관측 및 두 상태 문서다. 모델 수식·loss·Adam·tokenizer·저장
포맷·제품 검색/생성·SQLite·Cargo는 바꾸지 않았다. 실행 중 source diff0.

동결 executable: `artifacts/binding-learnability-20260921-executable`, SHA256
819dbbe752d38c99b86aca7394e6ad3184f1f8a69c8a17668f8a6beabf910872.
Compiled source digest b700e0460d07b6ec9dd739fecfd5b8eb7e1b084ef5aa688e16e19cc2f04a36a0.
실행 root `artifacts/binding-learnability-20260921-study-r1/A/`의 policy SHA256
374da364863daf7fb2326e1cf349edbab71b4331e8117617ca2d29bcac1b82d7,
corpus.r3cor SHA256 a7e71768c0313b02816729e8f274aa509d458a5901293256033bc54d7469c814.
최종 `segment-0001/final`의 **실제 전체 파일 SHA256**은
0630aa0ad8a576351df0beb7a7467e3610715daffb94ab69eb2ad099b05566ba다.
TRAIN_END 로그의 sha256=22af0b6d8e3738de0607c49baae394a417f3a1b5ac5e6d3c90adce674b71d140는
기존 출력 코드가 manifest.weights_sha256을 표시한 것이므로 전체 파일 해시와
구분한다. 평가의 model weight_hash는
3aafe3fd1e039dcf0bb02fdf9e413c7cf45b272bbdefb12294a8d6b6cc320c05다.

|step/panel|Full=value 첫 token=body|Both|EOS/생성 오류|Teacher response CE|
|---|---:|---:|---:|---:|
|0 train32|0/32|0/16|0/32|6.7892534435|
|0 dev64|0/64|0/32|0/64|6.7905149870|
|128 train32|4/32|0/16|32/0|1.2256494129|
|128 dev64|7/64|0/32|64/0|1.2931039004|
|256 train32|15/32|1/16|32/0|0.7774145659|
|256 dev64|25/64|1/32|64/0|0.8432347958|
|512 train256(전수)|213/256|85/128|256/0|0.2168226225|
|512 dev256(전수)|55/256|1/128|256/0|1.4753604117|

최종 raw에서 같은 ID·질문·근거의 첫 train32/dev64만 다시 추출하면
train28/32·both12/16, dev11/64·both0/32다. Step256의 같은 표본과 비교할 때
train15→28, dev25→11로 갈라졌다. 전수256과 중간 표본64를 같은 분모로
비교한 결과가 아니며, 이 추출의 신규 generation은0이다.

V에는 인용 요청이 없으므로 citation=N/A다. Short value 점수를 인용 포함
전체 QA나 Goal1 점수로 합산하지 않는다. 초기96개 길이 종료/EOS 누락은
분모와 원본에 남아 있으며 정상 평가 완료와 정답을 구분했다. 최종 양쪽
query view 정답은 train104/128·109/128, dev33/128·22/128이다.

최종 오답244개는 모두 `[잘못된 숫자 token, EOS]`였다. Train 오답43개 중
다른 기록의 값40·두 기록 밖 숫자3, dev 오답201개 중 다른 기록의 값195·
두 기록 밖 숫자6이다. 형식/UTF-8/실행 오류는0. 한 base의 두 query에 같은
값을 낸 경우 train40/128, dev53/128이었다. 두 view의 정답 개수[0,1,2]
분포는 train[0,43,85], dev[74,53,1]이다. 첫 값 token teacher CE는
train0.4331888402/dev2.9502633182, EOS CE는0.0004564049/0.0004575052다.
이 원자료는 인용 생성이나 EOS 종료 이전의 값 선택 문제를 보여 준다.
배운 train 조합에 대한 적합도도 완전하지 않아 순수 일반화 문제만으로
한정하지 않는다. 어떤 수식·Rust·저장·tokenizer 하나를 원인으로 확정하지 않는다.

실제512 updates에서128 train bases×2 views 각각16회 노출, input593,920,
target8,192, padding0, 미커밋/폐기 input0/target0이다. 최초 batch response
CE6.78829956, gradient norm38.14025465, 실제 weight delta L2 0.02885980,
첫 LR9.375e-6과32부터3e-4 고정을 trace에서 검산했다. Adam clock/cursor는
저장·새process 재개 후 누적 유지됐으며 모든512행의 draw/LR가 고정 tape와 같다.
마지막 batch CE0.32067224는 전체 train CE와 구분한다.

학습·예정 평가 두 segment elapsed 합288.427923833초, parity0.739871875초로
기록된 실행 합289.167795708초다. Sampled peak
RSS1,138,160KiB다. SMALL 실제 generation800/self-teacher800, 이후 별도 새
process의 고정 dev 첫16 parity16/16까지 generation816/teacher800이다.
독립 검토는 추가 generation0으로 실제 raw/token/EOS/teacher 및 parity
receipt를 재검산했다. Native model 재현과 낮은 정답률은 동시에 성립한다.
학습·평가에 NaN/Inf·자료 변경·UNKNOWN·취소·저장 오류는 없었다.
TINY 누적16 optimizer/129 generation/128 teacher는 별도이며 위 수치에
합산하지 않는다. 자동 재시도·새 seed·LR 변경·모델 크기 변경0.

최종 native 파일114,180,992bytes는 weights38,053,632bytes,
Adam m/v76,107,264bytes와 metadata/directory/alignment20,096bytes다.
기존 native 형식을 그대로 사용한 관측이며 저장 최적화 성과가 아니다.
학습·평가·IO를 포함한 두 segment 기준1.77514updates/s,
input2,059.163tokens/s·target28.402tokens/s다. 최종512건의 generation만
분리하면1,024출력token/기록된6.420초=159.502tokens/s다. 밀리초 해상도의
호출 시간이며 teacher/load/publication을 제외하므로 위 처리량과 구분한다.

실제 명령은 frozen executable의 아래 첫 명령을 두 개의 별도 process로
실행한 뒤 report/parity 명령을 실행했다. 각 process에
`VECLIB_MAXIMUM_THREADS=1 OMP_NUM_THREADS=1 RAYON_NUM_THREADS=1`을 적용했고
모두 exit0이었다. 학습 종료는 품질 합격이 아니다.

```text
artifacts/binding-learnability-20260921-executable fresh run --root artifacts/binding-learnability-20260921-study-r1/A
artifacts/binding-learnability-20260921-executable fresh binding-report --root artifacts/binding-learnability-20260921-study-r1/A
artifacts/binding-learnability-20260921-executable fresh binding-parity --root artifacts/binding-learnability-20260921-study-r1/A
cargo test --release --locked --offline --features accelerate --bin replica-train training::fresh::identifiable::binding::tests:: -- --nocapture --test-threads=1
```

마지막 명령은 학습 전 직접 회귀2건의 실행이며 후속 정확한 process 회귀는
그 두 이름 중 하나의 재검증이다. Unique tests2, zero-test invocation0;
다른 초기 실패 이력은 아래 준비 절에 별도 기록했다. 상세 증거의 로컬 위치:

- `artifacts/binding-learnability-20260921-evidence/`: 직접 tests/build,
  A-segment-0000.log/0001.log, A-report.log, A-parity.log,
  candidate.diff, source-commit.txt/source-remote.txt, final-artifact-hashes.txt.
- `artifacts/binding-learnability-20260921-study-r1/A/`: native corpus/metadata,
  tokenizer/initial/plan, eval-0000/0128/0256/0512 raw·teacher 및 prepared/resolved
  call receipts, segment 끝 기록, 최종 weights+Adam와 parity 원자료.
- `artifacts/binding-learnability-20260921-review-a/`, `...-review-b/`:
  서로 다른 설계·실제 준비물 검토와 L4 독립 재채점. 원자료는 게시하지 않는다.

L4 독립 실행 보고서는
`artifacts/binding-learnability-20260921-review-b/l4/A512/REPORT.md`,
SHA256 acf772213677b19b8130d14c8856187cfe0335d97201c3a493f8695399ccd683다.
별도 Rust reader의8 panel 재채점, usage/trace512, 최종 오류 분류,
동일 표본 대조와 native/receipt 검사가 모두 exit0이었다. 독립 검토자의
신규 모델 호출은0이며 기존16개 parity의 실제 token/receipt를 검증했다.
이 판정은 원자료 무결성 PASS이고 모델 품질 수용은 아니다.

다음 변경을 제안한다면 **입력 표현의 key/value 연결** 한 축만 대상으로 한다.
같은 사실·query·정답·분할을 유지한 채 기록 본문의 key/value 경계를 명시하는
일관된 슬롯 표현과 현재 문장 표현을 대조하는 가설이다. 숫자/EOS 출력은
안정됐지만 다른 기록의 값을 고르는 오류가 집중된 것이 조사 근거다.
표현이 원인이라는 증거나 입증된 처방은 아니며, 기본 연산 결함도 확정하지
않았다. 제안용 새 corpus·정책·모델 호출·학습은0이고 자동 후속 실행은 없다.

CODE_SCOPE/INDEPENDENT_PLAN_REVIEW/RAW_RECOUNT=PASS.
EASY_CONTROL_TRAIN=FAIL, EASY_CONTROL_UNSEEN=FAIL,
MULTITOKEN_KEY/CITATION_OUTPUT/NEW_IDENTIFIER_TRANSFER=NOT_RUN_PREREQUISITE.
역사 모델·실패 원본은 해시 재확인으로 보존했다. H3/S4/S5/S6는 이 작은 과제로
수용하지 않으며 final200은 생성·사용하지 않았다. GOAL1_READY=false,
GOAL1_ACCEPTED=false. 이번 계약의 제한 실험과 실패 경계 기록은 완료됐고
추가 학습은 시작하지 않는다.

## 2026-09-21 최소 선택 학습 준비 — R3-BINDING-LEARNABILITY-1.0

새 계약은 종료된 LOCAL5/GLOBAL6을 재개하지 않는다. 새 SMALL 학습 전
L0/L1 자료·수식·실행 경로 검증을 진행한다. 기준 source936d42258a6143ae664fca71ef8294b760b1dffe,
보고 e57294c2c829abcc9f5ef15f320f6c27351f2229 이후 변경을 보존했다.
Rust1.98.1, 기존 Cargo.lock/offline/Accelerate CPU/F32/thread1을 사용한다.
기존 native 저장·공유 trainer/evaluator를 사용하며 제품 추론에 generator나
정답 resolver를 넣지 않는다. 새 학습 내용은 후속 실제 실행 기록으로 구분한다.

L0 근거는 보존된 독립 MATCHED4096_REPORT와 원자료 identity다. 아래 train64는
전체8192의 표본이다. 과거 모델의 수치이며 이번 실행으로 재생성한 값이 아니다.

|train 과제(각8)|LOCAL full/body/citation|GLOBAL full/body/citation|
|---|---|---|
|A|7/7/8|5/5/8|
|B|6/8/6|6/8/6|
|C|2/4/2|1/4/3|
|D|0/3/1|0/3/2|
|E|1/2/3|0/3/3|
|F|1/4/4|1/4/2|
|G|6/7/6|7/7/7|
|H|8/8/8|8/8/8|

두 군 C/D/E train 공동정답0/12, primary0/96, transfer0/24다.
각4096 updates에서8192행은 정확히4회 노출됐다. 실제 mask 차이는
train164/8192·primary9/512·transfer3/128로, 활성 개입은 존재하지만 대부분
입력의 마스크는 같았다. 작은 train 적합과 새 조합 선택의 실패를 확인했으며,
용량·저장·Rust·tokenizer·local mask 하나를 원인으로 확정하지 않는다.

설계 검토 A는 실제 Astra SOURCE_ONLY, 설계 B는 실제 독립 자료 검토자다.
Fable 도구는 없어 NOT_RUN이며 전달 질문을 준비했다. 두 설계 검토는 실제
준비 corpus/native/tape 사전 수용이나 모델 품질 PASS의 대체물이 아니다.
원자료·검토 packet은 로컬 ignored artifacts에 보존하며 게시하지 않는다.

실제 준비 자료는 네 군 각각train256/dev256/new-name128, 총2560행이다.
독립 reader에서 네 숫자 중복 배제·request-only label·canonical scene 분할·
공통 latent/event/순서·tokenizer prompt parity·제공 근거2/제외0을 확인했다.
최대 prompt+target/EOS는 A146/B167/C154/D175다. 공통512 update tape는
각32회마다256행을 한 번씩 사용하여 각행16회 노출한다. 네 군 전체 계획
input2,613,248/target155,648은 계산값이며 실행 사용량이 아니다.
Source/status/time/context는 공통이고, 물리순서는train/dev 각각64/64다.
독립 event stream의 ID 크기 순서는70/58·71/57로 우연히 불균형이지만
양쪽 query를 함께 사용하므로 selected-ID-rank는128/128이다.

공통 SMALL은9,513,408 parameter/38,053,632 tensor bytes/F32/local5다.
네 최초 weight-content는
af6abb8fd48cdd0d4396b86c543aee688efc770470c52adc0dcf7016acf40306,
tokenizer mapping은ec945ee5f3cbd87992bdfa13f199de2a671b94b64337dff04d85e01d982ab9ef다.
새 seed17 초기화이며 과거 학습 weights/Adam을 불러오지 않았다. Native에
training state/optimizer가 없고, 첫 진입의 fresh Adam zero/clock0을 검증했다.

직접 회귀 두 이름: `binding_data_tape_native_batch_contract`,
`binding_tiny_continuous_vs_new_process_resume`. Release/locked/offline/
accelerate의 정확한 module 필터에서2 PASS, 최종 report 경로 추가 후 정확한
process 시험1 PASS. Native/전수 framing/label shift/EOS mask/512 tape·LR의
writer→publisher→reader와 실제 TINY2 vs새process1+1을 확인했다. 초기 대비
weights/Adam 변화, 최종 weights/Adam/clock/cursor/input/target 일치 및 target
NLL 평균=기존 CE를 검사했다. 추가 teacher forward 없이 관측값을 저장한다.

개발 중 native scene-binding 문자열/축소 fixture context 오류를 수정했다.
무작위 TINY의 첫 control-token 실패는 optimizer0/generation1/teacher0로
보존했다. 저장/재개 회귀에는 기존 TINY 전용 EOS tensor fixture를 재사용했다.
이 fixture는 production binary에 컴파일되지 않는다. 통과한 네 번의 process
회귀를 포함한 현재 TINY 누적optimizer16/generation129/self-teacher128이다.
0-test PASS0. 처음 넓은 `binding_` 필터에 우연히 포함된 기존
`harness_m04_native_checkpoint_and_eval_binding_before_close_gate`는 legacy-v2
objective binding 없는 fixture 로드에서 실패했다. 이번 신규 경로의 PASS에
합산하거나 이 무관한 fixture를 수정하지 않았고, 이후 정확한 필터를 썼다.

독립 검토가 발견한 parity 전체 예산 누락/TINY512 판정 가정과 committed
input 보고 오류도 좁게 수정했다. Committed는 실제 updates trace 합계,
discarded는 실행 사용량에서 그 합계를 뺀 값이다. 수정 전 준비물은학습0으로
보존하고, 동일 자료·초기 tensor·조건을 수정 source에 다시 결속했다.
실행 root: `artifacts/binding-learnability-20260921-study-r1/`.
Preparation SHA256 ab793fce6b513ba3ad8c8e030f5acd510b57e079381a624bb0a3749d794a14d6,
compiled source digest b700e0460d07b6ec9dd739fecfd5b8eb7e1b084ef5aa688e16e19cc2f04a36a0,
production binary SHA256819dbbe752d38c99b86aca7394e6ad3184f1f8a69c8a17668f8a6beabf910872.
직접 실행 로그와 검토 packet은 `artifacts/binding-learnability-20260921-evidence/`
및 review-a/review-b 경로에 보존한다. SMALL optimizer/generation/teacher는
준비 종료 시0이다. 실제 준비물 독립 사전검토 A/B는 모두 PASS로 닫혔고
그 수용을 exact preparation/source에 native receipt로 결속했다.
A 보고 SHA25618728d9f00db3336cea02dc63514ac5064f484bfccdde70053eabb1009381702,
B 보고 SHA256fc6d732be3c3b6c1f59d90ae42a032cf36e6aedf75520bccc8426fde67c5b478.
L1 코드/자료 준비 PASS이며 모델 품질 PASS가 아니다.
후속 품질 결과·source/remote SHA는 실제 단계가 닫힌 뒤
기록한다. S4/S5/S6·GOAL1_READY/ACCEPTED는 미충족이다.

## 2026-09-21 R3–R5 실제 비교 종료 — 실행 검산 PASS, 두 군 품질 미달

R3-IDENTIFIABLE-BASELINE-1.0의 사용자 인가 replacement 연구를 실제 끝점까지
실행했다. LOCAL5/GLOBAL6 각각4096, 총8192 optimizer updates로 등록 상한을
모두 사용했다. 두 최종 native는 Finished/resume=false/BUDGET_REACHED다.
새 가설·LR·seed 탐색이나 추가 학습을 시작하지 않았다. 이번 변경은 기존
실행 경로 사용과 상태 문서 갱신이며 제품/학습 소스·tests·Cargo 변경0이다.

동결 source936d42258a6143ae664fca71ef8294b760b1dffe, 실행 시작 report HEAD
a9e852a83d2775ba0242dc06a82b2a8af82c451e. Rust1.98.1, 기존 locked/offline
Accelerate CPU/F32/thread1 production binary를 사용했다. Test-support/시간
주입/fixture weights/외부 모델/API0. Binary SHA256
1eeea87e0c8d66fb5f99136e106c70fca37788422416b45478a94a45acc1e201,
compiled source digest f322766fb7ae198e1c5a6c8f6ad357676d35ebcdc529ca3f5b9a2083a92acb69.
최종 source/Cargo diff가 비었고 binary hash도 변함없음을 확인했다.
Preparation SHA256=f5f548ecadddcecf71eadc9e2f9752ded0b23baeda61173bb956ab7eb39c3376;
LOCAL/GLOBAL plan 파일 SHA256은 각각
4ba25d89b481c9b879a04e4cba5b4388da5378df745f46141d1ea907ef30b977 /
4348a01c119567e6bbf65c9c73c3a48da5cb58564076b8c0226246659c2ccb9a,
공통 tape hash=bcf6d73d59732af250525049e7df90fe66f14c4253d4fd3a1c6acb8b763e49ec.

독립 A가 실제 replacement preparation을 수용한 후 첫1 update를 저장하고
각각 새 process에서 재개했다. 두 군의 초기 tensor 내용
af6abb8fd48cdd0d4396b86c543aee688efc770470c52adc0dcf7016acf40306,
tokenizer mapping, train8192/primary512/transfer128, sample tape와 response CE,
Adam, LR schedule은 같았다. Architecture ID/local_layers만 구분했다.
두 군에서 매 update8과제 각1개, 전체4096 LR bit/step/sampler clock과
objective binding이 독립 검산에 일치했다. 각8192행은 정확히4회 노출됐다.
Train4096 semantic bases×2 views, primary256×2, transfer64×2를 구분한다.

1024에서 두 군 모두 평가를 마친 뒤 독립 재검산으로 연장을 판단했다.
LOCAL 고정 train CE1.2229007222446042→1.1316174637622385는7.4645% 감소,
GLOBAL1.2456659498382388→1.107368776334503은11.1023% 감소였다.
네 probe의 frozen 사례와 target1141이 같았다. GLOBAL의>=10% 조건으로
기존 OR 신호를 충족해 두 군을4096까지 연장했으며, 예산을 늘린 것은 아니다.
512→1024 primary 증가+16 또는 CDE both 증가+8 조건은 두 군 모두 미충족이었다.

| Arm / step | 고정 train full | Primary full | Transfer full | Primary CDE both | 생성 오류 |
|---|---:|---:|---:|---:|---:|
|LOCAL5 /0|0/32|screen0/32|NOT_DUE|NOT_DUE|64|
|GLOBAL6 /0|0/32|screen0/32|NOT_DUE|NOT_DUE|64|
|LOCAL5 /256|NOT_DUE|screen10/64|NOT_DUE|screen0/12|0|
|GLOBAL6 /256|NOT_DUE|screen10/64|NOT_DUE|screen0/12|0|
|LOCAL5 /512|11/64|75/512|19/128|0/96|0|
|GLOBAL6 /512|9/64|73/512|19/128|0/96|0|
|LOCAL5 /1024|11/64|72/512|19/128|0/96|0|
|GLOBAL6 /1024|11/64|77/512|20/128|0/96|0|
|LOCAL5 /2048|NOT_DUE|screen10/64|NOT_DUE|screen0/12|0|
|GLOBAL6 /2048|NOT_DUE|screen10/64|NOT_DUE|screen0/12|0|
|LOCAL5 /4096|31/64|199/512|53/128|0/96|0|
|GLOBAL6 /4096|28/64|189/512|50/128|0/96|0|

Step0 무작위 초기 모델의 길이 종료/EOS 누락128건은 원자료와 분모에 남아 있다.
최종 생성 오류0과 전체 학습 과정의 오류0은 다르다. 최종 각704건은 정상EOS/strict UTF-8다.

|4096 panel|LOCAL full / body / citation|GLOBAL full / body / citation|CDE both (두 군)|
|---|---|---|---:|
|train64|31 /43 /38|28 /42 /39|0/12|
|primary512|199 /282 /274|189 /282 /275|0/96|
|transfer128|53 /73 /73|50 /74 /65|0/24|

최종 primary A–H full은 LOCAL[23,53,7,7,6,5,34,64],
GLOBAL[19,51,6,8,7,8,26,64], 각분모64다. Train은
LOCAL[7,6,2,0,1,1,6,8], GLOBAL[5,6,1,0,0,1,7,8], 각8;
transfer는 LOCAL[9,12,1,0,5,2,8,16], GLOBAL[8,9,1,2,3,4,7,16], 각16이다.
각 과제 body/citation·base both, 길이/기록순서/context범위/leading-zero/
반복숫자/표현/view별 정확한 분모와 결과는 독립 재채점 보고서와 native
recount에 보존했다. Body나 citation 정답을 full로 대신하지 않는다.
동일 primary에서 양군 모두 정답158, LOCAL만41, GLOBAL만31, 모두 오답282다.

|실제 사용량/비용|LOCAL5|GLOBAL6|
|---|---:|---:|
|Optimizer updates|4096|4096|
|Committed tape input / target|6,883,740 /598,104|6,883,740 /598,104|
|Executed input / target (미커밋 포함)|6,885,380 /598,249|6,887,152 /598,395|
|실행 후 미커밋 input / target|1,640 /145|3,412 /291|
|Executed padding|1,225,164|1,225,504|
|Generation / self-teacher|2304 /2304|2304 /2304|
|Segment elapsed seconds|3316.574984667|3267.065457541|
|Sampled trainer peak RSS KiB|5,557,264|5,531,120|
|최종 native bytes|114,180,992|114,180,992|
|가중치 tensor bytes|38,053,632|38,053,632|
|Adam tensor bytes (136개)|76,107,264|76,107,264|
|Batch1 F32 KV 최대 payload 계산 bytes|2,555,904|9,437,184|
|4096 생성 중 기록된 최대 KV payload bytes|1,185,024|1,211,904|

새 SMALL8192/TINY0, 실제 input13,772,532/target1,196,644/padding2,450,668;
committed input13,767,480/target1,196,208이다. 새 generation4608,
self-teacher4608 모두 ENTERED=RETURNED이며 NOT_INVOKED/UNKNOWN0.
이전 고정P16 generation16은 재실행하지 않고 계약 누적4624에 포함한다.
Generation6400/teacher6400 상한 내이며 예약된 독립64회는 사용하지 않았다.
두 군 segment elapsed 합6583.640442208초이며 준비/기존parity와 이전r2
실패0.912484459초는 별도 기록이다. 이번 replacement 준비 수치 forward8/
backward2, optimizer0; 독립 최종 검토 모델/optimizer/teacher/generation0.

RSS는 trainer의 표본 최대이고 OS high-water나 전체 평가 메모리 최대가 아니다.
KV 용량은 실제 config/shape로 계산한 payload 한도이며 전체 working memory와
다르다. 최종 기록 generation timer 기반 약409.54 vs411.44 token/s는 teacher,
load, publication을 제외한다. Segment elapsed 기반 약1.2350 vs1.2537 update/s는
평가/IO 포함 값이다. 같은 updates/target의 비교이며 동일 FLOPs 또는 통제된
속도 benchmark라고 주장하지 않는다. 초기 tensor9513408 parameters와
vocab562는 실제 준비/파일 관측값이며 새로운 대형 모델 능력의 증거가 아니다.

LOCAL6개 segment durable step은1→1024→1024→2266→3553→4096,
GLOBAL은1→1024→1024→2296→3586→4096이다. 두1024 평가 재개는 각각
optimizer0, generation/teacher508/362회만 실행했다. 모든 시간 종료는
TIME_BUDGET 단독·저장 성공으로 확인한 뒤 이어갔다. 미커밋 input/target도
합산하며 추가 optimizer나 이미 반환된 평가행 재생성으로 보정하지 않았다.

**R5 결정:** 양군의 실행은 정상 종료했지만 개발 품질은 둘 다 FAIL이다.
Primary>=487, 각과제>=58/64, transfer>=116, CDE both>=87/96 조건을
충족하지 못했다. GLOBAL6 전환을 채택하지 않고 두 연구 artifact를 보존한다.
LOCAL의10개 primary 우위로 전체 local 구조가 우월하다고 주장하지 않는다.
실제 mask 차이는 train164/8192, primary9/512, transfer3/128의 작은 범위이고
seed1개 비교다. 새 balanced 점수와 과거 P6144 점수를 직접 증감률로 잇지 않는다.

미확정 항목 하나는 **고정 train의 C/D/E에서 질문/관측시각에 따른 양쪽 정답
전환과 정확한 인용을 학습하지 못한 병목**이다. 양군 train both0/12,
primary both0/96이며 raw에는 질문을 바꿔도 같은 답/근거 밖 인용 ID를 낸
사례가 있다. 이번 초기조건·4회 노출·예산에서 학습 미완성이라는 근거이며,
코드 결함·모델 용량 부족·local mask·저장 포맷을 단일 원인으로 확정하지 않는다.
추가 loss/LR/seed/노출 탐색을 자동 시작하지 않았다.

CODE_SCOPE=UNCHANGED_VERIFIED_SOURCE; DATA_QUERY_NECESSITY=INDEPENDENT_A_PASS;
NATIVE_INPUT_PARITY=PRIOR_VERIFIED_AND_P16_REUSED; STRUCTURE_TREATMENT_ACTIVE=true;
R3=COMPLETED_REGISTERED_BUDGET; R4=INDEPENDENT_RAW_USAGE_RECOUNT_PASS;
R5=NO_CANDIDATE_PROMOTION; TRAIN_FIT=INCOMPLETE; UNSEEN_BINDING=NOT_MET;
OLD_REFERENCE_AND_GATE=NOT_RUN_NO_ELIGIBLE_CANDIDATE; MODEL_QUALITY=FAIL;
final200/S4/S5/S6=NOT_RUN_PREREQUISITE_FAILED; GOAL1_READY=false;
GOAL1_ACCEPTED=false. Final200 내용 열람/새 생성0. 남은 optimizer 예산0이다.

실행 근거(EXECUTED_THIS_RUN): 고정 executable의 `fresh identifiable-prepare`
1회; LOCAL5/GLOBAL6의 `fresh run --root ...` 각6회,12회 모두 exit0;
1024와4096의 `fresh report` 각군2회,4회 모두 exit0. 기존 단위/전체 안정화
시험은 반복하지 않았고0-test를 PASS로 세지 않았다. 보조 metadata reader의
finished 파일 경로 오타1건은 파일 없음이었으며 실제 root의 파일을 읽어
확인했다. 모델 command나 raw를 재실행한 오류가 아니다. 검토 scratch의
컴파일/경로 오류와 교정 후 exit는 독립 보고서에 별도 보존한다.

허용된 로컬 증거 경로(모두 원자료 게시 금지):

- 연구: `artifacts/identifiable-baseline-20260921-r3/`.
- 각 arm의 `corpus.r3cor`, `transfer.r3cor`, `plan.r3b`, `metadata.r3b`,
  `tokenizer.r3b`, `initial.r3m`; root의 preparation/review-a/positions.
- 각 arm의 `eval-*.r3rows`, teacher/call/receipt, `segment-*/updates.r3rows`,
  train-control, started/finished 기록. 중간 raw를 saved endpoint로 부르지 않는다.
- 마지막 native는 각각 `LOCAL5/segment-0005/final`,
  `GLOBAL6/segment-0005/final`; 실제 physical SHA256
  9ae709df781074ad5c4b211840b5a326cc250aef9f1e3f07af6400f15c242d95 /
  5bd84314f2505b6706baf76a22ef2511cbe80ee3670843b6af7af1bcff893533.
- 실행 로그/최종 reader 출력/보존 hash:
  `artifacts/identifiable-learning-20260921-evidence/`.
- 독립 A: `artifacts/identifiable-review-a-20260921-r3/REPORT.md`;
  R4: `artifacts/identifiable-r4-review-20260921/`의 matched1024와 두 arm4096
  보고서·native 재채점·비용/층화 통계. 독립 검토는 원본을 수정하지 않았다.
- 실제 source diff:
  `artifacts/identifiable-baseline-20260921-evidence/candidate-code.diff`,
  base5c5fcf1f40080a8cbf3d79487ec1d27de235e1de→source936d42258a6143ae664fca71ef8294b760b1dffe,
  SHA256 fa193491bb11d26b651757824f7bd50fdd6aee2d6b109705d3d3cb944abd37e8.

종료 확인에서 P6144/GROUND native와 실패r2 control hash가 이전 값과 같았고,
실행 중 제품 소스 변경0, 원본/DB 삭제0, 미추적 .DS_Store도 보존했다.
이번 게시 대상은 이 상태와 기존 계획 문서뿐이며 source candidate와 report
commit은 구분한다. 큰 raw/corpus/weights/Adam/임시 지시문은 stage하지 않는다.

독립 최종 결과(EXECUTED_THIS_REVIEW)는
`artifacts/identifiable-r4-review-20260921/MATCHED4096_REPORT.md`, SHA256
ece9f91467d3f155ad6e12a28951041bfe33609f94176b8ed4d1f6952dca5922다.
MATCHED4096_EXECUTION_INTEGRITY=PASS / DEVELOPMENT_GATE=FAIL(양군)로 닫혔다.
각군13개 panel, 총4608 raw+4608 teacher 행과9216개 prepared/resolved 호출
쌍을 이전 중간 검토 및 신규 검토로 모두 확인했다. 연장 근거 보고서
MATCHED1024_REPORT.md SHA256
8cd16a47cb8a7a780ce00580907ab4aa0b76d2f325f798e2f964badebf0448c2와
LOCAL4096 원보고서도 덮어쓰지 않았다. 독립 검토의 무결성 PASS는
모델 품질/Goal1 수용이 아니며, 원본 평가나 제품 소스를 수정하지 않았다.

## 2026-09-21 사용자 인가 R3–R5 실제 학습 — 별도 등록

사용자가 저장 수리 후 R3–R5 실제 학습 진행을 명시적으로 승인했다. 실패 r2의
start/control/누락 final은 그대로 보존하고 새 root
`artifacts/identifiable-baseline-20260921-r3/`를 등록했다. Source는
936d42258a6143ae664fca71ef8294b760b1dffe, 시작 report HEAD는
7c7ada08c91e0334b533e11dec9ef4b8c6a8d467다. 미추적 .DS_Store 보존.
Rust1.98.1/locked offline/CPU F32 Accelerate/thread1, 새 의존성/모델/API0.
Production binary1eeea87e0c8d66fb5f99136e106c70fca37788422416b45478a94a45acc1e201,
compiled source f322766fb7ae198e1c5a6c8f6ad357676d35ebcdc529ca3f5b9a2083a92acb69.

새 preparation f5f548ecadddcecf71eadc9e2f9752ded0b23baeda61173bb956ab7eb39c3376.
기존 수용본과 같은 semantic corpus/모든 사례·정답, tokenizer, metadata,
sample tape, 초기 tensor 내용 af6abb8fd48cdd0d4396b86c543aee688efc770470c52adc0dcf7016acf40306이다.
Native corpus의 물리 hash 차이는 converted_at만 변경된 데 따른 것이며 변환기와
semantic 내용은 동일하다. LOCAL5/GLOBAL6 policy는 각각
4ba25d89b481c9b879a04e4cba5b4388da5378df745f46141d1ea907ef30b977 /
4348a01c119567e6bbf65c9c73c3a48da5cb58564076b8c0226246659c2ccb9a다.
예산/조건은 기존 각4096, 총8192, 각 input20M, active14400s,
generation/teacher 각6400 그대로이며1024 공동 비교 후에만 조건부 연장한다.

이번 준비 수치 SMALL forward8/backward2, optimizer0/generation0/teacher0.
기존 P16은 재실행하지 않고 그16 generation을 총 사용량에 계속 포함한다.
이전 실패의 실제 optimizer/input/target/generation/teacher0와0.912484459초는
보존·별도 보고한다. 새 준비 source/data binding의 독립 확인 뒤 첫 update를
저장하고 새 process 복원으로 진행한다. 독립 A binding은 PASS로 종료했다:
`artifacts/identifiable-review-a-20260921-r3/REPORT.md`, SHA256
6df045a1ea40b37128cd2ce818e9ef32da7018f6e26f5e7ae2087b35e73d8835.
실제 보고서를 승인 기록에 연결했다. 이는 모델 품질 PASS가 아니다.
증거: `artifacts/identifiable-learning-20260921-evidence/`.

## 2026-09-21 첫 실제 실행의 저장 실패 보존, tokenizer 출처 연결 수리

독립 A는 교정 source10b8f46e2e16809dc0b5719b89fbed4670e8832a와 preparation
e081286eeb1a345c8833f47e9961f77c2a079568ce03b4cc154d493aaaf8397d를 PASS로
수용했다. 보고서 `artifacts/identifiable-review-a-20260921-r2/REPORT.md`의
SHA256=f08588ad09c6517a8baa472a2c06472bf7ff12f8d89ba64c460ed9ca0cb196f4를
기존 native 승인 기록에 연결한 후 고정 binary27a1dcfda2cd7c3b6b4afb9d9849f27ddaa4e64a3058c6b6abdfba8d57323898로
LOCAL5 첫 command를 실행했다. A PASS는 자료/설계 사전수용이며 학습 성공이 아니다.

EXECUTED_THIS_RUN: `fresh run --root artifacts/identifiable-baseline-20260921-r2/LOCAL5`
는 exit1이었다. 첫 step0/start 저장과 보존용 final 저장이 모두
`corruption: checkpoint training state`로 실패했다. 실제 optimizer0,
input0/target0/padding0, generation0/teacher0, checkpoint_saved=false다.
train-control elapsed0.912484459s/cleanup0.075905292s. 마지막 사용 가능 파일은
학습 상태가 없는 initial.r3m이며, 성공한 step0 training checkpoint나 update1로
부르지 않는다. GLOBAL6은 실행하지 않았다. 새로운 비교 품질은 NOT_MEASURED다.

원인은 새 corpus/tokenizer 입구와 native 저장 불변식의 불일치였다. 새 연구는
P의 tokenizer mapping만 재사용하는데 초기 TrainingState의 previous_corpora가
비어 있어 native validator가 tokenizer 원본 corpus의 출처를 확인하지 못했다.
기존 checkpoint 검증을 완화하지 않고, 명시적으로 mapping을 재사용하는 새 상태
생성에만 그 출처를 기록했다. 이는 옛 corpus로 새 weights를 학습했다는 뜻이 아니다.
기존 resume에는 자동 보충하지 않는다. corpus 로그에 mapping-only provenance를
구분했고 모델 구조/손실/LR/저장 포맷/원본 파일은 변경하지 않았다.

직접 회귀는 수정 전 native 저장에서 같은 오류로1 FAIL, 수정 후6/6 PASS 및
그 안의 새 process exact test1 PASS다. child는 실제 native step0/zero Adam을
재로드했고 출처 누락·잘못된 출처·다른 tokenizer를 거부했다. generic plan은
재사용 출처를 자동 생성하지 않는다. 신규 optimizer/generation/teacher0이며
수정 과정의 첫 test compile 오류는 별도 보존했고 test 실패와 혼동하지 않는다.
failed SMALL root/started/control/빈 updates와 모든 raw는 그대로 보존했다.
실패의 final/terminal을 만들거나 기존 study를 자동 재시작하지 않았다.

허용 증거는 `artifacts/identifiable-baseline-20260921-evidence/`의
local5-segment-0000.log, review-registration.log, tokenizer-provenance-red-test.log,
tokenizer-provenance-green.log, build-provenance-fix.log 및 실패 root의
LOCAL5/segment-0000/train-control.r3b다. 원자료는 게시하지 않는다.
현재 누적 SMALL optimizer0, TINY optimizer0, generation16(P parity만), teacher0.
구현자 수치 시험은 SMALL24 forward/6 backward(세 준비), TINY48/12;
독립 A 두 실행의 TINY16/4는 별도다. tokenizer 출처 저장 시험에는 forward0.

CODE_SCOPE=REPAIRED_WITH_DIRECT_REGRESSION; DATA_QUERY_NECESSITY=INDEPENDENT_A_PASS;
STRUCTURE_TREATMENT_ACTIVE=true; R3=STOPPED_STORAGE_FAILURE_BEFORE_UPDATE1;
TRAIN_FIT/UNSEEN_BINDING/OLD_REFERENCE_AND_GATE=NOT_RUN; MODEL_QUALITY=NOT_MEASURED;
S4/S5/S6=NOT_RUN; GOAL1_READY=false; GOAL1_ACCEPTED=false.
이번 자료/구조 비교로 품질이 개선됐거나 어느 mask가 우월하다고 말할 근거는 없다.
수리 후보936d42258a6143ae664fca71ef8294b760b1dffe는 별도 독립 검토도 PASS했다.
`artifacts/identifiable-review-a-20260921-repair/REPORT.md`, SHA256
77f0abac6152229244706db327a250787cc65056854498ed5d3ac76a29bc7bfd.
검토자는 동일 native process 회귀를 직접 부모1/1·자식1/1 실행했고 모두 통과했다.
추가 optimizer/forward/generation/teacher0. 이 수용은 코드 수리만이며 실패 연구의
재시도 인가가 아니다. Source digest=f322766fb7ae198e1c5a6c8f6ad357676d35ebcdc529ca3f5b9a2083a92acb69,
수리 binary=1eeea87e0c8d66fb5f99136e106c70fca37788422416b45478a94a45acc1e201.
구현 source와 이 상태 보완의 report-only commit SHA는 구분한다.
수리 source push 후 실제 원격936d42258a6143ae664fca71ef8294b760b1dffe 일치를 확인했다.
종료 때 원본 GROUND/P native와 새 LOCAL5 initial physical hash도 변함없었다.
학습 예산8192회는 사용0이지만 실패 연구에 자동 이월/재개하지 않는다.
남은 항목은 별도 재실행 인가 이후 실제 두 군의 학습·품질 비교 및 조건부 S4/S5/S6다.
최종 범위 판정은 PARTIAL이다. 자료/수리 검증을 학습 완료나 Goal1로 바꾸지 않는다.

## 2026-09-21 독립 A 지적 두 경계 수정 — 재검토 대기

독립 검토는 source20c282272169f71e2526e714c7559bf5f684d3e3을 FAIL로 닫았다.
512점수에 고정된 회귀 guard와 동일 입력인 G/H 두 view가 이유다. 기존 준비
자료와 FAIL 보고서는 보존했으며 그 자료로 SMALL 학습을 시작하지 않았다.

기존 raw-panel 검사/고정64 subset을 재사용해 attained-best>=32, 감소>=12,
CE>=1.2배, 연속2회라는 기존 guard 의미를 복구했다. Step0의32문항은 제외한다.
1024에서 향상한 뒤2048/4096 하락, 중간 회복, 미학습 초기값 보호를 직접 검사했다.
G는 근거 추가/대상 일치/관측 동률 해소 중 한 관계를 변경한다. H는 관측시각을
교환해도 인과 유보를 유지한다. 모델 입력이 동일한 두 view는 검증에서 거부한다.

수정 후 관련 test5/5 PASS(0-test 아님), optimizer/generation/teacher0.
새 별도 준비 root는 `artifacts/identifiable-baseline-20260921-r2/`다.
기존 tokenizer/seed17/초기 tensor 내용은 유지했다. 새 corpus physical
0dbc6824f6b5ef3e56de88eb86827b7dd66221ea1cd35f5c85b6bb1770e383d7,
preparation e081286eeb1a345c8833f47e9961f77c2a079568ce03b4cc154d493aaaf8397d.
실제 input>256은 train164/8192, primary9/512, transfer3/128이며 층당
제거 edge는2234/121/7이다. 같은 고정 tape의 arm당 계획 input6883740,
target598104는 실행량이 아니다. 수정 준비의 SMALL 수치8 forward/2 backward,
optimizer0; 직접 TINY 수치8 forward/2 backward, optimizer0을 별도 기록했다.
기존 고정 P16은 재실행하지 않았다.

Compiled source920500d09431c3f525bf6d5526d6b25fa25182052387f5d5a00f87a74e3a6e43,
production binary27a1dcfda2cd7c3b6b4afb9d9849f27ddaa4e64a3058c6b6abdfba8d57323898.
허용 증거는 기존 evidence root의 tests-correction/build-correction/prepare-correction/
summary-correction 로그와 `artifacts/identifiable-review-a-20260921/REPORT.md`다.
해당 FAIL 보고서 SHA256은06e4684a57cc7d82658a485c1e4df433ca085ffd91d42906d035beb6f468413c.
INDEPENDENT_A=REVIEW_REQUIRED_FOR_CORRECTED_CANDIDATE; NEW_SMALL_UPDATES=0;
MODEL_QUALITY=NOT_RUN; S4/S5/S6=NOT_RUN; GOAL1_READY/ACCEPTED=false.

## 2026-09-21 Identifiable baseline R0–R2 준비, 독립 A 대기

CONTRACT=R3-IDENTIFIABLE-BASELINE-1.0. 시작 HEAD는
5c5fcf1f40080a8cbf3d79487ec1d27de235e1de이며 기존 source42af900c2f52264ed629e95560f63e7ae493108a
이후 제품 변경은 없었다. 미추적 .DS_Store 보존. Rust/Cargo1.98.1,
locked/offline, Accelerate/F32/CPU/thread1. 새 의존성/외부 모델/C++ 실행0.

EXECUTED_THIS_RUN: GROUND 실제 native를 읽어 step6634/objective5/Adam136 tensors,
CANCELLED/resume=false와 physical d5bbf20570ee3e372f863a81ad966b5368a50f1027464f9df854d0fce6f2bc59를
종료 기록과 대조했다. 새 forward/optimizer/generation/teacher0. +490 품질은
여전히 NOT_MEASURED이며 마지막 +256의350/512·67/128·both0를 대입하지 않는다.

기존 LibTorch REPORT뿐 아니라 저장된 두 state/metrics와 generation usage 파일도
기존 Rust 순수 reader로 읽었다.256 clocks와 입력/target counts 일치, finite 확인.
수치 차이는 기존 범위와 같았다. 이는 DERIVED_EXISTING_RAW이며 새 학습·C++ 실행이나
독립 수용이 아니다. 공유 자료/설계 전체의 정확성을 증명하지 않는다.

R1은 실제 native pool, frozen hashes, 실제 updates.r3rows의 sample_indices를
policy의 draw와 대조했다. 아래 노출은 해당 fork만이며 부모의 학습량을 합산하지
않는다. scene은 값/순서/질문을 제외하되 ID·대상·구역·관측시각·상태를 유지한다.
빈 근거는 같은 evidence scene이 될 수 있어 semantic base 수도 별도 보존한다.

| Pool | 물리 행 | semantic bases | 값 제외 evidence scenes | query bindings | 실제 노출 행 | 노출 min/median/max |
|---|---:|---:|---:|---:|---:|---|
| P-PHRASE |16384|2048|1963|11422|16384|1/1/1|
| FIT |32768|2048|2219|16024|6448|0/0/80|
| VALUE |32768|2048|2219|16024|6496|0/0/40|
| COVER |32768|2048|2219|16024|6784|0/0/10|
| COVER4 |32768|2048|2219|19096|7168|0/0/5|
| GROUND |32768|2048|2219|19096|3218|0/0/2|

모든 pool의 full entity는1361개다. 값 배치 수는 P/FIT/VALUE/COVER3420,
COVER4/GROUND4752다. 원본 primary512, transfer128, selector 반대쪽192와
보존된 VALUE recombined48의 실제 사례도 감사했다. label 해석 불일치,
동일 prepared prompt의 상충 정답, 필수 근거 제외는 검사 범위에서0이었다.
전체 pool과 실제 노출 가중 규칙 점수는 별도로 저장했다.

P 학습의 D에서는 질문을 읽지 않는 lower-context 규칙이2048/2048,
primary D에서64/64, 반대 selector D에서0/64다. shorter-entity 규칙은
P train C1792/2048, primary C56/64, 반대 selector C8/64다. 타이인 규칙은
정답으로 tie-break하지 않고 abstain 처리했다. GROUND pool의 D는 원본 범위
패턴2048/4096이므로 원본 generator의 편향을 파생 pool 전체로 일반화할 수 없다.
LABEL_INTEGRITY=VERIFIED; QUERY_NECESSITY=ORIGINAL_D_NOT_ENFORCED;
NUISANCE_BALANCE=ORIGINAL_C_D_BIASED; TRAIN_SUPPORT=MEASURED_PER_FORK;
EVAL_NOVELTY=SCENE_WORDING_VALUE_VIEWS_REPORTED_SEPARATELY. 모델이 실제로
이 규칙을 쓴다는 인과 결론은 아니다. 삭제된 초기화 이전 자료는 NOT_AVAILABLE.

R2는 별도 joint-binding-balanced-v1을 native로 준비했다.8192/512/128,
train4096 semantic bases, C/D/E의 양쪽 query/시간 관계를 모두 포함한다.
기존 request-only resolver, R3CORP 재로드, 전체 evidence, split 누수와
정확한 선언 marginal cross-table 검사를 통과했다. 기존 tokenizer mapping562만
재사용했고, 모델은9,513,408 parameters의 새 seed17이다. LOCAL5/GLOBAL6의
weight-content af6abb8fd48cdd0d4396b86c543aee688efc770470c52adc0dcf7016acf40306 일치,
architecture semantic ID는 다르다. 실제 새 Adam zero moments hash와 clock0도 기록했다.

최종 sequence(min/median/max)는 train77/225/269, primary79/225/264,
transfer81/224/259다. 실제 input>256인 사례는159/8192,8/512,3/128;
층당 제거 edge 합은2194,118,7이다. STRUCTURE_TREATMENT_ACTIVE=true지만 작다.
masked edge와 질문 정보의 완전 소실을 동일시하지 않는다. 실제 SMALL의76-token
forward/gradient는 두 구조가 정확히 같았고,268-token cached/full 최대차는
LOCAL1.1920929e-6/GLOBAL1.3113022e-6이다. mask257에서1개,268에서78개 차이.
고정4096 tape의 arm당 예정 input6815880/target578012는 실제 학습 사용량이 아니다.

관련 unit4개와 P16 parity1개 PASS, 모두0-test 아님. 기존 전체 안정화/quick을
반복하지 않았다. 마지막 정확 실행은 `cargo test --locked --offline --release
--features accelerate --bin replica-train identifiable_ -- --nocapture`(4/4),
별도 exact/ignored `training::fresh::tests::stabilization_fixed_parent_parity`(1/1).
P16은 raw160 tokens,16/16 일치, teacher0, 원본 inventory unchanged다.
새 process의 `fresh run`은 review A 부재로 exit1, segment 생성/학습0으로 거부했다.
수정 중 compile 오류2건과 이전 draft 준비를 보존했고 PASS로 합산하지 않았다.
서로 다른 source의 draft에는 최종 hash를 덧붙이지 않는다.

현재 최종 compiled source digest6e6f244279c3a48af70bd46decb2c15bd402c6a4d6d663cc46a1026b9ded440a,
production executable a9af3e5d2d3e6202bd58c16706ef56dde28a55dfbe5ada13aaceb61ec9548d96.
Preparation physical247a768ff388d2d31a45fc64003a6f737dc8aa81a44625c72f5d067870655075.
Source/report Git SHA는 게시 기록에서 별도로 식별한다.

허용된 로컬 증거: `artifacts/identifiable-baseline-20260921/`의 preparation,
LOCAL5/GLOBAL6 plan/native/corpus/tokenizer/metadata와
`artifacts/identifiable-baseline-20260921-evidence/`의 audit-final, parent16,
pure numeric/usage recount, 각 build/test/실행 로그. 원본이나 데이터는 게시하지 않는다.
Draft `artifacts/identifiable-baseline-20260921-preparation-01/`도 보존했으나 학습용 아님.

ACTUAL_NEW_SMALL_UPDATES=0; NEW_TINY_UPDATES=0; GENERATION=16_RETURNED;
TEACHER=0; UNKNOWN=0 for completed observations. 수치 forward는 SMALL16/backward4
(draft+최종 준비), TINY32/backward8(직접 시험 네 실행)이며 생성과 구분한다.
CODE_SCOPE=R0_R2_PREPARATION; NATIVE_INPUT_PARITY=PASS;
INDEPENDENT_A=PENDING; TRAIN_FIT/UNSEEN_BINDING=NOT_RUN;
OLD_REFERENCE_GATE=NOT_REOPENED; MODEL_QUALITY=NOT_IMPROVED_THIS_RUN;
S4/S5/S6=NOT_RUN; GOAL1_READY=false; GOAL1_ACCEPTED=false.
다음 의존은 독립 A의 실제 데이터/실험 사전수용이다. 실행자가 대신 PASS하지 않는다.

## 2026-09-20 사용자 요청 중지 — 현재 계보 종합 보고

사용자의 중지 요청으로 GROUND 프로세스에 SIGINT를 전달했다. 기존 RunControl이
취소를 관측하고 checkpoint를 저장한 뒤 command exit1로 종료했다. Goal 상태는
paused다. 학습/평가를 재시작하지 않으며 취소된 run의 resume=false를 변경하지 않는다.
아래 과거 비교는 보존된 상태 보고를 다시 읽은 DERIVED_EXISTING_REPORT다.
이번 GROUND 실행·중지·원시 행 재검산은 EXECUTED_THIS_RUN/DERIVED_EXISTING_RAW다.
과거 모든 실험을 이번에 다시 실행하거나 독립 재수용한 것은 아니다.

**실제 중지 상태와 사용량**

| 항목 | 확인 결과 |
|---|---|
| 실행 source |42af900c2f52264ed629e95560f63e7ae493108a; 학습 전 origin/main 일치|
| compiled source digest |8c788660847293ae99ae8d955cfcd5256725cea5973118bc18b4d09fbc349115|
| 고정 CLI SHA256 |e826ba0076151ed708f8e2e7972d14432f185f5c942f38740ddb5dac160e763f|
| policy logical digest |dacdc24ed8a0024e6a1e98789dc8ff165486b786ea8a49b1798482627291fccd|
| 실제 신규 SMALL optimizer |490 = 첫 process1 + 다음 process489; absolute6144→6634|
| 실제 input / target |838,299 /60,205; 미커밋 계산 포함|
| optimizer에 반영된 input / target |836,611 /60,073|
| 취소 때 미반영 input / target |1,688 /132; 사용량에서 제외하지 않음|
| generation / 자체 teacher |학습 평가1,160/1,160; 별도 P16 포함 generation1,176|
| 준비 TINY |optimizer50 /generation333 /teacher333; 실패 두 번 포함|
| 시간 |두 학습 command 합671.302171792초; 준비/빌드/P16과 구분|
| 마지막 저장 |GROUND/segment-0001/final, step6634, family5, Adam136 tensors|
| 종료 |CANCELLED, phase Failed, resume=false, checkpoint_saved=true|
| 저장/수치/UNKNOWN 오류 |관측 없음; work_error는 요청된 cancelled|
| 관측 최대 sampled RSS |1,546,144KiB; S6 성능 검증 아님|
| 마지막 전체 품질 평가 |step6400(+256); 중지 checkpoint6634는 최종 품질 미측정|

한도1280 중790회는 실행하지 않았다. 이를 다음 실행에 자동 이월하지 않는다.
마지막 step6634 raw update의 input 누계11125315와 terminal input11127003의
차이1688은 취소 전 미반영 계산이다. terminal의 target817009는 committed 상태,
사용량 보고의 executed target에는 추가132를 포함한다. 값의 차이를 손상이나
추가 optimizer 실행으로 잘못 해석하지 않는다. 순수 Rust reader가490개 update의
연속 clock/고정 LR, 사용량 합계, native objective/state와 terminal 파일 hash를 검산했다.
모델 forward/generation/teacher/optimizer를 추가 실행하지 않았다.

**구현된 구조와 저장의 현재 상태**

자체 random-init 계보의9,513,408-parameter SMALL이다. 6 layers/hidden384/
FFN1024, Q8/KV2/head48 GQA, pre-RMSNorm와 QK-RMSNorm, RoPE, SwiGLU,
tied embedding/output, local256 다섯 층과 global2048 한 층을 사용한다.
이는2017 원형을 그대로 옮긴 구조가 아니지만, 현대적 구성요소를 사용한 것과
현재 교육용 QA 품질 수용은 별개다. 자체 train-only BPE vocab562, CPU F32/
Accelerate/thread1이며 외부 학습 weights/tokenizer/teacher/API는 없다.

제품·학습·검증의 구현은 Rust/Candle이다. 사용자 승인 격리 C++ LibTorch 비교는
`artifacts/libtorch-parity-20260920/` 안의 별도 실험이며 제품/Cargo에 연결하지 않았다.
모델 weights와 Adam/objective/state는 R3MODEL, source corpus는 R3CORP,
train cache는 R3TOK, typed 실험 기록은 R3ER, metadata/IPC/row는 R3BIN이다.
직접 JSON 사용/의존은 제거됐고 범용 라이브러리 내부 의존은 허용한 범위에 남는다.
SQLite는 사용자 기억/관계의 기존 기능으로 유지하며 모델 tensor 저장소가 아니다.
이 저장 변경만으로 품질 개선을 주장하지 않는다. JSON→binary가 품질 저하의
원인이라는 통제 비교 증거도 현재 없다. 초기화 전 원자료는 사용자 승인 삭제로
현재 검산할 수 없으며 이번에 복원하지 않았다.

**초기화 이후 주요 실제 학습 비교**

Primary는 고정512, transfer는 별도 표현128, both는 원본/뒤집기 양쪽을
전체 답변·인용·EOS까지 맞힌192쌍이다. 다음은 각 endpoint의 점수이며 서로 다른
fork의 updates를 한 모델의 누적 학습으로 더하면 안 된다. 모든 행이 같은 loss/
자료인 것은 아니며, 통제된 해당 비교군 내부에서만 인과적 해석을 제한한다.

| 실험 / 개입 | 신규 updates | Primary /512 | Transfer /128 | Both /192 |
|---|---:|---:|---:|---:|
| Fresh random baseline |4096|392|40|당시 미측정|
| C-REPEAT |2048|425|44|당시 미측정|
| P-PHRASE: 질문 표현 노출 |2048|425|79|후속 관측0|
| SPACED: pair 간격128 |256|375|65|0|
| ADJACENT: pair 간격1 |256|367|70|0|
| ADJACENT 후속 고정 노출 |추가3584|368|67|4|
| first-target 가중치4 |3840|339|67|7|
| LR9e-5 |1280|330|60|0|
| COBATCH: 같은 batch의 pair |1280|371|60|0|
| CONTRAST: 합산 margin |1280|366|68|0|
| SIDE: 각 side margin |1280|372|70|2|
| REPLAY: 같은 pair 반복 |1280|369|64|6|
| WIDE: 반복 장면 확대 |1280|375|69|5|
| FIT: 적은 train pair 집중 |1280|398|76|0|
| VALUE: 값 교환 노출 |1280|393|72|0|
| COVER:32 장면/두 값 view |1280|392|68|3|
| COVER 고정 추가 노출 |추가2560|376|64|9|
| COVER 질문 표현 추가 노출 |별도 추가2560|386|67|12|
| DIVERSE:8 장면/네 값 view |1280|399|70|0|
| COVER4:32 장면/네 값 view |1280|375|67|2|
| GROUND: record attention 보조 loss |256 평가 /490에서 취소|350|67|0|

앞선 C-KEEP/S-SELECT 연구는 C2048회, S1024회에서 S의 품질 중단으로 종료돼
동일 예산 최종 비교가 아니다. 후속 안정화 R2와 고정 원자료 관측 R3의 독립 수용은
실행 경계/관측 신뢰성 범위이며, 이 표의 모델 품질 수용이 아니다. 초기화 전의
학습·저장·guard·raw close 수리는 각 역사 절에 남겨두되 현재 모델 계보와 섞지 않는다.

**배운 점과 미확정 원인**

FIT는 실제 노출된48개 전체 답변과24개 양쪽 pair를 모두 맞혔다. VALUE도
원본48/48 및 학습한 값 교환48/48을 맞혔지만 새 값 조합은22/48, both3/24였다.
DIVERSE 실제 학습 네 view는178/192, both83/96이며 각 입력20회 노출이었다.
반면 새 개발 both는0/192였다. 따라서 모든 종류의 학습이 전혀 안 되는 상태는
아니며, 학습한 사례를 벗어난 값·질문 조건·인용 연결의 일반화가 핵심 미달이다.

COVER 추가2560은 실제 train subset의 정답을 개선했지만 primary392→376,
transfer68→64, both3→9였다. 다른 다섯 과제 합계는297→300/320이고 원래
C/D/E 선택은95→76/192로 떨어졌다. 따라서 이를 모든 기존 능력의 망각으로
단정할 수 없다. 원본·반대 조건을 동시에 맞히는 선택 기능이 여전히 부족하다.
COVER4에서 각 사례5회 노출이 충분하다고 입증된 것도 아니다. 반복 수 부족과
데이터 커버리지 부족은 남은 가설이지만 무조건 더 돌릴 근거는 아니다.

Rust/LibTorch는 동일 부모/Adam/자료로 각256회 학습했고, 학습38/48,
개발45/64로 동일했다. fresh process112개 출력의 모든 토큰이 일치했다.
one-step 수치 대조도 사전 허용오차 내였다. 이는 시험한 경로에서 Rust/Candle
특유의 계산 문제가 설명력이 약함을 뜻하며, 공유한 설계/입력의 모든 결함을
배제하거나 PyTorch 전환만으로 개선된다고 말할 근거는 아니다.

고정 DIVERSE24입력 관측에서는 질문과 두 기록이 실제 제공되고 가려지지 않았다.
관측 forward와 일반 forward logits는 같았고 기존 greedy 첫 토큰과도 일치했다.
개발 첫 토큰 정답은3/12로 낮았다. 정답을 맞힌 train도 선택 기록의 attention이
항상 더 높지는 않았다. attention 크기는 원인 증명이나 정답 보증이 아니다.

GROUND의256회 대조는 COVER4 대비 primary347→350(획득3/손실0),
transfer67→67, 뒤집기10→8(획득0/손실2), both0→0이다. 미세한 일부 차이이며
품질 회복 증거가 아니다. 요청 취소로1280 endpoint 비교는 NOT_RUN이다.
490회 checkpoint에256회 점수를 붙이거나 취소를 정상 완료로 바꾸지 않는다.

**최근 구현 검증과 실패도 포함한 범위**

최근 source는 training.rs의 학습 보조 loss, fresh.rs의 train annotation/등록/
동일 자료 검증, checkpoint.rs/artifact.rs의 objective family5, 직접 tests와
기존 문서만 바꿨다. 새 tracked 파일은 없다. scalar/finite-difference/mask/
무작위 native gradient/annotation/tape/native 저장·재개/실제 process/P16을 검증했다.
첫 process 실패는 schema 연결 누락으로 학습 전에 거부된 것이며 수정했다.
두 번째는 Q/K가0인 EOS fixture에서 가중치 차이를 요구한 잘못된 assertion이었다.
이를 실제 random-model gradient 시험과 fixture objective 증가 검증으로 분리했다.
실패를 PASS나 실제 SMALL 품질로 세지 않았다. 이 취소 뒤 loader 재검산의 첫 시도는
오래된 top-level rlib을 링크해 새 family5를 거부했다. 현 production dependency
rlib으로 다시 읽은 결과 성공했으며 두 로그를 모두 보존했다. 모델 파일 수정0이다.

**현재 인계와 판정**

실행 code candidate는42af900c2f52264ed629e95560f63e7ae493108a다. 코드 diff:
`git diff 6321efe2ddea8eb899046dc0d5a4190391898001 42af900c2f52264ed629e95560f63e7ae493108a -- src tests docs`.
이 절을 게시한 commit은 report-only이며 학습 source와 구분한다.
원자료는 로컬 ignored artifacts에 보존하고 Git에 업로드하지 않는다.

- 부모: `artifacts/fresh-exposure-phrase-20260919/P-PHRASE/segment-0003/final`.
- 현재 연구: `artifacts/record-grounding-20260920/GROUND/`.
- native corpus: 위 연구의 `training-values.r3cor`,32768 train/512 validation.
- 마지막 저장: 위 연구의 `segment-0001/final`; physical
  d5bbf20570ee3e372f863a81ad966b5368a50f1027464f9df854d0fce6f2bc59;
  model452e912122f6df952a8cbbd02686326dac0c935d4534204ef805d11ccc8b7c55.
- 종료: `segment-0001-finished.r3b`; physical
  94ac8cc3196a360769c66269aa87327ccda0401bfd1c56c37f166f0b24835596.
- 마지막 평가: `eval-6400-{train64,dev512,transfer128,selector192}.r3rows`;
  판정 `paired-6400.r3b`; modeld17d99d4a15ff8dd37e36e1780f06617a3dfd0306c5fd4faa7a270b8eb73779b.
- 실행/실패/시험/재집계: `artifacts/record-grounding-20260920-evidence/`의
  `segment0000.log`, `segment0001.log`, `compare6400.log`, `stop-recount-current.log`,
  `candidate-source.diff` 및 개별 test/build/input/reader 로그.
- 격리 C++ 대조 보고: `artifacts/libtorch-parity-20260920/REPORT.md`.

CODE_PREPARATION=PASS; GROUND_STUDY=USER_CANCELLED_INCOMPLETE;
DEVELOPMENT_JOINT_PASS=false; MODEL_QUALITY_RECOVERED=false;
final200=NOT_CREATED/NOT_OPENED; S4 미통과; S5/S6 새 후보 수용=NOT_RUN;
GOAL1_READY=false; GOAL1_ACCEPTED=false; 목표 실행=PAUSED.

재개 시 권장은 동일한 작은 변수 실험의 자동 반복이 아니라, 보존된 source/raw를
독립 검토에 넘기고 데이터의 장면/값/표현 커버리지와 선택·복사의 학습 설계를 먼저
검토하는 것이다. 기준을 낮추거나 더 많은 step 자체를 개선 근거로 삼지 않는다.
이 권장은 신규 실행 등록이 아니다. 사용자 요청 전 추가 학습/진단을 실행하지 않는다.

## 2026-09-20 GROUND preparation verified; learning not yet executed

Under the continuing one-variable authorization, the existing trainer now adds
the preregistered record-attention loss only for GROUND training. It uses actual
last-layer Q/K/mask tensors from the same forward, first-response queries and
verified owned train-record spans. No labels or generation changes enter the
product. Native objective family5 binds exact annotation/policy using the
existing descriptor layout. All input, stop, usage and pure-report entry points
include the new policy. No new framework, tensor format or tracked file.

Direct checks executed with installed Rust1.98.1, locked/offline/Accelerate and
thread1: scalar loss/finite-difference gradients/mask/no-selector boundaries;
random native parameter-gradient and identical-logit check; train annotation
and native input equality; full3840-row tape writer/reader; native objective
roundtrip/default-loss rejection; actual TINY process resume. Each named check
executed at least one test. Final process test PASS1 in15.40s, comparing two
updates continuously with1+1 in separate processes, exact weights/Adam/cursor.
The16-row TINY tape/all views are validated separately without repeated learning.

Two failed process executions remain in local evidence, never counted PASS:
first exit101 after4 setup updates rejected an omitted schema allow-list before
GROUND training; second exit101 after37 updates passed16 versus1+15 state
equality but failed a wrongly asserted weight difference in the degenerate EOS
fixture. That fixture sets Q/K to0; its auxiliary derivative is0. The corrected
test checks increased objective with identical draw/CE/LR and uses a separate
random native model to prove nonzero Q/K/norm/embedding gradients. This fixture
explanation does not diagnose the SMALL model or claim quality recovery.

Preparation totals including failures: TINY optimizer4+37+9=50; generation/
own-teacher75+129+129=333 each, below64/768/768 caps. Scalar optimizer0;
direct random-model forward2/backward1 and scalar backward4 are separate.
Production release build PASS21.58s. P6144 fixed16 parity PASS1 in14.44s,
16/16 exact raw outputs,160 tokens, SMALL optimizer0/teacher0, originals unchanged.
Frozen production executable SHA256
e826ba0076151ed708f8e2e7972d14432f185f5c942f38740ddb5dac160e763f;
diagnostic executable28e3e16badd8240739c0bdcbf0c5e3783066c2dba922083a6595b888fceab190.
Evidence: `artifacts/record-grounding-20260920-evidence/`, including both failed
process logs, final process/numeric/annotation/tape/native/build/P16 logs and
preserved executable. Existing full-suite/clippy acceptance is not claimed.

CODE_PREPARATION=PASS, SMALL_LEARNING=NOT_RUN. Next register the separate
GROUND1280 policy and verify exact COVER4 content/tape/parent/Adam/LR before
the first update. Quality unchanged; final200 NOT_CREATED/NOT_OPENED;
S4/S5/S6 and GOAL1_READY/GOAL1_ACCEPTED remain false.

## 2026-09-20 selector attention observed; mask-loss explanation unsupported here

Corrected source58a6f97583a545b50ace20c7a936af16ad769276 was pushed and matched
origin/main before observation. Frozen executable
bd95a5da32c35fcbe7b6ca604519db4d8f82aa821a12c2569d4d462628034466 executed exactly
`training::fresh::tests::paired_selector_attention_diagnostic` in a new process:
executed1/PASS1, exit0,10.04s including input/native audit. Native DIVERSE7424
physical remains60c772b9ff71ae20438c4e33f535d53e73b1d4ef885ef3d041da704f6ebb5416,
weightsa0c36fc9aac19696d245eca60dfb130de290bb798eb25948f65d1e501c6c7a1c.

First two executed view0 train pairs and first two view0 development pairs per
C/D/E give24 inputs, selected without looking at outcomes. Each has one observed
and one reference prompt forward:48 own-model teacher-budget calls,9752 prompt
tokens, RunControl2.6192165s, optimizer/generation0. No target prefix was supplied.
All24 observed/reference last-position logits are exactly identical and their
argmax agrees with the preserved greedy raw. All prompts186–241 tokens retain
both records and the question; no question region is masked at any of48 layer/
head observations per case. This rules out missing/masked question input for
these cases, not for arbitrary long prompts or as a proof of semantic use.

| Fixed diagnostic sample | First-token correct /12 | Existing full /12 | Both full /6 |
|---|---:|---:|---:|
| Actual trained scenes |11|11|5|
| Development scenes |3|2|0|

C/D/E first-token train4/4/3 of4, development1/1/1 of4. Full train4/4/3,
development1/0/1. Existing whole-answer scores are reuses, not fresh generations.
Final-layer mean selected/other-record attention mass on development is
C0.336/0.343, D0.362/0.348, E0.366/0.339. Even correct train cases need not have
selected-record mass dominance (train D0.383/0.420); neither these averages nor
head visualizations establish the causal mechanism. No unique storage,
tokenizer, optimizer or attention implementation cause is established.

Pure Rust `recount.rs` verified all24 cases,48 prepared/resolved call receipts,
native/raw physical hashes, observation/reference logits, original raw token,
complete counts, finite normalized attention and usage; exit0, no model calls.
Evidence `artifacts/selector-attention-20260920-evidence/` includes preserved
initial compile/pre-call failures, corrected tests/executable, observation log,
`observation-content-bound/` cases/attention/start/finish/call binaries and reader.
Raw SHA25617574bd4c6c3022c48f0368e971725a2760993d453744c82b6dc06455fedf4ee;
finished55a0c18f645f1c2077b71471269aa6d90e85586fc211f1109f03fdbe18c4cc44;
recount36af0b841fdcef63e3b7091026ad04b8ca9f4eadbc3ee362559a92c66dc8a255.
No unknown/failed model calls. Initial failed preparation used0 forwards.
TINY direct observer regression used5 forward/2 backward, optimizer0;
all SMALL optimizer0/generation0/teacher-forwards48 for this diagnosis.

DIAGNOSTIC_EXECUTION=PASS, MODEL_QUALITY_RECOVERED=false. Existing DIVERSE
development399/70/both0 and COVER4 375/67/both2 remain unchanged. Final200 remains
NOT_CREATED/NOT_OPENED; S4/S5/S6 unmet; GOAL1_READY/GOAL1_ACCEPTED=false.
Next single hypothesis is a fixed train-only record-grounding loss with the
same COVER4 data and P parent. It is not implemented or executed at this entry;
direct gradient/native-resume gates and a new bounded registration precede it.

## 2026-09-20 attention pre-call association corrected

Initial source36b26fc50e37b7b3b780dcf1b72e0bfec07ad017/executable47551a9…
returned exit101 in7.66s before output directory or model calls. The merged
native pool deliberately prefixes unique container IDs while retaining exact
request/target bytes; the original train48 raw uses unmerged IDs. Diagnostic
lookup now requires a unique native-prompt digest and expected answer, retaining
native/model/EOS/raw validation. Original failure/log/binary are preserved.
Extended framing/content test executed1/PASS1 (0.00s), including duplicate,
different-prompt and different-answer rejection. SMALL/TINY model calls0 for
this correction. No production inference/data/weights change; no generation
retry or budget extension. Corrected observation remains NOT_RUN here.

## 2026-09-20 fixed selector-attention preparation verified

After COVER4 closure0f100e63a31eda40b1c48e0193c527a82b1f52b6, add only an
ignored fixed-weight diagnostic and its trusted-framing test in existing
`src/fresh.rs`. Product model/trainer/generation and native formats are unchanged.
Existing RunControl/confirmed publisher/attempt receipts and forward observer
are reused. Exact policy/case/raw/native binding precedes any model observation.
Select24 cases by train tape/development metadata order, not correct predictions;
cap48 prompt-only forwards (24 observed +24 reference), optimizer/generation0.
No learning policy or extra quality budget is registered here.

Initial compile failed on two local tuple/error conversion types, before tests
or model calls. Corrected framing unit executed1/PASS1 (0.00s); existing native
observer forward/gradient/cache regression executed1/PASS1 (0.01s). Actual TINY
5 forward calls,2 backward calls, optimizer/generation/teacher0. SMALL calls0 at
preparation. Locked/offline/Accelerate, Rust1.98.1, thread1. No suite-wide PASS.
Frozen production-feature diagnostic executable
47551a9a36e4639a7849e11f6ceb513dff4453d98182c243eb8d9ab75adb209f,
under `artifacts/selector-attention-20260920-evidence/`, alongside failed and
passing build/test logs. Runtime diagnostic NOT_RUN at this registration.
Attention is observational evidence, never a causal proof or quality substitute.

## 2026-09-20 COVER4 finalized: more scenes did not recover joint quality

EXECUTED_THIS_RUN / DERIVED_EXISTING_RAW; independent acceptance not claimed.
Source a132f3aa4888690447b3c59a63970a16062e6903 was pushed and remote-matched
before execution. Frozen production binary
d3d38d10f6cdeda4004a8bedaade47a09456c2733e21a9668c81cb46f0d5e94b and compiled
source digest32869df388d83d6696c91a5fdb09d31f5617a2e8c539cfa9975a65679b13d48d
remained unchanged. Policyc1d6be31dbf61da308fd50c0297358d0cda6c952806f769e770cdd2b23c1dfc6
binds the same P6144/Adam/tokenizer/LR3e-5/SIDE-family4 with scene coverage8→32.
Same-source P16 PASS1 (15.00s), sixteen outputs matched,160 generated tokens.

Native pool physical hashes differ because `converted_at` is a creation time.
The initial over-strict physical-equality check failed before learning; a local
reader compile error and an oversized whole-pool R3BIN comparison failure also
remain preserved. Final rowwise native verification passed: all32768 train and
512 validation episodes, manifest/content/provenance fields are identical to
DIVERSE. Ten other input files and the paired-sample table are byte-identical;
all19200 other-task draws in the full3840 tape are unchanged. First1280 uses
256 selector sides per C/D/E five times each. No failed check is counted PASS.
COVER4 pool physical860512b523eae4c47d74a1bf50bc03f9950f617736af59bf04073982a13e1ba6.

| Same step7424 | DIVERSE control | COVER4 | Paired gain / loss |
|---|---:|---:|---:|
| train64 |59|47|0 /12|
| primary512 |399|375|24 /48|
| transfer128 |70|67|4 /7|
| flipped192 |4|15|13 /2|
| both192 |0|2|2 /0|

Primary buckets54/56/32/36/10/63/60/64 of64;
transfer4/4/8/4/1/16/16/14 of16. Original selector78/192;
both/original-only/flip-only/neither2/76/13/101, normal same-output155/192.
Both C0/D2/E0 spans one independent base, complete-base count0.
At6400, train50/primary347/transfer67/flipped10/both0; screens32/64/128/768
are44/46/47/47 of64, flips0/24 each. Final required panels are complete with
normal EOS and generation errors0. Admission=false, development_joint_pass=false.

Three native segments performed1+666+613=1280 optimizer calls. The middle
segment ended with pure TIME_BUDGET and a durable step6811; the next process
continued its exact cursor/Adam. Last durable absolute7424 is
`artifacts/four-view-coverage-20260920/COVER4/segment-0002/final`, physical
a55895a2e833c1d90196e654c58bbb92a04f10575e2f705c9c0cb2423c9c9151,
weightsa0f3a93970b498d1c7e6c862dc2dd2add2e6e705f44470c7dcb050befc7e27cd.
Final phase Finished, BUDGET_REACHED, resume=false. No new retry/extension.
Actual input2190995/target157238 includes discarded1743/138;
committed2189252/157100, padding785245. SMALL generation2160/teacher2144 includes
entry16 parity; TINY36/129/129 is separate, scalar/C++0. Registered active usage
1683.29954275s; maximum sampled RSS1535056KiB, not an OS high-water measurement.
No unknown calls, cancellation, numeric or save errors in learning.

Same frozen CLI `fresh paired-report --root artifacts/four-view-coverage-20260920`
was read-only rechecked, exit0, no model calls; output bytes match the first
report. Rust raw comparator redecodes EOS/tokens and verifies frozen inputs and
same-step paired scores, exit0. Logs under
`artifacts/four-view-coverage-20260920-evidence/` include all preparation failures,
`inputs-rows-verified.log`, segment0000–0002, `compare6400.log`, `compare7424.log`,
and `paired-report-verified.log`. Report SHA256
1246d1ba267725084337ae38bdf2ca730ecc469328c874dc0222bd48537ec16b;
final comparison64e0becc99de812c5960f08acb7c842d36b84d5e5c48d7a461e7fb85fe904a8e;
input audit4e73192eee7e92599f3cb660133d930dd4376c22342f8d32f27deca1aec745d0.

CODE_VERDICT=PASS for the scoped direct regressions; LEARNING_EXECUTION=COMPLETE;
MODEL_QUALITY_RECOVERED=false; DEVELOPMENT_JOINT_PASS=false;
FINAL200=NOT_CREATED/NOT_OPENED; S4/S5/S6 prerequisites unmet;
GOAL1_READY=false; GOAL1_ACCEPTED=false. New tracked files0. Do not interpret
fewer repeats per scene and worse aggregate scores as proof of an attention,
optimizer or binary-storage defect. Next diagnosis must separate those claims
before any new one-variable learning registration.

## 2026-09-20 COVER4 scene-coverage preparation verified

Continue0d503d88e45280c21fd53ec176048f4f74120103 under the user's single-variable
quality authorization. DIVERSE's trained full178/192 versus development both0/192
motivates widening recurring scenes8→32 per C/D/E at the same four value views.
Existing `--diverse-pair-values --cover-value-pairs` combination registers the
explicit COVER4 policy. Old single-flag policies remain separate. Native data
format, tokenizer, model, loss, LR, Adam and other-five-task tape stay fixed.
Study cap1280 SMALL updates; no closed DIVERSE budget or terminal is changed.

Actual maximum3840-row tape writer/readback/exposure regression PASS1 (2.15s),
model calls0. It verifies256 sides per C/D/E×5 exposures in first1280 updates,
32 independent scenes, four views, paired gradients and unchanged anchor slots.
Actual TINY process regression PASS1 (18.06s): continuous16 versus fresh1+15,
identical native weights/Adam/step/counters; actual native-pool tamper rejected
before command entry; incompatible intervention rejected. The fixture exercises
all four views and uses the existing TINY EOS fixture, not quality evidence.
Counts36 optimizer,129 generation,129 own-teacher (train controls102 each plus
separate setup observations16+11). Scalar0. No failed test or unknown use.
Release production build PASS (23.00s), locked/offline/Accelerate.
Production executable d3d38d10f6cdeda4004a8bedaade47a09456c2733e21a9668c81cb46f0d5e94b;
production-feature test executable615f743d64c64738b1585954b39954e27438d17711a69c8bd8cc0cdc52f0eabb.
Evidence `artifacts/four-view-coverage-20260920-evidence/` tape/process/build logs.
New SMALL parity/registration/learning NOT_RUN at this preparation entry.
No whole-suite/Clippy PASS claimed. No new tracked source file or dependency.

## 2026-09-20 actual DIVERSE training-view fitting verified

EXECUTED_THIS_RUN / DERIVED_EXISTING_RAW; independent acceptance not claimed.
Corrected sourceee39f92843732de1be8e131ec0dedad747fae985 was pushed and matched
origin/main before calls. Retained executable
df13be5a0b0974e3669ae6c4c21337cd9cb4264650aaffa8f283a6673421b8e5.
Three separate process tests each executed1/PASS1 (13.53/13.61/13.63s).
No training, new model or original rewrite. The failed pre-call preparation and
its source2642078/executable remain preserved, never counted as a passing run.

| Actual trained value view | Whole answer /48 | Both sides /24 | New generation/teacher | Reused |
|---|---:|---:|---:|---:|
| Original0, prior evidence |44|21|0/0|48|
| Exchanged1 |42|18|47/47|1|
| First record changed2 |45|21|48/48|0|
| Second record changed3 |47|23|48/48|0|

Total trained-scene full178/192, both83/96. Each selected input/target matches
the actual frozen tape and was exposed20 times. New view1 C/D/E12/16/14,
view2 15/16/14, view3 16/16/15 (denominator16 each). Correct citations46/47/48
of48. Seven of the ten new-view errors copy the opposite supplied record's body;
the remaining three differ otherwise. No output normalization or repair.
All143 new raw rows have normal completion/EOS/strict UTF-8, generation errors0.
The separate Rust reader rechecked selected/pending inputs, reused provenance,
prompt/target/tape/model hashes, per-call prepared/returned receipts, raw decode
and exact/both totals. It performed no model calls.

Actual SMALL optimizer0, generation143, own-model teacher143, generated tokens
2079, teacher target tokens including EOS2079. RunControl active20.6335635s;
TINY/scalar/C++ calls0. Command/error/UNKNOWN conditions absent on these three
observations. Pure mapping tests before/after correction each PASS1, no calls.
No automatic retry after model entry and no budget extension.

Native physical remains
60c772b9ff71ae20438c4e33f535d53e73b1d4ef885ef3d041da704f6ebb5416,
modela0c36fc9aac19696d245eca60dfb130de290bb798eb25948f65d1e501c6c7a1c.
Evidence `artifacts/diverse-fit-20260920-evidence/`: original failure log,
mapping logs, frozen executables, `view1-verified`/`view2-verified`/`view3-verified`
binary cases/raw/calls/receipts and `recount.rs`/`recount.log`.
Recount log SHA256d676c245e2137e367646c2ba6f54a2466ac8f43b5930ac8760491d47abe23607.
Raw view hashes62bb824655d6daf80c97654520ae16bb304c23ecc258f2a6423c8beb0abe8402,
48a1877ad8ff4b0c4f218d834e56981696046c4b276e072e81e30a616e705d97,
f85f9de3ded1fec1e6f393f5a57ddad06f18b68d7caccf6276d2ca1372435224.

Fitting remains imperfect but substantially exceeds unseen scene selection;
this evidence does not establish a unique optimizer/tokenizer/storage cause.
Next testable variable is independent scene coverage at fixed four value views.
No new learning is registered here. Development stays399/512 primary,
70/128 transfer, both0/192. CODE_DIAGNOSTIC=PASS; MODEL_QUALITY_RECOVERED=false;
FINAL200=NOT_CREATED/NOT_OPENED; S4/S5/S6 prerequisites unmet;
GOAL1_READY=false; GOAL1_ACCEPTED=false. No new tracked files/dependencies.

## 2026-09-20 fixed DIVERSE actual-training-view diagnosis registered

Initial source26420784c80152c2ad0f70dfffc47b9ab931eb13 matched remote/main.
View1 test failed before model calls/output creation on an over-strict physical
file equality check. Evaluation and final files have different resume metadata;
the existing audit_panel verifies the evaluated physical/state while terminal
history verifies final. Corrected code additionally compares actual model hash
and step, retaining both physical bindings. Original failed log/executable remain.
Corrected pure mapping test PASS1 in0.28s, calls0. Corrected executable
df13be5a0b0974e3669ae6c4c21337cd9cb4264650aaffa8f283a6673421b8e5.
Only the explicitly registered preparation correction uses new output paths;
the total143 generation/143 teacher/optimizer0 cap is unchanged.

DERIVED_EXISTING_RAW: the bound first1280 tape exposes48 selected sides per
value view exactly20 times. Original48 already covers view0 (44 correct).
Verified endpoint train64 overlaps two view0 and one view1 inputs; views2/3
overlap none. New observation caps are47+48+48=143 generation and143 own-model
teacher calls, optimizer0. The one view1 observation is reused with its original
row/hash, not regenerated. Native remains DIVERSE7424 physical
60c772b9ff71ae20438c4e33f535d53e73b1d4ef885ef3d041da704f6ebb5416.

The existing ignored train diagnostic now reads the actual policy-bound native
pool, requires executed tape membership and records pending versus full48-case
selection separately. Only diagnostic/test code changes; production training,
generation, tokenizer, objective and original evidence remain unchanged.
Pure mapping/native test executed1/PASS1 in0.29s, no optimizer/generation/teacher.
Retained diagnostic executable SHA256
4da6e9ef5951917b590a6264a29cad9cafa64fa209dcaa0fe14409a058b91d28.
Strict Clippy exited101 on existing warnings (including unrelated test targets);
it executed no tests and is not counted as PASS. No unrelated lint cleanup.
Evidence: `artifacts/diverse-fit-20260920-evidence/` coverage/mapping/clippy logs.
New model observations NOT_RUN at registration; this does not change quality
gates, reopen learning or grant Goal1 acceptance.

## 2026-09-20 DIVERSE1280 closed: limited changes, joint quality not recovered

EXECUTED_THIS_RUN / DERIVED_EXISTING_RAW; independent acceptance not claimed.
Learning source d6330f78806bba2a30664678075c148ca7ce103f was normally pushed and
matched origin/main before registration. Frozen production binary
82d5645523d549f07abb4f22828d773fec889eff45e85f8cb25cbba84f922247 and source digest
0ef9e427bf1f3ec480a26609292db8edd443d079ea9c46bf352129a45e243796 remained unchanged
through learning and diagnostics. Policy
de27cc90dbc79e3a99fcd8e4e8e8dc695d15307b86995073a49f7a05e3a11446 binds P6144,
same Adam/tokenizer/LR3e-5/SIDE family4 and the separate four-value-view pool.
Native pool physicald51db542a8b62beac4dcc182763ecea565d3248f35c93db3c6de0bc791022c88.

Pre-call actual input comparison verified nine original files byte-identical to
VALUE (including initial native), same config/evaluation/fork state, all19200
anchor slots over the3840-row tape unchanged,11520 selector draws with fixed
base/phrase/side. New native pool changes6144 table rows; first1280 uses64 sides
per C/D/E bucket twenty times each. Prior recombination48 prompts overlap0 with
the new train pool. No dev/final input is used to generate training donors.

| Same saved endpoint observation | VALUE7424 control | DIVERSE7424 |
|---|---:|---:|
| Train64 whole answers |57|59|
| Primary512 |393|399|
| Transfer128 |72|70|
| Flipped192 |2|4|
| Original and flipped both192 |0|0|
| Generation errors in final required panels |0|0|
| Seen original48 / both24 |48 /24|44 /21|
| Fixed recombined48 / both24 |22 /3|23 /4|

Current final primary buckets[56,61,36,39,16,63,64,64], transfer
[4,8,5,4,1,16,16,16]. Original selector91/192, same normal output158/192;
paired outcomes[both0,original-only91,flipped-only4,neither97], C/D/E both0 each.
Strict raw decode/expected/EOS recount gives control→new gains/losses: primary
21/15, transfer0/2, flipped2/0, train2/0, both0/0. This is not quality recovery.
At256: train53, primary406, transfer74, flipped5, both0. Screens32/64/128/768
were47/47/46/48 of64, each flip0/24, errors0. All2144 required generated rows
across scheduled panels completed with EOS and no generation errors.

Actual SMALL optimizer1280=1+755+524,6144→7424; input2190136/target156256,
including time-discarded1684/116. Committed2188452/156140, padding788528.
First native6145 saved and restored by a new process. Segment0001 reached6900,
pure TIME_BUDGET at900s, saved with resume=true; segment0002 restored it and
completed7424. Final Finished/NO_FURTHER_PROGRESS/resume=false, underlying
trainer BUDGET_REACHED; no UNKNOWN, cancel, numerical or save error. Observed
peak sampled RSS1557728KiB. No automatic budget extension or extra optimizer.

Same frozen diagnostic executable performed original48 and recombined48 in
separate fresh processes (one test each,12.46s/13.34s). Native physical/model
identity is the final7424 for both. Original raw692 tokens, recombined670,
EOS/error48/0 each; own teacher target696 each. Recombined cases/metadata and
input digests equal the prior VALUE diagnostic exactly. First-body classes:
exact23, other supplied record12, old selected3, other10. Correct citation43/48;
teacher/greedy first-token mismatch0, first gold28/48 including5 later errors.
These are fixed developmental diagnostics, not heldout acceptance. The initial
read-only recount incorrectly required a nonexistent raw `case` field; its failure
is retained. Corrected reader verifies the actual prepared-case digest, complete
case artifact equality, raw decode, EOS and call receipts without new calls.

Total SMALL1280 optimizer/2256 generation/2240 teacher includes parent16 and
diagnostic96. Existing usage ledger1597.465755960s plus diagnostic control
7.041869459+6.874799333s =1611.382424752s active. Compilation/test wrapper times
are separate. TINY56 optimizer/462 generation/462 teacher, scalar0/C++0; no new
TINY calls after source publication. All registered limits respected.

Final native: `artifacts/value-diversity-20260920/DIVERSE/segment-0002/final`,
physical60c772b9ff71ae20438c4e33f535d53e73b1d4ef885ef3d041da704f6ebb5416,
modela0c36fc9aac19696d245eca60dfb130de290bb798eb25948f65d1e501c6c7a1c.
Terminaldfe50a490a97123b28dd744f0be8550dd8d1fc785f2c734237a272d073575e93;
paired receipt2a6323423947838083dabd0dee64631853fcff407a4ce1dec7e7e00e0c2eef6e.
Evidence `artifacts/value-diversity-20260920-evidence/`: prepare, input comparison,
segment0000–0002, paired-report, compare, original48/recombined48 and verified
diagnostic recount logs, raw/call records and retained executables. The existing
pure report reverified native/state, panels, objective/LR/tape, usage and guards.
Report log SHA256e26bd5e427b007e51bcf67c0b5d3b9b0ebd05d915d2895972d54fc389657aaeb;
comparison de99b5e12d80227d0f4de2531ef22a17924f2c0d05eb8347189e05e59312f6e4;
diagnostic recount069843500b35eb7f872be396c1704d7a48581e96c7772ffa7a2582840631420a.

CODE_DIRECT_CHECKS=PASS; LEARNING_EXECUTION=COMPLETE; DEVELOPMENT_JOINT_PASS=false;
MODEL_QUALITY_RECOVERED=false; FINAL200=NOT_CREATED/NOT_OPENED;
S4/S5/S6=NOT_RUN_PRECONDITION_FAILED; GOAL1_READY=false; GOAL1_ACCEPTED=false.
No operating model replacement, original rewrite, new tracked file or dependency.
Full strict Clippy remains blocked by the two documented pre-existing warnings.
This docs-only closure is separate from the reviewed/executed source above.

## 2026-09-20 bounded value-diversity preparation

Continue source26ada8c9c8ca9c6a9cacd2d756a58d8d4286eeac under the user's
one-variable improvement authorization. The closed recombination diagnostic found
VALUE7424 full22/48 and both3/24 on new value combinations despite original48/48.
This motivates DIVERSE from the intact P6144, compared with retained VALUE1280.
It changes only train value associations: four views instead of two, same eight
scenes per C/D/E, same selector/phrase/anchor rows, parent/Adam/LR/objective/model.
The existing Plan binds a separate native training pool; actual trainer and token
audit consume it. Original evaluation files remain separate. The merged pool has
unique container IDs while preserving model request IDs and framed tokens, and
retains an unchanged validation copy required by the existing corpus schema.

Direct final process regression PASS1 (16.08s): TINY continuous8 versus fresh1+7,
exact native weights/Adam/step/counters, altered-pool rejection before command
entry, objective binding and mutually exclusive flag rejection. The unchanged
VALUE process regression PASS1 (13.68s), continuous4 versus1+3. Pure native
input/label/anchor roundtrip PASS1 (0.28s), full3840 tape writer-reader/exposure
test PASS1 (2.08s), no optimizer/generation/teacher calls in those pure fixtures.
The final process source only folds the equivalent pool-hash condition into a
let-chain after Clippy identified a new style warning.

Failed preparation evidence is retained: initial pure test compile error, empty
native validation rejection, synthetic parent tape mismatch, and duplicate IDs
when merging original/phrase containers. No model result was repaired. The failed
process setup used TINY4 optimizer/75 generation/75 teacher. Successful DIVERSE
was executed twice (20/129/129 each, including setup), unchanged VALUE once
(12/129/129). Total TINY56 optimizer/462 generation/462 own-model teacher; scalar0,
SMALL optimizer0 at this preparation stage. No C++ calls or external models.

Local evidence: `artifacts/value-diversity-20260920-evidence/`, including
`process.log`, `process-verified.log`, `process-final-source.log`,
`control-process.log`, `native-values-container-final.log`, `full-tape-final.log`
and retained TINY executables. No new tracked file. Production release build PASS
(20.11s), Rust1.98.1 locked/offline Accelerate. Same-source P6144 normal parity
PASS16/16 (one test14.47s,160 raw output tokens, teacher0/optimizer0), preserving
original files. SMALL generation16 is counted against the new study allowance.
Strict Clippy still fails only the pre-existing map-key iteration and return-type
complexity warnings; the new warning is fixed. This is not an overall lint PASS.

Frozen production executable `artifacts/value-diversity-20260920-executable`
SHA25682d5645523d549f07abb4f22828d773fec889eff45e85f8cb25cbba84f922247;
diagnostic binarya5e7d7dddc4a036440025e6039576cb7469fe1824f2d249ef466eaa83ff3d0e7;
compiled source digest0ef9e427bf1f3ec480a26609292db8edd443d079ea9c46bf352129a45e243796.
P native physicalc4d6989fbcaad60516053dda206caf43abc08c4b388c3f765e4c446864f60e20
is unchanged. Registration/SMALL learning follow source publication; they have not
run at this code-verification closure. Joint gate/S4/S5/S6/Goal1 remain unmet.

## 2026-09-20 Fixed VALUE recombination diagnosis closed: values and citations separate

EXECUTED_THIS_RUN with source10ebba3ed3ef1a1bd1b14399b8158a1a16a80e8e,
compiled digestec6424f43b5c6a13cd497aba2741c1f681a3a856d66fc4ad13c290f055ad89ad.
Frozen production-feature test executable
bd03d74718e31462cda67725fcba30ed66c348c7c93fb4199f3adc7f8fadaa0a.
Only the preserved closed VALUE7424 model was loaded, never resumed for learning.
Its native physical09b0f972cebf37cd0ea1006a1a606176282c4a79d722374aa305ca9949a1e1b1
and model47041bd2a94075abc5e214fa110b91bb9eeaa76ae46dedf148371d2c83892a8e
remain unchanged. No corpus, Adam, original receipt or product pointer changed.

| Actual observation | Full48 | Both24 | Citation48 | EOS48 | Errors |
|---|---:|---:|---:|---:|---:|
| Original train cases, new process |48|24|48|48|0|
| Same selectors, disjoint owned-train value combinations |22|3|45|48|0|

The original raw token/text/finish/prompt/expected sequence matches retained
raw48/48 exactly. Recombined full C6/D6/E10 of16 each, same-output0/24. All48
changed prompts are absent from the combined owned train sources. Donor values
already occur in training; only their binding to these fixed scenes is new.
This is a derived train-scene diagnostic, not an independent heldout score.

First body-error classification: exact22, supplied other-record value16, old
other-record value2, old selected value1, other output7. Only1 output repeats
the complete old selected answer. Correct citation45/48 does not establish that
the model correctly binds that record to its value. These cases expose a failure
to transfer value/record correspondence, beyond simply repeating old answers.
Teacher and greedy first-token argmax agree48/48; first token gold30/48, including
8 later full-answer failures. This does not prove an optimizer, tokenizer or
architecture root cause and does not change development/S4 scoring.

SMALL optimizer0, generation96, own-model teacher96; TINY/scalar/C++ calls0.
All calls RETURNED, errors/cancellation/UNKNOWN0. Generated output tokens
696+699=1395 including EOS; teacher targets696+696=1392. Observation controls
6.962069791s and6.999441083s; whole exact tests12.37s/13.08s include loading and
input validation. Both tests execute exactly1 and PASS; no calls repeated.
Registered generation and teacher budgets are exhausted, not renewed.

Pure fixture192 cases and the previous value-swap fixture pass. Final strict
Clippy (including test targets) is not PASS: existing map/type/chunk-style and
journal test import warnings remain. One new constant-chunk loop warning was
fixed afterward by equivalent `as_chunks`; the final-source pure fixture was
rebuilt and rerun successfully with0 model calls. Actual observations above
remain bound to the retained executable/source, not relabelled as a later build.
The initial0-test listing is retained as NOT_EXECUTED, not a test PASS.

The reused local Rust recount independently verifies strict token decode/EOS,
derived cases hash/labels, actual checkpoint, panel/raw and96 generation/teacher
resolution bindings. Exit0 with scores above. Two earlier read-only recount
attempts failed: one looked for case identity in raw instead of its prepared
receipt; another wrongly assumed every actual answer had a citation. Both logs
remain; the reader now follows the actual schema and classifies missing-citation
text without altering strict full correctness. These caused no model calls or
changes to original observations. No malformed output was normalized to pass.

Evidence `artifacts/value-combination-20260920-evidence/`: candidate.diff,
build-list/exact-list, value-fixture/old-value-fixture/final-source-fixture logs,
original48 and recombined48 raw/teacher/call/start/finished receipts, parity.log,
recount.log and recount-verified.log (failed reader attempts), recount-final.log,
recount-with-usage.log and the retained executable. Private originals stay local.
Original raw1d27b881a7ec207bba728094aaa40341a9522b5e3ef2f00a4098a379115cc53d;
recombined rawa7bd80339e762a2fe764e2c75c17ca5f396b0332fb74cf91020c6ac442a02a89.
Derived cases0266411ae2bce6c801984ca207c8a05d6ab5c8137705953d3c1f796bd78da44a;
final observation receipt23ab322a62a875c66b6f80884d699403ec03304d276e8e3bc9e0762f58b3fe6b.

DIAGNOSTIC_EXECUTION=COMPLETE; ORIGINAL_OUTPUT_PARITY=48/48;
VALUE_COMBINATION_TRANSFER=BELOW_FITTING_GATE; NEW_SMALL_UPDATES=0;
DEVELOPMENT_JOINT_PASS=false; S4/S5/S6=NOT_RUN_PRECONDITION_FAILED;
FINAL200=NOT_CREATED/NOT_OPENED; GOAL1_READY=false; GOAL1_ACCEPTED=false.
Next single-variable hypothesis: increase independent value associations per
fixed training scene, keeping selector sides, question exposure, anchor draws,
parent/Adam/LR/objective/tokenizer fixed. This requires a separately bounded
registration and untouched diagnostic/final panels; no learning run starts here.

## 2026-09-20 New value-combination diagnostic prepared; observations not yet run

Extend only the existing test-scoped `paired_seen_train_diagnostic`; no product
training/inference, math, data source or storage changes. Recombine values from
verified owned train donors in deterministic input order, preserving numeric
width/type and all selector fields. Reject old-pair value overlap, invalid pairs
and any changed prompt already in the owned train sources. Request-only label
validation remains outside generation. New inputs/metadata are recorded locally.

Pure fixture verifies192 cases and malformed boundaries,1 PASS0.01s. Existing
value-swap fixture also1 PASS0.01s; no model calls or optimizer updates. Locked/offline
release test build PASS14.18s. Initial unqualified exact-list selected0 tests,
recorded as NOT_EXECUTED; corrected fully qualified list confirms exactly1 test.
The unchanged VALUE7424 physical file hash was directly verified. Only its new
original48 parity and recombined48 observation are registered, SMALL optimizer0,
generation/teacher max96 each, TINY/scalar0; no new learning is registered.

## 2026-09-20 Selector-wording exposure closed: limited gains, joint quality fails

EXECUTED_THIS_RUN. Frozen source6f28e024a370ea4b809c5f2e648a8ff669e13cce,
compiled digest879d697e8364ccd54a8cd24fbdc7961c226376ac555d1f9ccaa9bd8edc8105a3.
The registered schema17 research completed2560 SMALL updates from COVER7424
to9984. Only1920/20,480 training draws changed question wording; all target
digests, facts, evidence/order/time, parent weights/Adam, LR3e-5, objective,
tokenizer and other-five-task draws match the retained fixed-exposure control.
Neither old failed studies nor this study's limits were extended.

| Same-model endpoint | New updates | Train64 | Primary512 | Transfer128 | Flipped192 | Both192 |
|---|---:|---:|---:|---:|---:|---:|
| Retained COVER7424 parent |0|49|392|68|15|3|
| Wording8448 |1024|45|372|70|30|10|
| Wording9472 |2048|46|378|63|32|13|
| Wording9984 |2560|50|386|67|33|12|
| Retained fixed-exposure9984 |2560|46|376|64|20|9|

Final primary buckets[56,62,38,30,19,64,53,64], transfer[4,7,4,1,7,13,15,16].
Both C5/D7/E0,6 independent bases,1 complete four-view base. Original selection
87/192, same-output100/192, pair counts[12,75,21,84]. The retained control has
original76, same-output114 and both9. Strict same-case paired gain/loss:
train5/1, primary35/25, transfer8/5, flipped17/4, both5/2. These are one retained
control comparison, not replicated statistical evidence or product admission.
Against this study's own parent, primary392→386 and transfer68→67 still fall.
Other-five-task primary297→299/320; original C/D/E95→87/192.

The reused raw-strata diagnostic verifies complete raw/native/call/teacher
bindings with zero new model calls. Wording view3 flipped0→9/48 and both0→3/48
versus the matched control. Final view0/1/2/3 both2/5/2/3, nonnumeric11/96 and
numeric1/96. Final flipped first-error counts: exact33, other-record whole52,
abstention41, selected citation/wrong body31, selected body/wrong citation13,
other format/content22. Time selection still has both0/64. Wording exposure
has a limited observed effect; it does not explain all failures, establish a
tokenizer/architecture defect or recover the joint task. View2 still duplicates
view0 in24/48 scenes; frozen data and denominators were not changed.

Actual usage:2560 SMALL optimizer;4,365,578 input/314,416 target tokens including
3,442/216 discarded at deadlines. Committed4,362,136/314,200, padding1,569,398.
20,480 draws,2560 per task. Generation3160=3128 scheduled panels+32 entry parity;
own-model teacher3128. The final study report records3144 generations because
the earlier source-parity16 occurred before final registration and is added here.
Separate TINY92 optimizer/495 generation/495 teacher include failed setup work;
scalar0, new C++ calls0. Registered study elapsed2869.3186735839995s includes
its preparation/parity; preceding source-parity control4.010657s is additional.
Compilation and pure recount times are separate. Peak sampled trainer RSS
1,506,560KiB, not an OS peak or S6 result.

Five training/evaluation commands exit0: optimizer counts1+930+843+786+0,
durable steps7425/8355/9198/9984/9984. Segment0003 reaches the optimizer cap,
saves9984 and stops only TIME_BUDGET with evaluation pending. Fresh segment0004
completes518 generations/519 teachers with no optimizer calls, preserving the
already returned generation whose teacher was pending. Final checkpoint bytes
equal segment0003: evaluation-only continuation did not modify weights/Adam.
All3128 scheduled outputs complete with EOS and generation errors0. Final
terminal BUDGET_REACHED/Finished/resume=false; joint=false, admissible=false.
Cancellation, UNKNOWN, nonfinite, save error and mixed error conditions0.

Pure `fresh paired-report` exit0 rechecks all native/terminal/dataset/score/
decision/exposure/counter bindings. Log SHA256
d53c5f6361e1c275e75fcca4947a9a32ddf593b14b0b84bd5e35982d80c42021.
Existing exact raw-strata test1 PASS8.15s, optimizer/generation/teacher0.
Its typed output SHA256c0a8424232f7926b83df2ff47e3c7fca9f384d668c4ce41781e025cc99298ebd.
A local Rust reader reuses the native codec/tokenizer to verify same endpoint,
dataset, IDs, expected, evidence and prompt digests, independently decode raw
tokens/EOS and calculate the paired gains/losses above. Exit0; no model calls.
Comparison log7599e9fef9486f68b4dd642dddfc1b4993d66104b8de3b5d152dc0a7fdf1535e.
Direct numeric/input/process tests and their failed preparation attempts remain
documented in the preceding registration entry; they were not rerun or hidden.

Local durable native:
`artifacts/selector-phrase-20260920-final/COVER/segment-0004/final`, physical
a5a14b2113e8186a5a26510a8ae722d5e98e8f5b42808a17cf525391f706352b,
model4f2580ce93c9890f55894f1804f6fe6d9c6c1694524fb75407f3a219381d953d.
Terminal receiptc2776186189a2df69a9970dbbe61376604e7d7cd763baa5aef0f46ee6d1e75a6;
final paired receiptb16351054b961f5088e350c5bc8da852293aa0426e807ed6db41f935a2f88723.
Frozen CLI70aa6568fba1f0d30520feec0a43f38ed5376f1c298c3354b31eb157fb009bc3
and diagnostic binary9a1b8e50c731e8a781d55f5d54172cc2e68188fcb3ef634f7d17c9055d160ec2
remain unchanged. Evidence under `artifacts/selector-phrase-20260920-evidence/`
includes candidate.diff, registration/test/parity logs, segment-0000..0004.log,
paired-report.log, final-strata.r3b/log and paired-raw-comparison.log. Corpus,
checkpoints, original raw, failure logs and local Rust helpers stay local.

CODE_VERDICT=SCOPED_PASS; LEARNING_EXECUTION=COMPLETE;
WORDING_EXPOSURE_EFFECT=LIMITED_OBSERVED_GAIN; DEVELOPMENT_JOINT_PASS=false;
MODEL_QUALITY_RECOVERED=false; FINAL200=NOT_CREATED/NOT_OPENED;
S4/S5/S6=NOT_RUN_PRECONDITION_FAILED; GOAL1_READY=false; GOAL1_ACCEPTED=false.
This research is closed with optimizer budget0 remaining. No source/format/
SQLite/core changes or new tracked files in this closure. Existing strict
Clippy warnings remain as recorded; no global Clippy PASS is claimed.

## 2026-09-20 Selector-wording exposure implementation and registration verified

Reuse `fresh paired-continue --selector-phrase-exposure` for schema17, same
COVER7424 parent as the closed fixed-exposure control. Only1920/20,480 planned
draws change: C/D/E each640 substitute already-owned train view3 wording in
one of four64-update cycles. Actual registration checks192 unique replacements
against original serialized facts/answers/order/time and request-only labels.
Pure input comparison confirms all ten native input files, parent/Adam/config,
tokenizer, evaluation policy and all20,480 target digests equal the control.
No corpus creation, product oracle, new framework or model/storage equation.

Direct native TINY process tests: changed8 continuous versus fresh1+7 PASS22.25s;
unchanged COVER control PASS22.13s, exact weights/Adam/clock/tokens. Semantic
fixture96 valid cases/384 malformed cases PASS0.02s, optimizer/generation0.
Full3840 tape writer-reader/bounds/anchor/wording test PASS1.88s and final-source
repeat PASS1.90s. Parent parity16/16 PASS twice (14.43s,13.87s),162 raw tokens
each. SMALL optimizer0/teacher0/generation32. TINY actual92 optimizer and495
generation/teacher include the failed process's20/129/129 and both completed
processes36/183/183; setup observations are included, scalar0.

The failed process caught the missing updated `train_order` digest before new
wording learning. Its original log remains. An initial compile error was also
fixed before calls. Strict Clippy then caught one new divisibility-style warning;
the equivalent integer method fixed it, with final full tape/parity rechecked.
Final strict Clippy exit101 retains only the existing for_kv_map and
type_complexity warnings; no global Clippy PASS. Final release build PASS19.66s,
Rust/Cargo1.98.1, locked/offline/Accelerate/F32/thread1. No dependencies updated.

Only `artifacts/selector-phrase-20260920-final/` is registered for learning,
optimizer cap2560 total. The earlier preparation root is retained with no
optimizer calls or command segments; it used the preceding source before the
style fix and is not a second learning arm. Both parity16 calls are reported
against the total generation allowance. Closed studies/terminals are untouched.
New policy `daf68f5a2effdf757baec2cc6db48e5fe40bab6f5af6b37c36035276adc809b2`,
study physical `e1bc179dca9e95da41deb857c61d0e80b82f9c4428d9b57c4aa030167f25fc22`.
Frozen executable `artifacts/selector-phrase-20260920-final-executable`, SHA256
70aa6568fba1f0d30520feec0a43f38ed5376f1c298c3354b31eb157fb009bc3;
compiled source digest879d697e8364ccd54a8cd24fbdc7961c226376ac555d1f9ccaa9bd8edc8105a3.
Parent physical9845a43db02f197fca6026db058c503c1fa8856ffd673267904103c03b2e98e0,
weightsd8b2472422605e9305f2a25d8c39cbac555161d65d1303ca4b149c392dc60403,
Adam90c945663b2905c62af87a46a7bfcaf9aca0971480986cc1cf9b3d07c877c626.

Evidence `artifacts/selector-phrase-20260920-evidence/`: exact test logs,
retained test binaries, input-binding reader/source, initial/final preparation,
parity, Clippy and build logs. Final diagnostic binary
9a1b8e50c731e8a781d55f5d54172cc2e68188fcb3ef634f7d17c9055d160ec2;
TINY process binarya0bc3407de4a2cf3bca6325076a046f79cbdebd9a4ef12f2161614fd50b8b5ce.
SCOPED_CODE_VERIFICATION=PASS; SMALL_LEARNING=NOT_RUN_AT_THIS_ENTRY;
QUALITY_RECOVERY/GOAL1_READY/GOAL1_ACCEPTED=false; final200 NOT_CREATED/NOT_OPENED.

## 2026-09-20 Closed COVER raw selection strata, no model calls

DERIVED_EXISTING_RAW, EXECUTED_THIS_RUN: reuse the production corpus loader,
checkpoint/state audit, strict tokenizer/raw scorer, call/teacher verifier and
request-only selector mapping in one explicitly ignored diagnostic test. No
canonical score/receipt or product inference changes. Verified both full512 and
flipped192 panels at7424 and9984 against each endpoint's frozen data and model.

| Development pairs | COVER7424 original / flip / both | COVER9984 original / flip / both |
|---|---:|---:|
| All192 |95 /15 /3|76 /20 /9|
| Nonnumeric96 |53 /6 /3|52 /14 /9|
| Numeric96 |42 /9 /0|24 /6 /0|
| View0,48 |24 /3 /1|18 /8 /4|
| View1,48 |17 /5 /1|17 /7 /4|
| View2,48 |23 /6 /1|19 /5 /1|
| View3,48 |31 /1 /0|22 /0 /0|

Final both C3/D6/E0 and same-output114/192 match the existing report. First-error
classification for flipped rows: exact20, other-record whole50, abstention53,
selected citation/wrong body39, selected body/wrong citation16, other format or
content14. Parent counts15/81/34/41/14/7. These categories describe raw output;
they do not establish one underlying cause. View3 wording has no flipped success
despite the successful train subset. View2 input/order equals view0 in24/48
scenes, confirmed from owned episodes; the fixed evaluation is not rewritten.
Numeric length, value type and scene metadata are correlated in this corpus.

Two exact raw diagnostic executions PASS (6.86s,8.11s); existing independent
mapping/binary-pair fixture PASS0.05s, with malformed/order/label/null/citation
cases. All optimizer/generation/teacher calls0, including TINY/scalar.
Locked/offline release build and exact --list resolved one test; no zero-test
PASS. The first diagnostic attempt rejected an overly strict physical-file
equality in the new test: pre-evaluation and finalized checkpoint metadata
legitimately differ. The test now reuses native physical/state checks and also
compares the terminal model hash at the same step. Failed log retained; no old
raw, checkpoint, terminal or training source was changed to pass.

Local evidence `artifacts/selection-strata-20260920-evidence/`: build-list.log,
cover7424.log (failed test attempt), cover7424-verified.log, cover9984.log,
mapping-fixture.log and two typed recount records. Test binary SHA256
54c5b61e335cb5a66ea93f9d7862724c198aae80fe3067b2cda081fb1031a73d.
Record physical hashes:7424
`6fb3f4ff057464b2e35717f7fc2272df643123a117111b2d741b1d24c4eaf882`;
9984 `2096989f7721bb72c64d34181f293656fc566dcb59bae6edfb4360a29ae0bb9f`.
Original trained artifacts remain under their existing roots; no private raw
is published. Next single-variable wording-coverage hypothesis is recorded in
the active plan, not yet registered/executed. MODEL_QUALITY_RECOVERED=false;
final200 NOT_CREATED/NOT_OPENED; S4/S5/S6 unmet; GOAL1_READY/ACCEPTED=false.

## 2026-09-20 Fixed COVER exposure closed: fitting improves, development fails

EXECUTED_THIS_RUN with source336cf18d2903878d3c387e337c205f106bdd1aaf,
compiled digestfcba7275128f528b52372cef392e7d238347dac1c1f8c173640e4701114e691a.
The separately registered COVER7424 research completed2560 new native updates,
same weights/Adam lineage,9 unchanged native inputs, absolute tape1280..3840,
LR3e-5/CE/SIDE family4/tokenizer/F32. Each384 trained selector sides gains20
exposures,30 ancestral. No old terminal, budget, corpus or model was rewritten.

| Same-model endpoint | New updates | Train64 | Primary512 | Transfer128 | Flipped192 | Both192 |
|---|---:|---:|---:|---:|---:|---:|
| Retained parent COVER7424 | 0 | 49 | 392 | 68 | 15 | 3 |
| COVER8448 | 1024 | 47 | 373 | 74 | 19 | 7 |
| COVER9472 | 2048 | 46 | 367 | 74 | 31 | 5 |
| COVER9984 | 2560 | 46 | 376 | 64 | 20 | 9 |

Final primary buckets[55,64,30,29,17,63,54,64], transfer[4,7,4,1,3,16,13,16].
Both C3/D6/E0 across4 bases, base4-complete0; pair counts[9,67,11,105].
Original selection76/192 and same-output114/192 versus parent's95 and159.
Flipped body36/citation59 are auxiliary, not full-answer credit. Original C/D/E
drop95→76 accounts for19 lost primary answers, while the other five buckets
improve297→300/320. Thus the aggregate decline does not establish generalized
anchor forgetting or storage damage. It remains a selection/generalization failure.
Screens32/64/128/512/1536 score47/48/48/46/44; final derived screen48.
All3128 panel generations finish normally with EOS, errors0. Final both9 exceeds
the previous best7, so the existing plateau rule does not stop this endpoint.
Actual terminal is BUDGET_REACHED/Finished/resume=false, joint/admissible=false.

Fresh-process final original48 diagnostic improves23→44/48, both5→20/24;
value-exchanged48 improves24→45/48, both4→22/24. Both use the identical retained
first8 train pairs per C/D/E, a subset of the96 COVER pairs, not heldout. Each
has same-output1/24, EOS48/errors0. These still fail the47/48 and23/24 fitting
thresholds. First teacher token and normal generation argmax agree96/96; first
gold45/48 and46/48, correct first token followed by an error1 each. Exchanged
outputs repeat an old pre-swap answer2/48 and cite correctly47/48. The two tests
PASS13.01s/12.71s mean execution integrity, not a fitting or quality PASS.

Exposure alone substantially improves this trained subset, but does not recover
development performance. It is not proof of a unique root cause or permission
to continue this run. More repeats on the same pool are not established as a
Goal1 solution. Further hypotheses must address transfer of conditional selection
while retaining original answers; a next single intervention remains unregistered.
The closed independent Rust/LibTorch comparison is unchanged, with0 new C++ calls.

Actual usage2560 SMALL optimizer,4,382,970 input/314,521 target tokens including
5,074/321 discarded before updates at deadlines. Committed4,377,896/314,200,
padding1,565,238.20,480 draws,2560 per task. Generation3240=3128 panels+16 entry
parity+96 final diagnostics; own-model teacher3224=3128+96. Separate direct
TINY48 optimizer/312 generation/312 teacher, scalar0. Study control elapsed
3083.7953910009996s includes preparation/parity; diagnostic controls7.070986667s
and6.812254208s are additional. Model evaluation timing excludes compilation.
Segment0000 saves7425, then fresh processes confirm TIME_BUDGET saves8233,
8965,9806 and final9984; optimizer calls1+808+732+841+178. Failed/cancelled/
UNKNOWN/numeric/save conditions0. Peak sampled segment RSS1,542,304KiB,
not an OS-guaranteed peak or S6 measurement. All five training commands exit0.

Pure `fresh paired-report` exit0 rechecks native/terminal/model binding, complete
owned panels, raw scores/decisions and actual exposures/counters. Log SHA256
b3255a75fe5d25a08e5242169ff3e56d143ca8d3ec328ab7d3dd6cc76de95f59.
The reused local Rust recount readers verify strict tokenizer/EOS/full scores and
the generation/teacher resolution hashes. Only the endpoint lookup changes to
read the actual step; original and new case IDs/content/expected remain equal.
No normalization or oracle enters generation. Original final raw SHA256
9a32eb31e85f24063666f154af4fac9e225960b1e460dc57eda8294d8a2731f6;
value rawed90d0b021ae7938af010cc4298986df497811c4260fa78650bc360f40e3bff9.

Durable native `artifacts/cover-exposure-20260920/COVER/segment-0004/final`,
physicala4a4ceee379d2d8830b9c09e347337afbecb8f315ae40a1757513acc0fdafca0,
model6b3c827701aec25584bc8323105d4ed0b40567ba49381a28d5c36c34fd91c985.
Final receiptd68d81642042c032a1e9368038d651028febc8ab475e3832fd0346b1123a6191;
policy1c0d5c62eacb38f3a55efd02ed76d25e734570d94a73d80f780fe373f694a46d.
Source, frozen CLI55edf16fe2c9de460d75c6b109bef373b980efec6f0434f2fd2b05f9e8388907
and diagnostic binary4f4012e6c5707d7614e33c9c375387806721844feaac5e1cf08a118370e1fafe
remain unchanged. Retained TINY binary6d65b5ae3c07d2e48cdd21af8ecfed742e91742a013cad26778a4f06c7a166b7
matches the process log. Evidence `artifacts/cover-exposure-20260920-evidence/`
contains the candidate diff, test/segment/report logs, local recount source and
binary raw/receipts. No original data/checkpoints/private logs are published.
Scoped strict Clippy exit101 retains only the two preceding for_kv_map and
type_complexity warnings; it is not a global PASS. No new tracked file.

CODE_VERDICT=SCOPED_PASS; LEARNING_EXECUTION=COMPLETE;
TRAINED_SUBSET_FITTING=IMPROVED_BUT_BELOW_GATE; DEVELOPMENT_JOINT_PASS=false;
MODEL_QUALITY_RECOVERED=false; FINAL200=NOT_CREATED/NOT_OPENED;
S4/S5/S6=NOT_RUN_PRECONDITION_FAILED; GOAL1_READY=false; GOAL1_ACCEPTED=false.
The bounded research is closed. Independent acceptance remains external.

## 2026-09-20 Fixed COVER exposure registration verified; learning not yet run

Reuse `fresh paired-continue` for a separate schema16 research from the intact
closed COVER7424. It copies the exact native parent/Adam and all nine owned input
files, retains first_step6144 and all3840 tape rows, and executes only the remaining
absolute1280..3840 cursor. New maximum2560 updates; old terminal/resume=false and
failed studies remain unchanged. Explicit registration rejects other parent
policies and another continuation from itself. Shared parent16 verifier, prior
screen streak and full-point progress bind entry; originalP retention baseline
is copied unchanged. No math, corpus, tokenizer, framework, storage or C++ change.

Final-source direct tests: two TINY native continuous/split regressions PASS36.33s;
VALUE4 versus1+3, COVER8 versus1+7, then separate COVER continuation8 versus1+7.
Weights/Adam/clock/cursor/tokens match, default objective and unregistered parent
entries reject; old files and pure report inputs remain byte-identical. Actual
TINY48 optimizer/312 generation/312 teacher including54 setup observations,
scalar0. Full3840-row writer/reader/bounds/remaining-exposure/plateau test1 PASS1.84s,
optimizer/generation/teacher0. Initial shortened test filter selected0 tests;
retained as NOT_RUN, followed by the exact full name, not counted as an earlier PASS.
Production parent16 test1 PASS13.42s,16 matching outputs/162 tokens, teacher0,
optimizer0. Locked/offline Rust1.98.1 production build PASS19.08s, Accelerate/F32/thread1.

Actual registration and pure input audit exit0: all9 native files identical,
initial.r3m exactly the old durable final, mathematical config unchanged;
planned20 more exposures for each384 selector sides,30 ancestral. Planned
original/P phrase draws C1520/1040,D1440/1120,E1280/1280; no false phrase-balance claim.
Prior progress retains(425,79,0),(380,67,0),(392,68,3), not a reset plateau history.
Policy1c0d5c62eacb38f3a55efd02ed76d25e734570d94a73d80f780fe373f694a46d;
compiled sourcefcba7275128f528b52372cef392e7d238347dac1c1f8c173640e4701114e691a;
executable55edf16fe2c9de460d75c6b109bef373b980efec6f0434f2fd2b05f9e8388907;
study3d7415dd8ba1e228dd1816d7f32c85b3e0ac0f7cf9eb33e84753c9cb37f70a7c.
Parent Adam90c945663b2905c62af87a46a7bfcaf9aca0971480986cc1cf9b3d07c877c626.
Evidence `artifacts/cover-exposure-20260920-evidence/`; new native study
`artifacts/cover-exposure-20260920/`. No new tracked file. Implementation/registration
PASS; new SMALL optimizer0 at this stage. Learning and all quality gates remain
unverified. Final200 NOT_CREATED/NOT_OPENED; Goal1 remains unmet.

## 2026-09-20 COVER1280 closed: limited selection gain, trained scenes underfit

EXECUTED_THIS_RUN with frozen source `c0b23edf48853743d376eefdb91d46798351acde`,
compiled digest5eaab31b873506014c3d45930267ec14f65f548ba2593f80fa7f4032f3677993.
Original P6144 weights/Adam, same native inputs/tokenizer/LR3e-5/CE/SIDE family4.
Only the recurring scene prefix grows8→32 pairs per C/D/E, both value views;
384 sides get10 exposures each versus VALUE96 sides40 each. All other-five-task
slots stay identical. No new corpus, model, storage or external learned artifact.

| Same-model endpoint | New updates | Train64 | Primary512 | Transfer128 | Flipped192 | Both192 |
|---|---:|---:|---:|---:|---:|---:|
| COVER6400 | 256 | 55 | 380 | 67 | 7 | 0 |
| COVER7424 | 1280 | 49 | 392 | 68 | 15 | 3 |
| Retained VALUE7424 | 1280 | 57 | 393 | 72 | 2 | 0 |

Final primary buckets[51,62,30,45,20,64,56,64], transfer[4,4,7,4,1,16,16,16].
Selector original95/192, both C2/D1/E0 across2 independent bases, base4-complete0,
same output159/192. Pair counts[3,92,12,85]; flipped body29 and selected citation56
are auxiliary, not full-answer credit. Screens32/64/128/768:44/46/47/47; final49.
All2144 planned panel generations ended normally with EOS, errors0. Final joint
and admissible=false. The final point improves from its own256 point, so the
plateau guard did not stop it: actual terminal is BUDGET_REACHED, Finished,
resume=false. It is not a product candidate or a reopened old study.

Fresh-process existing final diagnostic ran once on each view of the same first8
train pairs per bucket, a subset of COVER's96 pairs. Original23/48 full,
both5/24, same9/24; exchanged24/48 full, both4/24, same6/24. C/D/E full counts
are9/10/4 and9/8/7 out of16 each. Errors0/EOS48 in each. The diagnostic tests
PASS11.91s/11.87s mean successful execution, while fitting thresholds FAIL.
First-token teacher and normal generation argmax agree96/96; first gold34/48
and35/48, correct first token but later wrong11 in each. Exchanged outputs retain
old pre-swap answers11/48 and the correct citation35/48. The retained VALUE
control was48/48 and both24/24 on each view; it was reread, not regenerated.

This intervention produced a small development both0→3 change, with worse
transfer and much poorer fitting of the retained train subset. It cannot establish
general selection, sufficient preservation or a unique root cause. The wider
pool's10 exposures per side versus the narrow pool's40 is a material confound
of the fixed-compute coverage tradeoff, disclosed before execution. Further
exposure at fixed coverage is a possible separately registered hypothesis,
not an automatic extension, acceptance or promise of improvement.

Actual usage1280 SMALL optimizer;2,190,883 input/157,216 target tokens including
1,631/116 discarded at the command deadline; committed2,189,252/157,100,
padding784,493.10,240 committed draws,1280 per task. Generation2256=
2144 panels+16 production parity+96 final diagnostics; own-model teacher2240.
Direct TINY32 optimizer/258 generation/258 teacher, scalar0, are separate.
First actual update saved6145 and reloaded. Segment0001 saved6924 under pure
TIME_BUDGET/TrainingPending/resume=true; new process0002 executed the remaining500.
No UNKNOWN, cancellation, numeric or save error. Source/binary/parent hashes
were preserved. Study control1524.3499557080002s including preparation/parity;
final diagnostic controls6.89300075s+6.871689959s. Segment0002 peak sampled
RSS1,425,648KiB, preceding segment1,497,616KiB; not guaranteed OS peaks or S6.

Durable native `artifacts/pair-cover-20260920/COVER/segment-0002/final`, physical
9845a43db02f197fca6026db058c503c1fa8856ffd673267904103c03b2e98e0;
modeld8b2472422605e9305f2a25d8c39cbac555161d65d1303ca4b149c392dc60403.
Final receipt1dc10022b251bcfd8f1dc2f9ed0b60b9041055f61b6d016bb289e0f4d5cbc9e6;
policy41a2bdf33be9c87fd11d8a41491c503884a27f60cb67828e24804142bfb801fb.
Frozen executable254e7894d48a0ba109da04e5e640148664527525a90a25c6735346643d34219e,
diagnostic executablec2d77bd51208675c2ba076c0a0ffd9f2620d189313c8c73521d65b488b29df7c.
Pure `fresh paired-report` exit0 audits owned inputs, actual exposure, native
lineage, complete raw panels/decisions and known usage, no model calls. Its log
SHA256a6b5d4ce5a6c287abf22884c40c7deaa570366c5f562d7f4c7d6008813c34f9c.

Pure Rust final recounts exit0 verify strict token decoding/EOS, scores, the
retained exact cases and192 generation/teacher resolution records. The local
value recount reuses the existing reader but independently scores the imperfect
original outputs instead of assuming48/48; quality failure is not integrity
failure. Original rawc45131f413fbd5133216a49983d47584941c1c2ca5d2085aa0a242b8469decc7;
exchanged raw46869d60ab12e80ea7b28996bf8f277f55366c8e8c6076b824bf0f864200d63a.
Evidence `artifacts/pair-cover-20260920-evidence/`: candidate diff, frozen
executables, direct tests, input audit, segment0000–0002 logs, `paired-report.log`,
`seen48.log`, `value48.log`, `seen-recount.log`, `value-recount.log`, and original
binary raw/receipts. No model/corpus/private raw was published; no new tracked file.
Strict Clippy retains the two pre-existing warnings and is not globally PASS.

CODE_VERDICT=SCOPED_PASS; process/final source distinction is recorded below;
LEARNING_EXECUTION=COMPLETE; TRAINED_SUBSET_FITTING=FAIL;
DEVELOPMENT_JOINT_PASS=false; MODEL_QUALITY_RECOVERED=false;
FINAL200=NOT_CREATED/NOT_OPENED; S4/S5/S6=NOT_RUN_PRECONDITION_FAILED;
GOAL1_READY=false; GOAL1_ACCEPTED=false. Independent acceptance remains external.
This registered learning/observation budget is closed without automatic extension.

## 2026-09-20 COVER implementation and input registration verified

The next separately bounded P6144 fork widens the recurring VALUE prefix8→32
train pairs per C/D/E, with both owned value views. At1280 its384 sides get10
exposures each; VALUE96 sides got40. The pool-size intervention consequently
lengthens the original/exchanged cycle16→64. All anchor update/slot rows, native
inputs, parent/Adam/tokenizer, LR3e-5, CE/SIDE family4 and quality gates remain.
No new corpus, framework, product oracle, storage or model change.

Direct full3840-row writer/reader/tape test1 PASS1.88s on final source; it checks
all anchor slots, exact32 bases/128 sides per selector bucket,10 exposures,
pair/phrase/view identity, boundary positions and replaced/duplicated tapes.
Initial test failed because its parent fixture still carried the preceding FIT
tape; corrected to the original parent before testing expanded coverage. Original
failure log retained, optimizer/generation/teacher0 for that failure.
Two actual TINY process regressions PASS28.98s: COVER continuous8 versus1+7 and
retained VALUE continuous4 versus1+3 match weights/Adam/cursor/tokens exactly.
Mixed flags and default-objective resume rejected. TINY32 optimizer/258 generation/
258 teacher, including54 setup observations; scalar0. Those process tests used
source digest a389fdac4c4129635dfded6659f3a62b13d88934c3be70c6d5a670319c387459.
Subsequent final source explicitly enforces COVER's existing8/1280 cap and includes
its P16 in usage, two control checks with unchanged sampling/numeric/resume code.
Final source tape test, production P16 and actual prepare/readback ran afterward;
no duplicate TINY training was used for those two checks.

Final production P16 test1 PASS13.48s,16 matching outputs/160 tokens,
optimizer0/teacher0; original parent unchanged. Release build PASS19.59s.
Strict Clippy exit101 retains the two prior for_kv_map/type_complexity warnings,
not an all-checks PASS. Cargo locked/offline, Rust1.98.1, Accelerate/F32/thread1.

Actual owned-input audit exit0: all10 native files match VALUE/SIDE byte-for-byte;
all19,200 planned anchor slots match,11,520 selector slots include5,760 swapped
views over the complete3840-row tape. Planned1280 phrase draws(original/P) are
C760/520,D720/560,E640/640. A prefix is not claimed as a balanced full256 block.
No optimizer has executed for COVER at this registration stage.
Policy41a2bdf33be9c87fd11d8a41491c503884a27f60cb67828e24804142bfb801fb;
compiled source5eaab31b873506014c3d45930267ec14f65f548ba2593f80fa7f4032f3677993;
frozen executable254e7894d48a0ba109da04e5e640148664527525a90a25c6735346643d34219e.
Study `artifacts/pair-cover-20260920/`, direct logs/input hashes and local Rust
audit source in `artifacts/pair-cover-20260920-evidence/`. No new tracked files.
Implementation/registration closes here; learning results must be recorded
separately. Goal1 remains unmet, final200 NOT_CREATED/NOT_OPENED.

## 2026-09-20 VALUE1280 closed: trained value swaps fit, development does not recover

EXECUTED_THIS_RUN. Frozen source `fb917e47016d26381288311ad217d8a30a1b8405`,
source digest1a2c8a9eccd7e70408cbfddc3e86dd67ab6f850fc2293df73ef4555895e405f9.
Same original P6144 weights/Adam/tokenizer, native SMALL, LR3e-5,
first-target1 CE and SIDE family4 auxiliary0.1 as the retained FIT control.
Only alternate existing view0/view1 every16 updates on its same24 C/D/E pairs.
All other-five-task slots and all10 owned native input files remain identical.
VALUE96 sides each40 exposures versus FIT48 sides each80 is an explicit
coverage/repetition tradeoff, not matched per-example exposure. Corpus edits0.

| Same-model endpoint | New updates | Train64 | Primary512 | Transfer128 | Flipped192 | Both192 |
|---|---:|---:|---:|---:|---:|---:|
| VALUE6400 | 256 | 57 | 411 | 75 | 2 | 0 |
| VALUE7424 | 1280 | 57 | 393 | 72 | 2 | 0 |
| Retained FIT7424 | 1280 | 57 | 398 | 76 | 2 | 0 |

Final primary buckets[53,61,34,41,18,61,61,64], transfer[4,8,7,4,1,16,16,16].
Selector original93/192, flipped2/192(D1/E1), both0 in every bucket,
same normal output153/192; pair counts[0,93,2,97]. Auxiliary flipped body18,
selected citation27/192 do not count as full correctness. Screens32/64/128/768
scored47/46/45/51, final screen47. All2144 planned panel generations have EOS,
errors0. Final joint=false, NO_FURTHER_PROGRESS at the1280 ceiling,
Finished/resume=false, underlying trainer BUDGET_REACHED. No candidate adoption.

Fresh-process final observations use the existing production-feature
`paired_seen_train_diagnostic`: normal48/48, both24/24; value-exchanged48/48,
both24/24; same0/24 and errors0 in each. Two executed tests PASS11.82s/11.84s,
48 generation/48 own-model teacher each, optimizer0. First-token teacher and
normal generation agree with gold96/96. Pure reused Rust recount verifies
strict tokenizer decode/EOS, exact cases/answers, checkpoint/raw hashes and
all192 generation/teacher resolution records. Exit0 for both recounts.
Value-swapped outputs repeat an old pre-swap answer0/48 and cite correctly48/48.
Raw hashes4481ab99099f29a553a8f3555a34f7245fd2a50d651e261f0c97558e934e1cd3
and2f911d592d92d469c7e478dd219da0f68db96f907035d4be4129adcab36df3f1.

The retained FIT observations were normal48/48 but value-exchanged10/48,
both0/24 with24 old answers. They were reread, not regenerated. VALUE trained
both variants, so its96 correct answers establish fitting of those supplied
variations, not heldout generalization. The development both0 and lower primary/
transfer remain the decisive failure. This narrows the observed problem to
transfer/retention despite successful seen-case optimization; a unique data,
objective or capacity cause is not proven. Closed Rust/LibTorch comparison still
has112/112 identical outputs, so switching backend alone did not fix these cases.

Actual VALUE usage:1280 SMALL optimizer;2,190,312 input/156,284 target tokens
including1,860/144 discarded at the command deadline; committed2,188,452/156,140,
padding788,344.10,240 committed draws,1280 per task. Generation2256=
2144 panels+16 production parity+96 final diagnostics; own-model teacher2240=
2144+96. Separate direct TINY12 optimizer/129 generation/129 teacher; scalar0.
Closed fixed-FIT value diagnosis48/48 generation/teacher and closed backend
comparison514 SMALL/2 TINY optimizer/224 generation/0 teacher remain separate.

First update saved6145 and restored. Segment0001 saved6922 at pure TIME_BUDGET,
TrainingPending/resume=true; new process0002 completed the remaining502 updates.
No UNKNOWN/cancel/save error. Study control1524.463255125s including preparation/
parity; final observation controls6.806514709s+6.855052209s. Segment0002 peak
sampled RSS1,449,360KiB, preceding segment1,493,056KiB; not an OS guaranteed peak
or S6 measurement. All three training commands, pure report and final tests exit0.

Durable native `artifacts/pair-value-20260920/VALUE/segment-0002/final`, physical
09b0f972cebf37cd0ea1006a1a606176282c4a79d722374aa305ca9949a1e1b1;
model47041bd2a94075abc5e214fa110b91bb9eeaa76ae46dedf148371d2c83892a8e.
Final receipt4791b36722a1b0a66ddd4b7c3491391421c5f283b3557d94078d952ce6dfb7b2;
policy0898b4bca43435b2e27c77c8eeea5ddb297f7b01a682e63469eb6d875f61a59a.
Frozen executable09c5a3ec8626ba6b85092dcc04ce354b192f01b7f3d777a42e96183ad40b490e,
diagnostic executablee8dd0fbd8e95a3faa49f3aca5a036690309b6200a2168cff2691273efef7015a.
Source files, executable and original parent hashes unchanged after execution.
`fresh paired-report` exit0 revalidates owned inputs, actual exposure, complete
raw panels, same-step decision, native lineage and usage without model calls.
Its log SHA256a2ca12266067b3332f049bc4cbb52b09009aa01014ffe13894c6d7a3ebede8e2.

Evidence `artifacts/pair-value-20260920-evidence/`: `candidate.diff`, frozen
executables, direct test logs, `segment-0000.log` through `segment-0002.log`,
`paired-report.log`, `seen48.log`, `value48.log`, `seen-recount.log`,
`value-recount.log` and their original binary rows/receipts. No new tracked file,
product model pointer, Cargo dependency, core, storage or SQLite change.
Strict Clippy remains non-PASS on the two pre-existing warnings below.

CODE_VERDICT=DIRECT_BOUNDARIES_PASS; LEARNING_EXECUTION=COMPLETE;
TRAINED_VALUE_PAIR_FITTING=PASS; DEVELOPMENT_JOINT_PASS=false;
MODEL_QUALITY_RECOVERED=false; FINAL200=NOT_CREATED/NOT_OPENED;
S4/S5/S6=NOT_RUN_PRECONDITION_FAILED; GOAL1_READY=false; GOAL1_ACCEPTED=false.
The execution/observation budget is closed. Independent acceptance is external.

## 2026-09-20 VALUE exposure implementation and registration verified

New explicit P6144 fork, same SIDE objective/LR3e-5/Adam/tokenizer and exact
anchor slots as FIT. Only alternate owned view0/view1 every16 updates on the
same C/D/E bases. Cap1280, unchanged retention/termination/joint quality gates.
Shared request validator enforces value-only changes before registration;
production generation receives no labels or resolver. Existing sampler/codec/
RunControl/report paths accept the explicit VALUE policy; no new framework.

Direct tests: full3840-row writer/reader/tape1 PASS(1.69s), value-only validator1
PASS(0.01s), actual TINY continuous4 versus fresh1+3 process1 PASS(14.34s).
TINY12 optimizer/129 generation/129 teacher including27 setup observations;
weights/Adam/cursor match, default-objective resume and mixed flags rejected.
Initial pure tape test failed on missing new-mode recurrence binding, before
any model call; fixed and passing output retained beside the original failure.
Production P16 parity1 PASS(13.34s),16 matching generations/160 raw tokens,
teacher0/optimizer0. Release build PASS18.61s. Strict Clippy101 retains only the
same two pre-existing for_kv_map/type_complexity warnings; no global PASS claim.

Actual owned input check exit0: all10 native files byte-identical to retained
FIT/SIDE;19,200 anchor slots identical,11,520 selector slots with5,760 mapped to
the same-base/value-exchanged view across the complete3840-row policy. No new
corpus. Source digest1a2c8a9eccd7e70408cbfddc3e86dd67ab6f850fc2293df73ef4555895e405f9;
frozen executable09c5a3ec8626ba6b85092dcc04ce354b192f01b7f3d777a42e96183ad40b490e.
Policy0898b4bca43435b2e27c77c8eeea5ddb297f7b01a682e63469eb6d875f61a59a.
Study `artifacts/pair-value-20260920/`, evidence `artifacts/pair-value-20260920-evidence/`.
This entry closes implementation/registration only: new SMALL optimizer0 at
registration; actual learning results are separate. Old FIT/source/failures remain
intact. GOAL1_READY=false; final200 NOT_CREATED/NOT_OPENED; S4/S5/S6 not eligible.

## 2026-09-20 Fixed FIT value-exchange diagnosis

EXECUTED_THIS_RUN: reuse the existing production evaluation path on the closed
FIT7424 weights. Same first8 pairs per C/D/E, existing owned view1 instead of
view0, same question/phrase/entity/context/citation IDs/order/status/times;
only the two supplied values exchange. Runtime prevalidation rejects any other
semantic input change or invalid target before generation. No new corpus or
model update, product oracle, decoder correction or hidden retry.

Original seen48/48/both24/24 becomes10/48/both0/24, C7/D2/E1 out of16 each.
Normal EOS48/errors0, same answer on the two questions5/24.24/48 outputs equal
the old pre-swap answer; citation remains correct38/48. Teacher/generation first
argmax agreement48/48, first gold16/48, correct first token but later wrong6.
This is direct evidence that perfect fitting did not guarantee reading changed
values in the same scenes; it motivates a controlled value-exposure intervention,
not a framework change or a claim that all quality failures share one cause.

Direct test `seen_value_swap_changes_only_values_and_rejects_other_changes`:
1 PASS,48 positive combinations spanning original/flip and phrase styles, plus
wrong label/order/time/question/missing-record/no-op rejection. Release build
14.03s, locked/offline/Accelerate/Rust1.98.1. Existing actual generation test
`paired_seen_train_diagnostic`:1 PASS,11.66s; SMALL generation48/teacher48,
optimizer0/TINY0, control6.830406916s. Pure Rust raw/token/EOS/receipt recount
exit0, no model calls. Dynamic check PASS does not mean its quality result PASS.

Source file SHA2565e78e44a0655398f0a588c466f1a7efb9af1caa12a551970df06e61b76dc13cb;
frozen diagnostic binary04c3fcb28cc61580753885eaf152f0b7ef7405a4cb65f9c044cc982ecbdef55d.
Original native physical3e1f69b0ef1b49c8a4aba1772cf87511f877a278ad68023ea091e38ec71daed1,
model9884c680639a1d77801d88601c3f210e8608147d905a1ea54092c0722801f0b8.
Evidence `artifacts/pair-value-probe-20260920-evidence/`: frozen executable,
source diff, `unit.log`, `value48.log`, `recount.log`, `value48/` raw/receipts.
Raw SHA2563608d1489526e5606e378c858570929433a79a50c3f3147870ebd9ce08facf5a.
Original fitting raw was reused read-only, not regenerated. Observation budget
closed. Quality/Goal1 remain unmet; final200 NOT_CREATED/NOT_OPENED, S4/S5/S6
NOT_RUN_PRECONDITION_FAILED. No independent acceptance is claimed.

## 2026-09-20 FIT1280 closed: seen-pair fitting succeeds, joint development fails

EXECUTED_THIS_RUN, frozen source `2cbf5b5c9dc4f7a0491d839f6c969161ef695a2e`,
compiled source digest97ddb3108ec9c8e51bbe720aaa30b46d617d4446cb887a61f7c7709b238876bf.
Same original P6144 weights/Adam/tokenizer, LR3e-5, first-target1 response CE,
SIDE family4 auxiliary0.1 and exact other-five-task slots. Only C/D/E recurrence
changes to the first16 updates:24 train pairs/48 sides,80 exposures per side at1280.
No new corpus, architecture, tokenizer, decoding policy or external learned model.

| Native endpoint | New updates | Train64 | Primary512 | Transfer128 | Flipped192 | Both192 |
|---|---:|---:|---:|---:|---:|---:|
| 6400 | 256 | 57 | 415 | 77 | 3 | 0 |
| 7424 | 1280 | 57 | 398 | 76 | 2 | 0 |

Final primary buckets[53,64,44,36,20,60,57,64], transfer[7,9,7,4,1,16,16,16].
Original100/192, both0 across all C/D/E, same normal answer164/192; pair counts
[0,100,2,90]. Flipped body11/192 and selected citation16/192 are auxiliary counts,
not whole-answer credit. All2144 planned evaluation generations ended with EOS,
errors0. Screens32/64/128/768 scored46/47/48/47; final screen44. Final same-step
joint gate=false. Decision NO_FURTHER_PROGRESS at the1280 ceiling, Finished,
resume=false; underlying trainer also reached BUDGET_REACHED. Neither old runs
nor this run are reopened, and this model is not adopted.

Existing production seen-train diagnostic1 PASS(11.74s): fresh-process normal
greedy48/48 full answers, both24/24, same0/24, EOS48/errors0. All48 first-token
teacher argmax values equal generation and gold. Pure Rust recount verifies
strict decoding, full expected equality, checkpoint/start/raw digests and all96
generation/teacher resolution records. The exact cases and answers match the
retained REPLAY observation18/48/both3/24; that old raw was reread without calls.
The registered fitting47/48 and both23/24 criteria PASS, not the heldout gate.

This demonstrates learnability of these48 seen sides under the existing model
and equations. It does not establish general record-selection competence or a
unique root cause. Together with the closed Rust/LibTorch256 comparison, the
observed failure is poor transfer of learned conditions/value/citation selection,
not evidence that replacing the numeric backend fixes quality. More recurrence
on these few cases learned them while development primary fell from parent425
to398 and both remained0. Shared data/design defects elsewhere remain unexcluded.

Additional pure endpoint check(exit0, model calls0): the real native production
6400 checkpoint matches the closed Rust diagnostic's9,513,408 weights and both
Adam moments bit-for-bit (28,540,224 values). This connects the independent C++
comparison to the actual production trainer, not only a diagnostic access probe.
Local evidence `artifacts/libtorch-parity-20260920/production-endpoint.log`.

Actual FIT usage:1280 SMALL optimizer,2,190,228 input/156,297 target tokens
including1,776/157 discarded at the command deadline; committed2,188,452/156,140,
padding788,788.10,240 draws,1280 per task. First update saved6145 and restored in
a new process; segment0001 saved6921 at pure TIME_BUDGET/TrainingPending,
then a new process performed the remaining503 updates. No UNKNOWN/cancel/save
error. Final durable file `artifacts/pair-fit-20260920/FIT/segment-0002/final`,
physical SHA2563e1f69b0ef1b49c8a4aba1772cf87511f877a278ad68023ea091e38ec71daed1,
model9884c680639a1d77801d88601c3f210e8608147d905a1ea54092c0722801f0b8.
Native policy a5cc598f8148c64bd889175984c57175bf8769f9919c1b84c57daee881e2d546.
Executable504d1a5a86c409f63536dfdbeee4709d6d906902f222b339f0e4d6afd77e625b
and source/parent hashes remained unchanged throughout actual learning.

FIT generation2208 =2144 panels+16 production parity+48 seen; own-model
teacher2192=2144+48. No additional optimizer for the fitting observation.
Study control time1528.701314667s including prepare/parity, plus6.767095791s
seen observation; diagnostic process wall11.74s includes setup/loading.
Separate direct TINY regression73/723/723 optimizer/generation/teacher and
closed Rust/LibTorch comparison514 SMALL/2 TINY optimizer/224 generation/0 teacher
are not counted as FIT quality updates. Final segment peak sampled RSS1,434,256KiB;
preceding segment1,493,248KiB, not a guaranteed OS peak or S6 measurement.

Pure `fresh paired-report` exit0 independently verifies owned inputs, native
lineage, actual exposure, full raw panels, guard decision and known usage.
Evidence `artifacts/pair-fit-20260920-evidence/`: `segment-0000.log`,
`segment-0001.log`, `segment-0002.log`, `paired-report.log`, `seen48.log`,
`seen-recount.log`, `candidate.diff`; raw stays under the arm and `seen48/`.
Seen raw SHA2569baf3f3fe7dc7168d55120f621740fd6b541d79815651536a21692929e16504e.
No unrelated full tests rerun. Direct code/process checks PASS; strict Clippy
still has the two pre-existing warnings recorded below, not an all-checks PASS.

CODE_VERDICT=DIRECT_BOUNDARIES_PASS; LEARNING_EXECUTION=COMPLETE;
SEEN_FITTING=PASS; DEVELOPMENT_JOINT_PASS=false; MODEL_QUALITY_RECOVERED=false.
FINAL200=NOT_CREATED/NOT_OPENED; S4/S5/S6=NOT_RUN_PRECONDITION_FAILED;
GOAL1_READY=false; GOAL1_ACCEPTED=false; independent acceptance remains external.
The study budget is closed; any next one-variable intervention needs its own
explicit policy/budget and must target the observed transfer gap, without
changing this failed decision or treating memorization as Goal1 acceptance.

## 2026-09-20 FIT native preparation and direct regression closure

The previously deferred implementation adds only FIT's first16 C/D/E recurrence
to existing native paired planning/binding/usage/report paths. Other five tasks
retain their exact per-update slots. SIDE family4/auxiliary0.1, P6144, Adam,
tokenizer, first-target1 response CE and LR3e-5 are unchanged. Native max1280;
training fit47/48 and both23/24 are diagnostic criteria, separate from joint gates.

Executed before the LibTorch comparison and reused unchanged: full3840-row
writer/reader/tape test1 PASS(1.58s); actual TINY continuous4 versus fresh1+3
within existing paired process regression1 PASS(85.06s), including wrong-policy
and mixed-flag rejection. TINY usage73 optimizer/723 generation/723 teacher
includes retained controls and setup; it is not new SMALL quality evidence.
Current source verification: release build PASS, production P6144 fixed parity
test1 PASS(13.26s),16 generations/160 raw tokens,0 teacher/optimizer, originals
unchanged. Strict Clippy exit101: two pre-existing for_kv_map/type_complexity
warnings, matching the preceding WIDE check; strict whole-clippy PASS is not claimed.

FIT source SHA is the commit containing this entry. Frozen production executable
SHA256504d1a5a86c409f63536dfdbeee4709d6d906902f222b339f0e4d6afd77e625b.
Evidence: `artifacts/pair-fit-20260920-evidence/`; planned native study
`artifacts/pair-fit-20260920/`, retained executable
`artifacts/pair-fit-20260920-executable`. Preparation is distinct from actual
SMALL execution; training and quality results are recorded separately below.
GOAL1_READY=false; final200 NOT_OPENED; S4/S5/S6 not yet eligible.

## 2026-09-20 Rust/LibTorch paired256 learning and112 generation comparison

EXECUTED_THIS_RUN: same P6144 native weights/Adam/tokenizer, actual256 identical
batch8 tapes, constant LR3e-5, first-target1 CE plus unchanged SIDE auxiliary0.1.
C/D/E repeat the same24 pairs/48 sides sixteen times; the other five tasks keep
their step-specific original draws. Each backend completed256 optimizer calls,
436940 input/31228 target tokens, ending at clock6400. No reset, additional
corpus, output correction, teacher or external learned artifact. Original
parent physical hash remains c4d6989fbcaad60516053dda206caf43abc08c4b388c3f765e4c446864f60e20.

| Observation | Rust/Candle | C++/LibTorch2.14.0 |
|---|---:|---:|
| Train whole answers | 38/48 | 38/48 |
| Both sides of train pair | 15/24 | 15/24 |
| Same output despite changed selector | 4/24 | 4/24 |
| Frozen development screen | 45/64 | 45/64 |
| Normal EOS / generation errors | 112 / 0 | 112 / 0 |
| Generated tokens, including EOS | 1445 | 1445 |

Raw token sequences and finishes match112/112. Normal Rust bounded-KV generation
and independent C++ full-prefix greedy both load their own saved256 endpoint in
fresh processes. All generated text uses strict UTF-8, complete answer/citation
equality and EOS; no trim or field-only promotion. Pure existing-parent recount
on these exact64 IDs/inputs/expected answers gives48/64, buckets
[7,8,4,4,5,8,4,8], versus both new endpoints [7,8,4,2,4,8,4,8]. The small fixed
screen is developmental; it does not establish full512/transfer/selector quality.

Mean training CE over first32 updates0.1937608917, last32 0.0300519866(Rust);
the two overall token-weighted means are0.0670846510 and0.0670838631. Final weight
RMS difference1.986363e-7, maximum3.236532e-5; each backend's parent-relative
weight-change L2 is about1.69034. All256 clocks/input/target counts agree and
all states are finite. Accumulated F32 drift is reported, not checked against a
retroactively widened one-step tolerance. Read-only Rust receipt checks and
C++ terminal counters confirm all attempted updates returned and were saved.

Independent request-only selection on inspected train48+devC/D/E24 agrees with
the frozen labels. Train10 errors: complete other-record answer3, correct value
with wrong citation6, wrong value with selected citation1. DevC/D/E14 errors:
seven ambiguity abstentions despite unique evidence, one malformed citation
answer, one complete other-record answer, one correct value/wrong citation,
four wrong value/selected citation. The raw classifier groups the first eight
as answer-format/citation-syntax; this breakdown is direct raw inspection.
These are output patterns, not proof of a single training/data/architecture
cause. The tested native backend does not show a material independent numeric
failure, and replacing it with LibTorch did not improve these responses.

Recorded learning-loop elapsed: Rust227.328s, C++92.694s. Generation command
observations4.784s/15.353s; C++ recomputes full prefixes and Rust uses KV.
Different instrumentation/execution paths prevent a general speed claim.
Total comparison usage SMALL514 optimizer(two numerical+256 per backend),
TINY2, generation224, teacher0. No hidden retry, further update or final200.
Earlier FIT regression usage is separate; its SMALL experiment remains deferred.

Local evidence root: `artifacts/libtorch-parity-20260920/`; detailed local
`REPORT.md`, separate `cpp/` and `rust-export/` source, `generation-grade.log`,
`trajectory.log`, `verify-usage.log`, `parent-screen64.log`, `failure-cases.log`.
Actual training executables: Rust8bdc336b1db06f458ba69f13ec88b0ed79cef97a1536292f6a3361773e8369d7;
C++270cf34ee6e003d1d0f4bc929b4f32b1ebe4ac5e77ae1f01ef2ecc0e4bf26eec.
Diagnostic endpoints (weights+Adam binary exports, not adopted .r3m):
Rust `rust-train256/state.r3x` ee5320a273d6fbcd2ae4a85e516654eb03f9059272b2b41611de90f4e78d69dc;
C++ `cpp-train256/state.r3x` 830edfd7d1129e207deda1e1f95ca117dc8acdb9102d5cc671ada429ad0fadd6.
COMPARISON_EXECUTION=PASS; NUMERICAL_PARITY=PASS_FOR_TESTED_INPUTS;
MODEL_QUALITY_RECOVERED=false; S4/S5/S6=NOT_RUN_PRECONDITION_FAILED;
GOAL1_READY=false; GOAL1_ACCEPTED=false. Independent acceptance remains external.

## 2026-09-20 Independent LibTorch one-step numerical parity

EXECUTED_THIS_RUN: isolated C++ LibTorch2.14.0 arm64 CPU/F32/thread1 reference
compared against the existing Rust forward/loss/Adam functions. Product/Cargo
dependencies unchanged. TINY optimizer1 per backend; SMALL optimizer1 per
backend from identical P6144/Adam, original artifacts read-only. Generation0,
teacher0. The disposable Rust access wrapper changes only module paths and adds
access to the probe; the copied Rust equation and optimizer bodies are unchanged.

Five pure parser/tolerance tests PASS. TINY173,701 and SMALL48,646,085 numerical
output elements all pass the limits registered before observation. SMALL's
9,513,408 gradients, updated weights and both Adam moments are included.
SMALL max absolute differences: logits1.5735626221e-5, normalized gradient
6.1839818954e-7, updated weights7.4505805969e-9. Valid argmax disagreements0/1761;
supervised0/153. Rust CE0.3667096794 versus C++0.3667092919. Each consumed1761
input/153 target tokens for the diagnostic step; neither output replaces a
native checkpoint. This is one tested batch, not proof that all paths are correct.

Local evidence: `artifacts/libtorch-parity-20260920/` contains separate source,
official library, build identities, input/output binary exports, comparison
logs and the removable-layout diagram. `small-step/input.r3x` SHA256
`7223b8d1e4ffebe4397dce94188d82a7b751a4321687427ae35e36f4bd8a9835`;
Rust output `82b6918453a85344f4440d5e7d1ca7bad7f62b0139ad3faf74de2d1986c15adc`;
C++ output `175e873c075dcae2eae5b29a09b11c2b27cd4d55e94d69e07bee81cd6d6a8a85`.
Native parent physical hash remains
`c4d6989fbcaad60516053dda206caf43abc08c4b388c3f765e4c446864f60e20`.
Paired256 learning and generation comparison are NOT_RUN at this entry.
NUMERIC_PARITY=PASS; MODEL_QUALITY_RECOVERED=false; GOAL1_READY=false.

## 2026-09-20 Seen-pair normal generation diagnosis, no updates

EXECUTED_THIS_RUN: existing production diagnostic2 PASS,13.93s(REPLAY) and
11.76s(WIDE). Each generated48 full answers and ran48 required own-model
teachers; total96/96, optimizer0/TINY0. Accounted control time6.829093458+
6.803203958s excludes setup/loading. All calls returned; normal EOS96/errors0.
Same first8 seen train pairs per C/D/E, frozen before calls; cases/expected
content identities match across endpoints and earlier margin observations.

| Closed7424 model | Exact48 | Both24 | Same output24 | First-token wrong48 | Correct first, later wrong48 |
|---|---:|---:|---:|---:|---:|
| REPLAY,5 exposures | 18 | 3 | 15 | 14 | 16 |
| WIDE,3 exposures | 19 | 1 | 16 | 15 | 14 |

First teacher argmax and actual generation agree on all96 sides. Teacher both
first-gold10/24 and9/24 exceed whole-answer both3/24 and1/24 respectively.
This is incomplete learning on the observed train cases, not only failure to
generalize. Low aggregate CE and positive counterpart margins do not establish
complete answer mastery. Three/five exposures alone do not prove a gradient
defect or that9.5M parameters cannot learn the task. No model-quality promotion.

Pure Rust recount exit0 checks strict tokenizer decoding, raw exact/EOS,
physical checkpoint and start/raw digests, all192 generation/teacher resolution
records and their row/prepared digests, pair totals and case/expected equality.
Existing training-only resolver validated labels before observation; no labels
or oracle entered model generation. Raw remains local:
`artifacts/pair-wide-20260920-evidence/replay-seen48/eval-7424-seen-train48.r3rows`,
SHA256 `6de896f08778e7319dfe945d3fb09f4ccf51206aa85f896690f2debe7d40bf02`;
`artifacts/pair-wide-20260920-evidence/wide-seen48/eval-7424-seen-train48.r3rows`,
SHA256 `cc46189164aaab515d565d427be215ffbdb8f45c6d8c0be204f5549fb0a8bc18`.
Same verified production test binary/source as WIDE; earlier receipts untouched.
This is a train-only diagnostic, not heldout/S4/Goal1 acceptance.

## 2026-09-20 Two-block recurrence completed; joint quality remains unmet

EXECUTED_THIS_RUN, source `def394b19426f49fe34074a84640bc3084dc6ad9`.
WIDE completed1280 updates P6144→7424. Pure report exit0 verifies frozen inputs,
actual tape/Adam/LR, strict raw/teacher panels, decision and terminal/usage.
No source or binary changes during learning; old checkpoints/failures preserved.

| Absolute step | New updates | Train64 | Primary512 | Transfer128 | Flipped192 | Both192 |
|---|---:|---:|---:|---:|---:|---:|
| 6400 | 256 | 47 | 367 | 69 | 6 | 0 |
| 7424 | 1280 | 47 | 375 | 69 | 32 | 5 |

Final primary buckets `[55,62,30,28,20,60,56,64]`, transfer
`[4,8,4,0,6,16,15,16]`; both C0/D5/E0 across3 bases, base4=0.
Original78/192, same normal output141/192, pair counts `[5,73,27,87]`.
Matched REPLAY369/64/25/6, original70/same126. Wider coverage improved primary/
transfer relative to REPLAY but not joint selection, and remains below P6144.
Not adopted. Screens32/64/128/768:44/46/41/43; flip24:0/0/1/1. Final screen42,
guard_streak0. All2144 evaluation generations had normal EOS/errors0.

Actual input2,197,323/target157,630; committed2,195,433/157,500; discarded
1,890/130; padding783,397.10,240 draws,1280 per task,5120 per phrase.
Generation2160 including P16, own-model teacher2144, active study1508.965294499s
including prepare. Separate TINY65/669/669. First update saved/restored6145;
900s TIME_BUDGET at6948 had TrainingPending/resume=true and counted its
uncommitted forward. New process completed476 updates and final896 generations/
teachers. Final Finished/BUDGET_REACHED/resume=false; no UNKNOWN/cancel/save error.
Trainer peak sampled RSS1,495,904KiB; separate OS ps samples reached4,638,784KiB.
These are distinct observation methods, not an S6 measurement or guaranteed peak.

DERIVED_FROZEN_TAPE: per C/D/E,512 distinct exact cases across256 bases,
256 cases twice/256 three times,3584 unused. All48 fixed diagnostic sides saw3
exposures. Input proof verifies unchanged parent/config/pools and all19,200
nonselector slots across the3840-row registered tape. First512 equals SIDE;
first256 model/raw scores also match SIDE/REPLAY. DERIVED_EXISTING_RAW:
119/192 flipped teacher first argmaxes incorrect, mean first NLL2.4457423820,
remaining0.2093856636. Teacher results are not free-generation accuracy.

Executable `artifacts/pair-wide-20260920-executable`, SHA256
`22864e624f927abfffabf545ca4bfeccd2a542e714a8a0cda46373d185d1a996`;
source digest `ef5a05b4a030dc3ce767d306bc464ce3e285558a0603f8db660e4de37c4fa38d`;
policy `6102a3a62a3308c1a9cf999918413b6c519eb8f1af6bdb6e9114af98af64bc07`.
Final native `artifacts/pair-wide-20260920/WIDE/segment-0002/final`, directly
hashed whole-file SHA256 `f224140c68c1887c6b1167c8d85947e86b45c2f810722db006c16313aab6cb76`,
model weights `17ebed6248f12362319dd3483138c194572d4d6428bd558a89d653bf95d684f0`.
Evidence/candidate.patch/commands/raw recounts under
`artifacts/pair-wide-20260920-evidence/`; corpus/weights/raw are local only.
CODE_VERDICT=IMPLEMENTER_TESTED_PASS; LEARNING_EXECUTION=COMPLETE;
MODEL_QUALITY_RECOVERED=false; FINAL200=NOT_CREATED/NOT_OPENED;
S4/S5/S6=NOT_RUN_PRECONDITION_FAILED; GOAL1_READY=false; GOAL1_ACCEPTED=false.

## 2026-09-20 Two-block recurrence — implementation verified

WIDE changes only the recurring C/D/E prefix from256 to512 updates relative to
REPLAY. Same P6144, Adam, native corpus/tokenizer, SIDE family4 auxiliary0.1,
response CE, batch8 and LR3e-5. Other five tasks retain exact update/slot identity.
At1280 each selector bucket has512 exact cases across256 bases, with256 cases
seen3 times and256 twice. Existing REPLAY failures and budgets remain closed.

EXECUTED_THIS_RUN: full3840-row writer-reader/recurrence/anchor/partial-cycle/
wrong-policy test1 PASS (1.44s, optimizer/generation/teacher0). Actual TINY
process test1 PASS (87.86s):65 optimizer,669 generations/669 teachers including
27 setup observations of each kind. Continuous8 equals fresh1+7 weights/Adam/
cursor/token counters; native family4 and default-loss resume rejection hold.
Pure report changes no files; incompatible flags and sticky failures are rejected.
Production P16 test1 PASS (13.33s),16/16 and160 raw tokens, errors0,
optimizer0/teacher0, original artifacts unchanged. This verifies implementation
and parent parity, not model quality. Evidence remains local under
`artifacts/pair-wide-20260920-evidence/`. SMALL learning is not yet executed.
Locked/offline Rust1.98.1 release build and relevant clippy exit0; the two existing
map-iteration/return-type style warnings remain. Test binaries SHA256
`5c8f4aa2dd147bf6f5c4e539ea60299317687aa2aee98e97fb6a7e73eb59fe6d`
(production tests) and `6098797dddd9daa3d08e2219f8b6aeff44c950f2ef34a0763486eb5b79fcdd97`
(TINY process suite) are recorded separately from the learning executable.

No new permanent files or storage/core changes. Source and executable will be
frozen before the bounded1280-update trial. FINAL200 remains unopened;
S4/S5/S6=NOT_RUN_PRECONDITION_FAILED; GOAL1_READY=false; GOAL1_ACCEPTED=false.

## 2026-09-20 Recurrence train-margin diagnostic verified

EXECUTED_THIS_RUN: existing teacher-only diagnostic1 PASS (12.76s),96 own-model
teacher calls, optimizer0/generation0, errors0. Control-accounted6.291378042s
excludes setup/loading. Pure Rust recount verifies97 raw rows, all96 call records,
receipt digests/completion and aggregates. Case digest and all48 P6144 teacher
records match the earlier SIDE observation exactly; no selection change.

On the same24 train pairs, both-positive margins4→9 and both>=1 margins1→5 from
SIDE to REPLAY. Mean sum0.3379638443→2.4854617119; REPLAY sum>=1 is15/24, of which
6 still have a nonpositive side. For46 sides diverging at token0, gold argmax
26→32 and both-first-gold pairs4→9; positive-margin/wrong-argmax1→0. The2 sides
with a later divergence are not inferred from token0. These are teacher facts,
not free whole-answer generation or heldout quality. Five exact exposures improved
this training discrimination but did not recover development/transfer quality.

Raw `artifacts/pair-replay-20260920-evidence/train-margin96/margins.r3rows`, SHA256
`c472eb9a8d468adb0f44c00b006064a41f237c8df93bb9888ed941b61633d6b2`;
start `ef4ee89668309b50866706bc57846cb263df3e9f08979cfcf0cc1001cf6dcf8d`.
margin96.log, margin-recount.log and margin-comparison.log preserve commands and
evidence locally. DIAGNOSTIC=VERIFIED; MODEL_QUALITY_RECOVERED=false;
S4/S5/S6=NOT_RUN_PRECONDITION_FAILED; FINAL200=NOT_OPENED; GOAL1_READY=false.
Next research may test a two-block recurring pool under the same1280-update cap,
with all other conditions fixed. This requires a new registration, not extension
or reopening of REPLAY.

## 2026-09-20 Exact-pair recurrence completed; joint gate failed

EXECUTED_THIS_RUN, source `c34f6b40d9a02591823db8657774c4d42c28260d`.
Completed1280 updates P6144→7424 under the registered REPLAY sampler. Parent,
Adam, corpus bytes, tokenizer, family4 objective/LR/config and all nonselector
update slots matched SIDE. First256 steps produced exactly the same evaluated
model hashes and scores as SIDE, confirming the unchanged prefix of the tape.

| Absolute step | New updates | Train64 | Primary512 | Transfer128 | Flipped192 | Both192 |
|---|---:|---:|---:|---:|---:|---:|
| 6400 | 256 | 47 | 367 | 69 | 6 | 0 |
| 7424 | 1280 | 48 | 369 | 64 | 25 | 6 |

Final primary buckets `[57,62,33,22,15,60,56,64]`, transfer
`[4,8,4,0,1,16,15,16]`; both C1/D5/E0 across5 bases, base4=0.
Original70/192 and same normal output126/192. Matched SIDE372/70/25/2,
original72/same168. Train43→48 and both2→6 do not offset failed development/
transfer gates. Screens32/64/128/768:44/46/41/44; flip24:0/0/1/3.
Final screen39/64, guard_streak1, not a quality stop; registered optimizer cap
exhausted. All2144 evaluation generations had EOS/errors0. REPLAY is not adopted.

Actual input2,194,593/target156,859; committed2,192,797/156,720; discarded
1,796/139, padding781,607.10,240 committed draws,1280 per task,5120 per phrase.
Generation2160 including P16, own-model teacher2144, active study1555.799169460s.
TINY49/615/615 is separate. First update saved/restored at6145. Real900s stop
at6912 occurred after complete evaluation and before the next optimizer update:
TrainingPending, TIME_BUDGET only, resume=true. Its uncommitted forward work
remains counted. Fresh process did512 updates and final896 generations/teachers.
Final Finished/BUDGET_REACHED, resume=false; no UNKNOWN/cancel/storage failure.
Peak sampled RSS1,510,112KiB is not S6. Pure paired-report exit0 verified all
native/raw/teacher/decision/usage bindings; prior failures were not changed.

DERIVED_FROZEN_TAPE, no model calls: each C/D/E bucket has256 distinct exact
samples at5 exposures,3840 unused;128 base scenes rather than SIDE's256. All48
fixed diagnostic sides were seen5 times. DERIVED_EXISTING_RAW:132/192 flipped
first-token teacher argmaxes incorrect, first mean NLL3.0166296231, remaining
mean NLL0.2324026901. Teacher scores are not free-generation quality measures.

Executable `artifacts/pair-replay-20260920-executable`, SHA256
`3dcd4baf563feb9d32a22d101100d45f79a7b6600cc5b49154525a76388ac685`;
source digest `87d56074d0cf380075a23eaa0a6794300383e097a470788782781e7d5bc7172e`;
policy `b2b967915a6f373742e606ce6d25313e42025ca4d555b8c39fb59008d20e5c06`.
Final `artifacts/pair-replay-20260920/REPLAY/segment-0002/final`, directly hashed
physical SHA256 `33748a93e708b0b497ff78d55ad2034adf548325c4307ba536004a5ce7bb114a`,
weights `ec43740107344cb2d50d615f28226d15c1e0d78b617f739b17b974d546405242`.
Commands/tests/hashes/candidate.patch/input proof/recounts are local under
`artifacts/pair-replay-20260920-evidence/`. Original raw remains unpublished.
CODE_VERDICT=IMPLEMENTER_TESTED_PASS; LEARNING_EXECUTION=COMPLETE;
MODEL_QUALITY_RECOVERED=false; FINAL200=NOT_CREATED/NOT_OPENED;
S4/S5/S6=NOT_RUN_PRECONDITION_FAILED; GOAL1_READY=false; GOAL1_ACCEPTED=false.

## 2026-09-20 Exact-pair recurrence — implementation verified

REPLAY is one new P6144 research registration. Same SIDE family4 loss, parent
weights/Adam, tokenizer, native corpus, batch8 and LR3e-5. Only C/D/E sample
recurrence changes: repeat their first256-update block. Other tasks remain
identical at every update/slot. At1280,256 exact samples per C/D/E get5 exposures
instead of1280 samples once. Reduced base/view coverage and changed token totals
are intrinsic to this sampling tradeoff; no stronger causal isolation is claimed.
No corpus expansion, core/format change or product oracle. Cap1280 remains explicit.

EXECUTED_THIS_RUN: full3840-row writer-reader/recurrence/anchor-preservation/
malformed-block test1 PASS (1.23s, optimizer/generation/teacher0). Actual TINY
process regression1 PASS (70.40s):49 optimizer calls,615 generations/615 teachers,
including27 setup observations of each kind. New recurrence uses a2-update
fixture block twice: continuous4 equals fresh1+3 weights/Adam. Native family4,
constant LR, pure reporting, mixed-intervention rejection and sticky failures
remain checked. Earlier tape run also passed; it is not added to model call counts.

Production P16 test1 PASS (13.91s):16/16 exact,160 raw tokens, errors0,
optimizer0/teacher0, originals unchanged. Locked/offline Rust1.98.1 release build
and clippy exit0; existing map-key/nested-return-type style warnings remain.
No SMALL learning at this verification stage. Local evidence:
`artifacts/pair-replay-20260920-evidence/`, test binary SHA256
`28f0d7a4d8da0d8e7d58efa3c3e1220aea586d54da81f015c657d813ce706487`,
process binary `856538db08ecfd604539388d460105710b0756bfcf0d63eda413c95757f61225`.
CODE_VERDICT=IMPLEMENTER_TESTED_PASS; QUALITY=NOT_YET_EVALUATED_FOR_REPLAY;
FINAL200=NOT_OPENED; S4/S5/S6=NOT_RUN_PRECONDITION_FAILED;
GOAL1_READY=false; GOAL1_ACCEPTED=false.

## 2026-09-20 SIDE train margins and exact exposure, no learning

EXECUTED_THIS_RUN: existing teacher-only diagnostic1 PASS (10.88s),96 own-model
teachers, generation0, optimizer0, errors0. Control-accounted6.040619416s excludes
loading/setup. Same first8 seen pairs per C/D/E as the previous CONTRAST probe.
Pure Rust verification confirms the case digest and all48 parent teacher records
are identical across the two observations. The raw/call/receipt recount passes.

| Final model | Both margins>0 /24 | Both>=1 /24 | Sum>=1 /24 | Sum>=1 but one<=0 /24 |
|---|---:|---:|---:|---:|
| CONTRAST7424 | 4 | 0 | 8 | 4 |
| SIDE7424 | 4 | 1 | 2 | 0 |

SIDE mean sum0.3379638443, minimum side-1.9402074814, maximum2.0724487305.
Both models have27 positive margins among46 sides whose first divergence is at
target token0. Of these positive margins, vocabulary argmax is wrong2 times for
CONTRAST and1 for SIDE. Both first-token gold argmax pairs4/24 in each; the2
nonzero-divergence sides are not inferred from token0. These are training-only
teacher facts, not full-answer generation scores or generalization evidence.

DERIVED_FROZEN_TAPE, model calls0: each C/D/E bucket has1280 draws from1280
distinct exact samples (each1 exposure),2816 exact samples unused, all256 base
scenes represented. The fixed48 diagnostic sides each had exactly1 exposure in
the new1280-update trial. Thus “seen train” does not mean repeatedly optimized
to convergence. The unchanged both-positive count supports testing exact-case
recurrence separately; it does not establish low exposure as the unique cause.

Evidence `artifacts/side-margin-20260920-evidence/`: margin96.log,
margin-recount.log, margin-comparison.log, exposure-recount.log and their Rust
helpers. Raw `train-margin96/margins.r3rows` SHA256
`de710010ef1cd525ea093bd5721229cc6c21ffe08e56db3010c367078a9cde4f`;
start `6eeadb8e9c9d6be8079466d86065afec9cf1316cbd91796ae76733669a6690ff`.
The bounded learning trial remains closed. DIAGNOSTIC=VERIFIED;
MODEL_QUALITY_RECOVERED=false; FINAL200=NOT_OPENED; GOAL1_READY=false.

## 2026-09-20 Separate side margins completed; joint quality failed

EXECUTED_THIS_RUN, source `4dfa81fc5a4e7c337872ef4d629976fbc75a8347`.
All1280 registered SMALL updates completed from P6144 to7424. Same initial
weights/Adam/tokenizer/corpus, exact3840-row tape and config as CONTRAST were
verified before learning. Only the paired auxiliary aggregation changed.
First-update CE0.36670968 matched the control; objective0.74849534,
gradient norm3.23801079, actual weight delta L2 0.01703960.

| Absolute step | New updates | Train64 | Primary512 | Transfer128 | Flipped192 | Both192 |
|---|---:|---:|---:|---:|---:|---:|
| 6400 | 256 | 47 | 367 | 69 | 6 | 0 |
| 7424 | 1280 | 43 | 372 | 70 | 25 | 2 |

Final primary buckets `[57,63,26,29,17,61,55,64]`, transfer
`[4,8,7,1,2,16,16,16]`; both C0/D2/E0 across2 bases, base4=0.
Original72/192 and same normal output168/192. Matched CONTRAST was366/68/39/0,
original64 and same145. Better primary/transfer and two both-correct pairs are
insufficient; flipped-only accuracy decreased. SIDE is not a quality recovery.
Screens32/64/128/768:44/46/41/47 out of64, flip24:0/0/1/1. All2144 evaluation
generations had normal EOS/errors0. No final200 was created or opened.

Actual/committed input2,195,433, target157,500, discarded0/0, padding780,759;
10,240 draws,1280 per task and5120 per phrase. Generation2160 including P16,
teacher2144, active study accounting1558.679617875s. TINY41/561/561 separately.
The first real update was saved at6145 and restored in a fresh process. A real
900s TIME_BUDGET saved6912 as EvaluationPending:1214 generations/1213 teachers,
no other condition, resume=true. The next process completed34 generations and35
teachers before any further update, then512 updates and the final896/896 panel
observations. Final segment930 generations/931 teachers; no returned row repeated.
Final Finished/BUDGET_REACHED, resume=false. Pure paired-report exit0 verified
all raw/native/teacher/usage bindings. No UNKNOWN/cancel/storage failure.
Peak sampled RSS1,490,480KiB is not S6. Intermediate6400 raw is an observation,
not a separately retained native endpoint.

DERIVED_EXISTING_RAW, new model calls0:135/192 flipped first-token teacher
argmaxes incorrect; first mean NLL1.9982523555, remaining mean NLL0.1726558861.
These teacher metrics do not replace normal greedy scores. The fixed train-pair
margin diagnosis is next, bounded separately without learning.

Executable `artifacts/side-margin-20260920-executable`, SHA256
`189ef8057517064b7023ed5c4de39416a6dd7cd84dc6e3b9482f89932814acd2`;
source digest `a2a4c5d716bb9e313929ed5da1a08cba0fc6d33ad73fb0b5dc212a4154fec080`;
policy `8767859d6e4fa2d0ae2406a2d6995c68289a5e2796347404ef976c7d948907ab`.
Final `artifacts/side-margin-20260920/SIDE/segment-0002/final`, physical SHA256
`de1a59ee2152793ac67a2de0f6ecd97a1137ba72c14e7beb50dee8e2c2bb1f41`, weights
`ae949f151a87aa1585bd24748e08d7e638be240290785a885512db7984dbf849`.
The trainer's printed `sha256=38d2eb...` is its manifest weights digest, not the
whole-file SHA256. The physical value above is directly hashed and agrees with
the terminal checkpoint binding; this corrects the prior report's mislabeled hash.
Commands/tests/hashes/candidate.patch/input equality/pure report/error recount:
`artifacts/side-margin-20260920-evidence/`. Original artifacts stay unpublished.
CODE_VERDICT=IMPLEMENTER_TESTED_PASS; LEARNING_EXECUTION=COMPLETE;
DEVELOPMENT_JOINT_PASS=false; MODEL_QUALITY_RECOVERED=false;
S4/S5/S6=NOT_RUN_PRECONDITION_FAILED; GOAL1_READY=false; GOAL1_ACCEPTED=false.

## 2026-09-20 Separate side-margin objective — direct verification

One new SIDE registration compares separate per-side margins with the retained
CONTRAST sum-margin control at the same6400/7424 cursors. Same P6144 weights/
Adam/data/tape/tokenizer/LR3e-5/batch8; coefficient0.1 and unchanged response CE.
Only aggregation changes to the average of two margin1 softplus penalties.
The native descriptor family4/normalizer4 distinguishes this exact equation;
existing family3 semantics and inference framing are preserved. No new module,
framework, dataset, production annotation or decoding behavior.

EXECUTED_THIS_RUN: scalar value/gradient/equal-sum wrong-side/extreme/bounds
test1 PASS (0.00s, no model calls); full3840-row tape/binary test1 PASS (1.05s,
no model calls); native objective writer-reader/default-resume rejection test1
PASS (0.08s, no model calls). Actual TINY process regression1 PASS (65.29s):
41 optimizer updates,561 generations and561 teachers, including27 setup
observations of each kind. Continuous2 versus fresh1+1 weights/Adam match;
pure reporting leaves files unchanged and mixed-intervention/failure blocks hold.
Production P16 test1 PASS:16 exact outputs,160 tokens, errors0, optimizer0/
teacher0, originals unchanged. No SMALL training at this verification stage.

Rust/Cargo1.98.1, locked/offline, Accelerate/thread1. Release build and clippy
exit0; clippy reports existing map-key and new nested return-type style warnings.
Local commands, test lists, hashes and logs:
`artifacts/side-margin-20260920-evidence/`. Test binary SHA256
`bd97447c214bbcef99d440fdc607bfd59f895668e2acd317391f9bf3f762bae6`,
process test binary `6dd63e61c41ecf32cc86f69857cd52c57463548cbdc2d2ca4777203b5995c6db`.
The forthcoming1280-update bounded execution has no quality result yet.
CODE_VERDICT=IMPLEMENTER_TESTED_PASS; MODEL_QUALITY_RECOVERED=false;
FINAL200=NOT_OPENED; S4/S5/S6=NOT_RUN_PRECONDITION_FAILED;
GOAL1_READY=false; GOAL1_ACCEPTED=false; independent review pending.

## 2026-09-20 Fixed train-pair margins independently recounted

EXECUTED_THIS_RUN: existing seen-pair diagnostic, optional teacher-only mode,
one ignored test PASS in40.33s. First8 seen pairs per C/D/E were fixed before
calls; P6144 and CONTRAST7424 used the same24 pairs/48 sides. New SMALL/TINY
updates0, generation0, own-model teacher96, errors0, COMPLETED. Control-accounted
time20.744033792s excludes setup/load. These are train-only conditional teacher
logit margins, not normal generation scores or heldout quality.

| Model | Both positive /24 | Both>=1 /24 | Sum>=1 /24 | Sum>=1 but one<=0 /24 |
|---|---:|---:|---:|---:|
| P6144 | 0 | 0 | 0 | 0 |
| CONTRAST7424 | 4 | 0 | 8 | 4 |

Mean sum margins0.0249998371 and1.1160218641. Aggregate success can hide a wrong
side in actual trained examples. Most pairs also fail the aggregate margin, so
this does not establish a unique cause. It supports a next single-variable test
of per-side aggregation; no coefficient/LR/data/core search follows implicitly.

Pure Rust recount exit0 verified97 raw records, endpoint ordinals,96 prepared/
resolved calls, row hashes, start/raw digests, completion and every aggregate;
that recount made0 model calls. Clippy exit0 with the existing map-key warning.
Teacher records now identify the old `objective` field as per-example CE rather
than the batch auxiliary; old artifacts are untouched. No product inference change.

Source digest `fe165a6b63fec130d8d8b96b7f79a5f1956047823912252aed06104f405b317d`;
test binary SHA256 `52e9d5173a9145d4ae0afb222922effa5edec1620c85403c86399317cf4b1848`.
Evidence root `artifacts/pair-contrast-20260920-evidence/`, diagnostic raw
`train-margin96/margins.r3rows`, SHA256
`06bdbd9351de338efb50786f8bb9727e0d5e80da1890208faead688439beefe9`.
The command, test list/build, margin96.log, margin-recount.log and Rust recount
source remain local. Historical checkpoints/receipts are unchanged.
DIAGNOSTIC=VERIFIED; MODEL_QUALITY_RECOVERED=false; FINAL200=NOT_OPENED;
S4/S5/S6=NOT_RUN_PRECONDITION_FAILED; GOAL1_READY=false; GOAL1_ACCEPTED=false.

## 2026-09-20 Paired discrimination completed; joint quality still failed

EXECUTED_THIS_RUN, source `725f152e5376118439e9b153579d529a5835bdf7`.
All1280 registered SMALL updates completed from P6144 to7424. The exact data,
batch8 tape, Adam at entry, tokenizer, LR3e-5 and model matched COBATCH; the sole
intervention was the bound family3 paired auxiliary objective. First-update CE
was exactly0.3667096794 in both, objective0.4954743385 in CONTRAST versus the
unchanged CE control. The auxiliary is training-only and changes no generation.

| Absolute step | New updates | Train64 | Primary512 | Transfer128 | Flipped192 | Both192 |
|---|---:|---:|---:|---:|---:|---:|
| 6400 | 256 | 47 | 368 | 70 | 16 | 3 |
| 7424 | 1280 | 43 | 366 | 68 | 39 | 0 |

Final original64/192, same normal output145/192, all C/D/E both0. Primary
buckets `[53,60,23,28,13,64,61,64]`, transfer `[7,5,3,5,1,15,16,16]`.
Matched pure-CE COBATCH was371/512,60/128,21/192,both0; original77 and same179.
The256 both3 were D-only across2 bases; they disappeared at1280 and are not
promoted retrospectively. Screens32/64/128/768:47/47/42/43, flip24:0/0/1/0.
Every2144 evaluation generation had EOS/errors0. Improved flipped accuracy and
fewer identical outputs did not yield joint selection. The objective is not adopted.

Actual/committed input2,195,433 and target157,500, discarded0/0, padding780,759;
10,240 draws,1280 per task and5120 per phrase form. Generation2160 including
P16, teacher2144, active accounting1573.025655666s. A real900s stop at6912
saved EvaluationPending with1220 generations/1220 teachers, TIME_BUDGET only.
The fresh process completed the remaining28 observations before its512 updates;
that segment had924 generations/924 teachers. Pure `paired-report` exit0
verified native/raw/teacher/usage binding. Final Finished/BUDGET_REACHED,
resume=false. No UNKNOWN/cancel/storage failure. Peak sampled RSS1,413,152KiB
is not an S6 benchmark. TINY37/507/507 and scalar optimizer0 are separate.

DERIVED_EXISTING_RAW, no model calls:110 flipped first-token teacher argmaxes
incorrect, first mean NLL2.4091949819 and remaining mean NLL0.1480741784.
DERIVED_NATIVE_INPUT_LAYOUT, no model calls: every one of192 selector prompts
retains the full query inside the last local256-token window. Maximum prompt
lengths C/D/E225/239/237; maximum distance from query start to answer position
170/184/182. Thus simple query exclusion by that window does not explain these
failures. This does not establish how attention actually uses the visible query.

Executable `artifacts/pair-contrast-20260920-executable`, SHA256
`dc79629673ceb7e917837f0dbfbb6c7c49f2bc40816c29ecbd92dfcfcbfa914f`;
source digest `9d5b417a078d789d88683b28292dc105c9595e23773e19192bb4613f42af5f95`;
policy `d3cd05fca28be81675a48a5bcd7aadafa0f73ee26e6f5420354bd850de152f35`.
Final `artifacts/pair-contrast-20260920/CONTRAST/segment-0002/final`, physical
SHA256 `602fcbbef3e3c9bb861d9a716641475e55ecf353adaec0fb8dd51a30e5ed8f43`,
weights `e17058855e6d2e948922832a0cbdb46daecc6b972a7ab9f13341805bf2c47b15`.
Raw/teachers/decisions are in the arm; hashes, candidate.patch, commands,
direct tests, input proof, pure recount and Rust layout/error observations are
under `artifacts/pair-contrast-20260920-evidence/`. Originals remain unpublished.

CODE_VERDICT=IMPLEMENTER_TESTED_PASS; LEARNING_EXECUTION=COMPLETE;
DEVELOPMENT_JOINT_PASS=false; FINAL200=NOT_CREATED/NOT_OPENED;
S4/S5/S6=NOT_RUN_PRECONDITION_FAILED; GOAL1_READY=false; GOAL1_ACCEPTED=false.
Independent current review pending. Further grounded work uses a new scope under
the continuing user authorization; this study and its failed endpoint stay closed.

## 2026-09-20 Paired discrimination — implementation and direct verification

The next user-authorized single-variable trial is CONTRAST, from the same P6144
parent as the closed COBATCH control. Sample tape, all native source data,
tokenizer, Adam, LR3e-5, batch8 and model remain unchanged. Training adds fixed
0.1 mean softplus pair discrimination at the first distinct target token after
a shared teacher prefix. Existing response CE is separately logged. No extra
forward, external teacher, product annotation or decoding change. Native objective
family3 binds the equation/coefficient/training table without changing the byte
layout; default-loss resume rejects it. No new production module/framework.

EXECUTED_THIS_RUN: independent scalar gradient/value/common-bias/finite-extreme/
invalid-pair test1 PASS (optimizer0); full3840-row native tape and actual pair
mapping test1 PASS (0.87s, model calls0); native writer-reader/inference-view and
objective-binding rejection test1 PASS (0.06s, model calls0). Actual TINY process
regression1 PASS (61.91s):37 updates,507 generations/507 teachers including27
setup observations. Continuous2 versus fresh1+1 weights/Adam are exact, learning
counts/config/tape match pure-CE co-batch, trained weights differ, pure reporting
does not write, and unsafe-failure/conflicting-intervention gates remain enforced.

Initial scalar test listing compilation failed on an inferred i32 fixture index;
explicit usize fixed it before execution. That failed build remains recorded,
not counted as PASS. Final production P16 test1 PASS (14.18s):16/16 exact raw
outputs,160 tokens, errors0, optimizer0/teacher0, originals unchanged. Clippy exit0
with only the pre-existing map-key warning. Rust/Cargo1.98.1, locked/offline,
Accelerate/thread1. New SMALL learning0 at this verification stage.
Local evidence: `artifacts/pair-contrast-20260920-evidence/`. The subsequent
bounded1280-update trial must record its actual results separately. Existing
failures remain closed; joint quality, S4/S5/S6 and Goal1 are not accepted.

## 2026-09-20 Co-batch completed; joint selection not recovered

EXECUTED_THIS_RUN on frozen source `e1e2ca4098bc55ec6fc9407fd837bbb16b464b85`.
All1280 registered SMALL updates completed from P6144 to7424. Batch8 grouping
alone changed against the retained ADJACENT control; weights/Adam at entry,
data/two-update multisets, tokenizer, LR3e-5 and response CE remained identical.

| Absolute step | New updates | Train64 | Primary512 | Transfer128 | Flipped192 | Both192 |
|---|---:|---:|---:|---:|---:|---:|
| 6400 | 256 | 43 | 368 | 67 | 5 | 0 |
| 7424 | 1280 | 48 | 371 | 60 | 21 | 0 |

At matched7424, retained ADJACENT was366/512,73/128,33/192,both2/192.
COBATCH original77/192, same normal output179/192, every C/D/E both0;
primary buckets `[57,58,30,25,22,62,53,64]`, transfer
`[7,8,1,0,4,12,12,16]`. Screens32/64/128/768 were47/48/41/47 out of64;
flipped24 scores0/0/1/1. All2144 evaluation generations ended with EOS/errors0.
Grouping alone is not adopted: retention/transfer/joint selection failed.

Actual input2,197,229/target157,654 includes discarded1,796/154;
committed input2,195,433/target157,500; padding781,139.10,240 draws,
1280 per task,5120 per familiar/P phrase. Generation2160 including P16,
teacher2144, active accounting1586.816652667s. The first update was saved
and restored in a fresh process. A real900s timeout at6904 saved successfully,
TIME_BUDGET only, resume=true; its discarded work remains counted. The next
process completed520 updates and all remaining evaluations. Final native at7424,
Finished/NO_FURTHER_PROGRESS, resume=false; training also records BUDGET_REACHED.
Peak sampled RSS1,330,320KiB is not an S6 benchmark. TINY33 updates/453 generation/
453 teacher and scalar optimizer0 are separate. No UNKNOWN/cancel/storage failure.
Pure `fresh paired-report` exit0 verified raw, teachers, native, policy, exposure
and actual usage. No historical endpoint or operational pointer changed.

DERIVED_EXISTING_RAW, optimizer/generation/teacher0: final192 target pairs first
differ at token0 for172 and token1 for20.137 flipped first-token teacher argmaxes
are incorrect; first mean NLL1.9534215490, remaining mean NLL0.1666936514.
The repeated179 identical outputs support investigating conditional discrimination;
they do not prove an attention/optimizer defect. A local Rust recount initially
used a nonexistent singular teacher filename; corrected plural input completed
without model calls. No missing observation was converted into a zero score.

Executable `artifacts/cobatch-20260920-executable`, SHA256
`22909120b1a096e9fd72c673adf726d9a6a68a209ded143f2f3d14e18504aca3`;
code digest `b5701a3a5a4b4e437b1f4d73da3a0bd5ba68f05849b7a59d1c55852d50b86a4d`;
policy `2dd9a8da3361925032dede655a7291ebb4cc96dcc9b195097c115e53d050c232`.
Final native `artifacts/cobatch-20260920/COBATCH/segment-0002/final`, physical
SHA256 `a32f855b41d030ec65565cf4f0c57e9535a333c1cf34a948108563769056f699`,
weights `4355a6423b1e3f5e43fab028549e1b1ee29d505464c79ee4da1c264dd74c44ae`.
Raw/teachers/decisions are in that arm; test/source/binary hashes, candidate.patch,
input-equality proof, Rust recounts and command logs are under
`artifacts/cobatch-20260920-evidence/`. Original/private artifacts stay unpublished.

CODE_VERDICT=IMPLEMENTER_TESTED_PASS; LEARNING_EXECUTION=COMPLETE;
DEVELOPMENT_JOINT_PASS=false; FINAL200=NOT_CREATED/NOT_OPENED;
S4/S5/S6=NOT_RUN_PRECONDITION_FAILED; GOAL1_READY=false; GOAL1_ACCEPTED=false.
Independent current review is pending. A subsequent intervention must use a new
bounded policy; this closed run and its unused alternatives are not reopened.

## 2026-09-20 Co-batch research — direct verification before SMALL

The continuing user authorization permits a new bounded batch-grouping hypothesis.
`fresh paired-prepare --co-batch` retains P6144/Adam, coefficient1, LR3e-5,
microbatch8/accumulation1 and exact case multisets every two updates against the
retained ADJACENT control. C/D/E opposite-query pairs share one batch; per-update
task composition and padding change, while overall task/phrase/side exposure and
request/target content do not. This is not batch16 or a different loss. True task
IDs in traces/recounts now come from the existing sample mapping, not row slots.
No product inference, native format, tokenizer or model equation changes.

EXECUTED_THIS_RUN:3840-row writer-reader/bounds/multiset/co-location and true task
mapping test1 PASS; actual process regression1 PASS (54.65s), TINY33 updates,
453 generations/453 teachers including27 setup observations. It verifies
continuous2 versus fresh1+1 exact weights/Adam/counters, equal learning config
and input/target totals to control, changed trained weights, pure report no-write,
and rejection of combined interventions before registration. An initial test
listing build failed because the new test block was inserted in the wrong test;
it was moved to the intended existing regression before any test model call.
That failed build is retained and is not a passing test or an optimizer call.

After the process test, equivalent `chunks_exact(2)` became `as_chunks::<2>()`
to remove a new lint. Final-source tape regression ran again:1 PASS, no model
update/generation/teacher. Production P6144 parity1 PASS:16/16,160 raw tokens,
errors0, optimizer0/teacher0, original inventory unchanged (13.59s). Clippy exit0
with only the pre-existing map-key warning. New SMALL learning is0 at this stage.
Evidence and test/source/binary hashes remain local under
`artifacts/cobatch-20260920-evidence/`. Current quality and Goal1 remain unaccepted;
the subsequent1280-update experiment must be separately recorded with actual counts.

## 2026-09-20 Fixed LR9e-5 completed; no joint progress

EXECUTED_THIS_RUN on source `58a8a3bd98174b56bc05372a9cad8193995bfa3f`.
All1280 registered SMALL updates completed from P6144 to7424 with the same native
parent/Adam/data/tape and coefficient1; constant LR3e-5→9e-5 was the sole learning
intervention. The first step had identical CE0.3104520440 and gradient norm2.01995113
to control, while actual delta0.04843216 versus0.01614407 confirmed the LR path.

| Absolute step | New updates | Train64 | Primary512 | Transfer128 | Flipped192 | Both192 |
|---|---:|---:|---:|---:|---:|---:|
| 6400 | 256 | 43 | 342 | 63 | 13 | 0 |
| 7424 | 1280 | 45 | 330 | 60 | 35 | 0 |

At matched7424 the retained LR3e-5 control was primary366/512, transfer73/128,
flip33/192, both2/192. LR9e-5 original51/192, same output172/192, every C/D/E
both0; primary buckets `[57,60,24,17,10,53,45,64]`, transfer
`[7,7,1,0,3,14,12,16]`. Screens at32/64/128/768 were45/42/47/44 out of64;
flip24 scores0/2/1/0. All2144 new generation rows had EOS and errors0.
This LR is not adopted. It reduced retention and did not improve joint selection;
the underlying cause remains unresolved. No further LR sweep is registered.

Actual input2,195,433/target157,500, discarded0/0, padding892,783;
10,240 draws,1280 per task,5120 each familiar/P phrase. New generation2160
including P16, teacher2144, active accounting1562.780943332s. A real900s deadline
left step6912 in EvaluationPending, native saved, TIME_BUDGET only,1183
generations/1182 teachers in that segment. A fresh process completed the pending
evaluation before512 remaining updates; final segment961 generations/962 teachers.
Pure `paired-report` exit0 verified raw/native/teachers/cursors/LR and usage.
Final state Finished, stop NO_FURTHER_PROGRESS, resume=false; the training receipt
also records BUDGET_REACHED. Both reasons are retained. No UNKNOWN/cancel/numeric
or storage failure. Peak sampled RSS1,489,200KiB is not an S6 benchmark.
Direct TINY29/399/399 and scalar optimizer0 are separate.

Retained executable `artifacts/lr9e5-20260920-executable`, SHA256
`5ec8c779b1eabc8495aae63248e180254eccbc41bd0eda90791af97daa1fbe28`;
code digest `7ffa0ff8a1a266c1261f3d81bb79b3f9c99a178b12c2f9b5f350ef00c5283e85`;
policy `917cb8155feba14c0db8408ff0a2e4f89827600284cf1ab05dc38a913e18c9e7`.
Final native `artifacts/lr9e5-20260920/ADJACENT/segment-0002/final`, physical
SHA256 `3f7bdd1191cd851722a63c6088d527df4dc4b5111bd9a1d7c26b6e305fdfbb25`,
model hash `5d6d8d29ce3ab4199875d2a4aa79c78bc5d1e7fe029c0ace5318ff29334ad434`.
Its arm contains bound raw/teachers/decisions; input equality proof, test/source/
binary hashes, candidate.patch, command logs and final recount are in
`artifacts/lr9e5-20260920-evidence/`. Originals remain unchanged and unpublished.

CODE_VERDICT=IMPLEMENTER_TESTED_PASS; LEARNING_EXECUTION=COMPLETE;
DEVELOPMENT_JOINT_PASS=false; FINAL200=NOT_CREATED/NOT_OPENED;
S4/S5/S6=NOT_RUN_PRECONDITION_FAILED; GOAL1_READY=false; GOAL1_ACCEPTED=false.
Independent current review remains pending. The continuing user authorization
permits the next evidence-based single-variable experiment; no closed run resumes.

## 2026-09-20 LR-only research — direct verification before SMALL

The next bounded intervention uses the same P6144 parent/Adam/native data/tape
and default coefficient1, changing fixed LR3e-5→9e-5 only against the retained
coefficient1 control. Max1280 new updates; no first-target4 combination or LR
sweep. Source changes reuse `src/fresh.rs` and `tests/training.rs`; no new
production file, framework, model/storage schema or inference math.

EXECUTED_THIS_RUN, Rust1.98.1/Cargo locked/offline/Accelerate/thread1:
actual process regression1 PASS (48.43s), TINY29 updates/399 generations/399
teachers including27 setup observations; continuous2 versus1+1 exact native
weights/Adam, fixed LR/objective, pure report unchanged and conflicting
interventions rejected before registration. Existing failure interlocks passed
in the same test. Full3840-row tape/binary/bounds/schedule test1 PASS,
optimizer/generation/teacher0. After those tests, the equivalent `5|6|7` pattern
was changed to `5..=7` to remove a new lint; clippy exit0 with only the existing
map-key warning. This lint-only change does not alter accepted schema values.

Final production-source P6144 parity1 test PASS:16/16 raw outputs,160 tokens,
errors0, optimizer0/teacher0, original inventory unchanged (13.54s). New SMALL
updates are0 at this stage. Invocation logs and executable hashes are preserved
under `artifacts/lr9e5-20260920-evidence/`. Execution counts will be added after
registration and learning. No quality or independent acceptance is claimed.

## 2026-09-20 First-target4 completed; joint quality failed

EXECUTED_THIS_RUN on frozen source `958d78c25848dab4b7ee200aab3bf5b8e9867f14`.
All3840 registered new updates completed from P6144 at absolute9984. Only the
first-target coefficient changed1→4 against the retained coefficient1 control;
native data/tape, weights/Adam at entry, tokenizer, LR3e-5 and model were equal.
The pure `fresh paired-report` exited0, verifying raw/teacher/native/policy and
actual update, LR, sample and token traces. No old endpoint was rewritten.

| Absolute step | New updates | Train64 | Primary512 | Transfer128 | Flipped192 | Both192 |
|---|---:|---:|---:|---:|---:|---:|
| 6400 | 256 | 50 | 371 | 71 | 7 | 0 |
| 7424 | 1280 | 42 | 361 | 62 | 24 | 1 |
| 8448 | 2304 | 50 | 368 | 67 | 41 | 4 |
| 9984 | 3840 | 42 | 339 | 67 | 67 | 7 |

Final original47/192, both C/D/E=2/5/0,4 bases, base4=0, same output145/192.
Primary buckets `[57,56,16,19,12,64,51,64]`; transfer `[7,8,1,0,4,16,15,16]`.
The matched coefficient1 endpoint had primary368, transfer67, flipped56, both4.
Thus weighting increased a few joint choices while reducing retained QA; it is
not adopted. All4200 generated rows had EOS and errors0. A valid output can still
contain a wrong/invented citation; these remain incorrect answers in the score.

Actual learning input6,591,497/target473,391 includes discarded6,936/501;
committed6,584,561/472,890, padding2,678,055.30,720 draws,3840 per bucket,
15,360 familiar/15,360 P-variant. Generation4216 including P16, teacher4200,
registered active4396.389329876s. Four pure time stops saved and resumed in new
processes. Final phase Finished/BUDGET_REACHED, resume=false. No UNKNOWN,
cancel, numerical or storage failure observed. Peak sampled RSS1,537,600KiB
is a training observation, not an S6 benchmark. Direct TINY25/345/345 and
scalar optimizer0 are separate from these SMALL counts.

Executable `artifacts/first-target4-20260920-executable`, SHA256
`b35c1f6099e2d1fa7562431846a36a020e588998f34eb917602318a7ce26228f`;
code digest `c1fe084635de120f52ab5fd4f4b5997bde2a6fdc68be0f5e6f7e9cd8bf883cb1`;
policy `2592f13212738e608da61a050f774f9516e171c85eb0394cf321ec386e6c417f`.
Final checkpoint `artifacts/first-target4-20260920/ADJACENT/segment-0005/final`,
physical SHA256 `8632096c61b386808e822530be117891e0d294ae669a72c2e6162a7aeac003d8`,
model hash `0e8778ce106bf0a8ad1810095f8cbdc42cf1e774f2d56f3c1aa9f69823f956e6`.
Raw/teachers/decisions remain in that arm; command logs, candidate.patch, input
equality proof, pure recount and native/final metrics are under
`artifacts/first-target4-20260920-evidence/`. None are published.

CODE_VERDICT=IMPLEMENTER_TESTED_PASS; LEARNING_EXECUTION=COMPLETE;
DEVELOPMENT_JOINT_PASS=false; FINAL200=NOT_CREATED/NOT_OPENED;
S4/S5/S6=NOT_RUN_PRECONDITION_FAILED; GOAL1_READY=false; GOAL1_ACCEPTED=false.
Independent current review is pending. The user's one-variable improvement
authorization remains active. A new bounded LR-only hypothesis will compare
against the coefficient1 control, not combine changes with this failed weight4.

## 2026-09-20 First-target4 research — implementation and direct checks

The user authorized further bounded experiments changing one grounded variable
at a time. Existing `fresh paired-prepare --first-target-four` now registers a
single P6144/ADJACENT fork, coefficient4, at most3840 new updates. Existing response
loss/native objective binding and actual training/evaluation paths are reused.
Native file formats, model core, tokenizer, sampler/data, LR3e-5 and Adam are
unchanged. The retained coefficient1 control spans paired256+follow-through3584;
matched full evaluation steps are6400/7424/8448/9984. It is a historical control,
not a newly executed second arm or retroactive approval of either closed study.

EXECUTED_THIS_RUN with Rust1.98.1, locked/offline Cargo, Accelerate/thread1:
weighted scalar CE/gradient/mask test1 PASS;3840-row tape/binary/bounds/evaluation
schedule test1 PASS; paired fresh-process regression1 PASS (342.91s). The latter
checks coefficient4 continuous2 versus1+1 exact weights/Adam, native coefficient
bits, an actual weight difference from coefficient1, pure-report no-write and
retained quality/publication failure interlocks. Actual TINY optimizer25,
generation345/teacher345 including27 setup observations; scalar optimizer0.
An initial exact-name listing returned0 tests and is NOT counted as PASS; the
correct fully qualified scalar test then ran and passed. Clippy exit0 with the
pre-existing `quality_recovery.rs` map-key warning; no new warning.

Production-feature release P6144 parity ran1 ignored test explicitly:16/16 exact
raw outputs,160 generated tokens, errors0, teacher0/optimizer0; original inventory
unchanged. New SMALL learning is0 at this verification stage. Future run counts
will be recorded separately. Tests and hashes are local under
`artifacts/first-target4-20260920-evidence/`; source changes are in `src/fresh.rs`,
the existing scalar test in `src/training.rs`, and `tests/training.rs`.
Current quality and Goal1 readiness/acceptance remain false. No independent
acceptance has been claimed; conditional final200/S5/S6 remain pending quality.

## 2026-09-20 Fixed exposure follow-through completed; joint quality failed

EXECUTED_THIS_RUN on frozen source `cd52b21a9af5276ff53ab2f63b946d1266578bb8`.
All3584 registered new updates completed, ADJACENT6400→9984, with unchanged
weights lineage, Adam, LR3e-5, first-target1 CE, tokenizer and finite tape.
The old failed comparison was not promoted. The pure `fresh paired-report`
completed with exit0 after recounting bound raw, teachers, native state and actual
sample/LR/token traces. Source code digest:
`fae89422296da477e2a4f3e6ea0fb0ff252666fd616f8d577b01810494c6186c`.

| Absolute step | New updates | Train64 | Primary512 | Transfer128 | Flipped192 | Both192 |
|---|---:|---:|---:|---:|---:|---:|
| 6400, preserved parent | 0 | 50 | 367 | 70 | 15 | 0 |
| 7424 | 1024 | 44 | 366 | 73 | 33 | 2 |
| 8448 | 2048 | 53 | 360 | 68 | 40 | 3 |
| 9984 | 3584 | 41 | 368 | 67 | 56 | 4 |

Final both C/D/E=1/3/0,3 independent bases, base4=0; original68/192,
same normal output165/192. Primary buckets `[57,62,28,22,18,63,54,64]`;
transfer `[7,8,3,0,1,16,16,16]`. All3040 new generation rows have normal EOS
and errors0. Extra exposure improved some flipped answers but did not restore
retention or joint selection. The underlying cause remains unresolved.

Actual new input6,153,639/target442,017 includes discarded input6,887/target471.
Committed input6,146,752/target441,546; padding2,503,169;28,672 draws,3584 per
bucket,14,336 each familiar/P-variant wording.3040 generations/3040 teachers,
registered active accounting3918.07347796s. Four pure900s time stops saved native
state and continued in fresh processes; final phase Finished/BUDGET_REACHED,
resume=false. No UNKNOWN/cancel/numeric/storage failure was observed.
Training sampled peak RSS1,509,760KiB; this is not an S6 benchmark.
The preceding SPACED256+ADJACENT256 plus this3584 totals4096 updates,
7,029,257 attempted input/504,705 target. Including the fixed P16 and train48
diagnostics, SMALL generation5424/teacher5408. No further update belongs to these
closed budgets. This follow-through's direct TINY verification used21 updates,
291 generations/291 teachers; fixture scores are not SMALL quality evidence.

Retained executable `artifacts/paired-followthrough-20260920-executable`, SHA256
`2741f7cbd7df663427a3cf7b2ae2d838bcfaf1e12d0f8fe31850338356661292`.
Final checkpoint `artifacts/paired-followthrough-20260920/ADJACENT/segment-0005/final`,
native file SHA256 `4408580b7e7aeb96c04684ea201a9f5205d4f4739b2d4310cbba3c811e3bc328`,
model weight hash `f92bfc4da46d03acc74734a6b8b2e1ce8b67771c051474f08534717304cc98d0`.
Policy hash `c1373ad64ed13ba5df1bb6dcc8e872ccd1bee75e2e5addc71e3eed45c870db9c`.
Raw/teachers/decisions remain under that arm; execution logs, pure recount,
native inspection, final metrics and candidate.patch are under
`artifacts/paired-followthrough-20260920-evidence/`. None are published in Git.

Read-only own-model teacher recount supports testing first-target weighting:
the seen-train48 diagnostic had22 incorrect first-target argmax choices;
at7424 selector192 had121. Mean first/remaining token NLL was0.7813/0.1034
and1.7146/0.1696 respectively. This is a hypothesis, not proof of a loss defect.
`teacher-nll-recount.txt` contains the corrected analysis. The earlier local
`teacher-error-recount.txt` divergence fields are INVALID_FOR_GENERATION: the
existing teacher adapter passed an empty raw sequence, so its difference index
does not measure actual generation divergence. No historical file was overwritten.

CODE_VERDICT=IMPLEMENTER_TESTED_PASS; LEARNING_EXECUTION=COMPLETE;
DEVELOPMENT_JOINT_PASS=false; FINAL200=NOT_CREATED/NOT_OPENED;
S4/S5/S6=NOT_RUN_PRECONDITION_FAILED; GOAL1_READY=false; GOAL1_ACCEPTED=false.
The user explicitly authorized subsequent one-variable improvement. Next proposed
variable is the existing first-target coefficient1→4, same P6144/Adam/data/tape/LR,
with bounded registration and direct checks before execution. At this closure it
has not yet been implemented or trained. Independent current review is pending.

## 2026-09-20 User-requested follow-through — diagnosis and direct verification

The user requested continued improvement after the paired comparison closed.
The old negative comparison and all parent/native/raw/terminal artifacts remain
unchanged. New research is bounded to the remaining3584 ADJACENT tape updates,
with the same data/math/Adam/LR and original quality thresholds. It is not a
retroactive G3 gate pass or a claim that longer learning will solve selection.

EXECUTED_THIS_RUN: completed ADJACENT6400 on its actually seen first8 pairs per
C/D/E: full12/48, both0/24, identical normal output23/24, EOS48/errors0,
teacher CE0.15019075920000516.48 generations/48 teachers/optimizer0, active6.862s;
this is TRAIN_DIAGNOSTIC_NOT_HELDOUT. Raw and explicit start/finish are local at
`artifacts/paired-followthrough-20260920-evidence/seen-train48/`. The seen-pair
failure supports further examination of learning, not a specific mathematical bug.

The existing Plan/Fork/finite tape/evaluator and native resume path now register
an explicit single-arm follow-through with an immutable parent and scope record.
Production generation/model/loss math is unchanged. New process regression passes
continuous2 versus split1+1, exact weights and Adam, parent preservation and pure
report no-write; existing pair/publication failure checks remain in the same test.
This test used21 TINY optimizer entries,291 generations and291 teachers including
its setup observations. One zero-model plateau test passed. The diagnostic used
production Accelerate/thread1; fixture scores do not constitute quality evidence.
Evidence logs: `artifacts/paired-followthrough-20260920-evidence/`.

New SMALL training has not started at this verification stage. Goal1 readiness
and independent acceptance remain false. Subsequent executed counts supersede
this stage's zero; no historical budget or failure is rewritten.

## 2026-09-20 Paired exposure — G2 executed and closed; joint quality failed

`STUDY_COMPLETE_NO_ADMISSIBLE_LEARNER` / EXECUTED_THIS_RUN.
Both policies completed256 real SMALL updates from the same retained P6144
weights/Adam. Both6400 endpoints are durably saved, phase Finished,
stop BUDGET_REACHED, resume=false. No execution/storage/numeric/time/cancel error
was observed in these four training commands. No further learning is authorized
by the conditional gate: neither endpoint retains primary417/512 and transfer75/128,
and neither produces a both-correct selector pair. No G3 restart was registered.

| Stage | Actual source | Updates | Generation / own-model teacher | Quality | Save / resume | Independent verification | Next condition |
|---|---|---:|---|---|---|---|---|
| G0/G1 | Working changes subsequently frozen as `2dfa56f`; exact test logs retained | SMALL0; TINY49 including failed attempts | SMALL16/0; TINY676/675 | P16 parity16/16; fixtures are not quality evidence | Both policies' continuous2 versus split1+1; evaluation-only resume passed | Prior R2/R3 reused; current changes pending review | Passed for G2 |
| G2 SPACED | `2dfa56faba85698ea386aaf23f387c577aa2c208` | SMALL256 | 1160/1160 | Primary375, transfer65, both0 | First update saved; new process continued255; durable6400 | Implementer executed and recounted; independent review pending | G3 entry failed |
| G2 ADJACENT | Same frozen source and executable | SMALL256 | 1160/1160 | Primary367, transfer70, both0 | First update saved; new process continued255; durable6400 | Implementer executed and recounted; independent review pending | G3 entry failed |
| G3–G7 | NOT_RUN | 0 | 0/0 | No qualifying candidate | No continuation registered | NOT_RUN | Joint signal and retention preconditions failed |

### Actual endpoint results

| Metric | P6144, existing verified raw | SPACED6400 | ADJACENT6400 |
|---|---:|---:|---:|
| New optimizer updates | 0 | 256 | 256 |
| Train probe | historical, not regenerated | 51/64 | 50/64 |
| Primary full | 425/512 | 375/512 | 367/512 |
| Transfer full | 79/128 | 65/128 | 70/128 |
| Selector original full | 131/192 | 85/192 | 83/192 |
| Selector flipped full | 2/192 | 5/192 | 15/192 |
| Both full | 0/192 | 0/192 | 0/192 |
| Both C/D/E; independent bases; base4 | 0 | 0/0/0; 0; 0 | 0/0/0; 0; 0 |
| Same normal output across the pair | 184/192 | 175/192 | 175/192 |
| New generated rows / errors | P16 parity16 / 0 | 1160 / 0 | 1160 / 0 |

Primary task counts A–H: SPACED `[56,61,40,28,17,55,54,64]`, ADJACENT
`[52,61,37,31,15,55,52,64]`, each denominator64. Transfer counts:
`[4,8,7,1,3,15,15,12]` versus `[7,8,2,3,6,13,15,16]`, each denominator16.
The +32/64/128 screens were47/49/45 and47/50/46 out of64; flip24 counts1/0/0
and0/1/0. Every screen both count was0; no registered retention stop was triggered.

At the equal256 endpoints, ADJACENT versus SPACED paired gain/loss is29/37
on primary,14/9 on transfer,14/4 on flipped selector, and0/0 on both. The extra
flipped answers do not establish joint selection: both remains0, including every
bucket and independent base. The preregistered practical both difference>=8 was
not observed. Retention deteriorated in both arms. This observation does not
identify a particular tokenizer, optimizer or model equation as the root cause.

### Actual usage and preserved state

Each arm consumed437,809 input and31,344 target tokens; combined875,618/62,688.
Every bucket had256 draws per arm. C/D/E each had128 original and128 flipped
draws; each side split64 familiar/64 P phrase forms. Other five tasks and their
wording matched update by update. The actual full-block sample/target multiset
matched between arms. Padding differed as permitted:175,151 versus175,191 tokens.
Discarded input/targets0, incomplete/unknown model calls0. The4,096 SMALL ceiling
was not extended:512 used,3,584 conditionally reserved but gate not satisfied.

New SMALL generation2336 = P16 +2320 study rows; required own-model teacher2320;
all generation rows normal EOS, strict UTF-8, errors0. G2 raw output tokens32,127,
plus160 parity tokens. Registered active accounting972.789972375 seconds, within
21,600; every command stayed under900 seconds. Training sampled peak RSS was
1,473,904 KiB versus1,448,016 KiB (not a full-system or S6 benchmark).
TINY totals including failed tests remain optimizer49/generation676/teacher675.

The pure report rechecked fixed cases, native/tokenizer/policy/model identity,
raw tokens/strict decode/EOS, teacher obligations, actual tape/LR/input/target
traces, summaries, both and paired metrics before reporting no admissible learner.
The entire new study inventory before/after report matched byte-for-byte:
9,406 files,2,106,110,913 bytes, manifest SHA256
`9f1c2ecf50c27ffcee2ae5f60b052cb82848d0886b70100008ee9712f8962768`.
The original P checkpoint and three consumed source corpus hashes still match
their pre-run identities. No operating model pointer or user DB was changed.

### Frozen source, native endpoints and local evidence

Executed source commit, normally pushed and remote-verified before training:
`2dfa56faba85698ea386aaf23f387c577aa2c208`.
Code digest `66c087f68914e631200d89878049c3a02a623711a6b72b6d3f06d123b2712811`.
Retained release/Accelerate/F32/thread1 executable:
`artifacts/paired-learning-20260920-executable`, SHA256
`428fdd43c30be8ff7cd9af63ef0ddc4c5b8adbfced4a5fe0daad53b16a142db7`.
The earlier production-feature P16 used a debug test executable of the identical
code digest; its wall time is not a release generation benchmark. Source/model
semantics stayed frozen throughout both arms. This subsequent commit is a report
update, not the source used to learn.

Study root: `artifacts/paired-learning-20260920/`. Each endpoint is
`SPACED/segment-0001/final` or `ADJACENT/segment-0001/final` under that root,
114,180,928 bytes each; fresh native loading with optimizer state verified:

| Binding | SPACED | ADJACENT |
|---|---|---|
| Native file SHA256 | `2eac4af4cd8c8ee2a3906007b69766f37ecd6c7f0acedfa2acd4831b6dc996f2` | `9a907f3d965a94d11eed5a4f7b17f914ffa594d9a68b0cae657c23482f9aae41` |
| Weight hash | `350213571e38b7288040ec06ba83fc99654511f78137cf03485f541ab30650c6` | `c55842369f0777d28e718d346832e9b049d319ed8e233bc09f1f1b2e8eadec72` |
| Adam hash | `a806d498a3319753321e92d27e042dcc2fed7d0ad19ec53966bde82e8bfb5420` | `20a1eddd1081e7e199797d69ceb55e2d2b60ccbebb76e5ded092b1b764a4b440` |
| Policy hash | `a82826072e368ca2d4cac8bee925b0830f82686ae1922e4a7e9c78f51f9668a3` | `6c5f3cd7c6919e8f5eb5245c2c669bd7a6c3d84bde9659a128c0d5f2d758299d` |

Authorized evidence paths: each arm's `plan.r3b`, `paired-samples.r3rows`,
`corpus.r3cor`, `variants.r3cor`, `selectors.r3cor`, `tokenizer.r3b`,
`segment-*/updates.r3rows`, `segment-*/train-control.r3b`,
`segment-*-started.r3b`, `segment-*-finished.r3b`,
`eval-6400-{train64,dev512,transfer128,selector192}.r3rows` and their teacher,
call-resolution and summary siblings; `paired-6400.r3b`; root `study.r3b`,
`study-ready.r3b`, `parent-audit.r3b`, `diagnostic.r3b`.
`artifacts/paired-learning-20260920-evidence/` contains exact command logs,
`paired-report.log`, before/after inventories, `native-endpoints.txt`,
`control-usage.txt`, `panel-metrics.txt`, `raw-usage.txt`, `learning-usage.txt`,
`originals-after.sha256`, `candidate.patch` and the G1 test/P16 evidence below.
The patch compares the accepted report HEAD `da4d3dc…` with executed source2dfa56f.
Only code/tests/docs are on GitHub; all models, corpus and raw stay local.

CODE_VERDICT=IMPLEMENTER_TESTED_PASS; ACCEPTED_STABILIZATION_REUSED=true;
PAIR_EXPOSURE_EFFECT=NO_BOTH_GAIN_OBSERVED; DEVELOPMENT_JOINT_PASS=false;
FINAL200=NOT_CREATED/NOT_OPENED; S4/S5/S6=NOT_RUN_PRECONDITION_FAILED;
QUANT_ADOPTION=NOT_RUN; DEPLOYMENT_PROFILE=F32_RESEARCH_ONLY;
GOAL1_READY=false; GOAL1_ACCEPTED=false; INDEPENDENT_CURRENT_REVIEW=PENDING.
The authorized bounded comparison is closed. Goal1 quality and acceptance are not.

## 2026-09-20 Paired exposure — preceding G0/G1 verification

R3-PAIRED-LEARNING-TO-GOAL1-1.0 / IMPLEMENT_AND_EXECUTE.
Accepted stabilization R2 and posthoc R3 are reused from reports
`898f57d7dd8109386f23549d391618c558097774` and
`da4d3dc29a092c13f5875182d0e5d45596aa53a7`. No repeat of the15-test acceptance or
384-row posthoc generation. Existing parent/corpus/failures and `.DS_Store` remain.

The retained parent is
`artifacts/fresh-exposure-phrase-20260919/P-PHRASE/segment-0003/final`, physical
`c4d6989fbcaad60516053dda206caf43abc08c4b388c3f765e4c446864f60e20`.
Train inputs are the original/phrase/selector native files under
`artifacts/selector-consistency-20260919/S-SELECT/`; physical hashes respectively
`108171c2ffad62afc9b0f09a52070871a4d3287017676208e9f43a88631fe04f`,
`14eb2cc2b5fd1f599510e49f7cda97a0d34673d651547e685f84c6aaab3b6fa9`,
`6e0c0cbdb669e62f0d005d075ff30f14cb1d37d228b45bec7e39676dbadd270b`.
Original file hashes and owned episode contents are verified separately: two
files can hold identical P phrase cases but different provenance metadata.

New production policy uses the existing Plan/Fork, native checkpoint, sampler,
RunControl, evaluator, teacher receipts, confirmed publisher and pure scorer.
It binds3840 finite tape rows, matching train request/target indexes, absolute
Adam clock and explicit local cursor. Only pair spacing differs. Actual optimizer
entries and combined input/target use include unsuccessful work. Quality stops
permit the peer budget; command/storage/unknown failures block the study.

Direct executed tests (all nonzero counts):

- `paired_tape_full_writer_reader_bounds_and_balance`: full3840-row publisher/read,
  gap128/1, balanced sides/phrases, matching multisets, unchanged five tasks,
  duplicate/missing/unknown cases and continuation bounds; optimizer/generation0.
- `fresh_paired_policies_match_continuous_and_split_processes`: both policies'
  actual TINY continuous2 versus1+1 match weights/Adam/clock; pure reports preserve
  file hashes; safe quality stop permits peer, failed sync blocks it in a new process.
- `fresh_accumulation_and_metadata_boundary`: batch-independent gradient equality,
  native metadata/trace boundary; optimizer0, backward3, synthetic trace is not training.
- `fresh_fx03_final_step_resumes_only_remaining_evaluation`: existing EOS tensor
  fixture, actual new-process evaluation-only continuation; no extra optimizer step.
- `first_target_objective_matches_scalar_ce_and_gradients_without_mask_leakage`.
- `native_numeric_references_and_causal_padding_gradients`.
- Production `stabilization_fixed_parent_parity`:16/16 identical raw output,
 160 tokens, original inventory unchanged, teacher/optimizer0;265.81s total test wall.

Earlier test failures are retained: phrase file-hash versus content comparison,
missing optimizer-entry field, and a random TINY mixed integrity/time stop before
the final-row EOS fixture was configured. The latter remained blocked; the intended
normal-return/time boundary passed with the existing EOS fixture. One short-name
exact filter ran0 tests and is excluded from PASS. Across all attempts TINY actual
optimizer49, generation676, teacher675; no unknown model usage. SMALL optimizer0,
generation16, teacher0. The reported fixture scores are not model-quality results.

Rust/Cargo1.98.1, locked/offline, Accelerate, one compute thread. Production build
and clippy exit0; one pre-existing `quality_recovery.rs` map-key warning remains,
no new warning. No whole test suite or unrelated formatting migration was run.
Production trainer SHA256:
`19c8acd49f9d9fd3f7c4fc0d4f9516a700b4d719302cde4de1d6d7992b82aaa7`.
Local evidence: `artifacts/paired-learning-20260920-evidence/`, including named
test logs, failure logs, executable hashes, P16 raw/selection/result, and Rust
read-only metadata/count helpers. These artifacts are not uploaded.

G2 registration/execution is next under the active quality plan. S4 final200 was
not found in the authorized fresh-study roots: NOT_CREATED, answers not opened.
CODE_VERDICT=IMPLEMENTER_TESTED; ACCEPTED_STABILIZATION_REUSED=true;
PAIR_EXPOSURE_EFFECT=NOT_RUN; DEVELOPMENT_JOINT_PASS=false; FINAL200/S4, S5,
S6/QUANT_ADOPTION=NOT_RUN; deployment remains unaccepted F32 research;
GOAL1_READY=false; GOAL1_ACCEPTED=false. Source and report publication are distinct.

## 2026-09-20 Equal-step selector — Q0–Q2 completed, quality not passed

MODE=IMPLEMENT_AND_MEASURE / EXECUTED_THIS_RUN + DERIVED_EXISTING_RAW.
R3-ACCEPTANCE-AND-QUALITY-CLOSURE-1.0 implementation scope is complete.
STABILIZATION_ACCEPTED=true (separate R2 report), EQUAL_STEP_SELECTOR_OBSERVED=true,
NEW_SMALL_UPDATES=0, H3/S4/GOAL1=false. New adapter/results await independent R3;
the implementation has not self-granted independent acceptance or Goal1 approval.

### Actual result and its limit

Both native7168 models passed the same metadata-fixed16-case output parity against
their own historical primary raw: tokens/text/error/EOS/prompt matched16/16 each.
Their answers were13/16 and6/16 correct; parity is not a correctness score.
Each then generated the frozen selector192 exactly once. All416 calls returned,
EOS416, errors0, UNKNOWN0, raw tokens5,998 (parity153+153, selector2,851+2,841).
No teacher, backward, Adam, SMALL/TINY/scalar updates or new training occurred.

| model | original full /192 | flipped full /192 | both | original-only | flip-only | neither | same valid output | errors |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| C7168 | 155 | 8 | 0 | 155 | 8 | 29 | 184 | 0 |
| S7168 | 70 | 28 | 0 | 70 | 28 | 94 | 180 | 0 |

The192 rows represent48 bases×4view, not192 independent scenes.
The original primary counts were revalidated from full512 rows and their native,
policy, teacher and call receipts: C436, S350; C/D/E155 versus70, other tasks281 versus280.
Primary originals were reused, not regenerated except the explicitly allowed parity16.

| bucket /64 | C original / flipped / both | S original / flipped / both |
| --- | --- | --- |
| C: entity selection | 46 /0 /0 | 20 /14 /0 |
| D: context selection | 56 /0 /0 | 27 /6 /0 |
| E: observation time | 53 /8 /0 | 23 /8 /0 |

| metric | C7168 | S7168 |
| --- | ---: | ---: |
| original base4 /48 | 23 | 7 |
| flipped base4 /48 | 0 | 0 |
| both base4 /48 | 0 | 0 |
| original body / citation exact | 170 /164 | 135 /87 |
| flipped body / citation exact | 15 /21 | 45 /94 |
| flipped citation: selected / other provided / absent / none | 21 /165 /4 /2 | 94 /84 /8 /6 |
| multiple distinct citations / malformed citation syntax | 0 /0 | 0 /0 |

Every pair's full flags, both, same-valid-output, bucket/base/view and citation IDs/
classes are present in the pure report output; raw tokens remain in native row files.
The fixture explicitly checks mixed citations, and the report retains multiple
classes instead of collapsing such a row into a misleading exclusive category.
C→S flipped full gain/loss=24/4, net+20; both-correct gain/loss=0/0.

Interpretation: RULE_SWITCH_WITHOUT_JOINT_SUCCESS is consistent with the observed
outputs. S answered more flipped cases but lost85 original C/D/E answers, and neither
model solved both sides of even one pair. This is not evidence of reliable selection.
Both models received distinct prepared prompt digests for all192 flips; both original
and flipped prompts retained both supplied records in all192 pairs, with no exclusions.
Despite those changed inputs, outputs stayed identical184/192 and180/192. This rules
out identical framed inputs or dropped evidence as the explanation for these pairs;
it does not establish an optimizer/architecture/attention cause. The model's field
and citation failures remain separate observations. No tokenizer/LR/binary-format
cause is inferred. New training0 means MODEL_QUALITY_IMPROVED_THIS_RUN=false.

### Source, execution, preservation and commands

SOURCE_COMMIT / CODE_SHA: `e71345e66e18d221db2e5cb43fb67569c6eace8e`.
It was normally pushed and actual origin/main matched before observation.
[Candidate diff](https://github.com/seoyd/replica-v3/compare/abd967980645a878f22069708daa9fbfce70bf87...e71345e66e18d221db2e5cb43fb67569c6eace8e).
Code-only diff is411 test-module lines in `src/fresh.rs`; product generation,
checkpoint, training, tokenizer, timeout and storage implementations are unchanged.
Patch SHA256: `19ad3941dd99b9bc1c8cc79ef325ea62677e150612d2ce91d9c99543d7bf893c`.
Compiled source digest and observer hash are in the Q0 identity section below.
The observer file was retained before registering or executing observations.

NEW_SMALL_GENERATIONS/TEACHERS=416/0; NEW_TINY_UPDATES/GENERATIONS/TEACHERS=0/0/0;
scalar0. Separate prior R1 used16 SMALL generations; contract total observed so far432,
with32 reserved for independent R3, not borrowed by this implementation.
Command reason=COMPLETED, observed_conditions=[], resume=false, command segments1,
no retry. RunControl39.093786292s, complete test/OS command44.12s, maximum command RSS
1,450,409,984 bytes. These include verification/load/I/O, not an inference benchmark.
Last durable output: `observation/command-0000-finished.r3b`; no new checkpoint exists.
Terminal SHA256 `99ef6c2f30f10e4351fc5c9dac20b892b4e987109fee1f0789eaae0cba19778f`.

Final unique test functions4 passed: the three directly relevant fixtures below and
the explicit ignored observer in register/observe/report modes. Actual test command
invocations8:7 PASS,1 initial fixture assertion FAIL preserved. No zero-test PASS.
The model-free modes registered inputs and independently reread results in separate
processes; only observe invoked generation. Existing R1 tests were not rerun or counted
as new implementation-side tests. Broad clippy failure and its limits are recorded below.

```sh
VECLIB_MAXIMUM_THREADS=1 RAYON_NUM_THREADS=1 cargo test --locked --offline \
  --release --features accelerate --bin replica-train \
  training::fresh::tests::posthoc_mapping_and_binary_pair_counts \
  -- --exact --nocapture --test-threads=1

# The frozen observer is a copy of that production-feature test executable.
# Actual modes were register, then observe, then report, once each.
VECLIB_MAXIMUM_THREADS=1 RAYON_NUM_THREADS=1 \
R3_POSTHOC_STUDY=artifacts/selector-consistency-20260919 \
R3_POSTHOC_EXECUTABLE=artifacts/selector-consistency-20260919-executable \
R3_POSTHOC_OUTPUT=artifacts/acceptance-quality-closure-20260920/observation \
R3_POSTHOC_MODE=observe \
  /usr/bin/time -l artifacts/acceptance-quality-closure-20260920/observer \
  training::fresh::tests::posthoc_equal_step_selector \
  --exact --ignored --nocapture --test-threads=1
```

READ_ONLY_INPUTS_UNCHANGED=true:4,129 bound files agreed at registration, observation
completion and pure reporting. This includes actual native bytes (weights and Adam),
tokenizer, data, policies, step7168 primary/teacher rows and their call receipts.
Only the new observation directory was written. Old C/S outcomes, failures, raw and
resume flags remain intact. Existing `.DS_Store` is preserved; no reset/stash/clean.
Strict score is independently rebuilt from frozen labels, decoded tokens and EOS;
self-reported flags must agree. Expected/metadata never enter native logits: existing
tokenizer preparation receives only ModelRequest, and native generation receives its
prepared token IDs and generation limits. Source is unchanged after the frozen commit.

Allowed local evidence root: `artifacts/acceptance-quality-closure-20260920/`.
`register.log`, `observe.log`, `report.log`, `input-identities.log`, `final-counts.log`,
`input-response-check.log`, fixture logs, frozen `observer`, `candidate-code.patch` and
`observation/` are retained locally, not uploaded. The small local Rust reader only
summarizes existing binary records; it makes no model calls and is not an independent review.
Observe log SHA256 `d5e9e3024d6ababd080c06e1224f04822b43c616622627a4f3b2f69069f77792`;
pure report log SHA256 `918c373f166656e98791c20d094efa9ba19c0af9482eda90b1ca62f2d22805dd`.

| observation raw | SHA256 |
| --- | --- |
| `arm-0-parity.r3rows` | `a83ad7ea8f74dbd493c26757af4400eb49925d236957e8af50f4834a17917781` |
| `arm-1-parity.r3rows` | `1a5d9bc9c054d43e1e75042f5609b64d3e8bfa12683a04c48568d702d9d5076d` |
| `arm-0-selector.r3rows` | `ba744a2bacc0fc804788b567d652e3e28e2a99a649ce3326d823f896448c1be7` |
| `arm-1-selector.r3rows` | `2d91cd5ec8b1d9aac9761da3199521d2d277dfd3d6eb29149c93bd1f405d63d3` |

The two source raw panels remain `C-KEEP/eval-7168-dev512.r3rows` and
`S-SELECT/eval-7168-dev512.r3rows` under the existing selector study. Their hashes are
`8470f320f9b3a4e469a2a5d84068c306d1730a7435d99d1a7c11ecac623c65f2` and
`66790e61eef433c46fe67ea77427158ad8bcfaca5a744d02a0ab1dc86c1dae40`.
The original192 are bound by the verified full primary dataset digest plus the ordered
selector source-ID mapping; no later re-selection of cases was performed.

### Next single hypothesis — proposal only, execution0

Question: can shorter separation between a scene's original and counterfactual training
exposures improve joint correctness instead of replacing one answer preference with another?
Propose comparing the preserved selector ordering with an ordering that places matched
original/flip exposures in consecutive updates. Change only that spacing/order; retain
the same P6144 parent, same total examples/wording/flip share, batch size, tokenizer,
Adam, CE and LR. Do not change to selector25% or add new training text. Proposed cap256
updates per arm, total512, with matched32/64/128/256 checks and no automatic extension.
Require both-correct gains together with no primary/transfer loss against the control;
stop on execution/numerical/storage errors or the preregistered retention boundary.
An independent reviewer must freeze the exact training tape, unseen acceptance data
and numeric stop rules before approval. The now-exposed192 remain development data;
they cannot be renamed sealed evaluation. This is a hypothesis, not a proven cause or
authorization to train. R3 raw verification comes first.

PRIMARY_GATE=FAIL (487/512 and bucket58/64 unmet); TRANSFER_GATE=FAIL on historical
same-step68/128 and67/128, not regenerated here. FINAL200=NOT_OPENED.
H3/S4=NOT_PASSED, S5/S6=NOT_RUN_PREREQUISITE, GOAL1_READY/GOAL1_ACCEPTED=false/false.
STABILIZATION_SCOPE_VERIFIED_INDEPENDENT=true applies to accepted abd967 source
boundaries only. INDEPENDENT_DYNAMIC_ACCEPTANCE of this new diagnostic diff/results
is PENDING_R3. Q2 publication changes these status/plan docs only; report commit and
actual remote SHA are recorded in the final handoff and local publication log.

## 2026-09-20 Acceptance / equal-step selector — Q0 complete

MODE=IMPLEMENT_AND_MEASURE; R3-ACCEPTANCE-AND-QUALITY-CLOSURE-1.0.
STABILIZATION_ACCEPTED=true per the separate independent report
`docs/INDEPENDENT_ACCEPTANCE_2026-09-19.md`, commit
`898f57d7dd8109386f23549d391618c558097774`. Its15 unique test exits and four executable
hashes were checked against local evidence. This is not a new implementation-side
independent review. Original accepted source is `abd967980645a878f22069708daa9fbfce70bf87`.

Q0 EXECUTED_THIS_RUN: C/S native absolute7168 and original primary512 raw/teacher/
completion bindings verified with existing readers and scorer: C436, S350, errors0.
The serialized selector mapping is C/D/E64 each,48 bases×4view, unique IDs, validated
request-only labels and inverse selections. No generated replacements are used as inputs.
Original C/D/E full155 versus70; five other tasks281 versus280.

The only source diff is a test-only adapter and a direct byte/mapping fixture in
`src/fresh.rs`'s existing test module. No product inference/trainer/timeout changes.
It separates register/observe/report, binds actual inputs/checkpoints, uses existing
RunControl and prepared/resolved calls, preserves partial rows and forbids teacher.
Test-only source digest `44351ced9f77c59ea79521ec438f3a4850db24f0dccc90c15209218cf54c28e1`;
production-feature observer SHA256
`542471cb669eeac618feae0a8fbb3b78a13ee3f42f5cf46cf943172cdd043fe0`.
Rust/Cargo1.98.1, locked/offline release, Accelerate/F32 CPU, compute thread1,
test-support absent. Product model policy/source remains historical, not overwritten.

Direct unique fixture tests3 PASS: `posthoc_mapping_and_binary_pair_counts`,
`fresh_strict_rows_roundtrip_command_stop_and_errors`,
`selector_involution_labels_balance_and_negative_cases`. Generation/teacher/optimizer0.
The first new fixture run failed because its expected citation order ignored the
existing citation parser's sorted IDs; corrected the fixture assertion and retained
the failed log. The final mapping fixture verifies4 malformed mappings,12 exact
pairs, null/error equality exclusion and mixed citations with explicit IDs/classes.
Its binary writer/reader uses the real codec. Strict rows cover6 paths; existing
selector label test verifies3,072 involutions and6 negatives without model calls.

One broad `clippy --tests` invocation also selected unrelated `tests/journal.rs:9`
and failed its existing unused `Command` import. The adapter's unused local was
removed and final targeted compile/tests passed. This is not whole clippy/fmt PASS;
the prior `for_kv_map` debt remains outside scope. No unrelated source was changed.

Allowed evidence root: `artifacts/acceptance-quality-closure-20260920/`.
Registration `observation/registration.r3b` SHA256
`7f6df85ea191b8cad8cd3d7fd7b71317cbb471f785694f323ebc97f6ff33b3f0` binds4,129 directly
read files (step7168 raw/call receipts, native files, corpora, policies and mapping).
Input hashes agree after Q0. No operating DB/private memory or final200 access.

| step7168 | checkpoint under `artifacts/selector-consistency-20260919/` | native file SHA256 |
| --- | --- | --- |
| C | `C-KEEP/segment-0002/step-007168` | `b74292531bf00ab1c21157936254f1205835a9771b969991b5b5f59bf16e407a` |
| S | `S-SELECT/segment-0002/step-007168` | `6c31d173c31a4f08e600f901289fa16d2779dd8639030d482a18b43b2bbbc8a0` |

The S step artifact matches its final file bytes. C8192 is not substituted for C7168.
Stored diagnostic semantic hash
`8cff0fe541fbe8596be688e2c86d9bd2250eeae75bd10d4e2ba504b5a4c179ce`, mapping hash
`d6b15b54f9cb765b38686a2f4d045ed70fb983b659ff9d34dc96d33802559045`.
Before observing output, parity indices are fixed as0..5,64..68,128..132.
Q1/Q2=NOT_RUN at this source freeze. NEW_SMALL_UPDATES/GENERATIONS/TEACHERS=0/0/0.
No new TINY/scalar updates or model calls. H3/S4/GOAL1=false; final200=NOT_OPENED;
old study remains inconclusive with candidate=null and S resume=false.

## 2026-09-19 Existing path stabilization — 검증·무학습 관측 완료

RESULT=PASS / STABILIZATION_SCOPE_VERIFIED=true. 아래 수리 source 검증 후 코드 변경 없이
기존 원자료 재채점과 P6144 고정16 출력을 완료했다. 이 절의 후속 결과는 report-only다.
품질 PASS가 아니며 새 SMALL 학습은0이다. 모델 품질 회복은 NOT_CLAIMED,
H3/S4=NOT_PASSED, S5/S6=NOT_RUN_PREREQUISITE, GOAL1_READY=false,
GOAL1_ACCEPTED=false, INDEPENDENT_REVIEW=NOT_RUN이다. final200은 개봉하지 않았다.

### Candidate·실행 identity·보존 확인

- Source: `abd967980645a878f22069708daa9fbfce70bf87`.
  origin/main 정상 push 후 실제 remote 전체 SHA 일치를 확인했다.
  이 source에서 strict binary row 회귀를 재확인했고, production feature로 P parity를 실행했다.
- [실제 코드 diff](https://github.com/seoyd/replica-v3/compare/b0e38e18cb66b87bdaf1f657b8e74332723cb996...abd967980645a878f22069708daa9fbfce70bf87).
  코드만의 diff: `git diff b0e38e18cb66b87bdaf1f657b8e74332723cb996 abd967980645a878f22069708daa9fbfce70bf87 -- src tests`.
- Current source digest: `cb89a1598abcea8baeab82e547d14feedfc2701295fcf996d1da9d781212ffe9`.
  보존한 release 보고 실행물: `artifacts/existing-path-stabilization-20260919-executable`,
  SHA256 `12645bd790286ed444fa05c4b72c4ef29718b05acac9b91412afd399ab7a71aa`.
- 실제 parity test 실행물: `target/release/deps/replica_train-b3f42933fdd30cfd`,
  SHA256 `af882a83366af9e4db21a0570a4fc172d9a3c7d514f0547bfe5dc502c31ab1b1`.
  `--features accelerate`만 사용했으며 test-support native clock은 포함하지 않았다.
- 과거 학습 실행물은 그대로 `artifacts/selector-consistency-20260919-executable`,
  SHA256 `40399be6f56f62864397f4d9c4ba2894e6209bb845a4128e1116e766b183b623`이다.
  과거 plan/source/binary hash를 이번 값으로 바꾸지 않았다.
- 원본 P root와 C/S study root의12,193개 파일,5,172,587,548 bytes를
  경로/유형/길이/hash로 전후 비교했다. binary inventory가 byte-identical이며 SHA256는
  `b1a3060d9bc2d950f65ec18d604460e6b03800c6a745fd02366d64f89e2a9fa8`다.

### DERIVED_EXISTING_RAW — 동결 원자료 재채점

기존 `study-report --frozen-executable`가 native/model/step/policy/corpus/raw/teacher/
command chain을 확인했다. 기존 Rust 재채점 도구를 현재 release library로 다시 빌드해
raw tokens를 decode하고 full/body/citation 및 원본/뒤집기 pair를 교차 확인했다.
재채점의 optimizer/generation/teacher는0/0/0이며 과거 사용량은 신규 호출이 아니다.

| 모델·절대 step | train /64 | primary /512 | transfer /128 | selector /192 |
| --- | ---: | ---: | ---: | --- |
| P6144 | 58 | 425 | 79 | 2 |
| C8192 | 58 | 420 | 80 | 7 |
| S7168 | 51 | 350 | 67 | NOT_RUN_QUALITY_STOP |

각 표의 primary/transfer EOS/UTF-8 오류는0이다. C8192와 S7168은 서로 다른 예산의
종료점이다. 같은+1024의 사후 재집계에서는 C7168 primary436, S7168 primary350이며
C/D/E는155 대70, 나머지5과제는281 대280이었다. 새로운 학습 결과가 아니다.

| selector 원본/뒤집기 관측 | P6144 | C8192 |
| --- | --- | --- |
| full | 131 / 2 | 129 / 7 |
| body | 149 / 13 | 159 / 15 |
| citation | 146 / 26 | 147 / 36 |
| both-correct /192 | 0 | 0 |
| 동일 출력 /192 | 184 | 186 |

기존 연구 판정은 여전히 STUDY_INCONCLUSIVE_UNEQUAL_BUDGET, candidate=null이다.
S의 나머지1,024회·selector 패널을 실행하지 않았고 resume=false를 보존했다.

### EXECUTED_THIS_RUN — P6144 출력 동등성

기존 metadata 순서에서 bucket당2개씩16개 ID를 과거 출력 읽기 전에 등록했다.
동일 native tokenizer/weights와 정상 greedy/EOS/strict UTF-8로16회 생성하여
raw token sequence, actual, error, finish, EOS index, native prompt digest가 모두 일치했다.
실제160 tokens, EOS16, 생성 오류0, teacher0, optimizer/backward/update0/0/0이다.
RunControl 관측 구간4.2924905초, 전체 test13.68초, release test 컴파일14.64초이며
이 수치를 성능 개선 benchmark로 해석하지 않는다. 모델 오류를 고친 점수도 아니다.

누적 고유 테스트15개는 최종 PASS다. runner 명령21회, 실제 테스트 실행21건
(PASS18/FAIL3),0-test 명령1회는 통과에서 제외했다. 내부 시나리오는 앞의 T01~T12
표로 구분한다. 아래 source 단계 합계에 post-commit binary row 회귀1회와 SMALL
parity1회가 추가됐으며 TINY 사용량55/507/482는 증가하지 않았다.

### 실제 명령과 허용된 로컬 증거

공통 실행 환경: `VECLIB_MAXIMUM_THREADS=1 RAYON_NUM_THREADS=1`.
직접 TINY tests는 `cargo test --locked --offline --features accelerate,test-support`에
각 로그명의 `--test training` 또는 `--bin replica-train` 필터를 사용했다.
FX05는 `R3_FRESH_TEST_COMPACT=1`; 프로세스 검사는 `--exact --nocapture`였다.
SMALL 생성은 아래 명령 한 번뿐이다.

```sh
R3_PARITY_PARENT=artifacts/fresh-exposure-phrase-20260919/P-PHRASE \
R3_PARITY_OUTPUT=artifacts/existing-path-stabilization-20260919-parity \
cargo test --locked --offline --release --features accelerate --bin replica-train \
  training::fresh::tests::stabilization_fixed_parent_parity \
  -- --exact --ignored --nocapture --test-threads=1

artifacts/existing-path-stabilization-20260919-executable fresh study-report \
  --root artifacts/selector-consistency-20260919 \
  --frozen-executable artifacts/selector-consistency-20260919-executable
```

허용된 원자료는 다음 경로 안의 기존 연구 자료와 별도 새 관측뿐이다. 업로드하지 않았다.

- P native: `artifacts/fresh-exposure-phrase-20260919/P-PHRASE/segment-0003/final`,
  file SHA256 `c4d6989fbcaad60516053dda206caf43abc08c4b388c3f765e4c446864f60e20`.
- C native: `artifacts/selector-consistency-20260919/C-KEEP/segment-0003/final`,
  file SHA256 `d3090407de205045d3992add382124417cce287ea5080bf2eda4078bcfcee011`.
- S native: `artifacts/selector-consistency-20260919/S-SELECT/segment-0002/final`,
  file SHA256 `6c31d173c31a4f08e600f901289fa16d2779dd8639030d482a18b43b2bbbc8a0`.
- 각 root의 `corpus.r3cor`, `transfer.r3cor`, `tokenizer.r3b`, `plan.r3b`,
  `eval-STEP-PANEL.r3rows`, `*-teachers.r3rows`, `segment-*-finished.r3b`.
- 새 parity: `artifacts/existing-path-stabilization-20260919-parity/`의
  `selection.r3b`, `parity.r3rows`, `result.r3b`.
  result SHA256 `139d0e79ee876e7dea83919c7a185de0be0810b87600bbdd0a56d0c731c8acf9`.
- 증거 root: `artifacts/existing-path-stabilization-20260919-evidence/`.
  `candidate-code.patch` SHA256 `948783a5c1ef1ceb83ec39d995a908475f535ba9711e31e7f4f2dc4271a48008`;
  `parent-parity.log`, `parity-receipt-recount.log`, `closed-study-recount.log`,
  `endpoints-recount.log`, `equal-1024-recount.log`, `selector-pairs-recount.log`,
  `native-identities.log`, `originals-before/after.r3rows`, source push와 직접 회귀 로그.

STABILIZATION_REMAINING=NONE. 기존 fmt/strict clippy 경고, 독립 검토와 모델 품질
미달은 별도 상태다. 이번 예산을 더 사용하지 않는다. 추적 새 파일0; ignored 로컬
관측·실행 로그와 재사용 Rust 검사 소스만 추가했다.

### Source 단계 종료 시점의 상세 기록

R3-EXISTING-PATH-STABILIZATION-1.0. 시작 HEAD는
`0edf0327e2aadaa0e1e0e39af58c03c3b7746804`, 기준 source는
`b0e38e18cb66b87bdaf1f657b8e74332723cb996`다. main/origin을 확인했고
기존 untracked `.DS_Store`와 모든 연구 artifact를 보존했다.
Rust1.98.1, locked/offline, Accelerate, compute thread1을 사용했다.

### 코드 범위와 실제 검증

CODE_SCOPE=VERIFIED, FX06=PASS, CALL_LIFECYCLE=PASS. 정상 생성 알고리즘,
tensor/저장 형식, native 가중치, tokenizer, loss, Adam, SQLite는 변경하지 않았다.
`observe_generation`은 호출 전 원래/실제 timeout 및 cap source를 기록하고,
정확한 native timeout만 command/request로 구분한다. command cap의 반환 오류는
TIME_BUDGET만 추가한다. 다른 관측 사유를 지우지 않으며 혼합 오류·UNKNOWN과
request timeout은 계속 차단한다. 기존 비-timeout 진단 정책은 유지했다.
`fresh::run`의 pure-time 조건이나 `study_usage_bound`의 실패 차단을 완화하지 않았다.
`test-support`의 TINY clock은 실제 native timeout 검사 시간만 제어한다.
부분-token fixture의 숫자 가중치는 실제 argmax를 사용하며 SMALL에 적용되지 않는다.

| 경계 | 이번 실행 증거 |
| --- | --- |
| T01/T02/T03/T04 | 실제 zero/partial/마지막-row timeout 3가지, writer→reader→scorer→새 process, 실패 행 유지·중복0·재개 optimizer0 |
| T05 | 실제 EOS 반환 후 시간 종료 및 final sync 실패 process 회귀 |
| T06 | case_started / prompt_prepared / teacher_forward 미호출의 확정·같은 cursor 재개 |
| T07 | optimizer 반환/최종 checkpoint 전/후 시간 종료3가지, EvaluationPending 유지 |
| T08/T09 | 실제 timeout+취소/NaN/I/O, resolution sync 실패, 잘못된 call binding, native 진입 후 exit86의 UNKNOWN 차단 |
| T10 | request cap과 command cap, teacher 자동 여부, 실제 length/UTF-8 오류 구분 |
| T11/T12 | 연속2와1+1 weights/Adam/clock 일치; raw 절단/추가 byte/누락, teacher 누락, 다른 native checkpoint, policy 변경 거부 |
| 연구 사용량 | native timeout 뒤 arm의 평가만 재개하며 shared usage 검사 통과; 미완료 peer의 close와 혼합 저장 실패 뒤 실행/보고 거부 |

고유 직접 테스트14개가 최종 PASS다. 테스트 runner 명령19회 중 첫 exact 필터1회는
0-test여서 PASS에서 제외했다. 실제 테스트 실행19건=PASS16/FAIL3이다.
FAIL은 수정 전 FX06 native 재현1건과, 미실행/미완료 peer를 정상 보고로 기대했던
시험 assertion2건이다. 후자는 제품의 기존 차단이 맞았으므로 시험 기대값을 고쳤다.
이 실패들의 실제 사용량도 아래에 포함했다. 소스 변경과 무관한 전체 테스트,
전체 quick, QA32 암기, 추가 SMALL 학습은 실행하지 않았다.

| 구분 | 실제 합계 |
| --- | ---: |
| SMALL optimizer/backward/update | 0/0/0 |
| SMALL generation/teacher | 0/0 (source 검증 단계) |
| TINY optimizer | 55 |
| TINY generation 진입 / 반환 확인 | 507 / 506 |
| TINY teacher 진입 / 반환 확인 | 482 / 481 |
| scalar optimizer | 0 |
| native returned timeout | 22 (unit 관측15 + process raw7) |

process timeout raw7건 중6건은 RETURNED 해소가 확정됐고,1건은 의도한 resolution
sync 실패로 차단됐다. 재개 과정의 실패 행 재생성0. 강제 종료2건은 generation1과
teacher1의 실제 진입을 exit86으로 확인했지만 반환·최종 token/시간은 UNKNOWN이다.
이를0 사용량으로 채우거나 자동 재시도하지 않았다. 각 호출 상한512 안에서 종료했다.
실험용 TINY의 마지막 durable step은2 또는4이며 실제 SMALL endpoint가 아니다.

`cargo check`와 production `cargo build --release`는 통과했다. strict clippy는
기준 source에도 있는 `quality_recovery.rs`의 `for_kv_map` 경고1건으로 실패했다.
해당 기존 경고만 명시적으로 제외한 `-D warnings -A clippy::for_kv_map` 검사는 PASS다.
전체 fmt는 작업 전부터 여러 파일에서 실패했다. 전체 재포맷은 하지 않았고,
새 테스트 블록은 rustfmt, transformer 파일 fmt와 `git diff --check`는 PASS다.

실행 로그는 로컬 `artifacts/existing-path-stabilization-20260919-evidence/`에 보존했다.
주요 파일: `red-executed.log`, `green-process-1.log`, `study-process*.log`,
`fresh_fx*.log`, `unit-*.log`, `check.log`, `clippy*.log`, `release.log`, `fmt-*.log`.
전역 fmt/strict lint 실패를 소스 경계 회귀 실패나 품질 실패와 혼합하지 않는다.

### Source 단계 이후의 의무와 품질 판정

source 단계 종료 당시 C/S 재검산과 고정 P6144 parity16은 NOT_RUN이었다.
source commit/push 후 동결 코드로 수행한 결과는 이 절 앞부분의 후속 기록과 같다.
추가 SMALL 학습0; 모델 품질 향상은 NOT_CLAIMED다. 원래 C8192/S7168의 서로 다른
예산, S의 품질 중단/resume=false, final200 NOT_OPENED는 유지한다.
S4=NOT_PASSED, S5/S6=NOT_RUN_PREREQUISITE, GOAL1_READY=false,
GOAL1_ACCEPTED=false, INDEPENDENT_REVIEW=NOT_RUN. 추적 새 파일0.

## 2026-09-19 Selector consistency — 실행 종료, S 품질 회귀로 중단

EXECUTED_THIS_RUN / DERIVED_EXISTING_RAW. C-KEEP는 신규2048회, S-SELECT는 신규1024회를
실행했다. C는 absolute8192/BUDGET_REACHED, S는 absolute7168/QUALITY_REGRESSION_PRIMARY로
닫혔고 둘 다 resume=false다. S primary350은 부모425보다75개 낮아 중단 기준64개를
넘었다. S의 나머지1024회는 사용하지 않았으며 예산 연장·재초기화·추가 arm은 없다.
전체 판정은 **STUDY_INCONCLUSIVE_UNEQUAL_BUDGET**이다. C의 최종 개발 gate도 FAIL이다.
코드 수리 PASS와 실제 모델 품질 회복을 구분한다. candidate=null, final200 NOT_OPENED,
Goal1 S4 미통과, S5/S6 NOT_RUN_PREREQUISITE, GOAL1_READY=false, GOAL1_ACCEPTED=false다.
독립 검토/수용은 NOT_RUN이다. S의8192 및 selector192는 NOT_RUN_QUALITY_STOP이며
7168을 계획된8192 endpoint로 부르지 않는다.

### 실제 candidate와 실행 identity

기준 source `e97e2c665c5de29a1a6a84b85864016406f92cf3`, 시작 report HEAD
`e9645747a54a45cb864394840ff8c4a21ccf0ae9`다. 원래 untracked `.DS_Store`와
P6144/과거 C6144의 원문·raw·checkpoint·실패·종료 기록은 보존했다.

| 구분 | 실제 commit |
| --- | --- |
| FX04/FX05 수리 | `c7b5943fac96fa36ba2f00a887b1916b66d354a6` |
| 두 군 학습에 사용한 고정 source | `fbffc0edf943818e008a37bf9c5b5cf921fdc36a` |
| 학습 종료 후 읽기 전용 보고 수리 candidate | `b0e38e18cb66b87bdaf1f657b8e74332723cb996` |

각 source commit은 origin/main에 정상 push하고 당시 remote 전체 SHA 일치를 확인했다.
이 절은 이후의 report-only 변경이다. 실제 diff는
`git diff e97e2c665c5de29a1a6a84b85864016406f92cf3 b0e38e18cb66b87bdaf1f657b8e74332723cb996`.
학습 source 이후 코드 차이는 `src/fresh.rs`의 종료된 연구 보고/노출 digest 수리뿐이다.
학습 중 source/binary/policy/data 변경0이며 종료된 run을 새 binary로 재개하지 않았다.

학습 실행물 `artifacts/selector-consistency-20260919-executable`의 SHA256는
`40399be6f56f62864397f4d9c4ba2894e6209bb845a4128e1116e766b183b623`,
embedded source digest는 `6c01663cc979fb99072aa328210034ffa23bb6d57e10cf097e175afa6ba35575`.
보고 실행물 `artifacts/selector-consistency-20260919-report-executable`의 SHA256는
`e0bcf12e680a314d1983af6edbd145bc53f08440c522513f377d7f178d1b7f3f`,
report source digest는 `144f37c98eb47dacb10e98a0a025848263f83a90886ba5034b9b9cbe6ec7fa4b`.
학습모델은 기존9,513,408 parameter SMALL/F32/Accelerate, stable Rust1.98.1,
locked/offline, compute thread1이다. topology/tokenizer/loss/Adam은 변경하지 않았다.

### 같은 가중치에 묶인 실제 패널

normal greedy→EOS→strict UTF-8의 전체 답변 exact다. body/citation은 보조 지표다.
원 raw를 native tokenizer로 다시 해석하고 frozen expected·질문·근거·model/step·
필수 teacher·완료 receipt와 대조했다. 서로 다른 panel의 평균 CE를 섞지 않았다.
S는 품질 중단 당시7168이며 C8192와 학습량이 다르다.

| model / panel | full | body | citation | base4 | EOS | errors | teacher CE |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| P6144 train64 |58|59|61|12/16|64|0|.035539381|
| C8192 train64 |58|60|60|12/16|64|0|.040672453|
| S7168 train64 |51|58|55|11/16|64|0|.060994693|
| P6144 primary512 |425|444|455|81/128|512|0|.050290101|
| C8192 primary512 |420|450|452|80/128|512|0|.060070283|
| S7168 primary512 |350|416|391|71/128|512|0|.070221167|
| P6144 transfer128 |79|103|81|15/32|128|0|.639897667|
| C8192 transfer128 |80|94|85|16/32|128|0|.673838937|
| S7168 transfer128 |67|91|76|15/32|128|0|.606143554|
| P6144 selector192 |2|13|26|0/48|192|0|1.360439311|
| C8192 selector192 |7|15|36|0/48|192|0|1.528235119|
| S selector192 |NOT_RUN|NOT_RUN|NOT_RUN|NOT_RUN|NOT_RUN|NOT_RUN|NOT_RUN|

| bucket A–H | P primary /64 | C8192 primary /64 | S7168 primary /64 | P transfer /16 | C8192 transfer /16 | S7168 transfer /16 |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| A full copy |51|56|47|7|4|4|
| B requested field |62|61|61|8|8|8|
| C entity selection |42|44|20|4|7|0|
| D context selection |52|48|27|4|7|4|
| E current/valid time |37|37|23|9|8|4|
| F past/correction/restore |63|61|59|16|16|16|
| G missing/ambiguous |54|49|49|15|14|15|
| H causal uncertainty |64|64|64|16|16|16|

| 새 update | C screen /64 | S screen /64 | C train/primary/transfer | S train/primary/transfer |
| --- | ---: | ---: | --- | --- |
|256|53|46|NOT_RUN|NOT_RUN|
|512|54|38|NOT_RUN|NOT_RUN|
|1024|52|42|59/64,436/512,68/128|51/64,350/512,67/128|
|1536|49|NOT_RUN|NOT_RUN|NOT_RUN|
|2048|46|NOT_RUN|58/64,420/512,80/128|NOT_RUN|

모든 실행된 screen의 생성 오류는0이다. 고정 train64 teacher CE는
C(+256/512/1024/1536/2048) .068373/.044666/.029985/.029673/.040672,
S(+256/512/1024) .047106/.076076/.060995였다. S +512 screen38은 중단 조건을
충족하지 않았고 +1024 primary350에서 실제 중단됐다. C의 중간436을 최종 winner로
승격하지 않는다. 개발 gate(primary487/각bucket58/transfer116/오류0)는 유지했다.

### paired 결과와 해석 범위

아래 C는8192, S는7168이므로 두 군 간 최종 동등 예산 비교로 해석하지 않는다.

| panel / metric | P→C gain/loss | P→S gain/loss | C→S gain/loss |
| --- | --- | --- | --- |
| train full |1/1|4/11|5/12|
| primary full |33/38|25/100|31/101|
| primary body |29/23|33/61|30/64|
| primary citation |28/31|16/80|23/84|
| primary base4 |9/10|6/16|5/14|
| transfer full |7/6|1/13|2/15|
| transfer body |3/12|5/17|8/11|
| transfer citation |7/3|1/6|4/13|
| transfer base4 |2/1|1/1|2/3|

사후 읽기 전용으로 같은+1024의 기존 raw도 비교했다. C7168 primary436은
[50,64,46,56,53,59,44,64], S7168은[47,61,20,27,23,59,49,64]였다.
C→S full gain20/loss106, C/D/E155→70(-85), 다른5과제281→280(-1)이다.
같은 시점 transfer는68→67(gain13/loss14); C/D/E21→8이나 F/G/H35→47이었다.
이 관측을 중간 checkpoint 채택이나 사후 독립 test로 사용하지 않았다.
모든 bucket별 body/citation/full gain/loss, 문구·ID길이·기록수·prompt길이 및
base4 집계는 아래 accounting 로그에 보존했다.

selector 원본/뒤집기192의 pair는 P [both0, original-only131, flip-only2, neither59],
C [0,129,7,56]이었다. 동일 출력은184/192→186/192, both-correct base4는둘다0/48.
뒤집은 출력의 [선택된/다른 제공된/없는event/무인용]은 P [26,143,4,19],
C [36,144,4,8]이다. C/D/E pair 분해도 `selector-accounting.log`에 있다.
S의 counterpart는 중단 이후 생성하지 않았다. S가 새 선택 능력을 얻었다는 결론은
NOT_MEASURED이며, 확인된 것은 기존 선택 과제의 큰 하락이다.

다음 단일 미실행 가설: C/D/E selector 변형 노출 비율50%가 현재 부모의 기존
선택 규칙 보존과 충돌할 수 있다. 새 승인 연구에서 그 비율만25%와 비교하는 것이
한 후보이며 현재 실행은0이다. 이번 결과만으로 shortcut·망각·tokenizer·수식의
원인을 확정하지 않는다. S의 counterfactual 중간 측정 부재도 해석의 한계다.

### 실제 노출·예산·중단

| 항목 | C-KEEP | S-SELECT |
| --- | ---: | ---: |
| 실제 새 optimizer updates |2048|1024|
| 각 bucket draw |2048|1024|
| 총 draw / original phrase / P variant |16384 /8192 /8192|8192 /4096 /4096|
| selector-mutated draw |0|1536|
| committed input / discarded input |3511224 /3337|1756444 /0|
| actual input(미커밋 포함) |3514561|1756444|
| committed target / discarded target |252312 /218|126156 /0|
| actual target(미커밋 포함) |252530|126156|
| padding token |1427807|712420|
| generation / teacher |1792 /1984|832 /960|
| arm active seconds |2297.348288834|1131.415137793|

부모 관측 generation208/teacher208을 포함한 전체는2832/3152, SMALL updates3072다.
준비·관측·segment receipt 기준 active3489.766446919초이며 compile 시간과 다르다.
각 segment900초+cleanup120초 안에서 저장했고, C의 두 pure timeout 및 S의 한
pure timeout은 실제 새 process에서 계속됐다. actual LR bits4539475662290099561
(3e-5) 하나뿐이다. C/S gradient norm 범위 .003219–11.986928 / .082109–11.171036,
parameter delta L2 .001464–.041293 / .017495–.037246, clip 적용279/507회였다.
원래8과제 순서와 P 문구비율을 보존했고 같은 실제1024 prefix에서 case/phrase/LR,
A/B/F/G/H 입력·target 노출을 별도로 검산했다. S는 한 epoch만 실행했으므로
전체3072 flip이나 원사례2회 노출을 달성했다고 하지 않는다.

### checkpoint·인가된 로컬 증거

학습 root는 `artifacts/selector-consistency-20260919/`, 로그 root는
`artifacts/selector-consistency-20260919-evidence/`다. 로컬 읽기 전용 검토용이며
checkpoint/corpus/raw/원문은 Git에 게시하지 않는다. 실험 root에는 study.r3b,
study-ready.r3b, parent-audit.r3b, diagnostic.r3b와 각 arm의 plan/registration,
corpus.r3cor, variants.r3cor, tokenizer.r3b, initial.r3m이 있다. S에는selectors.r3cor와
selector-metadata.r3b도 있다. raw는각arm의eval-STEP-PANEL.r3rows,
필수teacher는별도*-teachers.r3rows, call prepared/resolved와 segment start/finished를
함께 확인한다. 봉인 final200이나 사용자 DB는 읽지 않았다.

| identity | C-KEEP 마지막 durable | S-SELECT 마지막 durable |
| --- | --- | --- |
| 경로(root 기준) | `C-KEEP/segment-0003/final` | `S-SELECT/segment-0002/final` |
| step / bytes |8192 /114180928|7168 /114180928|
| 실제 물리 SHA256 |`d3090407de205045d3992add382124417cce287ea5080bf2eda4078bcfcee011`|`6c31d173c31a4f08e600f901289fa16d2779dd8639030d482a18b43b2bbbc8a0`|
| weights hash |`f57b372796770767df3b13b1303b5c2ee7ef61f55a4d008497f888d761151db2`|`934260e5966f3bc701237ce3653b4cbbcc7d7ed3322932cfeb1536ca3e6e929b`|
| Adam hash |`cc93d2dfcc4b0210c5f9601ace2032e4190cd3d15e8cc5c069f4de6855c902d4`|`b5eb34b06b2bc17fcd34d1cd04900eac0383f8e4c546d5604772e3a2346c25b3`|
| state hash |`d14462f476a0f16e6ead9168000db27654b1b6f19e9ca1c39676b74795af6fb9`|`764d4fa5102bd2f182491d91c37f763187daef790a557def54b6a78f91a22d6a`|

마지막 native 내용을 직접 읽어 확인했으며 optimizer/generation/teacher 추가0이다.
저장 도중의 binding 전 TRAIN_END hash와 최종 확정 파일 hash를 혼동하지 않는다.
부모 physical/weights/Adam/tokenizer hashes는 바로 아래 S0–S3 절과identity.log에 있다.

핵심 실행 증거: `C-KEEP-segment-0000.log`부터0003, `S-SELECT-segment-0000.log`부터0002,
각`*-closed-report.log`, `study-report-repaired.log`, `accounting.log`,
`equal-1024-accounting.log`, `selector-accounting.log`, `prefix-exposure-verified.log`,
`endpoint-native-hashes.log`. 집계는 읽기 전용 Rust 도구를 사용했다.

### 수리 검증과 보고 오류의 처리

FX04는 수정 전 실제 checkpoint-timeout이 Finished가 되는 RED를 보존했다.
수정 후 optimizer-returned/저장 전/후3개 actual TINY process를 통과했고 최종
evaluate-only의 optimizer0·weights/Adam/state 불변을 확인했다. FX05는 첫/중간/마지막
미호출6개와 UNKNOWN kill/sync/cancel/binding/teacher 누락 차단을 검증했다.
실제 native 진입 후 exit86, 반환된0token 취소/length/invalid UTF-8, EOS 뒤deadline,
마지막 teacher만 남은 새 process(optimizer0,generation0,teacher1)도 검사했다.

학습 source의 `replica-check quick --fresh-selector`는관련14개 test+cargo check PASS.
별도 FX04/FX05 process 행렬 및 native first1/fresh process도 위 로그에 있다.
새 전체 suite나 추가 암기시험은 실행하지 않았다. TINY 실행 집계는optimizer86,
generation757, teacher747(진입 후 kill의 알려진 호출 포함), scalar Adam은성공2회와
비유한 입력 거부1회다. kill의 알 수 없는 token/경과시간은UNKNOWN이며0이 아니다.
기존 fixture 실패와0-test 필터 호출은 PASS 수에 넣지 않았다. 전체 fmt의 기존
차이는 남아 있으며 전체 fmt PASS를 주장하지 않는다. 기존 map-key lint만 제외한
변경 target clippy와 release build는 PASS다.

두 군 종료 뒤 최초 study-report는 `Replica binary: value bounds`로 실패했다.
기존5과제의 전체 token tree가 binary의100만 항목 상한을 넘은 보고용 집계 결함이다.
각 bounded token row의 기존 native digest를 순서대로 묶도록 수정했으며 codec
상한이나 저장 포맷은 변경하지 않았다. 학습 중 source를 바꾸거나 실패한 명령을
고치지 않았다. 보고 전용 `--frozen-executable`은 실제 보존 실행물 hash·원 policy·
native·raw·종료를 검증하고 현재 보고 source/binary identity도 별도로 출력한다.

수정 후1,310,720 token 범위/순서/변조/binary roundtrip 회귀1개 PASS,
새 process 실제 전체 study-report PASS(추가 model calls0), 다른 실행물 hash 및
명시적 과거 실행물 없는 report2개는예상대로거부됐다. 다시 실행한 보고 bytes와
실험 전체 파일 hash 목록도전후동일했다. 실패`study-report.log`, 성공
`study-report-repaired.log`/`report-pure-read-repeat.log`, 최초잘못된0-test 필터,
독립 보조검산의 물리 variant bytes 동일성 가정 실패도 보존했다. variant는변환
timestamp 때문에 물리bytes가 다를 수 있으며 기존 native reader로내용을 비교했다.
이 결함을 모델의 선택 능력 하락 원인으로 해석하지 않는다.

CODE_VERDICT=PASS, FX04/FX05=VERIFIED, RAW_SCORE_AGREEMENT=VERIFIED.
LEARNING=BOUNDED_STOP, MODEL_QUALITY_RECOVERED=false, DEVELOPMENT_GATE=FAIL.
S의 미실행 최종 obligation과 Goal1 후속은 선행 품질 미달로 실행하지 않는다.

## 2026-09-19 Selector consistency — S0–S3 관측/등록 완료, 학습 전 동결

EXECUTED_THIS_RUN: `fresh study-prepare --selector`로 같은 P6144 native weights,
Adam, tokenizer와 원문/문구 변형을 검증했다. final과 step-006144의 물리 hash는
다르지만 weights/Adam/state가 같았다. 이전 C/P endpoint를 각각704행 독립
재채점해 C425/512·44/128, P425/512·79/128과 일치했다. 기존 원문·raw·종료
기록·resume=false는 변경하지 않았다. 수리 source c7b5943fac96fa36ba2f00a887b1916b66d354a6는
정상 push 후 remote SHA 일치를 확인했다.

`fresh study-observe` 실제 SMALL generation208/teacher208, optimizer0,
29.3016185초. parity16은 기존 raw tokens/EOS/error와 모두 동일했다(정답13/16).
selector192는 정상 greedy full2/body13/citation26, EOS192/errors0, teacher CE
1.3604393112425586이었다. 원본 대응192의 full131/body149/citation146과 비교하면
both-correct0/original-only131/flipped-only2/neither59, 같은 출력184/192,
뒤집은 답변의 인용은 selected26/other-provided143/invented4/none19다.
이는 선택 변화에 대한 둔감성의 개발 관측이며 특정 수식/모델 능력의 원인 증명은 아니다.

새 학습 자료는 기존 native original+aligned phrase+selector 변형으로 등록했다.
S(S(e)) request/answer 동등성3072개, malformed6개 거부를 직접 검사했다.
등록된 C/S 원사례 순서와 표현 선택은 같고, S의 selector 변형3072/16384회 및
bucket/epoch/phrase/view 균형을 검산했다. A/B/F/G/H는 같은 token 노출이다.
예정 input C3,511,224/S3,512,352, target 각252,312; 길이를 억지로 맞추지 않았다.
실제 trace에서 committed/discarded token·표현/선택 노출을 다시 확인한다.

고정 실행물: `artifacts/selector-consistency-20260919-executable`.
binary SHA256 `40399be6f56f62864397f4d9c4ba2894e6209bb845a4128e1116e766b183b623`,
source digest `6c01663cc979fb99072aa328210034ffa23bb6d57e10cf097e175afa6ba35575`.
P Adam hash `3cb3c8843169d1b5d1f63c881dab48f648d8d6fa217b449264764dd72ad8732c`,
state hash `53448189cbd091dae95ab6778e1bfe15c07ac3a1bd01861e6db9019349844d07`,
tokenizer `ec945ee5f3cbd87992bdfa13f199de2a671b94b64337dff04d85e01d982ab9ef`.
계획/변형/관측 원자료는 `artifacts/selector-consistency-20260919/`, 실행·검사 로그는
`artifacts/selector-consistency-20260919-evidence/`에 로컬 보존한다. Git에는 올리지 않는다.

`replica-check quick --fresh-selector`는14개 관련 검사와 cargo check를 통과했다.
TINY 실제 P→C/S 등록/관측/생성/teacher/비교를 거쳤고 C 연속2 대분할1+1은
weights/Adam/clock/tokens가 일치했다. future8192 native state와2048행 실제
writer→publisher→reader도 optimizer0으로 검사했다. 최종 history reader의
checkpoint/model 및 required teacher binding 후 FX04 process3개를 다시 통과했다.
엄격 clippy의 기존 map-key lint만 제외한 변경 target clippy와 release build PASS.
전체 fmt PASS는 주장하지 않는다. 최초 quick의 random TINY model 오류도 보존하며,
정상 재개 fixture에 기존 EOS 수치 fixture를 사용했다. 오류 반환 차단은 별도 회귀다.

이 절의 품질 학습은 아직0이다. 다음 단계는 등록된 두 군 각2048회뿐이며
source/binary/data를 바꾸거나 자동 연장하지 않는다. 코드/등록 완료는 개발 품질,
S4/S5/S6/Goal1 PASS가 아니다. final200 NOT_OPENED, GOAL1_ACCEPTED=false.

## 2026-09-19 Selector consistency — 호출/평가 경계 수리

기준 source `e97e2c665c5de29a1a6a84b85864016406f92cf3`, 시작 HEAD
`e9645747a54a45cb864394840ff8c4a21ccf0ae9`, main/origin은 지정 저장소다.
시작 시 기존 untracked `.DS_Store`를 보존했다. stable Rust/Cargo1.98.1,
기존 lock/offline/accelerate/compute1을 사용했다. 이전 run은 변경하지 않았다.

EXECUTED_THIS_RUN: FX04 수정 전 실제 마지막 TINY optimizer 뒤 checkpoint 전
timeout이 Finished로 기록되는 실패를 재현했다. 수정 후 optimizer-returned,
checkpoint 전/후 세 경로 모두 EvaluationPending→새 process 평가 완료를 확인했다.
추가 optimizer0, weights/Adam/학습 counter 불변, 각24 generation/24 teacher다.
FX05는 generation 첫/중간/마지막의 case_started/prompt_prepared와 teacher의
teacher_started/forward 직전 미호출6개를 새 process로 이어갔다. 실제 미호출은
raw에 추가되지 않고 확정된 시도만 같은 cursor에서 계속됐다. 진입 후 process
종료, 해소 sync 실패, 취소+time, identity 손상, teacher 누락은 차단했다.
실제 TINY native call의0token 취소 반환·length·invalid UTF-8도 Returned이며
NotInvoked로 오인하지 않는 단위 회귀를 실행했다. 합성 logits fixture는 TINY
테스트에만 사용하며 품질모델/SMALL parent로 사용하지 않는다.

직접 명령은 `cargo test --locked --offline --features accelerate,test-support
--test training fresh_fx04_checkpoint_timeouts_keep_final_evaluation_pending --
--exact --nocapture` 및 같은 target의
`fresh_fx05_not_invoked_and_unknown_process_boundaries`다. Returned 회귀는
`--bin replica-train fresh_fx05_actual_returned_zero_length_and_utf8_are_not_no_call`.
원본 실행 로그는 로컬 `artifacts/selector-consistency-20260919-evidence/`에 보존한다.
초기 fixture hook의 profile 차이와 입력준비에서 거부된0token fixture 실패도
삭제하지 않았다. 이는 모델 품질 실패/성공으로 합산하지 않는다.

엄격 clippy는 기존 `quality_recovery`의 map key 순회 lint로 실패했다.
해당 기존 lint만 허용한 변경 target clippy는 통과했다. 전체 fmt는 기존 차이가
남아 PASS가 아니다. 새 함수/수정 평가 경계는 국소 포맷했다. SMALL 업데이트는
수리 단계0이며 parity/선택 진단/공동학습 품질은 아직 NOT_RUN이다.

P6144 실제 final SHA256는
`c4d6989fbcaad60516053dda206caf43abc08c4b388c3f765e4c446864f60e20`,
동일 단계 step 파일은 `65cf6a2f6012ee2aa1b806ddc4b0bda4423a24e70ee5bded7962d379ef7ce58d`.
경로는 `artifacts/fresh-exposure-phrase-20260919/P-PHRASE/segment-0003/`다.
물리 hash 차이를 semantic weights/Adam 차이로 해석하지 않는다. 새 연구 등록은
실제 state/Adam/tokenizer와 이전 policy를 별도로 검증해야 한다.
코드 수리는 H3/S4/Goal1 PASS가 아니며 GOAL1_ACCEPTED=false다.

## 2026-09-19 Exposure/phrase study — G4/G5 완료, 개발 품질 미달

**EXECUTED_THIS_RUN / DERIVED_CURRENT_RAW:** C-REPEAT와 P-PHRASE를 각각2048회
실제로 학습했다. 두 군 모두 absolute6144에서 BUDGET_REACHED/resume=false로 닫혔다.
판정은 **STUDY_COMPLETE_QUALITY_FAIL**이다. source 수리와 bounded 비교 실행은
완료됐지만 개발 공동 gate는 실패했다. final200 NOT_OPENED, S4 미통과,
S5/S6 NOT_RUN_PREREQUISITE, GOAL1_READY=false, GOAL1_ACCEPTED=false다.
자동 연장·세 번째 군·재초기화·추가 자료·추가 LR 탐색은 실행하지 않았다.

SOURCE_SHA=`e97e2c665c5de29a1a6a84b85864016406f92cf3`.
이 commit을 origin/main에 정상 push하고 remote 전체 SHA 일치 확인 후 학습했다.
candidate diff는 `git diff a8376ff03ced939710f946609170d2fb6b798452 e97e2c665c5de29a1a6a84b85864016406f92cf3`.
실행 source digest와 고정 binary는 아래 G0–G3 identity 그대로이며 학습 도중
source/test/binary/tokenizer/data/policy 변경0이다. source 변경은 기존 fresh/trainer/
RunControl/publisher와 직접 tests/checker에 한정했다. 이 절은 별도 report-only commit이다.
독립 수용 검토는 NOT_RUN이며 구현자의 검산을 독립 승인으로 부르지 않는다.

### 같은 endpoint의 정상 생성 결과

normal logits→greedy→EOS→strict UTF-8, 전체 답변과 인용 exact다.
각 모델의 train/primary/transfer가 같은 step·weights에 묶였고 raw 재채점이 일치했다.
본문/인용 지표는 보조 지표이며 전체 정답을 대체하지 않는다. generation errors0은
EOS/생성/strict decode 오류0이라는 뜻이다. 틀린 본문·잘못된 인용은 오답에 포함한다.

| model / panel | full | body | citation | base4 | EOS | errors | teacher CE |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| parent4096 train64 |55|56|58|9/16|64|0|.074903241|
| C6144 train64 |58|59|62|11/16|64|0|.038898884|
| P6144 train64 |58|59|61|12/16|64|0|.035539381|
| parent4096 primary512 |392|419|439|65/128|512|0|.081706510|
| C6144 primary512 |425|440|454|74/128|512|0|.051300537|
| P6144 primary512 |425|444|455|81/128|512|0|.050290101|
| parent4096 transfer128 |40|73|51|6/32|128|0|.744842450|
| C6144 transfer128 |44|71|57|7/32|128|0|.828831871|
| P6144 transfer128 |79|103|81|15/32|128|0|.639897667|

| bucket A–H | parent primary /64 | C primary /64 | P primary /64 | parent transfer /16 | C transfer /16 | P transfer /16 |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| A full copy |43|58|51|4|1|7|
| B requested field |60|64|62|0|5|8|
| C entity selection |36|42|42|5|5|4|
| D context selection |46|51|52|4|5|4|
| E current/valid time |34|42|37|8|8|9|
| F past/correction/restore |60|58|63|8|9|16|
| G missing/ambiguous |49|46|54|11|11|15|
| H causal uncertainty |64|64|64|0|0|16|

각view를 독립base로 재해석하지 않았다. 최종 gate는 primary>=487/512,
각bucket>=58/64, transfer>=116/128이므로 두 군 모두 FAIL, candidate=null이다.
중간+1024의 높은 transfer나 익숙한 질문 진단80/128을 최종 점수로 대체하지 않았다.

| 새 update | C screen /64 | P screen /64 | C train/primary/transfer | P train/primary/transfer |
| --- | ---: | ---: | --- | --- |
|256|50|51|NOT_RUN|NOT_RUN|
|512|54|52|NOT_RUN|NOT_RUN|
|1024|50|47|56/64,409/512,54/128|58/64,415/512,76/128|
|1536|48|48|NOT_RUN|NOT_RUN|
|2048|47|48|58/64,425/512,44/128|58/64,425/512,79/128|

모든 screen 생성 오류0이고 필수 teacher64가 완료됐다. 고정 train64 CE 곡선
256/512/1024/1536/2048은 C .080706/.054336/.049303/.065665/.038899,
P .069046/.049696/.034915/.078305/.035539다. 서로 다른 panel의 CE를 섞지 않았다.
계약의 연속/급격한 screen 회귀 중단은 발동하지 않았다.

### paired 변화와 범위

| panel / metric | parent→C gain/loss | parent→P gain/loss | C→P gain/loss |
| --- | --- | --- | --- |
| train full |4/1|4/1|1/1|
| primary full |58/25|56/23|18/18|
| primary body |41/20|38/13|19/15|
| primary citation |40/25|45/29|13/12|
| primary base4 |16/7|20/4|9/2|
| transfer full |10/6|41/2|39/4|
| transfer body |7/9|31/1|35/3|
| transfer citation |12/6|33/3|28/4|
| transfer base4 |3/2|9/0|9/1|

최종 parent/C/P의 primary evidence0/1/2/3별 full은
24/24,115/148,193/276,60/64 → 24/24,134/148,209/276,58/64 →
24/24,126/148,212/276,63/64다. transfer는
8/8,4/20,15/60,13/40 → 8/8,5/20,17/60,14/40 → 8/8,19/20,32/60,20/40이다.
transfer full/current/past/cause 표현별 parent는4/16,28/88,8/8,0/16;
C는1/16,38/88,5/8,0/16; P는7/16,48/88,8/8,16/16이다.
ID길이·prompt길이·task·기록수·wording별 분모와 full/body/citation paired 통계는
아래 익명 accounting 로그에 전부 보존했다. 같은 frozen 입력/정답/근거와 실제 tokens를
다시 확인했으며 새 model/teacher/optimizer 호출0이다.

관측 결론: 원래 질문 반복만으로 primary는+33이나 transfer는+4에 그쳤다.
질문 표현을 늘린 P는 C와 primary 총점이 같고 transfer가+35였다. 이는 이 고정
조건에서의 표현 범위 개입 효과이며 범용 한국어 일반화·tokenizer 원인 증명이 아니다.
P의 한 기록 transfer19/20에 비해 두 기록32/60, 세 기록20/40과 낮은 C/D bucket이
남아 있다. 길이·기록조합 등의 결합이 있으므로 기록수 단독 인과로 단정하지 않는다.
다음 한 질문은 **질문 표현을 익숙하게 고정해도 두 근거의 대상·맥락·현재 관계가
바뀔 때 선택 결과가 정확히 따라 바뀌는가**다. 새 실험은 실행하지 않았다.

### 실제 사용량·저장·재시작

동일한 명령을 arm마다4개 새 process로 순차 실행했다:
`VECLIB_MAXIMUM_THREADS=1 RAYON_NUM_THREADS=1 artifacts/fresh-exposure-phrase-20260919-executable fresh run --root artifacts/fresh-exposure-phrase-20260919/C-REPEAT`
및 같은 명령의 P-PHRASE 경로다. 각군 첫1회 저장 후 재개와 순수 TIME_BUDGET2회를
포함하며 실패 재시도는 없다. 이번 SMALL의 시간 종료는 학습 경계에서 발생했다.
마지막-step 평가전용 continuation 자체는 G1의 실제 TINY process 회귀에서 검증했다.

| arm/segment | durable absolute step | gen | teacher | command seconds | stop |
| --- | ---: | ---: | ---: | ---: | --- |
|C/0000|4097|0|0|3.973067833|first update saved|
|C/0001|5084|128|256|901.917712708|TIME_BUDGET|
|C/0002|6005|768|832|901.570672584|TIME_BUDGET|
|C/0003|6144|704|704|195.641551208|BUDGET_REACHED|
|P/0000|4097|0|0|4.393697959|first update saved|
|P/0001|5058|128|256|901.032136084|TIME_BUDGET|
|P/0002|5956|768|832|901.265654166|TIME_BUDGET|
|P/0003|6144|704|704|238.290653000|BUDGET_REACHED|

| new usage | C | P |
| --- | ---: | ---: |
| optimizer updates |2048|2048|
| committed input |3384520|3511224|
| discarded input |3434|1780|
| actual input |3387954|3513004|
| committed target |252312|252312|
| discarded target |270|130|
| actual target |252582|252442|
| actual padding |1467742|1429636|
| original / variant draws |16384 / 0|8192 / 8192|
| generation / own teacher |1600 / 1792|1600 / 1792|
| arm command seconds |2003.103004333|2044.982141209|

전체4,096 update의 실제 trace를 검증했다. 각 bucket2,048 draws이며 C 각case 원형2회,
P 각case 원형1/새질문1회다. case order hash는 양군 동일
`641eb71b161a8af5176ebfc66dc9cc5f7720ae8037589f241893376d0d79349e`,
target order는 `875374ee96f2e1d0231896a6ff80c4b520e12dfef683e46447371ccf29ae5e8d`다.
모든 actual LR bits는4539475662290099561(3e-5)이다. Adam은 부모 moments/clock4096을
계승했다. P committed input은 C보다126704 많아 동일 FLOPs 비교라고 하지 않는다.
global grad norm C .005020–8.798589/P .006667–10.342487, clip 적용378/600 updates,
parameter delta L2 C .002419–.039797/P .002980–.036053이다.
sampled peak RSS는 C2160544/P3079376KiB이며 순간 전체 peak 보장은 아니다.

SMALL 합계 updates4096, committed input6895744/target504624,
discarded input5214/target400, actual input6900958/target505024, padding2897378이다.
부모의 과거 지출을 새 학습으로 합산하지 않는다. G2 포함 generation3344,
자체 teacher3728, 등록 prepare/observation/segments 시간4086.935602001초다.
컴파일과 사후 읽기 전용 집계 시간은 이 모델 실행 측정에 포함하지 않는다.
G1 fixture generation704/teacher698까지 보수적으로 합하면4048/4426으로
공통 상한4096/6000 안이다. TINY optimizer60/scalar1도 각128/32 안이다.
UNKNOWN 호출0, 사용자 취소0, 새 수치/자료/저장 오류0. command overrun은 최대
약1.92초로 cleanup120초 안이며 후속 optimizer 예산을 늘리지 않았다.

### 최종 증거와 판정

pure `fresh study-report --root artifacts/fresh-exposure-phrase-20260919`가 성공했다.
현재 arm 평가 raw3200행의 원자료·token/EOS·점수·model/step/policy·teacher를
재검산했고 요약/분모가 일치했다. 부모2496행 검증과 parent 파일 inventory 보존도
확인했다. source/기존 dirty(.DS_Store)와 부모 모델·Adam·corpus·실패는 보존했다.

| artifact | physical SHA256 / weights |
| --- | --- |
| C `segment-0003/final` bytes | `51dd03a676fcc739c1da10c3b273696fac48d6c62f8546ffe3bc98696a1603b4` |
| C model weights | `3c16b4112379025bb13959ca10c3d3a0166e32c2fef5a2985d1ea802f112d568` |
| P `segment-0003/final` bytes | `c4d6989fbcaad60516053dda206caf43abc08c4b388c3f765e4c446864f60e20` |
| P model weights | `e759531432a9c954c2b50f7c4c434a87d6de01e1c9675a60d49025d3199158f2` |

인가된 root `/Users/seo/Projects/Replica-v3/artifacts/fresh-exposure-phrase-20260919/`의
C-REPEAT/P-PHRASE 하위 `segment-0003/final`, `step-006144`, `updates.r3rows`,
`train-control.r3b`, 각segment 시작/종료, `eval-*.r3rows/.r3b`, `teacher-*.r3b`,
원 corpus/tokenizer/plan 및 G2 raw를 읽기 전용 검토할 수 있다. old parent root는 그대로다.
콘솔 실행 근거 `/tmp/r3-phrase-C-segment-0000.log`~`0003.log`,
`/tmp/r3-phrase-P-segment-0000.log`~`0003.log`, `/tmp/r3-phrase-study-report.log`.
세부 계층/paired 집계 `/tmp/r3-phrase-accounting-v2.log`, Rust 읽기 도구
`/tmp/r3_phrase_summary.rs`(SHA359e753f29047b76e3244bb081e5171fc96fcb8f412780e08503601720410f71).
이 보조 집계의 첫 실행은 malformed citation을 오답 대신 오류로 반환해 중단됐고,
기존 scorer와 같이 오답으로 집계한 두 번째 읽기가 완료됐다. 첫 로그도 보존했다.
모델 실행/점수/raw를 바꾼 것이 아니며 추가 generation/teacher/optimizer0이다.

CODE_VERDICT=PASS_BOUNDARIES, FX01/02/03=PASS, RAW_RESCORE=VERIFIED,
SAME_WEIGHT_PARITY=16/16, WORDING_DIAGNOSTIC=80/128,
STUDY_EFFECT=OBSERVED_TRANSFER_GAIN_WITH_PRIMARY_TIE, DEV_JOINT=FAIL,
FINAL200=NOT_OPENED, S4=NOT_PASSED, S5/S6=NOT_RUN_PREREQUISITE,
GOAL1_READY=false, GOAL1_ACCEPTED=false, INDEPENDENT_REVIEW=NOT_RUN.
고유 직접tests17 PASS와 fmt/Clippy의 기존 범위 제한은 아래 G0–G3 기록과 같다.
새 영구 source/test/doc 파일0; ignored native 실험 산출물과 로컬 분석 도구만 생성했다.

## 2026-09-19 Exposure/phrase study — G0–G3 검증, 학습 전 기록

R3-FRESH-EXPOSURE-PHRASE-1.0. 기준 source d126aff35d85cebd1bf2b40bc6bc0a5943a3084b,
보고서 a8376ff03ced939710f946609170d2fb6b798452 이후 기존4096 run을 그대로 보존했다.
이 절은 **EXECUTED_THIS_RUN / DERIVED_CURRENT_RAW**이며 아래 baseline 기록을
소급 수정하지 않는다. 이 시점 새 SMALL updates0, generation144, 자체 teacher144다.
C/P 학습 및 개발 합격, final200/S4/S5/S6/Goal1은 아직 NOT_RUN이다.

FX01: 기존 publisher의 공개 후 file/directory sync를 거친 뒤에만 pending을 해제한다.
공개된 finished가 있어도 pending/미확정이면 새 process의 재개를 거부한다.
FX02: 정상 Generated 반환과 command 중단을 row version2에서 구분하고, 필수 teacher를
별도 prefix로 보존한다. 정상 length는 완료된 생성이지만 strict 정답은 아니다.
FX03: 마지막 optimizer 이후 EvaluationPending을 같은 평가 경로로 재개하며,
평가만 남으면 trainer/Adam 호출0, native bytes 불변, 완료 prefix 재생성0이다.
단일 동기 tensor/fsync의 강제 선점이나 여러 파일의 원자적 transaction을 주장하지 않는다.

### 직접 검증

`target/debug/replica-check --output artifacts/fresh-phrase-check-20260919 quick --fresh`
고유16 tests PASS. 추가 EOS deadline+sync 실제 process 회귀1개 PASS: 고유17개다.
fresh fork 연속2 대1+새 process1을 최종 source에서 다시 실행해 PASS했다.
수정 전 FX01/03 직접 process 시험은 각각 실패했고 수정 후 통과했다.
초기 FX02 RED는 fixture가 정상 EOS를 반환하지 않아 증거로 사용하지 않는다.
수정한 수치 TINY fixture는 실제 logits→greedy EOS, timeout 직후 teacher 미실행,
binary writer/reader strict 재채점을 검증했다. SMALL 모델 출력 대체에는 사용하지 않는다.
정답EOS/오답EOS/length/UTF-8 오류/EOS 전 timeout/정상EOS 뒤 command timeout의
6행 독립 판정도 통과했다. 마지막 평가 첫/중간/끝/summary 경계와 UNKNOWN 호출,
pending 실패/잘림/공개 후 sync 실패에서 새 process 차단을 확인했다.

이 작업에서 실행한 fixture의 누적 optimizer는 TINY60/scalar1이다(실패·재실행 포함).
TINY generation704/teacher698은 process fixture와 단위 시험 호출에서 별도 집계했다.
이전 baseline의 시험 수를 합산하지 않았다. quick16 이후 추가 회귀 및 최종 fork 재실행을
포함한 수치이며, 품질 모델의 학습 횟수로 표시하지 않는다.

Rust/Cargo1.98.1, locked/offline, CPU F32/Accelerate/thread1. 릴리스 빌드 PASS.
변경 fresh.rs의 rustfmt 및 diff whitespace 검사 PASS. 전체 fmt는 기존 다른 파일의
형식 차이 때문에 FAIL이며 무관한 재포맷은 하지 않았다. Clippy는 기존 미수정
for_kv_map 경고만 허용하여 `-D warnings -A clippy::for_kv_map`으로 PASS했다.
실제 새 CLI의 study-prepare/study-observe와 --help를 실행했다.

### 부모와 표현 진단

read-only 검증이 기존2496 raw의 native corpus·prompt·model·EOS·strict text·분모와
저장 요약을 재계산하여 일치를 확인했다. final/step-004096의 weights/Adam/state가
동일하고 기존 부모 파일 inventory도 변경되지 않았다. 새 binary의 bucket별2개,
총16개 정상 greedy가 과거 raw tokens/actual/error/finish/EOS와 전부 일치했다.
16개 정답률12/16은 표본 확인일 뿐 전체 품질 추정이 아니다.

질문 suffix만 익숙한 train 표현으로 바꾼128개의 정상 생성은80/128, errors0,
EOS128이었다. 원 transfer40/128에 대해 gain42/loss2, 본문97/128·인용86/128,
base4=16/32, buckets=[4,8,6,8,8,16,14,16]이다. CE .5012038153372135.
이는 FAMILIAR_WORDING_DIAGNOSTIC이며 정상 transfer 점수나 제품 전처리가 아니다.
evidence/ID/time/status/order/answer를 유지하고 request-only resolver로 의도를 확인했다.
prompt의 불완전 UTF-8 piece 출현은 train0, primary0, 원 transfer2016, 진단0;
transfer prompt tokens28412→25916이다. 표현·길이·토큰 조합이 함께 달라지므로
byte fragment 단독 원인으로 단정하지 않는다. 관측 command13.808689584초,
prepare25.041766875초이며 컴파일 시간은 모델 평가 시간에 합산하지 않았다.

### 등록 및 실행 identity

두 arm은 같은 부모 weights/Adam/clock, 원래 case/target 순서로 등록됐다.
C 예정 committed input3384520, P3511224; target는 각각252312다.
이는 계획값이며 실행값이 아니다. 각 case 추가2노출, P는 original/variant 각1회,
각 bucket/epoch에서 두 질문형식512/512를 전수 확인했다. 정확한 heldout suffix는
train에 넣지 않았다. LR3e-5 constant, absolute4096→6144, 각2048 상한이다.

| identity | SHA256 |
| --- | --- |
| frozen executable | `ede5e6fe6fa431b1f9fd38923681b4ad3bd89291fc0b7bd5b9a72185436b9d02` |
| execution source digest | `5d4119ac19a1c367ea37804bca74f72b1fd6e6a409e4c5ac7b7b5d4a65dd1cb7` |
| study.r3b | `4079a5d567898004f733520f1a9e5f1aa45c20fc1f1484537c5642bc4e426c44` |
| C plan | `98269ef94c501a8a1a3eb5f9d9094aa311e642519addfbf450bc6711feafc5e0` |
| P plan | `699018f296714b257ed4cf881b8b67967d38a21d941cc421b06d7259ab5f629e` |
| inherited Adam | `15fb9f84241409c9cb940eda562fb6ba61dfa5dea6eaeda2946cbd1583041132` |

허용된 읽기 전용 증거 root는 `artifacts/fresh-exposure-phrase-20260919/`다.
`parent-audit.r3b`, `study.r3b`, `study-ready.r3b`, `observation-*.r3b`,
`eval-4096-parity*`, `eval-4096-familiar-wording*`, 각 arm의 plan/registration,
native corpus/tokenizer/initial 및 P question-variants가 해당한다.
원본 부모는 `artifacts/fresh-joint-20260919/`이며 아래 기존 hash를 유지한다.
고정 실행파일은 `artifacts/fresh-exposure-phrase-20260919-executable`이다.
콘솔 근거 `/tmp/r3-phrase-prepare.log`, `/tmp/r3-phrase-observe.log`,
직접 회귀 `/tmp/r3-phrase-process-red.log`, `process-green.log`, `eos-sync.log`,
`fork-final.log`(마지막 세 파일도 동일 r3-phrase- 접두어)을 보존한다.
원자료·corpus·모델·임시 도구는 commit 대상이 아니다.

## 2026-09-19 Fresh joint baseline — F4/F5 예산 종료

**EXECUTED_THIS_RUN:** 새 random-init SMALL을 4,096회 학습하고 정상 예산 종료했다.
dev392/512(76.5625%), transfer40/128(31.25%)로 개발 기준에 미달했다.
코드 검증과 학습 실행은 완료됐지만 모델 품질·S4·Goal1은 통과하지 않았다.
최종200은 NOT_OPENED, S5/S6는 선행 품질 미달로 NOT_RUN이다. 추가 학습·재초기화·
LR 탐색·기존 자료 복원은 하지 않았다. 아래 F0–F3는 학습 전 시점의 기록이다.

CODE_SHA=`d126aff35d85cebd1bf2b40bc6bc0a5943a3084b`.
reference1885626a4f84ec79137e4e79414269a019de7e3f에서 source/tests/계획을 구현했고,
정상 push 및 원격 전체 SHA 일치를 확인한 뒤 고정 실행파일로 학습했다.
이번 결과는 별도 report-only commit이다(REPORT_SHA는 이 절을 추가한 commit).
실행 중 source/binary/data/config는 변경하지 않았다. 실행 source digest는
`dc6b99f9c4542aa1ddfcc066e18fda7ae246c2715715d990284294122e22b849`다.

### 실제 실행과 저장

명령은 6개의 순차 새 process에서 동일했다.
`VECLIB_MAXIMUM_THREADS=1 RAYON_NUM_THREADS=1 artifacts/fresh-joint-20260919-executable fresh run --root artifacts/fresh-joint-20260919`.
첫1회 저장 후 새 process에서 재개했고, 이후 순수 시간 종료4회만 원래 예산 안에서
재개했다. 중간 학습의 weights/Adam/LR clock/sampler를 유지했다. 마지막은
BUDGET_REACHED, resume=false이며 실행 중인 학습 process는 없다.

| segment | 마지막 durable step | 누적 실제 input | 누적 committed target | generation | 자체 teacher | command 초 | 종료 |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | --- |
|0000|1|1727|124|128|192|43.728064250|첫1 저장|
|0001|966|1598279|118959|128|192|901.237864625|TIME_BUDGET|
|0002|1905|3151268|234813|640|768|901.629287459|TIME_BUDGET|
|0003|2817|4660621|347065|768|896|901.765446459|TIME_BUDGET|
|0004|3783|6259578|466048|64|192|901.624917542|TIME_BUDGET|
|0005|4096|6775700|504624|768|832|348.059778000|BUDGET_REACHED|

SMALL optimizer4096, generation2496, 자체 teacher3072. 앞서 실행한 TINY optimizer16,
scalar Adam2, 추가 TINY generation2는 별도다. 전체 command 시간3998.045358335초는
평가·저장·cleanup을 포함하며 순수 optimizer 속도가 아니다. 컴파일/fixture/준비 시간은
여기에 합산하지 않았다. 관측 sampled peak RSS1,423,904KiB이며 전체 순간 peak의 보장은
아니다. 각900초 경계의 약1.2–1.8초 초과는 허용 cleanup120초 안이었다.

4096개의 raw update를 전수 확인했다. committed input6,769,040/target504,624,
draw32,768, bucket별4,096 draws, bucket별256 base/1,024 view 각각4회 노출이다.
시간 종료 직전 계산됐으나 optimizer에 반영되지 않은4개 microbatch(32 draws)를
제외하지 않는다. 알려진 추가 input6,660과 동일한 재개 draw에서 유도한 target513을
포함하면 실제 input6,775,700/target505,137이다. target513은
DERIVED_FROM_CURRENT_RAW이며 checkpoint의 committed target와 구분한다.
실제 forward draws32,800, bucket별4,100이다. 숨은 retry/5번째 epoch는 없다.
첫 update CE6.39822626, LR0.00000234375, global grad15.15702471,
parameter delta0.00721444; 마지막 batch CE0.007974374108016491, LR0.00003.
committed padding2,932,744 tokens는 input 예산과 구분한다. global grad norm 범위
0.01612817–15.38915748, clip 적용3,231 updates, 마지막 grad0.60919830,
clip1.0/parameter delta0.01930301이다. task별 token 가중 누적 관측은 다음과 같다.
각 step에서 실제 기록한 값의 집계이며 최종 모델의 heldout token 정확도가 아니다.

| bucket | committed input | target | 정답 teacher tokens | 전체 학습 구간 token 가중 CE |
| --- | ---: | ---: | ---: | ---: |
|A|703552|119968|93219|.664097|
|B|638864|61152|48194|.614655|
|C|905312|61088|46534|.700182|
|D|909920|60960|48129|.615994|
|E|904880|61104|47587|.650369|
|F|1212560|69408|55723|.582404|
|G|610784|34080|32320|.190942|
|H|883168|36864|36422|.067881|

실제 tokenizer merges298; 단독 decode bytes가 유효 UTF-8이 아닌 piece230개다.
고정 source의 준비된 prompt/answer에서 그 piece 출현 수는 train0/0, primary0/0,
transfer2016/0이다. 새 표현의 byte fragment 사용과 낮은 전이 점수는 함께 관측됐지만
인과 증명이 아니다. tokenizer/decoding 정책을 수정하거나 자료를 추가하지 않았다.

### 동일 패널의 실제 품질

모든 점수는 normal greedy→strict UTF-8→정상 EOS→전체 답변·인용 exact다.
teacher 접두어나 정답 selector를 자유생성에 사용하지 않았다.

| step | train64 | screen64 | primary512 | transfer128 |
| --- | ---: | ---: | ---: | ---: |
|0|0|0|NOT_RUN|NOT_RUN|
|128|NOT_RUN|9|NOT_RUN|NOT_RUN|
|512|NOT_RUN|11|NOT_RUN|NOT_RUN|
|1024|17|13|96|NOT_RUN|
|2048|28|20|117|16|
|3072|NOT_RUN|45|NOT_RUN|NOT_RUN|
|4096|55|47|392|40|

초기 random 모델의128개 출력은 generation error/EOS 실패로 분모에 포함했다.
128 이후 모든 측정 panel과 최종 동일 checkpoint의 출력은 오류0, 정상 EOS였다.
accepted invalid citation0은 인용이 틀린 답을 exact로 인정하지 않았다는 뜻이다.
인용 오류 자체가0이었다는 뜻은 아니다.

| 4096 bucket | train /8 | screen /8 | primary /64 | transfer /16 |
| --- | ---: | ---: | ---: | ---: |
|A 전체 원문|7|7|43|4|
|B 요청 필드|8|8|60|0|
|C 대상 선택|6|2|36|5|
|D 맥락 선택|7|6|46|4|
|E 현재/유효시간|4|4|34|8|
|F 과거/정정/복원|8|8|60|8|
|G 근거 없음/모호함|7|4|49|11|
|H 인과 유보|8|8|64|0|

primary 기준487/512 및 각58/64, transfer 기준116/128을 모두 적용했고 미달했다.
2048에서도 통과하지 않았다. train은55/64로 자기 자료의 선택/복사 오류도 남아 있다.
최종 base4는 train9/16, screen9/16, primary65/128, transfer6/32다.
primary의 본문 exact419/512·인용 exact439/512와 transfer의 본문73/128·인용51/128은
보조 지표다. 본문 지표는 인용 suffix 앞 문자열 비교이며 독립 field/entity 완전 정답률로
해석하지 않는다. strict 전체 정답392/40을 이 지표로 대체하지 않았다.
primary 오답120개 중27개는 본문이 맞고 인용만 틀렸고93개는 본문/형식 오류를 포함한다.
transfer 오답88개는 인용만33개, 본문/형식55개다.

primary의 ID 길이1~8자리별 exact는 각각53/64,48/64,54/64,46/64,54/64,42/64,
54/64,41/64다. transfer1~4자리는15/32,9/32,11/32,5/32다.
primary의 evidence0/1/2/3개별24/24,115/148,193/276,60/64;
transfer는8/8,4/20,15/60,13/40이다. prompt 길이64-token 구간1/2/3/4별
primary24/24,148/193,168/239,52/56; transfer8/8,4/20,15/60,13/40이다.
전이의 새 phrasing template별 full4/16, current28/88, past8/8, cause0/16이다.
표현과 일부 record-count 조합이 함께 달라져 둘의 인과 효과를 분리한 대조는 아니다.

고정 train64 teacher CE(step0/512/1024/1536/2048/2560/3072/3584/4096)는
6.406142/.688201/.514079/.482159/.328692/.215014/.079570/.070271/.074903이다.
최종 primary CE.081707 및 transfer CE.744842는 다른 패널로서 같은 곡선에 섞지 않는다.
screen 연속 회귀·1024 무학습·2048 무전이 중단 조건은 발동하지 않았고 최종 예산으로 닫았다.

다음 한 질문은 **같은 사실·선택 관계에서 질문 표현만 바꿀 때 응답과 인용이 얼마나
달라지는가**다. transfer의 낮은 점수는 이를 조사할 근거지만 현재 자료는 표현/조합을
완전히 분리하지 않는다. 추가 대조는 NOT_RUN, 추가 학습0이며 별도 승인이 필요하다.
이번 baseline은 여러 조건을 새로 정한 것이므로 이전 실패 원인이나 저장 형식의 효과를
증명하지 않는다. 학습 신호는 있지만 범용 지능·S4 수용 수준의 성능은 확인되지 않았다.

### 검산·hash·인가된 로컬 원자료

별도 Rust 읽기 도구가 native corpus의 ID/질문/evidence/정답과 raw2496개를 대조했다.
실제 tokenizer의 tokens→text/EOS/error/exact 재계산은 저장 점수와 전부 일치했다.
93개 파일의 길이/경로/hash inventory는 전후 동일:
`23e225a69d977828c2ccbd5eb53578711f086dac9a7836b615c037812966f488`.
검산 자체 model/optimizer 호출0이며, 외부 독립 검토 승인을 의미하지 않는다.

기본 경로는 `/Users/seo/Projects/Replica-v3/artifacts/fresh-joint-20260919/`다.
`eval-STEP-PANEL.r3rows`와 대응`.r3b`, `teacher-STEP.r3b`, `segment-000N/updates.r3rows`,
`train-control.r3b` 및 checkpoint를 읽기 전용 검토에 사용할 수 있다.
마지막 durable는 `segment-0005/final`, 최종 평가 checkpoint는
`segment-0005/step-004096`이며 동일 step/weights다. 물리 파일 hash와 모델 hash는 다르다.

| identity | SHA256 |
| --- | --- |
| 실행파일 | `c5cc6702cbd56c940e8ba00664575465bcd89272428aa2e6220fcbe8b58752ec` |
| plan.r3b 실제 bytes | `17fcb4055f53a0c005411ff9322e29095c98ae75164544878acce7d64509f501` |
| corpus.r3cor 실제 bytes | `108171c2ffad62afc9b0f09a52070871a4d3287017676208e9f43a88631fe04f` |
| transfer.r3cor 실제 bytes | `e7878e78e19b55a42565e64af1107548651596303efe132e8b728cba32460c29` |
| metadata.r3b | `adc193231e59774e97576dc5386b02024c1f4ea226cfa5c17e00515f776a96b3` |
| sampler train-order | `d064f83d415dba3d6035d53606b36994574d6c82b4ab241ddc7b6251d1c2f1f8` |
| train config canonical R3BIN | `f225c59c70696e9ab9e85012323ae5852c5172ed2e5ad3a98b60e5e5a6a68299` |
| final 실제 bytes | `0478828fc2da96655f7ebc7ff34595132149b7f54e9ff1cc467bf031f5b6b512` |
| step-004096 실제 bytes | `7dd7705bc07826c4c18bb4f4c31eeb769f3892cf40ff0328a4f68e7e90c3da81` |
| 최종 모델 weights | `2da022b98bed607c1c6c54e4dd5482a3f631d1561fec87e965b3e6744a59cd98` |

typed plan/tokenizer/initial weights/train semantic hash는 바로 아래 준비 기록과 같다.
로컬 실행 로그 `/tmp/r3-fresh-segment-0000.log`~`0005.log`, pure report
`/tmp/r3-fresh-report.log`, 별도 Rust 검산 `/tmp/r3-fresh-recount.log`를 보존했다.
후속 token/gradient 집계는 `/tmp/r3-fresh-accounting.log`이며 다시 동일 파일 inventory를
확인했다. 임시 Rust 집계기 재컴파일의 첫 명령은 외부 crate `--extern` 인자 누락으로
실패했고 기존 locked rlib를 명시한 후 통과했다. 모델/optimizer 호출은0이었다.
로그/모델/원자료는 Git에 게시하지 않는다. 새 영구 source 파일은 `src/fresh.rs` 하나다.

RESET_STATE_SOURCE=locally_checked; CURRENT_MODEL_STATUS=EVALUATED;
TRAINED_FROM_SCRATCH=true; CODE_VERDICT=PASS; TRAINING_EXECUTION=COMPLETE;
MODEL_QUALITY_PASS=false; S4=NOT_ACCEPTED; S5/S6=NOT_RUN;
GOAL1_READY=false; GOAL1_ACCEPTED=false; INDEPENDENT_REVIEW=PENDING.
JSON_DIRECT_PATH=NONE; SQLITE_ACTUALLY_USED=NONE_IN_THIS_FRESH_TRAIN_EVAL;
제품 기억 SQLite는 RETAINED이며 실행하지 않았다. 저장 형식 변경0.

## 2026-09-19 Fresh joint baseline — F0–F3

R3-FRESH-JOINT-BASELINE-1.0. 시작 source1885626a4f84ec79137e4e79414269a019de7e3f,
main/origin seoyd/replica-v3, Rust1.98.1, locked/offline Accelerate CPU F32 thread1.
시작 시 artifacts/docs/logs/기본 운영 기억 경로가 없음을 로컬 확인했다. 추가 삭제나
과거 모델 복구는 하지 않았다. `.DS_Store`는 그대로 보존했다.

**EXECUTED_THIS_RUN:** 기존 trainer에 명시적인 fresh binary plan/균형 sampler를
연결했다. forward/backward/Adam loop는 복제하지 않았다. native checkpoint의 기존
execution-policy binding에 plan/order hash를 묶는다. 일반 default resume로의 우회는
거부한다. 별도 training-only `src/fresh.rs`에서 여덟 과제 corpus, 독립 request-only
resolver, 준비/실행/평가를 구성한다. 원문 full/짧은 값/대상/맥락/현재/과거·복원/
없음·모호함/인과 유보를 처음부터 함께 학습한다. 제품 inference에 oracle이 없다.
R3BIN/R3CORP/R3MODEL 형식, tokenizer 알고리즘, SMALL 구조와 SQLite는 변경하지 않았다.

`replica-check --output artifacts/fresh-joint-check-final-20260919 quick --fresh`:
관련 고유 회귀9개 PASS와 컴파일 PASS. tokenizer raw/역할/숫자, scalar CE와 gradient,
shift/mask/EOS, 다른 길이의 target 가중 gradient 누적, cached/uncached255/256/257,
4epoch sampler 전수, 실제 native 저장과 별도 process2 vs1+1 및 정책 누락 거부,
worker 오류·취소를 검사했다. 예정4096 step metadata는 합성 count이며 optimizer0.
추가 평가-row 검사는 같은 TINY checkpoint의 writer→reader→fresh-process 재독과
파일 불변을 확인했다. source 마지막 변경은 부분 평가의 완료 flag를 성공 후에만
기록하도록 맞췄으며 release 컴파일을 통과했다.

실패도 보존: 첫 TINY fixture의 context 변경은 지원 profile 검사에 거부됐다(optimizer0).
기존 bounded experimental 수치 profile(2x32, context512)로 fixture를 명시하고 통과했다.
metadata fixture의 corpus/tokenizer hash 불일치도 writer가 거부했고 일치시킨 뒤 통과했다.
전체 suite·역사 부모 복원·QA32 재학습은 하지 않았다. 여기까지 SMALL updates0,
TINY optimizer16(2 vs1+1을4번), scalar Adam2, 추가 TINY generation2; 중복 실행을
고유 테스트 수에 더하지 않았다. 학습 품질 증거가 아니다.

실제 준비 명령:
`VECLIB_MAXIMUM_THREADS=1 RAYON_NUM_THREADS=1 artifacts/fresh-joint-20260919-executable fresh prepare --output artifacts/fresh-joint-20260919`.
실행파일 SHA256 `c5cc6702cbd56c940e8ba00664575465bcd89272428aa2e6220fcbe8b58752ec`.
이후 이 실행파일/정책/자료를 고정한다. 큰 corpus/model/raw는 게시하지 않는다.

| 새 자료 | examples | framed tokens(노출 전) | 최대 sequence |
| --- | ---: | ---: | ---: |
| train |8192|1700452|351|
| primary dev |512|105940|343|
| transfer dev |128|30064|318|

학습 텍스트5,048,249 bytes, tokenizer용1,614,444 tokens. 실제 vocab562,
SMALL parameters9,513,408, tensor memory 계획5,440,696,320 bytes(실측 RSS 아님),
따라서 microbatch8/accumulation1을 고정했다. 모든 provided evidence/길이/conflicting
token-prefix 검사를 학습 전에 통과했다. primary는 다른 전체 entity/scene이며 문법은
공유한다. transfer는 새 표현 및 일부 record-count 조합이다. 독립 final200은 미개봉.

- train semantic `ccee0d2d9a6803b6b8b157ac7e46483d0f7558d30482a31f40f3366ce9183b5d`
- tokenizer `ec945ee5f3cbd87992bdfa13f199de2a671b94b64337dff04d85e01d982ab9ef`
- initial weights `6d435ddd77ed2314b2038054238b32a07038715dd8dbbbf953d06d4109f9dd86`
- typed plan digest `2e7d0f0f1d38dc474671cc11fc523393a433c3a8bb28cce3e7296597c9c227eb`

로컬 허용 경로 `artifacts/fresh-joint-20260919/`: plan.r3b, metadata.r3b,
corpus.r3cor, transfer.r3cor, tokenizer.r3b, initial.r3m. 단계 로그는 로컬
`/tmp/r3-fresh-*.log`, checker 실행기록은 위 check-final 디렉터리다.
CURRENT_MODEL_STATUS=NOT_TRAINED; MODEL_QUALITY_PASS=false; S4/S5/S6=NOT_RUN;
GOAL1_READY=false; INDEPENDENT_REVIEW=PENDING. 다음은 이 한 run의 첫1/fresh resume다.

## 2026-09-19 R3BIN 후속 점검·저장 최적화

기준 source `e2684bd3957a491959c44f4e6ae7df05ea58951b`를 이어서 수정했다.
학습/기억 데이터 초기화 상태를 다시 확인했다. `artifacts/`, `docs/logs/`, 기본
`~/Library/Application Support/Replica-v3/`가 없고, Git/target을 제외한 작업 트리에
DB/corpus/checkpoint/raw record 파일도 없었다. 추가 삭제 대상은 없었다. Rust 소스,
Git 이력과 기존 `.DS_Store`는 보존했다. Cargo build cache는 학습 데이터가 아니다.
벤치마크/회귀의 합성 파일은 각 임시 디렉터리에서 생성하고 정상 종료 시 제거했다.

직접 JSON parser/writer나 text fallback은 재발견되지 않았다. checker의 deny-pattern,
테스트의 거부용 JSON fixture, 허용된 generic library 내부 의존은 구분했다.
점검 중 R3BIN payload128MiB와 stream128MiB 한도가 header52바이트만큼 어긋나던
경계를 맞췄다. 압축 도입과 함께 stream 전체의 확장 bytes/항목 수를 누적 제한한다.
IPC/standalone tokenizer에는 압축 storage frame을 허용하지 않아 기존 byte 한도를
유지한다. 모델/Adam tensor, tokenizer mapping, loss/LR, SQLite는 변경하지 않았다.

이미 소유한 Value의 저장/읽기에서 중복 tree 복사를 제거하고 frame buffer를 하나로
합쳤다. 큰 map/array 저장에만 기존 Zstd level1을 적용하며 약12.5% 이상의 저장
이득이 있을 때 채택한다. 작은 frame과 bare bytes는 raw를 유지한다. 공통 metadata,
panel publisher, 실제 평가 row/checker 경로에 연결했다. generic API도 유지한다.
내용 hash/IPC의 raw canonical bytes는 그대로이며 receipt는 실제 저장 bytes를 묶는다.
구형 raw R3BIN과 새 compressed frame의 혼합 stream을 읽을 수 있다.

### 실행과 측정

Rust/Cargo1.98.1, `--locked --offline --release --features accelerate`,
`VECLIB_MAXIMUM_THREADS=1 RAYON_NUM_THREADS=1`, M4에서 `validate binary-measure`를
실행했다. 합성 small/panel256/entropy64k의 동일 원자료를 사용한다. 최종 코드 비교는
before→after, after→before, before→after 순서의3쌍이다. 각 process의 encode/decode는
warmup3+12측정, durable은 warmup2+5측정이다. 아래는 세 process 중앙값들의 중앙값이며
pooled median이 아니다. OS cache는 비우지 않았다. 컴파일 시간은 측정에서 제외했다.

| 합성 입력 | 저장 bytes 전→후 | encode ms 전→후 | decode ms 전→후 | sync+readback ms 전→후 |
| --- | ---: | ---: | ---: | ---: |
| small |107→107|0.0006→0.0003|0.0004→0.0004|3.9731→3.9935|
| panel256 |247143→44843|0.9380→0.8407|1.1814→1.0503|5.8834→5.7047|
| entropy64k |65592→65592|0.1192→0.1145|0.1156→0.1148|3.1577→3.8540|

panel 저장81.9% 감소. 최종 표의 CPU 중앙값은 encode10.4%/decode11.1% 감소를
관측했다. timing 변동은 있고 특히 fsync 속도는 모든 입력에서 개선되지 않았다.
실제 기존 학습자료는 삭제됐으므로 그 용량/성능이나 모델 생성 속도의 추정치가 아니다.
초기 탐색 및 중간 코드의3쌍 비교는 별도 기록이고 유리한 수치로 최종 표를 바꾸지 않았다.

panel raw canonical SHA256은 전후 모두
`e550ba2fb61cc14244d261c4f2cd785b0418de8ea2da2ae78f619aaea50f21e7`이다.
모든3종 입력에서 내용 hash/복원 값이 일치했다. 최종 after3개 process 각각3종을
별도 child로 다시 읽어9/9 통과했다. sync 계측은 create-new/write/flush/file sync/
directory sync/read/검증을 동일하게 포함하며 전원 차단 내구성 실험은 아니다.

측정 실행파일 SHA256:

- before `471de6155e710b61fc656a834644d2b95153c43bb0009264e2617de40f3cbe3a`
- after `e4df3645137adab249af40390328cfed03abecc060c41674a04347c0337ec782`

원 로그: 로컬 `/tmp/r3-storage-final-{before,after}-{1,2,3}.log`.
기준 실행파일: `/tmp/replica-storage-baseline`; 측정 source는 기존
`examples/validate.rs::binary_measure`. 모델/원문/원 로그는 commit하지 않는다.

관련 회귀10개 PASS: codec4, worker IPC1, tokenizer1, checker EOS/error1,
실제 TINY 평가 파일 생성/재독1, frozen 자료 변경 거부1, 정상 close 및10가지 raw 변조1.
명령은 `cargo test --locked --offline --features accelerate,test-support`에 각각
`--lib binary::tests`, `--test runtime rust_child_protocol_failures_timeouts_stderr_and_cancellation`,
`--test native own_byte_bpe_roundtrip_no_control_promotion_or_truncation`,
`--bin replica-check harness_model_rows_require_actual_eos_and_keep_failures`,
`--test training ordinary_qa_ablation_cli_keeps_gold_and_distinguishes_question_from_record`,
`--bin replica-train repair_rf01_changed_validation_rejected_before_model_or_output`,
`--bin replica-train state_data_complete_close_and_raw_negative_fixtures`를 지정했다.
압축/구형 혼합, f64 bits, 부분 frame, 변조, trailing compressed frame 및 누적 확장
한도를 검증했다. close의 step512는 합성 라벨이며 optimizer/generation0이다.
관련 all-targets 컴파일도 통과했다. 전체 테스트/품질 평가/추가 학습은 수행하지 않았다.

**STORAGE=PASS; DATA_RESET=EMPTY_CONFIRMED; NEW_SMALL_UPDATES=0;
NEW_TINY_UPDATES=0; MODEL_QUALITY=NOT_EVALUATED; GOAL1_READY=false.**
새 파일 없이 기존 Rust codec/하네스/문서를 수정했다.

## 2026-09-19 프로젝트 JSON 사용 제거 — native serialization

기준 HEAD `ed357919355d170c9130b889acaaefa03809bc3a`의 Rust 소스를 이어서
수정했다. 이전 데이터 초기화는 유지하며 새 품질 실험이나 원자료 복구는 없다.
최신 사용자 범위에 따라 SQLite는 유지한다. 아래 초기화 직후의 SQLite 제거
대기 설명은 그 시점의 기록이며 현재 작업 범위가 아니다.

**IMPLEMENTED:** 제품 IPC/request hash, standalone tokenizer/config, 학습 best/
terminal metadata, 평가 row stream, 진단 frozen/policy/result/checker 기록을
자체 R3BIN으로 연결했다. 숫자·문자열·배열·map을 typed binary로 기록하며 JSON
문자열을 넣지 않는다. 기존 R3MODEL weights/Adam/objective, R3CORP, R3TOK, R3ER,
RPV3/R3ARCH 및 SQLite의 domain 포맷은 유지한다. 구형 text checkpoint 읽기/저장
구현과 직접 serde_json/safetensors 의존은 제거했다. 명시 legacy 모델 import도
지원 중단 오류를 반환한다. 기존 binary corpus importer의 R3BIN staging 경로는
남아 있으나 JSON 입력/fallback은 없다. 의존성과 wire 범위는 STORAGE_FORMAT.md,
DEPENDENCY_STORAGE_AUDIT.md, 실제 CLI 입력은 RUNBOOK.md에 반영했다.

**EXECUTED_THIS_RUN:** Rust/Cargo1.98.1, locked/offline, accelerate/test-support.
관련 회귀17개 PASS. 전체 품질/전체 테스트 suite는 실행하지 않았다.
다음은 `cargo test --locked --offline --features accelerate,test-support`의
실행 대상/필터다. 모든 테스트 로그는 이번 실행의 로컬 `/tmp/r3-*.log`에 있으며
Git에 포함하지 않는다. 원래 raw/model/corpus 파일이 복구됐다는 뜻은 아니다.

| 대상 | 필터 | PASS |
| --- | --- | ---: |
| lib | `binary::tests` | 2 |
| test native | `own_byte_bpe_roundtrip_no_control_promotion_or_truncation` | 1 |
| test native | `native_checkpoint_` | 2 |
| test runtime | `rust_child_protocol_failures_timeouts_stderr_and_cancellation` | 1 |
| test training | `corpus_and_tokenizer_use_train_only_and_reject_split_leakage` | 1 |
| test training | `native_training_resume_is_identical_in_fresh_processes` | 1 |
| test training | `harness_m04_cli_close_reports_stopped_arm_before_any_replay` | 1 |
| test training | `ordinary_qa_ablation_cli_keeps_gold_and_distinguishes_question_from_record` | 1 |
| test training | `curriculum_resume_crosses_sampling_boundary_in_fresh_process` | 1 |
| test training | `native_training_cancel_keeps_optimizer_boundary_checkpoint` | 1 |
| bin replica-check | `harness_model_rows_require_actual_eos_and_keep_failures` | 1 |
| bin replica-train | `state_data_complete_close_and_raw_negative_fixtures` | 1 |
| bin replica-train | `progress_lr_native_split_adam_clock_and_cursor_parity` | 1 |
| bin replica-train | `legacy_owned_digest_is_from_the_consumed_bytes` | 1 |
| bin replica-train | `repair_rf01_changed_validation_rejected_before_model_or_output` | 1 |

TINY 실제 optimizer47회: fresh resume16 + LR/Adam parity10 + curriculum/group/
corpus-switch resume20 + cancel1. SMALL0. 완전 panel close/변조 회귀의 step512는
명시한 합성 상태 라벨이며 optimizer/generation0이다. 별도의 실제 평가 생성
회귀는 random TINY를 사용해 저장/읽기 경계만 검증했고 품질 점수로 채택하지 않았다.
native tensor/Adam/RNG/forward 동일성, f64 bit/negative-zero/큰 정수 및 잘림·변조·
누락/EOS/strict UTF-8 처리, 실제 child 통신·취소 및 binary-only 학습 저장을 확인했다.

수정 중 close fixture의 objective binding 누락, CLI fixture의 명시 archive 옵션
누락이 각각 실패했다. fixture를 현재 실제 계약으로 갱신한 뒤 둘 다 통과했다.
학습 허용 검사를 완화하지 않았다. 노출 보고의 text 중첩도 typed record로 고쳤다.
단일 close 회귀는 정상1/negative10을 검사하며 약66초였다. 관련 all-targets
컴파일 검사와 dependency tree/금지 직렬화 source 확인도 수행했다.

**JSON_MIGRATION=PASS; SQLITE=RETAINED; NEW_SMALL_UPDATES=0;
MODEL_QUALITY=NOT_EVALUATED; GOAL1_READY=false; INDEPENDENT_REVIEW=NOT_RUN.**
하위 generic library 내부 JSON은 사용자 허용대로 남아 있다. 포맷 변경만으로
크기/속도 향상이나 모델 품질 회복을 주장하지 않는다. 새 Rust 파일은
`src/binary.rs` 하나이며 회귀는 기존 테스트 파일에 추가/수정했다.

## 2026-09-19 사용자 승인 데이터 초기화 완료 — Rust 소스 유지

사용자가 학습자료·모델·원문·장기기억까지 모두 삭제하고 Rust 소스는 유지하도록
명시했다. `artifacts/` 전체, `docs/logs/` 전체 및 `target/`를 삭제했다.
삭제 전 실제 할당 크기는 각각 47,884,712 / 44,068 / 14,566,876 KiB였다.
기본 사용자 저장 위치인 `~/Library/Application Support/Replica-v3/`는 존재하지
않았다. 삭제 후 작업 트리의 파일 목록에서 별도 데이터·체크포인트·DB 파일은
발견되지 않았다. Git 이력은 유지했으므로 과거 커밋의 추적 로그까지 지운 것은 아니다.

**DATA_RESET=COMPLETE; RUST_SOURCE=UNCHANGED; NEW_TRAINING_UPDATES=0.**
아래의 원자료 보존·실제 경로·체크포인트 설명은 삭제 이전의 역사 기록이다.
해당 원자료는 이제 로컬에 없으며 현재 모델이나 재실행 가능한 증거로 제시하지 않는다.
삭제는 모델 품질 개선이나 Goal1 완료가 아니다. 새 모델도 아직 학습하지 않았다.

제품·학습·도구의 JSON 사용 및 직접 JSON/SQLite 의존성 제거 요청은
**PENDING_IMPLEMENTATION**이다. 현재 Cargo.toml의 `serde_json`, `rusqlite`와
관련 Rust 호출·SQL 스키마는 그대로 남아 있다. 하위 라이브러리 내부 JSON 의존은
사용자가 허용했다. 데이터 파일 삭제와 코드 의존성 제거를 혼동하지 않는다.

검증은 삭제 대상 부재, 기본 사용자 저장 위치 부재, 잔여 파일 목록,
Rust 소스·tests·examples·Cargo 파일의 변경 없음 확인으로 제한했다.
설치 Rust/Cargo 1.98.1을 확인했고, 구현 변경이 없어 컴파일·모델 테스트는 실행하지 않았다.

## Retention retry01 최종 — 실제 저장·재개 PASS, 8회에서 보존 실패로 중단

2026-09-19 / R3-RETENTION-FIRST-1.0의 사용자 명시 재실행 요청.
**RESULT=PARTIAL: 실행 경계 수리 확인, 품질 개선 미달.** 이전 구현 오류는
재발하지 않았다. 같은 A75-R24310에서 첫1회를 정상 native 저장하고 새 process가
weights/Adam/step/objective binding을 복원했다. 총8회 뒤 저장한 동일 모델의
고정224 평가에서 보존 한도를 넘어 **RETENTION_QUALITY_STOP**으로 종료했다.
이는 새 저장/수치 오류가 아니다. command는 Failed/QualityGuard, error=None,
resume=false/complete=false/candidate=false이며 pure report만 exit0으로 검산했다.

| NORMAL_GREEDY 고정 screen | 부모 | retry01 업데이트8 |
|---|---:|---:|
| OLD |64/64|9/64|
| CROSS |60/64|7/64|
| ordinary QA subset |20/32|18/32|
| NEW |0/64|0/64|
| NEW base4 공동성공 |0/16|0/16|
| 생성 오류 |0/224|18/224|

전체224건을 실행했고 full34/224, entity46/context92/value109/event112,
wrong-citation112, EOS220/224다. strict UTF-8 오류17건이며 이 중3건은 length
종료와 겹친다. length 종료는 총4건이다. 기존 집계의 empty17은 실제 출력이
없음(actual absent)을 뜻하며 zero-token/정상 decode 빈 문자열17건이라는 뜻이
아니다. raw tokens는 남아 있다. 오류와 오답을 분모에서 제외하지 않았다.
OLD55개/CROSS53개 상실로 즉시 중단 조건(12개 이상 상실)을 충족했다.
QA 손실2개는 별도의 연속4개 상실 조건에 해당하지 않는다. 생성 오류 조건도
충족하지만 terminal 대표 사유는 먼저 계산된 retention guard 그대로다.

16/32/64/128, 전체1424, 동등한32회 historical paired 비교는 NOT_RUN_QUALITY_STOP.
역사 T32의16/11/12/0과 retry01의9/7/18/0은 업데이트 수가 달라 우열 대조나
회복 증거로 사용하지 않는다. T32/C512 및 첫 실패 R의 원본/종료 상태는 보존했다.

### 실제 학습·고정 train probe·사용량

새 SMALL **8/128** = 첫1 + 새 process7. step24310→24318,
input20,163/target2,016 tokens. 같은 native Q/R/T pool, 같은 Q4/T2 tape,
R2 source-order 순환, LR1e-4 bits4547007122018943789, default loss/first weight8,
기존 Adam m/v·절대 clock·tokenizer801·TR++ F32/Accelerate threads1을 유지했다.

| pool | 실제 draws | unique views / bases | input / target tokens |
|---|---:|---:|---:|
| Q |32|32/32|9902/945|
| R |16|16/4|4281/476|
| T |16|16/15|5980/595|

R source 전체는512views/128bases이나 중단 전 실제 노출은4bases였다. 전체 pool을
전부 학습했다고 하지 않는다. fixed train probe8의 target286은 n0/1/8에 동일하다.

| 실제 update | fixed probe token-mean NLL | teacher argmax 첫 불일치 |
|---|---:|---:|
|0|0.0003067367875618484|0/8|
|1|0.00030175398595789517|0/8|
|8|0.2361068803993889|7/8, 모두 target index0|

이는 gold prefix를 사용하는 동일 자체 모델의 읽기 전용 관측이며 자유생성 점수는
위224 screen이다. probe gradient는 optimizer에 들어가지 않았다. 추가 diagnostic
backward0; optional g_old dot delta는 SKIPPED_NOT_REQUIRED다.

| update | batch CE | 실제 global gradient norm | 실제 Adam delta norm |
|---|---:|---:|---:|
|1|1.0097669|7.93250126303735|0.10187845539056552|
|2|0.69000226|3.988080724914675|0.15196615552235695|
|3|1.0632803|6.444760541094093|0.19696086019830011|
|4|0.31918082|3.942492362606018|0.1974686745884048|
|5|1.0271827|7.469119775708806|0.2221286960794575|
|6|0.83580434|4.419314633536732|0.2544809994608821|
|7|0.84097683|4.442205642074556|0.282410518723017|
|8|1.3148707|9.889061689934326|0.27260470814156723|

매 batch는 서로 다른 자료다. 이 CE 열을 동일 probe의 전후 NLL처럼 해석하지 않는다.
모든 update에서 clipping이 적용됐고 coefficient는 n1 0.12606364207712392,
n8 0.10112182847617972다. n8의 중복 없는 group theta/delta norm은
FFN77.36167691942913/0.22843413346734873,
Q-K-norm91.94637116266694/0.06626517978349562,
V-O36.852495626055074/0.06388047757024293,
shared embedding35.89124740970979/0.11687336780305964다.
n1의 weights hash와 delta는 이전 실패의 실제 첫 update와 동일하며 새 fork의
physical file hash는 새 run binding을 담아 다르다. LR/clip/Adam이 근본 원인이라고
이 수치만으로 단정하지 않는다. n32 관측은 중단되어 없다.

이번 normal generations **232 entered/232 returned** = 부모 parity8 + screen224,
generated tokens8,268 =236+8,032(EOS 포함). 자체 diagnostic forwards24,
추가 diagnostic backwards0, scalar0. 관련 quick 신규 TINY53; 이전61과 합쳐114/128.
이전 실패 비용까지 따로 합치면 SMALL9, input22,726/target2,254,
normal generations240, 자체 forwards32다. 실패 사용량을0으로 지우지 않았다.
prepare elapsed11.570519209초, segment0 command14.58736425초,
segment1 command54.495703875초: 모델 호출 명령 합80.653587334초.
명령의 마지막 typed F64를 기존 reader 검증 후 확인했다. cleanup reservation은
명령마다120초이며 실측 elapsed가 아니다. 컴파일17.82초와 quick은 모델 시간에
합산하지 않는다. 실행/자료/수치 오류 또는 UNKNOWN 사용량은 이번 SMALL에 없었다.

### 실행 코드와 원자료

실행 HEAD `49f0e6e693f8dc6b43b6b85a458e5f0877ef924a`는 정상 push 후 full remote
SHA 일치를 확인했다. Rust source candidate는
`c2ce9c3ce1c456b54702ee6611f26157794836e1`과 동일하며 이번 turn의 소스 diff0이다.
source digest `553bb13928d0334c0c054c49c8f36321f5e538f5b228fc1b2d4c358031d7d382`,
실행 binary SHA256 `48b3731677e40244f950451a3f0a830b53602c1eb17f432695878a0319ab7b96`,
input binding `270e23fd0bed028e7a9e7b4c6ac9b2e380705a493522a83808940d336d187ae1`.
Rust1.98.1, 기존 Cargo.lock, offline accelerate release를 사용했다.

허용된 로컬 증거 root: `artifacts/retention-first-retry01-20260919/`.
`prepare.log`, `first.log`, `resume.log`, `final-report.log`, `model-inspect.log`가
실제 명령/순수 재검산 기록이다. `quick.log`/`quick/`는19명령25테스트 PASS,
`release-build.log`는 실제 빌드다. `executed-replica-train`을 보존했다.
`retention-plan/r-replay-registration.r3er`가 이 시도 하나를 고정한다.
`R-REPLAY/inputs.r3er`, `parent-parity/`, `retention-probe-0000/`,
`retention-probe-0001/`, `retention-probe-0008/`, `segment-00/`와 `segment-01/`의
native/terminal/command/raw가 원 binary 증거다. 실제 명령은 기존
`retention-prepare --screen artifacts/quality-recovery-bounded-bridge-20260919/T-SCREEN
--output artifacts/retention-first-retry01-20260919/R-REPLAY`, 이어서 같은 root의
`run`, 새 process `run --resume segment-00/terminal.r3er`, 최종 읽기 전용
`screen-report --terminal segment-01/terminal.r3er`다. 모두
`replica-train recovery native` 아래 명령이며 전체 root를 명시했다.

마지막 durable checkpoint는
`R-REPLAY/segment-01/step-0008.r3m`,115,285,632 bytes, step24318,
physical `b6e9c4b543da19ae6d774de3c73ef3ea8082525c89ab6a0b7273cb029af52b42`,
actual weights `46639c2ad80f500a20b25a3eff43855c1527e2a2e1c92ace34d981a2da493519`.
state consumed43,333,422/target3,320,739/sampler6741870243436089021이다.
첫 정상 저장 `segment-00/step-0001.r3m`은 같은 크기, step24311,
physical `8c5644cb6688b528e28037a4845969a7eaae524f6bafa86d6be032d6a3fad600`,
actual weights `7b1b9b96bc10f60c646ae6f11ab32c6687e696ee207b4fc940879a3867929292`.
새 process의 학습 재개와 최종 pure report가 실제 Adam/state와 계보를 검증했다.

`frozen-source.sha256`37개 source/lock/binary 모두 종료 후 동일,
`closed-attempt.sha256`89개 파일 모두 pure report 후 동일.
이전1191개 원본과 첫 실패46개 파일도 전후 SHA256가 모두 같았다.
새 모델/원문/raw/임시 지시문/DB는 Git에 올리지 않았다. 새 영구 파일0,
운영 모델 pointer 변경0, 최종 owned 학습/검증 process0이다.

### 해석과 종료 판정

실제로 관측된 것은 첫8회에서 기존 copy 능력과 fixed train probe가 악화됐고,
새 선택 능력은 아직0이라는 결과다. 이 Q4/R2/T2 순서의 보존 효과는 지지되지
않았다. 저장 수리나 binary 형식 변경이 품질 회복을 만들었다고 하지 않는다.
첫 응답 분포가 변한 경계는 관측했지만 학습 부진의 근본 원인은 UNRESOLVED다.
다음 단일 가설 후보는 초기 R 노출의 base 집중이다: 같은 pool/16draw에서
4base 반복과 base 분산 순서만 비교해야 검증할 수 있다. 현재 관측만으로 이
가설을 확정하지 않으며, 새 LR/loss/tokenizer/비율 순회를 시작하지 않는다.
후속 비교 실행0이며 별도 인가가 필요하다. 이번 remaining120은 새 시도권이 아니다.

CODE_REPAIR/AUDIT_PUBLICATION/SCREEN_ANCESTRY=PASS_DIRECT_SCOPE;
NATIVE_EXISTING_SCOPE=UNCHANGED_BINARY; H3_SEAL=NOT_OPENED;
FIRST_SAVE_FRESH_RESUME=PASS_ACTUAL_SMALL;
EXPERIMENT=QUALITY_STOP_CLOSED_AT8;
RETENTION_OLD/CROSS=FAILED; RETENTION_QA=18/32(loss2);
NEW_SKILL=NOT_IMPROVED(base4=0); REPLAY_HYPOTHESIS=NOT_SUPPORTED_IN_THIS_RUN;
HISTORICAL_EQUAL32_COMPARISON=NOT_RUN;
H3_JOINT=NOT_PASSED; S4/S5/S6=NOT_RUN_NOT_PASSED;
GOAL1_READY=false; GOAL1_ACCEPTED=false; INDEPENDENT_REVIEW=NOT_RUN.

## Retention retry01 사전 검증 — 사용자 요청에 따른 별도 재실행

2026-09-19 사용자가 오류 수정 후 재실행을 명시적으로 요청했다. 저장 사유를
고친 source c2ce9c3ce1c456b54702ee6611f26157794836e1을 유지한다. 기존 실패의
1회와 step24311은 그대로 보존하고 A75-R24310에서 새 등록 한 번만 진행한다.
새 root는 `artifacts/retention-first-retry01-20260919/`이며 원래 Q4/R2/T2,
최대128회,8/16/32 보존 검사와64/128 연장·중단 기준은 그대로다.

현재 source의 관련 quick19명령/25테스트가 모두 PASS했다. 저장 사유의 실제
native writer→새 process loader, 발행 sync 실패와 모든 조상 command, 부분 평가
재개가 포함된다. 로그는 새 root의 `quick.log`와 `quick/`에 있다. 신규 TINY53,
지난 실행61과 합쳐114/128; 신규 SMALL0. 원본1191파일과 기존 실패46파일의
SHA256 일치를 재확인했다. 원본 T32/C512·실패 run의 재학습/재개는 하지 않는다.

이 절은 실행 전 사전 검증이다. 실제 SMALL 결과·사용량은 종료 후 따로 기록하며
코드 PASS가 보존/새 능력/H3/S4/Goal1 PASS라는 뜻은 아니다.

## P5 최종 — 경계 수리 PASS, R-REPLAY는 첫1회 후 실행 결함으로 종료

R3-RETENTION-FIRST-1.0 / 2026-09-19. **RESULT=PARTIAL.**
실제로 새 SMALL1회를 수행했으나, 구현자가 새 probe 저장 호출에 넣은
`RETENTION_PROBE` 문자열이 기존 `save_arm` 종료 사유에 없어 정상 첫 저장에
실패했다. 오류는 `invalid input: unknown recovery termination`이다.
disk sync나 모델 수치 실패로 확인된 것이 아니다. 이 실행은 품질 판정 전의
**EXECUTION_FAILED**이며, 원 terminal/command는 IntegrityFail/Failed,
resume=false/complete=false/candidate=false로 보존했다.

이후 해당 호출을 기존 `RECOVERY_SCREENING` 사유로 수정했고, 같은 native
writer와 새 process의 resume loader를 쓰는 TINY 직접 회귀에서 수정 전 실패,
수정 후 weights/Adam/state 복원 통과를 확인했다. 이 회귀의 optimizer/forward는0.
수정 후 SMALL 학습·실패 run 재개·새 attempt는 **실행하지 않았다**. 남은127회는
새 시도권으로 사용하지 않는다. 1회 뒤 teacher probe,8/16/32 screen,64/128 및
전체1424 평가, 동일32 역사 paired 비교는 모두 NOT_RUN_EXECUTION_FAILURE다.

### 실제 소스와 실행 파일

| 구분 | 전체 SHA / 의미 |
|---|---|
| 기준 source | `eb66357fbee97de67b637ec0b1986e636ebd372e` |
| P1 경계 수리 | `e221d338639a97f811d631a4b649045d46ecef80` |
| 실제 R 실행 source | `56f3f0eb3029dc549eee4cfd3bd052ffc4cb6243` |
| 저장 사유 수리 후 candidate | `c2ce9c3ce1c456b54702ee6611f26157794836e1` |
| 실제 실행 binary SHA256 | `58a723e08a4837e9a53053ffda319f590c3c185f0df82e33b54f2f0bc18fcbc2` |
| 실행 evaluator source digest | `eac9ffaa34e8d05192c9290c4411410e5f858bec622f31046b683d8e21b28b8f` |
| 실행 input binding | `7cb880ec9e67d171037dd543bd16ceaea757e440e8400bcec9f27b8e5995a338` |

[기준→최종 candidate 소스 diff](https://github.com/seoyd/replica-v3/compare/eb66357fbee97de67b637ec0b1986e636ebd372e...c2ce9c3ce1c456b54702ee6611f26157794836e1).
각 source는 정상 push하고 full remote SHA 일치를 확인했다. 이 최종 결과 절은
별도 docs-only 보고다. 실제 SMALL 실행 동안 frozen source/lock/binary hash는
유지됐고, 종료 뒤에만 위 저장 사유와 실패 보고를 고쳤다. 최종 candidate로
SMALL1이 성공했다고 소급 표시하지 않는다.

### EXECUTED_THIS_RUN: 사용량과 첫 update 관측

부모는 A75-R24310 그대로다. 원 native T corpus와 owned train2560의 bytes 일치,
R의 과거 train membership, Q4/T2 공통 tape와 sampler, LR1e-4 bits
4547007122018943789, default loss/first weight8, Adam/clock/tokenizer/F32를 검증했다.
별도 단일 등록 뒤 metadata-selected 부모8개를 normal greedy로 생성했고,
기존 raw token/EOS/error와 **8/8 동등**했다. 정답률8/8이라는 뜻이 아니다.
R128의 계획 input327,969/target34,458은 실제 사용량과 구분한다.

| 실제 첫 배치 | draws | input tokens | target tokens | mean CE | weighted objective |
|---|---:|---:|---:|---:|---:|
| 기존 Q |4|1295|111|0.01262501|0.01264575|
| 과거 full-copy R |2|542|60|0.0002938095|0.00031284845|
| 기존 temporal T |2|726|67|3.565754|6.5056934|
| 합계 |8|2563|238|1.0097669|1.8374114|

신규 SMALL **1/128**, TINY **61/128**, scalar0. 신규 SMALL normal generation
**8 entered/8 returned**, 생성 tokens236(EOS 포함); 학습 뒤 generation0.
자체 train diagnostic forward8/256, extra diagnostic backward0/8.
교사 관측은 같은 자체 모델이며 외부 teacher/모델/API 호출0이다.
TINY observer 동등성 두 실행에서 추가 teacher4회를 사용했으며 SMALL과 합산하지
않는다. 다른 TINY 실패/강제 종료 회귀의 UNKNOWN tail은 원 로그 그대로 유지한다.

학습 전 고정 R train8의 teacher target286, token-mean NLL
0.0003067367875618484, 첫 teacher mismatch는 모두 null이었다. free generation
품질 수치가 아니다. 첫 update global gradient norm7.93250126303735,
clip coefficient0.12606364207712392, 실제 Adam delta norm0.10187845539056552.

| 중복 없는 parameter group | parameters | theta norm | 실제 delta norm |
|---|---:|---:|---:|
| FFN |7077888|77.35740853593474|0.08900782995729|
| Q/K/norm |1113792|91.94951291310507|0.038001679380048545|
| attention V/O |1105920|36.84714223890798|0.022607355065883003|
| shared embedding |307584|35.898508058962214|0.02239655643804834|

tied embedding은 한 번만 합산했다. optional g_old dot delta는
SKIPPED_NOT_REQUIRED. 학습 후 probe1/8 및 delta8/32는 관측되지 않았다.
prepare 명령의 기록 elapsed11.3693625초, 첫 학습 명령12.507687667초다.
각120초 cleanup reservation은 실측 elapsed가 아니다. 컴파일/TINY/순수 파일
재검산을 이 두 모델 호출 명령의 성능 수치로 합산하지 않는다.

### 마지막 durable 파일과 허용된 원자료

정상 screening checkpoint 발행은 실패했지만 오류 보존 경로의
`artifacts/retention-first-20260919/R-REPLAY/segment-00/final.r3m`은 저장됐다.
115,285,632 bytes, 실제 step24311. 이것은 **실패 보존 checkpoint**이며 측정된
품질 endpoint나 재개 certificate가 아니다. 새 process의 pure report가 실제
weights/Adam/step/tape를 검사하고 실패 상태를 그대로 반환했다(exit1, 완료 문구 없음).

- physical SHA256: `11787d976a64423f7dc40b0cc6443128a5b4a09305dc572d3c50a81e50451be5`
- weights SHA256: `7b1b9b96bc10f60c646ae6f11ab32c6687e696ee207b4fc940879a3867929292`
- Adam SHA256: `052f10a7d7380e61d9e8dc64ec365fc489791987200972b9147dcbf331b083f2`

인가된 새 증거 root는 `artifacts/retention-first-20260919/`다.
`p3-prepare.log`, `p4-first.log`, `p5-final-failure-report.log`가 실제 실행 기록이다.
`R-REPLAY/inputs.r3er`, `parent-screen.r3er`, `parent-parity/`,
`retention-probe-0000/`, `segment-00/{terminal,command}.r3er`에 typed 상태와 raw가
있다. `retention-plan/r-replay-registration.r3er`가 이 한 시도를 고정한다.
`executed-replica-train`은 실제 binary 보존 사본이다. `final-candidate-source.diff`,
`p3-frozen-source.sha256`, `p4-execution-source-verified.log`가 코드 연결 근거다.

기존 모델/자료 위치와 hash는 아래 P2/P3 및 과거 절 그대로다. 원본1191파일과
이번 실패 시도46파일의 SHA256가 종료 후에도 전부 같았다. 모델/Adam/corpus/raw/
임시 지시문/DB/target은 Git에 게시하지 않았다. 운영 모델 pointer는 유지했다.

### 최종 대조와 범위별 판정

PF-AUDIT 발행 pending·digest·fresh denial, CHAIN-SCREEN 모든 조상 command,
실제128/257/511/512 Record 경계, native source/tape/parent/raw 연결은 직접 확인했다.
관련 quick19명령/24고유 테스트 PASS + 별도 native identity1 + 저장 사유 회귀1로
이번 고유 테스트 **26개 PASS**다. 마지막 source 수정분은 해당 직접 process
회귀와 all-targets clippy, 실제 실패의 pure-read 검산으로 확인했다.
`p5-save-red.log`와 수정 후 `p5-save-green.log`를 모두 보존한다. 중간 fixture
binding/환경/컴파일 실패와 SMALL 실패를 숨기거나 PASS 횟수로 합산하지 않는다.

CODE_REPAIR=PASS_DIRECT_SCOPE; AUDIT_PUBLICATION=PASS; SCREEN_ANCESTRY=PASS;
NATIVE_EXISTING_SCOPE=UNCHANGED_BINARY; RETENTION_SAVE_REASON_FIX=PASS_TINY;
EXPERIMENT=EXECUTION_FAILED_CLOSED; FIRST_SAVE_FRESH_RESUME=NOT_COMPLETED;
RETENTION_OLD/CROSS/QA=NOT_MEASURED; NEW_SKILL/base4=NOT_MEASURED;
REPLAY_HYPOTHESIS=UNRESOLVED; COMPARATOR=HISTORICAL_T32_NOT_EVALUATED_AT_EQUAL32;
H3_JOINT=NOT_PASSED; S4/S5/S6=NOT_RUN_NOT_PASSED;
GOAL1_READY=false; GOAL1_ACCEPTED=false; INDEPENDENT_REVIEW=NOT_RUN.

다음 한 가지 미해결 가설은 원래의 full-copy replay가 이전 능력을 보존하는지다.
첫 배치에서 T objective가 Q/R보다 컸다는 관측은 있으나, 실제 전후 NLL와8회
screen이 없어 망각 원인이나 R 효과의 근거로 단정할 수 없다. 먼저 이 동일 가설의
실행 증거가 필요하며, 새 ratio/LR/loss/tokenizer 비교를 추가하지 않는다. 기존
실패를 재개하거나 새 시도를 자동 등록하지 않았다. 후속 연구는 별도 인가가 필요하다.
활성 owned 학습/검증 process0. 새 영구 파일0; 로컬 실험 산출물만 생성했다.

## P2/P3 — 기존 copy train 확인, R-REPLAY 준비

P1 source `e221d338639a97f811d631a4b649045d46ecef80` 정상 push 후 remote 전체 SHA
일치 확인. P2는 기존 T32 terminal의 모든 command와 raw224를 순수 재검산했다.
OLD16/64, CROSS11/64, QA12/32, NEW0/64, errors28은 과거 보고와 같다.
첫32 tape input85,211/target8,322도 native/Adam/cursor와 일치했다. 신규 생성0.

검증된 replay 출처는
`artifacts/durability-pair-restart-20260918/attempt-R/A75-R/inputs.r3er`, SHA256
`515844a64a04cd2d7a670a065ae33f3cfe27942cf201c4747ebf62b6571902da`다.
그 arm의 실제 끝점 model/Adam/step이 A75-R24310과 일치한다. 실제 train-only
단일 current 원문+정확한 citation 문법을 검사했다. metadata hash로 base를 고르고
선정된 원문의 source 순서를 유지한다. 새 원문이나 평가 정답을 만들지 않았다.

| pool | views / bases | 원문 full-copy | available input / target tokens |
|---|---:|---:|---:|
| Q 기존 anchor |2048 /2048|0|641840 /62388|
| R 과거 train |512 /128|512|146001 /19062|
| T 기존 두 기록 선택 |512 /128|단일 copy0, 다중 선택512|192824 /19712|

R ordered native bytes SHA256는
`93d97946fbca87fd6d359b2a3db76189e11c6959a624dfea914da1e8e73000ba`.
부모 A75-R arm 안의 R 노출1024회는 저장된 draw로 확인했다. 그 이전의 정확한
개별 노출은 UNKNOWN이다. dev/CROSS/NEW/conditional과 ID·binding·본문·sequence
교집합을 검사했으며 seal은 읽지 않았다. R+Q+T3072의 정확한 prompt token과
서로 다른 target 충돌0. 자세한 분모·ID 길이/반복 분포는 로컬 `p2-inspect.log`.

구현은 기존 native input/registration/runner/RunControl/Adam/scorer를 사용한다.
새 R policy는 첫128 old tape의 Q4/T2와 sampler를 유지하고 두 slot만 R source
순환으로 교체한다. 데이터 변경은 명시적 새 objective execution binding을 가진
fork다. loss 수식과 Adam m/v·clock·rate·tokenizer·model은 동일하다.
부모 parity8과 probe0/1/8는 실제 호출 전에 entry를 저장하고 typed final/pending
발행을 거친다. 정상 free generation과 train teacher mismatch를 구분한다.
기존 Adam observer로 Q/K/norm·V/O·FFN·shared embedding delta/norm을 측정하며
tied embedding은 한 번만 센다. optional old-gradient dot은 SKIPPED_NOT_REQUIRED.
TINY observer ON/OFF weights/Adam/clock/sampler 정확 동등성 PASS(실제2 updates,
추가 자체 teacher2, diagnostic backward0). 이 단계에서 SMALL 학습은 아직0이다.

단일 연구의 실제 호출과 품질은 다음 실행 결과 절에 기록한다. 저장/계측 구현
PASS는 보존·새 선택 능력·H3/S4/Goal1 PASS의 대체물이 아니다.

P3 검증: 관련 quick19명령/24테스트 PASS. 그 후 최종 읽기 전용 native T 내용
검사와 pool 사용량 집계 추가분은 all-targets clippy 및 고정 gate 직접 회귀 PASS,
release build PASS. P1 별도 native identity 회귀까지 이번 고유 테스트25개다.
신규 TINY61/128(quick native48+standalone resume3+observer2, 별도 P1 native6와
observer2); 실패했던 fixture 인덱스 검사는 optimizer 진입 전이어서0이다.
초기 fmt/check/fixture 환경 실패 로그도 보존했고 최종 PASS로 덮지 않았다.
학습 source와 실행 binary를 고정한 뒤, 부모 parity8 및 단일 R 연구를 수행한다.

## P1 — retention 연구 전 발행·조상 경계 수리

R3-RETENTION-FIRST-1.0. Base source eb66357fbee97de67b637ec0b1986e636ebd372e,
시작 HEAD dbd0eb3fcaf377524574670cc72e325a81f5959c. 기존 dirty 원자료를 보존했다.
Rust/Cargo1.98.1, 기존 lockfile/offline. 모델/학습 수식 변경0, 신규 SMALL0.

EXECUTED_THIS_RUN: 새 process 회귀2 PASS. audit의 실제 typed ArtifactAudit
writer/publisher/admission에서 정상 공개와 공개 후 directory-sync 실패를 검사했다.
final이 존재해도 durable pending이 있으면 screen-prepare가 모델 호출 전에 거부한다.
모든 후속 record write도 실패하는 fault에서 동일하게 차단된다. 테스트 payload는
명시적 합성 자료이며 C512 재생성이나 실제 모델 진단 PASS로 세지 않는다.
정상 audit은 start/final/model/input/source/raw identity와 publication digest를 묶는다.
옛 audit에는 새 증명을 덧붙이지 않고 read-only 재감사와 새 admission을 구분한다.

실제 TINY 첫1+새process1 뒤 명시적 합성 오류 row로 QualityGuard를 만든 회귀에서
정상 screen-report는 연구 중단만 보고했다. 조상 command missing/corrupt/Failed는
모두 거부, 완료 문구 없음, 보고 전후 파일 목록/bytes/hash 불변. 합성 오류는 모델
품질 관측이 아니다. 정상 close와 중단 report가 같은 조상 effective_outcome을 사용한다.
기존 preflight 공개/sync/cleanup 회귀 PASS, clippy all-targets PASS.
정확한 이번 사용량과 후속 native/capacity 검사는 최종 절에 합산한다.

로컬 증거: `artifacts/retention-first-20260919/p0-{worktree.txt,local.diff,inputs.sha256,originals.sha256}`,
`p1-{process,preflight,capacity,native,clippy}.log`. 원 모델/raw/corpus는 게시하지 않는다.
P2 coverage·parent parity와 R-REPLAY는 아직 NOT_RUN. 기존 T32/C512의 실패와
resume=false는 그대로다. CODE_REPAIR와 H3/S4/Goal1은 별개이며 품질 미완료다.

## R6 최종 — 경계 수리 PASS, T-SCREEN32 품질 회귀로 종료

R3-QUALITY-RECOVERY-BOUNDED-BRIDGE-1.0 / 2026-09-19.
**RESULT=PARTIAL: 구현·진단·정해진 중단 프로토콜 완료, 모델 품질 회복 실패.**
T_SCREEN_COMPLETED=true는 승인된 조기 종료 절차를 마쳤다는 뜻이다.
SMALL512 완료나 정상 command/후보 승인이라는 뜻이 아니다.

REVIEW_BASE=`e08687eff768dd8b6426accebdfe148d3ce8b556`.
R2 source=`7cefe81b32a8fa98e1755f3348254e3271c7cd8f`.
최종 구현/실제 T 실행 SOURCE_COMMIT=
`eb66357fbee97de67b637ec0b1986e636ebd372e`.
각 source commit을 정상 push하고 실제 remote 전체 SHA 일치를 확인한 뒤 실행했다.
이 최종 결과 절은 별도 docs-only 보고이며 실행 소스를 바꾸지 않는다.
[실제 candidate diff](https://github.com/seoyd/replica-v3/compare/e08687eff768dd8b6426accebdfe148d3ce8b556...eb66357fbee97de67b637ec0b1986e636ebd372e).
T execution binary SHA256=
`a9dead446823e7ac606c988333bdabec3012dc2635aeaea981bf73927d74e068`;
evaluator source digest=
`ae9a015ee2f168355a22f162365d6078d14cede1937a2ef0bf776e707637eac5`.
학습 전후 source/test/binary hash가 같았다.

### 실제 동일 endpoint 결과

EXISTING_RAW: 부모의 complete olddev/CROSS/ordinary 및 replacement-01 newdev를
각 원래 모델/내용 binding으로 검증했다. metadata와 고정 hash 순서로 OLD/CROSS/
NEW의16 base×4 view, ordinary의 QA category 균형32를 고정했다. 부모224는
원래 row의 명시적인 projection이며 이번 generation0이다. aux는 QA에 넣지 않았다.

EXECUTED_THIS_RUN: 같은 A75-R24310/native/Adam/tokenizer/T corpus/6:2 tape,
default loss/first-target weight8, constantLR1e-4(bits4547007122018943789), F32
CPU Accelerate threads1. 첫1회 저장/정상 TimePause 뒤 새 process31회를 수행했다.
학습 중 소스·LR·정책·자료·평가 기준을 바꾸지 않았다.

| 고정 screen | 부모24310 | T-SCREEN24342 | 변화 |
|---|---:|---:|---:|
| OLD strict |64/64|16/64|−48|
| CROSS strict |60/64|11/64|−49|
| ordinary QA |20/32|12/32|−8|
| NEW strict |0/64|0/64|0|
| NEW base4 |0/16|0/16|0|
| 생성 오류/비정상 종료 |0/224|28/224|+28|

32회 native를 **평가 전에** 저장했고224행을 모두 반환했다. EOS 종료198/224,
strict UTF-8 오류가 명시된 로그2건이다. 이224표본의39/224를 전체 QA400이나
H3 전체 점수로 바꾸지 않는다. OLD/CROSS 손실이 각각12 이상이고 오류 급증
조건도 충족했다. 저장된 판정은 QUALITY_REGRESSION_STOP/QualityGuard이며
`resume=false, complete=false, candidate=false`다. command status는 Failed,
error=None(품질 정책 중단)으로 남았다. storage 실패로 뭉뚱그리지 않는다.

새 process의 `screen-report`가 native/step/tape/LR/guard/terminal/command를
재검산하고 동일한32회 중단 및 점수를 반환했다(exit0). 원 학습 명령 exit1과
보고 명령 exit0은 목적이 다르다. 정상 완료 certificate나 후보 승격을 만들지
않았다. 64/128/256/512 및 최종1568 panel은 **NOT_RUN_QUALITY_STOP**이다.
가장 좋은 중간 checkpoint를 선택하거나 남은480회를 재사용하지 않았다.

### 사용량·저장·증거

신규 SMALL32/512, input85,211/target8,322 tokens, anchor192/focus64 **노출**,
TINY63/128, scalar0. 신규 normal generations288/4096=R3 64+T224, 모두 반환.
기록된 generation tokens13,006=R3 1,962+T11,044(EOS 포함). 신규 own diagnostic
forwards60/192, 모두 반환; T teacher0, 외부 모델 호출0. 과거 부모320/teacher64는
순수 검증/재사용이며 새 호출로 합산하지 않았다. 기존 C512 소비량도 별도다.

최종 사용량 정정: 앞선 진행 보고의 TINY60은 native runner의 update entry와
그 직접 회귀만 센 값이었다. `quick-r2-fixed/command-08.stdout`의 별도 standalone
resume 회귀3회(`AMBIENT_RESUME_TINY_UPDATES=3`)를 포함하면 **63회**다.
실제 재실행3회가 추가된 것이 아니라 누락된 관측을 합산한 정정이다.

R3 command elapsed13.9488005초. T prepare/start의 보수적 budget ledger는
각 완료 명령에 publication/cleanup120초를 별도로 예약했고, 마지막 재개 시작에
430.552초가 charge돼 있었다. 이 예약을 실제 계산 시간으로 부르지 않는다.
정확한 신규 command elapsed는 각 binary command에 남아 있으며 build/test와
분리된다. 마지막 평가 중 관측 wall73.781초는 최종 command 시간의 대체값이 아니다.
과거 C의 누락 command 총사용량과 강제종료 TINY의 미확정 tail은 UNKNOWN 유지.

마지막 durable native:
`artifacts/quality-recovery-bounded-bridge-20260919/T-SCREEN/segment-01/step-0032.r3m`.
physical SHA256=`4d2eb3e7d248421adf85101c9680b96c0ae3125d0c4afb0296397c6827e9c5f4`,
weight hash=`70e9f858ff13b4f2bd56e759a71696c200fe09760d01c56c53adf36a24f6b934`,
step24342. 운영 모델 pointer는 바꾸지 않았다.

인가된 로컬 증거 root는 `artifacts/quality-recovery-bounded-bridge-20260919/`다.

| 파일/경로 | 내용 |
|---|---|
| `c512-diagnostic.log`, `c512-diagnostic/audit-final.r3er` | C full-state/raw/fresh64/numeric60 진단 |
| `c512-diagnostic/t-screen-registration.r3er` | 새 T-SCREEN 단일 등록 |
| `T-SCREEN/inputs.r3er`, `parent-screen.r3er` | 고정 native 자료·정책 및 기존 parent raw projection |
| `T-SCREEN/segment-00/{terminal,command}.r3er` | 첫1회 durable TimePause |
| `T-SCREEN/segment-01/{screen-0032,decision-0032,terminal,command}.r3er` | 224 raw와 guard/품질 중단 |
| `t-screen-{prepare,first,resume,report}.log` | 실제 명령 출력과 새 process 재검산 |
| `T-SCREEN/executed-replica-train` | 실제 학습 실행 파일 보존 사본 |
| `quick-r2-fixed/`, `screen-policy.log`, `screen-final-process.log`, `final-parent-stop-regressions.log` | 직접 회귀 증거 |
| `r0-originals.sha256`, `originals-after.sha256-check` | 기존927파일 보존 검산 |

부모는 `artifacts/native-corpus-objective-20260918/A75-R24310-v2.r3m`, 원 T corpus는
`artifacts/quality-first-bridge-20260919/data-ready/T-TEMPORAL.r3c`이며 시작 시
physical hash와 종료 후 hash가 같다. 기존 C/T study와 replacement 관측의927개
파일 hash가 모두 유지됐다. C의 누락 terminal/command는 계속 없다. 원 실패를
수리하거나 기존 T0을 실행 완료로 바꾸지 않았다. 모델/raw/코퍼스는 Git에 게시하지 않는다.

### 최종 대조와 판정

FB01_REAL_CAPACITY=PASS: kind14/metrics의 공통512, count/LR/정책/남은 bytes 검증,
실제 Record→publisher→reader 경계 및257/511 RED→GREEN. 합성 count는 optimizer0.
FB02_REGISTERED_ROOT=PASS: actual canonical observation root를 observe/prepare/
report에 공통 적용, 실제 replacement 등록 TINY positive/copy/source/binary/registry
negative 및 기존 native parent identity 변경 거부. 코드 변경은 기존
`src/experiment_record.rs`, `src/codec.rs`, `src/check_main.rs`, 기존 test와 문서에 한정.

관련 quick15명령/18고유 테스트 PASS, 후속 screen policy1/process1 및 기존 native
parent/종료 receipt 직접 unit2 PASS: 이번 **고유 테스트22개**. 기존 bridge parity의
중복 실행을 고유 수에 더하지 않는다. fmt/check/clippy/release PASS, zero-test0.
의도적인 기존 decoder RED2건과 초기 취소 핸들러 중복 fixture FAIL은 로그에 보존했다.
전체 무관 테스트·대규모 재학습은 실행하지 않았다.

CODE_FIX_ACCEPTED=PASS_IMPLEMENTER_DIRECT_SCOPE;
SAVED_C_ARTIFACT_VALID=VERIFIED; C512_RAW_RECOUNT=AGREED;
HISTORICAL_C_FINALIZATION=FAILED_UNCHANGED;
REGRESSION_DIAGNOSIS=OBSERVED_NOT_CAUSAL_PROOF;
T_SCREEN_COMPLETED=true/STOPPED_32; MODEL_QUALITY_RECOVERED=false;
H3_JOINT=false; NEW_BINDING_SKILL=false; H3_SEAL=NOT_OPENED;
S4/S5/S6=NOT_PASSED_NOT_RUN_THIS_CONTRACT; GOAL1_READY=false;
GOAL1_ACCEPTED=false; INDEPENDENT_REVIEW=NOT_RUN.

R3CORP/R3TOK/R3MODEL/R3ER를 유지했다. 신규 canonical JSON writer0, SQLite/IPC/
legacy JSON 원본·모델 구조·tokenizer·optimizer 수식 변경0. 신규 영구 Rust module0,
신규 영구 파일0. 로컬 실험 증거와 신규 checkpoint만 생성했다.

다음 **미실행 가설 하나**: 동일6:2 배합에서 기존 copy 동작을 보존할 anchor 내용의
커버리지가 부족할 수 있다. C와 T 모두 큰 보존 손실을 보였고, 이번 정확한 입력/캐시
검사에서는 계산 결함을 재현하지 못했다는 제한된 근거다. 기존 anchor가 실제 old
dev/CROSS 형식을 얼마나 포함하는지부터 확인하는 별도 사전등록 검토를 제안한다.
인과관계는 UNRESOLVED이며 자료 교체·비율/LR 변경·추가 학습은 실행하지 않았다.
T32와 과거 C512의 동일 예산 승패 비교는 하지 않는다.

## R3 / R4 진입 당시 기록 — C512 독립 검증 및 T-SCREEN 준비

R3-QUALITY-RECOVERY-BOUNDED-BRIDGE-1.0. R2 source
`7cefe81b32a8fa98e1755f3348254e3271c7cd8f`를 정상 push하고 remote 전체 SHA를
확인했다. 진단 실행 binary SHA256:
`bb0d8d7c4c8a9d8f382f58bff8969967e26f81396acf43877afa3e4c6700a827`.
`c512-diagnostic.log` / `c512-diagnostic/audit-final.r3er`에 실제 결과를 보존한다.

EXECUTED_THIS_RUN: C512 physical hash ce01daec…ef624, model ef7962e0…be440e,
Adam59ecfba1…c62939, step24822 및 부모24310, 실제 tape/counters/내장 objective
binding 전체를 검사했다. 기존 terminal/command는 여전히 없고 과거 실패·UNKNOWN
사용량·resume 부적격을 바꾸지 않았다. C/T 각각2560개 학습 사례의 train/product
prompt token이 같고 exact-input/differing-target 충돌0이다.

DERIVED_FROM_EXISTING_RAW: olddev6/256, CROSS4/512, ordinary QA188/336와
aux24/64, newdev4/256, conditional0/144를 원래 tokens/EOS/UTF-8/분모로
재채점했다. 기존 보고와 같다. olddev entity78/context39/value193/event83,
CROSS123/57/350/171이다. olddev 분류는 인용/기록 오류148, 대상44, context23,
해석불가 형식31, UTF-8오류4, strict정답6. CROSS는 각각277/119/31/70/11/4.
확정 문법의 짧은 QA 형식 전환은0건이었다. 이 분류는 우선순위 첫 분류이며
전체 필드 오류의 독립 합계가 아니다. 원출력·primary metric은 보정하지 않았다.

EXECUTED_THIS_RUN: metadata 고정32문항씩 부모/C의 normal generation64회
진입/반환, 그중 각10개 동일 pre-error prefix의 own forward60회 진입/반환.
C32의 tokens/EOS/error는 기존 raw와 모두 같았다. full/cached argmax20쌍 일치,
최대 logit 차이0.00002193450927734375; 선언 abs1e-4+rel1e-3 이내다. 확인한
범위에서 cache 결함은 재현되지 않았다. C의 첫 오류에서 틀린 token이 gold보다
높은 logit을 가졌다는 관측이며, 특정 망각 기제를 원인으로 확정하지 않는다.
진단 command elapsed13.9488005초; 컴파일/회귀 시간과 별개다. SMALL updates0.

R4는 기존 native runner/판정/재개를 확장한 단일 T-SCREEN 등록 경로다.
같은 부모/Adam/T corpus/6:2 tape/default loss/LR1e-4를 검사하고, original parent
raw에서 metadata224 패널을 도출한다. 등록 파일은 child보다 먼저 발행하며,
불완전한 등록·중단·실패를 새 실행으로 바꾸지 않는다. 32/64/128 및 조건부256
screen의 판정을 같은 immutable decision과 terminal에 연결한다. 품질 중단은
command Failed/비재개로 남고, 순수 report만 정상 연구의 조기 종료와 실행 오류를
구분한다. SMALL512 또는 최종 후보 성공으로 승격하지 않는다.

신규 정책 unit1 PASS. 실제 replacement/scope3 proof를 거친 explicit TINY
1+1+1+1 panel의 첫1/fresh1/close/moved-root 거부 PASS. 기존 bridge 연속2와
첫1/fresh1 weight/Adam/tape 정합성도 재확인했다. 이 단계 실제 TINY8회(2회 시험과
최종2+4 회귀)를 더해 누적63/128(standalone3회 포함 정정). 이 절 작성 시 T-SCREEN SMALL은 NOT_RUN이다.

## R0–R2 — bounded bridge 경계 수정, 모델 실행 전

R3-QUALITY-RECOVERY-BOUNDED-BRIDGE-1.0 / 2026-09-19.
시작 HEAD92c6ca1e0f12135c83a4f3bc334039332672bb83, 기준 소스
e08687eff768dd8b6426accebdfe148d3ce8b556. Rust/Cargo1.98.1, 기존 lock 사용.
기존 C512 실패 및 T0, 원문/체크포인트/raw/로컬 파일을 보존했다.

FB01: kind14와 metrics 변형의 draws/LR/metrics 상한을512로 일치시켰다.
실제 Record→create-new publisher→sync/readback의0/1/2/255/256/257/511/512
합성 capacity 시험이며 optimizer0이다. 513, count 불일치, 잘림, 비유한/
비양수 LR, 위조 bounds와 실제 정책에 다른 LR는 거부한다. 기존 decoder의
257 및511 실패를 각각 실제 Record/publisher 경로에서 재현했고(이전 LR 상한
격리 주입, optimizer0), 수정 후 두 직접 unit 시험이 통과했다.

FB02: 등록 observation root의 canonical 경로를 observe/prepare/report에서
공통 검증한다. TINY도 실제 replacement 등록과 별도 teacher 파일/scope3
proof를 사용한다. 복사본 B의 세 진입점 거부, 원 위치 A 별칭의 정상 실행,
읽기 전용 보고, teacher 실패 보존, 첫1+fresh1/연속2의 기존 모델·Adam·close
회귀가 실제 통과했다. 직접 process 시험4개 뒤 실행한 관련 quick은
15명령/18개 고유 테스트 PASS(0-test 없음). 해당 quick의 TINY47회(native44+
standalone3)와 선행 직접 process8회를 합쳐 이 단계 TINY55/128, scalar0이다. 반복 invocation을
새 고유 테스트로 더하지 않는다. 강제 종료 시험의 미확정 생성/teacher tail은
UNKNOWN이며 SMALL 사용량과 합치지 않는다.

초기 quick은 새 unit fixture 두 개가 같은 프로세스의 취소 핸들러를 중복
등록해 FAIL했다. CLI fixture를 각각 별도 process에서 실행하도록 수정한 뒤
위 quick 전체가 통과했다. 이 실패는 보존하며 PASS 숫자에 넣지 않는다.

C512 독립 감사 경로를 기존 native loader/scorer/teacher 순수 verifier에
연결했다. 과거 terminal/command를 만들지 않는 별도 진단이다. 이 절 작성
시점 SMALL updates/generation/diagnostic forward 모두0, C 감사와 T-SCREEN은
NOT_RUN이다. 품질·H3/S4/S5/S6/Goal1 상태도 변경하지 않았다.

허용된 신규 로컬 증거 root:
`artifacts/quality-recovery-bounded-bridge-20260919/`.
기존 실패 원본은 아래 B4/B5 절의 경로에 그대로 보존한다.

## B4/B5 — C512 terminal 발행 실패, T와 pair 비교 차단

R3-BRIDGE-EVIDENCE-RESTART-1.0 / 2026-09-19. **RESULT=PARTIAL**.
ER01_PLAN_REGISTRATION=PASS, ER02_TEACHER_EVIDENCE_BOUND=PASS,
ER03_REPORT_READ_ONLY=PASS, PRODUCTION_LAYOUT_E2E=PASS는 B1/B2 직접 회귀의
판정이다. B3 replacement-01 부모 관측/승인도 정상 완료했다. 실제 SMALL은
C512/T0이며 **STUDY_STATE=FAILED_PUBLICATION_AFTER_C512**다.
정상적인 양군 완료 또는 STUDY_COMPLETE_QUALITY_FAIL로 부르지 않는다.

SOURCE_COMMIT=EXECUTION_SOURCE_SHA=
`e08687eff768dd8b6426accebdfe148d3ce8b556`. 실행과 최종 보고 사이 source/test/
lockfile 변경0; 계산·저장 코드를 동결한 그대로 유지했다. B3 report commit은
`5aff3ad72d7dcafa85c62b0d8371a5a4b116b7a7`이며 각각 정상 push와 원격 전체 SHA
일치를 확인했다. 이 B4/B5 절은 이후 report-only 변경이다. 실행 바이너리 hash는
`a0db28bed4d8b4ff93d6495a66c16088e11b117538c6ef827ab011a9a0a8d0c3`로
보존 사본과 현재 release가 동일하다. 후보 diff의 기준 source는
`2a1010105e0914e4c84cc94b7da707171baaecbe`다.

### 실제 실행과 실패 경계

`c-first-update.log`: 실제 첫1회, step24311 저장, input2698/target236,
TimePause/정상 command receipt, exit0/wall9.65초. 새 process의
`c-resume.log`는 이어서511회 실행했다. +256(step24566)과 +512(step24822)의
native가 실제 저장됐고 최종 모든 generation 패널 및 자기 teacher64가 반환됐다.
그 뒤 terminal 직렬화가 `corruption: R3ER: count bound`로 실패했다.
명령 exit1/wall993.70초. segment-01에는 terminal/command/성공 close 증명이 없다.

SOURCE_READ + 실제 실패 로그의 원인 연결: `SegmentReceipt::decode`는 LR trace가
있고 objective_metrics가 없는 kind14에 최대256개를 허용한다. bridge는 이
kind14를 쓰며 이번 새 process에서 실제 LR511개를 모았다. writer가 같은 decoder로
검증하므로 terminal 발행 전에 거부된다. draws의 최대512와 LR bound256이 다르다.
직접 TINY 첫1+fresh1은 이 크기 경계를 통과하지 않아 이를 잡지 못했다.
이 원인 분리는 소스/로그 근거이며 별도511행 parser mutation을 실행했다고 하지
않는다. 후속 수리에는 256/257/511/512의 실제 writer→reader 회귀가 필요하다.
이번 동결 소스의 이 추가 결함은 **미수정**이며 남은 학습 예산을 재개 권한으로
사용하지 않는다. 이 저장 결함을 낮은 모델 점수의 원인으로 단정하지 않는다.

fresh `anchor-report`도 `MODEL_PAIR=FAILED_OR_INCOMPLETE ARM=C-COPYMATCH`,
candidate=false/GOAL1_ACCEPTED=false로 exit1(wall1.17초)했다. 전체 증거 root의
경로·유형·길이·SHA가 보고 전후 동일했다. 누락 terminal을 보충하지 않았고,
segment-00으로 되돌려511회 재실행하거나 T/replacement-02를 시작하지 않았다.
후속 arm의 같은 `arm_commands` 차단은 SOURCE_READ; T 실행 명령은 NOT_RUN이다.

### 같은 저장 모델에서 나온 관측 — close 수용 아님

부모 기존 패널은 같은 모델의 보존 raw 재집계, 부모 새dev는 이번 새 관측이다.
C 값은 저장 step24822의 실제 생성 후 기존 scorer 출력 및 terminal 직전 raw
재읽기 경로에서 나온 관측이다. **새 process의 완전한 endpoint/teacher close
재검산은 terminal 부재로 BLOCKED**다. T와 paired 비교를0점으로 채우지 않는다.

| 패널 | 부모 A75-R24310 | C512 관측 | T512 |
|---|---:|---:|---|
| 기존 dev 전체 |240/256|6/256|NOT_RUN|
| 기존 dev entity/event/오류 |247/250/1|78/83/4|NOT_RUN|
| CROSS 전체 |462/512|4/512|NOT_RUN|
| CROSS entity/event/오류 |499/500/0|123/171/11|NOT_RUN|
| ordinary QA |181/336|188/336|NOT_RUN|
| auxiliary |53/64|24/64|NOT_RUN|
| 새 dev 전체 |0/256|4/256|NOT_RUN|
| 새 dev entity/context/value/event |15/6/22/2|95/221/64/23|NOT_RUN|
| 새 dev 오류/EOS/base4 |2/255/0 of64|4/256/0 of64|NOT_RUN|
| conditional144 |이번 부모 재생성 NOT_RUN|0/144, 오류5, EOS143|NOT_RUN|

C 중간256은 새dev5/256, 오류29, base4=0/64, watch14/32였다. 최종512는
watch20/32이며 ordinary의 subset 재사용이다. 중간점을 최종 후보로 고르지 않았다.
최종 새dev의 current/past·양순서 각각의 점수와 첫 오류/전체 오류 세부는 원
`c-resume.log`에 보존했다. C→T paired gain/loss, question-only/value-swap
pair correctness 및 candidate48 parity는 NOT_RUN_BLOCKED다.

자기 teacher는 train32/dev32의 동일 metadata 표본이다. 부모 train
NLL3.4527065266943033→C0.20622504710091014, token736/1229→1126/1229,
foil margin1.2929080929607153→7.28656492382288. dev는
NLL4.0264493192556605→0.3387305017796899, token708/1268→1115/1268,
margin−0.6523758731782436→−0.06975007429718971이다.
teacher token 개선을 자유생성 정확도로 바꾸지 않는다. QA+7과 기존 dev−234,
CROSS−458, aux−29가 동시에 관측됐다. 새 입력의 완전 답변 일반화는 미해결이다.
T가 없으므로 동일 대상 경쟁 교육의 상대 효과는 **UNRESOLVED**이며 특정 모델
수식/tokenizer/망각 기제를 원인으로 확정하지 않는다.

### 실제 사용량·마지막 저장·보존

DERIVED_FROM_EXISTING_LOGS: 각 실제 update 로그의 연속 step 및 단일 LR bits를
검사해 C512, input1,349,176/target132,892, LR bits4547007122018943789
(actual1e-4)를 재합산했다. 등록 tape는 anchor3072/focus1024, 각 focus view2회다.
SMALL512/1024, TINY92/128, scalar0. 추가 SMALL parity0.
새 SMALL generation의 완료 패널 행은2176=부모320+C중간288+C최종1568,
반환 generation tokens75,647=부모8,577+C67,070. 최종 watch의32행/1,017tokens는
ordinary에서 재사용했으므로 이중 계산하지 않았다. 완료 자기 teacher128=부모64+C64.
이 수치는 로그의 완료 panel/teacher 반환 합계이며 누락 command receipt를 새로
만든 값이 아니다. C finalization의 canonical 총 시간/전체 사용량 확정은 UNKNOWN;
TINY 강제중단 시험의 미확정 tail과 과거 원 부모 관측 UNKNOWN도 그대로 남긴다.
TINY generation/teacher의 전체 호출 합계는 확정하지 않으며 개별 process의 entry/
returned/final 로그와 분리한다. 이를0으로 쓰거나 SMALL 집계에 합치지 않는다.

아래7개 setup/순수읽기/모델 명령의 shell wall 합은1118.85초(B3 네 명령114.33,
C9.65+993.70, 실패 pair report1.17)다. 모델 호출 시간만의 benchmark가 아니며
빌드/회귀·후속 파일 검사 시간을 포함하지 않는다. C 프로세스 wall을 누락된
정상 command의 canonical elapsed로 대신하지 않는다.

LAST_DURABLE_NATIVE=
`artifacts/bridge-evidence-restart-20260919/study/C-COPYMATCH/segment-01/step-0512.r3m`.
물리 SHA256=`ce01daec7d7c0abeb99ad7e166cfca11668a3e0444817c75f8aa1c4fb28ef624`,
실제 weight hash=`ef7962e07f3a8a7f968291c1ff3a51138555b722521155164ac17f65bbbe440e`,
native model content ID=`6abb23fe0e6ea91646a34f8e61fa34a7c15ad2f5c1b29b4837cf51d1f0cfd6ca`.
새 process의 기존 `model inspect`는 exit0으로 step24822/9,605,184 parameters와
weight hash를 확인했다. generation/teacher/update0이며 metadata JSON은 기존
stdout 보고 형식이다. native header/model content ID와 weight hash는 다른 정의다.
새 process Adam 포함 전체 resume 검증은 NOT_RUN_BLOCKED, 채택 가능한 endpoint나
정상 resume 후보가 아니다. ACTIVE_OWNED_TRAINING_PROCESSES=0.

원 관측 파일 전체와 지정 corpus/부모 파일 hash 재검증 OK, 기존 UNKNOWN/실패/원문/
로컬 untracked 작업 유지. 신규 canonical 파일은 기존 typed R3ER/R3MODEL이다.
원 JSON corpus 자동 읽기/새 SQLite 호출 추가 없음은 SOURCE_READ; OS syscall은
미계측이며0회 실측으로 주장하지 않는다. report JSON stdout과 checker summary는
canonical 실행 상태와 구분한다.

### 실행 증거와 최종 판정

인가된 로컬 원자료 root는 `artifacts/bridge-evidence-restart-20260919/`다.
`replacement-01/`은 이번 정상 부모 관측, `study/C-COPYMATCH/segment-01/`은
saved256/512 및 dev/cross/ordinary/watch/legacy-dev/conditional raw,
`study/C-COPYMATCH/final-probe-*`는 endpoint teacher 별도 파일이다.
`study/T-TEMPORAL/`에는 등록만 있고 실행 segment가 없다.
`executed-replica-train`, `c-first-update.log`, `c-resume.log`,
`b5-failed-pair-report.log`, `b5-native-inspect.log`, `b5-known-usage.log`,
`b5-{before,after}-report*`, `b5-*-verified.txt`를 보존했다.
`recount_logged_usage.rs`는 이 로그의 합계만 읽는 Rust 산술 도구이며 ignored
artifact다. binary raw 재채점기나 정상 certificate를 대신하지 않는다.

최종 동결 source의 `quick --bridge-receipts`:13명령/14고유테스트 PASS,
release build PASS. 구체 명령/exit는 `final-quick/summary.json` 및 각 stdout/
stderr에 있다. 이번 저장 결함 때문에 회귀 통과를 end-to-end SMALL512 성공으로
확장하지 않는다. 실패 이후 새 학습/생성/teacher/회귀 optimizer 호출0.

REPAIR_VERIFIED=ER01/ER02/ER03 직접 범위, TRAINING_COMPARISON_COMPLETE=false,
BINDING_LEARNING_SIGNAL=NOT_ESTABLISHED, MODEL_QUALITY_RECOVERED=false,
H3=NOT_PASSED, H3_SEAL=NOT_OPENED, S4/S5/S6=NOT_PASSED,
GOAL1_READY=false, INDEPENDENT_ACCEPTANCE=NOT_RUN.
REMAINING=kind14 LR trace 크기 경계 수리·직접 회귀, 정상 C/T 비교/close 및 품질 조건.
이 시도는 저장 실패로 닫혔고 어떤 미사용 budget도 자동 실행하지 않는다.
NEW_PERMANENT_FILES=0; 원 모델/corpus/raw/임시 산출물은 게시하지 않는다.

## B3 — replacement-01 부모 관측·순수 검증·C/T 등록 완료

EXECUTED_THIS_RUN. 실행 source는
`e08687eff768dd8b6426accebdfe148d3ce8b556`, 정상 push 후 원격 전체 SHA 일치.
release 빌드18.13초 PASS, 바이너리 SHA256
`a0db28bed4d8b4ff93d6495a66c16088e11b117538c6ef827ab011a9a0a8d0c3`.
R3ER execution source digest
`50edf5c171c1028d4cfa3aadaa547c87f91b53440d1678b5c92635346625f8b2`.
checker의 전체 source digest와 R3ER의 포함 파일 digest는 서로 다른 정의다.
이후 source/test/계산·저장 바이너리는 고정하며 보고 문서 commit은 분리한다.

`artifacts/bridge-evidence-restart-20260919/replacement-01.r3er`에 단 한 번
등록했다. 원 `quality-first-bridge-20260919/parent-observation`은 그대로
FAILED_PUBLICATION/INTERRUPTED_UNKNOWN/NOT_AUTHORIZED_TO_RESUME다.
기존 C/T/sanity 물리 hash와 순서·내용·target token·prompt 길이 일치,
제외된 근거0·상충 target0을 기존 native 자료에서 재검증했다. 재생성0.
부모는 원 migrated A75-R24310 물리hash81d180…6142, model dfc3ef…1b11,
Adam fb0dc945…3e19로 검증했다. 원본 전체 보존 hash도 재검증 OK.

새 관측 정상 종료: generation entries/returned320/320, teacher64,
반환 token8577, SMALL optimizer0. sanity0/64(생성오류1), 새dev0/256(오류2).
teacher train32 NLL3.4527065266943033/token736/1229,
dev32 NLL4.0264493192556605/token708/1268. 이는 아직 학습하지 않은 새 focus다.
완료 elapsed lower bound39.704044833초, publication 이후 command39.719602542초,
프로세스 실제 wall41.67초. 정상 final/proof가 존재하고 UNKNOWN_TAIL=false.

새 process pure report: teacher64의130개 파일 binding 검증,
원 실패 관측과 normal token/EOS/error parity sanity64/64+dev256/256.
보고 전후 전체 경로·유형·파일 크기·SHA 동일, 신규 generation/teacher/update0.
과거 실패 command를 성공으로 바꾸지 않았다. 보고 actual wall4.36초.

실제 bridge-prepare에서 두 군 등록 완료, 신규 optimizer0.
동일 parent/Adam, actualLR1e-4(bits4547007122018943789), first weight8,
6:2 batch8, 각512회·focus1024노출/각view2회. 각 군 등록된 실제 tape 예산은
input1,349,176/target132,892로 상한 내이며 미래 step24822 메타데이터를
검증했을 뿐 미래 weights는 만들지 않았다. 상속 config.lr=0.00003와
실제 고정 LR1e-4를 구분해 로그에 남겼다. 등록 실행 wall42.48초,
학습 정책 준비 wall25.82초. 네 명령 wall합114.33초, 빌드/회귀 제외.

근거: `b3-register.log`, `b3-observe.log`, `b3-pure-report.log`,
`b3-prepare.log`, `b3-{before,after}-report*`.
B1/B2 repair=PASS, B3 observation/admission=PASS, C/T 학습=NOT_RUN,
SMALL0/TINY92/scalar0, 새SMALLgeneration320/teacher64.
LAST_DURABLE_NATIVE=기존24310, H3/S4/S5/S6/Goal1=NOT_PASSED.

## B0–B2 — 관측 증거 경계 수리와 교체 시도 준비

R3-BRIDGE-EVIDENCE-RESTART-1.0 / 2026-09-19. 시작 HEAD
5833ee9f6790b7c0c9489b648e9f4f2e0b229acd, 기준 source
2a1010105e0914e4c84cc94b7da707171baaecbe. Rust/Cargo1.98.1,
기존 lock/offline/Accelerate threads1. 이전 untracked 로그와 원 관측 파일 보존.
로컬 실행 증거: `artifacts/bridge-evidence-restart-20260919/`.

ER01: plan 루트 immutable 등록 후에만 child/start/model load에 진입한다.
등록은 plan/source/binary/model/attempt/안전한 상대 경로/예정 사례 순서를 묶는다.
새 process에서 child 이동·누락 및 등록 공개 후 sync 실패를 차단한다.
`er01-red.log`는 수정 전 실제 재실행 허용으로 FAIL,
`er01-green.log`는 수정 후 같은/다음 모델 진입 차단 PASS다.

ER02/03: 별도 teacher collector와 순수 verifier를 분리했다. start/entry/row/final,
native 실제 file/weights/tokenizer/architecture/step, 고정 metadata 순서,
prompt/expected와 finite 수치·token 범위를 검증한다. 새 scope3 proof는 전체
파일 참조를 명시적으로 묶는 R3ER kind27이다. 구형 proof에 binding을 자동 보충하지
않는다. parent/endpoint 보고와 승인에서 같은 verifier를 사용하며 보고에 collector
호출이나 누락 파일 복구가 없다. 기존 scalar bits, 모델/코퍼스/토크나이저 포맷 유지.

`b2-e2e-first.log`: 실제 TINY 별도 teacher2·generation2 → scope3 proof/final →
fresh pure report → prepare → 두 arm 첫1+fresh1 → close/pair report PASS.
학습4회는 TINY이며 SMALL0. 정상 부모0점을 실행 오류로 취급하지 않았다.
teacher 네 파일 종류의 누락/손상은 report/prepare 거부, 출력 등록 없음,
정상/오류 report 전후 전체 경로·유형·길이·SHA 동일을 확인했다.
`b2-schema-fixed.log`: 실제 TINY2generation/2teacher, 수치/identity19변조 거부,
합성64행·누락/중복 스키마 검사 PASS. 합성 행을 실제 forward로 세지 않는다.

개발 중 실패도 보존: 첫 TINY 분리 시험의 중복 ordinal 및 native content ID와
기존 weight hash 혼동을 수정했다. 각각 모델 호출2+teacher2 후 실패였다.
스키마 시험의 Ctrl-C handler 중복 오류는 모델 호출 전 실패, 필터 오지정 실행은
0-test/NOT_RUN이며 PASS로 합산하지 않는다. clippy 경고와 컴파일 실패도 별도 로그다.
최종 범위 검사 `quick --bridge-receipts`는13명령/14테스트 PASS,
source_unchanged=true, source digest
`2e0f52004908e0fc8adda697df7a0040eeddd2d35e2a59243c036426757b311d`.
fmt/check/clippy, root missing-child/sync, separate teacher 파일의 정상/누락/손상,
취소/teacher 오류/최종 발행 실패, native corpus 두 지정 회귀, 주변 policy 비의존,
기존 pending/단일 writer/부분 패널/LR·Adam·기본 loss 재개를 실제 검사했다.
앞선 범위 검사도13명령/14테스트 PASS지만 최종 재실행과 고유 테스트 수를 중복
합산하지 않는다. TINY optimizer 누계92/128=직접E2E4+앞선quick44+최종quick44.
SMALL/scalar optimizer0. TINY generation/teacher와 강제 종료 UNKNOWN은 각
process log의 entry/returned/final로 구분한다. ER02/03 수정 전 근거는 SOURCE_READ,
수정 후는 실제 위조/누락/정상 process 검사이며 모두 RED 재현했다고 하지 않는다.

교체 등록은 R3ER kind29로 이전 실패 root 전체 파일hash/실행source/보존binary와
새 계약/attempt/source SHA/source digest/binary/예산을 묶는다. 데이터는 기존 파일
hash·membership·순서·C/T target/prompt 길이를 재검증하며 재생성하지 않는다.
새 관측 보고는 과거 같은 모델의 sanity/dev raw token/EOS/error를 대조한다.
최종 release와 source 게시 후 계산·저장 code를 동결하고 승인된 한 번만 진행한다.

OLD_ATTEMPT=FAILED_PUBLICATION/INTERRUPTED_UNKNOWN/NOT_AUTHORIZED_TO_RESUME.
REPLACEMENT_AUTHORIZATION=ONE_EXPLICIT_ATTEMPT, REPLACEMENT_EXECUTION=NOT_RUN.
H3/S4/S5/S6/GOAL1=NOT_PASSED, INDEPENDENT_ACCEPTANCE=NOT_RUN.
신규 영구 소스 모듈0, 기존 파일만 수정. 이후 실제 검사·학습 결과는 별도 기록한다.

## Q6 — candidate 전달, 전체 품질 실험은 미완료

SOURCE_COMMIT=`2a1010105e0914e4c84cc94b7da707171baaecbe`.
main 정상 push 후 `git ls-remote origin refs/heads/main`의 전체 SHA가 동일했다.
이 절은 source 이후 report-only 변경이다.
[candidate source](https://github.com/seoyd/replica-v3/tree/2a1010105e0914e4c84cc94b7da707171baaecbe),
[기준 대비 diff](https://github.com/seoyd/replica-v3/compare/a89fbc977ed9421489a43fc4b4eb6f42a5ebbd25...2a1010105e0914e4c84cc94b7da707171baaecbe).
로컬 diff는 `artifacts/quality-first-bridge-20260919/candidate.diff`다.

최종 release build PASS(15.38초), 바이너리 SHA256
`fd554bdfb6e80ccbab150fd066d32ce5caeaa4b86d703d11ca775a2942dc925c`.
동일 source의 최종 fresh-process admission도 실패 attempt를 거부했다
(`final-sticky-admission.log`, 의도한 exit1, 신규 generation/teacher/optimizer0).
원 부모 관측의 성공 final을 만들지 않았고 신규 study/SMALL checkpoint 없음.
현재 소유 학습 process0, 마지막 durable SMALL step24310,
TINY optimizer122/128, SMALL optimizer0/1024, scalar0.

RESULT=PARTIAL. CK01~03/직접 수리 PASS와 무학습 원자료 검증은 완료했다.
다만 Q3 SMALL 등록 및 C/T 실제 비교는 publication 실패에 의해 차단됐으므로
전체 구현 계약 완료·모델 품질 회복·H3/S4/S5/S6/Goal1 완료가 아니다.
새 파일은 ignored 실험 자료뿐이며 새 영구 소스/문서 파일은 없다.

## Q2/Q3 — 실제 부모 관측 완료, publication 실패로 학습 진입 차단

R3-QUALITY-FIRST-BRIDGE-1.0 / 2026-09-19. **계약 전체 PARTIAL**.
CK01/02/03 수리는 PASS, 자료 생성·native 재읽기·독립 문법 검사 PASS,
TINY 저장/재개 수치 정합 PASS. SMALL C/T 비교는 **NOT_RUN**이다.
코드/원자료 검사, 새 모델 품질 개선, H3/S4/S5/S6/Goal1은 별도 판정한다.

이번 부모 관측에 자체 teacher64를 연결하면서 기존 VerificationFinal reader의
`teachers != 0` 거부 조건을 함께 수정하지 못했다. 실제 generation320/teacher64
이후 positive final 발행 전 `R3ER: verification outcome counters/status`로 exit1.
이는 이번 구현의 저장 경계 오류다. 모델 학습 불능 원인으로 주장하지 않는다.
수정 후 scope3에서만 최대64/완료 시 고정 표본 수를 검증하며, 기존 scope는
teacher0 조건을 유지한다. 해당 TINY process 회귀가 PASS여도 실패한 원 관측의
final을 새로 발행하거나 품질 학습을 자동 재개하지 않았다.

`bridge-prepare`를 새 process에서 호출한 실제 결과는
`verification failed/incomplete/legacy; retry and subsequent arm prohibited`/exit1.
study 디렉터리가 생기지 않았고 C/T SMALL optimizer0, 마지막 실제 durable 모델은
기존 A75-R24310이다. 실행 중인 학습은 없다. pending/final 없는 intent는
INTERRUPTED_UNKNOWN으로 유지한다. generation raw가 완전하다는 사실을
command 성공·총 시간 확정으로 바꾸지 않는다.

### 실제 관측과 한계

모든 새 출력은 동일 부모 model
`dfc3efb664351578340e39d3cfc95b90f270541041d4641d72d6f41a53871b11`,
normal greedy/strict UTF-8/EOS, generation320이다. 추가 SMALL optimizer0,
optimizer input/target tokens0. 원 부모의 Adam/누적 counter는 보존했다.

| 패널 | full | entity | context | value | event | 생성 오류 | EOS |
|---|---:|---:|---:|---:|---:|---:|---:|
| 기존 H3 dev 재집계 |240/256|247|별도 raw|별도 raw|250|1|별도 raw|
| 기존 CROSS 재집계 |462/512|499|별도 raw|별도 raw|500|0|별도 raw|
| 기존 ordinary 재집계 |234/400|253|별도 raw|별도 raw|325|0|별도 raw|
| 새 sanity64 |0/64|2|9|18|7|1|64|
| 새 개발256 |0/256|15|6|22|2|2|255|

기존 ordinary는 QA181/336, aux53/64, watch19/32. 새 sanity A(single)/
B(distinct)/C(same entity)/D(past)는 각각0/16. 따라서 선택 부담을 없앤 A도
실패했다. 시간 선택만의 문제, 특정 kernel/수식/tokenizer 결함이라는 결론은
UNRESOLVED다. 새 C/T training을 실행하지 않아 자료 개입 효과는 알 수 없다.
개발256의 base4는0/64, current/past 각각0/128, 양순서 각각0/128이다.

자체 teacher는 metadata train32/dev32, 총64 forward. 여기의 train32는 부모가
아직 학습하지 않은 **새 focus** 표본이다. 부모의 원래 학습자료 암기 정확도와
혼동하지 않는다. train 평균 사례 NLL
3.4527065266943033, 동일 gold/foil prefix의 최초 차이 평균 margin
1.2929080929607153, teacher token736/1229. dev는 NLL4.0264493192556605,
margin−0.6523758731782436, token708/1268. 이는 자유생성 정답률이 아니다.
sanity 생성 token1678 + dev6899 = 반환 token8577. 실패 attempt reader가 보존한
elapsed lower bound36.3646395초, 전체 elapsed/token 사용량은 UNKNOWN으로 남긴다.
64개의 완료된 teacher raw도 별도로 보존했다. 재검산 신규generation/teacher0.
C/T midpoint/final/paired gain/loss, conditional144 새 평가, candidate48는 NOT_RUN.

### 자료·실행 경로와 보존

부모·B-BASE 각각의 ordinary anchor2048은 evidence0/1/2/3+=66/482/1098/402,
같은 slot 경쟁410, current ID가 큰/작은 경우201/209였다. 기존 focus512는
모두 단일 기록이었다. 따라서 이전 자료 전체에 선택 과제가 없었다고 하지 않는다.
새 focus512는 두 기록/current256/past256, alias 외 C/T 정답 token·prompt 길이가
모두 일치했다. train128base×4, dev64base×4, sanity16base×4; 기존 자료·예약된
seal namespace를 피하며 seal 파일은 읽지 않았다. 작은 부정 사례 검사는
뒤집힌 답·같은 값·동일 ID·time/status 모순·alias 충돌을 거부한다.

실제 prompt token 범위: focus305–371, dev313–368, sanity243–364;
source까지 거리 최대181/179/175. 자동 제외0, 같은 framed input의 상충 답0.
first-target weight8(bits4620693217682128896), 실제 LR 정책1e-4.
새 SMALL 가중치/Adam/tokenizer는 만들지 않았다. source-native 경로의 JSON 읽기와
SQLite 열기 없음은 SOURCE_READ 근거이며 OS syscall 계측0이라는 주장이 아니다.
기존 checker의 JSON 관측 요약, explicit legacy importer, 제품 IPC/운영 SQLite는
그대로 있다. 새 corpus와 학습/평가 제어의 canonical 자료는 R3CORP/R3ER다.

허용 원자료는 repository 아래 다음 경로다. 업로드하지 않는다.

- `artifacts/quality-first-bridge-20260919/data-ready/`: C-COPYMATCH.r3c,
  T-TEMPORAL.r3c, sanity.r3c. 앞선 `data/`와 `q2-data.log`의 sanity empty-train
  packing 실패도 보존했다. model 호출 이전의 실패이며 R3CORP의 두 nonempty
  split 규칙에 맞춰 sanity 파일 train에는 기존 anchor만 유지하고 validation64만
  관측했다. 두 번째 C/T의 semantic hash는 첫 출력과 같다.
- `artifacts/quality-first-bridge-20260919/parent-observation/`: inputs/parent,
  durable start, returned320 rows, sanity/dev 패널, teacher64 rows,
  parent-probe-final, provisional preflight-proof. **preflight-final은 없음**.
- `artifacts/native-corpus-objective-20260918/A75-R24310-v2.r3m`:
  physical81d18002580fd2b662f4fb4c7a7acfd45833b8f0ca1de49a62193b56bb3a6142.
  기존 `durability-pair-restart-20260918/attempt-R/A75-R`와 실제 weights/Adam/step 대조.
- 같은 evidence root의 `q2-parent-observe.log`, `q5-parent-reaudit.log`,
  `q3-sticky-admission.log`, `observation-preservation-{before,after}.sha256`.
  before/after 전체 파일 hash 목록 동일, 이전 원본8개도 재검증 OK.
- 관측 실행 바이너리 `observed-replica-train`, SHA256
  `5c22a933859a3708ab8ffd52b6e22fb57f841de397e0dce1a0b467b9333bdb87`.
  관측 source file experiment_record SHA256
  `1c9469d0142f407ac1295531e0578a69e709e832ba1ebe3a88234651f475824f`.
  관측 후 teacher receipt 수리·읽기전용 보고가 추가됐으므로 최종 candidate와 구분한다.

### 테스트와 최종 상태

Q1의 quick29commands/33tests PASS 이후, 관측 전 q3-quick31commands/35tests
PASS/exit0/source_unchanged=true. q3-quick source digest
`000faf8af706c6f9f0339127f3c568ab95cbb7478ae2226c8463536f289aa956`.
이후 finalization 수리에서 bridge process2tests PASS, fmt/clippy 및 최종 관련
teacher receipt/기존 publication 회귀를 별도 기록한다. 전체 quick을 수리 후
재실행했다고 하지 않는다. TINY 수치 회귀: continuous2와 fresh1+1의 weights,
Adam, LR bits, counter, guard, 전체 raw가 같고6패널 close가 통과했다.
새 teacher receipt 회귀는 TINY 실제generation2/teacher2, fresh read 신규호출0,
동일 attempt 재시도 거부 및 final bytes 불변을 검증한다.

최종 source에서 `quick --bridge-receipts`는 수정한 bridge process/기존 공개후sync/
두 지정 corpus 회귀와 generator를 다시 실행한다. 기존 `quick --native-corpus`
범위는 삭제하지 않았다. TINY128 상한 안에서 변경 경계만 재실행하기 위한 기존
runner의 명시적 좁은 scope이며 전체 저장소 검사나 품질 gate가 아니다.
실제 최종 결과는8commands/6tests PASS/exit0/source_unchanged=true,
source digest `e8862534fc2960c9eb5227b602873a375682b03620263c17ce2850b473bb483c`.
fmt/check/clippy를 포함한다. 수리 직후2tests, 마지막 observation1test 및 기존
publication1test의 별도 호출도 모두 PASS이며 중복 실행을 독립 테스트 수로
합산하지 않는다. 최종 release build는 학습 throughput과 분리한다.

이 작업의 TINY optimizer 실제 합계122/128:
CK01 red1+green3+Q1quick47+bridge smoke4+Q3quick51+
finalization 수리4+publication 재검사4+최종 narrow quick8.
신규 SMALL optimizer0, scalar optimizer0, SMALL generation320/4608,
자체 SMALL teacher64/192, 외부 teacher0. TINY generation/teacher는
각 subprocess의 실제 entry/returned/terminal에 별도 기록하며 강제종료/실패
관측의 UNKNOWN tail을 성공 receipt 사용량0으로 바꾸지 않았다.

최종 요구사항 대조: Q0/CK01~03 및 native 자료·독립 oracle·TINY numeric/receipt
경계 검증 완료. Q2 원 관측의 종료는 실패 보존. Q3 SMALL 등록/endpoint metadata
인가, Q4 C/T1+511·중간256·최종512 학습, Q5 C/T 전이/보존 비교는 미완료다.
정상 완료됐으나 품질 미달인 STUDY_COMPLETE_QUALITY_FAIL로 표시하지 않는다.
새 수리 이후 추가 SMALL 관측/재학습/새 root 생성은 실행하지 않았다.

CODE_VERDICT=IMPLEMENTED_WITH_BOUNDED_VALIDATION,
CK01/CK02/CK03=PASS, Q2_RAW_RECOUNT=VERIFIED,
Q2_COMMAND_FINALIZATION=FAILED_PRESERVED, Q3_SMALL_REGISTRATION=BLOCKED,
Q4/Q5_C_T_COMPARISON=NOT_RUN, MODEL_QUALITY_RECOVERED=false,
H3_PASS=false, H3_SEAL=NOT_OPENED, S4/S5/S6=NOT_PASSED,
GOAL1_READY=false, GOAL1_ACCEPTED=false, INDEPENDENT_REVIEW=NOT_RUN.
NEW_PERMANENT_FILES=NONE. 기존 하네스/codec/RunControl/scorer에 연결했으며
새 Graph/DB/IPC/검증 framework는 추가하지 않았다. 다음 승인 전 자동 연구 재시도 없음.

## Q1 — 세 직접 경계 회귀 완료

EXECUTED_THIS_RUN. `q1-quick`의29 commands(fmt/check/clippy 포함),33 tests PASS/exit0,
source_unchanged=true. native corpus 관련 필터에 실제 기존 caller15개를 추가했고,
두 지정 회귀는 typed Episode·ordered content/physical source 보존을 검증한다.
다른 기존 필드 assertion은 real native reader의 결과를 test 메모리에서만 투영한다.
generator가 JSON을 다시 쓰거나 제품 fallback을 추가한 것은 아니다.
CK01/CK02의 subprocess 및 기존 native default/Span/동일명다른weights/공개후sync/
부분평가 재개 회귀가 함께 통과했다. 수정 전 CK01/CK03 실패 로그는 보존한다.

신규 SMALL optimizer0, 품질 generation0/teacher0. TINY optimizer51/128=
CK01 red1+green3+quick47. CK02 별도 직접 회귀와 quick의 실제 TINY generation은
각27이며, 자체 teacher는 각각25다. 다른 TINY 관측은 각 command 로그에 분리한다.
이 단계의 PASS는 수리 범위이며 H3/S4/S5/S6/Goal1 승인이나 품질 개선이 아니다.
Q2~Q5는 아직 실행하지 않았으며 저장 확장 대신 승인된 자료 대조로 이어간다.

## Q0 — 조건부 기록 선택 대조 시작

R3-QUALITY-FIRST-BRIDGE-1.0 / 2026-09-19. 시작 HEAD91ce87ba3286637e9372f7e70f337b73de1b0aaf,
main/origin=https://github.com/seoyd/replica-v3.git. 기준a89fbc9 이후 변경은 두 상태 문서다.
시작 tracked dirty0, 소유 학습 process0. 기존 untracked 경로 목록과 원본8개 hash
대조는 `artifacts/quality-first-bridge-20260919/q0-*`에 기록했다. Rust/Cargo1.98.1.
신규 연구는 과거 N0–N5/BASE/SPAN의 실패·종료·예산을 재개하거나 변경하지 않는다.

CK01 수정 전 실제 TINY default1step은 주변JSON 없을 때 성공했지만, 같은 bytes의
상위 폴더에 무관한 H3 policy가 있으면 거부됐다(`ck01-red.log`, 신규TINY1).
ambient JSON 탐색 제거 후 없음/무관한valid/malformed 세 위치의 실제1step
weights/Adam/state가 일치했다(`ck01-green.log`, 신규TINY3). OS syscall trace가
아닌 source 검사와 실제 배치 조건의 근거다. SMALL optimizer0.
CK03 기존 binding-pairs 회귀의 NotADirectory 실패를 재현한 후 native reader의
typed Episode/ordered split 비교로 복구했다. 원래 source bytes 보존·질문/값 대조
assertion을 유지했다. 같은 prepare/transform/subset의 직접 caller도 함께 이관한다.

CK02 수정 전은 SOURCE_READ 결함 확인이며 동적 RED 실행으로 부르지 않는다.
실제 TINY subprocess는 첫토큰취소/raw후deadline/teacher오류/final쓰기실패/
공개후sync실패/start후종료에서 새 model1과 재시도를 차단했다. 정상 오답 완주는
다음 모델을 허용했다. plan inode 배타잠금, 반환 raw/teacher entry, pending/final
검증을 기존 binary record/publisher로 연결했다. 첫 관련 실행 TINY generations27,
teacher25, optimizer0 (`ck02-process.log`); 이후 quick 횟수는 별도 실제 집계한다.
현재 Q1 관련 quick 실행 중이며 아직 전체 범위 PASS로 표시하지 않는다.

허용 입력은 기존 durability-pair-restart-20260918/attempt-R/A75-R,
data-binary-target-loss-20260918/study/B-BASE 및 native-corpus-objective-20260918의
A75-R24310-v2.r3m/source-raw.r3c/conditional/이다. 봉인 H3·운영 DB는 탐색하지 않는다.
Q2~Q5 SMALL optimizer/generation/teacher는 이 절 시점 NOT_RUN, 이전 품질 미달 유지.

## N5 최종 결과와 검토 전달

R3-NATIVE-CORPUS-OBJECTIVE-BINDING-1.0 / N0–N5 계약 범위 PASS.
저장·재개 구현 검증과 가중치 고정 진단을 마쳤으며 모델 품질 회복은 아니다.
최종 candidate source=`a89fbc977ed9421489a43fc4b4eb6f42a5ebbd25`.
이 SHA의 정상 push와 실제 remote full SHA 일치를 확인했다. 이 절은 이후
report-only 변경이며 독립 검토/Goal1 승인을 부여하지 않는다.

PATH_PARITY 실행 source=b1565eb26d289086f3d195165a8d1ff5aa1852e6,
조건부 generation source=45d41267d7aae408ca3fd88b61937cad368a42ef,
최종 quick/측정 source=fde0601e799893c357fdfc9c49bbed4616dc317c.
그 후 runtime source 변경은 corpus 변환의 stdout이 이전 manifest 대신 실제 저장한
manifest를 표시하도록 한 수정이다(`5f46807b20113ba2817071fcc998dd853ad38f3c`).
canonical bytes/모델/학습 결과는 바뀌지 않았다.
해당 source에서 corpus 단위2개, 기본 generator50/50 및 subset16/16 CLI→native
reader, clippy/all-targets와 release를 다시 통과했다. SMALL/TINY optimizer 추가0.
마지막 candidate는 기존 test 파일에 같은 이름의 유효한 다른 weights를 넣는 직접
회귀1개만 추가했다. 새 process가 native physical digest 오류로 segment 생성/모델
작업 전에 거부했고, 이름만 바꾼 동일 파일은 내장 default binding 검사를 통과했다.
이 회귀와 fmt/clippy는 PASS, 신규 optimizer/generation/teacher 모두0이며
`n5-same-filename.log`에 보존했다. 최종 candidate의 runtime source는 위5f46807과 같다.
최종 binary=`delivered-runner`, SHA
8e2168d3bb2a13138bdaac3b58e67fd0c156874b866f53b1ab34e7318ff07bc2.
이 CLI fixture는 학습에 넣지 않았다. 원 source/실패/로그와487개 기존 untracked는
보존하며, 원본 checkpoint/corpus/policy 등록8개 hash를 재대조했다.

### 동등 source 저장과 준비 실측

EXECUTED_THIS_RUN. Apple M4, Rust/Cargo1.98.1, offline locked release/Accelerate/F32,
compute threads1. 각 format warm3 및 format마다 fresh-process3, 순서를 순환했다.
fresh process는 OS cache cold가 아니다. 공통 model/reference setup은 약1.3초이며
아래 first-batch-ready에서 분리했다. 소규모3회 관측으로 일반 성능 우위를 주장하지
않는다. 파일마다 create-new/hash 확인/fsync를 수행했고 JSON3개와 native1개의
publication 비용을 구분했다. 측정 도중 원본 보존 hash 검사1회가 겹친 환경의
관측값이며 독점 CPU/OS-cold benchmark로 해석하지 않는다.

전체 train2560/dev256 원문·metadata 비교. ms는 min / median / max다.

| 전체 source | bytes | warm first batch ms | fresh first batch ms | fresh peak RSS MiB min/median/max |
| --- | ---: | --- | --- | --- |
| 원 JSON3파일 | 4,633,037 | 146.817 / 147.052 / 148.200 | 148.407 / 148.762 / 149.304 | 234.44 / 234.70 / 236.13 |
| R3CORP raw | 3,086,269 | 160.943 / 161.541 / 162.677 | 164.039 / 164.493 / 165.304 | 232.02 / 234.23 / 234.58 |
| R3CORP Zstd3 | 365,282 | 146.970 / 147.132 / 148.167 | 148.829 / 149.296 / 150.442 | 228.11 / 228.86 / 229.63 |

별도 train-only cache 비교. source와 정보 범위가 같지 않으며 원문 대체물이 아니다.

| train2560 derivative | bytes | warm first batch ms | fresh first batch ms | fresh peak RSS MiB min/median/max |
| --- | ---: | --- | --- | --- |
| R3TOK raw | 1,726,385 | 10.136 / 10.191 / 11.442 | 10.025 / 10.273 / 10.351 | 213.53 / 214.05 / 214.39 |
| 같은 R3TOK Zstd3 | 158,644 | 8.224 / 8.281 / 8.686 | 8.124 / 8.160 / 8.215 | 213.56 / 213.69 / 215.23 |

각3회 fresh의 분리 단계 median(ms):

| 경로 | read | physical hash | decode+integrity | tokenize | 첫 batch | steady batch | encode | verify | durable publish |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| JSON | .480 | 8.186 | 14.901 | 125.207 | .019 | .0036 | 10.581 | 14.838 | 23.073 |
| native raw | .304 | 5.336 | 32.411 | 126.266 | .016 | .0038 | 20.074 | 32.692 | 9.442 |
| native Zstd3 | .115 | .644 | 24.092 | 124.429 | .017 | .0037 | 17.186 | 23.833 | 9.193 |
| cache raw | .193 | 2.997 | 7.062 | 0 | .016 | .0050 | 7.563 | 4.322 | 8.336 |
| cache Zstd3 | .051 | .280 | 7.813 | 0 | .013 | .0052 | 9.097 | 4.939 | 7.227 |

batch는 같은 등록 tape2개다. 표의 단계 median 합이 ready median과 같을 필요는 없다.
warm RSS는 전체15개 측정이 공유한 process peak260.672MiB로, 경로별 peak가 아니다.
fresh peak도 공통 model/reference setup을 포함한다. sampled RSS는 별도로 raw에 있다.
모든 단계의 warm/fresh min/median/range는 `n5-summary.txt`, 개별 timing/binding은
`n5-warm/measurement.r3er`와15개 `n5-fresh-*/measurement.r3er`, 외부 time -l 원문은
각 .log에 있다. Rust 집계 소스/바이너리는 같은 로컬 evidence root에 보존했다.
JSON/native/cold/cache 전체 logical sample/target/mask/order/role와 실제 batch가
동일함을 매 측정에서 검사했다. native raw는 더 작지만 준비는 느렸고, native Zstd3는
JSON과 비슷했다. cache 개선을 전체 학습 가속 또는 정답률 향상으로 부르지 않는다.
TOTAL_TRAINING_THROUGHPUT_IMPROVEMENT=NOT_RUN.

선택한 보존 집합 합계13,975,727 bytes = 원 JSON4,633,037 + native raw3,086,269
+ native Zstd3 365,282 + cache raw1,726,385 + cold158,644 + completion72
+ 실행 snapshot/policy4,006,038. origin mapping/record index는 native 안에 포함,
dictionary0. 모델·동일 파일의 작업용 복사·진단 raw·benchmark exports는 이 소계 밖에
추가 보존되므로 프로젝트 총 디스크 사용량이 줄었다고 주장하지 않는다. 원본 .r3m
115,285,312→default v2 115,285,632의320-byte 증가는 header/alignment이고 weights
및 Adam 감소가 아니다. 별도 provenance corpus20000/400도 native로 무손실 import했다.

### 요구사항 최종 대조와 판정

| 필드 | 판정·실행 근거 |
| --- | --- |
| CODE_VERDICT | PASS; 관련 quick15개, 추가 replacement process1개 및 최종 CLI/단위2개/clippy/release |
| OBJECTIVE_RESUME_BOUND | PASS; standalone default/Span, unsupported0 optimizer, 실제 process parity |
| LEGACY_MIGRATION | VERIFIED; v1 unknown 거부, A75/S의 정책 증명 이관, v1/v2 raw16/16 동일 |
| NATIVE_CORPUS_DEFAULT | PASS; loader/generator/transform/train/eval/prepare/cache 실제 연결 |
| TRAIN_EVAL_JSON_READS | native source 경로0 (격리 실행+의존 코드 검사); OS syscall 추적은 NOT_AVAILABLE |
| LOSSLESS_EPISODE_PARITY | PASS; F2560/256, provenance20000/400, native 이동/원본 비의존 |
| TOKEN_BATCH_PARITY | PASS; train2560/790401 tokens 및 mask/target/annotation/순서 |
| WEIGHTS_PARITY | PASS; 실제 SMALL2 대1+fresh1, 마지막 durable step24312 |
| STORAGE_MEASURED | PASS; full-source와 train-only cache의 warm3/fresh3 분리 |
| CURRENT_CONDITIONAL_DIAGNOSTIC | COMPLETE_NEGATIVE; 두 모델 각각0/144, 근본 원인 UNRESOLVED |
| NEXT_MODEL_EXPERIMENT | aux anchor replay 단일 가설 PROPOSAL_ONLY, 실행0 |
| H3 / S4 | NO / NO; 기존 미달 유지, 이번 수용 평가 아님 |
| S5 / S6 / GOAL1_READY | NO / NO / NO; seal NOT_OPENED, GOAL1_ACCEPTED=false |
| INDEPENDENT_REVIEW | PENDING |

실행량: SMALL 품질0/경로4, SMALL generation320/teacher288, TINY optimizer108,
scalar optimizer0. TINY 생성/teacher는 각 직접 회귀의 로그로 분리하며 SMALL320에
합산하지 않는다. 과거 실패/UNKNOWN tail은 수정하지 않는다. 기존 원문 기억/SQLite,
graph, 모델 수식/tokenizer/정밀도/optimizer 식은 그대로다. 새 영구 Rust 모듈은
`src/native_corpus.rs` 하나다. 제품 IPC·config 입력·명시 legacy reader·사람용 보고의
JSON은 남아 있으며 전 프로젝트 JSON0이 아니다. 계약 요구사항을 마지막으로 대조했다.

변경 파일: `src/neural/{checkpoint,artifact}.rs`, `src/{training,contrast,data,
native_corpus,train_main,quality_recovery,experiment_record,token_cache,check_main}.rs`,
`tests/experiment_record.rs`와 기존 NATIVE_MODEL/STORAGE_FORMAT/RUNBOOK/
QUALITY_RECOVERY_PLAN/EXPERIMENT_STATUS 문서. 외부 모델/teacher/API나 답변 하드코딩,
신규 범용 framework는 추가하지 않았다.

허용된 읽기 전용 원자료 root는 `/Users/seo/Projects/Replica-v3/artifacts/native-corpus-objective-20260918/`.
원본 부모는 `artifacts/durability-pair-restart-20260918/attempt-R/A75-R/segment-00/step-0512.r3m`,
BASE/SPAN은 `artifacts/data-binary-target-loss-20260918/study/{B-BASE,S-SPAN}/segment-00/step-0512.r3m`,
source JSON은 `artifacts/h3-controlled-20260917/a2/corpus-F/` 및 `artifacts/goal1-corpus-v9/`.
candidate diff는 새 root의 `candidate.diff` (base af638233→sourcea89fbc9),
실행·원자료 위치와 정확한 hashes는 위 절과 `n0-originals.sha256`, `n3-*.log`,
`conditional/`, `n4-*.log`, `n5-*.log`에 있다. 원자료/weights/DB/corpus/target/임시 지시문은
commit/push하지 않는다. 이 보고 다음에 게시되는 report commit은 source와 별개다.

## N4 — 조건부 반응 진단 완료, 품질 회복 아님

EXECUTED_THIS_RUN / DEVELOPMENT / 가중치 고정. 생성 source는
45d41267d7aae408ca3fd88b61937cad368a42ef이며 정상 push/remote 일치 확인.
diagnostic-runner SHA=e0abb73f191b6460b37aabe45f762d4a5dcce0d096f55a4b7f2870f771268e68.
seed19317의24 base×6조건을 출력 전에 binary로 동결했다. plan physical SHA
ece31554726187120095f5efbbf091825583bb50d9262dae2cf1121a0e4517cc.
동일 base의6조건은 종속 표본이며 독립144개 또는 최종 수용 패널로 해석하지 않는다.

| 측정 | A75-R24310 | B-BASE24822 |
| --- | ---: | ---: |
| strict full answer | 0/144 | 0/144 |
| 여섯 조건 공동 성공 | 0/24 | 0/24 |
| v0와 v1/v2/v3/v4/v5 동시 정답 | 각각0/24 | 각각0/24 |
| entity / context / value | 11 / 3 / 13 (각144) | 9 / 0 / 11 (각144) |
| citation exact / provided / nonempty | 2 / 14 / 116 (각144) | 4 / 20 / 115 (각144) |
| EOS / 빈 출력 / 길이 종료 | 144 / 0 / 0 | 142 / 0 / 2 |
| raw strict UTF-8 오류 | 17 | 14 |
| 재채점 오류 (길이와 겹칠 수 있음) | 17 | 15 |
| 첫 오류 entity / context / format | 79 / 30 / 35 | 86 / 29 / 29 |
| 조건부 generation / teacher forward | 144 / 144 | 144 / 144 |
| prompt tokens / generated tokens(EOS 포함) | 49374 / 3911 | 49374 / 4038 |
| teacher target tokens(EOS 포함) | 5313 | 5313 |
| 실제 command 초 | 23.963 | 23.755 |

각 조건 full exact는 모두0/24. v0..v5의 재채점 오류는 부모[0,0,0,1,0,16],
BASE[0,0,1,1,0,13]이다. ID2/4/6/8자리, 반복/비반복 및 방향/숫자 값 strata도
모두 full0이다. 평균 prompt 길이는[325.375,333.375,325.375,325.375,348.375,399.375].
unrelated distractor가 들어간 v5의 길이/오류 증가를 관측했지만 길이와 위치, 문자
조각화, prefix 전파, 질문/과제 전이의 원인을 이 패널만으로 분리하지 못했다.
gold/foil 최초 다른 token의 같은 gold prefix forward에서 gold 선호는
부모[10,15,8,14,8,5]/24, BASE[9,15,11,11,10,5]/24. 제품 응답 정확도가 아니다.

v1/v2 metadata-selected16개의 normal greedy를 각 fresh process에서32회 생성해
raw token/EOS/error까지16/16 일치했다. SMALL 합계: 품질optimizer0,
PATH_PARITY optimizer4, generation320/320, teacher288/288. SPAN 새 생성/학습0.
새 seal 개봉0. 모든 모델 관측은 여기서 종료했으며 추가 호출/자동 연장은 없다.

`conditional/model-0/final.r3er` SHA=f983250de804daaa74955bfec6d6697f23873306bec391fb435f41764a1d06f3,
`model-1/final.r3er` SHA=44d4b6e2c05ce8721a9d89050c4dacafb5c9bb6897dba8d6b87eb81a4450c6cc.
원 raw/계획/step별 entry와 first recount를 보존했다. read-only 재채점기는 실행
source와 자신의 source를 구분한다. 최초 report의 citation_in_provided 분모127/130은
invalid output의 null을 빠뜨렸고 전체144로 수정했다. 원 raw와 primary full0은
변경되지 않는다. `n4-final-recount.log`가 전체 분모/오류 종류 재검산이다.

### 과거 train64와 실제 노출 재집계

DERIVED_FROM_EXISTING_TRAIN_PROBES. 새 forward0. 부모/BASE/SPAN은 각각
2046/2051·2047/2051·2043/2051 teacher token, 전체 teacher59/64·60/64·56/64.
원문과 bound annotation으로 역할 분모를 다시 계산하고 기존 weighted mass와
일치하는지 검사했다. 아래는 비가중 NLL 합/분수 token당 평균이다. byte overlap로
한 token이 역할 사이에 나뉘므로 token 수에 소수가 있다.

| 역할 | token 수 | 부모 합/평균 | BASE 합/평균 | SPAN 합/평균 |
| --- | ---: | --- | --- | --- |
| format | 1215.1 | 6.175569 / .005082 | 6.278571 / .005167 | 7.625106 / .006275 |
| entity | 327 | .072496 / .000222 | .100895 / .000309 | 1.115523 / .003411 |
| context | 170 | .057849 / .000340 | .075426 / .000444 | .084998 / .000500 |
| value | 67.9 | 6.296661 / .092734 | 6.302240 / .092816 | 6.591218 / .097072 |
| citation | 217 | .581407 / .002679 | .538964 / .002484 | 1.313220 / .006052 |
| status | 54 | .008506 / .000158 | .006427 / .000119 | .006902 / .000128 |

첫 target weight8을 적용한 mass는[1278.1,670,170,67.9,217,96], 총2499이며
실제2051 target 분모와 다르다. weighted NLL 합과 weight-mass당 평균은
`n4-existing-probes-means.log`에 별도로 보존했다. format 합이 value보다 압도적이지
않고 value 평균이 높지만, 이미 실패한 SPAN 반복이나 새로운 loss의 근거가 아니다.
gold 접두어의 teacher59/64와 새로운 다중 기록 free0/144는 같은 테스트가 아니다.

B-BASE의 같은512-step tape에서2048 ordinary anchor views가3072번,128개 focus base의
4개 view(512 pool)가1024번 소비됐다. target 노출은 anchor93663/focus38124였다.
category/family/task별 unique base/draw/token은 `n4-exposure.log`에 있다.
aux의 entity-cue 과제는 이 pool에0건이며 부모53/64→BASE22/64 하락과 함께
관측됐다. 평가 사례가 train에 없는 정상 분리와 필요한 과제 자체의 pool 부재는
구분한다. 이것도 H3 실패 원인 또는 망각의 단독 증명은 아니다.

NEXT_MODEL_EXPERIMENT=PROPOSAL_ONLY: aux task 보존을 검증하는 anchor replay 하나.
부모는 동일 A75-R24310, model/tokenizer/Adam/LR1e-4/6:2/기본loss는 고정한다.
대조는 현재 anchor6; 처리군은 그중1slot을 기존 **train** entity-cue pool로 교체한다.
새 진단/heldout 문장을 학습에 복사하지 않는다. 먼저 그 train pool과 annotation을
명시 검증해야 하며 없으면 BLOCKED_INPUT이다. 제안 예산은 각128updates 이하,
각 input350000/target40000 tokens 및 command1800초(정리120초), 자동 연장0.
취소/수치/자료/저장 오류는 즉시 중단한다. aux가 부모53 이상이면서 대조보다4 이상
높고 QA178 이상, 같은 endpoint dev/CROSS가 대조보다 낮지 않아야 보존 가설을
지지한다고 판단한다. 별도 H3/S4 최종 gate를 낮추거나 제품 채택을 허가하지 않는다.
이번 실행 권한과 실제 신규 품질updates는0이며, 조건부 실패의 근본 원인은 UNRESOLVED.

최종 관련 quick(`n5-quick-final`)는 현재 source의 fmt/check/clippy 및15개 직접
단위/process 회귀 모두 PASS다. 초기 실패 quick와 별도 기록이며 전체 저장소 테스트를
실행한 것은 아니다. 이 재검증까지 TINY optimizer108/128, scalar0. release 성공.
최종 read-only/측정 binary SHA=74cb9c6b932ed0f4055a5c1f3b1951bf526be0c20dff677e3e719716c6bc4766.
N5 동등 저장 측정은 이 절 시점 NOT_RUN이며 SMALL/model 호출 예산은 모두 종료됐다.

## N3 — 실제 SMALL 입력 경로 동등성 완료

EXECUTED_THIS_RUN. 구현 source b1565eb26d289086f3d195165a8d1ff5aa1852e6를
정상 push하고 원격 SHA 일치를 확인했다. 이 source의 고정 native-runner로
legacy-reference2와 native/cache1→새 process1을 실행했다. 신규 품질 SMALL0,
경로 검증 SMALL4/4, generation0/320, teacher0/288. 각 경로 입력5135/target541,
실제 LR bits4547007122018943789 (1e-4), 마지막 durable step24312.
연속/재개 weights=3aecde88cf8593552229bf3ad4a04037965a60941cca917c2e454d7132ae48e9,
Adam=bc9a00b1a9ba915d539aa12a3a3e2e7b2770d8aff309fc32b5011ab0de85374c가 일치했다.
tokenizer·step·sampler·loss·config·consumed/target counters도 정확히 비교했다.
각 descriptor를 해당 정책과 검증한 뒤 실행 방식/경로를 묶은 policy/provenance
차이만 비교에서 분리했다. 이 endpoint는 품질 후보/후속 학습 부모가 아니다.

native branch는 원 JSON 경로가 없는 owned native source/cache/parent/snapshot만
소비했다. raw sample2560 및790401 tokens 전체 token/response/target/mask/annotation
동등성이 통과했다. 실제 tape batch 검사는2개다. 초기 compile 로그의 고정
`ALL_SAMPLES_AND_5_BATCHES`는 이 짧은 tape에서 부정확했고 표시를 실제 count로
수정했다. legacy-reference의 `PATH` 줄은 JSON_READS=3이며 기존 공통 SEGMENT
footer의 JSON_READS=0 문구는 잘못된 표시였다. footer를 explicit legacy 경로
여부로 수정했다. 원 로그는 수정하지 않았으며 계산/저장 결과는 바뀌지 않는다.

원자료: root의 `path-parity/legacy-reference/segment-00/final.r3m`,
`path-parity/native/segment-00/final.r3m`, `segment-01/final.r3m`와 각 binary
inputs/command/terminal/comparison 및 `n3-*.log`. native source semantic
3a2625940a67877f3f305718166f5e2638f99b562e1207cd887de45b92952365.
표시 수정 뒤 추가 SMALL 재실행은 하지 않는다. 허용 optimizer4회는 모두 사용했다.

## Native corpus / objective binding — 구현 경계

R3-NATIVE-CORPUS-OBJECTIVE-BINDING-1.0. 기준 source
af63823346ebb6e11cc23451a8ba0c704eb34039, 시작 report HEAD
5c35c8c452f6d6790f121af2e2a9e018cc01641c. 기존 tracked dirty0 및
untracked487개를 보존했다. 새 근거 root는
`artifacts/native-corpus-objective-20260918/`이며 원본이나 실험 종료 상태를 바꾸지 않는다.

EXECUTED_THIS_RUN: fmt/check/clippy/release 성공. 관련 단위10개 및 직접 process5개
통과. 초기 quick는 v1 golden/version/config fixture3개에서 실패했고 수정 후 통과했다.
두 번째 quick는 JSON deny sandbox의 macOS setuid ps 제한으로 자원 관측이 실패했다.
RSS gate를 끄지 않고 JSON 없는 격리 폴더의 compile/train/fresh resume/eval로 검증했다.
따라서 OS 수준 전 시스템 open 추적 증거는 없으며, 실제 native 경로 실행과 loader
의존성 검사를 구분한다. 새 binding으로 preflight 연속/분할 정책 값이 달라지는 비교도
각 정책 검증 후 실행 수치 비교로 수정했고 post-link 오류 차단 회귀를 재통과했다.
전체 quick PASS로 합산하지 않는다. 실패를 포함한 TINY optimizer64/128, scalar0.

N1: standalone SPAN에 인접 정책이 없을 때 기존 binary는 tensor 준비까지 진입했다
(없는 corpus에서 중단, optimizer0). 새 v1은 LEGACY_OBJECTIVE_UNKNOWN, 새 v2 SPAN은
OBJECTIVE_POLICY_UNSUPPORTED로 tensor 준비/optimizer/성공 출력 전에 거부한다.
기본 및 nonzero SPAN TINY 각각 연속2와 fresh1+1의 weights/Adam/state가 일치했다.
실제 원본의 policy/native/terminal/raw close를 검증한 migration은 default와 SPAN 모두
성공했다. inference graph와 tokenizer 및 Adam/clock을 변경하지 않는다.

| 원본 | 새 v2 physical SHA | 동일 model SHA |
| --- | --- | --- |
| A75-R24310 | 81d18002580fd2b662f4fb4c7a7acfd45833b8f0ca1de49a62193b56bb3a6142 | dfc3efb664351578340e39d3cfc95b90f270541041d4641d72d6f41a53871b11 |
| S-SPAN24822 | 883af65bb658923197df0bfa0d1b44e06239c209af3f160656a94ec7123d304b | 8536fc7d57c27920ed5095bbdf0c79c0bb9bdece9ad851675f2d3a10101b363f |

N2: native full source codec/import/loader/generator/transform 연결, corpus-F와
goal1-corpus-v9의 원문·순서·train/dev 전체 동등성을 실제 검사했다. 원 JSON은 역사 자료다.
N3 TINY: native 일반 trainer와 cache-backed native run 모두 연속2/fresh1+1 통과.
N3 SMALL4, v1/v2 생성32, N4 조건부 생성288/teacher288, N5 측정은 이 절 시점 NOT_RUN.
quality SMALL0이며 기존 H3/S4 미달, S5/S6/Goal1 미완료는 유지한다.

고정 release binary SHA=ec3dccc0055b24a24d1412b36a1a0b9aa61d41fd8a584e6f8c1e5d714ea005dc.
실행 경로는 root의 `native-runner`, 현재 diff는 `n5-tested.patch`와 새 native_corpus.rs의
별도 해시로 기록했다. 원자료와 binary는 git에 올리지 않는다. 후속 실행 결과는 다음
보고 절에서 실제 counts/hash와 함께 갱신하며 이전 실패 파일은 보존한다.

## 최종 게시와 검토 전달

최종 candidate source=`af63823346ebb6e11cc23451a8ba0c704eb34039`.
기준 source=`1e25be2cef801dd67a6515e5c33823a0d1b06a92`.
main/origin seoyd/replica-v3 정상 push 뒤 실제 remote full SHA가 candidate와 일치했다.
이 절은 이후 report-only 변경이다. 학습 source는 별도의
`414e1d3434224be1007e9717cb339109792dd333`이며 cache source로 소급 변경하지 않는다.

검토용 diff=`artifacts/data-binary-target-loss-20260918/candidate.diff`
(위 base→candidate, 관련 source/tests/docs만). 허용된 읽기 전용 원자료와 증거 root는
`artifacts/data-binary-target-loss-20260918/`다. study/B-BASE 및 S-SPAN의 inputs,
segment-00 native/raw/command/decision/comparison, train probe 원본, cache와
e1/e2/e3/e4/e5/e6 로그를 보존했다. 원 corpus 경로와 부모 SHA는 아래 절에 명시했다.
원문·checkpoint·corpus·로그·임시 지시문·target은 git stage/push하지 않았다.
candidate 이후 tracked source dirty0이며, 기존 untracked 자료는 보존했다.
추가 SMALL0/새 품질 gate0. 독립 검토 승인이나 Goal1 완료는 부여하지 않았다.

## E5/E6 — 저장 원형 검증과 최종 대조

R3-DATA-BINARY-AND-TARGET-LOSS-1.0 / 2026-09-18 / RESULT=PARTIAL.
저장·수리 구현과 승인된 모델 비교를 완료했지만 모델 공동 품질은 실패했다.
E4 report b771350ccc94c0baf6643290c3dd1387ac704c97 정상 push/remote SHA 확인.
E5는 학습 종료 이후 source다. 모든 기존 raw/corpus/DB/native/종료 기록을 유지했다.

### 저장 범위와 실제 측정

data::load의 원본 manifest/train/validation JSON loader는 남아 있다. native run은
R3ER owned Episodes를 한번 읽어 samples()/annotation을 준비하고 메모리 batch를
사용한다. 매 update JSON parsing을 하거나 이번 연구가 새 cache로 학습했다고 하지
않는다. `.r3m` weights/Adam, tokenizer, 원문 기억·SQLite·graph 형식은 변경하지 않았다.
token_cache.rs는 train-only packed derivative와 한정된 compile/measurement 명령이다.
기존 batch()/Candle와 실제 token/target/mask/role 동등성을 검증하며 기존 run의 준비
경로를 자동 교체하지 않는다. 향후 학습 사용은 새 정책·인가가 필요한 prototype이다.

Apple M4, Rust1.98.1, release/Accelerate, threads1, warm3회/새process3회.
공통 native/tokenizer/reference-sample setup 약1.22초는 별도 로그로 분리했다.
OS cache를 비우지 않았고 fresh process를 OS cold라고 부르지 않는다. 아래는 각3회
관측의 median이며 원 read/hash/parse/tokenize/first-batch 값은 그대로 로그에 있다.

| 경로·내용 | 실제 bytes | warm ready-first-batch ms | fresh-process median ms |
| --- | ---: | ---: | ---: |
| 원 JSON corpus: train2560+dev256+manifest | 4,633,037 | 161.015 | 163.333 |
| R3ER: 전체3728 episode+panels+tape/policy | 4,028,751 | 144.593 | 147.942 |
| R3TOK raw: train2560,790401tokens,u16,희소span | 1,726,385 | 10.211 | 10.352 |
| 동일 R3TOK whole-file Zstd3 | 158,645 | 8.022 | 8.029 |

각 경로는 **같은 train sample/batch**를 재현하지만 저장하는 전체 정보의 범위가 다르다.
JSON↔snapshot↔cache 수치를 같은 원문 전체의 단순 압축률로 해석하지 않는다.
raw/Zstd만 완전히 같은 container bytes의 무손실 압축 비교다. Zstd는 더 작아 cold
산출물로 보존했으며 random access 전에 전체 해제가 필요하다. raw indexed batch5는
약4.5–8.8µs, baseline memory batch는 약3.3–14.3µs였다. startup 개선과 batch/학습
throughput 개선을 구분한다. E4 backward만 각549.7/525.2초였고 tokenization은
초기에 한번 약0.13초였다. 이번 cache로 전체 학습속도나 정답률이 개선됐다는 실험은 없다.

warm read/hash/parse-integrity-role/tokenize median(ms):
JSON .698/8.003/25.623/125.572; snapshot .439/6.870/10.639/126.188;
raw .162/2.961/7.114/0; Zstd .049/.272/7.694/0. parse includes decoder integrity;
Zstd includes decompression. 독립 parity 비교는 측정한 first-batch 이후 수행했다.
각 repetition마다 같은 소비 tape5개를 실제 tensor로 준비해 exact parity도 검사했다.
공통 setup을 제외한 input 준비 비용이며 전체 CLI 시작시간으로 표기하지 않는다.

해당 비교 파일 subtotal: source+snapshot8,661,788B; raw cache+completion을 더하면
10,388,245B; cold도 보존하면10,546,890B. cache 추가로 총 디스크는 증가했다.
모델·이전 실험·로그는 이 데이터 subtotal 밖이다. 별도 보존된 원래 goal1-corpus-v9는
manifest713/train32164769/validation653687B이며 F corpus의 anchor provenance다.
모델 resume file은 기존과 같은115285312B; cache 성과를 weights binary 전환으로
부르지 않는다. cache compile: tokenize/annotation .147755s, encode/검사 .013429s,
compression/복원 .002457s, durable publication .029882s; native setup1.09339s 별도.

측정 binary=`e5-storage`, SHA256
72d5738223eff76321906563fcf481b63acf1f008858010d940f9d68078ff70a;
당시 source digest=b16a74eadcdd5f2104ecbfc94b78e4968f8f6ba98302c74423c4678ef9f01843.
이후 변경은 unequal-target/mixed-fallback scalar 회귀 보강과 문서뿐이며, 측정 코드와
학습 수식은 그대로다. 모든 실측은 해당 보존 binary의 결과로 표기한다.
cache raw SHA=10a4b3fc61f30bfd2c5649199ccce9a01b5a37ba9af7e7c55115aaced41f70cd.
실제 입력/산출물은 evidence root의 study/B-BASE/inputs.r3er,cache/train.r3tok,
cache/train.r3tok.zst,cache/complete.bin이다. source JSON은
artifacts/h3-controlled-20260917/a2/corpus-F/{manifest,train,validation}.json이다.

### 학습 관측 재집계와 한계

고정 metadata train64/2051 targets의 teacher 결과(독립 일반화 점수가 아님):

| 동일 train64 | 맞은tokens | 전체teacher 일치 | 기존 가중 objective | span objective |
| --- | ---: | ---: | ---: | ---: |
| 부모 | 2046 | 59 | .006447 | .006233 |
| BASE512 | 2047 | 60 | .006499 | .006264 |
| SPAN512 | 2043 | 56 | .008177 | .008040 |

모든 첫 오류 역할은 value였고 각각5/4/8개다. 부모 가중 NLL 합에서 format6.17664,
value6.29666이며, entity.10034/context.05785/citation.58141/status.01037이다.
format이 손실을 압도한다는 원인은 지지되지 않는다. SPAN에서는 format7.62606,
entity1.14672/citation1.31322/value6.59122로 증가했다. 이 값은 역할별 NLL 합이며
역할별 token 평균이나 normal greedy 정확도가 아니다. 기본 loss와 span loss는
서로 다른 식이므로 loss 숫자가 작다는 이유로 품질 승리를 부여하지 않았다.
자세한 mass/NLL/first-error/category 집계는 e5-probe-recount.log에 있다. 새teacher0.

근본 원인은 UNRESOLVED. 다음 단일 가설 후보는 현재 anchor pool에서 제외된
기존 auxiliary 과제의 보존 여부다: 새 자료 생성 없이 anchor task 구성 하나를
비교해 aux 하락과 관계를 확인하는 별도 인가를 제안한다. 부모aux53→BASE22/SPAN27,
BASE의 QA181 유지와 구분되는 손실이 근거다. 이것이 H3 실패 원인이라는 결론이나
새 학습 승인은 아니다. 같은 LR/암기 시험으로 자동 되돌아가지 않았다.

### 검증과 최종 판정

E5 cache 직접3 tests PASS: u16/u32/압축/실제 batch,6개 호환identity 및24개 corrupt
mutation/잘못된ID/offset/EOS/trailing,미확정·동시writer 거부. 전체2560 sample과
희소→dense annotation exact parity, warm3/fresh3의 각5batch PASS. clippy/release PASS.
후속 scalar1 PASS는 target 길이2/3의 실제 분모5, padding, microbatch gradient,
finite difference 및 혼합 zero-span gradient 보존을 추가로 확인했다(optimizer0).
quick1회 FAIL과 후속 지정 회귀 성공은 E3 기록대로 구분하며 전체 재실행하지 않았다.
새 SMALL1024, generation2928, 자체teacher192, TINY109로 종료했다.

PF_PUBLISH=IMPLEMENTER_VERIFIED; PARTIAL_RESUME=IMPLEMENTER_VERIFIED;
RAW_RECOUNT=VERIFIED; STORAGE_SCOPE=DOCUMENTED; TOKEN_CACHE_PARITY=PASS;
STORAGE_MEASURED=PASS; OBJECTIVE_IMPLEMENTED=PASS; STUDY_EXECUTED=COMPLETE;
OBJECTIVE_EFFECT=NEGATIVE_ON_PRIMARY_ENDPOINT; H3_JOINT=FAIL;
H3_SEAL=NOT_OPENED; S4/S5/S6=NOT_PASSED; GOAL1=false;
INDEPENDENT_REVIEW=PENDING. 최종 대조에서 등록 원본63개 SHA 전부 일치,
A75-R/K/D endpoint 전체SHA 일치, 기존 untracked487개 누락0을 확인했다.
정책/원문/실패/기존 종료파일을 수정하지 않았다. 관련 fmt/clippy/diff 검사도 통과했다.
임시 지시문은 그대로 로컬에 있고 source/영구문서 의존은 없으며 기존 deny-pattern만
검출됐다. 최종 source/report/remote SHA는 아래 게시 기록과 분리해 확인한다.

## E4 — BASE/SPAN 실행 완료, 공동 품질 실패

실행 source414e1d3434224be1007e9717cb339109792dd333 정상 push/remote full SHA 확인.
위 source와 e4-train binary를 두 군 전체에서 고정했다. 새 SMALL1024/1024,
generation2928/4096(부모16+각1456), 자체 teacher192/256(부모64+각final64),
TINY109/128. 추가 학습·generation retry 없음. 양군 command Complete/STOP=[]이고
최종 native는 각각 step24822,115285312B다. 운영 모델/기존 checkpoint 미교체.

| 같은 final512 모델 | dev full/entity/event/errors | CROSS full/entity/event/errors | QA336 | aux64 |
| --- | --- | --- | --- | --- |
| 부모24310, 기존 raw 재집계 | 240/247/250/1 | 462/499/500/0 | 181 | 53 |
| B-BASE24822 | 229/247/248/1 | 455/486/500/0 | 181 | 22 |
| S-SPAN24822 | 172/226/231/1 | 332/421/452/0 | 170 | 27 |

중간256은 B/S 모두 dev238/entity247/event251/errors1,watch20/19였으며
primary endpoint로 대체하지 않았다. B256 model hash는 이전 K256과 동일하다.
최종 원시 오류도 전체 분모에 포함했다. aux를 QA에 합산하지 않았다.
SPAN은 BASE 대비 dev−57/CROSS−123/QA−11,aux+5이고 부모 대비도 회복하지 못했다.
단일 부모/seed 관측이며 일반적인 loss 우월성이나 근본 원인 확정으로 해석하지 않는다.

각 군 exact input1255342/target131787, anchor/focus draws3072/1024,
unique views2048/512,focus bases128이다. pool별 input963340/292002,
target93663/38124, 동일 actualLR bits4547007122018943789(1e-4), 동일 초기Adam 유지.
최종 weights/Adam/cursor는 raw/native/종료와 검산했고 모델별 실제512를 합계1024
추가학습한 한 모델로 표기하지 않았다.

| actual phase/metric | BASE | SPAN |
| --- | ---: | ---: |
| batch prepare seconds | .015837 | .010704 |
| forward+loss seconds | 180.851960 | 175.540418 |
| backward seconds | 549.656262 | 525.189599 |
| optimizer seconds | 10.348126 | 9.894748 |
| mean gradient norm | .168975 | .235300 |
| mean update norm | .029985 | .041260 |
| clip calls /512 | 5 | 23 |
| command seconds | 986.491213 | 949.129424 |

새 목적함수의 gradient/update 크기와 clip 빈도가 실제 달랐다. 가중치 질량 정규화가
같은 gradient를 보장하지 않는다. 서로 다른 objective 평균을 직접 품질로 비교하지
않는다. teacher64의 공통 CE/역할별 NLL 재집계는 후속 무학습 보고에서 분리한다.
시간은 단일 순차 실행 관측이며 속도 우월성 주장/동시 조건 benchmark가 아니다.
SPAN terminal 뒤 command close가 끝나기 전 report launcher를 시작한 짧은 겹침이
있었고, 그 이후 process 확인에서 report만 남았다. 추가 forward/optimizer/generation
겹침은 없었으나 마지막 close 시간은 격리 성능측정으로 해석하지 않는다.

새 process `e4-recount.log` exit0. 전체 panel을 다시 채점하고 저장 점수와 일치,
native/step/receipt/stop 검증 후 `STUDY_COMPLETE_QUALITY_FAIL`을 확인했다.
paired [둘다오답,gain,loss,둘다정답] B→S:
dev[15,12,69,160],CROSS[46,11,134,321],QA[145,10,21,160],aux[35,7,2,20].
base-all-views B→S:dev[9,3,19,33],CROSS[20,8,40,60],ordinary[65,2,5,28].
부모→B/S 상세 case/base counts는 같은 log에 보존한다.

최종 physical SHA:
B=6e27095cfeb29dcc986db80867b030576989f030797c834aa44212601a313f4c,
S=253345504cf412134914f21737415192db7d8fe37a9d67242104fafb8e0c150d.
경로: `artifacts/data-binary-target-loss-20260918/study/{B-BASE,S-SPAN}/segment-00/step-0512.r3m`.
원 raw는 같은 segment의 dev/cross/ordinary-0512.r3er, 입력은 각 inputs.r3er,
실행 로그 e4-base.log/e4-span.log, 부모 parity e4-parent-parity.log다.
fresh confirmation은 실제 gate 명령에서 NOT_RUN_NO_JOINT_CANDIDATE,
H3_SEAL=NOT_OPENED. H3_JOINT=FAIL; S4/S5/S6/GOAL1=NOT_PASSED;
독립 승인 대기. 다음 E5는 학습 종료 이후 별도 source의 저장 동등성 측정이다.

## E3 — train-only objective 구현과 실행 준비

R3-DATA-BINARY-AND-TARGET-LOSS-1.0 / EXECUTED_THIS_RUN / 2026-09-18.
E1/E2 source13c7d86e224606673918a11f9aa88a2dccde2c2c를 정상 push했고 실제 remote
full SHA 일치를 확인했다. 근거 root는 `artifacts/data-binary-target-loss-20260918/`.

기존 response_loss를 BASE로 보존하고 SPAN은 per-episode 정규화된 coefficient의
차이만 기존 objective에 더한다. 따라서 unsupported/zero-span 행은 혼합 batch에서도
기본 loss를 유지한다. answer 전체 token의 byte overlap union, EOS/padding/shift,
질량 보존, 독립 scalar/analytic gradient/central difference, microbatch objective와
gradient, 반복 ID/leading zero/Korean byte/인용/fallback 회귀4 PASS. fixed tolerance는
loss2e-6, gradient1e-6, mass1e-12이며 실패 뒤 완화하지 않았다.
실제 TINY weighted full-response optimizer2회와 binary objective 연속2 대 fresh
process1+1(4회), bound Adam/native/clock/policy/probe/close 회귀가 통과했다.

필수 quick는1회 수행했다. 20 commands 중19 PASS, 마지막 process suite에서
4 PASS/1 FAIL이었다. 재사용 bootstrap의 부모step24를 untrained로 가정한 기존
fixture 문제였다. 무작위 초기화 fixture만 optimizer0으로 별도 만들도록 고친 뒤
실패한 단일 process 회귀 PASS(10updates), QualityGuard+Cancelled+TimeBudget
동시 보존/재개 차단 확인. quick에서 미실행된 journal6/archive1도 각각 PASS.
원 quick FAIL을 새 전체 quick PASS로 바꾸지 않았다. 후속 관련 loss4/clippy도 PASS.
compile의 잘못된 bin명 `replica` 실패는0calls이며 올바른 replica-v3/train release
build는 PASS. 새 학습용 binary는 test-support 없이 빌드했다.

현재 TINY 총109/128: 이전38 + loss 최초2 + objective process4 + quick53(실패포함)
+ 수정 process10 + 최종 loss2. SMALL optimizer/generation/teacher=0/0/0.
원본을 읽는 draft release 준비에서 부모 dev240/entity247/event250/오류1,
CROSS462/entity499/event500/오류0, ordinary QA181/336,aux53/64를 재집계했다.
focus512/512 지원, anchor1778/2048 지원·270 fallback, first-target weight8이었다.
draft registration은 실행 source 변경 전 것이므로 학습에 쓰지 않는다.
debug 준비는 모델 API/optimizer 진입 전 긴 read/hash 감사 중 구현자가 SIGINT 후
SIGTERM으로 종료했다(관측 경과5분37초; 최종 정확한 elapsed UNKNOWN). 기록은
e3-data-audit-draft.log에 보존하며 모델 속도/학습 실패/완료로 세지 않는다.

최종 실행 binary=`e4-train`, SHA256
194fb3b2b0c93f01f1a9510ef9b9383e4b75a494eb5e5e372ad1c912f344422b.
최종 등록은 e3-final-registration.log에서 exit0: actual parent/native/Adam/cursor와
512draw의 동일성을 검증했다. 각 군 예정 input1255342/target131787, endpoint24822.
기존 의미 검사 Validated2432/AmbiguousEvidence128/Contradicted0이며 모호128을
검증 완료로 바꾸지 않았다. 역할 coverage는 위 draft와 같았다. 새 root=`study/`.
등록 source digest=b9e7ed5ea8016e46460ace4c22505064c2972190d1cb90c0adcf9941ffeee356.
OBJECTIVE_IMPLEMENTED=IMPLEMENTER_VERIFIED; 학습/부모 parity/train64 probe는
아직 NOT_RUN, H3/S4/Goal1 미통과, 독립 검토 대기.

## E0–E2 — publication 승인 및 부분 평가 재개 수리

R3-DATA-BINARY-AND-TARGET-LOSS-1.0 / 2026-09-18 / EXECUTED_THIS_RUN.
시작HEAD1e25be2cef801dd67a6515e5c33823a0d1b06a92, main/origin seoyd/replica-v3.
tracked clean, 기존 untracked487 보존. Rust/Cargo1.98.1. 기존 등록 원본63개와
A75-R/K/D 실제 native SHA 확인. A75-R24310 물리SHA는 승인된50b927dd…f09c와
전체값이 일치한다. 근거 root=`artifacts/data-binary-target-loss-20260918/`.

E1: 격리 기준 코드에 실제 hard_link 뒤 fault만 추가한 실행에서 최초 명령Err,
final bytes 존재, 새process verification 성공을 재현했다(e1-red-actual.log).
초기 e1-red.log는 오래된 bootstrap binding 거부이며 결함 재현으로 세지 않는다.
수리 후 pending+OS lock으로 관측된 발행 실패를 차단했다. 최종 process 시험은
post-link sync 오류, 이후 모든 record 쓰기 실패 조건, Pending 후 process 종료,
Pending 해제 실패, commit 후 cleanup 경고, 손상 pending, 새process 같은 검증/
후속 arm/report 차단과 정상 읽기 전용 승인까지 PASS(e1-final-process.log).
정리 sync 경고는 이미 durable한 final의 commit 실패로 반환하지 않는다.

E2: isolated source에서 기존 RESTART-only consumer 조건을 유지해 COOLDOWN
partial resume의 최종 close 거부를 재현(e2-red.log); 이 source에는 E1 수리와
TINY fixture 확장이 포함되어 있으며 원 기준 전체와 동일하다고 하지 않는다.
공통 capability와 byte-exact prefix 검증 후 RESTART/COOLDOWN × 논리128/256 ×
prefix0/1/2의12개 fresh-process/segment/close 모두 PASS(e2-green.log).
실제 optimizer24회이며128/256 labels를 실제 학습step으로 세지 않았다.
후속 helper의 prefix/모델/tokenizer/policy/분모/complete 중복 거부 unit1 PASS,
관련 clippy PASS. 잘못된 exact-name 호출0-test는 PASS에서 제외했다.

현재 누적 실제 TINY38/128: E1 RED4+최초GREEN4+최종GREEN4, E2 GREEN24+RED2.
SMALL optimizer/generation/teacher=0/0/0, scalar optimizer0. 기존 bootstrap은
읽기 전용 재사용. 영향 quick는 E3 source 안정화 뒤1회 예정이며 아직 NOT_RUN.
PF_PUBLISH/ PARTIAL_RESUME=IMPLEMENTER_VERIFIED; 모델 연구와 token cache는
NOT_RUN. H3/S4/Goal1 미통과 유지, 독립 검토 대기. 다음 인가 단계E3.

## P5 최종 — 수리 검증 및 제한 실험 종료, 모델 joint 품질 실패

R3-PREFLIGHT-ONCE-AND-COOLDOWN-1.0 / RESULT=PARTIAL / 2026-09-18.
REPAIR_SCOPE=IMPLEMENTER_VERIFIED, STUDY_EXECUTION=COMPLETE_512,
MODEL_QUALITY=STUDY_COMPLETE_QUALITY_FAIL. 전체 합격이나 Goal1 완료가 아니다.
P4 report2e10ca064c949bc6912c3ba360eaf30cb073dc21도 정상 push/remote SHA를 확인했다.

최종 요구 대조에서 ordinary paired 통계를 QA336/aux64로 나누는 보고를 보완했다.
학습 종료 후 변경은 src/experiment_record.rs의 보고 집계·관련 inline test뿐이며
학습 source0e8aa3049a90626264052e6d117370c15089f4c7와 p3-train은 보존했다.
새 감사 binary는 p5-audit(SHA aa83f501d24b82b76d248b48e6d7028b50e75f263a6cad9420dafb43df8f624a),
그 source file SHA는2e43ffc474470c88b90112878ce84d9a3a218970dcb27b9ef7cf8abf4f5bbeda다.
이 감사 source에서 모델을 더 학습하거나 새 출력을 생성하지 않았다.

`p5-recount.log`와 `p5-final-recount.log`의 새 process 재감사는 모두 exit0이었다.
완전한 raw/native/step/source/policy/command/guard/분모를 읽어 다시 채점했고,
기존 comparison과 일치했다. 보고용 분리 집계 unit1 PASS, clippy/release PASS
(`p5-paired-unit.log`, `p5-clippy.log`, `p5-build.log`). 최종 quick는 P2의1회뿐이다.
P3/P5 후속 소스는 해당 직접 회귀로 구분하며 옛 quick를 새 전체 PASS로 세지 않는다.

### 같은 최종 checkpoint의 paired 변화

아래 벡터는 [둘 다 오답, gain, loss, 둘 다 정답]이다. QA와 aux 분류는 고정
episode metadata의 family이며, 정오 여부로 분모를 바꾸지 않았다.

| panel | 부모→K | 부모→D | K→D |
| --- | --- | --- | --- |
| dev256 | [15,1,3,237] | [16,0,3,237] | [18,0,1,237] |
| CROSS512 | [48,2,6,456] | [48,2,5,457] | [47,7,6,452] |
| ordinary400 | [140,26,29,205] | [144,22,22,212] | [153,16,13,218] |
| QA336 | [130,25,29,152] | [133,22,22,159] | [143,16,12,165] |
| aux64 | [10,1,0,53] | [11,0,0,53] | [10,0,1,53] |

CONDITIONAL_LR_EFFECT: 이 부모·tape의 D는 K보다 QA4/CROSS1 높고 dev1 낮았다.
D의 QA181은 부모181을 유지했지만 dev237/CROSS459는 부모240/462보다 낮다.
후기 LR 감소 하나로 복사·QA의 동시 기준을 충족하지 못했다. 망각·tokenizer·모델 수식의
근본 원인은 UNRESOLVED다. 단일 seed의 작은 차이를 통계적 우월성으로 주장하지 않는다.

### 노출·하위집단·관측 사용량

양군 각각 anchor/focus unique views1255/512, unique bases1255/128, draws1536/512다.
pool별 actual input481562/146001, supervised46710/19062로 양군이 정확히 같았다.
base4/4는 K dev56/64,CROSS106/128,ordinary39/100; D55/64,106/128,41/100이다.
256/512 correlated view를 독립 base256/512개로 해석하지 않는다.

| 최종 하위집단(정답/분모) | K | D |
| --- | --- | --- |
| dev input bytes64–95 /96–127 | 60/64,178/192 | 60/64,177/192 |
| CROSS input bytes64–95 /96–127 | 114/128,344/384 | 113/128,346/384 |
| ordinary category0/1/2 | 29/68,22/68,4/68 | 29/68,24/68,4/68 |
| ordinary category3/4 | 114/132,62/64 | 114/132,63/64 |
| ordinary input bytes32–63 /64–95 /96–127 | 54/64,153/308,24/28 | 53/64,156/308,25/28 |

길이는 request.input의 UTF-8 byte 수이며 전체 evidence/prompt 길이가 아니다.
질문 family, H3 digits/kind/pattern(반복 패턴), 범주별 전체 행은 p5-final-recount.log의
STRATUM에 보존한다. 바로 앞 NODE의 arm/step/panel에 속하며 중간과 최종을 합치지 않는다.
ordinary category 표는400 전체다; QA336/aux64 별도 점수·paired 표와 혼동하지 않는다.

실제 SMALL generation2928/4096, 관측 generated tokens100891(EOS 포함), teacher0.
부모16회525 tokens + K1456회50146 + D1456회50220이며, final watch 중복을 제거한
고유(step,ordinal) raw 개수와 실제 API counts가 일치했다. 실제 모델 실행의 UNKNOWN tail은 없다.
실패 TINY/강제 종료 회귀의 UNKNOWN은 원래대로 실패 기록이며 수치0으로 바꾸지 않았다.
TINY 총85/128, scalar optimizer0, SMALL512/512; 새 학습·generation 재시도는 없다.
각 arm command receipt 시간 합은 K615.495029584s,D601.397650542s다.
parent verification lower bound44.542827458s와120s publication reserve를 더한 집계는
1381.435507584s로7200s 안이다. 단일 command도1800s 이내였다. 마지막 immutable
record 자신의 fsync/exit 시간은 그 record의 elapsed에 넣을 수 없으며 실제0이라고 하지 않는다.
컴파일·read-only 재감사 시간은 모델 학습/생성 성능 측정에 합산하지 않았다.

최종 native는 양군 각각115285312 bytes. K Adam=d924a316f1404414457680bf62ad866a2e03327615661e3adc6e3c41f0e6ba71,
D Adam=3089ed191d713054950cd71ee6609ef342a87c8cf335cdb08a8677dd40efa460.
양군 counters=[43940822,3384495,6741870243436089269], step24566이다.
원래 config LR0.00003과 이번 실제 LR trace는 별도 값이다. 새 inference-only export는 만들지 않았다.

### 최종 판정과 보존

| 필드 | 판정 |
| --- | --- |
| PV01_CODE / PV01_REAL_PROCESS / PREFLIGHT_USAGE_ACCOUNTING | IMPLEMENTER_VERIFIED |
| PARENT_RAW_RECOUNT / SOURCE_AND_BINARY_BINDING | VERIFIED |
| LR_STUDY | COMPLETE_K256_D256; 추가 예산 없음 |
| NORMAL_H3_DEV / CROSS | 양군 FAIL |
| ORDINARY_RETENTION | K177 FAIL / D181 FLOOR_PASS(178), joint 아님 |
| JOINT_DEV_GATE / MODEL_QUALITY_RECOVERED | FAIL / false |
| FRESH_CONFIRMATION | NOT_RUN_NO_JOINT_CANDIDATE; 실제 gate command exit0, 생성0 |
| H3_SEAL_NOT_OPENED | true |
| S4 / S5 / S6 | NOT_PASSED / NOT_RUN_THIS_SCOPE / NOT_RUN_THIS_SCOPE |
| GOAL1_READY / GOAL1_ACCEPTED | false / false |
| INDEPENDENT_REVIEW | PENDING_EXTERNAL; 자체 독립 승인 아님 |

인가된 원자료 경로는 아래 P4/P3 절과 동일하다. 시작에 등록한 원본63개 SHA가 전부
유지됐다(p5-originals-verified.log,exit0). 기존487개 untracked도 유지했다.
새 canonical intent/raw/final/LR 상태는 typed R3ER이고 .r3m은 기존 native binary다.
legacy JSON corpus/policy/명시 import, 비기준 개발 JSON 보고와 기존 model inspect의
사람용 JSON 출력은 남아 있다. 새 identity나 재개 상태로 JSON 재직렬화를 쓰지 않았다.
journal/DB/압축/tokenizer/코어/precision/loss/decay 계수는 그대로다.
현 계약은 종료한다. NEXT_GATE는 새 명시 인가와 미해결 품질 가설이며, 추가 학습이나
봉인 평가를 자동 시작하지 않는다. 이 절을 포함한 마지막 감사 source/report commit의
remote SHA 확인은 로컬 p5-publication.log에 기록한다.

## P4 완료 — 고정 LR 비교 실행 완료, H3 조건 미달

R3-PREFLIGHT-ONCE-AND-COOLDOWN-1.0 / EXECUTED_THIS_RUN / 2026-09-18.
수리 source7cd0a58bc989373ec8494fdf99131c7850ad0f37와 연구 source
0e8aa3049a90626264052e6d117370c15089f4c7를 정상 push/remote SHA 확인했다.
두 군 실행 source=0e8aa3049a90626264052e6d117370c15089f4c7,
source digest926952e2ef451b67fb85d2699fd0bc001e285b47bd03d270eca478fa14bee071,
binary c3cc1587754f6c674805bce2d1d8c6e31bd12251cf6302622d3e48f6a4861f65.
실행 도중 소스·바이너리·tape·정책을 바꾸지 않았다. 이 절은 report-only 변경이다.

각 군은 같은 A75-R24310 부모에서 실제1회→저장→새 process 검증→255회로 끝났다.
첫 native 전체 SHA는 두 군 모두6158c3cff72925e9c76724aa3822a28690f0c56c9adf2f2220d2a0939b08c7ec였다.
첫 LR와 weights/Adam/state가 같은 실제 SMALL 저장 관측이며 추가 preflight4가 아니다.
두 최종 command는 STOP=[],complete=true,resume=false,candidate=false,
COMMAND_FINALIZATION=Complete로 exit0. 첫 TimePause는 예산에 포함된 사전등록 저장 경계다.

| 동일 endpoint 평가 | dev full/entity/event (/256) | CROSS full/entity/event (/512) | QA /336 | aux /64 | dev/CROSS/ordinary 오류 |
| --- | --- | --- | ---: | ---: | --- |
| 부모 A75-R24310, 재집계 | 240 /247 /250 | 462 /499 /500 | 181 | 53 | 1 /0 /0 |
| K 중간128,24438 | 238 /247 /250 | NOT_RUN | NOT_RUN | NOT_RUN | 1 /NOT_RUN /NOT_RUN |
| D 중간128,24438 | 240 /247 /250 | NOT_RUN | NOT_RUN | NOT_RUN | 1 /NOT_RUN /NOT_RUN |
| K 최종256,24566 | 238 /247 /251 | 458 /497 /501 | 177 | 54 | 1 /0 /0 |
| D 최종256,24566 | 237 /247 /250 | 459 /498 /501 | 181 | 53 | 1 /0 /0 |

중간 watch는 K20/32,D19/32, 최종 watch는 K20/32,D21/32다. 최종 watch는
동일 ordinary subset이며 중복 생성이 아니다. 모든 최종 필수 출력은 EOS를 가졌지만
양군 dev의 ordinal2772는49 tokens/EOS48 뒤 strict UTF-8 오류로 실패했다.
정상 생성 원시 출력과 고정 expected를 그대로 채점했고, 오류를 분모에서 제외하지 않았다.

| 실제 새 소비 | SMALL updates | input tokens | target tokens | anchor/focus draws | generation / teacher |
| --- | ---: | ---: | ---: | --- | --- |
| 부모 parity | 0 | 학습0 | 학습0 | 0 /0 | 16 /0 |
| K-KEEP | 256 | 627563 | 65772 | 1536 /512 | 1456 /0 |
| D-DECAY | 256 | 627563 | 65772 | 1536 /512 | 1456 /0 |
| 합계 | 512 /512 한도 | 1255126 | 131544 | 3072 /1024 | 2928 /4096 한도, teacher0 |

TINY는 실패·quick·최종 관련 재검증까지85/128, scalar optimizer0.
K actual LR=1e-4; D final LR=1e-5(bits4532020583610935537), midpoint bits는
4543282299299812598로 사전등록·실제 trace가 같았다. Adam absolute step은24566이며
모델 하나에512회를 더한 것이 아니라 두 모델 각256회다. 이전1028회 예산과 합치지 않는다.

허용된 로컬 근거 root=`artifacts/preflight-cooldown-20260918/`:
`p4-k-first.log`, `p4-k-resume.log`, `p4-d-first.log`, `p4-d-resume.log`가 실제 실행 로그다.
`study/{K-KEEP,D-DECAY}/inputs.r3er`, `segment-00/final.r3m`,
`segment-01/step-0128.r3m`, `segment-01/step-0256.r3m`, 같은 segment의 raw panels,
decision/terminal/command 및 arm의 comparison.r3er를 보존한다. 코퍼스·DB·원래 실패는 그대로다.

K final physical=ded3dbd682bd5b889bdd818bd749646a7de841363756547cfd8686c1b17a64e6,
model=d8c59bfa04d29d862bc017c784356ef57d052a2af6a98359a9759ae026acb476.
D final physical=81e28fdfb8b319647a924b4023297c86edb7ac9574875f63a72ec9edff0021af,
model=c36685cbc5b4e5972fe44ce4076c283e77e5524ad6fcc5cd7b4f5172bd96fe77.
P5 독립 raw 재집계/paired 비교는 진행 중이다. 품질 후보 추가 생성은 선행 joint 실패로
NOT_RUN이며 seal NOT_OPENED, S4/S5/S6와 GOAL1_READY/ACCEPTED는 false/미통과다.
도구 수리 PASS와 학습 실행 완료를 모델 품질 PASS로 바꾸지 않는다.

## P0–P2 진행 — preflight 시도·부분 결과·실패 회계 수리

R3-PREFLIGHT-ONCE-AND-COOLDOWN-1.0 / 2026-09-18. 시작 HEAD198a1a032368f023d81ab744e59eccb1cd6b6e53,
source50a0fb72f552cc130fbd5598019222c7a7bd95fe 이후 코드는 같고 tracked dirty는 없었다.
기존487개 untracked 로그를 보존했다. Rust/Cargo1.98.1, CPU Accelerate/threads1,
offline locked 의존을 유지한다. 부모 A75-R24310 physical SHA
50b927dd41771c39ca5e7138ca28aaf8193a96460eff9a2aae92ed444015f09c를 실제 확인했다.
새 증거 root=`artifacts/preflight-cooldown-20260918/`; originals.sha256가 기존 pair와
preflight 파일을 이번 시작 시점에 묶는다. 과거 시점의 receipt 증명으로 소급하지 않는다.

격리 기준 source+hook-only에서 첫 row 후 cancel의 Started 부재를 실제 재현했다
(`p1-red.log`, TINY4). 수리 후 실제 process 회귀1 PASS: 최초 token 취소/nonfinite,
row 후 취소/deadline, Started/proof 후 child exit, raw/proof/final publication 실패,
재시도·후속 arm 차단, 정상 read-only 재확인, 두 writer 경쟁, corrupt final/다른 native.
GREEN `p2-pv-process-fixed.log`: 실제 TINY4/verification generation16/teacher0,
후속 optimizer0. 첫 실행 `p2-pv-process.log`는 TINY panel을 분기당2개로 잘못 가정한
test assertion 실패(TINY4, verification generation7)였고 원래1개 fixture에 맞췄다.
SMALL0. 직접 실행 누적 TINY12, scalar0; RED/실패를 PASS에 합산하지 않았다.
이후 final quick는 한 번 실행해22 commands(19 test invocations),75 test executions/
고유74 모두 PASS,0-test0, source_unchanged=true를 확인했다. quick TINY53을 포함한
누적 TINY65/scalar0다. test-support 없는 train/product release build도 PASS다.
quick source digest=9483d2b57bb1dacb2ad549efa2a6d330f84b08c3cf3055fef118398488155b5a.
증거는 p2-quick.log, p2-quick/summary.json, p2-release-build.log다.

Started/entry reservation/returned row/final은 typed R3ER이다. 같은 reader가 성공 여부와
실패의 알려진 소비량을 출력한다. UNKNOWN tail은 예산을 잠그며 원래 SMALL proof는
명시 legacy read-only로만 재사용한다. 회귀의 강제 종료는 장치 정전 검증이 아니다.
PV01 direct process/quick=PASS. 새 release binary의 anchor-report가 기존 전체 raw와
native/command/close를 read-only 재검산해 exit0이었다(p2-old-pair-recount.log).
최종 C50-R dev241/CROSS461/QA170, A75-R240/462/181, dev 오류 각각1과 기존
paired 표가 모두 일치했다. 새 SMALL generation/optimizer0. 오류의 개별 token/empty
의미 분류는 P3에서 기존 row로 확인한다. P3–P5=NOT_RUN, INDEPENDENT_PENDING.
새 연구는 최종 A75-R 부모의 K-KEEP/D-DECAY256씩이며 이전 C50/A75와 preflight4를
반복하지 않는다. 모델 품질·S4/S5/S6·Goal1 수용은 그대로 미완료다.

### P3 source 및 직접 수치 검증

P1/P2 commit7cd0a58bc989373ec8494fdf99131c7850ad0f37를 정상 push했고
실제 remote full SHA 일치를 확인했다. P3는 기존 run/trainer/Adam/native 경로에
K constant1e-4 / D horizon256 cosine1e-4→1e-5 정책만 추가한다. 실제 Adam에 전달한
f64 LR bits를 typed segment에 저장하고 new_updates/cumulative step/tape와 재검증한다.
full TrainingState와 weights/Adam/cursor 비교의 TINY 연속2 대 새 process1+1 PASS
(p3-cooldown-process-3.log,4 updates/0 generations). scalar endpoint/midpoint 독립식
1 test PASS(p3-scalar.log), scalar optimizer0. 첫 fixture 경로 불일치 실행은0 updates,
그다음 CLI 인자 실패는4 updates였다. 이 실패를 지우거나 성공에 합산하지 않았다.
새 공통 verification 경로의 PV01 직접 회귀도 PASS(p3-pv-process.log,TINY4/gen16).
현재 누적 TINY77/128, scalar optimizer0, SMALL optimizer/generation0이다.
최종 quick는 P2의 한 번으로 유지하며, P3 변경에는 관련 직접 테스트·clippy·release를
별도로 적용한다. P2 quick source와 P3 source를 같은 검증으로 합산하지 않는다.
최종 source에서 두 process 회귀를 다시 실행해 각각 PASS했다(p3-final-lr-process.log,
p3-final-pv-process.log). 추가 TINY8을 포함한 총85/128, scalar optimizer0이다.
최종 clippy와 test-support 없는 train/product release build PASS. 새 source digest는
926952e2ef451b67fb85d2699fd0bc001e285b47bd03d270eca478fa14bee071,
실행 binary SHA는 c3cc1587754f6c674805bce2d1d8c6e31bd12251cf6302622d3e48f6a4861f65다.

P3 등록은 p3-prepare.log에서 exit0. 실제 부모 step24310 / model
dfc3efb664351578340e39d3cfc95b90f270541041d4641d72d6f41a53871b11 / Adam
fb0dc94572db5a6dfc05951217a20b1021c62193dd09e81263498bea5b303e19,
sampler6741870243436089013를 확인했다. 기존512 draws를 재현·대조하고 이어지는256을
고정했다. 각 군의 입력627563/정답65772 tokens로 각각1M/250K 이하다.
step1/128/256 metadata 검사는 실제 업데이트 없이 validator로 통과했다.
study/K-KEEP 및 study/D-DECAY의 binary inputs와 parent.r3m이 원래 연구와 분리된다.

부모 정상 생성16개(dev8/ordinary8, metadata-only 선택)가 실제 raw token/EOS/error와
모두 일치했다. Started/partial/proof/final을 같은 경로로 검증했다(p3-parent-parity.log).
실제 SMALL generation16,completed16,interrupted0,관측tokens525,teacher0,optimizer0.
final 기록 시간 lower bound44.542827458s, publication 이후 관측44.5498985s와
보수적120s reserve를 구분한다. 표본16은 전체 품질 점수 추정이 아니다.
추가 raw 오류 분류(p3-parent-error-classification.log,새 generation0)에서도 기존 점수와
paired 결과가 일치했다. 양군의 dev 오류 각각1은 ordinal2772, tokens49, EOS48,
strict_utf8 실패다. zero tokens/decoded empty가 아니라 decode 실패에 따른 actual 부재다.
P3 준비는 IMPLEMENTER_VERIFIED이며 실제 K/D 학습은 아직 NOT_RUN이다.

## D4 최종 — 수리·저장·정식 pair 실행 완료, 모델 품질 미달

MODE=IMPLEMENT; CONTRACT=R3-DURABILITY-PAIR-RESTART-1.0; RESULT=PARTIAL.
2026-09-18 / EXECUTED_THIS_RUN. D0–D4의 인가된 실행을 종료했다. 코드·저장 경계는
검증됐으며 실제 두 arm도 끝났지만 H3/S4/Goal1 품질을 통과한 것은 아니다.
SOURCE_COMMIT=50a0fb72f552cc130fbd5598019222c7a7bd95fe.
REPORT_COMMIT은 이 절을 추가하는 report-only commit이며 source와 구분한다. source와 D2 report
ea27a00f94dbe3621bb1f2c575f5de7632cc5fcc는 정상 push/remote full SHA 일치를
실제로 확인했다. 최종 report의 remote 확인은 로컬 `d4-publication.log`에 보존한다.

### 실제 모델 결과와 판정

같은 F51223798 weights/Adam, 고정801 tokenizer/F32/CPU Accelerate/threads1,
실제 constant LR1e-4에서 anchor/focus4:4와6:2만 비교했다. 각각 절대 step24310까지
512 updates를 실행했다. dev/CROSS/ordinary는 각각256/512/400 전체를 정상 greedy로
생성했고 watch32는 ordinary의 같은 모델 부분집합이다. 첫 행은 기존 raw를 이번에
재채점한 기준점이며 새 생성이 아니다. +256은 중간 관측, +512가 고정 primary endpoint다.

| 같은 checkpoint | dev full/entity/event (각 /256) | CROSS full/entity/event (각 /512) | ordinary QA /336 | aux /64 | dev/CROSS/ordinary 오류 |
| --- | --- | --- | ---: | ---: | --- |
| F512 부모, 기존 raw | 242 / 251 / 251 | 459 / 501 / 498 | 176 | 56 | 0 / 0 / 0 |
| C50-R +256 | 244 / 250 / 251 | 464 / 503 / 498 | 179 | 54 | 0 / 0 / 0 |
| C50-R +512 | 241 / 247 / 251 | 461 / 497 / 502 | 170 | 50 | 1 / 0 / 0 |
| A75-R +256 | 245 / 250 / 252 | 460 / 497 / 501 | 182 | 52 | 0 / 0 / 0 |
| A75-R +512 | 240 / 247 / 250 | 462 / 499 / 500 | 181 | 53 | 1 / 0 / 0 |

모든 panel에서 EOS 수는 계획 분모와 같았다. EOS가 있어도 생성 오류가 있는 출력은
정답으로 세지 않았다. 최종 dev 각1건은 error/empty이며 raw 그대로 보존했다.
최종 watch는 C50-R17/32, A75-R19/32다. 같은 오류를 watch와 ordinary에서 별개의
모델 호출로 합산하지 않는다. 기존 guard hard stop은 발생하지 않았다. 두 command는
STOP=[], complete=true, resume=false, COMMAND_FINALIZATION=Complete로 exit0 종료했다.
정해진512회를 다 쓴 SCREENING_BUDGET_REACHED이며 자동 연장은 없다.

새 process의 `recovery native anchor-report`가 command/segment/native/4panel 연결과
fixed expected·token/EOS/error를 다시 검증했다. raw 재채점이 저장된 comparison과
일치하며 report도 exit0이다. 결과는 STUDY_COMPLETE_QUALITY_FAIL, 두 candidate=false.
정상적인 낮은 품질 결과를 integrity failure로 바꾸지 않았다.

| paired C50-R → A75-R, +512 | 둘 다 오답 | gain | loss | 둘 다 정답 |
| --- | ---: | ---: | ---: | ---: |
| dev256 | 15 | 0 | 1 | 240 |
| CROSS512 | 45 | 6 | 5 | 456 |
| ordinary400 (QA+aux) | 148 | 32 | 18 | 202 |

ordinary의 paired 표는400 전체이며 QA336 점수에 aux를 합친 지표로 승격하지 않는다.
A75-R의 QA181은 보존 하한178을 넘고 C50-R보다11개 높다. 그러나 dev·CROSS 요소별
조건과 오류0을 충족하지 못했다. 이번 한 부모·한 고정 tape 비교는6:2의 제한된 QA
보존 관측이며 지능 회복·특정 수식/LR/망각 원인의 확정 근거가 아니다. 중간256의 더
높은 값을 사후 후보로 바꾸지 않았다. 추가 변수 시험과 학습은 실행하지 않는다.

| 최종 필드 | 판정 |
| --- | --- |
| JOURNAL_RETRY_ACK | VERIFIED_AT_SYNC_ALL_BOUNDARY |
| PERSISTED_CLOSE_STOP / COMMAND_CLOSE_STOP | IMPLEMENTER_VERIFIED: fresh process·다른 arm·report 차단 |
| COMMAND_FINALIZATION | IMPLEMENTER_VERIFIED: 실패/누락 certificate 차단, 실제 두 정상 command 확인 |
| FORK_METADATA | VERIFIED: validator와 실제23798→24054→24310 writer/reader 경로 |
| SMALL_SAVE_PREFLIGHT | PASS: 실제 연속2 대 새 process1+1, exact weights/Adam/state |
| C50_RUN / A75_RUN | COMPLETE_512 / COMPLETE_512, 저장·close 정상 |
| MODEL_PAIR | STUDY_COMPLETE_QUALITY_FAIL |
| ANCHOR_PRESERVED | C50-R FAIL170/336; A75-R QA_FLOOR_PASS181/336; joint candidate 아님 |
| RAW_H3 | VERIFIED_RECOUNT; JOINT_GATE_NOT_MET |
| H3_SEAL | NOT_OPENED; 추가 fresh candidate 전 패널 생성 NOT_RUN(선행 gate 미달) |
| S4 / S5 / S6 | NOT_PASSED / NOT_RUN_THIS_SCOPE / NOT_RUN_THIS_SCOPE |
| GOAL1_READY / GOAL1_ACCEPTED | false / false |
| INDEPENDENT_ACCEPTED | false; PENDING_EXTERNAL_REVIEW |

### 실제 소비량·시험·한계

| 실행 | SMALL updates | input tokens | target tokens | anchor/focus draws | 새 generation / teacher |
| --- | ---: | ---: | ---: | --- | --- |
| 저장 preflight 연속2+분할1+1 | 4 | 9,998 | 1,096 | 16 / 16 | 4 / 0 |
| C50-R | 512 | 1,225,844 | 138,636 | 2,048 / 2,048 | 2,336 / 0 |
| A75-R | 512 | 1,254,182 | 131,625 | 3,072 / 1,024 | 2,336 / 0 |
| 이번 합계 | 1,028 | 2,490,024 | 271,357 | 5,136 / 3,088 | 4,676 / 0 |

pair 각 군의 unique views는 anchor2048/focus512, unique scene bases는2048/128이다.
토큰 노출은 비율에 따라 달랐으며 동일 token compute 비교라고 부르지 않는다.
한 모델에1,028회를 누적한 것이 아니다. 과거 실패 C50의256회는 이전 실행으로 남긴다.
예산 SMALL1028/generation7500, arm별2M input/500K target를 넘지 않았다.
모델 작업 시작 기록00:42:45UTC부터 완료 후 확인01:25:02UTC까지2537초(명령 사이 대기와
재채점 포함)로7200초 이내다. 각 arm terminal 측정은1063.263s/1029.313s이며 마지막
close/publication 시간은 제외한다. command finalization의 elapsed는 별도 binary에
있으며 단일1800초 중단을 맞지 않았다. fsync/tensor 강제 선점 보장은 하지 않는다.

선택 직접 회귀와 최종 quick: 통과 test invocations25 / test executions93 / 고유73.
quick 자체는19 invocations/74 executions/고유73(겹친 filter1)이다. RED 두 번은 격리된
기준 소스의 의도한 assertion 실패이며 수정 후 통과했고, 별도 compile 실패1은 테스트
통과/RED로 세지 않았다. filtered-zero0. fmt/check/clippy/release PASS. 학습 이후 소스
수정이나 추가 학습·생성·테스트 실행은 없고, D4는 무학습 raw 검산과 원본/hash 확인이다.
이번 TINY79/scalar0; 직접/native fixture generation156/teacher152, 기타 기존 quick의
미계측 generation/teacher는 UNKNOWN이다. 이를 SMALL 또는 전체 호출0으로 쓰지 않는다.

journal poison unit은 실행당 신규 실패1·retry 실패1·reopen 성공1의 sync helper 호출을
검사했고, poisoned 재사용/conflict/read-only 추가 호출0을 확인했다. 직접 회귀와 quick에서
각각 실행했다. retry의 새 frame bytes는0이며 Commit/파일 bytes 동일성 assertion을 통과했다.
이는 sync_all 호출 경계 검증이다. 장치 정전/F_FULLFSYNC 동등성, 신규 throughput/latency
benchmark는 NOT_RUN이며 기존 G3/G4의 수치를 새 실측으로 바꾸지 않는다.

### Candidate·체크포인트·인가된 로컬 증거

Root=`artifacts/durability-pair-restart-20260918/`. Source diff는
`candidate-source.diff`(기준2e4121035→50a0fb7, src/tests), source SHA는 `source.sha`다.
변경 파일은 기존 `src/journal.rs`, `src/experiment_record.rs`, `src/check_main.rs`,
두 직접 test 파일과 QUALITY_RECOVERY_PLAN/RUNBOOK/STORAGE_FORMAT/이 상태 문서뿐이다.
새 영구 파일·의존성 없음. 실제 바이너리 `d2-train`의 SHA256은
b89540d30c89d96e96c0a8302053f259f48a292120856deaae32030d7d760822로 학습 전후 동일하다.

LAST_DURABLE_NATIVE(모두115,285,312bytes, weights+Adam/state, absolute24310):

| 경로(root 기준) | 실제 전체 파일 SHA-256 |
| --- | --- |
| attempt-R/C50-R/segment-00/step-0512.r3m | 0ee89cfb340aab06ae09b528acb4dd80cbc7b05b894aae7e47b349527725863f |
| attempt-R/A75-R/segment-00/step-0512.r3m | 50b927dd41771c39ca5e7138ca28aaf8193a96460eff9a2aae92ed444015f09c |

각 +256 native도 같은 segment의 `step-0256.r3m`으로 실제 존재한다. 4개의 새 정식
native bytes 합계461,141,248이며 최종 terminal 발행에서는 검증된 +512 파일을 재사용해
추가 model bytes0이었다. 이를 전체 실험 디스크 크기나 IO benchmark로 해석하지 않는다.
terminal/command/comparison과 각 step panel raw는 같은 segment의 `.r3er`에 있다.
소유 입력·정책·tape는 각 arm의 `inputs.r3er`, 원래 F512 복사본은 `parent.r3m`이다.
원래 corpus 위치는 `artifacts/h3-controlled-20260917/a2/corpus-F/`,
`artifacts/goal1-corpus-v9/`, ordinary는
`artifacts/s4-completion-20260917/binding-corpus/validation.json`이며 자동 변환/수정하지 않았다.

실행 증거: `d3-C50-R.log`, `d3-A75-R.log`, `d4-pair-report.log`, 앞선 D1/D2 로그,
`d4-originals-verified.log`, `d4-execution-verified.log`, `d4-status.txt`.
ORIGINALS_UNCHANGED: 원래 실패 pair manifest11/11 OK, 동결 실행/source3/3 OK,
시작과 최종 git status 동일. 소유 heavy process는 종료됐다. 원본 실패의 endpoint 부재와
resume=false를 보존했고 모델 포인터·운영 DB를 바꾸지 않았다. 원자료·모델·로그는 Git에
올리지 않는다. 새 canonical command/인가/preflight는 typed R3ER이며 run의 새 JSON쓰기0,
legacy JSON읽기0. 준비 시 기존 corpus/policy의 read-only import, 기존 개발 보고/IPC의
JSON 잔존은 아래 G5 저장 경계 표와 같다.

DERIVED_FROM_EXISTING_LOGS: 기존 F512/실패 C50 수치와 이전 저장 benchmark.
SOURCE_ONLY/UNRESOLVED: 두 수리 결함이 과거 모델 부진의 원인이었다는 인과는 확인하지
않았다. 실제 이번 경계는 RED→GREEN 및 native 실행으로 검증했다.
REMAINING: H3 joint 품질·독립 seal·S4/S5/S6·Goal1과 외부 독립 수용.
인가된 수리와 단일 비교의 미실행 필수 단계는 없으며, 다음 학습은 이 닫힌 예산으로
자동 실행하지 않는다.

## D2 완료 — 실제 SMALL 저장·새 process 재개 동일성 PASS

2026-09-18 / EXECUTED_THIS_RUN. Source50a0fb72f552cc130fbd5598019222c7a7bd95fe를
정상 push하고 원격 full SHA 일치를 확인했다. 동일 동결 실행 파일은
`artifacts/durability-pair-restart-20260918/d2-train`, SHA256
b89540d30c89d96e96c0a8302053f259f48a292120856deaae32030d7d760822,
native evaluator source digest184aa0885dec1d46bd73b247ab9e6cdeb428aef21ca9d089db4952599ba3403d.

실제 F512 baseline raw 재검산: dev242/256(entity251,event251), CROSS459/512
(501,498), ordinary QA176/336,aux56/64, watch15/32, errors0. 새 생성 없이 기존
native/자료 binding을 검증했다. attempt303b8375b50e0ee6fd5e29a505568a186a96110f87e307e2ee527ed983529083를
별도 등록했고, 이전 실패 receipt와 바뀌지 않은 train/case/panel/tape를 연결했다.

SMALL 실제 연속2 + 분할1/새 process1 = 총4 updates, input9,998/target1,096.
두 분기 모두 실제 step23800. 동일 weights hash
01f0fd501a78181e0a43ce4bd87ba7341584f08bf3f1fa7aebe53b03d87294f5,
Adam a5281b1d37e149e19c55f7d1585f1980c34629c47e418a24c182422555e53a73,
native physical file cfc9beb4886779d6606ef501895a6624f7a174da5d60cb26a1f2f33d1d566ca7.
누적 input42,064,076/target3,187,646/sampler6741870243436088503과 TrainingState가
일치했다. 실제 LR1e-4(bits4547007122018943789); 보존한 historical config LR3e-5와
구분하며 cosine으로 바꾸지 않았다. 정상 greedy parity generation4,teacher0, 차이0.

증거는 같은 root의 `d2-small-continuous.log`, `d2-small-split-first.log`,
`d2-small-split-resume.log`, `d2-small-verify.log`, `d2-registration.log`와
`attempt-R/preflight-{A,B}/`, `attempt-R/preflight-proof.r3er`다. 명시 목적은
SAVE_RESUME_PREFLIGHT/quality_eligible=false이며 정식 부모로 사용하지 않는다.
SMALL_SAVE_PREFLIGHT=PASS; 다음 C50-R/A75-R는 각각 원래 F512에서 시작한다.
H3/S4/Goal1 품질 수용은 여전히 미완료이며 이전 C50 실패256을 지우지 않았다.

## D2 준비 — native 사전검증 경로·학습 전 코드 gate 통과

2026-09-18 / EXECUTED_THIS_RUN. D1 source
c5f8d03fc08b61f451999930356b71f077f85d10을 정상 push하고 실제 remote SHA 일치를
확인했다. 이 절은 실제 SMALL preflight 이전의 코드 검증 상태다: SMALL0, D3 NOT_RUN.

기존 runner/optimizer/native writer/loader를 그대로 호출하는 한정 preflight 목적을
등록했다. 연속2와 별도 process1+1, 소수 greedy parity의 성공 proof가 있어야 정식
C50-R/A75-R를 시작한다. 새 계약/실패 receipt hash/실행 source·binary로 attempt를
구분하며 원래 parent·case·panel·pool·tape를 재검증한다. 정식 평가 전에 endpoint를
저장·로드한다. clean time-only partial panel은 원본 raw와 완료 prefix를 보존해 재개한다.

직접 TINY preflight/partial-panel process1 PASS, TINY6/gen8/teacher6. 마지막 source의
필수 quick는19 test invocations/74 test executions PASS, 중복된 필터1개를 제외한
고유73 tests다. fmt/check/clippy와 source_unchanged=true; release build PASS.
quick source digest=ab3e87fd8436acb015dd8ac857b15d87a42fb649528a50da91752d134c7bb0d9.
근거: 같은 evidence root의 `d2-tiny-preflight.log`, `d2-quick/summary.json`,
`d2-release-build.log`. 0-test 호출이나 실패를 PASS로 합산하지 않았다.

이번 누적 TINY79/scalar0(직접30 + quick49), SMALL0. quick의49는 native process30,
기존 LR/renewal/time-split19다. 기존 bootstrap은 재사용했다. 직접/native process의
generation156/teacher152와 기타 기존 quick의 미계측 호출을 구분한다; 후자는 UNKNOWN,
0으로 보고하지 않는다. 모두128회 TINY/scalar 실행 상한 안이다. 다음 단계는 동결된
동일 source/binary에서 실제 SMALL4 preflight이며, 실패 시 정식 pair를 실행하지 않는다.

## D1 — retry ACK와 command 종료 경계 수리

2026-09-18 / R3-DURABILITY-PAIR-RESTART-1.0 / EXECUTED_THIS_RUN.
D0 HEAD1ea5abe9ae63e12ee8a7e9e8e6731b6b274d07fb는 기준 source2e4121035 이후
보고서만 변경된 상태였으며 tracked dirty는 없었다. 기존 미추적 로그와 원본 실패
pair를 보존했다. 실제 Apple M4/24GiB/Rust1.98.1/Cargo1.98.1, CPU Accelerate/F32,
threads1/offline lock을 사용했다. 신규 SMALL0; D2/D3는 아직 NOT_RUN.

격리 기준 source에서 hook-only 두 RED를 실행했다. trailer 작성 후 sync 전 child
종료→재시도 sync 오류 주입에도 ACK가 반환됐고, 정상 terminal 뒤 close 취소를
저장한 pair의 report가 exit0으로 실패를 버렸다. 후자는 기존 command 복사 경로를
TINY에도 열어 주는 test-only hook이며 SMALL 실행이 아니다. 로그를 보존했다.

수정 후 journal integration6 + poison unit1 + post-terminal process1(6경우) +
기존 time-split process1 + binary unit9 = 고유18 tests PASS(5 test invocations).
RED2는 의도된 assertion 실패이며, 별도 컴파일 실패1은 RED/PASS에 포함하지 않는다.
0-test 호출 없음. fmt/diff check와 all-target clippy PASS. TINY24 = RED2 +
실패경계12 + 연속/재개10, scalar0; 해당 native generation70/teacher70.
기존 bootstrap은 재사용했으며 이번 optimizer에 합산하지 않았다.

같은 request/content는 writer poison→sync_all 성공→기존 Commit ACK 순서다.
실패하면 같은 인스턴스가 계속 poisoned이며 새 reopen도 retry sync가 필요하다.
새 frame/view/sequence 추가 없음. process 종료/호출 오류 경계만 검증했으며 실제
전원 차단이나 macOS fullfsync 동등성을 주장하지 않는다. 대규모 benchmark 재실행0.

새 typed R3ER kind7 command는 terminal/비교 physical FileRef, run/binding, 추가 stop,
오류, 상태와 elapsed를 기록한다. terminal은 불변이고 comparison은 최종 command
전에는 provisional이다. 공유 effective outcome이 fresh close/다른 arm/report에
적용되며, close-stop와 command 모두 쓰지 못해도 missing positive finalization으로
차단한다. 취소·검증 오류·각 publication 실패와 정상 시간 재개/f64/model identity
회귀를 검증했다. failed/incomplete report는 nonzero이고 읽기 전용이다.

근거 root=`artifacts/durability-pair-restart-20260918/`:
`d1-journal-red.log`, `d1-close-red.log`, `d1-journal-green.log`,
`d1-journal-poison.log`, `d1-command-green-fixed.log`, `d1-native-positive.log`,
`d1-binary-unit.log`, `d1-clippy.log`, `original-pair.sha256`.
JOURNAL_RETRY_ACK=VERIFIED_AT_SYNC_ALL_BOUNDARY, COMMAND_CLOSE_STOP=IMPLEMENTER_VERIFIED.
실제 SMALL save preflight와 replacement pair는 다음 의존 단계다. H3/S4/S5/S6 및
Goal1 상태는 바뀌지 않았으며 INDEPENDENT_ACCEPTED=false.

## G5 최종 — 저장 검증 완료, 모델 비교 중단, RESULT=PARTIAL

2026-09-18 / R3-NATIVE-STORAGE-QUALITY-1.0 / IMPLEMENTER_REPORT.
최종 source `2e4121035aabd11918d64875285661ad8e3a8f91`를 origin/main에 정상 push하고
실제 full remote SHA 일치를 확인했다. 이 절의 후속 report-only commit은 source와
구분한다. 기준 source는 b18ec31bb78a8d0ef0adf985b46697a2d4993474이며, 실행 당시
동결 source/binary와 실패 raw는 아래 G2에 별도 기록했다. independent review는 미실행.

| 판정 | 결과 |
| --- | --- |
| CODE_CLOSE | VERIFIED: 서로 다른 정상 모델 panel 혼합 거부, 저장된 취소 fresh close 거부 |
| BINARY_MODEL_STORAGE | VERIFIED: 기존 native F32/Adam 분리, exact cold 측정·생성 parity |
| GRAPH_JOURNAL | VERIFIED_PROTOTYPE: exact bytes/관계/재시작/tail 복구; 운영 이전 없음 |
| MODEL_PAIR | QUALITY_INCONCLUSIVE: C50+256 native 저장 실패, A75 NOT_RUN |
| ANCHOR_PRESERVED | UNRESOLVED: 관측 QA179/336을512 endpoint 또는 재현된 개선으로 승격하지 않음 |
| RAW_H3 | C50+256 dev244/256,CROSS464/512 관측; native binding 없음, joint gate 미달 |
| H3_SEAL | NOT_OPENED |
| S4 | NOT_PASSED |
| S5 / S6 | NOT_RUN_THIS_SCOPE / NOT_ACCEPTED |
| GOAL1_READY / GOAL1_ACCEPTED | false / false |
| INDEPENDENT_ACCEPTANCE | PENDING_EXTERNAL_REVIEW |

새 SMALL optimizer256, input613,608/target69,629 tokens, anchor/focus draws1024/1024.
SMALL generation1,180 = C50 평가1,168 + F512 저장 parity12, 자체 teacher0, 외부 모델
호출0. A75 optimizer/generation0. 실패 직후부터 추가 SMALL 학습0이며 재시도·예산연장
없다. TINY optimizer71/scalar0: G1 14 + 학습 전 quick31 + 최종 저장참조 process12
+ 중복 파일을 기대하던 기존 assertion 실패4 + 수정 후 time-split 회귀10.
새 binary TINY generation/teacher148/148. 이전 bootstrap24는 재사용했으며 이번 신규
업데이트에 포함하지 않는다. quick의 다른 legacy generation/teacher는 전역 계측하지
않았으므로 위 수치를 전체 프로젝트의 모든 테스트 호출 수라고 주장하지 않는다.

최종 직접 binary unit9 PASS. G1 source RED/GREEN과 최종 process2 PASS, 이후
same-step 재사용 time-split process1 PASS. journal5/기존 archive unit3/version graph1/
canonical CLI process1 PASS. 모델 저장 unit3/cold1 PASS. 학습 전 quick59 PASS와
최종 all-target clippy/fmt 검사를 구분한다. 전체 무관한 테스트나 새 대규모 학습은 없다.
최종 `g5-binary-unit.log`, `g5-native-process.log`, `g5-reuse-final.log`, `g5-clippy.log`
및 각 G3/G4 로그가 실제 실행 증거다. compile 실패와 실패 assertion 로그도 남겨 두었다.

저장 측정 후 작은 중복 최적화도 적용했다. 같은 step의 durable checkpoint가 현재
weights·Adam·전체 TrainingState와 실제 file hash까지 일치하면 새 평가/terminal에서
그 immutable 파일을 재사용한다. 상세 종료 사유는 terminal에 보존한다. 모델 파일
status를 고쳐 쓰지 않는다. 새 process의 시간 분할/최종 취소에서 BYTES_WRITTEN=0,
weights/Adam/token/guard/f64 bits 동일성을 확인했다. 다른 상태면 재사용하지 않는다.
원본 F512115,285,312B의 저장비용은 G3 실측이며 TINY 절감량을 SMALL 실측 속도로
확대하지 않는다. 재개 중 이미 평가한 step을 다시 저장하지 않는 assertion도 포함한다.

### 새 binary와 남은 JSON의 실제 경계

| 경로 | 기준 상태 및 이번 실행 |
| --- | --- |
| experiment_record native run/close | typed R3ER; C50 run CANONICAL_JSON_WRITES=0, LEGACY_JSON_READS=0 |
| anchor-prepare / 기존 F baseline | frozen corpus/policy JSON read-only 검증 후 owned R3ER 등록; 새 JSON corpus/receipt writer 없음 |
| 모델 artifact | 기존 R3MODEL F32 + binary mapping/state; compatibility ID와 기존 mapping 의미 유지 |
| graph 원장 | RPV3 exact envelope + R3ARCH + R3JRN; binary writer/reader, JSON 상태 없음 |
| model.rs worker IPC | 기존 JSON request/response 유지; G2 direct native 경로에서는 사용하지 않음 |
| data.rs / quality_recovery legacy commands | 기존 JSON corpus/manifest/legacy import·과거 실험 경로 보존; 자동 변환·삭제하지 않음 |
| checker / validate 표시 | 기존 개발자 summary 및 parity JSONL 출력 유지; native controller나 제품 정답 입력이 아님 |
| 사용자 JSON 원문 | 사건 payload일 때 bytes 그대로 보존 |

### Candidate와 인가된 로컬 증거

Root=`artifacts/native-storage-quality-20260918/`. `candidate-source.diff`는 위 기준
source→최종 source의 src/tests/examples diff, `candidate-source.sha256`는 digest 목록.
실제 F512 read-only 부모는 `anchor-pair/C50/parent.r3m`, 원문 corpus는 기존
`artifacts/h3-controlled-20260917/a2/corpus-F/`와 `artifacts/goal1-corpus-v9/`,
ordinary는 `artifacts/s4-completion-20260917/binding-corpus/validation.json`이다.
등록된 실제 소유 입력은 `anchor-pair/{C50,A75}/inputs.r3er`. 실패 raw/terminal은
`anchor-pair/C50/segment-00/`; 해당 새 model checkpoint는 존재하지 않는다.
`model-storage/`는 G3 새 export/cold 복원, `graph-final-*`는 G4 synthetic 저장소다.
이 파일들은 로컬 검토 자료이며 Git에는 올리지 않았다. 운영 DB는 탐색/수정하지 않았다.

원본 보존 manifest 재확인 native6/6,data/policy/raw192/192 OK. 소유 모델/benchmark
process는 종료됐다. 임시 지시문에 대한 source/doc 의존은 없고 checker deny-pattern만
남아 있다. 새 영구 파일은 `src/journal.rs`, `tests/journal.rs` 두 개다.
최종 계약 대조에서 G1/G3/G4를 검증했으며 G2 full pair·H3/S4/S5/S6·Goal1과 외부
독립 수용은 남아 있음을 확인했다. 다음 단일 제안은 **동일 anchor 비율 비교의 저장
가능성 사전검증을 갖춘 재등록 검토**다. 새로운 LR/tokenizer/코어 가설은 이 실패로
뒷받침되지 않는다. 종료된 pair의 남은 예산을 자동 재사용하지 않는다.

## G4 — 분리된 graph journal 원형 검증

2026-09-18 / EXECUTED_THIS_RUN. R3JRN v1 writer/reader를 추가하고 기존 Archive의
Event/link/head 검증, version/current/as_of, 방향별 bounded BFS를 공유한다.
운영 SQLite와 product ask는 변경하지 않았다. snapshot의 exact file/source hash,
연속 sequence/previous digest, 요청/본문 hash, commit trailer를 검사한다. 단일
writer lock 및 sync 후 ACK, 같은 request/content 재시도의 동일 commit 반환,
부분 tail/중간 손상에서 verified prefix만 조회하고 새 파일로 명시 recovery를 검증했다.
원장 tail을 자동 truncate/삭제하지 않는다. 원문은 기존 RPV3 bytes 그대로다.

직접 journal5 PASS, 기존 archive literal/손상3 PASS, SQLite/archive version/BFS
회귀1 PASS, 별도 canonical CLI process1 PASS. 최초 회귀는 같은 사건의 복원 edge
순서 차이를 발견했고 공유 projection의 정렬을 고친 뒤 통과했다. 실제 child를
fsync 후 ACK 전 exit91하고 새 process에서 재시도/조회했다. header/body/trailer
절단은 독립 파일 fixture이며 실제 장치 정전 시험으로 주장하지 않는다.

최종 크기·지연 원자료는 local `artifacts/native-storage-quality-20260918/`의
`g4-final-{1000,10000,100000}.log`, stores=`graph-final-{1000,10000,100000}/`.
이전 `g4-measure-*`는 캐시를 먼저 데우던 parity 순서 문제를 발견한 개발 측정으로
보존했다. 최종판은 추가 archive lookup 없이 같은 SQL 결과와 대조한다. 각 반복의
사건 기록 시각은 실제 SQL 할당값이며 각 비교 안에서는 동일 canonical bytes다.

| 사건 수 | SQLite DB+SHM (WAL checkpoint 후0) | base100+raw journal | base100+zstd3 journal |
| --- | ---: | ---: | ---: |
| 1,000 | 962,560 | 145,963 | 47,970 |
| 10,000 | 7,802,880 | 1,456,281 | 366,027 |
| 100,000 | 74,145,792 | 14,821,974 | 3,574,980 |

각 raw/zstd1/zstd3에서 사건 전체 bytes와 관계 inventory100% 일치. 100K payload
19,237,000B, 평균192B, facts3,000/retractions1,000/explicit relations4,000/전체 파생
edges8,000. 반복·장문 한국어와 고정 RNG의 불규칙 ASCII를 사용했다. 기존 Event는
UTF-8 payload만 허용하므로 invalid UTF-8를 몰래 지원한 것으로 보고하지 않는다.
보존/선택 성능은 이 synthetic workload에만 해당하며 일상 대화 평균이 아니다.

100K의64/256KiB×raw/zstd1/zstd3 snapshot6개도 동일 source/edge를 검증했다.
256KiB/zstd3 전체5,007,464B = header144 + block directory2,860 + record offset
index2,000,000 + stored blocks3,004,460. Dictionary0. journal은 base15,867B +
header224B +999 frame header/trailer183,816B +stored bodies이며, body 안의 event
length4B×99,900은 압축 전 구조 비용으로 이미 body에 포함된다. 중복 합산하지 않는다.
Journal persistent adjacency/lexical index/backup0, RAM에서 metadata·versions·edges와
canonical overlay를 재구축한다. SQLite의 table별 dbstat/FTS/relations/indices 및
checkpoint 전 WAL과 이후 SHM 크기도 원자료에 분리 기록했다.

100K에서100-event durable commit P50/P95: SQLite7.832/17.991ms,
journal raw4.018/4.984ms,zstd3 4.013/4.971ms. 초기100개는 snapshot에 있어 journal
999commit, SQLite1000commit이다. durability는 NOT_EQUIVALENT(std sync_all 대
SQLite FULL+macOS fullfsync), 기능은 FTS/동시성/트랜잭션 범위가 달라 DB 전체의
우월성 비교가 아니다. 동일 query100건의 journal zstd3 P50/P95(ms):
get .001083/.001375,history .002500/.003000,as_of .001333/.001833,
4-hop BFS .005250/.005833. 다른 후보의 동일 항목도 로그에 있다. query 순서는
SQL→archive→journal이고 OS cache를 비우지 않았다. 영구/일반적 지연 보장 아님.
각 profile warm index rebuild3회와 fresh process3회를 수행했다. 100K 측정 전체
OS peak footprint156,893,808B; RAM BTreeMap allocator 비용은 따로 분리 측정하지
않았고 persistent bytes와 합쳐 작은 RAM 사용을 주장하지 않는다. mmap 없음.

GRAPH_JOURNAL=VERIFIED_PROTOTYPE. 저장 exact, 해당 조회 parity PASS; 일반 검색
recall/false-positive와 모델 ANSWER 품질은 NOT_RUN. 이번 G4 모델 호출/학습0.

## G3 — 모델 저장 측정, raw F32 기본 유지

2026-09-18 / EXECUTED_THIS_RUN. 기존 F512 .r3m을 새 경로로 추론용/재개용 export했다.
weights는 원래 native binary다. 이번 변경은 byte/timing 계측과 별도 cold probe이며
모델 wire1/tokenizer/수식/정밀도는 바꾸지 않았다. 저장·로드·손상 직접 회귀4 PASS,
clippy PASS. 원본 physical SHA bdc28e3f4b31ad5d600cf1c6ec0615b591deed9e26b069d9167f6a175f0a0e16
보존. 실행 `validate native-storage-measure .../C50/parent.r3m .../model-storage`.

| 실제 파일 | inference bytes | resume bytes |
| --- | ---: | ---: |
| raw native F32 | 38,432,768 | 115,285,312 |
| chunk zstd1 (cold probe 전체) | 35,644,097 | 106,633,647 |
| chunk zstd3 (cold probe 전체) | 35,672,283 | 106,551,831 |

weights38,420,736B, Adam76,841,472B. inference header12,028B(그 안 directory4,972B),
padding4B. resume header23,067B(directory15,051B),padding37B. Directory는 header에
포함되어 이중 합산하지 않는다. Resume 파일의 inference view도 Adam 실제 읽기0B,
총 읽기38,443,840B다. seek skip과 tensor 복사 로딩이며 mmap/zero-copy가 아니다.

3회 warm ready-to-infer298.7–325.4ms, resume view441.9–485.1ms. fresh OS process를
각3회 별도 실행했고 파일 cache cold를 주장하지 않는다. zstd1/3은 약7–8% 줄지만
decode/read/hash가 inference168–208ms, resume499–528ms를 추가해 raw를 유지한다.
encode/hash, tensor write, readback, file-sync, link, directory-sync, total을 분리 기록했다.
예: resume 첫 save286.36/232.89/226.45ms, file-sync6.08ms, link1.11ms,
directory-sync2.89ms, total756.00ms. 세부 반복 원자료는 `g3-measure.log`.

Cold는1MiB chunk의 독립 zstd frame과 raw hash, 전체 길이/hash를 검증한 측정 전용
포장이다. hot .r3m으로 복원한 전체 bytes가12/12 반복 일치했다. 기존 weights/tokenizer,
Adam·metadata bits를 포함한다. 별도6 process에서 동일 ordinary 앞2건을 생성해
raw logits/token/EOS/UTF-8/finish 출력 모두 일치: SMALL generations12,teacher0,updates0.
단2건의 상대 parity이며 품질 추정이 아니다. 근거 `g3-parity-*.jsonl`.
측정 process26.62초, OS peak footprint3,334,049,296B. probe는 whole-file buffer와
chunk 작업공간을 사용하고 allocator cache가 누적된다. 제품 최소 RSS로 해석하지 않는다.
BINARY_MODEL_STORAGE=VERIFIED, COLD_EXACT=MEASURED_PROTOTYPE. 모델 품질 판정 불변.

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
