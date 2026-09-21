# R3-CITATION-CONTINUATION-1.0 독립 A/B 재검토 — 2026-09-22

**독립 A: PASS — 범위 종료. B 실행·raw·재현 검증: PASS. 모델의 최종 인용 공동 품질: FAIL.**

| 판정 대상 | 결과 |
| --- | --- |
| Code / 독립 A | PASS — 아래 candidate와 준비물에 한정, 확정 제품 결함 없음 |
| Training | 완료 — 기존 신규3,040회·최종7,424 step을 검산; 이번 새 SMALL 학습0 |
| 독립 B 무결성·재현 | PASS — 전수 raw 재채점 및 실제 부모/endpoint 재생성 일치 |
| 최종 V development | PASS — 전체 train fit 수용과 구분 |
| Citation joint / 새 값+인용 기준선 | FAIL — support·ALL4·외부 ID 기준 미달 |
| Candidate / 실제 citation confirmation | NONE / NOT_RUN — 기존 seal 미개봉 |
| 기존 scalar4352 기준선 | PRESERVED — 이번 새 confirmation으로 세지 않음 |
| 조건부 전체 QA640 / S4·S5·S6 | NOT_RUN / NOT_ACCEPTED |
| GOAL1_READY / GOAL1_ACCEPTED | false / false |

검토 source: `c9c121077bbe1aadc546e344ab237d38728e3d3e`.
검토 시작 HEAD: `012a73b389026615da2b7550e6eb3dabc94b49ae`.
두 SHA 사이 제품 source/tests/Cargo 변경은 없다. 정확한 candidate를 Git archive로 복사했으며 현재 제품 source/tests/Cargo와 바이트 비교도 일치했다.
계약 전체 SHA-256: `c7cd88a86340ba5669d8ded44401f7b3be5cb77e704a748e28a02cb26d818587`.
실제 학습 source digest: `65cee12e4d3c0c4149a7b02c7026614eb93206c2c61f6bdcab97ce33d92bd0c8`.
실제 학습 executable SHA-256: `9f150bf0d5855c2956db8802ee91f3038389765afaaeddf1d336cd2868981d31`.

이번 검토는 제품·기존 tests·원본 모델/corpus/raw를 변경하지 않았다. 별도 source/target/TMPDIR/fixture/로그/보고서만 생성했다. 외부 모델·teacher/API, 새 SMALL optimizer, 봉인 확인문제 사전 열람은 수행하지 않았다. 사용자 추가 승인에 따라 기존 TINY optimizer 128회에 20회를 더해 누적 상한 148회를 적용했다.

**발견 이슈:** 현재 검토 범위에서 확정된 제품 코드 결함 없음. 명목상 수정이나 제품 패치를 요구하지 않는다. 아래의 모델 품질 미달은 정상 음성 결과이며 코드 PASS와 구분한다. 범위 밖 일반 QA·제품 전체·Goal1의 정상 동작을 승인한 것은 아니다.

**A의 실제 동적 검증**

- 정확한 candidate의 기존 회귀 2개를 새로 실행: `citation_continuation_guard_and_record_boundaries`, `citation_continuation_process`. 2 passed / 0 failed, 315.65초. 실제 TINY optimizer20, generation175, teacher104, finite-difference0.
- 별도 두 번째 candidate 사본에는 검토용 시험만 추가했다. 제품 함수는 변경하지 않았다. native confirmation timeout/length/I/O/cancel, guard receipt 재진입, 잔여 호출 예산, 임계값 인접 경계의 4개 시험을 실행했다. 합성 0/1/511/512행은 wire/분모 검사이며 모델 생성 횟수로 세지 않았다.
- 최초 추가 시험은 검토용 코드가 표시 문자열 `CANCELLED`를 기대해 1건 실패했다. 제품의 typed error는 `Error::Cancelled`이고 표시 문자열은 `cancelled`이다. 검토용 assertion만 typed error로 바꾸고 새 fixture에서 다시 실행했다. 최종 4 passed / 0 failed. 최초 실패 로그·fixture·binary를 그대로 보존했고 최초7회 및 재실행7회의 실제 generation을 모두 포함했다. 제품 source 수정은 없다.
- candidate 확정 직후, 호출 전 NOT_INVOKED, 한 행 RETURNED 뒤, 마지막 행 저장 뒤의 순수 TIME_BUDGET에서 별도 process 재진입을 확인했다. 연속8행과 분할8행의 실제 native tokens/raw가 같고 누적 generation은8이다. 마지막 완료 게시와 완료 결과 재진입은 generation0이다. TINY8행은 실제 SMALL confirmation512행 품질 통과가 아니다.
- command timeout 반환의 raw0/1 token 실패 행과 정상 length 행을 보존하고 다음 행만 호출한다. I/O resolution 실패는 이미 반환된 token 사용량을 보존하면서 pending 상태의 재시도를 차단한다. 취소도 호출0으로 차단한다.
- 기존 무작위 TINY 연속2 대 새 process1+1의 weights/Adam/training state가 동일하다. 평가-only 재개에서는 추가 optimizer0이며 실제 control-token 실패는 실패로 남는다. 정상 EOS fixture 시험과 이 실패 경로 시험을 구분했다.
- 남은 generation 한도가0일 때 이미 반환된8행을 위한 추가0 호출은 허용하고 추가1 호출은 거부하는 production 예산 함수를 실행했다.
- guard는 FULL47/48/49, 오류3/4/5, ALL4 11/12/13, 경고·회복·연속 경고를 검사했다. 같은 raw/step 재검산이 경고를 중복 증가시키지 않고, 이전 guard/raw 변조나 이미 종료한 경로는 거부한다.
- 정확한 candidate test executable의 실제 confirm/report 진입44회: 서로 다른 candidate/source/binary/policy/B/seal, mixed stop, UNKNOWN/pending/missing resolution, 누락·중복·잘림 raw, attempt gap/추가 ordinal의40건을 모두 거부했다. 정상4건은 통과했다. 각 진입 전후 reviewer fixture의 파일 목록/hash가 동일하고 추가 모델 호출0이었다. 의도한 거부의 test exit101은 상위 검토에서 검증한 기대 결과이며 정상 검사를 실패로 숨긴 것이 아니다.

