# Replica v3 · B0 — 구현 전용 프롬프트

문서 버전: B0-CONTRACT-1.0  
적용 범위: 작동하는 첫 Replica v3의 최소 수직 실행 경로  
역할: 구현 담당자. 설계 심의를 다시 여는 사람이 아니라, 아래 계약을 구현하고 사실대로 보고하는 사람이다.

---

## 0. 이번 작업의 목적과 권한

다음 한 경로를 실제 프로그램으로 완성하라.

> 입력 원문 저장 → 관련 과거 증거 검색 → 기존 로컬 학습모델 하나로 답변 → 답변과 사용 근거 저장 → 정상 종료 → 재시작 후 동일 기록 회수.

제품은 **새로운 작은 Rust v3 실행 구조**로 만들고, **기존 Rust replica-v2의 저장·증거·복구 관련 작은 부분만 선별 이식**한다. 기존 v1/v2를 수정하거나 폐기하는 작업이 아니다. TR/GDN/SSM 등 아키텍처를 배제하지 않지만, 이번 제품 실행 경로에는 **기존 학습된 로컬 Transformer 모델 하나**만 연결한다.

사용자는 이 방향과 구현 시작을 승인했다. 아래 범위 안에서 같은 확인을 반복해서 요구하지 말고 진행하라. 다만 기존 사용자 파일 삭제, 원격 push, 모델의 대용량 다운로드, 전역 환경 변경은 승인된 작업에 포함되지 않는다.

이번 작업에서 구현해야 할 기능은 G0–G5 전체다. 단계마다 기능을 추가로 발명하지 않는다. 실행 환경 때문에 특정 단계가 막히면 독립적으로 가능한 단계는 마치고, 실제로 막힌 부분만 구체적으로 표시한다. 가짜 모델·고정 답변·하드코딩된 성공률로 막힌 단계를 대체하지 않는다.

### 이번에 하지 않는 것

- 새 언어모델 사전학습, 모델 가중치의 온라인 업데이트, LoRA 학습.
- MLA 변환, latent thought loop, MTP, FP4 커널, 자체 Metal 연산 구현.
- GDN2·Mamba·Precision-GDN2의 구현·결합·성능 경합.
- 다중 에이전트, 자율 계획 재귀, 도구 자동 실행, 상시 감시·백그라운드 작업.
- 그래프 DB 서버, 분산 DB, 직접 만든 WAL/스토리지 엔진, 별도 벡터 DB 서버.
- v2의 BC53–BC61 전체, 권한 seal 계층, textual-MIR 분석기와 대규모 검증 게이트 이식.
- 오래된 합성 실험표를 근거로 지능·기억률·M4 성능을 주장하는 일.

장래 연구를 위한 추상 계층을 과도하게 만들지 말라. 실제 사용하는 모델 어댑터 하나와 안정된 입출력 경계면이면 충분하다.

---

## 1. 확정된 사용자 조건

**R01 — 실행:** Mac mini M4, 통합 메모리 24GB가 최종 검증 대상이다. 다른 호스트에서 돌린 결과를 M4 실측이라고 쓰지 않는다.

**R02 — 독립성:** v3는 v2 전체를 빌드하거나 실행하지 않고 자체 빌드·테스트·실행할 수 있어야 한다.

**R03 — 원본:** 저장 범위에 들어온 사용자 발화, 명시적 명령, 해석 결과, 답변은 원문 바이트를 보존한다. 중요도·오래됨·요약 여부로 원문을 자동 삭제하지 않는다.

**R04 — 포맷:** 개인 기억의 canonical 레코드는 schema-specific binary다. JSON/JSONL/YAML/문자열 map의 직렬화를 canonical 본문으로 사용하지 않는다. SQLite 안의 BLOB에 바이너리를 저장하는 것은 허용한다.

**R05 — 구분:** 원문 UTF-8, 디버그 표시, SQL 이름, 모델의 표준 config/tokenizer 파일, 일시적인 로컬 IPC JSON은 R04의 금지 대상이 아니다. 바이너리라는 이유로 암호화됐다고 주장하지 않는다.

**R06 — 정정:** 정정은 새 버전의 추가다. 원래 발화나 과거 버전을 UPDATE/DELETE해서 바꾸지 않는다. 현재 사실, 과거 사실, 당시 알려져 있던 사실을 구별한다.

**R07 — 저장 기반:** B0는 한 SQLite 데이터베이스를 트랜잭션 경계로 사용한다. canonical 레코드와 payload를 별도 `.pack` 파일에 분산해 이중 commit 문제를 만들지 않는다.

**R08 — 파생 정보:** 검색 색인, 현재값 projection, 관계 adjacency는 canonical 레코드로 재구축할 수 있어야 한다. 모델이 생성한 해석 자체는 버전·출처를 가진 별도 레코드로 남긴다.

**R09 — 모델:** 제품에는 실제 기존 로컬 모델 하나만 연결한다. 모델은 기록·SQL·파일·프로세스 실행 권한을 갖지 않는다. 클라우드 fallback은 없다.

**R10 — 실행 한도:** B0의 질문 하나는 최대 한 번의 모델 생성 호출을 한다. 재시도·자기비평·재검색의 자동 순환은 없다. 시간·출력·검색 방문량이 유한해야 한다.

**R11 — 사실성:** `precedes`, `supports`, `causal_hypothesis`를 구분한다. 앞서 발생했다는 이유만으로 원인으로 확정하지 않는다.

**R12 — 검증:** 수치·PASS·테스트 횟수는 실제 실행에서 나온 것만 기록한다. mock 통과와 실제 모델 통과, 다른 OS 통과와 M4 통과를 분리한다.

**R13 — 변경:** 기존 사용자 작업물, 기존 저장소, 모델 파일, 개인 DB를 파괴하거나 자동 변환하지 않는다. 실험은 새 임시 DB와 합성 자료로 수행한다.

