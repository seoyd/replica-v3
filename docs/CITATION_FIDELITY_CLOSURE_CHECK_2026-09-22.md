# Citation fidelity — 독립 검토 종결 확인, 2026-09-22

**A: PASS 유지. B 무결성·재현 검증: PASS 유지. 모델 품질: FAIL.**

R3-CITATION-FIDELITY-CONSOLIDATION-1.0의 공통 기준서와 검토 지시문 전체를 읽고, 기준 source와 candidate의 실제 변경 경로를 검토했다. 동일 candidate의 독립 A와 B가 이미 완료되어 있었다. 이번에는 완료된 TINY/process 시험과 SMALL 재생성을 반복하지 않고, 정확한 source의 새 offline build와 독립 Rust reader 실행으로 현재 원자료·기존 실행 증거·판정의 연결을 다시 확인했다. 기존 로그 열람이나 순수 재채점을 이번 신규 모델 실행 또는 신규 제품 동적 시험 PASS로 세지 않는다.

| 판정 | 결과 |
| --- | --- |
| RESULT / CODE_CHANGED_SCOPE | 검토 범위 종결 / 확정된 제품 코드 결함 없음 |
| A_VERDICT | PASS 유지 — 동일 candidate의 실제 3개 시험 및 준비 검산 확인 |
| TRAINING_EXECUTED | 기존 실행 3,072/3,072 updates 완료; 이번 학습 0 |
| PARENT_PRESERVED | PASS — 기존7424의 품질 실패·종료·재개 금지 상태 유지 |
| LATEST_DURABLE | step/sampler10496, segment-0008/final |
| B_INTEGRITY / REPRODUCTION | PASS / 기존 V16·VC16 및 별도 승인된 오답4 재현 증거 확인 |
| MODEL_QUALITY | FAIL — VC의 제공 근거 밖 ID가 2개이며 허용값은 0 |
| FIT_VERIFIED / CANDIDATE_ELIGIBLE | NOT_RUN / false |
| CONFIRMATION_STATUS | NOT_RUN; 적격 후보 없음, seal 미개봉 |
| VALUE_CITATION_SCOPE_ACCEPTED | false |
| QA_GAP_STATUS / S4 / S5 / S6 | NOT_RUN / NOT_ACCEPTED / NOT_ACCEPTED / NOT_ACCEPTED |
| GOAL1_READY / GOAL1_ACCEPTED | false / false |
| 기존 scalar4352 수용 | 보존; 새 모델 품질 판정에 대입하지 않음 |

**검토 동일성**

- 기준 source: `c9c121077bbe1aadc546e344ab237d38728e3d3e`.
- Reviewed candidate 및 실제 학습 source: `820100a6a27eebefe0ba723fc03948ea6f2a0abb`.
- 검토 시작 report HEAD: `1fe4881220bd57d5ee720351f8e90655cce55b00`. 게시 직전 실제 origin/main도 이 전체 SHA와 일치했다.
- 기준 대비 제품 변경: 5개 파일, +208/−44. `binding.rs`, `check_main.rs`, `fresh.rs`, `quality_recovery.rs`, `value_citation.rs`의 실제 변경 및 연결된 상태·채점·confirmation 경로를 검토했다.
- Candidate 이후 source/tests/Cargo diff0. `git archive`로 만든 새 scratch의 source/tests/Cargo도 현재 제품 파일과 동일했다.
- Compile-time training source digest: `c8ee368de6ef5bcdb31ddcf85d42cb6485fe5278d2658e4119f0f13ba8047fcc`.
- 실제 학습 binary SHA256: `b0f974de8ffe5ff9aa0be81978c57b8b21dc07ac96182ea55b4aacbaf93f108c`.
- Preparation SHA256: `0d90de3e17d3a94e6bb8878277d8aaa0bbaa3a4b3be70ead74c39443ee926a8b`.
- Final10496 물리 checkpoint SHA256: `7845eb2e66bf333f07a4fde28d80601f418f212912c7c7b58fc41424646b5ff6`.

이 파일을 담는 commit은 보고서-only commit이다. 위 reviewed source와 구분하며, 실제 report commit/remote SHA는 게시 후 별도 인계한다. 보고서의 자기 commit SHA를 본문에 순환 참조하지 않는다.

**확인한 구현과 준비자료**