현재 직접 확인한 경로는 `src/value_citation.rs`의 `confirm`(1680), `continuation_plan`(1853), `continuation_prepare`(1873), `verify_continuation`(1923), `retention_decision`/`continuation_guard`(1162/1168), `verified_confirmation`(2900), `src/binding.rs`의 `confirmation_usage`/`confirmation_prefix`/`confirmation_collect`(1789/1842/1891), `src/fresh.rs`의 `call_attempt`/`resolve_call`(2462/2548)와 직접 호출되는 native loader/generator 및 회귀다. 관련 없는 보안·저장·전체 안정화 영역으로 확대하지 않았다.

**부모·정책·실제 tape**

부모는 기존 ANSWER4384의 완결된 `QUALITY_REGRESSION`, `resume=false` endpoint이다. 기존 실패 판정을 PASS로 바꾸지 않았다. 새 initial 파일은 부모와 바이트 동일하며 weights/136 Adam tensors/step/sampler/token clocks를 native loader로 검증했다. family6/normalizer2, first-target1, QE/tokenizer, LR0.0003, batch8/accumulation1 및 원본 자료가 유지된다.

새 update 범위는 절대 step4385–7424, 원래 citation suffix의 index32–3071이다. 총3,040회, 매 batch V4+VC0 2+VC1 2, actual targets76과 objective denominator8은 서로 다른 수치이다. 새 exposure는 V12,160/VC0 6,080/VC1 6,080회다. 새 input3,623,680/target231,040/padding97,280; 누적 input8,710,144/target303,104와 일치한다. 기존32회를 포함하면 전체 citation3,072회이고 V 각8회, VC0/VC1 각4회 노출이다. 새 구간만 보면 V128개는7회·1,408개는8회, 각 VC pool64개는3회·1,472개는4회다.

실제 guard 경고 streak는4416에서0→1,4480에서1→0, 이후0이다. 부모60/64·ALL4 12/16은 새 경고가 아니다. prospective guard 변경은 연구 중단 정책에만 적용되며 최종 dev/train/confirmation 품질 기준은 유지된다.

**B 원자료 재검산**

독립 Rust reader가 update3,040행, native endpoint11개, 예정 decision7개, generation raw4,096행, 별도 teacher4,096행 및 prepared/resolved8,192쌍을 전수 확인했다. label은 request의 두 원문 record로 검산했으며 정답 resolver를 inference에 전달하지 않았다. 순서·case/prompt/모델/policy/tokenizer·attempt·native checkpoint 연결, EOS/strict decoding, token 비용, 분모, joint score, support 및 teacher 진단 재집계가 저장값과 일치한다. 누락/중복 평가, UNKNOWN, 미반영 optimizer/input/target은 발견되지 않았다.

완료 step7424의 native physical SHA-256은 `8c2e9b0f670be09acbae40efb6e2d4cb4e5a3c01315b0d7e3a51aa467270ec47`이다. terminal은 `Finished`, `FINAL_QUALITY_FAIL_AT_7424`, `resume=false`다.

| 최종 panel | FULL | QUERY_BOTH | SWAP_BOTH | ALL4 | 첫값 | 정확한 support | 유효 외부 ID | EOS |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| V512 | 511/512 | 255/256 | 255/256 | 127/128 | 511/512 | 해당 없음 | 해당 없음 | 512/512 |
| VC512 | 492/512 | 236/256 | 241/256 | 113/128 | 509/512 | 495/512 | 17 | 512/512 |
| ID변형512 | 495/512 | 240/256 | 243/256 | 116/128 | 510/512 | 497/512 | 15 | 512/512 |

