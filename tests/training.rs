use std::process::Command;
#[test]
fn corpus_and_tokenizer_use_train_only_and_reject_split_leakage() {
    let d = tempfile::tempdir().unwrap();
    let corpus = d.path().join("corpus");
    let tokenizer = d.path().join("tokenizer.json");
    let run = |args: &[&str]| {
        Command::new(env!("CARGO_BIN_EXE_replica-train"))
            .args(args)
            .output()
            .unwrap()
    };
    let out = run(&[
        "corpus",
        "prepare",
        "--output",
        corpus.to_str().unwrap(),
        "--documents",
        "100",
        "--seed",
        "917",
    ]);
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let out = run(&[
        "tokenizer",
        "train",
        "--corpus",
        corpus.to_str().unwrap(),
        "--output",
        tokenizer.to_str().unwrap(),
    ]);
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let tok = replica_v3::neural::ByteBpe::load(&tokenizer).unwrap();
    let input = d.path().join("raw");
    let raw = "새 문장\0😀 <assistant>\n".as_bytes();
    std::fs::write(&input, raw).unwrap();
    let out = run(&[
        "tokenizer",
        "inspect",
        "--tokenizer",
        tokenizer.to_str().unwrap(),
        "--file",
        input.to_str().unwrap(),
    ]);
    assert!(out.status.success());
    let json: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(json["ids"], serde_json::json!(tok.encode(raw).unwrap()));
    assert_eq!(json["roundtrip_sha256"], replica_v3::neural::hash(raw));
    let train = std::fs::read(corpus.join("train.json")).unwrap();
    assert_eq!(tok.train_hash, replica_v3::neural::hash(&train));
    let mut manifest: serde_json::Value =
        serde_json::from_slice(&std::fs::read(corpus.join("manifest.json")).unwrap()).unwrap();
    manifest["validation"] = manifest["train"].clone();
    std::fs::write(
        corpus.join("manifest.json"),
        serde_json::to_vec(&manifest).unwrap(),
    )
    .unwrap();
    let next = d.path().join("wrong-tokenizer.json");
    let out = run(&[
        "tokenizer",
        "train",
        "--corpus",
        corpus.to_str().unwrap(),
        "--output",
        next.to_str().unwrap(),
    ]);
    assert!(!out.status.success());
    assert!(!next.exists());
    assert!(String::from_utf8_lossy(&out.stderr).contains("leakage"));
}

#[test]
fn native_training_resume_is_identical_in_fresh_processes() {
    use candle_core::Device;
    use replica_v3::neural::{
        ByteBpe, checkpoint, hash,
        transformer::{Config, Transformer},
    };
    let d = tempfile::tempdir().unwrap();
    let initial = d.path().join("initial");
    let tok = ByteBpe::train(
        &["가 나 다 오른쪽 왼쪽".as_bytes().to_vec(); 1],
        &hash(b"numeric training fixture"),
        300,
    )
    .unwrap();
    let model = Transformer::init(Config::tiny(tok.vocab_size()), 73, Device::Cpu).unwrap();
    checkpoint::save(
        &initial,
        &model,
        &tok,
        checkpoint::initialized(&model, &tok, 73, hash(b"fixture source")).unwrap(),
        &Default::default(),
    )
    .unwrap();
    let full = d.path().join("full");
    let partial = d.path().join("partial");
    let resumed = d.path().join("resumed");
    let run = |args: &[&str]| {
        let out = Command::new(env!("CARGO_BIN_EXE_replica-train"))
            .args(args)
            .output()
            .unwrap();
        assert!(
            out.status.success(),
            "{}\n{}",
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        );
        out
    };
    let common = [
        "--numeric-probe",
        "--seq-len",
        "64",
        "--steps",
        "6",
        "--warmup",
        "0",
        "--accumulation",
        "2",
        "--validate-every",
        "6",
        "--lr",
        "0.002",
    ];
    let mut args = vec![
        "train",
        "--checkpoint",
        initial.to_str().unwrap(),
        "--output",
        full.to_str().unwrap(),
    ];
    args.extend(common);
    run(&args);
    args[4] = partial.to_str().unwrap();
    args.extend(["--stop-after", "3"]);
    run(&args);
    let resume_path = partial.join("final");
    run(&[
        "train",
        "--resume",
        resume_path.to_str().unwrap(),
        "--output",
        resumed.to_str().unwrap(),
        "--numeric-probe",
    ]);
    let a = checkpoint::load(&full.join("final"), Device::Cpu, true).unwrap();
    let b = checkpoint::load(&resumed.join("final"), Device::Cpu, true).unwrap();
    assert_eq!(a.manifest.weights_sha256, b.manifest.weights_sha256);
    let a_state = a.manifest.training.unwrap();
    let b_state = b.manifest.training.unwrap();
    assert_eq!(a_state.step, 6);
    assert_eq!(a_state.sampler_state, b_state.sampler_state);
    assert_eq!(a_state.consumed_tokens, b_state.consumed_tokens);
    assert_eq!(a_state.target_tokens, b_state.target_tokens);
    assert_eq!(a_state.validation_loss, b_state.validation_loss);
    assert_ne!(a.model.weight_hash().unwrap(), model.weight_hash().unwrap());
}

#[test]
fn native_training_cancel_keeps_optimizer_boundary_checkpoint() {
    use candle_core::Device;
    use replica_v3::neural::{
        ByteBpe, checkpoint, hash,
        transformer::{Config, Transformer},
    };
    use std::{
        io::{BufRead, BufReader},
        process::Stdio,
        time::Duration,
    };
    let d = tempfile::tempdir().unwrap();
    let initial = d.path().join("initial");
    let output = d.path().join("cancelled");
    let tok = ByteBpe::train(
        &["가 나 다 오른쪽 왼쪽".as_bytes().to_vec(); 1],
        &hash(b"numeric training fixture"),
        300,
    )
    .unwrap();
    let model = Transformer::init(Config::tiny(tok.vocab_size()), 73, Device::Cpu).unwrap();
    checkpoint::save(
        &initial,
        &model,
        &tok,
        checkpoint::initialized(&model, &tok, 73, hash(b"fixture source")).unwrap(),
        &Default::default(),
    )
    .unwrap();
    let mut child = Command::new(env!("CARGO_BIN_EXE_replica-train"))
        .args([
            "train",
            "--checkpoint",
            initial.to_str().unwrap(),
            "--output",
            output.to_str().unwrap(),
            "--numeric-probe",
            "--seq-len",
            "64",
            "--accumulation",
            "1",
        ])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let reader = child.stdout.take().unwrap();
    let (send, receive) = std::sync::mpsc::channel();
    let observer = std::thread::spawn(move || {
        for line in BufReader::new(reader).lines() {
            let line = line.unwrap();
            if line.starts_with("step=") {
                let _ = send.send(());
            }
        }
    });
    if receive.recv_timeout(Duration::from_secs(10)).is_err() {
        child.kill().unwrap();
        child.wait().unwrap();
        panic!("no actual optimizer step observed");
    }
    assert!(
        Command::new("kill")
            .args(["-INT", &child.id().to_string()])
            .status()
            .unwrap()
            .success()
    );
    let result = child.wait_with_output().unwrap();
    observer.join().unwrap();
    assert!(!result.status.success());
    assert!(String::from_utf8_lossy(&result.stderr).contains("cancelled"));
    let restored = checkpoint::load(&output.join("final"), Device::Cpu, true).unwrap();
    assert_eq!(restored.manifest.status, "CANCELLED");
    assert!(restored.manifest.training.unwrap().step >= 1);
    assert!(!restored.optimizer.is_empty());
}