`value_citation.rs:1889`의 parent 검증은 Finished/resume=false, ANSWER arm, continuation profile, 실제7424 및 FINAL_QUALITY_FAIL_AT_7424를 요구한다. Native state, sampler, family6/normalizer2, QE, LR3e-4, batch8, Adam 및 입력 파일 연결은 이어지는 준비·검증 경로에서 검사한다. 이번 reader도 부모의 136개 Adam tensor와 native state를 직접 읽고 비교했다.

`value_citation.rs:1860`의 새 plan은 부모 tape가 소진됐는지 확인하고 원 citation cycle을 한 번 추가한다. Prefix7424는 원본과 같고 신규7425는 suffix index0,10496은 index3071이다. Corpus4608·dev1536·metadata·tokenizer는 부모와 byte 단위로 같다. 실제 token/mask를 전수 계산했으며 prompt와 padding은 target에서 제외되고 EOS는 포함된다.

| 새 구간 | Updates | Input | Target | Padding | V 각 행 노출 | VC0/VC1 각 행 노출 |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| 7424→8960 | 1,536 | 1,830,912 | 116,736 | 49,152 | 4 | 2 |
| 7424→10496 | 3,072 | 3,661,824 | 233,472 | 98,304 | 8 | 4 |

모든 batch의 구성은 V4/VC0 2/VC1 2이다. 실제 target token76과 answer-mean 분모8이 각각 유지된다. New input/target은 한도4,000,000/260,000 이내다. 최종 누적 input12,371,968/target536,576이다.

`value_citation.rs:51,70,867`의 예정 평가와 저장 경계를 대조했다. `evaluation_result`의 eligibility는 development와 full fit, guard를 모두 요구한다(`:1116`). 적격이면 즉시 CANDIDATE_FIXED, 최종 미달이면 FINAL_QUALITY_FAIL, 이후 extend=false이다. Train128은 full fit4608로 판정되지 않는다.

`fresh.rs:2074`는 닫힌 run을 거부한다. `:2088`의 호출 한도 equality 예외는 EvaluationPending에만 적용되며, 실제 남은 호출 한도는 0으로 유지된다. `:2159`의 평가-only 분기는 trainer 없이 기존 native를 보존한다. 이 변경의 실제 최종 teacher-cap/process 검증은 동일 candidate의 기존 A 시험에서 수행됐다.

`seal_receipt`/`seal_owner`는 fidelity→continuation→answer-mean→기존 citation seal 연결을 확인한다. `confirm`은 적격성, 동일 endpoint의 독립 B 및 V/VC 관측을 검증한 후 동일 seal의 소유 claim을 사용한다(`value_citation.rs:1685`). 품질 실패의 `mean_review` 허용은 학습 재개나 candidate 권한을 바꾸지 않는다(`:2205`). 이번에는 seal 본문을 decode하지 않았고 candidate·confirmation 증거를 생성하지 않았다.

**실제 명령과 이번 실행**

Scratch: `artifacts/citation-fidelity-20260922-closure-check/`. 모든 새 reader·로그·target은 이 안에 있다. Rust1.98.1, 기존 Cargo.lock, offline cache, accelerate, compute thread1을 사용했다. 외부 모델·API·teacher 호출은 없다.

| 이번 명령/작업 | Exit | 실제 범위 |
| --- | ---: | --- |
| `git diff --exit-code 820100a6… HEAD -- src tests Cargo.toml Cargo.lock`, scratch `diff`/`cmp` | 0 | 제품 동일성 |
| Scratch source에서 `cargo build --locked --offline --release --features accelerate --lib` | 0 | 37.40s; 새 제품 library build |
| 첫 `rustc` preparation reader build | 1 | reviewer 명령의 `--extern zstd` 누락; 실행 전 실패, 모델 호출0 |
| `rustc --edition=2024 -O` + locked build의 replica_v3/serde/sha2/candle_core/zstd rlib로 reader build | 0 | preparation/recount/parity/wrong; 원본 source 변경0 |
| `proof before` | 0 | 원본 파일 목록·해시 snapshot |
| `preparation` | 0 | 부모 native·Adam·자료·전체 tape/token/mask·seal 연결 재검산 |
| `recount` | 0 | raw3904, teacher3904, trace3072, native9 및 판정 재검산 |
| `parity` | 0 | 기존 parent32와 B32의 실제 호출·RETURNED·출력 일치 증거 확인 |
| `wrong` | 0 | 기존 별도 승인된 오답4의 실행·raw·해시·사용량 확인 |
| `proof-final after` | 0 | 전후 불변, 기존 결과와 독립 재채점 일치 확인 |

