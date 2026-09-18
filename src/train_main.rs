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
    /// Frozen, bounded quality recovery diagnostics; never a product answer path.
    Recovery {
        /// Explicit historical JSON controls; native commands need no legacy reader.
        #[arg(long)]
        legacy_json: bool,
        #[command(subcommand)]
        command: training::recovery::Command,
    },
    /// Bounded sixteen-case diagnostic; never part of the product response path.
    Contrast {
        #[command(subcommand)]
        command: Contrast,
    },
    /// Replay recorded sampler state to distinguish corpus membership from actual exposure.
    SamplingExposure {
        #[arg(long)]
        start: PathBuf,
        #[arg(long)]
        end: PathBuf,
        #[arg(long)]
        corpus: PathBuf,
        #[arg(long, default_value_t = 400)]
        limit: usize,
    },
    /// Corpus generation diagnostic; never a final heldout score.
    Evaluate {
        #[arg(long)]
        checkpoint: PathBuf,
        #[arg(long)]
        corpus: PathBuf,
        #[arg(long)]
        output: PathBuf,
        #[arg(long, default_value_t = 200)]
        limit: usize,
        #[arg(long, default_value = "validation", value_parser = ["train", "validation"])]
        split: String,
        /// Diagnostic only: paraphrase QA0/QA2 with known training wording, never a task score.
        #[arg(long)]
        known_question_form: bool,
        /// Oracle field-task paraphrase diagnostic only; never a candidate score.
        #[arg(long, conflicts_with = "known_question_form")]
        known_field_question_form: bool,
        /// Oracle value-task evidence selection diagnostic, never a task score.
        #[arg(long, conflicts_with = "known_question_form")]
        single_current_record: bool,
        /// Oracle support-only diagnostic on ordinary QA0/QA2; never a task score.
        #[arg(long, conflicts_with_all = ["known_field_question_form", "single_current_record"])]
        single_qa_record: bool,
    },
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
        /// For restricted environments denying ps; report None and measure externally.
        #[arg(long)]
        no_rss: bool,
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
        /// Draw complete consecutive blocks from the eligible sample pool.
        #[arg(long, default_value_t = 1)]
        sample_group_size: usize,
        #[arg(long, default_value_t = 4)]
        accumulation: usize,
        #[arg(long, default_value_t = 100)]
        validate_every: usize,
        #[arg(long, default_value_t = 17)]
        seed: u64,
        #[arg(long, default_value_t = 0)]
        curriculum_steps: usize,
        /// Optimize the first supervised token more strongly; reported CE stays unweighted.
        #[arg(long, default_value_t = 1.0)]
        first_target_weight: f64,
        /// Explicit new bounded run; retains optimizer/RNG and restarts the saved LR schedule.
        #[arg(long, requires_all=["resume", "source_id"])]
        extend_steps: Option<usize>,
        /// Change microbatch only at an explicitly declared continuation boundary.
        #[arg(long, requires = "extend_steps")]
        extend_microbatch: Option<usize>,
        #[arg(long, requires = "extend_steps", value_parser = clap::value_parser!(u8).range(1..=8))]
        extend_sample_group_size: Option<u8>,
        /// Number of initial copy-training steps in the explicitly extended run.
        #[arg(long, requires = "extend_steps")]
        extend_curriculum_steps: Option<usize>,
        #[arg(long, requires = "extend_steps")]
        extend_first_target_weight: Option<f64>,
        /// Explicit LR policy at a new continuation boundary; Adam/RNG remain intact.
        #[arg(long, requires = "extend_steps")]
        extend_lr: Option<f64>,
        #[arg(long, requires = "extend_steps")]
        extend_warmup: Option<usize>,
        #[arg(long, requires = "extend_steps")]
        source_id: Option<String>,
        /// Explicit continued training on a new declared split; preserves tokenizer/data lineage.
        #[arg(long, requires_all=["resume", "extend_steps", "corpus"], conflicts_with="numeric_probe")]
        replace_corpus: bool,
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
enum Contrast {
    /// One-factor changes of the frozen training inputs; no heldout evaluation or learning.
    Transfer {
        #[arg(long)]
        fixture: PathBuf,
        #[arg(long)]
        checkpoint: PathBuf,
    },
    Freeze {
        #[arg(long)]
        corpus: PathBuf,
        #[arg(long)]
        log: PathBuf,
        #[arg(long)]
        tokenizer: PathBuf,
        #[arg(long)]
        output: PathBuf,
    },
    Check {
        #[arg(long)]
        fixture: PathBuf,
        #[arg(long)]
        checkpoint: PathBuf,
    },
    Train {
        #[arg(long)]
        fixture: PathBuf,
        #[arg(long)]
        checkpoint: PathBuf,
        #[arg(long)]
        output: PathBuf,
        #[arg(long)]
        source_id: String,
        #[arg(long, value_parser=["random", "qa", "diagnostic"])]
        start: String,
        /// Explicit 32-case continuation: each original quartet and its reversed evidence order.
        #[arg(long)]
        both_orders: bool,
    },
    Evaluate {
        #[arg(long)]
        fixture: PathBuf,
        #[arg(long)]
        checkpoint: PathBuf,
        #[arg(long)]
        heldout: bool,
    },
}
#[derive(Subcommand)]
enum Models {
    /// Explicit legacy JSON+safetensors conversion; never changes the source directory.
    ImportLegacy {
        #[arg(long)]
        source: PathBuf,
        #[arg(long)]
        output: PathBuf,
        #[arg(long, value_parser=["inference", "resume"])]
        kind: String,
    },
    /// Export weights/config/tokenizer/lineage only, with no Adam or sampler state.
    ExportInference {
        #[arg(long)]
        checkpoint: PathBuf,
        #[arg(long)]
        output: PathBuf,
    },
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
    /// Explicit read-only JSON import. Normal train/eval accepts only native source files.
    ImportLegacy {
        #[arg(long)]
        source: PathBuf,
        #[arg(long)]
        output: PathBuf,
        #[arg(long)]
        raw: bool,
    },
    /// Read/hash/validate a native source without opening its historical origin paths.
    Verify {
        #[arg(long)]
        corpus: PathBuf,
    },
    Inspect {
        #[arg(long)]
        corpus: PathBuf,
    },
    /// Same-size training-only query/value pairs; preserve validation and other categories.
    BindingPairs {
        #[arg(long)]
        source: PathBuf,
        #[arg(long)]
        output: PathBuf,
    },
    /// Bounded full-answer query/value/order pairs from an existing query-pairs training split.
    QaPairs {
        #[arg(long)]
        source: PathBuf,
        #[arg(long)]
        output: PathBuf,
        #[arg(long, default_value_t = 128, value_parser = clap::value_parser!(u16).range(1..=128))]
        groups: u16,
    },
    /// Preserve a small existing QA set for a learning-path memorization diagnostic.
    Subset {
        #[arg(long)]
        source: PathBuf,
        #[arg(long)]
        output: PathBuf,
        #[arg(long, default_value_t = 32)]
        count: usize,
    },
    Prepare {
        #[arg(long)]
        output: PathBuf,
        #[arg(long, default_value_t = 41)]
        seed: u64,
        #[arg(long, default_value_t = 2000)]
        documents: usize,
        #[arg(long)]
        local: Vec<PathBuf>,
        #[arg(long, default_value="v1", value_parser=["v1", "curriculum", "balanced", "grounding", "counterfactual", "evidence-first", "record-copy", "entity-cue", "field-cue", "field-pairs", "query-pairs"])]
        profile: String,
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
        Commands::Corpus {
            command:
                Corpus::ImportLegacy {
                    source,
                    output,
                    raw,
                },
        } => data::native::import_legacy(&source, &output, !raw),
        Commands::Corpus {
            command: Corpus::Verify { corpus } | Corpus::Inspect { corpus },
        } => data::native::inspect(&corpus),
        Commands::Corpus {
            command: Corpus::BindingPairs { source, output },
        } => data::binding_pairs(&source, &output),
        Commands::Recovery {
            command,
            legacy_json,
        } => {
            if !legacy_json && !matches!(command, training::recovery::Command::Native { .. }) {
                return Err(replica_v3::Error::Invalid("historical recovery requires explicit --legacy-json; use recovery native for native source execution".into()));
            }
            training::recovery::run(command)
        }
        Commands::Contrast { command } => {
            use training::contrast;
            match command {
                Contrast::Transfer {
                    fixture,
                    checkpoint,
                } => contrast::transfer(&fixture, &checkpoint),
                Contrast::Freeze {
                    corpus,
                    log,
                    tokenizer,
                    output,
                } => contrast::freeze(&corpus, &log, &tokenizer, &output),
                Contrast::Check {
                    fixture,
                    checkpoint,
                } => contrast::check(&fixture, &checkpoint),
                Contrast::Evaluate {
                    fixture,
                    checkpoint,
                    heldout,
                } => contrast::evaluate(&fixture, &checkpoint, heldout),
                Contrast::Train {
                    fixture,
                    checkpoint,
                    output,
                    source_id,
                    start,
                    both_orders,
                } => {
                    let cancel = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
                    let signal = cancel.clone();
                    ctrlc::set_handler(move || {
                        signal.store(true, std::sync::atomic::Ordering::Relaxed)
                    })
                    .map_err(|e| replica_v3::Error::Invalid(e.to_string()))?;
                    contrast::train(
                        &fixture,
                        &checkpoint,
                        &output,
                        &source_id,
                        &start,
                        both_orders,
                        &cancel,
                    )
                }
            }
        }
        Commands::SamplingExposure {
            start,
            end,
            corpus,
            limit,
        } => training::sampling_exposure(&start, &end, &corpus, limit),
        Commands::Corpus {
            command:
                Corpus::QaPairs {
                    source,
                    output,
                    groups,
                },
        } => data::qa_pairs(&source, &output, usize::from(groups)),
        Commands::Corpus {
            command:
                Corpus::Subset {
                    source,
                    output,
                    count,
                },
        } => data::qa_subset(&source, &output, count),
        Commands::Evaluate {
            checkpoint,
            corpus,
            output,
            limit,
            split,
            known_question_form,
            known_field_question_form,
            single_current_record,
            single_qa_record,
        } => training::evaluate_corpus(
            &checkpoint,
            &corpus,
            &output,
            limit,
            &split,
            known_question_form,
            match (known_field_question_form, single_current_record) {
                (false, false) if single_qa_record => training::FieldAblation::QaRecord,
                (false, false) => training::FieldAblation::None,
                (true, false) => training::FieldAblation::Question,
                (false, true) => training::FieldAblation::Record,
                (true, true) => training::FieldAblation::QuestionAndRecord,
            },
        ),
        Commands::Model {
            command:
                Models::ImportLegacy {
                    source,
                    output,
                    kind,
                },
        } => {
            use replica_v3::neural::artifact::{self, ArtifactKind};
            let kind = if kind == "inference" {
                ArtifactKind::Inference
            } else {
                ArtifactKind::Resume
            };
            let start = std::time::Instant::now();
            let manifest = artifact::import_legacy(&source, &output, kind)?;
            println!(
                "export_kind={kind:?} file_bytes={} elapsed_ms={:.3} trained_steps={} diagnostic_only={} source_quality=INHERITED goal1_ready=false",
                std::fs::metadata(&output)?.len(),
                start.elapsed().as_secs_f64() * 1000.,
                manifest.trained_steps,
                manifest.diagnostic_only
            );
            Ok(())
        }
        Commands::Model {
            command: Models::ExportInference { checkpoint, output },
        } => {
            use replica_v3::neural::artifact;
            let start = std::time::Instant::now();
            let loaded = artifact::load(&checkpoint, candle_core::Device::Cpu, false)?;
            let load_ms = start.elapsed().as_secs_f64() * 1000.;
            let start = std::time::Instant::now();
            let manifest = artifact::export_inference(&output, &loaded)?;
            println!(
                "export_kind=Inference file_bytes={} trained_steps={} diagnostic_only={} source_load_ms={load_ms:.3} export_validate_sync_ms={:.3} source_quality=INHERITED goal1_ready=false",
                std::fs::metadata(&output)?.len(),
                manifest.trained_steps,
                manifest.diagnostic_only,
                start.elapsed().as_secs_f64() * 1000.
            );
            Ok(())
        }
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
            no_rss,
            stop_after,
            steps,
            max_tokens,
            seq_len,
            lr,
            warmup,
            microbatch,
            sample_group_size,
            accumulation,
            validate_every,
            seed,
            curriculum_steps,
            first_target_weight,
            extend_steps,
            extend_microbatch,
            extend_sample_group_size,
            extend_curriculum_steps,
            extend_first_target_weight,
            extend_lr,
            extend_warmup,
            source_id,
            replace_corpus,
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
                        sample_group_size,
                        accumulation,
                        seq_len,
                        validate_every,
                        seed,
                        curriculum_steps,
                        first_target_weight,
                        ..Default::default()
                    },
                    stop_after,
                    measure_rss: !no_rss,
                    extend_steps,
                    extend_microbatch,
                    extend_sample_group_size: extend_sample_group_size.map(usize::from),
                    extend_curriculum_steps,
                    extend_first_target_weight,
                    extend_lr,
                    extend_warmup,
                    source_id,
                    replace_corpus,
                },
                cancel,
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
                    profile,
                },
        } => data::prepare(&output, seed, documents, &local, &profile),
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
