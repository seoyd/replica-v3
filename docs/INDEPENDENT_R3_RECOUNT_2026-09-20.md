# R3 독립 결과 재검산 — 2026-09-20

```text
CONTRACT_ID=R3-ACCEPTANCE-AND-QUALITY-CLOSURE-1.0
STAGE=REVIEWER_R3
REVIEWED_OBSERVATION_SOURCE=e71345e66e18d221db2e5cb43fb67569c6eace8e
IMPLEMENTER_REPORT_HEAD=96cc73ad33c6de914f159fdd86cbc3a0babb96a1
ACCEPTED_STABILIZATION_SOURCE=abd967980645a878f22069708daa9fbfce70bf87
PRIOR_R2_REPORT=898f57d7dd8109386f23549d391618c558097774
R3_RESULT=PASS_WITHIN_OBSERVATION_SCOPE
CONFIRMED_CODE_FINDINGS=0
MODEL_QUALITY_PASS=false
NEW_SMALL_GENERATIONS=32
NEW_SMALL_TEACHERS=0
NEW_SMALL_UPDATES=0
```

구현자의 Q0–Q2 이후 수행한 최종 검토다. 이전 R0–R2의15개 회귀와 P16은
이번 실행으로 다시 계산하지 않았다. 새 관측384행과 실제 원본 primary1,024행을
독립 재채점하고, 사전 고정한 counterfactual16개를 각 모델에서 정상 재생성했다.
보고된 수치와32개 출력이 일치했다. 모델 품질 회복이나 배포 승인을 뜻하지 않는다.

## 발견 이슈와 수정 권장

검토 범위에서 코드로 확정할 수 있는 신규 결함은 없었다.
Severity High/Medium/Low를 부여할 발견 이슈, 수정 대상 파일·함수 및 직접 패치는 없다.
이미 공개된 품질 미달을 새 코드 결함으로 바꾸어 보고하지 않는다.

검토한 추가 소스는 `src/fresh.rs` 테스트 모듈의411행이며 삭제0이다.
candidate 이후 HEAD의 차이는 보고 문서뿐이었다. 주요 확인 지점은
`PosthocSelector::load`(3778), `rows`, `observe_panel`(3919 호출·3928 원시행 저장),
`report`(3956), `posthoc_pairs`(3984 부근) 및 직접 매핑 회귀다.
실제 생성 경로의 `quality_recovery::observe_generation`(1136),
tokenizer preparation 및 `Transformer::generate_observed`(749)도 대조했다.

입력·checkpoint·정답·순서를 등록 및 재읽기 시 결합하고, 원시행/호출 receipt/
완료 summary를 검사한다. expected와 학습 resolver는 검증·채점에 쓰이며,
native generation에는 준비된 request token과 생성 제한이 전달된다.
모델·학습·loss·tokenizer·저장 production 구현은 이 diff에서 변경되지 않았다.
이 경로 밖 인증·권한·보안 영역은 검토하지 않았다.

## 출처와 입력 보존

검토 시작 시 HEAD 및 실제 origin/main은 위 구현자 보고서 SHA와 일치했고,
tracked 미커밋 변경은 없었다. 기존 `.DS_Store`는 보존했다.
정확한 source의 독립 복사본과 기존 의존성 cache를 scratch에 두고
Rust/Cargo1.98.1, `--locked --offline --release --features accelerate`로 빌드했다.
제품 source, 기존 tests, Cargo 파일의 변경은 없다.

| 실제 모델 | checkpoint: selector-consistency-20260919 아래 | native SHA256 |
| --- | --- | --- |
| C7168 | C-KEEP/segment-0002/step-007168 | b74292531bf00ab1c21157936254f1205835a9771b969991b5b5f59bf16e407a |
| S7168 | S-SELECT/segment-0002/step-007168 | 6c31d173c31a4f08e600f901289fa16d2779dd8639030d482a18b43b2bbbc8a0 |

실제 native training state는 두 모델 모두 absolute7168이다.
SMALL topology, F32, tokenizer 및 native model/state digest를 직접 확인했다.
공통 tokenizer ID는
`ec945ee5f3cbd87992bdfa13f199de2a671b94b64337dff04d85e01d982ab9ef`이다.
C8192를 C7168로 대체하지 않았다. 원본은 각 arm의
`eval-7168-dev512.r3rows`와 원래 native corpus의 validation512다.

동결 `diagnostic.r3b`의 physical SHA256:
`7bb13256a20ea492eba1f2a685d8e693e9fd2fb6ac4e33f9a02804cb4d45dc94`.
실제 serialized mapping은 C/D/E64개씩,48 base×4view이고 중복 ID/view가 없다.
과거 study의 고정 diagnostic hash, source mapping, inverse-selection 검증을 통과했다.
새 입력이나 정답으로 교체하지 않았다.

