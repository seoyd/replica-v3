//! Parent-bound SMALL6 versus identity-inserted depth8 experiment.
use super::*;
use super::super::{Core, Endpoint, Panel, load_endpoint, save_endpoint, inputs, batch, panel_score, collect};
use neural::{artifact::{ComparisonCore, ComparisonState}, transformer::Config, Backend, Framing, RuntimeProfile};
use recovery::RunControl;
use std::{sync::Arc, time::Duration};

const CONTRACT: &str = "R3-TRPP-DEPTH8-V1";
const ARMS: [&str; 2] = ["D6", "D8"];
const PARENT_GLOBAL: usize = 3584;
const MAX_SECONDS: f64 = 5400.;
const TRAIN_CUTOFF: f64 = 3000.;
const EXECUTION_CONTRACT: &str = "R3-DEPTH8-EXECUTION-RECOVERY-1.0";
const FROZEN_PLAN_HASH: &str = "02d68606a20e4a249245752ce819706479787c7f4658ce142927f6db52c84013";
const FROZEN_SOURCE: &str = "9bea174444057013aed8c976f5c2bf350f680b71f39ab0c141cc6f3f79123f2d";
const FROZEN_BINARY: &str = "7fcc53789aadb3f938250a13a4f67fb271798dff7503c2a7f94bd4f6cfda45f0";
const ACCOUNTING_CONTRACT: &str = "R3-DEPTH8-ACCOUNTING-FIX-1.0";
const ACCOUNTING_SEAL_CONTRACT: &str = "R3-DEPTH8-ACCOUNTING-FIX-SEAL-1.0";
const ACCOUNTING_SUCCESSOR_CONTRACT: &str = "R3-DEPTH8-ACCOUNTING-FIX-SUCCESSOR-1.0";
const FIRST_SEAL_LOG_HASH: &str = "6113e01a05d04c735632fb377b26a35ff90a83fb97f771876a1ca107cca1ef9c";
const FIRST_SEAL_PREPARATION_HASH: &str = "f4e483e20e6f813bd022c626cc8994a685215a840857a730fd4b6c50465f56a2";
const FIRST_SEAL_COMMAND_006_HASH: &str = "7f11ff786f80195644a8b0871a87e8594c023bdd16e34ef558cfb3f0045d8c67";
const FIRST_SEAL_WALL_UPPER: f64 = 26.38;
const SUCCESSOR_OUTER_RESERVE: f64 = 30.;
const LEGACY_TOOL_EXCERPT_HASH: &str = "bce4e45f5e3a73dcc799bf10b13cc8b49340ef099034ae3d6d598f623be51377";
const LEGACY_BOUNDED_SECONDS: f64 = 130.242;
const FROZEN_EXECUTION_PLAN: &str = "64162924b7ed6c2d1534c6ec58631c0c4f771af7ed1670666fbee5d85192908f";
const FROZEN_EXECUTION_BINARY: &str = "f848a7164b8f476b4b040a53fec9833b5b51908f6f8ded8a80246af22a450368";
const JOURNAL_CLOSE_RESERVE: f64 = 10.;
static ACTIVE_COMMAND: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(usize::MAX);
static ACTIVE_PREFLIGHT: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(usize::MAX);

