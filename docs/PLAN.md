# Native Goal 1 work graph

현재 상태: **S4 IMPLEMENTING / renewed authorization**, GOAL1-NATIVE-TRPP-1.0 미완.
2026-09-17 사용자가 학습 제한 갱신과 S4·Goal1까지 계속 진행을 명시했다.
RF-01~03 수정은 검증·공개됐고 원격52e8b88과 일치한다. 먼저 같은 parent/자료/tape의
제한 C/L LR-policy 비교를 완료했다. C는200updates에서watch11/32로 품질 중단,
L은250updates에서15/32로 종료했다. 신규 합계450updates이며 S4/S5/S6는 미통과다.
구체적인 사전 예산과 중단 조건은 QUALITY_RECOVERY_PLAN.md, 실제 실행은
EXPERIMENT_STATUS.md에 유지한다. 아래 제한 진단 종료 기록과 실패는 보존한다.

이전 상태: **Q4 CLOSED_NEGATIVE / NOT_RUNNING**.
R3-S4-QUALITY-RECOVERY-1.0의 제한 진단을 종료했다. 일반 QA parent와 U2+250의
실패를 같은 입력으로 재현했고, decode 오류 시 생성 기록이 사라지는 평가 결함을 고쳤다.
학습 원인은 미확정이다. 같은 parent에서 C50/W50을 실행해 둘 다watch17/32로 종료했다.
50update 안에 원래 붕괴가 재현되지 않아 자동 연장하지 않았다. 기존U2의 남은750updates,
QA32/contrast16 재암기, 자료 확대, S5/S6 확대 작업은 실행하지 않았다.
[제한 진단 계획과 최종 대조](QUALITY_RECOVERY_PLAN.md),
[실제 결과와 판정](EXPERIMENT_STATUS.md)을 참조한다. 아래 P0~P6는 이전 검증 이력이다.
독립 heldout 전체95%/각분류90%/잘못된 인용 승인0 기준은 그대로이며 Goal1 완료가 아니다.

| 현재 node | 상태 | 선행 조건 | 완료 판정 |
|---|---|---|---|
| P0 | VERIFIED / PUBLISHED | 실제 로컬 WIP 보존 | 의존175개·tensor204개·합성 SQLite 실측, 직접 회귀45개/fmt/clippy; ce48514ab5fc8e76a9552ce5fabe7ce1617ff4ac 원격 일치 |
| P1 | VERIFIED; 새값 전이 FAIL | P0 | R-A600/R-B200에서 두 평가 및 fresh reload16/16·4/4; 선동결64 단1회0/64·0/16. R-C/추가 학습 없음 |
| P2 | VERIFIED | P0 | semantic/kernel/content/cache ID, 제한 config, 기본 출력/gradient/resume 보존; native9+resume1 통과 |
| P3 | VERIFIED | P2 | native binary inference/resume, 384입력 무손실 logits/생성, 접근 차단 실제 생성, exact resume 및 kill/retry |
| P4 | VERIFIED / KEEP_REFERENCE | P2 | Rust GEMV 수치 통과, 실제 SMALL decode2.17→4.73ms/전체생성50.05→53.00ms 악화로 기각 |
| P5 | VERIFIED | P2 | 동일10,000건 canonical bytes/ID 및329관계 일치; raw/zstd/SQLite 실측, DB 접근 차단 실제 조회; live SQLite 유지 |
| P6 | VERIFIED / IMPLEMENTER_VERIFIED | P1~P5 | 직접47개, fmt/check/clippy/release; native 실제 생성/16개재검증/exact resume/archive/kernel 실행 및 요구 최종 대조. 독립 검토 미실시 |

`P0 → {P1,P2}; P2 → {P3,P4,P5}; {P1,P2,P3,P4,P5} → P6`.
heavy 작업은 한 번에 하나이고 P1 실행 중 source 편집은 금지한다.
[실험 상태](EXPERIMENT_STATUS.md)에 EXECUTED_COMMANDS/OBSERVED_RESULTS/FILE_HASHES/
LIMITATIONS/NEXT_DEPENDENCY를 기록한다. DELIVERABLE_VERIFIED와 MODEL_QUALITY_PASS,
GOAL1_READY는 별개이며 독립 승인은 INDEPENDENT_PENDING이다.

Contract: [GOAL1-NATIVE-TRPP-1.0](GOAL1_CONTRACT.md).
Baseline: `436ed1d1cdc9efa18c3728bc75fe972b8a5fab14` on `main`.
Historical B0 execution is recorded in IMPLEMENTATION_REPORT.md; it is not current
verification. Source inspection and execution are separate evidence levels.

