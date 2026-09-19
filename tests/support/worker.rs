//! Explicit test double, excluded from the default/release product.
use replica_v3::{event::*, model::*};
use std::{io::Write, time::Duration};
fn main() {
    let mode = std::env::args().nth(1).expect("test mode");
    if mode == "early" {
        return;
    }
    if mode == "load_timeout" {
        std::thread::sleep(Duration::from_secs(10));
        return;
    }
    let mut out = std::io::stdout().lock();
    if mode == "invalid" {
        out.write_all(&3u32.to_le_bytes()).unwrap();
        out.write_all(b"bad").unwrap();
        return;
    }
    if mode == "oversize" {
        out.write_all(&u32::MAX.to_le_bytes()).unwrap();
        return;
    }
    write_frame(&mut out, &replica_v3::binary::record!({"ready":true}), MAX_RESPONSE).unwrap();
    let req: ModelRequest = read_frame(&mut std::io::stdin().lock(), MAX_REQUEST).unwrap();
    if mode == "timeout" {
        std::thread::sleep(Duration::from_secs(10));
        return;
    }
    if mode == "stderr" {
        let mut err = std::io::stderr().lock();
        for _ in 0..512 {
            err.write_all(&[b'x'; 4096]).unwrap();
        }
    }
    let res = ModelResponse {
        prepared: None,
        request_id: req.request_id,
        text: req.input,
        provided: req.evidence.items.iter().map(|e| e.event_id).collect(),
        excluded: Vec::new(),
        generation: GenerationInfo {
            model_id: "TEST_DOUBLE".into(),
            model_revision: "TEST_ONLY".into(),
            runtime_revision: "TEST_ONLY".into(),
            quantization: "NONE_TEST".into(),
            license: "TEST_ONLY".into(),
            finish_reason: "stop".into(),
            input_tokens: None,
            output_tokens: None,
            limits: req.limits,
            load_ms: 0,
            first_token_ms: None,
            generation_ms: 0,
        },
    };
    write_frame(&mut out, &res, MAX_RESPONSE).unwrap();
}