#[derive(Subcommand)]
pub enum Action {
    RepairPrepare { #[arg(long)] root: PathBuf, #[arg(long)] legacy_observation: PathBuf,
        #[arg(long)] legacy_binary: PathBuf },
    SealRepair { #[arg(long)] root: PathBuf },
    SealSuccessor { #[arg(long)] root: PathBuf },
    ReadFinal { #[arg(long)] root: PathBuf },
    ExecutePrepare { #[arg(long)] root: PathBuf, #[arg(long)] target_baseline_kib: u64,
        #[arg(long, default_value_t=0.)] prior_failed_prep_seconds: f64,
        #[arg(long)] prior_failed_binary: Option<String> },
    ExecuteCheck { #[arg(long)] root: PathBuf },
    Prepare { #[arg(long)] closed: PathBuf, #[arg(long)] correction: PathBuf,
        #[arg(long)] output: PathBuf, #[arg(long)] target_baseline_kib: u64 },
    Inspect { #[arg(long)] root: PathBuf },
    Direct { #[arg(long)] root: PathBuf },
    Tiny { #[arg(long)] output: PathBuf, #[arg(long, value_parser=["D6","D8"])] arm:String,
        #[arg(long)] until:usize, #[arg(long)] resume:bool },
    VerifyTiny { #[arg(long)] continuous:PathBuf, #[arg(long)] split:PathBuf },
    Admit { #[arg(long)] root: PathBuf, #[arg(long)] review: PathBuf },
    Train { #[arg(long)] root: PathBuf, #[arg(long, value_parser=["D6", "D8"])] arm: String,
        #[arg(long)] until: usize },
    Evaluate { #[arg(long)] root: PathBuf, #[arg(long, value_parser=["D6", "D8"])] arm: String,
        #[arg(long)] step: usize },
    Screen { #[arg(long)] root: PathBuf },
    Reproduce { #[arg(long)] root: PathBuf, #[arg(long, value_parser=["D6", "D8"])] arm: String },
    AdmitB { #[arg(long)] root: PathBuf, #[arg(long)] review: PathBuf },
    Finish { #[arg(long)] root: PathBuf },
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Plan {
    contract: String,
    root: PathBuf,
    source: String,
    runtime: RuntimeProfile,
    closed_root: PathBuf,
    correction: PathBuf,
    correction_hash: String,
    comparison_root: PathBuf,
    comparison_plan_hash: String,
    parent: Endpoint,
    parent_global: usize,
    tokenizer: PathBuf,
    tokenizer_id: String,
    corpus: String,
    cores: [ComparisonCore; 2],
    config: TrainConfig,
    tape: Vec<[usize; 8]>,
    panels: String,
    input_per_arm: u64,
    target_per_arm: u64,
    target_baseline_kib: u64,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Run {
    policy: String,
    arm: String,
    before: Endpoint,
    after: Endpoint,
    backwards: usize,
    input: u64,
    target: u64,
    seconds: f64,
    stop: String,
    trace: PathBuf,
    trace_hash: String,
    control: binary::Value,
    #[serde(default, skip_serializing_if="Option::is_none")]
    timing: Option<binary::Value>,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct ExecutionAmendment {
    contract: String,
    old_plan: PathBuf,
    old_plan_hash: String,
    new_plan_hash: String,
    source: String,
    binary: String,
    initial: [Endpoint; 2],
    initial_records: [String; 2],
    tokenizer_hash: String,
    cache: PathBuf,
    cache_hash: String,
    tape_hash: String,
    correction_hash: String,
    independent_r_hash: String,
    target_baseline_kib: u64,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct RepairAmendment {
    contract: String,
    predecessor_plan: String,
    predecessor_amendment: String,
    predecessor_command_head: String,
    predecessor_d6_initial: String,
    predecessor_d8_initial: String,
    predecessor_d6_complete: String,
    source: String,
    binary: String,
    runtime: RuntimeProfile,
    legacy_observation: PathBuf,
    legacy_observation_hash: String,
    legacy_binary: PathBuf,
    legacy_binary_hash: String,
    legacy_duration: f64,
    legacy_status: String,
    preparation_limit: u64,
    training_limit: u64,
    evaluation_limit: u64,
    total_limit: u64,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct RepairSeal {
    contract:String,
    predecessor_repair:String,
    predecessor_source:String,
    predecessor_binary:PathBuf,
    predecessor_binary_hash:String,
    predecessor_command_head:String,
    predecessor_command_entered:Vec<String>,
    predecessor_command_returned:Vec<String>,
    repair_attempt_entered:String,
    repair_attempt_returned:String,
    seal_attempt_returned:String,
    legacy_tool_excerpt:PathBuf,
    legacy_tool_excerpt_hash:String,
    bounded_charge_seconds:f64,
    previous_charge_seconds:f64,
    additional_charge_seconds:f64,
    source:String,
    binary:String,
    runtime:RuntimeProfile,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct RepairSuccessor {
    contract:String,
    predecessor_seal:String,
    predecessor_binary:PathBuf,
    predecessor_binary_hash:String,
    preparation_hash:String,
    command_006_entered:String,
    command_006_returned:String,
    first_seal_log:PathBuf,
    first_seal_log_hash:String,
    first_seal_wall_upper:f64,
    first_seal_attempt_seconds:f64,
    additional_first_seal_seconds:f64,
    successor_outer_reserve:f64,
    successor_attempt_returned:String,
    source:String,
    binary:String,
    runtime:RuntimeProfile,
}
#[derive(Clone, Copy)]
struct ExecutionLimits { preparation:f64, training:f64, evaluation:f64, total:f64 }
fn execution_limits(p:&Plan)->ExecutionLimits {
    if p.root.join("repair-amendment.r3b").exists() {
        ExecutionLimits{preparation:1800.,training:2400.,evaluation:2100.,total:6300.}
    } else {ExecutionLimits{preparation:900.,training:2400.,evaluation:2100.,total:5400.}}
}
fn evidence_policy(p:&Plan,arm:&str,step:usize)->Result<String> {
    if execution(p) && arm=="D6" && step==0 {
        let old:Plan=read_confirmed(&p.root.join("plan.r3b"))?;
        policy(&old,arm)
    } else if execution(p) && arm=="D8" && step==0 && p.root.join("repair-seal.r3b").exists() {
        let mut first:Plan=read_confirmed(&p.root.join("plan.r3b"))?;
        let repair=repair_amendment_inner(&first,true)?;
        first.source=repair.source;
        first.runtime=repair.runtime;
        policy(&first,arm)
    } else {policy(p,arm)}
}
fn initial_screen_policy(p:&Plan)->Result<String> {
    if execution(p) && p.root.join("repair-seal.r3b").exists() {
        let mut first:Plan=read_confirmed(&p.root.join("plan.r3b"))?;
        let repair=repair_amendment_inner(&first,true)?;
        first.source=repair.source;
        first.runtime=repair.runtime;
        digest(&first)
    } else {digest(p)}
}
fn command_identity(p:&Plan,index:usize)->Result<(String,String,String)> {
    if index<=2 {
        let old:Plan=read_confirmed(&p.root.join("plan.r3b"))?;
        Ok((digest(&old)?,old.source,old.runtime.binary))
    } else if index<=5 && p.root.join("repair-seal.r3b").exists() {
        let mut first:Plan=read_confirmed(&p.root.join("plan.r3b"))?;
        let repair=repair_amendment_inner(&first,true)?;
        first.source=repair.source;
        first.runtime=repair.runtime;
        Ok((digest(&first)?,first.source,first.runtime.binary))
    } else if index==6 && p.root.join("repair-successor.r3b").exists() {
        let mut second:Plan=read_confirmed(&p.root.join("plan.r3b"))?;
        let seal=repair_seal_inner(&second,true)?;
        second.source=seal.source;
        second.runtime=seal.runtime;
        Ok((digest(&second)?,second.source,second.runtime.binary))
    } else {Ok((digest(p)?,p.source.clone(),p.runtime.binary.clone()))}
}

fn amendment_path(p: &Plan) -> PathBuf {
    p.root.parent().unwrap_or(&p.root).join("execution-amendment.r3b")
}
fn execution(p: &Plan) -> bool {
    p.root.file_name().is_some_and(|name| name == "execution")
}
fn amendment(p: &Plan) -> Result<ExecutionAmendment> {
    let original:Plan=read_confirmed(&p.root.join("plan.r3b"))?;
    let a: ExecutionAmendment = read_confirmed(&amendment_path(p))?;
    let old_root = p.root.parent().ok_or_else(|| bad("depth execution parent"))?;
    let old: Plan = read_confirmed(&a.old_plan)?;
    if a.contract != EXECUTION_CONTRACT || a.old_plan != old_root.join("plan.r3b")
        || a.old_plan_hash != FROZEN_PLAN_HASH || file_hash(&a.old_plan)? != a.old_plan_hash
        || old.root != old_root || old.source != FROZEN_SOURCE || old.runtime.binary != FROZEN_BINARY
        || a.new_plan_hash != file_hash(&p.root.join("plan.r3b"))?
        || a.source != original.source || a.binary != original.runtime.binary
        || a.target_baseline_kib != original.target_baseline_kib
        || a.tape_hash != digest(&original.tape)? || a.correction_hash != file_hash(&original.correction)?
        || a.tokenizer_hash != file_hash(&original.tokenizer)?
        || a.cache_hash != file_hash(&a.cache)?
        || old.tape != original.tape || old.parent != original.parent || old.cores != original.cores
        || old.config != original.config || old.panels != original.panels
        || old.tokenizer_id != original.tokenizer_id || old.corpus != original.corpus
        || old.runtime.backend != original.runtime.backend || old.runtime.actual_device != original.runtime.actual_device
        || old.runtime.dtype != original.runtime.dtype || old.runtime.accumulator != original.runtime.accumulator
        || old.runtime.lock != original.runtime.lock || old.runtime.patch != original.runtime.patch
        || old.runtime.patch_source != original.runtime.patch_source
        || old.runtime.os_build != original.runtime.os_build
        || old.runtime.fast_math != original.runtime.fast_math {
        return Err(bad("depth execution amendment identity"));
    }
    let correction: binary::Value = read_confirmed(&original.correction)?;
    let reader = Path::new(correction["independent_R_path"].as_str()
        .ok_or_else(|| bad("depth independent R path"))?);
    if correction["independent_R"] != a.independent_r_hash || file_hash(reader)? != a.independent_r_hash {
        return Err(bad("depth independent R changed"));
    }
    for (i, arm) in ARMS.iter().enumerate() {
        let record = old.root.join(arm).join("initial.r3b");
        let original: Endpoint = read_confirmed(&record)?;
        let linked: Endpoint = read_confirmed(&p.root.join(arm).join("initial.r3b"))?;
        if a.initial_records[i] != file_hash(&record)? || original != a.initial[i]
            || linked != original || original.state != state(&old, arm)?
            || file_hash(&original.path)? != original.hash {
            return Err(bad("depth inherited native identity"));
        }
    }
    Ok(a)
}
fn repair_amendment_inner(p:&Plan,allow_historical:bool)->Result<RepairAmendment> {
    let a:RepairAmendment=read_confirmed(&p.root.join("repair-amendment.r3b"))?;
    let original:Plan=read_confirmed(&p.root.join("plan.r3b"))?;
    let legacy:binary::Value=read_confirmed(&p.root.join("preparation.r3b"))?;
    let old_runtime=&original.runtime;
    let new_runtime=&a.runtime;
    if a.contract!=ACCOUNTING_CONTRACT
        || a.predecessor_plan!=FROZEN_EXECUTION_PLAN
        || file_hash(&p.root.join("plan.r3b"))?!=a.predecessor_plan
        || original.runtime.binary!=FROZEN_EXECUTION_BINARY
        || a.predecessor_amendment!=file_hash(&amendment_path(&original))?
        || a.predecessor_command_head!=file_hash(&p.root.join("commands/command-002-returned.r3b"))?
        || a.predecessor_d6_initial!=file_hash(&p.root.join("D6/initial.r3b"))?
        || a.predecessor_d8_initial!=file_hash(&p.root.join("D8/initial.r3b"))?
        || a.predecessor_d6_complete!=file_hash(&p.root.join("D6/eval-0-complete.r3b"))?
        || (!allow_historical && (a.source!=source()? || a.binary!=file_hash(&std::env::current_exe()?)?))
        || new_runtime.binary!=a.binary || old_runtime.backend!=new_runtime.backend
        || old_runtime.actual_device!=new_runtime.actual_device
        || old_runtime.dtype!=new_runtime.dtype || old_runtime.accumulator!=new_runtime.accumulator
        || old_runtime.patch!=new_runtime.patch || old_runtime.patch_source!=new_runtime.patch_source
        || old_runtime.lock!=new_runtime.lock || old_runtime.os_build!=new_runtime.os_build
        || old_runtime.fast_math!=new_runtime.fast_math
        || a.legacy_status!="POSTHOC_TRANSCRIPTION_PENDING_A"
        || a.legacy_observation_hash!=file_hash(&a.legacy_observation)?
        || a.legacy_binary_hash!=file_hash(&a.legacy_binary)?
        || legacy["prior_failed_binary"]!=a.legacy_binary_hash
        || !a.legacy_duration.is_finite() || a.legacy_duration<=0.
        || legacy["prior_failed_prep_seconds"]!=a.legacy_duration
        || a.preparation_limit!=1800 || a.training_limit!=2400
        || a.evaluation_limit!=2100 || a.total_limit!=6300 {
        return Err(bad("depth accounting repair identity"));
    }
    Ok(a)
}
fn repair_amendment(p:&Plan)->Result<RepairAmendment> {
    repair_amendment_inner(p,p.root.join("repair-seal.r3b").exists())
}
fn legacy_command_hashes(root:&Path)->Result<(Vec<String>,Vec<String>)> {
    let mut entered=Vec::with_capacity(6);
    let mut returned=Vec::with_capacity(6);
    for index in 0..6 {
        entered.push(file_hash(&root.join("commands").join(format!("command-{index:03}-entered.r3b")))?);
        returned.push(file_hash(&root.join("commands").join(format!("command-{index:03}-returned.r3b")))?);
    }
    Ok((entered,returned))
}
fn verify_repair_attempts(root:&Path,first_source:&str,first_binary:&str,
    seal_source:&str,seal_binary:&str,first_entered:&str,first_returned:&str,
    seal_returned:&str)->Result<()> {
    preflight_usage(root)?;
    let entered=root.join("attempts/attempt-000-entered.r3b");
    let returned=root.join("attempts/attempt-000-returned.r3b");
    let seal=root.join("attempts/attempt-001-returned.r3b");
    let first:binary::Value=read_confirmed(&entered)?;
    let done:binary::Value=read_confirmed(&returned)?;
    let seal_done:binary::Value=read_confirmed(&seal)?;
    if file_hash(&entered)?!=first_entered || file_hash(&returned)?!=first_returned
        || file_hash(&seal)?!=seal_returned
        || first["kind"]!="repair-prepare" || done["kind"]!="repair-prepare"
        || first["source"]!=first_source || first["binary"]!=first_binary
        || done["source"]!=first_source || done["binary"]!=first_binary
        || done["result"]!="COMPLETED" || seal_done["result"]!="COMPLETED"
        || seal_done["kind"]!="seal-repair" || seal_done["source"]!=seal_source
        || seal_done["binary"]!=seal_binary {
        return Err(bad("depth repair attempt identity"));
    }
    Ok(())
}
fn repair_seal_inner(p:&Plan,allow_historical:bool)->Result<RepairSeal> {
    let s:RepairSeal=read_confirmed(&p.root.join("repair-seal.r3b"))?;
    let a=repair_amendment_inner(p,true)?;
    let excerpt=p.root.join("provenance/legacy-tool-records.txt");
    let previous=p.root.join("provenance/first-repair-replica-train");
    let entered=p.root.join("attempts/attempt-000-entered.r3b");
    let returned=p.root.join("attempts/attempt-000-returned.r3b");
    let seal_returned=p.root.join("attempts/attempt-001-returned.r3b");
    let (command_entered,command_returned)=legacy_command_hashes(&p.root)?;
    if s.contract!=ACCOUNTING_SEAL_CONTRACT
        || s.predecessor_repair!=file_hash(&p.root.join("repair-amendment.r3b"))?
        || s.predecessor_source!=a.source || s.predecessor_binary!=previous
        || s.predecessor_binary_hash!=a.binary || file_hash(&previous)?!=a.binary
        || s.predecessor_command_head!=file_hash(&p.root.join("commands/command-005-returned.r3b"))?
        || s.predecessor_command_entered!=command_entered
        || s.predecessor_command_returned!=command_returned
        || s.repair_attempt_entered!=file_hash(&entered)?
        || s.repair_attempt_returned!=file_hash(&returned)?
        || s.seal_attempt_returned!=file_hash(&seal_returned)?
        || s.legacy_tool_excerpt!=excerpt || s.legacy_tool_excerpt_hash!=LEGACY_TOOL_EXCERPT_HASH
        || file_hash(&excerpt)?!=s.legacy_tool_excerpt_hash
        || s.previous_charge_seconds!=a.legacy_duration
        || s.bounded_charge_seconds!=LEGACY_BOUNDED_SECONDS
        || s.additional_charge_seconds!=s.bounded_charge_seconds-s.previous_charge_seconds
        || s.additional_charge_seconds<=0.
        || (!allow_historical && (s.source!=source()? || s.binary!=file_hash(&std::env::current_exe()?)?))
        || s.runtime.binary!=s.binary || s.runtime.backend!=a.runtime.backend
        || s.runtime.actual_device!=a.runtime.actual_device || s.runtime.dtype!=a.runtime.dtype
        || s.runtime.accumulator!=a.runtime.accumulator || s.runtime.lock!=a.runtime.lock
        || s.runtime.patch!=a.runtime.patch || s.runtime.patch_source!=a.runtime.patch_source
        || s.runtime.os_build!=a.runtime.os_build || s.runtime.fast_math!=a.runtime.fast_math {
        return Err(bad("depth accounting seal identity"));
    }
    verify_repair_attempts(&p.root,&a.source,&a.binary,&s.source,&s.binary,
        &s.repair_attempt_entered,&s.repair_attempt_returned,&s.seal_attempt_returned)?;
    Ok(s)
}
fn repair_seal(p:&Plan)->Result<RepairSeal> {repair_seal_inner(p,false)}
fn observed_process_wall(path:&Path)->Result<f64> {
    let raw=neural::read_bounded(path,64*1024)?;
    let text=String::from_utf8(raw).map_err(|_|bad("depth process wall UTF8"))?;
    let observed=text.lines().filter_map(|line|line.strip_prefix("real ")).collect::<Vec<_>>();
    if observed.len()!=1 {return Err(bad("depth process wall count"));}
    let seconds=observed[0].parse::<f64>().map_err(|_|bad("depth process wall number"))?;
    if !seconds.is_finite()||seconds<=0. {return Err(bad("depth process wall duration"));}
    Ok(seconds)
}
fn expected_file_hash(path:&Path,expected:&str)->Result<()> {
    if file_hash(path)?!=expected {return Err(bad("depth bound file changed"));}
    Ok(())
}
fn outer_reserve_covers(wall:f64,inner:f64,reserve:f64)->bool {
    wall.is_finite() && inner.is_finite() && reserve.is_finite()
        && wall>0. && inner>=0. && reserve>=0.
        && wall+0.01>=inner && wall<=inner+reserve
}
fn repair_successor(p:&Plan)->Result<RepairSuccessor> {
    let s:RepairSuccessor=read_confirmed(&p.root.join("repair-successor.r3b"))?;
    let old=repair_seal_inner(p,true)?;
    let prior_binary=p.root.join("provenance/second-repair-replica-train");
    let log=p.root.join("provenance/seal-repair.log");
    let entered=p.root.join("commands/command-006-entered.r3b");
    let returned=p.root.join("commands/command-006-returned.r3b");
    let attempt=p.root.join("attempts/attempt-002-returned.r3b");
    let old_attempt:binary::Value=read_confirmed(&p.root.join("attempts/attempt-001-returned.r3b"))?;
    let new_attempt:binary::Value=read_confirmed(&attempt)?;
    let inner=old_attempt["seconds"].as_f64().ok_or_else(||bad("depth first seal duration"))?;
    let extra=FIRST_SEAL_WALL_UPPER-inner-JOURNAL_CLOSE_RESERVE;
    expected_file_hash(&p.root.join("preparation.r3b"),FIRST_SEAL_PREPARATION_HASH)?;
    if s.contract!=ACCOUNTING_SUCCESSOR_CONTRACT
        || s.predecessor_seal!=file_hash(&p.root.join("repair-seal.r3b"))?
        || s.predecessor_binary!=prior_binary || s.predecessor_binary_hash!=old.binary
        || file_hash(&prior_binary)?!=old.binary
        || s.preparation_hash!=FIRST_SEAL_PREPARATION_HASH
        || s.command_006_entered!=file_hash(&entered)?
        || s.command_006_returned!=FIRST_SEAL_COMMAND_006_HASH
        || file_hash(&returned)?!=s.command_006_returned
        || s.first_seal_log!=log || s.first_seal_log_hash!=FIRST_SEAL_LOG_HASH
        || file_hash(&log)?!=s.first_seal_log_hash
        || observed_process_wall(&log)?!=26.37
        || s.first_seal_wall_upper!=FIRST_SEAL_WALL_UPPER
        || s.first_seal_attempt_seconds!=inner
        || !extra.is_finite() || extra<=0. || s.additional_first_seal_seconds!=extra
        || s.successor_outer_reserve!=SUCCESSOR_OUTER_RESERVE
        || s.successor_attempt_returned!=file_hash(&attempt)?
        || new_attempt["kind"]!="seal-successor" || new_attempt["result"]!="COMPLETED"
        || new_attempt["source"]!=s.source || new_attempt["binary"]!=s.binary
        || s.source!=source()? || s.binary!=file_hash(&std::env::current_exe()?)?
        || s.runtime.binary!=s.binary || s.runtime.backend!=old.runtime.backend
        || s.runtime.actual_device!=old.runtime.actual_device || s.runtime.dtype!=old.runtime.dtype
        || s.runtime.accumulator!=old.runtime.accumulator || s.runtime.lock!=old.runtime.lock
        || s.runtime.patch!=old.runtime.patch || s.runtime.patch_source!=old.runtime.patch_source
        || s.runtime.os_build!=old.runtime.os_build || s.runtime.fast_math!=old.runtime.fast_math {
        return Err(bad("depth repair successor identity/cost"));
    }
    preflight_usage(&p.root)?;
    Ok(s)
}
fn successor_wall_verified(p:&Plan,r:&binary::Value)->Result<bool> {
    if !p.root.join("repair-successor.r3b").exists() {return Ok(true);}
    let s=repair_successor(p)?;
    let path=p.root.join("provenance/successor-seal.log");
    let attempt:binary::Value=read_confirmed(&p.root.join("attempts/attempt-002-returned.r3b"))?;
    let inner=attempt["seconds"].as_f64().ok_or_else(||bad("depth successor inner duration"))?;
    let wall=observed_process_wall(&path)?;
    Ok(r["successor_seal"]==file_hash(&p.root.join("repair-successor.r3b"))?
        && r["successor_wall_log"]==binary::record!(path)
        && r["successor_wall_log_hash"]==file_hash(&path)?
        && r["successor_wall_seconds"]==wall
        && outer_reserve_covers(wall,inner,s.successor_outer_reserve))
}

fn source() -> Result<String> {
    digest(&(source_digest()?,neural::hash(include_bytes!("core_depth8.rs")),
        neural::hash(include_bytes!("core_comparison.rs")),
        neural::hash(include_bytes!("core_binding_recovery.rs"))))
}
fn arm_index(arm: &str) -> Result<usize> {
    ARMS.iter().position(|a| *a == arm).ok_or_else(|| bad("depth arm"))
}
fn policy(p: &Plan, arm: &str) -> Result<String> {
    arm_index(arm)?;
    digest(&(CONTRACT, &p.source, &p.parent.hash, &p.correction_hash, &p.tape,
        &p.cores, &p.config, &p.runtime, arm))
}
fn train_config() -> TrainConfig {
    TrainConfig { lr: 3e-4, beta1: 0.9, beta2: 0.999, eps: 1e-8,
        weight_decay: 0., clip: 1., warmup: 20, max_steps: 512,
        max_tokens: 20_000_000, microbatch: 8, sample_group_size: 1,
        accumulation: 1, seq_len: 256, validate_every: 128, seed: 20260926,
        first_target_weight: 1., curriculum_steps: 0, budget_start_step: 0,
        budget_start_tokens: 0 }
}
fn state(p: &Plan, arm: &str) -> Result<ComparisonState> {
    Ok(ComparisonState { schema: 1, core: p.cores[arm_index(arm)?].clone(),
        policy: policy(p, arm)?, tokenizer: p.tokenizer_id.clone(),
        framing: Framing::QuestionEvidence.digest(), runtime: p.runtime.clone(),
        objective: checkpoint::ANSWER_MEAN_FAMILY,
        optimizer: "FRESH_ADAMW_ALL_V1".into(), config: p.config.clone(),
        committed: 0, adam_clock: 0, input_tokens: 0, target_tokens: 0 })
}
fn parent(p: &Plan) -> Result<(super::Study, super::super::Study)> {
    let (s, ep) = completion::repaired_c_parent(&p.closed_root, &p.correction)?;
    let comparison: super::super::Study = read_confirmed(&s.parent_root.join("plan.r3b"))?;
    if ep != p.parent || p.parent_global != PARENT_GLOBAL
        || p.correction_hash != file_hash(&p.correction)?
        || p.comparison_root != s.parent_root
        || p.comparison_plan_hash != file_hash(&s.parent_root.join("plan.r3b"))?
        || p.tokenizer != comparison.tokenizer || p.tokenizer_id != comparison.tokenizer_id
        || p.corpus != file_hash(&comparison.word_root.join("corpus.r3cor"))?
        || s.tape != comparison.tape[..512]
        || p.tape != comparison.tape[512..1024] {
        return Err(bad("depth frozen parent/tape identity"));
    }
    Ok((s, comparison))
}
fn checked(root: &Path) -> Result<Plan> {
    let root = root.canonicalize()?;
    let mut p: Plan = read_confirmed(&root.join("plan.r3b"))?;
    if execution(&p) && (p.source!=source()? || p.runtime.binary!=file_hash(&std::env::current_exe()?)?) {
        amendment(&p)?;
        if p.root.join("repair-successor.r3b").exists() {
            let successor=repair_successor(&p)?;
            p.source=successor.source;
            p.runtime=successor.runtime;
        } else if p.root.join("repair-seal.r3b").exists() {
            let seal=repair_seal(&p)?;
            p.source=seal.source;
            p.runtime=seal.runtime;
        } else {
            let repair=repair_amendment(&p)?;
            p.source=repair.source;
            p.runtime=repair.runtime;
        }
    }
    let cores_match = matches!(&p.cores[0], ComparisonCore::Trpp(c) if *c == Config::small(c.vocab))
        && matches!(&p.cores[1], ComparisonCore::Trpp(c) if *c == Config::depth8(c.vocab));
    if p.contract != CONTRACT || p.root != root || p.source != source()?
        || p.runtime.binary != file_hash(&std::env::current_exe()?)?
        || p.config != train_config() || !cores_match {
        return Err(bad("depth plan/source/config"));
    }
    if execution(&p) { amendment(&p)?; } else { parent(&p)?; }
    Ok(p)
}
fn execute_prepare(root: &Path, target_baseline_kib: u64, prior_failed_prep_seconds: f64,
    prior_failed_binary: Option<&str>) -> Result<()> {
    let started=Instant::now();
    if prior_failed_prep_seconds!=0. || prior_failed_binary.is_some() {
        return Err(bad("depth unverified legacy CLI cost cannot authorize execution"));
    }
    if !prior_failed_prep_seconds.is_finite() || prior_failed_prep_seconds<0. || prior_failed_prep_seconds>=900.
        || (prior_failed_prep_seconds>0.)!=prior_failed_binary.is_some() {
        return Err(bad("depth prior failed preparation duration"));
    }
    let old_root = root.canonicalize()?;
    let old_plan = old_root.join("plan.r3b");
    let old: Plan = read_confirmed(&old_plan)?;
    if file_hash(&old_plan)? != FROZEN_PLAN_HASH || old.root != old_root
        || old.source != FROZEN_SOURCE || old.runtime.binary != FROZEN_BINARY
        || old_root.join("execution-amendment.r3b").exists()
        || old_root.join("execution").exists() {
        return Err(bad("depth execution only once from frozen preparation"));
    }
    let direct: binary::Value = read_confirmed(&old_root.join("direct-numeric.r3b"))?;
    if direct["policy"] != digest(&old)? || direct["optimizer"] != 0
        || direct["generation"] != 0 { return Err(bad("depth old numeric evidence")); }
    let (_, comparison) = parent(&old)?;
    let cache = comparison.revision.as_ref().map_or_else(
        || comparison.root.join("samples.r3tok"), |r| r.cache.clone());
    let ((_, _, _, tok, samples),input_profile) = super::super::inputs_profiled(&comparison)?;
    if tok.semantic_id() != old.tokenizer_id || samples.len() != 7680 {
        return Err(bad("depth inherited tokenizer/samples"));
    }
    let mut initial = Vec::new();
    let mut initial_records = Vec::new();
    let mut loaded_cores = Vec::new();
    let device = Backend::Metal0.open()?;
    for arm in ARMS {
        let record = old_root.join(arm).join("initial.r3b");
        let ep: Endpoint = read_confirmed(&record)?;
        if ep.state != state(&old, arm)? || file_hash(&ep.path)? != ep.hash {
            return Err(bad("depth old initial native"));
        }
        let (core, adam) = load_endpoint(&ep, device.clone())?;
        if core.vars().len() * 2 != adam.moments.len() || adam.moments.values().any(|moment| {
            moment.flatten_all().and_then(|t| t.to_vec1::<f32>())
                .map_or(true, |values| values.iter().any(|value| *value != 0.))
        }) { return Err(bad("depth inherited Adam is not zero")); }
        initial_records.push(file_hash(&record)?);
        initial.push(ep);
        loaded_cores.push(core);
    }
    let Core::Tr(d6_model)=&loaded_cores[0] else {return Err(bad("depth D6 core"));};
    let Core::Tr(d8_model)=&loaded_cores[1] else {return Err(bad("depth D8 core"));};
    if d6_model.config!=Config::small(d6_model.config.vocab)
        || d8_model.config!=Config::depth8(d6_model.config.vocab)
        || d6_model.weights_content_id()?!=old.parent.content
        || d6_model.vars.len()!=d6_model.config.shapes().len()
        || d8_model.vars.len()!=d8_model.config.shapes().len() {
        return Err(bad("depth inherited registry/parent"));
    }
    for (old_layer,new_layer) in [(0,0),(1,1),(2,3),(3,4),(4,6),(5,7)] {
        for suffix in ["attn_norm","ffn_norm","q_norm","k_norm","q","k","v","o","gate","up","down"] {
            let old_name=format!("layer.{old_layer}.{suffix}");
            let new_name=format!("layer.{new_layer}.{suffix}");
            if d6_model.vars[&old_name].flatten_all()?.to_vec1::<f32>()?
                !=d8_model.vars[&new_name].flatten_all()?.to_vec1::<f32>()? {
                return Err(bad("depth inherited copied tensor"));
            }
        }
    }
    for name in ["embedding","final_norm"] {
        if d6_model.vars[name].flatten_all()?.to_vec1::<f32>()?
            !=d8_model.vars[name].flatten_all()?.to_vec1::<f32>()? {
            return Err(bad("depth inherited shared tensor"));
        }
    }
    for layer in [2,5] {for suffix in ["o","down"] {
        let name=format!("layer.{layer}.{suffix}");
        if d8_model.vars[&name].flatten_all()?.to_vec1::<f32>()?.iter().any(|v|*v!=0.) {
            return Err(bad("depth inserted residual not zero"));
        }
    }}
    let [d6, d8]: [Endpoint; 2] = initial.try_into().map_err(|_| bad("depth initial count"))?;
    let [d6_record, d8_record]: [String; 2] = initial_records.try_into().map_err(|_| bad("depth record count"))?;
    let correction: binary::Value = read_confirmed(&old.correction)?;
    let reader = Path::new(correction["independent_R_path"].as_str().ok_or_else(||bad("depth independent R path"))?);
    let reader_hash = file_hash(reader)?;
    if correction["independent_R"] != reader_hash {return Err(bad("depth independent R changed"));}
    let output = old_root.join("execution");
    let old_logical=super::super::new_bytes(&old_root)?;
    let old_allocated=completion::allocated(&old_root)?;
    let current_target = (completion::allocated(Path::new("target/debug"))? + 1023) / 1024;
    if current_target > target_baseline_kib.saturating_add(1024*1024) {
        return Err(bad("depth target grew beyond original baseline"));
    }
    let mut p = old.clone();
    p.root = output.clone();
    p.source = source()?;
    p.runtime = RuntimeProfile::capture(Backend::Metal0, &device)?;
    p.target_baseline_kib = target_baseline_kib;
    std::fs::create_dir(&output)?;
    for (arm, ep) in [("D6", &d6), ("D8", &d8)] {
        let dir = output.join(arm);
        std::fs::create_dir(&dir)?;
        publish_confirmed(&dir.join("initial.r3b"), ep)?;
    }
    publish_confirmed(&output.join("plan.r3b"), &p)?;
    let a = ExecutionAmendment {
        contract: EXECUTION_CONTRACT.into(), old_plan, old_plan_hash: FROZEN_PLAN_HASH.into(),
        new_plan_hash: file_hash(&output.join("plan.r3b"))?, source: p.source.clone(),
        binary: p.runtime.binary.clone(), initial: [d6, d8],
        initial_records: [d6_record, d8_record], tokenizer_hash: file_hash(&p.tokenizer)?,
        cache_hash: file_hash(&cache)?, cache,
        tape_hash: digest(&p.tape)?, correction_hash: p.correction_hash.clone(),
        independent_r_hash: reader_hash, target_baseline_kib,
    };
    publish_confirmed(&old_root.join("execution-amendment.r3b"), &a)?;
    publish_confirmed(&output.join("preparation.r3b"), &binary::record!({
        "contract":EXECUTION_CONTRACT,"amendment":file_hash(&old_root.join("execution-amendment.r3b"))?,
        "old_plan":FROZEN_PLAN_HASH,"source":p.source,"binary":p.runtime.binary,
        "initial_reused":2,"new_model_bytes":0,"model_calls":0,"optimizer_calls":0,
        "input_reads":input_profile.reads,"input_hashes":input_profile.hashes,
        "input_decodes":input_profile.decodes,"input_reference_seconds":input_profile.reference_seconds,
        "input_cache_seconds":input_profile.cache_seconds,
        "old_root_logical_bytes":old_logical,"old_root_allocated_bytes":old_allocated,
        "target_baseline_kib":target_baseline_kib,"target_now_kib":current_target,
        "seconds":started.elapsed().as_secs_f64()+prior_failed_prep_seconds,
        "prior_failed_prep_seconds":prior_failed_prep_seconds,
        "prior_failed_binary":prior_failed_binary.unwrap_or("NONE"),
        "prior_failed_reason":if prior_failed_prep_seconds>0.{"Metal feature absent before model call"}else{"NONE"},
        "historical_known_seconds":2346.937414541f64,
        "historical_unknown":"v1 preparation"}))?;
    println!("DEPTH_EXECUTION_PREPARED old_plan={} policy_D6={} policy_D8={} old_initials_reused=2 target_baseline_kib={target_baseline_kib}",
        a.old_plan_hash, policy(&p,"D6")?,policy(&p,"D8")?);
    Ok(())
}
fn preflight_attempt(root:&Path, kind:&str, request:&str, action:impl FnOnce()->Result<()>)->Result<()> {
    let started=Instant::now();
    let root=root.canonicalize()?;
    let lock=std::fs::OpenOptions::new().create(true).read(true).write(true).open(root.join("writer.lock"))?;
    lock.try_lock().map_err(|_|bad("depth preflight writer active"))?;
    let dir=root.join("attempts");std::fs::create_dir_all(&dir)?;
    let mut index=0usize;
    while index<16 && dir.join(format!("attempt-{index:03}-entered.r3b")).exists() {
        if !dir.join(format!("attempt-{index:03}-returned.r3b")).exists() {
            return Err(bad("depth preflight UNKNOWN attempt"));
        }
        index+=1;
    }
    if index==16 {return Err(bad("depth preflight attempt limit"));}
    let entered=dir.join(format!("attempt-{index:03}-entered.r3b"));
    let returned=dir.join(format!("attempt-{index:03}-returned.r3b"));
    let binary=file_hash(&std::env::current_exe()?)?;
    let source=source()?;
    let preceding=if index==0 && root.file_name().is_some_and(|n|n=="execution") {
        FROZEN_EXECUTION_PLAN.to_string()
    } else if index==0 {file_hash(&root.join("plan.r3b"))?}
        else {file_hash(&dir.join(format!("attempt-{:03}-returned.r3b",index-1)))?};
    let feature=format!("metal={};accelerate={}",cfg!(feature="metal"),cfg!(feature="accelerate"));
    let attempt=digest(&(root.as_path(),kind,request,&preceding,&source,&binary,index))?;
    publish_confirmed(&entered,&binary::record!({"contract":ACCOUNTING_CONTRACT,"scope":root,
        "attempt":attempt,"index":index,"kind":kind,"bucket":"preparation",
        "request":request,"predecessor":preceding,"source":source,"binary":binary,"feature":feature}))?;
    ACTIVE_PREFLIGHT.store(index,Ordering::Relaxed);
    let result=action();
    let elapsed=started.elapsed().as_secs_f64();
    let error=result.as_ref().err().map(|e|e.to_string()).unwrap_or_default();
    publish_confirmed(&returned,&binary::record!({"contract":ACCOUNTING_CONTRACT,"scope":root,
        "attempt":attempt,"index":index,"kind":kind,"bucket":"preparation",
        "request":request,"predecessor":preceding,"source":source,"binary":binary,"feature":feature,
        "seconds":elapsed,"result":if result.is_ok(){"COMPLETED"}else{"RETURNED_ERROR"},
        "error_hash":neural::hash(error.as_bytes()),"model_calls":0,"optimizer_calls":0}))?;
    ACTIVE_PREFLIGHT.store(usize::MAX,Ordering::Relaxed);
    result
}
fn preflight_usage(root:&Path)->Result<f64> {
    let root=root.canonicalize()?;
    let dir=root.join("attempts");if !dir.exists(){
        if root.join("repair-amendment.r3b").exists() {return Err(bad("depth repair attempt journal missing"));}
        return Ok(0.);
    }
    let mut total=0.;let mut preceding=if root.file_name().is_some_and(|n|n=="execution") {
        FROZEN_EXECUTION_PLAN.to_string()
    } else {file_hash(&root.join("plan.r3b"))?};
    let mut count=0usize;
    for index in 0..16 {
        let entered=dir.join(format!("attempt-{index:03}-entered.r3b"));
        if !entered.exists(){break;}
        let returned=dir.join(format!("attempt-{index:03}-returned.r3b"));
        if !returned.exists(){
            if ACTIVE_PREFLIGHT.load(Ordering::Relaxed)==index {break;}
            return Err(bad("depth preflight UNKNOWN attempt"));
        }
        let a:binary::Value=read_confirmed(&entered)?;
        let b:binary::Value=read_confirmed(&returned)?;
        let kind=a["kind"].as_str().ok_or_else(||bad("depth preflight kind"))?;
        let request=a["request"].as_str().ok_or_else(||bad("depth preflight request"))?;
        let src=a["source"].as_str().ok_or_else(||bad("depth preflight source"))?;
        let binary=a["binary"].as_str().ok_or_else(||bad("depth preflight binary"))?;
        let expected=digest(&(root.as_path(),kind,request,&preceding,src,binary,index))?;
        if a["scope"]!=binary::record!(root.as_path()) || a["scope"]!=b["scope"]
            || a["predecessor"]!=preceding || a["predecessor"]!=b["predecessor"]
            || a["index"]!=index || a["index"]!=b["index"]
            || a["attempt"]!=expected || b["attempt"]!=expected
            || a["contract"]!=ACCOUNTING_CONTRACT || b["contract"]!=ACCOUNTING_CONTRACT
            || a["kind"]!=b["kind"]
            || a["binary"]!=b["binary"] || a["source"]!=b["source"]
            || a["request"]!=b["request"] || a["feature"]!=b["feature"]
            || a["bucket"]!="preparation" || b["bucket"]!="preparation"
            || b["model_calls"].as_u64()!=Some(0) || b["optimizer_calls"].as_u64()!=Some(0)
            || !["COMPLETED","RETURNED_ERROR"].contains(&b["result"].as_str().unwrap_or("")) {
            return Err(bad("depth preflight attempt mismatch"));
        }
        let seconds=b["seconds"].as_f64().ok_or_else(||bad("depth preflight duration"))?;
        if !seconds.is_finite()||seconds<0. {return Err(bad("depth preflight duration"));}
        total+=seconds;preceding=file_hash(&returned)?;
        count+=1;
    }
    for entry in std::fs::read_dir(&dir)? {
        let name=entry?.file_name().to_string_lossy().to_string();
        if let Some(suffix)=name.strip_prefix("attempt-") {
            let Some((number,kind))=suffix.split_once('-') else {return Err(bad("depth preflight filename"));};
            let at=number.parse::<usize>().map_err(|_|bad("depth preflight filename"))?;
            let active=ACTIVE_PREFLIGHT.load(Ordering::Relaxed)==at && at==count && kind=="entered.r3b";
            if (at>=count && !active) || !["entered.r3b","returned.r3b"].contains(&kind) {
                return Err(bad("depth preflight attempt gap/duplicate"));
            }
        }
    }
    if count==0 && root.join("repair-amendment.r3b").exists()
        && ACTIVE_PREFLIGHT.load(Ordering::Relaxed)!=0 {
        return Err(bad("depth repair attempt missing"));
    }
    Ok(total)
}
fn preflight_head(root:&Path)->Result<String> {
    preflight_usage(root)?;
    let dir=root.join("attempts");
    let mut last=None;
    for index in 0..16 {
        let path=dir.join(format!("attempt-{index:03}-returned.r3b"));
        if !path.exists(){break;}
        last=Some(file_hash(&path)?);
    }
    last.ok_or_else(||bad("depth preflight head absent"))
}
fn parent_preflight_extra(child:&Path,body_seconds:f64)->Result<f64> {
    let parent=child.parent().ok_or_else(||bad("depth execution parent"))?;
    let link:binary::Value=read_confirmed(&child.join("preflight-link.r3b"))?;
    if link["parent"]!=binary::record!(parent)
        || link["scope"]!=binary::record!(child)
        || link["head"]!=preflight_head(parent)? {return Err(bad("depth execution parent preflight link"));}
    let dir=parent.join("attempts");
    let mut last=None;
    for index in 0..16 {
        let path=dir.join(format!("attempt-{index:03}-returned.r3b"));
        if !path.exists(){break;}
        last=Some(read_confirmed::<binary::Value>(&path)?);
    }
    let last=last.ok_or_else(||bad("depth parent preflight success absent"))?;
    let successful=last["seconds"].as_f64().ok_or_else(||bad("depth parent preflight duration"))?;
    if last["result"]!="COMPLETED" || !successful.is_finite() || successful<0.
        || !body_seconds.is_finite() || body_seconds<0. {return Err(bad("depth parent preflight successful attempt"));}
    let total=preflight_usage(parent)?;
    if total<successful || successful<body_seconds {
        return Err(bad("depth parent preflight monotonic duration"));
    }
    Ok(total-body_seconds)
}
fn repair_prepare(root:&Path, legacy_observation:&Path, legacy_binary:&Path)->Result<()> {
    let root=root.canonicalize()?;
    let observation=legacy_observation.canonicalize()?;
    let binary_path=legacy_binary.canonicalize()?;
    let request=digest(&(root.as_path(),observation.as_path(),binary_path.as_path()))?;
    preflight_attempt(&root,"repair-prepare",&request,||{
        if root.join("repair-amendment.r3b").exists() {return Err(bad("depth repair already registered"));}
        let p:Plan=read_confirmed(&root.join("plan.r3b"))?;
        if !execution(&p) || file_hash(&root.join("plan.r3b"))?!=FROZEN_EXECUTION_PLAN
            || p.runtime.binary!=FROZEN_EXECUTION_BINARY {return Err(bad("depth repair predecessor"));}
        amendment(&p)?;
        let (prep,training,evaluation,calls,tokens)=execution_usage(&p)?;
        if training!=0. || evaluation!=0. || calls!=8 || tokens!=128 || prep>=1800. {
            return Err(bad("depth repair predecessor usage"));
        }
        let old:binary::Value=read_confirmed(&root.join("preparation.r3b"))?;
        let duration=old["prior_failed_prep_seconds"].as_f64().ok_or_else(||bad("depth prior duration"))?;
        let binary_hash=file_hash(&binary_path)?;
        if !duration.is_finite() || duration<=0. || old["prior_failed_binary"]!=binary_hash
            || !cfg!(feature="metal") || !cfg!(feature="accelerate") {
            return Err(bad("depth repair legacy/feature identity"));
        }
        let observed=neural::read_bounded(&observation,64*1024)?;
        if !observed.windows(binary_hash.len()).any(|v|v==binary_hash.as_bytes())
            || !observed.windows(b"BEFORE_MODEL_CALL".len()).any(|v|v==b"BEFORE_MODEL_CALL") {
            return Err(bad("depth legacy observation incomplete"));
        }
        let device=Backend::Metal0.open()?;
        let runtime=RuntimeProfile::capture(Backend::Metal0,&device)?;
        let a=RepairAmendment {contract:ACCOUNTING_CONTRACT.into(),
            predecessor_plan:FROZEN_EXECUTION_PLAN.into(),
            predecessor_amendment:file_hash(&amendment_path(&p))?,
            predecessor_command_head:file_hash(&root.join("commands/command-002-returned.r3b"))?,
            predecessor_d6_initial:file_hash(&root.join("D6/initial.r3b"))?,
            predecessor_d8_initial:file_hash(&root.join("D8/initial.r3b"))?,
            predecessor_d6_complete:file_hash(&root.join("D6/eval-0-complete.r3b"))?,
            source:source()?,binary:runtime.binary.clone(),runtime,
            legacy_observation:observation.clone(),legacy_observation_hash:file_hash(&observation)?,
            legacy_binary:binary_path.clone(),legacy_binary_hash:binary_hash,
            legacy_duration:duration,legacy_status:"POSTHOC_TRANSCRIPTION_PENDING_A".into(),
            preparation_limit:1800,training_limit:2400,evaluation_limit:2100,total_limit:6300};
        publish_confirmed(&root.join("repair-amendment.r3b"),&a)
    })
}
fn seal_repair(root:&Path)->Result<()> {
    let root=root.canonicalize()?;
    let excerpt=root.join("provenance/legacy-tool-records.txt");
    let previous=root.join("provenance/first-repair-replica-train");
    let request=digest(&(root.as_path(),file_hash(&excerpt)?,file_hash(&previous)?))?;
    let mut captured=None;
    preflight_attempt(&root,"seal-repair",&request,||{
        if root.join("repair-seal.r3b").exists() {return Err(bad("depth seal already registered"));}
        let p:Plan=read_confirmed(&root.join("plan.r3b"))?;
        let a=repair_amendment_inner(&p,true)?;
        if root.join("commands/command-006-entered.r3b").exists() {
            return Err(bad("depth unsealed command after first repair"));
        }
        if file_hash(&previous)?!=a.binary || file_hash(&excerpt)?!=LEGACY_TOOL_EXCERPT_HASH {
            return Err(bad("depth historical binary/tool output identity"));
        }
        let raw=neural::read_bounded(&excerpt,64*1024)?;
        let lines=String::from_utf8(raw).map_err(|_|bad("depth tool excerpt UTF8"))?;
        let lines=lines.lines().collect::<Vec<_>>();
        if lines.len()!=6 || !lines[0].contains(&a.legacy_binary_hash)
            || !lines[1].contains("2026-09-26T07:08:07.890Z")
            || !lines[1].contains("depth8 execute-prepare")
            || !lines[2].contains("30.002559958") || !lines[3].contains("30.002914792")
            || !lines[4].contains("30.002992417") || !lines[5].contains("27.913886709")
            || !lines[5].contains("2026-09-26T07:10:18.131Z")
            || !lines[5].contains("BEFORE_MODEL_CALL") {
            return Err(bad("depth tool excerpt does not prove bounded charge"));
        }
        let _=preflight_usage(&root)?;
        let device=Backend::Metal0.open()?;
        let runtime=RuntimeProfile::capture(Backend::Metal0,&device)?;
        captured=Some(runtime);
        Ok(())
    })?;
    let p:Plan=read_confirmed(&root.join("plan.r3b"))?;
    let a=repair_amendment_inner(&p,true)?;
    let runtime=captured.ok_or_else(||bad("depth seal runtime missing"))?;
    let (command_entered,command_returned)=legacy_command_hashes(&root)?;
    let s=RepairSeal{contract:ACCOUNTING_SEAL_CONTRACT.into(),
        predecessor_repair:file_hash(&root.join("repair-amendment.r3b"))?,
        predecessor_source:a.source.clone(),predecessor_binary:previous.clone(),
        predecessor_binary_hash:a.binary.clone(),
        predecessor_command_head:file_hash(&root.join("commands/command-005-returned.r3b"))?,
        predecessor_command_entered:command_entered,
        predecessor_command_returned:command_returned,
        repair_attempt_entered:file_hash(&root.join("attempts/attempt-000-entered.r3b"))?,
        repair_attempt_returned:file_hash(&root.join("attempts/attempt-000-returned.r3b"))?,
        seal_attempt_returned:file_hash(&root.join("attempts/attempt-001-returned.r3b"))?,
        legacy_tool_excerpt:excerpt.clone(),legacy_tool_excerpt_hash:LEGACY_TOOL_EXCERPT_HASH.into(),
        bounded_charge_seconds:LEGACY_BOUNDED_SECONDS,previous_charge_seconds:a.legacy_duration,
        additional_charge_seconds:LEGACY_BOUNDED_SECONDS-a.legacy_duration,
        source:source()?,binary:runtime.binary.clone(),runtime};
    publish_confirmed(&root.join("repair-seal.r3b"),&s)?;
    repair_seal(&p)?;
    Ok(())
}
fn seal_successor(root:&Path)->Result<()> {
    let root=root.canonicalize()?;
    if root.join("attempts/attempt-002-entered.r3b").exists() {
        return Err(bad("depth successor one-shot attempt already entered"));
    }
    let log=root.join("provenance/seal-repair.log");
    let previous=root.join("provenance/second-repair-replica-train");
    let request=digest(&(root.as_path(),file_hash(&root.join("repair-seal.r3b"))?,
        file_hash(&root.join("preparation.r3b"))?,file_hash(&log)?,file_hash(&previous)?))?;
    let mut captured=None;
    preflight_attempt(&root,"seal-successor",&request,||{
        if root.join("repair-successor.r3b").exists()
            || root.join("commands/command-007-entered.r3b").exists() {
            return Err(bad("depth successor already registered or later command"));
        }
        let p:Plan=read_confirmed(&root.join("plan.r3b"))?;
        let old=repair_seal_inner(&p,true)?;
        if file_hash(&root.join("preparation.r3b"))?!=FIRST_SEAL_PREPARATION_HASH
            || file_hash(&root.join("commands/command-006-returned.r3b"))?!=FIRST_SEAL_COMMAND_006_HASH
            || file_hash(&previous)?!=old.binary || file_hash(&log)?!=FIRST_SEAL_LOG_HASH
            || observed_process_wall(&log)?!=26.37 {
            return Err(bad("depth successor original cost provenance"));
        }
        let device=Backend::Metal0.open()?;
        captured=Some(RuntimeProfile::capture(Backend::Metal0,&device)?);
        Ok(())
    })?;
    let p:Plan=read_confirmed(&root.join("plan.r3b"))?;
    let old=repair_seal_inner(&p,true)?;
    let runtime=captured.ok_or_else(||bad("depth successor runtime missing"))?;
    let first:binary::Value=read_confirmed(&root.join("attempts/attempt-001-returned.r3b"))?;
    let inner=first["seconds"].as_f64().ok_or_else(||bad("depth first seal duration"))?;
    let s=RepairSuccessor{contract:ACCOUNTING_SUCCESSOR_CONTRACT.into(),
        predecessor_seal:file_hash(&root.join("repair-seal.r3b"))?,
        predecessor_binary:previous.clone(),predecessor_binary_hash:old.binary,
        preparation_hash:file_hash(&root.join("preparation.r3b"))?,
        command_006_entered:file_hash(&root.join("commands/command-006-entered.r3b"))?,
        command_006_returned:file_hash(&root.join("commands/command-006-returned.r3b"))?,
        first_seal_log:log.clone(),first_seal_log_hash:file_hash(&log)?,
        first_seal_wall_upper:FIRST_SEAL_WALL_UPPER,first_seal_attempt_seconds:inner,
        additional_first_seal_seconds:FIRST_SEAL_WALL_UPPER-inner-JOURNAL_CLOSE_RESERVE,
        successor_outer_reserve:SUCCESSOR_OUTER_RESERVE,
        successor_attempt_returned:file_hash(&root.join("attempts/attempt-002-returned.r3b"))?,
        source:source()?,binary:runtime.binary.clone(),runtime};
    publish_confirmed(&root.join("repair-successor.r3b"),&s)?;
    repair_successor(&p)?;
    Ok(())
}
fn execute_check(root:&Path)->Result<()> {
    let started=Instant::now();
    let p=checked(root)?;
    if !execution(&p) {return Err(bad("depth execution check requires amendment"));}
    let study=VerifiedStudy::load(&p)?;
    let (_,comparison)=parent(&p)?;
    let word_plan:super::super::Plan=read_confirmed(&comparison.word_root.join("plan.r3b"))?;
    let corpus=verified_corpus(&comparison.word_root.join("corpus.r3cor"),&word_plan.corpus)?;
    let (train_meta,_,_)=verified_metadata(&comparison.word_root,&word_plan)?;
    let tokenizer=ByteBpe::load(&comparison.tokenizer)?;
    let framed=samples_with_framing(&corpus.train,&tokenizer,256,Framing::QuestionEvidence)?;
    let cache=comparison.revision.as_ref().map_or_else(||comparison.root.join("samples.r3tok"),|r|r.cache.clone());
    let reference=recovery::experiment_record::token_cache::comparison_cache(&cache,
        comparison.cache_key,256,tokenizer.vocab_size(),&framed,false)?;
    if reference.len()!=study.samples.len() || reference.len()!=train_meta.len()
        || reference.iter().zip(&study.samples).any(|(a,b)|
            a.tokens!=b.tokens||a.response_start!=b.response_start||a.curriculum!=b.curriculum) {
        return Err(bad("depth reference/token-cache parity"));
    }
    let device=Backend::Metal0.open()?;p.runtime.verify(&device)?;
    for index in [0,511] {let row=&p.tape[index];
        let a=batch(&reference,row,&device)?;let b=batch(&study.samples,row,&device)?;
        if a.tokens!=b.tokens||a.valid!=b.valid
            || a.input.flatten_all()?.to_vec1::<u32>()?!=b.input.flatten_all()?.to_vec1::<u32>()?
            || a.target.flatten_all()?.to_vec1::<u32>()?!=b.target.flatten_all()?.to_vec1::<u32>()?
            || a.mask.flatten_all()?.to_vec1::<f32>()?!=b.mask.flatten_all()?.to_vec1::<f32>()? {
            return Err(bad("depth command-local batch/mask/target parity"));
        }
    }
    for step in [0,128,512,998] {
        let panels=study.panels(step,true)?;
        let expected=match step {0=>8,128=>16,512=>832,_=>8};
        if panels.iter().map(|v|v.1.len()).sum::<usize>()!=expected {
            return Err(bad("depth execution panel case order/denominator"));
        }
    }
    let initial:Endpoint=read_confirmed(&p.root.join("D6/initial.r3b"))?;
    let mut tampered=initial.clone();tampered.hash="0".repeat(64);
    if load_endpoint(&tampered,device).is_ok() {return Err(bad("depth native tamper accepted"));}
    let report=binary::record!({"contract":EXECUTION_CONTRACT,"policy":digest(&p)?,
        "amendment":file_hash(&amendment_path(&p))?,"source":p.source,"binary":p.runtime.binary,
        "samples_compared":reference.len(),"batches_compared":2,"panels":[8,16,832,8],
        "native_tamper_rejected":true,"input_profile":study.load_profile,
        "generation":0,"teacher":0,"optimizer":0,"backward":0,
        "seconds":started.elapsed().as_secs_f64()});
    publish_confirmed(&p.root.join("execute-check.r3b"),&report)?;
    println!("DEPTH_EXECUTION_CHECK samples={} batches=2 native_tamper=REJECTED calls=0 seconds={}",
        reference.len(),report["seconds"]);
    Ok(())
}
fn admitted(p: &Plan) -> Result<()> {
    let link: binary::Value = read_confirmed(&p.root.join("review-a.r3b"))?;
    let path = Path::new(link["path"].as_str().ok_or_else(||bad("depth A report path"))?);
    let r: binary::Value = read(path)?;
    let repaired=execution(p)&&p.root.join("repair-amendment.r3b").exists();
    let contract=if repaired {ACCOUNTING_CONTRACT}else if execution(p){EXECUTION_CONTRACT}else{CONTRACT};
    if link["hash"] != file_hash(path)? || link["plan"] != file_hash(&p.root.join("plan.r3b"))?
        || r["contract"] != contract || r["plan_hash"] != link["plan"]
        || r["source"] != p.source || r["binary"] != p.runtime.binary
        || r["verdict"] != "PASS" || r["independent_R"] != p.correction_hash
        || (execution(p) && (r["amendment"]!=file_hash(&amendment_path(p))?
            || r["initial16"]!=true || r["changed_boundary"]!=true))
        || !successor_wall_verified(p,&r)?
        || (repaired && (r["repair_amendment"]!=file_hash(&p.root.join("repair-amendment.r3b"))?
            || r["repair_seal"]!=file_hash(&p.root.join("repair-seal.r3b"))?
            || r["legacy_observation_verified"]!=true || r["accounting_regressions"]!=true)) {
        return Err(bad("depth independent R+A admission"));
    }
    Ok(())
}
fn admit(root: &Path, review: &Path) -> Result<()> {
    let p = checked(root)?;
    if execution(&p) {
        let check:binary::Value=read_confirmed(&p.root.join("execute-check.r3b"))?;
        let old:Plan=read_confirmed(&p.root.join("plan.r3b"))?;
        if check["policy"]!=digest(&old)? || check["amendment"]!=file_hash(&amendment_path(&p))?
            || read_confirmed::<binary::Value>(&p.root.join("screen-0.r3b"))?!=initial_screen_value(&p)? {
            return Err(bad("depth execution direct/initial16 before A"));
        }
    }
    let r: binary::Value = read(review)?;
    let repaired=execution(&p)&&p.root.join("repair-amendment.r3b").exists();
    let contract=if repaired {ACCOUNTING_CONTRACT}else if execution(&p){EXECUTION_CONTRACT}else{CONTRACT};
    if r["contract"] != contract || r["plan_hash"] != file_hash(&p.root.join("plan.r3b"))?
        || r["source"] != p.source || r["binary"] != p.runtime.binary
        || r["verdict"] != "PASS" || r["independent_R"] != p.correction_hash
        || (execution(&p) && (r["amendment"]!=file_hash(&amendment_path(&p))?
            || r["initial16"]!=true || r["changed_boundary"]!=true))
        || !successor_wall_verified(&p,&r)?
        || (repaired && (r["repair_amendment"]!=file_hash(&p.root.join("repair-amendment.r3b"))?
            || r["repair_seal"]!=file_hash(&p.root.join("repair-seal.r3b"))?
            || r["legacy_observation_verified"]!=true || r["accounting_regressions"]!=true)) {
        return Err(bad("depth A scope/identity"));
    }
    publish_confirmed(&p.root.join("review-a.r3b"), &binary::record!({
        "path":review.canonicalize()?,"hash":file_hash(review)?,"plan":file_hash(&p.root.join("plan.r3b"))?}))
}
fn history(p: &Plan, arm: &str) -> Result<Vec<Run>> {
    let dir = p.root.join(arm);
    let mut before: Endpoint = read_confirmed(&dir.join("initial.r3b"))?;
    let expected_initial = if execution(p) {
        let a:ExecutionAmendment=read_confirmed(&amendment_path(p))?;
        a.initial[arm_index(arm)?].clone()
    }
        else { state(p, arm).and_then(|s| Ok(Endpoint {state:s,path:before.path.clone(),hash:before.hash.clone(),content:before.content.clone()}))? };
    if before != expected_initial || before.hash != file_hash(&before.path)? {
        return Err(bad("depth initial native"));
    }
    let mut out = vec![];
    for index in 0..16 {
        let entered = dir.join(format!("segment-{index:03}-entered.r3b"));
        let returned = dir.join(format!("segment-{index:03}-returned.r3b"));
        if !returned.exists() {
            if entered.exists() { return Err(bad("depth UNKNOWN training segment")); }
            break;
        }
        let r: Run = read_confirmed(&returned)?;
        let mark: binary::Value = read_confirmed(&entered)?;
        if r.policy != policy(p, arm)? || r.arm != arm || r.before != before
            || r.after.hash != file_hash(&r.after.path)? || r.after.state.core != p.cores[arm_index(arm)?]
            || r.after.state.policy != r.policy || r.after.state.runtime != p.runtime
            || r.after.state.committed != r.after.state.adam_clock
            || r.after.state.committed < before.state.committed
            || r.after.state.committed > 512
            || r.after.state.committed - before.state.committed > r.backwards
            || r.after.state.input_tokens != before.state.input_tokens + r.input
            || r.after.state.target_tokens != before.state.target_tokens + r.target
            || r.trace_hash != file_hash(&r.trace)? || !r.seconds.is_finite() || r.seconds < 0.
            || (execution(p) && r.timing.is_none())
            || !["COMPLETED", "TIME_BUDGET"].contains(&r.stop.as_str())
            || mark["policy"] != r.policy || mark["before"] != binary::record!(before)
            || mark["index"] != index {
            return Err(bad("depth training segment integrity"));
        }
        before = r.after.clone();
        out.push(r);
    }
    if out.iter().map(|r|r.backwards).sum::<usize>() > 528 { return Err(bad("depth backward budget")); }
    Ok(out)
}
fn endpoint(p: &Plan, arm: &str) -> Result<Endpoint> {
    Ok(history(p, arm)?.last().map(|r|r.after.clone()).unwrap_or(read_confirmed(&p.root.join(arm).join("initial.r3b"))?))
}
fn endpoint_at(p:&Plan,arm:&str,step:usize)->Result<Endpoint>{
    let step=if step==998 {512}else{step};
    if step==0 {return read_confirmed(&p.root.join(arm).join("initial.r3b"));}
    history(p,arm)?.into_iter().find(|r|r.after.state.committed==step)
        .map(|r|r.after).ok_or_else(||bad("depth historical endpoint missing"))
}
fn resource_guard(p: &Plan, reserve_bytes: u64) -> Result<(u64,u64)> {
    if execution(p) {
        let mut evidence=super::super::new_bytes(&p.root)?;
        for file in ["review-a.r3b","review-b.r3b"] {
            let link=p.root.join(file);
            if link.exists() {
                let value:binary::Value=read_confirmed(&link)?;
                let path=Path::new(value["path"].as_str().ok_or_else(||bad("depth review path"))?);
                let parent=path.parent().ok_or_else(||bad("depth review parent"))?;
                if !parent.starts_with(&p.root) {
                    evidence=evidence.checked_add(super::super::new_bytes(parent)?)
                        .ok_or_else(||bad("depth review bytes overflow"))?;
                }
            }
        }
        let executable=std::fs::metadata(std::env::current_exe()?)?.len();
        let logical=evidence.checked_add(executable).and_then(|n|n.checked_add(reserve_bytes))
            .ok_or_else(||bad("depth execution bytes overflow"))?;
        let target=(completion::allocated(Path::new("target/debug"))?+1023)/1024;
        if logical>1536*1024*1024 || target>p.target_baseline_kib+1024*1024 {
            return Err(bad("depth execution artifact/target budget"));
        }
        return Ok((logical,target));
    }
    let correction: binary::Value=read_confirmed(&p.correction)?;
    let reader=Path::new(correction["independent_R_path"].as_str()
        .ok_or_else(||bad("depth R resource path"))?);
    let mut roots=std::collections::BTreeSet::new();
    roots.insert(p.root.clone());
    roots.insert(p.correction.parent().ok_or_else(||bad("depth correction parent"))?.to_path_buf());
    roots.insert(reader.parent().ok_or_else(||bad("depth R parent"))?.to_path_buf());
    for file in ["review-a.r3b","review-b.r3b"] {
        let link=p.root.join(file);
        if link.exists() {
            let value:binary::Value=read_confirmed(&link)?;
            let path=Path::new(value["path"].as_str().ok_or_else(||bad("depth review resource path"))?);
            roots.insert(path.parent().ok_or_else(||bad("depth review parent"))?.to_path_buf());
        }
    }
    let sibling=p.root.parent().ok_or_else(||bad("depth artifact parent"))?;
    for name in ["depth8-comparison-20260926-v1","depth8-evidence-repair-20260926-v1",
        "depth8-tiny-d6-cont-20260926","depth8-tiny-d6-split-20260926",
        "depth8-tiny-d8-cont-20260926","depth8-tiny-d8-split-20260926"] {
        let path=sibling.join(name);
        if path.exists(){roots.insert(path);}
    }
    let mut evidence=0u64;
    for root in roots {
        evidence=evidence.checked_add(super::super::new_bytes(&root)?)
            .ok_or_else(||bad("depth evidence bytes overflow"))?;
    }
    let executable=std::fs::metadata(std::env::current_exe()?)?.len();
    let logical=evidence.checked_add(executable).and_then(|n|n.checked_add(reserve_bytes))
        .ok_or_else(||bad("depth logical bytes overflow"))?;
    let target=(completion::allocated(Path::new("target/debug"))?+1023)/1024;
    if logical>1536*1024*1024 || target>p.target_baseline_kib+1024*1024 {
        return Err(bad("depth artifact/target budget"));
    }
    Ok((logical,target))
}
fn usage(p: &Plan) -> Result<(f64, usize, usize)> {
    if execution(p) {
        let (prep,training,evaluation,calls,tokens)=execution_usage(p)?;
        let limits=execution_limits(p);
        if prep>limits.preparation || training>limits.training || evaluation>limits.evaluation
            || prep+training+evaluation>limits.total
            || calls>1760 || tokens>262144 {return Err(bad("depth execution budget"));}
        return Ok((prep+training+evaluation,calls,tokens));
    }
    let prep: binary::Value = read_confirmed(&p.root.join("preparation.r3b"))?;
    let mut seconds = prep["model_seconds"].as_f64().ok_or_else(||bad("depth prep seconds"))?;
    let direct = p.root.join("direct-numeric.r3b");
    if direct.exists() {
        let d: binary::Value = read_confirmed(&direct)?;
        seconds += d["model_seconds"].as_f64().ok_or_else(||bad("depth direct seconds"))?;
    }
    let mut calls = 0; let mut tokens = 0;
    for arm in ARMS {
        for r in history(p, arm)? {seconds += r.seconds;}
        for step in [0, 128, 512, 998] {
            for index in 0..16 {
                let entered = p.root.join(arm).join(format!("eval-{step}-segment-{index:03}-entered.r3b"));
                let path = p.root.join(arm).join(format!("eval-{step}-segment-{index:03}-returned.r3b"));
                if entered.exists() != path.exists() {return Err(bad("depth UNKNOWN evaluation segment"));}
                if !path.exists(){break;}
                let r: binary::Value = read_confirmed(&path)?;
                let mark: binary::Value = read_confirmed(&entered)?;
                if r["policy"] != policy(p, arm)? || r["step"] != step
                    || mark["policy"] != r["policy"] || mark["step"] != step
                    || mark["index"] != index || mark["native"] != r["native"]
                    || r["native"] != endpoint_at(p,arm,step)?.hash {
                    return Err(bad("depth evaluation receipt"));
                }
                seconds += r["seconds"].as_f64().ok_or_else(||bad("depth eval seconds"))?;
                calls += r["calls"].as_u64().ok_or_else(||bad("depth eval calls"))? as usize;
                tokens += r["tokens"].as_u64().ok_or_else(||bad("depth eval tokens"))? as usize;
            }
        }
    }
    if !seconds.is_finite() || seconds < 0. || calls > 1840 || tokens > 262144 {
        return Err(bad("depth total model budget"));
    }
    Ok((seconds, calls, tokens))
}
fn execution_usage(p: &Plan) -> Result<(f64,f64,f64,usize,usize)> {
    let prep:binary::Value=read_confirmed(&p.root.join("preparation.r3b"))?;
    let mut prep_seconds=prep["seconds"].as_f64().ok_or_else(||bad("depth execution prep seconds"))?
        +preflight_usage(&p.root)?;
    if p.root.join("repair-seal.r3b").exists() {
        if p.root.join("repair-successor.r3b").exists() {
            let s=repair_successor(p)?;
            let old=repair_seal_inner(p,true)?;
            prep_seconds+=old.additional_charge_seconds+JOURNAL_CLOSE_RESERVE
                +s.additional_first_seal_seconds+s.successor_outer_reserve;
        } else {prep_seconds+=repair_seal(p)?.additional_charge_seconds+JOURNAL_CLOSE_RESERVE;}
    }
    let link=p.root.join("preflight-link.r3b");
    if link.exists() {
        prep_seconds+=parent_preflight_extra(&p.root,prep["seconds"].as_f64()
            .ok_or_else(||bad("depth execution body seconds"))?)?;
    } else if p.root.join("plan.r3b").exists()
        && file_hash(&p.root.join("plan.r3b"))?!=FROZEN_EXECUTION_PLAN {
        return Err(bad("depth execution parent preflight link missing"));
    }
    let old:Plan=read_confirmed(&p.root.join("plan.r3b"))?;
    if p.root.join("execute-check.r3b").exists() {
        let check:binary::Value=read_confirmed(&p.root.join("execute-check.r3b"))?;
        if check["policy"]!=digest(&old)? || check["amendment"]!=file_hash(&amendment_path(p))?
            || check["generation"].as_u64()!=Some(0) || check["optimizer"].as_u64()!=Some(0) {
            return Err(bad("depth execution check receipt"));
        }
    }
    let(mut training,mut evaluation,mut calls,mut tokens)=(0f64,0f64,0usize,0usize);
    for arm in ARMS {
        history(p,arm)?;
        for step in [0,128,512,998] {
            for index in 0..16 {
                let dir=p.root.join(arm);
                let entered=dir.join(format!("eval-{step}-segment-{index:03}-entered.r3b"));
                let returned=dir.join(format!("eval-{step}-segment-{index:03}-returned.r3b"));
                if entered.exists()!=returned.exists(){return Err(bad("depth UNKNOWN evaluation segment"));}
                if !returned.exists(){break;}
                let r:binary::Value=read_confirmed(&returned)?;
                let mark:binary::Value=read_confirmed(&entered)?;
                let observed_policy=evidence_policy(p,arm,step)?;
                if mark["policy"]!=observed_policy || r["policy"]!=mark["policy"]
                    || r["native"]!=mark["native"] || r["step"]!=step
                    || r["calls"]!=r["control"]["generation_calls"] {
                    return Err(bad("depth execution evaluation receipt"));
                }
                let seconds=r["seconds"].as_f64().ok_or_else(||bad("depth evaluation seconds"))?;
                if !seconds.is_finite() || seconds<0. {return Err(bad("depth evaluation seconds"));}
                calls+=r["calls"].as_u64().ok_or_else(||bad("depth calls"))? as usize;
                tokens+=r["tokens"].as_u64().ok_or_else(||bad("depth tokens"))? as usize;
            }
        }
    }
    let commands=p.root.join("commands");
    let mut completed_commands=0usize;
    for index in 0..128 {
        let entered=commands.join(format!("command-{index:03}-entered.r3b"));
        if !entered.exists(){break;}
        let returned=commands.join(format!("command-{index:03}-returned.r3b"));
        if !returned.exists() {
            if ACTIVE_COMMAND.load(Ordering::Relaxed)==index {continue;}
            return Err(bad("depth UNKNOWN command"));
        }
        let mark:binary::Value=read_confirmed(&entered)?;
        let done:binary::Value=read_confirmed(&returned)?;
        let (command_policy,command_source,command_binary)=command_identity(p,index)?;
        if mark["policy"]!=command_policy || done["policy"]!=mark["policy"]
            || done["kind"]!=mark["kind"] || done["index"]!=index
            || mark["index"]!=index || done["bucket"]!=mark["bucket"] {
            return Err(bad("depth command receipt identity"));
        }
        if index>=6 {
            let predecessor=command_predecessor(&p.root,index)?;
            let kind=mark["kind"].as_str().ok_or_else(||bad("depth command kind"))?;
            let bucket=mark["bucket"].as_str().ok_or_else(||bad("depth command bucket"))?;
            let attempt=digest(&(p.root.as_path(),index,kind,bucket,&predecessor,&command_source,&command_binary))?;
            if mark["predecessor"]!=predecessor || done["predecessor"]!=predecessor
                || mark["attempt"]!=attempt || done["attempt"]!=attempt
                || mark["source"]!=command_source || done["source"]!=command_source
                || mark["binary"]!=command_binary || done["binary"]!=command_binary
                || !["COMPLETED","ERROR","FINALIZED","RESOURCE_NOT_MET"]
                    .contains(&done["result"].as_str().unwrap_or("")) {
                return Err(bad("depth command chained identity"));
            }
        }
        let seconds=if mark["kind"]=="finish" {
            done["charged_seconds"].as_f64().ok_or_else(||bad("depth final charged seconds"))?
        } else {done["seconds"].as_f64().ok_or_else(||bad("depth command seconds"))?};
        if !seconds.is_finite()||seconds<0. {return Err(bad("depth command seconds"));}
        match mark["bucket"].as_str().ok_or_else(||bad("depth command bucket"))? {
            "preparation"=>prep_seconds+=seconds,
            "training"=>training+=seconds,
            "evaluation"=>evaluation+=seconds,
            _=>return Err(bad("depth command bucket")),
        }
        completed_commands+=1;
    }
    for entry in std::fs::read_dir(&commands)? {
        let name=entry?.file_name().to_string_lossy().to_string();
        if let Some(suffix)=name.strip_prefix("command-") {
            let Some((number,kind))=suffix.split_once('-') else {return Err(bad("depth command filename"));};
            let at=number.parse::<usize>().map_err(|_|bad("depth command filename"))?;
            let active=ACTIVE_COMMAND.load(Ordering::Relaxed)==at
                && at==completed_commands && kind=="entered.r3b";
            if (at>=completed_commands && !active)
                || !["entered.r3b","returned.r3b"].contains(&kind) {
                return Err(bad("depth command gap/duplicate"));
            }
        }
    }
    if [prep_seconds,training,evaluation].iter().any(|v|!v.is_finite()||*v<0.) {
        return Err(bad("depth execution nonfinite elapsed"));
    }
    Ok((prep_seconds,training,evaluation,calls,tokens))
}
fn final_cost(limits:ExecutionLimits,before:[f64;3],elapsed:f64,reserve:f64)->Result<([f64;3],bool)> {
    if !elapsed.is_finite() || elapsed<0. || !reserve.is_finite() || reserve<0.
        || before.iter().any(|v|!v.is_finite()||*v<0.) {return Err(bad("depth final nonfinite cost"));}
    let after=[before[0],before[1],before[2]+elapsed+reserve];
    if after.iter().any(|v|!v.is_finite()) {return Err(bad("depth final cost overflow"));}
    let within=after[0]<=limits.preparation && after[1]<=limits.training
        && after[2]<=limits.evaluation && after.iter().sum::<f64>()<=limits.total;
    Ok((after,within))
}
#[derive(Clone,Copy)]
struct FinalIdentity<'a> {
    policy:&'a str,index:usize,predecessor:&'a str,attempt:&'a str,
    source:&'a str,binary:&'a str,manifest:&'a str,
}
fn emit_final_closure(payload:&Path,receipt:&Path,id:FinalIdentity<'_>,before:[f64;3],
    calls:usize,tokens:usize,measure:impl FnOnce()->f64,limits:ExecutionLimits)->Result<()> {
    let body:binary::Value=read_confirmed(payload)?;
    if body["status"]!="FINALIZATION_PENDING" || body["resource_status"]!="RESOURCE_PENDING"
        || body["manifest_hash"]!=id.manifest || body["attempt"]!=id.attempt {
        return Err(bad("depth final pending payload mismatch"));
    }
    let payload_hash=file_hash(payload)?;
    let measured=measure();
    let (after,within)=final_cost(limits,before,measured,JOURNAL_CLOSE_RESERVE)?;
    let report=binary::record!({"policy":id.policy,"index":id.index,"kind":"finish","bucket":"evaluation",
        "predecessor":id.predecessor,"attempt":id.attempt,"source":id.source,"binary":id.binary,
        "seconds":measured,"charged_seconds":measured+JOURNAL_CLOSE_RESERVE,
        "journal_close_reserve_seconds":JOURNAL_CLOSE_RESERVE,"journal_close_actual_seconds":"UNKNOWN",
        "whole_process_wall":"UNKNOWN","before":before,"after":after,
        "calls":calls,"tokens":tokens,"payload_hash":payload_hash,
        "manifest_hash":id.manifest,"result":if within{"FINALIZED"}else{"RESOURCE_NOT_MET"}});
    if binary::to_storage_vec(&report)?.len()>64*1024 {return Err(bad("depth final receipt size"));}
    publish_confirmed(receipt,&report)
}
fn verify_final_closure(payload:&Path,receipt:&Path,id:FinalIdentity<'_>,
    after_usage:[f64;3],calls:usize,tokens:usize,limits:ExecutionLimits)->Result<()> {
    verify_final_pair(payload,receipt,id.manifest,id.attempt,id.predecessor)?;
    let done:binary::Value=read_confirmed(receipt)?;
    let before=done["before"].as_array().ok_or_else(||bad("depth final before"))?;
    if before.len()!=3 {return Err(bad("depth final before size"));}
    let before=[before[0].as_f64().ok_or_else(||bad("depth final before prep"))?,
        before[1].as_f64().ok_or_else(||bad("depth final before train"))?,
        before[2].as_f64().ok_or_else(||bad("depth final before eval"))?];
    let measured=done["seconds"].as_f64().ok_or_else(||bad("depth final measured seconds"))?;
    let (after,within)=final_cost(limits,before,measured,JOURNAL_CLOSE_RESERVE)?;
    if !within || done["policy"]!=id.policy || done["index"]!=id.index
        || done["source"]!=id.source || done["binary"]!=id.binary
        || done["kind"]!="finish" || done["bucket"]!="evaluation"
        || done["after"]!=binary::record!(after) || done["charged_seconds"]!=measured+JOURNAL_CLOSE_RESERVE
        || done["journal_close_reserve_seconds"]!=JOURNAL_CLOSE_RESERVE
        || done["whole_process_wall"]!="UNKNOWN"
        || after_usage!=after || done["calls"]!=calls || done["tokens"]!=tokens {
        return Err(bad("depth final closure cost recount"));
    }
    Ok(())
}
fn command_predecessor(root:&Path,index:usize)->Result<String> {
    if index==0 {Ok(FROZEN_EXECUTION_PLAN.to_string())}
    else {file_hash(&root.join("commands").join(format!("command-{:03}-returned.r3b",index-1)))}
}
fn tracked_command(root:&Path, kind:&str, bucket:&str, action:impl FnOnce()->Result<()>)->Result<()> {
    let started=Instant::now();
    let root=root.canonicalize()?;
    let p=checked(&root)?;
    if !execution(&p) {return action();}
    let dir=root.join("commands");
    std::fs::create_dir_all(&dir)?;
    let mut index=0usize;
    while index<128 && dir.join(format!("command-{index:03}-entered.r3b")).exists() {
        if !dir.join(format!("command-{index:03}-returned.r3b")).exists() {
            return Err(bad("depth previous command UNKNOWN"));
        }
        index+=1;
    }
    if index==128 {return Err(bad("depth command receipt limit"));}
    let entered=dir.join(format!("command-{index:03}-entered.r3b"));
    let returned=dir.join(format!("command-{index:03}-returned.r3b"));
    let predecessor=command_predecessor(&root,index)?;
    let attempt=digest(&(root.as_path(),index,kind,bucket,&predecessor,&p.source,&p.runtime.binary))?;
    publish_confirmed(&entered,&binary::record!({"policy":digest(&p)?,"index":index,"kind":kind,"bucket":bucket,
        "predecessor":predecessor,"attempt":attempt,"source":p.source,"binary":p.runtime.binary}))?;
    ACTIVE_COMMAND.store(index,Ordering::Relaxed);
    let result=(||->Result<()> {
        let (prep,training,evaluation,_,_)=execution_usage(&p)?;
        let limits=execution_limits(&p);
        if prep>=limits.preparation || training>=limits.training || evaluation>=limits.evaluation
            || prep+training+evaluation>=limits.total {
            return Err(bad("depth command budget exhausted"));
        }
        action()
    })();
    let elapsed=started.elapsed().as_secs_f64();
    let closure=if kind=="finish" && result.is_ok() {
        Some((||->Result<()>{
            let payload=p.root.join("final-report.r3b");
            let manifest=final_manifest_hash(&p,&predecessor)?;
            let (prep,training,evaluation,calls,tokens)=execution_usage(&p)?;
            let before=[prep,training,evaluation];
            let pol=digest(&p)?;
            emit_final_closure(&payload,&returned,FinalIdentity{policy:&pol,index,
                predecessor:&predecessor,attempt:&attempt,source:&p.source,binary:&p.runtime.binary,
                manifest:&manifest},before,calls,tokens,||started.elapsed().as_secs_f64(),execution_limits(&p))
        })())
    } else {None};
    let saved=match closure {
        Some(Ok(()))=>Ok(()),
        Some(Err(e))=>Err(e),
        None=>publish_confirmed(&returned,&binary::record!({"policy":digest(&p)?,"index":index,
            "kind":kind,"bucket":bucket,"predecessor":predecessor,"attempt":attempt,
            "source":p.source,"binary":p.runtime.binary,"seconds":elapsed,
            "result":if result.is_ok(){"COMPLETED"}else{"ERROR"}})),
    };
    ACTIVE_COMMAND.store(usize::MAX,Ordering::Relaxed);
    saved?;
    if kind=="finish" {
        let final_receipt:binary::Value=read_confirmed(&returned)?;
        if final_receipt["result"]!="FINALIZED" {return Err(bad("depth final resource not met"));}
    }
    result
}
fn train(root: &Path, arm: &str, until: usize) -> Result<()> {
    let command_started=Instant::now();
    let p = checked(root)?; admitted(&p)?; arm_index(arm)?;
    resource_guard(&p,if execution(&p){192*1024*1024}else{96*1024*1024})?;
    if ![1, 32, 128, 512].contains(&until) { return Err(bad("depth train endpoint")); }
    let lock = std::fs::OpenOptions::new().create(true).read(true).write(true).open(p.root.join("writer.lock"))?;
    lock.try_lock().map_err(|_|bad("depth heavy process active"))?;
    let prior = history(&p, arm)?;
    if prior.iter().any(|r|r.stop != "COMPLETED") { return Err(bad("depth sticky training stop")); }
    let before = endpoint(&p, arm)?;
    let screen0:binary::Value=read_confirmed(&p.root.join("screen-0.r3b"))?;
    if screen0!=initial_screen_value(&p)? {return Err(bad("depth initial parity gate"));}
    let required_next=match before.state.committed {0=>1,1=>32,32=>128,128=>512,_=>return Err(bad("depth train cursor"))};
    if until!=required_next {return Err(bad("depth registered endpoint order"));}
    if until <= before.state.committed || prior.iter().map(|r|r.backwards).sum::<usize>() >= 528 {
        return Err(bad("depth training cursor/backward limit"));
    }
    if before.state.committed == 128 && until > 128 {
        let sentinel: binary::Value = read_confirmed(&p.root.join("screen-128.r3b"))?;
        if sentinel!=sentinel_value_128(&p)? || sentinel["QUALITY_COLLAPSE_STOP"] != false {
            return Err(bad("depth sentinel collapse/unknown"));
        }
    }
    if before.state.committed==32 {
        let forecast:binary::Value=read_confirmed(&p.root.join("cost-forecast-32.r3b"))?;
        if forecast!=forecast_value_32(&p)? || forecast["continue"]!=true {
            return Err(bad("depth 32-step conservative cost stop"));
        }
    }
    let (seconds, _, _) = usage(&p)?;
    let training_remaining=if execution(&p) {
        let (_,used,_,_,_)=execution_usage(&p)?;execution_limits(&p).training-used
    } else {TRAIN_CUTOFF-seconds};
    if training_remaining <= command_started.elapsed().as_secs_f64() {
        return Err(bad("depth training cutoff"));
    }
    let verified=VerifiedStudy::load(&p)?;
    let validation_seconds=command_started.elapsed().as_secs_f64();
    let samples=&verified.samples;
    let device = Backend::Metal0.open()?; p.runtime.verify(&device)?;
    let native_started=Instant::now();
    let (mut core, mut adam) = load_endpoint(&before, device)?;
    let native_load_seconds=native_started.elapsed().as_secs_f64();
    let cancel = Arc::new(AtomicBool::new(false)); let signal = cancel.clone();
    ctrlc::set_handler(move || signal.store(true, Ordering::Relaxed)).map_err(|e|bad(&e.to_string()))?;
    let total_cap=if execution(&p){execution_limits(&p).total}else{MAX_SECONDS};
    let remaining=(total_cap-seconds).min(training_remaining)-command_started.elapsed().as_secs_f64();
    if remaining<=0. {return Err(bad("depth model time cap"));}
    let mut ctl = RunControl::new(cancel, Duration::from_secs_f64(remaining.min(1800.)),16*1024*1024)?;
    ctl.set_call_limits(0,0);
    let index = prior.len(); let dir = p.root.join(arm);
    let trace = dir.join(format!("segment-{index:03}-trace.r3rows"));
    let mut f = std::fs::OpenOptions::new().write(true).create_new(true).open(&trace)?;
    publish_confirmed(&dir.join(format!("segment-{index:03}-entered.r3b")),&binary::record!({
        "policy":policy(&p,arm)?,"before":before,"index":index,"until":until}))?;
    let mut st = before.state.clone();
    if execution(&p) && st.committed==0 {
        st.policy=policy(&p,arm)?;
        st.runtime=p.runtime.clone();
    }
    let (mut backwards, mut in_optimizer) = (0usize, false);
    let (mut update_seconds,mut raw_publication_seconds)=(0f64,0f64);
    let result = (|| -> Result<()> {
        while st.committed < until {
            ctl.check("before_depth_update")?;
            if prior.iter().map(|r|r.backwards).sum::<usize>() + backwards >= 528 { return Err(bad("depth backward cap")); }
            let row = &p.tape[st.committed];
            let b = batch(&samples, row, core.device())?;
            let publishing=Instant::now();
            append_row(&mut f, &binary::record!({"phase":"ENTERED","cursor":st.committed+1,"batch":row}))?;
            raw_publication_seconds+=publishing.elapsed().as_secs_f64();
            let update = super::super::update(&core, &mut adam, &mut st, &b, &mut ctl, &mut backwards, &mut in_optimizer)?;
            update_seconds+=update["seconds"].as_f64().ok_or_else(||bad("depth update timing"))?;
            let publishing=Instant::now();
            append_row(&mut f, &update)?;
            raw_publication_seconds+=publishing.elapsed().as_secs_f64();
        }
        Ok(())
    })();
    if let Err(e)=&result { ctl.classify_error(e); }
    let stop=ctl.reason().unwrap_or("COMPLETED").to_string();
    let checkpoint_started=Instant::now();
    let after = if !in_optimizer && st.committed > before.state.committed {
        save_endpoint(&dir, &mut core, &adam, st, &format!("segment-{index:03}"))?
    } else {before.clone()};
    let checkpoint_seconds=checkpoint_started.elapsed().as_secs_f64();
    let publishing=Instant::now();
    append_row(&mut f, &binary::record!({"phase":"RETURNED","after":after,"stop":stop,
        "discarded_backwards":backwards.saturating_sub(after.state.committed-before.state.committed)}))?;
    raw_publication_seconds+=publishing.elapsed().as_secs_f64();
    let wall=command_started.elapsed().as_secs_f64();
    let classified=validation_seconds+native_load_seconds+update_seconds+raw_publication_seconds+checkpoint_seconds;
    let timing=binary::record!({"validation_read_hash_seconds":validation_seconds,
        "native_load_seconds":native_load_seconds,"input_profile":verified.load_profile,
        "forward_backward_optimizer_seconds":update_seconds,"raw_publication_seconds":raw_publication_seconds,
        "checkpoint_seconds":checkpoint_seconds,"unclassified_residual_seconds":(wall-classified).max(0.),
        "classified_seconds":classified,"wall_seconds":wall,"gpu_only_seconds":"UNKNOWN"});
    let r=Run {policy:policy(&p,arm)?,arm:arm.into(), before:before.clone(), after:after.clone(),
        backwards,input:after.state.input_tokens-before.state.input_tokens,
        target:after.state.target_tokens-before.state.target_tokens,seconds:wall,
        stop:stop.clone(),trace:trace.clone(),trace_hash:file_hash(&trace)?,control:ctl.receipt(),
        timing:if execution(&p){Some(timing)}else{None}};
    publish_confirmed(&dir.join(format!("segment-{index:03}-returned.r3b")),&r)?;
    println!("DEPTH_TRAIN arm={arm} local={} logical={} adam={} backwards={backwards} input={} target={} seconds={:.3} stop={stop} native={}",
        after.state.committed,PARENT_GLOBAL+after.state.committed,after.state.adam_clock,r.input,r.target,r.seconds,after.hash);
    result
}
fn numeric_close(name:&str,a:&Tensor,b:&Tensor)->Result<(f64,f64)> {
    if a.dims()!=b.dims(){return Err(bad("depth numeric shape"));}
    let x=a.flatten_all()?.to_vec1::<f32>()?;
    let y=b.flatten_all()?.to_vec1::<f32>()?;
    let mut max=0f64;let mut numerator=0f64;let mut denominator=0f64;
    for (&x,&y) in x.iter().zip(&y) {
        let d=(x as f64-y as f64).abs();max=max.max(d);
        numerator+=d*d;denominator+=(x as f64)*(x as f64);
        if !x.is_finite()||!y.is_finite()||d>5e-5+5e-5*(x as f64).abs(){
            return Err(bad(&format!("depth numeric pointwise {name} max={max}")));
        }
    }
    let nrmse=(numerator/denominator.max(1e-20)).sqrt();
    if nrmse>1e-5{return Err(bad(&format!("depth numeric nrmse {name} {nrmse}")));}
    Ok((max,nrmse))
}
fn direct(root:&Path)->Result<()> {
    let command_started=Instant::now();
    let p=checked(root)?;
    let device=Backend::Metal0.open()?;p.runtime.verify(&device)?;
    let (small,_)=load_endpoint(&endpoint(&p,"D6")?,device.clone())?;
    let (deep,_)=load_endpoint(&endpoint(&p,"D8")?,device.clone())?;
    let (Core::Tr(small),Core::Tr(deep))=(small,deep) else{return Err(bad("depth direct cores"));};
    if small.weights_content_id()?!=p.parent.content
        || deep.config!=Config::depth8(small.config.vocab){return Err(bad("depth direct identity"));}
    let (_,comparison)=parent(&p)?;let(_,_,_,tok,samples)=inputs(&comparison)?;
    let first=panels(&p,0)?;let e=&first[0].1[0];
    let prompt=tok.prepare_with_framing(&e.request,Framing::QuestionEvidence,2048,"depth-direct")?.token_ids;
    let sample=&samples[p.tape[0][4]];
    let mut results=BTreeMap::new();
    for (name,ids) in [("prompt",prompt), ("teacher-prefix",sample.tokens[..sample.tokens.len()-1].to_vec()),
        ("boundary255",vec![8u32;255]),("boundary256",vec![8u32;256]),
        ("boundary257",vec![8u32;257]),("boundary513",vec![8u32;513])] {
        let n=ids.len();if n==0{return Err(bad("depth numeric empty ids"));}
        let input=Tensor::from_vec(ids.clone(),(1,n),&device)?;
        let (a,b)=(small.forward(&input,None)?,deep.forward(&input,None)?);
        let (max,nrmse)=numeric_close(name,&a,&b)?;
        let mut cache_a=small.cache(name);let mut cache_b=deep.cache(name);
        let split=n.min(128);
        let prefix=Tensor::from_vec(ids[..split].to_vec(),(1,split),&device)?;
        small.forward_cached(&prefix,&mut cache_a,name)?;
        deep.forward_cached(&prefix,&mut cache_b,name)?;
        let tail=if n>split {Tensor::from_vec(ids[split..].to_vec(),(1,n-split),&device)?}
            else {prefix.clone()};
        let cached_a=if n>split {small.forward_cached(&tail,&mut cache_a,name)?}
            else {small.forward_cached(&input,&mut small.cache("separate"),"separate")?};
        let cached_b=if n>split {deep.forward_cached(&tail,&mut cache_b,name)?}
            else {deep.forward_cached(&input,&mut deep.cache("separate"),"separate")?};
        let ca=cached_a.narrow(1,cached_a.dim(1)?-1,1)?;
        let cb=cached_b.narrow(1,cached_b.dim(1)?-1,1)?;
        let full_a=a.narrow(1,n-1,1)?;let full_b=b.narrow(1,n-1,1)?;
        numeric_close(&format!("cached-pair-{name}"),&ca,&cb)?;
        let cached_max=ca.flatten_all()?.to_vec1::<f32>()?.iter()
            .zip(full_a.flatten_all()?.to_vec1::<f32>()?)
            .map(|(&x,y)|(x-y).abs() as f64).fold(0f64,f64::max);
        let cached_max_b=cb.flatten_all()?.to_vec1::<f32>()?.iter()
            .zip(full_b.flatten_all()?.to_vec1::<f32>()?)
            .map(|(&x,y)|(x-y).abs() as f64).fold(0f64,f64::max);
        if cached_max>5e-4||cached_max_b>5e-4{return Err(bad("depth cached/full gold-prefix tolerance"));}
        if n>=257 && (cache_a.retained_tokens()!=vec![256;5].into_iter().chain([n]).collect::<Vec<_>>()
            || cache_b.retained_tokens()!=vec![256;7].into_iter().chain([n]).collect::<Vec<_>>()) {
            return Err(bad("depth local/global cache topology"));
        }
        results.insert(name.to_string(),binary::record!({"max":max,"nrmse":nrmse,
            "cached_full_max_D6":cached_max,"cached_full_max_D8":cached_max_b,
            "retained_D6":cache_a.retained_tokens(),"retained_D8":cache_b.retained_tokens()}));
    }
    publish_confirmed(&p.root.join("direct-numeric.r3b"),&binary::record!({"policy":digest(&p)?,
        "actual_device":p.runtime.actual_device,"numeric":results,"generation":0,"teacher":0,
        "optimizer":0,"backward":0,"model_seconds":command_started.elapsed().as_secs_f64()}))?;
    println!("DEPTH_DIRECT numeric=PASS actual_device={} generation=0 optimizer=0",p.runtime.actual_device);
    Ok(())
}
fn tiny_model(arm:&str)->Result<Core> {
    arm_index(arm)?;
    let base=Transformer::init(Config::tiny(264),20260926,Device::Cpu)?;
    if arm=="D6" {return Ok(Core::Tr(base));}
    let mut config=Config::tiny(264);
    config.profile="NATIVE_TRPP_EXPERIMENTAL_V1".into();
    config.layers=4;config.local_layers=3;config.validate()?;
    let extra=Transformer::init(config.clone(),20260927,Device::Cpu)?;
    let mut tensors=extra.base_tensors();
    for (old,new) in [(0,0),(1,3)] {
        for suffix in ["attn_norm","ffn_norm","q_norm","k_norm","q","k","v","o","gate","up","down"] {
            let name=format!("layer.{new}.{suffix}");
            let source=base.vars[&format!("layer.{old}.{suffix}")].as_tensor();
            tensors.insert(name,Tensor::from_vec(source.flatten_all()?.to_vec1::<f32>()?,source.dims().to_vec(),&Device::Cpu)?);
        }
    }
    for name in ["embedding","final_norm"] {
        let source=base.vars[name].as_tensor();
        tensors.insert(name.into(),Tensor::from_vec(source.flatten_all()?.to_vec1::<f32>()?,source.dims().to_vec(),&Device::Cpu)?);
    }
    for index in [1,2] {for suffix in ["o","down"] {
        let name=format!("layer.{index}.{suffix}");
        tensors.insert(name.clone(),Tensor::zeros(config.shapes()[&name].as_slice(),DType::F32,&Device::Cpu)?);
    }}
    let model=Transformer::from_tensors(config,tensors,Device::Cpu)?;
    let ids=Tensor::from_vec(vec![8u32,9,10,11],(1,4),&Device::Cpu)?;
    numeric_close("tiny-identity",&base.forward(&ids,None)?,&model.forward(&ids,None)?)?;
    Ok(Core::Tr(model))
}
fn tiny(output:&Path,arm:&str,until:usize,resume:bool)->Result<()> {
    if ![1,2].contains(&until) {return Err(bad("depth tiny endpoint"));}
    let policy=digest(&(CONTRACT,"TINY_PROCESS",arm,source()?))?;
    let mut core;let mut adam;let mut st;
    if resume {
        let ep:Endpoint=read_confirmed(&output.join("step-1.r3b"))?;
        if until!=2||ep.state.committed!=1||ep.state.policy!=policy {return Err(bad("depth tiny resume policy"));}
        (core,adam)=load_endpoint(&ep,Device::Cpu)?;st=ep.state;
    }else {
        if output.exists() {return Err(bad("depth tiny output exists"));}
        std::fs::create_dir(output)?;
        core=tiny_model(arm)?;
        core.bind(&neural::hash(b"depth8-tiny-tokenizer-v1"))?;
        adam=Adam::new(core.vars())?;
        let c=match core.descriptor(){ComparisonCore::Trpp(c)=>c,_=>return Err(bad("depth tiny core"))};
        let mut config=train_config();config.max_steps=2;config.warmup=2;config.seq_len=32;config.microbatch=2;
        st=ComparisonState {schema:1,core:ComparisonCore::Trpp(c),policy:policy.clone(),
            tokenizer:neural::hash(b"depth8-tiny-tokenizer-v1"),framing:Framing::QuestionEvidence.digest(),
            runtime:RuntimeProfile::capture(Backend::Cpu,&Device::Cpu)?,objective:checkpoint::ANSWER_MEAN_FAMILY,
            optimizer:"FRESH_ADAMW_ALL_V1".into(),config,committed:0,adam_clock:0,input_tokens:0,target_tokens:0};
    }
    let samples=[Sample {tokens:vec![8,9,10,11,12,EOS],response_start:4,curriculum:false},
        Sample {tokens:vec![8,13,14,15,16,EOS],response_start:4,curriculum:false}];
    let ctl=Arc::new(AtomicBool::new(false));let mut ctl=RunControl::new(ctl,Duration::from_secs(120),16*1024*1024)?;
    ctl.set_call_limits(0,0);let mut backwards=0;let mut in_optimizer=false;
    while st.committed<until {
        let b=batch(&samples,&[0,1],core.device())?;
        super::super::update(&core,&mut adam,&mut st,&b,&mut ctl,&mut backwards,&mut in_optimizer)?;
        if in_optimizer {return Err(bad("depth tiny interrupted optimizer"));}
        let ep=save_endpoint(output,&mut core,&adam,st.clone(),"tiny")?;
        publish_confirmed(&output.join(format!("step-{}.r3b",st.committed)),&ep)?;
    }
    println!("DEPTH_TINY arm={arm} step={} Adam={} backward={backwards} native=PASS",st.committed,st.adam_clock);
    Ok(())
}
fn verify_tiny(continuous:&Path,split:&Path)->Result<()> {
    let a:Endpoint=read_confirmed(&continuous.join("step-2.r3b"))?;
    let b:Endpoint=read_confirmed(&split.join("step-2.r3b"))?;
    if a.state!=b.state {return Err(bad("depth tiny state mismatch"));}
    let (_,aw,aa)=neural::artifact::load_comparison(&a.path,&a.state,&Device::Cpu)?;
    let (_,bw,ba)=neural::artifact::load_comparison(&b.path,&b.state,&Device::Cpu)?;
    for (x,y) in [(&aw,&bw),(&aa,&ba)] {for (name,t) in x {
        if t.flatten_all()?.to_vec1::<f32>()?!=y[name].flatten_all()?.to_vec1::<f32>()? {
            return Err(bad("depth tiny continuous/split tensor"));
        }
    }}
    let mut wrong=b.state.clone();
    wrong.tokenizer=neural::hash(b"depth8-wrong-tokenizer");
    if neural::artifact::load_comparison(&b.path,&wrong,&Device::Cpu).is_ok() {
        return Err(bad("depth tiny wrong tokenizer accepted"));
    }
    wrong=b.state.clone(); wrong.core=if matches!(wrong.core,ComparisonCore::Trpp(ref c) if c.layers==4) {
        ComparisonCore::Trpp(Config::tiny(264))
    } else {
        let mut c=Config::tiny(264);c.profile="NATIVE_TRPP_EXPERIMENTAL_V1".into();c.layers=4;c.local_layers=3;
        ComparisonCore::Trpp(c)
    };
    if neural::artifact::load_comparison(&b.path,&wrong,&Device::Cpu).is_ok() {
        return Err(bad("depth tiny wrong core accepted"));
    }
    let mut dead_gradient=false;
    if let ComparisonCore::Trpp(c)=&b.state.core {if c.layers==4 {
        let (model,_) = load_endpoint(&b,Device::Cpu)?;
        let samples=[Sample {tokens:vec![8,9,10,11,12,EOS],response_start:4,curriculum:false},
            Sample {tokens:vec![8,13,14,15,16,EOS],response_start:4,curriculum:false}];
        let data=batch(&samples,&[0,1],model.device())?;
        let logits=model.forward(&data)?;
        let (_,loss,_,_)=super::super::response_objective(&logits,&data,1.,true)?;
        let graph=loss.backward()?;
        for index in [1,2] {for suffix in ["o","down","q","k","v","gate","up"] {
            let name=format!("layer.{index}.{suffix}");
            let Core::Tr(ref tr)=model else {return Err(bad("depth tiny core"));};
            let g=graph.get(&tr.vars[&name]).ok_or_else(||bad("depth tiny missing gradient"))?;
            let live=g.flatten_all()?.to_vec1::<f32>()?.iter().any(|x|x.is_finite() && *x!=0.);
            dead_gradient|=!live;
        }}
    }}
    if dead_gradient {return Err(bad("depth new block dead after two updates"));}
    println!("DEPTH_TINY_PROCESS_PARITY step=2 weights_Adam_exact=true wrong_core_tokenizer_rejected=true gradient_backward={}",
        if matches!(b.state.core,ComparisonCore::Trpp(ref c) if c.layers==4){1}else{0});Ok(())
}
fn sentinel_qa_indices(ms:&[Meta])->Result<Vec<usize>> {
    let mut counts=[0usize;8];let mut indices=Vec::new();
    for (i,meta) in ms.iter().enumerate() {
        if meta.bucket<8 && counts[meta.bucket]<2 {counts[meta.bucket]+=1;indices.push(i);}
    }
    if counts!=[2;8] || indices.len()!=16 {return Err(bad("depth sentinel QA A-H two each"));}
    Ok(indices)
}
struct VerifiedStudy {
    tokenizer: ByteBpe,
    samples: Vec<Sample>,
    all: Vec<Panel>,
    load_profile: binary::Value,
}
impl VerifiedStudy {
    fn load(p: &Plan) -> Result<Self> {
        let parent_started=Instant::now();
        let (study, comparison) = parent(p)?;
        let parent_seconds=parent_started.elapsed().as_secs_f64();
        let ((corpus, train_meta, dev_meta, tokenizer, samples),profile)=super::super::inputs_profiled(&comparison)?;
        let panel_started=Instant::now();
        let all = eval_panels_prepared(&study, &comparison,&corpus,&train_meta,&dev_meta)?;
        let panel_seconds=panel_started.elapsed().as_secs_f64();
        if digest(&all)? != p.panels || tokenizer.semantic_id() != p.tokenizer_id {
            return Err(bad("depth verified study binding"));
        }
        let load_profile=binary::record!({"parent_validation_seconds":parent_seconds,
            "reference_hash_seconds":profile.reference_seconds,
            "corpus_decode_seconds":profile.corpus_seconds,
            "metadata_decode_seconds":profile.metadata_seconds,
            "tokenizer_decode_seconds":profile.tokenizer_seconds,
            "sample_framing_seconds":profile.samples_seconds,
            "token_cache_verify_seconds":profile.cache_seconds,
            "panel_build_seconds":panel_seconds,
            "unique_file_reads":profile.reads,"hashes":profile.hashes,"decodes":profile.decodes});
        Ok(Self { tokenizer, samples, all,load_profile })
    }
    fn panels(&self, step: usize, short_initial: bool) -> Result<Vec<Panel>> {
        let all = self.all.clone();
        match step {
            0 => { let n = if short_initial {4} else {8};
                Ok(all.into_iter().filter(|x|x.0=="word"||x.0=="renamed")
                    .map(|(name, episodes, metadata)| (name, episodes[..n].to_vec(), metadata[..n].to_vec())).collect())
            }
            128 if short_initial => Ok(all.into_iter().filter(|x|["word","renamed","train192"].contains(&x.0.as_str()))
                .map(|(name, episodes, metadata)| {let n=if name=="train192"{8}else{4};
                    (name,episodes[..n].to_vec(),metadata[..n].to_vec())}).collect()),
            128 => panels_from_all(all,128),
            512 => Ok(all),
            998 => Ok(all.into_iter().filter(|x|x.0=="word")
                .map(|(_,episodes,metadata)| ("review-word8".into(),episodes[..8].to_vec(),metadata[..8].to_vec())).collect()),
            _ => Err(bad("depth evaluation step")),
        }
    }
}
fn panels_from_all(all: Vec<Panel>, step: usize) -> Result<Vec<Panel>> {
    if step != 128 {return Err(bad("depth panels legacy step"));}
    all.into_iter().filter(|x|["word","renamed","train192","qa-old_qa-primary64"].contains(&x.0.as_str()))
        .map(|(name,episodes,metadata)| {
            if name!="qa-old_qa-primary64" {return Ok((name,episodes[..16].to_vec(),metadata[..16].to_vec()));}
            let indices=sentinel_qa_indices(&metadata)?;
            Ok((name,indices.iter().map(|&i|episodes[i].clone()).collect(),
                indices.iter().map(|&i|metadata[i].clone()).collect()))
        }).collect()
}
fn panels(p: &Plan, step: usize) -> Result<Vec<Panel>> {
    VerifiedStudy::load(p)?.panels(step, execution(p))
}
fn score(p: &Plan, arm: &str, step: usize, name: &str) -> Result<binary::Value> {
    let raw = p.root.join(arm).join(format!("eval-{step}-{name}.r3rows"));
    let saved: binary::Value = read_confirmed(&p.root.join(arm).join(format!("eval-{step}-{name}-score.r3b")))?;
    let panel=panels(p,step)?.into_iter().find(|v|v.0==name)
        .ok_or_else(||bad("depth score panel not in manifest"))?;
    let ep=endpoint_at(p,arm,step)?;
    let (_,comparison)=parent(p)?;
    let (_,_,_,tok,_)=inputs(&comparison)?;
    let rows=binary::read_value_records(&raw)?;
    let dir=p.root.join(arm);
    let source_raw=if step==998 {Some(file_hash(&dir.join("eval-512-word.r3rows"))?)}else{None};
    let binding=binary::record!({"policy":evidence_policy(p,arm,step)?,"arm":arm,"native":ep.hash,
        "model":ep.content,"step":step,"cases":digest(&panel.1)?,"metadata":digest(&panel.2)?,
        "tokenizer":p.tokenizer_id,"source_raw":source_raw,"accuracy_denominator":step!=998});
    if rows.len()!=panel.1.len()+1 || rows[0]!=binding {return Err(bad("depth score raw binding/count"));}
    let previous=if step==998 {Some(binary::read_value_records(&dir.join("eval-512-word.r3rows"))?)}else{None};
    for (i,row) in rows.iter().skip(1).enumerate() {
        call_attempt(&dir,&format!("eval-{step}-{name}"),"generation",&binding,&panel.1[i],i,Some(row))?;
        verify_generated(row,&tok)?;
        if row["id"]!=panel.1[i].id || row["expected"]!=panel.1[i].answer {
            return Err(bad("depth score row identity"));
        }
        if let Some(old)=&previous {
            for field in ["raw_tokens","raw_bytes","actual","finish_reason","error","generation_completed"] {
                if row[field]!=old[i+1][field] {return Err(bad("depth B raw parity"));}
            }
        }
    }
    let raw_hash=file_hash(&raw)?;
    let mut expected=panel_score(&panel,&rows[1..],&tok,&ep.content,&raw_hash)?;
    expected["policy"]=binary::record!(evidence_policy(p,arm,step)?);
    expected["model"]=binary::record!(ep.content);
    expected["raw_hash"]=binary::record!(raw_hash);
    if saved!=expected {return Err(bad("depth score raw recount mismatch"));}
    Ok(saved)
}
fn score_verified(p: &Plan, study: &VerifiedStudy, ep: &Endpoint, arm: &str,
    step: usize, panel: &Panel) -> Result<binary::Value> {
    let dir = p.root.join(arm);
    let label = format!("eval-{step}-{}", panel.0);
    let raw = dir.join(format!("{label}.r3rows"));
    let saved: binary::Value = read_confirmed(&dir.join(format!("{label}-score.r3b")))?;
    let raw_bytes=neural::read_bounded(&raw,512*1024*1024)?;
    let raw_hash=neural::hash(&raw_bytes);
    let rows = binary::value_records_from_slice(&raw_bytes)?;
    let source_raw = if step==998 {Some(file_hash(&dir.join("eval-512-word.r3rows"))?)} else {None};
    let binding = binary::record!({"policy":evidence_policy(p,arm,step)?,"arm":arm,"native":ep.hash,
        "model":ep.content,"step":step,"cases":digest(&panel.1)?,"metadata":digest(&panel.2)?,
        "tokenizer":p.tokenizer_id,"source_raw":source_raw,"accuracy_denominator":step!=998});
    if rows.len()!=panel.1.len()+1 || rows[0]!=binding {return Err(bad("depth score raw binding/count"));}
    let previous = if step==998 {Some(binary::read_value_records(&dir.join("eval-512-word.r3rows"))?)}else{None};
    for (i,row) in rows.iter().skip(1).enumerate() {
        call_attempt(&dir,&label,"generation",&binding,&panel.1[i],i,Some(row))?;
        verify_generated(row,&study.tokenizer)?;
        if row["id"]!=panel.1[i].id || row["expected"]!=panel.1[i].answer {return Err(bad("depth score row case"));}
        if let Some(old)=&previous {for field in ["raw_tokens","raw_bytes","actual","finish_reason","error","generation_completed"] {
            if row[field]!=old[i+1][field] {return Err(bad("depth B raw parity"));}
        }}
    }
    let mut expected=panel_score(panel,&rows[1..],&study.tokenizer,&ep.content,&raw_hash)?;
    expected["policy"]=binary::record!(evidence_policy(p,arm,step)?);
    expected["model"]=binary::record!(ep.content);
    expected["raw_hash"]=binary::record!(raw_hash);
    if saved!=expected {return Err(bad("depth score raw recount mismatch"));}
    Ok(saved)
}
fn publish_or_compare_score(path:&Path,value:&binary::Value)->Result<()> {
    if path.exists() {
        let old:binary::Value=read_confirmed(path)?;
        if old!=*value {return Err(bad("depth resumed score mismatch"));}
        Ok(())
    } else {publish_confirmed(path,value)}
}
fn evaluate(root: &Path, arm: &str, step: usize) -> Result<()> {
    let command_started=Instant::now();
    let p=checked(root)?; if step!=0 {admitted(&p)?;} arm_index(arm)?;
    resource_guard(&p,16*1024*1024)?;
    let inherited_direct=if execution(&p) {p.root.parent().unwrap().join("direct-numeric.r3b")}
        else {p.root.join("direct-numeric.r3b")};
    if step==0 && !inherited_direct.exists() {return Err(bad("depth numeric direct test required"));}
    if ![0,128,512,998].contains(&step) {return Err(bad("depth eval step"));}
    let lock=std::fs::OpenOptions::new().create(true).read(true).write(true).open(p.root.join("writer.lock"))?;
    lock.try_lock().map_err(|_|bad("depth heavy process active"))?;
    let ep=endpoint(&p,arm)?;
    if ep.state.committed != if step==998 {512}else{step} {return Err(bad("depth eval endpoint"));}
    if step==998 {
        let screen:binary::Value=read_confirmed(&p.root.join("screen-512.r3b"))?;
        let (table,signal)=final_screen_scores(&p)?;
        if screen["scores"]!=binary::record!(table) || screen["depth_followup_signal_before_B"]!=signal {
            return Err(bad("depth B before verified final screen"));
        }
    }
    let dir=p.root.join(arm);
    if dir.join(format!("eval-{step}-complete.r3b")).exists() {return Err(bad("depth eval already complete"));}
    let index=next_eval_segment(&dir,step)?;
    let (_,previous_calls,previous_tokens,_)=eval_receipt_total(&p,arm,step)?;
    let (seconds,calls,tokens)=usage(&p)?;
    let call_cap=if execution(&p){1760}else{1840};
    let total_cap=if execution(&p){execution_limits(&p).total}else{MAX_SECONDS};
    if seconds+command_started.elapsed().as_secs_f64()>=total_cap || calls>=call_cap || tokens>=262144 {
        return Err(bad("depth eval resource cap"));
    }
    let verified=VerifiedStudy::load(&p)?;
    let tok=&verified.tokenizer;
    let validation_seconds=command_started.elapsed().as_secs_f64();
    let device=Backend::Metal0.open()?;p.runtime.verify(&device)?;
    let native_started=Instant::now();
    let (core,_)=load_endpoint(&ep,device)?;
    let native_load_seconds=native_started.elapsed().as_secs_f64();
    let cancel=Arc::new(AtomicBool::new(false));let signal=cancel.clone();
    ctrlc::set_handler(move||signal.store(true,Ordering::Relaxed)).map_err(|e|bad(&e.to_string()))?;
    let (prep_seconds,_,eval_seconds,_,_)=if execution(&p) {
        execution_usage(&p)?
    } else {(seconds,0.,0.,0,0)};
    let bucket_remaining=if execution(&p) {
        if step==0 {execution_limits(&p).preparation-prep_seconds}
        else {execution_limits(&p).evaluation-eval_seconds}
    } else {MAX_SECONDS-seconds};
    let remaining=(total_cap-seconds).min(bucket_remaining)-command_started.elapsed().as_secs_f64();
    if remaining<=0. {return Err(bad("depth model time cap"));}
    let mut ctl=RunControl::new(cancel,Duration::from_secs_f64(remaining.min(1800.)),
        if execution(&p){12*1024*1024}else{16*1024*1024})?;
    ctl.set_call_limits(call_cap-calls,0);
    let mut generated=0usize;
    let mut collection=super::super::CollectionTiming::default();
    let mut score_seconds=0f64;
    publish_confirmed(&dir.join(format!("eval-{step}-segment-{index:03}-entered.r3b")),&binary::record!({
        "policy":policy(&p,arm)?,"native":ep.hash,"step":step,"index":index,
        "calls_before":calls,"tokens_before":tokens,"seconds_before":seconds}))?;
    let result=(||->Result<()> {
        let ps=verified.panels(step,execution(&p))?;let mut total=0usize;
        for panel in &ps {
            let label=format!("eval-{step}-{}",panel.0);
            let source_raw=if step==998 {Some(file_hash(&dir.join("eval-512-word.r3rows"))?)}else{None};
            let binding=binary::record!({"policy":policy(&p,arm)?,"arm":arm,"native":ep.hash,
                "model":ep.content,"step":step,"cases":digest(&panel.1)?,"metadata":digest(&panel.2)?,
                "tokenizer":p.tokenizer_id,"source_raw":source_raw,"accuracy_denominator":step!=998});
            let rows=collect(&core,tok,&dir,&label,&panel.1,&binding,&mut ctl,&mut generated,262144-tokens,Some(&mut collection))?;
            if step==998 {
                let old=binary::read_value_records(&dir.join("eval-512-word.r3rows"))?;
                for (a,b) in rows.iter().zip(old.iter().skip(1)) {
                    for field in ["raw_tokens","raw_bytes","actual","finish_reason","error","generation_completed"] {
                        if a[field]!=b[field] {return Err(bad("depth B raw mismatch"));}
                    }
                }
            }
            let scoring=Instant::now();
            let raw_hash=file_hash(&dir.join(format!("{label}.r3rows")))?;
            let mut value=panel_score(panel,&rows,tok,&ep.content,&raw_hash)?;
            value["policy"]=binary::record!(policy(&p,arm)?);
            value["model"]=binary::record!(ep.content);
            value["raw_hash"]=binary::record!(raw_hash);
            publish_or_compare_score(&dir.join(format!("{label}-score.r3b")),&value)?;
            score_seconds+=scoring.elapsed().as_secs_f64();
            total+=rows.len();
        }
        let expected=if step==0 {if execution(&p){8}else{16}}
            else if step==128 {if execution(&p){16}else{64}}else if step==512 {832}else{8};
        if total!=expected {return Err(bad("depth eval panel count"));}
        ctl.seal_completed_no_call()?;
        publish_confirmed(&dir.join(format!("eval-{step}-complete.r3b")),&binary::record!({
            "policy":policy(&p,arm)?,"native":ep.hash,"rows":total,"stop":"COMPLETED",
            "calls":previous_calls+ctl.receipt()["generation_calls"].as_u64().ok_or_else(||bad("depth eval calls"))? as usize,
            "tokens":previous_tokens+generated,"segments":index+1}))?;
        Ok(())
    })();
    if let Err(e)=&result {ctl.classify_error(e);}
    let stop=ctl.reason().unwrap_or("COMPLETED").to_string();
    let wall=command_started.elapsed().as_secs_f64();
    let classified=validation_seconds+native_load_seconds+collection.observe_seconds
        +collection.raw_publish_seconds+collection.validation_seconds+score_seconds;
    let r=binary::record!({"policy":policy(&p,arm)?,"native":ep.hash,"step":step,
        "calls":ctl.receipt()["generation_calls"],"tokens":generated,
        "seconds":wall,"stop":stop,"control":ctl.receipt(),
        "timing":{"validation_read_hash_seconds":validation_seconds,"input_profile":verified.load_profile,
            "native_load_seconds":native_load_seconds,"generation_observe_seconds":collection.observe_seconds,
            "raw_publication_seconds":collection.raw_publish_seconds,"reused_raw_validation_seconds":collection.validation_seconds,
            "score_recount_seconds":score_seconds,"unclassified_residual_seconds":(wall-classified).max(0.),
            "classified_seconds":classified,"wall_seconds":wall,"new_calls":collection.new_calls,
            "reused_rows":collection.reused_rows,"gpu_only_seconds":"UNKNOWN"}});
    publish_confirmed(&dir.join(format!("eval-{step}-segment-{index:03}-returned.r3b")),&r)?;
    println!("DEPTH_EVAL arm={arm} step={step} calls={} tokens={generated} seconds={} stop={stop}",r["calls"],r["seconds"]);
    result
}
fn panel_full(p: &Plan, arm: &str, step: usize, name: &str) -> Result<usize> {
    let s=score(p,arm,step,name)?;
    if name.starts_with("qa-") { score_count(&s,"full") } else {joint_count(&s,"full")}
}
fn initial_screen_value(p:&Plan)->Result<binary::Value>{
    let study=VerifiedStudy::load(p)?;
    let panels=study.panels(0,execution(p))?;
    for arm in ARMS {
        let ep=endpoint_at(p,arm,0)?;
        verified_eval_complete_with(p,arm,0,&panels,&ep)?;
        for panel in &panels {score_verified(p,&study,&ep,arm,0,panel)?;}
    }
    let mut checks=0usize;
    for name in ["word","renamed"] {
        let original=binary::read_value_records(&p.closed_root.join("C").join(format!("eval-512-{name}.r3rows")))?;
        let a=binary::read_value_records(&p.root.join("D6").join(format!("eval-0-{name}.r3rows")))?;
        let b=binary::read_value_records(&p.root.join("D8").join(format!("eval-0-{name}.r3rows")))?;
        let n=if execution(p){4}else{8};
        if a.len()!=n+1||b.len()!=n+1||original.len()!=193{return Err(bad("depth baseline row count"));}
        for i in 0..n {for field in ["raw_tokens","raw_bytes","actual","finish_reason","error","generation_completed"] {
            if a[i+1][field]!=b[i+1][field] || a[i+1][field]!=original[i+1][field] {
                return Err(bad("depth initial normal greedy/parent parity"));
            }
        }checks+=1;}
    }
    Ok(binary::record!({"policy":initial_screen_policy(p)?,"matched_cases_per_arm":checks,
        "generation_calls":if execution(p){16}else{32},"quality_inference":"NOT_EVALUATED","optimizer":0}))
}
fn next_eval_segment(dir:&Path,step:usize)->Result<usize> {
    for index in 0..16 {
        let entered=dir.join(format!("eval-{step}-segment-{index:03}-entered.r3b"));
        let returned=dir.join(format!("eval-{step}-segment-{index:03}-returned.r3b"));
        if entered.exists()!=returned.exists() {return Err(bad("depth UNKNOWN evaluation segment"));}
        if !returned.exists(){return Ok(index);}
    }
    Err(bad("depth evaluation segment limit"))
}
fn eval_receipt_total(p:&Plan,arm:&str,step:usize)->Result<(usize,usize,usize,String)> {
    let dir=p.root.join(arm);
    let ep=endpoint_at(p,arm,step)?;
    let (mut segments,mut calls,mut tokens)=(0usize,0usize,0usize);
    let mut last=String::new();
    for index in 0..16 {
        let entered=dir.join(format!("eval-{step}-segment-{index:03}-entered.r3b"));
        let returned=dir.join(format!("eval-{step}-segment-{index:03}-returned.r3b"));
        if entered.exists()!=returned.exists(){return Err(bad("depth UNKNOWN evaluation segment"));}
        if !returned.exists(){break;}
        let mark:binary::Value=read_confirmed(&entered)?;
        let r:binary::Value=read_confirmed(&returned)?;
        if mark["policy"]!=evidence_policy(p,arm,step)? || mark["native"]!=ep.hash
            || mark["step"]!=step || mark["index"]!=index
            || r["policy"]!=mark["policy"] || r["native"]!=ep.hash || r["step"]!=step
            || !["COMPLETED","TIME_BUDGET"].contains(&r["stop"].as_str().unwrap_or(""))
            || r["calls"]!=r["control"]["generation_calls"] {
            return Err(bad("depth evaluation segment receipt"));
        }
        calls=calls.checked_add(r["calls"].as_u64().ok_or_else(||bad("depth eval receipt calls"))? as usize)
            .ok_or_else(||bad("depth eval receipt calls overflow"))?;
        tokens=tokens.checked_add(r["tokens"].as_u64().ok_or_else(||bad("depth eval receipt tokens"))? as usize)
            .ok_or_else(||bad("depth eval receipt tokens overflow"))?;
        segments+=1;
        last=r["stop"].as_str().unwrap().to_string();
    }
    Ok((segments,calls,tokens,last))
}
fn verified_eval_complete(p:&Plan,arm:&str,step:usize)->Result<()> {
    let ps=panels(p,step)?;
    let ep=endpoint_at(p,arm,step)?;
    verified_eval_complete_with(p,arm,step,&ps,&ep)
}
fn verified_eval_complete_with(p:&Plan,arm:&str,step:usize,ps:&[Panel],ep:&Endpoint)->Result<()> {
    let dir=p.root.join(arm);
    let mark:binary::Value=read_confirmed(&dir.join(format!("eval-{step}-complete.r3b")))?;
    let expected=ps.iter().map(|v|v.1.len()).sum::<usize>();
    let (segments,calls,tokens,last)=eval_receipt_total(p,arm,step)?;
    let mut raw_tokens=0usize;
    for panel in ps {
        let rows=binary::read_value_records(&dir.join(format!("eval-{step}-{}.r3rows",panel.0)))?;
        if rows.len()!=panel.1.len()+1 {return Err(bad("depth completed panel row count"));}
        for row in rows.iter().skip(1) {
            raw_tokens=raw_tokens.checked_add(row["raw_tokens"].as_array()
                .ok_or_else(||bad("depth completed raw tokens"))?.len())
                .ok_or_else(||bad("depth completed raw token overflow"))?;
        }
    }
    if segments==0 || last!="COMPLETED" || calls!=expected
        || raw_tokens!=tokens
        || mark["policy"]!=evidence_policy(p,arm,step)? || mark["native"]!=ep.hash
        || mark["rows"]!=expected || mark["calls"]!=calls || mark["tokens"]!=tokens
        || mark["segments"]!=segments || mark["stop"]!="COMPLETED" {
        return Err(bad("depth completed evaluation receipt"));
    }
    Ok(())
}
fn forecast_value_32(p:&Plan)->Result<binary::Value> {
    if execution(p) {return execution_forecast_32(p);}
    let prep:binary::Value=read_confirmed(&p.root.join("preparation.r3b"))?;
    let mut used=prep["model_seconds"].as_f64().ok_or_else(||bad("depth prep time"))?;
    let direct:binary::Value=read_confirmed(&p.root.join("direct-numeric.r3b"))?;
    used+=direct["model_seconds"].as_f64().ok_or_else(||bad("depth direct time"))?;
    let mut training=0f64;let mut details=BTreeMap::new();
    for arm in ARMS {
        let runs=history(p,arm)?;
        if !runs.iter().any(|r|r.after.state.committed==32) {return Err(bad("depth forecast requires both32"));}
        let mut per_update=Vec::new();let mut fixed=0f64;
        for run in runs {
            if run.before.state.committed>=32 {break;}
            used+=run.seconds;
            let rows=binary::read_value_records(&run.trace)?;
            let mut update_elapsed=0f64;
            for row in rows {if row["commit"].as_u64().is_some() {
                let seconds=row["seconds"].as_f64().ok_or_else(||bad("depth update elapsed"))?;
                if !seconds.is_finite() || seconds<0. {return Err(bad("depth update time"));}
                per_update.push(seconds);update_elapsed+=seconds;
            }}
            fixed+= (run.seconds-update_elapsed).max(0.);
        }
        if per_update.len()!=32 {return Err(bad("depth 32 actual update sample"));}
        per_update.sort_by(f64::total_cmp);
        let p50=per_update[15];let p95=per_update[30];
        let fixed_per_segment=fixed/2.;
        training+=480.*p95+2.*fixed_per_segment;
        details.insert(arm,binary::record!({"p50_update_seconds":p50,"p95_update_seconds":p95,
            "fixed_seconds_per_segment":fixed_per_segment,"measured_updates":32}));
    }
    let mut eval_overhead=0f64;let mut generation_per_call=0f64;
    for arm in ARMS {
        verified_eval_complete(p,arm,0)?;
        let (segments,_,_,_)=eval_receipt_total(p,arm,0)?;
        let (mut wall,mut active)=(0f64,0f64);
        for index in 0..segments {
            let r:binary::Value=read_confirmed(&p.root.join(arm).join(format!("eval-0-segment-{index:03}-returned.r3b")))?;
            wall+=r["seconds"].as_f64().ok_or_else(||bad("depth baseline eval wall"))?;
            active+=r["control"]["elapsed_seconds"].as_f64().ok_or_else(||bad("depth baseline active"))?;
        }
        used+=wall;
        if !wall.is_finite() || !active.is_finite() || active<=0. || wall<active {
            return Err(bad("depth baseline timing"));
        }
        eval_overhead+=3.*(wall-active);
        generation_per_call=generation_per_call.max(active/16.);
    }
    let evaluation=eval_overhead+1808.*generation_per_call*1.25;
    let projected=used+training+evaluation;
    let continue_run=used+training<TRAIN_CUTOFF && projected<=MAX_SECONDS;
    Ok(binary::record!({
        "policy":digest(p)?,"actual_used_seconds":used,"training_remaining_conservative_seconds":training,
        "evaluation_remaining_conservative_seconds":evaluation,"projected_total_seconds":projected,
        "per_arm":details,"continue":continue_run,
        "status":if continue_run{"TRAIN_CONTINUE"}else{"COST_FAIL"}}))
}
fn execution_forecast_32(p:&Plan)->Result<binary::Value> {
    // The forecast is an immutable decision about the common 32-update prefix.
    // Later runs must neither invalidate it nor change its measured inputs.
    let _=execution_usage(p)?;
    let preparation:binary::Value=read_confirmed(&p.root.join("preparation.r3b"))?;
    let check:binary::Value=read_confirmed(&p.root.join("execute-check.r3b"))?;
    let old:Plan=read_confirmed(&p.root.join("plan.r3b"))?;
    if check["policy"]!=digest(&old)? {return Err(bad("depth forecast check identity"));}
    let mut prep=preparation["seconds"].as_f64().ok_or_else(||bad("depth forecast preparation"))?
        +preflight_usage(&p.root)?;
    if p.root.join("repair-seal.r3b").exists() {
        if p.root.join("repair-successor.r3b").exists() {
            let s=repair_successor(p)?;
            let old=repair_seal_inner(p,true)?;
            prep+=old.additional_charge_seconds+JOURNAL_CLOSE_RESERVE
                +s.additional_first_seal_seconds+s.successor_outer_reserve;
        } else {prep+=repair_seal(p)?.additional_charge_seconds+JOURNAL_CLOSE_RESERVE;}
    }
    if p.root.join("preflight-link.r3b").exists() {
        prep+=parent_preflight_extra(&p.root,preparation["seconds"].as_f64()
            .ok_or_else(||bad("depth forecast body seconds"))?)?;
    }
    let (mut trained,mut evaluated,mut calls,mut tokens)=(0f64,0f64,0usize,0usize);
    let saved=p.root.join("cost-forecast-32.r3b");
    let prefix_limit=if saved.exists() {
        read_confirmed::<binary::Value>(&saved)?["prefix_commands"].as_u64()
            .ok_or_else(||bad("depth frozen forecast prefix"))? as usize
    } else {128};
    let mut prefix_hashes=Vec::new();
    for index in 0..prefix_limit {
        let path=p.root.join("commands").join(format!("command-{index:03}-returned.r3b"));
        if !path.exists() {
            if saved.exists() {return Err(bad("depth forecast prefix missing"));}
            break;
        }
        let value:binary::Value=read_confirmed(&path)?;
        let elapsed=value["seconds"].as_f64().ok_or_else(||bad("depth forecast command seconds"))?;
        let (expected,_,_)=command_identity(p,index)?;
        if value["policy"]!=expected || !elapsed.is_finite() || elapsed<0. {return Err(bad("depth forecast command"));}
        match value["bucket"].as_str().ok_or_else(||bad("depth forecast bucket"))? {
            "preparation"=>prep+=elapsed,"training"=>trained+=elapsed,
            "evaluation"=>evaluated+=elapsed,_=>return Err(bad("depth forecast bucket")),
        }
        prefix_hashes.push(file_hash(&path)?);
    }
    let prefix_digest=digest(&prefix_hashes)?;
    for arm in ARMS {
        let r:binary::Value=read_confirmed(&p.root.join(arm).join("eval-0-segment-000-returned.r3b"))?;
        calls+=r["calls"].as_u64().ok_or_else(||bad("depth forecast baseline calls"))? as usize;
        tokens+=r["tokens"].as_u64().ok_or_else(||bad("depth forecast baseline tokens"))? as usize;
    }
    let mut training_remaining=0f64;
    let mut details=BTreeMap::new();
    for arm in ARMS {
        let runs=history(p,arm)?;
        let prefix=runs.iter().take_while(|r|r.after.state.committed<=32).collect::<Vec<_>>();
        if prefix.last().is_none_or(|r|r.after.state.committed!=32) {
            return Err(bad("depth execution forecast requires both32"));
        }
        let mut warm=Vec::new();let mut steady=Vec::new();let mut fixed=0f64;
        for run in &prefix {
            let rows=binary::read_value_records(&run.trace)?;
            let mut update_elapsed=0f64;
            for row in rows {if let Some(commit)=row["commit"].as_u64() {
                let seconds=row["seconds"].as_f64().ok_or_else(||bad("depth execution update seconds"))?;
                if !seconds.is_finite()||seconds<0. {return Err(bad("depth execution update time"));}
                if commit<=20 {warm.push(seconds);}else{steady.push(seconds);}
                update_elapsed+=seconds;
            }}
            fixed+=(run.seconds-update_elapsed).max(0.);
        }
        if warm.len()!=20||steady.len()!=12 {return Err(bad("depth execution 32 update timing"));}
        steady.sort_by(f64::total_cmp);
        let p50=steady[5];let p95=steady[11];
        let fixed_per_command=fixed/prefix.len() as f64;
        training_remaining+=480.*p95+2.*fixed_per_command;
        details.insert(arm,binary::record!({"warmup_updates":warm.len(),"steady_updates":steady.len(),
            "steady_p50":p50,"steady_p95":p95,"fixed_per_command":fixed_per_command}));
    }
    let mut baseline_observe=0f64;let mut baseline_wall=0f64;
    for arm in ARMS {
        verified_eval_complete(p,arm,0)?;
        let r:binary::Value=read_confirmed(&p.root.join(arm).join("eval-0-segment-000-returned.r3b"))?;
        baseline_observe+=r["timing"]["generation_observe_seconds"].as_f64().ok_or_else(||bad("depth baseline observe"))?;
        baseline_wall+=r["seconds"].as_f64().ok_or_else(||bad("depth baseline wall"))?;
    }
    if baseline_wall<baseline_observe||tokens==0||calls!=16 {return Err(bad("depth baseline timing/calls"));}
    let study=VerifiedStudy::load(p)?;
    let panels=study.panels(512,true)?;
    let mut old_final_tokens=0usize;
    let mut final_input_tokens=0usize;
    let mut final_max_new=0usize;
    let mut auxiliary_tokens=0usize;
    for panel in &panels {
        let old=binary::read_value_records(&p.closed_root.join("C").join(format!("eval-512-{}.r3rows",panel.0)))?;
        if old.len()!=panel.1.len()+1 {return Err(bad("depth old final raw length"));}
        for (episode,row) in panel.1.iter().zip(old.iter().skip(1)) {
            if row["id"]!=episode.id {return Err(bad("depth old final raw case"));}
            old_final_tokens+=row["raw_tokens"].as_array().ok_or_else(||bad("depth old final tokens"))?.len();
            final_input_tokens+=study.tokenizer.prepare_with_framing(&episode.request,
                Framing::QuestionEvidence,2048,"depth-forecast")?.token_ids.len();
            final_max_new+=episode.request.limits.max_tokens as usize;
        }
    }
    for name in ["word","renamed","train192"] {
        let old=binary::read_value_records(&p.closed_root.join("C").join(format!("eval-512-{name}.r3rows")))?;
        let n=if name=="train192" {8}else{4};
        for row in old.iter().skip(1).take(n) {
            auxiliary_tokens+=row["raw_tokens"].as_array().ok_or_else(||bad("depth sentinel length"))?.len()*2;
        }
        if name=="word" {for row in old.iter().skip(1).take(8) {
            auxiliary_tokens+=row["raw_tokens"].as_array().ok_or_else(||bad("depth B length"))?.len()*2;
        }}
    }
    let observe_per_token=baseline_observe/tokens as f64;
    let fixed_per_command=(baseline_wall-baseline_observe)/2.;
    // The old parent outputs supply length evidence, not a claim about the new models' answers.
    // Parent lengths are evidence, not a promise that the new models stop there.
    // Reserve command overhead for sentinel/final/B and explicit recount/finalization.
    let eval_estimate=observe_per_token*((old_final_tokens*2+auxiliary_tokens) as f64)*1.25
        +fixed_per_command*6.+120.;
    let projected_train=trained+training_remaining;
    let projected_eval=evaluated+eval_estimate;
    let limits=execution_limits(p);
    let continue_run=projected_train<=limits.training && projected_eval<=limits.evaluation
        && prep+projected_train+projected_eval<=limits.total;
    Ok(binary::record!({"contract":EXECUTION_CONTRACT,"policy":digest(p)?,
        "prefix_commands":prefix_hashes.len(),"prefix_digest":prefix_digest,
        "actual_preparation_seconds":prep,"actual_training_seconds":trained,
        "actual_evaluation_seconds":evaluated,"remaining_training_estimate":training_remaining,
        "remaining_evaluation_estimate":eval_estimate,"projected_training_seconds":projected_train,
        "projected_evaluation_seconds":projected_eval,"old_parent_final_tokens_per_arm":old_final_tokens,
        "final_prompt_tokens_per_arm":final_input_tokens,"final_max_new_per_arm":final_max_new,
        "baseline_generation_observe_seconds":baseline_observe,"baseline_command_wall_seconds":baseline_wall,
        "baseline_generated_tokens":tokens,"generation_observe_seconds_per_token":observe_per_token,
        "auxiliary_parent_tokens":auxiliary_tokens,"fixed_per_command_seconds":fixed_per_command,
        "recount_finalization_reserve_seconds":120.,
        "per_arm_training":details,"continue":continue_run,
        "status":if continue_run{"TRAIN_CONTINUE"}else{"EXECUTION_COST_BLOCKED"}}))
}
fn cost_forecast_32(p:&Plan)->Result<()> {
    let value=forecast_value_32(p)?;
    if execution(p) {
        println!("DEPTH_FORECAST32 actual_prep={} actual_training={} projected_training={} projected_eval={} continue={}",
            value["actual_preparation_seconds"],value["actual_training_seconds"],
            value["projected_training_seconds"],value["projected_evaluation_seconds"],value["continue"]);
    } else {
        println!("DEPTH_FORECAST32 used={} train_remaining={} eval_remaining={} projected={} continue={}",
            value["actual_used_seconds"],value["training_remaining_conservative_seconds"],
            value["evaluation_remaining_conservative_seconds"],value["projected_total_seconds"],value["continue"]);
    }
    publish_confirmed(&p.root.join("cost-forecast-32.r3b"),&value)?;
    Ok(())
}
fn sentinel_value_128(p:&Plan)->Result<binary::Value> {
    if execution(p) {
        let study=VerifiedStudy::load(p)?;
        let panels=study.panels(128,true)?;
        let mut parent_full=0usize;
        for name in ["word","renamed"] {
            let panel=study.all.iter().find(|v|v.0==name).ok_or_else(||bad("depth sentinel parent panel"))?;
            let original=binary::read_value_records(&p.closed_root.join("C").join(format!("eval-512-{name}.r3rows")))?;
            let prefix=(name.into(),panel.1[..4].to_vec(),panel.2[..4].to_vec());
            let value=panel_score(&prefix,&original[1..5],&study.tokenizer,&p.parent.content,"parent-prefix")?;
            parent_full+=joint_count(&value,"full")?;
        }
        let mut collapse=false;let mut values=BTreeMap::new();
        for arm in ARMS {
            let ep=endpoint_at(p,arm,128)?;
            verified_eval_complete_with(p,arm,128,&panels,&ep)?;
            let mut word_renamed=0usize;
            for panel in &panels {
                let score=score_verified(p,&study,&ep,arm,128,panel)?;
                if panel.0=="word"||panel.0=="renamed" {word_renamed+=joint_count(&score,"full")?;}
            }
            collapse|=word_renamed*2<=parent_full && parent_full>=word_renamed+4;
            values.insert(arm,word_renamed);
        }
        return Ok(binary::record!({"policy":digest(p)?,"parent_full8":parent_full,
            "combined_full8":values,"QUALITY_COLLAPSE_STOP":collapse,"new_model_calls":0}));
    }
    for arm in ARMS {
        verified_eval_complete(p,arm,128)?;
        for panel in panels(p,128)? {score(p,arm,128,&panel.0)?;}
    }
    let (s,comparison)=parent(p)?;
    let (_,_,_,tok,_)=inputs(&comparison)?;
    let original=completion::repaired_c_parent(&p.closed_root,&p.correction)?.1;
    let all=eval_panels(&s,&comparison)?;
    let mut parent_full=0;
    for name in ["word","renamed"] {
        let panel=all.iter().find(|v|v.0==name).ok_or_else(||bad("depth sentinel panel"))?;
        let raw=binary::read_value_records(&p.closed_root.join("C").join(format!("eval-512-{name}.r3rows")))?;
        let prefix=(name.into(),panel.1[..16].to_vec(),panel.2[..16].to_vec());
        let v=panel_score(&prefix,&raw[1..17],&tok,&original.content,"parent-prefix")?;
        parent_full+=joint_count(&v,"full")?;
    }
    let mut collapse=false;let mut values=BTreeMap::new();
    for arm in ARMS {
        let full=panel_full(p,arm,128,"word")?+panel_full(p,arm,128,"renamed")?;
        collapse|=full*2<=parent_full && parent_full>=full+8;
        values.insert(arm,full);
    }
    Ok(binary::record!({"policy":digest(p)?,"parent_full32":parent_full,
        "combined_full32":values,"QUALITY_COLLAPSE_STOP":collapse,"new_model_calls":0}))
}
fn final_screen_scores(p:&Plan)->Result<(BTreeMap<String,binary::Value>,bool)> {
    if execution(p) {
        let study=VerifiedStudy::load(p)?;
        let panels=study.panels(512,true)?;
        let d6=endpoint_at(p,"D6",512)?;
        let d8=endpoint_at(p,"D8",512)?;
        verified_eval_complete_with(p,"D6",512,&panels,&d6)?;
        verified_eval_complete_with(p,"D8",512,&panels,&d8)?;
        let mut table=BTreeMap::new();let mut signal=true;
        for panel in &panels {
            let name=panel.0.as_str();
            let a=score_verified(p,&study,&d6,"D6",512,panel)?;
            let b=score_verified(p,&study,&d8,"D8",512,panel)?;
            let af=if name.starts_with("qa-"){score_count(&a,"full")?}else{joint_count(&a,"full")?};
            let bf=if name.starts_with("qa-"){score_count(&b,"full")?}else{joint_count(&b,"full")?};
            let paired=if ["word","renamed"].contains(&name) {
                let (gain,loss)=paired_counts(&a["joint"],&b["joint"],"exact")?;
                let (aa,ba)=(joint_count(&a,"all4")?,joint_count(&b,"all4")?);
                let (av,bv)=(score_count(&a,"value_correct")?,score_count(&b,"value_correct")?);
                let (asup,bsup)=(score_count(&a,"citation_support_correct")?,score_count(&b,"citation_support_correct")?);
                let (ao,bo)=(score_count(&a,"valid_outside_id")?,score_count(&b,"valid_outside_id")?);
                let malformed=score_count(&b,"parse_failure_rows")?;
                signal&=bf>=af+20&&ba>=aa+4&&bv>=av&&bsup>=asup&&bo<=ao&&malformed==0;
                binary::record!({"gain":gain,"loss":loss,"D6_all4":aa,"D8_all4":ba,
                    "D6_value":av,"D8_value":bv,"D6_support":asup,"D8_support":bsup,
                    "D6_outside":ao,"D8_outside":bo,"D8_malformed":malformed})
            } else {signal&=if name=="train192"{bf>=af}else{bf+1>=af};binary::record!(null)};
            if ["citation","S1Q1","qa-old_qa-primary64"].contains(&name) {
                signal&=invalid_citation_rows(name,&b)?<=invalid_citation_rows(name,&a)?;
            }
            table.insert(name.to_string(),binary::record!({"D6_full":af,"D8_full":bf,"paired":paired,
                "D6_score_hash":file_hash(&p.root.join("D6").join(format!("eval-512-{name}-score.r3b")))?,
                "D8_score_hash":file_hash(&p.root.join("D8").join(format!("eval-512-{name}-score.r3b")))?}));
        }
        return Ok((table,signal));
    }
    for arm in ARMS {verified_eval_complete(p,arm,512)?;}
    let mut table=BTreeMap::new();let mut signal=true;
    for name in ["word","renamed","train192","value","citation","S1Q1","qa-old_qa-primary64"] {
        let (a,b)=(score(p,"D6",512,name)?,score(p,"D8",512,name)?);
        let af=if name.starts_with("qa-"){score_count(&a,"full")?}else{joint_count(&a,"full")?};
        let bf=if name.starts_with("qa-"){score_count(&b,"full")?}else{joint_count(&b,"full")?};
        let paired=if ["word","renamed"].contains(&name) {
            let (gain,loss)=paired_counts(&a["joint"],&b["joint"],"exact")?;
            let (aa,ba)=(joint_count(&a,"all4")?,joint_count(&b,"all4")?);
            let (av,bv)=(score_count(&a,"value_correct")?,score_count(&b,"value_correct")?);
            let (asup,bsup)=(score_count(&a,"citation_support_correct")?,score_count(&b,"citation_support_correct")?);
            let (ao,bo)=(score_count(&a,"valid_outside_id")?,score_count(&b,"valid_outside_id")?);
            let malformed=score_count(&b,"parse_failure_rows")?;
            signal&=bf>=af+20 && ba>=aa+4 && bv>=av && bsup>=asup && bo<=ao && malformed==0;
            binary::record!({"gain":gain,"loss":loss,"D6_all4":aa,"D8_all4":ba,
                "D6_value":av,"D8_value":bv,"D6_support":asup,"D8_support":bsup,
                "D6_outside":ao,"D8_outside":bo,"D8_malformed":malformed})
        } else {signal&=if name=="train192" {bf>=af}else{bf+1>=af};binary::record!(null)};
        if ["citation","S1Q1","qa-old_qa-primary64"].contains(&name) {
            signal&=invalid_citation_rows(name,&b)?<=invalid_citation_rows(name,&a)?;
        }
        table.insert(name.to_string(),binary::record!({"D6_full":af,"D8_full":bf,
            "paired":paired,"D6_score_hash":file_hash(&p.root.join("D6").join(format!("eval-512-{name}-score.r3b")))?,
            "D8_score_hash":file_hash(&p.root.join("D8").join(format!("eval-512-{name}-score.r3b")))?}));
    }
    Ok((table,signal))
}
fn screen(root: &Path) -> Result<()> {
    let p=checked(root)?;
    if ARMS.iter().all(|arm|p.root.join(arm).join("eval-0-complete.r3b").exists())
        && !p.root.join("screen-0.r3b").exists() {
        let value=initial_screen_value(&p)?;
        println!("DEPTH_SCREEN0 pair_and_parent_matched={} calls={} optimizer=0",value["matched_cases_per_arm"],value["generation_calls"]);
        publish_confirmed(&p.root.join("screen-0.r3b"),&value)?;
        return Ok(());
    }
    admitted(&p)?;
    if ARMS.iter().all(|arm|endpoint(&p,arm).is_ok_and(|e|e.state.committed==32))
        && !p.root.join("cost-forecast-32.r3b").exists() {
        return cost_forecast_32(&p);
    }
    if ARMS.iter().all(|arm|p.root.join(arm).join("eval-128-complete.r3b").exists())
        && !p.root.join("screen-128.r3b").exists() {
        let value=sentinel_value_128(&p)?;
        println!("DEPTH_SCREEN128 parent_full={} collapse={}",
            if execution(&p){&value["parent_full8"]}else{&value["parent_full32"]},value["QUALITY_COLLAPSE_STOP"]);
        publish_confirmed(&p.root.join("screen-128.r3b"),&value)?;
        return Ok(());
    }
    let (table,signal)=final_screen_scores(&p)?;
    let (seconds,calls,tokens)=usage(&p)?;
    publish_confirmed(&p.root.join("screen-512.r3b"),&binary::record!({
        "policy":digest(&p)?,"scores":table,"depth_followup_signal_before_B":signal,
        "model_seconds":seconds,"generation_calls":calls,"generation_tokens":tokens,
        "MODEL_QUALITY_ACCEPTED":false,"GOAL1_ACCEPTED":false}))?;
    println!("DEPTH_SCREEN512 signal_before_B={signal} seconds={seconds:.3} calls={calls} tokens={tokens}");Ok(())
}
fn admit_b(root: &Path, review: &Path) -> Result<()> {
    let p=checked(root)?;admitted(&p)?;
    let screen:binary::Value=read_confirmed(&p.root.join("screen-512.r3b"))?;
    let (table,signal)=final_screen_scores(&p)?;
    if screen["scores"]!=binary::record!(table) || screen["depth_followup_signal_before_B"]!=signal {
        return Err(bad("depth screen recount before B"));
    }
    let r:binary::Value=read(review)?;
    let repaired=execution(&p)&&p.root.join("repair-amendment.r3b").exists();
    let contract=if repaired {ACCOUNTING_CONTRACT}else if execution(&p){EXECUTION_CONTRACT}else{CONTRACT};
    if r["contract"]!=contract || r["plan_hash"]!=file_hash(&p.root.join("plan.r3b"))?
        || r["source"]!=p.source || r["binary"]!=p.runtime.binary
        || r["verdict"]!="PASS" || r["raw_recount_rows"]!=1664
        || r["reproduction_rows"]!=16 || r["scores"]!=screen["scores"]
        || r["optimizer_calls"]!=0 || r["teacher_calls"]!=0
        || (execution(&p) && r["amendment"]!=file_hash(&amendment_path(&p))?)
        || (repaired && (r["repair_amendment"]!=file_hash(&p.root.join("repair-amendment.r3b"))?
            || r["repair_seal"]!=file_hash(&p.root.join("repair-seal.r3b"))?)) {
        return Err(bad("depth independent B identity/full recount"));
    }
    publish_confirmed(&p.root.join("review-b.r3b"),&binary::record!({
        "path":review.canonicalize()?,"hash":file_hash(review)?,"plan":file_hash(&p.root.join("plan.r3b"))?}))
}
fn verified_final(p: &Plan) -> Result<()> {
    if execution(p) {
        let study=VerifiedStudy::load(p)?;
        for arm in ARMS {
            let ep=endpoint(p,arm)?;
            if ep.state.committed!=512||ep.state.adam_clock!=512||file_hash(&ep.path)?!=ep.hash {
                return Err(bad("depth final native"));
            }
            for step in [512,998] {
                let panels=study.panels(step,true)?;
                verified_eval_complete_with(p,arm,step,&panels,&ep)?;
                for panel in &panels {score_verified(p,&study,&ep,arm,step,panel)?;}
            }
        }
        return Ok(());
    }
    let (_, comparison)=parent(p)?;
    let (_,_,_,tok,_)=inputs(&comparison)?;
    for arm in ARMS {
        let ep=endpoint(p,arm)?;
        if ep.state.committed!=512 || ep.state.adam_clock!=512 || ep.hash!=file_hash(&ep.path)? {
            return Err(bad("depth final native/evaluation marker"));
        }
        for step in [512,998] {
            verified_eval_complete(p,arm,step)?;
            for panel in panels(p,step)? {
                let label=format!("eval-{step}-{}",panel.0);
                let dir=p.root.join(arm);
                let raw=dir.join(format!("{label}.r3rows"));
                let rows=binary::read_value_records(&raw)?;
                let source_raw=if step==998 {Some(file_hash(&dir.join("eval-512-word.r3rows"))?)}else{None};
                let binding=binary::record!({"policy":policy(p,arm)?,"arm":arm,"native":ep.hash,
                    "model":ep.content,"step":step,"cases":digest(&panel.1)?,"metadata":digest(&panel.2)?,
                    "tokenizer":p.tokenizer_id,"source_raw":source_raw,"accuracy_denominator":step!=998});
                if rows.len()!=panel.1.len()+1 || rows[0]!=binding {return Err(bad("depth final raw binding/count"));}
                for (i,row) in rows.iter().skip(1).enumerate() {
                    call_attempt(&dir,&label,"generation",&binding,&panel.1[i],i,Some(row))?;
                    verify_generated(row,&tok)?;
                    if row["id"]!=panel.1[i].id || row["expected"]!=panel.1[i].answer {
                        return Err(bad("depth final row case"));
                    }
                    if step==998 {
                        let original=binary::read_value_records(&dir.join("eval-512-word.r3rows"))?;
                        for field in ["raw_tokens","raw_bytes","actual","finish_reason","error","generation_completed"] {
                            if row[field]!=original[i+1][field] {return Err(bad("depth final B raw parity"));}
                        }
                    }
                }
                let mut expected=panel_score(&panel,&rows[1..],&tok,&ep.content,&file_hash(&raw)?)?;
                expected["policy"]=binary::record!(policy(p,arm)?);
                expected["model"]=binary::record!(ep.content);
                expected["raw_hash"]=binary::record!(file_hash(&raw)?);
                if expected!=score(p,arm,step,&panel.0)? {return Err(bad("depth final score mismatch"));}
            }
        }
    }
    Ok(())
}
fn manifest_digest_from_paths(predecessor:&str,paths:&[PathBuf])->Result<String> {
    let files=paths.iter().map(|path|Ok((path.clone(),file_hash(path)?)))
        .collect::<Result<Vec<_>>>()?;
    digest(&(ACCOUNTING_CONTRACT,predecessor,files))
}
fn final_manifest_hash(p:&Plan,predecessor:&str)->Result<String> {
    let mut files=Vec::new();
    let mut add=|path:PathBuf|->Result<()> {files.push(path);Ok(())};
    for name in ["plan.r3b","preparation.r3b","review-a.r3b","screen-0.r3b",
        "cost-forecast-32.r3b","screen-128.r3b","screen-512.r3b","review-b.r3b"] {
        add(p.root.join(name))?;
    }
    if execution(p) {add(p.root.join("repair-amendment.r3b"))?;
        add(p.root.join("repair-seal.r3b"))?;
        if p.root.join("repair-successor.r3b").exists() {add(p.root.join("repair-successor.r3b"))?;}
        add(amendment_path(p))?;}
    for arm in ARMS {
        let dir=p.root.join(arm);
        add(dir.join("initial.r3b"))?;
        let ep=endpoint(p,arm)?;
        add(ep.path.clone())?;
        for step in [0,128,512,998] {add(dir.join(format!("eval-{step}-complete.r3b")))?;}
        for step in [512,998] {
            for panel in panels(p,step)? {
                let label=format!("eval-{step}-{}",panel.0);
                add(dir.join(format!("{label}.r3rows")))?;
                add(dir.join(format!("{label}-score.r3b")))?;
            }
        }
    }
    manifest_digest_from_paths(predecessor,&files)
}
fn verify_final_pair(payload:&Path,receipt:&Path,manifest:&str,attempt:&str,predecessor:&str)->Result<()> {
    let body:binary::Value=read_confirmed(payload)?;
    let done:binary::Value=read_confirmed(receipt)?;
    if body["status"]!="FINALIZATION_PENDING" || body["resource_status"]!="RESOURCE_PENDING"
        || body["manifest_hash"]!=manifest || body["attempt"]!=attempt
        || done["payload_hash"]!=file_hash(payload)? || done["manifest_hash"]!=manifest
        || done["attempt"]!=attempt || done["predecessor"]!=predecessor
        || done["result"]!="FINALIZED" {
        return Err(bad("depth final payload/closure pair"));
    }
    Ok(())
}
fn read_final(root:&Path)->Result<()> {
    let p=checked(root)?;admitted(&p)?;verified_final(&p)?;
    let body:binary::Value=read_confirmed(&p.root.join("final-report.r3b"))?;
    let mut found=None;
    for index in 0..128 {
        let path=p.root.join("commands").join(format!("command-{index:03}-entered.r3b"));
        if !path.exists(){break;}
        let entered:binary::Value=read_confirmed(&path)?;
        if entered["kind"]=="finish" {
            if found.is_some(){return Err(bad("depth duplicate final attempt"));}
            found=Some(index);
        }
    }
    let index=found.ok_or_else(||bad("depth final command absent"))?;
    let entered:binary::Value=read_confirmed(&p.root.join("commands").join(format!("command-{index:03}-entered.r3b")))?;
    let done:binary::Value=read_confirmed(&p.root.join("commands").join(format!("command-{index:03}-returned.r3b")))?;
    let predecessor=command_predecessor(&p.root,index)?;
    let manifest=final_manifest_hash(&p,&predecessor)?;
    let (table,recounted)=final_screen_scores(&p)?;
    let screen:binary::Value=read_confirmed(&p.root.join("screen-512.r3b"))?;
    let link:binary::Value=read_confirmed(&p.root.join("review-b.r3b"))?;
    let report_path=Path::new(link["path"].as_str().ok_or_else(||bad("depth final B path"))?);
    let b:binary::Value=read(report_path)?;
    if link["hash"]!=file_hash(report_path)? || b["verdict"]!="PASS" || b["scores"]!=screen["scores"]
        || screen["scores"]!=binary::record!(table) || screen["depth_followup_signal_before_B"]!=recounted
        || body["scores"]!=screen["scores"] || body["independent_B"]!=link["hash"]
        || body["status"]!="FINALIZATION_PENDING" || body["resource_status"]!="RESOURCE_PENDING"
        || body["manifest_hash"]!=manifest || body["attempt"]!=entered["attempt"]
        || entered["predecessor"]!=predecessor || done["predecessor"]!=predecessor
        || done["attempt"]!=entered["attempt"] {
        return Err(bad("depth final closure identity/resource"));
    }
    let (prep,training,evaluation,calls,tokens)=execution_usage(&p)?;
    let pol=digest(&p)?;
    verify_final_closure(&p.root.join("final-report.r3b"),
        &p.root.join("commands").join(format!("command-{index:03}-returned.r3b")),
        FinalIdentity{policy:&pol,index,predecessor:&predecessor,
            attempt:entered["attempt"].as_str().ok_or_else(||bad("depth final attempt"))?,
            source:&p.source,binary:&p.runtime.binary,manifest:&manifest},
        [prep,training,evaluation],calls,tokens,execution_limits(&p))?;
    println!("DEPTH_FINAL_READ status={} measured_scope=PASS whole_process=UNKNOWN prep={prep:.3} train={training:.3} eval={evaluation:.3} calls={calls} tokens={tokens}",body["candidate_status"]);
    Ok(())
}
fn finish(root: &Path) -> Result<()> {
    let p=checked(root)?;admitted(&p)?;
    resource_guard(&p,2*1024*1024)?;
    let lock=std::fs::OpenOptions::new().create(true).read(true).write(true).open(p.root.join("writer.lock"))?;
    lock.try_lock().map_err(|_|bad("depth final writer active"))?;
    verified_final(&p)?;
    let link:binary::Value=read_confirmed(&p.root.join("review-b.r3b"))?;
    let path=Path::new(link["path"].as_str().ok_or_else(||bad("depth B path"))?);
    let b:binary::Value=read(path)?;
    let screen:binary::Value=read_confirmed(&p.root.join("screen-512.r3b"))?;
    let (table,recounted_signal)=final_screen_scores(&p)?;
    if screen["policy"]!=digest(&p)? || screen["scores"]!=binary::record!(table)
        || screen["depth_followup_signal_before_B"]!=recounted_signal
        || screen["MODEL_QUALITY_ACCEPTED"]!=false || screen["GOAL1_ACCEPTED"]!=false {
        return Err(bad("depth final screen recount"));
    }
    if link["hash"]!=file_hash(path)? || link["plan"]!=file_hash(&p.root.join("plan.r3b"))?
        || b["verdict"]!="PASS" || b["scores"]!=screen["scores"]
        || b["source"]!=p.source || b["binary"]!=p.runtime.binary
        || (execution(&p) && b["amendment"]!=file_hash(&amendment_path(&p))?)
        || (execution(&p) && p.root.join("repair-amendment.r3b").exists()
            && (b["repair_amendment"]!=file_hash(&p.root.join("repair-amendment.r3b"))?
                || b["repair_seal"]!=file_hash(&p.root.join("repair-seal.r3b"))?))
        || b["raw_recount_rows"]!=1664 || b["reproduction_rows"]!=16 {
        return Err(bad("depth final independent B"));
    }
    let (seconds,calls,tokens)=usage(&p)?;
    let total_cap=if execution(&p){execution_limits(&p).total}else{MAX_SECONDS};
    let signal=recounted_signal && seconds<=total_cap
        && calls<=if execution(&p){1760}else{1840} && tokens<=262144;
    let word=screen["scores"]["word"].clone();
    let renamed=screen["scores"]["renamed"].clone();
    let fit=screen["scores"]["train192"].clone();
    let direction=if signal {"DEPTH_FOLLOWUP_SIGNAL_ONLY"}
        else if fit["D8_full"].as_u64()>fit["D6_full"].as_u64()
            && word["D8_full"].as_u64()<=word["D6_full"].as_u64()
            && renamed["D8_full"].as_u64()<=renamed["D6_full"].as_u64() {"FIT_ONLY_NO_TRANSFER_SIGNAL"}
        else if (word["D8_full"].as_u64()>word["D6_full"].as_u64())
            != (renamed["D8_full"].as_u64()>renamed["D6_full"].as_u64()) {"MIXED_NO_ADOPTION"}
        else {"COMPLETE_NO_CLEAR_DEPTH_SIGNAL"};
    resource_guard(&p,2*1024*1024)?;
    let index=ACTIVE_COMMAND.load(Ordering::Relaxed);
    if index==usize::MAX {return Err(bad("depth final requires tracked command"));}
    let predecessor=command_predecessor(&p.root,index)?;
    let entered:binary::Value=read_confirmed(&p.root.join("commands").join(format!("command-{index:03}-entered.r3b")))?;
    let manifest=final_manifest_hash(&p,&predecessor)?;
    publish_confirmed(&p.root.join("final-report.r3b"),&binary::record!({
        "contract":if execution(&p){EXECUTION_CONTRACT}else{CONTRACT},"policy":digest(&p)?,"parent_global":PARENT_GLOBAL,
        "local_optimizer":512,"logical_total":PARENT_GLOBAL+512,"status":"FINALIZATION_PENDING",
        "resource_status":"RESOURCE_PENDING","candidate_status":direction,
        "manifest_hash":manifest,"attempt":entered["attempt"],
        "depth_followup_signal":signal,"D8_ADOPTED":false,"scores":screen["scores"],
        "independent_B":file_hash(path)?,"repair":p.correction_hash,
        "model_seconds":seconds,"generation_calls":calls,"generation_tokens":tokens,
        "MODEL_QUALITY_ACCEPTED":false,"GOAL1_ACCEPTED":false}))?;
    resource_guard(&p,0)?;
    println!("DEPTH_FINAL_PENDING candidate={direction} signal={signal} prior_seconds={seconds:.3} calls={calls} tokens={tokens}");
    Ok(())
}
fn prepare(closed: &Path, correction: &Path, output: &Path, target_baseline_kib: u64) -> Result<()> {
    let command_started=Instant::now();
    if output.exists() { return Err(bad("depth output already exists")); }
    let target_now=(completion::allocated(Path::new("target/debug"))?+1023)/1024;
    if target_now>target_baseline_kib+1024*1024 {return Err(bad("depth target before prepare"));}
    let closed = closed.canonicalize()?;
    let correction = correction.canonicalize()?;
    let (s, ep) = completion::repaired_c_parent(&closed, &correction)?;
    let comparison: super::super::Study = read_confirmed(&s.parent_root.join("plan.r3b"))?;
    let (corpus, _, _, tok, samples) = inputs(&comparison)?;
    if s.tape != comparison.tape[..512] || comparison.tape.len() != 3072
        || corpus.train.len() != 7680 { return Err(bad("depth exact original cycle")); }
    let tape = comparison.tape[512..1024].to_vec();
    let (mut input, mut target) = (0u64, 0u64);
    for row in &tape { for &at in row { let sample = samples.get(at).ok_or_else(||bad("depth tape bound"))?;
        input += (sample.tokens.len() - 1) as u64;
        target += (sample.tokens.len() - sample.response_start) as u64; }}
    let panels = eval_panels(&s, &comparison)?;
    let device = Backend::Metal0.open()?;
    let runtime = RuntimeProfile::capture(Backend::Metal0, &device)?;
    let (loaded, _) = load_endpoint(&ep, device)?;
    let Core::Tr(parent_model) = loaded else { return Err(bad("depth parent core")); };
    let mut d6 = Transformer::from_tensors(parent_model.config.clone(), parent_model.base_tensors(), parent_model.device.clone())?;
    d6.bind_tokenizer(&tok.semantic_id())?;
    let d8 = Transformer::depth8_from_small(&parent_model, 20260926)?;
    if d6.weights_content_id()? != ep.content || d8.config.parameters() - d6.config.parameters() != 3_099_072 {
        return Err(bad("depth parent copy/parameter delta"));
    }
    std::fs::create_dir(output)?;
    let root = output.canonicalize()?;
    let p = Plan { contract: CONTRACT.into(), root: root.clone(), source: source()?, runtime,
        closed_root: closed, correction_hash: file_hash(&correction)?, correction,
        comparison_root: s.parent_root.clone(), comparison_plan_hash: file_hash(&s.parent_root.join("plan.r3b"))?,
        parent: ep, parent_global: PARENT_GLOBAL, tokenizer: comparison.tokenizer.clone(),
        tokenizer_id: tok.semantic_id(), corpus: file_hash(&comparison.word_root.join("corpus.r3cor"))?,
        cores: [ComparisonCore::Trpp(d6.config.clone()), ComparisonCore::Trpp(d8.config.clone())],
        config: train_config(), tape, panels: digest(&panels)?, input_per_arm: input,
        target_per_arm: target, target_baseline_kib };
    publish_confirmed(&root.join("plan.r3b"), &p)?;
    for (arm, model) in [("D6", d6), ("D8", d8)] {
        let dir = root.join(arm);
        std::fs::create_dir(&dir)?;
        let mut core = Core::Tr(model);
        let adam = Adam::new(core.vars())?;
        let initial = save_endpoint(&dir, &mut core, &adam, state(&p, arm)?, "initial")?;
        publish_confirmed(&dir.join("initial.r3b"), &initial)?;
    }
    publish_confirmed(&root.join("preparation.r3b"), &binary::record!({
        "policy":digest(&p)?, "parent_global":PARENT_GLOBAL,"local_optimizer":0,
        "tape_start":512,"tape_end":1024,"input_per_arm":input,"target_per_arm":target,
        "source":p.source,"runtime":p.runtime,"R_ACCEPTED":true,"A_PENDING":true,
        "new_small_updates":0,"generation":0,"teacher":0,
        "model_seconds":command_started.elapsed().as_secs_f64()}))?;
    resource_guard(&p,0)?;
    println!("DEPTH_PREPARED parent_global=3584 local=0 tape=512..1024 input={input} target={target} parameters_D6={} parameters_D8={} model_calls=0",Config::small(tok.vocab_size()).parameters(),Config::depth8(tok.vocab_size()).parameters());
    Ok(())
}

pub fn run(action: Action) -> Result<()> {
    let tracked=match &action {
        Action::ExecuteCheck{root}=>Some((root.clone(),"execute-check","preparation")),
        Action::Inspect{root}=>Some((root.clone(),"inspect",if root.join("review-a.r3b").exists(){"evaluation"}else{"preparation"})),
        Action::Direct{root}=>Some((root.clone(),"direct","preparation")),
        Action::Admit{root,..}=>Some((root.clone(),"admit-a","preparation")),
        Action::Train{root,..}=>Some((root.clone(),"train","training")),
        Action::Evaluate{root,step,..}=>Some((root.clone(),"evaluate",if *step==0{"preparation"}else{"evaluation"})),
        Action::Screen{root}=>Some((root.clone(),"screen",if root.join("review-a.r3b").exists(){"evaluation"}else{"preparation"})),
        Action::Reproduce{root,..}=>Some((root.clone(),"reproduce","evaluation")),
        Action::AdmitB{root,..}=>Some((root.clone(),"admit-b","evaluation")),
        Action::Finish{root}=>Some((root.clone(),"finish","evaluation")),
        _=>None,
    };
    let execute=||match action {
        Action::RepairPrepare {root,legacy_observation,legacy_binary} =>
            repair_prepare(&root,&legacy_observation,&legacy_binary),
        Action::SealRepair {root} => seal_repair(&root),
        Action::SealSuccessor {root} => seal_successor(&root),
        Action::ReadFinal {root} => read_final(&root),
        Action::ExecutePrepare {root,target_baseline_kib,prior_failed_prep_seconds,prior_failed_binary} => {
            let request=digest(&(root.as_path(),target_baseline_kib,prior_failed_prep_seconds,prior_failed_binary.as_deref()))?;
            preflight_attempt(&root,"execute-prepare",&request,||
                execute_prepare(&root,target_baseline_kib,prior_failed_prep_seconds,prior_failed_binary.as_deref()))?;
            let parent=root.canonicalize()?;
            let child=parent.join("execution");
            publish_confirmed(&child.join("preflight-link.r3b"),&binary::record!({
                "parent":parent,"scope":child,"head":preflight_head(&parent)?}))
        },
        Action::ExecuteCheck {root} => execute_check(&root),
        Action::Prepare {closed, correction, output, target_baseline_kib} => prepare(&closed, &correction, &output, target_baseline_kib),
        Action::Inspect {root} => { let p = checked(&root)?;
            if execution(&p) {
                let (prep,training,evaluation,calls,tokens)=execution_usage(&p)?;
                let d6=endpoint(&p,"D6")?;let d8=endpoint(&p,"D8")?;
                let limits=execution_limits(&p);
                println!("DEPTH_EXECUTION_INSPECT policy={} A={} D6_local={} D8_local={} prep={prep:.3}/{} train={training:.3}/{} eval={evaluation:.3}/{} calls={calls}/1760 tokens={tokens}/262144 D6_native={} D8_native={}",
                    digest(&p)?,p.root.join("review-a.r3b").exists(),d6.state.committed,d8.state.committed,
                    limits.preparation,limits.training,limits.evaluation,d6.hash,d8.hash);
            } else {println!("DEPTH_INSPECT policy={} A={} parent=3584 local=0",digest(&p)?,p.root.join("review-a.r3b").exists());}
            Ok(()) },
        Action::Direct {root} => direct(&root),
        Action::Tiny {output,arm,until,resume} => tiny(&output,&arm,until,resume),
        Action::VerifyTiny {continuous,split} => verify_tiny(&continuous,&split),
        Action::Admit {root, review} => admit(&root, &review),
        Action::Train {root, arm, until} => train(&root, &arm, until),
        Action::Evaluate {root, arm, step} => evaluate(&root, &arm, step),
        Action::Screen {root} => screen(&root),
        Action::Reproduce {root, arm} => evaluate(&root, &arm, 998),
        Action::AdmitB {root, review} => admit_b(&root, &review),
        Action::Finish {root} => finish(&root),
    };
    if let Some((root,kind,bucket))=tracked {tracked_command(&root,kind,bucket,execute)}else{execute()}
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn final_cost_counts_body_once_and_reserves_close() {
        let limit=ExecutionLimits{preparation:1800.,training:2400.,evaluation:2100.,total:6300.};
        let before=[800.,1000.,2080.];
        assert_eq!(final_cost(limit,before,10.,10.).unwrap(),([800.,1000.,2100.],true));
        assert_eq!(final_cost(limit,before,10.001,10.).unwrap().1,false);
        assert!(final_cost(limit,before,f64::NAN,10.).is_err());
        assert!(final_cost(limit,before,-1.,10.).is_err());
        assert!(final_cost(limit,before,f64::MAX,f64::MAX).is_err());
    }
    #[test]
    fn final_manifest_rejects_current_raw_score_marker_and_review_changes() {
        let d=tempfile::tempdir().unwrap();
        let paths=["raw.r3rows","score.r3b","complete.r3b","review-b.r3b"]
            .map(|name|d.path().join(name));
        for (i,path) in paths.iter().enumerate() {std::fs::write(path,format!("original-{i}")).unwrap();}
        let original=manifest_digest_from_paths("predecessor",&paths).unwrap();
        let payload=d.path().join("final-report.r3b");
        let receipt=d.path().join("final-returned.r3b");
        publish_confirmed(&payload,&binary::record!({"status":"FINALIZATION_PENDING",
            "resource_status":"RESOURCE_PENDING","manifest_hash":original,"attempt":"attempt"})).unwrap();
        let id=FinalIdentity{policy:"policy",index:3,predecessor:"predecessor",
            attempt:"attempt",source:"source",binary:"binary",manifest:&original};
        let limit=ExecutionLimits{preparation:1800.,training:2400.,evaluation:2100.,total:6300.};
        emit_final_closure(&payload,&receipt,id,[20.,30.,40.],0,0,||3.,limit).unwrap();
        verify_final_pair(&payload,&receipt,&original,"attempt","predecessor").unwrap();
        for (i,path) in paths.iter().enumerate() {
            std::fs::write(path,format!("changed-{i}")).unwrap();
            let current=manifest_digest_from_paths("predecessor",&paths).unwrap();
            assert_ne!(current,original,"{i}: changed evidence kept manifest");
            assert!(verify_final_pair(&payload,&receipt,&current,"attempt","predecessor").is_err(),
                "{i}: changed evidence accepted");
            std::fs::remove_file(path).unwrap();
            assert!(manifest_digest_from_paths("predecessor",&paths).is_err(),
                "{i}: missing evidence accepted");
            std::fs::write(path,format!("original-{i}")).unwrap();
        }
    }
    #[test]
    fn repair_journal_missing_or_gapped_fails_closed() {
        let d=tempfile::tempdir().unwrap();
        let root=d.path().join("execution");
        std::fs::create_dir(&root).unwrap();
        std::fs::write(root.join("repair-amendment.r3b"),b"fixture").unwrap();
        assert!(preflight_usage(&root).is_err());
        std::fs::create_dir(root.join("attempts")).unwrap();
        assert!(preflight_usage(&root).is_err());
        std::fs::write(root.join("attempts/attempt-001-entered.r3b"),b"fixture").unwrap();
        assert!(preflight_usage(&root).is_err());
        let old=117.922353876f64;
        assert!((LEGACY_BOUNDED_SECONDS-old-12.319646124).abs()<1e-9);
    }
    #[test]
    fn successor_rejects_changed_preparation_and_uncovered_wall() {
        let d=tempfile::tempdir().unwrap();
        let prep=d.path().join("preparation.r3b");
        std::fs::write(&prep,b"original preparation receipt").unwrap();
        let bound=file_hash(&prep).unwrap();
        expected_file_hash(&prep,&bound).unwrap();
        std::fs::write(&prep,b"smaller preparation seconds").unwrap();
        assert!(expected_file_hash(&prep,&bound).is_err());
        let log=d.path().join("successor-seal.log");
        std::fs::write(&log,b"real 26.37\nuser 25.00\nsys 0.20\n").unwrap();
        assert_eq!(observed_process_wall(&log).unwrap(),26.37);
        assert!(outer_reserve_covers(26.37,11.701742583,30.));
        assert!(!outer_reserve_covers(41.702,11.701742583,30.));
        assert!(!outer_reserve_covers(f64::NAN,11.701742583,30.));
        std::fs::write(&log,b"real 26.37\nreal 1.00\n").unwrap();
        assert!(observed_process_wall(&log).is_err());
    }
    #[test]
    fn successor_does_not_reenter_after_partial_one_shot_attempt() {
        let d=tempfile::tempdir().unwrap();
        std::fs::create_dir(d.path().join("attempts")).unwrap();
        std::fs::write(d.path().join("attempts/attempt-002-entered.r3b"),b"partial").unwrap();
        assert!(seal_successor(d.path()).is_err());
        assert!(!d.path().join("attempts/attempt-003-entered.r3b").exists());
    }
    #[test]
    fn repair_seal_rejects_changed_or_incomplete_attempts() {
        for case in ["valid","missing","smaller_seconds","other_scope","forged_binary","duplicate_import"] {
            let d=tempfile::tempdir().unwrap();
            let root=d.path().join("execution");
            std::fs::create_dir(&root).unwrap();
            preflight_attempt(&root,"repair-prepare","first",||Ok(())).unwrap();
            preflight_attempt(&root,"seal-repair","second",||Ok(())).unwrap();
            let entered=root.join("attempts/attempt-000-entered.r3b");
            let returned=root.join("attempts/attempt-000-returned.r3b");
            let seal=root.join("attempts/attempt-001-returned.r3b");
            let first:binary::Value=read_confirmed(&entered).unwrap();
            let source=first["source"].as_str().unwrap().to_string();
            let binary=first["binary"].as_str().unwrap().to_string();
            let hashes=[file_hash(&entered).unwrap(),file_hash(&returned).unwrap(),file_hash(&seal).unwrap()];
            let verify=||verify_repair_attempts(&root,&source,&binary,&source,&binary,
                &hashes[0],&hashes[1],&hashes[2]);
            assert!(verify().is_ok(),"{case}: original seal fixture");
            match case {
                "valid"=>continue,
                "missing"=>{std::fs::remove_file(&returned).unwrap();},
                "smaller_seconds"=>{
                    let mut row:binary::Value=read_confirmed(&returned).unwrap();
                    row["seconds"]=binary::record!(0.);
                    std::fs::remove_file(&returned).unwrap();
                    publish_confirmed(&returned,&row).unwrap();
                },
                "other_scope"=>{
                    let mut row:binary::Value=read_confirmed(&entered).unwrap();
                    row["scope"]=binary::record!(d.path());
                    std::fs::remove_file(&entered).unwrap();
                    publish_confirmed(&entered,&row).unwrap();
                },
                "forged_binary"=>{
                    let mut row:binary::Value=read_confirmed(&returned).unwrap();
                    row["binary"]=binary::record!("forged");
                    std::fs::remove_file(&returned).unwrap();
                    publish_confirmed(&returned,&row).unwrap();
                },
                "duplicate_import"=>{
                    let mut row:binary::Value=read_confirmed(&seal).unwrap();
                    row["kind"]=binary::record!("repair-prepare");
                    std::fs::remove_file(&seal).unwrap();
                    publish_confirmed(&seal,&row).unwrap();
                },
                _=>unreachable!(),
            }
            assert!(verify().is_err(),"{case}: changed attempt admitted");
            if ["other_scope","forged_binary","duplicate_import"].contains(&case) {
                let current=[file_hash(&entered).unwrap(),file_hash(&returned).unwrap(),file_hash(&seal).unwrap()];
                assert!(verify_repair_attempts(&root,&source,&binary,&source,&binary,
                    &current[0],&current[1],&current[2]).is_err(),
                    "{case}: semantic mismatch admitted with refreshed hashes");
            }
            // A lower completed duration with a rewritten receipt is rejected by the
            // immutable seal hash; elapsed wall time has no independent semantic oracle.
        }
    }
    #[test]
    fn repair_preseal_evidence_when_requested() {
        let Ok(root)=std::env::var("R3_DEPTH8_PRESEAL_ROOT") else {return};
        let root=PathBuf::from(root).canonicalize().unwrap();
        assert!(!root.join("repair-seal.r3b").exists());
        let p:Plan=read_confirmed(&root.join("plan.r3b")).unwrap();
        let a=repair_amendment_inner(&p,true).unwrap();
        let seconds=preflight_usage(&root).unwrap();
        let (entered,returned)=legacy_command_hashes(&root).unwrap();
        assert_eq!((entered.len(),returned.len()),(6,6));
        let attempt:binary::Value=read_confirmed(&root.join("attempts/attempt-000-returned.r3b")).unwrap();
        assert_eq!(attempt["result"],"COMPLETED");
        assert_eq!(attempt["source"],a.source);
        assert_eq!(attempt["binary"],a.binary);
        assert!(seconds>0. && seconds.is_finite());
        println!("DEPTH_PRESEAL_AUDIT attempts=1 seconds={seconds:.6} old_commands=6");
    }
    #[test]
    fn depth8_process_case() {
        let Ok(mode)=std::env::var("R3_DEPTH8_PROCESS_CASE") else {return};
        let root=PathBuf::from(std::env::var("R3_DEPTH8_PROCESS_ROOT").unwrap());
        let payload=root.join("final-report.r3b");
        let receipt=root.join("final-returned.r3b");
        let id=FinalIdentity{policy:"policy",index:3,predecessor:"predecessor",
            attempt:"attempt",source:"source",binary:"binary",manifest:"manifest"};
        let limit=ExecutionLimits{preparation:1800.,training:2400.,evaluation:2100.,total:6300.};
        match mode.as_str() {
            "B_emit"=>{
                publish_confirmed(&payload,&binary::record!({"status":"FINALIZATION_PENDING",
                    "resource_status":"RESOURCE_PENDING","manifest_hash":"manifest","attempt":"attempt"})).unwrap();
                emit_final_closure(&payload,&receipt,id,[20.,30.,2098.],0,0,||3.,limit).unwrap();
                assert_eq!(read_confirmed::<binary::Value>(&receipt).unwrap()["result"],"RESOURCE_NOT_MET");
            },
            "B_read"=>{
                assert!(verify_final_closure(&payload,&receipt,id,[20.,30.,2111.],0,0,limit).is_err());
            },
            "C_write"=>{
                publish_confirmed(&payload,&binary::record!({"status":"FINALIZATION_PENDING",
                    "resource_status":"RESOURCE_PENDING","manifest_hash":"manifest","attempt":"attempt"})).unwrap();
                assert!(emit_final_closure(&payload,&root.join("missing/receipt.r3b"),id,
                    [20.,30.,40.],0,0,||3.,limit).is_err());
            },
            "C_read_pending"=>{
                assert!(verify_final_closure(&payload,&receipt,id,[20.,30.,53.],0,0,limit).is_err());
            },
            "C_success"=>{
                publish_confirmed(&payload,&binary::record!({"status":"FINALIZATION_PENDING",
                    "resource_status":"RESOURCE_PENDING","manifest_hash":"manifest","attempt":"attempt"})).unwrap();
                emit_final_closure(&payload,&receipt,id,[20.,30.,40.],0,0,||3.,limit).unwrap();
            },
            "C_read_complete"=>{
                verify_final_closure(&payload,&receipt,id,[20.,30.,53.],0,0,limit).unwrap();
            },
            "C_tamper"=>{
                std::fs::write(&payload,b"tampered").unwrap();
                assert!(verify_final_closure(&payload,&receipt,id,[20.,30.,53.],0,0,limit).is_err());
            },
            "E"=>{
                let output=root.join("output");
                std::fs::write(root.join("plan.r3b"),b"fixture-parent").unwrap();
                assert!(preflight_attempt(&root,"execute-prepare","request",||Err(bad("feature unavailable"))).is_err());
                assert!(!output.exists());
                let first=preflight_usage(&root).unwrap();
                preflight_attempt(&root,"execute-prepare","request",||{std::fs::create_dir(&output)?;Ok(())}).unwrap();
                assert!(output.is_dir());
                let parent=root.canonicalize().unwrap();
                let child=output.canonicalize().unwrap();
                publish_confirmed(&child.join("preflight-link.r3b"),&binary::record!({
                    "parent":parent,"scope":child,"head":preflight_head(&parent).unwrap()})).unwrap();
                let total=preflight_usage(&root).unwrap();
                assert!(total>=first);
                let last:binary::Value=read_confirmed(&parent.join("attempts/attempt-001-returned.r3b")).unwrap();
                let successful=last["seconds"].as_f64().unwrap();
                let body=successful/2.;
                let extra=parent_preflight_extra(&child,body).unwrap();
                assert!((extra-(total-successful+successful-body)).abs()<1e-6);
                assert!(extra<total);
                assert!(parent_preflight_extra(&child,successful+1.).is_err());
                assert_eq!(read_confirmed::<binary::Value>(&root.join("attempts/attempt-000-returned.r3b")).unwrap()["result"],"RETURNED_ERROR");
            },
            _=>panic!("unknown depth8 process case"),
        }
    }
    #[test]
    fn depth8_fresh_process_boundaries() {
        let run=|root:&Path,mode:&str| {
            let output=std::process::Command::new(std::env::current_exe().unwrap())
                .arg("--exact").arg("training::fresh::core_comparison::binding_recovery::depth8::tests::depth8_process_case")
                .arg("--nocapture").env("R3_DEPTH8_PROCESS_CASE",mode)
                .env("R3_DEPTH8_PROCESS_ROOT",root).output().unwrap();
            assert!(output.status.success(),"{mode}: {}",String::from_utf8_lossy(&output.stderr));
            assert!(String::from_utf8_lossy(&output.stdout).contains("1 passed"),"{mode}: {}",String::from_utf8_lossy(&output.stdout));
        };
        for modes in [&["B_emit","B_read"][..],&["C_write","C_read_pending"][..],
            &["C_success","C_read_complete","C_tamper"][..],&["E"][..]] {
            let dir=tempfile::tempdir().unwrap();
            for mode in modes {run(dir.path(),mode);}
        }
    }
    #[test]
    fn sentinel_qa_uses_two_per_bucket_in_manifest_order() {
        let ms=(0..8).flat_map(|bucket|(0..8).map(move |view| Meta{
            id:format!("{bucket}-{view}"),base:String::new(),template:String::new(),
            bucket,view,split:String::new(),entities:Vec::new(),source_id:None,query_context:None,
        })).collect::<Vec<_>>();
        let chosen=sentinel_qa_indices(&ms).unwrap();
        assert_eq!(chosen,vec![0,1,8,9,16,17,24,25,32,33,40,41,48,49,56,57]);
        assert!(sentinel_qa_indices(&ms[..48]).is_err());
    }
    #[test]
    fn returned_evaluation_segment_is_reusable_but_unknown_is_not() {
        let d=tempfile::tempdir().unwrap();
        assert_eq!(next_eval_segment(d.path(),128).unwrap(),0);
        let entered=d.path().join("eval-128-segment-000-entered.r3b");
        let returned=d.path().join("eval-128-segment-000-returned.r3b");
        std::fs::write(&entered,b"entry").unwrap();
        assert!(next_eval_segment(d.path(),128).is_err());
        std::fs::write(&returned,b"returned").unwrap();
        assert_eq!(next_eval_segment(d.path(),128).unwrap(),1);
        std::fs::remove_file(entered).unwrap();
        assert!(next_eval_segment(d.path(),128).is_err());
    }
    #[test]
    fn resumed_panel_score_reuses_exact_record_and_rejects_changed_result() {
        let d=tempfile::tempdir().unwrap();
        let path=d.path().join("score.r3b");
        let expected=binary::record!({"full":7,"raw_hash":"fixed-raw"});
        publish_or_compare_score(&path,&expected).unwrap();
        let before=file_hash(&path).unwrap();
        publish_or_compare_score(&path,&expected).unwrap();
        assert_eq!(before,file_hash(&path).unwrap());
        let changed=binary::record!({"full":8,"raw_hash":"fixed-raw"});
        assert!(publish_or_compare_score(&path,&changed).is_err());
        assert_eq!(before,file_hash(&path).unwrap());
    }
}
