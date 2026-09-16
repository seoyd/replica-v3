# eplica v3 · B0 — 독립 검토 전용 프롬프트

문서 버전: B0-REVIEW-1.0  
검토 대상 계약: `01_IMPLEMENTATION_PROMPT.md`, B0-CONTRACT-1.0  
역할: 독립 검토자. 구현자가 아니라, 실제 코드·실행·데이터로 완료 주장을 검증하는 사람이다.

---

## 0. 임무와 금지 사항

사용자는 작은 Rust v3 runtime, v2의 선별 이식, 기존 로컬 Transformer 모델 하나, SQLite 내부의 binary canonical evidence, append-only 정정 이력을 승인했다. 아래 질문에 답하라.

> 구현물이 승인된 최소 경로를 실제로 수행하는가? 기존 v1/v2와 사용자 데이터를 보존하는가? 저장·정정·검색·모델 실행·종료가 보고서의 주장과 일치하는가?

새 architecture를 제안하거나 구현을 다시 시작하지 않는다. 검토 중 발견한 결함을 몰래 고치고 PASS로 만들지 않는다. 검토 대상 source, lockfile, 원본 fixture, 기존 DB, 모델 가중치를 수정하지 않는다.

허용되는 쓰기는 격리된 임시 테스트 DB·fixture·빌드 출력·review report뿐이다. 신규 독립 테스트가 필요하면 임시 복제본 또는 외부 harness에서 실행하고, 그 diff와 실행 경로를 표시한다. 제품 checkout을 정상인 것처럼 고쳐놓은 상태를 원래 결과로 보고하지 않는다.

**검토 범위도 유한하다.** v1/v2 전체 테스트, 컴파일러 MIR 분석, 전 저장소 공급망 감사, 수학적 완전성 증명, 새 모델 벤치마크는 요구하지 않는다. 이번 B0 계약에 연결되는 실제 위험만 다룬다.

---

## 1. 먼저 읽을 자료와 기준점

순서대로 읽는다.

1. `01_IMPLEMENTATION_PROMPT.md` 전체와 계약 버전.
2. `docs/PLAN.md`, `docs/STORAGE_FORMAT.md`, `docs/REUSE.md`, `docs/RUNBOOK.md`.
3. 구현 보고서와 제출된 실제 코드 diff.
4. 현재 source, dependency lock, SQL migration, worker 코드, 실제 테스트와 raw log.

보고서는 탐색 안내이지 정답이 아니다. “PASS”, “완전”, “99%”, “M4 최적화”, “학습”이라는 문구를 검증 없이 받아들이지 않는다.

다음을 확인하고 report 시작에 적는다.

- 실제 v3 위치, 시작 HEAD, dirty/untracked 상태, code package digest.
- 검토한 implementation contract의 SHA-256.
- v2 재사용 출처와 commit. 앞서 확인한 기준은 `seoyd/replica-v2 @ 063be980ab0c8233497dcc29fd7035f9f22074c6`이다. 실제 다른 commit을 썼다면 비교 대상을 분리한다.
- 첨부 `replica-main(260508).zip`은 Zig 참고본이지 Rust v2가 아니다. 이 파일만 읽고 Rust v2를 검토했다고 보고하지 않는다.
- 실제 OS/CPU/메모리, Rust/SQLite/runtime/model 식별자.
- 검토 전후 변경된 경로. 임시 결과 외 source 변경이 있으면 이유를 보고한다.

계약·source·diff가 없으면 없는 자료를 만들어 채우지 않는다. `BLOCKED_INPUT`을 보고하고 접근 가능한 부분의 제한적 검토만 한다. 다른 기기에서 수행했다면 `TARGET_M4=NOT_RUN`을 명시한다.

---

## 2. 승인 범위를 재확인

다음이 현재 B0의 필수 범위다.

- 별도 작은 v3 Rust 실행 경로. v2 전체가 dependency가 되어서는 안 된다.
- 사용자 원문을 생각보다 먼저 durable commit.
- schema-specific binary가 canonical이며 SQLite BLOB 사용 허용.
- 원문과 원래 발화의 불변성.
- 명시적 사실 정정·restore·current/history/as_of.
- lexical/time/scope/slot 검색 + 한도 있는 관계 탐색.
- 실제 기존 로컬 모델 하나. test double과 제품 경로 분리.
- 모델 응답·evidence reference·producer 정보를 commit한 뒤 최종 표시.
- 재시작, 파생 색인 재구축, 안전한 backup/new-path restore.
- 실행 횟수·시간·출력·할당·검색 범위 한도.
- 합성 fixture와 실제 모델 smoke, 측정 환경 구분.

