//! Explicit release measurements and real-model smoke. Not run by cargo test.
#![forbid(unsafe_code)]
use replica_v3::{
    Error, Result, app,
    codec::Compression,
    event::*,
    model::{self, LocalModel, ModelConfig},
    retrieval::Search,
    store::Store,
};
use std::{path::PathBuf, process::Command, sync::atomic::AtomicBool, time::Instant};
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
        drop(store);
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
fn config(args: &[String]) -> Result<ModelConfig> {
    if args.len() != 3 {
        return Err(Error::Invalid(
            "smoke MODEL.gguf TOKENIZER.json TOKENIZER_CONFIG.json".into(),
        ));
    }
    Ok(ModelConfig {
        model: PathBuf::from(&args[0]),
        tokenizer: PathBuf::from(&args[1]),
        tokenizer_config: PathBuf::from(&args[2]),
    })
}
fn smoke(config: ModelConfig) -> Result<()> {
    let root = tempfile::tempdir()?;
    let path = root.path().join("smoke.db");
    let mut store = Store::init(&path)?;
    for text in [
        "공장 보라터널 갈림길에서 오른쪽으로 가라는 지시를 받았다.",
        "공장 보라터널 이동 지시는 나중에 취소되었다.",
        "공장 보라터널 사고의 원인은 아직 조사되지 않았다.",
    ] {
        store.append(Event::observation(
            "smoke",
            "prior-session",
            "synthetic-user",
            text.as_bytes().to_vec(),
        ))?;
    }
    let queries = [
        "공장 보라터널 갈림길에서 어떤 지시를 받았나?",
        "공장 보라터널 이동 지시는 취소되었나?",
        "공장 보라터널 사고의 원인이 오른쪽 지시라고 확정할 수 있나?",
        "공장 보라터널 관련 기록의 근거를 인용해줘.",
        "공장 보라터널에서 알려지지 않은 사실은 무엇인가?",
    ];
    for (i, query) in queries.iter().enumerate() {
        drop(store);
        store = Store::open(&path)?;
        let mut input =
            Event::observation("smoke", "new-session", "user", query.as_bytes().to_vec());
        input.request_key = Some([(i + 1) as u8; 16]);
        let answer = app::ask(
            &mut store,
            input,
            GenerationLimits::default(),
            &mut LocalModel {
                config: config.clone(),
            },
            &AtomicBool::new(false),
        )?;
        println!(
            "QUERY {}: {}\nANSWER {}: {}\nPROVENANCE {:?}",
            i + 1,
            query,
            answer.id,
            String::from_utf8_lossy(&answer.payload),
            answer.kind
        );
        store.doctor(true)?;
        if let Kind::AssistantAnswer { evidence, .. } = answer.kind
            && evidence.is_empty()
        {
            return Err(Error::Model(
                "smoke requires an actual source citation".into(),
            ));
        }
    }
    println!("FIVE_RESPONSES_RECORDED; semantic quality and causality require human inspection");
    Ok(())
}
fn run() -> Result<()> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    match args.first().map(String::as_str) {
        Some("measure") => measure(),
        Some("smoke") => smoke(config(&args[1..])?),
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
            "usage: validate measure | smoke MODEL TOKENIZER TOKENIZER_CONFIG".into(),
        )),
    }
}
fn main() {
    if let Err(error) = run() {
        eprintln!("{error}");
        std::process::exit(1);
    }
}
