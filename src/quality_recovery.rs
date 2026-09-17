//! Bounded training-only recovery diagnostics; never imported by the product library.
use super::*;
use clap::Subcommand;
use replica_v3::{model::ModelRequest, neural::checkpoint::Loaded};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{collections::BTreeSet, io::Write, path::PathBuf, sync::Arc, time::Duration};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
enum StopReason {
    Cancelled,
    TimeBudget,
    ResourceLimit,
    ResourceObservationFailed,
    IntegrityFail,
    QualityGuard,
    TokenBudget,
    AuditIncomplete,
}
impl StopReason {
    fn name(self) -> &'static str {
        match self {
            Self::Cancelled => "CANCELLED",
            Self::TimeBudget => "TIME_BUDGET",
            Self::ResourceLimit => "RESOURCE_LIMIT",
            Self::ResourceObservationFailed => "RESOURCE_OBSERVATION_FAILED",
            Self::IntegrityFail => "INTEGRITY_FAIL",
            Self::QualityGuard => "QUALITY_GUARD",
            Self::TokenBudget => "TOKEN_BUDGET",
            Self::AuditIncomplete => "AUDIT_INCOMPLETE",
        }
    }
}
/// One cooperative command budget. Synchronous tensor operations/fsync cannot be preempted.
pub(super) struct RunControl {
    cancel: Arc<AtomicBool>,
    start: Instant,
    deadline: Instant,
    max_rss_kib: u64,
    last_rss_kib: Option<u64>,
    stop: Option<StopReason>,
    observed: Vec<StopReason>,
    terminal: bool,
    generation_calls: usize,
    completed_generation_count: usize,
    attempted_case_count: usize,
    interrupted_case_id: Option<String>,
    teacher_calls: usize,
    #[cfg(test)]
    elapsed_override: Option<Duration>,
    #[cfg(test)]
    #[allow(clippy::type_complexity)]
    hook: Option<Box<dyn FnMut(&str, &Arc<AtomicBool>)>>,
}
impl RunControl {
    fn new(cancel: Arc<AtomicBool>, duration: Duration, max_rss_kib: u64) -> Result<Self> {
        let start = Instant::now();
        let deadline = start
            .checked_add(duration)
            .ok_or_else(|| Error::Invalid("command deadline overflow".into()))?;
        Ok(Self {
            cancel,
            start,
            deadline,
            max_rss_kib,
            last_rss_kib: None,
            stop: None,
            observed: vec![],
            terminal: false,
            generation_calls: 0,
            completed_generation_count: 0,
            attempted_case_count: 0,
            interrupted_case_id: None,
            teacher_calls: 0,
            #[cfg(test)]
            elapsed_override: None,
            #[cfg(test)]
            hook: None,
        })
    }
    pub(super) fn command(training: bool) -> Result<Self> {
        let flag = Arc::new(AtomicBool::new(false));
        let signal = flag.clone();
        let control = Self::new(
            flag,
            Duration::from_secs(900),
            if training { 16 } else { 12 } * 1024 * 1024,
        )?;
        ctrlc::set_handler(move || signal.store(true, Ordering::Relaxed))
            .map_err(|e| Error::Model(e.to_string()))?;
        Ok(control)
    }
    fn now(&self) -> Instant {
        #[cfg(test)]
        if let Some(elapsed) = self.elapsed_override {
            return self.start + elapsed;
        }
        Instant::now()
    }
    fn observe(&mut self, reason: StopReason) {
        if self.terminal {
            return;
        }
        if !self.observed.contains(&reason) {
            self.observed.push(reason);
        }
        self.stop.get_or_insert(reason);
    }
    pub(super) fn stop_result(&self) -> Result<()> {
        match self.stop {
            None => Ok(()),
            Some(StopReason::Cancelled) => Err(Error::Cancelled),
            Some(r) => Err(Error::Model(r.name().into())),
        }
    }
    fn check_at(&mut self, now: Instant, rss: Result<u64>) -> Result<()> {
        if self.terminal {
            return self.stop_result();
        }
        // Simultaneous observation priority: user cancel, deadline, RSS failure, RSS limit.
        if self.cancel.load(Ordering::Relaxed) {
            self.observe(StopReason::Cancelled);
        }
        if now >= self.deadline {
            self.observe(StopReason::TimeBudget);
        }
        self.last_rss_kib = rss.as_ref().ok().copied();
        match rss {
            Err(_) => self.observe(StopReason::ResourceObservationFailed),
            Ok(n) if n > self.max_rss_kib => self.observe(StopReason::ResourceLimit),
            _ => {}
        }
        self.stop_result()
    }
    pub(super) fn check(&mut self, boundary: &str) -> Result<()> {
        #[cfg(test)]
        if let Some(hook) = &mut self.hook {
            hook(boundary, &self.cancel);
        }
        #[cfg(not(test))]
        let _ = boundary;
        let rss = rss_kib();
        self.check_at(self.now(), rss)
    }
    fn effective_timeout(&mut self, original: u64) -> Result<u64> {
        self.check("generation_budget")?;
        let remaining = self
            .deadline
            .saturating_duration_since(self.now())
            .as_millis();
        let remaining = u64::try_from(remaining)
            .map_err(|_| Error::Invalid("remaining timeout overflow".into()))?;
        if remaining == 0 {
            self.observe(StopReason::TimeBudget);
            return self.stop_result().map(|_| 0);
        }
        if original == 0 {
            return Err(Error::Invalid("zero request timeout".into()));
        }
        Ok(original.min(remaining))
    }
    fn classify_error(&mut self, e: &Error) {
        if matches!(e, Error::Cancelled) {
            self.observe(StopReason::Cancelled);
        } else if self.stop.is_none() {
            self.observe(StopReason::IntegrityFail);
        }
    }
    fn terminal_reason(&mut self, complete: bool) -> &'static str {
        let _ = self.check("terminal");
        if self.stop.is_none() && !complete {
            self.observe(StopReason::IntegrityFail);
        }
        self.terminal = true; // Later signals do not retroactively invalidate this decision.
        self.stop
            .map_or("SCREENING_BUDGET_REACHED", StopReason::name)
    }
    pub(super) fn seal_terminal(&mut self) -> Result<()> {
        let result = self.check("terminal");
        self.terminal = true;
        result
    }
    fn receipt(&self) -> Value {
        json!({"terminal_reason":self.stop.map(StopReason::name).or_else(||self.terminal.then_some("COMPLETED")),"observed_conditions":self.observed,"generation_calls":self.generation_calls,"teacher_calls":self.teacher_calls,"completed_generation_count":self.completed_generation_count,"attempted_case_count":self.attempted_case_count,"interrupted_case_id":self.interrupted_case_id,
            "elapsed_seconds":self.now().duration_since(self.start).as_secs_f64(),"work_budget_seconds":self.deadline.duration_since(self.start).as_secs_f64(),
            "work_deadline_overrun_seconds":self.now().saturating_duration_since(self.deadline).as_secs_f64(),"cooperative_only":true})
    }
}

#[derive(Subcommand)]
pub enum Command {
    /// Compare completed or stopped schedule/group runs from their actual records.
    ScheduleReport {
        #[arg(long)]
        fixture: PathBuf,
        #[arg(long)]
        control: PathBuf,
        #[arg(long)]
        treatment: PathBuf,
        #[arg(long, default_value="schedule", value_parser=["schedule","group"])]
        factor: String,
        #[arg(long)]
        output: PathBuf,
    },
    /// Recount preserved ledgers and verify C/W trace binding without model calls.
    Recount {
        #[arg(long)]
        fixture: PathBuf,
        #[arg(long)]
        parent_log: PathBuf,
        #[arg(long)]
        failed_log: PathBuf,
        #[arg(long)]
        control: PathBuf,
        #[arg(long)]
        treatment: PathBuf,
        #[arg(long)]
        audit: PathBuf,
        #[arg(long)]
        output: PathBuf,
    },
    /// Verify the actual one-factor traces, fresh-process artifacts and product worker.
    Close {
        #[arg(long)]
        fixture: PathBuf,
        #[arg(long)]
        control: PathBuf,
        #[arg(long)]
        treatment: PathBuf,
        #[arg(long)]
        legacy_tokenizer: PathBuf,
        #[arg(long)]
        worker_binary: PathBuf,
        #[arg(long)]
        source_id: String,
        #[arg(long)]
        output: PathBuf,
    },
    /// Fixed-tape screening; explicit renewed runs may compare the parent LR policy.
    Arm {
        #[arg(long)]
        fixture: PathBuf,
        #[arg(long, value_parser=["C","W","L","B"])]
        arm: String,
        #[arg(long, default_value_t = 50, value_parser = clap::value_parser!(u16).range(1..=250))]
        max_updates: u16,
        #[arg(long)]
        source_id: String,
        #[arg(long)]
        output: PathBuf,
    },
    Freeze {
        #[arg(long)]
        parent: PathBuf,
        #[arg(long)]
        start: PathBuf,
        #[arg(long)]
        failed: PathBuf,
        #[arg(long)]
        probe: PathBuf,
        #[arg(long)]
        corpus: PathBuf,
        #[arg(long)]
        parent_corpus: PathBuf,
        #[arg(long)]
        parent_log: PathBuf,
        #[arg(long)]
        failed_log: PathBuf,
        #[arg(long)]
        source_id: String,
        #[arg(long)]
        output: PathBuf,
    },
    Replay {
        #[arg(long)]
        fixture: PathBuf,
        #[arg(long)]
        checkpoint: PathBuf,
        #[arg(long)]
        source_id: Option<String>,
        /// Prior raw receipts for the identical panel; compared without extra generation.
        #[arg(long)]
        reference: Option<PathBuf>,
        #[arg(long)]
        output: PathBuf,
        #[arg(long, default_value="watch", value_parser=["watch", "failures", "all"])]
        panel: String,
    },
    Audit {
        #[arg(long)]
        fixture: PathBuf,
        #[arg(long)]
        output: PathBuf,
    },
    Numeric {
        #[arg(long)]
        fixture: PathBuf,
        #[arg(long)]
        checkpoint: PathBuf,
        #[arg(long)]
        output: PathBuf,
    },
}
#[derive(Serialize, Deserialize)]
struct Frozen {
    version: u32,
    registry: Value,
    corpus: PathBuf,
    parent_corpus: PathBuf,
    start: PathBuf,
    train_hash: String,
    validation_hash: String,
    parent_train_hash: String,
    tokenizer: String,
    watch: Vec<Episode>,
    failures: Vec<Episode>,
    previous_parent: Vec<Value>,
    previous_failed: Vec<Value>,
}
fn digest<T: Serialize + ?Sized>(value: &T) -> Result<String> {
    Ok(neural::hash(&serde_json::to_vec(value)?))
}
fn file_hash(path: &Path) -> Result<String> {
    use sha2::{Digest, Sha256};
    use std::io::Read;
    let mut file = std::fs::File::open(path)?;
    let mut hash = Sha256::new();
    let mut buf = [0u8; 65536];
    loop {
        let n = file.read(&mut buf)?;
        if n == 0 {
            break;
        }
        hash.update(&buf[..n]);
    }
    Ok(format!("{:x}", hash.finalize()))
}
fn source_commit() -> Result<String> {
    let out = std::process::Command::new("git")
        .args(["rev-parse", "HEAD"])
        .output()?;
    if !out.status.success() {
        return Err(Error::Invalid(
            "diagnostic source commit unavailable".into(),
        ));
    }
    Ok(String::from_utf8(out.stdout)
        .map_err(|_| Error::Invalid("source commit encoding".into()))?
        .trim()
        .to_string())
}
fn save(path: &Path, value: &impl Serialize) -> Result<()> {
    neural::write_new(path, &serde_json::to_vec_pretty(value)?)
}
fn load(path: &Path) -> Result<Frozen> {
    let f: Frozen = serde_json::from_slice(&neural::read_bounded(path, 16 * 1024 * 1024)?)?;
    if f.version != 1 || f.watch.len() != 32 || f.failures.len() > 16 {
        return Err(Error::Corrupt("recovery frozen panel".into()));
    }
    Ok(f)
}
fn scene(e: &Episode) -> &str {
    let id =
        e.id.strip_prefix("qa-pairs/")
            .and_then(|s| s.splitn(3, '/').nth(2))
            .unwrap_or(&e.id);
    id.rsplit_once('/').map_or(id, |(a, _)| a)
}
fn rows(path: &Path) -> Result<(Value, Vec<Value>)> {
    let bytes = neural::read_bounded(path, 16 * 1024 * 1024)?;
    let values: Vec<Value> = bytes
        .split(|&b| b == b'\n')
        .filter(|b| !b.is_empty())
        .map(serde_json::from_slice)
        .collect::<std::result::Result<_, _>>()?;
    let header = values
        .first()
        .filter(|v| v["header"] == true)
        .ok_or_else(|| Error::Corrupt("evaluation header".into()))?
        .clone();
    let cases: Vec<_> = values
        .iter()
        .filter(|v| v.get("exact_match").is_some())
        .cloned()
        .collect();
    let summary = summarize(&cases)?;
    let old = values
        .iter()
        .rev()
        .find(|v| v["summary"] == true)
        .ok_or_else(|| Error::Corrupt("evaluation summary".into()))?;
    if values
        .iter()
        .any(|v| v["terminal"] == true && v["comparison_eligible"] != true)
    {
        return Err(Error::Invalid(
            "partial evaluation is not comparison eligible".into(),
        ));
    }
    if old["summary"] != true
        || old["denominator"] != summary["denominator"]
        || old["exact_matches"] != summary["exact_matches"]
        || old["generation_failures"] != summary["generation_failures"]
    {
        return Err(Error::Corrupt("evaluation ledger count mismatch".into()));
    }
    Ok((header, cases))
}
fn summarize(rows: &[Value]) -> Result<Value> {
    let mut ids = BTreeSet::new();
    let mut groups: BTreeMap<String, [usize; 2]> = BTreeMap::new();
    let (mut errors, mut empty, mut utf8, mut first_eos, mut control, mut timeout) =
        (0, 0, 0, 0, 0, 0);
    let (mut nll, mut tokens, mut correct, mut macro_ce) = (0., 0u64, 0u64, 0.);
    let mut scored = 0;
    for row in rows {
        if !ids.insert(
            row["id"]
                .as_str()
                .ok_or_else(|| Error::Corrupt("missing case ID".into()))?,
        ) {
            return Err(Error::Corrupt(
                "duplicate case ID in score denominator".into(),
            ));
        }
        let error = row["error"].as_str().unwrap_or("");
        errors += usize::from(!error.is_empty());
        utf8 += usize::from(error.contains("UTF-8"));
        control += usize::from(error.contains("control token"));
        timeout += usize::from(error.contains("timeout"));
        empty += usize::from(row["actual"] == "" && error.is_empty());
        first_eos += usize::from(
            row["generation"]["finish"] == "stop" && row["generation"]["generated"] == 1,
        );
        let matched = row["actual"].is_string()
            && row["actual"] == row["expected"]
            && row["generation"]["finish"] == "stop"
            && error.is_empty();
        if row["exact_match"] != matched {
            return Err(Error::Corrupt("strict EM ledger mismatch".into()));
        }
        let g = if row["family"].as_str().unwrap_or("").starts_with("copy/") {
            "copy".into()
        } else {
            format!("qa-{}", row["category"])
        };
        let g = groups.entry(g).or_default();
        g[0] += usize::from(matched);
        g[1] += 1;
        let t = &row["teacher_forced_diagnostic_after_generation"];
        if let (Some(mean), Some(n), Some(c)) = (
            t["mean_nll"].as_f64(),
            t["target_tokens_including_eos"].as_u64(),
            t["teacher_forced_correct_tokens"].as_u64(),
        ) {
            nll += mean * n as f64;
            tokens += n;
            correct += c;
            macro_ce += mean;
            scored += 1;
        }
    }
    let aux = groups.get("copy").copied().unwrap_or_default();
    let qa: [usize; 2] = groups
        .iter()
        .filter(|(k, _)| *k != "copy")
        .fold([0, 0], |a, (_, v)| [a[0] + v[0], a[1] + v[1]]);
    Ok(
        json!({"summary":true,"denominator":rows.len(),"exact_matches":qa[0]+aux[0],"qa":qa,"auxiliary":aux,
        "groups_correct_total":groups,"generation_failures":errors,"invalid_utf8":utf8,"empty":empty,"first_eos":first_eos,"control":control,"timeout":timeout,
        "teacher_forced_micro_ce":(tokens>0).then(||nll/tokens as f64),"teacher_forced_macro_ce":(scored>0).then(||macro_ce/scored as f64),
        "teacher_forced_token_accuracy":(tokens>0).then(||correct as f64/tokens as f64),"teacher_forced_cases":scored,"target_tokens_including_eos":tokens,
        "final_heldout":false,"metric":"strict-full-answer-eos-v1; errors included"}),
    )
}
fn registry(path: &Path) -> Result<Value> {
    let l = checkpoint::load(path, Device::Cpu, true)?;
    let s = l
        .manifest
        .training
        .as_ref()
        .ok_or_else(|| Error::Invalid("resume state required".into()))?;
    let schedule: Vec<_> = [0,1,5,20,50,100].iter().map(|&n| json!({"segment_step":n,"model_step":s.config.budget_start_step+n,"lr":s.config.learning_rate(s.config.budget_start_step+n)})).collect();
    Ok(
        json!({"path":path,"physical_hash":file_hash(path)?,"model_content_hash":l.model.weight_hash()?,"manifest":l.manifest,
        "tokenizer_wire":l.tokenizer.id(),"tokenizer_semantic":l.tokenizer.semantic_id(),"architecture_id":l.model.config.id()?,
        "dtype":"F32","backend":neural::cpu_backend(),"adam_shapes":l.optimizer.iter().map(|(k,v)|(k,v.dims())).collect::<BTreeMap<_,_>>(),
        "cumulative_model_step":s.step,"optimizer_step":s.step,"schedule_step":s.step-s.config.budget_start_step,
        "segment_update_count":s.step-s.config.budget_start_step,"current_lr_derived_config":s.config.learning_rate(s.step),"schedule_derived_config":schedule}),
    )
}
pub fn run(command: Command) -> Result<()> {
    let mut budget = RunControl::command(matches!(&command, Command::Arm { .. }))?;
    budget.check("command_started")?;
    let outcome = match command {
        Command::ScheduleReport {
            fixture,
            control,
            treatment,
            factor,
            output,
        } => schedule_report(
            &fixture,
            [&control, &treatment],
            &factor,
            &output,
            &mut budget,
        ),
        Command::Recount {
            fixture,
            parent_log,
            failed_log,
            control,
            treatment,
            audit,
            output,
        } => recount(
            &fixture,
            [&parent_log, &failed_log],
            [&control, &treatment],
            &audit,
            &output,
            &mut budget,
        ),
        Command::Close {
            fixture,
            control,
            treatment,
            legacy_tokenizer,
            worker_binary,
            source_id,
            output,
        } => {
            let output_existed = output.exists();
            let outcome = close(
                &fixture,
                &control,
                &treatment,
                &legacy_tokenizer,
                &worker_binary,
                &source_id,
                &output,
                &mut budget,
            );
            if let Err(error) = &outcome {
                budget.classify_error(error);
                if !output_existed && output.is_dir() {
                    let planned = load(&fixture)?.watch.len() * 2 + 6;
                    let mut partial = budget.receipt();
                    partial["planned_case_count"] = json!(planned);
                    partial["not_run_count"] =
                        json!(planned.saturating_sub(budget.attempted_case_count));
                    partial["final_evaluation_complete"] = json!(false);
                    partial["comparison_eligible"] = json!(false);
                    partial["candidate_eligible"] = json!(false);
                    partial["worker_calls_are_separate"] = json!(true);
                    save(&output.join("interrupted.json"), &partial)?;
                }
            }
            outcome
        }
        Command::Arm {
            fixture,
            arm,
            max_updates,
            source_id,
            output,
        } => arm_run(
            &fixture,
            &arm,
            usize::from(max_updates),
            &source_id,
            &output,
            &mut budget,
        ),
        Command::Freeze {
            parent,
            start,
            failed,
            probe,
            corpus,
            parent_corpus,
            parent_log,
            failed_log,
            source_id,
            output,
        } => {
            let (m, _, v) = data::load(&corpus)?;
            let (p, _, _) = data::load(&parent_corpus)?;
            let mut watch = Vec::new();
            let mut scenes = BTreeSet::new();
            for (category, count) in [7, 7, 6, 6, 6].into_iter().enumerate() {
                let selected: Vec<_> = v
                    .iter()
                    .filter(|e| {
                        e.category == category
                            && !e.family.starts_with("copy/")
                            && scenes.insert(scene(e).to_string())
                    })
                    .take(count)
                    .cloned()
                    .collect();
                if selected.len() != count {
                    return Err(Error::Invalid("watch metadata coverage".into()));
                }
                watch.extend(selected);
            }
            let (ph, pr) = rows(&parent_log)?;
            let (fh, fr) = rows(&failed_log)?;
            let registry = json!({"GENERAL_QA_PARENT":registry(&parent)?,"U2_POLICY_START":registry(&start)?,"U2_AFTER_20":registry(&probe)?,"U2_AFTER_250":registry(&failed)?,
                "source_id":source_id,"binary_hash":file_hash(&std::env::current_exe()?)?,"old_parent_ledger":ph,"old_failed_ledger":fh,"parent_raw_hash":file_hash(&parent_log)?,"failed_raw_hash":file_hash(&failed_log)?,
                "parent_observed_log_summary":summarize(&pr)?,"failed_observed_log_summary":summarize(&fr)?});
            if ph["split_sha256"] != m.validation.sha256
                || fh["split_sha256"] != m.validation.sha256
                || ph["checkpoint_sha256"]
                    != registry["U2_POLICY_START"]["manifest"]["weights_sha256"]
                || fh["checkpoint_sha256"] != registry["U2_AFTER_250"]["manifest"]["weights_sha256"]
                || registry["GENERAL_QA_PARENT"]["model_content_hash"]
                    != registry["U2_POLICY_START"]["model_content_hash"]
                || pr.len() != v.len()
                || fr.len() != v.len()
            {
                return Err(Error::Corrupt("artifact/log comparison identity".into()));
            }
            for row in pr.iter().chain(&fr) {
                let e = v
                    .iter()
                    .find(|e| row["id"] == e.id)
                    .ok_or_else(|| Error::Corrupt("log case absent".into()))?;
                if row["question"] != e.request.input
                    || row["expected"] != e.answer
                    || row["evidence"] != serde_json::to_value(&e.request.evidence)?
                {
                    return Err(Error::Corrupt("old log request/target mismatch".into()));
                }
            }
            let mut failures = Vec::new();
            let mut ids = BTreeSet::new();
            for kind in 0..4 {
                for row in fr
                    .iter()
                    .filter(|r| match kind {
                        0 => r["error"].as_str().is_some_and(|e| e.contains("UTF-8")),
                        1 => r["actual"] == "",
                        2 => {
                            r["actual"].is_string()
                                && r["exact_match"] == false
                                && r["category"] != 4
                        }
                        _ => r["exact_match"] == false,
                    })
                    .filter(|r| ids.insert(r["id"].as_str().unwrap().to_string()))
                    .take(4)
                {
                    failures.push(v.iter().find(|e| row["id"] == e.id).unwrap().clone());
                }
            }
            let f = Frozen {
                version: 1,
                tokenizer: registry["U2_POLICY_START"]["tokenizer_semantic"]
                    .as_str()
                    .unwrap()
                    .into(),
                registry,
                corpus,
                parent_corpus,
                start,
                train_hash: m.train.sha256,
                validation_hash: m.validation.sha256,
                parent_train_hash: p.train.sha256,
                watch,
                failures,
                previous_parent: pr,
                previous_failed: fr,
            };
            save(&output, &f)?;
            println!(
                "{}",
                json!({"frozen":output,"hash":file_hash(&output)?,"watch":f.watch.len(),"failures":f.failures.len(),"parent":f.registry["parent_observed_log_summary"],"failed":f.registry["failed_observed_log_summary"],"optimizer_updates":0})
            );
            Ok(())
        }
        Command::Replay {
            fixture,
            checkpoint,
            source_id,
            reference,
            output,
            panel,
        } => replay(
            &fixture,
            &checkpoint,
            &output,
            &panel,
            source_id.as_deref(),
            &mut budget,
            true,
            reference.as_deref(),
        ),
        Command::Audit { fixture, output } => audit(&fixture, &output, &mut budget),
        Command::Numeric {
            fixture,
            checkpoint,
            output,
        } => numeric(&fixture, &checkpoint, &output),
    };
    if let Err(error) = &outcome {
        budget.classify_error(error);
    }
    let _ = budget.check("command_returned");
    eprintln!("{}", budget.receipt());
    outcome.and(budget.stop_result())
}

