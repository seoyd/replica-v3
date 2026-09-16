# Replica v3 읽기 전용 코드 리뷰 — 2026-09-16

```text
MODE: INDEPENDENT_REVIEW / STATIC_READ_ONLY
CONTRACT: B0-CONTRACT-1.0 (the implementation instruction에 명시된 버전)
IMPLEMENTATION_PROMPT_SHA256: 03e51e9e6c8826d4d1d7a1d6befa2afc3a9a63cac6aa6ec95c6ea36fba406ae8
REVIEW_PROMPT_SHA256: 7f678d1537493ec0ec4b4a13f9a5c4e7b56b923a7e033af1c7d37c720feb4bd2
REVIEWED_CODE_IDENTITY: 시작 HEAD 없음; 기존 48개 파일의 경로/내용 해시 manifest로 식별
REVIEW_MANIFEST_SHA256: b5065f2faaca4af54702519b496b079a411f34c6468d8901340797177b37b9ea
HOST: macOS 27.0 (26A428), Darwin arm64, Mac16,10, Apple M4, 24 GiB
CODE_VERDICT: FAIL — 아래 5건의 소스상 결함; 실행 재현은 NOT_RUN
REAL_MODEL_VERDICT: NOT_RUN
TARGET_M4_VERDICT: NOT_RUN (호스트 M4 확인, 이번 리뷰에서 제품 실행 없음)
B0_READY: NO
INPUT_LIMITATION: 지정 파일 the originally requested implementation input 및 시작 Git 이력/diff 없음
```

**결론**

- 발견 이슈는 High 2건, Medium 3건이다. 제품 코드와 테스트는 수정하지 않았다.
- tokenizer의 자동 truncation이 제공 근거 기록을 부정확하게 만들고, 손상된 결과 projection이 다른 사건을 정상 응답으로 반환할 수 있다.
- 검색 누락 표시 및 동시 저장 중 startup/backup 판정에도 결함이 있다.
- 기존 로그의 12개 테스트 통과는 확인했으나 이번 리뷰에서 다시 실행하지 않았다. 실모델 성공이나 현재 코드 전체 PASS로 해석하지 않는다.
- GitHub 저장은 리뷰 도중 사용자가 별도로 승인한 프로젝트/보고서 보관 작업이며, 리뷰 대상 구현을 수정하는 작업이 아니다.

**범위와 증거 수준**

먼저 현재 `the review instruction` 전체를 읽었다. 문서가 지목한 `the originally requested implementation input`는 없으므로 그 이름의 파일을 검토했다고 주장하지 않는다. 실제 존재하는 `the implementation instruction` 전체가 B0-CONTRACT-1.0을 명시하여 요구사항 대조에 사용했다. 위 implementation hash는 `the implementation instruction`의 값이다. 지정 파일 부재와 변경 전후 diff 부재는 `BLOCKED_INPUT`이다.

검토한 production 경로는 `main → app::ask → Store / search → LocalModel / worker → Store::append → display`와 직접 요구된 codec, fact lifecycle, doctor, reindex, backup/restore이다. 소스 8개, SQL migration, Cargo 설정/lock의 관련 dependency 항목, `tests/{codec,store,retrieval,runtime,cli}.rs`, `tests/support/worker.rs`, `examples/validate.rs`, PLAN/STORAGE_FORMAT/REUSE/RUNBOOK/IMPLEMENTATION_REPORT 및 관련 raw log를 읽었다.

dependency 동작이 판정에 직접 필요한 부분만 설치된 로컬 소스로 확인했다: tokenizers 0.22.2의 tokenizer 직렬화/encode/post_process, rusqlite 0.37.0의 transaction/backup, 연결된 SQLite의 backup 구현, Candle 0.11.0의 GGUF 정수 변환/모델 연결 부분. 외부 문서 조회나 dependency 다운로드는 하지 않았다.

시작 시 `/Users/seo/Projects/Replica-v3`는 Git 저장소가 아니므로 HEAD와 tracked/dirty/untracked 구분이 없었다. 구현 보고서의 v2 재사용 출처는 `6da531adebde3c4d001a313b2cdd180ae783353f`이며, 계약 기준 `063be980ab0c8233497dcc29fd7035f9f22074c6`와 다르다. 이번 리뷰에서 v2 원격/로컬 이력이나 Zig ZIP을 재검증하지 않았다. 현재 변경분과 과거 구현 사이의 회귀 여부는 확정할 수 없고, 현 소스에서 재현 조건이 성립하는 결함과 직접 연결된 테스트 공백만 보고한다.

