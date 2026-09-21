# R3-ANSWER-MEAN-CITATION-1.0 독립 재검토

이번에 실제 실행한 수학·native/process 회귀와 B 원자료·출력 재현은 PASS다. 다만 조건부 confirmation의 시간 분할 재진입에서 확정 코드 결함 1건(Medium)을 확인했으므로 **전체 계약의 코드 수용은 FAIL**로 남긴다. 두 실제 모델은 각각 32 updates 후 정상적인 `QUALITY_REGRESSION`으로 종료됐으며, 모델 품질은 FAIL이다. 이 결함은 이번 두 모델의 점수·중단·후보 부적격 판정을 변경하지 않는다.

제품 source/tests/Cargo와 기존 모델·Adam·corpus·raw·수용 기록은 수정하지 않았다. 이 보고서는 기존 독립 보고서를 덮어쓰지 않는 후속 검토다. 새로운 학습·후보·confirmation 권한을 부여하지 않는다.

## 검토 identity와 실행 범위

| 구분 | 확인값 |
|---|---|
| Reviewed/training source | `70d933b4b798b9d73be5b5f99db4db1c980889b9` |
| Source 비교 기준 | `04b5624f0855d9f1ff35c1a8a4a48df459ef3d8b` |
| 검토 시작 시 report HEAD / 실제 remote main | `b735fef1a3fa2ea2dce27b2874934c38b45ef6ef` |
| 실행 source digest | `c1e876f97339cc8e5fec3422c3d2670bdb03d4b0db439829c6d318083043a736` |
| 기존 학습 executable SHA256 | `351abbcb522d894a54e241f8d9b7dfd5c4c9fc3c0da750ae7a4d875cad45e549` |
| 이번 scratch trainer SHA256 | `8d495ab510c208cfaca7b26463acb0bafe6803a55e8d7525cc2bdc746875f854` |
| 이번 scratch 재생성 reader SHA256 | `a5fd9a191c6ea323d01075091c586624b271909437a0108f5ff1c5bee4644fd7` |
| Preparation SHA256 | `7116514763d4dff682ffeb817f149da4877f0759b51339c488c72fc7b3cc7be0` |

Candidate 이후 시작 HEAD까지 제품 코드·테스트·Cargo diff는 없었다. Candidate를 `git archive`로 격리 복사하고 별도 target/TMPDIR에서 Rust 1.98.1, locked/offline, accelerate/test-support, compute thread 1로 build했다. source와 tests의 원본/사본 byte 비교도 일치했다. 학습 executable과 검토 executable은 서로 다른 파일이며 SHA를 혼동하지 않는다.

이번 증거 경로는 `artifacts/answer-mean-citation-20260922-rereview-01/`이다. 모델·원자료·scratch와 큰 로그는 게시하지 않는다. 본 문서의 report commit은 이 문서를 추가한 Git commit이며 reviewed source와 별개다. 게시 후 실제 remote SHA는 별도 publication 기록과 사용자 전달에 남긴다.

## 실제 A 시험

다음 명령을 candidate 사본에서 실행했다. `CARGO_TARGET_DIR`, `TMPDIR`, `R3_MEAN_TEST_ROOT`는 각각 이번 검토의 별도 target/tmp/A-process 절대 경로였고 두 compute thread 환경변수는 1이었다.

```sh
cargo build --locked --offline --release --features accelerate,test-support --bin replica-train --bin replica-check
../target/release/replica-check --output /absolute/review/A-quick quick --answer-mean
```

| 실행 | 실제 결과 |
|---|---|
| `answer_mean_` trainer 필터 | 4 passed, 0 failed, exit 0 |
| `inference_view_never_reads_adam_and_publication_never_clobbers` | 1 passed, 0 failed, exit 0 |
| 독립 scalar boundary fixture | zero/negative/large logits 6조합, malformed masks 4종 통과 |
| 독립 corpus/parent/tape/native/raw Rust readers | 모두 exit 0, 모델 호출 0 |