**R14 — 재사용:** 이식 코드는 출처 commit·경로·함수·수정 이유·라이선스를 남긴다. 분리가 어렵다면 억지 이식보다 작은 독립 구현을 택하고 이유를 기록한다.

**R15 — 완성:** 메모리 API만 구현해서 B0 완성이라고 보고하지 않는다. 실제 로컬 모델 연결과 재시작 통합 검증이 포함돼야 한다.

**R16 — 종료:** 이번 범위가 끝나면 멈춘다. 다음 코어, GUI, 로봇, 학습기, 정교한 인과엔진으로 자발적으로 확장하지 않는다.

---

## 2. 입력 자료와 기준점 확인 — G0

### 2.1 자료를 혼동하지 말 것

참고용 Rust v2 기준:

- 저장소: `seoyd/replica-v2`
- 이전 검토에서 확인한 commit: `063be980ab0c8233497dcc29fd7035f9f22074c6`
- 확인 대상: `Cargo.toml`, `src/lib.rs`, `src/snapshot.rs`, `src/local_store.rs`, `README.md`, `WORK.md`.
- 이 commit의 `Cargo.toml`은 Rust 라이브러리이고 외부 dependency 항목이 비어 있다. v3가 이 제약까지 계승해야 한다는 뜻은 아니다.
- snapshot에는 `RPV2SNAP`, 버전, varint, CRC32C와 길이 제한 등이 있다. 저장 모듈 전체는 BrainGraph·정책 자료형에 결합되어 있으므로 통째 복사하지 않는다.

첨부 ZIP:

- 이름: `replica-main(260508).zip`
- SHA-256: `1439313184cff00f23158fd491df4306660be88a53ad0df9f65357dca997e53a`
- 실제 검사에서 `.zig` 파일 2,328개, `.rs` 0개, `Cargo.toml` 0개였다.
- 이것은 **Zig 계열 참고 자료**다. Rust replica-v2 최신본으로 취급하지 않는다. 파일명의 날짜만으로 구현 상태를 판단하지 않는다.
- 압축파일의 실행 코드·빌드 스크립트를 읽기 전에 실행하지 않는다. 필요한 파일만 안전하게 읽고, 경로 탈출·심볼릭링크를 통한 외부 쓰기를 허용하지 않는다.

현재 작업환경에서 자료가 달라졌다면 실제 HEAD와 dirty 상태를 읽고 위 기준과 구분해 보고하라. 원격 확인이 가능하면 connector 또는 인증된 read 경로를 사용한다. 인증정보를 출력하거나 파일에 복사하지 않는다.

### 2.2 시작 점검

대상 workspace에 실제 적용되는 `AGENTS.md` 등 작업 지침부터 읽는다. 이전 저장소 전용 지침을 새 형제 v3에 무분별하게 복제하지 않는다. 적용 범위가 충돌하면 기존 파일을 우회 수정하지 말고, 승인된 v3 격리 경계와 충돌 내용을 기록한다.

다음을 `docs/IMPLEMENTATION_REPORT.md`의 시작 부분에 짧게 기록한다.

- 실제 workspace와 v3 대상 경로, v2 read-only 경로.
- 각 저장소 HEAD, 시작 시 수정·미추적 파일 목록.
- 호스트 OS/architecture, `rustc -Vv`, `cargo -V`.
- 사용할 SQLite runtime 버전/컴파일 옵션.
- 실제 설치된 로컬 추론 runtime, 모델 경로·모델 식별자·revision·양자화 형식·라이선스.
- 이번 실행에서 읽지 못했거나 존재하지 않는 자료.

명령 출력으로 확인되지 않은 버전·모델·GPU 지원은 추정해 적지 않는다. v3 toolchain은 실제 사용 가능한 안정 버전을 선택해 고정한다. v2 문서의 Rust 1.97 또는 검증 전용 1.97.1을 이유 없이 요구하거나 전역 toolchain을 바꾸지 않는다.

### 2.3 v3 작업공간

기본은 기존 v1/v2의 형제 디렉터리 `replica-v3/`다. 이미 v3가 있으면 덮어쓰지 말고 상태를 읽어 이어간다. 사용자 지정 workspace가 있다면 그 안에서 격리된 v3 package를 사용한다.

기존 v2를 path dependency로 붙이지 않는다. `git reset --hard`, `git clean`, 무단 checkout·merge·commit·push를 하지 않는다. 필요한 버전·dependency는 v3의 lockfile에만 반영한다.

---

## 3. 그래프 엔지니어링의 정확한 적용

여기서 그래프 엔지니어링은 **요구사항 → 자료형/불변조건 → 구현 모듈 → 테스트 → 실행 증거**의 관계를 추적하는 개발 방식이다. 그래프가 신경망 대신 지능 전체를 수행한다는 뜻이 아니다.

`docs/PLAN.md` 하나에 다음 유향 비순환 작업 그래프를 둔다. 별도 graph DSL·그래프 실행 서버·계약 생성기를 만들지 않는다.

```
G0 소스·환경·범위 확인
 ├─ G1 사건 자료형 / 바이너리 codec
 │    └─ G2 SQLite 영속성 / 복구 / backup
 │          └─ G3 정정·history / 검색 / 관계 탐색
 └─ G4 실제 로컬 모델 어댑터
       G2 + G3 + G4
               └─ G5 CLI 통합 / 재시작 / 독립 fixture / 측정
```

각 노드는 상태 `NOT_STARTED | IMPLEMENTED | VERIFIED | BLOCKED`와 해당 파일·테스트·실행로그 경로를 갖는다. 보고서 행이 있다는 이유만으로 VERIFIED로 바꾸지 않는다.

한 번에 한 실행 경로를 닫는다. 다른 모듈의 미구현을 가리기 위해 가짜 성공을 반환하지 않는다. 같은 실패를 새 증거 없이 반복 실행하지 말고, 원인과 최소 재현을 먼저 정리한다.