모든 발견 사항은 **SOURCE에서 확인한 분기와 데이터 흐름**이다. 아래 재현 조건은 후속 테스트를 위한 조건이지 실행한 결과가 아니다. 실제 모델, CLI, DB, fault injection, 공격 코드, 외부 target 테스트를 실행하지 않았다. 사용자 홈 DB, 모델 가중치, credential 파일에도 접근하지 않았다.

| 단계 / 요구사항 | SOURCE | EXECUTED | OBSERVED | BOUNDARY |
|---|---|---|---|---|
| G0 / R01·R02·R14: 독립 crate, 출처 | PASS(독립 crate); BLOCKED(원출처 commit/diff 대조) | NOT_RUN | NOT_RUN | NOT_RUN |
| G1 / R03·R04·R05: 원문/binary codec | PASS(검토 범위에서 소스 결함 미발견) | NOT_RUN | NOT_RUN | NOT_RUN |
| G2 / R07·R08: 저장·복구·파생물 검증 | FAIL(RV-01·04·05) | NOT_RUN | NOT_RUN | NOT_RUN |
| G3 / R06·R11: 정정/history와 bounded 검색 | FAIL(검색 RV-03); 정정 경로에서 별도 결함 미발견 | NOT_RUN | NOT_RUN | NOT_RUN |
| G4 / R09·R10: 단일 로컬 모델/한도 | FAIL(RV-02); mock fallback/출력 실행 경로는 발견하지 않음 | NOT_RUN | NOT_RUN | NOT_RUN |
| G5 / R03·R15: 입력→답변 commit→표시 | FAIL(RV-01·02의 영향); 실제 모델 미검증 | NOT_RUN | NOT_RUN | NOT_RUN |
| R12: 수치/검증 보고 구분 | PASS(기존 보고서는 PARTIAL·B0_READY=NO 및 실제 모델 미실행 명시) | NOT_RUN | NOT_RUN | NOT_RUN |
| R13: 원본 보존 | PASS(이번 리뷰 기존 48개 파일 해시 불변) | NOT_RUN | NOT_RUN | NOT_RUN |
| R16: B0 외 확장 금지 | PASS(검토 범위 유지) | OUT_OF_SCOPE | OUT_OF_SCOPE | OUT_OF_SCOPE |

표의 SOURCE PASS는 해당 범위의 소스 대조 결과이며, 기능의 실행 합격을 뜻하지 않는다. 호스트/파일 해시 같은 읽기 전용 관찰은 수행했지만 제품 동작의 EXECUTED/OBSERVED/BOUNDARY를 대신하지 않는다.

**발견 이슈 RV-01 — 결과 projection과 canonical 질문 연결을 확인하지 않아 잘못된 사건을 정상 반환**

