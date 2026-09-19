# 의존성과 저장 비용 실측

2026-09-19 현재 상태: 제품·학습·도구·테스트의 JSON 직렬화는 자체 R3BIN으로
대체했다. `serde_json`/`safetensors` 직접 의존은 제거했고 SQLite는 유지한다.
실제 `cargo tree --locked --offline -i serde_json`에서 남은 경로는 기존
Candle의 safetensors 및 tokenizers 내부 의존이다. Serde는 Rust 타입 매핑에만
사용하며 JSON parser/writer가 아니다. 모델 weights/Adam은 기존 R3MODEL,
source corpus는 R3CORP, token cache는 R3TOK, native 실험은 R3ER 그대로다.
후속 R3BIN 크기·속도 benchmark는 EXPERIMENT_STATUS.md의 별도 합성 입력 실측에
기록했다. 아래 역사 측정치를 이번 전환의 성능 이득으로 인용하지 않는다.
삭제된 기존 모델·자료도 복구하지 않았다.

계약: R3-CUSTOMIZE-AND-DIAGNOSE-1.0. 조사일: 2026-09-17.
단계: P0 기준 실측, P6에서 현재 JSON/저장 경로 차이 보완. 독립 검토는 INDEPENDENT_PENDING이다.

프로젝트 소스는 Rust이다. 전체 의존 트리가 순수 Rust인 것은 아니다.
외부 모델 가중치·학습된 외부 tokenizer·teacher·모델 API는 사용하지 않았다.
조사는 저장소, 명시된 자체 모델 산출물, 새로 만든 임시 합성 DB에 한정했다.
운영 DB, 자격 증명, 기존 checkpoint를 변경하지 않았다.

## 재현 기준과 한계

- 기준 HEAD: `23cc5b0178048eb7b23775fefb3d9282c1f91152`, `main`, `origin/main`.
- cwd/root: `/Users/seo/Projects/Replica-v3`.
- Rust 1.98.0 / Cargo 1.98.0, edition 2024, Apple M4, 메모리 25,769,803,776 B.
- 실행: release, `--offline --locked --features accelerate`, CPU/F32. Metal 실행은 하지 않았다.
- 이전 S4 코드 WIP는 17개 파일, 6,034행 추가/542행 삭제였다. 이번 조사 구현으로 계산하지 않는다.
- 시작 diff SHA-256: `5b315b8423be0ce4d0e5e605deba5e90a95310b59906a30a23199dd508612ba1`.
- 시작 파일 hash 목록 SHA-256: `538c7cbeadc2bcdef63dd669a418464933b1c72b97daab899df1d5da730e30e8`.
- 원본 증거는 로컬 `artifacts/customize-20260917/`에 보존한다. 원문 로그·모델은 commit하지 않는다.
- 현재 의존 graph는 플랫폼을 `aarch64-apple-darwin`으로 제한한 Cargo metadata와 feature tree로 조사했다. manifest 선언과 실제 feature 합산을 구별했다.
- 측정은 새 프로세스 실행이지만 OS 파일 캐시를 비우지 않았다. 속도는 단일 관측이며 장기 평균이나 실제 저장장치 cold 성능이 아니다. crate별 컴파일 시간/RSS/연산 기여는 UNKNOWN이다.

실행 명령:

```sh
cargo tree --offline --locked --features accelerate -e features
cargo metadata --offline --locked --features accelerate --filter-platform aarch64-apple-darwin --format-version 1
cargo tree --offline --locked --features accelerate -i onig_sys
cargo build --offline --locked --release --features accelerate --example validate
cargo clippy --offline --locked --features accelerate --example validate -- -D warnings
/usr/bin/time -l target/release/examples/validate storage-audit artifacts/goal1-small-v12-field-train/final
/usr/bin/time -l target/release/examples/validate checkpoint-load-audit artifacts/goal1-small-v12-field-train/final inference
/usr/bin/time -l target/release/examples/validate checkpoint-load-audit artifacts/goal1-small-v12-field-train/final resume
/usr/bin/time -l target/release/examples/validate measure
```

## 실제 호출 및 native 의존

`candle-core`는 자체 Transformer의 Tensor/Var/autograd/CPU matmul에 연결된다.
`candle-nn`은 softmax/log_softmax에 쓰인다. AdamW는 `src/training.rs::Adam::step`에서
직접 구현하며 candle-nn optimizer는 호출하지 않는다. Candle 전체 교체는 이번 범위가 아니다.

`tokenizers`는 자체 corpus의 BPE 학습뿐 아니라 ByteBpe::encode의 runtime tokenize에도
쓰인다. 자체 vocab이라는 이유로 제거할 수 없다. 직접 선언은 default-features=false,
fancy-regex이지만 candle-core 경유 `onig` feature가 합산된다. 실제 활성 graph에는
onig 6.5.3 / onig_sys 69.9.3 C 라이브러리도 포함된다.

현재 direct dependencies에 candle-transformers/minijinja는 없다. 이 제거는 이번 P0
이전의 보존된 S4 WIP에 이미 있었으며 새 제거 성과로 계산하지 않는다. 모델 경로는
`LocalModel::generate → worker → checkpoint::load → Transformer::generate`이다.

SQLite는 bundled libsqlite3-sys 0.35.0의 C 구현이다. zstd-sys 2.1.0+zstd.1.5.7도 C
구현이며 canonical event body 단위 압축에 사용된다. Accelerate.framework는 Apple
시스템 BLAS이다. 현재 candle-core CPU F32 matmul의 feature 분기가 Accelerate sgemm을
호출한다. 일반 GEMM/Rayon crate도 graph에 남아 있다. 모든 GEMM crate가 이번 측정에서
실행됐다는 주장은 하지 않는다. Rayon은 Rust 스레드 풀과 OS 스레드 기능을 이용한다.
Metal/CUDA를 실제 사용했다는 증거는 없다.

동적 linkage의 libSystem, libobjc, Foundation, Accelerate, libiconv를 확인했다.
정적으로 링크된 SQLite/zstd/onig는 `otool -L`에 나타나지 않을 수 있어 graph와 원본
build 설정도 함께 조사했다. crc32c는 손상 검사, SHA-256은 내용 식별/손상 검사이며
서명이나 인증을 제공하지 않는다.

## JSON의 용도별 현황

용량·204개 tensor 표는 보존한 P0 원본의 측정이다. 아래 판정은 P3/P6 이후 기본 경로까지
반영한다. 의존 추가/제거는 없으며 P0의 accelerate feature 조사와 최종 기본 CPU/gemm
실행은 구분한다. 새 binary 및 archive의 실측 수치는 EXPERIMENT_STATUS.md에 있다.

| 경로 | 실제 JSON 용도 | 판정 |
|---|---|---|
| src/neural/checkpoint.rs | 명시 legacy importer/audit의 manifest/config/state/safetensors header | 기본 save/load는 neural/artifact.rs의 native binary; JSON fallback 없음 |
| src/neural.rs | 기존 tokenizer 학습/export 도구 및 legacy import JSON | native는 raw byte vocab/ordered merges와 직접 BPE builder; load 시 JSON 생성 없음 |
| src/neural/transformer.rs::Config::id | 보존한 legacy wire hash/호환 진단 | 기본 model/cache/artifact는 별도 canonical semantic ID; migration identity 보존 |
| src/model.rs | worker IPC, request digest | artifact JSON 제거와 별개; 현재 필요 |
| src/data.rs | 합성 corpus와 split manifest | 개발/학습 자료; 개인 기억 canonical 저장과 다름 |
| src/training.rs, src/train_main.rs | 평가 로그, checkpoint 검사/설정 출력 | 보고/개발용; 모델 추론 산출물과 분리 |
| src/event.rs | IPC용 serde derive | SQLite records.body는 JSON이 아닌 기존 canonical binary |
| Cargo metadata | 도구의 의존 graph 출력 | 감사용 도구 출력; 제품 artifact 아님 |

## 모델 파일 실제 계상

조사 원본: `artifacts/goal1-small-v12-field-train/final`.
weights SHA-256: `aed892878e3f974ca8989a73bac4e2c72ff3a6a4a71f708f94a6e35a4a5b789c`.
학습 step 21,750의 보조 진단 모델이다. 서비스 품질 합격 모델로 취급하지 않는다.

| 구성 | bytes | 근거 |
|---|---:|---|
| MODEL F32, 68 tensors, 9,605,184 scalars | 38,420,736 | tensor별 실제 shape/data 길이 합 |
| MOMENT1 F32, 68 tensors | 38,420,736 | adam.m prefix 실제 tensor |
| MOMENT2 F32, 68 tensors | 38,420,736 | adam.v prefix 실제 tensor |
| safetensors JSON header | 18,304 | 첫 8바이트 little-endian 길이 직접 파싱 |
| header 길이 필드 | 8 | 파일 layout |
| weights.safetensors 전체 | 115,280,520 | filesystem 및 파싱 합계 일치 |
| manifest.json | 13,828 | 실제 파일 길이 |
| tokenizer.json | 22,483 | 실제 파일 길이 |
| 현재 세 파일 합계 | 115,316,831 | optimizer 포함; 추론 전용 크기 아님 |

tied embedding은 model.embedding 하나가 입력 lookup과 출력 projection에 공유된다.
별도 model.output tensor는 없다. Adam의 embedding moment 두 개는 학습 상태이므로
가중치 중복과 혼동하지 않는다. JSON 두 파일 36,311 B와 optimizer 76,841,472 B를
분리해서 보고한다. 현재 새 inference-only 파일은 아직 구현 전이다.

읽기/복사 경로: metadata/tokenizer 읽기 → weights 전체 bounded read → 전체 SHA-256 →
모든 model/Adam shape/dtype/finite 검사 → 필요한 tensor별 F32 Vec 변환 → Tensor/Var.
추론은 Adam Tensor를 할당하지 않지만 Adam bytes의 읽기/hash/검사는 수행한다.
저장은 모든 tensor 직렬화 → fsync → weights 전체 readback → hash/검사 → manifest
publication → 디렉터리 sync이다. 각 할당의 allocator 내부 복사 비용은 UNKNOWN이다.

| 실제 작업 | 관측 시간 | 실제 읽기/할당 범위 | peak RSS |
|---|---:|---|---:|
| weights 단독 read | 34.907 ms | 115,280,520 B | 별도 미측정 |
| 읽은 weights SHA-256 | 211.337 ms | 같은 buffer 전체 scan | 별도 미측정 |
| tensor 파싱/유한수 검사/표 출력 | 9.116 ms | 204 tensors | 별도 미측정 |
| 새 프로세스 legacy inference load | 385.352 ms | weights 전체 read; model Tensor 38,420,736 B; Adam Tensor 0 B | 203,505,664 B |
| 새 프로세스 legacy resume load | 393.404 ms | weights 전체 read; model+Adam Tensor 115,262,208 B | 281,411,584 B |
| 기존 save 경로, 임시 신규 목적지 | 515.490 ms | 직렬화/readback/hash/validation/sync 포함 | 전체 감사 프로세스 515,719,168 B |

load 시간에는 준비/검사/할당이 포함된다. 구성요소 합으로 load 시간을 재구성하지
않는다. save 내부 항목별 시간은 UNKNOWN이며 위 read/hash 측정은 독립 작업이다.
RSS는 macOS `/usr/bin/time -l`의 maximum resident set size 바이트 값이다.

## SQLite 실제 계상

`Store::init`로 새 임시 DB를 두 개 만들었다. 같은 반복 한국어 observation 10,000개와
durable append 측정용 20개, 총 10,020개를 raw/Auto zstd에 각각 저장했다.
FULL/WAL/fullfsync를 유지했다. 이는 P0 저장 조사용이며 P5의 혼합 10,000사건 graph
fixture 완료로 계산하지 않는다. 운영 DB를 VACUUM/reindex하지 않았다.

schema의 FTS는 `fts5(text, tokenize='trigram')`이고 `Store::project`가 payload text를
다시 INSERT한다. contentless는 별도 migration, reindex 및 MATCH/rank 의미 검증 없이
채택하지 않는다. 이번에는 기존 실제 형식을 측정했으며 contentless 성능은 NOT_RUN이다.

| 항목 | raw bytes | Auto zstd bytes |
|---|---:|---:|
| original payload | 4,839,200 | 4,839,200 |
| encoded records.body 합 | 5,482,600 | 1,482,600 |
| FTS에 재저장된 text bytes | 4,839,200 | 4,839,200 |
| records table pages | 5,869,568 | 1,638,400 |
| record_meta pages | 512,000 | 512,000 |
| meta_scope_time / meta_slot | 311,296 / 131,072 | 311,296 / 131,072 |
| FTS content / data | 5,140,480 / 7,356,416 | 5,140,480 / 7,356,416 |
| FTS docsize / idx / config | 106,496 / 4,096 / 4,096 | 106,496 / 4,096 / 4,096 |
| FTS 전체 pages(DB에 이미 포함) | 12,611,584 | 12,611,584 |
| freelist 308 × 4,096 | 1,261,568 | 1,261,568 |
| main DB 파일 | 20,746,240 | 16,515,072 |
| 열린 WAL / SHM | 16,558,312 / 32,768 | 15,223,432 / 32,768 |
| 열린 DB+WAL+SHM | 37,337,320 | 31,771,272 |
| 모든 연결 닫은 DB / WAL / SHM | 20,746,240 / 0 / 0 | 16,515,072 / 0 / 0 |
| 별도 backup | 20,746,240 | 16,515,072 |

나머지 dbstat 객체는 각각 4,096 B: current_heads, edges_from, edges_to,
projection_state, relations, request_state, results, sqlite_autoindex_current_heads_1,
sqlite_autoindex_relations_1, sqlite_autoindex_request_state_1,
sqlite_autoindex_results_1, sqlite_schema. 모든 dbstat pages와 freelist 합이 main DB
길이에 일치한다. backup은 별도 파일이므로 DB 내부 FTS처럼 다시 더하지 않는다.
archive 크기는 아직 NOT_IMPLEMENTED이다.

| 실행(반복 수) | raw | Auto zstd |
|---|---:|---:|
| 10,000건 batch import | 265.282 ms | 309.178 ms |
| 단건 FULL commit median(20) | 3.9686 ms | 3.9673 ms |
| warm get median(128) | 0.0057 ms | 0.0062 ms |
| warm lexical median(100) | 7.1896 ms | 7.5530 ms |
| reindex | 260.473 ms | 280.855 ms |
| reopen+first get, OS cache 유지 | 82.608 ms | 95.395 ms |

감사 전체 peak RSS 98,320,384 B. query는 cap에 따른 truncated를 별도 raw 로그에
기록하며 완전한 전체 검색 성공률로 해석하지 않는다. 이벤트 압축만으로 FTS의
원문/색인 복제 비용이 사라지지 않는다.

## 저장 범주의 분리

A 추론 필수: model tensors, architecture, exact tokenizer, lineage/hash.
B 재개 필수: A + Adam m/v, optimizer/scheduler config, step, input/target counters,
sampler u64, corpus/validation IDs, previous corpora, initial hash, 종료 상태, loss.
현재 저장은 optimizer step 경계이며 부분 누적 gradient 저장은 지원하지 않는다.
C 기억: canonical body, projections/관계/FTS, DB/WAL/SHM, 별도 backup/archive.
D 개발 자료: 시작 시 `du -sk` target 2,827,204 KiB, artifacts 22,752,896 KiB,
docs/logs 46,856 KiB. 이는 filesystem 할당량이며 A/B/C 파일 논리 bytes와 더해서
모델 크기라고 부르지 않는다. 여러 checkpoint 세대/자료/로그가 포함된다.

## 전체 활성 의존 목록