등록된4,129개 입력의 해시가 맞았다. 검토 전후 별도 inventory는
13,063파일·5,189,526,650 bytes에 대해 동일했다. 이전 R2가 보존한
12,209개 file/directory entry도 이번 관측 후 그대로였다.
양군의 마지막 historical terminal은 여전히 resume=false이며,
C8192/S7168로 종료된 원 연구 이력과 별도 observation root가 유지됐다.
운영 DB·개인 자료·봉인 final200은 접근하지 않았다.

## 독립 raw 재채점

별도 Rust 채점기로 raw token→bytes→strict UTF-8, EOS 위치, Generated receipt,
실제 text/error 및 prepared prompt를 확인했다. frozen corpus의 정답으로
exact를 계산한 뒤 raw의 자기보고 값과 비교했다. 원본1,024행을 모두 확인했으며,
그중 각192행을 동결 source ID로 연결했다. 새 selector는 양군 각192행이다.

| 모델 | 원본 full /192 | flipped full /192 | both | original-only | flip-only | neither | 같은 정상 출력 |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| C7168 | 155 | 8 | 0 | 155 | 8 | 29 | 184 |
| S7168 | 70 | 28 | 0 | 70 | 28 | 94 | 180 |

각 행에서 both는 원본 exact AND flipped exact다.
네 pair 분류의 합은 각192이며, null/error를 같은 정상 답으로 계산하지 않았다.
새 raw의 EOS는 각192, 오류·malformed citation·복수 distinct citation은 모두0이다.

| bucket /64 | C original / flipped / both | S original / flipped / both |
| --- | --- | --- |
| C: entity | 46 /0 /0 | 20 /14 /0 |
| D: context | 56 /0 /0 | 27 /6 /0 |
| E: time | 53 /8 /0 | 23 /8 /0 |

| 별도 진단값 | C7168 | S7168 |
| --- | ---: | ---: |
| primary full /512 | 436 | 350 |
| other five tasks full /320 | 281 | 280 |
| original base4 /48 | 23 | 7 |
| flipped base4 /48 | 0 | 0 |
| both base4 /48 | 0 | 0 |
| original / flipped body exact | 170 /15 | 135 /45 |
| original / flipped citation exact | 164 /21 | 87 /94 |
| flipped citation: selected / other provided / absent / none | 21 /165 /4 /2 | 94 /84 /8 /6 |

body/citation 값은 full exact를 대체하지 않는다.
C→S flipped full gain/loss=24/4, net+20; both gain/loss=0/0이다.
원본 C/D/E에서는85개 정답이 감소했다. 각 모델의192쌍 모두 실제 prompt digest가
달라졌고 두 제공 근거가 제외 없이 남았다. 동일한 입력이나 근거 탈락으로
이 결과를 설명할 수는 없다. 학습 간격·attention·optimizer 등의 원인은
이 관측만으로 확정할 수 없다.

## 독립32회 재현과 사용량

새 selector raw를 읽기 전에 metadata 순서만으로 다음 index를 고정했다.
C6·D5·E5이며 두 모델에 같은16개를 사용했다.

```text
C: 0,11,22,33,44,55
D: 67,80,93,106,119
E: 130,143,156,169,182
selection SHA256:
6bc2e7b8e9680d5c3c7f284a1973af2a5258fbd84a4e66df2e5086211a6decaa
```

생성은 독립 scratch Rust adapter에서 기존 RunControl과
`observe_generation(..., automatic_teacher=false)`를 재사용했다.
정확한 source 복사본은 변경하지 않았다. 별도 도구의 training wrapper는
기존 모듈 경로를 복사본으로 지정하고 검토용 child module을 추가할 뿐이다.
native/model/decoding 함수 본문을 수정하지 않았다. test-support는 제외했다.
Accelerate/CPU/F32, compute threads1, hard generation cap32를 사용했다.

각 호출 전에 PREPARED를 발행하고 반환 raw와 resolution을 저장했다.
새 root는 create-new이며 재시도·resume를 제공하지 않았다.
별도 재읽기에서32개 반환과 terminal을 확인했다.

| EXECUTED_THIS_REVIEW | C7168 | S7168 | 합계 |
| --- | ---: | ---: | ---: |
| generation 진입 / 반환 | 16 /16 | 16 /16 | 32 /32 |
| 구현자 raw와 token/text/error/EOS/prompt 일치 | 16 | 16 | 32 |
| generated tokens, EOS 포함 | 242 | 242 | 484 |
| EOS / error | 16 /0 | 16 /0 | 32 /0 |
| teacher / optimizer / backward | 0 /0 /0 | 0 /0 /0 | 0 /0 /0 |

TINY/scalar update도0, 미반환 사용량 UNKNOWN도0이다.
재생성 command는 COMPLETED, observed_conditions=[], resume=false로 닫혔다.
RunControl6.149158958초, OS wall6.18초, 최대 RSS334,659,584 bytes다.
로드·검증·I/O를 포함한 이 명령 측정치를 추론 성능 benchmark로 보지 않는다.