fn bytes_receipt(tok: &ByteBpe, ids: &[u32]) -> Value {
    match tok.decode_bytes(ids) {
        Err(e) => json!({"byte_mapping_error":e.to_string()}),
        Ok(bytes) => {
            let utf8 = std::str::from_utf8(&bytes);
            json!({"length":bytes.len(),"sha256":neural::hash(&bytes),"hex":bytes.iter().take(2048).map(|b|format!("{b:02x}")).collect::<String>(),"hex_truncated":bytes.len()>2048,
                "utf8_valid":utf8.is_ok(),"valid_up_to":utf8.as_ref().err().map(|e|e.valid_up_to()),"error_len":utf8.as_ref().err().and_then(|e|e.error_len()),
                "utf8_error_class":utf8.err().map(|e|if e.error_len().is_none(){"incomplete_tail"}else{"invalid_sequence"})})
        }
    }
}
fn fields(text: &str) -> Option<(&str, &str, &str)> {
    let (entity, rest) = text.split_once("의 ")?;
    let (context, value) = rest.split_once(" 이동 지시는 ")?;
    let value = value.split_once("이다.")?.0;
    Some((entity, context, value))
}
fn components(actual: Option<&str>, expected: &str, provided: &[i64]) -> Value {
    let a = actual.and_then(fields);
    let e = fields(expected);
    let ids = actual.and_then(|a| citations(a).ok());
    let expected_ids = citations(expected).ok();
    json!({"entity":e.map(|e|a.is_some_and(|a|a.0==e.0)),"context":e.map(|e|a.is_some_and(|a|a.1==e.1)),"value":e.map(|e|a.is_some_and(|a|a.2==e.2)),
        "citation_exact":ids.as_ref().is_some_and(|a|Some(a)==expected_ids.as_ref()),"citation_in_provided":ids.as_ref().map(|a|a.iter().all(|id|provided.contains(id))),"citation_nonempty":ids.as_ref().is_some_and(|a|!a.is_empty())})
}
pub(super) fn evaluate_one(
    loaded: &Loaded,
    e: &Episode,
    request: &ModelRequest,
    control: &mut RunControl,
) -> Value {
    let mut row = json!({"id":e.id,"scene":scene(e),"category":e.category,"family":e.family,"question":e.request.input,"generated_question":request.input,
        "evidence":e.request.evidence,"generated_evidence":request.evidence,"expected":e.answer,"exact_match":false,"actual":null,"error":null,"generation_started":false,"generation_completed":false,"interruption":null});
    let result = (|| -> Result<()> {
        control.check("case_started")?;
        control.attempted_case_count += 1;
        let prompt = loaded.tokenizer.prepare(
            request,
            loaded.model.config.context as u32,
            &loaded.model.config.id()?,
        )?;
        control.check("prompt_prepared")?;
        let effective_timeout = control.effective_timeout(request.limits.timeout_ms)?;
        row["effective_timeout_ms"] = json!(effective_timeout);
        row["original_timeout_ms"] = json!(request.limits.timeout_ms);
        row["generation_started"] = json!(true);
        control.generation_calls += 1;
        let cancel = control.cancel.clone();
        let mut raw = Vec::new();
        let result = loaded.model.generate_observed(
            &prompt.token_ids,
            request.limits.max_tokens as usize,
            effective_timeout,
            &cancel,
            &e.id,
            |id| {
                raw.push(id);
                #[cfg(test)]
                if let Some(hook) = &mut control.hook {
                    hook("token_generated", &cancel);
                }
            },
        );
        if matches!(&result, Err(Error::Cancelled)) {
            control.observe(StopReason::Cancelled);
        }
        if result
            .as_ref()
            .err()
            .is_some_and(|e| e.to_string().contains("timeout"))
            && effective_timeout < request.limits.timeout_ms
        {
            control.observe(StopReason::TimeBudget);
        }
        if result
            .as_ref()
            .err()
            .is_some_and(|e| e.to_string().contains("nonfinite"))
        {
            control.observe(StopReason::IntegrityFail);
        }
        let after_generation = control.check("generation_returned");
        let (text, generated, error) = decode_generated(&loaded.tokenizer, result);
        let bytes_ids: Vec<_> = raw
            .iter()
            .copied()
            .take_while(|&id| id >= neural::SPECIALS as u32)
            .collect();
        row["raw_tokens"] = json!(raw);
        row["raw_bytes"] = bytes_receipt(&loaded.tokenizer, &bytes_ids);
        row["eos_index"] = json!(raw.iter().position(|&id| id == EOS));
        row["provided"] = json!(prompt.provided);
        row["excluded"] = json!(prompt.excluded);
        row["request_digest"] = json!(digest(request)?);
        row["prompt_digest"] = json!(digest(&prompt.token_ids)?);
        row["native_prompt_digest"] = json!(prompt.token_digest);
        row["prompt_length"] = json!(prompt.token_ids.len());
        row["exact_match"] = json!(
            text.as_deref() == Some(&e.answer)
                && generated.as_ref().is_some_and(|g| g.finish == "stop")
                && error.is_none()
        );
        row["components"] = components(text.as_deref(), &e.answer, &prompt.provided);
        row["actual"] = json!(text);
        row["generation"] = json!(generated);
        row["error"] = json!(error);
        row["error_class"] = json!(error.as_ref().map(|e| if e.contains("UTF-8") {
            "strict_utf8"
        } else if e.contains("control token") {
            "control_token"
        } else if e.contains("timeout") {
            "timeout"
        } else if e.contains("cancel") {
            "cancelled"
        } else {
            "generation_or_mapping"
        }));
        row["finish_reason"] = generated
            .as_ref()
            .map_or_else(|| row["error_class"].clone(), |g| json!(g.finish));
        row["raw_generated_count"] = json!(raw.len());
        row["generation_completed"] = json!(control.stop.is_none());
        control.completed_generation_count += usize::from(control.stop.is_none());
        row["whitespace_only"] = json!(
            text.as_ref()
                .is_some_and(|s| !s.is_empty() && s.trim().is_empty())
        );
        // Keep the actual generation receipt before propagating a command stop.
        after_generation?;
        control.check("before_teacher")?;
        // Gold enters only after free generation has completed, including failures.
        row["teacher_forced_diagnostic_after_generation"] =
            match teacher(loaded, e, &prompt.token_ids, &raw, control) {
                Ok(t) => t,
                Err(e) => {
                    if e.to_string().contains("nonfinite") {
                        control.classify_error(&e);
                    }
                    json!({"error":e.to_string()})
                }
            };
        Ok(())
    })();
    if let Err(error) = result {
        if control.stop.is_none() {
            row["error"] = json!(error.to_string());
            row["error_class"] = json!("preparation_or_receipt");
        } else {
            row["diagnostic_stop_error"] = json!(error.to_string());
        }
    }
    if let Some(stop) = control.stop {
        row["interruption"] = json!(stop);
        control.interrupted_case_id = Some(e.id.clone());
    }
    row
}
fn teacher(
    l: &Loaded,
    e: &Episode,
    prompt: &[u32],
    raw: &[u32],
    control: &mut RunControl,
) -> Result<Value> {
    control.check("teacher_started")?;
    control.teacher_calls += 1;
    let mut gold = l.tokenizer.encode(e.answer.as_bytes())?;
    gold.push(EOS);
    let mut sequence = prompt.to_vec();
    sequence.extend(&gold);
    if sequence.len() > l.model.config.context {
        return Err(Error::ContextTooSmall);
    }
    control.check("teacher_forward")?;
    let logits = l
        .model
        .forward(
            &Tensor::new(&sequence[..sequence.len() - 1], &Device::Cpu)?.unsqueeze(0)?,
            None,
        )?
        .narrow(1, prompt.len() - 1, gold.len())?
        .squeeze(0)?;
    control.check("teacher_returned")?;
    let lp = candle_nn::ops::log_softmax(&logits, 1)?.to_vec2::<f32>()?;
    if lp.iter().flatten().any(|x| !x.is_finite()) {
        return Err(Error::Model("nonfinite diagnostic logits".into()));
    }
    let predicted = logits.argmax(1)?.to_vec1::<u32>()?;
    let nll: Vec<_> = gold
        .iter()
        .enumerate()
        .map(|(i, &t)| -f64::from(lp[i][t as usize]))
        .collect();
    let mismatch = gold
        .iter()
        .zip(raw)
        .position(|(a, b)| a != b)
        .or_else(|| (gold.len() != raw.len()).then_some(gold.len().min(raw.len())));
    let bytes = l.tokenizer.decode_bytes(
        &raw.iter()
            .copied()
            .take_while(|&x| x >= neural::SPECIALS as u32)
            .collect::<Vec<_>>(),
    )?;
    let byte_difference = e
        .answer
        .as_bytes()
        .iter()
        .zip(&bytes)
        .position(|(a, b)| a != b)
        .or_else(|| (e.answer.len() != bytes.len()).then_some(e.answer.len().min(bytes.len())));
    let framed = samples(
        std::slice::from_ref(e),
        &l.tokenizer,
        l.manifest
            .training
            .as_ref()
            .map_or(512, |s| s.config.seq_len),
    )?;
    let w = l
        .manifest
        .training
        .as_ref()
        .map_or(1., |s| s.config.first_target_weight);
    let mut field_accuracy: BTreeMap<String, [usize; 2]> = BTreeMap::new();
    let mut byte = 0;
    for (i, &id) in gold.iter().enumerate() {
        let name = field_at(&e.answer, byte);
        let counts = field_accuracy.entry(name.into()).or_default();
        counts[0] += usize::from(id == predicted[i]);
        counts[1] += 1;
        if id != EOS {
            byte += l.tokenizer.decode_bytes(&[id])?.len();
        }
    }
    let difference=mismatch.filter(|&i|i<gold.len()).map(|i|{
        let rival=raw.get(i).copied().unwrap_or(predicted[i]);
        json!({"index":i,"gold_id":gold[i],"actual_id":raw.get(i),"teacher_argmax":predicted[i],"gold_log_probability":lp[i][gold[i] as usize],"gold_minus_rival_logit":lp[i][gold[i] as usize]-lp[i][rival as usize],"prefix":"gold; at first divergence identical to generation prefix"})
    });
    control.check("teacher_completed")?;
    Ok(
        json!({"target_tokens_including_eos":gold.len(),"mean_nll":nll.iter().sum::<f64>()/gold.len() as f64,"first_target_nll":nll[0],
        "remaining_mean_nll":nll.iter().skip(1).sum::<f64>()/(gold.len()-1).max(1) as f64,"objective":(nll.iter().sum::<f64>()+(w-1.)*nll[0])/gold.len() as f64,"first_target_weight":w,
        "teacher_forced_correct_tokens":gold.iter().zip(&predicted).filter(|(a,b)|a==b).count(),"first_target_correct":gold[0]==predicted[0],"last_content_correct":gold.len()>1 && gold[gold.len()-2]==predicted[gold.len()-2],"eos_correct":predicted.last()==Some(&EOS),
        "first_argmax":predicted[0],"first_eos_probability":lp[0][EOS as usize].exp(),"first_gold_probability":lp[0][gold[0] as usize].exp(),"first_argmax_probability":lp[0][predicted[0] as usize].exp(),"first_gold_id":gold[0],
        "first_difference":difference,"first_byte_difference":byte_difference,"first_difference_field":byte_difference.map(|p|field_at(&e.answer,p)),"field_token_accuracy":field_accuracy,"training_prompt_matches_generation":framed[0].tokens[..framed[0].response_start]==*prompt,
        "answer_tokenizer_roundtrip":l.tokenizer.decode(&gold[..gold.len()-1])?==e.answer}),
    )
}
fn field_at(answer: &str, byte: usize) -> &'static str {
    if byte >= answer.len() {
        return "eos";
    }
    if answer.find("[event:").is_some_and(|at| byte >= at) {
        return "citation";
    }
    if let Some((entity, context, _)) = fields(answer) {
        if byte < entity.len() {
            return "entity";
        }
        let context_start = entity.len() + "의 ".len();
        if (context_start..context_start + context.len()).contains(&byte) {
            return "context";
        }
        if byte >= context_start + context.len() + " 이동 지시는 ".len() {
            return "value";
        }
        return "format";
    }
    "text"
}
#[allow(clippy::too_many_arguments)] // Existing command inputs, shared budget, and optional raw parity receipt.
fn replay(
    fixture: &Path,
    path: &Path,
    output: &Path,
    panel: &str,
    source_id: Option<&str>,
    control: &mut RunControl,
    command_terminal: bool,
    reference: Option<&Path>,
) -> Result<()> {
    control.check("replay_started")?;
    let f = load(fixture)?;
    // This owns the exact bytes-validated episodes. Do not re-read the split after binding.
    // Reject before opening an output or loading a model.
    let (cases, binding) = replay_cases(&f, panel)?;
    let reference_rows = reference
        .map(|path| -> Result<_> {
            let (header, rows) = rows(path)?;
            let source_split = header
                .get("source_validation_hash")
                .unwrap_or(&header["split_hash"]);
            if header["fixture_hash"] != file_hash(fixture)?
                || header["tokenizer_semantic_hash"] != f.tokenizer
                || header["prompt_format"] != neural::PROMPT_FORMAT
                || *source_split != f.validation_hash
                || header["ordered_ids_hash"] != binding["ordered_ids_hash"]
                || header["decoding"]
                    != json!(cases.iter().map(|e| &e.request.limits).collect::<Vec<_>>())
            {
                return Err(Error::Corrupt("reference replay binding".into()));
            }
            verify_historical_rows(&cases, &rows)?;
            Ok((header, rows))
        })
        .transpose()?;
    control.check("replay_bound")?;
    let binary_hash = file_hash(&std::env::current_exe()?)?;
    let source_id = source_id
        .or_else(|| {
            (f.registry["binary_hash"] == binary_hash)
                .then(|| f.registry["source_id"].as_str())
                .flatten()
        })
        .filter(|s| s.len() == 64 && s.bytes().all(|b| b.is_ascii_hexdigit()))
        .ok_or_else(|| {
            Error::Invalid("changed evaluation binary requires explicit --source-id".into())
        })?;
    let l = checkpoint::load(path, Device::Cpu, false)?;
    control.check("replay_loaded")?;
    if l.tokenizer.semantic_id() != f.tokenizer {
        return Err(Error::Corrupt("replay tokenizer".into()));
    }
    let model_content_hash = l.model.weight_hash()?;
    if reference_rows
        .as_ref()
        .is_some_and(|(h, _)| h["model_content_hash"] != model_content_hash)
    {
        return Err(Error::Corrupt("reference model content binding".into()));
    }
    let mut out = std::fs::OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(output)?;
    let ledger = json!({"header":true,"fixture_hash":file_hash(fixture)?,"binary_hash":file_hash(&std::env::current_exe()?)?,"checkpoint_physical_hash":file_hash(path)?,"model_content_hash":model_content_hash,"tokenizer_semantic_hash":l.tokenizer.semantic_id(),"prompt_format":neural::PROMPT_FORMAT,"decoding":cases.iter().map(|e|&e.request.limits).collect::<Vec<_>>(),"metric":"strict-full-answer-eos-v1","panel":panel,"final_heldout":false});
    let mut ledger = ledger;
    ledger
        .as_object_mut()
        .unwrap()
        .extend(binding.as_object().unwrap().clone());
    ledger["source_commit"] = json!(source_commit()?);
    ledger["working_source_manifest_hash"] = json!(source_id);
    ledger["model_tensor_content_digest"] = json!(l.model.weights_content_id()?);
    ledger["reference_raw_hash"] = json!(reference.map(file_hash).transpose()?);
    writeln!(out, "{ledger}")?;
    let mut rows = Vec::new();
    for e in &cases {
        if control.check("panel_next_case").is_err() {
            break;
        }
        let row = evaluate_one(&l, e, &e.request, control);
        writeln!(out, "{row}")?;
        out.flush()?;
        rows.push(row);
        if control.check("case_recorded").is_err() {
            break;
        }
    }
    // A->B->A: each call uses a fresh actual model cache; gold never affects request IDs.
    let mut aba_equal = None;
    if cases.len() > 1 && control.check("before_aba").is_ok() && rows.len() == cases.len() {
        let a = evaluate_one(&l, &cases[0], &cases[0].request, control);
        if control.stop.is_none()
            && (a["raw_tokens"] != rows[0]["raw_tokens"]
                || a["error"] != rows[0]["error"]
                || a["prompt_digest"] != rows[0]["prompt_digest"])
        {
            return Err(Error::Model("cache/request order regression".into()));
        }
        aba_equal = control.stop.is_none().then_some(true);
    }
    let old = if model_content_hash == f.registry["U2_POLICY_START"]["model_content_hash"] {
        Some(&f.previous_parent)
    } else if model_content_hash == f.registry["U2_AFTER_250"]["model_content_hash"] {
        Some(&f.previous_failed)
    } else {
        None
    };
    let mut differences = 0;
    if let Some(old) = old {
        for row in &rows {
            let before = old
                .iter()
                .find(|r| r["id"] == row["id"])
                .ok_or_else(|| Error::Corrupt("missing old case".into()))?;
            differences += usize::from(
                before["actual"] != row["actual"]
                    || before["error"] != row["error"]
                    || before["exact_match"] != row["exact_match"],
            );
        }
    }
    let mut summary = summarize(&rows)?;
    let mut reference_differences = Vec::new();
    if let Some((_, previous)) = &reference_rows {
        for row in &rows {
            let before = previous
                .iter()
                .find(|r| r["id"] == row["id"])
                .ok_or_else(|| Error::Corrupt("reference case missing".into()))?;
            for key in [
                "raw_tokens",
                "actual",
                "error",
                "prompt_digest",
                "provided",
                "excluded",
            ] {
                if row[key] != before[key] {
                    reference_differences.push(json!({"id":row["id"],"field":key}));
                }
            }
        }
    }
    if !reference_differences.is_empty() && control.stop.is_none() {
        control.observe(StopReason::IntegrityFail);
    }
    summary["reference_differences"] = json!(reference_differences);
    summary["reference_parity"] =
        json!(reference_rows.as_ref().map(|_| if control.stop.is_some() {
            "PARTIAL_OR_FAILED"
        } else {
            "PASS"
        }));
    if differences > 0 {
        control.observe(StopReason::IntegrityFail);
    }
    summary["old_output_differences"] = json!(old.map(|_| differences));
    summary["aba_equal"] = json!(aba_equal);
    let _ = control.check("before_evaluation_record");
    add_partial_counts(&mut summary, &rows, cases.len(), control);
    writeln!(out, "{summary}")?;
    out.sync_all()?;
    let _ = control.check("evaluation_recorded");
    if command_terminal {
        control.terminal = true;
    }
    let mut terminal = json!({"terminal":true,"control":control.receipt()});
    terminal["command_terminal"] = json!(command_terminal);
    add_partial_counts(&mut terminal, &rows, cases.len(), control);
    writeln!(out, "{terminal}")?;
    out.sync_all()?;
    println!("{summary}");
    if differences > 0 {
        return Err(Error::Model(
            "replay differs; no training authorized by this gate".into(),
        ));
    }
    control.stop_result()
}

