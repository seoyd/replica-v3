# R3-VALUE-CITATION-BRIDGE-1.0 — 독립 A/B 실행 검토

검토 완료: 2026-09-22 KST. 실험 식별자는20260921을 유지한다.

**A_DYNAMIC: PASS. B_RESULT_INTEGRITY: PASS. MODEL_QUALITY: FAIL.**

정확한 candidate의 격리 빌드와 관련 회귀, 실제 준비자료 전수검산, 기존 확인 상태의 순수 보고, 실제4384 raw 재채점 및 새 process V16/VC16 재생성을 수행했다. 확인한 범위에서 제품 코드의 확정 결함은0건이며 추가 패치를 요구하지 않는다. 이는 전체 제품 무결함이나 모델 품질 통과 판정이 아니다.

|판정|결과|
|---|---|
|REVIEWED_SOURCE|`04b5624f0855d9f1ff35c1a8a4a48df459ef3d8b`|
|검토 시작 시 보고서 HEAD / 확인한 원격 main|`0a2b57508a94b5a5ee7c771a2acdd39ba57f6900`|
|A_DYNAMIC / 코드·준비 검증|PASS|
|B_RAW_INTEGRITY / B_RESULT_INTEGRITY|PASS|
|REPRODUCTION|PASS — 동일4384, V16/16·VC16/16, 오답·length 종료 포함|
|BASELINE4352_PRESERVED|PASS — 기존 confirmation254/256, ALL4 62/64 유지|
|학습 실행|32/3072 updates 후 QUALITY_REGRESSION으로 종료; 전체 예정 학습 완료 아님|
|VALUE_RETAINED|FAIL — 고정 V64 FULL41, ALL4 5, score errors15|
|CITATION_SUPPORT / 모델 품질|FAIL — VC64 FULL0, 지원 record 인용0|
|VC-ID의 실제 생성 품질|NOT_RUN — 교재 ID-only 관계는 검증했지만 해당 평가까지 학습하지 않음|
|CONFIRMATION_STATUS|NOT_ELIGIBLE / 신규 실행0|
|S4 / S5 / S6|NOT_RUN / NOT_ACCEPTED|
|GOAL1_READY / GOAL1_ACCEPTED|false / false|

A 수용은 변경 코드와 등록 준비물의 검증이다. 이미 품질중단된 연구에 남은3040회의 학습 권한을 새로 부여하지 않는다. B 수용은 실패한 결과가 정확히 기록되고 재현된다는 판정이다. 새 후보나 성공 review-b 승인 receipt를 원본 연구에 추가하지 않았다.

## 기준·권한과 실행 식별

공통 계약과 독립 검토 지시가 포함된 최신 기준서를 처음부터 읽었다. 기준서 SHA256은 `a3b2133401effc21a5a09b2271e0bd2be7f8af7ce1924ca5c2a54055b7e2097d`다. 별도 이름의 두 파일 대신 같은 CONTRACT_ID의 통합 본문을 사용했다.

제품 source/tests/Cargo.lock, 원본 corpus/model/Adam/raw/receipt는 읽기 전용으로 유지했다. 격리 source/build/target/TMPDIR/fixture/log/reader/report만 생성했다. Rust1.98.1, 기존 Cargo.lock, `--locked --offline`, CPU F32/Accelerate, compute thread1, heavy 실행 순차 조건을 사용했다. 외부 모델/API, 새 SMALL optimizer, 관측용 추가 backward는0이다.

§5의 QUALITY_REGRESSION 제외와 §10의 실패 endpoint 재현 요구가 현재4384에서 충돌할 수 있어 사용자에게 해석을 확인했다. 사용자는 **“§10 재현 적용: 4384에서 scratch로 V16·VC16만 생성”**을 선택했다. 이에 따라 original `citation::parity`의 승인 조건은 변경하지 않고, scratch 전용 entry에서 기존 close/validator/`orbit_observe`/native generation/scorer를 호출했다. 학습·후보·confirmation 자격은 그대로 false다. 이 명시적 예외를 다른 endpoint나 향후 실행에 일반화하지 않는다.

로컬 증거 root: `artifacts/value-citation-AB-independent-b3y7hJ/`.