---

## 4. 최소 코드 구조와 의존성

새 프로젝트에 아래 정도의 분리면 충분하다. 기존에 같은 책임의 v3 파일이 있으면 그것을 사용하고, 아래 이름을 맞추려고 중복 파일을 만들지 않는다.

```
Cargo.toml / Cargo.lock
src/main.rs             # CLI와 표시. 비즈니스 규칙을 여기에 몰지 않는다.
src/lib.rs
src/event.rs            # 사건, 버전, 관계, 식별자, 시간
src/codec.rs            # canonical binary encode/decode
src/store.rs            # SQLite transaction, replay, backup
src/retrieval.rs        # 검색 및 bounded relation traversal
src/model.rs            # 단일 실제 모델 어댑터와 공통 입출력
src/app.rs              # persist → retrieve → generate → persist
bridge/mlx_worker.py    # MLX를 실제 선택한 경우에만 생성
migrations/001_init.sql
 tests/                 # codec/store/history/runtime별 소수 파일
 docs/PLAN.md
 docs/STORAGE_FORMAT.md
 docs/REUSE.md
 docs/RUNBOOK.md
 docs/IMPLEMENTATION_REPORT.md
```

이 목록은 상한을 늘리라는 지시가 아니다. 한 상태·한 에러·한 phase마다 파일을 만들지 않는다. 공통 함수를 복사하는 대신 실제 책임 단위로 공유한다.

v3의 application 코드는 기본적으로 safe Rust로 작성한다. SQLite·compression·runtime dependency의 내부 구현까지 unsafe 금지 규칙을 확장하지 않는다. `rusqlite` 계열 SQLite 바인딩, 검증된 CRC32C, 필요시 zstd, CLI/오류/임시디렉터리 지원 정도의 작은 dependency 집합은 이 작업 범위에서 허용한다. 실사용 없는 라이브러리와 async framework를 선제 도입하지 않는다.

추론 worker 관리에 필요한 유한한 reader thread 또는 OS I/O 기능은 허용한다. 이것을 거대한 비동기 에이전트 framework로 확대하지 않는다.

### v2 선별 이식

`docs/REUSE.md`에 한 행씩 기록한다.

`source_repo / commit / path / symbol / reused_or_reimplemented / reason / destination / test / license`

우선 후보는 canonical varint 검사, CRC32C, checked length 계산, 출처·시간·정정 의미, bounded read·파일 복구에 관한 작은 테스트다. SQLite를 사용하면서 필요하지 않은 원자 파일 교체 코드까지 억지로 가져오지 않는다.

`use super::*`와 함께 전체 BrainCore가 따라오는 이식은 금지한다. 전이 의존성이 커지면 그 부분은 독립 구현하고 동등한 경계 테스트를 작성한다. 이식률 목표는 없다.

---

## 5. 사건과 사실의 계약 — G1

### 5.1 세 가지를 혼동하지 말 것

1. **Observation:** 누가 어느 시점에 정확히 무엇을 말하거나 실행했는지에 대한 원본 기록.
2. **Assertion / interpretation:** 원본을 바탕으로 특정 사실 또는 관계를 주장한 기록.
3. **Projection:** 위 기록으로부터 계산한 현재값, 검색 결과, adjacency.

사용자가 “오른쪽으로 가”라고 말한 기록은 이후 “왼쪽이었다”고 정정해도 사라지거나 변하지 않는다. 바뀌는 것은 해당 사실 슬롯의 현재 유효한 주장이다.

모델의 답변은 `AssistantAnswer` 또는 모델 출처의 해석으로 남긴다. 자동으로 신뢰된 사용자 사실이나 새 운영 지시로 승격하지 않는다.

### 5.2 필수 논리 필드

실제 자료형·kind별 바이너리 배치는 구현 전에 `STORAGE_FORMAT.md`에 고정한다. 범용 map이 아니라 필요한 필드만 갖는 enum/struct로 작성한다.

- 양수의 단조 증가 `EventId`. SQLite signed INTEGER 범위를 넘지 않는다. 미commit 번호의 gap/rollback은 허용하되, commit된 ID는 재사용하지 않는다.
- `recorded_at`: 시스템에 기록된 시점. UTC 기준 정수 단위와 해상도를 문서화한다.
- `observed_at`: 원본 사건 시점. 알려진 경우에만 저장하며 recorded_at과 구별한다.
- 필요한 사실 kind의 `valid_from`, 선택적 `valid_until`. unknown과 무한 범위를 숫자 0으로 혼동하지 않는다.
- `source`, `session/scope`, `kind`.
- 원문 payload의 정확한 바이트.
- kind별 `slot_key`, `previous_event_id`, `restored_from`, `input_event_id`, `evidence_refs` 등 필요한 참조만 저장.
- 재시도 식별용 `request_key`가 필요한 kind에는 이를 넣는다. UUID 크기나 표현은 고정하되 임의 추정으로 생성 규칙을 바꾸지 않는다.

source/session을 정수로 intern하면 dictionary 역시 canonical·append-only이고 같은 트랜잭션으로 commit해야 한다. 복원에 필요한 dictionary를 “삭제 가능한 index”라고 분류하지 않는다.

### 5.3 사실 슬롯

`slot_key`는 적어도 소유 scope, entity, predicate, context를 구분하는 안정된 바이너리 키다. 구분자 문자열 연결로 충돌을 만들지 않는다. 길이와 필드 경계가 명확해야 한다.

