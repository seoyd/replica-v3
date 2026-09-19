# Independent stabilization acceptance — 2026-09-19

```text
MODE: INDEPENDENT_REVIEW (R0–R2)
CONTRACT_ID: R3-ACCEPTANCE-AND-QUALITY-CLOSURE-1.0
CODE_VERDICT: PASS
INDEPENDENT_DYNAMIC_ACCEPTANCE: PASS
STABILIZATION_SCOPE_VERIFIED_INDEPENDENT: true
PRODUCT_SOURCE_CHANGED: NO
```

고정 candidate의 기존 고유 시험15개를 독립적으로 실행했고 모두 통과했다.
P6144의 기존 고정16문항도 production 기능 빌드에서 재생성하여 일치했다.
이번 범위에서 확정한 High/Medium/Low 결함은 없다. 수정 권고와 제품 패치는 없다.
이는 생성·중단·저장·평가 재개 경계의 수용이며, 전체 저장소의 무결점 보증,
모델 품질 통과 또는 제품 release/Goal1 승인이 아니다.

## Source, environment and isolation

| 항목 | 실제 확인 |
| --- | --- |
| REVIEWED_CODE_SHA / CODE_SHA / SOURCE_COMMIT | `abd967980645a878f22069708daa9fbfce70bf87` |
| 검토 시작 HEAD / 구현 보고서 | `585e2c881d3c3c9627967d1de989d265a8657025` |
| 후속 diff | candidate 이후 기존 보고서·계획 문서2개만 변경; source/tests/Cargo 동일 |
| SOURCE_AUDIT | PASS_WITHIN_SCOPE; timeout 분류와 직접 재개·분모·보존 경로 |
| TEST_ENV / TARGET_ENV | Apple M4, arm64, Rust1.98.1, Cargo1.98.1; CPU/Accelerate, compute thread1 |
| TINY FEATURES | `accelerate,test-support` |
| P6144 FEATURES | `accelerate`; test-support 없음 |
| 의존성 | 기존 Cargo.lock, 모든 Cargo 명령 `--locked --offline`; 설치·업그레이드 없음 |
| WORKTREE_BEFORE / AFTER | 기존 tracked 파일 내용 동일; 기존 untracked `.DS_Store` 보존 |
| 출력 경계 | 별도 source 복사본·Cargo home/target·TMPDIR·evidence; 원본 운영 DB와 final200 접근 없음 |

로컬 Git archive로 정확한 candidate를 복사했다. source/tests와 Cargo 파일은
원본 candidate와 바이트 대조했고 시험 이후에도 동일했다. 전용 Cargo home에는
기존 registry cache/src/index만 복사했다. 프로젝트 자체 build script는 없고,
사용된 SQLite/zstd build script의 생성 대상은 Cargo OUT_DIR이다.
환경변수 설정을 OS sandbox라고 주장하지 않는다.

증거 root는 `artifacts/independent-acceptance-20260919-Es0t0R`이다.
source, target, tmp, tools, evidence, parity-P6144를 이 root 아래에 두었다.
P6144는 scratch의 동일 상대 경로로 복사하고 원본과 `diff -qr`가 일치함을
확인하여, 시험 cwd와 과거 raw 안의 상대 경로가 원본 쓰기로 연결되지 않게 했다.

별도 로컬 Rust capture 도구는 이후 TINY child63회의 stdout/stderr/exit 및 binary
metadata/row snapshot을 보존했다. 실제 native 실행물의 바이트는 변경하지 않고
별도 이름으로 실행했으며 출력·반환코드를 그대로 전달했다. 제품·테스트 source는
수정하지 않았다. 초기 시험은 capture 설치 전에 실행됐으므로 전체 child snapshot이
보존됐다고 주장하지 않는다. 아래 사용량은 실제 통과한 counter/state assertion과
보존된 snapshot의 중복 제거 집계를 함께 사용했다. 초기 시험을 기록 확보 목적으로
다시 실행하지 않았다. 이 도구·임시 자료는 게시하지 않는다.

## Actual executable identities

