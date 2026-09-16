use crate::{
    Error, Result,
    codec::{self, Compression},
    event::*,
};
use rusqlite::{Connection, OpenFlags, OptionalExtension, TransactionBehavior, params};
#[cfg(unix)]
use std::os::unix::fs::{DirBuilderExt, OpenOptionsExt};
use std::{
    fs::{self, OpenOptions},
    path::{Path, PathBuf},
    time::Duration,
};
const SCHEMA: &str = include_str!("../migrations/001_init.sql");
const APP_ID: i64 = 0x52505633;
pub struct Store {
    pub(crate) conn: Connection,
    path: PathBuf,
    compression: Compression,
}
#[derive(Debug)]
pub struct Doctor {
    pub sqlite: String,
    pub compile_options: Vec<String>,
    pub pragmas: Vec<(String, String)>,
    pub records: u64,
    pub full: bool,
}

fn new_file(path: &Path) -> Result<()> {
    if let Some(parent) = path.parent().filter(|p| !p.as_os_str().is_empty())
        && !parent.exists()
    {
        let mut builder = fs::DirBuilder::new();
        builder.recursive(true);
        #[cfg(unix)]
        builder.mode(0o700);
        builder.create(parent)?;
    }
    let mut options = OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    options.mode(0o600);
    options.open(path)?;
    Ok(())
}
fn configure(conn: &Connection) -> Result<()> {
    conn.busy_timeout(Duration::from_millis(5000))?;
    conn.execute_batch(
        "PRAGMA foreign_keys=ON; PRAGMA journal_mode=WAL; PRAGMA synchronous=FULL;",
    )?;
    #[cfg(target_os = "macos")]
    conn.execute_batch("PRAGMA fullfsync=ON;")?;
    let mode: String = conn.query_row("PRAGMA journal_mode", [], |r| r.get(0))?;
    for (key, expected) in [
        ("foreign_keys", 1),
        ("synchronous", 2),
        ("busy_timeout", 5000),
    ] {
        let actual: i64 = conn.query_row(&format!("PRAGMA {key}"), [], |r| r.get(0))?;
        if actual != expected {
            return Err(Error::Invalid(format!(
                "SQLite {key}={actual}, expected {expected}"
            )));
        }
    }
    if mode != "wal" {
        return Err(Error::Invalid(format!("WAL unavailable: {mode}")));
    }
    #[cfg(target_os = "macos")]
    {
        let full: i64 = conn.query_row("PRAGMA fullfsync", [], |r| r.get(0))?;
        if full != 1 {
            return Err(Error::Invalid("fullfsync not applied".into()));
        }
    }
    Ok(())
}
impl Store {
    pub fn init(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();
        new_file(path)?;
        let conn = Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_WRITE)?;
        configure(&conn)?;
        conn.execute_batch(&format!("BEGIN IMMEDIATE; PRAGMA application_id={APP_ID}; PRAGMA user_version=1; {SCHEMA} COMMIT;"))?;
        let store = Self {
            conn,
            path: path.canonicalize()?,
            compression: Compression::Auto,
        };
        store.startup()?;
        Ok(store)
    }
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        Self::open_for_maintenance(path, false)
    }
    pub fn open_for_maintenance(path: impl AsRef<Path>, reindex: bool) -> Result<Self> {
        let path = path.as_ref();
        let conn = Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_WRITE)?;
        // Check identity before setting persistence options on an unrelated file.
        identity(&conn)?;
        configure(&conn)?;
        let store = Self {
            conn,
            path: path.canonicalize()?,
            compression: Compression::Auto,
        };
        if reindex {
            quick_check(&store.conn)?;
        } else {
            store.startup()?;
        }
        Ok(store)
    }
    fn startup(&self) -> Result<()> {
        identity(&self.conn)?;
        quick_check(&self.conn)?;
        let (version, last, max): (i64, i64, i64) = self.conn.query_row(
            "SELECT version,last_id,(SELECT coalesce(max(id),0) FROM records) FROM projection_state WHERE singleton=1",
            [],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
        )?;
        #[cfg(feature = "test-support")]
        sync_point("startup_observed")?;
        if version != 1 || last != max {
            return Err(Error::Corrupt(
                "projection version/watermark; run reindex".into(),
            ));
        }
        for table in [
            "record_meta",
            "current_heads",
            "relations",
            "request_state",
            "results",
            "record_fts",
        ] {
            self.conn
                .prepare(&format!("SELECT * FROM {table} LIMIT 0"))?;
        }
        Ok(())
    }
    pub fn path(&self) -> &Path {
        &self.path
    }
    pub fn set_compression(&mut self, c: Compression) {
        self.compression = c;
    }
    pub fn generation_lock(&self) -> Result<std::fs::File> {
        let mut path = self.path.as_os_str().to_os_string();
        path.push(".inference.lock");
        let mut options = OpenOptions::new();
        options.read(true).write(true).create(true).truncate(false);
        #[cfg(unix)]
        options.mode(0o600);
        let file = options.open(path)?;
        file.try_lock().map_err(|e| {
            Error::Conflict(format!(
                "generation already active or lock unavailable: {e}"
            ))
        })?;
        Ok(file)
    }
    pub fn count(&self) -> Result<u64> {
        Ok(self
            .conn
            .query_row("SELECT count(*) FROM records", [], |r| r.get(0))?)
    }
    pub fn get(&self, id: EventId) -> Result<Event> {
        get(&self.conn, id, true)
    }
    pub fn request(&self, scope: &str, key: [u8; 16]) -> Result<Option<Event>> {
        let id = self
            .conn
            .query_row(
                "SELECT event_id FROM request_state WHERE scope=?1 AND request_key=?2",
                params![scope, key.as_slice()],
                |r| r.get(0),
            )
            .optional()?;
        id.map(|id| self.get(id)).transpose()
    }
    pub fn result(&self, input: EventId) -> Result<Option<Event>> {
        result(&self.conn, input)
    }
    pub fn append(&mut self, event: Event) -> Result<Event> {
        let compression = self.compression;
        let tx = self
            .conn
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let e = append_in(&tx, event, compression)?;
        #[cfg(feature = "test-support")]
        test_pause("before_commit");
        tx.commit()?;
        #[cfg(feature = "test-support")]
        test_pause("after_commit");
        Ok(e)
    }
    // Explicit fixture/import API; ordinary user commands commit one event at a time.
    pub fn import(&mut self, events: Vec<Event>) -> Result<Vec<Event>> {
        if events.len() > 20_000 {
            return Err(Error::Invalid("import exceeds 20000 events".into()));
        }
        let compression = self.compression;
        let tx = self
            .conn
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let results = events
            .into_iter()
            .map(|e| append_in(&tx, e, compression))
            .collect::<Result<Vec<_>>>()?;
        tx.commit()?;
        Ok(results)
    }
    pub fn head(&self, scope: &str, slot: &Slot) -> Result<Option<Event>> {
        head(&self.conn, scope, slot)
    }
    pub fn current(
        &self,
        scope: &str,
        slot: &Slot,
        as_of: Option<i64>,
        valid_at: i64,
    ) -> Result<Option<Event>> {
        let id:Option<i64>=self.conn.query_row("SELECT id FROM record_meta WHERE slot=?1 AND recorded_at<=?2 ORDER BY id DESC LIMIT 1",params![slot.key(scope),as_of.unwrap_or(i64::MAX)],|r|r.get(0)).optional()?;
        let event = id.map(|id| self.get(id)).transpose()?;
        Ok(event.filter(|e| valid_at_time(e, valid_at)))
    }
    pub fn history(&self, scope: &str, slot: &Slot) -> Result<Vec<Event>> {
        let mut stmt = self
            .conn
            .prepare("SELECT id FROM record_meta WHERE slot=?1 ORDER BY id")?;
        let ids = stmt
            .query_map([slot.key(scope)], |r| r.get(0))?
            .collect::<std::result::Result<Vec<i64>, _>>()?;
        ids.into_iter().map(|id| self.get(id)).collect()
    }
    pub fn restore_fact(&mut self, mut event: Event, target: i64) -> Result<Event> {
        let old = self.get(target)?;
        let Kind::Fact {
            slot,
            restored_from,
            ..
        } = &mut event.kind
        else {
            return Err(Error::Invalid("restore requires fact".into()));
        };
        if old.scope != event.scope || old.kind.slot() != Some(slot) {
            return Err(Error::Conflict("restore outside lineage".into()));
        }
        *restored_from = Some(target);
        event.payload = old.payload;
        self.append(event)
    }
    pub fn doctor(&self, full: bool) -> Result<Doctor> {
        self.startup()?;
        if full {
            let tx = self.conn.unchecked_transaction()?;
            let replay = Connection::open_in_memory()?;
            replay.execute_batch(SCHEMA)?;
            let mut stmt = tx.prepare("SELECT id,body FROM records ORDER BY id")?;
            let mut rows = stmt.query([])?;
            let mut previous_time = i64::MIN;
            let mut previous_id = 0;
            while let Some(row) = rows.next()? {
                let id: i64 = row.get(0)?;
                let bytes: Vec<u8> = row.get(1)?;
                let e = codec::decode(&bytes)?;
                if e.id != id || id <= previous_id || e.recorded_at < previous_time {
                    return Err(Error::Corrupt("canonical ordering".into()));
                }
                validate_links(&replay, &e).map_err(|e| Error::Corrupt(e.to_string()))?;
                replay.execute("INSERT INTO records VALUES(?1,?2)", params![id, bytes])?;
                project(&replay, &e)?;
                previous_time = e.recorded_at;
                previous_id = id;
            }
            for table in [
                "projection_state",
                "record_meta",
                "current_heads",
                "relations",
                "request_state",
                "results",
                "record_fts",
            ] {
                if table_rows(&tx, table)? != table_rows(&replay, table)? {
                    return Err(Error::Corrupt(format!(
                        "projection mismatch: {table}; run reindex"
                    )));
                }
            }
            let fk: Option<String> = tx
                .query_row("PRAGMA foreign_key_check", [], |r| r.get(0))
                .optional()?;
            if fk.is_some() {
                return Err(Error::Corrupt("foreign key check".into()));
            }
        }
        let compile_options = self
            .conn
            .prepare("PRAGMA compile_options")?
            .query_map([], |r| r.get(0))?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        let mut pragmas = Vec::new();
        for key in [
            "journal_mode",
            "foreign_keys",
            "synchronous",
            "busy_timeout",
            "fullfsync",
            "application_id",
            "user_version",
        ] {
            let value = self.conn.query_row(&format!("PRAGMA {key}"), [], |r| {
                let v = r.get_ref(0)?;
                Ok(match v {
                    rusqlite::types::ValueRef::Text(t) => String::from_utf8_lossy(t).into_owned(),
                    rusqlite::types::ValueRef::Integer(n) => n.to_string(),
                    _ => format!("{v:?}"),
                })
            })?;
            pragmas.push((key.into(), value));
        }
        Ok(Doctor {
            sqlite: rusqlite::version().into(),
            compile_options,
            pragmas,
            records: self.count()?,
            full,
        })
    }
    pub fn reindex(&mut self) -> Result<()> {
        let tx = self
            .conn
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        tx.execute_batch(SCHEMA)?;
        tx.execute_batch("DELETE FROM record_meta; DELETE FROM current_heads; DELETE FROM relations; DELETE FROM request_state; DELETE FROM results; DELETE FROM record_fts; UPDATE projection_state SET version=1,last_id=0;")?;
        {
            let mut stmt = tx.prepare("SELECT id,body FROM records ORDER BY id")?;
            let mut rows = stmt.query([])?;
            let mut previous = i64::MIN;
            while let Some(row) = rows.next()? {
                let id: i64 = row.get(0)?;
                let e = codec::decode(&row.get::<_, Vec<u8>>(1)?)?;
                if e.id != id || e.recorded_at < previous {
                    return Err(Error::Corrupt("canonical ordering".into()));
                }
                validate_links(&tx, &e).map_err(|e| Error::Corrupt(e.to_string()))?;
                project(&tx, &e)?;
                previous = e.recorded_at;
            }
        }
        tx.commit()?;
        Ok(())
    }
    pub fn backup(&self, destination: impl AsRef<Path>) -> Result<()> {
        let path = destination.as_ref();
        new_file(path)?;
        // On failure keep this create-new destination as an explicitly failed
        // artifact. Never delete/overwrite a path a caller may have replaced.
        self.backup_into(path).map_err(|e| {
            Error::Corrupt(format!(
                "backup failed; untrusted destination retained at {}: {e}",
                path.display()
            ))
        })
    }
    fn backup_into(&self, path: &Path) -> Result<()> {
        let mut dest = Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_WRITE)?;
        // Hold a stable read snapshot for backup and exact comparison, including an open WAL.
        let snapshot = self.conn.unchecked_transaction()?;
        let _: i64 =
            snapshot.query_row("SELECT coalesce(max(id),0) FROM records", [], |r| r.get(0))?;
        #[cfg(feature = "test-support")]
        sync_point("backup_snapshot")?;
        {
            let backup = rusqlite::backup::Backup::new(&snapshot, &mut dest)?;
            let start = std::time::Instant::now();
            loop {
                #[cfg(feature = "test-support")]
                sync_point("backup_step")?;
                match backup.step(128)? {
                    rusqlite::backup::StepResult::Done => break,
                    _ => {
                        if start.elapsed() > Duration::from_secs(30) {
                            return Err(Error::Invalid("backup timeout".into()));
                        }
                        std::thread::sleep(Duration::from_millis(1));
                    }
                }
            }
        }
        drop(dest);
        #[cfg(feature = "test-support")]
        sync_point("backup_done")?;
        let restored = Self::open(path)?;
        restored.doctor(true)?;
        if table_rows(&snapshot, "records")? != table_rows(&restored.conn, "records")? {
            return Err(Error::Corrupt("backup canonical mismatch".into()));
        }
        Ok(())
    }
    pub fn restore(source: impl AsRef<Path>, destination: impl AsRef<Path>) -> Result<()> {
        Self::open(source)?.backup(destination)
    }
    pub fn storage_sizes(&self) -> Result<Vec<(String, u64)>> {
        let mut out = Vec::new();
        for suffix in ["", "-wal", "-shm"] {
            let p = PathBuf::from(format!("{}{suffix}", self.path.display()));
            out.push((
                format!("db{suffix}"),
                match fs::metadata(p) {
                    Ok(m) => m.len(),
                    Err(e) if e.kind() == std::io::ErrorKind::NotFound => 0,
                    Err(e) => return Err(e.into()),
                },
            ));
        }
        let mut stmt = self
            .conn
            .prepare("SELECT name,sum(pgsize) FROM dbstat GROUP BY name ORDER BY name")?;
        out.extend(
            stmt.query_map([], |r| Ok((r.get(0)?, r.get(1)?)))?
                .collect::<std::result::Result<Vec<(String, u64)>, _>>()?,
        );
        Ok(out)
    }
}
fn identity(conn: &Connection) -> Result<()> {
    let app: i64 = conn.query_row("PRAGMA application_id", [], |r| r.get(0))?;
    let version: i64 = conn.query_row("PRAGMA user_version", [], |r| r.get(0))?;
    if app != APP_ID || version != 1 {
        Err(Error::Corrupt("database identity/schema version".into()))
    } else {
        Ok(())
    }
}
fn quick_check(conn: &Connection) -> Result<()> {
    let result: String = conn.query_row("PRAGMA quick_check(1)", [], |r| r.get(0))?;
    if result != "ok" {
        Err(Error::Corrupt(result))
    } else {
        Ok(())
    }
}
pub(crate) fn get(conn: &Connection, id: i64, check_meta: bool) -> Result<Event> {
    let bytes: Vec<u8> = conn
        .query_row("SELECT body FROM records WHERE id=?1", [id], |r| r.get(0))
        .optional()?
        .ok_or_else(|| Error::NotFound(format!("event {id}")))?;
    let e = codec::decode(&bytes)?;
    if e.id != id {
        return Err(Error::Corrupt("event ID mismatch".into()));
    }
    if check_meta {
        let actual=conn.query_row("SELECT kind,scope,session,source,recorded_at,observed_at,slot,question,valid_from,valid_until FROM record_meta WHERE id=?1",[id],|r|{(0..10).map(|i|r.get::<_,rusqlite::types::Value>(i)).collect::<std::result::Result<Vec<_>,_>>()}).optional()?;
        if actual.as_ref() != Some(&metadata(&e)) {
            return Err(Error::Corrupt(format!("metadata mismatch for {id}")));
        }
    }
    Ok(e)
}
fn metadata(e: &Event) -> Vec<rusqlite::types::Value> {
    use rusqlite::types::Value;
    let (from, until) = if let Kind::Fact {
        valid_from,
        valid_until,
        ..
    } = e.kind
    {
        (valid_from, valid_until)
    } else {
        (None, None)
    };
    vec![
        Value::Integer(e.kind.tag().into()),
        Value::Text(e.scope.clone()),
        Value::Text(e.session.clone()),
        Value::Text(e.source.clone()),
        Value::Integer(e.recorded_at),
        e.observed_at.map_or(Value::Null, Value::Integer),
        e.kind
            .slot()
            .map_or(Value::Null, |s| Value::Blob(s.key(&e.scope))),
        Value::Integer(i64::from(matches!(
            e.kind,
            Kind::Observation { question: Some(_) }
        ))),
        from.map_or(Value::Null, Value::Integer),
        until.map_or(Value::Null, Value::Integer),
    ]
}
fn head(conn: &Connection, scope: &str, slot: &Slot) -> Result<Option<Event>> {
    let key = slot.key(scope);
    let (id, latest): (Option<i64>, Option<i64>) = conn.query_row(
        "SELECT (SELECT event_id FROM current_heads WHERE slot=?1),(SELECT max(id) FROM record_meta WHERE slot=?1)",
        [key], |r| Ok((r.get(0)?, r.get(1)?)),
    )?;
    #[cfg(feature = "test-support")]
    sync_point("head_observed")?;
    if id != latest {
        return Err(Error::Corrupt("head mismatch".into()));
    }
    id.map(|id| get(conn, id, true)).transpose()
}
fn result(conn: &Connection, input: EventId) -> Result<Option<Event>> {
    let id: Option<i64> = conn
        .query_row(
            "SELECT event_id FROM results WHERE input_id=?1",
            [input],
            |r| r.get(0),
        )
        .optional()?;
    id.map(|id| {
        let read = |id| {
            get(conn, id, true).map_err(|e| match e {
                Error::NotFound(_) => {
                    Error::Corrupt(format!("result references missing event {id}"))
                }
                other => other,
            })
        };
        let terminal = read(id)?;
        let question = read(input)?;
        if terminal.kind.input() != Some(input)
            || terminal.id <= input
            || !matches!(question.kind, Kind::Observation { question: Some(_) })
            || terminal.scope != question.scope
            || terminal.session != question.session
        {
            return Err(Error::Corrupt(
                "result canonical question/kind/scope/session mismatch; run reindex".into(),
            ));
        }
        Ok(terminal)
    })
    .transpose()
}