기존 로그 열람과 자식 process의 같은 테스트 호출을 고유 테스트 수에 중복 합산하지 않았다. quick summary SHA256은 `fdfb3f8a5423f7cccc33f428d6169f98fdba7ffd03171519eafb851f154c45f3`이다.

F64 log-sum-exp와 softmax 도함수로 scalar와 gradient를 확인했다. TOKEN의 분모는 실제 target 수, ANSWER의 분모는 예제 수였다. Prompt/padding 손실 0, EOS 포함, 빈 target 거부, 8/4+4/3+5/2+6 누적, permutation, 동일 길이 동등성, 2/17 혼합을 검증했다. 추가 boundary reader는 candidate의 loss 함수 본문을 그대로 추출하고 원본 문자열과 일치함을 assert한 뒤 별도 F64 계산과 비교했다. 제품 소스 사본을 수정하지 않았다.

실제 무작위 TINY의 nonzero gradient/Adam 비교와 random native 연속 2회 대 새 process 1+1회의 weights/state/Adam 48개 bitwise 일치를 확인했다. EOS fixture의 실제 trainer process 시험에서는 연속 2회, 1+1회, 마지막 평가-only 2+0회의 native/raw가 일치하고 평가-only optimizer는 0이었다. ANSWER native를 새 경로로 이동한 generic TOKEN 재개는 optimizer 0으로 거부됐다. Family 6/normalizer 2와 정확한 policy binding을 확인했고 inference load는 허용됐다. EOS fixture를 모델 품질 근거로 사용하지 않았다.

준비 검산은 V1536/VC0 1536/VC1 1536과 dev 3개 패널의 request에서 정답을 독립 도출하고 변형 관계·ID 분리·tokenizer·EOS·길이·고정 tape를 전수 확인했다. 두 군의 첫 32회 input/target IDs, mask, padding, 순서, LR bits가 같았다. Batch proof는 `f0aacbd92ab03bb18b78531435e3a75dc3e92ebbb0226b802eb70df525f0416c`이다. 두 initial native는 보호된 4352 부모와 물리 hash·weights·Adam이 일치했다.

Scalar 계수는 TOKEN V8/76, VC68/76, ANSWER 각 1/2이다. 이를 과제별 gradient norm이나 Adam 영향비로 해석하지 않았다. 실제 비용은 각각 input38144/target2432/padding1024였으며, ANSWER의 예제 분모 8을 target 사용량 76에 대입하지 않았다.

이번 TINY 사용량은 optimizer26, generation144, teacher116, scalar finite-difference8이다. 실제 fixture control은 optimizer18, 무작위 native 재개4, 무작위 gradient 비교4로 합계26을 검산했다. 기존 원장 86회에 이번 26회를 더하는 누적 상한112는 사용자가 이번 검토에서 명시적으로 허용했다. 누적 TINY 원장은 optimizer112/generation644/teacher520/finite-difference48이다. 추가 SMALL 학습은 0이다.

## 실제 B 재채점과 재생성

독립 Rust reader가 64개 실제 update row, sample indices, LR bits, native 상태/Adam, per-example CE, 목적 분모와 두 개의 1+31 segment를 다시 검산했다. 256개 원 generation row와 256개 teacher row의 준비/반환 journal, 실제 strict token decoding, request에서 도출한 정답, EOS/length, 지지 record ID와 공동 정답을 확인했다. 별도 summary reader는 제품 citation parser에 의존하지 않는 8자리 ID 검사도 수행했다.

TOKEN4384는 이전 VALUE-CITATION4384의 canonical raw128 전체, weights와 Adam136이 정확히 같았다. Physical native hash는 새 policy binding 때문에 달랐으며 tensor 동일성과 혼동하지 않았다. 이 검증을 위해 SMALL 32회 학습을 다시 실행하지 않았다.