최종 runtime error는0이고 citation 문법은 두 panel 모두512/512이다. 그러나 VC의 ALL4는116 기준 미달, support는 두 panel 모두508 기준 미달이며 외부 ID0 조건도 실패한다. ID_BOTH는477/512이다. V development 통과와 인용의 공동 품질 실패를 분리한다. 고정 train128의127/128·ALL4 31/32는 전체4608행 fit 검사가 아니다. 전수 train fit, 후보 고정, 실제 citation confirmation, 조건부 QA640은 선행 품질 gate 실패로 NOT_RUN이다.

5888의 VC474/512·ALL4 103/128과 ID488/512·ALL4 112/128도 재검산했다. 4416의 정상 length2행은 분모와 실패에 남는다. 첫값, grammar, ID 위치별 NLL과 gold-prefix EOS 진단도 재집계했으나 자유생성 FULL을 대체하지 않았다.

기존 scalar4352 confirmation254/256·ALL4 62/64는 파일·hash·기존 판정을 보존 확인했다. 이를 새 값+인용 모델이나 Goal1 수용으로 재사용하지 않았다. 미사용 citation seal은 registration/연결/hash/attempt 부재만 검증했고 원본 봉인 본문은 decode하지 않았다.

**이번 새 정상 재생성**

부모4384와 최종7424에서 각각 V16·VC16을 별도 process로 생성했다. 네 실행 모두16/16 기존 raw와 token·출력 문자열·EOS/finish/error가 일치했다. 부모의 오답과 인용 불능도 그대로 재현했다. 총64 generation/368 tokens, teacher0, optimizer0이다. 이 prefix 일치는 전체 panel 품질 통과를 뜻하지 않는다.

원본 CLI의 관측 기록을 덮어쓰지 않기 위해 정확한 candidate library의 `generate_observed`를 호출하는 독립 Rust reader를 사용했다. 동일 request/tokenizer/QE/normal-greedy/strict-UTF8/max32와 bounded timeout/cancellation을 사용하고 prepared/returned/raw/finished를 scratch에만 저장했다. expected answer/resolver/teacher를 generator에 넣거나 logits·EOS를 수정하지 않았다. 검토 helper executable은 원래 학습 executable과 구분한다. helper SHA-256: `e5f61fc2d1de686d11bc3ed222f1000be755ab08a69ddb538fe344868c043ea4`.

**실제 사용량**

| 이번 검토 신규 작업 | optimizer | generation | teacher | 생성 tokens |
| --- | ---: | ---: | ---: | ---: |
| 정확한 candidate의 TINY 회귀 | 20 | 175 | 104 | 175 |
| 첫 추가 fault 실행(검토 assertion 실패 보존) | 0 | 7 | 0 | 15 |
| 수정한 검토 assertion으로 새 fault 실행 | 0 | 7 | 0 | 15 |
| SMALL 부모32 + endpoint32 | 0 | 64 | 0 | 368 |
| 나머지 reader/negative/report/합성 wire | 0 | 0 | 0 | 0 |

TINY 누적은 이전 기록128/1149/704에 이번20/189/104를 더한 **optimizer148, generation1338, teacher808**, 새 FD0이다. optimizer 추가 승인 상한148회를 모두 사용했다. 의도한 TINY I/O fixture3건은 pending이지만 raw/반환 사용량이 알려져 있다는 사실과 자동 재시도가 금지된다는 사실을 구분했다. 과거 원장의 source-derived 항목·확인 불가 범위를 새 검증으로 바꿔 적지 않았다.

실제 연구와 이번 재생성을 합하면 SMALL 신규 학습3,040회, generation4,224, teacher4,096, 생성 token50,240이다. 이 중 이번에는 학습0/teacher0이다. 부모와 endpoint 네 replay의 측정 구간은 합계1.906088125초, 최대 process RSS323,698,688 bytes다. 학습 control의 기존 active 합계1,619.194935375초와 기존 부모/B 관측 포함1,624.452319209초는 원자료 재합산값이다. 이 수치에는 build·reader·주변 CLI 검증·wall idle 시간을 섞지 않았다. TINY suite의 wall 시간315.65초, 첫 추가40.16초, 최종 추가43.33초는 별도다.

**실행·불변성·게시 정보**

로컬 신규 증거는 `artifacts/citation-continuation-20260922-rereview-01/`에 보존한다. `source/`는 정확한 candidate, `harness-source/`는 검토용 시험 추가 사본이며 기존 제품을 수정한 파일이 아니다. `readers/`는 독립 검산/정상 native 재생성 도구다. 이전 검토 reader를 검토해 재사용하고 정확한 candidate의 Rust library/data 코드로 새로 빌드·실행했다. 이전 로그를 이번 실행 PASS로 세지 않았다.

