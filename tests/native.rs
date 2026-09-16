use replica_v3::{
    Error,
    event::GenerationLimits,
    model::{ModelRequest, SYSTEM},
    neural::*,
    retrieval::{Evidence, EvidenceBundle},
};
fn tokenizer() -> ByteBpe {
    let docs = vec![
        "한글 기록 오른쪽 왼쪽 😀 repeated repeated"
            .as_bytes()
            .to_vec();
        4
    ];
    ByteBpe::train(&docs, &hash(b"independent training fixture"), 512).unwrap()
}
#[test]
fn semantic_identity_and_bounded_experimental_configuration() {
    use transformer::{Config, MIXER, Transformer};
    let tiny = Config::tiny(264);
    let mut experimental = tiny.clone();
    experimental.profile = "NATIVE_TRPP_EXPERIMENTAL_V1".into();
    assert_eq!(
        tiny.semantic_id().unwrap(),
        experimental.semantic_id().unwrap()
    );
    assert_ne!(tiny.id().unwrap(), experimental.id().unwrap());
    experimental.rope_theta = 20000.;
    assert_ne!(
        tiny.semantic_id().unwrap(),
        experimental.semantic_id().unwrap()
    );
    experimental.context = 2049;
    assert!(experimental.validate().is_err());
    let a = Transformer::init(tiny.clone(), 7, candle_core::Device::Cpu).unwrap();
    let mut changed = tiny.clone();
    changed.profile = "NATIVE_TRPP_EXPERIMENTAL_V1".into();
    changed.rope_theta = 20000.;
    let b = Transformer::init(changed, 7, candle_core::Device::Cpu).unwrap();
    assert_eq!(
        a.weights_content_id().unwrap(),
        b.weights_content_id().unwrap()
    );
    let ids = candle_core::Tensor::new(&[[8u32, 9]], &candle_core::Device::Cpu).unwrap();
    let mut cache = a.cache("scope");
    let _ = a.forward_cached(&ids, &mut cache, "scope").unwrap();
    assert_eq!(
        cache.history_id(),
        hash(&[8u32.to_le_bytes(), 9u32.to_le_bytes()].concat())
    );
    assert!(b.forward_cached(&ids, &mut cache, "scope").is_err());
    cache.reset("scope");
    assert_eq!(cache.history_id(), hash(b""));
    assert_eq!(MIXER.state_schema_id, "absolute-kv-history-v1");
    let tok = tokenizer();
    let mut json: serde_json::Value = serde_json::from_slice(tok.bytes()).unwrap();
    json["train_hash"] = hash(b"different provenance").into();
    let other = ByteBpe::from_bytes(&serde_json::to_vec_pretty(&json).unwrap()).unwrap();
    assert_eq!(tok.semantic_id(), other.semantic_id());
    assert_ne!(tok.id(), other.id());
    assert_eq!(
        tok.encode(b"0123 separate words").unwrap(),
        other.encode(b"0123 separate words").unwrap()
    );
}
#[test]
fn rust_decode_gemv_independent_values_bounds_and_model_cache_parity() {
    use candle_core::{Device, Tensor};
    use transformer::{Config, Kernel, Rng, Transformer, decode_linear};
    let cpu = Device::Cpu;
    let x = Tensor::new(&[[[2f32, -3., 4.]]], &cpu).unwrap();
    let w = Tensor::new(&[[1f32, 2., 3.], [-1., 0., 2.]], &cpu).unwrap();
    assert_eq!(
        decode_linear(&x, &w, Kernel::RustDecodeGemv)
            .unwrap()
            .flatten_all()
            .unwrap()
            .to_vec1::<f32>()
            .unwrap(),
        [8., 6.]
    );
    let mut rng = Rng::new(19);
    // Predeclared forward error bound uses f64 dot and sum of magnitudes, including cancellation.
    for (n, k) in [(1, 1), (3, 7), (17, 33), (9, 384), (3, 1025)] {
        let values: Vec<f32> = (0..k)
            .map(|_| (rng.next_u64() % 2001) as f32 / 1000. - 1.)
            .collect();
        let weights: Vec<f32> = (0..n * k)
            .map(|i| {
                if i % 3 == 0 {
                    0.
                } else {
                    (rng.next_u64() % 2001) as f32 - 1000.
                }
            })
            .collect();
        let input = Tensor::from_vec(values.clone(), (1, 1, k), &cpu).unwrap();
        let weight = Tensor::from_vec(weights.clone(), (n, k), &cpu).unwrap();
        for kernel in [Kernel::Reference, Kernel::RustDecodeGemv] {
            let out = decode_linear(&input, &weight, kernel)
                .unwrap()
                .flatten_all()
                .unwrap()
                .to_vec1::<f32>()
                .unwrap();
            for (row, actual) in weights.chunks_exact(k).zip(out) {
                let exact: f64 = row
                    .iter()
                    .zip(&values)
                    .map(|(&a, &b)| f64::from(a) * f64::from(b))
                    .sum();
                let magnitude: f64 = row
                    .iter()
                    .zip(&values)
                    .map(|(&a, &b)| (f64::from(a) * f64::from(b)).abs())
                    .sum();
                let bound = 2. * k as f64 * f64::from(f32::EPSILON) * magnitude + 1e-6;
                assert!((f64::from(actual) - exact).abs() <= bound);
            }
        }
    }
    let zero = Tensor::zeros((1, 1, 3), candle_core::DType::F32, &cpu).unwrap();
    assert_eq!(
        decode_linear(&zero, &w, Kernel::RustDecodeGemv)
            .unwrap()
            .flatten_all()
            .unwrap()
            .to_vec1::<f32>()
            .unwrap(),
        [0., 0.]
    );
    assert!(
        decode_linear(
            &x.to_dtype(candle_core::DType::F64).unwrap(),
            &w,
            Kernel::RustDecodeGemv
        )
        .is_err()
    );
    assert!(decode_linear(&x, &w.t().unwrap(), Kernel::RustDecodeGemv).is_err());
    assert!(
        decode_linear(
            &Tensor::zeros((1, 1, 0), candle_core::DType::F32, &cpu).unwrap(),
            &Tensor::zeros((2, 0), candle_core::DType::F32, &cpu).unwrap(),
            Kernel::RustDecodeGemv
        )
        .is_err()
    );
    let mut model = Transformer::init(Config::tiny(264), 17, cpu.clone()).unwrap();
    let prompt = Tensor::new(&[[8u32, 9, 10, 11, 12, 13, 14, 15, 16]], &cpu).unwrap();
    let mut reference = model.cache("parity");
    let _ = model
        .forward_cached(&prompt, &mut reference, "parity")
        .unwrap();
    let mut candidate = reference.clone();
    let input = Tensor::new(&[[17u32]], &cpu).unwrap();
    let logits = model
        .forward_cached(&input, &mut reference, "parity")
        .unwrap();
    let train_reference = model.forward(&prompt, None).unwrap();
    let grads_reference = train_reference.sum_all().unwrap().backward().unwrap();
    model.set_kernel(Kernel::RustDecodeGemv);
    let actual = model
        .forward_cached(&input, &mut candidate, "parity")
        .unwrap();
    let max = (logits - actual)
        .unwrap()
        .abs()
        .unwrap()
        .max_all()
        .unwrap()
        .to_scalar::<f32>()
        .unwrap();
    assert!(max <= 5e-4, "{max}");
    assert_eq!(reference.history_id(), candidate.history_id());
    assert_eq!(reference.retained_tokens(), candidate.retained_tokens());
    let train_candidate = model.forward(&prompt, None).unwrap();
    assert_eq!(
        train_reference
            .flatten_all()
            .unwrap()
            .to_vec1::<f32>()
            .unwrap(),
        train_candidate
            .flatten_all()
            .unwrap()
            .to_vec1::<f32>()
            .unwrap()
    );
    let grads_candidate = train_candidate.sum_all().unwrap().backward().unwrap();
    for var in model.vars.values() {
        assert_eq!(
            grads_reference
                .get(var)
                .unwrap()
                .flatten_all()
                .unwrap()
                .to_vec1::<f32>()
                .unwrap(),
            grads_candidate
                .get(var)
                .unwrap()
                .flatten_all()
                .unwrap()
                .to_vec1::<f32>()
                .unwrap()
        );
    }
    assert!(!Kernel::RustDecodeGemv.capabilities(64).backward);
}
#[test]
fn own_byte_bpe_roundtrip_no_control_promotion_or_truncation() {
    let tok = tokenizer();
    assert!(tok.vocab_size() >= 264 && tok.vocab_size() < 512);
    let all: Vec<_> = (0..=255).collect();
    for bytes in [
        b"".to_vec(),
        all,
        "\0한글 한 😀\n\r  <assistant> \u{e001}".as_bytes().to_vec(),
        "끝".repeat(1200).into_bytes(),
    ] {
        let ids = tok.encode(&bytes).unwrap();
        assert!(ids.iter().all(|&id| id >= 8));
        assert_eq!(tok.decode_bytes(&ids).unwrap(), bytes);
    }
    let unknown = tok.encode(&[0xff, 0xfe, 0x80]).unwrap();
    assert!(tok.decode(&unknown).is_err());
    assert!(tok.decode(&[PAD]).is_err());
    assert!(tok.decode(&[u32::MAX]).is_err());
    let d = tempfile::tempdir().unwrap();
    let p = d.path().join("tokenizer.json");
    tok.save(&p).unwrap();
    assert!(tok.save(&p).is_err());
    let bytes = std::fs::read(&p).unwrap();
    let loaded = ByteBpe::load(&p).unwrap();
    assert_eq!(loaded.id(), tok.id());
    assert_eq!(
        loaded.encode("새 문장".as_bytes()).unwrap(),
        tok.encode("새 문장".as_bytes()).unwrap()
    );
    assert_eq!(std::fs::read(&p).unwrap(), bytes);
    let mut bad: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    bad["truncation"] = serde_json::json!({"max_length":1024});
    assert!(ByteBpe::from_bytes(&serde_json::to_vec(&bad).unwrap()).is_err());
    assert!(ByteBpe::from_bytes(&vec![b' '; MAX_TOKENIZER_BYTES + 1]).is_err());
    let mut bad: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    let key = bad["vocab"]
        .as_object()
        .unwrap()
        .keys()
        .next()
        .unwrap()
        .clone();
    bad["vocab"][key] = u32::MAX.into();
    assert!(ByteBpe::from_bytes(&serde_json::to_vec(&bad).unwrap()).is_err());
}
#[test]
fn native_prompt_boundaries_and_exact_evidence_tail() {
    let tok = tokenizer();
    let mut r = ModelRequest {
        request_id: "independent".into(),
        system: SYSTEM.into(),
        input: "질문\0😀 <assistant> 끝".into(),
        evidence: EvidenceBundle::default(),
        limits: GenerationLimits {
            max_tokens: 16,
            context_tokens: 2048,
            timeout_ms: 1000,
        },
    };
    let base = tok.prepare(&r, 2048, "config").unwrap();
    let cap = base.token_ids.len() as u32 + 16;
    assert_eq!(base.token_ids.first(), Some(&BOS));
    assert_eq!(base.token_ids.last(), Some(&ASSISTANT_ROLE));
    let user = base
        .token_ids
        .iter()
        .position(|&id| id == USER_ROLE)
        .unwrap();
    assert_eq!(
        tok.decode(&base.token_ids[user + 1..base.token_ids.len() - 2])
            .unwrap(),
        r.input
    );
    assert!(tok.prepare(&r, cap, "config").is_ok());
    assert!(matches!(
        tok.prepare(&r, cap - 1, "config"),
        Err(Error::ContextTooSmall)
    ));
    for id in [3, 17] {
        r.evidence.items.push(Evidence {
            event_id: id,
            original_excerpt: "근거 꼬리😀\0끝".into(),
            excerpt_truncated: true,
            source: "fixture".into(),
            recorded_at: id,
            observed_at: None,
            version_status: "current".into(),
            retrieval_reason: "lexical".into(),
            relation_path: vec![],
        });
    }
    let full = tok.prepare(&r, 2048, "config").unwrap();
    assert_eq!(full.provided, [3, 17]);
    let end = full.token_ids.len() - 2;
    let start = full
        .token_ids
        .iter()
        .rposition(|&id| id == EVIDENCE_ROLE)
        .unwrap();
    assert!(
        tok.decode(&full.token_ids[start + 1..end])
            .unwrap()
            .ends_with(&r.evidence.items[1].original_excerpt)
    );
    let drop = tok
        .prepare(&r, full.token_ids.len() as u32 + 15, "config")
        .unwrap();
    assert_eq!(drop.provided, [3]);
    assert_eq!(drop.excluded, [17]);
    let none = tok.prepare(&r, cap, "config").unwrap();
    assert!(none.provided.is_empty());
    assert_eq!(none.excluded, [17, 3]);
    assert_eq!(tok.bytes(), tokenizer().bytes());
}