| 실행물 | SHA-256 |
| --- | --- |
| TINY native child | `213e762b7de262336454a689e54f00454f4aefbfd76073497c7c71c728f8a275` |
| integration test binary | `b20aa0caf6f3a78ac640986a666c3e536981ad01e2e1f8e48ea053431938fcc8` |
| unit test binary | `09a5406da16c09f2f4642655c81265a71eea951d3e6ca3010223162dde906966` |
| production parity test binary | `8ce421ad8ada70b14447453d5b9f8006e242366efb1e6602f374c558e8cb45d0` |
| compiled source digest | `cb89a1598abcea8baeab82e547d14feedfc2701295fcf996d1da9d781212ffe9` |

새 빌드의 실행물 hash를 사용했다. 과거 학습 source
`fbffc0edf943818e008a37bf9c5b5cf921fdc36a`의 binary나 구현자의 test binary를
이번 독립 실행으로 계산하지 않았다.

## Executed tests and boundaries

EXECUTED_TEST_COMMANDS=15, UNIQUE_TESTS=15, PASS=15, FAIL=0.
목록 확인·no-run 빌드·hash 명령은 테스트 통과 수에 넣지 않았다.
SCENARIOS=14 requirements groups: 기존 T01–T12 경계12개와 연구 사용량 연결,
P16 parity. 이 수는 개별 assertion이나 생성 문항 수가 아니며 테스트 수와 합산하지 않는다.

실제 빌드/목록 명령은 아래와 같다. 공통으로 scratch Cargo home/target/TMPDIR와
`VECLIB_MAXIMUM_THREADS=1 RAYON_NUM_THREADS=1`을 사용했다.

```sh
cargo test --locked --offline --features accelerate,test-support --test training -- --list
cargo test --locked --offline --features accelerate,test-support --bin replica-train -- --list
cargo test --locked --offline --release --features accelerate --bin replica-train --no-run
```

목록에서 정확한 이름을 확인한 뒤 새 test binary를 각각
`<binary> <full-test-name> --exact --nocapture --test-threads=1`로 실행했다.
FX05에만 `R3_FRESH_TEST_COMPACT=1`을 사용했다. P parity는 release binary에
`--ignored`와 명시적인 parent/output 경로를 추가했다.
전체 orchestration은 로컬 `run-direct.sh`, 실제 exit는
`evidence/executed-tests.log`, assertion 결과와 OS 시간/RSS는
`evidence/test-*.log`에 보존했다.

| target | 실제 시험 이름 | exit |
| --- | --- | ---: |
| bin | training::recovery::tests::fresh_fx06_native_command_timeout_is_returned_failure | 0 |
| bin | training::recovery::tests::fresh_fx06_request_timeout_and_mixed_causes_stay_blocked | 0 |
| bin | training::fresh::tests::fresh_strict_rows_roundtrip_command_stop_and_errors | 0 |
| bin | training::recovery::tests::fresh_fx02_returned_generation_survives_command_deadline | 0 |
| bin | training::recovery::tests::fresh_fx05_actual_returned_zero_length_and_utf8_are_not_no_call | 0 |
| bin | training::recovery::tests::repair_rf03_actual_token_cancel_preserves_partial_and_skips_followup | 0 |
| bin | training::recovery::tests::repair_rf03_deadline_precision_priority_and_latched_cleanup | 0 |
| training | fresh_fx06_native_timeout_process_resume_preserves_failed_rows | 0 |
| training | fresh_fx06_timeout_study_usage_and_mixed_failure_process | 0 |
| training | fresh_fx04_checkpoint_timeouts_keep_final_evaluation_pending | 0 |
| training | fresh_fx05_not_invoked_and_unknown_process_boundaries | 0 |
| training | fresh_eos_deadline_process_and_sync_failure_stay_distinct | 0 |
| training | fresh_fx01_post_publication_failure_blocks_new_process | 0 |
| training | fresh_balanced_two_updates_match_fresh_process_resume_and_reject_unbound | 0 |
| release bin | training::fresh::tests::stabilization_fixed_parent_parity | 0 |

