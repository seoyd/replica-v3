//! Explicit release measurements and real-model smoke. Not run by cargo test.
#![forbid(unsafe_code)]
use replica_v3::{
    Error, Result, app,
    codec::Compression,
    event::*,
    model::{self, ModelConfig},
    retrieval::Search,
    store::Store,
};
use std::{process::Command, sync::atomic::AtomicBool, time::Instant};
fn stats(label: &str, mut values: Vec<f64>) {
    values.sort_by(f64::total_cmp);
    println!(
        "{label}: n={} min_ms={:.4} median_ms={:.4} max_ms={:.4}",
        values.len(),
        values[0],
        values[values.len() / 2],
        values[values.len() - 1]
    );
}
fn elapsed(start: Instant) -> f64 {
    start.elapsed().as_secs_f64() * 1000.0
}
fn native_load_audit(path: &std::path::Path, mode: &str) -> Result<()> {
    let resume = match mode {
        "inference" => false,
        "resume" => true,
        _ => return Err(Error::Invalid("load mode".into())),
    };
    let start = Instant::now();
    let (loaded, stats) =
        replica_v3::neural::artifact::load_with_stats(path, candle_core::Device::Cpu, resume)?;
    println!(
        "mode={mode} elapsed_ms={:.3} file_bytes={} actual_read_bytes={} header={} model={} optimizer={} padding={} model_tensors={} optimizer_tensors={} semantic_arch={} semantic_tokenizer={} weights_content={} trained_steps={} diagnostic_only={}",
        elapsed(start),
        std::fs::metadata(path)?.len(),
        stats.bytes_read(),
        stats.header_bytes,
        stats.model_bytes,
        stats.optimizer_bytes,
        stats.padding_bytes,
        loaded.model.vars.len(),
        loaded.optimizer.len(),
        loaded.model.config.semantic_id()?,
        loaded.tokenizer.semantic_id(),
        loaded.model.weights_content_id()?,
        loaded.manifest.trained_steps,
        loaded.manifest.diagnostic_only
    );
    println!(
        "tokenizer_native_metadata_bytes={}",
        replica_v3::neural::artifact::tokenizer_metadata_bytes(&loaded.tokenizer)
    );
    Ok(())
}
fn export_parity(legacy: &std::path::Path, native: &std::path::Path) -> Result<()> {
    use replica_v3::neural::{artifact, checkpoint};
    let original = checkpoint::legacy_load(legacy, candle_core::Device::Cpu, true)?;
    let resume = artifact::metadata(native)?.0.training.is_some();
    let converted = artifact::load(native, candle_core::Device::Cpu, resume)?;
    if original.model.config != converted.model.config
        || original.tokenizer.semantic_id() != converted.tokenizer.semantic_id()
        || original.model.vars.len() != converted.model.vars.len()
    {
        return Err(Error::Corrupt(
            "export config/tokenizer/count parity".into(),
        ));
    }
    for (name, var) in &original.model.vars {
        let other = converted
            .model
            .vars
            .get(name)
            .ok_or_else(|| Error::Corrupt("export missing tensor".into()))?;
        if var.dims() != other.dims()
            || var
                .flatten_all()?
                .to_vec1::<f32>()?
                .iter()
                .map(|v| v.to_bits())
                .ne(other
                    .flatten_all()?
                    .to_vec1::<f32>()?
                    .iter()
                    .map(|v| v.to_bits()))
        {
            return Err(Error::Corrupt(format!(
                "export bit pattern mismatch {name}"
            )));
        }
    }
    for bytes in [
        (0..=255).collect::<Vec<_>>(),
        "한글😀 한 0123456789 <assistant>\0".as_bytes().to_vec(),
    ] {
        let expected = original.tokenizer.encode(&bytes)?;
        let actual = converted.tokenizer.encode(&bytes)?;
        if expected != actual || converted.tokenizer.decode_bytes(&actual)? != bytes {
            return Err(Error::Corrupt("export byte tokenizer parity".into()));
        }
    }
    if resume {
        if original.manifest.training != converted.manifest.training
            || original.optimizer.len() != converted.optimizer.len()
        {
            return Err(Error::Corrupt("export resume state/count parity".into()));
        }
        for (name, t) in &original.optimizer {
            let other = converted
                .optimizer
                .get(name)
                .ok_or_else(|| Error::Corrupt("export Adam name".into()))?;
            if t.dims() != other.dims()
                || t.flatten_all()?
                    .to_vec1::<f32>()?
                    .iter()
                    .map(|v| v.to_bits())
                    .ne(other
                        .flatten_all()?
                        .to_vec1::<f32>()?
                        .iter()
                        .map(|v| v.to_bits()))
            {
                return Err(Error::Corrupt("export Adam bits".into()));
            }
        }
        println!(
            "all_training_config_state_exact=PASS adam_name_shape_bits=PASS tensors={}",
            converted.optimizer.len()
        );
    }
    println!(
        "tensor_name_shape_f32_bits=PASS count={} tokenizer_mapping_merge_identity=PASS byte_token_ids_roundtrip=PASS source_trained_steps={} exported_trained_steps={} diagnostic_only={} native_tokenizer_bytes={} source_quality=INHERITED",
        original.model.vars.len(),
        original.manifest.trained_steps,
        converted.manifest.trained_steps,
        converted.manifest.diagnostic_only,
        artifact::tokenizer_metadata_bytes(&converted.tokenizer)
    );
    Ok(())
}
// Separate-process relative export regression. Inputs are development data, never the new64 holdout.
fn artifact_probe(
    path: &std::path::Path,
    mode: &str,
    cases: &std::path::Path,
    limit: usize,
) -> Result<()> {
    use candle_core::{Device, Tensor};
    use replica_v3::neural::{checkpoint, hash, read_bounded};
    let loaded = match mode {
        "legacy" => checkpoint::legacy_load(path, Device::Cpu, false)?,
        "native" => checkpoint::load(path, Device::Cpu, false)?,
        _ => {
            return Err(Error::Invalid(
                "probe mode must explicitly be legacy/native".into(),
            ));
        }
    };
    let value: serde_json::Value = serde_json::from_slice(&read_bounded(cases, 64 * 1024 * 1024)?)?;
    let rows = value
        .as_array()
        .or_else(|| value.get("train").and_then(|v| v.as_array()))
        .ok_or_else(|| Error::Invalid("development cases".into()))?;
    if limit == 0 || limit > 400 || rows.len() < limit {
        return Err(Error::Invalid("probe case bound".into()));
    }
    for row in rows.iter().take(limit) {
        let request: model::ModelRequest = serde_json::from_value(row["request"].clone())?;
        let prompt = loaded.tokenizer.prepare(
            &request,
            loaded.model.config.context as u32,
            &loaded.model.config.semantic_id()?,
        )?;
        let input = Tensor::new(prompt.token_ids.as_slice(), &Device::Cpu)?.unsqueeze(0)?;
        let logits = loaded
            .model
            .forward(&input, None)?
            .narrow(1, prompt.token_ids.len() - 1, 1)?
            .flatten_all()?
            .to_vec1::<f32>()?;
        let generated = loaded.model.generate(
            &prompt.token_ids,
            request.limits.max_tokens as usize,
            30000,
            &AtomicBool::new(false),
            "artifact-relative",
        );
        let (tokens, text, finish) = match generated {
            Ok(g) => (
                g.tokens.clone(),
                loaded.tokenizer.decode(&g.tokens)?,
                g.finish,
            ),
            Err(e) => (Vec::new(), String::new(), format!("ERROR:{e}")),
        };
        println!(
            "{}",
            serde_json::json!({"id":row["id"],"prompt_digest":prompt.token_digest,"logits":logits,"tokens":tokens,"text":text,"finish":finish,"weights_content":loaded.model.weights_content_id()?,"semantic_tokenizer":loaded.tokenizer.semantic_id(),"tensor_bytes_digest":hash(&loaded.model.vars["embedding"].flatten_all()?.to_vec1::<f32>()?.iter().flat_map(|x|x.to_le_bytes()).collect::<Vec<_>>()),"relative_regression_only":true})
        );
    }
    Ok(())
}
fn kernel_profile(checkpoint_path: &std::path::Path, fixture: &std::path::Path) -> Result<()> {
    use candle_core::{Device, Tensor};
    use replica_v3::neural::{checkpoint, read_bounded, transformer::ForwardTimings};
    let loaded = checkpoint::load(checkpoint_path, Device::Cpu, false)?;
    let frozen: serde_json::Value =
        serde_json::from_slice(&read_bounded(fixture, 2 * 1024 * 1024)?)?;
    let request: model::ModelRequest =
        serde_json::from_value(frozen["train"][0]["request"].clone())?;
    let prompt = loaded.tokenizer.prepare(
        &request,
        loaded.model.config.context as u32,
        &loaded.model.config.id()?,
    )?;
    let ids = Tensor::new(prompt.token_ids.as_slice(), &Device::Cpu)?.unsqueeze(0)?;
    let mut cache = loaded.model.cache("kernel-profile");
    let logits = loaded
        .model
        .forward_cached(&ids, &mut cache, "kernel-profile")?;
    let next = logits
        .narrow(1, ids.dim(1)? - 1, 1)?
        .flatten_all()?
        .argmax(0)?
        .to_scalar::<u32>()?;
    let token = Tensor::new(&[next], &Device::Cpu)?.unsqueeze(0)?;
    println!(
        "weights={} prompt={} tokens={} backend={} dtype=F32 warmup=3 n=15 threads=VECLIB:{:?}/RAYON:{:?}; profile observer overhead excluded from linear timer, included in total",
        loaded.manifest.weights_sha256,
        prompt.token_digest,
        prompt.token_ids.len(),
        replica_v3::neural::cpu_backend(),
        std::env::var("VECLIB_MAXIMUM_THREADS"),
        std::env::var("RAYON_NUM_THREADS")
    );
    for decode in [false, true] {
        let mut totals = Vec::new();
        let mut linears = Vec::new();
        for i in 0..18 {
            let mut state = if decode {
                cache.clone()
            } else {
                loaded.model.cache("kernel-profile")
            };
            let mut profile = ForwardTimings::default();
            let start = Instant::now();
            let _ = loaded.model.forward_profiled(
                if decode { &token } else { &ids },
                &mut state,
                "kernel-profile",
                &mut profile,
            )?;
            if i >= 3 {
                totals.push(elapsed(start));
                linears.push(profile.linear_ns as f64 / 1e6);
            }
            if i == 17 {
                println!(
                    "decode={decode} linear_calls={} shapes={:?}",
                    profile.linear_calls, profile.linear_shapes
                );
            }
        }
        stats(
            if decode {
                "decode-total"
            } else {
                "prefill-total"
            },
            totals,
        );
        stats(
            if decode {
                "decode-linear"
            } else {
                "prefill-linear"
            },
            linears,
        );
    }
    Ok(())
}
fn kernel_compare(checkpoint_path: &std::path::Path, fixture: &std::path::Path) -> Result<()> {
    use candle_core::{Device, Tensor};
    use replica_v3::neural::{
        checkpoint, read_bounded,
        transformer::{Kernel, decode_linear},
    };
    let mut loaded = checkpoint::load(checkpoint_path, Device::Cpu, false)?;
    let frozen: serde_json::Value =
        serde_json::from_slice(&read_bounded(fixture, 2 * 1024 * 1024)?)?;
    let request: model::ModelRequest =
        serde_json::from_value(frozen["train"][0]["request"].clone())?;
    let prompt = loaded.tokenizer.prepare(
        &request,
        loaded.model.config.context as u32,
        &loaded.model.config.id()?,
    )?;
    let ids = Tensor::new(prompt.token_ids.as_slice(), &Device::Cpu)?.unsqueeze(0)?;
    let mut cache = loaded.model.cache("kernel-compare");
    let logits = loaded
        .model
        .forward_cached(&ids, &mut cache, "kernel-compare")?;
    let next = logits
        .narrow(1, ids.dim(1)? - 1, 1)?
        .flatten_all()?
        .argmax(0)?
        .to_scalar::<u32>()?;
    let token = Tensor::new(&[next], &Device::Cpu)?.unsqueeze(0)?;
    let mut reference_logits = None;
    let mut reference_generation = None;
    println!(
        "weights={} prompt={} backend={} CPU/F32 warmup=3 n=31 threads=VECLIB:{:?}/RAYON:{:?}; candidate copies per linear=(N*K+K+N)*4 bytes, no weight cache; cache clones share immutable tensors before append",
        loaded.manifest.weights_sha256,
        prompt.token_digest,
        replica_v3::neural::cpu_backend(),
        std::env::var("VECLIB_MAXIMUM_THREADS"),
        std::env::var("RAYON_NUM_THREADS")
    );
    for kernel in [Kernel::Reference, Kernel::RustDecodeGemv] {
        loaded.model.set_kernel(kernel);
        let mut prefill = Vec::new();
        let mut decode = Vec::new();
        let mut generation = Vec::new();
        let mut micro = Vec::new();
        let x = Tensor::from_vec(
            (0..384).map(|i| (i as f32).sin()).collect::<Vec<_>>(),
            (1, 1, 384),
            &Device::Cpu,
        )?;
        let weight = loaded.model.vars["layer.0.gate"].as_tensor();
        for i in 0..34 {
            let start = Instant::now();
            let _ = decode_linear(&x, weight, kernel)?;
            let micro_ms = elapsed(start);
            let mut state = cache.clone();
            let start = Instant::now();
            let out = loaded
                .model
                .forward_cached(&token, &mut state, "kernel-compare")?;
            let decode_ms = elapsed(start);
            if i == 0 {
                if let Some(reference) = &reference_logits {
                    let diff = (&out - reference)?.abs()?.max_all()?.to_scalar::<f32>()?;
                    println!(
                        "kernel={} decode_max_abs_logit={diff} tolerance=0.0005 cache_history={}",
                        kernel.id(),
                        state.history_id()
                    );
                    if diff > 5e-4 {
                        return Err(Error::Model("kernel logits differ".into()));
                    }
                } else {
                    reference_logits = Some(out);
                }
            }
            let mut state = loaded.model.cache("kernel-compare");
            let start = Instant::now();
            let _ = loaded
                .model
                .forward_cached(&ids, &mut state, "kernel-compare")?;
            let prefill_ms = elapsed(start);
            let start = Instant::now();
            let generated = loaded.model.generate(
                &prompt.token_ids,
                16,
                30000,
                &AtomicBool::new(false),
                "kernel-compare",
            )?;
            let generation_ms = elapsed(start);
            let actual = (generated.tokens, generated.finish);
            if let Some(reference) = &reference_generation {
                if *reference != actual {
                    return Err(Error::Model("kernel generation differs".into()));
                }
            } else {
                reference_generation = Some(actual);
            }
            if i >= 3 {
                micro.push(micro_ms);
                decode.push(decode_ms);
                prefill.push(prefill_ms);
                generation.push(generation_ms);
            }
        }
        stats(&format!("{} micro N=1024 K=384 M=1", kernel.id()), micro);
        stats(&format!("{} decode", kernel.id()), decode);
        stats(&format!("{} prefill", kernel.id()), prefill);
        stats(
            &format!("{} generation includes chunk128 prefill", kernel.id()),
            generation,
        );
    }
    println!(
        "relative_generation_parity=PASS; quality_not_evaluated; adoption_requires_full_model_speedup"
    );
    Ok(())
}
// Explicit P0 audit of a frozen legacy artifact. Never opens a user database or
// publishes a replacement checkpoint; save timing uses an owned temporary path.
fn storage_audit(path: &std::path::Path) -> Result<()> {
    use replica_v3::neural::{checkpoint, hash, read_bounded};
    let (manifest, tokenizer) = checkpoint::legacy_metadata(path)?;
    let start = Instant::now();
    let bytes = read_bounded(
        &path.join("weights.safetensors"),
        manifest.architecture.parameters() * 12 + 1024 * 1024,
    )?;
    let read_ms = elapsed(start);
    let start = Instant::now();
    let digest = hash(&bytes);
    let hash_ms = elapsed(start);
    if digest != manifest.weights_sha256 || bytes.len() != manifest.weights_bytes {
        return Err(Error::Corrupt("audit checkpoint identity".into()));
    }
    let header_bytes = u64::from_le_bytes(
        bytes
            .get(..8)
            .ok_or_else(|| Error::Corrupt("tensor header".into()))?
            .try_into()
            .map_err(|_| Error::Corrupt("tensor header".into()))?,
    );
    let start = Instant::now();
    let tensors =
        safetensors::SafeTensors::deserialize(&bytes).map_err(|e| Error::Corrupt(e.to_string()))?;
    let mut names = tensors.names();
    names.sort();
    let mut totals = [0usize; 4];
    println!("| tensor | shape | numel | dtype | bytes | 역할 |");
    println!("|---|---|---:|---|---:|---|");
    for name in names {
        let tensor = tensors
            .tensor(name)
            .map_err(|e| Error::Corrupt(e.to_string()))?;
        let (role, index) = if name.starts_with("model.") {
            ("MODEL", 0)
        } else if name.starts_with("adam.m.") {
            ("MOMENT1", 1)
        } else if name.starts_with("adam.v.") {
            ("MOMENT2", 2)
        } else {
            ("OTHER", 3)
        };
        totals[index] += tensor.data().len();
        if tensor.dtype() != safetensors::Dtype::F32
            || tensor
                .data()
                .as_chunks::<4>()
                .0
                .iter()
                .any(|v| !f32::from_le_bytes(*v).is_finite())
        {
            return Err(Error::Corrupt(format!("audit invalid tensor {name}")));
        }
        println!(
            "| {name} | {:?} | {} | {:?} | {} | {role} |",
            tensor.shape(),
            tensor.shape().iter().product::<usize>(),
            tensor.dtype(),
            tensor.data().len()
        );
    }
    let inspect_ms = elapsed(start);
    if totals.iter().sum::<usize>() as u64 + header_bytes + 8 != bytes.len() as u64 {
        return Err(Error::Corrupt("audit tensor byte accounting".into()));
    }
    println!(
        "\nweights_sha256={digest}; file_bytes={}; length_prefix=8; JSON_header_bytes={header_bytes}; role_bytes={totals:?}",
        bytes.len()
    );
    println!(
        "read_ms={read_ms:.3}; hash_ms={hash_ms:.3}; tensor_inventory_and_finite_scan_ms={inspect_ms:.3}; cache=OS_CACHE_NOT_FLUSHED"
    );
    for file in ["manifest.json", "tokenizer.json"] {
        let b = read_bounded(&path.join(file), 1024 * 1024)?;
        println!("{file}: bytes={} sha256={}", b.len(), hash(&b));
    }
    println!(
        "training_state={}",
        serde_json::to_string(&manifest.training)?
    );
    println!(
        "tokenizer_vocab={}; tokenizer_id={}",
        tokenizer.vocab_size(),
        tokenizer.id()
    );
    drop(bytes);
    let start = Instant::now();
    let loaded = checkpoint::legacy_load(path, candle_core::Device::Cpu, true)?;
    println!(
        "legacy_resume_load_ms={:.3}; model_tensors={}; optimizer_tensors={}",
        elapsed(start),
        loaded.model.vars.len(),
        loaded.optimizer.len()
    );
    let root = tempfile::tempdir()?;
    let start = Instant::now();
    let saved = checkpoint::legacy_save(
        &root.path().join("save"),
        &loaded.model,
        &loaded.tokenizer,
        loaded.manifest,
        &loaded.optimizer,
    )?;
    println!(
        "legacy_save_with_full_readback_hash_validation_sync_ms={:.3}; saved_bytes={}; tensor_shape_inventory_matches={}",
        elapsed(start),
        saved.weights_bytes,
        saved.tensors == manifest.tensors
    );
    Ok(())
}
fn checkpoint_load_audit(path: &std::path::Path, mode: &str) -> Result<()> {
    let resume = match mode {
        "inference" => false,
        "resume" => true,
        _ => return Err(Error::Invalid("load audit: inference or resume".into())),
    };
    let start = Instant::now();
    let loaded =
        replica_v3::neural::checkpoint::legacy_load(path, candle_core::Device::Cpu, resume)?;
    println!(
        "mode={mode} load_ms={:.3} weights_bytes_read={} model_tensor_bytes={} optimizer_tensor_bytes={} backend=CPU accelerate={} OS_cache=NOT_FLUSHED",
        elapsed(start),
        loaded.manifest.weights_bytes,
        loaded
            .model
            .vars
            .values()
            .map(|v| v.elem_count() * 4)
            .sum::<usize>(),
        loaded
            .optimizer
            .values()
            .map(|t| t.elem_count() * 4)
            .sum::<usize>(),
        cfg!(feature = "accelerate")
    );
    Ok(())
}
fn lineage_audit(paths: &[String]) -> Result<()> {
    use replica_v3::neural::{checkpoint, hash, read_bounded};
    println!(
        "| run 경로 | 상태 | 시작 step | 종료 step | updates | 시작 input | 종료 input | 종료 target | sampler u64 | weights SHA-256 |"
    );
    println!("|---|---|---:|---:|---:|---:|---:|---:|---:|---|");
    for path in paths {
        let path = std::path::Path::new(path);
        let (m, _) = checkpoint::legacy_metadata(path)?;
        let bytes = read_bounded(
            &path.join("weights.safetensors"),
            m.architecture.parameters() * 12 + 1024 * 1024,
        )?;
        if hash(&bytes) != m.weights_sha256 || bytes.len() != m.weights_bytes {
            return Err(Error::Corrupt(format!(
                "lineage artifact changed: {}",
                path.display()
            )));
        }
        if let Some(s) = &m.training {
            println!(
                "| {} | {} | {} | {} | {} | {} | {} | {} | {} | {} |",
                path.display(),
                m.status,
                s.config.budget_start_step,
                s.step,
                s.step.saturating_sub(s.config.budget_start_step),
                s.config.budget_start_tokens,
                s.consumed_tokens,
                s.target_tokens,
                s.sampler_state,
                m.weights_sha256
            );
        } else {
            println!(
                "| {} | {} | 0 | 0 | 0 | 0 | 0 | 0 | 해당 없음 | {} |",
                path.display(),
                m.status,
                m.weights_sha256
            );
        }
    }
    Ok(())
}
fn measure() -> Result<()> {
    let root = tempfile::tempdir()?;
    println!(
        "release={} model=EXCLUDED warm=OS_CACHE_NOT_FLUSHED temp_db_only=true",
        !cfg!(debug_assertions)
    );
    if cfg!(debug_assertions) {
        return Err(Error::Invalid("measure requires --release".into()));
    }
    for (label, compression) in [("raw", Compression::Raw), ("auto_zstd", Compression::Auto)] {
        let path = root.path().join(label);
        let mut store = Store::init(&path)?;
        store.set_compression(compression);
        let fixture: Vec<_> = (0..10_000)
            .map(|i| {
                Event::observation(
                    "synthetic",
                    &format!("session-{}", i % 13),
                    "benchmark",
                    format!(
                        "합성 이동 기록 {i}: {}\n",
                        ["LEFT RIGHT 직진 취소 같은경로 "; 12].concat()
                    )
                    .into_bytes(),
                )
            })
            .collect();
        let mut raw_bytes: usize = fixture.iter().map(|e| e.payload.len()).sum();
        let start = Instant::now();
        store.import(fixture)?;
        println!("{label} batch_import_10000_ms={:.4}", elapsed(start));
        let mut durable = Vec::new();
        for i in 0..20 {
            let e = Event::observation(
                "synthetic",
                "durable",
                "benchmark",
                format!("추가 단건 {i}").into_bytes(),
            );
            raw_bytes += e.payload.len();
            let start = Instant::now();
            store.append(e)?;
            durable.push(elapsed(start));
        }
        stats(&format!("{label} single_FULL_commit"), durable);
        let mut reads = Vec::new();
        for id in 1..=128 {
            let start = Instant::now();
            store.get(id)?;
            reads.push(elapsed(start));
        }
        stats(&format!("{label} warm_read"), reads);
        let mut latency = Vec::new();
        let mut truncations = 0;
        let mut q = Search::new("synthetic", "같은경로");
        q.graph = false;
        for _ in 0..100 {
            let start = Instant::now();
            let result = store.search(&q)?;
            truncations += usize::from(result.truncated);
            latency.push(elapsed(start));
        }
        stats(&format!("{label} warm_lexical"), latency);
        println!("{label} query_truncated_count={truncations}/100");
        let start = Instant::now();
        store.reindex()?;
        println!("{label} reindex_ms={:.4}", elapsed(start));
        println!(
            "{label} records={} original_payload_bytes={raw_bytes} sizes_before_close={:?}",
            store.count()?,
            store.storage_sizes()?
        );
        let sizes = store.storage_sizes()?;
        let index_bytes: u64 = sizes
            .iter()
            .filter(|(name, _)| name.starts_with("record_fts"))
            .map(|(_, size)| size)
            .sum();
        let total: u64 = sizes
            .iter()
            .filter(|(name, _)| name.starts_with("db"))
            .map(|(_, size)| size)
            .sum();
        println!("{label} physical_db_wal_shm_total_bytes={total} fts_pages_bytes={index_bytes}");
        let audit = rusqlite::Connection::open_with_flags(
            &path,
            rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
        )?;
        let encoded: u64 =
            audit.query_row("SELECT sum(length(body)) FROM records", [], |r| r.get(0))?;
        let fts_text: u64 = audit.query_row(
            "SELECT sum(length(CAST(text AS BLOB))) FROM record_fts",
            [],
            |r| r.get(0),
        )?;
        let page_count: u64 = audit.query_row("PRAGMA page_count", [], |r| r.get(0))?;
        let freelist: u64 = audit.query_row("PRAGMA freelist_count", [], |r| r.get(0))?;
        let page_size: u64 = audit.query_row("PRAGMA page_size", [], |r| r.get(0))?;
        println!(
            "{label} encoded_body_bytes={encoded} duplicated_fts_text_bytes={fts_text} page_count={page_count} page_size={page_size} freelist_pages={freelist}; FTS_pages_already_in_DB=true"
        );
        drop(audit);
        let backup = root.path().join(format!("{label}-backup"));
        store.backup(&backup)?;
        println!(
            "{label} separate_backup_bytes={}",
            std::fs::metadata(backup)?.len()
        );
        drop(store);
        for suffix in ["", "-wal", "-shm"] {
            let closed_path = std::path::PathBuf::from(format!("{}{suffix}", path.display()));
            let bytes = match std::fs::metadata(closed_path) {
                Ok(m) => m.len(),
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => 0,
                Err(e) => return Err(e.into()),
            };
            println!("{label} fully_closed_db{suffix}_bytes={bytes}");
        }
        let start = Instant::now();
        let store = Store::open(&path)?;
        store.get(1)?;
        println!(
            "{label} reopen_first_read_ms={:.4} (not_OS_cold)",
            elapsed(start)
        );
        println!("{label} checkpointed_sizes={:?}", store.storage_sizes()?);
        println!("{label} sqlite_doctor={:#?}", store.doctor(false)?);
    }
    let rss = Command::new("ps")
        .args(["-o", "rss=", "-p", &std::process::id().to_string()])
        .output()?;
    if !rss.status.success() {
        return Err(Error::Invalid("ps RSS query failed".into()));
    }
    println!(
        "process_current_rss_KiB={} GPU_shared_memory=NOT_MEASURED",
        String::from_utf8_lossy(&rss.stdout).trim()
    );
    println!(
        "model_load_first_token_generation=NOT_RUN; true_OS_cold=NOT_RUN; power_loss_disk_full=NOT_RUN"
    );
    Ok(())
}
fn smoke(checkpoint: &str, cli: &str, output: &str) -> Result<()> {
    use replica_v3::neural::{checkpoint as artifact, hash, write_new};
    use std::{io::Read, path::Path};
    let (manifest, _) = artifact::metadata(Path::new(checkpoint))?;
    if manifest.trained_steps == 0 {
        return Err(Error::Invalid(
            "smoke requires our actually trained checkpoint".into(),
        ));
    }
    // All memory is created now, after the supplied checkpoint has been trained.
    // Expected values stay in this example process, never in the worker input.
    let mut seed = [0; 8];
    std::fs::File::open("/dev/urandom")?.read_exact(&mut seed)?;
    let seed = u64::from_le_bytes(seed);
    let mut rng = replica_v3::neural::transformer::Rng::new(seed);
    let root = Path::new(output);
    std::fs::create_dir(root)?;
    let path = root.join("memory.db");
    let created_at = now_ms();
    let mut calls = 0;
    let mut invoke = |args: &[String]| -> Result<std::process::Output> {
        let start = Instant::now();
        let out = Command::new(cli)
            .arg("--db")
            .arg(&path)
            .args(args)
            .output()?;
        write_new(
            &root.join(format!("cli-{calls:03}.json")),
            &serde_json::to_vec_pretty(
                &serde_json::json!({"args":args,"exit":out.status.code(),"stdout":String::from_utf8_lossy(&out.stdout),"stderr":String::from_utf8_lossy(&out.stderr),"elapsed_ms":elapsed(start)}),
            )?,
        )?;
        calls += 1;
        Ok(out)
    };
    if !invoke(&["init".into()])?.status.success() {
        return Err(Error::Model(
            "smoke CLI init failed; raw output retained".into(),
        ));
    }
    let directions = [
        "오른쪽",
        "왼쪽",
        "직진",
        "대기",
        "동쪽",
        "서쪽",
        "남쪽",
        "북쪽",
    ];
    let mut cases = Vec::new();
    let mut originals = Vec::new();
    for category in 0..5 {
        let scope = format!("native-smoke-{seed:x}-{category}");
        let entity = format!("설비{}", 800_000 + rng.next_u64() % 100_000);
        let context = format!("통로{}", 20 + category);
        let index = rng.next_u64() as usize % directions.len();
        let value = if category == 4 {
            "오른쪽"
        } else {
            directions[index]
        };
        let other = directions[(index + 1) % directions.len()];
        let mut append = |mode: &str,
                          context: &str,
                          text: &str,
                          previous: Option<i64>,
                          target: Option<i64>|
         -> Result<i64> {
            let mut args: Vec<String> = if mode == "record" {
                vec!["record".into()]
            } else {
                vec![
                    "fact".into(),
                    mode.into(),
                    "--entity".into(),
                    entity.clone(),
                    "--predicate".into(),
                    "direction".into(),
                    "--context".into(),
                    context.into(),
                ]
            };
            args.extend([
                "--scope".into(),
                scope.clone(),
                "--session".into(),
                "recorded-after-training".into(),
            ]);
            if mode != "restore" {
                args.extend(["--text".into(), text.into()]);
            }
            if let Some(id) = previous {
                args.extend(["--expected-head".into(), id.to_string()]);
            }
            if let Some(id) = target {
                args.extend(["--target".into(), id.to_string()]);
            }
            if !invoke(&args)?.status.success() {
                return Err(Error::Model(
                    "smoke memory write failed; raw output retained".into(),
                ));
            }
            let store = Store::open(&path)?;
            let id = store.count()? as i64;
            originals.push(store.get(id)?);
            Ok(id)
        };
        let original_text = format!("{entity}의 {context} 이동 지시는 {value}이다.");
        let first = append("add", &context, &original_text, None, None)?;
        let mut queries = Vec::new();
        match category {
            0 => queries.push((
                format!("기록을 확인해서 {entity} {context}의 현재 방향을 말해줘."),
                Some(value.to_string()),
                vec![first],
            )),
            1 => {
                let corrected = append(
                    "correct",
                    &context,
                    &format!("{entity}의 {context} 이동 지시는 {other}이다."),
                    Some(first),
                    None,
                )?;
                queries.push((
                    format!("{entity} {context}의 정정을 반영한 현재 이동 방향은?"),
                    Some(other.to_string()),
                    vec![corrected],
                ));
                queries.push((
                    format!("{entity} {context}의 정정하기 전 과거 방향은?"),
                    Some(value.to_string()),
                    vec![first],
                ));
            }
            2 => {
                let corrected = append(
                    "correct",
                    &context,
                    &format!("{entity}의 {context} 이동 지시는 {other}이다."),
                    Some(first),
                    None,
                )?;
                let restored = append("restore", &context, "", Some(corrected), Some(first))?;
                queries.push((
                    format!("{entity} {context}는 복원한 후 현재 어느 방향으로 가야 하나?"),
                    Some(value.to_string()),
                    vec![restored],
                ));
                queries.push((
                    format!("{entity} {context}의 처음 기록했던 이동 방향과 근거는?"),
                    Some(value.to_string()),
                    vec![first],
                ));
            }
            3 => {
                append(
                    "add",
                    "구역31",
                    &format!("{entity}의 구역31 이동 지시는 {other}이다."),
                    None,
                    None,
                )?;
                append("record", "", "관련 없는 창고 점검은 내일이다.", None, None)?;
                queries.push((
                    format!("{entity}의 구역31과 구분하여 {context}에 해당하는 방향을 말해줘."),
                    Some(value.to_string()),
                    vec![first],
                ));
            }
            _ => {
                let executed = append(
                    "record",
                    "",
                    &format!("{entity}는 오른쪽 지시를 실행했다."),
                    None,
                    None,
                )?;
                let accident = append(
                    "record",
                    "",
                    &format!("이후 {entity}의 사고가 기록되었다. 원인은 확인되지 않았다."),
                    None,
                    None,
                )?;
                for (from, to) in [(first, executed), (executed, accident)] {
                    let out = invoke(&[
                        "relation".into(),
                        "add".into(),
                        "--scope".into(),
                        scope.clone(),
                        "--kind".into(),
                        "precedes".into(),
                        "--from".into(),
                        from.to_string(),
                        "--to".into(),
                        to.to_string(),
                        "--evidence".into(),
                        format!("{from},{to}"),
                        "--text".into(),
                        "시간 순서이며 인과관계의 확정이 아니다.".into(),
                    ])?;
                    if !out.status.success() {
                        return Err(Error::Model("smoke relation write failed".into()));
                    }
                    let store = Store::open(&path)?;
                    originals.push(store.get(store.count()? as i64)?);
                }
                queries.push((format!("{entity}의 지시, 실행 다음 사고라는 순서에서 확인된 기록과 모르는 원인을 구분해줘."), None, vec![first, executed, accident]));
            }
        }
        // A matching entity/context in another scope must never enter this prompt.
        let out = invoke(&[
            "record".into(),
            "--scope".into(),
            format!("private-{scope}"),
            "--text".into(),
            format!("{entity}의 {context} 이동 지시는 {other}이다."),
        ])?;
        if !out.status.success() {
            return Err(Error::Model("smoke scope fixture write failed".into()));
        }
        let store = Store::open(&path)?;
        originals.push(store.get(store.count()? as i64)?);
        for (question, required, cited) in queries {
            let mut search = Search::new(&scope, &question);
            search.history = true;
            search.memory_only = true;
            let evidence = store.search(&search)?;
            cases.push((
                scope.clone(),
                HoldoutCase {
                    id: cases.len(),
                    category,
                    required: required.clone(),
                    forbidden: directions
                        .iter()
                        .filter(|&&d| required.as_deref() != Some(d))
                        .map(|d| d.to_string())
                        .collect(),
                    citations: cited,
                    confirmed_sequence: category == 4,
                    request: model::ModelRequest {
                        request_id: String::new(),
                        system: model::SYSTEM.into(),
                        input: question,
                        evidence,
                        limits: GenerationLimits::default(),
                    },
                },
            ));
        }
    }
    write_new(
        &root.join("fixture.json"),
        &serde_json::to_vec_pretty(
            &serde_json::json!({"created_at":created_at,"seed":seed,"checkpoint":manifest,"cases":cases}),
        )?,
    )?;
    let mut correct = 0;
    let mut restarts = 0;
    let mut replays = 0;
    let mut results = Vec::new();
    for round in 0..2 {
        for (scope, case) in &cases {
            let mut key = [0u8; 16];
            std::fs::File::open("/dev/urandom")?.read_exact(&mut key)?;
            let key_text: String = key.iter().map(|b| format!("{b:02x}")).collect();
            let mut args = vec![
                "ask".into(),
                "--scope".into(),
                scope.clone(),
                "--session".into(),
                format!("fresh-process-{round}"),
                "--history".into(),
                "--request-key".into(),
                key_text,
                "--checkpoint".into(),
                checkpoint.into(),
                "--text".into(),
                case.request.input.clone(),
            ];
            let out = invoke(&args)?;
            let store = Store::open(&path)?;
            let input = store
                .request(scope, key)?
                .ok_or_else(|| Error::Model("smoke input was not persisted".into()))?;
            if input.payload != case.request.input.as_bytes() {
                return Err(Error::Corrupt(
                    "smoke original question bytes changed".into(),
                ));
            }
            let answer = store.result(input.id)?;
            let mut passed = false;
            let mut detail = serde_json::json!({"error":"no successful answer"});
            if let Some(answer) = &answer {
                if let Kind::AssistantAnswer {
                    evidence,
                    provided,
                    excluded,
                    generation,
                    ..
                } = &answer.kind
                {
                    if !out.status.success() || evidence.iter().any(|id| !provided.contains(id)) {
                        return Err(Error::Corrupt("smoke success/provided mismatch".into()));
                    }
                    for id in provided {
                        if store.get(*id)?.scope != *scope {
                            return Err(Error::Corrupt("smoke cross-scope evidence".into()));
                        }
                    }
                    let text = std::str::from_utf8(&answer.payload)
                        .map_err(|_| Error::Corrupt("smoke answer UTF-8".into()))?;
                    let score = score_answer(case, text, evidence);
                    passed = score.0;
                    detail = serde_json::json!({"event":answer.id,"text":text,"provided":provided,"excluded":excluded,"cited":evidence,"generation":generation,"score_reason":score.1});
                    let before = store.count()?;
                    drop(store);
                    // An absent model path makes any accidental second inference fail.
                    let model_arg = args
                        .iter()
                        .position(|s| s == "--checkpoint")
                        .expect("constructed CLI arguments")
                        + 1;
                    args[model_arg] = root
                        .join("nonexistent-checkpoint-replay")
                        .to_string_lossy()
                        .into_owned();
                    let replay = invoke(&args)?;
                    let store = Store::open(&path)?;
                    if !replay.status.success()
                        || replay.stdout != out.stdout
                        || store.count()? != before
                        || store.result(input.id)?.as_ref() != Some(answer)
                    {
                        return Err(Error::Corrupt(
                            "smoke replay changed committed result or called model".into(),
                        ));
                    }
                    replays += 1;
                } else if !out.stdout.is_empty() {
                    return Err(Error::Corrupt(
                        "smoke failure printed success output".into(),
                    ));
                }
            }
            correct += usize::from(passed);
            restarts += usize::from(round == 1 && passed);
            let row = serde_json::json!({"round":round,"case":case.id,"category":case.category,"question":case.request.input,"required":case.required,"expected_citations":case.citations,"passed":passed,"actual":detail});
            println!("{row}");
            results.push(row);
        }
    }
    let store = Store::open(&path)?;
    for original in &originals {
        if store.get(original.id)? != *original {
            return Err(Error::Corrupt("smoke canonical memory changed".into()));
        }
    }
    store.doctor(true)?;
    let summary = serde_json::json!({"correct":correct,"total":cases.len()*2,"fresh_process_repeat_correct":restarts,"same_key_no_model_replays":replays,"canonical_originals_checked":originals.len(),"memory_sizes":store.storage_sizes()?,"results_sha256":hash(&serde_json::to_vec(&results)?),"semantic_review":"separate"});
    println!("{summary}");
    write_new(
        &root.join("results.json"),
        &serde_json::to_vec_pretty(&serde_json::json!({"summary":summary,"results":results}))?,
    )?;
    if correct != cases.len() * 2 || replays != cases.len() * 2 {
        return Err(Error::Model(
            "native memory smoke quality/restart gate failed; all raw outputs retained".into(),
        ));
    }
    Ok(())
}
fn run() -> Result<()> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    match args.first().map(String::as_str) {
        Some("export-parity") if args.len()==3 => export_parity(std::path::Path::new(&args[1]),std::path::Path::new(&args[2])),
        Some("native-load-audit") if args.len()==3 => native_load_audit(std::path::Path::new(&args[1]),&args[2]),
        Some("artifact-probe") if args.len()==5 => artifact_probe(std::path::Path::new(&args[1]),&args[2],std::path::Path::new(&args[3]),args[4].parse().map_err(|_|Error::Invalid("probe limit".into()))?),
        Some("kernel-compare") if args.len() == 3 => kernel_compare(std::path::Path::new(&args[1]), std::path::Path::new(&args[2])),
        Some("kernel-profile") if args.len() == 3 => kernel_profile(std::path::Path::new(&args[1]), std::path::Path::new(&args[2])),
        Some("lineage-audit") if args.len() > 1 => lineage_audit(&args[1..]),
        Some("storage-audit") if args.len() == 2 => storage_audit(std::path::Path::new(&args[1])),
        Some("checkpoint-load-audit") if args.len() == 3 => checkpoint_load_audit(std::path::Path::new(&args[1]), &args[2]),
        Some("holdout-prepare") if args.len() == 2 => {
            prepare_holdout(std::path::Path::new(&args[1]))
        }
        Some("evaluate") if args.len() == 5 => evaluate_native(
            std::path::Path::new(&args[1]),
            std::path::Path::new(&args[2]),
            &args[3],
            std::path::Path::new(&args[4]),
        ),
        Some("retrieval-baseline") if args.len() == 3 => retrieval_baseline(
            std::path::Path::new(&args[1]),
            std::path::Path::new(&args[2]),
        ),
        Some("compare-pairs") if args.len() == 3 => compare_pairs(
            std::path::Path::new(&args[1]),
            std::path::Path::new(&args[2]),
        ),
        Some("native-boundaries") if args.len() == 2 => {
            native_boundaries(std::path::Path::new(&args[1]))
        }
        Some("measure") => measure(),
        Some("smoke") if args.len() == 4 => smoke(&args[1], &args[2], &args[3]),
        Some("native-failures") if args.len() == 4 => native_failures(&args[1], &args[2], &args[3]),
        Some("__model-worker") => {
            use clap::Parser;
            #[derive(Parser)]
            struct Worker {
                #[command(flatten)]
                config: ModelConfig,
            }
            model::worker(
                Worker::parse_from(
                    std::iter::once("worker".to_string()).chain(args.into_iter().skip(1)),
                )
                .config,
            )
        }
        _ => Err(Error::Invalid(
            "usage: validate measure | smoke CHECKPOINT CLI OUTPUT_DIRECTORY | native-failures CHECKPOINT CLI OUTPUT_DIRECTORY | native-boundaries CHECKPOINT | holdout-prepare FILE | evaluate CHECKPOINT FIXTURE MODE OUTPUT | retrieval-baseline FIXTURE OUTPUT | compare-pairs ORIGINAL SWAPPED".into(),
        )),
    }
}
// Explicit integration measurement; no test worker or model double participates.
fn native_failures(checkpoint: &str, cli: &str, output: &str) -> Result<()> {
    use replica_v3::neural::{checkpoint as artifact, write_new};
    use std::{path::Path, process::Stdio, time::Duration};
    let (manifest, tok) = artifact::metadata(Path::new(checkpoint))?;
    if manifest
        .training
        .as_ref()
        .is_none_or(|state| state.step == 0)
    {
        return Err(Error::Invalid(
            "failure diagnostic requires actual trained weights".into(),
        ));
    }
    let root = Path::new(output);
    std::fs::create_dir(root)?;
    let random_path = root.join("random");
    let random = replica_v3::neural::transformer::Transformer::init(
        manifest.architecture.clone(),
        manifest.init_seed,
        candle_core::Device::Cpu,
    )?;
    let random_manifest = artifact::initialized(
        &random,
        &tok,
        manifest.init_seed,
        manifest.source_id.clone(),
    )?;
    artifact::save(
        &random_path,
        &random,
        &tok,
        random_manifest,
        &Default::default(),
    )?;
    drop(random);
    let corrupt = root.join("corrupt");
    write_new(&corrupt, b"truncated")?;
    let question = "설비987643 통로27 현재 이동 지시는 무엇인가?";
    let mut measurements = Vec::new();
    let mut command = Command::new(cli);
    command.args(["__model-worker", "--checkpoint", checkpoint]);
    let request = model::ModelRequest {
        request_id: "native-load-timeout".into(),
        system: model::SYSTEM.into(),
        input: question.into(),
        evidence: Default::default(),
        limits: GenerationLimits::default(),
    };
    let start = Instant::now();
    let result = model::run_worker(
        command,
        &request,
        &AtomicBool::new(false),
        Duration::from_millis(1),
    );
    let error = result.err().map(|e| e.to_string());
    let passed = error
        .as_deref()
        .is_some_and(|e| e.contains("worker timeout"))
        && start.elapsed() < Duration::from_secs(3);
    let row = serde_json::json!({"mode":"load-timeout","trained_reference_sha256":manifest.weights_sha256,"passed":passed,"error":error,"elapsed_ms":elapsed(start),"layer":"actual native owned worker; no application DB in this transport check"});
    println!("{row}");
    write_new(
        &root.join("load-timeout.json"),
        &serde_json::to_vec_pretty(&row)?,
    )?;
    measurements.push(row);
    for (index, mode) in [
        "random",
        "corrupt",
        "generation-timeout",
        "cancel",
        "commit",
    ]
    .iter()
    .enumerate()
    {
        let path = root.join(format!("{mode}.db"));
        let mut store = Store::init(&path)?;
        let original = store.append(Event::observation(
            "failure-check",
            "after-training",
            "user",
            "설비987643의 통로27 이동 지시는 서쪽이다."
                .as_bytes()
                .to_vec(),
        ))?;
        if *mode == "commit" {
            // The input commit succeeds. Only a real validated AssistantAnswer
            // creates the deferred foreign-key failure at transaction COMMIT.
            rusqlite::Connection::open(&path)?.execute_batch("CREATE TABLE deferred_fault(x INTEGER REFERENCES records(id) DEFERRABLE INITIALLY DEFERRED); CREATE TRIGGER injected_commit_failure AFTER INSERT ON record_meta WHEN NEW.kind=3 BEGIN INSERT INTO deferred_fault VALUES(9223372036854775807); END;")?;
        }
        drop(store);
        let selected = match *mode {
            "random" => random_path.as_path(),
            "corrupt" => corrupt.as_path(),
            _ => Path::new(checkpoint),
        };
        let key = [(index + 1) as u8; 16];
        let key_text: String = key.iter().map(|b| format!("{b:02x}")).collect();
        let mut command = Command::new(cli);
        command
            .arg("--db")
            .arg(&path)
            .args([
                "ask",
                "--scope",
                "failure-check",
                "--request-key",
                &key_text,
                "--checkpoint",
            ])
            .arg(selected)
            .args(["--text", question]);
        if *mode == "generation-timeout" {
            command.args(["--timeout-ms", "1"]);
        }
        let start = Instant::now();
        let mut child = command
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?;
        if *mode == "cancel" {
            let deadline = Instant::now() + Duration::from_secs(3);
            loop {
                if child.try_wait()?.is_some() {
                    return Err(Error::Model(
                        "native cancellation process exited before input synchronization".into(),
                    ));
                }
                if Store::open(&path)?.request("failure-check", key)?.is_some() {
                    break;
                }
                if Instant::now() >= deadline {
                    child.kill()?;
                    child.wait()?;
                    return Err(Error::Model(
                        "native cancellation input synchronization timeout".into(),
                    ));
                }
                std::thread::sleep(Duration::from_millis(2));
            }
            if !Command::new("kill")
                .args(["-INT", &child.id().to_string()])
                .status()?
                .success()
            {
                child.kill()?;
                child.wait()?;
                return Err(Error::Model("native cancellation signal failed".into()));
            }
        }
        let deadline = Instant::now() + Duration::from_secs(10);
        while child.try_wait()?.is_none() {
            if Instant::now() >= deadline {
                child.kill()?;
                child.wait()?;
                return Err(Error::Model(format!(
                    "native {mode} did not exit within diagnostic bound"
                )));
            }
            std::thread::sleep(Duration::from_millis(5));
        }
        let result = child.wait_with_output()?;
        let store = Store::open(&path)?;
        let input = store
            .request("failure-check", key)?
            .ok_or_else(|| Error::Model("native failure lost original question".into()))?;
        let terminal = store
            .result(input.id)?
            .ok_or_else(|| Error::Model("native failure missing terminal record".into()))?;
        let stderr = String::from_utf8_lossy(&result.stderr).into_owned();
        let expected_error = match *mode {
            "random" => "no actual optimizer updates",
            "corrupt" => "corruption",
            "generation-timeout" => "timeout",
            "cancel" => "cancelled",
            "commit" => "FOREIGN KEY constraint failed",
            _ => unreachable!(),
        };
        let pass = !result.status.success()
            && result.stdout.is_empty()
            && stderr.contains(expected_error)
            && input.payload == question.as_bytes()
            && matches!(terminal.kind, Kind::Failure { .. })
            && store.get(original.id)? == original
            && store.count()? == 3;
        store.doctor(true)?;
        let before = store.count()?;
        drop(store);
        let mut retry = Command::new(cli);
        retry.arg("--db").arg(&path).args([
            "ask",
            "--scope",
            "failure-check",
            "--request-key",
            &key_text,
            "--checkpoint",
            "nonexistent-failed-request-replay",
            "--text",
            question,
        ]);
        if *mode == "generation-timeout" {
            retry.args(["--timeout-ms", "1"]);
        }
        let replay = retry.output()?;
        let store = Store::open(&path)?;
        let replay_ok = !replay.status.success()
            && replay.stdout.is_empty()
            && String::from_utf8_lossy(&replay.stderr).contains("persisted failure")
            && store.count()? == before
            && store.result(input.id)? == Some(terminal.clone());
        let row = serde_json::json!({"mode":mode,"trained_reference_sha256":manifest.weights_sha256,"selected_artifact":selected,"passed":pass && replay_ok,"exit":result.status.code(),"stdout":String::from_utf8_lossy(&result.stdout),"stderr":stderr,"elapsed_ms":elapsed(start),"failure_payload":String::from_utf8_lossy(&terminal.payload),"same_key_failure_preserved":replay_ok});
        println!("{row}");
        write_new(
            &root.join(format!("{mode}.json")),
            &serde_json::to_vec_pretty(&row)?,
        )?;
        measurements.push(row);
    }
    write_new(
        &root.join("summary.json"),
        &serde_json::to_vec_pretty(&measurements)?,
    )?;
    if measurements.iter().any(|r| r["passed"] != true) {
        return Err(Error::Model(
            "native failure diagnostic failed; raw records retained".into(),
        ));
    }
    Ok(())
}
fn native_boundaries(path: &std::path::Path) -> Result<()> {
    use candle_core::{Device, Tensor};
    let loaded = replica_v3::neural::checkpoint::load(path, Device::Cpu, false)?;
    let model = &loaded.model;
    let start = Instant::now();
    println!(
        "NUMERIC_ONLY profile={} parameters={} tokenizer={} checkpoint={} backend=CPU dtype=F32",
        model.config.profile,
        model.config.parameters(),
        loaded.tokenizer.id(),
        loaded.manifest.weights_sha256
    );
    for len in [255usize, 256, 257, 513, 1024, 2048] {
        if len > model.config.context {
            continue;
        }
        let ids: Vec<_> = (0..len)
            .map(|i| 8 + (i % (model.config.vocab - 8)) as u32)
            .collect();
        let reference = model
            .forward(
                &Tensor::new(ids.as_slice(), &Device::Cpu)?.unsqueeze(0)?,
                None,
            )?
            .detach();
        let mut cache = model.cache("numeric-boundary");
        let mut parts = Vec::new();
        for chunk in ids.chunks(128) {
            parts.push(
                model
                    .forward_cached(
                        &Tensor::new(chunk, &Device::Cpu)?.unsqueeze(0)?,
                        &mut cache,
                        "numeric-boundary",
                    )?
                    .detach(),
            );
        }
        let cached = Tensor::cat(&parts, 1)?;
        let error = (&cached - &reference)?
            .abs()?
            .max_all()?
            .to_scalar::<f32>()?;
        println!(
            "tokens={len} max_logit_error={error} retained={:?} kv_bytes={} max_attention_tensor_bytes={} elapsed_s={:.3}",
            cache.retained_tokens(),
            cache.bytes(),
            cache.max_attention_bytes,
            start.elapsed().as_secs_f64()
        );
        if error > 5e-5 {
            return Err(Error::Model("native cache/reference parity".into()));
        }
    }
    Ok(())
}
fn main() {
    if let Err(error) = run() {
        eprintln!("{error}");
        std::process::exit(1);
    }
}