- Severity: **High**
- 파일 / 함수 / 라인: `src/store.rs:184–193` `Store::result`, `src/store.rs:523–533` `append_in`, `src/app.rs:26–31` `ask`, `src/app.rs:114–123` `terminal`.
- 코드 근거: `Store::result(input)`는 `results.input_id`로 찾은 `event_id`를 `self.get(id)`로 읽어 그대로 반환한다. `get`의 검증은 해당 사건의 BLOB과 자체 `record_meta` 일치 여부뿐이다. 반환 사건의 `Kind::input()`이 요청한 input인지 확인하지 않는다. `terminal`은 Failure 외 모든 kind를 `Ok(event)`로 취급한다. migration의 results FK 역시 두 ID의 존재만 보장한다.
- 왜 문제인지: disposable projection이 잘못 연결되었을 때 canonical 연결과 대조하여 오류를 내야 하는데, 정상인 다른 사건의 BLOB이라는 이유만으로 성공 처리한다. `ask`는 이 결과를 모델 호출/응답 검증 이전에 반환한다. 이는 저장된 데이터의 논리적 손상 검출 문제이며, DB 수정 권한을 가진 상대에 대한 보안 보장을 요구하는 지적이 아니다.
- 실제 영향 범위: 결과 매핑이 손상된 질문의 재시도. 다른 질문의 답변, 다른 scope의 사건, 심지어 일반 Observation이 성공한 ask 결과로 표시될 수 있다. 정상 projection을 가진 DB에서 자연히 이 손상이 생성된다는 증거는 발견하지 않았다. 명시적 `doctor(true)`의 전체 비교는 이를 발견할 수 있으나 일반 startup/재시도는 이를 실행하지 않는다.
- 재현 가능한 조건: canonical 질문 Q와 다른 정상 사건 X가 존재하고, disposable `results`의 Q 매핑이 X를 가리키는 논리적 손상 fixture. X의 canonical/metadata는 정상이고 FK·UNIQUE 조건도 만족하며 watermark는 변하지 않은 상태에서 Q의 동일 request key를 재시도한다. 소스상 X가 반환되며, X가 Failure가 아니면 성공이 된다. 실행 재현은 NOT_RUN.
- 수정 권장 / 수정 대상: `Store::result`와 `append_in`의 기존 결과 조회를 공통 검증 함수로 모아, kind가 AssistantAnswer 또는 Failure인지, canonical input이 요청 input과 같은지, canonical 질문과 scope가 일치하는지 확인한다. 불일치는 `Corrupt`로 반환하고 명시적 reindex로 복구하게 한다. `terminal`도 성공 kind를 AssistantAnswer로 제한한다. 새 결과를 조용히 덧붙여 손상을 가리지 않는다.
- 기존 테스트 공백: `tests/runtime.rs:109–120`은 정상 매핑의 재시도만 확인하고, `tests/store.rs:134–145`는 record_meta source 및 body 손상만 다룬다. 잘못된 results 연결과 terminal의 잘못된 kind는 검증하지 않는다.

**발견 이슈 RV-02 — tokenizer의 내장 truncation이 입력/근거를 조용히 잘라 제공 근거 기록을 부정확하게 만듦**

- Severity: **High**
- 파일 / 함수 / 라인: `src/model.rs:307–308, 338–365, 404–423` `worker`.
- 코드 근거: `Tokenizer::from_bytes`로 파일 설정을 그대로 로드하고 `encode(prompt, false).get_ids()`의 길이만 검사한다. `tokenizers 0.22.2/src/tokenizer/serialization.rs:122–126`은 저장된 truncation/padding을 복원한다. 같은 dependency의 `tokenizer/mod.rs:1212–1228`은 `add_special_tokens=false`여도 truncation을 수행한다. worker는 자동 잘림 여부/overflow를 확인하지 않고 남은 `evidence` 배열 전체를 `provided`로 기록한다.
- 왜 문제인지: 실제 전체 prompt 길이를 평가한 뒤 낮은 순위 evidence를 명시적으로 제외해야 한다는 계약이 지켜지지 않는다. tokenizer가 먼저 자르면 길이 검사가 통과하여 입력이 과대해도 생성하고, 실제 token 입력에 없는 근거를 제공했다고 기록한다. 원문 DB bytes는 남지만 AssistantAnswer의 provenance와 context 검증 결과가 틀린다.
- 실제 영향 범위: truncation이 활성화된 유효한 로컬 tokenizer.json을 사용하는 모델 실행. truncation이 null인 파일에는 이 조건이 적용되지 않는다. 실모델 파일이 이번 리뷰에 제공되지 않았으므로 특정 설치 모델에서 발생했다고 주장하지 않는다.
- 재현 가능한 조건: tokenizer의 truncation max_length=1024, context=8192, max_tokens=512. 렌더된 prompt가 1024 token을 넘고 오른쪽 잘림으로 evidence 또는 질문 끝부분이 사라지는 입력을 사용한다. 반환된 IDs 길이는 1024여서 evidence 제거 분기가 실행되지 않고, 실제 잘린 evidence도 provided에 남는다. 전체 prompt가 context를 넘는 경우에도 같은 이유로 사전 오류가 생기지 않을 수 있다. 실행 재현은 NOT_RUN.
- 수정 권장 / 수정 대상: `worker`에서 로드한 tokenizer의 **인메모리** 자동 truncation을 끄거나, 활성 설정을 명시적으로 거부한다. 원본 tokenizer 파일은 수정하지 않는다. 단일 입력에 불필요한 padding 설정도 검증한다. 전체 tokenization 결과로 context를 판정하고, evidence 단위의 명시적 제거만 허용하여 provided/excluded를 실제 모델 입력과 일치시킨다.
- 기존 테스트 공백: `tests/runtime.rs`의 FakeModel 및 별도 Rust worker는 실제 tokenizer/chat template 경로를 실행하지 않는다. 실제 smoke는 기존 보고서에서도 NOT_RUN이다.