pub(super) fn add_partial_counts(
    value: &mut Value,
    rows: &[Value],
    planned: usize,
    control: &RunControl,
) {
    let complete = rows.len() == planned && control.stop.is_none();
    value["planned_case_count"] = json!(planned);
    value["attempted_case_count"] = json!(rows.len());
    value["completed_generation_count"] = json!(
        rows.iter()
            .filter(|r| r["generation_completed"] == true)
            .count()
    );
    value["not_run_count"] = json!(planned.saturating_sub(rows.len()));
    value["interrupted_case_id"] = rows
        .iter()
        .find(|r| !r["interruption"].is_null())
        .map_or(Value::Null, |r| r["id"].clone());
    value["terminal_reason"] = control.receipt()["terminal_reason"].clone();
    value["final_evaluation_complete"] = json!(complete);
    value["comparison_eligible"] = json!(complete);
    value["candidate_eligible"] = json!(false); // This receipt alone never selects a model.
    value["score_scope"] = json!(if complete {
        "complete_panel"
    } else {
        "partial_attempted_cases_only"
    });
}

fn evaluate_panel(l: &Loaded, cases: &[Episode], control: &mut RunControl) -> Vec<Value> {
    let mut rows = Vec::new();
    for e in cases {
        if control.check("panel_next_case").is_err() {
            break;
        }
        rows.push(evaluate_one(l, e, &e.request, control));
        if control.check("panel_case_returned").is_err() {
            break;
        }
    }
    let _ = control.check("panel_completed");
    rows
}

fn replay_cases(f: &Frozen, panel: &str) -> Result<(Vec<Episode>, Value)> {
    let (cases, actual_split_hash) = match panel {
        "watch" => (f.watch.clone(), None),
        "failures" => (f.failures.clone(), None),
        "all" => {
            let (manifest, _, validation) = data::load(&f.corpus)?;
            if manifest.validation.sha256 != f.validation_hash {
                return Err(Error::Corrupt(
                    "frozen/current validation binding mismatch".into(),
                ));
            }
            (validation, Some(manifest.validation.sha256))
        }
        _ => return Err(Error::Invalid("unknown recovery replay panel".into())),
    };
    let binding = json!({
        "source_validation_hash":f.validation_hash,
        "frozen_expected_validation_hash":f.validation_hash,
        "actual_split_hash":actual_split_hash,
        "evaluated_cases_hash":digest(&cases)?,
        "evaluated_cases_encoding":"sha256(serde_json::to_vec(ordered Vec<Episode>)); complete fields, no whitespace",
        "ordered_ids_hash":digest(&cases.iter().map(|e|&e.id).collect::<Vec<_>>())?,
        "planned_case_count":cases.len(),
        "input_source":if panel=="all" {"validated_current_snapshot"} else {"frozen_panel"}
    });
    Ok((cases, binding))
}