다음 표는 플랫폼 제한 Cargo resolve의 workspace 제외 175개 package이다.
features는 실제 합산값이다. 전이 crate의 사용 위치는 상위 호출 crate로 표시하며
직접 호출이 있다고 주장하지 않는다. build/proc-macro/test 전용 여부는 해당 열에
명시한다. `links`의 rayon-core는 C 라이브러리라는 뜻이 아니다. 비용 UNKNOWN은
개별 기여를 측정하지 않았다는 뜻이다. graph 포함과 실제 커널 실행은 별개이다.

<!-- dependency-table -->
| crate | exact locked version | direct/transitive | 활성 feature | 실제 파일/함수 호출 | 용도 | native 하위 의존 | 디스크/메모리/연산 영향 | 제거·대체 작업 | 현재 결정 | 근거 |
|---|---|---|---|---|---|---|---|---|---|---|
| accelerate-src | 0.3.2 | transitive | 없음 | 직접 호출 없음; candle-core 경유 | 상위 crate의 runtime/train/test/build 경유; 개별 실행 미측정 | Accelerate.framework | 개별 bytes/RSS/시간 UNKNOWN | 상위 crate 교체와 함께 검증 필요 | 유지; 직접 미사용으로 입증되지 않음 | target metadata resolve/accelerate-src; Cargo.lock; 상위 dependency edge |
| ahash | 0.8.12 | transitive | default, getrandom, runtime-rng, serde, std | 직접 호출 없음; tokenizers 경유 | 상위 crate의 runtime/train/test/build 경유; 개별 실행 미측정 | 명시 links 없음; OS/Rust 표준 기능 가능 | 개별 bytes/RSS/시간 UNKNOWN | 상위 crate 교체와 함께 검증 필요 | 유지; 직접 미사용으로 입증되지 않음 | target metadata resolve/ahash; Cargo.lock; 상위 dependency edge |
| aho-corasick | 1.1.5 | transitive | default, perf-literal, std | 직접 호출 없음; regex, regex-automata, tokenizers 경유 | 상위 crate의 runtime/train/test/build 경유; 개별 실행 미측정 | 명시 links 없음; OS/Rust 표준 기능 가능 | 개별 bytes/RSS/시간 UNKNOWN | 상위 crate 교체와 함께 검증 필요 | 유지; 직접 미사용으로 입증되지 않음 | target metadata resolve/aho-corasick; Cargo.lock; 상위 dependency edge |
| allocator-api2 | 0.2.21 | transitive | alloc | 직접 호출 없음; hashbrown 경유 | 상위 crate의 runtime/train/test/build 경유; 개별 실행 미측정 | 명시 links 없음; OS/Rust 표준 기능 가능 | 개별 bytes/RSS/시간 UNKNOWN | 상위 crate 교체와 함께 검증 필요 | 유지; 직접 미사용으로 입증되지 않음 | target metadata resolve/allocator-api2; Cargo.lock; 상위 dependency edge |
| anstream | 1.0.0 | transitive | auto, default, wincon | 직접 호출 없음; clap_builder 경유 | 상위 crate의 runtime/train/test/build 경유; 개별 실행 미측정 | 명시 links 없음; OS/Rust 표준 기능 가능 | 개별 bytes/RSS/시간 UNKNOWN | 상위 crate 교체와 함께 검증 필요 | 유지; 직접 미사용으로 입증되지 않음 | target metadata resolve/anstream; Cargo.lock; 상위 dependency edge |
| anstyle | 1.0.14 | transitive | default, std | 직접 호출 없음; anstream, clap_builder 경유 | 상위 crate의 runtime/train/test/build 경유; 개별 실행 미측정 | 명시 links 없음; OS/Rust 표준 기능 가능 | 개별 bytes/RSS/시간 UNKNOWN | 상위 crate 교체와 함께 검증 필요 | 유지; 직접 미사용으로 입증되지 않음 | target metadata resolve/anstyle; Cargo.lock; 상위 dependency edge |
| anstyle-parse | 1.0.0 | transitive | default, utf8 | 직접 호출 없음; anstream 경유 | 상위 crate의 runtime/train/test/build 경유; 개별 실행 미측정 | 명시 links 없음; OS/Rust 표준 기능 가능 | 개별 bytes/RSS/시간 UNKNOWN | 상위 crate 교체와 함께 검증 필요 | 유지; 직접 미사용으로 입증되지 않음 | target metadata resolve/anstyle-parse; Cargo.lock; 상위 dependency edge |
| anstyle-query | 1.1.5 | transitive | 없음 | 직접 호출 없음; anstream 경유 | 상위 crate의 runtime/train/test/build 경유; 개별 실행 미측정 | 명시 links 없음; OS/Rust 표준 기능 가능 | 개별 bytes/RSS/시간 UNKNOWN | 상위 crate 교체와 함께 검증 필요 | 유지; 직접 미사용으로 입증되지 않음 | target metadata resolve/anstyle-query; Cargo.lock; 상위 dependency edge |
| autocfg | 1.5.1 | transitive | 없음 | 직접 호출 없음; num-traits 경유 | 상위 crate의 runtime/train/test/build 경유; 개별 실행 미측정 | 명시 links 없음; OS/Rust 표준 기능 가능 | 개별 bytes/RSS/시간 UNKNOWN | 상위 crate 교체와 함께 검증 필요 | 유지; 직접 미사용으로 입증되지 않음 | target metadata resolve/autocfg; Cargo.lock; 상위 dependency edge |
| base64 | 0.13.1 | transitive | default, std | 직접 호출 없음; spm_precompiled 경유 | 상위 crate의 runtime/train/test/build 경유; 개별 실행 미측정 | 명시 links 없음; OS/Rust 표준 기능 가능 | 개별 bytes/RSS/시간 UNKNOWN | 상위 crate 교체와 함께 검증 필요 | 유지; 직접 미사용으로 입증되지 않음 | target metadata resolve/base64; Cargo.lock; 상위 dependency edge |
| bit-set | 0.8.0 | transitive | std | 직접 호출 없음; fancy-regex 경유 | 상위 crate의 runtime/train/test/build 경유; 개별 실행 미측정 | 명시 links 없음; OS/Rust 표준 기능 가능 | 개별 bytes/RSS/시간 UNKNOWN | 상위 crate 교체와 함께 검증 필요 | 유지; 직접 미사용으로 입증되지 않음 | target metadata resolve/bit-set; Cargo.lock; 상위 dependency edge |
| bit-vec | 0.8.0 | transitive | std | 직접 호출 없음; bit-set 경유 | 상위 crate의 runtime/train/test/build 경유; 개별 실행 미측정 | 명시 links 없음; OS/Rust 표준 기능 가능 | 개별 bytes/RSS/시간 UNKNOWN | 상위 crate 교체와 함께 검증 필요 | 유지; 직접 미사용으로 입증되지 않음 | target metadata resolve/bit-vec; Cargo.lock; 상위 dependency edge |
| bitflags | 2.13.2 | transitive | std | 직접 호출 없음; dispatch2, nix, onig, raw-cpuid, rusqlite, rustix, sysctl 경유 | 상위 crate의 runtime/train/test/build 경유; 개별 실행 미측정 | 명시 links 없음; OS/Rust 표준 기능 가능 | 개별 bytes/RSS/시간 UNKNOWN | 상위 crate 교체와 함께 검증 필요 | 유지; 직접 미사용으로 입증되지 않음 | target metadata resolve/bitflags; Cargo.lock; 상위 dependency edge |
| block-buffer | 0.10.4 | transitive | 없음 | 직접 호출 없음; digest 경유 | 상위 crate의 runtime/train/test/build 경유; 개별 실행 미측정 | 명시 links 없음; OS/Rust 표준 기능 가능 | 개별 bytes/RSS/시간 UNKNOWN | 상위 crate 교체와 함께 검증 필요 | 유지; 직접 미사용으로 입증되지 않음 | target metadata resolve/block-buffer; Cargo.lock; 상위 dependency edge |
| block2 | 0.6.2 | transitive | alloc | 직접 호출 없음; dispatch2 경유 | 상위 crate의 runtime/train/test/build 경유; 개별 실행 미측정 | 명시 links 없음; OS/Rust 표준 기능 가능 | 개별 bytes/RSS/시간 UNKNOWN | 상위 crate 교체와 함께 검증 필요 | 유지; 직접 미사용으로 입증되지 않음 | target metadata resolve/block2; Cargo.lock; 상위 dependency edge |
| bytemuck | 1.25.2 | transitive | aarch64_simd, bytemuck_derive, derive, wasm_simd | 직접 호출 없음; dyn-stack, gemm-common, half, num-complex, pulp 경유 | 상위 crate의 runtime/train/test/build 경유; 개별 실행 미측정 | 명시 links 없음; OS/Rust 표준 기능 가능 | 개별 bytes/RSS/시간 UNKNOWN | 상위 crate 교체와 함께 검증 필요 | 유지; 직접 미사용으로 입증되지 않음 | target metadata resolve/bytemuck; Cargo.lock; 상위 dependency edge |
| bytemuck_derive | 1.12.1 | transitive | 없음 | 직접 호출 없음; bytemuck 경유 | build proc-macro | 명시 links 없음; OS/Rust 표준 기능 가능 | 개별 bytes/RSS/시간 UNKNOWN | 상위 crate 교체와 함께 검증 필요 | 유지; 직접 미사용으로 입증되지 않음 | target metadata resolve/bytemuck_derive; Cargo.lock; 상위 dependency edge |
| byteorder | 1.5.0 | transitive | default, std | 직접 호출 없음; candle-core, sysctl 경유 | 상위 crate의 runtime/train/test/build 경유; 개별 실행 미측정 | 명시 links 없음; OS/Rust 표준 기능 가능 | 개별 bytes/RSS/시간 UNKNOWN | 상위 crate 교체와 함께 검증 필요 | 유지; 직접 미사용으로 입증되지 않음 | target metadata resolve/byteorder; Cargo.lock; 상위 dependency edge |
| candle-core | 0.11.0 | direct | accelerate, default | src/neural/transformer.rs::forward, Var, backward | runtime/train | Accelerate.framework, Rayon Rust/OS, onig | 개별 bytes/RSS/시간 UNKNOWN | Tensor/autograd 유지; hot op만 경계 추출 | 유지; 직접 미사용으로 입증되지 않음 | target metadata resolve/candle-core; Cargo.lock; 왼쪽 source 호출 |
| candle-nn | 0.11.0 | direct | default | transformer.rs::forward/masked_loss softmax/log_softmax | runtime/train | Accelerate.framework, Rayon Rust/OS, onig | 개별 bytes/RSS/시간 UNKNOWN | Adam은 자체 구현; ops 호출 때문에 유지 | 유지; 직접 미사용으로 입증되지 않음 | target metadata resolve/candle-nn; Cargo.lock; 왼쪽 source 호출 |
| castaway | 0.2.4 | transitive | alloc | 직접 호출 없음; compact_str 경유 | 상위 crate의 runtime/train/test/build 경유; 개별 실행 미측정 | 명시 links 없음; OS/Rust 표준 기능 가능 | 개별 bytes/RSS/시간 UNKNOWN | 상위 crate 교체와 함께 검증 필요 | 유지; 직접 미사용으로 입증되지 않음 | target metadata resolve/castaway; Cargo.lock; 상위 dependency edge |
| cc | 1.4.6 | transitive | parallel | 직접 호출 없음; libsqlite3-sys, onig_sys, zstd-sys 경유 | build 도구 | 명시 links 없음; OS/Rust 표준 기능 가능 | 개별 bytes/RSS/시간 UNKNOWN | 상위 crate 교체와 함께 검증 필요 | 유지; 직접 미사용으로 입증되지 않음 | target metadata resolve/cc; Cargo.lock; 상위 dependency edge |
| cfg-if | 1.0.4 | transitive | 없음 | 직접 호출 없음; ahash, compact_str, crc32fast, getrandom, half, nix, pulp, sha2 경유 | 상위 crate의 runtime/train/test/build 경유; 개별 실행 미측정 | 명시 links 없음; OS/Rust 표준 기능 가능 | 개별 bytes/RSS/시간 UNKNOWN | 상위 crate 교체와 함께 검증 필요 | 유지; 직접 미사용으로 입증되지 않음 | target metadata resolve/cfg-if; Cargo.lock; 상위 dependency edge |
| cfg_aliases | 0.2.2 | transitive | 없음 | 직접 호출 없음; nix 경유 | 상위 crate의 runtime/train/test/build 경유; 개별 실행 미측정 | 명시 links 없음; OS/Rust 표준 기능 가능 | 개별 bytes/RSS/시간 UNKNOWN | 상위 crate 교체와 함께 검증 필요 | 유지; 직접 미사용으로 입증되지 않음 | target metadata resolve/cfg_aliases; Cargo.lock; 상위 dependency edge |
| clap | 4.6.7 | direct | color, default, derive, error-context, help, std, suggestions, usage | main.rs/train_main.rs::parse; event.rs 인자 | runtime/train | 명시 links 없음; OS/Rust 표준 기능 가능 | 개별 bytes/RSS/시간 UNKNOWN | CLI 유지 | 유지; 직접 미사용으로 입증되지 않음 | target metadata resolve/clap; Cargo.lock; 왼쪽 source 호출 |
| clap_builder | 4.6.7 | transitive | color, error-context, help, std, suggestions, usage | 직접 호출 없음; clap 경유 | 상위 crate의 runtime/train/test/build 경유; 개별 실행 미측정 | 명시 links 없음; OS/Rust 표준 기능 가능 | 개별 bytes/RSS/시간 UNKNOWN | 상위 crate 교체와 함께 검증 필요 | 유지; 직접 미사용으로 입증되지 않음 | target metadata resolve/clap_builder; Cargo.lock; 상위 dependency edge |
| clap_derive | 4.6.7 | transitive | default | 직접 호출 없음; clap 경유 | build proc-macro | 명시 links 없음; OS/Rust 표준 기능 가능 | 개별 bytes/RSS/시간 UNKNOWN | 상위 crate 교체와 함께 검증 필요 | 유지; 직접 미사용으로 입증되지 않음 | target metadata resolve/clap_derive; Cargo.lock; 상위 dependency edge |
| clap_lex | 1.1.1 | transitive | 없음 | 직접 호출 없음; clap_builder 경유 | 상위 crate의 runtime/train/test/build 경유; 개별 실행 미측정 | 명시 links 없음; OS/Rust 표준 기능 가능 | 개별 bytes/RSS/시간 UNKNOWN | 상위 crate 교체와 함께 검증 필요 | 유지; 직접 미사용으로 입증되지 않음 | target metadata resolve/clap_lex; Cargo.lock; 상위 dependency edge |
| colorchoice | 1.0.5 | transitive | 없음 | 직접 호출 없음; anstream 경유 | 상위 crate의 runtime/train/test/build 경유; 개별 실행 미측정 | 명시 links 없음; OS/Rust 표준 기능 가능 | 개별 bytes/RSS/시간 UNKNOWN | 상위 crate 교체와 함께 검증 필요 | 유지; 직접 미사용으로 입증되지 않음 | target metadata resolve/colorchoice; Cargo.lock; 상위 dependency edge |
| compact_str | 0.9.1 | transitive | default, serde, std | 직접 호출 없음; tokenizers 경유 | 상위 crate의 runtime/train/test/build 경유; 개별 실행 미측정 | 명시 links 없음; OS/Rust 표준 기능 가능 | 개별 bytes/RSS/시간 UNKNOWN | 상위 crate 교체와 함께 검증 필요 | 유지; 직접 미사용으로 입증되지 않음 | target metadata resolve/compact_str; Cargo.lock; 상위 dependency edge |
| cpufeatures | 0.2.17 | transitive | 없음 | 직접 호출 없음; sha2 경유 | 상위 crate의 runtime/train/test/build 경유; 개별 실행 미측정 | 명시 links 없음; OS/Rust 표준 기능 가능 | 개별 bytes/RSS/시간 UNKNOWN | 상위 crate 교체와 함께 검증 필요 | 유지; 직접 미사용으로 입증되지 않음 | target metadata resolve/cpufeatures; Cargo.lock; 상위 dependency edge |
| crc32c | 0.6.8 | direct | 없음 | codec.rs::encode/decode | runtime | 명시 links 없음; OS/Rust 표준 기능 가능 | 개별 bytes/RSS/시간 UNKNOWN | 손상 검사 유지; 인증 아님 | 유지; 직접 미사용으로 입증되지 않음 | target metadata resolve/crc32c; Cargo.lock; 왼쪽 source 호출 |
| crc32fast | 1.5.2 | transitive | default, std | 직접 호출 없음; zip 경유 | 상위 crate의 runtime/train/test/build 경유; 개별 실행 미측정 | 명시 links 없음; OS/Rust 표준 기능 가능 | 개별 bytes/RSS/시간 UNKNOWN | 상위 crate 교체와 함께 검증 필요 | 유지; 직접 미사용으로 입증되지 않음 | target metadata resolve/crc32fast; Cargo.lock; 상위 dependency edge |
| crossbeam-deque | 0.8.8 | transitive | default, std | 직접 호출 없음; rayon-core 경유 | 상위 crate의 runtime/train/test/build 경유; 개별 실행 미측정 | 명시 links 없음; OS/Rust 표준 기능 가능 | 개별 bytes/RSS/시간 UNKNOWN | 상위 crate 교체와 함께 검증 필요 | 유지; 직접 미사용으로 입증되지 않음 | target metadata resolve/crossbeam-deque; Cargo.lock; 상위 dependency edge |
| crossbeam-epoch | 0.9.21 | transitive | alloc, std | 직접 호출 없음; crossbeam-deque 경유 | 상위 crate의 runtime/train/test/build 경유; 개별 실행 미측정 | 명시 links 없음; OS/Rust 표준 기능 가능 | 개별 bytes/RSS/시간 UNKNOWN | 상위 crate 교체와 함께 검증 필요 | 유지; 직접 미사용으로 입증되지 않음 | target metadata resolve/crossbeam-epoch; Cargo.lock; 상위 dependency edge |
| crossbeam-utils | 0.8.23 | transitive | default, std | 직접 호출 없음; crossbeam-deque, crossbeam-epoch, rayon-core 경유 | 상위 crate의 runtime/train/test/build 경유; 개별 실행 미측정 | 명시 links 없음; OS/Rust 표준 기능 가능 | 개별 bytes/RSS/시간 UNKNOWN | 상위 crate 교체와 함께 검증 필요 | 유지; 직접 미사용으로 입증되지 않음 | target metadata resolve/crossbeam-utils; Cargo.lock; 상위 dependency edge |
| crypto-common | 0.1.7 | transitive | std | 직접 호출 없음; digest 경유 | 상위 crate의 runtime/train/test/build 경유; 개별 실행 미측정 | 명시 links 없음; OS/Rust 표준 기능 가능 | 개별 bytes/RSS/시간 UNKNOWN | 상위 crate 교체와 함께 검증 필요 | 유지; 직접 미사용으로 입증되지 않음 | target metadata resolve/crypto-common; Cargo.lock; 상위 dependency edge |
| ctrlc | 3.5.2 | direct | 없음 | main.rs::main, train_main.rs train handler | runtime/train | 명시 links 없음; OS/Rust 표준 기능 가능 | 개별 bytes/RSS/시간 UNKNOWN | OS signal 취소 유지 | 유지; 직접 미사용으로 입증되지 않음 | target metadata resolve/ctrlc; Cargo.lock; 왼쪽 source 호출 |
| darling | 0.20.11 | transitive | default, suggestions | 직접 호출 없음; derive_builder_core 경유 | 상위 crate의 runtime/train/test/build 경유; 개별 실행 미측정 | 명시 links 없음; OS/Rust 표준 기능 가능 | 개별 bytes/RSS/시간 UNKNOWN | 상위 crate 교체와 함께 검증 필요 | 유지; 직접 미사용으로 입증되지 않음 | target metadata resolve/darling; Cargo.lock; 상위 dependency edge |
| darling_core | 0.20.11 | transitive | strsim, suggestions | 직접 호출 없음; darling, darling_macro 경유 | 상위 crate의 runtime/train/test/build 경유; 개별 실행 미측정 | 명시 links 없음; OS/Rust 표준 기능 가능 | 개별 bytes/RSS/시간 UNKNOWN | 상위 crate 교체와 함께 검증 필요 | 유지; 직접 미사용으로 입증되지 않음 | target metadata resolve/darling_core; Cargo.lock; 상위 dependency edge |
| darling_macro | 0.20.11 | transitive | 없음 | 직접 호출 없음; darling 경유 | build proc-macro | 명시 links 없음; OS/Rust 표준 기능 가능 | 개별 bytes/RSS/시간 UNKNOWN | 상위 crate 교체와 함께 검증 필요 | 유지; 직접 미사용으로 입증되지 않음 | target metadata resolve/darling_macro; Cargo.lock; 상위 dependency edge |
| dary_heap | 0.3.9 | transitive | serde | 직접 호출 없음; tokenizers 경유 | 상위 crate의 runtime/train/test/build 경유; 개별 실행 미측정 | 명시 links 없음; OS/Rust 표준 기능 가능 | 개별 bytes/RSS/시간 UNKNOWN | 상위 crate 교체와 함께 검증 필요 | 유지; 직접 미사용으로 입증되지 않음 | target metadata resolve/dary_heap; Cargo.lock; 상위 dependency edge |
| derive_builder | 0.20.2 | transitive | default, std | 직접 호출 없음; tokenizers 경유 | 상위 crate의 runtime/train/test/build 경유; 개별 실행 미측정 | 명시 links 없음; OS/Rust 표준 기능 가능 | 개별 bytes/RSS/시간 UNKNOWN | 상위 crate 교체와 함께 검증 필요 | 유지; 직접 미사용으로 입증되지 않음 | target metadata resolve/derive_builder; Cargo.lock; 상위 dependency edge |
| derive_builder_core | 0.20.2 | transitive | lib_has_std | 직접 호출 없음; derive_builder_macro 경유 | 상위 crate의 runtime/train/test/build 경유; 개별 실행 미측정 | 명시 links 없음; OS/Rust 표준 기능 가능 | 개별 bytes/RSS/시간 UNKNOWN | 상위 crate 교체와 함께 검증 필요 | 유지; 직접 미사용으로 입증되지 않음 | target metadata resolve/derive_builder_core; Cargo.lock; 상위 dependency edge |
| derive_builder_macro | 0.20.2 | transitive | lib_has_std | 직접 호출 없음; derive_builder 경유 | build proc-macro | 명시 links 없음; OS/Rust 표준 기능 가능 | 개별 bytes/RSS/시간 UNKNOWN | 상위 crate 교체와 함께 검증 필요 | 유지; 직접 미사용으로 입증되지 않음 | target metadata resolve/derive_builder_macro; Cargo.lock; 상위 dependency edge |
| digest | 0.10.7 | transitive | alloc, block-buffer, core-api, default, std | 직접 호출 없음; sha2 경유 | 상위 crate의 runtime/train/test/build 경유; 개별 실행 미측정 | 명시 links 없음; OS/Rust 표준 기능 가능 | 개별 bytes/RSS/시간 UNKNOWN | 상위 crate 교체와 함께 검증 필요 | 유지; 직접 미사용으로 입증되지 않음 | target metadata resolve/digest; Cargo.lock; 상위 dependency edge |
| dispatch2 | 0.3.1 | transitive | alloc, block2, default, libc, objc2, std | 직접 호출 없음; ctrlc 경유 | 상위 crate의 runtime/train/test/build 경유; 개별 실행 미측정 | 명시 links 없음; OS/Rust 표준 기능 가능 | 개별 bytes/RSS/시간 UNKNOWN | 상위 crate 교체와 함께 검증 필요 | 유지; 직접 미사용으로 입증되지 않음 | target metadata resolve/dispatch2; Cargo.lock; 상위 dependency edge |
| dyn-stack | 0.13.2 | transitive | alloc, std | 직접 호출 없음; gemm, gemm-c32, gemm-c64, gemm-common, gemm-f16, gemm-f32, gemm-f64 경유 | 상위 crate의 runtime/train/test/build 경유; 개별 실행 미측정 | 명시 links 없음; OS/Rust 표준 기능 가능 | 개별 bytes/RSS/시간 UNKNOWN | 상위 crate 교체와 함께 검증 필요 | 유지; 직접 미사용으로 입증되지 않음 | target metadata resolve/dyn-stack; Cargo.lock; 상위 dependency edge |
| dyn-stack-macros | 0.1.3 | transitive | 없음 | 직접 호출 없음; dyn-stack 경유 | build proc-macro | 명시 links 없음; OS/Rust 표준 기능 가능 | 개별 bytes/RSS/시간 UNKNOWN | 상위 crate 교체와 함께 검증 필요 | 유지; 직접 미사용으로 입증되지 않음 | target metadata resolve/dyn-stack-macros; Cargo.lock; 상위 dependency edge |
| either | 1.18.0 | transitive | default, std, use_std | 직접 호출 없음; itertools, rayon, rayon-cond 경유 | 상위 crate의 runtime/train/test/build 경유; 개별 실행 미측정 | 명시 links 없음; OS/Rust 표준 기능 가능 | 개별 bytes/RSS/시간 UNKNOWN | 상위 crate 교체와 함께 검증 필요 | 유지; 직접 미사용으로 입증되지 않음 | target metadata resolve/either; Cargo.lock; 상위 dependency edge |
| enum-as-inner | 0.6.1 | transitive | 없음 | 직접 호출 없음; sysctl 경유 | build proc-macro | 명시 links 없음; OS/Rust 표준 기능 가능 | 개별 bytes/RSS/시간 UNKNOWN | 상위 crate 교체와 함께 검증 필요 | 유지; 직접 미사용으로 입증되지 않음 | target metadata resolve/enum-as-inner; Cargo.lock; 상위 dependency edge |
| equivalent | 1.0.2 | transitive | 없음 | 직접 호출 없음; hashbrown, indexmap 경유 | 상위 crate의 runtime/train/test/build 경유; 개별 실행 미측정 | 명시 links 없음; OS/Rust 표준 기능 가능 | 개별 bytes/RSS/시간 UNKNOWN | 상위 crate 교체와 함께 검증 필요 | 유지; 직접 미사용으로 입증되지 않음 | target metadata resolve/equivalent; Cargo.lock; 상위 dependency edge |
| errno | 0.3.14 | transitive | std | 직접 호출 없음; rustix 경유 | 상위 crate의 runtime/train/test/build 경유; 개별 실행 미측정 | 명시 links 없음; OS/Rust 표준 기능 가능 | 개별 bytes/RSS/시간 UNKNOWN | 상위 crate 교체와 함께 검증 필요 | 유지; 직접 미사용으로 입증되지 않음 | target metadata resolve/errno; Cargo.lock; 상위 dependency edge |
| esaxx-rs | 0.1.10 | transitive | 없음 | 직접 호출 없음; tokenizers 경유 | 상위 crate의 runtime/train/test/build 경유; 개별 실행 미측정 | 명시 links 없음; OS/Rust 표준 기능 가능 | 개별 bytes/RSS/시간 UNKNOWN | 상위 crate 교체와 함께 검증 필요 | 유지; 직접 미사용으로 입증되지 않음 | target metadata resolve/esaxx-rs; Cargo.lock; 상위 dependency edge |
| fallible-iterator | 0.3.0 | transitive | alloc, default | 직접 호출 없음; rusqlite 경유 | 상위 crate의 runtime/train/test/build 경유; 개별 실행 미측정 | 명시 links 없음; OS/Rust 표준 기능 가능 | 개별 bytes/RSS/시간 UNKNOWN | 상위 crate 교체와 함께 검증 필요 | 유지; 직접 미사용으로 입증되지 않음 | target metadata resolve/fallible-iterator; Cargo.lock; 상위 dependency edge |
| fallible-streaming-iterator | 0.1.9 | transitive | 없음 | 직접 호출 없음; rusqlite 경유 | 상위 crate의 runtime/train/test/build 경유; 개별 실행 미측정 | 명시 links 없음; OS/Rust 표준 기능 가능 | 개별 bytes/RSS/시간 UNKNOWN | 상위 crate 교체와 함께 검증 필요 | 유지; 직접 미사용으로 입증되지 않음 | target metadata resolve/fallible-streaming-iterator; Cargo.lock; 상위 dependency edge |
| fancy-regex | 0.14.0 | transitive | default, perf, std, unicode | 직접 호출 없음; tokenizers 경유 | 상위 crate의 runtime/train/test/build 경유; 개별 실행 미측정 | 명시 links 없음; OS/Rust 표준 기능 가능 | 개별 bytes/RSS/시간 UNKNOWN | 상위 crate 교체와 함께 검증 필요 | 유지; 직접 미사용으로 입증되지 않음 | target metadata resolve/fancy-regex; Cargo.lock; 상위 dependency edge |
| fastrand | 2.5.0 | transitive | alloc, default, std | 직접 호출 없음; tempfile 경유 | 상위 crate의 runtime/train/test/build 경유; 개별 실행 미측정 | 명시 links 없음; OS/Rust 표준 기능 가능 | 개별 bytes/RSS/시간 UNKNOWN | 상위 crate 교체와 함께 검증 필요 | 유지; 직접 미사용으로 입증되지 않음 | target metadata resolve/fastrand; Cargo.lock; 상위 dependency edge |
| find-msvc-tools | 0.1.12 | transitive | 없음 | 직접 호출 없음; cc 경유 | build 도구 | 명시 links 없음; OS/Rust 표준 기능 가능 | 개별 bytes/RSS/시간 UNKNOWN | 상위 crate 교체와 함께 검증 필요 | 유지; 직접 미사용으로 입증되지 않음 | target metadata resolve/find-msvc-tools; Cargo.lock; 상위 dependency edge |
| float8 | 0.7.0 | transitive | default, num-traits, rand_distr, std | 직접 호출 없음; candle-core 경유 | 상위 crate의 runtime/train/test/build 경유; 개별 실행 미측정 | 명시 links 없음; OS/Rust 표준 기능 가능 | 개별 bytes/RSS/시간 UNKNOWN | 상위 crate 교체와 함께 검증 필요 | 유지; 직접 미사용으로 입증되지 않음 | target metadata resolve/float8; Cargo.lock; 상위 dependency edge |
| fnv | 1.0.7 | transitive | default, std | 직접 호출 없음; darling_core 경유 | 상위 crate의 runtime/train/test/build 경유; 개별 실행 미측정 | 명시 links 없음; OS/Rust 표준 기능 가능 | 개별 bytes/RSS/시간 UNKNOWN | 상위 crate 교체와 함께 검증 필요 | 유지; 직접 미사용으로 입증되지 않음 | target metadata resolve/fnv; Cargo.lock; 상위 dependency edge |
| foldhash | 0.1.5 | transitive | 없음 | 직접 호출 없음; hashbrown 경유 | 상위 crate의 runtime/train/test/build 경유; 개별 실행 미측정 | 명시 links 없음; OS/Rust 표준 기능 가능 | 개별 bytes/RSS/시간 UNKNOWN | 상위 crate 교체와 함께 검증 필요 | 유지; 직접 미사용으로 입증되지 않음 | target metadata resolve/foldhash; Cargo.lock; 상위 dependency edge |
| foldhash | 0.2.0 | transitive | 없음 | 직접 호출 없음; hashbrown 경유 | 상위 crate의 runtime/train/test/build 경유; 개별 실행 미측정 | 명시 links 없음; OS/Rust 표준 기능 가능 | 개별 bytes/RSS/시간 UNKNOWN | 상위 crate 교체와 함께 검증 필요 | 유지; 직접 미사용으로 입증되지 않음 | target metadata resolve/foldhash; Cargo.lock; 상위 dependency edge |
| gemm | 0.19.0 | transitive | default, f16, gemm-f16, rayon, std, wasm-simd128-enable | 직접 호출 없음; candle-core 경유 | 상위 crate의 runtime/train/test/build 경유; 개별 실행 미측정 | Rayon Rust/OS | 개별 bytes/RSS/시간 UNKNOWN | 상위 crate 교체와 함께 검증 필요 | 유지; 직접 미사용으로 입증되지 않음 | target metadata resolve/gemm; Cargo.lock; 상위 dependency edge |
| gemm-c32 | 0.19.0 | transitive | rayon, std | 직접 호출 없음; gemm 경유 | 상위 crate의 runtime/train/test/build 경유; 개별 실행 미측정 | Rayon Rust/OS | 개별 bytes/RSS/시간 UNKNOWN | 상위 crate 교체와 함께 검증 필요 | 유지; 직접 미사용으로 입증되지 않음 | target metadata resolve/gemm-c32; Cargo.lock; 상위 dependency edge |
| gemm-c64 | 0.19.0 | transitive | rayon, std | 직접 호출 없음; gemm 경유 | 상위 crate의 runtime/train/test/build 경유; 개별 실행 미측정 | Rayon Rust/OS | 개별 bytes/RSS/시간 UNKNOWN | 상위 crate 교체와 함께 검증 필요 | 유지; 직접 미사용으로 입증되지 않음 | target metadata resolve/gemm-c64; Cargo.lock; 상위 dependency edge |
| gemm-common | 0.19.0 | transitive | f16, half, rayon, std, sysctl, wasm-simd128-enable | 직접 호출 없음; gemm, gemm-c32, gemm-c64, gemm-f16, gemm-f32, gemm-f64 경유 | 상위 crate의 runtime/train/test/build 경유; 개별 실행 미측정 | Rayon Rust/OS | 개별 bytes/RSS/시간 UNKNOWN | 상위 crate 교체와 함께 검증 필요 | 유지; 직접 미사용으로 입증되지 않음 | target metadata resolve/gemm-common; Cargo.lock; 상위 dependency edge |
| gemm-f16 | 0.19.0 | transitive | rayon, std | 직접 호출 없음; gemm 경유 | 상위 crate의 runtime/train/test/build 경유; 개별 실행 미측정 | Rayon Rust/OS | 개별 bytes/RSS/시간 UNKNOWN | 상위 crate 교체와 함께 검증 필요 | 유지; 직접 미사용으로 입증되지 않음 | target metadata resolve/gemm-f16; Cargo.lock; 상위 dependency edge |
| gemm-f32 | 0.19.0 | transitive | rayon, std | 직접 호출 없음; gemm, gemm-f16 경유 | 상위 crate의 runtime/train/test/build 경유; 개별 실행 미측정 | Rayon Rust/OS | 개별 bytes/RSS/시간 UNKNOWN | 상위 crate 교체와 함께 검증 필요 | 유지; 직접 미사용으로 입증되지 않음 | target metadata resolve/gemm-f32; Cargo.lock; 상위 dependency edge |
| gemm-f64 | 0.19.0 | transitive | rayon, std | 직접 호출 없음; gemm 경유 | 상위 crate의 runtime/train/test/build 경유; 개별 실행 미측정 | Rayon Rust/OS | 개별 bytes/RSS/시간 UNKNOWN | 상위 crate 교체와 함께 검증 필요 | 유지; 직접 미사용으로 입증되지 않음 | target metadata resolve/gemm-f64; Cargo.lock; 상위 dependency edge |
| generic-array | 0.14.7 | transitive | more_lengths | 직접 호출 없음; block-buffer, crypto-common 경유 | 상위 crate의 runtime/train/test/build 경유; 개별 실행 미측정 | 명시 links 없음; OS/Rust 표준 기능 가능 | 개별 bytes/RSS/시간 UNKNOWN | 상위 crate 교체와 함께 검증 필요 | 유지; 직접 미사용으로 입증되지 않음 | target metadata resolve/generic-array; Cargo.lock; 상위 dependency edge |
| getrandom | 0.3.4 | transitive | std | 직접 호출 없음; ahash, rand_core, tokenizers 경유 | 상위 crate의 runtime/train/test/build 경유; 개별 실행 미측정 | 명시 links 없음; OS/Rust 표준 기능 가능 | 개별 bytes/RSS/시간 UNKNOWN | 상위 crate 교체와 함께 검증 필요 | 유지; 직접 미사용으로 입증되지 않음 | target metadata resolve/getrandom; Cargo.lock; 상위 dependency edge |
| getrandom | 0.4.3 | transitive | std | 직접 호출 없음; tempfile 경유 | 상위 crate의 runtime/train/test/build 경유; 개별 실행 미측정 | 명시 links 없음; OS/Rust 표준 기능 가능 | 개별 bytes/RSS/시간 UNKNOWN | 상위 crate 교체와 함께 검증 필요 | 유지; 직접 미사용으로 입증되지 않음 | target metadata resolve/getrandom; Cargo.lock; 상위 dependency edge |
| half | 2.7.1 | transitive | alloc, bytemuck, default, num-traits, rand_distr, std, use-intrinsics, zerocopy | 직접 호출 없음; candle-core, candle-nn, float8, gemm-common, gemm-f16 경유 | 상위 crate의 runtime/train/test/build 경유; 개별 실행 미측정 | 명시 links 없음; OS/Rust 표준 기능 가능 | 개별 bytes/RSS/시간 UNKNOWN | 상위 crate 교체와 함께 검증 필요 | 유지; 직접 미사용으로 입증되지 않음 | target metadata resolve/half; Cargo.lock; 상위 dependency edge |
| hashbrown | 0.15.5 | transitive | default-hasher, inline-more | 직접 호출 없음; hashlink 경유 | 상위 crate의 runtime/train/test/build 경유; 개별 실행 미측정 | 명시 links 없음; OS/Rust 표준 기능 가능 | 개별 bytes/RSS/시간 UNKNOWN | 상위 crate 교체와 함께 검증 필요 | 유지; 직접 미사용으로 입증되지 않음 | target metadata resolve/hashbrown; Cargo.lock; 상위 dependency edge |
| hashbrown | 0.16.1 | transitive | allocator-api2, default, default-hasher, equivalent, inline-more, raw-entry, serde | 직접 호출 없음; safetensors 경유 | 상위 crate의 runtime/train/test/build 경유; 개별 실행 미측정 | 명시 links 없음; OS/Rust 표준 기능 가능 | 개별 bytes/RSS/시간 UNKNOWN | 상위 crate 교체와 함께 검증 필요 | 유지; 직접 미사용으로 입증되지 않음 | target metadata resolve/hashbrown; Cargo.lock; 상위 dependency edge |
| hashbrown | 0.17.1 | transitive | 없음 | 직접 호출 없음; indexmap 경유 | 상위 crate의 runtime/train/test/build 경유; 개별 실행 미측정 | 명시 links 없음; OS/Rust 표준 기능 가능 | 개별 bytes/RSS/시간 UNKNOWN | 상위 crate 교체와 함께 검증 필요 | 유지; 직접 미사용으로 입증되지 않음 | target metadata resolve/hashbrown; Cargo.lock; 상위 dependency edge |
| hashlink | 0.10.0 | transitive | 없음 | 직접 호출 없음; rusqlite 경유 | 상위 crate의 runtime/train/test/build 경유; 개별 실행 미측정 | 명시 links 없음; OS/Rust 표준 기능 가능 | 개별 bytes/RSS/시간 UNKNOWN | 상위 crate 교체와 함께 검증 필요 | 유지; 직접 미사용으로 입증되지 않음 | target metadata resolve/hashlink; Cargo.lock; 상위 dependency edge |
| heck | 0.5.0 | transitive | 없음 | 직접 호출 없음; clap_derive, enum-as-inner 경유 | 상위 crate의 runtime/train/test/build 경유; 개별 실행 미측정 | 명시 links 없음; OS/Rust 표준 기능 가능 | 개별 bytes/RSS/시간 UNKNOWN | 상위 crate 교체와 함께 검증 필요 | 유지; 직접 미사용으로 입증되지 않음 | target metadata resolve/heck; Cargo.lock; 상위 dependency edge |
| ident_case | 1.0.1 | transitive | 없음 | 직접 호출 없음; darling_core 경유 | 상위 crate의 runtime/train/test/build 경유; 개별 실행 미측정 | 명시 links 없음; OS/Rust 표준 기능 가능 | 개별 bytes/RSS/시간 UNKNOWN | 상위 crate 교체와 함께 검증 필요 | 유지; 직접 미사용으로 입증되지 않음 | target metadata resolve/ident_case; Cargo.lock; 상위 dependency edge |
| indexmap | 2.14.2 | transitive | default, std | 직접 호출 없음; zip 경유 | 상위 crate의 runtime/train/test/build 경유; 개별 실행 미측정 | 명시 links 없음; OS/Rust 표준 기능 가능 | 개별 bytes/RSS/시간 UNKNOWN | 상위 crate 교체와 함께 검증 필요 | 유지; 직접 미사용으로 입증되지 않음 | target metadata resolve/indexmap; Cargo.lock; 상위 dependency edge |
| is_terminal_polyfill | 1.70.2 | transitive | default | 직접 호출 없음; anstream 경유 | 상위 crate의 runtime/train/test/build 경유; 개별 실행 미측정 | 명시 links 없음; OS/Rust 표준 기능 가능 | 개별 bytes/RSS/시간 UNKNOWN | 상위 crate 교체와 함께 검증 필요 | 유지; 직접 미사용으로 입증되지 않음 | target metadata resolve/is_terminal_polyfill; Cargo.lock; 상위 dependency edge |
| itertools | 0.14.0 | transitive | default, use_alloc, use_std | 직접 호출 없음; rayon-cond, tokenizers 경유 | 상위 crate의 runtime/train/test/build 경유; 개별 실행 미측정 | 명시 links 없음; OS/Rust 표준 기능 가능 | 개별 bytes/RSS/시간 UNKNOWN | 상위 crate 교체와 함께 검증 필요 | 유지; 직접 미사용으로 입증되지 않음 | target metadata resolve/itertools; Cargo.lock; 상위 dependency edge |
| itoa | 1.0.18 | transitive | 없음 | 직접 호출 없음; compact_str, serde_json 경유 | 상위 crate의 runtime/train/test/build 경유; 개별 실행 미측정 | 명시 links 없음; OS/Rust 표준 기능 가능 | 개별 bytes/RSS/시간 UNKNOWN | 상위 crate 교체와 함께 검증 필요 | 유지; 직접 미사용으로 입증되지 않음 | target metadata resolve/itoa; Cargo.lock; 상위 dependency edge |
| jobserver | 0.1.35 | transitive | 없음 | 직접 호출 없음; cc 경유 | 상위 crate의 runtime/train/test/build 경유; 개별 실행 미측정 | 명시 links 없음; OS/Rust 표준 기능 가능 | 개별 bytes/RSS/시간 UNKNOWN | 상위 crate 교체와 함께 검증 필요 | 유지; 직접 미사용으로 입증되지 않음 | target metadata resolve/jobserver; Cargo.lock; 상위 dependency edge |
| libc | 0.2.189 | transitive | default, extra_traits, std | 직접 호출 없음; candle-core, candle-nn, cc, cpufeatures, dispatch2, errno, getrandom, jobserver, memmap2, nix, num_cpus, rustix, safetensors, sysctl 경유 | 상위 crate의 runtime/train/test/build 경유; 개별 실행 미측정 | OS FFI | 개별 bytes/RSS/시간 UNKNOWN | 상위 crate 교체와 함께 검증 필요 | 유지; 직접 미사용으로 입증되지 않음 | target metadata resolve/libc; Cargo.lock; 상위 dependency edge |
| libm | 0.2.16 | transitive | arch, default | 직접 호출 없음; candle-core, gemm-common, num-traits, pulp 경유 | 상위 crate의 runtime/train/test/build 경유; 개별 실행 미측정 | 명시 links 없음; OS/Rust 표준 기능 가능 | 개별 bytes/RSS/시간 UNKNOWN | 상위 crate 교체와 함께 검증 필요 | 유지; 직접 미사용으로 입증되지 않음 | target metadata resolve/libm; Cargo.lock; 상위 dependency edge |
| libsqlite3-sys | 0.35.0 | transitive | bundled, bundled_bindings, cc, default, min_sqlite_version_3_14_0, pkg-config, vcpkg | 직접 호출 없음; rusqlite 경유 | 상위 crate의 runtime/train/test/build 경유; 개별 실행 미측정 | sqlite3 | 개별 bytes/RSS/시간 UNKNOWN | 상위 crate 교체와 함께 검증 필요 | 유지; 직접 미사용으로 입증되지 않음 | target metadata resolve/libsqlite3-sys; Cargo.lock; 상위 dependency edge |
| log | 0.4.34 | transitive | 없음 | 직접 호출 없음; tokenizers 경유 | 상위 crate의 runtime/train/test/build 경유; 개별 실행 미측정 | 명시 links 없음; OS/Rust 표준 기능 가능 | 개별 bytes/RSS/시간 UNKNOWN | 상위 crate 교체와 함께 검증 필요 | 유지; 직접 미사용으로 입증되지 않음 | target metadata resolve/log; Cargo.lock; 상위 dependency edge |
| macro_rules_attribute | 0.2.3 | transitive | default | 직접 호출 없음; tokenizers 경유 | 상위 crate의 runtime/train/test/build 경유; 개별 실행 미측정 | 명시 links 없음; OS/Rust 표준 기능 가능 | 개별 bytes/RSS/시간 UNKNOWN | 상위 crate 교체와 함께 검증 필요 | 유지; 직접 미사용으로 입증되지 않음 | target metadata resolve/macro_rules_attribute; Cargo.lock; 상위 dependency edge |
| macro_rules_attribute-proc_macro | 0.2.3 | transitive | 없음 | 직접 호출 없음; macro_rules_attribute 경유 | build proc-macro | 명시 links 없음; OS/Rust 표준 기능 가능 | 개별 bytes/RSS/시간 UNKNOWN | 상위 crate 교체와 함께 검증 필요 | 유지; 직접 미사용으로 입증되지 않음 | target metadata resolve/macro_rules_attribute-proc_macro; Cargo.lock; 상위 dependency edge |
| memchr | 2.8.3 | transitive | alloc, default, std | 직접 호출 없음; aho-corasick, nom, regex, regex-automata, serde_json, zip 경유 | 상위 crate의 runtime/train/test/build 경유; 개별 실행 미측정 | 명시 links 없음; OS/Rust 표준 기능 가능 | 개별 bytes/RSS/시간 UNKNOWN | 상위 crate 교체와 함께 검증 필요 | 유지; 직접 미사용으로 입증되지 않음 | target metadata resolve/memchr; Cargo.lock; 상위 dependency edge |
| memmap2 | 0.9.11 | transitive | stable_deref_trait | 직접 호출 없음; candle-core 경유 | 상위 crate의 runtime/train/test/build 경유; 개별 실행 미측정 | 명시 links 없음; OS/Rust 표준 기능 가능 | 개별 bytes/RSS/시간 UNKNOWN | 상위 crate 교체와 함께 검증 필요 | 유지; 직접 미사용으로 입증되지 않음 | target metadata resolve/memmap2; Cargo.lock; 상위 dependency edge |
| minimal-lexical | 0.2.1 | transitive | std | 직접 호출 없음; nom 경유 | 상위 crate의 runtime/train/test/build 경유; 개별 실행 미측정 | 명시 links 없음; OS/Rust 표준 기능 가능 | 개별 bytes/RSS/시간 UNKNOWN | 상위 crate 교체와 함께 검증 필요 | 유지; 직접 미사용으로 입증되지 않음 | target metadata resolve/minimal-lexical; Cargo.lock; 상위 dependency edge |
| monostate | 0.1.18 | transitive | 없음 | 직접 호출 없음; tokenizers 경유 | 상위 crate의 runtime/train/test/build 경유; 개별 실행 미측정 | 명시 links 없음; OS/Rust 표준 기능 가능 | 개별 bytes/RSS/시간 UNKNOWN | 상위 crate 교체와 함께 검증 필요 | 유지; 직접 미사용으로 입증되지 않음 | target metadata resolve/monostate; Cargo.lock; 상위 dependency edge |
| monostate-impl | 0.1.18 | transitive | 없음 | 직접 호출 없음; monostate 경유 | build proc-macro | 명시 links 없음; OS/Rust 표준 기능 가능 | 개별 bytes/RSS/시간 UNKNOWN | 상위 crate 교체와 함께 검증 필요 | 유지; 직접 미사용으로 입증되지 않음 | target metadata resolve/monostate-impl; Cargo.lock; 상위 dependency edge |
| nix | 0.31.3 | transitive | process, signal | 직접 호출 없음; ctrlc 경유 | 상위 crate의 runtime/train/test/build 경유; 개별 실행 미측정 | OS FFI | 개별 bytes/RSS/시간 UNKNOWN | 상위 crate 교체와 함께 검증 필요 | 유지; 직접 미사용으로 입증되지 않음 | target metadata resolve/nix; Cargo.lock; 상위 dependency edge |
| nom | 7.1.3 | transitive | alloc, default, std | 직접 호출 없음; spm_precompiled 경유 | 상위 crate의 runtime/train/test/build 경유; 개별 실행 미측정 | 명시 links 없음; OS/Rust 표준 기능 가능 | 개별 bytes/RSS/시간 UNKNOWN | 상위 crate 교체와 함께 검증 필요 | 유지; 직접 미사용으로 입증되지 않음 | target metadata resolve/nom; Cargo.lock; 상위 dependency edge |
| num-complex | 0.4.6 | transitive | bytemuck | 직접 호출 없음; gemm, gemm-c32, gemm-c64, gemm-common, gemm-f16, gemm-f32, gemm-f64, pulp 경유 | 상위 crate의 runtime/train/test/build 경유; 개별 실행 미측정 | 명시 links 없음; OS/Rust 표준 기능 가능 | 개별 bytes/RSS/시간 UNKNOWN | 상위 crate 교체와 함께 검증 필요 | 유지; 직접 미사용으로 입증되지 않음 | target metadata resolve/num-complex; Cargo.lock; 상위 dependency edge |
| num-traits | 0.2.19 | transitive | default, i128, libm, std | 직접 호출 없음; candle-core, candle-nn, float8, gemm, gemm-c32, gemm-c64, gemm-common, gemm-f16, gemm-f32, gemm-f64, half, num-complex, rand_distr 경유 | 상위 crate의 runtime/train/test/build 경유; 개별 실행 미측정 | 명시 links 없음; OS/Rust 표준 기능 가능 | 개별 bytes/RSS/시간 UNKNOWN | 상위 crate 교체와 함께 검증 필요 | 유지; 직접 미사용으로 입증되지 않음 | target metadata resolve/num-traits; Cargo.lock; 상위 dependency edge |
| num_cpus | 1.17.0 | transitive | 없음 | 직접 호출 없음; candle-core 경유 | 상위 crate의 runtime/train/test/build 경유; 개별 실행 미측정 | 명시 links 없음; OS/Rust 표준 기능 가능 | 개별 bytes/RSS/시간 UNKNOWN | 상위 crate 교체와 함께 검증 필요 | 유지; 직접 미사용으로 입증되지 않음 | target metadata resolve/num_cpus; Cargo.lock; 상위 dependency edge |
| objc2 | 0.6.4 | transitive | alloc, std | 직접 호출 없음; block2, dispatch2 경유 | 상위 crate의 runtime/train/test/build 경유; 개별 실행 미측정 | OS FFI | 개별 bytes/RSS/시간 UNKNOWN | 상위 crate 교체와 함께 검증 필요 | 유지; 직접 미사용으로 입증되지 않음 | target metadata resolve/objc2; Cargo.lock; 상위 dependency edge |
| objc2-encode | 4.1.0 | transitive | alloc, std | 직접 호출 없음; objc2 경유 | 상위 crate의 runtime/train/test/build 경유; 개별 실행 미측정 | OS FFI | 개별 bytes/RSS/시간 UNKNOWN | 상위 crate 교체와 함께 검증 필요 | 유지; 직접 미사용으로 입증되지 않음 | target metadata resolve/objc2-encode; Cargo.lock; 상위 dependency edge |
| once_cell | 1.21.4 | transitive | alloc, default, race, std | 직접 호출 없음; ahash, gemm-common, onig, tempfile 경유 | 상위 crate의 runtime/train/test/build 경유; 개별 실행 미측정 | 명시 links 없음; OS/Rust 표준 기능 가능 | 개별 bytes/RSS/시간 UNKNOWN | 상위 crate 교체와 함께 검증 필요 | 유지; 직접 미사용으로 입증되지 않음 | target metadata resolve/once_cell; Cargo.lock; 상위 dependency edge |
| onig | 6.5.3 | transitive | 없음 | 직접 호출 없음; tokenizers 경유 | 상위 crate의 runtime/train/test/build 경유; 개별 실행 미측정 | onig | 개별 bytes/RSS/시간 UNKNOWN | 상위 crate 교체와 함께 검증 필요 | 유지; 직접 미사용으로 입증되지 않음 | target metadata resolve/onig; Cargo.lock; 상위 dependency edge |
| onig_sys | 69.9.3 | transitive | 없음 | 직접 호출 없음; onig 경유 | 상위 crate의 runtime/train/test/build 경유; 개별 실행 미측정 | onig | 개별 bytes/RSS/시간 UNKNOWN | 상위 crate 교체와 함께 검증 필요 | 유지; 직접 미사용으로 입증되지 않음 | target metadata resolve/onig_sys; Cargo.lock; 상위 dependency edge |
| paste | 1.0.15 | transitive | 없음 | 직접 호출 없음; gemm, gemm-c32, gemm-c64, gemm-common, gemm-f16, gemm-f32, gemm-f64, pulp, tokenizers 경유 | build proc-macro | 명시 links 없음; OS/Rust 표준 기능 가능 | 개별 bytes/RSS/시간 UNKNOWN | 상위 crate 교체와 함께 검증 필요 | 유지; 직접 미사용으로 입증되지 않음 | target metadata resolve/paste; Cargo.lock; 상위 dependency edge |
| pastey | 0.2.3 | transitive | 없음 | 직접 호출 없음; macro_rules_attribute 경유 | build proc-macro | 명시 links 없음; OS/Rust 표준 기능 가능 | 개별 bytes/RSS/시간 UNKNOWN | 상위 crate 교체와 함께 검증 필요 | 유지; 직접 미사용으로 입증되지 않음 | target metadata resolve/pastey; Cargo.lock; 상위 dependency edge |
| pkg-config | 0.3.34 | transitive | 없음 | 직접 호출 없음; libsqlite3-sys, onig_sys, zstd-sys 경유 | build 도구 | 명시 links 없음; OS/Rust 표준 기능 가능 | 개별 bytes/RSS/시간 UNKNOWN | 상위 crate 교체와 함께 검증 필요 | 유지; 직접 미사용으로 입증되지 않음 | target metadata resolve/pkg-config; Cargo.lock; 상위 dependency edge |
| ppv-lite86 | 0.2.21 | transitive | simd, std | 직접 호출 없음; rand_chacha 경유 | 상위 crate의 runtime/train/test/build 경유; 개별 실행 미측정 | 명시 links 없음; OS/Rust 표준 기능 가능 | 개별 bytes/RSS/시간 UNKNOWN | 상위 crate 교체와 함께 검증 필요 | 유지; 직접 미사용으로 입증되지 않음 | target metadata resolve/ppv-lite86; Cargo.lock; 상위 dependency edge |
| proc-macro2 | 1.0.107 | transitive | default, proc-macro | 직접 호출 없음; bytemuck_derive, clap_derive, darling_core, derive_builder_core, enum-as-inner, monostate-impl, quote, serde_derive, syn, synstructure, thiserror-impl, yoke-derive, zerocopy-derive, zerofrom-derive 경유 | 상위 crate의 runtime/train/test/build 경유; 개별 실행 미측정 | 명시 links 없음; OS/Rust 표준 기능 가능 | 개별 bytes/RSS/시간 UNKNOWN | 상위 crate 교체와 함께 검증 필요 | 유지; 직접 미사용으로 입증되지 않음 | target metadata resolve/proc-macro2; Cargo.lock; 상위 dependency edge |
| pulp | 0.22.3 | transitive | std | 직접 호출 없음; gemm-common 경유 | 상위 crate의 runtime/train/test/build 경유; 개별 실행 미측정 | 명시 links 없음; OS/Rust 표준 기능 가능 | 개별 bytes/RSS/시간 UNKNOWN | 상위 crate 교체와 함께 검증 필요 | 유지; 직접 미사용으로 입증되지 않음 | target metadata resolve/pulp; Cargo.lock; 상위 dependency edge |
| pulp-wasm-simd-flag | 0.1.1 | transitive | 없음 | 직접 호출 없음; pulp 경유 | 상위 crate의 runtime/train/test/build 경유; 개별 실행 미측정 | 명시 links 없음; OS/Rust 표준 기능 가능 | 개별 bytes/RSS/시간 UNKNOWN | 상위 crate 교체와 함께 검증 필요 | 유지; 직접 미사용으로 입증되지 않음 | target metadata resolve/pulp-wasm-simd-flag; Cargo.lock; 상위 dependency edge |
| quote | 1.0.47 | transitive | default, proc-macro | 직접 호출 없음; bytemuck_derive, clap_derive, darling_core, darling_macro, derive_builder_core, enum-as-inner, monostate-impl, serde_derive, syn, synstructure, thiserror-impl, yoke-derive, zerocopy-derive, zerofrom-derive 경유 | 상위 crate의 runtime/train/test/build 경유; 개별 실행 미측정 | 명시 links 없음; OS/Rust 표준 기능 가능 | 개별 bytes/RSS/시간 UNKNOWN | 상위 crate 교체와 함께 검증 필요 | 유지; 직접 미사용으로 입증되지 않음 | target metadata resolve/quote; Cargo.lock; 상위 dependency edge |
| rand | 0.9.5 | transitive | alloc, default, os_rng, small_rng, std, std_rng, thread_rng | 직접 호출 없음; candle-core, float8, half, rand_distr, tokenizers 경유 | 상위 crate의 runtime/train/test/build 경유; 개별 실행 미측정 | 명시 links 없음; OS/Rust 표준 기능 가능 | 개별 bytes/RSS/시간 UNKNOWN | 상위 crate 교체와 함께 검증 필요 | 유지; 직접 미사용으로 입증되지 않음 | target metadata resolve/rand; Cargo.lock; 상위 dependency edge |
| rand_chacha | 0.9.0 | transitive | std | 직접 호출 없음; rand 경유 | 상위 crate의 runtime/train/test/build 경유; 개별 실행 미측정 | 명시 links 없음; OS/Rust 표준 기능 가능 | 개별 bytes/RSS/시간 UNKNOWN | 상위 crate 교체와 함께 검증 필요 | 유지; 직접 미사용으로 입증되지 않음 | target metadata resolve/rand_chacha; Cargo.lock; 상위 dependency edge |
| rand_core | 0.9.5 | transitive | os_rng, std | 직접 호출 없음; rand, rand_chacha 경유 | 상위 crate의 runtime/train/test/build 경유; 개별 실행 미측정 | 명시 links 없음; OS/Rust 표준 기능 가능 | 개별 bytes/RSS/시간 UNKNOWN | 상위 crate 교체와 함께 검증 필요 | 유지; 직접 미사용으로 입증되지 않음 | target metadata resolve/rand_core; Cargo.lock; 상위 dependency edge |
| rand_distr | 0.5.1 | transitive | alloc, default, std | 직접 호출 없음; candle-core, float8, half 경유 | 상위 crate의 runtime/train/test/build 경유; 개별 실행 미측정 | 명시 links 없음; OS/Rust 표준 기능 가능 | 개별 bytes/RSS/시간 UNKNOWN | 상위 crate 교체와 함께 검증 필요 | 유지; 직접 미사용으로 입증되지 않음 | target metadata resolve/rand_distr; Cargo.lock; 상위 dependency edge |
| raw-cpuid | 11.6.0 | transitive | 없음 | 직접 호출 없음; gemm, gemm-c32, gemm-c64, gemm-common, gemm-f16, gemm-f32, gemm-f64 경유 | 상위 crate의 runtime/train/test/build 경유; 개별 실행 미측정 | 명시 links 없음; OS/Rust 표준 기능 가능 | 개별 bytes/RSS/시간 UNKNOWN | 상위 crate 교체와 함께 검증 필요 | 유지; 직접 미사용으로 입증되지 않음 | target metadata resolve/raw-cpuid; Cargo.lock; 상위 dependency edge |
| rayon | 1.12.0 | transitive | 없음 | 직접 호출 없음; candle-core, candle-nn, gemm-common, gemm-f16, rayon-cond, tokenizers 경유 | 상위 crate의 runtime/train/test/build 경유; 개별 실행 미측정 | Rayon Rust/OS | 개별 bytes/RSS/시간 UNKNOWN | 상위 crate 교체와 함께 검증 필요 | 유지; 직접 미사용으로 입증되지 않음 | target metadata resolve/rayon; Cargo.lock; 상위 dependency edge |
| rayon-cond | 0.4.0 | transitive | 없음 | 직접 호출 없음; tokenizers 경유 | 상위 crate의 runtime/train/test/build 경유; 개별 실행 미측정 | Rayon Rust/OS | 개별 bytes/RSS/시간 UNKNOWN | 상위 crate 교체와 함께 검증 필요 | 유지; 직접 미사용으로 입증되지 않음 | target metadata resolve/rayon-cond; Cargo.lock; 상위 dependency edge |
| rayon-core | 1.13.0 | transitive | 없음 | 직접 호출 없음; rayon 경유 | 상위 crate의 runtime/train/test/build 경유; 개별 실행 미측정 | Rayon Rust/OS | 개별 bytes/RSS/시간 UNKNOWN | 상위 crate 교체와 함께 검증 필요 | 유지; 직접 미사용으로 입증되지 않음 | target metadata resolve/rayon-core; Cargo.lock; 상위 dependency edge |
| reborrow | 0.5.5 | transitive | default | 직접 호출 없음; pulp 경유 | 상위 crate의 runtime/train/test/build 경유; 개별 실행 미측정 | 명시 links 없음; OS/Rust 표준 기능 가능 | 개별 bytes/RSS/시간 UNKNOWN | 상위 crate 교체와 함께 검증 필요 | 유지; 직접 미사용으로 입증되지 않음 | target metadata resolve/reborrow; Cargo.lock; 상위 dependency edge |
| regex | 1.13.1 | transitive | default, perf, perf-backtrack, perf-cache, perf-dfa, perf-inline, perf-literal, perf-onepass, std, unicode, unicode-age, unicode-bool, unicode-case, unicode-gencat, unicode-perl, unicode-script, unicode-segment | 직접 호출 없음; tokenizers 경유 | 상위 crate의 runtime/train/test/build 경유; 개별 실행 미측정 | 명시 links 없음; OS/Rust 표준 기능 가능 | 개별 bytes/RSS/시간 UNKNOWN | 상위 crate 교체와 함께 검증 필요 | 유지; 직접 미사용으로 입증되지 않음 | target metadata resolve/regex; Cargo.lock; 상위 dependency edge |
| regex-automata | 0.4.18 | transitive | alloc, dfa, dfa-build, dfa-onepass, dfa-search, hybrid, meta, nfa, nfa-backtrack, nfa-pikevm, nfa-thompson, perf, perf-inline, perf-literal, perf-literal-multisubstring, perf-literal-substring, std, syntax, unicode, unicode-age, unicode-bool, unicode-case, unicode-gencat, unicode-perl, unicode-script, unicode-segment, unicode-word-boundary | 직접 호출 없음; fancy-regex, regex 경유 | 상위 crate의 runtime/train/test/build 경유; 개별 실행 미측정 | 명시 links 없음; OS/Rust 표준 기능 가능 | 개별 bytes/RSS/시간 UNKNOWN | 상위 crate 교체와 함께 검증 필요 | 유지; 직접 미사용으로 입증되지 않음 | target metadata resolve/regex-automata; Cargo.lock; 상위 dependency edge |
| regex-syntax | 0.8.11 | transitive | default, std, unicode, unicode-age, unicode-bool, unicode-case, unicode-gencat, unicode-perl, unicode-script, unicode-segment | 직접 호출 없음; fancy-regex, regex, regex-automata, tokenizers 경유 | 상위 crate의 runtime/train/test/build 경유; 개별 실행 미측정 | 명시 links 없음; OS/Rust 표준 기능 가능 | 개별 bytes/RSS/시간 UNKNOWN | 상위 crate 교체와 함께 검증 필요 | 유지; 직접 미사용으로 입증되지 않음 | target metadata resolve/regex-syntax; Cargo.lock; 상위 dependency edge |
| rusqlite | 0.37.0 | direct | backup, bundled, hooks, modern_sqlite | store.rs::init/append/backup; retrieval.rs::search | runtime | sqlite3 | 개별 bytes/RSS/시간 UNKNOWN | live transaction/FTS 유지; archive 별도 | 유지; 직접 미사용으로 입증되지 않음 | target metadata resolve/rusqlite; Cargo.lock; 왼쪽 source 호출 |
| rustc_version | 0.4.1 | transitive | 없음 | 직접 호출 없음; crc32c 경유 | 상위 crate의 runtime/train/test/build 경유; 개별 실행 미측정 | 명시 links 없음; OS/Rust 표준 기능 가능 | 개별 bytes/RSS/시간 UNKNOWN | 상위 crate 교체와 함께 검증 필요 | 유지; 직접 미사용으로 입증되지 않음 | target metadata resolve/rustc_version; Cargo.lock; 상위 dependency edge |
| rustix | 1.1.4 | transitive | alloc, default, fs, std | 직접 호출 없음; tempfile 경유 | 상위 crate의 runtime/train/test/build 경유; 개별 실행 미측정 | 명시 links 없음; OS/Rust 표준 기능 가능 | 개별 bytes/RSS/시간 UNKNOWN | 상위 crate 교체와 함께 검증 필요 | 유지; 직접 미사용으로 입증되지 않음 | target metadata resolve/rustix; Cargo.lock; 상위 dependency edge |
| rustversion | 1.0.23 | transitive | 없음 | 직접 호출 없음; castaway, compact_str 경유 | build proc-macro | 명시 links 없음; OS/Rust 표준 기능 가능 | 개별 bytes/RSS/시간 UNKNOWN | 상위 crate 교체와 함께 검증 필요 | 유지; 직접 미사용으로 입증되지 않음 | target metadata resolve/rustversion; Cargo.lock; 상위 dependency edge |
| ryu | 1.0.23 | transitive | 없음 | 직접 호출 없음; compact_str 경유 | 상위 crate의 runtime/train/test/build 경유; 개별 실행 미측정 | 명시 links 없음; OS/Rust 표준 기능 가능 | 개별 bytes/RSS/시간 UNKNOWN | 상위 crate 교체와 함께 검증 필요 | 유지; 직접 미사용으로 입증되지 않음 | target metadata resolve/ryu; Cargo.lock; 상위 dependency edge |
| safetensors | 0.8.0 | direct | default, std | checkpoint.rs::validate/save/load | runtime/train/legacy | 명시 links 없음; OS/Rust 표준 기능 가능 | 개별 bytes/RSS/시간 UNKNOWN | P3 기본 경로 대체; explicit import에 유지 | 유지; 직접 미사용으로 입증되지 않음 | target metadata resolve/safetensors; Cargo.lock; 왼쪽 source 호출 |
| semver | 1.0.28 | transitive | default, std | 직접 호출 없음; rustc_version 경유 | 상위 crate의 runtime/train/test/build 경유; 개별 실행 미측정 | 명시 links 없음; OS/Rust 표준 기능 가능 | 개별 bytes/RSS/시간 UNKNOWN | 상위 crate 교체와 함께 검증 필요 | 유지; 직접 미사용으로 입증되지 않음 | target metadata resolve/semver; Cargo.lock; 상위 dependency edge |
| seq-macro | 0.3.6 | transitive | 없음 | 직접 호출 없음; gemm, gemm-c32, gemm-c64, gemm-common, gemm-f16, gemm-f32, gemm-f64 경유 | build proc-macro | 명시 links 없음; OS/Rust 표준 기능 가능 | 개별 bytes/RSS/시간 UNKNOWN | 상위 crate 교체와 함께 검증 필요 | 유지; 직접 미사용으로 입증되지 않음 | target metadata resolve/seq-macro; Cargo.lock; 상위 dependency edge |
| serde | 1.0.229 | direct | alloc, default, derive, serde_derive, std | model.rs IPC; checkpoint/data 상태 derive | runtime/train | 명시 links 없음; OS/Rust 표준 기능 가능 | 개별 bytes/RSS/시간 UNKNOWN | artifact와 IPC 분리 후 유지 | 유지; 직접 미사용으로 입증되지 않음 | target metadata resolve/serde; Cargo.lock; 왼쪽 source 호출 |
| serde_core | 1.0.229 | transitive | alloc, result, std | 직접 호출 없음; hashbrown, monostate, serde, serde_json 경유 | 상위 crate의 runtime/train/test/build 경유; 개별 실행 미측정 | 명시 links 없음; OS/Rust 표준 기능 가능 | 개별 bytes/RSS/시간 UNKNOWN | 상위 crate 교체와 함께 검증 필요 | 유지; 직접 미사용으로 입증되지 않음 | target metadata resolve/serde_core; Cargo.lock; 상위 dependency edge |
| serde_derive | 1.0.229 | transitive | default | 직접 호출 없음; serde 경유 | build proc-macro | 명시 links 없음; OS/Rust 표준 기능 가능 | 개별 bytes/RSS/시간 UNKNOWN | 상위 crate 교체와 함께 검증 필요 | 유지; 직접 미사용으로 입증되지 않음 | target metadata resolve/serde_derive; Cargo.lock; 상위 dependency edge |
| serde_json | 1.0.151 | direct | alloc, default, std | model.rs IPC; neural.rs/tokenizer; checkpoint/data/training | runtime/train/legacy | 명시 links 없음; OS/Rust 표준 기능 가능 | 개별 bytes/RSS/시간 UNKNOWN | 새 artifact에서 제외; IPC/corpus에는 유지 | 유지; 직접 미사용으로 입증되지 않음 | target metadata resolve/serde_json; Cargo.lock; 왼쪽 source 호출 |
| sha2 | 0.10.9 | direct | default, std | neural.rs::hash; transformer.rs::weight_hash; model.rs | runtime/train | 명시 links 없음; OS/Rust 표준 기능 가능 | 개별 bytes/RSS/시간 UNKNOWN | 내용 식별 유지; 인증 아님 | 유지; 직접 미사용으로 입증되지 않음 | target metadata resolve/sha2; Cargo.lock; 왼쪽 source 호출 |
| shlex | 2.0.1 | transitive | default, std | 직접 호출 없음; cc 경유 | build 도구 | 명시 links 없음; OS/Rust 표준 기능 가능 | 개별 bytes/RSS/시간 UNKNOWN | 상위 crate 교체와 함께 검증 필요 | 유지; 직접 미사용으로 입증되지 않음 | target metadata resolve/shlex; Cargo.lock; 상위 dependency edge |
| smallvec | 1.16.1 | transitive | 없음 | 직접 호출 없음; rusqlite, unicode-normalization-alignments 경유 | 상위 crate의 runtime/train/test/build 경유; 개별 실행 미측정 | 명시 links 없음; OS/Rust 표준 기능 가능 | 개별 bytes/RSS/시간 UNKNOWN | 상위 crate 교체와 함께 검증 필요 | 유지; 직접 미사용으로 입증되지 않음 | target metadata resolve/smallvec; Cargo.lock; 상위 dependency edge |
| spm_precompiled | 0.1.4 | transitive | 없음 | 직접 호출 없음; tokenizers 경유 | 상위 crate의 runtime/train/test/build 경유; 개별 실행 미측정 | 명시 links 없음; OS/Rust 표준 기능 가능 | 개별 bytes/RSS/시간 UNKNOWN | 상위 crate 교체와 함께 검증 필요 | 유지; 직접 미사용으로 입증되지 않음 | target metadata resolve/spm_precompiled; Cargo.lock; 상위 dependency edge |
| stable_deref_trait | 1.2.1 | transitive | alloc, default, std | 직접 호출 없음; memmap2, yoke 경유 | 상위 crate의 runtime/train/test/build 경유; 개별 실행 미측정 | 명시 links 없음; OS/Rust 표준 기능 가능 | 개별 bytes/RSS/시간 UNKNOWN | 상위 crate 교체와 함께 검증 필요 | 유지; 직접 미사용으로 입증되지 않음 | target metadata resolve/stable_deref_trait; Cargo.lock; 상위 dependency edge |
| static_assertions | 1.1.0 | transitive | 없음 | 직접 호출 없음; compact_str 경유 | 상위 crate의 runtime/train/test/build 경유; 개별 실행 미측정 | 명시 links 없음; OS/Rust 표준 기능 가능 | 개별 bytes/RSS/시간 UNKNOWN | 상위 crate 교체와 함께 검증 필요 | 유지; 직접 미사용으로 입증되지 않음 | target metadata resolve/static_assertions; Cargo.lock; 상위 dependency edge |
| strsim | 0.11.1 | transitive | 없음 | 직접 호출 없음; clap_builder, darling_core 경유 | 상위 crate의 runtime/train/test/build 경유; 개별 실행 미측정 | 명시 links 없음; OS/Rust 표준 기능 가능 | 개별 bytes/RSS/시간 UNKNOWN | 상위 crate 교체와 함께 검증 필요 | 유지; 직접 미사용으로 입증되지 않음 | target metadata resolve/strsim; Cargo.lock; 상위 dependency edge |
| syn | 2.0.119 | transitive | clone-impls, default, derive, extra-traits, full, parsing, printing, proc-macro | 직접 호출 없음; darling_core, darling_macro, derive_builder_core, derive_builder_macro, enum-as-inner, monostate-impl, thiserror-impl, zerocopy-derive 경유 | 상위 crate의 runtime/train/test/build 경유; 개별 실행 미측정 | 명시 links 없음; OS/Rust 표준 기능 가능 | 개별 bytes/RSS/시간 UNKNOWN | 상위 crate 교체와 함께 검증 필요 | 유지; 직접 미사용으로 입증되지 않음 | target metadata resolve/syn; Cargo.lock; 상위 dependency edge |
| syn | 3.0.5 | transitive | clone-impls, default, derive, extra-traits, fold, full, parsing, printing, proc-macro, visit | 직접 호출 없음; bytemuck_derive, clap_derive, serde_derive, synstructure, thiserror-impl, yoke-derive, zerofrom-derive 경유 | 상위 crate의 runtime/train/test/build 경유; 개별 실행 미측정 | 명시 links 없음; OS/Rust 표준 기능 가능 | 개별 bytes/RSS/시간 UNKNOWN | 상위 crate 교체와 함께 검증 필요 | 유지; 직접 미사용으로 입증되지 않음 | target metadata resolve/syn; Cargo.lock; 상위 dependency edge |
| synstructure | 0.14.0 | transitive | default, proc-macro | 직접 호출 없음; yoke-derive, zerofrom-derive 경유 | 상위 crate의 runtime/train/test/build 경유; 개별 실행 미측정 | 명시 links 없음; OS/Rust 표준 기능 가능 | 개별 bytes/RSS/시간 UNKNOWN | 상위 crate 교체와 함께 검증 필요 | 유지; 직접 미사용으로 입증되지 않음 | target metadata resolve/synstructure; Cargo.lock; 상위 dependency edge |
| sysctl | 0.6.0 | transitive | 없음 | 직접 호출 없음; gemm-common 경유 | 상위 crate의 runtime/train/test/build 경유; 개별 실행 미측정 | 명시 links 없음; OS/Rust 표준 기능 가능 | 개별 bytes/RSS/시간 UNKNOWN | 상위 crate 교체와 함께 검증 필요 | 유지; 직접 미사용으로 입증되지 않음 | target metadata resolve/sysctl; Cargo.lock; 상위 dependency edge |
| tempfile | 3.27.0 | direct | default, getrandom | tests 및 examples/validate.rs::measure/storage_audit | test/example | 명시 links 없음; OS/Rust 표준 기능 가능 | 개별 bytes/RSS/시간 UNKNOWN | 소유 임시 파일/DB 유지 | 유지; 직접 미사용으로 입증되지 않음 | target metadata resolve/tempfile; Cargo.lock; 왼쪽 source 호출 |
| thiserror | 1.0.69 | transitive | 없음 | src/lib.rs::Error | runtime/train | 명시 links 없음; OS/Rust 표준 기능 가능 | 개별 bytes/RSS/시간 UNKNOWN | 오류 타입 유지 | 유지; 직접 미사용으로 입증되지 않음 | target metadata resolve/thiserror; Cargo.lock; 왼쪽 source 호출 |
| thiserror | 2.0.20 | direct | default, std | src/lib.rs::Error | runtime/train | 명시 links 없음; OS/Rust 표준 기능 가능 | 개별 bytes/RSS/시간 UNKNOWN | 오류 타입 유지 | 유지; 직접 미사용으로 입증되지 않음 | target metadata resolve/thiserror; Cargo.lock; 왼쪽 source 호출 |
| thiserror-impl | 1.0.69 | transitive | 없음 | 직접 호출 없음; thiserror 경유 | build proc-macro | 명시 links 없음; OS/Rust 표준 기능 가능 | 개별 bytes/RSS/시간 UNKNOWN | 상위 crate 교체와 함께 검증 필요 | 유지; 직접 미사용으로 입증되지 않음 | target metadata resolve/thiserror-impl; Cargo.lock; 상위 dependency edge |
| thiserror-impl | 2.0.20 | transitive | 없음 | 직접 호출 없음; thiserror 경유 | build proc-macro | 명시 links 없음; OS/Rust 표준 기능 가능 | 개별 bytes/RSS/시간 UNKNOWN | 상위 crate 교체와 함께 검증 필요 | 유지; 직접 미사용으로 입증되지 않음 | target metadata resolve/thiserror-impl; Cargo.lock; 상위 dependency edge |
| tokenizers | 0.22.2 | direct | fancy-regex, onig | src/neural.rs::train/from_bytes/encode | runtime/train | Rayon Rust/OS, onig | 개별 bytes/RSS/시간 UNKNOWN | 자체 BPE 학습과 실행; binary builder 전환 | 유지; 직접 미사용으로 입증되지 않음 | target metadata resolve/tokenizers; Cargo.lock; 왼쪽 source 호출 |
| typed-path | 0.12.3 | transitive | default, std | 직접 호출 없음; zip 경유 | 상위 crate의 runtime/train/test/build 경유; 개별 실행 미측정 | 명시 links 없음; OS/Rust 표준 기능 가능 | 개별 bytes/RSS/시간 UNKNOWN | 상위 crate 교체와 함께 검증 필요 | 유지; 직접 미사용으로 입증되지 않음 | target metadata resolve/typed-path; Cargo.lock; 상위 dependency edge |
| typenum | 1.20.1 | transitive | 없음 | 직접 호출 없음; crypto-common, generic-array 경유 | 상위 crate의 runtime/train/test/build 경유; 개별 실행 미측정 | 명시 links 없음; OS/Rust 표준 기능 가능 | 개별 bytes/RSS/시간 UNKNOWN | 상위 crate 교체와 함께 검증 필요 | 유지; 직접 미사용으로 입증되지 않음 | target metadata resolve/typenum; Cargo.lock; 상위 dependency edge |
| unicode-ident | 1.0.24 | transitive | 없음 | 직접 호출 없음; proc-macro2, syn 경유 | 상위 crate의 runtime/train/test/build 경유; 개별 실행 미측정 | 명시 links 없음; OS/Rust 표준 기능 가능 | 개별 bytes/RSS/시간 UNKNOWN | 상위 crate 교체와 함께 검증 필요 | 유지; 직접 미사용으로 입증되지 않음 | target metadata resolve/unicode-ident; Cargo.lock; 상위 dependency edge |
| unicode-normalization-alignments | 0.1.12 | transitive | 없음 | 직접 호출 없음; tokenizers 경유 | 상위 crate의 runtime/train/test/build 경유; 개별 실행 미측정 | 명시 links 없음; OS/Rust 표준 기능 가능 | 개별 bytes/RSS/시간 UNKNOWN | 상위 crate 교체와 함께 검증 필요 | 유지; 직접 미사용으로 입증되지 않음 | target metadata resolve/unicode-normalization-alignments; Cargo.lock; 상위 dependency edge |
| unicode-segmentation | 1.13.3 | transitive | 없음 | 직접 호출 없음; spm_precompiled, tokenizers 경유 | 상위 crate의 runtime/train/test/build 경유; 개별 실행 미측정 | 명시 links 없음; OS/Rust 표준 기능 가능 | 개별 bytes/RSS/시간 UNKNOWN | 상위 crate 교체와 함께 검증 필요 | 유지; 직접 미사용으로 입증되지 않음 | target metadata resolve/unicode-segmentation; Cargo.lock; 상위 dependency edge |
| unicode_categories | 0.1.1 | transitive | 없음 | 직접 호출 없음; tokenizers 경유 | 상위 crate의 runtime/train/test/build 경유; 개별 실행 미측정 | 명시 links 없음; OS/Rust 표준 기능 가능 | 개별 bytes/RSS/시간 UNKNOWN | 상위 crate 교체와 함께 검증 필요 | 유지; 직접 미사용으로 입증되지 않음 | target metadata resolve/unicode_categories; Cargo.lock; 상위 dependency edge |
| utf8parse | 0.2.2 | transitive | default | 직접 호출 없음; anstream, anstyle-parse 경유 | 상위 crate의 runtime/train/test/build 경유; 개별 실행 미측정 | 명시 links 없음; OS/Rust 표준 기능 가능 | 개별 bytes/RSS/시간 UNKNOWN | 상위 crate 교체와 함께 검증 필요 | 유지; 직접 미사용으로 입증되지 않음 | target metadata resolve/utf8parse; Cargo.lock; 상위 dependency edge |
| vcpkg | 0.2.15 | transitive | 없음 | 직접 호출 없음; libsqlite3-sys 경유 | build 도구 | 명시 links 없음; OS/Rust 표준 기능 가능 | 개별 bytes/RSS/시간 UNKNOWN | 상위 crate 교체와 함께 검증 필요 | 유지; 직접 미사용으로 입증되지 않음 | target metadata resolve/vcpkg; Cargo.lock; 상위 dependency edge |
| version_check | 0.9.5 | transitive | 없음 | 직접 호출 없음; ahash, generic-array, pulp 경유 | 상위 crate의 runtime/train/test/build 경유; 개별 실행 미측정 | 명시 links 없음; OS/Rust 표준 기능 가능 | 개별 bytes/RSS/시간 UNKNOWN | 상위 crate 교체와 함께 검증 필요 | 유지; 직접 미사용으로 입증되지 않음 | target metadata resolve/version_check; Cargo.lock; 상위 dependency edge |
| yoke | 0.8.3 | transitive | alloc, default, derive, zerofrom | 직접 호출 없음; candle-core 경유 | 상위 crate의 runtime/train/test/build 경유; 개별 실행 미측정 | 명시 links 없음; OS/Rust 표준 기능 가능 | 개별 bytes/RSS/시간 UNKNOWN | 상위 crate 교체와 함께 검증 필요 | 유지; 직접 미사용으로 입증되지 않음 | target metadata resolve/yoke; Cargo.lock; 상위 dependency edge |
| yoke-derive | 0.8.3 | transitive | 없음 | 직접 호출 없음; yoke 경유 | build proc-macro | 명시 links 없음; OS/Rust 표준 기능 가능 | 개별 bytes/RSS/시간 UNKNOWN | 상위 crate 교체와 함께 검증 필요 | 유지; 직접 미사용으로 입증되지 않음 | target metadata resolve/yoke-derive; Cargo.lock; 상위 dependency edge |
| zerocopy | 0.8.57 | transitive | derive, simd, zerocopy-derive | 직접 호출 없음; ahash, candle-core, half, ppv-lite86 경유 | 상위 crate의 runtime/train/test/build 경유; 개별 실행 미측정 | 명시 links 없음; OS/Rust 표준 기능 가능 | 개별 bytes/RSS/시간 UNKNOWN | 상위 crate 교체와 함께 검증 필요 | 유지; 직접 미사용으로 입증되지 않음 | target metadata resolve/zerocopy; Cargo.lock; 상위 dependency edge |
| zerocopy-derive | 0.8.57 | transitive | 없음 | 직접 호출 없음; zerocopy 경유 | build proc-macro | 명시 links 없음; OS/Rust 표준 기능 가능 | 개별 bytes/RSS/시간 UNKNOWN | 상위 crate 교체와 함께 검증 필요 | 유지; 직접 미사용으로 입증되지 않음 | target metadata resolve/zerocopy-derive; Cargo.lock; 상위 dependency edge |
| zerofrom | 0.1.8 | transitive | alloc, derive | 직접 호출 없음; yoke 경유 | 상위 crate의 runtime/train/test/build 경유; 개별 실행 미측정 | 명시 links 없음; OS/Rust 표준 기능 가능 | 개별 bytes/RSS/시간 UNKNOWN | 상위 crate 교체와 함께 검증 필요 | 유지; 직접 미사용으로 입증되지 않음 | target metadata resolve/zerofrom; Cargo.lock; 상위 dependency edge |
| zerofrom-derive | 0.1.8 | transitive | 없음 | 직접 호출 없음; zerofrom 경유 | build proc-macro | 명시 links 없음; OS/Rust 표준 기능 가능 | 개별 bytes/RSS/시간 UNKNOWN | 상위 crate 교체와 함께 검증 필요 | 유지; 직접 미사용으로 입증되지 않음 | target metadata resolve/zerofrom-derive; Cargo.lock; 상위 dependency edge |
| zip | 8.6.0 | transitive | 없음 | 직접 호출 없음; candle-core 경유 | 상위 crate의 runtime/train/test/build 경유; 개별 실행 미측정 | 명시 links 없음; OS/Rust 표준 기능 가능 | 개별 bytes/RSS/시간 UNKNOWN | 상위 crate 교체와 함께 검증 필요 | 유지; 직접 미사용으로 입증되지 않음 | target metadata resolve/zip; Cargo.lock; 상위 dependency edge |
| zmij | 1.0.23 | transitive | 없음 | 직접 호출 없음; serde_json 경유 | 상위 crate의 runtime/train/test/build 경유; 개별 실행 미측정 | 명시 links 없음; OS/Rust 표준 기능 가능 | 개별 bytes/RSS/시간 UNKNOWN | 상위 crate 교체와 함께 검증 필요 | 유지; 직접 미사용으로 입증되지 않음 | target metadata resolve/zmij; Cargo.lock; 상위 dependency edge |
| zstd | 0.13.3 | direct | arrays, default, legacy, zdict_builder | codec.rs::encode/decode | runtime | zstd | 개별 bytes/RSS/시간 UNKNOWN | event별 제한 압축 유지 | 유지; 직접 미사용으로 입증되지 않음 | target metadata resolve/zstd; Cargo.lock; 왼쪽 source 호출 |
| zstd-safe | 7.3.0 | transitive | arrays, legacy, std, zdict_builder | 직접 호출 없음; zstd 경유 | 상위 crate의 runtime/train/test/build 경유; 개별 실행 미측정 | zstd | 개별 bytes/RSS/시간 UNKNOWN | 상위 crate 교체와 함께 검증 필요 | 유지; 직접 미사용으로 입증되지 않음 | target metadata resolve/zstd-safe; Cargo.lock; 상위 dependency edge |
| zstd-sys | 2.1.0+zstd.1.5.7 | transitive | legacy, std, zdict_builder | 직접 호출 없음; zstd-safe 경유 | 상위 crate의 runtime/train/test/build 경유; 개별 실행 미측정 | zstd | 개별 bytes/RSS/시간 UNKNOWN | 상위 crate 교체와 함께 검증 필요 | 유지; 직접 미사용으로 입증되지 않음 | target metadata resolve/zstd-sys; Cargo.lock; 상위 dependency edge |