구현자의 이전416회도 실제 raw/RETURNED receipt/terminal과 대조했다.
416 반환,5,998 tokens, EOS416, teacher0이며 이번 신규 실행 수와 분리한다.
R1의16회, 구현자416회, R3의32회를 합한 계약의 SMALL generation은464회다.
새 checkpoint, 모델·Adam 갱신, 추가 학습은 없다.

## 테스트 체크리스트

| 분류 | 이번 실행 또는 검토 근거 |
| --- | --- |
| unit | 독립 빌드의 posthoc_mapping_and_binary_pair_counts:1 passed,0 failed |
| integration | preserved source-bound observer의 순수 report를 별도 process에서1회 실행:1 passed; 새 생성0 |
| regression | 별도 Rust 전체 재채점,32회 정상 출력 재현, 입력/원본/기존 terminal 보존 검사 PASS |
| malformed/boundary input | 직접 매핑 회귀의 순서/누락/정답/입력 변조4종 거부;12개 정상 pair·null pair·mixed citation fixture 확인 |
| failure-path | null/error 동일 출력 제외 및 실제 반환 receipt 대조; 이전 R1의 timeout/cancel/UNKNOWN/process 장애 수용은 별도 보고서를 유지 |

이번 직접 test 함수는2개이며32개 생성 문항이나 assertion 수와 합산하지 않았다.
재채점/재현 도구는 각각1회 실행, 모두 exit0이다. zero-test PASS는 없다.
기존 R1의15개 전체 회귀를 재실행한 것으로 기록하지 않는다.
whole quick/fmt/clippy/release gate를 새로 통과했다고 주장하지 않는다.
독립 release 빌드 및 직접 fixture는 통과했으며, 원 구현 보고서의 범위 밖
clippy 부채는 이번 수리 대상으로 확대하지 않았다.

## 증거와 최종 판정

로컬 검토 증거:
`artifacts/independent-r3-20260920-x7gy7x/`.
`evidence/selection.r3b`, `independent-rescore.log`,
`arm-0-independent-pairs.r3b`, `arm-1-independent-pairs.r3b`,
`independent-parity32.log`, `parity32/`, `final-receipts.log`,
`bound-pure-report.log`, `adapter-fixture.log` 및 보존 inventory가 있다.
Rust adapter와 독립 source/target도 같은 scratch에 남았다.
원자료·raw·checkpoint·도구 실행물은 게시하지 않는다.

| identity | SHA256 |
| --- | --- |
| reviewed code patch | 19ad3941dd99b9bc1c8cc79ef325ea62677e150612d2ce91d9c99543d7bf893c |
| preserved implementer observer | 542471cb669eeac618feae0a8fbb3b78a13ee3f42f5cf46cf943172cdd043fe0 |
| independent production-feature test binary | 6ee7db79fabce901140137bc7a3a65c2039fa4bf2e91397e4972d577a14616b0 |
| independent recount/generation adapter | a24b3a948e47c0033351d9e0ade130af40a9e174f62395ed854309a9ef37180e |
| independent rescore record | cf6b2983628d97847f2509cfacc6a0246dce55e40590ff46994bedf453cd83e4 |
| independent parity terminal | 26e0936b9710e655fee05b0165a724f7088c2ae68ace2aa4e1d70411b7b3f1ac |
| before/after inventory | e49d451b0c9992544d2070c2e460842bf7c79eed07455b9cb6ce56c656bafffb |

```text
EQUAL_STEP_POSTHOC_OBSERVATION=VERIFIED
RAW_RESCORE=PASS
PARITY_REPRODUCED=32/32
NEW_OBSERVATION_ADAPTER_REVIEW=PASS_WITHIN_SCOPE
S_SELECTOR_EFFECT=FLIPPED_GAIN_WITHOUT_JOINT_SUCCESS
RETENTION_TRADEOFF=OBSERVED_ORIGINAL_CDE_MINUS85
STABILIZATION_ACCEPTED=true (prior independent R2; not reissued)
HISTORICAL_STUDY=STUDY_INCONCLUSIVE_UNEQUAL_BUDGET
HISTORICAL_CANDIDATE=null
FINAL200=NOT_OPENED
H3/S4=NOT_PASSED
S5/S6=NOT_RUN_PREREQUISITE
GOAL1_READY=false
GOAL1_ACCEPTED=false
```

이번 결과는 같은 step에서 수행한 사후 관측이다. 과거 미완료 연구를
사전등록된 최종 실험 완료로 바꾸거나 실패한 모델을 소급 승격하지 않는다.
제안된 original/flip 노출 간격 비교는 다음 가설로만 남긴다.
간격이 원인이라는 증거는 아직 없으며, 실행 전 별도 승인과 고정 tape·예산·
수용/중단 기준이 필요하다. 이 검토는 새 학습 권한을 부여하지 않는다.

이 문서만 별도 report commit으로 게시한다. 검토한 source SHA는 위에 고정했다.
실제 report commit과 정상 push 후 origin/main 확인값은 최종 전달에 구분해 기록한다.