다음이 없다는 이유로 FAIL시키지 않는다.

- GDN/Mamba/Precision 코어, 자체 pretraining, LoRA.
- MLA 변환, FP4 kernel, latent CoT, MTP.
- GUI, 자동화·다중 에이전트, 로봇/외부 액션.
- embedding/벡터 검색, 그래프 DB 서버.
- 모든 원문을 bit-perfect 무한 보관하는 O(1) 메모리.
- 전원 차단 실험·정형 증명·MIR 전경로 감사.

반대로 mandatory 미구현을 “향후 과제”로 분류해서 PASS하지 않는다.

---

## 3. 검토 방법과 증거 수준

각 요구사항을 아래 네 단계로 분리한다.

1. **SOURCE:** 해당 경로가 코드에 존재하는가.
2. **EXECUTED:** 그 public API/CLI 경로를 실제로 호출했는가.
3. **OBSERVED:** 결과 DB/바이트/프로세스/응답에서 기대 성질이 관찰됐는가.
4. **BOUNDARY:** 반례·오류 상황에서도 같은 규칙이 적용됐는가.

상태 표기는 `PASS | FAIL | NOT_RUN | BLOCKED | OUT_OF_SCOPE`다. `NOT_RUN`은 PASS가 아니다. 한 성공 사례를 일반 성능 보장으로 확대하지 않는다.

검토 DAG는 구현의 G0–G5에 맞춘다. 선행 storage 계약이 깨졌다면 그 위의 성공률 숫자를 제품 합격 근거로 사용하지 않는다. 단, 독립적으로 검토 가능한 model adapter나 codec 결함은 함께 보고할 수 있다.

함수 이름에 `safe`, `verified`, `canonical`이 있다는 것은 증거가 아니다. public call 내부의 실제 분기와 부작용을 추적한다.

---

## 4. 소스·이식·복잡도 검토 — G0

### 확인 사항

- v3는 독립적으로 빌드 가능한가? v2 전체를 path dependency로 끌어왔는가?
- v2의 `use super::*`, BrainGraph·Constitution·BC authority 자료형이 필요 이상으로 따라오지 않았는가?
- REUSE 표의 symbol과 실제 copied/modified code가 일치하는가?
- upstream license와 attribution이 유지됐는가?
- source가 없는데 “검증된 코드 재사용”이라고 쓰지 않았는가?
- 기존 v1/v2·사용자 파일·모델 경로를 변경하지 않았는가?
- 새로운 runtime을 만들면서 이전 1,000단계 gate 시스템을 그대로 복제하지 않았는가?
- 외부 dependency를 하나도 쓰지 않겠다는 이유로 DB/압축/프로세스 관리를 다시 발명하지 않았는가?

모든 모듈을 다시 설계하라는 의견은 내지 않는다. 실제로 B0 경로를 막는 결합이나 유지보수 위험만 구체적인 파일·호출 경로로 지적한다.

---

## 5. canonical 데이터와 바이너리 검토 — G1

### 5.1 실제 저장 바이트 검사

테스트 DB에서 canonical BLOB을 직접 추출해 schema 문서와 독립적인 작은 reader로 검사한다. binary column 안에 JSON 문자열 또는 `serde_json` 결과를 그대로 넣어놓고 binary라고 부르지 않는지 확인한다.

원문 UTF-8이 읽힌다는 이유만으로 실패시키지는 않는다. 모델의 config JSON, 임시 IPC JSON, human-readable report는 canonical evidence JSON 금지와 다르다.

### 5.2 포맷 불변조건

다음을 최소 한 번씩 검증한다.

- 명시적 version/endianness와 kind tag.
- canonical integer encoding 및 overflow 거부.
- unknown codec/version/tag/flags와 truncated data 거부.
- encoded/raw length 상한, bounded decompression.
- 잘못된 checksum, 잘못된 참조, trailing bytes 처리.
- Rust in-memory struct layout·pointer·native padding을 저장하지 않음.
- 원문 normalization·trim·줄바꿈 변환이 없음.
- raw와 compressed 경로가 정확히 같은 원문으로 복원됨.
- dictionary를 쓴다면 그 definition이 canonical 자료에 포함됨.