## 원본 tensor별 파싱 결과

| tensor | shape | numel | dtype | bytes | 역할 |
|---|---|---:|---|---:|---|
| adam.m.embedding | [801, 384] | 307584 | F32 | 1230336 | MOMENT1 |
| adam.m.final_norm | [384] | 384 | F32 | 1536 | MOMENT1 |
| adam.m.layer.0.attn_norm | [384] | 384 | F32 | 1536 | MOMENT1 |
| adam.m.layer.0.down | [384, 1024] | 393216 | F32 | 1572864 | MOMENT1 |
| adam.m.layer.0.ffn_norm | [384] | 384 | F32 | 1536 | MOMENT1 |
| adam.m.layer.0.gate | [1024, 384] | 393216 | F32 | 1572864 | MOMENT1 |
| adam.m.layer.0.k | [96, 384] | 36864 | F32 | 147456 | MOMENT1 |
| adam.m.layer.0.k_norm | [2, 1, 48] | 96 | F32 | 384 | MOMENT1 |
| adam.m.layer.0.o | [384, 384] | 147456 | F32 | 589824 | MOMENT1 |
| adam.m.layer.0.q | [384, 384] | 147456 | F32 | 589824 | MOMENT1 |
| adam.m.layer.0.q_norm | [8, 1, 48] | 384 | F32 | 1536 | MOMENT1 |
| adam.m.layer.0.up | [1024, 384] | 393216 | F32 | 1572864 | MOMENT1 |
| adam.m.layer.0.v | [96, 384] | 36864 | F32 | 147456 | MOMENT1 |
| adam.m.layer.1.attn_norm | [384] | 384 | F32 | 1536 | MOMENT1 |
| adam.m.layer.1.down | [384, 1024] | 393216 | F32 | 1572864 | MOMENT1 |
| adam.m.layer.1.ffn_norm | [384] | 384 | F32 | 1536 | MOMENT1 |
| adam.m.layer.1.gate | [1024, 384] | 393216 | F32 | 1572864 | MOMENT1 |
| adam.m.layer.1.k | [96, 384] | 36864 | F32 | 147456 | MOMENT1 |
| adam.m.layer.1.k_norm | [2, 1, 48] | 96 | F32 | 384 | MOMENT1 |
| adam.m.layer.1.o | [384, 384] | 147456 | F32 | 589824 | MOMENT1 |
| adam.m.layer.1.q | [384, 384] | 147456 | F32 | 589824 | MOMENT1 |
| adam.m.layer.1.q_norm | [8, 1, 48] | 384 | F32 | 1536 | MOMENT1 |
| adam.m.layer.1.up | [1024, 384] | 393216 | F32 | 1572864 | MOMENT1 |
| adam.m.layer.1.v | [96, 384] | 36864 | F32 | 147456 | MOMENT1 |
| adam.m.layer.2.attn_norm | [384] | 384 | F32 | 1536 | MOMENT1 |
| adam.m.layer.2.down | [384, 1024] | 393216 | F32 | 1572864 | MOMENT1 |
| adam.m.layer.2.ffn_norm | [384] | 384 | F32 | 1536 | MOMENT1 |
| adam.m.layer.2.gate | [1024, 384] | 393216 | F32 | 1572864 | MOMENT1 |
| adam.m.layer.2.k | [96, 384] | 36864 | F32 | 147456 | MOMENT1 |
| adam.m.layer.2.k_norm | [2, 1, 48] | 96 | F32 | 384 | MOMENT1 |
| adam.m.layer.2.o | [384, 384] | 147456 | F32 | 589824 | MOMENT1 |
| adam.m.layer.2.q | [384, 384] | 147456 | F32 | 589824 | MOMENT1 |
| adam.m.layer.2.q_norm | [8, 1, 48] | 384 | F32 | 1536 | MOMENT1 |
| adam.m.layer.2.up | [1024, 384] | 393216 | F32 | 1572864 | MOMENT1 |
| adam.m.layer.2.v | [96, 384] | 36864 | F32 | 147456 | MOMENT1 |
| adam.m.layer.3.attn_norm | [384] | 384 | F32 | 1536 | MOMENT1 |
| adam.m.layer.3.down | [384, 1024] | 393216 | F32 | 1572864 | MOMENT1 |
| adam.m.layer.3.ffn_norm | [384] | 384 | F32 | 1536 | MOMENT1 |
| adam.m.layer.3.gate | [1024, 384] | 393216 | F32 | 1572864 | MOMENT1 |
| adam.m.layer.3.k | [96, 384] | 36864 | F32 | 147456 | MOMENT1 |
| adam.m.layer.3.k_norm | [2, 1, 48] | 96 | F32 | 384 | MOMENT1 |
| adam.m.layer.3.o | [384, 384] | 147456 | F32 | 589824 | MOMENT1 |
| adam.m.layer.3.q | [384, 384] | 147456 | F32 | 589824 | MOMENT1 |
| adam.m.layer.3.q_norm | [8, 1, 48] | 384 | F32 | 1536 | MOMENT1 |
| adam.m.layer.3.up | [1024, 384] | 393216 | F32 | 1572864 | MOMENT1 |
| adam.m.layer.3.v | [96, 384] | 36864 | F32 | 147456 | MOMENT1 |
| adam.m.layer.4.attn_norm | [384] | 384 | F32 | 1536 | MOMENT1 |
| adam.m.layer.4.down | [384, 1024] | 393216 | F32 | 1572864 | MOMENT1 |
| adam.m.layer.4.ffn_norm | [384] | 384 | F32 | 1536 | MOMENT1 |
| adam.m.layer.4.gate | [1024, 384] | 393216 | F32 | 1572864 | MOMENT1 |
| adam.m.layer.4.k | [96, 384] | 36864 | F32 | 147456 | MOMENT1 |
| adam.m.layer.4.k_norm | [2, 1, 48] | 96 | F32 | 384 | MOMENT1 |
| adam.m.layer.4.o | [384, 384] | 147456 | F32 | 589824 | MOMENT1 |
| adam.m.layer.4.q | [384, 384] | 147456 | F32 | 589824 | MOMENT1 |
| adam.m.layer.4.q_norm | [8, 1, 48] | 384 | F32 | 1536 | MOMENT1 |
| adam.m.layer.4.up | [1024, 384] | 393216 | F32 | 1572864 | MOMENT1 |
| adam.m.layer.4.v | [96, 384] | 36864 | F32 | 147456 | MOMENT1 |
| adam.m.layer.5.attn_norm | [384] | 384 | F32 | 1536 | MOMENT1 |
| adam.m.layer.5.down | [384, 1024] | 393216 | F32 | 1572864 | MOMENT1 |
| adam.m.layer.5.ffn_norm | [384] | 384 | F32 | 1536 | MOMENT1 |
| adam.m.layer.5.gate | [1024, 384] | 393216 | F32 | 1572864 | MOMENT1 |
| adam.m.layer.5.k | [96, 384] | 36864 | F32 | 147456 | MOMENT1 |
| adam.m.layer.5.k_norm | [2, 1, 48] | 96 | F32 | 384 | MOMENT1 |
| adam.m.layer.5.o | [384, 384] | 147456 | F32 | 589824 | MOMENT1 |
| adam.m.layer.5.q | [384, 384] | 147456 | F32 | 589824 | MOMENT1 |
| adam.m.layer.5.q_norm | [8, 1, 48] | 384 | F32 | 1536 | MOMENT1 |
| adam.m.layer.5.up | [1024, 384] | 393216 | F32 | 1572864 | MOMENT1 |
| adam.m.layer.5.v | [96, 384] | 36864 | F32 | 147456 | MOMENT1 |
| adam.v.embedding | [801, 384] | 307584 | F32 | 1230336 | MOMENT2 |
| adam.v.final_norm | [384] | 384 | F32 | 1536 | MOMENT2 |
| adam.v.layer.0.attn_norm | [384] | 384 | F32 | 1536 | MOMENT2 |
| adam.v.layer.0.down | [384, 1024] | 393216 | F32 | 1572864 | MOMENT2 |
| adam.v.layer.0.ffn_norm | [384] | 384 | F32 | 1536 | MOMENT2 |
| adam.v.layer.0.gate | [1024, 384] | 393216 | F32 | 1572864 | MOMENT2 |
| adam.v.layer.0.k | [96, 384] | 36864 | F32 | 147456 | MOMENT2 |
| adam.v.layer.0.k_norm | [2, 1, 48] | 96 | F32 | 384 | MOMENT2 |
| adam.v.layer.0.o | [384, 384] | 147456 | F32 | 589824 | MOMENT2 |
| adam.v.layer.0.q | [384, 384] | 147456 | F32 | 589824 | MOMENT2 |
| adam.v.layer.0.q_norm | [8, 1, 48] | 384 | F32 | 1536 | MOMENT2 |
| adam.v.layer.0.up | [1024, 384] | 393216 | F32 | 1572864 | MOMENT2 |
| adam.v.layer.0.v | [96, 384] | 36864 | F32 | 147456 | MOMENT2 |
| adam.v.layer.1.attn_norm | [384] | 384 | F32 | 1536 | MOMENT2 |
| adam.v.layer.1.down | [384, 1024] | 393216 | F32 | 1572864 | MOMENT2 |
| adam.v.layer.1.ffn_norm | [384] | 384 | F32 | 1536 | MOMENT2 |
| adam.v.layer.1.gate | [1024, 384] | 393216 | F32 | 1572864 | MOMENT2 |
| adam.v.layer.1.k | [96, 384] | 36864 | F32 | 147456 | MOMENT2 |
| adam.v.layer.1.k_norm | [2, 1, 48] | 96 | F32 | 384 | MOMENT2 |
| adam.v.layer.1.o | [384, 384] | 147456 | F32 | 589824 | MOMENT2 |
| adam.v.layer.1.q | [384, 384] | 147456 | F32 | 589824 | MOMENT2 |
| adam.v.layer.1.q_norm | [8, 1, 48] | 384 | F32 | 1536 | MOMENT2 |
| adam.v.layer.1.up | [1024, 384] | 393216 | F32 | 1572864 | MOMENT2 |
| adam.v.layer.1.v | [96, 384] | 36864 | F32 | 147456 | MOMENT2 |
| adam.v.layer.2.attn_norm | [384] | 384 | F32 | 1536 | MOMENT2 |
| adam.v.layer.2.down | [384, 1024] | 393216 | F32 | 1572864 | MOMENT2 |
| adam.v.layer.2.ffn_norm | [384] | 384 | F32 | 1536 | MOMENT2 |
| adam.v.layer.2.gate | [1024, 384] | 393216 | F32 | 1572864 | MOMENT2 |
| adam.v.layer.2.k | [96, 384] | 36864 | F32 | 147456 | MOMENT2 |
| adam.v.layer.2.k_norm | [2, 1, 48] | 96 | F32 | 384 | MOMENT2 |
| adam.v.layer.2.o | [384, 384] | 147456 | F32 | 589824 | MOMENT2 |
| adam.v.layer.2.q | [384, 384] | 147456 | F32 | 589824 | MOMENT2 |
| adam.v.layer.2.q_norm | [8, 1, 48] | 384 | F32 | 1536 | MOMENT2 |
| adam.v.layer.2.up | [1024, 384] | 393216 | F32 | 1572864 | MOMENT2 |
| adam.v.layer.2.v | [96, 384] | 36864 | F32 | 147456 | MOMENT2 |
| adam.v.layer.3.attn_norm | [384] | 384 | F32 | 1536 | MOMENT2 |
| adam.v.layer.3.down | [384, 1024] | 393216 | F32 | 1572864 | MOMENT2 |
| adam.v.layer.3.ffn_norm | [384] | 384 | F32 | 1536 | MOMENT2 |
| adam.v.layer.3.gate | [1024, 384] | 393216 | F32 | 1572864 | MOMENT2 |
| adam.v.layer.3.k | [96, 384] | 36864 | F32 | 147456 | MOMENT2 |
| adam.v.layer.3.k_norm | [2, 1, 48] | 96 | F32 | 384 | MOMENT2 |
| adam.v.layer.3.o | [384, 384] | 147456 | F32 | 589824 | MOMENT2 |
| adam.v.layer.3.q | [384, 384] | 147456 | F32 | 589824 | MOMENT2 |
| adam.v.layer.3.q_norm | [8, 1, 48] | 384 | F32 | 1536 | MOMENT2 |
| adam.v.layer.3.up | [1024, 384] | 393216 | F32 | 1572864 | MOMENT2 |
| adam.v.layer.3.v | [96, 384] | 36864 | F32 | 147456 | MOMENT2 |
| adam.v.layer.4.attn_norm | [384] | 384 | F32 | 1536 | MOMENT2 |
| adam.v.layer.4.down | [384, 1024] | 393216 | F32 | 1572864 | MOMENT2 |
| adam.v.layer.4.ffn_norm | [384] | 384 | F32 | 1536 | MOMENT2 |
| adam.v.layer.4.gate | [1024, 384] | 393216 | F32 | 1572864 | MOMENT2 |
| adam.v.layer.4.k | [96, 384] | 36864 | F32 | 147456 | MOMENT2 |
| adam.v.layer.4.k_norm | [2, 1, 48] | 96 | F32 | 384 | MOMENT2 |
| adam.v.layer.4.o | [384, 384] | 147456 | F32 | 589824 | MOMENT2 |
| adam.v.layer.4.q | [384, 384] | 147456 | F32 | 589824 | MOMENT2 |
| adam.v.layer.4.q_norm | [8, 1, 48] | 384 | F32 | 1536 | MOMENT2 |
| adam.v.layer.4.up | [1024, 384] | 393216 | F32 | 1572864 | MOMENT2 |
| adam.v.layer.4.v | [96, 384] | 36864 | F32 | 147456 | MOMENT2 |
| adam.v.layer.5.attn_norm | [384] | 384 | F32 | 1536 | MOMENT2 |
| adam.v.layer.5.down | [384, 1024] | 393216 | F32 | 1572864 | MOMENT2 |
| adam.v.layer.5.ffn_norm | [384] | 384 | F32 | 1536 | MOMENT2 |
| adam.v.layer.5.gate | [1024, 384] | 393216 | F32 | 1572864 | MOMENT2 |
| adam.v.layer.5.k | [96, 384] | 36864 | F32 | 147456 | MOMENT2 |
| adam.v.layer.5.k_norm | [2, 1, 48] | 96 | F32 | 384 | MOMENT2 |
| adam.v.layer.5.o | [384, 384] | 147456 | F32 | 589824 | MOMENT2 |
| adam.v.layer.5.q | [384, 384] | 147456 | F32 | 589824 | MOMENT2 |
| adam.v.layer.5.q_norm | [8, 1, 48] | 384 | F32 | 1536 | MOMENT2 |
| adam.v.layer.5.up | [1024, 384] | 393216 | F32 | 1572864 | MOMENT2 |
| adam.v.layer.5.v | [96, 384] | 36864 | F32 | 147456 | MOMENT2 |
| model.embedding | [801, 384] | 307584 | F32 | 1230336 | MODEL |
| model.final_norm | [384] | 384 | F32 | 1536 | MODEL |
| model.layer.0.attn_norm | [384] | 384 | F32 | 1536 | MODEL |
| model.layer.0.down | [384, 1024] | 393216 | F32 | 1572864 | MODEL |
| model.layer.0.ffn_norm | [384] | 384 | F32 | 1536 | MODEL |
| model.layer.0.gate | [1024, 384] | 393216 | F32 | 1572864 | MODEL |
| model.layer.0.k | [96, 384] | 36864 | F32 | 147456 | MODEL |
| model.layer.0.k_norm | [2, 1, 48] | 96 | F32 | 384 | MODEL |
| model.layer.0.o | [384, 384] | 147456 | F32 | 589824 | MODEL |
| model.layer.0.q | [384, 384] | 147456 | F32 | 589824 | MODEL |
| model.layer.0.q_norm | [8, 1, 48] | 384 | F32 | 1536 | MODEL |
| model.layer.0.up | [1024, 384] | 393216 | F32 | 1572864 | MODEL |
| model.layer.0.v | [96, 384] | 36864 | F32 | 147456 | MODEL |
| model.layer.1.attn_norm | [384] | 384 | F32 | 1536 | MODEL |
| model.layer.1.down | [384, 1024] | 393216 | F32 | 1572864 | MODEL |
| model.layer.1.ffn_norm | [384] | 384 | F32 | 1536 | MODEL |
| model.layer.1.gate | [1024, 384] | 393216 | F32 | 1572864 | MODEL |
| model.layer.1.k | [96, 384] | 36864 | F32 | 147456 | MODEL |
| model.layer.1.k_norm | [2, 1, 48] | 96 | F32 | 384 | MODEL |
| model.layer.1.o | [384, 384] | 147456 | F32 | 589824 | MODEL |
| model.layer.1.q | [384, 384] | 147456 | F32 | 589824 | MODEL |
| model.layer.1.q_norm | [8, 1, 48] | 384 | F32 | 1536 | MODEL |
| model.layer.1.up | [1024, 384] | 393216 | F32 | 1572864 | MODEL |
| model.layer.1.v | [96, 384] | 36864 | F32 | 147456 | MODEL |
| model.layer.2.attn_norm | [384] | 384 | F32 | 1536 | MODEL |
| model.layer.2.down | [384, 1024] | 393216 | F32 | 1572864 | MODEL |
| model.layer.2.ffn_norm | [384] | 384 | F32 | 1536 | MODEL |
| model.layer.2.gate | [1024, 384] | 393216 | F32 | 1572864 | MODEL |
| model.layer.2.k | [96, 384] | 36864 | F32 | 147456 | MODEL |
| model.layer.2.k_norm | [2, 1, 48] | 96 | F32 | 384 | MODEL |
| model.layer.2.o | [384, 384] | 147456 | F32 | 589824 | MODEL |
| model.layer.2.q | [384, 384] | 147456 | F32 | 589824 | MODEL |
| model.layer.2.q_norm | [8, 1, 48] | 384 | F32 | 1536 | MODEL |
| model.layer.2.up | [1024, 384] | 393216 | F32 | 1572864 | MODEL |
| model.layer.2.v | [96, 384] | 36864 | F32 | 147456 | MODEL |
| model.layer.3.attn_norm | [384] | 384 | F32 | 1536 | MODEL |
| model.layer.3.down | [384, 1024] | 393216 | F32 | 1572864 | MODEL |
| model.layer.3.ffn_norm | [384] | 384 | F32 | 1536 | MODEL |
| model.layer.3.gate | [1024, 384] | 393216 | F32 | 1572864 | MODEL |
| model.layer.3.k | [96, 384] | 36864 | F32 | 147456 | MODEL |
| model.layer.3.k_norm | [2, 1, 48] | 96 | F32 | 384 | MODEL |
| model.layer.3.o | [384, 384] | 147456 | F32 | 589824 | MODEL |
| model.layer.3.q | [384, 384] | 147456 | F32 | 589824 | MODEL |
| model.layer.3.q_norm | [8, 1, 48] | 384 | F32 | 1536 | MODEL |
| model.layer.3.up | [1024, 384] | 393216 | F32 | 1572864 | MODEL |
| model.layer.3.v | [96, 384] | 36864 | F32 | 147456 | MODEL |
| model.layer.4.attn_norm | [384] | 384 | F32 | 1536 | MODEL |
| model.layer.4.down | [384, 1024] | 393216 | F32 | 1572864 | MODEL |
| model.layer.4.ffn_norm | [384] | 384 | F32 | 1536 | MODEL |
| model.layer.4.gate | [1024, 384] | 393216 | F32 | 1572864 | MODEL |
| model.layer.4.k | [96, 384] | 36864 | F32 | 147456 | MODEL |
| model.layer.4.k_norm | [2, 1, 48] | 96 | F32 | 384 | MODEL |
| model.layer.4.o | [384, 384] | 147456 | F32 | 589824 | MODEL |
| model.layer.4.q | [384, 384] | 147456 | F32 | 589824 | MODEL |
| model.layer.4.q_norm | [8, 1, 48] | 384 | F32 | 1536 | MODEL |
| model.layer.4.up | [1024, 384] | 393216 | F32 | 1572864 | MODEL |
| model.layer.4.v | [96, 384] | 36864 | F32 | 147456 | MODEL |
| model.layer.5.attn_norm | [384] | 384 | F32 | 1536 | MODEL |
| model.layer.5.down | [384, 1024] | 393216 | F32 | 1572864 | MODEL |
| model.layer.5.ffn_norm | [384] | 384 | F32 | 1536 | MODEL |
| model.layer.5.gate | [1024, 384] | 393216 | F32 | 1572864 | MODEL |
| model.layer.5.k | [96, 384] | 36864 | F32 | 147456 | MODEL |
| model.layer.5.k_norm | [2, 1, 48] | 96 | F32 | 384 | MODEL |
| model.layer.5.o | [384, 384] | 147456 | F32 | 589824 | MODEL |
| model.layer.5.q | [384, 384] | 147456 | F32 | 589824 | MODEL |
| model.layer.5.q_norm | [8, 1, 48] | 384 | F32 | 1536 | MODEL |
| model.layer.5.up | [1024, 384] | 393216 | F32 | 1572864 | MODEL |
| model.layer.5.v | [96, 384] | 36864 | F32 | 147456 | MODEL |
