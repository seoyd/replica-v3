//! Bounded training-only recovery diagnostics; never imported by the product library.
use super::*;
use clap::Subcommand;
use replica_v3::{model::ModelRequest, neural::checkpoint::Loaded};
use serde::{Deserialize, Serialize};
use replica_v3::binary::{Value, record};
use std::{collections::BTreeSet, io::Write, path::PathBuf, sync::Arc, time::Duration};
#[path = "experiment_record.rs"]
mod experiment_record;
pub(super) fn reject_unbound_objective_resume(path: &Path) -> Result<()> {
    experiment_record::reject_unbound_objective_resume(path)
}
pub(super) fn evaluate_native_source(
    checkpoint: &Path,
    corpus: &Path,
    output: &Path,
    limit: usize,
    split: &str,
    control: &mut RunControl,
) -> Result<()> {
    experiment_record::evaluate_source(checkpoint, corpus, output, limit, split, control)
}

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
    generation_limit: usize,
    completed_generation_count: usize,
    attempted_case_count: usize,
    interrupted_case_id: Option<String>,
    pub(super) teacher_calls: usize,
    pub(super) teacher_limit: usize,
    #[cfg(feature = "test-support")]
    pub(super) fixture_post_generation_deadline: bool,
    #[cfg(feature = "test-support")]
    pub(super) fixture_boundary: Option<String>,
    #[cfg(test)]
    elapsed_override: Option<Duration>,
    #[cfg(test)]
    time_boundary: Option<&'static str>,
    #[cfg(test)]
    #[allow(clippy::type_complexity)]
    hook: Option<Box<dyn FnMut(&str, &Arc<AtomicBool>)>>,
}
impl RunControl {
    pub(super) fn cancellation(&self) -> Arc<AtomicBool> { self.cancel.clone() }
    pub(super) fn deadline(&self) -> Instant { self.deadline }
    pub(super) fn restrict_seconds(&mut self, seconds:f64) -> Result<()> {
        if !seconds.is_finite() || seconds<=0. { return Err(Error::Invalid("active time budget exhausted".into())); }
        self.deadline=self.deadline.min(self.start+Duration::from_secs_f64(seconds));Ok(())
    }
    pub(super) fn begin_external_generation(&mut self) -> Result<()> {
        self.effective_timeout(u64::MAX)?;
        self.generation_calls+=1;self.attempted_case_count+=1;Ok(())
    }
    pub(super) fn returned_external_generation(&mut self) { self.completed_generation_count+=1; }
    /// Caller must first verify all work and the durable endpoint. This seals
    /// command accounting without converting a pure time stop into work failure.
    pub(super) fn seal_completed_no_call(&mut self) -> Result<()> {
        let _=self.check("completed_no_call");
        self.terminal=true;
        if self.observed.iter().all(|s|*s==StopReason::TimeBudget) {Ok(())} else {self.stop_result()}
    }
    #[cfg(feature="test-support")]
    pub(super) fn fixture_deadline(&mut self){self.deadline=Instant::now();}
    #[cfg(feature = "test-support")]
    pub(super) fn fixture_native_timeout(&mut self, tokens: usize) {
        self.deadline = Instant::now() + Duration::from_secs(30);
        neural::transformer::fixture_timeout_after(tokens);
    }
    pub(super) fn set_call_limits(&mut self, generation: usize, teacher: usize) {
        self.generation_limit = generation;
        self.teacher_limit = teacher;
    }
    pub(super) fn restrict_rss(&mut self, max_rss_kib:u64) {
        self.max_rss_kib=self.max_rss_kib.min(max_rss_kib);
    }
    pub(super) fn begin_teacher(&mut self) -> Result<()> {
        self.begin_teacher_rows(1)
    }
    // Batched diagnostics reserve their actual sample-forward rows atomically.
    // Backend microbatch invocations are recorded separately by the observer.
    pub(super) fn begin_teacher_rows(&mut self, rows: usize) -> Result<()> {
        self.check("before_teacher_budget")?;
        if rows == 0 || self.teacher_calls.checked_add(rows).is_none_or(|n|n>self.teacher_limit) {
            self.observe(StopReason::TokenBudget);
            return self.stop_result();
        }
        self.teacher_calls += rows;
        Ok(())
    }
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
            generation_limit: usize::MAX,
            completed_generation_count: 0,
            attempted_case_count: 0,
            interrupted_case_id: None,
            teacher_calls: 0,
            teacher_limit: usize::MAX,
            #[cfg(feature = "test-support")]
            fixture_post_generation_deadline: false,
            #[cfg(feature = "test-support")]
            fixture_boundary: None,
            #[cfg(test)]
            elapsed_override: None,
            #[cfg(test)]
            time_boundary: None,
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
        #[cfg(feature = "test-support")]
        if self.fixture_boundary.as_deref() == Some(boundary) {
            self.deadline = Instant::now();
            if std::env::var("R3_FRESH_CALL_CANCEL").as_deref() == Ok("1") {
                self.cancel.store(true, Ordering::Relaxed);
            }
        }
        #[cfg(feature = "test-support")]
        if boundary == "generation_returned" && self.fixture_post_generation_deadline {
            self.deadline = Instant::now();
        }
        #[cfg(test)]
        if self.time_boundary == Some(boundary) {
            self.elapsed_override = Some(self.deadline.duration_since(self.start));
        }
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
        if self.generation_calls >= self.generation_limit {
            self.observe(StopReason::TokenBudget);
            return self.stop_result().map(|_|0);
        }
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
        } else if matches!(e, Error::Corrupt(_) | Error::Io(_) | Error::Binary(_) | Error::Tensor(_) | Error::Invalid(_) | Error::Conflict(_))
            || e.to_string().contains("nonfinite") || self.stop.is_none() {
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
        record!({"terminal_reason":self.stop.map(StopReason::name).or_else(||self.terminal.then_some("COMPLETED")),"observed_conditions":self.observed,"generation_calls":self.generation_calls,"teacher_calls":self.teacher_calls,"completed_generation_count":self.completed_generation_count,"attempted_case_count":self.attempted_case_count,"interrupted_case_id":self.interrupted_case_id,
            "elapsed_seconds":self.now().duration_since(self.start).as_secs_f64(),"work_budget_seconds":self.deadline.duration_since(self.start).as_secs_f64(),
            "work_deadline_overrun_seconds":self.now().saturating_duration_since(self.deadline).as_secs_f64(),"rss_observation_enabled":self.measure_rss,"cooperative_only":true})
    }
}