| 실제 고정 패널 | FULL | QB / SB | ALL4 | 첫 값 | EOS | length | runtime 오류 | 올바른 support |
|---|---:|---:|---:|---:|---:|---:|---:|---:|
| 부모4352 V64 |64/64|32/32|16/16|64|64|0|0|해당 없음|
| TOKEN4384 V64 |41/64|14/14|5/16|58|49|15|0|해당 없음|
| ANSWER4384 V64 |60/64|28/28|12/16|60|64|0|0|해당 없음|
| TOKEN4384 VC64 |0/64|0/0|0/16|47|55|9|0|0|
| ANSWER4384 VC64 |0/64|0/0|0/16|50|64|0|0|0|

VC 두 패널 모두 올바른 syntax/단일 8자리 ID/지원 record 인용이 0이며 유효한 외부 ID 인용도 0이었다. 잘못된 prefix를 유효한 외부 ID로 세지 않았다. TOKEN V는 첫 값은 맞지만 뒤에 추가 문자열을 출력한 행이17개였다. 완결된 length 반환과 runtime/UNKNOWN을 구분했다.

같은 32회 비교에서 ANSWER는 V strict 정답19개와 ALL4 7개를 얻고 잃은 사례는 0개였다. 그러나 부모 기준 ALL4가16→12로 정확히4개 감소하여 정해진 guard에 걸린다. VC strict 개선은0개다. 두 군 모두 step4384, `Finished`, `QUALITY_REGRESSION`, `resume=false`, eligible=false다. 이후 +128/+384/... trajectory, dev512/ID512 및 전체 train gate는 **NOT_RUN**이며 광범위한 일반화 결론을 내리지 않는다.

정상 production generator인 `Transformer::generate_observed`에 같은 framing/prompt/요청 제한을 전달하는 작은 Rust reader를 candidate 라이브러리에 연결했다. 이 observer는 이미 선택된 token을 기록할 뿐 logits/EOS를 바꾸지 않는다. 원본에 출력 파일을 만드는 등록된 reviewer command를 재호출하지 않고, cancellation/요청 timeout/900초 상한과 durable prepared/returned 기록을 갖는 scratch reader를 사용했다. 기대 정답은 생성 호출에 전달하지 않았다.

별도 process 5개를 순차 실행했고 모두 exit0, 각각16/16의 raw tokens/문자열/finish/error/generation-completed가 원본과 같았다.

| 새 process | 이번 호출 | 새 raw SHA256 |
|---|---:|---|
| 부모4352 V16 |16|`7fe8da4c954dbe747e7755dff79c8129d7b382ac5265ec9284a758e7d65f1bde`|
| TOKEN V16 |16|`05755d7822eb058f7503a464590f8c6f3d8d2996dbc47153a30f42803fe3dcf2`|
| TOKEN VC16 |16|`8b40d136d74629269b8394d04a000739863dfccbbede6dd04a8182d7dd7d2d20`|
| ANSWER V16 |16|`7da2703cee6ea84c1d4bd057ed1c2cfdd46bf23b5a78412c5b014ee1a43c0301`|
| ANSWER VC16 |16|`7ad81c86cf94d8742fbc861c3645d87b3f66448539d4f4fcbead0d1b1762c165`|

이번 신규 SMALL generation80 = 부모16+B64, teacher0, optimizer0, UNKNOWN0이다. 기존 연구 원장 generation336에 이번80을 더하면416이며 teacher256/학습64는 그대로다. 각 군 실제 32회 비용의 합계는 input76288/target4864/padding2048이다. 새 process wall time은 각각1.41–1.65초, 관측 최대 RSS는272,924,672 bytes였다. 이는 이번 작은 재생성의 관측값이며 전체 장치 성능 보장이 아니다.