// Independent final evaluation renderer. Neither library nor trainer links this
// example, and only ModelRequest (never the expected fields) reaches the model.
#[derive(Clone, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct HoldoutCase {
    id: usize,
    category: usize,
    request: model::ModelRequest,
    required: Option<String>,
    forbidden: Vec<String>,
    citations: Vec<i64>,
    #[serde(default)]
    confirmed_sequence: bool,
}
#[derive(serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct Holdout {
    version: u32,
    seed: u64,
    cases: Vec<HoldoutCase>,
    value_pairs: Vec<HoldoutCase>,
}
#[cfg(test)]
fn heldout(seed: u64, swapped: bool) -> Vec<HoldoutCase> {
    use replica_v3::retrieval::{Evidence, EvidenceBundle};
    let directions = ["오른쪽", "왼쪽", "직진", "대기"];
    let past = ["북쪽", "남쪽", "동쪽", "서쪽"];
    let mut cases = Vec::new();
    for category in 0..5 {
        for index in 0..40 {
            let serial = category * 40 + index;
            // Deliberately different construction and entity/template families from
            // the training generator; exact answer expectations stay in this process.
            let nonce = seed
                .wrapping_mul(17)
                .wrapping_add((serial as u64 + 1) * 7919);
            let entity = format!("해솔{:x}호", nonce);
            let context = format!("통로{}", index % 9);
            let chosen = index % 4;
            let value = directions[chosen];
            let previous = past[chosen];
            let other = directions[(chosen + 1) % 4];
            let id = 1 + (nonce % 990) as i64;
            let e = |id, text: String, status: &str| Evidence {
                event_id: id,
                original_excerpt: text,
                excerpt_truncated: false,
                source: "heldout-local".into(),
                recorded_at: id,
                observed_at: None,
                version_status: status.into(),
                retrieval_reason: "lexical".into(),
                relation_path: vec![],
            };
            let mut items = vec![e(
                id,
                format!("{context}에서 {entity}에 적용되는 이동 방향은 {value}이다."),
                "current",
            )];
            let (question, mut required, citations) = match category {
                0 => {
                    items.insert(
                        0,
                        e(
                            id + 1,
                            format!("다른 장비의 {context} 지시는 {other}이다."),
                            "current",
                        ),
                    );
                    (
                        format!(
                            "{context} {entity}에게 현재 적용되는 이동 지시를 근거와 함께 답해줘."
                        ),
                        Some(value.to_string()),
                        vec![id],
                    )
                }
                1 => {
                    items[0].event_id = id + 1;
                    items[0].recorded_at = id + 1;
                    items.insert(0,e(id,format!("처음 {entity}의 {context} 지시는 {previous}이었으나 나중에 정정되었다."),"superseded"));
                    if index % 3 == 0 {
                        items[1].version_status = "superseded".into();
                        items.push(e(
                            id + 2,
                            format!("이후 {entity}의 {context} 지시를 {previous}으로 복원했다."),
                            "current",
                        ));
                        (
                            format!("{entity} {context} 복원까지 끝난 지금 방향은 무엇이지?"),
                            Some(previous.to_string()),
                            vec![id + 2],
                        )
                    } else if index % 3 == 1 {
                        (
                            format!(
                                "{entity} {context} 지금 말고 정정하기 전의 이동 지시를 알려줘."
                            ),
                            Some(previous.to_string()),
                            vec![id],
                        )
                    } else {
                        (
                            format!(
                                "{entity} {context}의 최신 정정 내용을 따르려면 어느 방향으로 가야 해?"
                            ),
                            Some(value.to_string()),
                            vec![id + 1],
                        )
                    }
                }
                2 => {
                    items.push(e(
                        id + 1,
                        format!("{entity}는 별관에서 {other} 지시를 받았다."),
                        "current",
                    ));
                    items.push(e(
                        id + 2,
                        "유리창 점검은 내일 진행한다.".into(),
                        "observation_or_interpretation",
                    ));
                    (
                        format!("별관 기록과 구분해서 {entity}의 {context} 방향만 말해줘."),
                        Some(value.to_string()),
                        vec![id],
                    )
                }
                3 => {
                    items[0].original_excerpt =
                        format!("새 장비 {entity}의 {context} 이동값을 {value}으로 등록했다.");
                    (
                        format!("방금 등록된 {entity} {context}의 이동값은 어떤 값인가?"),
                        Some(value.to_string()),
                        vec![id],
                    )
                }
                _ => {
                    if index % 2 == 0 {
                        items.clear();
                        (
                            format!("{entity}가 사고를 낸 원인이 무엇이라고 기록되어 있나?"),
                            None,
                            vec![],
                        )
                    } else {
                        items.push(e(
                            id + 1,
                            format!("{entity}는 해당 지시를 실행했다."),
                            "observation_or_interpretation",
                        ));
                        items.push(e(id+2,format!("그 뒤 {entity}에게 사고가 발생했다. 사고 원인은 아직 확정되지 않았다."),"observation_or_interpretation"));
                        (
                            format!(
                                "{entity} 지시와 실행 다음 사고가 있었다는 순서만으로 원인도 확인된 것인가?"
                            ),
                            None,
                            vec![id + 2],
                        )
                    }
                }
            };
            if swapped && let Some(original) = &required {
                let pool = if directions.contains(&original.as_str()) {
                    &directions
                } else {
                    &past
                };
                let replacement = pool[(chosen + 1) % pool.len()];
                for item in &mut items {
                    if citations.contains(&item.event_id) {
                        item.original_excerpt =
                            item.original_excerpt.replace(original, replacement);
                    }
                }
                required = Some(replacement.to_string());
            }
            if index % 2 == 0 {
                items.reverse();
            }
            let forbidden = directions
                .iter()
                .chain(&past)
                .filter(|&&v| required.as_deref() != Some(v))
                .map(|s| s.to_string())
                .collect();
            cases.push(HoldoutCase {
                id: serial,
                category,
                request: model::ModelRequest {
                    request_id: format!("heldout-{serial}"),
                    system: model::SYSTEM.into(),
                    input: question,
                    evidence: EvidenceBundle {
                        items,
                        ..Default::default()
                    },
                    limits: GenerationLimits {
                        max_tokens: 128,
                        context_tokens: 2048,
                        timeout_ms: 120000,
                    },
                },
                required,
                forbidden,
                citations,
                confirmed_sequence: false,
            });
        }
    }
    cases
}
// Fresh evaluation construction, independent of data.rs and the exposed v1
// fixture. Random IDs/order/times cannot identify the answer. Restoration keeps
// original bytes, and chronological evidence is distinct from causal proof.
fn heldout_memory_v2(seed: u64, swapped: bool) -> Vec<HoldoutCase> {
    use replica_v3::{
        neural::transformer::Rng,
        retrieval::{Evidence, EvidenceBundle},
    };
    let mut rng = Rng::new(seed);
    let values = [
        "북쪽",
        "직진",
        "왼쪽",
        "동쪽",
        "대기",
        "남쪽",
        "오른쪽",
        "서쪽",
    ];
    let mut cases = Vec::with_capacity(200);
    for serial in 0usize..200 {
        let category = serial / 40;
        let variant = serial % 4;
        let entity = format!("장비{}", 1_000_000 + rng.next_u64() % 8_000_000);
        let different = format!("장비{}", 1_000_000 + rng.next_u64() % 8_000_000);
        // Ensure the distractor is different even in the unlikely random collision.
        let different = if different == entity {
            format!("{different}호")
        } else {
            different
        };
        let area = ["통로", "현장", "구역"][(rng.next_u64() % 3) as usize];
        let area_number = rng.next_u64() % 20;
        let place = format!("{area}{}", 20 + area_number);
        let elsewhere = format!(
            "{area}{}",
            20 + (area_number + 1 + rng.next_u64() % 19) % 20
        );
        let choice = (rng.next_u64() % values.len() as u64) as usize;
        let value = values[choice];
        let earlier = values[(choice + 3) % values.len()];
        let distractor = values[(choice + 5) % values.len()];
        let mut ids = Vec::with_capacity(4);
        while ids.len() < 4 {
            let id = 1 + (rng.next_u64() % 9999) as i64;
            if !ids.contains(&id) {
                ids.push(id);
            }
        }
        let base_time = 1_800_000_000_000i64 + (rng.next_u64() % 100_000_000) as i64;
        let row = |index: usize, text: String, status: &str| Evidence {
            event_id: ids[index],
            original_excerpt: text,
            excerpt_truncated: false,
            source: "local-evaluation".into(),
            recorded_at: base_time + index as i64 * 2000,
            observed_at: Some(base_time - 7000 + index as i64 * 2000),
            version_status: status.into(),
            retrieval_reason: "lexical".into(),
            relation_path: vec![],
        };
        let original = format!("{entity}의 {place} 이동 지시는 {earlier}이다.");
        let latest = format!("{entity}의 {place} 이동 지시는 {value}이다.");
        let mut items = Vec::new();
        let (question, mut required, mut citations, confirmed_sequence) = match category {
            0 => {
                items.push(row(0, latest, "current"));
                items.push(row(
                    1,
                    format!("{different}의 {place} 이동 지시는 {distractor}이다."),
                    "current",
                ));
                (
                    format!("{place}에서 {entity}에 적용할 현재 지시와 그 근거를 말해줘."),
                    Some(value.to_string()),
                    vec![ids[0]],
                    false,
                )
            }
            1 => {
                items.push(row(0, original.clone(), "superseded"));
                items.push(row(
                    1,
                    latest,
                    if variant >= 2 {
                        "superseded"
                    } else {
                        "current"
                    },
                ));
                if variant >= 2 {
                    items.push(row(2, original, "current"));
                }
                let (when, answer, index) = match variant {
                    0 => ("정정된 뒤의 현재 기록", value, 1),
                    1 => ("정정하기 전의 첫 기록", earlier, 0),
                    2 => ("복원까지 마친 뒤의 현재 기록", earlier, 2),
                    _ => (
                        "복원된 현재 기록이 아니라 처음 작성한 과거 기록",
                        earlier,
                        0,
                    ),
                };
                (
                    format!("{entity} {place}: {when}의 방향과 사건 번호를 답해줘."),
                    Some(answer.to_string()),
                    vec![ids[index]],
                    false,
                )
            }
            2 => {
                items.push(row(0, latest, "current"));
                items.push(row(
                    1,
                    format!("{entity}의 {elsewhere} 이동 지시는 {distractor}이다."),
                    "current",
                ));
                items.push(row(
                    2,
                    format!("{different}의 점검 기록은 이동 지시와 관계없다."),
                    "observation_or_interpretation",
                ));
                (
                    format!(
                        "{elsewhere}와 섞지 말고 {entity}의 {place}에만 해당하는 방향을 답해줘."
                    ),
                    Some(value.to_string()),
                    vec![ids[0]],
                    false,
                )
            }
            3 => {
                items.push(row(0, latest, "current"));
                if variant.is_multiple_of(2) {
                    items.push(row(
                        1,
                        format!("{different}의 {place} 이동 지시는 {distractor}이다."),
                        "current",
                    ));
                }
                (
                    format!(
                        "처음 보는 {entity}라도 지금 제공된 {place} 기록에서 이동 값을 확인해줘."
                    ),
                    Some(value.to_string()),
                    vec![ids[0]],
                    false,
                )
            }
            _ if variant.is_multiple_of(2) => {
                if variant == 2 {
                    items.push(row(
                        3,
                        format!("{different}의 점검을 완료했다."),
                        "observation_or_interpretation",
                    ));
                }
                (
                    format!("{entity}의 사고 원인에 관해 확인된 근거가 있는지 답해줘."),
                    None,
                    vec![],
                    false,
                )
            }
            _ => {
                items.push(row(0, latest, "current"));
                items.push(row(
                    1,
                    format!("{entity}는 앞서 받은 이동 지시를 실행했다."),
                    "observation_or_interpretation",
                ));
                items.push(row(
                    2,
                    format!("이후 {entity}의 사고가 기록되었다. 원인을 확정한 기록은 없다."),
                    "observation_or_interpretation",
                ));
                (
                    format!(
                        "{entity}의 지시, 실행, 사고 중 기록으로 확인되는 순서를 말하고 원인도 확정됐는지 구분해줘."
                    ),
                    None,
                    vec![ids[0], ids[1], ids[2]],
                    true,
                )
            }
        };
        if swapped && let Some(original) = &required {
            // Change only the correct evidence value. Question, IDs, ordering,
            // metadata and every distractor remain byte-for-byte the same.
            let index = values
                .iter()
                .position(|v| *v == original)
                .expect("fixture value");
            let replacement = values[(index + 1) % values.len()];
            for item in &mut items {
                if citations.contains(&item.event_id) {
                    item.original_excerpt = item.original_excerpt.replace(original, replacement);
                }
            }
            required = Some(replacement.to_string());
        }
        for index in (1..items.len()).rev() {
            let other = (rng.next_u64() % (index + 1) as u64) as usize;
            items.swap(index, other);
        }
        citations.sort_unstable();
        cases.push(HoldoutCase {
            id: serial,
            category,
            request: model::ModelRequest {
                request_id: format!("independent-memory-{serial}"),
                system: model::SYSTEM.into(),
                input: question,
                evidence: EvidenceBundle {
                    items,
                    ..Default::default()
                },
                limits: GenerationLimits {
                    max_tokens: 128,
                    context_tokens: 2048,
                    timeout_ms: 120000,
                },
            },
            forbidden: values
                .iter()
                .filter(|v| Some(**v) != required.as_deref())
                .map(|v| v.to_string())
                .collect(),
            required,
            citations,
            confirmed_sequence,
        });
    }
    cases
}
fn prepare_holdout(path: &std::path::Path) -> Result<()> {
    use std::io::Read;
    let mut bytes = [0u8; 8];
    std::fs::File::open("/dev/urandom")?.read_exact(&mut bytes)?;
    let seed = u64::from_le_bytes(bytes);
    let fixture = Holdout {
        version: 2,
        seed,
        cases: heldout_memory_v2(seed, false),
        value_pairs: heldout_memory_v2(seed, true),
    };
    let bytes = serde_json::to_vec(&fixture)?;
    replica_v3::neural::write_new(path, &bytes)?;
    println!(
        "heldout_sha256={} version=2 seed={seed} cases=200 categories=5 each=40 independent_renderer=true",
        replica_v3::neural::hash(&bytes)
    );
    Ok(())
}
fn score_answer(case: &HoldoutCase, text: &str, cited: &[i64]) -> (bool, String) {
    if text.is_empty() || text.chars().count() > 200 {
        return (false, "empty/verbose".into());
    }
    let mut expected = case.citations.clone();
    expected.sort();
    if expected != cited {
        return (false, "wrong/missing evidence or version citation".into());
    }
    // Citation extraction deliberately recognizes only the supported syntax.
    // The task grader must also reject stray fragments outside that syntax.
    let mut prose = text.to_owned();
    for id in cited {
        prose = prose.replace(&format!("[event:{id}]"), "");
    }
    if prose.contains(['[', ']']) {
        return (false, "broken citation fragment".into());
    }
    if let Some(required) = &case.required {
        if !text.contains(required) {
            return (false, "missing required value".into());
        }
        if case.forbidden.iter().any(|v| text.contains(v))
            || ["알 수 없", "모르", "불확실", "확인할 수 없"]
                .iter()
                .any(|s| text.contains(s))
        {
            return (false, "conflicting value/blanket uncertainty".into());
        }
        if !["입니다", "지시", "이동", "방향", "값"]
            .iter()
            .any(|s| text.contains(s))
        {
            return (false, "not a task answer".into());
        }
    } else {
        if case.confirmed_sequence
            && ["지시", "실행", "사고"]
                .iter()
                .any(|word| !text.contains(word))
        {
            return (false, "missing confirmed event sequence".into());
        }
        if case.request.evidence.items.is_empty() && case.forbidden.iter().any(|v| text.contains(v))
        {
            return (false, "unsupported value with no evidence".into());
        }
        if ![
            "알 수 없",
            "확정되지",
            "확정할 수 없",
            "확인되지",
            "확인할 수 없",
            "불확실",
            "근거가 없",
            "미확인",
        ]
        .iter()
        .any(|s| text.contains(s))
        {
            return (false, "unsupported causal certainty/absent fact".into());
        }
        if ["원인입니다", "때문입니다", "원인은 지시"]
            .iter()
            .any(|s| text.contains(s))
        {
            return (false, "causal overclaim".into());
        }
    }
    (
        true,
        "core value/time/citation rubric; language semantics still reviewed separately".into(),
    )
}
fn evaluate_native(
    checkpoint: &std::path::Path,
    fixture: &std::path::Path,
    mode: &str,
    output: &std::path::Path,
) -> Result<()> {
    use replica_v3::neural::{self, checkpoint as artifacts};
    use std::io::Write;
    if !["trained", "random", "no-evidence", "value-swap"].contains(&mode) {
        return Err(Error::Invalid("evaluation mode".into()));
    }
    let raw = neural::read_bounded(fixture, 8 * 1024 * 1024)?;
    let fixture: Holdout = serde_json::from_slice(&raw)?;
    if ![1, 2].contains(&fixture.version)
        || fixture.cases.len() != 200
        || fixture.value_pairs.len() != 200
    {
        return Err(Error::Invalid("fixed heldout denominator".into()));
    }
    let load_start = Instant::now();
    let loaded = artifacts::load(checkpoint, candle_core::Device::Cpu, false)?;
    let load_ms = elapsed(load_start);
    if (mode == "random") != loaded.manifest.training.is_none() {
        return Err(Error::Invalid("evaluation checkpoint status/mode".into()));
    }
    let config_id = loaded.model.config.id()?;
    let mut log = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(output)?;
    writeln!(
        log,
        "{}",
        serde_json::json!({"header":true,"mode":mode,"fixture_sha256":neural::hash(&raw),"checkpoint_sha256":loaded.manifest.weights_sha256,"tokenizer_sha256":loaded.tokenizer.id(),"architecture":loaded.model.config,"training":loaded.manifest.training,"load_ms":load_ms,"backend":neural::cpu_backend(),"dtype":"F32","rubric":"independent_core_value_time_citations_v2","semantic_review":"separate"})
    )?;
    let cases = if mode == "value-swap" {
        fixture.value_pairs
    } else {
        fixture.cases
    };
    let mut total = [0usize; 5];
    let mut correct = [0usize; 5];
    let mut invalid_citations = 0;
    let mut accepted_invalid_citations = 0;
    let mut failures = 0;
    let mut first_token_ms = Vec::new();
    let mut generation_ms = Vec::new();
    let mut generated_tokens = 0usize;
    for case in cases {
        if case.category >= 5 {
            return Err(Error::Invalid("heldout category".into()));
        }
        total[case.category] += 1;
        let mut request = case.request.clone();
        if mode == "no-evidence" {
            request.evidence.items.clear();
        }
        let start = Instant::now();
        let mut text = None;
        let mut generated = None;
        let mut provided = Vec::new();
        let mut excluded = Vec::new();
        let mut tokens = 0;
        let mut token_digest = None;
        let mut actual_citations = None;
        let result = (|| -> Result<(bool, String)> {
            let prompt = loaded.tokenizer.prepare(
                &request,
                loaded.model.config.context as u32,
                &config_id,
            )?;
            provided = prompt.provided.clone();
            excluded = prompt.excluded.clone();
            tokens = prompt.token_ids.len();
            token_digest = Some(prompt.token_digest);
            let actual = loaded.model.generate(
                &prompt.token_ids,
                request.limits.max_tokens as usize,
                request.limits.timeout_ms,
                &AtomicBool::new(false),
                &format!("heldout-{}", case.id),
            )?;
            let decoded = loaded.tokenizer.decode(&actual.tokens);
            generated = Some(actual);
            let decoded = decoded?;
            text = Some(decoded.clone());
            if generated.as_ref().is_none_or(|g| g.finish != "stop") {
                return Err(Error::Model("generation ended before EOS".into()));
            }
            let cited = match app::citations(&decoded) {
                Ok(ids) => ids,
                Err(e) => {
                    invalid_citations += 1;
                    return Err(e);
                }
            };
            actual_citations = Some(cited.clone());
            if cited.iter().any(|id| !provided.contains(id)) {
                invalid_citations += 1;
                return Err(Error::Model(
                    "citation outside actual provided evidence".into(),
                ));
            }
            Ok(score_answer(&case, &decoded, &cited))
        })();
        let (passed, reason) = match result {
            Ok(r) => r,
            Err(e) => {
                failures += 1;
                (false, e.to_string())
            }
        };
        if passed {
            correct[case.category] += 1;
        }
        let citation_invalid = text.as_ref().is_some_and(|s| {
            !app::citations(s).is_ok_and(|ids| ids.iter().all(|id| provided.contains(id)))
        });
        accepted_invalid_citations += usize::from(passed && citation_invalid);
        if let Some(actual) = &generated {
            first_token_ms.push(actual.first_token_ms as f64);
            generation_ms.push(actual.generation_ms as f64);
            generated_tokens += actual.generated;
        }
        writeln!(
            log,
            "{}",
            serde_json::json!({"case":case.id,"category":case.category,"question":request.input,"evidence":request.evidence,"expected_core_value":case.required,"expected_citations":case.citations,"actual_citations":actual_citations,"provided":provided,"excluded":excluded,"input_tokens":tokens,"input_digest":token_digest,"actual_text":text,"actual_generation":generated,"passed":passed,"reason":reason,"elapsed_ms":elapsed(start)})
        )?;
        log.flush()?;
        println!(
            "mode={mode} case={} category={} passed={passed} reason={reason} text={text:?}",
            case.id, case.category
        );
    }
    if total != [40; 5] {
        return Err(Error::Invalid("heldout category denominator".into()));
    }
    let success: usize = correct.iter().sum();
    let quality =
        success >= 190 && correct.iter().all(|&n| n >= 36) && accepted_invalid_citations == 0;
    let summary = serde_json::json!({"summary":true,"mode":mode,"correct":correct,"total":total,"success":success,"denominator":total.iter().sum::<usize>(),"task_target_pass":quality,"generation_or_validation_failures":failures,"rejected_invalid_citations":invalid_citations,"accepted_invalid_citations":accepted_invalid_citations});
    writeln!(log, "{summary}")?;
    log.sync_all()?;
    println!("{summary}");
    if !generation_ms.is_empty() {
        println!(
            "completed_generations={} generated_tokens_including_eos={} total_generation_ms={:.3}; failed generation timing excluded",
            generation_ms.len(),
            generated_tokens,
            generation_ms.iter().sum::<f64>()
        );
        stats("TTFT", first_token_ms);
        stats("generation", generation_ms);
    }
    if mode == "trained" && !quality {
        return Err(Error::Model(
            "heldout task quality gate failed; all results retained".into(),
        ));
    }
    Ok(())
}
fn retrieval_baseline(fixture: &std::path::Path, output: &std::path::Path) -> Result<()> {
    use std::io::Write;
    let raw = replica_v3::neural::read_bounded(fixture, 8 * 1024 * 1024)?;
    let fixture: Holdout = serde_json::from_slice(&raw)?;
    let mut log = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(output)?;
    let mut matched = 0;
    let mut known = 0;
    for case in fixture.cases {
        let dir = tempfile::tempdir()?;
        let mut store = Store::init(dir.path().join("baseline.db"))?;
        for evidence in &case.request.evidence.items {
            store.append(Event::observation(
                "baseline",
                "heldout",
                "synthetic",
                evidence.original_excerpt.as_bytes().to_vec(),
            ))?;
        }
        let mut search = Search::new("baseline", &case.request.input);
        search.graph = false;
        let bundle = store.search(&search)?;
        let hit = case.required.as_ref().is_some_and(|value| {
            bundle
                .items
                .first()
                .is_some_and(|e| e.original_excerpt.contains(value))
        });
        if case.required.is_some() {
            known += 1;
            matched += usize::from(hit);
        }
        writeln!(
            log,
            "{}",
            serde_json::json!({"case":case.id,"category":case.category,"query":case.request.input,"bundle":bundle,"required":case.required,"top1_value_hit":hit,"neural_generation":false})
        )?;
    }
    writeln!(
        log,
        "{}",
        serde_json::json!({"summary":true,"top1_value_hits":matched,"known_value_cases":known,"metric":"retrieval-only raw evidence, not answer accuracy"})
    )?;
    log.sync_all()?;
    println!("retrieval_only_top1_value_hits={matched}/{known}; not neural answer accuracy");
    Ok(())
}

