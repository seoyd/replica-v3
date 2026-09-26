//! One parent-bound completion of the preserved C256/T256 comparison.
use super::*;
use std::os::unix::fs::MetadataExt;

const CONTRACT: &str = "R3-TRPP-BINDING-COMPLETION-1.0";
const OLD_SOURCE: &str = "f130f53137d107b717ce2b1b628cec79b612fdc606c8f6186530c5a98a196aa5";
const OLD_BINARY: &str = "fc2c422a70419362d5747de66cf43bd31bd3a91d0e8f9357f6a47e25991e4423";
const OLD_PLAN: &str = "d4f48c84d658906fc44290b0ebbdc75b5aad9f6b309832a6b5c833d204a67649";
const OLD_REPORT: &str = "294e238df71330ee2e18e200606951a3d72857ad9c95d8b943401aebdd9e7535";
const OLD_B: &str = "d7d45a4abb0679cdddc6060a69089576ac531ddae3d3002a0d03ce1ba5872105";
const OLD_NATIVE: [&str; 2] = [
    "0c3fd8b2f63b1c36fb58e2e0d124fdec2e037bb48e92620f211cfe35b842fb75",
    "3dcc6317d80d929b47fd93389adf8cbdf3f2f0fc23b54dda193b36e41dcffde1",
];
const MAX_NEW_SECONDS: f64 = 2800.;
const MAX_TOTAL_SECONDS: f64 = 5000.;
const TRAIN_CUTOFF: f64 = 1400.;
const MAX_NEW_CALLS: usize = 1936;
const MAX_NEW_TOKENS: usize = 262144;
const MAX_NEW_BYTES: u64 = 512 * 1024 * 1024;
const MAX_COMBINED_BYTES: u64 = 1024 * 1024 * 1024;

#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Plan {
    contract: String,
    root: PathBuf,
    original: PathBuf,
    source: String,
    original_source: String,
    original_binary: String,
    original_plan: String,
    original_report: String,
    original_b: String,
    original_native: [Endpoint; 2],
    original_policy: [String; 2],
    old_runtime: RuntimeProfile,
    runtime: RuntimeProfile,
    tape_suffix: String,
    manifest: String,
    review_first8: String,
    sealed: String,
    tokenizer: String,
    corpus: String,
    lr: f64,
    old_seconds: f64,
    old_calls: usize,
    old_tokens: usize,
    expected_input: u64,
    expected_target: u64,
    target_baseline_kib: u64,
    old_artifact_logical: u64,
    old_artifact_allocated: u64,
    old_raw_hashes: BTreeMap<String, String>,
}