#[derive(Subcommand)]
pub enum Command {
    /// Typed binary experiment records, explicit legacy import and verified native resume.
    Native {
        #[command(subcommand)]
        action: experiment_record::Action,
    },
    /// Conditional F/N registration after a closed, safe, improving A1 comparison.
    ProgressRenewal {
        #[arg(long)]
        experiment: PathBuf,
        #[arg(long)]
        harness: PathBuf,
        #[arg(long)]
        output: PathBuf,
        #[arg(long)]
        seed: u64,
    },
    /// Register both independent LR forks before either optimizer is called.
    ProgressPrepare {
        #[arg(long)]
        a0: PathBuf,
        #[arg(long)]
        harness: PathBuf,
        #[arg(long)]
        output: PathBuf,
    },
    /// Execute one registered controlled-progress arm; only clean time stops can resume.
    ProgressArm {
        #[arg(long)]
        experiment: PathBuf,
        #[arg(long)]
        arm: String,
        #[arg(long)]
        resume: Option<PathBuf>,
    },
    /// Compare actual common prefixes and choose a safe endpoint without opening the seal.
    ProgressClose {
        #[arg(long)]
        experiment: PathBuf,
    },
    /// Recount immutable legacy panels; never changes historical eligibility or resumes learning.
    ProgressReaudit {
        #[arg(long)]
        experiment: PathBuf,
        #[arg(long)]
        output: PathBuf,
        #[arg(long, value_parser=["F16","N16","N256"])]
        observe: Option<String>,
    },
    /// A0 only: immutable H3 parent replay, ordinary QA, token/QK/exposure and crossed development.
    ProgressBaseline {
        #[arg(long)]
        baseline: PathBuf,
        #[arg(long)]
        run: PathBuf,
        #[arg(long)]
        inference: PathBuf,
        #[arg(long)]
        corpus: PathBuf,
        #[arg(long)]
        harness: PathBuf,
        #[arg(long)]
        output: PathBuf,
        #[arg(long)]
        seed: u64,
    },
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
    Ok(neural::hash(&replica_v3::binary::to_vec(value)?))
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
    neural::write_new(path, &replica_v3::binary::to_storage_vec(value)?)
}
fn load(path: &Path) -> Result<Frozen> {
    let f: Frozen = replica_v3::binary::from_slice(&neural::read_bounded(path, 16 * 1024 * 1024)?)?;
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
    let values = replica_v3::binary::value_records_from_slice(&bytes)?;
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
        let matched = row["expected"].is_string()
            && strict_answer_match(
                row["actual"].as_str(),
                row["expected"].as_str().unwrap_or_default(),
                row["generation"]["finish"] == "stop",
                !error.is_empty(),
            );
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
        record!({"summary":true,"denominator":rows.len(),"exact_matches":qa[0]+aux[0],"qa":qa,"auxiliary":aux,
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
    let schedule: Vec<_> = [0,1,5,20,50,100].iter().map(|&n| record!({"segment_step":n,"model_step":s.config.budget_start_step+n,"lr":s.config.learning_rate(s.config.budget_start_step+n)})).collect();
    Ok(
        record!({"path":path,"physical_hash":file_hash(path)?,"model_content_hash":l.model.weight_hash()?,"manifest":l.manifest,
        "tokenizer_wire":l.tokenizer.id(),"tokenizer_semantic":l.tokenizer.semantic_id(),"architecture_id":l.model.config.id()?,
        "dtype":"F32","backend":neural::cpu_backend(),"adam_shapes":l.optimizer.iter().map(|(k,v)|(k,v.dims())).collect::<BTreeMap<_,_>>(),
        "cumulative_model_step":s.step,"optimizer_step":s.step,"schedule_step":s.step-s.config.budget_start_step,
        "segment_update_count":s.step-s.config.budget_start_step,"current_lr_derived_config":s.config.learning_rate(s.step),"schedule_derived_config":schedule}),
    )
}
pub fn run(command: Command) -> Result<()> {
    if matches!(
        command,
        Command::SkillRun { .. } | Command::ProgressArm { .. } | Command::Arm { .. }
    ) {
        return Err(Error::Invalid("legacy optimizer control has no supported v2 execution binding; use explicit migration and bound recovery native run; optimizer_calls=0".into()));
    }
    if let Command::Native { action } = command {
        return experiment_record::command(action);
    }
    let mut budget = RunControl::command(matches!(
        &command,
        Command::Arm { .. } | Command::SkillRun { .. } | Command::ProgressArm { .. }
    ))?;
    if matches!(
        &command,
        Command::ProgressBaseline { .. }
            | Command::ProgressArm { .. }
            | Command::ProgressReaudit { .. }
    ) {
        budget.deadline = budget.start + Duration::from_secs(1800);
    }
    budget.check("command_started")?;
    let outcome = match command {
        Command::Native { .. } => unreachable!("native command dispatched before legacy control"),
        Command::ProgressRenewal {
            experiment,
            harness,
            output,
            seed,
        } => progress_renewal(&experiment, &harness, &output, seed, &mut budget),
        Command::ProgressPrepare {
            a0,
            harness,
            output,
        } => progress_prepare(&a0, &harness, &output, &mut budget),
        Command::ProgressArm {
            experiment,
            arm,
            resume,
        } => progress_arm(&experiment, &arm, resume.as_deref(), &mut budget),
        Command::ProgressClose { experiment } => progress_close(&experiment, &mut budget),
        Command::ProgressReaudit {
            experiment,
            output,
            observe,
        } => {
            if let Some(observe) = observe {
                progress_observe(&experiment, &output, &observe, &mut budget)
            } else {
                std::fs::create_dir(&output)?;
                progress_close_to(&experiment, &output.join("reaudit.r3b"), true, &mut budget)
            }
        }
        Command::ProgressBaseline {
            baseline,
            run,
            inference,
            corpus,
            harness,
            output,
            seed,
        } => progress_baseline(
            [&baseline, &run, &inference, &corpus, &harness],
            &output,
            seed,
            &mut budget,
        ),
        Command::SkillRecount { evaluation, output } => {
            let recorded = read_metadata(&evaluation)?;
            let dev = recorded["dev_rows"]
                .as_array()
                .ok_or_else(|| Error::Corrupt("skill dev rows".into()))?;
            let watch = recorded["watch_rows"]
                .as_array()
                .ok_or_else(|| Error::Corrupt("skill watch rows".into()))?;
            budget.check("skill_recount")?;
            save(
                &output,
                &record!({"evidence_level":"DERIVED_FROM_EXISTING_LOGS","evaluation_sha256":file_hash(&evaluation)?,
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
            let report = read_metadata(&baseline.join("summary.r3b"))?;
            let f = load(&baseline.join("frozen.r3b"))?;
            if report["baseline_verified"] != true
                || report["data_audit_status"] != "CHECKED_BOUNDARIES_PASS"
                || report["parent"]["physical_hash"] != file_hash(&f.start)?
                || report["original_manifest"]["train"]["sha256"] != f.parent_train_hash
            {
                return Err(Error::Invalid("H2 verified baseline required".into()));
            }
            data::copy_curriculum(&f.parent_corpus, &output, seed)?;
            let (manifest, train, dev) = data::load_legacy(&output)?;
            let seal_descriptor: data::Split =
                replica_v3::binary::from_value(read_metadata(&output.join("seal-manifest.r3b"))?)?;
            let seal = data::load_split_legacy(&output, &seal_descriptor)?;
            let l = checkpoint::load(&f.start, Device::Cpu, false)?;
            let checked = verify_copy_curriculum(&train, &dev, &seal, &l)?;
            let anchors = scan_controlled(&train[..2048], &l, 2048, Some(&mut budget))?;
            if anchors["status"] != "CHECKED_BOUNDARIES_PASS" {
                return Err(Error::Invalid("unverified QA anchors".into()));
            }
            save(
                &output.join("prepared.r3b"),
                &record!({"stage":"H3","status":"PRETRAIN_STRUCTURE_VERIFIED","baseline_summary_hash":file_hash(&baseline.join("summary.r3b"))?,
                "manifest_hash":file_hash(&output.join("manifest.r3b"))?,"train_hash":manifest.train.sha256,"dev_hash":manifest.validation.sha256,"seal":seal_descriptor,
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
                    partial["planned_case_count"] = record!(planned);
                    partial["not_run_count"] =
                        record!(planned.map(|n| n.saturating_sub(budget.attempted_case_count)));
                    partial["final_evaluation_complete"] = record!(false);
                    partial["comparison_eligible"] = record!(false);
                    partial["candidate_eligible"] = record!(false);
                    partial["worker_calls_are_separate"] = record!(true);
                    save(&output.join("interrupted.r3b"), &partial)?;
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
            let (m, _, v) = data::load_legacy(&corpus)?;
            let (p, _, _) = data::load_legacy(&parent_corpus)?;
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
            let registry = record!({"GENERAL_QA_PARENT":registry(&parent)?,"U2_POLICY_START":registry(&start)?,"U2_AFTER_20":registry(&probe)?,"U2_AFTER_250":registry(&failed)?,
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
                    || row["evidence"] != replica_v3::binary::to_value(&e.request.evidence)?
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
            replica_v3::binary::print_record(&record!({"frozen":output,"hash":file_hash(&output)?,"watch":f.watch.len(),"failures":f.failures.len(),"parent":f.registry["parent_observed_log_summary"],"failed":f.registry["failed_observed_log_summary"],"optimizer_updates":0}))?;
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
        Err(e) => record!({"byte_mapping_error":e.to_string()}),
        Ok(bytes) => {
            let utf8 = std::str::from_utf8(&bytes);
            record!({"length":bytes.len(),"sha256":neural::hash(&bytes),"hex":bytes.iter().take(2048).map(|b|format!("{b:02x}")).collect::<String>(),"hex_truncated":bytes.len()>2048,
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
    record!({"entity":e.map(|e|a.is_some_and(|a|a.0==e.0)),"context":e.map(|e|a.is_some_and(|a|a.1==e.1)),"value":e.map(|e|a.is_some_and(|a|a.2==e.2)),
        "citation_exact":ids.as_ref().is_some_and(|a|Some(a)==expected_ids.as_ref()),"citation_in_provided":ids.as_ref().map(|a|a.iter().all(|id|provided.contains(id))),"citation_nonempty":ids.as_ref().is_some_and(|a|!a.is_empty())})
}
pub(super) fn strict_answer_match(actual: Option<&str>, expected: &str, eos: bool, error: bool) -> bool {
    actual.is_some_and(|text| !text.is_empty() && text == expected) && eos && !error
}
pub(super) fn evaluate_one(
    loaded: &Loaded,
    e: &Episode,
    request: &ModelRequest,
    control: &mut RunControl,
) -> Value {
    evaluate_one_policy(loaded, e, request, control, true)
}
pub(super) fn evaluate_one_policy(
    loaded: &Loaded,
    e: &Episode,
    request: &ModelRequest,
    control: &mut RunControl,
    automatic_teacher: bool,
) -> Value {
    match observe_generation(loaded, e, request, control, automatic_teacher) {
        ObservedCall::NotInvoked(row) | ObservedCall::Returned(row) => row,
    }
}
// Prepared without a returned observation is UNKNOWN after a crash. Only this
// synchronous boundary can prove NotInvoked; token count is never entry evidence.
pub(super) enum ObservedCall<T> { NotInvoked(T), Returned(T) }
pub(super) fn observe_generation(
    loaded: &Loaded,
    e: &Episode,
    request: &ModelRequest,
    control: &mut RunControl,
    automatic_teacher: bool,
) -> ObservedCall<Value> {
    observe_generation_mode(loaded,e,request,control,automatic_teacher,false)
}
pub(super) fn observe_generation_profiled(loaded:&Loaded,e:&Episode,request:&ModelRequest,control:&mut RunControl)->ObservedCall<Value> {
    observe_generation_mode(loaded,e,request,control,false,true)
}
fn observe_generation_mode(loaded:&Loaded,e:&Episode,request:&ModelRequest,control:&mut RunControl,automatic_teacher:bool,measured:bool)->ObservedCall<Value> {
    let mut entered = false;
    let mut row = record!({"id":e.id,"scene":scene(e),"category":e.category,"family":e.family,"question":e.request.input,"generated_question":request.input,
        "evidence":e.request.evidence,"generated_evidence":request.evidence,"expected":e.answer,"exact_match":false,"actual":null,"error":null,"generation_started":false,"generation_completed":false,"interruption":null});
    row["row_version"] = record!(2);
    row["command_stop"] = Value::Null;
    row["teacher_forced_diagnostic_after_generation"] = record!({"status":"NOT_RUN"});
    let result = (|| -> Result<()> {
        control.check("case_started")?;
        control.attempted_case_count += 1;
        let prompt = loaded.tokenizer.prepare_with_framing(
            request,
            loaded.manifest.framing()?,
            loaded.model.config.context as u32,
            &loaded.model.config.id()?,
        )?;
        control.check("prompt_prepared")?;
        let effective_timeout = control.effective_timeout(request.limits.timeout_ms)?;
        row["effective_timeout_ms"] = record!(effective_timeout);
        row["original_timeout_ms"] = record!(request.limits.timeout_ms);
        let command_capped = effective_timeout < request.limits.timeout_ms;
        row["timeout_cap_source"] = record!(if command_capped { "command" } else { "request" });
        row["generation_started"] = record!(true);
        control.generation_calls += 1;
        let cancel = control.cancel.clone();
        let mut raw = Vec::new();
        entered = true;
        #[cfg(test)]
        if let Some(hook) = &mut control.hook {
            hook("native_generation_entered", &cancel);
        }
        let observe = |id| {
                raw.push(id);
                #[cfg(feature = "test-support")]
                if loaded.model.config.hidden == 32
                    && loaded.model.config.layers == 2
                    && std::env::var("R3_FRESH_KILL_ENTERED").as_deref() == Ok("generation")
                {
                    std::process::exit(86);
                }
                #[cfg(test)]
                if let Some(hook) = &mut control.hook {
                    hook("token_generated", &cancel);
                }
            };
        let result = if measured {
            loaded.model.generate_profiled(&prompt.token_ids,request.limits.max_tokens as usize,effective_timeout,&cancel,&e.id,observe)
        }else{
            loaded.model.generate_observed(&prompt.token_ids,request.limits.max_tokens as usize,effective_timeout,&cancel,&e.id,observe)
        };
        if let Err(error) = &result {
            if matches!(error, Error::Cancelled) {
                control.observe(StopReason::Cancelled);
            }
            // Only this native error and the cap chosen before entry qualify.
            // Teacher scheduling cannot change the meaning of a generation error.
            if matches!(error, Error::Model(message) if message == "native generation timeout") {
                control.observe(if command_capped {
                    StopReason::TimeBudget
                } else {
                    StopReason::IntegrityFail
                });
            } else if !automatic_teacher || error.to_string().contains("nonfinite") {
                // Preserve the existing non-timeout diagnostic/quality policy.
                control.observe(StopReason::IntegrityFail);
            }
        }
        let returned = result.is_ok();
        let after_generation = control.check("generation_returned");
        let (text, generated, error) = decode_generated(&loaded.tokenizer, result);
        let bytes_ids: Vec<_> = raw
            .iter()
            .copied()
            .take_while(|&id| id >= neural::SPECIALS as u32)
            .collect();
        row["raw_tokens"] = record!(raw);
        row["raw_bytes"] = bytes_receipt(&loaded.tokenizer, &bytes_ids);
        row["eos_index"] = record!(raw.iter().position(|&id| id == EOS));
        row["provided"] = record!(prompt.provided);
        row["excluded"] = record!(prompt.excluded);
        row["request_digest"] = record!(digest(request)?);
        row["prompt_digest"] = record!(digest(&prompt.token_ids)?);
        row["native_prompt_digest"] = record!(prompt.token_digest);
        row["framing"] = record!(loaded.manifest.framing()?.id());
        row["prompt_length"] = record!(prompt.token_ids.len());
        row["exact_match"] = record!(strict_answer_match(
            text.as_deref(),
            &e.answer,
            generated.as_ref().is_some_and(|g| g.finish == "stop"),
            error.is_some()
        ));
        row["components"] = components(text.as_deref(), &e.answer, &prompt.provided);
        row["actual"] = record!(text);
        row["generation"] = record!(generated);
        row["error"] = record!(error);
        row["generation_error"] = record!(if returned { None } else { error.clone() });
        row["decode_error"] = record!(if returned { error.clone() } else { None });
        row["error_class"] = record!(error.as_ref().map(|e| if e.contains("UTF-8") {
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
            .map_or_else(|| row["error_class"].clone(), |g| record!(g.finish));
        row["raw_generated_count"] = record!(raw.len());
        row["generation_completed"] = record!(returned);
        row["eos"] = record!(
            raw.last() == Some(&EOS) && generated.as_ref().is_some_and(|g| g.finish == "stop")
        );
        control.completed_generation_count += usize::from(returned);
        row["whitespace_only"] = record!(
            text.as_ref()
                .is_some_and(|s| !s.is_empty() && s.trim().is_empty())
        );
        // Keep the actual generation receipt before propagating a command stop.
        after_generation?;
        if !automatic_teacher {
            return Ok(());
        }
        control.check("before_teacher")?;
        // Gold enters only after free generation has completed, including failures.
        row["teacher_forced_diagnostic_after_generation"] =
            match teacher(loaded, e, &prompt.token_ids, &raw, control) {
                Ok(t) => t,
                Err(e) => {
                    if e.to_string().contains("nonfinite") {
                        control.classify_error(&e);
                    }
                    record!({"error":e.to_string()})
                }
            };
        Ok(())
    })();
    if let Err(error) = result {
        if control.stop.is_none() {
            row["error"] = record!(error.to_string());
            row["error_class"] = record!("preparation_or_receipt");
        } else {
            row["diagnostic_stop_error"] = record!(error.to_string());
        }
    }
    if let Some(stop) = control.stop {
        row["command_stop"] = record!(stop);
        row["interruption"] = record!(stop);
        if row["teacher_forced_diagnostic_after_generation"]["status"] == "NOT_RUN" {
            row["teacher_forced_diagnostic_after_generation"] =
                record!({"status":format!("NOT_RUN_{}",stop.name())});
        }
        control.interrupted_case_id = Some(e.id.clone());
    }
    if entered {
        ObservedCall::Returned(row)
    } else {
        ObservedCall::NotInvoked(row)
    }
}
pub(super) fn fresh_teacher(
    l: &Loaded,
    e: &Episode,
    control: &mut RunControl,
) -> ObservedCall<Result<Value>> {
    fresh_teacher_with_foil(l, e, None, control)
}
pub(super) fn fresh_teacher_with_foil(
    l: &Loaded,
    e: &Episode,
    foil: Option<&str>,
    control: &mut RunControl,
) -> ObservedCall<Result<Value>> {
    let mut entered = false;
    let result = (|| {
        let p = l.tokenizer.prepare_with_framing(
            &e.request,
            l.manifest.framing()?,
            l.model.config.context as u32,
            &l.model.config.id()?,
        )?;
        teacher_observation(
            l,
            e,
            &p.token_ids,
            &[],
            control,
            foil,
            e.family.starts_with("binding-learnability-v1/") || e.family.starts_with("foundation-orbit-v1/") || e.family.starts_with("learned-binding-expansion-v1/"),
            true,
            &mut entered,
        )
    })();
    if entered {
        ObservedCall::Returned(result)
    } else {
        ObservedCall::NotInvoked(result)
    }
}
fn teacher(
    l: &Loaded,
    e: &Episode,
    prompt: &[u32],
    raw: &[u32],
    control: &mut RunControl,
) -> Result<Value> {
    teacher_with_foil(l, e, prompt, raw, control, None)
}
fn teacher_with_foil(
    l: &Loaded,
    e: &Episode,
    prompt: &[u32],
    raw: &[u32],
    control: &mut RunControl,
    foil: Option<&str>,
) -> Result<Value> {
    teacher_observation(l, e, prompt, raw, control, foil, false, false, &mut false)
}

// Read-only train probe: mismatch is teacher argmax versus gold, never free output.
fn teacher_probe(
    l: &Loaded,
    e: &Episode,
    prompt: &[u32],
    control: &mut RunControl,
) -> Result<Value> {
    teacher_observation(l, e, prompt, &[], control, None, true, false, &mut false)
}
#[allow(clippy::too_many_arguments)] // Existing diagnostic inputs plus durable returned-result semantics.
fn teacher_observation(
    l: &Loaded,
    e: &Episode,
    prompt: &[u32],
    raw: &[u32],
    control: &mut RunControl,
    foil: Option<&str>,
    probe: bool,
    preserve_returned: bool,
    entered: &mut bool,
) -> Result<Value> {
    control.check("teacher_started")?;
    let mut gold = l.tokenizer.encode(e.answer.as_bytes())?;
    gold.push(EOS);
    let mut sequence = prompt.to_vec();
    sequence.extend(&gold);
    if sequence.len() > l.model.config.context {
        return Err(Error::ContextTooSmall);
    }
    let input = Tensor::new(&sequence[..sequence.len() - 1], &Device::Cpu)?.unsqueeze(0)?;
    control.check("teacher_forward")?;
    control.begin_teacher()?;
    *entered = true;
    let logits = l
        .model
        .forward(&input, None)?
        .narrow(1, prompt.len() - 1, gold.len())?
        .squeeze(0)?;
    #[cfg(feature = "test-support")]
    if l.model.config.hidden == 32
        && l.model.config.layers == 2
        && std::env::var("R3_FRESH_KILL_ENTERED").as_deref() == Ok("teacher")
    {
        std::process::exit(86);
    }
    #[cfg(feature = "test-support")]
    if l.model.config.profile == "TINY_NUMERIC_TEST_ONLY"
        && std::env::var("R3ER_TEST_STOP").as_deref() == Ok("conditional-teacher-error")
    {
        return Err(Error::Model(
            "nonfinite injected after conditional teacher forward".into(),
        ));
    }
    let returned_stop = control.check("teacher_returned");
    if !preserve_returned {
        returned_stop?;
    }
    let lp = candle_nn::ops::log_softmax(&logits, 1)?.to_vec2::<f32>()?;
    if lp.iter().flatten().any(|x| !x.is_finite()) {
        return Err(Error::Model("nonfinite diagnostic logits".into()));
    }
    let predicted = logits.argmax(1)?.to_vec1::<u32>()?;
    let raw = if probe { predicted.as_slice() } else { raw };
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
    let framed = samples_with_framing(
        std::slice::from_ref(e),
        &l.tokenizer,
        l.manifest
            .training
            .as_ref()
            .map_or(512, |s| s.config.seq_len),
        l.manifest.framing()?,
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
        record!({"index":i,"gold_id":gold[i],"actual_id":raw.get(i),"teacher_argmax":predicted[i],"gold_log_probability":lp[i][gold[i] as usize],"gold_minus_rival_logit":lp[i][gold[i] as usize]-lp[i][rival as usize],"prefix":if probe { "gold; teacher-only first argmax mismatch; no free generation" } else { "gold; at first divergence identical to generation prefix" }})
    });
    let foil_difference = foil.map(|foil| -> Result<Value> {
        let mut other=l.tokenizer.encode(foil.as_bytes())?; other.push(EOS);
        let i=gold.iter().zip(&other).position(|(a,b)|a!=b).ok_or_else(||Error::Invalid("foil must differ from gold".into()))?;
        if e.family.starts_with("foundation-orbit-v1/") || e.family.starts_with("learned-binding-expansion-v1/") {
            let raw=logits.get(i)?.to_vec1::<f32>()?;
            let g=f64::from(raw[gold[i] as usize]); let f=f64::from(raw[other[i] as usize]); let delta=g-f;
            if !g.is_finite() || !f.is_finite() { return Err(Error::Model("nonfinite gold/foil logits".into())); }
            let binary_nll=(-delta).max(0.)+(-delta.abs()).exp().ln_1p();
            return Ok(record!({"index":i,"gold":gold[i],"foil":other[i],"gold_logit":g,"foil_logit":f,"margin":delta,"binary_nll":binary_nll,"scope":"gold/foil renormalized diagnostic only; decoding unchanged","prefix":"identical gold/foil token prefix; one full gold teacher forward"}));
        }
        Ok(record!({"index":i,"gold":gold[i],"foil":other[i],"margin":lp[i][gold[i] as usize]-lp[i][other[i] as usize],"prefix":"identical gold/foil token prefix; one full gold teacher forward"}))
    }).transpose()?;
    let complete_stop = control.check("teacher_completed");
    if !preserve_returned {
        complete_stop?;
    }
    let after_value = if (e.family.starts_with("value-citation-bridge-v1/") || e.family.starts_with("foundation-orbit-v1/") || e.family.starts_with("learned-binding-expansion-v1/")) && gold.len()>=2 {
        let prefix=l.tokenizer.encode(format!("{}입니다. [event:",e.answer.chars().next().ok_or_else(||Error::Invalid("empty citation target".into()))?).as_bytes())?;
        if prefix.len()<2 || prefix[0]!=gold[0] {return Err(Error::Invalid("citation first-value tokenizer boundary".into()));}
        Some(record!({"position":1,"eos_id":EOS,"citation_start_id":prefix[1],"eos_log_probability":lp[1][EOS as usize],
            "citation_start_log_probability":lp[1][prefix[1] as usize],"eos_minus_citation_logit":f64::from(lp[1][EOS as usize])-f64::from(lp[1][prefix[1] as usize]),
            "scope":"same existing gold-prefix forward; no extra model call; generation unchanged"}))
    } else {None};
    Ok(
        record!({"after_value":after_value,"target_token_observation":(e.family.starts_with("qa-word-value-v1/") || e.family.starts_with("value-citation-bridge-v1/") || e.family.starts_with("binding-learnability-v1/") || e.family.starts_with("foundation-orbit-v1/") || e.family.starts_with("learned-binding-expansion-v1/")).then(||record!({"gold":gold,"nll":nll,"argmax":predicted,"scope":"gold-prefix teacher; separate from free generation"})),"conditional_foil":foil_difference,"target_tokens_including_eos":gold.len(),"mean_nll":nll.iter().sum::<f64>()/gold.len() as f64,"first_target_nll":nll[0],
        "remaining_mean_nll":nll.iter().skip(1).sum::<f64>()/(gold.len()-1).max(1) as f64,"objective":(nll.iter().sum::<f64>()+(w-1.)*nll[0])/gold.len() as f64,"first_target_weight":w,
        "objective_scope":"per-example first-target-weighted response CE; batch/pair/span auxiliary not measured here",
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
                    != record!(cases.iter().map(|e| &e.request.limits).collect::<Vec<_>>())
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
    let ledger = record!({"header":true,"fixture_hash":file_hash(fixture)?,"binary_hash":file_hash(&std::env::current_exe()?)?,"checkpoint_physical_hash":file_hash(path)?,"model_content_hash":model_content_hash,"tokenizer_semantic_hash":l.tokenizer.semantic_id(),"prompt_format":neural::PROMPT_FORMAT,"decoding":cases.iter().map(|e|&e.request.limits).collect::<Vec<_>>(),"metric":"strict-full-answer-eos-v1","panel":panel,"final_heldout":false});
    let mut ledger = ledger;
    ledger
        .as_object_mut()
        .unwrap()
        .extend(binding.as_object().unwrap().clone());
    ledger["source_commit"] = record!(source_commit()?);
    ledger["working_source_manifest_hash"] = record!(source_id);
    ledger["model_tensor_content_digest"] = record!(l.model.weights_content_id()?);
    ledger["reference_raw_hash"] = record!(reference.map(file_hash).transpose()?);
    replica_v3::binary::write_value_record(&mut out, &ledger)?;
    let mut rows = Vec::new();
    for e in &cases {
        if control.check("panel_next_case").is_err() {
            break;
        }
        let row = evaluate_one(&l, e, &e.request, control);
        replica_v3::binary::write_value_record(&mut out, &row)?;
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
                    reference_differences.push(record!({"id":row["id"],"field":key}));
                }
            }
        }
    }
    if !reference_differences.is_empty() && control.stop.is_none() {
        control.observe(StopReason::IntegrityFail);
    }
    summary["reference_differences"] = record!(reference_differences);
    summary["reference_parity"] =
        record!(reference_rows.as_ref().map(|_| if control.stop.is_some() {
            "PARTIAL_OR_FAILED"
        } else {
            "PASS"
        }));
    if differences > 0 {
        control.observe(StopReason::IntegrityFail);
    }
    summary["old_output_differences"] = record!(old.map(|_| differences));
    summary["aba_equal"] = record!(aba_equal);
    let _ = control.check("before_evaluation_record");
    add_partial_counts(&mut summary, &rows, cases.len(), control);
    replica_v3::binary::write_value_record(&mut out, &summary)?;
    out.sync_all()?;
    let _ = control.check("evaluation_recorded");
    if command_terminal {
        control.terminal = true;
    }
    let mut terminal = record!({"terminal":true,"control":control.receipt()});
    terminal["command_terminal"] = record!(command_terminal);
    add_partial_counts(&mut terminal, &rows, cases.len(), control);
    replica_v3::binary::write_value_record(&mut out, &terminal)?;
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
    value["planned_case_count"] = record!(planned);
    value["attempted_case_count"] = record!(rows.len());
    value["completed_generation_count"] = record!(
        rows.iter()
            .filter(|r| r["generation_completed"] == true)
            .count()
    );
    value["not_run_count"] = record!(planned.saturating_sub(rows.len()));
    value["interrupted_case_id"] = rows
        .iter()
        .find(|r| !r["interruption"].is_null())
        .map_or(Value::Null, |r| r["id"].clone());
    value["terminal_reason"] = control.receipt()["terminal_reason"].clone();
    value["final_evaluation_complete"] = record!(complete);
    value["comparison_eligible"] = record!(complete);
    value["candidate_eligible"] = record!(false); // This receipt alone never selects a model.
    value["score_scope"] = record!(if complete {
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
            let (manifest, _, validation) = load_frozen_corpus(f)?;
            (validation, Some(manifest.validation.sha256))
        }
        _ => return Err(Error::Invalid("unknown recovery replay panel".into())),
    };
    let binding = record!({
        "source_validation_hash":f.validation_hash,
        "frozen_expected_validation_hash":f.validation_hash,
        "actual_split_hash":actual_split_hash,
        "evaluated_cases_hash":digest(&cases)?,
        "evaluated_cases_encoding":"sha256(replica_v3::binary::to_vec(ordered Vec<Episode>)); complete fields, no whitespace",
        "ordered_ids_hash":digest(&cases.iter().map(|e|&e.id).collect::<Vec<_>>())?,
        "planned_case_count":cases.len(),
        "input_source":if panel=="all" {"validated_current_snapshot"} else {"frozen_panel"}
    });
    Ok((cases, binding))
}

fn load_frozen_corpus(f: &Frozen) -> Result<(data::CorpusManifest, Vec<Episode>, Vec<Episode>)> {
    let (m, train, validation) = data::load_legacy(&f.corpus)?;
    if m.validation.sha256 != f.validation_hash || m.train.sha256 != f.train_hash {
        return Err(Error::Corrupt(
            "FROZEN_INPUT_MISMATCH: frozen/current validation binding mismatch or train".into(),
        ));
    }
    Ok((m, train, validation))
}
fn verified_ordinary(
    baseline: &Path,
    expected: Option<&Value>,
) -> Result<(Frozen, Vec<Episode>, Vec<Episode>, String)> {
    let bytes = neural::read_bounded(&baseline.join("frozen.r3b"), 16 * 1024 * 1024)?;
    let hash = neural::hash(&bytes);
    if expected.is_some_and(|h| h != &hash) {
        return Err(Error::Corrupt(
            "FROZEN_INPUT_MISMATCH: A0/frozen digest".into(),
        ));
    }
    let f: Frozen = replica_v3::binary::from_slice(&bytes)?;
    let (_, train, cases) = load_frozen_corpus(&f)?;
    let qa = cases
        .iter()
        .filter(|e| !e.family.starts_with("copy/"))
        .count();
    let unique: BTreeSet<_> = cases.iter().map(|e| &e.id).collect();
    if f.version != 1
        || f.watch.len() != 32
        || cases.len() != 400
        || qa != 336
        || unique.len() != 400
        || f.watch.iter().map(|e| &e.id).collect::<BTreeSet<_>>().len() != 32
        || f.watch.iter().any(|e| {
            cases
                .iter()
                .find(|c| c.id == e.id)
                .is_none_or(|c| digest(c).ok() != digest(e).ok())
        })
    {
        return Err(Error::Corrupt(
            "FROZEN_INPUT_MISMATCH: ordinary336/aux64/watch32 membership".into(),
        ));
    }
    Ok((f, train, cases, hash))
}
struct VerifiedProgressInputs {
    a0: Value,
    frozen: Frozen,
    manifest: data::CorpusManifest,
    train: Vec<Episode>,
    dev: Vec<Episode>,
    ordinary: Vec<Episode>,
    cross: Vec<Episode>,
    hashes: Value,
}
fn load_verified_inputs(a0: &Path, policy: Option<&Value>) -> Result<VerifiedProgressInputs> {
    let bytes = neural::read_bounded(&a0.join("summary.r3b"), 16 * 1024 * 1024)?;
    let a: Value = replica_v3::binary::from_slice(&bytes)?;
    if policy.is_some_and(|p| p["a0_hash"] != neural::hash(&bytes)) {
        return Err(Error::Corrupt("FROZEN_INPUT_MISMATCH: policy/A0".into()));
    }
    let (f, _, ordinary, frozen_hash) =
        verified_ordinary(&progress_path(&a, "baseline")?, Some(&a["baseline_hash"]))?;
    let p = policy.unwrap_or(&a);
    let (manifest, train, dev) = data::load_legacy(&progress_path(p, "corpus")?)?;
    let cross_bytes = neural::read_bounded(&a0.join("cross-development.r3b"), 16 * 1024 * 1024)?;
    let cross_hash = neural::hash(&cross_bytes);
    let cross: Vec<Episode> = replica_v3::binary::from_slice(&cross_bytes)?;
    if manifest.train.sha256 != p["train_hash"]
        || manifest.validation.sha256 != p["dev_hash"]
        || a["cross"]["file_sha256"] != cross_hash
        || policy.is_some_and(|p| p["cross_hash"] != cross_hash)
        || dev.len() != 256
        || cross.len() != 512
        || dev.iter().map(|e| &e.id).collect::<BTreeSet<_>>().len() != 256
        || cross.iter().map(|e| &e.id).collect::<BTreeSet<_>>().len() != 512
    {
        return Err(Error::Corrupt(
            "FROZEN_INPUT_MISMATCH: corpus/CROSS identity".into(),
        ));
    }
    if p["node"] == "A2" {
        let receipt = progress_path(p, "parent_receipt")?;
        let parent_policy = receipt
            .parent()
            .and_then(Path::parent)
            .ok_or_else(|| Error::Corrupt("parent policy path".into()))?
            .join("policy.r3b");
        let r = read_metadata(&receipt)?;
        let parent = read_metadata(&parent_policy)?;
        let prior = parent_policy
            .parent()
            .and_then(Path::parent)
            .ok_or_else(|| Error::Corrupt("prior experiment".into()))?;
        if parent["node"] != "A1"
            || r["policy_sha256"] != file_hash(&parent_policy)?
            || p["parent_receipt_sha256"] != file_hash(&receipt)?
            || p["parent_comparison_hash"] != file_hash(&prior.join("comparison.r3b"))?
        {
            return Err(Error::Corrupt(
                "FROZEN_INPUT_MISMATCH: A2 parent/policy/comparison".into(),
            ));
        }
        load_verified_inputs(&progress_path(&parent, "a0")?, Some(&parent))?;
    }
    let parent_binding = if let Some(p) = policy {
        let native_hash = file_hash(&progress_path(p, "parent")?)?;
        let receipt_hash = file_hash(&progress_path(p, "parent_receipt")?)?;
        if p["parent_sha256"] != native_hash || p["parent_receipt_sha256"] != receipt_hash {
            return Err(Error::Corrupt(
                "FROZEN_INPUT_MISMATCH: parent native/receipt".into(),
            ));
        }
        record!({"native":native_hash,"receipt":receipt_hash})
    } else {
        Value::Null
    };
    let hashes = record!({"a0":neural::hash(&bytes),"frozen":frozen_hash,"ordinary":f.validation_hash,"parent":parent_binding,
        "train":manifest.train.sha256,"dev":manifest.validation.sha256,"cross":cross_hash,
        "ordinary_cases":digest(&ordinary)?,"watch_cases":digest(&f.watch)?,"dev_cases":digest(&dev)?,"cross_cases":digest(&cross)?});
    Ok(VerifiedProgressInputs {
        a0: a,
        frozen: f,
        manifest,
        train,
        dev,
        ordinary,
        cross,
        hashes,
    })
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
                causal_records.push(record!({"id":e.id,"event_id":r.event_id,"classification":state,"parsed_entity":entity}));
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
            invalid.push(record!({"id":e.id,"status":semantic,"reason":reason}));
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
                collisions.push(record!({"a":old_id,"b":e.id,"a_answer":old_answer,"b_answer":e.answer,"prompt":key}));
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
    let mut report = record!({"scanned":episodes.len().min(limit),"total":episodes.len(),"independent_full_qa_checked":checked,"auxiliary_semantics_not_checked":auxiliary,"prompt_target_contradictions":collisions,"data_ambiguities":invalid,"train_generation_prefix_mismatches":prefix_mismatch,"category_counts":categories,"first_target_counts":starts,"target_length_histogram":lengths,"supervised_tokens_including_eos":target_count,"eos_targets":episodes.len().min(limit),"record_position":positions,"citation_digit_lengths":digits,"no_evidence":no_evidence,"unique_base_ids":scenes.len(),"unique_questions":questions.len(),"unique_values":values.len(),"unique_evidence_id_orders":orders.len(),"unique_token_prompts":prompts.len(),"max_prompt_length":max_prompt,"prompts_over_window256":over_window});
    report["unique_question_forms_ascii_digit_runs_collapsed"] = record!(question_forms.len());
    report["unique_evidence_value_multisets_including_empty"] = record!(value_combinations.len());
    report["entity_digit_lengths_per_structured_evidence"] = record!(entity_digit_lengths);
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
        report[name] = record!(n);
    }
    report["ordinary_in_scope"] = record!(checked);
    report["semantic_findings"] = report["data_ambiguities"].clone();
    report["causal_record_classifications"] = record!(causal_records);
    report["status"] = record!(audit_status(&[&report], &[]));
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
    let (m, train, validation) = data::load_legacy(&f.corpus)?;
    let (pm, parent, pv) = data::load_legacy(&f.parent_corpus)?;
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
    let result = record!({"u2_all":u2,"parent_bounded_first2048":original,"validation400":development,"cross_split_entity_overlap":overlap,"optimizer_updates":0,"status":status,"fixture_hash":file_hash(fixture)?,"actual_train_hash":m.train.sha256,"actual_validation_hash":m.validation.sha256,"frozen_expected_validation_hash":f.validation_hash,"evaluated_validation_cases_hash":digest(&validation)?,"parent_train_hash":pm.train.sha256,"scope":"U2 all ordinary; parent first2048 and validation400; copy/* auxiliary OUT_OF_SCOPE"});
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
    let (manifest, train, validation) = data::load_legacy(corpus)?;
    let (original_manifest, original_train, original_validation) = data::load_legacy(original)?;
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
    let mut report = record!({"stage":"H2","source_commit":source_commit()?,"source_digest":source_id,"binary_hash":file_hash(&std::env::current_exe()?)?,
        "parent":parent,"inference":{"path":inference,"physical_hash":inference_hash,"model_content_hash":weight_hash,"same_content":true},
        "train_manifest":manifest,"original_manifest":original_manifest,"train_ordinary":ordinary(&train),"original_train_ordinary":ordinary(&original_train),
        "validation_ordinary":ordinary(&validation),"validation_auxiliary":validation.len()-ordinary(&validation),"previous_log_sha256":file_hash(previous_log)?,
        "recount":summarize(&historical)?,"next_parent_lr":next_lr,"proposed_constant_lr":next_lr.min(3e-5),"new_small_updates":0,"new_input_tokens":0,"new_target_tokens":0,"goal1_ready":false});
    save(&output.join("identity-recount.r3b"), &report)?;
    let training_audit = scan_controlled(&train, &loaded, train.len(), Some(control))?;
    save(&output.join("binding-train-audit.r3b"), &training_audit)?;
    let original_audit = scan_controlled(
        &original_train,
        &loaded,
        original_train.len(),
        Some(control),
    )?;
    save(&output.join("original-train-audit.r3b"), &original_audit)?;
    let validation_audit = scan_controlled(&validation, &loaded, validation.len(), Some(control))?;
    save(&output.join("validation-audit.r3b"), &validation_audit)?;
    let status = audit_status(&[&training_audit, &original_audit, &validation_audit], &[]);
    report["data_audit_status"] = record!(status);
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
        registry: record!({"V1000":parent,"source_id":source_id,"binary_hash":file_hash(&std::env::current_exe()?)?}),
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
    let fixture = output.join("frozen.r3b");
    save(&fixture, &frozen)?;
    // Only the frozen ordinary32 are regenerated; the known400 denominator is re-counted above.
    replay(
        &fixture,
        inference,
        &output.join("watch32.r3rows"),
        "watch",
        Some(source_id),
        control,
        false,
        None,
    )?;
    let (_, actual) = rows(&output.join("watch32.r3rows"))?;
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
                differences.push(record!({"id":row["id"],"field":key}));
            }
        }
    }
    report["watch"] = summarize(&actual)?;
    report["watch_differences"] = record!(differences);
    report["generation_calls"] = record!(control.generation_calls);
    report["artifacts_unchanged"] = record!(
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
    report["baseline_verified"] = record!(sealed.is_ok());
    report["h2_split_materialization"] = record!("NEXT_IF_BASELINE_VERIFIED");
    save(&output.join("summary.r3b"), &report)?;
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
                || replica_v3::binary::to_value(records[0])? != replica_v3::binary::to_value(records[3])?
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
        record!({"train":4096,"anchors":2048,"focus":2048,"dev":256,"seal":256,"strata":strata,"identifier_sets":identifiers.iter().map(BTreeSet::len).collect::<Vec<_>>(),
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
    score["entity_correct"] = record!(entity);
    score["event_id_correct"] = record!(event);
    score["context_correct"] = record!(context);
    score["value_correct"] = record!(value_correct);
    score["first_difference_fields"] = record!(first_fields);
    score["digit_accuracy_counts"] = record!([digit_correct, digit_total]);
    score["whole_base_correct_total"] = record!([
        groups.values().filter(|g| g[0] == 4 && g[1] == 4).count(),
        groups.len()
    ]);
    score["strata"] = record!(strata);
    score["generation_error_cases"] = record!(errors);
    score["skill_pass"] = record!(
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
    let mut value = record!({"dev":skill_score(&dev_rows)?,"watch":summarize(&watch_rows)?,"dev_rows":dev_rows,"watch_rows":watch_rows,
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
    let h = read_metadata(path)?;
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
        && previous["control"]["observed_conditions"] == record!(["QUALITY_GUARD"])
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
    let base_report = read_metadata(&baseline.join("summary.r3b"))?;
    let frozen = load(&baseline.join("frozen.r3b"))?;
    let prepared = read_metadata(&corpus.join("prepared.r3b"))?;
    let (manifest, episodes, dev) = data::load_legacy(corpus)?;
    if base_report["baseline_verified"] != true
        || prepared["status"] != "PRETRAIN_STRUCTURE_VERIFIED"
        || prepared["baseline_summary_hash"] != file_hash(&baseline.join("summary.r3b"))?
        || prepared["manifest_hash"] != file_hash(&corpus.join("manifest.r3b"))?
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
    checkpoint::ResumeBinding::require_default(
        l.manifest
            .training
            .as_ref()
            .ok_or_else(|| Error::Invalid("resume state absent".into()))?,
        &l.tokenizer,
    )?;
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
            previous_root.join("policy.r3b")
        } else {
            output.join("policy.r3b")
        };
        policy = read_metadata(&policy_path)?;
        previous = read_metadata(
            &resume
                .parent()
                .ok_or_else(|| Error::Invalid("resume segment".into()))?
                .join("result.r3b"),
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
            || policy["prepared_hash"] != file_hash(&corpus.join("prepared.r3b"))?
            || policy["baseline_hash"] != file_hash(&baseline.join("summary.r3b"))?
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
                let receipt = read_metadata(&path.join("result.r3b"))?;
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
                replica_v3::binary::from_value(previous["baseline_error_ids"].clone())?;
            acknowledged.extend(skill_error_ids(&previous["last_evaluation"]));
            policy["renewal"] = record!({"authorization":"explicit renewed user request after reported quality stop; one new attempt, not automatic resume",
                "original_checkpoint":resume,"original_checkpoint_sha256":file_hash(resume)?,"original_result_sha256":file_hash(&resume.parent().unwrap().join("result.r3b"))?,
                "original_policy_sha256":file_hash(&policy_path)?,"original_reason":previous["reason"],"original_source_id":policy["source_id"],
                "original_binary_hash":policy["binary_hash"],"starting_new_updates":previous["new_updates"],
                "acknowledged_error_ids":acknowledged,"guard":"same watch baseline/streak; additional new UTF-8/control/empty still stop; acceptance requires zero errors"});
            if finish_copy_budget {
                policy["finish_copy_budget"] = record!(true);
                policy["renewal"]["guard"] = record!(
                    "explicitly approved completion to1024 total updates: intermediate UTF-8 count must not increase at two consecutive evaluations; control/empty and original QA/resource/cancel guards retained; unchanged final acceptance"
                );
                previous["extension_allowed"] = record!(true);
            }
            policy["source_id"] = record!(source);
            policy["binary_hash"] = record!(binary_hash);
            policy["harness_hash"] = record!(file_hash(harness)?);
            previous["baseline_error_ids"] = record!(acknowledged);
            previous["previous_dev_correct"] =
                previous["last_evaluation"]["dev"]["exact_matches"].clone();
            std::fs::create_dir(output)?;
            save(&output.join("policy.r3b"), &policy)?;
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
        policy = record!({"stage":"H3","source_id":source,"binary_hash":binary_hash,"harness_hash":file_hash(harness)?,"baseline_hash":file_hash(&baseline.join("summary.r3b"))?,
            "prepared_hash":file_hash(&corpus.join("prepared.r3b"))?,"train_hash":manifest.train.sha256,"dev_hash":manifest.validation.sha256,"parent":base_report["parent"],
            "tape":tape,"tape_hash":digest(&tape)?,"config":state.config,"constant_lr":rate,"lr_policy":"explicit constant; native config schedule is not used by this stage",
            "default_updates":512,"maximum_updates":1024,"max_input_tokens":6_000_000,"max_target_tokens":1_500_000,"max_stage_seconds":3600,"command_seconds":900,"cleanup_seconds":120,
            "sampler":"4 distinct anchor bases +4 distinct focus bases; each frozen pool without replacement per512 updates; inherited RNG"});
        std::fs::create_dir(output)?;
        save(&output.join("policy.r3b"), &policy)?;
        previous = Value::Null;
    }
    if replica_v3::binary::to_value(&state.config)? != policy["config"]
        || policy["tape_hash"] != digest(&policy["tape"])?
    {
        return Err(Error::Corrupt("skill config/tape".into()));
    }
    let tape: Vec<(Vec<usize>, u64)> = replica_v3::binary::from_value(policy["tape"].clone())?;
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
        .open(segment.join("trace.r3rows"))?;
    let mut last = previous["last_evaluation"].clone();
    let mut base_errors: BTreeSet<String> = if previous.is_null() {
        BTreeSet::new()
    } else {
        replica_v3::binary::from_value(previous["baseline_error_ids"].clone())?
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
                evaluation["new_updates"] = record!(n);
                evaluation["model_content_hash"] = record!(l.model.weight_hash()?);
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
                evaluation["new_error_ids"] = record!(new_errors);
                evaluation["utf8_errors"] = record!(previous_utf8);
                evaluation["utf8_growth_streak"] = record!(utf8_growth_streak);
                evaluation["finish_copy_budget_policy"] = record!(finish_copy_budget);
                evaluation["bad_streak"] = record!(streak);
                save(&segment.join(format!("eval-{n:04}.r3b")), &evaluation)?;
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
                    let (_, _, ordinary) = data::load_legacy(&frozen.corpus)?;
                    let rows = evaluate_panel(&l, &ordinary, control);
                    full_qa = summarize(&rows)?;
                    save(
                        &segment.join("original400.r3b"),
                        &record!({"score":full_qa,"rows":rows}),
                    )?;
                    control.check("skill_original_qa_recorded")?;
                    if full_qa["qa"][0].as_u64().unwrap_or(0)
                        < base_report["recount"]["qa"][0].as_u64().unwrap_or(u64::MAX)
                    {
                        control.observe(StopReason::QualityGuard);
                        return control.stop_result();
                    }
                    save(
                        &output.join("seal-attempt.r3b"),
                        &record!({"model_content_hash":l.model.weight_hash()?,"dev":last["dev"],"new_updates":n}),
                    )?;
                    let descriptor: data::Split = replica_v3::binary::from_value(prepared["seal"].clone())?;
                    let seal = data::load_split_legacy(corpus, &descriptor)?;
                    let rows = evaluate_panel(&l, &seal, control);
                    seal_score = skill_score(&rows)?;
                    save(
                        &segment.join("seal.r3b"),
                        &record!({"score":seal_score,"rows":rows}),
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
            let row = record!({"new_update":n+1,"cumulative_model_step":state.step,"optimizer_step":state.step,"lr":rate,"lr_policy":"constant","sampler_state":sampler,"indices":indices,
                "ids":indices.iter().map(|i|&episodes[*i].id).collect::<Vec<_>>(),"pools":["anchor","anchor","anchor","anchor","focus","focus","focus","focus"],
                "input_tokens":batch.tokens,"target_tokens":targets,"consumed_input_tokens":state.consumed_tokens-start_input,"consumed_target_tokens":state.target_tokens-start_target,
                "ce":ce,"objective":objective,"first_target_weight":c.first_target_weight,"gradient_norm":norm,"update_norm":delta,"rss_kib":control.last_rss_kib,"elapsed_seconds":elapsed_before+control.start.elapsed().as_secs_f64()});
            replica_v3::binary::write_value_record(&mut log, &row)?;
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
        result["comparison_eligible"] = record!(false);
    }
    let details = record!({"stage":"H3","source_id":source,"policy_sha256":file_hash(&output.join("policy.r3b"))?,"new_updates":state.step-start_step,"cumulative_model_step":state.step,"sampler_state":state.sampler_state,
        "additional_input_tokens":state.consumed_tokens-start_input,"additional_target_tokens":state.target_tokens-start_target,"stage_elapsed_seconds":elapsed,
        "baseline_error_ids":base_errors,"baseline_watch":base_watch,"bad_streak":streak,"previous_dev_correct":previous_dev,"extension_allowed":extension,"last_evaluation":last,
        "previous_utf8_errors":previous_utf8,"utf8_growth_streak":utf8_growth_streak,"finish_copy_budget_policy":finish_copy_budget,
        "model_content_hash":l.model.weight_hash()?,"full_qa":full_qa,"seal":seal_score,"skill_quality_pass":quality_pass&&control.stop.is_none()&&!cleanup_overrun,
        "candidate_eligible":quality_pass&&control.stop.is_none()&&!cleanup_overrun,"cleanup_limit_exceeded":cleanup_overrun,"error":outcome.as_ref().err().map(ToString::to_string),"goal1_ready":false,"h4":"NOT_RUN_UNTIL_H3_PASS",
        "resume_allowed":control.reason()==Some("TIME_BUDGET") && elapsed<3600. && !cleanup_overrun && !output.join("seal-attempt.r3b").exists() && result["checkpoint_saved"]==true});
    result
        .as_object_mut()
        .unwrap()
        .extend(details.as_object().unwrap().clone());
    if result["checkpoint_saved"] == true {
        result["checkpoint_file_sha256"] = record!(file_hash(&segment.join("final"))?);
    }
    save(&segment.join("result.r3b"), &result)?;
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
const PROGRESS_CONTRACT: &str = "R3-H3-CONTROLLED-PROGRESS-1.0";

fn progress_lr(policy: &str, j: usize) -> Result<f64> {
    if j == 0 {
        return Err(Error::Invalid("LR clock starts at update one".into()));
    }
    match policy {
        "C" => Ok(3e-5),
        "L" => Ok(3e-5 + (1e-4 - 3e-5) * j.min(64) as f64 / 64.),
        _ => Err(Error::Invalid("unknown controlled LR policy".into())),
    }
}
fn progress_path(v: &Value, key: &str) -> Result<PathBuf> {
    v[key]
        .as_str()
        .map(PathBuf::from)
        .ok_or_else(|| Error::Corrupt(format!("missing path {key}")))
}
fn progress_u64(v: &Value, key: &str) -> Result<u64> {
    v[key]
        .as_u64()
        .ok_or_else(|| Error::Corrupt(format!("missing count {key}")))
}
// Compare a persisted diagnostic snapshot using its actual R3BIN reader. Native
// state/weights/moments remain authoritative and are independently byte/content hashed.
fn progress_snapshot(v: &impl Serialize) -> Result<Value> {
    Ok(replica_v3::binary::from_slice(&replica_v3::binary::to_vec(v)?)?)
}
fn progress_policy_valid(p: &Value) -> bool {
    p["contract"] == PROGRESS_CONTRACT
        && p["entry"] == "EXPERIMENT_FORK"
        && p["parent_sha256"]
            .as_str()
            .is_some_and(|h| h.len() == 64 && h.bytes().all(|c| c.is_ascii_hexdigit()))
        && p["maximum_updates"] == 512
        && p["max_input_tokens"] == 2_000_000
        && p["max_target_tokens"] == 500_000
        && p["h3_max_updates"] == 2048
        && p["h3_seconds"] == 7200
        && p["normal_policy"] == "normal_greedy_v1"
        && p["lr_policy"]
            .as_str()
            .is_some_and(|s| progress_lr(s, 1).is_ok())
}
fn progress_config(initial: &TrainingState) -> TrainConfig {
    let mut c = initial.config.clone();
    c.budget_start_step = initial.step;
    c.max_steps = initial.step + 512;
    c.budget_start_tokens = initial.consumed_tokens;
    c.max_tokens = initial.consumed_tokens + 2_000_000;
    c.validate_every = 128;
    c
}
fn progress_prepare(
    a0: &Path,
    harness: &Path,
    output: &Path,
    control: &mut RunControl,
) -> Result<()> {
    control.check("progress_prepare")?;
    let checked = verified_harness(harness)?;
    let inputs = load_verified_inputs(a0, None)?;
    let a = inputs.a0;
    let parent = progress_path(&a, "parent")?;
    let receipt_path = parent.parent().unwrap().join("result.r3b");
    let receipt = read_metadata(&receipt_path)?;
    let l = checkpoint::load(&parent, Device::Cpu, true)?;
    let state = l
        .manifest
        .training
        .as_ref()
        .ok_or_else(|| Error::Invalid("fork needs native Adam".into()))?;
    let corpus = progress_path(&a, "corpus")?;
    let manifest = inputs.manifest;
    let episodes = inputs.train;
    if a["a0_pass"] != true
        || a["contract"] != PROGRESS_CONTRACT
        || a["parent_file_sha256"] != file_hash(&parent)?
        || a["model_hash"] != l.model.weight_hash()?
        || a["optimizer_hash"] != optimizer_hash(&l.optimizer)?
        || a["state"] != progress_snapshot(state)?
        || receipt["resume_allowed"] != false
        || receipt["new_updates"] != 1024
        || a["train_hash"] != manifest.train.sha256
        || a["dev_hash"] != manifest.validation.sha256
        || a["cross"]["validation"]["status"] != "SERIALIZED_INPUT_VERIFIED"
        || a["cross"]["file_sha256"] != file_hash(&a0.join("cross-development.r3b"))?
        || state.config.microbatch != 8
        || state.config.accumulation != 1
        || state.config.sample_group_size != 1
        || state.config.first_target_weight != 8.
        || state.config.weight_decay != 0.01
    {
        return Err(Error::Corrupt(
            "A0/fork identity or fixed training contract".into(),
        ));
    }
    let tape = skill_tape(&episodes, state.sampler_state)?[..512].to_vec();
    let framed = samples(&episodes, &l.tokenizer, 512)?;
    let denominators: Vec<_> = tape
        .iter()
        .map(|(indices, _)| {
            let input: usize = indices.iter().map(|i| framed[*i].tokens.len() - 1).sum();
            let target: usize = indices
                .iter()
                .map(|i| framed[*i].tokens.len() - framed[*i].response_start)
                .sum();
            (input, target)
        })
        .collect();
    let mut common = record!({"contract":PROGRESS_CONTRACT,"node":"A1","entry":"EXPERIMENT_FORK","run_id":output,
        "a0":a0,"a0_hash":file_hash(&a0.join("summary.r3b"))?,"parent":parent,"parent_sha256":file_hash(&parent)?,
        "parent_receipt":receipt_path,"parent_receipt_sha256":file_hash(&receipt_path)?,"initial_model_hash":a["model_hash"],
        "initial_adam_hash":a["optimizer_hash"],"initial_state":state,"initial_state_native_hash":digest(state)?,"tokenizer_hash":l.tokenizer.semantic_id(),
        "source_commit":source_commit()?,"source_digest":checked["source_digest"],"binary_hash":file_hash(&std::env::current_exe()?)?,
        "harness":harness,"harness_hash":file_hash(harness)?,"backend":neural::cpu_backend(),"compute_threads":1,
        "corpus":corpus,"train_hash":manifest.train.sha256,"dev_hash":manifest.validation.sha256,
        "cross_hash":a["cross"]["file_sha256"],"tape":tape,"tape_hash":digest(&tape)?,"denominators":denominators,
        "denominators_hash":digest(&denominators)?});
    let watch = read_metadata(&a0.join("watch32.r3b"))?;
    let watch_score = skill_score(
        watch["rows"]
            .as_array()
            .ok_or_else(|| Error::Corrupt("A0 watch rows".into()))?,
    )?;
    let limits = record!({"maximum_updates":512,"max_input_tokens":2_000_000,"max_target_tokens":500_000,
        "h3_max_updates":2048,"h3_seconds":7200,"command_seconds":1800,"cleanup_seconds":120,"normal_policy":"normal_greedy_v1",
        "baseline_dev":a["old_dev"]["exact_matches"],"baseline_watch":a["watch"]["exact_matches"],
        "baseline_errors":progress_u64(&a["old_dev"],"generation_error_cases")?+progress_u64(&watch_score,"generation_error_cases")?,
        "anchor_floor":a["ordinary_parent"]["qa"][0].as_u64().ok_or_else(||Error::Corrupt("A0 ordinary baseline".into()))?.max(175),
        "evaluation_updates":[0,128,256,512],"guard":{"dev_drop":26,"watch_drop":4,"error_increase":6,"streak":2,"one_shot_error_fraction":0.2,"error_panel":"dev256+watch32; complete denominator288"},
        "zero_evaluation":"DERIVED_FROM_A0_IDENTICAL_MODEL","new_lr_clock":0,"lr_offset":0,"seal":"NOT_OPENED"});
    common
        .as_object_mut()
        .unwrap()
        .extend(limits.as_object().unwrap().clone());
    std::fs::create_dir(output)?;
    for arm in ["C", "L"] {
        common["arm"] = record!(arm);
        common["lr_policy"] = record!(arm);
        common["lr_function"] = record!(if arm == "C" {
            "3e-5"
        } else {
            "3e-5+(1e-4-3e-5)*min(j,64)/64; j=1..512, cumulative Adam retained"
        });
        let dir = output.join(arm);
        std::fs::create_dir(&dir)?;
        save(&dir.join("policy.r3b"), &common)?;
    }
    control.check("progress_pair_registered")?;
    save(
        &output.join("pair.r3b"),
        &record!({"node":"A1","contract":PROGRESS_CONTRACT,"arms":["C","L"],
        "policy_hashes":[file_hash(&output.join("C/policy.r3b"))?,file_hash(&output.join("L/policy.r3b"))?],
        "a0_elapsed_seconds":a["control"]["elapsed_seconds"],"prior_experiment":null,"maximum_pair_updates":1024}),
    )?;
    Ok(())
}

fn progress_endpoint_safe(r: &Value) -> bool {
    r["reason"] == "SCREENING_BUDGET_REACHED"
        && r["comparison_eligible"] == true
        && r["cleanup_limit_exceeded"] == false
        && r["new_updates"] == 512
        && r["checkpoint_saved"] == true
        && r.get("save_error") == Some(&Value::Null)
        && r["final_evaluation_complete"] == true
        && r["control"]["terminal_reason"] == "COMPLETED"
        && r["control"]["observed_conditions"] == record!([])
        && r["resume_allowed"] == false
}
#[allow(clippy::type_complexity)] // Reuse the existing persisted skill-tape tuple representation.
fn progress_renewal_tapes(
    original: &[Episode],
    sampler: u64,
) -> Result<[Vec<(Vec<usize>, u64)>; 2]> {
    let anchors = skill_tape(original, sampler)?;
    let mut rng = Rng { state: sampler };
    let mut order: Vec<_> = (0..128).collect();
    for i in (1..128).rev() {
        order.swap(i, (rng.next_u64() % (i + 1) as u64) as usize);
    }
    let mut variants = vec![[0, 1, 2, 3]; 128];
    for v in &mut variants {
        for i in (1..4).rev() {
            v.swap(i, (rng.next_u64() % (i + 1) as u64) as usize);
        }
    }
    let mut tapes = [Vec::new(), Vec::new()];
    for round in 0..4 {
        for cycle in 0..4 {
            for group in order.as_chunks::<4>().0 {
                let n = tapes[0].len();
                for (arm, tape) in tapes.iter_mut().enumerate() {
                    let mut indices = anchors[n].0[..4].to_vec();
                    indices.extend(group.iter().map(|base| {
                        2048 + (base + if arm == 0 { 0 } else { cycle * 128 }) * 4
                            + variants[*base][(round + cycle) % 4]
                    }));
                    tape.push((indices, anchors[n].1));
                }
            }
        }
    }
    Ok(tapes)
}
fn progress_renewal(
    root: &Path,
    harness: &Path,
    output: &Path,
    seed: u64,
    control: &mut RunControl,
) -> Result<()> {
    control.check("progress_renewal_prepare")?;
    let checked = verified_harness(harness)?;
    let comparison = read_metadata(&root.join("comparison.r3b"))?;
    let selected = progress_u64(&comparison, "selected_index")? as usize;
    if comparison["node"] != "A1"
        || comparison["contract"] != PROGRESS_CONTRACT
        || comparison["next"] != "A2_CONDITIONAL"
        || comparison["raw_development_gate"] != false
        || selected > 1
        || !progress_endpoint_safe(&comparison["endpoints"][selected])
    {
        return Err(Error::Invalid(
            "A2 requires closed safe improving A1 endpoint, not another sweep".into(),
        ));
    }
    let end = &comparison["endpoints"][selected];
    let segment = progress_path(end, "segment")?;
    let mut p = read_metadata(&segment.parent().unwrap().join("policy.r3b"))?;
    let inputs = load_verified_inputs(&progress_path(&p, "a0")?, Some(&p))?;
    let ledger = progress_ledger(root)?;
    if ledger.0 > 1024 {
        return Err(Error::Invalid("A1 update ledger".into()));
    }
    if end["policy_sha256"] != file_hash(&segment.parent().unwrap().join("policy.r3b"))?
        || end["checkpoint_file_sha256"] != file_hash(&segment.join("final"))?
    {
        return Err(Error::Corrupt("selected endpoint changed".into()));
    }
    let l = checkpoint::load(&segment.join("final"), Device::Cpu, true)?;
    let state = l
        .manifest
        .training
        .as_ref()
        .ok_or_else(|| Error::Corrupt("A2 native Adam".into()))?;
    if end["model_content_hash"] != l.model.weight_hash()?
        || end["adam_hash"] != optimizer_hash(&l.optimizer)?
        || end["cumulative_model_step"] != state.step
        || end["sampler_state"] != state.sampler_state
    {
        return Err(Error::Corrupt("A2 native parent tensor/Adam/clock".into()));
    }
    let mut heldout = inputs.cross;
    heldout.extend(inputs.dev.clone());
    heldout.extend(inputs.ordinary);
    let original_data = (inputs.manifest, inputs.train, inputs.dev);
    control.check("progress_renewal_data_verified")?;
    std::fs::create_dir(output)?;
    let mut generated = data::renewed_copy_curricula(&original_data, &heldout, output, seed)?;
    let tapes = progress_renewal_tapes(&original_data.1, state.sampler_state)?;
    p["node"] = record!("A2");
    p["run_id"] = record!(output);
    p["parent"] = record!(segment.join("final"));
    p["parent_sha256"] = end["checkpoint_file_sha256"].clone();
    p["parent_receipt"] = record!(segment.join("result.r3b"));
    p["parent_receipt_sha256"] = record!(file_hash(&segment.join("result.r3b"))?);
    p["initial_model_hash"] = end["model_content_hash"].clone();
    p["initial_adam_hash"] = end["adam_hash"].clone();
    p["initial_state"] = replica_v3::binary::to_value(state)?;
    p["initial_state_native_hash"] = record!(digest(state)?);
    p["source_commit"] = record!(source_commit()?);
    p["source_digest"] = checked["source_digest"].clone();
    p["binary_hash"] = record!(file_hash(&std::env::current_exe()?)?);
    p["harness"] = record!(harness);
    p["harness_hash"] = record!(file_hash(harness)?);
    p["baseline_dev"] = end["last_evaluation"]["dev"]["exact_matches"].clone();
    p["baseline_watch"] = end["last_evaluation"]["watch"]["exact_matches"].clone();
    p["baseline_errors"] = record!(
        progress_u64(&end["last_evaluation"]["dev"], "generation_error_cases")?
            + progress_u64(&end["last_evaluation"]["watch"], "generation_error_cases")?
    );
    p["lr_offset"] = record!(progress_u64(&p, "lr_offset")? + progress_u64(end, "new_updates")?);
    p["lr_function"] = record!(
        "continue selected A1 policy clock; no restart of ramp; same F/N LR and inherited cumulative Adam"
    );
    p["zero_evaluation"] = record!("DERIVED_FROM_A1_IDENTICAL_SELECTED_MODEL");
    p["zero_evaluation_file"] = record!(segment.join("eval-0512.r3b"));
    p["zero_evaluation_hash"] = record!(file_hash(&segment.join("eval-0512.r3b"))?);
    p["parent_comparison_hash"] = record!(file_hash(&root.join("comparison.r3b"))?);
    p["generator"] = generated["generator"].clone();
    for (index, arm) in ["F", "N"].iter().enumerate() {
        let corpus = output.join(format!("corpus-{arm}"));
        let (manifest, episodes, _) = data::load_legacy(&corpus)?;
        let count = if index == 0 { 512 } else { 2048 };
        generated["arms"][*arm]["validation"] =
            verify_cross_panel(&episodes[2048..], &heldout, &l, count)?;
        let framed = samples(&episodes, &l.tokenizer, 512)?;
        let denominators: Vec<_> = tapes[index]
            .iter()
            .map(|(ids, _)| {
                (
                    ids.iter()
                        .map(|i| framed[*i].tokens.len() - 1)
                        .sum::<usize>(),
                    ids.iter()
                        .map(|i| framed[*i].tokens.len() - framed[*i].response_start)
                        .sum::<usize>(),
                )
            })
            .collect();
        let mut coordinates = Vec::new();
        for (n, (ids, _)) in tapes[index].iter().enumerate() {
            if ids
                .iter()
                .map(|i| scene(&episodes[*i]))
                .collect::<BTreeSet<_>>()
                .len()
                != 8
            {
                return Err(Error::Corrupt("renewal batch repeats base".into()));
            }
            for (slot, i) in ids[4..].iter().enumerate() {
                coordinates.push(record!({"generator_revision":"controlled-renewal-H3-v1","namespace":"train-renewal","seed":seed,"update":n+1,"slot":slot,"base":(i-2048)/4,"view":(i-2048)%4,"id":episodes[*i].id,"sequence":episodes[*i].sequence}));
            }
        }
        p["arm"] = record!(arm);
        p["corpus"] = record!(corpus);
        p["train_hash"] = record!(manifest.train.sha256);
        p["dev_hash"] = record!(manifest.validation.sha256);
        p["tape"] = record!(tapes[index]);
        p["tape_hash"] = record!(digest(&tapes[index])?);
        p["denominators"] = record!(denominators);
        p["denominators_hash"] = record!(digest(&denominators)?);
        let dir = output.join(arm);
        std::fs::create_dir(&dir)?;
        save(&dir.join("coordinates.r3b"), &record!(coordinates))?;
        p["coordinates_hash"] = record!(file_hash(&dir.join("coordinates.r3b"))?);
        save(&dir.join("policy.r3b"), &p)?;
    }
    save(&output.join("generated.r3b"), &generated)?;
    control.check("progress_renewal_registered")?;
    save(
        &output.join("pair.r3b"),
        &record!({"node":"A2","contract":PROGRESS_CONTRACT,"arms":["F","N"],"policy_hashes":[file_hash(&output.join("F/policy.r3b"))?,file_hash(&output.join("N/policy.r3b"))?],
        "prior_experiment":root,"prior_comparison_hash":file_hash(&root.join("comparison.r3b"))?,"maximum_pair_updates":1024,"same_strata_schedule":true,
        "same_anchor_tape":true,"renewal_only":"F512views repeated4; N2048views once, four views on distinct batches; F is balanced first128base quarter of N",
        "learning_before_registration":0,"actual_prior_updates":ledger.0}),
    )?;
    Ok(())
}
/// Counts every closed segment, including failed segments; an unclosed segment blocks reruns.
fn progress_ledger(root: &Path) -> Result<(u64, u64, u64, f64)> {
    let pair = read_metadata(&root.join("pair.r3b"))?;
    let mut total = (0, 0, 0, 0.);
    if let Some(prior) = pair["prior_experiment"].as_str() {
        if pair["node"] != "A2"
            || read_metadata(&Path::new(prior).join("pair.r3b"))?["node"] != "A1"
            || pair["prior_comparison_hash"]
                != file_hash(&Path::new(prior).join("comparison.r3b"))?
        {
            return Err(Error::Corrupt("bounded A1 to A2 ledger lineage".into()));
        }
        total = progress_ledger(Path::new(prior))?;
    } else {
        if pair["node"] != "A1" {
            return Err(Error::Corrupt("missing A1 prior ledger".into()));
        }
        total.3 = pair["a0_elapsed_seconds"]
            .as_f64()
            .filter(|v| v.is_finite() && *v >= 0.)
            .ok_or_else(|| Error::Corrupt("A0 elapsed budget".into()))?;
    }
    for arm in pair["arms"]
        .as_array()
        .ok_or_else(|| Error::Corrupt("pair arms".into()))?
    {
        let directory = root.join(
            arm.as_str()
                .ok_or_else(|| Error::Corrupt("arm name".into()))?,
        );
        for entry in std::fs::read_dir(directory)? {
            let path = entry?.path();
            if !path.is_dir() {
                continue;
            }
            let r = read_metadata(&path.join("result.r3b"))?;
            total.0 += progress_u64(&r, "segment_updates")?;
            total.1 += progress_u64(&r, "segment_input_tokens")?;
            total.2 += progress_u64(&r, "segment_target_tokens")?;
            total.3 += r["segment_elapsed_seconds"]
                .as_f64()
                .filter(|v| v.is_finite() && *v >= 0.)
                .ok_or_else(|| Error::Corrupt("segment elapsed".into()))?;
            if r["reason"] == "CANCELLED"
                || r["reason"] == "INTEGRITY_FAIL"
                || r["checkpoint_saved"] != true
            {
                return Err(Error::Invalid(
                    "cancelled/invalid pair cannot automatically continue".into(),
                ));
            }
        }
    }
    if total.0 > 2048 || total.1 > 8_000_000 || total.2 > 2_000_000 || total.3 >= 7200. {
        return Err(Error::Invalid(
            "controlled H3 total budget exhausted".into(),
        ));
    }
    Ok(total)
}
fn progress_guard(p: &Value, e: &Value, streak: &mut [u64; 3]) -> Result<bool> {
    if e["final_evaluation_complete"] != true
        || e["dev"]["denominator"] != 256
        || e["watch"]["denominator"] != 32
    {
        return Err(Error::Corrupt(
            "guard requires all288 cases including errors".into(),
        ));
    }
    let errors = progress_u64(&e["dev"], "generation_error_cases")?
        + progress_u64(&e["watch"], "generation_error_cases")?;
    Ok(guard_counts(
        [
            progress_u64(p, "baseline_dev")?,
            progress_u64(p, "baseline_watch")?,
            progress_u64(p, "baseline_errors")?,
        ],
        [
            progress_u64(&e["dev"], "exact_matches")?,
            progress_u64(&e["watch"], "exact_matches")?,
            errors,
        ],
        streak,
        288,
    ))
}
fn guard_counts(baseline: [u64; 3], current: [u64; 3], streak: &mut [u64; 3], total: u64) -> bool {
    let bad = [
        baseline[0].saturating_sub(current[0]) >= 26,
        baseline[1].saturating_sub(current[1]) >= 4,
        current[2] >= baseline[2].saturating_add(6),
    ];
    for (s, b) in streak.iter_mut().zip(bad) {
        *s = if b { *s + 1 } else { 0 };
    }
    streak.iter().any(|n| *n >= 2) || current[2].saturating_mul(5) >= total
}
fn progress_time_resume(r: &Value) -> bool {
    r["reason"] == "TIME_BUDGET"
        && r["resume_allowed"] == true
        && r["checkpoint_saved"] == true
        && r["control"]["observed_conditions"] == record!(["TIME_BUDGET"])
        && r.get("save_error") == Some(&Value::Null)
        && r["cleanup_limit_exceeded"] == false
        && r["new_updates"].as_u64().is_some_and(|n| n <= 512)
}
fn verify_resume_evaluation_files(p: &Value, previous: &Value, segments: &[PathBuf]) -> Result<()> {
    let expected = evaluation_identity(p, &previous["last_evaluation"])?;
    let mut found = false;
    for segment in segments {
        let result = read_metadata(&segment.join("result.r3b"))?;
        let receipts = result["panel_receipts"]
            .as_object()
            .ok_or_else(|| Error::Corrupt("AMBIGUOUS_GUARD_STATE: missing raw receipts".into()))?;
        for (name, _) in receipts {
            if Path::new(name).file_name().and_then(|n| n.to_str()) != Some(name) {
                return Err(Error::Corrupt(
                    "INTEGRITY_FAIL: owned panel filename".into(),
                ));
            }
            let path = segment.join(name);
            let (raw, hash) = read_panel(&path)?;
            verify_panel_file(&result, &path, &raw, &hash, false)?;
            if name.starts_with("eval-")
                && raw["new_updates"] == previous["last_evaluation"]["new_updates"]
            {
                if evaluation_identity(p, &raw)? != expected {
                    return Err(Error::Corrupt(
                        "INTEGRITY_FAIL: resume/raw evaluation identity".into(),
                    ));
                }
                found = true;
            }
        }
    }
    if !found {
        return Err(Error::Corrupt(
            "AMBIGUOUS_GUARD_STATE: evaluation raw missing".into(),
        ));
    }
    Ok(())
}

// The same durable evaluation boundary is used by new execution and time resume.
#[derive(Clone, Serialize, Deserialize)]
struct EvaluationDecision {
    key: String,
    evaluation_digest: String,
    panel_digest: String,
    before: [u64; 3],
    after: Option<[u64; 3]>,
    quality_stop: Option<bool>,
    applied: Option<String>,
}
fn evaluation_identity(p: &Value, e: &Value) -> Result<(String, String, String)> {
    let mut raw = e.clone();
    raw.as_object_mut()
        .ok_or_else(|| Error::Corrupt("evaluation object".into()))?
        .remove("decision");
    let content = digest(&raw)?;
    let panel = digest(&record!([e["dev_rows"], e["watch_rows"]]))?;
    let key = digest(&record!([
        p["run_id"],
        digest(p)?,
        e["model_content_hash"],
        e["new_updates"],
        panel,
        content
    ]))?;
    Ok((key, content, panel))
}
fn reconcile_evaluation_decision(
    p: &Value,
    n: usize,
    model: &str,
    last: &mut Value,
    streak: &mut [u64; 3],
    control: &mut RunControl,
) -> Result<bool> {
    if last.is_null() || last["new_updates"] == 0 {
        return Ok(false); // Registered parent observation does not consume a guard streak.
    }
    let step = progress_u64(last, "new_updates")? as usize;
    if last["final_evaluation_complete"] != true
        || step > n
        || (step == n && last["model_content_hash"] != model)
    {
        return Err(Error::Corrupt(
            "INTEGRITY_FAIL: incomplete/step/model evaluation".into(),
        ));
    }
    let mut decision: EvaluationDecision = replica_v3::binary::from_value(last["decision"].clone())
        .map_err(|_| Error::Corrupt("AMBIGUOUS_GUARD_STATE".into()))?;
    let (key, content, panel) = evaluation_identity(p, last)?;
    if (
        decision.key.as_str(),
        decision.evaluation_digest.as_str(),
        decision.panel_digest.as_str(),
    ) != (key.as_str(), content.as_str(), panel.as_str())
    {
        return Err(Error::Corrupt(
            "INTEGRITY_FAIL: evaluation decision digest".into(),
        ));
    }
    let mut after = decision.before;
    let stop = progress_guard(p, last, &mut after)?;
    if let Some(applied) = &decision.applied {
        if applied != &key
            || decision.after != Some(after)
            || decision.quality_stop != Some(stop)
            || *streak != after
        {
            return Err(Error::Corrupt("INTEGRITY_FAIL: applied guard state".into()));
        }
    } else {
        if step != n
            || *streak != decision.before
            || decision.after.is_some()
            || decision.quality_stop.is_some()
        {
            return Err(Error::Corrupt("INTEGRITY_FAIL: pending guard state".into()));
        }
        decision.after = Some(after);
        decision.quality_stop = Some(stop);
        decision.applied = Some(key);
        *streak = after;
        last["decision"] = replica_v3::binary::to_value(decision)?;
    }
    if stop {
        control.observe(StopReason::QualityGuard);
    }
    Ok(stop)
}
#[allow(clippy::too_many_arguments)]
fn progress_evaluation_boundary(
    p: &Value,
    n: usize,
    model: &str,
    evaluation: Option<Value>,
    last: &mut Value,
    streak: &mut [u64; 3],
    segment: &Path,
    control: &mut RunControl,
    checkpoint: impl FnOnce() -> Result<()>,
) -> Result<()> {
    if let Some(e) = evaluation {
        *last = e;
        if last["final_evaluation_complete"] == true {
            let (key, evaluation_digest, panel_digest) = evaluation_identity(p, last)?;
            last["decision"] = replica_v3::binary::to_value(EvaluationDecision {
                key,
                evaluation_digest,
                panel_digest,
                before: *streak,
                after: None,
                quality_stop: None,
                applied: None,
            })?;
        }
        save(&segment.join(format!("eval-{n:04}.r3b")), last)?;
        control.check("progress_eval_recorded")?;
    }
    let pending =
        !last.is_null() && last["new_updates"] != 0 && last["decision"]["applied"].is_null();
    reconcile_evaluation_decision(p, n, model, last, streak, control)?;
    if pending {
        save(
            &segment.join(format!("decision-{n:04}.r3b")),
            &last["decision"],
        )?;
        checkpoint()?;
        let _ = control.check("progress_eval_checkpoint");
    }
    control.stop_result()
}

fn progress_arm(
    root: &Path,
    arm: &str,
    resume: Option<&Path>,
    control: &mut RunControl,
) -> Result<()> {
    control.check("progress_arm_start")?;
    let pair = read_metadata(&root.join("pair.r3b"))?;
    let arms = pair["arms"]
        .as_array()
        .ok_or_else(|| Error::Corrupt("pair arms".into()))?;
    let arm_index = arms
        .iter()
        .position(|a| a == arm)
        .ok_or_else(|| Error::Invalid("unregistered arm".into()))?;
    let output = root.join(arm);
    let policy_path = output.join("policy.r3b");
    let p = read_metadata(&policy_path)?;
    let inputs = load_verified_inputs(&progress_path(&p, "a0")?, Some(&p))?;
    for (i, a) in arms.iter().enumerate() {
        if pair["policy_hashes"][i]
            != file_hash(&root.join(a.as_str().unwrap()).join("policy.r3b"))?
        {
            return Err(Error::Corrupt("registered pair policy changed".into()));
        }
    }
    let checked = verified_harness(&progress_path(&p, "harness")?)?;
    if !progress_policy_valid(&p)
        || p["source_digest"] != checked["source_digest"]
        || p["binary_hash"] != file_hash(&std::env::current_exe()?)?
        || p["backend"] != neural::cpu_backend()
        || p["harness_hash"] != file_hash(&progress_path(&p, "harness")?)?
        || std::env::var("VECLIB_MAXIMUM_THREADS").as_deref() != Ok("1")
        || std::env::var("RAYON_NUM_THREADS").as_deref() != Ok("1")
    {
        return Err(Error::Invalid(
            "frozen experiment/source/binary/thread policy required".into(),
        ));
    }
    let ledger = progress_ledger(root)?;
    control.deadline = control.start + Duration::from_secs_f64((7200. - ledger.3).min(1800.));
    let parent = progress_path(&p, "parent")?;
    let a0 = progress_path(&p, "a0")?;
    if p["a0_hash"] != file_hash(&a0.join("summary.r3b"))?
        || p["cross_hash"] != file_hash(&a0.join("cross-development.r3b"))?
    {
        return Err(Error::Corrupt("frozen development baseline changed".into()));
    }
    if p["node"] == "A2" && p["coordinates_hash"] != file_hash(&output.join("coordinates.r3b"))? {
        return Err(Error::Corrupt("renewal replay coordinates changed".into()));
    }
    let frozen = &inputs.frozen;
    let manifest = &inputs.manifest;
    let episodes = &inputs.train;
    let dev = &inputs.dev;
    let ordinary_cases = &inputs.ordinary;
    let cross_cases = &inputs.cross;
    if p["train_hash"] != manifest.train.sha256 || p["dev_hash"] != manifest.validation.sha256 {
        return Err(Error::Corrupt("frozen experiment corpus changed".into()));
    }
    let mut previous = Value::Null;
    let mut dirs = std::fs::read_dir(&output)?
        .map(|e| e.map(|e| e.path()))
        .collect::<std::io::Result<Vec<_>>>()?;
    dirs.retain(|d| d.is_dir());
    dirs.sort();
    if let Some(path) = resume {
        if dirs.last().is_none_or(|last| last != path) {
            return Err(Error::Invalid(
                "resume must use latest owned segment".into(),
            ));
        }
        previous = read_metadata(&path.join("result.r3b"))?;
        if !progress_time_resume(&previous)
            || previous["policy_sha256"] != pair["policy_hashes"][arm_index]
            || previous["checkpoint_file_sha256"] != file_hash(&path.join("final"))?
            || previous["evaluation_decision_digest"]
                != digest(&previous["last_evaluation"]["decision"])?
        {
            return Err(Error::Invalid("only matching clean time stop may resume; closed/failed parent requires explicit fork".into()));
        }
        verify_resume_evaluation_files(&p, &previous, &dirs)?;
    } else if !dirs.is_empty() {
        return Err(Error::Invalid(
            "arm already started; no budget reset".into(),
        ));
    }
    let checkpoint = resume.map(|r| r.join("final")).unwrap_or(parent.clone());
    let mut l = checkpoint::load(&checkpoint, Device::Cpu, true)?;
    checkpoint::ResumeBinding::require_default(
        l.manifest
            .training
            .as_ref()
            .ok_or_else(|| Error::Invalid("resume state absent".into()))?,
        &l.tokenizer,
    )?;
    let mut state = l
        .manifest
        .training
        .clone()
        .ok_or_else(|| Error::Corrupt("native Adam state required".into()))?;
    let initial = if resume.is_none() {
        state.clone()
    } else {
        checkpoint::load(&parent, Device::Cpu, true)?
            .manifest
            .training
            .ok_or_else(|| Error::Corrupt("native parent state".into()))?
    };
    if p["initial_state_native_hash"] != digest(&initial)? {
        return Err(Error::Corrupt("native parent state identity".into()));
    }
    let start = initial.step;
    let start_input = initial.consumed_tokens;
    let start_target = initial.target_tokens;
    let tape: Vec<(Vec<usize>, u64)> = replica_v3::binary::from_value(p["tape"].clone())?;
    let denominators: Vec<(usize, usize)> = replica_v3::binary::from_value(p["denominators"].clone())?;
    if tape.len() != 512
        || denominators.len() != 512
        || p["tape_hash"] != digest(&tape)?
        || p["denominators_hash"] != digest(&denominators)?
        || l.tokenizer.semantic_id() != p["tokenizer_hash"]
        || state.step < start
        || state.step > start + 512
        || (resume.is_none()
            && (p["initial_model_hash"] != l.model.weight_hash()?
                || p["initial_adam_hash"] != optimizer_hash(&l.optimizer)?
                || p["initial_state"] != progress_snapshot(&state)?))
        || (resume.is_some()
            && (previous["model_content_hash"] != l.model.weight_hash()?
                || previous["adam_hash"] != optimizer_hash(&l.optimizer)?
                || previous["new_updates"] != state.step - start
                || previous["sampler_state"] != state.sampler_state
                || previous["additional_input_tokens"]
                    .as_u64()
                    .and_then(|n| start_input.checked_add(n))
                    != Some(state.consumed_tokens)
                || previous["additional_target_tokens"]
                    .as_u64()
                    .and_then(|n| start_target.checked_add(n))
                    != Some(state.target_tokens)))
        || (state.step > start && state.sampler_state != tape[state.step - start - 1].1)
    {
        return Err(Error::Corrupt(
            "fork/resume tensor, Adam, clock or tape mismatch".into(),
        ));
    }
    let framed = samples(episodes, &l.tokenizer, 512)?;
    let c = progress_config(&initial);
    c.validate(l.model.config.context)?;
    if resume.is_some() && replica_v3::binary::to_value(&state.config)? != replica_v3::binary::to_value(&c)? {
        return Err(Error::Corrupt("resumed fork config changed".into()));
    }
    state.config = c.clone();
    if resume.is_some()
        && (state.corpus_hash != manifest.train.sha256
            || state.validation_hash != manifest.validation.sha256)
    {
        return Err(Error::Corrupt(
            "resumed renewal corpus state changed".into(),
        ));
    }
    if state.corpus_hash != manifest.train.sha256 {
        state.previous_corpora.push(state.corpus_hash.clone());
        state.previous_corpora.sort();
        state.previous_corpora.dedup();
        state.corpus_hash = manifest.train.sha256.clone();
        state.validation_hash = manifest.validation.sha256.clone();
    }
    state.parent_checkpoint_hash = Some(
        p["initial_model_hash"]
            .as_str()
            .ok_or_else(|| Error::Corrupt("parent model identity".into()))?
            .into(),
    );
    let mut adam = Adam {
        moments: std::mem::take(&mut l.optimizer),
    };
    l.manifest.source_id = checked["source_digest"].as_str().unwrap().into();
    let segment_start = (state.step, state.consumed_tokens, state.target_tokens);
    let segment = output.join(format!(
        "segment-{:02}-{:04}",
        dirs.len(),
        state.step - start
    ));
    std::fs::create_dir(&segment)?;
    let mut log = std::fs::OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(segment.join("trace.r3rows"))?;
    let mut streak: [u64; 3] = if previous.is_null() {
        [0; 3]
    } else {
        replica_v3::binary::from_value(previous["guard_streaks"].clone())?
    };
    let mut last = previous["last_evaluation"].clone();
    let mut cross = Value::Null;
    let mut ordinary = Value::Null;
    let mut ordinary_generation_ok = false;
    let mut complete = false;
    let mut panel_receipts = record!({});
    let outcome = (|| -> Result<()> {
        if last.is_null() {
            if p["node"] == "A2" {
                let path = progress_path(&p, "zero_evaluation_file")?;
                if p["zero_evaluation_hash"] != file_hash(&path)? {
                    return Err(Error::Corrupt("selected parent raw changed".into()));
                }
                last = read_metadata(&path)?;
                if last["model_content_hash"] != p["initial_model_hash"]
                    || last["final_evaluation_complete"] != true
                {
                    return Err(Error::Corrupt("derived A2 zero provenance".into()));
                }
                let parent_policy_path = path
                    .parent()
                    .and_then(Path::parent)
                    .ok_or_else(|| Error::Corrupt("zero parent policy".into()))?
                    .join("policy.r3b");
                let parent_policy = read_metadata(&parent_policy_path)?;
                let parent_inputs = load_verified_inputs(
                    &progress_path(&parent_policy, "a0")?,
                    Some(&parent_policy),
                )?;
                verify_progress_evaluation(&parent_policy, &last, &parent_inputs, &l, true)?;
                last.as_object_mut().unwrap().remove("bindings");
                last.as_object_mut().unwrap().remove("decision");
                last["derived_parent_new_updates"] = last["new_updates"].clone();
                last["new_updates"] = record!(0);
                last["evidence_level"] = p["zero_evaluation"].clone();
            } else {
                let d = read_metadata(&a0.join("normal-dev.r3b"))?;
                let w = read_metadata(&a0.join("watch32.r3b"))?;
                let watch_score = skill_score(
                    w["rows"]
                        .as_array()
                        .ok_or_else(|| Error::Corrupt("A0 watch rows".into()))?,
                )?;
                last = record!({"new_updates":0,"dev":d["score"],"watch":watch_score,"dev_rows":d["rows"],"watch_rows":w["rows"],
                "final_evaluation_complete":true,"model_content_hash":p["initial_model_hash"],"evidence_level":"DERIVED_FROM_A0_IDENTICAL_MODEL",
                "dev_file_hash":file_hash(&a0.join("normal-dev.r3b"))?,"watch_file_hash":file_hash(&a0.join("watch32.r3b"))?});
            }
            verify_progress_evaluation(&p, &last, &inputs, &l, true)?;
            bind_progress_evaluation(&p, &mut last, &inputs, &l)?;
            record_bound_panel(&segment.join("eval-0000.r3b"), &last, &mut panel_receipts)?;
        }
        loop {
            let n = state.step - start;
            if last["new_updates"] == n && n > 0 {
                verify_progress_evaluation(&p, &last, &inputs, &l, false)?;
            }
            progress_evaluation_boundary(
                &p,
                n,
                &l.model.weight_hash()?,
                None,
                &mut last,
                &mut streak,
                &segment,
                control,
                || {
                    save_arm(
                        &mut l,
                        &state,
                        &adam,
                        &segment.join(format!("step-{n:04}")),
                        "RECOVERY_SCREENING",
                    )
                },
            )?;
            control.check("progress_next_boundary")?;
            if [128, 256, 512].contains(&n)
                && (last["new_updates"] != n || last["final_evaluation_complete"] != true)
            {
                l.model.refresh_identity()?;
                let mut evaluation = skill_evaluation(&l, dev, &frozen.watch, control)?;
                evaluation["watch"] = skill_score(
                    evaluation["watch_rows"]
                        .as_array()
                        .ok_or_else(|| Error::Corrupt("watch rows".into()))?,
                )?;
                evaluation["new_updates"] = record!(n);
                evaluation["model_content_hash"] = record!(l.model.weight_hash()?);
                bind_progress_evaluation(&p, &mut evaluation, &inputs, &l)?;
                progress_evaluation_boundary(
                    &p,
                    n,
                    &l.model.weight_hash()?,
                    Some(evaluation),
                    &mut last,
                    &mut streak,
                    &segment,
                    control,
                    || {
                        save_arm(
                            &mut l,
                            &state,
                            &adam,
                            &segment.join(format!("step-{n:04}")),
                            "RECOVERY_SCREENING",
                        )
                    },
                )?;
                println!(
                    "NODE={} ARM={arm} eval update={n} dev={}/256 entity={} event={} watch={}/32 errors={}+{} streak={streak:?}",
                    p["node"],
                    last["dev"]["exact_matches"],
                    last["dev"]["entity_correct"],
                    last["dev"]["event_id_correct"],
                    last["watch"]["exact_matches"],
                    last["dev"]["generation_error_cases"],
                    last["watch"]["generation_error_cases"]
                );
            }
            if n == 512 {
                let rows = evaluate_panel(&l, cross_cases, control);
                cross = progress_copy_score(&rows, 512)?;
                let binding = bind_progress_panel(
                    &p,
                    "cross",
                    cross_cases,
                    &inputs.hashes["cross"],
                    state.step,
                    &rows,
                    &l,
                )?;
                record_bound_panel(
                    &segment.join("cross.r3b"),
                    &record!({"policy":"normal_greedy_v1","score":cross,"rows":rows,"bindings":{"cross":binding}}),
                    &mut panel_receipts,
                )?;
                control.check("progress_cross_recorded")?;
                let rows = evaluate_panel(&l, ordinary_cases, control);
                ordinary = summarize(&rows)?;
                ordinary_generation_ok = skill_score(&rows)?["generation_error_cases"] == 0;
                let binding = bind_progress_panel(
                    &p,
                    "ordinary",
                    ordinary_cases,
                    &inputs.hashes["ordinary"],
                    state.step,
                    &rows,
                    &l,
                )?;
                record_bound_panel(
                    &segment.join("ordinary400.r3b"),
                    &record!({"policy":"normal_greedy_v1","score":ordinary,"rows":rows,"bindings":{"ordinary":binding}}),
                    &mut panel_receipts,
                )?;
                control.check("progress_ordinary_recorded")?;
                complete = rows.len() == 400
                    && cross["denominator"] == 512
                    && last["final_evaluation_complete"] == true;
                break;
            }
            let (indices, sampler) = &tape[n];
            let b = batch(&framed, indices, &Device::Cpu)?;
            let targets: usize = indices
                .iter()
                .map(|i| framed[*i].tokens.len() - framed[*i].response_start)
                .sum();
            if denominators[n] != (b.tokens, targets) {
                return Err(Error::Corrupt(
                    "registered actual token denominator changed".into(),
                ));
            }
            if state.consumed_tokens - start_input + b.tokens as u64 > 2_000_000
                || state.target_tokens - start_target + targets as u64 > 500_000
                || ledger.0 + (state.step - segment_start.0) as u64 >= 2048
            {
                control.observe(StopReason::TokenBudget);
                return control.stop_result();
            }
            let (ce, obj, actual_targets) = response_loss(
                &l.model.forward(&b.input, Some(&b.valid))?,
                &b,
                c.first_target_weight,
            )?;
            control.check("progress_forward")?;
            let ce = ce.to_scalar::<f32>()?;
            let objective = obj.to_scalar::<f32>()?;
            if !ce.is_finite() || !objective.is_finite() || actual_targets != targets {
                return Err(Error::Model("nonfinite loss/target denominator".into()));
            }
            let gradients = obj.backward()?;
            control.check("progress_backward")?;
            let gradients = l
                .model
                .vars
                .iter()
                .map(|(name, var)| -> Result<_> {
                    let g = gradients
                        .get(var)
                        .ok_or_else(|| Error::Model(format!("missing gradient {name}")))?;
                    Ok((
                        name.clone(),
                        ((g * targets as f64)?.detach() / targets as f64)?,
                    ))
                })
                .collect::<Result<BTreeMap<_, _>>>()?;
            let rate = progress_lr(
                p["lr_policy"].as_str().unwrap(),
                n + 1 + progress_u64(&p, "lr_offset")? as usize,
            )?;
            control.check("progress_before_optimizer")?;
            let (norm, delta) =
                adam.step_constant(&l.model.vars, &gradients, &c, state.step + 1, rate)?;
            state.step += 1;
            state.consumed_tokens += b.tokens as u64;
            state.target_tokens += targets as u64;
            state.sampler_state = *sampler;
            state.train_loss = Some(ce as f64);
            state.validation_loss = None;
            let _ = control.check("progress_optimizer_committed");
            let row = record!({"new_update":n+1,"cumulative_model_step":state.step,"optimizer_step":state.step,"lr_clock":n+1+progress_u64(&p,"lr_offset")? as usize,"lr":rate,"lr_bits":rate.to_bits(),
                "sampler_state":sampler,"indices":indices,"ids":indices.iter().map(|i|&episodes[*i].id).collect::<Vec<_>>(),
                "input_tokens":b.tokens,"target_tokens":targets,"consumed_input_tokens":state.consumed_tokens-start_input,"consumed_target_tokens":state.target_tokens-start_target,
                "ce":ce,"objective":objective,"gradient_norm":norm,"update_norm":delta,"rss_kib":control.last_rss_kib,"elapsed_seconds":control.start.elapsed().as_secs_f64()});
            replica_v3::binary::write_value_record(&mut log, &row)?;
            log.flush()?;
            if (n + 1).is_multiple_of(32) {
                println!(
                    "NODE={} ARM={arm} step={} cumulative={} input={} target={} lr={rate:.9} ce={ce:.6}",
                    p["node"],
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
    if let Err(e) = &outcome {
        control.classify_error(e);
    }
    let model_hash = l.model.weight_hash()?;
    let adam_hash = optimizer_hash(&adam.moments)?;
    log.sync_all()?;
    let trace_hash = file_hash(&segment.join("trace.r3rows"))?;
    for n in [0, 128, 256, 512] {
        let name = format!("eval-{n:04}.r3b");
        let path = segment.join(&name);
        if path.exists() {
            let raw = read_metadata(&path)?;
            panel_receipts[&name] = record!({"sha256":file_hash(&path)?,"bindings":raw["bindings"]});
        }
    }
    let mut checkpoint_hash = Value::Null;
    let mut result = finish_arm(control, complete, |reason| {
        save_arm(&mut l, &state, &adam, &segment.join("final"), reason)?;
        checkpoint_hash = record!(file_hash(&segment.join("final"))?);
        if p["parent_sha256"] != file_hash(&parent)?
            || p["parent_receipt_sha256"] != file_hash(&progress_path(&p, "parent_receipt")?)?
        {
            return Err(Error::Corrupt(
                "fork parent changed during execution".into(),
            ));
        }
        Ok(())
    });
    let cleanup = result["cleanup_elapsed_seconds"]
        .as_f64()
        .is_none_or(|n| n > 120.);
    let raw_gate = complete
        && last["dev"]["skill_pass"] == true
        && cross["skill_pass"] == true
        && ordinary["qa"][0]
            .as_u64()
            .is_some_and(|n| n >= p["anchor_floor"].as_u64().unwrap_or(u64::MAX));
    let details = record!({"contract":PROGRESS_CONTRACT,"node":p["node"],"arm":arm,"policy_sha256":file_hash(&policy_path)?,"new_updates":state.step-start,
        "segment_updates":state.step-segment_start.0,"segment_input_tokens":state.consumed_tokens-segment_start.1,"segment_target_tokens":state.target_tokens-segment_start.2,
        "segment_elapsed_seconds":control.start.elapsed().as_secs_f64(),"additional_input_tokens":state.consumed_tokens-start_input,"additional_target_tokens":state.target_tokens-start_target,
        "cumulative_model_step":state.step,"sampler_state":state.sampler_state,"model_content_hash":model_hash,"adam_hash":adam_hash,
        "last_evaluation":last,"evaluation_decision_digest":digest(&last["decision"])?,"panel_receipts":panel_receipts,
        "verified_inputs":inputs.hashes,"cross":cross,"ordinary":ordinary,"guard_streaks":streak,"raw_development_gate":raw_gate&&control.stop.is_none()&&!cleanup,
        "candidate_eligible":raw_gate&&control.stop.is_none()&&!cleanup&&ordinary_generation_ok&&last["watch"]["generation_error_cases"]==0,"seal":"NOT_OPENED","goal1_ready":false,"cleanup_limit_exceeded":cleanup,
        "resume_allowed":control.reason()==Some("TIME_BUDGET") && control.observed==[StopReason::TimeBudget] && last["final_evaluation_complete"]==true && ledger.3+control.start.elapsed().as_secs_f64()<7200. && !cleanup && result["checkpoint_saved"]==true,
        "error":outcome.as_ref().err().map(ToString::to_string)});
    result
        .as_object_mut()
        .unwrap()
        .extend(details.as_object().unwrap().clone());
    let exposed: Vec<_> = tape[..state.step - start]
        .iter()
        .flat_map(|(indices, _)| indices.iter().copied())
        .collect();
    result["exposure"] = record!({"draws":exposed.len(),"unique_views":exposed.iter().copied().collect::<BTreeSet<_>>().len(),
        "unique_bases":exposed.iter().map(|i|scene(&episodes[*i])).collect::<BTreeSet<_>>().len(),
        "anchor_draws":exposed.iter().filter(|i|**i<2048).count(),"focus_draws":exposed.iter().filter(|i|**i>=2048).count(),"trace_sha256":trace_hash});
    if cleanup {
        result["comparison_eligible"] = record!(false);
    }
    if result["checkpoint_saved"] == true {
        result["checkpoint_file_sha256"] = checkpoint_hash;
    }
    log.sync_all()?;
    save(&segment.join("result.r3b"), &result)?;
    println!(
        "NODE={} ARM={arm} reason={} updates={} dev={} cross={} ordinary={} raw_gate={} resume={}",
        p["node"],
        result["reason"],
        result["new_updates"],
        result["last_evaluation"]["dev"]["exact_matches"],
        result["cross"]["exact_matches"],
        result["ordinary"]["qa"],
        result["raw_development_gate"],
        result["resume_allowed"]
    );
    outcome.and(control.stop_result())
}

fn progress_pair_delta(a: &[Value], b: &[Value]) -> Result<Value> {
    if a.len() != b.len()
        || a.iter().zip(b).any(|(a, b)| {
            a["id"] != b["id"]
                || a["expected"] != b["expected"]
                || a["prompt_digest"] != b["prompt_digest"]
        })
    {
        return Err(Error::Corrupt(
            "paired raw membership/input mismatch".into(),
        ));
    }
    let mut cells = [0usize; 4];
    let mut bases: BTreeMap<String, (bool, bool)> = BTreeMap::new();
    for (a, b) in a.iter().zip(b) {
        let x = a["exact_match"] == true;
        let y = b["exact_match"] == true;
        cells[x as usize * 2 + y as usize] += 1;
        let base = a["scene"]
            .as_str()
            .ok_or_else(|| Error::Corrupt("paired base missing".into()))?;
        let entry = bases.entry(base.into()).or_insert((true, true));
        entry.0 &= x;
        entry.1 &= y;
    }
    let mut groups = [0usize; 4];
    for (x, y) in bases.values() {
        groups[*x as usize * 2 + *y as usize] += 1;
    }
    Ok(
        record!({"case_order":["both_wrong","treatment_gain","treatment_loss","both_correct"],"cases":cells,"base_all_views":groups,"base_count":bases.len(),"statistical_superiority":"NOT_CLAIMED"}),
    )
}
struct PanelSpec<'a> {
    id: &'a str,
    cases: &'a [Episode],
    split_hash: &'a str,
    policy_hash: &'a str,
    source: &'a str,
    model: &'a str,
    step: usize,
}
fn panel_binding(spec: &PanelSpec<'_>, rows: &[Value], l: &Loaded) -> Result<Value> {
    let completed = rows
        .iter()
        .filter(|r| r["generation_completed"] == true && r["interruption"].is_null())
        .count();
    Ok(
        record!({"version":"native-panel-v1","panel_id":spec.id,"expected_count":spec.cases.len(),
        "completed_count":completed,"ordered_id_digest":digest(&spec.cases.iter().map(|e|&e.id).collect::<Vec<_>>())?,
        "case_content_digest":digest(spec.cases)?,"dataset_digest":spec.split_hash,
        "model_hash":spec.model,"tokenizer_hash":l.tokenizer.semantic_id(),"model_step":spec.step,
        "policy_hash":spec.policy_hash,"decoding":"normal_greedy_v1","evaluator_source":spec.source,
        "raw_rows_digest":digest(rows)?,"complete":rows.len()==spec.cases.len() && completed==spec.cases.len(),
        "terminal":if rows.len()==spec.cases.len() && completed==spec.cases.len(){"COMPLETE"}else{"INCOMPLETE"}}),
    )
}
fn verify_panel_and_rescore(
    spec: &PanelSpec<'_>,
    rows: &[Value],
    binding: Option<&Value>,
    l: &Loaded,
    legacy: bool,
) -> Result<Value> {
    if rows.len() != spec.cases.len() || rows.is_empty() {
        return Err(Error::Corrupt(format!(
            "INCOMPLETE: {} expected{} actual{}",
            spec.id,
            spec.cases.len(),
            rows.len()
        )));
    }
    verify_historical_rows(spec.cases, rows)?;
    let actual_binding = panel_binding(spec, rows, l)?;
    if let Some(binding) = binding {
        if binding != &actual_binding {
            return Err(Error::Corrupt(
                "INTEGRITY_FAIL: panel receipt/model/step/digest".into(),
            ));
        }
    } else if !legacy {
        return Err(Error::Corrupt(
            "INTEGRITY_FAIL: missing panel receipt; explicit READ_ONLY_REAUDIT required".into(),
        ));
    }
    let mut verified = rows.to_vec();
    for ((row, original), e) in verified.iter_mut().zip(rows).zip(spec.cases) {
        if row["id"] != e.id
            || row["scene"] != scene(e)
            || row["family"] != e.family
            || row["category"] != e.category
        {
            return Err(Error::Corrupt(
                "INTEGRITY_FAIL: ordered membership/metadata".into(),
            ));
        }
        if row["generation_started"] != true
            || row["generation_completed"] != true
            || row.get("interruption") != Some(&Value::Null)
        {
            return Err(Error::Corrupt(
                "INCOMPLETE: interrupted/not executed row".into(),
            ));
        }
        let prompt = l.tokenizer.prepare(
            &e.request,
            l.model.config.context as u32,
            &l.model.config.id()?,
        )?;
        let raw: Vec<u32> = replica_v3::binary::from_value(row["raw_tokens"].clone())?;
        let eos = raw.iter().position(|id| *id == EOS);
        let bytes_ids: Vec<_> = raw
            .iter()
            .copied()
            .take_while(|id| *id >= neural::SPECIALS as u32)
            .collect();
        if row["request_digest"] != digest(&e.request)?
            || row["prompt_digest"] != digest(&prompt.token_ids)?
            || row["native_prompt_digest"] != prompt.token_digest
            || row["provided"] != record!(prompt.provided)
            || row["excluded"] != record!(prompt.excluded)
            || row["prompt_length"] != prompt.token_ids.len()
            || row["eos_index"] != record!(eos)
            || row["raw_generated_count"] != raw.len()
            || row["raw_bytes"] != bytes_receipt(&l.tokenizer, &bytes_ids)
        {
            return Err(Error::Corrupt(
                "INTEGRITY_FAIL: raw prompt/token/bytes/EOS".into(),
            ));
        }
        let control_token = raw
            .iter()
            .any(|id| *id < neural::SPECIALS as u32 && *id != EOS);
        if eos.is_some_and(|i| i + 1 != raw.len())
            || raw.len() > e.request.limits.max_tokens as usize
        {
            return Err(Error::Corrupt("INTEGRITY_FAIL: generation sequence".into()));
        }
        let decoded = l.tokenizer.decode(&bytes_ids);
        if let Some(t) = row.get("teacher_forced_diagnostic_after_generation")
            && t.get("first_difference_field").is_some()
        {
            let bytes = l.tokenizer.decode_bytes(&bytes_ids)?;
            let position = e
                .answer
                .as_bytes()
                .iter()
                .zip(&bytes)
                .position(|(a, b)| a != b)
                .or_else(|| {
                    (e.answer.len() != bytes.len()).then_some(e.answer.len().min(bytes.len()))
                });
            if t["first_byte_difference"] != record!(position)
                || t["first_difference_field"] != record!(position.map(|n| field_at(&e.answer, n)))
            {
                return Err(Error::Corrupt(
                    "INTEGRITY_FAIL: first raw difference field".into(),
                ));
            }
        }
        if !control_token && (eos.is_some() || raw.len() == e.request.limits.max_tokens as usize) {
            let finish = if eos.is_some() { "stop" } else { "length" };
            let text = decoded.as_ref().ok();
            let error = decoded.as_ref().err().map(ToString::to_string);
            if row["actual"] != record!(text)
                || row["error"] != record!(error)
                || row["error_class"] != record!(error.as_ref().map(|_| "strict_utf8"))
                || row["generation"]["finish"] != finish
                || row["finish_reason"] != finish
                || row["generation"]["tokens"] != record!(bytes_ids)
                || row["generation"]["generated"] != raw.len()
            {
                return Err(Error::Corrupt(
                    "INTEGRITY_FAIL: decode/finish/error mismatch".into(),
                ));
            }
        } else if !row["actual"].is_null()
            || !row["generation"].is_null()
            || !row["error"].is_string()
            || (control_token && row["error_class"] != "control_token")
        {
            return Err(Error::Corrupt(
                "INTEGRITY_FAIL: failed generation receipt".into(),
            ));
        }
        let exact = decoded.as_ref().ok().is_some_and(|s| s == &e.answer)
            && eos.is_some()
            && !control_token
            && row.get("error") == Some(&Value::Null)
            && row["finish_reason"] == "stop";
        let c = components(row["actual"].as_str(), &e.answer, &prompt.provided);
        let whitespace = row["actual"]
            .as_str()
            .is_some_and(|s| !s.is_empty() && s.trim().is_empty());
        if original["exact_match"] != exact
            || original["components"] != c
            || original["whitespace_only"] != whitespace
        {
            return Err(Error::Corrupt(
                "INTEGRITY_FAIL: raw score/components mismatch".into(),
            ));
        }
        row["exact_match"] = record!(exact);
        row["components"] = c;
    }
    match spec.id {
        "ordinary" => summarize(&verified),
        "cross" => progress_copy_score(&verified, spec.cases.len()),
        _ => skill_score(&verified),
    }
}
fn verify_score(recorded: &Value, derived: &Value) -> Result<()> {
    // Floating teacher aggregates use typed binary values; strict integer scores/gates must match exactly.
    for (key, value) in derived
        .as_object()
        .ok_or_else(|| Error::Corrupt("score object".into()))?
    {
        if value.is_f64() {
            if recorded[key]
                .as_f64()
                .zip(value.as_f64())
                .is_none_or(|(a, b)| (a - b).abs() > 1e-12)
            {
                return Err(Error::Corrupt(format!("INTEGRITY_FAIL: score {key}")));
            }
        } else if recorded[key] != *value {
            return Err(Error::Corrupt(format!("INTEGRITY_FAIL: score {key}")));
        }
    }
    Ok(())
}
fn record_bound_panel(path: &Path, value: &Value, receipts: &mut Value) -> Result<()> {
    let bytes = value.to_storage_vec()?;
    neural::write_new(path, &bytes)?;
    receipts[path.file_name().unwrap().to_str().unwrap()] =
        record!({"sha256":neural::hash(&bytes),"bindings":value["bindings"]});
    Ok(())
}
fn verify_progress_evaluation(
    p: &Value,
    e: &Value,
    inputs: &VerifiedProgressInputs,
    l: &Loaded,
    legacy: bool,
) -> Result<()> {
    let n = progress_u64(e, "new_updates")? as usize;
    let model = e["model_content_hash"]
        .as_str()
        .ok_or_else(|| Error::Corrupt("evaluation model".into()))?;
    let policy_hash = digest(p)?;
    for (id, cases, hash) in [
        ("dev", inputs.dev.as_slice(), inputs.hashes["dev"].as_str()),
        (
            "watch",
            inputs.frozen.watch.as_slice(),
            inputs.hashes["frozen"].as_str(),
        ),
    ] {
        let spec = PanelSpec {
            id,
            cases,
            split_hash: hash.unwrap(),
            policy_hash: &policy_hash,
            source: p["source_digest"].as_str().unwrap_or("UNKNOWN"),
            model,
            step: progress_u64(&p["initial_state"], "step")? as usize + n,
        };
        let rows = e[format!("{id}_rows")]
            .as_array()
            .ok_or_else(|| Error::Corrupt("missing evaluation rows".into()))?;
        let derived = verify_panel_and_rescore(&spec, rows, e["bindings"].get(id), l, legacy)?;
        verify_score(&e[id], &derived)?;
    }
    if e["final_evaluation_complete"] != true {
        return Err(Error::Corrupt("INCOMPLETE: dev/watch evaluation".into()));
    }
    Ok(())
}
fn bind_progress_panel(
    p: &Value,
    id: &str,
    cases: &[Episode],
    hash: &Value,
    step: usize,
    rows: &[Value],
    l: &Loaded,
) -> Result<Value> {
    panel_binding(
        &PanelSpec {
            id,
            cases,
            split_hash: hash
                .as_str()
                .ok_or_else(|| Error::Corrupt("panel split hash".into()))?,
            policy_hash: &digest(p)?,
            source: p["source_digest"].as_str().unwrap_or("UNKNOWN"),
            model: &l.model.weight_hash()?,
            step,
        },
        rows,
        l,
    )
}
fn bind_progress_evaluation(
    p: &Value,
    e: &mut Value,
    inputs: &VerifiedProgressInputs,
    l: &Loaded,
) -> Result<()> {
    let step = progress_u64(&p["initial_state"], "step")? as usize
        + progress_u64(e, "new_updates")? as usize;
    for (id, cases, hash) in [
        ("dev", inputs.dev.as_slice(), &inputs.hashes["dev"]),
        (
            "watch",
            inputs.frozen.watch.as_slice(),
            &inputs.hashes["frozen"],
        ),
    ] {
        e["bindings"][id] = bind_progress_panel(
            p,
            id,
            cases,
            hash,
            step,
            e[format!("{id}_rows")]
                .as_array()
                .ok_or_else(|| Error::Corrupt("evaluation rows".into()))?,
            l,
        )?;
    }
    Ok(())
}
fn read_panel(path: &Path) -> Result<(Value, String)> {
    let bytes = neural::read_bounded(path, 16 * 1024 * 1024)?;
    Ok((replica_v3::binary::from_slice(&bytes)?, neural::hash(&bytes)))
}
fn verify_panel_file(
    result: &Value,
    path: &Path,
    raw: &Value,
    raw_hash: &str,
    legacy: bool,
) -> Result<()> {
    let name = path.file_name().unwrap().to_str().unwrap();
    if let Some(receipt) = result["panel_receipts"].get(name) {
        if receipt["sha256"] != raw_hash || receipt["bindings"] != raw["bindings"] {
            return Err(Error::Corrupt(
                "INTEGRITY_FAIL: raw file digest/binding".into(),
            ));
        }
    } else if !legacy {
        return Err(Error::Corrupt(
            "INTEGRITY_FAIL: absent raw file receipt; READ_ONLY_REAUDIT only".into(),
        ));
    }
    Ok(())
}
fn verify_endpoint_panels(
    p: &Value,
    result: &mut Value,
    segment: &Path,
    inputs: &VerifiedProgressInputs,
    legacy: bool,
    control: &mut RunControl,
) -> Result<()> {
    control.check("verify_endpoint_panels")?;
    let l = checkpoint::load(&segment.join("final"), Device::Cpu, true)?;
    let state = l
        .manifest
        .training
        .as_ref()
        .ok_or_else(|| Error::Corrupt("native endpoint training state".into()))?;
    if result["model_content_hash"] != l.model.weight_hash()?
        || result["adam_hash"] != optimizer_hash(&l.optimizer)?
        || result["cumulative_model_step"] != state.step
        || result["sampler_state"] != state.sampler_state
        || result["new_updates"]
            .as_u64()
            .and_then(|n| p["initial_state"]["step"].as_u64().map(|start| start + n))
            != Some(state.step as u64)
        || p["tokenizer_hash"] != l.tokenizer.semantic_id()
    {
        return Err(Error::Corrupt(
            "INTEGRITY_FAIL: endpoint native/model/Adam/step/tokenizer".into(),
        ));
    }
    let evaluation = &result["last_evaluation"];
    verify_progress_evaluation(p, evaluation, inputs, &l, legacy)?;
    if evaluation["new_updates"] != result["new_updates"]
        || evaluation["model_content_hash"] != result["model_content_hash"]
    {
        return Err(Error::Corrupt(
            "INTEGRITY_FAIL: endpoint evaluation identity".into(),
        ));
    }
    if !legacy {
        if result["evaluation_decision_digest"] != digest(&evaluation["decision"])? {
            return Err(Error::Corrupt(
                "INTEGRITY_FAIL: terminal decision binding".into(),
            ));
        }
        let mut copy = evaluation.clone();
        let mut streak: [u64; 3] = replica_v3::binary::from_value(result["guard_streaks"].clone())?;
        let mut decision_control = RunControl::new(
            Arc::new(AtomicBool::new(false)),
            Duration::from_secs(1),
            u64::MAX,
        )?;
        let stop = reconcile_evaluation_decision(
            p,
            progress_u64(result, "new_updates")? as usize,
            &l.model.weight_hash()?,
            &mut copy,
            &mut streak,
            &mut decision_control,
        )?;
        if stop
            && !result["control"]["observed_conditions"]
                .as_array()
                .is_some_and(|v| v.contains(&record!("QUALITY_GUARD")))
        {
            return Err(Error::Corrupt(
                "INTEGRITY_FAIL: missing sticky quality stop".into(),
            ));
        }
    }
    let policy_hash = digest(p)?;
    let model = l.model.weight_hash()?;
    let mut bindings = record!({});
    let mut ordinary_generation_ok = false;
    for (id, file, cases, hash) in [
        (
            "cross",
            "cross.r3b",
            inputs.cross.as_slice(),
            &inputs.hashes["cross"],
        ),
        (
            "ordinary",
            "ordinary400.r3b",
            inputs.ordinary.as_slice(),
            &inputs.hashes["ordinary"],
        ),
    ] {
        let path = segment.join(file);
        let (raw, raw_hash) = read_panel(&path)?;
        verify_panel_file(result, &path, &raw, &raw_hash, legacy)?;
        let spec = PanelSpec {
            id,
            cases,
            split_hash: hash.as_str().unwrap(),
            policy_hash: &policy_hash,
            source: p["source_digest"].as_str().unwrap_or("UNKNOWN"),
            model: &model,
            step: state.step,
        };
        let rows = raw["rows"]
            .as_array()
            .ok_or_else(|| Error::Corrupt("INCOMPLETE: final rows".into()))?;
        let score = verify_panel_and_rescore(&spec, rows, raw["bindings"].get(id), &l, legacy)?;
        if id == "ordinary" {
            ordinary_generation_ok = skill_score(rows)?["generation_error_cases"] == 0;
        }
        verify_score(&raw["score"], &score)?;
        verify_score(&result[id], &score)?;
        result[id] = score;
        bindings[id] = record!({"current_file_hash":raw_hash,"current_binding":panel_binding(&spec,rows,&l)?,"verified_rows":rows,
            "historical_receipt_binding":if result["panel_receipts"].get(file).is_some(){"PRESENT"}else{"ABSENT"}});
    }
    if result["candidate_eligible"] == true
        && (!ordinary_generation_ok
            || result["last_evaluation"]["watch"]["generation_error_cases"] != 0)
    {
        return Err(Error::Corrupt(
            "INTEGRITY_FAIL: candidate has non-normal final generation".into(),
        ));
    }
    let gate = result["last_evaluation"]["dev"]["skill_pass"] == true
        && result["cross"]["skill_pass"] == true
        && result["ordinary"]["qa"][0]
            .as_u64()
            .is_some_and(|n| n >= p["anchor_floor"].as_u64().unwrap_or(u64::MAX))
        && progress_endpoint_safe(result);
    if result["raw_development_gate"] != gate {
        return Err(Error::Corrupt(
            "INTEGRITY_FAIL: derived endpoint gate".into(),
        ));
    }
    result["current_reaudit"] = record!({"verified_inputs":inputs.hashes,"panels":bindings,
        "source_provenance":p["source_digest"],"historical_candidate_promotion":"NOT_AUTHORIZED"});
    Ok(())
}
fn progress_close(root: &Path, control: &mut RunControl) -> Result<()> {
    let result = progress_close_to(root, &root.join("comparison.r3b"), false, control);
    if let Err(e) = &result {
        let _ = save(
            &root.join("close-integrity-failure.r3b"),
            &record!({"status":if e.to_string().contains("INCOMPLETE"){"INCOMPLETE"}else{"INTEGRITY_FAIL"},
            "error":e.to_string(),"comparison_eligible":false,"candidate_eligible":false,"new_small_updates":0}),
        );
    }
    result
}
fn progress_close_to(
    root: &Path,
    output: &Path,
    legacy: bool,
    control: &mut RunControl,
) -> Result<()> {
    control.check("progress_close")?;
    let pair = read_metadata(&root.join("pair.r3b"))?;
    let mut ends = Vec::new();
    let mut evals = Vec::new();
    let mut policies = Vec::new();
    let mut traces = Vec::new();
    for (arm_index, arm) in pair["arms"]
        .as_array()
        .ok_or_else(|| Error::Corrupt("pair arms".into()))?
        .iter()
        .enumerate()
    {
        let name = arm.as_str().ok_or_else(|| Error::Corrupt("arm".into()))?;
        let dir = root.join(name);
        let p = read_metadata(&dir.join("policy.r3b"))?;
        if pair["policy_hashes"][arm_index] != file_hash(&dir.join("policy.r3b"))? {
            return Err(Error::Corrupt(
                "INTEGRITY_FAIL: registered close policy".into(),
            ));
        }
        let mut entries = std::fs::read_dir(&dir)?
            .map(|e| e.map(|e| e.path()))
            .collect::<std::io::Result<Vec<_>>>()?;
        entries.retain(|p| p.is_dir());
        entries.sort();
        let mut evaluations = BTreeMap::new();
        let mut last = Value::Null;
        let mut trace = Vec::new();
        let inputs = load_verified_inputs(&progress_path(&p, "a0")?, Some(&p))?;
        let episodes = &inputs.train;
        for segment in entries {
            last = read_metadata(&segment.join("result.r3b"))?;
            if last["policy_sha256"] != file_hash(&dir.join("policy.r3b"))?
                || last["checkpoint_file_sha256"] != file_hash(&segment.join("final"))?
            {
                return Err(Error::Corrupt("close identity".into()));
            }
            if last["exposure"]["trace_sha256"] != file_hash(&segment.join("trace.r3rows"))? {
                return Err(Error::Corrupt("actual draw trace changed".into()));
            }
            let before = trace.len();
            for row in replica_v3::binary::read_value_records(&segment.join("trace.r3rows"))? {
                let n = trace.len();
                let ids: Vec<usize> = replica_v3::binary::from_value(p["tape"][n][0].clone())?;
                if row["new_update"] != n + 1
                    || row["indices"] != p["tape"][n][0]
                    || row["sampler_state"] != p["tape"][n][1]
                    || row["input_tokens"] != p["denominators"][n][0]
                    || row["target_tokens"] != p["denominators"][n][1]
                    || row["ids"] != record!(ids.iter().map(|i| &episodes[*i].id).collect::<Vec<_>>())
                    || row["optimizer_step"]
                        != progress_u64(&p["initial_state"], "step")? + n as u64 + 1
                    || row["lr_bits"]
                        != progress_lr(
                            p["lr_policy"].as_str().unwrap(),
                            n + 1 + progress_u64(&p, "lr_offset")? as usize,
                        )?
                        .to_bits()
                    || ["ce", "objective", "gradient_norm", "update_norm"]
                        .iter()
                        .any(|key| row[*key].as_f64().is_none_or(|x| !x.is_finite()))
                {
                    return Err(Error::Corrupt(
                        "executed trace differs from registered policy/tape/denominator".into(),
                    ));
                }
                trace.push(row);
            }
            if last["segment_updates"] != trace.len() - before || last["new_updates"] != trace.len()
            {
                return Err(Error::Corrupt("actual optimizer call ledger".into()));
            }
            for n in [0, 128, 256, 512] {
                let path = segment.join(format!("eval-{n:04}.r3b"));
                if path.exists() {
                    let (e, raw_hash) = read_panel(&path)?;
                    let native = if n == 0 {
                        progress_path(&p, "parent")?
                    } else {
                        segment.join(format!("step-{n:04}"))
                    };
                    let l = checkpoint::load(&native, Device::Cpu, false)?;
                    if e["new_updates"] != n || e["model_content_hash"] != l.model.weight_hash()? {
                        return Err(Error::Corrupt(
                            "INTEGRITY_FAIL: evaluation native model/step".into(),
                        ));
                    }
                    if l.manifest.training.as_ref().is_none_or(|s| {
                        s.step
                            != progress_u64(&p["initial_state"], "step").unwrap_or(u64::MAX)
                                as usize
                                + n
                    }) {
                        return Err(Error::Corrupt(
                            "INTEGRITY_FAIL: native evaluation clock".into(),
                        ));
                    }
                    verify_panel_file(&last, &path, &e, &raw_hash, legacy)?;
                    verify_progress_evaluation(&p, &e, &inputs, &l, legacy)?;
                    if evaluations.insert(n, e).is_some() {
                        return Err(Error::Corrupt("duplicate evaluation step".into()));
                    }
                }
            }
            last["segment"] = record!(segment);
        }
        if last.is_null() || last["reason"] == "TIME_BUDGET" {
            return Err(Error::Invalid(
                "both arms must close before comparison".into(),
            ));
        }
        let endpoint_segment = progress_path(&last, "segment")?;
        verify_endpoint_panels(&p, &mut last, &endpoint_segment, &inputs, legacy, control)?;
        let n = progress_u64(&last, "new_updates")? as usize;
        let evaluation = evaluations
            .get(&n)
            .ok_or_else(|| Error::Corrupt("INCOMPLETE: endpoint raw evaluation".into()))?;
        if evaluation_identity(&p, evaluation)?
            != evaluation_identity(&p, &last["last_evaluation"])?
        {
            return Err(Error::Corrupt(
                "INTEGRITY_FAIL: result/evaluation content".into(),
            ));
        }
        ends.push(last);
        evals.push(evaluations);
        policies.push(p);
        traces.push(trace);
    }
    if ends.len() != 2
        || policies[0]["initial_model_hash"] != policies[1]["initial_model_hash"]
        || policies[0]["initial_adam_hash"] != policies[1]["initial_adam_hash"]
        || policies[0]["initial_state"] != policies[1]["initial_state"]
    {
        return Err(Error::Corrupt("paired parents differ".into()));
    }
    let mut paired = Vec::new();
    for (n, e) in &evals[0] {
        if let Some(other) = evals[1].get(n)
            && e["final_evaluation_complete"] == true
            && other["final_evaluation_complete"] == true
        {
            paired.push(record!({"new_updates":n,"dev":progress_pair_delta(e["dev_rows"].as_array().unwrap(),other["dev_rows"].as_array().unwrap())?,"watch":progress_pair_delta(e["watch_rows"].as_array().unwrap(),other["watch_rows"].as_array().unwrap())?}));
        }
    }
    let safe = progress_endpoint_safe;
    let rank = |r: &Value| {
        let d = &r["last_evaluation"]["dev"];
        (
            d["exact_matches"].as_u64().unwrap_or(0),
            d["entity_correct"]
                .as_u64()
                .unwrap_or(0)
                .min(d["event_id_correct"].as_u64().unwrap_or(0)),
            std::cmp::Reverse(d["generation_error_cases"].as_u64().unwrap_or(u64::MAX)),
        )
    };
    let accepted = (0..2).find(|i| {
        !legacy
            && safe(&ends[*i])
            && ends[*i]["candidate_eligible"] == true
            && ends[*i]["raw_development_gate"] == true
    });
    let selected = accepted.or_else(|| {
        (0..2)
            .filter(|i| {
                safe(&ends[*i])
                    && rank(&ends[*i]).0
                        >= policies[*i]["baseline_dev"].as_u64().unwrap_or(u64::MAX)
            })
            .max_by(|a, b| rank(&ends[*a]).cmp(&rank(&ends[*b])).then_with(|| b.cmp(a)))
    });
    let exposure_equal = policies[0]["tape_hash"] == policies[1]["tape_hash"]
        && policies[0]["denominators_hash"] == policies[1]["denominators_hash"];
    if pair["node"] == "A1" && !exposure_equal {
        return Err(Error::Corrupt("C/L exposure differs".into()));
    }
    let common_prefix = traces[0].len().min(traces[1].len());
    if pair["node"] == "A1" {
        for (a, b) in traces[0][..common_prefix]
            .iter()
            .zip(&traces[1][..common_prefix])
        {
            for key in [
                "new_update",
                "optimizer_step",
                "sampler_state",
                "indices",
                "ids",
                "input_tokens",
                "target_tokens",
                "consumed_input_tokens",
                "consumed_target_tokens",
            ] {
                if a[key] != b[key] {
                    return Err(Error::Corrupt(format!(
                        "actual paired exposure mismatch: {key}"
                    )));
                }
            }
        }
    }
    if pair["node"] == "A2" {
        if policies[0]["lr_policy"] != policies[1]["lr_policy"]
            || policies[0]["lr_offset"] != policies[1]["lr_offset"]
        {
            return Err(Error::Corrupt("F/N LR changed".into()));
        }
        for (a, b) in traces[0][..common_prefix]
            .iter()
            .zip(&traces[1][..common_prefix])
        {
            if a["indices"].as_array().unwrap()[..4] != b["indices"].as_array().unwrap()[..4]
                || a["ids"].as_array().unwrap()[..4] != b["ids"].as_array().unwrap()[..4]
                || a["lr_bits"] != b["lr_bits"]
                || a["optimizer_step"] != b["optimizer_step"]
                || a["sampler_state"] != b["sampler_state"]
            {
                return Err(Error::Corrupt("actual F/N anchor/LR/clock mismatch".into()));
            }
        }
    }
    let mut final_pairs = Value::Null;
    if safe(&ends[0]) && safe(&ends[1]) {
        final_pairs = record!({});
        for name in ["cross", "ordinary"] {
            let a = &ends[0]["current_reaudit"]["panels"][name];
            let b = &ends[1]["current_reaudit"]["panels"][name];
            final_pairs[name] = progress_pair_delta(
                a["verified_rows"]
                    .as_array()
                    .ok_or_else(|| Error::Corrupt("final panel rows".into()))?,
                b["verified_rows"]
                    .as_array()
                    .ok_or_else(|| Error::Corrupt("final panel rows".into()))?,
            )?;
        }
    }
    // Pair the verified owned rows before dropping duplicate payloads from the bounded report.
    // The immutable original files, current byte hashes and complete bindings remain inspectable.
    for end in &mut ends {
        for id in ["cross", "ordinary"] {
            if let Some(panel) = end["current_reaudit"]["panels"][id].as_object_mut() {
                panel.remove("verified_rows");
            }
        }
    }
    control.seal_terminal()?;
    save(
        output,
        &record!({"contract":PROGRESS_CONTRACT,"node":pair["node"],"paired":paired,"policies_same_exposure":exposure_equal,
        "endpoints":ends,"selected_index":selected,"selected_arm":selected.map(|i|&pair["arms"][i]),"raw_development_gate":accepted.is_some(),"actual_common_prefix_updates":common_prefix,"final_paired":final_pairs,
        "mode":if legacy{"READ_ONLY_REAUDIT"}else{"BOUND_PANEL_CLOSE"},"evidence_level":"DERIVED_FROM_EXISTING_LOGS",
        "historical_receipt_binding":if ends.iter().any(|r|r["panel_receipts"].is_null()){"ABSENT"}else{"PRESENT"},"current_raw_recount":"VERIFIED",
        "source_sha":source_commit()?,"execution_binary_sha256":file_hash(&std::env::current_exe()?)?,
        "historical_candidate_promotion":"NOT_AUTHORIZED","new_small_updates":0,
        "next":if legacy{"DIAGNOSTIC_ONLY_NO_PROMOTION"}else if accepted.is_some(){"A3_FRESH_PROCESS_THEN_SEAL"}else if selected.is_some() && pair["node"]=="A1"{"A2_CONDITIONAL"}else{"H3_BUDGET_CLOSED"},
        "endpoint_exposure_equal":ends[0]["new_updates"]==ends[1]["new_updates"],"h3_seal":"NOT_OPENED","s4":"NOT_PASSED","goal1_ready":false,"control":control.receipt()}),
    )?;
    println!(
        "NODE={} closed selected={selected:?} raw_gate={} paired_points={}",
        pair["node"],
        accepted.is_some(),
        paired.len()
    );
    Ok(())
}

fn progress_observe(
    root: &Path,
    output: &Path,
    kind: &str,
    control: &mut RunControl,
) -> Result<()> {
    let recount = read_metadata(&output.join("reaudit.r3b"))?;
    if recount["current_raw_recount"] != "VERIFIED" || recount["mode"] != "READ_ONLY_REAUDIT" {
        return Err(Error::Invalid(
            "verified read-only B4 recount required before observations".into(),
        ));
    }
    let (arm, n) = match kind {
        "F16" => ("F", 512),
        "N16" => ("N", 512),
        "N256" => ("N", 256),
        _ => return Err(Error::Invalid("bounded observation only".into())),
    };
    let p = read_metadata(&root.join(arm).join("policy.r3b"))?;
    let inputs = load_verified_inputs(&progress_path(&p, "a0")?, Some(&p))?;
    let endpoint = recount["endpoints"]
        .as_array()
        .and_then(|rows| rows.iter().find(|r| r["arm"] == arm))
        .ok_or_else(|| Error::Corrupt("recount endpoint".into()))?;
    if endpoint["policy_sha256"] != file_hash(&root.join(arm).join("policy.r3b"))? {
        return Err(Error::Corrupt(
            "FROZEN_INPUT_MISMATCH: observation policy".into(),
        ));
    }
    for previous in ["F16", "N16", "N256"] {
        let path = output.join(previous).join("result.r3b");
        if path.exists() && read_metadata(&path)?["status"] != "VERIFIED" {
            return Err(Error::Invalid(
                "prior observation failed; no further generation".into(),
            ));
        }
    }
    let segment = progress_path(endpoint, "segment")?;
    let native = segment.join(if n == 512 {
        "final".into()
    } else {
        format!("step-{n:04}")
    });
    if !native.is_file() {
        return Err(Error::Invalid("NOT_RUN_CHECKPOINT_MISSING".into()));
    }
    let l = checkpoint::load(&native, Device::Cpu, true)?;
    let state = l
        .manifest
        .training
        .as_ref()
        .ok_or_else(|| Error::Corrupt("observed native state".into()))?;
    if state.step != progress_u64(&p["initial_state"], "step")? as usize + n
        || l.tokenizer.semantic_id() != p["tokenizer_hash"]
    {
        return Err(Error::Corrupt("observation checkpoint lineage".into()));
    }
    let old = read_metadata(&segment.join(format!("eval-{n:04}.r3b")))?;
    if old["model_content_hash"] != l.model.weight_hash()? {
        return Err(Error::Corrupt("observation raw/model".into()));
    }
    verify_progress_evaluation(&p, &old, &inputs, &l, true)?;
    let dir = output.join(kind);
    std::fs::create_dir(&dir)?; // No retry can overwrite or silently spend the same allowance again.
    let indices: Vec<usize> = if n == 512 {
        let rows = old["dev_rows"].as_array().unwrap();
        [true, false]
            .into_iter()
            .flat_map(|correct| {
                rows.iter()
                    .enumerate()
                    .filter(move |(_, r)| r["exact_match"] == correct)
                    .take(8)
                    .map(|(i, _)| i)
            })
            .collect()
    } else {
        vec![]
    };
    let observation_source = source_commit()?;
    save(
        &dir.join("registration.r3b"),
        &record!({"kind":kind,"mode":if n==256{"POST_HOC_DIAGNOSTIC"}else{"FRESH_PROCESS_RAW_PARITY"},
        "checkpoint":native,"native_sha256":file_hash(&native)?,"model_hash":l.model.weight_hash()?,"tokenizer":l.tokenizer.semantic_id(),"step":state.step,
        "policy_sha256":file_hash(&root.join(arm).join("policy.r3b"))?,"verified_inputs":inputs.hashes,
        "source_sha":observation_source,"binary_sha256":file_hash(&std::env::current_exe()?)?,
        "selection":if n==256{"reuse verified dev256; generate full frozen CROSS512 and ordinary400 once; post-hoc DEVELOPMENT"}else{"first8 exact and first8 incorrect in frozen dev order; fixed before generation; not quality estimate"},
        "indices":indices,"maximum_new_generations":if n==256{912}else{16},"new_small_updates":0,
        "post_hoc_notice":"이 checkpoint는 기존 dev 결과를 본 뒤 선정한 사후 진단 대상이다. 과거 F/N 실험의 사전등록 승자가 아니며, 이번에는 seal·제품 승격·학습 재개를 하지 않는다."}),
    )?;
    let mut report = record!({"kind":kind,"dev":old["dev"],"new_small_updates":0,"seal":"NOT_OPENED","candidate_eligible":false,"resume_allowed":false,
        "registration_sha256":file_hash(&dir.join("registration.r3b"))?,"source_sha":observation_source,
        "reused_dev_sha256":file_hash(&segment.join(format!("eval-{n:04}.r3b")))?,"panel_receipts":{},
        "evidence_level":"EXECUTED_THIS_RUN","dev_evidence":"DERIVED_FROM_EXISTING_LOGS","generation_budget":if n==256{912}else{16}});
    let outcome = (|| -> Result<()> {
        if n == 512 {
            let selected: Vec<_> = indices.iter().map(|i| inputs.dev[*i].clone()).collect();
            let rows = evaluate_panel(&l, &selected, control);
            let spec = PanelSpec {
                id: "dev",
                cases: &selected,
                split_hash: inputs.hashes["dev"].as_str().unwrap(),
                policy_hash: &digest(&p)?,
                source: &observation_source,
                model: &l.model.weight_hash()?,
                step: state.step,
            };
            let binding = panel_binding(&spec, &rows, &l)?;
            record_bound_panel(
                &dir.join("raw.r3b"),
                &record!({"rows":rows,"bindings":{"dev":binding},"new_small_updates":0}),
                &mut report["panel_receipts"],
            )?;
            control.check("fresh_parity_recorded")?;
            verify_panel_and_rescore(&spec, &rows, Some(&binding), &l, false)?;
            let mut differences = Vec::new();
            for (row, i) in rows.iter().zip(&indices) {
                for key in [
                    "raw_tokens",
                    "raw_bytes",
                    "actual",
                    "error",
                    "error_class",
                    "finish_reason",
                    "eos_index",
                    "prompt_digest",
                ] {
                    if row[key] != old["dev_rows"][*i][key] {
                        differences.push(record!({"index":i,"field":key}));
                    }
                }
            }
            report["raw_differences"] = record!(differences);
            if !differences.is_empty() {
                return Err(Error::Corrupt("fresh-process raw parity".into()));
            }
        } else {
            for (id, cases, hash) in [
                ("cross", inputs.cross.as_slice(), &inputs.hashes["cross"]),
                (
                    "ordinary",
                    inputs.ordinary.as_slice(),
                    &inputs.hashes["ordinary"],
                ),
            ] {
                control.check("posthoc_next_panel")?;
                let rows = evaluate_panel(&l, cases, control);
                let spec = PanelSpec {
                    id,
                    cases,
                    split_hash: hash.as_str().unwrap(),
                    policy_hash: &digest(&p)?,
                    source: &observation_source,
                    model: &l.model.weight_hash()?,
                    step: state.step,
                };
                let binding = panel_binding(&spec, &rows, &l)?;
                record_bound_panel(
                    &dir.join(format!("{id}.r3b")),
                    &record!({"rows":rows,"bindings":{id:binding},"new_small_updates":0}),
                    &mut report["panel_receipts"],
                )?;
                control.check("posthoc_panel_recorded")?;
                report[id] = verify_panel_and_rescore(&spec, &rows, Some(&binding), &l, false)?;
            }
        }
        Ok(())
    })();
    if let Err(e) = &outcome {
        control.classify_error(e);
    }
    let _ = control.seal_terminal();
    report["control"] = control.receipt();
    report["status"] = record!(if outcome.is_ok() && control.stop.is_none() {
        "VERIFIED"
    } else {
        "FAILED_OR_INCOMPLETE"
    });
    report["error"] = record!(outcome.as_ref().err().map(ToString::to_string));
    save(&dir.join("result.r3b"), &report)?;
    println!("{report}");
    outcome.and(control.stop_result())
}

fn optimizer_hash(tensors: &BTreeMap<String, Tensor>) -> Result<String> {
    use sha2::{Digest, Sha256};
    let mut h = Sha256::new();
    for (name, t) in tensors {
        h.update(name.as_bytes());
        h.update([0]);
        for &d in t.dims() {
            h.update((d as u64).to_le_bytes());
        }
        for x in t.flatten_all()?.to_vec1::<f32>()? {
            h.update(x.to_le_bytes());
        }
    }
    Ok(format!("{:x}", h.finalize()))
}
fn progress_copy_score(rows: &[Value], expected: usize) -> Result<Value> {
    let mut score = skill_score(rows)?;
    score["skill_pass"] = record!(
        expected > 0
            && rows.len() == expected
            && score["exact_matches"].as_u64().unwrap_or(0) >= (expected * 95).div_ceil(100) as u64
            && score["entity_correct"].as_u64().unwrap_or(0)
                >= (expected * 99).div_ceil(100) as u64
            && score["event_id_correct"].as_u64().unwrap_or(0)
                >= (expected * 99).div_ceil(100) as u64
            && score["generation_error_cases"] == 0
    );
    let mut strata = BTreeMap::<String, [usize; 2]>::new();
    for row in rows {
        for part in row["family"]
            .as_str()
            .unwrap_or_default()
            .split('/')
            .filter(|p| p.starts_with("pattern-") || p.starts_with("kind-"))
        {
            let count = strata.entry(part.to_owned()).or_default();
            count[0] += usize::from(row["exact_match"] == true);
            count[1] += 1;
        }
    }
    score["cross_strata"] = record!(strata);
    score["generation_policy"] = record!("normal_greedy_v1");
    Ok(score)
}
fn verify_cross_development(cases: &[Episode], prior: &[Episode], l: &Loaded) -> Result<Value> {
    verify_cross_panel(cases, prior, l, 512)
}
fn verify_cross_panel(
    cases: &[Episode],
    prior: &[Episode],
    l: &Loaded,
    expected: usize,
) -> Result<Value> {
    if ![512, 2048].contains(&expected) || cases.len() != expected {
        return Err(Error::Invalid(
            "CROSS denominator; never shrink capacity failures".into(),
        ));
    }
    let seen: BTreeSet<_> = prior
        .iter()
        .flat_map(|e| e.request.evidence.items.iter())
        .filter_map(|r| exact_fact(&r.original_excerpt).map(|f| f.0.to_owned()))
        .collect();
    let prior_bindings: BTreeSet<_> = prior.iter().map(|e| e.binding.as_str()).collect();
    let prior_text: BTreeSet<_> = prior
        .iter()
        .flat_map(|e| {
            e.request
                .evidence
                .items
                .iter()
                .map(|r| r.original_excerpt.as_str())
        })
        .collect();
    let mut bases = BTreeSet::new();
    let mut strata = BTreeMap::<String, usize>::new();
    let mut identities = BTreeSet::new();
    let mut bindings = BTreeSet::new();
    for group in cases.as_chunks::<4>().0 {
        if !bases.insert(scene(&group[0])) || group.iter().any(|e| scene(e) != scene(&group[0])) {
            return Err(Error::Invalid("CROSS base/view identity".into()));
        }
        let mut facts = Vec::new();
        for (view, e) in group.iter().enumerate() {
            if e.category != 0
                || e.request.evidence.items.len() != 1
                || e.request.request_id != e.id
            {
                return Err(Error::Invalid("CROSS request".into()));
            }
            let r = &e.request.evidence.items[0];
            let (entity, context, value) = exact_fact(&r.original_excerpt)
                .ok_or_else(|| Error::Invalid("CROSS unsupported serialized fact".into()))?;
            let digits = entity.trim_start_matches(|c: char| !c.is_ascii_digit());
            let prefix = &entity[..entity.len() - digits.len()];
            let question = if view == 3 {
                "제공된 유일한 기록의 원문을 빠짐없이 쓰고 그 사건을 인용해줘.".to_owned()
            } else {
                format!(
                    "{entity}의 {context}에서 현재 유효한 사건의 원문을 빠짐없이 쓰고 그 사건을 인용해줘."
                )
            };
            if !["장치", "설비", "센서", "장비"].contains(&prefix)
                || !(1..=8).contains(&digits.len())
                || !digits.bytes().all(|b| b.is_ascii_digit())
                || seen.contains(entity)
                || usize::from_str_radix(&neural::hash(entity.as_bytes())[..8], 16).unwrap() % 3
                    == 2
                || r.version_status != "current"
                || r.excerpt_truncated
                || r.event_id <= 0
                || e.request.input != question
                || e.answer != format!("{} [event:{}]", r.original_excerpt, r.event_id)
                || e.binding != format!("{entity}/{context}/{value}")
                || prior_bindings.contains(e.binding.as_str())
                || prior_text.contains(r.original_excerpt.as_str())
            {
                return Err(Error::Invalid(
                    "CROSS ambiguity/seen entity/namespace/target".into(),
                ));
            }
            identities.insert(entity);
            bindings.insert(&e.binding);
            facts.push((entity, context, value, r.event_id, r.recorded_at));
            *strata
                .entry(format!(
                    "digits-{}/{}",
                    digits.len(),
                    if value.starts_with("경로") {
                        "short-value"
                    } else {
                        "direction"
                    }
                ))
                .or_default() += 1;
        }
        let a = facts[0];
        if a.0 == facts[1].0
            || a.1 != facts[1].1
            || a.2 != facts[1].2
            || a.0 != facts[2].0
            || a.1 != facts[2].1
            || a.2 == facts[2].2
            || a != facts[3]
            || facts.iter().any(|f| f.3 != a.3 || f.4 != a.4)
        {
            return Err(Error::Invalid("CROSS contrast semantics".into()));
        }
    }
    if bases.len() != expected / 4
        || strata.len() != 16
        || strata.values().any(|&n| n != expected / 16)
    {
        return Err(Error::Invalid("CROSS balanced length/value strata".into()));
    }
    let framed = samples(cases, &l.tokenizer, 512)?;
    Ok(
        record!({"status":"SERIALIZED_INPUT_VERIFIED","bases":bases.len(),"views":cases.len(),
        "unique_entities":identities.len(),"unique_bindings":bindings.len(),"strata":strata,
        "max_tokens":framed.iter().map(|s|s.tokens.len()).max(),"gold_source":"serialized single current record; no chosen_index"}),
    )
}
fn progress_teacher_relation(rows: &[Value]) -> Value {
    let mut table = [[0usize; 2]; 2];
    let mut missing = 0;
    let mut lengths = BTreeMap::<u64, usize>::new();
    for row in rows {
        let t = &row["teacher_forced_diagnostic_after_generation"];
        if let (Some(correct), Some(total)) = (
            t["teacher_forced_correct_tokens"].as_u64(),
            t["target_tokens_including_eos"].as_u64(),
        ) {
            table[usize::from(correct == total)][usize::from(row["exact_match"] == true)] += 1;
            *lengths.entry(total).or_default() += 1;
        } else {
            missing += 1;
        }
    }
    record!({"derived_from":"recorded per-case argmax counts, not p^mean_length","teacher_all_correct_by_free_exact_false_true":table,
        "target_length_histogram":lengths,"missing_teacher_cases":missing,"denominator":rows.len()})
}
fn progress_tokenizer(
    l: &Loaded,
    train: &[Episode],
    dev: &[Episode],
    control: &mut RunControl,
) -> Result<Value> {
    let mut kinds = Vec::new();
    let mut groups = BTreeMap::<String, BTreeMap<String, usize>>::new();
    let mut fragments = BTreeMap::<u32, Value>::new();
    for id in 0..l.tokenizer.vocab_size() as u32 {
        let kind = if id < 8 {
            "reserved"
        } else {
            let b = l.tokenizer.decode_bytes(&[id])?;
            match std::str::from_utf8(&b) {
                Ok(_) => "standalone_valid",
                Err(e) if e.error_len().is_none() => "incomplete_prefix",
                Err(_) => "invalid_standalone",
            }
        };
        let group = if id < 8 {
            "reserved8"
        } else if id < 264 {
            "base256"
        } else {
            "learned"
        };
        *groups
            .entry(group.into())
            .or_default()
            .entry(kind.into())
            .or_default() += 1;
        kinds.push(kind);
        if id >= 8 && kind != "standalone_valid" {
            fragments.insert(id,record!({"classification":kind,"bytes":l.tokenizer.decode_bytes(&[id])?,"train":[0,0],"dev":[0,0],"neighbors_train":{},"neighbors_dev":{}}));
        }
    }
    for (split, cases) in [("train", train), ("dev", dev)] {
        for e in cases {
            control.check("progress_tokenizer_case")?;
            let framed = samples(std::slice::from_ref(e), &l.tokenizer, 512)?;
            let s = &framed[0];
            let gold = l.tokenizer.encode(e.answer.as_bytes())?;
            if l.tokenizer.decode(&gold)? != e.answer {
                return Err(Error::Corrupt("gold tokenizer roundtrip".into()));
            }
            for (i, &id) in s.tokens.iter().enumerate() {
                if let Some(f) = fragments.get_mut(&id) {
                    let region = usize::from(i >= s.response_start);
                    f[split][region] = record!(f[split][region].as_u64().unwrap() + 1);
                    let key = format!(
                        "{}:{}",
                        i.checked_sub(1)
                            .map(|j| s.tokens[j].to_string())
                            .unwrap_or_else(|| "BOS".into()),
                        s.tokens
                            .get(i + 1)
                            .map(u32::to_string)
                            .unwrap_or_else(|| "END".into())
                    );
                    let field = format!("neighbors_{split}");
                    let count = f[&field][&key].as_u64().unwrap_or(0) + 1;
                    f[&field][&key] = record!(count);
                }
            }
        }
    }
    Ok(
        record!({"vocab":l.tokenizer.vocab_size(),"groups":groups,"fragments":fragments,
        "occurrence_regions":["prompt","response_including_eos"],"roundtrip_cases":train.len()+dev.len(),
        "interpretation":"standalone invalid/incomplete bytes are not a codec defect; arbitrary concatenations may fail UTF-8"}),
    )
}
fn progress_exposure(
    run: &Path,
    train: &[Episode],
    l: &Loaded,
    control: &mut RunControl,
) -> Result<Value> {
    let policy = read_metadata(&run.parent().unwrap().join("policy.r3b"))?;
    let tape: Vec<(Vec<usize>, u64)> = replica_v3::binary::from_value(policy["tape"].clone())?;
    let framed = samples(train, &l.tokenizer, 512)?;
    let mut rows = BTreeMap::<usize, Value>::new();
    let mut path = run.to_owned();
    for depth in 0..4 {
        control.check("progress_exposure_segment")?;
        let receipt = read_metadata(&path.join("result.r3b"))?;
        let p = read_metadata(&path.parent().unwrap().join("policy.r3b"))?;
        if receipt["policy_sha256"] != file_hash(&path.parent().unwrap().join("policy.r3b"))?
            || receipt["checkpoint_file_sha256"] != file_hash(&path.join("final"))?
            || p["tape"] != policy["tape"]
        {
            return Err(Error::Corrupt("progress trace provenance".into()));
        }
        for row in replica_v3::binary::read_value_records(&path.join("trace.r3rows"))? {
            let n = row["new_update"]
                .as_u64()
                .filter(|n| *n > 0 && *n <= 1024)
                .ok_or_else(|| Error::Corrupt("progress update index".into()))?
                as usize;
            if rows.insert(n, row).is_some() {
                return Err(Error::Corrupt("duplicate consumed update".into()));
            }
        }
        if let Some(parent) = p["renewal"]["original_checkpoint"].as_str() {
            if depth == 3 {
                return Err(Error::Corrupt("progress lineage depth".into()));
            }
            path = Path::new(parent)
                .parent()
                .ok_or_else(|| Error::Corrupt("progress parent".into()))?
                .into();
        } else {
            break;
        }
    }
    let mut exposure = vec![0usize; train.len()];
    let (mut inputs, mut targets) = (0u64, 0u64);
    let mut sum_lr = 0.;
    let mut decay_product = 1.;
    for n in 1..=1024 {
        let row = rows
            .get(&n)
            .ok_or_else(|| Error::Corrupt("missing consumed update".into()))?;
        let (indices, rng) = tape
            .get(n - 1)
            .ok_or_else(|| Error::Corrupt("short frozen tape".into()))?;
        if indices.len() != 8 || indices.iter().any(|&i| i >= framed.len()) {
            return Err(Error::Corrupt("progress tape sample bounds".into()));
        }
        let input = indices
            .iter()
            .map(|&i| framed[i].tokens.len() - 1)
            .sum::<usize>();
        let target = indices
            .iter()
            .map(|&i| framed[i].tokens.len() - framed[i].response_start)
            .sum::<usize>();
        if row["indices"] != record!(indices)
            || row["ids"] != record!(indices.iter().map(|&i| &train[i].id).collect::<Vec<_>>())
            || row["sampler_state"] != *rng
            || row["input_tokens"] != input
            || row["target_tokens"] != target
        {
            return Err(Error::Corrupt(
                "progress actual exposure/token mismatch".into(),
            ));
        }
        for &i in indices {
            exposure[i] += 1;
        }
        inputs += input as u64;
        targets += target as u64;
        let lr = row["lr"]
            .as_f64()
            .filter(|r| r.is_finite() && *r > 0.)
            .ok_or_else(|| Error::Corrupt("trace LR".into()))?;
        sum_lr += lr;
        decay_product *= 1. - lr * l.manifest.training.as_ref().unwrap().config.weight_decay;
    }
    let pool = |range: std::ops::Range<usize>| {
        record!({"available_views":range.len(),
        "available_bases":range.clone().map(|i|scene(&train[i])).collect::<BTreeSet<_>>().len(),
        "available_input_tokens":range.clone().map(|i|framed[i].tokens.len()-1).sum::<usize>(),
        "available_target_tokens":range.clone().map(|i|framed[i].tokens.len()-framed[i].response_start).sum::<usize>(),
        "actual_draws":range.clone().map(|i|exposure[i]).sum::<usize>(),
        "unique_consumed_views":range.clone().filter(|&i|exposure[i]>0).count(),
        "min_exposure":range.clone().map(|i|exposure[i]).min(),"max_exposure":range.map(|i|exposure[i]).max()})
    };
    Ok(
        record!({"evidence_level":"DERIVED_FROM_LOGS","updates":rows.len(),"input_tokens":inputs,"target_tokens":targets,
        "anchor":pool(0..2048),"focus":pool(2048..4096),"actual_lr_sum":sum_lr,
        "decay_only_product":decay_product,"scope":"this H3 branch only; prior history and gradient updates excluded"}),
    )
}
fn progress_qk(
    l: &Loaded,
    cases: &[Episode],
    rows: &[Value],
    control: &mut RunControl,
) -> Result<Value> {
    let mut picked = Vec::new();
    if let Some(pair) = cases
        .iter()
        .zip(rows)
        .find(|(_, r)| r["exact_match"] == true)
    {
        picked.push(pair);
    }
    if let Some(pair) = cases
        .iter()
        .zip(rows)
        .find(|(_, r)| r["error_class"] == "strict_utf8")
    {
        picked.push(pair);
    }
    let mut observations = Vec::new();
    for (e, row) in picked {
        control.check("progress_qk_case")?;
        let prepared = l.tokenizer.prepare(
            &e.request,
            l.model.config.context as u32,
            &l.model.config.id()?,
        )?;
        let raw: Vec<u32> = replica_v3::binary::from_value(row["raw_tokens"].clone())?;
        let pos = row["teacher_forced_diagnostic_after_generation"]["first_difference"]["index"]
            .as_u64()
            .map_or(0, |n| n as usize);
        if pos > raw.len() {
            return Err(Error::Corrupt("QK recorded prefix".into()));
        }
        let mut prefix = prepared.token_ids;
        prefix.extend(&raw[..pos]);
        let mut heads = Vec::new();
        let scalar_stats = |xs: &[f32]| {
            record!({"min":xs.iter().copied().fold(f32::INFINITY,f32::min),
            "max":xs.iter().copied().fold(f32::NEG_INFINITY,f32::max),
            "rms":(xs.iter().map(|&x|f64::from(x).powi(2)).sum::<f64>()/xs.len() as f64).sqrt()})
        };
        let mut observer = |layer: usize, q: &Tensor, k: &Tensor, mask: &Tensor| -> Result<()> {
            control.check("progress_qk_layer")?;
            let (_, h, t, d) = q.dims4()?;
            let qlast = q.narrow(2, t - 1, 1)?;
            let logits = (qlast.contiguous()?.matmul(
                &neural::transformer::repeat_kv(k, h)?
                    .transpose(2, 3)?
                    .contiguous()?,
            )? / (d as f64).sqrt())?
            .squeeze(0)?
            .squeeze(1)?
            .to_vec2::<f32>()?;
            let allowed = mask.narrow(2, t - 1, 1)?.flatten_all()?.to_vec1::<f32>()?;
            let qv = qlast.squeeze(0)?.squeeze(1)?.to_vec2::<f32>()?;
            let kv = k.squeeze(0)?.to_vec3::<f32>()?;
            let qg = l.model.vars[&format!("layer.{layer}.q_norm")]
                .squeeze(1)?
                .to_vec2::<f32>()?;
            let kg = l.model.vars[&format!("layer.{layer}.k_norm")]
                .squeeze(1)?
                .to_vec2::<f32>()?;
            for head in 0..h {
                let kh = head / (h / kv.len());
                let mut entries: Vec<_> = logits[head]
                    .iter()
                    .copied()
                    .enumerate()
                    .filter(|(i, _)| allowed[*i] > 0.)
                    .collect();
                if entries.len() < 2 || entries.iter().any(|(_, x)| !x.is_finite()) {
                    return Err(Error::Model(
                        "nonfinite/insufficient QK observations".into(),
                    ));
                }
                entries.sort_by(|a, b| b.1.total_cmp(&a.1).then(a.0.cmp(&b.0)));
                let max = f64::from(entries[0].1);
                let z = entries
                    .iter()
                    .map(|(_, x)| (f64::from(*x) - max).exp())
                    .sum::<f64>();
                let entropy = -entries
                    .iter()
                    .map(|(_, x)| {
                        let p = (f64::from(*x) - max).exp() / z;
                        p * p.ln()
                    })
                    .filter(|x| x.is_finite())
                    .sum::<f64>();
                let kn: Vec<f32> = entries
                    .iter()
                    .map(|(i, _)| kv[kh][*i].iter().map(|x| x * x).sum::<f32>().sqrt())
                    .collect();
                let inf = |v: &[f32]| v.iter().map(|x| f64::from(x.abs())).fold(0., f64::max);
                heads.push(record!({"layer":layer,"head":head,"kv_head":kh,"query_position":prefix.len()-1,
                    "q_gain":scalar_stats(&qg[head]),"k_gain":scalar_stats(&kg[kh]),
                    "q_post_rope_l2":qv[head].iter().map(|x|f64::from(*x).powi(2)).sum::<f64>().sqrt(),
                    "k_post_rope_l2":scalar_stats(&kn),"allowed_keys":entries.len(),
                    "finite_unmasked_min":entries.last().unwrap().1,"finite_unmasked_max":entries[0].1,
                    "top1_top2_gap":entries[0].1-entries[1].1,"entropy":entropy,
                    "conservative_abs_bound":(d as f64).sqrt()*inf(&qg[head])*inf(&kg[kh]),
                    "top_keys":entries.iter().take(5).map(|(i,x)|record!({"position":i,"token_id":prefix[*i],"logit":x})).collect::<Vec<_>>() }));
            }
            Ok(())
        };
        let input = Tensor::new(prefix.as_slice(), &Device::Cpu)?.unsqueeze(0)?;
        let observed = l.model.forward_observed(&input, None, &mut observer)?;
        control.check("progress_qk_returned")?;
        let reference = l.model.forward(&input, None)?;
        let parity = compare(&observed, &reference)?;
        observations.push(record!({"id":e.id,"raw_prefix_position":pos,"observed_forward_parity":parity,"heads":heads}));
    }
    Ok(
        record!({"scope":"last query of one successful and one invalid recorded free-running prefix; observational, not causal",
        "cases":observations,"mask_bias_excluded":true,"parameter_changes":false,"temperature_changes":false}),
    )
}
fn progress_baseline(
    paths: [&Path; 5],
    output: &Path,
    seed: u64,
    control: &mut RunControl,
) -> Result<()> {
    let [baseline, run, inference, corpus, harness] = paths;
    control.check("progress_a0_start")?;
    let (frozen, ordinary_train, ordinary, baseline_hash) = verified_ordinary(baseline, None)?;
    let checked = verified_harness(harness)?;
    let receipt = read_metadata(&run.join("result.r3b"))?;
    let policy_path = run
        .parent()
        .ok_or_else(|| Error::Invalid("parent run".into()))?
        .join("policy.r3b");
    let policy = read_metadata(&policy_path)?;
    let parent = run.join("final");
    let l = checkpoint::load(&parent, Device::Cpu, true)?;
    let inference_model = checkpoint::load(inference, Device::Cpu, false)?;
    let state = l
        .manifest
        .training
        .as_ref()
        .ok_or_else(|| Error::Invalid("native resume/Adam required".into()))?;
    let (manifest, train, dev) = data::load_legacy(corpus)?;
    if receipt["reason"] != "SCREENING_BUDGET_REACHED"
        || receipt["new_updates"] != 1024
        || receipt["resume_allowed"] != false
        || receipt["candidate_eligible"] != false
        || receipt["checkpoint_saved"] != true
        || receipt["control"]["terminal_reason"] != "COMPLETED"
        || receipt["save_error"] != Value::Null
        || receipt["checkpoint_file_sha256"] != file_hash(&parent)?
        || receipt["policy_sha256"] != file_hash(&policy_path)?
        || receipt["model_content_hash"] != l.model.weight_hash()?
        || receipt["cumulative_model_step"] != state.step
        || receipt["sampler_state"] != state.sampler_state
        || policy["train_hash"] != manifest.train.sha256
        || policy["dev_hash"] != manifest.validation.sha256
        || state.corpus_hash != manifest.train.sha256
        || train.len() != 4096
        || dev.len() != 256
        || l.optimizer.is_empty()
        || inference_model.manifest.training.is_some()
        || inference_model.model.weight_hash()? != l.model.weight_hash()?
        || inference_model.tokenizer.semantic_id() != l.tokenizer.semantic_id()
        || inference_model.model.config != l.model.config
    {
        return Err(Error::Corrupt(
            "A0 parent native/policy/data identity".into(),
        ));
    }
    drop(inference_model);
    std::fs::create_dir(output)?;
    let mut report = record!({"contract":PROGRESS_CONTRACT,"node":"A0","source_commit":source_commit()?,"source_digest":checked["source_digest"],
        "binary_hash":file_hash(&std::env::current_exe()?)?,"parent":parent,"parent_file_sha256":file_hash(&parent)?,
        "inference_path":inference,"inference_sha256":file_hash(inference)?,"model_hash":l.model.weight_hash()?,
        "optimizer_hash":optimizer_hash(&l.optimizer)?,"tokenizer_hash":l.tokenizer.semantic_id(),"state":state,
        "policy_sha256":file_hash(&policy_path)?,"corpus":corpus,"train_hash":manifest.train.sha256,"dev_hash":manifest.validation.sha256,
        "baseline":baseline,"baseline_hash":baseline_hash,
        "normal_policy":"normal_greedy_v1","actual_new_updates":0,"seal":"NOT_OPENED","a0_pass":false,"goal1_ready":false});
    let outcome = (|| -> Result<()> {
        let prior = receipt["last_evaluation"]["dev_rows"]
            .as_array()
            .ok_or_else(|| Error::Corrupt("parent raw rows".into()))?;
        verify_historical_rows(&dev, prior)?;
        report["recorded_dev"] = progress_copy_score(prior, 256)?;
        report["teacher_relation"] = progress_teacher_relation(prior);
        report["exposure"] = progress_exposure(run, &train, &l, control)?;
        report["tokenizer"] = progress_tokenizer(&l, &train, &dev, control)?;
        save(&output.join("identity-tokenizer-exposure.r3b"), &report)?;
        let actual = evaluate_panel(&l, &dev, control);
        save(
            &output.join("normal-dev.r3b"),
            &record!({"policy":"normal_greedy_v1","model_hash":report["model_hash"],"score":progress_copy_score(&actual,256)?,"rows":actual}),
        )?;
        control.check("progress_dev_saved")?;
        if actual.len() != prior.len() {
            return Err(Error::Corrupt("A0 replay denominator".into()));
        }
        let mut differences = Vec::new();
        for (a, b) in actual.iter().zip(prior) {
            for field in [
                "id",
                "question",
                "evidence",
                "expected",
                "raw_tokens",
                "actual",
                "prompt_digest",
                "provided",
                "excluded",
                "finish_reason",
                "eos_index",
                "error_class",
                "exact_match",
            ] {
                if a[field] != b[field] {
                    differences.push(record!({"id":a["id"],"field":field}));
                }
            }
        }
        report["replay_differences"] = record!(differences);
        report["old_dev"] = progress_copy_score(&actual, 256)?;
        if !differences.is_empty() {
            return Err(Error::Corrupt("A0 normal replay changed".into()));
        }
        let original = evaluate_panel(&l, &ordinary, control);
        report["ordinary_parent"] = summarize(&original)?;
        save(
            &output.join("ordinary400.r3b"),
            &record!({"policy":"normal_greedy_v1","model_hash":report["model_hash"],"score":report["ordinary_parent"],"rows":original}),
        )?;
        control.check("progress_ordinary_saved")?;
        if original.len() != 400 {
            return Err(Error::Corrupt("A0 ordinary denominator".into()));
        }
        let watch: Vec<_> = frozen
            .watch
            .iter()
            .map(|e| {
                original
                    .iter()
                    .find(|r| r["id"] == e.id)
                    .cloned()
                    .ok_or_else(|| Error::Corrupt("A0 watch membership".into()))
            })
            .collect::<Result<_>>()?;
        report["watch"] = summarize(&watch)?;
        save(
            &output.join("watch32.r3b"),
            &record!({"rows":watch,"score":report["watch"],"derived_from":"same-process ordinary400 subset"}),
        )?;
        let mut parity = Vec::new();
        for (e, row) in dev
            .iter()
            .zip(&actual)
            .filter(|(_, r)| r["error_class"] == "strict_utf8")
        {
            let p = recorded_prefix_parity(&l, e, row, control)?;
            if p["recorded_prefix_reproduced"] != true || p["raw_bytes_hash_matches"] != true {
                return Err(Error::Corrupt("A0 prefix parity".into()));
            }
            parity.push(p);
        }
        report["final_utf8_prefix_parity"] = record!(parity);
        save(
            &output.join("final-prefix-parity.r3b"),
            &report["final_utf8_prefix_parity"],
        )?;
        report["qk"] = progress_qk(&l, &dev, &actual, control)?;
        save(&output.join("qk-observations.r3b"), &report["qk"])?;
        let mut reservations = train.clone();
        reservations.extend(dev.clone());
        reservations.extend(ordinary_train);
        reservations.extend(ordinary);
        let (cross, mut cross_meta) = data::crossed_copy_development(&reservations, seed)?;
        save(&output.join("cross-development.r3b"), &cross)?;
        if cross_meta["status"] == "CAPACITY" {
            report["cross"] = cross_meta;
            control.observe(StopReason::AuditIncomplete);
            return Err(Error::Invalid(
                "CROSS_CAPACITY; no seen IDs or reduced gate substituted".into(),
            ));
        }
        cross_meta["validation"] = verify_cross_development(&cross, &reservations, &l)?;
        cross_meta["file_sha256"] = record!(file_hash(&output.join("cross-development.r3b"))?);
        save(&output.join("cross-manifest.r3b"), &cross_meta)?;
        let cross_rows = evaluate_panel(&l, &cross, control);
        report["cross"] = cross_meta;
        report["cross_parent"] = progress_copy_score(&cross_rows, 512)?;
        save(
            &output.join("cross-parent.r3b"),
            &record!({"policy":"normal_greedy_v1","score":report["cross_parent"],"rows":cross_rows}),
        )?;
        control.check("progress_cross_saved")?;
        if cross_rows.len() != 512 {
            return Err(Error::Corrupt("CROSS incomplete".into()));
        }
        Ok(())
    })();
    if let Err(e) = &outcome {
        control.classify_error(e);
    }
    if report["parent_file_sha256"] != file_hash(&parent)?
        || report["inference_sha256"] != file_hash(inference)?
        || report["optimizer_hash"] != optimizer_hash(&l.optimizer)?
        || report["model_hash"] != l.model.weight_hash()?
    {
        control.observe(StopReason::IntegrityFail);
    }
    let sealed = control.seal_terminal();
    report["control"] = control.receipt();
    report["a0_pass"] = record!(outcome.is_ok() && sealed.is_ok());
    report["error"] = record!(outcome.as_ref().err().map(ToString::to_string));
    save(&output.join("summary.r3b"), &report)?;
    println!(
        "NODE=A0 pass={} old_dev={} ordinary={} cross={} updates=0 reason={}",
        report["a0_pass"],
        report["old_dev"]["exact_matches"],
        report["ordinary_parent"]["qa"],
        report["cross_parent"]["exact_matches"],
        report["control"]["terminal_reason"]
    );
    outcome.and(sealed)
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
        .join("policy.r3b");
    let policy = read_metadata(&policy_path)?;
    let receipt = read_metadata(&run.join("result.r3b"))?;
    let last = &receipt["last_evaluation"];
    let (manifest, train, dev) = data::load_legacy(corpus)?;
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
    let tape: Vec<(Vec<usize>, u64)> = replica_v3::binary::from_value(policy["tape"].clone())?;
    let trace = replica_v3::binary::read_value_records(&run.join("trace.r3rows"))?;
    let framed = samples(&train, &l.tokenizer, 512)?;
    let mut seen = BTreeSet::new();
    let (mut inputs, mut targets) = (0u64, 0u64);
    for (position, row) in trace.iter().enumerate() {
        control.check("skill_diagnostic_trace")?;
        let (indices, rng) = tape
            .get(position)
            .ok_or_else(|| Error::Corrupt("trace outside tape".into()))?;
        if row["new_update"] != position + 1
            || row["indices"] != record!(indices)
            || row["sampler_state"] != *rng
            || row["ids"]
                != record!(
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
        let panel = record!({"name":name,"indices":indices,"score":skill_score(&rows)?,"rows":rows,
            "heldout":false,"scope":"training views; unexposed views may share a base with an exposed view"});
        save(&output.join(format!("{name}.r3b")), &panel)?;
        control.check("skill_diagnostic_panel_saved")?;
        println!(
            "H3_DIAGNOSTIC {name} exact={}/32 entity={} event={} utf8={}",
            panel["score"]["exact_matches"],
            panel["score"]["entity_correct"],
            panel["score"]["event_id_correct"],
            panel["score"]["invalid_utf8"]
        );
        panels.push(record!({"name":name,"score":panel["score"]}));
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
        parity.push(recorded_prefix_parity(&l, e, row, control)?);
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
        &output.join("summary.r3b"),
        &record!({"scope":"H3 no-update diagnostic; not a skill gate","actual_optimizer_updates":0,
        "checkpoint_file_sha256":file_hash(&checkpoint)?,"model_content_hash":before,"trace_sha256":file_hash(&run.join("trace.r3rows"))?,
        "input_tokens_verified":inputs,"target_tokens_verified":targets,"completed_updates_verified":trace.len(),
        "unique_exposed_views":seen.len(),"panels":panels,"dev_first_difference_fields":fields,"cache_parity":parity,
        "terminal":control.receipt(),"candidate_eligible":false,"seal":"NOT_OPENED","data_hash":manifest.train.sha256}),
    )?;
    Ok(())
}
fn recorded_prefix_parity(
    l: &Loaded,
    e: &Episode,
    row: &Value,
    control: &mut RunControl,
) -> Result<Value> {
    if row["question"] != e.request.input
        || row["evidence"] != replica_v3::binary::to_value(&e.request.evidence)?
        || row["expected"] != e.answer
    {
        return Err(Error::Corrupt("error row content".into()));
    }
    let raw: Vec<u32> = replica_v3::binary::from_value(row["raw_tokens"].clone())?;
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
        positions.push(record!({"position":position,"prefix_tokens":prefix.len(),"recorded":token,"cached":cached_token,"full":full_token,"numeric":numeric}));
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
    Ok(record!({"id":e.id,"positions":positions,
        "recorded_prefix_reproduced":positions.iter().all(|p|p["recorded"]==p["cached"]&&p["recorded"]==p["full"]),
        "raw_bytes":row["raw_bytes"],"strict_utf8_error":std::str::from_utf8(&bytes).err().map(|e|e.to_string()),
        "raw_bytes_hash_matches":row["raw_bytes"]["sha256"]==neural::hash(&bytes),
        "scope":"recorded free-running prefix; no gold fed into forward"}))
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
    Ok(record!({"max_abs":max_abs,"abs_tolerance":1e-4,"rel_tolerance":1e-3,"violations":0}))
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
        checks.push(record!({"case":cases[row].id,"alone_batch":compare(&l.model.forward(&single.input,Some(&single.valid))?,&batched.narrow(0,row,1)?.narrow(1,0,s.tokens.len()-1)?)?}));
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
                checks.push(record!({"case":row,"length":len,"chunk":chunk,"prefill":parity,"same_prefix_next":compare(&full,&cached)?,"next_argmax_equal":full.argmax(2)?.to_vec2::<u32>()?==cached.argmax(2)?.to_vec2::<u32>()?}));
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
            checks.push(record!({"case":row,"length":len,"causal_future":compare(&direct.narrow(1,0,at)?,&modified.narrow(1,0,at)?)?}));
        }
    }
    // One real batch: labels/masks/EOS are independently derived from literal sample IDs.
    let (_, train, _) = data::load_legacy(&f.corpus)?;
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
            finite_difference.push(record!({"parameter":name,"coordinate":at,"autograd":g[at],"central_difference":reference,"step":h,"absolute_error":error,"tolerance":"0.002 + 0.05 * max(abs(reference),abs(autograd))"}));
            if error > 0.002 + 0.05 * reference.abs().max(g[at].abs() as f64) {
                return Err(Error::Model("actual objective finite difference".into()));
            }
        }
    }
    if before != l.model.weight_hash()? {
        return Err(Error::Corrupt("numeric diagnosis mutated model".into()));
    }
    let result = record!({"model_content_hash":before,"checks":checks,"actual_batch_targets":n,"plain_ce":ce.to_scalar::<f32>()?,"weighted_objective":obj.to_scalar::<f32>()?,"gradient_norms":norms,"finite_difference":finite_difference,"optimizer_updates":0,"elapsed_seconds":started.elapsed().as_secs_f64(),"status":"CHECKED_BOUNDARIES_PASS"});
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
fn read_metadata(path: &Path) -> Result<Value> {
    Ok(replica_v3::binary::value_from_slice(&neural::read_bounded(
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
        .map(|p| read_metadata(&p.join("policy.r3b")))
        .into_iter()
        .collect::<Result<Vec<_>>>()?;
    let traces = paths
        .map(|p| trace(&p.join("trace.r3rows")))
        .into_iter()
        .collect::<Result<Vec<_>>>()?;
    let results = paths
        .map(|p| read_metadata(&p.join("result.r3b")))
        .into_iter()
        .collect::<Result<Vec<_>>>()?;
    let base: TrainConfig = replica_v3::binary::from_value(policies[0]["config"].clone())?;
    let parent: TrainingState =
        replica_v3::binary::from_value(f.registry["GENERAL_QA_PARENT"]["manifest"]["training"].clone())?;
    let group = match factor {
        "group" => true,
        "schedule" => false,
        _ => return Err(Error::Invalid("comparison factor".into())),
    };
    let treatment = if group {
        let initial: TrainingState =
            replica_v3::binary::from_value(f.registry["U2_POLICY_START"]["manifest"]["training"].clone())?;
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
        || record!(treatment) != policies[1]["config"]
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
    let (manifest, episodes, _) = data::load_legacy(&f.corpus)?;
    if manifest.train.sha256 != f.train_hash {
        return Err(Error::Corrupt("comparison corpus binding".into()));
    }
    let pool: Vec<_> = (0..episodes.len()).collect();
    for ((policy, rows), result) in policies.iter().zip(&traces).zip(&results) {
        budget.check("schedule_trace")?;
        let config: TrainConfig = replica_v3::binary::from_value(policy["config"].clone())?;
        let tape: Vec<(Vec<usize>, u64)> = replica_v3::binary::from_value(policy["tape"].clone())?;
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
            if row["indices"] != record!(indices)
                || row["ids"] != record!(ids)
                || row["sampler_state"] != record!(rng)
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
            score["components"][key] = record!(counts);
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
            let evaluation = read_metadata(&path.join(format!("eval-{n:03}.r3b")))?;
            if evaluation["final_evaluation_complete"] != true {
                return Err(Error::Invalid("incomplete comparison evaluation".into()));
            }
            let rows: Vec<Value> = replica_v3::binary::from_value(evaluation["watch_rows"].clone())?;
            verify_historical_rows(&f.watch, &rows)?;
            scores.push(details(&rows)?);
            panels.push(rows);
            let train_rows: Vec<Value> =
                replica_v3::binary::from_value(evaluation["train_exposure_panel"].clone())?;
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
        let comparison = record!({"new_updates":n,"control":scores[0],"treatment":scores[1],"control_train16":train_scores[0],"treatment_train16":train_scores[1]});
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
        &record!({"status":"VERIFIED_COMPARISON","factor":factor,"fixture_hash":fixture_hash,"source_ids":policies.iter().map(|p|&p["source_id"]).collect::<Vec<_>>(),"binary_hashes":policies.iter().map(|p|&p["binary_hash"]).collect::<Vec<_>>(),"same_tape":!group,"same_optimizer_clocks":true,"control_reused":group,"comparisons":comparisons,"runs":results,"recorded_small_optimizer_updates":traces.iter().map(Vec::len).sum::<usize>(),"new_small_optimizer_updates":0,"model_calls":0,"final_heldout":false,"goal1_ready":false}),
    )?;
    println!("schedule comparison saved: {}", output.display());
    Ok(())
}
fn trace(path: &Path) -> Result<Vec<Value>> {
    Ok(replica_v3::binary::value_records_from_slice(&neural::read_bounded(path, 16 * 1024 * 1024)?)?)
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
        || result["policy_sha256"] != file_hash(&directory.join("policy.r3b"))?
        || result["checkpoint_file_sha256"] != file_hash(&directory.join("final"))?
        || result["final_evaluation_sha256"] != file_hash(&directory.join("eval-050.r3b"))?
        || result["cumulative_model_step"]
            != trace.last().ok_or_else(invalid)?["cumulative_model_step"]
        || policy["tape_hash"] != digest(&policy["tape"])?
        || policy["source_id"].as_str().is_none_or(|s| s.len() != 64)
        || policy["binary_hash"].as_str().is_none_or(|s| s.len() != 64)
    {
        return Err(invalid());
    }
    let evaluation = read_metadata(&directory.join("eval-050.r3b"))?;
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
        || record!(state.step) != result["cumulative_model_step"]
        || replica_v3::binary::to_value(&state.config)? != policy["config"]
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
        record!({"verified":true,"checkpoint_file_sha256":result["checkpoint_file_sha256"],"policy_sha256":result["policy_sha256"],"final_evaluation_sha256":result["final_evaluation_sha256"]}),
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
    summary["arm_terminals"] = record!(arms);
    summary["provenance_verified"] = record!(provenance_verified);
    summary["final_evaluation_complete"] = record!(eligible);
    summary["comparison_eligible"] = record!(eligible);
    summary["candidate_eligible"] = record!(eligible && summary["candidate_eligible"] == true);
    summary["control"] = budget.receipt();
    save(&output.join("summary.r3b"), &summary)?;
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
            read_metadata(&dir.join("result.r3b")).unwrap_or_else(
                |e| record!({"reason":"UNKNOWN_UNVERIFIED","read_error":e.to_string()}),
            )
        })
        .collect();
    if !arms.iter().all(arm_terminal_eligible) {
        std::fs::create_dir(output)?;
        return finish_close(
            output,
            record!({"stage":"close_preflight","candidate_eligible":false,"model_calls":0,"status":"INELIGIBLE_OR_UNVERIFIED_ARM"}),
            &arms,
            false,
            budget,
        );
    }
    let f = load(fixture)?;
    let cp = read_metadata(&control.join("policy.r3b"))?;
    let wp = read_metadata(&treatment.join("policy.r3b"))?;
    let c = trace(&control.join("trace.r3rows"))?;
    let w = trace(&treatment.join("trace.r3rows"))?;
    let mut config = cp["config"].clone();
    config["first_target_weight"] = record!(1.);
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
            record!({"stage":"close_provenance","candidate_eligible":false,"model_calls":0,"error":error.to_string()}),
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
        let replay_path = output.join(format!("{name}-fresh.r3rows"));
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
        let scored = read_metadata(&directory.join("eval-050.r3b"))?;
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
            let ids: Vec<u32> = replica_v3::binary::from_value(row["raw_tokens"].clone())?;
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
        save(&output.join(format!("{name}-direct.r3b")), &direct)?;
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
        stats["components"] = record!(fields);
        reports.push(record!({"arm":name,"result":read_metadata(&directory.join("result.r3b"))?,"fresh_watch":stats,"physical_hash":file_hash(&checkpoint)?,"model_tensor_content_digest":loaded.model.weights_content_id()?,"legacy_architecture_weight_hash":loaded.model.weight_hash()?,"clock":loaded.manifest.training,"fresh_outputs_identical":true}));
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
        save(&output.join(format!("failed-{kind}-direct.r3b")), &direct)?;
        budget.check("close_failure_returned")?;
        let ids: Vec<u32> = replica_v3::binary::from_value(direct["raw_tokens"].clone())?;
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
    let base = read_metadata(&control.join("eval-000.r3b"))?["watch"]["exact_matches"]
        .as_u64()
        .ok_or_else(|| Error::Corrupt("baseline score".into()))?;
    let mut streak = 0;
    let mut regression = false;
    for n in [10, 25, 50] {
        let evaluation = read_metadata(&control.join(format!("eval-{n:03}.r3b")))?;
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
    let mut summary = record!({"one_factor_actual_trace_verified":true,"optimizer_updates_small":c.len()+w.len(),"same_sample_multiset_and_order":true,"same_clocks_and_lr":true,"reports":reports,"product_worker":worker_checks,"tokenizer_native_legacy_mapping":"PASS","candidate_eligible":eligible,"confirmation":if eligible{"REQUIRED_NOT_RUN"}else{"NOT_RUN_NO_SCREENING_EFFECT"},"regression_within_50":if regression{"REPRODUCED_ON_WATCH"}else{"NOT_REPRODUCED_WITHIN_BUDGET"},"s4_quality":"NOT_EVALUATED_HERE","goal1_ready":false});
    budget.check("close_before_terminal")?;
    save(&output.join("evaluations.r3b"), &summary)?;
    summary["arm_provenance"] = record!(provenance?);
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
            || row["evidence"] != replica_v3::binary::to_value(&e.request.evidence)?
            || row["generated_question"] != e.request.input
            || row["generated_evidence"] != replica_v3::binary::to_value(&e.request.evidence)?
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
    let (manifest, train, _) = data::load_legacy(&f.corpus)?;
    if manifest.train.sha256 != f.train_hash {
        return Err(Error::Corrupt("recount train binding".into()));
    }
    let audit = read_metadata(audit_path)?;
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
        historical.push(record!({"artifact":label,"raw_hash":file_hash(path)?,"score":summarize(&cases)?,"score_interpretation":"SCORE_AGAINST_FROZEN_LABELS","missing_error_generation_receipts":cases.iter().filter(|r|!r["error"].is_null()&&r["generation"].is_null()).count(),"missing_receipt_finish":"UNKNOWN"}));
    }
    let policies = arms
        .map(|p| read_metadata(&p.join("policy.r3b")))
        .into_iter()
        .collect::<Result<Vec<_>>>()?;
    let traces = arms
        .map(|p| trace(&p.join("trace.r3rows")))
        .into_iter()
        .collect::<Result<Vec<_>>>()?;
    let mut expected_w = policies[0]["config"].clone();
    expected_w["first_target_weight"] = record!(1.);
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
        let config: TrainConfig = replica_v3::binary::from_value(policy["config"].clone())?;
        let tape: Vec<(Vec<usize>, u64)> = replica_v3::binary::from_value(policy["tape"].clone())?;
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
            if row["indices"] != record!(indices)
                || row["ids"] != record!(ids)
                || row["sampler_state"] != record!(rng)
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
        let evaluation = read_metadata(&directory.join("eval-050.r3b"))?;
        let scored: Vec<Value> = replica_v3::binary::from_value(evaluation["watch_rows"].clone())?;
        verify_historical_rows(&f.watch, &scored)?;
        let score = summarize(&scored)?;
        for k in ["denominator", "exact_matches", "generation_failures"] {
            if score[k] != evaluation["watch"][k] {
                return Err(Error::Corrupt("historical watch recount".into()));
            }
        }
        arm_reports.push(record!({"arm":policy["arm"],"trace_hash":file_hash(&directory.join("trace.r3rows"))?,"historical_updates":trace.len(),"watch":score,"fixture_bound":true,"policy_bound":true}));
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
        &record!({"status":"PASS","binding":binding,"fixture_hash":fixture_hash,"historical":historical,"arms":arm_reports,"data_audit":audit["status"],"label_findings":audit["validation400"]["semantic_findings"],"new_small_optimizer_updates":0,"model_calls":control.generation_calls,"final_heldout":false}),
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
        record!({"id":e.id,"success":success,"direct_worker_parity":true,"worker_error":error.trim(),"raw_bytes":direct["raw_bytes"],"raw_tokens":direct["raw_tokens"],"finish_reason":direct["finish_reason"]}),
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
    // belongs to result.r3b, not a new artifact schema or an invented success state.
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
    let mut evaluation = record!({"watch":score,"watch_rows":rows,"train_exposure_panel":train_rows});
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
    record!({"reason":reason,"observed_conditions":control.observed,"checkpoint_saved":saved.is_ok(),"checkpoint_save_status_reason":saved_reason,"save_error":saved.err().map(|e|e.to_string()),
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
    checkpoint::ResumeBinding::require_default(
        l.manifest
            .training
            .as_ref()
            .ok_or_else(|| Error::Invalid("resume state absent".into()))?,
        &l.tokenizer,
    )?;
    control.check("arm_loaded")?;
    if l.model.weight_hash()? != f.registry["U2_POLICY_START"]["model_content_hash"]
        || file_hash(&f.start)? != f.registry["U2_POLICY_START"]["physical_hash"]
    {
        return Err(Error::Corrupt("arm parent changed".into()));
    }
    let (manifest, episodes, _) = data::load_legacy(&f.corpus)?;
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
        let parent: TrainingState = replica_v3::binary::from_value(
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
        &output.join("policy.r3b"),
        &record!({"arm":arm,"parent":f.registry["U2_POLICY_START"],"fixture_hash":file_hash(fixture)?,"source_id":source_id,"binary_hash":file_hash(&std::env::current_exe()?)?,"tape":tape,"tape_hash":digest(&tape)?,"config":c,"max_new_updates":max_updates,"max_input_tokens":max_input_tokens,"max_target_tokens":max_target_tokens,"max_seconds":900,"max_rss_bytes":17179869184u64,"clock_policy":if matches!(arm,"L"|"B"){"moments + cumulative optimizer clock retained; parent endpoint LR, no warmup, same cosine horizon"}else{"moments + cumulative optimizer clock retained; saved U2 schedule unchanged"}}),
    )?;
    let mut log = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(output.join("trace.r3rows"))?;
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
                evaluation["new_updates"] = record!(n);
                evaluation["model_step"] = record!(state.step);
                evaluation["model_content_hash"] = record!(l.model.weight_hash()?);
                evaluation["new_error_cases"] = record!(new_errors);
                evaluation["bad_streak"] = record!(bad_streak);
                evaluation["exposure_counts"] = record!(
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
                    evaluation["final_evaluation_complete"] = record!(false);
                    evaluation["comparison_eligible"] = record!(false);
                    evaluation["terminal_reason"] = record!(control.stop);
                }
                save(&output.join(format!("eval-{n:03}.r3b")), &evaluation)?;
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
            let stats: BTreeMap<_,_>=groups.into_iter().map(|(k,v)|(k,record!({"gradient_norm":v[0].sqrt(),"update_norm":v[1].sqrt(),"weight_norm":v[2].sqrt(),"update_to_weight":v[1].sqrt()/v[2].sqrt().max(1e-30)}))).collect();
            let row = record!({"arm":arm,"new_update":n+1,"cumulative_model_step":state.step,"optimizer_step":state.step,"schedule_step":state.step-c.budget_start_step,"lr":c.learning_rate(state.step),"indices":indices,"ids":indices.iter().map(|&i|&episodes[i].id).collect::<Vec<_>>(),"sampler_state":sampler,"input_tokens":b.tokens,"target_tokens":targets,"ce":ce,"objective":objective,"first_target_weight":c.first_target_weight,"grad_norm":norm,"update_norm":delta,"parameter_groups":stats,"elapsed_seconds":control.start.elapsed().as_secs_f64(),"rss_kib":control.last_rss_kib});
            replica_v3::binary::write_value_record(&mut log, &row)?;
            log.flush()?;
            control.stop_result()?;
            if n + 1 == 20 && arm == "C" {
                let hash = l.model.weight_hash()?;
                let expected = &f.registry["U2_AFTER_20"]["model_content_hash"];
                save(
                    &output.join("control-step20-parity.r3b"),
                    &record!({"actual":hash,"recorded_u2_after20":expected,"equal":hash==*expected}),
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
    let info = record!({"arm":arm,"new_updates":state.step-start_step,"cumulative_model_step":state.step,"additional_input_tokens":state.consumed_tokens-start_input,"additional_target_tokens":state.target_tokens-start_targets,"model_content_hash":l.model.weight_hash()?,"elapsed_seconds":control.start.elapsed().as_secs_f64(),"error":outcome.as_ref().err().map(ToString::to_string),"goal1_ready":false,"last_evaluation":last_evaluation});
    result
        .as_object_mut()
        .unwrap()
        .extend(info.as_object().unwrap().clone());
    result["policy_sha256"] = record!(file_hash(&output.join("policy.r3b"))?);
    if result["checkpoint_saved"] == true {
        result["checkpoint_file_sha256"] = record!(file_hash(&output.join("final"))?);
    }
    let final_eval = output.join(format!("eval-{:03}.r3b", state.step - start_step));
    if final_eval.is_file() {
        result["final_evaluation_sha256"] = record!(file_hash(&final_eval)?);
    }
    save(&output.join("result.r3b"), &result)?;
    log.sync_all()?;
    println!("{result}");
    outcome.and(control.stop_result())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn citation_fidelity_inference_budget() {
        let gib=1024*1024;
        let mut c=RunControl::new(Arc::new(AtomicBool::new(false)),Duration::from_secs(10),16*gib).unwrap();
        c.restrict_rss(12*gib);c.restrict_rss(16*gib);
        assert!(c.check_at(c.start,Ok(12*gib)).is_ok());
        assert!(c.check_at(c.start,Ok(12*gib+1)).is_err());
        assert_eq!(c.receipt()["terminal_reason"],"RESOURCE_LIMIT");
        assert_eq!(c.receipt()["generation_calls"],0);assert_eq!(c.receipt()["teacher_calls"],0);
    }
    fn state_fixture(root: &Path) -> (PathBuf, PathBuf) {
        let mut l = repair_loaded();
        let ordinary: Vec<_> = (0..400)
            .map(|i| {
                let mut e = repair_episode(&format!("ordinary/{i}"));
                if i >= 336 {
                    e.family = format!("copy/{i}");
                }
                e.request.limits.max_tokens = 2;
                e
            })
            .collect();
        let mut f = repair_frozen(root, &ordinary, &l);
        f.watch = ordinary[..32].to_vec();
        let baseline = root.join("baseline");
        std::fs::create_dir(&baseline).unwrap();
        save(&baseline.join("frozen.r3b"), &f).unwrap();
        let dev: Vec<_> = (0..256)
            .map(|i| {
                let mut e = repair_episode(&format!("dev/{i}"));
                e.request.limits.max_tokens = 2;
                e
            })
            .collect();
        let corpus = root.join("h3");
        let m = repair_corpus(&corpus, &dev);
        let cross: Vec<_> = (0..512)
            .map(|i| {
                let mut e = repair_episode(&format!("cross/{i}"));
                e.request.limits.max_tokens = 2;
                e
            })
            .collect();
        let a0 = root.join("a0");
        std::fs::create_dir(&a0).unwrap();
        save(&a0.join("cross-development.r3b"), &cross).unwrap();
        save(&a0.join("summary.r3b"),&record!({"baseline":baseline,"baseline_hash":file_hash(&baseline.join("frozen.r3b")).unwrap(),
            "corpus":corpus,"train_hash":m.train.sha256,"dev_hash":m.validation.sha256,"cross":{"file_sha256":file_hash(&a0.join("cross-development.r3b")).unwrap()}})).unwrap();
        let mut state = TrainingState {
            resume_binding: None,
            contrast16: false,
            parent_checkpoint_hash: None,
            config: TrainConfig {
                seq_len: 32,
                max_steps: 512,
                warmup: 0,
                ..Default::default()
            },
            step: 0,
            consumed_tokens: 0,
            target_tokens: 0,
            sampler_state: 17,
            corpus_hash: m.train.sha256.clone(),
            validation_hash: m.validation.sha256.clone(),
            previous_corpora: vec![l.tokenizer.train_hash.clone()],
            initial_weight_hash: l.manifest.initial_weight_hash.clone(),
            train_loss: None,
            validation_loss: None,
        };
        let adam = Adam::new(&l.model.vars).unwrap();
        let parent = root.join("parent");
        state.resume_binding = Some(checkpoint::ResumeBinding::default_for(&state, &l.tokenizer));
        save_arm(&mut l, &state, &adam, &parent, "RECOVERY_SCREENING").unwrap();
        let pair = root.join("pair");
        let parent_receipt = root.join("parent-result.r3b");
        save(&parent_receipt, &record!({"fixture":true})).unwrap();
        std::fs::create_dir(&pair).unwrap();
        let mut hashes = Vec::new();
        for arm in ["C", "L"] {
            let dir = pair.join(arm);
            std::fs::create_dir(&dir).unwrap();
            let p = record!({"node":"A1","run_id":pair,"contract":PROGRESS_CONTRACT,"arm":arm,"a0":a0,"a0_hash":file_hash(&a0.join("summary.r3b")).unwrap(),
                "corpus":corpus,"train_hash":m.train.sha256,"dev_hash":m.validation.sha256,"cross_hash":file_hash(&a0.join("cross-development.r3b")).unwrap(),
                "parent":parent,"parent_sha256":file_hash(&parent).unwrap(),"parent_receipt":parent_receipt,"parent_receipt_sha256":file_hash(&parent_receipt).unwrap(),
                "initial_model_hash":l.model.weight_hash().unwrap(),"initial_adam_hash":optimizer_hash(&adam.moments).unwrap(),"initial_state":state,
                "tokenizer_hash":l.tokenizer.semantic_id(),"source_digest":neural::hash(b"synthetic fixture source"),"lr_policy":arm,"lr_offset":0,
                "tape":(0..512).map(|_|record!([[0],17])).collect::<Vec<_>>(),"denominators":vec![[1,1];512],"tape_hash":"fixture","denominators_hash":"fixture",
                "anchor_floor":178,"baseline_dev":0,"baseline_watch":0,"baseline_errors":0});
            save(&dir.join("policy.r3b"), &p).unwrap();
            hashes.push(file_hash(&dir.join("policy.r3b")).unwrap());
            let inputs = load_verified_inputs(&a0, Some(&p)).unwrap();
            let segment = dir.join("segment-00-0000");
            std::fs::create_dir(&segment).unwrap();
            let mut receipts = record!({});
            let mut last = Value::Null;
            for n in [0, 128, 256, 512] {
                let dr: Vec<_> = dev.iter().map(|e| state_row(&l, e)).collect();
                let wr: Vec<_> = f.watch.iter().map(|e| state_row(&l, e)).collect();
                last = record!({"new_updates":n,"model_content_hash":l.model.weight_hash().unwrap(),"final_evaluation_complete":true,
                    "dev":skill_score(&dr).unwrap(),"watch":skill_score(&wr).unwrap(),"dev_rows":dr,"watch_rows":wr});
                bind_progress_evaluation(&p, &mut last, &inputs, &l).unwrap();
                if n > 0 {
                    let (key, evaluation_digest, panel_digest) =
                        evaluation_identity(&p, &last).unwrap();
                    last["decision"] = replica_v3::binary::to_value(EvaluationDecision {
                        key,
                        evaluation_digest,
                        panel_digest,
                        before: [0; 3],
                        after: None,
                        quality_stop: None,
                        applied: None,
                    })
                    .unwrap();
                    reconcile_evaluation_decision(
                        &p,
                        n,
                        &l.model.weight_hash().unwrap(),
                        &mut last,
                        &mut [0; 3],
                        &mut repair_control(),
                    )
                    .unwrap();
                }
                record_bound_panel(
                    &segment.join(format!("eval-{n:04}.r3b")),
                    &last,
                    &mut receipts,
                )
                .unwrap();
                if n > 0 {
                    let mut s = state.clone();
                    s.step = n;
                    save_arm(
                        &mut l,
                        &s,
                        &adam,
                        &segment.join(format!("step-{n:04}")),
                        "RECOVERY_SCREENING",
                    )
                    .unwrap();
                }
            }
            let mut scores = record!({});
            for (id, file, cases, hash) in [
                (
                    "cross",
                    "cross.r3b",
                    cross.as_slice(),
                    &inputs.hashes["cross"],
                ),
                (
                    "ordinary",
                    "ordinary400.r3b",
                    ordinary.as_slice(),
                    &inputs.hashes["ordinary"],
                ),
            ] {
                let rows: Vec<_> = cases.iter().map(|e| state_row(&l, e)).collect();
                let binding = bind_progress_panel(&p, id, cases, hash, 512, &rows, &l).unwrap();
                let score = if id == "cross" {
                    progress_copy_score(&rows, 512).unwrap()
                } else {
                    summarize(&rows).unwrap()
                };
                record_bound_panel(&segment.join(file),&record!({"policy":"normal_greedy_v1","rows":rows,"score":score,"bindings":{id:binding}}),&mut receipts).unwrap();
                scores[id] = score;
            }
            let mut trace = Vec::new();
            for n in 0..512 {
                replica_v3::binary::write_value_record(&mut trace,&record!({"new_update":n+1,"indices":[0],"sampler_state":17,"input_tokens":1,"target_tokens":1,
                "ids":["train/0"],"optimizer_step":n+1,"lr_bits":progress_lr(arm,n+1).unwrap().to_bits(),"ce":0.,"objective":0.,"gradient_norm":0.,"update_norm":0.})).unwrap();
            }
            std::fs::write(segment.join("trace.r3rows"), trace).unwrap();
            let mut final_state = state.clone();
            final_state.step = 512;
            save_arm(
                &mut l,
                &final_state,
                &adam,
                &segment.join("final"),
                "SCREENING_BUDGET_REACHED",
            )
            .unwrap();
            let r = record!({"arm":arm,"policy_sha256":hashes.last().unwrap(),"checkpoint_file_sha256":file_hash(&segment.join("final")).unwrap(),
                "exposure":{"trace_sha256":file_hash(&segment.join("trace.r3rows")).unwrap()},"segment_updates":512,"new_updates":512,"cumulative_model_step":512,"sampler_state":17,
                "last_evaluation":last,"model_content_hash":l.model.weight_hash().unwrap(),"adam_hash":optimizer_hash(&adam.moments).unwrap(),"cross":scores["cross"],"ordinary":scores["ordinary"],
                "raw_development_gate":false,"candidate_eligible":false,"resume_allowed":false,"reason":"SCREENING_BUDGET_REACHED","comparison_eligible":true,"cleanup_limit_exceeded":false,
                "checkpoint_saved":true,"save_error":null,"final_evaluation_complete":true,"control":{"terminal_reason":"COMPLETED","observed_conditions":[]},"panel_receipts":receipts,
                "evaluation_decision_digest":digest(&last["decision"]).unwrap(),"guard_streaks":[0,0,0]});
            save(&segment.join("result.r3b"), &r).unwrap();
            state.step = 0;
        }
        save(
            &pair.join("pair.r3b"),
            &record!({"node":"A1","arms":["C","L"],"policy_hashes":hashes}),
        )
        .unwrap();
        (a0, pair)
    }
    fn state_row(l: &Loaded, e: &Episode) -> Value {
        // Serialized oracle fixture only: no inference or optimizer, never a model-quality observation.
        let prompt = l
            .tokenizer
            .prepare(
                &e.request,
                l.model.config.context as u32,
                &l.model.config.id().unwrap(),
            )
            .unwrap();
        let tokens = l.tokenizer.encode(e.answer.as_bytes()).unwrap();
        let mut raw = tokens.clone();
        raw.push(EOS);
        record!({"id":e.id,"scene":scene(e),"family":e.family,"category":e.category,"question":e.request.input,"generated_question":e.request.input,
            "evidence":e.request.evidence,"generated_evidence":e.request.evidence,"expected":e.answer,"actual":e.answer,"raw_tokens":raw,"raw_bytes":bytes_receipt(&l.tokenizer,&tokens),
            "raw_generated_count":raw.len(),"eos_index":raw.len()-1,"provided":prompt.provided,"excluded":prompt.excluded,"prompt_length":prompt.token_ids.len(),
            "request_digest":digest(&e.request).unwrap(),"prompt_digest":digest(&prompt.token_ids).unwrap(),"native_prompt_digest":prompt.token_digest,
            "generation":{"tokens":tokens,"generated":raw.len(),"finish":"stop"},"finish_reason":"stop","generation_started":true,"generation_completed":true,
            "error":null,"error_class":null,"interruption":null,"whitespace_only":false,"components":components(Some(&e.answer),&e.answer,&prompt.provided),"exact_match":true})
    }
    #[test]
    fn state_data_frozen_all_entries_reject_content_and_partition_changes() {
        for mutation in 0..8 {
            let dir = tempfile::tempdir().unwrap();
            let (a0, pair) = state_fixture(dir.path());
            let p = read_metadata(&pair.join("C/policy.r3b")).unwrap();
            let before = load_verified_inputs(&a0, Some(&p)).unwrap();
            assert_eq!(
                (before.ordinary.len(), before.dev.len(), before.cross.len()),
                (400, 256, 512)
            );
            if mutation < 4 {
                let mut cases = before.ordinary.clone();
                match mutation {
                    0 => cases[0].request.input.push('x'),
                    1 => cases[0].answer.push('x'),
                    2 => cases[0].request.evidence.items.push(replica_v3::binary::from_value(record!({
                        "event_id":1,"original_excerpt":"changed original evidence","source":"manual","recorded_at":1,"observed_at":1,
                        "version_status":"current","retrieval_reason":"lexical","relation_path":[],"excerpt_truncated":false
                    })).unwrap()),
                    _ => {
                        cases[0].family = "copy/changed".into();
                        cases[336].family = "qa/changed".into();
                    }
                }
                repair_corpus(&before.frozen.corpus, &cases);
                assert!(data::load_legacy(&before.frozen.corpus).is_ok());
                let baseline = progress_path(&before.a0, "baseline").unwrap();
                let missing = dir.path().join("absent");
                let out = dir.path().join("never-created");
                let err = progress_baseline(
                    [&baseline, &missing, &missing, &missing, &missing],
                    &out,
                    1,
                    &mut repair_control(),
                )
                .unwrap_err();
                assert!(err.to_string().contains("FROZEN_INPUT_MISMATCH"), "{err}");
                assert!(!out.exists());
            } else if mutation == 4 {
                let path = progress_path(&before.a0, "baseline")
                    .unwrap()
                    .join("frozen.r3b");
                let mut f = read_metadata(&path).unwrap();
                f["registry"]["changed"] = record!(true);
                std::fs::write(path, replica_v3::binary::to_vec(&f).unwrap()).unwrap();
            } else if mutation == 5 {
                let mut cases = before.dev.clone();
                cases[0].request.input.push('x');
                repair_corpus(&progress_path(&p, "corpus").unwrap(), &cases);
            } else {
                let mut cross = before.cross.clone();
                if mutation == 6 {
                    cross.swap(0, 1);
                } else {
                    cross.pop();
                }
                std::fs::write(
                    a0.join("cross-development.r3b"),
                    replica_v3::binary::to_vec(&cross).unwrap(),
                )
                .unwrap();
            }
            assert!(load_verified_inputs(&a0, Some(&p)).is_err());
            let mut c = repair_control();
            let err = progress_arm(&pair, "C", None, &mut c).unwrap_err();
            assert!(err.to_string().contains("FROZEN_INPUT_MISMATCH"), "{err}");
            assert_eq!(c.generation_calls, 0);
            assert!(progress_close(&pair, &mut repair_control()).is_err());
            assert!(!pair.join("comparison.r3b").exists());
        }
        println!(
            "T-D01/02/03/05/06/07: actual entry and owned-input fixtures; SMALL/TINY/scalar updates0"
        );
    }
    #[test]
    fn state_data_candidate_rejects_ordinary_length_end_despite_high_qa() {
        let dir = tempfile::tempdir().unwrap();
        let (a0, pair) = state_fixture(dir.path());
        let p = read_metadata(&pair.join("C/policy.r3b")).unwrap();
        let inputs = load_verified_inputs(&a0, Some(&p)).unwrap();
        let segment = pair.join("C/segment-00-0000");
        let l = checkpoint::load(&segment.join("final"), Device::Cpu, true).unwrap();
        let path = segment.join("ordinary400.r3b");
        let mut panel = read_metadata(&path).unwrap();
        let tokens = l.tokenizer.encode(b"xx").unwrap();
        assert_eq!(tokens.len(), 2);
        let row = &mut panel["rows"][0];
        row["actual"] = record!("xx");
        row["raw_tokens"] = record!(tokens);
        row["raw_bytes"] = bytes_receipt(&l.tokenizer, &tokens);
        row["raw_generated_count"] = record!(2);
        row["eos_index"] = Value::Null;
        row["generation"] = record!({"tokens":tokens,"generated":2,"finish":"length"});
        row["finish_reason"] = record!("length");
        row["exact_match"] = record!(false);
        row["components"] = components(Some("xx"), &inputs.ordinary[0].answer, &[]);
        let rows = panel["rows"].as_array().unwrap();
        let score = summarize(rows).unwrap();
        assert_eq!(score["qa"], record!([335, 336]));
        assert_eq!(score["generation_failures"], 0); // Length end needs the full generation check.
        assert_eq!(skill_score(rows).unwrap()["generation_error_cases"], 1);
        let binding = bind_progress_panel(
            &p,
            "ordinary",
            &inputs.ordinary,
            &inputs.hashes["ordinary"],
            512,
            rows,
            &l,
        )
        .unwrap();
        panel["score"] = score.clone();
        panel["bindings"]["ordinary"] = binding;
        std::fs::write(&path, replica_v3::binary::to_vec(&panel).unwrap()).unwrap();
        let result_path = segment.join("result.r3b");
        let mut result = read_metadata(&result_path).unwrap();
        result["ordinary"] = score;
        result["panel_receipts"]["ordinary400.r3b"] =
            record!({"sha256":file_hash(&path).unwrap(),"bindings":panel["bindings"]});
        result["candidate_eligible"] = record!(true);
        std::fs::write(&result_path, replica_v3::binary::to_vec(&result).unwrap()).unwrap();
        let error = progress_close(&pair, &mut repair_control()).unwrap_err();
        assert!(
            error.to_string().contains("non-normal final generation"),
            "{error}"
        );
        assert!(!pair.join("comparison.r3b").exists());
        // A fully recorded wrong/length-ended model output still belongs in the denominator.
        result["candidate_eligible"] = record!(false);
        std::fs::write(&result_path, replica_v3::binary::to_vec(&result).unwrap()).unwrap();
        progress_close(&pair, &mut repair_control()).unwrap();
        let comparison = read_metadata(&pair.join("comparison.r3b")).unwrap();
        assert_eq!(comparison["endpoints"][0]["ordinary"]["denominator"], 400);
        assert_eq!(
            comparison["endpoints"][0]["ordinary"]["qa"],
            record!([335, 336])
        );
        assert_eq!(comparison["raw_development_gate"], false);
        println!(
            "candidate ordinary length-end regression: optimizer/generation/teacher0; QA335/336 is synthetic fixture"
        );
    }
    #[test]
    fn state_data_observation_stop_preserves_raw_binding() {
        let dir = tempfile::tempdir().unwrap();
        let (_, pair) = state_fixture(dir.path());
        // Reuse the TINY fixture's checked lineage; the labels are not optimizer calls.
        for (kind, arm, raw_file, panel) in [
            ("F16", "F", "raw.r3b", "dev"),
            ("N256", "N", "cross.r3b", "cross"),
        ] {
            std::fs::create_dir(pair.join(arm)).unwrap();
            std::fs::copy(
                pair.join("C/policy.r3b"),
                pair.join(arm).join("policy.r3b"),
            )
            .unwrap();
            let output = dir.path().join(kind);
            std::fs::create_dir(&output).unwrap();
            save(&output.join("reaudit.r3b"), &record!({"current_raw_recount":"VERIFIED","mode":"READ_ONLY_REAUDIT",
                "endpoints":[{"arm":arm,"policy_sha256":file_hash(&pair.join(arm).join("policy.r3b")).unwrap(),"segment":pair.join("C/segment-00-0000")}]})).unwrap();
            let mut control = repair_control();
            control.time_boundary = Some("panel_next_case");
            assert!(progress_observe(&pair, &output, kind, &mut control).is_err());
            assert_eq!((control.generation_calls, control.teacher_calls), (0, 0));
            let observed = output.join(kind);
            let registration = read_metadata(&observed.join("registration.r3b")).unwrap();
            let result = read_metadata(&observed.join("result.r3b")).unwrap();
            let raw = read_metadata(&observed.join(raw_file)).unwrap();
            assert_eq!(result["status"], "FAILED_OR_INCOMPLETE");
            assert_eq!(result["candidate_eligible"], false);
            assert_eq!(result["resume_allowed"], false);
            assert_eq!(result["control"]["terminal_reason"], "TIME_BUDGET");
            assert_eq!(
                result["registration_sha256"],
                file_hash(&observed.join("registration.r3b")).unwrap()
            );
            assert_eq!(
                result["panel_receipts"][raw_file]["sha256"],
                file_hash(&observed.join(raw_file)).unwrap()
            );
            assert_eq!(raw["rows"], record!([]));
            assert_eq!(raw["bindings"][panel]["complete"], false);
            assert_eq!(
                raw["bindings"][panel]["evaluator_source"],
                registration["source_sha"]
            );
            assert!(!observed.join("ordinary.r3b").exists());
            if kind == "N256" {
                assert_eq!(registration["maximum_new_generations"], 912);
                assert!(
                    registration["selection"]
                        .as_str()
                        .unwrap()
                        .contains("full frozen CROSS512 and ordinary400")
                );
            }
            // An incomplete observation cannot be silently retried or overwritten.
            assert!(progress_observe(&pair, &output, kind, &mut repair_control()).is_err());
        }
        println!(
            "observation metadata/terminal actual path: SMALL/TINY/scalar optimizer0, generation0, teacher0"
        );
    }
    #[test]
    fn state_data_complete_close_and_raw_negative_fixtures() {
        for mutation in 0..11 {
            let dir = tempfile::tempdir().unwrap();
            let (_, pair) = state_fixture(dir.path());
            if mutation > 0 {
                for arm in ["C", "L"] {
                    let segment = pair.join(arm).join("segment-00-0000");
                    let file = if mutation == 2 {
                        "ordinary400.r3b"
                    } else {
                        "cross.r3b"
                    };
                    let path = segment.join(file);
                    let mut raw = read_metadata(&path).unwrap();
                    match mutation {
                        1 => raw["rows"] = record!([]),
                        2 | 3 => {
                            raw["rows"].as_array_mut().unwrap().pop();
                        }
                        4 => raw["rows"][1] = raw["rows"][0].clone(),
                        5 => raw["rows"][0]["expected"] = record!("different"),
                        6 => raw["rows"].as_array_mut().unwrap().swap(0, 1),
                        7 => raw["rows"][0]["actual"] = record!("wrong"),
                        8 => raw["bindings"]["cross"]["model_hash"] = record!("other model"),
                        9 => raw["rows"][0]["eos_index"] = Value::Null,
                        10 => raw["rows"][0]["error"] = record!("invalid UTF-8"),
                        _ => unreachable!(),
                    }
                    std::fs::write(&path, replica_v3::binary::to_vec(&raw).unwrap()).unwrap();
                    // Even freshly forged file receipts do not override frozen membership or tokenizer checks.
                    if mutation != 9 {
                        let rp = segment.join("result.r3b");
                        let mut result = read_metadata(&rp).unwrap();
                        if mutation == 1 {
                            result["raw_development_gate"] = record!(true);
                        }
                        result["panel_receipts"][file] =
                            record!({"sha256":file_hash(&path).unwrap(),"bindings":raw["bindings"]});
                        std::fs::write(rp, replica_v3::binary::to_vec(&result).unwrap()).unwrap();
                    }
                }
            }
            let result = progress_close(&pair, &mut repair_control());
            if mutation == 0 {
                result.unwrap();
                assert!(pair.join("comparison.r3b").is_file());
            } else {
                assert!(result.is_err(), "mutation{mutation}");
                assert!(!pair.join("comparison.r3b").exists());
            }
        }
        println!(
            "T-P01..06/08/09 final public close fixtures; labels512, actual optimizer/generation calls0"
        );
    }
    #[test]
    fn state_data_renewal_rejects_changed_source_before_generation() {
        for split in ["train", "dev"] {
            let dir = tempfile::tempdir().unwrap();
            let (a0, pair) = state_fixture(dir.path());
            progress_close(&pair, &mut repair_control()).unwrap();
            let p = read_metadata(&pair.join("C/policy.r3b")).unwrap();
            let input = load_verified_inputs(&a0, Some(&p)).unwrap();
            let corpus = progress_path(&p, "corpus").unwrap();
            let mut manifest = input.manifest;
            let mut cases = if split == "train" {
                input.train
            } else {
                input.dev
            };
            cases[0].answer.push('x');
            let bytes = replica_v3::binary::to_vec(&cases).unwrap();
            let target = if split == "train" {
                &mut manifest.train
            } else {
                &mut manifest.validation
            };
            std::fs::write(corpus.join(&target.file), &bytes).unwrap();
            target.sha256 = neural::hash(&bytes);
            target.bytes = bytes.len();
            std::fs::write(
                corpus.join("manifest.r3b"),
                replica_v3::binary::to_vec(&manifest).unwrap(),
            )
            .unwrap();
            assert!(data::load_legacy(&corpus).is_ok());
            let files: BTreeMap<_, _> = [
                "Cargo.lock",
                "src/training.rs",
                "src/quality_recovery.rs",
                "src/neural/transformer.rs",
            ]
            .into_iter()
            .map(|p| (p, file_hash(Path::new(p)).unwrap()))
            .collect();
            let harness = dir.path().join("fixture-harness.r3b");
            save(&harness,&record!({"result":"CHECKED_SCOPE_PASS","source_unchanged":true,"source_files":files})).unwrap();
            let out = dir.path().join("never-generated");
            let mut c = repair_control();
            let error = progress_renewal(&pair, &harness, &out, 1, &mut c).unwrap_err();
            assert!(
                error.to_string().contains("FROZEN_INPUT_MISMATCH"),
                "{error}"
            );
            assert!(!out.exists());
            assert_eq!(c.generation_calls, 0);
        }
    }
    #[test]
    fn state_data_resume_binds_immutable_raw_files() {
        let dir = tempfile::tempdir().unwrap();
        let (_, pair) = state_fixture(dir.path());
        let p = read_metadata(&pair.join("C/policy.r3b")).unwrap();
        let segment = pair.join("C/segment-00-0000");
        let r = read_metadata(&segment.join("result.r3b")).unwrap();
        verify_resume_evaluation_files(&p, &r, std::slice::from_ref(&segment)).unwrap();
        let path = segment.join("eval-0512.r3b");
        let original = std::fs::read(&path).unwrap();
        let mut raw: Value = replica_v3::binary::from_slice(&original).unwrap();
        raw["dev_rows"][0]["actual"] = record!("changed after checkpoint");
        std::fs::write(&path, replica_v3::binary::to_vec(&raw).unwrap()).unwrap();
        assert!(verify_resume_evaluation_files(&p, &r, std::slice::from_ref(&segment)).is_err());
        assert_eq!(r["last_evaluation"]["dev_rows"][0]["actual"], "b");
        let mut changed = p.clone();
        changed["parent_sha256"] = record!("0".repeat(64));
        assert!(load_verified_inputs(&progress_path(&p, "a0").unwrap(), Some(&changed)).is_err());
    }
    #[test]
    fn state_data_legacy_reaudit_sticky_cancel_and_error_denominator() {
        let dir = tempfile::tempdir().unwrap();
        let (_, pair) = state_fixture(dir.path());
        for arm in ["C", "L"] {
            let segment = pair.join(arm).join("segment-00-0000");
            for name in [
                "eval-0000.r3b",
                "eval-0128.r3b",
                "eval-0256.r3b",
                "eval-0512.r3b",
                "cross.r3b",
                "ordinary400.r3b",
            ] {
                let path = segment.join(name);
                let mut raw = read_metadata(&path).unwrap();
                raw.as_object_mut().unwrap().remove("bindings");
                std::fs::write(path, replica_v3::binary::to_vec(&raw).unwrap()).unwrap();
            }
            let path = segment.join("result.r3b");
            let mut r = read_metadata(&path).unwrap();
            r.as_object_mut().unwrap().remove("panel_receipts");
            r["last_evaluation"]
                .as_object_mut()
                .unwrap()
                .remove("bindings");
            r["reason"] = record!("CANCELLED");
            r["comparison_eligible"] = record!(false);
            r["control"] =
                record!({"terminal_reason":"CANCELLED","observed_conditions":["CANCELLED"]});
            std::fs::write(path, replica_v3::binary::to_vec(&r).unwrap()).unwrap();
        }
        let original = file_hash(&pair.join("C/segment-00-0000/result.r3b")).unwrap();
        assert!(progress_close(&pair, &mut repair_control()).is_err());
        assert!(!pair.join("comparison.r3b").exists());
        let out = dir.path().join("reaudit.r3b");
        progress_close_to(&pair, &out, true, &mut repair_control()).unwrap();
        let report = read_metadata(&out).unwrap();
        assert!(
            report["endpoints"][0]["current_reaudit"]["panels"]["cross"]
                .get("verified_rows")
                .is_none()
        );
        assert_eq!(report["selected_index"], Value::Null);
        assert_eq!(report["historical_receipt_binding"], "ABSENT");
        assert_eq!(report["endpoints"][0]["candidate_eligible"], false);
        assert_eq!(report["endpoints"][0]["resume_allowed"], false);
        assert_eq!(
            original,
            file_hash(&pair.join("C/segment-00-0000/result.r3b")).unwrap()
        );
        let l = repair_loaded();
        let mut e = repair_episode("invalid-output/0");
        e.request.limits.max_tokens = 3;
        let mut row = state_row(&l, &e);
        let tokens = l.tokenizer.encode(&[0xea, 0xb0]).unwrap();
        let mut raw = tokens.clone();
        raw.push(EOS);
        let error = l.tokenizer.decode(&tokens).unwrap_err().to_string();
        row["actual"] = Value::Null;
        row["error"] = record!(error);
        row["error_class"] = record!("strict_utf8");
        row["raw_tokens"] = record!(raw);
        row["raw_bytes"] = bytes_receipt(&l.tokenizer, &tokens);
        row["raw_generated_count"] = record!(raw.len());
        row["eos_index"] = record!(raw.len() - 1);
        row["generation"] = record!({"tokens":tokens,"generated":raw.len(),"finish":"stop"});
        row["exact_match"] = record!(false);
        row["components"] = components(None, &e.answer, &[]);
        let spec = PanelSpec {
            id: "dev",
            cases: &[e],
            split_hash: "test",
            policy_hash: "test",
            source: "test",
            model: "test",
            step: 0,
        };
        let score = verify_panel_and_rescore(&spec, &[row], None, &l, true).unwrap();
        assert_eq!(score["denominator"], 1);
        assert_eq!(score["invalid_utf8"], 1);
        assert_eq!(score["exact_matches"], 0);
        println!(
            "T-P07/09/10 sticky cancellation, strict UTF8 denominator, read-only legacy; optimizer/generation0"
        );
    }
    #[test]
    fn state_data_guard_time_after_raw_and_checkpoint() {
        for boundary in ["progress_eval_recorded", "progress_eval_checkpoint"] {
            let dir = tempfile::tempdir().unwrap();
            let p = record!({"run_id":"isolated","baseline_dev":208,"baseline_watch":18,"baseline_errors":2});
            let e = record!({"new_updates":256,"model_content_hash":"fixture-model","final_evaluation_complete":true,
                "dev":{"denominator":256,"exact_matches":180,"generation_error_cases":0},
                "watch":{"denominator":32,"exact_matches":18,"generation_error_cases":0},"dev_rows":[],"watch_rows":[]});
            let mut last = Value::Null;
            let mut streak = [1, 0, 0];
            let mut c = repair_control();
            c.time_boundary = Some(boundary);
            assert!(
                progress_evaluation_boundary(
                    &p,
                    256,
                    "fixture-model",
                    Some(e),
                    &mut last,
                    &mut streak,
                    dir.path(),
                    &mut c,
                    || Ok(())
                )
                .is_err()
            );
            let terminal = finish_arm(&mut c, false, |_| Ok(()));
            let mut resumed = repair_control();
            let result = progress_evaluation_boundary(
                &p,
                256,
                "fixture-model",
                None,
                &mut last,
                &mut streak,
                dir.path(),
                &mut resumed,
                || Ok(()),
            );
            assert!(
                result.is_err(),
                "pending guard lost after {boundary}: {terminal}"
            );
            assert!(resumed.observed.contains(&StopReason::QualityGuard));
            assert_eq!(streak, [2, 0, 0]);
        }
        println!("T-G01/T-G02 mock evaluation labels128/256; actual TINY/scalar/SMALL updates0");
    }
    #[test]
    fn state_data_guard_idempotence_corruption_and_simultaneous_cancel() {
        let dir = tempfile::tempdir().unwrap();
        let p =
            record!({"run_id":"fixture","baseline_dev":208,"baseline_watch":18,"baseline_errors":2});
        let e = record!({"new_updates":128,"model_content_hash":"model","final_evaluation_complete":true,"dev_rows":[],"watch_rows":[],
            "dev":{"denominator":256,"exact_matches":180,"generation_error_cases":0},"watch":{"denominator":32,"exact_matches":18,"generation_error_cases":0}});
        let mut c = repair_control();
        c.time_boundary = Some("progress_eval_recorded");
        let mut last = Value::Null;
        let mut streak = [0; 3];
        assert!(
            progress_evaluation_boundary(
                &p,
                128,
                "model",
                Some(e.clone()),
                &mut last,
                &mut streak,
                dir.path(),
                &mut c,
                || Ok(())
            )
            .is_err()
        );
        let persisted = replica_v3::binary::to_vec(&last).unwrap();
        let mut resumed = repair_control();
        progress_evaluation_boundary(
            &p,
            128,
            "model",
            None,
            &mut last,
            &mut streak,
            dir.path(),
            &mut resumed,
            || Ok(()),
        )
        .unwrap();
        assert_eq!(streak, [1, 0, 0]);
        for _ in 0..2 {
            progress_evaluation_boundary(
                &p,
                128,
                "model",
                None,
                &mut last,
                &mut streak,
                dir.path(),
                &mut resumed,
                || panic!("duplicate checkpoint"),
            )
            .unwrap();
            assert_eq!(streak, [1, 0, 0]);
        }
        for (field, value) in [
            ("final_evaluation_complete", record!(false)),
            ("new_updates", record!(256)),
            ("model_content_hash", record!("wrong")),
            ("dev", record!({})),
        ] {
            let mut bad: Value = replica_v3::binary::from_slice(&persisted).unwrap();
            bad[field] = value;
            assert!(
                progress_evaluation_boundary(
                    &p,
                    128,
                    "model",
                    None,
                    &mut bad,
                    &mut [0; 3],
                    dir.path(),
                    &mut repair_control(),
                    || panic!("invalid checkpoint")
                )
                .is_err()
            );
        }
        let another = tempfile::tempdir().unwrap();
        let mut cancelled = repair_control();
        cancelled.time_boundary = Some("progress_eval_checkpoint");
        cancelled.hook = Some(Box::new(|boundary, cancel| {
            if boundary == "progress_eval_checkpoint" {
                cancel.store(true, Ordering::Relaxed);
            }
        }));
        let mut last = Value::Null;
        let mut streak = [1, 0, 0];
        assert!(
            progress_evaluation_boundary(
                &p,
                128,
                "model",
                Some(e),
                &mut last,
                &mut streak,
                another.path(),
                &mut cancelled,
                || Ok(())
            )
            .is_err()
        );
        let terminal = finish_arm(&mut cancelled, false, |_| Ok(()));
        for reason in [
            StopReason::QualityGuard,
            StopReason::TimeBudget,
            StopReason::Cancelled,
        ] {
            assert!(cancelled.observed.contains(&reason));
        }
        assert!(!progress_time_resume(&terminal));
        println!(
            "T-G03/04/05: repeated resume, corrupt receipts, simultaneous stop; actual updates0"
        );
    }
    #[test]
    fn state_data_time_split_native_optimizer_and_sampler_continuity() {
        let dir = tempfile::tempdir().unwrap();
        let mut l = repair_loaded();
        let c = TrainConfig {
            seq_len: 32,
            warmup: 0,
            max_steps: 100,
            microbatch: 1,
            accumulation: 1,
            ..Default::default()
        };
        let framed = samples(&[repair_episode("tiny-boundary")], &l.tokenizer, 32).unwrap();
        let batch = batch(&framed, &[0], &Device::Cpu).unwrap();
        let update = |l: &Loaded, adam: &mut Adam, step| {
            let (_, loss, _) = response_loss(
                &l.model.forward(&batch.input, Some(&batch.valid)).unwrap(),
                &batch,
                1.,
            )
            .unwrap();
            let g = loss.backward().unwrap();
            let grads = l
                .model
                .vars
                .iter()
                .map(|(k, v)| (k.clone(), g.get(v).unwrap().detach()))
                .collect();
            adam.step_constant(&l.model.vars, &grads, &c, step, 3e-5)
                .unwrap();
        };
        let mut adam = Adam::new(&l.model.vars).unwrap();
        update(&l, &mut adam, 18);
        l.model.refresh_identity().unwrap();
        let state = TrainingState {
            resume_binding: None,
            contrast16: false,
            parent_checkpoint_hash: None,
            config: c.clone(),
            step: 18,
            consumed_tokens: 100,
            target_tokens: 20,
            sampler_state: 987,
            corpus_hash: l.tokenizer.train_hash.clone(),
            validation_hash: "3".repeat(64),
            previous_corpora: vec![],
            initial_weight_hash: l.manifest.initial_weight_hash.clone(),
            train_loss: None,
            validation_loss: None,
        };
        let p = record!({"run_id":"tiny-split","baseline_dev":208,"baseline_watch":18,"baseline_errors":2});
        let model = l.model.weight_hash().unwrap();
        let e = record!({"new_updates":128,"model_content_hash":model,"final_evaluation_complete":true,"dev_rows":[],"watch_rows":[],
            "dev":{"denominator":256,"exact_matches":180,"generation_error_cases":0},"watch":{"denominator":32,"exact_matches":18,"generation_error_cases":0}});
        let mut control = repair_control();
        control.time_boundary = Some("progress_eval_recorded");
        let mut last = Value::Null;
        let mut streak = [0; 3];
        assert!(
            progress_evaluation_boundary(
                &p,
                128,
                &model,
                Some(e),
                &mut last,
                &mut streak,
                dir.path(),
                &mut control,
                || panic!("time interrupted before evaluation checkpoint")
            )
            .is_err()
        );
        let native = dir.path().join("final");
        let terminal = finish_arm(&mut control, false, |reason| {
            save_arm(&mut l, &state, &adam, &native, reason)
        });
        assert_eq!(terminal["reason"], "TIME_BUDGET");
        let mut restored = checkpoint::load(&native, Device::Cpu, true).unwrap();
        let mut restored_adam = Adam {
            moments: std::mem::take(&mut restored.optimizer),
        };
        let mut resumed = repair_control();
        progress_evaluation_boundary(
            &p,
            128,
            &restored.model.weight_hash().unwrap(),
            None,
            &mut last,
            &mut streak,
            dir.path(),
            &mut resumed,
            || {
                save_arm(
                    &mut restored,
                    &state,
                    &restored_adam,
                    &dir.path().join("evaluation-native"),
                    "RECOVERY_SCREENING",
                )
            },
        )
        .unwrap();
        for _ in 0..2 {
            progress_evaluation_boundary(
                &p,
                128,
                &restored.model.weight_hash().unwrap(),
                None,
                &mut last,
                &mut streak,
                dir.path(),
                &mut resumed,
                || panic!("double application"),
            )
            .unwrap();
        }
        assert_eq!(streak, [1, 0, 0]);
        update(&l, &mut adam, 19);
        update(&restored, &mut restored_adam, 19);
        assert_eq!(
            l.model.weight_hash().unwrap(),
            restored.model.weight_hash().unwrap()
        );
        assert_eq!(
            optimizer_hash(&adam.moments).unwrap(),
            optimizer_hash(&restored_adam.moments).unwrap()
        );
        let mut a = Rng {
            state: state.sampler_state,
        };
        let mut b = Rng {
            state: restored.manifest.training.unwrap().sampler_state,
        };
        assert_eq!(a.next_u64(), b.next_u64());
        println!(
            "T-G06 actual_TINY_optimizer_calls=3 SMALL=0; evaluation label128, initial mock native clock17; time-split model/Adam/sampler equal"
        );
    }
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
    fn progress_cross_capacity_and_independent_serialized_validation() {
        let (cases, meta) = data::crossed_copy_development(&[], 971031).unwrap();
        assert_eq!(cases.len(), 512);
        assert_eq!(meta["bases"], 128);
        let mut l = repair_loaded();
        let docs: Vec<_> = cases
            .iter()
            .map(|e| replica_v3::binary::to_vec(e).unwrap())
            .collect();
        l.tokenizer = ByteBpe::train(&docs, &neural::hash(b"cross test only"), 801).unwrap();
        assert_eq!(
            verify_cross_development(&cases, &[], &l).unwrap()["status"],
            "SERIALIZED_INPUT_VERIFIED"
        );
        let again = data::crossed_copy_development(&[], 971031).unwrap().0;
        assert_eq!(digest(&cases).unwrap(), digest(&again).unwrap());
        for change in 0..4 {
            let mut bad = cases.clone();
            match change {
                0 => bad[0].answer = "invented answer".into(),
                1 => bad[0].request.evidence.items[0].version_status = "historical".into(),
                2 => bad[0].request.input = "다른 대상".into(),
                _ => {
                    let additional = bad[1].request.evidence.items[0].clone();
                    bad[0].request.evidence.items.push(additional);
                }
            }
            assert!(verify_cross_development(&bad, &[], &l).is_err());
        }
        assert!(verify_cross_development(&cases, &cases[..1], &l).is_err());
        let mut used = Vec::new();
        for prefix in ["장치", "설비", "센서", "장비"] {
            for digit in 0..10 {
                let mut e = cases[0].clone();
                e.binding = format!("{prefix}{digit}/구역1/직진");
                used.push(e);
            }
        }
        let (none, capacity) = data::crossed_copy_development(&used, 971031).unwrap();
        assert!(none.is_empty());
        assert_eq!(capacity["status"], "CAPACITY");
        assert_eq!(capacity["one_digit_unseen_unreserved"], record!([]));
    }
    #[test]
    fn progress_metric_denominator_teacher_and_precancel() {
        let row = record!({"id":"fixture/0","scene":"fixture","family":"cross/H3/digits-1/kind-0/pattern-0/view-0",
            "actual":"센서1의 구역1 이동 지시는 직진이다. [event:1]","expected":"센서1의 구역1 이동 지시는 직진이다. [event:1]",
            "exact_match":true,"error":null,"generation_completed":true,"generation":{"finish":"stop","generated":10},"finish_reason":"stop","eos_index":9,"interruption":null,
            "components":{"entity":true,"citation_exact":true},"teacher_forced_diagnostic_after_generation":{"teacher_forced_correct_tokens":10,"target_tokens_including_eos":10}});
        let mut rows = vec![row; 512];
        for (i, row) in rows.iter_mut().enumerate() {
            row["id"] = record!(format!("fixture/{}/{i}", i / 4));
            row["scene"] = record!(format!("fixture/{}", i / 4));
        }
        assert_eq!(progress_copy_score(&rows, 512).unwrap()["skill_pass"], true);
        assert_eq!(
            progress_copy_score(&rows[..511], 512).unwrap()["skill_pass"],
            false
        );
        rows[0]["error"] = record!("UTF-8");
        rows[0]["exact_match"] = record!(false);
        let score = progress_copy_score(&rows, 512).unwrap();
        assert_eq!(score["denominator"], 512);
        assert_eq!(score["skill_pass"], false);
        assert_eq!(
            progress_teacher_relation(&rows)["teacher_all_correct_by_free_exact_false_true"],
            record!([[0, 0], [1, 511]])
        );
        let dir = tempfile::tempdir().unwrap();
        let out = dir.path().join("unused");
        let mut c = repair_control();
        c.cancel.store(true, Ordering::Relaxed);
        assert!(progress_baseline([dir.path(); 5], &out, 1, &mut c).is_err());
        assert!(!out.exists());
        assert_eq!(c.generation_calls, 0);
    }
    #[test]
    fn skill_explicit_renewal_preserves_stop_and_rejects_other_terminal() {
        let previous = record!({"reason":"QUALITY_GUARD","control":{"terminal_reason":"QUALITY_GUARD","observed_conditions":["QUALITY_GUARD"]},
            "checkpoint_saved":true,"comparison_eligible":false,"candidate_eligible":false,"resume_allowed":false,
            "save_error":null,"cleanup_limit_exceeded":false,"new_updates":128,"bad_streak":0,
            "last_evaluation":{"final_evaluation_complete":true,"new_error_ids":["case/new-invalid"]}});
        let before = previous.clone();
        assert!(quality_renewal_eligible(&previous, &record!({}), false));
        assert_eq!(before, previous);
        for reason in [
            "CANCELLED",
            "TIME_BUDGET",
            "RESOURCE_LIMIT",
            "INTEGRITY_FAIL",
            "COMPLETED",
        ] {
            let mut bad = previous.clone();
            bad["reason"] = record!(reason);
            assert!(!quality_renewal_eligible(&bad, &record!({}), false));
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
            assert!(!quality_renewal_eligible(&bad, &record!({}), false), "{key}");
        }
        for (key, value) in [
            ("save_error", record!("disk error")),
            ("cleanup_limit_exceeded", record!(true)),
            ("bad_streak", record!(2)),
            ("new_updates", record!(512)),
        ] {
            let mut bad = previous.clone();
            bad[key] = value;
            assert!(!quality_renewal_eligible(&bad, &record!({}), false), "{key}");
        }
        assert!(!quality_renewal_eligible(
            &previous,
            &record!({"renewal":{}}),
            false
        ));
        assert!(!quality_renewal_eligible(&previous, &record!({}), true));
        let mut at512 = previous.clone();
        at512["new_updates"] = record!(512);
        assert!(quality_renewal_eligible(
            &at512,
            &record!({"renewal":{}}),
            true
        ));
        assert!(!quality_renewal_eligible(
            &at512,
            &record!({"finish_copy_budget":true}),
            true
        ));
        at512["new_updates"] = record!(1024);
        assert!(!quality_renewal_eligible(&at512, &record!({}), true));
    }
    #[test]
    fn skill_generation_guard_growth_resume_and_unchanged_strict_mode() {
        let evaluation = |n| record!({"dev_rows":(0..n).map(|i|record!({"id":format!("utf8/{i}"),"error_class":"strict_utf8","actual":null})).collect::<Vec<_>>(),"watch_rows":[]});
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
        let persisted = replica_v3::binary::to_vec(&(previous, streak)).unwrap();
        let (mut restored_previous, mut restored_streak): (usize, usize) =
            replica_v3::binary::from_slice(&persisted).unwrap();
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
            ("error_class", record!("control_token")),
            ("actual", record!("")),
            ("whitespace_only", record!(true)),
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
            resume_binding: None,
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
        let persisted = replica_v3::binary::to_vec(&tape).unwrap();
        let restored: Vec<(Vec<usize>, u64)> = replica_v3::binary::from_slice(&persisted).unwrap();
        assert_eq!(&tape[193..], &restored[193..]);
        let mut duplicate = episodes;
        duplicate[1].id = duplicate[0].id.clone();
        assert!(skill_tape(&duplicate, 921_917).is_err());
    }
    #[test]
    fn harness_h3_metric_requires_complete_eos_and_entity_citation_thresholds() {
        let row = record!({"expected":"센서31의 구역1 이동 지시는 동쪽이다. [event:7]","actual":"센서31의 구역1 이동 지시는 동쪽이다. [event:7]",
            "question":"센서31의 구역1 원문은?","category":0,"family":"skill/H3/test","exact_match":true,"error":null,"interruption":null,
            "finish_reason":"stop","generation":{"finish":"stop","generated":21},"eos_index":20,"generation_completed":true,"components":{"entity":true,"citation_exact":true}});
        let rows: Vec<_> = (0..256)
            .map(|i| {
                let mut r = row.clone();
                r["id"] = record!(format!("metric/{i}"));
                r["scene"] = record!(format!("base/{}", i / 4));
                r
            })
            .collect();
        assert_eq!(skill_score(&rows).unwrap()["skill_pass"], true);
        assert_eq!(skill_score(&rows).unwrap()["context_correct"], 256);
        assert_eq!(skill_score(&rows).unwrap()["value_correct"], 256);
        for (field, value) in [
            ("finish_reason", record!("length")),
            ("eos_index", Value::Null),
            ("generation_completed", record!(false)),
            ("actual", record!("")),
        ] {
            let mut bad = rows.clone();
            bad[0][field] = value;
            if field == "actual" {
                bad[0]["exact_match"] = record!(false);
            }
            if field == "finish_reason" {
                bad[0]["generation"]["finish"] = record!("length");
                bad[0]["exact_match"] = record!(false);
            }
            assert_eq!(skill_score(&bad).unwrap()["skill_pass"], false, "{field}");
            assert_eq!(skill_score(&bad).unwrap()["denominator"], 256);
        }
        for field in ["entity", "citation_exact"] {
            let mut bad = rows.clone();
            for row in &mut bad[..3] {
                row["components"][field] = record!(false);
                row["exact_match"] = record!(false);
                row["actual"] = record!("잘못된 답변");
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
            &dir.path().join("policy.r3b"),
            &record!({"stage":"H3","constant_lr":3e-5}),
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
        let source_hash = file_hash(&source.join("train.r3b")).unwrap();
        data::copy_curriculum(&source, &output, 917_260_311).unwrap();
        let (_, train, dev) = data::load_legacy(&output).unwrap();
        let seal_descriptor: data::Split =
            replica_v3::binary::from_value(read_metadata(&output.join("seal-manifest.r3b")).unwrap()).unwrap();
        let seal = data::load_split_legacy(&output, &seal_descriptor).unwrap();
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
        assert_eq!(file_hash(&source.join("train.r3b")).unwrap(), source_hash);
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
            let bytes = replica_v3::binary::to_vec(episodes).unwrap();
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
            train: write("train.r3b", &[repair_episode("train/0")]),
            validation: write("validation.r3b", validation),
        };
        std::fs::write(root.join("manifest.r3b"), replica_v3::binary::to_vec(&m).unwrap()).unwrap();
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
            registry: record!({}),
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
        let fixture = dir.path().join("frozen.r3b");
        save(&fixture, &f).unwrap();
        episodes[0].request.input = "changed same ID".into();
        repair_corpus(&f.corpus, &episodes); // Updated manifest is valid for the changed bytes.
        assert!(data::load_legacy(&f.corpus).is_ok());
        let output = dir.path().join("new.r3rows");
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
            assert!(data::load_legacy(&f.corpus).is_ok());
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
            let fixture = dir.path().join(format!("frozen-{mutation}.r3b"));
            save(&fixture, &f).unwrap();
            let existing = dir.path().join("existing.r3rows");
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
            f.corpus.join("manifest.r3b"),
            replica_v3::binary::to_vec(&manifest).unwrap(),
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
        let fixture = dir.path().join("frozen.r3b");
        save(&fixture, &f).unwrap();
        let output = dir.path().join("all.r3rows");
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
        let matched = dir.path().join("matched.r3rows");
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
    fn fresh_fx02_returned_generation_survives_command_deadline() {
        let l = repair_loaded();
        // A numeric fixture producing EOS through the real logits/greedy path.
        // No generated answer is substituted and this fixture is never a SMALL parent.
        for (name,v) in &l.model.vars {
            let mut data=vec![if name.ends_with("norm")||name=="embedding"{1f32}else{0f32};v.elem_count()];
            if name=="embedding" {let h=l.model.config.hidden;data[EOS as usize*h..(EOS as usize+1)*h].fill(2.);}
            v.set(&Tensor::from_vec(data,v.dims(),&Device::Cpu).unwrap()).unwrap();
        }
        let mut e = repair_episode("fresh/deadline");
        e.request.limits.max_tokens = 1;
        let mut c = repair_control();
        c.time_boundary = Some("generation_returned");
        let row = evaluate_one(&l, &e, &e.request, &mut c);
        assert_eq!(row["generation_started"], true);
        assert_eq!(row["generation_completed"], true);
        assert_eq!(row["finish_reason"], "stop");
        assert_eq!(row["command_stop"], "TIME_BUDGET");
        assert_eq!(c.teacher_calls, 0);
    }
    #[test]
    #[cfg(feature = "test-support")]
    fn fresh_fx06_native_command_timeout_is_returned_failure() {
        let l = repair_loaded();
        let mut e = repair_episode("native-timeout");
        e.request.limits.timeout_ms = 120000;
        let mut c = repair_control();
        neural::transformer::fixture_timeout_after(0);
        let ObservedCall::Returned(row) = observe_generation(&l, &e, &e.request, &mut c, false)
        else {
            panic!("native entry must be returned, not NotInvoked")
        };
        assert_eq!(c.generation_calls, 1);
        assert_eq!(row["raw_tokens"], record!([]));
        assert_eq!(row["generation_completed"], false);
        assert_eq!(row["error_class"], "timeout");
        assert_eq!(c.observed, vec![StopReason::TimeBudget]);
        println!("TINY_GENERATIONS=1 TEACHERS=0 OPTIMIZER=0 native_timeout=RETURNED");
    }
    #[test]
    #[cfg(feature = "test-support")]
    fn fresh_fx06_request_timeout_and_mixed_causes_stay_blocked() {
        let l = repair_loaded();
        for automatic in [false, true] {
            for request_cap in [false, true] {
                let mut e = repair_episode("cap-classification");
                e.request.limits.timeout_ms = if request_cap { 1 } else { 120000 };
                let mut c = repair_control();
                neural::transformer::fixture_timeout_after(0);
                let ObservedCall::Returned(row) =
                    observe_generation(&l, &e, &e.request, &mut c, automatic)
                else {
                    panic!("native timeout must return")
                };
                assert_eq!(
                    row["timeout_cap_source"],
                    if request_cap { "request" } else { "command" }
                );
                assert_eq!(
                    c.observed,
                    vec![if request_cap {
                        StopReason::IntegrityFail
                    } else {
                        StopReason::TimeBudget
                    }]
                );
                assert_eq!((c.generation_calls, c.teacher_calls), (1, 0));
            }
        }
        let mut e = repair_episode("mixed-timeout");
        e.request.limits.timeout_ms = 120000;
        for mixed in ["cancel", "io", "nonfinite"] {
            let mut c = repair_control();
            if mixed == "cancel" {
                c.hook = Some(Box::new(|boundary, cancel| {
                    if boundary == "generation_returned" {
                        cancel.store(true, Ordering::Relaxed);
                    }
                }));
            }
            if mixed == "nonfinite" {
                let v = &l.model.vars["embedding"];
                v.set(
                    &Tensor::from_vec(vec![f32::NAN; v.elem_count()], v.dims(), &Device::Cpu)
                        .unwrap(),
                )
                .unwrap();
                c.time_boundary = Some("generation_returned");
            } else {
                neural::transformer::fixture_timeout_after(0);
            }
            let ObservedCall::Returned(row) = observe_generation(&l, &e, &e.request, &mut c, false)
            else {
                panic!("native error must return")
            };
            if mixed == "io" {
                let d = tempfile::tempdir().unwrap();
                let error = std::fs::File::open(d.path().join("absent")).unwrap_err();
                c.classify_error(&Error::Io(error));
            }
            assert_eq!(row["generation_completed"], false);
            assert!(c.observed.contains(&StopReason::TimeBudget));
            assert!(c.observed.contains(&if mixed == "cancel" {
                StopReason::Cancelled
            } else {
                StopReason::IntegrityFail
            }));
        }
        println!(
            "TINY_GENERATIONS=7 TEACHERS=0 OPTIMIZER=0 request_vs_command_and_teacher_independence=VERIFIED mixed_causes=BLOCKED"
        );
    }
    #[test]
    fn fresh_fx05_actual_returned_zero_length_and_utf8_are_not_no_call() {
        let l = repair_loaded();
        for (token, limit) in [(b'x', 0), (b'x', 1), (0xff, 1)] {
            let id = l.tokenizer.encode(&[token]).unwrap()[0] as usize;
            for (name, v) in &l.model.vars {
                let mut data = vec![
                    if name.ends_with("norm") || name == "embedding" {
                        1f32
                    } else {
                        0f32
                    };
                    v.elem_count()
                ];
                if name == "embedding" {
                    let h = l.model.config.hidden;
                    data[id * h..(id + 1) * h].fill(2.);
                }
                v.set(&Tensor::from_vec(data, v.dims(), &Device::Cpu).unwrap())
                    .unwrap();
            }
            let mut e = repair_episode("returned-failure");
            e.request.limits.max_tokens = 1;
            let mut c = repair_control();
            if limit == 0 {
                c.hook = Some(Box::new(|boundary, flag| {
                    if boundary == "native_generation_entered" {
                        flag.store(true, Ordering::Relaxed);
                    }
                }));
            }
            let ObservedCall::Returned(row) = observe_generation(&l, &e, &e.request, &mut c, false)
            else {
                panic!("actual model call misclassified")
            };
            assert_eq!(c.generation_calls, 1);
            assert_eq!(c.teacher_calls, 0);
            assert_eq!(row["exact_match"], false);
            assert_eq!(row["raw_tokens"].as_array().unwrap().len(), limit as usize);
            if limit == 0 {
                assert_eq!(row["generation_completed"], false);
                assert!(!row["generation_error"].is_null());
            } else if token == 0xff {
                assert_eq!(row["generation_completed"], true);
                assert_eq!(row["error_class"], "strict_utf8");
            } else {
                assert_eq!(row["finish_reason"], "length");
                assert!(row["error"].is_null());
            }
        }
        println!("ACTUAL_TINY_GENERATIONS=3 TEACHERS=0 OPTIMIZER=0");
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
            evaluation["new_updates"] = record!(50);
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
        let bytes = replica_v3::binary::to_vec(&e).unwrap();
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
            save(&arm.join("result.r3b"), &result).unwrap();
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
        let summary = read_metadata(&out.join("summary.r3b"))
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
                arm["comparison_eligible"] = record!(false);
            }
            if boundary == "missing" {
                arm.as_object_mut()
                    .unwrap()
                    .remove("final_evaluation_complete");
            }
            let mut close_control = repair_control();
            let result = finish_close(
                dir.path(),
                record!({"candidate_eligible":true}),
                &[arm.clone(), arm],
                true,
                &mut close_control,
            );
            let report = read_metadata(&dir.path().join("summary.r3b")).unwrap();
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
            stopped["reason"] = record!(failure);
            assert!(!arm_terminal_eligible(&stopped));
        }
        let dir = tempfile::tempdir().unwrap();
        let mut final_cancel = repair_control();
        final_cancel.cancel.store(true, Ordering::Relaxed);
        assert!(
            finish_close(
                dir.path(),
                record!({"candidate_eligible":true}),
                &[arm.clone(), arm],
                true,
                &mut final_cancel
            )
            .is_err()
        );
        assert_eq!(
            read_metadata(&dir.path().join("summary.r3b")).unwrap()["candidate_eligible"],
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
            resume_binding: None,
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
        let tape: Vec<_> = (1..=50).map(|i| record!([[i], i])).collect();
        let policy = record!({"fixture_hash":"4".repeat(64),"source_id":l.manifest.source_id,"binary_hash":"5".repeat(64),"config":config,"parent":{"manifest":{"training":{"step":0}}},"tape":tape,"tape_hash":digest(&tape).unwrap()});
        save(&dir.path().join("policy.r3b"), &policy).unwrap();
        let trace: Vec<_> = (1..=50).map(|i|record!({"new_update":i,"cumulative_model_step":i,"optimizer_step":i,"schedule_step":i,"lr":config.learning_rate(i),"indices":[i],"ids":[format!("fixture-{i}")],"sampler_state":i,"input_tokens":2,"target_tokens":1})).collect();
        let row = record!({"generation_completed":true,"interruption":null});
        let eval = record!({"watch_rows":vec![row.clone();32],"train_exposure_panel":vec![row;16],"final_evaluation_complete":true,"comparison_eligible":true,"not_run_count":0,"attempted_case_count":48,"completed_generation_count":48,"planned_case_count":48,"terminal_reason":null,"model_step":50,"model_content_hash":l.model.weight_hash().unwrap()});
        save(&dir.path().join("eval-050.r3b"), &eval).unwrap();
        let mut control = repair_control();
        let mut arm = finish_arm(&mut control, true, |_| Ok(()));
        for (key, value) in [
            ("error", Value::Null),
            ("new_updates", record!(50)),
            ("cumulative_model_step", record!(50)),
            (
                "policy_sha256",
                record!(file_hash(&dir.path().join("policy.r3b")).unwrap()),
            ),
            (
                "checkpoint_file_sha256",
                record!(file_hash(&dir.path().join("final")).unwrap()),
            ),
            (
                "final_evaluation_sha256",
                record!(file_hash(&dir.path().join("eval-050.r3b")).unwrap()),
            ),
            ("last_evaluation", eval.clone()),
            ("model_content_hash", record!(l.model.weight_hash().unwrap())),
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
            record!({"candidate_eligible":true}),
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
            bad[key] = record!("f".repeat(64));
            assert!(
                verify_arm_receipt(dir.path(), &policy, &trace, &bad, &"4".repeat(64)).is_err(),
                "{key}"
            );
        }
        let mut bad_clock = trace.clone();
        bad_clock[49]["optimizer_step"] = record!(49);
        assert!(
            verify_arm_receipt(dir.path(), &policy, &bad_clock, &arm, &"4".repeat(64)).is_err()
        );
        let mut partial = eval;
        partial["train_exposure_panel"] = record!([]);
        partial["final_evaluation_complete"] = record!(false);
        std::fs::write(
            dir.path().join("eval-050.r3b"),
            replica_v3::binary::to_vec(&partial).unwrap(),
        )
        .unwrap();
        arm["final_evaluation_sha256"] =
            record!(file_hash(&dir.path().join("eval-050.r3b")).unwrap());
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
        let mut av = replica_v3::binary::to_value(&a).unwrap();
        let bv = replica_v3::binary::to_value(&b).unwrap();
        av["first_target_weight"] = record!(1.);
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
        let good = record!({"id":"a","family":"qa/test","category":0,"actual":"ok","expected":"ok","error":null,"generation":{"finish":"stop","generated":2},"exact_match":true});
        let bad = record!({"id":"b","family":"copy/test","category":0,"actual":null,"expected":"x","error":"invalid UTF-8","generation":{"finish":"stop","generated":3},"exact_match":false});
        let s = summarize(&[good.clone(), bad]).unwrap();
        assert_eq!(s["denominator"], 2);
        assert_eq!(s["qa"], record!([1, 1]));
        assert_eq!(s["auxiliary"], record!([0, 1]));
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
    #[test]
    fn progress_fork_required_fields_closed_resume_and_final_cancel() {
        let endpoint = record!({"reason":"SCREENING_BUDGET_REACHED","comparison_eligible":true,"cleanup_limit_exceeded":false,"new_updates":512,"checkpoint_saved":true,
            "save_error":null,"final_evaluation_complete":true,"control":{"terminal_reason":"COMPLETED","observed_conditions":[]},"resume_allowed":false});
        assert!(progress_endpoint_safe(&endpoint));
        for key in endpoint.as_object().unwrap().keys() {
            let mut bad = endpoint.clone();
            bad.as_object_mut().unwrap().remove(key);
            assert!(!progress_endpoint_safe(&bad), "{key}");
        }
        let mut interrupted = endpoint.clone();
        interrupted["control"]["terminal_reason"] = record!("CANCELLED");
        assert!(!progress_endpoint_safe(&interrupted));
        let policy = record!({"contract":PROGRESS_CONTRACT,"entry":"EXPERIMENT_FORK","parent_sha256":"a".repeat(64),"maximum_updates":512,
            "max_input_tokens":2_000_000,"max_target_tokens":500_000,"h3_max_updates":2048,"h3_seconds":7200,"normal_policy":"normal_greedy_v1","lr_policy":"C"});
        assert!(progress_policy_valid(&policy));
        for key in [
            "contract",
            "entry",
            "parent_sha256",
            "maximum_updates",
            "max_input_tokens",
            "max_target_tokens",
            "h3_max_updates",
            "h3_seconds",
            "normal_policy",
            "lr_policy",
        ] {
            let mut bad = policy.clone();
            bad.as_object_mut().unwrap().remove(key);
            assert!(!progress_policy_valid(&bad), "{key}");
        }
        let closed = record!({"reason":"SCREENING_BUDGET_REACHED","resume_allowed":false,"checkpoint_saved":true,"new_updates":1024});
        let bytes = replica_v3::binary::to_vec(&closed).unwrap();
        assert!(!progress_time_resume(&closed));
        assert_eq!(bytes, replica_v3::binary::to_vec(&closed).unwrap());
        let time = record!({"reason":"TIME_BUDGET","resume_allowed":true,"checkpoint_saved":true,"control":{"observed_conditions":["TIME_BUDGET"]},"save_error":null,"cleanup_limit_exceeded":false,"new_updates":256});
        assert!(progress_time_resume(&time));
        for reason in ["CANCELLED", "QUALITY_GUARD", "INTEGRITY_FAIL"] {
            let mut bad = time.clone();
            bad["reason"] = record!(reason);
            assert!(!progress_time_resume(&bad));
        }
        let dir = tempfile::tempdir().unwrap();
        let segment = dir.path().join("segment");
        std::fs::create_dir(&segment).unwrap();
        save(&dir.path().join("policy.r3b"), &policy).unwrap();
        let checkpoint = segment.join("not-loaded");
        let output = dir.path().join("not-created");
        let mut run = training_run(&checkpoint, &output);
        run.resume = true;
        assert!(
            train_controlled(run, &mut repair_control())
                .unwrap_err()
                .to_string()
                .contains("requires recovery progress-arm")
        );
        assert!(!output.exists());
        let mut control = repair_control();
        control.hook = Some(Box::new(|boundary, cancel| {
            if boundary == "checkpoint_preserved" {
                cancel.store(true, Ordering::Relaxed);
            }
        }));
        let finalization = finish_arm(&mut control, true, |_| Ok(()));
        assert_eq!(finalization["reason"], "CANCELLED");
        assert_eq!(finalization["comparison_eligible"], false);
        let mut cancelled = repair_control();
        cancelled.cancel.store(true, Ordering::Relaxed);
        assert!(progress_arm(dir.path(), "C", None, &mut cancelled).is_err());
        assert!(!dir.path().join("C").exists());
    }
    #[test]
    fn progress_guards_keep_errors_in_denominator_and_require_streaks() {
        let p = record!({"baseline_dev":208,"baseline_watch":18,"baseline_errors":2});
        let mut e = record!({"final_evaluation_complete":true,"dev":{"denominator":256,"exact_matches":208,"generation_error_cases":2},"watch":{"denominator":32,"exact_matches":18,"generation_error_cases":0}});
        let mut streak = [0; 3];
        assert!(!progress_guard(&p, &e, &mut streak).unwrap());
        e["dev"]["generation_error_cases"] = record!(4);
        assert!(!progress_guard(&p, &e, &mut streak).unwrap());
        e["dev"]["exact_matches"] = record!(182);
        assert!(!progress_guard(&p, &e, &mut streak).unwrap());
        assert!(progress_guard(&p, &e, &mut streak).unwrap());
        e["dev"]["exact_matches"] = record!(208);
        assert!(!progress_guard(&p, &e, &mut streak).unwrap());
        assert_eq!(streak, [0; 3]);
        e["watch"]["exact_matches"] = record!(14);
        assert!(!progress_guard(&p, &e, &mut streak).unwrap());
        assert!(progress_guard(&p, &e, &mut streak).unwrap());
        e["watch"]["exact_matches"] = record!(18);
        e["dev"]["generation_error_cases"] = record!(8);
        assert!(!progress_guard(&p, &e, &mut streak).unwrap());
        assert!(progress_guard(&p, &e, &mut streak).unwrap());
        e["dev"]["generation_error_cases"] = record!(58);
        assert!(progress_guard(&p, &e, &mut [0; 3]).unwrap());
        e["dev"]["denominator"] = record!(255);
        assert!(progress_guard(&p, &e, &mut streak).is_err());
        e["dev"]["denominator"] = record!(256);
        e["final_evaluation_complete"] = record!(false);
        assert!(progress_guard(&p, &e, &mut streak).is_err());
    }
    #[test]
    fn progress_renewal_materialization_distribution_exposure_and_split_cursor() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("original");
        let dev: Vec<_> = (0..256)
            .map(|i| repair_episode(&format!("heldout/{i}/0")))
            .collect();
        let mut manifest = repair_corpus(&root, &dev);
        let old: Vec<_> = (0..4096)
            .map(|i| {
                repair_episode(&if i < 2048 {
                    format!("anchor/{i}/0")
                } else {
                    format!("focus/{}/{}", (i - 2048) / 4, (i - 2048) % 4)
                })
            })
            .collect();
        let bytes = replica_v3::binary::to_vec(&old).unwrap();
        std::fs::write(root.join("train.r3b"), &bytes).unwrap();
        manifest.train = data::Split {
            file: "train.r3b".into(),
            sha256: neural::hash(&bytes),
            bytes: bytes.len(),
            documents: 4096,
            tokens: None,
        };
        std::fs::write(
            root.join("manifest.r3b"),
            replica_v3::binary::to_vec(&manifest).unwrap(),
        )
        .unwrap();
        let output = dir.path().join("pair");
        std::fs::create_dir(&output).unwrap();
        let original = data::load_legacy(&root).unwrap();
        let report = data::renewed_copy_curricula(&original, &dev, &output, 82119).unwrap();
        assert_eq!(report["arms"]["F"]["focus_views"], 512);
        assert_eq!(report["arms"]["N"]["focus_views"], 2048);
        let (_, f, _) = data::load_legacy(&output.join("corpus-F")).unwrap();
        let (_, n, _) = data::load_legacy(&output.join("corpus-N")).unwrap();
        assert_eq!(digest(&f[..2048]).unwrap(), digest(&old[..2048]).unwrap());
        assert_eq!(digest(&f).unwrap(), digest(&n[..2560]).unwrap());
        let mut l = repair_loaded();
        let docs: Vec<_> = f[2048..]
            .iter()
            .map(|e| replica_v3::binary::to_vec(e).unwrap())
            .collect();
        l.tokenizer =
            ByteBpe::train(&docs, &neural::hash(b"renewal test tokenizer only"), 801).unwrap();
        verify_cross_panel(&f[2048..], &dev, &l, 512).unwrap();
        verify_cross_panel(&n[2048..], &dev, &l, 2048).unwrap();
        assert!(verify_cross_panel(&n[2048..], &[n[2048].clone()], &l, 2048).is_err());
        let mut broken = f[2048..].to_vec();
        broken[0].request.evidence.items[0].original_excerpt.clear();
        assert!(verify_cross_panel(&broken, &dev, &l, 512).is_err());
        broken = f[2048..].to_vec();
        broken[0].request.evidence.items[0]
            .original_excerpt
            .push_str(" another unsupported fact");
        assert!(verify_cross_panel(&broken, &dev, &l, 512).is_err());
        let tapes = progress_renewal_tapes(&old, 19177).unwrap();
        assert_eq!(tapes, progress_renewal_tapes(&old, 19177).unwrap());
        let restored: [Vec<(Vec<usize>, u64)>; 2] =
            replica_v3::binary::from_slice(&replica_v3::binary::to_vec(&tapes).unwrap()).unwrap();
        for arm in 0..2 {
            let data = if arm == 0 { &f } else { &n };
            let mut draws = BTreeMap::<usize, usize>::new();
            assert_eq!(tapes[arm].len(), 512);
            assert_eq!(tapes[arm][193..], restored[arm][193..]);
            for (step, (indices, rng)) in tapes[arm].iter().enumerate() {
                assert_eq!(indices[..4], tapes[1 - arm][step].0[..4]);
                assert_eq!(*rng, tapes[1 - arm][step].1);
                assert_eq!(
                    indices
                        .iter()
                        .map(|i| scene(&data[*i]))
                        .collect::<BTreeSet<_>>()
                        .len(),
                    8
                );
                for (slot, i) in indices[4..].iter().enumerate() {
                    *draws.entry(*i).or_default() += 1;
                    let j = tapes[1 - arm][step].0[slot + 4];
                    assert_eq!((i - 2048) % 512, (j - 2048) % 512); // Same base stratum AND view.
                    assert_eq!(data[*i].family, if arm == 0 { &n } else { &f }[j].family);
                }
            }
            assert_eq!(draws.len(), if arm == 0 { 512 } else { 2048 });
            assert!(draws.values().all(|n| *n == if arm == 0 { 4 } else { 1 }));
        }
        let again = dir.path().join("again");
        std::fs::create_dir(&again).unwrap();
        data::renewed_copy_curricula(&original, &dev, &again, 82119).unwrap();
        assert_eq!(
            file_hash(&output.join("corpus-N/train.r3b")).unwrap(),
            file_hash(&again.join("corpus-N/train.r3b")).unwrap()
        );
    }
    #[test]
    fn progress_renewal_native_checkpoint_cursor_and_moments() {
        let original: Vec<_> = (0..4096)
            .map(|i| {
                repair_episode(&if i < 2048 {
                    format!("anchor/{i}/0")
                } else {
                    format!("focus/{}/{}", (i - 2048) / 4, (i - 2048) % 4)
                })
            })
            .collect();
        let tapes = progress_renewal_tapes(&original, 81727).unwrap();
        for (arm, tape) in tapes.iter().enumerate() {
            let dir = tempfile::tempdir().unwrap();
            let mut l = repair_loaded();
            let numeric: Vec<_> = (0..if arm == 0 { 2560 } else { 4096 })
                .map(|i| {
                    let mut e = repair_episode(&format!("numeric/{i}"));
                    e.request.input = format!("a{i}");
                    e.answer = format!("b{i}");
                    e
                })
                .collect();
            let framed = samples(&numeric, &l.tokenizer, 32).unwrap();
            let config = TrainConfig {
                seq_len: 32,
                microbatch: 8,
                accumulation: 1,
                max_steps: 1024,
                warmup: 0,
                first_target_weight: 8.,
                ..Default::default()
            };
            let mut state = TrainingState {
                resume_binding: None,
                contrast16: false,
                parent_checkpoint_hash: None,
                config: config.clone(),
                step: 209,
                consumed_tokens: 10000,
                target_tokens: 1000,
                sampler_state: tape[191].1,
                corpus_hash: l.tokenizer.train_hash.clone(),
                validation_hash: "3".repeat(64),
                previous_corpora: vec![],
                initial_weight_hash: l.manifest.initial_weight_hash.clone(),
                train_loss: None,
                validation_loss: None,
            };
            let mut adam = Adam::new(&l.model.vars).unwrap();
            let update = |l: &Loaded, adam: &mut Adam, state: &mut TrainingState| {
                let cursor = state.step - 17;
                let b = batch(&framed, &tape[cursor].0, &Device::Cpu).unwrap();
                let (_, loss, targets) =
                    response_loss(&l.model.forward(&b.input, Some(&b.valid)).unwrap(), &b, 8.)
                        .unwrap();
                let gradients = loss.backward().unwrap();
                let gradients = l
                    .model
                    .vars
                    .iter()
                    .map(|(name, var)| (name.clone(), gradients.get(var).unwrap().detach()))
                    .collect();
                adam.step_constant(
                    &l.model.vars,
                    &gradients,
                    &config,
                    state.step + 1,
                    progress_lr("L", 512 + cursor + 1).unwrap(),
                )
                .unwrap();
                state.step += 1;
                state.consumed_tokens += b.tokens as u64;
                state.target_tokens += targets as u64;
                state.sampler_state = tape[cursor].1;
            };
            update(&l, &mut adam, &mut state);
            save_arm(
                &mut l,
                &state,
                &adam,
                &dir.path().join("resume"),
                "RECOVERY_SCREENING",
            )
            .unwrap();
            let restored = checkpoint::load(&dir.path().join("resume"), Device::Cpu, true).unwrap();
            let mut restored_state = restored.manifest.training.clone().unwrap();
            let mut restored_adam = Adam {
                moments: restored.optimizer.clone(),
            };
            update(&l, &mut adam, &mut state);
            update(&restored, &mut restored_adam, &mut restored_state);
            assert_eq!(
                l.model.weight_hash().unwrap(),
                restored.model.weight_hash().unwrap()
            );
            assert_eq!(
                optimizer_hash(&adam.moments).unwrap(),
                optimizer_hash(&restored_adam.moments).unwrap()
            );
            assert_eq!(
                replica_v3::binary::to_value(state).unwrap(),
                replica_v3::binary::to_value(restored_state).unwrap()
            );
        }
        println!(
            "RENEWAL_NATIVE actual_TINY_updates=6 SMALL_updates=0; starting cursor192/clock209 are constructed numeric fixtures"
        );
    }
    #[test]
    fn progress_lr_native_split_adam_clock_and_cursor_parity() {
        let diagnostic = record!({"loss":f64::from(0.009906131_f32)});
        let stored: Value =
            replica_v3::binary::from_slice(&replica_v3::binary::to_vec(&diagnostic).unwrap()).unwrap();
        assert_eq!(diagnostic, stored); // Native metadata preserves the original f64 bits.
        assert_eq!(progress_snapshot(&diagnostic).unwrap(), stored);
        assert!(progress_lr("L", 0).is_err());
        assert_eq!(progress_lr("C", 512).unwrap(), 3e-5);
        assert_eq!(progress_lr("L", 1).unwrap(), 3e-5 + (1e-4 - 3e-5) / 64.);
        assert!((progress_lr("L", 64).unwrap() - 1e-4).abs() < 1e-18);
        assert_eq!(
            progress_lr("L", 64).unwrap(),
            progress_lr("L", 512).unwrap()
        );
        for policy in ["C", "L"] {
            let dir = tempfile::tempdir().unwrap();
            let mut l = repair_loaded();
            let framed = samples(&[repair_episode("lr-native")], &l.tokenizer, 32).unwrap();
            let b = batch(&framed, &[0], &Device::Cpu).unwrap();
            let config = TrainConfig {
                seq_len: 32,
                max_steps: 100,
                microbatch: 1,
                accumulation: 1,
                ..Default::default()
            };
            let mut state = TrainingState {
                resume_binding: None,
                contrast16: false,
                parent_checkpoint_hash: None,
                config: config.clone(),
                step: 17,
                consumed_tokens: 100,
                target_tokens: 10,
                sampler_state: 919,
                corpus_hash: l.tokenizer.train_hash.clone(),
                validation_hash: "3".repeat(64),
                previous_corpora: vec![],
                initial_weight_hash: l.manifest.initial_weight_hash.clone(),
                train_loss: None,
                validation_loss: None,
            };
            let update = |l: &Loaded, adam: &mut Adam, s: &mut TrainingState, j| {
                let (_, loss, n) =
                    response_loss(&l.model.forward(&b.input, Some(&b.valid)).unwrap(), &b, 1.)
                        .unwrap();
                let g = loss.backward().unwrap();
                let grads = l
                    .model
                    .vars
                    .iter()
                    .map(|(k, v)| (k.clone(), g.get(v).unwrap().detach()))
                    .collect();
                adam.step_constant(
                    &l.model.vars,
                    &grads,
                    &config,
                    s.step + 1,
                    progress_lr(policy, j).unwrap(),
                )
                .unwrap();
                s.step += 1;
                s.consumed_tokens += b.tokens as u64;
                s.target_tokens += n as u64;
                let mut rng = Rng {
                    state: s.sampler_state,
                };
                rng.next_u64();
                s.sampler_state = rng.state;
            };
            state.resume_binding = Some(checkpoint::ResumeBinding::default_for(&state, &l.tokenizer));
            let mut adam = Adam::new(&l.model.vars).unwrap();
            update(&l, &mut adam, &mut state, 63);
            save_arm(
                &mut l,
                &state,
                &adam,
                &dir.path().join("native"),
                "RECOVERY_SCREENING",
            )
            .unwrap();
            let restored = checkpoint::load(&dir.path().join("native"), Device::Cpu, true).unwrap();
            let mut other_state = restored.manifest.training.clone().unwrap();
            let mut other = Adam {
                moments: restored.optimizer.clone(),
            };
            for j in [64, 65] {
                update(&l, &mut adam, &mut state, j);
                update(&restored, &mut other, &mut other_state, j);
            }
            assert_eq!(
                l.model.weight_hash().unwrap(),
                restored.model.weight_hash().unwrap()
            );
            assert_eq!(
                optimizer_hash(&adam.moments).unwrap(),
                optimizer_hash(&other.moments).unwrap()
            );
            assert_eq!(
                replica_v3::binary::to_value(state).unwrap(),
                replica_v3::binary::to_value(other_state).unwrap()
            );
        }
        println!(
            "PROGRESS_LR_NATIVE actual_TINY_updates=10 SMALL_updates=0; clock17 and policy prefix62 are constructed fixtures"
        );
    }
}
