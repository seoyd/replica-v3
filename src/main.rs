#![forbid(unsafe_code)]
use clap::{Args, Parser, Subcommand};
use replica_v3::{
    Error, Result, app,
    event::*,
    model::{self, LocalModel, Model, ModelConfig},
    retrieval::Search,
    store::Store,
};
use std::{
    io::{BufRead, Read, Write},
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
    Retract {
        #[command(flatten)]
        args: FactWrite,
        #[arg(long)]
        expected_head: i64,
        #[command(flatten)]
        input: Input,
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
    Archive {
        #[arg(long)]
        path: PathBuf,
        /// Explicit experimental journal overlay; never opens the default database.
        #[arg(long)]
        journal: Option<PathBuf>,
        #[command(subcommand)]
        command: Archives,
    },
    /// Experimental snapshot+journal store, separate from product ask/SQLite.
    Journal {
        #[arg(long)]
        snapshot: PathBuf,
        #[arg(long)]
        path: PathBuf,
        #[command(subcommand)]
        command: Journals,
    },
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
        #[command(flatten)]
        limits: GenerationLimits,
        #[arg(long)]
        history: bool,
    },
    Generate {
        #[command(flatten)]
        input: Input,
        #[command(flatten)]
        model: ModelConfig,
        #[command(flatten)]
        limits: GenerationLimits,
    },
    Chat {
        #[command(flatten)]
        identity: Identity,
        #[command(flatten)]
        model: ModelConfig,
        #[command(flatten)]
        limits: GenerationLimits,
        #[arg(long)]
        history: bool,
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
#[derive(Subcommand)]
enum Archives {
    Export {
        #[arg(long,default_value="zstd",value_parser=["raw","zstd"])]
        compression: String,
    },
    Inspect,
    Show {
        id: i64,
        #[arg(long)]
        metadata: bool,
        /// Export the exact RPV3 envelope, including assigned ID/time.
        #[arg(long, conflicts_with = "metadata")]
        canonical: bool,
    },
    History {
        #[arg(long)]
        scope: String,
        #[command(flatten)]
        slot: SlotArgs,
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
    Graph {
        #[arg(long)]
        scope: String,
        #[arg(long)]
        session: Option<String>,
        #[arg(long, value_delimiter = ',')]
        seeds: Vec<i64>,
        #[arg(long,default_value="both",value_parser=["both","outgoing","incoming"])]
        direction: String,
        #[arg(long)]
        after: Option<i64>,
        #[arg(long)]
        before: Option<i64>,
        #[arg(long)]
        history: bool,
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
    },
}
#[derive(Subcommand)]
enum Journals {
    Init {
        #[arg(long)]
        store_id: String,
    },
    Inspect,
    Append {
        #[arg(long)]
        request: String,
        /// Files containing exact, already assigned RPV3 event envelopes in commit order.
        #[arg(long, required = true)]
        events: Vec<PathBuf>,
        #[arg(long,default_value="raw",value_parser=["raw","zstd1","zstd3"])]
        codec: String,
    },
    Recover {
        #[arg(long)]
        output: PathBuf,
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
    if let Commands::Journal {
        snapshot,
        path,
        command,
    } = cli.command
    {
        use replica_v3::journal::Journal;
        if let Journals::Init { store_id } = command {
            return Journal::create(&snapshot, &path, parse_key(&store_id)?);
        }
        let mut journal =
            Journal::open(&snapshot, &path, matches!(command, Journals::Append { .. }))?;
        match command {
            Journals::Inspect => println!(
                "committed={:?} events={} tail={:?} bytes={} experimental=true durability=NOT_EQUIVALENT",
                journal.last(),
                journal.view().event_count(),
                journal.tail(),
                journal.file_bytes()?
            ),
            Journals::Recover { output } => journal.recover_to(&output)?,
            Journals::Append {
                request,
                events,
                codec,
            } => {
                if events.len() > 256 {
                    return Err(Error::Invalid("journal event count".into()));
                }
                let mut bodies = Vec::new();
                let mut bytes = 0;
                for p in events {
                    let body = replica_v3::neural::read_bounded(
                        &p,
                        replica_v3::codec::MAX_BODY + replica_v3::codec::HEADER,
                    )?;
                    bytes += body.len() + 4;
                    if bytes > 4 * 1024 * 1024 {
                        return Err(Error::Invalid("journal transaction byte bound".into()));
                    }
                    bodies.push(body);
                }
                let level = match codec.as_str() {
                    "zstd1" => Some(1),
                    "zstd3" => Some(3),
                    _ => None,
                };
                println!(
                    "ACK {:?}",
                    journal.append(parse_key(&request)?, bodies, level)?
                );
            }
            Journals::Init { .. } => unreachable!(),
        }
        return Ok(());
    }
    // Archive read commands return before default_db/Store::open: they need no SQLite file.
    if let Commands::Archive {
        path,
        journal,
        command,
    } = cli.command
    {
        if let Archives::Export { compression } = command {
            if journal.is_some() {
                return Err(Error::Invalid(
                    "journal overlay is read-only in archive commands".into(),
                ));
            }
            let db = cli.db.map(Ok).unwrap_or_else(default_db)?;
            let info = Store::open(db)?.export_archive(
                &path,
                if compression == "raw" {
                    replica_v3::codec::Compression::Raw
                } else {
                    replica_v3::codec::Compression::Auto
                },
            )?;
            println!("{}", serde_json::to_string(&info)?);
            return Ok(());
        }
        let overlay = journal
            .map(|j| replica_v3::journal::Journal::open(&path, &j, false))
            .transpose()?;
        let base = if overlay.is_none() {
            Some(replica_v3::archive::Archive::open(&path)?)
        } else {
            None
        };
        let archive = overlay
            .as_ref()
            .map(|j| j.view())
            .or(base.as_ref())
            .expect("one explicit evidence view");
        if let Some(j) = &overlay {
            if let Some(tail) = j.tail() {
                eprintln!(
                    "verified_prefix={} damaged_tail={tail:?}; writes disabled",
                    j.last().end
                );
            }
            if matches!(command, Archives::Inspect) {
                println!(
                    "journal_committed={:?} total_events={}",
                    j.last(),
                    j.view().event_count()
                );
            }
        }
        match command {
            Archives::Inspect => println!(
                "{}\nindex_rebuild_ms={:.3} live_sqlite_replacement=false",
                serde_json::to_string(&archive.info)?,
                archive.index_rebuild_ms
            ),
            Archives::Show {
                id,
                metadata,
                canonical,
            } => {
                if canonical {
                    std::io::stdout()
                        .lock()
                        .write_all(&archive.canonical_body(id)?)?;
                    return Ok(());
                }
                let e = archive.get(id)?;
                if metadata {
                    display(&e)
                } else {
                    std::io::stdout().lock().write_all(&e.payload)?;
                }
            }
            Archives::History { scope, slot } => {
                for e in archive.history(&scope, &slot.slot())? {
                    display(&e)
                }
            }
            Archives::Current {
                scope,
                slot,
                as_of,
                valid_at,
            } => {
                match archive.current(
                    &scope,
                    &slot.slot(),
                    as_of,
                    valid_at.unwrap_or_else(now_ms),
                )? {
                    Some(e) => display(&e),
                    None => println!("no valid fact"),
                }
            }
            Archives::Graph {
                scope,
                session,
                seeds,
                direction,
                after,
                before,
                history,
            } => {
                let mut q = Search::new(&scope, "");
                q.session = session;
                q.after = after;
                q.before = before;
                q.history = history;
                let direction = match direction.as_str() {
                    "outgoing" => replica_v3::retrieval::GraphDirection::Outgoing,
                    "incoming" => replica_v3::retrieval::GraphDirection::Incoming,
                    _ => replica_v3::retrieval::GraphDirection::Both,
                };
                println!(
                    "{}",
                    serde_json::to_string(&archive.directed_graph(&q, &seeds, direction)?)?
                );
            }
            Archives::Search {
                scope,
                session,
                query,
                after,
                before,
                history,
                lexical_only,
            } => {
                let mut q = Search::new(&scope, &query);
                q.session = session;
                q.after = after;
                q.before = before;
                q.history = history;
                q.graph = !lexical_only;
                println!("{}", serde_json::to_string(&archive.search(&q)?)?);
            }
            Archives::Export { .. } => unreachable!(),
        }
        return Ok(());
    }
    if let Commands::ModelWorker { config } = cli.command {
        return model::worker(config);
    }
    if let Commands::Generate {
        input,
        model,
        limits,
    } = cli.command
    {
        let cancel = cancellation()?;
        let request = model::ModelRequest {
            request_id: "direct-native-generation".into(),
            system: model::SYSTEM.into(),
            input: String::from_utf8(input_bytes(input)?)
                .map_err(|_| Error::Invalid("input UTF-8".into()))?,
            evidence: Default::default(),
            limits,
        };
        let response = LocalModel { config: model }.generate(&request, &cancel)?;
        app::validate_response(&request, &response)?;
        if cancel.load(Ordering::Relaxed) {
            return Err(Error::Cancelled);
        }
        println!("{}", response.text);
        eprintln!("{}", serde_json::to_string(&response.generation)?);
        return Ok(());
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
            Facts::Retract {
                args,
                expected_head,
                input,
            } => {
                let mut event = fact(args, input_bytes(input)?, Some(expected_head))?;
                let (valid_from, valid_until) = event.kind.validity();
                event.kind = Kind::Retraction {
                    slot: event.kind.slot().expect("fact constructor").clone(),
                    previous: expected_head,
                    valid_from,
                    valid_until,
                };
                display(&store.append(event)?);
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
            limits,
            history,
        } => {
            let cancel = cancellation()?;
            let event = app::ask_with_history(
                &mut store,
                observation(identity, input_bytes(input)?)?,
                limits,
                &mut LocalModel { config: model },
                &cancel,
                history,
            )?;
            display_answer(&event);
        }
        Commands::Chat {
            identity,
            model,
            limits,
            history,
        } => {
            if identity.request_key.is_some() {
                return Err(Error::Invalid(
                    "chat assigns a fresh request key per input; explicit keys use ask".into(),
                ));
            }
            let cancel = cancellation()?;
            let mut model = LocalModel { config: model };
            limits.validate()?;
            let (send, receive) = std::sync::mpsc::sync_channel(1);
            // One bounded reader lets the foreground observe cancellation even
            // while a terminal has supplied no newline. It ends with this CLI process.
            std::thread::spawn(move || {
                let mut input = std::io::stdin().lock();
                loop {
                    let mut line = Vec::new();
                    match input
                        .by_ref()
                        .take((MAX_PAYLOAD + 1) as u64)
                        .read_until(b'\n', &mut line)
                    {
                        Ok(0) => break,
                        Err(e) => {
                            let _ = send.send(Err(Error::from(e)));
                            break;
                        }
                        Ok(_) if line.len() > MAX_PAYLOAD => {
                            let _ = send.send(Err(Error::Invalid("chat input byte limit".into())));
                            break;
                        }
                        Ok(_) => {
                            if send.send(Ok(line)).is_err() {
                                break;
                            }
                        }
                    }
                }
            });
            'session: loop {
                eprint!("> ");
                std::io::stderr().flush()?;
                let mut line = loop {
                    if cancel.load(Ordering::Relaxed) {
                        return Err(Error::Cancelled);
                    }
                    match receive.recv_timeout(std::time::Duration::from_millis(10)) {
                        Ok(line) => break line?,
                        Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {}
                        Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => break 'session,
                    }
                };
                if line.last() == Some(&b'\n') {
                    line.pop();
                    if line.last() == Some(&b'\r') {
                        line.pop();
                    }
                }
                if line == b"/quit" {
                    break;
                }
                if line.is_empty() {
                    continue;
                }
                let mut event =
                    Event::observation(&identity.scope, &identity.session, &identity.source, line);
                event.observed_at = identity.observed_at;
                let mut key = [0u8; 16];
                std::fs::File::open("/dev/urandom")?.read_exact(&mut key)?;
                event.request_key = Some(key);
                display_answer(&app::ask_with_history(
                    &mut store,
                    event,
                    limits.clone(),
                    &mut model,
                    &cancel,
                    history,
                )?);
            }
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
        Commands::Init
        | Commands::Restore { .. }
        | Commands::ModelWorker { .. }
        | Commands::Generate { .. } => unreachable!(),
        Commands::Archive { .. } | Commands::Journal { .. } => unreachable!(),
    }
    Ok(())
}
fn cancellation() -> Result<Arc<AtomicBool>> {
    let cancel = Arc::new(AtomicBool::new(false));
    let signal = cancel.clone();
    ctrlc::set_handler(move || signal.store(true, Ordering::Relaxed))
        .map_err(|e| Error::Model(e.to_string()))?;
    Ok(cancel)
}
fn display_answer(event: &Event) {
    if let Kind::AssistantAnswer { evidence, .. } = &event.kind
        && evidence.is_empty()
    {
        println!("[확인된 사건 인용 없음 — 답변의 사실성은 검증되지 않았습니다]");
    }
    display(event);
}
fn main() {
    if let Err(error) = run() {
        eprintln!("{error}");
        std::process::exit(1);
    }
}