// Deterministic, thread-local scheduling only in explicitly enabled test builds.
#[cfg(feature = "test-support")]
type SyncHook = Box<dyn FnMut(&str) -> Result<()>>;
#[cfg(feature = "test-support")]
thread_local! {
    static SYNC_HOOK: std::cell::RefCell<Option<SyncHook>> =
        std::cell::RefCell::new(None);
}
#[cfg(feature = "test-support")]
pub fn with_sync_hook<T>(
    hook: impl FnMut(&str) -> Result<()> + 'static,
    run: impl FnOnce() -> T,
) -> T {
    struct Reset;
    impl Drop for Reset {
        fn drop(&mut self) {
            SYNC_HOOK.with(|h| *h.borrow_mut() = None);
        }
    }
    SYNC_HOOK.with(|h| *h.borrow_mut() = Some(Box::new(hook)));
    let _reset = Reset;
    run()
}
#[cfg(feature = "test-support")]
fn sync_point(point: &str) -> Result<()> {
    SYNC_HOOK.with(|h| match &mut *h.borrow_mut() {
        Some(hook) => hook(point),
        None => Ok(()),
    })
}
fn append_in(conn: &Connection, mut e: Event, compression: Compression) -> Result<Event> {
    e.validate(false)?;
    if let Some(key) = e.request_key {
        let existing: Option<i64> = conn
            .query_row(
                "SELECT event_id FROM request_state WHERE scope=?1 AND request_key=?2",
                params![e.scope, key.as_slice()],
                |r| r.get(0),
            )
            .optional()?;
        if let Some(id) = existing {
            let old = get(conn, id, true)?;
            return if old.same_request(&e) {
                Ok(old)
            } else {
                Err(Error::Conflict("request key content mismatch".into()))
            };
        }
    }
    let existing = e
        .kind
        .input()
        .map(|input| result(conn, input))
        .transpose()?
        .flatten();
    let last_id: Option<i64> = conn.query_row("SELECT max(id) FROM records", [], |r| r.get(0))?;
    let last = last_id
        .map(|id| get(conn, id, true).map(|e| (id, e.recorded_at)))
        .transpose()?;
    e.id = last
        .map_or(0, |x| x.0)
        .checked_add(1)
        .ok_or_else(|| Error::Invalid("event ID exhausted".into()))?;
    e.recorded_at = now_ms().max(last.map_or(i64::MIN, |x| x.1));
    validate_links(conn, &e)?;
    if let Some(existing) = existing {
        return Ok(existing);
    }
    let body = codec::encode(&e, compression)?;
    conn.execute("INSERT INTO records VALUES(?1,?2)", params![e.id, body])?;
    project(conn, &e)?;
    Ok(e)
}
fn reference(conn: &Connection, e: &Event, id: i64) -> Result<Event> {
    if id <= 0 || id >= e.id {
        return Err(Error::Invalid("reference must precede event".into()));
    }
    let target = get(conn, id, false)?;
    if target.scope != e.scope {
        return Err(Error::Invalid("cross-scope reference".into()));
    }
    Ok(target)
}
fn validate_links(conn: &Connection, e: &Event) -> Result<()> {
    e.validate(true)?;
    match &e.kind {
        Kind::Fact {
            slot,
            previous,
            restored_from,
            valid_from,
            valid_until,
        } => {
            let current = head(conn, &e.scope, slot)?;
            if current.as_ref().map(|e| e.id) != *previous {
                return Err(Error::Conflict(
                    "stale expected head or slot already exists".into(),
                ));
            }
            if let Some(prev) = previous {
                let p = reference(conn, e, *prev)?;
                if let Kind::Fact {
                    slot: s,
                    valid_from: f,
                    valid_until: u,
                    ..
                } = p.kind
                {
                    if s != *slot || f != *valid_from || u != *valid_until {
                        return Err(Error::Invalid(
                            "changing lineage or validity interval is unsupported".into(),
                        ));
                    }
                } else {
                    return Err(Error::Invalid("previous is not fact".into()));
                }
            }
            if let Some(target) = restored_from {
                let old = reference(conn, e, *target)?;
                if old.kind.slot() != Some(slot) || old.payload != e.payload {
                    return Err(Error::Invalid("restore lineage/value mismatch".into()));
                }
            }
        }
        Kind::Relation {
            relation,
            from,
            to,
            evidence,
        } => {
            if matches!(
                relation,
                RelationKind::UsedEvidence | RelationKind::Supersedes | RelationKind::Restores
            ) {
                return Err(Error::Invalid("relation is transaction-derived".into()));
            }
            if evidence.is_empty() {
                return Err(Error::Invalid("relation needs evidence".into()));
            }
            for id in [*from, *to].iter().chain(evidence) {
                reference(conn, e, *id)?;
            }
        }
        Kind::AssistantAnswer {
            input,
            evidence,
            provided,
            excluded,
            ..
        } => {
            let question = reference(conn, e, *input)?;
            if !matches!(question.kind, Kind::Observation { question: Some(_) })
                || question.session != e.session
            {
                return Err(Error::Invalid("answer input must be question".into()));
            }
            for id in evidence.iter().chain(provided).chain(excluded) {
                let target = reference(conn, e, *id)?;
                if *id >= *input
                    || matches!(
                        target.kind,
                        Kind::AssistantAnswer { .. }
                            | Kind::Failure { .. }
                            | Kind::Observation { question: Some(_) }
                    )
                {
                    return Err(Error::Invalid("invalid answer evidence".into()));
                }
            }
        }
        Kind::Failure { input, .. } => {
            let q = reference(conn, e, *input)?;
            if !matches!(q.kind, Kind::Observation { question: Some(_) }) || q.session != e.session
            {
                return Err(Error::Invalid("failure input must be question".into()));
            }
        }
        _ => {}
    }
    Ok(())
}
pub fn valid_at_time(e: &Event, at: i64) -> bool {
    match e.kind {
        Kind::Fact {
            valid_from,
            valid_until,
            ..
        } => valid_from.is_none_or(|n| n <= at) && valid_until.is_none_or(|n| at < n),
        _ => true,
    }
}
pub(crate) fn prefix(s: &str, max: usize) -> (&str, bool) {
    let mut end = s.len().min(max);
    while !s.is_char_boundary(end) {
        end -= 1;
    }
    (&s[..end], end < s.len())
}
pub(crate) fn normalized(e: &Event) -> String {
    prefix(
        std::str::from_utf8(&e.payload).expect("validated UTF-8"),
        16384,
    )
    .0
    .to_lowercase()
}
fn project(conn: &Connection, e: &Event) -> Result<()> {
    let mut values = vec![rusqlite::types::Value::Integer(e.id)];
    values.extend(metadata(e));
    conn.execute(
        "INSERT INTO record_meta VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11)",
        rusqlite::params_from_iter(values),
    )?;
    if let Some(key) = e.request_key {
        conn.execute(
            "INSERT INTO request_state VALUES(?1,?2,?3)",
            params![e.scope, key.as_slice(), e.id],
        )?;
    }
    if let Some(input) = e.kind.input() {
        conn.execute("INSERT INTO results VALUES(?1,?2)", params![input, e.id])?;
    }
    conn.execute(
        "INSERT INTO record_fts(rowid,text) VALUES(?1,?2)",
        params![e.id, normalized(e)],
    )?;
    let edge = |from, to, kind: RelationKind| -> Result<()> {
        conn.execute(
            "INSERT INTO relations VALUES(?1,?2,?3,?4)",
            params![e.id, from, to, kind as u8],
        )?;
        Ok(())
    };
    match &e.kind {
        Kind::Fact {
            slot,
            previous,
            restored_from,
            ..
        } => {
            conn.execute("INSERT INTO current_heads VALUES(?1,?2) ON CONFLICT(slot) DO UPDATE SET event_id=excluded.event_id",params![slot.key(&e.scope),e.id])?;
            if let Some(p) = previous {
                edge(e.id, *p, RelationKind::Supersedes)?;
            }
            if let Some(r) = restored_from {
                edge(e.id, *r, RelationKind::Restores)?;
            }
        }
        Kind::Relation {
            relation, from, to, ..
        } => edge(*from, *to, *relation)?,
        Kind::AssistantAnswer { evidence, .. } => {
            for id in evidence {
                edge(e.id, *id, RelationKind::UsedEvidence)?;
            }
        }
        _ => {}
    }
    conn.execute(
        "UPDATE projection_state SET last_id=?1 WHERE singleton=1",
        [e.id],
    )?;
    Ok(())
}
fn table_rows(conn: &Connection, table: &str) -> Result<Vec<Vec<rusqlite::types::Value>>> {
    let columns = if table == "record_fts" {
        "rowid,*"
    } else {
        "*"
    };
    let mut stmt = conn.prepare(&format!("SELECT {columns} FROM {table} ORDER BY 1,2"))?;
    let count = stmt.column_count();
    Ok(stmt
        .query_map([], |r| {
            (0..count)
                .map(|i| r.get(i))
                .collect::<std::result::Result<Vec<_>, _>>()
        })?
        .collect::<std::result::Result<Vec<_>, _>>()?)
}

#[cfg(feature = "test-support")]
pub fn test_pause(point: &str) {
    if std::env::var("REPLICA_TEST_PAUSE").as_deref() == Ok(point)
        && let Ok(marker) = std::env::var("REPLICA_TEST_MARKER")
    {
        std::fs::write(marker, b"ready").expect("test marker");
        loop {
            std::thread::sleep(std::time::Duration::from_millis(20));
        }
    }
}