시험 이름 예시에 있던 다른 FX03/fork 함수를 별도 중복 실행하지 않았다.
원보고의 고유15개를 실제 목록과 대조했고, 평가만 재개·마지막 행·연속/분할 의무는
위의 실행된 시험에서 직접 확인했다. 새로운 테스트를 만들어 통과 수를 늘리지 않았다.

| checklist | EXECUTED_THIS_REVIEW |
| --- | --- |
| unit | command/request cap, teacher 설정별 동일 timeout 의미, deadline/RSS/취소, strict raw 채점 |
| integration | 실제 native zero/partial/마지막-row timeout3가지; 새 process에서 실패 문항 재생성0, 남은 의무 수행 |
| regression | checkpoint 전/후·optimizer 반환 경계3가지; 평가 재개 optimizer0; weights/Adam/clock/sampler 일치 |
| malformed/boundary input | zero/partial token, length, strict UTF-8; raw 절단/추가 byte/누락, teacher 누락, model/policy 불일치 거부 |
| failure-path | publication 실패4가지, resolution sync 실패, 혼합 취소/비유한 값/I/O; 미반환 native 진입 후 UNKNOWN 차단 |

FX05 compact는 미호출3가지와 부정 경계5가지를 실행했다. 의도한 child 실패·exit86은
부모 시험이 예상 상태/재시도 차단을 검증한 성공이며, 정상 모델 응답으로 바꾸지 않았다.
실패 raw는 고정 분모에 남고, lifecycle 완료와 candidate eligibility가 분리됐다.

## New execution usage

아래는 이번 독립 실행의 사용량이다. 구현자의 과거 55/507/482를 복사하지 않았다.

| 실행 묶음 | TINY optimizer | generation 진입 | teacher 진입 |
| --- | ---: | ---: | ---: |
| unit7개 | 0 | 13 | 0 |
| native timeout process3경계 | 6 | 72 | 72 |
| study usage / mixed resolution | 6 | 65 | 64 |
| FX04 | 6 | 72 | 72 |
| FX05 compact | 16 | 74 | 73 |
| EOS deadline / sync | 4 | 25 | 24 |
| FX01 publication | 4 | 0 | 0 |
| balanced continuous/split + reused fixture | 4 | 49 | 49 |
| 합계 | 46 | 370 | 354 |

TINY generation 반환 확인369, teacher 반환 확인353이다. 강제 종료한 각1회는
실제 진입을 exit86과 실행 경로로 확인했지만 반환·최종 token/시간은 UNKNOWN이다.
이를0으로 치환하지 않았다. native returned timeout은12건이며 request timeout도
실패로 남았다. 평가만 재개하는 실행의 추가 optimizer는0이다.
TINY 전체 생성 token 합계는 일부 초기/단위 raw가 별도 보존되지 않아 UNKNOWN이다.
확정된 진입·반환 수와 이를 혼동하지 않는다. scalar optimizer는0이다.
허용 상한 TINY optimizer128, generation/teacher 각768 안에서 종료했다.

NEW_SMALL_UPDATES/BACKWARD/PARAMETER_UPDATES=0/0/0.
NEW_SMALL_GENERATIONS/TEACHERS=16/0. 유일한 SMALL 호출은 아래 P6144 parity다.
새 C/S 관측·학습·teacher는 실행하지 않았다.

## P6144 production parity

EXECUTED_THIS_REVIEW: 기존 고정16 ID, cases, policy, step, tokenizer, source와 호출
상한이 구현자의 selection과 같음을 binary 자료에서 확인했다. metadata 순서로
고정된 입력을 쓰고 결과를 보고 문항을 다시 선택하지 않았다.

실제 generation16, raw token160, EOS16, 오류0. 원 primary raw와 token/actual/error/
finish/EOS index/native prompt digest가16/16 일치했다. 독립 Rust reader로 구현자의
기존 parity raw와도 같은 필드를 대조했고16/16 일치했다. 정상 greedy와 strict UTF-8,
원 native tokenizer/weights를 사용했다. SMALL fixture hook/정답 대체는 없다.