**발견 이슈 RV-03 — 그래프 조회 한도에 도달해도 truncated=false로 반환하는 경로**

- Severity: **Medium**
- 파일 / 함수 / 라인: `src/retrieval.rs:211–250` `Store::search`.
- 코드 근거: 이웃 SQL은 session/time origin 필터 적용 전에 `LIMIT 257`을 적용한다. 이후 필터에서 제외하거나 이미 방문한 노드를 건너뛰면 queue/visited가 증가하지 않는다. 257개를 읽은 사실 자체로 `bundle.truncated`를 설정하지 않는다. 또한 `path.len() >= HOPS` 분기는 곧바로 continue하여 hop 한도로 생긴 누락을 알리지 않는다.
- 왜 문제인지: `docs/STORAGE_FORMAT.md:81–83`과 the review instruction §8.2는 검색 범위 한도 도달을 표시하도록 요구한다. 현재 결과는 실제로 검사하지 않은 관계가 남아 있어도 완전하게 검색한 것처럼 `truncated=false`가 된다.
- 실제 영향 범위: 관계가 많은 노드에 session/time 필터를 건 검색, 또는 4 hop을 넘어 이어지는 관계 검색. 원본 데이터는 유지되지만 CLI 및 모델 응답에 저장하는 retrieval_truncated가 누락되어 증거 부재와 미탐색을 구분할 수 없다.
- 재현 가능한 조건: scope S/session A의 유일한 lexical seed에 대해, 먼저 생성한 관계 257개의 origin session은 B이고, 258번째 관계의 origin session은 A이며 A의 다른 증거 T를 연결한다. query는 seed에만 일치한다. A로 검색하면 앞 257개는 모두 필터에서 제외되고 T의 관계는 SQL limit 때문에 읽히지 않는다. 시간 제한 미도달 시 seed만 반환되고 truncated=false다. 더 작은 조건은 seed 하나에서 5개 edge로 연결된 chain으로, 5 hop의 사건이 제외되어도 해당 플래그가 설정되지 않는다. 실행 재현은 NOT_RUN.
- 수정 권장 / 수정 대상: `Store::search`에서 scope/session/time 등 필요한 origin 필터를 가능한 한 SQL limit 전에 적용하고, 별도의 이웃 조회 budget과 초과 감지를 둔다. 필터·중복 제거 여부와 관계없이 조회를 예산 때문에 중단하면 truncated=true를 보존한다. hop 경계도 미탐색 확장이 존재하면 표시하거나 계약에 맞게 보수적으로 표시한다. 한도를 늘리거나 제거할 필요는 없다.
- 기존 테스트 공백: `tests/retrieval.rs:95–134`는 이미 300개의 lexical 일치 결과가 있어 candidate cap만으로 truncated가 true가 된다. 따라서 이 테스트로 이웃 fetch 및 hop 한도의 플래그 동작을 확인할 수 없다.

**발견 이슈 RV-04 — 정상 동시 commit을 startup에서 projection 손상으로 오판**

