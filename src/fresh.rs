//! Training-only fresh corpus/policy. No renderer or oracle enters product inference.
use super::*;
use clap::Subcommand;
use replica_v3::{
    binary,
    event::GenerationLimits,
    model::ModelRequest,
    neural::transformer::Config,
    retrieval::{Evidence, EvidenceBundle},
};
use serde::{Deserialize, Serialize};
use std::{collections::{HashMap, BTreeSet}, io::Write, path::PathBuf};
const REVISION: &str = "joint-educational-v1";
pub(super) fn is_binding_expansion(p:&Plan)->bool {
    identifiable::binding::is_expansion(p) || identifiable::binding::citation::is(p)
}
const SYSTEM: &str = "제공된 기록과 질문만으로 답하세요. 요구한 원문 또는 값을 쓰고 근거를 [event:번호]로 인용하세요. 근거가 없거나 모호하면 구별해서 유보하세요. 순서만으로 원인을 단정하지 마세요.";
#[path = "identifiable.rs"]
mod identifiable;
#[path = "metal_runtime.rs"]
mod metal_runtime;
#[path = "muon.rs"]
mod muon;
#[derive(Subcommand)]
pub enum Command {
    /// Fixed parent/data Metal F32 optimizer comparison.
    Muon { #[command(subcommand)] action: muon::Action },
    /// Bounded, explicitly registered backend validation; no quality learning.
    MetalRuntime { #[command(subcommand)] action: metal_runtime::Action },
    /// Same sealed citation data/tape, TOKEN reference and answer-mean CE fork.
    AnswerMeanPrepare { #[arg(long)] previous: PathBuf, #[arg(long)] output: PathBuf },
    CitationContinuePrepare { #[arg(long)] previous: PathBuf, #[arg(long)] output: PathBuf },
    /// One unchanged citation cycle from the complete ANSWER7424 endpoint.
    CitationFidelityPrepare { #[arg(long)] previous: PathBuf, #[arg(long)] output: PathBuf, #[arg(long)] parent_review: PathBuf },
    /// Same ANSWER10496 and half cycle, with constant LR3e-5 and inherited Adam.
    CitationPrecisionPrepare { #[arg(long)] previous: PathBuf, #[arg(long)] output: PathBuf, #[arg(long)] parent_review: PathBuf },
    /// Preserve accepted value/citation while learning the existing balanced QA pool.
    RetainedQaPrepare { #[arg(long)] previous: PathBuf, #[arg(long)] balanced: PathBuf, #[arg(long)] old_qa: PathBuf, #[arg(long)] output: PathBuf, #[arg(long)] parent_review: PathBuf },
    RetainedQaParent { #[arg(long)] study: PathBuf, #[arg(long,value_parser=["value","citation","balanced","transfer"])] panel: String },
    RetainedQaReport { #[arg(long)] study: PathBuf },
    /// Versioned read-only correction of historical QA metrics; never rewrites receipts.
    RetainedQaRecount { #[arg(long)] study: PathBuf, #[arg(long)] output: PathBuf },
    QaBridgeProbePrepare { #[arg(long)] previous: PathBuf, #[arg(long)] balanced: PathBuf, #[arg(long)] output: PathBuf, #[arg(long)] review_a1: PathBuf },
    QaBridgeProbe { #[arg(long)] study: PathBuf, #[arg(long,value_parser=["value","citation","S1Q0","S0Q1","S1Q1"])] panel: String },
    QaBridgeProbeReport { #[arg(long)] study: PathBuf },
    QaBridgePrepare {
        #[arg(long,required_unless_present="continue_from",conflicts_with="continue_from")] probe: Option<PathBuf>,
        #[arg(long,required_unless_present="continue_from",conflicts_with="continue_from")] old_qa: Option<PathBuf>,
        #[arg(long)] continue_from: Option<PathBuf>, #[arg(long)] output: PathBuf
    },
    QaBridgeReport { #[arg(long)] study: PathBuf },
    WordPrepare { #[arg(long)] previous: PathBuf, #[arg(long)] output: PathBuf },
    AdapterPrepare { #[arg(long)] previous: PathBuf, #[arg(long)] output: PathBuf },
    AdapterParity { #[arg(long)] study: PathBuf },
    Fp4Prepare { #[arg(long)] parent: PathBuf, #[arg(long)] output: PathBuf },
    Fp4Observe { #[arg(long)] study: PathBuf, #[arg(long)] parity: bool },
    WordParent { #[arg(long)] study: PathBuf, #[arg(long,value_parser=["parity","dev","renamed"])] panel: String },
    WordReport { #[arg(long)] study: PathBuf },
    WordReview { #[arg(long)] study: PathBuf },
    /// Complete the adapter's fixed normal32 using only missing RETURNED cases.
    AdapterReviewComplete { #[arg(long)] study: PathBuf, #[arg(long)] output: PathBuf, #[arg(long)] check: bool },
    WordQa { #[arg(long)] study: PathBuf, #[arg(long)] transfer: bool },
    QaBridgeReview { #[arg(long)] study: PathBuf, #[arg(long)] errors: bool, #[arg(long,conflicts_with="errors")] parent: bool },
    QaBridgeQa { #[arg(long)] study: PathBuf, #[arg(long)] transfer: bool, #[arg(long)] diagnostic: bool },
    RetainedQaReview { #[arg(long)] root: PathBuf, #[arg(long,value_parser=["old_qa","balanced"])] panel: String },
    CitationContinueParent { #[arg(long)] study: PathBuf, #[arg(long)] citation: bool },
    AnswerMeanParent { #[arg(long)] study: PathBuf },
    AnswerMeanReport { #[arg(long)] study: PathBuf },
    /// Explicit read-only reproduction of complete endpoints, including quality guard stops.
    AnswerMeanReview { #[arg(long)] root: PathBuf, #[arg(long)] citation: bool },
    /// Prepare the single parent-bound value/citation continuation.
    CitationPrepare { #[arg(long)] parent: PathBuf, #[arg(long)] output: PathBuf, #[arg(long)] reservation: PathBuf, #[arg(long)] used_ids: PathBuf },
    CitationParent { #[arg(long)] study: PathBuf, #[arg(long)] citation: bool },
    CitationReport { #[arg(long)] study: PathBuf },
    CitationParity { #[arg(long)] study: PathBuf, #[arg(long)] citation: bool, #[arg(long)] reviewer: bool },
    CitationSeal { #[arg(long)] study: PathBuf, #[arg(long)] reservation_private: PathBuf },
    CitationConfirm { #[arg(long)] study: PathBuf },
    /// One bounded fork from closed REBIND; repeats its actual tape suffix.
    ConsolidationPrepare { #[arg(long)] parent: PathBuf, #[arg(long)] output: PathBuf },
    /// Once-only first16 parent dev parity, after independent preparation review.
    ConsolidationParent { #[arg(long)] study: PathBuf },
    /// Pure consolidation raw/endpoint recount; never changes historical results.
    ConsolidationReport { #[arg(long)] study: PathBuf },
    /// Prepare the fixed parent-bound old/new binding comparison; no model calls.
    ExpansionPrepare { #[arg(long)] parent: PathBuf, #[arg(long)] output: PathBuf },
    /// Once-only parent parity and new64 generation/teacher observation.
    ExpansionParent { #[arg(long)] study: PathBuf, #[arg(long)] new_pool: bool },
    /// Read-only endpoint recount and paired comparison.
    ExpansionReport { #[arg(long)] study: PathBuf },
    /// Explicit QE512 continuation with unchanged inputs, Adam and constant LR.
    SignalPrepare { #[arg(long)] parent: PathBuf, #[arg(long)] output: PathBuf },
    /// Once-only parent parity before the registered continuation.
    SignalParentParity { #[arg(long)] study: PathBuf },
    /// Pure endpoint and discrimination-signal recount.
    SignalReport { #[arg(long)] study: PathBuf },
    /// Existing BOTH data and tensor; QE/EQ block order is the sole intervention.
    FramingPrepare {
        #[arg(long)]
        parent: PathBuf,
        #[arg(long)]
        output: PathBuf,
    },
    /// Freeze the matched512 decision; no optimizer calls.
    FramingDecide {
        #[arg(long)]
        study: PathBuf,
    },
    FramingCompare {
        #[arg(long)]
        study: PathBuf,
    },
    FramingLegacyParity {
        #[arg(long)]
        study: PathBuf,
    },
    /// Prepare the bounded four-condition selection learnability study.
    BindingPrepare {
        #[arg(long)] parent: PathBuf,
        #[arg(long)] output: PathBuf,
    },
    /// Read-only score/call/checkpoint recount for the selection study.
    BindingReport { #[arg(long)] root: PathBuf },
    /// One registered, fresh-process A endpoint parity (16 calls, no teacher).
    BindingParity { #[arg(long)] root: PathBuf },
    /// Once-only held-out names after the registered K8 development gate.
    BindingProbe { #[arg(long)] root: PathBuf },
    /// One values-only A512 diagnostic; creates a separate study root.
    OrbitSwap { #[arg(long)] parent: PathBuf, #[arg(long)] output: PathBuf },
    /// Same initial tensor and native pool; only assignment exposure differs.
    OrbitPrepare { #[arg(long)] parent: PathBuf, #[arg(long)] output: PathBuf },
    /// Independent preparation of the reserved, never-trained confirmation panel.
    OrbitSeal { #[arg(long)] study: PathBuf },
    /// Read-only paired endpoint recount, including orbit-unit uncertainty.
    OrbitCompare { #[arg(long)] study: PathBuf },
    /// Fixed16 endpoint reproduction by the independent reviewer (separate ledger).
    OrbitReviewParity { #[arg(long)] root: PathBuf },
    /// Gate-bound single-candidate independent confirmation, without training.
    OrbitConfirm { #[arg(long)] study: PathBuf },
    /// Read frozen research corpora and executed tapes without model calls.
    IdentifiableAudit {
        #[arg(long, required = true, num_args = 1..)] roots: Vec<PathBuf>,
        #[arg(long, num_args = 1..)] diagnostics: Vec<PathBuf>,
        #[arg(long)] output: PathBuf,
    },
    /// Prepare a separate balanced study; never resumes an existing model.
    IdentifiablePrepare {
        #[arg(long)] parent: PathBuf,
        #[arg(long)] output: PathBuf,
    },
    /// Explicit new research from a completed paired endpoint; never edits its terminal.
    PairedContinue {
        #[arg(long)] parent: PathBuf,
        #[arg(long)] output: PathBuf,
        #[arg(long)] frozen_executable: PathBuf,
        /// Explicit exposure-only research from closed COVER1280; no old resume.
        #[arg(long, conflicts_with = "selector_phrase_exposure")] fixed_cover_exposure: bool,
        /// Same COVER parent/tape budget, adding only owned selector question wording.
        #[arg(long)] selector_phrase_exposure: bool,
        #[arg(long)] parity: Option<PathBuf>,
    },
    /// Register the bounded pair-spacing comparison from preserved native train pools.
    PairedPrepare {
        #[arg(long)] parent: PathBuf,
        #[arg(long)] source_data: PathBuf,
        #[arg(long)] output: PathBuf,
        #[arg(long)] parity: Option<PathBuf>,
        /// Explicit single-variable research: existing first-target coefficient 4,
        /// same ADJACENT tape; compare with the retained coefficient-1 execution.
        #[arg(long, conflicts_with_all = ["learning_rate_threefold", "co_batch", "pair_contrast", "sidewise_contrast", "repeat_pair_block", "repeat_two_blocks"])] first_target_four: bool,
        /// Bounded LR-only trial: same P6144/data/Adam, coefficient1, constant9e-5.
        #[arg(long, conflicts_with_all = ["co_batch", "pair_contrast", "sidewise_contrast", "repeat_pair_block", "repeat_two_blocks"])] learning_rate_threefold: bool,
        /// Same batch8 and two-update multiset, with selector pairs in one batch.
        #[arg(long, conflicts_with_all = ["pair_contrast", "sidewise_contrast", "repeat_pair_block", "repeat_two_blocks"])] co_batch: bool,
        /// Same co-batch tape; add fixed paired first-divergence contrast loss.
        #[arg(long, conflicts_with_all = ["sidewise_contrast", "repeat_pair_block", "repeat_two_blocks"])] pair_contrast: bool,
        /// Same contrast tape/coefficient; average separate per-side margin losses.
        #[arg(long, conflicts_with_all = ["repeat_pair_block", "repeat_two_blocks"])] sidewise_contrast: bool,
        /// Same SIDE loss; repeat first C/D/E block while preserving other tasks.
        #[arg(long, conflicts_with = "repeat_two_blocks")] repeat_pair_block: bool,
        /// Same REPLAY conditions; widen recurring C/D/E pool to two blocks.
        #[arg(long)] repeat_two_blocks: bool,
        /// Train-only diagnostic: repeat the first eight seen pairs per C/D/E.
        #[arg(long, conflicts_with_all = ["first_target_four", "learning_rate_threefold", "co_batch", "pair_contrast", "sidewise_contrast", "repeat_pair_block", "repeat_two_blocks"])] fit_seen_pairs: bool,
        /// Same FIT scenes and conditions; alternate owned original/value-swap views.
        #[arg(long, conflicts_with_all = ["first_target_four", "learning_rate_threefold", "co_batch", "pair_contrast", "sidewise_contrast", "repeat_pair_block", "repeat_two_blocks", "fit_seen_pairs"])] alternate_pair_values: bool,
        /// Same VALUE conditions; widen its recurring train scenes fourfold.
        #[arg(long, conflicts_with_all = ["first_target_four", "learning_rate_threefold", "co_batch", "pair_contrast", "sidewise_contrast", "repeat_pair_block", "repeat_two_blocks", "fit_seen_pairs", "alternate_pair_values"])] cover_value_pairs: bool,
        /// Four value combinations; with cover-value-pairs, use32 recurring scenes.
        #[arg(long, conflicts_with_all = ["first_target_four", "learning_rate_threefold", "co_batch", "pair_contrast", "sidewise_contrast", "repeat_pair_block", "repeat_two_blocks", "fit_seen_pairs", "alternate_pair_values"])] diverse_pair_values: bool,
        /// Same COVER4 tape/data; add fixed train-only selected-record attention loss.
        #[arg(long, requires_all = ["cover_value_pairs", "diverse_pair_values"])] ground_selected_record: bool,
    },
    /// Pure recount and the preregistered continuation gate; never starts training.
    PairedReport { #[arg(long)] root: PathBuf },
    /// Register an explicit exposure/wording study from a verified completed parent.
    StudyPrepare {
        #[arg(long)]
        parent: PathBuf,
        #[arg(long)]
        output: PathBuf,
        /// Preserve the completed P wording policy and compare selector exposure.
        #[arg(long)]
        selector: bool,
    },
    /// Fixed same-weight parity and familiar-wording diagnostic, no optimizer calls.
    StudyObserve {
        #[arg(long)]
        root: PathBuf,
    },
    /// Pure read-only parent/C/P comparison; never creates missing observations.
    StudyReport {
        #[arg(long)]
        root: PathBuf,
        /// Read a closed study using its exact retained executable identity.
        /// This grants no permission to resume training or observations.
        #[arg(long)]
        frozen_executable: Option<PathBuf>,
    },
    Prepare {
        #[arg(long)]
        output: PathBuf,
    },
    Run {
        #[arg(long)]
        root: PathBuf,
    },
    Report {
        #[arg(long)]
        root: PathBuf,
    },
    #[cfg(feature = "test-support")]
    Fixture {
        #[arg(long)]
        output: PathBuf,
    },
    #[cfg(feature = "test-support")]
    FixtureFull {
        #[arg(long)]
        root: PathBuf,
    },
    #[cfg(feature = "test-support")]
    FixtureEval {
        #[arg(long)]
        root: PathBuf,
    },
}
fn bad(s: &str) -> Error {
    Error::Invalid(format!("fresh: {s}"))
}
fn write<T: Serialize>(p: &Path, v: &T) -> Result<()> {
    let bytes = binary::to_storage_vec(v)?;
    replica_v3::codec::publish_new(p, |f, owned| {
        f.write_all(&bytes)?;
        if std::fs::read(owned)? != bytes {
            return Err(bad("publication readback"));
        }
        Ok(())
    })
}
fn pending_path(p: &Path) -> PathBuf {
    p.with_extension("pending.r3b")
}
fn publish_confirmed<T: Serialize>(p: &Path, v: &T) -> Result<()> {
    let pending = pending_path(p);
    #[cfg(feature = "test-support")]
    if p.file_name()
        .is_some_and(|n| n.to_string_lossy().ends_with("-finished.r3b"))
        && std::env::var("R3_FRESH_TEST_STOP").as_deref() == Ok("pending-write")
    {
        return Err(std::io::Error::other("injected pending write failure").into());
    }
    write(
        &pending,
        &binary::record!({"schema":2,"destination":p,"content":digest(v)?}),
    )?;
    #[cfg(feature = "test-support")]
    if p.file_name()
        .is_some_and(|n| n.to_string_lossy().ends_with("-finished.r3b"))
        && std::env::var("R3_FRESH_TEST_STOP").as_deref() == Ok("finished-pending-only")
    {
        return Err(std::io::Error::other("injected stop after durable pending").into());
    }
    write(p, v)?;
    // All required file/directory syncs completed before releasing the interlock.
    // A crash can resurrect this unlink and conservatively block, never approve a failed sync.
    std::fs::remove_file(pending)?;
    Ok(())
}
fn read_confirmed<T: serde::de::DeserializeOwned>(p: &Path) -> Result<T> {
    if pending_path(p).exists() {
        return Err(bad(
            "publication pending: optimizer/generation=0; retry blocked",
        ));
    }
    read(p)
}
fn read<T: serde::de::DeserializeOwned>(p: &Path) -> Result<T> {
    Ok(binary::from_slice(&neural::read_bounded(
        p,
        128 * 1024 * 1024,
    )?)?)
}
fn file_hash(p: &Path) -> Result<String> {
    Ok(neural::hash(&neural::read_bounded(p, 512 * 1024 * 1024)?))
}
fn verified_corpus(path: &Path, expected: &str) -> Result<data::native::Corpus> {
    let c = data::native::read(path)?;
    if hex(&c.physical) != expected {
        return Err(bad("owned native corpus differs from frozen bytes"));
    }
    Ok(c)
}
fn verified_metadata(root: &Path, p: &Plan) -> Result<(Vec<Meta>, Vec<Meta>, Vec<Meta>)> {
    let bytes = neural::read_bounded(&root.join("metadata.r3b"), 128 * 1024 * 1024)?;
    if neural::hash(&bytes) != p.metadata {
        return Err(bad("owned metadata differs from plan"));
    }
    Ok(binary::from_slice(&bytes)?)
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Plan {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    framing: Option<neural::Framing>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    identifiable: Option<identifiable::Policy>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    schema: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    fork: Option<Fork>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    paired: Option<PairTape>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    training_values: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    grounding: Option<String>,
    revision: String,
    source: String,
    binary: String,
    corpus: String,
    transfer: String,
    tokenizer: String,
    initial: String,
    initial_weights: String,
    pub(super) config: TrainConfig,
    architecture: Config,
    sampler: String,
    order: Vec<Vec<usize>>,
    train_order: String,
    split_policy: String,
    data_seed: u64,
    model_seed: u64,
    tiny: bool,
    metadata: String,
    evaluation: EvaluationPolicy,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
struct PairTape {
    mode: String,
    first_step: usize,
    block: usize,
    samples: String,
    rows: Vec<[usize; 8]>,
}
impl PairTape {
    fn at(&self, absolute: usize) -> Result<[usize; 8]> {
        absolute.checked_sub(self.first_step).and_then(|i| self.rows.get(i)).copied()
            .ok_or_else(|| bad("finite pair tape cursor outside registered range"))
    }
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
struct Fork {
    study: PathBuf,
    study_hash: String,
    arm: String,
    parent_policy: String,
    parent_state: String,
    parent_adam: String,
    origin_step: usize,
    origin_input: u64,
    origin_target: u64,
    original_corpus: String,
    tokenizer_training_hash: String,
    variants: Option<String>,
    variant_metadata: Option<String>,
    alternate_first: Vec<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    selector: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    selector_metadata: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    flip_first: Vec<Option<bool>>,
    constant_lr: f64,
    target_limit: u64,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
struct EvaluationPolicy {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    teacher_steps: Vec<usize>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    fixture_per_bucket: Option<usize>,
    screen_steps: Vec<usize>,
    primary_steps: Vec<usize>,
    transfer_steps: Vec<usize>,
    train_steps: Vec<usize>,
    generation_limit: usize,
    teacher_limit: usize,
    active_seconds: u64,
    segment_seconds: u64,
    cleanup_seconds: u64,
    primary_min: usize,
    bucket_min: usize,
    transfer_min: usize,
    regression_exact_loss: usize,
    regression_ce_ratio: f64,
}
impl Default for EvaluationPolicy {
    fn default() -> Self {
        Self {
            teacher_steps: vec![],
            fixture_per_bucket: None,
            screen_steps: vec![0, 128, 512, 1024, 2048, 3072, 4096],
            primary_steps: vec![1024, 2048, 4096],
            transfer_steps: vec![2048, 4096],
            train_steps: vec![0, 1024, 2048, 4096],
            generation_limit: 4096,
            teacher_limit: 6000,
            active_seconds: 10800,
            segment_seconds: 900,
            cleanup_seconds: 120,
            primary_min: 487,
            bucket_min: 58,
            transfer_min: 116,
            regression_exact_loss: 12,
            regression_ce_ratio: 1.2,
        }
    }
}
#[derive(Clone, Serialize, Deserialize)]
struct Meta {
    id: String,
    base: String,
    template: String,
    bucket: usize,
    view: usize,
    split: String,
    entities: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    source_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    query_context: Option<String>,
}
fn digest<T: Serialize>(v: &T) -> Result<String> {
    Ok(neural::hash(&binary::to_vec(v)?))
}

// Read-only, training-tool observation. Its autograd graph and gradient map
// never enter response CE or Adam; only Adam's already-computed delta is read.
fn signal_probe_path(root: &Path, step: usize, attempt: usize, kind: &str) -> PathBuf {
    root.join(if attempt == 0 {
        format!("signal-probe-{step:04}-{kind}.r3b")
    } else {
        format!("signal-probe-{step:04}-attempt-{attempt:03}-{kind}.r3b")
    })
}
// One decision for producer, preserved segment and next admission. Missing
// counters are UNKNOWN, never zero; a timeout message alone grants no retry.
fn signal_observation_decision(r: &binary::Value) -> Result<&'static str> {
    if r["schema"] != 2 {
        return Err(bad("unsupported signal observation schema"));
    }
    let f = r["microbatch_forwards"]
        .as_u64()
        .ok_or_else(|| bad("signal entry UNKNOWN"))?;
    let returned = r["returned_forwards"]
        .as_u64()
        .ok_or_else(|| bad("signal return UNKNOWN"))?;
    let b = r["diagnostic_backwards"]
        .as_u64()
        .ok_or_else(|| bad("signal backward UNKNOWN"))?;
    let committed = r["optimizer_committed"]
        .as_bool()
        .ok_or_else(|| bad("signal commit UNKNOWN"))?;
    if returned > f
        || f > 4
        || b > 2
        || r["sample_forwards"] != f * 8
        || r["gradient_enters_optimizer"] != false
    {
        return Err(bad("signal observation counters"));
    }
    let before = &r["before_S_valueNLL_eosNLL"];
    let after = &r["after_S_valueNLL_eosNLL"];
    let finite = |v: &binary::Value| {
        v.as_array().is_some_and(|a| {
            a.len() == 3 && a.iter().all(|x| x.as_f64().is_some_and(f64::is_finite))
        })
    };
    if r["error"].is_null()
        && committed
        && f == 4
        && returned == 4
        && finite(before)
        && finite(after)
        && b == if r["gradient"] == true { 2 } else { 0 }
    {
        return Ok("COMPLETED");
    }
    if committed || f != 0 || returned != 0 || b != 0 || !before.is_null() || !after.is_null() {
        return Ok("BLOCKED_PARTIAL_OBSERVATION");
    }
    let c = &r["control"];
    if r["interruption_is_time"] == true
        && r["preserved_pre_update"] == true
        && c["reason"] == "TIME_BUDGET"
        && c["terminal_reason"] == "TIME_BUDGET"
        && c["observed_conditions"] == binary::record!(["TIME_BUDGET"])
        && c["checkpoint_saved"] == true
        && c["save_error"].is_null()
        && c["trace_error"].is_null()
    {
        return Ok("NOT_INVOKED_TIME_PAUSE");
    }
    Ok("BLOCKED_OBSERVATION")
}
fn signal_checkpoint_state(path: &Path) -> Result<String> {
    let l = checkpoint::load(path, Device::Cpu, true)?;
    digest(&(
        l.model.weights_content_id()?,
        optimizer_hash(&l.optimizer)?,
        &l.manifest.training,
    ))
}
fn signal_preservation_control(c: &binary::Value) -> binary::Value {
    let mut c = c.clone();
    if let binary::Value::Object(fields) = &mut c {
        fields.remove("signal_observation_state");
    }
    c
}
fn signal_segment_resumable(c: &binary::Value) -> bool {
    c["signal_observation_state"].is_null()
        || c["signal_observation_state"] == "NOT_INVOKED_TIME_PAUSE"
}
// Pure read: never fills a missing finish or changes an earlier attempt.
fn signal_probe_next(
    root: &Path,
    p: &Plan,
    step: usize,
    current_step: usize,
) -> Result<Option<(usize, Option<String>)>> {
    let mut prior = None;
    let prefix = format!("signal-probe-{step:04}-");
    let mut inventory = std::fs::read_dir(root)?
        .collect::<std::io::Result<Vec<_>>>()?
        .iter()
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .filter(|n| n.starts_with(&prefix))
        .collect::<BTreeSet<_>>();
    let expected = if inventory.is_empty() {
        None
    } else {
        let tok = ByteBpe::load(&root.join("tokenizer.r3b"))?;
        let (samples, _, gradient) =
            identifiable::binding::signal_probe_cases(p, root, step, &tok)?
                .ok_or_else(|| bad("unexpected signal probe"))?;
        Some((
            digest(
                &samples
                    .iter()
                    .map(|s| (&s.tokens, s.response_start))
                    .collect::<Vec<_>>(),
            )?,
            gradient,
        ))
    };
    for attempt in 0..128 {
        let start = signal_probe_path(root, step, attempt, "started");
        let finish = signal_probe_path(root, step, attempt, "finished");
        if !start.exists() {
            if !inventory.is_empty() {
                return Err(bad("signal attempt gap/orphan"));
            }
            if prior.is_some() && current_step != step - 1 {
                return Err(bad("pending observation cursor advanced"));
            }
            return Ok(Some((attempt, prior)));
        }
        for path in [&start, &finish, &pending_path(&finish)] {
            inventory.remove(
                path.file_name()
                    .unwrap()
                    .to_str()
                    .ok_or_else(|| bad("signal attempt filename"))?,
            );
        }
        let a: binary::Value = read(&start)?;
        let r: binary::Value =
            read_confirmed(&finish).map_err(|_| bad("signal observation unfinished/UNKNOWN"))?;
        let (sample_hash, gradient) = expected
            .as_ref()
            .ok_or_else(|| bad("signal sample obligation"))?;
        if a["policy"] != digest(p)?
            || r["policy"] != a["policy"]
            || a["step"] != step
            || r["step"] != step
            || a["samples"] != sample_hash.as_str()
            || a["gradient"] != *gradient
            || a["denominator"] != 16
            || a["microbatch"] != 8
        {
            return Err(bad("signal observation identity"));
        }
        if a["schema"].is_null() {
            if attempt != 0
                || !r["error"].is_null()
                || current_step < step
                || !inventory.is_empty()
                || r["microbatch_forwards"] != 4
                || r["sample_forwards"] != 32
                || r["diagnostic_backwards"] != if *gradient { 2 } else { 0 }
            {
                return Err(bad("legacy failed/ambiguous signal observation"));
            }
            return Ok(None);
        }
        if a["schema"] != 2
            || r["schema"] != 2
            || a["attempt"] != attempt
            || r["attempt"] != attempt
            || a["previous_finish"] != binary::record!(prior)
            || r["start"] != file_hash(&start)?
            || r["samples"] != a["samples"]
            || r["gradient"] != a["gradient"]
        {
            return Err(bad("signal attempt chain"));
        }
        let pre = Path::new(
            a["pre_checkpoint"]
                .as_str()
                .ok_or_else(|| bad("signal pre checkpoint"))?,
        );
        if a["pre_physical"] != file_hash(pre)? || a["pre_state"] != signal_checkpoint_state(pre)? {
            return Err(bad("signal pre-update binding"));
        }
        let entries = (0..4)
            .filter(|i| signal_probe_path(root, step, attempt, &format!("entry-{i}")).exists())
            .count();
        if r["microbatch_forwards"] != entries {
            return Err(bad("signal durable entry mismatch"));
        }
        for i in 0..entries {
            let path = signal_probe_path(root, step, attempt, &format!("entry-{i}"));
            inventory.remove(
                path.file_name()
                    .unwrap()
                    .to_str()
                    .ok_or_else(|| bad("signal entry filename"))?,
            );
            let entry: binary::Value = read(&path)?;
            if entry["start"] != file_hash(&start)? || entry["ordinal"] != i {
                return Err(bad("signal call entry binding"));
            }
        }
        let decision = signal_observation_decision(&r)?;
        if r["state"] != decision {
            return Err(bad("signal decision mismatch"));
        }
        if decision == "COMPLETED" {
            if current_step < step || !inventory.is_empty() {
                return Err(bad("duplicate/uncommitted completed observation"));
            }
            return Ok(None);
        }
        if decision != "NOT_INVOKED_TIME_PAUSE" {
            return Err(bad(decision));
        }
        let segment = Path::new(
            a["segment"]
                .as_str()
                .ok_or_else(|| bad("signal segment path"))?,
        );
        let c: binary::Value = read(&segment.join("train-control.r3b"))?;
        let post = segment.join("final");
        if signal_preservation_control(&c) != r["control"]
            || c["signal_observation_state"] != decision
            || r["checkpoint"] != file_hash(&post)?
            || signal_checkpoint_state(&post)?
                != a["pre_state"]
                    .as_str()
                    .ok_or_else(|| bad("signal pre state"))?
            || current_step < step - 1
        {
            return Err(bad("signal pause preservation mismatch"));
        }
        prior = Some(file_hash(&finish)?);
    }
    Err(bad("signal attempt limit"))
}
pub(super) struct SignalObservation {
    root: PathBuf,
    step: usize,
    policy: String,
    samples: Vec<Sample>,
    foils: Vec<u32>,
    gradient: bool,
    gs: BTreeMap<String, Tensor>,
    before: Option<[f64; 3]>,
    after: Option<[f64; 3]>,
    forwards: usize,
    backwards: usize,
    dot: f64,
    delta2: f64,
    returned: usize,
    attempt: usize,
    start: binary::Value,
}
impl SignalObservation {
    pub(super) fn due(p: &Plan, step: usize) -> bool {
        identifiable::binding::signal_probe_due(p, step)
    }
    pub(super) fn start(
        p: &Plan,
        root: &Path,
        step: usize,
        tok: &ByteBpe,
        pre: &Path,
    ) -> Result<Option<Self>> {
        let Some((samples, foils, gradient)) =
            identifiable::binding::signal_probe_cases(p, root, step, tok)?
        else {
            return Ok(None);
        };
        let policy = digest(p)?;
        let (attempt, previous) = signal_probe_next(root, p, step, step - 1)?
            .ok_or_else(|| bad("completed signal update cannot repeat"))?;
        let (m, _) = checkpoint::metadata(pre)?;
        if m.training.as_ref().is_none_or(|s| s.step + 1 != step) {
            return Err(bad("signal pre-update step"));
        }
        let start = binary::record!({"schema":2,"policy":policy,"step":step,"attempt":attempt,"previous_finish":previous,
            "pre_checkpoint":pre,"pre_physical":file_hash(pre)?,"pre_state":signal_checkpoint_state(pre)?,"segment":pre.parent(),
            "samples":digest(&samples.iter().map(|s|(&s.tokens,s.response_start)).collect::<Vec<_>>())?,
            "gradient":gradient,"denominator":16,"microbatch":8,"model_calls_started":0});
        write(&signal_probe_path(root, step, attempt, "started"), &start)?;
        Ok(Some(Self {
            root: root.into(),
            step,
            policy,
            samples,
            foils,
            gradient,
            gs: BTreeMap::new(),
            before: None,
            after: None,
            forwards: 0,
            backwards: 0,
            dot: 0.,
            delta2: 0.,
            returned: 0,
            attempt,
            start,
        }))
    }
    fn measure(
        &mut self,
        model: &Transformer,
        control: &mut recovery::RunControl,
        backward: bool,
    ) -> Result<[f64; 3]> {
        let mut result = [0.; 3];
        for offset in [0, 8] {
            let indices = (offset..offset + 8).collect::<Vec<_>>();
            let b = batch(&self.samples, &indices, &model.device)?;
            control.begin_teacher_rows(8)?;
            write(
                &signal_probe_path(
                    &self.root,
                    self.step,
                    self.attempt,
                    &format!("entry-{}", self.forwards),
                ),
                &binary::record!({"start":file_hash(&signal_probe_path(&self.root,self.step,self.attempt,"started"))?,
                    "ordinal":self.forwards,"phase":if self.before.is_none(){"before"}else{"after"},"offset":offset}),
            )?;
            self.forwards += 1;
            let logits = model.forward(&b.input, Some(&b.valid))?;
            self.returned += 1;
            let mut margins = vec![];
            for (j, &i) in indices.iter().enumerate() {
                let s = &self.samples[i];
                let pos = s.response_start - 1;
                let z = logits.narrow(0, j, 1)?.narrow(1, pos, 1)?.flatten_all()?;
                let gold = s.tokens[s.response_start] as usize;
                let foil = self.foils[i] as usize;
                let margin =
                    (z.narrow(0, gold, 1)?.sum_all()? - z.narrow(0, foil, 1)?.sum_all()?)?;
                result[0] += margin.to_scalar::<f32>()? as f64 / 16.;
                for (target, slot) in [(gold, 1usize), (EOS as usize, 2usize)] {
                    let z = logits
                        .narrow(0, j, 1)?
                        .narrow(1, pos + slot - 1, 1)?
                        .flatten_all()?;
                    let logp = candle_nn::ops::log_softmax(&z, 0)?;
                    result[slot] -=
                        logp.narrow(0, target, 1)?.sum_all()?.to_scalar::<f32>()? as f64 / 16.;
                }
                margins.push(margin);
            }
            if backward {
                let s = (Tensor::stack(&margins, 0)?.sum_all()? / 16.)?;
                control.check("signal_before_diagnostic_backward")?;
                self.backwards += 1;
                let gradients = s.backward()?;
                for (name, var) in &model.vars {
                    let g = gradients
                        .get(var)
                        .ok_or_else(|| bad("signal disconnected diagnostic gradient"))?;
                    let next = if let Some(old) = self.gs.get(name) {
                        (old + g)?
                    } else {
                        g.clone()
                    };
                    self.gs.insert(name.clone(), next);
                }
            }
            control.check("signal_microbatch_returned")?;
        }
        if result.iter().any(|x| !x.is_finite()) {
            return Err(bad("signal nonfinite measurement"));
        }
        Ok(result)
    }
    pub(super) fn before(
        &mut self,
        model: &Transformer,
        control: &mut recovery::RunControl,
    ) -> Result<()> {
        #[cfg(feature = "test-support")]
        if model.config.hidden == 32 && model.config.layers == 2 {
            control.fixture_boundary = std::env::var("R3_SIGNAL_CALL_STOP").ok();
        }
        self.before = Some(self.measure(model, control, self.gradient)?);
        Ok(())
    }
    pub(super) fn delta(&mut self, name: &str, old: &Tensor, next: &Tensor) -> Result<()> {
        let delta = (next - old)?;
        self.delta2 += delta.sqr()?.sum_all()?.to_scalar::<f32>()? as f64;
        if self.gradient {
            self.dot += self
                .gs
                .get(name)
                .ok_or_else(|| bad("signal gradient name"))?
                .mul(&delta)?
                .sum_all()?
                .to_scalar::<f32>()? as f64;
        }
        if !self.dot.is_finite() || !self.delta2.is_finite() {
            return Err(bad("signal nonfinite Adam delta"));
        }
        Ok(())
    }
    pub(super) fn after(
        &mut self,
        model: &Transformer,
        control: &mut recovery::RunControl,
    ) -> Result<()> {
        self.after = Some(self.measure(model, control, false)?);
        Ok(())
    }
    pub(super) fn finish(
        &self,
        error: Option<String>,
        step: usize,
        preservation: Option<(&Path, &binary::Value, bool)>,
    ) -> Result<&'static str> {
        let gs2 = self.gs.values().try_fold(0., |a, g| -> Result<f64> {
            Ok(a + g.sqr()?.sum_all()?.to_scalar::<f32>()? as f64)
        })?;
        let difference = self.before.zip(self.after).map(|(a, b)| b[0] - a[0]);
        let mut record = binary::record!({"schema":2,"attempt":self.attempt,
            "start":file_hash(&signal_probe_path(&self.root,self.step,self.attempt,"started"))?,"samples":self.start["samples"],"gradient":self.gradient,
            "optimizer_committed":step>=self.step,"returned_forwards":self.returned,
            "checkpoint":preservation.map(|(path,_,_)|file_hash(path)).transpose()?,
            "control":preservation.map(|(_,c,_)|c),"interruption_is_time":preservation.is_some_and(|(_,_,time)|time),
            "preserved_pre_update":preservation.map(|(path,_,_)|signal_checkpoint_state(path).map(|s|binary::record!(s)==self.start["pre_state"])).transpose()?.unwrap_or(false),
            "policy":self.policy,"step":self.step,"before_S_valueNLL_eosNLL":self.before,"after_S_valueNLL_eosNLL":self.after,
            "gS_norm":if self.gradient{Some(gs2.sqrt())}else{None},"actual_delta_norm":self.delta2.sqrt(),
            "gS_dot_delta":if self.gradient{Some(self.dot)}else{None},"actual_S_change":difference,
            "first_order_residual":if self.gradient{difference.map(|v|v-self.dot)}else{None},
            "microbatch_forwards":self.forwards,"sample_forwards":self.forwards*8,"diagnostic_backwards":self.backwards,
            "gradient_enters_optimizer":false,"error":error});
        let decision = signal_observation_decision(&record)?;
        record["state"] = binary::record!(decision);
        publish_confirmed(
            &signal_probe_path(&self.root, self.step, self.attempt, "finished"),
            &record,
        )?;
        println!(
            "SIGNAL_PROBE step={} before={:?} after={:?} gS_dot_delta={} delta_norm={} forwards={} rows={} backwards={} error={:?}",
            self.step,
            self.before,
            self.after,
            self.dot,
            self.delta2.sqrt(),
            self.forwards,
            self.forwards * 8,
            self.backwards,
            error
        );
        Ok(decision)
    }
    pub(super) fn counts(&self) -> (usize, usize) {
        (self.forwards, self.backwards)
    }
}

fn entity_pool(split: usize, length: usize) -> Vec<String> {
    // Full entity hash partitions a finite namespace. One-digit names are reused
    // within their split, never falsely described as thousands of unique names.
    let capacity = 10usize.pow(length as u32).min(2048);
    (0..capacity)
        .map(|n| format!("장치{n:0length$}"))
        .filter(|s| {
            u64::from_str_radix(&neural::hash(s.as_bytes())[..8], 16).unwrap() % 3 == split as u64
        })
        .collect()
}
fn record(entity: &str, context: &str, value: &str, id: i64, time: i64, status: &str) -> Evidence {
    Evidence {
        event_id: id,
        original_excerpt: format!("{entity}의 {context} 값은 {value}이다."),
        excerpt_truncated: false,
        source: "교육 기록".into(),
        recorded_at: time + 100,
        observed_at: Some(time),
        version_status: status.into(),
        retrieval_reason: "lexical".into(),
        relation_path: vec![],
    }
}
fn parsed_record(e: &Evidence) -> Result<(&str, &str, &str)> {
    let (entity, rest) = e
        .original_excerpt
        .split_once("의 ")
        .ok_or_else(|| bad("record grammar"))?;
    let (context, rest) = rest
        .split_once(" 값은 ")
        .ok_or_else(|| bad("record grammar"))?;
    let value = rest
        .strip_suffix("이다.")
        .filter(|v| !v.is_empty())
        .ok_or_else(|| bad("record grammar"))?;
    Ok((entity, context, value))
}
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
enum Intent {
    Full,
    Current,
    Previous,
    Restored,
    Cause,
}
fn phrases(i: Intent) -> [&'static str; 2] {
    match i {
        Intent::Full => [
            "기록에 적힌 문장을 빠짐없이 쓰고 인용하라.",
            "기록 원문을 온전히 옮기고 근거 번호를 덧붙여라.",
        ],
        Intent::Current => [
            "유효한 값을 답하고 기록 번호를 인용하라.",
            "지금의 값에 해당하는 근거를 함께 제시하라.",
        ],
        Intent::Previous => [
            "정정 이전의 값을 답하고 기록 번호를 인용하라.",
            "수정하기 이전 값과 그 근거 번호를 제시하라.",
        ],
        Intent::Restored => [
            "복원 이후 유효한 값을 기록 번호와 함께 답하라.",
            "되살린 기록의 현재 값과 근거 번호를 제시하라.",
        ],
        Intent::Cause => [
            "시간상 앞섰다는 이유만으로 원인을 결론 내릴 수 있는가?",
            "시간의 선후만으로 인과관계를 확정할 수 있는가?",
        ],
    }
}
fn question_intent(input: &str) -> Result<(&str, &str, Intent)> {
    let mut words = input.splitn(3, ' ');
    let e = words.next().ok_or_else(|| bad("question entity"))?;
    let c = words.next().ok_or_else(|| bad("question context"))?;
    let t = words.next().ok_or_else(|| bad("question task"))?;
    for (i, old) in [
        (
            Intent::Full,
            [
                "기록 원문 전체를 인용과 함께 써라.",
                "원문 전체와 인용을 답하라.",
                "해당 기록의 문장 전체를 근거 번호와 함께 알려줘.",
            ],
        ),
        (
            Intent::Current,
            [
                "현재 값을 인용과 함께 써라.",
                "현재 값과 인용을 답하라.",
                "지금 적용할 값을 근거 번호와 함께 알려줘.",
            ],
        ),
        (
            Intent::Previous,
            [
                "정정 전 과거 값을 인용과 함께 써라.",
                "이전 값을 인용해 답하라.",
                "바꾸기 전의 값을 근거 번호와 함께 알려줘.",
            ],
        ),
        (
            Intent::Restored,
            [
                "복원 후 현재 값을 인용과 함께 써라.",
                "복원된 값과 인용을 답하라.",
                "다시 복원한 뒤의 값을 근거 번호와 함께 알려줘.",
            ],
        ),
        (
            Intent::Cause,
            [
                "기록의 시간순서만으로 원인이 확정되는가?",
                "앞선 기록이 뒤 기록의 원인이라고 확정할 수 있는가?",
                "먼저 일어났다는 사실이 원인임을 입증하는가?",
            ],
        ),
    ] {
        if old.contains(&t) || phrases(i).contains(&t) {
            return Ok((e, c, i));
        }
    }
    Err(bad("unsupported question grammar"))
}
fn familiar(i: Intent) -> &'static str {
    match i {
        Intent::Full => "기록 원문 전체를 인용과 함께 써라.",
        Intent::Current => "현재 값을 인용과 함께 써라.",
        Intent::Previous => "정정 전 과거 값을 인용과 함께 써라.",
        Intent::Restored => "복원 후 현재 값을 인용과 함께 써라.",
        Intent::Cause => "기록의 시간순서만으로 원인이 확정되는가?",
    }
}
// Independent constrained resolver consumes serialized request only: no Episode
// binding, bucket, selected index, seed or expected answer is available here.
fn resolve(q: &ModelRequest) -> Result<String> {
    let (entity, context, intent) = question_intent(&q.input)?;
    let full = intent == Intent::Full;
    let past = intent == Intent::Previous;
    let cause = intent == Intent::Cause;
    let mut matched = vec![];
    for e in &q.evidence.items {
        let (name, ctx, value) = parsed_record(e)?;
        if name == entity && ctx == context {
            matched.push((e, value));
        }
    }
    if q.evidence.items.is_empty() {
        return Ok("근거가 없습니다.".into());
    }
    if matched.is_empty() {
        return Ok("요청한 대상의 근거가 없습니다.".into());
    }
    if cause {
        return Ok("시간순서만으로 원인은 확정되지 않습니다.".into());
    }
    matched.retain(|(e, _)| {
        if past {
            e.version_status == "superseded"
        } else {
            e.version_status == "current"
        }
    });
    if matched.is_empty() {
        return Ok("요청한 대상의 근거가 없습니다.".into());
    }
    let latest = matched
        .iter()
        .map(|(e, _)| e.observed_at.unwrap_or(e.recorded_at))
        .max()
        .unwrap();
    matched.retain(|(e, _)| e.observed_at.unwrap_or(e.recorded_at) == latest);
    if matched.len() != 1 {
        return Ok("근거가 모호하여 확정할 수 없습니다.".into());
    }
    let (e, value) = matched[0];
    Ok(if full {
        format!("{} [event:{}]", e.original_excerpt, e.event_id)
    } else {
        format!("{value}입니다. [event:{}]", e.event_id)
    })
}
fn generate(bases: usize, split: usize, seed: u64) -> Result<(Vec<Episode>, Vec<Meta>)> {
    let pools: Vec<_> = (1..=8).map(|len| entity_pool(split, len)).collect();
    if pools.iter().any(Vec::is_empty) {
        return Err(bad("finite entity namespace exhausted"));
    }
    let mut rng = Rng::new(seed);
    let mut episodes = vec![];
    let mut metadata = vec![];
    for bucket in 0..8 {
        for base in 0..bases {
            let pool = &pools[base % 8];
            let entity = &pool[rng.next_u64() as usize % pool.len()];
            let other_pool = &pools[(base + 1) % 8];
            let other = &other_pool[rng.next_u64() as usize % other_pool.len()];
            let context = format!("구역{}", rng.next_u64() % 1000);
            let other_context = format!("구역{}", 1000 + rng.next_u64() % 1000);
            let a = 1 + (rng.next_u64() % (10u64.pow((base % 8 + 1) as u32) - 1)) as i64;
            let b = if a == 1 || a % 2 == 0 { a + 1 } else { a - 1 };
            let c = a + 100_000_000;
            let time = 1000 + (rng.next_u64() % 100_000) as i64;
            let value = if base % 2 == 0 {
                ["왼쪽", "오른쪽", "직진", "대기"][base % 4].to_string()
            } else {
                format!(
                    "{:0width$}",
                    rng.next_u64() % 10u64.pow((base % 8 + 1) as u32),
                    width = base % 8 + 1
                )
            };
            let alt = if base % 2 == 0 {
                ["대기", "직진", "오른쪽", "왼쪽"][base % 4].to_string()
            } else {
                format!("{:0width$}", (rng.next_u64() % 9) + 1, width = base % 8 + 1)
            };
            let family = neural::hash(format!("{entity}/{context}/{time}/{a}/{bucket}").as_bytes());
            for view in 0..4 {
                let (v, w) = if view == 1 {
                    (&alt, &value)
                } else {
                    (&value, &alt)
                };
                let mut items = vec![record(entity, &context, v, a, time, "current")];
                let mut selected = 0;
                match bucket {
                    0 | 1 => {}
                    2 => items.push(record(other, &context, w, b, time, "current")),
                    3 => items.push(record(entity, &other_context, w, b, time, "current")),
                    4 => {
                        items[0].observed_at = Some(time - 1);
                        items[0].original_excerpt =
                            record(entity, &context, w, a, time, "current").original_excerpt;
                        items.push(record(entity, &context, v, b, time, "current"));
                        selected = 1;
                    }
                    5 => {
                        items[0].version_status = "superseded".into();
                        items.push(record(entity, &context, w, b, time + 1, "superseded"));
                        items.push(record(entity, &context, v, c, time + 2, "current"));
                        selected = if base % 2 == 0 { 1 } else { 2 };
                    }
                    6 => match base % 3 {
                        0 => items.clear(),
                        1 => items[0] = record(other, &context, w, a, time, "current"),
                        _ => items.push(record(entity, &context, w, b, time, "current")),
                    },
                    7 => items.push(record(entity, &context, w, b, time + 1, "current")),
                    _ => unreachable!(),
                }
                let past = bucket == 5 && base % 2 == 0;
                let form = if split == 2 {
                    2
                } else if view == 3 {
                    1
                } else {
                    0
                };
                let tasks = if bucket == 0 {
                    [
                        "기록 원문 전체를 인용과 함께 써라.",
                        "원문 전체와 인용을 답하라.",
                        "해당 기록의 문장 전체를 근거 번호와 함께 알려줘.",
                    ]
                } else if past {
                    [
                        "정정 전 과거 값을 인용과 함께 써라.",
                        "이전 값을 인용해 답하라.",
                        "바꾸기 전의 값을 근거 번호와 함께 알려줘.",
                    ]
                } else if bucket == 5 {
                    [
                        "복원 후 현재 값을 인용과 함께 써라.",
                        "복원된 값과 인용을 답하라.",
                        "다시 복원한 뒤의 값을 근거 번호와 함께 알려줘.",
                    ]
                } else if bucket == 7 {
                    [
                        "기록의 시간순서만으로 원인이 확정되는가?",
                        "앞선 기록이 뒤 기록의 원인이라고 확정할 수 있는가?",
                        "먼저 일어났다는 사실이 원인임을 입증하는가?",
                    ]
                } else {
                    [
                        "현재 값을 인용과 함께 써라.",
                        "현재 값과 인용을 답하라.",
                        "지금 적용할 값을 근거 번호와 함께 알려줘.",
                    ]
                };
                let answer = if bucket == 6 {
                    [
                        "근거가 없습니다.",
                        "요청한 대상의 근거가 없습니다.",
                        "근거가 모호하여 확정할 수 없습니다.",
                    ][base % 3]
                        .to_string()
                } else if bucket == 7 {
                    "시간순서만으로 원인은 확정되지 않습니다.".into()
                } else if bucket == 0 {
                    format!(
                        "{} [event:{}]",
                        items[selected].original_excerpt, items[selected].event_id
                    )
                } else {
                    format!(
                        "{}입니다. [event:{}]",
                        if past { w } else { v },
                        items[selected].event_id
                    )
                };
                // Transfer contains both unseen wording and unseen record-count combinations.
                if split == 2 && base >= 2 && bucket < 6 && items.len() < 3 {
                    items.push(record(other, &other_context, w, c + 1, time, "current"));
                }
                if view == 2 || (base + bucket) % 2 == 0 {
                    items.reverse();
                }
                let id = format!("{family}-{view}");
                let request = ModelRequest {
                    request_id: id.clone(),
                    system: SYSTEM.into(),
                    input: format!("{entity} {context} {}", tasks[form]),
                    evidence: EvidenceBundle {
                        items,
                        ..Default::default()
                    },
                    limits: GenerationLimits {
                        context_tokens: 2048,
                        max_tokens: 128,
                        timeout_ms: 120000,
                    },
                };
                if resolve(&request)? != answer {
                    return Err(bad("independent label disagreement"));
                }
                metadata.push(Meta {
                    id: id.clone(),
                    base: family.clone(),
                    template: format!(
                        "{}-{form}",
                        if bucket == 0 {
                            "full"
                        } else if past {
                            "past"
                        } else if bucket == 7 {
                            "cause"
                        } else {
                            "current"
                        }
                    ),
                    bucket,
                    view,
                    split: ["train", "primary", "transfer"][split].into(),
                    entities: vec![entity.clone(), other.clone()],
                    source_id: None,
                    query_context: None,
                });
                episodes.push(Episode {
                    id,
                    category: [3, 3, 0, 2, 1, 1, 4, 4][bucket],
                    family: family.clone(),
                    binding: format!("{entity}/{other}"),
                    sequence: format!("{entity}/{context}/{time}"),
                    request,
                    answer,
                });
            }
        }
    }
    data::validate_episodes(&episodes)?;
    Ok((episodes, metadata))
}
fn corpus(train: Vec<Episode>, dev: Vec<Episode>, seed: u64) -> Result<data::native::Corpus> {
    let m=data::CorpusManifest {version:1,scope:"educational synthetic Korean only".into(),permission:"synthetic project-owned".into(),generator:REVISION.into(),seed,split_rule:"full-entity SHA256 namespace mod3; base/scene disjoint; primary grammar shared; transfer phrasing heldout".into(),train:data::native::split("train",&train),validation:data::native::split("validation",&dev)};
    data::native::from_episodes(m, train, dev)
}
impl Plan {
    pub(super) fn is_answer_mean_study(&self) -> bool {
        identifiable::binding::citation::is_mean(self)
    }
    pub(super) fn framing(&self) -> neural::Framing {
        self.framing.unwrap_or_default()
    }
    pub(super) fn reuses_tokenizer_mapping(&self, tok: &ByteBpe) -> bool {
        self.identifiable.is_some() && self.tokenizer == tok.id()
    }
    pub(super) fn new_state_tokenizer_provenance(&self, corpus: &str, tok: &ByteBpe) -> Vec<String> {
        // The existing lineage slot also authenticates a frozen mapping's
        // training source. This does not represent exposure of the new weights.
        // Only new state construction calls this; never repair a resumed state.
        if self.reuses_tokenizer_mapping(tok) && corpus != tok.train_hash {
            vec![tok.train_hash.clone()]
        } else {
            Vec::new()
        }
    }
    #[cfg(feature = "test-support")]
    pub(super) fn is_tiny(&self) -> bool {
        self.tiny
    }
    pub(super) fn training_corpus(
        &self,
        path: &Path,
    ) -> Result<(data::CorpusManifest, Vec<Episode>, Vec<Episode>)> {
        let c = verified_corpus(path, &self.corpus)?;
        Ok((c.manifest, c.train, c.validation))
    }
    pub(super) fn source_identity(&self) -> &str {
        &self.source
    }
    pub(super) fn remaining_targets(&self, root: &Path) -> Result<Option<u64>> {
        if identifiable::binding::is(self) { return identifiable::binding::remaining(self, true).map(Some); }
        let Some(f) = &self.fork else { return Ok(None) };
        if self.paired.is_some() {
            return Ok(Some(1_000_000u64.checked_sub(paired_work(&f.study)?.1)
                .ok_or_else(|| bad("paired target budget exhausted"))?));
        }
        let mut used = 0u64;
        // run() already validated history before publishing this command's start.
        for i in 0..128 {
            let end = root.join(format!("segment-{i:04}-finished.r3b"));
            if !end.exists() {
                break;
            }
            let _: Segment = read_confirmed(&end)?;
            let c: binary::Value = read(&root.join(format!("segment-{i:04}/train-control.r3b")))?;
            used = used
                .checked_add(
                    c["executed_target_tokens_including_uncommitted"]
                        .as_u64()
                        .ok_or_else(|| bad("target usage UNKNOWN"))?,
                )
                .ok_or_else(|| bad("target usage overflow"))?;
        }
        Ok(Some(
            f.target_limit
                .checked_sub(used)
                .ok_or_else(|| bad("target budget exhausted"))?,
        ))
    }
    pub(super) fn remaining_input(&self) -> Result<Option<u64>> {
        if self.identifiable.is_some() { return identifiable::remaining_input(self).map(Some); }
        if self.paired.is_none() { return Ok(None); }
        Ok(Some(12_000_000u64.checked_sub(paired_work(&self.fork.as_ref().unwrap().study)?.0)
            .ok_or_else(|| bad("paired input budget exhausted"))?))
    }
    pub(super) fn learning_rate(&self, step: usize) -> f64 {
        if identifiable::binding::citation::is(self) || identifiable::binding::is_signal(self) || identifiable::binding::is_expansion(self) || identifiable::binding::is_consolidation(self) { return self.config.lr; }
        if identifiable::binding::is(self) { return self.config.lr * (step as f64 / self.config.warmup as f64).min(1.); }
        self.fork
            .as_ref()
            .map_or_else(|| self.config.learning_rate(step), |f| f.constant_lr)
    }
    pub(super) fn origin_step(&self) -> usize {
        self.fork.as_ref().map_or(0, |f| f.origin_step)
    }
    pub(super) fn epoch(&self, step: usize) -> usize {
        step / if identifiable::binding::is(self) { 32 } else { 1024 }
    }
    pub(super) fn observes_target_tokens(&self, step: usize) -> bool {
        if identifiable::binding::citation::is(self) {
            return step == self.origin_step() || self.evaluation_due(step + 1);
        }
        identifiable::binding::is(self) && step == 0
    }
    pub(super) fn pair_position(&self, absolute: usize) -> Option<binary::Value> {
        self.paired.as_ref().map(|t| binary::record!({"absolute":absolute,
            "local":absolute-t.first_step,"block":(absolute-t.first_step)/t.block,
            "within_block":(absolute-t.first_step)%t.block,"tape":self.train_order}))
    }
    pub(super) fn additional_samples(&self, root: &Path, tok: &ByteBpe) -> Result<Vec<Sample>> {
        let Some(f) = &self.fork else {
            return Ok(vec![]);
        };
        if f.variants.is_none() {
            return Ok(vec![]);
        }
        let c = verified_corpus(&root.join("variants.r3cor"), f.variants.as_ref().unwrap())?;
        let mut out = samples(&c.train, tok, self.config.seq_len)?;
        if let Some(h) = &f.selector {
            let c = verified_corpus(&root.join("selectors.r3cor"), h)?;
            out.extend(samples(&c.train, tok, self.config.seq_len)?);
        }
        Ok(out)
    }
    pub(super) fn training_draw(&self, step: usize) -> Vec<usize> {
        if let Some(tape) = &self.paired {
            return tape.at(step).expect("validated finite pair tape cursor").to_vec();
        }
        let mut ids = self.draw(step);
        if let Some(f) = &self.fork
            && f.variants.is_some()
        {
            let n = self.order.iter().map(Vec::len).sum::<usize>();
            let epoch = (step - f.origin_step) / self.order[0].len();
            for i in &mut ids {
                let original = *i;
                if f.alternate_first[*i] ^ (epoch % 2 == 1) {
                    *i += n;
                }
                if f.selector.is_some()
                    && f.flip_first[original].is_some_and(|bit| bit ^ (epoch % 2 == 1))
                {
                    *i += 2 * n;
                }
            }
        }
        ids
    }
    pub(super) fn apply_training_values(&self, root:&Path, tok:&ByteBpe, out:&mut Vec<Sample>)->Result<()> {
        if let Some(hash)=&self.training_values {
            let c=verified_corpus(&root.join("training-values.r3cor"),hash)?;
            let original=verified_corpus(&root.join("corpus.r3cor"),&self.corpus)?;
            if c.train.len()!=out.len() || c.manifest.validation.sha256!=original.manifest.validation.sha256 {return Err(bad("training value pool shape/evaluation binding"));}
            *out=samples(&c.train,tok,self.config.seq_len)?;
        }
        Ok(())
    }
    pub(super) fn grounding_labels(&self,root:&Path,tok:&ByteBpe,train:&[Sample])->Result<Option<Vec<Option<RecordGrounding>>>> {
        let Some(expected)=&self.grounding else {
            if self.paired.as_ref().is_some_and(|t|t.mode=="GROUND") {return Err(bad("grounding annotation required"));}
            return Ok(None);
        };
        if self.paired.as_ref().is_none_or(|t|t.mode!="GROUND") {return Err(bad("grounding objective missing"));}
        let c=verified_corpus(&root.join("training-values.r3cor"),self.training_values.as_ref().ok_or_else(||bad("grounding native pool required"))?)?;
        let framed=samples(&c.train,tok,self.config.seq_len)?;
        if framed.len()!=train.len() || framed.iter().zip(train).any(|(a,b)|a.tokens!=b.tokens || a.response_start!=b.response_start) {
            return Err(bad("grounding labels differ from actual training tokens"));
        }
        let (tm,_,_)=verified_metadata(root,self)?;
        let labels=record_groundings(&c.train,&tm,&framed)?;
        if digest(&labels)?!=*expected {return Err(bad("grounding annotation digest mismatch"));}
        Ok(Some(labels))
    }
    pub(super) fn sample_bucket(&self, index: usize) -> usize {
        if identifiable::binding::citation::retained_qa(self) {
            return if index<4608 {index/1536}else{3+((index-4608)%8192)/1024};
        }
        let n = self.order.iter().map(Vec::len).sum::<usize>();
        self.order.iter().position(|ids| ids.contains(&(index%n)))
            .expect("validated training sample bucket")
    }
    pub(super) fn parent_entry(&self, path: &Path, l: &checkpoint::Loaded) -> Result<bool> {
        let Some(f) = &self.fork else {
            return Ok(false);
        };
        let Some(s) = &l.manifest.training else {
            return Err(bad("fork missing parent state"));
        };
        let own_policy = digest(self)?;
        if s.step != f.origin_step
            || s.resume_binding
                .as_ref()
                .is_some_and(|b| hex(&b.policy) == own_policy)
        {
            return Ok(false);
        }
        if file_hash(path)? != self.initial
            || digest(s)? != f.parent_state
            || optimizer_hash(&l.optimizer)? != f.parent_adam
            || l.model.weight_hash()? != self.initial_weights
            || l.tokenizer.train_hash != f.tokenizer_training_hash
            || s.resume_binding
                .as_ref()
                .is_none_or(|b| hex(&b.policy) != f.parent_policy)
        {
            return Err(bad("explicit parent fork binding mismatch"));
        }
        Ok(true)
    }
    pub(super) fn evaluation_due(&self, step: usize) -> bool {
        [
            &self.evaluation.screen_steps,
            &self.evaluation.primary_steps,
            &self.evaluation.transfer_steps,
            &self.evaluation.train_steps,
            &self.evaluation.teacher_steps,
        ]
        .iter()
        .any(|s| s.contains(&step))
    }
    pub(super) fn binding(
        &self,
        s: &TrainingState,
        t: &ByteBpe,
    ) -> Result<checkpoint::ResumeBinding> {
        let mut b = checkpoint::ResumeBinding::default_for(s, t);
        b.framing = self.framing().digest();
        b.execution = 1;
        b.policy = checkpoint::ResumeBinding::digest_bytes(&binary::to_vec(self)?);
        b.provenance = b.policy;
        b.train_order = checkpoint::ResumeBinding::digest_bytes(self.train_order.as_bytes());
        if identifiable::binding::citation::answer_mean(self) {
            b.family = checkpoint::ANSWER_MEAN_FAMILY;
            b.normalizer = 2;
        }
        if let Some(tape) = &self.paired && ["CONTRAST","SIDE","REPLAY","WIDE","FIT","VALUE","COVER","DIVERSE","COVER4","GROUND"].contains(&tape.mode.as_str()) {
            b.family = if tape.mode == "CONTRAST" {3}else if tape.mode=="GROUND" {5}else{4};
            b.normalizer = b.family;
            b.span_alpha_bits = Some(0.1f64.to_bits());
            b.annotation = Some(checkpoint::ResumeBinding::digest_bytes(tape.samples.as_bytes()));
            if b.family==5 {
                let annotations=self.grounding.as_ref().ok_or_else(||bad("grounding policy annotation absent"))?;
                b.annotation=Some(checkpoint::ResumeBinding::digest_bytes(&binary::to_vec(&(&tape.samples,annotations,
                    "last-layer/all-heads/first-response/record-content-ratio/floor1e-8/coefficient0.1"))?));
            }
        }
        Ok(b)
    }
    pub(super) fn contrast_pairs(&self, indices: &[usize]) -> Result<Option<(bool,Vec<(usize,usize)>)>> {
        let Some(tape) = self.paired.as_ref().filter(|t| ["CONTRAST","SIDE","REPLAY","WIDE","FIT","VALUE","COVER","DIVERSE","COVER4","GROUND"].contains(&t.mode.as_str())) else { return Ok(None); };
        let n = self.order.iter().map(Vec::len).sum::<usize>();
        if n == 0 || indices.len() != 8 || indices.iter().any(|&i| i >= 4*n) {
            return Err(bad("contrast sample bounds"));
        }
        let mut pairs = vec![];
        let mut selectors = 0;
        for (row,&i) in indices.iter().enumerate() {
            if !(2..=4).contains(&self.sample_bucket(i)) { continue; }
            selectors += 1;
            if i < 2*n {
                let other = indices.iter().position(|&v| v == i+2*n)
                    .ok_or_else(|| bad("contrast opposite side absent"))?;
                pairs.push((row,other));
            }
        }
        if selectors != 2*pairs.len() || ![0,3].contains(&pairs.len()) {
            return Err(bad("contrast requires complete C/D/E pairs or anchor-only batch"));
        }
        Ok(Some((tape.mode!="CONTRAST",pairs)))
    }
    pub(super) fn draw(&self, step: usize) -> Vec<usize> {
        if let Some(policy) = &self.identifiable {
            return policy.rows[step].to_vec();
        }
        if let Some(tape) = &self.paired {
            let n = self.order.iter().map(Vec::len).sum::<usize>();
            return tape.at(step).expect("validated finite pair tape cursor").iter().map(|i| i % n).collect();
        }
        let epoch = step / self.order[0].len();
        let position = step % self.order[0].len();
        (0..8)
            .map(|bucket| {
                let order = &self.order[bucket];
                let bases = order.len() / 4;
                let mut permutation: Vec<_> = (0..bases).collect();
                let mut r = Rng::new(
                    self.config.seed
                        ^ (bucket as u64 + 1).wrapping_mul(0x9e3779b97f4a7c15)
                        ^ (epoch as u64).wrapping_mul(0x517cc1b727220a95),
                );
                for i in (1..bases).rev() {
                    let j = r.next_u64() as usize % (i + 1);
                    permutation.swap(i, j);
                }
                // Each pass uses a different view; adjacent draws use distinct bases.
                order[permutation[position % bases] * 4 + (position / bases + epoch + bucket) % 4]
            })
            .collect()
    }
}
#[cfg(any(test, feature = "test-support"))]
fn numeric_token_fixture(model: &Transformer, id: u32) -> Result<()> {
    if model.config.hidden != 32 || model.config.layers != 2 {return Err(bad("TINY tensor fixture only"));}
    for (name,v) in &model.vars {
        let mut data=vec![if name.ends_with("norm")||name=="embedding"{1f32}else{0.};v.elem_count()];
        if name=="embedding" {let h=model.config.hidden;data[id as usize*h..(id as usize+1)*h].fill(2.);}
        v.set(&Tensor::from_vec(data,v.dims(),&Device::Cpu)?)?;
    }
    Ok(())
}
fn prepare(root: &Path, tiny: bool) -> Result<()> {
    std::fs::create_dir(root)?;
    let tiny_bases=if cfg!(feature="test-support") && std::env::var("R3_FRESH_FIXTURE_VALUE_DONORS").as_deref()==Ok("1") {4}else{2};
    let (train, tm) = generate(if tiny { tiny_bases } else { 256 }, 0, 20260919)?;
    let (dev, dm) = generate(if tiny { 1 } else { 16 }, 1, 20260920)?;
    let (transfer, xm) = generate(if tiny { 1 } else { 4 }, 2, 20260921)?;
    let c = corpus(train.clone(), dev.clone(), 20260919)?;
    let x = corpus(train.clone(), transfer.clone(), 20260919)?;
    data::native::write(&root.join("corpus.r3cor"), &c, true)?;
    data::native::write(&root.join("transfer.r3cor"), &x, true)?;
    write(&root.join("metadata.r3b"), &(tm.clone(), dm, xm))?;
    data::tokenizer(
        &root.join("corpus.r3cor"),
        &root.join("tokenizer.r3b"),
        4096,
    )?;
    let tok = ByteBpe::load(&root.join("tokenizer.r3b"))?;
    let mut prompts = HashMap::new();
    for es in [&train, &dev, &transfer] {
        let framed = samples(es, &tok, 512)?;
        for (e, s) in es.iter().zip(&framed) {
            let p = tok.prepare(&e.request, 2048, "length-check")?;
            if s.tokens.len() > 513
                || p.provided.len() != e.request.evidence.items.len()
                || !p.excluded.is_empty()
            {
                return Err(bad("full evidence/sequence length"));
            }
            if prompts
                .insert(s.tokens[..s.response_start].to_vec(), e.answer.clone())
                .is_some_and(|old| old != e.answer)
            {
                return Err(bad("conflicting target for identical prompt"));
            }
        }
        println!(
            "FRESH_DATA examples={} input_target_tokens={} max_sequence={}",
            es.len(),
            framed.iter().map(|s| s.tokens.len()).sum::<usize>(),
            framed.iter().map(|s| s.tokens.len()).max().unwrap()
        );
    }
    let mut config = TrainConfig {
        lr: 3e-4,
        warmup: 128,
        max_steps: 4096,
        max_tokens: 20_000_000,
        microbatch: 8,
        accumulation: 1,
        validate_every: 512,
        seed: 29,
        ..Default::default()
    };
    let mut architecture = if tiny {
        Config::tiny(tok.vocab_size())
    } else {
        Config::small(tok.vocab_size())
    };
    if tiny {
        architecture.context = 512;
        architecture.profile = "NATIVE_TRPP_EXPERIMENTAL_V1".into();
        config.max_steps = 2;
        config.warmup = 0;
        config.validate_every = 2;
    }
    let estimate =
        config.microbatch * architecture.layers * architecture.heads * 512 * 512 * 4 * 12
            + architecture.parameters() * 4 * 16;
    if estimate > 16usize * 1024 * 1024 * 1024 {
        config.microbatch = 2;
        config.accumulation = 4;
    }
    config.validate(architecture.context)?;
    let source = source_digest()?;
    let model = Transformer::init(architecture.clone(), 17, Device::Cpu)?;
    #[cfg(feature = "test-support")]
    if tiny
        && matches!(
            std::env::var("R3_FRESH_FIXTURE_EOS").as_deref(),
            Ok("1" | "token")
        )
    {
        // Numerical tensor fixture only. The token is selected by real logits/greedy,
        // not by a decoder override; ordinary SMALL preparation cannot enter it.
        let id=if std::env::var("R3_FRESH_FIXTURE_EOS").as_deref()==Ok("token"){tok.encode(b"x")?[0]}else{EOS};
        numeric_token_fixture(&model,id)?;
    }
    let m = checkpoint::initialized(&model, &tok, 17, source.clone())?;
    checkpoint::save(&root.join("initial.r3m"), &model, &tok, m, &BTreeMap::new())?;
    let order: Vec<Vec<usize>> = (0..8)
        .map(|b| {
            tm.iter()
                .enumerate()
                .filter_map(|(i, m)| (m.bucket == b).then_some(i))
                .collect()
        })
        .collect();
    let plan = Plan {
        framing: None,
        identifiable: None,
        schema: Some(2),
        fork: None,
        paired: None,
        training_values: None,
        grounding: None,
        revision: REVISION.into(),
        source,
        binary: file_hash(&std::env::current_exe()?)?,
        corpus: file_hash(&root.join("corpus.r3cor"))?,
        transfer: file_hash(&root.join("transfer.r3cor"))?,
        tokenizer: tok.id(),
        initial: file_hash(&root.join("initial.r3m"))?,
        initial_weights: model.weight_hash()?,
        config,
        architecture,
        sampler: "bucket-base-permutation-v1".into(),
        train_order: digest(&order)?,
        order,
        split_policy: c.manifest.split_rule,
        data_seed: 20260919,
        model_seed: 17,
        tiny,
        metadata: file_hash(&root.join("metadata.r3b"))?,
        evaluation: if tiny {
            EvaluationPolicy {
                screen_steps: vec![],
                primary_steps: vec![2],
                transfer_steps: vec![2],
                train_steps: vec![2],
                teacher_steps: vec![],
                fixture_per_bucket: Some(1),
                ..EvaluationPolicy::default()
            }
        } else {
            EvaluationPolicy {
                teacher_steps: (0..=4096).step_by(512).collect(),
                ..EvaluationPolicy::default()
            }
        },
    };
    write(&root.join("plan.r3b"), &plan)?;
    let check: Plan = read(&root.join("plan.r3b"))?;
    if check != plan {
        return Err(bad("plan readback"));
    }
    println!(
        "FRESH_PREPARED root={} vocab={} parameters={} initial_weights={} plan={} model=NOT_TRAINED planning_bytes={estimate} optimizer=0",
        root.display(),
        tok.vocab_size(),
        model.config.parameters(),
        plan.initial_weights,
        digest(&plan)?
    );
    Ok(())
}
fn source_digest() -> Result<String> {
    let mut bytes = vec![];
    // Compile-time source identity; no temporary instruction is a runtime input.
    for source in [
        include_bytes!("fresh.rs").as_slice(),
        include_bytes!("identifiable.rs").as_slice(),
        include_bytes!("binding.rs").as_slice(),
        include_bytes!("value_citation.rs").as_slice(),
        include_bytes!("word_value.rs").as_slice(),
        include_bytes!("protected_adaptation.rs").as_slice(),
        include_bytes!("precision_probe.rs").as_slice(),
        include_bytes!("training.rs").as_slice(),
        include_bytes!("data.rs").as_slice(),
        include_bytes!("native_corpus.rs").as_slice(),
        include_bytes!("neural.rs").as_slice(),
        include_bytes!("model.rs").as_slice(),
        include_bytes!("token_cache.rs").as_slice(),
        include_bytes!("neural/transformer.rs").as_slice(),
        include_bytes!("neural/artifact.rs").as_slice(),
        include_bytes!("neural/checkpoint.rs").as_slice(),
        include_bytes!("quality_recovery.rs").as_slice(),
        include_bytes!("codec.rs").as_slice(),
        include_bytes!("binary.rs").as_slice(),
        include_bytes!("train_main.rs").as_slice(),
        include_bytes!("../Cargo.lock").as_slice(),
    ] {
        bytes.extend_from_slice(source);
    }
    Ok(neural::hash(&bytes))
}
pub fn execute(command: Command) -> Result<()> {
    match command {
        Command::MetalRuntime {action} => metal_runtime::run(action),
        Command::Muon {action} => muon::run(action),
        Command::AnswerMeanPrepare { previous,output } => identifiable::binding::citation::mean_prepare(&previous,&output,false),
        Command::CitationContinuePrepare { previous,output } => identifiable::binding::citation::continuation_prepare(&previous,&output,false),
        Command::CitationFidelityPrepare { previous,output,parent_review } => identifiable::binding::citation::fidelity_prepare(&previous,&output,&parent_review,false),
        Command::CitationPrecisionPrepare { previous,output,parent_review } => identifiable::binding::citation::precision_prepare(&previous,&output,&parent_review,false),
        Command::RetainedQaPrepare {previous,balanced,old_qa,output,parent_review} => identifiable::binding::citation::qa_prepare(&previous,&balanced,&old_qa,&output,&parent_review,false),
        Command::RetainedQaParent {study,panel} => identifiable::binding::citation::qa_parent(&study,&panel),
        Command::RetainedQaReport {study} => identifiable::binding::citation::qa_report(&study),
        Command::RetainedQaRecount {study,output} => identifiable::binding::citation::qa_recount(&study,&output),
        Command::QaBridgeProbePrepare {previous,balanced,output,review_a1} => identifiable::binding::citation::bridge_probe_prepare(&previous,&balanced,&output,&review_a1),
        Command::QaBridgeProbe {study,panel} => identifiable::binding::citation::bridge_probe(&study,&panel),
        Command::QaBridgeProbeReport {study} => identifiable::binding::citation::bridge_probe_report(&study),
        Command::QaBridgePrepare {probe,old_qa,continue_from,output} => match continue_from {
            Some(parent)=>identifiable::binding::citation::bridge_prepare_completion(&parent,&output,false),
            None=>identifiable::binding::citation::bridge_prepare(probe.as_deref().ok_or_else(||bad("bridge probe required"))?,old_qa.as_deref().ok_or_else(||bad("bridge original QA required"))?,&output),
        },
        Command::QaBridgeReport {study} => identifiable::binding::citation::bridge_report(&study),
        Command::WordPrepare {previous,output} => identifiable::binding::citation::word::prepare(&previous,&output,false),
        Command::AdapterPrepare {previous,output} => identifiable::binding::citation::adapt::prepare(&previous,&output,false),
        Command::AdapterParity {study} => identifiable::binding::citation::adapt::parity(&study),
        Command::Fp4Prepare {parent,output} => identifiable::binding::citation::fp4::prepare(&parent,&output),
        Command::Fp4Observe {study,parity} => identifiable::binding::citation::fp4::observe(&study,parity),
        Command::WordParent {study,panel} => identifiable::binding::citation::word::parent_observe(&study,&panel),
        Command::WordReport {study} => identifiable::binding::citation::word::report(&study),
        Command::WordReview {study} => identifiable::binding::citation::word::review(&study),
        Command::AdapterReviewComplete {study,output,check} => identifiable::binding::citation::word::complete_review(&study,&output,check),
        Command::WordQa {study,transfer} => identifiable::binding::citation::word::qa(&study,transfer),
        Command::QaBridgeReview {study,errors,parent} => if parent{identifiable::binding::citation::bridge_parent_parity(&study)}else{identifiable::binding::citation::bridge_review(&study,errors)},
        Command::QaBridgeQa {study,transfer,diagnostic} => if diagnostic{identifiable::binding::citation::bridge_diagnostic_qa(&study,transfer)}else{identifiable::binding::citation::bridge_qa(&study,transfer)},
        Command::RetainedQaReview {root,panel} => identifiable::binding::citation::qa_review(&root,&panel),
        Command::CitationContinueParent { study,citation } => identifiable::binding::citation::continuation_parent(&study,citation),
        Command::AnswerMeanParent { study } => identifiable::binding::citation::mean_parent(&study),
        Command::AnswerMeanReport { study } => identifiable::binding::citation::mean_report(&study),
        Command::AnswerMeanReview { root,citation } => identifiable::binding::citation::mean_review(&root,citation),
        Command::CitationPrepare {parent,output,reservation,used_ids} => identifiable::binding::citation::prepare(&parent,&output,&reservation,&used_ids),
        Command::CitationParent {study,citation} => identifiable::binding::citation::parent_observe(&study,citation),
        Command::CitationReport {study} => identifiable::binding::citation::report(&study),
        Command::CitationParity {study,citation,reviewer} => identifiable::binding::citation::parity(&study,citation,reviewer),
        Command::CitationSeal {study,reservation_private} => identifiable::binding::citation::seal(&study,&reservation_private),
        Command::CitationConfirm {study} => identifiable::binding::citation::confirm(&study),
        Command::ConsolidationPrepare {parent,output} => identifiable::binding::consolidation_prepare(&parent,&output,false),
        Command::ConsolidationParent {study} => identifiable::binding::consolidation_parent_parity(&study),
        Command::ConsolidationReport {study} => identifiable::binding::consolidation_report(&study),
        Command::ExpansionPrepare {parent,output} => identifiable::binding::expansion_prepare(&parent,&output),
        Command::ExpansionParent {study,new_pool} => identifiable::binding::expansion_parent_observe(&study,new_pool),
        Command::ExpansionReport {study} => identifiable::binding::expansion_report(&study),
        Command::SignalPrepare {parent,output} => identifiable::binding::signal_prepare(&parent,&output,false),
        Command::SignalParentParity {study} => identifiable::binding::signal_parent_parity(&study),
        Command::SignalReport {study} => identifiable::binding::signal_report(&study),
        Command::FramingPrepare { parent, output } => {
            identifiable::binding::framing_prepare(&parent, &output, false)
        }
        Command::FramingDecide { study } => identifiable::binding::framing_decide(&study),
        Command::FramingCompare { study } => identifiable::binding::framing_compare(&study),
        Command::FramingLegacyParity { study } => {
            identifiable::binding::framing_legacy_parity(&study)
        }
        Command::BindingPrepare { parent, output } => {
            identifiable::binding::prepare(&parent, &output, false)
        }
        Command::BindingReport { root } => identifiable::binding::report(&root),
        Command::BindingParity { root } => identifiable::binding::parity(&root),
        Command::BindingProbe { root } => identifiable::binding::probe(&root),
        Command::OrbitSwap { parent, output } => identifiable::binding::orbit_swap(&parent, &output),
        Command::OrbitPrepare { parent, output } => identifiable::binding::orbit_prepare(&parent, &output, false),
        Command::OrbitSeal { study } => identifiable::binding::orbit_seal(&study),
        Command::OrbitCompare { study } => identifiable::binding::orbit_compare(&study),
        Command::OrbitReviewParity { root } => { let p=identifiable::binding::historical_plan(&root)?; identifiable::binding::orbit_parity(&root,&p,true) },
        Command::OrbitConfirm { study } => identifiable::binding::orbit_confirm(&study),
        Command::IdentifiableAudit { roots, diagnostics, output } => identifiable::audit(&roots, &diagnostics, &output),
        Command::IdentifiablePrepare { parent, output } => identifiable::prepare(&parent, &output),
        Command::PairedContinue { parent, output, frozen_executable, fixed_cover_exposure, selector_phrase_exposure, parity } => paired_continue(&parent, &output, &frozen_executable, fixed_cover_exposure || selector_phrase_exposure, selector_phrase_exposure, parity.as_deref()),
        Command::PairedPrepare { parent, source_data, output, parity, first_target_four, learning_rate_threefold, co_batch, pair_contrast, sidewise_contrast, repeat_pair_block, repeat_two_blocks, fit_seen_pairs, alternate_pair_values, cover_value_pairs, diverse_pair_values, ground_selected_record } => paired_prepare(&parent, &source_data, &output, parity.as_deref(), [first_target_four, learning_rate_threefold, co_batch, pair_contrast, sidewise_contrast, repeat_pair_block, repeat_two_blocks, fit_seen_pairs, alternate_pair_values, cover_value_pairs, diverse_pair_values, ground_selected_record]),
        Command::PairedReport { root } => paired_report(&root).map(|r| println!("PAIRED_REPORT {r}")),
        Command::StudyPrepare { parent, output, selector } => study_prepare(&parent, &output, selector),
        Command::StudyObserve { root } => study_observe(&root),
        Command::StudyReport {
            root,
            frozen_executable,
        } => study_report(&root, frozen_executable.as_deref()),
        Command::Prepare { output } => prepare(&output, false),
        Command::Run { root } => run(&root, false),
        Command::Report { root } => report(&root),
        #[cfg(feature = "test-support")]
        Command::Fixture { output } => prepare(&output, true),
        #[cfg(feature = "test-support")]
        Command::FixtureFull { root } => run(&root, true),
        #[cfg(feature = "test-support")]
        Command::FixtureEval { root } => {
            let p = plan_read(&root)?;
            if !p.tiny {
                return Err(bad("numerical fixture required"));
            }
            let history = history(&root, &p)?;
            let last = history
                .last()
                .ok_or_else(|| bad("fixture missing checkpoint"))?;
            let (_, _, dev) = data::load(&root.join("corpus.r3cor"))?;
            let (_, dm, _): (Vec<Meta>, Vec<Meta>, Vec<Meta>) = read(&root.join("metadata.r3b"))?;
            let mut control = recovery::RunControl::command(false)?;
            evaluate_panel(
                &p,
                &root,
                &root.join(&last.checkpoint),
                last.step,
                "fixture",
                &dev[..1],
                &dm[..1],
                &mut control,
            )?;
            println!("FIXTURE_EVAL {}", control.receipt());
            Ok(())
        }
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
struct Segment {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    schema: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    start_hash: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    control_hash: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    phase: Option<String>,
    policy: String,
    checkpoint: String,
    checkpoint_hash: String,
    step: usize,
    input: u64,
    target: u64,
    elapsed: f64,
    generations: usize,
    teachers: usize,
    resume: bool,
    stop: String,
}
fn plan_read(root: &Path) -> Result<Plan> {
    plan_read_bound(
        root,
        &source_digest()?,
        &file_hash(&std::env::current_exe()?)?,
    )
}
fn plan_read_bound(root: &Path, source: &str, executable: &str) -> Result<Plan> {
    let p: Plan = read(&root.join("plan.r3b"))?;
    if p.revision != REVISION
        || p.source != source
        || p.binary != executable
        || p.sampler != if identifiable::binding::is(&p) { "query-pair-tape-v1" } else { "bucket-base-permutation-v1" }
        || p.order.len() != if identifiable::binding::is(&p) { 1 } else { 8 }
        || (p.fork.is_none() && p.training_values.is_some())
        || p.train_order != if let Some(policy) = &p.identifiable { digest(&policy.rows)? } else if let Some(tape) = &p.paired { digest(&tape.rows)? } else { digest(&p.order)? }
    {
        return Err(bad("source/binary/sampler plan mismatch"));
    }
    for (name, expected) in [
        ("corpus.r3cor", &p.corpus),
        ("transfer.r3cor", &p.transfer),
        ("initial.r3m", &p.initial),
        ("metadata.r3b", &p.metadata),
    ] {
        if &file_hash(&root.join(name))? != expected {
            return Err(bad("frozen input changed"));
        }
    }
    let expected = if p.identifiable.is_some() {
        identifiable::verify_plan(root, &p)?;
        if identifiable::binding::is(&p) { identifiable::binding::evaluation_for(&p) } else { identifiable::evaluation() }
    } else if let Some(f) = &p.fork {
        let study: Study = read(&f.study.join("study.r3b"))?;
        if file_hash(&f.study.join("study.r3b"))? != f.study_hash
            || study.source != p.source
            || study.binary != p.binary
            || study.parent_step != f.origin_step
            || study.parent_model != p.initial_weights
            || study.parent_file != p.initial
            || study.parent_adam != f.parent_adam
            || !study.tie_break.contains(&f.arm)
            || f.constant_lr.to_bits() != if study.schema==7 {9e-5f64}else{3e-5f64}.to_bits()
            || p.config.max_steps != f.origin_step + study.updates
            || p.config.budget_start_step != f.origin_step
            || p.config.budget_start_tokens != f.origin_input
            || p.config.warmup != 0
            || p.config.lr != f.constant_lr
            || p.config.first_target_weight != if study.schema==6 {4.}else{1.}
            || (study.schema==6 && study.updates!=if p.tiny {2}else{3840})
            || ([7,8,9,10].contains(&study.schema) && study.updates!=if p.tiny {2}else{1280})
            || ([11,13,14].contains(&study.schema) && study.updates!=if p.tiny {4}else{1280})
            || ([12,15,18].contains(&study.schema) && study.updates!=if p.tiny {8}else{1280})
            || (study.schema==19 && study.updates!=if p.tiny {16}else{1280})
            || (study.schema==20 && study.updates!=if p.tiny {2}else{1280})
            || ([16,17].contains(&study.schema) && study.updates!=if p.tiny {8}else{2560})
            || p.corpus != f.original_corpus
            || p.tokenizer != study.tokenizer
            || p.training_values.is_some() != [18,19,20].contains(&study.schema)
            || p.grounding.is_some() != (study.schema==20)
        {
            return Err(bad("unsupported explicit fork policy"));
        }
        if let Some(h) = &f.variants
            && &file_hash(&root.join("variants.r3cor"))? != h
        {
            return Err(bad("frozen question variants changed"));
        }
        if let Some(h) = &f.variant_metadata
            && &file_hash(&root.join("question-variants.r3b"))? != h
        {
            return Err(bad("frozen variant metadata changed"));
        }
        if let Some(tape) = &p.paired {
            let n = p.order.iter().map(Vec::len).sum::<usize>();
            if ![4,5,6,7,8,9,10,11,12,13,14,15,16,17,18,19,20].contains(&study.schema) || !["SPACED", "ADJACENT", "COBATCH", "CONTRAST", "SIDE", "REPLAY", "WIDE", "FIT","VALUE","COVER","DIVERSE","COVER4","GROUND"].contains(&tape.mode.as_str())
                || (study.schema==8) != (tape.mode=="COBATCH")
                || (study.schema==9) != (tape.mode=="CONTRAST")
                || (study.schema==10) != (tape.mode=="SIDE")
                || (study.schema==11) != (tape.mode=="REPLAY")
                || (study.schema==12) != (tape.mode=="WIDE")
                || (study.schema==13) != (tape.mode=="FIT")
                || (study.schema==14) != (tape.mode=="VALUE")
                || (study.schema==18) != (tape.mode=="DIVERSE")
                || (study.schema==19) != (tape.mode=="COVER4")
                || (study.schema==20) != (tape.mode=="GROUND")
                || ([15,16,17].contains(&study.schema)) != (tape.mode=="COVER")
                || tape.mode != f.arm || (![5,16,17].contains(&study.schema) && tape.first_step != f.origin_step)
                || tape.block != if p.tiny { 2 } else { 256 }
                || tape.rows.len() != if p.tiny { if [19,20].contains(&study.schema) {16}else if [12,15,16,17,18].contains(&study.schema) {8}else if [11,13,14].contains(&study.schema) {4}else{2} } else { 3840 }
                || f.origin_step + study.updates > tape.first_step + tape.rows.len()
                || tape.samples != file_hash(&root.join("paired-samples.r3rows"))?
                || tape.rows.iter().any(|r| r.iter().enumerate().any(|(b, i)| *i >= 4*n || (![8,9,10,11,12,13,14,15,16,17,18,19,20].contains(&study.schema) && !p.order[b].contains(&(i%n)))))
            { return Err(bad("paired policy/tape boundary")); }
            let parent: Plan = read(&study.parent.join("plan.r3b"))?;
            let (tm, _, _) = verified_metadata(root, &p)?;
            if [5,16,17].contains(&study.schema) {
                let mut expected=parent.paired.clone().ok_or_else(||bad("continuation parent tape"))?;
                if p.tiny {expected.first_step=f.origin_step;}
                if study.schema==17 {
                    expected.rows=selector_phrase_rows(&expected.rows,&tm,
                        f.origin_step-expected.first_step,if p.tiny {4}else{64})?;
                }
                if expected!=*tape || f.origin_step!=parent.config.max_steps
                    || p.config.max_steps!=tape.first_step+tape.rows.len() {
                    return Err(bad("continuation must preserve its exact parent tape/cursor"));
                }
                plan_read_bound(&study.parent, &parent.source, &parent.binary)?;
            } else { verify_pair_tape(&parent, &tm, tape)?; }
        } else if [4,5,6,7,8,9,10,11,12,13,14,15,16,17,18,19,20].contains(&study.schema) { return Err(bad("paired study missing finite tape")); }
        if let Some(hash)=&p.training_values
            && file_hash(&root.join("training-values.r3cor"))?!=*hash {
            return Err(bad("frozen training values changed"));
        }
        for (name, h) in [
            ("selectors.r3cor", &f.selector),
            ("selector-metadata.r3b", &f.selector_metadata),
        ] {
            if let Some(h) = h
                && &file_hash(&root.join(name))? != h
            {
                return Err(bad("frozen selector data changed"));
            }
        }
        let mut e = if p.paired.is_some() { paired_evaluation(f.origin_step, study.updates, p.tiny) }
            else { fork_evaluation(f.origin_step, study.updates, p.tiny) };
        if study.schema == 3 {
            e.generation_limit = 4608;
        }
        e
    } else if p.tiny {
        EvaluationPolicy {
            screen_steps: vec![],
            primary_steps: vec![2],
            transfer_steps: vec![2],
            train_steps: vec![2],
            teacher_steps: vec![],
            fixture_per_bucket: Some(1),
            ..EvaluationPolicy::default()
        }
    } else {
        EvaluationPolicy {
            teacher_steps: (0..=4096).step_by(512).collect(),
            ..EvaluationPolicy::default()
        }
    };
    if p.schema != Some(2) || p.evaluation != expected {
        return Err(bad("unsupported evaluation policy"));
    }
    Ok(p)
}
fn history(root: &Path, p: &Plan) -> Result<Vec<Segment>> {
    let mut out = vec![];
    for i in 0..128 {
        if !root.join(format!("segment-{i:04}-started.r3b")).exists() {
            return Ok(out);
        }
        let start = root.join(format!("segment-{i:04}-started.r3b"));
        let s: Segment = read_confirmed(&root.join(format!("segment-{i:04}-finished.r3b")))
            .map_err(|_| bad("started segment without completion: usage UNKNOWN, retry blocked"))?;
        if s.policy != digest(p)? || file_hash(&root.join(&s.checkpoint))? != s.checkpoint_hash {
            return Err(bad("segment binding"));
        }
        let started: binary::Value = read(&start)?;
        let input = Path::new(
            started["checkpoint"]
                .as_str()
                .ok_or_else(|| bad("start checkpoint"))?,
        );
        let control_path = root.join(format!("segment-{i:04}/train-control.r3b"));
        let c: binary::Value = read(&control_path)?;
        if s.schema != Some(2)
            || s.start_hash.as_ref() != Some(&file_hash(&start)?)
            || s.control_hash.as_ref() != Some(&file_hash(&control_path)?)
            || started["policy"] != s.policy
            || started["physical"] != file_hash(input)?
            || c["checkpoint_saved"] != true
            || !c["save_error"].is_null()
        {
            return Err(bad("unconfirmed/failed segment; optimizer/generation=0"));
        }
        let (m, _) = checkpoint::metadata(&root.join(&s.checkpoint))?;
        let state = m
            .training
            .ok_or_else(|| bad("segment missing native state"))?;
        let expected_prior = out.last().map_or(p.initial.as_str(), |prior: &Segment| {
            prior.checkpoint_hash.as_str()
        });
        if started["physical"] != expected_prior
            || s.step != state.step
            || s.input != state.consumed_tokens
            || s.target != state.target_tokens
            || s.generations as u64
                != c["generation_calls"]
                    .as_u64()
                    .ok_or_else(|| bad("generation usage UNKNOWN"))?
            || s.teachers as u64
                != c["teacher_calls"]
                    .as_u64()
                    .ok_or_else(|| bad("teacher usage UNKNOWN"))?
            || !s.elapsed.is_finite()
            || s.elapsed < 0.
            || (s.resume && (!matches!(s.stop.as_str(), "TIME_BUDGET" | "TRAINING") || !signal_segment_resumable(&c)))
        {
            return Err(bad("segment native/counter/chain mismatch"));
        }
        if c["final_evaluation_complete"] == true && p.evaluation_due(s.step) {
            let (_, panels) = audit_panels_range(root, p, s.step, s.step)?;
            let native = checkpoint::load(&root.join(&s.checkpoint), Device::Cpu, false)?;
            let model = native.model.weight_hash()?;
            if panels.values().any(|panel| panel.model != model) {
                return Err(bad("completed panel differs from segment checkpoint"));
            }
        }
        if s.phase.as_deref() == Some("Finished")
            && p.evaluation_due(s.step)
            && c["final_evaluation_complete"] != true
        {
            return Err(bad("Finished with outstanding evaluation"));
        }
        out.push(s);
    }
    Err(bad("segment count bound"))
}
fn run(root: &Path, uninterrupted_fixture: bool) -> Result<()> {
    let p = plan_read(root)?;
    if p.identifiable.is_some() { identifiable::authorize_run(root, &p)?; }
    let previous = history(root, &p)?;
    if uninterrupted_fixture && !p.tiny {
        return Err(bad("fixture requires TINY"));
    }
    if previous.last().is_some_and(|s| !s.resume) {
        return Err(bad("closed run cannot resume"));
    }
    let (elapsed, generations, teachers) = if p.identifiable.is_some() {
        identifiable::usage(&p)?
    } else if let Some(f) = &p.fork {
        if p.paired.is_some() { paired_usage(&f.study)? } else { study_usage(&f.study, true)? }
    } else {
        (
            previous.iter().map(|s| s.elapsed).sum(),
            previous.iter().map(|s| s.generations).sum(),
            previous.iter().map(|s| s.teachers).sum(),
        )
    };
    let evaluation_pending=previous.last().is_some_and(|s|s.phase.as_deref()==Some("EvaluationPending"));
    // A complete RETURNED prefix may consume the exact call cap before its
    // summary/decision is durable. Evaluation-only reentry reserves zero calls;
    // RunControl still rejects any additional generation or teacher invocation.
    if elapsed >= p.evaluation.active_seconds as f64
        || generations > p.evaluation.generation_limit
        || teachers > p.evaluation.teacher_limit
        || (!evaluation_pending && (generations == p.evaluation.generation_limit
            || teachers == p.evaluation.teacher_limit))
    {
        return Err(bad("total budget exhausted"));
    }
    let step = previous.last().map_or(p.origin_step(), |s| s.step);
    let input = root.join(
        previous
            .last()
            .map_or("initial.r3m", |s| s.checkpoint.as_str()),
    );
    let index = previous.len();
    let output = root.join(format!("segment-{index:04}"));
    let stop_after = if previous.is_empty() && !uninterrupted_fixture {
        p.origin_step() + 1
    } else if identifiable::binding::citation::is(&p) {
        identifiable::binding::citation::endpoint(&p,step,previous.last().is_some_and(|s|s.phase.as_deref()==Some("EvaluationPending")))?
    } else if identifiable::binding::is_consolidation(&p) {
        identifiable::binding::consolidation_endpoint(&p,step,previous.last().is_some_and(|s|s.phase.as_deref()==Some("EvaluationPending")))?
    } else if identifiable::binding::is_expansion(&p) {
        identifiable::binding::expansion_endpoint(&p,step,previous.last().is_some_and(|s|s.phase.as_deref()==Some("EvaluationPending")))?
    } else if identifiable::binding::is_signal(&p) {
        identifiable::binding::signal_endpoint(&p,step,previous.last().is_some_and(|s| s.phase.as_deref()==Some("EvaluationPending")))?
    } else if identifiable::binding::is_framing(&p) {
        identifiable::binding::framing_run_endpoint(
            root,
            &p,
            step,
            previous
                .last()
                .is_some_and(|s| s.phase.as_deref() == Some("EvaluationPending")),
        )?
    } else if p.identifiable.is_some()
        && !identifiable::binding::is(&p)
        && (step < 1024
            || (step == 1024
                && previous
                    .last()
                    .is_some_and(|s| s.phase.as_deref() == Some("EvaluationPending"))))
    {
        1024
    } else {
        p.config.max_steps
    };
    let cancel = std::sync::Arc::new(AtomicBool::new(false));
    let signal = cancel.clone();
    ctrlc::set_handler(move || signal.store(true, Ordering::Relaxed))
        .map_err(|e| bad(&e.to_string()))?;
    let mut control = recovery::RunControl::new(
        cancel,
        std::time::Duration::from_secs_f64(
            (p.evaluation.active_seconds as f64 - elapsed).min(p.evaluation.segment_seconds as f64),
        ),
        16 * 1024 * 1024,
    )?;
    control.set_call_limits(
        p.evaluation.generation_limit - generations,
        p.evaluation.teacher_limit - teachers,
    );
    let timer = Instant::now();
    write(
        &root.join(format!("segment-{index:04}-started.r3b")),
        &binary::record!({"schema":2,"policy":digest(&p)?,"checkpoint":input,"physical":file_hash(&input)?,"step":step,"prior_seconds":elapsed,"generation_limit":p.evaluation.generation_limit-generations,"teacher_limit":p.evaluation.teacher_limit-teachers}),
    )?;
    let r = if step == stop_after {
        // No trainer/Adam call on evaluation-only continuation, including final row publication.
        std::fs::create_dir(&output)?;
        let evaluated = evaluate_boundary(&p, root, &input, step, &mut control);
        if let Err(e) = &evaluated {
            control.classify_error(e);
        }
        let bytes = std::fs::read(&input)?;
        let saved = replica_v3::codec::publish_new(&output.join("final"), |f, _| {
            f.write_all(&bytes)?;
            Ok(())
        });
        if let Err(e) = &saved {
            control.classify_error(e);
        }
        let _ = control.seal_terminal();
        let mut receipt = control.receipt();
        receipt["reason"] = binary::record!(control.reason().unwrap_or(
            if step < p.config.max_steps { "TRAINING" } else { "BUDGET_REACHED" }
        ));
        receipt["checkpoint_saved"] = binary::record!(saved.is_ok());
        receipt["save_error"] = binary::record!(saved.as_ref().err().map(ToString::to_string));
        receipt["work_error"] = binary::record!(evaluated.as_ref().err().map(ToString::to_string));
        receipt["fresh_stop"] = binary::record!(evaluated.as_ref().ok().cloned().flatten());
        receipt["final_evaluation_complete"] = binary::record!(evaluated.is_ok());
        receipt["optimizer_calls"] = binary::record!(0);
        receipt["executed_input_tokens_including_uncommitted"] = binary::record!(0);
        receipt["executed_target_tokens_including_uncommitted"] = binary::record!(0);
        receipt["executed_padding_tokens"] = binary::record!(0);
        write(&output.join("train-control.r3b"), &receipt)?;
        control.stop_result().and(evaluated.map(|_| ())).and(saved)
    } else {
        super::train_with_policy(
            Run {
                checkpoint: &input,
                corpus: Some(&root.join("corpus.r3cor")),
                output: &output,
                resume: !previous.is_empty() || p.fork.is_some(),
                numeric_probe: false,
                config: p.config.clone(),
                stop_after: Some(stop_after),
                measure_rss: true,
                extend_steps: None,
                extend_microbatch: None,
                extend_sample_group_size: None,
                extend_curriculum_steps: None,
                extend_first_target_weight: None,
                extend_lr: None,
                extend_warmup: None,
                source_id: None,
                replace_corpus: false,
            },
            &mut control,
            Some((&p, root)),
        )
    };
    let receipt: binary::Value = read(&output.join("train-control.r3b"))?;
    let path = output.join("final");
    let (m, _) = checkpoint::metadata(&path)?;
    let s = m.training.ok_or_else(|| bad("missing state"))?;
    let stop = receipt["fresh_stop"]
        .as_str()
        .or_else(|| receipt["reason"].as_str())
        .ok_or_else(|| bad("missing stop"))?
        .to_string();
    let evaluation_pending =
        p.evaluation_due(s.step) && receipt["final_evaluation_complete"] != true;
    let clean_time = matches!(&r,Err(Error::Model(message)) if message=="TIME_BUDGET")
        && signal_segment_resumable(&receipt)
        && receipt["reason"] == "TIME_BUDGET"
        && receipt["save_error"].is_null()
        && receipt["observed_conditions"]
            .as_array()
            .is_some_and(|v| !v.is_empty() && v.iter().all(|x| x == "TIME_BUDGET"));
    let resume = (s.step < p.config.max_steps || evaluation_pending)
        && signal_segment_resumable(&receipt)
        && s.consumed_tokens < p.config.max_tokens
        && receipt["fresh_stop"].is_null()
        && receipt["save_error"].is_null()
        && receipt["token_budget_reached"] != true
        && (r.is_ok() || clean_time);
    let segment = Segment {
        schema: Some(2),
        start_hash: Some(file_hash(
            &root.join(format!("segment-{index:04}-started.r3b")),
        )?),
        control_hash: Some(file_hash(&output.join("train-control.r3b"))?),
        phase: Some(
            if !resume && r.is_err() && !clean_time {
                "Failed"
            } else if evaluation_pending {
                "EvaluationPending"
            } else if resume {
                "TrainingPending"
            } else {
                "Finished"
            }
            .into(),
        ),
        policy: digest(&p)?,
        checkpoint: path.strip_prefix(root).unwrap().to_string_lossy().into(),
        checkpoint_hash: file_hash(&path)?,
        step: s.step,
        input: s.consumed_tokens,
        target: s.target_tokens,
        elapsed: timer.elapsed().as_secs_f64(),
        generations: receipt["generation_calls"]
            .as_u64()
            .ok_or_else(|| bad("missing generation usage"))? as usize,
        teachers: receipt["teacher_calls"]
            .as_u64()
            .ok_or_else(|| bad("missing teacher usage"))? as usize,
        resume,
        stop,
    };
    publish_confirmed(
        &root.join(format!("segment-{index:04}-finished.r3b")),
        &segment,
    )?;
    println!(
        "FRESH_SEGMENT step={} input={} target={} generations={} teachers={} elapsed={} durable={} resume={} stop={}",
        segment.step,
        segment.input,
        segment.target,
        segment.generations,
        segment.teachers,
        segment.elapsed,
        segment.checkpoint,
        segment.resume,
        segment.stop
    );
    if clean_time { Ok(()) } else { r }
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
struct PanelResult {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    paired_both: Option<[usize; 8]>,
    step: usize,
    panel: String,
    model: String,
    dataset: String,
    exact: usize,
    total: usize,
    errors: usize,
    eos: usize,
    buckets: [usize; 8],
    denominators: [usize; 8],
    base4: usize,
    raw_hash: String,
    ce: Option<f64>,
    citation_exact: usize,
    field_exact: usize,
    invalid_utf8: usize,
    length_ended: usize,
    strata: BTreeMap<String, [usize; 2]>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    teachers_completed: Option<usize>,
}
fn score(rows: &[binary::Value], episodes: &[Episode], meta: &[Meta]) -> Result<PanelResult> {
    if rows.len() != episodes.len() || meta.len() != episodes.len() {
        return Err(bad("incomplete evaluation"));
    }
    let mut s = PanelResult {
        paired_both: None,
        step: 0,
        panel: String::new(),
        model: String::new(),
        dataset: String::new(),
        exact: 0,
        total: rows.len(),
        errors: 0,
        eos: 0,
        buckets: [0; 8],
        denominators: [0; 8],
        base4: 0,
        raw_hash: String::new(),
        ce: None,
        citation_exact: 0,
        field_exact: 0,
        invalid_utf8: 0,
        length_ended: 0,
        strata: BTreeMap::new(),
        teachers_completed: None,
    };
    let mut bases: BTreeMap<&str, (usize, usize)> = BTreeMap::new();
    for ((row, e), m) in rows.iter().zip(episodes).zip(meta) {
        if row["id"] != e.id
            || row["expected"] != e.answer
            || row["question"] != e.request.input
            || row["generated_evidence"] != binary::record!(e.request.evidence)
        {
            return Err(bad("raw input/content mismatch"));
        }
        let eos = row["finish_reason"] == "stop" && row["generation_completed"] == true;
        let error = !row["error"].is_null() || !eos;
        let exact = !error && row["actual"] == e.answer;
        if row["exact_match"] != exact {
            return Err(bad("raw score mismatch"));
        }
        s.exact += usize::from(exact);
        s.errors += usize::from(error);
        s.eos += usize::from(eos);
        s.denominators[m.bucket] += 1;
        s.buckets[m.bucket] += usize::from(exact);
        s.invalid_utf8 += usize::from(row["error_class"] == "strict_utf8");
        s.length_ended += usize::from(row["finish_reason"] == "length");
        if let Some(actual) = row["actual"].as_str() {
            s.citation_exact += usize::from(citations(actual).ok() == citations(&e.answer).ok());
            let field = |s: &str| s.split_once(" [event:").map_or(s, |(a, _)| a).to_string();
            s.field_exact += usize::from(field(actual) == field(&e.answer));
        }
        for key in [
            format!("template:{}", m.template),
            format!("records:{}", e.request.evidence.items.len()),
            format!(
                "entity_digits:{}",
                m.entities[0].chars().filter(char::is_ascii_digit).count()
            ),
            format!(
                "prompt_length_64_bin:{}",
                row["prompt_length"].as_u64().unwrap_or(0) / 64
            ),
        ] {
            let count = s.strata.entry(key).or_default();
            count[0] += usize::from(exact);
            count[1] += 1;
        }
        if let Some(context)=&m.query_context {
            let count=s.strata.entry(format!("query_context:{context}")).or_default();
            count[0]+=usize::from(exact);count[1]+=1;
        }
        let b = bases.entry(&m.base).or_default();
        b.0 += 1;
        b.1 += usize::from(exact);
    }
    s.base4 = bases.values().filter(|&&(n, k)| n == 4 && k == 4).count();
    if meta.iter().all(|m| m.base.starts_with("joint-binding-balanced-v1/") || m.base.starts_with("binding-learnability-v1/")) {
        let mut both = [0;8];
        for m in meta.iter().filter(|m|m.view==0) {
            if bases.get(m.base.as_str()) == Some(&(2,2)) { both[m.bucket]+=1; }
        }
        s.paired_both = Some(both);
    }
    Ok(s)
}
fn verify_generated(row: &binary::Value, tok: &ByteBpe) -> Result<()> {
    let raw = row["raw_tokens"]
        .as_array()
        .ok_or_else(|| bad("missing raw tokens"))?;
    let ids = raw
        .iter()
        .map(|x| {
            x.as_u64()
                .and_then(|n| u32::try_from(n).ok())
                .ok_or_else(|| bad("raw token"))
        })
        .collect::<Result<Vec<_>>>()?;
    if row["finish_reason"] == "stop" && ids.last() != Some(&EOS) {
        return Err(bad("EOS receipt mismatch"));
    }
    if row["row_version"] == 2 {
        if row["generation_completed"] == true && row["generation"].is_null() {
            return Err(bad("completed generation without Generated receipt"));
        }
        if row["generation_completed"] == true {
            let n = ids.len() - usize::from(ids.last() == Some(&EOS));
            if row["generation"]["tokens"] != binary::record!(&ids[..n])
                || row["generation"]["generated"] != ids.len()
                || row["generation"]["finish"] != row["finish_reason"]
            {
                return Err(bad("Generated/raw token mismatch"));
            }
            match tok.decode(&ids[..n]) {
                Ok(text) => {
                    if row["actual"] != text || !row["error"].is_null() {
                        return Err(bad("decoded text/error mismatch"));
                    }
                }
                Err(_) => {
                    if !row["actual"].is_null() || row["error"].is_null() {
                        return Err(bad("missing strict decode failure"));
                    }
                }
            }
        }
        let exact = row["generation_completed"] == true
            && row["error"].is_null()
            && row["finish_reason"] == "stop"
            && ids.last() == Some(&EOS)
            && row["actual"]
                .as_str()
                .is_some_and(|s| !s.is_empty() && row["expected"] == s);
        if row["exact_match"] != exact {
            return Err(bad("strict raw verdict"));
        }
    }
    if let Some(actual) = row["actual"].as_str() {
        let n = ids.len() - usize::from(ids.last() == Some(&EOS));
        if tok.decode(&ids[..n])? != actual {
            return Err(bad("raw text mismatch"));
        }
    }
    Ok(())
}
fn append_row(f: &mut std::fs::File, row: &binary::Value) -> Result<()> {
    binary::write_value_record(f, row)?;
    f.flush()?;
    f.sync_all()?;
    Ok(())
}
// Immutable prepared/resolved pairs: absence of a resolution is never evidence
// that a native call did not happen. Reuse is permitted only for proven no-call timeouts.
fn call_attempt(
    root: &Path,
    prefix: &str,
    kind: &str,
    binding: &binary::Value,
    e: &Episode,
    ordinal: usize,
    returned: Option<&binary::Value>,
) -> Result<Option<PathBuf>> {
    for attempt in 0..128 {
        let path = root.join(format!(
            "{prefix}-{kind}-{ordinal:04}-{attempt:03}-prepared.r3b"
        ));
        let identity = binary::record!({"schema":1,"binding":binding,"kind":kind,"case":digest(e)?,"ordinal":ordinal,"attempt":attempt});
        if !path.exists() {
            if returned.is_some() {
                return Err(bad("returned row without call resolution"));
            }
            return Ok(Some(path));
        }
        let old: binary::Value = read(&path)?;
        if old != identity {
            return Err(bad("call attempt identity mismatch"));
        }
        let resolved = path.with_file_name(
            path.file_name()
                .unwrap()
                .to_string_lossy()
                .replace("-prepared", "-resolved"),
        );
        if !resolved.exists() || pending_path(&resolved).exists() {
            return Err(bad(
                "entered or unreturned call usage UNKNOWN; retry blocked",
            ));
        }
        let r: binary::Value = read_confirmed(&resolved)?;
        if r["prepared"] != file_hash(&path)? {
            return Err(bad("call resolution binding"));
        }
        if r["state"] == "RETURNED" {
            let row = returned.ok_or_else(|| bad("returned call missing durable row"))?;
            if row["attempt"] != path.file_name().unwrap().to_string_lossy().as_ref()
                || r["row"] != digest(row)?
            {
                return Err(bad("returned call/raw mismatch"));
            }
            if root
                .join(format!(
                    "{prefix}-{kind}-{ordinal:04}-{:03}-prepared.r3b",
                    attempt + 1
                ))
                .exists()
            {
                return Err(bad("attempt after returned observation"));
            }
            return Ok(None);
        }
        if r["state"] != "NOT_INVOKED" || r["resumable"] != true || r["calls"] != 0 {
            return Err(bad("non-resumable call attempt"));
        }
    }
    Err(bad("call attempt limit"))
}
fn prepare_call(
    root: &Path,
    prefix: &str,
    kind: &str,
    binding: &binary::Value,
    e: &Episode,
    ordinal: usize,
) -> Result<PathBuf> {
    let path = call_attempt(root, prefix, kind, binding, e, ordinal, None)?
        .ok_or_else(|| bad("call already returned"))?;
    let name = path.file_name().unwrap().to_string_lossy();
    let attempt: usize = name
        .rsplit('-')
        .nth(1)
        .ok_or_else(|| bad("attempt name"))?
        .parse()
        .map_err(|_| bad("attempt number"))?;
    write(
        &path,
        &binary::record!({"schema":1,"binding":binding,"kind":kind,"case":digest(e)?,"ordinal":ordinal,"attempt":attempt}),
    )?;
    Ok(path)
}
fn resolve_call(
    path: &Path,
    row: Option<&binary::Value>,
    control: &recovery::RunControl,
) -> Result<()> {
    let c = control.receipt();
    let pure_time = c["observed_conditions"] == binary::record!(["TIME_BUDGET"]);
    let dest = path.with_file_name(
        path.file_name()
            .unwrap()
            .to_string_lossy()
            .replace("-prepared", "-resolved"),
    );
    let value = binary::record!({"prepared":file_hash(path)?,"state":if row.is_some(){"RETURNED"}else{"NOT_INVOKED"},"row":row.map(digest).transpose()?,"calls":usize::from(row.is_some()),"resumable":row.is_none()&&pure_time,"control":c});
    #[cfg(feature = "test-support")]
    if std::env::var("R3_FRESH_RESOLUTION_FAIL").as_deref() == Ok("1") {
        write(&pending_path(&dest), &value)?;
        return Err(std::io::Error::other("injected resolution sync failure").into());
    }
    publish_confirmed(&dest, &value)
}
#[cfg(feature="test-support")]
fn call_fixture(
    l: &checkpoint::Loaded,
    control: &mut recovery::RunControl,
    prefix: &str,
    kind: &str,
    ordinal: usize,
) {
    if l.model.config.hidden == 32 && l.model.config.layers == 2 {
        control.fixture_boundary = std::env::var("R3_FRESH_CALL_STOP").ok().and_then(|s| {
            s.strip_prefix(&format!("{prefix}/{kind}/{ordinal}/"))
                .map(str::to_owned)
        });
        if let Some(tokens) = control
            .fixture_boundary
            .as_deref()
            .and_then(|s| s.strip_prefix("native-timeout-"))
            .and_then(|s| s.parse().ok())
        {
            control.fixture_native_timeout(tokens);
        }
    }
}
fn teacher_prefix(
    root: &Path,
    prefix: &str,
    binding: &binary::Value,
    l: &checkpoint::Loaded,
    episodes: &[Episode],
    control: &mut recovery::RunControl,
) -> Result<Vec<binary::Value>> {
    teacher_prefix_until(root, prefix, binding, l, episodes, episodes.len(), control)
}
// A deliberate prefix retains the full manifest and immutable ordinal identity.
// It is not a timeout, and callers must not publish a complete lane for it.
fn teacher_prefix_until(
    root: &Path, prefix: &str, binding: &binary::Value, l: &checkpoint::Loaded,
    episodes: &[Episode], until: usize, control: &mut recovery::RunControl,
) -> Result<Vec<binary::Value>> {
    if (until == 0 && !episodes.is_empty()) || until > episodes.len() { return Err(bad("invalid teacher prefix limit")); }
    let raw = root.join(format!("{prefix}-teachers.r3rows"));
    let pending = root.join(format!("{prefix}-teacher-pending.r3b"));
    if pending.exists() {
        return Err(bad("unreturned teacher usage UNKNOWN; retry blocked"));
    }
    let mut entries = if raw.exists() {
        binary::read_value_records(&raw)?
    } else {
        vec![]
    };
    if entries.first().is_some_and(|v| v != binding) {
        return Err(bad("teacher binding"));
    }
    let rows = if entries.is_empty() {
        vec![]
    } else {
        entries.drain(1..).collect::<Vec<_>>()
    };
    if rows.len() > until {
        return Err(bad("teacher cursor beyond completed prefix"));
    }
    for (i, (r, e)) in rows.iter().zip(episodes).enumerate() {
        if binding["call_protocol"] == 1 || !r["attempt"].is_null() {
            call_attempt(root, prefix, "teacher", binding, e, i, Some(r))?;
        }
        if r["ordinal"] != i
            || r["id"] != e.id
            || r["case"] != digest(e)?
            || r["teacher"]["mean_nll"]
                .as_f64()
                .is_none_or(|v| !v.is_finite())
            || r["teacher"]["target_tokens_including_eos"]
                .as_u64()
                .is_none_or(|n| n == 0)
        {
            return Err(bad("teacher prefix integrity"));
        }
    }
    let mut f = std::fs::OpenOptions::new()
        .create_new(!raw.exists())
        .append(true)
        .open(&raw)?;
    if entries.is_empty() {
        append_row(&mut f, binding)?;
        std::fs::File::open(root)?.sync_all()?;
    }
    let mut rows = rows;
    for (i, e) in episodes.iter().enumerate().take(until).skip(rows.len()) {
        control.check("fresh_teacher_next")?;
        let attempt = prepare_call(root, prefix, "teacher", binding, e, i)?;
        #[cfg(feature = "test-support")]
        call_fixture(l, control, prefix, "teacher", i);
        let observed = if e.family.starts_with("foundation-orbit-v1/") || e.family.starts_with("learned-binding-expansion-v1/") {
            let foil = identifiable::binding::orbit_foil(e)?;
            recovery::fresh_teacher_with_foil(l, e, Some(&foil), control)
        } else { recovery::fresh_teacher(l, e, control) };
        let t = match observed {
            recovery::ObservedCall::NotInvoked(result) => {
                if let Err(ref error) = result {
                    control.classify_error(error);
                }
                resolve_call(&attempt, None, control)?;
                result?;
                return Err(bad("teacher not invoked without failure"));
            }
            recovery::ObservedCall::Returned(result) => result,
        };
        let value = match &t {
            Ok(v) => v.clone(),
            Err(error) => binary::record!({"error":error.to_string()}),
        };
        let row = binary::record!({"ordinal":i,"id":e.id,"case":digest(e)?,"teacher":value,"attempt":attempt.file_name().unwrap().to_string_lossy()});
        append_row(&mut f, &row)?;
        resolve_call(&attempt, Some(&row), control)?;
        t?;
        rows.push(row);
        control.check("fresh_teacher_durable")?;
    }
    Ok(rows)
}
fn teacher_ce(rows: &[binary::Value]) -> Result<f64> {
    let mut sum = 0.;
    let mut n = 0u64;
    for r in rows {
        let t = &r["teacher"];
        let k = t["target_tokens_including_eos"]
            .as_u64()
            .ok_or_else(|| bad("teacher count"))?;
        sum += t["mean_nll"].as_f64().ok_or_else(|| bad("teacher NLL"))? * k as f64;
        n += k;
    }
    if n == 0 || !sum.is_finite() {
        return Err(bad("incomplete teacher CE"));
    }
    Ok(sum / n as f64)
}
#[allow(clippy::too_many_arguments)] // One shared evaluator; caller owns the fixed cases and metadata.
fn evaluate_panel(
    p: &Plan,
    root: &Path,
    path: &Path,
    step: usize,
    name: &str,
    episodes: &[Episode],
    meta: &[Meta],
    control: &mut recovery::RunControl,
) -> Result<PanelResult> {
    if identifiable::binding::citation::word::is(p) || identifiable::binding::citation::instruction_bridge(p) || identifiable::binding::citation::retained_qa(p) || identifiable::binding::citation::fidelity(p) || identifiable::binding::citation::precision(p) {
        control.restrict_rss(12*1024*1024);
        control.check("fidelity_inference_rss")?;
    }
    let prefix = format!("eval-{step:04}-{name}");
    let raw = root.join(format!("{prefix}.r3rows"));
    let summary = root.join(format!("{prefix}.r3b"));
    let dataset = digest(&episodes)?;
    let l = checkpoint::load(path, Device::Cpu, false)?;
    let model = l.model.weight_hash()?;
    if l.manifest.training.as_ref().is_none_or(|s| {
        s.step != step || s.resume_binding.as_ref() != p.binding(s, &l.tokenizer).ok().as_ref()
    }) {
        return Err(bad("evaluation checkpoint step/policy"));
    }
    let binding = binary::record!({"call_protocol":1,"policy":digest(p)?,"step":step,"model":model,"dataset":dataset,"tokenizer":l.tokenizer.id(),"panel":name,"planned":episodes.len(),"decoding":"normal-greedy-strict-utf8-eos"});
    let header = binary::record!({"binding":binding,"checkpoint":path,"physical":file_hash(path)?});
    let mut existing = if raw.exists() {
        binary::read_value_records(&raw)?
    } else {
        vec![]
    };
    if let Some(old) = existing.first() {
        let original = Path::new(
            old["checkpoint"]
                .as_str()
                .ok_or_else(|| bad("missing evaluation checkpoint"))?,
        );
        if old["binding"] != binding
            || old["physical"] != file_hash(original)?
            || checkpoint::metadata(original)?.0.model_content_digest
                != l.manifest.model_content_digest
        {
            return Err(bad("evaluation header mismatch"));
        }
    }
    let rows = if existing.is_empty() {
        vec![]
    } else {
        existing.drain(1..).collect::<Vec<_>>()
    };
    if rows.len() > episodes.len() {
        return Err(bad("extra rows"));
    }
    for (i, row) in rows.iter().enumerate() {
        if binding["call_protocol"] == 1 || !row["attempt"].is_null() {
            call_attempt(
                root,
                &prefix,
                "generation",
                &binding,
                &episodes[i],
                i,
                Some(row),
            )?;
        }
        verify_generated(row, &l.tokenizer)?;
    }
    score(&rows, &episodes[..rows.len()], &meta[..rows.len()])?;
    if pending_path(&summary).exists() {
        return Err(bad("panel publication unconfirmed"));
    }
    let mut f = std::fs::OpenOptions::new()
        .create_new(!raw.exists())
        .append(true)
        .open(&raw)?;
    if existing.is_empty() {
        append_row(&mut f, &header)?;
        std::fs::File::open(root)?.sync_all()?;
    }
    let mut rows = rows;
    let pending = root.join(format!("{prefix}-pending.r3b"));
    if pending.exists() {
        return Err(bad("unreturned generation usage UNKNOWN; do not retry"));
    }
    let mut teachers = teacher_prefix(
        root,
        &prefix,
        &binding,
        &l,
        &episodes[..rows.len()],
        control,
    )?;
    for e in episodes.iter().skip(rows.len()) {
        control.check("fresh_panel_next")?;
        let attempt = prepare_call(root, &prefix, "generation", &binding, e, rows.len())?;
        #[cfg(feature = "test-support")]
        call_fixture(&l, control, &prefix, "generation", rows.len());
        #[cfg(feature = "test-support")]
        {
            control.fixture_post_generation_deadline = p.tiny
                && step == p.config.max_steps
                && name == "train64"
                && std::env::var("R3_FRESH_TEST_STOP")
                    .is_ok_and(|v| v == format!("final-row-{}", rows.len() + 1));
            control.fixture_post_generation_deadline |= p.tiny
                && step == p.config.max_steps
                && std::env::var("R3_FRESH_TEST_STOP")
                    .is_ok_and(|v| v == format!("final-{name}-row-{}", rows.len() + 1));
        }
        let mut row = match recovery::observe_generation(&l, e, &e.request, control, false) {
            recovery::ObservedCall::Returned(row) => row,
            recovery::ObservedCall::NotInvoked(row) => {
                resolve_call(&attempt, None, control)?;
                control.stop_result()?;
                return Err(bad(&format!("generation not invoked: {}", row["error"])));
            }
        };
        row["attempt"] = binary::record!(attempt.file_name().unwrap().to_string_lossy());
        append_row(&mut f, &row)?;
        resolve_call(&attempt, Some(&row), control)?;
        verify_generated(&row, &l.tokenizer)?;
        rows.push(row);
        control.check("fresh_panel_row_durable")?;
        teachers = teacher_prefix(
            root,
            &prefix,
            &binding,
            &l,
            &episodes[..rows.len()],
            control,
        )?;
    }
    let mut s = score(&rows, episodes, meta)?;
    s.step = step;
    s.panel = name.into();
    s.model = model;
    s.dataset = dataset;
    s.raw_hash = file_hash(&raw)?;
    s.ce = Some(teacher_ce(&teachers)?);
    s.teachers_completed = Some(teachers.len());
    #[cfg(feature = "test-support")]
    if p.tiny
        && step == p.config.max_steps
        && std::env::var("R3_FRESH_TEST_STOP").is_ok_and(|v| v == format!("final-{name}-summary"))
    {
        control.fixture_deadline();
        control.check("fresh_summary_pending")?;
    }
    if summary.exists() {
        let old: PanelResult = read_confirmed(&summary)?;
        if old != s {
            return Err(bad("evaluation summary disagreement"));
        }
        return Ok(s);
    }
    publish_confirmed(&summary, &s)?;
    println!(
        "FRESH_EVAL step={step} panel={name} exact={}/{} errors={} eos={} buckets={:?} base4={} model={}",
        s.exact, s.total, s.errors, s.eos, s.buckets, s.base4, s.model
    );
    Ok(s)
}
pub(super) fn evaluate_boundary(
    p: &Plan,
    root: &Path,
    path: &Path,
    step: usize,
    control: &mut recovery::RunControl,
) -> Result<Option<String>> {
    if !p.evaluation_due(step) {
        return Ok(None);
    }
    if p.paired.is_some() {
        return paired_evaluate(p, root, path, step, control);
    }
    if p.identifiable.is_some() {
        return identifiable::evaluate(p, root, path, step, control);
    }
    let (_, train, dev) = p.training_corpus(&root.join("corpus.r3cor"))?;
    let transfer = verified_corpus(&root.join("transfer.r3cor"), &p.transfer)?.validation;
    let (tm, dm, xm) = verified_metadata(root, p)?;
    let subset = |es: &[Episode], ms: &[Meta]| {
        let ids: Vec<_> = (0..8)
            .flat_map(|b| {
                ms.iter()
                    .enumerate()
                    .filter(move |(_, m)| m.bucket == b)
                    .take(p.evaluation.fixture_per_bucket.unwrap_or(8))
                    .map(|(i, _)| i)
            })
            .collect();
        (
            ids.iter().map(|&i| es[i].clone()).collect::<Vec<_>>(),
            ids.iter().map(|&i| ms[i].clone()).collect::<Vec<_>>(),
        )
    };
    let (probe, pm) = subset(&train, &tm);
    // Train generation is fixed64 at0/1024/2048/4096; auxiliary probes never select labels.
    if p.evaluation.train_steps.contains(&step) {
        evaluate_panel(p, root, path, step, "train64", &probe, &pm, control)?;
    }
    let mut screen = None;
    if p.evaluation.screen_steps.contains(&step) {
        let (es, ms) = subset(&dev, &dm);
        screen = Some(evaluate_panel(
            p, root, path, step, "screen64", &es, &ms, control,
        )?);
    }
    if p.evaluation.teacher_steps.contains(&step) {
        let l = checkpoint::load(path, Device::Cpu, false)?;
        let teacher = root.join(format!("teacher-{step:04}.r3b"));
        let binding = binary::record!({"call_protocol":1,"policy":digest(p)?,"step":step,"model":l.model.weight_hash()?,"dataset":digest(&probe)?});
        if teacher.exists() {
            let old: binary::Value = read_confirmed(&teacher)?;
            if old["binding"] != binding {
                return Err(bad("fixed teacher binding"));
            }
        } else {
            let (loss, calls, reused) = if p.evaluation.train_steps.contains(&step) {
                let panel: PanelResult =
                    read_confirmed(&root.join(format!("eval-{step:04}-train64.r3b")))?;
                (
                    panel
                        .ce
                        .ok_or_else(|| bad("required train teacher incomplete"))?,
                    0,
                    true,
                )
            } else {
                let ts = teacher_prefix(
                    root,
                    &format!("teacher-{step:04}"),
                    &binding,
                    &l,
                    &probe,
                    control,
                )?;
                (teacher_ce(&ts)?, ts.len(), false)
            };
            publish_confirmed(
                &teacher,
                &binary::record!({"binding":binding,"step":step,"model":l.model.weight_hash()?,"panel":"train64-fixed","dataset":digest(&probe)?,"ce":loss,"teacher_calls":calls,"reused_complete_train_panel":reused}),
            )?;
        }
    }
    let mut full = None;
    if p.evaluation.primary_steps.contains(&step) {
        let (dev, dm) = if p.tiny {
            subset(&dev, &dm)
        } else {
            (dev.clone(), dm.clone())
        };
        full = Some(evaluate_panel(
            p, root, path, step, "dev512", &dev, &dm, control,
        )?);
    }
    if p.fork.is_some()
        && let Some(full) = &full
    {
        let prefix = format!("eval-{step:04}-dev512");
        let rows = binary::read_value_records(&root.join(format!("{prefix}.r3rows")))?;
        let tr = binary::read_value_records(&root.join(format!("{prefix}-teachers.r3rows")))?;
        let (es, ms) = subset(&dev, &dm);
        let mut selected = vec![];
        let mut ts = vec![];
        for e in &es {
            let n = rows[1..]
                .iter()
                .position(|r| r["id"] == e.id)
                .ok_or_else(|| bad("screen subset missing"))?;
            selected.push(rows[n + 1].clone());
            ts.push(tr[n + 1].clone());
        }
        let mut r = score(&selected, &es, &ms)?;
        r.step = step;
        r.panel = "screen64".into();
        r.model = full.model.clone();
        r.dataset = digest(&es)?;
        r.raw_hash = full.raw_hash.clone();
        r.ce = Some(teacher_ce(&ts)?);
        r.teachers_completed = Some(ts.len());
        let dest = root.join(format!("eval-{step:04}-screen64.r3b"));
        if dest.exists() {
            let old: PanelResult = read_confirmed(&dest)?;
            if old != r {
                return Err(bad("reused screen summary mismatch"));
            }
        } else {
            publish_confirmed(&dest, &r)?;
        }
        screen = Some(r);
    }
    if p.evaluation.transfer_steps.contains(&step) {
        let (transfer, xm) = if p.tiny {
            subset(&transfer, &xm)
        } else {
            (transfer.clone(), xm.clone())
        };
        let transfer = evaluate_panel(p, root, path, step, "transfer128", &transfer, &xm, control)?;
        let f = full.as_ref().unwrap();
        if step == p.config.max_steps
            && p.fork
                .as_ref()
                .is_some_and(|f| matches!(f.arm.as_str(), "C-KEEP" | "S-SELECT"))
        {
            let (es, ms) = selector_panel(&dev, &dm, p.tiny)?;
            evaluate_panel(p, root, path, step, "selector192", &es, &ms, control)?;
        }
        if (p.fork.is_none() || step == p.config.max_steps)
            && f.exact >= p.evaluation.primary_min
            && f.buckets.iter().all(|&x| x >= p.evaluation.bucket_min)
            && f.errors == 0
            && transfer.exact >= p.evaluation.transfer_min
            && transfer.errors == 0
        {
            return Ok(Some("DEVELOPMENT_PASS_FINAL_NOT_RUN".into()));
        }
    }
    if let Some(f) = &p.fork {
        if matches!(f.arm.as_str(), "C-KEEP" | "S-SELECT")
            && !p.tiny
            && full
                .as_ref()
                .is_some_and(|panel| panel.exact + 64 <= 425 || panel.errors >= 8)
        {
            return Ok(Some("QUALITY_REGRESSION_PRIMARY".into()));
        }
        if let Some(current) = screen {
            let parent: binary::Value = read(&f.study.join("parent-audit.r3b"))?;
            let baseline: PanelResult = binary::from_value(
                parent["panels"][format!("eval-{:04}-screen64", f.origin_step).as_str()].clone(),
            )
            .or_else(|_| {
                binary::from_value(
                    parent["panels"][format!("eval-{:04}-dev512", f.origin_step).as_str()].clone(),
                )
            })?;
            let mut best = baseline.exact;
            let mut ce = baseline.ce;
            let mut streak = 0;
            let mut steps = p.evaluation.screen_steps.clone();
            steps.extend(&p.evaluation.primary_steps);
            steps.sort();
            steps.dedup();
            for n in steps.into_iter().filter(|&n| n <= step) {
                let r: PanelResult =
                    read_confirmed(&root.join(format!("eval-{n:04}-screen64.r3b")))?;
                if r.exact + 24 <= best || r.errors >= 8 {
                    return Ok(Some("QUALITY_REGRESSION_SEVERE".into()));
                }
                if let Some((a, b)) = r.ce.zip(ce) {
                    if r.exact + 12 <= best && a >= b * 1.2 {
                        streak += 1;
                    } else {
                        streak = 0;
                    }
                }
                if r.exact > best {
                    best = r.exact;
                    ce = r.ce;
                }
            }
            if streak >= 2 {
                return Ok(Some("QUALITY_REGRESSION".into()));
            }
            println!(
                "FORK_SCREEN step={step} exact={}/{} errors={} complete_teacher={:?}",
                current.exact, current.total, current.errors, current.teachers_completed
            );
        }
        return Ok(None);
    }
    if step == 1024 && full.as_ref().is_some_and(|f| f.exact == 0) {
        let t: PanelResult = read(&root.join("eval-1024-train64.r3b"))?;
        let a: binary::Value = read(&root.join("teacher-0000.r3b"))?;
        let b: binary::Value = read(&root.join("teacher-1024.r3b"))?;
        if t.exact == 0 && b["ce"].as_f64() >= a["ce"].as_f64() {
            return Ok(Some("NO_LEARNING_SIGNAL".into()));
        }
    }
    if step == 2048 {
        let f = full.as_ref().unwrap();
        let prev: PanelResult = read(&root.join("eval-1024-dev512.r3b"))?;
        if f.buckets.iter().filter(|&&x| x == 0).count() >= 4 && f.exact <= prev.exact {
            return Ok(Some("NO_TRANSFER_SIGNAL".into()));
        }
    }
    if let Some(s) = screen {
        if step >= 1024 {
            let mut streak = 0;
            let mut best = 0;
            let mut best_ce = None;
            for n in [0, 128, 512, 1024, 2048, 3072, 4096]
                .into_iter()
                .filter(|&n| n <= step)
            {
                let f = root.join(format!("eval-{n:04}-screen64.r3b"));
                if !f.exists() {
                    continue;
                }
                let r: PanelResult = read(&f)?;
                if best >= 32
                    && r.exact + 12 <= best
                    && r.ce.zip(best_ce).is_some_and(|(a, b)| a >= b * 1.2)
                {
                    streak += 1;
                } else {
                    streak = 0;
                }
                if r.exact > best {
                    best = r.exact;
                    best_ce = r.ce;
                }
            }
            if streak >= 2 {
                return Ok(Some("QUALITY_REGRESSION".into()));
            }
        }
        let _ = s;
    }
    Ok(None)
}
fn report(root: &Path) -> Result<()> {
    let p = plan_read(root)?;
    let h = history(root, &p)?;
    for s in &h {
        println!(
            "segment step={} input={} target={} gen={} teacher={} resume={} stop={} durable={}",
            s.step, s.input, s.target, s.generations, s.teachers, s.resume, s.stop, s.checkpoint
        );
    }
    println!(
        "SMALL_UPDATES={} GENERATIONS={} TEACHERS={} ACTIVE_SECONDS={} S4=NOT_ACCEPTED GOAL1_READY=false",
        if p.tiny {
            0
        } else {
            h.last().map_or(0, |s| s.step - p.origin_step())
        },
        h.iter().map(|s| s.generations).sum::<usize>(),
        h.iter().map(|s| s.teachers).sum::<usize>(),
        h.iter().map(|s| s.elapsed).sum::<f64>()
    );
    Ok(())
}
fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}
fn optimizer_hash(m: &BTreeMap<String, Tensor>) -> Result<String> {
    use sha2::{Digest, Sha256};
    let mut h = Sha256::new();
    for (name, t) in m {
        h.update(name.as_bytes());
        for x in t.flatten_all()?.to_vec1::<f32>()? {
            h.update(x.to_bits().to_le_bytes());
        }
    }
    Ok(hex(&h.finalize()))
}
fn subset(es: &[Episode], ms: &[Meta], per_bucket: usize) -> (Vec<Episode>, Vec<Meta>) {
    let ids: Vec<_> = (0..8)
        .flat_map(|b| {
            ms.iter()
                .enumerate()
                .filter(move |(_, m)| m.bucket == b)
                .take(per_bucket)
                .map(|(i, _)| i)
        })
        .collect();
    (
        ids.iter().map(|&i| es[i].clone()).collect(),
        ids.iter().map(|&i| ms[i].clone()).collect(),
    )
}
fn inventory(root: &Path) -> Result<Vec<(PathBuf, String, u64)>> {
    fn walk(root: &Path, p: &Path, out: &mut Vec<(PathBuf, String, u64)>) -> Result<()> {
        for e in std::fs::read_dir(p)? {
            let e = e?;
            if e.file_type()?.is_dir() {
                walk(root, &e.path(), out)?;
            } else {
                out.push((
                    e.path()
                        .strip_prefix(root)
                        .map_err(|_| bad("inventory path"))?
                        .into(),
                    file_hash(&e.path())?,
                    e.metadata()?.len(),
                ));
            }
        }
        Ok(())
    }
    let mut out = vec![];
    walk(root, root, &mut out)?;
    out.sort_by(|a, b| a.0.cmp(&b.0));
    Ok(out)
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Study {
    schema: u32,
    source: String,
    binary: String,
    parent: PathBuf,
    parent_plan: String,
    parent_checkpoint: PathBuf,
    parent_file: String,
    parent_model: String,
    parent_adam: String,
    parent_state: String,
    parent_step: usize,
    tokenizer: String,
    updates: usize,
    tiny: bool,
    inventory: Vec<(PathBuf, String, u64)>,
    diagnostic_hash: String,
    parent_audit_hash: String,
    tie_break: Vec<String>,
}
fn fork_evaluation(origin: usize, updates: usize, tiny: bool) -> EvaluationPolicy {
    let offsets = if tiny { vec![] } else { vec![256, 512, 1536] };
    let endpoints = if tiny {
        vec![origin + updates]
    } else {
        vec![origin + 1024, origin + updates]
    };
    let mut teacher_steps: Vec<_> = offsets
        .iter()
        .map(|n| origin + n)
        .chain(endpoints.iter().copied())
        .collect();
    teacher_steps.sort();
    EvaluationPolicy {
        screen_steps: offsets.into_iter().map(|n| origin + n).collect(),
        primary_steps: endpoints.clone(),
        transfer_steps: endpoints.clone(),
        train_steps: endpoints.clone(),
        teacher_steps,
        fixture_per_bucket: tiny.then_some(1),
        active_seconds: 14400,
        ..EvaluationPolicy::default()
    }
}
// Pure reader for both explicitly historical and current producer records. This
// never relaxes plan_read's current-source check used to authorize learning.
fn audit_panels(root: &Path, p: &Plan) -> Result<(usize, BTreeMap<String, PanelResult>)> {
    audit_panels_through(root, p, p.config.max_steps)
}
fn audit_panels_through(
    root: &Path,
    p: &Plan,
    last: usize,
) -> Result<(usize, BTreeMap<String, PanelResult>)> {
    audit_panels_range(root, p, 0, last)
}
#[allow(clippy::too_many_arguments)] // One existing panel identity and owned expected cases.
fn audit_panel(
    root: &Path,
    p: &Plan,
    step: usize,
    name: &str,
    es: &[Episode],
    ms: &[Meta],
    tok: &ByteBpe,
) -> Result<PanelResult> {
    let key = format!("eval-{step:04}-{name}");
    let raw = root.join(format!("{key}.r3rows"));
    let rows = binary::read_value_records(&raw)?;
    let header = rows.first().ok_or_else(|| bad("empty raw"))?;
    let cp = Path::new(
        header["checkpoint"]
            .as_str()
            .ok_or_else(|| bad("raw checkpoint"))?,
    );
    let l = checkpoint::load(cp, Device::Cpu, false)?;
    let state = l
        .manifest
        .training
        .as_ref()
        .ok_or_else(|| bad("raw state"))?;
    if header["physical"] != file_hash(cp)?
        || header["binding"]["policy"] != digest(p)?
        || header["binding"]["dataset"] != digest(&es)?
        || header["binding"]["step"] != step
        || header["binding"]["planned"] != es.len()
        || header["binding"]["model"] != l.model.weight_hash()?
        || state.step != step
        || state.resume_binding.as_ref() != Some(&p.binding(state, tok)?)
    {
        return Err(bad("raw lineage/dataset mismatch"));
    }
    for (i, (r, e)) in rows[1..].iter().zip(es).enumerate() {
        if header["binding"]["call_protocol"] == 1 || !r["attempt"].is_null() {
            call_attempt(root, &key, "generation", &header["binding"], e, i, Some(r))?;
        }
        verify_generated(r, tok)?;
        let prompt = tok.prepare_with_framing(
            &e.request,
            p.framing(),
            l.model.config.context as u32,
            &l.model.config.id()?,
        )?;
        if r["native_prompt_digest"] != prompt.token_digest {
            return Err(bad("raw actual framing mismatch"));
        }
        if (identifiable::binding::citation::is(p) || identifiable::binding::is_framing(p) || identifiable::binding::is_signal(p) || identifiable::binding::is_expansion(p) || identifiable::binding::is_consolidation(p)) && r["framing"] != p.framing().id() {
            return Err(bad("raw framing descriptor mismatch"));
        }
    }
    let mut s = score(&rows[1..], es, ms)?;
    s.step = step;
    s.panel = name.into();
    s.model = l.model.weight_hash()?;
    s.dataset = digest(&es)?;
    s.raw_hash = file_hash(&raw)?;
    let teacher_rows = if p.schema == Some(2) {
        let t = binary::read_value_records(&root.join(format!("{key}-teachers.r3rows")))?;
        if t.len() != es.len() + 1 || t[0] != header["binding"] {
            return Err(bad("teacher complete binding"));
        }
        for (i, (r, e)) in t[1..].iter().zip(es).enumerate() {
            if header["binding"]["call_protocol"] == 1 || !r["attempt"].is_null() {
                call_attempt(root, &key, "teacher", &header["binding"], e, i, Some(r))?;
            }
            if r["id"] != e.id || r["ordinal"] != i || r["case"] != digest(e)? {
                return Err(bad("teacher case"));
            }
            if (identifiable::binding::citation::is(p) || identifiable::binding::is_framing(p) || identifiable::binding::is_signal(p) || identifiable::binding::is_expansion(p) || identifiable::binding::is_consolidation(p))
                && r["teacher"]["training_prompt_matches_generation"] != true
            {
                return Err(bad("teacher actual framing mismatch"));
            }
        }
        s.teachers_completed = Some(es.len());
        t[1..].to_vec()
    } else {
        rows[1..]
            .iter()
            .map(|r| binary::record!({"teacher":r["teacher_forced_diagnostic_after_generation"]}))
            .collect()
    };
    s.ce = Some(teacher_ce(&teacher_rows)?);
    let saved: PanelResult = if p.schema == Some(2) {
        read_confirmed(&root.join(format!("{key}.r3b")))?
    } else {
        read(&root.join(format!("{key}.r3b")))?
    };
    if s != saved {
        return Err(bad("raw independently recomputed summary mismatch"));
    }

    Ok(s)
}
fn audit_panels_range(
    root: &Path,
    p: &Plan,
    first: usize,
    last: usize,
) -> Result<(usize, BTreeMap<String, PanelResult>)> {
    if identifiable::binding::is(p) { return identifiable::binding::audit(root, p, first, last); }
    let (_, tr, dv) = p.training_corpus(&root.join("corpus.r3cor"))?;
    let tx = verified_corpus(&root.join("transfer.r3cor"), &p.transfer)?.validation;
    let (tm, dm, xm) = verified_metadata(root, p)?;
    let tok = ByteBpe::load(&root.join("tokenizer.r3b"))?;
    if tok.id() != p.tokenizer {
        return Err(bad("parent tokenizer"));
    }
    let mut count = 0;
    let mut results = BTreeMap::new();
    for (steps, name, es, ms, sub) in [
        (&p.evaluation.train_steps, "train64", &tr, &tm, true),
        (&p.evaluation.screen_steps, "screen64", &dv, &dm, true),
        (&p.evaluation.primary_steps, "dev512", &dv, &dm, p.tiny),
        (
            &p.evaluation.transfer_steps,
            "transfer128",
            &tx,
            &xm,
            p.tiny,
        ),
    ] {
        for &step in steps.iter().filter(|&&step| step >= first && step <= last) {
        let (es, ms) = if sub {
                subset(es, ms, if p.identifiable.is_some() && step==0 {4}else{p.evaluation.fixture_per_bucket.unwrap_or(8)})
            } else {
                (es.clone(), ms.clone())
            };
            let key = format!("eval-{step:04}-{name}");
            let s = audit_panel(root, p, step, name, &es, &ms, &tok)?;
            count += es.len();
            results.insert(key, s);
        }
    }
    if p.schema == Some(2) {
        for &step in p
            .evaluation
            .teacher_steps
            .iter()
            .filter(|&&s| s >= first && s <= last)
        {
            let key = format!("teacher-{step:04}");
            let summary: binary::Value = read_confirmed(&root.join(format!("{key}.r3b")))?;
            let (probe, _) = subset(&tr, &tm, p.evaluation.fixture_per_bucket.unwrap_or(8));
            if summary["binding"]["policy"] != digest(p)?
                || summary["binding"]["step"] != step
                || summary["binding"]["dataset"] != digest(&probe)?
            {
                return Err(bad("teacher obligation binding"));
            }
            if results
                .values()
                .filter(|r| r.step == step)
                .any(|r| summary["binding"]["model"] != r.model)
            {
                return Err(bad("fixed teacher differs from panel model"));
            }
            let ce = if p.evaluation.train_steps.contains(&step) {
                results
                    .get(&format!("eval-{step:04}-train64"))
                    .and_then(|s| s.ce)
                    .ok_or_else(|| bad("missing required panel teacher"))?
            } else {
                let ts = binary::read_value_records(&root.join(format!("{key}-teachers.r3rows")))?;
                if ts.len() != probe.len() + 1 || ts[0] != summary["binding"] {
                    return Err(bad("incomplete fixed teacher"));
                }
                for (i, (r, e)) in ts[1..].iter().zip(&probe).enumerate() {
                    if r["ordinal"] != i || r["id"] != e.id || r["case"] != digest(e)? {
                        return Err(bad("fixed teacher case"));
                    }
                    if ts[0]["call_protocol"] == 1 || !r["attempt"].is_null() {
                        call_attempt(root, &key, "teacher", &ts[0], e, i, Some(r))?;
                    }
                }
                teacher_ce(&ts[1..])?
            };
            if summary["ce"] != binary::record!(ce) {
                return Err(bad("required teacher summary"));
            }
        }
    }
    if first <= p.config.max_steps
        && last >= p.config.max_steps
        && p.fork
            .as_ref()
            .is_some_and(|f| matches!(f.arm.as_str(), "C-KEEP" | "S-SELECT"))
    {
        let (es, ms) = selector_panel(&dv, &dm, p.tiny)?;
        let s = audit_panel(root, p, p.config.max_steps, "selector192", &es, &ms, &tok)?;
        count += es.len();
        results.insert(format!("eval-{:04}-selector192", p.config.max_steps), s);
    }
    if p.paired.is_some() {
        for &step in p.evaluation.screen_steps.iter().chain(&p.evaluation.primary_steps)
            .filter(|&&n| n >= first && n <= last) {
            let (name, es, ms) = paired_flip_panel(p, root, step, &dv, &dm)?;
            let s = audit_panel(root, p, step, name, &es, &ms, &tok)?;
            count += es.len();
            results.insert(format!("eval-{step:04}-{name}"), s);
        }
    }
    Ok((count, results))
}
fn changed_question(e: &Episode, suffix: &str) -> Result<Episode> {
    let (entity, context, intent) = question_intent(&e.request.input)?;
    let mut out = e.clone();
    out.request.input = format!("{entity} {context} {suffix}");
    if question_intent(&out.request.input)?.2 != intent
        || resolve(&e.request)? != e.answer
        || resolve(&out.request)? != e.answer
    {
        return Err(bad("question-only meaning changed"));
    }
    let mut check = out.clone();
    check.request.input = e.request.input.clone();
    if digest(&check)? != digest(e)? {
        return Err(bad("non-question change"));
    }
    Ok(out)
}
fn validate_framed(es: &[Episode], tok: &ByteBpe) -> Result<()> {
    let seq = samples(es, tok, 512)?;
    let mut seen = HashMap::new();
    for (e, s) in es.iter().zip(seq) {
        let p = tok.prepare(&e.request, 2048, "input-check")?;
        if s.tokens.len() > 513
            || p.provided.len() != e.request.evidence.items.len()
            || !p.excluded.is_empty()
        {
            return Err(bad("question length/evidence"));
        }
        if seen
            .insert(s.tokens[..s.response_start].to_vec(), e.answer.clone())
            .is_some_and(|a| a != e.answer)
        {
            return Err(bad("conflicting framed targets"));
        }
    }
    Ok(())
}
fn verify_seen_value_swap(before: &Episode, after: &Episode) -> Result<()> {
    let a=&before.request.evidence.items; let b=&after.request.evidence.items;
    if a.len()!=2 || b.len()!=2 || before.family!=after.family || before.category!=after.category
        || before.binding!=after.binding || before.sequence!=after.sequence {
        return Err(bad("seen value swap scene/count"));
    }
    for i in 0..2 {
        let (entity,context,value)=parsed_record(&a[i])?;
        let (new_entity,new_context,new_value)=parsed_record(&b[i])?;
        if entity!=new_entity || context!=new_context || value==new_value
            || new_value!=parsed_record(&a[1-i])?.2 {
            return Err(bad("seen value swap must exchange different values only"));
        }
    }
    let mut restored=after.request.clone();
    restored.request_id=before.request.request_id.clone();
    for (record,old) in restored.evidence.items.iter_mut().zip(a) {
        record.original_excerpt=old.original_excerpt.clone();
    }
    if digest(&restored)?!=digest(&before.request)? || resolve(&before.request)?!=before.answer
        || resolve(&after.request)?!=after.answer || before.answer==after.answer
        || citations(&before.answer)?!=citations(&after.answer)? {
        return Err(bad("seen value swap changed another input field or label"));
    }
    Ok(())
}
fn flip_selection(e: &Episode, m: &Meta) -> Result<(Episode, Meta)> {
    if !(2..=4).contains(&m.bucket) {
        return Err(bad("selector bucket"));
    }
    let mut out = e.clone();
    let mut meta = m.clone();
    let (entity, context, intent) = question_intent(&e.request.input)?;
    if intent != Intent::Current || e.request.evidence.items.len() != 2 {
        return Err(bad("selector grammar/count"));
    }
    let items = &e.request.evidence.items;
    if items[0].event_id == items[1].event_id
        || items.iter().any(|r| {
            r.version_status != "current"
                || r.observed_at.is_none()
                || r.observed_at.unwrap() > r.recorded_at
        })
    {
        return Err(bad("selector status/id/time"));
    }
    let (a, b) = (parsed_record(&items[0])?, parsed_record(&items[1])?);
    let suffix = e
        .request
        .input
        .splitn(3, ' ')
        .nth(2)
        .ok_or_else(|| bad("selector suffix"))?;
    match m.bucket {
        2 => {
            if a.0 == b.0 || a.1 != b.1 || a.1 != context {
                return Err(bad("entity selector fields"));
            }
            let other = if a.0 == entity {
                b.0
            } else if b.0 == entity {
                a.0
            } else {
                return Err(bad("query entity absent"));
            };
            out.request.input = format!("{other} {context} {suffix}");
        }
        3 => {
            if a.0 != b.0 || a.0 != entity || a.1 == b.1 {
                return Err(bad("context selector fields"));
            }
            let other = if a.1 == context {
                b.1
            } else if b.1 == context {
                a.1
            } else {
                return Err(bad("query context absent"));
            };
            out.request.input = format!("{entity} {other} {suffix}");
        }
        4 => {
            if a.0 != entity
                || b.0 != entity
                || a.1 != context
                || b.1 != context
                || items[0].observed_at == items[1].observed_at
            {
                return Err(bad("time selector fields"));
            }
            out.request.evidence.items[0].observed_at = items[1].observed_at;
            out.request.evidence.items[1].observed_at = items[0].observed_at;
            if out
                .request
                .evidence
                .items
                .iter()
                .any(|r| r.observed_at.unwrap() > r.recorded_at)
            {
                return Err(bad("swapped observation in future"));
            }
        }
        _ => unreachable!(),
    }
    // Request-only resolver supplies targets. An independent field/time selection
    // verifies the chosen citation; original expected is never a selector input.
    out.answer = resolve(&out.request)?;
    let (q, c, _) = question_intent(&out.request.input)?;
    let mut matching = out
        .request
        .evidence
        .items
        .iter()
        .filter(|r| parsed_record(r).is_ok_and(|(n, ctx, _)| n == q && ctx == c))
        .collect::<Vec<_>>();
    matching.sort_by_key(|r| r.observed_at);
    let selected = matching.last().ok_or_else(|| bad("no selected evidence"))?;
    let expected = format!(
        "{}입니다. [event:{}]",
        parsed_record(selected)?.2,
        selected.event_id
    );
    if out.answer != expected
        || citations(&out.answer)? == citations(&resolve(&e.request)?)?
        || resolve(&e.request)? != e.answer
    {
        return Err(bad("independent selector label"));
    }
    out.id = format!("{}/selector", e.id);
    meta.source_id = Some(e.id.clone());
    meta.id = out.id.clone();
    meta.query_context = Some(c.into());
    meta.entities[0] = q.into();
    Ok((out, meta))
}
fn selector_panel(es: &[Episode], ms: &[Meta], tiny: bool) -> Result<(Vec<Episode>, Vec<Meta>)> {
    let mut out = vec![];
    let mut metadata = vec![];
    for (e, m) in es
        .iter()
        .zip(ms)
        .filter(|(_, m)| (2..=4).contains(&m.bucket))
    {
        if tiny && m.view != 0 {
            continue;
        }
        let (f, fm) = flip_selection(e, m)?;
        let (reverse, _) = flip_selection(&f, &fm)?;
        if digest(&reverse.request)? != digest(&e.request)? || reverse.answer != e.answer {
            return Err(bad("selector involution"));
        }
        out.push(f);
        metadata.push(fm);
    }
    Ok((out, metadata))
}
fn paired_evaluation(origin: usize, updates: usize, tiny: bool) -> EvaluationPolicy {
    let (screen, full) = if tiny { (vec![], vec![updates]) }
        else if updates == 256 { (vec![32, 64, 128], vec![256]) }
        else if updates == 1280 { (vec![32,64,128,768], vec![256,1280]) }
        else if updates == 3840 { (vec![32,64,128,768,1792,2816,3328], vec![256,1280,2304,3840]) }
        else if updates == 2560 { (vec![32,64,128,512,1536], vec![1024,2048,2560]) }
        else { (vec![512, 1536, 2560, 3072], vec![1024, 2048, 3584]) };
    let full: Vec<_> = full.into_iter().map(|n| origin+n).collect();
    EvaluationPolicy {
        screen_steps: screen.into_iter().map(|n| origin+n).collect(),
        primary_steps: full.clone(), train_steps: full.clone(), transfer_steps: full,
        teacher_steps: vec![], fixture_per_bucket: tiny.then_some(1),
        generation_limit: 12_000, teacher_limit: 10_000, active_seconds: 21_600,
        ..EvaluationPolicy::default()
    }
}
fn paired_flip_panel(p: &Plan, root: &Path, step: usize, dev: &[Episode], dm: &[Meta])
    -> Result<(&'static str, Vec<Episode>, Vec<Meta>)> {
    let f = p.fork.as_ref().ok_or_else(|| bad("paired fork missing"))?;
    let s: Study = read(&f.study.join("study.r3b"))?;
    let path = f.study.join("diagnostic.r3b");
    if file_hash(&path)? != s.diagnostic_hash { return Err(bad("paired frozen flip panel")); }
    let (es, ms): (Vec<Episode>, Vec<Meta>) = read(&path)?;
    if p.evaluation.primary_steps.contains(&step) { return Ok(("selector192", es, ms)); }
    if !p.evaluation.screen_steps.contains(&step) { return Err(bad("unregistered paired evaluation")); }
    let (screen, _) = subset(dev, dm, p.evaluation.fixture_per_bucket.unwrap_or(8));
    let mut selected = (vec![], vec![]);
    for (e, m) in es.into_iter().zip(ms) {
        if screen.iter().any(|x| Some(&x.id) == m.source_id.as_ref()) {
            selected.0.push(e); selected.1.push(m);
        }
    }
    let _ = root;
    if selected.0.len() != 3 * p.evaluation.fixture_per_bucket.unwrap_or(8) {
        return Err(bad("paired screen/flip coverage"));
    }
    Ok(("flip24", selected.0, selected.1))
}
fn paired_screen(p: &Plan, root: &Path, step: usize) -> Result<PanelResult> {
    let (_, _, dev) = p.training_corpus(&root.join("corpus.r3cor"))?;
    let (_, dm, _) = verified_metadata(root, p)?;
    let (es, ms) = subset(&dev, &dm, p.evaluation.fixture_per_bucket.unwrap_or(8));
    let tok = ByteBpe::load(&root.join("tokenizer.r3b"))?;
    if !p.evaluation.primary_steps.contains(&step) {
        return audit_panel(root, p, step, "screen64", &es, &ms, &tok);
    }
    let (all, am) = if p.tiny { (es.clone(), ms.clone()) } else { (dev, dm) };
    let full = audit_panel(root, p, step, "dev512", &all, &am, &tok)?;
    let prefix = format!("eval-{step:04}-dev512");
    let rows = binary::read_value_records(&root.join(format!("{prefix}.r3rows")))?;
    let teachers = binary::read_value_records(&root.join(format!("{prefix}-teachers.r3rows")))?;
    let mut selected = vec![]; let mut ts = vec![];
    for e in &es {
        let i = rows[1..].iter().position(|r| r["id"] == e.id).ok_or_else(|| bad("paired original missing"))? + 1;
        selected.push(rows[i].clone()); ts.push(teachers[i].clone());
    }
    let mut r = score(&selected, &es, &ms)?;
    r.step = step; r.panel = "screen64".into(); r.model = full.model;
    r.dataset = digest(&es)?; r.raw_hash = full.raw_hash;
    r.ce = Some(teacher_ce(&ts)?); r.teachers_completed = Some(ts.len());
    Ok(r)
}
// Pure decision: all outcomes are reconstructed from the same endpoint's raw panels.
fn paired_decision(p: &Plan, root: &Path, step: usize) -> Result<binary::Value> {
    let f = p.fork.as_ref().ok_or_else(|| bad("paired fork missing"))?;
    let (_, _, dev) = p.training_corpus(&root.join("corpus.r3cor"))?;
    let (_, dm, _) = verified_metadata(root, p)?;
    let (name, es, ms) = paired_flip_panel(p, root, step, &dev, &dm)?;
    let tok = ByteBpe::load(&root.join("tokenizer.r3b"))?;
    let flip = audit_panel(root, p, step, name, &es, &ms, &tok)?;
    let full = p.evaluation.primary_steps.contains(&step);
    let original_name = if full { "dev512" } else { "screen64" };
    let original = binary::read_value_records(&root.join(format!("eval-{step:04}-{original_name}.r3rows")))?;
    let flipped = binary::read_value_records(&root.join(format!("eval-{step:04}-{name}.r3rows")))?;
    if ["model", "step", "policy", "tokenizer"].iter()
        .any(|key| original[0]["binding"][*key] != flipped[0]["binding"][*key]) {
        return Err(bad("paired endpoints differ"));
    }
    let mut pairs = selector_pairs(&original[1..], &flipped[1..], &es, &ms)?;
    let mut bases = std::collections::BTreeSet::new();
    let mut same = 0;
    for ((row, _), m) in flipped[1..].iter().zip(&es).zip(&ms) {
        let old = original[1..].iter().find(|r| Some(r["id"].as_str().unwrap_or("")) == m.source_id.as_deref())
            .ok_or_else(|| bad("paired source row missing"))?;
        if old["exact_match"] == true && row["exact_match"] == true { bases.insert(&m.base); }
        let normal = |r: &binary::Value| r["generation_completed"] == true && r["finish_reason"] == "stop" && r["error"].is_null();
        same += usize::from(normal(old) && normal(row) && old["actual"] == row["actual"]);
    }
    pairs["both_bases"] = binary::record!(bases.len()); pairs["same_output"] = binary::record!(same);
    let screen = paired_screen(p, root, step)?;
    if screen.model != flip.model { return Err(bad("paired model mismatch")); }
    let parent: binary::Value = read(&f.study.join("parent-audit.r3b"))?;
    let baseline: PanelResult = binary::from_value(parent["baseline_screen"].clone())?;
    let study:Study=read(&f.study.join("study.r3b"))?;
    let prior = if [16,17].contains(&study.schema) {Some(read::<binary::Value>(&f.study.join("research-authorization.r3b"))?)} else {None};
    let mut streak = match &prior {
        Some(a) => a["prior_decision"]["guard_streak"].as_u64().ok_or_else(||bad("prior guard UNKNOWN"))?,
        None => 0,
    };
    let mut stop = None;
    let mut steps = p.evaluation.screen_steps.clone(); steps.extend(&p.evaluation.primary_steps); steps.sort(); steps.dedup();
    for n in steps.into_iter().filter(|&n| n <= step) {
        let r = if n == step { screen.clone() } else { paired_screen(p, root, n)? };
        if !p.tiny {
            if r.exact + 16 <= baseline.exact || r.errors >= 8 { stop = Some("QUALITY_REGRESSION_SEVERE"); }
            if r.exact + 8 <= baseline.exact { streak += 1; } else { streak = 0; }
            if streak >= 2 { stop = Some("QUALITY_REGRESSION_SCREEN"); }
        }
    }
    #[cfg(feature = "test-support")]
    if p.tiny && std::env::var("R3_FRESH_TEST_STOP").as_deref() == Ok("paired-quality") {
        stop = Some("QUALITY_REGRESSION_FIXTURE");
    }
    let mut primary = None; let mut transfer = None; let mut admissible = false; let mut joint = false;
    if full {
        let (ds, dms) = if p.tiny { subset(&dev, &dm, 1) } else { (dev, dm) };
        let a = audit_panel(root, p, step, "dev512", &ds, &dms, &tok)?;
        let tx = verified_corpus(&root.join("transfer.r3cor"), &p.transfer)?.validation;
        let (_, _, xm) = verified_metadata(root, p)?;
        let (tx, xm) = if p.tiny { subset(&tx, &xm, 1) } else { (tx, xm) };
        let b = audit_panel(root, p, step, "transfer128", &tx, &xm, &tok)?;
        if a.model != b.model || a.model != flip.model { return Err(bad("paired full model mismatch")); }
        let both = pairs["pairs"][0].as_u64().ok_or_else(|| bad("paired both count"))?;
        let each = |limit| (2..=4).all(|bucket| pairs["bucket_pairs"][bucket.to_string().as_str()][0].as_u64().is_some_and(|n| n >= limit));
        let normal = a.errors == 0 && b.errors == 0 && flip.errors == 0 && stop.is_none();
        admissible = normal && a.exact >= 417 && b.exact >= 75 && both >= 12 && each(1) && bases.len() >= 4;
        joint = normal && a.exact >= 487 && a.buckets.iter().all(|&n| n >= 58) && b.exact >= 116 && both >= 173 && each(56);
        primary = Some(a.exact); transfer = Some(b.exact);
        if joint { stop = Some("DEVELOPMENT_JOINT_PASS_FINAL_NOT_RUN"); }
        let study:Study=read(&f.study.join("study.r3b"))?;
        if [5,6,7,8,9,10,11,12,13,14,15,16,17,18,19,20].contains(&study.schema) && !joint && stop.is_none() && !p.tiny {
            let prior=if [5,16,17].contains(&study.schema) {
                let authorization:binary::Value=read(&f.study.join("research-authorization.r3b"))?;
                authorization["prior_decision"].clone()
            } else {
                binary::record!({"primary":parent["panels"][format!("eval-{:04}-dev512",study.parent_step).as_str()]["exact"],
                    "transfer":parent["panels"][format!("eval-{:04}-transfer128",study.parent_step).as_str()]["exact"],"pairs":parent["pairs"]})
            };
            let mut values=vec![(prior["primary"].as_u64().ok_or_else(||bad("prior primary missing"))?,
                prior["transfer"].as_u64().ok_or_else(||bad("prior transfer missing"))?,
                prior["pairs"]["pairs"][0].as_u64().ok_or_else(||bad("prior both missing"))?)];
            if [16,17].contains(&study.schema) {
                let authorization:binary::Value=read(&f.study.join("research-authorization.r3b"))?;
                values=binary::from_value(authorization["prior_progress"].clone())?;
                if values.is_empty() {return Err(bad("prior progress UNKNOWN"));}
            }
            for n in p.evaluation.primary_steps.iter().copied().filter(|&n|n<step) {
                let r=paired_decision(p,root,n)?;
                let saved:binary::Value=read_confirmed(&root.join(format!("paired-{n:04}.r3b")))?;
                if r!=saved {return Err(bad("previous progression decision differs from raw"));}
                values.push((r["primary"].as_u64().ok_or_else(||bad("previous primary missing"))?,
                    r["transfer"].as_u64().ok_or_else(||bad("previous transfer missing"))?,
                    r["pairs"]["pairs"][0].as_u64().ok_or_else(||bad("previous both missing"))?));
            }
            values.push((a.exact as u64,b.exact as u64,both));
            if no_further_progress(&values) {stop=Some("NO_FURTHER_PROGRESS");}
        }
    }
    Ok(binary::record!({"policy":digest(p)?,"step":step,"local_step":step-f.origin_step,"model":flip.model,
        "original_raw":file_hash(&root.join(format!("eval-{step:04}-{original_name}.r3rows")))?,"flip_raw":flip.raw_hash,
        "screen":screen,"pairs":pairs,"primary":primary,"transfer":transfer,"admissible":admissible,
        "development_joint_pass":joint,"guard_streak":streak,"stop":stop}))
}
fn no_further_progress(values:&[(u64,u64,u64)])->bool {
    let Some(&mut_best)=values.first() else{return false};
    let mut best=mut_best;let mut streak=0;
    for &(a,b,c) in &values[1..] {
        if a>best.0||b>best.1||c>best.2 {streak=0;}else{streak+=1;}
        best=(best.0.max(a),best.1.max(b),best.2.max(c));
        if streak>=2 {return true;}
    }
    false
}
fn paired_evaluate(p: &Plan, root: &Path, path: &Path, step: usize, control: &mut recovery::RunControl) -> Result<Option<String>> {
    let (_, train, dev) = p.training_corpus(&root.join("corpus.r3cor"))?;
    let (tm, dm, xm) = verified_metadata(root, p)?;
    let count = p.evaluation.fixture_per_bucket.unwrap_or(8);
    if p.evaluation.primary_steps.contains(&step) {
        let (es, ms) = subset(&train, &tm, count);
        evaluate_panel(p, root, path, step, "train64", &es, &ms, control)?;
        let (es, ms) = if p.tiny { subset(&dev, &dm, count) } else { (dev.clone(), dm.clone()) };
        evaluate_panel(p, root, path, step, "dev512", &es, &ms, control)?;
        let tx = verified_corpus(&root.join("transfer.r3cor"), &p.transfer)?.validation;
        let (es, ms) = if p.tiny { subset(&tx, &xm, count) } else { (tx, xm) };
        evaluate_panel(p, root, path, step, "transfer128", &es, &ms, control)?;
    } else {
        let (es, ms) = subset(&dev, &dm, count);
        evaluate_panel(p, root, path, step, "screen64", &es, &ms, control)?;
    }
    let (name, es, ms) = paired_flip_panel(p, root, step, &dev, &dm)?;
    evaluate_panel(p, root, path, step, name, &es, &ms, control)?;
    let result = paired_decision(p, root, step)?;
    let dest = root.join(format!("paired-{step:04}.r3b"));
    if dest.exists() {
        let saved: binary::Value = read_confirmed(&dest)?;
        if saved != result { return Err(bad("paired decision mismatch")); }
    } else { publish_confirmed(&dest, &result)?; }
    println!("PAIRED_EVALUATION {result}");
    Ok(result["stop"].as_str().map(str::to_owned))
}
fn pair_tapes(parent: &Plan, tm: &[Meta], block: usize, steps: usize) -> Result<[Vec<[usize;8]>;2]> {
    if block < 2 || !block.is_multiple_of(2) || !steps.is_multiple_of(block) || steps > 3840 {
        return Err(bad("pair block/maximum length"));
    }
    let n = tm.len();
    let pairs = block/2;
    let mut rows: Vec<[usize;8]> = (0..steps).map(|i| parent.training_draw(parent.config.max_steps+i)
        .try_into().map_err(|_| bad("pair effective batch must be eight"))).collect::<Result<_>>()?;
    let mut out = [rows.clone(), std::mem::take(&mut rows)];
    for bucket in [2, 3, 4] {
        let mut bases: BTreeMap<&str, [Option<usize>;4]> = BTreeMap::new();
        for (i, m) in tm.iter().enumerate().filter(|(_,m)|m.bucket==bucket) {
            if m.id.is_empty() || m.view>3 || bases.entry(&m.base).or_default()[m.view].replace(i).is_some() {
                return Err(bad("duplicate/invalid train base view"));
            }
        }
        if bases.len()<pairs || !bases.len().is_multiple_of(pairs) || bases.values().any(|v|v.contains(&None)) {
            return Err(bad("insufficient distinct train bases / missing view"));
        }
        let mut bases: Vec<_> = bases.into_iter().collect();
        bases.sort_by_key(|(name,_)| neural::hash(format!("pair/{}/{bucket}/{name}",parent.config.seed).as_bytes()));
        for b in 0..steps/block {
            let view = (b*pairs/bases.len())%4;
            let start = (b*pairs)%bases.len();
            let mut group: Vec<_> = bases[start..start+pairs].iter().map(|(id,v)|(*id,v[view].unwrap())).collect();
            group.sort_by_key(|(id,_)| neural::hash(format!("pair-order/{}/{bucket}/{b}/{id}",parent.config.seed).as_bytes()));
            let mut first: Vec<_> = (0..pairs).map(|j|j%2).collect();
            let mut phrase: Vec<_> = (0..pairs).map(|j|(j+b)%2).collect();
            let mut rng = Rng::new(parent.config.seed ^ ((bucket+1) as u64*31) ^ (b as u64*977));
            for v in [&mut first, &mut phrase] {
                for j in (1..v.len()).rev() { let k = rng.next_u64() as usize%(j+1); v.swap(j,k); }
            }
            for (j,(_,id)) in group.iter().enumerate() {
                for side in 0..2 {
                    let sample = id+phrase[j]*n+(first[j]^side)*2*n;
                    out[0][b*block+j+side*pairs][bucket]=sample;
                    out[1][b*block+2*j+side][bucket]=sample;
                }
            }
        }
    }
    for b in 0..steps/block {
        let mut a: Vec<_> = out[0][b*block..(b+1)*block].iter().flat_map(|r|r.iter().copied().enumerate()).collect();
        let mut c: Vec<_> = out[1][b*block..(b+1)*block].iter().flat_map(|r|r.iter().copied().enumerate()).collect();
        a.sort();c.sort();if a!=c {return Err(bad("pair block multiset differs"));}
    }
    Ok(out)
}
fn verify_pair_tape(parent: &Plan, tm: &[Meta], tape: &PairTape) -> Result<()> {
    let arm = match tape.mode.as_str() { "SPACED" => 0, "ADJACENT" | "COBATCH" | "CONTRAST" | "SIDE" | "REPLAY" | "WIDE" | "FIT" | "VALUE" | "COVER" | "DIVERSE" | "COVER4" | "GROUND" => 1, _ => return Err(bad("unknown pair policy")) };
    let mut expected = pair_tapes(parent, tm, tape.block, tape.rows.len())?;
    if ["COBATCH","CONTRAST","SIDE","REPLAY","WIDE","FIT","VALUE","COVER","DIVERSE","COVER4","GROUND"].contains(&tape.mode.as_str()) {
        let repeat = match tape.mode.as_str() { "REPLAY" => Some(tape.block), "WIDE" => Some(2*tape.block), "FIT" | "VALUE" | "DIVERSE" => Some(if parent.tiny {2}else{16}), "COVER" | "COVER4" | "GROUND" => Some(if parent.tiny {4}else{64}), _ => None };
        expected[1]=co_batch_rows(&expected[1],repeat)?;
        if ["VALUE","COVER","DIVERSE","COVER4","GROUND"].contains(&tape.mode.as_str()) {expected[1]=value_cycle_rows(&expected[1],tm,repeat.unwrap(),if ["DIVERSE","COVER4","GROUND"].contains(&tape.mode.as_str()) {4}else{2})?;}
    }
    if expected[arm] != tape.rows { return Err(bad("pair tape duplicate/missing/unknown case or altered order")); }
    Ok(())
}
// Repack each adjacent pair of eight-case updates without adding/removing a case.
// The six C/D/E cases share the first batch; the other ten cases fill its two
// remaining slots and the next batch. Rotate the two anchor slots without labels.
// Optional recurrence changes only C/D/E sample selection, not the anchor rows.
fn co_batch_rows(adjacent: &[[usize;8]], repeat_block: Option<usize>) -> Result<Vec<[usize;8]>> {
    if adjacent.is_empty() || !adjacent.len().is_multiple_of(2) {return Err(bad("co-batch needs complete adjacent update pairs"));}
    // A finite tape may end partway through a recurrence cycle, but never a pair.
    if repeat_block.is_some_and(|n| n<2 || !n.is_multiple_of(2) || n>adjacent.len()) {return Err(bad("recurrence needs complete even pairs"));}
    let mut out=Vec::with_capacity(adjacent.len());
    let others=[0,1,5,6,7];
    for (j,pair) in adjacent.as_chunks::<2>().0.iter().enumerate() {
        let mut a=[0;8];let mut b=[0;8];
        let selected: &[[usize;8]]=if let Some(n)=repeat_block {&adjacent[(2*j)%n..][..2]}else{pair};
        for i in 0..3 {a[2*i]=selected[0][i+2];a[2*i+1]=selected[1][i+2];}
        for k in 0..5 {
            let bucket=others[(j+k)%5];
            if k<2 {a[6+k]=pair[0][bucket];}else{b[k-2]=pair[0][bucket];}
            b[3+k]=pair[1][bucket];
        }
        out.extend([a,b]);
    }
    Ok(out)
}
// Keep the fitted scenes/phrase/sides and anchors; every other cycle changes
// only the owned original view0 to its same-scene value-exchange view1.
fn value_exchange_rows(rows: &[[usize;8]], tm: &[Meta], cycle: usize) -> Result<Vec<[usize;8]>> {
    value_cycle_rows(rows,tm,cycle,2)
}
fn value_cycle_rows(rows: &[[usize;8]], tm: &[Meta], cycle: usize, view_count:usize) -> Result<Vec<[usize;8]>> {
    if ![2,4].contains(&view_count) || tm.is_empty() || cycle<2 || !cycle.is_multiple_of(2) || rows.is_empty() || !rows.len().is_multiple_of(cycle) {
        return Err(bad("value exposure cycle/metadata"));
    }
    let n=tm.len();let mut views=BTreeMap::new();
    for (i,m) in tm.iter().enumerate().filter(|(_,m)|m.view<view_count) {
        if views.insert((&m.base,m.bucket,m.view),i).is_some() {return Err(bad("duplicate value view"));}
    }
    let mut out=rows.to_vec();
    for (step,row) in out.iter_mut().enumerate() {
        for index in row {
            if *index>=4*n {return Err(bad("value exposure index"));}
            let m=&tm[*index%n];
            if !(2..=4).contains(&m.bucket) {continue;}
            if m.view!=0 {return Err(bad("value exposure requires original view0"));}
            let view=(step/cycle)%view_count;
            let other=*views.get(&(&m.base,m.bucket,view)).ok_or_else(||bad("missing value view"))?;
            *index=(*index/n)*n+other;
        }
    }
    Ok(out)
}
// Training-only pool. Additional views change one record value each; evaluation
// continues to read the original corpus. Labels never enter product inference.
fn diverse_training_values(all:&[Episode],tm:&[Meta])->Result<Vec<Episode>> {
    let n=tm.len();
    if n==0 || all.len()!=4*n {return Err(bad("diverse value pool shape"));}
    let numeric=|s:&str|!s.is_empty()&&s.bytes().all(|b|b.is_ascii_digit());
    let mut donors=std::collections::BTreeSet::new();
    for e in &all[..n] {for r in &e.request.evidence.items {donors.insert(parsed_record(r)?.2.to_owned());}}
    let mut original=BTreeMap::new();
    for (i,m) in tm.iter().enumerate().filter(|(_,m)|m.view==0) {
        if original.insert((&m.base,m.bucket),i).is_some() {return Err(bad("duplicate original value scene"));}
    }
    let mut out=all.to_vec();
    for (i,e) in out.iter_mut().enumerate() {
        let m=&tm[i%n];if !(2..=4).contains(&m.bucket)||m.view<2 {continue;}
        if m.view>3 {return Err(bad("diverse value view bounds"));}
        let at=*original.get(&(&m.base,m.bucket)).ok_or_else(||bad("diverse value original missing"))?+(i/n)*n;
        let old=&all[at];let records=&old.request.evidence.items;
        if records.len()!=2 || resolve(&old.request)?!=old.answer {return Err(bad("diverse value source label"));}
        let values=[parsed_record(&records[0])?.2,parsed_record(&records[1])?.2];
        let slot=m.view-2;let value=donors.iter().find(|v|!values.contains(&v.as_str())
            &&numeric(v)==numeric(values[slot])&&(!numeric(v)||v.len()==values[slot].len()))
            .ok_or_else(||bad("diverse value donor unavailable"))?;
        let (id,request_id)=(e.id.clone(),e.request.request_id.clone());
        *e=old.clone();e.id=id;e.request.request_id=request_id;
        let (entity,context,_)=parsed_record(&records[slot])?;
        e.request.evidence.items[slot].original_excerpt=format!("{entity}의 {context} 값은 {value}이다.");
        e.answer=resolve(&e.request)?;
        if citations(&e.answer)?!=citations(&old.answer)? {return Err(bad("diverse values changed selector"));}
    }
    Ok(out)
}
fn training_value_corpus(mut all:Vec<Episode>,dev:Vec<Episode>,seed:u64)->Result<data::native::Corpus> {
    // Original and phrase files deliberately share episode IDs. The merged
    // train-only pool needs unique container IDs; request IDs/tokens stay intact.
    for (i,e) in all.iter_mut().enumerate() {e.id=format!("training-slot-{i}/{}",e.id);}
    corpus(all,dev,seed)
}
fn record_groundings(es:&[Episode],tm:&[Meta],framed:&[Sample])->Result<Vec<Option<RecordGrounding>>> {
    if tm.is_empty() || es.len()!=4*tm.len() || framed.len()!=es.len() {return Err(bad("grounding owned samples shape"));}
    es.iter().zip(framed).enumerate().map(|(i,(e,s))| {
        if !(2..=4).contains(&tm[i%tm.len()].bucket) {return Ok(None);}
        if resolve(&e.request)?!=e.answer || e.request.evidence.items.len()!=2 {return Err(bad("grounding training label not uniquely resolved"));}
        let refs=citations(&e.answer)?;
        if refs.len()!=1 {return Err(bad("grounding needs one selected training record"));}
        let selected=e.request.evidence.items.iter().position(|r|r.event_id==refs[0]).ok_or_else(||bad("grounding training citation absent"))?;
        Ok(Some(RecordGrounding::from_sample(s,selected)?))
    }).collect()
}
// Same scene/value/order/selector sides; one in four COVER cycles uses the
// already-owned view3 wording. P variants and view0 still occur in other cycles.
fn selector_phrase_rows(rows: &[[usize;8]], tm: &[Meta], start:usize, cycle:usize) -> Result<Vec<[usize;8]>> {
    if tm.is_empty() || cycle<2 || !cycle.is_multiple_of(2) || start>=rows.len() || !start.is_multiple_of(cycle) {
        return Err(bad("selector phrase cursor/cycle"));
    }
    let n=tm.len();let mut views=BTreeMap::new();
    for (i,m) in tm.iter().enumerate().filter(|(_,m)|m.view==3) {
        if views.insert((&m.base,m.bucket),i).is_some() {return Err(bad("duplicate wording view"));}
    }
    let mut out=rows.to_vec();let mut changes=0;
    for (step,row) in out.iter_mut().enumerate().skip(start) {
        for index in row {
            if *index>=4*n {return Err(bad("wording sample bound"));}
            let m=&tm[*index%n];
            if !(2..=4).contains(&m.bucket) || !(step/cycle).is_multiple_of(4) {continue;}
            if m.view!=0 {return Err(bad("wording cycle must use unchanged value view0"));}
            let other=*views.get(&(&m.base,m.bucket)).ok_or_else(||bad("owned wording view missing"))?;
            *index=(*index/(2*n))*(2*n)+other;changes+=1;
        }
    }
    if changes==0 {return Err(bad("wording intervention changed no samples"));}
    Ok(out)
}
fn verify_selector_phrase_change(a:&Episode,b:&Episode)->Result<()> {
    let mut expected=a.clone();
    expected.id=b.id.clone();expected.request.request_id=b.request.request_id.clone();
    expected.request.input=b.request.input.clone();
    if digest(&expected)?!=digest(b)? || a.request.input==b.request.input
        || question_intent(&a.request.input)?!=question_intent(&b.request.input)?
        || resolve(&a.request)?!=a.answer || resolve(&b.request)?!=b.answer {
        return Err(bad("selector phrase must change only wording and case identity"));
    }
    Ok(())
}
fn paired_prepare(parent: &Path, source_data: &Path, output: &Path, parity: Option<&Path>, interventions: [bool;12]) -> Result<()> {
    let [first_target_four, learning_rate_threefold, co_batch, pair_contrast, sidewise_contrast, repeat_pair_block, repeat_two_blocks, fit_seen_pairs, alternate_pair_values, cover_value_pairs, diverse_pair_values, ground_selected_record] = interventions;
    let diverse_cover=cover_value_pairs&&diverse_pair_values;
    if (ground_selected_record && !diverse_cover) || interventions.into_iter().filter(|x|*x).count()>if ground_selected_record {3}else if diverse_cover {2}else{1} {return Err(bad("only one research intervention may be registered"));}
    let single = interventions.into_iter().any(|x|x);
    let lr = if learning_rate_threefold {9e-5}else{3e-5};
    let started=Instant::now();
    let parent=parent.canonicalize()?;
    let pp: Plan=read(&parent.join("plan.r3b"))?;
    let h=history(&parent,&pp)?;
    let end=h.last().ok_or_else(||bad("paired parent missing durable endpoint"))?;
    if end.resume || end.step!=pp.config.max_steps || (!pp.tiny && end.step!=6144) {
        return Err(bad("paired requires closed P6144 parent"));
    }
    let cp=parent.join(&end.checkpoint);
    let l=checkpoint::load(&cp,Device::Cpu,true)?;
    let state=l.manifest.training.as_ref().ok_or_else(||bad("paired parent missing Adam"))?;
    if state.step!=end.step || state.resume_binding.as_ref()!=Some(&pp.binding(state,&l.tokenizer)?)
        || l.tokenizer.id()!=pp.tokenizer || state.config.first_target_weight!=1. {
        return Err(bad("paired parent objective/native binding"));
    }
    let dp: Plan=read(&source_data.join("plan.r3b"))?;
    let f=dp.fork.as_ref().ok_or_else(||bad("pair data provenance missing"))?;
    if f.arm!="S-SELECT" || dp.corpus!=pp.corpus || dp.tokenizer!=pp.tokenizer {
        return Err(bad("paired training pool differs from P wording/original"));
    }
    let (_, train, dev)=pp.training_corpus(&parent.join("corpus.r3cor"))?;
    let (tm,dm,_)=verified_metadata(&parent,&pp)?;
    let variants=verified_corpus(&source_data.join("variants.r3cor"),f.variants.as_ref().ok_or_else(||bad("missing phrases"))?)?.train;
    let parent_variants = verified_corpus(&parent.join("variants.r3cor"), pp.fork.as_ref()
        .and_then(|p|p.variants.as_ref()).ok_or_else(||bad("P phrase provenance missing"))?)?.train;
    if digest(&variants)? != digest(&parent_variants)? { return Err(bad("P phrase content changed")); }
    let flips=verified_corpus(&source_data.join("selectors.r3cor"),f.selector.as_ref().ok_or_else(||bad("missing train selectors"))?)?.train;
    let sm: Vec<Meta>=read(&source_data.join("selector-metadata.r3b"))?;
    if Some(file_hash(&source_data.join("selector-metadata.r3b"))?)!=f.selector_metadata
        || train.len()!=variants.len() || flips.len()!=train.len()*2 || sm.len()!=flips.len() {
        return Err(bad("paired owned train pool shape/hash"));
    }
    let n=train.len();
    for (style,es) in [&train,&variants].iter().enumerate() {
        for (i,(e,m)) in es.iter().zip(&tm).enumerate() {
            let other=&flips[style*n+i];
            if (2..=4).contains(&m.bucket) {
                let (expected,_) = flip_selection(e,m)?;
                if digest(&expected.request)?!=digest(&other.request)? || expected.answer!=other.answer
                    || other.answer==e.answer || sm[style*n+i].source_id.as_deref()!=Some(train[i].id.as_str()) {
                    return Err(bad("paired train annotation mismatch"));
                }
            } else if digest(&e.request)?!=digest(&other.request)? || e.answer!=other.answer {
                return Err(bad("nonselector train content changed"));
            }
        }
    }
    let mut all=train.clone();all.extend(variants);all.extend(flips);
    if diverse_pair_values {all=diverse_training_values(&all,&tm)?;}
    validate_framed(&all,&l.tokenizer)?;
    let framed=samples(&all,&l.tokenizer,pp.config.seq_len)?;
    let grounding=if ground_selected_record {Some(digest(&record_groundings(&all,&tm,&framed)?)?)}else{None};
    let block=if pp.tiny {2}else{256};
    let tapes=pair_tapes(&pp,&tm,block,if pp.tiny {if diverse_cover {16}else if repeat_two_blocks || cover_value_pairs || diverse_pair_values {8}else if repeat_pair_block || fit_seen_pairs || alternate_pair_values {4}else{2}}else{3840})?;
    let repeat = if cover_value_pairs {Some(if pp.tiny {4}else{64})}else if fit_seen_pairs || alternate_pair_values || diverse_pair_values {Some(if pp.tiny {2}else{16})}else if repeat_two_blocks {Some(2*block)}else{repeat_pair_block.then_some(block)};
    let updates=if ground_selected_record&&pp.tiny {2}else if first_target_four {tapes[1].len()}else if single {if pp.tiny {tapes[1].len()}else{1280}}else{block};
    let mut planned=if repeat.is_some() {[tapes[0].clone(),co_batch_rows(&tapes[1],repeat)?]}else{tapes.clone()};
    if alternate_pair_values || (cover_value_pairs && !diverse_pair_values) {
        let before=planned[1].clone();
        planned[1]=value_exchange_rows(&before,&tm,repeat.unwrap())?;
        for (old,new) in before.iter().flatten().zip(planned[1].iter().flatten()) {
            if old!=new {verify_seen_value_swap(&all[*old],&all[*new])?;}
        }
    }
    if diverse_pair_values {planned[1]=value_cycle_rows(&planned[1],&tm,repeat.unwrap(),4)?;}
    let totals: Vec<_>=planned.iter().map(|t| t.iter().map(|r| r.iter().fold((0u64,0u64),|(i,v),&j|
        (i+(framed[j].tokens.len()-1) as u64,v+(framed[j].tokens.len()-framed[j].response_start) as u64))).collect::<Vec<_>>()).collect();
    for arm in 0..2 {
        let work: Vec<_>=totals[arm].iter().chain(totals[1-arm][..block].iter()).collect();
        if work.iter().map(|x|x.0).sum::<u64>()>12_000_000 || work.iter().map(|x|x.1).sum::<u64>()>1_000_000 {
            return Err(bad("registered full study token budget insufficient"));
        }
    }
    let parity_receipt=verified_parent_parity(&pp,&cp,parity)?;
    let (_,mut panels)=audit_panels_range(&parent,&pp,end.step,end.step)?;
    let raw=binary::read_value_records(&parent.join(format!("eval-{:04}-dev512.r3rows",end.step)))?;
    let (screen,ms)=subset(&dev,&dm,if pp.tiny {1}else{8});
    let selected: Vec<_>=screen.iter().map(|e|raw[1..].iter().find(|r|r["id"]==e.id).cloned().ok_or_else(||bad("parent screen source missing"))).collect::<Result<_>>()?;
    let baseline=score(&selected,&screen,&ms)?;
    panels.insert(format!("eval-{:04}-screen64",end.step),baseline.clone());
    let (diagnostic,metadata): (Vec<Episode>,Vec<Meta>)=read(&f.study.join("diagnostic.r3b"))?;
    if digest(&(diagnostic.clone(),metadata.clone()))? != digest(&selector_panel(&dev,&dm,pp.tiny)?)? {
        return Err(bad("paired frozen development mapping"));
    }
    let parent_selector=audit_panel(&f.study,&pp,end.step,"selector192",&diagnostic,&metadata,&l.tokenizer)?;
    let flip_raw=binary::read_value_records(&f.study.join(format!("eval-{:04}-selector192.r3rows",end.step)))?;
    let parent_pairs=selector_pairs(&raw[1..],&flip_raw[1..],&diagnostic,&metadata)?;
    println!("PAIRED_PARENT step={} weights={} adam={} primary={} transfer={} selector={} both={}",end.step,
        l.model.weight_hash()?,optimizer_hash(&l.optimizer)?,panels[&format!("eval-{:04}-dev512",end.step)].exact,
        panels[&format!("eval-{:04}-transfer128",end.step)].exact,parent_selector.exact,parent_pairs["pairs"][0]);
    std::fs::create_dir(output)?;
    let output=output.canonicalize()?;
    write(&output.join("diagnostic.r3b"),&(diagnostic,metadata))?;
    write(&output.join("parent-audit.r3b"),&binary::record!({"panels":panels,"baseline_screen":baseline,"selector":parent_selector,"pairs":parent_pairs,"parity":parity_receipt,
        "global_optimizer_limit":if single {updates}else{4096},"global_input_limit":12_000_000,"global_target_limit":1_000_000,
        "G2_per_arm":if single {None}else{Some(256)},"G3_additional":if single {None}else{Some(3584)},"G3_gate":{"primary":417,"transfer":75,"both":12,"both_each":1,"both_bases":4},
        "joint_gate":{"primary":487,"bucket":58,"transfer":116,"both":173,"both_bucket":56},
        "planned_tokens":totals.iter().map(|t|binary::record!({"first_block":t[..block].iter().fold((0u64,0u64),|a,b|(a.0+b.0,a.1+b.1)),"all":t.iter().fold((0u64,0u64),|a,b|(a.0+b.0,a.1+b.1))})).collect::<Vec<_>>()}))?;
    let mut inventory=vec![];
    if single {
        let path=output.join("research-authorization.r3b");
        let mut authorization=binary::record!({"scope":if diverse_cover {"USER_AUTHORIZED_SINGLE_VARIABLE_FOUR_VIEW_COVERAGE_20260920"}else if diverse_pair_values {"USER_AUTHORIZED_SINGLE_VARIABLE_VALUE_DIVERSITY_20260920"}else if cover_value_pairs {"USER_AUTHORIZED_SINGLE_VARIABLE_VALUE_COVERAGE_20260920"}else if alternate_pair_values {"USER_AUTHORIZED_SINGLE_VARIABLE_VALUE_EXPOSURE_20260920"}else if fit_seen_pairs {"USER_AUTHORIZED_SEEN_PAIR_LEARNABILITY_20260920"}else if repeat_two_blocks {"USER_AUTHORIZED_SINGLE_VARIABLE_RECURRENCE_COVERAGE_20260920"}else if repeat_pair_block {"USER_AUTHORIZED_SINGLE_VARIABLE_PAIR_RECURRENCE_20260920"}else if sidewise_contrast {"USER_AUTHORIZED_SINGLE_VARIABLE_SIDE_MARGIN_20260920"}else if pair_contrast {"USER_AUTHORIZED_SINGLE_VARIABLE_PAIR_CONTRAST_20260920"}else if co_batch {"USER_AUTHORIZED_SINGLE_VARIABLE_COBATCH_20260920"}else if learning_rate_threefold {"USER_AUTHORIZED_SINGLE_VARIABLE_LR9E5_20260920"}else{"USER_AUTHORIZED_SINGLE_VARIABLE_FIRST_TARGET4_20260920"},
            "hypothesis":if diverse_cover {"widen DIVERSE recurring scenes8 to32 per C/D/E; same four-value native pool, parent/Adam/LR/objective/anchor draws;256 sides each5 exposures instead of64 sides each20 at1280 updates"}else if diverse_pair_values {"four value combinations versus retained VALUE two; same eight scenes per C/D/E, selectors/phrase/anchor draws/parent/Adam/LR/loss; extra views change one supplied record each"}else if cover_value_pairs {"widen VALUE recurring bases from8 to32 per C/D/E; alternate owned value views each64 updates; fewer exposures at same1280 budget, unchanged anchors/parent/Adam/LR/loss; retained VALUE control"}else if alternate_pair_values {"alternate owned value-swap view every first16-pair cycle; same bases/phrase/selector sides/anchor rows and SIDE objective; retained FIT control"}else if fit_seen_pairs {"test full-answer learnability by repeating first8 seen pairs per C/D/E; unchanged SIDE objective and exact other-task rows; narrowed coverage is diagnostic only"}else if repeat_two_blocks {"repeat first two C/D/E blocks instead of one; same SIDE objective and anchor rows; trade exact exposure for base coverage; retained REPLAY control"}else if repeat_pair_block {"repeat first C/D/E pair block; other five tasks updatewise unchanged; increased exact exposure reduces distinct sample/view coverage; retained SIDE control"}else if sidewise_contrast {"penalize each wrong side instead of allowing aggregate cancellation; retained sum-contrast control at matched cursors"}else if pair_contrast {"paired target-divergence discrimination at unchanged co-batch tape; retained pure-CE control at matched cursors"}else if co_batch {"joint selector-pair gradients at unchanged batch8, LR, loss, Adam and two-update case multiset"}else if learning_rate_threefold {"test adaptation rate with fixed threefold LR; retained coefficient-1 LR3e-5 control at matched cursors"}else{"increase first-response-token learning weight; retained coefficient-1 control at matched tape cursors"},
            "first_target_weight":if first_target_four {4.}else{1.},"constant_lr":lr,
            "objective":if sidewise_contrast || repeat_pair_block || repeat_two_blocks || fit_seen_pairs || alternate_pair_values || cover_value_pairs || diverse_pair_values {"response CE + 0.1 * mean_pair (softplus(1-(za[ya]-za[yb])) + softplus(1-(zb[yb]-zb[ya])))/2; first divergent target, shared teacher prefix; no pairs means zero auxiliary"}else if pair_contrast {"response CE + 0.1 * mean_pair softplus(1 - ((za[ya]-za[yb]) + (zb[yb]-zb[ya]))); first divergent target, shared teacher prefix; no pairs means zero auxiliary"}else if first_target_four {"(sum response NLL + 3 * sum first-target NLL) / actual response token count"}else{"sum response NLL / actual response token count"},
            "new_optimizer_limit":updates,"generation_limit":12000,"teacher_limit":10000,"active_seconds":21600,"input_limit":12000000,"target_limit":1000000,
            "control_absolute_steps":if pp.tiny {vec![state.step+updates]}else if first_target_four {vec![6400,7424,8448,9984]}else{vec![6400,7424]},"prior_candidate_promoted":false});
        if ground_selected_record {
            authorization["scope"]=binary::record!("USER_AUTHORIZED_SINGLE_VARIABLE_RECORD_GROUNDING_20260920");
            authorization["hypothesis"]=binary::record!("add selected-record attention supervision to unchanged COVER4 inputs/tape/parent/Adam/LR/SIDE loss; normal inference unmodified");
            authorization["objective"]=binary::record!("family4 +0.1 mean_CDE -ln(max(A/B,1e-8)); A=last-layer first-response attention summed over heads/selected record content; B=both record contents; no selector gives zero");
            authorization["grounding_annotations"]=binary::record!(grounding);
        }
        write(&path,&authorization)?;
        inventory.push((PathBuf::from("research-authorization.r3b"),file_hash(&path)?,std::fs::metadata(&path)?.len()));
    }
    let mut study=Study {schema:if diverse_cover {19}else if diverse_pair_values {18}else if cover_value_pairs {15}else if alternate_pair_values {14}else if fit_seen_pairs {13}else if repeat_two_blocks {12}else if repeat_pair_block {11}else if sidewise_contrast {10}else if pair_contrast {9}else if co_batch {8}else if learning_rate_threefold {7}else if first_target_four {6}else{4},source:source_digest()?,binary:file_hash(&std::env::current_exe()?)?,parent:parent.clone(),
        parent_plan:file_hash(&parent.join("plan.r3b"))?,parent_checkpoint:cp.clone(),parent_file:file_hash(&cp)?,
        parent_model:l.model.weight_hash()?,parent_adam:optimizer_hash(&l.optimizer)?,parent_state:digest(state)?,
        parent_step:state.step,tokenizer:pp.tokenizer.clone(),updates,tiny:pp.tiny,inventory,
        diagnostic_hash:file_hash(&output.join("diagnostic.r3b"))?,parent_audit_hash:file_hash(&output.join("parent-audit.r3b"))?,
        tie_break:if diverse_cover {vec!["COVER4".into()]}else if diverse_pair_values {vec!["DIVERSE".into()]}else if cover_value_pairs {vec!["COVER".into()]}else if alternate_pair_values {vec!["VALUE".into()]}else if fit_seen_pairs {vec!["FIT".into()]}else if repeat_two_blocks {vec!["WIDE".into()]}else if repeat_pair_block {vec!["REPLAY".into()]}else if sidewise_contrast {vec!["SIDE".into()]}else if pair_contrast {vec!["CONTRAST".into()]}else if co_batch {vec!["COBATCH".into()]}else if single {vec!["ADJACENT".into()]}else{vec!["SPACED".into(),"ADJACENT".into()]}};
    if ground_selected_record {study.schema=20;study.tie_break=vec!["GROUND".into()];}
    write(&output.join("study.r3b"),&study)?;
    for arm in &study.tie_break {
        let tape=if ["VALUE","COVER","DIVERSE","COVER4","GROUND"].contains(&arm.as_str()) {planned[1].clone()}else if ["COBATCH","CONTRAST","SIDE","REPLAY","WIDE","FIT","VALUE","COVER","DIVERSE"].contains(&arm.as_str()) {co_batch_rows(&tapes[1],repeat)?}else{tapes[usize::from(arm=="ADJACENT")].clone()};
        let root=output.join(arm);std::fs::create_dir(&root)?;
        for name in ["corpus.r3cor","transfer.r3cor","metadata.r3b","tokenizer.r3b"] {
            neural::write_new(&root.join(name),&std::fs::read(parent.join(name))?)?;
        }
        for name in ["variants.r3cor","question-variants.r3b","selectors.r3cor","selector-metadata.r3b"] {
            neural::write_new(&root.join(name),&std::fs::read(source_data.join(name))?)?;
        }
        neural::write_new(&root.join("initial.r3m"),&std::fs::read(&cp)?)?;
        let mut table=std::fs::OpenOptions::new().write(true).create_new(true).open(root.join("paired-samples.r3rows"))?;
        for (i,(e,s)) in all.iter().zip(&framed).enumerate() {
            let m=&tm[i%n];
            binary::write_value_record(&mut table,&binary::record!({"index":i,"source":train[i%n].id,"bucket":m.bucket,"base":m.base,"view":m.view,
                "phrase":(i/n)%2,"side":i/(2*n),"request":digest(&e.request)?,"target":digest(&&s.tokens[s.response_start..])?,
                "prompt":digest(&&s.tokens[..s.response_start])?}))?;
        }
        table.flush()?;
        table.sync_all()?;
        let mut plan=pp.clone();plan.source=study.source.clone();plan.binary=study.binary.clone();
        plan.grounding=grounding.clone();
        if diverse_pair_values {
            data::native::write(&root.join("training-values.r3cor"),&training_value_corpus(all.clone(),dev.clone(),pp.data_seed)?,true)?;
            plan.training_values=Some(file_hash(&root.join("training-values.r3cor"))?);
        }
        plan.initial=study.parent_file.clone();plan.initial_weights=study.parent_model.clone();
        plan.config.budget_start_step=state.step;plan.config.budget_start_tokens=state.consumed_tokens;
        plan.config.max_steps=state.step+updates;plan.config.max_tokens=state.consumed_tokens+12_000_000;
        plan.config.lr=lr;plan.config.warmup=0;plan.config.validate_every=block;
        plan.config.first_target_weight=if first_target_four {4.}else{1.};
        plan.evaluation=paired_evaluation(state.step,updates,pp.tiny);
        plan.train_order=digest(&tape)?;
        plan.paired=Some(PairTape {mode:arm.clone(),first_step:state.step,block,samples:file_hash(&root.join("paired-samples.r3rows"))?,rows:tape});
        plan.fork=Some(Fork {study:output.clone(),study_hash:file_hash(&output.join("study.r3b"))?,arm:arm.clone(),
            parent_policy:digest(&pp)?,parent_state:digest(state)?,parent_adam:study.parent_adam.clone(),
            origin_step:state.step,origin_input:state.consumed_tokens,origin_target:state.target_tokens,
            original_corpus:pp.corpus.clone(),tokenizer_training_hash:l.tokenizer.train_hash.clone(),
            variants:f.variants.clone(),variant_metadata:f.variant_metadata.clone(),alternate_first:vec![],
            selector:f.selector.clone(),selector_metadata:f.selector_metadata.clone(),flip_first:vec![],constant_lr:lr,target_limit:1_000_000});
        write(&root.join("plan.r3b"),&plan)?;
        let back:Plan=read(&root.join("plan.r3b"))?;if back!=plan {return Err(bad("paired max tape readback"));}
        let initial=checkpoint::load(&root.join("initial.r3m"),Device::Cpu,true)?;
        if !plan.parent_entry(&root.join("initial.r3m"),&initial)? {return Err(bad("paired initial native readback"));}
        println!("PAIRED_REGISTERED arm={arm} parent={} policy={} tape_steps={} actual_updates=0",state.step,digest(&plan)?,plan.paired.as_ref().unwrap().rows.len());
    }
    publish_confirmed(&output.join("study-ready.r3b"),&binary::record!({"study":file_hash(&output.join("study.r3b"))?,
        "C":file_hash(&output.join(&study.tie_break[0]).join("plan.r3b"))?,"P":if single {None}else{Some(file_hash(&output.join("ADJACENT/plan.r3b"))?)},"elapsed_seconds":started.elapsed().as_secs_f64()}))?;
    Ok(())
}
fn verified_parent_parity(p:&Plan, checkpoint:&Path, parity:Option<&Path>)->Result<binary::Value> {
    if p.tiny { return Ok(binary::Value::Null); }
    let parity=parity.ok_or_else(||bad("production parent16 parity required"))?;
    let proof: binary::Value=read_confirmed(&parity.join("result.r3b"))?;
    let selection: binary::Value=read(&parity.join("selection.r3b"))?;
    let rows=binary::read_value_records(&parity.join("parity.r3rows"))?;
    if proof["matched"]!=16 || proof["originals_unchanged"]!=true || !proof["error"].is_null()
        || proof["control"]["terminal_reason"]!="COMPLETED" || proof["control"]["generation_calls"]!=16
        || proof["control"]["teacher_calls"]!=0 || selection["source"]!=source_digest()?
        || selection["policy"]!=digest(p)? || proof["selection"]!=file_hash(&parity.join("selection.r3b"))?
        || proof["raw"]!=file_hash(&parity.join("parity.r3rows"))? || rows.len()!=17
        || rows[0]["physical"]!=file_hash(checkpoint)? {
        return Err(bad("paired parent16 binding/parity incomplete"));
    }
    Ok(proof)
}
fn paired_continue(parent:&Path, output:&Path, executable:&Path, cover:bool, phrase:bool, parity:Option<&Path>)->Result<()> {
    let started=Instant::now();
    let parent=parent.canonicalize()?;
    let pp:Plan=read(&parent.join("plan.r3b"))?;
    let pf=pp.fork.as_ref().ok_or_else(||bad("paired continuation parent fork"))?;
    let old=study_read_bound(&pf.study,Some(executable))?;
    let (schema,arm)=if phrase {(17,"COVER")}else if cover {(16,"COVER")}else{(5,"ADJACENT")};
    if old.schema!=if cover {15}else{4} || pf.arm!=arm {return Err(bad("unregistered continuation parent"));}
    plan_read_bound(&parent,&old.source,&old.binary)?;
    let h=history(&parent,&pp)?;
    let end=h.last().ok_or_else(||bad("continuation parent missing"))?;
    if end.resume || end.phase.as_deref()!=Some("Finished") || end.step!=pp.config.max_steps
        || end.stop!="BUDGET_REACHED" {return Err(bad("continuation requires intact completed parent, not failed command"));}
    audit_panels_through(&parent,&pp,end.step)?;
    audit_updates(&parent,&pp,&h)?;
    let decision=paired_decision(&pp,&parent,end.step)?;
    let saved:binary::Value=read_confirmed(&parent.join(format!("paired-{:04}.r3b",end.step)))?;
    if saved!=decision {return Err(bad("continuation parent raw mismatch"));}
    let cp=parent.join(&end.checkpoint);
    let l=checkpoint::load(&cp,Device::Cpu,true)?;
    let state=l.manifest.training.as_ref().ok_or_else(||bad("continuation Adam missing"))?;
    if state.resume_binding.as_ref()!=Some(&pp.binding(state,&l.tokenizer)?) || state.config!=pp.config {
        return Err(bad("continuation native policy mismatch"));
    }
    let tape=pp.paired.as_ref().unwrap();
    let updates=if pp.tiny {if cover {8}else{2}}else{(tape.first_step+tape.rows.len()).checked_sub(state.step).ok_or_else(||bad("continuation tape exhausted"))?};
    if updates!=if pp.tiny {if cover {8}else{2}}else if cover {2560}else{3584} {return Err(bad("continuation fixed remaining tape length"));}
    let parity_receipt=if cover {verified_parent_parity(&pp,&cp,parity)?}else{binary::Value::Null};
    let mut next_tape=tape.clone();
    if pp.tiny {next_tape.first_step=state.step;}
    if phrase {
        let (tm,_,_)=verified_metadata(&parent,&pp)?;
        next_tape.rows=selector_phrase_rows(&tape.rows,&tm,state.step-next_tape.first_step,if pp.tiny {4}else{64})?;
        let mut all=verified_corpus(&parent.join("corpus.r3cor"),&pp.corpus)?.train;
        all.extend(verified_corpus(&parent.join("variants.r3cor"),pf.variants.as_ref().unwrap())?.train);
        all.extend(verified_corpus(&parent.join("selectors.r3cor"),pf.selector.as_ref().unwrap())?.train);
        let changed:std::collections::BTreeSet<_>=tape.rows.iter().flatten().zip(next_tape.rows.iter().flatten())
            .filter(|(a,b)|a!=b).map(|(a,b)|(*a,*b)).collect();
        for (a,b) in &changed {verify_selector_phrase_change(&all[*a],&all[*b])?;}
        println!("PHRASE_INPUT_CHANGED_PAIRS={} only_owned_train_question_wording=VERIFIED optimizer=0",changed.len());
    }
    let mut prior_progress=vec![];
    if cover && !pp.tiny {
        let baseline:binary::Value=read(&pf.study.join("parent-audit.r3b"))?;
        prior_progress.push((baseline["panels"][format!("eval-{:04}-dev512",old.parent_step).as_str()]["exact"].as_u64().ok_or_else(||bad("prior primary UNKNOWN"))?,
            baseline["panels"][format!("eval-{:04}-transfer128",old.parent_step).as_str()]["exact"].as_u64().ok_or_else(||bad("prior transfer UNKNOWN"))?,
            baseline["pairs"]["pairs"][0].as_u64().ok_or_else(||bad("prior pairs UNKNOWN"))?));
        for step in &pp.evaluation.primary_steps {
            let d=paired_decision(&pp,&parent,*step)?;
            let saved:binary::Value=read_confirmed(&parent.join(format!("paired-{step:04}.r3b")))?;
            if d!=saved || !d["stop"].is_null() {return Err(bad("failed prior decision cannot continue"));}
            prior_progress.push((d["primary"].as_u64().ok_or_else(||bad("prior primary UNKNOWN"))?,
                d["transfer"].as_u64().ok_or_else(||bad("prior transfer UNKNOWN"))?,
                d["pairs"]["pairs"][0].as_u64().ok_or_else(||bad("prior pairs UNKNOWN"))?));
        }
    }
    std::fs::create_dir(output)?;let output=output.canonicalize()?;
    for name in ["diagnostic.r3b","parent-audit.r3b"] {
        neural::write_new(&output.join(name),&std::fs::read(pf.study.join(name))?)?;
    }
    write(&output.join("research-authorization.r3b"),&binary::record!({"scope":if phrase {"USER_AUTHORIZED_SELECTOR_PHRASE_COVERAGE_20260920"}else if cover {"USER_REQUESTED_FIXED_COVER_EXPOSURE_20260920"}else{"USER_REQUESTED_BOUNDED_FOLLOW_THROUGH_20260920"},"prior_study":file_hash(&pf.study.join("study.r3b"))?,"prior_terminal":file_hash(&parent.join(format!("segment-{:04}-finished.r3b",h.len()-1)))?,"prior_decision":decision,"prior_progress":prior_progress,"entry_parity":parity_receipt,"prior_candidate_promoted":false,"new_optimizer_limit":updates,"hypothesis":if phrase {"same COVER7424 parent and2560 updates; one in four cycles exposes owned train view3 wording, same facts/values/order/answers/anchors/math; retained fixed-exposure control"}else{"additional fixed-pool exposure; all mathematical and data policies unchanged"},"generation_limit":12000,"teacher_limit":10000,"active_seconds":21600,"input_limit":12000000,"target_limit":1000000}))?;
    let study=Study {schema,source:source_digest()?,binary:file_hash(&std::env::current_exe()?)?,parent:parent.clone(),
        parent_plan:file_hash(&parent.join("plan.r3b"))?,parent_checkpoint:cp.clone(),parent_file:file_hash(&cp)?,
        parent_model:l.model.weight_hash()?,parent_adam:optimizer_hash(&l.optimizer)?,parent_state:digest(state)?,
        parent_step:state.step,tokenizer:pp.tokenizer.clone(),updates,tiny:pp.tiny,
        inventory:vec![("research-authorization.r3b".into(),file_hash(&output.join("research-authorization.r3b"))?,std::fs::metadata(output.join("research-authorization.r3b"))?.len())],
        diagnostic_hash:file_hash(&output.join("diagnostic.r3b"))?,parent_audit_hash:file_hash(&output.join("parent-audit.r3b"))?,tie_break:vec![arm.into()]};
    write(&output.join("study.r3b"),&study)?;
    let root=output.join(arm);std::fs::create_dir(&root)?;
    for name in ["corpus.r3cor","transfer.r3cor","metadata.r3b","tokenizer.r3b","variants.r3cor","question-variants.r3b","selectors.r3cor","selector-metadata.r3b","paired-samples.r3rows"] {
        neural::write_new(&root.join(name),&std::fs::read(parent.join(name))?)?;
    }
    neural::write_new(&root.join("initial.r3m"),&std::fs::read(&cp)?)?;
    let mut p=pp.clone();p.source=study.source.clone();p.binary=study.binary.clone();
    p.paired=Some(next_tape);
    p.train_order=digest(&p.paired.as_ref().unwrap().rows)?;
    p.initial=study.parent_file.clone();p.initial_weights=study.parent_model.clone();
    p.config.budget_start_step=state.step;p.config.budget_start_tokens=state.consumed_tokens;
    p.config.max_steps=state.step+updates;p.config.max_tokens=state.consumed_tokens+12_000_000;
    p.evaluation=paired_evaluation(state.step,updates,p.tiny);
    let f=p.fork.as_mut().unwrap();f.study=output.clone();f.study_hash=file_hash(&output.join("study.r3b"))?;
    f.parent_policy=digest(&pp)?;f.parent_state=digest(state)?;f.parent_adam=study.parent_adam.clone();
    f.origin_step=state.step;f.origin_input=state.consumed_tokens;f.origin_target=state.target_tokens;
    write(&root.join("plan.r3b"),&p)?;
    if !p.parent_entry(&root.join("initial.r3m"),&checkpoint::load(&root.join("initial.r3m"),Device::Cpu,true)?)? {
        return Err(bad("continuation parent readback"));
    }
    publish_confirmed(&output.join("study-ready.r3b"),&binary::record!({"study":file_hash(&output.join("study.r3b"))?,"C":file_hash(&root.join("plan.r3b"))?,"elapsed_seconds":started.elapsed().as_secs_f64()}))?;
    plan_read(&root)?;
    println!("PAIRED_CONTINUATION parent_step={} max_step={} actual_updates=0 weights={} adam={} policy={}",state.step,p.config.max_steps,study.parent_model,study.parent_adam,digest(&p)?);
    Ok(())
}
fn paired_work(root:&Path)->Result<(u64,u64,u64)> {
    let mut used=(0u64,0u64,0u64);
    let study:Study=read(&root.join("study.r3b"))?;
    for arm in &study.tie_break {
        let a=root.join(arm);
        for i in 0..128 {
            let path=a.join(format!("segment-{i:04}-finished.r3b"));if !path.exists(){break;}
            let _:Segment=read_confirmed(&path)?;
            let r:binary::Value=read(&a.join(format!("segment-{i:04}/train-control.r3b")))?;
            used.0+=r["executed_input_tokens_including_uncommitted"].as_u64().ok_or_else(||bad("paired input UNKNOWN"))?;
            used.1+=r["executed_target_tokens_including_uncommitted"].as_u64().ok_or_else(||bad("paired target UNKNOWN"))?;
            used.2+=r["optimizer_calls"].as_u64().ok_or_else(||bad("paired updates UNKNOWN"))?;
        }
    }
    if used.0>12_000_000||used.1>1_000_000||used.2>if [5,6,7,8,9,10,11,12,13,14,15,16,17,18,19,20].contains(&study.schema) {study.updates as u64}else{4096}{return Err(bad("paired global learning budget"));}
    Ok(used)
}
fn paired_usage(root:&Path)->Result<(f64,usize,usize)> {
    let s=study_read(root)?;if ![4,5,6,7,8,9,10,11,12,13,14,15,16,17,18,19,20].contains(&s.schema){return Err(bad("paired study schema"));}
    let (mut elapsed,mut generation,teacher)=study_usage_bound(root,false,&s)?;
    for arm in &s.tie_break {
        let a=root.join(arm);let p=plan_read(&a)?;
        for h in history(&a,&p)? {
            // A completed, safe quality stop is permitted; execution failures are sticky.
            if h.phase.as_deref()==Some("Failed"){return Err(bad("paired execution failure blocks all arms"));}
            let c:binary::Value=read(&a.join(h.checkpoint).parent().unwrap().join("train-control.r3b"))?;
            let conditions=c["observed_conditions"].as_array().ok_or_else(||bad("paired stop conditions UNKNOWN"))?;
            if conditions.iter().any(|x|x!="TIME_BUDGET"){return Err(bad("paired unsafe command blocks all arms"));}
        }
    }
    let baseline:binary::Value=read(&root.join("parent-audit.r3b"))?;
    if !s.tiny && [4,6,7,8,9,10,11,12,13,14,15,18,19,20].contains(&s.schema) {
        generation+=16;
        elapsed+=baseline["parity"]["control"]["elapsed_seconds"].as_f64().ok_or_else(||bad("parity time UNKNOWN"))?;
    }
    if !s.tiny && [16,17].contains(&s.schema) {
        let authorization:binary::Value=read(&root.join("research-authorization.r3b"))?;
        generation+=16;
        elapsed+=authorization["entry_parity"]["control"]["elapsed_seconds"].as_f64().ok_or_else(||bad("entry parity time UNKNOWN"))?;
    }
    paired_work(root)?;
    Ok((elapsed,generation,teacher))
}
fn paired_report(root:&Path)->Result<binary::Value> {
    let s=study_read(root)?;let usage=paired_usage(root)?;
    let mut results=BTreeMap::new();let mut eligible=vec![];let mut endpoints=true;let mut endpoint_steps=vec![];
    for arm in &s.tie_break {
        let a=root.join(arm);let p=plan_read(&a)?;let h=history(&a,&p)?;
        let end=h.last().ok_or_else(||bad("paired arm NOT_RUN"))?;
        if end.resume||end.phase.as_deref()!=Some("Finished"){return Err(bad("paired arm incomplete"));}
        endpoints&=end.step==p.config.max_steps;
        endpoint_steps.push(end.step);
        let (_,panels)=audit_panels_through(&a,&p,end.step)?;
        let exposure=audit_updates(&a,&p,&h)?;
        let r = paired_decision(&p, &a, end.step)?;
        let saved:binary::Value=read_confirmed(&a.join(format!("paired-{:04}.r3b",end.step)))?;
        if r != saved { return Err(bad("paired decision differs from raw recount")); }
        if r["admissible"]==true && end.step==p.config.max_steps {
            eligible.push((arm.clone(),r["pairs"]["pairs"][0].as_u64().unwrap(),
                r["primary"].as_u64().unwrap(),r["transfer"].as_u64().unwrap()));
        }
        results.insert(arm.clone(),binary::record!({"end":end,"panels":panels,"endpoint":r,"exposure":exposure}));
    }
    eligible.sort_by(|a,b|b.1.cmp(&a.1).then(b.2.cmp(&a.2)).then(b.3.cmp(&a.3)).then(a.0.cmp(&b.0)));
    let mut comparisons = BTreeMap::new();
    let mut both_gain_loss = None;
    if endpoints && s.schema==4 {
        for name in ["train64", "dev512", "transfer128", "selector192"] {
            let step=s.parent_step+s.updates;
            let a=binary::read_value_records(&root.join("SPACED").join(format!("eval-{step:04}-{name}.r3rows")))?;
            let b=binary::read_value_records(&root.join("ADJACENT").join(format!("eval-{step:04}-{name}.r3rows")))?;
            comparisons.insert(name,paired(&a[1..],&b[1..])?);
        }
        if results["SPACED"]["exposure"]["unchanged_five_tasks"] != results["ADJACENT"]["exposure"]["unchanged_five_tasks"]
            || results["SPACED"]["exposure"]["actual_multiset"] != results["ADJACENT"]["exposure"]["actual_multiset"] {
            return Err(bad("paired actual exposure mismatch"));
        }
        let (_, metadata): (Vec<Episode>, Vec<Meta>) = read(&root.join("diagnostic.r3b"))?;
        let mut outcomes = vec![];
        for arm in ["SPACED", "ADJACENT"] {
            let step=s.parent_step+s.updates;
            let originals=binary::read_value_records(&root.join(arm).join(format!("eval-{step:04}-dev512.r3rows")))?;
            let flips=binary::read_value_records(&root.join(arm).join(format!("eval-{step:04}-selector192.r3rows")))?;
            let bits: Vec<_> = metadata.iter().zip(&flips[1..]).map(|(m,r)| {
                originals[1..].iter().any(|o| o["id"].as_str()==m.source_id.as_deref() && o["exact_match"]==true) && r["exact_match"]==true
            }).collect();
            outcomes.push(bits);
        }
        both_gain_loss=Some(binary::record!({"gain":outcomes[0].iter().zip(&outcomes[1]).filter(|(a,b)| !**a && **b).count(),
            "loss":outcomes[0].iter().zip(&outcomes[1]).filter(|(a,b)| **a && !**b).count()}));
    }
    if [19,20].contains(&s.schema) {
        return Ok(binary::record!({"source":s.source,"binary":s.binary,"results":results,"same_budget":endpoints,
            "effect":if s.schema==20 {"RECORD_GROUNDING_RETAINED_COVER4_CONTROL_SEPARATE"}else{"FOUR_VIEW_SCENE_COVERAGE_RETAINED_DIVERSE_CONTROL_SEPARATE"},"usage":usage,"training_usage":paired_work(root)?,
            "result":if s.schema==20 {"RECORD_GROUNDING_CLOSED_CHECK_JOINT_RESULT"}else{"FOUR_VIEW_COVERAGE_CLOSED_CHECK_JOINT_RESULT"},"GOAL1_READY":false}));
    }
    if [16,17].contains(&s.schema) {
        return Ok(binary::record!({"source":s.source,"binary":s.binary,"results":results,"same_budget":endpoints,
            "effect":if s.schema==17 {"WORDING_COVERAGE_RETAINED_SAME_PARENT_EXPOSURE_CONTROL"}else{"FIXED_COVER_EXPOSURE_NOT_MATCHED_COMPUTE_COMPARISON"},"usage":usage,"training_usage":paired_work(root)?,
            "result":if s.schema==17 {"WORDING_COVERAGE_CLOSED_CHECK_JOINT_RESULT"}else{"FIXED_COVER_EXPOSURE_CLOSED_CHECK_JOINT_RESULT"},"GOAL1_READY":false}));
    }
    Ok(binary::record!({"source":s.source,"binary":s.binary,"results":results,"same_budget":endpoints,
        "paired_gain_loss":comparisons,"both_gain_loss":both_gain_loss,"effect":if s.schema==18 {"VALUE_DIVERSITY_RETAINED_VALUE_CONTROL"}else if s.schema==15 {"VALUE_COVERAGE_RETAINED_VALUE_CONTROL_SEPARATE"}else if s.schema==14 {"VALUE_EXPOSURE_RETAINED_FIT_CONTROL_SEPARATE"}else if s.schema==13 {"SEEN_PAIR_FITTING_DIAGNOSTIC_NOT_HELDOUT"}else if s.schema==12 {"RECURRENCE_COVERAGE_RETAINED_REPLAY_CONTROL_SEPARATE"}else if s.schema==11 {"PAIR_RECURRENCE_RETAINED_SIDE_CONTROL_SEPARATE"}else if s.schema==10 {"SIDE_MARGIN_RETAINED_SUM_CONTROL_SEPARATE"}else if s.schema==9 {"PAIR_CONTRAST_RETAINED_COBATCH_CONTROL_SEPARATE"}else if s.schema==8 {"COBATCH_RETAINED_CONTROL_COMPARISON_SEPARATE"}else if s.schema==7 {"LR9E5_RETAINED_CONTROL_COMPARISON_SEPARATE"}else if s.schema==6 {"FIRST_TARGET4_RETAINED_CONTROL_COMPARISON_SEPARATE"}else if s.schema==5 {"SINGLE_ARM_EXPOSURE_NOT_COMPARISON"}else if endpoints {"EQUAL_BUDGET"}else if endpoint_steps[0]==endpoint_steps[1]{"EARLY_STOPS_NO_256_COMPARISON"}else{"UNEQUAL_BUDGET_NOT_ESTABLISHED"},
        "selected_for_continuation":eligible.first().map(|v|&v.0),"usage":usage,"training_usage":paired_work(root)?,
        "result":if s.schema==18 {"VALUE_DIVERSITY_CLOSED_CHECK_JOINT_RESULT"}else if s.schema==15 {"VALUE_COVERAGE_CLOSED_CHECK_JOINT_RESULT"}else if s.schema==14 {"VALUE_EXPOSURE_CLOSED_CHECK_JOINT_RESULT"}else if s.schema==13 {"SEEN_PAIR_FITTING_CLOSED_CHECK_TRAIN_AND_JOINT_SEPARATELY"}else if s.schema==12 {"RECURRENCE_COVERAGE_CLOSED_CHECK_JOINT_RESULT"}else if s.schema==11 {"PAIR_RECURRENCE_CLOSED_CHECK_JOINT_RESULT"}else if s.schema==10 {"SIDE_MARGIN_CLOSED_CHECK_JOINT_RESULT"}else if s.schema==9 {"PAIR_CONTRAST_CLOSED_CHECK_JOINT_RESULT"}else if s.schema==8 {"COBATCH_CLOSED_CHECK_JOINT_RESULT"}else if s.schema==7 {"LR9E5_CLOSED_CHECK_JOINT_RESULT"}else if s.schema==6 {"FIRST_TARGET4_CLOSED_CHECK_JOINT_RESULT"}else if s.schema==5 {"FOLLOW_THROUGH_CLOSED_CHECK_JOINT_RESULT"}else if eligible.is_empty(){"STUDY_COMPLETE_NO_ADMISSIBLE_LEARNER"}else{"G3_ENTRY_PASS"},"GOAL1_READY":false}))
}
fn study_prepare(parent: &Path, output: &Path, selector: bool) -> Result<()> {
    let started = Instant::now();
    let parent = parent.canonicalize()?;
    let p: Plan = read(&parent.join("plan.r3b"))?;
    for (name, h) in [
        ("corpus.r3cor", &p.corpus),
        ("transfer.r3cor", &p.transfer),
        ("metadata.r3b", &p.metadata),
        ("initial.r3m", &p.initial),
    ] {
        if &file_hash(&parent.join(name))? != h {
            return Err(bad("parent frozen input mismatch"));
        }
    }
    if selector
        && p.fork
            .as_ref()
            .is_none_or(|f| f.arm != "P-PHRASE" || f.variants.is_none())
    {
        return Err(bad(
            "selector study requires the preserved P phrase endpoint",
        ));
    }
    if !selector && p.fork.is_some() {
        return Err(bad(
            "study requires original joint parent, not another fork",
        ));
    }
    let mut last = None;
    for i in 0..128 {
        let started = parent.join(format!("segment-{i:04}-started.r3b"));
        if !started.exists() {
            break;
        }
        let path = parent.join(format!("segment-{i:04}-finished.r3b"));
        let s: Segment = if p.schema == Some(2) {
            read_confirmed(&path)?
        } else {
            read(&path)?
        };
        if s.policy != digest(&p)? || s.checkpoint_hash != file_hash(&parent.join(&s.checkpoint))? {
            return Err(bad("historical segment binding"));
        }
        last = Some(s);
    }
    let end = last.ok_or_else(|| bad("missing completed parent"))?;
    if end.resume || end.step != p.config.max_steps || end.stop != "BUDGET_REACHED" {
        return Err(bad("parent is not a closed endpoint"));
    }
    let cp = parent.join(&end.checkpoint);
    let l = checkpoint::load(&cp, Device::Cpu, true)?;
    let s = l
        .manifest
        .training
        .clone()
        .ok_or_else(|| bad("parent Adam state"))?;
    if s.step != end.step
        || s.sampler_state != s.step as u64
        || s.config != p.config
        || s.resume_binding.as_ref() != Some(&p.binding(&s, &l.tokenizer)?)
        || l.tokenizer.id() != p.tokenizer
        || s.config.first_target_weight != 1.
    {
        return Err(bad("parent objective/sampler/tokenizer"));
    }
    let inv = inventory(&parent)?;
    let (raw_count, mut panels) = if selector {
        audit_panels_range(&parent, &p, s.step, s.step)?
    } else {
        audit_panels(&parent, &p)?
    };
    if selector {
        let full = &panels[&format!("eval-{:04}-dev512", s.step)];
        let old: PanelResult =
            read_confirmed(&parent.join(format!("eval-{:04}-screen64.r3b", s.step)))?;
        if old.model != full.model || old.raw_hash != full.raw_hash {
            return Err(bad("parent screen model/raw"));
        }
        panels.insert(format!("eval-{:04}-screen64", s.step), old);
    }
    let eval = checkpoint::load(
        &parent
            .join(end.checkpoint)
            .parent()
            .unwrap()
            .join(format!("step-{:06}", s.step)),
        Device::Cpu,
        true,
    )?;
    if eval.model.weight_hash()? != l.model.weight_hash()?
        || optimizer_hash(&eval.optimizer)? != optimizer_hash(&l.optimizer)?
        || eval.manifest.training != l.manifest.training
    {
        return Err(bad("parent final/evaluated checkpoint differ"));
    }
    let (manifest, train, dev) = data::load(&parent.join("corpus.r3cor"))?;
    let (_, _, transfer) = data::load(&parent.join("transfer.r3cor"))?;
    let (tm, dm, xm): (Vec<Meta>, Vec<Meta>, Vec<Meta>) = read(&parent.join("metadata.r3b"))?;
    let (diagnostic_source, diagnostic_meta) = if p.tiny {
        subset(&transfer, &xm, 1)
    } else {
        (transfer.clone(), xm.clone())
    };
    let mut diagnostic = diagnostic_source
        .iter()
        .map(|e| {
            let i = question_intent(&e.request.input)?.2;
            changed_question(e, familiar(i))
        })
        .collect::<Result<Vec<_>>>()?;
    let held_suffixes: std::collections::BTreeSet<_> = transfer
        .iter()
        .map(|e| e.request.input.splitn(3, ' ').nth(2).unwrap().to_owned())
        .collect();
    let mut diagnostic_meta = diagnostic_meta;
    if selector {
        (diagnostic, diagnostic_meta) = selector_panel(&dev, &dm, p.tiny)?;
    }
    let variants = if selector {
        let f = p.fork.as_ref().unwrap();
        if f.variant_metadata.as_ref() != Some(&file_hash(&parent.join("question-variants.r3b"))?) {
            return Err(bad("parent phrase metadata"));
        }
        verified_corpus(&parent.join("variants.r3cor"), f.variants.as_ref().unwrap())?.train
    } else {
        train
            .iter()
            .zip(&tm)
            .map(|(e, m)| {
                let i = question_intent(&e.request.input)?.2;
                let h = u64::from_str_radix(&neural::hash(m.base.as_bytes())[..8], 16)
                    .map_err(|_| bad("base digest"))?;
                let suffix = phrases(i)[h as usize % 2];
                if held_suffixes.contains(suffix) {
                    return Err(bad("heldout suffix copied into train"));
                }
                changed_question(e, suffix)
            })
            .collect::<Result<Vec<_>>>()?
    };
    if variants.len() != train.len() {
        return Err(bad("unaligned P variants"));
    }
    validate_framed(
        &train.iter().chain(&variants).cloned().collect::<Vec<_>>(),
        &l.tokenizer,
    )?;
    validate_framed(&diagnostic, &l.tokenizer)?;
    let mut alternate = vec![false; train.len()];
    for b in 0..8 {
        let mut bases: Vec<_> = tm
            .iter()
            .filter(|m| m.bucket == b)
            .map(|m| m.base.clone())
            .collect();
        bases.sort();
        bases.dedup();
        bases.sort_by_key(|base| neural::hash(format!("29/{base}").as_bytes()));
        let chosen: std::collections::BTreeSet<_> = bases.iter().take(bases.len() / 2).collect();
        for (i, m) in tm.iter().enumerate().filter(|(_, m)| m.bucket == b) {
            alternate[i] = chosen.contains(&m.base);
        }
    }
    if selector {
        alternate = p.fork.as_ref().unwrap().alternate_first.clone();
    }
    let mut flip_first = vec![None; train.len()];
    if selector {
        for bucket in 2..=4 {
            for style in [false, true] {
                let mut bases = tm
                    .iter()
                    .enumerate()
                    .filter(|(i, m)| m.bucket == bucket && alternate[*i] == style)
                    .map(|(_, m)| m.base.clone())
                    .collect::<Vec<_>>();
                bases.sort();
                bases.dedup();
                bases.sort_by_key(|b| neural::hash(format!("selector/29/{b}").as_bytes()));
                let half = bases[..bases.len() / 2]
                    .iter()
                    .collect::<std::collections::BTreeSet<_>>();
                for (i, m) in tm
                    .iter()
                    .enumerate()
                    .filter(|(i, m)| m.bucket == bucket && alternate[*i] == style)
                {
                    flip_first[i] = Some(half.contains(&m.base));
                }
            }
        }
    }
    let mut selector_rows = vec![];
    let mut selector_meta = vec![];
    if selector {
        for (style, es) in [&train, &variants].into_iter().enumerate() {
            for (e, m) in es.iter().zip(&tm) {
                let (mut row, mut meta) = if (2..=4).contains(&m.bucket) {
                    let pair = flip_selection(e, m)?;
                    let (back, _) = flip_selection(&pair.0, &pair.1)?;
                    if digest(&back.request)? != digest(&e.request)? || back.answer != e.answer {
                        return Err(bad("train selector involution"));
                    }
                    pair
                } else {
                    (e.clone(), m.clone())
                };
                row.id = format!("{}/selector-style-{style}", e.id);
                meta.id = row.id.clone();
                meta.source_id = Some(e.id.clone());
                selector_rows.push(row);
                selector_meta.push(meta);
            }
        }
        validate_framed(
            &train
                .iter()
                .chain(&variants)
                .chain(&selector_rows)
                .cloned()
                .collect::<Vec<_>>(),
            &l.tokenizer,
        )?;
    }
    std::fs::create_dir(output)?;
    let output = output.canonicalize()?;
    let peer = if selector && !p.tiny {
        let path = parent
            .parent()
            .ok_or_else(|| bad("parent study directory"))?
            .join("C-REPEAT");
        let plan: Plan = read(&path.join("plan.r3b"))?;
        if plan.corpus != p.corpus
            || plan.transfer != p.transfer
            || plan.tokenizer != p.tokenizer
            || plan.config.max_steps != s.step
        {
            return Err(bad("historical C/P endpoint mismatch"));
        }
        let before = inventory(&path)?;
        let (count, result) = audit_panels_range(&path, &plan, s.step, s.step)?;
        if before != inventory(&path)? {
            return Err(bad("historical peer changed"));
        }
        Some(binary::record!({"path":path,"raw_count":count,"panels":result,"inventory":before}))
    } else {
        None
    };
    write(
        &output.join("parent-audit.r3b"),
        &binary::record!({"raw_count":raw_count,"panels":panels,"peer":peer,"parent_policy":digest(&p)?,"historical_receipts":"READ_ONLY_VERIFIED_NOT_REISSUED"}),
    )?;
    write(
        &output.join("diagnostic.r3b"),
        &(diagnostic.clone(), diagnostic_meta),
    )?;
    let study = Study {
        schema: if selector { 3 } else { 2 },
        source: source_digest()?,
        binary: file_hash(&std::env::current_exe()?)?,
        parent: parent.clone(),
        parent_plan: file_hash(&parent.join("plan.r3b"))?,
        parent_checkpoint: cp.clone(),
        parent_file: file_hash(&cp)?,
        parent_model: l.model.weight_hash()?,
        parent_adam: optimizer_hash(&l.optimizer)?,
        parent_state: digest(&s)?,
        parent_step: s.step,
        tokenizer: p.tokenizer.clone(),
        updates: if p.tiny { 2 } else { 2048 },
        tiny: p.tiny,
        inventory: inv,
        diagnostic_hash: file_hash(&output.join("diagnostic.r3b"))?,
        parent_audit_hash: file_hash(&output.join("parent-audit.r3b"))?,
        tie_break: if selector {
            vec!["C-KEEP".into(), "S-SELECT".into()]
        } else {
            vec!["C-REPEAT".into(), "P-PHRASE".into()]
        },
    };
    write(&output.join("study.r3b"), &study)?;
    for arm in study.tie_break.iter().map(String::as_str) {
        let root = output.join(arm);
        std::fs::create_dir(&root)?;
        for name in [
            "corpus.r3cor",
            "transfer.r3cor",
            "metadata.r3b",
            "tokenizer.r3b",
        ] {
            let bytes = std::fs::read(parent.join(name))?;
            neural::write_new(&root.join(name), &bytes)?;
        }
        neural::write_new(&root.join("initial.r3m"), &std::fs::read(&cp)?)?;
        let variant_hash = if arm == "P-PHRASE" || selector {
            let c = data::native::from_episodes(manifest.clone(), variants.clone(), dev.clone())?;
            data::native::write(&root.join("variants.r3cor"), &c, true)?;
            let aligned=variants.iter().zip(&tm).enumerate().map(|(n,(e,m))|->Result<binary::Value>{let(_,_,i)=question_intent(&e.request.input)?;
                Ok(binary::record!({"source_episode_id":e.id,"base":m.base,"intent":i,"variant_id":phrases(i).iter().position(|t|e.request.input.ends_with(t)).ok_or_else(||bad("variant id"))?,"alternate_first_epoch":alternate[n]}))}).collect::<Result<Vec<_>>>()?;
            write(&root.join("question-variants.r3b"), &aligned)?;
            Some(file_hash(&root.join("variants.r3cor"))?)
        } else {
            None
        };
        let variant_metadata = if variant_hash.is_some() {
            Some(file_hash(&root.join("question-variants.r3b"))?)
        } else {
            None
        };
        let (selector_hash, selector_metadata) = if arm == "S-SELECT" {
            let c =
                data::native::from_episodes(manifest.clone(), selector_rows.clone(), dev.clone())?;
            data::native::write(&root.join("selectors.r3cor"), &c, true)?;
            let owned = data::native::read(&root.join("selectors.r3cor"))?;
            for e in &owned.train {
                if resolve(&e.request)? != e.answer {
                    return Err(bad("native selector label verification"));
                }
            }
            write(&root.join("selector-metadata.r3b"), &selector_meta)?;
            (
                Some(file_hash(&root.join("selectors.r3cor"))?),
                Some(file_hash(&root.join("selector-metadata.r3b"))?),
            )
        } else {
            (None, None)
        };
        let mut plan = p.clone();
        plan.schema = Some(2);
        plan.source = study.source.clone();
        plan.binary = study.binary.clone();
        plan.initial = study.parent_file.clone();
        plan.initial_weights = study.parent_model.clone();
        plan.config.budget_start_step = s.step;
        plan.config.budget_start_tokens = s.consumed_tokens;
        plan.config.max_steps = s.step + study.updates;
        plan.config.max_tokens = s.consumed_tokens + 9_000_000;
        plan.config.lr = 3e-5;
        plan.config.warmup = 0;
        plan.config.validate_every = if p.tiny { 2 } else { 256 };
        plan.evaluation = fork_evaluation(s.step, study.updates, p.tiny);
        if selector {
            plan.evaluation.generation_limit = 4608;
        }
        plan.fork = Some(Fork {
            study: output.clone(),
            study_hash: file_hash(&output.join("study.r3b"))?,
            arm: arm.into(),
            parent_policy: digest(&p)?,
            parent_state: digest(&s)?,
            parent_adam: study.parent_adam.clone(),
            origin_step: s.step,
            origin_input: s.consumed_tokens,
            origin_target: s.target_tokens,
            original_corpus: p.corpus.clone(),
            tokenizer_training_hash: l.tokenizer.train_hash.clone(),
            variants: variant_hash,
            variant_metadata,
            alternate_first: alternate.clone(),
            selector: selector_hash,
            selector_metadata,
            flip_first: if selector { flip_first.clone() } else { vec![] },
            constant_lr: 3e-5,
            target_limit: 1_000_000,
        });
        let original = samples(&train, &l.tokenizer, 512)?;
        let variant = samples(&variants, &l.tokenizer, 512)?;
        let mut all = original.clone();
        all.extend(plan.additional_samples(&root, &l.tokenizer)?);
        let mut counts = vec![[0usize; 2]; train.len()];
        let mut epoch_counts = [[[0usize; 2]; 8]; 2];
        let mut flips = 0usize;
        let mut flip_counts = vec![[0usize; 2]; train.len()];
        let mut strata: BTreeMap<(usize, usize, bool, usize), [usize; 2]> = BTreeMap::new();
        let mut input = 0usize;
        let mut targets = 0usize;
        for step in s.step..plan.config.max_steps {
            let ids = plan.draw(step);
            let draw = plan.training_draw(step);
            for (bucket, (&i, &j)) in ids.iter().zip(&draw).enumerate() {
                let selected = (j / train.len()) % 2;
                let flipped = usize::from(j >= 2 * train.len());
                flips += flipped;
                flip_counts[i][flipped] += 1;
                if (2..=4).contains(&bucket) {
                    strata
                        .entry((
                            (step - s.step) / plan.order[0].len(),
                            bucket,
                            selected == 1,
                            tm[i].view,
                        ))
                        .or_default()[flipped] += 1;
                }
                counts[i][selected] += 1;
                epoch_counts[(step - s.step) / plan.order[0].len()][bucket][selected] += 1;
                let a = &original[i];
                let b = &variant[i];
                if a.tokens[a.response_start..] != b.tokens[b.response_start..] {
                    return Err(bad("variant changed target tokens"));
                }
                let row = &all[j];
                input += row.tokens.len() - 1;
                targets += row.tokens.len() - row.response_start;
            }
        }
        if !p.tiny
            && counts.iter().any(|c| {
                *c != if arm == "P-PHRASE" || selector {
                    [1, 1]
                } else {
                    [2, 0]
                }
            })
        {
            return Err(bad("two-epoch exposure contract"));
        }
        if !p.tiny
            && epoch_counts.iter().flatten().any(|c| {
                *c != if arm == "P-PHRASE" || selector {
                    [512, 512]
                } else {
                    [1024, 0]
                }
            })
        {
            return Err(bad("per-bucket epoch wording balance"));
        }
        if !p.tiny
            && arm == "S-SELECT"
            && (flips != 3072
                || strata.values().any(|x| x[0] != x[1])
                || flip_counts.iter().zip(&tm).any(|(c, m)| {
                    *c != if (2..=4).contains(&m.bucket) {
                        [1, 1]
                    } else {
                        [2, 0]
                    }
                }))
        {
            return Err(bad("selector balanced exposure contract"));
        }
        if input > 9_000_000 || targets > 1_000_000 {
            return Err(bad("registered token budget insufficient"));
        }
        write(&root.join("plan.r3b"), &plan)?;
        write(
            &root.join("registration.r3b"),
            &binary::record!({"policy":digest(&plan)?,"counts":counts,"epoch_variant_counts":epoch_counts,"selector_draws":flips,"selector_counts":flip_counts,"selector_strata":strata.into_iter().collect::<Vec<_>>(),"planned_input":input,"planned_target":targets,"origin_step":s.step,"end_step":plan.config.max_steps,"lr_bits":3e-5f64.to_bits(),"optimizer_calls":0}),
        )?;
        println!(
            "FORK_REGISTERED arm={arm} origin={} end={} planned_input={input} planned_target={targets} parent={} optimizer=0",
            s.step, plan.config.max_steps, study.parent_model
        );
    }
    if inventory(&parent)? != study.inventory {
        return Err(bad("parent changed during read-only registration"));
    }
    publish_confirmed(
        &output.join("study-ready.r3b"),
        &binary::record!({"study":file_hash(&output.join("study.r3b"))?,"C":file_hash(&output.join(&study.tie_break[0]).join("plan.r3b"))?,"P":file_hash(&output.join(&study.tie_break[1]).join("plan.r3b"))?,"elapsed_seconds":started.elapsed().as_secs_f64()}),
    )?;
    println!(
        "PARENT_RAW_RESCORE={raw_count} VERIFIED initial_weights_Adam=SHARED new_updates=0 diagnostic_planned={}",
        diagnostic.len()
    );
    Ok(())
}
fn study_read(root: &Path) -> Result<Study> {
    study_read_bound(root, None)
}
fn study_read_bound(root: &Path, frozen_executable: Option<&Path>) -> Result<Study> {
    let s: Study = read(&root.join("study.r3b"))?;
    let names = match s.schema {
        2 => vec!["C-REPEAT", "P-PHRASE"],
        3 => vec!["C-KEEP", "S-SELECT"],
        4 => vec!["SPACED", "ADJACENT"],
        5..=7 => vec!["ADJACENT"],
        8 => vec!["COBATCH"],
        9 => vec!["CONTRAST"],
        10 => vec!["SIDE"],
        11 => vec!["REPLAY"],
        12 => vec!["WIDE"],
        13 => vec!["FIT"],
        14 => vec!["VALUE"],
        15..=17 => vec!["COVER"],
        18 => vec!["DIVERSE"],
        19 => vec!["COVER4"],
        20 => vec!["GROUND"],
        _ => return Err(bad("study schema")),
    };
    if s.tie_break != names {
        return Err(bad("study arm identity"));
    }
    let ready: binary::Value = read_confirmed(&root.join("study-ready.r3b"))?;
    if ready["study"] != file_hash(&root.join("study.r3b"))?
        || ready["C"] != file_hash(&root.join(&s.tie_break[0]).join("plan.r3b"))?
        || (s.tie_break.len()==2 && ready["P"] != file_hash(&root.join(&s.tie_break[1]).join("plan.r3b"))?)
        || (s.tie_break.len()==1 && !ready["P"].is_null())
    {
        return Err(bad("study registration incomplete/changed"));
    }
    let executable = frozen_executable.map_or_else(std::env::current_exe, |p| Ok(p.to_path_buf()))?;
    if ![2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13,14,15,16,17,18,19,20].contains(&s.schema)
        || (frozen_executable.is_none() && s.source != source_digest()?)
        || s.binary != file_hash(&executable)?
        || s.parent_plan != file_hash(&s.parent.join("plan.r3b"))?
        || s.parent_file != file_hash(&s.parent_checkpoint)?
        || s.diagnostic_hash != file_hash(&root.join("diagnostic.r3b"))?
        || s.parent_audit_hash != file_hash(&root.join("parent-audit.r3b"))?
    {
        return Err(bad("frozen study binding"));
    }
    if [5,6,7,8,9,10,11,12,13,14,15,16,17,18,19,20].contains(&s.schema)
        && (s.inventory.len()!=1 || s.inventory[0].0!=Path::new("research-authorization.r3b")
            || s.inventory[0].1!=file_hash(&root.join("research-authorization.r3b"))?
            || s.inventory[0].2!=std::fs::metadata(root.join("research-authorization.r3b"))?.len()) {
            return Err(bad("explicit follow-through registration missing"));
    }
    Ok(s)
}
fn paired(a: &[binary::Value], b: &[binary::Value]) -> Result<binary::Value> {
    if a.len() != b.len() {
        return Err(bad("paired denominator"));
    }
    let (mut gain, mut loss) = (0, 0);
    let mut body = [0usize; 2];
    let mut citation = [0usize; 2];
    for (a, b) in a.iter().zip(b) {
        if a["id"] != b["id"]
            || a["expected"] != b["expected"]
            || a["generated_evidence"] != b["generated_evidence"]
        {
            return Err(bad("paired source changed"));
        }
        gain += usize::from(a["exact_match"] != true && b["exact_match"] == true);
        loss += usize::from(a["exact_match"] == true && b["exact_match"] != true);
        let body_ok = |r: &binary::Value| {
            r["actual"]
                .as_str()
                .zip(r["expected"].as_str())
                .is_some_and(|(x, y)| x.split(" [event:").next() == y.split(" [event:").next())
        };
        let citation_ok = |r: &binary::Value| {
            r["actual"]
                .as_str()
                .zip(r["expected"].as_str())
                .is_some_and(|(x, y)| citations(x).ok() == citations(y).ok())
        };
        body[0] += usize::from(!body_ok(a) && body_ok(b));
        body[1] += usize::from(body_ok(a) && !body_ok(b));
        citation[0] += usize::from(!citation_ok(a) && citation_ok(b));
        citation[1] += usize::from(citation_ok(a) && !citation_ok(b));
    }
    Ok(
        binary::record!({"paired":a.len(),"gain":gain,"loss":loss,"body_gain_loss":body,"citation_gain_loss":citation}),
    )
}
fn selector_pairs(
    original: &[binary::Value],
    flipped: &[binary::Value],
    es: &[Episode],
    ms: &[Meta],
) -> Result<binary::Value> {
    if flipped.len() != es.len() || es.len() != ms.len() {
        return Err(bad("selector pair denominator"));
    }
    let mut outcomes = [0usize; 4];
    let mut same = 0;
    let mut citation = [0usize; 4];
    let mut by_bucket: BTreeMap<usize, [usize; 4]> = BTreeMap::new();
    let mut base: BTreeMap<&str, (usize, usize)> = BTreeMap::new();
    let mut original_full = 0;
    let mut flip_full = 0;
    let mut original_body = 0;
    let mut flip_body = 0;
    let mut original_citation = 0;
    for ((row, e), m) in flipped.iter().zip(es).zip(ms) {
        let source = m
            .source_id
            .as_ref()
            .ok_or_else(|| bad("selector source case"))?;
        let old = original
            .iter()
            .find(|r| r["id"] == *source)
            .ok_or_else(|| bad("selector original missing"))?;
        if row["id"] != e.id || row["expected"] != e.answer {
            return Err(bad("selector paired content"));
        }
        let a = old["exact_match"] == true;
        let b = row["exact_match"] == true;
        let index = match (a, b) {
            (true, true) => 0,
            (true, false) => 1,
            (false, true) => 2,
            (false, false) => 3,
        };
        outcomes[index] += 1;
        by_bucket.entry(m.bucket).or_default()[index] += 1;
        original_full += usize::from(a);
        flip_full += usize::from(b);
        same += usize::from(old["actual"].is_string() && old["actual"] == row["actual"]);
        let fields = |v: &binary::Value| {
            v.as_str()
                .unwrap_or("")
                .split_once(" [event:")
                .map(|(b, _)| b.to_owned())
        };
        original_body += usize::from(
            fields(&old["actual"]).is_some() && fields(&old["actual"]) == fields(&old["expected"]),
        );
        flip_body += usize::from(
            fields(&row["actual"]).is_some() && fields(&row["actual"]) == fields(&row["expected"]),
        );
        original_citation += usize::from(
            old["actual"].as_str().and_then(|s| citations(s).ok())
                == old["expected"].as_str().and_then(|s| citations(s).ok()),
        );
        let got = row["actual"]
            .as_str()
            .and_then(|s| citations(s).ok())
            .unwrap_or_default();
        let expected = citations(&e.answer)?;
        let c = if got.is_empty() {
            3
        } else if got == expected {
            0
        } else if got
            .iter()
            .all(|id| e.request.evidence.items.iter().any(|r| r.event_id == *id))
        {
            1
        } else {
            2
        };
        citation[c] += 1;
        let x = base.entry(&m.base).or_default();
        x.0 += 1;
        x.1 += usize::from(a && b);
    }
    Ok(
        binary::record!({"planned":es.len(),"pair_order":["both","original_only","flipped_only","neither"],"pairs":outcomes,"bucket_pairs":by_bucket,"same_output":same,
        "original_full":original_full,"flipped_full":flip_full,"original_body":original_body,"flipped_body":flip_body,"original_citation":original_citation,
        "flipped_citation_order":["selected","other_provided","invented","none"],"flipped_citations":citation,"both_base4":base.values().filter(|&&(n,k)|n==4&&k==4).count()}),
    )
}
fn fragments(tok: &ByteBpe, es: &[Episode]) -> Result<binary::Value> {
    let ids: std::collections::BTreeSet<_> = (0..tok.vocab_size() as u32)
        .filter(|&i| {
            tok.decode_bytes(&[i])
                .is_ok_and(|b| std::str::from_utf8(&b).is_err())
        })
        .collect();
    let mut prompt = 0;
    let mut answer = 0;
    let mut pt = 0;
    let mut at = 0;
    for e in es {
        let p = tok.prepare(&e.request, 2048, "wording-diagnostic")?;
        let a = tok.encode(e.answer.as_bytes())?;
        pt += p.token_ids.len();
        at += a.len();
        prompt += p.token_ids.iter().filter(|i| ids.contains(i)).count();
        answer += a.iter().filter(|i| ids.contains(i)).count();
    }
    Ok(
        binary::record!({"examples":es.len(),"prompt_tokens":pt,"answer_tokens":at,"fragment_prompt":prompt,"fragment_answer":answer,"fragment_pieces":ids.len()}),
    )
}
fn study_observe(root: &Path) -> Result<()> {
    let s = study_read(root)?;
    if root.join("observation-started.r3b").exists() {
        return Err(bad("observation already attempted; no automatic retry"));
    }
    let p: Plan = read(&s.parent.join("plan.r3b"))?;
    let (_, train, dev) = data::load(&s.parent.join("corpus.r3cor"))?;
    let (_, _, transfer) = data::load(&s.parent.join("transfer.r3cor"))?;
    let (_, dm, _): (Vec<Meta>, Vec<Meta>, Vec<Meta>) = read(&s.parent.join("metadata.r3b"))?;
    let (diagnostic, xm): (Vec<Episode>, Vec<Meta>) = read(&root.join("diagnostic.r3b"))?;
    let tok = ByteBpe::load(&s.parent.join("tokenizer.r3b"))?;
    let (probe, pm) = subset(&dev, &dm, if p.tiny { 1 } else { 2 });
    let mut control = recovery::RunControl::command(false)?;
    control.set_call_limits(if s.schema == 3 { 208 } else { 192 }, 6000);
    write(
        &root.join("observation-started.r3b"),
        &binary::record!({"study":file_hash(&root.join("study.r3b"))?,"parity_ids":probe.iter().map(|e|&e.id).collect::<Vec<_>>(),"generation_limit":if s.schema==3{208}else{192},"teacher_policy":"required per returned case; separate durable records"}),
    )?;
    let result = (|| -> Result<binary::Value> {
        let parity = evaluate_panel(
            &p,
            root,
            &s.parent_checkpoint,
            s.parent_step,
            "parity",
            &probe,
            &pm,
            &mut control,
        )?;
        let original = binary::read_value_records(
            &s.parent
                .join(format!("eval-{:04}-dev512.r3rows", s.parent_step)),
        )?;
        let current = binary::read_value_records(
            &root.join(format!("eval-{:04}-parity.r3rows", s.parent_step)),
        )?;
        for row in &current[1..] {
            let old = original[1..]
                .iter()
                .find(|r| r["id"] == row["id"])
                .ok_or_else(|| bad("parity missing case"))?;
            for field in [
                "raw_tokens",
                "actual",
                "error",
                "finish_reason",
                "eos_index",
            ] {
                if old[field] != row[field] {
                    return Err(bad("same-weight output parity changed"));
                }
            }
        }
        let diagnostic_name = if s.schema == 3 {
            "selector192"
        } else {
            "familiar-wording"
        };
        let diagnostic_result = evaluate_panel(
            &p,
            root,
            &s.parent_checkpoint,
            s.parent_step,
            diagnostic_name,
            &diagnostic,
            &xm,
            &mut control,
        )?;
        let orig = binary::read_value_records(&s.parent.join(format!(
            "eval-{:04}-{}.r3rows",
            s.parent_step,
            if s.schema == 3 {
                "dev512"
            } else {
                "transfer128"
            }
        )))?;
        let altered = binary::read_value_records(&root.join(format!(
            "eval-{:04}-{diagnostic_name}.r3rows",
            s.parent_step
        )))?;
        let comparison = if s.schema == 3 {
            selector_pairs(&orig[1..], &altered[1..], &diagnostic, &xm)?
        } else {
            paired(&orig[1..], &altered[1..])?
        };
        Ok(
            binary::record!({"parity":parity,"diagnostic_label":if s.schema==3{"SELECTOR_DEVELOPMENT_DIAGNOSTIC"}else{"FAMILIAR_WORDING_DIAGNOSTIC_NOT_PRODUCT_SCORE"},"diagnostic":diagnostic_result,"paired":comparison,
            "fragments":{"train":fragments(&tok,&train)?,"primary":fragments(&tok,&dev)?,"transfer":fragments(&tok,&transfer)?,"familiar":fragments(&tok,&diagnostic)?}}),
        )
    })();
    if let Err(e) = &result {
        control.classify_error(e);
    }
    let _ = control.seal_terminal();
    let record = binary::record!({"study":file_hash(&root.join("study.r3b"))?,"start":file_hash(&root.join("observation-started.r3b"))?,"result":result.as_ref().ok(),"error":result.as_ref().err().map(ToString::to_string),"control":control.receipt()});
    publish_confirmed(&root.join("observation-finished.r3b"), &record)?;
    println!("STUDY_OBSERVATION {record}");
    if inventory(&s.parent)? != s.inventory {
        return Err(bad("parent changed during observation"));
    }
    control.stop_result().and(result.map(|_| ()))
}
fn study_usage(root: &Path, require_observation: bool) -> Result<(f64, usize, usize)> {
    let s = study_read(root)?;
    study_usage_bound(root, require_observation, &s)
}
fn study_usage_bound(
    root: &Path,
    require_observation: bool,
    s: &Study,
) -> Result<(f64, usize, usize)> {
    let ready: binary::Value = read_confirmed(&root.join("study-ready.r3b"))?;
    let mut elapsed = ready["elapsed_seconds"]
        .as_f64()
        .ok_or_else(|| bad("prepare time UNKNOWN"))?;
    let mut generation = 0;
    let mut teacher = 0;
    if require_observation || root.join("observation-started.r3b").exists() {
        let r: binary::Value = read_confirmed(&root.join("observation-finished.r3b"))?;
        if !r["error"].is_null()
            || r["study"] != file_hash(&root.join("study.r3b"))?
            || r["start"] != file_hash(&root.join("observation-started.r3b"))?
            || r["control"]["terminal_reason"] != "COMPLETED"
        {
            return Err(bad("study observation failed/incomplete; blocked"));
        }
        let parent: Plan = read(&s.parent.join("plan.r3b"))?;
        let corpus = verified_corpus(&s.parent.join("corpus.r3cor"), &parent.corpus)?;
        let (_, dm, _) = verified_metadata(&s.parent, &parent)?;
        let tok = ByteBpe::load(&s.parent.join("tokenizer.r3b"))?;
        let (es, ms) = subset(&corpus.validation, &dm, if s.tiny { 1 } else { 2 });
        let parity = audit_panel(root, &parent, s.parent_step, "parity", &es, &ms, &tok)?;
        let (es, ms): (Vec<Episode>, Vec<Meta>) = read(&root.join("diagnostic.r3b"))?;
        let diag = audit_panel(
            root,
            &parent,
            s.parent_step,
            if s.schema == 3 {
                "selector192"
            } else {
                "familiar-wording"
            },
            &es,
            &ms,
            &tok,
        )?;
        if r["result"]["parity"] != binary::to_value(parity)?
            || r["result"]["diagnostic"] != binary::to_value(diag)?
        {
            return Err(bad("observation raw summary"));
        }
        elapsed += r["control"]["elapsed_seconds"]
            .as_f64()
            .ok_or_else(|| bad("observation time UNKNOWN"))?;
        generation = r["control"]["generation_calls"]
            .as_u64()
            .ok_or_else(|| bad("generation UNKNOWN"))? as usize;
        teacher = r["control"]["teacher_calls"]
            .as_u64()
            .ok_or_else(|| bad("teacher UNKNOWN"))? as usize;
    }
    for arm in &s.tie_break {
        let a = root.join(arm);
        let p = plan_read_bound(&a, &s.source, &s.binary)?;
        for segment in history(&a, &p)? {
            if require_observation && segment.phase.as_deref() == Some("Failed") {
                return Err(bad("study arm command failed; study blocked"));
            }
            elapsed += segment.elapsed;
            generation += segment.generations;
            teacher += segment.teachers;
        }
    }
    Ok((elapsed, generation, teacher))
}
fn study_report(root: &Path, frozen_executable: Option<&Path>) -> Result<()> {
    let s = study_read_bound(root, frozen_executable)?;
    let (elapsed, generation, teacher) = study_usage_bound(root, false, &s)?;
    let mut results = BTreeMap::new();
    let mut eligible = vec![];
    let mut all_endpoints = true;
    for arm in &s.tie_break {
        let a = root.join(arm);
        let p = plan_read_bound(&a, &s.source, &s.binary)?;
        let h = history(&a, &p)?;
        let last = h.last().ok_or_else(|| bad("arm has not run"))?;
        if last.resume || last.phase.as_deref() != Some("Finished") {
            return Err(bad("arm incomplete"));
        }
        all_endpoints &= last.step == p.config.max_steps;
        let (rows, panels) = audit_panels_through(&a, &p, last.step)?;
        let exposure = audit_updates(&a, &p, &h)?;
        let full = panels.get(&format!("eval-{:04}-dev512", p.config.max_steps));
        let transfer = panels.get(&format!("eval-{:04}-transfer128", p.config.max_steps));
        if let (Some(f), Some(t)) = (full, transfer)
            && f.exact >= p.evaluation.primary_min
            && f.buckets.iter().all(|&k| k >= p.evaluation.bucket_min)
            && t.exact >= p.evaluation.transfer_min
            && f.errors == 0
            && t.errors == 0
        {
            eligible.push((
                arm.clone(),
                f.buckets.iter().min().copied().unwrap_or(0),
                f.exact,
                t.exact,
            ));
        }
        results.insert(arm.clone(),binary::record!({"panels":panels,"raw_rows":rows,"exposure":exposure,"last_step":last.step,"new_updates":last.step-s.parent_step,"durable":last.checkpoint,"stop":last.stop}));
    }
    let mut comparisons = BTreeMap::new();
    let c_arm = &s.tie_break[0];
    let p_arm = &s.tie_break[1];
    if all_endpoints
        && (results[c_arm]["exposure"]["case_order"] != results[p_arm]["exposure"]["case_order"]
            || (s.schema == 2
                && results[c_arm]["exposure"]["target_order"]
                    != results[p_arm]["exposure"]["target_order"])
            || (s.schema == 3
                && (results[c_arm]["exposure"]["phrase_order"]
                    != results[p_arm]["exposure"]["phrase_order"]
                    || results[c_arm]["exposure"]["unchanged_five_tasks"]
                        != results[p_arm]["exposure"]["unchanged_five_tasks"])))
    {
        return Err(bad("actual C/P case/target exposures differ"));
    }
    for panel in ["train64", "dev512", "transfer128"] {
        if !all_endpoints {
            continue;
        }
        let parent = binary::read_value_records(
            &s.parent
                .join(format!("eval-{:04}-{panel}.r3rows", s.parent_step)),
        )?;
        let c = binary::read_value_records(&root.join(c_arm).join(format!(
            "eval-{:04}-{panel}.r3rows",
            s.parent_step + s.updates
        )))?;
        let p = binary::read_value_records(&root.join(p_arm).join(format!(
            "eval-{:04}-{panel}.r3rows",
            s.parent_step + s.updates
        )))?;
        comparisons.insert(panel,binary::record!({"control":c_arm,"treatment":p_arm,"parent_to_control":paired(&parent[1..],&c[1..])?,"parent_to_treatment":paired(&parent[1..],&p[1..])?,"control_to_treatment":paired(&c[1..],&p[1..])?}));
    }
    let mut selector_comparison = BTreeMap::new();
    if s.schema == 3 && all_endpoints {
        let (es, ms): (Vec<Episode>, Vec<Meta>) = read(&root.join("diagnostic.r3b"))?;
        for arm in &s.tie_break {
            let a = root.join(arm);
            let step = s.parent_step + s.updates;
            let original =
                binary::read_value_records(&a.join(format!("eval-{step:04}-dev512.r3rows")))?;
            let flipped =
                binary::read_value_records(&a.join(format!("eval-{step:04}-selector192.r3rows")))?;
            selector_comparison.insert(
                arm,
                selector_pairs(&original[1..], &flipped[1..], &es, &ms)?,
            );
        }
    }
    eligible.sort_by(|a, b| {
        b.1.cmp(&a.1)
            .then(b.2.cmp(&a.2))
            .then(b.3.cmp(&a.3))
            .then(a.0.cmp(&b.0))
    });
    println!(
        "STUDY_REPORT {}",
        binary::record!({"execution_source":s.source,"execution_binary":s.binary,"report_source":source_digest()?,"report_binary":file_hash(&std::env::current_exe()?)?,"read_only_frozen_execution":frozen_executable.is_some(),"results":results,"paired":comparisons,"selector":selector_comparison,"arm_order":s.tie_break,"candidate":eligible.first().map(|x|&x.0),"generation":generation,"teacher":teacher,"active_seconds":elapsed,"result":if !all_endpoints{"STUDY_INCONCLUSIVE_UNEQUAL_BUDGET"}else if eligible.is_empty(){"STUDY_COMPLETE_QUALITY_FAIL"}else{"DEVELOPMENT_PASS_FINAL_NOT_RUN"},"FINAL200":"NOT_OPENED","GOAL1_READY":false})
    );
    if inventory(&s.parent)? != s.inventory {
        return Err(bad("parent preservation inventory mismatch"));
    }
    Ok(())
}
fn audit_updates(root: &Path, p: &Plan, h: &[Segment]) -> Result<binary::Value> {
    let f = p
        .fork
        .as_ref()
        .ok_or_else(|| bad("update audit requires explicit fork"))?;
    let (_, episodes, _) = p.training_corpus(&root.join("corpus.r3cor"))?;
    let tok = ByteBpe::load(&root.join("tokenizer.r3b"))?;
    let mut framed = samples(&episodes, &tok, p.config.seq_len)?;
    framed.extend(p.additional_samples(root, &tok)?);
    p.apply_training_values(root,&tok,&mut framed)?;
    let mut steps = 0;
    let mut input = 0u64;
    let mut target = 0u64;
    let mut actual_input = 0u64;
    let mut actual_target = 0u64;
    let mut padding = 0u64;
    let mut order = vec![];
    let mut target_order = vec![];
    let mut phrase_order = vec![];
    let mut unchanged = vec![];
    let mut actual_multiset = vec![];
    let mut bucket_side_phrase = [[[0usize;2];2];8];
    let mut flips = 0usize;
    let mut counts = vec![[0usize; 2]; episodes.len()];
    for (i, segment) in h.iter().enumerate() {
        let path = root.join(format!("segment-{i:04}/updates.r3rows"));
        if path.exists() {
            for r in binary::read_value_records(&path)? {
                let step = f.origin_step + steps;
                let ids = p.draw(step);
                let draw = p.training_draw(step);
                let expected_input = draw
                    .iter()
                    .map(|&j| (framed[j].tokens.len() - 1) as u64)
                    .sum::<u64>();
                let expected_target = draw
                    .iter()
                    .map(|&j| (framed[j].tokens.len() - framed[j].response_start) as u64)
                    .sum::<u64>();
                if r["step"] != step + 1
                    || r["draw"] != binary::record!(ids)
                    || r["sample_indices"] != binary::record!(draw)
                    || r["input"] != expected_input
                    || r["target"] != expected_target
                    || r["lr_bits"] != f.constant_lr.to_bits()
                {
                    return Err(bad("actual sampler/target/LR trace mismatch"));
                }
                for (&j, &selected) in ids.iter().zip(&draw) {
                    let bucket=p.sample_bucket(selected);
                    let phrase = (selected / episodes.len()) % 2;
                    counts[j][phrase] += 1;
                    phrase_order.push(phrase);
                    actual_multiset.push((bucket, selected));
                    bucket_side_phrase[bucket][usize::from(selected >= 2 * episodes.len())][phrase] += 1;
                    flips += usize::from(selected >= 2 * episodes.len());
                    if !(2..=4).contains(&bucket) {
                        unchanged.push(digest(&framed[selected].tokens)?);
                    }
                    target_order
                        .push(framed[selected].tokens[framed[selected].response_start..].to_vec());
                }
                order.push(ids);
                input += expected_input;
                target += expected_target;
                steps += 1;
            }
        }
        if segment.step != f.origin_step + steps {
            return Err(bad("checkpoint step differs from update trace"));
        }
        let c: binary::Value = read(&root.join(format!("segment-{i:04}/train-control.r3b")))?;
        actual_input += c["executed_input_tokens_including_uncommitted"]
            .as_u64()
            .ok_or_else(|| bad("input UNKNOWN"))?;
        actual_target += c["executed_target_tokens_including_uncommitted"]
            .as_u64()
            .ok_or_else(|| bad("target UNKNOWN"))?;
        padding += c["executed_padding_tokens"]
            .as_u64()
            .ok_or_else(|| bad("padding UNKNOWN"))?;
        if segment.input != f.origin_input + actual_input
            || segment.target != f.origin_target + target
        {
            return Err(bad("native counter/trace disagreement"));
        }
    }
    if !p.tiny && p.paired.is_none()
        && f.origin_step + steps == p.config.max_steps
        && counts
            .iter()
            .any(|x| *x != if f.variants.is_some() { [1, 1] } else { [2, 0] })
    {
        return Err(bad("completed study exposure contract"));
    }
    if !p.tiny
        && f.origin_step + steps == p.config.max_steps
        && f.arm == "S-SELECT"
        && flips != 3072
    {
        return Err(bad("actual selector exposures"));
    }
    actual_multiset.sort();
    Ok(
        binary::record!({"updates":steps,"actual_multiset":digest(&actual_multiset)?,"bucket_side_phrase":bucket_side_phrase,"case_order":digest(&order)?,"target_order":digest(&target_order)?,"phrase_order":digest(&phrase_order)?,"unchanged_five_tasks":digest(&unchanged)?,"unchanged_digest_kind":"ordered-native-token-row-sha256-v1","selector_draws":flips,"committed_input":input,"committed_target":target,
        "actual_input":actual_input,"actual_target":actual_target,"discarded_input":actual_input-input,"discarded_target":actual_target-target,"padding":padding,
        "case_count":counts.len(),"case_exposure_hash":digest(&counts)?,"original_draws":counts.iter().map(|x|x[0]).sum::<usize>(),"variant_draws":counts.iter().map(|x|x[1]).sum::<usize>()}),
    )
}
#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;

    #[test]
    fn diverse_native_values_preserve_selector_anchor_and_runtime_tokens()->Result<()> {
        let (base,tm)=generate(8,0,19)?;let n=base.len();let mut all=vec![];
        for side in 0..2 {for style in 0..2 {for (original,m) in base.iter().zip(&tm) {
            let mut e=if style==0 {original.clone()}else{changed_question(original,phrases(question_intent(&original.request.input)?.2)[0])?};
            if side==1&&(2..=4).contains(&m.bucket) {e=flip_selection(&e,m)?.0;}
            e.id=format!("{side}-{style}-{}",original.id);e.request.request_id=e.id.clone();all.push(e);
        }}}
        let changed=diverse_training_values(&all,&tm)?;let mut count=0;
        for (i,e) in changed.iter().enumerate() {
            let m=&tm[i%n];
            if !(2..=4).contains(&m.bucket)||m.view<2 {assert_eq!(digest(e)?,digest(&all[i])?);continue;}
            let at=tm.iter().position(|a|a.base==m.base&&a.bucket==m.bucket&&a.view==0).unwrap()+(i/n)*n;
            let before=&all[at];let mut restored=e.clone();
            restored.id=before.id.clone();restored.answer=before.answer.clone();restored.request.request_id=before.request.request_id.clone();
            assert_eq!(citations(&e.answer)?,citations(&before.answer)?);assert_eq!(resolve(&e.request)?,e.answer);
            let mut edits=0;
            for (a,b) in restored.request.evidence.items.iter_mut().zip(&before.request.evidence.items) {
                edits+=usize::from(a.original_excerpt!=b.original_excerpt);a.original_excerpt=b.original_excerpt.clone();
            }
            assert_eq!(edits,1);assert_eq!(digest(&restored)?,digest(before)?);count+=1;
        }
        assert_eq!(count,192);
        for (i,m) in tm.iter().enumerate().filter(|(_,m)|(2..=4).contains(&m.bucket)&&m.view==0) {
            for view in 1..=3 {
                let j=tm.iter().position(|x|x.base==m.base&&x.bucket==m.bucket&&x.view==view).unwrap();
                let rows=vec![[j;8]];
                let (e,selected)=owned_value_view(&changed,&tm,i,view,&rows)?;
                assert_eq!(digest(&e)?,digest(&changed[j])?);assert_eq!(selected.view,view);
                assert!(owned_value_view(&changed,&tm,i,view,&[]).is_err());
            }
        }
        assert!(owned_value_view(&changed,&tm,0,0,&[]).is_err());
        assert!(owned_value_view(&changed,&tm,changed.len(),2,&[]).is_err());
        assert!(diverse_training_values(&all[..all.len()-1],&tm).is_err());
        let mut wrong=tm.clone();wrong[1].view=0;
        assert!(diverse_training_values(&all,&wrong).is_err());
        let (short,short_meta)=generate(2,0,19)?;let short=(0..4).flat_map(|_|short.iter().cloned()).collect::<Vec<_>>();
        assert!(diverse_training_values(&short,&short_meta).is_err());
        let dir=tempfile::tempdir()?;let root=dir.path().join("fixture");prepare(&root,true)?;
        let mut p:Plan=read(&root.join("plan.r3b"))?;
        p.order=(0..8).map(|b|tm.iter().enumerate().filter(|(_,m)|m.bucket==b).map(|(i,_)|i).collect()).collect();
        let tok=ByteBpe::load(&root.join("tokenizer.r3b"))?;
        let (_,_,dev)=p.training_corpus(&root.join("corpus.r3cor"))?;
        let path=root.join("training-values.r3cor");data::native::write(&path,&training_value_corpus(changed.clone(),dev,19)?,true)?;
        p.training_values=Some(file_hash(&path)?);
        let mut actual=samples(&all,&tok,p.config.seq_len)?;
        p.apply_training_values(&root,&tok,&mut actual)?;
        for (actual,expected) in actual.iter().zip(samples(&changed,&tok,p.config.seq_len)?) {
            assert_eq!(actual.tokens,expected.tokens);assert_eq!(actual.response_start,expected.response_start);
        }
        let labels=record_groundings(&changed,&tm,&actual)?;
        assert_eq!(labels.iter().filter(|l|l.is_some()).count(),4*tm.iter().filter(|m|(2..=4).contains(&m.bucket)).count());
        for (i,label) in labels.iter().enumerate() {
            if let Some(label)=label {
                let refs=citations(&changed[i].answer)?;
                assert_eq!(changed[i].request.evidence.items[label.selected].event_id,refs[0]);
                assert_eq!(label.query,actual[i].response_start-1);
                for &(a,b) in &label.records {assert_eq!(actual[i].tokens[a-1],neural::EVIDENCE_ROLE);assert_eq!(actual[i].tokens[b],neural::END_ROLE);}
                assert_ne!(label.selected,labels[(i+2*n)%(4*n)].as_ref().unwrap().selected);
            }
        }
        let i=labels.iter().position(Option::is_some).unwrap();
        let mut invalid=changed.clone();invalid[i].answer.push('x');assert!(record_groundings(&invalid,&tm,&actual).is_err());
        let mut invalid=actual.clone();invalid[i].response_start-=1;assert!(record_groundings(&changed,&tm,&invalid).is_err());
        assert!(record_groundings(&changed[..changed.len()-1],&tm,&actual).is_err());
        p.training_values=Some("wrong".into());assert!(p.apply_training_values(&root,&tok,&mut actual).is_err());
        println!("DIVERSE_NATIVE changed={count} original_anchor_unchanged=true selector_unchanged=true native_runtime_tokens=EXACT optimizer=0 generation=0 teacher=0");
        Ok(())
    }

    #[test]
    fn followthrough_plateau_requires_two_complete_nonimproving_panels() {
        assert!(!no_further_progress(&[]));
        assert!(!no_further_progress(&[(367,70,0),(367,70,0)]));
        assert!(no_further_progress(&[(367,70,0),(367,70,0),(366,69,0)]));
        assert!(!no_further_progress(&[(367,70,0),(366,69,0),(365,68,1)]));
        assert!(!no_further_progress(&[(367,70,0),(368,68,0),(367,69,0)]));
    }
    // Diagnostic-only: the owned view1 must differ from fitted view0 solely in
    // the two supplied values. Labels validate the test; they never enter generation.
    #[test]
    fn seen_value_swap_changes_only_values_and_rejects_other_changes() -> Result<()> {
        let (all,metadata)=generate(4,0,19)?;
        let mut checked=0;
        for (e,m) in all.iter().zip(&metadata).filter(|(_,m)|(2..=4).contains(&m.bucket)&&m.view==0) {
            let at=metadata.iter().position(|n|n.base==m.base&&n.view==1).unwrap();
            for variant in [false,true] {
                let (a,b)=if variant {(changed_question(e,phrases(Intent::Current)[0])?,
                    changed_question(&all[at],phrases(Intent::Current)[0])?)}else{(e.clone(),all[at].clone())};
                for flipped in [false,true] {
                    let (a,b)=if flipped {(flip_selection(&a,m)?.0,flip_selection(&b,&metadata[at])?.0)}else{(a.clone(),b.clone())};
                    verify_seen_value_swap(&a,&b)?;checked+=1;
                    let mut wrong=b.clone();wrong.answer=a.answer.clone();assert!(verify_seen_value_swap(&a,&wrong).is_err());
                    let mut wrong=b.clone();wrong.request.evidence.items.reverse();assert!(verify_seen_value_swap(&a,&wrong).is_err());
                    let mut wrong=b.clone();wrong.request.evidence.items[0].recorded_at+=1;assert!(verify_seen_value_swap(&a,&wrong).is_err());
                    let mut wrong=b.clone();wrong.request.input.push(' ');assert!(verify_seen_value_swap(&a,&wrong).is_err());
                    let mut wrong=b.clone();wrong.request.evidence.items.pop();assert!(verify_seen_value_swap(&a,&wrong).is_err());
                    assert!(verify_seen_value_swap(&a,&a).is_err());
                }
            }
        }
        assert_eq!(checked,48);Ok(())
    }
    #[test]
    fn recombined_values_preserve_selection_and_reject_overlap() -> Result<()> {
        let (all, metadata)=generate(32,0,19)?;
        let mut es=vec![];
        for (e,m) in all.iter().zip(&metadata).filter(|(_,m)|(2..=4).contains(&m.bucket)&&m.view==0) {
            es.extend([e.clone(),flip_selection(e,m)?.0]);
        }
        let new=recombine_train_values(&es,&all)?;
        assert_eq!(new.len(),192);
        for (a,b) in es.iter().zip(&new) {
            assert_ne!(a.answer,b.answer);
            assert_eq!(citations(&a.answer)?,citations(&b.answer)?);
            assert_eq!(resolve(&b.request)?,b.answer);
            let mut restored=b.clone();restored.id=a.id.clone();restored.answer=a.answer.clone();
            restored.request.request_id=a.request.request_id.clone();
            for (x,y) in restored.request.evidence.items.iter_mut().zip(&a.request.evidence.items) {x.original_excerpt=y.original_excerpt.clone();}
            assert_eq!(digest(&restored)?,digest(a)?);
        }
        assert!(recombine_train_values(&es[..1],&all).is_err());
        assert!(recombine_train_values(&es[..2],&[]).is_err());
        assert!(recombine_train_values(&es[..2],&es[..2]).is_err());
        let mut wrong=es[..2].to_vec();wrong[1].request.evidence.items.reverse();
        assert!(recombine_train_values(&wrong,&all).is_err());
        wrong=es[..2].to_vec();wrong[0].answer.clear();
        assert!(recombine_train_values(&wrong,&all).is_err());
        Ok(())
    }
    // Diagnostic only: choose donor values in owned train order, never by output.
    // Preserve numeric width; text lengths can differ and are not compute matched.
    fn recombine_train_values(es:&[Episode],donors:&[Episode])->Result<Vec<Episode>> {
        if es.is_empty() || !es.len().is_multiple_of(2) {return Err(bad("value diagnostic pair count"));}
        let mut out=vec![];
        for pair in es.as_chunks::<2>().0 {
            let a=&pair[0].request.evidence.items; let b=&pair[1].request.evidence.items;
            if a.len()!=2 || b.len()!=2 || a.iter().zip(b).any(|(a,b)|a.original_excerpt!=b.original_excerpt)
                || resolve(&pair[0].request)?!=pair[0].answer || resolve(&pair[1].request)?!=pair[1].answer
                || pair[0].answer==pair[1].answer {return Err(bad("value diagnostic invalid source pair"));}
            let old=[parsed_record(&a[0])?.2,parsed_record(&a[1])?.2];
            let numeric=|s:&str|!s.is_empty()&&s.bytes().all(|b|b.is_ascii_digit());
            let mut chosen=None;
            for donor in donors {
                if donor.request.evidence.items.len()!=2 {continue;}
                let values=[parsed_record(&donor.request.evidence.items[0])?.2,parsed_record(&donor.request.evidence.items[1])?.2];
                if values[0]==values[1] || values.iter().any(|v|old.contains(v))
                    || old.iter().zip(values).any(|(a,b)|numeric(a)!=numeric(b)||(numeric(a)&&a.len()!=b.len())) {continue;}
                chosen=Some(values);break;
            }
            let values=chosen.ok_or_else(||bad("no disjoint owned train values of matching type/width"))?;
            for e in pair {
                let mut changed=e.clone();changed.id=format!("{}-recombined-values",e.id);
                changed.request.request_id=changed.id.clone();
                for (item,value) in changed.request.evidence.items.iter_mut().zip(values) {
                    let (entity,context,_)=parsed_record(item)?;
                    item.original_excerpt=format!("{entity}의 {context} 값은 {value}이다.");
                }
                changed.answer=resolve(&changed.request)?;
                if changed.answer==e.answer || citations(&changed.answer)?!=citations(&e.answer)? {return Err(bad("value diagnostic selection changed"));}
                out.push(changed);
            }
        }
        Ok(out)
    }
    fn owned_value_view(all:&[Episode],tm:&[Meta],index:usize,view:usize,executed:&[[usize;8]])->Result<(Episode,Meta)> {
        if !(1..=3).contains(&view)||tm.is_empty()||all.len()!=4*tm.len()||index>=all.len() {return Err(bad("owned value view bounds"));}
        let n=tm.len();let m=&tm[index%n];
        if !(2..=4).contains(&m.bucket)||m.view!=0 {return Err(bad("owned value view requires selector original"));}
        let matches:Vec<_>=tm.iter().enumerate().filter(|(_,x)|x.base==m.base&&x.bucket==m.bucket&&x.view==view).collect();
        if matches.len()!=1 {return Err(bad("owned value view missing/duplicate"));}
        let j=(index/n)*n+matches[0].0;
        if !executed.iter().flatten().any(|&i|i==j) {return Err(bad("owned value view not actually exposed"));}
        let e=all[j].clone();let mut selected=matches[0].1.clone();selected.id=e.id.clone();selected.source_id=None;
        if resolve(&e.request)?!=e.answer {return Err(bad("owned value view label"));}
        Ok((e,selected))
    }
    #[test]
    #[ignore = "explicit completed arm/new output; normal/value-swap/recombined48/48, actual views<=48/48, margins0/96 generation/teacher; optimizer0"]
    fn paired_seen_train_diagnostic() -> Result<()> {
        if cfg!(feature = "test-support") || !cfg!(feature = "accelerate") {
            return Err(bad("train diagnostic requires production features"));
        }
        let root = PathBuf::from(std::env::var("R3_TRAIN_PAIR_ROOT").map_err(|_|bad("explicit arm required"))?);
        let output = PathBuf::from(std::env::var("R3_TRAIN_PAIR_OUTPUT").map_err(|_|bad("new diagnostic output required"))?);
        let value_swap=std::env::var("R3_TRAIN_PAIR_VALUE_SWAP").as_deref()==Ok("1");
        let recombine=std::env::var("R3_TRAIN_PAIR_RECOMBINE_VALUES").as_deref()==Ok("1");
        let margin_only=std::env::var("R3_TRAIN_PAIR_MARGIN_ONLY").as_deref()==Ok("1");
        let actual_view=std::env::var("R3_TRAIN_PAIR_ACTUAL_VIEW").ok().map(|s|s.parse::<usize>().map_err(|_|bad("actual view integer"))).transpose()?;
        if actual_view.is_some_and(|v|!(1..=3).contains(&v))||[value_swap,recombine,margin_only,actual_view.is_some()].into_iter().filter(|x|*x).count()>1 {return Err(bad("choose one train diagnostic"));}
        let p: Plan = read(&root.join("plan.r3b"))?;
        let end = history(&root, &p)?.pop().ok_or_else(||bad("completed arm missing"))?;
        if end.resume || end.phase.as_deref()!=Some("Finished") || end.step!=p.config.max_steps {
            return Err(bad("train diagnostic needs a completed endpoint"));
        }
        let tape = p.paired.as_ref().ok_or_else(||bad("finite tape missing"))?;
        let f = p.fork.as_ref().unwrap();
        let original = verified_corpus(&root.join("corpus.r3cor"), &p.corpus)?.train;
        let n = original.len();
        let mut all = original;
        all.extend(verified_corpus(&root.join("variants.r3cor"), f.variants.as_ref().unwrap())?.train);
        all.extend(verified_corpus(&root.join("selectors.r3cor"), f.selector.as_ref().unwrap())?.train);
        if actual_view.is_some() {
            let hash=p.training_values.as_ref().ok_or_else(||bad("actual value pool required"))?;
            all=verified_corpus(&root.join("training-values.r3cor"),hash)?.train;
            if all.len()!=4*n {return Err(bad("actual value pool shape"));}
        }
        let (tm, _, _) = verified_metadata(&root, &p)?;
        let mut es = vec![]; let mut ms = vec![];
        for bucket in [2,3,4] {
            let mut seen = std::collections::BTreeSet::new();
            for &index in tape.rows[..tape.block].iter().flatten().filter(|&&i|tm[i%n].bucket==bucket) {
                let base = index%(2*n);
                if !seen.insert(base) { continue; }
                for i in [base, base+2*n] {
                    if !tape.rows[..tape.block].iter().flatten().any(|&j|j==i) { return Err(bad("unseen train pair")); }
                    let mut e = all[i].clone(); let mut m=tm[i%n].clone();
                    if let Some(view)=actual_view {
                        (e,m)=owned_value_view(&all,&tm,i,view,&tape.rows[..end.step-f.origin_step])?;
                    }
                    if value_swap {
                        if m.view!=0 {return Err(bad("seen value diagnostic requires fitted view0"));}
                        let j=tm.iter().position(|candidate|candidate.base==m.base&&candidate.bucket==m.bucket&&candidate.view==1)
                            .ok_or_else(||bad("owned value-swap view missing"))?;
                        let swapped=&all[(i/n)*n+j];
                        verify_seen_value_swap(&e,swapped)?;
                        e=swapped.clone();m=tm[j].clone();
                    }
                    m.id=e.id.clone(); m.source_id=None;
                    if resolve(&e.request)?!=e.answer {return Err(bad("train pair label"));}
                    es.push(e); ms.push(m);
                }
                if seen.len()==8 { break; }
            }
        }
        if es.len()!=48 {return Err(bad("train pair denominator"));}
        let full_es=es.clone();let full_ms=ms.clone();
        let mut reused:BTreeMap<String,binary::Value>=BTreeMap::new();
        let mut reuse_binding=None;
        if actual_view.is_some() {
            let tok=ByteBpe::load(&root.join("tokenizer.r3b"))?;
            let original=verified_corpus(&root.join("corpus.r3cor"),&p.corpus)?.train;
            let (train,metadata)=subset(&original,&tm,8);
            let score=audit_panel(&root,&p,end.step,"train64",&train,&metadata,&tok)?;
            let raw=root.join(format!("eval-{:04}-train64.r3rows",end.step));
            let rows=binary::read_value_records(&raw)?;
            // Finalization changes resume metadata, not the evaluated weights.
            // audit_panel verifies the evaluated file/state; history binds final.
            let endpoint=checkpoint::load(&root.join(&end.checkpoint),Device::Cpu,false)?;
            if endpoint.manifest.trained_steps!=end.step || score.model!=endpoint.model.weight_hash()? {
                return Err(bad("reused train observation endpoint"));
            }
            let mut pending=vec![];let mut pending_meta=vec![];
            for (e,m) in es.into_iter().zip(ms) {
                let prompt=tok.prepare(&e.request,p.architecture.context as u32,&p.architecture.id()?)?;
                let prompt_hash=digest(&prompt.token_ids)?;
                let matched:Vec<_>=rows[1..].iter().filter(|r|r["prompt_digest"]==prompt_hash&&r["expected"]==e.answer).collect();
                if matched.len()>1 {return Err(bad("ambiguous reused observation"));}
                if let Some(row)=matched.first() {reused.insert(e.id.clone(),(*row).clone());}
                else {pending.push(e);pending_meta.push(m);}
            }
            es=pending;ms=pending_meta;
            if reused.len()!=usize::from(actual_view==Some(1)) {return Err(bad("actual view reuse differs from registered coverage"));}
            let references=reused.iter().map(|(id,row)|Ok(binary::record!({"selected_id":id,"original_id":row["id"],"row_digest":digest(row)?}))).collect::<Result<Vec<_>>>()?;
            reuse_binding=Some(binary::record!({"source":raw,"raw_hash":file_hash(&raw)?,"evaluated_physical":rows[0]["physical"],"model":score.model,"step":end.step,"rows":references}));
        }
        if recombine {
            es=recombine_train_values(&es,&all)?;
            for (e,m) in es.iter().zip(&mut ms) {m.id=e.id.clone();}
            let tok=ByteBpe::load(&root.join("tokenizer.r3b"))?;
            validate_framed(&es,&tok)?;
            let seen: BTreeSet<_>=all.iter().map(|e|tok.prepare(&e.request,2048,"value-diagnostic").map(|p|p.token_digest)).collect::<Result<_>>()?;
            for e in &es {
                if seen.contains(&tok.prepare(&e.request,2048,"value-diagnostic")?.token_digest) {return Err(bad("recombined diagnostic input already in owned training sources"));}
            }
        }
        if margin_only {
            let study:Study=read(&f.study.join("study.r3b"))?;
            if file_hash(&f.study.join("study.r3b"))?!=f.study_hash {return Err(bad("margin study binding"));}
            std::fs::create_dir(&output)?;
            let endpoints=[("parent",study.parent_checkpoint,study.parent_file,study.parent_step),
                ("final",root.join(&end.checkpoint),end.checkpoint_hash.clone(),end.step)];
            let start=binary::record!({"source":source_digest()?,"binary":file_hash(&std::env::current_exe()?)?,
                "policy":digest(&p)?,"cases":digest(&es)?,"endpoints":endpoints,
                "scope":"TRAIN_ONLY_TEACHER_MARGIN_NOT_GENERATION","selection":"first eight actually seen pairs per C/D/E in first tape block",
                "optimizer_limit":0,"generation_limit":0,"teacher_limit":96,"automatic_retry":false});
            write(&output.join("started.r3b"),&start)?;
            let mut raw=std::fs::OpenOptions::new().write(true).create_new(true).open(output.join("margins.r3rows"))?;
            append_row(&mut raw,&start)?;
            let mut control=recovery::RunControl::command(false)?;control.set_call_limits(0,96);
            let mut summaries=BTreeMap::new();
            let result=(||->Result<()> {
                for (label,path,physical,step) in &endpoints {
                    control.check("margin_before_native_load")?;
                    if file_hash(path)?!=*physical {return Err(bad("margin checkpoint physical mismatch"));}
                    let l=checkpoint::load(path,Device::Cpu,false)?;
                    if l.manifest.trained_steps!=*step || l.tokenizer.id()!=p.tokenizer {return Err(bad("margin native step/tokenizer"));}
                    let binding=binary::record!({"start":file_hash(&output.join("started.r3b"))?,"physical":physical,"model":l.model.weight_hash()?});
                    let mut margins=vec![];
                    for (i,e) in es.iter().enumerate() {
                        control.check("margin_next_teacher")?;
                        let foil=&es[i^1].answer;
                        let binding=binary::record!({"endpoint":binding,"foil":digest(foil)?});
                        let attempt=prepare_call(&output,label,"teacher",&binding,e,i)?;
                        let observed=match recovery::fresh_teacher_with_foil(&l,e,Some(foil),&mut control) {
                            recovery::ObservedCall::Returned(result)=>result,
                            recovery::ObservedCall::NotInvoked(result)=> {
                                resolve_call(&attempt,None,&control)?;
                                return Err(result.err().unwrap_or_else(||bad("margin teacher not invoked")));
                            }
                        };
                        let row=binary::record!({"endpoint":label,"ordinal":i,"bucket":ms[i].bucket,"id":e.id,
                            "teacher":observed.as_ref().ok(),"error":observed.as_ref().err().map(ToString::to_string),
                            "attempt":attempt.file_name().unwrap().to_string_lossy()});
                        append_row(&mut raw,&row)?;resolve_call(&attempt,Some(&row),&control)?;
                        let value=observed?;
                        let margin=value["conditional_foil"]["margin"].as_f64().filter(|v|v.is_finite()).ok_or_else(||bad("missing/nonfinite conditional margin"))?;
                        margins.push(margin);
                        control.check("margin_row_durable")?;
                    }
                    let pairs=margins.as_chunks::<2>().0;
                    summaries.insert(*label,binary::record!({"pairs":pairs.len(),"both_positive":pairs.iter().filter(|p|p[0]>0.&&p[1]>0.).count(),
                        "both_margin_one":pairs.iter().filter(|p|p[0]>=1.&&p[1]>=1.).count(),
                        "sum_margin_one":pairs.iter().filter(|p|p[0]+p[1]>=1.).count(),
                        "sum_pass_but_one_side_nonpositive":pairs.iter().filter(|p|p[0]+p[1]>=1.&&(p[0]<=0.||p[1]<=0.)).count(),
                        "mean_sum":pairs.iter().map(|p|p[0]+p[1]).sum::<f64>()/pairs.len() as f64,
                        "minimum_side":margins.iter().copied().fold(f64::INFINITY,f64::min),"maximum_side":margins.iter().copied().fold(f64::NEG_INFINITY,f64::max)}));
                }
                Ok(())
            })();
            if let Err(e)=&result {control.classify_error(e);}
            let result=control.seal_terminal().and(result);
            let receipt=binary::record!({"start":file_hash(&output.join("started.r3b"))?,"raw":file_hash(&output.join("margins.r3rows"))?,
                "control":control.receipt(),"summaries":summaries,"error":result.as_ref().err().map(ToString::to_string),"optimizer":0,"quality_score":false});
            publish_confirmed(&output.join("finished.r3b"),&receipt)?;
            println!("TRAIN_MARGIN_DIAGNOSTIC {receipt}");
            return result;
        }
        std::fs::create_dir(&output)?;
        let checkpoint = root.join(&end.checkpoint);
        let owned_panel=actual_view.map(|v|format!("actual-train-view{v}"));
        let panel=owned_panel.as_deref().unwrap_or(if value_swap {"seen-value-swap48"}else if recombine {"recombined-values48"}else{"seen-train48"});
        let limit=es.len();
        write(&output.join("started.r3b"), &binary::record!({"source":source_digest()?,"binary":file_hash(&std::env::current_exe()?)?,"policy":digest(&p)?,"checkpoint":checkpoint,"physical":file_hash(&checkpoint)?,"cases":digest(&es)?,"scope":if actual_view.is_some() {"ACTUAL_TRAIN_VIEW_FITTING_NOT_HELDOUT"}else if value_swap {"SAME_SCENE_VALUE_SWAP_DIAGNOSTIC_NOT_HELDOUT"}else if recombine {"OWNED_TRAIN_VALUES_NEW_COMBINATIONS_NOT_HELDOUT"}else{"TRAIN_DIAGNOSTIC_NOT_HELDOUT"},"generation_limit":limit,"teacher_limit":limit,"optimizer_limit":0,"actual_view":actual_view,"reused":reuse_binding}))?;
        if recombine || actual_view.is_some() {publish_confirmed(&output.join("diagnostic-cases.r3b"),&(&es,&ms))?;}
        if actual_view.is_some() {publish_confirmed(&output.join("full-selection.r3b"),&(&full_es,&full_ms))?;}
        let mut control = recovery::RunControl::command(false)?;
        control.set_call_limits(limit,limit);
        let result = evaluate_panel(&p,&output,&checkpoint,end.step,panel,&es,&ms,&mut control);
        if let Err(e)=&result {control.classify_error(e);}
        let result=control.seal_terminal().and(result);
        let rows=binary::read_value_records(&output.join(format!("eval-{:04}-{panel}.r3rows",end.step)))?;
        let both=rows[1..].chunks_exact(2).filter(|r|r[0]["exact_match"]==true&&r[1]["exact_match"]==true).count();
        let same=rows[1..].chunks_exact(2).filter(|r|r[0]["generation_completed"]==true&&r[1]["generation_completed"]==true&&r[0]["actual"]==r[1]["actual"]).count();
        let combined=if actual_view.is_some() && result.is_ok() {
            let decisions:Vec<_>=full_es.iter().map(|e| {
                let r=reused.get(&e.id).or_else(||rows[1..].iter().find(|r|r["id"]==e.id)).ok_or_else(||bad("incomplete actual train view"))?;
                Ok(r["generation_completed"]==true&&r["error"].is_null()&&r["finish_reason"]=="stop"&&r["actual"]==e.answer)
            }).collect::<Result<_>>()?;
            Some(binary::record!({"planned":48,"new":limit,"reused":reused.len(),"full":decisions.iter().filter(|&&v|v).count(),"both":decisions.as_chunks::<2>().0.iter().filter(|p|p[0]&&p[1]).count()}))
        }else{None};
        let receipt=binary::record!({"start":file_hash(&output.join("started.r3b"))?,"control":control.receipt(),"score":result.as_ref().ok(),"both":if actual_view.is_some(){None}else{Some(both)},"same":if actual_view.is_some(){None}else{Some(same)},"combined":combined,"error":result.as_ref().err().map(ToString::to_string),"optimizer":0});
        publish_confirmed(&output.join("finished.r3b"), &receipt)?;
        println!("TRAIN_PAIR_DIAGNOSTIC {receipt}");
        result.map(|_|())
    }
    // Read-only attribution regions from trusted framing, never answer-dependent packing.
    fn attention_regions(ids: &[u32]) -> Result<Vec<usize>> {
        let roles: Vec<_> = ids.iter().copied().filter(|&t|t < neural::SPECIALS as u32).collect();
        if roles != [neural::BOS,neural::SYSTEM_ROLE,neural::END_ROLE,neural::USER_ROLE,
            neural::END_ROLE,neural::EVIDENCE_ROLE,neural::END_ROLE,neural::EVIDENCE_ROLE,
            neural::END_ROLE,neural::ASSISTANT_ROLE] {return Err(bad("attention diagnostic requires exact two-record framing"));}
        let mut regions=vec![4;ids.len()]; let mut current=4; let mut evidence=0;
        for (i,&t) in ids.iter().enumerate() {
            match t {
                neural::SYSTEM_ROLE=>current=3,
                neural::USER_ROLE=>current=0,
                neural::EVIDENCE_ROLE=>{current=1+evidence;evidence+=1;},
                neural::END_ROLE|neural::ASSISTANT_ROLE=>current=4,
                t if t>=neural::SPECIALS as u32=>regions[i]=current,
                _=>(),
            }
        }
        if (0..4).any(|r|!regions.contains(&r)) {return Err(bad("empty attention diagnostic region"));}
        Ok(regions)
    }
    fn attention_recorded_row(rows:&[binary::Value],prompt:&str,answer:&str)->Result<binary::Value> {
        let matches:Vec<_>=rows.iter().filter(|r|r["native_prompt_digest"]==prompt && r["expected"]==answer).collect();
        if matches.len()!=1 {return Err(bad("attention recorded input missing/ambiguous"));}
        Ok(matches[0].clone())
    }
    #[test]
    fn paired_attention_regions_use_trusted_framing() -> Result<()> {
        let ids=[1,3,8,7,4,9,10,7,5,11,7,5,12,13,7,6];
        assert_eq!(attention_regions(&ids)?,[4,4,3,4,4,0,0,4,4,1,4,4,2,2,4,4]);
        assert!(attention_regions(&[]).is_err());
        for i in 0..ids.len() {let mut wrong=ids.to_vec();wrong.remove(i);assert!(attention_regions(&wrong).is_err() || ids[i]>=8);}
        let mut wrong=ids;wrong[12]=neural::EOS;assert!(attention_regions(&wrong).is_err());
        let row=binary::record!({"id":"old-container-id","native_prompt_digest":"same-owned-prompt","expected":"same-answer"});
        assert_eq!(attention_recorded_row(std::slice::from_ref(&row),"same-owned-prompt","same-answer")?,row);
        assert!(attention_recorded_row(&[row.clone(),row.clone()],"same-owned-prompt","same-answer").is_err());
        assert!(attention_recorded_row(std::slice::from_ref(&row),"different-prompt","same-answer").is_err());
        assert!(attention_recorded_row(&[row],"same-owned-prompt","different-answer").is_err());
        Ok(())
    }
    #[test]
    #[ignore = "explicit completed DIVERSE/new output/train raw; 24 prompt-only observations plus24 reference forwards, optimizer/generation0"]
    fn paired_selector_attention_diagnostic() -> Result<()> {
        if cfg!(feature="test-support") || !cfg!(feature="accelerate") {return Err(bad("attention diagnostic production features required"));}
        let env=|key|std::env::var(key).map(PathBuf::from).map_err(|_|bad("explicit attention diagnostic paths required"));
        let root=env("R3_ATTENTION_ROOT")?;let output=env("R3_ATTENTION_OUTPUT")?;
        let train_raw=env("R3_ATTENTION_TRAIN_RAW")?;
        let p:Plan=read(&root.join("plan.r3b"))?;
        let end=history(&root,&p)?.pop().ok_or_else(||bad("attention endpoint absent"))?;
        let tape=p.paired.as_ref().ok_or_else(||bad("attention tape missing"))?;
        if tape.mode!="DIVERSE" || end.resume || end.phase.as_deref()!=Some("Finished") || end.step!=p.config.max_steps {
            return Err(bad("attention diagnostic requires closed DIVERSE endpoint"));
        }
        let f=p.fork.as_ref().unwrap();
        let corpus=verified_corpus(&root.join("corpus.r3cor"),&p.corpus)?;
        let (tm,dm,_)=verified_metadata(&root,&p)?;
        let n=corpus.train.len();let dev=corpus.validation;
        let all=verified_corpus(&root.join("training-values.r3cor"),p.training_values.as_ref().ok_or_else(||bad("attention owned pool"))?)?.train;
        if all.len()!=4*n {return Err(bad("attention training pool shape"));}
        let checkpoint=root.join(&end.checkpoint);
        let l=checkpoint::load(&checkpoint,Device::Cpu,false)?;
        if file_hash(&checkpoint)?!=end.checkpoint_hash || l.manifest.trained_steps!=end.step || l.tokenizer.id()!=p.tokenizer {return Err(bad("attention endpoint binding"));}
        let model=l.model.weight_hash()?;
        let (_,flips,fm)=paired_flip_panel(&p,&root,end.step,&dev,&dm)?;
        audit_panel(&root,&p,end.step,"dev512",&dev,&dm,&l.tokenizer)?;
        audit_panel(&root,&p,end.step,"selector192",&flips,&fm,&l.tokenizer)?;
        let paths=[train_raw,root.join(format!("eval-{:04}-dev512.r3rows",end.step)),root.join(format!("eval-{:04}-selector192.r3rows",end.step))];
        let raws=paths.iter().map(|path|Ok(binary::read_value_records(path)?)).collect::<Result<Vec<_>>>()?;
        for rows in &raws {if rows[0]["binding"]["model"]!=model || rows[0]["binding"]["step"]!=end.step {return Err(bad("attention raw endpoint mismatch"));}}
        let mut selected:Vec<(&str,usize,Episode,binary::Value)>=vec![];
        for bucket in [2,3,4] {
            let mut seen=std::collections::BTreeSet::new();
            for &index in tape.rows[..tape.block].iter().flatten().filter(|&&i|tm[i%n].bucket==bucket && tm[i%n].view==0) {
                let base=index%(2*n);if !seen.insert(tm[index%n].base.clone()) {continue;}
                for i in [base,base+2*n] {
                    if !tape.rows[..end.step-f.origin_step].iter().flatten().any(|&j|j==i) {return Err(bad("attention train side unexposed"));}
                    let e=all[i].clone();
                    // Native merged pools have unique container IDs; prompt/target stay identical.
                    let prompt=l.tokenizer.prepare(&e.request,p.architecture.context as u32,&p.architecture.id()?)?;
                    let row=attention_recorded_row(&raws[0][1..],&prompt.token_digest,&e.answer)?;
                    selected.push(("trained",bucket,e,row));
                }
                if seen.len()==2 {break;}
            }
            for (e,m) in dev.iter().zip(&dm).filter(|(_,m)|m.bucket==bucket && m.view==0).take(2) {
                let (flip,_)=flip_selection(e,m)?;
                for (e,rows) in [(e.clone(),&raws[1]),(flip,&raws[2])] {
                    let row=rows[1..].iter().find(|r|r["id"]==e.id).ok_or_else(||bad("attention development raw missing"))?;
                    selected.push(("development",bucket,e,row.clone()));
                }
            }
        }
        if selected.len()!=24 {return Err(bad("attention diagnostic case count"));}
        for (_,_,e,row) in &selected {
            let prepared=l.tokenizer.prepare(&e.request,p.architecture.context as u32,&p.architecture.id()?)?;
            let ids:Vec<u32>=binary::from_value(row["raw_tokens"].clone())?;
            if resolve(&e.request)?!=e.answer || row["expected"]!=e.answer || row["native_prompt_digest"]!=prepared.token_digest
                || !prepared.excluded.is_empty() || prepared.provided.len()!=2 || row["generation_completed"]!=true
                || !row["error"].is_null() || row["finish_reason"]!="stop" || ids.last()!=Some(&neural::EOS)
                || row["actual"]!=l.tokenizer.decode(&ids[..ids.len()-1])? {return Err(bad("attention frozen case/raw mismatch"));}
            attention_regions(&prepared.token_ids)?;
        }
        std::fs::create_dir(&output)?;
        let start=binary::record!({"source":source_digest()?,"binary":file_hash(&std::env::current_exe()?)?,"policy":digest(&p)?,
            "checkpoint":checkpoint,"physical":end.checkpoint_hash,"model":model,"step":end.step,"cases":digest(&selected)?,
            "raw_files":paths.iter().map(|path|Ok((path,file_hash(path)?))).collect::<Result<Vec<_>>>()?,
            "scope":"PROMPT_ONLY_ATTENTION_OBSERVATION_NOT_CAUSAL_OR_QUALITY_ACCEPTANCE","selection":"first two executed view0 train pairs and first two view0 dev pairs per C/D/E",
            "optimizer_limit":0,"generation_limit":0,"teacher_forward_limit":48,"automatic_retry":false,"region_order":["question","record0","record1","system","framing"]});
        write(&output.join("started.r3b"),&start)?;
        publish_confirmed(&output.join("cases.r3b"),&selected)?;
        let mut raw=std::fs::OpenOptions::new().write(true).create_new(true).open(output.join("attention.r3rows"))?;
        append_row(&mut raw,&start)?;
        let mut control=recovery::RunControl::command(false)?;control.set_call_limits(0,48);
        let result=(||->Result<()> {
            for (i,(scope,bucket,e,old)) in selected.iter().enumerate() {
                let prompt=l.tokenizer.prepare(&e.request,p.architecture.context as u32,&p.architecture.id()?)?;
                let regions=attention_regions(&prompt.token_ids)?;
                let refs=citations(&e.answer)?;
                if refs.len()!=1 {return Err(bad("attention selected citation count"));}
                let slot=e.request.evidence.items.iter().position(|item|item.event_id==refs[0]).ok_or_else(||bad("attention selected record absent"))?;
                let input=Tensor::new(prompt.token_ids.as_slice(),&Device::Cpu)?.unsqueeze(0)?;
                let mut previous=None;
                for observed in [true,false] {
                    let label=if observed {"observed"}else{"reference"};
                    control.check("attention_before_call")?;
                    let attempt=prepare_call(&output,label,"teacher",&start,e,i)?;
                    if let Err(error)=control.begin_teacher() {resolve_call(&attempt,None,&control)?;return Err(error);}
                    let mut heads=vec![];
                    let returned=(||->Result<Vec<f32>> {
                        let mut observer=|layer:usize,q:&Tensor,k:&Tensor,mask:&Tensor|->Result<()> {
                            control.check("attention_layer")?;
                            let (_,h,t,d)=q.dims4()?;
                            let scores=(q.narrow(2,t-1,1)?.contiguous()?.matmul(&neural::transformer::repeat_kv(k,h)?.transpose(2,3)?.contiguous()?)?/(d as f64).sqrt())?
                                .squeeze(0)?.squeeze(1)?.to_vec2::<f32>()?;
                            let allowed=mask.narrow(2,t-1,1)?.flatten_all()?.to_vec1::<f32>()?;
                            for (head,logits) in scores.iter().enumerate() {
                                let permitted:Vec<_>=logits.iter().enumerate().filter(|(j,_)|allowed[*j]>0.).collect();
                                if permitted.is_empty() || permitted.iter().any(|(_,v)|!v.is_finite()) {return Err(bad("attention nonfinite scores"));}
                                let max=permitted.iter().map(|(_,v)|f64::from(**v)).fold(f64::NEG_INFINITY,f64::max);
                                let z=permitted.iter().map(|(_,v)|(f64::from(**v)-max).exp()).sum::<f64>();
                                let mut mass=[0f64;5];let mut visible=[0usize;5];let mut entropy=0.;
                                for (j,v) in permitted {let prob=(f64::from(*v)-max).exp()/z;mass[regions[j]]+=prob;visible[regions[j]]+=1;if prob>0. {entropy-=prob*prob.ln();}}
                                if (mass.iter().sum::<f64>()-1.).abs()>1e-9 {return Err(bad("attention mass sum"));}
                                heads.push(binary::record!({"layer":layer,"head":head,"mass":mass,"visible_tokens":visible,"entropy":entropy}));
                            }
                            Ok(())
                        };
                        let logits=if observed {l.model.forward_observed(&input,None,&mut observer)?}else{l.model.forward(&input,None)?};
                        control.check("attention_forward_returned")?;
                        let last=logits.narrow(1,prompt.token_ids.len()-1,1)?.flatten_all()?.to_vec1::<f32>()?;
                        if last.iter().any(|v|!v.is_finite()) {return Err(bad("attention nonfinite logits"));}
                        Ok(last)
                    })();
                    let value=returned.as_ref().ok();
                    let argmax=value.map(|v|v.iter().enumerate().max_by(|a,b|a.1.total_cmp(b.1).then(b.0.cmp(&a.0))).unwrap().0);
                    let row=binary::record!({"ordinal":i,"id":e.id,"case":digest(e)?,"scope":scope,"bucket":bucket,"mode":label,
                        "prompt_tokens":prompt.token_ids.len(),"prompt_digest":prompt.token_digest,"selected_slot":slot,"heads":heads,
                        "logits":value,"argmax":argmax,"expected_first":l.tokenizer.encode(e.answer.as_bytes())?[0],"old_first":old["raw_tokens"][0],
                        "error":returned.as_ref().err().map(ToString::to_string),"attempt":attempt.file_name().unwrap().to_string_lossy()});
                    append_row(&mut raw,&row)?;resolve_call(&attempt,Some(&row),&control)?;
                    let logits=returned?;
                    if old["raw_tokens"][0]!=argmax.unwrap() {return Err(bad("attention old greedy first token mismatch"));}
                    if observed {previous=Some(logits);}else if previous.as_ref()!=Some(&logits) {return Err(bad("attention observed/reference logits mismatch"));}
                    control.check("attention_row_durable")?;
                }
            }
            Ok(())
        })();
        if let Err(error)=&result {control.classify_error(error);}
        let result=control.seal_terminal().and(result);
        let receipt=binary::record!({"start":file_hash(&output.join("started.r3b"))?,"raw":file_hash(&output.join("attention.r3rows"))?,
            "control":control.receipt(),"error":result.as_ref().err().map(ToString::to_string),"optimizer":0,"generation":0,"quality_score":false});
        publish_confirmed(&output.join("finished.r3b"),&receipt)?;
        println!("SELECTOR_ATTENTION_DIAGNOSTIC {receipt}");
        result
    }
    #[test]
    fn paired_tape_full_writer_reader_bounds_and_balance() {
        let d = tempfile::tempdir().unwrap();
        let root = d.path().join("parent");
        prepare(&root, true).unwrap();
        let mut p: Plan = read(&root.join("plan.r3b")).unwrap();
        let (_, tm) = generate(256, 0, 20260919).unwrap();
        let n = tm.len();
        p.config.max_steps = 6144;
        p.order = (0..8).map(|b| tm.iter().enumerate().filter(|(_,m)| m.bucket == b).map(|(i,_)|i).collect()).collect();
        let tapes = pair_tapes(&p, &tm, 256, 3840).unwrap();
        let co=co_batch_rows(&tapes[1],None).unwrap();
        let replay=co_batch_rows(&tapes[1],Some(256)).unwrap();
        let wide=co_batch_rows(&tapes[1],Some(512)).unwrap();
        assert!(co_batch_rows(&[],None).is_err());assert!(co_batch_rows(&tapes[1][..3],None).is_err());
        for block in [0,1,3,257,4000] {assert!(co_batch_rows(&tapes[1],Some(block)).is_err());}
        for (mode, rows) in [("SPACED",&tapes[0]),("ADJACENT",&tapes[1]),("COBATCH",&co),("CONTRAST",&co),("SIDE",&co),("REPLAY",&replay),("WIDE",&wide)] {
            let tape = PairTape { mode: mode.into(), first_step: 6144, block: 256, samples: "fixture-explicit".into(), rows: rows.clone() };
            let path = d.path().join(format!("{mode}.r3b"));
            publish_confirmed(&path, &tape).unwrap();
            let restored: PairTape = read_confirmed(&path).unwrap();
            assert_eq!(tape, restored);
            verify_pair_tape(&p,&tm,&restored).unwrap();
            if ["CONTRAST","SIDE","REPLAY","WIDE"].contains(&mode) {
                let mut bound=p.clone();bound.paired=Some(restored.clone());
                for (i,row) in rows.iter().enumerate() {
                    let (sidewise,pairs)=bound.contrast_pairs(row).unwrap().unwrap();
                    assert_eq!(sidewise,mode!="CONTRAST");
                    assert_eq!(pairs.len(),if i%2==0 {3}else{0});
                    for (a,b) in pairs {assert_eq!(row[b],row[a]+2*n);}
                }
                let mut incomplete=rows[0];incomplete[1]=incomplete[0];
                assert!(bound.contrast_pairs(&incomplete).is_err());
                assert!(bound.contrast_pairs(&rows[0][..7]).is_err());
            }
            let mut wrong=restored.clone();wrong.rows[1]=wrong.rows[0];assert!(verify_pair_tape(&p,&tm,&wrong).is_err());
            let mut wrong=restored.clone();wrong.rows[255][2]=4*n;assert!(verify_pair_tape(&p,&tm,&wrong).is_err());
            let mut wrong=restored.clone();wrong.rows.pop();assert!(verify_pair_tape(&p,&tm,&wrong).is_err());
            if mode=="WIDE" {
                let mut wrong=restored.clone();wrong.rows=replay.clone();assert!(verify_pair_tape(&p,&tm,&wrong).is_err());
            }
            for i in [0, 1, 255, 256, 257, 511, 512, 1279, 3839] { assert_eq!(restored.at(6144+i).unwrap(), rows[i]); }
            assert!(restored.at(6143).is_err()); assert!(restored.at(9984).is_err());
            for block in rows.chunks_exact(256) {
                for bucket in 2..=4 {
                    let mut pairs: BTreeMap<usize, Vec<(usize,usize)>> = BTreeMap::new();
                    let mut phrase = [0;2]; let mut first_side = [0;2]; let mut bases = BTreeSet::new();
                    for (i,row) in block.iter().enumerate() {
                        for &j in row.iter().filter(|&&j|tm[j%n].bucket==bucket) {
                            assert_eq!(p.sample_bucket(j),bucket);
                            pairs.entry(j%(2*n)).or_default().push((i,j/(2*n)));
                            phrase[(j/n)%2] += 1; bases.insert(&tm[j%n].base);
                        }
                    }
                    assert_eq!(pairs.len(),128); assert_eq!(bases.len(),128); assert_eq!(phrase,[128,128]);
                    for entries in pairs.values() {
                        assert_eq!(entries.len(),2); assert_eq!(entries[0].1+entries[1].1,1);
                        assert_eq!(entries[1].0-entries[0].0,if mode=="SPACED" {128}else if ["COBATCH","CONTRAST","SIDE","REPLAY","WIDE"].contains(&mode) {0}else{1});
                        first_side[entries[0].1]+=1;
                    }
                    assert_eq!(first_side,[64,64]);
                }
            }
        }
        for (a,b) in tapes[0].iter().zip(&tapes[1]) {
            for bucket in [0,1,5,6,7] { assert_eq!(a[bucket],b[bucket]); }
        }
        for (a,b) in tapes[0].chunks_exact(256).zip(tapes[1].chunks_exact(256)) {
            let mut a: Vec<_> = a.iter().flat_map(|r|r.iter().copied()).collect();
            let mut b: Vec<_> = b.iter().flat_map(|r|r.iter().copied()).collect(); a.sort(); b.sort(); assert_eq!(a,b);
        }
        for (control,packed) in tapes[1].chunks_exact(2).zip(co.chunks_exact(2)) {
            let mut a:Vec<_>=control.iter().flatten().copied().collect();
            let mut b:Vec<_>=packed.iter().flatten().copied().collect();a.sort();b.sort();assert_eq!(a,b);
            for bucket in 2..=4 {assert_eq!(packed[0].iter().filter(|&&i|tm[i%n].bucket==bucket).count(),2);}
            assert!(packed[1].iter().all(|&i|!(2..=4).contains(&tm[i%n].bucket)));
        }
        for block in co.chunks_exact(256) {
            let mut counts=[0;8];for &i in block.iter().flatten(){counts[tm[i%n].bucket]+=1;}
            assert_eq!(counts,[256;8]);
        }
        assert_eq!(&replay[..256],&co[..256]);
        let mut repeat_counts=vec![0usize;4*n];
        for (step,row) in replay.iter().enumerate() {
            for (slot,&i) in row.iter().enumerate() {
                let bucket=tm[i%n].bucket;
                assert_eq!(i,if (2..=4).contains(&bucket) {co[step%256][slot]}else{co[step][slot]});
                if step<1280 {repeat_counts[i]+=1;}
            }
        }
        for bucket in 2..=4 {
            let seen:Vec<_>=repeat_counts.iter().enumerate().filter(|(i,c)|tm[*i%n].bucket==bucket&&**c>0).collect();
            assert_eq!(seen.len(),256);assert!(seen.iter().all(|(_,c)|**c==5));
        }
        assert_eq!(&wide[..512],&co[..512]);
        assert_eq!(co_batch_rows(&tapes[1][..1280],Some(512)).unwrap(),wide[..1280]);
        let mut wide_counts=vec![0usize;4*n];
        for (step,row) in wide.iter().enumerate() {
            for (slot,&i) in row.iter().enumerate() {
                let bucket=tm[i%n].bucket;
                assert_eq!(i,if (2..=4).contains(&bucket) {co[step%512][slot]}else{co[step][slot]});
                if step<1280 {wide_counts[i]+=1;}
            }
        }
        for bucket in 2..=4 {
            let seen:Vec<_>=wide_counts.iter().enumerate().filter(|(i,c)|tm[*i%n].bucket==bucket&&**c>0).collect();
            assert_eq!(seen.len(),512);
            assert_eq!(seen.iter().filter(|(_,c)|**c==2).count(),256);
            assert_eq!(seen.iter().filter(|(_,c)|**c==3).count(),256);
            assert_eq!(seen.iter().map(|(i,_)|&tm[*i%n].base).collect::<BTreeSet<_>>().len(),256);
        }
        // Diagnostic recurrence is a fixed prefix, not a balanced full256 block.
        let fit=co_batch_rows(&tapes[1],Some(16)).unwrap();
        let mut production=p.clone();production.tiny=false;
        let tape=PairTape {mode:"FIT".into(),first_step:6144,block:256,samples:"fixture-explicit".into(),rows:fit.clone()};
        let path=d.path().join("FIT.r3b");publish_confirmed(&path,&tape).unwrap();
        let restored:PairTape=read_confirmed(&path).unwrap();assert_eq!(restored,tape);
        verify_pair_tape(&production,&tm,&restored).unwrap();
        production.paired=Some(restored.clone());
        let mut counts=vec![0usize;4*n];
        assert_eq!(&fit[..16],&co[..16]);
        for (step,row) in fit.iter().enumerate() {
            let (sidewise,pairs)=production.contrast_pairs(row).unwrap().unwrap();
            assert!(sidewise);assert_eq!(pairs.len(),if step%2==0 {3}else{0});
            for (a,b) in pairs {assert_eq!(row[b],row[a]+2*n);}
            for (slot,&i) in row.iter().enumerate() {
                assert_eq!(i,if (2..=4).contains(&tm[i%n].bucket) {co[step%16][slot]}else{co[step][slot]});
                if step<1280 {counts[i]+=1;}
            }
        }
        for bucket in 2..=4 {
            let seen:Vec<_>=counts.iter().enumerate().filter(|(i,c)|tm[*i%n].bucket==bucket&&**c>0).collect();
            assert_eq!(seen.len(),16);assert!(seen.iter().all(|(_,c)|**c==80));
            assert_eq!(seen.iter().filter(|(i,_)|*i<2*n).count(),8);
            assert_eq!(seen.iter().map(|(i,_)|&tm[*i%n].base).collect::<BTreeSet<_>>().len(),8);
        }
        for i in [0,1,15,16,255,256,1279,3839] {assert_eq!(restored.at(6144+i).unwrap(),fit[i]);}
        let mut wrong=restored.clone();wrong.rows=replay.clone();assert!(verify_pair_tape(&production,&tm,&wrong).is_err());
        wrong=restored.clone();wrong.rows[16][0]=co[16][0];assert!(verify_pair_tape(&production,&tm,&wrong).is_err());
        let values=value_exchange_rows(&fit,&tm,16).unwrap();
        let value_tape=PairTape {mode:"VALUE".into(),rows:values.clone(),..tape.clone()};
        let path=d.path().join("VALUE.r3b");publish_confirmed(&path,&value_tape).unwrap();
        let value_back:PairTape=read_confirmed(&path).unwrap();assert_eq!(value_back,value_tape);
        let mut value_parent=production.clone();value_parent.paired=None;
        verify_pair_tape(&value_parent,&tm,&value_back).unwrap();
        value_parent.paired=Some(value_back.clone());
        let mut counts=vec![0usize;4*n];
        for (step,(old,row)) in fit.iter().zip(&values).enumerate() {
            let (sidewise,pairs)=value_parent.contrast_pairs(row).unwrap().unwrap();assert!(sidewise);
            assert_eq!(pairs.len(),if step%2==0 {3}else{0});
            for (a,b) in pairs {assert_eq!(row[b],row[a]+2*n);}
            for (&before,&after) in old.iter().zip(row) {
                if (2..=4).contains(&tm[before%n].bucket) {
                    assert_eq!(tm[before%n].base,tm[after%n].base);
                    assert_eq!(tm[after%n].view,(step/16)%2);
                    assert_eq!(before/n,after/n,"phrase and side retained");
                }else{assert_eq!(before,after,"anchor slot retained");}
                if step<1280 {counts[after]+=1;}
            }
        }
        for bucket in 2..=4 {
            let seen:Vec<_>=counts.iter().enumerate().filter(|(i,c)|tm[*i%n].bucket==bucket&&**c>0).collect();
            assert_eq!(seen.len(),32);assert!(seen.iter().all(|(_,c)|**c==40));
            assert_eq!(seen.iter().map(|(i,_)|&tm[*i%n].base).collect::<BTreeSet<_>>().len(),8);
        }
        for step in [0,15,16,31,32,255,256,1279,3839] {assert_eq!(value_back.at(6144+step).unwrap(),values[step]);}
        let mut wrong=value_back.clone();wrong.rows=fit.clone();assert!(verify_pair_tape(&production,&tm,&wrong).is_err());
        for cycle in [0,1,3,17] {assert!(value_exchange_rows(&fit,&tm,cycle).is_err());}
        assert!(value_exchange_rows(&[],&tm,16).is_err());
        let mut wrong=fit.clone();wrong[0][0]=4*n;assert!(value_exchange_rows(&wrong,&tm,16).is_err());
        let mut duplicate=tm.clone();duplicate.push(tm.iter().find(|m|m.view==1).unwrap().clone());
        assert!(value_exchange_rows(&fit,&duplicate,16).is_err());
        let diverse=value_cycle_rows(&fit,&tm,16,4).unwrap();
        let diverse_tape=PairTape {mode:"DIVERSE".into(),rows:diverse.clone(),..tape.clone()};
        let path=d.path().join("DIVERSE.r3b");publish_confirmed(&path,&diverse_tape).unwrap();
        let restored:PairTape=read_confirmed(&path).unwrap();assert_eq!(restored,diverse_tape);
        let mut diverse_parent=production.clone();diverse_parent.paired=None;
        verify_pair_tape(&diverse_parent,&tm,&restored).unwrap();
        let clean_parent=diverse_parent.clone();diverse_parent.paired=Some(restored.clone());
        let mut exposure=vec![0;4*n];
        for (step,(old,row)) in fit.iter().zip(&diverse).enumerate() {
            let (sidewise,pairs)=diverse_parent.contrast_pairs(row).unwrap().unwrap();assert!(sidewise);
            assert_eq!(pairs.len(),if step%2==0 {3}else{0});
            for (&a,&b) in old.iter().zip(row) {
                if (2..=4).contains(&tm[a%n].bucket) {
                    assert_eq!(tm[a%n].base,tm[b%n].base);assert_eq!(a/n,b/n);
                    assert_eq!(tm[b%n].view,(step/16)%4);
                }else{assert_eq!(a,b);}
                if step<1280 {exposure[b]+=1;}
            }
        }
        for bucket in 2..=4 {
            let seen:Vec<_>=exposure.iter().enumerate().filter(|(i,c)|tm[*i%n].bucket==bucket&&**c>0).collect();
            assert_eq!(seen.len(),64);assert!(seen.iter().all(|(_,c)|**c==20));
            assert_eq!(seen.iter().map(|(i,_)|&tm[*i%n].base).collect::<BTreeSet<_>>().len(),8);
        }
        let mut wrong=restored.clone();wrong.rows=values.clone();assert!(verify_pair_tape(&clean_parent,&tm,&wrong).is_err());
        assert!(value_cycle_rows(&fit,&tm,16,3).is_err());
        let cover_original=co_batch_rows(&tapes[1],Some(64)).unwrap();
        for mode in ["COVER4","GROUND"] {
            let rows=value_cycle_rows(&cover_original,&tm,64,4).unwrap();
            let bound=PairTape {mode:mode.into(),rows:rows.clone(),..tape.clone()};
            let path=d.path().join(format!("{mode}.r3b"));publish_confirmed(&path,&bound).unwrap();
            let back:PairTape=read_confirmed(&path).unwrap();assert_eq!(back,bound);
            verify_pair_tape(&clean_parent,&tm,&back).unwrap();
            let mut model_plan=production.clone();model_plan.paired=Some(back.clone());
            let mut counts=vec![0;4*n];
            for (step,row) in rows.iter().enumerate() {
                let (sidewise,pairs)=model_plan.contrast_pairs(row).unwrap().unwrap();assert!(sidewise);
                assert_eq!(pairs.len(),if step%2==0 {3}else{0});
                for (a,b) in pairs {assert_eq!(row[b],row[a]+2*n);}
                for (slot,&i) in row.iter().enumerate() {
                    if (2..=4).contains(&tm[i%n].bucket) {
                        let old=cover_original[step][slot];
                        assert_eq!(tm[i%n].base,tm[old%n].base);assert_eq!(i/n,old/n);
                        assert_eq!(tm[i%n].view,(step/64)%4);
                    }else{assert_eq!(i,diverse[step][slot],"DIVERSE anchor unchanged");}
                    if step<1280 {counts[i]+=1;}
                }
            }
            for bucket in 2..=4 {
                let seen:Vec<_>=counts.iter().enumerate().filter(|(i,c)|tm[*i%n].bucket==bucket&&**c>0).collect();
                assert_eq!(seen.len(),256);assert!(seen.iter().all(|(_,c)|**c==5));
                assert_eq!(seen.iter().map(|(i,_)|&tm[*i%n].base).collect::<BTreeSet<_>>().len(),32);
            }
            let mut wrong=back.clone();wrong.rows=diverse.clone();assert!(verify_pair_tape(&clean_parent,&tm,&wrong).is_err());
            for step in [0,1,63,64,127,128,191,192,255,256,1279,3839] {assert_eq!(back.at(6144+step).unwrap(),rows[step]);}
        }
        let cover=value_exchange_rows(&cover_original,&tm,64).unwrap();
        let cover_tape=PairTape {mode:"COVER".into(),rows:cover.clone(),..tape.clone()};
        let path=d.path().join("COVER.r3b");publish_confirmed(&path,&cover_tape).unwrap();
        let restored:PairTape=read_confirmed(&path).unwrap();assert_eq!(restored,cover_tape);
        let mut cover_parent=production.clone();cover_parent.paired=None;
        verify_pair_tape(&cover_parent,&tm,&restored).unwrap();
        let mut bound=production.clone();bound.paired=Some(restored.clone());
        let mut counts=vec![0usize;4*n];
        for (step,row) in cover.iter().enumerate() {
            let (sidewise,pairs)=bound.contrast_pairs(row).unwrap().unwrap();assert!(sidewise);
            assert_eq!(pairs.len(),if step%2==0 {3}else{0});
            for (a,b) in pairs {assert_eq!(row[b],row[a]+2*n);}
            for (slot,&i) in row.iter().enumerate() {
                if (2..=4).contains(&tm[i%n].bucket) {
                    let original=co[step%64][slot];
                    assert_eq!(tm[i%n].base,tm[original%n].base);
                    assert_eq!(tm[i%n].view,(step/64)%2);
                    assert_eq!(i/n,original/n,"same phrase/side for each widened scene");
                }else{assert_eq!(i,values[step][slot],"all anchor slots unchanged");}
                if step<1280 {counts[i]+=1;}
            }
        }
        for bucket in 2..=4 {
            let seen:Vec<_>=counts.iter().enumerate().filter(|(i,c)|tm[*i%n].bucket==bucket&&**c>0).collect();
            assert_eq!(seen.len(),128);assert!(seen.iter().all(|(_,c)|**c==10));
            assert_eq!(seen.iter().map(|(i,_)|&tm[*i%n].base).collect::<BTreeSet<_>>().len(),32);
        }
        for step in [0,15,16,63,64,127,128,255,256,1279,3839] {assert_eq!(restored.at(6144+step).unwrap(),cover[step]);}
        // Production continuation retains the original absolute tape, never rewinds it.
        let mut exposure=production.clone();exposure.paired=Some(restored.clone());
        let mut extra=vec![0usize;4*n];
        for step in 7424..9984 {
            let row=exposure.training_draw(step);assert_eq!(row,cover[step-6144]);
            for i in row {extra[i]+=1;}
        }
        for bucket in 2..=4 {
            let seen:Vec<_>=extra.iter().enumerate().filter(|(i,c)|tm[*i%n].bucket==bucket&&**c>0).collect();
            assert_eq!(seen.len(),128);assert!(seen.iter().all(|(_,c)|**c==20));
        }
        assert_eq!(paired_evaluation(7424,2560,false).primary_steps,vec![8448,9472,9984]);
        let wording=selector_phrase_rows(&cover,&tm,1280,64).unwrap();
        assert_eq!(wording[..1280],cover[..1280]);
        let phrase_tape=PairTape {rows:wording.clone(),..restored.clone()};
        let back:PairTape=binary::from_slice(&binary::to_vec(&phrase_tape).unwrap()).unwrap();
        assert_eq!(back,phrase_tape);
        let mut changed=0;let mut phrase_bases=[BTreeSet::new(),BTreeSet::new(),BTreeSet::new()];
        for step in 1280..3840 {
            assert_eq!(back.at(6144+step).unwrap(),wording[step]);
            for (&before,&after) in cover[step].iter().zip(&wording[step]) {
                if before==after {continue;}
                let (a,b)=(&tm[before%n],&tm[after%n]);
                assert!((2..=4).contains(&a.bucket));assert_eq!(a.bucket,b.bucket);
                assert_eq!(a.base,b.base);assert_eq!((a.view,b.view),(0,3));
                assert_eq!(before/(2*n),after/(2*n));assert_eq!((step/64)%4,0);
                phrase_bases[a.bucket-2].insert(&a.base);changed+=1;
            }
        }
        assert_eq!(changed,1920);assert!(phrase_bases.iter().all(|s|s.len()==32));
        assert!(selector_phrase_rows(&cover,&tm,1281,64).is_err());
        assert!(selector_phrase_rows(&cover,&tm,3840,64).is_err());
        let mut missing_view=tm.clone();missing_view.retain(|m|m.view!=3);
        assert!(selector_phrase_rows(&cover,&missing_view,1280,64).is_err());
        assert!(!no_further_progress(&[(425,79,0),(380,67,0),(392,68,3),(400,70,4)]));
        assert!(no_further_progress(&[(425,79,0),(380,67,0),(392,68,3),(392,68,3),(392,68,3)]));
        let mut wrong=restored.clone();wrong.rows=values.clone();assert!(verify_pair_tape(&cover_parent,&tm,&wrong).is_err());
        wrong=restored.clone();wrong.rows[64]=wrong.rows[0];assert!(verify_pair_tape(&cover_parent,&tm,&wrong).is_err());
        let mut duplicate = tm.clone(); duplicate.push(tm[2*1024].clone());
        assert!(pair_tapes(&p,&duplicate,256,3840).is_err());
        let mut missing=tm.clone(); missing.remove(2*1024); assert!(pair_tapes(&p,&missing,256,3840).is_err());
        assert!(pair_tapes(&p,&tm,256,3841).is_err());
        assert_eq!(paired_evaluation(6400,3584,false).primary_steps,vec![7424,8448,9984]);
        let weighted=paired_evaluation(6144,3840,false);
        assert_eq!(weighted.primary_steps,vec![6400,7424,8448,9984]);
        assert_eq!(weighted.screen_steps,vec![6176,6208,6272,6912,7936,8960,9472]);
        let lr=paired_evaluation(6144,1280,false);
        assert_eq!(lr.primary_steps,vec![6400,7424]);
        assert_eq!(lr.screen_steps,vec![6176,6208,6272,6912]);
        println!("TAPE_STEPS=3840 MODEL_UPDATES=0 GENERATION=0 TEACHER=0 full_binary_readback=EXACT");
    }

    // Explicit, training-free adapter for a closed study. It never calls run or
    // study_observe, and reporting cannot create observations or repair history.
    struct PosthocSelector {
        binding: binary::Value,
        inputs: BTreeMap<PathBuf, String>,
        checkpoints: Vec<PathBuf>,
        original: Vec<Episode>,
        original_meta: Vec<Meta>,
        original_rows: Vec<Vec<binary::Value>>,
        flipped: Vec<Episode>,
        meta: Vec<Meta>,
        parity: Vec<usize>,
    }
    fn posthoc_mapping(
        original: &[Episode],
        om: &[Meta],
        flipped: &[Episode],
        fm: &[Meta],
    ) -> Result<Vec<usize>> {
        let selected: Vec<_> = original.iter().zip(om)
            .filter(|(_, m)| (2..=4).contains(&m.bucket)).collect();
        if selected.len() != flipped.len() || flipped.len() != fm.len() {
            return Err(bad("posthoc mapping denominator"));
        }
        let mut ids = BTreeSet::new();
        let mut bases: BTreeMap<&str, BTreeSet<usize>> = BTreeMap::new();
        let mut counts = [0; 3];
        for (((e, m), f), n) in selected.iter().zip(flipped).zip(fm) {
            // Validate stored cases; never use a regenerated case for inference.
            let expected = flip_selection(e, m)?;
            if digest(&expected)? != digest(&(f, n))? || !ids.insert(&f.id)
                || n.view > 3 || !bases.entry(&n.base).or_default().insert(n.view)
            {
                return Err(bad("posthoc mapping content/order/view"));
            }
            counts[n.bucket - 2] += 1;
        }
        if counts.iter().any(|&n| n != counts[0])
            || bases.values().any(|v| v.len() != 4)
        {
            return Err(bad("posthoc bucket/base coverage"));
        }
        Ok((2..=4).flat_map(|bucket| fm.iter().enumerate()
            .filter(move |(_, m)| m.bucket == bucket)
            .take(if bucket == 2 { 6 } else { 5 }).map(|(i, _)| i)).collect())
    }
    impl PosthocSelector {
        fn load(root: &Path, executable: &Path) -> Result<Self> {
            let study = study_read_bound(root, Some(executable))?;
            let (flipped, meta): (Vec<Episode>, Vec<Meta>) = read(&root.join("diagnostic.r3b"))?;
            if study.schema != 3 || study.tiny || study.parent_step != 6144 || flipped.len() != 192 {
                return Err(bad("posthoc requires the preserved selector study"));
            }
            let mut paths: BTreeSet<PathBuf> = [
                root.join("study.r3b"), root.join("study-ready.r3b"),
                root.join("diagnostic.r3b"), root.join("parent-audit.r3b"),
                study.parent.join("plan.r3b"), study.parent_checkpoint.clone(), executable.into(),
            ].into_iter().collect();
            let mut checkpoints = vec![];
            let mut identities = vec![];
            let mut originals = vec![];
            let mut original = vec![];
            let mut original_meta = vec![];
            for arm in &study.tie_break {
                let arm_root = root.join(arm);
                // Only directly read inputs and the step7168 panel, not the whole artifact tree.
                for entry in std::fs::read_dir(&arm_root)? {
                    let entry = entry?;
                    let name = entry.file_name().to_string_lossy().into_owned();
                    if name.starts_with("eval-7168-dev512") || ["plan.r3b", "corpus.r3cor",
                        "transfer.r3cor", "initial.r3m", "metadata.r3b", "tokenizer.r3b",
                        "variants.r3cor", "question-variants.r3b", "selectors.r3cor",
                        "selector-metadata.r3b"].contains(&name.as_str()) {
                        paths.insert(entry.path());
                    }
                }
                let p = plan_read_bound(&arm_root, &study.source, &study.binary)?;
                let dev = verified_corpus(&arm_root.join("corpus.r3cor"), &p.corpus)?.validation;
                let (_, dm, _) = verified_metadata(&arm_root, &p)?;
                if dev.len() != 512 || dm.len() != 512 {
                    return Err(bad("posthoc original primary denominator"));
                }
                if !original.is_empty() && (digest(&original)? != digest(&dev)?
                    || digest(&original_meta)? != digest(&dm)?) {
                    return Err(bad("posthoc arms differ in frozen inputs"));
                }
                let rows = binary::read_value_records(&arm_root.join("eval-7168-dev512.r3rows"))?;
                let cp = PathBuf::from(rows.first().and_then(|h| h["checkpoint"].as_str())
                    .ok_or_else(|| bad("posthoc original checkpoint missing"))?);
                if !cp.canonicalize()?.starts_with(arm_root.canonicalize()?) {
                    return Err(bad("posthoc checkpoint outside authorized arm"));
                }
                paths.insert(cp.clone());
                let l = checkpoint::load(&cp, Device::Cpu, false)?;
                let state = l.manifest.training.as_ref().ok_or_else(|| bad("posthoc native state"))?;
                if state.step != 7168 || l.tokenizer.id() != study.tokenizer
                    || l.model.config != p.architecture || p.tiny
                    || p.fork.as_ref().is_none_or(|f| f.parent_adam != study.parent_adam
                        || f.parent_state != study.parent_state || f.arm != *arm)
                {
                    return Err(bad("posthoc equal-step parent/native binding"));
                }
                let panel = audit_panel(&arm_root, &p, 7168, "dev512", &dev, &dm, &l.tokenizer)?;
                identities.push(binary::record!({"arm":arm,"checkpoint":cp,"physical":file_hash(&cp)?,
                    "model":l.model.weight_hash()?,"native_content":l.manifest.model_content_digest,
                    "state":digest(state)?,"training_state":state,"policy":digest(&p)?,
                    "historical_source":p.source,"historical_binary":p.binary,"primary":panel}));
                checkpoints.push(cp);
                originals.push(rows[1..].to_vec());
                original = dev;
                original_meta = dm;
            }
            let parity = posthoc_mapping(&original, &original_meta, &flipped, &meta)?;
            if parity.len() != 16 || meta.iter().map(|m| &m.base).collect::<BTreeSet<_>>().len() != 48 {
                return Err(bad("posthoc fixed panel coverage"));
            }
            let inputs: BTreeMap<_, _> = paths.into_iter().map(|p| {
                let hash = file_hash(&p)?; Ok((p, hash))
            }).collect::<Result<_>>()?;
            let binding = binary::record!({"contract":"R3-ACCEPTANCE-AND-QUALITY-CLOSURE-1.0",
                "label":"POSTHOC_EQUAL_STEP_SELECTOR_OBSERVATION","source":source_digest()?,
                "binary":file_hash(&std::env::current_exe()?)?,"features":"accelerate; no test-support",
                "backend":"CPU/F32/Accelerate","threads":1,"models":identities,"inputs":inputs,
                "original":digest(&original)?,"flipped":digest(&flipped)?,"mapping":digest(&meta)?,
                "parity_indices":parity,"decoding":"normal-greedy-strict-utf8-eos",
                "generation_limit":416,"teacher_limit":0,"optimizer_limit":0});
            Ok(Self { binding, inputs, checkpoints, original, original_meta,
                original_rows: originals, flipped, meta, parity })
        }
        fn unchanged(&self) -> Result<()> {
            for (path, hash) in &self.inputs {
                if file_hash(path)? != *hash { return Err(bad("posthoc original input changed")); }
            }
            Ok(())
        }
        fn cases(&self, parity: bool) -> (Vec<Episode>, Vec<Meta>) {
            if !parity { return (self.flipped.clone(), self.meta.clone()); }
            self.parity.iter().map(|&i| {
                let id = self.meta[i].source_id.as_ref().unwrap();
                let j = self.original.iter().position(|e| e.id == *id).unwrap();
                (self.original[j].clone(), self.original_meta[j].clone())
            }).unzip()
        }
        fn rows(&self, output: &Path, arm: usize, parity: bool) -> Result<Vec<binary::Value>> {
            let (es, ms) = self.cases(parity);
            let name = format!("arm-{arm}-{}", if parity { "parity" } else { "selector" });
            let path = output.join(format!("{name}.r3rows"));
            if !path.exists() { return Ok(vec![]); }
            let all = binary::read_value_records(&path)?;
            let binding = binary::record!({"registration":digest(&self.binding)?,"arm":arm,
                "checkpoint":self.checkpoints[arm],"dataset":digest(&es)?,"panel":name});
            if all.first() != Some(&binding) || all.len() > es.len() + 1 {
                return Err(bad("posthoc raw binding/count"));
            }
            let rows = &all[1..];
            let l = checkpoint::load(&self.checkpoints[arm], Device::Cpu, false)?;
            for (i, row) in rows.iter().enumerate() {
                call_attempt(output, &name, "generation", &binding, &es[i], i, Some(row))?;
                verify_generated(row, &l.tokenizer)?;
                let prompt = l.tokenizer.prepare(&es[i].request, l.model.config.context as u32, &l.model.config.id()?)?;
                if row["native_prompt_digest"] != prompt.token_digest {
                    return Err(bad("posthoc native framing changed"));
                }
                if parity {
                    let old = self.original_rows[arm].iter().find(|r| r["id"] == es[i].id).unwrap();
                    for field in ["raw_tokens", "actual", "error", "finish_reason", "eos_index", "native_prompt_digest"] {
                        if old[field] != row[field] { return Err(bad("posthoc output parity changed")); }
                    }
                }
            }
            score(rows, &es[..rows.len()], &ms[..rows.len()])?;
            Ok(rows.to_vec())
        }
        fn observe_panel(&self, output: &Path, arm: usize, parity: bool,
            control: &mut recovery::RunControl) -> Result<()> {
            let (es, _) = self.cases(parity);
            let name = format!("arm-{arm}-{}", if parity { "parity" } else { "selector" });
            let mut rows = self.rows(output, arm, parity)?;
            let l = checkpoint::load(&self.checkpoints[arm], Device::Cpu, false)?;
            let binding = binary::record!({"registration":digest(&self.binding)?,"arm":arm,
                "checkpoint":self.checkpoints[arm],"dataset":digest(&es)?,"panel":name});
            let path = output.join(format!("{name}.r3rows"));
            let new = !path.exists();
            let mut f = std::fs::OpenOptions::new().create_new(new).append(true).open(&path)?;
            if new { append_row(&mut f, &binding)?; std::fs::File::open(output)?.sync_all()?; }
            for e in es.iter().skip(rows.len()) {
                control.check("posthoc_next")?;
                let attempt = prepare_call(output, &name, "generation", &binding, e, rows.len())?;
                let mut row = match recovery::observe_generation(&l, e, &e.request, control, false) {
                    recovery::ObservedCall::Returned(row) => row,
                    recovery::ObservedCall::NotInvoked(_) => {
                        resolve_call(&attempt, None, control)?;
                        control.stop_result()?;
                        return Err(bad("posthoc preparation failed without entry"));
                    }
                };
                row["attempt"] = binary::record!(attempt.file_name().unwrap().to_string_lossy());
                append_row(&mut f, &row)?;
                resolve_call(&attempt, Some(&row), control)?;
                rows.push(row);
                control.check("posthoc_row_durable")?;
            }
            // Pure reread also checks parity before either selector panel can start.
            let verified = self.rows(output, arm, parity)?;
            let summary = self.panel_score(output, arm, parity, &verified)?;
            let dest = output.join(format!("{name}-complete.r3b"));
            if dest.exists() || pending_path(&dest).exists() {
                let old: PanelResult = read_confirmed(&dest)?;
                if old != summary { return Err(bad("posthoc summary differs")); }
            } else { publish_confirmed(&dest, &summary)?; }
            println!("POSTHOC panel={name} returned={} exact={} errors={} raw_tokens={} teacher=0 optimizer=0",
                verified.len(), summary.exact, summary.errors,
                verified.iter().map(|r| r["raw_tokens"].as_array().unwrap().len()).sum::<usize>());
            Ok(())
        }
        fn panel_score(&self, output: &Path, arm: usize, parity: bool, rows: &[binary::Value]) -> Result<PanelResult> {
            let (es, ms) = self.cases(parity);
            let mut s = score(rows, &es, &ms)?;
            s.step = 7168;
            s.panel = format!("arm-{arm}-{}", if parity { "parity" } else { "selector" });
            s.model = self.binding["models"][arm]["model"].as_str().ok_or_else(|| bad("posthoc model identity"))?.into();
            s.dataset = digest(&es)?;
            s.raw_hash = file_hash(&output.join(format!("{}.r3rows", s.panel)))?;
            Ok(s)
        }
        fn report(&self, output: &Path) -> Result<binary::Value> {
            let mut results = vec![];
            let mut flips = vec![];
            let mut both = vec![];
            for arm in 0..2 {
                for parity in [true, false] {
                    let rows = self.rows(output, arm, parity)?;
                    let summary = self.panel_score(output, arm, parity, &rows)?;
                    let name = format!("arm-{arm}-{}-complete.r3b", if parity { "parity" } else { "selector" });
                    let saved: PanelResult = read_confirmed(&output.join(name))?;
                    if saved != summary { return Err(bad("posthoc complete summary differs")); }
                    if !parity {
                        let pairs = posthoc_pairs(&self.original_rows[arm], &rows, &self.flipped, &self.meta)?;
                        both.push(pairs["rows"].as_array().unwrap().iter().map(|r| r["both"] == true).collect::<Vec<_>>());
                        flips.push(rows.iter().map(|r| r["exact_match"] == true).collect::<Vec<_>>());
                        results.push(binary::record!({"arm":arm,"score":summary,"pairs":pairs}));
                    }
                }
            }
            let gain_loss = |v: &[Vec<bool>]| [v[0].iter().zip(&v[1]).filter(|(a,b)| !**a && **b).count(),
                v[0].iter().zip(&v[1]).filter(|(a,b)| **a && !**b).count()];
            self.unchanged()?;
            Ok(binary::record!({"registration":digest(&self.binding)?,"arms":results,
                "flipped_gain_loss":gain_loss(&flips),"both_gain_loss":gain_loss(&both),
                "originals_unchanged":true,"new_optimizer":0,"new_teacher":0,
                "quality_improved_this_run":false,"candidate_promotion":false,"final200":"NOT_OPENED"}))
        }
    }
    fn posthoc_pairs(original: &[binary::Value], flipped: &[binary::Value], es: &[Episode], ms: &[Meta]) -> Result<binary::Value> {
        let mut summary = selector_pairs(original, flipped, es, ms)?;
        let valid = |r: &binary::Value| r["generation_completed"] == true && r["error"].is_null()
            && r["finish_reason"] == "stop" && r["actual"].is_string();
        let mut rows = vec![];
        for ((r, e), m) in flipped.iter().zip(es).zip(ms) {
            let old = original.iter().find(|r| r["id"] == *m.source_id.as_ref().unwrap()).unwrap();
            let got = r["actual"].as_str().map(citations).transpose();
            let expected = citations(&e.answer)?;
            let mut classes = vec![];
            if let Ok(Some(ids)) = &got {
                for id in ids {
                    classes.push(if expected.contains(id) { "selected" }
                        else if e.request.evidence.items.iter().any(|x| x.event_id == *id) { "other_provided" }
                        else { "absent" });
                }
            }
            rows.push(binary::record!({"id":e.id,"source":m.source_id,"bucket":m.bucket,"base":m.base,"view":m.view,
                "original":old["exact_match"],"flipped":r["exact_match"],"both":old["exact_match"]==true && r["exact_match"]==true,
                "same_valid_output":valid(old) && valid(r) && old["actual"]==r["actual"],
                "citation_ids":got.as_ref().ok().and_then(|v| v.as_ref()),
                "citation_classes":classes,"citation_parse_error":got.is_err(),
                "no_citation":classes.is_empty(),"multiple_citations":classes.len()>1}));
        }
        summary["same_output"] = binary::record!(rows.iter().filter(|r| r["same_valid_output"]==true).count());
        summary["rows"] = binary::record!(rows);
        Ok(summary)
    }
    #[test]
    fn posthoc_mapping_and_binary_pair_counts() -> Result<()> {
        let (es, ms) = generate(1, 1, 20260919)?;
        let (flipped, fm) = selector_panel(&es, &ms, false)?;
        assert_eq!(posthoc_mapping(&es, &ms, &flipped, &fm)?.len(), 12);
        for fault in 0..4 {
            let mut broken = flipped.clone();
            match fault {
                0 => { broken.swap(0, 1); }
                1 => { broken.pop(); }
                2 => { broken[0].answer.push(' '); }
                _ => { broken[0].request.input.push(' '); }
            }
            assert!(posthoc_mapping(&es, &ms, &broken, &fm).is_err());
        }
        let tok = ByteBpe::train(&[b"pair fixture".to_vec()], &neural::hash(b"pair fixture"), 264)?;
        let row = |e: &Episode| -> Result<binary::Value> {
            let body = tok.encode(e.answer.as_bytes())?;
            let mut ids = body.clone(); ids.push(EOS);
            Ok(binary::record!({"row_version":2,"id":e.id,"question":e.request.input,
                "generated_evidence":e.request.evidence,"expected":e.answer,"actual":e.answer,
                "generation_completed":true,"error":null,"finish_reason":"stop","exact_match":true,
                "raw_tokens":ids,"generation":{"tokens":body,"finish":"stop","generated":ids.len()}}))
        };
        let original = es.iter().map(row).collect::<Result<Vec<_>>>()?;
        let flipped_rows = flipped.iter().map(row).collect::<Result<Vec<_>>>()?;
        let d = tempfile::tempdir()?;
        let path = d.path().join("pairs.r3rows");
        let mut f = std::fs::File::create(&path)?;
        for r in &flipped_rows { append_row(&mut f, r)?; }
        let mut loaded = binary::read_value_records(&path)?;
        for r in &loaded { verify_generated(r, &tok)?; }
        assert_eq!(score(&loaded, &flipped, &fm)?.exact, 12);
        let pairs = posthoc_pairs(&original, &loaded, &flipped, &fm)?;
        assert_eq!(pairs["pairs"], binary::record!([12, 0, 0, 0]));
        assert_eq!(pairs["both_base4"], 3);
        loaded[0]["expected"] = binary::record!("untrusted answer");
        assert!(score(&loaded, &flipped, &fm).is_err());
        loaded = flipped_rows;
        let mut original = original;
        for r in original.iter_mut().chain(&mut loaded) {
            r["actual"] = binary::Value::Null;
            r["error"] = binary::record!("failed");
            r["exact_match"] = binary::record!(false);
        }
        assert_eq!(posthoc_pairs(&original, &loaded, &flipped, &fm)?["same_output"], 0);
        let target = citations(&flipped[0].answer)?[0];
        let other = flipped[0].request.evidence.items.iter().find(|r| r.event_id != target).unwrap().event_id;
        loaded[0]["actual"] = binary::record!(format!("x [event:{target}] [event:{other}] [event:999999999]"));
        let mixed = posthoc_pairs(&original, &loaded, &flipped, &fm)?;
        // citations() sorts IDs; classes retain that ID order, not textual order.
        let classes: BTreeSet<_> = mixed["rows"][0]["citation_classes"].as_array().unwrap()
            .iter().map(|v| v.as_str().unwrap()).collect();
        assert_eq!(classes, BTreeSet::from(["selected", "other_provided", "absent"]));
        assert_eq!(mixed["rows"][0]["multiple_citations"], true);
        assert_eq!(mixed["same_output"], 0);
        println!("POSTHOC_FIXTURE mapping_faults=4 full_pairs=12 null_pairs=12 mixed_citations=1 optimizer=0 generation=0 teacher=0");
        Ok(())
    }
    #[test]
    fn selector_phrase_only_keeps_serialized_facts_and_targets() -> Result<()> {
        let (es,ms)=generate(8,0,20260920)?;let mut checked=0;
        for (a,m) in es.iter().zip(&ms).filter(|(_,m)|(2..=4).contains(&m.bucket)&&m.view==0) {
            let j=ms.iter().position(|n|n.base==m.base&&n.view==3).unwrap();
            for variant in [false,true] { for flip in [false,true] {
                let a=if variant {changed_question(a,phrases(Intent::Current)[0])?}else{a.clone()};
                let (a,b)=if flip {(flip_selection(&a,m)?.0,flip_selection(&es[j],&ms[j])?.0)}else{(a,es[j].clone())};
                verify_selector_phrase_change(&a,&b)?;checked+=1;
                let mut broken=b.clone();broken.answer.push(' ');assert!(verify_selector_phrase_change(&a,&broken).is_err());
                let mut broken=b.clone();broken.request.evidence.items.reverse();assert!(verify_selector_phrase_change(&a,&broken).is_err());
                let mut broken=b.clone();broken.request.evidence.items[0].observed_at=Some(0);assert!(verify_selector_phrase_change(&a,&broken).is_err());
                assert!(verify_selector_phrase_change(&a,&a).is_err());
            }}
        }
        assert_eq!(checked,96);println!("PHRASE_CASES=96 malformed=384 optimizer=0 generation=0 teacher=0");Ok(())
    }
    #[test]
    #[ignore = "explicit closed paired arm/new output; existing raw only, optimizer/generation/teacher0"]
    fn paired_raw_selection_strata() -> Result<()> {
        let env = |name| std::env::var(name).map(PathBuf::from).map_err(|_| bad("explicit raw diagnostic paths required"));
        let root = env("R3_RAW_PAIR_ROOT")?;
        let output = env("R3_RAW_PAIR_OUTPUT")?;
        let executable = env("R3_RAW_PAIR_EXECUTABLE")?;
        let p: Plan = read(&root.join("plan.r3b"))?;
        plan_read_bound(&root, &p.source, &file_hash(&executable)?)?;
        let end = history(&root, &p)?.pop().ok_or_else(|| bad("closed endpoint missing"))?;
        if end.resume || end.phase.as_deref() != Some("Finished") || end.step != p.config.max_steps {
            return Err(bad("raw diagnostic requires a completed endpoint"));
        }
        let f = p.fork.as_ref().ok_or_else(|| bad("paired fork missing"))?;
        let s: Study = read(&f.study.join("study.r3b"))?;
        if file_hash(&f.study.join("study.r3b"))? != f.study_hash
            || file_hash(&f.study.join("diagnostic.r3b"))? != s.diagnostic_hash {
            return Err(bad("raw diagnostic study binding"));
        }
        let dev = verified_corpus(&root.join("corpus.r3cor"), &p.corpus)?.validation;
        let (_, dm, _) = verified_metadata(&root, &p)?;
        let (flips, fm): (Vec<Episode>, Vec<Meta>) = read(&f.study.join("diagnostic.r3b"))?;
        posthoc_mapping(&dev, &dm, &flips, &fm)?;
        let tok = ByteBpe::load(&root.join("tokenizer.r3b"))?;
        // Finalization changes resume metadata after evaluation, so its physical
        // file may differ. audit_panel verifies each physical file and full state;
        // require the same model as the already verified terminal at this step.
        let endpoint = checkpoint::load(&root.join(&end.checkpoint), Device::Cpu, false)?;
        let model = endpoint.model.weight_hash()?;
        let mut scores = vec![];
        let mut panels = vec![];
        for (name, es, ms) in [("dev512", &dev, &dm), ("selector192", &flips, &fm)] {
            let score = audit_panel(&root, &p, end.step, name, es, ms, &tok)?;
            let raw = binary::read_value_records(&root.join(format!("eval-{:04}-{name}.r3rows", end.step)))?;
            if score.model != model || score.errors != 0 {
                return Err(bad("strata endpoint binding or generation failure"));
            }
            scores.push(score); panels.push(raw);
        }
        let mut strata: BTreeMap<String, [usize; 5]> = BTreeMap::new();
        let mut error_classes: BTreeMap<String, usize> = BTreeMap::new();
        let mut duplicates = [0usize; 2];
        let mut outcomes = vec![];
        for ((e, m), r) in flips.iter().zip(&fm).zip(&panels[1][1..]) {
            let source = m.source_id.as_ref().unwrap();
            let at = dev.iter().position(|e| &e.id == source).ok_or_else(|| bad("source missing"))?;
            let original = &dev[at]; let old = &panels[0][at + 1];
            if resolve(&original.request)? != original.answer || resolve(&e.request)? != e.answer {
                return Err(bad("request-only selection disagrees with frozen answer"));
            }
            let a = old["exact_match"] == true; let b = r["exact_match"] == true;
            let position = |e: &Episode| -> Result<usize> {
                let ids = citations(&e.answer)?;
                if ids.len() != 1 { return Err(bad("one selected record required")); }
                e.request.evidence.items.iter().position(|v| v.event_id == ids[0]).ok_or_else(|| bad("selected record absent"))
            };
            let digits = original.request.evidence.items[position(original)?].original_excerpt
                .split_whitespace().next().unwrap_or("").chars().filter(char::is_ascii_digit).count();
            let numeric = original.answer.chars().next().is_some_and(|c| c.is_ascii_digit());
            for key in ["all".to_owned(), format!("bucket:{}", m.bucket), format!("view:{}", m.view),
                format!("bucket:{}:view:{}", m.bucket, m.view), format!("entity_digits:{digits}"),
                format!("bucket:{}:digits:{digits}", m.bucket), format!("numeric:{numeric}"),
                format!("original_position:{}", position(original)?), format!("bucket:{}:original_position:{}", m.bucket, position(original)?)] {
                let counts = strata.entry(key).or_default();
                for (v, add) in counts.iter_mut().zip([usize::from(a), usize::from(b), usize::from(a && b), usize::from(old["actual"] == r["actual"]), 1]) { *v += add; }
            }
            if m.view == 2 {
                let zero = dev.iter().zip(&dm).find(|(_, z)| z.base == m.base && z.view == 0).unwrap().0;
                duplicates[1] += 1;
                duplicates[0] += usize::from(zero.request.input == original.request.input && zero.request.evidence == original.request.evidence);
            }
            for (side, row, target, other) in [("original", old, &original.answer, &e.answer), ("flip", r, &e.answer, &original.answer)] {
                let actual = row["actual"].as_str().ok_or_else(|| bad("normal text missing"))?;
                let body = |s: &str| s.split_once(" [event:").map(|(x, _)| x.to_owned());
                let class = if actual == target { "exact" } else if actual == other { "other_record_whole" }
                    else if actual.contains("근거") && !actual.contains("[event:") { "abstention" }
                    else if citations(actual).is_ok_and(|ids| !ids.is_empty() && Some(ids) == citations(target).ok()) { "selected_citation_wrong_body" }
                    else if body(actual).is_some() && body(actual) == body(target) { "selected_body_wrong_citation" }
                    else { "other_format_or_content" };
                *error_classes.entry(format!("{side}:{class}")).or_default() += 1;
            }
            outcomes.push(binary::record!({"id":e.id,"source":source,"bucket":m.bucket,"view":m.view,"original":a,"flipped":b,"both":a&&b}));
        }
        let pairs = posthoc_pairs(&panels[0][1..], &panels[1][1..], &flips, &fm)?;
        if strata["all"][2] != pairs["pairs"][0].as_u64().unwrap() as usize { return Err(bad("pair recount disagreement")); }
        let result = binary::record!({"scope":"DERIVED_EXISTING_RAW_DEVELOPMENT","source":source_digest()?,
            "binary":file_hash(&std::env::current_exe()?)?,"policy":digest(&p)?,"step":end.step,
            "checkpoint":end.checkpoint_hash,"metadata":p.metadata,"diagnostic":s.diagnostic_hash,
            "panel_checkpoint_files":panels.iter().map(|r| &r[0]["physical"]).collect::<Vec<_>>(),
            "scores":scores,"stratum_fields":["original","flip","both","same_output","planned"],
            "strata":strata,"first_error_classes":error_classes,"view2_same_as_view0":duplicates,
            "pairs":outcomes,"optimizer":0,"generation":0,"teacher":0,"candidate_promotion":false});
        publish_confirmed(&output, &result)?;
        println!("RAW_STRATA step={} full={} flipped={} groups={strata:?} error_classes={error_classes:?} view2_duplicate={duplicates:?} optimizer=0 generation=0 teacher=0 output_sha256={}",
            end.step, scores[0].exact, scores[1].exact, file_hash(&output)?);
        Ok(())
    }
    #[test]
    #[ignore = "explicit closed study/output/mode; register/report are read-only on originals; observe has at most416 SMALL generations"]
    fn posthoc_equal_step_selector() -> Result<()> {
        if cfg!(feature = "test-support") || !cfg!(feature = "accelerate")
            || std::env::var("VECLIB_MAXIMUM_THREADS").as_deref() != Ok("1")
            || std::env::var("RAYON_NUM_THREADS").as_deref() != Ok("1") {
            return Err(bad("posthoc requires production Accelerate, threads1"));
        }
        let env = |k| std::env::var(k).map_err(|_| bad("explicit posthoc paths/mode required"));
        let output = PathBuf::from(env("R3_POSTHOC_OUTPUT")?);
        let data = PosthocSelector::load(Path::new(&env("R3_POSTHOC_STUDY")?), Path::new(&env("R3_POSTHOC_EXECUTABLE")?))?;
        let mode = env("R3_POSTHOC_MODE")?;
        if mode == "register" {
            std::fs::create_dir(&output)?;
            publish_confirmed(&output.join("registration.r3b"), &data.binding)?;
            data.unchanged()?;
            println!("POSTHOC_REGISTERED {}", data.binding);
            return Ok(());
        }
        let registration: binary::Value = read_confirmed(&output.join("registration.r3b"))?;
        if registration != data.binding { return Err(bad("posthoc registration changed")); }
        let mut index = 0;
        let mut used = 0;
        let mut finished = false;
        while output.join(format!("command-{index:04}-started.r3b")).exists() {
            let start = output.join(format!("command-{index:04}-started.r3b"));
            let r: binary::Value = read_confirmed(&output.join(format!("command-{index:04}-finished.r3b")))
                .map_err(|_| bad("posthoc started without terminal: usage UNKNOWN, blocked"))?;
            if r["start"] != file_hash(&start)? || r["registration"] != digest(&registration)?
                || r["control"]["teacher_calls"] != 0 || finished {
                return Err(bad("posthoc command chain"));
            }
            used += r["control"]["generation_calls"].as_u64().ok_or_else(|| bad("posthoc usage UNKNOWN"))?;
            finished = r["complete"] == true;
            if finished && (!r["error"].is_null() || r["control"]["terminal_reason"] != "COMPLETED"
                || r["control"]["observed_conditions"] != binary::record!([]) || used != 416) {
                return Err(bad("posthoc completion/usage mismatch"));
            }
            if !finished && (r["control"]["observed_conditions"] != binary::record!(["TIME_BUDGET"])
                || r["resume"] != true) { return Err(bad("posthoc failed command is sticky")); }
            index += 1;
        }
        if used > 416 { return Err(bad("posthoc generation budget")); }
        if mode == "report" {
            if !finished { return Err(bad("posthoc incomplete command")); }
            println!("POSTHOC_REPORT {}", data.report(&output)?);
            return Ok(());
        }
        if mode != "observe" || finished { return Err(bad("posthoc mode/already complete")); }
        let start = output.join(format!("command-{index:04}-started.r3b"));
        write(&start, &binary::record!({"registration":digest(&registration)?,"prior_calls":used}))?;
        let mut control = recovery::RunControl::command(false)?;
        control.set_call_limits(416 - used as usize, 0);
        let result = (|| -> Result<()> {
            for parity in [true, false] { for arm in 0..2 {
                data.observe_panel(&output, arm, parity, &mut control)?;
            }}
            data.unchanged()
        })();
        if let Err(e) = &result { control.classify_error(e); }
        let result = control.seal_terminal().and(result);
        publish_confirmed(&output.join(format!("command-{index:04}-finished.r3b")),
            &binary::record!({"start":file_hash(&start)?,"registration":digest(&registration)?,
                "control":control.receipt(),"complete":result.is_ok(),
                "resume":result.is_err() && control.receipt()["observed_conditions"]==binary::record!(["TIME_BUDGET"]),
                "error":result.as_ref().err().map(ToString::to_string)}))?;
        println!("POSTHOC_COMMAND {}", control.receipt());
        result
    }

    #[test]
    fn fresh_exposure_digest_preserves_order_above_aggregate_codec_limit() {
        // The actual two-epoch five-task report contains more than one million
        // tokens. Keep the codec limit and bind each bounded native row first.
        let rows: Vec<Vec<u32>> = (0..10_240).map(|n| vec![n as u32; 128]).collect();
        assert!(digest(&rows).is_err());
        let mut hashes: Vec<_> = rows.iter().map(digest).collect::<Result<_>>().unwrap();
        let bound = digest(&hashes).unwrap();
        let record = binary::record!({"unchanged_five_tasks":bound,"rows":rows.len()});
        let bytes = binary::to_storage_vec(&record).unwrap();
        assert_eq!(binary::from_slice::<binary::Value>(&bytes).unwrap(), record);
        hashes.swap(0, 1);
        assert_ne!(digest(&hashes).unwrap(), bound);
        hashes.swap(0, 1);
        let mut changed = rows[0].clone();
        changed[0] += 1;
        hashes[0] = digest(&changed).unwrap();
        assert_ne!(digest(&hashes).unwrap(), bound);
    }
    #[test]
    fn selector_involution_labels_balance_and_negative_cases() {
        let (train, ms) = generate(256, 0, 20260919).unwrap();
        let mut counts = [0usize; 8];
        for (e, m) in train
            .iter()
            .zip(&ms)
            .filter(|(_, m)| (2..=4).contains(&m.bucket))
        {
            let (flipped, fm) = flip_selection(e, m).unwrap();
            let (back, _) = flip_selection(&flipped, &fm).unwrap();
            assert_eq!(digest(&e.request).unwrap(), digest(&back.request).unwrap());
            assert_eq!(e.answer, back.answer);
            assert_eq!(flipped.answer, resolve(&flipped.request).unwrap());
            assert_ne!(
                citations(&e.answer).unwrap(),
                citations(&flipped.answer).unwrap()
            );
            assert_eq!(fm.base, m.base);
            assert_eq!(fm.view, m.view);
            assert_eq!(
                fm.entities[0],
                question_intent(&flipped.request.input).unwrap().0
            );
            if m.bucket < 4 {
                assert_eq!(
                    digest(&e.request.evidence).unwrap(),
                    digest(&flipped.request.evidence).unwrap()
                );
            } else {
                assert_eq!(e.request.input, flipped.request.input);
                let mut restored = flipped.request.evidence.clone();
                for (r, o) in restored.items.iter_mut().zip(&e.request.evidence.items) {
                    r.observed_at = o.observed_at;
                }
                assert_eq!(
                    digest(&restored).unwrap(),
                    digest(&e.request.evidence).unwrap()
                );
            }
            counts[m.bucket] += 1;
        }
        assert_eq!(counts, [0, 0, 1024, 1024, 1024, 0, 0, 0]);
        let index = ms.iter().position(|m| m.bucket == 4).unwrap();
        let e = &train[index];
        let m = &ms[index];
        for fault in 0..6 {
            let mut wrong = e.clone();
            match fault {
                0 => wrong.request.input.clear(),
                1 => {
                    wrong.request.evidence.items[1].event_id =
                        wrong.request.evidence.items[0].event_id
                }
                2 => wrong.request.evidence.items[1].version_status = "superseded".into(),
                3 => {
                    wrong.request.evidence.items[1].observed_at =
                        wrong.request.evidence.items[0].observed_at
                }
                4 => wrong.request.evidence.items[1].original_excerpt.clear(),
                _ => wrong.request.evidence.items[1].observed_at = None,
            };
            assert!(flip_selection(&wrong, m).is_err(), "fault={fault}");
        }
        println!(
            "SELECTOR_NATIVE_REQUEST_INVOLUTION=3072 INVALID_CASES=6 OPTIMIZER=0 GENERATION=0"
        );
    }
    #[test]
    #[ignore = "explicit preserved parent/output paths; at most 16 SMALL generations, no teacher or training"]
    fn stabilization_fixed_parent_parity() -> Result<()> {
        let parent = PathBuf::from(
            std::env::var("R3_PARITY_PARENT")
                .map_err(|_| bad("explicit parity parent required"))?,
        );
        let output = PathBuf::from(
            std::env::var("R3_PARITY_OUTPUT")
                .map_err(|_| bad("explicit new parity output required"))?,
        );
        std::fs::create_dir(&output)?;
        let before = inventory(&parent)?;
        let p: Plan = read(&parent.join("plan.r3b"))?;
        let dev = verified_corpus(&parent.join("corpus.r3cor"), &p.corpus)?.validation;
        let (_, dm, _) = verified_metadata(&parent, &p)?;
        let (es, _) = subset(&dev, &dm, 2);
        if es.len() != 16 || p.tiny {
            return Err(bad("parity fixed panel"));
        }
        // Freeze metadata-selected IDs before history/audit reads any old output.
        let selection = binary::record!({"ids":es.iter().map(|e|&e.id).collect::<Vec<_>>(),"cases":digest(&es)?,"policy":digest(&p)?,"step":p.config.max_steps,"tokenizer":p.tokenizer,"source":source_digest()?,"binary":file_hash(&std::env::current_exe()?)?,"generation_limit":16,"teacher_limit":0});
        write(&output.join("selection.r3b"), &selection)?;
        let h = history(&parent, &p)?;
        let last = h.last().ok_or_else(|| bad("missing parent endpoint"))?;
        if last.step != p.config.max_steps || last.resume {
            return Err(bad("parity requires a closed final parent"));
        }
        let checkpoint = parent.join(&last.checkpoint);
        let l = checkpoint::load(&checkpoint, Device::Cpu, false)?;
        if l.tokenizer.id() != p.tokenizer {
            return Err(bad("parity tokenizer"));
        }
        let old_panel = audit_panel(&parent, &p, last.step, "dev512", &dev, &dm, &l.tokenizer)?;
        if old_panel.model != l.model.weight_hash()? {
            return Err(bad("parity endpoint model"));
        }
        let original = binary::read_value_records(
            &parent.join(format!("eval-{:04}-dev512.r3rows", last.step)),
        )?;
        let mut raw = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(output.join("parity.r3rows"))?;
        append_row(
            &mut raw,
            &binary::record!({"selection":file_hash(&output.join("selection.r3b"))?,"checkpoint":checkpoint,"physical":file_hash(&checkpoint)?,"model":l.model.weight_hash()?}),
        )?;
        let mut control = recovery::RunControl::command(false)?;
        control.set_call_limits(16, 0);
        let mut matched = 0;
        let mut tokens = 0;
        let result = (|| -> Result<()> {
            for e in &es {
                let row = match recovery::observe_generation(&l, e, &e.request, &mut control, false)
                {
                    recovery::ObservedCall::Returned(row) => row,
                    recovery::ObservedCall::NotInvoked(_) => {
                        control.stop_result()?;
                        return Err(bad("parity call not invoked"));
                    }
                };
                append_row(&mut raw, &row)?;
                control.stop_result()?;
                verify_generated(&row, &l.tokenizer)?;
                let old = original[1..]
                    .iter()
                    .find(|r| r["id"] == e.id)
                    .ok_or_else(|| bad("parity ID absent"))?;
                for field in [
                    "raw_tokens",
                    "actual",
                    "error",
                    "finish_reason",
                    "eos_index",
                    "native_prompt_digest",
                ] {
                    if old[field] != row[field] {
                        return Err(bad("normal output parity changed"));
                    }
                }
                tokens += row["raw_tokens"]
                    .as_array()
                    .ok_or_else(|| bad("parity tokens"))?
                    .len();
                matched += 1;
            }
            Ok(())
        })();
        if let Err(error) = &result {
            control.classify_error(error);
        }
        let _ = control.seal_terminal();
        let same = before == inventory(&parent)?;
        publish_confirmed(
            &output.join("result.r3b"),
            &binary::record!({"selection":file_hash(&output.join("selection.r3b"))?,"raw":file_hash(&output.join("parity.r3rows"))?,"control":control.receipt(),"matched":matched,"tokens":tokens,"originals_unchanged":same,"error":result.as_ref().err().map(ToString::to_string),"quality_improvement":"NOT_CLAIMED"}),
        )?;
        result?;
        if !same {
            return Err(bad("parity parent changed"));
        }
        println!(
            "PARENT_PARITY matched={matched}/16 raw_tokens={tokens} SMALL_OPTIMIZER=0 SMALL_TEACHER=0 ORIGINALS_UNCHANGED={same}"
        );
        Ok(())
    }
    #[test]
    fn fresh_strict_rows_roundtrip_command_stop_and_errors() {
        let (episodes, meta) = generate(1, 0, 20260919).unwrap();
        let e = &episodes[0];
        let m = &meta[0];
        let tok = ByteBpe::train(
            &[e.answer.as_bytes().to_vec()],
            &neural::hash(b"strict test"),
            264,
        )
        .unwrap();
        let d = tempfile::tempdir().unwrap();
        for case in 0..6 {
            let text = if case == 1 { "wrong" } else { &e.answer };
            let mut ids = if case == 3 {
                tok.encode(&[255]).unwrap()
            } else {
                tok.encode(text.as_bytes()).unwrap()
            };
            let eos = case != 2 && case != 4;
            let body = ids.clone();
            if eos {
                ids.push(EOS);
            }
            let complete = case != 4;
            let finish = if eos {
                "stop"
            } else if complete {
                "length"
            } else {
                "timeout"
            };
            let error = match case {
                3 => Some("invalid/incomplete output UTF-8"),
                4 => Some("native generation timeout"),
                _ => None,
            };
            let expected_exact = case == 0 || case == 5;
            let row = binary::record!({"row_version":2,"id":e.id,"question":e.request.input,"generated_evidence":e.request.evidence,"expected":e.answer,
                "generation_completed":complete,"generation":if complete{binary::record!({"tokens":body,"finish":finish,"generated":ids.len()})}else{binary::Value::Null},
                "raw_tokens":ids,"actual":if error.is_none(){Some(text)}else{None},"error":error,"error_class":if case==3{Some("strict_utf8")}else{None},
                "finish_reason":finish,"exact_match":expected_exact,"command_stop":if case==5{Some("TIME_BUDGET")}else{None}});
            let path = d.path().join(format!("case-{case}.r3rows"));
            let mut f = std::fs::File::create(&path).unwrap();
            append_row(&mut f, &row).unwrap();
            let loaded = binary::read_value_records(&path).unwrap();
            verify_generated(&loaded[0], &tok).unwrap();
            let scored = score(&loaded, std::slice::from_ref(e), std::slice::from_ref(m)).unwrap();
            assert_eq!(scored.exact, usize::from(expected_exact));
            let mut false_row = loaded[0].clone();
            false_row["exact_match"] = binary::record!(!expected_exact);
            assert!(verify_generated(&false_row, &tok).is_err());
        }
        println!("STRICT_ROW_CASES=6 OPTIMIZER=0 GENERATION=0 independent_oracle=true");
    }
    #[test]
    fn fresh_accumulation_and_metadata_boundary() {
        let tok =
            ByteBpe::train(&[b"abc123".to_vec()], &neural::hash(b"fresh numeric"), 264).unwrap();
        for digit in b'0'..=b'9' {
            assert_eq!(tok.encode(&[digit, digit]).unwrap().len(), 2);
        }
        let model = Transformer::init(Config::tiny(264), 17, Device::Cpu).unwrap();
        let inputs = vec![
            Sample {
                tokens: vec![BOS, 8, 9, 10, EOS],
                response_start: 3,
                curriculum: false,
            },
            Sample {
                tokens: vec![BOS, 11, EOS],
                response_start: 1,
                curriculum: false,
            },
        ];
        let grad = |ids: &[usize]| {
            let b = batch(&inputs, ids, &Device::Cpu).unwrap();
            let (_, loss, n) =
                response_loss(&model.forward(&b.input, Some(&b.valid)).unwrap(), &b, 1.).unwrap();
            let g = loss.backward().unwrap();
            (
                model
                    .vars
                    .iter()
                    .map(|(k, v)| (k.clone(), g.get(v).unwrap().detach()))
                    .collect::<BTreeMap<_, _>>(),
                n,
            )
        };
        let (full, n) = grad(&[0, 1]);
        let (a, na) = grad(&[0]);
        let (b, nb) = grad(&[1]);
        assert_eq!(n, na + nb);
        for (k, g) in &full {
            let micro = ((&a[k] * na as f64).unwrap() + (&b[k] * nb as f64).unwrap()).unwrap();
            let error = (g - (micro / n as f64).unwrap())
                .unwrap()
                .abs()
                .unwrap()
                .max_all()
                .unwrap()
                .to_scalar::<f32>()
                .unwrap();
            assert!(error < 3e-5, "{k} {error}");
        }
        let d = tempfile::tempdir().unwrap();
        let mut m = checkpoint::initialized(
            &model,
            &tok,
            17,
            neural::hash(b"synthetic-count-not-executed"),
        )
        .unwrap();
        let mut state = TrainingState {
            resume_binding: None,
            contrast16: false,
            parent_checkpoint_hash: None,
            config: TrainConfig {
                max_steps: 8192,
                budget_start_step: 6144,
                warmup: 0,
                seq_len: 64,
                seed: 29,
                ..Default::default()
            },
            step: 8192,
            consumed_tokens: 20_000_000,
            target_tokens: 1_000_000,
            sampler_state: 8192,
            corpus_hash: tok.train_hash.clone(),
            validation_hash: neural::hash(b"synthetic-dev"),
            previous_corpora: vec![],
            initial_weight_hash: model.weight_hash().unwrap(),
            train_loss: Some(0.12345678901234567),
            validation_loss: None,
        };
        state.resume_binding = Some(checkpoint::ResumeBinding::default_for(&state, &tok));
        m.training = Some(state.clone());
        m.status = "BUDGET_REACHED".into();
        let adam = Adam::new(&model.vars).unwrap();
        checkpoint::save(
            &d.path().join("synthetic6144.r3m"),
            &model,
            &tok,
            m,
            &adam.moments,
        )
        .unwrap();
        let loaded =
            checkpoint::load(&d.path().join("synthetic6144.r3m"), Device::Cpu, true).unwrap();
        assert_eq!(loaded.manifest.training.unwrap(), state);
        let path = d.path().join("trace.r3rows");
        let expected=(6145..=8192).map(|step|binary::record!({"step":step,"sampler":step,"lr_bits":3e-5f64.to_bits(),"draw":[0,1,2,3,4,5,6,7],"input":1234,"target":128})).collect::<Vec<_>>();
        replica_v3::codec::publish_new(&path, |f, _| {
            for row in &expected {
                binary::write_value_record(f, row)?;
            }
            Ok(())
        })
        .unwrap();
        assert_eq!(binary::read_value_records(&path).unwrap(), expected);
        println!("SYNTHETIC_METADATA_STEP=8192 TRACE_ROWS=2048 OPTIMIZER_CALLS=0 backward_calls=3");
    }
    #[test]
    fn fresh_data_disjoint_resolver_and_balanced_epoch() {
        let (train, tm) = generate(256, 0, 20260919).unwrap();
        let (dev, dm) = generate(16, 1, 20260920).unwrap();
        let (transfer, xm) = generate(4, 2, 20260921).unwrap();
        assert_eq!((train.len(), dev.len(), transfer.len()), (8192, 512, 128));
        data::check_split(&train, &dev).unwrap();
        data::check_split(&train, &transfer).unwrap();
        let entities = |m: &[Meta]| {
            m.iter()
                .flat_map(|m| m.entities.iter().cloned())
                .collect::<BTreeSet<_>>()
        };
        assert!(entities(&tm).is_disjoint(&entities(&dm)));
        assert!(entities(&tm).is_disjoint(&entities(&xm)));
        assert!(entities(&dm).is_disjoint(&entities(&xm)));
        let mut prompts = HashMap::new();
        for e in train.iter().chain(&dev).chain(&transfer) {
            assert_eq!(resolve(&e.request).unwrap(), e.answer);
            let key = digest(&(
                e.request.system.clone(),
                e.request.input.clone(),
                e.request.evidence.clone(),
            ))
            .unwrap();
            assert!(
                prompts
                    .insert(key, e.answer.clone())
                    .is_none_or(|old| old == e.answer)
            );
            let mut reversed = e.request.clone();
            reversed.evidence.items.reverse();
            assert_eq!(resolve(&reversed).unwrap(), e.answer);
        }
        let mut wrong = train[0].request.clone();
        wrong.input.push_str("unsupported");
        assert!(resolve(&wrong).is_err());
        wrong = train[0].request.clone();
        wrong.evidence.items[0].original_excerpt.push('!');
        assert!(resolve(&wrong).is_err());
        let p = Plan {
            framing: None,
            identifiable: None,
            schema: None,
            fork: None,
            paired: None,
            training_values: None,
            grounding: None,
            revision: REVISION.into(),
            source: String::new(),
            binary: String::new(),
            corpus: String::new(),
            transfer: String::new(),
            tokenizer: String::new(),
            initial: String::new(),
            initial_weights: String::new(),
            config: TrainConfig {
                seed: 29,
                ..Default::default()
            },
            architecture: Config::tiny(264),
            sampler: "bucket-base-permutation-v1".into(),
            order: (0..8)
                .map(|b| {
                    tm.iter()
                        .enumerate()
                        .filter_map(|(i, m)| (m.bucket == b).then_some(i))
                        .collect()
                })
                .collect(),
            train_order: String::new(),
            split_policy: String::new(),
            data_seed: 20260919,
            model_seed: 17,
            tiny: true,
            metadata: String::new(),
            evaluation: EvaluationPolicy::default(),
        };
        for epoch in 0..4 {
            let mut seen = BTreeSet::new();
            let mut last = [
                String::new(),
                String::new(),
                String::new(),
                String::new(),
                String::new(),
                String::new(),
                String::new(),
                String::new(),
            ];
            for step in epoch * 1024..(epoch + 1) * 1024 {
                let draw = p.draw(step);
                assert_eq!(draw.len(), 8);
                for (b, i) in draw.into_iter().enumerate() {
                    assert_eq!(tm[i].bucket, b);
                    assert!(seen.insert(i));
                    assert_ne!(last[b], tm[i].base);
                    last[b] = tm[i].base.clone();
                }
            }
            assert_eq!(seen.len(), 8192);
        }
        println!("SYNTHETIC_CORPUS_ONLY optimizer=0 examples=8832 epochs_checked=4");
    }
}