producer의 `encode`를 다시 호출한 결과를 expected bytes로 사용하는 테스트만 있다면 독립 검증이 부족하다. literal golden 또는 별도로 만든 parser로 최소한의 cross-check를 추가한다.

### 5.3 모델 정보의 재현성

AssistantAnswer에서 실제 모델 식별자, revision/가중치 식별, tokenizer/chat template 또는 runtime 식별, generation 조건과 근거 참조가 추적되는지 확인한다. 모든 모델 파일을 매 요청 다시 hash할 필요는 없으나, 모델이 바뀌었는데 같은 producer라고 기록되면 안 된다.

기억 저장을 weight training으로, cosine 값을 기억 정확률로, compressed representation을 원문으로 오인한 보고를 적발한다.

---

## 6. 트랜잭션·원본·재시도 검토 — G2

### 6.1 실제 PRAGMA 확인

열려 있는 실제 connection에서 `journal_mode`, `synchronous`, `foreign_keys`, busy timeout을 읽는다. 설정 명령을 실행했다는 코드만으로 확인을 끝내지 않는다.

- WAL + FULL과 다른 durability 설정을 혼용한 성능표가 없는가?
- 설정 실패나 unknown pragma를 무시하지 않는가?
- macOS fullfsync 사용 여부는 실제 설정과 분리돼 보고됐는가?
- “FULL 설정”을 물리 디스크 모든 고장에 대한 보장이라고 주장하지 않는가?

### 6.2 transaction 경계 추적

append public call에서 ID 발급, canonical blob, metadata/index/heads/관계가 어떤 transaction에 묶이는지 코드로 추적한다. 여러 파일 사이의 원자성 문제가 없는지 확인한다.

모델 생성 동안 write transaction을 붙잡고 있으면 경합·timeout·원자성 경계를 확인한다. save 완료는 COMMIT 성공 이후여야 한다.

원문 테이블 변경 방지 trigger나 동등한 제어가 실제 있는지 검사한다. 개인 DB의 raw SQL 권한을 가진 관리자에게도 변조 불가능하다는 과장된 주장과 구분한다.

### 6.3 실제 crash 시나리오

격리된 DB와 child process로 다음 지점을 확인한다.


| 지점               | 허용 결과                   | 금지 결과                   |
| ---------------- | ----------------------- | ----------------------- |
| canonical 삽입 전   | 새 사건 없음                 | 성공 ack                  |
| 삽입 후 commit 전 종료 | transaction 전체 rollback | canonical만 남고 참조/머리값 누락 |
| commit 후 표시 전 종료 | 재시도에 같은 사건/결과           | 동일 요청의 중복 commit        |
| 모델 실패            | 입력 보존 + 오류              | input 원문 rollback/삭제    |
| 답변 commit 실패     | 답변 성공 미표시               | 저장 안 된 최종 답변을 성공 표시     |


crash fixture의 hook이 production 동작을 우회하지 않았는지 확인한다. 통제된 종료·SIGKILL만 했다면 전원 차단 복구 시험이라고 쓰지 않는다.

### 6.4 idempotency

같은 request_key+같은 내용, 같은 request_key+다른 내용, 같은 내용+서로 다른 request_key를 구분해 검증한다. 저장 중복 방지와 모델 실행의 exactly-once는 다른 성질이다.

commit이 성공했지만 사용자에게 반환되지 않은 경계를 시험한다. 중간 실패에서 재시도 status가 영원한 pending loop로 남지 않는지, 사용자에게 복구 가능한 상태를 명시하는지 확인한다.

---

## 7. 정정·복원·시간 의미 검토 — G3

다음 fixture를 독립적으로 작성한다.

```
O1: 사용자 원문 “오른쪽으로 가”
A1: scope S / entity E / predicate route / context C = RIGHT
A2: A1을 명시적으로 정정한 LEFT
A3: A2 뒤에 A1의 값을 restore한 RIGHT
B1: 같은 entity와 predicate지만 context D = LEFT
```

필수 검증:

