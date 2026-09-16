#![forbid(unsafe_code)]
use clap::{Args, Parser, Subcommand};
use replica_v3::{
    Error, Result, app,
    event::*,
    model::{self, LocalModel, ModelConfig},
    retrieval::Search,
    store::Store,
};
use std::{
    io::{Read, Write},
    path::PathBuf,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
};
#[derive(Parser)]
#[command(
    version,
    about = "Local append-only memory with one bounded Rust model generation"
)]
struct Cli {
    #[arg(long, global = true)]
    db: Option<PathBuf>,
    #[command(subcommand)]
    command: Commands,
}
#[derive(Args)]
struct Identity {
    #[arg(long)]
    scope: String,
    #[arg(long, default_value = "default")]
    session: String,
    #[arg(long, default_value = "user")]
    source: String,
    #[arg(long)]
    observed_at: Option<i64>,
    #[arg(long)]
    request_key: Option<String>,
}
#[derive(Args)]
struct Input {
    #[arg(long, conflicts_with = "text")]
    file: Option<PathBuf>,
    #[arg(long)]
    text: Option<String>,
}
#[derive(Args)]
struct SlotArgs {
    #[arg(long)]
    entity: String,
    #[arg(long)]
    predicate: String,
    #[arg(long)]
    context: String,
}
impl SlotArgs {
    fn slot(&self) -> Slot {
        Slot {
            entity: self.entity.clone(),
            predicate: self.predicate.clone(),
            context: self.context.clone(),
        }
    }
}
#[derive(Args)]
struct FactWrite {
    #[command(flatten)]
    identity: Identity,
    #[command(flatten)]
    slot: SlotArgs,
    #[arg(long)]
    valid_from: Option<i64>,
    #[arg(long)]
    valid_until: Option<i64>,
}
#[derive(Subcommand)]
enum Facts {
    Add {
        #[command(flatten)]
        args: FactWrite,
        #[command(flatten)]
        input: Input,
    },
    Correct {
        #[command(flatten)]
        args: FactWrite,
        #[arg(long)]
        expected_head: i64,
        #[command(flatten)]
        input: Input,
    },
    Restore {
        #[command(flatten)]
        args: FactWrite,
        #[arg(long)]
        expected_head: i64,
        #[arg(long)]
        target: i64,
    },
    Current {
        #[arg(long)]
        scope: String,
        #[command(flatten)]
        slot: SlotArgs,
        #[arg(long)]
        as_of: Option<i64>,
        #[arg(long)]
        valid_at: Option<i64>,
    },
    History {
        #[arg(long)]
        scope: String,
        #[command(flatten)]
        slot: SlotArgs,
    },
}
#[derive(Subcommand)]
enum Relations {
    Add {
        #[command(flatten)]
        identity: Identity,
        #[arg(long)]
        kind: String,
        #[arg(long)]
        from: i64,
        #[arg(long)]
        to: i64,
        #[arg(long, value_delimiter = ',')]
        evidence: Vec<i64>,
        #[command(flatten)]
        input: Input,
    },
}
#[derive(Subcommand)]
enum Commands {
    Init,
    Record {
        #[command(flatten)]
        identity: Identity,
        #[command(flatten)]
        input: Input,
    },
    Fact {
        #[command(subcommand)]
        command: Facts,
    },
    Relation {
        #[command(subcommand)]
        command: Relations,
    },
    Search {
        #[arg(long)]
        scope: String,
        #[arg(long)]
        session: Option<String>,
        #[arg(long)]
        query: String,
        #[arg(long)]
        after: Option<i64>,
        #[arg(long)]
        before: Option<i64>,
        #[arg(long)]
        history: bool,
        #[arg(long)]
        lexical_only: bool,
        #[arg(long,requires_all=["predicate","context"])]
        entity: Option<String>,
        #[arg(long,requires_all=["entity","context"])]
        predicate: Option<String>,
        #[arg(long,requires_all=["entity","predicate"])]
        context: Option<String>,
    },
    Show {
        id: i64,
        #[arg(long)]
        metadata: bool,
    },
    Ask {
        #[command(flatten)]
        identity: Identity,
        #[command(flatten)]
        input: Input,
        #[command(flatten)]
        model: ModelConfig,
        #[arg(long, default_value_t = 512)]
        max_tokens: u32,
        #[arg(long, default_value_t = 8192)]
        context_tokens: u32,
        #[arg(long, default_value_t = 120000)]
        timeout_ms: u64,
    },
    Doctor {
        #[arg(long)]
        full: bool,
    },
    Reindex,
    Backup {
        destination: PathBuf,
    },
    Restore {
        source: PathBuf,
    },
    #[command(name = "__model-worker", hide = true)]
    ModelWorker {
        #[command(flatten)]
        config: ModelConfig,
    },
}
fn input_bytes(input: Input) -> Result<Vec<u8>> {
    if let Some(text) = input.text {
        return Ok(text.into_bytes());
    }
    let mut reader: Box<dyn Read> = match input.file {
        Some(p) => Box::new(std::fs::File::open(p)?),
        None => Box::new(std::io::stdin()),
    };
    let mut bytes = Vec::new();
    reader
        .by_ref()
        .take((MAX_PAYLOAD + 1) as u64)
        .read_to_end(&mut bytes)?;
    if bytes.len() > MAX_PAYLOAD {
        return Err(Error::Invalid("input exceeds 262144 bytes".into()));
    }
    Ok(bytes)
}
fn observation(identity: Identity, payload: Vec<u8>) -> Result<Event> {
    let mut event = Event::observation(
        &identity.scope,
        &identity.session,
        &identity.source,
        payload,
    );
    event.observed_at = identity.observed_at;
    event.request_key = identity.request_key.as_deref().map(parse_key).transpose()?;
    Ok(event)
}
fn fact(args: FactWrite, payload: Vec<u8>, previous: Option<i64>) -> Result<Event> {
    let slot = args.slot.slot();
    let mut event = observation(args.identity, payload)?;
    event.kind = Kind::Fact {
        slot,
        previous,
        restored_from: None,
        valid_from: args.valid_from,
        valid_until: args.valid_until,
    };
    Ok(event)
}
fn display(event: &Event) {
    println!(
        "event={} recorded_at={} observed_at={:?} source={:?} kind={:?}\n{}",
        event.id,
        event.recorded_at,
        event.observed_at,
        event.source,
        event.kind,
        String::from_utf8_lossy(&event.payload)
    );
}
fn default_db() -> Result<PathBuf> {
    Ok(PathBuf::from(
        std::env::var_os("HOME")
            .ok_or_else(|| Error::Invalid("HOME unavailable; specify --db".into()))?,
    )
    .join("Library/Application Support/Replica-v3/memory.db"))
}
fn run() -> Result<()> {
    let cli = Cli::parse();
    if let Commands::ModelWorker { config } = cli.command {
        return model::worker(config);
    }
    let path = cli.db.map(Ok).unwrap_or_else(default_db)?;
    match cli.command {
        Commands::Init => {
            let store = Store::init(&path)?;
            println!("initialized {}\n{:?}", path.display(), store.doctor(false)?);
            return Ok(());
        }
        Commands::Restore { source } => {
            Store::restore(source, &path)?;
            println!("restored {}", path.display());
            return Ok(());
        }
        _ => {}
    }
    let mut store = Store::open_for_maintenance(&path, matches!(cli.command, Commands::Reindex))?;
    match cli.command {
        Commands::Record { identity, input } => {
            let e = store.append(observation(identity, input_bytes(input)?)?)?;
            #[cfg(feature = "test-support")]
            replica_v3::store::test_pause("before_display");
            println!("{}", e.id);
        }
        Commands::Fact { command } => match command {
            Facts::Add { args, input } => {
                display(&store.append(fact(args, input_bytes(input)?, None)?)?)
            }
            Facts::Correct {
                args,
                expected_head,
                input,
            } => display(&store.append(fact(args, input_bytes(input)?, Some(expected_head))?)?),
            Facts::Restore {
                args,
                expected_head,
                target,
            } => {
                display(&store.restore_fact(fact(args, Vec::new(), Some(expected_head))?, target)?)
            }
            Facts::Current {
                scope,
                slot,
                as_of,
                valid_at,
            } => {
                match store.current(&scope, &slot.slot(), as_of, valid_at.unwrap_or_else(now_ms))? {
                    Some(e) => display(&e),
                    None => println!("no valid fact"),
                }
            }
            Facts::History { scope, slot } => {
                for e in store.history(&scope, &slot.slot())? {
                    display(&e)
                }
            }
        },
        Commands::Relation {
            command:
                Relations::Add {
                    identity,
                    kind,
                    from,
                    to,
                    evidence,
                    input,
                },
        } => {
            let mut e = observation(identity, input_bytes(input)?)?;
            e.kind = Kind::Relation {
                relation: kind.parse()?,
                from,
                to,
                evidence,
            };
            display(&store.append(e)?);
        }
        Commands::Search {
            scope,
            session,
            query,
            after,
            before,
            history,
            lexical_only,
            entity,
            predicate,
            context,
        } => {
            let mut q = Search::new(&scope, &query);
            q.session = session;
            q.after = after;
            q.before = before;
            q.history = history;
            q.graph = !lexical_only;
            if let (Some(entity), Some(predicate), Some(context)) = (entity, predicate, context) {
                q.slot = Some(Slot {
                    entity,
                    predicate,
                    context,
                });
            }
            let result = store.search(&q)?;
            println!("truncated={} visited={}", result.truncated, result.visited);
            for e in result.items {
                println!("{e:?}");
            }
        }
        Commands::Show { id, metadata } => {
            let e = store.get(id)?;
            if metadata {
                display(&e)
            } else {
                std::io::stdout().lock().write_all(&e.payload)?;
            }
        }
        Commands::Ask {
            identity,
            input,
            model,
            max_tokens,
            context_tokens,
            timeout_ms,
        } => {
            let cancel = Arc::new(AtomicBool::new(false));
            let signal = cancel.clone();
            ctrlc::set_handler(move || signal.store(true, Ordering::Relaxed))
                .map_err(|e| Error::Model(e.to_string()))?;
            let event = app::ask(
                &mut store,
                observation(identity, input_bytes(input)?)?,
                GenerationLimits {
                    max_tokens,
                    context_tokens,
                    timeout_ms,
                },
                &mut LocalModel { config: model },
                &cancel,
            )?;
            if let Kind::AssistantAnswer { evidence, .. } = &event.kind
                && evidence.is_empty()
            {
                println!("[확인된 사건 인용 없음 — 답변의 사실성은 검증되지 않았습니다]");
            }
            display(&event);
        }
        Commands::Doctor { full } => {
            println!("{:#?}", store.doctor(full)?);
            println!("storage_bytes={:?}", store.storage_sizes()?);
        }
        Commands::Reindex => {
            store.reindex()?;
            println!("reindexed");
        }
        Commands::Backup { destination } => {
            store.backup(&destination)?;
            println!("backed up {}", destination.display());
        }
        Commands::Init | Commands::Restore { .. } | Commands::ModelWorker { .. } => unreachable!(),
    }
    Ok(())
}
fn main() {
    if let Err(error) = run() {
        eprintln!("{error}");
        std::process::exit(1);
    }
}