- 같은 entity라도 predicate 또는 context가 다르면 다른 슬롯.
- observation timestamp만 달라졌다는 이유로 항상 다른 슬롯을 만들지는 않는다.
- 유사도·cosine·문장 유사성만으로 동일 슬롯이라고 확정하지 않는다.
- B0의 확정적인 슬롯 지정·정정은 명시적 CLI/API 명령으로 수행한다. 자연어 자동 정정 판정기는 이번 필수 범위가 아니다.
- 일반 `ask` 입력도 원문으로 저장되지만 자동으로 사실 덮어쓰기를 수행하지 않는다.

### 5.4 값과 근거는 임의로 압축 요약하지 않는다

원문 NUL, 줄바꿈, 따옴표, 한글 조합형/완성형, emoji를 round-trip 보존한다. 검색을 위해 정규화한 사본은 파생 정보다. canonical 원문에 normalization·trim·말줄임을 적용하지 않는다.

가장 작은 레코드라는 전역 최적성을 주장하지 않는다. B0에서는 읽기 비용과 복구 가능성을 포함한 측정 기준점을 만든다.

---

## 6. 바이너리 포맷과 압축 — G1

### 6.1 범위

canonical 사건 본문에 JSON/CBOR/MessagePack/Protobuf map을 넣지 않는다. 짧은 header와 고정 순서/variant별 명시 필드를 갖는 schema-specific format을 구현한다. 문자열 원문 자체가 UTF-8인 것은 허용한다.

권장 envelope는 다음의 의미를 갖는다. 실제 byte 수·endianness·tag 값은 코딩 전에 문서와 golden fixture에 하나로 고정한다.

`magic / format_version / codec_id / flags / raw_length / stored_length / checksum / encoded_body`

- magic은 v2와 구분되는 v3 식별값을 사용한다. v2 snapshot이라고 위장하지 않는다.
- 정수는 명시적인 little-endian 고정필드 또는 canonical unsigned LEB128/zigzag로 인코딩한다.
- Rust 메모리 layout, `repr(Rust)`, native endianness, 포인터, padding을 디스크 규격으로 사용하지 않는다.
- unknown version/codec/tag/flag, 과장된 길이, truncated varint, overflow, noncanonical varint, 허용하지 않은 trailing bytes를 명시적 오류로 거부한다.
- compression은 `raw`와 `zstd` 두 경로까지만 구현한다. LZ4·dictionary·block pack·다중 파일 compaction은 뒤로 미룬다.
- 짧거나 압축 이득이 없는 payload는 raw로 둔다. 손실 없는 실제 길이 비교로 선택하고 임계값은 configuration 상수로 문서화한다.
- raw_length를 믿고 무제한 할당하지 않는다. encoded/decoded 모두 상한을 확인하며 bounded decompression을 사용한다.
- checksum은 손상 탐지다. 공격자에 대한 인증·암호화·변조 불가능성의 증명이라고 쓰지 않는다.

### 6.2 포맷 크기를 줄이는 우선순위

반복 header를 줄이기 위해 저장 안전성을 희생하지 않는다. B0는 레코드 단위 SQLite BLOB로 충분하다. block 단위 압축의 잠재적 이득은 후속 benchmark 대상이다. 사용자의 정상 입력을 일정 건수까지 메모리에 묶어두고 저장 완료라고 먼저 답하는 일은 금지한다.

payload deduplication은 선택 기능이다. 넣으면 동일한 바이트와 서로 다른 사건을 구별한다. 같은 말 두 번은 사건 두 개이며, payload만 공유할 수 있다. digest 일치만 믿지 말고 충돌 시 원문 바이트 확인 정책을 둔다. 이 기능이 B0를 지연시키면 구현하지 말고 명시적으로 남긴다.

### 6.3 codec 최소 테스트

서로 독립적으로 작성한 literal bytes golden fixture를 최소 1개 둔다. encode를 호출해 만든 결과를 그대로 expected로 사용하지 않는다.

정상 fixture, 경계 정수, 빈 payload, 다국어, 줄바꿈, invalid tag/version, 잘린 입력, 비정상 varint, checksum 손상, 과대 길이, zstd 손상/과대 해제의 경우를 검증한다. deterministic encode와 decode(encode(x)) == x를 둘 다 확인한다.

---

## 7. SQLite canonical 저장과 트랜잭션 — G2

### 7.1 저장 경계

B0는 하나의 SQLite DB 안에 canonical records/payload/dictionary를 두고, 파생 metadata/index/heads/edges를 둔다. SQL schema 자체가 이름과 자료형을 갖는 것은 허용한다. 사건별 반복 JSON object는 허용하지 않는다.

대표 개념 테이블은 다음과 같다. 실제 테이블 수는 필요한 만큼만 둔다.

- `records`: immutable event ID + canonical binary.
- 필요하면 immutable source/session dictionary.
- 파생 `record_meta`, `current_heads`, `relations`, `request_state`.
- 파생 검색 색인.

원문과 모델 해석의 원천은 canonical records다. metadata는 검색 가속용이며, decoder가 읽은 값과 metadata가 불일치하면 조용히 한쪽을 신뢰하지 않는다. typed corruption 오류 또는 검증된 재구축 경로로 처리한다.

### 7.2 내구성

한 writer를 사용하고 새 DB에서 다음 설정을 적용·재조회하여 실제 적용값을 기록한다.

- `foreign_keys = ON`.
- `journal_mode = WAL`.
- `synchronous = FULL`.
- 유한한 `busy_timeout`.
- macOS의 `fullfsync` 적용 여부와 실제 반환값은 확인·기록한다. flag만으로 물리적 전원 장애 내구성을 검증했다고 주장하지 않는다.

설정 실패를 성공으로 넘어가지 않는다. WAL 미지원 filesystem에서는 정상 모드로 위장하지 말고 원인과 대안 필요성을 보고한다. 공유 network filesystem을 초기 데이터 경로로 사용하지 않는다.