- Severity: **Medium**
- 파일 / 함수 / 라인: `src/store.rs:115–129` `Store::startup`.
- 코드 근거: projection_state.last_id와 records.max(id)를 각각 별도 SELECT로 읽으며, 두 조회를 동일 read transaction으로 묶지 않는다. 두 SELECT 사이에 다른 연결이 정상 append를 commit할 수 있다.
- 왜 문제인지: atomic append가 지키는 일관성을 검사자가 서로 다른 시점에서 읽어 깨진 것으로 판단한다. 정상 DB에 `Corrupt("projection version/watermark; run reindex")`를 반환한다.
- 실제 영향 범위: 다른 CLI/process가 append 중일 때 Store를 여는 모든 명령과 startup을 호출하는 doctor. 저장 데이터 손실은 확인되지 않았으나 정상 명령이 실패하고 불필요한 재구축을 안내한다.
- 재현 가능한 조건: 연결 A의 startup이 last_id=N을 읽은 직후 연결 B가 N+1과 모든 projections를 원자적으로 commit하고, A가 max(id)=N+1을 읽는 interleaving. DB는 매 commit 경계에서 정상인데 A는 mismatch를 반환한다. 실행 재현은 NOT_RUN.
- 수정 권장 / 수정 대상: `startup`의 비교값을 하나의 SELECT/서브쿼리에서 읽거나, 검사 전체를 일관된 read transaction에 둔다. 불일치를 무조건 무시하거나 retry 후 성공 처리하여 실제 손상을 숨기지 않는다. public `head`의 두 조회도 같은 유형의 읽기 경계가 있으므로 공통 조회 구조를 변경할 때 직접 회귀를 확인한다.
- 기존 테스트 공백: `tests/store.rs:92–94`의 concurrent correction 테스트는 양쪽 Store::open을 마친 뒤 barrier에서 append를 시작한다. startup과 commit이 겹치는 조건은 검증하지 않는다.

**발견 이슈 RV-05 — backup의 deferred transaction이 비교 대상 snapshot을 고정하지 않음**

- Severity: **Medium**
- 파일 / 함수 / 라인: `src/store.rs:366–396` `Store::backup`, `Store::restore`.
- 코드 근거: `unchecked_transaction()` 직후 source SELECT 없이 Backup::new/step을 실행한다. rusqlite 0.37.0의 기본 behavior는 Deferred이며 BEGIN DEFERRED만으로 source read snapshot이 시작되지 않는다. 연결된 SQLite 소스의 `sqlite3_backup_step`은 자체 시작한 source read transaction을 각 step 끝에서 닫는다(`sqlite3.c:83411–83417, 83608–83616`). backup 종료 뒤 `restored.doctor(true)`를 거쳐서야 source `table_rows`를 처음 읽는다.
- 왜 문제인지: 주석의 “stable read snapshot for backup and exact comparison”이 실제로 성립하지 않는다. 백업 완료 후 새 원본 commit이 들어오면 서로 다른 시점의 정상 DB를 비교하여 canonical mismatch라고 판정한다. SQLite backup API 자체가 불일치 snapshot을 만든다는 지적은 아니다.
- 실제 영향 범위: 다른 writer가 동시에 사용하는 DB의 online backup, 그리고 source에 동시 쓰기가 있는 restore. 정상 백업이 오류로 반환되고 이미 생성된 destination이 남아 동일 경로 재시도도 create_new에서 실패한다. 원본 손실이나 기존 destination 덮어쓰기는 확인하지 않았다.
- 재현 가능한 조건: 백업 최종 step이 N개 사건 snapshot으로 Done을 반환한 후, 다른 연결이 N+1을 commit하고, 이어서 source `table_rows`가 실행되는 interleaving. destination doctor는 정상이어도 N+1 대 N 비교로 `backup canonical mismatch`가 발생한다. 실행 재현은 NOT_RUN.
- 수정 권장 / 수정 대상: `backup`에서 BEGIN DEFERRED 후 source canonical 테이블을 실제로 읽어 snapshot을 확립하고, 해당 read transaction을 Backup::new 이전부터 종료 후 비교까지 유지한다. 비교는 그 snapshot과 destination 사이에서 수행한다. 실패 시 생성된 파일의 처리는 새로 만든 파일과 기존 사용자 파일을 구분하여 명시적으로 설계하되 기존 파일을 덮어쓰지 않는다.
- 기존 테스트 공백: `tests/store.rs:109–121`은 열린 WAL에서 백업하지만 백업/검증 도중 다른 writer의 commit이 없다. “WAL이 열려 있음”만으로 위 동시성 경계는 검증되지 않는다.

**테스트 체크리스트 — 구현 변경 후 수행할 항목, 이번 리뷰에서는 모두 NOT_RUN**