fn compare_pairs(original: &std::path::Path, swapped: &std::path::Path) -> Result<()> {
    use serde_json::Value;
    let read = |path: &std::path::Path| -> Result<Vec<Value>> {
        let raw = replica_v3::neural::read_bounded(path, 16 * 1024 * 1024)?;
        let lines = std::str::from_utf8(&raw).map_err(|e| Error::Invalid(e.to_string()))?;
        let rows: Vec<Value> = lines
            .lines()
            .map(serde_json::from_str)
            .collect::<std::result::Result<_, _>>()?;
        Ok(rows
            .into_iter()
            .filter(|r| r.get("case").is_some())
            .collect())
    };
    let original = read(original)?;
    let swapped = read(swapped)?;
    if original.len() != 200 || swapped.len() != original.len() {
        return Err(Error::Invalid("paired evaluation count".into()));
    }
    let mut total = 0;
    let mut unchanged = 0;
    let mut both_correct = 0;
    let mut missing_output = 0;
    for (a, b) in original.iter().zip(&swapped) {
        if a["case"] != b["case"] || a["question"] != b["question"] {
            return Err(Error::Invalid("paired evaluation identity".into()));
        }
        if a["expected_core_value"].is_null() {
            continue;
        }
        if a["expected_core_value"] == b["expected_core_value"] {
            return Err(Error::Invalid("paired value was not changed".into()));
        }
        total += 1;
        if a["actual_text"].is_null() || b["actual_text"].is_null() {
            missing_output += 1;
        } else {
            unchanged += usize::from(a["actual_text"] == b["actual_text"]);
        }
        both_correct += usize::from(a["passed"] == true && b["passed"] == true);
    }
    println!(
        "{}",
        serde_json::json!({"value_changed_pairs":total,"identical_nonnull_output":unchanged,"at_least_one_missing_output":missing_output,"both_answers_correct":both_correct,"metric":"paired actual generation; unchanged output cannot demonstrate value binding"})
    );
    Ok(())
}