`S0 → S1 → S2 → S3 → S4 → S5 → S6 → independent review`

| Node | State | Paths / checks / evidence | Remaining exit condition |
|---|---|---|---|
| S0 | VERIFIED | GOAL1_CONTRACT.md, logs/goal1-environment.txt; local instruction bytes unchanged by index-only removal | published 7d831d0; remote matched |
| S1 | VERIFIED | src/{store,app,retrieval,model}.rs; tests/{store,runtime,retrieval,cli}.rs; logs/goal1-s1-tests.txt; logs/goal1-s1-source-digest.txt | published 4edd62c; remote matched |
| S2 | VERIFIED | src/{data,neural,train_main}.rs; tests/{native,training}.rs; logs/goal1-s2-final-tests.txt, goal1-s2-tokenizer.txt; goal1-s2-source-digest.txt | published 6ded741; remote matched |
| S3 | VERIFIED | src/neural/{transformer,checkpoint}.rs, src/training.rs; logs/goal1-s3-exit-tests.txt, goal1-s3-small-boundaries.txt; goal1-s3-source-digest.txt | published 23cc5b0; remote matched |
| S4 | IMPLEMENTING; C/L CLOSED_NEGATIVE | RF-01~03 수정 검증 후 사용자 갱신 예산으로 C200/L250 실행; 동일200update watch11/32 vs14/32, L최종15/32. 원본 checkpoints 보존. | 이전 수준 복구·새 전이 미확정; final200 NOT_RUN_NOT_ELIGIBLE |
| S5 | IMPLEMENTING | src/{model,main,app,retrieval}.rs native-only ask/generate/chat; examples/validate.rs actual CLI smoke; intermediate diagnostic4/14 correct,14/14 same-key no-model replays | prerequisite S4 pending; required five categories and fresh-process quality still FAIL |
| S6 | BLOCKED | quantization not implemented | S4/S5 prerequisites unmet; T-N08 and complete T-I04/T-D02 pending |

| Trace | Required observation | State |
|---|---|---|
| T-R01 | canonical result binding, no model call on corruption | VERIFIED |
| T-R02 | full prompt IDs and actual evidence partition | VERIFIED |
| T-R03 | each search budget plus complete short search | VERIFIED |
| T-R04 | synchronized startup/head commits vs genuine corruption | VERIFIED |
| T-R05 | pinned backup under writes, restored bytes | VERIFIED |
| T-N01 | own tokenizer roundtrip, train-only hash, no truncation | VERIFIED |
| T-N02 | native forward/masks/GQA/RoPE/QK norm/SwiGLU gradients | VERIFIED (CPU numerical/state boundary) |
| T-N03 | bounded prefill/decode cache and reference parity | VERIFIED (CPU numerical/state boundary) |
| T-N04 | actual gradient, weight update, masked loss | VERIFIED (CPU numerical/state boundary) |
| T-N05 | strict checkpoint load and fresh-process resume | VERIFIED (CPU numerical/state boundary) |
| T-N06 | corpus/split/random-init/training lineage | VERIFIED (own corpus/init and actual5000-step run) |
| T-N07 | >=200 heldout generations and category quality | FAILED (200 executed; automatic1/200; no semantic acceptance) |
| T-N08 | packed inference, quality and memory comparison | NOT_STARTED |
| T-I01 | five own-model memory queries and fresh restart | FAILED (intermediate actual CLI diagnostic4/14; no quality waiver) |
| T-I02 | timeout/cancel/citations/COMMIT failure | VERIFIED (S1 transport/application plus actual native artifact/timeout/cancel/COMMIT diagnostics; final candidate repeat pending) |
| T-I03 | raw bytes/history/restore/as_of/reindex/backup | VERIFIED (S1 persistence; native responses pending S5) |
| T-I04 | offline runtime and measured M4 backend | IMPLEMENTING (CPU training and actual CLI under deny-network; final model/quant closure pending) |
| T-D01 | temporary instructions decoupled, originals preserved | VERIFIED |
| T-D02 | phase code identity equals remote branch | VERIFIED (S0–S3; S4 workspace unpublished) |

Executed symbol/test trace (phase source manifests above identify the code):

