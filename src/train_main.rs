#![forbid(unsafe_code)]
//! Training-only entrypoint; the product worker never imports the corpus generator.
mod data;
mod training;
use clap::{Parser, Subcommand};
use replica_v3::Result;
use std::path::PathBuf;
#[derive(Parser)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}
#[derive(Subcommand)]
enum Commands {
    Model {
        #[command(subcommand)]
        command: Models,
    },
    Train {
        #[arg(long, required_unless_present = "resume", conflicts_with = "resume")]
        checkpoint: Option<PathBuf>,
        #[arg(long)]
        resume: Option<PathBuf>,
        #[arg(long)]
        corpus: Option<PathBuf>,
        #[arg(long)]
        output: PathBuf,
        #[arg(long)]
        numeric_probe: bool,
        #[arg(long)]
        stop_after: Option<usize>,
        #[arg(long, default_value_t = 5000)]
        steps: usize,
        #[arg(long, default_value_t = 20_000_000)]
        max_tokens: u64,
        #[arg(long, default_value_t = 512)]
        seq_len: usize,
        #[arg(long, default_value_t = 0.001)]
        lr: f64,
        #[arg(long, default_value_t = 100)]
        warmup: usize,
        #[arg(long, default_value_t = 1)]
        microbatch: usize,
        #[arg(long, default_value_t = 4)]
        accumulation: usize,
        #[arg(long, default_value_t = 100)]
        validate_every: usize,
        #[arg(long, default_value_t = 17)]
        seed: u64,
    },
    Corpus {
        #[command(subcommand)]
        command: Corpus,
    },
    Tokenizer {
        #[command(subcommand)]
        command: Tokenizer,
    },
}
#[derive(Subcommand)]
enum Models {
    Init {
        #[arg(long)]
        tokenizer: PathBuf,
        #[arg(long)]
        output: PathBuf,
        #[arg(long,default_value="small",value_parser=["small","tiny"])]
        profile: String,
        #[arg(long)]
        seed: u64,
        #[arg(long)]
        source_id: String,
    },
    Inspect {
        #[arg(long)]
        checkpoint: PathBuf,
    },
}
#[derive(Subcommand)]
enum Corpus {
    Prepare {
        #[arg(long)]
        output: PathBuf,
        #[arg(long, default_value_t = 41)]
        seed: u64,
        #[arg(long, default_value_t = 2000)]
        documents: usize,
        #[arg(long)]
        local: Vec<PathBuf>,
    },
}
#[derive(Subcommand)]
enum Tokenizer {
    Inspect {
        #[arg(long)]
        tokenizer: PathBuf,
        #[arg(long)]
        file: PathBuf,
    },
    Train {
        #[arg(long)]
        corpus: PathBuf,
        #[arg(long)]
        output: PathBuf,
        #[arg(long, default_value_t = 4096)]
        vocab: usize,
    },
}
fn run() -> Result<()> {
    match Cli::parse().command {
        Commands::Model {
            command:
                Models::Init {
                    tokenizer,
                    output,
                    profile,
                    seed,
                    source_id,
                },
        } => {
            use replica_v3::neural::{
                ByteBpe, checkpoint,
                transformer::{Config, Transformer},
            };
            let tok = ByteBpe::load(&tokenizer)?;
            let config = if profile == "tiny" {
                Config::tiny(tok.vocab_size())
            } else {
                Config::small(tok.vocab_size())
            };
            let model = Transformer::init(config, seed, candle_core::Device::Cpu)?;
            let manifest = checkpoint::initialized(&model, &tok, seed, source_id)?;
            let saved = checkpoint::save(&output, &model, &tok, manifest, &Default::default())?;
            println!("{}", serde_json::to_string_pretty(&saved)?);
            Ok(())
        }
        Commands::Model {
            command: Models::Inspect { checkpoint },
        } => {
            let loaded =
                replica_v3::neural::checkpoint::load(&checkpoint, candle_core::Device::Cpu, false)?;
            println!("{}", serde_json::to_string_pretty(&loaded.manifest)?);
            println!(
                "parameters={} actual_weight_hash={}",
                loaded.model.config.parameters(),
                loaded.model.weight_hash()?
            );
            Ok(())
        }
        Commands::Train {
            checkpoint,
            resume,
            corpus,
            output,
            numeric_probe,
            stop_after,
            steps,
            max_tokens,
            seq_len,
            lr,
            warmup,
            microbatch,
            accumulation,
            validate_every,
            seed,
        } => {
            use std::sync::{
                Arc,
                atomic::{AtomicBool, Ordering},
            };
            let cancel = Arc::new(AtomicBool::new(false));
            let signal = cancel.clone();
            ctrlc::set_handler(move || signal.store(true, Ordering::Relaxed))
                .map_err(|e| replica_v3::Error::Model(e.to_string()))?;
            let is_resume = resume.is_some();
            let checkpoint = resume.or(checkpoint).expect("clap required");
            training::train(
                training::Run {
                    checkpoint: &checkpoint,
                    corpus: corpus.as_deref(),
                    output: &output,
                    resume: is_resume,
                    numeric_probe,
                    config: replica_v3::neural::checkpoint::TrainConfig {
                        lr,
                        warmup,
                        max_steps: steps,
                        max_tokens,
                        microbatch,
                        accumulation,
                        seq_len,
                        validate_every,
                        seed,
                        ..Default::default()
                    },
                    stop_after,
                },
                &cancel,
            )
        }
        Commands::Tokenizer {
            command: Tokenizer::Inspect { tokenizer, file },
        } => {
            use replica_v3::neural::{ByteBpe, hash, read_bounded};
            let tok = ByteBpe::load(&tokenizer)?;
            let bytes = read_bounded(&file, 262144)?;
            let ids = tok.encode(&bytes)?;
            let recovered = tok.decode_bytes(&ids)?;
            if recovered != bytes {
                return Err(replica_v3::Error::Corrupt("tokenizer roundtrip".into()));
            }
            println!(
                "{}",
                serde_json::json!({"tokenizer_sha256":tok.id(),"vocab":tok.vocab_size(),"ids":ids,"roundtrip_sha256":hash(&recovered)})
            );
            Ok(())
        }
        Commands::Corpus {
            command:
                Corpus::Prepare {
                    output,
                    seed,
                    documents,
                    local,
                },
        } => data::prepare(&output, seed, documents, &local),
        Commands::Tokenizer {
            command:
                Tokenizer::Train {
                    corpus,
                    output,
                    vocab,
                },
        } => data::tokenizer(&corpus, &output, vocab),
    }
}
fn main() {
    if let Err(e) = run() {
        eprintln!("{e}");
        std::process::exit(1);
    }
}