Candidate의 `fresh answer-mean-report`도 실제 실행하여 exit0을 확인했다. 원본에 누락 증거를 보충하지 않았다. Protected source/tests/Cargo/docs 및 새 연구·직전 인용 연구·4352 연구의 총11,666개 파일에 대해 실행 전후 목록·크기·SHA256이 완전히 같았다. 두 hash manifest의 SHA256은 모두 `93b458559ffbc3256f8458a949652c090ffb4ad909a61ec3b7a915e19bc90f86`이다. 이후 게시 변경은 본 보고서 파일 하나뿐이다.

기존 scalar confirmation 수용254/256·QB126/128·ALL4 62/64는 기존 기록으로 보존했다. 이번에는 이를 새 confirmation으로 실행하거나 재사용하지 않았다. 미사용 citation seal은 hash와 실행 이력만 확인했고 봉인 문제를 모델에 제시하지 않았다. 적격 ANSWER 후보가 없으므로 새 confirmation은 NOT_ELIGIBLE/NOT_RUN, 호출0이다.

## 발견 이슈 M1

**Severity: Medium — 새 ANSWER confirmation이 정상 시간 분할 후 재진입할 수 없음.**

**Location:** `src/value_citation.rs::confirm` 1645–1673, `src/binding.rs::orbit_observe` 1740–1757 및 `work` 953–960, `src/fresh.rs::write` 207–216 및 `call_attempt` 2497–2516, `src/codec.rs::publish_new_measured` 490.

**Evidence:** `confirm`은 기존 candidate record를 검증·재사용하는 분기 없이 항상 `write(confirmation-candidate.r3b, ...)`를 수행한다. `write`의 publisher는 hard-link로 새 파일만 공개하므로 동일한 내용이 있어도 `AlreadyExists`다. 이어지는 `orbit_observe` 역시 started record와 raw stream을 항상 새로 만들며 저장된 RETURNED 행을 읽어 건너뛰는 경로가 없다. 하위 `call_attempt`에는 확정 NOT_INVOKED의 다음 시도를 허용하는 로직이 있지만 이 경로에서는 그 전에 막힌다. `work` 역시 confirmation의 non-COMPLETED terminal을 일괄 sticky 오류로 처리하므로 candidate publication만 바꾸는 것으로는 충분하지 않다.

**Problem:** 공통 계약은 같은 적격 후보의 실제 미호출/시간 분할을 기존 호출 상태 계약으로 처리하고 RETURNED 행을 재생성하지 않도록 요구한다. 새 ANSWER 경로가 재사용한 단발 confirmation 함수는 이 조건을 충족하지 않는다. 완료되지 않은 정상 시간 중단이 영구적인 파일 중복 오류로 굳어진다.

**Impact:** 적격 후보와 독립 B를 확보한 향후 confirmation에서만 발생한다. 현재 두 군은 부적격이므로 이번 점수·학습 사용량·중단 판정에는 영향이 없고, 실패를 품질 PASS로 승격시키는 문제도 아니다. 원본 파일 삭제/receipt 편집 없이 합법적으로 미호출 나머지를 마칠 경로가 없다는 가용성·상태 전이 결함이다.

**Reproduction:** 정상 후보 연결 record를 저장한 뒤 첫 호출 직전 또는 일부 RETURNED 행 이후 순수 TIME_BUDGET으로 중단하고, 같은 source/policy/candidate/B/seal로 confirmation을 다시 시작한다. 두 번째 진입은 1646의 candidate publication에서 `File exists (os error 17)`로 막힌다. 이번 scratch fixture는 candidate의 실제 `write` 본문과 publisher로 동일 record 두 번 공개를 실행하여 이 오류와 원본 불변을 확인했다. 코드의 호출 순서가 나머지 근거다. 적격 SMALL 후보가 없으므로 전체 confirmation/time-split 모델 실행을 했다고 주장하지 않는다.

