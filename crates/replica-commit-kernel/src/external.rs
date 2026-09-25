//! I/J/M local external-effect protocol. Trusted host supplies transport and
//! attributable observations; no model, network client or general inverse maker.
use crate::{
    durable::persist_bytes,
    kernel::{effect_digest, Effect as DigestEffect},
};
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    fs::{self, File, OpenOptions},
    io,
    path::{Path, PathBuf},
};
const MAX_TASKS: usize = 10_000;
const MAX_BYTES: u64 = 32 * 1024 * 1024;
fn bad(s: &str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, s)
}
fn require(ok: bool, s: &str) -> io::Result<()> {
    if ok {
        Ok(())
    } else {
        Err(bad(s))
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Effect {
    pub receiver: String,
    pub id: String,
    pub delta: i64,
}
impl Effect {
    pub fn digest(&self) -> String {
        // Length-prefixed receiver/effect IDs prevent concatenation ambiguity.
        effect_digest(&DigestEffect {
            kind: "EXTERNAL_DELTA_V1".into(),
            target: format!(
                "{}:{}{}:{}",
                self.receiver.len(),
                self.receiver,
                self.id.len(),
                self.id
            ),
            delta: self.delta,
        })
    }
    fn validate(&self) -> io::Result<()> {
        require(
            !self.receiver.is_empty()
                && self.receiver.len() <= 128
                && !self.id.is_empty()
                && self.id.len() <= 512,
            "invalid effect identity",
        )
    }
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Receipt {
    pub effect: Effect,
    pub digest: String,
    pub sequence: u64,
    pub value_after: i64,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum DeliveryKind {
    Commit,
    Replay,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Delivery {
    pub kind: DeliveryKind,
    pub receipt: Receipt,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Observed {
    Applied,
    Failed,
    Partial,
    Absent,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Observation {
    pub receiver: String,
    pub effect_id: String,
    pub digest: Option<String>,
    pub status: Observed,
    pub authoritative: bool,
    pub fresh: bool,
    pub closed_world: bool,
    pub watermark_closed: bool,
}
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Evidence {
    pub receipt: Option<Observation>,
    pub query: Option<Observation>,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Resolution {
    VerifiedSuccess,
    VerifiedFailure,
    Unknown,
    Disputed,
    PartialEffect,
}
/// Authoritativeness/freshness/watermark are supplied by a trusted adapter, never
/// by retrieved text or a model. Closed absence requires settled/fenced delivery.
pub fn reconcile(effect: &Effect, evidence: &Evidence) -> Resolution {
    let (mut success, mut failure, mut partial, mut conflict) = (false, false, false, false);
    for (is_query, o) in [
        (false, evidence.receipt.as_ref()),
        (true, evidence.query.as_ref()),
    ] {
        let Some(o) = o else { continue };
        if !o.authoritative || !o.fresh || o.receiver != effect.receiver || o.effect_id != effect.id
        {
            continue;
        }
        if is_query && o.status == Observed::Absent {
            if o.closed_world && o.watermark_closed {
                failure = true;
            }
        } else if matches!(o.status, Observed::Applied | Observed::Failed)
            || (!is_query && o.status == Observed::Partial)
        {
            if o.digest.as_deref() != Some(effect.digest().as_str()) {
                conflict = true;
            } else {
                match o.status {
                    Observed::Applied => success = true,
                    Observed::Failed => failure = true,
                    Observed::Partial => partial = true,
                    _ => {}
                }
            }
        }
    }
    // A conflicting attributed digest cannot be hidden by a partial observation.
    if conflict || (success && failure) || (partial && (success || failure)) {
        Resolution::Disputed
    } else if partial {
        Resolution::PartialEffect
    } else if success {
        Resolution::VerifiedSuccess
    } else if failure {
        Resolution::VerifiedFailure
    } else {
        Resolution::Unknown
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Receiver {
    pub id: String,
    pub idempotent: bool,
    pub initial: i64,
    pub value: i64,
    pub journal: Vec<Receipt>,
    pub ledger: BTreeMap<String, Receipt>,
}
impl Receiver {
    pub fn new(id: &str, initial: i64, idempotent: bool) -> Self {
        Self {
            id: id.into(),
            initial,
            value: initial,
            idempotent,
            journal: vec![],
            ledger: BTreeMap::new(),
        }
    }
    fn apply(&mut self, e: &Effect) -> io::Result<Delivery> {
        e.validate()?;
        require(e.receiver == self.id, "receiver mismatch")?;
        if self.idempotent {
            if let Some(r) = self.ledger.get(&e.id) {
                require(r.effect == *e, "REJECT_EFFECT_MISMATCH")?;
                return Ok(Delivery {
                    kind: DeliveryKind::Replay,
                    receipt: r.clone(),
                });
            }
        }
        require(self.journal.len() < MAX_TASKS, "receiver journal full")?;
        let value = self
            .value
            .checked_add(e.delta)
            .ok_or_else(|| bad("REJECT_OVERFLOW"))?;
        let r = Receipt {
            effect: e.clone(),
            digest: e.digest(),
            sequence: self.journal.len() as u64 + 1,
            value_after: value,
        };
        self.value = value;
        self.journal.push(r.clone());
        if self.idempotent {
            self.ledger.insert(e.id.clone(), r.clone());
        }
        Ok(Delivery {
            kind: DeliveryKind::Commit,
            receipt: r,
        })
    }
    pub fn query(&self, e: &Effect) -> Observation {
        let found = self.ledger.get(&e.id);
        Observation {
            receiver: self.id.clone(),
            effect_id: e.id.clone(),
            digest: found.map(|r| r.digest.clone()),
            status: if found.is_some() {
                Observed::Applied
            } else {
                Observed::Absent
            },
            authoritative: self.idempotent,
            fresh: true,
            closed_world: self.idempotent,
            // The host must establish no older send can subsequently arrive.
            watermark_closed: false,
        }
    }
    fn validate(&self) -> io::Result<()> {
        require(
            !self.id.is_empty() && self.id.len() <= 128 && self.journal.len() <= MAX_TASKS,
            "receiver bounds",
        )?;
        let mut replay = Self::new(&self.id, self.initial, self.idempotent);
        for r in &self.journal {
            let applied = replay.apply(&r.effect)?;
            require(
                applied.kind == DeliveryKind::Commit && applied.receipt == *r,
                "receiver effect/ledger/receipt mismatch",
            )?;
        }
        require(replay == *self, "receiver state mismatch")
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum TaskStatus {
    Prepared,
    InFlight,
    Acked,
    Cancelled,
    UnknownEffect,
    ReconcileRequired,
    Disputed,
    PartialEffect,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum TaskKind {
    Original,
    Compensate { cause: u64 },
    Restore { cause: u64, compensation: String },
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Task {
    pub effect: Effect,
    pub kind: TaskKind,
    pub status: TaskStatus,
    pub attempts: u64,
    pub receipt: Option<Receipt>,
    pub evidence: Vec<Evidence>,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Compensator {
    pub original_id: String,
    /// Both actions are explicitly declared by the host; never inferred inverses.
    pub compensation_delta: i64,
    pub restore_delta: i64,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Sender {
    pub receiver: String,
    pub receiver_idempotent: bool,
    pub tasks: BTreeMap<String, Task>,
    pub world: u64,
    pub original_valid: bool,
    pub compensator: Option<Compensator>,
    pub manual_intervention: bool,
}
impl Sender {
    pub fn new(receiver: &str, idempotent: bool, compensator: Option<Compensator>) -> Self {
        Self {
            receiver: receiver.into(),
            receiver_idempotent: idempotent,
            tasks: BTreeMap::new(),
            world: 1,
            original_valid: true,
            compensator,
            manual_intervention: false,
        }
    }
    fn prepare(&mut self, e: Effect, kind: TaskKind) -> io::Result<()> {
        e.validate()?;
        require(e.receiver == self.receiver, "receiver mismatch")?;
        if let Some(old) = self.tasks.get(&e.id) {
            return require(
                old.effect == e && old.kind == kind,
                "REJECT_EFFECT_MISMATCH",
            );
        }
        require(self.tasks.len() < MAX_TASKS, "sender task limit")?;
        self.tasks.insert(
            e.id.clone(),
            Task {
                effect: e,
                kind,
                status: TaskStatus::Prepared,
                attempts: 0,
                receipt: None,
                evidence: Vec::new(),
            },
        );
        Ok(())
    }
    fn allowed(&self, t: &Task) -> bool {
        match &t.kind {
            TaskKind::Original => true,
            TaskKind::Compensate { cause } => *cause == self.world && !self.original_valid,
            TaskKind::Restore { cause, .. } => *cause == self.world && self.original_valid,
        }
    }
    fn restore_for(&mut self, id: &str) -> io::Result<Option<String>> {
        let t = self.tasks.get(id).ok_or_else(|| bad("missing task"))?;
        if !matches!(t.kind, TaskKind::Compensate { .. }) || !self.original_valid {
            return Ok(None);
        }
        if let Some((id, _)) = self
            .tasks
            .iter()
            .find(|(_, t)| matches!(&t.kind,TaskKind::Restore{compensation,..} if compensation==id))
        {
            return Ok(Some(id.clone()));
        }
        let spec = self
            .compensator
            .as_ref()
            .ok_or_else(|| bad("missing declared compensator"))?;
        let restore = format!("RESTORE:{}:v{}", spec.original_id, self.world);
        let e = Effect {
            receiver: self.receiver.clone(),
            id: restore.clone(),
            delta: spec.restore_delta,
        };
        self.prepare(
            e,
            TaskKind::Restore {
                cause: self.world,
                compensation: id.into(),
            },
        )?;
        Ok(Some(restore))
    }
    fn change_validity(&mut self, valid: bool) -> io::Result<Option<String>> {
        if valid == self.original_valid {
            return Ok(None);
        }
        // One unresolved compensation cycle at a time. Do not issue another
        // effect while a previous restore or reconciliation could still change it.
        if !valid {
            require(
                !self.tasks.values().any(|t| {
                    !matches!(t.kind, TaskKind::Original)
                        && !matches!(t.status, TaskStatus::Acked | TaskStatus::Cancelled)
                }),
                "RECONCILE_REQUIRED",
            )?;
        }
        let previous = self.world;
        self.world = self
            .world
            .checked_add(1)
            .ok_or_else(|| bad("world overflow"))?;
        self.original_valid = valid;
        if !valid {
            let Some(spec) = self.compensator.as_ref() else {
                self.manual_intervention = true;
                return Ok(None);
            };
            let id = format!("COMP:{}:v{}", spec.original_id, self.world);
            self.prepare(
                Effect {
                    receiver: self.receiver.clone(),
                    id: id.clone(),
                    delta: spec.compensation_delta,
                },
                TaskKind::Compensate { cause: self.world },
            )?;
            return Ok(Some(id));
        }
        let ids: Vec<_> = self
            .tasks
            .iter()
            .filter(|(_, t)| matches!(t.kind,TaskKind::Compensate{cause} if cause==previous))
            .map(|(id, _)| id.clone())
            .collect();
        let mut restore = None;
        for id in ids {
            match self.tasks[&id].status {
                TaskStatus::Prepared => {
                    self.tasks.get_mut(&id).unwrap().status = TaskStatus::Cancelled
                }
                TaskStatus::InFlight | TaskStatus::UnknownEffect => {
                    self.tasks.get_mut(&id).unwrap().status = TaskStatus::ReconcileRequired
                }
                TaskStatus::Acked => restore = self.restore_for(&id)?,
                _ => {}
            }
        }
        Ok(restore)
    }
    fn begin(&mut self, id: &str) -> io::Result<Option<Effect>> {
        let t = self
            .tasks
            .get(id)
            .ok_or_else(|| bad("missing task"))?
            .clone();
        if t.status == TaskStatus::Acked {
            return Ok(None);
        }
        if !matches!(t.status, TaskStatus::Prepared | TaskStatus::InFlight) {
            return Ok(None);
        }
        if !self.allowed(&t) {
            self.tasks.get_mut(id).unwrap().status = if t.status == TaskStatus::Prepared {
                TaskStatus::Cancelled
            } else {
                TaskStatus::ReconcileRequired
            };
            return Ok(None);
        }
        if t.status == TaskStatus::InFlight && !self.receiver_idempotent {
            self.tasks.get_mut(id).unwrap().status = TaskStatus::UnknownEffect;
            return Ok(None);
        }
        let t = self.tasks.get_mut(id).unwrap();
        t.attempts = t
            .attempts
            .checked_add(1)
            .ok_or_else(|| bad("attempt overflow"))?;
        t.status = TaskStatus::InFlight;
        Ok(Some(t.effect.clone()))
    }
    fn complete(&mut self, id: &str, delivery: Option<Delivery>) -> io::Result<()> {
        let t = self.tasks.get_mut(id).ok_or_else(|| bad("missing task"))?;
        require(
            matches!(
                t.status,
                TaskStatus::InFlight | TaskStatus::ReconcileRequired
            ),
            "no in-flight task",
        )?;
        if let Some(d) = delivery {
            require(
                d.receipt.effect == t.effect
                    && d.receipt.digest == t.effect.digest()
                    && d.receipt.sequence > 0,
                "receipt identity mismatch",
            )?;
            t.receipt = Some(d.receipt);
            t.status = TaskStatus::Acked;
            self.restore_for(id)?;
        } else if !self.receiver_idempotent && t.status != TaskStatus::ReconcileRequired {
            t.status = TaskStatus::UnknownEffect;
        }
        Ok(())
    }
    fn resolve(&mut self, id: &str, evidence: Evidence) -> io::Result<Resolution> {
        let t = self
            .tasks
            .get(id)
            .ok_or_else(|| bad("missing task"))?
            .clone();
        require(
            matches!(
                t.status,
                TaskStatus::InFlight
                    | TaskStatus::UnknownEffect
                    | TaskStatus::ReconcileRequired
                    | TaskStatus::Disputed
                    | TaskStatus::PartialEffect
            ),
            "not unresolved",
        )?;
        let r = reconcile(&t.effect, &evidence);
        let status = match r {
            Resolution::VerifiedSuccess => TaskStatus::Acked,
            Resolution::VerifiedFailure => {
                if self.allowed(&t) {
                    TaskStatus::Prepared
                } else {
                    TaskStatus::Cancelled
                }
            }
            Resolution::Unknown => TaskStatus::ReconcileRequired,
            Resolution::Disputed => TaskStatus::Disputed,
            Resolution::PartialEffect => TaskStatus::PartialEffect,
        };
        let task = self.tasks.get_mut(id).unwrap();
        task.status = status;
        require(task.evidence.len() < 64, "reconciliation history limit")?;
        task.evidence.push(evidence);
        if r == Resolution::VerifiedSuccess {
            self.restore_for(id)?;
        }
        Ok(r)
    }
    fn validate(&self) -> io::Result<()> {
        require(
            !self.receiver.is_empty()
                && self.receiver.len() <= 128
                && self.world > 0
                && self.tasks.len() <= MAX_TASKS,
            "sender bounds",
        )?;
        if let Some(c) = &self.compensator {
            require(
                !c.original_id.is_empty() && c.original_id.len() <= 128,
                "compensator identity",
            )?;
        }
        for (id, t) in &self.tasks {
            require(t.evidence.len() <= 64, "reconciliation history limit")?;
            t.effect.validate()?;
            require(
                id == &t.effect.id && t.effect.receiver == self.receiver,
                "task identity",
            )?;
            let bound = match &t.kind {
                TaskKind::Original => true,
                TaskKind::Compensate { cause } => self.compensator.as_ref().is_some_and(|s| {
                    *cause <= self.world
                        && *cause > 1
                        && id == &format!("COMP:{}:v{cause}", s.original_id)
                        && t.effect.delta == s.compensation_delta
                }),
                TaskKind::Restore {
                    cause,
                    compensation,
                } => {
                    self.compensator.as_ref().is_some_and(|s| {
                        *cause <= self.world
                            && *cause > 1
                            && id == &format!("RESTORE:{}:v{cause}", s.original_id)
                            && t.effect.delta == s.restore_delta
                    }) && self.tasks.get(compensation).is_some_and(|t| {
                        matches!(t.kind, TaskKind::Compensate { .. })
                            && t.status == TaskStatus::Acked
                    })
                }
            };
            require(bound, "task policy mismatch")?;
            if let Some(r) = &t.receipt {
                require(
                    r.effect == t.effect
                        && r.digest == t.effect.digest()
                        && r.sequence > 0
                        && t.status == TaskStatus::Acked,
                    "invalid saved receipt",
                )?;
            }
            if t.status == TaskStatus::Acked {
                require(
                    t.receipt.is_some()
                        || t.evidence.last().is_some_and(|e| {
                            reconcile(&t.effect, e) == Resolution::VerifiedSuccess
                        }),
                    "acked without evidence",
                )?;
            }
            if matches!(
                t.status,
                TaskStatus::InFlight
                    | TaskStatus::Acked
                    | TaskStatus::UnknownEffect
                    | TaskStatus::ReconcileRequired
                    | TaskStatus::Disputed
                    | TaskStatus::PartialEffect
            ) {
                require(t.attempts > 0, "in-flight without admission")?;
            }
            if t.status == TaskStatus::Prepared && t.attempts > 0 {
                require(
                    t.evidence
                        .last()
                        .is_some_and(|e| reconcile(&t.effect, e) == Resolution::VerifiedFailure),
                    "retry without verified failure",
                )?;
            }
        }
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum State {
    Receiver(Receiver),
    Sender(Sender),
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Snapshot {
    pub format: String,
    pub version: u32,
    pub state: State,
}
impl Snapshot {
    pub fn new(state: State) -> Self {
        Self {
            format: "replica-external-native".into(),
            version: 1,
            state,
        }
    }
    pub fn validate(&self) -> io::Result<()> {
        require(
            self.format == "replica-external-native" && self.version == 1,
            "external snapshot header",
        )?;
        match &self.state {
            State::Receiver(r) => r.validate(),
            State::Sender(s) => s.validate(),
        }
    }
}
pub trait NativeCodec {
    fn encode(&self, s: &Snapshot) -> io::Result<Vec<u8>>;
    fn decode(&self, b: &[u8]) -> io::Result<Snapshot>;
}
/// Exclusive writer per sender/receiver file; transport is outside this adapter.
pub struct Endpoint<C> {
    root: PathBuf,
    codec: C,
    snapshot: Snapshot,
    _lock: File,
    serial: u64,
    poisoned: bool,
}
impl<C: NativeCodec> Endpoint<C> {
    fn lock(root: &Path) -> io::Result<File> {
        let f = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(root.join("external.lock"))?;
        f.try_lock().map_err(|e| match e {
            std::fs::TryLockError::WouldBlock => io::Error::from(io::ErrorKind::WouldBlock),
            std::fs::TryLockError::Error(e) => e,
        })?;
        Ok(f)
    }
    pub fn create(root: &Path, state: State, codec: C) -> io::Result<Self> {
        let snapshot = Snapshot::new(state);
        snapshot.validate()?;
        fs::create_dir(root)?;
        let lock = Self::lock(root)?;
        let mut s = Self {
            root: root.into(),
            codec,
            snapshot,
            _lock: lock,
            serial: 0,
            poisoned: false,
        };
        let bytes = s.codec.encode(&s.snapshot)?;
        persist_bytes(
            root,
            "external",
            &mut s.serial,
            &bytes,
            MAX_BYTES,
            &mut |_| Ok(()),
        )?;
        Ok(s)
    }
    pub fn open(root: &Path, codec: C) -> io::Result<Self> {
        let lock = Self::lock(root)?;
        let path = root.join("external.state.r3b");
        let meta = fs::symlink_metadata(&path)?;
        require(
            meta.is_file() && meta.len() <= MAX_BYTES,
            "external file/size",
        )?;
        let snapshot = codec.decode(&fs::read(path)?)?;
        snapshot.validate()?;
        Ok(Self {
            root: root.into(),
            codec,
            snapshot,
            _lock: lock,
            serial: 0,
            poisoned: false,
        })
    }
    pub fn snapshot(&self) -> io::Result<&Snapshot> {
        require(!self.poisoned, "disk-only reopen required")?;
        Ok(&self.snapshot)
    }
    pub fn receiver(&self) -> io::Result<&Receiver> {
        match &self.snapshot()?.state {
            State::Receiver(r) => Ok(r),
            _ => Err(bad("not receiver")),
        }
    }
    pub fn sender(&self) -> io::Result<&Sender> {
        match &self.snapshot()?.state {
            State::Sender(s) => Ok(s),
            _ => Err(bad("not sender")),
        }
    }
    fn transaction<T>(
        &mut self,
        op: impl FnOnce(&mut State) -> io::Result<T>,
        hook: &mut impl FnMut(&str) -> io::Result<()>,
    ) -> io::Result<T> {
        let mut next = self.snapshot()?.clone();
        let result = op(&mut next.state)?;
        next.validate()?;
        if next != self.snapshot {
            let bytes = self.codec.encode(&next)?;
            if let Err(e) = persist_bytes(
                &self.root,
                "external",
                &mut self.serial,
                &bytes,
                MAX_BYTES,
                hook,
            ) {
                self.poisoned = true;
                return Err(e);
            }
            self.snapshot = next;
        }
        if let Err(e) = hook("reply") {
            self.poisoned = true;
            return Err(e);
        }
        Ok(result)
    }
    pub fn apply(
        &mut self,
        e: &Effect,
        hook: &mut impl FnMut(&str) -> io::Result<()>,
    ) -> io::Result<Delivery> {
        self.transaction(
            |s| match s {
                State::Receiver(r) => r.apply(e),
                _ => Err(bad("not receiver")),
            },
            hook,
        )
    }
    pub fn prepare(&mut self, e: Effect) -> io::Result<()> {
        self.transaction(
            |s| match s {
                State::Sender(s) => s.prepare(e, TaskKind::Original),
                _ => Err(bad("not sender")),
            },
            &mut |_| Ok(()),
        )
    }
    pub fn begin_send(
        &mut self,
        id: &str,
        hook: &mut impl FnMut(&str) -> io::Result<()>,
    ) -> io::Result<Option<Effect>> {
        self.transaction(
            |s| match s {
                State::Sender(s) => s.begin(id),
                _ => Err(bad("not sender")),
            },
            hook,
        )
    }
    pub fn complete_send(&mut self, id: &str, delivery: Option<Delivery>) -> io::Result<()> {
        self.transaction(
            |s| match s {
                State::Sender(s) => s.complete(id, delivery),
                _ => Err(bad("not sender")),
            },
            &mut |_| Ok(()),
        )
    }
    pub fn change_validity(&mut self, valid: bool) -> io::Result<Option<String>> {
        self.transaction(
            |s| match s {
                State::Sender(s) => s.change_validity(valid),
                _ => Err(bad("not sender")),
            },
            &mut |_| Ok(()),
        )
    }
    pub fn resolve(&mut self, id: &str, e: Evidence) -> io::Result<Resolution> {
        self.transaction(
            |s| match s {
                State::Sender(s) => s.resolve(id, e),
                _ => Err(bad("not sender")),
            },
            &mut |_| Ok(()),
        )
    }
}