|실행물|SHA256 / 용도|
|---|---|
|동결 production 실행물의 정확한 사본 `replica-train`|`ab4495bd4a6a4ae3b1ba58130fe2e438629f4d841cb03fe69711ee70fc9cadc6`; 기존·새 연구의 순수 report|
|정확한 candidate 직접 회귀 실행물|`1e51b6f2583f077b5117e023844d8272b53f53becbc6e1bb5730d0737546cdb0`|
|V16 scratch 실행물 `reproduction-V-executable`|`64330edf21726fa5fd78caae5ce2e7fc8be4c7757b29366d28a1ec20cb48e904`|
|VC16 scratch 실행물 `reproduction-VC-executable`|`20eff45b584630f89e1b9c11676decd98c9f717cb56d4c9fffc5c918ca6aea93`|
|candidate의 compiled source digest|`097e3dbbbc17e2a6557f8255cf718a65306bafc3690ad2214bf279cd466518fa`|

`source/`는 candidate의 Git archive 사본이고 original source/tests/Cargo와 동일하다. 독립 reader는 examples에만 추가했다. `reproduction-source/`는 별도 scratch 사본에 reviewer test entry만 덧붙였으며 기존 제품 함수 본문은 변경하지 않았다. 해당 entry는 `--features accelerate`로 빌드했고 `test-support`를 사용하지 않았다. 변경 내역·실행별 source·binary hash는 `logs/reproduction-harness-{only,final}.diff`, `logs/reproduction-harness-{initial,final}.rs`, `logs/*identity.sha256`에 보존했다. 이 두 reviewer binary를 원래 학습 binary와 같은 것으로 주장하지 않는다.

## A — 실제 실행 및 준비자료 전수검산

다음 candidate 회귀4개가 실제 통과했다. 필터0개 성공은 없다.

|검사|실행 결과|증거|
|---|---|---|
|`value_citation_data_scorer_and_native_boundaries`|PASS|`logs/direct-tests.log`|
|`value_citation_native_process_resume`|PASS|같은 로그, `fixtures/citation/`|
|`value_citation_v1_confirmation_and_reviewer_process`|PASS|같은 로그, `fixtures/.tmpHai6AL/`|
|`consolidation_t2_native_process_resume`|PASS|`logs/consolidation-process.log`, `fixtures/consolidation/`|

첫 Cargo 명령은 `cargo test --locked --offline --release --features accelerate,test-support --bin replica-train value_citation -- --nocapture --test-threads=1`이며 exit0, 3 passed였다. 두 번째는 동일 옵션에 필터 `consolidation_t2_native_process_resume`이며 exit0, 1 passed였다. `CARGO_TARGET_DIR`와 `TMPDIR`은 위 scratch root에 격리했다. `R3_FRESH_FIXTURE_EOS=1`을 명시했고 consolidation은 이번 citation 시험에서 이미 만든 TINY 부모를 재사용해 부모 학습4회를 반복하지 않았다.

V/VC를 함께 포함한 연속2회, 새 process1+1, 마지막 평가만2+0에서 native weights/Adam/clock/token/raw가 일치했다. 평가-only 재개의 optimizer는0이고 이미 반환·저장된 행이 보존됐다. 정상 최종 품질실패에서 reviewer 재현이 가능하되 후보·confirmation·추가 학습은 거부됐다. 이는 작은 EOS tensor fixture의 실행 경계 검증이다. 별도 무작위 TINY 시험의 control-token 오류도 그대로 재현·보존했으며, 이를 정상 답변이나 SMALL 품질로 계산하지 않았다. 실제 SMALL5120/7424 학습은 반복하지 않았다.

독립 Rust reader `review_a`와 `review_groups`는 exit0이었다. 생성기 정답 함수를 호출하지 않는 별도 request resolver가 제공 기록의 key/context/value와 선택 event를 직접 읽었다.

- 원래 V train1536 및 dev512의 request/answer/원문순서는 부모와 동일하다.
- VC0는 질문의 출력 지시와 답만 변경한다. VC1 및 VC-ID는 이에 대응하는 event ID와 gold 인용만 변경한다.
- train/dev 총6144행에서 request 기반 정답, tokenizer roundtrip, 두 근거 제공·제외0, 길이 상한, prompt 충돌0을 확인했다.
- 모든4-view에서 두 질문·두 배정 관계와 key별 event ID의 일관성을 확인했다. V/VC0/VC1 및 Vdev/VCdev/ID의 semantic skeleton이 같다.
- train384 / dev128 / 사용된 기존 confirmation64 / 새 예약 confirmation64는 서로 분리된다. 유한 전체1260 중 기존 사용576, 새 예약 전 잔여684, 예약 후620이다. 새 ID가 새 독립 장면512개를 뜻하지 않는다.
- 기존 distinct ID512의 중복 사용은 보존했다. 새 train ID768, dev ID256은 서로 및 기존/사용된 확인/새 예약 ID와 겹치지 않는8자리 값이다. 각 질문·배정에서 선택 관계를 교차 확인했다.
- 새 confirmation은 기존 A에서 봉인된 동일 파일의 hash와 예약 skeleton을 검증했다. 이번 검토에서 새 봉인을 만들거나 모델로 확인자료를 평가하지 않았다. 새 봉인 정답을 출력·게시하지 않았다.