하나의 논리 append는 `BEGIN IMMEDIATE` 트랜잭션에서 ID 발급, canonical 레코드, 참조 무결성, 현재 projection·검색 색인 갱신을 함께 처리한다. **COMMIT 성공 후에만 저장 완료를 반환**한다. model inference를 수행하는 동안 DB write transaction을 열어놓지 않는다.

B0의 애플리케이션 경로에는 canonical UPDATE/DELETE API가 없다. DB trigger 또는 동등한 제한으로 우발적인 변경을 막는다. 관리자·root가 파일을 직접 바꾸는 것까지 불가능하다고 주장하지 않는다.

### 7.3 재시도와 중복

같은 `request_key`와 같은 요청 내용은 기존 commit 결과를 돌려준다. 같은 key에 다른 내용이 오면 충돌 오류다. 내용이 같아도 별개의 request_key로 들어온 실제 두 발화는 두 사건으로 남긴다.

commit 후 출력 전에 종료된 경우 재시도로 중복 사건·중복 확정 답변이 생기지 않아야 한다. crash 뒤 모델 추론을 절대로 두 번 하지 않는다는 보장은 하지 않는다. 보장 범위는 **영속적인 request/input/result의 중복 commit 방지**다.

### 7.4 재시작과 손상

DB가 기존 경로에 있는데 읽을 수 없다는 이유로 새 빈 DB를 생성하지 않는다. schema/version 불일치를 자동 삭제로 해결하지 않는다.

startup은 DB 식별자·schema·빠른 무결성 검사·필요한 projection 버전을 확인한다. 전체 원문 checksum scan은 명시적 `doctor --full`에 둔다. 정상 startup마다 전체 과거를 재학습하거나 재인덱싱하지 않는다.

파생 색인의 재구축은 명시적 maintenance 명령이다. canonical 스캔→새 projection 작성→같은 DB의 원자 교체/transaction 경계를 사용한다. 중간 종료 시 과거 projection 또는 완성된 새 projection 중 하나가 있어야 한다. canonical 손상이 있으면 이를 index rebuild로 숨기지 않는다.

### 7.5 backup / restore

SQLite backup API 등 일관된 snapshot 경로를 사용한다. 열린 WAL DB의 `.db` 파일만 임의 복사하지 않는다.

backup은 사용자가 명시적으로 호출하며, 새 destination에 만든다. restore는 기존 개인 DB에 덮어쓰지 않고 새 경로로 수행한다. 복원본에서 format·checksum·참조·원문 동일성을 검사한다. checksum만으로 손상된 원문을 복구할 수 있다고 주장하지 않는다.

---

## 8. 정정·history·관계 — G3

### 8.1 명시적 정정의 원자성

초기 Assertion E1 이후 Correction E2는 같은 slot에 속하며 `previous_event_id=E1`을 가진다. E1은 그대로 남는다. 사용자는 expected current head를 지정할 수 있고, 실제 head와 다르면 `Conflict`를 반환한다.

두 요청이 같은 head를 동시에 정정하려는 경우 하나만 성공하고 다른 하나는 stale-head 충돌이어야 한다. 후자가 앞선 결과를 조용히 덮어쓰면 안 된다.

새 context의 Assertion은 다른 slot이다. 유사한 텍스트·근처 시점·같은 entity만으로 correction으로 바꾸지 않는다.

### 8.2 rollback

Rollback은 DB를 과거 파일로 되돌리는 작업이 아니다. E3를 새로 추가한다.

`E1 = RIGHT; E2 = LEFT, previous=E1; E3 = RIGHT, previous=E2, restored_from=E1`

현재 head는 E3이고 E1/E2/E3 모두 조회 가능해야 한다. 다른 lineage/scope의 event를 restore target으로 쓰는 것은 거부한다. 원본 값의 hash만 남기는 것이 아니라 실제 값이 복원되어야 한다.

### 8.3 시간 질의

- `history`: 기록 순서대로 전체 lineage.
- `current`: 현재 known snapshot에서 현재 유효한 버전.
- `as_of`: 당시까지 기록된 사건만 사용한 결과.
- event time과 recorded time이 뒤바뀐 late arrival fixture를 포함한다.

B0는 단일 슬롯의 명시적 선형 버전 계보를 지원한다. 일반적인 모든 구간 겹침·다중 분기 병합 엔진을 만들지 않는다. 지원하지 않는 모호한 유효구간 요청은 명시적으로 거부하고 현재값을 임의 선택하지 않는다. 정확한 supported semantics와 tie-break는 STORAGE_FORMAT/RUNBOOK에 적는다.

### 8.4 관계

관계 기록도 근거·출처가 있는 canonical event다. graph adjacency 테이블은 파생물이다.

최소 관계: `used_evidence`, `precedes`, `supports`, `contradicts`, `causal_hypothesis`, `supersedes`, `restores`.

정정/복원 관계는 transaction 규칙으로 검증한다. 일반 관계는 존재하는 event들을 참조해야 한다. 모델의 추정은 자동으로 확정 인과 edge가 되지 않는다. 순환 관계 입력이 있어도 traversal은 visited set와 유한 budget으로 종료해야 한다.

---

## 9. 검색 — G3

처음에는 시간·scope·slot의 exact filter와 lexical search, bounded relation expansion만 구현한다. embedding 모델과 vector index는 B0 완료 조건에서 제외한다. 이를 semantic search 완성이라고 부르지 않는다.

### 9.1 한국어와 원본 보존

FTS5 등 설치된 SQLite 기능을 실제 확인하고 사용한다. 한국어 형태소 처리가 자동으로 된다고 가정하지 않는다. trigram을 채택한다면 1–2글자 질의의 별도 bounded fallback을 구현하고, 긴 문서의 무제한 전체 스캔으로 숨기지 않는다. 짧은 질의가 너무 넓으면 범위 제한을 요청하는 명시적 결과를 준다.

