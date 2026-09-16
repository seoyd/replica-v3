#![forbid(unsafe_code)]
//! Training-only entrypoint; the product worker never imports the corpus generator.
mod data;
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