// Independent semantic check: only serialized question, original records and status/time.
// It deliberately does not accept Episode labels, answers, categories or binding metadata.
fn support(request: &ModelRequest) -> Result<Vec<i64>> {
    let q = &request.input;
    let mut focus = q.as_str();
    for marker in ["말고 ", "아닌 ", "나오지만 ", "대신 "] {
        if let Some((_, right)) = focus.rsplit_once(marker) {
            focus = right;
        }
    }
    if q.contains("원인") || q.contains("인과관계") {
        let accident: Vec<_> = request
            .evidence
            .items
            .iter()
            .filter(|r| {
                r.original_excerpt.contains("사고가 기록")
                    && r.original_excerpt.contains("원인은 확인되지 않았다")
                    && r.original_excerpt
                        .strip_prefix("이후 ")
                        .and_then(|s| s.split_once("의 사고"))
                        .is_some_and(|(entity, _)| mentions_target(q, entity))
            })
            .collect();
        if accident.len() > 1 {
            return Err(Error::Invalid(
                "DATA_AMBIGUITY: multiple accident records".into(),
            ));
        }
        if q.contains("함께") || q.contains("불확실성을 설명") {
            return chronology_citations(request)
                .ok_or_else(|| Error::Invalid("DATA_AMBIGUITY: unsupported chronology".into()));
        }
        return Ok(accident.iter().map(|r| r.event_id).collect());
    }
    let earliest = focus.contains("최초") || focus.contains("처음");
    let past = earliest
        || ["과거", "이전", "정정 전", "예전에"]
            .iter()
            .any(|s| focus.contains(s));
    let focus_has_context = request
        .evidence
        .items
        .iter()
        .filter_map(|r| fields(&r.original_excerpt))
        .any(|(_, c, _)| mentions_target(focus, c));
    let mut selected = Vec::new();
    for r in &request.evidence.items {
        if let Some((entity, context, _)) = fields(&r.original_excerpt) {
            let context_mentioned =
                mentions_target(if focus_has_context { focus } else { q }, context);
            if mentions_target(q, entity)
                && context_mentioned
                && (earliest || r.version_status == if past { "superseded" } else { "current" })
                && !r.excerpt_truncated
            {
                selected.push(r);
            }
        }
    }
    if earliest && !selected.is_empty() {
        selected.sort_by_key(|r| r.recorded_at);
        if selected.len() > 1 && selected[0].recorded_at == selected[1].recorded_at {
            return Err(Error::Invalid(
                "DATA_AMBIGUITY: tied first record timestamp".into(),
            ));
        }
        selected.truncate(1);
    }
    if selected.len() != 1 {
        return Err(Error::Invalid(format!(
            "DATA_AMBIGUITY: independent question support count={}",
            selected.len()
        )));
    }
    Ok(vec![selected[0].event_id])
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
enum SemanticState {
    Validated,
    Contradicted,
    UnsupportedForm,
    AmbiguousEvidence,
    OutOfScope,
}
enum Obligation<'a> {
    Fact {
        fields: (&'a str, &'a str, &'a str),
        id: i64,
    },
    Cause {
        accident: Option<i64>,
        entity: Option<&'a str>,
        chronology: Option<Vec<i64>>,
        require_chronology: bool,
    },
}
enum TargetClaims<'a> {
    Fact {
        fields: (&'a str, &'a str, &'a str),
        id: i64,
    },
    NoEvidence,
    Uncertain(i64),
    Chronology {
        entity: Option<&'a str>,
        ids: Vec<i64>,
        uncertain: bool,
    },
    CauseAssertion,
}
fn exact_fact(text: &str) -> Option<(&str, &str, &str)> {
    let text = text.strip_suffix("이다.")?;
    let (entity, rest) = text.split_once("의 ")?;
    let (context, value) = rest.split_once(" 이동 지시는 ")?;
    let atom =
        |s: &str| !s.is_empty() && !s.chars().any(|c| c.is_whitespace() || ".[]!?".contains(c));
    (atom(entity) && atom(context) && atom(value)).then_some((entity, context, value))
}
fn accident_entity(text: &str) -> Option<&str> {
    text.strip_prefix("이후 ")?
        .strip_suffix("의 사고가 기록되었다. 원인은 확인되지 않았다.")
        .filter(|s| !s.is_empty() && !s.chars().any(char::is_whitespace))
}
fn derive_obligation(
    request: &ModelRequest,
) -> std::result::Result<Obligation<'_>, (SemanticState, &'static str)> {
    let q = request.input.as_str();
    if q.contains("원인") || q.contains("인과관계") {
        if ![
            "사고 원인은 무엇인가?",
            "사고의 원인이 자료로 확인되는가?",
            "원인으로 확정됐나?",
            "원인을 알 수 있나?",
            "원인을 확인할 수 있어?",
            "원인을 알 수 있는지 답해줘.",
            "원인으로 확정한 기록이 있어?",
            "원인도 확인되는가?",
            "원인을 구분하여 말해줘.",
            "한계를 함께 답해줘.",
            "불확실성을 설명해줘.",
            "인과관계도 확정된 것인가?",
        ]
        .iter()
        .any(|s| q.ends_with(s))
        {
            return Err((
                SemanticState::UnsupportedForm,
                "unsupported causal question grammar",
            ));
        }
        let records: Vec<_> = request
            .evidence
            .items
            .iter()
            .filter_map(|r| {
                accident_entity(&r.original_excerpt)
                    .filter(|entity| mentions_target(q, entity))
                    .map(|entity| (r, entity))
            })
            .collect();
        if records.len() > 1 || records.iter().any(|(r, _)| r.excerpt_truncated) {
            return Err((
                SemanticState::AmbiguousEvidence,
                "multiple/truncated accident records",
            ));
        }
        // Unparsed accident evidence cannot be treated as proof of absence.
        if records.is_empty()
            && request.evidence.items.iter().any(|r| {
                r.original_excerpt.contains("사고")
                    && r.original_excerpt
                        .strip_suffix("의 점검은 끝났지만 사고 자료는 없다.")
                        .is_none_or(|entity| {
                            entity.is_empty()
                                || entity.chars().any(char::is_whitespace)
                                || r.excerpt_truncated
                        })
            })
        {
            return Err((
                SemanticState::AmbiguousEvidence,
                "accident evidence outside supported record grammar or target",
            ));
        }
        let require_chronology = q.contains("함께")
            || q.contains("불확실성을 설명")
            || q.contains("순서와 확정 원인을 구분");
        let chronology = chronology_citations(request);
        if require_chronology && chronology.is_none() {
            return Err((
                SemanticState::AmbiguousEvidence,
                "required chronology not uniquely supported",
            ));
        }
        return Ok(Obligation::Cause {
            accident: records.first().map(|(r, _)| r.event_id),
            entity: records.first().map(|(_, s)| *s),
            chronology,
            require_chronology,
        });
    }
    if !["지시", "방향", "기록", "원문", "이동 값"]
        .iter()
        .any(|s| q.contains(s))
    {
        return Err((
            SemanticState::UnsupportedForm,
            "unsupported fact question grammar",
        ));
    }
    let ids = support(request)
        .map_err(|_| (SemanticState::AmbiguousEvidence, "fact support not unique"))?;
    let record = request
        .evidence
        .items
        .iter()
        .find(|r| ids == [r.event_id])
        .ok_or((SemanticState::AmbiguousEvidence, "fact support missing"))?;
    let fields = exact_fact(&record.original_excerpt).ok_or((
        SemanticState::UnsupportedForm,
        "unsupported fact record grammar",
    ))?;
    Ok(Obligation::Fact {
        fields,
        id: record.event_id,
    })
}
fn citation_literal(text: &str) -> Option<i64> {
    let digits = text.strip_prefix("[event:")?.strip_suffix(']')?;
    if digits.is_empty() || !digits.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    digits.parse().ok().filter(|id| *id > 0)
}
fn cause_assertion(text: &str) -> bool {
    if [
        "원인은 확정되었습니다.",
        "원인은 확인되었습니다.",
        "원인이 확인되었습니다.",
    ]
    .contains(&text)
    {
        return true;
    }
    ["이 원인이다.", "은 원인이 아니다.", " 때문에 사고가 났다."]
        .iter()
        .any(|suffix| {
            text.strip_suffix(suffix).is_some_and(|subject| {
                !subject.is_empty() && !subject.chars().any(|c| ".[]!?".contains(c))
            })
        })
}
fn parse_target_claims(answer: &str) -> Option<TargetClaims<'_>> {
    if answer == "근거가 없어 알 수 없습니다." {
        return Some(TargetClaims::NoEvidence);
    }
    if let Some((left, right)) = answer.split_once(" 하지만 ") {
        let left_known = left == "원인은 확인되지 않았다." || parse_target_claims(left).is_some();
        if left_known && cause_assertion(right) {
            return Some(TargetClaims::CauseAssertion);
        }
        return None;
    }
    let (body, id) = answer
        .rsplit_once(' ')
        .and_then(|(a, b)| citation_literal(b).map(|id| (a, id)))
        .map_or((answer, None), |(a, id)| (a, Some(id)));
    if cause_assertion(body) {
        return Some(TargetClaims::CauseAssertion);
    }
    if body == "원인은 확정되지 않았습니다." {
        return id.map(TargetClaims::Uncertain);
    }
    if let Some(fields) = exact_fact(body) {
        return id.map(|id| TargetClaims::Fact { fields, id });
    }
    let (body, uncertain) = answer
        .strip_suffix(" 원인은 확정되지 않았습니다.")
        .map_or((answer, false), |b| (b, true));
    let body = body.strip_suffix("가 기록되었습니다.")?;
    let (entity, body) = if let Some(body) = body.strip_prefix("지시 ") {
        (None, body)
    } else {
        let (entity, body) = body.split_once("의 지시 ")?;
        (Some(entity), body)
    };
    let (a, body) = body.split_once(" 뒤 실행 ")?;
    let (b, c) = body.split_once(", 이후 사고 ")?;
    Some(TargetClaims::Chronology {
        entity,
        ids: vec![
            citation_literal(a)?,
            citation_literal(b)?,
            citation_literal(c)?,
        ],
        uncertain,
    })
}
fn check_target(obligation: &Obligation<'_>, claims: &TargetClaims<'_>) -> bool {
    match (obligation, claims) {
        (
            Obligation::Fact {
                fields: a,
                id: a_id,
            },
            TargetClaims::Fact {
                fields: b,
                id: b_id,
            },
        ) => a == b && a_id == b_id,
        (
            Obligation::Cause {
                accident: None,
                require_chronology: false,
                ..
            },
            TargetClaims::NoEvidence,
        ) => true,
        (
            Obligation::Cause {
                accident: Some(a),
                require_chronology: false,
                ..
            },
            TargetClaims::Uncertain(b),
        ) => a == b,
        (
            Obligation::Cause {
                entity,
                chronology: Some(expected),
                ..
            },
            TargetClaims::Chronology {
                entity: claimed,
                ids,
                uncertain,
            },
        ) => *uncertain && expected == ids && claimed.is_none_or(|s| Some(s) == *entity),
        _ => false,
    }
}
fn target_semantics(request: &ModelRequest, answer: &str) -> (SemanticState, &'static str) {
    let obligation = match derive_obligation(request) {
        Ok(o) => o,
        Err(e) => return e,
    };
    let Some(claims) = parse_target_claims(answer) else {
        return (
            SemanticState::UnsupportedForm,
            "unsupported complete target grammar",
        );
    };
    if check_target(&obligation, &claims) {
        (
            SemanticState::Validated,
            "facts/citation/order/uncertainty match",
        )
    } else {
        (
            SemanticState::Contradicted,
            "target claim contradicts evidence or required facts/order/uncertainty",
        )
    }
}
fn audit_status(reports: &[&Value], overlap: &[String]) -> &'static str {
    if !overlap.is_empty()
        || reports.iter().any(|r| {
            r["contradicted"].as_u64().unwrap_or(0) > 0
                || [
                    "prompt_target_contradictions",
                    "train_generation_prefix_mismatches",
                ]
                .iter()
                .any(|k| r[*k].as_array().is_some_and(|a| !a.is_empty()))
        })
    {
        "INTEGRITY_FAIL"
    } else if reports.iter().any(|r| {
        r["unsupported"].as_u64().unwrap_or(0) > 0 || r["ambiguous"].as_u64().unwrap_or(0) > 0
    }) {
        "AUDIT_INCOMPLETE"
    } else {
        "CHECKED_BOUNDARIES_PASS"
    }
}
fn scan(episodes: &[Episode], l: &Loaded, limit: usize) -> Result<Value> {
    let mut prompts: BTreeMap<String, (String, String)> = BTreeMap::new();
    let mut collisions = Vec::new();
    let mut invalid = Vec::new();
    let mut prefix_mismatch = Vec::new();
    let mut categories: BTreeMap<usize, usize> = BTreeMap::new();
    let mut starts: BTreeMap<u32, usize> = BTreeMap::new();
    let mut positions: BTreeMap<usize, usize> = BTreeMap::new();
    let mut lengths = BTreeMap::new();
    let mut digits = BTreeMap::new();
    let mut scenes = BTreeSet::new();
    let mut questions = BTreeSet::new();
    let mut question_forms = BTreeSet::new();
    let mut values = BTreeSet::new();
    let mut value_combinations = BTreeSet::new();
    let mut entity_digit_lengths = BTreeMap::new();
    let mut orders = BTreeSet::new();
    let mut semantic_counts = [0usize; 5];
    let (
        mut checked,
        mut auxiliary,
        mut no_evidence,
        mut max_prompt,
        mut over_window,
        mut target_count,
    ) = (0, 0, 0, 0, 0, 0);
    for e in episodes.iter().take(limit) {
        *categories.entry(e.category).or_default() += 1;
        let (semantic, reason) = if e.family.starts_with("copy/") {
            (SemanticState::OutOfScope, "predeclared auxiliary task")
        } else {
            target_semantics(&e.request, &e.answer)
        };
        semantic_counts[match semantic {
            SemanticState::Validated => 0,
            SemanticState::Contradicted => 1,
            SemanticState::UnsupportedForm => 2,
            SemanticState::AmbiguousEvidence => 3,
            SemanticState::OutOfScope => 4,
        }] += 1;
        if !matches!(
            semantic,
            SemanticState::Validated | SemanticState::OutOfScope
        ) {
            invalid.push(json!({"id":e.id,"status":semantic,"reason":reason}));
        }
        let s = samples(std::slice::from_ref(e), &l.tokenizer, 512)?.remove(0);
        let p = l.tokenizer.prepare(
            &e.request,
            l.model.config.context as u32,
            &l.model.config.id()?,
        )?;
        max_prompt = max_prompt.max(p.token_ids.len());
        over_window += usize::from(p.token_ids.len() > 256);
        if s.tokens[..s.response_start] != p.token_ids {
            prefix_mismatch.push(e.id.clone());
        }
        let key = digest(&p.token_ids)?;
        if let Some((old_answer, old_id)) = prompts.get(&key) {
            if old_answer != &e.answer {
                collisions.push(json!({"a":old_id,"b":e.id,"a_answer":old_answer,"b_answer":e.answer,"prompt":key}));
            }
        } else {
            prompts.insert(key, (e.answer.clone(), e.id.clone()));
        }
        let target = &s.tokens[s.response_start..];
        target_count += target.len();
        *starts.entry(target[0]).or_default() += 1;
        *lengths.entry(target.len()).or_insert(0usize) += 1;
        scenes.insert(scene(e).to_string());
        questions.insert(e.request.input.clone());
        // Count surface forms separately from exact questions: each ASCII digit run
        // is replaced by one '#'. This is descriptive only, never model input.
        let mut form = String::new();
        let mut in_digits = false;
        for c in e.request.input.chars() {
            if !c.is_ascii_digit() {
                form.push(c);
            } else if !in_digits {
                form.push('#');
            }
            in_digits = c.is_ascii_digit();
        }
        question_forms.insert(form);
        orders.insert(digest(
            &e.request
                .evidence
                .items
                .iter()
                .map(|r| r.event_id)
                .collect::<Vec<_>>(),
        )?);
        let mut combination = Vec::new();
        for r in &e.request.evidence.items {
            if let Some((entity, _, v)) = fields(&r.original_excerpt) {
                values.insert(v.to_string());
                combination.push(v.to_string());
                *entity_digit_lengths
                    .entry(entity.chars().filter(char::is_ascii_digit).count())
                    .or_insert(0usize) += 1;
            }
            *digits.entry(r.event_id.to_string().len()).or_insert(0usize) += 1;
        }
        combination.sort();
        value_combinations.insert(combination);
        if e.family.starts_with("copy/") {
            auxiliary += 1;
            continue;
        }
        checked += 1;
        let support = support(&e.request).unwrap_or_default(); // Distribution only, never approval.
        if support.is_empty() {
            no_evidence += 1;
        }
        let mut reversed = e.request.clone();
        reversed.evidence.items.reverse();
        if target_semantics(&reversed, &e.answer) != (semantic, reason) {
            return Err(Error::Corrupt(
                "semantic verdict changes with evidence permutation".into(),
            ));
        }
        if let Some(id) = support.first()
            && let Some(position) = e
                .request
                .evidence
                .items
                .iter()
                .position(|r| r.event_id == *id)
        {
            *positions.entry(position).or_default() += 1;
        }
    }
    let mut report = json!({"scanned":episodes.len().min(limit),"total":episodes.len(),"independent_full_qa_checked":checked,"auxiliary_semantics_not_checked":auxiliary,"prompt_target_contradictions":collisions,"data_ambiguities":invalid,"train_generation_prefix_mismatches":prefix_mismatch,"category_counts":categories,"first_target_counts":starts,"target_length_histogram":lengths,"supervised_tokens_including_eos":target_count,"eos_targets":episodes.len().min(limit),"record_position":positions,"citation_digit_lengths":digits,"no_evidence":no_evidence,"unique_base_ids":scenes.len(),"unique_questions":questions.len(),"unique_values":values.len(),"unique_evidence_id_orders":orders.len(),"unique_token_prompts":prompts.len(),"max_prompt_length":max_prompt,"prompts_over_window256":over_window});
    report["unique_question_forms_ascii_digit_runs_collapsed"] = json!(question_forms.len());
    report["unique_evidence_value_multisets_including_empty"] = json!(value_combinations.len());
    report["entity_digit_lengths_per_structured_evidence"] = json!(entity_digit_lengths);
    for (name, n) in [
        "validated",
        "contradicted",
        "unsupported",
        "ambiguous",
        "explicitly_out_of_scope",
    ]
    .into_iter()
    .zip(semantic_counts)
    {
        report[name] = json!(n);
    }
    report["ordinary_in_scope"] = json!(checked);
    report["semantic_findings"] = report["data_ambiguities"].clone();
    report["status"] = json!(audit_status(&[&report], &[]));
    Ok(report)
}
fn chronology_citations(request: &ModelRequest) -> Option<Vec<i64>> {
    if !(request.input.contains("원인") || request.input.contains("인과관계"))
        || request.evidence.items.len() != 3
    {
        return None;
    }
    let mut records: Vec<_> = request.evidence.items.iter().collect();
    records.sort_by_key(|r| r.recorded_at);
    let (entity, _, value) = exact_fact(&records[0].original_excerpt)?;
    if !mentions_target(&request.input, entity)
        || !records
            .iter()
            .all(|r| !r.excerpt_truncated && mentions_target(&r.original_excerpt, entity))
    {
        return None;
    }
    let execution = records[1]
        .original_excerpt
        .strip_suffix(" 지시를 실행했다.")?
        .split_once("는 ")?;
    if execution != (entity, value)
        || accident_entity(&records[2].original_excerpt) != Some(entity)
        || records
            .windows(2)
            .any(|pair| pair[0].recorded_at >= pair[1].recorded_at)
    {
        return None;
    }
    Some(records.iter().map(|r| r.event_id).collect())
}
fn audit(fixture: &Path, output: &Path, control: &mut RunControl) -> Result<()> {
    control.check("audit_started")?;
    let f = load(fixture)?;
    let (m, train, validation) = data::load(&f.corpus)?;
    let (pm, parent, pv) = data::load(&f.parent_corpus)?;
    if m.train.sha256 != f.train_hash
        || m.validation.sha256 != f.validation_hash
        || pm.train.sha256 != f.parent_train_hash
    {
        return Err(Error::Corrupt("audit corpus changed".into()));
    }
    let l = checkpoint::load(&f.start, Device::Cpu, false)?;
    control.check("audit_loaded")?;
    let u2 = scan(&train, &l, train.len())?;
    control.check("audit_u2_scanned")?;
    let original = scan(&parent, &l, 2048)?;
    control.check("audit_parent_scanned")?;
    let development = scan(&validation, &l, validation.len())?;
    control.check("audit_validation_scanned")?;
    let entities = |cases: &[Episode]| -> BTreeSet<String> {
        cases
            .iter()
            .flat_map(|e| e.request.evidence.items.iter())
            .filter_map(|r| fields(&r.original_excerpt).map(|f| f.0.to_string()))
            .collect()
    };
    let mut overlap = entities(&train)
        .intersection(&entities(&validation))
        .cloned()
        .collect::<Vec<_>>();
    overlap.extend(entities(&parent).intersection(&entities(&pv)).cloned());
    let status = audit_status(&[&u2, &original, &development], &overlap);
    let result = json!({"u2_all":u2,"parent_bounded_first2048":original,"validation400":development,"cross_split_entity_overlap":overlap,"optimizer_updates":0,"status":status,"fixture_hash":file_hash(fixture)?,"actual_train_hash":m.train.sha256,"actual_validation_hash":m.validation.sha256,"frozen_expected_validation_hash":f.validation_hash,"evaluated_validation_cases_hash":digest(&validation)?,"parent_train_hash":pm.train.sha256,"scope":"U2 all ordinary; parent first2048 and validation400; copy/* auxiliary OUT_OF_SCOPE"});
    save(output, &result)?;
    println!(
        "audit status={} output={}",
        result["status"],
        output.display()
    );
    if status != "CHECKED_BOUNDARIES_PASS" {
        control.observe(if status == "AUDIT_INCOMPLETE" {
            StopReason::AuditIncomplete
        } else {
            StopReason::IntegrityFail
        });
        Err(Error::Invalid(format!(
            "{status}; inspect preserved semantic scope and findings"
        )))
    } else {
        Ok(())
    }
}
fn compare(a: &Tensor, b: &Tensor) -> Result<Value> {
    if a.dims() != b.dims() {
        return Err(Error::Model("numeric shape mismatch".into()));
    }
    let av = a.flatten_all()?.to_vec1::<f32>()?;
    let bv = b.flatten_all()?.to_vec1::<f32>()?;
    let mut max_abs = 0f64;
    let mut violations = 0;
    for (&a, &b) in av.iter().zip(&bv) {
        let error = f64::from((a - b).abs());
        max_abs = max_abs.max(error);
        violations += usize::from(
            !a.is_finite()
                || !b.is_finite()
                || error > 1e-4 + 1e-3 * f64::from(a.abs().max(b.abs())),
        );
    }
    if violations > 0 {
        return Err(Error::Model(format!(
            "numeric parity violations={violations} max_abs={max_abs}"
        )));
    }
    Ok(json!({"max_abs":max_abs,"abs_tolerance":1e-4,"rel_tolerance":1e-3,"violations":0}))
}
fn numeric(fixture: &Path, path: &Path, output: &Path) -> Result<()> {
    let started = Instant::now();
    let f = load(fixture)?;
    let l = checkpoint::load(path, Device::Cpu, false)?;
    let before = l.model.weight_hash()?;
    let cases = [f.watch[0].clone(), f.watch[31].clone()];
    let framed = samples(&cases, &l.tokenizer, 512)?;
    let b = batch(&framed, &[0, 1], &Device::Cpu)?;
    let batched = l.model.forward(&b.input, Some(&b.valid))?;
    let mut checks = Vec::new();
    for (row, s) in framed.iter().enumerate() {
        let single = batch(&framed, &[row], &Device::Cpu)?;
        checks.push(json!({"case":cases[row].id,"alone_batch":compare(&l.model.forward(&single.input,Some(&single.valid))?,&batched.narrow(0,row,1)?.narrow(1,0,s.tokens.len()-1)?)?}));
        let prompt = &s.tokens[..s.response_start];
        for len in [prompt.len(), 255, 256, 257] {
            let ids: Vec<_> = prompt.iter().copied().cycle().take(len).collect();
            let direct = l.model.forward(
                &Tensor::new(ids.as_slice(), &Device::Cpu)?.unsqueeze(0)?,
                None,
            )?;
            for chunk in [127, 128, 129] {
                let mut cache = l.model.cache("recovery-parity");
                let mut parts = Vec::new();
                for part in ids.chunks(chunk) {
                    parts.push(l.model.forward_cached(
                        &Tensor::new(part, &Device::Cpu)?.unsqueeze(0)?,
                        &mut cache,
                        "recovery-parity",
                    )?);
                }
                let parity = compare(&direct, &Tensor::cat(&parts, 1)?)?;
                let mut extended = ids.clone();
                let next = s.tokens[s.response_start];
                extended.push(next);
                let full = l
                    .model
                    .forward(
                        &Tensor::new(extended.as_slice(), &Device::Cpu)?.unsqueeze(0)?,
                        None,
                    )?
                    .narrow(1, len, 1)?;
                let cached = l.model.forward_cached(
                    &Tensor::new(&[next], &Device::Cpu)?.unsqueeze(0)?,
                    &mut cache,
                    "recovery-parity",
                )?;
                checks.push(json!({"case":row,"length":len,"chunk":chunk,"prefill":parity,"same_prefix_next":compare(&full,&cached)?,"next_argmax_equal":full.argmax(2)?.to_vec2::<u32>()?==cached.argmax(2)?.to_vec2::<u32>()?}));
            }
            let mut changed = ids.clone();
            let at = len / 2;
            for id in &mut changed[at..] {
                *id = 8 + (*id + 1) % ((l.model.config.vocab - 8) as u32);
            }
            let modified = l.model.forward(
                &Tensor::new(changed.as_slice(), &Device::Cpu)?.unsqueeze(0)?,
                None,
            )?;
            checks.push(json!({"case":row,"length":len,"causal_future":compare(&direct.narrow(1,0,at)?,&modified.narrow(1,0,at)?)?}));
        }
    }
    // One real batch: labels/masks/EOS are independently derived from literal sample IDs.
    let (_, train, _) = data::load(&f.corpus)?;
    let s = samples(&train[..8], &l.tokenizer, 512)?;
    let b = batch(&s, &(0..8).collect::<Vec<_>>(), &Device::Cpu)?;
    let ids = b.input.to_vec2::<u32>()?;
    let target = b.target.to_vec2::<u32>()?;
    let mask = b.mask.to_vec2::<f32>()?;
    let mut count = 0;
    for (r, s) in s.iter().enumerate() {
        for p in 0..ids[r].len() {
            let valid = p + 1 < s.tokens.len();
            let m = valid && p + 1 >= s.response_start;
            if mask[r][p] != f32::from(m)
                || (valid && (ids[r][p] != s.tokens[p] || target[r][p] != s.tokens[p + 1]))
            {
                return Err(Error::Corrupt("actual shift/mask boundary".into()));
            }
            count += usize::from(m);
        }
        if s.tokens.last() != Some(&EOS) {
            return Err(Error::Corrupt("missing EOS supervision".into()));
        }
    }
    let logits = l.model.forward(&b.input, Some(&b.valid))?;
    let (ce, obj, n) = response_loss(&logits, &b, 8.)?;
    if n != count {
        return Err(Error::Corrupt("actual target denominator".into()));
    }
    let grad = obj.backward()?;
    let mut norms = BTreeMap::new();
    let mut finite_difference = Vec::new();
    for (name, var) in &l.model.vars {
        let g = grad
            .get(var)
            .ok_or_else(|| Error::Model(format!("missing gradient {name}")))?;
        let norm = g.sqr()?.sum_all()?.to_scalar::<f32>()?;
        if !norm.is_finite() {
            return Err(Error::Model("nonfinite actual gradient".into()));
        }
        norms.insert(name.clone(), norm.sqrt());
    }
    // Two selected coordinates, central difference on the actual objective; no optimizer update.
    for name in ["final_norm", "embedding"] {
        if let Some(var) = l.model.vars.get(name) {
            let g = grad.get(var).unwrap().flatten_all()?.to_vec1::<f32>()?;
            let at = g
                .iter()
                .enumerate()
                .max_by(|a, b| a.1.abs().total_cmp(&b.1.abs()))
                .unwrap()
                .0;
            let original = var.flatten_all()?.to_vec1::<f32>()?;
            let h = 0.002f32;
            let mut observed = Vec::new();
            for direction in [-1., 1.] {
                let mut values = original.clone();
                values[at] += h * direction;
                var.set(&Tensor::from_vec(values, var.dims(), &Device::Cpu)?)?;
                let result = response_loss(&l.model.forward(&b.input, Some(&b.valid))?, &b, 8.)?
                    .1
                    .to_scalar::<f32>();
                var.set(&Tensor::from_vec(
                    original.clone(),
                    var.dims(),
                    &Device::Cpu,
                )?)?;
                observed.push(result? as f64);
            }
            let reference = (observed[1] - observed[0]) / (2. * h as f64);
            let error = (reference - g[at] as f64).abs();
            finite_difference.push(json!({"parameter":name,"coordinate":at,"autograd":g[at],"central_difference":reference,"step":h,"absolute_error":error,"tolerance":"0.002 + 0.05 * max(abs(reference),abs(autograd))"}));
            if error > 0.002 + 0.05 * reference.abs().max(g[at].abs() as f64) {
                return Err(Error::Model("actual objective finite difference".into()));
            }
        }
    }
    if before != l.model.weight_hash()? {
        return Err(Error::Corrupt("numeric diagnosis mutated model".into()));
    }
    let result = json!({"model_content_hash":before,"checks":checks,"actual_batch_targets":n,"plain_ce":ce.to_scalar::<f32>()?,"weighted_objective":obj.to_scalar::<f32>()?,"gradient_norms":norms,"finite_difference":finite_difference,"optimizer_updates":0,"elapsed_seconds":started.elapsed().as_secs_f64(),"status":"CHECKED_BOUNDARIES_PASS"});
    save(output, &result)?;
    println!("numeric PASS output={}", output.display());
    Ok(())
}

