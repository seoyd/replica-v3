# Independent selected-event ID preparation review

2026-09-24. Contract: `R3-SELECTED-EVENT-ID-PROTOCOL-1.0`.

**A: BLOCKED_INPUT_LENGTH / NOT_ACCEPTED.** The required ID_ONLY request plus
gold answer and EOS occupies **259 tokens in all 1,920 transformed word cases**,
exceeding the fixed sequence limit of 256. Both evidence records are retained;
none are excluded. The FULL condition fits. No TINY or SMALL training,
generation, teacher, or backward calls were made.

The implementer's preparation rejected the first ID_ONLY row before creating
the study. I independently reconstructed both modes from the preserved source
corpus, resolving the selected event from the question's entity and context and
the unique current record, then cross-checking the original FULL answer. The
reader uses the exact required neutral system and task clauses, unchanged
records and tokenizer, and QuestionEvidence framing. It does not call the
candidate transformation or construct a model.

| Condition / split | Cases checked | Minimum / maximum prompt + gold + EOS | Over 256 |
|---|---:|---:|---:|
| FULL / train | 1,536 | 234 / 234 | 0 |
| FULL / dev + renamed | 384 | 234 / 234 | 0 |
| ID_ONLY / train | 1,536 | 259 / 259 | 1,536 |
| ID_ONLY / dev + renamed | 384 | 259 / 259 | 384 |

Dev and renamed contain 192 cases each and share semantic bases; they are not
384 independent semantic groups. This check covers 3,840 transformed requests.
It does not replace the later exposure, native continuation, or quality gates.

The first failure is source train index 6144,
`qa-word-value-v1/train/0/id0/0`: ID_ONLY prompt 250 + eight digit targets and EOS
9 = 259. The original request and FULL target occupy 186 + 16 = 202. The required
longer ID instruction outweighs its shorter target. No truncation, instruction
substitution, tokenizer change, or limit increase was applied.

Evidence is under `artifacts/selected-event-id-20260924-review/`:

- `prepare_reader.rs`: independent Rust source, SHA256
  `de9c9eb51cba9c4262eff59c8811f5b5c19c75d44b434e758bf62f0e554ba671`.
- `prepare-reader`: executable, SHA256
  `2716e284259237bd99451b4222bb72e7dca65c064ef7ec45391b19564704f310`.
- `bounds-receipt.r3b`: typed summary and all 1,920 violating source/request row
  hashes, SHA256
  `d9e5998fd8bc90f7e77fb784887633000e7ff902f6360400a1cf30fd4fd1a8c4`.
- `bounds-01.log`: exit 0, SHA256
  `c0e86df42598a4c2503265cbb7c0806b88f54a714b2c9e26d5211b7066d82c8d`.

The reader compiled successfully against the existing release dependency target.
Its bounds-only invocation used one compute thread, completed in 0.75 seconds
wall time, and observed maximum RSS 63,684,608 bytes. Its model construction,
forward, backward, optimizer, generation, and teacher counts are all zero.
The `bounds-only` receipt is evidence of the input failure, not an A admission
receipt. Draft scorer, endpoint, and teacher readers remain unexecuted.

Input physical hashes were verified unchanged before and after the audit:

| Input | SHA256 |
|---|---|
| Original `corpus.r3cor` | `96e36843c792354bff98195e95db3ec98c4d0dde534aaa619787b5259665fa3c` |
| Original `tokenizer.r3b` | `ec945ee5f3cbd87992bdfa13f199de2a671b94b64337dff04d85e01d982ab9ef` |

The inputs are the existing
`artifacts/qa-word-value-20260924-study-final/ANSWER-MEAN` files. Existing model,
corpus, failed records, and closed teacher evidence were preserved.

Source baseline is `b9cf630a1b9a6dc91ecbe144006e7d06a7d9191e`. The unfinished
local candidate was statically reviewed; its inspected file hashes are:

| Candidate file | SHA256 |
|---|---|
| `src/muon.rs` | `7611a5ae20f80687f87c87c0ee3e52be92724a172bee538d8ee3e3d5b97bdf83` |
| `src/muon_diagnosis.rs` | `58ae80a91d71d77039f010f0e672dfa3f8646e4b24feef64084a1540486765ce` |
| `src/neural/checkpoint.rs` | `10bae1c179d652ddd6808708161532500d127418eb40e9c35f101b46b4bd0968` |

This report accepts no inherited-state execution or model quality result.
`EXECUTION_COMPLETE=false`, `F_I_ENDPOINT=NOT_RUN`, independent dynamic
`CODE_A=NOT_ACCEPTED`, `B_RECOUNT/PARITY=NOT_RUN`, and `GOAL1_READY=false`.
All new TINY/SMALL optimizer, generation, teacher, and backward counts remain
zero. Any continuation requires an explicit contract revision resolving the
259-versus-256 incompatibility before preparation and independent A can resume.