- O1 bytes는 A2/A3 이후에도 그대로다.
- A1/A2/A3가 모두 조회되고 chain 순서와 source가 보인다.
- current는 A3, original/history는 A1 및 A2를 구분한다.
- B1은 C의 current를 바꾸지 않는다.
- A3는 새로운 event이지 기존 DB의 물리적 rollback이 아니다.
- 다른 lineage의 event를 restore target으로 사용할 수 없다.
- 같은 expected head를 가진 두 correction은 하나만 성공한다.
- recorded time과 observed time이 다른 late arrival에서도 as_of 범위가 맞는다.
- 시간 동률은 문서화된 ID/기록 순서로 결정되고, clock 변화가 event ID 재사용을 만들지 않는다.
- 지원하지 않는 복잡한 유효구간 요청은 조용한 임의 병합 대신 명시적 오류다.

가장 최근 timestamp 하나만 선택하는 구현으로 전체 version 의미를 대체하지 않았는지 확인한다. 모델이 말한 “정정입니다”만으로 사용자의 confirmed head가 바뀌면 안 된다.

---

## 8. 그래프·검색·증거 해석 검토 — G3

### 8.1 그래프의 역할

canonical 관계 주장과 derived adjacency가 구분되는지 확인한다. `precedes`와 `causal_hypothesis`가 구별되는가? 모델 생성 관계가 입력 근거·모델 producer 없이 확정 사실이 되는가?

data graph에는 순환이 생길 수 있다. traversal은 visited·hop·node·시간 제한으로 종료해야 한다. development DAG와 evidence graph의 성격을 혼동하지 않는다.

### 8.2 검색의 실제 경로

쿼리 → scope/time/slot filter → lexical 후보 → 관계 확장 → top-k → 원문 decode → EvidenceBundle까지 직접 따라간다.

- SQL 문자열 연결과 FTS raw 문법 주입 여부.
- 한국어 “오른쪽”, “오른쪽으로”, 1–2음절 query의 실제 처리.
- raw normalization과 검색 normalization의 분리.
- 현재 질문이 자기 자신의 evidence가 되는 self-match 여부.
- 과거 AssistantAnswer가 사용자 사실의 유일한 근거로 다시 사용되는지.
- scope/session filter가 graph 확장에도 동일하게 적용되는지.
- 범위 한도에 도달했는데 “자료 없음”이라고 단정하는지.
- `truncated` 표시와 원문 조회 기능이 있는지.

### 8.3 10k 지연 증거 fixture

비슷한 지시가 여러 번 나온 합성 corpus를 사용한다. 정답은 채점기만 가지고, 제품에는 일반 원문과 명시된 관계만 제공한다.

다음 비교를 분리한다.

1. `show known_event_id`: byte-exact 원문 회수.
2. 자연어/범위 query: 관련 증거 검색.
3. 모델에 같은 증거 전달: 답변의 사용 정확성.

(1)의 100%를 (2)·(3)의 성공률이라고 계산하지 않는다. fixture가 제공한 원인 edge를 읽었다면 인과를 새로 발견했다고 주장하지 않는다.

“우회전이 있었음을 설명”하는 정답과 “그 우회전이 사고 원인이라고 확정하지 않음”이라는 정답을 각각 검사한다. 관련 기록이 실제로 없거나 후보가 모호한 negative control을 반드시 포함한다.

### 8.4 공정한 비교

lexical-only와 graph-expanded 경로는 같은 원문·관계 자료, 같은 top-k와 budget을 사용해야 한다. 그래프 쪽에만 정답 parent ID나 특별한 metadata를 주고 우월하다고 보고하면 invalid benchmark다.

---

## 9. 모델 어댑터와 종료 경계 검토 — G4/G5

### 9.1 실제 모델 여부

product build에서 실제로 실행되는 경로를 확인한다. 다음 패턴을 집중 점검한다.

- model load 실패 시 canned answer를 돌려주는 fallback.
- 테스트 mode를 runtime default로 켜는 flag.
- 입력의 특정 단어에 맞는 답을 직접 반환하는 branch.
- evidence lookup만 수행하고 “모델이 추론했다”고 기록하는 경로.
- 테스트 보고서의 model name과 실제 load path 불일치.

모델에 보낼 context를 확인하고 해당 원문이 실제 입력으로 들어갔는지 검사한다. 정답 문자열이 system prompt나 fixture의 hidden evaluator 결과에서 들어오지 않는지 확인한다.

### 9.2 로컬과 권한

- cloud API·자동 업로드·원격 fallback이 없는가?
- worker는 명시적인 argv이며 `sh -c`로 prompt를 실행하지 않는가?
- 모델의 출력이 SQL, 파일명, subprocess 명령, confirmed correction의 권한이 되지 않는가?
- private evidence가 diagnostic에 무제한 출력되거나 repo에 commit되지 않는가?
- 모델 경로는 설정에서 오며 user evidence 문자열로 바뀌지 않는가?