|예정3072-update tape|Pool 행|행별 노출|총 sample|input / target(EOS 포함)|
|---|---:|---:|---:|---:|
|V|1536|8|12288|1781760 / 24576|
|VC0|1536|4|6144|940032 / 104448|
|VC1|1536|4|6144|940032 / 104448|
|합계|4608|—|24576|3661824 / 233472|

모든 batch는 서로 다른4 base의 두 질문씩이며 V4/VC0-2/VC1-2다. Padding98304는 소비 input·response CE에서 분리했다. V의 prompt/input/target은144/145/2, VC는137/153/17이다. 같은 sample 수를 같은 input/target 문자열·token 수 또는 gradient 비중으로 해석하지 않았다. 실제 batch CE가 response-token 가중 sample CE와 F32 오차 범위에서 일치했다. 과제별 gradient norm은 NOT_MEASURED이며 기록된 grad/delta는 전체 batch 값이다. 추가 backward로 이를 보충하지 않았다.

초기 native는 보호된4352와 byte-identical이다. physical `d5d1b61e4df7cc7bfa911a03766006d71f7e883b870e55283ef85e4b4ebe62da`, tensor `18c1ff90dd9be84ede87f05698a4d06b8436c7b2badd4b53176312197b44dd4a`, Adam136 `ddc38f4983d003c08f4d8db847875d844541161ae943e9e055a11708e0437f72`를 직접 확인했다. step/sampler4352, input5048320/target69632, LR3e-4/QE/first-target1/기존 tokenizer와 원 tape prefix가 일치한다.

## 기존 confirmation 보고와 원본 보존

동결 실행물의 `fresh consolidation-report --study artifacts/rebind-consolidation-20260921-study`가 exit0이었다. 원래 candidate comparison은 semantic hash `b11f8ce9a6af0d99cb73f11cec99ccf389746fc553cdd746f3ac89166e16da48`를 유지한다. 현재 투영은 `COMPLETED_PASS`, FULL254/QB126/SB126/ALL4 62, EOS256/errors0, minimal binding=true, Goal1=false다. 과거 comparison 내부의 NOT_OPENED를 현재 상태로 혼동하지 않는다.

reader fixture는 실제 미개봉, 부분 실행, 완료 품질실패, pending, 모델/seal 불일치, 중복·누락 행을 구별했다. 실제 과거 자료의 재채점은 신규 generation0이다. 접근 거부의 상태 매핑은 코드로 확인했으며 별도 OS 권한 실패 주입은 이번 실행에서 반복하지 않았다.

검토 전후 source/tests/Cargo/기준서, 새 연구 전체, 기존 consolidation 연구 전체 및 동결 실행물의 **파일10394개·디렉터리14개·2207492174 bytes**가 경로·크기·SHA256까지 동일했다. 두 inventory의 hash는 모두 `a2f27b83250070ae884fd41d089a424e1b7620b22f35c92a38774b776ca42142`다. 별도 원본128개 manifest도 종료 후 다시 검증했다. 누락된 original reviewer/confirmation/다음 segment 증거는 생성되지 않았다.

## B — 실제4384 원자료와32회 재생성

`review_b`는 actual corpus/metadata, native checkpoint, raw/teacher, prepared/resolved call journal 및 update trace를 읽어 exit0으로 재집계했다. source·policy·tokenizer·framing·같은 checkpoint 연결이 일치했다. 반환된 오류와 비EOS 행을 분모에서 제거하지 않았다.

|고정 개발 표본|FULL|QUERY_BOTH|SWAP_BOTH|ALL4|EOS / score errors|첫 값|
|---|---:|---:|---:|---:|---:|---:|
|부모4352 V64|64/64|32/32|32/32|16/16|64 / 0|64/64|
|4384 V64|41/64|14/32|14/32|5/16|49 / 15|58/64|
|부모4352 VC64|0/64|0/32|0/32|0/16|64 / 0|34/64|
|4384 VC64|0/64|0/32|0/32|0/16|55 / 9|47/64|

