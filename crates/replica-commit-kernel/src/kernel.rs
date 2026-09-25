use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Effect {
    pub kind: String,
    pub target: String,
    pub delta: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Envelope {
    pub attempt_id: u64,
    pub capability_epoch: u64,
    pub capability_id: u64,
    pub effect: Effect,
    pub nonce: u64,
    pub object_version: u64,
    pub operation_id: u64,
    pub parent_generation: u64,
    pub policy_epoch: u64,
    pub predicate_version: u64,
    pub reviewed_effect_digest: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct LedgerEntry {
    pub effect_digest: String,
    pub operation_id: u64,
    pub value_after: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Receipt {
    pub effect_digest: String,
    pub operation_id: u64,
    pub outcome: String,
    pub value_after: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct JournalEntry {
    pub outcome: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub receipt: Option<Receipt>,
    pub value_after: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct State {
    pub authorized_capability_ids: Vec<u64>,
    pub capability_epoch: u64,
    pub object_version: u64,
    pub operation_ledger: Vec<LedgerEntry>,
    pub parent_active: bool,
    pub parent_generation: u64,
    pub policy_epoch: u64,
    pub predicate_version: u64,
    pub used_nonces: Vec<u64>,
    pub value: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Event {
    #[serde(rename = "type")]
    pub kind: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub env: Option<Envelope>,
}

impl Event {
    fn valid(&self) -> bool {
        match self.kind.as_str() {
            "ATTEMPT" => self.env.is_some(),
            "CANCEL" | "PARENT_REUSE" | "REVOKE" | "POLICY_UPDATE" | "OBJECT_CHANGE"
            | "PREDICATE_CHANGE" | "CRASH" | "RESTART" | "REPLY_LOST" => self.env.is_none(),
            _ => false,
        }
    }
}

impl<'de> Deserialize<'de> for Event {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct Wire {
            #[serde(rename = "type")]
            kind: String,
            // Missing is allowed for control events; explicit null is never an envelope.
            #[serde(default, deserialize_with = "present_envelope")]
            env: Option<Envelope>,
        }
        fn present_envelope<'de, D: serde::Deserializer<'de>>(
            d: D,
        ) -> Result<Option<Envelope>, D::Error> {
            Envelope::deserialize(d).map(Some)
        }
        let wire = Wire::deserialize(deserializer)?;
        let event = Self {
            kind: wire.kind,
            env: wire.env,
        };
        if !event.valid() {
            return Err(serde::de::Error::custom(
                "invalid event type/envelope combination",
            ));
        }
        Ok(event)
    }
}

#[derive(Debug, Clone)]
pub struct Kernel {
    state: State,
    ledger: BTreeMap<u64, LedgerEntry>,
    nonces: BTreeSet<u64>,
}

impl Kernel {
    pub fn new(mut state: State) -> Self {
        state.authorized_capability_ids.sort_unstable();
        state.operation_ledger.sort_by_key(|x| x.operation_id);
        state.used_nonces.sort_unstable();
        let ledger = state
            .operation_ledger
            .iter()
            .cloned()
            .map(|x| (x.operation_id, x))
            .collect();
        let nonces = state.used_nonces.iter().copied().collect();
        Self {
            state,
            ledger,
            nonces,
        }
    }

    pub fn state(&self) -> &State {
        &self.state
    }

    pub fn into_state(mut self) -> State {
        self.refresh_materialized_indexes();
        self.state
    }

    pub fn apply(&mut self, event: &Event) -> JournalEntry {
        if !event.valid() {
            return JournalEntry {
                outcome: "REJECT_EVENT".into(),
                receipt: None,
                value_after: self.state.value,
            };
        }
        let mut outcome = "OK".to_string();
        let mut receipt = None;

        match event.kind.as_str() {
            "CANCEL" => self.state.parent_active = false,
            "PARENT_REUSE" => match self.state.parent_generation.checked_add(1) {
                Some(v) => {
                    self.state.parent_generation = v;
                    self.state.parent_active = true;
                }
                None => outcome = "REJECT_OVERFLOW".to_string(),
            },
            "REVOKE" => match self.state.capability_epoch.checked_add(1) {
                Some(v) => self.state.capability_epoch = v,
                None => outcome = "REJECT_OVERFLOW".to_string(),
            },
            "POLICY_UPDATE" => match self.state.policy_epoch.checked_add(1) {
                Some(v) => self.state.policy_epoch = v,
                None => outcome = "REJECT_OVERFLOW".to_string(),
            },
            "OBJECT_CHANGE" => match self.state.object_version.checked_add(1) {
                Some(v) => self.state.object_version = v,
                None => outcome = "REJECT_OVERFLOW".to_string(),
            },
            "PREDICATE_CHANGE" => match self.state.predicate_version.checked_add(1) {
                Some(v) => self.state.predicate_version = v,
                None => outcome = "REJECT_OVERFLOW".to_string(),
            },
            "CRASH" | "RESTART" | "REPLY_LOST" => {}
            "ATTEMPT" => {
                let env = event.env.as_ref().expect("ATTEMPT requires env");
                let actual_digest = effect_digest(&env.effect);

                if let Some(existing) = self.ledger.get(&env.operation_id).cloned() {
                    if existing.effect_digest == actual_digest {
                        outcome = "REPLAY".to_string();
                        receipt = Some(commit_receipt(&existing));
                    } else {
                        outcome = "REJECT_OP_MISMATCH".to_string();
                    }
                } else if !self.state.parent_active
                    || env.parent_generation != self.state.parent_generation
                {
                    outcome = "REJECT_PARENT".to_string();
                } else if env.policy_epoch != self.state.policy_epoch {
                    outcome = "REJECT_POLICY".to_string();
                } else if !self
                    .state
                    .authorized_capability_ids
                    .binary_search(&env.capability_id)
                    .is_ok()
                    || env.capability_epoch != self.state.capability_epoch
                {
                    outcome = "REJECT_CAPABILITY".to_string();
                } else if env.reviewed_effect_digest != actual_digest {
                    outcome = "REJECT_EFFECT_DIGEST".to_string();
                } else if env.object_version != self.state.object_version {
                    outcome = "REJECT_OBJECT_DEP".to_string();
                } else if env.predicate_version != self.state.predicate_version {
                    outcome = "REJECT_PREDICATE_DEP".to_string();
                } else if self.nonces.contains(&env.nonce) {
                    outcome = "REJECT_NONCE".to_string();
                } else if env.effect.kind != "add" || env.effect.target != "value" {
                    outcome = "REJECT_EFFECT_DIGEST".to_string();
                } else if let Some(next_value) = self.state.value.checked_add(env.effect.delta) {
                    self.state.value = next_value;
                    self.nonces.insert(env.nonce);
                    let entry = LedgerEntry {
                        effect_digest: actual_digest,
                        operation_id: env.operation_id,
                        value_after: next_value,
                    };
                    self.ledger.insert(env.operation_id, entry.clone());
                    outcome = "COMMIT".to_string();
                    receipt = Some(commit_receipt(&entry));
                    self.refresh_materialized_indexes();
                } else {
                    outcome = "REJECT_OVERFLOW".to_string();
                }
            }
            other => panic!("unsupported event type: {other}"),
        }

        JournalEntry {
            outcome,
            receipt,
            value_after: self.state.value,
        }
    }

    fn refresh_materialized_indexes(&mut self) {
        self.state.operation_ledger = self.ledger.values().cloned().collect();
        self.state.used_nonces = self.nonces.iter().copied().collect();
        self.state.authorized_capability_ids.sort_unstable();
    }
}

pub fn effect_digest(effect: &Effect) -> String {
    // v1.2B profile fixes the exact byte form to:
    // {"delta":<i64>,"kind":"add","target":"value"}
    let canonical = format!(
        "{{\"delta\":{},\"kind\":\"{}\",\"target\":\"{}\"}}",
        effect.delta, effect.kind, effect.target
    );
    hex::encode(Sha256::digest(canonical.as_bytes()))
}

pub fn state_sha256(state: &State) -> String {
    // Serialize a dedicated lexical-key-order view, independent of Rust struct field order.
    let ledger = serde_json::to_value(&state.operation_ledger).expect("ledger serializes");
    let nonces = serde_json::to_value(&state.used_nonces).expect("nonces serialize");
    let caps = serde_json::to_value(&state.authorized_capability_ids).expect("caps serialize");
    let value = serde_json::json!({
        "authorized_capability_ids": caps,
        "capability_epoch": state.capability_epoch,
        "object_version": state.object_version,
        "operation_ledger": ledger,
        "parent_active": state.parent_active,
        "parent_generation": state.parent_generation,
        "policy_epoch": state.policy_epoch,
        "predicate_version": state.predicate_version,
        "used_nonces": nonces,
        "value": state.value,
    });
    let raw = serde_json::to_vec(&value).expect("state serializes");
    hex::encode(Sha256::digest(raw))
}

fn commit_receipt(entry: &LedgerEntry) -> Receipt {
    Receipt {
        effect_digest: entry.effect_digest.clone(),
        operation_id: entry.operation_id,
        outcome: "COMMIT".to_string(),
        value_after: entry.value_after,
    }
}
