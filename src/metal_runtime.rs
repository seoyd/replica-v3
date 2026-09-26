//! Bounded Metal profile observations using the existing native readers and RunControl.
use super::*;
use neural::{Backend,RuntimeProfile};
use recovery::{RunControl,ObservedCall};

const CONTRACT:&str="R3-METAL-REDUCTION-REPAIR-1.0";
#[derive(Subcommand)]
pub enum Action {
    Prepare { #[arg(long)] protected:PathBuf, #[arg(long)] protected_root:PathBuf,
        #[arg(long)] parent:PathBuf, #[arg(long)] word_root:PathBuf,
        #[arg(long)] review_a:PathBuf, #[arg(long)] output:PathBuf },
    Observe { #[arg(long)] root:PathBuf, #[arg(long,value_enum)] device:Backend,
        #[arg(long,value_parser=["protected","challenge"])] panel:String },
    Report { #[arg(long)] root:PathBuf },
    PrepareExecution { #[arg(long)] root:PathBuf, #[arg(long)] worker:PathBuf },
    Worker { #[arg(long)] root:PathBuf, #[arg(long,value_enum)] device:Backend },
    Cache { #[arg(long)] root:PathBuf },
    Train { #[arg(long)] root:PathBuf, #[arg(long,value_parser=["cpu16","metal16","metal-split"])] arm:String,
        #[arg(long,value_parser=clap::value_parser!(u8).range(1..=16))] until:u8 },
    Endpoints { #[arg(long)] root:PathBuf },
    Benchmark { #[arg(long)] root:PathBuf, #[arg(long,value_enum)] device:Backend },
}
#[derive(Clone,Serialize,Deserialize)]
#[serde(deny_unknown_fields)]
struct Registration {
    contract:String,protected:PathBuf,parent:PathBuf,protected_root:PathBuf,word_root:PathBuf,
    protected_hash:String,parent_hash:String,protected_plan:String,word_plan:String,
    protected_corpus:String,word_corpus:String,review_a:String,root:PathBuf,
    cpu:RuntimeProfile,metal:RuntimeProfile,
}
fn prepare(protected:&Path,protected_root:&Path,parent:&Path,word_root:&Path,review_a:&Path,output:&Path)->Result<()> {
    let p:Plan=read(&protected_root.join("plan.r3b"))?;let w:Plan=read(&word_root.join("plan.r3b"))?;
    let pc=verified_corpus(&protected_root.join("corpus.r3cor"),&p.corpus)?;
    let wc=verified_corpus(&word_root.join("corpus.r3cor"),&w.corpus)?;
    if pc.validation.len()!=1536 || wc.validation.len()!=3456 || wc.train.len()!=7680
        || w.config.lr!=3e-5 || w.config.microbatch*w.config.accumulation!=8 || w.framing()!=neural::Framing::QuestionEvidence {
        return Err(bad("Metal validation frozen input contract"));
    }
    // Exact independently issued report for this numerical patch, not a substring
    // in a caller-supplied document or a runtime temporary instruction.
    if file_hash(review_a)?!="2c0ba8c72cfc59ca4aed35835ba33d0f33227bc272f4e71c2b00259a7c0087d7" {
        return Err(bad("independent Runtime A identity mismatch"));
    }
    let protected=std::fs::canonicalize(protected)?;let parent=std::fs::canonicalize(parent)?;
    let mut identities=Vec::new();
    for (path,step) in [(&protected,11264),(&parent,14336)] {
        let x=checkpoint::load(path,Device::Cpu,true)?;
        if x.manifest.trained_steps!=step || x.manifest.framing()?!=neural::Framing::QuestionEvidence
            || x.manifest.training.as_ref().and_then(|s|s.resume_binding.as_ref()).is_none_or(|b|b.family!=checkpoint::ANSWER_MEAN_FAMILY)
            || x.optimizer.is_empty() || x.model.config.profile.contains("tiny") {return Err(bad("registered parent native state"));}
        identities.push(file_hash(path)?);
    }
    if identities[0]!="c47e34c7ac4f88dd08b5e719bf4d0364f139ac5002bf4572fe55b37dac823925"
        || identities[1]!="15015900a3c91a6b8ed87e9c2ecb8f7faf124849e5dcf8f40203cddf267e30d9" {return Err(bad("unregistered backend parent"));}
    let gpu=Backend::Metal0.open()?;
    let cpu=RuntimeProfile::capture(Backend::Cpu,&Device::Cpu)?;
    let metal=RuntimeProfile::capture(Backend::Metal0,&gpu)?;
    std::fs::create_dir(output)?;
    let r=Registration{contract:CONTRACT.into(),protected,parent,protected_root:std::fs::canonicalize(protected_root)?,word_root:std::fs::canonicalize(word_root)?,
        protected_hash:identities[0].clone(),parent_hash:identities[1].clone(),protected_plan:file_hash(&protected_root.join("plan.r3b"))?,word_plan:file_hash(&word_root.join("plan.r3b"))?,
        protected_corpus:p.corpus,word_corpus:w.corpus,review_a:file_hash(review_a)?,root:std::fs::canonicalize(output)?,cpu,metal};
    publish_confirmed(&output.join("runtime-plan.r3b"),&r)?;
    println!("METAL_REGISTER protected11264 parent14336 generation0 optimizer0 plan={}",file_hash(&output.join("runtime-plan.r3b"))?);Ok(())
}
fn registered(root:&Path)->Result<Registration> {
    let r:Registration=read_confirmed(&root.join("runtime-plan.r3b"))?;
    if r.contract!=CONTRACT || std::fs::canonicalize(root)?!=r.root
        || file_hash(&r.protected)?!=r.protected_hash || file_hash(&r.parent)?!=r.parent_hash
        || file_hash(&r.protected_root.join("plan.r3b"))?!=r.protected_plan || file_hash(&r.word_root.join("plan.r3b"))?!=r.word_plan {
        return Err(bad("registered runtime/input binding changed"));
    }Ok(r)
}
fn cases(r:&Registration,panel:&str)->Result<Vec<Episode>> {
    if panel=="protected" {
        let c=verified_corpus(&r.protected_root.join("corpus.r3cor"),&r.protected_corpus)?;
        Ok(c.validation[..64].iter().chain(&c.validation[512..576]).cloned().collect())
    } else if panel=="challenge" {
        let c=verified_corpus(&r.word_root.join("corpus.r3cor"),&r.word_corpus)?;
        Ok(c.validation[2560..2592].iter().chain(&c.validation[3072..3104]).cloned().collect())
    } else {Err(bad("unregistered Metal panel"))}
}
fn observe(root:&Path,backend:Backend,panel:&str)->Result<()> {
    let r=registered(root)?;let es=cases(&r,panel)?;let device=backend.open()?;
    let profile=if backend==Backend::Cpu{&r.cpu}else{&r.metal};profile.verify(&device)?;
    let reference=if backend==Backend::Cpu && panel=="protected" {
        let mut raw=Vec::new();
        for name in ["value512","citation512"] {
            let rows=binary::read_value_records(&r.protected_root.join(format!("eval-11264-{name}.r3rows")))?;
            if rows.len()!=513{return Err(bad("protected historical raw incomplete"));}
            raw.extend(rows[1..65].iter().cloned());
        }
        for (e,row) in es.iter().zip(&raw){if row["id"]!=e.id||row["expected"]!=e.answer{return Err(bad("protected reference input mismatch"));}}
        Some(raw)
    } else {None};
    let label=format!("{}-{panel}",backend.id().replace(':',"-"));
    let start_path=root.join(format!("{label}-started.r3b"));
    // create_new means an interrupted observation cannot masquerade as a new run.
    publish_confirmed(&start_path,&binary::record!({"plan":file_hash(&root.join("runtime-plan.r3b"))?,"profile":profile,"cases":digest(&es)?,"planned":es.len()}))?;
    let mut control=RunControl::command(false)?;control.set_call_limits(es.len(),0);
    let mut raw=std::fs::OpenOptions::new().create_new(true).write(true).open(root.join(format!("{label}.r3rows")))?;
    let result=(||->Result<()> {
        control.check("before_backend_load")?;
        let load=Instant::now();let loaded=checkpoint::load(if panel=="protected"{&r.protected}else{&r.parent},device.clone(),false)?;
        device.synchronize()?;println!("LOAD panel={panel} backend={backend:?} seconds={}",load.elapsed().as_secs_f64());
        for (i,e) in es.iter().enumerate() {
            control.check("before_backend_generation")?;
            publish_confirmed(&root.join(format!("{label}-{i:03}-entered.r3b")),&binary::record!({"case":digest(e)?,"profile":profile,"state":"CALL_MAY_ENTER"}))?;
            device.synchronize()?;let now=Instant::now();
            let observed=recovery::observe_generation(&loaded,e,&e.request,&mut control,false);
            device.synchronize()?;
            let mut row=match observed {ObservedCall::NotInvoked(mut v)=>{v["invocation"]=binary::record!("NOT_INVOKED");v},ObservedCall::Returned(mut v)=>{v["invocation"]=binary::record!("RETURNED");v}};
            row["synchronized_seconds"]=binary::record!(now.elapsed().as_secs_f64());
            row["runtime"]=binary::record!(profile);row["model"]=binary::record!(if panel=="protected"{&r.protected_hash}else{&r.parent_hash});
            binary::write_value_record(&mut raw,&row)?;raw.sync_all()?;
            println!("GEN {label} {}/{} exact={} finish={} seconds={}",i+1,es.len(),row["exact_match"],row["finish_reason"],row["synchronized_seconds"]);
            if row["invocation"]!="RETURNED" || !row["error"].is_null() {return Err(bad("backend generation error; no retry"));}
            if let Some(old)=&reference {
                for field in ["raw_tokens","actual","finish_reason","provided","native_prompt_digest"] {
                    if row[field]!=old[i][field]{return Err(bad(&format!("CPU reference parity mismatch case={} field={field}",e.id)));}
                }
            }
            control.check("after_backend_generation")?;
        }control.seal_terminal()?;Ok(())
    })();
    if let Err(e)=&result {control.classify_error(e);}
    publish_confirmed(&root.join(format!("{label}-finished.r3b")),&binary::record!({"success":result.is_ok(),"control":control.receipt(),"rss_kib":control.last_rss_kib,"raw":file_hash(&root.join(format!("{label}.r3rows")))?,"error":result.as_ref().err().map(ToString::to_string)}))?;
    result
}
fn report(root:&Path)->Result<()> {
    let r=registered(root)?;let mut preservation=true;let mut totals=Vec::new();
    for panel in ["protected","challenge"] {
        let es=cases(&r,panel)?;let mut rows=Vec::new();
        for name in ["cpu","metal-0"] {
            let label=format!("{name}-{panel}");let f:binary::Value=read_confirmed(&root.join(format!("{label}-finished.r3b")))?;
            let path=root.join(format!("{label}.r3rows"));
            if f["success"]!=true || f["raw"]!=file_hash(&path)? {return Err(bad("backend observation incomplete"));}
            let v=binary::read_value_records(&path)?;
            if v.len()!=es.len(){return Err(bad("backend panel incomplete"));}
            for (e,row) in es.iter().zip(&v) {
                if row["id"]!=e.id || row["expected"]!=e.answer || row["invocation"]!="RETURNED" {return Err(bad("backend raw binding"));}
            }rows.push(v);
        }
        let exact=|row:&binary::Value,e:&Episode|row["actual"]==e.answer&&row["finish_reason"]=="stop"&&row["error"].is_null();
        let mut lost=0;let mut cpu=0;let mut metal=0;let mut different=0;
        for ((e,c),m) in es.iter().zip(&rows[0]).zip(&rows[1]) {
            let c_ok=exact(c,e);let m_ok=exact(m,e);cpu+=usize::from(c_ok);metal+=usize::from(m_ok);
            lost+=usize::from(c_ok&&!m_ok);different+=usize::from(c["raw_tokens"]!=m["raw_tokens"]);
        }
        if panel=="protected"&&lost>0{preservation=false;}
        totals.push(binary::record!({"panel":panel,"planned":es.len(),"cpu":cpu,"metal":metal,"lost_cpu_correct":lost,"different_raw":different}));
    }
    let out=binary::record!({"contract":CONTRACT,"panels":totals,"protected_preserved":preservation,"SMALL_updates":0,"GENERAL_QA_IMPROVED":"NOT_ESTABLISHED","GOAL1_ACCEPTED":false});
    publish_confirmed(&root.join("generation-comparison.r3b"),&out)?;println!("{out}");
    if !preservation{return Err(bad("PROTECTED_QUALITY_FAIL; SMALL48 blocked"));}Ok(())
}

#[derive(Serialize,Deserialize)]
#[serde(deny_unknown_fields)]
struct Execution {
    parent_plan:String, generation_proof:String, cpu:RuntimeProfile, metal:RuntimeProfile,
    worker:PathBuf,worker_hash:String,
}
fn prepare_execution(root:&Path,worker:&Path)->Result<()> {
    let r=registered(root)?;
    let proof:binary::Value=read_confirmed(&root.join("generation-comparison.r3b"))?;
    if proof["protected_preserved"]!=true{return Err(bad("protected quality required"));}
    let gpu=Backend::Metal0.open()?;
    let cpu=RuntimeProfile::capture(Backend::Cpu,&Device::Cpu)?;let metal=RuntimeProfile::capture(Backend::Metal0,&gpu)?;
    // A new executable is declared explicitly for this later phase. Kernel,
    // dtype, lock, device and OS must match the completed observation phase.
    for (old,new) in [(&r.cpu,&cpu),(&r.metal,&metal)] {
        let mut expected=old.clone();expected.binary=new.binary.clone();
        if expected!=*new{return Err(bad("unreviewed runtime change after protected output"));}
    }
    let e=Execution{parent_plan:file_hash(&root.join("runtime-plan.r3b"))?,generation_proof:file_hash(&root.join("generation-comparison.r3b"))?,cpu,metal,
        worker:std::fs::canonicalize(worker)?,worker_hash:file_hash(worker)?};
    publish_confirmed(&root.join("execution-plan.r3b"),&e)?;
    println!("EXECUTION_REGISTER updates48 maximum; cpu16/metal16/metal1+15; parent={}",r.parent_hash);Ok(())
}
fn execution(root:&Path,backend:Backend,device:&Device)->Result<Execution> {
    let e:Execution=read_confirmed(&root.join("execution-plan.r3b"))?;
    if e.parent_plan!=file_hash(&root.join("runtime-plan.r3b"))? || e.generation_proof!=file_hash(&root.join("generation-comparison.r3b"))?
        || e.worker_hash!=file_hash(&e.worker)? {return Err(bad("execution lineage changed"));}
    (if backend==Backend::Cpu{&e.cpu}else{&e.metal}).verify(device)?;Ok(e)
}
fn worker(root:&Path,backend:Backend)->Result<()> {
    use replica_v3::model::{run_worker,run_worker_observed,verify_prepared,native_revision};
    let r=registered(root)?;let device=backend.open()?;let e=execution(root,backend,&device)?;
    let name=format!("worker-{}",backend.id().replace(':',"-"));
    publish_confirmed(&root.join(format!("{name}-started.r3b")),&binary::record!({"worker":e.worker_hash,"planned":4}))?;
    let mut control=execution_control(root,false)?;control.set_call_limits(4,0);
    let (manifest,tok)=checkpoint::metadata(&r.protected)?;
    let es=cases(&r,"protected")?;let mut rows=Vec::new();
    let result=(||->Result<()> { for (i,episode) in es.iter().take(4).enumerate() {
        control.check("before_worker")?;
        let mut request=episode.request.clone();request.request_id=format!("{name}/{i}");
        let mut command=std::process::Command::new(&e.worker);
        command.arg("__model-worker").arg("--device").arg(backend.id()).arg("--checkpoint").arg(&r.protected);
        let cancel=control.cancellation();let deadline=control.deadline();
        let response=run_worker_observed(command,&request,&cancel,replica_v3::model::LOAD_TIMEOUT,deadline,||{
            publish_confirmed(&root.join(format!("{name}-{i}-entered.r3b")),&binary::record!({"request":digest(&request)?,"state":"MAY_ENTER"}))?;
            control.begin_external_generation()
        })?;
        control.returned_external_generation();
        publish_confirmed(&root.join(format!("{name}-{i}-returned.r3b")),&response)?;
        let prompt=tok.prepare(&request,manifest.architecture.context as u32,&manifest.architecture.semantic_id()?)?;
        verify_prepared(&request,&prompt,&response)?;
        if response.generation.runtime_revision!=backend.runtime_revision() || response.generation.model_revision!=native_revision(&manifest,&tok)?
            || response.text!=episode.answer || response.generation.finish_reason!="stop" {return Err(bad("worker generation/identity parity"));}
        rows.push(response);
    }
    // Both stops occur before the parent sends the request frame: model calls0.
    for cancelled in [true,false] {
        let mut command=std::process::Command::new(&e.worker);
        command.arg("__model-worker").arg("--device").arg(backend.id()).arg("--checkpoint").arg(&r.protected);
        let failure=run_worker(command,&es[0].request,&AtomicBool::new(cancelled),std::time::Duration::ZERO).unwrap_err();
        if cancelled {if !matches!(failure,Error::Cancelled){return Err(bad("worker cancel"));}}
        else if !failure.to_string().contains("worker timeout"){return Err(bad("worker zero load deadline"));}
    }
    control.seal_completed_no_call()?;Ok(())})();
    if let Err(error)=&result{control.classify_error(error);}
    publish_confirmed(&root.join(format!("{name}-finished.r3b")),&binary::record!({"success":result.is_ok(),"generations":control.receipt()["generation_calls"],"returned":control.receipt()["completed_generation_count"],"responses":rows,"error":result.as_ref().err().map(ToString::to_string),"control":control.receipt()}))?;
    result?;println!("WORKER backend={backend:?} generations4 cancel/timeout before_request0 PASS");Ok(())
}
fn pointwise(reference:&Tensor,actual:&Tensor)->Result<(f64,f64)> {
    if reference.dims()!=actual.dims(){return Err(bad("cache numerical shape"));}
    let a=reference.flatten_all()?.to_vec1::<f32>()?;let b=actual.flatten_all()?.to_vec1::<f32>()?;
    let mut max=0f64;let mut square=0f64;
    for (&x,&y) in a.iter().zip(&b){let delta=(x as f64-y as f64).abs();
        if !x.is_finite()||!y.is_finite()||delta>1e-4+1e-3*(x as f64).abs(){return Err(bad(&format!("cache numeric mismatch expected={x} actual={y}")));}
        max=max.max(delta);square+=delta*delta;
    }Ok((max,(square/a.len()as f64).sqrt()))
}
fn cache(root:&Path)->Result<()> {
    let r=registered(root)?;let device=Backend::Metal0.open()?;execution(root,Backend::Metal0,&device)?;
    publish_confirmed(&root.join("cache-started.r3b"),&binary::record!({"parent":r.parent_hash,"lengths":[1,127,128,255,256,257,512,2048]}))?;
    let mut control=execution_control(root,false)?;
    let loaded=checkpoint::load(&r.parent,device.clone(),false)?;let model=&loaded.model;
    let ids:Vec<u32>=(0..2048).map(|i|8+(i%(loaded.tokenizer.vocab_size()-8))as u32).collect();
    let mut token_cache=model.cache("token" );let mut full_at=BTreeMap::new();let mut stats=Vec::new();let mut forwards=0;
    for len in [1,127,128,255,256,257,512,2048] {
        control.check("before_full_cache_reference")?;
        let tensor=Tensor::new(&ids[..len],&device)?.unsqueeze(0)?;
        forwards+=1;let full=model.forward(&tensor,None)?;device.synchronize()?;
        let mut chunk_cache=model.cache("chunk");let mut parts=Vec::new();
        for part in ids[..len].chunks(128){control.check("chunk_cache")?;forwards+=1;parts.push(model.forward_cached(&Tensor::new(part,&device)?.unsqueeze(0)?,&mut chunk_cache,"chunk")?);}
        let chunk=Tensor::cat(&parts,1)?;device.synchronize()?;
        let errors=pointwise(&full,&chunk)?;
        full_at.insert(len,full.narrow(1,len-1,1)?.contiguous()?);
        stats.push(binary::record!({"length":len,"chunk_max":errors.0,"chunk_rms":errors.1,"retained":chunk_cache.retained_tokens(),"bytes":chunk_cache.bytes()}));
        println!("CACHE full/chunk len={len} max={} rms={}",errors.0,errors.1);
    }
    for (pos,id) in ids.iter().enumerate(){control.check("token_cache")?;
        forwards+=1;let got=model.forward_cached(&Tensor::new(&[[*id]],&device)?,&mut token_cache,"token")?;
        if let Some(expected)=full_at.get(&(pos+1)){device.synchronize()?;let errors=pointwise(expected,&got)?;println!("CACHE token len={} max={} rms={}",pos+1,errors.0,errors.1);}
    }
    control.seal_terminal()?;publish_confirmed(&root.join("cache-finished.r3b"),&binary::record!({"success":true,"small_diagnostic_forwards":forwards,"generation":0,"backward":0,"stats":stats,"control":control.receipt()}))?;Ok(())
}
fn vector_metrics(a:&Tensor,b:&Tensor,limit:f64,zero:f64,cosine:bool)->Result<binary::Value> {
    if a.dims()!=b.dims(){return Err(bad("backend vector shape"));}
    let a=a.flatten_all()?.to_vec1::<f32>()?;let b=b.flatten_all()?.to_vec1::<f32>()?;
    if a.iter().chain(&b).any(|x|!x.is_finite()){return Err(bad("nonfinite backend vector"));}
    let norm=a.iter().map(|&x|(x as f64).powi(2)).sum::<f64>().sqrt();let bn=b.iter().map(|&x|(x as f64).powi(2)).sum::<f64>().sqrt();
    let floor=1e-6*(a.len()as f64).sqrt();let mut errors=a.iter().zip(&b).map(|(&x,&y)|(x as f64-y as f64).abs()).collect::<Vec<_>>();
    let l2=errors.iter().map(|x|x*x).sum::<f64>().sqrt();errors.sort_by(f64::total_cmp);let max=*errors.last().ok_or_else(||bad("empty numeric vector"))?;
    let cos=if norm>floor&&bn>floor{Some(a.iter().zip(&b).map(|(&x,&y)|x as f64*y as f64).sum::<f64>()/(norm*bn))}else{None};
    let pass=if norm<=floor{max<=zero}else{l2/norm<=limit&&(!cosine||cos.is_some_and(|x|x>=0.999))};
    let out=binary::record!({"norm":norm,"near_zero_floor":floor,"nrmse":l2/norm.max(floor),"max":max,"p99":errors[(errors.len()-1)*99/100],"rms":l2/(a.len()as f64).sqrt(),"cosine":cos,"pass":pass});
    if !pass{return Err(bad(&format!("backend numeric gate {out}")));}Ok(out)
}
fn training_binding(r:&Registration,e:&Execution,arm:&str,state:&TrainingState,tok:&ByteBpe,tape:&[[usize;8]])->Result<checkpoint::ResumeBinding> {
    let mut b=checkpoint::ResumeBinding::default_for(state,tok);
    b.family=checkpoint::ANSWER_MEAN_FAMILY;b.normalizer=2;b.execution=1;b.framing=neural::Framing::QuestionEvidence.digest();
    b.policy=checkpoint::ResumeBinding::digest_bytes(&binary::to_vec(&(r,e,arm))?);b.provenance=b.policy;
    b.train_order=checkpoint::ResumeBinding::digest_bytes(&binary::to_vec(tape)?);Ok(b)
}
fn train(root:&Path,arm:&str,until:usize)->Result<()> {
    let backend=if arm=="cpu16"{Backend::Cpu}else{Backend::Metal0};let r=registered(root)?;let device=backend.open()?;
    let e=execution(root,backend,&device)?;
    for file in ["worker-cpu-finished.r3b","worker-metal-0-finished.r3b","cache-finished.r3b"] {
        let gate:binary::Value=read_confirmed(&root.join(file))?;if gate["success"]!=true{return Err(bad("runtime prerequisites incomplete"));}
    }
    let (from,to)=match (arm,until) {("cpu16",16)|("metal16",16)=>(0,16),("metal-split",1)=>(0,1),("metal-split",16)=>(1,16),_=>return Err(bad("unregistered SMALL update segment"))};
    if arm!="cpu16" {let prior:binary::Value=read_confirmed(&root.join("cpu16/segment-0-16-finished.r3b"))?;if prior["success"]!=true{return Err(bad("CPU reference execution failed"));}}
    if arm=="metal-split" {let prior:binary::Value=read_confirmed(&root.join("metal16/segment-0-16-finished.r3b"))?;if prior["success"]!=true{return Err(bad("Metal continuous execution failed"));}}
    let plan:Plan=read(&r.word_root.join("plan.r3b"))?;let c=verified_corpus(&r.word_root.join("corpus.r3cor"),&r.word_corpus)?;
    let tape=plan.identifiable.as_ref().ok_or_else(||bad("word tape absent"))?.rows.get(plan.origin_step()..plan.origin_step()+16).ok_or_else(||bad("word tape range"))?;
    let dir=root.join(arm);let label=format!("segment-{from}-{to}");
    if dir.join(format!("{label}-started.r3b")).exists() {
        return finalize_training(&dir,&label,from,to);
    }
    if from==0 {std::fs::create_dir(&dir)?;}
    let path=if from==0{r.parent.clone()}else{dir.join("step-1.r3m")};
    let mut loaded=checkpoint::load(&path,device.clone(),true)?;let mut state=loaded.manifest.training.clone().ok_or_else(||bad("backend Adam state absent"))?;
    if from==0 {
        if state.step!=14336 || state.config.lr!=3e-5{return Err(bad("backend parent clock/LR"));}
        state.parent_checkpoint_hash=Some(r.parent_hash.clone());
        state.config.budget_start_step=14336;state.config.max_steps=14352;state.config.warmup=0;
        state.config.budget_start_tokens=state.consumed_tokens;state.config.max_tokens=state.consumed_tokens+20_000_000;
        state.config.microbatch=8;state.config.accumulation=1;state.config.seq_len=plan.config.seq_len;
        state.corpus_hash=c.manifest.train.sha256.clone();state.validation_hash=c.manifest.validation.sha256.clone();
        if !state.previous_corpora.contains(&loaded.tokenizer.train_hash){state.previous_corpora.push(loaded.tokenizer.train_hash.clone());}
        state.resume_binding=Some(training_binding(&r,&e,arm,&state,&loaded.tokenizer,tape)?);
    } else {
        let prior:binary::Value=read_confirmed(&dir.join("segment-0-1-finished.r3b"))?;
        if prior["success"]!=true||prior["checkpoint"]!=file_hash(&path)?||state.step!=14337
            || state.resume_binding!=Some(training_binding(&r,&e,arm,&state,&loaded.tokenizer,tape)?) {return Err(bad("native backend resume mismatch"));}
    }
    state.config.validate(loaded.model.config.context)?;
    if state.resume_binding.as_ref().is_none_or(|b|b.family!=checkpoint::ANSWER_MEAN_FAMILY)||loaded.model.vars.values().any(|v|v.dtype()!=DType::F32||!v.device().same_device(&device)) {
        return Err(bad("backend profile/precision/objective admission"));
    }
    let ss=samples_with_framing(&c.train,&loaded.tokenizer,plan.config.seq_len,neural::Framing::QuestionEvidence)?;
    let mut adam=Adam{moments:std::mem::take(&mut loaded.optimizer)};
    if adam.moments.is_empty()||adam.moments.values().any(|v|v.dtype()!=DType::F32||!v.device().same_device(&device)){return Err(bad("backend Adam admission"));}
    publish_confirmed(&dir.join(format!("{label}-started.r3b")),&binary::record!({"native":file_hash(&path)?,"policy":state.resume_binding,"optimizer_limit":to-from,"from":from,"to":to}))?;
    let mut control=execution_control(root,true)?;control.set_call_limits(0,0);
    let mut raw=std::fs::OpenOptions::new().create_new(true).write(true).open(dir.join(format!("{label}.r3rows")))?;
    let mut last_saved=None;let mut updates=0;
    let result=(||->Result<()> {
        for cursor in from..to {
            control.check("before_backend_training")?;
            let b=batch(&ss,&tape[cursor],&device)?;device.synchronize()?;let now=Instant::now();
            publish_confirmed(&dir.join(format!("step-{}-entered.r3b",cursor+1)),&binary::record!({"clock":state.step+1,"indices":tape[cursor],"state":"MAY_ENTER"}))?;
            let logits=loaded.model.forward(&b.input,Some(&b.valid))?;
            let (_,loss,target,examples)=response_objective(&logits,&b,1.,true)?;
            let scalar=loss.to_scalar::<f32>()?;if !scalar.is_finite(){return Err(bad("nonfinite backend loss"));}
            let graph=loss.backward()?;let mut grads=BTreeMap::new();
            for (name,var) in &loaded.model.vars {grads.insert(name.clone(),graph.get(var).ok_or_else(||bad("disconnected backend gradient"))?.detach());}
            let mut first_reference=None;let mut diagnostic_seconds=0.;
            if cursor==0 && backend==Backend::Metal0 {
                let diagnostic_start=Instant::now();
                publish_confirmed(&dir.join("first-reference-entered.r3b"),&binary::record!({"model":r.parent_hash,"planned_forward":1,"planned_backward":1}))?;
                let reference=checkpoint::load(&r.parent,Device::Cpu,false)?;
                let rb=batch(&ss,&tape[cursor],&Device::Cpu)?;
                let rl=reference.model.forward(&rb.input,Some(&rb.valid))?;
                let (_,ro,_,_)=response_objective(&rl,&rb,1.,true)?;
                let expected=ro.to_scalar::<f32>()? as f64;
                if (expected-scalar as f64).abs()>1e-5+1e-3*expected.abs(){return Err(bad("SMALL first CE gate"));}
                let rg=ro.backward()?;let mut metrics=BTreeMap::new();
                for (name,v) in &reference.model.vars {metrics.insert(name.clone(),vector_metrics(rg.get(v).ok_or_else(||bad("reference gradient disconnected"))?,&grads[name],1e-2,1e-6,true)?);}
                publish_confirmed(&dir.join("first-gradient.r3b"),&binary::record!({"metrics":metrics,"additional_SMALL_diagnostic_forward":1,"additional_SMALL_diagnostic_backward":1}))?;
                first_reference=Some((reference,checkpoint::load(&root.join("cpu16/step-1.r3m"),Device::Cpu,true)?));
                diagnostic_seconds=diagnostic_start.elapsed().as_secs_f64();
            }
            control.check("before_backend_optimizer")?;
            // Sole production caller below ordinary admission: exact registered
            // runtime + native parent/objective/tape + fixed16 cap verified above.
            let (norm,delta)=adam.apply_admitted_step(&loaded.model.vars,&grads,&state.config,state.step+1,3e-5,|_,_,_,_|Ok(()))?;
            device.synchronize()?;updates+=1;state.step+=1;state.consumed_tokens+=b.tokens as u64;state.target_tokens+=target as u64;
            state.sampler_state=(cursor+1)as u64;state.train_loss=Some(scalar as f64);
            if let Some((before,after))=first_reference {
                let mut metrics=BTreeMap::new();
                for (name,v) in &before.model.vars {
                    let reference=(after.model.vars[name].as_tensor()-v.as_tensor())?;
                    let actual=(&loaded.model.vars[name].to_device(&Device::Cpu)?-v.as_tensor())?;
                    metrics.insert(format!("delta/{name}"),vector_metrics(&reference,&actual,2e-2,1e-7,false)?);
                }
                for (name,v) in &after.optimizer {metrics.insert(name.clone(),vector_metrics(v,&adam.moments[name],1e-2,1e-6,false)?);}
                publish_confirmed(&dir.join("first-update.r3b"),&metrics)?;
            }
            let row=binary::record!({"step":state.step,"cursor":cursor+1,"input":b.tokens,"target":target,"examples":examples,"loss":scalar,"lr":3e-5,"gradient_norm":norm,"clip":(state.config.clip/(norm+1e-12)).min(1.),"delta_norm":delta,"synchronized_seconds":now.elapsed().as_secs_f64(),"reference_diagnostic_seconds":diagnostic_seconds});
            binary::write_value_record(&mut raw,&row)?;raw.sync_all()?;println!("UPDATE {arm} {row}");
            if [1,16].contains(&(cursor+1)) {
                loaded.manifest.training=Some(state.clone());loaded.manifest.trained_steps=state.step;loaded.manifest.status="DIAGNOSTIC_COMPLETE".into();
                let saved=dir.join(format!("step-{}.r3m",cursor+1));
                checkpoint::save(&saved,&loaded.model,&loaded.tokenizer,loaded.manifest.clone(),&adam.moments)?;last_saved=Some(file_hash(&saved)?);
            }
            if cursor+1<to {control.check("after_backend_optimizer")?;}
        }control.seal_completed_no_call()?;Ok(())
    })();
    if let Err(error)=&result{control.classify_error(error);}
    let final_record=binary::record!({"success":result.is_ok(),"updates":updates,"step":state.step,"checkpoint":last_saved,"control":control.receipt(),"rss_kib":control.last_rss_kib,"error":result.as_ref().err().map(ToString::to_string)});
    if result.is_ok() {
        publish_confirmed(&dir.join(format!("{label}-completed.r3b")),&binary::record!({"record":final_record,"start":file_hash(&dir.join(format!("{label}-started.r3b")))?,"raw":file_hash(&dir.join(format!("{label}.r3rows")))?}))?;
        finalize_training(&dir,&label,from,to)
    } else {
        // Save the known current state on cooperative failure; an unknown
        // in-flight optimizer error is never granted resume authorization.
        if updates>0 {
            loaded.manifest.training=Some(state.clone());loaded.manifest.trained_steps=state.step;loaded.manifest.status="DIAGNOSTIC_FAILED".into();
            checkpoint::save(&dir.join(format!("{label}-stopped.r3m")),&loaded.model,&loaded.tokenizer,loaded.manifest.clone(),&adam.moments)?;
        }
        publish_confirmed(&dir.join(format!("{label}-finished.r3b")),&final_record)?;result
    }
}
fn finalize_training(dir:&Path,label:&str,from:usize,to:usize)->Result<()> {
    let complete:binary::Value=read_confirmed(&dir.join(format!("{label}-completed.r3b")))?;
    let f=&complete["record"];
    let start_path=dir.join(format!("{label}-started.r3b"));let start:binary::Value=read_confirmed(&start_path)?;
    let raw=dir.join(format!("{label}.r3rows"));let rows=binary::read_value_records(&raw)?;
    let path=dir.join(format!("step-{to}.r3m"));let (native,_)=checkpoint::metadata(&path)?;
    let state=native.training.as_ref().ok_or_else(||bad("completed native state absent"))?;
    let conditions=f["control"]["observed_conditions"].as_array().ok_or_else(||bad("completed command conditions absent"))?;
    if f["success"]!=true || !f["error"].is_null() || conditions.iter().any(|x|x!="TIME_BUDGET")
        || complete["start"]!=file_hash(&start_path)? || complete["raw"]!=file_hash(&raw)?
        || f["checkpoint"]!=file_hash(&path)? || rows.len()!=to-from || f["updates"]!=to-from
        || start["from"]!=from || start["to"]!=to || f["step"]!=14336+to
        || state.step!=14336+to || state.sampler_state!=to as u64 || start["policy"]!=binary::record!(state.resume_binding)
        || rows.iter().enumerate().any(|(i,r)|r["step"]!=14336+from+i+1 || r["cursor"]!=from+i+1 || r["loss"].as_f64().is_none_or(|x|!x.is_finite())) {
        return Err(bad("completed backend work incomplete/ambiguous"));
    }
    let path=dir.join(format!("{label}-finished.r3b"));
    if path.exists() {if read_confirmed::<binary::Value>(&path)?!=*f{return Err(bad("sticky backend terminal mismatch"));}}
    else {publish_confirmed(&path,f)?;}
    println!("BACKEND_FINALIZED {label} durable_step={} new_optimizer0 new_generation0",state.step);Ok(())
}
fn endpoint_cases(r:&Registration)->Result<Vec<Episode>> {
    let c=verified_corpus(&r.word_root.join("corpus.r3cor"),&r.word_corpus)?;
    Ok([0,512,2560,3072].into_iter().flat_map(|n|c.validation[n..n+4].iter().cloned()).collect())
}
fn tensor_digest(vars:impl Iterator<Item=(String,Tensor)>)->Result<String> {
    use sha2::{Digest,Sha256};let mut hash=Sha256::new();
    for (name,t) in vars {hash.update(name.as_bytes());hash.update([0]);for v in t.flatten_all()?.to_vec1::<f32>()?{hash.update(v.to_bits().to_le_bytes());}}
    Ok(format!("{:x}",hash.finalize()))
}
fn endpoints(root:&Path)->Result<()> {
    let r=registered(root)?;let device=Backend::Metal0.open()?;execution(root,Backend::Metal0,&device)?;
    let es=endpoint_cases(&r)?;let mut identities=Vec::new();
    // Validate complete receipts and SAME Metal state before spending generation.
    for arm in ["cpu16","metal16","metal-split"] {
        let segment=if arm=="metal-split"{"segment-1-16"}else{"segment-0-16"};
        let end:binary::Value=read_confirmed(&root.join(arm).join(format!("{segment}-finished.r3b")))?;
        let path=root.join(arm).join("step-16.r3m");
        if end["success"]!=true || end["checkpoint"]!=file_hash(&path)?{return Err(bad("backend endpoint incomplete"));}
        let x=checkpoint::load(&path,Device::Cpu,true)?;let s=x.manifest.training.as_ref().ok_or_else(||bad("endpoint state"))?;
        identities.push(binary::record!({"arm":arm,"step":s.step,"cursor":s.sampler_state,"input_tokens":s.consumed_tokens,"target_tokens":s.target_tokens,
            "weights":x.model.weights_content_id()?,"adam":tensor_digest(x.optimizer.into_iter())?}));
    }
    for field in ["step","cursor","input_tokens","target_tokens","weights","adam"] {
        if identities[1][field]!=identities[2][field]{publish_confirmed(&root.join("restart-failed.r3b"),&binary::record!({"field":field,"states":identities}))?;return Err(bad("SAME_METAL_RESTART_DIFFERENCE; no endpoint generation"));}
    }
    publish_confirmed(&root.join("restart-verified.r3b"),&identities)?;
    let mut control=execution_control(root,false)?;control.set_call_limits(48,0);
    for (arm,backend) in [("cpu16",Backend::Cpu),("metal16",Backend::Metal0),("metal-split",Backend::Metal0)] {
        let device=backend.open()?;execution(root,backend,&device)?;
        publish_confirmed(&root.join(format!("{arm}-generation-started.r3b")),&binary::record!({"planned":16,"cases":digest(&es)?}))?;
        let x=checkpoint::load(&root.join(arm).join("step-16.r3m"),device.clone(),false)?;
        let mut f=std::fs::OpenOptions::new().create_new(true).write(true).open(root.join(format!("{arm}-generation.r3rows")))?;
        let mut full=0;
        for e in &es {
            control.check("before_endpoint_generation")?;
            let row=recovery::evaluate_one_policy(&x,e,&e.request,&mut control,false);device.synchronize()?;
            binary::write_value_record(&mut f,&row)?;f.sync_all()?;
            if !row["error"].is_null(){return Err(bad("endpoint generation failed"));}
            full+=usize::from(row["exact_match"]==true);
        }
        publish_confirmed(&root.join(format!("{arm}-generation-finished.r3b")),&binary::record!({"full":full,"planned":16,"control":control.receipt()}))?;
        println!("ENDPOINT {arm} full={full}/16");
    }control.seal_terminal()?;Ok(())
}
fn benchmark(root:&Path,backend:Backend)->Result<()> {
    let r=registered(root)?;let device=backend.open()?;execution(root,backend,&device)?;
    let es=endpoint_cases(&r)?;let es=[0,1,4,5,8,9,12,13].map(|i|es[i].clone());
    let label=format!("benchmark-{}",backend.id().replace(':',"-"));
    publish_confirmed(&root.join(format!("{label}-started.r3b")),&binary::record!({"cases":digest(&es)?,"warmup_per_case":1,"measured_per_case":3,"parent":r.parent_hash}))?;
    let mut control=execution_control(root,false)?;control.set_call_limits(32,0);
    let now=Instant::now();let loaded=checkpoint::load(&r.parent,device.clone(),false)?;device.synchronize()?;let load_seconds=now.elapsed().as_secs_f64();
    let mut f=std::fs::OpenOptions::new().create_new(true).write(true).open(root.join(format!("{label}.r3rows")))?;
    let mut times=Vec::new();let mut per_token=Vec::new();
    for round in 0..4 {for e in &es {
        control.check("before_timed_generation")?;device.synchronize()?;let now=Instant::now();
        let observed=recovery::observe_generation_profiled(&loaded,e,&e.request,&mut control);
        device.synchronize()?;let seconds=now.elapsed().as_secs_f64();
        let mut row=match observed {ObservedCall::Returned(v)=>v,ObservedCall::NotInvoked(v)=>{binary::write_value_record(&mut f,&v)?;f.sync_all()?;return Err(bad("benchmark not invoked"));}};
        row["synchronized_seconds"]=binary::record!(seconds);row["round"]=binary::record!(round);
        binary::write_value_record(&mut f,&row)?;f.sync_all()?;
        if !row["error"].is_null(){return Err(bad("timed generation failed"));}
        let n=row["raw_generated_count"].as_u64().ok_or_else(||bad("timed token count"))?;
        if round>0 {times.push(seconds);per_token.push(1000.*seconds/n.max(1)as f64);}
        println!("TIMED {label} round={round} seconds={seconds} tokens={n}");
    }}
    times.sort_by(f64::total_cmp);per_token.sort_by(f64::total_cmp);control.seal_terminal()?;
    publish_confirmed(&root.join(format!("{label}-finished.r3b")),&binary::record!({"n":24,"generations":32,"load_seconds":load_seconds,"median_seconds":(times[11]+times[12])/2.,"p95_seconds":times[22],"range_seconds":[times[0],times[23]],"median_ms_per_actual_token":(per_token[11]+per_token[12])/2.,"rss_kib":control.last_rss_kib,"gpu_allocator_peak":"UNKNOWN","shader_preparation":"included in warmup; not isolated","canonicalization_model_copy_bytes":"UNKNOWN; G1 exact temporary24576/copy24576 bytes","control":control.receipt()}))?;Ok(())
}
const EXECUTION_ACTIONS:[&str;10]=["worker-cpu","worker-metal-0","cache","cpu16-16","metal16-16","metal-split-1","metal-split-16","endpoints","benchmark-cpu","benchmark-metal-0"];
fn previous_active(root:&Path,current:&str)->Result<f64> {
    // Reserve (not claim as measured) 120 seconds for all preceding tiny/numeric
    // tests, whose retained test elapsed totals are below this bound.
    let mut used=120.;
    for label in ["cpu-protected","metal-0-protected","cpu-challenge","metal-0-challenge"] {
        let f:binary::Value=read_confirmed(&root.join(format!("{label}-finished.r3b")))?;
        let seconds=f["control"]["elapsed_seconds"].as_f64().ok_or_else(||bad("prior elapsed UNKNOWN"))?;
        if f["success"]!=true || !seconds.is_finite() || seconds<0. {return Err(bad("prior runtime failed/usage UNKNOWN"));}used+=seconds;
    }
    for label in EXECUTION_ACTIONS {
        let start=root.join(format!("{label}-budget-started.r3b"));
        if label==current {continue;}
        if start.exists()||pending_path(&start).exists() {
            let _:binary::Value=read_confirmed(&start)?;
            let f:binary::Value=read_confirmed(&root.join(format!("{label}-budget-finished.r3b")))?;
            let seconds=f["elapsed_seconds"].as_f64().ok_or_else(||bad("execution elapsed UNKNOWN"))?;
            if f["success"]!=true || !seconds.is_finite() || seconds<0. {return Err(bad("previous execution failed/usage UNKNOWN"));}used+=seconds;
        }
    }Ok(used)
}
fn execution_control(root:&Path,training:bool)->Result<RunControl> {
    let mut active=None;
    for label in EXECUTION_ACTIONS {
        let p=root.join(format!("{label}-budget-started.r3b"));
        if p.exists()&&!root.join(format!("{label}-budget-finished.r3b")).exists() {
            if active.is_some(){return Err(bad("multiple active backend commands"));}
            active=Some(read_confirmed::<binary::Value>(&p)?);
        }
    }
    let v=active.ok_or_else(||bad("backend command budget not registered"))?;
    let deadline=v["deadline_unix_seconds"].as_f64().ok_or_else(||bad("backend command deadline"))?;
    let now=std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_err(|_|bad("system clock"))?.as_secs_f64();
    let remaining=deadline-now;
    if remaining>900. {return Err(bad("backwards command clock"));}
    let mut control=RunControl::command(training)?;control.restrict_seconds(remaining)?;Ok(control)
}
pub fn run(action:Action)->Result<()> {
    let bounded=match &action {
        Action::Worker{root,device}=>Some((root.clone(),format!("worker-{}",device.id().replace(':',"-")))),
        Action::Cache{root}=>Some((root.clone(),"cache".into())),
        Action::Train{root,arm,until}=>Some((root.clone(),format!("{arm}-{until}"))),
        Action::Endpoints{root}=>Some((root.clone(),"endpoints".into())),
        Action::Benchmark{root,device}=>Some((root.clone(),format!("benchmark-{}",device.id().replace(':',"-")))),_=>None,
    };
    if let Some((root,label))=&bounded {
        // Idempotent zero-call close only, never a second optimizer attempt.
        if root.join(format!("{label}-budget-started.r3b")).exists() {
            if let Action::Train{root,arm,until}=&action {
                let _=registered(root)?;let backend=if arm=="cpu16"{Backend::Cpu}else{Backend::Metal0};let device=backend.open()?;
                execution(root,backend,&device)?;
                let from=if arm=="metal-split"&&*until==16{1}else{0};
                let finished=root.join(format!("{label}-budget-finished.r3b"));
                if finished.exists()||pending_path(&finished).exists() {
                    let f:binary::Value=read_confirmed(&finished)?;
                    if f["success"]!=true {return Err(bad("sticky backend command failure"));}
                }
                finalize_training(&root.join(arm),&format!("segment-{from}-{until}"),from,*until as usize)?;
                if !finished.exists() {
                    let start:binary::Value=read_confirmed(&root.join(format!("{label}-budget-started.r3b")))?;
                    let began=start["started_unix_seconds"].as_f64().ok_or_else(||bad("interrupted active time UNKNOWN"))?;
                    let now=std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_err(|_|bad("system clock"))?.as_secs_f64();
                    let upper=now-began;
                    if !upper.is_finite()||upper<0. {return Err(bad("interrupted elapsed UNKNOWN"));}
                    // Wall elapsed includes downtime. It is a conservative upper
                    // bound, not a fabricated zero or measured GPU duration.
                    publish_confirmed(&finished,&binary::record!({"success":true,"elapsed_seconds":upper,"elapsed_kind":"WALL_UPPER_BOUND_INCLUDES_DOWNTIME","completed":file_hash(&root.join(arm).join(format!("segment-{from}-{until}-completed.r3b")))?,"new_model_calls":0}))?;
                }return Ok(());
            }return Err(bad("backend action already attempted; no retry"));
        }
        let used=previous_active(root,label)?;let allowance=(7200.-used).min(900.);
        if allowance<=0. {return Err(bad("backend total active budget exhausted"));}
        let now=std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_err(|_|bad("system clock"))?.as_secs_f64();
        publish_confirmed(&root.join(format!("{label}-budget-started.r3b")),&binary::record!({"previous_active_or_reserved_seconds":used,"started_unix_seconds":now,"deadline_unix_seconds":now+allowance,"allowance_seconds":allowance,"binary":file_hash(&std::env::current_exe()?)?}))?;
    }
    let now=Instant::now();let result=match action {
    Action::Prepare{protected,protected_root,parent,word_root,review_a,output}=>prepare(&protected,&protected_root,&parent,&word_root,&review_a,&output),
    Action::Observe{root,device,panel}=>observe(&root,device,&panel),Action::Report{root}=>report(&root),
    Action::PrepareExecution{root,worker}=>prepare_execution(&root,&worker),Action::Worker{root,device}=>worker(&root,device),
    Action::Cache{root}=>cache(&root),Action::Train{root,arm,until}=>train(&root,&arm,until as usize),
    Action::Endpoints{root}=>endpoints(&root),Action::Benchmark{root,device}=>benchmark(&root,device),
};
    if let Some((root,label))=bounded {
        publish_confirmed(&root.join(format!("{label}-budget-finished.r3b")),&binary::record!({"success":result.is_ok(),"elapsed_seconds":now.elapsed().as_secs_f64(),"error":result.as_ref().err().map(ToString::to_string)}))?;
    }result
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn external_call_and_no_call_finalization_fail_closed()->Result<()> {
        use std::{sync::Arc,time::Duration};
        let cancel=Arc::new(AtomicBool::new(false));
        let mut c=RunControl::new(cancel.clone(),Duration::from_secs(1),u64::MAX)?;c.measure_rss=false;c.set_call_limits(1,0);
        c.begin_external_generation()?;c.returned_external_generation();
        assert_eq!(c.receipt()["generation_calls"],1);assert_eq!(c.receipt()["completed_generation_count"],1);
        assert!(c.begin_external_generation().is_err());assert!(c.seal_completed_no_call().is_err());
        for cancelled in [false,true] {
            let flag=Arc::new(AtomicBool::new(cancelled));let mut c=RunControl::new(flag,Duration::ZERO,u64::MAX)?;c.measure_rss=false;
            assert_eq!(c.seal_completed_no_call().is_ok(),!cancelled);
            assert_eq!(c.receipt()["generation_calls"],0);
        }
        println!("CONTROL external1/returned1 counters only; actual models0; TIME_BUDGET complete allowed; cancel/limit blocked");Ok(())
    }
    #[test]
    #[ignore="bounded actual CPU/Metal TINY timing parity, four generation calls"]
    fn profiled_generation_preserves_native_decoder()->Result<()> {
        let cpu=Transformer::init(Config::tiny(264),20260924,Device::Cpu)?;
        let gpu=Backend::Metal0.open()?;
        let vars=cpu.vars.iter().map(|(name,v)|Ok((name.clone(),v.as_detached_tensor().to_device(&gpu)?))).collect::<Result<BTreeMap<_,_>>>()?;
        let metal=Transformer::from_tensors(cpu.config.clone(),vars,gpu)?;
        for model in [&cpu,&metal] {
            let cancel=AtomicBool::new(false);let mut a=Vec::new();let mut b=Vec::new();
            let normal=model.generate_observed(&[8,9,10],2,30000,&cancel,"timing",|id|a.push(id))?;
            let profiled=model.generate_profiled(&[8,9,10],2,30000,&cancel,"timing",|id|b.push(id))?;
            assert_eq!(a,b);assert_eq!(normal.tokens,profiled.tokens);assert_eq!(normal.finish,profiled.finish);
            assert!(normal.synchronized_phases_ms.is_none());assert!(profiled.synchronized_phases_ms.is_some());
            println!("TIMING_PARITY device={:?} raw={a:?} phases={:?}",model.device.location(),profiled.synchronized_phases_ms);
        }println!("TINY_GENERATIONS4 optimizer0 backward0");Ok(())
    }
    #[test]
    fn runtime_profile_is_execution_only_and_fail_closed()->Result<()> {
        let cpu=RuntimeProfile::capture(Backend::Cpu,&Device::Cpu)?;
        let bytes=binary::to_vec(&cpu)?;
        let restored:RuntimeProfile=binary::from_slice(&bytes)?;
        restored.verify(&Device::Cpu)?;
        for field in ["dtype","patch","lock","binary","actual_device"] {
            let mut value=binary::to_value(&cpu)?;value[field]=binary::record!("UNKNOWN");
            let other:RuntimeProfile=binary::from_value(value)?;
            assert!(other.verify(&Device::Cpu).is_err(),"{field}");
        }
        let revision=format!("replica-native-trpp-v1;candle-0.11.0;{};greedy;native-role-bytes-v1",neural::cpu_backend());
        assert_eq!(Backend::Cpu.runtime_revision(),revision);
        assert!(Backend::Metal0.runtime_revision().contains(neural::METAL_SUM_PATCH));
        assert!(Backend::Metal0.verify(&Device::Cpu).is_err());
        println!("PROFILE model_calls0 optimizer0 generation0");Ok(())
    }
}