V FULL 하락23·ALL4 하락11·score errors15는 각각 등록된16/4/4 guard를 넘는다. V15/VC9 오류는32-token length 종료이며 raw.error0, 미반환 UNKNOWN0이다. VC의 syntax/provided/support/full 정답은 모두0이다. `outside_id=10`은 malformed `[event:`도 포함하는 기존 parser 진단으로, 유효한 외부 ID10개가 파싱됐다는 뜻이 아니다. 모든 parsed-ID는 null이었다. 일부 첫 값 정답 개선을 인용 능력 또는 전체 문자열 성공으로 바꾸지 않았다.

실제 학습은 새 process1+31=32회, committed32/discarded0/UNKNOWN0이다. Trace4353..4384와 Adam/sampler4384가 일치했다. V128/VC0-64/VC1-64의 고유행을 각각1회 소비했다. 실제 input38144/target2432/padding1024, 누적 input5086464/target72064다. 기존 관측은 SMALL generation208/teacher128이며 부모 관측80회를 포함한다. 계상 active39.200256918초는 전체 검토 wall time이 아니다.

동일4384 physical은 `2e620f3c2b958cca0e6b72ded677a1c8db0e330bcc38779939a8b55e034cfd13`, tensor `6058acb491de09d64a99743cb1c47e2da147e8125240f36116eb813c52766ae3`, Adam136 `91ae9a1d9a35ea8cec38c88ec17f87e878ee12a9d1f805f80a9f0ac8e9e4ca53`다. V/VC 평가 raw가 이 동일 endpoint에 결속된다.

사용자 예외 허용에 따른 새 process 두 개에서 metadata 첫16행을 그대로 사용했다. 표본의 오답을 교체하지 않았다.

|새 생성|기존 raw와 출력 일치|FULL/QB/SB/ALL4|EOS / errors|새 사용량|
|---|---:|---|---|---|
|V16|16/16|12/4/4/1|16 / 0|generation16, teacher0, update0|
|VC16|16/16|0/0/0/0|15 / 1|generation16, teacher0, update0|

Raw token·strict 문자열·EOS/finish·error/error_class·실제 prompt/framing/제공 근거가 일치했다. VC16의 length 종료1개도 그대로 일치한다. 모든32 call은 RETURNED이며 UNKNOWN0이다. 재생성 RunControl active 합2.609849791초, 연구 전체 계상 generation은240(기존208+새32), teacher128, optimizer32다. 이32 generation을 기존 학습 실행물의 original ledger에 덧쓰지 않고 독립 증거로 분리했다.

V16 raw hash: `0e41f2f1f3b1f0036316e01a02f6e0088534325a7e8232361c1dfa242e7e77f4`.
VC16 raw hash: `c691f85a15c6262fa81c69549159b95d25f79eca5ef4a7e83cb011fa1646e68c`.

실행·비교 과정의 scratch 실패도 보존했다. V16 첫 process는 generation16 및 기존 parity16을 완료한 뒤, reviewer가 추가한 `generation` 객체 전체 비교에서 서로 다른 실행시간 때문에 exit101이었다. 시간은 계약의 출력 일치 조건이 아니다. VC harness에서 이 비교를 제거했고 VC16 process는 exit0이었다. V16은 재생성하지 않았다. 두 durable raw를 별도 reader로 재검산하여 generation의 token/count/finish/cache와 계약 필드가 모두 일치함을 확인했다.

사용량 reader의 최초 집계는 V1의 synthetic256 gold rows를 모델 호출로 잘못 포함했고, 첫 재생성 reader는 generation-only receipt에 없는 optimizer 필드를 숫자0으로 단정해 실패했다. 각각 fixture 표시를 검증해 synthetic 행을 제외하고 observation receipt schema를 구별한 뒤 순수 읽기로 수정했다. 원 실패 로그와 최초 reader source는 보존했다. 이 수정들로 추가 모델 호출은 발생하지 않았으며 제품 결함이나 제품 테스트 PASS로 바꾸어 세지 않았다.

## 호출 예산과 검증 한계

이번 검토의 신규 실제 사용량은 TINY optimizer16/generation154/teacher104, SMALL optimizer0/generation32/teacher0이다. TINY 사용량은 distinct actual RunControl26개에서 합산했고 synthetic256행은 제외했다. 기존 확인량44/430/304와 합하면 TINY60/584/408이며, 과거 최초 실패의 generation UNKNOWN(소스상 최대4)은 별도로 유지한다. 전체 상한96/1024/1024 안이다.