핵심 로그: `logs/A-tests.log`, `logs/A-additional-tests.log`(보존한 검토 assertion 실패), `logs/A-additional-tests-02.log`, `logs/A-usage.log`, `logs/preparation.log`, `logs/B-recount.log`.

추가 핵심 증거: `logs/A-negative.log`, `logs/closing-usage.log`, 네 `logs/parent-*.log`/`logs/endpoint-*.log`와 각 scratch raw 디렉터리. 실행/컴파일 명령은 `compile-readers.zsh`, `replay-commands.zsh` 및 위에 기재한 Cargo 명령으로 보존한다. unique test는 최종 통과한 기존2+검토4=6개이며 최초 재실행이나 child process 수를 중복 합산하지 않는다.

제품의 순수 `fresh answer-mean-report`를 실제 원본 study에 두 번 실행했다. 두 명령 모두 exit0이고 출력 SHA-256이 `af5ddab96b719be9ca9d2669f116d0525eb4f5289fcb4dfa769d9c5691da5768`로 동일했다. 원본 comparison은 selected=null, eligible=false를 유지한다. 누락된 confirmation 증거를 생성하지 않았다.

검토 전후 제품 source/tests/Cargo·기존 문서·현재/부모/선행 모델·corpus·raw **28,361파일의 목록, 크기, SHA-256이 모두 동일**하다. SHA manifest 전후 hash는 `c38194682143b72110ba862a959960496298a3f5076516f8a59c5f0311428b70`, 크기/목록 manifest는 `8098ee2fe1025e66d4373b1d4db25e371e51aca3da48a93c1300a47c50129be5`다. 이 비교 후 허용된 본 보고서만 추가했다. 기존 A/B 승인·실패·confirmation과 학습 원장은 수정하지 않았다.

| 신규 핵심 로그 | SHA-256 |
| --- | --- |
| A-tests.log | `ab5f4087e29135b894ee4dc5d6e06d8d059db9640da5fa514b634f1e121af27f` |
| A-additional-tests-02.log | `101c02e588d19f90e5dd5b4d74249db55148deda5238c3ff0904551075ba356f` |
| A-negative.log | `7a869a2fbdedd4f95fd22d56f4246d27d5b64bafb66d602da19336868d574f02` |
| B-recount.log | `589c0c51857476976a51530361a1e2f1a982f93b2b208dde0048d7ab307a515c` |
| closing-usage.log | `4b42a643a98ad5c503b5baca5a657cd65d51348a17fc5a3d3bba22b8b8cf7c0f` |

정확한 candidate의 Cargo 명령은 격리 source cwd와 별도 CARGO_TARGET_DIR/TMPDIR, `VECLIB_MAXIMUM_THREADS=1 OMP_NUM_THREADS=1`에서 실행했다:

```text
cargo build --locked --offline --release --features accelerate,test-support --bin replica-train --bin replica-check
cargo test --locked --offline --features accelerate,test-support --release --bin replica-train citation_continuation_ -- --nocapture --test-threads=1
```

별도 검토 harness는 동일 locked/offline 옵션으로 `training::fresh::identifiable::binding::citation::tests::reviewer_` 필터를 실행했다. 전체 loss 수학·PyTorch·이전 전면 안정화는 반복하지 않았다. fmt/clippy/전체 release suite를 이번 실행의 PASS로 주장하지 않는다.

**테스트 체크리스트**

- unit: guard 임계값, 잔여 generation0/1, 기록0/1/511/512 분모.
- integration: production confirmation의 candidate/claim 등록, 순수 TIME 분할, 별도 process, native state와 teacher 관측 분리.
- regression: ANSWER4384 실패 유지, 기존4352 수용 보존, family6·Adam·LR·tape32, final-only 및 평가-only의 추가 optimizer0.
- malformed/boundary: candidate/B/seal/policy mismatch, 누락·중복·잘린 raw, ordinal/attempt gap, UNKNOWN/pending resolution, guard raw 연결.
- failure path: command timeout0/1 token, 정상 오답/length 재호출0, I/O known usage, 취소/mixed stop 차단, 실패 결과 보고의 무생성·불변성.

새 source 수정 요구는 없다. 이 A/B 검토는 닫으며 기존 학습도 정상 음성 결과로 종료된 상태를 유지한다. 새 학습·후보 승격·confirmation 권한은 부여하지 않는다. 게시 대상은 본 보고서 한 파일뿐이다. 검토 source SHA와 본 보고서 commit/remote SHA는 별개이며 report SHA는 정상 commit/push 후 전달한다.