각 reader 실행의 stdout/stderr와 build 실패·성공 로그를 scratch에 따로 보존했다. 첫 실패는 제품 결함이나 통과 시험으로 기록하지 않았다. `proof-final`은 ID 배타적 분류와 최종 결과 대조를 추가한 reader이며, before snapshot과 같은 파일 수집 논리를 사용한다.

기존 A의 `citation_fidelity_process`, `citation_fidelity_tape_and_schedule`, `citation_fidelity_inference_budget`은 실제3 passed/0 failed다. A-quick의 command/summary hash, 정확한 시험 이름과 3-test 결과, source_unchanged를 다시 확인했다. 기존 실제 process 시험에는 연속2/새 process1+1의 weights·Adam·cursor, 최종 평가-only optimizer0/중복 반환0, 잘못된 parent·seal 연결 거부와 품질 실패 재현/학습 차단이 포함된다. **이번 신규 제품 동적 시험 수는 0이며, 기존3개를 이번 실행 수로 다시 합산하지 않는다.**

**B 독립 채점과 실제 재현의 구분**

새 `recount.rs`는 제품 citation parser를 호출하지 않는 별도 Rust ID parser를 사용했다. Request의 key/context와 제공 record에서 정답을 도출하고 corpus answer와 비교했다. Raw tokens의 strict decode, 전체 문자열, EOS, 오류 및 실제 prompt digest를 확인했다. ID를 치환·기각해 점수를 올리거나 실패 행을 분모에서 제외하지 않았다. 각 원자료 행의 prepared/resolved와 RETURNED 연결도 확인했다.

| 최종 dev 패널 | FULL /512 | QB /256 | SB /256 | ALL4 /128 | 첫 값 /512 | 정확 support /512 | 근거 밖 ID | EOS /512 | 실행 오류 |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| V | 510 | 254 | 254 | 126 | 510 | 해당 없음 | 해당 없음 | 512 | 0 |
| VC | 510 | 254 | 255 | 127 | 512 | 510 | 2 | 512 | 0 |
| ID 변경 | 512 | 256 | 256 | 128 | 512 | 512 | 0 | 512 | 0 |

VC ID의 배타적 분류는 목표record510/다른 제공record0/근거 밖2/파싱 불가0이다. ID 변경 패널은512/0/0/0이다. 같은512 cases의 ID_BOTH는510/512이며 두 패널을 독립1024 표본으로 합치지 않았다. Teacher의 gold-prefix NLL은 저장된 진단으로 재합산했고 생성 성공률로 대체하지 않았다.

부모7424→10496의 FULL gain/loss는 V1/2, VC20/2, ID17/0이고 ALL4 gain/loss는0/1,15/1,12/0이다. 최종 V dev 기준은 충족하지만 full train 보존4608은 NOT_RUN이므로 전체 보존 기준 통과를 주장하지 않는다.

기존 B의 고정 첫 V16/VC16은 각각16/16로 token·문자열·EOS·종료·오류가 원 raw와 일치했다. 당시32개가 모두 정답이어서 추가 오답 재현이 별도 승인됐고, V23/V100/VC477/VC479의4회도 원 오답과 일치했다. 이번에는 이 완료된 호출의 native/정책/자료/hash/RETURNED를 검증했으며 **새 generation을 하지 않았다**. B 보고서 앞부분의 PARTIAL은 당시 기록이며, 명시적 추가4회 이후 addendum의 현재 B 판정은 PASS다. 그 이전 PARTIAL 기록은 수정하지 않았다.

**사용량·보존·미실행**