| ID | Source symbol → independent test / execution | Raw evidence |
|---|---|---|
| T-R01 | store::result/append_in, app::terminal → runtime::rv01_results_bind_canonical_question_and_reindex_repairs; cli::rv01_corrupt_result_has_no_success_stdout_or_write | logs/goal1-s1-tests.txt |
| T-R02 | model::verify_prepared; ByteBpe::prepare → runtime::rv02_real_tokenizer_never_silently_truncates, rv02_whole_evidence_packing_and_receipt_binding; native::native_prompt_boundaries_and_exact_evidence_tail | logs/goal1-s1-tests.txt; goal1-s3-exit-tests.txt |
| T-R03 | retrieval Store::search → retrieval::rv03_origin_filter_before_limit, rv03_hops_output_cycles_and_duplicate_fetch_caps, rv03_distinct_neighbor_budget_and_snapshot | logs/goal1-s1-tests.txt |
| T-R04 | Store::startup/head → store::rv04_atomic_startup_and_head_with_writer_and_corrupt_control | logs/goal1-s1-tests.txt |
| T-R05 | Store::backup → store::rv05_pinned_backup_before_copy_and_after_done, rv05_failed_artifact_is_explicit_and_existing_destination_preserved | logs/goal1-s1-tests.txt |
| T-N01 | ByteBpe::train/encode/decode → native::own_byte_bpe_roundtrip_no_control_promotion_or_truncation; training::corpus_and_tokenizer_use_train_only_and_reject_split_leakage | logs/goal1-s3-exit-tests.txt |
| T-N02 | Transformer::forward, rms_norm/rotary/repeat_kv/attention_mask → native::native_numeric_references_and_causal_padding_gradients, native_local_global_mask_and_greedy_generation_boundaries | logs/goal1-s3-exit-tests.txt |
| T-N03 | Transformer::forward_cached/Cache → native::native_kv_chunk_rollover_parity_reset_and_identity; validate native-boundaries on actual SMALL trained artifact | logs/goal1-s3-exit-tests.txt; goal1-s4-trained-cache-parity.txt |
| T-N04 | training::Adam::step/batch, masked_loss → training::tests::adam_matches_independent_reference_and_teacher_forcing_masks; actual train command | logs/goal1-s4-rss-regression.txt; goal1-s4-small-train.txt |
| T-N05 | checkpoint::save/load → native checkpoint corruption/roundtrip tests; training::native_training_resume_is_identical_in_fresh_processes, native_training_cancel_keeps_optimizer_boundary_checkpoint | logs/goal1-s3-exit-tests.txt |
| T-N06 | data::prepare/check_split, model init/train commands → actual manifest/init/training lineage plus split regression | logs/goal1-s2-tokenizer-manifest.txt; goal1-s4-small-init.txt; goal1-s4-small-train.txt |
| T-N07 | validate::heldout/evaluate_native/compare_pairs → heldout_rubric_rejects_blank_wrong_time_citation_and_blanket_unknown;200 actual cases/four variants; semantic rejection of the sole automatic hit | logs/goal1-s4-pair-regression.txt; goal1-s4-heldout-*.jsonl; goal1-s4-pair-comparison.txt |
| T-N08 | no quantization source/test yet | NOT_RUN |
| T-I01 | model::worker/app::ask_with_history/main ask → validate smoke; new OS-seeded facts after selected intermediate checkpoint, seven questions twice in separate CLI processes and same-key replay with absent checkpoint | logs/goal1-native-cli-smoke-diagnostic-5000-fixed.txt; quality4/14, original19 events unchanged |
| T-I02 | app::ask/model::run_worker → runtime protocol tests plus validate native-failures with actual trained step5000, random/corrupt artifacts, generation timeout, synchronized SIGINT and deferred FK failure on answer COMMIT | logs/goal1-s1-tests.txt; goal1-native-failures-5000.txt (five scenarios pass) |
| T-I03 | Store lifecycle/history/restore/reindex/backup → store and cli lifecycle/restart tests; native response path pending | logs/goal1-s1-tests.txt |
| T-I04 | native train/evaluate and fourteen actual CLI answers under sandbox deny-network; CPU/Accelerate explicit, no Metal claim | logs/goal1-s4-small-v3-probe.txt; goal1-native-cli-smoke-diagnostic-5000-fixed.txt |
| T-D01 | index-only removal, unchanged local bytes and source/docs reference scan | logs/goal1-environment.txt; permanent contract |
| T-D02 | normal phase pushes and actual ls-remote matches; S4 remains unpublished | IMPLEMENTATION_REPORT.md native closure |