#[test]
fn native_numeric_references_and_causal_padding_gradients() {
    use candle_core::{DType, Device, Tensor};
    use replica_v3::neural::transformer::*;
    let device = Device::Cpu;
    let x = Tensor::new(&[1f32, 2., 3., 4.], &device).unwrap();
    let w = Tensor::ones(4, DType::F32, &device).unwrap();
    let actual = rms_norm(&x, &w, 1e-6).unwrap().to_vec1::<f32>().unwrap();
    for (i, v) in actual.iter().enumerate() {
        let expected = (i + 1) as f64 / (7.5f64 + 1e-6).sqrt();
        assert!((*v as f64 - expected).abs() < 1e-6);
    }
    let rotary_input = x.reshape((1, 1, 1, 4)).unwrap();
    let rotated = rotary(&rotary_input, 1, 10000.)
        .unwrap()
        .flatten_all()
        .unwrap()
        .to_vec1::<f32>()
        .unwrap();
    let expected = [
        1f64.cos() - 2. * 1f64.sin(),
        1f64.sin() + 2. * 1f64.cos(),
        3. * 0.01f64.cos() - 4. * 0.01f64.sin(),
        3. * 0.01f64.sin() + 4. * 0.01f64.cos(),
    ];
    for (a, b) in rotated.iter().zip(expected) {
        assert!((*a as f64 - b).abs() < 1e-6);
    }
    let kv = Tensor::new(&[10f32, 20.], &device)
        .unwrap()
        .reshape((1, 2, 1, 1))
        .unwrap();
    assert_eq!(
        repeat_kv(&kv, 4)
            .unwrap()
            .flatten_all()
            .unwrap()
            .to_vec1::<f32>()
            .unwrap(),
        [10., 10., 20., 20.]
    );
    assert!(repeat_kv(&kv, 3).is_err());
    let m = Transformer::init(Config::tiny(264), 17, device.clone()).unwrap();
    let ids = Tensor::new(&[[8u32, 9, 10, 11, 12, 13]], &device).unwrap();
    let mut changed = vec![8u32, 9, 10, 99, 98, 97];
    let later = Tensor::new(changed.as_slice(), &device)
        .unwrap()
        .unsqueeze(0)
        .unwrap();
    let a = m.forward(&ids, None).unwrap();
    let b = m.forward(&later, None).unwrap();
    assert!(max_error(&a.narrow(1, 0, 3).unwrap(), &b.narrow(1, 0, 3).unwrap()) < 1e-6);
    let padding = [true, true, false, true, true, true];
    let padded_change = Tensor::new(&[[8u32, 9, 98, 11, 12, 13]], &device).unwrap();
    assert!(
        max_error(
            &m.forward(&ids, Some(&padding))
                .unwrap()
                .narrow(1, 3, 3)
                .unwrap(),
            &m.forward(&padded_change, Some(&padding))
                .unwrap()
                .narrow(1, 3, 3)
                .unwrap()
        ) < 1e-6
    );
    assert!(
        m.forward(&ids, Some(&[false; 6]))
            .unwrap()
            .flatten_all()
            .unwrap()
            .to_vec1::<f32>()
            .unwrap()
            .iter()
            .all(|v| v.is_finite())
    );
    changed[0] = 264;
    assert!(
        m.forward(
            &Tensor::new(changed.as_slice(), &device)
                .unwrap()
                .unsqueeze(0)
                .unwrap(),
            None
        )
        .is_err()
    );
    let targets = Tensor::new(&[[9u32, 10, 11, 12, 13, 14]], &device).unwrap();
    let mask = Tensor::new(&[[0f32, 0., 1., 1., 1., 0.]], &device).unwrap();
    let (loss, count) = masked_loss(&a, &targets, &mask).unwrap();
    assert_eq!(count, 3);
    assert!(loss.to_scalar::<f32>().unwrap().is_finite());
    let grads = loss.backward().unwrap();
    for name in [
        "embedding",
        "layer.0.q",
        "layer.0.k",
        "layer.0.v",
        "layer.0.gate",
        "layer.0.up",
        "layer.0.down",
    ] {
        let var = &m.vars[name];
        let gradient = grads
            .get(var)
            .unwrap()
            .flatten_all()
            .unwrap()
            .to_vec1::<f32>()
            .unwrap();
        assert!(gradient.iter().all(|g| g.is_finite()));
        assert!(gradient.iter().any(|g| g.abs() > 1e-10), "{name}");
    }
    for name in ["embedding", "layer.0.q", "layer.0.down"] {
        let var = &m.vars[name];
        let gradient = grads
            .get(var)
            .unwrap()
            .flatten_all()
            .unwrap()
            .to_vec1::<f32>()
            .unwrap();
        let index = gradient
            .iter()
            .enumerate()
            .max_by(|a, b| a.1.abs().total_cmp(&b.1.abs()))
            .unwrap()
            .0;
        let original = var.flatten_all().unwrap().to_vec1::<f32>().unwrap();
        let mut values = original.clone();
        let epsilon = 0.001;
        values[index] += epsilon;
        var.set(&Tensor::from_vec(values, var.dims(), &device).unwrap())
            .unwrap();
        let plus = masked_loss(&m.forward(&ids, None).unwrap(), &targets, &mask)
            .unwrap()
            .0
            .to_scalar::<f32>()
            .unwrap();
        let mut values = original.clone();
        values[index] -= epsilon;
        var.set(&Tensor::from_vec(values, var.dims(), &device).unwrap())
            .unwrap();
        let minus = masked_loss(&m.forward(&ids, None).unwrap(), &targets, &mask)
            .unwrap()
            .0
            .to_scalar::<f32>()
            .unwrap();
        var.set(&Tensor::from_vec(original, var.dims(), &device).unwrap())
            .unwrap();
        assert!(
            ((plus - minus) / (2. * epsilon) - gradient[index]).abs() < 0.005,
            "finite difference {name}"
        );
    }
}
fn max_error(a: &candle_core::Tensor, b: &candle_core::Tensor) -> f32 {
    a.flatten_all()
        .unwrap()
        .to_vec1::<f32>()
        .unwrap()
        .into_iter()
        .zip(b.flatten_all().unwrap().to_vec1::<f32>().unwrap())
        .map(|(a, b)| (a - b).abs())
        .fold(0., f32::max)
}
#[test]
fn native_kv_chunk_rollover_parity_reset_and_identity() {
    use candle_core::{Device, Tensor};
    use replica_v3::neural::transformer::*;
    let m = Transformer::init(Config::tiny(264), 19, Device::Cpu).unwrap();
    for len in [7, 8, 9, 17, 33, 64] {
        let ids: Vec<u32> = (8..8 + len).map(|v| v as u32).collect();
        let full = m
            .forward(
                &Tensor::new(ids.as_slice(), &Device::Cpu)
                    .unwrap()
                    .unsqueeze(0)
                    .unwrap(),
                None,
            )
            .unwrap();
        for chunk in [1, 7, 8, 9, 32] {
            let mut cache = m.cache("scope-a");
            let mut parts = Vec::new();
            for part in ids.chunks(chunk) {
                parts.push(
                    m.forward_cached(
                        &Tensor::new(part, &Device::Cpu)
                            .unwrap()
                            .unsqueeze(0)
                            .unwrap(),
                        &mut cache,
                        "scope-a",
                    )
                    .unwrap(),
                );
            }
            let cached = Tensor::cat(&parts, 1).unwrap();
            let error = max_error(&full, &cached);
            assert!(error < 2e-5, "len={len} chunk={chunk} error={error}");
            assert_eq!(cache.retained_tokens(), [len.min(8), len]);
            assert_eq!(cache.bytes(), (len.min(8) + len) * 2 * 2 * 8 * 4);
            let single = Tensor::new(&[[8u32]], &Device::Cpu).unwrap();
            assert!(m.forward_cached(&single, &mut cache, "scope-b").is_err());
            cache.reset("scope-b");
            assert_eq!(cache.bytes(), 0);
            assert!(
                max_error(
                    &m.forward_cached(&single, &mut cache, "scope-b").unwrap(),
                    &m.forward(&single, None).unwrap()
                ) < 1e-6
            );
            let other = Transformer::init(Config::tiny(264), 20, Device::Cpu).unwrap();
            assert!(
                other
                    .forward_cached(&single, &mut cache, "scope-b")
                    .is_err()
            );
        }
    }
}
#[test]
fn legacy_checkpoint_import_roundtrip_and_corruption_rejection() {
    use candle_core::Device;
    use replica_v3::neural::{checkpoint, transformer::*};
    let tok = tokenizer();
    let model = Transformer::init(Config::tiny(tok.vocab_size()), 27, Device::Cpu).unwrap();
    let d = tempfile::tempdir().unwrap();
    let p = d.path().join("checkpoint");
    let manifest = checkpoint::initialized(&model, &tok, 27, hash(b"fixture-source")).unwrap();
    let original =
        checkpoint::legacy_save(&p, &model, &tok, manifest, &Default::default()).unwrap();
    assert!(
        checkpoint::legacy_save(&p, &model, &tok, original.clone(), &Default::default()).is_err()
    );
    let loaded = checkpoint::legacy_load(&p, Device::Cpu, false).unwrap();
    assert_eq!(
        loaded.model.weight_hash().unwrap(),
        model.weight_hash().unwrap()
    );
    let path = p.join("weights.safetensors");
    let bytes = std::fs::read(&path).unwrap();
    std::fs::write(&path, &bytes[..bytes.len() - 1]).unwrap();
    assert!(checkpoint::legacy_load(&p, Device::Cpu, false).is_err());
    std::fs::write(&path, &bytes).unwrap();
    let mut broken = original;
    broken.tokenizer_sha256 = hash(b"wrong tokenizer");
    std::fs::write(
        p.join("manifest.json"),
        serde_json::to_vec(&broken).unwrap(),
    )
    .unwrap();
    assert!(checkpoint::legacy_load(&p, Device::Cpu, false).is_err());
}