**Recommended Fix:** 일반 publisher의 overwrite 금지는 유지한다. `confirm`에서 기존 candidate의 정확한 identity를 검증하고 같은 후보에 한해 재사용하도록 한다. `orbit_observe`와 confirmation 사용량 검증이 durable raw/attempt 기록을 검증해 RETURNED는 재사용하고, 확정 NOT_INVOKED/pure TIME_BUDGET의 남은 행만 기존 허용 예산 안에서 실행하도록 한다. UNKNOWN·반환된 실행 오류·후보/B/seal 불일치는 계속 차단하고, 정상 최종 receipt는 한 번만 발행해야 한다. 정상 오답·length 반환은 보존하며 재생성하지 않는다. 제품 패치는 적용하지 않았다.

기존 process 회귀는 학습 평가-only 재개와 부적격 confirmation 거부를 검증하지만 적격 confirmation의 시간 분할 재진입은 검증하지 않는다. 테스트 공백을 실제 이번 원자료 오염으로 해석하지 않는다.

## 판정과 후속 테스트

| 판정 대상 | 결과 |
|---|---|
| 전체 계약 코드 수용 / A_CODE_AND_DYNAMIC 종합 | **FAIL — M1**, 아래 직접 실행 범위의 PASS와 구분 |
| CE_REDUCTION / OBJECTIVE_MATH_AND_RESUME | **PASS**, 이번 실제 회귀·독립 수학 fixture |
| DATA_TAPE_PARENT_PARITY | **PASS**, 전수 자료·batch·부모 검산 + 새 부모16 재생성 |
| TOKEN32_REFERENCE_REPRODUCED | **PASS**, raw128·weights·Adam 대조와 새 V16/VC16 |
| B_RESULT_INTEGRITY / OUTPUT_REPRODUCTION | **PASS**, 이번 독립 재채점·새 B64 재생성 |
| 학습 실행 | 두 군32회씩 완결 후 등록 guard에 따라 종료; 이후 trajectory NOT_RUN |
| ANSWER_VALUE_RETAINED | **FAIL**, ALL4 부모16→12 |
| ANSWER_CITATION_QUALITY | **FAIL**, VC64 FULL/support0; full dev512/ID gate는 NOT_RUN |
| NEW_CONFIRMATION | **NOT_ELIGIBLE / NOT_RUN**, 호출0 |
| BASELINE4352_PRESERVED | **PASS**, 기존 수용·원본 불변, 새 수용으로 재발행하지 않음 |
| S4/S5/S6·GOAL1_READY/ACCEPTED | **미수용 / false**, 이번 범위 밖으로 승격하지 않음 |

M1 수정 시 필요한 직접 체크리스트:

- **unit:** 같은 candidate identity 재진입은 기존 record를 검증해 재사용하고 다른 checkpoint/policy/source/B/seal은 거부한다. Publisher 자체의 overwrite 거부는 유지한다.
- **integration:** 적격 작은 fixture에서 confirmation 연속 실행과 시간 분할·새 process 완료의 raw/분모/최종 판정이 같고 실제 호출 합이 정확히512인지 확인한다.
- **regression:** 이미 RETURNED인 오답·length 행을 포함해 동일 행 재호출0, 새 후보 선택0, 학습 재개0, 기존 scalar 수용과 두 품질 실패 기록 불변을 확인한다.
- **malformed/boundary:** 0/1/511/512행 경계, raw 누락/절단/중복, ordinal·case·binding 불일치, 잘못된 candidate/B/seal을 거부한다.
- **failure path:** 후보 공개 직후·최초 호출 전·RETURNED 저장 후·최종 receipt 공개 전 순수 시간 중단을 구분한다. UNKNOWN/I/O/취소와 TIME_BUDGET을 섞어 재시도 권한을 늘리지 않는다.

시험 중 reviewer reader의 초기 컴파일 오류(명시적 crate link 누락, edition 예약 식별자, 함수 추출 끝 경계)는 scratch 안에서만 수정했다. 해당 실패 로그를 보존했고 모델 호출은0이었다. 제품 회귀나 모델 재생성은 재시도하지 않았다. 과거 전체 안정화/PyTorch 비교, 신규 SMALL 학습, 적격하지 않은 confirmation은 실행하지 않았다.