기존 B 숫자를 재실행한 학습 결과처럼 보고하지 않았다. 확인된 최종 VC-ID/full-dev/full-train 모델 점수는 없고 새 confirmation도 없다. 과제별 gradient norm, 전체 release/기존 fmt·strict-clippy 부채, S4/S5/S6는 이번 PASS 범위에 포함하지 않는다. 과거 안정화 전체, LibTorch/PyTorch 대조, 저장 benchmark를 반복하지 않았다.

## 발견 이슈와 테스트 체크리스트

**확정 제품 코드 이슈: 0건. Severity High/Medium/Low로 등록할 신규 결함 없음. 수정 권장: 추가 제품 패치 없음.** 값 보존·인용 품질 실패는 실제 관측으로 유지하며 코드 결함으로 추측하지 않는다.

|관점|실행·확인 범위|
|---|---|
|unit|request 기반 gold, V/VC/ID 변형, strict scorer의 wrong support·foreign ID·format·누락 행, tape index/balance|
|integration|정확한 candidate build, native writer→confirmed reader, TINY 실제 trainer/evaluator/새 process, 기존·현재 순수 report|
|regression|연속2 vs1+1, 평가-only2+0의 optimizer0·raw 보존, 정상 품질실패 재현 후 후보/confirmation/resume 차단|
|malformed / boundary|512/1024/1536/3072행·512 LR 배열 writer/reader, 중복·누락 raw, 다른 모델/seal·불일치 label, 정해진 seq/generation 상한|
|failure path|무작위 TINY 오류, pending/불완전 확인 상태, 실제4384 품질 guard, length 종료 보존, 원본 누락 증거 미생성|

주요 production 검토 위치는 `src/binding.rs`의 `confirmation_current_checked`, `consolidation_reproduction_endpoint`, `consolidation_report`; `src/value_citation.rs`의 `variant/check_variant`, `score_citation`, `evaluation_result`, `close/report/parity/confirm`; 직접 연결된 `fresh.rs`, `quality_recovery.rs`, `check_main.rs`와 같은 파일의 직접 회귀다. 무관한 인증·외부 대상 영역으로 확장하지 않았다.

## 증거와 보고서 게시

원본은 `artifacts/value-citation-20260921-study`, 구현 증거는 `artifacts/value-citation-20260921-evidence`에 있다. 이번 독립 증거는 위 scratch root의 다음 파일에 보존했다.

- `logs/direct-tests.log`, `logs/consolidation-process.log`: candidate 회귀4개, 모두 exit0.
- `logs/A-independent-data.log`, `logs/A-independent-groups.log`: 준비자료 및 semantic 결합군, 모두 exit0.
- `logs/A-old-confirmation-projection.log`, `logs/B-production-report.log`: 동결 production 보고, 모두 exit0.
- `logs/B-independent-raw.log`: actual raw/teacher/trace 독립 재채점, exit0. 이 로그의 parity NOT_RUN 문구는 추가 허용 전 raw 검산 시점 및 original CLI 경로 상태다.
- `logs/B-reproduction-V16.log`: 시간 비교 때문에 exit101; native16회 및 RETURNED raw는 보존.
- `logs/B-reproduction-VC16.log`: 새 VC16 process, exit0.
- `logs/B-reproduction-pure-closure-corrected.log`: 양쪽16행 순수 재검산, exit0; SHA256 `4e9f91669e78ac41bcb62b26dfb56017f3204670fc828baa740f5ed9df7e6247`.
- `logs/review-usage-corrected.log`, `logs/original-{before,after}.tsv`, `logs/post-original128-verification.log`, `logs/evidence.sha256`: 실제 사용량·보존·증거 hash.
- `reproduction-V16/`, `reproduction-VC16/`: 각각 raw 및 immutable prepared/resolved/finished receipts. 신규32회 외 추가 generation 없음.

게시 범위는 이 검토 보고서 파일 하나다. Reviewed source SHA는 위 candidate로 고정하며 보고서 commit SHA와 구분한다. 보고서 commit 및 push 후 확인한 전체 remote SHA는 최종 전달 응답과 로컬 `PUBLICATION.txt`에 별도로 기록한다. source/tests/Cargo/기준서/원본/모델/raw/봉인자료/scratch는 게시하지 않는다.