#[test]
fn native_local_global_mask_and_greedy_generation_boundaries() {
    use candle_core::{Device, Tensor};
    use replica_v3::neural::transformer::*;
    use std::sync::atomic::AtomicBool;
    let device = Device::Cpu;
    let local = attention_mask(2..5, 0..5, Some(2), None, 1, &device)
        .unwrap()
        .reshape((3, 5))
        .unwrap()
        .to_vec2::<f32>()
        .unwrap();
    assert_eq!(
        local,
        vec![
            vec![0., 1., 1., 0., 0.],
            vec![0., 0., 1., 1., 0.],
            vec![0., 0., 0., 1., 1.]
        ]
    );
    let global = attention_mask(2..5, 0..5, None, None, 1, &device)
        .unwrap()
        .reshape((3, 5))
        .unwrap()
        .to_vec2::<f32>()
        .unwrap();
    assert_eq!(
        global,
        vec![
            vec![1., 1., 1., 0., 0.],
            vec![1., 1., 1., 1., 0.],
            vec![1., 1., 1., 1., 1.]
        ]
    );
    let mask = attention_mask(0..3, 0..3, None, Some(&[true, false, true]), 1, &device)
        .unwrap()
        .reshape((3, 3))
        .unwrap()
        .to_vec2::<f32>()
        .unwrap();
    assert_eq!(
        mask,
        vec![vec![1., 0., 0.], vec![0., 0., 0.], vec![1., 0., 1.]]
    );
    let model = Transformer::init(Config::tiny(264), 11, device.clone()).unwrap();
    let input = vec![8u32, 9, 10];
    let mut full = input.clone();
    let mut expected = Vec::new();
    for _ in 0..3 {
        let logits = model
            .forward(
                &Tensor::new(full.as_slice(), &device)
                    .unwrap()
                    .unsqueeze(0)
                    .unwrap(),
                None,
            )
            .unwrap();
        let id = logits
            .narrow(1, full.len() - 1, 1)
            .unwrap()
            .flatten_all()
            .unwrap()
            .argmax(0)
            .unwrap()
            .to_scalar::<u32>()
            .unwrap();
        if id == EOS {
            break;
        }
        assert!(id >= 8);
        expected.push(id);
        full.push(id);
    }
    let generated = model
        .generate(&input, 3, 1000, &AtomicBool::new(false), "test")
        .unwrap();
    assert_eq!(generated.tokens, expected);
    let boundary = vec![8u32; 63];
    assert!(!matches!(
        model.generate(&boundary, 1, 1000, &AtomicBool::new(false), "test"),
        Err(Error::ContextTooSmall)
    ));
    assert!(matches!(
        model.generate(&boundary, 2, 1000, &AtomicBool::new(false), "test"),
        Err(Error::ContextTooSmall)
    ));
    assert!(matches!(
        model.generate(&input, 1, 1000, &AtomicBool::new(true), "test"),
        Err(Error::Cancelled)
    ));
    assert!(
        model
            .generate(&[], 1, 1000, &AtomicBool::new(false), "test")
            .is_err()
    );
}

