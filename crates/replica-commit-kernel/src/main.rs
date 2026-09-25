use replica_commit_kernel::{run_vector, vectors::load_frozen_bundle};
use std::env;
use std::process::ExitCode;

fn main() -> ExitCode {
    let path = env::args()
        .nth(1)
        .unwrap_or_else(|| "tests/data/commit_kernel_reference_vectors_v1_2B.json".to_string());

    let bundle = match load_frozen_bundle(&path) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("PREFLIGHT_FAIL: {e}");
            return ExitCode::from(2);
        }
    };

    let mut failures = 0usize;
    for vector in &bundle.directed_vectors {
        let got = run_vector(vector);
        let ok = got.journal == vector.expected_journal_v1_2b
            && got.final_state == vector.expected_final_state_v1_2b
            && got.state_sha256 == vector.expected_state_sha256_v1_2b;
        if ok {
            println!("PASS {} state_sha256={}", vector.name, got.state_sha256);
        } else {
            failures += 1;
            println!("FAIL {}", vector.name);
            if got.journal != vector.expected_journal_v1_2b {
                println!(
                    "  journal expected={:?} actual={:?}",
                    vector.expected_journal_v1_2b, got.journal
                );
            }
            if got.final_state != vector.expected_final_state_v1_2b {
                println!(
                    "  final state expected={:?} actual={:?}",
                    vector.expected_final_state_v1_2b, got.final_state
                );
            }
            if got.state_sha256 != vector.expected_state_sha256_v1_2b {
                println!(
                    "  hash mismatch expected={} got={}",
                    vector.expected_state_sha256_v1_2b, got.state_sha256
                );
            }
        }
    }

    println!(
        "vectors={} failures={}",
        bundle.directed_vectors.len(),
        failures
    );
    if failures == 0 {
        ExitCode::SUCCESS
    } else {
        ExitCode::from(1)
    }
}
