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
    pub(super) last_rss_kib: Option<u64>,
    pub(super) measure_rss: bool,
    stop: Option<StopReason>,
    observed: Vec<StopReason>,
    terminal: bool,
    generation_calls: usize,
    completed_generation_count: usize,
    attempted_case_count: usize,
    interrupted_case_id: Option<String>,
    pub(super) teacher_calls: usize,
    #[cfg(test)]
    elapsed_override: Option<Duration>,
    #[cfg(test)]
    #[allow(clippy::type_complexity)]
    hook: Option<Box<dyn FnMut(&str, &Arc<AtomicBool>)>>,
}
impl RunControl {
    pub(super) fn new(
        cancel: Arc<AtomicBool>,
        duration: Duration,
        max_rss_kib: u64,
    ) -> Result<Self> {
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
            measure_rss: true,
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
    fn check_time(&mut self, now: Instant) {
        // Simultaneous observation priority: user cancel, deadline, RSS failure, RSS limit.
        if self.cancel.load(Ordering::Relaxed) {
            self.observe(StopReason::Cancelled);
        }
        if now >= self.deadline {
            self.observe(StopReason::TimeBudget);
        }
    }
    fn check_at(&mut self, now: Instant, rss: Result<u64>) -> Result<()> {
        if self.terminal {
            return self.stop_result();
        }
        self.check_time(now);
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
        if self.measure_rss {
            let rss = rss_kib();
            self.check_at(self.now(), rss)
        } else {
            if !self.terminal {
                self.check_time(self.now());
            }
            self.stop_result()
        }
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
    pub(super) fn classify_error(&mut self, e: &Error) {
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
    pub(super) fn reason(&self) -> Option<&'static str> {
        self.stop.map(StopReason::name)
    }
    pub(super) fn receipt(&self) -> Value {
        json!({"terminal_reason":self.stop.map(StopReason::name).or_else(||self.terminal.then_some("COMPLETED")),"observed_conditions":self.observed,"generation_calls":self.generation_calls,"teacher_calls":self.teacher_calls,"completed_generation_count":self.completed_generation_count,"attempted_case_count":self.attempted_case_count,"interrupted_case_id":self.interrupted_case_id,
            "elapsed_seconds":self.now().duration_since(self.start).as_secs_f64(),"work_budget_seconds":self.deadline.duration_since(self.start).as_secs_f64(),
            "work_deadline_overrun_seconds":self.now().saturating_duration_since(self.deadline).as_secs_f64(),"rss_observation_enabled":self.measure_rss,"cooperative_only":true})
    }
}

#[derive(Subcommand)]
pub enum Command {
    /// Recount preserved skill rows without loading or calling a model; never promotes a candidate.
    SkillRecount {
        #[arg(long)]
        evaluation: PathBuf,
        #[arg(long)]
        output: PathBuf,
    },
    /// Inspect a stopped H3 run: observed/unobserved train views and actual failing cache path.
    SkillDiagnose {
        #[arg(long)]
        run: PathBuf,
        #[arg(long)]
        corpus: PathBuf,
        #[arg(long)]
        output: PathBuf,
    },
    /// Bounded H3 only; constant LR, inherited Adam/clock and frozen without-replacement tape.
    SkillRun {
        #[arg(long)]
        baseline: PathBuf,
        #[arg(long)]
        corpus: PathBuf,
        #[arg(long)]
        output: PathBuf,
        #[arg(long)]
        harness: PathBuf,
        /// Explicit plain resume of a clean time-limited segment; never resumes cancellation.
        #[arg(long, conflicts_with = "renew_from_quality_stop")]
        resume: Option<PathBuf>,
        /// Explicit renewed attempt after a reported quality stop; retains the original total budget.
        #[arg(long, conflicts_with = "resume")]
        renew_from_quality_stop: Option<PathBuf>,
        /// Explicitly authorized final512 updates; intermediate UTF-8 count uses two consecutive increases.
        #[arg(long, requires = "renew_from_quality_stop", conflicts_with = "resume")]
        finish_copy_budget: bool,
    },
    /// Freeze H3 train/dev/seal only after a verified parent/data baseline; no learning.
    SkillPrepare {
        #[arg(long)]
        baseline: PathBuf,
        #[arg(long)]
        output: PathBuf,
        #[arg(long)]
        seed: u64,
    },
    /// Freeze actual parent/data identities, audit all ordinary data and replay a fixed watch32.
    Baseline {
        #[arg(long)]
        checkpoint: PathBuf,
        #[arg(long)]
        inference: PathBuf,
        #[arg(long)]
        corpus: PathBuf,
        #[arg(long)]
        original_corpus: PathBuf,
        #[arg(long)]
        previous_log: PathBuf,
        #[arg(long)]
        source_id: String,
        #[arg(long)]
        output: PathBuf,
    },
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
pub(super) fn summarize(rows: &[Value]) -> Result<Value> {
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
    let mut budget = RunControl::command(matches!(
        &command,
        Command::Arm { .. } | Command::SkillRun { .. }
    ))?;
    budget.check("command_started")?;
    let outcome = match command {
        Command::SkillRecount { evaluation, output } => {
            let recorded = read_json(&evaluation)?;
            let dev = recorded["dev_rows"]
                .as_array()
                .ok_or_else(|| Error::Corrupt("skill dev rows".into()))?;
            let watch = recorded["watch_rows"]
                .as_array()
                .ok_or_else(|| Error::Corrupt("skill watch rows".into()))?;
            budget.check("skill_recount")?;
            save(
                &output,
                &json!({"evidence_level":"DERIVED_FROM_EXISTING_LOGS","evaluation_sha256":file_hash(&evaluation)?,
                "recorded_model_content_hash":recorded["model_content_hash"],"new_updates":recorded["new_updates"],
                "dev":skill_score(dev)?,"watch":summarize(watch)?,"actual_optimizer_updates":0,"model_calls":0,"candidate_eligible":false}),
            )
        }
        Command::SkillDiagnose {
            run,
            corpus,
            output,
        } => skill_diagnose(&run, &corpus, &output, &mut budget),
        Command::SkillRun {
            baseline,
            corpus,
            output,
            harness,
            resume,
            renew_from_quality_stop,
            finish_copy_budget,
        } => skill_run(
            &baseline,
            &corpus,
            &output,
            &harness,
            resume.as_deref(),
            renew_from_quality_stop
                .as_deref()
                .map(|path| (path, finish_copy_budget)),
            &mut budget,
        ),
        Command::SkillPrepare {
            baseline,
            output,
            seed,
        } => {
            let report = read_json(&baseline.join("summary.json"))?;
            let f = load(&baseline.join("frozen.json"))?;
            if report["baseline_verified"] != true
                || report["data_audit_status"] != "CHECKED_BOUNDARIES_PASS"
                || report["parent"]["physical_hash"] != file_hash(&f.start)?
                || report["original_manifest"]["train"]["sha256"] != f.parent_train_hash
            {
                return Err(Error::Invalid("H2 verified baseline required".into()));
            }
            data::copy_curriculum(&f.parent_corpus, &output, seed)?;
            let (manifest, train, dev) = data::load(&output)?;
            let seal_descriptor: data::Split =
                serde_json::from_value(read_json(&output.join("seal-manifest.json"))?)?;
            let seal = data::load_split(&output, &seal_descriptor)?;
            let l = checkpoint::load(&f.start, Device::Cpu, false)?;
            let checked = verify_copy_curriculum(&train, &dev, &seal, &l)?;
            let anchors = scan_controlled(&train[..2048], &l, 2048, Some(&mut budget))?;
            if anchors["status"] != "CHECKED_BOUNDARIES_PASS" {
                return Err(Error::Invalid("unverified QA anchors".into()));
            }
            save(
                &output.join("prepared.json"),
                &json!({"stage":"H3","status":"PRETRAIN_STRUCTURE_VERIFIED","baseline_summary_hash":file_hash(&baseline.join("summary.json"))?,
                "manifest_hash":file_hash(&output.join("manifest.json"))?,"train_hash":manifest.train.sha256,"dev_hash":manifest.validation.sha256,"seal":seal_descriptor,
                "checks":checked,"anchors":anchors,"seed":seed,"seal_exposure":"structural validation only; no model evaluation or selection","optimizer_updates":0}),
            )?;
            Ok(())
        }
        Command::Baseline {
            checkpoint,
            inference,
            corpus,
            original_corpus,
            previous_log,
            source_id,
            output,
        } => baseline(
            [
                &checkpoint,
                &inference,
                &corpus,
                &original_corpus,
                &previous_log,
            ],
            &output,
            &source_id,
            &mut budget,
        ),
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
                    let planned = load(&fixture).ok().map(|f| f.watch.len() * 2 + 6);
                    let mut partial = budget.receipt();
                    partial["planned_case_count"] = json!(planned);
                    partial["not_run_count"] =
                        json!(planned.map(|n| n.saturating_sub(budget.attempted_case_count)));
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
// Restricted corpus grammar only. An unparsed record is never assumed irrelevant.
fn causal_record_state<'a>(
    q: &str,
    record: &'a replica_v3::retrieval::Evidence,
) -> (&'static str, Option<&'a str>) {
    let text = record.original_excerpt.as_str();
    let atom = |s: &str| !s.is_empty() && s.chars().all(char::is_alphanumeric);
    let (state, entity) = if let Some(entity) = accident_entity(text).filter(|s| atom(s)) {
        ("SUPPORTED", entity)
    } else if let Some(entity) = text
        .strip_suffix("의 점검은 끝났지만 사고 자료는 없다.")
        .filter(|s| atom(s))
    {
        ("EXPLICIT_NO_EVIDENCE", entity)
    } else if let Some((entity, _, _)) = exact_fact(text) {
        ("CONTEXT", entity)
    } else if let Some((entity, value)) = text
        .strip_suffix(" 지시를 실행했다.")
        .and_then(|s| s.split_once("는 "))
        && atom(entity)
        && atom(value)
    {
        ("CONTEXT", entity)
    } else if let Some((entity, rest)) = text.split_once("의 ")
        && atom(entity)
        && (rest.starts_with("사고") || rest.starts_with("원인"))
    {
        ("UNSUPPORTED", entity)
    } else {
        return ("AMBIGUOUS_RELEVANCE", None);
    };
    if record.excerpt_truncated {
        return ("AMBIGUOUS_TRUNCATED", Some(entity));
    }
    if mentions_target(q, entity) {
        (state, Some(entity))
    } else {
        ("IRRELEVANT_EXPLICIT_OTHER_ENTITY", Some(entity))
    }
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
        let classified: Vec<_> = request
            .evidence
            .items
            .iter()
            .map(|r| (r, causal_record_state(q, r)))
            .collect();
        if classified
            .iter()
            .any(|(_, (state, _))| *state == "UNSUPPORTED")
        {
            return Err((
                SemanticState::UnsupportedForm,
                "additional related causal record outside supported grammar",
            ));
        }
        if classified
            .iter()
            .any(|(_, (state, _))| state.starts_with("AMBIGUOUS"))
        {
            return Err((
                SemanticState::AmbiguousEvidence,
                "causal record relevance or truncation unresolved",
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
        if records.len() > 1
            || records.iter().any(|(r, _)| r.excerpt_truncated)
            || (!records.is_empty()
                && classified
                    .iter()
                    .any(|(_, (s, _))| *s == "EXPLICIT_NO_EVIDENCE"))
        {
            return Err((
                SemanticState::AmbiguousEvidence,
                "multiple/truncated accident records",
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
    scan_controlled(episodes, l, limit, None)
}
fn scan_controlled(
    episodes: &[Episode],
    l: &Loaded,
    limit: usize,
    mut control: Option<&mut RunControl>,
) -> Result<Value> {
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
    let mut causal_records = Vec::new();
    let (
        mut checked,
        mut auxiliary,
        mut no_evidence,
        mut max_prompt,
        mut over_window,
        mut target_count,
    ) = (0, 0, 0, 0, 0, 0);
    for (index, e) in episodes.iter().take(limit).enumerate() {
        if index.is_multiple_of(128)
            && let Some(control) = control.as_deref_mut()
        {
            control.check("semantic_scan_batch")?;
        }
        *categories.entry(e.category).or_default() += 1;
        if e.request.input.contains("원인") || e.request.input.contains("인과관계") {
            for r in &e.request.evidence.items {
                let (state, entity) = causal_record_state(&e.request.input, r);
                causal_records.push(json!({"id":e.id,"event_id":r.event_id,"classification":state,"parsed_entity":entity}));
            }
        }
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
    report["causal_record_classifications"] = json!(causal_records);
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
fn baseline(
    paths: [&Path; 5],
    output: &Path,
    source_id: &str,
    control: &mut RunControl,
) -> Result<()> {
    let [resume, inference, corpus, original, previous_log] = paths;
    if source_id.len() != 64 || !source_id.bytes().all(|b| b.is_ascii_hexdigit()) {
        return Err(Error::Invalid("source digest required".into()));
    }
    control.check("baseline_start")?;
    let parent = registry(resume)?;
    let loaded = checkpoint::load(resume, Device::Cpu, false)?;
    let exported = checkpoint::load(inference, Device::Cpu, false)?;
    let inference_hash = file_hash(inference)?;
    let weight_hash = loaded.model.weight_hash()?;
    let state = loaded
        .manifest
        .training
        .as_ref()
        .ok_or_else(|| Error::Invalid("resume state required".into()))?;
    if exported.manifest.training.is_some()
        || exported.model.weight_hash()? != weight_hash
        || exported.tokenizer.semantic_id() != loaded.tokenizer.semantic_id()
        || exported.model.config != loaded.model.config
    {
        return Err(Error::Corrupt("resume/inference content mismatch".into()));
    }
    let next_lr = state.config.learning_rate(
        state
            .step
            .checked_add(1)
            .ok_or_else(|| Error::Corrupt("optimizer clock overflow".into()))?,
    );
    if !next_lr.is_finite() || next_lr <= 0. {
        return Err(Error::Corrupt("parent next LR".into()));
    }
    let (manifest, train, validation) = data::load(corpus)?;
    let (original_manifest, original_train, original_validation) = data::load(original)?;
    if state.corpus_hash != manifest.train.sha256
        || state.validation_hash != manifest.validation.sha256
        || manifest.validation.sha256 != original_manifest.validation.sha256
        || digest(&validation)? != digest(&original_validation)?
    {
        return Err(Error::Corrupt(
            "baseline corpus lineage/validation mismatch".into(),
        ));
    }
    let (header, historical) = rows(previous_log)?;
    verify_historical_rows(&validation, &historical)?;
    if header["checkpoint_sha256"] != loaded.manifest.weights_sha256
        || header["split_sha256"] != manifest.validation.sha256
        || [
            "oracle_question_ablation",
            "oracle_field_task_label",
            "oracle_record_selection",
        ]
        .iter()
        .any(|k| header[*k] == true)
    {
        return Err(Error::Corrupt(
            "baseline log/artifact/normal decoding mismatch".into(),
        ));
    }
    std::fs::create_dir(output)?;
    let ordinary = |cases: &[Episode]| {
        cases
            .iter()
            .filter(|e| !e.family.starts_with("copy/"))
            .count()
    };
    let mut report = json!({"stage":"H2","source_commit":source_commit()?,"source_digest":source_id,"binary_hash":file_hash(&std::env::current_exe()?)?,
        "parent":parent,"inference":{"path":inference,"physical_hash":inference_hash,"model_content_hash":weight_hash,"same_content":true},
        "train_manifest":manifest,"original_manifest":original_manifest,"train_ordinary":ordinary(&train),"original_train_ordinary":ordinary(&original_train),
        "validation_ordinary":ordinary(&validation),"validation_auxiliary":validation.len()-ordinary(&validation),"previous_log_sha256":file_hash(previous_log)?,
        "recount":summarize(&historical)?,"next_parent_lr":next_lr,"proposed_constant_lr":next_lr.min(3e-5),"new_small_updates":0,"new_input_tokens":0,"new_target_tokens":0,"goal1_ready":false});
    save(&output.join("identity-recount.json"), &report)?;
    let training_audit = scan_controlled(&train, &loaded, train.len(), Some(control))?;
    save(&output.join("binding-train-audit.json"), &training_audit)?;
    let original_audit = scan_controlled(
        &original_train,
        &loaded,
        original_train.len(),
        Some(control),
    )?;
    save(&output.join("original-train-audit.json"), &original_audit)?;
    let validation_audit = scan_controlled(&validation, &loaded, validation.len(), Some(control))?;
    save(&output.join("validation-audit.json"), &validation_audit)?;
    let status = audit_status(&[&training_audit, &original_audit, &validation_audit], &[]);
    report["data_audit_status"] = json!(status);
    let mut watch = Vec::new();
    let mut used = BTreeSet::new();
    for (category, count) in [7, 7, 6, 6, 6].into_iter().enumerate() {
        let selected: Vec<_> = validation
            .iter()
            .filter(|e| {
                e.category == category
                    && !e.family.starts_with("copy/")
                    && used.insert(scene(e).to_owned())
            })
            .take(count)
            .cloned()
            .collect();
        if selected.len() != count {
            return Err(Error::Corrupt("watch coverage".into()));
        }
        watch.extend(selected);
    }
    let frozen = Frozen {
        version: 1,
        registry: json!({"V1000":parent,"source_id":source_id,"binary_hash":file_hash(&std::env::current_exe()?)?}),
        corpus: corpus.into(),
        parent_corpus: original.into(),
        start: resume.into(),
        train_hash: manifest.train.sha256,
        validation_hash: manifest.validation.sha256,
        parent_train_hash: original_manifest.train.sha256,
        tokenizer: loaded.tokenizer.semantic_id(),
        watch,
        failures: vec![],
        previous_parent: historical.clone(),
        previous_failed: vec![],
    };
    let fixture = output.join("frozen.json");
    save(&fixture, &frozen)?;
    // Only the frozen ordinary32 are regenerated; the known400 denominator is re-counted above.
    replay(
        &fixture,
        inference,
        &output.join("watch32.jsonl"),
        "watch",
        Some(source_id),
        control,
        false,
        None,
    )?;
    let (_, actual) = rows(&output.join("watch32.jsonl"))?;
    let mut differences = Vec::new();
    for row in &actual {
        let prior = historical
            .iter()
            .find(|v| v["id"] == row["id"])
            .ok_or_else(|| Error::Corrupt("watch baseline ID".into()))?;
        for key in [
            "raw_tokens",
            "actual",
            "error",
            "exact_match",
            "prompt_digest",
            "provided",
            "excluded",
        ] {
            if row[key] != prior[key] {
                differences.push(json!({"id":row["id"],"field":key}));
            }
        }
    }
    report["watch"] = summarize(&actual)?;
    report["watch_differences"] = json!(differences);
    report["generation_calls"] = json!(control.generation_calls);
    report["artifacts_unchanged"] = json!(
        file_hash(resume)? == parent["physical_hash"] && file_hash(inference)? == inference_hash
    );
    if status != "CHECKED_BOUNDARIES_PASS" {
        control.observe(if status == "AUDIT_INCOMPLETE" {
            StopReason::AuditIncomplete
        } else {
            StopReason::IntegrityFail
        });
    }
    if !differences.is_empty() || report["artifacts_unchanged"] != true {
        control.observe(StopReason::IntegrityFail);
    }
    let sealed = control.seal_terminal();
    report["control"] = control.receipt();
    report["baseline_verified"] = json!(sealed.is_ok());
    report["h2_split_materialization"] = json!("NEXT_IF_BASELINE_VERIFIED");
    save(&output.join("summary.json"), &report)?;
    println!(
        "H2 audit={status} watch={}/32 new_updates=0 generation_calls={}",
        report["watch"]["exact_matches"], control.generation_calls
    );
    sealed
}
fn verify_copy_curriculum(
    train: &[Episode],
    dev: &[Episode],
    seal: &[Episode],
    l: &Loaded,
) -> Result<Value> {
    let invalid = || Error::Invalid("INVALID_SKILL_DATA: copy grammar/contrast/split".into());
    if train.len() != 4096 || dev.len() != 256 || seal.len() != 256 {
        return Err(invalid());
    }
    let mut identifiers = Vec::new();
    let mut scene_sets = Vec::new();
    let mut binding_sets = Vec::new();
    let mut strata = BTreeMap::<String, usize>::new();
    for (split, all) in [train, dev, seal].into_iter().enumerate() {
        let mut ids = BTreeSet::new();
        let mut scenes = BTreeSet::new();
        let mut bindings = BTreeSet::new();
        for e in all {
            for record in &e.request.evidence.items {
                if let Some((entity, _, _)) = exact_fact(&record.original_excerpt) {
                    ids.insert(entity.to_string());
                }
            }
            scenes.insert(scene(e).to_string());
            bindings.insert(e.binding.clone());
        }
        let focus = if split == 0 { &all[2048..] } else { all };
        for group in focus.as_chunks::<4>().0 {
            if group.iter().any(|e| scene(e) != scene(&group[0])) {
                return Err(invalid());
            }
            for (view, e) in group.iter().enumerate() {
                if e.request.evidence.items.len() != 1
                    || e.category != 0
                    || e.request.request_id != e.id
                {
                    return Err(invalid());
                }
                let record = &e.request.evidence.items[0];
                let (entity, context, value) =
                    exact_fact(&record.original_excerpt).ok_or_else(invalid)?;
                let digits = entity.trim_start_matches(|c: char| !c.is_ascii_digit());
                let prefix = &entity[..entity.len() - digits.len()];
                if !["장치", "설비", "센서", "장비"].contains(&prefix)
                    || !(1..=8).contains(&digits.len())
                    || !digits.bytes().all(|b| b.is_ascii_digit())
                    || record.version_status != "current"
                    || record.excerpt_truncated
                    || record.event_id <= 0
                    || e.binding != format!("{entity}/{context}/{value}")
                {
                    return Err(invalid());
                }
                if view == 3 {
                    if e.request.input
                        != "제공된 유일한 기록의 원문을 빠짐없이 쓰고 그 사건을 인용해줘."
                    {
                        return Err(invalid());
                    }
                } else if !mentions_target(&e.request.input, entity)
                    || !mentions_target(&e.request.input, context)
                {
                    return Err(invalid());
                }
                let (prose, citation) = e.answer.rsplit_once(' ').ok_or_else(invalid)?;
                if prose != record.original_excerpt
                    || citation_literal(citation) != Some(record.event_id)
                {
                    return Err(invalid());
                }
                *strata
                    .entry(format!("split-{split}/digits-{}", digits.len()))
                    .or_default() += 1;
                *strata
                    .entry(format!(
                        "split-{split}/{}",
                        if value.starts_with("경로") {
                            "short-value"
                        } else {
                            "direction"
                        }
                    ))
                    .or_default() += 1;
            }
            let records: Vec<_> = group.iter().map(|e| &e.request.evidence.items[0]).collect();
            let facts: Vec<_> = records
                .iter()
                .map(|r| exact_fact(&r.original_excerpt).unwrap())
                .collect();
            if facts[0].0 == facts[1].0
                || facts[0].1 != facts[1].1
                || facts[0].2 != facts[1].2
                || facts[0].0 != facts[2].0
                || facts[0].1 != facts[2].1
                || facts[0].2 == facts[2].2
                || serde_json::to_value(records[0])? != serde_json::to_value(records[3])?
                || records.iter().any(|r| {
                    r.event_id != records[0].event_id || r.recorded_at != records[0].recorded_at
                })
                || group[0].answer != group[3].answer
            {
                return Err(invalid());
            }
        }
        identifiers.push(ids);
        scene_sets.push(scenes);
        binding_sets.push(bindings);
    }
    for a in 0..3 {
        for b in a + 1..3 {
            if !identifiers[a].is_disjoint(&identifiers[b])
                || !scene_sets[a].is_disjoint(&scene_sets[b])
                || !binding_sets[a].is_disjoint(&binding_sets[b])
            {
                return Err(invalid());
            }
        }
    }
    let framed = samples(train, &l.tokenizer, 512)?;
    if framed.len() != train.len() || framed.iter().any(|s| s.tokens.len() > 513) {
        return Err(Error::Invalid(
            "skill training length; no truncation".into(),
        ));
    }
    Ok(
        json!({"train":4096,"anchors":2048,"focus":2048,"dev":256,"seal":256,"strata":strata,"identifier_sets":identifiers.iter().map(BTreeSet::len).collect::<Vec<_>>(),
        "base_scene_counts":scene_sets.iter().map(BTreeSet::len).collect::<Vec<_>>(),"max_train_tokens_including_eos":framed.iter().map(|s|s.tokens.len()).max(),
        "input_target_training_tokens_planned_one_epoch":[framed.iter().map(|s|s.tokens.len()-1).sum::<usize>(),framed.iter().map(|s|s.tokens.len()-s.response_start).sum::<usize>()],"model_calls":0,"eos_is_supervised":true}),
    )
}
fn skill_tape(episodes: &[Episode], sampler: u64) -> Result<Vec<(Vec<usize>, u64)>> {
    if episodes.len() != 4096 {
        return Err(Error::Invalid("skill pool size".into()));
    }
    if episodes[..2048]
        .iter()
        .map(scene)
        .collect::<BTreeSet<_>>()
        .len()
        != 2048
        || episodes[2048..]
            .as_chunks::<4>()
            .0
            .iter()
            .any(|g| g.iter().any(|e| scene(e) != scene(&g[0])))
        || episodes[2048..]
            .as_chunks::<4>()
            .0
            .iter()
            .map(|g| scene(&g[0]))
            .collect::<BTreeSet<_>>()
            .len()
            != 512
    {
        return Err(Error::Invalid("skill pool base lineage".into()));
    }
    let mut rng = Rng { state: sampler };
    let shuffle = |v: &mut [usize], rng: &mut Rng| {
        for i in (1..v.len()).rev() {
            v.swap(i, (rng.next_u64() % (i + 1) as u64) as usize);
        }
    };
    let mut tape = Vec::new();
    for _epoch in 0..2 {
        let mut anchors: Vec<_> = (0..2048).collect();
        shuffle(&mut anchors, &mut rng);
        let mut variants = vec![[0, 1, 2, 3]; 512];
        for v in &mut variants {
            shuffle(v, &mut rng);
        }
        for view in [0, 1, 2, 3] {
            let mut bases: Vec<_> = (0..512).collect();
            shuffle(&mut bases, &mut rng);
            for (batch, group) in bases.as_chunks::<4>().0.iter().enumerate() {
                let at = view * 512 + batch * 4;
                let mut indices = anchors[at..at + 4].to_vec();
                indices.extend(
                    group
                        .iter()
                        .map(|base| 2048 + base * 4 + variants[*base][view]),
                );
                if indices
                    .iter()
                    .map(|i| scene(&episodes[*i]))
                    .collect::<BTreeSet<_>>()
                    .len()
                    != 8
                {
                    return Err(Error::Invalid("skill batch repeats a base".into()));
                }
                tape.push((indices, rng.state));
            }
        }
    }
    for epoch in tape.as_chunks::<512>().0 {
        let ids: BTreeSet<_> = epoch
            .iter()
            .flat_map(|(ids, _)| ids.iter().copied())
            .collect();
        if ids.len() != 4096 || ids.first() != Some(&0) || ids.last() != Some(&4095) {
            return Err(Error::Corrupt("skill tape exposure".into()));
        }
    }
    Ok(tape)
}
fn skill_score(rows: &[Value]) -> Result<Value> {
    let mut score = summarize(rows)?;
    let mut entity = 0;
    let mut event = 0;
    let mut digit_correct = 0;
    let mut digit_total = 0;
    let (mut context, mut value_correct) = (0, 0);
    let mut first_fields = BTreeMap::<String, usize>::new();
    let mut groups = BTreeMap::<String, [usize; 2]>::new();
    let mut strata = BTreeMap::<String, [usize; 2]>::new();
    for row in rows {
        entity += usize::from(row["components"]["entity"] == true);
        event += usize::from(row["components"]["citation_exact"] == true);
        if let (Some(a), Some(e)) = (
            row["actual"].as_str().and_then(fields),
            row["expected"].as_str().and_then(fields),
        ) {
            context += usize::from(a.1 == e.1);
            value_correct += usize::from(a.2 == e.2);
        }
        if let Some(field) =
            row["teacher_forced_diagnostic_after_generation"]["first_difference_field"].as_str()
        {
            *first_fields.entry(field.into()).or_default() += 1;
        }
        let g = groups
            .entry(row["scene"].as_str().unwrap_or("MISSING").into())
            .or_default();
        g[0] += usize::from(row["exact_match"] == true);
        g[1] += 1;
        if let Some((gold, _, value)) = row["expected"].as_str().and_then(fields) {
            let digits = gold.trim_start_matches(|c: char| !c.is_ascii_digit());
            let actual = row["actual"]
                .as_str()
                .and_then(fields)
                .map(|(e, _, _)| e.trim_start_matches(|c: char| !c.is_ascii_digit()))
                .unwrap_or("");
            digit_total += digits.len();
            digit_correct += digits
                .bytes()
                .enumerate()
                .filter(|(i, b)| actual.as_bytes().get(*i) == Some(b))
                .count();
            for key in [
                format!("digits-{}", digits.len()),
                if value.starts_with("경로") {
                    "short-value".into()
                } else {
                    "direction".into()
                },
                if row["question"]
                    .as_str()
                    .is_some_and(|q| mentions_target(q, gold))
                {
                    "explicit-entity".into()
                } else {
                    "source-only".into()
                },
            ] {
                let s = strata.entry(key).or_default();
                s[0] += usize::from(row["exact_match"] == true);
                s[1] += 1;
            }
        }
    }
    let errors = rows
        .iter()
        .filter(|r| {
            !r["error"].is_null()
                || r["actual"].as_str().is_none_or(str::is_empty)
                || r["whitespace_only"] == true
                || r["finish_reason"] != "stop"
                || !r["eos_index"].is_number()
                || r["generation_completed"] != true
                || !r["interruption"].is_null()
        })
        .count();
    score["entity_correct"] = json!(entity);
    score["event_id_correct"] = json!(event);
    score["context_correct"] = json!(context);
    score["value_correct"] = json!(value_correct);
    score["first_difference_fields"] = json!(first_fields);
    score["digit_accuracy_counts"] = json!([digit_correct, digit_total]);
    score["whole_base_correct_total"] = json!([
        groups.values().filter(|g| g[0] == 4 && g[1] == 4).count(),
        groups.len()
    ]);
    score["strata"] = json!(strata);
    score["generation_error_cases"] = json!(errors);
    score["skill_pass"] = json!(
        rows.len() == 256
            && score["exact_matches"].as_u64().unwrap_or(0) >= 244
            && entity >= 254
            && event >= 254
            && errors == 0
    );
    Ok(score)
}
fn skill_evaluation(
    l: &Loaded,
    dev: &[Episode],
    watch: &[Episode],
    control: &mut RunControl,
) -> Result<Value> {
    let dev_rows = evaluate_panel(l, dev, control);
    let watch_rows = if control.stop.is_none() {
        evaluate_panel(l, watch, control)
    } else {
        vec![]
    };
    let all: Vec<_> = dev_rows.iter().chain(&watch_rows).cloned().collect();
    let mut value = json!({"dev":skill_score(&dev_rows)?,"watch":summarize(&watch_rows)?,"dev_rows":dev_rows,"watch_rows":watch_rows,
        "previous_skill":"NOT_APPLICABLE_H3","oracle":false});
    add_partial_counts(&mut value, &all, dev.len() + watch.len(), control);
    Ok(value)
}
fn skill_error_ids(evaluation: &Value) -> BTreeSet<String> {
    ["dev_rows", "watch_rows"]
        .iter()
        .flat_map(|k| evaluation[*k].as_array().into_iter().flatten())
        .filter(|r| {
            matches!(
                r["error_class"].as_str(),
                Some("strict_utf8" | "control_token")
            ) || r["actual"] == ""
                || r["whitespace_only"] == true
        })
        .filter_map(|r| r["id"].as_str().map(str::to_owned))
        .collect()
}
fn verified_harness(path: &Path) -> Result<Value> {
    let h = read_json(path)?;
    if h["result"] != "CHECKED_SCOPE_PASS" || h["source_unchanged"] != true {
        return Err(Error::Invalid("passing quick harness required".into()));
    }
    let files = h["source_files"]
        .as_object()
        .ok_or_else(|| Error::Invalid("harness source files required".into()))?;
    if ![
        "Cargo.lock",
        "src/training.rs",
        "src/quality_recovery.rs",
        "src/neural/transformer.rs",
    ]
    .iter()
    .all(|p| files.contains_key(*p))
    {
        return Err(Error::Invalid("harness source coverage".into()));
    }
    for (file, hash) in files {
        if file_hash(Path::new(file))? != *hash {
            return Err(Error::Corrupt(format!(
                "source changed after harness: {file}"
            )));
        }
    }
    Ok(h)
}
fn quality_renewal_eligible(previous: &Value, policy: &Value, finish_copy_budget: bool) -> bool {
    previous["reason"] == "QUALITY_GUARD"
        && previous["control"]["terminal_reason"] == "QUALITY_GUARD"
        && previous["control"]["observed_conditions"] == json!(["QUALITY_GUARD"])
        && previous["checkpoint_saved"] == true
        && previous["comparison_eligible"] == false
        && previous["candidate_eligible"] == false
        && previous["resume_allowed"] == false
        && previous["save_error"].is_null()
        && previous.get("save_error").is_some()
        && previous["cleanup_limit_exceeded"] == false
        && previous["last_evaluation"]["final_evaluation_complete"] == true
        && if finish_copy_budget {
            previous["new_updates"] == 512 && policy.get("finish_copy_budget").is_none()
        } else {
            previous["new_updates"]
                .as_u64()
                .is_some_and(|n| n > 0 && n < 512)
                && policy.get("renewal").is_none()
        }
        && previous["bad_streak"] == 0
        && previous["last_evaluation"]["new_error_ids"]
            .as_array()
            .is_some_and(|ids| !ids.is_empty())
}
fn skill_generation_guard(
    evaluation: &Value,
    acknowledged: &BTreeSet<String>,
    finish_copy_budget: bool,
    previous_utf8: &mut usize,
    growth_streak: &mut usize,
) -> bool {
    let rows: Vec<_> = ["dev_rows", "watch_rows"]
        .iter()
        .flat_map(|key| evaluation[*key].as_array().into_iter().flatten())
        .collect();
    let utf8 = rows
        .iter()
        .filter(|r| r["error_class"] == "strict_utf8")
        .count();
    *growth_streak = if utf8 > *previous_utf8 {
        *growth_streak + 1
    } else {
        0
    };
    *previous_utf8 = utf8;
    if finish_copy_budget {
        *growth_streak >= 2
            || rows.iter().any(|r| {
                r["error_class"] == "control_token"
                    || r["actual"] == ""
                    || r["whitespace_only"] == true
            })
    } else {
        skill_error_ids(evaluation)
            .difference(acknowledged)
            .next()
            .is_some()
    }
}
fn skill_run(
    baseline: &Path,
    corpus: &Path,
    output: &Path,
    harness: &Path,
    resume: Option<&Path>,
    renewal: Option<(&Path, bool)>,
    control: &mut RunControl,
) -> Result<()> {
    control.check("skill_start")?;
    let renew_from_quality_stop = renewal.map(|(path, _)| path);
    let finish_copy_budget = renewal.is_some_and(|(_, finish)| finish);
    let checked = verified_harness(harness)?;
    let source = checked["source_digest"]
        .as_str()
        .ok_or_else(|| Error::Invalid("harness source identity".into()))?;
    let base_report = read_json(&baseline.join("summary.json"))?;
    let frozen = load(&baseline.join("frozen.json"))?;
    let prepared = read_json(&corpus.join("prepared.json"))?;
    let (manifest, episodes, dev) = data::load(corpus)?;
    if base_report["baseline_verified"] != true
        || prepared["status"] != "PRETRAIN_STRUCTURE_VERIFIED"
        || prepared["baseline_summary_hash"] != file_hash(&baseline.join("summary.json"))?
        || prepared["manifest_hash"] != file_hash(&corpus.join("manifest.json"))?
        || base_report["parent"]["physical_hash"] != file_hash(&frozen.start)?
        || manifest.train.documents != 4096
        || dev.len() != 256
    {
        return Err(Error::Invalid(
            "verified H2 parent/data prerequisite".into(),
        ));
    }
    if resume.is_some() && renew_from_quality_stop.is_some() {
        return Err(Error::Invalid(
            "plain resume and renewed attempt conflict".into(),
        ));
    }
    let continuation = resume.or(renew_from_quality_stop);
    let mut l = checkpoint::load(continuation.unwrap_or(&frozen.start), Device::Cpu, true)?;
    control.check("skill_loaded")?;
    if l.model.config.profile != "NATIVE_TRPP_G1_SMALL"
        || l.tokenizer.semantic_id() != frozen.tokenizer
    {
        return Err(Error::Invalid("frozen SMALL/tokenizer required".into()));
    }
    let mut state = l
        .manifest
        .training
        .clone()
        .ok_or_else(|| Error::Invalid("inherited Adam required".into()))?;
    if state.config.microbatch != 8
        || state.config.accumulation != 1
        || state.config.sample_group_size != 1
    {
        return Err(Error::Invalid(
            "4+4 batch8 accumulation1/group1 required".into(),
        ));
    }
    let start_step = base_report["parent"]["cumulative_model_step"]
        .as_u64()
        .ok_or_else(|| Error::Corrupt("parent clock".into()))? as usize;
    let start_input = base_report["parent"]["manifest"]["training"]["consumed_tokens"]
        .as_u64()
        .ok_or_else(|| Error::Corrupt("parent tokens".into()))?;
    let start_target = base_report["parent"]["manifest"]["training"]["target_tokens"]
        .as_u64()
        .ok_or_else(|| Error::Corrupt("parent targets".into()))?;
    let rate = base_report["proposed_constant_lr"]
        .as_f64()
        .filter(|v| v.is_finite() && *v > 0. && *v <= 3e-5)
        .ok_or_else(|| Error::Corrupt("constant LR".into()))?;
    let binary_hash = file_hash(&std::env::current_exe()?)?;
    let mut policy;
    let mut previous;
    if let Some(resume) = continuation {
        let previous_root = resume
            .parent()
            .and_then(Path::parent)
            .ok_or_else(|| Error::Invalid("continuation root".into()))?;
        let policy_path = if renew_from_quality_stop.is_some() {
            previous_root.join("policy.json")
        } else {
            output.join("policy.json")
        };
        policy = read_json(&policy_path)?;
        previous = read_json(
            &resume
                .parent()
                .ok_or_else(|| Error::Invalid("resume segment".into()))?
                .join("result.json"),
        )?;
        let terminal_allowed = if renew_from_quality_stop.is_some() {
            quality_renewal_eligible(&previous, &policy, finish_copy_budget)
        } else {
            previous["reason"] == "TIME_BUDGET" && previous["resume_allowed"] == true
        };
        if !terminal_allowed
            || previous["checkpoint_saved"] != true
            || previous["checkpoint_file_sha256"] != file_hash(resume)?
            || previous["model_content_hash"] != l.model.weight_hash()?
            || previous["cumulative_model_step"] != state.step
            || previous["sampler_state"] != state.sampler_state
            || previous["stage_elapsed_seconds"]
                .as_f64()
                .is_none_or(|s| !s.is_finite() || s < 0. || s >= 3600.)
            || (renew_from_quality_stop.is_none()
                && (policy["source_id"] != source || policy["binary_hash"] != binary_hash))
            || policy["train_hash"] != manifest.train.sha256
            || policy["dev_hash"] != manifest.validation.sha256
            || policy["constant_lr"] != rate
            || policy["prepared_hash"] != file_hash(&corpus.join("prepared.json"))?
            || policy["baseline_hash"] != file_hash(&baseline.join("summary.json"))?
            || previous["policy_sha256"] != file_hash(&policy_path)?
            || previous["additional_input_tokens"]
                .as_u64()
                .and_then(|n| start_input.checked_add(n))
                != Some(state.consumed_tokens)
            || previous["additional_target_tokens"]
                .as_u64()
                .and_then(|n| start_target.checked_add(n))
                != Some(state.target_tokens)
        {
            return Err(Error::Invalid("plain skill resume requires matching clean time-bound recovery; no cancellation resume".into()));
        }
        let previous_directory = std::fs::canonicalize(
            resume
                .parent()
                .ok_or_else(|| Error::Invalid("resume segment".into()))?,
        )?;
        for entry in std::fs::read_dir(previous_root)? {
            let path = entry?.path();
            if path.is_dir()
                && path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .is_some_and(|n| n.starts_with("segment-"))
            {
                let receipt = read_json(&path.join("result.json"))?;
                if std::fs::canonicalize(&path)? != previous_directory
                    && receipt["stage_elapsed_seconds"].as_f64().is_none_or(|s| {
                        s >= previous["stage_elapsed_seconds"].as_f64().unwrap_or(0.)
                    })
                {
                    return Err(Error::Invalid(
                        "skill resume must use the latest closed owned segment".into(),
                    ));
                }
            }
        }
        if renew_from_quality_stop.is_some() {
            let mut acknowledged: BTreeSet<String> =
                serde_json::from_value(previous["baseline_error_ids"].clone())?;
            acknowledged.extend(skill_error_ids(&previous["last_evaluation"]));
            policy["renewal"] = json!({"authorization":"explicit renewed user request after reported quality stop; one new attempt, not automatic resume",
                "original_checkpoint":resume,"original_checkpoint_sha256":file_hash(resume)?,"original_result_sha256":file_hash(&resume.parent().unwrap().join("result.json"))?,
                "original_policy_sha256":file_hash(&policy_path)?,"original_reason":previous["reason"],"original_source_id":policy["source_id"],
                "original_binary_hash":policy["binary_hash"],"starting_new_updates":previous["new_updates"],
                "acknowledged_error_ids":acknowledged,"guard":"same watch baseline/streak; additional new UTF-8/control/empty still stop; acceptance requires zero errors"});
            if finish_copy_budget {
                policy["finish_copy_budget"] = json!(true);
                policy["renewal"]["guard"] = json!(
                    "explicitly approved completion to1024 total updates: intermediate UTF-8 count must not increase at two consecutive evaluations; control/empty and original QA/resource/cancel guards retained; unchanged final acceptance"
                );
                previous["extension_allowed"] = json!(true);
            }
            policy["source_id"] = json!(source);
            policy["binary_hash"] = json!(binary_hash);
            policy["harness_hash"] = json!(file_hash(harness)?);
            previous["baseline_error_ids"] = json!(acknowledged);
            previous["previous_dev_correct"] =
                previous["last_evaluation"]["dev"]["exact_matches"].clone();
            std::fs::create_dir(output)?;
            save(&output.join("policy.json"), &policy)?;
        }
    } else {
        if state.step != start_step
            || l.model.weight_hash()? != base_report["parent"]["model_content_hash"]
        {
            return Err(Error::Corrupt("H3 starting state".into()));
        }
        let tape = skill_tape(&episodes, state.sampler_state)?;
        let c = &mut state.config;
        c.lr = rate;
        c.warmup = 0;
        c.budget_start_step = start_step;
        c.max_steps = start_step + 1024;
        c.budget_start_tokens = start_input;
        c.max_tokens = start_input + 6_000_000;
        c.validate_every = 128;
        c.validate(l.model.config.context)?;
        state.previous_corpora.push(state.corpus_hash.clone());
        state.previous_corpora.sort();
        state.previous_corpora.dedup();
        state.corpus_hash = manifest.train.sha256.clone();
        state.validation_hash = manifest.validation.sha256.clone();
        state.train_loss = None;
        state.validation_loss = None;
        state.parent_checkpoint_hash = Some(l.manifest.weights_sha256.clone());
        policy = json!({"stage":"H3","source_id":source,"binary_hash":binary_hash,"harness_hash":file_hash(harness)?,"baseline_hash":file_hash(&baseline.join("summary.json"))?,
            "prepared_hash":file_hash(&corpus.join("prepared.json"))?,"train_hash":manifest.train.sha256,"dev_hash":manifest.validation.sha256,"parent":base_report["parent"],
            "tape":tape,"tape_hash":digest(&tape)?,"config":state.config,"constant_lr":rate,"lr_policy":"explicit constant; native config schedule is not used by this stage",
            "default_updates":512,"maximum_updates":1024,"max_input_tokens":6_000_000,"max_target_tokens":1_500_000,"max_stage_seconds":3600,"command_seconds":900,"cleanup_seconds":120,
            "sampler":"4 distinct anchor bases +4 distinct focus bases; each frozen pool without replacement per512 updates; inherited RNG"});
        std::fs::create_dir(output)?;
        save(&output.join("policy.json"), &policy)?;
        previous = Value::Null;
    }
    if serde_json::to_value(&state.config)? != policy["config"]
        || policy["tape_hash"] != digest(&policy["tape"])?
    {
        return Err(Error::Corrupt("skill config/tape".into()));
    }
    let tape: Vec<(Vec<usize>, u64)> = serde_json::from_value(policy["tape"].clone())?;
    if tape.len() != 1024 || state.step < start_step || state.step > start_step + 1024 {
        return Err(Error::Corrupt("skill remaining budget".into()));
    }
    if continuation.is_some()
        && state.step > start_step
        && state.sampler_state != tape[state.step - start_step - 1].1
    {
        return Err(Error::Corrupt("skill resume RNG/tape position".into()));
    }
    let samples = samples(&episodes, &l.tokenizer, 512)?;
    if samples.iter().any(|s| s.tokens.len() > 513) {
        return Err(Error::Invalid("skill sample would truncate".into()));
    }
    let c = state.config.clone();
    let mut adam = Adam {
        moments: std::mem::take(&mut l.optimizer),
    };
    l.manifest.source_id = source.into();
    let elapsed_before = previous["stage_elapsed_seconds"].as_f64().unwrap_or(0.);
    let segment_number = std::fs::read_dir(output)?
        .filter_map(std::result::Result::ok)
        .filter(|e| {
            e.file_name()
                .to_str()
                .is_some_and(|n| n.starts_with("segment-"))
        })
        .count();
    let segment = output.join(format!(
        "segment-{segment_number:02}-{:04}",
        state.step - start_step
    ));
    std::fs::create_dir(&segment)?;
    let mut log = std::fs::OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(segment.join("trace.jsonl"))?;
    let mut last = previous["last_evaluation"].clone();
    let mut base_errors: BTreeSet<String> = if previous.is_null() {
        BTreeSet::new()
    } else {
        serde_json::from_value(previous["baseline_error_ids"].clone())?
    };
    let mut base_watch = previous["baseline_watch"].as_u64().unwrap_or(0);
    let mut streak = previous["bad_streak"].as_u64().unwrap_or(0);
    let mut previous_dev = previous["previous_dev_correct"].as_u64().unwrap_or(0);
    let mut extension = previous["extension_allowed"].as_bool().unwrap_or(false);
    let finish_copy_budget = policy["finish_copy_budget"] == true;
    if finish_copy_budget
        && resume.is_some()
        && (!previous["previous_utf8_errors"].is_u64() || !previous["utf8_growth_streak"].is_u64())
    {
        return Err(Error::Corrupt("resumed UTF-8 guard state missing".into()));
    }
    let mut previous_utf8 = previous["previous_utf8_errors"]
        .as_u64()
        .unwrap_or_else(|| {
            last["dev"]["invalid_utf8"].as_u64().unwrap_or(0)
                + last["watch"]["invalid_utf8"].as_u64().unwrap_or(0)
        }) as usize;
    let mut utf8_growth_streak = previous["utf8_growth_streak"].as_u64().unwrap_or(0) as usize;
    let mut final_complete = false;
    let mut quality_pass = false;
    let mut full_qa = Value::Null;
    let mut seal_score = Value::Null;
    let outcome = (|| -> Result<()> {
        loop {
            control.check("skill_next_boundary")?;
            if elapsed_before + control.start.elapsed().as_secs_f64() >= 3600. {
                control.observe(StopReason::TimeBudget);
                return control.stop_result();
            }
            let n = state.step - start_step;
            let due = [0, 128, 256, 512, 768, 1024].contains(&n)
                && (last["new_updates"] != n || last["final_evaluation_complete"] != true);
            if due {
                l.model.refresh_identity()?;
                let mut evaluation = skill_evaluation(&l, &dev, &frozen.watch, control)?;
                evaluation["new_updates"] = json!(n);
                evaluation["model_content_hash"] = json!(l.model.weight_hash()?);
                let score = evaluation["dev"]["exact_matches"].as_u64().unwrap_or(0);
                let watch = evaluation["watch"]["exact_matches"].as_u64().unwrap_or(0);
                let errors = skill_error_ids(&evaluation);
                if n == 0 {
                    base_errors = errors.clone();
                    base_watch = watch;
                }
                let new_errors: Vec<_> = errors.difference(&base_errors).cloned().collect();
                let generation_guard = skill_generation_guard(
                    &evaluation,
                    &base_errors,
                    finish_copy_budget,
                    &mut previous_utf8,
                    &mut utf8_growth_streak,
                );
                streak = if base_watch.saturating_sub(watch) >= 3 {
                    streak + 1
                } else {
                    0
                };
                evaluation["new_error_ids"] = json!(new_errors);
                evaluation["utf8_errors"] = json!(previous_utf8);
                evaluation["utf8_growth_streak"] = json!(utf8_growth_streak);
                evaluation["finish_copy_budget_policy"] = json!(finish_copy_budget);
                evaluation["bad_streak"] = json!(streak);
                save(&segment.join(format!("eval-{n:04}.json")), &evaluation)?;
                last = evaluation;
                println!(
                    "NODE=H3 eval updates={n} dev={score}/256 entity={} event={} watch={watch}/32 new_errors={} input={} target={} elapsed_s={:.2}",
                    last["dev"]["entity_correct"],
                    last["dev"]["event_id_correct"],
                    new_errors.len(),
                    state.consumed_tokens - start_input,
                    state.target_tokens - start_target,
                    elapsed_before + control.start.elapsed().as_secs_f64()
                );
                control.check("skill_evaluation_recorded")?;
                save_arm(
                    &mut l,
                    &state,
                    &adam,
                    &segment.join(format!("step-{n:04}")),
                    "RECOVERY_SCREENING",
                )?;
                if streak >= 2 || generation_guard {
                    control.observe(StopReason::QualityGuard);
                    return control.stop_result();
                }
                if n == 512 {
                    extension = score >= previous_dev + 4 && last["dev"]["skill_pass"] != true;
                }
                previous_dev = score;
            }
            if (n == 0 && last["dev"]["skill_pass"] == true)
                || (n == 512 && !extension)
                || n == 1024
            {
                final_complete = last["final_evaluation_complete"] == true;
                if last["dev"]["skill_pass"] == true {
                    if control.start.elapsed().as_secs_f64() > 600. {
                        control.observe(StopReason::TimeBudget);
                        return control.stop_result();
                    }
                    let (_, _, ordinary) = data::load(&frozen.corpus)?;
                    let rows = evaluate_panel(&l, &ordinary, control);
                    full_qa = summarize(&rows)?;
                    save(
                        &segment.join("original400.json"),
                        &json!({"score":full_qa,"rows":rows}),
                    )?;
                    control.check("skill_original_qa_recorded")?;
                    if full_qa["qa"][0].as_u64().unwrap_or(0)
                        < base_report["recount"]["qa"][0].as_u64().unwrap_or(u64::MAX)
                    {
                        control.observe(StopReason::QualityGuard);
                        return control.stop_result();
                    }
                    save(
                        &output.join("seal-attempt.json"),
                        &json!({"model_content_hash":l.model.weight_hash()?,"dev":last["dev"],"new_updates":n}),
                    )?;
                    let descriptor: data::Split = serde_json::from_value(prepared["seal"].clone())?;
                    let seal = data::load_split(corpus, &descriptor)?;
                    let rows = evaluate_panel(&l, &seal, control);
                    seal_score = skill_score(&rows)?;
                    save(
                        &segment.join("seal.json"),
                        &json!({"score":seal_score,"rows":rows}),
                    )?;
                    control.check("skill_seal_recorded")?;
                    quality_pass = seal_score["skill_pass"] == true;
                }
                break;
            }
            let (indices, sampler) = &tape[n];
            control.check("skill_before_batch")?;
            let batch = batch(&samples, indices, &Device::Cpu)?;
            let target_count: usize = indices
                .iter()
                .map(|i| samples[*i].tokens.len() - samples[*i].response_start)
                .sum();
            if state.consumed_tokens - start_input + batch.tokens as u64 > 6_000_000
                || state.target_tokens - start_target + target_count as u64 > 1_500_000
            {
                control.observe(StopReason::TokenBudget);
                return control.stop_result();
            }
            let (ce, obj, targets) = response_loss(
                &l.model.forward(&batch.input, Some(&batch.valid))?,
                &batch,
                c.first_target_weight,
            )?;
            control.check("skill_forward_returned")?;
            let ce = ce.to_scalar::<f32>()?;
            let objective = obj.to_scalar::<f32>()?;
            if !ce.is_finite() || !objective.is_finite() {
                return Err(Error::Model("nonfinite skill loss".into()));
            }
            let gradients = obj.backward()?;
            control.check("skill_backward_returned")?;
            let gradients = l
                .model
                .vars
                .iter()
                .map(|(name, var)| -> Result<_> {
                    let g = gradients
                        .get(var)
                        .ok_or_else(|| Error::Model(format!("missing {name}")))?;
                    Ok((
                        name.clone(),
                        ((g * targets as f64)?.detach() / targets as f64)?,
                    ))
                })
                .collect::<Result<BTreeMap<_, _>>>()?;
            control.check("skill_before_optimizer")?;
            let (norm, delta) =
                adam.step_constant(&l.model.vars, &gradients, &c, state.step + 1, rate)?;
            state.step += 1;
            state.consumed_tokens += batch.tokens as u64;
            state.target_tokens += targets as u64;
            state.sampler_state = *sampler;
            state.train_loss = Some(ce as f64);
            state.validation_loss = None;
            let _ = control.check("skill_optimizer_committed");
            let row = json!({"new_update":n+1,"cumulative_model_step":state.step,"optimizer_step":state.step,"lr":rate,"lr_policy":"constant","sampler_state":sampler,"indices":indices,
                "ids":indices.iter().map(|i|&episodes[*i].id).collect::<Vec<_>>(),"pools":["anchor","anchor","anchor","anchor","focus","focus","focus","focus"],
                "input_tokens":batch.tokens,"target_tokens":targets,"consumed_input_tokens":state.consumed_tokens-start_input,"consumed_target_tokens":state.target_tokens-start_target,
                "ce":ce,"objective":objective,"first_target_weight":c.first_target_weight,"gradient_norm":norm,"update_norm":delta,"rss_kib":control.last_rss_kib,"elapsed_seconds":elapsed_before+control.start.elapsed().as_secs_f64()});
            writeln!(log, "{row}")?;
            log.flush()?;
            if (n + 1).is_multiple_of(32) {
                println!(
                    "NODE=H3 step={} cumulative={} input={} target={} lr={rate} ce={ce:.6}",
                    n + 1,
                    state.step,
                    state.consumed_tokens - start_input,
                    state.target_tokens - start_target
                );
            }
            control.stop_result()?;
        }
        Ok(())
    })();
    if let Err(error) = &outcome {
        control.classify_error(error);
    }
    let mut result = finish_arm(control, final_complete, |reason| {
        save_arm(&mut l, &state, &adam, &segment.join("final"), reason)
    });
    let elapsed = elapsed_before + control.start.elapsed().as_secs_f64();
    let cleanup_overrun = result["cleanup_elapsed_seconds"]
        .as_f64()
        .is_none_or(|seconds| seconds > 120.);
    if cleanup_overrun {
        result["comparison_eligible"] = json!(false);
    }
    let details = json!({"stage":"H3","source_id":source,"policy_sha256":file_hash(&output.join("policy.json"))?,"new_updates":state.step-start_step,"cumulative_model_step":state.step,"sampler_state":state.sampler_state,
        "additional_input_tokens":state.consumed_tokens-start_input,"additional_target_tokens":state.target_tokens-start_target,"stage_elapsed_seconds":elapsed,
        "baseline_error_ids":base_errors,"baseline_watch":base_watch,"bad_streak":streak,"previous_dev_correct":previous_dev,"extension_allowed":extension,"last_evaluation":last,
        "previous_utf8_errors":previous_utf8,"utf8_growth_streak":utf8_growth_streak,"finish_copy_budget_policy":finish_copy_budget,
        "model_content_hash":l.model.weight_hash()?,"full_qa":full_qa,"seal":seal_score,"skill_quality_pass":quality_pass&&control.stop.is_none()&&!cleanup_overrun,
        "candidate_eligible":quality_pass&&control.stop.is_none()&&!cleanup_overrun,"cleanup_limit_exceeded":cleanup_overrun,"error":outcome.as_ref().err().map(ToString::to_string),"goal1_ready":false,"h4":"NOT_RUN_UNTIL_H3_PASS",
        "resume_allowed":control.reason()==Some("TIME_BUDGET") && elapsed<3600. && !cleanup_overrun && !output.join("seal-attempt.json").exists() && result["checkpoint_saved"]==true});
    result
        .as_object_mut()
        .unwrap()
        .extend(details.as_object().unwrap().clone());
    if result["checkpoint_saved"] == true {
        result["checkpoint_file_sha256"] = json!(file_hash(&segment.join("final"))?);
    }
    save(&segment.join("result.json"), &result)?;
    log.sync_all()?;
    println!(
        "NODE=H3 terminal reason={} updates={} input={} target={} skill_pass={} resume_allowed={}",
        result["reason"],
        result["new_updates"],
        result["additional_input_tokens"],
        result["additional_target_tokens"],
        result["skill_quality_pass"],
        result["resume_allowed"]
    );
    if cleanup_overrun {
        return Err(Error::Model(
            "skill cleanup deadline exceeded; checkpoint preserved but ineligible".into(),
        ));
    }
    outcome.and(control.stop_result())
}
fn skill_probe_indices(
    episodes: &[Episode],
    seen: &BTreeSet<usize>,
    exposed: bool,
) -> Result<Vec<usize>> {
    let mut counts = [0usize; 8];
    let mut bases = BTreeSet::new();
    let mut indices = Vec::new();
    for (i, e) in episodes.iter().enumerate().skip(2048) {
        if seen.contains(&i) != exposed {
            continue;
        }
        let entity = fields(&e.answer)
            .ok_or_else(|| Error::Invalid("copy probe fields".into()))?
            .0;
        let digits = entity
            .trim_start_matches(|c: char| !c.is_ascii_digit())
            .len();
        if !(1..=8).contains(&digits) {
            return Err(Error::Invalid("copy probe digit stratum".into()));
        }
        if counts[digits - 1] < 4 && bases.insert(scene(e)) {
            counts[digits - 1] += 1;
            indices.push(i);
        }
    }
    if counts != [4; 8] {
        return Err(Error::Invalid("insufficient copy probe strata".into()));
    }
    Ok(indices)
}
fn skill_diagnose(
    run: &Path,
    corpus: &Path,
    output: &Path,
    control: &mut RunControl,
) -> Result<()> {
    control.check("skill_diagnostic_start")?;
    let policy_path = run
        .parent()
        .ok_or_else(|| Error::Invalid("run directory".into()))?
        .join("policy.json");
    let policy = read_json(&policy_path)?;
    let receipt = read_json(&run.join("result.json"))?;
    let last = &receipt["last_evaluation"];
    let (manifest, train, dev) = data::load(corpus)?;
    let checkpoint = run.join("final");
    let l = checkpoint::load(&checkpoint, Device::Cpu, false)?;
    let state = l
        .manifest
        .training
        .as_ref()
        .ok_or_else(|| Error::Corrupt("skill diagnostic clock".into()))?;
    if receipt["stage"] != "H3"
        || receipt["checkpoint_saved"] != true
        || receipt["checkpoint_file_sha256"] != file_hash(&checkpoint)?
        || receipt["policy_sha256"] != file_hash(&policy_path)?
        || receipt["model_content_hash"] != l.model.weight_hash()?
        || receipt["cumulative_model_step"] != state.step
        || policy["train_hash"] != manifest.train.sha256
        || policy["dev_hash"] != manifest.validation.sha256
        || train.len() != 4096
        || dev.len() != 256
        || last["final_evaluation_complete"] != true
    {
        return Err(Error::Corrupt(
            "skill diagnostic artifact/data binding".into(),
        ));
    }
    let tape: Vec<(Vec<usize>, u64)> = serde_json::from_value(policy["tape"].clone())?;
    let trace: Vec<Value> = std::fs::read_to_string(run.join("trace.jsonl"))?
        .lines()
        .map(serde_json::from_str)
        .collect::<std::result::Result<_, _>>()?;
    let framed = samples(&train, &l.tokenizer, 512)?;
    let mut seen = BTreeSet::new();
    let (mut inputs, mut targets) = (0u64, 0u64);
    for (position, row) in trace.iter().enumerate() {
        control.check("skill_diagnostic_trace")?;
        let (indices, rng) = tape
            .get(position)
            .ok_or_else(|| Error::Corrupt("trace outside tape".into()))?;
        if row["new_update"] != position + 1
            || row["indices"] != json!(indices)
            || row["sampler_state"] != *rng
            || row["ids"]
                != json!(
                    indices
                        .iter()
                        .map(|i| train.get(*i).map(|e| &e.id))
                        .collect::<Vec<_>>()
                )
        {
            return Err(Error::Corrupt(
                "trace/index/RNG mismatch (diagnostic requires initial segment)".into(),
            ));
        }
        let mut input = 0usize;
        let mut target = 0usize;
        for &i in indices {
            let s = framed
                .get(i)
                .ok_or_else(|| Error::Corrupt("trace sample index".into()))?;
            input += s.tokens.len() - 1;
            target += s.tokens.len() - s.response_start;
            seen.insert(i);
        }
        if row["input_tokens"] != input || row["target_tokens"] != target {
            return Err(Error::Corrupt("trace token accounting".into()));
        }
        inputs += input as u64;
        targets += target as u64;
    }
    if receipt["new_updates"] != trace.len()
        || receipt["additional_input_tokens"] != inputs
        || receipt["additional_target_tokens"] != targets
        || trace.is_empty()
    {
        return Err(Error::Corrupt("trace/terminal accounting".into()));
    }
    let exposed = skill_probe_indices(&train, &seen, true)?;
    let unexposed = skill_probe_indices(&train, &seen, false)?;
    let before = l.model.weight_hash()?;
    std::fs::create_dir(output)?;
    let mut panels = Vec::new();
    for (name, indices) in [
        ("exposed_train_views", exposed),
        ("unexposed_train_views", unexposed),
    ] {
        let episodes: Vec<_> = indices.iter().map(|i| train[*i].clone()).collect();
        let rows = evaluate_panel(&l, &episodes, control);
        let panel = json!({"name":name,"indices":indices,"score":skill_score(&rows)?,"rows":rows,
            "heldout":false,"scope":"training views; unexposed views may share a base with an exposed view"});
        save(&output.join(format!("{name}.json")), &panel)?;
        control.check("skill_diagnostic_panel_saved")?;
        println!(
            "H3_DIAGNOSTIC {name} exact={}/32 entity={} event={} utf8={}",
            panel["score"]["exact_matches"],
            panel["score"]["entity_correct"],
            panel["score"]["event_id_correct"],
            panel["score"]["invalid_utf8"]
        );
        panels.push(json!({"name":name,"score":panel["score"]}));
    }
    let new_errors = last["new_error_ids"]
        .as_array()
        .ok_or_else(|| Error::Corrupt("new error IDs".into()))?;
    let rows = last["dev_rows"]
        .as_array()
        .ok_or_else(|| Error::Corrupt("dev rows".into()))?;
    let mut parity = Vec::new();
    for id in new_errors.iter().take(4) {
        let e = dev
            .iter()
            .find(|e| id == &e.id)
            .ok_or_else(|| Error::Corrupt("error case absent".into()))?;
        let row = rows
            .iter()
            .find(|r| r["id"] == *id)
            .ok_or_else(|| Error::Corrupt("error row absent".into()))?;
        if row["question"] != e.request.input
            || row["evidence"] != serde_json::to_value(&e.request.evidence)?
            || row["expected"] != e.answer
        {
            return Err(Error::Corrupt("error row content".into()));
        }
        let raw: Vec<u32> = serde_json::from_value(row["raw_tokens"].clone())?;
        let prompt = l.tokenizer.prepare(
            &e.request,
            l.model.config.context as u32,
            &l.model.config.id()?,
        )?;
        if row["prompt_digest"] != digest(&prompt.token_ids)? || raw.is_empty() || raw.len() > 128 {
            return Err(Error::Corrupt("error prompt/raw bounds".into()));
        }
        let scope = "skill-diagnostic-prefix";
        let mut cache = l.model.cache(scope);
        let mut prefix = prompt.token_ids.clone();
        let mut cached = l.model.forward_cached(
            &Tensor::new(prefix.as_slice(), &Device::Cpu)?.unsqueeze(0)?,
            &mut cache,
            scope,
        )?;
        let mut positions = Vec::new();
        for (position, &token) in raw.iter().enumerate() {
            control.check("skill_diagnostic_cache_prefix")?;
            let full = l
                .model
                .forward(
                    &Tensor::new(prefix.as_slice(), &Device::Cpu)?.unsqueeze(0)?,
                    None,
                )?
                .narrow(1, prefix.len() - 1, 1)?;
            let cached_last = cached.narrow(1, cached.dim(1)? - 1, 1)?;
            let numeric = compare(&full, &cached_last)?;
            let full_token = full.argmax(2)?.flatten_all()?.to_vec1::<u32>()?[0];
            let cached_token = cached_last.argmax(2)?.flatten_all()?.to_vec1::<u32>()?[0];
            positions.push(json!({"position":position,"prefix_tokens":prefix.len(),"recorded":token,"cached":cached_token,"full":full_token,"numeric":numeric}));
            if position + 1 < raw.len() {
                prefix.push(token);
                cached = l.model.forward_cached(
                    &Tensor::new(&[token], &Device::Cpu)?.unsqueeze(0)?,
                    &mut cache,
                    scope,
                )?;
            }
        }
        let bytes = l.tokenizer.decode_bytes(
            &raw.iter()
                .copied()
                .take_while(|t| *t >= neural::SPECIALS as u32)
                .collect::<Vec<_>>(),
        )?;
        let utf8 = std::str::from_utf8(&bytes);
        parity.push(json!({"id":e.id,"positions":positions,"recorded_prefix_reproduced":positions.iter().all(|p|p["recorded"]==p["cached"]&&p["recorded"]==p["full"]),
            "raw_bytes":row["raw_bytes"],"strict_utf8_error":utf8.err().map(|e|e.to_string()),"raw_bytes_hash_matches":row["raw_bytes"]["sha256"]==neural::hash(&bytes),
            "scope":"recorded free-running prefix; no gold fed into forward"}));
    }
    let mut fields = BTreeMap::<String, usize>::new();
    for row in rows {
        if let Some(field) =
            row["teacher_forced_diagnostic_after_generation"]["first_difference_field"].as_str()
        {
            *fields.entry(field.into()).or_default() += 1;
        }
    }
    if before != l.model.weight_hash()? {
        return Err(Error::Corrupt("diagnostic modified weights".into()));
    }
    control.check("skill_diagnostic_done")?;
    save(
        &output.join("summary.json"),
        &json!({"scope":"H3 no-update diagnostic; not a skill gate","actual_optimizer_updates":0,
        "checkpoint_file_sha256":file_hash(&checkpoint)?,"model_content_hash":before,"trace_sha256":file_hash(&run.join("trace.jsonl"))?,
        "input_tokens_verified":inputs,"target_tokens_verified":targets,"completed_updates_verified":trace.len(),
        "unique_exposed_views":seen.len(),"panels":panels,"dev_first_difference_fields":fields,"cache_parity":parity,
        "terminal":control.receipt(),"candidate_eligible":false,"seal":"NOT_OPENED","data_hash":manifest.train.sha256}),
    )?;
    Ok(())
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
fn arm_terminal_eligible(result: &Value) -> bool {
    result["reason"] == "SCREENING_BUDGET_REACHED"
        && result["checkpoint_save_status_reason"] == "SCREENING_BUDGET_REACHED"
        && result["checkpoint_saved"] == true
        && result["final_evaluation_complete"] == true
        && result["comparison_eligible"] == true
        && result.get("save_error").is_some_and(Value::is_null)
        && result.get("error").is_some_and(Value::is_null)
        && result["observed_conditions"]
            .as_array()
            .is_some_and(Vec::is_empty)
        && result["control"]["terminal_reason"] == "COMPLETED"
}
fn verify_arm_receipt(
    directory: &Path,
    policy: &Value,
    trace: &[Value],
    result: &Value,
    fixture_hash: &str,
) -> Result<Value> {
    let invalid = || Error::Corrupt("UNVERIFIED arm completion/provenance".into());
    if !arm_terminal_eligible(result)
        || trace.len() != 50
        || policy["fixture_hash"] != fixture_hash
        || result["new_updates"] != trace.len()
        || result["policy_sha256"] != file_hash(&directory.join("policy.json"))?
        || result["checkpoint_file_sha256"] != file_hash(&directory.join("final"))?
        || result["final_evaluation_sha256"] != file_hash(&directory.join("eval-050.json"))?
        || result["cumulative_model_step"]
            != trace.last().ok_or_else(invalid)?["cumulative_model_step"]
        || policy["tape_hash"] != digest(&policy["tape"])?
        || policy["source_id"].as_str().is_none_or(|s| s.len() != 64)
        || policy["binary_hash"].as_str().is_none_or(|s| s.len() != 64)
    {
        return Err(invalid());
    }
    let evaluation = read_json(&directory.join("eval-050.json"))?;
    let watch = evaluation["watch_rows"].as_array().ok_or_else(invalid)?;
    let train = evaluation["train_exposure_panel"]
        .as_array()
        .ok_or_else(invalid)?;
    if watch.len() != 32
        || train.len() != 16
        || evaluation["final_evaluation_complete"] != true
        || evaluation["comparison_eligible"] != true
        || evaluation["not_run_count"] != 0
        || evaluation["attempted_case_count"] != 48
        || evaluation["completed_generation_count"] != 48
        || evaluation["planned_case_count"] != 48
        || evaluation
            .get("terminal_reason")
            .is_none_or(|v| !v.is_null() && v != "COMPLETED")
        || evaluation["model_step"] != result["cumulative_model_step"]
        || evaluation["model_content_hash"] != result["model_content_hash"]
        || result["last_evaluation"] != evaluation
        || watch
            .iter()
            .chain(train)
            .any(|r| r["generation_completed"] != true || !r["interruption"].is_null())
    {
        return Err(invalid());
    }
    let loaded = checkpoint::load(&directory.join("final"), Device::Cpu, false)?;
    let state = loaded.manifest.training.as_ref().ok_or_else(invalid)?;
    if loaded.model.weight_hash()? != result["model_content_hash"]
        || loaded.manifest.source_id != policy["source_id"]
        || json!(state.step) != result["cumulative_model_step"]
        || serde_json::to_value(&state.config)? != policy["config"]
    {
        return Err(invalid());
    }
    let parent_step = policy["parent"]["manifest"]["training"]["step"]
        .as_u64()
        .ok_or_else(invalid)?;
    let tape = policy["tape"].as_array().ok_or_else(invalid)?;
    if tape.len() != trace.len() {
        return Err(invalid());
    }
    for (i, row) in trace.iter().enumerate() {
        let step = parent_step.checked_add(i as u64 + 1).ok_or_else(invalid)?;
        if row["new_update"] != i + 1
            || row["optimizer_step"] != step
            || row["cumulative_model_step"] != step
            || row["schedule_step"]
                != step
                    .checked_sub(state.config.budget_start_step as u64)
                    .ok_or_else(invalid)?
            || row["lr"].as_f64() != Some(state.config.learning_rate(step as usize))
            || row["indices"] != tape[i][0]
            || row["sampler_state"] != tape[i][1]
            || row["ids"]
                .as_array()
                .is_none_or(|v| v.len() != state.config.microbatch)
            || row["input_tokens"].as_u64().is_none_or(|n| n == 0)
            || row["target_tokens"].as_u64().is_none_or(|n| n == 0)
        {
            return Err(invalid());
        }
    }
    Ok(
        json!({"verified":true,"checkpoint_file_sha256":result["checkpoint_file_sha256"],"policy_sha256":result["policy_sha256"],"final_evaluation_sha256":result["final_evaluation_sha256"]}),
    )
}
fn finish_close(
    output: &Path,
    mut summary: Value,
    arms: &[Value],
    provenance_verified: bool,
    budget: &mut RunControl,
) -> Result<()> {
    let sealed = budget.seal_terminal();
    let eligible = sealed.is_ok()
        && provenance_verified
        && arms.len() == 2
        && arms.iter().all(arm_terminal_eligible);
    summary["arm_terminals"] = json!(arms);
    summary["provenance_verified"] = json!(provenance_verified);
    summary["final_evaluation_complete"] = json!(eligible);
    summary["comparison_eligible"] = json!(eligible);
    summary["candidate_eligible"] = json!(eligible && summary["candidate_eligible"] == true);
    summary["control"] = budget.receipt();
    save(&output.join("summary.json"), &summary)?;
    sealed?;
    if !eligible {
        return Err(Error::Invalid(
            "INELIGIBLE_OR_UNVERIFIED_ARM: original termination retained".into(),
        ));
    }
    Ok(())
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
    let arms: Vec<_> = [control, treatment]
        .iter()
        .map(|dir| {
            read_json(&dir.join("result.json")).unwrap_or_else(
                |e| json!({"reason":"UNKNOWN_UNVERIFIED","read_error":e.to_string()}),
            )
        })
        .collect();
    if !arms.iter().all(arm_terminal_eligible) {
        std::fs::create_dir(output)?;
        return finish_close(
            output,
            json!({"stage":"close_preflight","candidate_eligible":false,"model_calls":0,"status":"INELIGIBLE_OR_UNVERIFIED_ARM"}),
            &arms,
            false,
            budget,
        );
    }
    let f = load(fixture)?;
    let cp = read_json(&control.join("policy.json"))?;
    let wp = read_json(&treatment.join("policy.json"))?;
    let c = trace(&control.join("trace.jsonl"))?;
    let w = trace(&treatment.join("trace.jsonl"))?;
    let mut config = cp["config"].clone();
    config["first_target_weight"] = json!(1.);
    if config != wp["config"]
        || cp["parent"] != wp["parent"]
        || cp["parent"] != f.registry["U2_POLICY_START"]
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
    let provenance = (|| -> Result<_> {
        let fixture_hash = file_hash(fixture)?;
        Ok([
            verify_arm_receipt(control, &cp, &c, &arms[0], &fixture_hash)?,
            verify_arm_receipt(treatment, &wp, &w, &arms[1], &fixture_hash)?,
        ])
    })();
    if let Err(error) = &provenance {
        return finish_close(
            output,
            json!({"stage":"close_provenance","candidate_eligible":false,"model_calls":0,"error":error.to_string()}),
            &arms,
            false,
            budget,
        );
    }
    budget.check("close_provenance_verified")?;
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
    summary["arm_provenance"] = json!(provenance?);
    let sealed = finish_close(output, summary, &arms, true, budget);
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
        "final_evaluation_complete":complete,"comparison_eligible":complete&&control.stop.is_none(),"candidate_eligible":false,"cooperative_only":true,"control":control.receipt()})
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
                evaluation["model_content_hash"] = json!(l.model.weight_hash()?);
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
    result["policy_sha256"] = json!(file_hash(&output.join("policy.json"))?);
    if result["checkpoint_saved"] == true {
        result["checkpoint_file_sha256"] = json!(file_hash(&output.join("final"))?);
    }
    let final_eval = output.join(format!("eval-{:03}.json", state.step - start_step));
    if final_eval.is_file() {
        result["final_evaluation_sha256"] = json!(file_hash(&final_eval)?);
    }
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
    #[test]
    fn skill_explicit_renewal_preserves_stop_and_rejects_other_terminal() {
        let previous = json!({"reason":"QUALITY_GUARD","control":{"terminal_reason":"QUALITY_GUARD","observed_conditions":["QUALITY_GUARD"]},
            "checkpoint_saved":true,"comparison_eligible":false,"candidate_eligible":false,"resume_allowed":false,
            "save_error":null,"cleanup_limit_exceeded":false,"new_updates":128,"bad_streak":0,
            "last_evaluation":{"final_evaluation_complete":true,"new_error_ids":["case/new-invalid"]}});
        let before = previous.clone();
        assert!(quality_renewal_eligible(&previous, &json!({}), false));
        assert_eq!(before, previous);
        for reason in [
            "CANCELLED",
            "TIME_BUDGET",
            "RESOURCE_LIMIT",
            "INTEGRITY_FAIL",
            "COMPLETED",
        ] {
            let mut bad = previous.clone();
            bad["reason"] = json!(reason);
            assert!(!quality_renewal_eligible(&bad, &json!({}), false));
        }
        for key in [
            "save_error",
            "checkpoint_saved",
            "candidate_eligible",
            "resume_allowed",
            "control",
        ] {
            let mut bad = previous.clone();
            bad.as_object_mut().unwrap().remove(key);
            assert!(!quality_renewal_eligible(&bad, &json!({}), false), "{key}");
        }
        for (key, value) in [
            ("save_error", json!("disk error")),
            ("cleanup_limit_exceeded", json!(true)),
            ("bad_streak", json!(2)),
            ("new_updates", json!(512)),
        ] {
            let mut bad = previous.clone();
            bad[key] = value;
            assert!(!quality_renewal_eligible(&bad, &json!({}), false), "{key}");
        }
        assert!(!quality_renewal_eligible(
            &previous,
            &json!({"renewal":{}}),
            false
        ));
        assert!(!quality_renewal_eligible(&previous, &json!({}), true));
        let mut at512 = previous.clone();
        at512["new_updates"] = json!(512);
        assert!(quality_renewal_eligible(
            &at512,
            &json!({"renewal":{}}),
            true
        ));
        assert!(!quality_renewal_eligible(
            &at512,
            &json!({"finish_copy_budget":true}),
            true
        ));
        at512["new_updates"] = json!(1024);
        assert!(!quality_renewal_eligible(&at512, &json!({}), true));
    }
    #[test]
    fn skill_generation_guard_growth_resume_and_unchanged_strict_mode() {
        let evaluation = |n| json!({"dev_rows":(0..n).map(|i|json!({"id":format!("utf8/{i}"),"error_class":"strict_utf8","actual":null})).collect::<Vec<_>>(),"watch_rows":[]});
        let mut previous = 4;
        let mut streak = 0;
        let acknowledged = BTreeSet::from(["utf8/0".to_owned()]);
        assert!(!skill_generation_guard(
            &evaluation(3),
            &acknowledged,
            true,
            &mut previous,
            &mut streak
        ));
        assert_eq!((previous, streak), (3, 0));
        assert!(!skill_generation_guard(
            &evaluation(4),
            &acknowledged,
            true,
            &mut previous,
            &mut streak
        ));
        assert_eq!((previous, streak), (4, 1));
        let persisted = serde_json::to_vec(&(previous, streak)).unwrap();
        let (mut restored_previous, mut restored_streak): (usize, usize) =
            serde_json::from_slice(&persisted).unwrap();
        assert!(skill_generation_guard(
            &evaluation(5),
            &acknowledged,
            true,
            &mut restored_previous,
            &mut restored_streak
        ));
        assert!(!skill_generation_guard(
            &evaluation(4),
            &acknowledged,
            true,
            &mut previous,
            &mut streak
        ));
        assert_eq!(streak, 0);
        assert!(!skill_generation_guard(
            &evaluation(1),
            &acknowledged,
            false,
            &mut previous,
            &mut streak
        ));
        assert!(skill_generation_guard(
            &evaluation(2),
            &acknowledged,
            false,
            &mut previous,
            &mut streak
        ));
        for (key, value) in [
            ("error_class", json!("control_token")),
            ("actual", json!("")),
            ("whitespace_only", json!(true)),
        ] {
            let mut bad = evaluation(1);
            bad["dev_rows"][0][key] = value;
            assert!(skill_generation_guard(
                &bad,
                &acknowledged,
                true,
                &mut previous,
                &mut streak
            ));
        }
    }
    #[test]
    fn skill_diagnostic_exposure_strata_and_precancel() {
        let mut episodes: Vec<_> = (0..4096)
            .map(|i| {
                let mut e = repair_episode(&format!("probe/{}/{i}", i / 4));
                e.answer = format!(
                    "센서{}의 구역7 이동 지시는 직진이다. [event:19]",
                    "1".repeat((i / 4) % 8 + 1)
                );
                e
            })
            .collect();
        let seen = (2048..4096).filter(|i| i % 4 == 0).collect();
        let exposed = skill_probe_indices(&episodes, &seen, true).unwrap();
        let unexposed = skill_probe_indices(&episodes, &seen, false).unwrap();
        assert_eq!(exposed.len(), 32);
        assert_eq!(unexposed.len(), 32);
        assert!(exposed.iter().all(|i| seen.contains(i)));
        assert!(unexposed.iter().all(|i| !seen.contains(i)));
        assert_eq!(
            exposed
                .iter()
                .map(|i| scene(&episodes[*i]))
                .collect::<BTreeSet<_>>()
                .len(),
            32
        );
        assert!(skill_probe_indices(&episodes, &BTreeSet::new(), true).is_err());
        for e in &mut episodes[2048..] {
            e.id = "same-base/0".into();
        }
        assert!(skill_probe_indices(&episodes, &seen, true).is_err());
        let dir = tempfile::tempdir().unwrap();
        let output = dir.path().join("unused");
        let mut control = repair_control();
        control.cancel.store(true, Ordering::Relaxed);
        assert!(skill_diagnose(dir.path(), dir.path(), &output, &mut control).is_err());
        assert!(!output.exists());
        assert_eq!(control.generation_calls, 0);
    }
    #[test]
    fn harness_h3_constant_rate_native_resume_and_precancel() {
        let dir = tempfile::tempdir().unwrap();
        let mut l = repair_loaded();
        let config = TrainConfig {
            seq_len: 32,
            warmup: 0,
            max_steps: 100,
            microbatch: 1,
            accumulation: 1,
            ..Default::default()
        };
        let episode = repair_episode("constant-rate-real-native");
        let framed = samples(&[episode], &l.tokenizer, 32).unwrap();
        let b = batch(&framed, &[0], &Device::Cpu).unwrap();
        let update = |l: &Loaded, adam: &mut Adam, step| {
            let (_, loss, n) =
                response_loss(&l.model.forward(&b.input, Some(&b.valid)).unwrap(), &b, 1.).unwrap();
            let g = loss.backward().unwrap();
            let grads = l
                .model
                .vars
                .iter()
                .map(|(k, v)| (k.clone(), g.get(v).unwrap().detach()))
                .collect();
            adam.step_constant(&l.model.vars, &grads, &config, step, 3e-5)
                .unwrap();
            n
        };
        let mut adam = Adam::new(&l.model.vars).unwrap();
        let n = update(&l, &mut adam, 18);
        let state = TrainingState {
            contrast16: false,
            parent_checkpoint_hash: None,
            config: config.clone(),
            step: 18,
            consumed_tokens: 100,
            target_tokens: n as u64,
            sampler_state: 987,
            corpus_hash: l.tokenizer.train_hash.clone(),
            validation_hash: "3".repeat(64),
            previous_corpora: vec![],
            initial_weight_hash: l.manifest.initial_weight_hash.clone(),
            train_loss: None,
            validation_loss: None,
        };
        save_arm(
            &mut l,
            &state,
            &adam,
            &dir.path().join("native-resume"),
            "RECOVERY_SCREENING",
        )
        .unwrap();
        update(&l, &mut adam, 19);
        let resumed =
            checkpoint::load(&dir.path().join("native-resume"), Device::Cpu, true).unwrap();
        assert_eq!(resumed.manifest.training.as_ref().unwrap().step, 18);
        let mut restored = Adam {
            moments: resumed.optimizer.clone(),
        };
        update(&resumed, &mut restored, 19);
        assert_eq!(
            l.model.weight_hash().unwrap(),
            resumed.model.weight_hash().unwrap()
        );
        for (name, moment) in &adam.moments {
            assert_eq!(
                moment.flatten_all().unwrap().to_vec1::<f32>().unwrap(),
                restored.moments[name]
                    .flatten_all()
                    .unwrap()
                    .to_vec1::<f32>()
                    .unwrap()
            );
        }
        assert_ne!(config.learning_rate(19), 3e-5);
        let mut cancelled = repair_control();
        cancelled.cancel.store(true, Ordering::Relaxed);
        assert!(
            skill_run(
                dir.path(),
                dir.path(),
                &dir.path().join("unused"),
                dir.path(),
                None,
                None,
                &mut cancelled
            )
            .is_err()
        );
        assert_eq!(cancelled.generation_calls, 0);
        assert!(!dir.path().join("unused").exists());
        println!(
            "H3_NUMERIC_REGRESSION actual_TINY_updates=3 SMALL_updates=0; constructed starting clock17 is not training"
        );
    }
    #[test]
    fn harness_h3_tape_mixture_no_repeat_base_and_exact_resume_position() {
        let episodes: Vec<_> = (0..4096)
            .map(|i| {
                repair_episode(&if i < 2048 {
                    format!("anchor/{i}/0")
                } else {
                    format!("focus/{}/{}", (i - 2048) / 4, (i - 2048) % 4)
                })
            })
            .collect();
        let tape = skill_tape(&episodes, 921_917).unwrap();
        assert_eq!(tape.len(), 1024);
        assert_eq!(
            digest(&tape).unwrap(),
            digest(&skill_tape(&episodes, 921_917).unwrap()).unwrap()
        );
        for epoch in tape.as_chunks::<512>().0 {
            let mut seen = BTreeSet::new();
            for (indices, _) in epoch {
                assert!(indices[..4].iter().all(|i| *i < 2048));
                assert!(indices[4..].iter().all(|i| *i >= 2048));
                assert_eq!(
                    indices
                        .iter()
                        .map(|i| scene(&episodes[*i]))
                        .collect::<BTreeSet<_>>()
                        .len(),
                    8
                );
                for i in indices {
                    assert!(seen.insert(*i));
                }
            }
            assert_eq!(seen.len(), 4096);
        }
        let persisted = serde_json::to_vec(&tape).unwrap();
        let restored: Vec<(Vec<usize>, u64)> = serde_json::from_slice(&persisted).unwrap();
        assert_eq!(&tape[193..], &restored[193..]);
        let mut duplicate = episodes;
        duplicate[1].id = duplicate[0].id.clone();
        assert!(skill_tape(&duplicate, 921_917).is_err());
    }
    #[test]
    fn harness_h3_metric_requires_complete_eos_and_entity_citation_thresholds() {
        let row = json!({"expected":"센서31의 구역1 이동 지시는 동쪽이다. [event:7]","actual":"센서31의 구역1 이동 지시는 동쪽이다. [event:7]",
            "question":"센서31의 구역1 원문은?","category":0,"family":"skill/H3/test","exact_match":true,"error":null,"interruption":null,
            "finish_reason":"stop","generation":{"finish":"stop","generated":21},"eos_index":20,"generation_completed":true,"components":{"entity":true,"citation_exact":true}});
        let rows: Vec<_> = (0..256)
            .map(|i| {
                let mut r = row.clone();
                r["id"] = json!(format!("metric/{i}"));
                r["scene"] = json!(format!("base/{}", i / 4));
                r
            })
            .collect();
        assert_eq!(skill_score(&rows).unwrap()["skill_pass"], true);
        assert_eq!(skill_score(&rows).unwrap()["context_correct"], 256);
        assert_eq!(skill_score(&rows).unwrap()["value_correct"], 256);
        for (field, value) in [
            ("finish_reason", json!("length")),
            ("eos_index", Value::Null),
            ("generation_completed", json!(false)),
            ("actual", json!("")),
        ] {
            let mut bad = rows.clone();
            bad[0][field] = value;
            if field == "actual" {
                bad[0]["exact_match"] = json!(false);
            }
            if field == "finish_reason" {
                bad[0]["generation"]["finish"] = json!("length");
                bad[0]["exact_match"] = json!(false);
            }
            assert_eq!(skill_score(&bad).unwrap()["skill_pass"], false, "{field}");
            assert_eq!(skill_score(&bad).unwrap()["denominator"], 256);
        }
        for field in ["entity", "citation_exact"] {
            let mut bad = rows.clone();
            for row in &mut bad[..3] {
                row["components"][field] = json!(false);
                row["exact_match"] = json!(false);
                row["actual"] = json!("잘못된 답변");
            }
            assert_eq!(skill_score(&bad).unwrap()["skill_pass"], false, "{field}");
        }
    }
    #[test]
    fn harness_h3_ordinary_resume_cannot_change_saved_constant_policy() {
        let dir = tempfile::tempdir().unwrap();
        let segment = dir.path().join("segment");
        std::fs::create_dir(&segment).unwrap();
        save(
            &dir.path().join("policy.json"),
            &json!({"stage":"H3","constant_lr":3e-5}),
        )
        .unwrap();
        let checkpoint = segment.join("must-not-load");
        let output = dir.path().join("must-not-create");
        let mut run = training_run(&checkpoint, &output);
        run.resume = true;
        let result = train_controlled(run, &mut repair_control());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("requires recovery skill-run")
        );
        assert!(!output.exists());
    }
    #[test]
    fn harness_h2_copy_materialization_independent_grammar_and_split_rejection() {
        let dir = tempfile::tempdir().unwrap();
        let source = dir.path().join("source");
        let output = dir.path().join("copy");
        data::prepare(&source, 317, 20_000, &[], "entity-cue").unwrap();
        let source_hash = file_hash(&source.join("train.json")).unwrap();
        data::copy_curriculum(&source, &output, 917_260_311).unwrap();
        let (_, train, dev) = data::load(&output).unwrap();
        let seal_descriptor: data::Split =
            serde_json::from_value(read_json(&output.join("seal-manifest.json")).unwrap()).unwrap();
        let seal = data::load_split(&output, &seal_descriptor).unwrap();
        let tok_path = dir.path().join("tokenizer");
        data::tokenizer(&output, &tok_path, 4096).unwrap();
        let tok = ByteBpe::load(&tok_path).unwrap();
        let mut config = neural::transformer::Config::tiny(tok.vocab_size());
        config.profile = "NATIVE_TRPP_EXPERIMENTAL_V1".into();
        config.context = 2048;
        let model = Transformer::init(config, 17, Device::Cpu).unwrap();
        let manifest = checkpoint::initialized(
            &model,
            &tok,
            17,
            neural::hash(b"independent structure test"),
        )
        .unwrap();
        let l = Loaded {
            model,
            tokenizer: tok,
            manifest,
            optimizer: BTreeMap::new(),
        };
        assert_eq!(
            verify_copy_curriculum(&train, &dev, &seal, &l).unwrap()["model_calls"],
            0
        );
        let mut bad = dev.clone();
        bad[0].answer = bad[2].answer.clone();
        assert!(verify_copy_curriculum(&train, &bad, &seal, &l).is_err());
        let mut leaking = dev.clone();
        leaking[..4].clone_from_slice(&train[2048..2052]);
        assert!(verify_copy_curriculum(&train, &leaking, &seal, &l).is_err());
        let mut duplicate = seal;
        let extra = duplicate[0].request.evidence.items[0].clone();
        duplicate[0].request.evidence.items.push(extra);
        assert!(verify_copy_curriculum(&train, &dev, &duplicate, &l).is_err());
        assert_eq!(file_hash(&source.join("train.json")).unwrap(), source_hash);
    }
    fn training_run<'a>(checkpoint: &'a Path, output: &'a Path) -> Run<'a> {
        Run {
            checkpoint,
            corpus: None,
            output,
            resume: false,
            numeric_probe: true,
            config: TrainConfig {
                max_steps: 1,
                warmup: 0,
                seq_len: 64,
                accumulation: 2,
                ..TrainConfig::default()
            },
            stop_after: None,
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
        }
    }
    fn training_initial(path: &Path) -> Loaded {
        let l = repair_loaded();
        checkpoint::save(
            path,
            &l.model,
            &l.tokenizer,
            l.manifest.clone(),
            &l.optimizer,
        )
        .unwrap();
        l
    }
    #[test]
    fn repair_training_precancel_and_deadline_do_no_work() {
        let dir = tempfile::tempdir().unwrap();
        let initial = dir.path().join("initial");
        training_initial(&initial);
        for expired in [false, true] {
            let output = dir.path().join(format!("stopped-{expired}"));
            let mut c = repair_control();
            if expired {
                c.elapsed_override = Some(Duration::from_secs(60));
            } else {
                c.cancel.store(true, Ordering::Relaxed);
            }
            let result = train_controlled(training_run(&initial, &output), &mut c);
            assert!(result.is_err());
            assert_eq!(c.teacher_calls, 0);
            assert!(
                !output.exists(),
                "pre-stopped command must not start validation/save"
            );
        }
    }
    #[test]
    fn repair_training_teacher_cancel_saves_without_more_validation() {
        let dir = tempfile::tempdir().unwrap();
        let initial = dir.path().join("initial");
        let output = dir.path().join("cancelled");
        let l = training_initial(&initial);
        let mut c = repair_control();
        c.hook = Some(Box::new(|boundary, flag| {
            if boundary == "validation_teacher_returned" {
                flag.store(true, Ordering::Relaxed);
            }
        }));
        assert!(matches!(
            train_controlled(training_run(&initial, &output), &mut c),
            Err(Error::Cancelled)
        ));
        assert_eq!(c.teacher_calls, 1);
        let saved = checkpoint::load(&output.join("final"), Device::Cpu, true).unwrap();
        assert_eq!(saved.manifest.status, "CANCELLED");
        assert_eq!(
            saved.model.weight_hash().unwrap(),
            l.model.weight_hash().unwrap()
        );
        let state = saved.manifest.training.unwrap();
        assert_eq!(state.step, 0);
        assert_eq!(
            state.validation_loss, None,
            "partial CE is not a complete validation"
        );
    }
    #[test]
    fn repair_training_aborted_accumulation_keeps_weights_rng_and_token_accounting() {
        let dir = tempfile::tempdir().unwrap();
        let initial = dir.path().join("initial");
        let l = training_initial(&initial);
        for cancellation in [false, true] {
            let output = dir.path().join(format!("abort-{cancellation}"));
            let mut run = training_run(&initial, &output);
            let mut c = repair_control();
            let samples = numeric_samples(&l.tokenizer).unwrap();
            let pool: Vec<_> = (0..samples.len()).collect();
            let mut rng = Rng::new(run.config.seed);
            let first_indices = draw_indices(&pool, &run.config, &mut rng).unwrap();
            let first_input = batch(&samples, &first_indices, &Device::Cpu)
                .unwrap()
                .tokens as u64;
            if cancellation {
                c.hook = Some(Box::new(|boundary, flag| {
                    if boundary == "training_microbatch_returned" {
                        flag.store(true, Ordering::Relaxed);
                    }
                }));
            } else {
                let second_indices = draw_indices(&pool, &run.config, &mut rng).unwrap();
                let second_input = batch(&samples, &second_indices, &Device::Cpu)
                    .unwrap()
                    .tokens as u64;
                run.config.max_tokens = first_input + second_input - 1;
            }
            let seed = run.config.seed;
            let result = train_controlled(run, &mut c);
            if cancellation {
                assert!(matches!(result, Err(Error::Cancelled)));
            } else {
                result.unwrap();
            }
            let saved = checkpoint::load(&output.join("final"), Device::Cpu, true).unwrap();
            let state = saved.manifest.training.unwrap();
            assert_eq!(state.step, 0);
            assert_eq!(
                state.consumed_tokens, first_input,
                "aborted computation must still consume budget"
            );
            assert_eq!(state.target_tokens, 0);
            assert_eq!(state.sampler_state, seed);
            assert_eq!(
                saved.model.weight_hash().unwrap(),
                l.model.weight_hash().unwrap()
            );
            for moment in saved.optimizer.values() {
                assert!(
                    moment
                        .flatten_all()
                        .unwrap()
                        .to_vec1::<f32>()
                        .unwrap()
                        .iter()
                        .all(|v| *v == 0.)
                );
            }
            assert_eq!(
                c.teacher_calls, 2,
                "no final re-evaluation after an aborted update"
            );
        }
    }
    #[test]
    fn repair_training_save_failure_keeps_cancel_and_existing_artifact() {
        let dir = tempfile::tempdir().unwrap();
        let initial = dir.path().join("initial");
        let output = dir.path().join("cancelled");
        training_initial(&initial);
        let final_path = output.join("final");
        let existing = final_path.clone();
        let mut c = repair_control();
        c.hook = Some(Box::new(move |boundary, flag| {
            if boundary == "validation_teacher_returned" {
                std::fs::write(&existing, b"existing artifact must survive").unwrap();
                flag.store(true, Ordering::Relaxed);
            }
        }));
        assert!(matches!(
            train_controlled(training_run(&initial, &output), &mut c),
            Err(Error::Cancelled)
        ));
        assert_eq!(c.reason(), Some("CANCELLED"));
        assert_eq!(c.teacher_calls, 1);
        assert_eq!(
            std::fs::read(final_path).unwrap(),
            b"existing artifact must survive"
        );
    }
    #[test]
    fn repair_training_terminal_cancel_and_positive_completion() {
        let dir = tempfile::tempdir().unwrap();
        let initial = dir.path().join("initial");
        training_initial(&initial);
        for boundary in ["training_checkpoint_preserved", "terminal", "never"] {
            let output = dir.path().join(boundary);
            let mut c = repair_control();
            c.hook = Some(Box::new(move |at, flag| {
                if at == boundary {
                    flag.store(true, Ordering::Relaxed);
                }
            }));
            let mut run = training_run(&initial, &output);
            run.stop_after = Some(0); // Exercise finalization without optimizer work.
            run.measure_rss = boundary != "never";
            let result = train_controlled(run, &mut c);
            if boundary == "never" {
                result.unwrap();
                assert_eq!(c.receipt()["terminal_reason"], "COMPLETED");
                assert_eq!(c.receipt()["rss_observation_enabled"], false);
                assert_eq!(c.last_rss_kib, None);
                c.cancel.store(true, Ordering::Relaxed);
                c.check("after_terminal").unwrap();
            } else {
                assert!(matches!(result, Err(Error::Cancelled)), "{boundary}");
                assert_eq!(c.receipt()["terminal_reason"], "CANCELLED");
            }
            assert_eq!(c.teacher_calls, 2);
            let saved = checkpoint::load(&output.join("final"), Device::Cpu, true).unwrap();
            assert_eq!(saved.manifest.training.unwrap().step, 0);
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
    fn harness_m03_mixed_related_causal_records_fail_actual_scan() {
        let mut request = independent_request();
        request.input = "센서31의 사고 원인은 무엇인가?".into();
        request.evidence.items.truncate(2);
        request.evidence.items[0].original_excerpt =
            "이후 센서31의 사고가 기록되었다. 원인은 확인되지 않았다.".into();
        request.evidence.items[1].original_excerpt = "센서31의 사고 원인은 충돌이다.".into();
        let answer = format!(
            "원인은 확정되지 않았습니다. [event:{}]",
            request.evidence.items[0].event_id
        );
        for _ in 0..2 {
            let report = repair_semantic_scan(&request, &answer);
            assert_eq!(report["validated"], 0, "{report}");
            assert_eq!(audit_status(&[&report], &[]), "AUDIT_INCOMPLETE");
            assert_eq!(report["contradicted"], 0);
            request.evidence.items.reverse();
        }
        request.evidence.items.truncate(1);
        let positive = repair_semantic_scan(&request, &answer);
        assert_eq!(positive["validated"], 1);
        let mut other = request.evidence.items[0].clone();
        other.event_id += 100;
        other.original_excerpt = "센서310의 사고 원인은 충돌이다.".into();
        request.evidence.items.push(other);
        let unrelated = repair_semantic_scan(&request, &answer);
        assert_eq!(unrelated["validated"], 1);
        assert_eq!(
            unrelated["causal_record_classifications"][1]["classification"],
            "IRRELEVANT_EXPLICIT_OTHER_ENTITY"
        );
        request.evidence.items[1].original_excerpt =
            "추가 사고의 원인에 관한 별도 조사 결과가 있다.".into();
        let unknown = repair_semantic_scan(&request, &answer);
        assert_eq!(unknown["ambiguous"], 1);
        assert_eq!(audit_status(&[&unknown], &[]), "AUDIT_INCOMPLETE");
        request.evidence.items[1].original_excerpt =
            request.evidence.items[0].original_excerpt.clone();
        request.evidence.items[1].recorded_at = request.evidence.items[0].recorded_at;
        let multiple = repair_semantic_scan(&request, &answer);
        assert_eq!(multiple["ambiguous"], 1);
        request.evidence.items.truncate(1);
        let wrong = format!(
            "원인은 확정되었습니다. [event:{}]",
            request.evidence.items[0].event_id
        );
        let contradiction = repair_semantic_scan(&request, &wrong);
        assert_eq!(contradiction["contradicted"], 1);
    }
    #[test]
    fn harness_m04_close_preserves_cancel_before_missing_model_work() {
        let dir = tempfile::tempdir().unwrap();
        let c = dir.path().join("C");
        let w = dir.path().join("W");
        std::fs::create_dir(&c).unwrap();
        std::fs::create_dir(&w).unwrap();
        let mut arm_control = repair_control();
        arm_control.observe(StopReason::Cancelled);
        let result = finish_arm(&mut arm_control, false, |_| Ok(()));
        for arm in [&c, &w] {
            save(&arm.join("result.json"), &result).unwrap();
        }
        let mut budget = repair_control();
        let out = dir.path().join("close");
        assert!(
            close(
                &dir.path().join("unused-fixture"),
                &c,
                &w,
                &dir.path().join("unused-tokenizer"),
                &dir.path().join("unused-worker"),
                &"0".repeat(64),
                &out,
                &mut budget
            )
            .is_err()
        );
        assert_eq!(budget.generation_calls, 0);
        let summary = read_json(&out.join("summary.json"))
            .expect("close must retain the actual arm disqualification");
        assert_eq!(summary["comparison_eligible"], false);
        assert_eq!(summary["candidate_eligible"], false);
        assert!(summary.to_string().contains("CANCELLED"));
    }
    #[test]
    fn harness_m04_finish_arm_to_close_terminal_positive_and_negative() {
        let l = repair_loaded();
        let watch = vec![repair_episode("watch/0")];
        let train = vec![repair_episode("train/0")];
        for boundary in [
            "complete",
            "panel_completed",
            "before_teacher",
            "terminal",
            "explicit_false",
            "missing",
            "save_error",
        ] {
            let dir = tempfile::tempdir().unwrap();
            let mut c = repair_control();
            if matches!(boundary, "panel_completed" | "before_teacher" | "terminal") {
                let mut seen = 0;
                c.hook = Some(Box::new(move |at, flag| {
                    if at == "panel_completed" {
                        seen += 1;
                    }
                    // Stop after watch, or inside the final train teacher/finalization boundary.
                    if at == boundary && (boundary != "before_teacher" || seen == 1) {
                        flag.store(true, Ordering::Relaxed);
                    }
                }));
            }
            let evaluation = arm_evaluation(&l, &watch, &train, &mut c).unwrap();
            let mut arm = finish_arm(
                &mut c,
                evaluation["final_evaluation_complete"] == true,
                |_| {
                    if boundary == "save_error" {
                        Err(Error::Invalid("independent save failure".into()))
                    } else {
                        Ok(())
                    }
                },
            );
            arm["error"] = Value::Null;
            if boundary == "explicit_false" {
                arm["comparison_eligible"] = json!(false);
            }
            if boundary == "missing" {
                arm.as_object_mut()
                    .unwrap()
                    .remove("final_evaluation_complete");
            }
            let mut close_control = repair_control();
            let result = finish_close(
                dir.path(),
                json!({"candidate_eligible":true}),
                &[arm.clone(), arm],
                true,
                &mut close_control,
            );
            let report = read_json(&dir.path().join("summary.json")).unwrap();
            assert_eq!(
                result.is_ok(),
                boundary == "complete",
                "{boundary}: {report}"
            );
            assert_eq!(report["comparison_eligible"], boundary == "complete");
            assert_eq!(report["candidate_eligible"], boundary == "complete");
            assert_eq!(close_control.generation_calls, 0);
        }
        let mut c = repair_control();
        let mut arm = finish_arm(&mut c, true, |_| Ok(()));
        arm["error"] = Value::Null;
        for failure in [
            "TIME_BUDGET",
            "RESOURCE_LIMIT",
            "QUALITY_GUARD",
            "INTEGRITY_FAIL",
        ] {
            let mut stopped = arm.clone();
            stopped["reason"] = json!(failure);
            assert!(!arm_terminal_eligible(&stopped));
        }
        let dir = tempfile::tempdir().unwrap();
        let mut final_cancel = repair_control();
        final_cancel.cancel.store(true, Ordering::Relaxed);
        assert!(
            finish_close(
                dir.path(),
                json!({"candidate_eligible":true}),
                &[arm.clone(), arm],
                true,
                &mut final_cancel
            )
            .is_err()
        );
        assert_eq!(
            read_json(&dir.path().join("summary.json")).unwrap()["candidate_eligible"],
            false
        );
    }
    #[test]
    fn harness_m04_native_checkpoint_and_eval_binding_before_close_gate() {
        // State/receipt fixture only: fifty historical clocks, ZERO optimizer updates.
        let dir = tempfile::tempdir().unwrap();
        let mut l = repair_loaded();
        let config = TrainConfig {
            seq_len: 32,
            max_steps: 100,
            warmup: 0,
            accumulation: 1,
            ..Default::default()
        };
        l.manifest.training = Some(TrainingState {
            contrast16: false,
            parent_checkpoint_hash: Some("1".repeat(64)),
            config: config.clone(),
            step: 50,
            consumed_tokens: 100,
            target_tokens: 50,
            sampler_state: 50,
            corpus_hash: l.tokenizer.train_hash.clone(),
            validation_hash: "3".repeat(64),
            previous_corpora: vec![],
            initial_weight_hash: l.manifest.initial_weight_hash.clone(),
            train_loss: None,
            validation_loss: None,
        });
        l.manifest.status = "BUDGET_EXHAUSTED".into();
        l.optimizer = Adam::new(&l.model.vars).unwrap().moments;
        checkpoint::save(
            &dir.path().join("final"),
            &l.model,
            &l.tokenizer,
            l.manifest.clone(),
            &l.optimizer,
        )
        .unwrap();
        let tape: Vec<_> = (1..=50).map(|i| json!([[i], i])).collect();
        let policy = json!({"fixture_hash":"4".repeat(64),"source_id":l.manifest.source_id,"binary_hash":"5".repeat(64),"config":config,"parent":{"manifest":{"training":{"step":0}}},"tape":tape,"tape_hash":digest(&tape).unwrap()});
        save(&dir.path().join("policy.json"), &policy).unwrap();
        let trace: Vec<_> = (1..=50).map(|i|json!({"new_update":i,"cumulative_model_step":i,"optimizer_step":i,"schedule_step":i,"lr":config.learning_rate(i),"indices":[i],"ids":[format!("fixture-{i}")],"sampler_state":i,"input_tokens":2,"target_tokens":1})).collect();
        let row = json!({"generation_completed":true,"interruption":null});
        let eval = json!({"watch_rows":vec![row.clone();32],"train_exposure_panel":vec![row;16],"final_evaluation_complete":true,"comparison_eligible":true,"not_run_count":0,"attempted_case_count":48,"completed_generation_count":48,"planned_case_count":48,"terminal_reason":null,"model_step":50,"model_content_hash":l.model.weight_hash().unwrap()});
        save(&dir.path().join("eval-050.json"), &eval).unwrap();
        let mut control = repair_control();
        let mut arm = finish_arm(&mut control, true, |_| Ok(()));
        for (key, value) in [
            ("error", Value::Null),
            ("new_updates", json!(50)),
            ("cumulative_model_step", json!(50)),
            (
                "policy_sha256",
                json!(file_hash(&dir.path().join("policy.json")).unwrap()),
            ),
            (
                "checkpoint_file_sha256",
                json!(file_hash(&dir.path().join("final")).unwrap()),
            ),
            (
                "final_evaluation_sha256",
                json!(file_hash(&dir.path().join("eval-050.json")).unwrap()),
            ),
            ("last_evaluation", eval.clone()),
            ("model_content_hash", json!(l.model.weight_hash().unwrap())),
        ] {
            arm[key] = value;
        }
        let verified =
            verify_arm_receipt(dir.path(), &policy, &trace, &arm, &"4".repeat(64)).unwrap();
        assert_eq!(verified["verified"], true);
        let good = dir.path().join("good-close");
        std::fs::create_dir(&good).unwrap();
        finish_close(
            &good,
            json!({"candidate_eligible":true}),
            &[arm.clone(), arm.clone()],
            true,
            &mut repair_control(),
        )
        .unwrap();
        for key in [
            "checkpoint_file_sha256",
            "model_content_hash",
            "final_evaluation_sha256",
            "policy_sha256",
        ] {
            let mut bad = arm.clone();
            bad[key] = json!("f".repeat(64));
            assert!(
                verify_arm_receipt(dir.path(), &policy, &trace, &bad, &"4".repeat(64)).is_err(),
                "{key}"
            );
        }
        let mut bad_clock = trace.clone();
        bad_clock[49]["optimizer_step"] = json!(49);
        assert!(
            verify_arm_receipt(dir.path(), &policy, &bad_clock, &arm, &"4".repeat(64)).is_err()
        );
        let mut partial = eval;
        partial["train_exposure_panel"] = json!([]);
        partial["final_evaluation_complete"] = json!(false);
        std::fs::write(
            dir.path().join("eval-050.json"),
            serde_json::to_vec(&partial).unwrap(),
        )
        .unwrap();
        arm["final_evaluation_sha256"] =
            json!(file_hash(&dir.path().join("eval-050.json")).unwrap());
        arm["last_evaluation"] = partial;
        assert!(verify_arm_receipt(dir.path(), &policy, &trace, &arm, &"4".repeat(64)).is_err());
        assert_eq!(control.generation_calls, 0);
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
