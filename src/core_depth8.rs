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

#[derive(Subcommand)]
pub enum Action {
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
}

fn source() -> Result<String> {
    digest(&(source_digest()?, neural::hash(include_bytes!("core_depth8.rs"))))
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
    let p: Plan = read_confirmed(&root.join("plan.r3b"))?;
    let cores_match = matches!(&p.cores[0], ComparisonCore::Trpp(c) if *c == Config::small(c.vocab))
        && matches!(&p.cores[1], ComparisonCore::Trpp(c) if *c == Config::depth8(c.vocab));
    if p.contract != CONTRACT || p.root != root || p.source != source()?
        || p.runtime.binary != file_hash(&std::env::current_exe()?)?
        || p.config != train_config() || !cores_match {
        return Err(bad("depth plan/source/config"));
    }
    parent(&p)?;
    Ok(p)
}
fn admitted(p: &Plan) -> Result<()> {
    let link: binary::Value = read_confirmed(&p.root.join("review-a.r3b"))?;
    let path = Path::new(link["path"].as_str().ok_or_else(||bad("depth A report path"))?);
    let r: binary::Value = read(path)?;
    if link["hash"] != file_hash(path)? || link["plan"] != file_hash(&p.root.join("plan.r3b"))?
        || r["contract"] != CONTRACT || r["plan_hash"] != link["plan"]
        || r["source"] != p.source || r["binary"] != p.runtime.binary
        || r["verdict"] != "PASS" || r["independent_R"] != p.correction_hash {
        return Err(bad("depth independent R+A admission"));
    }
    Ok(())
}
fn admit(root: &Path, review: &Path) -> Result<()> {
    let p = checked(root)?;
    let r: binary::Value = read(review)?;
    if r["contract"] != CONTRACT || r["plan_hash"] != file_hash(&p.root.join("plan.r3b"))?
        || r["source"] != p.source || r["binary"] != p.runtime.binary
        || r["verdict"] != "PASS" || r["independent_R"] != p.correction_hash {
        return Err(bad("depth A scope/identity"));
    }
    publish_confirmed(&p.root.join("review-a.r3b"), &binary::record!({
        "path":review.canonicalize()?,"hash":file_hash(review)?,"plan":file_hash(&p.root.join("plan.r3b"))?}))
}
fn history(p: &Plan, arm: &str) -> Result<Vec<Run>> {
    let dir = p.root.join(arm);
    let mut before: Endpoint = read_confirmed(&dir.join("initial.r3b"))?;
    if before.state != state(p, arm)? || before.hash != file_hash(&before.path)? {
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
fn train(root: &Path, arm: &str, until: usize) -> Result<()> {
    let command_started=Instant::now();
    let p = checked(root)?; admitted(&p)?; arm_index(arm)?;
    resource_guard(&p,96*1024*1024)?;
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
    if seconds + command_started.elapsed().as_secs_f64() >= TRAIN_CUTOFF {
        return Err(bad("depth training cutoff"));
    }
    let (_, comparison) = parent(&p)?;
    let (_, _, _, _, samples) = inputs(&comparison)?;
    let device = Backend::Metal0.open()?; p.runtime.verify(&device)?;
    let (mut core, mut adam) = load_endpoint(&before, device)?;
    let cancel = Arc::new(AtomicBool::new(false)); let signal = cancel.clone();
    ctrlc::set_handler(move || signal.store(true, Ordering::Relaxed)).map_err(|e|bad(&e.to_string()))?;
    let remaining=MAX_SECONDS-seconds-command_started.elapsed().as_secs_f64();
    if remaining<=0. {return Err(bad("depth model time cap"));}
    let mut ctl = RunControl::new(cancel, Duration::from_secs_f64(remaining.min(1800.)),16*1024*1024)?;
    ctl.set_call_limits(0,0);
    let index = prior.len(); let dir = p.root.join(arm);
    let trace = dir.join(format!("segment-{index:03}-trace.r3rows"));
    let mut f = std::fs::OpenOptions::new().write(true).create_new(true).open(&trace)?;
    publish_confirmed(&dir.join(format!("segment-{index:03}-entered.r3b")),&binary::record!({
        "policy":policy(&p,arm)?,"before":before,"index":index,"until":until}))?;
    let mut st = before.state.clone();
    let (mut backwards, mut in_optimizer) = (0usize, false);
    let result = (|| -> Result<()> {
        while st.committed < until {
            ctl.check("before_depth_update")?;
            if prior.iter().map(|r|r.backwards).sum::<usize>() + backwards >= 528 { return Err(bad("depth backward cap")); }
            let row = &p.tape[st.committed];
            let b = batch(&samples, row, core.device())?;
            append_row(&mut f, &binary::record!({"phase":"ENTERED","cursor":st.committed+1,"batch":row}))?;
            let update = super::super::update(&core, &mut adam, &mut st, &b, &mut ctl, &mut backwards, &mut in_optimizer)?;
            append_row(&mut f, &update)?;
        }
        Ok(())
    })();
    if let Err(e)=&result { ctl.classify_error(e); }
    let stop=ctl.reason().unwrap_or("COMPLETED").to_string();
    let after = if !in_optimizer && st.committed > before.state.committed {
        save_endpoint(&dir, &mut core, &adam, st, &format!("segment-{index:03}"))?
    } else {before.clone()};
    append_row(&mut f, &binary::record!({"phase":"RETURNED","after":after,"stop":stop,
        "discarded_backwards":backwards.saturating_sub(after.state.committed-before.state.committed)}))?;
    let r=Run {policy:policy(&p,arm)?,arm:arm.into(), before:before.clone(), after:after.clone(),
        backwards,input:after.state.input_tokens-before.state.input_tokens,
        target:after.state.target_tokens-before.state.target_tokens,seconds:command_started.elapsed().as_secs_f64(),
        stop:stop.clone(),trace:trace.clone(),trace_hash:file_hash(&trace)?,control:ctl.receipt()};
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
fn panels(p: &Plan, step: usize) -> Result<Vec<Panel>> {
    let (s, comparison) = parent(p)?;
    let all = eval_panels(&s, &comparison)?;
    if digest(&all)? != p.panels { return Err(bad("depth panel manifest")); }
    match step {
        0 => Ok(all.into_iter().filter(|x|x.0=="word"||x.0=="renamed")
            .map(|(n,e,m)|(n,e[..8].to_vec(),m[..8].to_vec())).collect()),
        128 => all.into_iter().filter(|x|["word","renamed","train192","qa-old_qa-primary64"].contains(&x.0.as_str()))
            .map(|(n,e,m)|{
                if n!="qa-old_qa-primary64" {return Ok((n,e[..16].to_vec(),m[..16].to_vec()));}
                let indices=sentinel_qa_indices(&m)?;
                Ok((n,indices.iter().map(|&i|e[i].clone()).collect(),
                    indices.iter().map(|&i|m[i].clone()).collect()))
            }).collect(),
        512 => Ok(all),
        998 => Ok(all.into_iter().filter(|x|x.0=="word")
            .map(|(_,e,m)|("review-word8".into(),e[..8].to_vec(),m[..8].to_vec())).collect()),
        _ => Err(bad("depth evaluation step")),
    }
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
    let binding=binary::record!({"policy":policy(p,arm)?,"arm":arm,"native":ep.hash,
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
    expected["policy"]=binary::record!(policy(p,arm)?);
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
    if step==0 && !p.root.join("direct-numeric.r3b").exists() {return Err(bad("depth numeric direct test required"));}
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
    if seconds+command_started.elapsed().as_secs_f64()>=MAX_SECONDS || calls>=1840 || tokens>=262144 {
        return Err(bad("depth eval resource cap"));
    }
    let (_,comparison)=parent(&p)?;let (_,_,_,tok,_)=inputs(&comparison)?;
    let device=Backend::Metal0.open()?;p.runtime.verify(&device)?;
    let (core,_)=load_endpoint(&ep,device)?;
    let cancel=Arc::new(AtomicBool::new(false));let signal=cancel.clone();
    ctrlc::set_handler(move||signal.store(true,Ordering::Relaxed)).map_err(|e|bad(&e.to_string()))?;
    let remaining=MAX_SECONDS-seconds-command_started.elapsed().as_secs_f64();
    if remaining<=0. {return Err(bad("depth model time cap"));}
    let mut ctl=RunControl::new(cancel,Duration::from_secs_f64(remaining.min(1800.)),16*1024*1024)?;
    ctl.set_call_limits(1840-calls,0);
    let mut generated=0usize;
    publish_confirmed(&dir.join(format!("eval-{step}-segment-{index:03}-entered.r3b")),&binary::record!({
        "policy":policy(&p,arm)?,"native":ep.hash,"step":step,"index":index,
        "calls_before":calls,"tokens_before":tokens,"seconds_before":seconds}))?;
    let result=(||->Result<()> {
        let ps=panels(&p,step)?;let mut total=0usize;
        for panel in &ps {
            let label=format!("eval-{step}-{}",panel.0);
            let source_raw=if step==998 {Some(file_hash(&dir.join("eval-512-word.r3rows"))?)}else{None};
            let binding=binary::record!({"policy":policy(&p,arm)?,"arm":arm,"native":ep.hash,
                "model":ep.content,"step":step,"cases":digest(&panel.1)?,"metadata":digest(&panel.2)?,
                "tokenizer":p.tokenizer_id,"source_raw":source_raw,"accuracy_denominator":step!=998});
            let rows=collect(&core,&tok,&dir,&label,&panel.1,&binding,&mut ctl,&mut generated,262144-tokens)?;
            if step==998 {
                let old=binary::read_value_records(&dir.join("eval-512-word.r3rows"))?;
                for (a,b) in rows.iter().zip(old.iter().skip(1)) {
                    for field in ["raw_tokens","raw_bytes","actual","finish_reason","error","generation_completed"] {
                        if a[field]!=b[field] {return Err(bad("depth B raw mismatch"));}
                    }
                }
            }
            let raw_hash=file_hash(&dir.join(format!("{label}.r3rows")))?;
            let mut value=panel_score(panel,&rows,&tok,&ep.content,&raw_hash)?;
            value["policy"]=binary::record!(policy(&p,arm)?);
            value["model"]=binary::record!(ep.content);
            value["raw_hash"]=binary::record!(raw_hash);
            publish_or_compare_score(&dir.join(format!("{label}-score.r3b")),&value)?;
            total+=rows.len();
        }
        let expected=if step==0 {16}else if step==128 {64}else if step==512 {832}else{8};
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
    let r=binary::record!({"policy":policy(&p,arm)?,"native":ep.hash,"step":step,
        "calls":ctl.receipt()["generation_calls"],"tokens":generated,
        "seconds":command_started.elapsed().as_secs_f64(),"stop":stop,"control":ctl.receipt()});
    publish_confirmed(&dir.join(format!("eval-{step}-segment-{index:03}-returned.r3b")),&r)?;
    println!("DEPTH_EVAL arm={arm} step={step} calls={} tokens={generated} seconds={} stop={stop}",r["calls"],r["seconds"]);
    result
}
fn panel_full(p: &Plan, arm: &str, step: usize, name: &str) -> Result<usize> {
    let s=score(p,arm,step,name)?;
    if name.starts_with("qa-") { score_count(&s,"full") } else {joint_count(&s,"full")}
}
fn initial_screen_value(p:&Plan)->Result<binary::Value>{
    for arm in ARMS {
        verified_eval_complete(p,arm,0)?;
        for name in ["word","renamed"] {score(p,arm,0,name)?;}
    }
    let mut checks=0usize;
    for name in ["word","renamed"] {
        let original=binary::read_value_records(&p.closed_root.join("C").join(format!("eval-512-{name}.r3rows")))?;
        let a=binary::read_value_records(&p.root.join("D6").join(format!("eval-0-{name}.r3rows")))?;
        let b=binary::read_value_records(&p.root.join("D8").join(format!("eval-0-{name}.r3rows")))?;
        if a.len()!=9||b.len()!=9||original.len()!=193{return Err(bad("depth baseline row count"));}
        for i in 0..8 {for field in ["raw_tokens","raw_bytes","actual","finish_reason","error","generation_completed"] {
            if a[i+1][field]!=b[i+1][field] || a[i+1][field]!=original[i+1][field] {
                return Err(bad("depth initial normal greedy/parent parity"));
            }
        }checks+=1;}
    }
    Ok(binary::record!({"policy":digest(p)?,"matched_cases_per_arm":checks,
        "generation_calls":32,"quality_inference":"NOT_EVALUATED","optimizer":0}))
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
        if mark["policy"]!=policy(p,arm)? || mark["native"]!=ep.hash
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
    let dir=p.root.join(arm);
    let mark:binary::Value=read_confirmed(&dir.join(format!("eval-{step}-complete.r3b")))?;
    let ps=panels(p,step)?;
    let expected=ps.iter().map(|v|v.1.len()).sum::<usize>();
    let (segments,calls,tokens,last)=eval_receipt_total(p,arm,step)?;
    let mut raw_tokens=0usize;
    for panel in &ps {
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
        || mark["policy"]!=policy(p,arm)? || mark["native"]!=endpoint_at(p,arm,step)?.hash
        || mark["rows"]!=expected || mark["calls"]!=calls || mark["tokens"]!=tokens
        || mark["segments"]!=segments || mark["stop"]!="COMPLETED" {
        return Err(bad("depth completed evaluation receipt"));
    }
    Ok(())
}
fn forecast_value_32(p:&Plan)->Result<binary::Value> {
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
fn cost_forecast_32(p:&Plan)->Result<()> {
    let value=forecast_value_32(p)?;
    println!("DEPTH_FORECAST32 used={} train_remaining={} eval_remaining={} projected={} continue={}",
        value["actual_used_seconds"],value["training_remaining_conservative_seconds"],
        value["evaluation_remaining_conservative_seconds"],value["projected_total_seconds"],value["continue"]);
    publish_confirmed(&p.root.join("cost-forecast-32.r3b"),&value)?;
    Ok(())
}
fn sentinel_value_128(p:&Plan)->Result<binary::Value> {
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
        println!("DEPTH_SCREEN0 pair_and_parent_matched={} calls=32 optimizer=0",value["matched_cases_per_arm"]);
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
        println!("DEPTH_SCREEN128 parent_full32={} collapse={}",value["parent_full32"],value["QUALITY_COLLAPSE_STOP"]);
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
    if r["contract"]!=CONTRACT || r["plan_hash"]!=file_hash(&p.root.join("plan.r3b"))?
        || r["source"]!=p.source || r["binary"]!=p.runtime.binary
        || r["verdict"]!="PASS" || r["raw_recount_rows"]!=1664
        || r["reproduction_rows"]!=16 || r["scores"]!=screen["scores"]
        || r["optimizer_calls"]!=0 || r["teacher_calls"]!=0 {
        return Err(bad("depth independent B identity/full recount"));
    }
    publish_confirmed(&p.root.join("review-b.r3b"),&binary::record!({
        "path":review.canonicalize()?,"hash":file_hash(review)?,"plan":file_hash(&p.root.join("plan.r3b"))?}))
}
fn verified_final(p: &Plan) -> Result<()> {
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
fn finish(root: &Path) -> Result<()> {
    let p=checked(root)?;admitted(&p)?;
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
        || b["raw_recount_rows"]!=1664 || b["reproduction_rows"]!=16 {
        return Err(bad("depth final independent B"));
    }
    let (seconds,calls,tokens)=usage(&p)?;
    let signal=recounted_signal && seconds<=MAX_SECONDS
        && calls<=1840 && tokens<=262144;
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
    publish_confirmed(&p.root.join("final-report.r3b"),&binary::record!({
        "contract":CONTRACT,"policy":digest(&p)?,"parent_global":PARENT_GLOBAL,
        "local_optimizer":512,"logical_total":PARENT_GLOBAL+512,"status":direction,
        "depth_followup_signal":signal,"D8_ADOPTED":false,"scores":screen["scores"],
        "independent_B":file_hash(path)?,"repair":p.correction_hash,
        "model_seconds":seconds,"generation_calls":calls,"generation_tokens":tokens,
        "MODEL_QUALITY_ACCEPTED":false,"GOAL1_ACCEPTED":false}))?;
    println!("DEPTH_FINAL status={direction} signal={signal} seconds={seconds:.3} calls={calls} tokens={tokens}");
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
    match action {
        Action::Prepare {closed, correction, output, target_baseline_kib} => prepare(&closed, &correction, &output, target_baseline_kib),
        Action::Inspect {root} => { let p = checked(&root)?; println!("DEPTH_INSPECT policy={} A={} parent=3584 local=0",digest(&p)?,p.root.join("review-a.r3b").exists()); Ok(()) },
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
    }
}

#[cfg(test)]
mod tests {
    use super::*;
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