| 구분 | 체크리스트 | 직접 대상 |
|---|---|---|
| unit | 정상 terminal 종류/질문 ID와 잘못된 kind·다른 질문 연결의 구분; 손상은 Corrupt로 반환 | store 결과 검증, app::terminal / RV-01 |
| unit | truncation 활성/비활성 tokenizer에 같은 prompt를 넣어 전체 token 길이를 검증; 인메모리 처리 뒤 원본 파일 불변 | model context 준비 / RV-02 |
| unit | 이웃 fetch cap 및 hop cap 도달 시 플래그; 필터로 제외되는 origin과 중복 이웃 | retrieval / RV-03 |
| integration | 두 정상 질문의 결과 재시도는 각각 자기 답변만 반환; projection 손상은 모델 재호출/성공 출력 없이 오류; 명시적 reindex 뒤 정상 복구 | app+store+CLI / RV-01 |
| integration | 실제 tokenizer/template → 낮은 순위 evidence 제거 → provided/excluded → canonical AssistantAnswer 일치 | model+app+codec / RV-02 |
| integration | 조회 snapshot을 동기화한 두 연결로 startup 중 commit 및 backup Done 이후 비교 전 commit 경계 확인 | store / RV-04·05 |
| regression | 동일/다른 request key, commit 전후 종료, 입력 보존, 답변 commit 전 성공 미표시 | tests/cli.rs, tests/runtime.rs |
| regression | RIGHT→LEFT→restore RIGHT, 다른 context, stale-head 단일 승자, as_of 동률, late arrival, 만료된 최신 버전의 과거값 부활 방지 | tests/store.rs |
| regression | 동일 10k corpus/top-k의 lexical·graph 비교와 source/session/time/slot 필터, seed 수가 8개 이상인 경우의 실제 graph 기여 확인 | tests/retrieval.rs |
| malformed/boundary input | token 길이가 context−max_tokens와 같음/1 초과; truncation이 질문 또는 evidence를 자르는 방향별 설정; 4/5 hop, 256/257/258 이웃 | model, retrieval |
| malformed/boundary input | version/tag/flags, varint overflow·noncanonical, 잘린 BLOB, checksum, raw/zstd 상한·trailing bytes, 빈 payload 및 UTF-8/NUL 원문 | tests/codec.rs |
| failure-path | 결과 mapping 손상, startup의 실제 watermark 손상, canonical 손상 backup/reindex 거부를 정상 동시성 경우와 구별 | store |
| failure-path | model 없음, load/generation timeout, child 비정상 종료, stderr 포화, 취소, 잘못된 인용, COMMIT 실패에서 input 보존 및 정상 답변 미표시 | tests/runtime.rs |
| failure-path | 실제 local model 5개 한국어 smoke에서 input/evidence IDs·output·latency·종료 이유 기록; 의미 판정은 인용 ID 검사와 분리 | examples/validate.rs |

동시성 테스트는 단순 반복으로 운에 맡기기보다 격리 harness의 동기화 지점으로 해당 interleaving을 확인한다. 제품 checkout을 수정한 상태를 이번 리뷰 결과로 소급하지 않는다. malformed/projection fixture는 후속 허용된 격리 DB에서만 만들며, 사용자 DB를 대상으로 하지 않는다. 테스트 항목은 공격 코드나 외부 target 실행을 요구하지 않는다.

**기존 실행 증거와 미검증 범위**

- `docs/logs/final-tests.txt`: CLI 2, codec 2, retrieval 2, runtime 3, store 3으로 실제 integration 테스트 12개 통과 로그. unit/doc의 0개 실행은 세지 않았다.
- `docs/logs/retrieval-regression.txt`: 기존 retrieval 2/runtime 3 재실행 통과. 별도 신규 테스트 수에 더하지 않는다.
- `docs/logs/runtime-regression.txt`: CLI 2/runtime 3 통과 로그.
- `docs/logs/core-tests.txt`: 초기 codec 테스트 1건이 `Corrupt("zstd decode")`로 실패한 로그가 보존돼 있다. 이후 로그의 통과와 구분했으며 과거 실패를 현행 결함으로 보고하지 않았다.
- `docs/logs/cli-smoke.txt`: 기존 합성 DB lifecycle, reindex, backup/restore/doctor 및 권한 확인 로그. 이번 리뷰가 실행한 결과는 아니다.
- 구현 보고서는 실모델 미설치/미검증, Rust Candle CPU 경로, PARTIAL, B0_READY=NO를 명시한다. 모델 부재 자체를 소스 결함으로 만들거나 fabricated PASS라고 판정하지 않았다.
- G0–G3 VERIFIED는 구현자의 기존 테스트 범위 표기다. 위 RV-01·03·04·05 경계까지 독립 검증됐다는 근거로 사용할 수 없다.
- v1/v2 전체, 관련 없는 인증/권한 영역, 공급망 감사, 외부 서비스/target, 모델 품질 벤치마크는 검토하지 않았다.