fn arm_config(original: &TrainConfig, arm: &str) -> Result<TrainConfig> {
    let mut c = original.clone();
    if c.microbatch != 8
        || c.accumulation != 1
        || c.sample_group_size != 8
        || c.first_target_weight != 8.
    {
        return Err(Error::Invalid(
            "C/W requires the frozen U2 policy, no implicit policy changes".into(),
        ));
    }
    match arm {
        "C" => {}
        "W" => c.first_target_weight = 1.,
        _ => return Err(Error::Invalid("screening arm".into())),
    }
    Ok(c)
}
fn schedule_config(
    original: &TrainConfig,
    parent: &TrainConfig,
    parent_step: usize,
) -> Result<TrainConfig> {
    let mut c = arm_config(original, "C")?;
    // The entire intervention is the saved LR policy; Adam, step and tape stay intact.
    // Keep the same saved horizon, start at the parent's end-of-run LR,
    // and use the existing cosine formula without another warmup ramp.
    c.lr = parent.learning_rate(parent_step);
    c.warmup = 0;
    Ok(c)
}
fn read_json(path: &Path) -> Result<Value> {
    Ok(serde_json::from_slice(&neural::read_bounded(
        path,
        16 * 1024 * 1024,
    )?)?)
}
fn schedule_report(
    fixture: &Path,
    paths: [&Path; 2],
    factor: &str,
    output: &Path,
    budget: &mut RunControl,
) -> Result<()> {
    let f = load(fixture)?;
    let policies = paths
        .map(|p| read_json(&p.join("policy.json")))
        .into_iter()
        .collect::<Result<Vec<_>>>()?;
    let traces = paths
        .map(|p| trace(&p.join("trace.jsonl")))
        .into_iter()
        .collect::<Result<Vec<_>>>()?;
    let results = paths
        .map(|p| read_json(&p.join("result.json")))
        .into_iter()
        .collect::<Result<Vec<_>>>()?;
    let base: TrainConfig = serde_json::from_value(policies[0]["config"].clone())?;
    let parent: TrainingState =
        serde_json::from_value(f.registry["GENERAL_QA_PARENT"]["manifest"]["training"].clone())?;
    let group = match factor {
        "group" => true,
        "schedule" => false,
        _ => return Err(Error::Invalid("comparison factor".into())),
    };
    let treatment = if group {
        let initial: TrainingState =
            serde_json::from_value(f.registry["U2_POLICY_START"]["manifest"]["training"].clone())?;
        if base != schedule_config(&initial.config, &parent.config, parent.step)? {
            return Err(Error::Corrupt(
                "group comparison requires the recorded L policy".into(),
            ));
        }
        TrainConfig {
            sample_group_size: 1,
            ..base.clone()
        }
    } else {
        schedule_config(&base, &parent.config, parent.step)?
    };
    let fixture_hash = file_hash(fixture)?;
    if policies[0]["arm"] != if group { "L" } else { "C" }
        || policies[1]["arm"] != if group { "B" } else { "L" }
        || json!(treatment) != policies[1]["config"]
    {
        return Err(Error::Corrupt("C/L schedule-only configuration".into()));
    }
    for key in [
        "parent",
        "max_new_updates",
        "max_input_tokens",
        "max_target_tokens",
        "max_seconds",
        "max_rss_bytes",
    ] {
        if policies[0][key] != policies[1][key] {
            return Err(Error::Corrupt(format!("C/L changed {key}")));
        }
    }
    if !group {
        for key in ["tape", "tape_hash", "source_id", "binary_hash"] {
            if policies[0][key] != policies[1][key] {
                return Err(Error::Corrupt(format!("C/L changed {key}")));
            }
        }
    }
    let (manifest, episodes, _) = data::load(&f.corpus)?;
    if manifest.train.sha256 != f.train_hash {
        return Err(Error::Corrupt("comparison corpus binding".into()));
    }
    let pool: Vec<_> = (0..episodes.len()).collect();
    for ((policy, rows), result) in policies.iter().zip(&traces).zip(&results) {
        budget.check("schedule_trace")?;
        let config: TrainConfig = serde_json::from_value(policy["config"].clone())?;
        let tape: Vec<(Vec<usize>, u64)> = serde_json::from_value(policy["tape"].clone())?;
        let mut rng = Rng::new(parent.sampler_state);
        for (indices, state) in &tape {
            if draw_indices(&pool, &config, &mut rng)? != *indices || rng.state != *state {
                return Err(Error::Corrupt("comparison sampler replay".into()));
            }
        }
        if policy["fixture_hash"] != fixture_hash
            || policy["parent"] != f.registry["U2_POLICY_START"]
            || digest(&tape)? != policy["tape_hash"]
            || rows.len() > tape.len()
            || result["new_updates"] != rows.len()
            || result["cumulative_model_step"] != parent.step + rows.len()
            || result["checkpoint_saved"] != true
        {
            return Err(Error::Corrupt("C/L actual run binding".into()));
        }
        for (i, (row, (indices, rng))) in rows.iter().zip(&tape).enumerate() {
            let step = parent.step + i + 1;
            let ids = indices
                .iter()
                .map(|&index| {
                    episodes
                        .get(index)
                        .map(|e| &e.id)
                        .ok_or_else(|| Error::Corrupt("comparison sample index".into()))
                })
                .collect::<Result<Vec<_>>>()?;
            if row["indices"] != json!(indices)
                || row["ids"] != json!(ids)
                || row["sampler_state"] != json!(rng)
                || row["new_update"] != i + 1
                || row["cumulative_model_step"] != step
                || row["optimizer_step"] != step
                || row["schedule_step"] != step - config.budget_start_step
                || row["lr"]
                    .as_f64()
                    .is_none_or(|lr| (lr - config.learning_rate(step)).abs() > 1e-15)
            {
                return Err(Error::Corrupt("C/L actual clock/tape/LR".into()));
            }
        }
    }
    for (a, b) in traces[0].iter().zip(&traces[1]) {
        for key in [
            "indices",
            "ids",
            "input_tokens",
            "target_tokens",
            "sampler_state",
            "optimizer_step",
            "schedule_step",
        ] {
            if group && !["optimizer_step", "schedule_step"].contains(&key) {
                continue;
            }
            if a[key] != b[key] {
                return Err(Error::Corrupt(format!("C/L actual trace {key}")));
            }
        }
    }
    let mut comparisons = Vec::new();
    let details = |rows: &[Value]| -> Result<Value> {
        let mut score = summarize(rows)?;
        for key in [
            "entity",
            "context",
            "value",
            "citation_exact",
            "citation_in_provided",
        ] {
            let mut counts = [0usize; 2];
            for row in rows {
                if let Some(correct) = row["components"][key].as_bool() {
                    counts[0] += usize::from(correct);
                    counts[1] += 1;
                }
            }
            score["components"][key] = json!(counts);
        }
        Ok(score)
    };
    for n in [0, 10, 25, 50, 100, 150, 200, 250] {
        if n > traces[0].len().min(traces[1].len()) {
            break;
        }
        let mut scores = Vec::new();
        let mut train_scores = Vec::new();
        let mut panels: Vec<Vec<Value>> = Vec::new();
        let mut train_ids = None;
        for path in paths {
            let evaluation = read_json(&path.join(format!("eval-{n:03}.json")))?;
            if evaluation["final_evaluation_complete"] != true {
                return Err(Error::Invalid("incomplete comparison evaluation".into()));
            }
            let rows: Vec<Value> = serde_json::from_value(evaluation["watch_rows"].clone())?;
            verify_historical_rows(&f.watch, &rows)?;
            scores.push(details(&rows)?);
            panels.push(rows);
            let train_rows: Vec<Value> =
                serde_json::from_value(evaluation["train_exposure_panel"].clone())?;
            let ids = train_rows
                .iter()
                .map(|r| r["id"].clone())
                .collect::<Vec<_>>();
            if train_ids.as_ref().is_some_and(|old| old != &ids) {
                return Err(Error::Corrupt("comparison train panel changed".into()));
            }
            train_ids = Some(ids);
            train_scores.push(details(&train_rows)?);
        }
        if n == 0 {
            for (a, b) in panels[0].iter().zip(&panels[1]) {
                for key in [
                    "id",
                    "raw_tokens",
                    "actual",
                    "error",
                    "prompt_digest",
                    "provided",
                    "excluded",
                ] {
                    if a[key] != b[key] {
                        return Err(Error::Corrupt(format!(
                            "comparison initial generation differs: {key}"
                        )));
                    }
                }
            }
        }
        let comparison = json!({"new_updates":n,"control":scores[0],"treatment":scores[1],"control_train16":train_scores[0],"treatment_train16":train_scores[1]});
        println!(
            "step={n} control_watch={} treatment_watch={} control_train={} treatment_train={} control_components={} treatment_components={}",
            scores[0]["exact_matches"],
            scores[1]["exact_matches"],
            train_scores[0]["exact_matches"],
            train_scores[1]["exact_matches"],
            scores[0]["components"],
            scores[1]["components"]
        );
        comparisons.push(comparison);
    }
    budget.check("schedule_report_complete")?;
    save(
        output,
        &json!({"status":"VERIFIED_COMPARISON","factor":factor,"fixture_hash":fixture_hash,"source_ids":policies.iter().map(|p|&p["source_id"]).collect::<Vec<_>>(),"binary_hashes":policies.iter().map(|p|&p["binary_hash"]).collect::<Vec<_>>(),"same_tape":!group,"same_optimizer_clocks":true,"control_reused":group,"comparisons":comparisons,"runs":results,"recorded_small_optimizer_updates":traces.iter().map(Vec::len).sum::<usize>(),"new_small_optimizer_updates":0,"model_calls":0,"final_heldout":false,"goal1_ready":false}),
    )?;
    println!("schedule comparison saved: {}", output.display());
    Ok(())
}
fn trace(path: &Path) -> Result<Vec<Value>> {
    neural::read_bounded(path, 16 * 1024 * 1024)?
        .split(|&b| b == b'\n')
        .filter(|b| !b.is_empty())
        .map(|b| Ok(serde_json::from_slice(b)?))
        .collect()
}
fn worker_receipt(
    binary: &Path,
    checkpoint: &Path,
    request: &ModelRequest,
    budget: &mut RunControl,
) -> Result<(bool, Value, String)> {
    use std::{
        io::Read,
        process::{Command, Stdio},
        time::Duration,
    };
    budget.check("worker_started")?;
    let mut child = Command::new(binary)
        .args(["__model-worker", "--checkpoint"])
        .arg(checkpoint)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;
    let stdout = child.stdout.take().unwrap();
    let stderr = child.stderr.take().unwrap();
    let out = std::thread::spawn(move || {
        let mut bytes = Vec::new();
        stdout
            .take((replica_v3::model::MAX_RESPONSE * 2) as u64)
            .read_to_end(&mut bytes)
            .map(|_| bytes)
    });
    let err = std::thread::spawn(move || {
        let mut bytes = Vec::new();
        stderr
            .take(replica_v3::model::MAX_STDERR as u64)
            .read_to_end(&mut bytes)
            .map(|_| bytes)
    });
    let write = replica_v3::model::write_frame(
        &mut child.stdin.take().unwrap(),
        request,
        replica_v3::model::MAX_REQUEST,
    );
    if write.is_err() {
        child.kill()?;
        let _ = child.wait();
        return Err(Error::Model("worker input write failed".into()));
    }
    let started = Instant::now();
    let status = loop {
        if let Err(error) = budget.check("worker_poll") {
            let _ = child.kill();
            let _ = child.wait();
            return Err(error);
        }
        if let Some(status) = child.try_wait()? {
            break status;
        }
        if started.elapsed()
            > replica_v3::model::LOAD_TIMEOUT
                + Duration::from_millis(
                    request
                        .limits
                        .timeout_ms
                        .checked_add(1000)
                        .ok_or_else(|| Error::Invalid("worker timeout overflow".into()))?,
                )
        {
            child.kill()?;
            let _ = child.wait();
            return Err(Error::Model("worker diagnostic deadline".into()));
        }
        std::thread::sleep(Duration::from_millis(10));
    };
    budget.check("worker_returned")?;
    let bytes = out
        .join()
        .map_err(|_| Error::Model("worker reader panic".into()))??;
    let error = String::from_utf8(
        err.join()
            .map_err(|_| Error::Model("worker stderr reader panic".into()))??,
    )
    .map_err(|_| Error::Model("worker stderr encoding".into()))?;
    let mut cursor = std::io::Cursor::new(bytes);
    let ready: Value = replica_v3::model::read_frame(&mut cursor, replica_v3::model::MAX_RESPONSE)?;
    if ready["ready"] != true {
        return Err(Error::Model("native worker not ready".into()));
    }
    let response = if status.success() {
        replica_v3::model::read_frame(&mut cursor, replica_v3::model::MAX_RESPONSE)?
    } else {
        Value::Null
    };
    Ok((status.success(), response, error))
}
#[allow(clippy::too_many_arguments)] // Existing CLI inputs plus the shared command budget.
fn close(
    fixture: &Path,
    control: &Path,
    treatment: &Path,
    legacy: &Path,
    worker: &Path,
    source_id: &str,
    output: &Path,
    budget: &mut RunControl,
) -> Result<()> {
    budget.check("close_started")?;
    let f = load(fixture)?;
    let cp = read_json(&control.join("policy.json"))?;
    let wp = read_json(&treatment.join("policy.json"))?;
    let c = trace(&control.join("trace.jsonl"))?;
    let w = trace(&treatment.join("trace.jsonl"))?;
    let mut config = cp["config"].clone();
    config["first_target_weight"] = json!(1.);
    if config != wp["config"]
        || cp["parent"] != wp["parent"]
        || cp["tape"] != wp["tape"]
        || cp["source_id"] != wp["source_id"]
        || cp["binary_hash"] != wp["binary_hash"]
        || c.len() != 50
        || w.len() != 50
    {
        return Err(Error::Corrupt(
            "one-factor policy/tape budget mismatch".into(),
        ));
    }
    let keys = [
        "new_update",
        "cumulative_model_step",
        "optimizer_step",
        "schedule_step",
        "lr",
        "indices",
        "ids",
        "sampler_state",
        "input_tokens",
        "target_tokens",
    ];
    for (i, (a, b)) in c.iter().zip(&w).enumerate() {
        if keys.iter().any(|k| a[*k] != b[*k])
            || a["new_update"] != i + 1
            || a["first_target_weight"] != 8.
            || b["first_target_weight"] != 1.
        {
            return Err(Error::Corrupt("actual trace one-factor mismatch".into()));
        }
    }
    std::fs::create_dir(output)?;
    let tok = ByteBpe::load(legacy)?;
    let mut reports = Vec::new();
    let mut worker_checks = Vec::new();
    for (name, directory) in [("C", control), ("W", treatment)] {
        budget.check("close_next_artifact")?;
        let checkpoint = directory.join("final");
        let loaded = checkpoint::load(&checkpoint, Device::Cpu, false)?;
        budget.check("close_loaded")?;
        if loaded.tokenizer.semantic_id() != tok.semantic_id() {
            return Err(Error::Corrupt("legacy/native tokenizer mapping".into()));
        }
        let replay_path = output.join(format!("{name}-fresh.jsonl"));
        replay(
            fixture,
            &checkpoint,
            &replay_path,
            "watch",
            Some(source_id),
            budget,
            false,
            None,
        )?;
        let (_, actual) = rows(&replay_path)?;
        let scored = read_json(&directory.join("eval-050.json"))?;
        for row in &actual {
            let before = scored["watch_rows"]
                .as_array()
                .unwrap()
                .iter()
                .find(|r| r["id"] == row["id"])
                .ok_or_else(|| Error::Corrupt("fresh case missing".into()))?;
            if [
                "actual",
                "raw_tokens",
                "error",
                "prompt_digest",
                "exact_match",
            ]
            .iter()
            .any(|k| row[*k] != before[*k])
            {
                return Err(Error::Corrupt("fresh-process generation mismatch".into()));
            }
            let ids: Vec<u32> = serde_json::from_value(row["raw_tokens"].clone())?;
            let ids: Vec<_> = ids
                .into_iter()
                .take_while(|&id| id >= neural::SPECIALS as u32)
                .collect();
            if tok.decode_bytes(&ids)? != loaded.tokenizer.decode_bytes(&ids)? {
                return Err(Error::Corrupt("generated byte mapping mismatch".into()));
            }
        }
        let case = &f.watch[0];
        let direct = evaluate_one(&loaded, case, &case.request, budget);
        save(&output.join(format!("{name}-direct.json")), &direct)?;
        budget.check("close_direct_returned")?;
        worker_checks.push(check_worker(worker, &checkpoint, case, &direct, budget)?);
        let mut stats = summarize(&actual)?;
        let mut fields: BTreeMap<String, [usize; 2]> = BTreeMap::new();
        for row in &actual {
            for key in [
                "entity",
                "context",
                "value",
                "citation_exact",
                "citation_in_provided",
            ] {
                if let Some(correct) = row["components"][key].as_bool() {
                    let n = fields.entry(key.into()).or_default();
                    n[0] += usize::from(correct);
                    n[1] += 1;
                }
            }
        }
        stats["components"] = json!(fields);
        reports.push(json!({"arm":name,"result":read_json(&directory.join("result.json"))?,"fresh_watch":stats,"physical_hash":file_hash(&checkpoint)?,"model_tensor_content_digest":loaded.model.weights_content_id()?,"legacy_architecture_weight_hash":loaded.model.weight_hash()?,"clock":loaded.manifest.training,"fresh_outputs_identical":true}));
    }
    let failed_path = PathBuf::from(
        f.registry["U2_AFTER_250"]["path"]
            .as_str()
            .ok_or_else(|| Error::Corrupt("failed path".into()))?,
    );
    budget.check("close_before_failed_load")?;
    let failed = checkpoint::load(&failed_path, Device::Cpu, false)?;
    budget.check("close_failed_loaded")?;
    for kind in ["utf8", "empty"] {
        let e = f
            .failures
            .iter()
            .find(|e| {
                f.previous_failed.iter().any(|r| {
                    r["id"] == e.id
                        && if kind == "utf8" {
                            r["error"].as_str().is_some_and(|e| e.contains("UTF-8"))
                        } else {
                            r["actual"] == ""
                        }
                })
            })
            .ok_or_else(|| Error::Corrupt("frozen failure coverage".into()))?;
        let direct = evaluate_one(&failed, e, &e.request, budget);
        save(&output.join(format!("failed-{kind}-direct.json")), &direct)?;
        budget.check("close_failure_returned")?;
        let ids: Vec<u32> = serde_json::from_value(direct["raw_tokens"].clone())?;
        let ids: Vec<_> = ids
            .into_iter()
            .take_while(|&id| id >= neural::SPECIALS as u32)
            .collect();
        if tok.decode_bytes(&ids)? != failed.tokenizer.decode_bytes(&ids)? {
            return Err(Error::Corrupt(
                "failed legacy/native mapping mismatch".into(),
            ));
        }
        worker_checks.push(check_worker(worker, &failed_path, e, &direct, budget)?);
    }
    let base = read_json(&control.join("eval-000.json"))?["watch"]["exact_matches"]
        .as_u64()
        .ok_or_else(|| Error::Corrupt("baseline score".into()))?;
    let mut streak = 0;
    let mut regression = false;
    for n in [10, 25, 50] {
        let evaluation = read_json(&control.join(format!("eval-{n:03}.json")))?;
        let count = evaluation["watch"]["exact_matches"]
            .as_u64()
            .ok_or_else(|| Error::Corrupt("watch count".into()))?;
        let bad = base.saturating_sub(count) >= 4
            || evaluation["new_error_cases"].as_u64().unwrap_or(0) >= 2;
        streak = if bad { streak + 1 } else { 0 };
        regression |= streak >= 2;
    }
    let cs = &reports[0]["fresh_watch"];
    let ws = &reports[1]["fresh_watch"];
    let eligible = ws["exact_matches"]
        .as_u64()
        .is_some_and(|n| n >= base && n > cs["exact_matches"].as_u64().unwrap_or(0))
        && ws["generation_failures"].as_u64().unwrap_or(u64::MAX) == 0
        && ws["empty"].as_u64().unwrap_or(u64::MAX) == 0;
    let mut summary = json!({"one_factor_actual_trace_verified":true,"optimizer_updates_small":c.len()+w.len(),"same_sample_multiset_and_order":true,"same_clocks_and_lr":true,"reports":reports,"product_worker":worker_checks,"tokenizer_native_legacy_mapping":"PASS","candidate_eligible":eligible,"confirmation":if eligible{"REQUIRED_NOT_RUN"}else{"NOT_RUN_NO_SCREENING_EFFECT"},"regression_within_50":if regression{"REPRODUCED_ON_WATCH"}else{"NOT_REPRODUCED_WITHIN_BUDGET"},"s4_quality":"NOT_EVALUATED_HERE","goal1_ready":false});
    budget.check("close_before_terminal")?;
    save(&output.join("evaluations.json"), &summary)?;
    let sealed = budget.seal_terminal();
    summary["final_evaluation_complete"] = json!(sealed.is_ok());
    summary["comparison_eligible"] = json!(sealed.is_ok());
    summary["control"] = budget.receipt();
    if sealed.is_err() {
        summary["candidate_eligible"] = json!(false);
    }
    save(&output.join("summary.json"), &summary)?;
    println!(
        "closure report={} SMALL_updates={}",
        output.display(),
        c.len() + w.len()
    );
    sealed
}