prompt injection에 대한 완전 면역을 요구하지는 않는다. 그러나 실제 명령 권한을 모델에게 주지 않는 경계는 구현되어 있어야 한다.

### 9.3 실행 한도와 프로세스

timeout·출력 초과·child 종료·stderr 포화·사용자 취소에서 끝나는지 실제로 확인한다. `wait_with_output` 등으로 무한 wait 또는 무제한 buffer가 생기는지 조사한다.

질문 하나가 한 모델 호출로 끝나는가? 실패가 재귀적인 재질문이나 숨은 무제한 retry를 일으키지 않는가? reader thread·child process가 정상적으로 회수되는가?

context 제한은 실제 tokenizer 기준인가? byte 수를 token 수라고 속이지 않는가? 범위 밖 evidence를 버려도 canonical 기록은 남는가?

### 9.4 실제 한국어 smoke

실제 기존 모델로 최소 5개 질의를 실행한다. 모델 identifier·host·input evidence IDs·output·latency·종료 이유를 기록한다. deterministic mode라도 모델 출력 전체가 모든 하드웨어에서 byte-identical해야 한다고 요구하지 않는다.

인용 ID가 bundle에 존재하는지 검사하고, 그 인용이 주장과 맞는지는 사람이 읽어 별도 평가한다. ID 검사만으로 semantic truth PASS를 주지 않는다. 의미 오류가 나왔다면 실제 output을 첨부하고 심각도·수정 범위를 판정한다.

M4에서 실행하지 못했으면 `REAL_MODEL` 또는 `TARGET_M4`를 정확히 NOT_RUN으로 남긴다. mock 테스트를 해당 칸에 넣지 않는다.

---

## 10. 저장 복구·개인정보·측정 검토

### 10.1 rebuild와 backup

원본 복사본에서 파생물만 삭제하고 rebuild한다. canonical ID/바이트·정정 계보가 변하지 않는지 확인한다. rebuilt graph가 모델을 다시 실행하지 않고 저장된 관계 주장으로 복원되는가?

backup API 결과를 새 경로로 복원하고 원본과 비교한다. 열린 DB 파일만 복사해 WAL의 최신 commit을 놓치는 구현은 blocker다. 기존 개인 DB를 restore 과정에서 덮어쓰면 안 된다.

손상된 canonical blob을 임시 복사본에 넣어 명시적 오류를 확인한다. doctor가 손상 레코드를 몰래 버린 뒤 “정상”이라고 말하면 실패다.

### 10.2 파일과 데이터 경계

private DB·WAL·backup의 경로와 권한을 확인한다. 테스트가 실제 사용자 홈 DB나 모델 파일에 쓰지 않았는지 확인한다. unencrypted SQLite를 encrypted storage라고 표현하면 보고서 오류다. OS 수준 암호화의 지원을 자동으로 가정하거나 사용자 설정을 변경하지 않는다.

### 10.3 측정 분모

보고서에서 다음을 직접 대조한다.

- 원문 byte 수, 실제 record 수, DB/WAL/SHM/index/backup의 포함 범위.
- warm/cold, debug/release, 단건 commit/batch import, 모델 load 포함 여부.
- GPU/shared memory를 포함하지 못한 RSS의 정확한 표기.
- percentile 산출에 사용된 표본 수.
- 같은 durable 설정과 같은 정보량으로 비교했는지.
- 순수 연산 proxy나 사전에 넣은 상수가 실측으로 포장되지 않았는지.

압축률은 해당 corpus의 결과로만 보고한다. “세계에서 가장 작은 저장 방식”, “AI 기억 99% 입증”, “새 코어가 TR을 이김”은 이번 검토로 승인하지 않는다.

---

## 11. 재현 실행 순서

1. source/diff/contract를 먼저 읽는다.
2. 격리된 target/temp 디렉터리에서 lockfile을 바꾸지 않고 build한다.
3. 실패 위험이 큰 codec·transaction·version tests를 먼저 수행한다.
4. retrieval·worker 실패 경계와 B0 통합 테스트를 수행한다.
5. 실제 모델 smoke를 명시적으로 수행한다.
6. 실제 목표 기기라면 release 측정과 restart/backup을 확인한다.
7. 새로운 v3 crate 전체 test를 최종 한 번 수행한다.