검색용 정규화는 별도 version을 가진다. 원문은 변경하지 않는다. FTS가 원문을 중복 저장한다면 해당 크기를 별도 계상한다. 처음부터 contentless FTS를 위해 복잡한 extension을 만들지 않는다.

사용자 입력은 SQL parameter로 바인딩한다. FTS query 문법 역시 무조건 직접 통과시키지 않고 escape/construct한다. malformed query는 명시적 오류이며 SQL이나 파일 동작으로 바뀌지 않는다.

### 9.2 공통 검색 결과

결과는 다음을 포함한다.

`event_id / original_excerpt / source / recorded_at / observed_at / version_status / retrieval_reason / relation_path`

검색 결과를 보여줄 때 발췌가 잘렸으면 표시한다. exact 원문 조회는 별도 `show event_id`로 제공한다. 현재값 질문에서는 이전 버전을 현재값처럼 섞지 않는다. history 질문에서는 superseded 기록도 보여준다.

질문 자체를 저장한 Observation과 과거 AssistantAnswer를 기본 근거 후보에서 제외한다. 모델이 자기 답변을 반복 인용해 가짜 증거를 쌓지 않게 한다. 사용자 명시 요청으로 역사 조회할 때는 그 기록들도 조회할 수 있다.

### 9.3 검색 budget 기본값

아래는 측정 결과가 아니라 B0의 안전한 출발 설정이다. 코드에서 조용히 무제한으로 바꾸지 않는다.

- candidate 최대 64개, 최종 evidence 최대 8개.
- relation 최대 4 hop, visited event 최대 256개.
- scope 필터는 모든 검색 경로에 동일하게 적용.
- node budget 또는 시간 budget 도달은 `truncated=true`로 보고.
- seed 후보에 정답 ID를 benchmark가 몰래 주지 않는다.

그래프를 사용하지 않는 검색, 그래프 확장 검색 모두 같은 canonical 자료와 같은 top-k로 시험한다. 저장된 관계가 있다는 것과 인과를 추론해냈다는 것을 구분한다.

---

## 10. 실제 로컬 모델 어댑터 — G4

### 10.1 선택

현재 환경에서 실제로 로드되는 기존 모델 하나를 확인한다. 공개 모델 이름을 추정해서 존재한다고 적지 않는다. 사용자의 설치본 또는 지정한 local path를 우선한다.

Mac에서는 이미 사용 가능한 MLX-LM 경로가 있으면 이를 얇은 worker로 연결하는 것이 기본 선택이다. 다른 기존 로컬 runtime을 쓰는 경우에도 **어댑터는 하나만** 구현하고 선택 이유를 기록한다. 최종 기준점의 모델·revision·quantization·runtime을 고정한다.

모델을 처음부터 학습하지 않는다. FP4, MLA, latent reasoning을 추가로 구현하지 않는다. 모델이 원래 제공하는 동작과 generation limit만 사용한다. 가중치·tokenizer·chat template는 원본 runtime 형식을 유지한다.

모델이 없거나 M4가 아니면 저장·검색 구현은 진행한다. 실제 추론은 `BLOCKED_MODEL` 또는 `NOT_RUN_TARGET_DEVICE`로 남긴다. 모의 모델의 문장을 실제 응답처럼 반환하는 제품 fallback은 금지한다.

### 10.2 경계

Rust application은 `ModelRequest → ModelResponse/ModelError`만 안다. backend 내부 KV와 다른 모델의 state를 공통 자료형으로 강제하지 않는다.

ModelRequest에는 system instruction, 사용자 입력, bounded EvidenceBundle, 출력/시간 한도, request ID가 들어간다. evidence는 인용자료이며 운영 지시보다 낮은 신뢰도다. 원문 내부의 “이전 지시를 무시해라”, shell 명령, SQL은 실행 지시가 아니다.

ModelResponse에는 최종 텍스트, 종료 이유, 실제 사용량을 얻을 수 있는 경우의 토큰 수, 모델 식별자, 요청 ID가 들어간다. runtime이 주지 않은 값을 추정치와 실측치로 구분한다.

### 10.3 프로세스 및 IPC

MLX worker를 쓴다면 explicit argv로 실행하고 `sh -c`, 사용자 문자열 shell interpolation을 사용하지 않는다. worker는 v3가 시작한 foreground 세션 동안만 존재한다. 정상 종료·취소·timeout에 소유한 child를 회수하고 좀비를 남기지 않는다.

길이 제한된 IPC를 사용한다. 기존 protocol의 ephemeral JSON을 사용해도 되지만 원본 request JSON을 canonical memory로 저장하지 않는다. stdout은 protocol, stderr는 bounded diagnostic으로 분리한다. stderr를 읽지 않아 child가 교착되는 문제를 방지한다.

실행 가능한 worker 경로는 configuration에서만 온다. 모델 출력이 경로·argv를 바꾸지 못한다. 개인 evidence·prompt를 cloud telemetry나 Hugging Face upload에 보내지 않는다. 자동 모델 다운로드도 기본 OFF다.

### 10.4 한도와 context

- 동시 생성 1개, 질문당 생성 호출 1개.
- 기본 새 토큰 한도 512. context 한도는 모델이 실제 지원하는 값 이하이며 B0 기본 상한은 8,192에서 시작한다.
- tokenizer로 실제 입력 토큰을 계산한다. evidence가 넘치면 순위가 낮은 묶음부터 제외하고 제외 사실을 기록한다. 원문 보관에는 영향이 없어야 한다.
- startup/load와 generation timeout을 분리하고 유한하게 설정한다. 초기 설정값과 실제 측정시간은 구분한다.
- max request bytes, max response bytes, stderr 보관량을 명시한다.
- 별도의 hidden CoT 추출·무제한 thinking loop를 구현하지 않는다. 근거와 간단한 답변 이유를 요청한다.

### 10.5 가짜 모델의 허용 범위