fn source() -> Result<String> {
    digest(&(
        super::source()?,
        neural::hash(include_bytes!("core_binding_completion.rs")),
    ))
}
fn policy(p: &Plan, arm: &str) -> Result<String> {
    digest(&(
        CONTRACT,
        &p.source,
        &p.original_plan,
        &p.original_report,
        &p.original_b,
        &p.original_native,
        &p.tape_suffix,
        &p.manifest,
        &p.runtime,
        p.lr,
        RULE,
        LAMBDA,
        arm,
    ))
}
fn allocated(root: &Path) -> Result<u64> {
    fn visit(root: &Path, seen: &mut BTreeSet<(u64, u64)>) -> Result<u64> {
        let meta = std::fs::symlink_metadata(root)?;
        if !seen.insert((meta.dev(), meta.ino())) {
            return Ok(0);
        }
        let mut n = meta
            .blocks()
            .checked_mul(512)
            .ok_or_else(|| bad("allocated overflow"))?;
        if meta.file_type().is_dir() {
            for e in std::fs::read_dir(root)? {
                n = n
                    .checked_add(visit(&e?.path(), seen)?)
                    .ok_or_else(|| bad("allocated overflow"))?;
            }
        }
        Ok(n)
    }
    visit(root, &mut BTreeSet::new())
}
fn old(root: &Path) -> Result<Study> {
    let root = root.canonicalize()?;
    let s: Study = read_confirmed(&root.join("plan.r3b"))?;
    if s.contract != super::CONTRACT
        || s.root != root
        || s.source != OLD_SOURCE
        || s.runtime.binary != OLD_BINARY
        || s.tape.len() != 512
        || s.pairs.len() != WORD_COUNT
        || file_hash(&root.join("plan.r3b"))? != OLD_PLAN
        || file_hash(&root.join("report-256.r3b"))? != OLD_REPORT
        || file_hash(&root.join("independent-b256-closure-v2.r3b"))? != OLD_B
        || file_hash(&root.join("sealed-confirmation.r3b"))? != s.confirmation_hash
    {
        return Err(bad("original C/T study identity"));
    }
    let report: binary::Value = read_confirmed(&root.join("report-256.r3b"))?;
    if report["common_step"] != 256
        || report["full_study_complete"] != false
        || report["generation_calls"] != 1664
        || report["generated_tokens"] != 30970
    {
        return Err(bad("original partial report"));
    }
    super::admitted(&s)?;
    let (p, ep) = parent(&s.parent_root)?;
    if ep != s.parent_endpoint
        || file_hash(&s.parent_root.join("plan.r3b"))? != s.parent_plan_hash
        || file_hash(&s.parent_root.join("TRPP/segment-006-returned.r3b"))? != s.parent_segment_hash
        || file_hash(&p.word_root.join("corpus.r3cor"))? != s.corpus
        || p.tokenizer_id != s.tokenizer
        || p.tape[..512] != s.tape
        || last_lr(&p)? != s.lr
    {
        return Err(bad("original parent/corpus/tape"));
    }
    for (i, arm) in ["C", "T"].iter().enumerate() {
        let rs = runs(&s, arm)?;
        let last = rs.last().ok_or_else(|| bad("original arm missing"))?;
        if rs.len() != 1
            || last.stop != "COMPLETED"
            || last.committed != 256
            || last.attempted != 256
            || last.input != 378496
            || last.target != 29952
            || last.after.hash != OLD_NATIVE[i]
            || last.after.state.committed != 3328
            || last.after.state.adam_clock != 3328
            || last.after.state.objective
                != if *arm == "C" {
                    checkpoint::ANSWER_MEAN_FAMILY
                } else {
                    8
                }
            || last.after.state.policy != arm_policy(&s, arm)?
            || file_hash(&last.after.path)? != OLD_NATIVE[i]
            || !root.join(arm).join("eval-256-complete.r3b").exists()
        {
            return Err(bad("original C/T256 native/receipt/score"));
        }
    }
    Ok(s)
}
fn prepare(original: &Path, output: &Path, target_baseline_kib: u64) -> Result<()> {
    let s = old(original)?;
    let registration = s.root.join("binding-completion-registration.r3b");
    if registration.exists() {
        return Err(bad("completion already registered for original study"));
    }
    if output.exists() {
        return Err(bad("completion output already exists"));
    }
    let p: StudyParent = read_confirmed(&s.parent_root.join("plan.r3b"))?;
    let panels = eval_panels(&s, &p)?;
    let word = panels
        .iter()
        .find(|p| p.0 == "word")
        .ok_or_else(|| bad("word panel"))?;
    let previous_report: binary::Value = read_confirmed(&s.root.join("report-256.r3b"))?;
    let mut old_raw_hashes = BTreeMap::new();
    for arm in ["C", "T"] {
        for panel in &panels {
            let score = s
                .root
                .join(arm)
                .join(format!("eval-256-{}-score.r3b", panel.0));
            let key = format!("{arm}_hash");
            if previous_report["scores"][panel.0.as_str()][key.as_str()] != file_hash(&score)? {
                return Err(bad("original 256 score hash changed"));
            }
            let raw = s
                .root
                .join(arm)
                .join(format!("eval-256-{}.r3rows", panel.0));
            let rows = binary::read_value_records(&raw)?;
            if rows.len() != panel.1.len() + 1
                || rows[0]["policy"] != arm_policy(&s, arm)?
                || rows[0]["native"] != OLD_NATIVE[arm_index(arm)?]
                || rows[0]["cases"] != digest(&panel.1)?
            {
                return Err(bad("original 256 raw identity"));
            }
            old_raw_hashes.insert(format!("{arm}/{}", panel.0), file_hash(&raw)?);
        }
    }
    let (_, _, _, _, samples) = inputs(&p)?;
    let mut total = (0u64, 0u64);
    let mut first = (0u64, 0u64);
    for (i, row) in s.tape.iter().enumerate() {
        for &at in row {
            let x = samples
                .get(at)
                .ok_or_else(|| bad("supplement tape bound"))?;
            let pair = (
                (x.tokens.len() - 1) as u64,
                (x.tokens.len() - x.response_start) as u64,
            );
            total.0 += pair.0;
            total.1 += pair.1;
            if i < 256 {
                first.0 += pair.0;
                first.1 += pair.1;
            }
        }
    }
    if total != (753536, 59904)
        || first != (378496, 29952)
        || (total.0 - first.0, total.1 - first.1) != (375040, 29952)
    {
        return Err(bad("supplement actual tape token totals"));
    }
    let device = Backend::Metal0.open()?;
    let runtime = RuntimeProfile::capture(Backend::Metal0, &device)?;
    compatible_parent_runtime(&s.runtime, &runtime)?;
    let rs = [runs(&s, "C")?, runs(&s, "T")?];
    let report: binary::Value = read_confirmed(&s.root.join("report-256.r3b"))?;
    let old_seconds = report["model_work_seconds"]
        .as_f64()
        .ok_or_else(|| bad("old model seconds"))?;
    if !old_seconds.is_finite() || old_seconds < 0. || old_seconds >= MAX_TOTAL_SECONDS {
        return Err(bad("old model seconds bound"));
    }
    let target_now = (allocated(Path::new("target/debug"))? + 1023) / 1024;
    if target_now > target_baseline_kib + 1024 * 1024 {
        return Err(bad("new target growth cap before preparation"));
    }
    std::fs::create_dir(output)?;
    let root = output.canonicalize()?;
    let plan = Plan {
        contract: CONTRACT.into(),
        root: root.clone(),
        original: s.root.clone(),
        source: source()?,
        original_source: s.source.clone(),
        original_binary: s.runtime.binary.clone(),
        original_plan: OLD_PLAN.into(),
        original_report: OLD_REPORT.into(),
        original_b: OLD_B.into(),
        original_native: [rs[0][0].after.clone(), rs[1][0].after.clone()],
        original_policy: [arm_policy(&s, "C")?, arm_policy(&s, "T")?],
        old_runtime: s.runtime.clone(),
        runtime,
        tape_suffix: digest(&s.tape[256..512].to_vec())?,
        manifest: digest(&panels)?,
        review_first8: digest(&word.1[..8].to_vec())?,
        sealed: s.confirmation_hash.clone(),
        tokenizer: s.tokenizer.clone(),
        corpus: s.corpus.clone(),
        lr: s.lr,
        old_seconds,
        old_calls: 1664,
        old_tokens: 30970,
        expected_input: 375040,
        expected_target: 29952,
        target_baseline_kib,
        old_artifact_logical: new_bytes(&s.root)?,
        old_artifact_allocated: allocated(&s.root)?,
        old_raw_hashes,
    };
    publish_confirmed(&root.join("plan.r3b"), &plan)?;
    let link = binary::record!({"contract":CONTRACT,"root":root,"plan_hash":file_hash(&root.join("plan.r3b"))?,
        "source":plan.source,"old_plan":OLD_PLAN,"old_report":OLD_REPORT,"old_b":OLD_B});
    publish_confirmed(&registration, &link)?;
    publish_confirmed(
        &root.join("preparation.r3b"),
        &binary::record!({"policy":digest(&plan)?,"old_seconds":old_seconds,
        "old_calls":1664,"old_tokens":30970,"target_baseline_kib":target_baseline_kib,
        "old_artifact_logical":plan.old_artifact_logical,"old_artifact_allocated":plan.old_artifact_allocated,
        "expected_input_per_arm":375040,"expected_target_per_arm":29952,"A":"PENDING","new_model_calls":0}),
    )?;
    println!("BINDING_COMPLETION_PREPARED policy={} old_seconds={old_seconds:.6} target_baseline_kib={target_baseline_kib} expected_input=375040 expected_target=29952 A_PENDING",digest(&plan)?);
    Ok(())
}
fn checked(root: &Path) -> Result<(Plan, Study)> {
    let root = root.canonicalize()?;
    let p: Plan = read_confirmed(&root.join("plan.r3b"))?;
    if p.contract != CONTRACT
        || p.root != root
        || p.source != source()?
        || p.original_source != OLD_SOURCE
        || p.original_binary != OLD_BINARY
        || p.original_plan != OLD_PLAN
        || p.original_report != OLD_REPORT
        || p.original_b != OLD_B
        || p.old_calls != 1664
        || p.old_tokens != 30970
        || p.expected_input != 375040
        || p.expected_target != 29952
    {
        return Err(bad("completion plan/source/old accounting"));
    }
    let s = old(&p.original)?;
    let link: binary::Value = read_confirmed(&s.root.join("binding-completion-registration.r3b"))?;
    if link["contract"] != CONTRACT
        || link["root"] != binary::record!(p.root)
        || link["plan_hash"] != file_hash(&root.join("plan.r3b"))?
        || link["source"] != p.source
    {
        return Err(bad("single completion registration"));
    }
    if file_hash(&std::env::current_exe()?)? != p.runtime.binary {
        return Err(bad("completion executable identity"));
    }
    compatible_parent_runtime(&s.runtime, &p.runtime)?;
    if p.old_runtime != s.runtime
        || p.tokenizer != s.tokenizer
        || p.corpus != s.corpus
        || p.lr != s.lr
        || p.sealed != s.confirmation_hash
        || p.tape_suffix != digest(&s.tape[256..512].to_vec())?
    {
        return Err(bad("completion unchanged training/confirmation identity"));
    }
    let parent: StudyParent = read_confirmed(&s.parent_root.join("plan.r3b"))?;
    let panels = eval_panels(&s, &parent)?;
    let word = panels
        .iter()
        .find(|x| x.0 == "word")
        .ok_or_else(|| bad("word panel"))?;
    if p.manifest != digest(&panels)? || p.review_first8 != digest(&word.1[..8].to_vec())? {
        return Err(bad("completion fixed evaluation manifest"));
    }
    if p.old_raw_hashes.len() != 14 {
        return Err(bad("completion original raw manifest"));
    }
    for (key, hash) in &p.old_raw_hashes {
        let (arm, name) = key.split_once('/').ok_or_else(|| bad("original raw key"))?;
        let raw = s.root.join(arm).join(format!("eval-256-{name}.r3rows"));
        if file_hash(&raw)? != *hash {
            return Err(bad("original 256 raw changed"));
        }
    }
    for (i, arm) in ["C", "T"].iter().enumerate() {
        let last = runs(&s, arm)?.pop().ok_or_else(|| bad("old arm"))?;
        if p.original_native[i] != last.after || p.original_policy[i] != arm_policy(&s, arm)? {
            return Err(bad("completion old endpoint/arm policy"));
        }
    }
    Ok((p, s))
}
fn admit(root: &Path, review: &Path) -> Result<()> {
    let (p, _) = checked(root)?;
    let r: binary::Value = read(review)?;
    if r["contract"] != CONTRACT
        || r["plan_hash"] != file_hash(&root.join("plan.r3b"))?
        || r["source"] != p.source
        || r["binary"] != p.runtime.binary
        || r["verdict"] != "PASS"
        || r["model_work_seconds"]
            .as_f64()
            .is_none_or(|v| !v.is_finite() || v < 0.)
    {
        return Err(bad("completion independent A binding"));
    }
    publish_confirmed(
        &root.join("review-a.r3b"),
        &binary::record!({"plan_hash":file_hash(&root.join("plan.r3b"))?,
        "path":review.canonicalize()?,"hash":file_hash(review)?}),
    )
}
fn admitted(p: &Plan) -> Result<f64> {
    let a: binary::Value = read_confirmed(&p.root.join("review-a.r3b"))?;
    let path = Path::new(a["path"].as_str().ok_or_else(|| bad("A path"))?);
    let r: binary::Value = read(path)?;
    if a["plan_hash"] != file_hash(&p.root.join("plan.r3b"))?
        || a["hash"] != file_hash(path)?
        || r["contract"] != CONTRACT
        || r["source"] != p.source
        || r["binary"] != p.runtime.binary
        || r["verdict"] != "PASS"
    {
        return Err(bad("completion A changed"));
    }
    r["model_work_seconds"]
        .as_f64()
        .filter(|v| v.is_finite() && *v >= 0.)
        .ok_or_else(|| bad("A model seconds"))
}
fn arm_index(arm: &str) -> Result<usize> {
    match arm {
        "C" => Ok(0),
        "T" => Ok(1),
        _ => Err(bad("completion arm")),
    }
}
fn resume_boundary(
    before: &Endpoint,
    original: &Endpoint,
    new_policy: &str,
    new_runtime: &RuntimeProfile,
    arm: &str,
) -> Result<()> {
    arm_index(arm)?;
    let a = &before.state;
    let b = &original.state;
    let expected_objective = if arm == "C" {
        checkpoint::ANSWER_MEAN_FAMILY
    } else {
        8
    };
    if b.committed != 3328
        || b.adam_clock != 3328
        || a.committed < 3328
        || a.committed > 3584
        || a.adam_clock != a.committed
        || a.objective != expected_objective
        || a.objective != b.objective
        || a.core != b.core
        || a.tokenizer != b.tokenizer
        || a.framing != b.framing
        || a.optimizer != b.optimizer
        || a.config != b.config
        || a.input_tokens < b.input_tokens
        || a.target_tokens < b.target_tokens
        || a.input_tokens - b.input_tokens > 400000
        || a.target_tokens - b.target_tokens > 32000
    {
        return Err(bad("completion native/Adam/objective/learning identity"));
    }
    if a.committed == 3328 {
        if before != original {
            return Err(bad("completion initial must be exact old C/T native"));
        }
    } else if a.policy != new_policy || &a.runtime != new_runtime {
        return Err(bad("completion continued policy/runtime"));
    }
    Ok(())
}
fn segments(p: &Plan, arm: &str) -> Result<Vec<Run>> {
    let i = arm_index(arm)?;
    let dir = p.root.join(arm);
    let mut before = p.original_native[i].clone();
    let mut out = vec![];
    for index in 0..16 {
        let entered = dir.join(format!("segment-{index:03}-entered.r3b"));
        let returned = dir.join(format!("segment-{index:03}-returned.r3b"));
        let trace = dir.join(format!("segment-{index:03}-trace.r3rows"));
        if !returned.exists() {
            if entered.exists() || trace.exists() {
                return Err(bad("UNKNOWN supplement training segment"));
            }
            break;
        }
        let r: Run = read_confirmed(&returned)?;
        let mark: binary::Value = read_confirmed(&entered)?;
        if r.policy != policy(p, arm)?
            || r.arm != arm
            || r.before != before
            || r.after.hash != file_hash(&r.after.path)?
            || r.after.state.committed != before.state.committed + r.committed
            || r.after.state.adam_clock != r.after.state.committed
            || r.attempted < r.committed
            || r.attempted > 264
            || r.committed > 256
            || r.trace != trace
            || r.trace_hash != file_hash(&trace)?
            || !r.seconds.is_finite()
            || r.seconds < 0.
            || r.after.state.committed > 3584
            || r.after.state.objective != before.state.objective
            || r.after.state.tokenizer != before.state.tokenizer
            || r.after.state.core != before.state.core
            || r.after.state.config != before.state.config
            || r.after.state.input_tokens != before.state.input_tokens + r.input
            || r.after.state.target_tokens != before.state.target_tokens + r.target
            || r.after.state.policy
                != if r.committed == 0 {
                    before.state.policy.clone()
                } else {
                    r.policy.clone()
                }
            || r.after.state.runtime
                != if r.committed == 0 {
                    before.state.runtime.clone()
                } else {
                    p.runtime.clone()
                }
            || !["COMPLETED", "TIME_BUDGET"].contains(&r.stop.as_str())
            || r.stop == "TIME_BUDGET"
                && r.control["observed_conditions"] != binary::record!(["TIME_BUDGET"])
            || mark["policy"] != r.policy
            || mark["before"] != binary::record!(before)
            || mark["index"] != index
        {
            return Err(bad("supplement training receipt integrity"));
        }
        if r.stop == "COMPLETED" && r.after.state.committed != 3584 {
            return Err(bad("completed supplement before endpoint"));
        }
        before = r.after.clone();
        out.push(r);
    }
    if out.iter().map(|r| r.attempted).sum::<usize>() > 264 {
        return Err(bad("supplement backward total"));
    }
    Ok(out)
}
fn endpoint(p: &Plan, arm: &str) -> Result<Endpoint> {
    Ok(segments(p, arm)?.last().map_or_else(
        || p.original_native[arm_index(arm).unwrap()].clone(),
        |r| r.after.clone(),
    ))
}
fn eval_runs(p: &Plan, arm: &str, step: usize) -> Result<Vec<EvalRun>> {
    arm_index(arm)?;
    let dir = p.root.join(arm);
    let mut out = vec![];
    for index in 0..16 {
        let entered = dir.join(format!("eval-{step}-segment-{index:03}-entered.r3b"));
        let returned = dir.join(format!("eval-{step}-segment-{index:03}-returned.r3b"));
        if !returned.exists() {
            if entered.exists() {
                return Err(bad("UNKNOWN supplement evaluation segment"));
            }
            break;
        }
        let r: EvalRun = read_confirmed(&returned)?;
        let mark: binary::Value = read_confirmed(&entered)?;
        if r.policy != policy(p, arm)?
            || r.arm != arm
            || r.step != step
            || r.native != endpoint(p, arm)?.hash
            || !r.seconds.is_finite()
            || r.seconds < 0.
            || r.calls > MAX_NEW_CALLS
            || r.tokens > MAX_NEW_TOKENS
            || !["COMPLETED", "TIME_BUDGET"].contains(&r.stop.as_str())
            || r.stop == "TIME_BUDGET"
                && r.control["observed_conditions"] != binary::record!(["TIME_BUDGET"])
            || mark["policy"] != r.policy
            || mark["native"] != r.native
            || mark["index"] != index
            || mark["step"] != step
        {
            return Err(bad("supplement evaluation receipt integrity"));
        }
        out.push(r);
    }
    Ok(out)
}
fn usage(p: &Plan) -> Result<(f64, usize, usize)> {
    let mut seconds = admitted(p)?;
    let (mut calls, mut tokens) = (0, 0);
    for arm in ["C", "T"] {
        seconds += segments(p, arm)?.iter().map(|r| r.seconds).sum::<f64>();
        for step in [512, 998, 999] {
            for r in eval_runs(p, arm, step)? {
                seconds += r.seconds;
                calls += r.calls;
                tokens += r.tokens;
            }
        }
    }
    if !seconds.is_finite() || seconds < 0. {
        return Err(bad("supplement model seconds"));
    }
    Ok((seconds, calls, tokens))
}
fn budget(
    p: &Plan,
    s: &Study,
    reserve_seconds: f64,
    reserve_bytes: u64,
) -> Result<(f64, usize, usize)> {
    let (seconds, calls, tokens) = usage(p)?;
    if !reserve_seconds.is_finite()
        || reserve_seconds < 0.
        || seconds + reserve_seconds > MAX_NEW_SECONDS
        || p.old_seconds + seconds + reserve_seconds > MAX_TOTAL_SECONDS
        || calls > MAX_NEW_CALLS
        || tokens > MAX_NEW_TOKENS
    {
        return Err(bad("supplement time/call/token admission"));
    }
    let n = new_bytes(&p.root)?;
    let old_n = new_bytes(&s.root)?;
    if n.saturating_add(reserve_bytes) > MAX_NEW_BYTES
        || old_n.saturating_add(n).saturating_add(reserve_bytes) > MAX_COMBINED_BYTES
    {
        return Err(bad("supplement artifact admission"));
    }
    let target = (allocated(Path::new("target/debug"))? + 1023) / 1024;
    if target > p.target_baseline_kib + 1024 * 1024 {
        return Err(bad("supplement shared-target growth"));
    }
    Ok((seconds, calls, tokens))
}
fn resource_state(p: &Plan, s: &Study) -> Result<(bool, f64, usize, usize, u64, u64, u64)> {
    let (seconds, calls, tokens) = usage(p)?;
    let bytes = new_bytes(&p.root)?;
    let old_bytes = new_bytes(&s.root)?;
    let target = (allocated(Path::new("target/debug"))? + 1023) / 1024;
    let met = seconds <= MAX_NEW_SECONDS
        && p.old_seconds + seconds <= MAX_TOTAL_SECONDS
        && calls <= MAX_NEW_CALLS
        && p.old_calls + calls <= 3600
        && tokens <= MAX_NEW_TOKENS
        && p.old_tokens + tokens <= 524288
        && bytes <= MAX_NEW_BYTES
        && old_bytes + bytes <= MAX_COMBINED_BYTES
        && target <= p.target_baseline_kib + 1024 * 1024;
    Ok((met, seconds, calls, tokens, bytes, old_bytes, target))
}
fn train(root: &Path, arm: &str) -> Result<()> {
    let began = Instant::now();
    let (p, s) = checked(root)?;
    admitted(&p)?;
    let i = arm_index(arm)?;
    let dir = p.root.join(arm);
    std::fs::create_dir_all(&dir)?;
    let lock = std::fs::OpenOptions::new()
        .create(true)
        .read(true)
        .write(true)
        .open(p.root.join("writer.lock"))?;
    lock.try_lock()
        .map_err(|_| bad("completion heavy process active"))?;
    let prior = segments(&p, arm)?;
    let before = endpoint(&p, arm)?;
    resume_boundary(
        &before,
        &p.original_native[i],
        &policy(&p, arm)?,
        &p.runtime,
        arm,
    )?;
    if before.state.committed >= 3584 || before.state.committed < 3328 {
        return Err(bad("completion training cursor"));
    }
    let attempted_before = prior.iter().map(|r| r.attempted).sum::<usize>();
    let (used, _, _) = budget(&p, &s, 600., 160 * 1024 * 1024)?;
    if used + 600. > TRAIN_CUTOFF || attempted_before >= 264 {
        return Err(bad("supplement training cutoff/backward cap"));
    }
    let parent: StudyParent = read_confirmed(&s.parent_root.join("plan.r3b"))?;
    let (_, _, _, _, samples) = inputs(&parent)?;
    let index = prior.len();
    let trace = dir.join(format!("segment-{index:03}-trace.r3rows"));
    let mark = binary::record!({"policy":policy(&p,arm)?,"before":before,"index":index,
        "cursor":before.state.committed-3328,"old_cursor":256,"until":3584,"used_seconds":used});
    publish_confirmed(&dir.join(format!("segment-{index:03}-entered.r3b")), &mark)?;
    let device = Backend::Metal0.open()?;
    p.runtime.verify(&device)?;
    let (mut core, mut adam) = load_endpoint(&before, device)?;
    if !matches!(core, Core::Tr(_))
        || before.state.objective
            != if arm == "C" {
                checkpoint::ANSWER_MEAN_FAMILY
            } else {
                8
            }
    {
        return Err(bad("supplement core/objective"));
    }
    let cancel = Arc::new(AtomicBool::new(false));
    let flag = cancel.clone();
    ctrlc::set_handler(move || flag.store(true, Ordering::Relaxed))
        .map_err(|e| bad(&e.to_string()))?;
    let remaining = (TRAIN_CUTOFF - used - began.elapsed().as_secs_f64())
        .min(MAX_NEW_SECONDS - used)
        .min(900.);
    if remaining <= 0. {
        return Err(bad("supplement no training time"));
    }
    let mut ctl = RunControl::new(cancel, Duration::from_secs_f64(remaining), 16 * 1024 * 1024)?;
    ctl.set_call_limits(0, 0);
    let mut file = std::fs::OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(&trace)?;
    append_row(&mut file, &mark)?;
    let mut st = before.state.clone();
    st.policy = policy(&p, arm)?;
    st.runtime = p.runtime.clone();
    let (mut backward, mut in_optimizer) = (0, false);
    let result = (|| -> Result<()> {
        while st.committed < 3584 {
            ctl.check("before_completion_update")?;
            if attempted_before + backward >= 264 {
                return Err(bad("completion backward cap"));
            }
            let row = &s.tape[st.committed - 3072];
            let b = batch(&samples, row, core.device())?;
            if st.input_tokens - p.original_native[i].state.input_tokens + b.tokens as u64 > 400000
            {
                return Err(bad("completion input cap"));
            }
            let logits = core.forward(&b)?;
            let (plain, base, n, _) = response_objective(&logits, &b, 1., true)?;
            if st.target_tokens - p.original_native[i].state.target_tokens + n as u64 > 32000 {
                return Err(bad("completion target cap"));
            }
            let loss = if arm == "T" {
                (base + branch_loss(&logits, row, &s.pairs, LAMBDA)?)?
            } else {
                base
            };
            let value = loss.to_scalar::<f32>()?;
            if !value.is_finite() {
                return Err(bad("completion nonfinite loss"));
            }
            backward += 1;
            let graph = loss.backward()?;
            let mut grads = BTreeMap::new();
            for (name, var) in core.vars() {
                grads.insert(
                    name.clone(),
                    graph
                        .get(var)
                        .ok_or_else(|| bad("completion missing gradient"))?
                        .detach(),
                );
            }
            core.device().synchronize()?;
            ctl.check("before_completion_optimizer")?;
            let clock = st.committed + 1;
            in_optimizer = true;
            let (norm, delta) = adam.apply_admitted_step(
                core.vars(),
                &grads,
                &st.config,
                clock,
                p.lr,
                |_, _, _, _| Ok(()),
            )?;
            core.device().synchronize()?;
            st.committed = clock;
            st.adam_clock = clock;
            st.input_tokens += b.tokens as u64;
            st.target_tokens += n as u64;
            in_optimizer = false;
            append_row(
                &mut file,
                &binary::record!({"commit":clock,"supplement_cursor":clock-3328,"original_cursor":clock-3072,
            "input":b.tokens,"target":n,"loss":value,"plain_ce":plain.to_scalar::<f32>()?,"lr":p.lr,
            "gradient_norm":norm,"update_l2":delta,"batch":row}),
            )?;
        }
        Ok(())
    })();
    if let Err(e) = &result {
        ctl.classify_error(e);
    }
    let stop = ctl.reason().unwrap_or("COMPLETED").to_string();
    let mut after = before.clone();
    if !in_optimizer && st.committed > before.state.committed {
        after = save_endpoint(
            &dir,
            &mut core,
            &adam,
            st.clone(),
            &format!("segment-{index:03}"),
        )?;
    }
    append_row(
        &mut file,
        &binary::record!({"phase":"RETURNED","after":after,"stop":stop}),
    )?;
    let r = Run {
        policy: policy(&p, arm)?,
        arm: arm.into(),
        before: before.clone(),
        after: after.clone(),
        attempted: backward,
        committed: after.state.committed - before.state.committed,
        input: after.state.input_tokens - before.state.input_tokens,
        target: after.state.target_tokens - before.state.target_tokens,
        seconds: began.elapsed().as_secs_f64(),
        stop: stop.clone(),
        trace: trace.clone(),
        trace_hash: file_hash(&trace)?,
        control: ctl.receipt(),
    };
    publish_confirmed(&dir.join(format!("segment-{index:03}-returned.r3b")), &r)?;
    println!("BINDING_COMPLETION_TRAIN arm={arm} supplement={} original={} global={} backward={} input={} target={} seconds={:.3} stop={stop} hash={}",
        after.state.committed-3328,after.state.committed-3072,after.state.committed,backward,r.input,r.target,r.seconds,after.hash);
    result
}
fn cases(p: &Plan, s: &Study, step: usize) -> Result<Vec<Panel>> {
    let parent: StudyParent = read_confirmed(&s.parent_root.join("plan.r3b"))?;
    if step == 512 {
        return eval_panels(s, &parent);
    }
    if step == 998 {
        let all = eval_panels(s, &parent)?;
        let word = all
            .into_iter()
            .find(|v| v.0 == "word")
            .ok_or_else(|| bad("review word panel"))?;
        if digest(&word.1[..8].to_vec())? != p.review_first8 {
            return Err(bad("review fixed first8"));
        }
        return Ok(vec![(
            "review-word8".into(),
            word.1[..8].to_vec(),
            word.2[..8].to_vec(),
        )]);
    }
    if step == 999 {
        let (es, ms): (Vec<Episode>, Vec<Meta>) =
            read_confirmed(&s.root.join("sealed-confirmation.r3b"))?;
        if file_hash(&s.root.join("sealed-confirmation.r3b"))? != p.sealed
            || es.len() != 128
            || ms.len() != 128
        {
            return Err(bad("unchanged sealed confirmation"));
        }
        return Ok(vec![("confirmation".into(), es, ms)]);
    }
    Err(bad("completion evaluation step"))
}
fn score(p: &Plan, arm: &str, step: usize, name: &str) -> Result<binary::Value> {
    let path = p
        .root
        .join(arm)
        .join(format!("eval-{step}-{name}-score.r3b"));
    let v: binary::Value = read_confirmed(&path)?;
    let raw = p.root.join(arm).join(format!("eval-{step}-{name}.r3rows"));
    if v["policy"] != policy(p, arm)?
        || v["model"] != endpoint(p, arm)?.content
        || v["raw_hash"] != file_hash(&raw)?
    {
        return Err(bad("completion score binding"));
    }
    Ok(v)
}
fn complete(p: &Plan, arm: &str, step: usize) -> Result<bool> {
    let path = p.root.join(arm).join(format!("eval-{step}-complete.r3b"));
    if !path.exists() {
        return Ok(false);
    }
    let v: binary::Value = read_confirmed(&path)?;
    let n = if step == 512 {
        832
    } else if step == 998 {
        8
    } else {
        128
    };
    if v["policy"] != policy(p, arm)?
        || v["native"] != endpoint(p, arm)?.hash
        || v["rows"] != n
        || v["stop"] != "COMPLETED"
    {
        return Err(bad("completion evaluation complete marker"));
    }
    Ok(true)
}
fn evaluation_binding(
    p: &Plan,
    arm: &str,
    step: usize,
    panel: &Panel,
    ep: &Endpoint,
    source_raw: Option<&str>,
) -> Result<binary::Value> {
    Ok(
        binary::record!({"policy":policy(p,arm)?,"arm":arm,"native":ep.hash,"model":ep.content,
        "step":step,"cases":digest(&panel.1)?,"metadata":digest(&panel.2)?,"tokenizer":p.tokenizer,
        "source_raw":source_raw,"accuracy_denominator":step!=998}),
    )
}
fn finalize_no_call(
    p: &Plan,
    s: &Study,
    arm: &str,
    step: usize,
    tok: &ByteBpe,
    panels: &[Panel],
    ep: &Endpoint,
) -> Result<bool> {
    let dir = p.root.join(arm);
    eval_runs(p, arm, step)?; // A missing RETURNED is UNKNOWN even when raw rows exist.
    for panel in panels {
        let label = format!("eval-{step}-{}", panel.0);
        let path = dir.join(format!("{label}.r3rows"));
        if !path.exists() {
            return Ok(false);
        }
        let rows = binary::read_value_records(&path)?;
        if rows.len() < panel.1.len() + 1 {
            return Ok(false);
        }
        if rows.len() != panel.1.len() + 1 {
            return Err(bad("completion extra RETURNED"));
        }
        let old_hash = if step == 998 {
            Some(file_hash(&dir.join("eval-512-word.r3rows"))?)
        } else {
            None
        };
        let binding = evaluation_binding(p, arm, step, panel, ep, old_hash.as_deref())?;
        if rows[0] != binding {
            return Err(bad("completion RETURNED binding"));
        }
        for (i, row) in rows.iter().skip(1).enumerate() {
            call_attempt(
                &dir,
                &label,
                "generation",
                &binding,
                &panel.1[i],
                i,
                Some(row),
            )?;
            verify_generated(row, tok)?;
            if row["id"] != panel.1[i].id || row["expected"] != panel.1[i].answer {
                return Err(bad("completion RETURNED case"));
            }
        }
        if step == 998 {
            let original = binary::read_value_records(&dir.join("eval-512-word.r3rows"))?;
            if original.len() != 193 {
                return Err(bad("B source word incomplete"));
            }
            for (a, b) in rows.iter().skip(1).zip(original.iter().skip(1)) {
                for field in [
                    "raw_tokens",
                    "raw_bytes",
                    "actual",
                    "finish_reason",
                    "error",
                    "generation_completed",
                ] {
                    if a[field] != b[field] {
                        return Err(bad("new process B raw mismatch"));
                    }
                }
            }
        }
    }
    for panel in panels {
        let label = format!("eval-{step}-{}", panel.0);
        let path = dir.join(format!("{label}.r3rows"));
        let rows = binary::read_value_records(&path)?;
        let raw = file_hash(&path)?;
        let mut value = panel_score(panel, &rows[1..], tok, &ep.content, &raw)?;
        value["policy"] = binary::record!(policy(p, arm)?);
        value["model"] = binary::record!(ep.content);
        value["raw_hash"] = binary::record!(raw);
        if step == 512 && panel.0 == "train192" {
            let parent: StudyParent = read_confirmed(&s.parent_root.join("plan.r3b"))?;
            let (corpus, _, _, _, _) = inputs(&parent)?;
            let mut exposure = vec![0usize; corpus.train.len()];
            for &at in s.tape.iter().flatten() {
                exposure[at] += 1;
            }
            value["actual_exposures"] = binary::record!(panel
                .1
                .iter()
                .map(|e| corpus
                    .train
                    .iter()
                    .position(|v| v.id == e.id)
                    .map(|i| exposure[i]))
                .collect::<Vec<_>>());
        }
        let score_path = dir.join(format!("{label}-score.r3b"));
        if score_path.exists() {
            if read_confirmed::<binary::Value>(&score_path)? != value {
                return Err(bad("completion changed score"));
            }
        } else {
            publish_confirmed(&score_path, &value)?;
        }
    }
    publish_confirmed(
        &dir.join(format!("eval-{step}-complete.r3b")),
        &binary::record!({"policy":policy(p,arm)?,
        "native":ep.hash,"rows":if step==512{832}else if step==998{8}else{128},
        "accuracy_denominator":step!=998,"stop":"COMPLETED",
        "new_calls":eval_runs(p,arm,step)?.iter().map(|r|r.calls).sum::<usize>()}),
    )?;
    println!("BINDING_COMPLETION_FINALIZED arm={arm} step={step} new_calls=0");
    Ok(true)
}
fn evaluate(root: &Path, arm: &str, step: usize) -> Result<()> {
    let began = Instant::now();
    let (p, s) = checked(root)?;
    admitted(&p)?;
    arm_index(arm)?;
    if ![512, 998, 999].contains(&step) {
        return Err(bad("completion eval step"));
    }
    if ["C", "T"]
        .iter()
        .any(|a| endpoint(&p, a).map_or(true, |e| e.state.committed != 3584))
    {
        return Err(bad("both same-512 natives required"));
    }
    if step == 998
        && ["C", "T"]
            .iter()
            .any(|a| !complete(&p, a, 512).unwrap_or(false))
    {
        return Err(bad("B requires both complete 512 panels"));
    }
    if step == 999 {
        let screen: binary::Value = read_confirmed(&p.root.join("screen.r3b"))?;
        if screen["same512_quality_criteria_met"] != true
            || !b_admitted(&p)?
            || !resource_state(&p, &s)?.0
        {
            return Err(bad("confirmation not eligible"));
        }
    }
    let lock = std::fs::OpenOptions::new()
        .create(true)
        .read(true)
        .write(true)
        .open(p.root.join("writer.lock"))?;
    lock.try_lock()
        .map_err(|_| bad("completion heavy process active"))?;
    if complete(&p, arm, step)? {
        return Err(bad("completion evaluation already complete"));
    }
    let ep = endpoint(&p, arm)?;
    let panels = cases(&p, &s, step)?;
    let parent: StudyParent = read_confirmed(&s.parent_root.join("plan.r3b"))?;
    let tok = ByteBpe::load(&parent.tokenizer)?;
    if finalize_no_call(&p, &s, arm, step, &tok, &panels, &ep)? {
        return Ok(());
    }
    let reserve = if step == 512 {
        750.
    } else if step == 998 {
        75.
    } else {
        220.
    };
    let bytes = if step == 512 {
        32 * 1024 * 1024
    } else {
        8 * 1024 * 1024
    };
    let (used, calls, tokens) = budget(&p, &s, reserve, bytes)?;
    if calls >= MAX_NEW_CALLS || tokens >= MAX_NEW_TOKENS {
        return Err(bad("completion generation exhausted"));
    }
    let dir = p.root.join(arm);
    std::fs::create_dir_all(&dir)?;
    let index = eval_runs(&p, arm, step)?.len();
    publish_confirmed(
        &dir.join(format!("eval-{step}-segment-{index:03}-entered.r3b")),
        &binary::record!({"policy":policy(&p,arm)?,"native":ep.hash,"index":index,"step":step,
            "model_seconds_before":used,"calls_before":calls,"tokens_before":tokens}),
    )?;
    let device = Backend::Metal0.open()?;
    p.runtime.verify(&device)?;
    let (core, _) = load_endpoint(&ep, device)?;
    let cancel = Arc::new(AtomicBool::new(false));
    let flag = cancel.clone();
    ctrlc::set_handler(move || flag.store(true, Ordering::Relaxed))
        .map_err(|e| bad(&e.to_string()))?;
    let remaining = (MAX_NEW_SECONDS - used - began.elapsed().as_secs_f64())
        .min(MAX_TOTAL_SECONDS - p.old_seconds - used - began.elapsed().as_secs_f64())
        .min(900.);
    if remaining <= 0. {
        return Err(bad("completion eval no time"));
    }
    let mut ctl = RunControl::new(cancel, Duration::from_secs_f64(remaining), 16 * 1024 * 1024)?;
    ctl.set_call_limits(MAX_NEW_CALLS - calls, 0);
    let mut generated = 0usize;
    let result = (|| -> Result<()> {
        for panel in &panels {
            let label = format!("eval-{step}-{}", panel.0);
            let original_hash = if step == 998 {
                Some(file_hash(&dir.join("eval-512-word.r3rows"))?)
            } else {
                None
            };
            let binding = evaluation_binding(&p, arm, step, panel, &ep, original_hash.as_deref())?;
            collect(
                &core,
                &tok,
                &dir,
                &label,
                &panel.1,
                &binding,
                &mut ctl,
                &mut generated,
                MAX_NEW_TOKENS - tokens,
            )?;
        }
        ctl.seal_completed_no_call()
    })();
    if let Err(e) = &result {
        ctl.classify_error(e);
    }
    let stop = ctl.reason().unwrap_or("COMPLETED").to_string();
    let r = EvalRun {
        policy: policy(&p, arm)?,
        arm: arm.into(),
        step,
        native: ep.hash.clone(),
        seconds: began.elapsed().as_secs_f64(),
        calls: ctl.receipt()["generation_calls"].as_u64().unwrap_or(0) as usize,
        tokens: generated,
        stop: stop.clone(),
        control: ctl.receipt(),
    };
    publish_confirmed(
        &dir.join(format!("eval-{step}-segment-{index:03}-returned.r3b")),
        &r,
    )?;
    if result.is_ok() {
        finalize_no_call(&p, &s, arm, step, &tok, &panels, &ep)?;
    }
    println!("BINDING_COMPLETION_EVAL arm={arm} step={step} calls={} tokens={} seconds={:.3} stop={stop}",r.calls,r.tokens,r.seconds);
    result
}
fn screen(root: &Path) -> Result<()> {
    let (p, s) = checked(root)?;
    admitted(&p)?;
    if ["C", "T"]
        .iter()
        .any(|arm| !complete(&p, arm, 512).unwrap_or(false))
    {
        return Err(bad("screen needs complete same-512"));
    }
    let mut scores = BTreeMap::<String, binary::Value>::new();
    let mut signal = true;
    for name in [
        "word",
        "renamed",
        "train192",
        "value",
        "citation",
        "S1Q1",
        "qa-old_qa-primary64",
    ] {
        let c = score(&p, "C", 512, name)?;
        let t = score(&p, "T", 512, name)?;
        let (cf, tf) = if name.starts_with("qa-") {
            (score_count(&c, "full")?, score_count(&t, "full")?)
        } else {
            (joint_count(&c, "full")?, joint_count(&t, "full")?)
        };
        let invalid = invalid_citation_rows(name, &t)?;
        signal &= invalid == 0;
        let paired = if name == "word" || name == "renamed" {
            let (gain, loss) = paired_counts(&c["joint"], &t["joint"], "exact")?;
            let (c_all, t_all) = (joint_count(&c, "all4")?, joint_count(&t, "all4")?);
            let (c_out, t_out) = (
                score_count(&c, "valid_outside_id")?,
                score_count(&t, "valid_outside_id")?,
            );
            let malformed = score_count(&t, "parse_failure_rows")?;
            signal &= tf >= cf + 20 && t_all >= c_all + 4 && t_out <= c_out && malformed == 0;
            binary::record!({"gain":gain,"loss":loss,"C_all4":c_all,"T_all4":t_all,
                "C_outside_id":c_out,"T_outside_id":t_out,"T_malformed":malformed})
        } else {
            signal &= if name == "train192" {
                tf >= cf
            } else {
                tf + 1 >= cf
            };
            binary::record!(null)
        };
        scores.insert(name.into(),binary::record!({"C_full":cf,"T_full":tf,"paired":paired,
            "T_invalid_citation_rows":invalid,
            "C_score_hash":file_hash(&p.root.join("C").join(format!("eval-512-{name}-score.r3b")))?,
            "T_score_hash":file_hash(&p.root.join("T").join(format!("eval-512-{name}-score.r3b")))?}));
    }
    let (resource, seconds, calls, tokens, bytes, old_bytes, target) = resource_state(&p, &s)?;
    let result = binary::record!({"contract":CONTRACT,"policy":digest(&p)?,"original_status":"PARTIAL_UNCHANGED",
        "supplement_execution_complete":true,"same512_quality_criteria_met":signal,
        "supplement_resource_met":resource,"CUMULATIVE_ACCOUNTED_MODEL_SECONDS":p.old_seconds+seconds,
        "old_shared_target_growth":"UNKNOWN","new_model_seconds":seconds,"new_generation_calls":calls,
        "new_generated_tokens":tokens,"new_artifact_bytes":bytes,"combined_artifact_bytes":old_bytes+bytes,
        "target_baseline_kib":p.target_baseline_kib,"target_now_kib":target,
        "scores":scores,"confirmation_eligible_before_B":signal&&resource,
        "MODEL_QUALITY_ACCEPTED":false,"GOAL1_ACCEPTED":false});
    publish_confirmed(&p.root.join("screen.r3b"), &result)?;
    println!("BINDING_COMPLETION_SCREEN same512=true quality={signal} resource={resource} calls={calls} tokens={tokens} seconds={seconds:.3}");
    Ok(())
}
fn admit_b(root: &Path, review: &Path) -> Result<()> {
    let (p, _) = checked(root)?;
    admitted(&p)?;
    for arm in ["C", "T"] {
        if !complete(&p, arm, 998)? {
            return Err(bad("B missing first8 fresh-process replay"));
        }
    }
    let r: binary::Value = read(review)?;
    if r["contract"] != CONTRACT
        || r["plan_hash"] != file_hash(&p.root.join("plan.r3b"))?
        || r["source"] != p.source
        || r["binary"] != p.runtime.binary
        || r["verdict"] != "PASS"
        || r["raw_recount_rows"] != 1664
        || r["reproduction_rows"] != 16
        || r["model_work_seconds"] != 0
    {
        return Err(bad("independent B raw/reproduction binding"));
    }
    publish_confirmed(
        &p.root.join("review-b.r3b"),
        &binary::record!({"plan_hash":file_hash(&p.root.join("plan.r3b"))?,
        "path":review.canonicalize()?,"hash":file_hash(review)?}),
    )
}
fn b_admitted(p: &Plan) -> Result<bool> {
    let path = p.root.join("review-b.r3b");
    if !path.exists() {
        return Ok(false);
    }
    let link: binary::Value = read_confirmed(&path)?;
    let raw = Path::new(link["path"].as_str().ok_or_else(|| bad("B report path"))?);
    let report: binary::Value = read(raw)?;
    if link["plan_hash"] != file_hash(&p.root.join("plan.r3b"))?
        || link["hash"] != file_hash(raw)?
        || report["contract"] != CONTRACT
        || report["source"] != p.source
        || report["binary"] != p.runtime.binary
        || report["verdict"] != "PASS"
        || report["raw_recount_rows"] != 1664
        || report["reproduction_rows"] != 16
    {
        return Err(bad("independent B changed"));
    }
    Ok(true)
}
fn confirmation_report(p: &Plan) -> Result<binary::Value> {
    for arm in ["C", "T"] {
        if !complete(p, arm, 999)? {
            return Err(bad("confirmation incomplete"));
        }
    }
    let c = score(p, "C", 999, "confirmation")?;
    let t = score(p, "T", 999, "confirmation")?;
    let (cf, tf) = (joint_count(&c, "full")?, joint_count(&t, "full")?);
    let (ca, ta) = (joint_count(&c, "all4")?, joint_count(&t, "all4")?);
    let (co, to) = (
        score_count(&c, "valid_outside_id")?,
        score_count(&t, "valid_outside_id")?,
    );
    let consistent = tf >= cf + 8 && ta >= ca + 2 && to <= co;
    Ok(
        binary::record!({"C_full":cf,"T_full":tf,"C_all4":ca,"T_all4":ta,
        "C_outside_id":co,"T_outside_id":to,"CONFIRM_DIRECTION_CONSISTENT":consistent,
        "MODEL_QUALITY_ACCEPTED":false,"GOAL1_ACCEPTED":false}),
    )
}
fn finish(root: &Path) -> Result<()> {
    let (p, s) = checked(root)?;
    admitted(&p)?;
    let screen: binary::Value = read_confirmed(&p.root.join("screen.r3b"))?;
    if screen["policy"] != digest(&p)?
        || screen["supplement_execution_complete"] != true
        || !b_admitted(&p)?
    {
        return Err(bad("completion final requires same512 and B"));
    }
    let quality = screen["same512_quality_criteria_met"] == true;
    let resources = screen["supplement_resource_met"] == true && resource_state(&p, &s)?.0;
    let confirmed = ["C", "T"]
        .iter()
        .map(|arm| complete(&p, arm, 999))
        .collect::<Result<Vec<_>>>()?;
    let missing = confirmed.iter().filter(|done| !**done).count();
    let confirmation_budget_exhausted = if quality && resources && missing != 0 {
        let (_, seconds, calls, tokens, bytes, old_bytes, _) = resource_state(&p, &s)?;
        let per_arm_tokens = cases(&p, &s, 999)?
            .iter()
            .flat_map(|panel| panel.1.iter())
            .map(|e| e.request.limits.max_tokens as usize)
            .sum::<usize>();
        seconds + 220. * missing as f64 > MAX_NEW_SECONDS
            || p.old_seconds + seconds + 220. * missing as f64 > MAX_TOTAL_SECONDS
            || calls + 128 * missing > MAX_NEW_CALLS
            || tokens + per_arm_tokens * missing > MAX_NEW_TOKENS
            || bytes + 8 * 1024 * 1024 * missing as u64 > MAX_NEW_BYTES
            || old_bytes + bytes + 8 * 1024 * 1024 * missing as u64 > MAX_COMBINED_BYTES
    } else {
        false
    };
    if quality && resources && missing != 0 && !confirmation_budget_exhausted {
        return Err(bad("eligible confirmation still pending"));
    }
    let confirmation = if quality && resources && missing == 0 {
        let value = confirmation_report(&p)?;
        publish_confirmed(&p.root.join("confirmation-report.r3b"), &value)?;
        value
    } else {
        binary::record!(null)
    };
    let direction = if !resources {
        "PARTIAL_RESOURCE_NOT_MET"
    } else if !quality {
        "COMPLETE_NO_CLEAR_SIGNAL"
    } else if confirmation_budget_exhausted {
        "SCREEN_POSITIVE_CONFIRMATION_NOT_RUN"
    } else if confirmation["CONFIRM_DIRECTION_CONSISTENT"] == true {
        "COMPLETE_FOLLOWUP_SIGNAL_ONLY"
    } else {
        "COMPLETE_CONFIRMATION_FAILED"
    };
    let (seconds, calls, tokens) = usage(&p)?;
    publish_confirmed(
        &p.root.join("final-report.r3b"),
        &binary::record!({"contract":CONTRACT,
        "policy":digest(&p)?,"original_status":"PARTIAL_UNCHANGED","supplement_status":direction,
        "same512_quality_criteria_met":quality,"supplement_resource_met":resources,
        "independent_B_reproduction":true,"new_model_seconds":seconds,
        "cumulative_accounted_model_seconds":p.old_seconds+seconds,"new_calls":calls,
        "cumulative_calls":p.old_calls+calls,"new_tokens":tokens,"confirmation":confirmation,
        "MODEL_QUALITY_ACCEPTED":false,"PRODUCT_INTEGRATED":false,"GOAL1_ACCEPTED":false}),
    )?;
    println!("BINDING_COMPLETION_FINAL status={direction} quality={quality} resource={resources} new_calls={calls} new_tokens={tokens} seconds={seconds:.3}");
    Ok(())
}
pub(super) fn run(
    root: &Path,
    phase: &str,
    original: Option<&Path>,
    review: Option<&Path>,
    arm: Option<&str>,
    target_baseline_kib: Option<u64>,
) -> Result<()> {
    match phase {
        "prepare" => prepare(
            original.ok_or_else(|| bad("original required"))?,
            root,
            target_baseline_kib.ok_or_else(|| bad("target baseline required"))?,
        ),
        "inspect" => {
            let (p, _) = checked(root)?;
            if p.root.join("review-a.r3b").exists() {
                let (seconds, calls, tokens) = usage(&p)?;
                println!("BINDING_COMPLETION_INSPECT policy={} A=true new_seconds={seconds:.3} new_calls={calls} new_tokens={tokens}",digest(&p)?);
            } else {
                println!(
                    "BINDING_COMPLETION_INSPECT policy={} A_PENDING new_model_work=NOT_STARTED",
                    digest(&p)?
                );
            }
            Ok(())
        }
        "admit-a" => admit(root, review.ok_or_else(|| bad("A review required"))?),
        "train" => train(root, arm.ok_or_else(|| bad("arm required"))?),
        "evaluate" => evaluate(root, arm.ok_or_else(|| bad("arm required"))?, 512),
        "screen" => screen(root),
        "reproduce" => evaluate(root, arm.ok_or_else(|| bad("arm required"))?, 998),
        "admit-b" => admit_b(root, review.ok_or_else(|| bad("B review required"))?),
        "confirm" => evaluate(root, arm.ok_or_else(|| bad("arm required"))?, 999),
        "finish" => finish(root),
        _ => Err(bad("completion phase")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn allocated_bytes_count_hardlinked_build_output_once() -> Result<()> {
        let dir = tempfile::tempdir()?;
        let first = dir.path().join("first");
        std::fs::write(&first, vec![1u8; 8192])?;
        std::fs::hard_link(&first, dir.path().join("second"))?;
        let directory_blocks = std::fs::symlink_metadata(dir.path())?.blocks() * 512;
        assert_eq!(
            allocated(dir.path())?,
            directory_blocks + allocated(&first)?
        );
        Ok(())
    }

    #[test]
    fn exact_parent_and_continued_clock_are_required() -> Result<()> {
        let runtime = RuntimeProfile {
            backend: Backend::Metal0,
            actual_device: "Metal(0)".into(),
            dtype: "F32".into(),
            accumulator: "F32".into(),
            patch: "patch".into(),
            patch_source: "source".into(),
            lock: "lock".into(),
            binary: "old".into(),
            os_build: "os".into(),
            fast_math: "UNKNOWN".into(),
        };
        let mut next_runtime = runtime.clone();
        next_runtime.binary = "new".into();
        let mut config = super::super::config();
        config.max_steps = 3584;
        config.warmup = 0;
        let original = Endpoint {
            state: ComparisonState {
                schema: 1,
                core: ComparisonCore::Trpp(Config::tiny(264)),
                policy: "old-policy".into(),
                tokenizer: "tokenizer".into(),
                framing: Framing::QuestionEvidence.digest(),
                runtime,
                objective: checkpoint::ANSWER_MEAN_FAMILY,
                optimizer: "FRESH_ADAMW_ALL_V1".into(),
                config,
                committed: 3328,
                adam_clock: 3328,
                input_tokens: 100000,
                target_tokens: 10000,
            },
            path: PathBuf::from("original.r3model"),
            hash: "physical".into(),
            content: "weights".into(),
        };
        resume_boundary(&original, &original, "new-policy", &next_runtime, "C")?;
        let mut changed = original.clone();
        changed.path = PathBuf::from("different.r3model");
        assert!(resume_boundary(&changed, &original, "new-policy", &next_runtime, "C").is_err());
        for mutation in ["clock", "objective", "lr", "core", "tokenizer", "arm"] {
            let mut bad = original.clone();
            match mutation {
                "clock" => bad.state.adam_clock += 1,
                "objective" => bad.state.objective = 8,
                "lr" => bad.state.config.lr *= 2.,
                "core" => bad.state.core = ComparisonCore::Trpp(Config::tiny(265)),
                "tokenizer" => bad.state.tokenizer = "other".into(),
                _ => {}
            }
            assert!(
                resume_boundary(
                    &bad,
                    &original,
                    "new-policy",
                    &next_runtime,
                    if mutation == "arm" { "T" } else { "C" }
                )
                .is_err(),
                "{mutation}"
            );
        }
        let mut continued = original.clone();
        continued.state.committed = 3329;
        continued.state.adam_clock = 3329;
        continued.state.policy = "new-policy".into();
        continued.state.runtime = next_runtime.clone();
        continued.state.input_tokens += 100;
        continued.state.target_tokens += 10;
        resume_boundary(&continued, &original, "new-policy", &next_runtime, "C")?;
        continued.state.policy = "old-policy".into();
        assert!(resume_boundary(&continued, &original, "new-policy", &next_runtime, "C").is_err());
        Ok(())
    }
}