한 번 실패한 전체 suite를 아무 분석 없이 반복하지 않는다. dependency를 바꾸거나 lockfile을 갱신해서 검토물을 다른 것으로 만들지 않는다. network/model 부족은 환경 blocker로 분리한다.

새로운 독립 테스트는 필요한 최소 개수만 만든다. 구현과 동일한 helper를 써서 expected 결과를 생성하지 않는다. test count 문자열 검사가 아니라 실제 결과·실행 경로를 확인한다. 0개 테스트 실행, ignored test, `|| true` 은폐를 체크한다.

---

## 12. 심각도와 중단 기준

### P0 — 즉시 차단

원본 손실·무단 덮어쓰기, 승인 안 된 외부 전송/명령 실행, 개인 DB 파괴, fabricated PASS/실측, commit 전 성공 표시로 인한 계약 위반 등.

### P1 — B0 합격 차단

정정 계보 오류, stale-head 손상, restart/backup 실패, 제품의 mock fallback, 무한 wait/무제한 retry, 필수 기능 누락, scope 누출, 재현 가능한 잘못된 근거 연결 등.

### P2 — 비차단 개선

명확한 기능 결함이 아닌 성능 개선 여지, 선택적 compression 최적화, 작은 문서 부족, B0 밖의 semantic/vector 기능.

환경에서 모델이 없다는 사실 자체는 source bug가 아니다. `BLOCKED_ENVIRONMENT`로 구분한다. 다만 그런 상태에서 B0_READY라고 주장한 보고는 별도 오류다.

치명적 문제가 발견되어도 코드가 아닌 최소 재현·관련 경로·영향을 제시한다. 관련 없는 스타일 수정이나 새로운 architecture를 해법으로 요구하지 않는다.

---

## 13. 최종 판정 규칙

두 축을 반드시 따로 보고한다.

### 코드·계약 판정

- `PASS`: 검토 범위의 필수 계약에서 blocker 없음, 필요한 deterministic 실행 확인.
- `FAIL`: 재현 가능한 P0/P1 또는 필수 계약 위반.
- `BLOCKED_INPUT/ENVIRONMENT`: 증거 부족으로 해당 부분 판정 불가.

### 제품 기준점 판정

- `B0_READY=YES`: 실제 단일 모델 + 실제 M4 + 저장/검색/정정/재시작/종료의 필수 경로가 검증됨.
- `B0_READY=NO`: 위 조건 중 하나라도 미검증 또는 실패.

코드 PASS와 B0_READY=NO는 동시에 가능하다. 이를 하나의 PASS로 압축하지 않는다. B0 통과는 모델 지능·기억률99%·신규성·모든 장애 안전성 보장이 아니다.

### 보고서 형식

```
MODE: INDEPENDENT_REVIEW
CONTRACT: B0-CONTRACT-1.0
IMPLEMENTATION_PROMPT_SHA256:
REVIEWED_CODE_IDENTITY:
HOST:
CODE_VERDICT:
REAL_MODEL_VERDICT:
TARGET_M4_VERDICT:
B0_READY:

A. 5줄 이내 결론
B. 실제 읽은 파일·commit·diff와 미검토 범위
C. G0–G5 / R01–R16의 SOURCE·EXECUTED·OBSERVED·BOUNDARY 표
D. 재현한 테스트와 실제 명령·로그·결과
E. 결함 목록
   - ID / severity
   - file:line 또는 실제 symbol/call path
   - 최소 재현
   - 실제 결과 / 기대 계약
   - 영향
   - 최소 수정 방향(패치 적용은 하지 않음)
F. 구현 보고서의 과장·누락·환경 오표기
G. 실제 저장량·latency·메모리의 측정 조건
H. 허용된 임시 산출물 외 검토 전후 변경 여부
I. 재검토 범위: blocker 수정 경로와 직접 회귀만 지정
```

결함이 없으면 결함을 만들지 않는다. 이미 충족된 계약을 다시 더 큰 요구사항으로 바꾸지 않는다. 선택적 개선 때문에 완료를 무기한 미루지 않는다. 반대로 실측하지 않은 부분은 끝까지 NOT_RUN으로 남긴다.

**지금 계약과 구현물을 독립적으로 읽고, 실제 실행과 바이트를 확인한 뒤 위 형식으로 판정하라. 제품 코드는 수정하지 말라.**