unit/integration test의 의존성 대역으로만 명시적인 FakeModel을 허용한다. release 제품의 기본 경로와 real smoke 경로에서는 사용할 수 없다. 단순한 예문 답변기로 제품 동작을 대신하지 않는다.

---

## 11. 애플리케이션 실행 순서 — G5

`ask`의 필수 순서:

1. 입력 크기·scope·request_key 검증.
2. 원문 Observation을 transaction으로 저장하고 commit.
3. commit된 snapshot 기준으로 과거 증거 검색. 질문 자신을 근거로 검색하지 않는다.
4. 기존 모델 하나에 bounded EvidenceBundle을 전달.
5. 출력의 event 인용 ID가 실제 제공한 bundle 안에 있는지 확인. 존재하지 않는 인용을 조용히 정상 답변으로 표시하지 않는다.
6. AssistantAnswer, input 참조, 사용한 evidence ID, 모델·runtime revision, generation 조건을 binary record로 commit.
7. commit 완료 후 최종 답변과 인용을 사용자에게 표시.
8. 요청 종료. 자동 반성·후속 질문·추가 생성으로 계속 돌지 않는다.

모델이 실패하면 입력 원문은 남는다. 가능한 경우 실패 event를 추가하고 오류를 알린다. DB가 실패하여 오류 event마저 기록할 수 없으면 stderr에 명확히 알리고 영속 기록 성공을 주장하지 않는다.

답변을 commit할 수 없으면 성공한 최종 답변으로 출력하지 않는다. B0는 이 경계를 간단히 유지하기 위해 최종 답변을 buffer한 뒤 commit하여 표시한다. token streaming은 후속 기능이다.

인용 ID가 존재한다는 검사만으로 답변의 의미적 사실성을 보증하지 않는다. 출력에 근거가 부족하면 불확실성을 표시하고, 해당 성능은 실제 모델 평가 항목으로 남긴다.

---

## 12. 최소 CLI 계약

아래 기능은 실제 실행 가능해야 한다. 정확한 옵션 이름은 기존 v3 CLI가 있으면 호환되게 유지하되 RUNBOOK에 최종 명령을 제시한다.

- `init`: 새 DB 생성. 기존 DB를 초기화/삭제하지 않는다.
- `record`: raw Observation 추가. stdin 또는 파일을 받을 수 있고 문자열을 임의 trim하지 않는다.
- `fact add`: 명시적 새 사실 슬롯 생성.
- `fact correct`: expected head와 새 값을 받아 새 correction event 추가.
- `fact restore`: 같은 계보의 과거 버전을 새 current version으로 복원.
- `fact current` / `fact history`: 현재와 과거를 구분해서 조회.
- `search` / `show`: bounded 검색과 정확 원문 조회.
- `relation add`: 타입·근거·참조를 검증하는 명시적 관계 기록. 모델의 자동 실행은 없다.
- `ask` / 선택적 `chat`: 실제 로컬 모델 응답. chat도 각 turn은 같은 bounded 경로를 사용.
- `doctor`: 무결성 및 상태 점검. 기본적으로 자료를 수정하지 않는다.
- `reindex`: 파생물만 다시 만든다.
- `backup` / `restore`: 일관된 snapshot과 새 경로 복구.

전역 `--data-dir` 또는 `--db`를 명시적으로 지원한다. 테스트는 임시 경로만 사용한다. 개인 데이터 경로는 source repository와 분리하고 파일 권한을 제한한다. 로컬 DB의 plaintext 접근 가능성과 OS 수준 보호 의존성은 문서화한다.

---

## 13. 독립적인 필수 테스트와 측정

테스트 갯수를 목표로 늘리지 않는다. 아래 의미 계약을 각각 실제 public API 또는 CLI를 통해 검증한다.

### T01 — codec / binary

정상 round-trip, literal golden bytes, 잘린/손상된/과대 입력, canonical varint, 다국어 바이트 보존, raw/zstd의 동일 복원. decoder가 입력 길이만 믿고 큰 메모리를 할당하지 않는지 확인한다.

### T02 — append와 restart

여러 사건을 저장하고 프로세스를 종료·재실행한 뒤 ID와 원문을 exact 비교한다. 같은 문장 두 번은 서로 다른 사건이어야 한다. 동일 request_key 재시도는 기존 사건이어야 한다.

### T03 — transaction 원자성과 crash

임시 DB에서 commit 직전·직후, response 표시 전 등 제한된 지점에 실제 child-process 종료 시험을 둔다. 재시작 결과는 일관된 이전 상태 또는 commit된 상태여야 한다. success ack를 이미 받은 기록은 남아 있어야 한다.

SIGKILL 테스트를 실제 전원 차단 시험이라고 부르지 않는다. disk-full/I/O fault를 주입했다면 실제 주입 경계를 보고하고, 수행하지 않았다면 별도 미검증으로 표시한다.

### T04 — version lifecycle

`RIGHT → LEFT → restore RIGHT`의 current/history/as_of와 raw observation 불변을 검증한다. 다른 context의 LEFT를 넣어도 원래 slot current는 바뀌지 않아야 한다. stale-head correction·다른 계보 restore·없는 참조는 실패한다.

### T05 — index rebuild / backup

파생 index만 제거한 disposable DB에서 rebuild한 뒤 같은 canonical rows·원문과 검색 결과를 얻는지 검사한다. canonical 손상은 명시적 오류여야 한다. 열린 WAL 상태의 backup을 새 DB로 복원해서 비교한다.

### T06 — 늦게 중요해진 증거

10,000개 이상 합성 사건에 여러 세션, 같은 경로 이름, 반복된 LEFT/RIGHT/직진/취소, 가까운 시점의 다른 사건을 넣는다. 정답표는 제품 검색 코드에 전달하지 않는다.