fn verify_historical_rows(cases: &[Episode], rows: &[Value]) -> Result<()> {
    if rows.len() != cases.len() {
        return Err(Error::Corrupt("historical missing/count mismatch".into()));
    }
    let expected: BTreeMap<_, _> = cases.iter().map(|e| (e.id.as_str(), e)).collect();
    let mut seen = BTreeSet::new();
    for row in rows {
        let id = row["id"]
            .as_str()
            .ok_or_else(|| Error::Corrupt("historical ID".into()))?;
        let e = expected
            .get(id)
            .ok_or_else(|| Error::Corrupt("historical unknown ID".into()))?;
        if !seen.insert(id)
            || row["question"] != e.request.input
            || row["expected"] != e.answer
            || row["evidence"] != serde_json::to_value(&e.request.evidence)?
            || row["generated_question"] != e.request.input
            || row["generated_evidence"] != serde_json::to_value(&e.request.evidence)?
        {
            return Err(Error::Corrupt(
                "historical duplicate/content binding".into(),
            ));
        }
    }
    Ok(())
}
fn recount(
    fixture: &Path,
    logs: [&Path; 2],
    arms: [&Path; 2],
    audit_path: &Path,
    output: &Path,
    control: &mut RunControl,
) -> Result<()> {
    control.check("recount_started")?;
    let f = load(fixture)?;
    let (validation, binding) = replay_cases(&f, "all")?;
    let (manifest, train, _) = data::load(&f.corpus)?;
    if manifest.train.sha256 != f.train_hash {
        return Err(Error::Corrupt("recount train binding".into()));
    }
    let audit = read_json(audit_path)?;
    let fixture_hash = file_hash(fixture)?;
    if audit["fixture_hash"] != fixture_hash || audit["actual_validation_hash"] != f.validation_hash
    {
        return Err(Error::Corrupt("audit/recount binding".into()));
    }
    let mut historical = Vec::new();
    for ((path, label), raw_key) in logs
        .into_iter()
        .zip(["U2_POLICY_START", "U2_AFTER_250"])
        .zip(["parent_raw_hash", "failed_raw_hash"])
    {
        control.check("recount_next_log")?;
        let (header, cases) = rows(path)?;
        if file_hash(path)? != f.registry[raw_key]
            || header["split_sha256"] != f.validation_hash
            || header["checkpoint_sha256"] != f.registry[label]["manifest"]["weights_sha256"]
        {
            return Err(Error::Corrupt(
                "historical raw/split/checkpoint binding".into(),
            ));
        }
        verify_historical_rows(&validation, &cases)?;
        historical.push(json!({"artifact":label,"raw_hash":file_hash(path)?,"score":summarize(&cases)?,"score_interpretation":"SCORE_AGAINST_FROZEN_LABELS","missing_error_generation_receipts":cases.iter().filter(|r|!r["error"].is_null()&&r["generation"].is_null()).count(),"missing_receipt_finish":"UNKNOWN"}));
    }
    let policies = arms
        .map(|p| read_json(&p.join("policy.json")))
        .into_iter()
        .collect::<Result<Vec<_>>>()?;
    let traces = arms
        .map(|p| trace(&p.join("trace.jsonl")))
        .into_iter()
        .collect::<Result<Vec<_>>>()?;
    let mut expected_w = policies[0]["config"].clone();
    expected_w["first_target_weight"] = json!(1.);
    if expected_w != policies[1]["config"]
        || policies[0]["config"]["first_target_weight"] != 8.
        || ["parent", "tape", "tape_hash", "source_id", "binary_hash"]
            .iter()
            .any(|k| policies[0][*k] != policies[1][*k])
    {
        return Err(Error::Corrupt("historical C/W one-factor binding".into()));
    }
    let mut arm_reports = Vec::new();
    for ((directory, policy), trace) in arms.iter().zip(&policies).zip(&traces) {
        control.check("recount_next_arm")?;
        let config: TrainConfig = serde_json::from_value(policy["config"].clone())?;
        let tape: Vec<(Vec<usize>, u64)> = serde_json::from_value(policy["tape"].clone())?;
        let parent_step = policy["parent"]["cumulative_model_step"]
            .as_u64()
            .ok_or_else(|| Error::Corrupt("policy parent step".into()))?
            as usize;
        if policy["fixture_hash"] != fixture_hash
            || policy["parent"] != f.registry["U2_POLICY_START"]
            || digest(&tape)? != policy["tape_hash"]
            || trace.len() != tape.len()
        {
            return Err(Error::Corrupt(
                "historical arm fixture/tape/parent binding".into(),
            ));
        }
        for (i, (row, (indices, rng))) in trace.iter().zip(&tape).enumerate() {
            let ids = indices
                .iter()
                .map(|&n| {
                    train
                        .get(n)
                        .map(|e| &e.id)
                        .ok_or_else(|| Error::Corrupt("trace sample index".into()))
                })
                .collect::<Result<Vec<_>>>()?;
            if row["indices"] != json!(indices)
                || row["ids"] != json!(ids)
                || row["sampler_state"] != json!(rng)
                || row["new_update"] != i + 1
                || row["cumulative_model_step"] != parent_step + i + 1
                || row["optimizer_step"] != parent_step + i + 1
                || row["schedule_step"] != parent_step + i + 1 - config.budget_start_step
                || row["lr"]
                    .as_f64()
                    .is_none_or(|lr| (lr - config.learning_rate(parent_step + i + 1)).abs() > 1e-15)
            {
                return Err(Error::Corrupt("historical actual trace contract".into()));
            }
        }
        let evaluation = read_json(&directory.join("eval-050.json"))?;
        let scored: Vec<Value> = serde_json::from_value(evaluation["watch_rows"].clone())?;
        verify_historical_rows(&f.watch, &scored)?;
        let score = summarize(&scored)?;
        for k in ["denominator", "exact_matches", "generation_failures"] {
            if score[k] != evaluation["watch"][k] {
                return Err(Error::Corrupt("historical watch recount".into()));
            }
        }
        arm_reports.push(json!({"arm":policy["arm"],"trace_hash":file_hash(&directory.join("trace.jsonl"))?,"historical_updates":trace.len(),"watch":score,"fixture_bound":true,"policy_bound":true}));
    }
    for (a, b) in traces[0].iter().zip(&traces[1]) {
        for k in [
            "indices",
            "ids",
            "input_tokens",
            "target_tokens",
            "lr",
            "optimizer_step",
            "schedule_step",
        ] {
            if a[k] != b[k] {
                return Err(Error::Corrupt("C/W actual trace mismatch".into()));
            }
        }
    }
    control.check("recount_terminal")?;
    save(
        output,
        &json!({"status":"PASS","binding":binding,"fixture_hash":fixture_hash,"historical":historical,"arms":arm_reports,"data_audit":audit["status"],"label_findings":audit["validation400"]["semantic_findings"],"new_small_optimizer_updates":0,"model_calls":control.generation_calls,"final_heldout":false}),
    )?;
    println!("recount PASS: {}", output.display());
    Ok(())
}
fn check_worker(
    binary: &Path,
    path: &Path,
    e: &Episode,
    direct: &Value,
    budget: &mut RunControl,
) -> Result<Value> {
    let (success, response, error) = worker_receipt(binary, path, &e.request, budget)?;
    let expected_success = direct["error"].is_null()
        && direct["generation"]["finish"] == "stop"
        && direct["actual"].as_str().is_some_and(|s| !s.is_empty());
    if success != expected_success
        || (success
            && (response["text"] != direct["actual"]
                || response["provided"] != direct["provided"]
                || response["excluded"] != direct["excluded"]
                || response["prepared"]["token_digest"] != direct["native_prompt_digest"]))
    {
        return Err(Error::Model("product/direct generation differs".into()));
    }
    if !success {
        let expected_error = if direct["generation"]["finish"] == "length" {
            "token limit before EOS"
        } else if direct["error_class"] == "strict_utf8" {
            "invalid/incomplete output UTF-8"
        } else if direct["actual"] == "" {
            "empty native generation"
        } else {
            direct["error"].as_str().unwrap_or("UNKNOWN")
        };
        if !error.contains(expected_error) {
            return Err(Error::Model(
                "worker/direct error classification differs".into(),
            ));
        }
    }
    Ok(
        json!({"id":e.id,"success":success,"direct_worker_parity":true,"worker_error":error.trim(),"raw_bytes":direct["raw_bytes"],"raw_tokens":direct["raw_tokens"],"finish_reason":direct["finish_reason"]}),
    )
}
fn save_arm(
    l: &mut Loaded,
    state: &TrainingState,
    adam: &Adam,
    output: &Path,
    reason: &str,
) -> Result<()> {
    l.model.refresh_identity()?;
    let mut m = l.manifest.clone();
    m.training = Some(state.clone());
    // Native format/status vocabulary is frozen; the detailed diagnostic reason
    // belongs to result.json, not a new artifact schema or an invented success state.
    m.status = match reason {
        "RECOVERY_SCREENING" => "TRAINING",
        "CANCELLED" => "CANCELLED",
        "RESOURCE_LIMIT" => "RESOURCE_LIMIT",
        "SCREENING_BUDGET_REACHED" | "TOKEN_BUDGET" | "TIME_BUDGET" => "BUDGET_EXHAUSTED",
        "QUALITY_GUARD" | "INTEGRITY_FAIL" | "RESOURCE_OBSERVATION_FAILED" | "AUDIT_INCOMPLETE" => {
            "DIAGNOSTIC_COMPLETE"
        }
        _ => return Err(Error::Invalid("unknown recovery termination".into())),
    }
    .into();
    m.diagnostic_only = true;
    checkpoint::save(output, &l.model, &l.tokenizer, m, &adam.moments)?;
    Ok(())
}
fn arm_evaluation(
    l: &Loaded,
    watch: &[Episode],
    train: &[Episode],
    control: &mut RunControl,
) -> Result<Value> {
    let rows = evaluate_panel(l, watch, control);
    let mut score = summarize(&rows)?;
    add_partial_counts(&mut score, &rows, watch.len(), control);
    let train_rows = if control.stop.is_none() {
        evaluate_panel(l, train, control)
    } else {
        vec![]
    };
    let mut evaluation = json!({"watch":score,"watch_rows":rows,"train_exposure_panel":train_rows});
    let all: Vec<_> = rows.iter().chain(&train_rows).cloned().collect();
    add_partial_counts(&mut evaluation, &all, watch.len() + train.len(), control);
    Ok(evaluation)
}
fn finish_arm(
    control: &mut RunControl,
    complete: bool,
    save_checkpoint: impl FnOnce(&str) -> Result<()>,
) -> Value {
    let _ = control.check("before_checkpoint_preservation");
    if control.stop.is_none() && !complete {
        control.observe(StopReason::IntegrityFail);
    }
    let work_elapsed = control.now().duration_since(control.start).as_secs_f64();
    let cleanup_start = Instant::now();
    let saved_reason = control
        .stop
        .map_or("SCREENING_BUDGET_REACHED", StopReason::name);
    let saved = save_checkpoint(saved_reason); // Only consistent checkpoint/log preservation is allowed after stop.
    if let Err(error) = &saved {
        control.classify_error(error);
    }
    let _ = control.check("checkpoint_preserved");
    let reason = control.terminal_reason(complete && saved.is_ok());
    json!({"reason":reason,"observed_conditions":control.observed,"checkpoint_saved":saved.is_ok(),"checkpoint_save_status_reason":saved_reason,"save_error":saved.err().map(|e|e.to_string()),
        "work_elapsed_seconds":work_elapsed,"cleanup_elapsed_seconds":cleanup_start.elapsed().as_secs_f64(),"work_deadline_overrun_seconds":control.now().saturating_duration_since(control.deadline).as_secs_f64(),
        "final_evaluation_complete":complete,"comparison_eligible":complete&&control.stop.is_none(),"candidate_eligible":false,"cooperative_only":true})
}
fn arm_run(
    fixture: &Path,
    arm: &str,
    max_updates: usize,
    source_id: &str,
    output: &Path,
    control: &mut RunControl,
) -> Result<()> {
    control.check("arm_started")?;
    if max_updates == 0 || max_updates > 250 || (arm == "W" && max_updates > 50) {
        return Err(Error::Invalid("explicit screening update budget".into()));
    }
    if source_id.len() != 64 || !source_id.bytes().all(|b| b.is_ascii_hexdigit()) {
        return Err(Error::Invalid("frozen source digest required".into()));
    }
    let f = load(fixture)?;
    if f.registry["U2_POLICY_START"]["backend"] != neural::cpu_backend() {
        return Err(Error::Invalid("frozen training backend mismatch".into()));
    }
    let mut l = checkpoint::load(&f.start, Device::Cpu, true)?;
    control.check("arm_loaded")?;
    if l.model.weight_hash()? != f.registry["U2_POLICY_START"]["model_content_hash"]
        || file_hash(&f.start)? != f.registry["U2_POLICY_START"]["physical_hash"]
    {
        return Err(Error::Corrupt("arm parent changed".into()));
    }
    let (manifest, episodes, _) = data::load(&f.corpus)?;
    control.check("arm_corpus_loaded")?;
    if manifest.train.sha256 != f.train_hash || manifest.validation.sha256 != f.validation_hash {
        return Err(Error::Corrupt("arm corpus changed".into()));
    }
    let mut state = l
        .manifest
        .training
        .clone()
        .ok_or_else(|| Error::Invalid("arm optimizer required".into()))?;
    let starting_config = state.config.clone();
    let c = if matches!(arm, "L" | "B") {
        let parent: TrainingState = serde_json::from_value(
            f.registry["GENERAL_QA_PARENT"]["manifest"]["training"].clone(),
        )?;
        if parent.step != state.step {
            return Err(Error::Corrupt("LR comparison parent clock".into()));
        }
        let mut c = schedule_config(&state.config, &parent.config, parent.step)?;
        if arm == "B" {
            c.sample_group_size = 1;
        }
        c
    } else {
        arm_config(&state.config, arm)?
    };
    c.validate(l.model.config.context)?;
    if state
        .step
        .checked_add(max_updates)
        .is_none_or(|end| end > c.max_steps)
    {
        return Err(Error::Invalid(
            "screening exceeds saved schedule horizon".into(),
        ));
    }
    let max_input_tokens = max_updates as u64 * 4000;
    let max_target_tokens = max_updates as u64 * 1000;
    let start_step = state.step;
    let start_input = state.consumed_tokens;
    let start_targets = state.target_tokens;
    state.config = c.clone();
    l.manifest.source_id = source_id.into();
    let s = samples(&episodes, &l.tokenizer, c.seq_len)?;
    control.check("arm_prepared")?;
    let pool: Vec<_> = (0..s.len()).collect();
    let mut rng = neural::transformer::Rng {
        state: state.sampler_state,
    };
    let mut tape = Vec::new();
    for _ in 0..max_updates {
        let ids = draw_indices(&pool, &c, &mut rng)?;
        tape.push((ids, rng.state));
    }
    let mut seen = BTreeSet::new();
    // Keep the declared train panel identical even when the sampling policy changes.
    let mut panel_rng = Rng {
        state: state.sampler_state,
    };
    let mut selected = Vec::new();
    for _ in 0..max_updates {
        for index in draw_indices(&pool, &starting_config, &mut panel_rng)? {
            if seen.insert(index) && selected.len() < 16 {
                selected.push(index);
            }
        }
        if selected.len() == 16 {
            break;
        }
    }
    let mut adam = Adam {
        moments: std::mem::take(&mut l.optimizer),
    };
    std::fs::create_dir(output)?;
    save(
        &output.join("policy.json"),
        &json!({"arm":arm,"parent":f.registry["U2_POLICY_START"],"fixture_hash":file_hash(fixture)?,"source_id":source_id,"binary_hash":file_hash(&std::env::current_exe()?)?,"tape":tape,"tape_hash":digest(&tape)?,"config":c,"max_new_updates":max_updates,"max_input_tokens":max_input_tokens,"max_target_tokens":max_target_tokens,"max_seconds":900,"max_rss_bytes":17179869184u64,"clock_policy":if matches!(arm,"L"|"B"){"moments + cumulative optimizer clock retained; parent endpoint LR, no warmup, same cosine horizon"}else{"moments + cumulative optimizer clock retained; saved U2 schedule unchanged"}}),
    )?;
    let mut log = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(output.join("trace.jsonl"))?;
    let mut base_score = 0;
    let mut base_errors = BTreeSet::new();
    let mut bad_streak = 0;
    let mut final_evaluation_complete = false;
    let mut last_evaluation = Value::Null;
    let train_panel: Vec<_> = selected.iter().map(|&i| episodes[i].clone()).collect();
    let mut exposure = BTreeMap::<usize, usize>::new();
    let outcome = (|| -> Result<()> {
        for (n, entry) in tape
            .iter()
            .map(Some)
            .chain(std::iter::once(None))
            .enumerate()
        {
            control.check("arm_loop")?;
            if [0, 10, 25, 50].contains(&n) || (n > 50 && n.is_multiple_of(50)) || n == max_updates
            {
                l.manifest.training = Some(state.clone());
                l.model.refresh_identity()?;
                let mut evaluation = arm_evaluation(&l, &f.watch, &train_panel, control)?;
                let score = &evaluation["watch"];
                let errors: BTreeSet<_> = evaluation["watch_rows"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .filter(|r| !r["error"].is_null() || r["actual"] == "")
                    .map(|r| r["id"].as_str().unwrap().to_string())
                    .collect();
                let count = score["exact_matches"].as_u64().unwrap() as usize;
                if n == 0 {
                    base_score = count;
                    base_errors = errors.clone();
                }
                let new_errors = errors.difference(&base_errors).count();
                let bad = base_score.saturating_sub(count) >= 4 || new_errors >= 2;
                bad_streak = if bad { bad_streak + 1 } else { 0 };
                evaluation["new_updates"] = json!(n);
                evaluation["model_step"] = json!(state.step);
                evaluation["new_error_cases"] = json!(new_errors);
                evaluation["bad_streak"] = json!(bad_streak);
                evaluation["exposure_counts"] = json!(
                    selected
                        .iter()
                        .map(|i| (
                            episodes[*i].id.clone(),
                            exposure.get(i).copied().unwrap_or(0)
                        ))
                        .collect::<BTreeMap<_, _>>()
                );
                let _ = control.check("before_evaluation_record");
                if control.stop.is_some() {
                    evaluation["final_evaluation_complete"] = json!(false);
                    evaluation["comparison_eligible"] = json!(false);
                    evaluation["terminal_reason"] = json!(control.stop);
                }
                save(&output.join(format!("eval-{n:03}.json")), &evaluation)?;
                last_evaluation = evaluation;
                control.check("evaluation_recorded")?;
                println!(
                    "arm={arm} new_updates={n}/{max_updates} cumulative_step={} watch={count}/32 new_errors={new_errors} elapsed_s={:.3}",
                    state.step,
                    control.start.elapsed().as_secs_f64()
                );
                control.check("before_evaluation_checkpoint")?;
                save_arm(
                    &mut l,
                    &state,
                    &adam,
                    &output.join(format!("step-{n:03}")),
                    "RECOVERY_SCREENING",
                )?;
                control.check("evaluation_checkpoint_saved")?;
                final_evaluation_complete =
                    n == max_updates && last_evaluation["final_evaluation_complete"] == true;
                if bad_streak >= 2 {
                    control.observe(StopReason::QualityGuard);
                    break;
                }
            }
            let Some((indices, sampler)) = entry else {
                break;
            };
            control.check("before_training_batch")?;
            let b = batch(&s, indices, &Device::Cpu)?;
            let target_count: usize = indices
                .iter()
                .map(|&i| s[i].tokens.len() - s[i].response_start)
                .sum();
            if state.consumed_tokens - start_input + b.tokens as u64 > max_input_tokens
                || state.target_tokens - start_targets + target_count as u64 > max_target_tokens
            {
                control.observe(StopReason::TokenBudget);
                break;
            }
            let (ce, obj, targets) = response_loss(
                &l.model.forward(&b.input, Some(&b.valid))?,
                &b,
                c.first_target_weight,
            )?;
            control.check("training_forward_returned")?;
            let ce = ce.to_scalar::<f32>()?;
            let objective = obj.to_scalar::<f32>()?;
            if !ce.is_finite() || !objective.is_finite() {
                return Err(Error::Model("nonfinite recovery loss".into()));
            }
            let grads = obj.backward()?;
            control.check("training_backward_returned")?;
            // Preserve the production accumulation arithmetic even for accumulation=1.
            let gradients = l
                .model
                .vars
                .iter()
                .map(|(name, var)| -> Result<_> {
                    let g = grads
                        .get(var)
                        .ok_or_else(|| Error::Model(format!("missing {name}")))?;
                    Ok((
                        name.clone(),
                        ((g * targets as f64)?.detach() / targets as f64)?,
                    ))
                })
                .collect::<Result<BTreeMap<_, _>>>()?;
            let mut groups: BTreeMap<String, [f64; 3]> = BTreeMap::new();
            let inspect = [1, 5, 20].contains(&(n + 1));
            control.check("before_optimizer")?;
            let (norm, delta) = adam.step_observed(
                &l.model.vars,
                &gradients,
                &c,
                state.step + 1,
                |name, g, old, next| {
                    if inspect {
                        let group = if name == "embedding" {
                            "embedding_tied_output"
                        } else if name.ends_with("q_norm") || name.ends_with("k_norm") {
                            "qk_norm"
                        } else if name.ends_with("gate")
                            || name.ends_with("up")
                            || name.ends_with("down")
                        {
                            "ffn"
                        } else if name.ends_with("norm") {
                            "norm"
                        } else {
                            "attention"
                        };
                        let a = groups.entry(group.into()).or_default();
                        a[0] += g.sqr()?.sum_all()?.to_scalar::<f32>()? as f64;
                        a[1] += (next - old)?.sqr()?.sum_all()?.to_scalar::<f32>()? as f64;
                        a[2] += old.sqr()?.sum_all()?.to_scalar::<f32>()? as f64;
                    }
                    Ok(())
                },
            )?;
            state.step += 1;
            state.consumed_tokens += b.tokens as u64;
            state.target_tokens += targets as u64;
            state.sampler_state = *sampler;
            state.train_loss = Some(ce as f64);
            state.validation_loss = None;
            let _ = control.check("optimizer_committed"); // State/clock now describe the entire atomic step.
            for &i in indices {
                *exposure.entry(i).or_default() += 1;
            }
            let stats: BTreeMap<_,_>=groups.into_iter().map(|(k,v)|(k,json!({"gradient_norm":v[0].sqrt(),"update_norm":v[1].sqrt(),"weight_norm":v[2].sqrt(),"update_to_weight":v[1].sqrt()/v[2].sqrt().max(1e-30)}))).collect();
            let row = json!({"arm":arm,"new_update":n+1,"cumulative_model_step":state.step,"optimizer_step":state.step,"schedule_step":state.step-c.budget_start_step,"lr":c.learning_rate(state.step),"indices":indices,"ids":indices.iter().map(|&i|&episodes[i].id).collect::<Vec<_>>(),"sampler_state":sampler,"input_tokens":b.tokens,"target_tokens":targets,"ce":ce,"objective":objective,"first_target_weight":c.first_target_weight,"grad_norm":norm,"update_norm":delta,"parameter_groups":stats,"elapsed_seconds":control.start.elapsed().as_secs_f64(),"rss_kib":control.last_rss_kib});
            writeln!(log, "{row}")?;
            log.flush()?;
            control.stop_result()?;
            if n + 1 == 20 && arm == "C" {
                let hash = l.model.weight_hash()?;
                let expected = &f.registry["U2_AFTER_20"]["model_content_hash"];
                save(
                    &output.join("control-step20-parity.json"),
                    &json!({"actual":hash,"recorded_u2_after20":expected,"equal":hash==*expected}),
                )?;
                if hash != *expected {
                    return Err(Error::Model(
                        "control differs from historical 20-update replay".into(),
                    ));
                }
            }
        }
        Ok(())
    })();
    if let Err(error) = &outcome {
        control.classify_error(error);
    }
    let mut result = finish_arm(control, final_evaluation_complete, |reason| {
        save_arm(&mut l, &state, &adam, &output.join("final"), reason)
    });
    let info = json!({"arm":arm,"new_updates":state.step-start_step,"cumulative_model_step":state.step,"additional_input_tokens":state.consumed_tokens-start_input,"additional_target_tokens":state.target_tokens-start_targets,"model_content_hash":l.model.weight_hash()?,"elapsed_seconds":control.start.elapsed().as_secs_f64(),"error":outcome.as_ref().err().map(ToString::to_string),"goal1_ready":false,"last_evaluation":last_evaluation});
    result
        .as_object_mut()
        .unwrap()
        .extend(info.as_object().unwrap().clone());
    save(&output.join("result.json"), &result)?;
    log.sync_all()?;
    println!("{result}");
    outcome.and(control.stop_result())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn repair_control() -> RunControl {
        RunControl::new(
            Arc::new(AtomicBool::new(false)),
            Duration::from_secs(60),
            u64::MAX,
        )
        .unwrap()
    }
    fn repair_loaded() -> Loaded {
        let tok = ByteBpe::train(&[b"abc".to_vec()], &neural::hash(b"fixture"), 264).unwrap();
        let model =
            Transformer::init(neural::transformer::Config::tiny(264), 17, Device::Cpu).unwrap();
        let manifest = checkpoint::initialized(&model, &tok, 17, neural::hash(b"fixture")).unwrap();
        Loaded {
            model,
            tokenizer: tok,
            manifest,
            optimizer: BTreeMap::new(),
        }
    }
    fn repair_episode(id: &str) -> Episode {
        Episode {
            id: id.into(),
            category: 0,
            family: id.into(),
            binding: id.into(),
            sequence: id.into(),
            request: ModelRequest {
                request_id: id.into(),
                system: String::new(),
                input: "abc".into(),
                evidence: Default::default(),
                limits: replica_v3::event::GenerationLimits {
                    context_tokens: 64,
                    max_tokens: 1,
                    timeout_ms: 30000,
                },
            },
            answer: "b".into(),
        }
    }
    fn repair_corpus(root: &Path, validation: &[Episode]) -> data::CorpusManifest {
        std::fs::create_dir_all(root).unwrap();
        let write = |name: &str, episodes: &[Episode]| {
            let bytes = serde_json::to_vec(episodes).unwrap();
            std::fs::write(root.join(name), &bytes).unwrap();
            data::Split {
                file: name.into(),
                sha256: neural::hash(&bytes),
                bytes: bytes.len(),
                documents: episodes.len(),
                tokens: None,
            }
        };
        let m = data::CorpusManifest {
            version: 1,
            scope: "test".into(),
            permission: "synthetic".into(),
            generator: "independent".into(),
            seed: 0,
            split_rule: "distinct".into(),
            train: write("train.json", &[repair_episode("train/0")]),
            validation: write("validation.json", validation),
        };
        std::fs::write(root.join("manifest.json"), serde_json::to_vec(&m).unwrap()).unwrap();
        m
    }
    fn repair_frozen(root: &Path, validation: &[Episode], l: &Loaded) -> Frozen {
        let corpus = root.join("corpus");
        let m = repair_corpus(&corpus, validation);
        let start = root.join("model");
        checkpoint::save(
            &start,
            &l.model,
            &l.tokenizer,
            l.manifest.clone(),
            &BTreeMap::new(),
        )
        .unwrap();
        Frozen {
            version: 1,
            registry: json!({}),
            corpus: corpus.clone(),
            parent_corpus: corpus,
            start,
            train_hash: m.train.sha256.clone(),
            validation_hash: m.validation.sha256,
            parent_train_hash: m.train.sha256,
            tokenizer: l.tokenizer.semantic_id(),
            watch: (0..32)
                .map(|i| repair_episode(&format!("watch/{i}")))
                .collect(),
            failures: vec![repair_episode("failure/0")],
            previous_parent: vec![],
            previous_failed: vec![],
        }
    }
    #[test]
    fn repair_rf01_changed_validation_rejected_before_model_or_output() {
        let dir = tempfile::tempdir().unwrap();
        let l = repair_loaded();
        let mut episodes = vec![
            repair_episode("validation/0"),
            repair_episode("validation/1"),
        ];
        let f = repair_frozen(dir.path(), &episodes, &l);
        let fixture = dir.path().join("frozen.json");
        save(&fixture, &f).unwrap();
        episodes[0].request.input = "changed same ID".into();
        repair_corpus(&f.corpus, &episodes); // Updated manifest is valid for the changed bytes.
        assert!(data::load(&f.corpus).is_ok());
        let output = dir.path().join("new.jsonl");
        let mut control = repair_control();
        let result = replay(
            &fixture,
            &f.start,
            &output,
            "all",
            Some(&neural::hash(b"test source")),
            &mut control,
            true,
            None,
        );
        assert!(
            result.is_err(),
            "same IDs with changed content must reject frozen binding"
        );
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("validation binding")
        );
        assert!(!output.exists());
        assert_eq!(control.generation_calls, 0);
    }
    #[test]
    fn repair_rf01_owned_snapshot_panel_content_and_preload_binding() {
        let dir = tempfile::tempdir().unwrap();
        let l = repair_loaded();
        let original = vec![
            repair_episode("validation/0"),
            repair_episode("validation/1"),
        ];
        let f = repair_frozen(dir.path(), &original, &l);
        let (snapshot, ledger) = replay_cases(&f, "all").unwrap();
        assert_eq!(ledger["actual_split_hash"], f.validation_hash);
        assert_eq!(ledger["planned_case_count"], 2);
        assert_eq!(ledger["evaluated_cases_hash"], digest(&original).unwrap());
        for mutation in 0..4 {
            let mut changed = original.clone();
            match mutation {
                0 => changed[0].request.input.push('x'),
                1 => changed[0].answer.push('x'),
                2 => changed[0].request.evidence.items = independent_request().evidence.items,
                _ => {
                    changed.pop();
                }
            }
            repair_corpus(&f.corpus, &changed);
            assert!(data::load(&f.corpus).is_ok());
            if mutation < 3 {
                assert_eq!(
                    digest(&original.iter().map(|e| &e.id).collect::<Vec<_>>()).unwrap(),
                    digest(&changed.iter().map(|e| &e.id).collect::<Vec<_>>()).unwrap()
                );
            }
            assert_ne!(digest(&original).unwrap(), digest(&changed).unwrap());
            let mut model_call_count = 0;
            let guarded = replay_cases(&f, "all").map(|_| {
                model_call_count += 1;
            });
            assert!(guarded.is_err());
            assert_eq!(model_call_count, 0);
            let fixture = dir.path().join(format!("frozen-{mutation}.json"));
            save(&fixture, &f).unwrap();
            let existing = dir.path().join("existing.jsonl");
            std::fs::write(&existing, b"preserve output").unwrap();
            // Binding error, not a missing-model error, must take precedence even here.
            let error = replay(
                &fixture,
                &dir.path().join("missing-model"),
                &existing,
                "all",
                Some(&neural::hash(b"source")),
                &mut repair_control(),
                true,
                None,
            )
            .unwrap_err();
            assert!(error.to_string().contains("validation binding"));
            assert_eq!(std::fs::read(existing).unwrap(), b"preserve output");
        }
        assert_eq!(digest(&snapshot).unwrap(), digest(&original).unwrap());
        let mut manifest = repair_corpus(&f.corpus, &original);
        manifest.validation.sha256 = neural::hash(b"wrong manifest");
        std::fs::write(
            f.corpus.join("manifest.json"),
            serde_json::to_vec(&manifest).unwrap(),
        )
        .unwrap();
        assert!(replay_cases(&f, "all").is_err());
        for (panel, expected) in [("watch", &f.watch), ("failures", &f.failures)] {
            let (cases, ledger) = replay_cases(&f, panel).unwrap();
            assert_eq!(digest(&cases).unwrap(), digest(expected).unwrap());
            assert_eq!(ledger["source_validation_hash"], f.validation_hash);
            assert!(ledger["actual_split_hash"].is_null());
            assert_eq!(ledger["evaluated_cases_hash"], digest(expected).unwrap());
        }
        assert!(replay_cases(&f, "typo").is_err());
    }
    #[test]
    fn repair_rf01_normal_all_replay_binds_actual_content_before_generation() {
        let dir = tempfile::tempdir().unwrap();
        let l = repair_loaded();
        let cases = vec![
            repair_episode("validation/0"),
            repair_episode("validation/1"),
        ];
        let f = repair_frozen(dir.path(), &cases, &l);
        let fixture = dir.path().join("frozen.json");
        save(&fixture, &f).unwrap();
        let output = dir.path().join("all.jsonl");
        let mut control = repair_control();
        replay(
            &fixture,
            &f.start,
            &output,
            "all",
            Some(&neural::hash(b"fixture source")),
            &mut control,
            true,
            None,
        )
        .unwrap();
        let (header, rows) = rows(&output).unwrap();
        assert_eq!(header["actual_split_hash"], f.validation_hash);
        assert_eq!(header["frozen_expected_validation_hash"], f.validation_hash);
        assert_eq!(header["evaluated_cases_hash"], digest(&cases).unwrap());
        assert_eq!(rows.len(), 2);
        assert_eq!(control.generation_calls, 3);
        let matched = dir.path().join("matched.jsonl");
        let mut second = repair_control();
        replay(
            &fixture,
            &f.start,
            &matched,
            "all",
            Some(&neural::hash(b"fixture source")),
            &mut second,
            true,
            Some(&output),
        )
        .unwrap();
        assert_eq!(second.generation_calls, 3);
        let (_, matched_rows) = super::rows(&matched).unwrap();
        assert_eq!(matched_rows[0]["raw_tokens"], rows[0]["raw_tokens"]);
    }
    #[test]
    fn repair_rf02_no_evidence_causal_assertion_must_not_pass_scan() {
        let mut l = repair_loaded();
        l.model.config.profile = "NATIVE_TRPP_EXPERIMENTAL_V1".into();
        l.model.config.context = 512; // Tokenization-only fixture, no forward/update.
        let mut e = repair_episode("semantic/0");
        e.request.input = "센서31의 사고 원인은 무엇인가?".into();
        e.request.limits.context_tokens = 512;
        e.request.limits.max_tokens = 128;
        e.answer = "우회전이 원인이다.".into();
        let report = scan(&[e], &l, 1).unwrap();
        assert!(
            !report["data_ambiguities"].as_array().unwrap().is_empty(),
            "no citations must not authorize unsupported causal assertions"
        );
    }
    #[test]
    fn repair_rf03_pre_cancel_blocks_real_generation_and_teacher() {
        let l = repair_loaded();
        let e = repair_episode("cancel/0");
        let flag = Arc::new(AtomicBool::new(true));
        let mut control = RunControl::new(flag.clone(), Duration::from_secs(10), u64::MAX).unwrap();
        assert!(matches!(
            l.model
                .generate_observed(&[BOS], 1, 1000, &flag, "cancel", |_| panic!(
                    "already cancelled"
                )),
            Err(Error::Cancelled)
        ));
        let row = evaluate_one(&l, &e, &e.request, &mut control);
        assert!(
            row["raw_tokens"]
                .as_array()
                .is_none_or(|ids| ids.is_empty()),
            "command's true flag must reach actual evaluation"
        );
        assert!(row["teacher_forced_diagnostic_after_generation"].is_null());
        assert_eq!(control.generation_calls, 0);
        assert_eq!(control.teacher_calls, 0);
    }
    #[test]
    fn repair_rf03_deadline_precision_priority_and_latched_cleanup() {
        for (elapsed, expected) in [
            (Duration::from_nanos(999_999_999), None),
            (Duration::from_secs(1), Some(StopReason::TimeBudget)),
            (
                Duration::from_nanos(1_000_000_001),
                Some(StopReason::TimeBudget),
            ),
        ] {
            let mut c = RunControl::new(
                Arc::new(AtomicBool::new(false)),
                Duration::from_secs(1),
                100,
            )
            .unwrap();
            let _ = c.check_at(c.start + elapsed, Ok(10));
            assert_eq!(c.stop, expected);
        }
        let mut c = repair_control();
        c.elapsed_override = Some(Duration::from_millis(59_750));
        assert_eq!(c.effective_timeout(1000).unwrap(), 250);
        assert_eq!(c.effective_timeout(100).unwrap(), 100);
        c.elapsed_override = Some(Duration::from_nanos(59_999_500_000));
        assert!(c.effective_timeout(1000).is_err());
        assert_eq!(c.stop, Some(StopReason::TimeBudget));
        c.classify_error(&Error::Model("timeout".into()));
        assert_eq!(c.stop, Some(StopReason::TimeBudget));
        let mut c = repair_control();
        c.cancel.store(true, Ordering::Relaxed);
        let _ = c.check_at(c.deadline, Err(Error::Invalid("RSS unavailable".into())));
        assert_eq!(c.stop, Some(StopReason::Cancelled));
        assert_eq!(
            c.observed,
            vec![
                StopReason::Cancelled,
                StopReason::TimeBudget,
                StopReason::ResourceObservationFailed
            ]
        );
        let report = finish_arm(&mut c, false, |_| Err(Error::Invalid("save failed".into())));
        assert_eq!(report["reason"], "CANCELLED");
        assert_eq!(report["checkpoint_saved"], false);
        assert!(
            report["save_error"]
                .as_str()
                .unwrap()
                .contains("save failed")
        );
        let mut c = repair_control();
        let _ = c.check_at(c.start, Err(Error::Invalid("RSS unavailable".into())));
        assert_eq!(c.stop, Some(StopReason::ResourceObservationFailed));
        let mut c =
            RunControl::new(Arc::new(AtomicBool::new(false)), Duration::from_secs(1), 10).unwrap();
        let _ = c.check_at(c.start, Ok(11));
        assert_eq!(c.stop, Some(StopReason::ResourceLimit));
    }
    #[test]
    fn repair_rf03_actual_token_cancel_preserves_partial_and_skips_followup() {
        let l = repair_loaded();
        let cases = vec![repair_episode("cancel/0"), repair_episode("cancel/1")];
        let mut c = repair_control();
        c.hook = Some(Box::new(|boundary, flag| {
            if boundary == "token_generated" {
                flag.store(true, Ordering::Relaxed);
            }
        }));
        let rows = evaluate_panel(&l, &cases, &mut c);
        assert_eq!(rows.len(), 1);
        assert!(!rows[0]["raw_tokens"].as_array().unwrap().is_empty());
        assert_eq!(rows[0]["interruption"], "CANCELLED");
        assert_eq!(c.generation_calls, 1);
        assert_eq!(c.teacher_calls, 0);
        let mut report = summarize(&rows).unwrap();
        add_partial_counts(&mut report, &rows, 2, &c);
        assert_eq!(report["planned_case_count"], 2);
        assert_eq!(report["attempted_case_count"], 1);
        assert_eq!(report["completed_generation_count"], 0);
        assert_eq!(report["not_run_count"], 1);
        assert_eq!(report["interrupted_case_id"], "cancel/0");
        assert_eq!(report["comparison_eligible"], false);
        assert_eq!(report["candidate_eligible"], false);
        let mut updates = 0;
        if c.check("before_optimizer").is_ok() {
            updates += 1;
        }
        assert_eq!(updates, 0);
    }
    #[test]
    fn repair_rf03_deadline_finalization_and_native_save_failure_preserve_original() {
        let l = repair_loaded();
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("existing");
        checkpoint::save(
            &path,
            &l.model,
            &l.tokenizer,
            l.manifest.clone(),
            &BTreeMap::new(),
        )
        .unwrap();
        let before = std::fs::read(&path).unwrap();
        let mut c = repair_control();
        c.elapsed_override = Some(Duration::from_secs(60));
        let result = finish_arm(&mut c, true, |_| {
            checkpoint::save(
                &path,
                &l.model,
                &l.tokenizer,
                l.manifest.clone(),
                &BTreeMap::new(),
            )
            .map(|_| ())
        });
        assert_eq!(result["reason"], "TIME_BUDGET");
        assert_eq!(result["checkpoint_saved"], false);
        assert!(result["save_error"].is_string());
        assert_eq!(std::fs::read(path).unwrap(), before);
        assert_eq!(result["comparison_eligible"], false);
        assert!(result["cleanup_elapsed_seconds"].as_f64().unwrap() >= 0.);
    }
    #[test]
    fn repair_rf03_final_evaluation_and_terminal_stop_without_training() {
        let l = repair_loaded();
        let watch = vec![repair_episode("watch/0")];
        let train = vec![repair_episode("train/0")];
        for boundary in [
            "before_teacher",
            "teacher_returned",
            "panel_completed",
            "checkpoint_preserved",
            "terminal",
        ] {
            let mut c = repair_control();
            let mut panels = 0;
            c.hook = Some(Box::new(move |at, flag| {
                if at == "panel_completed" {
                    panels += 1;
                }
                if at == boundary && (boundary != "panel_completed" || panels == 2) {
                    flag.store(true, Ordering::Relaxed);
                }
            }));
            // Enter the same final-evaluation/finalization path used at n=50, no optimizer calls.
            let mut evaluation = arm_evaluation(&l, &watch, &train, &mut c).unwrap();
            evaluation["new_updates"] = json!(50);
            let complete = evaluation["final_evaluation_complete"] == true;
            let mut saves = 0;
            let report = finish_arm(&mut c, complete, |_| {
                saves += 1;
                Ok(())
            });
            assert_eq!(saves, 1);
            assert_eq!(report["reason"], "CANCELLED", "{boundary}");
            assert_eq!(report["comparison_eligible"], false);
            assert_eq!(report["candidate_eligible"], false);
            if boundary == "before_teacher" {
                assert_eq!(c.teacher_calls, 0);
                assert_eq!(c.generation_calls, 1);
            }
            if boundary == "teacher_returned" {
                assert_eq!(c.teacher_calls, 1);
                assert_eq!(c.generation_calls, 1);
            }
        }
        let mut c = repair_control();
        let evaluation = arm_evaluation(&l, &watch, &train, &mut c).unwrap();
        assert_eq!(evaluation["not_run_count"], 0);
        assert_eq!(evaluation["final_evaluation_complete"], true);
        let result = finish_arm(&mut c, true, |_| Ok(()));
        assert_eq!(result["reason"], "SCREENING_BUDGET_REACHED");
        assert_eq!(result["checkpoint_saved"], true);
        assert_eq!(c.receipt()["terminal_reason"], "COMPLETED");
        c.cancel.store(true, Ordering::Relaxed);
        assert!(c.check("after_terminal").is_ok());
        assert!(c.stop.is_none());
    }
    #[test]
    fn recovery_generation_never_uses_gold_or_gold_length() {
        let tok = ByteBpe::train(&[b"abc".to_vec()], &neural::hash(b"fixture"), 264).unwrap();
        let model =
            Transformer::init(neural::transformer::Config::tiny(264), 17, Device::Cpu).unwrap();
        let manifest = checkpoint::initialized(&model, &tok, 17, neural::hash(b"fixture")).unwrap();
        let loaded = Loaded {
            model,
            tokenizer: tok,
            manifest,
            optimizer: BTreeMap::new(),
        };
        let request = ModelRequest {
            request_id: "gold-independence".into(),
            system: String::new(),
            input: "abc".into(),
            evidence: Default::default(),
            limits: replica_v3::event::GenerationLimits {
                context_tokens: 64,
                max_tokens: 8,
                timeout_ms: 30000,
            },
        };
        let mut e = Episode {
            id: "one/0".into(),
            category: 0,
            family: "fixture".into(),
            binding: String::new(),
            sequence: String::new(),
            request,
            answer: "a".into(),
        };
        let a = evaluate_one(&loaded, &e, &e.request, &mut repair_control());
        e.answer = "b".repeat(512); // Deliberately cannot fit teacher forcing; generation is unaffected.
        let b = evaluate_one(&loaded, &e, &e.request, &mut repair_control());
        assert!(
            a["raw_tokens"]
                .as_array()
                .is_some_and(|ids| !ids.is_empty())
        );
        for key in [
            "request_digest",
            "prompt_digest",
            "raw_tokens",
            "finish_reason",
            "actual",
            "error",
        ] {
            assert_eq!(a[key], b[key], "{key}");
        }
        assert!(b["teacher_forced_diagnostic_after_generation"]["error"].is_string());
    }
    fn independent_request() -> ModelRequest {
        use replica_v3::retrieval::{Evidence, EvidenceBundle};
        let record = |id, entity: &str, context: &str, value: &str, status: &str| Evidence {
            event_id: id,
            original_excerpt: format!("{entity}의 {context} 이동 지시는 {value}이다."),
            version_status: status.into(),
            recorded_at: id,
            observed_at: None,
            source: "fixture".into(),
            retrieval_reason: "fixture".into(),
            relation_path: vec![],
            excerpt_truncated: false,
        };
        ModelRequest {
            request_id: "test".into(),
            system: String::new(),
            input: "센서31의 구역1 현재 방향은?".into(),
            limits: Default::default(),
            evidence: EvidenceBundle {
                items: vec![
                    record(19, "센서310", "구역1", "왼쪽", "current"),
                    record(7, "센서31", "구역1", "북쪽", "superseded"),
                    record(23, "센서31", "구역1", "동쪽", "current"),
                    record(5, "센서31", "구역10", "남쪽", "current"),
                ],
                ..Default::default()
            },
        }
    }
    #[test]
    fn recovery_support_uses_question_boundaries_status_and_not_record_position() {
        let mut r = independent_request();
        assert_eq!(support(&r).unwrap(), vec![23]);
        r.evidence.items.reverse();
        assert_eq!(support(&r).unwrap(), vec![23]);
        r.input = "센서31 구역1의 현재 말고 처음 기록한 방향은?".into();
        assert_eq!(support(&r).unwrap(), vec![7]);
        let mut tied = r.clone();
        tied.evidence
            .items
            .iter_mut()
            .find(|e| e.event_id == 23)
            .unwrap()
            .recorded_at = 7;
        assert!(support(&tied).is_err());
        r.input = "센서31의 구역10 말고 구역1 방향을 알려줘.".into();
        assert_eq!(support(&r).unwrap(), vec![23]);
        r.evidence.items.push(
            r.evidence
                .items
                .iter()
                .find(|e| e.event_id == 23)
                .unwrap()
                .clone(),
        );
        assert!(support(&r).is_err());
    }
    #[test]
    fn recovery_causal_richer_answer_is_supported_not_a_conflicting_fact() {
        let mut r = independent_request();
        r.input = "센서31의 시간 순서만으로 사고 원인을 알 수 있나?".into();
        r.evidence.items.truncate(3);
        r.evidence.items[0].original_excerpt = "센서31의 구역1 이동 지시는 동쪽이다.".into();
        for (i, e) in r.evidence.items.iter_mut().enumerate() {
            e.recorded_at = i as i64;
        }
        r.evidence.items[1].original_excerpt = "센서31는 동쪽 지시를 실행했다.".into();
        r.evidence.items[2].original_excerpt =
            "이후 센서31의 사고가 기록되었다. 원인은 확인되지 않았다.".into();
        assert_eq!(support(&r).unwrap(), vec![23]);
        assert_eq!(chronology_citations(&r), Some(vec![19, 7, 23]));
        let report = repair_semantic_scan(
            &r,
            "지시 [event:19] 뒤 실행 [event:7], 이후 사고 [event:23]가 기록되었습니다. 원인은 확정되지 않았습니다.",
        );
        assert_eq!(report["validated"], 1);
        assert_eq!(audit_status(&[&report], &[]), "CHECKED_BOUNDARIES_PASS");
        r.evidence.items.reverse();
        assert_eq!(chronology_citations(&r), Some(vec![19, 7, 23]));
        r.evidence.items[2].original_excerpt = "센서310의 구역1 이동 지시는 동쪽이다.".into();
        assert!(chronology_citations(&r).is_none());
    }
    fn repair_semantic_scan(request: &ModelRequest, answer: &str) -> Value {
        let mut e = repair_episode("semantic/0");
        e.request = request.clone();
        e.request.limits.context_tokens = 512;
        e.request.limits.max_tokens = 128;
        e.answer = answer.into();
        let bytes = serde_json::to_vec(&e).unwrap();
        let tok = ByteBpe::train(
            &[bytes.clone(), bytes],
            &neural::hash(b"independent grammar fixture"),
            512,
        )
        .unwrap();
        let mut config = neural::transformer::Config::tiny(tok.vocab_size());
        config.profile = "NATIVE_TRPP_EXPERIMENTAL_V1".into();
        config.context = 512;
        let model = Transformer::init(config, 17, Device::Cpu).unwrap();
        let manifest = checkpoint::initialized(&model, &tok, 17, neural::hash(b"fixture")).unwrap();
        let l = Loaded {
            model,
            tokenizer: tok,
            manifest,
            optimizer: BTreeMap::new(),
        };
        scan(&[e], &l, 1).unwrap()
    }
    #[test]
    fn repair_rf02_scan_audit_positive_negative_and_unknown_grammar() {
        let mut r = independent_request();
        let check = |r: &ModelRequest, a: &str, state: &str, status: &str| {
            let report = repair_semantic_scan(r, a);
            assert_eq!(report[state], 1, "{a}: {report}");
            assert_eq!(audit_status(&[&report], &[]), status, "{a}: {report}");
            assert_eq!(
                [
                    "validated",
                    "contradicted",
                    "unsupported",
                    "ambiguous",
                    "explicitly_out_of_scope"
                ]
                .iter()
                .map(|k| report[*k].as_u64().unwrap())
                .sum::<u64>(),
                report["scanned"].as_u64().unwrap()
            );
            let mut reversed = r.clone();
            reversed.evidence.items.reverse();
            assert_eq!(target_semantics(r, a), target_semantics(&reversed, a));
        };
        check(
            &r,
            "센서31의 구역1 이동 지시는 동쪽이다. [event:23]",
            "validated",
            "CHECKED_BOUNDARIES_PASS",
        );
        r.input = "다른 현장과 구분하여 센서31의 구역1 이동 값을 알려줘.".into();
        check(
            &r,
            "센서31의 구역1 이동 지시는 동쪽이다. [event:23]",
            "validated",
            "CHECKED_BOUNDARIES_PASS",
        );
        for a in [
            "센서310의 구역1 이동 지시는 동쪽이다. [event:23]",
            "센서31의 구역10 이동 지시는 동쪽이다. [event:23]",
            "센서31의 구역1 이동 지시는 남쪽이다. [event:23]",
        ] {
            check(&r, a, "contradicted", "INTEGRITY_FAIL");
        }
        check(
            &r,
            "센서31의 구역1 이동 지시는 동쪽이다. [event:23] 추가 지시도 확정됐다.",
            "unsupported",
            "AUDIT_INCOMPLETE",
        );
        r.input = "센서31의 사고 원인은 무엇인가?".into();
        r.evidence.items.clear();
        check(
            &r,
            "근거가 없어 알 수 없습니다.",
            "validated",
            "CHECKED_BOUNDARIES_PASS",
        );
        r.evidence.items = independent_request().evidence.items[..1].to_vec();
        r.evidence.items[0].original_excerpt = "센서310의 점검은 끝났지만 사고 자료는 없다.".into();
        check(
            &r,
            "근거가 없어 알 수 없습니다.",
            "validated",
            "CHECKED_BOUNDARIES_PASS",
        );
        for a in [
            "우회전이 원인이다.",
            "이 회전은 원인이 아니다.",
            "원인은 확인되지 않았다. 하지만 우회전 때문에 사고가 났다.",
        ] {
            check(&r, a, "contradicted", "INTEGRITY_FAIL");
        }
        check(
            &r,
            "원인이 확인되지 않은 것이 아니지 않다.",
            "unsupported",
            "AUDIT_INCOMPLETE",
        );
        r.evidence = independent_request().evidence;
        r.evidence.items.truncate(3);
        r.evidence.items[0].original_excerpt = "센서31의 구역1 이동 지시는 동쪽이다.".into();
        r.evidence.items[1].original_excerpt = "센서31는 동쪽 지시를 실행했다.".into();
        r.evidence.items[2].original_excerpt =
            "이후 센서31의 사고가 기록되었다. 원인은 확인되지 않았다.".into();
        for (i, e) in r.evidence.items.iter_mut().enumerate() {
            e.recorded_at = i as i64;
        }
        check(
            &r,
            "원인은 확정되지 않았습니다. [event:23]",
            "validated",
            "CHECKED_BOUNDARIES_PASS",
        );
        check(
            &r,
            "원인은 확정되었습니다. [event:23]",
            "contradicted",
            "INTEGRITY_FAIL",
        );
        check(
            &r,
            "원인은 확정되지 않았습니다. [event:7]",
            "contradicted",
            "INTEGRITY_FAIL",
        );
        let positive = "지시 [event:19] 뒤 실행 [event:7], 이후 사고 [event:23]가 기록되었습니다. 원인은 확정되지 않았습니다.";
        check(&r, positive, "validated", "CHECKED_BOUNDARIES_PASS");
        for a in [
            "지시 [event:19] 뒤 실행 [event:7], 이후 사고 [event:23]가 기록되었습니다.",
            "지시 [event:23] 뒤 실행 [event:7], 이후 사고 [event:19]가 기록되었습니다. 원인은 확정되지 않았습니다.",
            "센서310의 지시 [event:19] 뒤 실행 [event:7], 이후 사고 [event:23]가 기록되었습니다. 원인은 확정되지 않았습니다.",
        ] {
            check(&r, a, "contradicted", "INTEGRITY_FAIL");
        }
        r.input =
            "센서31의 지시부터 실행과 사고까지 확인된 일과 원인의 불확실성을 설명해줘.".into();
        check(&r, positive, "validated", "CHECKED_BOUNDARIES_PASS");
        check(
            &r,
            "원인은 확정되지 않았습니다. [event:23]",
            "contradicted",
            "INTEGRITY_FAIL",
        );
        r.evidence.items[2].recorded_at = 0;
        check(&r, positive, "ambiguous", "AUDIT_INCOMPLETE");
    }
    #[test]
    fn recovery_one_factor_preserves_tape_and_both_existing_clocks() {
        let base = TrainConfig {
            first_target_weight: 8.,
            microbatch: 8,
            sample_group_size: 8,
            accumulation: 1,
            budget_start_step: 19750,
            max_steps: 20750,
            ..Default::default()
        };
        let a = arm_config(&base, "C").unwrap();
        let b = arm_config(&base, "W").unwrap();
        let mut av = serde_json::to_value(&a).unwrap();
        let bv = serde_json::to_value(&b).unwrap();
        av["first_target_weight"] = json!(1.);
        assert_eq!(av, bv);
        let (mut r1, mut r2) = (Rng::new(73), Rng::new(73));
        let pool: Vec<_> = (0..64).collect();
        for step in 19751..=19800 {
            assert_eq!(
                draw_indices(&pool, &a, &mut r1).unwrap(),
                draw_indices(&pool, &b, &mut r2).unwrap()
            );
            assert_eq!(a.learning_rate(step), b.learning_rate(step));
        }
        assert_eq!(r1.state, r2.state);
    }
    #[test]
    fn recovery_parent_lr_policy_changes_only_schedule_and_preserves_250_draws() {
        let base = TrainConfig {
            lr: 0.0003,
            first_target_weight: 8.,
            microbatch: 8,
            sample_group_size: 8,
            accumulation: 1,
            budget_start_step: 19750,
            max_steps: 20750,
            ..Default::default()
        };
        let parent = TrainConfig {
            budget_start_step: 19371,
            max_steps: 19871,
            ..base.clone()
        };
        let l = schedule_config(&base, &parent, 19750).unwrap();
        let expected_endpoint =
            0.0003 * (0.1 + 0.45 * (1. + (std::f64::consts::PI * 279. / 400.).cos()));
        assert!((l.lr - expected_endpoint).abs() < 1e-15);
        assert_eq!(l.warmup, 0);
        let mut expected = base.clone();
        expected.lr = expected_endpoint;
        expected.warmup = 0;
        assert!((expected.lr - l.lr).abs() < 1e-15);
        expected.lr = l.lr;
        assert_eq!(l, expected);
        let (mut a, mut b) = (Rng::new(73), Rng::new(73));
        let pool: Vec<_> = (0..2048).collect();
        for n in 1..=250 {
            assert_eq!(
                draw_indices(&pool, &base, &mut a).unwrap(),
                draw_indices(&pool, &l, &mut b).unwrap()
            );
            let reference = expected_endpoint
                * (0.1 + 0.45 * (1. + (std::f64::consts::PI * n as f64 / 1000.).cos()));
            assert!((l.learning_rate(19750 + n) - reference).abs() < 1e-15);
        }
        assert_eq!(a.state, b.state);
        assert!(l.learning_rate(19850) < base.learning_rate(19850));
    }
    #[test]
    fn recovery_unequal_target_accumulation_matches_combined_batch() {
        let model =
            Transformer::init(neural::transformer::Config::tiny(264), 91, Device::Cpu).unwrap();
        let samples = vec![
            Sample {
                tokens: vec![BOS, 8, 9, 10, EOS],
                response_start: 3,
                curriculum: false,
            },
            Sample {
                tokens: vec![BOS, 11, 12, 13, 14, 15, EOS],
                response_start: 1,
                curriculum: false,
            },
        ];
        let combined = batch(&samples, &[0, 1], &Device::Cpu).unwrap();
        let (_, obj, n) = response_loss(
            &model
                .forward(&combined.input, Some(&combined.valid))
                .unwrap(),
            &combined,
            8.,
        )
        .unwrap();
        assert_eq!(n, 8);
        let reference = obj.backward().unwrap();
        let mut summed: BTreeMap<String, Tensor> = BTreeMap::new();
        for i in 0..2 {
            let b = batch(&samples, &[i], &Device::Cpu).unwrap();
            let (_, obj, n) =
                response_loss(&model.forward(&b.input, Some(&b.valid)).unwrap(), &b, 8.).unwrap();
            let g = obj.backward().unwrap();
            for (name, v) in &model.vars {
                let term = (g.get(v).unwrap() * n as f64).unwrap();
                let next = match summed.remove(name) {
                    Some(old) => (old + term).unwrap(),
                    None => term,
                };
                summed.insert(name.clone(), next);
            }
        }
        for (name, v) in &model.vars {
            compare(reference.get(v).unwrap(), &(&summed[name] / 8.).unwrap()).unwrap();
        }
    }
    #[test]
    fn recovery_ledger_counts_decode_errors_and_rejects_duplicates() {
        let good = json!({"id":"a","family":"qa/test","category":0,"actual":"ok","expected":"ok","error":null,"generation":{"finish":"stop","generated":2},"exact_match":true});
        let bad = json!({"id":"b","family":"copy/test","category":0,"actual":null,"expected":"x","error":"invalid UTF-8","generation":{"finish":"stop","generated":3},"exact_match":false});
        let s = summarize(&[good.clone(), bad]).unwrap();
        assert_eq!(s["denominator"], 2);
        assert_eq!(s["qa"], json!([1, 1]));
        assert_eq!(s["auxiliary"], json!([0, 1]));
        assert_eq!(s["invalid_utf8"], 1);
        assert!(summarize(&[good.clone(), good]).is_err());
    }
    #[test]
    fn recovery_raw_bytes_classify_utf8_without_lossy_repair() {
        let t = ByteBpe::train(&[b"abc".to_vec()], &neural::hash(b"fixture"), 264).unwrap();
        let incomplete = t.encode(&[0xea, 0xb0]).unwrap();
        let invalid = t.encode(&[0xea, 0x20]).unwrap();
        assert_eq!(
            bytes_receipt(&t, &incomplete)["utf8_error_class"],
            "incomplete_tail"
        );
        assert_eq!(bytes_receipt(&t, &incomplete)["valid_up_to"], 0);
        assert_eq!(
            bytes_receipt(&t, &invalid)["utf8_error_class"],
            "invalid_sequence"
        );
        assert_eq!(
            bytes_receipt(&t, &t.encode("가".as_bytes()).unwrap())["utf8_valid"],
            true
        );
        assert!(t.decode(&incomplete).is_err());
    }
    #[test]
    fn recovery_observer_sees_control_without_changing_product_error() {
        let model =
            Transformer::init(neural::transformer::Config::tiny(264), 17, Device::Cpu).unwrap();
        for var in model.vars.values() {
            var.set(&Tensor::zeros(var.dims(), DType::F32, &Device::Cpu).unwrap())
                .unwrap();
        }
        let mut raw = Vec::new();
        let actual =
            model.generate_observed(&[BOS, 8], 4, 30000, &AtomicBool::new(false), "x", |id| {
                raw.push(id)
            });
        let product = model.generate(&[BOS, 8], 4, 30000, &AtomicBool::new(false), "x");
        assert_eq!(raw, vec![PAD]);
        assert_eq!(
            actual.unwrap_err().to_string(),
            product.unwrap_err().to_string()
        );
    }
    #[test]
    fn recovery_adam_three_steps_match_f64_clip_moments_and_clocks() {
        let vars = BTreeMap::from([("x".into(), Var::new(&[1f32, -2.], &Device::Cpu).unwrap())]);
        let mut adam = Adam::new(&vars).unwrap();
        let c = TrainConfig {
            warmup: 2,
            max_steps: 10,
            clip: 1.,
            ..Default::default()
        };
        let (mut weights, mut m, mut v) = ([1f64, -2.], [0f64; 2], [0f64; 2]);
        for (i, g) in [[3f32, 4.], [-2., 1.], [0.5, -0.25]].iter().enumerate() {
            let step = i + 1;
            let norm = (g.iter().map(|v| (*v as f64).powi(2)).sum::<f64>()).sqrt();
            let clip = (1. / (norm + 1e-12)).min(1.);
            for j in 0..2 {
                let g = g[j] as f64 * clip;
                m[j] = 0.9 * m[j] + 0.1 * g;
                v[j] = 0.999 * v[j] + 0.001 * g * g;
                let rate = if step <= 2 {
                    0.001 * step as f64 / 2.
                } else {
                    0.001
                        * (0.1
                            + 0.45 * (1. + (std::f64::consts::PI * (step - 2) as f64 / 8.).cos()))
                };
                weights[j] = weights[j] * (1. - rate * 0.01)
                    - rate * (m[j] / (1. - 0.9f64.powi(step as i32)))
                        / ((v[j] / (1. - 0.999f64.powi(step as i32))).sqrt() + 1e-8);
            }
            adam.step(
                &vars,
                &BTreeMap::from([("x".into(), Tensor::new(g, &Device::Cpu).unwrap())]),
                &c,
                step,
            )
            .unwrap();
            for (actual, expected) in vars["x"].to_vec1::<f32>().unwrap().iter().zip(weights) {
                assert!((*actual as f64 - expected).abs() < 1e-6);
            }
            for (name, expected) in [("adam.m.x", m), ("adam.v.x", v)] {
                for (a, b) in adam.moments[name]
                    .to_vec1::<f32>()
                    .unwrap()
                    .iter()
                    .zip(expected)
                {
                    assert!((*a as f64 - b).abs() < 1e-6);
                }
            }
        }
        let continued = TrainConfig {
            budget_start_step: 100,
            max_steps: 110,
            ..c
        };
        assert_eq!(continued.learning_rate(101), 0.0005);
        assert_eq!(continued.learning_rate(102), 0.001);
    }
}
