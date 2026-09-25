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
const MAX_TEMP_ATTEMPTS: usize = 1024;
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
        persist_bytes(
            &self.root,
            "kernel",
            &mut self.serial,
            &bytes,
            MAX_SNAPSHOT,
            hook,
        )
    }
}

/// Shared atomic native-file boundary; callers validate their own typed state.
pub(crate) fn persist_bytes(
    root: &Path,
    name: &str,
    serial: &mut u64,
    bytes: &[u8],
    max: u64,
    hook: &mut impl FnMut(&str) -> io::Result<()>,
) -> io::Result<()> {
    if bytes.len() as u64 > max {
        return Err(invalid("snapshot size limit"));
    }
    let (temp, mut file) = allocate_temp(root, name, serial)?;
    hook("temp-open")?;
    file.write_all(bytes)?;
    hook("write")?;
    file.sync_all()?;
    hook("file-sync")?;
    drop(file);
    hook("close")?;
    fs::rename(&temp, root.join(format!("{name}.state.r3b")))?;
    hook("rename")?;
    File::open(root)?.sync_all()?;
    hook("directory-sync")?;
    Ok(())
}

fn allocate_temp(root: &Path, name: &str, serial: &mut u64) -> io::Result<(PathBuf, File)> {
    for _ in 0..MAX_TEMP_ATTEMPTS {
        *serial = serial
            .checked_add(1)
            .ok_or_else(|| invalid("snapshot serial overflow"))?;
        let temp = root.join(format!("{name}.{}.{serial}.tmp", std::process::id()));
        match OpenOptions::new().write(true).create_new(true).open(&temp) {
            Ok(file) => return Ok((temp, file)),
            Err(e) if e.kind() == io::ErrorKind::AlreadyExists => continue,
            Err(e) => return Err(e),
        }
    }
    Err(io::Error::new(
        io::ErrorKind::AlreadyExists,
        "snapshot temp allocation attempts exhausted",
    ))
}

#[cfg(test)]
mod temp_recovery_tests {
    use super::*;

    #[test]
    #[ignore = "explicit isolated evidence root; real filesystem allocation boundaries"]
    fn allocation_boundaries() {
        let base = PathBuf::from(std::env::var_os("R3_TEMP_EVIDENCE").unwrap());
        let root = base.join("allocation");
        fs::create_dir(&root).unwrap();
        let canonical = root.join("probe.state.r3b");
        fs::write(&canonical, b"old").unwrap();
        let candidate = |n| root.join(format!("probe.{}.{n}.tmp", std::process::id()));
        for n in 1..=MAX_TEMP_ATTEMPTS {
            fs::write(candidate(n), b"stale").unwrap();
        }
        let mut serial = 0;
        let mut calls = Vec::new();
        let error = persist_bytes(&root, "probe", &mut serial, b"new", 3, &mut |at| {
            calls.push(at.to_owned());
            Ok(())
        })
        .unwrap_err();
        assert_eq!(error.kind(), io::ErrorKind::AlreadyExists);
        assert!(error.to_string().contains("attempts exhausted"));
        assert_eq!(serial, 1024);
        assert!(calls.is_empty());
        assert!(!candidate(1025).exists());
        assert_eq!(fs::read(&canonical).unwrap(), b"old");
        for n in 1..=MAX_TEMP_ATTEMPTS {
            assert_eq!(fs::read(candidate(n)).unwrap(), b"stale");
        }
        // Overflow and size validation cannot create or publish anything.
        serial = u64::MAX;
        assert!(
            persist_bytes(&root, "probe", &mut serial, b"new", 3, &mut |_| panic!(
                "hook"
            ))
            .unwrap_err()
            .to_string()
            .contains("overflow")
        );
        assert_eq!(serial, u64::MAX);
        serial = 0;
        assert!(
            persist_bytes(&root, "probe", &mut serial, b"large", 3, &mut |_| panic!(
                "hook"
            ))
            .is_err()
        );
        assert_eq!(serial, 0);
        // Missing directory gives an actual OS open error, forwarded without retry.
        let absent = root.join("absent");
        let expected = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(absent.join("x"))
            .unwrap_err();
        let error = persist_bytes(&absent, "probe", &mut serial, b"new", 3, &mut |_| {
            panic!("hook")
        })
        .unwrap_err();
        assert_eq!(error.kind(), expected.kind());
        assert_eq!(error.raw_os_error(), expected.raw_os_error());
        assert_eq!(serial, 1);
        // A free candidate is used once. AlreadyExists AFTER open is not retried.
        serial = 1024;
        let mut count = 0;
        let error = persist_bytes(&root, "probe", &mut serial, b"new", 3, &mut |_| {
            count += 1;
            Err(io::Error::from(io::ErrorKind::AlreadyExists))
        })
        .unwrap_err();
        assert_eq!(error.kind(), io::ErrorKind::AlreadyExists);
        assert_eq!(count, 1);
        assert_eq!(serial, 1025);
        assert!(!candidate(1026).exists());
        assert_eq!(fs::read(&canonical).unwrap(), b"old");
        calls.clear();
        persist_bytes(&root, "probe", &mut serial, b"new", 3, &mut |at| {
            calls.push(at.to_owned());
            Ok(())
        })
        .unwrap();
        assert_eq!(
            calls,
            [
                "temp-open",
                "write",
                "file-sync",
                "close",
                "rename",
                "directory-sync"
            ]
        );
        assert_eq!(fs::read(&canonical).unwrap(), b"new");
        #[cfg(unix)]
        {
            let link = root.join("symlink");
            fs::create_dir(&link).unwrap();
            let target = link.join("missing-target");
            let temp = link.join(format!("probe.{}.1.tmp", std::process::id()));
            std::os::unix::fs::symlink(&target, &temp).unwrap();
            serial = 0;
            persist_bytes(&link, "probe", &mut serial, b"new", 3, &mut |_| Ok(())).unwrap();
            assert_eq!(serial, 2);
            assert_eq!(fs::read_link(temp).unwrap(), target);
            assert!(!target.exists());
        }
        println!("allocation: limit=1024 overflow/size/OS-open/hook-AlreadyExists/symlink PASS");
    }
}