RunControl 관측 구간4.509273375초, test14.55초, OS command14.59초.
`time -l`의 command maximum RSS는2,439,725,056 bytes다. 로드·검산을 포함한 시험의
관측값이며 순수 inference peak나 성능 개선 benchmark로 해석하지 않는다.

증거: `evidence/test-parity-P6144.log`,
`evidence/parity-independent-recount.log`, `parity-P6144/selection.r3b`,
`parity-P6144/parity.r3rows`, `parity-P6144/result.r3b`.
새 raw hash는 `fe435ea0434d859efda36e2998bc0a41a9d2fae4f9d60fe7a45a10f25a34c405`다.

## Preservation, limitations and quality

READ_ONLY_INPUTS_UNCHANGED=true. 원본 P6144, C/S 연구 root와 기존 P16 parity의
12,196개 파일,5,172,631,618 bytes를 경로/유형/길이/hash로 전후 대조했다.
inventory binary가 byte-identical이며 SHA-256은
`705b5527c1eb67d527174d78db628fb1c2c327a93dfd4a4ff2826f798a05492a`다.
기존 tracked 파일 hash 검사와 source/tests 복사본 비교도 통과했다.
체크포인트·corpus·raw·정책·실패 기록·종료 상태를 수정하지 않았다.

기존 전체 fmt와 strict clippy 부채는 이번에 수정하거나 재검사하지 않았다.
IMPLEMENTER_REPORTED: 전체 fmt 차이와 `src/quality_recovery.rs:4167`의
`clippy::for_kv_map` 때문에 strict 명령이 실패했던 로그가 있다.
이번 코드 경계의 build/test PASS를 전체 fmt/clippy PASS로 표현하지 않는다.

R0–R2의 필수 동적 경계와 P16에 NOT_RUN/BLOCKED 항목은 없다.
다음 품질 관측은 구현자의 별도 Q 단계이며 이번 수용 실행에 포함하지 않았다.

| 후속/품질 항목 | 상태 |
| --- | --- |
| C7168_IDENTITY / S7168_IDENTITY / PANEL_IDENTITY | R2에서 신규 Q 등록·관측 identity 검증 NOT_RUN; 기존 자료 보존만 확인 |
| EQUAL_STEP_POSTHOC_OBSERVATION | NOT_RUN_Q_STAGE |
| ORIGINAL_AND_FLIPPED_COUNTS / BOTH_CORRECT / BASE4 | 신규 관측 없음; 기존 S selector NOT_RUN을0점으로 바꾸지 않음 |
| MODEL_QUALITY_IMPROVED | NOT_CLAIMED; 신규 학습0 |
| PRIMARY_GATE / TRANSFER_GATE | 기존 공동 gate 미달; 이번 재평가로 바꾸지 않음 |
| 역사적 연구 판정 | STUDY_INCONCLUSIVE_UNEQUAL_BUDGET, candidate=null 유지 |
| FINAL200 | NOT_OPENED |
| S4 / S5 / S6 | NOT_PASSED / NOT_RUN_PREREQUISITE / NOT_RUN_PREREQUISITE |
| GOAL1_READY / GOAL1_ACCEPTED | false / false |
| NEXT_SINGLE_HYPOTHESIS | NOT_PROPOSED_R2; Q 관측 후 근거를 좁혀 별도 제안 |

## Publication

게시 범위는 이 검토 보고서 하나다. 제품 source·테스트·Cargo·모델·원자료·로컬 도구와
전체 실행 로그는 게시하지 않는다. 시작 시 branch main과 실제 origin/main은 모두
`585e2c881d3c3c9627967d1de989d265a8657025`였다.
REVIEWED_CODE_SHA는 위의 고정 source이며 report commit과 다르다.
REPORT_COMMIT/REMOTE_SHA의 게시 후 실제 일치 결과는 전달 응답과 로컬 publication log로
기록한다. 이 파일을 포함한 commit은
`git log -1 --format=%H -- docs/INDEPENDENT_ACCEPTANCE_2026-09-19.md`로 식별할 수 있다.