- 이번 신규 SMALL optimizer0 / TINY optimizer0 / generation0 / teacher0 / backward0 / finite-difference0.
- 기존 fidelity 누적 SMALL: optimizer3,072, generation3,972, teacher3,904, generated tokens48,774. 별도 승인된 오답4회가 포함된다. Canonical active1756.414305584s. 빌드/reader wall time과 모델 active time은 다르다.
- 기존 fidelity TINY: optimizer62/64, generation500/1024, teacher348/1024. 기존 실패 실행을 포함한 수치를 보존한다.
- 원 source/tests/Cargo, 관련 native/corpus/raw/seal/기존 보고서·실행 증거와 사용자의 `.DS_Store`를 포함한33,480개의 파일 목록과 SHA256이 전후 동일했다. 원본 증거의 추가·삭제·수정이 없다. Seal 본문은 hash 검증만 했고 열어 채점하지 않았다.
- 최종 terminal은 Finished / FINAL_QUALITY_FAIL_AT_10496 / resume=false. 추가 학습, full fit4608, confirmation512, QA640은 미실행이다. 적격 후보가 없어 후속 gate를 실행하지 않으며, 품질 실패 재현 PASS로 이를 허용하지 않는다.
- 기존 A/B와 scalar4352 confirmation을 반복 실행하지 않았다. PyTorch 비교·전면 안정화·무관한 보안 영역도 검토 범위에 추가하지 않았다.

**발견 이슈 및 필요한 시험 관점**

확정된 제품 코드 결함은 발견하지 않았다. 따라서 Severity를 붙인 추측성 이슈나 명목상 수정 권장은 없다. 관측된 모델 품질 미달은 정상적으로 FAIL·후보 없음으로 반영되어 있으며 코드 수리 PASS와 구분한다.

| 시험 관점 | 검토 증거 / 한계 |
| --- | --- |
| unit | 기존 tape/schedule·RSS 경계 시험3개 중 해당 시험 확인; 이번 실제 전수 token/mask 재계산 |
| integration | 기존 실제 새 process2 대1+1 및 평가-only 증거; 이번9개 native/Adam/trace 연결 확인 |
| regression | 기존7424 실패 및4352 수용 불변; 신규 첫/절반/최종 tape 및 cap·평가 구간 확인 |
| malformed / boundary | 기존 잘못된 parent·seal 거부, 최종 정확 teacher-cap 및 RSS 경계 증거; 이번 별도 parser와 모든 raw 정합 검사 |
| failure path | 기존 NOT_INVOKED/RETURNED, quality-fail 재현과 재학습 거부 증거 보존; 현재 후보 없음·confirmation 미생성 확인 |

위 범위는 완료됐다. 실제 실행하지 않은 추가 fault matrix나 새로운 모델 품질을 PASS로 표시하지 않으며, 기존에 수용된 동일 source의 시험을 다시 수행할 근거는 없다.

**이번 검산 식별자**

| 항목 | SHA256 |
| --- | --- |
| preparation reader binary | `7e759ffbbb7cb3d4fc713f285f25d72dd5748d53da17b7d08c9b83ee94d29df7` |
| 별도 parser 포함 recount source | `529d961c082afb7d23ce43d260ad981684197a379a0562fc54799d0e7f573e5f` |
| recount reader binary | `17aeceddaa6c1ca24fbcd9912795f08a48d45eb4e6863981719536dacae5584e` |
| parity reader binary | `2fffa198aa7443991f58504b158502da7340884c5459f22b06fdde519eac66e5` |
| wrong-case readback binary | `90fe459a54f52cb5f567d805cf8254af21bc68b7264fd5ff75f2c0f04bd9c158` |
| final proof reader binary | `e0d1cebf3c95622e96739ffa974894ca7580fbdba029cf1bf39518f7807adedc` |
| B-recount.r3b | `314897c54b9a656fd1c4b88d1e363aa79759b7042bd648475fa2d1b3696ea56c` |
| B-parity-usage.r3b | `570ab4fb56d828611e2bbecc03bb5864cde64ae642c61fc9c94e4df1c362c5ad` |
| B4-readback.r3b | `ae3a861d08e47ad264a346f8368c5157d50887e87d043d738db4aa2582d0da1b` |
| 원본 before/after manifest | `dc4bcb7a34cc7e1ddac03a2ca6530e13b62875bc4a26130c6628cd4faad85fe1` |
| closure-proof.r3b | `530020ad3002b74040ec15a2412c137ac9322d0522fc169305da9b36499d03bc` |

기존 상세 동적 증거는 [A 보고서](CITATION_FIDELITY_REVIEW_A_2026-09-22.md)와 [B 및 오답4 addendum](CITATION_FIDELITY_REVIEW_B_2026-09-22.md)에 보존되어 있다. A report commit은 `305609fe4d570c7268bddd6fd29a5a98597e50e8`, B addendum commit은 `40e9c9977042e0696b532a92a7c6c8544f87babc`이며 reviewed source와 구분한다. 이번 게시 대상은 이 보고서 한 파일뿐이다.