**실측 조건 — 기존 로그에서 확인, 새 측정 없음**

`docs/logs/measurements.txt`는 M4/24 GiB, release, synthetic 10,020 events, payload 4,839,200 bytes, 모델 제외, OS cache 미삭제 조건을 명시한다. raw/auto_zstd의 DB+WAL+SHM 총합은 각각 37,337,320 / 31,771,272 bytes이고 FTS pages 12,611,584 bytes는 이미 DB 내부에 포함된다. 백업 크기는 이 총합에 포함되지 않는다. warm lexical n=100의 median은 6.9562 / 7.1869 ms이며 두 경우 모두 100/100 truncated=true다. durable commit n=20, warm read n=128과 구분돼 있다. RSS 27,488 KiB는 모델/GPU/shared memory를 포함한 측정이 아니다. reopen은 OS-cold가 아니다. 실모델 load/first token/generation, 전원 장애, physical disk-full은 이번에도 NOT_RUN이다.

**읽기 전용 검토 명령과 보존 확인**

실행한 검토 명령은 `rg --files`, `rg -n`, `cat`, `sed`, `nl -ba`, `wc -l`, `git status --short`/`git diff --stat`(Git 이력 부재 오류), `uname -sm`, `sysctl -n hw.model hw.memsize machdep.cpu.brand_string`, `sw_vers`, 이미 설치된 toolchain의 `rustc -Vv`/`cargo -V`, `python3 -B`를 통한 파일 SHA-256 계산이다. 빌드/제품 실행/test runner는 실행하지 않았다. Python은 파일 바이트 해시 읽기에만 사용했고, 리뷰 보고서는 사용자 승인 이후에 새로 생성했다.

리뷰 시작과 소스 검토 종료 시점의 48개 기존 파일에 대해, 정렬된 상대 경로별 `경로 + NUL + 내용 SHA-256 + LF`를 SHA-256한 값이 위 REVIEW_MANIFEST_SHA256와 일치했다. `target/` 및 `.git/`은 대상에서 제외했다. 구현자가 남긴 `docs/logs/package-digest.txt`의 26개 항목도 모두 현재 파일 해시와 일치했다(기존 manifest_sha256: `14ea6048addd97ad2a56352ac050dfb1393643ad2bf34cc1b5dd2321d0809063`). 이 검사는 로그가 실제로 현재 코드로 생성됐음을 증명하는 실행 attestation은 아니다.

별도 승인된 저장 단계의 변경은 이 리뷰 보고서 생성과 Git 메타데이터/commit/push이다. 기존 소스, 테스트, lockfile, imsi 문서 및 기존 보고서/로그는 수정하지 않는다. 모델/DB/빌드 산출물은 저장 대상에 포함하지 않는다. 원격 `seoyd/replica-v3`는 저장 단계의 읽기 조회에서 빈 저장소로 확인했다. 업로드 결과와 commit은 최종 응답에서 보고한다.

재검토 범위는 RV-01~05의 수정 함수, 이에 직접 연결된 위 회귀, tokenizer/model 실제 smoke와 저장·재시작·종료 경로로 한정한다. 직접 패치는 적용하지 않았다.

편집 이력 (Goal 1 S0): 임시 입력의 경로 참조만 설명 문구로 바꿨다. 당시 계약 ID,
해시, SOURCE 결함, 실행 NOT_RUN 및 FAIL 판정은 과거 사실로 보존했다. 이 문서와
옛 manifest는 현행 수정 코드의 인증이나 신규 독립 승인으로 사용하지 않는다.
