//! v1.2F operation semantics under a single commit authority.
//! Separate from frozen v1.2B: DELTA advances object version; predicate effects
//! advance predicate version. This is an in-memory concurrency gate, not a
//! durable receiver, natural-language verifier, or product-memory integration.
use crate::kernel::{effect_digest, Effect, LedgerEntry, Receipt};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet, VecDeque},
    sync::{Arc, Mutex},
};

pub const MAX_OPERATIONS: usize = 100_000;
pub const QUEUE_CAPS: [usize; 3] = [32, 64, 64];

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub enum Mutation {
    Delta(i64),
    AOff,
    BOff,
    NoMatch,
}
impl Mutation {
    pub fn digest(&self) -> String {
        let (kind, target, delta) = match self {
            Self::Delta(n) => ("DELTA", "value", *n),
            Self::AOff => ("A_OFF", "a", 0),
            Self::BOff => ("B_OFF", "b", 0),
            Self::NoMatch => ("NO_MATCH", "matching_count", 0),
        };
        effect_digest(&Effect {
            kind: kind.into(),
            target: target.into(),
            delta,
        })
    }
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Snapshot {
    pub value: i64,
    pub a: bool,
    pub b: bool,
    pub object_version: u64,
    pub predicate_version: u64,
    pub matching_count: u64,
    pub parent_active: bool,
    pub parent_generation: u64,
    pub policy_epoch: u64,
    pub capability_epoch: u64,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Proposal {
    pub operation_id: u64,
    pub nonce: u64,
    pub capability_id: u64,
    pub mutation: Mutation,
    pub expected: Snapshot,
    pub reviewed_effect_digest: String,
}
impl Proposal {
    pub fn new(
        operation_id: u64,
        nonce: u64,
        mutation: Mutation,
        expected: Snapshot,
        capability_id: u64,
    ) -> Self {
        let reviewed_effect_digest = mutation.digest();
        Self {
            operation_id,
            nonce,
            mutation,
            expected,
            capability_id,
            reviewed_effect_digest,
        }
    }
}
/// Trusted host controls are a separate API from neural/user proposals.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Control {
    Cancel,
    ReuseParent,
    Revoke,
    Policy,
    ObjectChange,
    InsertMatch,
    ResetAB,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Request {
    Snapshot,
    Propose(Proposal),
    Control(Control),
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Response {
    pub sequence: u64,
    pub outcome: String,
    pub value: i64,
    pub receipt: Option<Receipt>,
    pub snapshot: Option<Snapshot>,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct State {
    current: Snapshot,
    sequence: u64,
    capabilities: BTreeSet<u64>,
    ledger: BTreeMap<u64, LedgerEntry>,
    nonces: BTreeMap<u64, u64>,
}
impl Default for State {
    fn default() -> Self {
        Self {
            current: Snapshot {
                value: 0,
                a: true,
                b: true,
                object_version: 0,
                predicate_version: 0,
                matching_count: 0,
                parent_active: true,
                parent_generation: 1,
                policy_epoch: 1,
                capability_epoch: 1,
            },
            sequence: 0,
            capabilities: BTreeSet::from([1]),
            ledger: BTreeMap::new(),
            nonces: BTreeMap::new(),
        }
    }
}
impl State {
    pub fn snapshot(&self) -> Snapshot {
        self.current.clone()
    }
    pub fn ledger(&self) -> &BTreeMap<u64, LedgerEntry> {
        &self.ledger
    }
    pub fn nonces(&self) -> &BTreeMap<u64, u64> {
        &self.nonces
    }
    pub fn digest(&self) -> String {
        // Explicit C protocol identity, separate from the frozen B digest.
        let mut h = Sha256::new();
        h.update(b"replica-commit-concurrency-F-checked-v1\0");
        h.update(serde_json::to_vec(self).expect("typed integer state serializes"));
        hex::encode(h.finalize())
    }
    /// Serial transition used both by the authority and replay verification.
    pub fn execute(&mut self, request: &Request) -> Result<Response, Error> {
        let sequence = self
            .sequence
            .checked_add(1)
            .ok_or(Error::SequenceOverflow)?;
        let mut response = Response {
            sequence,
            outcome: String::new(),
            value: self.current.value,
            receipt: None,
            snapshot: None,
        };
        response.outcome = match request {
            Request::Snapshot => {
                response.snapshot = Some(self.snapshot());
                "SNAPSHOT".into()
            }
            Request::Control(control) => self.control(control).into(),
            Request::Propose(proposal) => self.commit(proposal, &mut response).into(),
        };
        self.sequence = sequence;
        if response.receipt.is_none() {
            response.value = self.current.value;
        }
        Ok(response)
    }
    fn control(&mut self, control: &Control) -> &'static str {
        let s = &mut self.current;
        match control {
            Control::Cancel => {
                s.parent_active = false;
                "OK_CANCEL"
            }
            Control::ReuseParent => match s.parent_generation.checked_add(1) {
                Some(n) => {
                    s.parent_generation = n;
                    s.parent_active = true;
                    "OK_REUSE"
                }
                None => "REJECT_OVERFLOW",
            },
            Control::Revoke => match s.capability_epoch.checked_add(1) {
                Some(n) => {
                    s.capability_epoch = n;
                    "OK_REVOKE"
                }
                None => "REJECT_OVERFLOW",
            },
            Control::Policy => match s.policy_epoch.checked_add(1) {
                Some(n) => {
                    s.policy_epoch = n;
                    "OK_POLICY"
                }
                None => "REJECT_OVERFLOW",
            },
            Control::ObjectChange => match s.object_version.checked_add(1) {
                Some(n) => {
                    s.object_version = n;
                    "OK_OBJ"
                }
                None => "REJECT_OVERFLOW",
            },
            Control::InsertMatch => match (
                s.matching_count.checked_add(1),
                s.predicate_version.checked_add(1),
            ) {
                (Some(count), Some(version)) => {
                    s.matching_count = count;
                    s.predicate_version = version;
                    "OK_INSERT"
                }
                _ => "REJECT_OVERFLOW",
            },
            Control::ResetAB => match s.predicate_version.checked_add(1) {
                Some(n) => {
                    s.a = true;
                    s.b = true;
                    s.predicate_version = n;
                    "OK_RESET_AB"
                }
                None => "REJECT_OVERFLOW",
            },
        }
    }
    fn commit(&mut self, p: &Proposal, response: &mut Response) -> &'static str {
        let digest = p.mutation.digest();
        if let Some(entry) = self.ledger.get(&p.operation_id) {
            if entry.effect_digest != digest {
                return "REJECT_OP_MISMATCH";
            }
            response.value = entry.value_after;
            response.receipt = Some(receipt(entry));
            return "REPLAY";
        }
        if self.nonces.contains_key(&p.nonce) {
            return "REJECT_NONCE";
        }
        let s = &self.current;
        if !s.parent_active || p.expected.parent_generation != s.parent_generation {
            return "REJECT_PARENT";
        }
        if p.expected.policy_epoch != s.policy_epoch {
            return "REJECT_POLICY";
        }
        if !self.capabilities.contains(&p.capability_id)
            || p.expected.capability_epoch != s.capability_epoch
        {
            return "REJECT_CAP";
        }
        if p.reviewed_effect_digest != digest {
            return "REJECT_EFFECT_DIGEST";
        }
        if self.ledger.len() >= MAX_OPERATIONS {
            return "REJECT_FULL";
        }
        let mut next = s.clone();
        match p.mutation {
            Mutation::Delta(delta) => {
                if p.expected.object_version != s.object_version {
                    return "REJECT_OBJ";
                }
                match (s.value.checked_add(delta), s.object_version.checked_add(1)) {
                    (Some(value), Some(version)) => {
                        next.value = value;
                        next.object_version = version;
                    }
                    _ => return "REJECT_OVERFLOW",
                }
            }
            Mutation::AOff | Mutation::BOff => {
                if p.expected.predicate_version != s.predicate_version {
                    return "REJECT_PRED";
                }
                let turn_off = match p.mutation {
                    Mutation::AOff => s.b,
                    _ => s.a,
                };
                if turn_off {
                    let Some(version) = s.predicate_version.checked_add(1) else {
                        return "REJECT_OVERFLOW";
                    };
                    next.predicate_version = version;
                    match p.mutation {
                        Mutation::AOff => next.a = false,
                        _ => next.b = false,
                    }
                }
            }
            Mutation::NoMatch => {
                if p.expected.predicate_version != s.predicate_version {
                    return "REJECT_PRED";
                }
                if s.matching_count != 0 {
                    return "REJECT_NOT_EMPTY";
                }
            }
        }
        let entry = LedgerEntry {
            effect_digest: digest,
            operation_id: p.operation_id,
            value_after: next.value,
        };
        response.value = next.value;
        response.receipt = Some(receipt(&entry));
        // All checks and checked arithmetic precede this single-authority mutation.
        self.current = next;
        self.nonces.insert(p.nonce, p.operation_id);
        self.ledger.insert(p.operation_id, entry);
        "COMMIT"
    }
}
fn receipt(e: &LedgerEntry) -> Receipt {
    Receipt {
        effect_digest: e.effect_digest.clone(),
        operation_id: e.operation_id,
        outcome: "COMMIT".into(),
        value_after: e.value_after,
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Tier {
    Critical = 0,
    High = 1,
    Low = 2,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Error {
    Backpressure(Tier),
    Poisoned,
    SequenceOverflow,
}
#[derive(Clone, Debug)]
pub struct Ticket {
    completed: Arc<Mutex<Option<Result<Response, Error>>>>,
    pub coalesced: bool,
    pub tier: Tier,
}
impl Ticket {
    pub fn poll(&self) -> Result<Option<Result<Response, Error>>, Error> {
        let mut result = self.completed.lock().map_err(|_| Error::Poisoned)?.clone();
        // A coalesced retry obtains the same completed receipt/linearization,
        // not a second COMMIT. Rejections are not relabeled as replay.
        if self.coalesced {
            if let Some(Ok(response)) = &mut result {
                if response.outcome == "COMMIT" {
                    response.outcome = "REPLAY".into();
                }
            }
        }
        Ok(result)
    }
}
struct Pending {
    request: Request,
    completed: Arc<Mutex<Option<Result<Response, Error>>>>,
}
struct Inner {
    state: State,
    queues: [VecDeque<Pending>; 3],
    cursor: usize,
}
/// Reads and submissions may come from many threads; only service_one mutates
/// canonical state under the authority mutex. Pending tickets are not durable receipts.
pub struct Authority {
    inner: Mutex<Inner>,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Execution {
    pub request: Request,
    pub response: Response,
}
impl Default for Authority {
    fn default() -> Self {
        Self {
            inner: Mutex::new(Inner {
                state: State::default(),
                queues: std::array::from_fn(|_| VecDeque::new()),
                cursor: 0,
            }),
        }
    }
}
impl Authority {
    pub fn snapshot(&self) -> Result<Snapshot, Error> {
        Ok(self
            .inner
            .lock()
            .map_err(|_| Error::Poisoned)?
            .state
            .snapshot())
    }
    pub fn state(&self) -> Result<State, Error> {
        Ok(self
            .inner
            .lock()
            .map_err(|_| Error::Poisoned)?
            .state
            .clone())
    }
    pub fn queued(&self) -> Result<[usize; 3], Error> {
        Ok(self
            .inner
            .lock()
            .map_err(|_| Error::Poisoned)?
            .queues
            .each_ref()
            .map(VecDeque::len))
    }
    pub fn submit(&self, proposal: Proposal, requested: Tier) -> Result<Ticket, Error> {
        self.enqueue(Request::Propose(proposal), requested.min(Tier::High))
    }
    pub fn submit_control(&self, control: Control) -> Result<Ticket, Error> {
        self.enqueue(Request::Control(control), Tier::Critical)
    }
    pub fn submit_snapshot(&self) -> Result<Ticket, Error> {
        self.enqueue(Request::Snapshot, Tier::Low)
    }
    fn enqueue(&self, request: Request, tier: Tier) -> Result<Ticket, Error> {
        let mut inner = self.inner.lock().map_err(|_| Error::Poisoned)?;
        let queue = &mut inner.queues[tier as usize];
        // Only byte-equivalent typed mutation requests coalesce. Controls such
        // as two INSERT_MATCH calls are distinct effects and never coalesce.
        if matches!(request, Request::Propose(_)) {
            if let Some(pending) = queue.iter().find(|p| p.request == request) {
                return Ok(Ticket {
                    completed: pending.completed.clone(),
                    coalesced: true,
                    tier,
                });
            }
        }
        if queue.len() == QUEUE_CAPS[tier as usize] {
            return Err(Error::Backpressure(tier));
        }
        let completed = Arc::new(Mutex::new(None));
        queue.push_back(Pending {
            request,
            completed: completed.clone(),
        });
        Ok(Ticket {
            completed,
            coalesced: false,
            tier,
        })
    }
    pub fn service_one(&self) -> Result<Option<Execution>, Error> {
        let mut inner = self.inner.lock().map_err(|_| Error::Poisoned)?;
        // 65/25/10 weighted cycle. Empty shares are immediately borrowed; each
        // nonempty tier keeps its guaranteed slots under sustained load.
        let mut pending = None;
        for _ in 0..100 {
            let tier = match inner.cursor {
                0..=64 => 0,
                65..=89 => 1,
                _ => 2,
            };
            inner.cursor = (inner.cursor + 1) % 100;
            if let Some(p) = inner.queues[tier].pop_front() {
                pending = Some(p);
                break;
            }
        }
        let Some(pending) = pending else {
            return Ok(None);
        };
        let result = inner.state.execute(&pending.request);
        *pending.completed.lock().map_err(|_| Error::Poisoned)? = Some(result.clone());
        result.map(|response| {
            Some(Execution {
                request: pending.request,
                response,
            })
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn overflow_rejects_without_partial_canonical_effect() {
        for case in 0..11 {
            let mut s = State::default();
            let request = match case {
                0 => {
                    s.current.value = i64::MAX;
                    Request::Propose(Proposal::new(1, 1, Mutation::Delta(1), s.snapshot(), 1))
                }
                1 => {
                    s.current.object_version = u64::MAX;
                    Request::Propose(Proposal::new(1, 1, Mutation::Delta(1), s.snapshot(), 1))
                }
                2 => {
                    s.current.parent_generation = u64::MAX;
                    Request::Control(Control::ReuseParent)
                }
                3 => {
                    s.current.capability_epoch = u64::MAX;
                    Request::Control(Control::Revoke)
                }
                4 => {
                    s.current.policy_epoch = u64::MAX;
                    Request::Control(Control::Policy)
                }
                5 => {
                    s.current.predicate_version = u64::MAX;
                    Request::Control(Control::InsertMatch)
                }
                6 => {
                    s.current.matching_count = u64::MAX;
                    Request::Control(Control::InsertMatch)
                }
                7 => {
                    s.current.predicate_version = u64::MAX;
                    Request::Propose(Proposal::new(1, 1, Mutation::AOff, s.snapshot(), 1))
                }
                8 => {
                    s.current.value = i64::MIN;
                    Request::Propose(Proposal::new(1, 1, Mutation::Delta(-1), s.snapshot(), 1))
                }
                9 => {
                    s.current.object_version = u64::MAX;
                    Request::Control(Control::ObjectChange)
                }
                _ => {
                    s.current.a = false;
                    s.current.predicate_version = u64::MAX;
                    Request::Control(Control::ResetAB)
                }
            };
            let before = s.clone();
            let r = s.execute(&request).unwrap();
            assert_eq!(r.outcome, "REJECT_OVERFLOW");
            assert!(r.receipt.is_none());
            assert_eq!(s.current, before.current);
            assert_eq!(s.ledger, before.ledger);
            assert_eq!(s.nonces, before.nonces);
        }
        let mut s = State::default();
        s.sequence = u64::MAX;
        let before = s.clone();
        assert_eq!(
            s.execute(&Request::Control(Control::Cancel)),
            Err(Error::SequenceOverflow)
        );
        assert_eq!(s, before);
    }
}
