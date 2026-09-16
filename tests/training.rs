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
