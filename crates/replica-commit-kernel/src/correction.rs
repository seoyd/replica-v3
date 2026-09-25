//! K/L version/dependency boundary. Historical reads confer no current authority.
//! No language-model integration, external effect execution or JSON persistence.
use crate::durable::persist_bytes;
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet, VecDeque},
    fs::{self, File, OpenOptions},
    io,
    path::{Path, PathBuf},
};

pub type Key = (String, u64);
const MAX_RECORDS: usize = 200_000;
const MAX_BYTES: u64 = 64 * 1024 * 1024;
fn invalid(message: &str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, message)
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Kind {
    Evidence,
    Claim,
    Action,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Record {
    pub id: String,
    pub version: u64,
    pub kind: Kind,
    /// Epistemic label is preserved, not upgraded by this consistency layer.
    pub status: String,
    pub payload: String,
    pub deps: Vec<Key>,
    pub correction_of: Option<Key>,
    pub executed_external: bool,
}
impl Record {
    pub fn key(&self) -> Key {
        (self.id.clone(), self.version)
    }
    fn validate_shape(&self) -> io::Result<()> {
        if self.id.is_empty()
            || self.id.len() > 256
            || self.version == 0
            || self.status.len() > 256
            || self.payload.len() > 65536
            || self.deps.len() > 256
            || self.deps.iter().collect::<BTreeSet<_>>().len() != self.deps.len()
            || (self.executed_external && self.kind != Kind::Action)
        {
            return Err(invalid("invalid correction record"));
        }
        Ok(())
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Validity {
    Active,
    Superseded,
    Stale,
    CompensationRequired,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Snapshot {
    pub format: String,
    pub version: u32,
    pub records: Vec<Record>,
    pub heads: BTreeMap<String, u64>,
}
#[derive(Clone, Debug, Default)]
pub struct Store {
    records: BTreeMap<Key, Record>,
    heads: BTreeMap<String, u64>,
    reverse: BTreeMap<Key, BTreeSet<Key>>,
    cache: BTreeMap<Key, Validity>,
}
impl Store {
    /// Immutable historical access; callers must separately validate current use.
    pub fn record(&self, key: &Key) -> Option<&Record> {
        self.records.get(key)
    }
    pub fn head(&self, id: &str) -> Option<u64> {
        self.heads.get(id).copied()
    }
    pub fn len(&self) -> usize {
        self.records.len()
    }
    pub fn is_empty(&self) -> bool {
        self.records.is_empty()
    }
    /// Iterative closure traversal also fails closed for a missing/cyclic edge.
    /// Cache is deliberately absent from this authority path.
    pub fn is_current(&self, key: &Key) -> bool {
        let mut todo = vec![(key.clone(), false)];
        let mut visiting = BTreeSet::new();
        let mut done = BTreeSet::new();
        while let Some((k, exit)) = todo.pop() {
            if exit {
                visiting.remove(&k);
                done.insert(k);
                continue;
            }
            if done.contains(&k) {
                continue;
            }
            if !visiting.insert(k.clone()) {
                return false;
            }
            let Some(r) = self.records.get(&k) else {
                return false;
            };
            if self.heads.get(&r.id) != Some(&r.version) {
                return false;
            }
            todo.push((k, true));
            for d in r.deps.iter().rev() {
                todo.push((d.clone(), false));
            }
        }
        true
    }
    pub fn validity(&self, key: &Key) -> io::Result<Validity> {
        let r = self
            .records
            .get(key)
            .ok_or_else(|| invalid("missing record"))?;
        if self.is_current(key) {
            Ok(Validity::Active)
        } else if r.kind == Kind::Action && r.executed_external {
            Ok(Validity::CompensationRequired)
        } else if self.heads.get(&r.id) != Some(&r.version) {
            Ok(Validity::Superseded)
        } else {
            Ok(Validity::Stale)
        }
    }
    pub fn append(&mut self, record: Record) -> io::Result<Key> {
        record.validate_shape()?;
        let key = record.key();
        if self.records.len() >= MAX_RECORDS || self.records.contains_key(&key) {
            return Err(invalid("duplicate version or record limit"));
        }
        match self.head(&record.id) {
            Some(old)
                if old.checked_add(1) == Some(record.version)
                    && record.correction_of == Some((record.id.clone(), old)) => {}
            None if record.version == 1 && record.correction_of.is_none() => {}
            _ => return Err(invalid("invalid version/head correction")),
        }
        // Reject self-history dependencies which would become stale at head switch.
        for dep in &record.deps {
            if dep.0 == record.id || !self.is_current(dep) {
                return Err(invalid("REJECT_STALE_CLOSURE"));
            }
            // A transitive dependency on our old version has the same problem.
            let mut pending = vec![dep.clone()];
            let mut seen = BTreeSet::new();
            while let Some(k) = pending.pop() {
                if !seen.insert(k.clone()) {
                    continue;
                }
                if k.0 == record.id {
                    return Err(invalid("self dependency through closure"));
                }
                pending.extend(self.records[&k].deps.iter().cloned());
            }
        }
        let old = record.correction_of.clone();
        for dep in &record.deps {
            self.reverse
                .entry(dep.clone())
                .or_default()
                .insert(key.clone());
        }
        self.heads.insert(record.id.clone(), record.version);
        self.records.insert(key.clone(), record);
        self.cache.insert(key.clone(), Validity::Active);
        if let Some(old) = old {
            self.cache.insert(old.clone(), self.validity(&old)?);
            let mut queue = VecDeque::from([old]);
            let mut seen = BTreeSet::new();
            while let Some(k) = queue.pop_front() {
                if let Some(children) = self.reverse.get(&k) {
                    for child in children {
                        if seen.insert(child.clone()) {
                            let r = &self.records[child];
                            self.cache.insert(
                                child.clone(),
                                if r.executed_external {
                                    Validity::CompensationRequired
                                } else {
                                    Validity::Stale
                                },
                            );
                            queue.push_back(child.clone());
                        }
                    }
                }
            }
        }
        Ok(key)
    }
    pub fn snapshot(&self) -> Snapshot {
        Snapshot {
            format: "replica-correction-native".into(),
            version: 1,
            records: self.records.values().cloned().collect(),
            heads: self.heads.clone(),
        }
    }
    pub fn restore(snapshot: Snapshot) -> io::Result<Self> {
        if snapshot.format != "replica-correction-native"
            || snapshot.version != 1
            || snapshot.records.len() > MAX_RECORDS
        {
            return Err(invalid("bad correction header/limit"));
        }
        let mut store = Self {
            heads: snapshot.heads,
            ..Self::default()
        };
        let mut max = BTreeMap::new();
        for r in snapshot.records {
            r.validate_shape()?;
            if store.records.insert(r.key(), r.clone()).is_some() {
                return Err(invalid("duplicate record"));
            }
            max.entry(r.id)
                .and_modify(|v: &mut u64| *v = (*v).max(r.version))
                .or_insert(r.version);
        }
        if max != store.heads {
            return Err(invalid("dangling or regressed head"));
        }
        let mut indegree = BTreeMap::new();
        for (k, r) in &store.records {
            if r.version == 1 {
                if r.correction_of.is_some() {
                    return Err(invalid("invalid initial correction"));
                }
            } else if r.correction_of != Some((r.id.clone(), r.version - 1))
                || !store.records.contains_key(&(r.id.clone(), r.version - 1))
            {
                return Err(invalid("missing previous version"));
            }
            indegree.insert(k.clone(), r.deps.len());
            for d in &r.deps {
                if !store.records.contains_key(d) || d.0 == r.id {
                    return Err(invalid("dangling/self dependency"));
                }
                store
                    .reverse
                    .entry(d.clone())
                    .or_default()
                    .insert(k.clone());
            }
        }
        let mut ready: VecDeque<_> = indegree
            .iter()
            .filter(|(_, n)| **n == 0)
            .map(|(k, _)| k.clone())
            .collect();
        let mut visited = 0;
        while let Some(k) = ready.pop_front() {
            visited += 1;
            if let Some(children) = store.reverse.get(&k) {
                for c in children {
                    let n = indegree.get_mut(c).unwrap();
                    *n -= 1;
                    if *n == 0 {
                        ready.push_back(c.clone());
                    }
                }
            }
        }
        if visited != store.records.len() {
            return Err(invalid("cyclic dependency"));
        }
        for key in store.records.keys() {
            store.cache.insert(key.clone(), store.validity(key)?);
        }
        Ok(store)
    }
}

pub trait SnapshotCodec {
    fn encode(&self, snapshot: &Snapshot) -> io::Result<Vec<u8>>;
    fn decode(&self, bytes: &[u8]) -> io::Result<Snapshot>;
}
/// One exclusive disk writer. A batch appends versions and switches heads in
/// one native snapshot. Advisory cache files are never read during restore/use.
pub struct DurableStore<C> {
    root: PathBuf,
    codec: C,
    store: Store,
    _lock: File,
    serial: u64,
    poisoned: bool,
}
impl<C: SnapshotCodec> DurableStore<C> {
    fn lock(root: &Path) -> io::Result<File> {
        let f = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(root.join("correction.lock"))?;
        f.try_lock().map_err(io::Error::other)?;
        Ok(f)
    }
    pub fn create(root: &Path, store: Store, codec: C) -> io::Result<Self> {
        fs::create_dir(root)?;
        let lock = Self::lock(root)?;
        let mut s = Self {
            root: root.into(),
            codec,
            store,
            _lock: lock,
            serial: 0,
            poisoned: false,
        };
        let bytes = s.codec.encode(&s.store.snapshot())?;
        persist_bytes(
            root,
            "correction",
            &mut s.serial,
            &bytes,
            MAX_BYTES,
            &mut |_| Ok(()),
        )?;
        Ok(s)
    }
    pub fn open(root: &Path, codec: C) -> io::Result<Self> {
        let lock = Self::lock(root)?;
        let path = root.join("correction.state.r3b");
        let meta = fs::symlink_metadata(&path)?;
        if !meta.is_file() || meta.len() > MAX_BYTES {
            return Err(invalid("invalid correction file/size"));
        }
        let store = Store::restore(codec.decode(&fs::read(path)?)?)?;
        Ok(Self {
            root: root.into(),
            codec,
            store,
            _lock: lock,
            serial: 0,
            poisoned: false,
        })
    }
    pub fn store(&self) -> io::Result<&Store> {
        if self.poisoned {
            Err(invalid("disk-only reopen required"))
        } else {
            Ok(&self.store)
        }
    }
    pub fn append_batch(
        &mut self,
        records: &[Record],
        hook: &mut impl FnMut(&str) -> io::Result<()>,
    ) -> io::Result<()> {
        let mut next = self.store()?.clone();
        for r in records {
            next.append(r.clone())?;
        }
        let bytes = self.codec.encode(&next.snapshot())?;
        if let Err(e) = persist_bytes(
            &self.root,
            "correction",
            &mut self.serial,
            &bytes,
            MAX_BYTES,
            hook,
        ) {
            self.poisoned = true;
            return Err(e);
        }
        self.store = next;
        if let Err(e) = hook("reply") {
            self.poisoned = true;
            return Err(e);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn lying_and_half_propagated_cache_has_no_authority() {
        let mut s = Store::default();
        let make = |id: &str, version, deps, correction_of| Record {
            id: id.into(),
            version,
            kind: Kind::Claim,
            status: "UNKNOWN".into(),
            payload: String::new(),
            deps,
            correction_of,
            executed_external: false,
        };
        s.append(make("root", 1, vec![], None)).unwrap();
        s.append(make("claim", 1, vec![("root".into(), 1)], None))
            .unwrap();
        s.append(make("action", 1, vec![("claim".into(), 1)], None))
            .unwrap();
        s.append(make("root", 2, vec![], Some(("root".into(), 1))))
            .unwrap();
        s.cache.insert(("action".into(), 1), Validity::Active);
        s.cache.remove(&("claim".into(), 1));
        assert!(!s.is_current(&("action".into(), 1)));
        assert!(s
            .append(make("attempt", 1, vec![("action".into(), 1)], None))
            .is_err());
        let restored = Store::restore(s.snapshot()).unwrap();
        assert_eq!(restored.cache[&("action".into(), 1)], Validity::Stale);
    }
}