#[cfg(test)]
mod evaluation_tests {
    use super::*;
    #[test]
    fn fresh_holdout_keeps_versions_pairs_and_confirmed_sequence_independent() {
        let cases = heldout_memory_v2(439_187, false);
        let paired = heldout_memory_v2(439_187, true);
        assert_eq!(cases.len(), 200);
        let mut positions = std::collections::BTreeSet::new();
        let mut id_time_orders = std::collections::BTreeSet::new();
        for category in 0..5 {
            assert_eq!(cases.iter().filter(|c| c.category == category).count(), 40);
        }
        for (case, pair) in cases.iter().zip(&paired) {
            assert_eq!(case.request.input, pair.request.input);
            assert_eq!(case.citations, pair.citations);
            let rows = &case.request.evidence.items;
            assert!(
                case.citations
                    .iter()
                    .all(|id| rows.iter().any(|r| r.event_id == *id))
            );
            for (a, b) in rows.iter().zip(&pair.request.evidence.items) {
                let mut expected = serde_json::to_value(a).unwrap();
                if let Some(value) = &case.required
                    && case.citations.contains(&a.event_id)
                {
                    assert_ne!(case.required, pair.required);
                    expected["original_excerpt"] = a
                        .original_excerpt
                        .replace(value, pair.required.as_ref().unwrap())
                        .into();
                }
                assert_eq!(expected, serde_json::to_value(b).unwrap());
            }
            if case.category == 0 {
                positions.insert(
                    rows.iter()
                        .position(|r| case.citations.contains(&r.event_id))
                        .unwrap(),
                );
                id_time_orders.insert(
                    rows[0].event_id.cmp(&rows[1].event_id)
                        == rows[0].recorded_at.cmp(&rows[1].recorded_at),
                );
            }
            if case.category == 1 {
                let mut chronology: Vec<_> = rows.iter().collect();
                chronology.sort_by_key(|r| r.recorded_at);
                let expected = match case.id % 4 {
                    0 => chronology[1],
                    1 | 3 => chronology[0],
                    _ => chronology[2],
                };
                assert_eq!(case.citations, [expected.event_id]);
                assert!(
                    expected
                        .original_excerpt
                        .contains(case.required.as_ref().unwrap())
                );
                if rows.len() == 3 {
                    assert_eq!(
                        chronology[0].original_excerpt,
                        chronology[2].original_excerpt
                    );
                    assert_eq!(chronology[2].version_status, "current");
                }
            }
            if case.confirmed_sequence {
                assert_eq!(case.citations.len(), 3);
                assert!(!score_answer(case, "원인은 확정되지 않았습니다.", &case.citations).0);
                assert!(
                    score_answer(
                        case,
                        "지시 뒤 실행과 사고가 기록되었습니다. 원인은 확정되지 않았습니다.",
                        &case.citations
                    )
                    .0
                );
                assert!(
                    !score_answer(
                        case,
                        "지시 뒤 실행과 사고가 기록되었습니다. 원인은 확정되지 않았습니다.",
                        &case.citations[..1]
                    )
                    .0
                );
            }
        }
        assert_eq!(positions, [0, 1].into());
        assert_eq!(id_time_orders, [false, true].into());
        assert_eq!(cases.iter().filter(|c| c.confirmed_sequence).count(), 20);
        // A different explicit seed changes facts/IDs; expected fields are never
        // serialized into the ModelRequest passed to the production model.
        let another = heldout_memory_v2(721_919, false);
        assert_ne!(
            serde_json::to_value(&cases[0].request).unwrap(),
            serde_json::to_value(&another[0].request).unwrap()
        );
        assert!(
            serde_json::to_value(&cases[0].request)
                .unwrap()
                .get("required")
                .is_none()
        );
    }
    #[test]
    fn heldout_rubric_rejects_blank_wrong_time_citation_and_blanket_unknown() {
        let cases = heldout(917, false);
        let pairs = heldout(917, true);
        assert_eq!(cases.len(), 200);
        for category in 0..5 {
            assert_eq!(cases.iter().filter(|c| c.category == category).count(), 40);
        }
        for (case, pair) in cases.iter().zip(pairs) {
            assert_eq!(case.request.input, pair.request.input);
            if case.required.is_some() {
                assert_ne!(case.required, pair.required);
            }
            for (original, changed) in case
                .request
                .evidence
                .items
                .iter()
                .zip(&pair.request.evidence.items)
            {
                assert_eq!(original.event_id, changed.event_id);
                if case.required.is_some() && case.citations.contains(&original.event_id) {
                    assert_ne!(original.original_excerpt, changed.original_excerpt);
                } else {
                    assert_eq!(original.original_excerpt, changed.original_excerpt);
                }
            }
            assert!(!score_answer(case, "", &case.citations).0);
            if let Some(value) = &case.required {
                assert!(!score_answer(case, "알 수 없습니다.", &case.citations).0);
                assert!(score_answer(case, &format!("{value}입니다."), &case.citations).0);
                assert!(!score_answer(case, &format!("{value}입니다."), &[999999]).0);
                assert!(
                    !score_answer(
                        case,
                        &format!("{value} {} 입니다.", case.forbidden[0]),
                        &case.citations
                    )
                    .0
                );
            } else {
                assert!(!score_answer(case, "앞선 지시가 원인입니다.", &case.citations).0);
                assert!(score_answer(case, "원인은 확정되지 않았습니다.", &case.citations).0);
                assert!(
                    !score_answer(
                        case,
                        "오른쪽입니다. 수 없습니다. 수 없어 없어 알 수 확정되지 않았습니다284]",
                        &case.citations,
                    )
                    .0
                );
                if case.request.evidence.items.is_empty() {
                    assert!(!score_answer(case, "오른쪽입니다. 원인은 알 수 없습니다.", &[]).0);
                }
            }
        }
    }
}
