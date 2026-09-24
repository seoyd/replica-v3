//! The fixed Metal F32 optimizer comparison; training-only, no inference routing.
use super::*;
use neural::{Backend, RuntimeProfile, checkpoint::OptimizerProtocol};
use recovery::{ObservedCall,RunControl};

const CONTRACT:&str="R3-METAL-F32-MUON-QUALITY-1.0";
const ARMS:[&str;2]=["A","M"];
#[derive(Subcommand)]
pub enum Action {
    Prepare { #[arg(long)] parent:PathBuf, #[arg(long)] word_root:PathBuf, #[arg(long)] output:PathBuf },
    Admit { #[arg(long)] root:PathBuf, #[arg(long)] review:PathBuf },
    Baseline { #[arg(long)] root:PathBuf },
    Train { #[arg(long)] root:PathBuf },
    Report { #[arg(long)] root:PathBuf },
    Review { #[arg(long)] root:PathBuf, #[arg(long,value_parser=["A","M"])] arm:String },
}

#[derive(Clone,Serialize,Deserialize)]
#[serde(deny_unknown_fields)]
struct Study {
    contract:String,root:PathBuf,parent:PathBuf,word_root:PathBuf,
    parent_hash:String,parent_content:String,parent_adam:String,parent_step:usize,
    plan_hash:String,metadata_hash:String,corpus_hash:String,tokenizer:String,
    source:String,runtime:RuntimeProfile,config:TrainConfig,tape:Vec<[usize;8]>,
    costs:binary::Value,baseline_raw:PathBuf,baseline_hash:String,
    max_updates:usize,generation_cap:usize,teacher_cap:usize,active_cap:f64,segment_cap:f64,bytes_cap:u64,
}
fn sources()->Result<String>{digest(&(source_digest()?,neural::hash(include_bytes!("muon.rs")),neural::hash(include_bytes!("metal_runtime.rs"))))}
fn inputs(s:&Study)->Result<(data::native::Corpus,Vec<Meta>,Vec<Meta>)> {
    if file_hash(&s.parent)?!=s.parent_hash||file_hash(&s.word_root.join("plan.r3b"))?!=s.plan_hash
        ||file_hash(&s.baseline_raw)?!=s.baseline_hash{return Err(bad("frozen Muon source inputs changed"));}
    let p:Plan=read(&s.word_root.join("plan.r3b"))?;
    if p.metadata!=s.metadata_hash||p.corpus!=s.corpus_hash{return Err(bad("frozen corpus metadata policy"));}
    let c=verified_corpus(&s.word_root.join("corpus.r3cor"),&s.corpus_hash)?;
    let (tm,dm,_)=verified_metadata(&s.word_root,&p)?;
    if tm.len()!=c.train.len()||dm.len()!=c.validation.len(){return Err(bad("owned corpus/metadata counts"));}
    Ok((c,tm,dm))
}
fn prepare(parent:&Path,word_root:&Path,output:&Path)->Result<()> {
    let parent=parent.canonicalize()?;let word_root=word_root.canonicalize()?;
    let p:Plan=read(&word_root.join("plan.r3b"))?;
    let c=verified_corpus(&word_root.join("corpus.r3cor"),&p.corpus)?;
    verified_metadata(&word_root,&p)?;
    let l=checkpoint::load(&parent,Device::Cpu,true)?;
    let st=l.manifest.training.as_ref().ok_or_else(||bad("missing parent training descriptor"))?;
    if st.step!=14336||l.manifest.optimizer_protocol.is_some()||l.model.adapter().is_some()
        ||p.initial!=file_hash(&parent)?||p.origin_step()!=st.step||p.framing()!=neural::Framing::QuestionEvidence
        ||p.config.lr!=3e-5||p.config.seq_len!=256||p.config.microbatch!=8||p.config.accumulation!=1
        ||st.resume_binding.as_ref().is_none_or(|b|b.family!=checkpoint::ANSWER_MEAN_FAMILY)
        ||c.train.len()!=7680||c.validation.len()!=3456 {return Err(bad("Muon exact original parent/data contract"));}
    let rows=&p.identifiable.as_ref().ok_or_else(||bad("missing word tape"))?.rows;
    let tape=rows.get(st.step..st.step+1024).ok_or_else(||bad("finite word prefix"))?.to_vec();
    let ss=samples_with_framing(&c.train,&l.tokenizer,256,p.framing())?;
    let(mut input,mut target,mut padding)=(0usize,0usize,0usize);let mut exposure=vec![0usize;c.train.len()];
    for row in &tape {
        if row[..4].iter().any(|&i|i>=6144)||row[4..].iter().any(|&i|!(6144..7680).contains(&i)){return Err(bad("word4/review4 tape"));}
        let length=row.iter().map(|&i|ss[i].tokens.len()-1).max().unwrap();
        for &i in row {exposure[i]+=1;input+=ss[i].tokens.len()-1;target+=ss[i].tokens.len()-ss[i].response_start;padding+=length-(ss[i].tokens.len()-1);}
    }
    let mut config=st.config.clone();config.lr=3e-5;config.warmup=0;config.seq_len=256;config.microbatch=8;config.accumulation=1;
    config.budget_start_step=st.step;config.max_steps=st.step+1024;config.budget_start_tokens=st.consumed_tokens;config.max_tokens=st.consumed_tokens+input as u64;
    config.validate(l.model.config.context)?;
    let sel:binary::Value=read(&p.fork.as_ref().ok_or_else(||bad("word fork absent"))?.study.join("selection.r3b"))?;
    let baseline_raw=PathBuf::from(sel["parent"].as_str().ok_or_else(||bad("word original parent panel root"))?).join(format!("eval-{:04}-value512.r3rows",st.step));
    let historical=binary::read_value_records(&baseline_raw)?;
    if historical.len()!=513||historical[1..17].iter().zip(&c.validation[..16]).any(|(r,e)|r["id"]!=e.id||r["expected"]!=e.answer){return Err(bad("historical parent parity panel"));}
    let device=Backend::Metal0.open()?;
    let runtime=RuntimeProfile::capture(Backend::Metal0,&device)?;
    std::fs::create_dir(output)?;let root=output.canonicalize()?;
    let s=Study{contract:CONTRACT.into(),root:root.clone(),parent_hash:file_hash(&parent)?,parent_content:l.model.weights_content_id()?,parent_adam:optimizer_hash(&l.optimizer)?,parent,parent_step:st.step,
        word_root:word_root.clone(),plan_hash:file_hash(&word_root.join("plan.r3b"))?,metadata_hash:p.metadata,corpus_hash:p.corpus,tokenizer:l.tokenizer.semantic_id(),source:sources()?,runtime,config,tape,
        costs:binary::record!({"input":input,"target":target,"padding":padding,"exposures":exposure,"samples":8192,
            "sample_content":digest(&c.train.iter().map(digest).collect::<Result<Vec<_>>>()?)?,
            "sample_tokens":digest(&ss.iter().map(|v|digest(&(&v.tokens,v.response_start))).collect::<Result<Vec<_>>>()?)?}),
        baseline_hash:file_hash(&baseline_raw)?,baseline_raw,max_updates:1024,generation_cap:6400,teacher_cap:6400,active_cap:7200.,segment_cap:900.,bytes_cap:1610612736};
    for arm in ARMS {std::fs::create_dir(root.join(arm))?;}
    publish_confirmed(&root.join("plan.r3b"),&s)?;
    let roles=[OptimizerProtocol::new(&l.model.config,false,digest(&s.runtime)?,st.step)?,OptimizerProtocol::new(&l.model.config,true,digest(&s.runtime)?,st.step)?];
    publish_confirmed(&root.join("preparation.r3b"),&binary::record!({"policy":digest(&s)?,"parent":s.parent_hash,"content":s.parent_content,"parent_adam_preserved":s.parent_adam,"roles":roles,"fresh_state":"all zeros on allocation; no inherited moments","costs":s.costs,"optimizer_calls":0,"generation_calls":0,"phase_order":[["A",32],["M",32],["M",128],["A",128],["A",512],["M",512],["M",1024],["A",1024]]}))?;
    println!("MUON_PREPARED policy={} parent={} input={input} target={target} padding={padding} new_optimizer0 A_PENDING",digest(&s)?,s.parent_hash);Ok(())
}

fn load_study(root:&Path,execute:bool)->Result<Study>{
    let s:Study=read_confirmed(&root.join("plan.r3b"))?;
    if s.contract!=CONTRACT||s.root!=root.canonicalize()?{return Err(bad("Muon root/contract binding"));}
    inputs(&s)?;
    if execute {
        if s.source!=sources()?{return Err(bad("Muon frozen source changed"));}
        s.runtime.verify(&Backend::Metal0.open()?)?;
        let a:binary::Value=read_confirmed(&root.join("review-a.r3b"))?;
        let path=Path::new(a["report_path"].as_str().ok_or_else(||bad("independent A absent"))?);
        if a["policy"]!=digest(&s)?||a["source"]!=s.source||a["report_hash"]!=file_hash(path)?||a["accepted"]!=true{return Err(bad("independent A identity"));}
    }Ok(s)
}
fn admit(root:&Path,review:&Path)->Result<()> {
    let s=load_study(root,false)?;let r:binary::Value=read(review)?;
    // The independent reviewer authors this typed receipt after real tests.
    if r["contract"]!=CONTRACT||r["source"]!=s.source||r["policy"]!=digest(&s)?||r["runtime"]!=binary::record!(s.runtime)
        ||r["verdict"]!="PASS"||r["tests_passed"].as_u64().is_none_or(|n|n<6)||r["active_seconds"].as_f64().is_none_or(|v|!v.is_finite()||v<0.||v>=7200.) {
        return Err(bad("independent A incomplete/mismatch"));
    }
    publish_confirmed(&root.join("review-a.r3b"),&binary::record!({"accepted":true,"policy":digest(&s)?,"source":s.source,"report_path":review.canonicalize()?,"report_hash":file_hash(review)?,"active_seconds":r["active_seconds"]}))
}

#[derive(Clone,Serialize,Deserialize)]
struct Progress { local:usize,native:PathBuf,physical:String,stop:Option<String>,evaluated:usize,fit:bool }
#[derive(Clone,Serialize,Deserialize)]
struct Segment { policy:String,previous:Option<String>,phase:usize,arms:[Progress;2],success:bool,resume:bool,control:binary::Value,error:Option<String> }
fn history(s:&Study)->Result<Vec<Segment>> {
    let mut out:Vec<Segment>=vec![];
    for index in 0..128 {
        let start=s.root.join(format!("segment-{index:03}-started.r3b"));
        if !start.exists()&&!pending_path(&start).exists(){break;}
        let begun:binary::Value=read_confirmed(&start)?;
        let end=s.root.join(format!("segment-{index:03}-finished.r3b"));
        let seg:Segment=read_confirmed(&end).map_err(|_|bad("UNKNOWN/unfinished study segment; new work blocked"))?;
        let expected=if index==0{None}else{Some(file_hash(&s.root.join(format!("segment-{:03}-finished.r3b",index-1)))?)};
        if begun["policy"]!=digest(s)?||seg.policy!=digest(s)?||seg.previous!=expected||seg.phase>8
            ||seg.arms.iter().any(|a|a.local>s.max_updates||a.evaluated>a.local) {return Err(bad("segment chain/policy/cursor"));}
        for a in &seg.arms {if file_hash(&a.native)?!=a.physical{return Err(bad("durable native changed"));}}
        out.push(seg);
    }Ok(out)
}
fn previous_usage(s:&Study,h:&[Segment])->Result<(f64,usize,usize)> {
    let a:binary::Value=read_confirmed(&s.root.join("review-a.r3b"))?;
    let mut time=a["active_seconds"].as_f64().ok_or_else(||bad("A active usage unknown"))?;let(mut generations,mut teacher)=(0,0);
    let observations=["baseline","review-A","review-M"].into_iter().map(|label|observation_history(s,label)).collect::<Result<Vec<_>>>()?;
    for c in h.iter().map(|h|h.control.clone()).chain(observations.into_iter().flatten().map(|v|v["control"].clone())) {
        time+=c["elapsed_seconds"].as_f64().filter(|v|v.is_finite()&&*v>=0.).ok_or_else(||bad("active time UNKNOWN"))?;
        generations+=c["generation_calls"].as_u64().ok_or_else(||bad("generation usage UNKNOWN"))? as usize;
        teacher+=c["teacher_calls"].as_u64().ok_or_else(||bad("teacher usage UNKNOWN"))? as usize;
        if c["observed_conditions"].as_array().is_none_or(|v|v.iter().any(|x|x!="TIME_BUDGET")){return Err(bad("sticky execution error"));}
    }
    if generations>s.generation_cap||teacher>s.teacher_cap{return Err(bad("study call budget exceeded"));}Ok((time,generations,teacher))
}
fn observation_history(s:&Study,label:&str)->Result<Vec<binary::Value>>{
    let mut records=vec![];
    for i in 0..128 {
        let start=s.root.join(format!("{label}-segment-{i:03}-started.r3b"));
        if !start.exists()&&!pending_path(&start).exists(){break;}
        let start_value:binary::Value=read_confirmed(&start)?;
        let end:binary::Value=read_confirmed(&s.root.join(format!("{label}-segment-{i:03}-finished.r3b")))?;
        if start_value["policy"]!=digest(s)?||end["start"]!=file_hash(&start)?||end["control"].is_null()
            ||end["success"]!=true&&end["resume"]!=true{return Err(bad("observation unknown/sticky failure"));}
        records.push(end);
    }Ok(records)
}
fn start_observation(s:&Study,label:&str,binding:&binary::Value)->Result<(usize,RunControl)>{
    let previous=observation_history(s,label)?;
    if previous.last().is_some_and(|r|r["success"]==true){return Err(bad("observation already complete; pure report only"));}
    let h=history(s)?;let c=control(s,&h)?;let i=previous.len();
    let fixed=s.root.join(format!("{label}-started.r3b"));
    if fixed.exists(){if read_confirmed::<binary::Value>(&fixed)?!=*binding{return Err(bad("observation binding changed"));}}else{publish_confirmed(&fixed,binding)?;}
    publish_confirmed(&s.root.join(format!("{label}-segment-{i:03}-started.r3b")),&binary::record!({"policy":digest(s)?,"binding":file_hash(&fixed)?,"segment":i}))?;Ok((i,c))
}
fn finish_observation(s:&Study,label:&str,index:usize,c:&mut RunControl,result:&Result<()>,extra:binary::Value)->Result<()> {
    if let Err(e)=result{c.classify_error(e);}
    let clean=c.receipt()["observed_conditions"]==binary::record!(["TIME_BUDGET"]);
    let r=binary::record!({"start":file_hash(&s.root.join(format!("{label}-segment-{index:03}-started.r3b")))?,"success":result.is_ok(),"resume":result.is_err()&&clean,"control":c.receipt(),"extra":extra,"error":result.as_ref().err().map(ToString::to_string)});
    publish_confirmed(&s.root.join(format!("{label}-segment-{index:03}-finished.r3b")),&r)?;
    if result.is_ok(){publish_confirmed(&s.root.join(format!("{label}-finished.r3b")),&r)?;}Ok(())
}
fn control(s:&Study,h:&[Segment])->Result<RunControl>{
    let (time,g,t)=previous_usage(s,h)?;
    let cancel=std::sync::Arc::new(AtomicBool::new(false));let signal=cancel.clone();
    ctrlc::set_handler(move||signal.store(true,Ordering::Relaxed)).map_err(|e|bad(&e.to_string()))?;
    let mut c=RunControl::new(cancel,std::time::Duration::from_secs_f64((s.active_cap-time).max(0.).min(s.segment_cap)),12*1024*1024)?;
    c.set_call_limits(s.generation_cap-g,s.teacher_cap-t);Ok(c)
}
fn owned_bytes(root:&Path)->Result<u64>{
    let mut bytes=0u64;for e in std::fs::read_dir(root)?{let e=e?;let m=e.metadata()?;if m.is_dir(){bytes+=owned_bytes(&e.path())?;}else{bytes+=m.len();}}Ok(bytes)
}
fn guard_bytes(s:&Study,reserve:u64)->Result<()> {if owned_bytes(&s.root)?.saturating_add(reserve)>s.bytes_cap{return Err(bad("BLOCKED_DISK: study immutable byte cap"));}Ok(())}
fn bind(s:&Study,arm:usize,state:&TrainingState,tok:&ByteBpe)->Result<checkpoint::ResumeBinding>{
    let mut b=checkpoint::ResumeBinding::default_for(state,tok);b.family=checkpoint::ANSWER_MEAN_FAMILY;b.normalizer=2;b.execution=1;
    b.policy=checkpoint::ResumeBinding::digest_bytes(&binary::to_vec(&(s,ARMS[arm]))?);b.provenance=b.policy;
    b.train_order=checkpoint::ResumeBinding::digest_bytes(&binary::to_vec(&s.tape)?);b.framing=neural::Framing::QuestionEvidence.digest();Ok(b)
}
fn load_arm(s:&Study,arm:usize,p:&Progress,device:&Device)->Result<(checkpoint::Loaded,Optimizer)> {
    let mut l=checkpoint::load(&p.native,device.clone(),true)?;let mut state=l.manifest.training.clone().ok_or_else(||bad("missing native training state"))?;
    let corpus=verified_corpus(&s.word_root.join("corpus.r3cor"),&s.corpus_hash)?;
    let opt=if p.local==0 {
        if p.physical!=s.parent_hash||state.step!=s.parent_step||l.model.weights_content_id()?!=s.parent_content||optimizer_hash(&l.optimizer)?!=s.parent_adam{return Err(bad("fresh parent weights/Adam provenance"));}
        state.parent_checkpoint_hash=Some(s.parent_hash.clone());state.config=s.config.clone();state.corpus_hash=corpus.manifest.train.sha256.clone();
        state.validation_hash=corpus.manifest.validation.sha256.clone();
        if !state.previous_corpora.contains(&l.tokenizer.train_hash){state.previous_corpora.push(l.tokenizer.train_hash.clone());}
        state.sampler_state=0;state.resume_binding=Some(bind(s,arm,&state,&l.tokenizer)?);
        Optimizer::fresh(&l.model.config,&l.model.vars,arm==1,digest(&s.runtime)?,s.parent_step)?
    } else {
        let protocol=l.manifest.optimizer_protocol.clone().ok_or_else(||bad("optimizer identity absent"))?;
        protocol.validate(&l.model.config,&state)?;
        let mut expected=OptimizerProtocol::new(&l.model.config,arm==1,digest(&s.runtime)?,s.parent_step)?;expected.local_step=p.local;
        if protocol!=expected||state.config!=s.config||state.sampler_state!=p.local as u64||state.corpus_hash!=corpus.manifest.train.sha256
            ||state.validation_hash!=corpus.manifest.validation.sha256||state.parent_checkpoint_hash.as_deref()!=Some(s.parent_hash.as_str())
            ||state.resume_binding!=Some(bind(s,arm,&state,&l.tokenizer)?){return Err(bad("wrong optimizer policy/runtime/local clock"));}
        Optimizer{protocol,tensors:std::mem::take(&mut l.optimizer)}
    };
    opt.validate(&l.model.vars)?;l.manifest.training=Some(state);l.manifest.optimizer_protocol=Some(opt.protocol.clone());l.optimizer.clear();Ok((l,opt))
}
fn save_arm(s:&Study,index:usize,arm:usize,l:&mut checkpoint::Loaded,o:&Optimizer,p:&mut Progress)->Result<()> {
    l.model.device.synchronize()?;l.manifest.optimizer_protocol=Some(o.protocol.clone());l.manifest.status="DIAGNOSTIC_COMPLETE".into();
    let path=s.root.join(ARMS[arm]).join(format!("segment-{index:03}-step-{}.r3m",p.local));
    let bytes=(l.model.vars.values().map(|v|v.elem_count()).sum::<usize>()+o.tensors.values().map(Tensor::elem_count).sum::<usize>())as u64*4+1024*1024;
    guard_bytes(s,bytes)?;
    checkpoint::save(&path,&l.model,&l.tokenizer,l.manifest.clone(),&o.tensors)?;
    p.physical=file_hash(&path)?;p.native=path;println!("DURABLE arm={} local={} absolute={} bytes={}",ARMS[arm],p.local,s.parent_step+p.local,std::fs::metadata(&p.native)?.len());Ok(())
}
fn generated(l:&checkpoint::Loaded,root:&Path,label:&str,es:&[Episode],binding:&binary::Value,c:&mut RunControl)->Result<Vec<binary::Value>>{
    let path=root.join(format!("{label}.r3rows"));let mut entries=if path.exists(){binary::read_value_records(&path)?}else{vec![]};
    if entries.first().is_some_and(|v|v!=binding){return Err(bad("evaluation binding changed"));}
    let mut rows=if entries.is_empty(){vec![]}else{entries.drain(1..).collect()};
    if rows.len()>es.len(){return Err(bad("extra RETURNED rows"));}
    for(i,row)in rows.iter().enumerate(){call_attempt(root,label,"generation",binding,&es[i],i,Some(row))?;verify_generated(row,&l.tokenizer)?;if row["id"]!=es[i].id||row["expected"]!=es[i].answer{return Err(bad("RETURNED case changed"));}}
    let mut f=std::fs::OpenOptions::new().append(true).create_new(!path.exists()).open(&path)?;
    if entries.is_empty(){append_row(&mut f,binding)?;std::fs::File::open(root)?.sync_all()?;}
    for(i,e)in es.iter().enumerate().skip(rows.len()){
        c.check("muon_next_generation")?;let attempt=prepare_call(root,label,"generation",binding,e,i)?;
        let mut row=match recovery::observe_generation(l,e,&e.request,c,false){ObservedCall::Returned(r)=>r,ObservedCall::NotInvoked(_)=>{resolve_call(&attempt,None,c)?;c.stop_result()?;return Err(bad("not invoked without stop"));}};
        row["attempt"]=binary::record!(attempt.file_name().unwrap().to_string_lossy());append_row(&mut f,&row)?;resolve_call(&attempt,Some(&row),c)?;
        verify_generated(&row,&l.tokenizer)?;let error=row["error"].clone();rows.push(row);
        if !error.is_null(){c.stop_result()?;return Err(bad(&format!("native generation error {error}")));}
        if i%64==63{println!("PANEL {label} returned={}/{}",i+1,es.len());}
    }Ok(rows)
}
fn baseline(s:&Study)->Result<()> {
    let h=history(s)?;if !h.is_empty(){return Err(bad("baseline after training forbidden"));}
    let (c,_,_)=inputs(s)?;let es=&c.validation[..16];
    let expected=binary::read_value_records(&s.baseline_raw)?;
    let(index,mut ctl)=start_observation(s,"baseline",&binary::record!({"policy":digest(s)?,"planned":16,"parent":s.parent_hash}))?;
    let result=(||->Result<()>{let l=checkpoint::load(&s.parent,Backend::Metal0.open()?,false)?;
        let b=binary::record!({"policy":digest(s)?,"native":s.parent_hash,"runtime":s.runtime,"cases":digest(&es)?});
        let rows=generated(&l,&s.root,"baseline",es,&b,&mut ctl)?;
        for(a,b)in rows.iter().zip(&expected[1..17]){for field in ["raw_tokens","actual","finish_reason","error","generation_completed"]{if a[field]!=b[field]{return Err(bad("same-parent normal16 differs"));}}}
        ctl.seal_completed_no_call()?;Ok(())})();
    finish_observation(s,"baseline",index,&mut ctl,&result,binary::record!({"parity":result.is_ok(),"planned":16}))?;
    result
}
type Panel=(String,Vec<Episode>,Vec<Meta>);
fn panels(s:&Study,local:usize,train:bool)->Result<Vec<Panel>>{
    let(c,tm,dm)=inputs(s)?;let mut out=vec![];
    if train {
        let ids=(0..2).flat_map(|v|(0..16).flat_map(move|b|(0..4).map(move|q|6144+v*768+b*4+q))).collect::<Vec<_>>();
        out.push(("word-train128".into(),ids.iter().map(|&i|c.train[i].clone()).collect(),ids.iter().map(|&i|tm[i].clone()).collect()));
    }else{
        for(name,at,n)in [("value",0,512),("citation",512,512),("S1Q1",2560,512),("word",3072,192),("renamed",3264,192)]{
            let n=if local==s.max_updates{n}else{64};out.push((name.into(),c.validation[at..at+n].to_vec(),dm[at..at+n].to_vec()));
        }
    }Ok(out)
}
fn scored(name:&str,es:&[Episode],ms:&[Meta],rows:&[binary::Value],tok:&ByteBpe)->Result<binary::Value>{
    if name=="value" {Ok(binary::record!({"joint":identifiable::binding::orbit_score(es,ms,rows,tok)?}))}
    else {identifiable::binding::citation::word::rows_score(es,ms,rows,tok)}
}
fn evaluate(s:&Study,arm:usize,l:&mut checkpoint::Loaded,p:&Progress,train:bool,ctl:&mut RunControl)->Result<BTreeMap<String,binary::Value>>{
    l.model.refresh_identity()?;let model=l.model.weights_content_id()?;let dir=s.root.join(ARMS[arm]);let mut out=BTreeMap::new();
    for(name,es,ms)in panels(s,p.local,train)?{
        let label=format!("eval-{}-{name}",p.local);
        let binding=binary::record!({"policy":digest(s)?,"arm":ARMS[arm],"model":model,"local":p.local,"absolute":s.parent_step+p.local,"runtime":s.runtime,"cases":digest(&es)?,"metadata":digest(&ms)?,"tokenizer":s.tokenizer,"planned":es.len(),"call_protocol":1});
        let rows=generated(l,&dir,&label,&es,&binding,ctl)?;
        let mut score=scored(&name,&es,&ms,&rows,&l.tokenizer)?;score["raw_hash"]=binary::record!(file_hash(&dir.join(format!("{label}.r3rows")))?);score["model"]=binary::record!(model);
        if train{
            let teachers=teacher_prefix(&dir,&label,&binding,l,&es,ctl)?;score["teacher_nll"]=binary::record!(teacher_ce(&teachers)?);
            let(c,_,_)=inputs(s)?;let seen=s.tape[..p.local].iter().flatten().fold(BTreeMap::<usize,usize>::new(),|mut a,&i|{*a.entry(i).or_default()+=1;a});
            score["actual_train_exposures"]=binary::record!(es.iter().map(|e|c.train.iter().position(|x|x.id==e.id).map(|i|seen.get(&i).copied().unwrap_or(0))).collect::<Vec<_>>());
        }
        let path=dir.join(format!("{label}-score.r3b"));if path.exists(){if read_confirmed::<binary::Value>(&path)?!=score{return Err(bad("score disagreement"));}}else{publish_confirmed(&path,&score)?;}
        println!("SCORE arm={} local={} {name} {}",ARMS[arm],p.local,score["joint"]);out.insert(name,score);
    }Ok(out)
}

fn decision(s:&Study,arm:usize,p:&Progress,scores:&BTreeMap<String,binary::Value>)->Result<Option<String>>{
    let dir=s.root.join(ARMS[arm]);let mut streak=[0usize;3];let mut stop=None;
    for at in [32,128,512,1024].into_iter().filter(|&at|at<p.local){
        let d:binary::Value=read_confirmed(&dir.join(format!("decision-{at}.r3b")))?;
        if d["policy"]!=digest(s)?||d["local"]!=at||d["stop"].is_string(){return Err(bad("prior guard decision invalid/stopped"));}
        streak=binary::from_value(d["streak"].clone())?;
    }
    for(i,name)in ["value","citation","S1Q1"].iter().enumerate(){
        let j=&scores[*name]["joint"];let total=j["total"].as_u64().ok_or_else(||bad("guard missing denominator"))?;
        let full=j["full"].as_u64().ok_or_else(||bad("guard missing full"))?;
        let all4=j["all4"].as_u64().ok_or_else(||bad("guard missing ALL4"))?;
        // At the full endpoint, independently score the same fixed first64.
        let (full,all4,errors)=if total==64{(full,all4,total-j["eos"].as_u64().ok_or_else(||bad("guard missing EOS"))?)}else{
            let panel=panels(s,p.local,false)?.into_iter().find(|v|v.0==*name).ok_or_else(||bad("guard panel"))?;
            let raw=binary::read_value_records(&dir.join(format!("eval-{}-{name}.r3rows",p.local)))?;
            let tok=scoring_tokenizer(s)?;let r=scored(name,&panel.1[..64],&panel.2[..64],&raw[1..65],&tok)?;
            (r["joint"]["full"].as_u64().unwrap(),r["joint"]["all4"].as_u64().unwrap(),64-r["joint"]["eos"].as_u64().unwrap())
        };
        streak[i]=if full<60||all4<12{streak[i]+1}else{0};
        if full<=48||errors>=4{stop=Some("SEVERE_RETENTION".to_owned());}
        else if streak[i]>=2&&stop.is_none(){stop=Some("PERSISTENT_RETENTION".to_owned());}
    }
    let d=binary::record!({"policy":digest(s)?,"local":p.local,"scores":scores,"streak":streak,"stop":stop});let path=dir.join(format!("decision-{}.r3b",p.local));
    if path.exists(){if read_confirmed::<binary::Value>(&path)?!=d{return Err(bad("guard decision mismatch"));}}else{publish_confirmed(&path,&d)?;}Ok(stop)
}
fn train(s:&Study)->Result<()> {
    let baseline:binary::Value=read_confirmed(&s.root.join("baseline-finished.r3b"))?;if baseline["success"]!=true{return Err(bad("baseline not accepted"));}
    let h=history(s)?;if h.last().is_some_and(|s|!s.resume){return Err(bad("closed pair; no resume"));}
    let mut ctl=control(s,&h)?;let index=h.len();
    let initial=Progress{local:0,native:s.parent.clone(),physical:s.parent_hash.clone(),stop:None,evaluated:0,fit:false};
    let mut ps=h.last().map(|h|h.arms.clone()).unwrap_or([initial.clone(),initial]);let mut phase=h.last().map_or(0,|h|h.phase);
    let previous=if index==0{None}else{Some(file_hash(&s.root.join(format!("segment-{:03}-finished.r3b",index-1)))?)};
    publish_confirmed(&s.root.join(format!("segment-{index:03}-started.r3b")),&binary::record!({"policy":digest(s)?,"previous":previous,"phase":phase,"arms":ps}))?;
    let device=Backend::Metal0.open()?;let mut models=vec![];let saved_local=ps.each_ref().map(|p|p.local);
    let mut optimizer_entered=false;
    let result=(||->Result<()> {
        // Native load is not a model call. Complete RETURNED evaluation can be
        // sealed at zero remaining time; every new forward is checked below.
        for arm in 0..2 {models.push(load_arm(s,arm,&ps[arm],&device)?);}
        if models[0].0.tokenizer.semantic_id()!=s.tokenizer||models[1].0.tokenizer.semantic_id()!=s.tokenizer{return Err(bad("tokenizer changed"));}
        if index==0 {
            if models[0].0.model.weights_content_id()?!=models[1].0.model.weights_content_id()?||models.iter().any(|(_,o)|o.protocol.local_step!=0){return Err(bad("fresh pair initial state"));}
            for(_,o)in &models {for t in o.tensors.values(){if norm(t)?!=0.{return Err(bad("nonzero fresh optimizer"));}}}
            publish_confirmed(&s.root.join("fresh-state.r3b"),&binary::record!({"parent":s.parent_hash,"content":s.parent_content,"both_identical":true,"state0":true,"optimizer_reset":"all states only; weights retained"}))?;
        }
        let(c,_,_)=inputs(s)?;
        let samples=if phase<8{samples_with_framing(&c.train,&models[0].0.tokenizer,s.config.seq_len,neural::Framing::QuestionEvidence)?}else{vec![]};
        let phases=[(0,32),(1,32),(1,128),(0,128),(0,512),(1,512),(1,1024),(0,1024)];
        loop {
            let stopping=ps.iter().filter(|p|p.stop.is_some()).map(|p|p.local).min();
            if phase>=phases.len()||stopping.is_some_and(|n|ps.iter().all(|p|p.local==n&&p.evaluated==n)){break;}
            let (arm,endpoint)=if index==0 {if ps[0].local==0{(0,1)}else if ps[1].local==0{(1,1)}else{break;}}else{phases[phase]};
            if stopping.is_some_and(|n|endpoint>n){return Err(bad("peer endpoint exceeds quality stop"));}
            let(l,o)=&mut models[arm];let p=&mut ps[arm];
            let path=s.root.join(ARMS[arm]).join(format!("updates-{index:03}.r3rows"));
            let mut trace=std::fs::OpenOptions::new().append(true).create(true).open(&path)?;
            while p.local<endpoint {
                ctl.check("before_study_microbatch")?;guard_bytes(s,0)?;
                let b=batch(&samples,&s.tape[p.local],&device)?;device.synchronize()?;let start=Instant::now();
                let entry=s.root.join(ARMS[arm]).join(format!("step-{}-segment-{index:03}-entered.r3b",p.local+1));
                publish_confirmed(&entry,&binary::record!({"policy":digest(s)?,"local":p.local+1,"rows":s.tape[p.local],"status":"MAY_ENTER"}))?;
                let logits=l.model.forward(&b.input,Some(&b.valid))?;let(_,loss,targets,examples)=response_objective(&logits,&b,1.,true)?;
                let value=loss.to_scalar::<f32>()?;if !value.is_finite(){return Err(bad("nonfinite study loss"));}
                let graph=loss.backward()?;let mut grads=BTreeMap::new();
                for(n,v)in &l.model.vars{grads.insert(n.clone(),graph.get(v).ok_or_else(||bad("missing gradient"))?.detach());}
                device.synchronize()?;let fb=start.elapsed().as_secs_f64();
                if let Err(e)=ctl.check("before_study_optimizer"){
                    publish_confirmed(&entry.with_file_name(format!("step-{}-segment-{index:03}-discarded.r3b",p.local+1)),&binary::record!({"input":b.tokens,"target":targets,"forward_backward":1,"optimizer":0,"reason":e.to_string()}))?;
                    return Err(e);
                }
                optimizer_entered=true;let began=Instant::now();
                let stats=o.step(&l.model.vars,&grads,&s.config,[1,32,128,512,1024].contains(&(p.local+1)))?;
                device.synchronize()?;let opt=began.elapsed().as_secs_f64();optimizer_entered=false;p.local+=1;
                let st=l.manifest.training.as_mut().unwrap();st.step=s.parent_step+p.local;st.sampler_state=p.local as u64;st.consumed_tokens+=b.tokens as u64;st.target_tokens+=targets as u64;st.train_loss=Some(value as f64);
                l.manifest.optimizer_protocol=Some(o.protocol.clone());
                let row=binary::record!({"local":p.local,"model_step":st.step,"optimizer_local":o.protocol.local_step,"rows":s.tape[p.local-1],"input":b.tokens,"target":targets,"padding":b.input.elem_count()-b.tokens,"examples":examples,"answer_ce":value,"lr":s.config.lr,"stats":stats,"forward_backward_seconds":fb,"optimizer_seconds":opt,"step_seconds":start.elapsed().as_secs_f64()});
                append_row(&mut trace,&row)?;
                if p.local==1||p.local%8==0{println!("MUON_UPDATE arm={} local={} model={} input={} target={} loss={value} opt_s={opt:.4} delta={} remaining={}",ARMS[arm],p.local,st.step,st.consumed_tokens,st.target_tokens,stats["delta_norm"],s.max_updates-p.local);}
            }
            if index==0 {continue;}
            let scores=evaluate(s,arm,l,p,false,&mut ctl)?;p.stop=decision(s,arm,p,&scores)?;p.evaluated=p.local;
            if p.local==512{save_arm(s,index,arm,l,o,p)?;}
            phase+=1;
        }
        let end=phase>=8||ps.iter().any(|p|p.stop.is_some())&&ps[0].local==ps[1].local&&ps.iter().all(|p|p.evaluated==p.local);
        if end {
            for arm in 0..2 {if !ps[arm].fit{evaluate(s,arm,&mut models[arm].0,&ps[arm],true,&mut ctl)?;ps[arm].fit=true;}}
        }
        ctl.seal_completed_no_call()?;Ok(())
    })();
    if let Err(e)=&result {ctl.classify_error(e);}
    let pure_time=ctl.receipt()["observed_conditions"]==binary::record!(["TIME_BUDGET"]);
    // A failed/in-flight update is never certified as a clean restart.
    if optimizer_entered{return result;}
    for arm in 0..models.len(){if ps[arm].local>saved_local[arm]&&!ps[arm].native.ends_with(format!("segment-{index:03}-step-{}.r3m",ps[arm].local)){
        let(l,o)=&mut models[arm];save_arm(s,index,arm,l,o,&mut ps[arm])?;
    }}
    let terminal=ps.iter().all(|p|p.fit);
    let seg=Segment{policy:digest(s)?,previous,phase,arms:ps,success:result.is_ok(),resume:!terminal&&(result.is_ok()||pure_time),control:ctl.receipt(),error:result.as_ref().err().map(ToString::to_string)};
    publish_confirmed(&s.root.join(format!("segment-{index:03}-finished.r3b")),&seg)?;
    println!("MUON_SEGMENT index={index} phase={phase} local={:?} resume={} stop={:?} active={} new_bytes={}",seg.arms.each_ref().map(|p|p.local),seg.resume,seg.arms.each_ref().map(|p|&p.stop),seg.control["elapsed_seconds"],owned_bytes(&s.root)?);
    if pure_time{Ok(())}else{result}
}

fn report(s:&Study)->Result<()> {
    let h=history(s)?;let end=h.last().ok_or_else(||bad("NOT_RUN"))?;let tok=scoring_tokenizer(s)?;
    for arm in 0..2 {let p=&end.arms[arm];
        for at in [32,128,512,1024].into_iter().filter(|&v|v<=p.evaluated){for(name,es,ms)in panels(s,at,false)?{
            let label=format!("eval-{at}-{name}");let dir=s.root.join(ARMS[arm]);let raw=binary::read_value_records(&dir.join(format!("{label}.r3rows")))?;
            if raw.len()!=es.len()+1||raw[0]["policy"]!=digest(s)?||raw[0]["cases"]!=digest(&es)?{return Err(bad("raw panel binding/count"));}
            for(i,(e,row))in es.iter().zip(&raw[1..]).enumerate(){call_attempt(&dir,&label,"generation",&raw[0],e,i,Some(row))?;}
            let r=scored(&name,&es,&ms,&raw[1..],&tok)?;let saved:binary::Value=read_confirmed(&dir.join(format!("{label}-score.r3b")))?;
            if r.as_object().unwrap().iter().any(|(k,v)|saved[k]!=*v)||saved["raw_hash"]!=file_hash(&dir.join(format!("{label}.r3rows")))?{return Err(bad("raw score disagreement"));}
            if raw[0]["model"]!=saved["model"]||at==p.local&&saved["model"]!=checkpoint::metadata(&p.native)?.0.model_content_digest{return Err(bad("endpoint panel/native content mismatch"));}
            println!("MUON_RESULT arm={} local={at} panel={name} full={} QB={} SB={} ALL4={} value={} support={} outside={} parse={} EOS={}",ARMS[arm],r["joint"]["full"],r["joint"]["query_both"],r["joint"]["swap_both"],r["joint"]["all4"],r["value_correct"],r["citation_support_correct"],r["valid_outside_id"],r["parse_failure_rows"],r["joint"]["eos"]);
        }}
        if p.fit {for(name,es,ms)in panels(s,p.local,true)?{
            let dir=s.root.join(ARMS[arm]);let label=format!("eval-{}-{name}",p.local);let raw=binary::read_value_records(&dir.join(format!("{label}.r3rows")))?;
            if raw.len()!=es.len()+1||raw[0]["policy"]!=digest(s)?||raw[0]["cases"]!=digest(&es)?{return Err(bad("fit raw binding/count"));}
            for(i,(e,row))in es.iter().zip(&raw[1..]).enumerate(){call_attempt(&dir,&label,"generation",&raw[0],e,i,Some(row))?;}
            let score=scored(&name,&es,&ms,&raw[1..],&tok)?;let saved:binary::Value=read_confirmed(&dir.join(format!("{label}-score.r3b")))?;
            if score.as_object().unwrap().iter().any(|(k,v)|saved[k]!=*v)||saved["raw_hash"]!=file_hash(&dir.join(format!("{label}.r3rows")))?{return Err(bad("fit raw recount"));}
            let teachers=binary::read_value_records(&dir.join(format!("{label}-teachers.r3rows")))?;
            if teachers.len()!=es.len()+1||teachers[0]!=raw[0]{return Err(bad("fit teacher binding/count"));}
            for(i,(e,row))in es.iter().zip(&teachers[1..]).enumerate(){call_attempt(&dir,&label,"teacher",&raw[0],e,i,Some(row))?;}
            if saved["teacher_nll"].as_f64()!=Some(teacher_ce(&teachers[1..])?){return Err(bad("fit teacher NLL recount"));}
            println!("FIT arm={} local={} FULL={} ALL4={} NLL={} exposure={}",ARMS[arm],p.local,score["joint"]["full"],score["joint"]["all4"],saved["teacher_nll"],saved["actual_train_exposures"]);
        }}
        let mut seen=vec![0usize;7680];let(mut input,mut target,mut padding,mut count)=(0u64,0u64,0u64,0usize);
        for index in 0..h.len(){let path=s.root.join(ARMS[arm]).join(format!("updates-{index:03}.r3rows"));if !path.exists(){continue;}
            for row in binary::read_value_records(&path)?{count+=1;
                if row["local"]!=count||row["model_step"]!=s.parent_step+count||row["optimizer_local"]!=count||row["rows"]!=binary::record!(s.tape[count-1])||row["lr"]!=s.config.lr{return Err(bad("trace clock/tape/LR mismatch"));}
                for &i in &s.tape[count-1]{seen[i]+=1;}input+=row["input"].as_u64().ok_or_else(||bad("input UNKNOWN"))?;target+=row["target"].as_u64().ok_or_else(||bad("target UNKNOWN"))?;padding+=row["padding"].as_u64().ok_or_else(||bad("padding UNKNOWN"))?;
            }
        }
        if count!=p.local{return Err(bad("trace count mismatch"));}
        println!("TRACE arm={} updates={count} exposures={} unique={} input={input} target={target} padding={padding}",ARMS[arm],count*8,seen.iter().filter(|&&v|v>0).count());
        if p.local==s.max_updates {let mut gate=true;
            for name in ["value","citation","S1Q1","word","renamed"]{
                let r:binary::Value=read_confirmed(&s.root.join(ARMS[arm]).join(format!("eval-{}-{name}-score.r3b",p.local)))?;
                let (full,pair,all4,component)=if name=="word"||name=="renamed"{(183,88,44,190)}else{(488,232,116,508)};
                gate&=r["joint"]["full"].as_u64().is_some_and(|n|n>=full)&&r["joint"]["query_both"].as_u64().is_some_and(|n|n>=pair)&&r["joint"]["swap_both"].as_u64().is_some_and(|n|n>=pair)&&r["joint"]["all4"].as_u64().is_some_and(|n|n>=all4)&&r["joint"]["errors"]==0;
                if name!="value"{gate&=r["value_correct"].as_u64().is_some_and(|n|n>=component)&&r["citation_support_correct"].as_u64().is_some_and(|n|n>=component)&&r["valid_outside_id"]==0&&r["parse_failure_rows"]==0;}
            }
            println!("DEVELOPMENT_GATE arm={} pass={gate} general_QA_NOT_RUN Goal1=false",ARMS[arm]);
        }
        println!("ENDPOINT arm={} local={} native={} hash={} stop={:?} fit={}",ARMS[arm],p.local,p.native.display(),p.physical,p.stop,p.fit);
    }
    if end.arms[0].evaluated==end.arms[1].evaluated&&end.arms[0].evaluated>0 {
        for name in ["word","renamed"] {
            let mut exact=vec![];for arm in ARMS{let r:binary::Value=read_confirmed(&s.root.join(arm).join(format!("eval-{}-{name}-score.r3b",end.arms[0].evaluated)))?;exact.push(binary::from_value::<Vec<bool>>(r["joint"]["exact"].clone())?);}
            let mut pair=[0usize;4];for(a,m)in exact[0].chunks_exact(4).zip(exact[1].chunks_exact(4)){let a=a.iter().all(|v|*v);let m=m.iter().all(|v|*v);pair[match(a,m){(true,true)=>0,(false,true)=>1,(true,false)=>2,_=>3}]+=1;}
            println!("PAIRED_BASES {name} both/M_gain/M_loss/neither={pair:?}; word/renamed share semantic bases");
        }
    }
    println!("MUON_USAGE {:?} bytes={} Goal1=false",previous_usage(s,&h)?,owned_bytes(&s.root)?);Ok(())
}
fn scoring_tokenizer(s:&Study)->Result<ByteBpe>{let tok=ByteBpe::load(&s.word_root.join("tokenizer.r3b"))?;if tok.semantic_id()!=s.tokenizer{return Err(bad("frozen scorer tokenizer changed"));}Ok(tok)}
fn review(s:&Study,arm:usize)->Result<()> {
    let h=history(s)?;let end=h.last().ok_or_else(||bad("not run"))?;
    if end.resume||!end.success||!end.arms.iter().all(|p|p.fit){return Err(bad("B requires normally complete endpoint"));}
    report(s)?;let p=&end.arms[arm];let mut cases=vec![];let mut expected=vec![];let mut failures=vec![];
    for((name,es,_),count)in panels(s,p.local,false)?.into_iter().zip([8,6,6,6,6]){
        let raw=binary::read_value_records(&s.root.join(ARMS[arm]).join(format!("eval-{}-{name}.r3rows",p.local)))?;
        for(i,e)in es.into_iter().enumerate(){if i<count{cases.push(e);expected.push(raw[i+1].clone());}else if failures.len()<32&&(raw[i+1]["actual"]!=e.answer||raw[i+1]["finish_reason"]!="stop"){failures.push((e,raw[i+1].clone()));}}
    }
    for(e,r)in failures{cases.push(e);expected.push(r);}
    let label=format!("review-{}",ARMS[arm]);
    let(index,mut ctl)=start_observation(s,&label,&binary::record!({"policy":digest(s)?,"native":p.physical,"cases":digest(&cases)?}))?;
    let result=(||->Result<()>{let l=checkpoint::load(&p.native,Backend::Metal0.open()?,false)?;
        let b=binary::record!({"policy":digest(s)?,"model":p.physical,"cases":digest(&cases)?,"planned":cases.len(),"call_protocol":1});
        let rows=generated(&l,&s.root,&label,&cases,&b,&mut ctl)?;
        for(a,b)in rows.iter().zip(&expected){for field in ["raw_tokens","actual","finish_reason","error","generation_completed"]{if a[field]!=b[field]{return Err(bad("B raw mismatch"));}}}
        ctl.seal_completed_no_call()?;Ok(())})();
    finish_observation(s,&label,index,&mut ctl,&result,binary::record!({"normal":32,"unique":cases.len(),"native":p.physical}))?;result
}
pub fn run(a:Action)->Result<()> {
    match a {
        Action::Prepare{parent,word_root,output}=>prepare(&parent,&word_root,&output),
        Action::Admit{root,review}=>admit(&root,&review),
        Action::Baseline{root}=>baseline(&load_study(&root,true)?),
        Action::Train{root}=>train(&load_study(&root,true)?),
        Action::Report{root}=>report(&load_study(&root,false)?),
        Action::Review{root,arm}=>review(&load_study(&root,true)?,usize::from(arm=="M")),
    }
}

fn norm(t:&Tensor)->Result<f64> {
    let v=t.sqr()?.sum_all()?.to_scalar::<f32>()? as f64;
    if !v.is_finite()||v<0. {return Err(bad("nonfinite optimizer tensor"));}Ok(v.sqrt())
}
fn ns5(h:&Tensor)->Result<Tensor> {
    if h.dtype()!=DType::F32 || h.rank()!=2 || h.dims().contains(&0){return Err(bad("NS5 requires nonempty F32 matrix"));}
    let (m,n)=h.dims2()?;
    let transposed=m>n;
    let h=if transposed{h.t()?.contiguous()?}else{h.contiguous()?};
    let mut x=(&h/(norm(&h)?+1e-7))?;
    for _ in 0..5 {
        let a=x.matmul(&x.t()?)?;
        let b=((&a * -4.7750)?+(a.matmul(&a)?*2.0315)?)?;
        x=((&x*3.4445)?+b.matmul(&x)?)?;
    }
    norm(&x)?;
    Ok(if transposed{x.t()?.contiguous()?}else{x})
}
fn muon_proposal(old:&Tensor,g:&Tensor,previous:&Tensor,c:&TrainConfig)->Result<(Tensor,Tensor)> {
    let (m,n)=old.dims2()?;
    let momentum=((previous*0.95)?+g)?;
    let h=(g+(&momentum*0.95)?)?;
    let next=((old*(1.-c.lr*c.weight_decay))?-(ns5(&h)?*(c.lr*0.2*(m.max(n)as f64).sqrt()))?)?;
    Ok((next.detach(),momentum.detach()))
}
struct Optimizer { protocol:OptimizerProtocol, tensors:BTreeMap<String,Tensor> }
impl Optimizer {
    fn fresh(c:&Config,vars:&BTreeMap<String,Var>,muon:bool,runtime:String,parent:usize)->Result<Self> {
        let protocol=OptimizerProtocol::new(c,muon,runtime,parent)?;
        let mut tensors=BTreeMap::new();
        for (name,v) in vars {
            for prefix in if protocol.roles[name]{vec!["muon.m"]}else{vec!["adam.m","adam.v"]} {
                tensors.insert(format!("{prefix}.{name}"),Tensor::zeros(v.dims(),DType::F32,v.device())?);
            }
        }
        let s=Self{protocol,tensors};s.validate(vars)?;Ok(s)
    }
    fn validate(&self,vars:&BTreeMap<String,Var>)->Result<()> {
        if self.protocol.roles.keys().ne(vars.keys()){return Err(bad("optimizer parameter coverage"));}
        let mut count=0;
        for (name,v) in vars {
            if v.dtype()!=DType::F32 {return Err(bad("optimizer master dtype"));}
            for prefix in if self.protocol.roles[name]{vec!["muon.m"]}else{vec!["adam.m","adam.v"]} {
                let t=self.tensors.get(&format!("{prefix}.{name}")).ok_or_else(||bad("missing optimizer state"))?;
                if t.dims()!=v.dims()||t.dtype()!=DType::F32||!t.device().same_device(v.device()){return Err(bad("optimizer state shape/device"));}
                norm(t)?;count+=1;
            }
        }
        if count!=self.tensors.len(){return Err(bad("extra optimizer state"));}Ok(())
    }
    // The caller must bind native family/runtime/policy before this private step.
    fn step(&mut self,vars:&BTreeMap<String,Var>,grads:&BTreeMap<String,Tensor>,c:&TrainConfig,observe:bool)->Result<binary::Value> {
        self.validate(vars)?;
        if grads.keys().ne(vars.keys())||self.protocol.local_step>=i32::MAX as usize||c.lr!=3e-5{return Err(bad("gradient coverage/local clock/registered LR"));}
        let t=self.protocol.local_step+1;
        let mut total=0.;
        for (name,v) in vars {
            let g=&grads[name];
            if g.dims()!=v.dims()||g.dtype()!=DType::F32||!g.device().same_device(v.device()){return Err(bad("gradient shape/device/dtype"));}
            total+=norm(g)?.powi(2);
        }
        let total=total.sqrt();let clip=(c.clip/(total+1e-12)).min(1.);
        let mut pending=vec![];let mut observations=BTreeMap::new();let mut delta2=0.;
        for (name,v) in vars {
            let g=(&grads[name].detach()*clip)?;let old=v.as_detached_tensor();
            let (next,states,momentum)=if self.protocol.roles[name] {
                let key=format!("muon.m.{name}");
                let(next,m)=muon_proposal(&old,&g,&self.tensors[&key],c)?;
                (next,vec![(key,m.detach())],m)
            } else {
                let mk=format!("adam.m.{name}");let vk=format!("adam.v.{name}");
                let (next,m,v)=Adam::proposal(&old,&g,&self.tensors[&mk],&self.tensors[&vk],c,t,c.lr)?;
                (next,vec![(mk,m.clone()),(vk,v)],m)
            };
            for (_,v) in &states{norm(v)?;}
            let d=norm(&(&next-&old)?)?;delta2+=d*d;
            if observe {let w=norm(&old)?;observations.insert(name.clone(),binary::record!({"gradient_l2":norm(&g)?,"moment_l2":norm(&momentum)?,"delta_l2":d,"weight_l2":w,"delta_weight_ratio":if w>0.{Some(d/w)}else{None},"muon":self.protocol.roles[name]}));}
            pending.push((name.clone(),next.detach(),states));
        }
        // All finite proposals are prepared before any master/state is mutated.
        for(name,next,states)in pending{vars[&name].set(&next)?;for(k,v)in states{self.tensors.insert(k,v);}}
        self.protocol.local_step=t;
        Ok(binary::record!({"local_step":t,"gradient_norm":total,"clip":clip,"delta_norm":delta2.sqrt(),"parameters":observations}))
    }
}

#[cfg(all(test,feature="metal"))]
mod tests {
    use super::*;
    fn oracle(v:&[f64],m:usize,n:usize)->Vec<f64>{
        let (a,b)=if m>n{(n,m)}else{(m,n)};
        let mut x=vec![0.;a*b];
        let norm=v.iter().map(|v|v*v).sum::<f64>().sqrt()+1e-7;
        for i in 0..a{for j in 0..b{x[i*b+j]=v[if m>n{j*n+i}else{i*n+j}]/norm;}}
        for _ in 0..5 {
            let mut gram=vec![0.;a*a];let mut poly=vec![0.;a*a];let mut next=vec![0.;a*b];
            for i in 0..a{for j in 0..a{gram[i*a+j]=(0..b).map(|k|x[i*b+k]*x[j*b+k]).sum();}}
            for i in 0..a{for j in 0..a{poly[i*a+j]=-4.7750*gram[i*a+j]+2.0315*(0..a).map(|k|gram[i*a+k]*gram[k*a+j]).sum::<f64>();}}
            for i in 0..a{for j in 0..b{next[i*b+j]=3.4445*x[i*b+j]+(0..a).map(|k|poly[i*a+k]*x[k*b+j]).sum::<f64>();}}
            x=next;
        }
        let mut out=vec![0.;m*n];for i in 0..a{for j in 0..b{out[if m>n{j*n+i}else{i*n+j}]=x[i*b+j];}}out
    }
    fn compare(label:&str,expected:&[f64],actual:&Tensor,update:bool)->Result<()> {
        let a=actual.flatten_all()?.to_vec1::<f32>()?;assert_eq!(a.len(),expected.len());
        let(mut error,mut dot,mut nn,mut rr,mut max)=(0.,0.,0.,0.,0f64);
        for(&r,&x)in expected.iter().zip(&a){let x=x as f64;assert!(r.is_finite()&&x.is_finite());let d=(r-x).abs();max=max.max(d);
            assert!(d<=1e-4+1e-3*r.abs(),"{label} r{r} x{x}");error+=d*d;dot+=r*x;nn+=x*x;rr+=r*r;}
        let floor=1e-6*(a.len()as f64).sqrt();
        if update {if rr.sqrt()>floor{assert!(error.sqrt()/rr.sqrt()<=0.02,"{label} NRMSE");assert!(dot/(rr*nn).sqrt()>=0.999,"{label} cosine");}else{assert!(max<=1e-7,"{label} nearzero {max}");}}
        println!("NUMERIC {label} N={} max={max:e} rms={:e} reference_norm={:e} error_norm={:e} cosine={:?}",a.len(),(error/a.len()as f64).sqrt(),rr.sqrt(),error.sqrt(),if rr*nn>0.{Some(dot/(rr*nn).sqrt())}else{None});Ok(())
    }
    #[test]
    #[ignore="explicit actual Metal; primitive only"]
    fn scalar_oracle_cpu_metal_ns_and_updates()->Result<()> {
        let gpu=Backend::Metal0.open()?;let c=TrainConfig{lr:3e-5,weight_decay:0.01,..Default::default()};let mut count=0;
        for(m,n)in [(3,3),(2,5),(5,2)]{for variant in 0..3{
            let gradient=(0..m*n).map(|i|match variant{0=>0.,1=>(i%n)as f64*0.07,_=>((i*7%13)as f64-6.)*0.03125}).collect::<Vec<_>>();
            let mut momentum=(0..m*n).map(|i|(i%3)as f64*0.02-0.01).collect::<Vec<_>>();
            let mut old=vec![0.01;m*n];let mut am=vec![0.02;m*n];let mut av=vec![0.03;m*n];let mut aw=old.clone();
            for step in 1..=2 {
                let before_momentum=momentum.clone();
                for(i,g)in gradient.iter().enumerate(){momentum[i]=0.95*momentum[i]+g;}
                let h=gradient.iter().zip(&momentum).map(|(g,m)|g+0.95*m).collect::<Vec<_>>();let o=oracle(&h,m,n);
                let next=old.iter().zip(&o).map(|(w,o)|(1.-c.lr*c.weight_decay)*w-c.lr*0.2*(m.max(n)as f64).sqrt()*o).collect::<Vec<_>>();
                let before_am=am.clone();let before_av=av.clone();
                let anext=(0..m*n).map(|i|{am[i]=c.beta1*am[i]+(1.-c.beta1)*gradient[i];av[i]=c.beta2*av[i]+(1.-c.beta2)*gradient[i]*gradient[i];(1.-c.lr*c.weight_decay)*aw[i]-c.lr*(am[i]/(1.-c.beta1.powi(step)))/((av[i]/(1.-c.beta2.powi(step))).sqrt()+c.eps)}).collect::<Vec<_>>();
                for device in [&Device::Cpu,&gpu] {
                    let tensor=|a:&[f64]|Tensor::from_vec(a.iter().map(|&v|v as f32).collect::<Vec<_>>(),(m,n),device);
                    let actual=ns5(&tensor(&h)?)?;compare("NS5",&o,&actual,false)?;
                    let base=tensor(&old)?;let(updated,mm)=muon_proposal(&base,&tensor(&gradient)?,&tensor(&before_momentum)?,&c)?;
                    compare("Muon-momentum",&momentum,&mm,false)?;
                    compare("Muon-delta",&next.iter().zip(&old).map(|(a,b)|a-b).collect::<Vec<_>>(),&(&updated-&base)?,true)?;
                    let base=tensor(&aw)?;let (updated,mm,vv)=Adam::proposal(&base,&tensor(&gradient)?,&tensor(&before_am)?,&tensor(&before_av)?,&c,step as usize,c.lr)?;
                    compare("Adam-delta",&anext.iter().zip(&aw).map(|(a,b)|a-b).collect::<Vec<_>>(),&(&updated-&base)?,true)?;
                    compare("Adam-m",&am,&mm,false)?;compare("Adam-v",&av,&vv,false)?;count+=1;
                }old=next;aw=anext;
            }
        }}
        // Variant0 above has zero gradient with nonzero history. All-zero H is also valid.
        let h=Tensor::zeros((2,3),DType::F32,&gpu)?;assert_eq!(norm(&ns5(&h)?)?,0.);
        println!("FIXTURES {} SMALL_updates0 TINY_updates0",count+1);Ok(())
    }
    #[test]
    #[ignore="explicit actual SMALL-shape primitive, no model update"]
    fn small_shape_ns_cpu_metal()->Result<()> {
        let gpu=Backend::Metal0.open()?;let c=Config::small(562);let shapes=c.shapes();
        for name in ["layer.0.q","layer.0.k","layer.0.up"]{
            let shape=&shapes[name];let values=(0..shape.iter().product()).map(|i|((i*17%97)as f32-48.)/256.).collect::<Vec<_>>();
            let cpu=Tensor::from_vec(values.clone(),shape.as_slice(),&Device::Cpu)?;let metal=Tensor::from_vec(values,shape.as_slice(),&gpu)?;
            let reference=ns5(&cpu)?;let actual=ns5(&metal)?;gpu.synchronize()?;
            compare(name,&reference.flatten_all()?.to_vec1::<f32>()?.iter().map(|&v|v as f64).collect::<Vec<_>>(),&actual,true)?;
            let c=TrainConfig{lr:3e-5,..Default::default()};
            let old=Tensor::full(0.01f32,shape.as_slice(),&Device::Cpu)?;let prev=(&cpu*0.3)?;
            let (a,_)=muon_proposal(&old,&cpu,&prev,&c)?;
            let (b,_)=muon_proposal(&old.to_device(&gpu)?,&metal,&prev.to_device(&gpu)?,&c)?;
            compare("SMALL-shape-actual-delta",&(&a-&old)?.flatten_all()?.to_vec1::<f32>()?.iter().map(|&v|v as f64).collect::<Vec<_>>(),&(&b-&old.to_device(&gpu)?)?,true)?;
        }println!("FIXTURES6 SMALL_updates0 TINY_updates0");Ok(())
    }
    fn tiny()->Result<(Transformer,ByteBpe,TrainConfig,Vec<Sample>)>{
        let tok=ByteBpe::train(&[b"native training fixture".to_vec()],&"a".repeat(64),264)?;
        let model=Transformer::init(Config::tiny(tok.vocab_size()),20260924,Backend::Metal0.open()?)?;
        let c=TrainConfig{lr:3e-5,warmup:0,microbatch:8,accumulation:1,seq_len:32,max_steps:102,max_tokens:1_000_000,budget_start_step:100,..Default::default()};
        let samples=(0..8).map(|row|Sample{tokens:(0..12-row%3).map(|n|8+(n+row)%32).chain([EOS]).collect(),response_start:7,curriculum:false}).collect();
        Ok((model,tok,c,samples))
    }
    fn gradients(m:&Transformer,ss:&[Sample],micro:usize)->Result<BTreeMap<String,Tensor>>{
        let mut out=BTreeMap::new();for at in (0..8).step_by(micro){let b=batch(ss,&(at..at+micro).collect::<Vec<_>>(),&m.device)?;
            let logits=m.forward(&b.input,Some(&b.valid))?;let (_,loss,_,mass)=response_objective(&logits,&b,1.,true)?;let g=loss.backward()?;
            for(n,v)in &m.vars{let weighted=(g.get(v).ok_or_else(||bad("TINY gradient"))?*(mass as f64/8.))?.detach();let t=if let Some(old)=out.remove(n){(old+weighted)?}else{weighted};out.insert(n.clone(),t);}}
        Ok(out)
    }
    #[test]
    #[ignore="actual TINY Metal roles/negative boundaries, no optimizer calls"]
    fn roles_and_rejection()->Result<()> {
        let (m,tok,c,_)=tiny()?;let mut opt=Optimizer::fresh(&m.config,&m.vars,true,"a".repeat(64),100)?;
        assert_eq!(opt.protocol.roles.values().filter(|&&v|v).count(),7*m.config.layers);assert!(!opt.protocol.roles["embedding"]);assert!(!opt.protocol.roles["layer.0.q_norm"]);
        let original=m.weights_content_id()?;let mut g=m.vars.iter().map(|(n,v)|Ok((n.clone(),Tensor::zeros(v.dims(),DType::F32,&m.device)?))).collect::<Result<BTreeMap<_,_>>>()?;
        g.remove("embedding");assert!(opt.step(&m.vars,&g,&c,false).is_err());
        g.insert("embedding".into(),Tensor::full(f32::NAN,m.vars["embedding"].dims(),&m.device)?);assert!(opt.step(&m.vars,&g,&c,false).is_err());assert_eq!(original,m.weights_content_id()?);
        opt.tensors.remove("muon.m.layer.0.q");assert!(opt.validate(&m.vars).is_err());
        let mut st=fixture_state(&c,&tok,100);let mut p=opt.protocol.clone();p.family="UNKNOWN".into();assert!(p.validate(&m.config,&st).is_err());p=opt.protocol.clone();st.step=101;assert!(p.validate(&m.config,&st).is_err());
        println!("ROLES/INVALID PASS optimizer0 generation0");Ok(())
    }
    fn fixture_state(c:&TrainConfig,tok:&ByteBpe,step:usize)->TrainingState{
        let mut s=TrainingState{resume_binding:None,contrast16:false,parent_checkpoint_hash:Some("b".repeat(64)),config:c.clone(),step,consumed_tokens:0,target_tokens:0,sampler_state:(step-100)as u64,corpus_hash:tok.train_hash.clone(),validation_hash:"c".repeat(64),previous_corpora:vec![],initial_weight_hash:String::new(),train_loss:None,validation_loss:None};
        let mut b=checkpoint::ResumeBinding::default_for(&s,tok);b.family=checkpoint::ANSWER_MEAN_FAMILY;b.normalizer=2;b.execution=1;s.resume_binding=Some(b);s
    }
    #[test]
    #[ignore="actual TINY batch8 vs4+4;4 optimizer calls"]
    fn answer_accumulation_updates()->Result<()> {
        for muon in [false,true]{let(m,_,c,ss)=tiny()?;let(n,_,_,_)=tiny()?;
            let a=gradients(&m,&ss,8)?;let b=gradients(&n,&ss,4)?;
            for(k,v)in &a{compare(k,&v.flatten_all()?.to_vec1::<f32>()?.iter().map(|&v|v as f64).collect::<Vec<_>>(),&b[k],false)?;}
            let mut ao=Optimizer::fresh(&m.config,&m.vars,muon,"a".repeat(64),100)?;let mut bo=Optimizer::fresh(&n.config,&n.vars,muon,"a".repeat(64),100)?;
            ao.step(&m.vars,&a,&c,false)?;bo.step(&n.vars,&b,&c,false)?;
            for(k,v)in &m.vars{compare(k,&v.flatten_all()?.to_vec1::<f32>()?.iter().map(|&v|v as f64).collect::<Vec<_>>(),&n.vars[k],false)?;}
        }println!("TINY_updates4 forward6 backward6 generation0");Ok(())
    }
    fn fixture_save(path:&Path,m:&Transformer,tok:&ByteBpe,c:&TrainConfig,o:&Optimizer)->Result<()> {
        let mut manifest=checkpoint::initialized(m,tok,20260924,"d".repeat(64))?;
        let mut s=fixture_state(c,tok,100+o.protocol.local_step);s.initial_weight_hash=manifest.initial_weight_hash.clone();
        s.consumed_tokens=o.protocol.local_step as u64*80;s.target_tokens=o.protocol.local_step as u64*24;
        manifest.training=Some(s);manifest.optimizer_protocol=Some(o.protocol.clone());manifest.status="DIAGNOSTIC_COMPLETE".into();
        checkpoint::save(path,m,tok,manifest,&o.tensors)?;Ok(())
    }
    #[test]
    #[ignore="typed native negative cases and actual no-call collector; optimizer0 generation0"]
    fn native_policy_and_completed_no_call()->Result<()> {
        let(m,tok,c,_)=tiny()?;let opt=Optimizer::fresh(&m.config,&m.vars,true,"a".repeat(64),100)?;
        let dir=std::env::temp_dir().join(format!("replica-muon-boundary-{}-{}",std::process::id(),std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()));std::fs::create_dir(&dir)?;
        fixture_save(&dir.join("native.r3m"),&m,&tok,&c,&opt)?;
        let mut l=checkpoint::load(&dir.join("native.r3m"),m.device.clone(),true)?;
        let original=l.manifest.clone();
        for kind in 0..4 {
            l.manifest=original.clone();let p=l.manifest.optimizer_protocol.as_mut().unwrap();
            match kind{0=>p.family="ADAM".into(),1=>p.roles.insert("embedding".into(),true).map(|_|()).unwrap(),2=>p.local_step=1,_=>p.runtime_digest.clear()};
            assert!(checkpoint::save(&dir.join(format!("invalid{kind}")),&l.model,&l.tokenizer,l.manifest.clone(),&l.optimizer).is_err());
        }
        l.manifest=original;
        let e=Episode{id:"fixture".into(),category:0,family:"fixture".into(),binding:"fixture".into(),sequence:"fixture".into(),answer:"expected".into(),request:ModelRequest{request_id:String::new(),system:String::new(),input:"question".into(),evidence:EvidenceBundle::default(),limits:GenerationLimits{context_tokens:64,max_tokens:4,timeout_ms:1000}}};
        let b=binary::record!({"model":file_hash(&dir.join("native.r3m"))?,"call_protocol":1,"cases":digest(&[e.clone()])?});
        let mut initial=RunControl::new(std::sync::Arc::new(AtomicBool::new(false)),std::time::Duration::from_secs(10),u64::MAX)?;
        let attempt=prepare_call(&dir,"complete","generation",&b,&e,0)?;
        // Synthetic RETURNED fixture, deliberately wrong; never a model result.
        let row=binary::record!({"id":e.id,"expected":e.answer,"raw_tokens":[EOS],"actual":"","finish_reason":"stop","error":null,"generation_completed":true,"attempt":attempt.file_name().unwrap().to_string_lossy()});
        let mut f=std::fs::OpenOptions::new().write(true).create_new(true).open(dir.join("complete.r3rows"))?;append_row(&mut f,&b)?;append_row(&mut f,&row)?;resolve_call(&attempt,Some(&row),&initial)?;
        initial.seal_completed_no_call()?;
        let mut zero=RunControl::new(std::sync::Arc::new(AtomicBool::new(false)),std::time::Duration::ZERO,u64::MAX)?;zero.set_call_limits(0,0);
        assert_eq!(generated(&l,&dir,"complete",&[e.clone()],&b,&mut zero)?,vec![row]);zero.seal_completed_no_call()?;
        assert_eq!(zero.receipt()["generation_calls"],0);
        let mut zero=RunControl::new(std::sync::Arc::new(AtomicBool::new(false)),std::time::Duration::ZERO,u64::MAX)?;
        assert!(generated(&l,&dir,"missing",&[e.clone()],&b,&mut zero).is_err());
        prepare_call(&dir,"unknown","generation",&b,&e,0)?;
        let mut zero=RunControl::new(std::sync::Arc::new(AtomicBool::new(false)),std::time::Duration::ZERO,u64::MAX)?;
        assert!(generated(&l,&dir,"unknown",&[e.clone()],&b,&mut zero).is_err());
        let mut cancelled=RunControl::new(std::sync::Arc::new(AtomicBool::new(true)),std::time::Duration::from_secs(1),u64::MAX)?;
        assert!(cancelled.seal_completed_no_call().is_err());
        println!("NATIVE_BOUNDARY evidence={} optimizer0 generation0 teacher0",dir.display());Ok(())
    }
    #[test]
    #[ignore="actual TINY new process continuation2 vs1+1;8 optimizer total"]
    fn native_new_process_restart()->Result<()> {
        if let Ok(dir)=std::env::var("R3_MUON_TEST_CHILD") {
            let path=PathBuf::from(dir);let mut l=checkpoint::load(&path.join("one.r3m"),Backend::Metal0.open()?,true)?;
            let o=l.manifest.optimizer_protocol.clone().ok_or_else(||bad("missing optimizer protocol"))?;
            let c=l.manifest.training.as_ref().unwrap().config.clone();o.validate(&l.model.config,l.manifest.training.as_ref().unwrap())?;
            let (_,_,_,ss)=tiny()?;let mut opt=Optimizer{protocol:o,tensors:std::mem::take(&mut l.optimizer)};
            let g=gradients(&l.model,&ss,8)?;opt.step(&l.model.vars,&g,&c,false)?;fixture_save(&path.join("split.r3m"),&l.model,&l.tokenizer,&c,&opt)?;return Ok(());
        }
        for muon in [false,true]{
            let dir=std::env::temp_dir().join(format!("replica-muon-{}-{}-{}",std::process::id(),muon,std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()));std::fs::create_dir(&dir)?;
            let(m,tok,c,ss)=tiny()?;let mut opt=Optimizer::fresh(&m.config,&m.vars,muon,"a".repeat(64),100)?;
            let g=gradients(&m,&ss,8)?;opt.step(&m.vars,&g,&c,false)?;fixture_save(&dir.join("one.r3m"),&m,&tok,&c,&opt)?;
            let g=gradients(&m,&ss,8)?;opt.step(&m.vars,&g,&c,false)?;fixture_save(&dir.join("two.r3m"),&m,&tok,&c,&opt)?;
            let result=std::process::Command::new(std::env::current_exe()?).args(["--exact","training::fresh::muon::tests::native_new_process_restart","--ignored","--nocapture","--test-threads=1"]).env("R3_MUON_TEST_CHILD",&dir).status()?;assert!(result.success());
            let a=checkpoint::load(&dir.join("two.r3m"),Device::Cpu,true)?;let b=checkpoint::load(&dir.join("split.r3m"),Device::Cpu,true)?;
            assert_eq!(a.model.weights_content_id()?,b.model.weights_content_id()?);assert_eq!(optimizer_hash(&a.optimizer)?,optimizer_hash(&b.optimizer)?);assert_eq!(a.manifest.optimizer_protocol,b.manifest.optimizer_protocol);assert_eq!(a.manifest.training,b.manifest.training);
            let inference=checkpoint::load(&dir.join("split.r3m"),Device::Cpu,false)?;assert_eq!(inference.model.weights_content_id()?,b.model.weights_content_id()?);assert!(inference.optimizer.is_empty());
            println!("RESTART muon={muon} evidence={} optimizer3 weights/state/clock/cursor EXACT",dir.display());
        }println!("TINY_updates6 generation0");Ok(())
    }
    #[test]
    #[ignore="new process actual train caller at exhausted time; synthetic RETURNED, model calls0"]
    fn final_evaluation_only_process()->Result<()> {
        if let Ok(root)=std::env::var("R3_MUON_FINALIZE_CHILD"){
            let s:Study=read_confirmed(&PathBuf::from(root).join("plan.r3b"))?;
            train(&s)?;let h=history(&s)?;let last=h.last().unwrap();
            assert!(!last.resume&&last.success&&last.arms.iter().all(|p|p.fit));
            assert_eq!(last.control["generation_calls"],0);assert_eq!(last.control["teacher_calls"],0);
            assert_eq!(h[0].arms.each_ref().map(|p|&p.physical),last.arms.each_ref().map(|p|&p.physical));return Ok(());
        }
        let source=PathBuf::from(std::env::var("R3_MUON_PREPARATION").map_err(|_|bad("explicit prepared study path needed for caller fixture"))?);
        let mut s:Study=read_confirmed(&source.join("plan.r3b"))?;
        let dir=std::env::temp_dir().join(format!("replica-muon-finalize-{}-{}",std::process::id(),std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()));std::fs::create_dir(&dir)?;s.root=dir.canonicalize()?;
        s.runtime=RuntimeProfile::capture(Backend::Metal0,&Backend::Metal0.open()?)?;
        publish_confirmed(&s.root.join("plan.r3b"),&s)?;
        publish_confirmed(&s.root.join("review-a.r3b"),&binary::record!({"active_seconds":s.active_cap,"synthetic_fixture":true}))?;
        publish_confirmed(&s.root.join("baseline-finished.r3b"),&binary::record!({"success":true,"synthetic_fixture":true}))?;
        let tok=ByteBpe::load(&s.word_root.join("tokenizer.r3b"))?;
        let(corpus,_,_)=inputs(&s)?;
        let mut cfg=Config::tiny(tok.vocab_size());cfg.context=2048;cfg.profile="NATIVE_TRPP_EXPERIMENTAL_V1".into();
        let mut ps=vec![];
        for arm in 0..2 {
            let root=s.root.join(ARMS[arm]);std::fs::create_dir(&root)?;
            let model=Transformer::init(cfg.clone(),17,Device::Cpu)?;
            let mut op=Optimizer::fresh(&model.config,&model.vars,arm==1,digest(&s.runtime)?,s.parent_step)?;op.protocol.local_step=s.max_updates;
            let mut manifest=checkpoint::initialized(&model,&tok,17,"d".repeat(64))?;
            let mut st=fixture_state(&s.config,&tok,s.parent_step+s.max_updates);st.parent_checkpoint_hash=Some(s.parent_hash.clone());st.sampler_state=s.max_updates as u64;
            st.corpus_hash=corpus.manifest.train.sha256.clone();st.validation_hash=corpus.manifest.validation.sha256.clone();st.previous_corpora.push(tok.train_hash.clone());
            st.consumed_tokens=s.config.budget_start_tokens;st.initial_weight_hash=manifest.initial_weight_hash.clone();
            st.resume_binding=Some(bind(&s,arm,&st,&tok)?);manifest.training=Some(st);manifest.optimizer_protocol=Some(op.protocol);manifest.status="DIAGNOSTIC_COMPLETE".into();
            let native=root.join("tiny.r3m");checkpoint::save(&native,&model,&tok,manifest,&op.tensors)?;
            let p=Progress{local:s.max_updates,native:native.clone(),physical:file_hash(&native)?,stop:None,evaluated:s.max_updates,fit:false};
            let l=checkpoint::load(&native,Device::Cpu,false)?;
            for(name,episodes,meta)in panels(&s,p.local,true)?{
                let label=format!("eval-{}-{name}",p.local);
                let binding=binary::record!({"policy":digest(&s)?,"arm":ARMS[arm],"model":l.model.weights_content_id()?,"local":p.local,"absolute":s.parent_step+p.local,"runtime":s.runtime,"cases":digest(&episodes)?,"metadata":digest(&meta)?,"tokenizer":s.tokenizer,"planned":episodes.len(),"call_protocol":1});
                let mut raw=std::fs::OpenOptions::new().write(true).create_new(true).open(root.join(format!("{label}.r3rows")))?;append_row(&mut raw,&binding)?;
                let mut teacher=std::fs::OpenOptions::new().write(true).create_new(true).open(root.join(format!("{label}-teachers.r3rows")))?;append_row(&mut teacher,&binding)?;
                let ctl=RunControl::new(std::sync::Arc::new(AtomicBool::new(false)),std::time::Duration::from_secs(3600),u64::MAX)?;
                for(i,e)in episodes.iter().enumerate(){
                    let attempt=prepare_call(&root,&label,"generation",&binding,e,i)?;let output=tok.encode(e.answer.as_bytes())?;let mut ids=output.clone();ids.push(EOS);
                    let row=binary::record!({"row_version":2,"id":e.id,"expected":e.answer,"question":e.request.input,"generated_evidence":e.request.evidence,"raw_tokens":ids,"actual":e.answer,"error":null,"finish_reason":"stop","generation_completed":true,"generation":{"tokens":output,"generated":ids.len(),"finish":"stop"},"exact_match":true,"attempt":attempt.file_name().unwrap().to_string_lossy()});
                    append_row(&mut raw,&row)?;resolve_call(&attempt,Some(&row),&ctl)?;
                    let attempt=prepare_call(&root,&label,"teacher",&binding,e,i)?;let row=binary::record!({"ordinal":i,"id":e.id,"case":digest(e)?,"teacher":{"mean_nll":1.,"target_tokens_including_eos":ids.len()},"attempt":attempt.file_name().unwrap().to_string_lossy()});
                    append_row(&mut teacher,&row)?;resolve_call(&attempt,Some(&row),&ctl)?;
                }
            }ps.push(p);
        }
        let arms:[Progress;2]=ps.try_into().ok().unwrap();let control=binary::record!({"elapsed_seconds":0.,"generation_calls":0,"teacher_calls":0,"observed_conditions":["TIME_BUDGET"]});
        publish_confirmed(&s.root.join("segment-000-started.r3b"),&binary::record!({"policy":digest(&s)?}))?;
        publish_confirmed(&s.root.join("segment-000-finished.r3b"),&Segment{policy:digest(&s)?,previous:None,phase:8,arms,success:false,resume:true,control,error:Some("TIME_BUDGET".into())})?;
        let status=std::process::Command::new(std::env::current_exe()?).args(["--exact","training::fresh::muon::tests::final_evaluation_only_process","--ignored","--nocapture","--test-threads=1"]).env("R3_MUON_FINALIZE_CHILD",&s.root).status()?;assert!(status.success());
        println!("ACTUAL_CALLER exhausted7200 fresh_process finalfit/score/saved_native unchanged; fixture_rows_only; optimizer0 generation0 teacher0 path={}",s.root.display());Ok(())
    }
}