질의는 자연스러운 문장과 범위 정보만 사용한다. `show known-id` 테스트와 retrieval recall 테스트를 분리한다. “오른쪽 지시가 있었음”과 “그 지시가 사고의 원인임”은 다른 정답 항목이다. 고정 fixture에서 관련 증거가 top-k 안에 있는지, 근거 없는 원인을 확정하지 않는지를 각각 채점한다.

graph 확장과 lexical-only를 같은 데이터·관계·top-k에서 비교한다. edge가 사전에 주어진 결과는 retrieval 결과이지 인과 발견 증명이 아니다.

### T07 — runtime 실패 경계

model 없음, child 조기 종료, timeout, stderr 대량 출력, 길이 초과, invalid IPC, 잘못된 인용 ID, DB commit 실패, 사용자 취소를 시험한다. 각각의 대역 시험은 대역임을 표시한다. 실패가 무한 재시도나 가짜 성공으로 바뀌면 안 된다.

### T08 — 실제 모델 smoke

실제 local model load, 한국어 응답, 과거 evidence 포함, restart 후 다시 질문, source citation 확인, 정상 종료를 실제로 실행한다. 최소 5개 고정 질의와 그 결과를 사람이 확인할 수 있게 남긴다. 모델 출력의 임의 문장을 hardcoded 기대 답변으로 대체하지 않는다.

### T09 — 보안·권한 경계

evidence 안의 prompt injection, SQL metacharacter, shell 문자열이 명령으로 실행되지 않는지 검사한다. scope를 달리한 증거가 섞이지 않는지, 모델이 canonical record를 임의 정정할 수 없는지 확인한다. 합성 fixture만 사용한다.

### T10 — 실측

최소한 실제 record 수, 원문 byte 수, DB/WAL/SHM/index 크기, raw와 compression의 전체 저장량, append/read/reindex 시간, 조회 지연 분포, 프로세스 메모리, 모델 load/첫 응답/생성 시간과 측정 환경을 보고한다.

성능 측정은 release build에서 한다. warm/cold, 단건 durable commit/batch import, model 포함/제외를 분리한다. 10건으로 P99를 신뢰도 높은 통계처럼 쓰지 않는다. 작은 표본은 표본 수·관측 min/median/max로 보고한다.

Mac GPU와 shared memory 사용량을 읽지 못하면 RSS를 전체 GPU 메모리라고 쓰지 않는다. 실제 모델 미실행 결과로 “M4에서 여유롭게 동작”이라고 보고하지 않는다. 99% 기억률·프론티어 수준·신규 코어 우월성은 이번 통과 조건이 아니다.

---

## 14. 검증 실행 정책 — 다시 늪에 빠지지 않기

변경한 함수의 unit test → 해당 경로 integration test → v3 전체 테스트를 최종 한 번 수행한다. v1/v2 전체 테스트와 compiler-MIR audit를 자동으로 실행하지 않는다.

예시 명령은 실제 workspace 구조에 맞게 조정하고 최종 실행 명령을 기록한다.

```
cargo fmt --all -- --check
cargo check --locked
cargo test --locked
cargo clippy --locked --all-targets -- -D warnings
cargo build --release --locked
```

real model과 장시간 benchmark는 일반 `cargo test`에 무조건 숨겨 넣지 말고 명시적 명령으로 분리한다. 단, 분리했다는 이유로 수행하지 않고 B0 완료라고 하면 안 된다.

첫 dependency 설치는 v3 격리환경에만 수행하고 lock을 만든다. 이후 검증 중 dependency를 자동 갱신하지 않는다. unavailable network/runtime/model은 환경 blocker로 기록한다.

시간 예산에 걸리면 실제 timeout 결과와 미검증 범위를 남긴다. 테스트 입력 규모를 몰래 줄이고 원래 조건에서 통과했다고 쓰지 않는다. `|| true`, 실제 0개 실행된 test filter, ignored test를 통과로 계산하지 않는다.

---

## 15. 완료 판정과 제출물

### 상태는 둘로 나눈다

- **Implementation:** `COMPLETE | PARTIAL | BLOCKED`.
- **Validation:** `DETERMINISTIC_TESTS_VERIFIED`, `REAL_MODEL_VERIFIED`, `TARGET_M4_VERIFIED`, `NOT_RUN`을 각각 별도로 표시.

**B0_READY**는 G0–G5 필수 구현과 deterministic test, 실제 모델 smoke, 목표 M4 실행이 모두 증거로 확인된 경우에만 사용한다. 다른 환경에서 저장 테스트만 통과했다면 그 사실은 유효하지만 B0_READY는 아니다.

완료 보고서 형식:

```
MODE: IMPLEMENT
CONTRACT: B0-CONTRACT-1.0
TARGET: 실제 v3 경로
SOURCE_BASELINE: 실제 source commit / archive hash
IMPLEMENTATION_STATUS:
VALIDATION_STATUS:
B0_READY: YES | NO

1. 구현한 사용자 실행 경로
2. v2 선별 이식 목록과 제외 이유
3. 최종 파일·dependency 변경 범위
4. G0–G5 상태 / 관련 테스트 / 실제 로그 경로
5. 고정된 모델·runtime·양자화·context 설정
6. 실제 측정값과 환경; 미측정값은 NOT_RUN
7. 남은 재현 가능한 결함 또는 환경 blocker
8. 기존 v1/v2·사용자 데이터가 보존됐는지
9. 독립 검토자가 실행할 정확한 재현 명령
10. 작업 diff 또는 package 식별 digest
```

구현 완료 후 독립 검토자의 PASS를 스스로 대신 선언하지 않는다. optional 기능 미구현을 치명적 결함처럼 늘어놓지 않으며, 필수 기능 미구현을 optional로 바꾸지 않는다.

**지금 G0부터 실행하라. 이미 확정된 전체 아키텍처를 다시 설계하지 말고, 실제로 저장되고 검색되고 답하고 종료되는 가장 작은 v3를 만들어라.**