#[test]
fn legacy_checkpoint_import_rejects_self_checksummed_nan_unknown_tensor_and_precision() {
    use candle_core::{DType, Device, Tensor};
    use replica_v3::neural::{checkpoint, transformer::*};
    let tok = tokenizer();
    let model = Transformer::init(Config::tiny(tok.vocab_size()), 37, Device::Cpu).unwrap();
    let d = tempfile::tempdir().unwrap();
    let p = d.path().join("checkpoint");
    let manifest = checkpoint::legacy_save(
        &p,
        &model,
        &tok,
        checkpoint::initialized(&model, &tok, 37, hash(b"fixture-source")).unwrap(),
        &Default::default(),
    )
    .unwrap();
    let base: std::collections::HashMap<String, Tensor> = model
        .vars
        .iter()
        .map(|(k, v)| (format!("model.{k}"), v.as_detached_tensor()))
        .collect();
    for corruption in ["nan", "unknown", "missing", "precision", "shape"] {
        let mut tensors = base.clone();
        match corruption {
            "nan" => {
                let mut v = tensors["model.final_norm"].to_vec1::<f32>().unwrap();
                v[0] = f32::NAN;
                tensors.insert(
                    "model.final_norm".into(),
                    Tensor::new(v.as_slice(), &Device::Cpu).unwrap(),
                );
            }
            "unknown" => {
                tensors.insert(
                    "unknown".into(),
                    Tensor::new(&[1f32], &Device::Cpu).unwrap(),
                );
            }
            "missing" => {
                tensors.remove("model.final_norm");
            }
            "precision" => {
                tensors.insert(
                    "model.final_norm".into(),
                    tensors["model.final_norm"].to_dtype(DType::F16).unwrap(),
                );
            }
            _ => {
                tensors.insert(
                    "model.final_norm".into(),
                    Tensor::new(&[1f32], &Device::Cpu).unwrap(),
                );
            }
        }
        let path = p.join("weights.safetensors");
        candle_core::safetensors::save(&tensors, &path).unwrap();
        let bytes = std::fs::read(&path).unwrap();
        let mut m = manifest.clone();
        m.weights_sha256 = hash(&bytes);
        m.weights_bytes = bytes.len();
        std::fs::write(p.join("manifest.json"), serde_json::to_vec(&m).unwrap()).unwrap();
        assert!(
            checkpoint::legacy_load(&p, Device::Cpu, false).is_err(),
            "{corruption}"
        );
    }
}
