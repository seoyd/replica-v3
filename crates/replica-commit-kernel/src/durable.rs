//! Single-writer snapshot boundary. The host supplies its existing native codec.
//! No JSON persistence, model dependency, temp/backup promotion, or power-loss claim.
use crate::kernel::{Event, JournalEntry, Kernel, State};
use serde::{Deserialize, Serialize};
use std::{
    fs::{self, File, OpenOptions},
    io::{self, Write},
    path::{Path, PathBuf},
};

const MAX_SNAPSHOT: u64 = 8 * 1024 * 1024;
const MAX_EVENTS: usize = 100_000;
fn invalid(message: &str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, message)
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Snapshot {
    pub version: u32,
    pub initial: State,
    pub events: Vec<Event>,
    pub journal: Vec<JournalEntry>,
    pub state: State,
}
impl Snapshot {
    pub fn validate(&self) -> io::Result<()> {
        if self.version != 1
            || self.events.len() > MAX_EVENTS
            || !self.initial.operation_ledger.is_empty()
            || !self.initial.used_nonces.is_empty()
            || self
                .initial
                .authorized_capability_ids
                .windows(2)
                .any(|p| p[0] >= p[1])
        {
            return Err(invalid(
                "invalid kernel snapshot initial state/version/limits",
            ));
        }
        let mut kernel = Kernel::new(self.initial.clone());
        let journal: Vec<_> = self.events.iter().map(|e| kernel.apply(e)).collect();
        if journal != self.journal || kernel.into_state() != self.state {
            return Err(invalid("kernel snapshot replay/state/receipt mismatch"));
        }
        Ok(())
    }
}

/// Adapter boundary: Replica's host/test uses R3BIN, not a new persistent schema codec.
pub trait SnapshotCodec {
    fn encode(&self, snapshot: &Snapshot) -> io::Result<Vec<u8>>;
    fn decode(&self, bytes: &[u8]) -> io::Result<Snapshot>;
}

pub struct DurableKernel<C> {
    root: PathBuf,
    codec: C,
    snapshot: Snapshot,
    _lock: File,
    poisoned: bool,
    serial: u64,
}
impl<C: SnapshotCodec> DurableKernel<C> {
    fn lock(root: &Path) -> io::Result<File> {
        let lock = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(root.join("kernel.lock"))?;
        lock.try_lock().map_err(io::Error::other)?;
        Ok(lock)
    }
    pub fn create(root: &Path, initial: State, codec: C) -> io::Result<Self> {
        fs::create_dir(root)?; // explicit new store; never replaces an existing store
        let lock = Self::lock(root)?;
        let snapshot = Snapshot {
            version: 1,
            initial: initial.clone(),
            state: initial,
            events: vec![],
            journal: vec![],
        };
        snapshot.validate()?;
        let mut store = Self {
            root: root.into(),
            codec,
            snapshot,
            _lock: lock,
            poisoned: false,
            serial: 0,
        };
        store.persist(&store.snapshot.clone(), &mut |_| Ok(()))?;
        Ok(store)
    }
    pub fn open(root: &Path, codec: C) -> io::Result<Self> {
        let lock = Self::lock(root)?;
        let committed = root.join("kernel.state.r3b");
        let metadata = fs::symlink_metadata(&committed)?;
        if !metadata.is_file() || metadata.len() > MAX_SNAPSHOT {
            return Err(invalid("invalid committed snapshot file/size"));
        }
        let snapshot = codec.decode(&fs::read(committed)?)?;
        snapshot.validate()?; // temps, newer backups and advisory caches are never consulted
        Ok(Self {
            root: root.into(),
            codec,
            snapshot,
            _lock: lock,
            poisoned: false,
            serial: 0,
        })
    }
    pub fn snapshot(&self) -> io::Result<&Snapshot> {
        if self.poisoned {
            return Err(invalid(
                "store requires disk-only reopen after uncertain save",
            ));
        }
        Ok(&self.snapshot)
    }
    pub fn apply(&mut self, event: &Event) -> io::Result<JournalEntry> {
        self.apply_with_hook(event, &mut |_| Ok(()))
    }
    /// Host fault/process tests observe exact boundaries; normal callers use `apply`.
    pub fn apply_with_hook(
        &mut self,
        event: &Event,
        hook: &mut impl FnMut(&str) -> io::Result<()>,
    ) -> io::Result<JournalEntry> {
        let mut next = self.snapshot()?.clone();
        if next.events.len() == MAX_EVENTS {
            return Err(invalid("kernel event limit"));
        }
        let mut kernel = Kernel::new(next.state.clone());
        let reply = kernel.apply(event);
        if reply.outcome == "REJECT_EVENT" {
            return Err(invalid("invalid event"));
        }
        next.events.push(event.clone());
        next.journal.push(reply.clone());
        next.state = kernel.into_state();
        if let Err(error) = self.persist(&next, hook) {
            self.poisoned = true;
            return Err(error);
        }
        self.snapshot = next;
        // The committed state is durable before the reply is exposed.
        if let Err(error) = hook("reply") {
            self.poisoned = true;
            return Err(error);
        }
        Ok(reply)
    }
    fn persist(
        &mut self,
        snapshot: &Snapshot,
        hook: &mut impl FnMut(&str) -> io::Result<()>,
    ) -> io::Result<()> {
        let bytes = self.codec.encode(snapshot)?;
        if bytes.len() as u64 > MAX_SNAPSHOT {
            return Err(invalid("kernel snapshot size limit"));
        }
        self.serial = self
            .serial
            .checked_add(1)
            .ok_or_else(|| invalid("snapshot serial overflow"))?;
        let temp = self
            .root
            .join(format!("kernel.{}.{}.tmp", std::process::id(), self.serial));
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temp)?;
        hook("temp-open")?;
        file.write_all(&bytes)?;
        hook("write")?;
        file.sync_all()?;
        hook("file-sync")?;
        drop(file);
        hook("close")?;
        fs::rename(&temp, self.root.join("kernel.state.r3b"))?;
        hook("rename")?;
        File::open(&self.root)?.sync_all()?;
        hook("directory-sync")?;
        Ok(())
    }
}
