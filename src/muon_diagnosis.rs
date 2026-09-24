//! Bounded observations of the two preserved, independently checked endpoints.
//! No optimizer, backward, training admission or product inference routing here.
use super::*;
type Value=binary::Value;
const ID:&str="R3-MUON-ENDPOINT-DIAGNOSIS-1.0";
const CLOSURE:&str="R3-TEACHER-DEVICE-CLOSURE-1.0";

#[derive(Subcommand)]
pub enum Action {
    Successor { #[arg(long)] predecessor:PathBuf, #[arg(long)] executor:PathBuf, #[arg(long)] audit:PathBuf, #[arg(long)] output:PathBuf },
    Smoke { #[arg(long)] root:PathBuf, #[arg(long,value_parser=["parent","A","M"])] model:String },
    ReadSmoke { #[arg(long)] root:PathBuf },
    AdmitCaller { #[arg(long)] root:PathBuf, #[arg(long)] review:PathBuf },
    Prepare { #[arg(long)] original:PathBuf, #[arg(long)] audit:PathBuf, #[arg(long)] output:PathBuf },
    Admit { #[arg(long)] root:PathBuf, #[arg(long)] review:PathBuf },
    Observe { #[arg(long)] root:PathBuf, #[arg(long,value_parser=["missing","train-A","train-M","teacher-parent","teacher-A","teacher-M","replay-A","replay-M","check-parent","check-A","check-M"])] lane:String },
    Report { #[arg(long)] root:PathBuf },
    Close { #[arg(long)] root:PathBuf },
}
#[derive(Clone,Serialize,Deserialize)]
struct RawRef { arm:usize,name:String,path:PathBuf,header:Value,count:usize }
#[derive(Clone,Serialize,Deserialize)]
struct Diagnostic {
    contract:String,root:PathBuf,original:Study,endpoints:[Progress;2],refs:BTreeMap<PathBuf,String>,
    panels:Vec<RawRef>,source:String,runtime:RuntimeProfile,teacher_cases:Vec<usize>,
    generation_cap:usize,teacher_cap:usize,active_cap:f64,segment_cap:f64,bytes_cap:u64,
    #[serde(default,skip_serializing_if="Option::is_none")]
    predecessor:Option<Predecessor>,
}
#[derive(Clone,Serialize,Deserialize)]
struct Predecessor { root:PathBuf,policy:String,failed_row:String,executor:PathBuf,manifests:BTreeMap<String,Value> }
fn source()->Result<String>{digest(&(sources()?,neural::hash(include_bytes!("muon_diagnosis.rs"))))}
fn bind(refs:&mut BTreeMap<PathBuf,String>,p:&Path)->Result<()> {refs.insert(p.canonicalize()?,file_hash(p)?);Ok(())}
fn check_refs(d:&Diagnostic)->Result<()> {
    for(p,h)in &d.refs {if file_hash(p)?!=*h||pending_path(p).exists(){return Err(bad("BLOCKED_INPUT_INTEGRITY: changed/pending original"));}}
    Ok(())
}
fn original_stop(h:&[Segment],row:&Value)->Result<()> {
    let e=h.last().ok_or_else(||bad("original terminal absent"))?;
    if e.success||e.resume||e.arms.iter().any(|p|p.local!=512||p.fit)
        ||e.arms[0].evaluated!=512||e.arms[1].evaluated!=128
        ||e.arms[0].stop.as_deref()!=Some("PERSISTENT_RETENTION")||e.arms[1].stop.is_some()
        ||e.control["observed_conditions"]!=binary::record!(["INTEGRITY_FAIL"])
        ||row["error_class"]!="strict_utf8"||row["generation_completed"]!=true
        ||!row["generation_error"].is_null()||!row["command_stop"].is_null()
        ||row["finish_reason"]!="stop"||!row["actual"].is_null()
        ||e.error.as_deref()!=Some(bad(&format!("native generation error {}",row["error"])).to_string().as_str()) {
        return Err(bad("BLOCKED_INPUT_INTEGRITY: original failure is not the checked returned decode boundary"));
    }
    for old in &h[..h.len()-1] {
        if old.control["observed_conditions"].as_array().is_none_or(|a|a.iter().any(|v|v!="TIME_BUDGET")) {
            return Err(bad("BLOCKED_INPUT_INTEGRITY: mixed original execution failure"));
        }
    }Ok(())
}
fn check_row(e:&Episode,r:&Value,l:&checkpoint::Loaded)->Result<()> {
    verify_output_result(r,&l.tokenizer)?;
    let p=l.tokenizer.prepare_with_framing(&e.request,l.manifest.framing()?,l.model.config.context as u32,&l.model.config.id()?)?;
    if r["id"]!=e.id||r["expected"]!=e.answer||r["request_digest"]!=digest(&e.request)?
        ||r["native_prompt_digest"]!=p.token_digest||r["prompt_digest"]!=digest(&p.token_ids)?
        ||r["provided"]!=binary::record!(p.provided)||r["generated_question"]!=e.request.input
        ||r["generated_evidence"]!=binary::record!(e.request.evidence) {return Err(bad("case/prompt/raw mismatch"));}
    Ok(())
}
fn prepare(original:&Path,audit:&Path,root:&Path)->Result<()> {
    let s=load_study(original,false)?;let h=history(&s)?;
    let end=h.last().ok_or_else(||bad("original missing"))?;
    let a=std::fs::read_to_string(audit)?;
    if !a.contains("PARTIAL_EVIDENCE_RECOUNT VERIFIED same_local=512")||!a.contains("B_FULL=BLOCKED_RUNTIME") {
        return Err(bad("independent original raw/state audit absent"));
    }
    let (corpus,_,_)=inputs(&s)?;let mut refs=BTreeMap::new();
    for p in [s.root.join("plan.r3b"),s.root.join("preparation.r3b"),s.root.join("review-a.r3b"),s.parent.clone(),
        s.word_root.join("plan.r3b"),s.word_root.join("corpus.r3cor"),s.word_root.join("metadata.r3b"),s.word_root.join("tokenizer.r3b"),audit.to_path_buf()] {bind(&mut refs,&p)?;}
    for i in 0..h.len(){for suffix in ["started","finished"]{bind(&mut refs,&s.root.join(format!("segment-{i:03}-{suffix}.r3b")))?;}}
    for label in ["baseline","review-A","review-M"] {
        let observations=observation_history(&s,label)?;
        for(i,o)in observations.iter().enumerate(){
            if o["control"]["observed_conditions"].as_array().is_none_or(|a|a.iter().any(|v|v!="TIME_BUDGET")){return Err(bad("original observation execution failure"));}
            for suffix in ["started","finished"]{bind(&mut refs,&s.root.join(format!("{label}-segment-{i:03}-{suffix}.r3b")))?;}
        }
    }
    let mut panels_out=vec![];let mut error_row=None;
    let parent=checkpoint::load(&s.parent,Device::Cpu,false)?;
    if parent.model.weights_content_id()?!=s.parent_content||parent.tokenizer.semantic_id()!=s.tokenizer
        ||s.max_updates!=1024||s.tape.len()!=1024{return Err(bad("original parent/tape mismatch"));}
    for arm in 0..2 {
        let p=&end.arms[arm];bind(&mut refs,&p.native)?;
        let l=checkpoint::load(&p.native,Device::Cpu,false)?;
        let st=l.manifest.training.as_ref().ok_or_else(||bad("native state missing"))?;
        let protocol=l.manifest.optimizer_protocol.as_ref().ok_or_else(||bad("native optimizer descriptor missing"))?;
        let mut expected=OptimizerProtocol::new(&l.model.config,arm==1,digest(&s.runtime)?,s.parent_step)?;expected.local_step=512;
        if *protocol!=expected||st.step!=s.parent_step+512||s.parent_step!=14336||st.sampler_state!=512
            ||l.tokenizer.semantic_id()!=s.tokenizer||l.model.config.id()?!=parent.model.config.id()?
            ||l.manifest.framing()?!=neural::Framing::QuestionEvidence||binary::record!(st.config)!=binary::record!(s.config)
            ||st.parent_checkpoint_hash.as_deref()!=Some(s.parent_hash.as_str())
            ||st.corpus_hash!=corpus.manifest.train.sha256||st.validation_hash!=corpus.manifest.validation.sha256
            ||st.resume_binding!=Some(super::bind(&s,arm,st,&l.tokenizer)?) {
            return Err(bad("BLOCKED_INPUT_INTEGRITY: endpoint lineage/config/state mismatch"));
        }
        for v in l.model.vars.values(){if v.flatten_all()?.to_vec1::<f32>()?.iter().any(|v|!v.is_finite()){return Err(bad("nonfinite original weight"));}}
        let mut updates=0;
        for i in 0..h.len(){let path=s.root.join(ARMS[arm]).join(format!("updates-{i:03}.r3rows"));if !path.exists(){continue;}bind(&mut refs,&path)?;
            for r in binary::read_value_records(&path)? {updates+=1;
                if updates>512||r["local"]!=updates||r["optimizer_local"]!=updates||r["model_step"]!=s.parent_step+updates
                    ||r["rows"]!=binary::record!(s.tape[updates-1])||r["lr"]!=s.config.lr{return Err(bad("actual tape/clock mismatch"));}
            }
        }if updates!=512{return Err(bad("actual endpoint trace incomplete"));}
        for(name,es,ms)in panels(&s,512,false)? {
            let dir=s.root.join(ARMS[arm]);let label=format!("eval-512-{name}");let path=dir.join(format!("{label}.r3rows"));bind(&mut refs,&path)?;
            let raw=binary::read_value_records(&path)?;
            let n=if arm==1&&name=="renamed"{10}else{64};
            if raw.len()!=n+1||raw[0]["policy"]!=digest(&s)?||raw[0]["model"]!=l.model.weights_content_id()?
                ||raw[0]["local"]!=512||raw[0]["absolute"]!=st.step||raw[0]["cases"]!=digest(&es)?
                ||raw[0]["metadata"]!=digest(&ms)?||raw[0]["tokenizer"]!=s.tokenizer||raw[0]["runtime"]!=binary::record!(s.runtime) {
                return Err(bad("frozen endpoint panel identity/count"));
            }
            for(i,(e,r))in es.iter().zip(&raw[1..]).enumerate(){check_row(e,r,&l)?;call_attempt(&dir,&label,"generation",&raw[0],e,i,Some(r))?;
                let path=dir.join(r["attempt"].as_str().ok_or_else(||bad("attempt absent"))?);bind(&mut refs,&path)?;
                bind(&mut refs,&path.with_file_name(path.file_name().unwrap().to_string_lossy().replace("-prepared","-resolved")))?;
            }
            if n<es.len(){call_attempt(&dir,&label,"generation",&raw[0],&es[n],n,None)?;}
            if n==64 {let score=scored(&name,&es,&ms,&raw[1..],&l.tokenizer)?;let path=dir.join(format!("{label}-score.r3b"));bind(&mut refs,&path)?;
                let old:Value=read_confirmed(&path)?;if score.as_object().unwrap().iter().any(|(k,v)|old[k]!=*v){return Err(bad("original score mismatch"));}
            }else{error_row=raw.last().cloned();}
            panels_out.push(RawRef{arm,name,path,header:raw[0].clone(),count:n});
        }
    }
    original_stop(&h,&error_row.ok_or_else(||bad("original error row missing"))?)?;
    let cases=(0..2).flat_map(|v|(0..4).flat_map(move|b|(0..4).map(move|q|6144+v*768+b*4+q)))
        .chain((3072..3104).map(|v|corpus.train.len()+v)).collect::<Vec<_>>();
    std::fs::create_dir(root)?;let root=root.canonicalize()?;
    let d=Diagnostic{contract:ID.into(),root:root.clone(),original:s,endpoints:end.arms.clone(),refs,panels:panels_out,
        source:source()?,runtime:RuntimeProfile::capture(Backend::Metal0,&Backend::Metal0.open()?)?,teacher_cases:cases,
        generation_cap:438,teacher_cap:216,active_cap:1800.,segment_cap:900.,bytes_cap:256*1024*1024,predecessor:None};
    if d.runtime.patch!=d.original.runtime.patch||d.runtime.patch_source!=d.original.runtime.patch_source||d.runtime.lock!=d.original.runtime.lock
        ||d.runtime.actual_device!=d.original.runtime.actual_device{return Err(bad("diagnostic backend differs"));}
    publish_confirmed(&root.join("diagnostic-plan.r3b"),&d)?;
    println!("POSTHOC_PREPARED policy={} old_RETURNED586 missing54 train256 teacher192 replay_cap128 teacher_check_cap24 optimizer0 backward0 original_resume=false",digest(&d)?);Ok(())
}
// Read-only exception for three completed lanes, never an exception to segments().
fn verified_d2(old:&Diagnostic)->Result<String>{
    if old.contract!=ID||old.predecessor.is_some(){return Err(bad("specific predecessor scope required"));}
    let mut totals=vec![];
    for (i,name) in ["missing","train-A","train-M","teacher-parent"].iter().enumerate(){
        let start_path=old.root.join(format!("observation-{i:03}-started.r3b"));
        let start:Value=read_confirmed(&start_path)?;
        let end:Value=read_confirmed(&old.root.join(format!("observation-{i:03}-finished.r3b")))?;
        let(_,physical,es,teacher,_)=lane(old,name)?;let b=binding(old,name,&physical,&es)?;
        if start["policy"]!=digest(old)?||start["lane"]!=*name||start["binding"]!=b||end["binding"]!=b||end["start"]!=file_hash(&start_path)? {return Err(bad("predecessor segment identity"));}
        if i<3 {
            if end["success"]!=true||end["resume"]!=false||end["control"]["observed_conditions"]!=binary::record!([])
                ||end["control"]["generation_calls"]!=es.len()||end["control"]["teacher_calls"]!=0
                ||read_confirmed::<Value>(&old.root.join(format!("{name}-finished.r3b")))?!=end {return Err(bad("D2 lane not complete"));}
            let rows=new_rows(old,name,&es,&b,teacher)?;let tok=scoring_tokenizer(&old.original)?;
            for(e,r)in es.iter().zip(rows){verify_output_result(&r,&tok)?;if r["id"]!=e.id||r["expected"]!=e.answer||r["request_digest"]!=digest(&e.request)?{return Err(bad("D2 case binding"));}}
        } else {
            let rows=binary::read_value_records(&old.root.join("teacher-parent-teachers.r3rows"))?;
            if rows.len()!=2||rows[0]!=b{return Err(bad("predecessor teacher must have exactly one failed attempt"));}
            call_attempt(&old.root,name,"teacher",&b,&es[0],0,Some(&rows[1]))?;
            if end["success"]!=false||end["resume"]!=false||end["control"]["observed_conditions"]!=binary::record!(["INTEGRITY_FAIL"])
                ||end["control"]["generation_calls"]!=0||end["control"]["teacher_calls"]!=1
                ||rows[1]["ordinal"]!=0||rows[1]["id"]!=es[0].id||rows[1]["case"]!=digest(&es[0])?
                ||rows[1]["teacher"]!=binary::record!({"error":"invalid input: native input shape/dtype/device/context/cache identity"})
                ||end["error"]!=rows[1]["teacher"]["error"] {return Err(bad("not the verified device-input predecessor failure"));}
            for absent in ["teacher-parent-finished.r3b","observation-004-started.r3b","composite.r3b"] {
                if old.root.join(absent).exists()||pending_path(&old.root.join(absent)).exists(){return Err(bad("unexpected predecessor continuation/completion"));}
            }
            // The returned row is bound to the entered call's control and failed segment.
            let attempt=old.root.join(rows[1]["attempt"].as_str().ok_or_else(||bad("failed attempt absent"))?);
            let resolution:Value=read_confirmed(&attempt.with_file_name(attempt.file_name().unwrap().to_string_lossy().replace("-prepared","-resolved")))?;
            if resolution["control"]["teacher_calls"]!=1||resolution["calls"]!=1||resolution["control"]["generation_calls"]!=0
                ||resolution["control"]["observed_conditions"]!=binary::record!([]){return Err(bad("predecessor entry usage/mixed failure"));}
            totals.push(end);let(_,g,t)=usage(&totals)?;if (g,t)!=(310,1){return Err(bad("predecessor usage"));}
            return digest(&rows[1]);
        }
        totals.push(end);
    }Err(bad("missing predecessor failure"))
}
fn predecessor(d:&Diagnostic)->Result<Diagnostic>{
    let p=d.predecessor.as_ref().ok_or_else(||bad("successor required"))?;
    let old=load(&p.root,false)?;
    let mut runtime=old.runtime.clone();runtime.binary=d.runtime.binary.clone();
    if digest(&old)?!=p.policy||digest(&old.original)?!=digest(&d.original)?||digest(&old.endpoints)?!=digest(&d.endpoints)?
        ||old.teacher_cases!=d.teacher_cases||digest(&old.panels)?!=digest(&d.panels)?
        ||verified_d2(&old)?!=p.failed_row||file_hash(&p.executor)?!=old.runtime.binary
        ||runtime!=d.runtime {
        return Err(bad("successor/predecessor lineage mismatch"));
    }Ok(old)
}
fn successor(root:&Path,executor:&Path,audit:&Path,output:&Path)->Result<()> {
    let old=load(root,false)?;let failed_row=verified_d2(&old)?;
    if file_hash(executor)?!=old.runtime.binary{return Err(bad("predecessor executable mismatch"));}
    // Source/native hashes and the independent provenance audit are retained as evidence,
    // not accepted merely because a report contains a PASS word.
    let mut refs=old.refs.clone();bind(&mut refs,audit)?;bind(&mut refs,executor)?;
    for e in std::fs::read_dir(&old.root)? {let e=e?;if e.file_type()?.is_file(){bind(&mut refs,&e.path())?;}}
    let mut manifests=BTreeMap::new();
    for name in ["teacher-parent","teacher-A","teacher-M","check-parent","check-A","check-M","replay-A","replay-M"] {
        let(_,model,es,teacher,expected)=lane(&old,name)?;
        manifests.insert(name.into(),binary::record!({"native":model,"cases":digest(&es)?,"ids":es.iter().map(|e|&e.id).collect::<Vec<_>>(),"teacher":teacher,"reference_rows":expected.as_ref().map(digest).transpose()?}));
    }
    let runtime=RuntimeProfile::capture(Backend::Metal0,&Backend::Metal0.open()?)?;
    std::fs::create_dir(output)?;
    let mut d=old.clone();d.contract=CLOSURE.into();d.root=output.canonicalize()?;d.source=source()?;d.runtime=runtime;d.refs=refs;d.generation_cap=128;
    d.predecessor=Some(Predecessor{root:old.root.clone(),policy:digest(&old)?,failed_row,executor:executor.canonicalize()?,manifests});
    predecessor(&d)?;
    publish_confirmed(&d.root.join("diagnostic-plan.r3b"),&d)?;
    println!("SUCCESSOR_PREPARED policy={} reused_D2=896 historical_generation310 failed_teacher1 new_generation0 teacher0 optimizer0 backward0",digest(&d)?);Ok(())
}
fn load(root:&Path,execute:bool)->Result<Diagnostic> {
    let d:Diagnostic=read_confirmed(&root.join("diagnostic-plan.r3b"))?;
    if (d.predecessor.is_some()&&d.contract!=CLOSURE)||(d.predecessor.is_none()&&d.contract!=ID)
        ||d.root!=root.canonicalize()?||d.generation_cap!=if d.predecessor.is_some(){128}else{438}||d.teacher_cap!=216||d.active_cap!=1800.||d.segment_cap!=900.||d.bytes_cap!=256*1024*1024 {
        return Err(bad("diagnosis policy/root/budget mismatch"));}
    check_refs(&d)?;inputs(&d.original)?;
    if d.predecessor.is_some(){predecessor(&d)?;}
    if execute {
        if d.source!=source()?{return Err(bad("diagnostic frozen source mismatch"));}d.runtime.verify(&Backend::Metal0.open()?)?;
        let a:Value=read_confirmed(&root.join("review-a.r3b"))?;let path=Path::new(a["report"].as_str().ok_or_else(||bad("independent A absent"))?);
        if a["policy"]!=digest(&d)?||a["report_hash"]!=file_hash(path)?||a["accepted"]!=true{return Err(bad("independent A mismatch"));}
    }Ok(d)
}
fn admit(root:&Path,path:&Path)->Result<()> {
    let d=load(root,false)?;let r:Value=read(path)?;
    if d.predecessor.is_some(){return admit_phase(&d,path,"A-delta");}
    if r["contract"]!=ID||r["source"]!=d.source||r["policy"]!=digest(&d)?||r["verdict"]!="PASS"
        ||r["tests_passed"].as_u64().is_none_or(|n|n<6)||r["optimizer_calls"]!=0||r["backward_calls"]!=0
        ||r["generation_calls"]!=0||r["teacher_calls"]!=0{return Err(bad("A-delta incomplete"));}
    publish_confirmed(&root.join("review-a.r3b"),&binary::record!({"accepted":true,"policy":digest(&d)?,"report":path.canonicalize()?,"report_hash":file_hash(path)?}))
}
fn admit_phase(d:&Diagnostic,path:&Path,phase:&str)->Result<()> {
    if d.predecessor.is_none(){return Err(bad("successor admission required"));}
    let r:Value=read_confirmed(path)?;
    let caller=phase=="A-caller";
    if r["contract"]!=CLOSURE||r["phase"]!=phase||r["policy"]!=digest(d)?||r["source"]!=d.source
        ||r["runtime"]!=binary::record!(d.runtime)||r["verdict"]!="PASS"||r["optimizer_calls"]!=0||r["backward_calls"]!=0
        ||r["generation_calls"]!=0||r["teacher_calls"]!=if caller{3}else{0}
        ||r["evidence"].as_object().is_none_or(|v|v.is_empty()) {return Err(bad("independent successor admission binding"));}
    for (path,hash) in r["evidence"].as_object().unwrap(){if file_hash(Path::new(path))?!=hash.as_str().ok_or_else(||bad("evidence hash"))?{return Err(bad("review evidence changed"));}}
    if caller&&r["smoke"]!=digest(&smoke_readback(d)?)?{return Err(bad("A-caller smoke mismatch"));}
    publish_confirmed(&d.root.join(if caller{"review-caller.r3b"}else{"review-a.r3b"}),&binary::record!({"accepted":true,"policy":digest(d)?,"report":path.canonicalize()?,"report_hash":file_hash(path)?}))
}
fn caller_accepted(d:&Diagnostic)->Result<()> {
    let a:Value=read_confirmed(&d.root.join("review-caller.r3b"))?;
    let path=Path::new(a["report"].as_str().ok_or_else(||bad("A-caller required"))?);
    let r:Value=read_confirmed(path)?;
    if a["accepted"]!=true||a["policy"]!=digest(d)?||a["report_hash"]!=file_hash(path)?||r["phase"]!="A-caller"||r["policy"]!=digest(d)?||r["source"]!=d.source||r["verdict"]!="PASS" {return Err(bad("A-caller evidence mismatch"));}Ok(())
}
fn smoke_readback(d:&Diagnostic)->Result<Value> {
    if d.predecessor.is_none(){return Err(bad("successor smoke required"));}
    let h=segments(d)?;let(_,g,t)=usage(&h)?;if g!=0||t!=3{return Err(bad("first-row caller usage must be three"));}
    let tok=scoring_tokenizer(&d.original)?;let mut out=vec![];
    for model in ["parent","A","M"] {
        let name=format!("teacher-{model}");let(_,native,es,_,_)=lane(d,&name)?;let b=binding(d,&name,&native,&es)?;
        let rows=prefix_rows(d,&name,&es,&b,true)?;
        let done:Value=read_confirmed(&d.root.join(format!("{name}-prefix.r3b")))?;
        if rows.len()!=1||done["binding"]!=b||done["success"]!=true||done["coverage"]!=1||done["planned"]!=64||done["work_remaining"]!=true{return Err(bad("smoke is not exactly first1/full64"));}
        validate_forward(d,&es[..1],&rows,&tok)?;
        out.push(binary::record!({"model":model,"binding":b,"row":digest(&rows[0])?,"prefix_final":file_hash(&d.root.join(format!("{name}-prefix.r3b")))?,"score":teacher_scores(&es[..1],&rows,&tok)?}));
    }Ok(binary::record!({"policy":digest(d)?,"teacher_completed":3,"teacher_new":0,"generation_new":0,"models":out}))
}
// Same immutable publisher and RunControl as the learning harness, but a new
// budget and observations only. No old-control exception and no training state.
fn segments(d:&Diagnostic)->Result<Vec<Value>> {
    let mut out=vec![];
    for i in 0..128 {let p=d.root.join(format!("observation-{i:03}-started.r3b"));if !p.exists()&&!pending_path(&p).exists(){break;}
        let start:Value=read_confirmed(&p)?;let end:Value=read_confirmed(&d.root.join(format!("observation-{i:03}-finished.r3b")))?;
        if start["policy"]!=digest(d)?||end["start"]!=file_hash(&p)?||end["control"]["observed_conditions"].as_array().is_none_or(|v|v.iter().any(|x|x!="TIME_BUDGET"))
            ||end["success"]!=true&&end["resume"]!=true{return Err(bad("diagnostic UNKNOWN/cancel/sticky execution failure"));}
        out.push(end);
    }Ok(out)
}
fn usage(records:&[Value])->Result<(f64,usize,usize)> {
    let(mut time,mut g,mut t)=(0.,0,0);
    for r in records {let c=&r["control"];time+=c["elapsed_seconds"].as_f64().filter(|v|v.is_finite()&&*v>=0.).ok_or_else(||bad("time UNKNOWN"))?;
        g+=c["generation_calls"].as_u64().ok_or_else(||bad("calls UNKNOWN"))? as usize;t+=c["teacher_calls"].as_u64().ok_or_else(||bad("teacher UNKNOWN"))? as usize;}
    Ok((time,g,t))
}
fn original_rows(d:&Diagnostic,r:&RawRef)->Result<Vec<Value>> {
    if file_hash(&r.path)?!=d.refs[&r.path]{return Err(bad("original raw changed"));}
    let mut v=binary::read_value_records(&r.path)?;if v.len()!=r.count+1||v[0]!=r.header{return Err(bad("original row binding"));}Ok(v.drain(1..).collect())
}
fn new_rows(d:&Diagnostic,label:&str,es:&[Episode],b:&Value,teacher:bool)->Result<Vec<Value>> {
    if d.predecessor.is_some()&&matches!(label,"missing"|"train-A"|"train-M"){
        let old=predecessor(d)?;let(_,native,expected,t,_)=lane(&old,label)?;
        if digest(&es)?!=digest(&expected)?||t!=teacher{return Err(bad("imported case manifest changed"));}
        return new_rows(&old,label,es,&binding(&old,label,&native,es)?,teacher);
    }
    let rows=prefix_rows(d,label,es,b,teacher)?;
    if rows.len()!=es.len(){return Err(bad("new raw incomplete"));}Ok(rows)
}
fn prefix_rows(d:&Diagnostic,label:&str,es:&[Episode],b:&Value,teacher:bool)->Result<Vec<Value>> {
    let path=d.root.join(format!("{label}{}.r3rows",if teacher{"-teachers"}else{""}));
    let v=binary::read_value_records(&path)?;if v.is_empty()||v.len()>es.len()+1||v[0]!=*b{return Err(bad("new raw incomplete/binding"));}
    for(i,(e,r))in es.iter().zip(&v[1..]).enumerate(){call_attempt(&d.root,label,if teacher{"teacher"}else{"generation"},b,e,i,Some(r))?;
        if r["id"]!=e.id||teacher&&(r["ordinal"]!=i||r["case"]!=digest(e)?){return Err(bad("raw case/ordinal mismatch"));}}
    if v.len()<=es.len(){call_attempt(&d.root,label,if teacher{"teacher"}else{"generation"},b,&es[v.len()-1],v.len()-1,None)?;}
    Ok(v[1..].to_vec())
}
fn binding(d:&Diagnostic,label:&str,model:&str,es:&[Episode])->Result<Value>{Ok(binary::record!({"policy":digest(d)?,"source":d.source,"runtime":d.runtime,"lane":label,"native":model,"cases":digest(&es)?,"planned":es.len(),"call_protocol":1}))}
fn composite(d:&Diagnostic)->Result<Vec<(usize,Panel,Vec<Value>,Vec<Value>)>> {
    if d.predecessor.is_some(){return composite(&predecessor(d)?);}
    let mut out=vec![];
    for r in &d.panels {let p=panels(&d.original,512,false)?.into_iter().find(|p|p.0==r.name).ok_or_else(||bad("unknown frozen panel"))?;
        let mut rows=original_rows(d,r)?;let mut refs=rows.iter().enumerate().map(|(i,row)|Ok(binary::record!({"path":r.path,"file_hash":d.refs[&r.path],"ordinal":i,"row_hash":digest(row)?,"producer":d.original.source,"historical":true}))).collect::<Result<Vec<_>>>()?;
        if rows.len()<p.1.len(){let es=&p.1[rows.len()..];let b=binding(d,"missing",&d.endpoints[1].physical,es)?;
            let new=new_rows(d,"missing",es,&b,false)?;let path=d.root.join("missing.r3rows");let h=file_hash(&path)?;
            for(i,row)in new.iter().enumerate(){refs.push(binary::record!({"path":path,"file_hash":h,"ordinal":i,"row_hash":digest(row)?,"producer":d.source,"historical":false}));}rows.extend(new);
        }
        out.push((r.arm,p,rows,refs));
    }Ok(out)
}
fn teacher_selection(d:&Diagnostic,check:bool)->Result<Vec<Episode>>{
    let(c,_,_)=inputs(&d.original)?;
    let ids=if check{vec![0,1,16,17,32,33,60,61]}else{(0..64).collect()};
    if d.teacher_cases.len()!=64{return Err(bad("teacher manifest count"));}
    ids.into_iter().map(|i|{let n=d.teacher_cases[i];if n<c.train.len(){Ok(c.train[n].clone())}else{c.validation.get(n-c.train.len()).cloned().ok_or_else(||bad("teacher index"))}}).collect()
}
fn replay_selection(d:&Diagnostic,arm:usize)->Result<(Vec<Episode>,Vec<Value>)>{
    let panels=composite(d)?;let mut cases=vec![];let mut rows=vec![];let mut normal=BTreeSet::new();
    for(_,p,rs,_)in panels.into_iter().filter(|(a,_,_,_)|*a==arm){let count=if p.0=="value"{8}else{6};
        for(i,(e,r))in p.1.into_iter().zip(rs).enumerate(){if i<count{normal.insert(cases.len());}cases.push(e);rows.push(r);}
    }
    let selected=normal.iter().copied().chain(failure_pairs(&rows,&normal,32)).collect::<Vec<_>>();
    Ok((selected.iter().map(|&i|cases[i].clone()).collect(),selected.iter().map(|&i|rows[i].clone()).collect()))
}
fn failure_pairs(rows:&[Value],normal:&BTreeSet<usize>,cap:usize)->Vec<usize>{
    let mut candidates=(0..rows.len()).filter(|&i|rows[i]["exact_match"]!=true).collect::<Vec<_>>();
    candidates.sort_by_key(|&i|(rows[i]["error_class"]!="strict_utf8",rows[i]["id"].as_str().unwrap_or("")));
    let mut selected=vec![];let mut seen=normal.clone();
    for i in candidates{for index in [i,i^1]{if selected.len()<cap&&index<rows.len()&&seen.insert(index){selected.push(index);}}}
    selected
}
pub(super) fn role_tokens(answer:&str,gold:&[u32],tok:&ByteBpe)->Result<Vec<String>> {
    if answer.len()==8&&answer.bytes().all(|b|b.is_ascii_digit()) {
        if gold.len()!=9||gold.last()!=Some(&EOS)||tok.decode_bytes(&gold[..8])?!=answer.as_bytes()
            ||gold[..8].iter().any(|&t|tok.decode_bytes(&[t]).map_or(true,|v|v.len()!=1)){return Err(bad("ID_ONLY byte alignment"));}
        return Ok((0..9).map(|i|if i==8{"EOS"}else{"ID"}.into()).collect());
    }
    let (value,rest)=answer.split_once("입니다. [event:").ok_or_else(||bad("AMBIGUOUS_ALIGNMENT: answer grammar"))?;
    let id=rest.strip_suffix(']').ok_or_else(||bad("AMBIGUOUS_ALIGNMENT: closing grammar"))?;
    if value.is_empty()||id.len()!=8||!id.bytes().all(|b|b.is_ascii_digit())||gold.last()!=Some(&EOS)
        ||tok.decode_bytes(&gold[..gold.len()-1])?!=answer.as_bytes(){return Err(bad("AMBIGUOUS_ALIGNMENT: target bytes"));}
    let v=value.len();let start=v+"입니다. [event:".len();let end=start+id.len();
    let at=|p|if p<v{"VALUE"}else if p<start{"FORMAT"}else if p<end{"ID"}else{"CLOSE"};
    let mut offset=0;let mut out=vec![];
    for &token in gold {if token==EOS{out.push("EOS".into());continue;}
        let bytes=tok.decode_bytes(&[token])?;if bytes.is_empty(){return Err(bad("AMBIGUOUS_ALIGNMENT: empty target token"));}
        let a=at(offset);let b=at(offset+bytes.len()-1);out.push(if a==b{a}else{"MIXED"}.into());offset+=bytes.len();
    }if offset!=answer.len(){return Err(bad("AMBIGUOUS_ALIGNMENT: byte length"));}Ok(out)
}
pub(super) fn teacher_scores(es:&[Episode],rows:&[Value],tok:&ByteBpe)->Result<Value> {
    if es.len()!=rows.len(){return Err(bad("teacher incomplete"));}
    let mut totals=BTreeMap::<String,(usize,usize,f64)>::new();let mut details=vec![];
    let mut spans=BTreeMap::<String,[usize;3]>::new();
    for(e,r)in es.iter().zip(rows){let t=&r["teacher"];let v=&t["target_token_observation"];
        let gold:Vec<u32>=binary::from_value(v["gold"].clone())?;let predicted:Vec<u32>=binary::from_value(v["argmax"].clone())?;let nll:Vec<f64>=binary::from_value(v["nll"].clone())?;
        let mut expected=tok.encode(e.answer.as_bytes())?;expected.push(EOS);
        if gold!=expected||gold.len()!=predicted.len()||gold.len()!=nll.len()||nll.iter().any(|n|!n.is_finite()||*n<0.)
            ||t["training_prompt_matches_generation"]!=true||t["answer_tokenizer_roundtrip"]!=true||r["case"]!=digest(e)?{return Err(bad("teacher shift/target/scalar mismatch"));}
        let roles=role_tokens(&e.answer,&gold,tok)?;
        for(i,role)in roles.iter().enumerate(){let x=totals.entry(role.clone()).or_default();x.0+=1;x.1+=usize::from(gold[i]==predicted[i]);x.2+=nll[i];}
        let boundaries=if let Some((v,_))=e.answer.split_once("입니다. [event:"){vec![("VALUE",v.len()),("FORMAT","입니다. [event:".len()),("ID",8)]}else{vec![("ID",8)]};
        for (role,expected_bytes) in boundaries {let x=spans.entry(role.into()).or_default();
            let mut covered=0;for(i,r)in roles.iter().enumerate(){if r==role{covered+=tok.decode_bytes(&[gold[i]])?.len();}}
            if covered!=expected_bytes{x[2]+=1;}else{x[0]+=1;x[1]+=usize::from(roles.iter().enumerate().filter(|(_,r)|r.as_str()==role).all(|(i,_)|gold[i]==predicted[i]));}}
        details.push(binary::record!({"id":e.id,"roles":roles,"first_argmax_difference":gold.iter().zip(&predicted).position(|(a,b)|a!=b),"target_tokens":gold.len()}));
    }
    let means=totals.iter().map(|(k,(n,_,sum))|(k.clone(),sum/(*n as f64))).collect::<BTreeMap<_,_>>();
    Ok(binary::record!({"condition":"gold prefix; not free generation","examples":es.len(),"forward_invocations":rows.len(),"roles":totals,"mean_nll":means,"span_covered_correct_excluded":spans,"span_exclusion":"MIXED crossing this span boundary leaves incomplete byte coverage","cases":details}))
}
fn validate_forward(d:&Diagnostic,es:&[Episode],rows:&[Value],tok:&ByteBpe)->Result<()> {
    validate_teacher_forward(&d.runtime,d.original.config.seq_len,es,rows,tok)
}
pub(super) fn validate_teacher_forward(runtime:&RuntimeProfile,seq_len:usize,es:&[Episode],rows:&[Value],tok:&ByteBpe)->Result<()> {
    teacher_scores(es,rows,tok)?;
    let samples=samples_with_framing(es,tok,seq_len,neural::Framing::QuestionEvidence)?;
    for(s,r)in samples.iter().zip(rows){let n=s.response_start;let f=&r["teacher"]["native_forward"];
        if f["model_device"]!=runtime.actual_device||f["input_device"]!=runtime.actual_device||f["logits_device"]!=runtime.actual_device
            ||f["input_dtype"]!="U32"||f["logits_dtype"]!="F32"||f["logits_finite"]!=true
            ||f["input_shape"]!=binary::record!([1,s.tokens.len()-1])||f["prompt_tokens"]!=n
            ||f["prompt_digest"]!=digest(&&s.tokens[..n])?||f["input_tokens_digest"]!=digest(&&s.tokens[..s.tokens.len()-1])?
            ||f["logits_shape"][0]!=s.tokens.len()-n {return Err(bad("actual teacher forward device/shift/finite evidence"));}
    }Ok(())
}
fn lane(d:&Diagnostic,name:&str)->Result<(PathBuf,String,Vec<Episode>,bool,Option<Vec<Value>>)>{
    let arm=usize::from(name.ends_with('M'));let p=&d.endpoints[arm];
    if name=="missing" {let r=d.panels.iter().find(|r|r.arm==1&&r.name=="renamed").ok_or_else(||bad("missing panel"))?;
        let p=panels(&d.original,512,false)?.pop().unwrap();return Ok((d.endpoints[1].native.clone(),d.endpoints[1].physical.clone(),p.1[r.count..].to_vec(),false,None));}
    if name.starts_with("train-"){return Ok((p.native.clone(),p.physical.clone(),panels(&d.original,512,true)?.remove(0).1,false,None));}
    if name.starts_with("teacher-")||name.starts_with("check-"){
        let parent=name.ends_with("parent");return Ok((if parent{d.original.parent.clone()}else{p.native.clone()},if parent{d.original.parent_hash.clone()}else{p.physical.clone()},teacher_selection(d,name.starts_with("check-"))?,true,None));}
    if name.starts_with("replay-"){let(es,rs)=replay_selection(d,arm)?;return Ok((p.native.clone(),p.physical.clone(),es,false,Some(rs)));}
    Err(bad("unknown diagnostic lane"))
}
fn parity(a:&[Value],b:&[Value])->Result<()> {
    if a.len()!=b.len(){return Err(bad("POSTHOC_PARITY_FAIL count"));}
    for(x,y)in a.iter().zip(b){for key in ["raw_tokens","actual","finish_reason","decode_error","error_class","generation_completed"] {
        if x[key]!=y[key]{return Err(bad(&format!("POSTHOC_PARITY_FAIL {} {key}",x["id"])));}
    }}Ok(())
}
fn observe(d:&Diagnostic,name:&str)->Result<()> {observe_until(d,name,false)}
fn observe_until(d:&Diagnostic,name:&str,smoke:bool)->Result<()> {
    let history=segments(d)?;let (time,g,t)=usage(&history)?;
    if g>d.generation_cap||t>d.teacher_cap{return Err(bad("new diagnostic budget exceeded"));}
    let (path,model,es,teacher,expected)=lane(d,name)?;let b=binding(d,name,&model,&es)?;
    if let Some(p)=&d.predecessor {
        if matches!(name,"missing"|"train-A"|"train-M"){return Err(bad("D2 is read-only; regeneration forbidden"));}
        if p.manifests.get(name)!=Some(&binary::record!({"native":model,"cases":digest(&es)?,"ids":es.iter().map(|e|&e.id).collect::<Vec<_>>(),"teacher":teacher,"reference_rows":expected.as_ref().map(digest).transpose()?})){return Err(bad("frozen successor manifest changed"));}
        if smoke {if !name.starts_with("teacher-"){return Err(bad("smoke requires main teacher"));}}else{caller_accepted(d)?;}
    }else if smoke{return Err(bad("prefix collection is successor-only"));}
    let final_path=d.root.join(format!("{name}-{}.r3b",if smoke{"prefix"}else{"finished"}));
    if final_path.exists(){let done:Value=read_confirmed(&final_path)?;
        let rows=if smoke{prefix_rows(d,name,&es,&b,true)?}else{new_rows(d,name,&es,&b,teacher)?};if done["binding"]!=b||done["success"]!=true||smoke&&rows.len()!=1{return Err(bad("completed observation binding"));}
        println!("REUSED_COMPLETE {name} new_calls0");return Ok(());}
    if owned_bytes(&d.root)?+8*1024*1024>d.bytes_cap{return Err(bad("new evidence byte budget"));}
    let cancel=std::sync::Arc::new(AtomicBool::new(false));let flag=cancel.clone();ctrlc::set_handler(move||flag.store(true,Ordering::Relaxed)).map_err(|e|bad(&e.to_string()))?;
    let mut ctl=RunControl::new(cancel,std::time::Duration::from_secs_f64((d.active_cap-time).max(0.).min(d.segment_cap)),12*1024*1024)?;
    ctl.set_call_limits(d.generation_cap-g,d.teacher_cap-t);let index=history.len();
    let start=d.root.join(format!("observation-{index:03}-started.r3b"));
    publish_confirmed(&start,&binary::record!({"policy":digest(d)?,"lane":name,"binding":b,"generation_used":g,"teacher_used":t}))?;
    let result=(||->Result<Value>{let l=checkpoint::load(&path,Backend::Metal0.open()?,false)?;
        if file_hash(&path)?!=model||l.tokenizer.semantic_id()!=d.original.tokenizer{return Err(bad("endpoint changed before call"));}
        let rows=if teacher{teacher_prefix_until(&d.root,name,&b,&l,&es,if smoke{1}else{es.len()},&mut ctl)?}else{generated(&l,&d.root,name,&es,&b,&mut ctl)?};
        let score=if teacher{
            if d.predecessor.is_some(){validate_forward(d,&es[..rows.len()],&rows,&l.tokenizer)?;}
            let score=teacher_scores(&es[..rows.len()],&rows,&l.tokenizer)?;
            if name.starts_with("check-"){
                let original=name.replacen("check-","teacher-",1);let(_,native,all,_,_)=lane(d,&original)?;
                let old=new_rows(d,&original,&all,&binding(d,&original,&native,&all)?,true)?;
                for(e,r)in es.iter().zip(&rows){let i=all.iter().position(|x|x.id==e.id).ok_or_else(||bad("teacher reference absent"))?;
                    let x=&r["teacher"]["target_token_observation"];let y=&old[i]["teacher"]["target_token_observation"];
                    if x["gold"]!=y["gold"]||x["argmax"]!=y["argmax"]{return Err(bad("teacher token parity"));}
                    let a:Vec<f64>=binary::from_value(x["nll"].clone())?;let b:Vec<f64>=binary::from_value(y["nll"].clone())?;
                    if a.len()!=b.len()||a.iter().zip(&b).any(|(a,b)|(a-b).abs()>1e-5){return Err(bad("teacher scalar parity"));}
                }
            }score
        }else{
            for(e,r)in es.iter().zip(&rows){check_row(e,r,&l)?;}if let Some(old)=expected{parity(&rows,&old)?;}
            binary::record!({"completed":rows.len(),"raw_tokens":rows.iter().map(|r|r["raw_tokens"].as_array().map_or(0,Vec::len)).sum::<usize>()})};
        l.model.device.synchronize()?;ctl.seal_completed_no_call()?;Ok(score)
    })();
    if let Err(e)=&result{ctl.classify_error(e);}
    let record=binary::record!({"start":file_hash(&start)?,"binding":b,"success":result.is_ok(),"coverage":if result.is_ok(){Some(if smoke{1}else{es.len()})}else{None},"planned":es.len(),"work_remaining":smoke,"resume":result.is_err()&&ctl.receipt()["observed_conditions"]==binary::record!(["TIME_BUDGET"]),"control":ctl.receipt(),"score":result.as_ref().ok(),"error":result.as_ref().err().map(ToString::to_string)});
    // Publish the lane final before declaring its segment successful. A failed
    // final publication leaves a started segment without success (fail closed).
    if result.is_ok(){publish_confirmed(&final_path,&record)?;}
    publish_confirmed(&d.root.join(format!("observation-{index:03}-finished.r3b")),&record)?;
    println!("POSTHOC {name} success={} new_generation={} new_teacher={} seconds={} optimizer0 backward0",result.is_ok(),record["control"]["generation_calls"],record["control"]["teacher_calls"],record["control"]["elapsed_seconds"]);
    result.map(|_|())
}
fn brief(score:&Value)->Value {binary::record!({"joint":score["joint"].as_object().map(|v|v.iter().filter(|(k,_)|["total","full","query_both","swap_both","all4","eos","errors"].contains(&k.as_str())).map(|(k,v)|(k.clone(),v.clone())).collect::<BTreeMap<_,_>>()),"value":score["value_correct"],"support":score["citation_support_correct"],"other_provided":score["other_provided_id"],"outside":score["valid_outside_id"],"parse":score["parse_failure_rows"],"output_errors":score["output_errors"]})}
fn output_errors(rows:&[Value])->Value {binary::record!({"strict_utf8":rows.iter().filter(|r|r["error_class"]=="strict_utf8").count(),"length":rows.iter().filter(|r|r["finish_reason"]=="length").count(),"runtime":rows.iter().filter(|r|!r["generation_error"].is_null()).count(),"overlapping_counts":true})}
fn assemble(d:&Diagnostic)->Result<Value> {
    let tok=scoring_tokenizer(&d.original)?;let mut outputs=vec![];
    for(arm,p,rows,refs)in composite(d)?{let mut score=scored(&p.0,&p.1,&p.2,&rows,&tok)?;score["output_errors"]=output_errors(&rows);
        println!("POSTHOC arm={} panel={} score={}",ARMS[arm],p.0,brief(&score));
        outputs.push(binary::record!({"arm":ARMS[arm],"panel":p.0,"score":score,"refs":refs}));}
    let(c,_,_)=inputs(&d.original)?;let seen=d.original.tape[..512].iter().flatten().fold(BTreeMap::<usize,usize>::new(),|mut m,&i|{*m.entry(i).or_default()+=1;m});
    let p=panels(&d.original,512,true)?.remove(0);let exposure=p.1.iter().map(|e|{let i=c.train.iter().position(|x|x.id==e.id).unwrap();seen.get(&i).copied().unwrap_or(0)}).collect::<Vec<_>>();
    for arm in 0..2{let name=format!("train-{}",ARMS[arm]);let b=binding(d,&name,&d.endpoints[arm].physical,&p.1)?;
        let rows=new_rows(d,&name,&p.1,&b,false)?;let mut score=scored(&p.0,&p.1,&p.2,&rows,&tok)?;score["output_errors"]=output_errors(&rows);
        let mut groups=BTreeMap::<String,[usize;4]>::new();let mut histogram=BTreeMap::<usize,usize>::new();
        for(i,r)in rows.iter().enumerate(){*histogram.entry(exposure[i]).or_default()+=1;let x=groups.entry(if exposure[i]>0{"seen"}else{"unseen"}.into()).or_default();x[0]+=1;x[1]+=usize::from(r["exact_match"]==true);
            let answer=&p.1[i].answer;let value=answer.split_once("입니다. ").unwrap().0;x[2]+=usize::from(r["actual"].as_str().and_then(|a|a.split_once("입니다. ")).is_some_and(|(v,_)|v==value));
            let gold=identifiable::binding::citation::individually_valid_ids(answer);x[3]+=usize::from(r["actual"].as_str().map(identifiable::binding::citation::individually_valid_ids).is_some_and(|ids|ids==gold));}
        println!("TRAIN {} score={} groups={groups:?} histogram={histogram:?}",ARMS[arm],brief(&score));outputs.push(binary::record!({"arm":ARMS[arm],"panel":"train128","score":score,"exposure":exposure,"groups_total_full_value_support":groups,"histogram":histogram}));}
    for model in ["parent","A","M"]{let name=format!("teacher-{model}");let(_,physical,es,_,_)=lane(d,&name)?;let b=binding(d,&name,&physical,&es)?;let rows=new_rows(d,&name,&es,&b,true)?;
        let score=teacher_scores(&es,&rows,&tok)?;let train=teacher_scores(&es[..32],&rows[..32],&tok)?;let dev=teacher_scores(&es[32..],&rows[32..],&tok)?;
        let mut distribution=BTreeMap::<String,usize>::new();
        for(i,e)in es.iter().enumerate(){let word=e.answer.split_once("입니다. ").unwrap().0;let exposure=if i<32{*seen.get(&d.teacher_cases[i]).unwrap_or(&0)}else{0};
            *distribution.entry(format!("{}/{word}/bytes{}/chars{}/seen{exposure}",if i<32{"train"}else{"dev"},word.len(),word.chars().count())).or_default()+=1;}
        let mut divergences=vec![];
        if model!="parent"{let arm=usize::from(model=="M");let train_name=format!("train-{model}");let all=panels(&d.original,512,true)?.remove(0).1;
            let train_rows=new_rows(d,&train_name,&all,&binding(d,&train_name,&d.endpoints[arm].physical,&all)?,false)?;
            let word=d.panels.iter().find(|r|r.arm==arm&&r.name=="word").unwrap();let dev_rows=original_rows(d,word)?;
            for(i,(e,r))in es.iter().zip(&rows).enumerate(){let free=if i<32{&train_rows[all.iter().position(|x|x.id==e.id).unwrap()]}else{&dev_rows[i-32]};
                let gold:Vec<u32>=binary::from_value(r["teacher"]["target_token_observation"]["gold"].clone())?;
                let raw:Vec<u32>=binary::from_value(free["raw_tokens"].clone())?;
                let at=gold.iter().zip(&raw).position(|(a,b)|a!=b).or_else(||(gold.len()!=raw.len()).then_some(gold.len().min(raw.len())));
                let roles=role_tokens(&e.answer,&gold,&tok)?;
                divergences.push(binary::record!({"id":e.id,"first_token_difference":at,"role":at.and_then(|i|roles.get(i)),"text_exact":free["exact_match"],"teacher_argmax_at_same_prefix":at.and_then(|i|r["teacher"]["target_token_observation"]["argmax"].as_array().and_then(|a|a.get(i))),"free_token":at.and_then(|i|raw.get(i))}));
            }
        }
        println!("TEACHER {model} train={} dev={}",train["roles"],dev["roles"]);outputs.push(binary::record!({"model":model,"panel":"teacher","score":score,"train":train,"dev":dev,"free_divergence":divergences,"sample_distribution":distribution}));}
    let h=segments(d)?;println!("DIAGNOSIS_READBACK originals586 composite640 usage={:?} bytes={} original_B=BLOCKED_AS_RECORDED MODEL_QUALITY_NOT_ACCEPTED Goal1=false",usage(&h)?,owned_bytes(&d.root)?);
    // stdout is a human report only. The execute commands own immutable finals;
    // this pure reader never creates missing evidence or changes eligibility.
    println!("COMPOSITE_CONTENT_DIGEST {}",digest(&outputs)?);
    Ok(binary::record!({"policy":digest(d)?,"classification":if d.predecessor.is_some(){"COMPOSITE_POSTHOC_SUCCESSOR"}else{"COMPOSITE_READBACK/POSTHOC"},"predecessor":d.predecessor,"original_study":"FAILED_UNCHANGED","original_B":"BLOCKED_AS_RECORDED","outputs":outputs,"usage":usage(&h)?,"segments":digest(&h)?,"Goal1":false}))
}
fn require_complete(d:&Diagnostic)->Result<()> {
    segments(d)?;
    if d.predecessor.is_some(){predecessor(d)?;caller_accepted(d)?;}
    for name in ["missing","train-A","train-M","teacher-parent","teacher-A","teacher-M","replay-A","replay-M","check-parent","check-A","check-M"]{
        if d.predecessor.is_some()&&matches!(name,"missing"|"train-A"|"train-M"){continue;}
        let(_,native,es,teacher,_)=lane(d,name)?;let b=binding(d,name,&native,&es)?;
        let r:Value=read_confirmed(&d.root.join(format!("{name}-finished.r3b")))?;
        if r["success"]!=true||r["binding"]!=b{return Err(bad("required observation final absent/failed"));}
        new_rows(d,name,&es,&b,teacher)?;
    }Ok(())
}
// Reuse the already accepted closure by its exact native identities. This is
// a read-only source link, not admission to the failed learning/diagnosis runs.
pub(super) fn event_parent(root:&Path)->Result<(Study,Progress,Value,BTreeMap<PathBuf,String>)>{
    let d:Diagnostic=read_confirmed(&root.join("diagnostic-plan.r3b"))?;
    let v:Value=read_confirmed(&root.join("composite.r3b"))?;
    if d.contract!=CLOSURE||d.root!=root.canonicalize()?||v["policy"]!=digest(&d)?
        ||v["classification"]!="COMPOSITE_POSTHOC_SUCCESSOR"||v["original_study"]!="FAILED_UNCHANGED"
        ||v["segments"]!=digest(&segments(&d)?)?||d.endpoints[0].local!=512{return Err(bad("completed teacher closure identity"));}
    let mut refs=BTreeMap::new();
    for p in [root.join("diagnostic-plan.r3b"),root.join("composite.r3b"),d.original.root.join("plan.r3b"),d.endpoints[0].native.clone()] {bind(&mut refs,&p)?;}
    if digest(&read_confirmed::<Study>(&d.original.root.join("plan.r3b"))?)?!=digest(&d.original)?
        ||file_hash(&d.endpoints[0].native)?!=d.endpoints[0].physical{return Err(bad("original A512 linkage"));}
    Ok((d.original,d.endpoints[0].clone(),v,refs))
}
pub fn run(a:Action)->Result<()>{match a {
    Action::Successor{predecessor,executor,audit,output}=>successor(&predecessor,&executor,&audit,&output),
    Action::Smoke{root,model}=>observe_until(&load(&root,true)?,&format!("teacher-{model}"),true),
    Action::ReadSmoke{root}=>{let d=load(&root,false)?;let v=smoke_readback(&d)?;println!("CALLER_READBACK digest={} {}",digest(&v)?,v);Ok(())},
    Action::AdmitCaller{root,review}=>admit_phase(&load(&root,false)?,&review,"A-caller"),
    Action::Prepare{original,audit,output}=>prepare(&original,&audit,&output),
    Action::Admit{root,review}=>admit(&root,&review),
    Action::Observe{root,lane}=>observe(&load(&root,true)?,&lane),
    Action::Report{root}=>{let d=load(&root,false)?;let value=assemble(&d)?;let path=root.join("composite.r3b");if path.exists()&&read_confirmed::<Value>(&path)?!=value{return Err(bad("composite mismatch"));}Ok(())},
    Action::Close{root}=>{let d=load(&root,false)?;require_complete(&d)?;let value=assemble(&d)?;let path=root.join("composite.r3b");if path.exists(){if read_confirmed::<Value>(&path)?!=value{return Err(bad("composite mismatch"));}Ok(())}else{publish_confirmed(&path,&value)}},
}}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    #[ignore="native metadata load and synthetic journals only; no forward"]
    fn prefix_empty_legacy_and_full_manifest_new_process()->Result<()> {
        if let Ok(root)=std::env::var("R3_CLOSURE_PREFIX_CHILD") {
            let d:Diagnostic=read(&PathBuf::from(root).join("fixture.r3b"))?;
            let(_,native,es,_,_)=lane(&d,"teacher-parent")?;let b=binding(&d,"teacher-parent",&native,&es)?;
            assert_eq!(prefix_rows(&d,"teacher-parent",&es,&b,true)?.len(),1);
            assert!(new_rows(&d,"teacher-parent",&es,&b,true).is_err());
            assert!(call_attempt(&d.root,"teacher-parent","teacher",&b,&es[1],1,None)?.is_some());
            return Ok(());
        }
        let mut d=review_source()?;d.root=dir("prefix-process")?;
        let l=checkpoint::load(&d.original.parent,Device::Cpu,false)?;
        let mut c=RunControl::new(std::sync::Arc::new(AtomicBool::new(false)),std::time::Duration::ZERO,u64::MAX)?;
        assert!(teacher_prefix(&d.root,"empty",&binary::record!({"planned":0}),&l,&[],&mut c)?.is_empty());
        let(_,native,es,_,_)=lane(&d,"teacher-parent")?;let b=binding(&d,"teacher-parent",&native,&es)?;
        assert!(teacher_prefix_until(&d.root,"teacher-parent",&b,&l,&es,0,&mut c).is_err());
        let path=d.root.join("teacher-parent-teachers.r3rows");let mut file=std::fs::File::create(&path)?;append_row(&mut file,&b)?;
        let mut first=String::new();
        for(i,e)in es.iter().enumerate(){let attempt=prepare_call(&d.root,"teacher-parent","teacher",&b,e,i)?;
            let row=binary::record!({"ordinal":i,"id":e.id,"case":digest(e)?,"teacher":{"mean_nll":1.,"target_tokens_including_eos":2},"attempt":attempt.file_name().unwrap().to_string_lossy()});
            append_row(&mut file,&row)?;resolve_call(&attempt,Some(&row),&c)?;
            if i==0 {
                first=digest(&row)?;assert_eq!(teacher_prefix_until(&d.root,"teacher-parent",&b,&l,&es,1,&mut c)?.len(),1);
                write(&d.root.join("fixture.r3b"),&d)?;
                let status=std::process::Command::new(std::env::current_exe()?).args(["--exact","training::fresh::muon::diagnosis::tests::prefix_empty_legacy_and_full_manifest_new_process","--ignored","--nocapture","--test-threads=1"]).env("R3_CLOSURE_PREFIX_CHILD",&d.root).status()?;assert!(status.success());
            }
        }
        let rows=teacher_prefix_until(&d.root,"teacher-parent",&b,&l,&es,64,&mut c)?;
        assert_eq!(rows.len(),64);assert_eq!(digest(&rows[0])?,first);assert_eq!(c.receipt()["teacher_calls"],0);
        let mut broken=rows.clone();broken[1]=broken[0].clone();let mut records=vec![b.clone()];records.extend(broken);review_rows(&path,&records)?;
        assert!(prefix_rows(&d,"teacher-parent",&es,&b,true).is_err());
        assert_eq!(c.receipt()["generation_calls"],0);println!("PREFIX_TEST first1->64 same_binding, new_process, empty_legacy, exhausted_budget, duplicate_rejected; model_calls0");Ok(())
    }
// Independently authored by the closure reviewer; insert inside diagnosis tests.
fn review_source() -> Result<Diagnostic> {
    load(&PathBuf::from(std::env::var("R3_ENDPOINT_PREDECESSOR").map_err(|_|bad("explicit predecessor required"))?),false)
}
fn review_rows(path:&Path,rows:&[Value])->Result<()> {
    let mut file=std::fs::File::create(path)?;for row in rows{append_row(&mut file,row)?;}Ok(())
}
fn review_put(path:&Path,value:&Value)->Result<()> {std::fs::write(path,binary::to_vec(value)?)?;Ok(())}
// Copy only four narrow diagnostic journals. Native/corpus/original refs remain
// in place, and every synthetic root-dependent digest is rebound consistently.
fn review_fixture()->Result<Diagnostic> {
    let old=review_source()?;let mut d=old.clone();d.root=dir("independent-closure")?;
    for(i,label)in ["missing","train-A","train-M","teacher-parent"].iter().enumerate(){
        let(_,native,es,teacher,_)=lane(&d,label)?;let b=binding(&d,label,&native,&es)?;
        let filename=format!("{label}{}.r3rows",if teacher{"-teachers"}else{""});
        let mut rows=binary::read_value_records(&old.root.join(&filename))?;rows[0]=b.clone();
        for(j,row)in rows[1..].iter().enumerate(){
            let name=row["attempt"].as_str().unwrap();let mut prepared:Value=read_confirmed(&old.root.join(name))?;
            prepared["binding"]=b.clone();assert_eq!(prepared["ordinal"],j);review_put(&d.root.join(name),&prepared)?;
            let resolved=name.replace("-prepared","-resolved");let mut value:Value=read_confirmed(&old.root.join(&resolved))?;
            value["prepared"]=binary::record!(file_hash(&d.root.join(name))?);review_put(&d.root.join(&resolved),&value)?;
        }
        review_rows(&d.root.join(filename),&rows)?;
        let start_name=format!("observation-{i:03}-started.r3b");let mut start:Value=read_confirmed(&old.root.join(&start_name))?;
        start["policy"]=binary::record!(digest(&d)?);start["binding"]=b.clone();review_put(&d.root.join(&start_name),&start)?;
        let end_name=format!("observation-{i:03}-finished.r3b");let mut end:Value=read_confirmed(&old.root.join(&end_name))?;
        end["start"]=binary::record!(file_hash(&d.root.join(&start_name))?);end["binding"]=b;review_put(&d.root.join(end_name),&end)?;
        if i<3{review_put(&d.root.join(format!("{label}-finished.r3b")),&end)?;}
    }
    review_put(&d.root.join("diagnostic-plan.r3b"),&binary::to_value(&d)?)?;
    Ok(d)
}
#[test]
#[ignore="independent closure predecessor negatives; narrow journals only; no model calls"]
fn independent_successor_rejects_d2_and_failure_forgery()->Result<()> {
    let d=review_fixture()?;let expected=verified_d2(&d)?;assert_eq!(expected,verified_d2(&review_source()?)?);
    let raw=d.root.join("train-A.r3rows");let original=binary::read_value_records(&raw)?;
    for kind in ["missing","duplicate","modified"]{
        let mut changed=original.clone();match kind{"missing"=>{changed.pop();},"duplicate"=>{changed[2]=changed[1].clone();},_=>{changed[1]["id"]=binary::record!("incorrect-case");}}
        review_rows(&raw,&changed)?;assert!(verified_d2(&d).is_err(),"accepted {kind} D2 row");review_rows(&raw,&original)?;
    }
    let resolution=d.root.join(original[1]["attempt"].as_str().unwrap().replace("-prepared","-resolved"));let value:Value=read_confirmed(&resolution)?;
    for state in ["UNKNOWN","NOT_INVOKED"]{let mut changed=value.clone();changed["state"]=binary::record!(state);review_put(&resolution,&changed)?;assert!(verified_d2(&d).is_err(),"accepted incomplete D2 resolution");}
    review_put(&resolution,&value)?;std::fs::write(pending_path(&resolution),b"unconfirmed")?;assert!(verified_d2(&d).is_err());std::fs::remove_file(pending_path(&resolution))?;
    let finish=d.root.join("observation-003-finished.r3b");let original:Value=read_confirmed(&finish)?;
    for condition in ["CANCELLED","UNKNOWN","NONFINITE","IO_ERROR"]{let mut changed=original.clone();changed["control"]["observed_conditions"]=binary::record!([condition]);review_put(&finish,&changed)?;assert!(verified_d2(&d).is_err(),"accepted mixed failure {condition}");}
    let mut changed=original.clone();changed["error"]=binary::record!("unrelated runtime failure");review_put(&finish,&changed)?;assert!(verified_d2(&d).is_err());review_put(&finish,&original)?;
    let failed=binary::read_value_records(&d.root.join("teacher-parent-teachers.r3rows"))?;
    let resolved=d.root.join(failed[1]["attempt"].as_str().unwrap().replace("-prepared","-resolved"));let original:Value=read_confirmed(&resolved)?;
    for condition in ["CANCELLED","UNKNOWN","NONFINITE","IO_ERROR"]{let mut changed=original.clone();changed["control"]["observed_conditions"]=binary::record!([condition]);review_put(&resolved,&changed)?;assert!(verified_d2(&d).is_err(),"accepted disguised failed resolution {condition}");}
    review_put(&resolved,&original)?;assert_eq!(verified_d2(&d)?,expected);
    assert!(segments(&d).is_err(),"old failed root was reopened without successor contract");
    let mut wrong=d.clone();wrong.source=source()?;assert!(verified_d2(&wrong).is_err(),"historical D2 relabeled current producer");
    println!("INDEPENDENT_D2_NEGATIVES_PASS fixture={} actual_generation0 teacher0 optimizer0 backward0",d.root.display());Ok(())
}
#[test]
#[ignore="independent successor lineage negatives; original native refs read in place; no model calls"]
fn independent_successor_rejects_identity_changes()->Result<()> {
    let old=review_source()?;let parent=dir("independent-successor")?;let root=parent.join("run");
    let executor=PathBuf::from(std::env::var("R3_ENDPOINT_EXECUTOR").map_err(|_|bad("explicit predecessor executor required"))?);
    let audit=parent.join("audit.txt");std::fs::write(&audit,b"independent scoped fixture; not model evidence")?;
    successor(&old.root,&executor,&audit,&root)?;let d=load(&root,false)?;predecessor(&d)?;
    let mut changed=d.clone();changed.original.parent=changed.endpoints[0].native.clone();assert!(predecessor(&changed).is_err());
    let mut changed=d.clone();changed.original.parent_step+=1;assert!(predecessor(&changed).is_err());
    let mut changed=d.clone();changed.original.tokenizer="different-tokenizer".into();assert!(predecessor(&changed).is_err());
    let mut changed=d.clone();changed.runtime.actual_device="Cpu".into();assert!(predecessor(&changed).is_err());
    for arm in 0..2{let mut changed=d.clone();changed.endpoints[arm].physical="0".repeat(64);assert!(predecessor(&changed).is_err());let mut changed=d.clone();changed.endpoints[arm].local-=1;assert!(predecessor(&changed).is_err());}
    let mut changed=d.clone();changed.teacher_cases.swap(0,1);assert!(predecessor(&changed).is_err());
    println!("INDEPENDENT_LINEAGE_NEGATIVES_PASS parent/A/M/step/tokenizer/runtime/selection rejected; actual_generation0 teacher0 optimizer0 backward0");Ok(())
}
    fn original()->Result<Study>{let p=PathBuf::from(std::env::var("R3_MUON_PREPARATION").map_err(|_|bad("explicit original study required"))?);load_study(&p,false)}
    fn dir(name:&str)->Result<PathBuf>{let p=std::env::temp_dir().join(format!("endpoint-{name}-{}-{}",std::process::id(),std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()));std::fs::create_dir(&p)?;Ok(p)}
    fn policy(s:Study,root:PathBuf)->Result<Diagnostic>{
        let end=history(&s)?.last().unwrap().arms.clone();
        Ok(Diagnostic{contract:ID.into(),root,original:s,endpoints:end,refs:BTreeMap::new(),panels:vec![],source:source()?,runtime:RuntimeProfile::capture(Backend::Metal0,&Backend::Metal0.open()?)?,teacher_cases:vec![],generation_cap:438,teacher_cap:216,active_cap:1800.,segment_cap:900.,bytes_cap:256*1024*1024,predecessor:None})
    }
    #[test]
    #[ignore="scoped original artifacts, no model call"]
    fn original_decode_failure_is_not_general_failure_permission()->Result<()> {
        let s=original()?;let h=history(&s)?;let rs=binary::read_value_records(&s.root.join("M/eval-512-renamed.r3rows"))?;
        let tok=scoring_tokenizer(&s)?;assert_eq!(rs.len(),11);verify_output_result(&rs[10],&tok)?;original_stop(&h,&rs[10])?;
        assert!(previous_usage(&s,&h).is_err());assert!(!h.last().unwrap().resume);
        for reason in ["CANCELLED","UNKNOWN","NONFINITE","IO_ERROR"] {let mut v=h.clone();v.last_mut().unwrap().control["observed_conditions"]=binary::record!([reason]);assert!(original_stop(&v,&rs[10]).is_err());}
        let mut v=h.clone();v.last_mut().unwrap().arms[1].local=511;assert!(original_stop(&v,&rs[10]).is_err());
        println!("old study remains closed; original UTF8 RETURNED10 preserved; missing=54; model_calls0");Ok(())
    }
    #[test]
    #[ignore="bound original refs only, no model call"]
    fn changed_refs_reject_before_model_call()->Result<()> {
        let s=original()?;let root=dir("refs")?;let mut d=policy(s,root)?;
        for p in [d.original.parent.clone(),d.original.word_root.join("corpus.r3cor"),d.endpoints[0].native.clone(),d.endpoints[1].native.clone()]{bind(&mut d.refs,&p)?;}
        check_refs(&d)?;
        for p in d.refs.keys().cloned().collect::<Vec<_>>(){let old=d.refs.insert(p.clone(),"0".repeat(64)).unwrap();assert!(check_refs(&d).is_err());d.refs.insert(p,old);}
        let local=d.root.join("fixture.r3b");write(&local,&binary::record!(1))?;bind(&mut d.refs,&local)?;write(&pending_path(&local),&binary::record!(true))?;assert!(check_refs(&d).is_err());Ok(())
    }
    #[test]
    fn teacher_roles_scalar_and_mixed_alignment()->Result<()> {
        let answer="정지입니다. [event:12345678]";let tok=ByteBpe::train(&[answer.as_bytes().to_vec()],&"a".repeat(64),264)?;
        let mut gold=tok.encode(answer.as_bytes())?;gold.push(EOS);let roles=role_tokens(answer,&gold,&tok)?;
        assert_eq!(roles.iter().filter(|s|*s=="VALUE").count(),"정지".len());assert_eq!(roles.iter().filter(|s|*s=="ID").count(),8);assert_eq!(roles.last().unwrap(),"EOS");
        let mixed=ByteBpe::train(&vec![answer.as_bytes().to_vec();8],&"b".repeat(64),300)?;let mut ids=mixed.encode(answer.as_bytes())?;ids.push(EOS);assert!(role_tokens(answer,&ids,&mixed)?.contains(&"MIXED".into()));
        let e=Episode{id:"scalar".into(),answer:answer.into(),category:0,family:"scalar-fixture".into(),binding:String::new(),sequence:String::new(),
            request:ModelRequest{request_id:"scalar".into(),system:String::new(),input:String::new(),evidence:EvidenceBundle{items:vec![],truncated:false,visited:0,candidates_fetched:0,edges_fetched:0,eligible:0},limits:GenerationLimits::default()}};
        let logits=[-1f64,2.,0.];let logz=logits.iter().map(|x|x.exp()).sum::<f64>().ln();let nll=logz-logits[1];
        let row=binary::record!({"case":digest(&e)?,"teacher":{"target_token_observation":{"gold":gold,"argmax":gold,"nll":vec![nll;gold.len()]},"training_prompt_matches_generation":true,"answer_tokenizer_roundtrip":true}});
        let score=teacher_scores(std::slice::from_ref(&e),&[row.clone()],&tok)?;assert_eq!(score["roles"]["ID"][0],8);assert_eq!(score["roles"]["ID"][1],8);assert!((score["roles"]["ID"][2].as_f64().unwrap()-8.*nll).abs()<1e-12);
        let mut broken=row;broken["teacher"]["target_token_observation"]["gold"]=binary::record!(&gold[1..]);assert!(teacher_scores(&[e],&[broken],&tok).is_err());Ok(())
    }
    #[test]
    fn failure_selection_keeps_correct_mates_and_invalid_utf8(){
        let rows=(0..16).map(|i|binary::record!({"id":format!("r{i:02}"),"exact_match":i!=7&&i!=11,"error_class":if i==11{Some("strict_utf8")}else{None}})).collect::<Vec<_>>();
        assert_eq!(failure_pairs(&rows,&BTreeSet::from([0,1]),32),vec![11,10,7,6]);
        assert_eq!(failure_pairs(&rows,&BTreeSet::from([6,7]),2),vec![11,10]);
    }
    #[test]
    #[ignore="native/sample input read only; no forward"]
    fn teacher_shift_matches_full_answer_and_causal_positions()->Result<()> {
        let s=original()?;let l=checkpoint::load(&s.parent,Device::Cpu,false)?;let es=panels(&s,512,true)?.remove(0).1;
        let samples=samples_with_framing(&es[..4],&l.tokenizer,256,neural::Framing::QuestionEvidence)?;
        for(e,sample)in es[..4].iter().zip(samples){let p=l.tokenizer.prepare_with_framing(&e.request,l.manifest.framing()?,l.model.config.context as u32,&l.model.config.id()?)?;
            let mut gold=l.tokenizer.encode(e.answer.as_bytes())?;gold.push(EOS);
            assert_eq!(sample.tokens[..sample.response_start],p.token_ids);assert_eq!(sample.tokens[sample.response_start..],gold);
            let mut sequence=p.token_ids.clone();sequence.extend(&gold);for j in 0..gold.len(){let pos=p.token_ids.len()-1+j;assert_eq!(sequence[pos+1],gold[j]);assert_eq!(&sequence[p.token_ids.len()..pos+1],&gold[..j]);}
        }Ok(())
    }
    #[test]
    #[ignore="synthetic immutable call records, no generation"]
    fn not_invoked_returned_unknown_and_pure_reader()->Result<()> {
        let s=original()?;let es=panels(&s,512,true)?.remove(0).1;let root=dir("attempt")?;let b=binary::record!({"fixture":true});
        let mut zero=RunControl::new(std::sync::Arc::new(AtomicBool::new(false)),std::time::Duration::ZERO,u64::MAX)?;assert!(zero.check("fixture").is_err());
        let p=prepare_call(&root,"no-call","generation",&b,&es[0],0)?;resolve_call(&p,None,&zero)?;assert!(call_attempt(&root,"no-call","generation",&b,&es[0],0,None)?.is_some());
        let _=prepare_call(&root,"unknown","generation",&b,&es[0],0)?;assert!(call_attempt(&root,"unknown","generation",&b,&es[0],0,None).is_err());
        let mut cancel=RunControl::new(std::sync::Arc::new(AtomicBool::new(true)),std::time::Duration::ZERO,u64::MAX)?;assert!(cancel.seal_completed_no_call().is_err());
        let d=policy(s,root.clone())?;let n=std::fs::read_dir(&root)?.count();assert!(assemble(&d).is_err());assert!(require_complete(&d).is_err());assert_eq!(std::fs::read_dir(&root)?.count(),n);Ok(())
    }
    #[test]
    #[ignore="fresh process actual observation caller at1800s, synthetic RETURNED; no model calls"]
    fn completed_observation_process_at_exhausted_budget()->Result<()> {
        if let Ok(root)=std::env::var("R3_ENDPOINT_CHILD") {let d:Diagnostic=read_confirmed(&PathBuf::from(root).join("diagnostic-plan.r3b"))?;observe(&d,"train-A")?;observe(&d,"train-A")?;let h=segments(&d)?;assert_eq!(usage(&h)?.1,0);assert_eq!(usage(&h)?.2,0);return Ok(());}
        let s=original()?;let d=policy(s,dir("process")?)?;let(path,model,es,_,_)=lane(&d,"train-A")?;let b=binding(&d,"train-A",&model,&es)?;
        let l=checkpoint::load(&path,Device::Cpu,false)?;let mut f=std::fs::OpenOptions::new().write(true).create_new(true).open(d.root.join("train-A.r3rows"))?;append_row(&mut f,&b)?;
        let ctl=RunControl::new(std::sync::Arc::new(AtomicBool::new(false)),std::time::Duration::from_secs(60),u64::MAX)?;
        for(i,e)in es.iter().enumerate(){let p=l.tokenizer.prepare_with_framing(&e.request,l.manifest.framing()?,l.model.config.context as u32,&l.model.config.id()?)?;let ids=l.tokenizer.encode(e.answer.as_bytes())?;let mut raw=ids.clone();raw.push(EOS);
            let attempt=prepare_call(&d.root,"train-A","generation",&b,e,i)?;
            let row=binary::record!({"row_version":2,"id":e.id,"expected":e.answer,"generated_question":e.request.input,"generated_evidence":e.request.evidence,"request_digest":digest(&e.request)?,"native_prompt_digest":p.token_digest,"prompt_digest":digest(&p.token_ids)?,"provided":p.provided,"raw_tokens":raw,"actual":e.answer,"error":null,"generation_error":null,"decode_error":null,"finish_reason":"stop","generation_completed":true,"generation":{"tokens":ids,"generated":raw.len(),"finish":"stop"},"exact_match":true,"attempt":attempt.file_name().unwrap().to_string_lossy()});
            append_row(&mut f,&row)?;resolve_call(&attempt,Some(&row),&ctl)?;
        }
        let start=d.root.join("observation-000-started.r3b");publish_confirmed(&start,&binary::record!({"policy":digest(&d)?}))?;
        publish_confirmed(&d.root.join("observation-000-finished.r3b"),&binary::record!({"start":file_hash(&start)?,"success":false,"resume":true,"control":{"elapsed_seconds":1800.,"generation_calls":0,"teacher_calls":0,"observed_conditions":["TIME_BUDGET"]},"fixture":true}))?;
        publish_confirmed(&d.root.join("diagnostic-plan.r3b"),&d)?;let before=file_hash(&d.root.join("train-A.r3rows"))?;
        let status=std::process::Command::new(std::env::current_exe()?).args(["--exact","training::fresh::muon::diagnosis::tests::completed_observation_process_at_exhausted_budget","--ignored","--nocapture","--test-threads=1"]).env("R3_ENDPOINT_CHILD",&d.root).status()?;assert!(status.success());assert_eq!(before,file_hash(&d.root.join("train-A.r3rows"))?);println!("fixture={} optimizer0 backward0 generation0 teacher0",d.root.display());Ok(())
    }
}
